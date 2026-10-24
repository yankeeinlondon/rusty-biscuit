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
    assert!(provided.len() > 120 && provided.contains("info") && provided.contains("shell"), "{provided:?}");
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
