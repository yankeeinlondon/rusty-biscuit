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
        // A root dictionary never carries `when`, as md's parser reads it.
        ("{ info: hi, when: 'true', no_error: true }", false),
        ("{ info: hi, no_error: true }", true),
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
    // `set` takes no flag, even a written `null`: both refuse it, and the
    // completed value parsed with its authored source is refused the same way.
    let frontmatter = "composed: [{ set: { answer: 42 }, no_error: null }]";
    let authored: serde_json::Value = serde_yaml_ng::from_str(frontmatter).expect("YAML frontmatter");
    let report = fixture.validate(frontmatter);
    assert!(!report.valid, "the baseline refuses a flag beside `set`");
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
/// the generated type), and md's baseline, which leaves the side effect's
/// arguments untyped, still judges its flag by the parser's rule: both refuse
/// a number there.
#[test]
fn a_long_form_no_error_follows_the_flag_rule_whether_or_not_the_verb_is_typed() {
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
    assert!(!report.valid, "md's baseline refuses the numeric flag too");
    assert!(
        report.problems.iter().any(|problem| problem.path == "/composed/0/ensure_file/no_error"),
        "{:?}",
        problems(&report)
    );
}

/// The key-count constraints of the md action dictionaries are judged on the
/// completed value: the filled `no_error: false` is one of the
/// `uncounted-keys`, so a one-verb action and a gated conditional stay
/// valid once it is filled, while a second verb still breaks `max-keys`
/// beside the filled flag, and a flag never makes a bare `when` reach the
/// conditional's `min-keys`. Re-validating the completed document, as
/// composition does once it has inserted the defaults, gives the same
/// verdicts when the authored values show which flags were written.
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
    let path = fixture.root.join("doc.md");
    std::fs::write(&path, format!("---\n{schema}{values}---\nBody\n")).expect("write document");
    let markdown = Markdown::try_from(path.as_path()).expect("the document loads").with_source(ComposeSource::File(path));
    let effective = fixture.plain.effective_for(&markdown).expect("prepares").expect("the document schema applies");
    let again = effective.validate_completed(&completed, &authored, None).expect("validates");
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

/// An authored `no_error` beside `then` is refused by both, `false` included:
/// the conditional's `literal(false)` slot exists only so the filled default
/// validates, and a filled flag is not authored. Absent and `null` are
/// omitted flags and pass both.
#[test]
fn an_authored_flag_beside_then_fails_the_schema_and_the_parser() {
    let fixture = Fixture::new();
    for flag in ["false", "true", "'no'"] {
        let frontmatter = format!("composed: [{{ when: 'true', then: [stop], no_error: {flag} }}]");
        let report = fixture.validate(&frontmatter);
        assert!(!report.valid, "the baseline refuses `{frontmatter}`");
        assert!(report.problems.iter().all(|problem| problem.path.starts_with("/composed/0")), "{:?}", problems(&report));
        let error = md_parses(&frontmatter).expect_err("an authored flag beside `then`");
        assert!(error.contains("no_error"), "{error}");
    }
    for frontmatter in ["composed: [{ when: 'true', then: [stop] }]", "composed: [{ when: 'true', then: [stop], no_error: null }]"] {
        let report = fixture.validate(frontmatter);
        assert!(report.valid, "`{frontmatter}`: {:?}", problems(&report));
        assert_eq!(md_parses(frontmatter), Ok(()), "{frontmatter}");
    }
}

/// How one authored frontmatter is judged by both checks.
enum Verdict {
    /// Both accept it.
    Accept,
    /// Both refuse it: the schema at the pointer, the parser with a message
    /// holding the needle.
    Reject(&'static str, &'static str),
}

/// One agreement row: `frontmatter` gets `verdict` from the baseline and
/// from md's parser.
fn agree(frontmatter: &str, verdict: Verdict) {
    let report = Fixture::new().validate(frontmatter);
    let parsed = md_parses(frontmatter);
    match verdict {
        Verdict::Accept => {
            assert!(report.valid, "the baseline accepts `{frontmatter}`: {:?}", problems(&report));
            assert_eq!(parsed, Ok(()), "md parses `{frontmatter}`");
        }
        Verdict::Reject(pointer, needle) => {
            assert!(!report.valid, "the baseline refuses `{frontmatter}`");
            assert!(
                report.problems.iter().any(|problem| problem.path == pointer),
                "`{frontmatter}` is refused at `{pointer}`: {:?}",
                problems(&report)
            );
            assert!(
                report.problems.iter().all(|problem| problem.line.is_some()),
                "every problem is located: {:?}",
                report.problems
            );
            let error = parsed.expect_err(&format!("md refuses `{frontmatter}`"));
            assert!(error.contains(needle), "`{frontmatter}`: `{needle}` in {error}");
        }
    }
}

/// Rows of authored frontmatter that the baseline and md's parser must
/// judge alike, one test each.
macro_rules! agreement_rows {
    ($($name:ident: $frontmatter:expr => $verdict:expr;)+) => {
        $(
            #[test]
            fn $name() {
                agree($frontmatter, $verdict);
            }
        )+
    };
}

agreement_rows! {
    // A side effect's and an expression function's long form flag, which
    // the baseline leaves untyped, follows the flag rule.
    side_effect_numeric_flag_is_refused_by_both:
        "composed: [{ ensure_file: { file: a.md, no_error: 1 } }]" => Verdict::Reject("/composed/0/ensure_file/no_error", "no_error");
    expression_function_numeric_flag_is_refused_by_both:
        "composed: [{ abs: { x: 3, no_error: 1 } }]" => Verdict::Reject("/composed/0/abs/no_error", "no_error");
    side_effect_absent_flag_is_accepted_by_both:
        "composed: [{ ensure_file: { file: a.md } }]" => Verdict::Accept;
    side_effect_null_flag_is_accepted_by_both:
        "composed: [{ ensure_file: { file: a.md, no_error: null } }]" => Verdict::Accept;
    side_effect_false_flag_is_accepted_by_both:
        "composed: [{ ensure_file: { file: a.md, no_error: false } }]" => Verdict::Accept;
    side_effect_boolean_like_flag_is_accepted_by_both:
        "composed: [{ ensure_file: { file: a.md, no_error: 'yes' } }]" => Verdict::Accept;
    side_effect_word_flag_is_refused_by_both:
        "composed: [{ ensure_file: { file: a.md, no_error: maybe } }]" => Verdict::Reject("/composed/0/ensure_file/no_error", "no_error");
    side_effect_empty_text_flag_is_refused_by_both:
        "composed: [{ ensure_file: { file: a.md, no_error: '' } }]" => Verdict::Reject("/composed/0/ensure_file/no_error", "no_error");
    side_effect_list_flag_is_refused_by_both:
        "composed: [{ ensure_file: { file: a.md, no_error: [] } }]" => Verdict::Reject("/composed/0/ensure_file/no_error", "no_error");
    side_effect_mapping_flag_is_refused_by_both:
        "composed: [{ ensure_file: { file: a.md, no_error: {} } }]" => Verdict::Reject("/composed/0/ensure_file/no_error", "no_error");
    // A verb md does not provide is never read, so neither judges its value.
    unprovided_verb_long_form_flag_is_left_unread_by_both:
        "composed: [{ say: { text: hi, no_error: 1 } }]" => Verdict::Accept;
    unprovided_verb_duplicate_flag_is_left_unread_by_both:
        "composed: [{ say: { text: hi, no_error: true }, no_error: true }]" => Verdict::Accept;

    // An authored flag beside `then`, at every typed conditional level and
    // in `initialize`'s profile.
    md_conditional_1_authored_false_flag_is_refused_by_both:
        "composed: [{ when: 'true', then: [stop], no_error: false }]" => Verdict::Reject("/composed/0/no_error", "cannot sit beside `then`/`else`");
    md_conditional_2_authored_false_flag_is_refused_by_both:
        "composed: [{ when: 'true', then: [{ when: 'true', then: [stop], no_error: false }] }]" => Verdict::Reject("/composed/0/then/0/no_error", "cannot sit beside `then`/`else`");
    md_conditional_3_authored_false_flag_is_refused_by_both:
        "composed: [{ when: 'true', then: [{ when: 'true', then: [{ when: 'true', then: [stop], no_error: false }] }] }]" => Verdict::Reject("/composed/0/then/0/then/0/no_error", "cannot sit beside `then`/`else`");
    md_initialize_conditional_1_authored_false_flag_is_refused_by_both:
        "initialize: [{ when: 'true', then: [skip], no_error: false }]" => Verdict::Reject("/initialize/0/no_error", "cannot sit beside `then`/`else`");
    md_initialize_conditional_2_authored_false_flag_is_refused_by_both:
        "initialize: [{ when: 'true', then: [{ when: 'true', then: [skip], no_error: false }] }]" => Verdict::Reject("/initialize/0/then/0/no_error", "cannot sit beside `then`/`else`");
    md_initialize_conditional_3_authored_false_flag_is_refused_by_both:
        "initialize: [{ when: 'true', then: [{ when: 'true', then: [{ when: 'true', then: [skip], no_error: false }] }] }]" => Verdict::Reject("/initialize/0/then/0/then/0/no_error", "cannot sit beside `then`/`else`");
    conditional_flag_beside_else_is_refused_by_both:
        "composed: [{ when: 'true', then: [stop], else: [{ info: hi }], no_error: false }]" => Verdict::Reject("/composed/0/no_error", "cannot sit beside `then`/`else`");

    // Two authored placements of one flag, in list items, in a stack-root
    // category, and in the bundled long form of a side effect.
    edit_duplicate_flags_are_refused_by_both:
        "composed: [{ edit: { file: notes.md, no_error: false }, no_error: false }]" => Verdict::Reject("/composed/0/edit/no_error", "set both beside `edit` and inside it");
    info_duplicate_flags_are_refused_by_both:
        "composed: [{ info: { text: hi, no_error: false }, no_error: false }]" => Verdict::Reject("/composed/0/info/no_error", "set both beside `info` and inside it");
    shell_duplicate_flags_are_refused_by_both:
        "composed: [{ shell: { command: ls, no_error: true }, no_error: true }]" => Verdict::Reject("/composed/0/shell/no_error", "set both beside `shell` and inside it");
    root_category_duplicate_flags_are_refused_by_both:
        "composed: { info: { text: hi, no_error: 'yes' }, no_error: false }" => Verdict::Reject("/composed/info/no_error", "set both beside `info` and inside it");
    body_category_duplicate_flags_are_refused_by_both:
        "composed: [{ when: 'true', then: { warn: { text: hi, no_error: true }, no_error: true } }]" => Verdict::Reject("/composed/0/then/warn/no_error", "set both beside `warn` and inside it");
    side_effect_duplicate_flags_are_refused_by_both:
        "composed: [{ ensure_file: { file: a.md, no_error: false }, no_error: false }]" => Verdict::Reject("/composed/0/ensure_file/no_error", "set both beside `ensure_file` and inside it");
    one_flag_beside_a_long_form_is_accepted_by_both:
        "composed: [{ info: { text: hi }, no_error: true }]" => Verdict::Accept;
    null_sibling_beside_an_inner_flag_is_accepted_by_both:
        "composed: [{ info: { text: hi, no_error: true }, no_error: null }]" => Verdict::Accept;

    // `set` takes no flag, a written `null` included.
    set_null_flag_is_refused_by_both:
        "composed: [{ set: { answer: 42 }, no_error: null }]" => Verdict::Reject("/composed/0/no_error", "`no_error` does not apply to `set`");
    set_false_flag_is_refused_by_both:
        "composed: [{ set: { answer: 42 }, no_error: false }]" => Verdict::Reject("/composed/0/no_error", "`no_error` does not apply to `set`");
    set_root_null_flag_is_refused_by_both:
        "composed: { set: { answer: 42 }, no_error: null }" => Verdict::Reject("/composed/no_error", "`no_error` does not apply to `set`");
    set_without_a_flag_is_accepted_by_both:
        "composed: [{ set: { answer: 42 } }]" => Verdict::Accept;
    set_in_a_flagged_bundle_is_accepted_by_both:
        "composed: { set: { answer: 42 }, info: hi, no_error: true }" => Verdict::Accept;

    // A `when` with nothing to guard, and the other branch shapes.
    bare_when_is_refused_by_both:
        "composed: [{ when: 'true' }]" => Verdict::Reject("/composed/0", "needs `then` or sibling actions");
    bare_when_with_a_flag_is_refused_by_both:
        "composed: [{ when: 'true', no_error: true }]" => Verdict::Reject("/composed/0", "needs `then` or sibling actions");
    initialize_bare_when_is_refused_by_both:
        "initialize: [{ when: 'true' }, skip]" => Verdict::Reject("/initialize/0", "needs `then` or sibling actions");
    nested_bare_when_is_refused_by_both:
        "composed: [{ when: 'true', then: [{ when: 'false' }] }]" => Verdict::Reject("/composed/0/then/0", "needs `then` or sibling actions");
    guarded_action_is_accepted_by_both:
        "composed: [{ when: 'true', info: hi }]" => Verdict::Accept;
    when_beside_a_root_dictionary_is_refused_by_both:
        "composed: { when: 'true', info: hi }" => Verdict::Reject("/composed/when", "never carries `when`");
    when_beside_a_body_dictionary_is_refused_by_both:
        "composed: [{ when: 'true', then: { when: 'false', info: hi } }]" => Verdict::Reject("/composed/0/then/when", "never carries `when`");
    else_without_then_is_refused_by_both:
        "composed: [{ when: 'true', else: [stop] }]" => Verdict::Reject("/composed/0/else", "`else` requires `then`");
    sibling_action_beside_then_is_refused_by_both:
        "composed: [{ when: 'true', then: [stop], info: hi }]" => Verdict::Reject("/composed/0", "cannot also carry the sibling `info`");
    then_without_when_is_refused_by_both:
        "composed: [{ then: [stop] }]" => Verdict::Reject("/composed/0", "requires `when`");
    flag_alone_in_an_item_is_refused_by_both:
        "composed: [{ no_error: true }]" => Verdict::Reject("/composed/0", "needs an action beside it");
    flag_alone_at_a_stack_root_is_refused_by_both:
        "composed: { no_error: true }" => Verdict::Reject("/composed", "needs an action beside it");
    null_flag_alone_at_a_stack_root_is_an_empty_stack_to_both:
        "composed: { no_error: null }" => Verdict::Accept;
}

/// `event`'s stack whose level-3 conditional holds `body` as its `then`: the
/// body md's baseline types loosely (`md-loose-body`), which validation judges
/// with the parser's placement and flag rules all the same.
macro_rules! below_level_3 {
    ($event:literal, $body:literal) => {
        concat!($event, ": [{ when: 'true', then: [{ when: 'true', then: [{ when: 'true', then: ", $body, " }] }] }]")
    };
}

agreement_rows! {
    // Below the typed levels, in `composed` and in `initialize`'s profile.
    loose_body_authored_flag_beside_then_is_refused_by_both:
        below_level_3!("composed", "[{ when: 'true', then: [stop], no_error: false }]")
            => Verdict::Reject("/composed/0/then/0/then/0/then/0/no_error", "cannot sit beside `then`/`else`");
    loose_body_fifth_level_flag_beside_then_is_refused_by_both:
        below_level_3!("composed", "[{ when: 'true', then: [{ when: 'true', then: [stop], no_error: false }] }]")
            => Verdict::Reject("/composed/0/then/0/then/0/then/0/then/0/no_error", "cannot sit beside `then`/`else`");
    loose_body_bare_when_is_refused_by_both:
        below_level_3!("composed", "[{ when: 'true' }]")
            => Verdict::Reject("/composed/0/then/0/then/0/then/0", "needs `then` or sibling actions");
    loose_else_body_bare_when_is_refused_by_both:
        "composed: [{ when: 'true', then: [{ when: 'true', then: [{ when: 'true', then: [stop], else: [{ when: 'false' }] }] }] }]"
            => Verdict::Reject("/composed/0/then/0/then/0/else/0", "needs `then` or sibling actions");
    loose_body_duplicate_flags_are_refused_by_both:
        below_level_3!("composed", "[{ info: { text: hi, no_error: false }, no_error: false }]")
            => Verdict::Reject("/composed/0/then/0/then/0/then/0/info/no_error", "set both beside `info` and inside it");
    loose_body_set_flag_is_refused_by_both:
        below_level_3!("composed", "[{ set: { answer: 42 }, no_error: null }]")
            => Verdict::Reject("/composed/0/then/0/then/0/then/0/no_error", "`no_error` does not apply to `set`");
    loose_body_side_effect_numeric_flag_is_refused_by_both:
        below_level_3!("composed", "[{ ensure_file: { file: a.md, no_error: 1 } }]")
            => Verdict::Reject("/composed/0/then/0/then/0/then/0/ensure_file/no_error", "no_error");
    loose_body_info_numeric_flag_is_refused_by_both:
        below_level_3!("composed", "[{ info: { text: hi, no_error: 1 } }]")
            => Verdict::Reject("/composed/0/then/0/then/0/then/0/info/no_error", "no_error");
    loose_body_numeric_sibling_flag_is_refused_by_both:
        below_level_3!("composed", "[{ info: hi, no_error: 1 }]")
            => Verdict::Reject("/composed/0/then/0/then/0/then/0/no_error", "no_error");
    loose_body_dictionary_with_when_is_refused_by_both:
        below_level_3!("composed", "{ when: 'false', info: hi }")
            => Verdict::Reject("/composed/0/then/0/then/0/then/when", "never carries `when`");
    loose_body_dictionary_duplicate_flags_are_refused_by_both:
        below_level_3!("composed", "{ warn: { text: hi, no_error: true }, no_error: true }")
            => Verdict::Reject("/composed/0/then/0/then/0/then/warn/no_error", "set both beside `warn` and inside it");
    loose_body_valid_actions_are_accepted_by_both:
        below_level_3!("composed", "[{ when: 'true', then: [{ info: { text: hi }, no_error: true }], else: { warn: careful } }, { ensure_file: { file: a.md, no_error: 'yes' } }, stop]")
            => Verdict::Accept;
    loose_body_unprovided_verb_flag_is_left_unread_by_both:
        below_level_3!("composed", "[{ say: { text: hi, no_error: 1 } }, { say: { text: hi, no_error: true }, no_error: true }]")
            => Verdict::Accept;
    loose_body_null_flags_are_omitted_by_both:
        below_level_3!("composed", "[{ when: 'true', then: [stop], no_error: null }]")
            => Verdict::Accept;
    initialize_loose_body_authored_flag_beside_then_is_refused_by_both:
        below_level_3!("initialize", "[{ when: 'true', then: [skip], no_error: false }]")
            => Verdict::Reject("/initialize/0/then/0/then/0/then/0/no_error", "cannot sit beside `then`/`else`");
    initialize_loose_body_bare_when_is_refused_by_both:
        below_level_3!("initialize", "[{ when: 'true' }]")
            => Verdict::Reject("/initialize/0/then/0/then/0/then/0", "needs `then` or sibling actions");
    initialize_loose_body_duplicate_flags_are_refused_by_both:
        below_level_3!("initialize", "[{ info: { text: hi, no_error: false }, no_error: false }]")
            => Verdict::Reject("/initialize/0/then/0/then/0/then/0/info/no_error", "set both beside `info` and inside it");
    initialize_loose_body_set_flag_is_refused_by_both:
        below_level_3!("initialize", "[{ set: { answer: 42 }, no_error: false }]")
            => Verdict::Reject("/initialize/0/then/0/then/0/then/0/no_error", "`no_error` does not apply to `set`");
    initialize_loose_body_side_effect_numeric_flag_is_refused_by_both:
        below_level_3!("initialize", "[{ ensure_file: { file: a.md, no_error: 1 } }]")
            => Verdict::Reject("/initialize/0/then/0/then/0/then/0/ensure_file/no_error", "no_error");
    initialize_loose_body_valid_actions_are_accepted_by_both:
        below_level_3!("initialize", "[{ when: 'true', then: [skip] }, { info: hi, no_error: true }]")
            => Verdict::Accept;
}

/// `event`'s stack of `depth` nested conditionals, each descending through
/// `then`, the innermost holding `leaf`.
fn nested_conditionals(event: &str, depth: usize, leaf: &str) -> String {
    let body = (0..depth).fold(leaf.to_string(), |inner, _| format!("[{{ when: 'true', then: {inner} }}]"));
    format!("{event}: {body}")
}

/// The schema encodes no conditional depth, as a recursive profile does not:
/// five levels pass both checks in `composed` and `initialize`, and a sixth
/// is the parser's alone to refuse, with nothing in the untyped bodies
/// reported beyond the parser's rules.
#[test]
fn a_sixth_conditional_level_is_the_parsers_alone_to_refuse() {
    let fixture = Fixture::new();
    for (event, leaf) in [("composed", "[stop]"), ("initialize", "[skip]")] {
        let five = nested_conditionals(event, 5, leaf);
        assert!(fixture.validate(&five).valid, "`{five}`");
        assert_eq!(md_parses(&five), Ok(()), "{five}");
        let six = nested_conditionals(event, 6, leaf);
        let report = fixture.validate(&six);
        assert!(report.valid, "`{six}`: {:?}", problems(&report));
        let error = md_parses(&six).expect_err("a sixth level");
        assert!(error.contains("conditionals nest at most 5 levels"), "{error}");
    }
}

/// Composition completes the typed levels and leaves an untyped body as
/// written, so a completed document with actions below level 3 keeps its
/// authored verdict: the filled flags of the typed levels are not
/// placements, and an authored one in the untyped body still is.
#[test]
fn a_completed_document_with_an_untyped_body_is_judged_by_its_authored_flags() {
    let fixture = Fixture::new();
    let frontmatter = nested_conditionals("composed", 4, "[{ edit: { file: notes.md } }, { set: { answer: 42 } }, { when: 'true', then: [stop] }]");
    let report = fixture.validate(&frontmatter);
    assert!(report.valid, "{:?}", problems(&report));
    let authored: serde_json::Value = serde_yaml_ng::from_str(&frontmatter).expect("YAML frontmatter");
    let completed = completed(&report, &authored);
    assert_eq!(completed.pointer("/composed/0/then/0/then/0/no_error"), Some(&serde_json::json!(false)), "level 3 is completed: {completed}");
    assert_eq!(
        completed.pointer("/composed/0/then/0/then/0/then/0"),
        authored.pointer("/composed/0/then/0/then/0/then/0"),
        "the untyped body is left as written"
    );
    let effective = {
        let path = fixture.root.join("doc.md");
        std::fs::write(&path, format!("---\n{frontmatter}\n---\nBody\n")).expect("write document");
        let markdown = Markdown::try_from(path.as_path()).expect("the document loads").with_source(ComposeSource::File(path));
        fixture.baseline.effective_for(&markdown).expect("prepares").expect("the baseline applies")
    };
    let report = effective.validate_completed(&completed, &authored, None).expect("validates");
    assert!(report.valid, "{:?}", problems(&report));
    assert_eq!(md_parse_value(&completed, Some(&authored)).map(|_| ()), Ok(()));

    let frontmatter = nested_conditionals("composed", 4, "[{ set: { answer: 42 }, no_error: false }]");
    let authored: serde_json::Value = serde_yaml_ng::from_str(&frontmatter).expect("YAML frontmatter");
    let report = effective.validate_completed(&authored, &authored, None).expect("validates");
    assert!(
        report.problems.iter().any(|problem| problem.path == "/composed/0/then/0/then/0/then/0/then/0/no_error"),
        "an authored flag on `set` below level 3 is refused: {:?}",
        problems(&report)
    );
}

/// A completed document keeps its authored verdict when the validator knows
/// which flags were authored: the filled flags (beside `then`, beside and
/// inside a long form, beside `set`) were not written. Judged as authored,
/// the same completed value holds those placements, so
/// `DarkmatterSchemas::validate` judges a document as it is given, and
/// composition, which completes the document in place, tells its verdict
/// what it filled (`composition_defaults.rs`).
#[test]
fn a_completed_document_is_judged_by_its_authored_flags() {
    let fixture = Fixture::new();
    let frontmatter = "composed:\n  - when: 'true'\n    then: [stop]\n  - edit: { file: notes.md }\n  - set: { answer: 42 }\n";
    let report = fixture.validate(frontmatter);
    assert!(report.valid, "{:?}", problems(&report));
    let authored: serde_json::Value = serde_yaml_ng::from_str(frontmatter).expect("YAML frontmatter");
    let completed = completed(&report, &authored);
    for pointer in ["/composed/0/no_error", "/composed/1/no_error", "/composed/1/edit/no_error", "/composed/2/no_error"] {
        assert_eq!(completed.pointer(pointer), Some(&serde_json::json!(false)), "`{pointer}` is filled: {completed}");
    }

    // The document completed in place is judged as given.
    let path = fixture.root.join("doc.md");
    std::fs::write(&path, format!("---\n{frontmatter}---\nBody\n")).expect("write document");
    let mut markdown = Markdown::try_from(path.as_path()).expect("the document loads").with_source(ComposeSource::File(path));
    for (key, value) in completed.as_object().expect("a mapping") {
        markdown.frontmatter_mut().as_map_mut().insert(key.clone(), value.clone());
    }
    let report = fixture.baseline.validate(&markdown).expect("the schema prepares");
    assert!(!report.valid, "a document completed in place is judged as written");

    let effective = fixture.baseline.effective_for(&markdown).expect("prepares").expect("the baseline applies");
    let report = effective.validate_completed(&completed, &authored, None).expect("validates");
    assert!(report.valid, "{:?}", problems(&report));
    let report = effective.validate(&completed);
    assert!(!report.valid, "judged as authored, the completed value holds placements");

    // An authored placement in the same document still fails.
    let frontmatter = "composed:\n  - when: 'true'\n    then: [stop]\n    no_error: false\n";
    let authored: serde_json::Value = serde_yaml_ng::from_str(frontmatter).expect("YAML frontmatter");
    let report = effective.validate_completed(&authored, &authored, None).expect("validates");
    assert!(!report.valid, "an authored flag beside `then` is refused");
}
