//! `$schema` resolution with Darkmatter's shipped documents supplied.
//!
//! The resolver is the library's ([`simplified_schema::resolve`]); these
//! wrappers keep Darkmatter's signatures and hand it Darkmatter's reserved
//! namespace ([`embedded::SHIPPED_SOURCES`], `Name@darkmatter:<stem>`), so a
//! Darkmatter caller resolves exactly as before.

use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use serde_json::Value;
use serde_yaml_ng::Value as YamlValue;
use simplified_schema::resolve::*;
use simplified_schema::shipped::PreparationMode;
use simplified_schema::{SchemaError, SchemaRoot, SchemaShape, StandaloneSchemaDocument, embedded::EmbeddedDocument};

use super::embedded::SHIPPED_SOURCES;

/// [`simplified_schema::resolve::resolve_schema`] with Darkmatter's shipped
/// documents.
pub fn resolve_schema(value: &Value, base_dir: &Path, context: &FileResolutionContext) -> Result<ResolvedSchema, SchemaError> {
    simplified_schema::resolve::resolve_schema(value, base_dir, context, SHIPPED_SOURCES)
}

/// [`simplified_schema::resolve::resolve_schema_with_roots`] with
/// Darkmatter's shipped documents.
pub fn resolve_schema_with_roots(
    value: &Value,
    base_dir: &Path,
    schema_roots: &[PathBuf],
    request_context: &FileResolutionContext,
) -> Result<ResolvedSchema, SchemaError> {
    simplified_schema::resolve::resolve_schema_with_roots(value, base_dir, schema_roots, request_context, SHIPPED_SOURCES)
}

/// [`simplified_schema::resolve::resolve_authored_schema_with_roots`] with
/// Darkmatter's shipped documents.
pub fn resolve_authored_schema_with_roots(
    value: &Value,
    authored: Option<&AuthoredSchemaValue>,
    base_dir: &Path,
    schema_roots: &[PathBuf],
    request_context: &FileResolutionContext,
) -> Result<ResolvedSchema, SchemaError> {
    simplified_schema::resolve::resolve_authored_schema_with_roots(
        value,
        authored,
        base_dir,
        schema_roots,
        request_context,
        SHIPPED_SOURCES,
    )
}

/// [`simplified_schema::resolve::resolve_authored_yaml_schema`] with
/// Darkmatter's shipped documents.
pub fn resolve_authored_yaml_schema(
    value: &YamlValue,
    owner: InlineOwner<'_>,
    authored: Option<&AuthoredSchemaValue>,
    base_dir: &Path,
    schema_roots: &[PathBuf],
    request_context: &FileResolutionContext,
) -> Result<ResolvedSchema, SchemaError> {
    simplified_schema::resolve::resolve_authored_yaml_schema(
        value,
        owner,
        authored,
        base_dir,
        schema_roots,
        request_context,
        SHIPPED_SOURCES,
    )
}

/// [`simplified_schema::resolve::resolve_yaml_schema`] with Darkmatter's
/// shipped documents.
pub fn resolve_yaml_schema(
    value: &YamlValue,
    base_dir: &Path,
    context: &FileResolutionContext,
) -> Result<ResolvedSchema, SchemaError> {
    simplified_schema::resolve::resolve_yaml_schema(value, base_dir, context, SHIPPED_SOURCES)
}

/// [`simplified_schema::resolve::resolve_yaml_schema_with_roots`] with
/// Darkmatter's shipped documents.
pub fn resolve_yaml_schema_with_roots(
    value: &YamlValue,
    base_dir: &Path,
    schema_roots: &[PathBuf],
    request_context: &FileResolutionContext,
) -> Result<ResolvedSchema, SchemaError> {
    simplified_schema::resolve::resolve_yaml_schema_with_roots(value, base_dir, schema_roots, request_context, SHIPPED_SOURCES)
}

/// [`simplified_schema::resolve::resolve_yaml_schema_owned_by`] with
/// Darkmatter's shipped documents.
pub fn resolve_yaml_schema_owned_by(
    value: &YamlValue,
    owner: InlineOwner<'_>,
    base_dir: &Path,
    schema_roots: &[PathBuf],
    request_context: &FileResolutionContext,
) -> Result<ResolvedSchema, SchemaError> {
    simplified_schema::resolve::resolve_yaml_schema_owned_by(value, owner, base_dir, schema_roots, request_context, SHIPPED_SOURCES)
}

/// [`simplified_schema::resolve::resolve_catalog_declaration`] with
/// Darkmatter's shipped documents.
pub fn resolve_catalog_declaration(
    value: &YamlValue,
    base_dir: &Path,
    request_context: &FileResolutionContext,
) -> Result<ResolvedSchema, SchemaError> {
    simplified_schema::resolve::resolve_catalog_declaration(value, base_dir, request_context, SHIPPED_SOURCES)
}

/// [`simplified_schema::resolve::named_type_reference_path`] with
/// Darkmatter's reserved namespace: `Name@darkmatter:<stem>` names no file.
pub fn named_type_reference_path(
    reference: &str,
    base_dir: &Path,
    schema_roots: &[PathBuf],
    request_context: &FileResolutionContext,
) -> Result<Option<PathBuf>, SchemaError> {
    simplified_schema::resolve::named_type_reference_path(reference, base_dir, schema_roots, request_context, SHIPPED_SOURCES)
}

/// [`simplified_schema::resolve::expand_named_types`] with Darkmatter's
/// shipped documents.
pub fn expand_named_types(
    schema: &SchemaRoot,
    base_dir: &Path,
    defining_file: Option<&Path>,
    schema_roots: &[PathBuf],
    request_context: &FileResolutionContext,
) -> Result<NamedTypeExpansion, SchemaError> {
    simplified_schema::resolve::expand_named_types(schema, base_dir, defining_file, schema_roots, request_context, SHIPPED_SOURCES)
}

/// [`simplified_schema::resolve::expand_named_types_with_locals`] with
/// Darkmatter's shipped documents.
pub fn expand_named_types_with_locals(
    schema: &SchemaRoot,
    local_types: Option<&SchemaShape>,
    base_dir: &Path,
    defining_file: Option<&Path>,
    schema_roots: &[PathBuf],
    request_context: &FileResolutionContext,
) -> Result<NamedTypeExpansion, SchemaError> {
    simplified_schema::resolve::expand_named_types_with_locals(
        schema,
        local_types,
        base_dir,
        defining_file,
        schema_roots,
        request_context,
        SHIPPED_SOURCES,
    )
}

/// [`simplified_schema::resolve::check_standalone_named_types`] with
/// Darkmatter's shipped documents.
pub fn check_standalone_named_types(
    document: &StandaloneSchemaDocument,
    schema_roots: &[PathBuf],
    request_context: &FileResolutionContext,
) -> Result<Vec<PathBuf>, SchemaError> {
    crate::note_work!(StandaloneImportCheck);
    simplified_schema::resolve::check_standalone_named_types(document, schema_roots, request_context, SHIPPED_SOURCES)
}

/// [`simplified_schema::resolve::prepare_embedded_schema_in`] with
/// Darkmatter's shipped documents.
pub fn prepare_embedded_schema_in(
    path: &str,
    documents: &[EmbeddedDocument<'_>],
    bundle: &str,
) -> Result<(ResolvedSchema, Vec<PathBuf>), SchemaError> {
    simplified_schema::resolve::prepare_embedded_schema_in(path, documents, bundle, SHIPPED_SOURCES)
}

/// [`simplified_schema::resolve::prepare_embedded_schema_with_mode`] with
/// Darkmatter's shipped documents.
pub fn prepare_embedded_schema_with_mode(
    path: &str,
    documents: &[EmbeddedDocument<'_>],
    bundle: &str,
    mode: simplified_schema::shipped::PreparationMode<'_>,
) -> Result<(ResolvedSchema, Vec<PathBuf>), SchemaError> {
    simplified_schema::resolve::prepare_embedded_schema_with_mode(path, documents, bundle, SHIPPED_SOURCES, mode)
}

/// [`prepare_embedded_schema_in`] with no bundle confinement, for tests.
#[cfg(test)]
pub fn prepare_embedded_schema(path: &str, documents: &[EmbeddedDocument<'_>]) -> Result<ResolvedSchema, SchemaError> {
    prepare_embedded_schema_in(path, documents, "").map(|(resolved, _)| resolved)
}

/// [`simplified_schema::resolve::prepare_embedded_inline_schema`] with
/// Darkmatter's shipped documents.
pub fn prepare_embedded_inline_schema(
    value: &YamlValue,
    importer: &str,
    documents: &[EmbeddedDocument<'_>],
    bundle: &str,
) -> Result<(ResolvedSchema, Vec<PathBuf>), SchemaError> {
    simplified_schema::resolve::prepare_embedded_inline_schema(value, importer, documents, bundle, SHIPPED_SOURCES)
}

/// [`simplified_schema::resolve::prepare_embedded_inline_schema_with_mode`]
/// with Darkmatter's shipped documents.
pub fn prepare_embedded_inline_schema_with_mode(
    value: &YamlValue,
    importer: &str,
    documents: &[EmbeddedDocument<'_>],
    bundle: &str,
    mode: simplified_schema::shipped::PreparationMode<'_>,
) -> Result<(ResolvedSchema, Vec<PathBuf>), SchemaError> {
    simplified_schema::resolve::prepare_embedded_inline_schema_with_mode(value, importer, documents, bundle, SHIPPED_SOURCES, mode)
}

/// [`simplified_schema::resolve::prepare_embedded_catalog_declaration`] with
/// Darkmatter's shipped documents.
pub fn prepare_embedded_catalog_declaration(
    value: &YamlValue,
    importer: &str,
    documents: &[EmbeddedDocument<'_>],
    sharing: Sharing,
) -> Result<(ResolvedSchema, Vec<PathBuf>), SchemaError> {
    prepare_embedded_catalog_declaration_with_mode(value, importer, documents, sharing, PreparationMode::Runtime)
}

/// [`prepare_embedded_catalog_declaration`] in `mode`; shipped-artifact
/// generation passes [`PreparationMode::Uncached`].
pub fn prepare_embedded_catalog_declaration_with_mode(
    value: &YamlValue,
    importer: &str,
    documents: &[EmbeddedDocument<'_>],
    sharing: Sharing,
    mode: PreparationMode<'_>,
) -> Result<(ResolvedSchema, Vec<PathBuf>), SchemaError> {
    simplified_schema::resolve::prepare_embedded_catalog_declaration_with_mode(
        value,
        importer,
        documents,
        sharing,
        SHIPPED_SOURCES,
        mode,
    )
}

#[cfg(test)]
mod reserved_namespace {
    //! `Name@darkmatter:<stem>` names a type of a shipped partial whoever
    //! writes it: a disk schema, a Darkmatter embedded document, or a host
    //! bundle document. References inside the partial resolve among the
    //! shipped documents, also during a disk expansion.

    use std::collections::BTreeSet;

    use biscuit_file::FileResolutionContext;
    use serde_json::Value;

    use super::*;
    use crate::markdown::schemas::embedded;

    fn yaml(input: &str) -> YamlValue {
        serde_yaml_ng::from_str(input).expect("yaml parse")
    }

    /// The property names of the object `json` lowers to, or of its
    /// non-null `anyOf` arm.
    fn keys(json: &Value) -> BTreeSet<String> {
        let object = json["anyOf"]
            .as_array()
            .and_then(|arms| arms.iter().find(|arm| arm["type"] == "object"))
            .unwrap_or(json);
        object["properties"].as_object().expect("an object").keys().cloned().collect()
    }

    #[test]
    fn a_disk_schema_reaches_a_shipped_partial_and_records_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let context = FileResolutionContext::new(dir.path());
        let resolved = resolve_yaml_schema(&yaml("talk: communicate@darkmatter:lifecycle\n"), dir.path(), &context)
            .expect("the reserved reference resolves");
        assert!(keys(&resolved.json_schema["properties"]["talk"]).contains("say"));
        assert!(resolved.referenced_files.is_empty(), "{:?}", resolved.referenced_files);
        assert!(resolved.imports.is_empty(), "a shipped partial is no file dependency: {:?}", resolved.imports);
    }

    #[test]
    fn a_disk_library_file_reaches_a_shipped_partial_from_any_folder() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a/b");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("types.yaml"), "kind: schema\ntypes:\n  tally: count@darkmatter:lifecycle\n").unwrap();
        let context = FileResolutionContext::new(dir.path());
        let resolved = resolve_yaml_schema(&yaml("n: tally@./a/b/types.yaml\n"), dir.path(), &context)
            .expect("the library's reserved reference resolves");
        let arms = resolved.json_schema["properties"]["n"]["anyOf"].as_array().expect("count is a union");
        assert!(arms.iter().any(|arm| arm["type"] == "integer"), "{arms:?}");
        assert!(arms.iter().any(|arm| arm["pattern"].is_string()), "count's `interpolation` arm resolved locally");
        assert_eq!(resolved.imports, vec![canonical_path(&nested.join("types.yaml"))]);
    }

    #[test]
    fn relative_references_inside_a_reached_partial_stay_in_the_shipped_table() {
        // md-lifecycle.yaml names its verbs `info-short@./lifecycle.yaml`;
        // the importer's folder holds a decoy that must not be read.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("lifecycle.yaml"), "kind: schema\ntypes:\n  info-short: number\n").unwrap();
        let context = FileResolutionContext::new(dir.path());
        let resolved =
            resolve_yaml_schema(&yaml("talk: md-communicate@darkmatter:md-lifecycle\n"), dir.path(), &context)
                .expect("the partial's own imports resolve");
        let talk = &resolved.json_schema["properties"]["talk"];
        let object = talk["anyOf"].as_array().and_then(|arms| arms.iter().find(|arm| arm["type"] == "object")).unwrap_or(talk);
        let info = object["properties"]["info"].to_string();
        assert!(info.contains("\"string\"") && !info.contains("\"number\""), "{info}");
        assert!(resolved.imports.is_empty(), "{:?}", resolved.imports);
    }

    #[test]
    fn a_darkmatter_embedded_document_reaches_a_shipped_partial() {
        let (resolved, reads) = prepare_embedded_inline_schema(
            &yaml("talk: communicate@darkmatter:lifecycle\n"),
            embedded::SHIPPED[0].path,
            embedded::SHIPPED,
            "",
        )
        .expect("resolves in the shipped table");
        assert!(keys(&resolved.json_schema["properties"]["talk"]).contains("info"));
        assert!(reads.contains(&PathBuf::from("darkmatter/schemas/partials/lifecycle.yaml")), "{reads:?}");
    }

    #[test]
    fn a_host_bundle_document_reaches_a_shipped_partial_outside_its_bundle() {
        let documents = [EmbeddedDocument {
            path: "host/actions.yaml",
            text: "kind: schema\ntypes:\n  talk: communicate@darkmatter:lifecycle\n",
        }];
        let (resolved, _) = prepare_embedded_inline_schema(
            &yaml("talk: talk@./actions.yaml\n"),
            "host/trigger.yaml",
            &documents,
            "host",
        )
        .expect("the bundle's reserved reference resolves");
        assert!(keys(&resolved.json_schema["properties"]["talk"]).contains("say"));
        // A plain relative climb out of the bundle is still refused.
        let escape = prepare_embedded_inline_schema(
            &yaml("talk: communicate@../darkmatter/schemas/partials/lifecycle.yaml\n"),
            "host/trigger.yaml",
            &documents,
            "host",
        );
        assert!(escape.is_err(), "{escape:?}");
    }

    #[test]
    fn an_unknown_stem_is_an_error_naming_the_reference() {
        let dir = tempfile::tempdir().unwrap();
        let context = FileResolutionContext::new(dir.path());
        for reference in ["darkmatter:bogus", "darkmatter:", "darkmatter:../fm", "darkmatter:fm.yaml"] {
            let error = resolve_yaml_schema(&yaml(&format!("x: \"t@{reference}\"\n")), dir.path(), &context)
                .expect_err("an unknown stem fails");
            let SchemaError::NoMatch { reference: named, .. } = &error else {
                panic!("`{reference}`: {error:?}")
            };
            assert_eq!(named, reference);
            assert!(error.to_string().contains(reference), "{error}");
        }
        let embedded = prepare_embedded_inline_schema(&yaml("x: t@darkmatter:bogus\n"), "host/a.yaml", &[], "host");
        assert!(matches!(embedded, Err(SchemaError::NoMatch { .. })), "{embedded:?}");
        let missing_type = resolve_yaml_schema(&yaml("x: nope@darkmatter:lifecycle\n"), dir.path(), &context);
        assert!(
            matches!(&missing_type, Err(SchemaError::UnknownNamedType { name, .. }) if name == "nope"),
            "{missing_type:?}"
        );
    }

    #[test]
    fn a_reserved_reference_names_no_host_file() {
        let dir = tempfile::tempdir().unwrap();
        let context = FileResolutionContext::new(dir.path());
        assert_eq!(named_type_reference_path("darkmatter:lifecycle", dir.path(), &[], &context).unwrap(), None);
        assert_eq!(named_type_reference_path(" darkmatter:bogus ", dir.path(), &[], &context).unwrap(), None);
    }
}

