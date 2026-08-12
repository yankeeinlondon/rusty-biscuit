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
pub mod lifecycle;

pub use approval::approval_set;
pub use collect::collect_shell_commands;
pub use lifecycle::{ComposePreflightApprovals, PreflightApprovalStats};

use crate::markdown::Markdown;
use crate::markdown::compose::transclusion;
use crate::markdown::compose::{ComposeOptions, ComposeWarning};
use crate::markdown::compose::shell_expansion::types::{ShellCommandEntry, ShellExpansionError};
use crate::markdown::types::MarkdownResult;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// One edge in the preflight graph: a transclusion directive and the
/// resolved target it points to, captured during the condition-blind
/// pre-flight walk.
///
/// The edge carries the directive's parsed shape (span, line, options,
/// kind) **and** the resolved target. The transclusion engine reuses the
/// resolved target to skip a second `resolve_target` pass, but re-parses the
/// directive against the current content for the replacement span (the
/// preflight span predates frontmatter shell expansion and other
/// offset-shifting stages). It then recurses into [`Self::child`] for the next
/// level of the graph.
#[derive(Debug, Clone)]
pub struct PreflightGraphEdge {
    /// The parsed directive that produced this edge. Its `raw_target` keys the
    /// resolution cache; its span is **not** reused for replacement.
    pub directive: transclusion::BlockDirective,
    /// Resolved target for the directive: a canonical local path or a
    /// remote URL. The transclusion engine uses this as a resolution cache to
    /// avoid calling `transclusion::resolve_target` a second time.
    pub resolved_target: PreflightResolvedTarget,
    /// The child document this edge points to. Reuse recurses through
    /// `child.edges` for nested transclusions.
    ///
    /// Held behind an `Arc` (Finding 16) so the [`PreflightGraphNode::children`]
    /// flat view shares the same subtree instead of deep-cloning it — for a deep
    /// transclusion tree the old duplication was O(n²) in nodes.
    pub child: Arc<PreflightGraphNode>,
}

/// Resolved target for a preflight graph edge.
#[derive(Debug, Clone)]
pub enum PreflightResolvedTarget {
    /// Local file with canonicalized path.
    File(PathBuf),
    /// Remote URL.
    Url(url::Url),
}

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
    /// the child's [`Self::edges`].
    pub entries: Vec<ShellCommandEntry>,
    /// Outgoing transclusion edges, in directive order. Each edge carries the
    /// directive that produced the child reference plus the resolved target,
    /// so a consumer (e.g. [`TransclusionEngine`](crate::markdown::compose::transclusion::TransclusionEngine))
    /// can reuse the resolved targets as a resolution cache and skip a second
    /// `resolve_target` pass. Replacement spans are taken from a fresh parse of
    /// the current content, not from these cached directives.
    ///
    /// Empty when this document is a leaf or has no transclusion directives.
    pub edges: Vec<PreflightGraphEdge>,
    /// Flat view of the children. Body-transclusion children come first in
    /// [`Self::edges`] order (each is the same `Arc` as the corresponding
    /// [`PreflightGraphEdge::child`], so the flat view is a set of refcount
    /// bumps, not a subtree deep-clone — Finding 16), followed by frontmatter
    /// prologue/epilogue children, which have no directive edge.
    pub children: Vec<Arc<PreflightGraphNode>>,
}

/// Best-effort canonical form of a path for source-equality comparison.
///
/// Falls back to the path as-given when it does not exist on disk (e.g. a
/// remote source recorded as its URL string), so comparison still works.
fn canonical_key(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

impl PreflightGraphNode {
    /// Returns `true` when this node has no entries and no edges.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.edges.is_empty()
    }

    /// Returns the child node this document transcluded from `path`, if any.
    ///
    /// Used by the transclusion engine to thread the matching sub-graph into a
    /// recursive child compose, so grandchild target resolution is reused too
    /// (the recursive half of the graph reuse the v2 design calls for). Sources
    /// are compared after a best-effort canonicalization so a relative directive
    /// target and the stored absolute source still match. Both body-edge
    /// children and frontmatter prologue/epilogue children are searched.
    pub(crate) fn child_for_source(&self, path: &Path) -> Option<&Arc<PreflightGraphNode>> {
        let want = canonical_key(path);
        self.children.iter().find(|child| {
            child
                .source
                .as_deref()
                .is_some_and(|src| canonical_key(src) == want)
        })
    }

    /// Returns the child node this document transcluded from `url`, if any.
    ///
    /// The remote analogue of [`Self::child_for_source`]. A fetched Markdown
    /// child is recorded during the walk with its source stored as the URL
    /// string (see [`PreflightGraphNode::source`] and the `Url` arm of the
    /// collector). Threading the matching sub-node into a remote child compose
    /// lets grandchild URL resolution be reused too — the recursive half of the
    /// graph reuse the v2 design calls for, now closed for URL children as well
    /// as local files.
    pub(crate) fn child_for_url(&self, url: &url::Url) -> Option<&Arc<PreflightGraphNode>> {
        let want = PathBuf::from(url.to_string());
        self.children
            .iter()
            .find(|child| child.source.as_deref() == Some(want.as_path()))
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
        for edge in &self.edges {
            edge.child.collect_unique(out, seen);
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
            // The default per-command shell timeout is 10s. These acceptance
            // tests spawn real `echo` subprocesses, which finish in
            // milliseconds in isolation but can be starved well past 10s under
            // full-suite parallel load — tripping `ShellTimeoutBehavior::Error`
            // and failing compose with a spurious timeout. A generous leash
            // (far below the 90s nextest termination ceiling for these tests)
            // keeps contention from masquerading as a real hang.
            .with_shell_timeout(std::time::Duration::from_secs(60))
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
        let sentinel_str = biscuit_file::to_portable_string(&sentinel);

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
        let sentinel_str = biscuit_file::to_portable_string(&sentinel);

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

    /// Pre-flight validation covers `ComposeOperation::ShellBlocks` even when
    /// frontmatter and body `::shell` are disabled: a first approved shell
    /// block creates a sentinel, a later unapproved shell block's command is
    /// missing from the pre-approved set, and the compose must fail with
    /// `NotPreApproved` *before* the first block runs.
    #[test]
    fn preflight_blocks_shell_blocks_when_later_block_unapproved() {
        let temp = TempDir::new().unwrap();
        let sentinel = temp.path().join("sentinel");

        // The first shell block would create the sentinel if it ran. The
        // second shell block holds the unapproved command. The first block's
        // `touch` is in the pre-approved set; the second block's `echo` is not.
        let content = format!(
            "::shell-block\ntouch {sentinel_str}\n::end-block\n\
             ::shell-block\necho unapproved\n::end-block\n",
            sentinel_str = sentinel.display()
        );
        let md: Markdown = content.into();

        // Approve only the first block's command; the second block's command
        // is unapproved. The order is intentional: the approved block comes
        // first so the test would observe the sentinel *if* the gate did not
        // catch the unapproved block before any shell block executed.
        let approved: HashSet<String> = [format!("touch {}", sentinel.display())]
            .into_iter()
            .collect();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellBlocks])
            .with_shell_policy_root(temp.path())
            .with_pre_approved_commands(approved);

        let result = md.compose_with(options);
        assert!(
            result.is_err(),
            "compose should fail because the second shell block's command is unapproved"
        );
        assert!(
            !sentinel.exists(),
            "first shell block executed before pre-flight caught the unapproved later block"
        );
    }

    /// Pre-flight validation walks transcluded children for shell blocks: a
    /// first approved root shell block creates a sentinel, a child document
    /// (transcluded unconditionally) holds an unapproved shell block, and the
    /// compose must fail before the root block runs.
    #[test]
    fn preflight_blocks_shell_blocks_when_transcluded_child_unapproved() {
        use std::io::Write;

        let temp = TempDir::new().unwrap();
        let sentinel = temp.path().join("sentinel");
        let sentinel_str = sentinel.display().to_string();

        let child_path = temp.path().join("child.md");
        let mut child = std::fs::File::create(&child_path).unwrap();
        writeln!(child, "::shell-block").unwrap();
        writeln!(child, "echo unapproved-in-child").unwrap();
        writeln!(child, "::end-block").unwrap();

        let root_path = temp.path().join("root.md");
        let mut root = std::fs::File::create(&root_path).unwrap();
        writeln!(root, "::shell-block").unwrap();
        writeln!(root, "touch {}", sentinel_str).unwrap();
        writeln!(root, "::end-block").unwrap();
        writeln!(root, "::file ./child.md").unwrap();

        let root_content = std::fs::read_to_string(&root_path).unwrap();
        let md: Markdown = root_content.into();

        // Approve only the root block's `touch`. The child's `echo` is missing.
        let approved: HashSet<String> = [format!("touch {}", sentinel_str)]
            .into_iter()
            .collect();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::ShellBlocks, ComposeOperation::BlockTransclusion])
            .with_shell_policy_root(temp.path())
            .with_source_file(&root_path)
            .with_pre_approved_commands(approved);

        let result = md.compose_with(options);
        assert!(
            result.is_err(),
            "compose should fail because the child shell block's command is unapproved"
        );
        assert!(
            !sentinel.exists(),
            "root shell block executed before pre-flight caught the unapproved child block"
        );
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
        const ITERATIONS: usize = 8;

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

    /// The preflight-collected graph is reusable by the transclusion engine:
    /// composing with `with_preflight_graph(...)` produces the same composed
    /// content as composing without it, proving the cached edges carry the
    /// directive metadata and resolved target the engine would otherwise have
    /// to re-derive.
    #[test]
    fn preflight_graph_seeds_block_transclusion() {
        use std::io::Write;

        let temp = TempDir::new().unwrap();
        let child_path = temp.path().join("child.md");
        let mut child = std::fs::File::create(&child_path).unwrap();
        writeln!(child, "# Child").unwrap();
        writeln!(child, "Child body content").unwrap();

        let root_path = temp.path().join("root.md");
        let mut root = std::fs::File::create(&root_path).unwrap();
        writeln!(root, "# Root").unwrap();
        writeln!(root, "::file ./child.md").unwrap();

        let root_content = std::fs::read_to_string(&root_path).unwrap();
        let md: Markdown = root_content.into();

        // Collect a preflight report so we can attach the graph to a later
        // compose. Approval set is empty here (no shell directives) so the
        // pre-approved channel is unused.
        let preflight = md
            .compose_preflight(
                &ComposeOptions::new().with_source_file(&root_path),
            )
            .unwrap();
        assert_eq!(
            preflight.preflight_graph.edges.len(),
            1,
            "preflight graph should have one body-transclusion edge: {:?}",
            preflight.preflight_graph
        );
        let edge = &preflight.preflight_graph.edges[0];
        let resolved_path = match &edge.resolved_target {
            super::PreflightResolvedTarget::File(p) => p.clone(),
            super::PreflightResolvedTarget::Url(_) => {
                panic!("local child edge should resolve to a File target")
            }
        };
        assert_eq!(
            resolved_path.canonicalize().unwrap(),
            child_path.canonicalize().unwrap(),
            "preflight edge should carry the resolved child path"
        );

        // Compose WITHOUT the preflight graph — baseline output.
        let baseline_options = ComposeOptions::new()
            .only(&[ComposeOperation::BlockTransclusion])
            .with_source_file(&root_path);
        let (baseline, _) = md.compose_with(baseline_options.clone()).unwrap();
        let baseline_text = baseline.content().to_string();
        assert!(
            baseline_text.contains("Child body content"),
            "baseline compose did not transclude child: {baseline_text}"
        );

        // Compose WITH the preflight graph — must produce equivalent output.
        let with_graph_options = baseline_options
            .with_preflight_graph(preflight.preflight_graph.clone());
        let (with_graph, _) = md.compose_with(with_graph_options).unwrap();
        let with_graph_text = with_graph.content().to_string();
        assert!(
            with_graph_text.contains("Child body content"),
            "preflight-graph-seeded compose did not transclude child: {with_graph_text}"
        );
        assert_eq!(
            with_graph_text, baseline_text,
            "preflight-graph-seeded compose must produce identical output to the baseline"
        );
    }

    /// A `when=`-gated edge in the preflight graph is still evaluated
    /// condition-aware by the transclusion engine: a false-condition
    /// transclusion is replaced with empty content (skipped) when the
    /// preflight graph is reused, matching the baseline behavior.
    #[test]
    fn preflight_graph_respects_when_condition() {
        use std::io::Write;

        let temp = TempDir::new().unwrap();
        let child_path = temp.path().join("child.md");
        let mut child = std::fs::File::create(&child_path).unwrap();
        writeln!(child, "# Child").unwrap();
        writeln!(child, "Child body content").unwrap();

        let root_path = temp.path().join("root.md");
        let mut root = std::fs::File::create(&root_path).unwrap();
        writeln!(root, "# Root").unwrap();
        writeln!(
            root,
            "::file ./child.md when=\"include_child == true\""
        )
        .unwrap();

        let root_content = std::fs::read_to_string(&root_path).unwrap();
        let md: Markdown = root_content.into();

        // The preflight graph walks the edge regardless of `when=`, so the
        // graph contains the edge. The condition-aware check happens later.
        let preflight = md
            .compose_preflight(
                &ComposeOptions::new().with_source_file(&root_path),
            )
            .unwrap();
        assert_eq!(preflight.preflight_graph.edges.len(), 1);

        // Compose WITHOUT the preflight graph, with `include_child=false` —
        // the child must NOT be transcluded.
        let baseline_options = ComposeOptions::new()
            .only(&[ComposeOperation::BlockTransclusion])
            .with_source_file(&root_path)
            .with_set_overrides(json!({ "include_child": false }));
        let (baseline, _) = md.compose_with(baseline_options.clone()).unwrap();
        assert!(
            !baseline.content().contains("Child body content"),
            "baseline compose unexpectedly transcluded the gated child: {}",
            baseline.content()
        );

        // Compose WITH the preflight graph, same override — same result.
        let with_graph_options = baseline_options
            .with_preflight_graph(preflight.preflight_graph.clone());
        let (with_graph, _) = md.compose_with(with_graph_options).unwrap();
        assert!(
            !with_graph.content().contains("Child body content"),
            "preflight-graph compose unexpectedly transcluded the gated child: {}",
            with_graph.content()
        );
    }

    /// Regression (review-4 High): the preflight walk captures a directive span
    /// *before* page-block pruning, but the Transclusion phase runs *after*
    /// pruning has shifted every following byte offset. Reusing the preflight
    /// graph must therefore re-anchor the replacement span against the current
    /// content; trusting the cached span splices the child at the wrong region.
    /// The graph-seeded compose must be byte-identical to the baseline.
    #[test]
    fn preflight_graph_reanchors_spans_after_offset_shifting_stage() {
        use std::io::Write;

        let temp = TempDir::new().unwrap();
        let child_path = temp.path().join("child.md");
        let mut child = std::fs::File::create(&child_path).unwrap();
        writeln!(child, "CHILD-CONTENT-MARKER").unwrap();

        // A `when=`-false page block precedes the `::file` directive. Page
        // blocks run during the normal pipeline (after the condition-blind
        // preflight walk), so pruning the block moves the `::file` directive
        // many bytes earlier than its preflight-captured offset.
        let root_path = temp.path().join("root.md");
        let mut root = std::fs::File::create(&root_path).unwrap();
        writeln!(root, "---").unwrap();
        writeln!(root, "flag: false").unwrap();
        writeln!(root, "---").unwrap();
        writeln!(root, "::block when=\"flag\"").unwrap();
        writeln!(root, "This pruned block shifts every following byte offset.").unwrap();
        writeln!(root, "More padding so the shift is large and unmistakable.").unwrap();
        writeln!(root, "::end-block").unwrap();
        writeln!(root, "::file ./child.md").unwrap();

        let root_content = std::fs::read_to_string(&root_path).unwrap();
        let md: Markdown = root_content.into();

        // The preflight graph captures the `::file` edge with its pre-pruning
        // span (page blocks are not evaluated during the condition-blind walk).
        let preflight = md
            .compose_preflight(&ComposeOptions::new().with_source_file(&root_path))
            .unwrap();
        assert_eq!(preflight.preflight_graph.edges.len(), 1);

        let baseline_options = ComposeOptions::new()
            .only(&[
                ComposeOperation::PageBlocks,
                ComposeOperation::BlockTransclusion,
            ])
            .with_source_file(&root_path);
        let (baseline, _) = md.compose_with(baseline_options.clone()).unwrap();
        assert!(
            baseline.content().contains("CHILD-CONTENT-MARKER"),
            "baseline did not transclude child: {}",
            baseline.content()
        );
        assert!(!baseline.content().contains("pruned block"));

        let with_graph_options =
            baseline_options.with_preflight_graph(preflight.preflight_graph.clone());
        let (with_graph, _) = md.compose_with(with_graph_options).unwrap();

        assert_eq!(
            with_graph.content(),
            baseline.content(),
            "preflight-graph reuse must produce byte-identical output to the no-graph baseline"
        );
    }

    /// Recursive graph reuse (review-5 Medium): the preflight walk discovers the
    /// full root → child → grandchild tree, and reusing it must reach every
    /// level — the child compose threads its own sub-node so the grandchild edge
    /// is reused too. The graph-seeded compose must be byte-identical to the
    /// no-graph baseline, proving the threaded sub-graph does not corrupt the
    /// deeper transclusions.
    #[test]
    fn preflight_graph_reuse_recurses_to_grandchild() {
        use std::io::Write;

        let temp = TempDir::new().unwrap();

        let grandchild_path = temp.path().join("grandchild.md");
        let mut grandchild = std::fs::File::create(&grandchild_path).unwrap();
        writeln!(grandchild, "GRANDCHILD-CONTENT-MARKER").unwrap();

        let child_path = temp.path().join("child.md");
        let mut child = std::fs::File::create(&child_path).unwrap();
        writeln!(child, "# Child").unwrap();
        writeln!(child, "::file ./grandchild.md").unwrap();

        let root_path = temp.path().join("root.md");
        let mut root = std::fs::File::create(&root_path).unwrap();
        writeln!(root, "# Root").unwrap();
        writeln!(root, "::file ./child.md").unwrap();

        let root_content = std::fs::read_to_string(&root_path).unwrap();
        let md: Markdown = root_content.into();

        let preflight = md
            .compose_preflight(&ComposeOptions::new().with_source_file(&root_path))
            .unwrap();

        // The walked graph is recursive: the root's single edge points at the
        // child, whose own single edge points at the grandchild.
        assert_eq!(
            preflight.preflight_graph.edges.len(),
            1,
            "root should have one edge: {:?}",
            preflight.preflight_graph
        );
        let child_node = &preflight.preflight_graph.edges[0].child;
        assert_eq!(
            child_node.edges.len(),
            1,
            "child node should carry one grandchild edge: {child_node:?}"
        );
        let grandchild_target = match &child_node.edges[0].resolved_target {
            super::PreflightResolvedTarget::File(p) => p.clone(),
            super::PreflightResolvedTarget::Url(_) => panic!("grandchild should resolve to a File"),
        };
        assert_eq!(
            grandchild_target.canonicalize().unwrap(),
            grandchild_path.canonicalize().unwrap(),
            "child node's edge should carry the resolved grandchild path"
        );

        let baseline_options = ComposeOptions::new()
            .only(&[ComposeOperation::BlockTransclusion])
            .with_source_file(&root_path);
        let (baseline, _) = md.compose_with(baseline_options.clone()).unwrap();
        assert!(
            baseline.content().contains("GRANDCHILD-CONTENT-MARKER"),
            "baseline did not transclude grandchild: {}",
            baseline.content()
        );

        let with_graph_options =
            baseline_options.with_preflight_graph(preflight.preflight_graph.clone());
        let (with_graph, _) = md.compose_with(with_graph_options).unwrap();
        assert!(
            with_graph.content().contains("GRANDCHILD-CONTENT-MARKER"),
            "graph-seeded compose did not transclude grandchild: {}",
            with_graph.content()
        );
        assert_eq!(
            with_graph.content(),
            baseline.content(),
            "recursive preflight-graph reuse must produce byte-identical output to the baseline"
        );
    }

    /// Unit: the lookup the engine relies on to thread the matching sub-graph
    /// into a child compose returns the child whose resolved source matches the
    /// requested path, and `None` for an unrelated path.
    #[test]
    fn child_for_source_matches_resolved_child() {
        use std::io::Write;

        let temp = TempDir::new().unwrap();

        let child_path = temp.path().join("child.md");
        let mut child = std::fs::File::create(&child_path).unwrap();
        writeln!(child, "# Child").unwrap();
        writeln!(child, "::shell echo from-child").unwrap();

        let root_path = temp.path().join("root.md");
        let mut root = std::fs::File::create(&root_path).unwrap();
        writeln!(root, "::file ./child.md").unwrap();

        let root_content = std::fs::read_to_string(&root_path).unwrap();
        let md: Markdown = root_content.into();

        let preflight = md
            .compose_preflight(&ComposeOptions::new().with_source_file(&root_path))
            .unwrap();
        let graph = &preflight.preflight_graph;

        let matched = graph
            .child_for_source(&child_path)
            .expect("child_for_source should find the transcluded child");
        let matched_raw: Vec<&str> =
            matched.entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(
            matched_raw.contains(&"echo from-child"),
            "matched node should be the child: {matched_raw:?}"
        );

        assert!(
            graph
                .child_for_source(&temp.path().join("nope.md"))
                .is_none(),
            "an unrelated path must not match any child"
        );
    }
}
