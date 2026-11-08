//! The `kind: yaml-schema` standalone envelope: a definition of a YAML
//! document kind. The library parses it like a `kind: schema` document (same
//! metadata and `types` namespace, a required `$schema` export), but it is
//! reached only through `kind`: naming it as a `$schema` target or a
//! `Name@file` import source is refused as
//! [`NotSchemaReason::SelectedByKind`], on every load path.

use simplified_schema::embedded::{EmbeddedDocument, ShippedSources};
use simplified_schema::resolve::{inspect_schema_target, prepare_embedded_schema};
use simplified_schema::{
    NotSchemaReason, SchemaCategory, SchemaError, SchemaRoot, SimplifiedSchema, StandaloneSchemaEnvelope,
    parse_standalone_schema_document,
};

/// A user kind, in the shape `md` users write: local `types`, a claimed
/// `kind`, and metadata.
const ROSTER: &str = "kind: yaml-schema\ndescription: A roster of platforms.\ncategories: [Core]\ntypes:\n  platform:\n    name: string(required)\n$schema:\n  kind: literal(roster; required)\n  roster_version: literal(1; required)\n  platforms: platform[](required)\n";

/// The first `NotASchemaDocument` in `error`'s source chain.
fn not_schema(error: &SchemaError) -> Option<NotSchemaReason> {
    let mut cause: &dyn std::error::Error = error;
    loop {
        if let Some(SchemaError::NotASchemaDocument { reason, .. }) = cause.downcast_ref::<SchemaError>() {
            return Some(*reason);
        }
        cause = cause.source()?;
    }
}

#[test]
fn a_yaml_schema_parses_as_a_standalone_document_with_a_types_namespace() {
    let document = parse_standalone_schema_document(ROSTER, "roster.yaml")
        .expect("admitted")
        .expect("recognized");
    assert_eq!(document.envelope, StandaloneSchemaEnvelope::YamlSchema);
    assert!(document.exports_schema);
    assert_eq!(document.description.as_deref(), Some("A roster of platforms."));
    assert_eq!(document.categories, Some(vec![SchemaCategory { name: "Core".into(), description: None }]));
    // `types` is the local namespace, not exported properties.
    let names = document.named_types().expect("a types table");
    assert!(names.properties.contains_key("platform"));
    let Some(SchemaRoot::Single(export)) = document.schema() else {
        panic!("a single exported root");
    };
    assert!(export.properties.contains_key("platforms") && !export.properties.contains_key("platform"));
}

#[test]
fn the_envelope_admits_kind_and_names_its_claiming_value() {
    assert!(StandaloneSchemaEnvelope::YamlSchema.admits_top_level_key("kind"));
    assert!(StandaloneSchemaEnvelope::Tagged.admits_top_level_key("kind"));
    assert!(!StandaloneSchemaEnvelope::Pure.admits_top_level_key("kind"));
    assert_eq!(StandaloneSchemaEnvelope::YamlSchema.kind(), Some("yaml-schema"));
    assert_eq!(StandaloneSchemaEnvelope::Tagged.kind(), Some("schema"));
    assert_eq!(StandaloneSchemaEnvelope::Pure.kind(), None);
}

#[test]
fn a_yaml_schema_without_an_export_is_rejected_by_its_definition() {
    let text = "kind: yaml-schema\ntypes:\n  platform:\n    name: string\n";
    match parse_standalone_schema_document(text, "roster.yaml") {
        Err(SchemaError::SchemaDocument { message, span, .. }) => {
            assert!(message.contains("simplified-schema/schemas/yaml-schema.yaml"), "{message}");
            assert!(message.contains("\"$schema\" is a required property"), "{message}");
            assert_eq!(span.map(|span| &text[span]), Some("kind: yaml-schema"));
        }
        other => panic!("expected a located envelope error, got {other:?}"),
    }
}

#[test]
fn the_factories_load_a_yaml_schema_and_export_it_in_its_envelope() {
    let schema = SimplifiedSchema::try_from(ROSTER).expect("a direct load accepts the envelope");
    assert!(schema.property("platforms").is_some());
    assert!(schema.named_type("platform").is_some());
    let exported = schema.to_simplified_yaml().expect("exports");
    assert!(exported.starts_with("kind: yaml-schema\n"), "{exported}");
    let reparsed = parse_standalone_schema_document(&exported, "roster.yaml").expect("admitted").expect("recognized");
    assert_eq!(reparsed.envelope, StandaloneSchemaEnvelope::YamlSchema);
    assert_eq!(SimplifiedSchema::try_from(exported.as_str()).unwrap().to_simplified_yaml().unwrap(), exported);
}

#[test]
fn an_embedded_yaml_schema_is_refused_as_a_schema_target_and_an_import_source() {
    const DOCUMENTS: &[EmbeddedDocument<'static>] = &[
        EmbeddedDocument { path: "kinds/roster.yaml", text: ROSTER },
        EmbeddedDocument { path: "kinds/consumer.yaml", text: "$schema:\n  platform: platform@./roster.yaml\n" },
    ];
    let error = prepare_embedded_schema("kinds/roster.yaml", DOCUMENTS, ShippedSources::NONE)
        .expect_err("an embedded yaml-schema is not a `$schema` target");
    assert_eq!(not_schema(&error), Some(NotSchemaReason::SelectedByKind), "{error}");
    assert!(error.to_string().contains("it is selected by `kind`, not referenced"), "{error}");

    let error = prepare_embedded_schema("kinds/consumer.yaml", DOCUMENTS, ShippedSources::NONE)
        .expect_err("a yaml-schema is not an import source");
    assert_eq!(not_schema(&error), Some(NotSchemaReason::SelectedByKind), "{error}");
}

#[test]
fn target_inspection_reports_a_yaml_schema_as_selected_by_kind() {
    let inspection = inspect_schema_target(std::path::Path::new("roster.yaml"), ROSTER.as_bytes());
    match inspection.classification {
        Err(SchemaError::NotASchemaDocument { reason, .. }) => assert_eq!(reason, NotSchemaReason::SelectedByKind),
        other => panic!("expected SelectedByKind, got {other:?}"),
    }
    assert_eq!(inspection.lines, None);
}

#[cfg(feature = "file")]
mod files {
    use std::fs;
    use std::path::PathBuf;

    use biscuit_file::{FileResolutionContext, canonicalize_simplified};
    use serde_json::json;
    use simplified_schema::embedded::ShippedSources;
    use simplified_schema::resolve::{ResolvedSchema, resolve_schema_with_roots};
    use simplified_schema::{NotSchemaReason, SchemaError};

    use super::{ROSTER, not_schema};

    struct Fixture {
        _tmp: tempfile::TempDir,
        dir: PathBuf,
        context: FileResolutionContext,
    }

    impl Fixture {
        fn new() -> Self {
            let tmp = tempfile::tempdir().unwrap();
            let dir = canonicalize_simplified(tmp.path()).unwrap();
            let context = FileResolutionContext::new(&dir);
            Self { _tmp: tmp, dir, context }
        }

        fn write(&self, name: &str, text: &str) -> PathBuf {
            let path = self.dir.join(name);
            fs::write(&path, text).unwrap();
            path
        }

        fn resolve(&self, reference: &str) -> Result<ResolvedSchema, SchemaError> {
            resolve_schema_with_roots(&json!(reference), &self.dir, &[], &self.context, ShippedSources::NONE)
        }
    }

    #[track_caller]
    fn assert_selected_by_kind(result: Result<ResolvedSchema, SchemaError>, path: &std::path::Path) {
        let error = result.expect_err("a yaml-schema is reached only through `kind`");
        let mut cause: &dyn std::error::Error = &error;
        loop {
            if let Some(SchemaError::NotASchemaDocument { path: named, reason }) = cause.downcast_ref::<SchemaError>() {
                assert_eq!((named.as_path(), *reason), (path, NotSchemaReason::SelectedByKind), "{error}");
                return;
            }
            cause = cause.source().unwrap_or_else(|| panic!("no NotASchemaDocument in: {error}"));
        }
    }

    #[test]
    fn a_yaml_schema_is_not_a_schema_target() {
        let fixture = Fixture::new();
        let path = fixture.write("roster.yaml", ROSTER);
        assert_selected_by_kind(fixture.resolve("./roster.yaml"), &path);
        // A delegation and a root-union arm are the same refusal.
        fixture.write("delegate.yaml", "$schema: ./roster.yaml\n");
        assert_selected_by_kind(fixture.resolve("./delegate.yaml"), &path);
        fixture.write("union.yaml", "$schema:\n  - ./roster.yaml\n  - count: number\n");
        assert_selected_by_kind(fixture.resolve("./union.yaml"), &path);
    }

    #[test]
    fn a_malformed_yaml_schema_is_still_refused_as_selected_by_kind() {
        let fixture = Fixture::new();
        let path = fixture.write("broken.yaml", "kind: yaml-schema\nnotes: draft\n");
        assert_selected_by_kind(fixture.resolve("./broken.yaml"), &path);
    }

    #[test]
    fn a_yaml_schema_is_not_a_named_type_import_source() {
        let fixture = Fixture::new();
        let path = fixture.write("roster.yaml", ROSTER);
        fixture.write("consumer.yaml", "$schema:\n  platform: platform@./roster.yaml\n");
        assert_selected_by_kind(fixture.resolve("./consumer.yaml"), &path);
        // The same types in a `kind: schema` file import, and a `types`-only
        // one stays an invalid `$schema` target.
        fixture.write("shared.yaml", "kind: schema\ntypes:\n  platform:\n    name: string(required)\n");
        fixture.write("consumer.yaml", "$schema:\n  platform: platform@./shared.yaml\n");
        fixture.resolve("./consumer.yaml").expect("a kind: schema file is an import source");
        let error = fixture.resolve("./shared.yaml").expect_err("types-only is not a target");
        assert_eq!(not_schema(&error), Some(NotSchemaReason::TypesOnly), "{error}");
    }

    #[test]
    fn kinded_data_and_triggers_stay_non_targets() {
        let fixture = Fixture::new();
        fixture.write("platforms.yaml", "kind: roster\nroster_version: 1\nplatforms: []\n");
        let error = fixture.resolve("./platforms.yaml").expect_err("kinded data is not a schema target");
        assert_eq!(not_schema(&error), Some(NotSchemaReason::NoSchemaProperty), "{error}");
        // A trigger carrying a `$schema` payload is refused too: its `kind`
        // and `match` are keys no schema envelope admits.
        fixture.write("trigger.yaml", "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema:\n  title: string\n");
        match fixture.resolve("./trigger.yaml") {
            Err(SchemaError::SchemaDocument { message, .. }) => {
                assert!(message.contains("unsupported top-level key(s): kind, match"), "{message}");
            }
            other => panic!("a trigger must not be a `$schema` target: {other:?}"),
        }
    }
    /// A definition on disk (or unsaved text for it) prepares like the
    /// embedded ones: its `Name@file` imports resolve from its own folder,
    /// it claims its kinds, and it is a dependency of its own preparation.
    #[test]
    fn a_yaml_schema_on_disk_prepares_its_export_with_file_imports() {
        use simplified_schema::resolve::prepare_yaml_schema_text;
        let fixture = Fixture::new();
        fixture.write("platform.yaml", "kind: schema\ntypes:\n  platform:\n    name: string(required)\n");
        let text = ROSTER.replace("  platform:\n    name: string(required)\n", "  platform: platform@./platform.yaml\n");
        assert_ne!(text, ROSTER);
        let path = fixture.write("roster.yaml", &text);
        let resolved = prepare_yaml_schema_text(&path, &text, &[], &fixture.context, ShippedSources::NONE)
            .expect("prepares");
        assert_eq!(simplified_schema::claimed_values(&resolved, "kind").unwrap(), ["roster"]);
        assert!(resolved.referenced_files.contains(&canonicalize_simplified(&path).unwrap()));
        assert!(resolved.imports.iter().any(|import| import.ends_with("platform.yaml")), "{:?}", resolved.imports);

        // Not a yaml-schema: refused, never prepared as a schema export.
        let schema = "kind: schema\n$schema:\n  a: string\n";
        let error = prepare_yaml_schema_text(&path, schema, &[], &fixture.context, ShippedSources::NONE).unwrap_err();
        assert_eq!(not_schema(&error), Some(NotSchemaReason::NoSchemaProperty), "{error}");
        // An import that does not resolve is the preparation's error.
        let broken = text.replace("./platform.yaml", "./absent.yaml");
        assert!(prepare_yaml_schema_text(&path, &broken, &[], &fixture.context, ShippedSources::NONE).is_err());
    }
}

#[test]
fn a_kind_selection_error_names_the_kind_and_its_cause() {
    use simplified_schema::KindSelectionCause;
    use std::path::PathBuf;
    let duplicate = SchemaError::KindSelection {
        kind: "roster".into(),
        cause: KindSelectionCause::Duplicate { files: vec![PathBuf::from("a.yaml"), PathBuf::from("b.yaml")] },
    };
    assert_eq!(
        duplicate.to_string(),
        "cannot select a schema for `kind: roster`: it is claimed by more than one definition: `a.yaml`, `b.yaml`"
    );
    let unavailable = SchemaError::KindSelection {
        kind: "inventory".into(),
        cause: KindSelectionCause::Unavailable { file: PathBuf::from("broken.yaml"), defect: "not valid YAML".into(), code: None },
    };
    assert!(unavailable.to_string().contains("schema selection is unavailable: definition `broken.yaml`"), "{unavailable}");
}
