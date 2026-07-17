//! The `agent-errors` fleet prompt (`docs/research/agent-errors/_fleet.md`)
//! wires the spec-D10 deterministic recovery policy into its lifecycle: one
//! shared resume budget repairs missing output, research-document gate errors,
//! and findings; a separate retry budget handles transient provider failures.
//! Infrastructure and protocol failures stay fail-closed, and a clean-only
//! `finalize` guard prevents exhausted remediation from turning a durable
//! failing outcome into fleet success. These tests parse and execute the real
//! committed lifecycle through the production machinery.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use biscuit_terminal::terminal::Terminal;
use claudine::composition::{
    ControlDispatch, DefaultLifecycleEmitter, LifecycleActionKind, LifecycleControlAction,
    LifecycleSignal, RetryBackoff, ShellRunError, ShellRunner, StackControl, StackExecutionContext,
    control_budget_for, decide_control, parse_lifecycle_config,
};
use claudine::events::GlobalSettings;
use claudine::messaging::RuntimeMessagingSettings;
use darkmatter::effects::EffectEngine;
use darkmatter::markdown::Markdown;
use serde_json::json;

/// The committed fleet prompt under the claudine package area.
fn fleet_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("lib crate lives under the claudine package area")
        .join("docs/research/agent-errors/_fleet.md")
}

/// Reads the fleet prompt's raw frontmatter as a JSON value (interpolation
/// spans stay literal — the lifecycle parser defers them by design).
fn fleet_frontmatter() -> serde_json::Value {
    let path = fleet_path();
    let md = Markdown::try_from(path.as_path()).expect("parse _fleet.md");
    serde_json::to_value(md.frontmatter().as_map()).expect("frontmatter to json")
}

#[test]
fn fleet_lifecycle_config_parses() {
    let frontmatter = fleet_frontmatter();
    let config = parse_lifecycle_config(&frontmatter, &fleet_path())
        .expect("the agent-errors fleet lifecycle must parse");

    let success = config
        .stack(LifecycleSignal::Success)
        .expect("success stack present");

    // Missing-output resume, gate shell, missing report, three gate-error
    // scopes, unknown status, findings resume, and clean report.
    assert_eq!(
        success.len(),
        9,
        "expected the nine-item recovery-aware success stack"
    );

    // Report persistence failures must stop the stack before stale report
    // conditions can run.
    let shell_actions: Vec<_> = success
        .iter()
        .flat_map(|item| &item.actions)
        .filter(|action| matches!(action.kind, LifecycleActionKind::Shell(_)))
        .collect();
    assert_eq!(shell_actions.len(), 1, "exactly one gate shell is present");
    assert!(
        !shell_actions[0].no_error,
        "the gate shell must fail closed on report persistence errors"
    );

    let conditions: Vec<String> = success
        .iter()
        .filter_map(|item| item.when.as_ref().map(ToString::to_string))
        .collect();
    for status in ["clean", "findings", "gate_error"] {
        assert!(
            conditions
                .iter()
                .any(|condition| condition.contains("frontmatter(findings")
                    && condition.contains("status")
                    && condition.contains("==")
                    && condition.contains(status)),
            "success stack must branch explicitly on {status}; got {conditions:?}"
        );
    }
    assert!(
        conditions.iter().any(|condition| condition == "!file_exists(findings)"),
        "a missing report must be an explicit failure branch"
    );
    assert!(conditions.iter().any(|condition| {
        condition.contains("error_scope") && condition.contains("research_document")
    }));
    assert!(conditions
        .iter()
        .any(|condition| condition.contains("error_scope") && condition.contains("gate_input")));

    let missing_report_item = success
        .iter()
        .find(|item| {
            item.when
                .as_ref()
                .is_some_and(|condition| condition.to_string() == "!file_exists(findings)")
        })
        .expect("missing-report branch present");
    assert!(matches!(
        missing_report_item.actions.last().map(|action| &action.kind),
        Some(LifecycleActionKind::LifecycleControl(
            LifecycleControlAction::Error { .. }
        ))
    ));

    // All three agent-correctable postconditions resume, and each control is
    // terminal in its stack item so later status branches cannot also fire.
    let resume_items: Vec<_> = success
        .iter()
        .filter(|item| {
            item.actions.iter().any(|a| {
                matches!(&a.kind, LifecycleActionKind::LifecycleControl(c) if c.verb() == "resume")
            })
        })
        .collect();
    assert_eq!(resume_items.len(), 3, "missing output, document error, and findings resume");
    for item in resume_items {
        let last = item.actions.last().expect("resume stack item is non-empty");
        assert!(
            matches!(&last.kind, LifecycleActionKind::LifecycleControl(c) if c.verb() == "resume"),
            "resume must be the last action in its stack item"
        );
        let LifecycleActionKind::LifecycleControl(LifecycleControlAction::Resume {
            max_attempts,
            ..
        }) = &last.kind
        else {
            unreachable!("the terminal control was asserted as resume")
        };
        assert_eq!(
            max_attempts.as_ref().map(ToString::to_string).as_deref(),
            Some("2"),
            "all remediation branches must share the two-additional-turn ceiling"
        );
    }

    let failure = config
        .stack(LifecycleSignal::Failure)
        .expect("failure recovery stack present");
    assert_eq!(failure.len(), 2, "timeout resume plus transient retry");
    let timeout_control = failure[0].actions.last().expect("timeout control");
    let LifecycleActionKind::LifecycleControl(LifecycleControlAction::Resume {
        max_attempts,
        ..
    }) = &timeout_control.kind
    else {
        panic!("timeout recovery must resume")
    };
    assert_eq!(max_attempts.as_ref().map(ToString::to_string).as_deref(), Some("2"));

    let transient_control = failure[1].actions.last().expect("transient control");
    let LifecycleActionKind::LifecycleControl(LifecycleControlAction::Retry {
        max_attempts,
        backoff,
        delay,
    }) = &transient_control.kind
    else {
        panic!("transient recovery must retry")
    };
    assert_eq!(max_attempts.as_ref().map(ToString::to_string).as_deref(), Some("2"));
    assert_eq!(*backoff, Some(RetryBackoff::Exponential));
    assert_eq!(
        delay.as_ref().map(ToString::to_string).as_deref(),
        Some("\"30s\"")
    );
}

/// Gate double that deliberately leaves the research document in findings on
/// every lifecycle attempt while durably replacing the same outcome report.
struct PersistentFindingsGate {
    report_path: PathBuf,
    runs: AtomicU32,
}

impl ShellRunner for PersistentFindingsGate {
    fn run(&self, command: &str) -> Result<i32, ShellRunError> {
        self.runs.fetch_add(1, Ordering::Relaxed);
        std::fs::write(
            &self.report_path,
            "---\nstatus: findings\nprovider: codex\nfindings:\n  - invalid research remains\n---\n",
        )
        .map_err(|source| ShellRunError::Spawn {
            command: command.to_string(),
            source,
        })?;
        Ok(0)
    }
}

#[test]
fn exhausted_remediation_fails_finalize_and_preserves_findings() {
    let dir = tempfile::tempdir().expect("temp fleet area");
    let research_path = dir.path().join("codex.md");
    let findings_path = dir.path().join("codex-findings.md");
    std::fs::write(&research_path, "invalid research across every attempt\n")
        .expect("write invalid research fixture");

    let mut frontmatter = fleet_frontmatter();
    let root = frontmatter
        .as_object_mut()
        .expect("fleet frontmatter is an object");
    root.insert(
        "file".to_string(),
        json!(research_path.to_string_lossy().into_owned()),
    );
    root.insert(
        "findings".to_string(),
        json!(findings_path.to_string_lossy().into_owned()),
    );
    root.insert(
        "state".to_string(),
        json!({"name": "Codex", "slug": "codex"}),
    );

    // The fixture begins immediately after the provider claims success. The
    // gate itself and every durable-status branch remain the authored fleet
    // lifecycle; only the unrelated file-date precondition is omitted.
    root.get_mut("success")
        .and_then(|success| success.get_mut("stack"))
        .and_then(serde_json::Value::as_array_mut)
        .expect("success stack is an array")
        .remove(0);

    let config = parse_lifecycle_config(&frontmatter, &fleet_path())
        .expect("materialized fleet lifecycle parses");
    let frontmatter = frontmatter
        .as_object()
        .expect("materialized frontmatter is an object");
    let effect_engine = EffectEngine::builder()
        .mutation_root(dir.path())
        .auto_rehash(false)
        .build();
    let gate = PersistentFindingsGate {
        report_path: findings_path.clone(),
        runs: AtomicU32::new(0),
    };
    let emitter = DefaultLifecycleEmitter;
    let term = Terminal::default();
    let settings = GlobalSettings::default();
    let messaging = RuntimeMessagingSettings {
        user: None,
        repo: None,
    };
    let source_path = dir.path().join("_fleet.md");
    let context = |signal| StackExecutionContext {
        signal,
        frontmatter,
        live_frontmatter: None,
        err: None,
        timing: None,
        current: None,
        base_dir: Some(dir.path()),
        ctx_base_dir: Some(dir.path()),
        prepared_context: None,
        effect_engine: &effect_engine,
        shell_runner: &gate,
        emitter: &emitter,
        term: &term,
        source_path: &source_path,
        repo_root: Some(dir.path()),
        messaging: &messaging,
        settings: &settings,
    };

    let mut resume_budget = None;
    for attempt in 1..=3 {
        let outcome = context(LifecycleSignal::Success).execute_event(&config);
        assert!(outcome.action_error.is_none(), "attempt {attempt} gate failed");
        assert!(
            outcome.evaluation_error.is_none(),
            "attempt {attempt} lifecycle evaluation failed"
        );
        let control = outcome.control.expect("findings must request a resume");
        let max_attempts = match &control {
            StackControl::Resume { max_attempts, .. } => *max_attempts,
            other => panic!("expected resume control, got {other:?}"),
        };
        let budget = *resume_budget.get_or_insert_with(|| control_budget_for(attempt, max_attempts));
        let dispatch = decide_control(&control, attempt, budget, true, true);
        if attempt < 3 {
            assert!(
                matches!(dispatch, ControlDispatch::Resume { .. }),
                "attempt {attempt} must resume"
            );
        } else {
            assert_eq!(dispatch, ControlDispatch::Exhausted);
        }
    }
    assert_eq!(gate.runs.load(Ordering::Relaxed), 3);

    let report_before_finalize = std::fs::read_to_string(&findings_path)
        .expect("findings report survives all gate attempts");
    let finalize = context(LifecycleSignal::Finalize).execute_event(&config);
    assert!(finalize.action_error.is_none());
    assert!(finalize.evaluation_error.is_none());
    assert!(
        matches!(
            finalize.control,
            Some(StackControl::Error { ref reason })
                if reason.as_deref() == Some("deterministic gate did not reach a clean outcome")
        ),
        "a non-clean durable outcome must make finalization non-successful: {finalize:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&findings_path).expect("findings report remains readable"),
        report_before_finalize,
        "finalize must preserve the machine-readable findings report"
    );
}
