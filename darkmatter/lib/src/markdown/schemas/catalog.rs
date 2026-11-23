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
//! with its partials, prepared ahead of time into the shipped artifact
//! ([`super::shipped`]), and read through [`GlobalCatalog::darkmatter`]. The
//! runtime baseline is its resolved `fm` definition, the `ctx.*` descriptor
//! catalog projects its `ctx` declaration, and `current` declares the same
//! shape. A host declares its own globals in a catalog of the same format and
//! loads it with [`GlobalCatalog::from_documents`] and its own
//! [`GlobalRequirement`]s.
//!
//! A catalog becomes the baseline or a host's globals only by explicit
//! registration through [`GlobalCatalog`], never by discovery: trigger
//! discovery never claims it. To the standalone schema loader it is an
//! ordinary tagged schema document, so a document `$schema` that names a
//! catalog is accepted and gets the catalog's entries as document properties,
//! optional unless an entry says `required`. Shape is separate from
//! availability: which events may read a global is the host's
//! [`BindingView`](crate::markdown::compose::expression::BindingView).
//!
//! The loader is strict. Every required global must be declared exactly once
//! with a non-null, non-empty, resolvable definition of the required kind; an
//! undeclared name, a duplicate key, or trailing YAML is a [`CatalogError`]
//! naming the field. A declaration resolves like any document schema: a
//! recursive named type, and a structured acyclic type the declaration uses
//! at two or more sites, is stored once in [`GlobalDeclaration::definitions`]
//! and named by reference. A global typed by such a type is of the object
//! kinds when the type's body is an object.

use std::borrow::Cow;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_yaml_ng::{Mapping, Value as YamlValue};
use simplified_schema::shipped::{PreparationMode, RecordError};
use thiserror::Error;

use super::embedded::{self, EmbeddedDocument};
use simplified_schema::errors::SchemaError;
use super::resolve;
use simplified_schema::resolve::describe_yaml;
use super::{
    DefinitionOuter, DefinitionTable, PropertyAtom, PropertyDef, SchemaShape, SchemaRoot, SimplifiedType, TypeExpr,
};

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
    /// The definition holds `{{ … }}` schema slots (directly or through an
    /// import). A catalog declares globals, which apply before any slot
    /// could be resolved, so it never holds one.
    #[error("{}", super::slots::codes::slots_not_allowed_message("a global catalog", .0))]
    SlotsNotAllowed(Box<simplified_schema::UnresolvedSlots>),
}

impl CatalogError {
    /// The slot code of a catalog holding slots,
    /// [`SLOT_NOT_ALLOWED_IN_BASELINE_CODE`](super::slots::codes::SLOT_NOT_ALLOWED_IN_BASELINE_CODE);
    /// `None` for every other failure.
    #[must_use]
    pub fn slot_code(&self) -> Option<&'static str> {
        matches!(self.reason, CatalogErrorReason::SlotsNotAllowed(_))
            .then_some(super::slots::codes::SLOT_NOT_ALLOWED_IN_BASELINE_CODE)
    }
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
/// Every stored named type the declaration reaches, recursive or shared, stays
/// a named reference ([`TypeExpr::Ref`]) into [`Self::definitions`]; a
/// whole-type description names the type instead of expanding it.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalDeclaration {
    name: String,
    definition: PropertyDef,
    definitions: Arc<DefinitionTable>,
}

#[allow(missing_docs)]
impl GlobalDeclaration {
    /// A declaration prepared outside a catalog document: a host's
    /// `kind: namespace-schema` file
    /// ([`crate::config::registration::namespaces`]).
    pub(crate) fn new(name: String, definition: PropertyDef, definitions: Arc<DefinitionTable>) -> Self {
        Self { name, definition, definitions }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The resolved definition, with every import expanded; a use of a
    /// stored type stays a reference into [`Self::definitions`].
    pub fn definition(&self) -> &PropertyDef {
        &self.definition
    }

    /// The stored definitions, recursive or shared, that
    /// [`Self::definition`]'s references point into; empty when it reaches
    /// none.
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
    /// typed by a stored object type yields that type's body; members typed
    /// by stored types still reference [`Self::definitions`].
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
    Embedded(&'a [EmbeddedDocument<'a>], PreparationMode<'a>),
    Disk {
        dir: &'a Path,
        context: &'a biscuit_file::FileResolutionContext,
    },
}

impl GlobalCatalog {
    /// Darkmatter's compiled-in catalog: the accepted shipped artifact's
    /// ([`super::shipped`]), or, when that artifact or its section was refused, resolved once
    /// per process from the embedded documents. It reads no disk or network
    /// and does not depend on the launch directory.
    ///
    /// ## Panics
    ///
    /// When the artifact was refused and the compiled-in catalog does not
    /// load. This is a library bug, not an author error; a test loads it.
    pub fn darkmatter() -> &'static GlobalCatalog {
        super::shipped::registry().global_catalog()
    }

    /// [`Self::darkmatter`] prepared from the embedded documents, once per
    /// process: the fallback when the shipped artifact or its section was
    /// refused.
    pub(crate) fn darkmatter_from_sources() -> &'static GlobalCatalog {
        static CATALOG: OnceLock<GlobalCatalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            crate::note_work!(ShippedRuntimePreparation);
            Self::load_embedded(DARKMATTER_CATALOG_PATH, embedded::SHIPPED, DARKMATTER_GLOBALS, PreparationMode::Runtime)
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
        Self::load_embedded(path, &table, requirements, PreparationMode::Runtime)
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
        Self::from_text(path, &text, requirements, context)
    }

    /// [`Self::from_file`] over `text`, the bytes the caller already read
    /// from `path`; `path` names the catalog and locates its imports, and is
    /// not read again.
    pub(crate) fn from_text(
        path: &Path,
        text: &str,
        requirements: &[GlobalRequirement],
        context: &biscuit_file::FileResolutionContext,
    ) -> Result<Self, CatalogError> {
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        Self::load_disk(&path.display().to_string(), text, dir, context, requirements)
    }

    /// [`Self::load`] for a catalog on the disk. Each global resolves on its
    /// own, so a failure names its field; the retaining scope makes a file
    /// that several globals import (`ctx` and `current` typed alike) one read.
    fn load_disk(
        catalog: &str,
        text: &str,
        dir: &Path,
        context: &biscuit_file::FileResolutionContext,
        requirements: &[GlobalRequirement],
    ) -> Result<Self, CatalogError> {
        simplified_schema::resolve::buffers::with_reads_retained(|| {
            Self::load(catalog, text, &Source::Disk { dir, context }, requirements)
        })
    }

    fn load_embedded(
        path: &str,
        documents: &[EmbeddedDocument<'_>],
        requirements: &[GlobalRequirement],
        mode: PreparationMode<'_>,
    ) -> Result<Self, CatalogError> {
        let Some(document) = embedded::find(documents, path) else {
            return Err(CatalogError {
                catalog: path.to_string(),
                field: String::new(),
                reason: CatalogErrorReason::Missing,
            });
        };
        Self::load(path, document.text, &Source::Embedded(documents, mode), requirements)
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

    /// A catalog of `declarations` prepared one by one (a registered host's
    /// namespace files), in the given order. The caller has checked their
    /// names are distinct.
    pub(crate) fn from_declarations(declarations: impl IntoIterator<Item = GlobalDeclaration>) -> Self {
        let globals = declarations.into_iter().map(|declaration| (declaration.name.clone(), declaration)).collect();
        Self { description: None, globals }
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

    /// The resolved `fm` definition as a document schema, with the
    /// definitions its references point into: the built-in baseline when this
    /// is Darkmatter's catalog. Lower the pair with
    /// [`to_json_schema_with_definitions`](super::to_json_schema_with_definitions).
    /// `None` for a catalog that declares no `fm`.
    pub fn baseline(&self) -> Option<(SchemaRoot, Arc<DefinitionTable>)> {
        let fm = self.global("fm")?;
        let shape = fm.shape()?.clone();
        Some((SchemaRoot::Single(shape), Arc::clone(&fm.definitions)))
    }

    /// The catalog that `record` describes, checked against `requirements`:
    /// the recorded globals are exactly the required ones, in requirement
    /// order, each of its required kind and `same_as` shape. Shipped-artifact
    /// acceptance builds a catalog this way instead of resolving its text.
    ///
    /// ## Errors
    ///
    /// A [`RecordError`] whose pointer is relative to the record
    /// (`/globals/2`) when a global is missing, extra, out of order, or not
    /// of its required kind.
    pub fn from_record(record: GlobalCatalogRecord, requirements: &[GlobalRequirement]) -> Result<Self, RecordError> {
        if record.globals.len() != requirements.len() {
            return Err(RecordError::new(
                "/globals",
                format!("{} globals are recorded, {} are required", record.globals.len(), requirements.len()),
            ));
        }
        let mut globals = IndexMap::with_capacity(requirements.len());
        for (index, (global, requirement)) in record.globals.into_iter().zip(requirements).enumerate() {
            let at = format!("/globals/{index}");
            if global.name != requirement.name {
                return Err(RecordError::new(
                    format!("{at}/name"),
                    format!("`{}` is recorded where `{}` is required", global.name, requirement.name),
                ));
            }
            check_kind(requirement, &global.definition, &global.definitions)
                .map_err(|reason| RecordError::new(at.clone(), format!("`{}` {reason}", global.name)))?;
            if let Some(other) = &requirement.same_as {
                let same = globals
                    .get(other.as_ref())
                    .is_some_and(|earlier: &GlobalDeclaration| same_shape(&earlier.definition, &global.definition));
                if !same {
                    return Err(RecordError::new(at, format!("`{}` must declare the same shape as `{other}`", global.name)));
                }
            }
            globals.insert(
                global.name.clone(),
                GlobalDeclaration { name: global.name, definition: global.definition, definitions: Arc::new(global.definitions) },
            );
        }
        Ok(Self { description: record.description, globals })
    }
}

/// A loaded [`GlobalCatalog`] as shipped-artifact data: the catalog's
/// virtual path, its description, and each global's resolved definition and
/// stored-definition table, in requirement order.
///
/// A host that ships a catalog prepares its record at generation
/// ([`Self::prepare`]) and rebuilds the catalog from an accepted artifact
/// with [`GlobalCatalog::from_record`] and the same requirements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlobalCatalogRecord {
    /// The catalog's virtual path.
    pub identity: String,
    #[serde(deserialize_with = "simplified_schema::shipped::present")]
    pub description: Option<String>,
    pub globals: Vec<GlobalDeclarationRecord>,
}

/// One global of a [`GlobalCatalogRecord`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlobalDeclarationRecord {
    pub name: String,
    /// The resolved definition; a stored type stays a reference into
    /// `definitions`.
    pub definition: PropertyDef,
    pub definitions: DefinitionTable,
}

impl GlobalCatalogRecord {
    /// Loads the catalog at virtual path `path` from `documents`, as
    /// [`GlobalCatalog::from_documents`] does, in `mode`; generation passes
    /// [`PreparationMode::Uncached`].
    ///
    /// ## Errors
    ///
    /// The [`CatalogError`] of the first failing field.
    pub fn prepare(
        path: &str,
        documents: &[EmbeddedDocument<'_>],
        requirements: &[GlobalRequirement],
        mode: PreparationMode<'_>,
    ) -> Result<Self, CatalogError> {
        GlobalCatalog::load_embedded(path, documents, requirements, mode).map(|catalog| Self::of(path, &catalog))
    }

    /// The record of an already loaded `catalog` found at `path`.
    #[must_use]
    pub fn of(path: &str, catalog: &GlobalCatalog) -> Self {
        Self {
            identity: path.to_string(),
            description: catalog.description.clone(),
            globals: catalog
                .globals()
                .map(|global| GlobalDeclarationRecord {
                    name: global.name.clone(),
                    definition: global.definition.clone(),
                    definitions: global.definitions.as_ref().clone(),
                })
                .collect(),
        }
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

/// Why a host function catalog did not load.
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

/// The artifact's state, as part of a cache key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ArtifactKey {
    Absent,
    Unreadable(String),
    Text(String),
}

impl ArtifactKey {
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
    /// Where the facts artifact was read: beside the catalog, or where a
    /// host registration manifest named it.
    facts_path: std::path::PathBuf,
    artifact: ArtifactKey,
    result: HostFunctionCacheValue,
}

type HostFunctionCache = std::collections::HashMap<std::path::PathBuf, HostFunctionEntry>;

fn host_function_cache() -> std::sync::MutexGuard<'static, HostFunctionCache> {
    static CACHE: std::sync::LazyLock<std::sync::Mutex<HostFunctionCache>> = std::sync::LazyLock::new(Default::default);
    CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The analysis-only set of the host function catalog at `path` (bytes
/// `text`) with its facts read from `facts_path`, which a host registration
/// manifest names; `facts` is that read's result.
///
/// The process keeps one result per catalog path: the catalog's content
/// hash, the artifact as read, and the loaded set or its error. An unchanged
/// revision reuses that result, an error included; a changed catalog or
/// artifact replaces it, so a replaced catalog is freed once no caller holds
/// a set, clone, or scope guard from it. A set obtained earlier keeps
/// working after its path's entry is replaced.
///
/// ## Errors
///
/// A catalog that does not load or names a built-in function. A missing or
/// bad facts artifact makes the affected functions incomplete instead.
pub(crate) fn host_function_set_at(
    path: &Path,
    text: &str,
    facts_path: &Path,
    facts: std::io::Result<Vec<u8>>,
) -> HostFunctionCacheValue {
    host_function_set_from(path, text, facts_path, ArtifactKey::from_read(facts))
}

fn host_function_set_from(path: &Path, text: &str, facts_path: &Path, artifact: ArtifactKey) -> HostFunctionCacheValue {
    use crate::markdown::compose::expression::{HostFunctionCatalog, HostFunctions};

    let catalog_hash = biscuit_hash::blake3_hash(text);
    // Lookup, build, and replacement happen under one lock, so an unchanged
    // revision is never built twice and each path keeps one result.
    let mut cache = host_function_cache();
    if let Some(cached) = cache.get(path)
        && cached.catalog_hash == catalog_hash
        && cached.facts_path == facts_path
        && cached.artifact == artifact
    {
        return cached.result.clone();
    }
    discovery_probe::note_host_function_build();
    let fail = |message: String| HostFunctionCatalogError { path: path.to_path_buf(), message };
    let result = HostFunctionCatalog::load(text, path).map_err(|error| fail(error.to_string())).and_then(|catalog| {
        HostFunctions::analysis_only_at(catalog, facts_path, artifact.artifact()).map_err(|error| fail(error.to_string()))
    });
    cache.insert(
        path.to_path_buf(),
        HostFunctionEntry { catalog_hash, facts_path: facts_path.to_path_buf(), artifact, result: result.clone() },
    );
    result
}

/// Test-only observation of the host function cache on the current thread:
/// how many sets it builds and what it holds for a path. Production builds
/// count nothing.
#[cfg(test)]
mod discovery_probe {
    use std::cell::RefCell;
    use std::path::Path;

    thread_local! {
        static BUILDS: RefCell<Option<usize>> = const { RefCell::new(None) };
    }

    /// Observes host function builds on this thread until dropped.
    pub(super) struct DiscoveryProbe {
        // Thread-local state: the probe must stay on its thread.
        _not_send: std::marker::PhantomData<*const ()>,
    }

    impl DiscoveryProbe {
        /// Starts observing this thread's builds.
        ///
        /// ## Panics
        ///
        /// When this thread already has a probe.
        #[must_use]
        pub(super) fn install() -> Self {
            BUILDS.with_borrow_mut(|builds| {
                assert!(builds.is_none(), "one discovery probe per thread");
                *builds = Some(0);
            });
            Self { _not_send: std::marker::PhantomData }
        }

        /// The host function sets the cache loaded (successfully or not) so
        /// far; a cached result is not counted.
        pub(super) fn host_function_builds(&self) -> usize {
            BUILDS.with_borrow(|builds| builds.expect("the probe is installed"))
        }
    }

    impl Drop for DiscoveryProbe {
        fn drop(&mut self) {
            BUILDS.with_borrow_mut(|builds| *builds = None);
        }
    }

    /// The facts artifact as the host function cache recorded it.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) enum CachedArtifact {
        Absent,
        Unreadable(String),
        Text(String),
    }

    /// The host function cache's entry for one catalog path.
    #[derive(Debug, Clone)]
    pub(super) struct CachedHostFunctions {
        pub(super) catalog_hash: String,
        pub(super) artifact: CachedArtifact,
        /// The cached set's function names, or the cached load error.
        pub(super) result: Result<Vec<String>, super::HostFunctionCatalogError>,
    }

    /// The host function cache's entry for `path`, if it holds one. Holds
    /// nothing of the cached set alive.
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
    pub(super) fn host_function_entries_under(dir: &Path) -> usize {
        super::host_function_cache().keys().filter(|path| path.starts_with(dir)).count()
    }

    pub(super) fn note_host_function_build() {
        BUILDS.with_borrow_mut(|builds| {
            if let Some(builds) = builds {
                *builds += 1;
            }
        });
    }
}

#[cfg(not(test))]
mod discovery_probe {
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
/// global. The definition's references point into the returned table.
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
        Source::Embedded(documents, mode) => resolve::prepare_embedded_catalog_declaration_with_mode(
            &single,
            catalog,
            documents,
            simplified_schema::resolve::Sharing::Automatic,
            *mode,
        )
        .map(|(resolved, _)| resolved),
        Source::Disk { dir, context } => resolve::resolve_catalog_declaration(&single, dir, context),
    }
    .map_err(|error| match error {
        SchemaError::UnresolvedSlots(unresolved) => CatalogErrorReason::SlotsNotAllowed(unresolved),
        error => CatalogErrorReason::Resolve(Box::new(error)),
    })?;
    match resolved.simplified {
        Some(SchemaRoot::Single(mut shape)) => shape
            .properties
            .shift_remove(name)
            .map(|definition| (definition, resolved.definitions))
            .ok_or(CatalogErrorReason::Missing),
        _ => Err(CatalogErrorReason::WrongType { expected: "a single definition", found: "a union" }),
    }
}

/// Darkmatter's built-in baseline as a definition-free schema: the shipped
/// catalog's `fm` declaration resolved with every acyclic use inlined, the
/// form [`super::darkmatter_base_schema`] publishes. The catalog itself
/// ([`GlobalCatalog::darkmatter`]) stores the shared types once.
///
/// ## Panics
///
/// When the compiled-in declaration does not resolve: a library bug, which
/// a test catches.
pub(crate) fn darkmatter_inline_baseline() -> SchemaRoot {
    darkmatter_inline_baseline_with_mode(PreparationMode::Runtime)
        .unwrap_or_else(|reason| panic!("the embedded baseline must resolve inline: {reason}"))
}

/// [`darkmatter_inline_baseline`] in `mode`, failing with a message instead
/// of panicking; shipped-artifact generation passes
/// [`PreparationMode::Uncached`].
pub(crate) fn darkmatter_inline_baseline_with_mode(mode: PreparationMode<'_>) -> Result<SchemaRoot, String> {
    let document = embedded::find(embedded::SHIPPED, DARKMATTER_CATALOG_PATH).ok_or("the catalog is missing")?;
    let root: YamlValue = serde_yaml_ng::from_str(document.text).map_err(|error| error.to_string())?;
    let value = root
        .get("$schema")
        .and_then(|declarations| declarations.get("fm"))
        .ok_or("the catalog declares no `fm`")?;
    let mut single = Mapping::new();
    single.insert(YamlValue::String("fm".into()), value.clone());
    let (resolved, _) = resolve::prepare_embedded_catalog_declaration_with_mode(
        &YamlValue::Mapping(single),
        DARKMATTER_CATALOG_PATH,
        embedded::SHIPPED,
        simplified_schema::resolve::Sharing::Off,
        mode,
    )
    .map_err(|error| error.to_string())?;
    let Some(SchemaRoot::Single(mut shape)) = resolved.simplified else {
        return Err("`fm` is not a single definition".into());
    };
    let definition = shape.properties.shift_remove("fm").ok_or("`fm` is missing")?;
    match resolved_atom(&definition, &resolved.definitions).map(|atom| &atom.ty) {
        Some(TypeExpr::InlineObject(shape)) => Ok(SchemaRoot::Single(shape.clone())),
        _ => Err("`fm` is not an object shape".into()),
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
}
