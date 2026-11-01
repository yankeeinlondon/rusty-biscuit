//! The registry of YAML document kinds one document context can select.
//!
//! A registry holds every `kind: yaml-schema` definition that claims a kind
//! for documents in one [`FileResolutionContext`]: the shipped definitions
//! ([`shipped_kind_definitions`]) and, for [`RegistryScope::WithRoots`], the
//! definitions found directly inside the context's five schema roots (the
//! roots triggers use, [`SchemaRoots::for_document`]). Filenames never decide
//! anything: two definitions claiming one kind are a [`Claim::Duplicate`],
//! whichever root each came from. Kinds compare exactly, so `roster` and
//! `Roster` are different kinds.
//!
//! ## Building
//!
//! ```mermaid
//! flowchart TD
//!     S[shipped definitions] -->|prepared, claims read| R[(claims)]
//!     F[file in a schema root] --> C{top-level kind: yaml-schema?}
//!     C -- no --> X[not a candidate]
//!     C -- yes --> P{admitted, prepared,<br/>claims a finite kind?}
//!     P -- yes --> H[Healthy] --> R
//!     P -- no --> V{inline literal / enum kind<br/>in the authored $schema?}
//!     V -- yes --> Res[Reserved] --> R
//!     V -- no --> I[Incomplete]
//! ```
//!
//! A file at a shipped definition's repository-relative path that claims that
//! definition's kind is the shipped definition's source: it replaces the
//! compiled-in copy and is not a duplicate (an edited checkout definition
//! applies at once). Building never fails: a failed definition, an unreadable
//! file, and an unreadable root are recorded in the registry and decide the
//! selection of the documents they could affect (see [`super::select`]).
//!
//! Building reads local files through the context and a [`SchemaRootSource`];
//! it evaluates nothing, runs no commands, fetches nothing, and writes
//! nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, OnceLock};

use biscuit_file::FileResolutionContext;
use serde_yaml_ng::Value as YamlValue;
use simplified_schema::resolve::ResolvedSchema;
use simplified_schema::{SchemaArm, SchemaError, SchemaRoot, SimplifiedType, claimed_values};

use super::super::embedded::{SHIPPED_SOURCES, shipped_kind_definitions};
use super::super::roots::SchemaRoots;
use super::super::triggers::discovery::SchemaRootSource;
use super::super::triggers::envelope::lexically_declared_kind;

/// Which definitions a registry holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryScope {
    /// The shipped definitions only.
    ShippedOnly,
    /// The shipped definitions and those in the context's schema roots.
    WithRoots,
}

impl RegistryScope {
    /// The scope `md` uses: the schema roots are searched inside a
    /// repository, and only the shipped definitions apply outside one.
    #[must_use]
    pub fn for_context(context: &FileResolutionContext) -> Self {
        if context.repository_root().is_some() { Self::WithRoots } else { Self::ShippedOnly }
    }
}

/// Where a definition came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionSource {
    /// Compiled into the binary, named by its repository-relative path.
    Shipped(&'static str),
    /// A file in one of the context's schema roots.
    File(PathBuf),
}

impl DefinitionSource {
    /// The path that names the definition: the file, or the shipped
    /// definition's repository-relative path.
    #[must_use]
    pub fn path(&self) -> PathBuf {
        match self {
            Self::Shipped(path) => PathBuf::from(path),
            Self::File(path) => path.clone(),
        }
    }

    /// A label for messages: the file's path, or `built-in <path>`.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Shipped(path) => format!("built-in {path}"),
            Self::File(path) => path.display().to_string(),
        }
    }
}

/// A prepared definition of one or more YAML document kinds.
#[derive(Debug)]
pub struct KindDefinition {
    source: DefinitionSource,
    resolved: ResolvedSchema,
    kinds: Vec<String>,
}

impl KindDefinition {
    #[allow(missing_docs)]
    #[must_use]
    pub fn source(&self) -> &DefinitionSource {
        &self.source
    }

    /// The exported `$schema`: the contract of a document of its kinds.
    #[must_use]
    pub fn resolved(&self) -> &ResolvedSchema {
        &self.resolved
    }

    /// The kinds it claims, in first-declared order.
    #[must_use]
    pub fn kinds(&self) -> &[String] {
        &self.kinds
    }
}

/// What a registry knows about one kind.
#[derive(Debug, Clone)]
pub enum Claim {
    /// Exactly one definition claims it.
    Healthy(Arc<KindDefinition>),
    /// More than one definition claims it; the paths are sorted.
    Duplicate(Vec<PathBuf>),
    /// The one definition that claims it failed.
    Reserved {
        /// The failed definition.
        file: PathBuf,
        /// Why it failed.
        defect: Arc<SchemaError>,
    },
}

/// A failure that leaves the registry unable to say what some file claims.
#[derive(Debug, Clone)]
pub struct Incomplete {
    /// The definition file, or the schema root that could not be read.
    pub file: PathBuf,
    /// The failure.
    pub problem: Arc<SchemaError>,
}

/// The YAML document kinds one document context can select. See the
/// [module docs](self).
#[derive(Debug, Clone)]
pub struct KindRegistry {
    claims: BTreeMap<String, Claim>,
    incomplete: Vec<Incomplete>,
    root_failures: Vec<Incomplete>,
    roots: SchemaRoots,
    dependencies: Vec<PathBuf>,
    searched: Vec<PathBuf>,
}

/// One definition's contribution before claims are grouped by kind.
enum Entry {
    Healthy(Arc<KindDefinition>),
    Reserved(Arc<SchemaError>),
}

impl KindRegistry {
    /// The registry of `context`.
    ///
    /// `context` must be the document's own context: its package, package
    /// area, tree root, `SCHEMAS_DIR`, and home decide the schema roots.
    /// `source` supplies the roots' file lists and texts (DMLS passes its
    /// unsaved buffers); imports of a discovered definition are read through
    /// `context`.
    pub fn build(context: &FileResolutionContext, source: &mut dyn SchemaRootSource, scope: RegistryScope) -> Self {
        let _discover = simplified_schema::timings::stage(simplified_schema::ValidationStage::Discover);
        let mut builder = Builder::default();
        if scope == RegistryScope::WithRoots {
            builder.discover(context, source);
        }
        builder.finish()
    }

    /// [`Self::build`] reading the schema roots from the filesystem.
    pub fn from_filesystem(context: &FileResolutionContext, scope: RegistryScope) -> Self {
        Self::build(context, &mut super::super::triggers::discovery::FilesystemRoots, scope)
    }

    /// The registry of the shipped definitions alone, prepared once.
    #[must_use]
    pub fn shipped() -> Arc<Self> {
        static SHIPPED: OnceLock<Arc<KindRegistry>> = OnceLock::new();
        Arc::clone(SHIPPED.get_or_init(|| Arc::new(Builder::default().finish())))
    }

    /// What the registry knows about `kind`, compared exactly; `None` when no
    /// definition claims it.
    #[must_use]
    pub fn claim(&self, kind: &str) -> Option<&Claim> {
        self.claims.get(kind)
    }

    /// Every claimed kind and its claim, in kind order.
    pub fn claims(&self) -> impl Iterator<Item = (&str, &Claim)> {
        self.claims.iter().map(|(kind, claim)| (kind.as_str(), claim))
    }

    /// Definitions that failed without revealing a claim, and files that
    /// could not be read, sorted by path. While any exist, a kind no healthy
    /// definition claims cannot be called unclaimed.
    #[must_use]
    pub fn incomplete(&self) -> &[Incomplete] {
        &self.incomplete
    }

    /// Schema roots that could not be read.
    #[must_use]
    pub fn root_failures(&self) -> &[Incomplete] {
        &self.root_failures
    }

    /// The schema roots the registry searched (none for
    /// [`RegistryScope::ShippedOnly`]); a fallback `$schema` bare name is
    /// looked up in them.
    #[must_use]
    pub fn roots(&self) -> &SchemaRoots {
        &self.roots
    }

    /// Files whose content the registry depends on: every file of a searched
    /// root and every file a definition imported. Sorted.
    #[must_use]
    pub fn dependencies(&self) -> &[PathBuf] {
        &self.dependencies
    }

    /// Locations whose appearance can change the registry: the searched
    /// roots and every location a definition's resolution probed. Sorted.
    #[must_use]
    pub fn searched_locations(&self) -> &[PathBuf] {
        &self.searched
    }
}

/// A registry under construction.
#[derive(Default)]
struct Builder {
    entries: Vec<(String, PathBuf, Entry)>,
    incomplete: Vec<Incomplete>,
    root_failures: Vec<Incomplete>,
    roots: Option<SchemaRoots>,
    dependencies: BTreeSet<PathBuf>,
    searched: BTreeSet<PathBuf>,
    /// Shipped definitions a checkout copy replaces.
    replaced: BTreeSet<&'static str>,
}

impl Builder {
    /// Adds the definitions in `context`'s schema roots, read from `source`.
    fn discover(&mut self, context: &FileResolutionContext, source: &mut dyn SchemaRootSource) {
        let roots = match SchemaRoots::for_document(context) {
            Ok(roots) => roots,
            Err(error) => {
                let file = failed_path(&error, context.base_dir());
                self.root_failures.push(Incomplete { file, problem: Arc::new(error) });
                return;
            }
        };
        for root in roots.searched() {
            self.searched.insert(root.path.clone());
            let files = match source.root_files(&root.path) {
                Ok(files) => files,
                Err(error) => {
                    self.root_failures.push(Incomplete { file: root.path.clone(), problem: Arc::new(error) });
                    continue;
                }
            };
            for file in files {
                self.dependencies.insert(file.clone());
                match source.root_file_text(&file) {
                    Ok(text) => self.add_candidate(context, &roots, file, &text),
                    Err(error) => self.incomplete.push(Incomplete { file, problem: Arc::new(error) }),
                }
            }
        }
        self.roots = Some(roots);
    }

    /// Adds `file`, whose text is `text`, when it is a definition.
    fn add_candidate(&mut self, context: &FileResolutionContext, roots: &SchemaRoots, file: PathBuf, text: &str) {
        let Some(candidate) = Candidate::classify(text) else {
            return;
        };
        let shipped = shipped_identity(context, &file);
        let (prepared, observed) = simplified_schema::resolve::observed::collect(|| {
            prepare_candidate(&file, text, roots.search_paths(), context)
        });
        self.dependencies.extend(observed.read);
        self.searched.extend(observed.searched);
        let (kinds, entry) = match prepared {
            Ok(definition) => {
                let definition = Arc::new(definition);
                (definition.kinds.clone(), Entry::Healthy(definition))
            }
            Err(defect) => {
                let revealed = candidate.revealed_claims();
                let defect = Arc::new(defect);
                if revealed.is_empty() {
                    self.incomplete.push(Incomplete { file, problem: defect });
                    return;
                }
                (revealed, Entry::Reserved(defect))
            }
        };
        if let Some(shipped) = shipped
            && kinds.iter().any(|kind| shipped_claims(shipped, kind))
        {
            self.replaced.insert(shipped);
        }
        for kind in kinds {
            let entry = match &entry {
                Entry::Healthy(definition) => Entry::Healthy(Arc::clone(definition)),
                Entry::Reserved(defect) => Entry::Reserved(Arc::clone(defect)),
            };
            self.entries.push((kind, file.clone(), entry));
        }
    }

    /// Adds every shipped definition no checkout copy replaced and groups
    /// the claims.
    fn finish(mut self) -> KindRegistry {
        for shipped in shipped_definitions() {
            if self.replaced.contains(shipped.path) {
                continue;
            }
            match &shipped.prepared {
                Ok(definition) => {
                    for kind in &definition.kinds {
                        self.entries.push((kind.clone(), PathBuf::from(shipped.path), Entry::Healthy(Arc::clone(definition))));
                    }
                }
                Err(defect) => {
                    self.incomplete.push(Incomplete { file: PathBuf::from(shipped.path), problem: Arc::clone(defect) });
                }
            }
        }
        self.incomplete.sort_by(|a, b| a.file.cmp(&b.file));
        KindRegistry {
            claims: group_claims(self.entries),
            incomplete: self.incomplete,
            root_failures: self.root_failures,
            roots: self.roots.unwrap_or_else(SchemaRoots::none),
            dependencies: self.dependencies.into_iter().collect(),
            searched: self.searched.into_iter().collect(),
        }
    }
}

/// Groups each kind's entries into its claim; two or more is a duplicate
/// naming every file, independent of discovery order.
fn group_claims(entries: Vec<(String, PathBuf, Entry)>) -> BTreeMap<String, Claim> {
    let mut by_kind: BTreeMap<String, Vec<(PathBuf, Entry)>> = BTreeMap::new();
    for (kind, file, entry) in entries {
        by_kind.entry(kind).or_default().push((file, entry));
    }
    by_kind
        .into_iter()
        .map(|(kind, mut claimants)| {
            let claim = if claimants.len() == 1 {
                match claimants.pop().expect("one claimant") {
                    (_, Entry::Healthy(definition)) => Claim::Healthy(definition),
                    (file, Entry::Reserved(defect)) => Claim::Reserved { file, defect },
                }
            } else {
                let mut files: Vec<PathBuf> = claimants.into_iter().map(|(file, _)| file).collect();
                files.sort();
                files.dedup();
                Claim::Duplicate(files)
            };
            (kind, claim)
        })
        .collect()
}

/// A shipped definition and its preparation.
struct ShippedDefinition {
    path: &'static str,
    prepared: Result<Arc<KindDefinition>, Arc<SchemaError>>,
}

/// The shipped definitions, prepared once per process.
fn shipped_definitions() -> &'static [ShippedDefinition] {
    static PREPARED: OnceLock<Vec<ShippedDefinition>> = OnceLock::new();
    PREPARED.get_or_init(|| {
        let documents = shipped_kind_definitions();
        documents
            .iter()
            .map(|document| ShippedDefinition {
                path: document.path,
                prepared: prepare_shipped(document.path, documents).map(Arc::new).map_err(Arc::new),
            })
            .collect()
    })
}

fn prepare_shipped(
    path: &'static str,
    documents: &[simplified_schema::embedded::EmbeddedDocument<'static>],
) -> Result<KindDefinition, SchemaError> {
    let resolved = simplified_schema::resolve::prepare_embedded_yaml_schema(path, documents, SHIPPED_SOURCES)?;
    let kinds = claimed_values(&resolved, "kind").map_err(|problem| claim_error(Path::new(path), &problem))?;
    Ok(KindDefinition { source: DefinitionSource::Shipped(path), resolved, kinds })
}

/// Whether the shipped definition at `path` claims `kind`.
fn shipped_claims(path: &str, kind: &str) -> bool {
    shipped_definitions()
        .iter()
        .find(|shipped| shipped.path == path)
        .and_then(|shipped| shipped.prepared.as_ref().ok())
        .is_some_and(|definition| definition.kinds.iter().any(|claimed| claimed == kind))
}

/// The shipped definition `file` is the source of, by repository-relative
/// path: `file` relative to the context's repository root, compared as
/// `/`-joined components.
fn shipped_identity(context: &FileResolutionContext, file: &Path) -> Option<&'static str> {
    let root = context.repository_root()?;
    let canonical = |path: &Path| biscuit_file::canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf());
    let relative = canonical(file).strip_prefix(canonical(root)).ok()?.to_path_buf();
    let mut segments = Vec::new();
    for component in relative.components() {
        let Component::Normal(segment) = component else {
            return None;
        };
        segments.push(segment.to_str()?.to_string());
    }
    let relative = segments.join("/");
    shipped_kind_definitions().iter().map(|document| document.path).find(|path| *path == relative)
}

/// Prepares a discovered definition: admitted against the shipped
/// `yaml-schema` definition, its export resolved in `context`, its kinds read.
fn prepare_candidate(
    file: &Path,
    text: &str,
    schema_roots: &[PathBuf],
    context: &FileResolutionContext,
) -> Result<KindDefinition, SchemaError> {
    let resolved = simplified_schema::resolve::prepare_yaml_schema_text(file, text, schema_roots, context, SHIPPED_SOURCES)?;
    let kinds = claimed_values(&resolved, "kind").map_err(|problem| claim_error(file, &problem))?;
    Ok(KindDefinition { source: DefinitionSource::File(file.to_path_buf()), resolved, kinds })
}

/// The error for a definition whose export claims no finite set of kinds.
pub(crate) fn claim_error(file: &Path, problem: &simplified_schema::ClaimProblem) -> SchemaError {
    SchemaError::SchemaDocument {
        path: file.to_path_buf(),
        message: format!("the exported `$schema` claims no document kind: {problem}"),
        source: None,
        span: None,
    }
}

/// The path an error names, for an unreadable root.
fn failed_path(error: &SchemaError, fallback: &Path) -> PathBuf {
    match error {
        SchemaError::Io { path, .. } => path.clone(),
        _ => fallback.to_path_buf(),
    }
}

/// A file in a schema root that claims `kind: yaml-schema`.
enum Candidate {
    /// It parsed; its authored document.
    Parsed(YamlValue),
    /// It does not parse but lexically claims the kind.
    Malformed,
}

impl Candidate {
    /// `None` for a file that is not a definition (any other `kind`, no
    /// `kind`, a non-mapping root, or unparseable text that does not claim
    /// `kind: yaml-schema`).
    fn classify(text: &str) -> Option<Self> {
        let body = text.strip_prefix('\u{feff}').unwrap_or(text);
        match simplified_schema::numeric::load_admitted_yaml(body) {
            Ok(value) => (value.get("kind").and_then(YamlValue::as_str) == Some("yaml-schema")).then_some(Self::Parsed(value)),
            Err(_) => lexically_declared_kind(body, &["yaml-schema"]).is_some().then_some(Self::Malformed),
        }
    }

    /// The kinds a failed definition still reveals: the string choices of an
    /// inline `literal(...)` or `enum(...)` `kind` in its authored `$schema`
    /// (each inline root-union arm included), read without resolving
    /// anything.
    fn revealed_claims(&self) -> Vec<String> {
        let Self::Parsed(document) = self else {
            return Vec::new();
        };
        let Some(export) = document.get("$schema") else {
            return Vec::new();
        };
        let Ok(schema) = simplified_schema::parse_yaml_schema(export) else {
            return Vec::new();
        };
        let shapes: Vec<&simplified_schema::SchemaShape> = match &schema {
            SchemaRoot::Single(shape) => vec![shape],
            SchemaRoot::Union(arms) => arms
                .iter()
                .filter_map(|arm| match arm {
                    SchemaArm::Inline(shape) => Some(shape),
                    SchemaArm::FileRef(_) => None,
                })
                .collect(),
        };
        let mut kinds = Vec::new();
        for shape in shapes {
            let Some(definition) = shape.properties.get("kind") else {
                continue;
            };
            let atoms: Vec<&simplified_schema::PropertyAtom> = match definition {
                simplified_schema::PropertyDef::Single(atom) => vec![atom],
                simplified_schema::PropertyDef::Union(atoms) => atoms.iter().collect(),
            };
            for atom in atoms {
                for kind in inline_choices(atom) {
                    if !kind.is_empty() && !kinds.contains(&kind) {
                        kinds.push(kind);
                    }
                }
            }
        }
        kinds
    }
}

/// The string choices of an inline `literal(...)` or `enum(...)` atom.
fn inline_choices(atom: &simplified_schema::PropertyAtom) -> Vec<String> {
    use simplified_schema::{Constraint, TypeExpr};
    if atom.is_array {
        return Vec::new();
    }
    match &atom.ty {
        TypeExpr::Primitive(SimplifiedType::Literal) => {
            atom.literal_value().and_then(|value| value.as_str()).map(str::to_string).into_iter().collect()
        }
        TypeExpr::Primitive(SimplifiedType::Enum) => atom
            .constraints
            .iter()
            .find_map(|constraint| match constraint {
                Constraint::Members(members) => Some(members.clone()),
                _ => None,
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}
