//! How Claudine's triggers identify its documents. Every trigger of
//! `claudine/registration/` identifies the host (there is no separate
//! identifying file), and the two that type a Claudine Markdown document
//! (`composition.yaml` and `inline-composition.yaml`) select it exactly one
//! at a time, so each receives the main host contract.
//!
//! `initialize`, `cancelled`, and `finalize` are Darkmatter events the `md`
//! host owns too, so no file selects a document on them alone. The matching
//! triggers decide which host a document belongs to; the selected trigger
//! still types those three events.
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

/// Claudine's triggers, each admitted as identifying.
fn claudine_triggers() -> Vec<IdentifyingTrigger> {
    claudine::composition::inventory::members_of(claudine::composition::inventory::MemberKind::Trigger)
        .map(|member| identifying(member.text))
        .collect()
}

/// Whether any Claudine trigger identifies `document`.
fn identifies_claudine(triggers: &[IdentifyingTrigger], document: &Value, subject: &PathSubject) -> bool {
    triggers.iter().any(|trigger| identifies(trigger, document, subject))
}

fn one(key: &str, value: Value) -> Value {
    json!({ key: value })
}

#[test]
fn every_claudine_trigger_is_admitted_as_identifying_and_no_identify_file_remains() {
    let triggers: Vec<&str> = claudine::composition::inventory::members_of(claudine::composition::inventory::MemberKind::Trigger)
        .map(|member| member.path)
        .collect();
    assert_eq!(triggers, ["composition.yaml", "inline-composition.yaml", "sequence-document.yaml"]);
    for member in claudine::composition::inventory::members_of(claudine::composition::inventory::MemberKind::Trigger) {
        assert!(admit_identifying_trigger(&envelope(member.text).conditions, md_owned_names()).is_ok(), "{}", member.path);
    }
    assert!(claudine::composition::inventory::member("claudine-document.yaml").is_none(), "no identify-only file");
    // The cross product of the identifying keys is what any Claudine key
    // identifies; the helper is kept for that description.
    let _ = cross_product(&IDENTIFYING_KEYS);

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
    let triggers = claudine_triggers();
    for key in IDENTIFYING_KEYS {
        for value in non_null_values() {
            let document = one(key, value.clone());
            assert!(identifies_claudine(&triggers, &document, &PathSubject::detached()), "{document}");
        }
    }
    // A wrongly typed value still identifies Claudine, whose schema reports it.
    for document in [json!({ "agent": 7 }), json!({ "sequence": true }), json!({ "start": "stop" })] {
        assert!(identifies_claudine(&triggers, &document, &PathSubject::detached()), "{document}");
    }
    let mixed = json!({ "finalize": [{ "info": "done" }], "title": "x", "prompt": "hi" });
    assert!(identifies_claudine(&triggers, &mixed, &PathSubject::detached()), "a shared event beside a Claudine key");
}

#[test]
fn shared_events_md_keys_nulls_and_an_empty_document_do_not_identify_claudine() {
    let triggers = claudine_triggers();
    let subject = PathSubject::detached();
    for event in SHARED_EVENTS {
        for value in non_null_values() {
            let document = one(event, value);
            assert!(!identifies_claudine(&triggers, &document, &subject), "{document}");
        }
    }
    let together = json!({
        "initialize": [{ "info": "hi" }],
        "cancelled": [{ "stderr": "cancelled" }],
        "finalize": [{ "info": "done" }],
    });
    assert!(!identifies_claudine(&triggers, &together, &subject), "{together}");

    let every_md_key: serde_json::Map<String, Value> =
        md_owned_names().iter().map(|name| (name.clone(), json!("x"))).collect();
    assert!(!identifies_claudine(&triggers, &Value::Object(every_md_key), &subject), "every md key at once");

    let nulls: serde_json::Map<String, Value> =
        IDENTIFYING_KEYS.iter().map(|key| ((*key).to_string(), Value::Null)).collect();
    assert!(!identifies_claudine(&triggers, &Value::Object(nulls), &subject), "the nine keys, each null");
    assert!(!identifies_claudine(&triggers, &json!({}), &subject), "an empty document");
}

#[test]
fn exactly_one_claudine_trigger_selects_each_identified_document_and_none_selects_shared_events_alone() {
    let composition = envelope(include_str!("../../../registration/composition.yaml")).conditions;
    let inline = envelope(include_str!("../../../registration/inline-composition.yaml")).conditions;
    // A document identified as Claudine's but selected by neither trigger
    // would be judged without the main contract (`sequence: []` beside an
    // invalid `timeout` once did); one selected by both would layer two
    // contracts for one document.
    let subject = PathSubject::detached();
    let selected_by = |document: &Value| (composition.matches(document, &subject), inline.matches(document, &subject));
    for key in &IDENTIFYING_KEYS {
        for value in non_null_values() {
            let document = one(key, value);
            let expected = if *key == "prompt" { (false, true) } else { (true, false) };
            assert_eq!(selected_by(&document), expected, "{document}");
        }
        assert_eq!(selected_by(&one(key, Value::Null)), (false, false), "`{key}: null` selects nothing");
    }
    for event in SHARED_EVENTS {
        for value in non_null_values() {
            let document = one(event, value);
            assert_eq!(selected_by(&document), (false, false), "{document} alone selects no Claudine trigger");
        }
    }
    let together = json!({ "initialize": [], "cancelled": [], "finalize": [] });
    assert_eq!(selected_by(&together), (false, false), "the three shared events together");
    let with_prompt = json!({ "initialize": [], "cancelled": [], "finalize": [], "prompt": "hi" });
    assert_eq!(selected_by(&with_prompt), (false, true), "a Claudine key beside them");
    let sequence_with_prompt = json!({ "sequence": [], "prompt": "hi" });
    assert_eq!(selected_by(&sequence_with_prompt), (true, false), "a sequence is never an inline composition");
}

/// A document a Claudine trigger selects still gets its types for the shared
/// events; without a Claudine key, no Claudine trigger judges them.
#[test]
fn a_selected_document_keeps_claudines_types_for_the_shared_events() {
    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    let claudine = include_str!("../../../registration/composition.yaml");
    let inline = include_str!("../../../registration/inline-composition.yaml");
    for (name, selector, event, stack, pointer) in [
        ("finalize-typo", "prompt: hi", "finalize", "[{ inf0: hi }]", "/finalize/0"),
        ("initialize-shell", "start: [stop]", "initialize", "[{ shell: ls }]", "/initialize/0"),
        ("cancelled-retry", "model: opus", "cancelled", "[retry]", "/cancelled/0"),
    ] {
        let selected = repo.write_document(&format!("{name}.md"), &format!("---\n{selector}\n{event}: {stack}\n---\n"));
        let judged = discovered.judge(&selected, claudine, inline).expect("judged");
        assert!(judged.claudine || judged.inline, "{name}: `{selector}` selects a Claudine trigger");
        assert!(judged.properties.contains(event), "{name}: Claudine types `{event}`");
        let found = problems(&judged.report);
        assert!(found.iter().any(|(path, _)| path == pointer), "{name}: rejected at `{pointer}`: {found:?}");

        let alone = repo.write_document(&format!("{name}-alone.md"), &format!("---\n{event}: {stack}\n---\n"));
        let judged = discovered.judge(&alone, claudine, inline).expect("judged");
        assert!(!judged.claudine && !judged.inline, "{name}: `{event}` alone selects nothing");
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
/// composition.yaml, so a Claudine control in it is ordinary data; a `start:`
/// selects composition.yaml and the same control is checked. Claudine's
/// lifecycle parser still judges the events of both documents.
#[test]
fn the_launch_verdict_applies_claudine_yaml_only_beside_a_claudine_key() {
    let launch = Launch::new();
    let shared = launch.document("shared.md", "finalize: [stop]\ntimeout: [1]");
    launch.verdict(&shared).expect("composition.yaml does not type `timeout` here");

    let claudine = launch.document("claudine.md", "start: [stop]\nfinalize: [stop]\ntimeout: [1]");
    let error = launch.verdict(&claudine).expect_err("composition.yaml types `timeout` as a string");
    assert!(error.to_string().contains("/timeout"), "{error}");

    let shell = launch.document("shell.md", "initialize: [{ shell: ls }]");
    launch.verdict(&shell).expect("no schema rejects the shell here");
    let markdown = darkmatter::markdown::Markdown::try_from(shell.as_path()).expect("document loads");
    let refused = parse_lifecycle_config(&frontmatter_json(&markdown), &shell).expect_err("the parser refuses it");
    assert!(refused.to_string().contains("not valid in the `initialize` event"), "{refused}");
}
