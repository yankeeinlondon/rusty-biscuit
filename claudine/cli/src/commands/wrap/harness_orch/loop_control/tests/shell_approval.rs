//! Per-target shell discovery and approval (R9).
//!
//! The contract these lock: a freeze is scoped to the document that earned it.
//! A proxy target gets the same discovery and approval opportunity as a direct
//! invocation, while an exact command already approved anywhere in the
//! invocation is never re-prompted.

use super::*;
use crate::commands::wrap::harness_orch::CachedHarnessLoopContext;
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
};
use std::sync::Arc;

/// Approval handler that records every command it is asked about and approves
/// each one.
///
/// Recording is the whole point: "did the target get a chance to ask?" and
/// "was the source's approval reused?" are both questions about the prompt
/// count, which a silently-auto-approving handler cannot answer.
#[derive(Default)]
struct RecordingApprovalHandler {
    prompted: Mutex<Vec<String>>,
}

impl RecordingApprovalHandler {
    fn prompted(&self) -> Vec<String> {
        self.prompted.lock().unwrap().clone()
    }
}

impl ShellApprovalHandler for RecordingApprovalHandler {
    fn approve(
        &self,
        request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, ShellExpansionError> {
        self.prompted.lock().unwrap().push(request.normalized_exact);
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

fn recording_context(
    source: &Path,
    repo_root: Option<&Path>,
) -> (CachedHarnessLoopContext, Arc<RecordingApprovalHandler>) {
    let handler = Arc::new(RecordingApprovalHandler::default());
    let options = claudine::harness::ShellApprovalOptions {
        policy_root: repo_root.map(Path::to_path_buf),
        approval_handler: Some(handler.clone()),
        ..Default::default()
    };
    (
        CachedHarnessLoopContext::with_shell_options(source, repo_root, options),
        handler,
    )
}

fn has_handler(context: &CachedHarnessLoopContext) -> bool {
    context.shell_options().approval_handler.is_some()
}

/// A freeze closes the current document's approval window, and a hand-off to a
/// different document reopens it.
///
/// Before this, `freeze_shell_approvals` dropped the handler for the whole
/// invocation. Adoption resets the attempt counter to 1, so the freeze *ran*
/// again for the target — against an already-`None` handler. A target-only
/// command was therefore denied without ever being offered for review.
#[test]
fn a_hand_off_to_a_new_document_reopens_the_approval_window() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("router.md");
    let target = dir.path().join("target.md");
    let (mut context, _handler) = recording_context(&source, Some(dir.path()));

    assert!(has_handler(&context), "the run launches able to prompt");

    context.freeze_shell_approvals();
    assert!(
        !has_handler(&context),
        "the freeze closes the source document's approval window"
    );

    context.refresh(&target, Some(dir.path()));
    assert!(
        has_handler(&context),
        "a proxy target must get the same approval opportunity as a direct invocation",
    );
}

/// Retry and resume re-read the same document mid-run; the freeze is the point
/// there, so it must survive.
///
/// The distinction is TTY contention plus the spec contract that approvals
/// resolve before the provider workflow begins. A hand-off happens between
/// child processes; a retry happens behind one.
#[test]
fn a_retry_or_resume_of_the_same_document_keeps_the_freeze() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("prompt.md");
    let (mut context, _handler) = recording_context(&source, Some(dir.path()));

    context.freeze_shell_approvals();
    context.refresh(&source, Some(dir.path()));

    assert!(
        !has_handler(&context),
        "re-reading the same document must not reopen the window a freeze closed",
    );
}

/// The thaw restores the run's *original* handler, not an arbitrary one — a
/// non-TTY run that launched without a handler must never gain one at a
/// hand-off.
#[test]
fn a_run_that_cannot_prompt_gains_no_handler_at_a_hand_off() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("router.md");
    let target = dir.path().join("target.md");
    let options = claudine::harness::ShellApprovalOptions {
        policy_root: Some(dir.path().to_path_buf()),
        approval_handler: None,
        ..Default::default()
    };
    let mut context =
        CachedHarnessLoopContext::with_shell_options(&source, Some(dir.path()), options);

    context.freeze_shell_approvals();
    context.refresh(&target, Some(dir.path()));

    assert!(
        !has_handler(&context),
        "a non-TTY run has no handler to restore; the hand-off must not invent one",
    );
}

/// The thaw is repeatable across a multi-hop chain: every document in the
/// chain earns its own window.
#[test]
fn every_document_in_a_chain_gets_its_own_approval_window() {
    let dir = tempfile::tempdir().unwrap();
    let (mut context, _handler) = recording_context(&dir.path().join("a.md"), Some(dir.path()));

    for hop in ["b.md", "c.md"] {
        context.freeze_shell_approvals();
        context.refresh(&dir.path().join(hop), Some(dir.path()));
        assert!(
            has_handler(&context),
            "hop '{hop}' must be able to request approval for its own commands",
        );
    }
}

/// A target-only command is offered for review even though the source's
/// commands were already approved and frozen — and an exact command the source
/// already approved is *not* re-prompted at the target.
///
/// Both halves of R9's named acceptance case, on one invocation-wide cache.
#[test]
fn a_target_only_command_prompts_while_a_shared_command_reuses_the_approval() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("router.md");
    let target = dir.path().join("target.md");
    // Both documents run `echo shared`; only the target runs `echo target-only`.
    std::fs::write(
        &source,
        "---\nshared: \"$(echo shared)\"\n---\nrouting\n",
    )
    .unwrap();
    std::fs::write(
        &target,
        "---\nshared: \"$(echo shared)\"\nonly: \"$(echo target-only)\"\n---\nrun\n",
    )
    .unwrap();

    let (mut context, handler) = recording_context(&source, Some(dir.path()));

    let mut source_state = prompt_state(&source);
    preflight_proxy_target(&mut source_state, context.shell_options(), dir.path())
        .expect("the source's own command is approved by the recording handler");
    assert_eq!(
        handler.prompted(),
        vec!["echo shared".to_string()],
        "the source prompts once for its own command",
    );

    // The source's window closes, then the run hands off.
    context.freeze_shell_approvals();
    context.refresh(&target, Some(dir.path()));

    let mut target_state = prompt_state(&target);
    preflight_proxy_target(&mut target_state, context.shell_options(), dir.path())
        .expect("the target's new command must reach the approval handler, not a silent denial");

    assert_eq!(
        handler.prompted(),
        vec!["echo shared".to_string(), "echo target-only".to_string()],
        "the target prompts for its own new command and reuses the cached `echo shared`",
    );

    let approved = target_state
        .input_layers
        .pre_approved_commands
        .as_ref()
        .expect("the target's approvals are folded into the carried input layers");
    assert!(
        approved.contains("echo target-only"),
        "the target-only command is approved for execution; got {approved:?}",
    );
}

/// A `with:` value that influences a command must be present at approval, so
/// the approved bytes are the bytes that execute.
///
/// The overlay merges into the authored frontmatter *before* the audit reads
/// it, which is what makes the approved string the resolved one rather than
/// the unresolved `$(echo {{ flag }})` template.
#[test]
fn approved_bytes_equal_the_bytes_a_with_value_resolves_to() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target.md");
    std::fs::write(
        &target,
        "---\nflag: authored\nprobe: \"$(echo {{ flag }})\"\n---\n{{ probe }}\n",
    )
    .unwrap();

    let (context, handler) = recording_context(&target, Some(dir.path()));

    let mut state = prompt_state(&target);
    state.overlay.insert(
        "flag".to_string(),
        serde_json::Value::String("from-overlay".to_string()),
    );

    preflight_proxy_target(&mut state, context.shell_options(), dir.path())
        .expect("the overlay-influenced command is approved");

    assert_eq!(
        handler.prompted(),
        vec!["echo from-overlay".to_string()],
        "the audit must see the overlay-resolved command, not the authored default \
         and not an unresolved template",
    );

    // ...and the bytes that execute are the bytes that were approved.
    let materialized = materialize_harness_prompt(
        &state,
        None,
        dir.path(),
        None,
        claudine::composition::SchemaStage::Validate,
    )
        .expect("the approved command expands at execution");
    assert_eq!(
        materialized.prompt.trim(),
        "from-overlay",
        "the executed command produced the overlay-resolved output",
    );
}
