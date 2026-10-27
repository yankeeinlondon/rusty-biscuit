//! Global catalogs: the one declaration of each global's shape.
//!
//! A catalog is a YAML document with `kind: schema`, an optional
//! `description`, and a `$schema` mapping from each global's name to its type,
//! usually a named import from a types-only partial:
//!
//! ```yaml
//! kind: schema
//! description: the shapes of every Darkmatter global
//! $schema:
//!     fm: fm@./partials/fm.yaml -> the frontmatter
//!     env: object -> environment variables
//! ```
//!
//! Darkmatter's catalog, `darkmatter/schemas/darkmatter.yaml`, is compiled in
//! with its partials and resolved through [`GlobalCatalog::darkmatter`]. The
//! runtime baseline is its resolved `fm` definition, the `ctx.*` descriptor
//! catalog projects its `ctx` declaration, and `current` declares the same
//! shape. A host declares its own globals in a catalog of the same format and
//! loads it with [`GlobalCatalog::from_documents`] and its own
//! [`GlobalRequirement`]s.
//!
//! A catalog is registered explicitly, never discovered: the standalone
//! schema loader refuses its envelope, so a document `$schema` that reaches a
//! catalog by any path fails rather than applying its globals as frontmatter
//! properties, and trigger discovery never claims it. Shape is separate from
//! availability: which events may read a global is the host's
//! [`BindingView`](crate::markdown::compose::expression::BindingView).
//!
//! The loader is strict. Every required global must be declared exactly once
//! with a non-null, non-empty, resolvable definition of the required kind; an
//! undeclared name, a duplicate key, or trailing YAML is a [`CatalogError`]
//! naming the field. A global typed by a recursive named type is of the
//! object kinds when the type's body is an object; it keeps the type as a
//! named reference into [`GlobalDeclaration::definitions`].

use std::borrow::Cow;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use indexmap::IndexMap;
use serde_yaml_ng::{Mapping, Value as YamlValue};
use thiserror::Error;

use super::embedded::{self, EmbeddedDocument};
use super::errors::SchemaError;
use super::resolve::{self, describe_yaml};
use super::{DefinitionOuter, DefinitionTable, PropertyAtom, PropertyDef, SchemaShape, SimplifiedSchema, SimplifiedType, TypeExpr};

/// Virtual path of Darkmatter's global catalog.
pub const DARKMATTER_CATALOG_PATH: &str = "darkmatter/schemas/darkmatter.yaml";

/// What a global's resolved definition must be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalKind {
    /// A non-empty inline object shape with declared members.
    Shape,
    /// An open object: the `object` primitive or a non-empty shape.
    OpenObject,
}

/// One global a catalog must declare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalRequirement {
    pub name: Cow<'static, str>,
    pub kind: GlobalKind,
    /// A global whose resolved shape this one must equal.
    pub same_as: Option<Cow<'static, str>>,
}

impl GlobalRequirement {
    /// A global whose definition is a non-empty object shape.
    #[must_use]
    pub const fn shape(name: &'static str) -> Self {
        Self { name: Cow::Borrowed(name), kind: GlobalKind::Shape, same_as: None }
    }

    /// [`Self::shape`] for a name read at runtime, such as one a discovered
    /// host catalog declares.
    #[must_use]
    pub fn owned_shape(name: String) -> Self {
        Self { name: Cow::Owned(name), kind: GlobalKind::Shape, same_as: None }
    }
}

/// Darkmatter's globals, in catalog order.
pub const DARKMATTER_GLOBALS: &[GlobalRequirement] = &[
    GlobalRequirement::shape("fm"),
    GlobalRequirement::shape("ctx"),
    GlobalRequirement { name: Cow::Borrowed("current"), kind: GlobalKind::Shape, same_as: Some(Cow::Borrowed("ctx")) },
    GlobalRequirement { name: Cow::Borrowed("env"), kind: GlobalKind::OpenObject, same_as: None },
    GlobalRequirement::shape("err"),
    GlobalRequirement::shape("timing"),
];

/// Why a catalog field failed to load.
#[derive(Debug, Error)]
pub enum CatalogErrorReason {
    /// The document is not valid YAML: a syntax error, a duplicate key, or a
    /// second document.
    #[error("is not valid YAML: {0}")]
    Yaml(String),
    #[error("is required")]
    Missing,
    #[error("is null")]
    Null,
    #[error("must be {expected}, found {found}")]
    WrongType { expected: &'static str, found: &'static str },
    #[error("is empty")]
    Empty,
    #[error("must be `schema`, found `{0}`")]
    WrongKind(String),
    /// A key the catalog format or its requirements do not declare.
    #[error("is not a field or global this catalog declares")]
    Unknown,
    /// The definition did not parse or resolve.
    #[error("does not resolve: {0}")]
    Resolve(Box<SchemaError>),
    /// A [`GlobalRequirement::same_as`] mismatch.
    #[error("must declare the same shape as `{0}`")]
    Mismatch(Cow<'static, str>),
}

/// A typed, field-located catalog load failure.
#[derive(Debug, Error)]
#[error("global catalog `{catalog}`: `{field}` {reason}")]
pub struct CatalogError {
    /// The catalog's path, virtual or on disk.
    pub catalog: String,
    /// The failing field: `kind`, `description`, `$schema`, or
    /// `$schema.<global>`.
    pub field: String,
    pub reason: CatalogErrorReason,
}

/// One global's resolved declaration.
///
/// A global typed by a recursive named type keeps its definition as a named
/// reference ([`TypeExpr::Ref`]) into [`Self::definitions`]; its description,
/// like every whole-type description, names the type instead of expanding it.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalDeclaration {
    name: String,
    definition: PropertyDef,
    definitions: Arc<DefinitionTable>,
}

#[allow(missing_docs)]
impl GlobalDeclaration {
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The resolved definition, every acyclic import expanded; a recursive
    /// type stays a reference into [`Self::definitions`].
    pub fn definition(&self) -> &PropertyDef {
        &self.definition
    }

    /// The recursive definitions [`Self::definition`]'s references point
    /// into; empty for an acyclic global.
    pub fn definitions(&self) -> &DefinitionTable {
        &self.definitions
    }

    /// The catalog's `-> description` of the global.
    pub fn description(&self) -> Option<&str> {
        match &self.definition {
            PropertyDef::Single(atom) => atom.description.as_deref(),
            PropertyDef::Union(_) => None,
        }
    }

    /// The global's member shape; `None` for an open `object`. A global
    /// typed by a recursive object type yields that type's body, whose
    /// members still name the type rather than expanding it.
    pub fn shape(&self) -> Option<&SchemaShape> {
        match &resolved_atom(&self.definition, &self.definitions)?.ty {
            TypeExpr::InlineObject(shape) => Some(shape),
            _ => None,
        }
    }

    /// The declared member's definition.
    pub fn member(&self, name: &str) -> Option<&PropertyDef> {
        self.shape().and_then(|shape| shape.properties.get(name))
    }
}

/// A loaded catalog: each required global's resolved declaration, in
/// requirement order.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalCatalog {
    description: Option<String>,
    globals: IndexMap<String, GlobalDeclaration>,
}

/// Where a catalog's imports resolve.
enum Source<'a> {
    Embedded(&'a [EmbeddedDocument<'a>]),
    Disk {
        dir: &'a Path,
        context: &'a biscuit_file::FileResolutionContext,
    },
}

impl GlobalCatalog {
    /// Darkmatter's compiled-in catalog, resolved once per process from the
    /// embedded bundle. It reads no disk or network and does not depend on the
    /// launch directory.
    ///
    /// ## Panics
    ///
    /// When the compiled-in catalog does not load. This is a library bug, not
    /// an author error; a test loads it.
    pub fn darkmatter() -> &'static GlobalCatalog {
        static CATALOG: OnceLock<GlobalCatalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            Self::load_embedded(DARKMATTER_CATALOG_PATH, embedded::SHIPPED, DARKMATTER_GLOBALS)
                .unwrap_or_else(|error| panic!("the embedded global catalog must load: {error}"))
        })
    }

    /// Loads the catalog at virtual path `path` from `documents`, pairs of
    /// repository-relative virtual path and text. Imports resolve lexically
    /// among `documents` only, never on the disk.
    ///
    /// ## Errors
    ///
    /// A [`CatalogError`] naming the first failing field.
    pub fn from_documents(
        path: &str,
        documents: &[(&str, &str)],
        requirements: &[GlobalRequirement],
    ) -> Result<Self, CatalogError> {
        let table: Vec<EmbeddedDocument<'_>> =
            documents.iter().map(|&(path, text)| EmbeddedDocument { path, text }).collect();
        Self::load_embedded(path, &table, requirements)
    }

    /// Loads the catalog file at `path`, resolving its imports from the
    /// file's directory through `context`.
    ///
    /// ## Errors
    ///
    /// A [`CatalogError`] naming the first failing field; an unreadable file
    /// is a [`CatalogErrorReason::Resolve`] on the whole document.
    pub fn from_file(
        path: &Path,
        requirements: &[GlobalRequirement],
        context: &biscuit_file::FileResolutionContext,
    ) -> Result<Self, CatalogError> {
        let catalog = path.display().to_string();
        let text = std::fs::read_to_string(path).map_err(|source| CatalogError {
            catalog: catalog.clone(),
            field: String::new(),
            reason: CatalogErrorReason::Resolve(Box::new(SchemaError::Io { path: path.to_path_buf(), source })),
        })?;
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        Self::load(&catalog, &text, &Source::Disk { dir, context }, requirements)
    }

    /// Loads a host catalog file found by discovery, whose requirements the
    /// reader does not know: every global it declares must be a non-empty
    /// object shape, and none may be a reserved root or Darkmatter global.
    /// Editors use this; a host loads its own catalog with
    /// [`Self::from_documents`] and its exact [`GlobalRequirement`]s.
    ///
    /// ## Errors
    ///
    /// As [`Self::from_file`]; a reserved or Darkmatter name is
    /// [`CatalogErrorReason::Unknown`].
    pub fn from_host_file(path: &Path, context: &biscuit_file::FileResolutionContext) -> Result<Self, CatalogError> {
        let text = std::fs::read_to_string(path).map_err(|source| CatalogError {
            catalog: path.display().to_string(),
            field: String::new(),
            reason: CatalogErrorReason::Resolve(Box::new(SchemaError::Io { path: path.to_path_buf(), source })),
        })?;
        Self::from_host_text(path, &text, context)
    }

    /// [`Self::from_host_file`] over `text`, the bytes the caller already
    /// read from `path`; `path` names the catalog and locates its imports,
    /// and is not read again. Lifecycle discovery builds a profile from the
    /// same bytes it hashed against the artifact.
    fn from_host_text(
        path: &Path,
        text: &str,
        context: &biscuit_file::FileResolutionContext,
    ) -> Result<Self, CatalogError> {
        let catalog = path.display().to_string();
        // A permissive probe for the declared names only: a document it
        // cannot read, or a non-string key it skips, still fails in `load`,
        // which validates the whole document with a located error.
        let declared: Vec<String> = serde_yaml_ng::from_str::<YamlValue>(text)
            .ok()
            .and_then(|root| root.get("$schema").and_then(YamlValue::as_mapping).cloned())
            .map(|map| map.keys().filter_map(|key| key.as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        if let Some(name) = declared
            .iter()
            .find(|name| DARKMATTER_GLOBALS.iter().any(|global| global.name == name.as_str()))
        {
            return Err(CatalogError {
                catalog,
                field: format!("$schema.{name}"),
                reason: CatalogErrorReason::Unknown,
            });
        }
        let requirements: Vec<GlobalRequirement> = declared.into_iter().map(GlobalRequirement::owned_shape).collect();
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        Self::load(&catalog, text, &Source::Disk { dir, context }, &requirements)
    }

    fn load_embedded(
        path: &str,
        documents: &[EmbeddedDocument<'_>],
        requirements: &[GlobalRequirement],
    ) -> Result<Self, CatalogError> {
        let Some(document) = embedded::find(documents, path) else {
            return Err(CatalogError {
                catalog: path.to_string(),
                field: String::new(),
                reason: CatalogErrorReason::Missing,
            });
        };
        Self::load(path, document.text, &Source::Embedded(documents), requirements)
    }

    fn load(
        catalog: &str,
        text: &str,
        source: &Source<'_>,
        requirements: &[GlobalRequirement],
    ) -> Result<Self, CatalogError> {
        let fail = |field: &str, reason| CatalogError { catalog: catalog.to_string(), field: field.to_string(), reason };
        // Parsing to a `Value` rejects duplicate keys and a second document
        // before any map could merge them.
        let root: YamlValue = serde_yaml_ng::from_str(text).map_err(|error| fail("", CatalogErrorReason::Yaml(error.to_string())))?;
        let root = expect_mapping(&root, "").map_err(|reason| fail("", reason))?;
        for key in root.keys() {
            match key.as_str() {
                Some("kind" | "description" | "$schema") => {}
                Some(other) => return Err(fail(other, CatalogErrorReason::Unknown)),
                None => return Err(fail("", CatalogErrorReason::WrongType { expected: "string keys", found: describe_yaml(key) })),
            }
        }
        match required(root, "kind").map_err(|reason| fail("kind", reason))? {
            YamlValue::String(kind) if kind.is_empty() => return Err(fail("kind", CatalogErrorReason::Empty)),
            YamlValue::String(kind) if kind != "schema" => return Err(fail("kind", CatalogErrorReason::WrongKind(kind.clone()))),
            YamlValue::String(_) => {}
            other => return Err(fail("kind", wrong_type("a string", other))),
        }
        let description = match root.get("description") {
            None => None,
            Some(YamlValue::Null) => return Err(fail("description", CatalogErrorReason::Null)),
            Some(YamlValue::String(text)) if text.trim().is_empty() => return Err(fail("description", CatalogErrorReason::Empty)),
            Some(YamlValue::String(text)) => Some(text.clone()),
            Some(other) => return Err(fail("description", wrong_type("a string", other))),
        };
        let declarations = required(root, "$schema").map_err(|reason| fail("$schema", reason))?;
        let declarations = expect_mapping(declarations, "a mapping of globals").map_err(|reason| fail("$schema", reason))?;
        if declarations.is_empty() {
            return Err(fail("$schema", CatalogErrorReason::Empty));
        }
        for key in declarations.keys() {
            let known = key.as_str().is_some_and(|name| requirements.iter().any(|r| r.name == name));
            if !known {
                let field = format!("$schema.{}", key.as_str().unwrap_or("?"));
                return Err(fail(&field, CatalogErrorReason::Unknown));
            }
        }

        let mut globals = IndexMap::with_capacity(requirements.len());
        for requirement in requirements {
            let field = format!("$schema.{}", requirement.name);
            let value = required(declarations, &requirement.name).map_err(|reason| fail(&field, reason))?;
            let (definition, definitions) =
                resolve_declaration(catalog, &requirement.name, value, source).map_err(|reason| fail(&field, reason))?;
            check_kind(requirement, &definition, &definitions).map_err(|reason| fail(&field, reason))?;
            if let Some(other) = &requirement.same_as {
                let same = globals
                    .get(other.as_ref())
                    .is_some_and(|earlier: &GlobalDeclaration| same_shape(&earlier.definition, &definition));
                if !same {
                    return Err(fail(&field, CatalogErrorReason::Mismatch(other.clone())));
                }
            }
            globals.insert(
                requirement.name.to_string(),
                GlobalDeclaration { name: requirement.name.to_string(), definition, definitions },
            );
        }
        Ok(Self { description, globals })
    }

    /// The catalog's `description`.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The declaration of `name`.
    pub fn global(&self, name: &str) -> Option<&GlobalDeclaration> {
        self.globals.get(name)
    }

    /// Every declaration, in requirement order.
    pub fn globals(&self) -> impl Iterator<Item = &GlobalDeclaration> {
        self.globals.values()
    }

    /// The resolved `fm` definition as a document schema: the built-in
    /// baseline when this is Darkmatter's catalog. `None` for a catalog that
    /// declares no `fm`.
    pub fn baseline(&self) -> Option<SimplifiedSchema> {
        self.global("fm")?.shape().cloned().map(SimplifiedSchema::Single)
    }
}

/// A catalog an [`EventRegistry`](crate::lifecycle::registry::EventRegistry)
/// registers: a compiled host's process-lifetime catalog, borrowed, or one
/// loaded at runtime and owned by everything that shares it. Equality
/// compares the catalogs, not where they live.
#[derive(Debug, Clone)]
pub enum CatalogHandle {
    Static(&'static GlobalCatalog),
    Shared(Arc<GlobalCatalog>),
}

impl std::ops::Deref for CatalogHandle {
    type Target = GlobalCatalog;

    fn deref(&self) -> &GlobalCatalog {
        match self {
            Self::Static(catalog) => catalog,
            Self::Shared(catalog) => catalog,
        }
    }
}

impl PartialEq for CatalogHandle {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl From<&'static GlobalCatalog> for CatalogHandle {
    fn from(catalog: &'static GlobalCatalog) -> Self {
        Self::Static(catalog)
    }
}

impl From<Arc<GlobalCatalog>> for CatalogHandle {
    fn from(catalog: Arc<GlobalCatalog>) -> Self {
        Self::Shared(catalog)
    }
}

/// The host catalogs in a document's schema roots: every top-level
/// `.yaml`/`.yml` file claiming the catalog envelope, other than one
/// declaring Darkmatter's own globals, loaded with
/// [`GlobalCatalog::from_host_file`]. A file name in an earlier root shadows
/// the same name in later ones. Passive: files are read, nothing runs.
///
/// A catalog that fails to load is returned as its error so a caller can
/// report it; the others still load.
pub fn host_catalogs(context: &biscuit_file::FileResolutionContext) -> Vec<Result<GlobalCatalog, CatalogError>> {
    let Ok(roots) = super::roots::SchemaRoots::for_document(context) else {
        return Vec::new();
    };
    let mut seen = std::collections::BTreeSet::new();
    let mut catalogs = Vec::new();
    for root in roots.search_paths() {
        let Ok(entries) = std::fs::read_dir(root) else { continue };
        let mut files: Vec<_> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "yaml" || ext == "yml"))
            .collect();
        files.sort();
        for path in files {
            let Some(name) = path.file_name().map(|name| name.to_os_string()) else { continue };
            if !seen.insert(name) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            if !is_global_catalog(&text) || declares_fm(&text) {
                continue;
            }
            catalogs.push(GlobalCatalog::from_host_file(&path, context));
        }
    }
    catalogs
}

/// Why a host function catalog in a schema root did not load.
#[derive(Debug, Clone, PartialEq)]
pub struct HostFunctionCatalogError {
    pub path: std::path::PathBuf,
    pub message: String,
}

impl std::fmt::Display for HostFunctionCatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "function catalog `{}`: {}", self.path.display(), self.message)
    }
}

impl std::error::Error for HostFunctionCatalogError {}

/// The host function catalogs in a document's schema roots, each as an
/// analysis-only [`HostFunctions`](crate::markdown::compose::expression::HostFunctions)
/// set: every top-level `.yaml`/`.yml` file claiming the function-catalog
/// envelope ([`is_host_function_catalog`]), with its facts read from
/// `<stem>.facts.json` beside it. A file name in an earlier root shadows the
/// same name in later ones. Passive: files are read, nothing runs, and no
/// host handler exists in the result.
///
/// A catalog that does not load, or that names a built-in function, is
/// returned as its error; a missing or bad facts artifact is not an error but
/// makes the affected functions incomplete.
///
/// The process keeps one result per catalog path: the catalog's content
/// hash, the artifact as read, and the loaded set or its error. An unchanged
/// revision reuses that result, an error included; a changed catalog or
/// artifact replaces it, so a replaced catalog is freed once no caller holds
/// a set, clone, or scope guard from it. A set obtained earlier keeps
/// working after its path's entry is replaced.
pub fn host_function_catalogs(
    context: &biscuit_file::FileResolutionContext,
) -> Vec<Result<crate::markdown::compose::expression::HostFunctions, HostFunctionCatalogError>> {
    let Ok(roots) = super::roots::SchemaRoots::for_document(context) else {
        return Vec::new();
    };
    let mut seen = std::collections::BTreeSet::new();
    let mut sets = Vec::new();
    for root in roots.search_paths() {
        let Ok(entries) = std::fs::read_dir(root) else { continue };
        let mut files: Vec<_> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "yaml" || ext == "yml"))
            .collect();
        files.sort();
        for path in files {
            let Some(name) = path.file_name().map(|name| name.to_os_string()) else { continue };
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            if !is_host_function_catalog(&text) || !seen.insert(name) {
                continue;
            }
            sets.push(host_function_set(&path, &text));
        }
    }
    sets
}

/// The artifact's state, as part of a cache key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ArtifactKey {
    Absent,
    Unreadable(String),
    Text(String),
}

impl ArtifactKey {
    /// The companion file at `path` as a reader receives it.
    fn read(path: &Path) -> Self {
        Self::from_read(std::fs::read(path))
    }

    /// The companion file as `read` returned its bytes.
    fn from_read(read: std::io::Result<Vec<u8>>) -> Self {
        match read {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::Absent,
            Err(error) => Self::Unreadable(error.to_string()),
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(text) => Self::Text(text),
                Err(_) => Self::Unreadable("the file is not UTF-8 text".to_string()),
            },
        }
    }

    fn artifact(&self) -> crate::markdown::compose::expression::host::FactsArtifact<'_> {
        use crate::markdown::compose::expression::host::FactsArtifact;
        match self {
            Self::Absent => FactsArtifact::Absent,
            Self::Unreadable(error) => FactsArtifact::Unreadable(error),
            Self::Text(text) => FactsArtifact::Text(text),
        }
    }
}

type HostFunctionCacheValue = Result<crate::markdown::compose::expression::HostFunctions, HostFunctionCatalogError>;

/// The latest revision `host_function_set` processed for one catalog path.
struct HostFunctionEntry {
    catalog_hash: String,
    artifact: ArtifactKey,
    result: HostFunctionCacheValue,
}

type HostFunctionCache = std::collections::HashMap<std::path::PathBuf, HostFunctionEntry>;

fn host_function_cache() -> std::sync::MutexGuard<'static, HostFunctionCache> {
    static CACHE: std::sync::LazyLock<std::sync::Mutex<HostFunctionCache>> = std::sync::LazyLock::new(Default::default);
    CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn host_function_set(path: &Path, text: &str) -> HostFunctionCacheValue {
    use crate::markdown::compose::expression::host::facts_artifact_path;
    use crate::markdown::compose::expression::{HostFunctionCatalog, HostFunctions};

    let catalog_hash = biscuit_hash::blake3_hash(text);
    let artifact = ArtifactKey::read(&facts_artifact_path(path));
    // Lookup, build, and replacement happen under one lock, so an unchanged
    // revision is never built twice and each path keeps one result.
    let mut cache = host_function_cache();
    if let Some(cached) = cache.get(path)
        && cached.catalog_hash == catalog_hash
        && cached.artifact == artifact
    {
        return cached.result.clone();
    }
    discovery_probe::note_host_function_build();
    let fail = |message: String| HostFunctionCatalogError { path: path.to_path_buf(), message };
    let result = HostFunctionCatalog::load(text, path).map_err(|error| fail(error.to_string())).and_then(|catalog| {
        HostFunctions::analysis_only(catalog, artifact.artifact()).map_err(|error| fail(error.to_string()))
    });
    cache.insert(path.to_path_buf(), HostFunctionEntry { catalog_hash, artifact, result: result.clone() });
    result
}

/// The host lifecycle profile recorded beside one host catalog in a
/// document's schema roots.
#[derive(Debug, Clone)]
pub struct HostLifecycleDiscovery {
    /// The host catalog the artifact belongs to.
    pub catalog: std::path::PathBuf,
    /// The rebuilt profile, or why the artifact
    /// (`<stem>.lifecycle.json`) could not be trusted: absent, unreadable,
    /// stale against the catalog, or invalid.
    pub profile: Result<Arc<crate::lifecycle::host_profile::HostLifecycleProfile>, String>,
}

/// The host lifecycle profiles of a document's schema roots, discovered
/// lazily: for each host catalog [`host_catalogs`] loads, in the same order
/// and with the same shadowing, the profile its `<stem>.lifecycle.json`
/// records ([`crate::lifecycle::host_profile`]). A catalog that is
/// unreadable or does not load is skipped here; [`host_catalogs`] reports
/// it. Passive: files are read, nothing runs.
///
/// Each root's directory is listed when the iterator reaches it, and a
/// candidate's catalog and artifact are read only when it is visited, so a
/// caller that stops at the first usable profile reads nothing after it. A
/// file name is shadowed by an earlier root's file of the same name even
/// when that earlier file is unusable.
///
/// The process keeps one observation per catalog path: the catalog's
/// content hash, the artifact as read, and the profile, the reason the
/// artifact was refused, or that the file is not a host catalog. An
/// unchanged revision is served from it without being parsed or rebuilt; a
/// changed one replaces it, so an obsolete profile is released once no
/// caller holds it. A refusal retains no catalog or registry. A visited path
/// whose catalog is gone, unreadable, or no longer loads loses its entry.
pub fn host_lifecycle_profiles(context: &biscuit_file::FileResolutionContext) -> HostLifecycleProfiles<'_> {
    let roots = super::roots::SchemaRoots::for_document(context)
        .map(|roots| roots.search_paths().to_vec())
        .unwrap_or_default();
    HostLifecycleProfiles {
        context,
        roots: roots.into_iter(),
        files: Vec::new().into_iter(),
        seen: std::collections::BTreeSet::new(),
    }
}

/// The lazy iterator [`host_lifecycle_profiles`] returns.
pub struct HostLifecycleProfiles<'c> {
    context: &'c biscuit_file::FileResolutionContext,
    roots: std::vec::IntoIter<std::path::PathBuf>,
    /// The current root's remaining candidates, sorted.
    files: std::vec::IntoIter<std::path::PathBuf>,
    /// File names already claimed by a visited candidate.
    seen: std::collections::BTreeSet<std::ffi::OsString>,
}

impl Iterator for HostLifecycleProfiles<'_> {
    type Item = HostLifecycleDiscovery;

    fn next(&mut self) -> Option<HostLifecycleDiscovery> {
        loop {
            let Some(path) = self.files.next() else {
                let root = self.roots.next()?;
                self.files = list_root(&root).into_iter();
                continue;
            };
            let Some(name) = path.file_name() else { continue };
            // Claimed before the candidate is tried, so an unusable file
            // still shadows its name in later roots.
            if !self.seen.insert(name.to_os_string()) {
                continue;
            }
            if let Some(found) = observe_candidate(&path, self.context) {
                return Some(found);
            }
        }
    }
}

/// What discovery last observed at one catalog path.
struct Observation {
    catalog_hash: String,
    artifact: ArtifactKey,
    outcome: Observed,
}

enum Observed {
    /// The file is not a host catalog, or it declares Darkmatter's own
    /// globals.
    NotHostCatalog,
    Profile(Result<Arc<crate::lifecycle::host_profile::HostLifecycleProfile>, String>),
}

type ProfileCache = std::collections::HashMap<std::path::PathBuf, Observation>;

fn profile_cache() -> std::sync::MutexGuard<'static, ProfileCache> {
    static CACHE: std::sync::LazyLock<std::sync::Mutex<ProfileCache>> = std::sync::LazyLock::new(Default::default);
    CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The `.yaml`/`.yml` files directly in `root`, sorted. A cached path in
/// `root` that is no longer listed loses its entry: the listing alone shows
/// it is gone.
fn list_root(root: &Path) -> Vec<std::path::PathBuf> {
    // An unreadable root has no candidates, as a missing one has none.
    let mut files: Vec<_> = std::fs::read_dir(root)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "yaml" || ext == "yml"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    profile_cache().retain(|cached, _| cached.parent() != Some(root) || files.binary_search(cached).is_ok());
    files
}

/// The profile of the candidate at `path`, from its current catalog and
/// artifact bytes; `None` when it is not a usable host catalog.
fn observe_candidate(
    path: &Path,
    context: &biscuit_file::FileResolutionContext,
) -> Option<HostLifecycleDiscovery> {
    use crate::lifecycle::host_profile::{HostLifecycleProfile, lifecycle_artifact_path};

    let Some(text) = discovery_probe::read(path).ok().and_then(|bytes| String::from_utf8(bytes).ok()) else {
        profile_cache().remove(path);
        return None;
    };
    let catalog_hash = biscuit_hash::blake3_hash(&text);
    let artifact = ArtifactKey::from_read(discovery_probe::read(&lifecycle_artifact_path(path)));
    // Lookup, build, and replacement happen under one lock, so an unchanged
    // revision is never built twice and each path keeps one observation.
    let mut cache = profile_cache();
    if let Some(cached) = cache.get(path)
        && cached.catalog_hash == catalog_hash
        && cached.artifact == artifact
    {
        return match &cached.outcome {
            Observed::NotHostCatalog => None,
            Observed::Profile(profile) => {
                Some(HostLifecycleDiscovery { catalog: path.to_path_buf(), profile: profile.clone() })
            }
        };
    }
    if !is_global_catalog(&text) || declares_fm(&text) {
        cache.insert(path.to_path_buf(), Observation { catalog_hash, artifact, outcome: Observed::NotHostCatalog });
        return None;
    }
    let Ok(catalog) = GlobalCatalog::from_host_text(path, &text, context) else {
        cache.remove(path);
        return None;
    };
    discovery_probe::note_build();
    let profile =
        HostLifecycleProfile::from_artifact(Arc::new(catalog), path, &text, artifact.artifact()).map(Arc::new);
    cache.insert(
        path.to_path_buf(),
        Observation { catalog_hash, artifact, outcome: Observed::Profile(profile.clone()) },
    );
    Some(HostLifecycleDiscovery { catalog: path.to_path_buf(), profile })
}

/// Test-only observation of schema-root discovery on the current thread:
/// which lifecycle catalog and artifact files it reads, how many lifecycle
/// profiles it builds, read failures to inject, and what the profile cache
/// holds for a path.
/// Compiled for this crate's tests and with the test-only `work-counters`
/// feature; production builds read the disk directly. The host function
/// cache's inspection items are crate-private and compiled for tests only.
#[cfg(any(test, feature = "work-counters"))]
pub mod discovery_probe {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    #[derive(Default)]
    struct State {
        builds: usize,
        #[cfg(test)]
        host_function_builds: usize,
        reads: Vec<PathBuf>,
        failures: HashMap<PathBuf, String>,
    }

    thread_local! {
        static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    }

    /// Observes discovery on this thread until dropped.
    pub struct DiscoveryProbe {
        // Thread-local state: the probe must stay on its thread.
        _not_send: std::marker::PhantomData<*const ()>,
    }

    impl DiscoveryProbe {
        /// Starts observing this thread's discovery.
        ///
        /// ## Panics
        ///
        /// When this thread already has a probe.
        #[must_use]
        pub fn install() -> Self {
            STATE.with_borrow_mut(|state| {
                assert!(state.is_none(), "one discovery probe per thread");
                *state = Some(State::default());
            });
            Self { _not_send: std::marker::PhantomData }
        }

        /// Makes each later discovery read of `path` fail with `message`.
        pub fn fail_reads_of(&self, path: &Path, message: &str) {
            with_state(|state| state.failures.insert(path.to_path_buf(), message.to_string()));
        }

        /// Lets discovery read `path` again.
        pub fn restore_reads_of(&self, path: &Path) {
            with_state(|state| state.failures.remove(path));
        }

        /// The profiles discovery built (successfully or not) so far.
        pub fn builds(&self) -> usize {
            with_state(|state| state.builds)
        }

        /// The host function sets `host_function_catalogs` loaded (successfully
        /// or not) so far; a cached result is not counted.
        #[cfg(test)]
        pub(super) fn host_function_builds(&self) -> usize {
            with_state(|state| state.host_function_builds)
        }

        /// The catalog and artifact files discovery read so far, in order,
        /// failed reads included; clears the record.
        pub fn take_reads(&self) -> Vec<PathBuf> {
            with_state(|state| std::mem::take(&mut state.reads))
        }
    }

    impl Drop for DiscoveryProbe {
        fn drop(&mut self) {
            STATE.with_borrow_mut(|state| *state = None);
        }
    }

    fn with_state<T>(f: impl FnOnce(&mut State) -> T) -> T {
        STATE.with_borrow_mut(|state| f(state.as_mut().expect("the probe is installed")))
    }

    /// What the process cache holds for one catalog path.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum CachedOutcome {
        NotHostCatalog,
        /// A profile, whose registry holds its catalog.
        Profile,
        /// A refused artifact's reason; nothing else is retained.
        Refused(String),
    }

    /// The cache's observation of `path`, if it holds one.
    pub fn cached_outcome(path: &Path) -> Option<CachedOutcome> {
        super::profile_cache().get(path).map(|cached| match &cached.outcome {
            super::Observed::NotHostCatalog => CachedOutcome::NotHostCatalog,
            super::Observed::Profile(Ok(_)) => CachedOutcome::Profile,
            super::Observed::Profile(Err(reason)) => CachedOutcome::Refused(reason.clone()),
        })
    }

    /// The facts artifact as the host function cache recorded it.
    #[cfg(test)]
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) enum CachedArtifact {
        Absent,
        Unreadable(String),
        Text(String),
    }

    /// The host function cache's entry for one catalog path.
    #[cfg(test)]
    #[derive(Debug, Clone)]
    pub(super) struct CachedHostFunctions {
        pub(super) catalog_hash: String,
        pub(super) artifact: CachedArtifact,
        /// The cached set's function names, or the cached load error.
        pub(super) result: Result<Vec<String>, super::HostFunctionCatalogError>,
    }

    /// The host function cache's entry for `path`, if it holds one. Holds
    /// nothing of the cached set alive.
    #[cfg(test)]
    pub(super) fn cached_host_functions(path: &Path) -> Option<CachedHostFunctions> {
        super::host_function_cache().get(path).map(|cached| CachedHostFunctions {
            catalog_hash: cached.catalog_hash.clone(),
            artifact: match &cached.artifact {
                super::ArtifactKey::Absent => CachedArtifact::Absent,
                super::ArtifactKey::Unreadable(reason) => CachedArtifact::Unreadable(reason.clone()),
                super::ArtifactKey::Text(text) => CachedArtifact::Text(text.clone()),
            },
            result: cached
                .result
                .as_ref()
                .map(|set| set.function_names().into_iter().map(str::to_string).collect())
                .map_err(Clone::clone),
        })
    }

    /// How many host function cache entries are for paths under `dir`.
    #[cfg(test)]
    pub(super) fn host_function_entries_under(dir: &Path) -> usize {
        super::host_function_cache().keys().filter(|path| path.starts_with(dir)).count()
    }

    pub(super) fn read(path: &Path) -> std::io::Result<Vec<u8>> {
        let injected = STATE.with_borrow_mut(|state| {
            let state = state.as_mut()?;
            state.reads.push(path.to_path_buf());
            state.failures.get(path).cloned()
        });
        match injected {
            Some(message) => Err(std::io::Error::other(message)),
            None => std::fs::read(path),
        }
    }

    pub(super) fn note_build() {
        STATE.with_borrow_mut(|state| {
            if let Some(state) = state {
                state.builds += 1;
            }
        });
    }

    pub(super) fn note_host_function_build() {
        #[cfg(test)]
        STATE.with_borrow_mut(|state| {
            if let Some(state) = state {
                state.host_function_builds += 1;
            }
        });
    }
}

#[cfg(not(any(test, feature = "work-counters")))]
mod discovery_probe {
    pub(super) fn read(path: &std::path::Path) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    pub(super) fn note_build() {}

    pub(super) fn note_host_function_build() {}
}

/// Whether `text` claims the function-catalog envelope: `kind: schema` with
/// a `categories` list and a `types` mapping. Passive: nothing is resolved.
#[must_use]
pub fn is_host_function_catalog(text: &str) -> bool {
    serde_yaml_ng::from_str::<YamlValue>(text).is_ok_and(|value| {
        value.as_mapping().is_some_and(|map| {
            map.get("kind").and_then(YamlValue::as_str) == Some("schema")
                && matches!(map.get("categories"), Some(YamlValue::Sequence(_)))
                && matches!(map.get("types"), Some(YamlValue::Mapping(_)))
        })
    })
}

fn declares_fm(text: &str) -> bool {
    serde_yaml_ng::from_str::<YamlValue>(text)
        .ok()
        .is_some_and(|root| root.get("$schema").and_then(|schema| schema.get("fm")).is_some())
}

/// Whether `text` claims the global-catalog envelope: `kind: schema` with a
/// `$schema` mapping and no `types`. Passive: nothing is resolved.
#[must_use]
pub fn is_global_catalog(text: &str) -> bool {
    serde_yaml_ng::from_str::<YamlValue>(text).is_ok_and(|value| {
        value.as_mapping().is_some_and(|map| {
            map.get("kind").and_then(YamlValue::as_str) == Some("schema")
                && matches!(map.get("$schema"), Some(YamlValue::Mapping(_)))
                && !map.contains_key("types")
        })
    })
}

fn wrong_type(expected: &'static str, found: &YamlValue) -> CatalogErrorReason {
    match found {
        YamlValue::Null => CatalogErrorReason::Null,
        other => CatalogErrorReason::WrongType { expected, found: describe_yaml(other) },
    }
}

fn expect_mapping<'v>(value: &'v YamlValue, expected: &'static str) -> Result<&'v Mapping, CatalogErrorReason> {
    match value {
        YamlValue::Mapping(map) => Ok(map),
        other => Err(wrong_type(if expected.is_empty() { "a mapping" } else { expected }, other)),
    }
}

/// The value of `key`; absence and an explicit null are distinct failures.
fn required<'v>(map: &'v Mapping, key: &str) -> Result<&'v YamlValue, CatalogErrorReason> {
    match map.get(key) {
        None => Err(CatalogErrorReason::Missing),
        Some(YamlValue::Null) => Err(CatalogErrorReason::Null),
        Some(value) => Ok(value),
    }
}

/// Parses and resolves one declaration on its own, so a failure names its
/// global. The definition's recursive references point into the returned
/// table.
fn resolve_declaration(
    catalog: &str,
    name: &str,
    value: &YamlValue,
    source: &Source<'_>,
) -> Result<(PropertyDef, Arc<DefinitionTable>), CatalogErrorReason> {
    match value {
        YamlValue::String(text) if text.trim().is_empty() => return Err(CatalogErrorReason::Empty),
        YamlValue::Mapping(map) if map.is_empty() => return Err(CatalogErrorReason::Empty),
        YamlValue::String(_) | YamlValue::Mapping(_) => {}
        other => return Err(wrong_type("a type expression or mapping", other)),
    }
    let mut single = Mapping::new();
    single.insert(YamlValue::String(name.to_string()), value.clone());
    let single = YamlValue::Mapping(single);
    let resolved = match source {
        Source::Embedded(documents) => {
            resolve::prepare_embedded_catalog_declaration(&single, catalog, documents).map(|(resolved, _)| resolved)
        }
        Source::Disk { dir, context } => resolve::resolve_catalog_declaration(&single, dir, context),
    }
    .map_err(|error| CatalogErrorReason::Resolve(Box::new(error)))?;
    match resolved.simplified {
        Some(SimplifiedSchema::Single(mut shape)) => shape
            .properties
            .shift_remove(name)
            .map(|definition| (definition, resolved.definitions))
            .ok_or(CatalogErrorReason::Missing),
        _ => Err(CatalogErrorReason::WrongType { expected: "a single definition", found: "a union" }),
    }
}

/// The single non-array atom `definition` is, looking through a reference to
/// a recursive type to that type's body; `None` for a union or an array.
///
/// Alias chains between definitions are acyclic (admission rejects a cycle
/// that never descends into a value), and the walk stops after as many steps
/// as the table has entries in any case.
fn resolved_atom<'a>(definition: &'a PropertyDef, definitions: &'a DefinitionTable) -> Option<&'a PropertyAtom> {
    let mut current = definition;
    for _ in 0..=definitions.len() {
        let PropertyDef::Single(atom) = current else {
            return None;
        };
        if atom.is_array {
            return None;
        }
        match &atom.ty {
            TypeExpr::Ref(key) => current = &definitions.get(key)?.body,
            _ => return Some(atom),
        }
    }
    None
}

fn check_kind(
    requirement: &GlobalRequirement,
    definition: &PropertyDef,
    definitions: &DefinitionTable,
) -> Result<(), CatalogErrorReason> {
    let Some(atom) = resolved_atom(definition, definitions) else {
        let found = match definition {
            PropertyDef::Single(atom) if atom.is_array => "an array",
            PropertyDef::Single(PropertyAtom { ty: TypeExpr::Ref(key), .. })
                if definitions.outer(key) == Some(DefinitionOuter::Array) =>
            {
                "an array"
            }
            _ => "a union",
        };
        return Err(CatalogErrorReason::WrongType { expected: "an object", found });
    };
    match (&atom.ty, requirement.kind) {
        (TypeExpr::InlineObject(shape), _) if shape.properties.is_empty() && shape.pattern_keys.is_empty() => {
            Err(CatalogErrorReason::Empty)
        }
        (TypeExpr::InlineObject(_), _) => Ok(()),
        (TypeExpr::Primitive(SimplifiedType::Object), GlobalKind::OpenObject) => Ok(()),
        (TypeExpr::Primitive(other), _) => Err(CatalogErrorReason::WrongType {
            expected: match requirement.kind {
                GlobalKind::Shape => "an object shape",
                GlobalKind::OpenObject => "an object",
            },
            found: other.as_keyword(),
        }),
        _ => Err(CatalogErrorReason::WrongType { expected: "an object shape", found: "a union" }),
    }
}

/// Equal resolved members, ignoring the declarations' own descriptions.
fn same_shape(left: &PropertyDef, right: &PropertyDef) -> bool {
    match (left, right) {
        (PropertyDef::Single(left), PropertyDef::Single(right)) => {
            let mut left = left.clone();
            let mut right = right.clone();
            left.description = None;
            right.description = None;
            left == right
        }
        _ => left == right,
    }
}

#[cfg(test)]
mod discovery_tests;

#[cfg(test)]
mod host_function_cache_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_catalog_loads_every_darkmatter_global_in_order() {
        let names: Vec<_> = GlobalCatalog::darkmatter().globals().map(GlobalDeclaration::name).collect();
        assert_eq!(names, ["fm", "ctx", "current", "env", "err", "timing"]);
    }

    #[test]
    fn the_catalog_envelope_is_detected_by_content() {
        assert!(is_global_catalog(include_str!("../../../../schemas/darkmatter.yaml")));
        assert!(!is_global_catalog(include_str!("../../../../schemas/partials/fm.yaml")));
        assert!(!is_global_catalog("$schema:\n  title: string\n"));
    }

    /// A schema-root file is classified by its current bytes: a file that
    /// becomes a host catalog is discovered, one that stops being a catalog
    /// is dropped, and an unchanged catalog is served from the cache (the
    /// same profile) rather than parsed again.
    #[test]
    fn lifecycle_profiles_follow_each_root_file_s_current_content() {
        let catalog = include_str!("../../../benches/fixtures/claudine-schemas/globals.yaml");
        let artifact = include_str!("../../../benches/fixtures/pipeline-work/globals.lifecycle.json");
        let dir = tempfile::TempDir::new().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(root.join(".git")).expect("repository marker");
        std::fs::create_dir_all(root.join("schemas")).expect("schema root");
        let globals = root.join("schemas/globals.yaml");
        std::fs::write(root.join("schemas/globals.lifecycle.json"), artifact).expect("artifact");
        let context =
            biscuit_file::FileResolutionContext::from_snapshot(&root, None, Default::default()).with_repository_root(&root);
        let discover = || host_lifecycle_profiles(&context).collect::<Vec<_>>();
        let profile = |found: &[HostLifecycleDiscovery]| match found {
            [only] => only.profile.clone().expect("the frozen artifact is fresh for the frozen catalog"),
            other => panic!("one host catalog: {other:?}"),
        };

        std::fs::write(&globals, "title: not a catalog\n").expect("plain YAML");
        assert!(discover().is_empty(), "plain YAML is not a host catalog");
        assert!(discover().is_empty(), "nor is it on a cached call");

        std::fs::write(&globals, catalog).expect("catalog");
        let first = profile(&discover());
        let again = profile(&discover());
        assert!(std::sync::Arc::ptr_eq(&first, &again), "an unchanged catalog is served from the cache");

        std::fs::write(&globals, "title: no longer a catalog\n").expect("plain YAML again");
        assert!(discover().is_empty(), "a file that stops being a catalog is dropped");
    }
}
