//! Pre-flight approval lifecycle.
//!
//! Combines a condition-blind pre-flight collection with the blacklist /
//! whitelist policy check and the single interactive prompt the v2 design
//! requires. Returns the set the caller hands back to the execution
//! pipeline as `ComposeOptions::with_pre_approved_commands(...)`.
//!
//! The lifecycle is:
//!
//! 1. Collect every command the document *could* run (condition-blind).
//! 2. Validate the set against the built-in blacklist, the user
//!    blacklist, and the whitelist.
//! 3. For the remainder, prompt the supplied approval handler.
//! 4. Return the approved set.
//!
//! Darkmatter never prompts directly; the caller (CLI, Claudine, …) owns
//! the handler. When no handler is supplied and the set contains a
//! command the policy does not cover, the function surfaces
//! [`ShellExpansionError::ApprovalRequired`] for the offending command.

use std::collections::HashSet;
use std::sync::Arc;

use crate::markdown::Markdown;
use crate::markdown::compose::ComposeOptions;
use crate::markdown::compose::shell_expansion::policy::{
    check_builtin_blacklist, check_user_blacklist, check_whitelist,
};
use crate::markdown::compose::shell_expansion::store::{
    load_ruleset, resolve_policy_paths,
};
use crate::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
};

/// Counts from a single pre-flight approval run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreflightApprovalStats {
    /// Commands the collector discovered in the document graph.
    pub total_discovered: usize,
    /// Commands covered by the whitelist (no prompt needed).
    pub already_whitelisted: usize,
    /// Commands the approval handler admitted.
    pub user_approved: usize,
}

/// Result of the pre-flight approval lifecycle.
///
/// The [`pre_approved_commands`](Self::pre_approved_commands) set is the
/// input for `ComposeOptions::with_pre_approved_commands(...)`. The
/// caller passes it to `compose_with(...)` so the execution path is a
/// pure membership check against an already-batched approval set.
///
/// The [`preflight_graph`](Self::preflight_graph) is the transclusion graph
/// the collection walk already discovered. The caller hands it back through
/// `ComposeOptions::with_preflight_graph(...)` so the final transclusion stage
/// reuses the resolved targets instead of resolving them a third time — the v2
/// design's "reuse the collection walk" goal on the primary user-facing path.
#[derive(Debug, Clone, Default)]
pub struct ComposePreflightApprovals {
    pub pre_approved_commands: HashSet<String>,
    pub stats: PreflightApprovalStats,
    pub preflight_graph: super::PreflightGraphNode,
}

impl Markdown {
    /// Runs the pre-flight approval lifecycle.
    ///
    /// Walks the document condition-blind, validates each discovered
    /// command against the built-in blacklist, the user blacklist, and
    /// the whitelist, and prompts `approval_handler` (if supplied) for
    /// the remainder. Returns the final approval set.
    ///
    /// The v2 design places this stage between schema validation and
    /// any shell execution (design step 4): the caller (CLI, Claudine,
    /// …) is expected to compute the approval set once, then call
    /// `compose_with(options.with_pre_approved_commands(set))`. The
    /// in-stage approval flow is reserved as a fallback for callers
    /// that opt into per-directive prompting directly.
    ///
    /// ## Errors
    ///
    /// - Built-in or user blacklist match → `ShellExpansionError::Blacklisted`
    ///   for the offending command.
    /// - Handler denied a command → `ShellExpansionError::Denied`.
    /// - Handler blacklisted a command → `ShellExpansionError::Blacklisted`.
    /// - No handler and a command is not whitelisted →
    ///   `ShellExpansionError::ApprovalRequired` for the first such
    ///   command.
    /// - Collection, policy-file, or pre-flight failures.
    pub fn compose_preflight_approvals(
        &self,
        options: &ComposeOptions,
        approval_handler: Option<Arc<dyn ShellApprovalHandler>>,
    ) -> Result<ComposePreflightApprovals, ShellExpansionError> {
        let report = self.compose_preflight(options).map_err(|e| match e {
            crate::markdown::types::MarkdownError::ShellExpansion(inner) => *inner,
            // Preserve the rich error (transform parse, ctx merge, schema
            // validation, remote fetch, …) so the caller renders its styled
            // block instead of an opaque policy-I/O string.
            other => ShellExpansionError::Preflight(Box::new(other)),
        })?;

        let shell_opts = options.shell_options();
        let policy_paths = resolve_policy_paths(&shell_opts, &options.source)?;
        let whitelist = load_ruleset(&policy_paths.whitelist)?;
        let user_blacklist = load_ruleset(&policy_paths.blacklist)?;

        let ctx = self.source_context_for_errors();
        let mut pre_approved: HashSet<String> = HashSet::new();
        let mut already_whitelisted = 0usize;
        let mut user_approved = 0usize;

        for entry in &report.entries {
            if let Some(reason) = check_builtin_blacklist(&entry.executable, &entry.args) {
                return Err(ShellExpansionError::Blacklisted {
                    ctx: Box::new(ctx.clone()),
                    command: entry.normalized.clone(),
                    reason,
                    origin: entry.origin.clone(),
                });
            }

            if check_user_blacklist(
                &user_blacklist,
                &entry.executable,
                &entry.args,
                &entry.normalized,
            ) {
                return Err(ShellExpansionError::Blacklisted {
                    ctx: Box::new(ctx.clone()),
                    command: entry.normalized.clone(),
                    reason: "user blacklist".to_string(),
                    origin: entry.origin.clone(),
                });
            }

            if check_whitelist(&whitelist, &entry.executable, &entry.normalized) {
                pre_approved.insert(entry.normalized.clone());
                already_whitelisted += 1;
                continue;
            }

            let Some(ref handler) = approval_handler else {
                return Err(ShellExpansionError::ApprovalRequired {
                    ctx: Box::new(ctx.clone()),
                    command: entry.normalized.clone(),
                    whitelist_path: policy_paths.whitelist.clone(),
                    blacklist_path: policy_paths.blacklist.clone(),
                    origin: entry.origin.clone(),
                });
            };

            let request = ShellApprovalRequest {
                source: options.source.clone(),
                origin: entry.origin.clone(),
                raw_command: entry.raw_command.clone(),
                executable: entry.executable.clone(),
                args: entry.args.clone(),
                normalized_exact: entry.normalized.clone(),
                whitelist_path: policy_paths.whitelist.clone(),
                blacklist_path: policy_paths.blacklist.clone(),
                chain_executables: Vec::new(),
            };

            match handler.approve(request)? {
                ShellApprovalDecision::AllowExactPersist
                | ShellApprovalDecision::AllowCommandPersist
                | ShellApprovalDecision::AllowOnce => {
                    pre_approved.insert(entry.normalized.clone());
                    user_approved += 1;
                }
                ShellApprovalDecision::Deny => {
                    return Err(ShellExpansionError::Denied {
                        ctx: Box::new(ctx.clone()),
                        command: entry.normalized.clone(),
                        origin: entry.origin.clone(),
                    });
                }
                ShellApprovalDecision::BlacklistPersist => {
                    return Err(ShellExpansionError::Blacklisted {
                        ctx: Box::new(ctx.clone()),
                        command: entry.normalized.clone(),
                        reason: "user blacklisted".to_string(),
                        origin: entry.origin.clone(),
                    });
                }
            }
        }

        Ok(ComposePreflightApprovals {
            pre_approved_commands: pre_approved,
            stats: PreflightApprovalStats {
                total_discovered: report.entries.len(),
                already_whitelisted,
                user_approved,
            },
            preflight_graph: report.preflight_graph,
        })
    }
}

#[cfg(test)]
mod lifecycle_tests {
    //! L1 coverage for the pre-flight approval lifecycle.
    //!
    //! The v2 design places the approval pass between schema validation
    //! and any shell execution (design step 4). These tests exercise
    //! the library surface that the CLI (and Claudine) use to implement
    //! that lifecycle: collect → policy check → batched prompt →
    //! `pre_approved_commands` for `compose_with(...)`.

    use std::sync::Arc;

    use crate::markdown::Markdown;
    use crate::markdown::compose::{ComposeOperation, ComposeOptions};
    use crate::markdown::compose::shell_expansion::types::{
        ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
    };
    use tempfile::TempDir;

    /// Recording handler: counts invocations and replies with a preset
    /// decision for every request. Individual `approve(...)` calls may
    /// inspect the request via the optional `on_request` hook.
    struct RecordingHandler {
        decision: ShellApprovalDecision,
        calls: std::sync::Mutex<Vec<ShellApprovalRequest>>,
    }

    impl RecordingHandler {
        fn new(decision: ShellApprovalDecision) -> Self {
            Self {
                decision,
                calls: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<ShellApprovalRequest> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl ShellApprovalHandler for RecordingHandler {
        fn approve(
            &self,
            request: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, ShellExpansionError> {
            self.calls.lock().unwrap().push(request);
            Ok(self.decision.clone())
        }
    }

    /// Per-call handler: invokes a closure for every request and
    /// returns its decision. Used to test mixed approve/deny flows
    /// within a single pre-flight batch.
    struct PerCallHandler {
        callback: Box<dyn Fn(&ShellApprovalRequest) -> ShellApprovalDecision + Send + Sync>,
        calls: std::sync::Mutex<Vec<ShellApprovalRequest>>,
    }

    impl PerCallHandler {
        fn new<F>(callback: F) -> Self
        where
            F: Fn(&ShellApprovalRequest) -> ShellApprovalDecision + Send + Sync + 'static,
        {
            Self {
                callback: Box::new(callback),
                calls: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<ShellApprovalRequest> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl ShellApprovalHandler for PerCallHandler {
        fn approve(
            &self,
            request: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, ShellExpansionError> {
            self.calls.lock().unwrap().push(request.clone());
            Ok((self.callback)(&request))
        }
    }

    fn build_options(policy_root: &std::path::Path) -> ComposeOptions {
        ComposeOptions::new()
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::FrontmatterShellExpansion,
                ComposeOperation::ShellExpansion,
            ])
            .with_shell_policy_root(policy_root)
    }

    /// The CLI's user-facing lifecycle: a frontmatter `touch` plus a
    /// body `echo` discovered up front, an approval handler that denies
    /// the body, and `compose_with(pre_approved_commands)`. The
    /// frontmatter side effect must NOT have happened by the time
    /// `compose_with` is called — the review's production-blocker case.
    #[test]
    fn preflight_lifecycle_blocks_frontmatter_when_body_denied() {
        let temp = TempDir::new().unwrap();
        let sentinel = temp.path().join("sentinel");
        let sentinel_str = sentinel.display().to_string();

        let content = format!(
            "---\nsentinel: \"$(touch {sentinel_str})\"\n---\n::shell echo body-cmd\n"
        );
        let md: Markdown = content.into();

        // Per-call handler: approve the first command, deny the second.
        // This mirrors the user pressing "Allow" for the frontmatter
        // command and "Deny" for the body command in a single batched
        // pre-flight pass.
        let denied_exact = "echo body-cmd".to_string();
        let handler = Arc::new(PerCallHandler::new(move |req| {
            if req.normalized_exact == denied_exact {
                ShellApprovalDecision::Deny
            } else {
                ShellApprovalDecision::AllowOnce
            }
        }));

        let options = build_options(temp.path());
        let err = md
            .compose_preflight_approvals(&options, Some(handler.clone()))
            .expect_err("body command denial must surface as a ShellExpansionError");
        assert!(
            matches!(err, ShellExpansionError::Denied { .. }),
            "expected Denied, got: {err:?}"
        );

        // The handler saw both commands in discovery order.
        let calls = handler.calls();
        assert_eq!(calls.len(), 2, "calls: {calls:?}");
        assert!(calls.iter().any(|c| c.normalized_exact == format!("touch {sentinel_str}")));
        assert!(calls.iter().any(|c| c.normalized_exact == "echo body-cmd"));

        // The frontmatter `touch` did NOT execute — pre-flight caught the
        // denied body before any shell command ran.
        assert!(
            !sentinel.exists(),
            "frontmatter command executed before pre-flight caught the denied body command"
        );
    }

    /// The positive CLI lifecycle: a doc with two body commands, an
    /// approval handler that approves both, and `compose_with(...)` with
    /// the pre-approved set. Both commands execute, the in-stage
    /// approval handler is not consulted (zero new prompts), and the
    /// pre-flight prompt count is 2.
    #[test]
    fn preflight_lifecycle_approves_all_and_compose_uses_membership_gate() {
        let temp = TempDir::new().unwrap();
        let content = "::shell echo first\n\n::shell echo second\n";
        let md: Markdown = content.into();

        let handler = Arc::new(RecordingHandler::new(ShellApprovalDecision::AllowOnce));

        let options = build_options(temp.path());
        let approvals = md
            .compose_preflight_approvals(&options, Some(handler.clone()))
            .expect("preflight should succeed with an approving handler");
        assert_eq!(approvals.stats.total_discovered, 2);
        assert_eq!(approvals.stats.user_approved, 2);
        assert_eq!(approvals.stats.already_whitelisted, 0);
        assert!(approvals.pre_approved_commands.contains("echo first"));
        assert!(approvals.pre_approved_commands.contains("echo second"));

        // Compose with the pre-approved set — the in-stage approval flow
        // is a pure membership check, so no handler call is made.
        let compose_options = build_options(temp.path())
            .with_pre_approved_commands(approvals.pre_approved_commands);
        let (composed, report) = md.compose_with(compose_options).unwrap();
        assert!(composed.content().contains("first"));
        assert!(composed.content().contains("second"));
        // Handler count stays at 2 (the pre-flight batch) — the in-stage
        // membership fast path never consults the handler.
        assert_eq!(handler.calls().len(), 2);
        assert_eq!(report.shell_approvals_used, 0);
    }

    /// A pre-existing whitelist is honored: the handler is not consulted
    /// for already-allowed commands, and the stats report the count.
    #[test]
    fn preflight_lifecycle_honors_whitelist_without_prompting() {
        let temp = TempDir::new().unwrap();
        let whitelist = temp.path().join(".darkmatter-shell-whitelist");
        std::fs::write(&whitelist, "prefix echo\n").unwrap();

        let content = "::shell echo whitelisted\n";
        let md: Markdown = content.into();
        let handler = Arc::new(RecordingHandler::new(ShellApprovalDecision::AllowOnce));

        let options = build_options(temp.path());
        let approvals = md
            .compose_preflight_approvals(&options, Some(handler.clone()))
            .unwrap();

        assert_eq!(approvals.stats.already_whitelisted, 1);
        assert_eq!(approvals.stats.user_approved, 0);
        assert!(handler.calls().is_empty(), "whitelisted commands must not prompt");
        assert!(approvals.pre_approved_commands.contains("echo whitelisted"));
    }

    /// No approval handler and no whitelist coverage: the first
    /// unapproved command surfaces as `ShellExpansionError::ApprovalRequired`
    /// with the policy paths populated so the user can act on the error.
    #[test]
    fn preflight_lifecycle_without_handler_reports_approval_required() {
        let temp = TempDir::new().unwrap();
        let content = "::shell echo unapproved\n";
        let md: Markdown = content.into();

        let options = build_options(temp.path());
        let err = md
            .compose_preflight_approvals(&options, None)
            .expect_err("no handler and no whitelist must surface ApprovalRequired");
        assert!(
            matches!(err, ShellExpansionError::ApprovalRequired { .. }),
            "expected ApprovalRequired, got: {err:?}"
        );
    }

    /// The built-in blacklist is enforced: a destructive command is
    /// caught during the policy check, before the handler is consulted,
    /// and the lifecycle aborts without ever calling the handler.
    #[test]
    fn preflight_lifecycle_enforces_builtin_blacklist() {
        let temp = TempDir::new().unwrap();
        let content = "::shell rm -rf /\n";
        let md: Markdown = content.into();
        let handler = Arc::new(RecordingHandler::new(ShellApprovalDecision::AllowOnce));

        let options = build_options(temp.path());
        let err = md
            .compose_preflight_approvals(&options, Some(handler.clone()))
            .expect_err("builtin-blacklisted command must be rejected");
        assert!(
            matches!(err, ShellExpansionError::Blacklisted { .. }),
            "expected Blacklisted, got: {err:?}"
        );
        assert!(
            handler.calls().is_empty(),
            "handler must not be consulted for blacklisted commands"
        );
    }

    /// A user-blacklist entry short-circuits the lifecycle with a
    /// targeted `Blacklisted` error naming the policy file.
    #[test]
    fn preflight_lifecycle_enforces_user_blacklist() {
        let temp = TempDir::new().unwrap();
        let blacklist = temp.path().join(".darkmatter-shell-blacklist");
        std::fs::write(&blacklist, "exact echo sneaky\n").unwrap();

        let content = "::shell echo sneaky\n";
        let md: Markdown = content.into();
        let handler = Arc::new(RecordingHandler::new(ShellApprovalDecision::AllowOnce));

        let options = build_options(temp.path());
        let err = md
            .compose_preflight_approvals(&options, Some(handler.clone()))
            .expect_err("user-blacklisted command must be rejected");
        assert!(
            matches!(err, ShellExpansionError::Blacklisted { ref reason, .. } if reason == "user blacklist"),
            "expected Blacklisted with user-blacklist reason, got: {err:?}"
        );
    }

    /// A handler that replies `BlacklistPersist` is treated as a hard
    /// abort: the lifecycle returns `Blacklisted` rather than adding the
    /// command to the pre-approved set.
    #[test]
    fn preflight_lifecycle_treats_blacklist_persist_as_abort() {
        let temp = TempDir::new().unwrap();
        let content = "::shell echo blacklist-me\n";
        let md: Markdown = content.into();
        let handler = Arc::new(RecordingHandler::new(ShellApprovalDecision::BlacklistPersist));

        let options = build_options(temp.path());
        let err = md
            .compose_preflight_approvals(&options, Some(handler.clone()))
            .expect_err("BlacklistPersist must abort the lifecycle");
        assert!(
            matches!(err, ShellExpansionError::Blacklisted { ref reason, .. } if reason == "user blacklisted"),
            "expected Blacklisted(user blacklisted), got: {err:?}"
        );
    }

    /// A handler that replies `AllowCommandPersist` is accepted: the
    /// command is added to the pre-approved set just like `AllowOnce`.
    #[test]
    fn preflight_lifecycle_accepts_allow_command_persist() {
        let temp = TempDir::new().unwrap();
        let content = "::shell echo accept-me\n";
        let md: Markdown = content.into();
        let handler = Arc::new(RecordingHandler::new(ShellApprovalDecision::AllowCommandPersist));

        let options = build_options(temp.path());
        let approvals = md
            .compose_preflight_approvals(&options, Some(handler.clone()))
            .unwrap();
        assert!(approvals.pre_approved_commands.contains("echo accept-me"));
    }

    /// The CLI's "single batched approval" invariant: a doc with
    /// multiple commands is presented to the handler in discovery
    /// order, and the resulting set is the same shape regardless of
    /// how many times it is re-collected.
    #[test]
    fn preflight_lifecycle_handler_sees_every_command_in_discovery_order() {
        let temp = TempDir::new().unwrap();
        let content = "::shell echo a\n\n::shell echo b\n\n::shell echo c\n";
        let md: Markdown = content.into();
        let handler = Arc::new(RecordingHandler::new(ShellApprovalDecision::AllowOnce));

        let options = build_options(temp.path());
        let approvals = md
            .compose_preflight_approvals(&options, Some(handler.clone()))
            .unwrap();

        let seen: Vec<String> = handler
            .calls()
            .into_iter()
            .map(|c| c.normalized_exact)
            .collect();
        assert_eq!(seen, vec!["echo a", "echo b", "echo c"]);
        assert_eq!(approvals.pre_approved_commands.len(), 3);
    }

    /// Review-4 Medium: the approval lifecycle returns the transclusion graph
    /// it already walked, and the CLI-shaped flow attaches it to the compose
    /// options so the Transclusion phase reuses it. The `::file` sits after a
    /// pruned page block, so a graph that reuses stale spans would corrupt
    /// output — proving the attached graph is actually exercised. The
    /// graph-seeded compose must match the no-graph baseline byte-for-byte.
    #[test]
    fn preflight_lifecycle_carries_graph_into_transclusion() {
        use std::io::Write;

        let temp = TempDir::new().unwrap();
        let child_path = temp.path().join("child.md");
        let mut child = std::fs::File::create(&child_path).unwrap();
        writeln!(child, "CHILD-FROM-LIFECYCLE-GRAPH").unwrap();

        let root_path = temp.path().join("root.md");
        let mut root = std::fs::File::create(&root_path).unwrap();
        writeln!(root, "---").unwrap();
        writeln!(root, "flag: false").unwrap();
        writeln!(root, "---").unwrap();
        writeln!(root, "::shell echo hello").unwrap();
        writeln!(root, "::block when=\"flag\"").unwrap();
        writeln!(root, "Padding that is pruned and shifts the ::file offset.").unwrap();
        writeln!(root, "::end-block").unwrap();
        writeln!(root, "::file ./child.md").unwrap();

        let root_content = std::fs::read_to_string(&root_path).unwrap();
        let md: Markdown = root_content.into();

        let options = ComposeOptions::new()
            .only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::PageBlocks,
                ComposeOperation::ShellExpansion,
                ComposeOperation::BlockTransclusion,
            ])
            .with_shell_policy_root(temp.path())
            .with_source_file(&root_path);

        let handler = Arc::new(RecordingHandler::new(ShellApprovalDecision::AllowOnce));
        let approvals = md
            .compose_preflight_approvals(&options, Some(handler.clone()))
            .unwrap();

        // The lifecycle surfaced the graph with the child edge.
        assert_eq!(
            approvals.preflight_graph.edges.len(),
            1,
            "lifecycle must return the walked graph: {:?}",
            approvals.preflight_graph
        );

        // CLI-shaped flow: attach BOTH the approval set and the graph.
        let graph_options = options
            .clone()
            .with_pre_approved_commands(approvals.pre_approved_commands.clone())
            .with_preflight_graph(approvals.preflight_graph.clone());
        let (with_graph, _) = md.compose_with(graph_options).unwrap();

        // Baseline: same approvals, no graph.
        let baseline_options =
            options.with_pre_approved_commands(approvals.pre_approved_commands);
        let (baseline, _) = md.compose_with(baseline_options).unwrap();

        assert_eq!(
            with_graph.content(),
            baseline.content(),
            "graph-seeded compose must match the no-graph baseline byte-for-byte"
        );
        assert!(with_graph.content().contains("CHILD-FROM-LIFECYCLE-GRAPH"));
        assert!(!with_graph.content().contains("Padding that is pruned"));
    }
}
