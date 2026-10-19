//! The `agent-errors` fleet prompt (`docs/research/agent-errors/_research.md`)
//! wires the spec-D10 deterministic recovery policy into its lifecycle: one
//! shared resume budget repairs missing output, research-document gate errors,
//! and findings; a separate retry budget handles transient provider failures.
//! Infrastructure and protocol failures stay fail-closed, and a clean-only
//! `finalize` guard prevents exhausted remediation from turning a durable
//! failing outcome into fleet success. These tests parse and execute the real
//! committed lifecycle through the production machinery.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use biscuit_terminal::terminal::Terminal;
use claudine::composition::{
    ControlDispatch, DefaultLifecycleEmitter, LifecycleAction, LifecycleActionKind,
    LifecycleControlAction, LifecycleNode, LifecycleSignal, RetryBackoff, ShellRunError, ShellRunner, StackControl, StackExecutionContext,
    control_budget_for, decide_control, parse_lifecycle_config,
};
use claudine::events::GlobalSettings;
use claudine::messaging::RuntimeMessagingSettings;
use darkmatter::effects::EffectEngine;
use darkmatter::markdown::Markdown;
use serde_json::json;

/// The committed fleet prompt under the claudine package area.
fn fleet_path() -> PathBuf {
    biscuit_test_harness::manifest_dir!()
        .parent()
        .expect("lib crate lives under the claudine package area")
        .join("docs/research/agent-errors/_research.md")
}

/// Reads the fleet prompt's raw frontmatter as a JSON value (interpolation
/// spans stay literal — the lifecycle parser defers them by design).
fn fleet_frontmatter() -> serde_json::Value {
    let path = fleet_path();
    let md = Markdown::try_from(path.as_path()).expect("parse _research.md");
    serde_json::to_value(md.frontmatter().as_map()).expect("frontmatter to json")
}

/// The guard of a conditional item, or `None` for an unconditional action.
fn guard(node: &LifecycleNode) -> Option<String> {
    match node {
        LifecycleNode::Conditional(conditional) => Some(conditional.when.to_string()),
        LifecycleNode::Action(_) => None,
    }
}

/// The selected (`then`) body of a conditional item, or the action itself.
fn body(node: &LifecycleNode) -> Vec<&LifecycleAction> {
    match node {
        LifecycleNode::Conditional(conditional) => {
            assert!(
                conditional.otherwise.is_empty(),
                "the fleet lifecycle has no `else` branches"
            );
            conditional
                .then
                .iter()
                .map(|node| match node {
                    LifecycleNode::Action(action) => action,
                    LifecycleNode::Conditional(_) => panic!("the fleet nests no conditionals"),
                })
                .collect()
        }
        LifecycleNode::Action(action) => vec![action],
    }
}

fn is_verb(action: &LifecycleAction, verb: &str) -> bool {
    matches!(&action.kind, LifecycleActionKind::LifecycleControl(c) if c.verb() == verb)
}

#[test]
fn fleet_lifecycle_config_parses() {
    let frontmatter = fleet_frontmatter();
    let config = parse_lifecycle_config(&frontmatter, &fleet_path())
        .expect("the agent-errors fleet lifecycle must parse");

    let success = config.stack(LifecycleSignal::Success);

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
        .flat_map(body)
        .filter(|action| matches!(action.kind, LifecycleActionKind::Shell(_)))
        .collect();
    assert_eq!(shell_actions.len(), 1, "exactly one gate shell is present");
    assert!(
        guard(&success[1]).is_none(),
        "the gate shell runs unconditionally, second in the stack"
    );
    assert!(
        !shell_actions[0].no_error,
        "the gate shell must fail closed on report persistence errors"
    );

    let conditions: Vec<String> = success.iter().filter_map(guard).collect();
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
        .find(|item| guard(item).as_deref() == Some("!file_exists(findings)"))
        .expect("missing-report branch present");
    assert!(matches!(
        body(missing_report_item).last().map(|action| &action.kind),
        Some(LifecycleActionKind::LifecycleControl(
            LifecycleControlAction::Error { .. }
        ))
    ));

    // All three agent-correctable postconditions resume, and each control is
    // terminal in its branch so later status branches cannot also fire.
    let resume_items: Vec<_> = success
        .iter()
        .filter(|item| body(item).iter().any(|action| is_verb(action, "resume")))
        .collect();
    assert_eq!(resume_items.len(), 3, "missing output, document error, and findings resume");
    for item in resume_items {
        let body = body(item);
        let last = body.last().expect("resume branch is non-empty");
        assert!(
            is_verb(last, "resume"),
            "resume must be the last action in its branch"
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

    // `failure` reports first (warn, then message), then the two guarded
    // recovery branches.
    let failure = config.stack(LifecycleSignal::Failure);
    assert_eq!(failure.len(), 4, "two reports, timeout resume, transient retry");
    let reports: Vec<_> = failure[..2]
        .iter()
        .map(|node| match node {
            LifecycleNode::Action(LifecycleAction {
                kind: LifecycleActionKind::Communication(comm),
                ..
            }) => comm.channel.verb(),
            other => panic!("expected an unconditional report, got {other:?}"),
        })
        .collect();
    assert_eq!(reports, ["warn", "message"]);
    let timeout_guard = guard(&failure[2]).expect("the timeout branch is guarded");
    assert!(
        timeout_guard.contains("err.category") && timeout_guard.contains("timeout"),
        "{timeout_guard}"
    );
    assert_eq!(guard(&failure[3]).as_deref(), Some("err.is_transient"));

    let timeout_branch = body(&failure[2]);
    let timeout_control = timeout_branch.last().expect("timeout control");
    let LifecycleActionKind::LifecycleControl(LifecycleControlAction::Resume {
        max_attempts,
        ..
    }) = &timeout_control.kind
    else {
        panic!("timeout recovery must resume")
    };
    assert_eq!(max_attempts.as_ref().map(ToString::to_string).as_deref(), Some("2"));

    let transient_branch = body(&failure[3]);
    let transient_control = transient_branch.last().expect("transient control");
    let LifecycleActionKind::LifecycleControl(LifecycleControlAction::Retry {
        max_attempts,
        backoff,
        delay,
        with,
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
    assert!(with.is_empty(), "the transient retry carries no overlay");
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

    fn run_structured(
        &self,
        _chain: &darkmatter::markdown::compose::shell_expansion::template::FilledChain,
    ) -> Result<i32, ShellRunError> {
        unreachable!("the fleet gate's commands hold no placeholder")
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
    let seq = json!({"name": "Codex", "slug": "codex"});

    // The fixture begins immediately after the provider claims success. The
    // gate itself and every durable-status branch remain the authored fleet
    // lifecycle; only the unrelated file-date precondition is omitted.
    root.get_mut("success")
        .and_then(serde_json::Value::as_array_mut)
        .expect("success stack is a list")
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
    let source_path = dir.path().join("_research.md");
    let file_resolution_context = biscuit_file::FileResolutionContext::new(dir.path());
    let context = |signal| StackExecutionContext {
        seq: Some(&seq),
        signal,
        scope: None,
        frontmatter,
        live_frontmatter: None,
        runtime_state: None,
        err: None,
        timing: None,
        current: None,
        group: None,
        loop_values: None,
        base_dir: Some(dir.path()),
        ctx_base_dir: Some(dir.path()),
        prepared_context: None,
        file_resolution_context: &file_resolution_context,
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
