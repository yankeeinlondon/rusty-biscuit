use claudine::composition::LifecycleRunGuardExt;
use super::*;

#[test]
fn phase_result_preserves_all_pipeline_outcomes() {
    let proceed: CompositionPhaseResult<u8> = CompositionPhaseResult::Proceed(1);
    assert!(matches!(proceed, CompositionPhaseResult::Proceed(1)));

    let blocked: CompositionPhaseResult<()> =
        CompositionPhaseResult::Blocked(eyre!("blocked"));
    assert!(matches!(blocked, CompositionPhaseResult::Blocked(_)));

    let failed: CompositionPhaseResult<()> =
        CompositionPhaseResult::Failed(eyre!("failed"));
    assert!(matches!(failed, CompositionPhaseResult::Failed(_)));
}

struct CancellingInitializeEmitter {
    token: claudine::composition::lifecycle::cancellation::CancellationToken,
    events: std::sync::Mutex<Vec<(LifecycleSignal, String)>>,
    warnings: std::sync::Mutex<Vec<String>>,
}

impl claudine::composition::lifecycle::LifecycleEmitter for CancellingInitializeEmitter {
    fn emit_stderr(&self, signal: LifecycleSignal, text: &str, _: &Terminal) {
        self.events.lock().unwrap().push((signal, text.into()));
        if signal == LifecycleSignal::Initialize {
            self.token.cancel(claudine::composition::lifecycle::cancellation::CancellationCause::Interrupt);
        }
    }
    fn emit_message(&self, _: &str, _: &Path, _: Option<&Path>, _: &claudine::messaging::RuntimeMessagingSettings) {
        panic!("unexpected message");
    }
    fn emit_speech(&self, _: &str, _: biscuit_speaks::TtsConfig) {
        panic!("unexpected speech");
    }
    fn emit_effect(&self, _: &str) { panic!("unexpected effect"); }
    fn emit_notification(&self, _: &str) { panic!("unexpected notification"); }
    fn emit_warn(&self, text: &str, _: &Terminal) {
        self.warnings.lock().unwrap().push(text.into());
    }
}

fn cancelled_initialize_route(control: serde_json::Value, cleanup: serde_json::Value) {
    use claudine::composition::lifecycle::{LifecycleRuntimeContext, parse_lifecycle_config};
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("prompt.md");
    let marker = dir.path().join("must-not-exist");
    let frontmatter = serde_json::json!({
        "initialize": [{"stderr":"entered-initialize"}, control],
        "blocked": {"stderr":"unexpected-blocked"},
        "failure": {"stderr":"unexpected-failure"},
        "start": {"stderr":"unexpected-start"},
        "cancelled": {"stderr":"cancelled"},
        "finalize": [{"stderr":"finalized"}, cleanup]
    });
    let config = parse_lifecycle_config(&frontmatter, &source).unwrap();
    let token = claudine::composition::lifecycle::cancellation::CancellationToken::default();
    let _scope = token.enter();
    let emitter = CancellingInitializeEmitter {
        token, events: Default::default(), warnings: Default::default(),
    };
    let settings = claudine::events::GlobalSettings::default();
    let messaging = claudine::messaging::RuntimeMessagingSettings { user: None, repo: None };
    let term = Terminal::default();
    let context = darkmatter::markdown::compose::ComposeContext::capture_for_dir(dir.path());
    let runtime = LifecycleRuntimeContext {
        settings: &settings, messaging: &messaging, term: &term, source_path: &source,
        repo_root: None, launch_area: Some(dir.path()), context: Some(&context),
    };
    let mut guard = LifecycleRunGuard::new(&config, &runtime, &emitter);
    let resolution = biscuit_file::FileResolutionContext::new(dir.path());
    let engine = EffectEngine::builder().mutation_root(dir.path()).auto_rehash(false).build();
    let init = StackExecutionContext {
        seq: None, signal: LifecycleSignal::Initialize, scope: None,
        frontmatter: &darkmatter::markdown::compose::SharedFrontmatter::from(frontmatter.as_object().unwrap().clone()), live_frontmatter: None,
        runtime_state: None, err: None, timing: None, current: None, group: None,
        loop_values: None, base_dir: Some(dir.path()), ctx_base_dir: Some(dir.path()),
        prepared_context: Some(&context), file_resolution_context: &resolution,
        effect_engine: &engine, shell_runner: &SystemShellRunner, emitter: &emitter,
        term: &term, source_path: &source, repo_root: None, messaging: &messaging,
        settings: &settings,
    };
    let result = route_initialize(&mut guard, &init, &source, Provider::Claude, &term);
    let CompositionPhaseResult::Completed(outcome) = result else {
        panic!("cancelled initialize must complete instead of recovering");
    };
    assert_eq!(outcome.exit_code, 130);
    assert!(outcome.handoff.is_none());
    assert_eq!(outcome.terminal_signal, Some(LifecycleSignal::Cancelled));
    assert_eq!(*emitter.events.lock().unwrap(), [
        (LifecycleSignal::Initialize, "entered-initialize".into()),
        (LifecycleSignal::Cancelled, "cancelled".into()),
        (LifecycleSignal::Finalize, "finalized".into()),
    ]);
    assert!(guard.finalize_emitted());
    assert!(!marker.exists());
    assert!(!emitter.warnings.lock().unwrap().is_empty(), "cleanup failures must remain visible");
}

#[test]
fn cancelled_initialize_skip_keeps_exit_130_and_refuses_cleanup_shell() {
    cancelled_initialize_route(serde_json::json!({"skip":null}),
        serde_json::json!({"shell":"echo forbidden > must-not-exist"}));
}

#[test]
fn cancelled_initialize_proxy_never_commits_handoff_or_cleanup_recovery() {
    cancelled_initialize_route(serde_json::json!({"proxy":"missing.md"}),
        serde_json::json!({"proxy":"missing-cleanup.md"}));
}

#[test]
fn cancelled_initialize_error_never_dispatches_failure_catch() {
    cancelled_initialize_route(serde_json::json!({"error":"initialize failed"}),
        serde_json::json!({"error":"cleanup failed"}));
}
