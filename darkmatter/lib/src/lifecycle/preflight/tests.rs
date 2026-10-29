use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use serde_json::{Map, Value, json};

use super::*;
use crate::lifecycle::actions::{LifecycleActionKind, for_each_action};
use crate::lifecycle::registry::EventRegistry;
use crate::lifecycle::{
    LifecycleConfig, LifecycleParseOptions, LifecycleSignal, UnknownVerbPolicy, parse_lifecycle_config_with,
};
use crate::markdown::compose::expression::{ResolutionContext, ScopeId};
use crate::markdown::compose::shell_expansion::template::{HoleTiming, ModeRequirement};
use crate::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest};

const SOURCE: &str = "doc.md";

fn plan() -> PreflightPlan {
    PreflightPlan::new()
        .register(CheckSpec { name: "agent" })
        .and_then(|plan| plan.register(CheckSpec { name: "schema" }))
        .and_then(|plan| plan.register(CheckSpec { name: "shells" }))
        .expect("three distinct checks")
}

fn info() -> LifecycleErrorInfo {
    LifecycleErrorInfo {
        kind: "HostCheck",
        variant: "agent".to_string(),
        msg: "the agent did not resolve".to_string(),
        snapshot: None,
        property: None,
        reason: Default::default(),
        cause: None,
    }
}

#[test]
fn a_check_name_registers_once() {
    let error = plan().register(CheckSpec { name: "schema" }).expect_err("a duplicate");
    assert_eq!(error, PreflightRegistrationError::DuplicateCheck("schema"));
}

#[test]
fn a_run_reaches_checks_in_registration_order_and_may_pass_over_one() {
    let plan = plan();
    let mut run = plan.begin(EntryReason::Initial);
    assert_eq!(run.advance("agent"), Ok(()));
    assert_eq!(run.advance("shells"), Ok(()), "a check with nothing to do may be passed over");
    assert_eq!(run.advance("schema"), Err(PreflightOrderError::OutOfOrder { check: "schema", after: "shells" }));
    assert_eq!(run.advance("shells"), Err(PreflightOrderError::OutOfOrder { check: "shells", after: "shells" }));
    assert_eq!(run.advance("model"), Err(PreflightOrderError::Unregistered("model")));
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "runs after `shells`")]
fn a_check_run_out_of_order_fails_a_debug_assertion() {
    let plan = plan();
    let mut run = plan.begin(EntryReason::Initial);
    run.skip("shells");
    let _ = run.check("agent", CheckStage::BeforeRun, || Ok::<_, String>(()));
}

#[test]
fn a_passing_walk_is_ready_at_its_epoch() {
    let plan = plan();
    let mut run = plan.begin(EntryReason::Retry);
    assert_eq!(run.check("agent", CheckStage::BeforeRun, || Ok::<_, String>("claude")).unwrap(), "claude");
    run.skip("schema");
    run.check("shells", CheckStage::InRun, || Ok::<_, String>(())).unwrap();
    assert_eq!(run.ready(ApprovalEpoch(7)), Preparation::Ready { epoch: ApprovalEpoch(7) });
}

#[test]
fn a_failure_before_the_run_is_not_a_host_failure() {
    let plan = plan();
    let mut run = plan.begin(EntryReason::Initial);
    let failure = run.check("agent", CheckStage::BeforeRun, || Err::<(), _>("ambiguous agent")).unwrap_err();
    assert_eq!((failure.check, failure.stage, failure.error), ("agent", CheckStage::BeforeRun, "ambiguous agent"));
    assert_eq!(failure.host_failure(info()), None);
}

#[test]
fn a_failure_inside_the_run_is_a_preflight_host_failure_of_its_entry() {
    let plan = plan();
    let mut run = plan.begin(EntryReason::Resume);
    let failure = run.check("shells", CheckStage::InRun, || Err::<(), _>("denied")).unwrap_err();
    assert_eq!(
        failure.host_failure(info()),
        Some(HostFailure { stage: FailureStage::Preflight, entry: EntryReason::Resume, error: info() })
    );
}

// ── the lifecycle shell walk ────────────────────────────────────────────

/// A catalog with no late roots: every read resolves from the document.
struct NoLateRoots;

impl ShellAuditBindings for NoLateRoots {
    fn approval_findings(&self, _raw: &str) -> Vec<ExpressionError> {
        Vec::new()
    }
    fn approval_bindings(&self) -> (Arc<BindingView>, HashMap<String, RuntimeBinding<'static>>) {
        let view = BindingView::builder(ScopeId::new("test.approval")).build().expect("an empty view");
        (Arc::new(view), HashMap::new())
    }
    fn task_setup_bindings(
        &self,
        _group: Option<&Map<String, Value>>,
        _seq: Option<&Value>,
    ) -> (Arc<BindingView>, HashMap<String, RuntimeBinding<'static>>) {
        self.approval_bindings()
    }
    fn loop_pending(&self) -> bool {
        false
    }
}

fn parse(frontmatter: &Value) -> LifecycleConfig {
    let options = LifecycleParseOptions {
        is_known_effect: &|_| true,
        authored: None,
        unknown_verbs: UnknownVerbPolicy::Reject,
        registry: &EventRegistry::md(),
    };
    parse_lifecycle_config_with(frontmatter, Path::new(SOURCE), &options).expect("a valid lifecycle")
}

fn resolution() -> Result<ResolutionContext, ContextBuildError> {
    let options = ComposeOptions::new_with_context(ComposeContext::capture_minimal()).with_source_file(SOURCE);
    ComposeRequest::with_context(options, biscuit_file::FileResolutionContext::new(std::env::temp_dir()))
        .map(|request| request.local_expression_resolution_context())
}

/// `frontmatter`'s lifecycle with every shell site fixed, as `md` would.
fn stamped(frontmatter: &Value) -> Result<LifecycleConfig, PreflightError> {
    let mut config = parse(frontmatter);
    let registry = EventRegistry::md();
    let context = ComposeContext::capture_minimal();
    let sites = LifecycleShellSites {
        effective_frontmatter: frontmatter,
        context: &context,
        source_path: Path::new(SOURCE),
        registry: &registry,
        owner_schema: None,
        bindings: &NoLateRoots,
        options: None,
    };
    sites.stamp(&mut config, resolution)?;
    Ok(config)
}

#[test]
fn the_audit_collects_every_lifecycle_shell_condition_blind() {
    let frontmatter = json!({
        "name": "docs",
        "composed": [
            { "when": "false", "then": [{ "shell": "echo never" }], "else": [{ "shell": "echo otherwise" }] },
            { "shell": "echo {{ name }}" },
        ],
        "failed": [{ "shell": "echo caught" }],
        "finalize": [{ "when": "name", "then": [{ "set": { "stamp": "$(date +%s)" } }] }],
    });
    let config = stamped(&frontmatter).expect("every site resolves");

    let audit = collect_lifecycle_shell_audit(&config, &LifecycleSignal::ALL, Path::new(SOURCE));

    let commands: Vec<&str> = audit.commands.iter().map(|(command, _)| command.as_str()).collect();
    for expected in ["echo never", "echo otherwise", "echo docs", "echo caught", "date +%s"] {
        assert!(commands.contains(&expected), "{expected} in {commands:?}");
    }
    let [candidate] = audit.candidates.as_slice() else { panic!("one placeholder site: {:?}", audit.candidates) };
    assert_eq!(candidate.authored_command, "echo {{ name }}");
    assert_eq!(candidate.resolved.as_deref(), Some(&["echo docs".to_string()][..]));
}

#[test]
fn the_audit_of_selected_events_collects_only_theirs() {
    let config = stamped(&json!({
        "composed": [{ "shell": "echo start" }],
        "finalize": [{ "shell": "echo end" }],
    }))
    .unwrap();

    let audit = collect_lifecycle_shell_audit(&config, &[LifecycleSignal::Finalize], Path::new(SOURCE));

    assert_eq!(audit.commands, vec![("echo end".to_string(), "finalize[0].shell.command".to_string())]);
}

#[test]
fn a_placeholder_a_guard_proves_present_is_approved_as_a_typed_template() {
    let config = stamped(&json!({
        "dirty": true,
        "composed": [{ "when": "dirty && message", "then": [{ "shell": "git commit -m {{ message }}" }] }],
    }))
    .expect("a guard-proven missing value passes preflight");

    let mut sites = Vec::new();
    for_each_action(config.stack(LifecycleSignal::Start), &mut |action| {
        if let LifecycleActionKind::Shell(shell) = &action.kind {
            sites.extend(shell.template.clone());
        }
    });
    let [site] = sites.as_slice() else { panic!("one site") };
    assert_eq!(site.resolved(), None, "no bytes stand in for the missing value");
    let guards: Vec<_> = site
        .template()
        .placeholders()
        .filter_map(|placeholder| match &placeholder.timing {
            HoleTiming::Guarded { guard, .. } => Some((placeholder.expression.clone(), guard.label().to_string())),
            _ => None,
        })
        .collect();
    assert_eq!(guards, vec![("message".to_string(), "composed[0].when".to_string())]);
    let audit = collect_lifecycle_shell_audit(&config, &LifecycleSignal::ALL, Path::new(SOURCE));
    assert!(audit.commands.is_empty(), "{:?}", audit.commands);
    let [candidate] = audit.candidates.as_slice() else { panic!("one site") };
    assert_eq!(candidate.requirement, ModeRequirement::TemplateRequired);
}

#[test]
fn an_unguarded_missing_placeholder_fails_preflight_naming_its_property() {
    let error = stamped(&json!({ "composed": [{ "shell": "echo {{ message }}" }] })).expect_err("a missing value");

    let PreflightError::LifecycleShellResolution { property, .. } = &error else { panic!("{error:?}") };
    assert_eq!(property, "composed[0].shell.command");
    assert!(error.to_string().contains("which is missing"), "{error}");
}
