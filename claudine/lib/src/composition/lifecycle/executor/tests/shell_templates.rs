//! Lifecycle `shell` actions and `set` values approved by their typed shape.

use super::*;
use crate::composition::lifecycle::shell_template::{lifecycle_declared_types, typed_template};
use darkmatter::markdown::compose::expression::TypeScope;
use crate::composition::preflight::{resolve_lifecycle_shell_commands, settle_lifecycle_shell_modes};
use darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals;

const SOURCE: &str = "t.md";

/// `frontmatter`'s lifecycle with its shell sites stamped, as pre-flight
/// leaves it, and approved by `approvals`.
fn prepared(frontmatter: &Value, approvals: Option<&ShellPreApprovals>) -> LifecycleConfig {
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
    )
    .unwrap();
    if let Some(approvals) = approvals {
        settle_lifecycle_shell_modes(&config, &LifecycleSignal::ALL, approvals, None);
    }
    config
}

/// `command` typed with no enclosing guard and no schema.
fn unguarded(command: &str) -> darkmatter::markdown::compose::shell_expansion::template::CommandTemplate {
    let declared = lifecycle_declared_types(None, crate::composition::claudine_registry());
    typed_template(command, &TypeScope::unguarded(&declared)).expect("a template")
}

fn shape_of(command: &str) -> ShellPreApprovals {
    let mut approvals = ShellPreApprovals::new();
    approvals.insert_template(TemplateShape::from_template(&unguarded(command)));
    approvals
}

fn start(config: &LifecycleConfig, state: &Map<String, Value>, shell: &dyn ShellRunner) -> LifecycleEventOutcome {
    let (_dir, engine) = temp_engine();
    let recorder = Recorder::default();
    let harness = Harness::default();
    ctx(LifecycleSignal::Start, state, None, &engine, shell, &recorder, &harness, Path::new(SOURCE))
        .execute_event(config)
}

fn args_of(shell: &MockShell) -> Vec<Vec<String>> {
    shell
        .chains()
        .into_iter()
        .flat_map(|chain| chain.segments)
        .map(|segment| std::iter::once(segment.program).chain(segment.args).collect())
        .collect()
}

#[test]
fn a_shape_approved_shell_fills_from_the_state_current_when_it_runs() {
    let frontmatter = json!({ "x": "at approval", "start": [{ "shell": "tool {{ x }} --flag" }] });
    let config = prepared(&frontmatter, Some(&shape_of("tool {{ x }} --flag")));
    let shell = MockShell::new(0);

    let outcome = start(&config, &map(json!({ "x": "a b; c" })), &shell);

    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert!(shell.commands().is_empty(), "no shell route: {:?}", shell.commands());
    assert_eq!(args_of(&shell), vec![vec!["tool", "a b; c", "--flag"]]);
}

#[test]
fn an_exact_approval_runs_the_stamped_bytes_and_never_rereads_the_value() {
    let frontmatter = json!({ "x": "plain", "start": [{ "shell": "tool {{ x }}" }] });
    let approvals: ShellPreApprovals = std::iter::once("tool plain".to_string()).collect();
    let config = prepared(&frontmatter, Some(&approvals));
    let shell = MockShell::new(0);

    let outcome = start(&config, &map(json!({ "x": "changed" })), &shell);

    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(shell.commands(), vec!["tool plain"]);
    assert!(shell.chains().is_empty());
}

#[test]
fn a_site_approved_in_neither_mode_never_runs() {
    let frontmatter = json!({ "x": "plain", "start": [{ "shell": "tool {{ x }}" }] });
    let config = prepared(&frontmatter, None);
    let shell = MockShell::new(0);

    let outcome = start(&config, &map(json!({ "x": "plain" })), &shell);

    let error = outcome.evaluation_error.expect("an unapproved site is refused");
    assert!(error.msg.contains("never approved"), "{}", error.msg);
    assert!(shell.commands().is_empty() && shell.chains().is_empty());
}

#[test]
fn a_refused_value_is_unsuppressible_and_never_shown() {
    let frontmatter = json!({
        "x": "present",
        "start": [{ "shell": { "command": "tool {{ x }}", "no_error": true } }],
    });
    let config = prepared(&frontmatter, Some(&shape_of("tool {{ x }}")));
    for (value, expected) in [(json!(null), "missing"), (json!(["SECRET-ITEM"]), "array or object")] {
        let shell = MockShell::new(0);

        let outcome = start(&config, &map(json!({ "x": value })), &shell);

        assert!(outcome.action_error.is_none(), "{outcome:?}");
        let error = outcome.evaluation_error.expect("a refused value halts the event");
        assert_eq!(error.variant, "LifecycleShellTemplateRefused", "{error:?}");
        assert!(error.msg.contains(expected), "{}", error.msg);
        assert!(!format!("{error:?}").contains("SECRET-ITEM"), "{error:?}");
        assert!(shell.chains().is_empty(), "nothing starts");
    }
}

#[test]
fn a_non_zero_exit_stays_a_dispatch_error_naming_the_authored_command() {
    let frontmatter = json!({ "x": "v", "start": [{ "shell": "tool {{ x }}" }] });
    let config = prepared(&frontmatter, Some(&shape_of("tool {{ x }}")));
    let shell = MockShell::new(3);

    let outcome = start(&config, &map(json!({ "x": "SECRET-VALUE" })), &shell);

    let error = outcome.action_error.expect("a failed command is a dispatch error");
    assert!(error.msg.contains("tool {{ x }}") && error.msg.contains("code 3"), "{}", error.msg);
    assert!(!error.msg.contains("SECRET-VALUE"), "{}", error.msg);
}

#[test]
fn the_disabled_runner_refuses_every_structured_path() {
    let frontmatter = json!({
        "x": "v",
        "start": [{ "shell": "tool {{ x }}" }, { "set": { "y": "$( tool {{ x }} )" } }],
    });
    let config = prepared(&frontmatter, Some(&shape_of("tool {{ x }}")));
    let state = map(json!({ "x": "v" }));
    let chain = darkmatter::markdown::compose::shell_expansion::template::fill_template(
        &unguarded("tool {{ x }}"),
        &|_: &str| Ok(json!("v")),
        None,
    )
    .unwrap();

    assert!(matches!(DisabledShellRunner.run_structured(&chain), Err(ShellRunError::BeforePreflight)));
    let outcome = start(&config, &state, &DisabledShellRunner);
    let error = outcome.evaluation_error.expect("a shell-free route refuses the template");
    assert!(error.msg.contains("forbidden"), "{}", error.msg);

    // The `set` alone, so the `shell` before it does not answer first.
    let set_only = json!({ "x": "v", "start": [{ "set": { "y": "$( tool {{ x }} )" } }] });
    let config = prepared(&set_only, Some(&shape_of("tool {{ x }}")));
    let outcome = start(&config, &state, &DisabledShellRunner);
    let error = outcome.evaluation_error.expect("a shell-free route refuses the template value");
    assert!(error.msg.contains("forbidden"), "{}", error.msg);
}

#[test]
fn a_template_set_value_refuses_a_missing_value_before_it_runs() {
    let frontmatter = json!({ "x": "v", "start": [{ "set": { "y": "$( tool {{ x }} )" } }] });
    let config = prepared(&frontmatter, Some(&shape_of("tool {{ x }}")));
    let shell = MockShell::new(0);

    let outcome = start(&config, &map(json!({ "x": null })), &shell);

    let error = outcome.evaluation_error.expect("a missing value fails the set");
    assert_eq!(error.variant, "LifecycleShellTemplateRefused", "{error:?}");
    assert!(error.msg.contains("missing"), "{}", error.msg);
}

#[test]
fn a_host_approved_site_fills_its_late_values_without_reading_them_again() {
    use darkmatter::markdown::compose::shell_expansion::template::{ArgPart, HoleTiming, resolve_late_values};
    let frontmatter = json!({ "start": [{ "shell": "placeholder" }] });
    let mut config = parse_lifecycle_config(&frontmatter, Path::new(SOURCE)).unwrap();
    let mut template = unguarded("tool {{ produced }} {{ x }}");
    if let ArgPart::Hole(hole) = &mut template.segments[0].args[0].parts[0] {
        hole.timing = HoleTiming::Deferred { roots: vec!["produced".to_string()] };
    }
    let late = resolve_late_values([("setup[0]", &template)], &|_: &str| Ok(json!("from the barrier")))
        .unwrap()
        .site("setup[0]")
        .cloned()
        .unwrap();
    let site = crate::composition::lifecycle::shell_template::LifecycleShellTemplate::approved(template, None)
        .with_late_values(late);
    crate::composition::lifecycle_actions::for_each_action_mut(
        config.stack_mut(LifecycleSignal::Start),
        &mut |action| {
            if let LifecycleActionKind::Shell(shell) = &mut action.kind {
                shell.template = Some(site.clone());
            }
        },
    );
    let shell = MockShell::new(0);

    // `produced` is absent now: reading it again would be a missing value.
    let outcome = start(&config, &map(json!({ "x": "now" })), &shell);

    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(args_of(&shell), vec![vec!["tool", "from the barrier", "now"]]);
}
