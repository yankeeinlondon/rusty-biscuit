//! The `md` host's registry and Darkmatter's runtime baseline agree: the
//! baseline types exactly the registry's authored event names as lifecycle
//! stacks, and each event admits exactly the action types the registry
//! declares for it. The md profile's open unknown-action arm excludes every
//! verb md provides and admits every verb it does not.
//!
//! Everything is judged through the public validation path (a document
//! validated against the shipped baseline, or against a `$schema` naming a
//! shipped type through the reserved `darkmatter:` namespace); the expected
//! names and action types come from the registry and Darkmatter's own verb
//! inventories, never from a list kept here.

use std::collections::BTreeSet;
use std::path::PathBuf;

use darkmatter::lifecycle::actions::all_lifecycle_verbs;
use darkmatter::lifecycle::md_host::{UNPROVIDED_VERBS, registry};
use darkmatter::lifecycle::registry::{ActionType, ActionTypes};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::{DarkmatterSchemas, ValidationReport, darkmatter_base_json_schema_ref};

use crate::request_support::context_at;

/// Every action type, each with one well-formed action of that type that
/// every host provides.
const SAMPLES: [(ActionType, &str); 4] = [
    (ActionType::Communicate, "{ info: hi }"),
    (ActionType::Flow, "{ stop: null }"),
    (ActionType::Shell, "{ shell: echo hi }"),
    (ActionType::Expression, "{ set: { answer: 42 } }"),
];

/// A stack whose one action is well formed for md.
const WELL_FORMED: &str = "[{ info: hi }]";
/// A stack whose one action is a known md verb in a malformed long form.
const MALFORMED: &str = "[{ info: { text: hi, bogus: 1 } }]";

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    baseline: DarkmatterSchemas,
    plain: DarkmatterSchemas,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        let baseline = DarkmatterSchemas::new(context_at(&root))
            .with_darkmatter_baseline_json_schema()
            .expect("the shipped baseline attaches");
        let plain = DarkmatterSchemas::new(context_at(&root));
        Self { _dir: dir, root, baseline, plain }
    }

    fn validate_with(&self, api: &DarkmatterSchemas, frontmatter: &str) -> ValidationReport {
        let path = self.root.join("doc.md");
        std::fs::write(&path, format!("---\n{frontmatter}\n---\nBody\n")).expect("write document");
        let markdown = Markdown::try_from(path.as_path())
            .expect("the document loads")
            .with_source(ComposeSource::File(path));
        api.validate(&markdown).expect("the schema prepares")
    }

    /// Validates `frontmatter` against the shipped baseline alone.
    fn validate(&self, frontmatter: &str) -> ValidationReport {
        self.validate_with(&self.baseline, frontmatter)
    }

    /// Whether the baseline types `key` as a lifecycle stack: it accepts a
    /// well-formed stack and rejects a malformed md action.
    fn types_as_stack(&self, key: &str) -> bool {
        self.validate(&format!("{key}: {WELL_FORMED}")).valid && !self.validate(&format!("{key}: {MALFORMED}")).valid
    }
}

fn problems(report: &ValidationReport) -> Vec<(String, String)> {
    report.problems.iter().map(|problem| (problem.path.clone(), problem.message.clone())).collect()
}

/// Every verb the lifecycle parser knows, `set` included.
fn known_verbs() -> BTreeSet<String> {
    all_lifecycle_verbs().into_iter().map(str::to_string).chain(["set".to_string()]).collect()
}

/// The verbs md provides: every known verb but the unprovided ones.
fn md_verbs() -> BTreeSet<String> {
    known_verbs().into_iter().filter(|verb| !UNPROVIDED_VERBS.contains(&verb.as_str())).collect()
}

#[test]
fn the_samples_cover_every_action_type_once() {
    let types: Vec<ActionType> = SAMPLES.iter().map(|(action, _)| *action).collect();
    assert_eq!(ActionTypes::of(types.iter().copied()), ActionTypes::all());
    assert_eq!(types.iter().collect::<BTreeSet<_>>().len(), types.len(), "each type once");
}

#[test]
fn the_baseline_types_exactly_the_md_registry_event_names() {
    let fixture = Fixture::new();
    let registry = registry();
    let authored: BTreeSet<String> = registry.authored_event_keys().into_iter().map(str::to_string).collect();
    assert!(!authored.is_empty());

    let properties = darkmatter_base_json_schema_ref()["properties"].as_object().expect("baseline properties");
    // `$schema` declares the document's own schema rather than holding data.
    let typed: BTreeSet<String> = properties
        .keys()
        .filter(|key| *key != "$schema" && fixture.types_as_stack(key))
        .cloned()
        .collect();
    assert_eq!(typed, authored, "the baseline's stack-typed properties are the md registry's event names");

    // A name another host gives an event is not typed: `start` is Claudine's
    // `composed`, and md does not know it.
    assert!(registry.id("start").is_none());
    assert!(!properties.contains_key("start"));
    let start = fixture.validate(&format!("start: {MALFORMED}"));
    assert!(start.valid, "`start` is untyped under the baseline: {:?}", problems(&start));
}

#[test]
fn each_event_admits_exactly_the_registry_action_types() {
    let fixture = Fixture::new();
    let registry = registry();
    for event in registry.authored_event_keys() {
        let id = registry.id(event).expect("an authored key names an event");
        for (action, sample) in SAMPLES {
            let report = fixture.validate(&format!("{event}: [{sample}]"));
            assert_eq!(
                report.valid,
                registry.actions(id).contains(action),
                "`{event}` with a {action:?} action ({sample}): {:?}",
                problems(&report)
            );
        }
    }
}

#[test]
fn shell_never_escapes_initialize_through_a_bundle_or_the_open_arm() {
    let fixture = Fixture::new();
    let registry = registry();
    let initialize = registry
        .authored_event_keys()
        .into_iter()
        .find(|event| !registry.actions(registry.id(event).expect("event")).contains(ActionType::Shell))
        .expect("one md event excludes shells");
    for stack in [
        "{ info: hi, shell: echo hi }".to_string(),
        "[{ info: hi, shell: echo hi }]".to_string(),
        "[{ when: 'true', then: [{ shell: echo hi }] }]".to_string(),
        "[{ when: 'true', then: [{ when: 'true', else: [{ shell: echo hi }] }] }]".to_string(),
    ] {
        let report = fixture.validate(&format!("{initialize}: {stack}"));
        assert!(!report.valid, "`{initialize}: {stack}` is rejected");
    }
}

#[test]
fn verbs_md_does_not_provide_pass_untyped_and_malformed_md_verbs_do_not() {
    let fixture = Fixture::new();
    for verb in UNPROVIDED_VERBS {
        for stack in [format!("[{{ {verb}: {{ any: [shape] }} }}]"), format!("{{ info: hi, {verb}: [1, 2] }}")] {
            let report = fixture.validate(&format!("composed: {stack}"));
            assert!(report.valid, "`composed: {stack}` passes: {:?}", problems(&report));
        }
    }
    let report = fixture.validate("composed: [{ hosted_elsewhere: 1 }]");
    assert!(report.valid, "an unknown verb passes: {:?}", problems(&report));
    let report = fixture.validate(&format!("composed: {MALFORMED}"));
    assert!(!report.valid, "a malformed `info` long form is rejected");
    assert!(report.problems.iter().all(|problem| problem.path.starts_with("/composed/0")), "{:?}", problems(&report));
}

/// The open arm, judged on its own through a `$schema` that names it with the
/// reserved namespace.
#[test]
fn the_open_arm_rejects_every_md_verb_and_admits_every_unprovided_verb() {
    let fixture = Fixture::new();
    let provided = md_verbs();
    assert!(
        provided.len() > 120 && provided.contains("info") && provided.contains("shell") && provided.contains("edit"),
        "{provided:?}"
    );
    for verb in UNPROVIDED_VERBS {
        assert!(known_verbs().contains(*verb), "`{verb}` is a lifecycle verb");
    }
    let rejected: Vec<String> = provided.iter().map(|verb| format!("{{ {verb}: 1 }}")).collect();
    let admitted: Vec<String> = UNPROVIDED_VERBS.iter().map(|verb| format!("{{ {verb}: 1 }}")).collect();
    let report = fixture.validate_with(
        &fixture.plain,
        &format!(
            "$schema:\n    rejected: \"md-unknown-action[]@darkmatter:md-lifecycle\"\n    admitted: \"md-unknown-action[]@darkmatter:md-lifecycle\"\n    keys: \"md-unknown-action[]@darkmatter:md-lifecycle\"\nrejected: [{}]\nadmitted: [{}]\nkeys: [{{ when: 1 }}, {{ then: 1 }}, {{ else: 1 }}, {{ no_error: 1 }}]",
            rejected.join(", "),
            admitted.join(", ")
        ),
    );
    let failed_at = |property: &str| -> BTreeSet<usize> {
        report
            .problems
            .iter()
            .filter_map(|problem| problem.path.strip_prefix(&format!("/{property}/")))
            .map(|rest| rest.split('/').next().expect("an index").parse().expect("a numeric index"))
            .collect()
    };
    let rejected_indexes = failed_at("rejected");
    for (index, verb) in provided.iter().enumerate() {
        assert!(rejected_indexes.contains(&index), "the open arm admits md's `{verb}`");
    }
    assert!(failed_at("admitted").is_empty(), "the open arm rejects an unprovided verb: {:?}", problems(&report));
    assert_eq!(failed_at("keys"), BTreeSet::from([0, 1, 2, 3]), "conditional and bundle keys stay closed");
}

#[test]
fn a_root_dictionary_and_conditionals_keep_their_structure() {
    let fixture = Fixture::new();
    for (stack, valid) in [
        ("{ info: hi, when: 'true', no_error: true }", true),
        ("[{ when: 'true', then: [stop], else: [{ warn: no }] }]", true),
        ("[{ when: 'true', then: [stop], bogus: 1 }]", false),
        ("[{ then: [stop] }]", false),
        ("[stop, skip, hosted_elsewhere]", true),
        ("[{ info: hi, then: [stop] }]", false),
    ] {
        let report = fixture.validate(&format!("finalize: {stack}"));
        assert_eq!(report.valid, valid, "`finalize: {stack}`: {:?}", problems(&report));
    }
}

/// The md parser's verdict on `frontmatter`.
fn md_parses(frontmatter: &str) -> Result<(), String> {
    let value: serde_json::Value = serde_yaml_ng::from_str(frontmatter).expect("YAML frontmatter");
    darkmatter::lifecycle::md_host::parse_lifecycle(&value, std::path::Path::new("doc.md"), None)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// A single-action dictionary takes its verb's long form at a stack root and
/// in either branch, with `no_error` beside it there and `when` beside it in
/// a list item, as md's parser reads it; several verbs beside a long form,
/// and a shell in `initialize`, stay refused by both.
#[test]
fn single_action_dictionaries_take_long_forms_as_the_md_parser_does() {
    let fixture = Fixture::new();
    for body in ["{ info: { text: hi } }", "{ shell: { command: \"git status\" } }", "{ error: { reason: failed } }", "{ warn: { text: careful }, no_error: true }"] {
        for frontmatter in [
            format!("composed: {body}"),
            format!("failed: [{{ when: 'true', then: {body} }}]"),
            format!("finalize: [{{ when: 'true', then: [stop], else: {body} }}]"),
        ] {
            let report = fixture.validate(&frontmatter);
            assert!(report.valid, "the baseline accepts `{frontmatter}`: {:?}", problems(&report));
            assert_eq!(md_parses(&frontmatter), Ok(()), "md parses `{frontmatter}`");
        }
    }
    for frontmatter in ["initialize: { info: { text: hi } }", "composed: [{ when: 'true', info: { text: hi } }]"] {
        let report = fixture.validate(frontmatter);
        assert!(report.valid, "the baseline accepts `{frontmatter}`: {:?}", problems(&report));
        assert_eq!(md_parses(frontmatter), Ok(()), "md parses `{frontmatter}`");
    }
    for (frontmatter, needle) in [
        ("composed: { info: { text: hi }, warn: careful }", "long form"),
        ("composed: { info: { text: hi }, warn: { text: careful } }", "long form"),
        ("initialize: { shell: { command: ls } }", "shell"),
    ] {
        let report = fixture.validate(frontmatter);
        assert!(!report.valid, "the baseline rejects `{frontmatter}`");
        let error = md_parses(frontmatter).expect_err(frontmatter);
        assert!(error.contains(needle), "`{frontmatter}`: `{needle}` in {error}");
    }
}

/// Every `x-darkmatter-suggest` list under `fragment`.
fn suggestions(fragment: &serde_json::Value, found: &mut BTreeSet<Vec<String>>) {
    match fragment {
        serde_json::Value::Object(map) => {
            if let Some(list) = map.get("x-darkmatter-suggest").and_then(serde_json::Value::as_array) {
                found.insert(list.iter().filter_map(|value| value.as_str().map(str::to_string)).collect());
            }
            map.values().for_each(|value| suggestions(value, found));
        }
        serde_json::Value::Array(items) => items.iter().for_each(|value| suggestions(value, found)),
        _ => {}
    }
}

/// The bare-verb arm suggests exactly the md verbs md's own parser accepts
/// as a bare list item, so the list cannot drift from the parser.
#[test]
fn the_bare_verb_suggestions_are_the_md_parsers_bare_verbs() {
    let parsed: BTreeSet<String> = md_verbs()
        .into_iter()
        .filter(|verb| {
            darkmatter::lifecycle::parse_lifecycle_config_with(
                &serde_json::json!({ "initialize": [verb] }),
                std::path::Path::new("bare.md"),
                &darkmatter::lifecycle::md_host::parse_options(None),
            )
            .is_ok()
        })
        .collect();
    let mut found = BTreeSet::new();
    for event in registry().authored_event_keys() {
        suggestions(&darkmatter_base_json_schema_ref()["properties"][event], &mut found);
    }
    let lists: Vec<BTreeSet<String>> = found.into_iter().map(|list| list.into_iter().collect()).collect();
    assert!(!lists.is_empty(), "the stacks suggest bare verbs");
    for list in lists {
        assert_eq!(list, parsed, "every bare-verb suggestion list is the parser's bare verbs");
    }
}

/// `frontmatter` with the defaults `report` says the completion fills.
fn completed(report: &ValidationReport, frontmatter: &serde_json::Value) -> serde_json::Value {
    let mut map: indexmap::IndexMap<String, serde_json::Value> =
        frontmatter.as_object().expect("a mapping").clone().into_iter().collect();
    for applied in &report.defaults {
        simplified_schema::set_at(&mut map, &applied.path, applied.value.clone()).expect("the default's path exists");
    }
    serde_json::Value::Object(map.into_iter().collect())
}

fn md_parse_value(
    parsed: &serde_json::Value,
    authored: Option<&serde_json::Value>,
) -> Result<darkmatter::lifecycle::LifecycleConfig, String> {
    let options = darkmatter::lifecycle::LifecycleParseOptions {
        authored_values: authored,
        ..darkmatter::lifecycle::md_host::parse_options(None)
    };
    darkmatter::lifecycle::parse_lifecycle_config_with(parsed, std::path::Path::new("doc.md"), &options)
        .map_err(|error| error.to_string())
}

/// Every `no_error` default, beside an action and inside a generated or
/// hand-written long form, is `false`; the completed document is valid
/// wherever the authored one is, and md's parser reads the completed value
/// with its authored source exactly as it reads the authored value. Cells
/// cover generated side effects and expression functions, bundles,
/// conditionals, `cancelled`, and `set`.
#[test]
fn completed_flags_keep_the_schema_and_the_md_parser_in_agreement() {
    let fixture = Fixture::new();
    let accepted = [
        "composed: [{ set_frontmatter: { file: a.md, prop: k, value: 0 } }]",
        "composed: [{ set_frontmatter: { file: a.md, prop: k, value: 1 }, no_error: true }]",
        "composed: [{ ensure_file: { file: a.md } }]",
        "composed: [{ ensure_file: { file: a.md, content: hi, no_error: 'yes' } }]",
        "composed: [{ append_line: [a.md, hi], info: hi }]",
        "composed: { info: hi, warn: careful }",
        "composed: { shell: { command: ls } }",
        "composed: [{ shell: { command: ls } , no_error: false }]",
        "composed: [{ when: 'true', then: [{ info: { text: hi } }], else: { stop: { } } }]",
        "composed: [{ when: 'true', info: hi }]",
        "composed: [{ set: { answer: 42 } }]",
        "composed: { set: { answer: 42 }, info: hi, no_error: true }",
        "cancelled: [{ info: { text: bye } }, stop]",
        "cancelled: []",
        "finalize: [{ is_file_type: { value: a.md } }]",
        "initialize: [{ abs: { x: 3 } }, skip]",
    ];
    for frontmatter in accepted {
        let report = fixture.validate(frontmatter);
        assert!(report.valid, "the baseline accepts `{frontmatter}`: {:?}", problems(&report));
        let authored: serde_json::Value = serde_yaml_ng::from_str(frontmatter).expect("YAML frontmatter");
        let completed = completed(&report, &authored);
        assert!(
            report.defaults.iter().all(|applied| applied.value == serde_json::json!(false)
                && applied.path.segments().last().is_some_and(|key| key == "no_error" || key == "create_new")),
            "`{frontmatter}` fills only false flags: {completed}"
        );
        let alone = md_parse_value(&authored, None);
        assert!(alone.is_ok(), "md parses `{frontmatter}`: {alone:?}");
        assert_eq!(md_parse_value(&completed, Some(&authored)), alone, "`{frontmatter}` completed as {completed}");
    }
    // `set` takes no flag, even a written `null`, which the schema's
    // category admits as an omitted flag: S-accept, P-reject.
    let frontmatter = "composed: [{ set: { answer: 42 }, no_error: null }]";
    let authored: serde_json::Value = serde_yaml_ng::from_str(frontmatter).expect("YAML frontmatter");
    let report = fixture.validate(frontmatter);
    assert!(report.valid, "a null flag is an omitted one to the schema: {:?}", problems(&report));
    let error = md_parse_value(&completed(&report, &authored), Some(&authored)).expect_err("set takes no flag");
    assert!(error.contains("`no_error` does not apply to `set`"), "{error}");
    assert_eq!(md_parses(frontmatter), Err(error));
}

/// The shared literal flag binder behind every `no_error` placement (beside
/// an action, inside a generated long form, on a bundle): each placement
/// takes Boolean-like text and refuses a native number, through both the
/// schema and md's parser. The binder's whole shape walk is
/// `lifecycle_edit_schema_parity.rs`'s `edit.gui` rows; these are the
/// control and one invalid shape for each other placement.
#[test]
fn every_no_error_placement_converts_text_and_refuses_a_number_in_both_checks() {
    let fixture = Fixture::new();
    for (placement, accepted, refused) in [
        ("beside an action", "{ info: hi, no_error: 'yes' }", "{ info: hi, no_error: 5 }"),
        ("inside a hand-written long form", "{ warn: { text: hi, no_error: 'yes' } }", "{ warn: { text: hi, no_error: 1 } }"),
        ("on a bundle", "{ info: hi, warn: careful, no_error: 'no' }", "{ info: hi, warn: careful, no_error: 2 }"),
    ] {
        let frontmatter = format!("composed: [{accepted}]");
        let report = fixture.validate(&frontmatter);
        assert!(report.valid, "{placement}: the baseline accepts `{frontmatter}`: {:?}", problems(&report));
        assert_eq!(md_parses(&frontmatter), Ok(()), "{placement}: md parses `{frontmatter}`");

        let frontmatter = format!("composed: [{refused}]");
        let report = fixture.validate(&frontmatter);
        assert!(!report.valid, "{placement}: the baseline rejects `{frontmatter}`");
        assert!(
            report.problems.iter().all(|problem| problem.path.starts_with("/composed/0")),
            "{placement}: every problem sits at the action: {:?}",
            problems(&report)
        );
        let error = md_parses(&frontmatter).expect_err(&format!("{placement}: md refuses `{frontmatter}`"));
        assert!(error.contains("no_error"), "{placement}: the parser names the flag: {error}");
    }
}

/// A side effect's generated long form types `no_error` with the same flag
/// rule wherever a host types the verb (Claudine's recursive profile names
/// the generated type), and md's parser agrees. md's own baseline types every
/// side-effect and expression-function verb `unknown` and leaves their
/// arguments to the parser (see `md-lifecycle.yaml`), so there the schema
/// admits what the parser refuses.
#[test]
fn a_generated_long_form_no_error_follows_the_flag_rule_where_the_verb_is_typed() {
    let fixture = Fixture::new();
    let typed = "$schema:\n    action: ensure_file-long@darkmatter:lifecycle\n";
    let report = fixture.validate_with(&fixture.plain, &format!("{typed}action: {{ file: a.md, no_error: 'yes' }}"));
    assert!(report.valid, "{:?}", problems(&report));
    let report = fixture.validate_with(&fixture.plain, &format!("{typed}action: {{ file: a.md, no_error: 1 }}"));
    assert!(!report.valid, "a number is no flag");
    assert!(report.problems.iter().all(|problem| problem.path.starts_with("/action")), "{:?}", problems(&report));

    assert_eq!(md_parses("composed: [{ ensure_file: { file: a.md, no_error: 'yes' } }]"), Ok(()));
    let frontmatter = "composed: [{ ensure_file: { file: a.md, no_error: 1 } }]";
    let error = md_parses(frontmatter).expect_err("md refuses a numeric flag");
    assert!(error.contains("no_error"), "{error}");
    let report = fixture.validate(frontmatter);
    assert!(report.valid, "md's baseline leaves a side effect's arguments to the parser: {:?}", problems(&report));
}

/// The key-count constraints of the md action dictionaries are judged on the
/// completed value: the filled `no_error: false` is one of the
/// `uncounted-keys`, so a one-verb action and a gated conditional stay
/// valid once it is filled, while a second verb still breaks `max-keys`
/// beside the filled flag, and a flag never makes a bare `when` reach the
/// conditional's `min-keys`. Re-validating the completed value as authored
/// gives the same verdicts.
#[test]
fn key_counts_are_judged_on_the_completed_value_and_a_filled_flag_is_never_counted() {
    let fixture = Fixture::new();
    let schema = "$schema:\n    one: md-communicate@darkmatter:md-lifecycle\n    two: md-communicate@darkmatter:md-lifecycle\n    \
                  gate: md-item-1@darkmatter:md-lifecycle\n    bare: md-conditional-1@darkmatter:md-lifecycle\n";
    let values = "one: { info: hi }\ntwo: { info: hi, warn: careful }\ngate: { when: 'true', then: [stop] }\nbare: { when: 'true', no_error: false }\n";
    let report = fixture.validate_with(&fixture.plain, &format!("{schema}{values}"));
    let filled: BTreeSet<String> = report.defaults.iter().map(|applied| applied.path.segments().join("/")).collect();
    for path in ["one/no_error", "gate/no_error"] {
        assert!(filled.contains(path), "`{path}` is filled: {filled:?}");
    }
    let failed: BTreeSet<String> = report
        .problems
        .iter()
        .map(|problem| problem.path.trim_start_matches('/').split('/').next().unwrap_or_default().to_string())
        .collect();
    assert_eq!(failed, BTreeSet::from(["two".to_string(), "bare".to_string()]), "{:?}", problems(&report));

    let authored: serde_json::Value = serde_yaml_ng::from_str(values).expect("YAML values");
    let completed = completed(&report, &authored);
    assert_eq!(completed["one"], serde_json::json!({ "info": "hi", "no_error": false }));
    let completed_yaml: String = completed
        .as_object()
        .expect("a mapping")
        .iter()
        .map(|(key, value)| format!("{key}: {value}\n"))
        .collect();
    let again = fixture.validate_with(&fixture.plain, &format!("{schema}{completed_yaml}"));
    let failed_again: BTreeSet<String> = again
        .problems
        .iter()
        .map(|problem| problem.path.trim_start_matches('/').split('/').next().unwrap_or_default().to_string())
        .collect();
    assert_eq!(failed_again, failed, "the completed value keeps every verdict: {:?}", problems(&again));
}

/// No default creates an action or an event: a document without lifecycle
/// keys gains none, and an authored stack gains only its `false` flags.
#[test]
fn no_default_creates_an_absent_event_or_action_payload() {
    let fixture = Fixture::new();
    let report = fixture.validate("title: T");
    assert!(report.valid, "{:?}", problems(&report));
    let registry = registry();
    let events: BTreeSet<&str> = registry.authored_event_keys().into_iter().collect();
    let verbs = known_verbs();
    for applied in &report.defaults {
        let segments = applied.path.segments();
        assert!(
            !segments.iter().any(|segment| events.contains(segment.as_str()) || verbs.contains(segment)),
            "a default fills a lifecycle key: {}",
            segments.join("/")
        );
    }
}

/// Pins a known gap: an authored `no_error: false` beside `then` meets the
/// conditional's `literal(false)` (the slot exists for the filled default)
/// and passes the schema, while md's parser refuses every authored flag
/// there. An authored `true` fails both.
#[test]
fn an_authored_false_flag_beside_then_passes_the_schema_and_fails_the_parser() {
    let fixture = Fixture::new();
    let frontmatter = "composed: [{ when: 'true', then: [stop], no_error: false }]";
    let report = fixture.validate(frontmatter);
    assert!(report.valid, "{:?}", problems(&report));
    let error = md_parses(frontmatter).expect_err("an authored flag beside `then`");
    assert!(error.contains("cannot sit beside `then`/`else`"), "{error}");

    let frontmatter = "composed: [{ when: 'true', then: [stop], no_error: true }]";
    assert!(!fixture.validate(frontmatter).valid, "an authored `true` is no `literal(false)`");
    assert!(md_parses(frontmatter).is_err());
}
