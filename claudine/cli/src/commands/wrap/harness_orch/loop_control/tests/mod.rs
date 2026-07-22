//! Tests for harness-loop control and requeue fallback behavior.

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
        selection_hints: claudine::composition::EffectiveSelectionHints::default(),
        inline_closure_plan: None,
        file_resolution_context: None,
        live_frontmatter,
        runtime_state: std::sync::Arc::new(claudine::composition::RuntimeState::new()),
        lifecycle: None,
        mcp_body_tags: Vec::new(),
    }
}

/// A real [`ProxyCommitError::Resolution`] for a target that does not exist.
///
/// Built by driving `commit_proxy` rather than by constructing the variant, so
/// the authoring-context wrapper and its resolver cause are the ones production
/// exposes through rendering and `err.*`.
fn unresolvable_commit_error(source_path: &Path) -> claudine::composition::ProxyCommitError {
    let mut ledger = claudine::composition::RunLedger::new(
        source_path.to_path_buf(),
        claudine::composition::SharedApprovalCache::default(),
    );
    let request = claudine::composition::EvaluatedProxyRequest::new(
        source_path
            .parent()
            .expect("the fixture source lives in a directory")
            .join("no-such-target.md")
            .display()
            .to_string(),
        indexmap::IndexMap::new(),
        claudine::composition::ProxyProvenance::new(
            source_path.to_path_buf(),
            claudine::composition::ActionLocation::new(LifecycleSignal::Failure, 0, 0),
            vec![source_path.to_path_buf()],
        ),
    );
    claudine::composition::commit_proxy(&mut ledger, request, source_path.parent())
        .expect_err("a target that does not exist cannot be committed")
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
        runtime_state: std::sync::Arc::new(claudine::composition::RuntimeState::new()),
        suppress_output_commit: false,
        last_final_output: None,
        input_layers: Default::default(),
        entry: claudine::composition::DocumentEntryReason::Direct,
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

/// A run ledger seeded at `origin`, for dispatches that read the chain.
///
/// A default approval cache is correct here: these tests exercise transition
/// decisions, and no dispatch path reads the cache.
fn ledger(origin: &Path) -> claudine::composition::RunLedger {
    claudine::composition::RunLedger::new(origin.to_path_buf(), Default::default())
}

/// A coordinator whose run originates at `origin`.
fn coordinator(origin: &Path) -> ActiveDocumentCoordinator {
    ActiveDocumentCoordinator::new(origin.to_path_buf(), Default::default())
}

fn dispatch_guard<'a>(
    config: &'a LifecycleConfig,
    ctx: &'a LifecycleRuntimeContext<'a>,
    emitter: &'a RecordingEmitter,
) -> LifecycleRunGuard<'a> {
    LifecycleRunGuard::new(config, ctx, emitter)
}


mod active_state_wiring;
mod budget_scoping;
mod coordinator_adoption;
mod lifecycle_ordering;
mod overlay_layering;
mod proxy;
mod recovery_identity;
mod requeue;
mod retry_resume;
mod shell_approval;
mod terminal_evaluation;
mod terminal_routing;
mod unowned_handoff;
