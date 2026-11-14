//! The shipped configuration schema: the one authority for the shape and
//! defaults of `~/.darkmatter.yaml` and `{repo-root}/.darkmatter.yaml`.
//!
//! The schema text is `darkmatter/schemas/darkmatter-config.yaml`, compiled
//! in among Darkmatter's shipped documents. Other schemas import its types
//! through the reserved namespace alias `config`
//! ([`PERMISSIONS_TYPE_REFERENCE`]); [`super::load`] validates a
//! configuration file against [`ConfigScope::config_type`] and nothing else.
//!
//! User-facing guide: `darkmatter/docs/topics/configuration.md`.

use std::sync::OnceLock;

use simplified_schema::SimplifiedSchema;

/// The virtual path of the shipped configuration schema.
pub const CONFIG_SCHEMA_PATH: &str = crate::markdown::schemas::embedded::CONFIG_SCHEMA_PATH;

/// The configuration schema's text, as shipped.
pub const CONFIG_SCHEMA_SOURCE: &str = include_str!("../../../schemas/darkmatter-config.yaml");

/// How another schema imports the shared permissions block. The scope
/// projections are `user-permissions@darkmatter:config` and
/// `repository-permissions@darkmatter:config`.
pub const PERMISSIONS_TYPE_REFERENCE: &str = "permissions@darkmatter:config";

/// The file name of both configuration files.
pub const CONFIG_FILE_NAME: &str = ".darkmatter.yaml";

/// The two configuration scopes, each with its own projection of the shared
/// shape (the `file-read` default differs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigScope {
    /// `~/.darkmatter.yaml`.
    User,
    /// `{repo-root}/.darkmatter.yaml`.
    Repository,
}

impl ConfigScope {
    /// The named type a whole configuration file of this scope satisfies.
    #[must_use]
    pub const fn config_type(self) -> &'static str {
        match self {
            Self::User => "user-config",
            Self::Repository => "repository-config",
        }
    }

    /// The named type of this scope's inline permissions block.
    #[must_use]
    pub const fn permissions_type(self) -> &'static str {
        match self {
            Self::User => "user-permissions",
            Self::Repository => "repository-permissions",
        }
    }
}

/// The prepared contract of a whole configuration file of `scope`.
///
/// ## Panics
///
/// When the shipped schema does not prepare, which
/// `config::schema::tests` and the shipped artifact rule out.
pub(crate) fn contract(scope: ConfigScope) -> &'static SimplifiedSchema {
    static USER: OnceLock<SimplifiedSchema> = OnceLock::new();
    static REPOSITORY: OnceLock<SimplifiedSchema> = OnceLock::new();
    selected(scope.config_type(), if scope == ConfigScope::User { &USER } else { &REPOSITORY })
}

/// The prepared contract of `scope`'s inline permissions block alone, which
/// words a problem inside a written block more precisely than the
/// mapping-or-reference union around it.
///
/// ## Panics
///
/// As [`contract`].
pub(crate) fn permissions_contract(scope: ConfigScope) -> &'static SimplifiedSchema {
    static USER: OnceLock<SimplifiedSchema> = OnceLock::new();
    static REPOSITORY: OnceLock<SimplifiedSchema> = OnceLock::new();
    selected(scope.permissions_type(), if scope == ConfigScope::User { &USER } else { &REPOSITORY })
}

/// Prepares the object type `name` of a host program's compiled-in
/// configuration schema (`source`, cited as `virtual_path`) as a closed root
/// contract. Imports written as `Name@darkmatter:<stem>`, such as
/// [`PERMISSIONS_TYPE_REFERENCE`], resolve among Darkmatter's shipped
/// documents; nothing is read from disk, so the empty context it resolves in
/// names no file.
///
/// ## Errors
///
/// A [`SchemaError::SchemaDocument`] when `source` is not a schema document
/// with a named-type table holding an object type `name`, or the error that
/// expanding or lowering it raised.
pub fn prepare_host_config_type(
    source: &str,
    virtual_path: &str,
    name: &str,
) -> Result<simplified_schema::Prepared, simplified_schema::SchemaError> {
    use simplified_schema::{PropertyAtom, PropertyDef, SchemaDeclaration, SchemaError, SchemaRoot, TypeExpr};
    use std::path::{Path, PathBuf};

    let unusable = |message: String| SchemaError::SchemaDocument {
        path: PathBuf::from(virtual_path),
        message,
        source: None,
        span: None,
    };
    let path = Path::new(virtual_path);
    let document = simplified_schema::parse_standalone_schema_document(source, path)?
        .ok_or_else(|| unusable(format!("{virtual_path} is not a schema document")))?;
    let table = match document.declaration {
        SchemaDeclaration::Schema(SchemaRoot::Single(table)) => table,
        _ => return Err(unusable(format!("{virtual_path} has no named-type table"))),
    };
    let shape = match table.properties.get(name) {
        Some(PropertyDef::Single(PropertyAtom { ty: TypeExpr::InlineObject(shape), is_array: false, .. })) => {
            shape.clone()
        }
        _ => return Err(unusable(format!("{virtual_path} has no object type `{name}`"))),
    };
    let context = biscuit_file::FileResolutionContext::from_snapshot(PathBuf::new(), None, Default::default());
    let expansion = crate::markdown::schemas::resolve::expand_named_types_with_locals(
        &SchemaRoot::Single(shape),
        Some(&table),
        Path::new(""),
        Some(path),
        &[],
        &context,
    )?;
    let lowered = simplified_schema::to_json_schema_with_definitions(&expansion.schema, &expansion.definitions)?;
    simplified_schema::Prepared::new(std::sync::Arc::new(lowered), None, context)
}

fn selected(name: &str, cell: &'static OnceLock<SimplifiedSchema>) -> &'static SimplifiedSchema {
    cell.get_or_init(|| {
        catalog().select(name).unwrap_or_else(|error| panic!("the shipped `{name}` type selects: {error}"))
    })
}

fn catalog() -> &'static SimplifiedSchema {
    static CATALOG: OnceLock<SimplifiedSchema> = OnceLock::new();
    CATALOG.get_or_init(|| {
        SimplifiedSchema::try_from(CONFIG_SCHEMA_SOURCE)
            .unwrap_or_else(|error| panic!("the shipped configuration schema prepares: {error}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_shipped_document_is_the_text_the_namespace_serves() {
        let shipped = crate::markdown::schemas::embedded::SHIPPED_SOURCES
            .partial("config")
            .expect("`darkmatter:config` names a shipped document");
        assert_eq!((shipped.path, shipped.text), (CONFIG_SCHEMA_PATH, CONFIG_SCHEMA_SOURCE));
    }

    #[test]
    fn another_schema_imports_the_permissions_types_through_the_alias() {
        let dir = tempfile::tempdir().unwrap();
        let context = biscuit_file::FileResolutionContext::from_snapshot(dir.path(), None, Default::default());
        let schema: serde_yaml_ng::Value = serde_yaml_ng::from_str(&format!(
            "shared: {PERMISSIONS_TYPE_REFERENCE}\nuser: user-permissions@darkmatter:config\ntalk: communicate@darkmatter:lifecycle\n"
        ))
        .unwrap();
        let resolved = crate::markdown::schemas::resolve::resolve_yaml_schema(&schema, dir.path(), &context)
            .expect("the alias and the partial stems resolve");
        let properties = &resolved.json_schema["properties"];
        let shared = &properties["shared"]["anyOf"][1]["properties"];
        assert!(shared["file-read"].get("default").is_none(), "the shared shape has no scope default");
        assert_eq!(shared["network"]["default"], json!("deny"));
        let user = &properties["user"]["anyOf"][1]["properties"];
        assert_eq!(user["file-read"]["default"][0], json!("./*.txt"));
        assert!(resolved.referenced_files.is_empty(), "a shipped document is no file dependency");

        let missing: serde_yaml_ng::Value = serde_yaml_ng::from_str("x: nope@darkmatter:config\n").unwrap();
        assert!(crate::markdown::schemas::resolve::resolve_yaml_schema(&missing, dir.path(), &context).is_err());
    }

    /// A parent default is judged whole, so each scope's `permissions`
    /// default restates its children's defaults; they must agree.
    #[test]
    fn each_permissions_default_equals_its_children_defaults() {
        for scope in [ConfigScope::User, ConfigScope::Repository] {
            let defaulted = |document: serde_json::Value| {
                let success = contract(scope).validate_strict(&document).expect("valid");
                let mut value = document.clone();
                for default in &success.defaults {
                    let mut target = &mut value;
                    let (last, parents) = default.path.segments().split_last().expect("a property");
                    for segment in parents {
                        target = target.get_mut(segment).expect("parent before child");
                    }
                    target[last] = default.value.clone();
                }
                value["permissions"].clone()
            };
            assert_eq!(defaulted(json!({})), defaulted(json!({ "permissions": {} })), "{scope:?}");
        }
    }
}
