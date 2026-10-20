//! Execution of one shell command site in either mode.
//!
//! A command whose authored text holds a placeholder can run two ways:
//!
//! - **Resolved bytes**: the text after interpolation, tokenized, exactly as
//!   before templates existed. Eligible only when the site may run in any
//!   mode and its current text still parses as a command.
//! - **Template**: the authored text re-parsed with its placeholders as
//!   expression nodes, typed against the owning document's schema, then each
//!   hole filled with exactly one argument (see [`template::fill_template`]).
//!
//! The mode follows from what authorizes the site. With pre-approvals, the
//! site's freshly resolved bytes must be exact members, or its re-typed shape
//! a template member; nothing here inserts into either set. Without them, an
//! exact rule or a cached resolved-byte decision wins, then a template rule or
//! a cached shape decision, then a prefix rule, and finally the approval
//! handler, which is offered the shape alongside the bytes.
//!
//! In template mode every hole is filled and the complete chain is checked
//! against the built-in and user blacklists before a result-cache lookup or a
//! spawn can happen; each program is then resolved to a native target that
//! needs no interpreter.

use super::template::{
    self, ApprovalKey, CommandTemplate, HoleEvaluator, ModeRequirement, OfferedChoices, ShellCandidate,
    TemplateShape,
};
use super::types::{
    CommandAction, ErrorHandling, PipelineAction, ShellApprovalDecision, ShellApprovalRequest, ShellCommandOrigin,
    ShellDirective, ShellExpansionError, ShellExpansionRuntime, ShellPipeline, ShellPolicyPaths, ShellRuntimeSnapshot,
};
use super::{
    ExecutionMode, PreparedShellDirective, ReservationGuard, check_filled_chain, check_template_blacklist, executor,
    prepare_resolved, probe, store,
};
use biscuit_terminal::errors::SourceContext;
use std::ops::Range;
use std::time::Duration;

/// The parts of a directive that do not come from its command words: where
/// its output goes and its own options. Known even when the current text no
/// longer parses as a command.
#[derive(Debug, Clone)]
pub(crate) struct SiteFrame {
    pub span: Range<usize>,
    pub indent: String,
    pub error_handling: ErrorHandling,
    pub timeout_override: Option<Duration>,
    pub no_cache: bool,
}

impl SiteFrame {
    /// The frame of a directive that parsed.
    pub(crate) fn of(directive: &ShellDirective) -> Self {
        Self {
            span: directive.span.clone(),
            indent: directive.indent.clone(),
            error_handling: directive.error_handling.clone(),
            timeout_override: directive.timeout_override,
            no_cache: directive.no_cache,
        }
    }
}

/// A command site's authored template, ready to fill: typed against its
/// owner's schema, with the state its placeholders are read from.
pub(crate) struct TemplateSite<'a> {
    candidate: ShellCandidate,
    evaluator: &'a dyn HoleEvaluator,
    /// The authored document, for diagnostics that must not show a value.
    ctx: SourceContext,
    frame: SiteFrame,
    /// Late values a host already validated at its barrier; those holes are
    /// not evaluated again.
    late: Option<template::SiteValues>,
}

impl<'a> TemplateSite<'a> {
    /// The site of `candidate`, or `None` when it has no shape (no
    /// placeholder, or not a valid template), so it can run only as resolved
    /// bytes.
    pub(crate) fn new(
        candidate: ShellCandidate,
        evaluator: &'a dyn HoleEvaluator,
        ctx: SourceContext,
        frame: SiteFrame,
    ) -> Option<Self> {
        candidate.shape()?;
        Some(Self {
            candidate,
            evaluator,
            ctx,
            frame,
            late: None,
        })
    }

    /// This site, filling the holes `late` holds from it instead of
    /// evaluating them.
    pub(crate) fn with_late_values(mut self, late: Option<template::SiteValues>) -> Self {
        self.late = late;
        self
    }

    fn template(&self) -> &CommandTemplate {
        self.candidate
            .template
            .as_ref()
            .expect("a site is built only for a valid template")
    }

    pub(crate) fn shape(&self) -> TemplateShape {
        TemplateShape::from_template(self.template())
    }

    pub(crate) fn candidate(&self) -> &ShellCandidate {
        &self.candidate
    }

    fn authored(&self) -> &str {
        &self.template().authored
    }

    fn origin(&self) -> &ShellCommandOrigin {
        &self.candidate.site.origin
    }

    /// The choices a prompt for this site offers when its resolved bytes may
    /// also run.
    pub(crate) fn offered(&self) -> OfferedChoices {
        OfferedChoices {
            exact: true,
            shape: true,
            prefix: true,
        }
    }

    /// Fills the template, checks the complete filled chain against both
    /// blacklists, and resolves every program to a native target.
    ///
    /// ## Errors
    ///
    /// - [`ShellExpansionError::MissingValue`], [`ShellExpansionError::TemplateValueRejected`],
    ///   or [`ShellExpansionError::ExpressionEvaluation`] for a hole that
    ///   cannot fill its argument.
    /// - [`ShellExpansionError::Blacklisted`] when any filled segment is
    ///   blacklisted.
    /// - [`ShellExpansionError::TemplateTarget`] when a program is not found
    ///   or needs an interpreter.
    pub(crate) fn prepare_template(
        &self,
        options: &crate::markdown::compose::ComposeRequest,
        snapshot: &ShellRuntimeSnapshot,
    ) -> Result<PreparedShellDirective, ShellExpansionError> {
        let authored = self.authored().to_string();
        let document_late = options.document_shell_values.as_ref().and_then(|values| values.get(&self.candidate));
        if options.document_shell_values.is_some() && self.late.is_none() && document_late.is_none()
            && self.template().placeholders().any(|hole| matches!(hole.timing, template::HoleTiming::Deferred { .. }))
        {
            return Err(ShellExpansionError::NotPreApproved {
                ctx: Box::new(self.ctx.clone()),
                command: authored,
                origin: self.origin().clone(),
                source_desc: " (late command site changed after its task barrier)".to_string(),
            });
        }
        let chain = template::fill_template(self.template(), self.evaluator, self.late.as_ref().or(document_late.as_ref()))
            .map_err(|error| error.into_shell_error(&self.ctx, &authored, self.origin()))?;
        if let Some(reason) = check_filled_chain(&chain, &snapshot.user_blacklist) {
            return Err(ShellExpansionError::Blacklisted {
                ctx: Box::new(self.ctx.clone()),
                command: authored,
                reason,
                origin: self.origin().clone(),
            });
        }

        let shell_opts = options.shell_options();
        let working_dir = executor::resolve_working_directory(&shell_opts, &options.source, options.resolution_context());
        let search_path = options.shell_child_environment().and_then(|environment| environment.path());
        let mut actions = Vec::with_capacity(chain.segments.len());
        for segment in chain.segments {
            let target = template::resolve_direct_target(&segment.program, search_path, &working_dir).map_err(
                |reason| ShellExpansionError::TemplateTarget {
                    ctx: Box::new(self.ctx.clone()),
                    command: authored.clone(),
                    reason: Box::new(reason),
                    origin: self.origin().clone(),
                },
            )?;
            actions.push(PipelineAction {
                operator: segment.operator,
                command: CommandAction {
                    executable: target.to_string_lossy().into_owned(),
                    args: segment.args,
                    redirection: segment.redirection,
                },
            });
        }
        let first = &actions[0].command;
        let effective = ShellDirective {
            raw_command: authored.clone(),
            executable: first.executable.clone(),
            args: first.args.clone(),
            span: self.frame.span.clone(),
            indent: self.frame.indent.clone(),
            origin: self.origin().clone(),
            error_handling: self.frame.error_handling.clone(),
            timeout_override: self.frame.timeout_override,
            no_cache: self.frame.no_cache,
            pipeline: Some(ShellPipeline { actions }),
            ctx: self.ctx.clone(),
        };
        Ok(PreparedShellDirective {
            effective,
            display_command: authored,
            mode: ExecutionMode::Template,
        })
    }

    fn blacklisted(&self, reason: String) -> ShellExpansionError {
        ShellExpansionError::Blacklisted {
            ctx: Box::new(self.ctx.clone()),
            command: self.authored().to_string(),
            reason,
            origin: self.origin().clone(),
        }
    }
}

/// The ` (in <file>)` suffix of a [`ShellExpansionError::NotPreApproved`].
pub(crate) fn source_desc(options: &crate::markdown::compose::ComposeRequest) -> String {
    match &options.source {
        crate::markdown::compose::ComposeSource::File(path) => format!(" (in {})", path.display()),
        _ => String::new(),
    }
}

/// Prepares one command site, choosing its execution mode.
///
/// `resolved` is the site's current text parsed as a resolved-byte directive,
/// or why it no longer parses. `site` is its authored template, when it has
/// one with a placeholder; without one this is [`super::prepare_directive`].
///
/// ## Errors
///
/// Every error [`super::prepare_directive`] reports, plus the template-mode
/// errors of [`TemplateSite::prepare_template`]. A site with neither
/// parseable bytes nor a template fails with its parse error.
pub(crate) fn prepare_site(
    resolved: Result<ShellDirective, ShellExpansionError>,
    site: Option<&TemplateSite<'_>>,
    options: &crate::markdown::compose::ComposeRequest,
    policy_paths: &ShellPolicyPaths,
    runtime: &mut ShellExpansionRuntime,
) -> Result<PreparedShellDirective, ShellExpansionError> {
    let Some(site) = site else {
        return super::prepare_directive(&resolved?, options, policy_paths, runtime);
    };
    // Resolved bytes may run only for a site that allows either mode and
    // whose current text still reads as a command.
    let bytes = match (site.candidate.requirement, resolved) {
        (ModeRequirement::AnyMode, Ok(directive)) => Some(directive),
        _ => None,
    };
    let shape = site.shape();

    if let Some(approved) = &options.pre_approved_commands {
        if let Some(directive) = &bytes {
            match prepare_resolved(directive, Some(site), options, policy_paths, runtime) {
                Err(ShellExpansionError::NotPreApproved { .. }) if approved.contains_template(&shape) => {}
                other => return other,
            }
        }
        if approved.contains_template(&shape) {
            return site.prepare_template(options, &runtime.snapshot());
        }
        return Err(ShellExpansionError::NotPreApproved {
            ctx: Box::new(site.ctx.clone()),
            command: site.authored().to_string(),
            origin: site.origin().clone(),
            source_desc: source_desc(options),
        });
    }

    let snapshot = runtime.snapshot();
    if let Some(reason) = check_template_blacklist(&shape, &snapshot.user_blacklist) {
        return Err(site.blacklisted(reason));
    }
    let policy_root = ApprovalKey::policy_root_of(policy_paths);
    if let Some(directive) = &bytes {
        // An exact rule or a cached resolved-byte decision answers first.
        let (effective, _) = super::resolve_or_passthrough(
            directive,
            &options.alias_resolutions,
            probe::login_shell(options.context().env()),
        );
        let exact = super::collect_normalized_commands(&effective).iter().all(|normalized| {
            snapshot.whitelist.matches_exact(normalized)
                || snapshot
                    .allow_once
                    .contains(&ApprovalKey::exact(policy_root, normalized.clone()))
        });
        if exact {
            return prepare_resolved(directive, Some(site), options, policy_paths, runtime);
        }
    }
    let template_key = ApprovalKey::template(policy_root, shape.clone());
    if snapshot.whitelist.template_covers(&shape) || snapshot.allow_once.contains(&template_key) {
        return site.prepare_template(options, &snapshot);
    }
    if let Some(directive) = &bytes {
        // A prefix rule, then a prompt offering both the bytes and the shape.
        return prepare_resolved(directive, Some(site), options, policy_paths, runtime);
    }
    let programs: Vec<&str> = site
        .template()
        .segments
        .iter()
        .map(|segment| segment.program.as_str())
        .collect();
    if programs.iter().all(|program| snapshot.whitelist.matches_prefix(program)) {
        return site.prepare_template(options, &snapshot);
    }
    approve_template_only(site, &shape, template_key, &programs, options, policy_paths, runtime)
}

/// Asks the approval handler about a site that can run only as a template,
/// recording a session decision under the shape's key.
fn approve_template_only(
    site: &TemplateSite<'_>,
    shape: &TemplateShape,
    key: ApprovalKey,
    programs: &[&str],
    options: &crate::markdown::compose::ComposeRequest,
    policy_paths: &ShellPolicyPaths,
    runtime: &mut ShellExpansionRuntime,
) -> Result<PreparedShellDirective, ShellExpansionError> {
    let approval_required = || ShellExpansionError::ApprovalRequired {
        ctx: Box::new(site.ctx.clone()),
        command: site.authored().to_string(),
        whitelist_path: Box::new(policy_paths.whitelist.clone()),
        blacklist_path: Box::new(policy_paths.blacklist.clone()),
        origin: site.origin().clone(),
    };
    let Some(handler) = options.shell_approval_handler.as_ref() else {
        return Err(approval_required());
    };
    let mut guard = ReservationGuard::new(runtime);
    match guard.reserve(&key, true) {
        super::types::ReserveOutcome::Reserved => {}
        super::types::ReserveOutcome::AlreadyAllowed => {
            return site.prepare_template(options, &guard.runtime.snapshot());
        }
        super::types::ReserveOutcome::Pending => return Err(approval_required()),
    }

    let mut distinct = std::collections::HashSet::new();
    let chain_executables: Vec<String> = programs
        .iter()
        .filter(|program| distinct.insert(**program))
        .map(|program| (*program).to_string())
        .collect();
    let offered = OfferedChoices {
        exact: false,
        shape: true,
        prefix: true,
    };
    let request = ShellApprovalRequest {
        source: options.source.clone(),
        origin: site.origin().clone(),
        raw_command: site.authored().to_string(),
        executable: programs.first().map(|program| (*program).to_string()).unwrap_or_default(),
        args: Vec::new(),
        // No value is known as bytes, so none is offered as bytes.
        normalized_exact: String::new(),
        whitelist_path: policy_paths.whitelist.clone(),
        blacklist_path: policy_paths.blacklist.clone(),
        alias_name: None,
        chain_executables: if chain_executables.len() > 1 { chain_executables } else { Vec::new() },
        offered,
        template: site.candidate.approval_view(),
    };

    match handler.approve(request)? {
        ShellApprovalDecision::AllowTemplatePersist => {
            guard.release(&key, false);
            store::append_whitelist_template(policy_paths, shape)?;
            guard.runtime.persist_whitelist_template(shape.clone());
            site.prepare_template(options, &guard.runtime.snapshot())
        }
        ShellApprovalDecision::AllowCommandPersist => {
            guard.release(&key, false);
            let mut persisted = std::collections::HashSet::new();
            for program in programs {
                if persisted.insert(*program) {
                    store::append_whitelist_prefix(policy_paths, program)?;
                    guard.runtime.persist_whitelist_prefix((*program).to_string());
                }
            }
            site.prepare_template(options, &guard.runtime.snapshot())
        }
        ShellApprovalDecision::AllowOnce => {
            guard.release(&key, true);
            guard.runtime.approvals_used += 1;
            site.prepare_template(options, &guard.runtime.snapshot())
        }
        ShellApprovalDecision::BlacklistPersist => {
            guard.release(&key, false);
            store::append_blacklist_template(policy_paths, shape)?;
            guard.runtime.persist_blacklist_template(shape.clone());
            Err(site.blacklisted("user blacklisted".to_string()))
        }
        // The exact choice was not offered: no bytes are known to approve.
        ShellApprovalDecision::Deny | ShellApprovalDecision::AllowExactPersist => {
            guard.release(&key, false);
            Err(ShellExpansionError::Denied {
                ctx: Box::new(site.ctx.clone()),
                command: site.authored().to_string(),
                origin: site.origin().clone(),
            })
        }
    }
}

/// What one document's shell stages need to build its command sites: the
/// owner schema its placeholders are typed by and the roots its host
/// declared.
pub(crate) struct SiteScope<'a> {
    schema: Option<crate::markdown::schemas::SimplifiedSchema>,
    host_globals: Vec<String>,
    deferred: &'a template::DeferredShellRoots,
    source_file: std::path::PathBuf,
    owner: Option<String>,
}

impl<'a> SiteScope<'a> {
    /// The scope of the document `options` compose, whose effective schema
    /// projection is `schema`.
    pub(crate) fn new(
        options: &'a crate::markdown::compose::ComposeRequest,
        schema: Option<crate::markdown::schemas::SimplifiedSchema>,
    ) -> Self {
        use crate::markdown::compose::ComposeSource;
        let (source_file, owner) = match &options.source {
            ComposeSource::File(path) => (
                path.clone(),
                Some(
                    biscuit_file::canonicalize_simplified(path)
                        .unwrap_or_else(|_| path.clone())
                        .to_string_lossy()
                        .to_string(),
                ),
            ),
            ComposeSource::Url(url) => (std::path::PathBuf::from(url.as_str()), Some(url.to_string())),
            ComposeSource::Unknown => (std::path::PathBuf::from("<unknown>"), None),
        };
        Self {
            schema,
            host_globals: options
                .host_globals
                .as_ref()
                .map(|globals| globals.roots().map(str::to_string).collect())
                .unwrap_or_default(),
            deferred: options.deferred_shell_roots(),
            source_file,
            owner,
        }
    }

    /// The template site of the authored command `authored`, or `None` when
    /// it has no placeholder or is not a valid template (it then runs only as
    /// resolved bytes).
    ///
    /// ## Errors
    ///
    /// [`ShellExpansionError::TemplateGrammar`] for a command that reads a
    /// declared late root and is not a valid template.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn site<'e>(
        &self,
        authored: &str,
        template: Result<CommandTemplate, template::TemplateGrammarError>,
        origin: ShellCommandOrigin,
        ctx: SourceContext,
        evaluator: &'e dyn HoleEvaluator,
        frame: SiteFrame,
    ) -> Result<Option<TemplateSite<'e>>, ShellExpansionError> {
        let derived = std::collections::HashSet::new();
        let scope = template::candidate::CandidateScope {
            schema: self.schema.as_ref(),
            host_globals: self.host_globals.clone(),
            deferred: self.deferred,
            derived: &derived,
            source_file: self.source_file.clone(),
            owner: self.owner.clone(),
            ctx: &ctx,
        };
        // Presence is checked when each hole is filled, so discovery's
        // approval-time evaluation is not repeated here.
        let candidate = scope.build(authored, template, origin, &|_| None)?;
        Ok(TemplateSite::new(candidate, evaluator, ctx, frame))
    }
}

/// The context a template diagnostic is excerpted from: the document as it
/// was loaded, never the current text, into which values were interpolated.
pub(crate) fn authored_context(markdown: &crate::markdown::Markdown, authored_body: Option<&str>) -> SourceContext {
    markdown.loaded_source_context_for_errors().unwrap_or_else(|| {
        let path = std::path::PathBuf::from("unknown");
        SourceContext::new(path.clone(), path, authored_body.unwrap_or_default())
    })
}

#[cfg(test)]
mod acceptance_tests {
    use super::*;
    use super::super::executor::ShellStatus;
    use super::super::template::CommandSite;
    use crate::markdown::compose::{ComposeContext, ComposeOptions};
    use serde_json::json;

    #[test]
    fn a_runtime_blacklist_refuses_a_preapproved_template_with_a_cached_result() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("doc.md");
        let program = std::env::current_exe().unwrap().to_string_lossy().into_owned();
        let authored = format!("{} {{{{ argument }}}}", serde_json::to_string(&program).unwrap());
        let context = SourceContext::new(source.clone(), source.clone(), authored.clone());
        let candidate = ShellCandidate {
            authored_command: authored.clone(),
            site: CommandSite { source_file: source.clone(), origin: ShellCommandOrigin::Body { line: 1 }, owner: None },
            template: template::parse_authored(&authored), resolved: None,
            requirement: ModeRequirement::TemplateRequired,
        };
        let evaluate = |_: &str| Ok(json!("--help"));
        let site = TemplateSite::new(candidate, &evaluate, context, SiteFrame {
            span: 0..0, indent: String::new(), error_handling: ErrorHandling::default(),
            timeout_override: None, no_cache: false,
        }).unwrap();
        let mut approved = template::ShellPreApprovals::new();
        approved.insert_template(site.shape());
        let options = crate::markdown::compose::test_request(ComposeOptions::new_with_context(
            ComposeContext::capture_for_content(directory.path(), ""))
            .with_source_file(&source).with_shell_working_directory(directory.path())
            .with_pre_approved_commands(approved));
        let mut runtime = ShellExpansionRuntime::new();
        let paths = ShellPolicyPaths {
            whitelist: directory.path().join(".darkmatter-shell-whitelist"),
            blacklist: directory.path().join(".darkmatter-shell-blacklist"),
        };
        let prepare = |runtime: &mut ShellExpansionRuntime| prepare_site(
            Err(ShellExpansionError::NotPreApproved { ctx: Box::new(site.ctx.clone()), command: authored.clone(), origin: site.origin().clone(), source_desc: String::new() }), Some(&site), &options, &paths, runtime);
        let prepared = prepare(&mut runtime).unwrap();
        let first = super::super::execute_prepared_outcome(&prepared, &options, &runtime).unwrap();
        assert!(first.fresh);
        assert_eq!(first.outcome.status, ShellStatus::Exited(0));
        assert!(!first.outcome.stdout.is_empty());
        let cached = super::super::execute_prepared_outcome(&prepare(&mut runtime).unwrap(), &options, &runtime).unwrap();
        assert!(!cached.fresh);
        assert_eq!(cached.outcome.stdout, first.outcome.stdout);
        runtime.persist_blacklist_exact(super::super::normalize_command(&program, &["--help".to_string()]));
        assert!(matches!(prepare(&mut runtime), Err(ShellExpansionError::Blacklisted { .. })));
    }

    struct Decision(ShellApprovalDecision);

    impl super::super::types::ShellApprovalHandler for Decision {
        fn approve(&self, _: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
            Ok(self.0.clone())
        }
    }

    fn template_attempt(
        directory: &std::path::Path,
        options: &crate::markdown::compose::ComposeRequest,
        runtime: &mut ShellExpansionRuntime,
    ) -> Result<PreparedShellDirective, ShellExpansionError> {
        let source = directory.join("doc.md");
        let program = std::env::current_exe().unwrap().to_string_lossy().into_owned();
        let authored = format!("{} {{{{ argument }}}}", serde_json::to_string(&program).unwrap());
        let candidate = ShellCandidate {
            authored_command: authored.clone(),
            site: CommandSite { source_file: source.clone(), origin: ShellCommandOrigin::Body { line: 1 }, owner: None },
            template: template::parse_authored(&authored), resolved: None,
            requirement: ModeRequirement::TemplateRequired,
        };
        let evaluate = |_: &str| Ok(json!("--help"));
        let site = TemplateSite::new(candidate, &evaluate,
            SourceContext::new(source.clone(), source, authored.clone()), SiteFrame {
                span: 0..0, indent: String::new(), error_handling: ErrorHandling::default(),
                timeout_override: None, no_cache: false,
            }).unwrap();
        prepare_site(Err(ShellExpansionError::NotPreApproved {
            ctx: Box::new(site.ctx.clone()), command: authored, origin: site.origin().clone(), source_desc: String::new(),
        }), Some(&site), options, &ShellPolicyPaths {
            whitelist: directory.join(".darkmatter-shell-whitelist"),
            blacklist: directory.join(".darkmatter-shell-blacklist"),
        }, runtime)
    }

    #[test]
    fn a_failed_template_write_can_be_retried_without_a_cached_approval() {
        let directory = tempfile::tempdir().unwrap();
        let whitelist = directory.path().join(".darkmatter-shell-whitelist");
        std::fs::create_dir(&whitelist).unwrap();
        let options = crate::markdown::compose::test_request(ComposeOptions::new_with_context(
            ComposeContext::capture_for_content(directory.path(), ""))
            .with_source_file(directory.path().join("doc.md"))
            .with_shell_working_directory(directory.path())
            .with_shell_approval_handler(std::sync::Arc::new(Decision(ShellApprovalDecision::AllowTemplatePersist))));
        let mut runtime = ShellExpansionRuntime::new();
        assert!(matches!(template_attempt(directory.path(), &options, &mut runtime), Err(ShellExpansionError::PolicyIo { .. })));
        std::fs::remove_dir(&whitelist).unwrap();
        let prepared = template_attempt(directory.path(), &options, &mut runtime).unwrap();
        let outcome = super::super::execute_prepared_outcome(&prepared, &options, &runtime).unwrap();
        assert_eq!(outcome.outcome.status, ShellStatus::Exited(0));
        let persisted = std::fs::read_to_string(&whitelist).unwrap();
        assert_eq!(persisted.lines().count(), 1);
        assert!(persisted.starts_with("template "));
        template_attempt(directory.path(), &options, &mut runtime).unwrap();
        assert_eq!(std::fs::read_to_string(&whitelist).unwrap(), persisted);
    }

    #[test]
    fn a_failed_template_approver_wakes_its_parked_peer() {
        use std::sync::{Arc, mpsc};
        use std::time::{Duration, Instant};

        struct FailsAfterPeerParks {
            entered: mpsc::SyncSender<()>,
            probe: ShellExpansionRuntime,
            command: String,
        }
        impl super::super::types::ShellApprovalHandler for FailsAfterPeerParks {
            fn approve(&self, _: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
                self.entered.send(()).unwrap();
                let deadline = Instant::now() + Duration::from_secs(3);
                while self.probe.parked_waiters_for_test(&self.command) == 0 {
                    assert!(Instant::now() < deadline, "the peer must actually park on this template approval");
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(ShellExpansionError::PolicyIo {
                    path: std::path::PathBuf::from("approval-handler"), source: std::io::Error::other("prompt failed"),
                })
            }
        }

        let directory = tempfile::tempdir().unwrap();
        let runtime = ShellExpansionRuntime::new();
        let program = std::env::current_exe().unwrap().to_string_lossy().into_owned();
        let command = TemplateShape::from_template(&template::parse_authored(
            &format!("{} {{{{ argument }}}}", serde_json::to_string(&program).unwrap())).unwrap()).canonical_text();
        let (entered_tx, entered_rx) = mpsc::sync_channel(1);
        let base = ComposeOptions::new_with_context(ComposeContext::capture_for_content(directory.path(), ""))
            .with_source_file(directory.path().join("doc.md"))
            .with_shell_working_directory(directory.path());
        let approver_options = crate::markdown::compose::test_request(base.clone().with_shell_approval_handler(Arc::new(FailsAfterPeerParks {
            entered: entered_tx, probe: runtime.clone_for_child(), command,
        })));
        let waiter_options = crate::markdown::compose::test_request(base.with_shell_approval_handler(Arc::new(Decision(ShellApprovalDecision::AllowOnce))));
        let approver_directory = directory.path().to_path_buf();
        let mut approver_runtime = runtime.clone_for_child();
        let approver = std::thread::spawn(move || template_attempt(&approver_directory, &approver_options, &mut approver_runtime));
        entered_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        let (result_tx, result_rx) = mpsc::sync_channel(1);
        let waiter_directory = directory.path().to_path_buf();
        let mut waiter_runtime = runtime.clone_for_child();
        let waiter = std::thread::spawn(move || {
            let result = template_attempt(&waiter_directory, &waiter_options, &mut waiter_runtime)
                .and_then(|prepared| super::super::execute_prepared_outcome(&prepared, &waiter_options, &waiter_runtime));
            result_tx.send(result).unwrap();
        });
        let outcome = result_rx.recv_timeout(Duration::from_secs(3)).expect("failed approval must wake its waiting peer").unwrap();
        assert_eq!(outcome.outcome.status, ShellStatus::Exited(0));
        assert!(!outcome.outcome.stdout.is_empty());
        assert!(matches!(approver.join().unwrap(), Err(ShellExpansionError::PolicyIo { .. })));
        waiter.join().unwrap();
    }

}
