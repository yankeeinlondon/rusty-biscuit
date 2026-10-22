//! Lifecycle shell sites typed in the scope of their enclosing `when` guards:
//! pre-flight accepts a value a guard proves present even when it is missing
//! now, the selected action reads and checks it live, and writes and other
//! guards never stretch that proof.

use super::*;
use crate::composition::error::CompositionError;
use crate::composition::lifecycle::shell_template::{LifecycleShellTemplate, lifecycle_declared_types, typed_template};
use crate::composition::preflight::{resolve_lifecycle_shell_commands, settle_lifecycle_shell_modes};
use darkmatter::markdown::compose::expression::TypeScope;
use darkmatter::markdown::compose::shell_expansion::template::{HoleTiming, ModeRequirement, ShellPreApprovals};

const SOURCE: &str = "t.md";

/// `frontmatter`'s lifecycle after pre-flight resolved its shell sites.
pub(super) fn resolve(frontmatter: &Value) -> Result<LifecycleConfig, CompositionError> {
    let mut config = parse_lifecycle_config(frontmatter, Path::new(SOURCE)).unwrap();
    let context = ComposeContext::capture_for_content(Path::new("."), "");
    resolve_lifecycle_shell_commands(
        &mut config,
        frontmatter,
        &context,
        Path::new(SOURCE),
        crate::test_support::process_context(),
        None,
        crate::composition::lifecycle::bindings::EnclosingLoop::Outside,
        None,
    )?;
    Ok(config)
}

/// The missing-value refusal pre-flight raised, naming its placeholder.
pub(super) fn missing(result: Result<LifecycleConfig, CompositionError>) -> String {
    match result {
        Err(error @ CompositionError::LifecycleShellResolution { .. }) => {
            let message = error.to_string();
            assert!(message.contains("which is missing"), "{message}");
            message
        }
        other => panic!("expected a missing-value refusal, got {other:?}"),
    }
}

/// Every shell site of `signal`'s stack, in authored order.
pub(super) fn shell_sites(config: &LifecycleConfig, signal: LifecycleSignal) -> Vec<LifecycleShellTemplate> {
    let mut sites = Vec::new();
    crate::composition::lifecycle_actions::for_each_action(config.stack(signal), &mut |action| match &action.kind {
        LifecycleActionKind::Shell(shell) => sites.extend(shell.template.clone()),
        LifecycleActionKind::RuntimeSet(set) => sites.extend(set.shell_values().filter_map(|(_, value)| value.template.clone())),
        _ => {}
    });
    sites
}

/// The site's placeholders whose timing is `Guarded`, with their guard label.
pub(super) fn guarded_holes(site: &LifecycleShellTemplate) -> Vec<(String, String)> {
    site.template()
        .placeholders()
        .filter_map(|placeholder| match &placeholder.timing {
            HoleTiming::Guarded { guard, .. } => Some((placeholder.expression.clone(), guard.label().to_string())),
            _ => None,
        })
        .collect()
}

pub(super) fn shape_of(command: &str) -> ShellPreApprovals {
    let declared = lifecycle_declared_types(None, crate::composition::claudine_registry());
    let template = typed_template(command, &TypeScope::unguarded(&declared)).expect("a template");
    let mut approvals = ShellPreApprovals::new();
    approvals.insert_template(TemplateShape::from_template(&template));
    approvals
}

pub(super) fn run(signal: LifecycleSignal, config: &LifecycleConfig, state: Value, shell: &MockShell) -> LifecycleEventOutcome {
    let (_dir, engine) = temp_engine();
    let recorder = Recorder::default();
    let harness = Harness::default();
    ctx(signal, &map(state), None, &engine, shell, &recorder, &harness, Path::new(SOURCE)).execute_event(config)
}

pub(super) fn args_of(shell: &MockShell) -> Vec<Vec<String>> {
    shell
        .chains()
        .into_iter()
        .flat_map(|chain| chain.segments)
        .map(|segment| std::iter::once(segment.program).chain(segment.args).collect())
        .collect()
}

const COMMIT: &str = "git commit -m \"{{commit_message}}\"";

#[test]
fn a_guard_proven_missing_value_waits_for_its_template_and_runs_only_when_selected() {
    let frontmatter = json!({
        "dirty": true,
        "success": [{ "when": "dirty && commit_message", "then": [{ "shell": COMMIT }] }],
    });

    let config = resolve(&frontmatter).expect("a guarded missing value passes pre-flight");

    let [site] = shell_sites(&config, LifecycleSignal::Success).try_into().expect("one site");
    assert_eq!(site.resolved(), None, "no bytes stand in for the missing value");
    assert_eq!(site.candidate(Path::new(SOURCE), "success[0].then[0].shell.command").requirement, ModeRequirement::TemplateRequired);
    assert_eq!(guarded_holes(&site), vec![("commit_message".to_string(), "success[0].when".to_string())]);

    // Approved by its shape, the selected branch reads the live message as
    // one argument; the unselected branch launches nothing.
    settle_lifecycle_shell_modes(&config, &LifecycleSignal::ALL, &shape_of(COMMIT), None);
    let selected = MockShell::new(0);
    let outcome = run(LifecycleSignal::Success, &config, json!({ "dirty": true, "commit_message": "a b; c" }), &selected);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(args_of(&selected), vec![vec!["git", "commit", "-m", "a b; c"]]);

    let skipped = MockShell::new(0);
    let outcome = run(LifecycleSignal::Success, &config, json!({ "dirty": false }), &skipped);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert!(skipped.chains().is_empty() && skipped.commands().is_empty());
}

#[test]
fn the_same_command_outside_any_guard_still_fails_pre_flight() {
    let message = missing(resolve(&json!({ "success": [{ "shell": COMMIT }] })));
    assert!(message.contains("commit_message") && message.contains("success[0].shell.command"), "{message}");
}

#[test]
fn an_inline_guard_proves_its_sibling_action_like_an_explicit_then() {
    let config = resolve(&json!({ "start": [{ "when": "msg", "shell": "tool {{ msg }}" }] }))
        .expect("the inline form is the true side");
    let [site] = shell_sites(&config, LifecycleSignal::Start).try_into().expect("one site");
    assert_eq!(guarded_holes(&site), vec![("msg".to_string(), "start[0].when".to_string())]);
}

#[test]
fn an_else_body_learns_the_false_side_only() {
    resolve(&json!({
        "start": [{ "when": "!msg", "then": [{ "say": "none" }], "else": [{ "shell": "tool {{ msg }}" }] }],
    }))
    .expect("the false side of `!msg` proves `msg` present");

    missing(resolve(&json!({
        "start": [{ "when": "msg", "then": [{ "say": "some" }], "else": [{ "shell": "tool {{ msg }}" }] }],
    })));
}

#[test]
fn nested_guards_compose_to_five_levels() {
    let innermost = json!([{ "shell": "tool {{ a }} {{ e }}" }]);
    let nested = ["e", "d", "c", "b", "a"]
        .into_iter()
        .fold(innermost, |body, guard| json!([{ "when": guard, "then": body }]));

    let config = resolve(&json!({ "start": nested })).expect("every enclosing guard holds at the innermost site");

    let [site] = shell_sites(&config, LifecycleSignal::Start).try_into().expect("one site");
    let mut holes = guarded_holes(&site);
    holes.sort();
    assert_eq!(
        holes,
        vec![
            ("a".to_string(), "start[0].when".to_string()),
            ("e".to_string(), "start[0].then[0].then[0].then[0].then[0].when".to_string()),
        ]
    );
}

#[test]
fn a_guard_on_one_value_does_not_defer_another() {
    let message = missing(resolve(&json!({ "start": [{ "when": "x", "then": [{ "shell": "tool {{ x }} {{ y }}" }] }] })));
    assert!(message.contains("`y`"), "{message}");
}

#[test]
fn a_guard_in_one_event_proves_nothing_in_another() {
    missing(resolve(&json!({
        "start": [{ "when": "msg", "then": [{ "say": "{{ msg }}" }] }],
        "success": [{ "shell": "tool {{ msg }}" }],
    })));
}

#[test]
fn both_branches_are_discovered_without_evaluating_the_guard() {
    let config = resolve(&json!({
        "x": "v",
        "start": [{ "when": "x == 'never'", "then": [{ "shell": "tool {{ x }} then" }], "else": [{ "shell": "tool {{ x }} else" }] }],
    }))
    .expect("both branches resolve");

    let sites = shell_sites(&config, LifecycleSignal::Start);
    assert_eq!(sites.len(), 2);
    assert!(sites.iter().all(|site| site.resolved().is_some()), "a present value keeps its bytes in both branches");
}

#[test]
fn a_set_before_the_shell_ends_the_guard_proof() {
    // Inside the branch, after the write.
    missing(resolve(&json!({
        "start": [{ "when": "msg", "then": [{ "set": { "msg": "replaced" } }, { "shell": "tool {{ msg }}" }] }],
    })));
    // `fm.` and bare name one document property, in either direction.
    missing(resolve(&json!({
        "start": [{ "when": "msg", "then": [{ "set": { "fm.msg": "replaced" } }, { "shell": "tool {{ msg }}" }] }],
    })));
    missing(resolve(&json!({
        "start": [{ "when": "fm.msg", "then": [{ "set": { "msg": "replaced" } }, { "shell": "tool {{ fm.msg }}" }] }],
    })));
    // A write to another property leaves the proof intact.
    resolve(&json!({
        "start": [{ "when": "msg", "then": [{ "set": { "other": "x" } }, { "shell": "tool {{ msg }}" }] }],
    }))
    .expect("an unrelated write keeps the guard's proof");
}

#[test]
fn a_write_inside_an_inner_branch_invalidates_after_the_join() {
    missing(resolve(&json!({
        "start": [{
            "when": "msg",
            "then": [
                { "when": "other", "then": [{ "set": { "msg": "replaced" } }] },
                { "shell": "tool {{ msg }}" },
            ],
        }],
    })));
}

#[test]
fn a_set_value_is_checked_before_its_own_write() {
    let config = resolve(&json!({
        "start": [{ "when": "msg", "then": [{ "set": { "msg": "$(tool {{ msg }})" } }] }],
    }))
    .expect("the value is read under the guard, before the set writes `msg`");
    let [site] = shell_sites(&config, LifecycleSignal::Start).try_into().expect("one site");
    assert_eq!(site.resolved(), None);
    assert_eq!(guarded_holes(&site), vec![("msg".to_string(), "start[0].when".to_string())]);

    missing(resolve(&json!({ "start": [{ "set": { "out": "$(tool {{ msg }})" } }] })));
}

#[test]
fn a_guarded_set_value_checks_its_live_value_when_reached() {
    let frontmatter = json!({ "start": [{ "when": "msg", "then": [{ "set": { "out": "$(tool {{ msg }})" } }] }] });
    let config = resolve(&frontmatter).unwrap();
    settle_lifecycle_shell_modes(&config, &LifecycleSignal::ALL, &shape_of("tool {{ msg }}"), None);
    let shell = MockShell::new(0);

    // A non-empty list selects the branch but is no scalar argument.
    let outcome = run(LifecycleSignal::Start, &config, json!({ "msg": ["SECRET-ITEM"] }), &shell);

    let error = outcome.evaluation_error.expect("a refused value halts the event");
    assert_eq!(error.variant, "LifecycleShellTemplateRefused", "{error:?}");
    assert!(!format!("{error:?}").contains("SECRET-ITEM"), "{error:?}");
}

#[test]
fn a_selected_action_whose_value_a_set_cleared_fails_and_launches_nothing() {
    let frontmatter = json!({
        "msg": "at approval",
        "start": [{
            "when": "msg",
            "then": [
                { "shell": "first" },
                { "set": { "msg": null } },
                { "shell": { "command": "tool {{ msg }}", "no_error": true } },
            ],
        }],
    });
    let config = resolve(&frontmatter).expect("the value is present at approval");
    let mut approvals = shape_of("tool {{ msg }}");
    approvals.insert_exact("first");
    settle_lifecycle_shell_modes(&config, &LifecycleSignal::ALL, &approvals, None);
    let shell = MockShell::new(0);

    let outcome = run(LifecycleSignal::Start, &config, json!({ "msg": "live" }), &shell);

    assert!(outcome.action_error.is_none(), "`no_error` cannot absorb it: {outcome:?}");
    let error = outcome.evaluation_error.expect("the reached action fails");
    assert_eq!(error.variant, "LifecycleShellTemplateRefused", "{error:?}");
    assert!(error.msg.contains("missing"), "{}", error.msg);
    assert_eq!(shell.commands(), vec!["first"], "the earlier action's effect remains");
    assert!(shell.chains().is_empty(), "nothing of the failed action launches");
}

#[test]
fn a_value_a_set_changed_to_an_unapproved_type_is_refused_unsuppressibly() {
    let frontmatter = json!({
        "target": "at approval",
        "start": [{
            "when": "target",
            "then": [
                { "set": { "target": ["SECRET-ITEM"] } },
                { "shell": { "command": "tool {{ target }}", "no_error": true } },
            ],
        }],
    });
    let config = resolve(&frontmatter).expect("a present value resolves");
    settle_lifecycle_shell_modes(&config, &LifecycleSignal::ALL, &shape_of("tool {{ target }}"), None);
    let shell = MockShell::new(0);

    let outcome = run(LifecycleSignal::Start, &config, json!({ "target": "live" }), &shell);

    assert!(outcome.action_error.is_none(), "`no_error` cannot absorb it: {outcome:?}");
    let error = outcome.evaluation_error.expect("the changed type is refused");
    assert_eq!(error.variant, "LifecycleShellTemplateRefused", "{error:?}");
    assert!(error.msg.contains("array or object"), "{}", error.msg);
    assert!(!format!("{error:?}").contains("SECRET-ITEM"), "{error:?}");
    assert!(shell.chains().is_empty());
}

/// A guard cannot make a global the event never binds available: the
/// command is refused as the unguarded one is, never deferred as a guarded
/// missing value.
#[test]
fn a_guard_on_an_unavailable_global_leaves_its_refusal_unchanged() {
    let refusal = |stack: Value| match resolve(&json!({ "start": stack })) {
        Err(CompositionError::LifecycleShellResolution { message, .. }) => message,
        other => panic!("expected the late-binding refusal, got {other:?}"),
    };

    let guarded = refusal(json!([{ "when": "err.message", "then": [{ "shell": "tool {{ err.message }}" }] }]));
    let unguarded = refusal(json!([{ "shell": "tool {{ err.message }}" }]));

    assert!(guarded.contains("late-binding reference `err`"), "{guarded}");
    assert!(!guarded.contains("which is missing"), "{guarded}");
    assert_eq!(guarded, unguarded);
}
