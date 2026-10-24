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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalRequirement {
    pub name: &'static str,
    pub kind: GlobalKind,
    /// A global whose resolved shape this one must equal.
    pub same_as: Option<&'static str>,
}

impl GlobalRequirement {
    /// A global whose definition is a non-empty object shape.
    #[must_use]
    pub const fn shape(name: &'static str) -> Self {
        Self { name, kind: GlobalKind::Shape, same_as: None }
    }
}

/// Darkmatter's globals, in catalog order.
pub const DARKMATTER_GLOBALS: &[GlobalRequirement] = &[
    GlobalRequirement::shape("fm"),
    GlobalRequirement::shape("ctx"),
    GlobalRequirement { name: "current", kind: GlobalKind::Shape, same_as: Some("ctx") },
    GlobalRequirement { name: "env", kind: GlobalKind::OpenObject, same_as: None },
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
    Mismatch(&'static str),
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
        let catalog = path.display().to_string();
        let text = std::fs::read_to_string(path).map_err(|source| CatalogError {
            catalog: catalog.clone(),
            field: String::new(),
            reason: CatalogErrorReason::Resolve(Box::new(SchemaError::Io { path: path.to_path_buf(), source })),
        })?;
        let declared: Vec<String> = serde_yaml_ng::from_str::<YamlValue>(&text)
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
        // Requirement names are `'static`; a discovered catalog is loaded
        // once per editor preparation, so its few names are leaked.
        let requirements: Vec<GlobalRequirement> = declared
            .into_iter()
            .map(|name| GlobalRequirement::shape(Box::leak(name.into_boxed_str())))
            .collect();
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        Self::load(&catalog, &text, &Source::Disk { dir, context }, &requirements)
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
            let value = required(declarations, requirement.name).map_err(|reason| fail(&field, reason))?;
            let (definition, definitions) =
                resolve_declaration(catalog, requirement.name, value, source).map_err(|reason| fail(&field, reason))?;
            check_kind(requirement, &definition, &definitions).map_err(|reason| fail(&field, reason))?;
            if let Some(other) = requirement.same_as {
                let same = globals
                    .get(other)
                    .is_some_and(|earlier: &GlobalDeclaration| same_shape(&earlier.definition, &definition));
                if !same {
                    return Err(fail(&field, CatalogErrorReason::Mismatch(other)));
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
        Source::Embedded(documents) => resolve::prepare_embedded_inline_schema(&single, catalog, documents, "").map(|(resolved, _)| resolved),
        Source::Disk { dir, context } => resolve::resolve_yaml_schema(&single, dir, context),
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
