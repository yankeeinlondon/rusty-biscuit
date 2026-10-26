//! `md compose` as Darkmatter's core-only lifecycle host.
//!
//! A root document that authors `initialize`, `composed`, `failed`,
//! `cancelled`, or `finalize` composes as a lifecycle run: the shell-free
//! frontmatter bootstrap, `initialize`, reference validation with the shell
//! approval and composition, `composed`, the document write, then
//! `finalize`. The run itself is [`darkmatter::lifecycle::md_host::drive`];
//! this module performs its requests on the terminal and the filesystem.
//!
//! - `stdout` actions and lifecycle command output share stdout with the
//!   document, in run order; every other terminal verb writes to stderr.
//! - Every step reuses the invocation's one [`ComposeRequest`]: nothing here
//!   captures a context of its own.
//! - The first Ctrl-C cancels the run (cleanup still runs, exit 130); a
//!   second one exits at once.

use std::cell::OnceCell;
use std::io::{IsTerminal as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Report, Result, eyre};
use darkmatter::effects::EffectEngine;
use darkmatter::lifecycle::md_host::{
    self, CommandOutput, MdEvent, MdRunHost, MdRuntimeState, MdStackContext,
};
use darkmatter::lifecycle::{
    LifecycleConfig, LifecycleEffects, LifecycleErrorInfo, LifecycleSignal, LifecycleTiming, render_prose_line,
    render_status_line,
};
use darkmatter::lifecycle::executor::LifecycleEventOutcome;
use darkmatter::lifecycle::run::ApprovalEpoch;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals;
use darkmatter::markdown::compose::shell_expansion::{ShellApprovalHandler, ShellCommandEntry, ShellRuleSet};
use darkmatter::markdown::compose::report_diagnostics::lifecycle_failure_diagnostics;
use darkmatter::markdown::compose::{ComposeOperation, ComposeReport, ComposeRequest, ComposeWarning};
use serde_json::{Map, Value};

use crate::args::policy::ResolvedPolicy;

use super::{
    DocumentOutput, PendingTriggerWarnings, ReferenceCheck, blocked_diagnostics, check_references, compose_error,
    open_output_artifact, preflight_approval_error, render_reference_report, render_reference_warnings,
    report_compose_diagnostics,
};
use super::report::ReportedExit;

/// The root document's lifecycle under the `md` profile, or `None` when it
/// authors none of `md`'s events.
///
/// ## Errors
///
/// A malformed lifecycle, before anything runs.
pub(super) fn root_lifecycle(md: &Markdown, source_path: &Path) -> Result<Option<LifecycleConfig>> {
    let frontmatter = frontmatter_value(md);
    if !md_host::authors_lifecycle(&frontmatter) {
        return Ok(None);
    }
    md_host::parse_lifecycle(&frontmatter, source_path, Some(md.frontmatter()))
        .map(Some)
        .map_err(Report::new)
}

/// Rejects a lifecycle call the design-time check proves wrong, against the
/// effective schema `request` composes `md` with, before the run starts.
///
/// The skipped triggers the schema's discovery found are held in
/// `pending_triggers`, before the check can reject.
///
/// ## Returns
///
/// The lifecycle surfaces' call warnings (category 3, always-false calls),
/// which the caller reports with composition's own.
///
/// ## Errors
///
/// The first rejected call, or a schema composition would refuse.
pub(super) fn reject_incompatible_calls(
    md: &Markdown,
    config: &LifecycleConfig,
    source_path: &Path,
    request: &ComposeRequest,
    pending_triggers: &mut PendingTriggerWarnings,
) -> Result<Vec<ComposeWarning>> {
    let (schema, triggers) = darkmatter::markdown::compose::effective_schema_for(md, request).map_err(compose_error)?;
    pending_triggers.hold(&triggers);
    darkmatter::lifecycle::reject_incompatible_calls(
        &frontmatter_value(md),
        config,
        source_path,
        md.loaded_text(),
        md_host::registry(),
        schema.as_ref(),
    )
    .map_err(Report::new)
}

/// The keys composition keeps deferred for a lifecycle run.
pub(super) fn event_keys() -> Vec<&'static str> {
    md_host::registry().authored_event_keys()
}

/// The lifecycle's shell commands for the report-only `--shell` mode,
/// collected without running a stack.
pub(super) fn report_candidates(config: &LifecycleConfig, source_path: &Path) -> Vec<ShellCommandEntry> {
    md_host::lifecycle_shell_sources(config, source_path).entries
}

fn frontmatter_value(md: &Markdown) -> Value {
    Value::Object(md.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect())
}

/// Everything a lifecycle compose needs from `run_compose`.
pub(super) struct LifecycleCompose<'a> {
    pub(super) md: &'a Markdown,
    pub(super) request: ComposeRequest,
    pub(super) config: LifecycleConfig,
    pub(super) source_path: PathBuf,
    pub(super) approval_handler: Option<Arc<dyn ShellApprovalHandler>>,
    /// The policy, when the root is a file whose references are checked.
    pub(super) check_references: Option<&'a ResolvedPolicy>,
    pub(super) document: DocumentOutput<'a>,
    pub(super) verbose: u8,
    /// The lifecycle surfaces' call warnings ([`reject_incompatible_calls`]),
    /// reported ahead of composition's own.
    pub(super) call_warnings: Vec<ComposeWarning>,
    /// The skipped triggers the call check found, which the bootstrap adds to.
    pub(super) pending_triggers: PendingTriggerWarnings,
}

/// Runs the lifecycle and exits with its status when it did not succeed.
///
/// ## Errors
///
/// A bootstrap failure, before `initialize` runs.
pub(super) fn run(mut compose: LifecycleCompose<'_>) -> Result<()> {
    let sink = compose.document.sink;
    let bootstrap = bootstrap_frontmatter(compose.md, &compose.request, &mut compose.pending_triggers, sink)?;
    let interrupted = install_interrupt_handler();
    ignore_broken_pipe_signal();
    let term = Terminal::default();
    let effects = TerminalEffects {
        term: &term,
        stdout_is_terminal: std::io::stdout().is_terminal(),
        stdout_is_report: sink.is_some(),
    };
    let mut host = ComposeHost::new(compose, bootstrap, &effects, interrupted);
    let config = host.config.clone();
    let end = md_host::drive(&mut host, &config);
    // A run that never reached composition (`initialize` skipped or failed)
    // still delivers what the bootstrap found.
    let term_cell = OnceCell::new();
    let _ = term_cell.set(term.clone());
    host.pending_triggers.deliver(sink, None, &term_cell);
    if let Some(error) = &end.error
        && host.reported.as_deref() != Some(error.msg.as_str())
    {
        match sink {
            Some(sink) => sink.diagnostics(lifecycle_failure_diagnostics(error)),
            None => effects.status(StatusState::Error, &error.msg),
        }
    }
    let code = match end.exit_code() {
        // Only while the refusal is still the selected error; a cleanup error
        // that replaced it exits as any other failure.
        1 if host.refused_references && end.error.as_ref().map(|error| &error.msg) == host.reported.as_ref() => 2,
        code => code,
    };
    if code != 0 {
        if sink.is_some() {
            return Err(ReportedExit(code).into());
        }
        let _ = std::io::stderr().flush();
        std::process::exit(code);
    }
    Ok(())
}

/// The shell-free frontmatter bootstrap `initialize` reads: the frontmatter
/// surface only, with frontmatter `$( … )` disabled and the event keys still
/// deferred.
///
/// Its skipped triggers join `pending_triggers`; a failed bootstrap delivers
/// them with its failure.
fn bootstrap_frontmatter(
    md: &Markdown,
    request: &ComposeRequest,
    pending_triggers: &mut PendingTriggerWarnings,
    sink: Option<&super::report::ReportSink>,
) -> Result<Map<String, Value>> {
    let bootstrap = request.clone().map_options(|options| {
        options
            .with_deferred_schema_verdict(true)
            .only_frontmatter_surface()
            .disable(ComposeOperation::FrontmatterShellExpansion)
            .with_pre_approved_commands(ShellPreApprovals::new())
    });
    let (projected, report) = md.compose_with_warnings(&bootstrap).map_err(|failure| {
        pending_triggers.hold(&failure.warnings);
        pending_triggers.deliver(sink, failure.error.blocked_diagnostics(), &OnceCell::new());
        compose_error(failure.error)
    })?;
    pending_triggers.hold(&report.warnings);
    Ok(projected.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect())
}

/// The process's interrupt flag. The first Ctrl-C sets it; a second exits
/// with the interrupted status without waiting for cleanup.
fn install_interrupt_handler() -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    let handler_flag = Arc::clone(&flag);
    // A handler that cannot install leaves the run uninterruptible by
    // cleanup, as `md compose` was before; Ctrl-C then ends it outright.
    let _ = ctrlc::set_handler(move || {
        if handler_flag.swap(true, Ordering::SeqCst) {
            std::process::exit(130);
        }
    });
    flag
}

/// A write to a closed stdout must fail as an error the run reports, then
/// `finalize`, instead of the signal ending the process mid-run.
#[cfg(unix)]
fn ignore_broken_pipe_signal() {
    // SAFETY: setting a signal's disposition to SIG_IGN installs no handler
    // code and is async-signal-safe.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }
}

#[cfg(not(unix))]
fn ignore_broken_pipe_signal() {}

/// Lifecycle communication on the terminal.
struct TerminalEffects<'t> {
    term: &'t Terminal,
    /// Off a terminal, stdout carries the Markdown document, so a `stdout`
    /// action's line joins it as authored rather than styled.
    stdout_is_terminal: bool,
    /// `--output report-json`: stdout carries only the report, so a `stdout`
    /// action's line and a command's output go to stderr.
    stdout_is_report: bool,
}

impl TerminalEffects<'_> {
    fn status(&self, state: StatusState, text: &str) {
        eprintln!("{}", render_status_line(state, text, self.term));
    }

    fn prose_to_stderr(&self, text: &str) {
        eprintln!("{}", render_prose_line(text, self.term));
    }
}

impl LifecycleEffects for TerminalEffects<'_> {
    fn stdout(&self, text: &str) {
        if self.stdout_is_report {
            self.prose_to_stderr(text);
            return;
        }
        let mut stdout = std::io::stdout().lock();
        // A closed stdout fails the document write, which the run reports;
        // an action's own line is not an operation of its own.
        let line = if self.stdout_is_terminal { render_prose_line(text, self.term) } else { text.to_string() };
        let _ = writeln!(stdout, "{line}").and_then(|()| stdout.flush());
    }

    fn stderr(&self, _signal: LifecycleSignal, text: &str) {
        self.prose_to_stderr(text);
    }

    fn info(&self, text: &str) {
        self.status(StatusState::Info, text);
    }

    fn warn(&self, text: &str) {
        self.status(StatusState::Warning, text);
    }

    fn success(&self, text: &str) {
        self.status(StatusState::Success, text);
    }

    fn message(&self, text: &str) {
        self.prose_to_stderr(text);
    }

    // `md` parses `say`, `effect`, and `notify` as ignored actions, so these
    // are never reached.
    fn speech(&self, _text: &str) {}

    fn effect(&self, _name: &str) {}

    fn notification(&self, _title: &str) {}
}

impl CommandOutput for TerminalEffects<'_> {
    fn stdout(&self, text: &str) {
        if self.stdout_is_report {
            let _ = std::io::stderr().write_all(text.as_bytes());
            return;
        }
        let mut stdout = std::io::stdout().lock();
        let _ = stdout.write_all(text.as_bytes()).and_then(|()| stdout.flush());
    }

    fn stderr(&self, text: &str) {
        let _ = std::io::stderr().write_all(text.as_bytes());
    }
}

/// The work of one `md compose` lifecycle run.
struct ComposeHost<'a> {
    md: &'a Markdown,
    request: ComposeRequest,
    config: LifecycleConfig,
    source_path: PathBuf,
    approval_handler: Option<Arc<dyn ShellApprovalHandler>>,
    check_references: Option<&'a ResolvedPolicy>,
    document: DocumentOutput<'a>,
    verbose: u8,
    call_warnings: Vec<ComposeWarning>,
    /// Delivered with the first report or failure of the composition pass,
    /// or at the run's end when it never composes.
    pending_triggers: PendingTriggerWarnings,
    frontmatter: Map<String, Value>,
    live: Mutex<Map<String, Value>>,
    state: MdRuntimeState,
    approved: ShellPreApprovals,
    user_blacklist: ShellRuleSet,
    effect_engine: EffectEngine,
    effects: &'a TerminalEffects<'a>,
    interrupted: Arc<AtomicBool>,
    started: Instant,
    composed: Option<(Markdown, ComposeReport)>,
    deferred_report: Option<darkmatter::markdown::reference::validate::ReferenceValidationReport>,
    /// The message of the failure already rendered in full, so the run's end
    /// does not repeat it.
    reported: Option<String>,
    /// Unresolved references refused the document; the failed run keeps the
    /// non-lifecycle path's exit status 2.
    refused_references: bool,
}

impl<'a> ComposeHost<'a> {
    fn new(
        compose: LifecycleCompose<'a>,
        bootstrap: Map<String, Value>,
        effects: &'a TerminalEffects<'a>,
        interrupted: Arc<AtomicBool>,
    ) -> Self {
        let mutation_root = compose
            .source_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map_or_else(|| compose.request.resolution_context().cwd().to_path_buf(), Path::to_path_buf);
        let user_blacklist = md_host::user_blacklist(&compose.request);
        Self {
            md: compose.md,
            request: compose.request,
            config: compose.config,
            source_path: compose.source_path,
            approval_handler: compose.approval_handler,
            check_references: compose.check_references,
            document: compose.document,
            verbose: compose.verbose,
            call_warnings: compose.call_warnings,
            pending_triggers: compose.pending_triggers,
            live: Mutex::new(bootstrap.clone()),
            frontmatter: bootstrap,
            state: MdRuntimeState::default(),
            approved: ShellPreApprovals::new(),
            user_blacklist,
            effect_engine: EffectEngine::builder().mutation_root(mutation_root).build(),
            effects,
            interrupted,
            started: Instant::now(),
            composed: None,
            deferred_report: None,
            reported: None,
            refused_references: false,
        }
    }

    /// Renders `report` in full and returns the error the run selects.
    fn fail(&mut self, verb: &str, report: Report) -> LifecycleErrorInfo {
        match self.document.sink {
            Some(sink) => sink.failure(&report),
            None => eprintln!("{}", render_report(&report)),
        }
        let error = LifecycleErrorInfo::from_action_failure_with(&md_host::MdDiagnostics, verb, report.to_string());
        self.reported = Some(error.msg.clone());
        error
    }

    fn try_prepare(&mut self) -> std::result::Result<(), (&'static str, Report)> {
        if let Some(allow) = self.check_references {
            match check_references(self.md, &self.request, allow) {
                ReferenceCheck::Clean => {}
                ReferenceCheck::Report(report) => {
                    self.pending_triggers.hold(&report.trigger_warnings);
                    if let Some(sink) = self.document.sink {
                        sink.references(&report);
                    }
                    self.deferred_report = Some(report);
                }
                ReferenceCheck::Refused(report) => {
                    self.pending_triggers.hold(&report.trigger_warnings);
                    let term_cell = OnceCell::new();
                    let _ = term_cell.set(self.effects.term.clone());
                    self.pending_triggers.deliver(self.document.sink, None, &term_cell);
                    match self.document.sink {
                        Some(sink) => sink.references(&report),
                        None => {
                            eprint!("{}", render_reference_report(&report, self.effects.term));
                            eprint!("{}", render_reference_warnings(&report, self.effects.term));
                        }
                    }
                    self.refused_references = true;
                    return Err(("references", eyre!("the document has unresolved references")));
                }
            }
        }

        let effective = Value::Object(self.live.lock().expect("live frontmatter lock").clone());
        md_host::stamp_lifecycle_shells(&mut self.config, &effective, self.md, &self.request, &self.source_path)
            .map_err(|error| ("shell", Report::new(error)))?;

        // `set` values from `initialize` are data to the first composition.
        let mutations = self.state.snapshot();
        if !mutations.is_empty() {
            let data: Map<String, Value> = mutations.into_iter().collect();
            self.request = self.request.clone().map_options(|options| options.with_data_overrides(Value::Object(data)));
        }

        let extra = md_host::lifecycle_shell_sources(&self.config, &self.source_path);
        let options = self.request.options();
        let body_shells = options.is_enabled(ComposeOperation::ShellExpansion)
            || options.is_enabled(ComposeOperation::ShellBlocks)
            || options.is_enabled(ComposeOperation::FrontmatterShellExpansion);
        // Lifecycle shells are audited whether or not a body shell operation
        // is enabled; disabling one never authorizes the other.
        if body_shells || !extra.entries.is_empty() || !extra.candidates.is_empty() {
            let approvals = self
                .md
                .compose_preflight_approvals_with(&self.request, self.approval_handler.clone(), extra)
                .map_err(|error| {
                    let mut warnings = self.request.options().take_preflight_failure_warnings();
                    self.pending_triggers.merge_into(&mut warnings);
                    match self.document.sink {
                        Some(sink) => sink.warnings(&warnings),
                        None => super::report_failure_trigger_warnings(
                            &warnings,
                            super::preflight_blocked(&error),
                            &OnceCell::new(),
                        ),
                    }
                    ("shell", preflight_approval_error(error))
                })?;
            // A trigger only a child composition skips holds is reported by
            // this walk alone, whatever the bootstrap found.
            self.pending_triggers.hold(&approvals.warnings);
            if self.verbose > 0 {
                eprintln!(
                    "pre-flight: {} discovered, {} whitelisted, {} approved",
                    approvals.stats.total_discovered, approvals.stats.already_whitelisted, approvals.stats.user_approved
                );
            }
            md_host::settle_lifecycle_shells(&self.config, &approvals.pre_approved_commands, &self.request);
            self.approved = approvals.pre_approved_commands.clone();
            self.request = self.request.clone().map_options(|options| {
                options
                    .with_pre_approved_commands(approvals.pre_approved_commands)
                    .with_preflight_graph(approvals.preflight_graph)
            });
        }

        let (composed, mut report) =
            self.md.compose_with_warnings(&self.request).map_err(|mut failure| {
                self.pending_triggers.merge_into(&mut failure.warnings);
                match self.document.sink {
                    Some(sink) => sink.warnings(&failure.warnings),
                    None => super::report_failure_trigger_warnings(
                        &failure.warnings,
                        failure.error.blocked_diagnostics(),
                        &OnceCell::new(),
                    ),
                }
                ("compose", compose_error(failure.error))
            })?;
        self.pending_triggers.merge_into(&mut report.warnings);
        if let Some(sink) = self.document.sink {
            sink.warnings(&report.warnings);
        }
        report.warnings.splice(0..0, self.call_warnings.iter().cloned());
        // Composition stops at a blocking warning before its next effect; one
        // reported by its last stages stops the run here, before `write`.
        blocked_diagnostics(self.request.options().blocking_policy(), &report.warnings, &[])
            .map_err(|error| ("compose", error))?;
        self.frontmatter =
            composed.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect();
        let mut live = self.live.lock().expect("live frontmatter lock");
        for (key, value) in &self.frontmatter {
            live.insert(key.clone(), value.clone());
        }
        drop(live);
        self.composed = Some((composed, report));
        Ok(())
    }

    fn try_write(&self) -> Result<()> {
        let (composed, report) = self.composed.as_ref().expect("the run writes only after a preparation");
        let artifact = self.document.artifact(composed)?;
        if let Some(sink) = self.document.sink {
            sink.document(artifact.content);
            return Ok(());
        }
        if self.document.prints() {
            let mut stdout = std::io::stdout().lock();
            writeln!(stdout, "{}", artifact.content)
                .and_then(|()| stdout.flush())
                .map_err(|error| eyre!("failed to write the composed document: {error}"))?;
        }
        if self.document.show {
            open_output_artifact(&artifact)?;
        }
        let term_cell = OnceCell::new();
        let _ = term_cell.set(self.effects.term.clone());
        report_compose_diagnostics(report, self.deferred_report.as_ref(), &term_cell);
        Ok(())
    }
}

impl MdRunHost for ComposeHost<'_> {
    fn event(&mut self, event: MdEvent) -> LifecycleEventOutcome {
        let interrupted = Arc::clone(&self.interrupted);
        let probe = move || interrupted.load(Ordering::SeqCst);
        let timing = LifecycleTiming::from_instants(self.started, Some(self.started), Instant::now());
        let context = MdStackContext {
            request: &self.request,
            source_path: &self.source_path,
            frontmatter: &self.frontmatter,
            live_frontmatter: &self.live,
            state: &self.state,
            approved: &self.approved,
            user_blacklist: &self.user_blacklist,
            effect_engine: &self.effect_engine,
            effects: self.effects,
            output: self.effects,
            timing: &timing,
            interrupted: &probe,
        };
        md_host::evaluate(&context, &self.config, &event)
    }

    fn prepare(&mut self) -> std::result::Result<ApprovalEpoch, LifecycleErrorInfo> {
        match self.try_prepare() {
            Ok(()) => Ok(ApprovalEpoch(1)),
            Err((verb, report)) => {
                // A preparation that stopped before composing (a lifecycle
                // shell that cannot be stamped) delivers the bootstrap's
                // findings ahead of its failure; refused references
                // delivered them with their own report.
                let term_cell = OnceCell::new();
                let _ = term_cell.set(self.effects.term.clone());
                self.pending_triggers.deliver(self.document.sink, None, &term_cell);
                Err(self.fail(verb, report))
            }
        }
    }

    fn write(&mut self) -> std::result::Result<(), LifecycleErrorInfo> {
        self.try_write().map_err(|report| {
            // The composition's warnings follow the document; one that could
            // not be written still delivers its skipped triggers.
            if self.document.sink.is_none()
                && let Some((_, composed_report)) = &self.composed
            {
                let term_cell = OnceCell::new();
                let _ = term_cell.set(self.effects.term.clone());
                super::report_failure_trigger_warnings(&composed_report.warnings, None, &term_cell);
            }
            LifecycleErrorInfo::from_action_failure_with(&md_host::MdDiagnostics, "write", report.to_string())
        })
    }

    fn interrupted(&self) -> bool {
        self.interrupted.load(Ordering::SeqCst)
    }
}

/// `report` as `md` renders an error: the styled block of a Darkmatter error
/// in its chain, or the chain's messages.
fn render_report(report: &Report) -> String {
    if let Some(block) = report.chain().find_map(darkmatter::markdown::errors::as_block_error) {
        return if std::io::stderr().is_terminal() {
            block.report_block_error(&Terminal::new())
        } else {
            block.report_block_error_optimistic(None)
        };
    }
    let mut rows = vec![format!("Error: {report}")];
    let mut seen = report.to_string();
    for cause in report.chain().skip(1) {
        let message = cause.to_string();
        if !seen.contains(&message) {
            seen.push_str(&message);
            rows.push(format!("  ▸ {message}"));
        }
    }
    rows.join("\n")
}
