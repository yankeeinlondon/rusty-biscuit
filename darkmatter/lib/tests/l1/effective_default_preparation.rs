//! Default preparation through Darkmatter's `EffectiveSchema`: the library's
//! `DefaultPlan` runs on every value, valid as written or not, so an optional
//! default and a `null` value are filled exactly as `simplified_schema::Prepared`
//! fills them. `data_copy` converts but never shows a default as if it were
//! written (`md set` writes from it).
//!
//! Boundary: the crate's public API (`DarkmatterSchemas::effective_for`,
//! `EffectiveSchema::{validate, validate_raw, data_copy}`), in-process, over a
//! document in a temp dir (F2).

use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema};
use serde_json::json;
use simplified_schema::Prepared;

/// The effective schema of a document at `dir/doc.md` whose frontmatter
/// begins with `frontmatter` (a `$schema` entry).
fn effective(dir: &Path, frontmatter: &str) -> EffectiveSchema {
    let source: Markdown = format!("---\n{frontmatter}---\nbody\n").into();
    let source = source.with_source(ComposeSource::File(dir.join("doc.md")));
    DarkmatterSchemas::new(crate::request_support::context_at(dir))
        .effective_for(&source)
        .expect("schema resolves")
        .expect("schema is present")
}

fn paths(defaults: &[simplified_schema::AppliedDefault]) -> Vec<String> {
    defaults.iter().map(|applied| applied.path.to_string()).collect()
}

const CFG: &str = "$schema:\n  cfg: '{ on: boolean(default(true)), n: number }'\n  mode: string(default(auto))\n";

#[test]
fn a_value_valid_as_written_still_has_its_defaults_prepared() {
    let workspace = tempfile::tempdir().expect("workspace");
    let schema = effective(workspace.path(), CFG);
    let input = json!({ "cfg": {}, "mode": null });

    for report in [schema.validate(&input), schema.validate_raw(&input)] {
        assert!(report.valid, "{:?}", report.problems);
        assert_eq!(paths(&report.defaults), ["cfg.on", "mode"]);
    }
}

#[test]
fn the_host_and_the_library_fill_the_same_defaults() {
    let workspace = tempfile::tempdir().expect("workspace");
    let schema = effective(workspace.path(), CFG);
    let prepared = Prepared::new(
        Arc::clone(&schema.json_schema),
        Some(workspace.path().to_path_buf()),
        biscuit_file::FileResolutionContext::new(workspace.path()),
    )
    .expect("compiles");

    for input in [json!({}), json!({ "cfg": { "n": "4" } }), json!({ "cfg": { "on": null }, "mode": "manual" })] {
        let host = schema.validate(&input);
        let library = prepared.validate_soft(&input).expect("library accepts");
        assert!(host.valid, "{input}: {:?}", host.problems);
        // Only the host knows where a default is declared.
        let without_origin = |defaults: Vec<simplified_schema::AppliedDefault>| -> Vec<_> {
            defaults.into_iter().map(|applied| simplified_schema::AppliedDefault { origin: None, ..applied }).collect()
        };
        assert_eq!(without_origin(host.defaults), library.defaults, "{input}");
    }
}

#[test]
fn data_copy_converts_without_showing_a_default_as_written() {
    let workspace = tempfile::tempdir().expect("workspace");
    let schema = effective(workspace.path(), CFG);

    let converted = json!({ "cfg": { "n": "4" } });
    assert_eq!(schema.data_copy(&converted).as_ref(), &json!({ "cfg": { "n": 4 } }), "cfg.on and mode stay unwritten");

    let written_null = json!({ "cfg": { "on": null }, "mode": null });
    assert!(
        matches!(schema.data_copy(&written_null), Cow::Borrowed(value) if *value == written_null),
        "a replaced null is put back, so nothing changed"
    );
}

#[test]
fn conflicting_raw_defaults_are_reported_at_the_schema_location() {
    let workspace = tempfile::tempdir().expect("workspace");
    let raw = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "anyOf": [
            { "type": "object", "properties": { "n": { "type": "number", "default": 1 } } },
            { "type": "object", "properties": { "n": { "type": "number", "default": 2 } } }
        ]
    });
    std::fs::write(workspace.path().join("schema.json"), raw.to_string()).expect("writes the schema");
    let schema = effective(workspace.path(), "$schema: ./schema.json\n");

    let report = schema.validate(&json!({ "n": 4 }));

    assert!(!report.valid, "a value the arms accept as written still fails");
    let problem = &report.problems[0];
    assert!(problem.message.contains("declares different defaults"), "{}", problem.message);
    assert_eq!(problem.schema_path.as_ref().map(|pointer| pointer.as_pointer_string()).as_deref(), Some("/anyOf/0"));
}
