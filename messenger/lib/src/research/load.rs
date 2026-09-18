//! Passive loading of research contract files.
//!
//! Loading reads the file, binds it to the contract schema for its kind,
//! validates the frontmatter through Darkmatter's library (the `md schema
//! validate` code path), gates the schema version, and deserializes the same
//! vocabulary into the typed model. It never evaluates frontmatter
//! expressions, spawns a process, or touches the network, and it never
//! shells out to `md`.
//!
//! File references (`$schema`, roster `file`) resolve through one
//! [`FileResolutionContext`] captured when the [`Loader`] is built, anchored
//! at the workspace's repository root.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use biscuit_file::{FileReference, FileResolutionContext};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema, ValidationProblemCode};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use super::diagnostics::{Diagnostic, Findings, Rule};
use super::error::ResearchError;
use super::model::{Mappings, Overrides, PlatformDocument, Roster};
use super::paths::{RepoPath, Workspace, normalize};

/// The only supported schema version for every contract file.
pub const SUPPORTED_SCHEMA_VERSION: u64 = 1;

/// The four kinds of contract file, each bound to one shipped schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContractKind {
    Document,
    Roster,
    Overrides,
    Mappings,
}

impl ContractKind {
    /// The top-level key that carries the contract version.
    pub fn version_key(self) -> &'static str {
        match self {
            ContractKind::Roster => "roster_version",
            _ => "schema_version",
        }
    }

    /// The shipped schema a file of this kind must declare.
    pub fn schema_path(self, workspace: &Workspace) -> PathBuf {
        match self {
            ContractKind::Document => workspace.document_schema(),
            ContractKind::Roster => workspace.roster_schema(),
            ContractKind::Overrides => workspace.overrides_schema(),
            ContractKind::Mappings => workspace.mappings_schema(),
        }
    }
}

/// A typed contract file.
pub trait ContractFile: DeserializeOwned {
    const KIND: ContractKind;
}

impl ContractFile for PlatformDocument {
    const KIND: ContractKind = ContractKind::Document;
}

impl ContractFile for Roster {
    const KIND: ContractKind = ContractKind::Roster;
}

impl ContractFile for Overrides {
    const KIND: ContractKind = ContractKind::Overrides;
}

impl ContractFile for Mappings {
    const KIND: ContractKind = ContractKind::Mappings;
}

/// The result of loading one file: its frontmatter, the typed record when
/// the file passed schema, version, and typed checks, and any findings.
///
/// A file with findings stays inspectable through `frontmatter`.
#[derive(Debug, Clone)]
pub struct Loaded<T> {
    pub path: RepoPath,
    /// The frontmatter object without `$schema`, exactly as authored (no
    /// Darkmatter type coercion). Canonical fact fingerprints hash records
    /// taken from here.
    pub frontmatter: Value,
    pub record: Option<T>,
    pub diagnostics: Vec<Diagnostic>,
}

impl<T> Loaded<T> {
    /// Loaded, typed, and free of load-stage findings.
    pub fn is_clean(&self) -> bool {
        self.record.is_some() && self.diagnostics.is_empty()
    }
}

/// Loads contract files from one [`Workspace`].
#[derive(Clone)]
pub struct Loader {
    workspace: Workspace,
    context: FileResolutionContext,
    schemas: DarkmatterSchemas,
    /// Resolved schemas by kind. Binding guarantees every file of a kind
    /// declares the same shipped schema, so it resolves once per loader
    /// (Darkmatter otherwise re-resolves imports for every document).
    effective: Arc<Mutex<HashMap<ContractKind, Arc<EffectiveSchema>>>>,
}

impl Loader {
    /// Captures the file-resolution context once, anchored at the repository
    /// root, and shares it with Darkmatter's schema resolution.
    pub fn new(workspace: Workspace) -> Self {
        let root = workspace.repo_root().to_path_buf();
        let context = FileResolutionContext::new(root.clone()).with_repository_root(root);
        let schemas = DarkmatterSchemas::new().with_file_resolution_context(context.clone());
        Self {
            workspace,
            context,
            schemas,
            effective: Arc::default(),
        }
    }

    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// Loads one platform research document.
    ///
    /// ## Errors
    ///
    /// See [`Loader::load`].
    pub fn load_document(&self, path: &Path) -> Result<Loaded<PlatformDocument>, ResearchError> {
        self.load(path)
    }

    /// Loads a roster.
    ///
    /// ## Errors
    ///
    /// See [`Loader::load`].
    pub fn load_roster(&self, path: &Path) -> Result<Loaded<Roster>, ResearchError> {
        self.load(path)
    }

    /// Loads an overrides file.
    ///
    /// ## Errors
    ///
    /// See [`Loader::load`].
    pub fn load_overrides(&self, path: &Path) -> Result<Loaded<Overrides>, ResearchError> {
        self.load(path)
    }

    /// Loads an implementation-mappings file.
    ///
    /// ## Errors
    ///
    /// See [`Loader::load`].
    pub fn load_mappings(&self, path: &Path) -> Result<Loaded<Mappings>, ResearchError> {
        self.load(path)
    }

    /// Loads one contract file of kind `T`.
    ///
    /// ## Errors
    ///
    /// Returns [`ResearchError`] only when the file cannot be read, is outside
    /// the workspace, has unparseable frontmatter, or its bound schema cannot
    /// be resolved or compiled. Contract problems are findings instead.
    pub fn load<T: ContractFile>(&self, path: &Path) -> Result<Loaded<T>, ResearchError> {
        let absolute = normalize(&self.absolute(path));
        let repo_path = self.workspace.repo_path(&absolute)?;
        std::fs::metadata(&absolute).map_err(|source| ResearchError::Io {
            path: repo_path.clone(),
            source,
        })?;
        let markdown = Markdown::try_from(absolute.as_path()).map_err(|error| {
            ResearchError::Frontmatter {
                path: repo_path.clone(),
                message: self.scrub(&error.to_string()),
            }
        })?;

        let mut frontmatter: Map<String, Value> = markdown
            .frontmatter()
            .as_map()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        let declared_schema = frontmatter.remove("$schema");
        let frontmatter = Value::Object(frontmatter);

        let mut findings = Findings::new(repo_path.clone());
        let mut loaded = Loaded {
            path: repo_path.clone(),
            frontmatter,
            record: None,
            diagnostics: Vec::new(),
        };

        if !self.bind_schema(T::KIND, &absolute, declared_schema.as_ref(), &mut findings) {
            loaded.diagnostics = findings.finish();
            return Ok(loaded);
        }

        let effective = self.effective_schema(T::KIND, &markdown, &repo_path)?;
        let report = effective.validate(&loaded.frontmatter);
        for problem in &report.problems {
            let mut pointer = problem.path.clone();
            let mut rule = Rule::Schema;
            if problem.code == ValidationProblemCode::UnknownKey
                && let Some(key) = &problem.offending_property
            {
                if pointer.is_empty() {
                    rule = Rule::TopLevel;
                }
                pointer = format!("{pointer}/{}", escape_pointer(key));
            }
            findings.push(rule, pointer, None, self.scrub(&problem.message));
        }

        if report.problems.is_empty() && version_supported(T::KIND, &loaded.frontmatter, &mut findings) {
            loaded.record = deserialize::<T>(&loaded.frontmatter, &mut findings);
        }
        loaded.diagnostics = findings.finish();
        Ok(loaded)
    }

    fn effective_schema(
        &self,
        kind: ContractKind,
        markdown: &Markdown,
        path: &RepoPath,
    ) -> Result<Arc<EffectiveSchema>, ResearchError> {
        let mut cache = self.effective.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(schema) = cache.get(&kind) {
            return Ok(Arc::clone(schema));
        }
        let resolution = |message: String| ResearchError::SchemaResolution {
            path: path.clone(),
            message: self.scrub(&message),
        };
        let schema = self
            .schemas
            .effective_for(markdown)
            .map_err(|error| resolution(error.to_string()))?
            .ok_or_else(|| resolution("the bound schema resolved to nothing".to_string()))?;
        let schema = Arc::new(schema);
        cache.insert(kind, Arc::clone(&schema));
        Ok(schema)
    }

    /// Resolves an authored file reference relative to `source`.
    pub(crate) fn resolve_reference(&self, source: &Path, reference: &str) -> Option<PathBuf> {
        let reference = FileReference::new(reference).ok()?;
        let context = self.context.for_source(source.to_path_buf());
        reference
            .resolve_in_context(&context)
            .ok()
            .flatten()
            .map(|path| normalize(&path))
    }

    /// Checks that `$schema` names the shipped contract schema for `kind`.
    /// A file bound elsewhere (or not at all) would validate against the
    /// wrong contract, so schema validation is skipped for it.
    fn bind_schema(
        &self,
        kind: ContractKind,
        source: &Path,
        declared: Option<&Value>,
        findings: &mut Findings,
    ) -> bool {
        let expected = kind.schema_path(&self.workspace);
        let expected_display = self
            .workspace
            .repo_path(&expected)
            .map(|path| path.to_string())
            .unwrap_or_default();
        let Some(declared) = declared else {
            findings.push(
                Rule::SchemaBinding,
                "",
                None,
                format!("no `$schema`; a research file must bind {expected_display}"),
            );
            return false;
        };
        let Some(reference) = declared.as_str() else {
            findings.push(
                Rule::SchemaBinding,
                "/$schema",
                None,
                format!("`$schema` must be a file reference to {expected_display}"),
            );
            return false;
        };
        let resolved = self.resolve_reference(source, reference);
        let bound = resolved.is_some_and(|resolved| same_file(&resolved, &expected));
        if !bound {
            findings.push(
                Rule::SchemaBinding,
                "/$schema",
                None,
                format!("`$schema: {reference}` does not resolve to {expected_display}"),
            );
        }
        bound
    }

    fn absolute(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace.repo_root().join(path)
        }
    }

    /// Removes the host's repository-root spelling from third-party messages.
    fn scrub(&self, message: &str) -> String {
        let root = self.workspace.repo_root().display().to_string();
        let mut scrubbed = message.replace(&format!("{root}{}", std::path::MAIN_SEPARATOR), "");
        scrubbed = scrubbed.replace(&root, ".");
        scrubbed
    }
}

fn same_file(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => normalize(left) == normalize(right),
    }
}

/// SR-VERSION: the version key must be the integer 1, independently of the
/// schema literal (Darkmatter coerces `"1"` to `1`).
fn version_supported(kind: ContractKind, frontmatter: &Value, findings: &mut Findings) -> bool {
    let key = kind.version_key();
    let value = &frontmatter[key];
    if value.as_u64() == Some(SUPPORTED_SCHEMA_VERSION) {
        return true;
    }
    let shown = if value.is_null() { "missing".to_string() } else { value.to_string() };
    findings.push(
        Rule::Version,
        format!("/{key}"),
        None,
        format!("unsupported {key} {shown}; this build reads only version {SUPPORTED_SCHEMA_VERSION}"),
    );
    false
}

/// Typed deserialization of the authored frontmatter. Darkmatter coerces
/// scalar types during validation; this pass sees the authored values, so a
/// quoted number or an unquoted version fails here (SR-STRICT-SCALARS).
fn deserialize<T: DeserializeOwned>(frontmatter: &Value, findings: &mut Findings) -> Option<T> {
    match serde_path_to_error::deserialize::<_, T>(frontmatter.clone()) {
        Ok(record) => Some(record),
        Err(error) => {
            let pointer = pointer_for(error.path());
            let depth = error.path().iter().count();
            let message = error.inner().to_string();
            let rule = if message.starts_with("invalid type") {
                Rule::StrictScalars
            } else if message.starts_with("unknown field") && depth <= 1 {
                Rule::TopLevel
            } else {
                Rule::Schema
            };
            findings.push(rule, pointer, None, format!("typed load: {message}"));
            None
        }
    }
}

fn pointer_for(path: &serde_path_to_error::Path) -> String {
    use serde_path_to_error::Segment;
    let mut pointer = String::new();
    for segment in path.iter() {
        match segment {
            Segment::Seq { index } => pointer.push_str(&format!("/{index}")),
            Segment::Map { key } => pointer.push_str(&format!("/{}", escape_pointer(key))),
            Segment::Enum { variant } => pointer.push_str(&format!("/{}", escape_pointer(variant))),
            Segment::Unknown => {}
        }
    }
    pointer
}

fn escape_pointer(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}
