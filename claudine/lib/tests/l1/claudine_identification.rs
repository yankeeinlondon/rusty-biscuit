//! Claudine's identifying trigger (`claudine/schemas/claudine-document.yaml`)
//! and the narrowed root trigger (`claudine/schemas/claudine.yaml`).
//!
//! `initialize`, `cancelled`, and `finalize` are Darkmatter events the `md`
//! host owns too, so neither file selects a document on them alone. The
//! identifying trigger decides which host a document belongs to; the root
//! trigger still types those three events for every document it selects.
//! Each test reads the repository files it judges with `include_str!`.

use std::path::{Path, PathBuf};

use claudine::composition::{
    CompositionError, PreValidatedSchema, TriggerFindings, parse_lifecycle_config, pre_validate_schema,
    resolve_composition_source_in_context,
};
use darkmatter::config::registration::md_owned_names;
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
use serde_json::{Value, json};
use simplified_schema::triggers::{
    IdentifyingTrigger, MatchConditions, PathSubject, TriggerEnvelope, admit_identifying_trigger, identifies,
    parse_trigger_envelope_from_str,
};

use crate::claudine_schemas::{SchemaRepo, frontmatter_json, problems, shipped_bundle};

/// The keys that identify a Claudine document.
const IDENTIFYING_KEYS: [&str; 9] =
    ["sequence", "start", "blocked", "success", "failure", "loop", "prompt", "agent", "model"];

/// The events Claudine shares with Darkmatter's `md` host.
const SHARED_EVENTS: [&str; 3] = ["initialize", "cancelled", "finalize"];

/// The type atom of each non-null YAML type, as the triggers spell them.
const NON_NULL_ATOMS: [&str; 5] = ["string(required)", "number(required)", "boolean(required)", "object(required)", "\"unknown[](required)\""];

/// One value of each non-null YAML type.
fn non_null_values() -> [Value; 5] {
    [json!("text"), json!(7), json!(true), json!({ "a": 1 }), json!([1])]
}

fn envelope(text: &str) -> TriggerEnvelope {
    parse_trigger_envelope_from_str(text).expect("the trigger parses").expect("a trigger envelope")
}

/// A trigger whose `match:` is one `group:` of `keys` × [`NON_NULL_ATOMS`].
fn cross_product(keys: &[&str]) -> MatchConditions {
    let mut text = String::from("kind: schema-trigger\nmatch:\n    group:\n");
    for key in keys {
        for atom in NON_NULL_ATOMS {
            text.push_str(&format!("        - {key}: {atom}\n"));
        }
    }
    envelope(&text).conditions
}

fn identifying(text: &str) -> IdentifyingTrigger {
    admit_identifying_trigger(&envelope(text).conditions, md_owned_names()).expect("admitted against md's names")
}

fn one(key: &str, value: Value) -> Value {
    json!({ key: value })
}

#[test]
fn the_identifying_trigger_is_admitted_and_is_the_key_by_type_cross_product() {
    let text = include_str!("../../../schemas/claudine-document.yaml");
    let parsed = envelope(text);
    assert!(admit_identifying_trigger(&parsed.conditions, md_owned_names()).is_ok());
    assert_eq!(parsed.conditions, cross_product(&IDENTIFYING_KEYS), "one alternative per key and non-null type");
    let payload = parsed.payload.as_ref().and_then(|payload| payload.as_mapping());
    assert!(payload.is_some_and(|payload| payload.is_empty()), "identification adds no types: {payload:?}");

    // Claudine's runtime never applies or runs anything through it.
    let embedded = claudine::composition::schema::bundle::CLAUDINE_SCHEMA_FILES;
    assert!(embedded.iter().all(|(name, _)| *name != "claudine-document.yaml"), "not in the embedded bundle");

    let owned = md_owned_names();
    for key in IDENTIFYING_KEYS {
        assert!(!owned.contains(key), "`{key}` is not an md key");
    }
    for event in SHARED_EVENTS {
        assert!(owned.contains(event), "`{event}` is md's too, so it cannot identify Claudine");
    }
}

#[test]
fn every_identifying_key_at_any_non_null_value_identifies_claudine() {
    let trigger = identifying(include_str!("../../../schemas/claudine-document.yaml"));
    for key in IDENTIFYING_KEYS {
        for value in non_null_values() {
            let document = one(key, value.clone());
            assert!(identifies(&trigger, &document, &PathSubject::detached()), "{document}");
        }
    }
    // A wrongly typed value still identifies Claudine, whose schema reports it.
    for document in [json!({ "agent": 7 }), json!({ "sequence": true }), json!({ "start": "stop" })] {
        assert!(identifies(&trigger, &document, &PathSubject::detached()), "{document}");
    }
    let mixed = json!({ "finalize": [{ "info": "done" }], "title": "x", "prompt": "hi" });
    assert!(identifies(&trigger, &mixed, &PathSubject::detached()), "a shared event beside a Claudine key");
}

#[test]
fn shared_events_md_keys_nulls_and_an_empty_document_do_not_identify_claudine() {
    let trigger = identifying(include_str!("../../../schemas/claudine-document.yaml"));
    let subject = PathSubject::detached();
    for event in SHARED_EVENTS {
        for value in non_null_values() {
            let document = one(event, value);
            assert!(!identifies(&trigger, &document, &subject), "{document}");
        }
    }
    let together = json!({
        "initialize": [{ "info": "hi" }],
        "cancelled": [{ "stderr": "cancelled" }],
        "finalize": [{ "info": "done" }],
    });
    assert!(!identifies(&trigger, &together, &subject), "{together}");

    let every_md_key: serde_json::Map<String, Value> =
        md_owned_names().iter().map(|name| (name.clone(), json!("x"))).collect();
    assert!(!identifies(&trigger, &Value::Object(every_md_key), &subject), "every md key at once");

    let nulls: serde_json::Map<String, Value> =
        IDENTIFYING_KEYS.iter().map(|key| ((*key).to_string(), Value::Null)).collect();
    assert!(!identifies(&trigger, &Value::Object(nulls), &subject), "the nine keys, each null");
    assert!(!identifies(&trigger, &json!({}), &subject), "an empty document");
}

#[test]
fn claudine_yaml_selects_on_its_own_keys_and_never_on_shared_events_alone() {
    let claudine = envelope(include_str!("../../../schemas/claudine.yaml")).conditions;
    let selecting: Vec<&str> = IDENTIFYING_KEYS.iter().copied().filter(|key| *key != "sequence").collect();
    // `sequence` is selected by `sequence.yaml`, so claudine.yaml's selectors
    // are the identifying keys without it: every document it selects is a
    // Claudine document.
    assert_eq!(claudine, cross_product(&selecting), "claudine.yaml's selectors");

    let subject = PathSubject::detached();
    for key in &selecting {
        for value in non_null_values() {
            let document = one(key, value);
            assert!(claudine.matches(&document, &subject), "{document} selects claudine.yaml");
        }
        assert!(!claudine.matches(&one(key, Value::Null), &subject), "`{key}: null` selects nothing");
    }
    for event in SHARED_EVENTS {
        for value in non_null_values() {
            let document = one(event, value);
            assert!(!claudine.matches(&document, &subject), "{document} alone does not select claudine.yaml");
        }
    }
    let together = json!({ "initialize": [], "cancelled": [], "finalize": [] });
    assert!(!claudine.matches(&together, &subject), "the three shared events together");
    let with_prompt = json!({ "initialize": [], "cancelled": [], "finalize": [], "prompt": "hi" });
    assert!(claudine.matches(&with_prompt, &subject), "a Claudine key beside them");
}

/// A document claudine.yaml selects still gets its types for the shared
/// events; without a Claudine key, claudine.yaml does not judge them.
#[test]
fn a_selected_document_keeps_claudines_types_for_the_shared_events() {
    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    let claudine = include_str!("../../../schemas/claudine.yaml");
    let inline = include_str!("../../../schemas/inline-compose.yaml");
    for (name, selector, event, stack, pointer) in [
        ("finalize-typo", "prompt: hi", "finalize", "[{ inf0: hi }]", "/finalize/0"),
        ("initialize-shell", "start: [stop]", "initialize", "[{ shell: ls }]", "/initialize/0"),
        ("cancelled-retry", "model: opus", "cancelled", "[retry]", "/cancelled/0"),
    ] {
        let selected = repo.write_document(&format!("{name}.md"), &format!("---\n{selector}\n{event}: {stack}\n---\n"));
        let judged = discovered.judge(&selected, claudine, inline).expect("judged");
        assert!(judged.claudine, "{name}: `{selector}` selects claudine.yaml");
        assert!(judged.properties.contains(event), "{name}: Claudine types `{event}`");
        let found = problems(&judged.report);
        assert!(found.iter().any(|(path, _)| path == pointer), "{name}: rejected at `{pointer}`: {found:?}");

        let alone = repo.write_document(&format!("{name}-alone.md"), &format!("---\n{event}: {stack}\n---\n"));
        let judged = discovered.judge(&alone, claudine, inline).expect("judged");
        assert!(!judged.claudine, "{name}: `{event}` alone selects nothing");
        assert!(!judged.properties.contains("prompt"), "{name}: no Claudine payload applies");
    }
}

/// A temporary launch folder with a fixture home: Claudine's launch verdict
/// discovers its embedded bundle there, with no disk schema root.
struct Launch {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Launch {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(root.join("docs")).expect("docs dir");
        std::fs::create_dir_all(root.join("home")).expect("home dir");
        Self { _dir: dir, root }
    }

    fn document(&self, name: &str, frontmatter: &str) -> PathBuf {
        let path = self.root.join("docs").join(name);
        std::fs::write(&path, format!("---\n{frontmatter}\n---\nbody\n")).expect("write document");
        path
    }

    #[allow(clippy::result_large_err, reason = "the public verdict's own result, returned unchanged")]
    fn verdict(&self, document: &Path) -> Result<PreValidatedSchema, CompositionError> {
        let snapshot = RequestSnapshot::new(self.root.join("docs")).with_home(Some(self.root.join("home")));
        let context = build_resolution_context(&snapshot).expect("fixture context");
        let source = resolve_composition_source_in_context(document.to_str().expect("UTF-8 path"), &context)
            .expect("the document resolves");
        pre_validate_schema(&source, None, None, &context, &TriggerFindings::default())
    }
}

/// Claudine's launch verdict judges a shared-events-only document without
/// claudine.yaml, so a Claudine control in it is ordinary data; a `start:`
/// selects claudine.yaml and the same control is checked. Claudine's
/// lifecycle parser still judges the events of both documents.
#[test]
fn the_launch_verdict_applies_claudine_yaml_only_beside_a_claudine_key() {
    let launch = Launch::new();
    let shared = launch.document("shared.md", "finalize: [stop]\ntimeout: [1]");
    launch.verdict(&shared).expect("claudine.yaml does not type `timeout` here");

    let claudine = launch.document("claudine.md", "start: [stop]\nfinalize: [stop]\ntimeout: [1]");
    let error = launch.verdict(&claudine).expect_err("claudine.yaml types `timeout` as a string");
    assert!(error.to_string().contains("/timeout"), "{error}");

    let shell = launch.document("shell.md", "initialize: [{ shell: ls }]");
    launch.verdict(&shell).expect("no schema rejects the shell here");
    let markdown = darkmatter::markdown::Markdown::try_from(shell.as_path()).expect("document loads");
    let refused = parse_lifecycle_config(&frontmatter_json(&markdown), &shell).expect_err("the parser refuses it");
    assert!(refused.to_string().contains("not valid in the `initialize` event"), "{refused}");
}
