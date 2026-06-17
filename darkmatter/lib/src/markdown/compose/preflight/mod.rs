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
use crate::markdown::compose::shell_expansion::types::{ShellCommandEntry, ShellExpansionError};
use crate::markdown::types::MarkdownResult;
use std::path::PathBuf;

/// Reusable graph metadata for one document visited during pre-flight
/// collection.
///
/// The v2 design ([`tech-design.md`](../../../../features/2026-06-16-compose-pipe-2/tech-design.md)
/// §"Reusing the collection walk") calls for the pre-flight walk to cache the
/// graph shape it discovered — the resolved child paths/URLs, the discovered
/// shell entries attributed to each child, and the children themselves — so
/// the final transclusion stage does not have to re-parse directives and
/// re-resolve targets a second time. This struct is that cached shape for a
/// single document; the root's [`ComposePreflightReport::preflight_graph`]
/// is the full tree.
///
/// The body itself is **not** cached: the design deliberately keeps rendering
/// condition-aware at the normal pipeline point. The cached artifact is
/// *metadata about the graph*, not the rendered Markdown.
#[derive(Debug, Clone, Default)]
pub struct PreflightGraphNode {
    /// The resolved source of this document — a canonicalized local path or a
    /// remote URL. `None` when the document was constructed from in-memory
    /// content with no source (the typical CLI test case).
    pub source: Option<PathBuf>,
    /// Shell entries first discovered *in this document*, in discovery order.
    /// Entries from nested children are not duplicated here — they live in
    /// the child's [`Self::children`].
    pub entries: Vec<ShellCommandEntry>,
    /// Nested transclusion children, in the order they were discovered.
    pub children: Vec<PreflightGraphNode>,
}

impl PreflightGraphNode {
    /// Returns `true` when this node has no entries and no children.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.children.is_empty()
    }

    /// Walks this node and every descendant, yielding one entry per unique
    /// normalized command in discovery order.
    pub fn flattened_entries(&self) -> Vec<&ShellCommandEntry> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        self.collect_unique(&mut out, &mut seen);
        out
    }

    fn collect_unique<'a>(
        &'a self,
        out: &mut Vec<&'a ShellCommandEntry>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        for entry in &self.entries {
            if seen.insert(entry.normalized.clone()) {
                out.push(entry);
            }
        }
        for child in &self.children {
            child.collect_unique(out, seen);
        }
    }
}

/// The result of the document-side pre-flight collection.
///
/// Carries the deduped, condition-blind approval candidates discovered across
/// the document graph plus any warnings raised during collection. The caller
/// (e.g. Claudine) merges its own harness commands, authorizes the union, and
/// hands the approved set back to the pipeline through
/// [`ComposeOptions::with_pre_approved_commands`].
///
/// The [`preflight_graph`](Self::preflight_graph) field carries the hierarchical
/// transclusion shape the collector walked (resolved child sources, per-child
/// shell entries, and the nested child tree). It is the reusable artifact
/// that lets a future integration skip re-discovering the graph during
/// transclusion — see the v2 design § "Reusing the collection walk".
#[derive(Debug, Clone, Default)]
pub struct ComposePreflightReport {
    /// Deduped source entries, one per unique normalized command, in the order
    /// they were first discovered.
    pub entries: Vec<ShellCommandEntry>,
    /// Non-fatal warnings raised during collection.
    pub warnings: Vec<ComposeWarning>,
    /// Hierarchical graph metadata for the walked document tree. The root node
    /// represents the source document passed to
    /// [`Markdown::compose_preflight`](crate::markdown::Markdown::compose_preflight);
    /// its `children` are the documents it directly transcludes, and so on.
    pub preflight_graph: PreflightGraphNode,
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

/// Validates that every condition-blindly collected command is a member of the
/// caller-supplied pre-approved set, before any shell execution begins.
///
/// This is the up-front membership gate the v2 design places between schema
/// validation and frontmatter shell expansion (design step 4). It runs only
/// when the caller supplies [`ComposeOptions::with_pre_approved_commands`];
/// without it the legacy per-directive approval path is unchanged. Failing
/// here — before the first frontmatter `$(...)` executes — removes the
/// "execute an earlier command before discovering a later one needs approval"
/// failure mode.
pub(crate) fn validate_pre_approved(
    markdown: &Markdown,
    options: &ComposeOptions,
) -> MarkdownResult<()> {
    let Some(ref approved) = options.pre_approved_commands else {
        return Ok(());
    };

    let ctx = markdown.source_context_for_errors();
    let entries = collect::collect_shell_commands(markdown, options)?;
    for entry in &entries {
        if !approved.contains(&entry.normalized) {
            return Err(ShellExpansionError::NotPreApproved {
                ctx: Box::new(ctx),
                command: entry.raw_command.clone(),
                origin: entry.origin.clone(),
                source_desc: format!(" (in {})", entry.source_file.display()),
            }
            .into());
        }
    }
    Ok(())
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
        let (entries, preflight_graph) = collect::collect_shell_commands_with_graph(self, options)?;
        Ok(ComposePreflightReport {
            entries,
            warnings: Vec::new(),
            preflight_graph,
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

    /// Pre-flight validation runs before any shell execution: when a body
    /// command is missing from the pre-approved set, the compose fails before
    /// the frontmatter `$(...)` command executes.
    #[test]
    fn preflight_blocks_frontmatter_shell_when_body_unapproved() {
        let temp = TempDir::new().unwrap();
        let sentinel = temp.path().join("sentinel");
        let sentinel_str = sentinel.display().to_string();

        let content = format!(
            "---\nsentinel: \"$(touch {sentinel_str})\"\n---\n::shell echo body-cmd\n"
        );
        let md: Markdown = content.into();

        // Approve only the frontmatter command; the body command is unapproved.
        let approved: HashSet<String> = [format!("touch {sentinel_str}")]
            .into_iter()
            .collect();

        let options = ComposeOptions::new()
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::FrontmatterShellExpansion,
                ComposeOperation::ShellExpansion,
            ])
            .with_shell_policy_root(temp.path())
            .with_pre_approved_commands(approved);

        let result = md.compose_with(options);
        assert!(
            result.is_err(),
            "compose should fail because the body command is unapproved"
        );

        // The frontmatter `touch` must NOT have executed — pre-flight caught
        // the unapproved body command before any shell execution began.
        assert!(
            !sentinel.exists(),
            "frontmatter command executed before pre-flight caught the unapproved body command"
        );
    }

    /// When the pre-approved set covers every collected command, the frontmatter
    /// shell command executes normally and the compose succeeds.
    #[test]
    fn preflight_allows_frontmatter_shell_when_all_approved() {
        let temp = TempDir::new().unwrap();
        let sentinel = temp.path().join("sentinel");
        let sentinel_str = sentinel.display().to_string();

        let content = format!(
            "---\nsentinel: \"$(touch {sentinel_str})\"\n---\n::shell echo body-cmd\n"
        );
        let md: Markdown = content.into();

        // Approve both the frontmatter and body commands.
        let approved: HashSet<String> = [
            format!("touch {sentinel_str}"),
            "echo body-cmd".to_string(),
        ]
        .into_iter()
        .collect();

        let options = ComposeOptions::new()
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::FrontmatterShellExpansion,
                ComposeOperation::ShellExpansion,
            ])
            .with_shell_policy_root(temp.path())
            .with_pre_approved_commands(approved);

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(sentinel.exists(), "frontmatter command should have executed");
        assert!(composed.content().contains("body-cmd"));
        assert_eq!(report.shell_approvals_used, 0);
    }

    /// T3 (property): for a doc with N independent flags, the
    /// `execution_set ⊆ approval_set` invariant must hold across a battery of
    /// randomized flag combinations — not just the two-state flip the
    /// fixed-example T3 covers.
    ///
    /// The generated doc has N page blocks, each guarded by a unique
    /// `flag_i` value. The approval set is computed once and reused for every
    /// iteration. For each combination, the post-compose body is checked: any
    /// "echo branch-i" line present must be a member of the approval set (it
    /// always is, because approval is a superset), and the compose must
    /// succeed (i.e. execution never reaches a `NotPreApproved` miss).
    #[test]
    fn execution_subset_of_approval_across_randomized_conditions() {
        const BRANCH_COUNT: usize = 4;
        const ITERATIONS: usize = 16;

        let mut body = String::from("---\n");
        for i in 0..BRANCH_COUNT {
            body.push_str(&format!("flag_{i}: 0\n"));
        }
        body.push_str("---\n");
        for i in 0..BRANCH_COUNT {
            body.push_str(&format!("::block when=\"flag_{i} == 1\"\n"));
            body.push_str(&format!("::shell echo branch-{i}\n"));
            body.push_str("::end-block\n");
        }

        let md: Markdown = body.into();
        let approval = approval_set(&md);
        for i in 0..BRANCH_COUNT {
            assert!(
                approval.contains(&format!("echo branch-{i}")),
                "approval missing branch-{i}: {approval:?}"
            );
        }

        // Deterministic pseudo-random combination generator: a small LCG
        // seeded from a constant gives a stable, reproducible sequence of
        // flag combinations without pulling in the `rand` crate.
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            state
        };

        let temp = TempDir::new().unwrap();
        for _ in 0..ITERATIONS {
            let mut overrides = serde_json::Map::new();
            for i in 0..BRANCH_COUNT {
                let value = (next() % 2) as i64;
                overrides.insert(format!("flag_{i}"), json!(value));
            }
            let options = execute_options(approval.clone(), temp.path())
                .with_set_overrides(serde_json::Value::Object(overrides.clone()));
            let (composed, _) = md
                .compose_with(options)
                .unwrap_or_else(|e| panic!("compose failed for {overrides:?}: {e}"));

            // For every branch whose `flag_i == 1`, the branch's command must
            // have executed and produced output.
            for i in 0..BRANCH_COUNT {
                let marker = format!("branch-{i}");
                let expected_executed = overrides.get(&format!("flag_{i}"))
                    .and_then(|v| v.as_i64())
                    .map(|v| v == 1)
                    .unwrap_or(false);
                let appears = composed.content().contains(&marker);
                assert_eq!(
                    appears, expected_executed,
                    "branch-{i} execution mismatch for {overrides:?}: appears={appears}"
                );
                // And in either case, the executed command (if any) is a
                // member of the approval set — the invariant.
                if appears {
                    let cmd = format!("echo {marker}");
                    assert!(
                        approval.contains(&cmd),
                        "executed command {cmd:?} not in approval set: {approval:?}"
                    );
                }
            }
        }
    }
}
