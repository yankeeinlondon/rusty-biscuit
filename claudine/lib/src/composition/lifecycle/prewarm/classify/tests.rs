//! Eligibility rules and speculative resolution, one table row per rule.

use std::collections::BTreeSet;
use std::path::Path;

use darkmatter::markdown::compose::ComposeContext;
use darkmatter::markdown::compose::expression::{Expr, reserved_root_descriptors};
use serde_json::{Map, Value, json};

use super::*;
use crate::composition::lifecycle::bindings::binding_view;
use crate::composition::lifecycle::bindings::GroupMembership;
use crate::composition::lifecycle::{LifecycleSignal, parse_lifecycle_config};
use crate::composition::lifecycle::actions::{LifecycleActionKind, LifecycleNode};

const ENV_NAME: &str = "PREWARM_CLASSIFY_NAME";

/// The phrase `say: <authored>` parses to in `signal`'s stack.
fn phrase_in(signal: LifecycleSignal, authored: Value) -> Expr {
    let stack = json!([{ "say": authored }]);
    let stack = match signal {
        LifecycleSignal::Loop => json!({ "while": "false", "gate": stack }),
        _ => stack,
    };
    let mut frontmatter = Map::new();
    frontmatter.insert(signal.property_name().to_string(), stack);
    let config = parse_lifecycle_config(&Value::Object(frontmatter), Path::new("test.md"))
        .unwrap_or_else(|error| panic!("phrase parses: {error}"));
    let LifecycleNode::Action(action) = &config.stack(signal)[0] else {
        panic!("expected an action");
    };
    let LifecycleActionKind::Communication(communication) = &action.kind else {
        panic!("expected speech");
    };
    communication.text.clone()
}

fn frontmatter() -> Map<String, Value> {
    let Value::Object(map) = json!({
        "title": "Widget",
        "count": 3,
        "items": ["alpha", "beta"],
        "err": "document err",
        "nested": { "k": "v" },
        "blank": "",
        "flag": true,
        "key_name": "title",
        "template_value": "{{ title }} $(touch never)",
        "task_input": "per task",
    }) else {
        unreachable!()
    };
    map
}

/// A document capture whose `env` holds exactly one known variable.
fn captured_context() -> ComposeContext {
    let mut context = ComposeContext::capture_minimal();
    let env = context.env_mut();
    env.clear();
    env.insert(ENV_NAME.to_string(), "Ada".to_string());
    context
}

fn inputs() -> PrewarmInputs {
    PrewarmInputs {
        frontmatter: frontmatter(),
        context: DocumentContext::Captured(captured_context()),
        sequence_injected_keys: BTreeSet::new(),
    }
}

fn early(text: &str) -> Classification {
    Classification::Early(text.to_string())
}

fn skip(reason: SkipReason) -> Classification {
    Classification::Skip(reason)
}

fn late(root: &str) -> Classification {
    skip(SkipReason::LateRoot(root.to_string()))
}

fn assert_rows(signal: LifecycleSignal, inputs: &PrewarmInputs, rows: &[(Value, Classification)]) {
    for (authored, expected) in rows {
        let text = phrase_in(signal, authored.clone());
        assert_eq!(
            classify(LifecycleScope::Event(signal), &text, inputs),
            *expected,
            "{authored} in {}",
            signal.property_name()
        );
    }
}

#[test]
fn literal_and_frontmatter_ctx_env_phrases_are_early_with_playback_text() {
    let inputs = inputs();
    let DocumentContext::Captured(context) = &inputs.context else { unreachable!() };
    let today = context.today().to_string();
    assert_rows(
        LifecycleSignal::Success,
        &inputs,
        &[
            (json!("plain literal"), early("plain literal")),
            (json!("{{ fm.title }} is done"), early("Widget is done")),
            (json!("{{ title }} is done"), early("Widget is done")),
            (json!("{{ fm['title'] }}"), early("Widget")),
            (json!("{{ fm.err }}"), early("document err")),
            (json!("{{ nested.k }}"), early("v")),
            (json!("{{ count }}"), early("3")),
            (json!("today is {{ ctx.today }}"), early(&format!("today is {today}"))),
            (json!(format!("{{{{ env.{ENV_NAME} }}}} is done")), early("Ada is done")),
        ],
    );
}

#[test]
fn operators_indexing_fallback_and_ternary_over_early_values_are_early() {
    assert_rows(
        LifecycleSignal::Finalize,
        &inputs(),
        &[
            (json!("{{ count + 1 }} items"), early("4 items")),
            (json!("{{ items[1] }}"), early("beta")),
            (json!("{{ items[-1] }}"), early("beta")),
            (json!("{{ fm[key_name] }}"), early("Widget")),
            (json!("{{ fm.missing || 'fallback' }}"), early("fallback")),
            (json!("{{ flag ? 'yes' : 'no' }}"), early("yes")),
            (json!("{{ count > 2 ? title : 'small' }}"), early("Widget")),
        ],
    );
}

#[test]
fn late_roots_are_skipped_even_in_untaken_branches() {
    let inputs = inputs();
    assert_rows(
        LifecycleSignal::Failure,
        &inputs,
        &[
            (json!("failed: {{ err.msg }}"), late("err")),
            (json!("{{ timing.total }}"), late("timing")),
            (json!("{{ current.branch }}"), late("current")),
            (json!("{{ loop.count }}"), late("loop")),
            (json!("{{ group.name }}"), late("group")),
            (json!("{{ flag ? title : err.msg }}"), late("err")),
            (json!("{{ title || err.msg }}"), late("err")),
            (json!("{{ items[timing.total] }}"), late("timing")),
        ],
    );
    assert_rows(
        LifecycleSignal::Loop,
        &inputs,
        &[(json!("pass {{ loop.count }}"), late("loop"))],
    );
}

#[test]
fn function_calls_are_ineligible_wherever_they_appear() {
    assert_rows(
        LifecycleSignal::Success,
        &inputs(),
        &[
            (json!("{{ upper(title) }}"), skip(SkipReason::FunctionCall)),
            // Zero arguments: no variable read, so a root-only check passes it.
            (json!("on {{ date() }}"), skip(SkipReason::FunctionCall)),
            // A read-only function that probes the login shell.
            (json!("{{ has_alias('ll') ? 'aliased' : 'plain' }}"), skip(SkipReason::FunctionCall)),
            // An untaken branch still disqualifies the phrase.
            (json!("{{ false ? upper(title) : title }}"), skip(SkipReason::FunctionCall)),
            (json!("{{ title || length(items) }}"), skip(SkipReason::FunctionCall)),
            (json!("done: {{ title }} and {{ length(items) }}"), skip(SkipReason::FunctionCall)),
        ],
    );
}

#[test]
fn authored_shell_expansion_is_ineligible_and_never_runs() {
    let dir = tempfile::tempdir().expect("temp dir");
    let marker = dir.path().join("ran");
    let command = format!("$(touch {})", marker.display());
    assert_rows(
        LifecycleSignal::Success,
        &inputs(),
        &[
            (json!(format!("{command} done")), skip(SkipReason::ShellExpansion)),
            (json!("`date` done"), skip(SkipReason::ShellExpansion)),
            (json!("{{ title }} $(date)"), skip(SkipReason::ShellExpansion)),
        ],
    );
    assert!(!marker.exists(), "classification ran an authored command");
}

#[test]
fn data_returned_by_an_interpolation_is_never_evaluated_again() {
    assert_rows(
        LifecycleSignal::Success,
        &inputs(),
        &[
            (json!("{{ template_value }}"), early("{{ title }} $(touch never)")),
            (json!("said: {{ template_value }}"), early("said: {{ title }} $(touch never)")),
        ],
    );
}

#[test]
fn empty_results_are_not_submitted() {
    assert_rows(
        LifecycleSignal::Success,
        &inputs(),
        &[
            (json!(""), skip(SkipReason::Empty)),
            (json!("   "), skip(SkipReason::Empty)),
            (json!("{{ blank }}"), skip(SkipReason::Empty)),
            (json!("{{ fm.missing }}"), skip(SkipReason::Empty)),
            (json!("  {{ blank }}  "), skip(SkipReason::Empty)),
        ],
    );
}

#[test]
fn a_failed_speculative_resolution_is_a_silent_skip() {
    assert_rows(
        LifecycleSignal::Success,
        &inputs(),
        &[(json!("{{ title * 2 }}"), skip(SkipReason::ResolutionFailed))],
    );
}

#[test]
fn a_whole_value_phrase_whose_text_cannot_be_recovered_is_skipped() {
    // `Display` renders the index as `fm["a"b"]`, which does not parse back,
    // so the phrase is not analyzed rather than analyzed as something else.
    assert_rows(
        LifecycleSignal::Success,
        &inputs(),
        &[(json!("{{ fm['a\"b'] }}"), skip(SkipReason::Unparseable))],
    );
}

#[test]
fn sequence_injected_keys_are_late_through_bare_dotted_and_bracket_reads() {
    let mut inputs = inputs();
    inputs.sequence_injected_keys = BTreeSet::from(["task_input".to_string()]);
    let injected = || skip(SkipReason::SequenceInjected("task_input".to_string()));
    assert_rows(
        LifecycleSignal::Success,
        &inputs,
        &[
            (json!("{{ task_input }}"), injected()),
            (json!("{{ fm.task_input }}"), injected()),
            (json!("{{ fm['task_input'] }}"), injected()),
            (json!("{{ flag ? title : task_input }}"), injected()),
            (json!("{{ fm[key_name] }}"), skip(SkipReason::TaskDependentRead)),
            (json!("{{ fm }}"), skip(SkipReason::TaskDependentRead)),
            (json!("{{ (fm).title }}"), skip(SkipReason::TaskDependentRead)),
            // A key the sequence does not inject stays early.
            (json!("{{ fm.title }} and {{ title }}"), early("Widget and Widget")),
        ],
    );
}

#[test]
fn ctx_and_env_wait_for_the_documents_own_capture() {
    let mut inputs = inputs();
    inputs.context = DocumentContext::Pending(captured_context());
    assert_rows(
        LifecycleSignal::Success,
        &inputs,
        &[
            (json!("{{ ctx.today }}"), skip(SkipReason::ContextUnavailable)),
            (json!(format!("{{{{ env.{ENV_NAME} }}}}")), skip(SkipReason::ContextUnavailable)),
            (json!("{{ title }}"), early("Widget")),
        ],
    );
}

#[test]
fn task_stacks_classify_against_their_own_scope() {
    let text = phrase_in(LifecycleSignal::Success, json!("setting up {{ title }}"));
    for scope in [LifecycleScope::TaskSetup, LifecycleScope::TaskTeardown] {
        assert_eq!(classify(scope, &text, &inputs()), early("setting up Widget"));
    }
    let late_text = phrase_in(LifecycleSignal::Failure, json!("{{ err.msg }}"));
    for scope in [LifecycleScope::TaskSetup, LifecycleScope::TaskTeardown] {
        assert_eq!(classify(scope, &late_text, &inputs()), late("err"));
    }
}

/// Every root a lifecycle phrase can name, with its decided timing. Adding a
/// reserved namespace or a declared lifecycle global fails this test until
/// the new root is placed here.
const DECIDED_ROOTS: &[(&str, bool)] = &[
    ("fm", true),
    ("ctx", true),
    ("env", true),
    ("current", false),
    ("loop", false),
    ("err", false),
    ("timing", false),
    ("group", false),
    ("outputs", false),
];

#[test]
fn every_reserved_namespace_and_declared_global_has_a_decided_timing() {
    let mut roots: BTreeSet<String> = reserved_root_descriptors()
        .iter()
        .map(|root| root.name.to_string())
        .collect();
    for scope in LifecycleScope::ALL {
        for group in [GroupMembership::Member, GroupMembership::Outside, GroupMembership::Unknown] {
            roots.extend(binding_view(scope, group).globals().map(|global| global.root().to_string()));
        }
    }
    let decided: BTreeSet<String> = DECIDED_ROOTS.iter().map(|(root, _)| root.to_string()).collect();
    assert_eq!(roots, decided, "a root was added or removed without deciding its pre-warm timing");

    let inputs = inputs();
    for scope in LifecycleScope::ALL {
        let view = binding_view(scope, GroupMembership::Unknown);
        for (root, is_early) in DECIDED_ROOTS {
            let declared = reserved_root_descriptors().iter().any(|r| r.name == *root)
                || view.global(root).is_some();
            if !declared {
                continue;
            }
            let text = Expr::StringLiteral(format!("{{{{ {root}.today }}}}"));
            let outcome = classify(scope, &text, &inputs);
            if *is_early {
                assert!(
                    !matches!(outcome, Classification::Skip(SkipReason::LateRoot(_))),
                    "{root} in {scope:?}: {outcome:?}"
                );
            } else {
                assert_eq!(outcome, late(root), "{root} in {scope:?}");
            }
        }
    }
}
