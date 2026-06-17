//! Pre-flight: the approval-set lifecycle.
//!
//! Pre-flight has two clearly separated responsibilities — **approval** and
//! **execution** — and this module owns the approval half:
//!
//! - [`collect`] performs a condition-**blind** walk of the document graph
//!   (frontmatter `$(...)`, body `::shell`/`::shell-block`, and transcluded
//!   children) and returns every command that *could* execute under *any*
//!   document state.
//! - [`approval`] turns that collection into the deduped *approval set* the
//!   orchestrator boundary hands back as the execution membership source.
//!
//! Condition-**aware** execution stays in the inline shell-expansion stage,
//! gated by `ComposeOptions::pre_approved_commands`. The governing invariant is
//! `execution_set ⊆ approval_set`: because approval is a superset of anything
//! reachable under any state, the execution-time gate degrades to a pure
//! membership check that never prompts.
//!
//! See [`darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md`] for the
//! full design.

pub mod approval;
pub mod collect;

pub use approval::approval_set;
pub use collect::collect_shell_commands;

use crate::markdown::Markdown;
use crate::markdown::compose::{ComposeOptions, ComposeWarning};
use crate::markdown::compose::shell_expansion::types::ShellCommandEntry;
use crate::markdown::types::MarkdownResult;

/// The result of the document-side pre-flight collection.
///
/// Carries the deduped, condition-blind approval candidates discovered across
/// the document graph plus any warnings raised during collection. The caller
/// (e.g. Claudine) merges its own harness commands, authorizes the union, and
/// hands the approved set back to the pipeline through
/// [`ComposeOptions::with_pre_approved_commands`].
#[derive(Debug, Clone, Default)]
pub struct ComposePreflightReport {
    /// Deduped source entries, one per unique normalized command, in the order
    /// they were first discovered.
    pub entries: Vec<ShellCommandEntry>,
    /// Non-fatal warnings raised during collection.
    pub warnings: Vec<ComposeWarning>,
}

impl ComposePreflightReport {
    /// Returns the deduped normalized approval set, in discovery order.
    pub fn approval_set(&self) -> Vec<String> {
        approval::approval_set(&self.entries)
    }

    /// Returns `true` when no commands were discovered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Markdown {
    /// Runs the document-side pre-flight collection without executing anything.
    ///
    /// Walks the document graph condition-blind and returns a
    /// [`ComposePreflightReport`] with the approval candidates. No approval
    /// checks, whitelist lookups, or shell execution occur; the caller owns
    /// authorization.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    /// use darkmatter::markdown::compose::ComposeOptions;
    ///
    /// let md: Markdown = "::shell echo hello\n".into();
    /// let report = md.compose_preflight(&ComposeOptions::new()).unwrap();
    /// assert_eq!(report.approval_set(), vec!["echo hello".to_string()]);
    /// ```
    ///
    /// ## Errors
    ///
    /// Propagates collection failures (interpolation/transclusion resolution,
    /// `::shell` parse errors, and `DynamicCommandShape`).
    pub fn compose_preflight(
        &self,
        options: &ComposeOptions,
    ) -> MarkdownResult<ComposePreflightReport> {
        let entries = collect::collect_shell_commands(self, options)?;
        Ok(ComposePreflightReport {
            entries,
            warnings: Vec::new(),
        })
    }
}

#[cfg(test)]
mod acceptance_tests {
    //! Acceptance tests for the approval/execution split (tech-design T2–T4).
    //!
    //! T1 (condition-blind collection) and T10 (`DynamicCommandShape`) live in
    //! [`collect`]; these exercise the execution side: dead branches never run,
    //! the `execution_set ⊆ approval_set` invariant holds across states, and a
    //! single approval set is loop-stable.

    use std::collections::HashSet;

    use crate::markdown::Markdown;
    use crate::markdown::compose::{ComposeOperation, ComposeOptions};
    use serde_json::json;
    use tempfile::TempDir;

    fn approval_set(md: &Markdown) -> HashSet<String> {
        md.compose_preflight(&ComposeOptions::new())
            .unwrap()
            .approval_set()
            .into_iter()
            .collect()
    }

    fn execute_options(approval: HashSet<String>, policy_root: &std::path::Path) -> ComposeOptions {
        ComposeOptions::new()
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::PageBlocks,
                ComposeOperation::ShellExpansion,
            ])
            .with_shell_policy_root(policy_root)
            .with_pre_approved_commands(approval)
    }

    /// T2: a dead-branch side-effecting command is approved but never executes.
    #[test]
    fn dead_branch_is_approved_but_not_executed() {
        let content = "\
---
cleanup: false
---
::block when=\"cleanup\"
::shell echo PURGED
::end-block
::shell echo ALWAYS
";
        let md: Markdown = content.into();

        let approval = approval_set(&md);
        // Condition-blind: the dead branch is in the approval set.
        assert!(approval.contains("echo PURGED"), "approval: {approval:?}");
        assert!(approval.contains("echo ALWAYS"), "approval: {approval:?}");

        let temp = TempDir::new().unwrap();
        let (composed, _report) = md
            .compose_with(execute_options(approval, temp.path()))
            .unwrap();

        assert!(composed.content().contains("ALWAYS"));
        // The pruned dead branch never reached the shell stage, so even though
        // it was approved, its output never appears.
        assert!(
            !composed.content().contains("PURGED"),
            "dead branch executed: {}",
            composed.content()
        );
    }

    /// T3: `execution_set ⊆ approval_set` across flipped condition states — no
    /// state reaches a `NotPreApproved` miss.
    #[test]
    fn execution_is_a_subset_of_approval_across_states() {
        let content = "\
---
flag: a
---
::block when=\"flag == 'a'\"
::shell echo branch-a
::end-block
::block when=\"flag == 'b'\"
::shell echo branch-b
::end-block
";
        let md: Markdown = content.into();
        let approval = approval_set(&md);
        assert!(approval.contains("echo branch-a"), "approval: {approval:?}");
        assert!(approval.contains("echo branch-b"), "approval: {approval:?}");

        let temp = TempDir::new().unwrap();
        for flag in ["a", "b"] {
            let options =
                execute_options(approval.clone(), temp.path()).with_set_overrides(json!({ "flag": flag }));
            let (composed, _) = md
                .compose_with(options)
                .unwrap_or_else(|e| panic!("flag={flag} should not miss approval: {e}"));
            if flag == "a" {
                assert!(composed.content().contains("branch-a"));
                assert!(!composed.content().contains("branch-b"));
            } else {
                assert!(composed.content().contains("branch-b"));
                assert!(!composed.content().contains("branch-a"));
            }
        }
    }

    /// T4: a single up-front approval set is loop-stable — re-composing across a
    /// flipped condition issues zero new prompts.
    #[test]
    fn approval_set_is_loop_stable() {
        let content = "\
---
flag: a
---
::block when=\"flag == 'a'\"
::shell echo branch-a
::end-block
::block when=\"flag == 'b'\"
::shell echo branch-b
::end-block
";
        let md: Markdown = content.into();
        // Computed exactly once, then reused for every iteration.
        let approval = approval_set(&md);

        let temp = TempDir::new().unwrap();
        for flag in ["a", "b", "a"] {
            let options =
                execute_options(approval.clone(), temp.path()).with_set_overrides(json!({ "flag": flag }));
            let (_, report) = md.compose_with(options).unwrap();
            // The pre-approved membership fast-path never prompts.
            assert_eq!(
                report.shell_approvals_used, 0,
                "iteration flag={flag} prompted unexpectedly"
            );
        }
    }
}
