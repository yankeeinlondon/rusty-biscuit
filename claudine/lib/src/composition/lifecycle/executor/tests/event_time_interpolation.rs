//! event time interpolation executor tests.

use super::*;

#[test]
fn message_interpolates_frontmatter_in_literal() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"action": "info", "message": "done {{ name }}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"name": "alpha"}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(recorder.events(), vec![Emitted::Info("done alpha".to_string())]);
}

fn io_err(msg: &str) -> LifecycleErrorInfo {
    LifecycleErrorInfo {
        kind: "ClaudineError",
        variant: "Io".to_string(),
        msg: msg.to_string(),
        snapshot: None,
    }
}

/// Top-level `failure.message: "{{err.msg}}"` is a deferred (raw) key that
/// must interpolate the real error at event-time — the original bug.
#[test]
fn top_level_message_interpolates_err_at_event_time() {
    let config = parse_lifecycle_config(
        &json!({"failure": {"message": "❌️ {{err.msg}}"}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let err = io_err("disk full");
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        Some(&err),
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(
        recorder.events(),
        vec![Emitted::Message("❌️ disk full".to_string())]
    );
}

/// A `failure` stack `message(❌️ {{err.msg}})` renders the real error
/// end-to-end through composition (parse → executor → DM2).
#[test]
fn stack_message_interpolates_err_at_event_time() {
    let config = parse_lifecycle_config(
        &json!({"failure": {"stack": [{"action": {"message": "❌️ {{err.msg}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let err = io_err("disk full");
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        Some(&err),
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(
        recorder.events(),
        vec![Emitted::Message("❌️ disk full".to_string())]
    );
}

/// A mixed body resolves both an early-binding frontmatter span (`phase`)
/// and a late-binding global span (`err.msg`) at event-time.
#[test]
fn mixed_body_resolves_both_spans_at_event_time() {
    let config = parse_lifecycle_config(
        &json!({"failure": {"stack": [
            {"action": {"message": "phase {{phase}} failed: {{err.msg}}"}}
        ]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"phase": 6}));
    let err = io_err("disk full");
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        Some(&err),
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(
        recorder.events(),
        vec![Emitted::Message("phase 6 failed: disk full".to_string())]
    );
}

/// Currentness: the same lifecycle config re-resolves `{{phase}}` against
/// each event's live frontmatter, so a loop message reflects the current
/// iteration's value (the raw deferred subtree stays the stored definition).
#[test]
fn message_reflects_current_frontmatter_per_event() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"message": "iter {{phase}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let harness = Harness::default();
    for (phase, expected) in [(1u64, "iter 1"), (2u64, "iter 2")] {
        let fm = map(json!({ "phase": phase }));
        let recorder = Recorder::default();
        let context = ctx(
            LifecycleSignal::Success,
            &fm,
            None,
            &engine,
            &shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        );
        context.execute_event(&config);
        assert_eq!(recorder.events(), vec![Emitted::Message(expected.to_string())]);
    }
}

/// Event-time rendering stays identical to Darkmatter subtree composition;
/// Claudine does not introduce a second interpolation engine.
#[test]
fn event_time_rendering_matches_compose() {
    use darkmatter::markdown::compose::EffectiveStateBuilder;
    use darkmatter::markdown::compose::subtree::{SubtreeStrictness, compose_subtree};

    let template = "phase {{phase}}: {{err.msg}}";
    let err = io_err("disk full");

    // Executor path.
    let config = parse_lifecycle_config(
        &json!({"failure": {"stack": [{"action": {"message": template}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"phase": 6}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        Some(&err),
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    let Emitted::Message(executor_text) = &recorder.events()[0] else {
        panic!("expected a Message emission");
    };

    // Direct DM2 subtree compose for the same string + data.
    let state = EffectiveStateBuilder::new()
        .with_frontmatter(
            [("phase".to_string(), json!(6))].into_iter().collect(),
        )
        .with_context(
            darkmatter::markdown::compose::ComposeContext::capture_for_content(
                Path::new("."),
                "",
            ),
        )
        .build()
        .unwrap();
    let compose_value = compose_subtree(
        &json!(template),
        &state,
        lifecycle_injected_globals(Some(&err), None),
        SubtreeStrictness::Lenient,
    )
    .unwrap();
    assert_eq!(executor_text, compose_value.as_str().unwrap());
    assert_eq!(executor_text, "phase 6: disk full");
}

/// Phase 7 reproduction fixture (acceptance criterion 1): a top-level
/// `failure` block shaped like `prompts/implement-plan.md` — both a `say`
/// and a `message` field mixing an early-binding frontmatter span
/// (`{{phase}}`) with the late-binding `err` global — renders the real
/// values when the failure event fires. This is the original bug: before
/// late binding, `{{err.msg}}` collapsed to empty at compose time.
#[test]
fn reproduction_failure_block_renders_real_error_at_event_time() {
    let config = parse_lifecycle_config(
        &json!({"failure": {
            "say": "Phase {{phase}} ran into problems!",
            "message": "❌️ phase {{phase}} failed: {{err.msg}}",
        }}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"phase": 6}));
    let err = io_err("disk full");
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        Some(&err),
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(
        recorder.events(),
        vec![
            Emitted::Message("❌️ phase 6 failed: disk full".to_string()),
            Emitted::Speech("Phase 6 ran into problems!".to_string()),
        ]
    );
}

// ── Phase 5 (C4): fail-closed event-time resolution ─────────────────

/// A reference whose root is a *known* frontmatter key that resolves to
/// `null`/empty renders empty and does **not** error (5.6).
#[test]
fn known_but_empty_reference_renders_empty() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"message": "spec={{spec_file}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"spec_file": null}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(recorder.events(), vec![Emitted::Message("spec=".to_string())]);
}

/// A typo (an unknown root) fails closed: the action errors and nothing is
/// dispatched (5.6).
#[test]
fn unknown_root_typo_fails_closed() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"message": "{{spec_fil}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"spec_file": "x"}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert!(
        outcome.evaluation_error.is_some(),
        "typo must fail closed through the evaluation channel"
    );
    assert!(outcome.action_error.is_none());
    assert!(recorder.events().is_empty(), "nothing dispatched");
}

/// A top-level field with an unknown root fails the event closed before any
/// side effect is dispatched (5.5).
#[test]
fn top_level_unknown_root_fails_event_closed() {
    let config = parse_lifecycle_config(
        &json!({"success": {"message": "{{spec_fil}}"}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert!(
        outcome.evaluation_error.is_some(),
        "a top-level interpolation raise is an evaluation error"
    );
    assert!(outcome.action_error.is_none());
    assert!(recorder.events().is_empty());
}

/// Post-DM2 leak guard (5.4): a known reference whose resolved value is
/// itself raw template text leaves a surviving `{{ … }}` span, which fails
/// before dispatch.
#[test]
fn post_dm2_surviving_span_fails_before_dispatch() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"message": "{{tmpl}}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    // The frontmatter value is literal template text — resolving `{{tmpl}}`
    // yields `{{x}}`, a surviving recognized span.
    let fm = map(json!({"tmpl": "{{x}}"}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert!(
        outcome.evaluation_error.is_some(),
        "surviving span is an evaluation-layer failure"
    );
    assert!(outcome.action_error.is_none());
    assert!(recorder.events().is_empty(), "no side effect dispatched");
}


// ── the expression layer's typed transport (spec §D1/§D4/§D10) ──────────────

/// An interpolation that raises inside Darkmatter reaches the snapshot as the
/// typed `LifecycleExprError`, and the projection leaves the `err.*` matching
/// surface exactly where it was.
///
/// This is the property that makes retyping the executor's expression layer
/// behavior-neutral: the Darkmatter cause carries no *Claudine* facets, so
/// `select_effective_diagnostic` finds nothing to classify with and
/// `err.kind` / `err.variant` keep their action-failure spellings instead of
/// silently flipping to `category` / `code` aliases and breaking authored rules.
#[test]
fn evaluation_raise_projects_typed_cause_without_moving_err_aliases() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"message": "{{ no_such_function() }}"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    let info = outcome
        .evaluation_error
        .expect("an unknown function is an expression-layer raise");

    assert_eq!(info.kind, "LifecycleAction");
    assert_eq!(info.variant, "message");
    assert!(
        info.snapshot.is_none(),
        "a Darkmatter cause supplies no Claudine facets, so none project (and \
         with no snapshot there is no `err.cause` either)"
    );
    assert!(recorder.events().is_empty(), "nothing dispatched");

    // `err.msg` stays notification-safe: single line, escape-free, non-empty,
    // and inside the ~240-char cap the TTS/webhook routes rely on.
    assert!(!info.msg.is_empty());
    assert!(!info.msg.contains('\n'));
    assert!(!info.msg.contains('\u{1b}'));
    assert!(info.msg.chars().count() <= 240, "msg: {}", info.msg);
}

/// The `Prose` arm carries a failure the expression layer describes itself, and
/// round-trips its text unchanged through `Display` — the leak guard's message
/// is a user surface.
#[test]
fn lifecycle_expr_error_prose_arm_round_trips_its_text() {
    let error = LifecycleExprError::Prose("references undefined variable `x`".to_string());
    assert_eq!(error.to_string(), "references undefined variable `x`");
    assert!(std::error::Error::source(&error).is_none());
}

/// A refresh capability scripted with one answer, shared by the lazy-root
/// tests below.
#[derive(Debug)]
struct OneKeyRefresh {
    key: &'static str,
    value: std::sync::Mutex<String>,
}

impl OneKeyRefresh {
    fn new(key: &'static str, value: &str) -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            key,
            value: std::sync::Mutex::new(value.to_string()),
        })
    }

    fn set(&self, value: &str) {
        *self.value.lock().expect("scripted value mutex") = value.to_string();
    }

    fn authority(self: &std::sync::Arc<Self>) -> darkmatter::markdown::compose::CurrentAuthority {
        darkmatter::markdown::compose::CurrentAuthority::default().with_provider(self.clone())
    }
}

impl darkmatter::markdown::compose::CurrentProvider for OneKeyRefresh {
    fn refresh(&self, key: &str) -> darkmatter::markdown::compose::CurrentRefresh {
        use darkmatter::markdown::compose::CurrentRefresh;
        if key == self.key {
            return CurrentRefresh::Observed(Value::String(
                self.value.lock().expect("scripted value mutex").clone(),
            ));
        }
        CurrentRefresh::Unsupported
    }
}

/// `current.*` resolves in **every** lifecycle event, not just the ones that
/// happen to carry an error or a timing snapshot.
///
/// Each event builds its own effective state, so a regression that wired the
/// refresh authority into one route would leave the rest silently empty —
/// which renders as a blank operational message rather than a failure.
#[test]
fn current_resolves_in_every_lifecycle_event() {
    let provider = OneKeyRefresh::new("branch", "main");
    let authority = provider.authority();

    for signal in LifecycleSignal::ALL {
        let event_key = match signal {
            LifecycleSignal::Initialize => "initialize",
            LifecycleSignal::Start => "start",
            LifecycleSignal::Success => "success",
            LifecycleSignal::Blocked => "blocked",
            LifecycleSignal::Failure => "failure",
            LifecycleSignal::Finalize => "finalize",
            LifecycleSignal::Loop => "loop",
        };
        let config = parse_lifecycle_config(
            &json!({
                event_key: {
                    "stack": [{"action": {"action": "info", "message": "on {{ current.branch }}"}}]
                }
            }),
            Path::new("t.md"),
        )
        .unwrap();
        let fm = map(json!({}));
        let (_dir, engine) = temp_engine();
        let shell = MockShell::new(0);
        let recorder = Recorder::default();
        let harness = Harness::default();
        let context = StackExecutionContext {
            current: Some(authority.clone()),
            ..ctx(
                signal,
                &fm,
                None,
                &engine,
                &shell,
                &recorder,
                &harness,
                Path::new("t.md"),
            )
        };
        context.execute_event(&config);
        assert_eq!(
            recorder.events(),
            vec![Emitted::Info("on main".to_string())],
            "`current.branch` must resolve in the `{event_key}` event"
        );
    }
}

/// A later event observes the fact as it stands then.
///
/// The memo lives inside one expression evaluation (Q2), and every event
/// builds a fresh state, so a branch that moved between `start` and `success`
/// is reported by `success` rather than replayed from `start`.
#[test]
fn a_later_event_observes_a_fact_that_changed_since_the_earlier_one() {
    let provider = OneKeyRefresh::new("branch", "main");
    let authority = provider.authority();
    let config = parse_lifecycle_config(
        &json!({
            "start": {"stack": [{"action": {"action": "info", "message": "{{ current.branch }}"}}]},
            "success": {"stack": [{"action": {"action": "info", "message": "{{ current.branch }}"}}]}
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();

    let event = |signal| StackExecutionContext {
        current: Some(authority.clone()),
        ..ctx(
            signal,
            &fm,
            None,
            &engine,
            &shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        )
    };

    event(LifecycleSignal::Start).execute_event(&config);
    provider.set("feat/x");
    event(LifecycleSignal::Success).execute_event(&config);

    assert_eq!(
        recorder.events(),
        vec![
            Emitted::Info("main".to_string()),
            Emitted::Info("feat/x".to_string()),
        ],
        "each event observes the branch as it stood when that event fired"
    );
}

/// Fail-closed at the dispatch boundary: an event whose invocation holds no
/// capability emits an empty value rather than a host observation.
#[test]
fn an_event_without_a_refresh_capability_emits_empty_rather_than_probing() {
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"action": "info", "message": "host=[{{ current.hostname }}]"}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(
        recorder.events(),
        vec![Emitted::Info("host=[]".to_string())],
        "no capability means `null`, never this host's real name"
    );
}

/// `current_env.<KEY>` rereads the live process environment at the moment the
/// event reaches it — the contract that replaced the removed `current.env.*`.
#[test]
#[serial_test::serial(env_lifecycle_current)]
fn current_env_rereads_the_process_environment_at_event_time() {
    let key = "CLAUDINE_TEST_EVENT_TIME_CURRENT_ENV";
    let config = parse_lifecycle_config(
        &json!({"success": {"stack": [{"action": {"action": "info", "message": format!("v={{{{ current_env.{key} }}}}")}}]}}),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let event = || {
        ctx(
            LifecycleSignal::Success,
            &fm,
            None,
            &engine,
            &shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        )
        .execute_event(&config)
    };

    // SAFETY: serialized via #[serial]; no other thread reads this var.
    unsafe { std::env::set_var(key, "before") };
    event();
    unsafe { std::env::set_var(key, "after") };
    event();
    unsafe { std::env::remove_var(key) };

    assert_eq!(
        recorder.events(),
        vec![
            Emitted::Info("v=before".to_string()),
            Emitted::Info("v=after".to_string()),
        ],
        "a parent-process environment change between events is observable"
    );
}
