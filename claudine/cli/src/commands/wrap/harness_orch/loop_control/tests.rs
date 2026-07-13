//! Tests for harness-loop control and requeue fallback behavior.

use super::*;

mod terminal_event_tests {
    use super::*;
    use claudine::composition::{
        LifecycleConfig, LifecycleEmitter, LifecycleRunGuard, LifecycleRuntimeContext,
        parse_lifecycle_config,
    };
    use claudine::events::GlobalSettings;
    use claudine::messaging::RuntimeMessagingSettings;
    use std::sync::Mutex;

    /// The harness-loop wiring captures non-empty `timing`/`current` globals so
    /// terminal events expose `timing.document_ms`/`timing.total_ms` and a
    /// populated `current.env` — the regression this feature closes (previously
    /// every site hardcoded `timing: None, current: None`).
    #[test]
    fn capture_lifecycle_globals_populates_timing_and_current() {
        let loop_start = std::time::Instant::now();
        let (timing, current) =
            capture_lifecycle_globals(Path::new("prompt.md"), Some(Path::new(".")), None, loop_start);

        assert!(timing.document_ms.is_some(), "document_ms is populated");
        assert!(timing.total_ms.is_some(), "total_ms is populated");
        assert!(
            current.env.is_object() && !current.env.as_object().unwrap().is_empty(),
            "current.env is a non-empty environment snapshot"
        );
    }

    /// The injected globals the harness-loop builder attaches resolve
    /// `current.env.*` and `timing.document_ms` through Darkmatter's layered
    /// lookup (DM2) — proving the wiring reaches expression evaluation, not just
    /// the struct fields.
    #[test]
    #[serial_test::serial(env_loop_control_current)]
    fn attached_globals_resolve_through_lookup() {
        use claudine::composition::lifecycle_injected_globals;
        use darkmatter::markdown::compose::expression::{
            EvaluationLookup, evaluate, is_truthy, parse,
        };
        use darkmatter::markdown::compose::subtree::LayeredLookup;
        use darkmatter::markdown::compose::{ComposeContext, EffectiveStateBuilder};

        let key = "CLAUDINE_TEST_LOOP_CONTROL_LATE_BIND";
        // SAFETY: serialized via #[serial]; no other thread reads this var.
        unsafe { std::env::set_var(key, "ready") };
        let (timing, current) =
            capture_lifecycle_globals(
                Path::new("prompt.md"),
                Some(Path::new(".")),
                None,
                loop_start_now(),
            );
        unsafe { std::env::remove_var(key) };

        let state = EffectiveStateBuilder::new()
            .with_context(ComposeContext::capture_for_content(Path::new("."), ""))
            .build()
            .unwrap();
        let globals = lifecycle_injected_globals(None, Some(&timing), Some(&current));
        let lookup = LayeredLookup::new(&state, &globals, None);

        let when = parse(&format!("current.env.{key} == 'ready'")).expect("parses");
        assert!(
            is_truthy(&evaluate(&when, &lookup).expect("evaluates")),
            "the late-bound env value resolves through the attached current global"
        );
        assert!(
            lookup.get("timing.document_ms").is_some(),
            "timing.document_ms resolves through the attached timing global"
        );
    }

    fn loop_start_now() -> std::time::Instant {
        std::time::Instant::now()
    }

    /// One emitted top-level communication, recorded by [`RecordingEmitter`].
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Emitted {
        Stderr(LifecycleSignal, String),
        Message(String),
        Speech(String),
    }

    /// Lifecycle emitter test double that records every emission.
    #[derive(Default)]
    struct RecordingEmitter {
        events: Mutex<Vec<Emitted>>,
    }

    impl RecordingEmitter {
        fn events(&self) -> Vec<Emitted> {
            self.events.lock().unwrap().clone()
        }
    }

    impl LifecycleEmitter for RecordingEmitter {
        fn emit_stderr(&self, signal: LifecycleSignal, text: &str, _term: &Terminal) {
            self.events
                .lock()
                .unwrap()
                .push(Emitted::Stderr(signal, text.to_string()));
        }
        fn emit_message(
            &self,
            text: &str,
            _source_path: &Path,
            _repo_root: Option<&Path>,
            _messaging: &RuntimeMessagingSettings,
        ) {
            self.events
                .lock()
                .unwrap()
                .push(Emitted::Message(text.to_string()));
        }
        fn emit_speech(&self, text: &str, _config: biscuit_speaks::TtsConfig) {
            self.events
                .lock()
                .unwrap()
                .push(Emitted::Speech(text.to_string()));
        }
        fn emit_effect(&self, _name: &str) {}
        fn emit_notification(&self, _title: &str) {}
    }

    fn materialized(frontmatter: serde_json::Value) -> MaterializedHarnessPrompt {
        let live_frontmatter = MaterializedHarnessPrompt::live_cell_from(&frontmatter);
        MaterializedHarnessPrompt {
            frontmatter,
            prompt: String::new(),
            env_overrides: Vec::new(),
            inline_closure_plan: None,
            live_frontmatter,
        }
    }

    /// Number of lines a stack's `append_line` side effect wrote — i.e. the
    /// number of times the stack actually executed its side effects.
    fn line_count(path: &Path) -> usize {
        std::fs::read_to_string(path)
            .map(|s| s.lines().count())
            .unwrap_or(0)
    }

    struct Fixture {
        _dir: tempfile::TempDir,
        log_path: PathBuf,
        config: LifecycleConfig,
        settings: GlobalSettings,
        messaging: RuntimeMessagingSettings,
        term: Terminal,
        source_path: PathBuf,
        materialized: MaterializedHarnessPrompt,
    }

    use std::path::PathBuf;

    /// Build a fixture whose `success` and `blocked` stacks each append one
    /// line to `events.log` (a side-effect counter) and carry a top-level
    /// `stderr` communication. When `with_error` is set, the named event's
    /// stack ends in `{error: "downgraded"}` so it routes to `failure`.
    fn fixture(frontmatter: serde_json::Value) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let source_path = dir.path().join("prompt.md");
        let log_path = dir.path().join("events.log");
        let config = parse_lifecycle_config(&frontmatter, &source_path).unwrap();
        Fixture {
            _dir: dir,
            log_path,
            config,
            settings: GlobalSettings::default(),
            messaging: RuntimeMessagingSettings {
                user: None,
                repo: None,
            },
            term: Terminal::default(),
            source_path,
            materialized: materialized(frontmatter),
        }
    }

    fn engine(root: &Path) -> EffectEngine {
        EffectEngine::builder()
            .mutation_root(root)
            .auto_rehash(false)
            .build()
    }

    #[test]
    fn success_stack_side_effects_run_exactly_once() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stderr": "succeeded",
                "stack": [{"action": {"append_line": ["events.log", "ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let TerminalEventOutcome { outcome, .. } = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        assert!(outcome.control.is_none());
        // The stack's side effect fired exactly once (was twice before the fix).
        assert_eq!(line_count(&fx.log_path), 1, "stack ran exactly once");
        // Top-level success communication fired (the stack stayed success).
        assert_eq!(
            emitter.events(),
            vec![Emitted::Stderr(LifecycleSignal::Success, "succeeded".to_string())]
        );
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Success));
    }

    /// Phase 2: a terminal-phase `success.when` evaluation error is now filed in
    /// the dedicated evaluation-error channel, not the tolerated `action_error`.
    ///
    /// The first stack item's `when:` references an undefined frontmatter root,
    /// so it *raises* at event time (it does not evaluate cleanly to `false`).
    /// The spec (Decision #1) requires distinguishing such an **evaluation**
    /// error from a side-effect **dispatch** failure: the former must halt the
    /// run, the latter keeps today's log-and-continue policy.
    ///
    /// This asserts only the executor-level classification (Phase 2): the raise
    /// surfaces through `evaluation_error` and never lands in `action_error`. The
    /// orchestration that turns this into a `finalize`-with-`err` + non-zero
    /// outcome is wired in Phase 3.
    #[test]
    fn success_when_evaluation_error_is_not_swallowed_as_action_error() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stack": [
                    {"when": "missing_root == true", "action": {"stderr": "ready"}}
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let TerminalEventOutcome { outcome, .. } = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        // The `when:` raised, so the stack never reached an action: nothing was
        // emitted and no control action fired.
        assert!(outcome.control.is_none(), "no control action ran");
        assert!(emitter.events().is_empty(), "the guarded action never ran");
        // The raise is filed in the dedicated evaluation-error channel, not the
        // tolerated dispatch-failure `action_error` channel the success path
        // would otherwise drop.
        assert!(
            outcome.action_error.is_none(),
            "a terminal-phase `when:` evaluation error must not be filed as an `action_error`"
        );
        assert!(
            outcome.evaluation_error.is_some(),
            "the `when:` raise surfaces through the halting evaluation-error channel"
        );
    }

    /// Phase 3: a terminal-phase `success` evaluation error halts the run.
    ///
    /// `handle_terminal_evaluation_error` runs `finalize` exactly once with the
    /// evaluation error exposed as `err` (so a `when: "err"` finalize branch
    /// fires) and returns the typed `LifecycleEvaluationError`. It does **not**
    /// fire `failure` (Decision #3): the provider already succeeded.
    #[test]
    fn success_evaluation_error_runs_finalize_with_err_and_returns_failure() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stack": [{"when": "missing_root == true", "action": {"stderr": "ready"}}]
            },
            "finalize": {
                "stack": [{"when": "err", "action": {"append_line": ["events.log", "finalized-with-err"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let success = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );
        assert!(
            success.outcome.evaluation_error.is_some(),
            "the success `when:` raised"
        );

        let err = handle_terminal_evaluation_error(
            &success.outcome,
            "success",
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        )
        .expect("a terminal evaluation error produces a run failure");

        let rendered = err.to_string();
        assert!(
            rendered.contains("`success`") && rendered.contains("evaluation error"),
            "the run failure names the event and is a typed evaluation error: {rendered}"
        );
        // `finalize` fired exactly once and saw `err` populated (its `when: err`
        // branch ran), proving the error was threaded into the finalize context.
        assert!(guard.finalize_emitted(), "finalize fired");
        assert_eq!(
            line_count(&fx.log_path),
            1,
            "finalize ran once with `err` available"
        );
        // The provider succeeded, so `failure` was NOT fired (Decision #3).
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Success));
    }

    /// A `success` stack that downgrades to `failure` via explicit `error()`,
    /// where the resulting `failure` stack then raises an evaluation error,
    /// must surface the error attributed to `failure` — not `success`. After a
    /// downgrade, `outcome` holds the failure event's result, so
    /// `effective_event` must be `"failure"`; without it, the success caller
    /// would hardcode `"success"` and the diagnostic would point at the wrong
    /// event.
    #[test]
    fn downgraded_success_failure_raise_reports_failure_event() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stack": [{"action": {"error": "downgraded"}}]
            },
            "failure": {
                "stack": [{"when": "missing_root == true", "action": {"stderr": "unreachable"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let success = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        // The success stack downgraded via `error()`, and the failure stack
        // raised an evaluation error in its `when:`.
        assert!(
            success.outcome.evaluation_error.is_some(),
            "the downgraded failure stack raised"
        );
        assert_eq!(
            success.effective_event, "failure",
            "effective_event must be `failure` after a downgrade"
        );

        let err = handle_terminal_evaluation_error(
            &success.outcome,
            success.effective_event,
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        )
        .expect("the failure evaluation error halts the run");

        let rendered = err.to_string();
        assert!(
            rendered.contains("`failure`"),
            "the error must name the failure event, not success; got: {rendered}"
        );
        assert!(
            !rendered.contains("`success`"),
            "the error must NOT name the success event; got: {rendered}"
        );
    }

    /// Regression guard: a `success` stack whose own `when:` raises (no
    /// downgrade) keeps `effective_event == "success"`. Confirms the fix did
    /// not break the non-downgrading path.
    #[test]
    fn success_evaluation_error_non_downgrading_reports_success_event() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stack": [{"when": "missing_root == true", "action": {"stderr": "unreachable"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let success = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        assert!(
            success.outcome.evaluation_error.is_some(),
            "the success `when:` raised"
        );
        assert_eq!(
            success.effective_event, "success",
            "effective_event stays `success` when no downgrade occurred"
        );
    }

    /// Phase 3: a terminal-phase side-effect **dispatch** failure is NOT
    /// escalated — `handle_terminal_evaluation_error` returns `None` and runs no
    /// `finalize`, so the caller keeps today's log-and-continue policy.
    #[test]
    fn terminal_dispatch_failure_keeps_previous_outcome() {
        let fx = fixture(serde_json::json!({
            "finalize": {
                "stack": [{"action": {"append_line": ["events.log", "finalized"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        assert!(guard.record_event_emission(LifecycleSignal::Success));
        let eng = engine(fx._dir.path());

        // A dispatch failure populates `action_error`, never `evaluation_error`.
        let outcome = LifecycleEventOutcome {
            action_error: Some(LifecycleErrorInfo::from_action_failure("shell", "boom")),
            ..Default::default()
        };
        let halted = handle_terminal_evaluation_error(
            &outcome,
            "success",
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        );
        assert!(halted.is_none(), "a dispatch failure does not halt the run");
        assert!(!guard.finalize_emitted(), "no finalize was forced");
        assert_eq!(line_count(&fx.log_path), 0, "the finalize stack did not run");
    }

    /// Behavior-matrix counterpart to the `success.when` raise: a terminal-phase
    /// `when:` that evaluates **cleanly to `false`** just skips its item — no
    /// evaluation error, no halt. This is the crashed-vs-clean-false distinction
    /// at the orchestration layer: a clean `false` guard must never be confused
    /// with a swallowed raise.
    #[test]
    fn terminal_clean_false_guard_skips_without_halting() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stack": [{"when": "1 == 2", "action": {"append_line": ["events.log", "ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let success = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        // A clean `false` guard raises nothing and dispatches nothing.
        assert!(
            success.outcome.evaluation_error.is_none(),
            "a clean `false` guard must not file an evaluation error"
        );
        assert!(success.outcome.action_error.is_none());
        assert_eq!(line_count(&fx.log_path), 0, "the guarded action was skipped");

        // The clean false therefore does not halt the run.
        let halted = handle_terminal_evaluation_error(
            &success.outcome,
            "success",
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        );
        assert!(halted.is_none(), "a clean false guard does not halt the run");
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Success));
    }

    /// Phase 3: a setup-phase evaluation error routes through `failure` then
    /// `finalize` (Decision #5), threading the error as `err`, and returns the
    /// typed run failure.
    #[test]
    fn setup_evaluation_error_routes_through_failure_and_finalize() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stack": [{"when": "err", "action": {"append_line": ["events.log", "failure-ran"]}}]
            },
            "finalize": {
                "stack": [{"when": "err", "action": {"append_line": ["events.log", "finalize-ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());

        // Model a `start` stack that raised at event time (no terminal recorded).
        let outcome = LifecycleEventOutcome {
            evaluation_error: Some(LifecycleErrorInfo::from_action_failure(
                "when",
                "`when:` references undefined variable `missing_root`",
            )),
            ..Default::default()
        };
        let err = handle_setup_evaluation_error(
            &outcome,
            "start",
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        )
        .expect("a setup evaluation error produces a run failure");

        assert!(err.to_string().contains("`start`"), "names the start event");
        // Both `failure` and `finalize` ran, each seeing `err` populated.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec!["failure-ran", "finalize-ran"],
            "setup evaluation error routes through failure then finalize, both with `err`"
        );
        assert!(guard.finalize_emitted(), "finalize fired");
    }

    /// Phase 3: an evaluation error raised *inside* `finalize` aborts the run
    /// without re-entering `finalize` (the re-entry guard). `finalize` fires
    /// exactly once and the recovery path returns `Abort` with the typed error.
    #[test]
    fn finalize_evaluation_error_aborts_without_reentry() {
        let fx = fixture(serde_json::json!({
            "finalize": {
                "stack": [{"when": "missing_root == true", "action": {"stderr": "x"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        // Model the live call site: a terminal already fired this iteration.
        assert!(guard.record_event_emission(LifecycleSignal::Failure));
        let eng = engine(fx._dir.path());
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        let mut proxy = ProxyTracking::default();

        let action = run_finalize_with_recovery(
            &mut guard,
            &fx.materialized,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
            1,
            &mut budgets,
            Some("sess-1"),
            resume_capable_profile(),
            Provider::Claude,
            &mut state,
            &mut proxy,
            false,
        );

        match action {
            TerminalControlAction::Abort(err) => {
                assert!(
                    err.to_string().contains("`finalize`"),
                    "the abort names the finalize event: {err}"
                );
            }
            other => panic!("expected Abort from a finalize evaluation error, got {other:?}"),
        }
        // `finalize` ran exactly once — the abort did not loop back into it.
        assert!(guard.finalize_emitted(), "finalize fired exactly once");
    }

    #[test]
    fn success_stack_error_routes_to_failure_keeps_success_comm_before_failure() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stderr": "succeeded",
                "stack": [{"action": [{"append_line": ["events.log", "ran"]}, {"error": "downgraded"}]}]
            },
            "failure": {
                "stderr": "failed",
                "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let TerminalEventOutcome { outcome, .. } = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        // Outcome reflects the failure event's run (no Error control surviving).
        assert!(outcome.control.is_none());
        // Success stack ran once (append + error), failure stack ran once.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec!["ran", "failure-ran"],
            "success stack and failure stack each ran exactly once"
        );
        // The success top-level comm fired FIRST (top-level-before-stack), then
        // the downgrade fired the failure top-level comm. The success comm is
        // NOT suppressed — the spec requires top-level to fire before stack
        // processing, so a later `error()` cannot un-fire it.
        assert_eq!(
            emitter.events(),
            vec![
                Emitted::Stderr(LifecycleSignal::Success, "succeeded".to_string()),
                Emitted::Stderr(LifecycleSignal::Failure, "failed".to_string()),
            ]
        );
        // Guard recorded the downgraded terminal signal as Failure.
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    }

    #[test]
    fn blocked_stack_side_effects_run_exactly_once() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stderr": "blocked",
                "stack": [{"action": {"append_line": ["events.log", "ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());

        let TerminalEventOutcome { outcome, .. } = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Blocked,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        assert!(outcome.control.is_none());
        assert_eq!(line_count(&fx.log_path), 1, "blocked stack ran exactly once");
        assert_eq!(
            emitter.events(),
            vec![Emitted::Stderr(LifecycleSignal::Blocked, "blocked".to_string())]
        );
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Blocked));
    }

    /// Top-level communication for `success` fires before any `stack:`
    /// communication in the same event.
    #[test]
    fn success_top_level_communication_fires_before_stack_communication() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stderr": "success-top",
                "stack": [{"action": {"stderr": "success-stack"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let TerminalEventOutcome { outcome, .. } = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        assert!(outcome.control.is_none());
        assert_eq!(
            emitter.events(),
            vec![
                Emitted::Stderr(LifecycleSignal::Success, "success-top".to_string()),
                Emitted::Stderr(LifecycleSignal::Success, "success-stack".to_string()),
            ],
            "top-level communication must fire before stack communication"
        );
    }

    /// Top-level communication for `blocked` fires before any `stack:`
    /// communication in the same event.
    #[test]
    fn blocked_top_level_communication_fires_before_stack_communication() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stderr": "blocked-top",
                "stack": [{"action": {"stderr": "blocked-stack"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());

        let TerminalEventOutcome { outcome, .. } = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Blocked,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        assert!(outcome.control.is_none());
        assert_eq!(
            emitter.events(),
            vec![
                Emitted::Stderr(LifecycleSignal::Blocked, "blocked-top".to_string()),
                Emitted::Stderr(LifecycleSignal::Blocked, "blocked-stack".to_string()),
            ],
            "top-level communication must fire before stack communication"
        );
    }

    /// Reproduces the exact guard call sequence of the `run_harness_loop`
    /// `routes_to_failure(Start)` branch: a `start` stack action errored, so
    /// the failure path records `Failure`, runs the error-carrying failure
    /// stack, and then must reach `finalize`. Asserts the failure AND finalize
    /// stacks each ran exactly once and `finalize_emitted()` is true — proving
    /// `finalize` is not skipped (the Finding 2 defect).
    #[test]
    fn start_stack_action_error_records_failure_then_runs_finalize() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stderr": "failed",
                "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
            },
            "finalize": {
                "stderr": "finalized",
                "stack": [{"action": {"append_line": ["events.log", "finalize-ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // The provider never launched (a setup-phase `start` error routes here),
        // but `start` was emitted — mirror the loop's pre-launch state.
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        // --- The fixed `routes_to_failure(Start)` branch sequence ---
        let action_error = LifecycleErrorInfo::from_action_failure("shell", "boom");
        // 1. Record `Failure` FIRST (the fix). This sets `terminal_emitted`.
        assert!(guard.record_event_emission(LifecycleSignal::Failure));
        // 2. Run the error-carrying failure stack via `run_event_stack`.
        let failure_ctx = build_lifecycle_stack_context_for_materialized(
            LifecycleSignal::Failure,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            None,
            None,
            &fx.term,
            guard.emitter(),
            guard.context().settings,
            guard.context().messaging,
            &eng,
            Some(&action_error),
            None,
            None,
        );
        guard.run_event_stack(LifecycleSignal::Failure, &failure_ctx);
        // 3. Finalize must now fire (records + runs because terminal_emitted).
        run_lifecycle_event(
            &mut guard,
            LifecycleSignal::Finalize,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            Some(&action_error),
            std::time::Instant::now(),
        );

        // Finalize was NOT skipped.
        assert!(
            guard.finalize_emitted(),
            "finalize must fire after a setup-phase failure"
        );
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        // Both the failure stack and the finalize stack ran exactly once.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec!["failure-ran", "finalize-ran"],
            "failure stack and finalize stack each ran exactly once"
        );
        // Both top-level comms fired, failure before finalize.
        assert_eq!(
            emitter.events(),
            vec![
                Emitted::Stderr(LifecycleSignal::Failure, "failed".to_string()),
                Emitted::Stderr(LifecycleSignal::Finalize, "finalized".to_string()),
            ]
        );
    }

    /// Locks in WHY the fix is needed: calling `run_event_stack(Failure, ...)`
    /// WITHOUT first `record_event_emission(Failure)` leaves `terminal_emitted`
    /// false, so a subsequent `Finalize` is a no-op (the finalize stack never
    /// runs). This documents the Finding 2 defect the fix removes.
    #[test]
    fn failure_stack_without_record_skips_finalize() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
            },
            "finalize": {
                "stack": [{"action": {"append_line": ["events.log", "finalize-ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        // Defective sequence: run the failure stack directly, no record.
        let action_error = LifecycleErrorInfo::from_action_failure("shell", "boom");
        let failure_ctx = build_lifecycle_stack_context_for_materialized(
            LifecycleSignal::Failure,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            None,
            None,
            &fx.term,
            guard.emitter(),
            guard.context().settings,
            guard.context().messaging,
            &eng,
            Some(&action_error),
            None,
            None,
        );
        guard.run_event_stack(LifecycleSignal::Failure, &failure_ctx);
        // `Finalize` is a no-op because no terminal signal was recorded.
        run_lifecycle_event(
            &mut guard,
            LifecycleSignal::Finalize,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        assert!(
            !guard.finalize_emitted(),
            "without record_event_emission(Failure) the finalize is skipped"
        );
        // The failure stack ran but the finalize stack did not.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec!["failure-ran"],
            "finalize stack must not run when the terminal signal was never recorded"
        );
    }

    #[test]
    fn blocked_stack_error_routes_to_failure_keeps_blocked_comm_before_failure() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stderr": "blocked",
                "stack": [{"action": [{"append_line": ["events.log", "ran"]}, {"error": "downgraded"}]}]
            },
            "failure": {
                "stderr": "failed",
                "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());

        execute_terminal_event(
            &mut guard,
            LifecycleSignal::Blocked,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec!["ran", "failure-ran"],
            "blocked stack and failure stack each ran exactly once"
        );
        // The blocked top-level comm fired FIRST (top-level-before-stack), then
        // the downgrade fired the failure top-level comm.
        assert_eq!(
            emitter.events(),
            vec![
                Emitted::Stderr(LifecycleSignal::Blocked, "blocked".to_string()),
                Emitted::Stderr(LifecycleSignal::Failure, "failed".to_string()),
            ]
        );
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    }

    // -- dispatch_terminal_control runtime-wiring tests --------------------

    use claudine::composition::lifecycle_executor::{LifecycleEventOutcome, StackControl};
    use claudine::composition::RetryBackoff;

    fn prompt_state(source: &Path) -> HarnessPromptState {
        HarnessPromptState {
            mode: HarnessPromptMode::Compose,
            source_path: source.to_path_buf(),
            original_ref: source.display().to_string(),
            base_prompt: None,
            overlay: indexmap::IndexMap::new(),
            prompt_tail: Vec::new(),
            next_prompt_override: None,
            next_resume_session_id: None,
            rematerialize: Default::default(),
        }
    }

    /// A real provider profile that supports session resume (Claude).
    fn resume_capable_profile() -> &'static dyn crate::commands::wrap::profile::WrapperProfile {
        crate::commands::wrap::profile::profile_for_provider(Provider::Claude)
            .expect("claude profile exists")
    }

    fn outcome_with(control: StackControl) -> LifecycleEventOutcome {
        LifecycleEventOutcome {
            control: Some(control),
            ..Default::default()
        }
    }

    fn dispatch_guard<'a>(
        config: &'a LifecycleConfig,
        ctx: &'a LifecycleRuntimeContext<'a>,
        emitter: &'a RecordingEmitter,
    ) -> LifecycleRunGuard<'a> {
        LifecycleRunGuard::new(config, ctx, emitter)
    }

    #[test]
    fn dispatch_retry_from_failure_continues_and_resets_guard() {
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        // Mark a Failure terminal as already emitted to model the live call site.
        assert!(guard.record_event_emission(LifecycleSignal::Failure));
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();

        let outcome = outcome_with(StackControl::Retry {
            max_attempts: 2,
            backoff: RetryBackoff::Fixed,
            delay: "0s".to_string(),
        });
        let action = dispatch_terminal_control(
            &outcome,
            1,
            &mut budgets,
            Some("sess-1"),
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        match action {
            TerminalControlAction::Continue { next_attempt } => assert_eq!(next_attempt, 2),
            other => panic!("expected Continue, got {other:?}"),
        }
        // Guard was reset so the retried attempt can emit a fresh terminal.
        assert_eq!(guard.terminal_signal(), None);
    }

    #[test]
    fn dispatch_retry_from_finalize_continues_and_resets_guard() {
        // `finalize` is a last-chance recovery surface: a `finalize.stack`
        // ending in `retry` must re-enter the loop exactly as `failure` does.
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        // Model the live call site: a terminal signal and `finalize` already
        // fired this iteration before the finalize stack's control dispatches.
        assert!(guard.record_event_emission(LifecycleSignal::Failure));
        assert!(guard.record_event_emission(LifecycleSignal::Finalize));
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();

        let outcome = outcome_with(StackControl::Retry {
            max_attempts: 1,
            backoff: RetryBackoff::Fixed,
            delay: "0s".to_string(),
        });
        let action = dispatch_terminal_control(
            &outcome,
            1,
            &mut budgets,
            Some("sess-1"),
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        match action {
            TerminalControlAction::Continue { next_attempt } => assert_eq!(next_attempt, 2),
            other => panic!("expected Continue, got {other:?}"),
        }
        // Guard was reset so the retried attempt can emit a fresh terminal.
        assert_eq!(guard.terminal_signal(), None);
    }

    #[test]
    fn dispatch_resume_from_finalize_seeds_prompt_state() {
        // `resume` is valid at `finalize` too (parity with `failure`).
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();

        let outcome = outcome_with(StackControl::Resume {
            message: "finish the task".to_string(),
            max_attempts: 1,
        });
        let action = dispatch_terminal_control(
            &outcome,
            1,
            &mut budgets,
            Some("sess-1"),
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        assert!(matches!(action, TerminalControlAction::Continue { .. }));
        assert_eq!(state.next_prompt_override.as_deref(), Some("finish the task"));
        assert_eq!(state.next_resume_session_id.as_deref(), Some("sess-1"));
    }

    #[test]
    fn dispatch_retry_exhausts_after_budget() {
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        let mut state = prompt_state(&fx.source_path);
        // Pre-seed the retry budget to ceiling 2 (max_attempts 1 firing at 1).
        let mut budgets = ControlBudgets {
            retry: Some(2),
            resume: None,
        };
        let outcome = outcome_with(StackControl::Retry {
            max_attempts: 1,
            backoff: RetryBackoff::Fixed,
            delay: "0s".to_string(),
        });
        // attempt 2 has reached the ceiling → fall through (no continue).
        let action = dispatch_terminal_control(
            &outcome,
            2,
            &mut budgets,
            None,
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        assert!(matches!(action, TerminalControlAction::Fallthrough));
    }

    #[test]
    fn dispatch_resume_with_session_seeds_prompt_state() {
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        let outcome = outcome_with(StackControl::Resume {
            message: "please finish the task".to_string(),
            max_attempts: 1,
        });
        let action = dispatch_terminal_control(
            &outcome,
            1,
            &mut budgets,
            Some("sess-42"),
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        assert!(matches!(
            action,
            TerminalControlAction::Continue { next_attempt: 2 }
        ));
        assert_eq!(state.next_resume_session_id.as_deref(), Some("sess-42"));
        assert_eq!(
            state.next_prompt_override.as_deref(),
            Some("please finish the task")
        );
    }

    #[test]
    fn dispatch_resume_without_session_aborts_typed() {
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        let outcome = outcome_with(StackControl::Resume {
            message: "x".to_string(),
            max_attempts: 1,
        });
        let action = dispatch_terminal_control(
            &outcome,
            1,
            &mut budgets,
            None,
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        match action {
            TerminalControlAction::Abort(err) => {
                assert!(
                    err.to_string().contains("requires a live provider session"),
                    "unexpected: {err}"
                );
            }
            other => panic!("expected Abort, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_proxy_swaps_source_and_resets_guard_for_fresh_run() {
        let fx = fixture(serde_json::json!({}));
        let target = fx._dir.path().join("target.md");
        std::fs::write(&target, "---\n---\nbody\n").unwrap();
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        assert!(guard.record_event_emission(LifecycleSignal::Failure));
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        // Use an absolute target so resolution is unambiguous.
        let outcome = outcome_with(StackControl::Proxy {
            target: target.display().to_string(),
        });
        let action = dispatch_terminal_control(
            &outcome,
            3,
            &mut budgets,
            None,
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        // Proxy re-enters at attempt 1 for a fresh run.
        assert!(matches!(
            action,
            TerminalControlAction::Continue { next_attempt: 1 }
        ));
        assert_eq!(state.source_path, target);
        // The guard was fully reset (initialize will fire again).
        assert!(!guard.initialize_emitted());
        assert_eq!(guard.terminal_signal(), None);
    }

    #[test]
    fn dispatch_defer_aborts_not_implemented() {
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        let outcome = outcome_with(StackControl::Defer {
            delay: "5m".to_string(),
            reason: Some("later".to_string()),
        });
        let action = dispatch_terminal_control(
            &outcome,
            1,
            &mut budgets,
            None,
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        match action {
            TerminalControlAction::Abort(err) => {
                let msg = err.to_string();
                assert!(
                    msg.contains("defer")
                        && msg.contains("not implemented")
                        && msg.contains("rendezvous"),
                    "expected the defer-not-implemented error, got: {err}"
                );
            }
            other => panic!("expected Abort, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_stop_falls_through() {
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        let action = dispatch_terminal_control(
            &outcome_with(StackControl::Stop),
            1,
            &mut budgets,
            None,
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        assert!(matches!(action, TerminalControlAction::Fallthrough));
    }

    #[test]
    fn dispatch_error_aborts_without_changing_stop_semantics() {
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        let action = dispatch_terminal_control(
            &outcome_with(StackControl::Error {
                reason: Some("durable findings remain".to_string()),
            }),
            1,
            &mut budgets,
            None,
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        match action {
            TerminalControlAction::Abort(error) => {
                assert_eq!(error.to_string(), "durable findings remain");
            }
            other => panic!("expected Error to abort, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_no_control_falls_through() {
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        let action = dispatch_terminal_control(
            &LifecycleEventOutcome::default(),
            1,
            &mut budgets,
            None,
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        assert!(matches!(action, TerminalControlAction::Fallthrough));
    }

    // -- emit_blocked_finalize_with_err (Finding 5) ------------------------

    /// Before the provider launches, the helper selects `Blocked` as the
    /// terminal signal (matching `emit_blocked_or_failure`'s pre/post-launch
    /// rule) and runs both the blocked and finalize stacks, with `err`
    /// available to the stack expression engine.
    #[test]
    fn emit_blocked_finalize_pre_launch_runs_blocked_then_finalize_with_err() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stack": [
                    {"action": {"append_line": ["events.log", "{{ 'blocked-kind=' + err.kind }}"]}},
                    {"action": {"append_line": ["events.log", "{{ 'blocked-variant=' + err.variant }}"]}},
                ]
            },
            "finalize": {
                "stack": [
                    {"when": "err", "action": {"append_line": ["events.log", "{{ 'finalize-msg=' + err.msg }}"]}},
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

        emit_blocked_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        // Pre-launch → the terminal signal is Blocked, and finalize fired.
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Blocked));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec![
                "blocked-kind=LifecycleAction",
                "blocked-variant=materialize",
                "finalize-msg=boom",
            ],
            "blocked stack observes err.kind/err.variant; finalize `when: err` is \
             truthy and observes err.msg"
        );
    }

    /// Once the provider has launched, the helper selects `Failure` as the
    /// terminal signal (the post-launch branch of `emit_blocked_or_failure`).
    #[test]
    fn emit_blocked_finalize_post_launch_selects_failure() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
            },
            "finalize": {
                "stack": [{"action": {"append_line": ["events.log", "finalize-ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

        emit_blocked_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec!["failure-ran", "finalize-ran"],
        );
    }

    /// The materialize-failure call site has no real prompt, so it passes a
    /// synthetic prompt whose `frontmatter` is `Value::Null`. The stack-context
    /// builder must fall back to an empty frontmatter map (rather than panic or
    /// skip the stack), so the guard's own blocked/finalize stacks still fire
    /// and `err` remains available.
    #[test]
    fn emit_blocked_finalize_tolerates_null_frontmatter_materialized() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stack": [{"action": {"append_line": ["events.log", "{{ 'blocked-kind=' + err.kind }}"]}}]
            },
            "finalize": {
                "stack": [{"action": {"append_line": ["events.log", "finalize-ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");
        // The site-1 synthetic prompt: materialization failed, so there is no
        // frontmatter to carry — `Value::Null` exercises the empty-map fallback.
        let synthetic = materialized(serde_json::Value::Null);

        emit_blocked_finalize_with_err(
            &mut guard,
            &synthetic,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Blocked));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec!["blocked-kind=LifecycleAction", "finalize-ran"],
            "the guard's blocked/finalize stacks fire and observe err even when the \
             materialized prompt's frontmatter is null"
        );
    }

    // -- emit_failure_finalize_with_err (post-start setup `?` sites) --------

    /// The post-start setup sites (snapshot / launch / pre-spawn attempt) run
    /// after `start` and pre-flight have passed, so their terminal signal is
    /// always `Failure` — never `Blocked` — and `finalize` must follow with
    /// `err` available to both stacks. Here the guard has emitted `start` but
    /// the provider has NOT launched, the case the existing
    /// `provider_launched()`-driven helper would mis-route to `Blocked`.
    #[test]
    fn emit_failure_finalize_forces_failure_when_not_launched() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stderr": "failed",
                "stack": [
                    {"action": {"append_line": ["events.log", "{{ 'failure-kind=' + err.kind }}"]}},
                    {"action": {"append_line": ["events.log", "{{ 'failure-variant=' + err.variant }}"]}},
                ]
            },
            "finalize": {
                "stack": [
                    {"when": "err", "action": {"append_line": ["events.log", "{{ 'finalize-msg=' + err.msg }}"]}},
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // Reach `start` without launching the provider — exactly the state at
        // the snapshot / launch / pre-spawn-attempt `?` sites.
        assert!(guard.record_event_emission(LifecycleSignal::Start));
        assert!(!guard.provider_launched());
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("harness_snapshot", "boom");

        emit_failure_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        // Terminal is Failure (not Blocked) and finalize fired.
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        // Top-level failure communication fired.
        assert_eq!(
            emitter.events(),
            vec![Emitted::Stderr(LifecycleSignal::Failure, "failed".to_string())]
        );
        // Both stacks ran with `err` available.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec![
                "failure-kind=LifecycleAction",
                "failure-variant=harness_snapshot",
                "finalize-msg=boom",
            ],
            "failure stack observes err.kind/err.variant; finalize `when: err` is \
             truthy and observes err.msg"
        );
    }

    /// The materialized prompt for an attempt-execution failure carries the
    /// real frontmatter, but the helper must equally tolerate a synthetic
    /// `Value::Null` frontmatter (empty-map fallback) without skipping the
    /// stacks — mirroring the blocked-helper's null tolerance.
    #[test]
    fn emit_failure_finalize_tolerates_null_frontmatter() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
            },
            "finalize": {
                "stack": [{"action": {"append_line": ["events.log", "finalize-ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        assert!(guard.record_event_emission(LifecycleSignal::Start));
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("harness_attempt", "boom");
        let synthetic = materialized(serde_json::Value::Null);

        emit_failure_finalize_with_err(
            &mut guard,
            &synthetic,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec!["failure-ran", "finalize-ran"],
        );
    }

    // -- emit_*_with_err: late-binding evaluation error surfacing ------------

    /// Convenience: assert the helper returned a `LifecycleEvaluationError`
    /// naming `event`, then hand back the inner error for any extra checks.
    ///
    /// These surfacing helpers now mark the error already-emitted (Decision #2:
    /// the styled block was rendered to stderr at the catch point), so the
    /// returned error is wrapped in `LifecycleEvaluationAlreadyEmitted`. Unwrap
    /// it before asserting the inner shape — the presence of the marker confirms
    /// the early emit fired.
    fn assert_lifecycle_eval_error(
        result: Option<CompositionError>,
        event: &str,
    ) -> CompositionError {
        let err = result.expect("helper must return Some on a lifecycle evaluation raise");
        let inner = match &err {
            CompositionError::LifecycleEvaluationAlreadyEmitted { inner } => inner.as_ref(),
            other => other,
        };
        match inner {
            CompositionError::LifecycleEvaluationError { event: got, .. } => {
                assert_eq!(
                    got, event,
                    "expected LifecycleEvaluationError for `{event}`, got `{got}`"
                );
                err
            }
            other => panic!("expected LifecycleEvaluationError, got {other:?}"),
        }
    }

    /// Pre-launch: a `blocked.stack` `when:` raise must surface as a typed
    /// evaluation error naming `blocked`, and the helper must still fire the
    /// `failure` and `finalize` stacks (with the evaluation error as `err`) by
    /// redesignating the already-taken terminal slot. Without the redesignate
    /// fix, the failure stack would be silently refused and "failure-ran" would
    /// never appear.
    #[test]
    fn emit_blocked_finalize_pre_launch_blocked_raise_surfaces_failure_and_finalize() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stack": [
                    {"when": "missing_root == true", "action": {"stderr": "never"}}
                ]
            },
            "failure": {
                "stack": [
                    {"when": "err", "action": {"append_line": ["events.log", "failure-ran"]}}
                ]
            },
            "finalize": {
                "stack": [
                    {"when": "err", "action": {"append_line": ["events.log", "finalize-ran"]}}
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // Pre-launch — do NOT call `mark_provider_launched()` — so the helper
        // selects `Blocked` as the terminal signal.
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

        let result = emit_blocked_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        let typed = assert_lifecycle_eval_error(result, "blocked");
        assert!(
            typed.to_string().contains("evaluation error"),
            "error message surfaces evaluation error: {}",
            typed
        );
        // Redesignation took effect: terminal signal flipped Blocked → Failure.
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        // The key assertion: both failure and finalize stacks ran with the
        // evaluation error as `err` (the redesignate fix lets failure fire).
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        let logged: Vec<&str> = lines.lines().collect();
        assert!(
            logged.contains(&"failure-ran"),
            "failure stack fired with eval error as err: {logged:?}"
        );
        assert!(
            logged.contains(&"finalize-ran"),
            "finalize stack fired with eval error as err: {logged:?}"
        );
    }

    /// Post-launch: the helper selects `Failure` as the terminal signal. A
    /// `failure.stack` `when:` raise surfaces as a typed evaluation error
    /// naming `failure`, and the `finalize` stack still fires with the
    /// evaluation error as `err`. Failure is already terminal, so no
    /// redesignation is needed.
    #[test]
    fn emit_blocked_finalize_post_launch_failure_raise_surfaces_finalize() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stack": [
                    {"when": "missing_root == true", "action": {"stderr": "never"}}
                ]
            },
            "finalize": {
                "stack": [
                    {"when": "err", "action": {"append_line": ["events.log", "finalize-ran"]}}
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

        let result = emit_blocked_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert_lifecycle_eval_error(result, "failure");
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        let logged: Vec<&str> = lines.lines().collect();
        assert!(
            logged.contains(&"finalize-ran"),
            "finalize stack fired with eval error as err: {logged:?}"
        );
        // Finalize fired exactly once — the helper did not re-enter failure.
        assert_eq!(
            logged.iter().filter(|l| **l == "finalize-ran").count(),
            1,
            "finalize fired exactly once (no re-entry into failure)"
        );
    }

    /// A `finalize.stack` raise surfaces as a typed evaluation error naming
    /// `finalize`. The helper must not re-enter finalize, and the (already
    /// fired) blocked stack must not fire a second time.
    #[test]
    fn emit_blocked_finalize_finalize_raise_surfaces_without_reentry() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stack": [{"action": {"append_line": ["events.log", "blocked-ran"]}}]
            },
            "finalize": {
                "stack": [
                    {"when": "missing_root == true", "action": {"stderr": "never"}}
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

        let result = emit_blocked_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert_lifecycle_eval_error(result, "finalize");
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Blocked));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        let logged: Vec<&str> = lines.lines().collect();
        assert_eq!(
            logged.iter().filter(|l| **l == "blocked-ran").count(),
            1,
            "blocked stack fired exactly once (no re-entry)"
        );
    }

    /// review-4 regression: the pre-start **missing-source** setup-failure
    /// branch routes through `emit_blocked_finalize_with_err` (pre-launch →
    /// `Blocked`). A `blocked.when` raise must surface a typed evaluation error
    /// — proving the branch no longer swallows it in favor of the generic
    /// "source file does not exist" fallback. The surfaced event names the
    /// terminal event (`blocked`); the redesignate-to-failure path runs the
    /// `failure`/`finalize` stacks but the typed error still reports the slot
    /// where the raise occurred.
    #[test]
    fn missing_source_branch_blocked_raise_surfaces_not_swallowed() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stack": [
                    {"when": "missing_root == true", "action": {"stderr": "never"}}
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // Pre-launch — the missing-source branch is reached before the provider
        // launches, so do NOT mark it launched; the helper selects `Blocked`.
        let eng = engine(fx._dir.path());
        // The exact err_info the missing-source branch builds.
        let err_info = LifecycleErrorInfo::from_action_failure(
            "missing_source",
            "source file does not exist: prompt.md",
        );

        let result = emit_blocked_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        // The terminal slot was `blocked`, so the typed error names `blocked`;
        // the evaluation error is surfaced rather than swallowed.
        let typed = assert_lifecycle_eval_error(result, "blocked");
        let rendered = typed.to_string();
        assert!(
            rendered.contains("evaluation error"),
            "error surfaces the evaluation error: {rendered}"
        );
        // The generic missing-source fallback is NOT the surfaced error.
        assert!(
            !rendered.contains("source file does not exist"),
            "the lifecycle raise supersedes the generic fallback: {rendered}"
        );
    }

    /// review-4 regression: the pre-start **shell-audit** setup-failure branch
    /// routes through `emit_blocked_finalize_with_err`. A `finalize.when` raise
    /// (with a clean `blocked`) must surface a typed evaluation error naming
    /// `finalize` without re-entering finalize — proving the branch no longer
    /// swallows it in favor of the generic "shell audit failed" fallback.
    #[test]
    fn shell_audit_branch_finalize_raise_surfaces_not_swallowed() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stack": [{"action": {"append_line": ["events.log", "blocked-ran"]}}]
            },
            "finalize": {
                "stack": [
                    {"when": "missing_root == true", "action": {"stderr": "never"}}
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // Pre-launch — the shell-audit branch fires before launch.
        let eng = engine(fx._dir.path());
        // The exact err_info the shell-audit branch builds.
        let err_info = LifecycleErrorInfo::from_action_failure(
            "shell_audit",
            "shell audit failed: 1 denied directive(s) in source page",
        );

        let result = emit_blocked_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        let typed = assert_lifecycle_eval_error(result, "finalize");
        let rendered = typed.to_string();
        assert!(
            rendered.contains("evaluation error"),
            "error surfaces the evaluation error: {rendered}"
        );
        // The generic shell-audit fallback is NOT the surfaced error.
        assert!(
            !rendered.contains("shell audit failed"),
            "the lifecycle raise supersedes the generic fallback: {rendered}"
        );
        // The clean blocked stack fired exactly once and finalize did not
        // re-enter.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().filter(|l| *l == "blocked-ran").count(),
            1,
            "blocked stack fired exactly once (no re-entry)"
        );
    }

    /// `emit_failure_finalize_with_err` — a `failure.stack` raise surfaces as
    /// a typed evaluation error naming `failure`, and the `finalize` stack
    /// still fires with the evaluation error as `err`.
    #[test]
    fn emit_failure_finalize_failure_raise_surfaces_finalize() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stack": [
                    {"when": "missing_root == true", "action": {"stderr": "never"}}
                ]
            },
            "finalize": {
                "stack": [
                    {"when": "err", "action": {"append_line": ["events.log", "finalize-ran"]}}
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // Reach `start` without launching the provider — exactly the state at
        // the snapshot / launch / pre-spawn-attempt `?` sites.
        assert!(guard.record_event_emission(LifecycleSignal::Start));
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("harness_snapshot", "boom");

        let result = emit_failure_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert_lifecycle_eval_error(result, "failure");
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        let logged: Vec<&str> = lines.lines().collect();
        assert!(
            logged.contains(&"finalize-ran"),
            "finalize stack fired with eval error as err: {logged:?}"
        );
    }

    /// `emit_failure_finalize_with_err` — a `finalize.stack` raise surfaces as
    /// a typed evaluation error naming `finalize`. The failure stack (already
    /// fired) must not fire a second time.
    #[test]
    fn emit_failure_finalize_finalize_raise_surfaces_without_reentry() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
            },
            "finalize": {
                "stack": [
                    {"when": "missing_root == true", "action": {"stderr": "never"}}
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        assert!(guard.record_event_emission(LifecycleSignal::Start));
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("harness_attempt", "boom");

        let result = emit_failure_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert_lifecycle_eval_error(result, "finalize");
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        let logged: Vec<&str> = lines.lines().collect();
        assert_eq!(
            logged.iter().filter(|l| **l == "failure-ran").count(),
            1,
            "failure stack fired exactly once (no re-entry)"
        );
    }

    /// Precedence: when both `failure` and `finalize` raise after a setup
    /// error, the surfaced error must name `finalize` — the latest lifecycle
    /// crash — not `failure`. Previously the failure raise hid the finalize
    /// raise behind it.
    #[test]
    fn emit_failure_finalize_both_raise_surfaces_finalize() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
            },
            "finalize": {
                "stack": [{"when": "also_missing == true", "action": {"stderr": "never"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        assert!(guard.record_event_emission(LifecycleSignal::Start));
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("harness_attempt", "boom");

        let result = emit_failure_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert_lifecycle_eval_error(result, "finalize");
        assert!(guard.finalize_emitted(), "finalize must have fired");
    }

    /// Precedence: a `success.when` raise followed by a `finalize.when` raise
    /// must surface the finalize raise — not the original `success` raise.
    /// Drives the same path the runtime takes for a terminal evaluation
    /// error: `execute_terminal_event` records the raise, then
    /// `handle_terminal_evaluation_error` runs `finalize` carrying it.
    #[test]
    fn success_raise_then_finalize_raise_surfaces_finalize() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
            },
            "finalize": {
                "stack": [{"when": "also_missing == true", "action": {"stderr": "never"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let success = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );
        assert!(
            success.outcome.evaluation_error.is_some(),
            "the success `when:` raised"
        );

        let err = handle_terminal_evaluation_error(
            &success.outcome,
            "success",
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        )
        .expect("the terminal evaluation error halts the run");

        let rendered = err.to_string();
        assert!(
            rendered.contains("`finalize`"),
            "the error must name the finalize event, not success; got: {rendered}"
        );
        assert!(
            !rendered.contains("`success`"),
            "the error must NOT name the success event; got: {rendered}"
        );
    }

    /// Precedence: a setup-phase `initialize`/`start` raise followed by a
    /// `failure.when` raise must surface `failure`, and `finalize` must
    /// receive the FAILURE evaluation error as `err` (not the original). The
    /// `finalize.stack` interpolates `{{ err.event }}` so we can prove it
    /// observed the failure raise.
    #[test]
    fn setup_raise_then_failure_raise_surfaces_failure_and_threads_into_finalize() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stack": [{"when": "failure_typo == true", "action": {"stderr": "never"}}]
            },
            "finalize": {
                "stack": [{
                    "when": "err",
                    "action": {"append_line": ["events.log", "finalize-saw-{{err.variant}}"]}
                }]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());

        // Model a `start` stack that raised at event time.
        let outcome = LifecycleEventOutcome {
            evaluation_error: Some(LifecycleErrorInfo::from_action_failure(
                "when",
                "`when:` references undefined variable `missing_root`",
            )),
            ..Default::default()
        };
        let err = handle_setup_evaluation_error(
            &outcome,
            "start",
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        )
        .expect("the setup evaluation error halts the run");

        let rendered = err.to_string();
        assert!(
            rendered.contains("`failure`"),
            "the error must name the failure event (failure raised); got: {rendered}"
        );
        assert!(
            !rendered.contains("`start`"),
            "the error must NOT name the start event; got: {rendered}"
        );

        // `finalize` ran with the FAILURE evaluation error as `err` — its
        // appended marker interpolates `err.variant`, which the failure raise
        // fills with `when` (the variant of the failure `when:` raise), not
        // the original `missing_root` text.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert!(
            lines.contains("finalize-saw-when"),
            "finalize must have observed the failure evaluation error (variant=when); got: {lines}"
        );
    }

    /// Precedence: a `blocked.when` raise (terminal) followed by a catch
    /// `finalize.when` raise must surface `finalize`. Pre-launch so the
    /// helper selects `Blocked`; the redesignation path runs `failure` (no
    /// raise authored), then `finalize` raises.
    #[test]
    fn emit_blocked_finalize_blocked_raise_then_finalize_raise_surfaces_finalize() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
            },
            "failure": {
                "stack": [{"when": "err", "action": {"append_line": ["events.log", "failure-ran"]}}]
            },
            "finalize": {
                "stack": [{"when": "also_missing == true", "action": {"stderr": "never"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // Pre-launch — do NOT call `mark_provider_launched()` — so the helper
        // selects `Blocked` as the terminal signal and redesignates to Failure.
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

        let result = emit_blocked_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert_lifecycle_eval_error(result, "finalize");
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        // The failure stack ran (no raise authored) and saw `err`.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert!(
            lines.contains("failure-ran"),
            "failure stack ran with the original blocked evaluation error as err: {lines}"
        );
    }

    /// Happy-path regression: with no evaluation raises the helper returns
    /// `None` and the caller propagates the original setup error unchanged.
    #[test]
    fn emit_blocked_finalize_returns_none_when_no_evaluation_error() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stack": [{"action": {"append_line": ["events.log", "blocked-ran"]}}]
            },
            "finalize": {
                "stack": [{"action": {"append_line": ["events.log", "finalize-ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

        let result = emit_blocked_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert!(result.is_none(), "no evaluation error → returns None");
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Blocked));
        assert!(guard.finalize_emitted(), "finalize still fires on the happy path");
    }

    // -- Broken-path regression tests: explicit error(...) / routes_to_failure
    //    catch paths that previously discarded failure/finalize outcomes ------
    //
    // These exercise the previously-broken catch paths where an explicit
    // lifecycle control (`error(...)`), action-error routing (`routes_to_failure`),
    // or terminal-control abort still runs failure/finalize but discarded the
    // returned outcomes — swallowing any evaluation error raised by those catch
    // events.

    /// `run_target_initialize` — a target's `initialize.error(...)` whose catch
    /// `failure.when:` raises surfaces the FAILURE evaluation error, not the
    /// original `error(...)` reason. Proves the previously-discarded failure
    /// outcome now threads through `catch_evaluation_error`.
    #[test]
    fn target_initialize_error_with_failure_raise_surfaces_failure_evaluation_error() {
        let fx = fixture(serde_json::json!({
            "initialize": {
                "stack": [{"action": {"error": "target refused"}}]
            },
            "failure": {
                "stderr": "fail",
                "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
            },
            "finalize": { "stderr": "final" }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());

        let action = run_target_initialize(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        );

        match action {
            TargetInitializeAction::Abort(report) => {
                let rendered = report.to_string();
                assert!(
                    rendered.contains("`failure`"),
                    "the surfaced error must name the failure event; got: {rendered}"
                );
                assert!(
                    rendered.contains("evaluation error"),
                    "the surfaced error must mention evaluation error; got: {rendered}"
                );
                assert!(
                    !rendered.contains("target refused"),
                    "the original `error(...)` reason must NOT be the surfaced error; got: {rendered}"
                );
            }
            other => panic!("expected Abort, got {other:?}"),
        }
    }

    /// `run_target_initialize` — a target's `initialize` action error that
    /// `routes_to_failure` whose catch `failure.when:` raises surfaces the
    /// FAILURE evaluation error, not the generic "lifecycle initialize failed"
    /// fallback. Proves the previously-discarded failure outcome now threads
    /// through `catch_evaluation_error` for the routes_to_failure path.
    #[test]
    fn target_initialize_routes_to_failure_with_raise_surfaces_failure_evaluation_error() {
        let fx = fixture(serde_json::json!({
            // A `shell: false` action errors and routes_to_failure(Initialize).
            "initialize": {
                "stack": [{"action": {"shell": "false"}}]
            },
            "failure": {
                "stderr": "fail",
                "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
            },
            "finalize": { "stderr": "final" }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());

        let action = run_target_initialize(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        );

        match action {
            TargetInitializeAction::Abort(report) => {
                let rendered = report.to_string();
                assert!(
                    rendered.contains("`failure`"),
                    "the surfaced error must name the failure event; got: {rendered}"
                );
                assert!(
                    rendered.contains("evaluation error"),
                    "the surfaced error must mention evaluation error; got: {rendered}"
                );
                assert!(
                    !rendered.contains("lifecycle initialize failed"),
                    "the generic fallback message must NOT be the surfaced error; got: {rendered}"
                );
            }
            other => panic!("expected Abort, got {other:?}"),
        }
    }

    /// Start `routes_to_failure` catch path (Location G): when `failure.when`
    /// raises after a start action error, the surfaced error must name
    /// `failure`, and finalize must receive the FAILURE evaluation error as
    /// `err` (not the original action error) so a `finalize.stack` can branch
    /// on the failure raise. Simulates the inline `run_harness_loop` code
    /// path's primitives directly (record_event_emission + run_event_stack +
    /// run_lifecycle_event) since the surrounding function is impractical to
    /// call from a unit test.
    #[test]
    fn start_routes_to_failure_with_raise_surfaces_failure_and_threads_into_finalize() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stderr": "fail",
                "stack": [{
                    "when": "missing_root == true",
                    "action": {"stderr": "never"}
                }]
            },
            "finalize": {
                "stderr": "final",
                "stack": [{
                    "when": "err",
                    "action": {"append_line": ["events.log", "finalize-saw-{{err.variant}}"]}
                }]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // Mirror run_harness_loop's pre-start state.
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        // Model a `start` outcome that routed to failure with an action error.
        let action_error = LifecycleErrorInfo::from_action_failure("shell", "boom");

        // Replicate the Location G fix: record Failure FIRST, then run the
        // error-carrying failure stack via run_event_stack, threading any
        // failure raise into finalize.
        assert!(guard.record_event_emission(LifecycleSignal::Failure));
        let failure_ctx = build_lifecycle_stack_context_for_materialized(
            LifecycleSignal::Failure,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            None,
            None,
            &fx.term,
            guard.emitter(),
            guard.context().settings,
            guard.context().messaging,
            &eng,
            Some(&action_error),
            None,
            None,
        );
        let failure_outcome = guard.run_event_stack(LifecycleSignal::Failure, &failure_ctx);

        // The fix: thread active_err into finalize (failure raise > original).
        // When failure raises, active_err is the failure evaluation error; the
        // synthetic-fallback case (no original action_error and no failure
        // raise) is exercised by the runtime paths but not duplicated here.
        let active_err = failure_outcome
            .evaluation_error
            .as_ref()
            .unwrap_or(&action_error);
        let finalize_outcome = run_lifecycle_event(
            &mut guard,
            LifecycleSignal::Finalize,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            Some(active_err),
            std::time::Instant::now(),
        );

        // The failure raised, so the surfaced error must name `failure`.
        assert!(
            failure_outcome.evaluation_error.is_some(),
            "the failure `when:` raised"
        );
        let ce = CompositionError::catch_evaluation_error(
            &fx.source_path,
            "start",
            &action_error,
            Some(&failure_outcome),
            Some(&finalize_outcome),
        );
        let rendered = ce.to_string();
        assert!(
            rendered.contains("`failure`"),
            "the surfaced error must name the failure event; got: {rendered}"
        );
        assert!(
            !rendered.contains("`start`"),
            "the surfaced error must NOT name the start event; got: {rendered}"
        );

        // finalize ran with the FAILURE evaluation error as `err` — its
        // appended marker interpolates `err.variant`, which the failure raise
        // fills with `when` (the variant of the failure `when:` raise), not
        // the original `shell` action_error variant.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert!(
            lines.contains("finalize-saw-when"),
            "finalize must have observed the failure evaluation error (variant=when); got: {lines}"
        );
    }

    /// Terminal-control abort catch path (Locations H/I/J): when `finalize.when`
    /// raises after a terminal-control Abort decision, the surfaced error must
    /// name `finalize` (the catch event's raise), not the original abort
    /// reason. Simulates the inline `run_harness_loop` Abort arm directly.
    #[test]
    fn terminal_control_abort_with_finalize_raise_surfaces_finalize_evaluation_error() {
        let fx = fixture(serde_json::json!({
            "finalize": {
                "stderr": "final",
                "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // The failure/success event already fired cleanly before the Abort.
        guard.mark_provider_launched();
        guard.record_event_emission(LifecycleSignal::Failure);
        let eng = engine(fx._dir.path());

        // Replicate the Location H/I/J fix: run finalize carrying the abort's
        // err_info; if finalize raises, surface the finalize evaluation error.
        let err_info = LifecycleErrorInfo::from_action_failure("agent_failure", "boom");
        let finalize_outcome = run_lifecycle_event(
            &mut guard,
            LifecycleSignal::Finalize,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            Some(&err_info),
            std::time::Instant::now(),
        );

        let surfaced_err: color_eyre::eyre::Report = if let Some(eval_info) =
            finalize_outcome.evaluation_error.as_ref()
        {
            CompositionError::lifecycle_evaluation("finalize", &fx.source_path, eval_info).into()
        } else {
            // The original abort reason would surface here on the happy path.
            eyre!("original abort reason")
        };

        let rendered = surfaced_err.to_string();
        assert!(
            rendered.contains("`finalize`"),
            "the surfaced error must name the finalize event; got: {rendered}"
        );
        assert!(
            rendered.contains("evaluation error"),
            "the surfaced error must mention evaluation error; got: {rendered}"
        );
        assert!(
            !rendered.contains("original abort reason"),
            "the original abort reason must NOT be the surfaced error; got: {rendered}"
        );
    }

    /// Interrupt branch (review-4 Sites B+C): when the run is interrupted and a
    /// `failure.when` raises, `handle_terminal_evaluation_error` must surface a
    /// `failure`-named evaluation error and run `finalize` exactly once (the
    /// helper owns the finalize run; the interrupt branch must not also run a
    /// second finalize). Drives the fixed primitives directly since
    /// `run_harness_loop` is impractical from a unit test.
    #[test]
    fn interrupt_failure_when_raise_surfaces_failure_and_runs_finalize_once() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stderr": "fail",
                "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
            },
            "finalize": {
                "stderr": "final",
                "stack": [{"action": {"append_line": ["events.log", "finalized"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // The provider launched before the interrupt, so the Failure slot path
        // is taken (mirrors the interrupt branch's `execute_terminal_event`).
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let err_info =
            LifecycleErrorInfo::from_action_failure("interrupted", "user interrupted the run");
        let failure_outcome = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Failure,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            Some(&err_info),
            std::time::Instant::now(),
        )
        .outcome;

        let surfaced = handle_terminal_evaluation_error(
            &failure_outcome,
            "failure",
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        );

        let report = surfaced.expect("failure `when:` raise must surface a halting error");
        let rendered = report.to_string();
        assert!(
            rendered.contains("`failure`"),
            "the surfaced error must name the failure event; got: {rendered}"
        );
        assert!(
            rendered.contains("evaluation error"),
            "the surfaced error must mention evaluation error; got: {rendered}"
        );
        // `handle_terminal_evaluation_error` runs `finalize` once internally; the
        // interrupt branch must NOT run it again (no recursive re-entry).
        assert_eq!(
            line_count(&fx.log_path),
            1,
            "finalize ran exactly once (handler-owned, no double finalize)"
        );
    }

    /// Interrupt branch (review-4 Sites B+C): a clean `failure` followed by a
    /// raising `finalize.when`. `handle_terminal_evaluation_error` returns
    /// `None` (failure did not raise), then the interrupt branch's own finalize
    /// run raises → a `finalize`-named evaluation error halts the run, and the
    /// `Ok((exit_code, ...))` happy path is NOT taken.
    #[test]
    fn interrupt_finalize_when_raise_surfaces_finalize_evaluation_error() {
        let fx = fixture(serde_json::json!({
            "failure": { "stderr": "fail" },
            "finalize": {
                "stderr": "final",
                "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let err_info =
            LifecycleErrorInfo::from_action_failure("interrupted", "user interrupted the run");
        let failure_outcome = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Failure,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            Some(&err_info),
            std::time::Instant::now(),
        )
        .outcome;

        // The clean `failure` stack does not raise.
        assert!(
            handle_terminal_evaluation_error(
                &failure_outcome,
                "failure",
                &mut guard,
                &fx.materialized,
                &fx.source_path,
                Some(fx._dir.path()),
                &fx.term,
                &eng,
                std::time::Instant::now(),
            )
            .is_none(),
            "a clean failure must not surface an evaluation error"
        );

        // The interrupt branch then runs `finalize`, which raises here.
        let finalize_outcome = run_lifecycle_event(
            &mut guard,
            LifecycleSignal::Finalize,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            Some(&err_info),
            std::time::Instant::now(),
        );

        let surfaced: Option<color_eyre::eyre::Report> =
            finalize_outcome.evaluation_error.as_ref().map(|eval_info| {
                CompositionError::lifecycle_evaluation("finalize", &fx.source_path, eval_info).into()
            });
        let report = surfaced.expect("finalize `when:` raise must halt instead of returning Ok");
        let rendered = report.to_string();
        assert!(
            rendered.contains("`finalize`"),
            "the surfaced error must name the finalize event; got: {rendered}"
        );
        assert!(
            rendered.contains("evaluation error"),
            "the surfaced error must mention evaluation error; got: {rendered}"
        );
    }

    /// Start control-abort site (review-4 Site A): when the `start`
    /// control-dispatch aborts and `finalize.when` raises, the surfaced error
    /// must name `finalize` (the catch event's raise), not the original abort
    /// reason. The start-abort finalize runs with `None` `err` (no error info is
    /// available at that point), so this mirrors
    /// `terminal_control_abort_with_finalize_raise_surfaces_finalize_evaluation_error`
    /// but with a `None` finalize `err`.
    #[test]
    fn start_control_abort_with_finalize_raise_surfaces_finalize_evaluation_error() {
        let fx = fixture(serde_json::json!({
            "finalize": {
                "stderr": "final",
                "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // A terminal slot was taken before the control-abort decision, so the
        // subsequent `finalize` is eligible to fire (its run is gated on a
        // recorded terminal emission).
        guard.mark_provider_launched();
        guard.record_event_emission(LifecycleSignal::Failure);
        let eng = engine(fx._dir.path());

        // Replicate the Site A fix: finalize runs with `None` err; if it raises,
        // surface the finalize evaluation error in place of the abort reason.
        let finalize_outcome = run_lifecycle_event(
            &mut guard,
            LifecycleSignal::Finalize,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        let surfaced_err: color_eyre::eyre::Report =
            if let Some(eval_info) = finalize_outcome.evaluation_error.as_ref() {
                CompositionError::lifecycle_evaluation("finalize", &fx.source_path, eval_info).into()
            } else {
                eyre!("original abort reason")
            };

        let rendered = surfaced_err.to_string();
        assert!(
            rendered.contains("`finalize`"),
            "the surfaced error must name the finalize event; got: {rendered}"
        );
        assert!(
            rendered.contains("evaluation error"),
            "the surfaced error must mention evaluation error; got: {rendered}"
        );
        assert!(
            !rendered.contains("original abort reason"),
            "the original abort reason must NOT be the surfaced error; got: {rendered}"
        );
    }
}

mod requeue_fallback_tests {
    use super::*;
    use indexmap::IndexMap;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Build a `requeue(...)`-shaped prompt state pointing at `source`.
    fn requeue_prompt_state(source: &Path) -> HarnessPromptState {
        HarnessPromptState {
            mode: HarnessPromptMode::Compose,
            source_path: source.to_path_buf(),
            original_ref: source.display().to_string(),
            base_prompt: None,
            overlay: IndexMap::new(),
            prompt_tail: Vec::new(),
            next_prompt_override: None,
            next_resume_session_id: None,
            rematerialize: Default::default(),
        }
    }

    /// Build a materialized prompt with the deferred-prompt body the requeue
    /// action is supposed to persist.
    fn requeue_materialized(prompt: &str) -> MaterializedHarnessPrompt {
        let frontmatter = serde_json::json!({"title": "deferred"});
        let live_frontmatter = MaterializedHarnessPrompt::live_cell_from(&frontmatter);
        MaterializedHarnessPrompt {
            frontmatter,
            prompt: prompt.to_string(),
            env_overrides: Vec::new(),
            inline_closure_plan: None,
            live_frontmatter,
        }
    }

    /// The cross-platform Windows-facing contract: when the rendezvous daemon
    /// is unreachable, `enqueue_requeue_entry` must NOT abort — it must
    /// return `Ok(())` and append exactly one durable fallback entry whose
    /// shape matches what the daemon would have received. This is the exact
    /// code path a Windows user takes (no daemon runs there), proven on the
    /// macOS host by pointing `RENDEZVOUS_SOCKET` at a non-existent socket.
    #[tokio::test]
    #[serial_test::serial(requeue_fallback)]
    async fn enqueue_requeue_entry_falls_back_to_durable_file_when_daemon_unreachable() {
        let fallback_dir = TempDir::new().expect("tempdir");
        let fallback_path: PathBuf = fallback_dir.path().join(REQUEUE_FALLBACK_FILE_NAME);
        let _socket_env =
            test_toolkit::EnvGuard::set_safe("RENDEZVOUS_SOCKET", "/tmp/does-not-exist-rs.sock");
        let _fallback_env =
            test_toolkit::EnvGuard::set_safe(REQUEUE_FALLBACK_DIR_ENV, fallback_dir.path());

        let workspace = TempDir::new().expect("workspace tempdir");
        let source_path = workspace.path().join("deferred.md");
        std::fs::write(&source_path, "defer body").expect("write source");
        let prompt_state = requeue_prompt_state(&source_path);
        let materialized = requeue_materialized("Body to defer through rendezvous\n");

        let result = enqueue_requeue_entry_async(
            Provider::Goose,
            &prompt_state,
            &materialized,
            Some(workspace.path()),
            "5m",
            Some("provider failed"),
        )
        .await;
        assert!(
            result.is_ok(),
            "daemon-unreachable requeue must succeed via fallback; got {:?}",
            result.err()
        );

        // Exactly one JSONL line was appended.
        let contents = std::fs::read_to_string(&fallback_path).expect("fallback file written");
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 1, "exactly one fallback entry; got {lines:?}");
        let entry: serde_json::Value =
            serde_json::from_str(lines[0]).expect("fallback line is valid JSON");

        // The entry carries the same shape as AppendEntryRequest.
        assert_eq!(entry["source"], REQUEUE_SOURCE);
        assert_eq!(entry["level"], "info");
        assert_eq!(entry["session_id"], REQUEUE_SESSION_ID);
        assert_eq!(entry["owner_node_id"], "");
        let message = entry["message"].as_str().expect("message is a string");
        assert!(
            message.contains("deferred.md") && message.contains("5m"),
            "entry message should identify the prompt and delay; got {message:?}"
        );

        // `metadata_json` is embedded as a parsed object — its inner shape is
        // the contract a future daemon drain depends on.
        let metadata = &entry["metadata_json"];
        assert_eq!(metadata["kind"], "claudine.lifecycle.requeue");
        assert_eq!(metadata["provider"], "goose");
        assert_eq!(metadata["delay"], "5m");
        assert_eq!(metadata["reason"], "provider failed");
        assert_eq!(metadata["prompt"], "Body to defer through rendezvous\n");
        assert!(
            metadata["source_path"]
                .as_str()
                .is_some_and(|p| p.ends_with("deferred.md")),
            "metadata should record source_path; got {metadata}"
        );
    }

    /// A second requeue on the same fallback file appends rather than
    /// overwriting — the queue is durable and accumulates across runs.
    #[tokio::test]
    #[serial_test::serial(requeue_fallback)]
    async fn enqueue_requeue_entry_fallback_appends_across_calls() {
        let fallback_dir = TempDir::new().expect("tempdir");
        let fallback_path: PathBuf = fallback_dir.path().join(REQUEUE_FALLBACK_FILE_NAME);
        let _socket_env =
            test_toolkit::EnvGuard::set_safe("RENDEZVOUS_SOCKET", "/tmp/does-not-exist-rs.sock");
        let _fallback_env =
            test_toolkit::EnvGuard::set_safe(REQUEUE_FALLBACK_DIR_ENV, fallback_dir.path());

        let workspace = TempDir::new().expect("workspace tempdir");
        let source_path = workspace.path().join("deferred.md");
        std::fs::write(&source_path, "defer body").expect("write source");
        let prompt_state = requeue_prompt_state(&source_path);
        let materialized = requeue_materialized("body\n");

        enqueue_requeue_entry_async(
            Provider::Goose,
            &prompt_state,
            &materialized,
            Some(workspace.path()),
            "1m",
            None,
        )
        .await
        .expect("first enqueue");
        enqueue_requeue_entry_async(
            Provider::Goose,
            &prompt_state,
            &materialized,
            Some(workspace.path()),
            "2m",
            None,
        )
        .await
        .expect("second enqueue");

        let contents = std::fs::read_to_string(&fallback_path).expect("fallback file");
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2, "fallback file accumulates entries");
        let first: serde_json::Value = serde_json::from_str(lines[0]).expect("first entry parses");
        let second: serde_json::Value =
            serde_json::from_str(lines[1]).expect("second entry parses");
        assert_eq!(first["metadata_json"]["delay"], "1m");
        assert_eq!(second["metadata_json"]["delay"], "2m");
    }
}
