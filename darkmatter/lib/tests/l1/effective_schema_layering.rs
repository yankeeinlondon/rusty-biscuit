//! Darkmatter's effective schema is the library's generic layering with
//! Darkmatter's host policy around it: the document's `$schema` is the
//! receiver and keeps every property it declares, and the baseline and the
//! matched triggers are layered under it, low to high, exactly as
//! [`SimplifiedSchema::layer_under`] does. Each case builds the same layers
//! through the library's schema object and compares the merged JSON Schema
//! and the verdicts.
//!
//! The cases also pin what stays host policy: an absent document schema is
//! not an error (the baseline alone applies, or nothing does), and the
//! baseline, a lone trigger payload, and the compiled validators are shared,
//! never reparsed or cloned per page.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::{DarkmatterSchemas, darkmatter_base_json_schema_ref};
use serde_json::{Value, json};
use simplified_schema::{SimplifiedSchema, SimplifiedSchemaError, parse_yaml_schema};

/// The custom baseline: `title` and `owner`.
const BASELINE: &str = "$schema:\n  title: string\n  owner: string\n";

/// The trigger's payload: replaces the baseline's `title`, adds `level`.
const TRIGGER_PAYLOAD: &str = "$schema:\n  title: number\n  level: number\n";

/// The trigger file: matches a document with a `track` key.
const TRIGGER: &str = "kind: schema-trigger\nmatch:\n  track: string(required)\n$schema:\n  title: number\n  level: number\n";

/// The document schema: replaces `title` again, adds `extra`.
const DOCUMENT_SCHEMA: &str = "$schema:\n  title: boolean\n  extra: string\n";

/// `{dir}/work` holds the documents and the `schemas/` trigger root; `{dir}/home`
/// is an empty fixture `HOME`.
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    context: FileResolutionContext,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(root.join("work/schemas")).expect("schemas dir");
        std::fs::create_dir_all(root.join("home")).expect("home dir");
        let snapshot = RequestSnapshot::new(root.join("work"))
            .with_home(Some(root.join("home")))
            .with_env(HashMap::new());
        let context = build_resolution_context(&snapshot).expect("fixture context");
        Self { _dir: dir, root, context }
    }

    /// Writes the trigger into the `schemas/` root and discovers it.
    fn with_trigger(&self, api: DarkmatterSchemas) -> DarkmatterSchemas {
        std::fs::write(self.root.join("work/schemas/track.yaml"), TRIGGER).expect("write trigger");
        let api = api.with_trigger_discovery().expect("trigger discovery");
        assert!(api.discovery_findings().is_empty(), "{:?}", api.discovery_findings());
        api
    }

    fn api(&self) -> DarkmatterSchemas {
        DarkmatterSchemas::new(self.context.clone())
    }
}

fn custom_baseline(api: DarkmatterSchemas) -> DarkmatterSchemas {
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(BASELINE).expect("baseline yaml");
    let schema = parse_yaml_schema(&yaml["$schema"]).expect("baseline schema");
    api.with_baseline(schema).expect("custom baseline")
}

/// An in-memory document whose frontmatter is `frontmatter`.
fn document(frontmatter: &str) -> Markdown {
    Markdown::from(format!("---\n{frontmatter}---\nbody\n").as_str())
}

/// A document with the shared document schema and `rest`.
fn authored_document(rest: &str) -> Markdown {
    document(&format!("{DOCUMENT_SCHEMA}{rest}"))
}

fn schema(content: &str) -> SimplifiedSchema {
    SimplifiedSchema::try_from(content).expect("schema content")
}

fn merged_json(api: &DarkmatterSchemas, source: &Markdown) -> Arc<Value> {
    Arc::clone(&api.effective_for(source).expect("effective schema").expect("a schema applies").json_schema)
}

fn layered_under(receiver: &str, incoming: &[&str]) -> Value {
    let incoming: Vec<SimplifiedSchema> = incoming.iter().map(|content| schema(content)).collect();
    let refs: Vec<&SimplifiedSchema> = incoming.iter().collect();
    let mut receiver = schema(receiver);
    receiver.layer_under(&refs).expect("layer under");
    receiver.to_json_schema().expect("lowered").as_ref().clone()
}

#[test]
fn the_document_schema_is_layered_over_the_baseline_and_a_matched_trigger() {
    let fixture = Fixture::new();
    let api = fixture.with_trigger(custom_baseline(fixture.api()));
    let source = authored_document("track: a\n");

    let expected = layered_under(DOCUMENT_SCHEMA, &[BASELINE, TRIGGER_PAYLOAD]);
    assert_eq!(*merged_json(&api, &source), expected);
    let properties = expected["properties"].as_object().expect("properties");
    let optional_boolean = json!({ "anyOf": [{ "type": "null" }, { "type": "boolean" }] });
    assert_eq!(properties["title"], optional_boolean, "the document wins");
    for name in ["owner", "level", "extra"] {
        assert!(properties.contains_key(name), "{name} comes from one layer");
    }

    assert!(api.validate(&authored_document("track: a\ntitle: true\nlevel: 2\n")).expect("report").valid);
    for rejected in ["title: 1\n", "level: high\n", "owner: [a]\n"] {
        let report = api.validate(&authored_document(&format!("track: a\n{rejected}"))).expect("report");
        assert!(!report.valid, "{rejected}");
    }
}

#[test]
fn a_trigger_wins_over_the_baseline_when_the_document_has_no_schema() {
    let fixture = Fixture::new();
    let api = fixture.with_trigger(custom_baseline(fixture.api()));
    let source = document("track: a\n");

    // With no receiver, the highest incoming layer plays its part.
    let expected = layered_under(TRIGGER_PAYLOAD, &[BASELINE]);
    assert_eq!(*merged_json(&api, &source), expected);
    assert!(api.validate(&document("track: a\ntitle: 3\nowner: o\n")).expect("report").valid);
    assert!(!api.validate(&document("track: a\ntitle: words\n")).expect("report").valid);
}

#[test]
fn an_absent_document_schema_keeps_the_shared_baseline_and_no_layers_is_no_schema() {
    let fixture = Fixture::new();

    // An enabled baseline applies to a page with no `$schema`, shared rather
    // than rebuilt per page.
    let api = custom_baseline(fixture.api());
    let first = merged_json(&api, &document("owner: a\n"));
    let second = merged_json(&api, &document("title: other page\n"));
    assert!(Arc::ptr_eq(&first, &second), "the baseline is shared across pages");
    assert_eq!(*first, *schema(BASELINE).to_json_schema().expect("lowered"));

    // Disabled baseline, no trigger, no `$schema`: no schema applies, which
    // is not an error, while the library's constructor reports the absence.
    let bare = fixture.api();
    let source = document("owner: a\n");
    assert!(bare.effective_for(&source).expect("effective schema").is_none());
    let (report, summary) = bare.validate_summarized(&source).expect("report");
    assert!(report.valid && report.problems.is_empty());
    assert!(!summary.has_schema);
    assert!(matches!(
        SimplifiedSchema::try_from("---\nowner: a\n---\nbody\n"),
        Err(SimplifiedSchemaError::MissingSchema { .. })
    ));
}

#[test]
fn a_lone_matched_trigger_is_shared_as_resolved() {
    let fixture = Fixture::new();
    let api = fixture.with_trigger(fixture.api());

    let first = merged_json(&api, &document("track: a\n"));
    let second = merged_json(&api, &document("track: b\n"));
    assert!(Arc::ptr_eq(&first, &second), "one payload serves every matching page");
    assert_eq!(*first, *schema(TRIGGER_PAYLOAD).to_json_schema().expect("lowered"));
    // A page the trigger does not match has no schema.
    assert!(api.effective_for(&document("other: a\n")).expect("effective schema").is_none());
}

#[test]
fn the_shipped_baseline_layers_like_the_library_and_is_never_cloned_per_page() {
    let fixture = Fixture::new();
    let api = fixture.api().with_darkmatter_baseline_json_schema().expect("shipped baseline");

    let bare = merged_json(&api, &document("title: page\n"));
    assert!(std::ptr::eq(bare.as_ref(), darkmatter_base_json_schema_ref()), "the process baseline itself");

    // The shipped baseline is raw JSON Schema to the library. It goes in as
    // YAML text: the content loader reads JSON through the YAML parser, which
    // caps a key at 1024 characters, and one of its pattern keys is longer.
    let baseline = serde_yaml_ng::to_string(darkmatter_base_json_schema_ref()).expect("baseline text");
    let expected = layered_under(DOCUMENT_SCHEMA, &[&baseline]);
    assert_eq!(*merged_json(&api, &authored_document("")), expected);
    assert!(!api.validate(&authored_document("title: words\n")).expect("report").valid);
    assert!(api.validate(&authored_document("title: true\n")).expect("report").valid);
}

#[test]
fn compiled_validators_are_reused_across_pages_with_the_same_effective_schema() {
    let fixture = Fixture::new();
    let api = fixture.with_trigger(custom_baseline(fixture.api()));

    let first = api.effective_for(&authored_document("track: a\n")).expect("effective").expect("schema");
    let second = api.effective_for(&authored_document("track: b\nextra: x\n")).expect("effective").expect("schema");
    assert!(Arc::ptr_eq(&first.json_schema, &second.json_schema), "the merge is memoized");
    let (Some(first), Some(second)) = (first.validator(), second.validator()) else {
        panic!("an object schema has one validator");
    };
    assert!(std::ptr::eq(first, second), "one compiled validator serves both pages");
}
