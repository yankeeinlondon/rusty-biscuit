//! Pre-flight approval lifecycle.
//!
//! Combines a condition-blind pre-flight collection with the blacklist /
//! whitelist policy check and the single interactive prompt the v2 design
//! requires. Returns the set the caller hands back to the execution
//! pipeline as `ComposeOptions::with_pre_approved_commands(...)`.
//!
//! The lifecycle is:
//!
//! 1. Collect every command the document *could* run (condition-blind), and
//!    stop when a warning the walk found blocks under the request's policy.
//! 2. Validate the set against the built-in blacklist, the user
//!    blacklist, and the whitelist. Each command site is decided once: a
//!    site whose value is late by its shape; any other by an `exact` rule,
//!    then a `template` rule, then a `prefix` rule.
//! 3. For the remainder, prompt the supplied approval handler, offering only
//!    the choices the site is eligible for, and persist the chosen row.
//! 4. Return the approved set: resolved bytes and template shapes.
//!
//! Darkmatter never prompts directly; the caller (CLI, Claudine, …) owns
//! the handler. When no handler is supplied and the set contains a
//! command the policy does not cover, the function surfaces
//! [`ShellExpansionError::ApprovalRequired`] for the offending command.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::markdown::Markdown;
use crate::markdown::compose::shell_expansion::policy::{
    check_builtin_blacklist, check_template_blacklist, check_user_blacklist, check_whitelist,
};
use crate::markdown::compose::shell_expansion::store::{
    self, load_ruleset, resolve_policy_paths,
};
use crate::markdown::compose::shell_expansion::template::{
    ModeRequirement, OfferedChoices, ShellCandidate, ShellPreApprovals, SuggestedRules,
};
use crate::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellCommandEntry,
    ShellCommandOrigin, ShellExpansionError, ShellPolicyPaths, ShellRuleSet,
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
///
/// The [`warnings`](Self::warnings) are the collection walk's skipped-trigger
/// warnings ([`ComposePreflightReport::warnings`](super::ComposePreflightReport::warnings)),
/// for the caller to deliver with the invocation: composition does not
/// report a trigger only a child it skips holds.
#[derive(Debug, Clone, Default)]
pub struct ComposePreflightApprovals {
    pub pre_approved_commands: ShellPreApprovals,
    pub stats: PreflightApprovalStats,
    pub preflight_graph: super::PreflightGraphNode,
    pub warnings: Vec<crate::markdown::compose::ComposeWarning>,
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
    /// …) is expected to compute the approval set once, then compose with
    /// `request.map_options(|options| options.with_pre_approved_commands(set))`. The
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
    /// - A walk warning the request's
    ///   [`BlockingPolicy`](crate::markdown::compose::policy::BlockingPolicy)
    ///   blocks (a denied skipped trigger) → `ShellExpansionError::Preflight`
    ///   holding [`MarkdownError::BlockedDiagnostics`](crate::markdown::MarkdownError::BlockedDiagnostics),
    ///   judged before any policy file is read or the handler is asked.
    ///
    /// Whatever the error, the warnings the walk reported before it are left
    /// for [`ComposeOptions::take_preflight_failure_warnings`](crate::markdown::compose::ComposeOptions::take_preflight_failure_warnings):
    /// a policy error after a successful walk leaves the walk's
    /// skipped-trigger warnings there.
    pub fn compose_preflight_approvals(
        &self,
        request: &crate::markdown::compose::ComposeRequest,
        approval_handler: Option<Arc<dyn ShellApprovalHandler>>,
    ) -> Result<ComposePreflightApprovals, ShellExpansionError> {
        self.compose_preflight_approvals_with(request, approval_handler, ExtraShellSources::default())
    }

    /// [`Self::compose_preflight_approvals`] with `extra` commands and sites
    /// judged in the same run, under the same policy and the same batched
    /// prompt: a host's lifecycle shells, which the document walk does not
    /// see. They are approved whether or not a body shell operation is
    /// enabled.
    ///
    /// ## Errors
    ///
    /// As [`Self::compose_preflight_approvals`].
    pub fn compose_preflight_approvals_with(
        &self,
        request: &crate::markdown::compose::ComposeRequest,
        approval_handler: Option<Arc<dyn ShellApprovalHandler>>,
        extra: ExtraShellSources,
    ) -> Result<ComposePreflightApprovals, ShellExpansionError> {
        let mut report = self.compose_preflight(request).map_err(|e| match e {
            crate::markdown::types::MarkdownError::ShellExpansion(inner) => *inner,
            // Preserve the rich error (transform parse, ctx merge, schema
            // validation, remote fetch, …) so the caller renders its styled
            // block instead of an opaque policy-I/O string.
            other => ShellExpansionError::Preflight(Box::new(other)),
        })?;
        let warnings = std::mem::take(&mut report.warnings);
        // The walk is the first to learn of a trigger only a skipped child
        // holds: a policy denying it stops here, before any policy file is
        // read, any prompt is shown, or any approved command can run.
        let judged = crate::markdown::compose::pipeline::stop_if_warnings_blocked(request, &warnings)
            .map_err(|stop| ShellExpansionError::Preflight(Box::new(stop)))
            .and_then(|()| self.approve_collected(request, approval_handler, extra, report));
        match judged {
            Ok(approvals) => Ok(ComposePreflightApprovals { warnings, ..approvals }),
            Err(error) => {
                request.options().preflight_failure_warnings.record(warnings);
                Err(error)
            }
        }
    }

    /// Judges a successful walk's `report`, with `extra`, against the
    /// request's shell policy.
    fn approve_collected(
        &self,
        request: &crate::markdown::compose::ComposeRequest,
        approval_handler: Option<Arc<dyn ShellApprovalHandler>>,
        extra: ExtraShellSources,
        mut report: super::ComposePreflightReport,
    ) -> Result<ComposePreflightApprovals, ShellExpansionError> {
        let options = request.options();
        report.entries.extend(extra.entries);
        report.candidates.extend(extra.candidates);

        let shell_opts = options.shell_options();
        let policy_paths = resolve_policy_paths(&shell_opts, &options.source, request.resolution_context())?;
        let whitelist = load_ruleset(&policy_paths.whitelist)?;
        let user_blacklist = load_ruleset(&policy_paths.blacklist)?;

        let ctx = self.source_context_for_errors();
        let mut policy = SitePolicy {
            source: &options.source,
            ctx: &ctx,
            paths: &policy_paths,
            whitelist,
            user_blacklist,
            handler: approval_handler.as_deref(),
            approved: ShellPreApprovals::new(),
            already_whitelisted: 0,
            user_approved: 0,
        };
        let entries_by_key: HashMap<&str, &ShellCommandEntry> =
            report.entries.iter().map(|entry| (entry.normalized.as_str(), entry)).collect();

        // Blacklists first, for every resolved-byte command.
        for entry in &report.entries {
            policy.check_blacklists(entry)?;
        }

        // Then one decision per command site: template-required sites by
        // their shape, resolved-byte sites by exact rule, template rule, then
        // prefix rule, before any prompt.
        let mut site_keys = HashSet::new();
        for candidate in &report.candidates {
            if let Some(keys) = &candidate.resolved {
                site_keys.extend(keys.iter().map(String::as_str));
            }
            policy.decide_site(candidate, &entries_by_key)?;
        }

        // A command no site claims (its site could not be matched to its
        // authored form) is decided on its bytes alone.
        for entry in &report.entries {
            if !site_keys.contains(entry.normalized.as_str()) {
                policy.decide_entry(entry)?;
            }
        }

        let template_sites = report
            .candidates
            .iter()
            .filter(|candidate| candidate.requirement == ModeRequirement::TemplateRequired)
            .count();
        Ok(ComposePreflightApprovals {
            pre_approved_commands: policy.approved,
            stats: PreflightApprovalStats {
                total_discovered: report.entries.len() + template_sites,
                already_whitelisted: policy.already_whitelisted,
                user_approved: policy.user_approved,
            },
            preflight_graph: report.preflight_graph,
            warnings: Vec::new(),
        })
    }
}

/// Commands and sites a host adds to one preflight approval run.
#[derive(Debug, Clone, Default)]
pub struct ExtraShellSources {
    /// Commands with known bytes, one entry per chain segment.
    pub entries: Vec<ShellCommandEntry>,
    /// Sites holding a placeholder.
    pub candidates: Vec<ShellCandidate>,
}

/// The policy one preflight approval run judges every site against, and the
/// approvals it has granted so far.
struct SitePolicy<'a> {
    source: &'a crate::markdown::compose::ComposeSource,
    ctx: &'a biscuit_terminal::errors::SourceContext,
    paths: &'a ShellPolicyPaths,
    whitelist: ShellRuleSet,
    user_blacklist: ShellRuleSet,
    handler: Option<&'a dyn ShellApprovalHandler>,
    approved: ShellPreApprovals,
    already_whitelisted: usize,
    user_approved: usize,
}

impl SitePolicy<'_> {
    fn blacklisted(&self, command: &str, reason: String, origin: &ShellCommandOrigin) -> ShellExpansionError {
        ShellExpansionError::Blacklisted {
            ctx: Box::new(self.ctx.clone()),
            command: command.to_string(),
            reason,
            origin: origin.clone(),
        }
    }

    fn approval_required(
        &self,
        command: &str,
        origin: &ShellCommandOrigin,
        suggested_rules: SuggestedRules,
    ) -> ShellExpansionError {
        ShellExpansionError::ApprovalRequired {
            ctx: Box::new(self.ctx.clone()),
            command: command.to_string(),
            whitelist_path: Box::new(self.paths.whitelist.clone()),
            blacklist_path: Box::new(self.paths.blacklist.clone()),
            origin: origin.clone(),
            suggested_rules: Box::new(suggested_rules),
        }
    }

    fn denied(&self, command: &str, origin: &ShellCommandOrigin) -> ShellExpansionError {
        ShellExpansionError::Denied {
            ctx: Box::new(self.ctx.clone()),
            command: command.to_string(),
            origin: origin.clone(),
        }
    }

    fn check_blacklists(&self, entry: &ShellCommandEntry) -> Result<(), ShellExpansionError> {
        if let Some(reason) = check_builtin_blacklist(&entry.executable, &entry.args) {
            return Err(self.blacklisted(&entry.normalized, reason, &entry.origin));
        }
        if check_user_blacklist(&self.user_blacklist, &entry.executable, &entry.args, &entry.normalized) {
            return Err(self.blacklisted(&entry.normalized, "user blacklist".to_string(), &entry.origin));
        }
        Ok(())
    }

    /// Decides one command site.
    fn decide_site(
        &mut self,
        candidate: &ShellCandidate,
        entries: &HashMap<&str, &ShellCommandEntry>,
    ) -> Result<(), ShellExpansionError> {
        let shape = candidate.shape();
        let command = candidate.display_command();
        let origin = &candidate.site.origin;
        if let Some(shape) = &shape
            && let Some(reason) = check_template_blacklist(shape, &self.user_blacklist)
        {
            return Err(self.blacklisted(&command, reason, origin));
        }

        let resolved = match (&candidate.resolved, candidate.requirement) {
            (Some(keys), ModeRequirement::AnyMode) => Some(keys),
            (_, ModeRequirement::TemplateRequired) => None,
            // Not matched to its resolved form: its bytes are decided as
            // entries of their own.
            (None, ModeRequirement::AnyMode) => return Ok(()),
        };
        if let Some(keys) = resolved {
            if keys.iter().all(|key| self.approved.contains_exact(key)) {
                return Ok(());
            }
            if keys.iter().all(|key| self.whitelist.matches_exact(key)) {
                self.approved.extend_exact(keys.iter().cloned());
                self.already_whitelisted += 1;
                return Ok(());
            }
        }
        if let Some(shape) = &shape {
            if self.approved.contains_template(shape) {
                return Ok(());
            }
            if self.whitelist.template_covers(shape) {
                self.approved.insert_template(shape.clone());
                self.already_whitelisted += 1;
                return Ok(());
            }
        }
        let executables = site_executables(candidate, entries);
        if let Some(keys) = resolved
            && keys
                .iter()
                .zip(&executables)
                .all(|(key, executable)| check_whitelist(&self.whitelist, executable, key))
        {
            self.approved.extend_exact(keys.iter().cloned());
            self.already_whitelisted += 1;
            return Ok(());
        }
        if resolved.is_none()
            && !executables.is_empty()
            && executables.iter().all(|executable| self.whitelist.matches_prefix(executable))
        {
            // A prefix rule covers a template-required site's fixed programs.
            if let Some(shape) = shape {
                self.approved.insert_template(shape);
                self.already_whitelisted += 1;
                return Ok(());
            }
        }

        let Some(handler) = self.handler else {
            let rules = SuggestedRules::new(
                candidate.offered(),
                resolved.map(Vec::as_slice),
                shape.as_ref(),
                &executables,
            );
            return Err(self.approval_required(&command, origin, rules));
        };
        let first = resolved
            .and_then(|keys| keys.first())
            .and_then(|key| entries.get(key.as_str()).copied());
        let mut distinct = HashSet::new();
        let chain_executables: Vec<String> = executables
            .iter()
            .filter(|executable| distinct.insert(executable.as_str()))
            .cloned()
            .collect();
        let offered = candidate.offered();
        let request = ShellApprovalRequest {
            source: self.source.clone(),
            origin: origin.clone(),
            raw_command: command.clone(),
            executable: executables.first().cloned().unwrap_or_default(),
            args: first.map(|entry| entry.args.clone()).unwrap_or_default(),
            normalized_exact: resolved.map(|keys| keys.join(" && ")).unwrap_or_default(),
            whitelist_path: self.paths.whitelist.clone(),
            blacklist_path: self.paths.blacklist.clone(),
            alias_name: None,
            chain_executables: if chain_executables.len() > 1 { chain_executables } else { Vec::new() },
            offered,
            template: candidate.approval_view(),
        };

        let decision = handler.approve(request)?;
        match (decision, resolved, shape) {
            (ShellApprovalDecision::AllowExactPersist, Some(keys), _) if offered.exact => {
                for key in keys {
                    store::append_whitelist_exact(self.paths, key)?;
                }
                self.approved.extend_exact(keys.iter().cloned());
            }
            (ShellApprovalDecision::AllowTemplatePersist, _, Some(shape)) if offered.shape => {
                store::append_whitelist_template(self.paths, &shape)?;
                self.approved.insert_template(shape);
            }
            (ShellApprovalDecision::AllowCommandPersist, resolved, shape) if offered.prefix => {
                let mut persisted = HashSet::new();
                for executable in &executables {
                    if persisted.insert(executable.as_str()) {
                        store::append_whitelist_prefix(self.paths, executable)?;
                    }
                }
                match (resolved, shape) {
                    (Some(keys), _) => self.approved.extend_exact(keys.iter().cloned()),
                    (None, Some(shape)) => {
                        self.approved.insert_template(shape);
                    }
                    (None, None) => return Err(self.denied(&command, origin)),
                }
            }
            (ShellApprovalDecision::AllowOnce, Some(keys), _) => self.approved.extend_exact(keys.iter().cloned()),
            (ShellApprovalDecision::AllowOnce, None, Some(shape)) => {
                self.approved.insert_template(shape);
            }
            (ShellApprovalDecision::BlacklistPersist, resolved, shape) => {
                match (resolved, shape) {
                    (Some(keys), _) => {
                        for key in keys {
                            store::append_blacklist_exact(self.paths, key)?;
                        }
                    }
                    (None, Some(shape)) => store::append_blacklist_template(self.paths, &shape)?,
                    (None, None) => {}
                }
                return Err(self.blacklisted(&command, "user blacklisted".to_string(), origin));
            }
            // Deny, or a choice the request did not offer.
            _ => return Err(self.denied(&command, origin)),
        }
        self.user_approved += 1;
        Ok(())
    }

    /// Decides a resolved-byte command no site claims.
    fn decide_entry(&mut self, entry: &ShellCommandEntry) -> Result<(), ShellExpansionError> {
        if self.approved.contains_exact(&entry.normalized) {
            return Ok(());
        }
        if check_whitelist(&self.whitelist, &entry.executable, &entry.normalized) {
            self.approved.insert_exact(entry.normalized.clone());
            self.already_whitelisted += 1;
            return Ok(());
        }
        let Some(handler) = self.handler else {
            let rules = SuggestedRules::new(
                OfferedChoices::resolved_bytes(),
                Some(std::slice::from_ref(&entry.normalized)),
                None,
                std::slice::from_ref(&entry.executable),
            );
            return Err(self.approval_required(&entry.normalized, &entry.origin, rules));
        };
        let request = ShellApprovalRequest {
            source: self.source.clone(),
            origin: entry.origin.clone(),
            raw_command: entry.raw_command.clone(),
            executable: entry.executable.clone(),
            args: entry.args.clone(),
            normalized_exact: entry.normalized.clone(),
            whitelist_path: self.paths.whitelist.clone(),
            blacklist_path: self.paths.blacklist.clone(),
            alias_name: None,
            chain_executables: Vec::new(),
            offered: OfferedChoices::resolved_bytes(),
            template: None,
        };
        match handler.approve(request)? {
            ShellApprovalDecision::AllowExactPersist => {
                store::append_whitelist_exact(self.paths, &entry.normalized)?;
            }
            ShellApprovalDecision::AllowCommandPersist => {
                store::append_whitelist_prefix(self.paths, &entry.executable)?;
            }
            ShellApprovalDecision::AllowOnce => {}
            ShellApprovalDecision::BlacklistPersist => {
                store::append_blacklist_exact(self.paths, &entry.normalized)?;
                return Err(self.blacklisted(&entry.normalized, "user blacklisted".to_string(), &entry.origin));
            }
            ShellApprovalDecision::Deny | ShellApprovalDecision::AllowTemplatePersist => {
                return Err(self.denied(&entry.normalized, &entry.origin));
            }
        }
        self.approved.insert_exact(entry.normalized.clone());
        self.user_approved += 1;
        Ok(())
    }
}

/// The program of each segment of `candidate`: its resolved executables
/// when it has resolved bytes, otherwise its template's fixed programs.
fn site_executables(candidate: &ShellCandidate, entries: &HashMap<&str, &ShellCommandEntry>) -> Vec<String> {
    match (&candidate.resolved, &candidate.template) {
        (Some(keys), _) => keys
            .iter()
            .map(|key| {
                entries
                    .get(key.as_str())
                    .map(|entry| entry.executable.clone())
                    .unwrap_or_else(|| key.split(' ').next().unwrap_or_default().to_string())
            })
            .collect(),
        (None, Ok(template)) => template.segments.iter().map(|segment| segment.program.clone()).collect(),
        (None, Err(_)) => Vec::new(),
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
        let sentinel_str = biscuit_file::to_portable_string(&sentinel);

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
            .compose_preflight_approvals(&crate::markdown::compose::test_request(options.clone()), Some(handler.clone()))
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
            .compose_preflight_approvals(&crate::markdown::compose::test_request(options.clone()), Some(handler.clone()))
            .expect("preflight should succeed with an approving handler");
        assert_eq!(approvals.stats.total_discovered, 2);
        assert_eq!(approvals.stats.user_approved, 2);
        assert_eq!(approvals.stats.already_whitelisted, 0);
        assert!(approvals.pre_approved_commands.contains_exact("echo first"));
        assert!(approvals.pre_approved_commands.contains_exact("echo second"));

        // Compose with the pre-approved set — the in-stage approval flow
        // is a pure membership check, so no handler call is made.
        let compose_options = build_options(temp.path())
            .with_pre_approved_commands(approvals.pre_approved_commands);
        let (composed, report) = md.compose_with(&crate::markdown::compose::test_request(compose_options)).unwrap();
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
            .compose_preflight_approvals(&crate::markdown::compose::test_request(options.clone()), Some(handler.clone()))
            .unwrap();

        assert_eq!(approvals.stats.already_whitelisted, 1);
        assert_eq!(approvals.stats.user_approved, 0);
        assert!(handler.calls().is_empty(), "whitelisted commands must not prompt");
        assert!(approvals.pre_approved_commands.contains_exact("echo whitelisted"));
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
            .compose_preflight_approvals(&crate::markdown::compose::test_request(options.clone()), None)
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
            .compose_preflight_approvals(&crate::markdown::compose::test_request(options.clone()), Some(handler.clone()))
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
            .compose_preflight_approvals(&crate::markdown::compose::test_request(options.clone()), Some(handler.clone()))
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
            .compose_preflight_approvals(&crate::markdown::compose::test_request(options.clone()), Some(handler.clone()))
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
            .compose_preflight_approvals(&crate::markdown::compose::test_request(options.clone()), Some(handler.clone()))
            .unwrap();
        assert!(approvals.pre_approved_commands.contains_exact("echo accept-me"));
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
            .compose_preflight_approvals(&crate::markdown::compose::test_request(options.clone()), Some(handler.clone()))
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
            .compose_preflight_approvals(&crate::markdown::compose::test_request(options.clone()), Some(handler.clone()))
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
        let (with_graph, _) = md.compose_with(&crate::markdown::compose::test_request(graph_options)).unwrap();

        // Baseline: same approvals, no graph.
        let baseline_options =
            options.with_pre_approved_commands(approvals.pre_approved_commands);
        let (baseline, _) = md.compose_with(&crate::markdown::compose::test_request(baseline_options)).unwrap();

        assert_eq!(
            with_graph.content(),
            baseline.content(),
            "graph-seeded compose must match the no-graph baseline byte-for-byte"
        );
        assert!(with_graph.content().contains("CHILD-FROM-LIFECYCLE-GRAPH"));
        assert!(!with_graph.content().contains("Padding that is pruned"));
    }
}
