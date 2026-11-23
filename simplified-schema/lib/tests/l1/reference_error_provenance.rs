//! A failure about a reference that names a file carries the declaration
//! that wrote it ([`SchemaError::reference_site`]): the source and the path
//! of the declaration where the reference was first read, in the file that
//! wrote it even after expansion inlined that declaration elsewhere, and no
//! site when two declarations wrote the same spelling.
#![cfg(feature = "file")]

use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use simplified_schema::embedded::ShippedSources;
use simplified_schema::resolve::{check_declared_types, resolve_yaml_schema};
use simplified_schema::{SchemaError, SchemaReferenceSite, SchemaSourceId, SchemaSourcePath};

/// `lib.yaml` declares `widget`, whose `size` names an absent example file.
const LIBRARY: &str = "kind: schema\ntypes:\n    widget:\n        size: string(example(./absent.yaml))\n";

fn folder() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    std::fs::write(root.join("lib.yaml"), LIBRARY).unwrap();
    (dir, root)
}

fn resolve_error(root: &Path, payload: &str) -> SchemaError {
    let payload: serde_yaml_ng::Value = serde_yaml_ng::from_str(payload).unwrap();
    let context = FileResolutionContext::new(root);
    resolve_yaml_schema(&payload, root, &context, ShippedSources::NONE).expect_err("the reference fails")
}

fn inline(path: SchemaSourcePath) -> SchemaReferenceSite {
    SchemaReferenceSite { source: SchemaSourceId::InlineDocument, path }
}

#[test]
fn an_import_of_a_missing_file_carries_the_declaration_that_wrote_it() {
    let (_dir, root) = folder();
    let error = resolve_error(&root, "w: Widget@./absent.yaml\n");
    assert!(matches!(error, SchemaError::NoMatch { .. }), "{error:?}");
    assert_eq!(error.reference_site(), Some(&inline(SchemaSourcePath::root().property("w"))));
}

#[test]
fn an_example_in_a_union_arm_carries_the_arm_that_wrote_it() {
    let (_dir, root) = folder();
    let error = resolve_error(&root, "a:\n  - number\n  - string(example(./absent.yaml))\n");
    assert_eq!(error.reference_site(), Some(&inline(SchemaSourcePath::root().property("a").union_arm(1))));
}

/// The library's declaration is inlined into the importer before examples
/// are judged; the failure still names the library's declaration.
#[test]
fn an_inherited_example_error_carries_the_library_declaration_that_wrote_it() {
    let (_dir, root) = folder();
    let error = resolve_error(&root, "w: widget@./lib.yaml\n");
    let SchemaError::ReferencedSchema { path, source } = &error else { panic!("named in the library: {error:?}") };
    assert_eq!(path, &root.join("lib.yaml"));
    let written = SchemaReferenceSite {
        source: SchemaSourceId::File(root.join("lib.yaml")),
        path: SchemaSourcePath::root().property("widget").property("size"),
    };
    assert_eq!(source.reference_site(), Some(&written), "{error:?}");
}

/// The control: a spelling two declarations wrote has no single writer.
#[test]
fn an_example_error_two_declarations_wrote_carries_no_site() {
    let (_dir, root) = folder();
    let error = resolve_error(&root, "a: string(example(./absent.yaml))\nb: string(example(./absent.yaml))\n");
    assert!(matches!(error, SchemaError::NoMatch { .. }), "{error:?}");
    assert_eq!(error.reference_site(), None);
}

/// An unused declaration checked on its own carries its `types` entry.
#[test]
fn a_declared_type_error_carries_its_types_entry() {
    let (_dir, root) = folder();
    let path = root.join("lib.yaml");
    let error = check_declared_types(LIBRARY, &path, &FileResolutionContext::new(&root), ShippedSources::NONE).expect_err("refused");
    let written = SchemaReferenceSite { source: SchemaSourceId::File(path), path: SchemaSourcePath::root().property("widget").property("size") };
    assert_eq!(error.reference_site(), Some(&written), "{error:?}");
}

// ── A selected file refused for the role a declaration named it in ─────────

/// Writes `files` into a fresh folder and resolves `payload` from it.
fn refusal(files: &[(&str, &str)], payload: &str) -> (tempfile::TempDir, PathBuf, SchemaError) {
    let (dir, root) = folder();
    for (name, text) in files {
        std::fs::write(root.join(name), text).unwrap();
    }
    let error = resolve_error(&root, payload);
    (dir, root, error)
}

const TYPES_ONLY: &str = "kind: schema\ntypes:\n  stamp: string\n";
const YAML_KIND: &str = "kind: yaml-schema\nclaims:\n  kind: [roster]\n$schema:\n  roster_version: number\n";
const NAMESPACE: &str = "kind: namespace-schema\nnamespace: loop\n$schema:\n  count: number\n";
const TRIGGER: &str = "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema:\n  title: string\n";

/// A valid file named as a document schema in a role it cannot serve is
/// refused at the scalar or union arm that named it, with its role kept.
#[test]
fn a_listed_file_refused_as_a_document_schema_carries_the_declaration_that_named_it() {
    use simplified_schema::NotSchemaReason;
    for (name, text, reason) in [
        ("shared.yaml", TYPES_ONLY, NotSchemaReason::TypesOnly),
        ("formal.yaml", YAML_KIND, NotSchemaReason::SelectedByKind),
        ("loop.yaml", NAMESPACE, NotSchemaReason::NamespaceCatalog),
        ("trigger.yaml", TRIGGER, NotSchemaReason::TriggerEnvelope),
    ] {
        for (payload, site) in [
            (format!("./{name}"), SchemaSourcePath::root()),
            (format!("- title: string\n- ./{name}\n"), SchemaSourcePath::root().union_arm(1)),
        ] {
            let (_dir, root, error) = refusal(&[(name, text)], &payload);
            let SchemaError::NotASchemaDocument { path, reason: actual, .. } = &error else { panic!("{name}: {error:?}") };
            assert_eq!((path, *actual), (&root.join(name), reason), "{name}");
            assert!(error.to_string().contains(name), "the message names the target: {error}");
            assert_eq!(error.reference_site(), Some(&inline(site)), "{name} {payload:?}");
        }
    }
}

/// A JSON Schema in another dialect is refused at the declaration that
/// named it; one the Draft 2020-12 meta-schema rejects, and a file that is
/// no schema document at all, are defects of their own content and carry
/// no declaration.
#[test]
fn a_json_target_refused_for_its_dialect_carries_the_declaration_and_a_defective_one_does_not() {
    let files = [
        ("draft7.json", r#"{"$schema": "http://json-schema.org/draft-07/schema#", "type": "object"}"#),
        ("invalid.json", r#"{"$schema": "https://json-schema.org/draft/2020-12/schema", "type": 7}"#),
    ];
    let (_dir, _root, error) = refusal(&files, "./draft7.json");
    assert!(matches!(error, SchemaError::UnsupportedJsonDialect { .. }), "{error:?}");
    assert_eq!(error.reference_site(), Some(&inline(SchemaSourcePath::root())));
    let (_dir, _root, error) = refusal(&files, "- title: string\n- ./invalid.json\n");
    assert!(matches!(error, SchemaError::InvalidJsonSchema { .. }), "{error:?}");
    assert_eq!(error.reference_site(), None);
    let (_dir, _root, error) = refusal(&[("notes.yaml", "title: notes\n")], "./notes.yaml");
    assert!(matches!(error, SchemaError::NotASchemaDocument { .. }), "{error:?}");
    assert_eq!(error.reference_site(), None);
}

/// Through a delegating file, the refusal belongs to the delegating file's
/// `$schema`, the declaration that named the refused file, not the
/// document's.
#[test]
fn a_target_refused_behind_a_delegating_file_carries_the_delegating_declaration() {
    let (_dir, root, error) = refusal(&[("shared.yaml", TYPES_ONLY), ("alias.yaml", "$schema: ./shared.yaml\n")], "./alias.yaml");
    assert!(matches!(error, SchemaError::NotASchemaDocument { .. }), "{error:?}");
    let written = SchemaReferenceSite { source: SchemaSourceId::File(root.join("alias.yaml")), path: SchemaSourcePath::root() };
    assert_eq!(error.reference_site(), Some(&written));
}

/// A `Name@file` import of a file that cannot supply named types is
/// refused at the import, in the file that wrote it; an import a library
/// wrote arrives wrapped in the library's `ReferencedSchema`.
#[test]
fn an_import_of_a_file_that_cannot_supply_types_carries_the_import_that_named_it() {
    use simplified_schema::NotSchemaReason;
    let (_dir, _root, error) = refusal(&[("trigger.yaml", TRIGGER)], "w: title@./trigger.yaml\n");
    assert!(matches!(error, SchemaError::NotASchemaDocument { reason: NotSchemaReason::TriggerEnvelope, .. }), "{error:?}");
    assert_eq!(error.reference_site(), Some(&inline(SchemaSourcePath::root().property("w"))));

    let library = "kind: schema\ntypes:\n  stamped:\n    last_updated: stamp@./loop.yaml\n";
    let (_dir, root, error) = refusal(&[("loop.yaml", NAMESPACE), ("stamps.yaml", library)], "w: stamped@./stamps.yaml\n");
    let SchemaError::ReferencedSchema { path, source } = &error else { panic!("named in the library: {error:?}") };
    assert_eq!(path, &root.join("stamps.yaml"));
    assert!(matches!(**source, SchemaError::NotASchemaDocument { reason: NotSchemaReason::NamespaceCatalog, .. }), "{source:?}");
    let written = SchemaReferenceSite {
        source: SchemaSourceId::File(root.join("stamps.yaml")),
        path: SchemaSourcePath::root().property("stamped").property("last_updated"),
    };
    assert_eq!(source.reference_site(), Some(&written));
}

/// A refused trigger file keeps the declaration through its conversion
/// into the trigger-specific document error.
#[test]
fn a_trigger_file_refusal_keeps_its_declaration_as_a_trigger_reference() {
    use simplified_schema::triggers::assemble::{refused_trigger_file, trigger_schema_referenced};
    let (_dir, root, error) = refusal(&[("composition.trigger.yaml", TRIGGER)], "- title: string\n- ./composition.trigger.yaml\n");
    let refused = refused_trigger_file(&error).expect("a trigger file");
    assert_eq!(refused.path, root.join("composition.trigger.yaml"));
    let converted = trigger_schema_referenced(&refused.path, refused.site);
    assert!(converted.to_string().contains("reference the payload `composition.yaml` instead"), "{converted}");
    assert_eq!(converted.reference_site(), Some(&inline(SchemaSourcePath::root().union_arm(1))));
}

/// The file whose reference closes a `$schema` cycle is the declaration a
/// cycle is reported at.
#[test]
fn a_reference_cycle_carries_the_declaration_that_closed_it() {
    let (_dir, root, error) = refusal(&[("a.yaml", "$schema: ./b.yaml\n"), ("b.yaml", "$schema: ./a.yaml\n")], "./a.yaml");
    assert!(matches!(error, SchemaError::ReferenceCycle { .. }), "{error:?}");
    let written = SchemaReferenceSite { source: SchemaSourceId::File(root.join("b.yaml")), path: SchemaSourcePath::root() };
    assert_eq!(error.reference_site(), Some(&written));
}

/// The control: a target that does not parse is refused for its own
/// content, naming the target, with no declaration; a host places it at the
/// parser's position in that file.
#[test]
fn a_target_that_does_not_parse_keeps_its_own_position() {
    let (_dir, root, error) = refusal(&[("broken.yaml", "$schema:\n  title: [\n")], "./broken.yaml");
    let SchemaError::SchemaDocument { path, message, .. } = &error else { panic!("{error:?}") };
    assert_eq!(path, &root.join("broken.yaml"));
    assert!(message.starts_with("not valid YAML"), "{error:?}");
    assert_eq!(error.reference_site(), None);
}

/// A composition operand naming a file that cannot supply named types
/// (`$extends`, `...spread`) is refused at the operand, under the
/// composition error that reached it.
#[test]
fn a_composition_operand_naming_a_trigger_carries_the_operand_that_named_it() {
    use simplified_schema::NotSchemaReason;
    for payload in ["w:\n  $extends: title@./trigger.yaml\n  extra: string\n", "w: \"{ ...title@./trigger.yaml, extra: string }\"\n"] {
        let (_dir, _root, error) = refusal(&[("trigger.yaml", TRIGGER)], payload);
        let mut cause: &dyn std::error::Error = &error;
        let refused = loop {
            if let Some(found @ SchemaError::NotASchemaDocument { reason: NotSchemaReason::TriggerEnvelope, .. }) =
                cause.downcast_ref::<SchemaError>().or_else(|| cause.downcast_ref::<Box<SchemaError>>().map(|boxed| &**boxed))
            {
                break found;
            }
            cause = cause.source().unwrap_or_else(|| panic!("{payload}: no trigger refusal in {error:?}"));
        };
        let site = refused.reference_site().unwrap_or_else(|| panic!("{payload}: {refused:?}"));
        assert_eq!(site.source, SchemaSourceId::InlineDocument, "{payload}");
        assert!(site.path.to_string().starts_with('w'), "{payload}: {site:?}");
    }
}
