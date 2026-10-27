//! Public resolution evidence for the process-wide named-type memo: each
//! imported file keeps only its latest successfully parsed revision, every
//! use still reads the file, and a failed read or parse is never answered
//! with an earlier table.
//!
//! Parse counts are asserted only with the `work-counters` feature; the
//! freshness assertions run either way.

use std::path::Path;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::schemas::SchemaError;
use darkmatter::markdown::schemas::resolve::{ResolvedSchema, resolve_yaml_schema};
use serde_json::{Value, json};
use serde_yaml_ng::Value as YamlValue;

/// The frozen fixture each case edits once.
const STRING_TYPES: &str = "kind: schema\ntypes:\n  name: string\n";
/// The one edit that changes the parsed table.
const NUMBER_TYPES: &str = "kind: schema\ntypes:\n  name: number\n";

/// Resolves an inline schema importing `name` from `./{file}` in `dir`.
fn import(dir: &Path, file: &str) -> Result<ResolvedSchema, SchemaError> {
    let schema: YamlValue = serde_yaml_ng::from_str(&format!("value: name@./{file}\n")).expect("fixture YAML parses");
    resolve_yaml_schema(&schema, dir, &FileResolutionContext::new(dir))
}

/// The JSON Schema type the imported `value` property admits besides null.
fn imported_type(result: Result<ResolvedSchema, SchemaError>) -> Value {
    let resolved = result.expect("the import resolves");
    resolved.json_schema["properties"]["value"]["anyOf"][1]["type"].clone()
}

/// The counters a parse count is measured from; nothing without the
/// `work-counters` feature, where [`assert_parses`] checks nothing.
#[cfg(feature = "work-counters")]
type Parses = darkmatter::work_counters::WorkSnapshot;
#[cfg(not(feature = "work-counters"))]
struct Parses;

fn parses_now() -> Parses {
    #[cfg(feature = "work-counters")]
    return darkmatter::work_counters::snapshot();
    #[cfg(not(feature = "work-counters"))]
    Parses
}

#[cfg_attr(not(feature = "work-counters"), allow(unused_variables))]
fn assert_parses(label: &str, before: &Parses, expected: u64) {
    #[cfg(feature = "work-counters")]
    assert_eq!(
        darkmatter::work_counters::snapshot()
            .since(before)
            .get(darkmatter::work_counters::Work::NamedTypeTableParse),
        expected,
        "{label}: named-type table parses"
    );
}

/// More than the 256 paths the memo once held before clearing itself:
/// reuse parses nothing, and one edit reparses only the edited file.
#[cfg(feature = "work-counters")]
#[test]
fn named_type_memo_reuses_every_path_past_256_files_and_reparses_only_an_edit() {
    const FILES: usize = 300;
    let dir = tempfile::tempdir().expect("tempdir");
    let mut wiring = serde_yaml_ng::Mapping::new();
    for index in 0..FILES {
        std::fs::write(dir.path().join(format!("t{index}.yaml")), format!("kind: schema\ntypes:\n  t{index}: string\n"))
            .expect("write types");
        wiring.insert(format!("p{index}").into(), format!("t{index}@./t{index}.yaml").into());
    }
    let wiring = YamlValue::Mapping(wiring);
    let context = FileResolutionContext::new(dir.path());
    let resolve = || resolve_yaml_schema(&wiring, dir.path(), &context).expect("every import resolves");

    let cold = parses_now();
    resolve();
    assert_parses("cold", &cold, FILES as u64);

    let reuse = parses_now();
    resolve();
    assert_parses("reuse", &reuse, 0);

    std::fs::write(dir.path().join("t0.yaml"), "kind: schema\ntypes:\n  t0: number\n").expect("edit t0");
    let edited = parses_now();
    let resolved = resolve();
    assert_parses("after one edit", &edited, 1);
    assert_eq!(resolved.json_schema["properties"]["p0"]["anyOf"][1]["type"], json!("number"));
    assert_eq!(resolved.json_schema["properties"]["p299"]["anyOf"][1]["type"], json!("string"));

    let reuse_after_edit = parses_now();
    resolve();
    assert_parses("reuse after the edit", &reuse_after_edit, 0);
}

/// A previously successful file turned malformed fails with the located
/// classifier error, an unedited control stays reused, and restoring the
/// original text parses it again rather than reviving the dropped entry.
#[test]
fn a_malformed_revision_after_a_good_one_is_a_located_error_and_never_the_old_table() {
    let dir = tempfile::tempdir().expect("tempdir");
    let types = dir.path().join("types.yaml");
    std::fs::write(&types, STRING_TYPES).expect("write types");
    std::fs::write(dir.path().join("control.yaml"), STRING_TYPES).expect("write control");
    assert_eq!(imported_type(import(dir.path(), "types.yaml")), json!("string"));
    assert_eq!(imported_type(import(dir.path(), "control.yaml")), json!("string"));

    std::fs::write(&types, "kind: schema\ntypes: [\n").expect("break types");
    match import(dir.path(), "types.yaml") {
        Err(SchemaError::SchemaDocument { path, message }) => {
            assert_eq!(
                biscuit_file::canonicalize_simplified(&path).unwrap_or(path),
                biscuit_file::canonicalize_simplified(&types).expect("types canonicalizes")
            );
            assert!(message.starts_with("not valid YAML"), "{message}");
        }
        other => panic!("a malformed revision must fail, got {:?}", other.map(|resolved| resolved.json_schema)),
    }

    let control = parses_now();
    assert_eq!(imported_type(import(dir.path(), "control.yaml")), json!("string"));
    assert_parses("unedited control", &control, 0);

    std::fs::write(&types, STRING_TYPES).expect("repair types");
    let repaired = parses_now();
    assert_eq!(imported_type(import(dir.path(), "types.yaml")), json!("string"));
    assert_parses("repaired", &repaired, 1);
}

/// A malformed file never resolved before fails the same way cold.
#[test]
fn a_malformed_file_fails_cold_and_recovers_once_repaired() {
    let dir = tempfile::tempdir().expect("tempdir");
    let types = dir.path().join("types.yaml");
    std::fs::write(&types, "kind: schema\ntypes:\n  name: string\n]\n").expect("write malformed types");

    assert!(
        matches!(import(dir.path(), "types.yaml"), Err(SchemaError::SchemaDocument { .. })),
        "a cold malformed import fails"
    );

    std::fs::write(&types, NUMBER_TYPES).expect("repair types");
    let repaired = parses_now();
    assert_eq!(imported_type(import(dir.path(), "types.yaml")), json!("number"));
    assert_parses("repaired", &repaired, 1);
}

/// Deleting a previously successful file is an error naming the reference,
/// cold or warm; a file recreated with new text resolves to that text.
#[test]
fn a_deleted_file_is_an_error_never_the_old_table() {
    let dir = tempfile::tempdir().expect("tempdir");
    let types = dir.path().join("types.yaml");

    let cold = import(dir.path(), "types.yaml");
    assert!(matches!(cold, Err(SchemaError::NoMatch { .. })), "cold: {:?}", cold.map(|r| r.json_schema));

    std::fs::write(&types, STRING_TYPES).expect("write types");
    assert_eq!(imported_type(import(dir.path(), "types.yaml")), json!("string"));
    std::fs::remove_file(&types).expect("delete types");
    let warm = import(dir.path(), "types.yaml");
    assert!(matches!(warm, Err(SchemaError::NoMatch { .. })), "warm: {:?}", warm.map(|r| r.json_schema));

    std::fs::write(&types, NUMBER_TYPES).expect("recreate types");
    assert_eq!(imported_type(import(dir.path(), "types.yaml")), json!("number"));
}

/// A schema resolved before an edit keeps the revision it was built from.
#[test]
fn a_schema_resolved_before_an_edit_keeps_its_revision() {
    let dir = tempfile::tempdir().expect("tempdir");
    let types = dir.path().join("types.yaml");
    std::fs::write(&types, STRING_TYPES).expect("write types");
    let before = import(dir.path(), "types.yaml").expect("resolves");

    std::fs::write(&types, NUMBER_TYPES).expect("edit types");
    let after = import(dir.path(), "types.yaml");

    assert_eq!(before.json_schema["properties"]["value"]["anyOf"][1]["type"], json!("string"));
    assert_eq!(imported_type(after), json!("number"));
}
