//! `md compose` subcommand implementation: positional argument parsing, the
//! compose pipeline runner, the shell-command report, and the compose
//! performance report.

use crate::args::{Cli, OutputFormat};
use crate::artifact::{
    OutputArtifact, html_artifact, json_artifact, markdown_plus_artifact,
    open_output_artifact,
};
use crate::io::{load_markdown, open_argument, resolve_file_path};
use crate::request::MdRequest;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::cleanup::rules::{Indentation, ListItemPadding};
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest};
use std::path::{Path, PathBuf};
use tracing::{info, instrument};

mod lifecycle;

/// Resolved allow flags for compose validation.
pub struct ComposeAllowFlags {
    pub hyperlinks: bool,
    pub image_refs: bool,
    pub transclusions: bool,
}

impl ComposeAllowFlags {
    /// Returns `true` if no allow flags are set (strict mode).
    fn is_strict(&self) -> bool {
        !self.hyperlinks && !self.image_refs && !self.transclusions
    }

    /// Returns `true` if the given issue kind is allowed by the current flags.
    fn is_allowed(&self, kind: darkmatter::markdown::reference::types::ReferenceKind) -> bool {
        use darkmatter::markdown::reference::types::ReferenceKind;
        match kind {
            ReferenceKind::Hyperlink => self.hyperlinks,
            ReferenceKind::Image => self.image_refs,
            ReferenceKind::Transclusion => self.transclusions,
            _ => false,
        }
    }
}

// ── Compose positional classification ─────────────────────────────────

#[derive(Debug)]
pub(crate) struct ParsedComposeArgs {
    pub(crate) input: Option<PathBuf>,
    pub(crate) shorthand_setters: serde_json::Map<String, serde_json::Value>,
}

fn parse_compose_setter(
    token: &str,
) -> Option<std::result::Result<(String, serde_json::Value), String>> {
    let eq_pos = token.find('=')?;
    let key = &token[..eq_pos];
    let raw_value = &token[eq_pos + 1..];

    if key.is_empty() {
        return Some(Err("setter key must not be empty".to_string()));
    }

    let mut chars = key.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return None;
    }

    for ch in chars {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            continue;
        }
        return None;
    }

    let value = parse_shorthand_value(raw_value);
    Some(Ok((key.to_string(), value)))
}

fn parse_shorthand_value(raw: &str) -> serde_json::Value {
    if raw.is_empty() {
        return serde_json::Value::String(String::new());
    }
    match biscuit_file::Json5::from_str(raw) {
        Ok(parsed) if darkmatter::markdown::numeric::reject_non_finite_json5(&parsed).is_ok() => {
            parsed.value().clone()
        }
        _ => serde_json::Value::String(raw.to_string()),
    }
}

pub(crate) fn parse_compose_positionals(args: &[String]) -> Result<ParsedComposeArgs> {
    let mut input: Option<PathBuf> = None;
    let mut shorthand_setters = serde_json::Map::new();

    for token in args {
        match parse_compose_setter(token) {
            Some(Ok((key, value))) => {
                shorthand_setters.insert(key, value);
            }
            Some(Err(e)) => {
                return Err(eyre!("Invalid setter '{}': {}", token, e));
            }
            None => {
                if input.is_some() {
                    return Err(eyre!(
                        "expected at most one input path, but got multiple: {}",
                        args.iter()
                            .filter(|t| parse_compose_setter(t).is_none())
                            .map(|t| t.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                input = Some(PathBuf::from(token));
            }
        }
    }

    Ok(ParsedComposeArgs {
        input,
        shorthand_setters,
    })
}

pub(crate) fn build_remote_read_config(
    allowed_hosts: &[String],
    concurrency: Option<usize>,
    ttl_secs: Option<u64>,
    refresh: bool,
    freshness: crate::args::RemoteFreshness,
) -> darkmatter::markdown::compose::RemoteReadConfig {
    use crate::args::RemoteFreshness;
    use darkmatter::markdown::compose::{RemoteFreshnessMode, resolve_remote_concurrency};

    let freshness_mode = match freshness {
        RemoteFreshness::Optimistic => RemoteFreshnessMode::Optimistic,
        RemoteFreshness::Strict => RemoteFreshnessMode::Strict,
        RemoteFreshness::Fallback => RemoteFreshnessMode::Fallback,
    };

    darkmatter::markdown::compose::RemoteReadConfig {
        allowed_hosts: allowed_hosts.to_vec(),
        // Precedence: CLI `--remote-concurrency` > env var > spec default.
        remote_concurrency: resolve_remote_concurrency(concurrency),
        remote_ttl: ttl_secs.map(std::time::Duration::from_secs),
        refresh,
        freshness_mode,
    }
}

/// Run the compose pipeline.
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all, fields(command = "compose"))]
pub fn run_compose(
    input: Option<&PathBuf>,
    state_json: Option<&str>,
    set_json: Option<&str>,
    shorthand_setters: serde_json::Map<String, serde_json::Value>,
    output: OutputFormat,
    show: bool,
    include_frontmatter: bool,
    list_spacing: ListItemPadding,
    indent: Option<usize>,
    allow: &ComposeAllowFlags,
    allow_ctx_override: bool,
    allow_invalid_frontmatter_assignment: bool,
    allow_reassigned_frontmatter_property: bool,
    baseline_schema: Option<&PathBuf>,
    no_baseline_schema: bool,
    no_schema_triggers: bool,
    timeout_secs: Option<u64>,
    allow_shell_timeout: bool,
    shell_report: bool,
    perf: bool,
    remote_config: darkmatter::markdown::compose::RemoteReadConfig,
    cache_root: Option<&PathBuf>,
    cli: &Cli,
    request: &MdRequest,
) -> Result<()> {
    info!("starting compose pipeline");
    use biscuit_terminal::terminal::Terminal;
    use std::time::Instant;

    let cmd_start = perf.then(Instant::now);

    // Detect the terminal once per invocation and reuse it across every
    // human-rendered branch (strict validation report, verbose summary, `-vv`
    // perf metrics, warnings footer, deferred validation report). Built lazily
    // via `OnceCell` so pure Markdown/HTML/JSON output never detects a terminal
    // at all — finding 3 (`run_compose` previously constructed up to four fresh
    // `Terminal::default()`s per invocation). A per-invocation local rather than
    // a process-global `LazyLock`, so tests and library hosts that change the
    // cwd/env/attached streams within one process still detect afresh.
    let term_cell: std::cell::OnceCell<Terminal> = std::cell::OnceCell::new();

    let launch_dir = request.launch_dir().to_path_buf();

    // Resolve the input path once through FileReference (handles @-prefixed paths)
    // and reuse for both loading and source_file/policy_root.
    let resolve_start = perf.then(Instant::now);
    let opened_input = if let Some(path) = input
        && path.to_str() != Some("-")
    {
        Some(open_argument(path, request)?)
    } else {
        None
    };
    let resolved_input = opened_input.as_ref().map(|opened| opened.path().to_path_buf());
    let resolve_input_dur = resolve_start.map(|s| s.elapsed()).unwrap_or_default();

    let load_start = perf.then(Instant::now);
    let md = if let Some(ref resolved) = resolved_input {
        Markdown::try_from(resolved.as_path())
            .wrap_err_with(|| format!("Failed to load file: {:?}", resolved))?
    } else {
        load_markdown(None, request)?
    };
    let load_input_dur = load_start.map(|s| s.elapsed()).unwrap_or_default();
    // One context for validation, pre-flight, and compose: the document's own
    // (derived from the launch context, or built at the document when it lies
    // in another repository), or the launch context itself for stdin. It
    // derives from the input as authored, so a quoted `~/…` or `{{VAR}}/…`
    // argument supplies the document's tree root (an unquoted `~` was already
    // expanded by the shell and carries no anchor).
    let file_resolution_context = match &opened_input {
        Some(opened) => opened.document_context(request)?,
        None => request.launch_context()?.clone(),
    };

    // The request boundary (D3): one demand-driven context capture for the
    // document plus the request's repository observation, both anchored on the
    // launch directory and fixed before validation, pre-flight, and compose
    // run against the same options.
    // A root that authors one of `md`'s events composes as a lifecycle run.
    // Its stacks are parsed now, before anything can run, and stay deferred
    // through every interpolation pass.
    let lifecycle_source = resolved_input.clone().unwrap_or_else(|| launch_dir.join("<stdin>"));
    let lifecycle = lifecycle::root_lifecycle(&md, &lifecycle_source)?;

    let ctx_start = perf.then(Instant::now);
    let mut options = ComposeOptions::for_document(&launch_dir, &md);
    let capture_context_dur = ctx_start.map(|s| s.elapsed()).unwrap_or_default();
    if lifecycle.is_some() {
        options = options.with_exclude_keys(lifecycle::event_keys());
    }
    // Keep a cheap Arc clone for perf report context timings
    let options_ctx_ref = options.context().clone();

    // ── Parse --state and --set early ────────────────────────────────────
    // These must be available before reference validation so that
    // interpolation inside transclusion targets (e.g., `::file @{{ctx.pkg}}/{{plan}}`)
    // can resolve user-provided variables during the validation pass.
    let opts_start = perf.then(Instant::now);
    options = apply_compose_baseline_schema(options, baseline_schema, no_baseline_schema, request)?;
    options = options.with_trigger_schemas(!no_schema_triggers);

    // Parse --state as JSON or JSON5
    if let Some(json_str) = state_json {
        let parsed = biscuit_file::Json5::from_str(json_str)
            .wrap_err("Invalid JSON/JSON5 in --state argument")?;
        darkmatter::markdown::numeric::reject_non_finite_json5(&parsed)
            .map_err(|reason| eyre!("Invalid --state argument: a number is {reason}"))?;
        let state = parsed.value().clone();
        if !state.is_object() {
            return Err(eyre!(
                "Invalid --state argument: expected a JSON object like {{\"name\":\"Alice\"}}"
            ));
        }
        options = options.with_external_state(state);
    }

    // Parse --set as JSON or JSON5
    let mut override_map: serde_json::Map<String, serde_json::Value> =
        if let Some(json_str) = set_json {
            let parsed = biscuit_file::Json5::from_str(json_str)
                .wrap_err("Invalid JSON/JSON5 in --set argument")?;
            darkmatter::markdown::numeric::reject_non_finite_json5(&parsed)
                .map_err(|reason| eyre!("Invalid --set argument: a number is {reason}"))?;
            let set = parsed.value().clone();
            if let serde_json::Value::Object(map) = set {
                map
            } else {
                return Err(eyre!(
                    "Invalid --set argument: expected a JSON object like {{\"name\":\"Alice\"}}"
                ));
            }
        } else {
            serde_json::Map::new()
        };

    for (key, value) in shorthand_setters {
        override_map.insert(key, value);
    }

    if !override_map.is_empty() {
        options = options
            .with_file_ref_fallback_dir(&launch_dir)
            .with_set_overrides(serde_json::Value::Object(override_map));
    }
    let prepare_request = |options: ComposeOptions| -> Result<ComposeRequest> {
        ComposeRequest::with_context(options, file_resolution_context.clone())
            .wrap_err("Failed to prepare the compose request")
    };

    // ── Reference validation ───────────────────────────────────────────
    // Validate before composing so broken references are caught early.
    // Uses the same options (including --state/--set) so interpolation
    // inside transclusion targets resolves correctly.
    let val_start = perf.then(Instant::now);
    // A lifecycle run checks references after `initialize`, which may create
    // the files the body includes.
    let deferred_report = if resolved_input.is_some() && lifecycle.is_none() {
        match check_references(&md, &prepare_request(options.clone())?, allow) {
            ReferenceCheck::Clean => None,
            ReferenceCheck::Report(report) => Some(report),
            ReferenceCheck::Refused(report) => {
                let term = term_cell.get_or_init(Terminal::default);
                eprint!("{}", render_reference_report(&report, term));
                std::process::exit(2);
            }
        }
    } else {
        None
    };
    let validate_refs_dur = val_start.map(|s| s.elapsed()).unwrap_or_default();

    // Set source file for relative transclusion resolution
    if let Some(ref resolved) = resolved_input {
        options = options.with_source_file(resolved);
    }

    // Build shell expansion options. The per-directive `approval_handler`
    // is intentionally left `None` on the runtime shell options: the CLI
    // runs the v2 pre-flight approval lifecycle (`compose_preflight` →
    // policy check → batched prompt → `with_pre_approved_commands`) just
    // before `compose_with`, so the in-stage handler is no longer the
    // approval path. The pre-flight handler is held locally and used once.
    use darkmatter::markdown::compose::shell_expansion::ShellExpansionOptions;
    use std::sync::Arc;

    let is_file_input = resolved_input.is_some();

    let shell_opts = ShellExpansionOptions {
        timeout: timeout_secs
            .map(std::time::Duration::from_secs)
            .unwrap_or(std::time::Duration::from_secs(10)),
        // Shell policy lives beside the document, or in the launch directory
        // for stdin, so pre-flight and execution read the same files and a
        // stdin run never falls through to the user's home.
        policy_root: Some(
            resolved_input
                .as_ref()
                .and_then(|p| p.parent())
                .filter(|parent| !parent.as_os_str().is_empty())
                .map_or_else(|| launch_dir.clone(), Path::to_path_buf),
        ),
        approval_handler: None,
        ..Default::default()
    };

    options = options.with_shell(shell_opts);

    // The handler used for the single batched pre-flight prompt. Built once
    // so we can hand it to `compose_preflight_approvals` (and reuse it on
    // the off-chance a future re-preflight needs to run in the same flow).
    let preflight_handler: Option<Arc<dyn darkmatter::markdown::compose::shell_expansion::ShellApprovalHandler>> =
        if is_file_input && crate::approval::can_prompt_interactively() {
            Some(Arc::new(crate::approval::CliShellApprovalHandler))
        } else {
            None
        };
    if allow_shell_timeout {
        options = options.with_allow_shell_timeout(true);
    }
    options = options.with_list_spacing(list_spacing);
    options = options.with_allow_ctx_override(allow_ctx_override);
    options =
        options.with_allow_invalid_frontmatter_assignment(allow_invalid_frontmatter_assignment);
    options =
        options.with_allow_reassigned_frontmatter_property(allow_reassigned_frontmatter_property);
    options = options.with_perf(perf);
    options = options.with_remote_read_config(remote_config);
    options = options.with_allow_remote_transclusion(true);
    if let Some(cache_root) = cache_root {
        options = options.with_cache_root(cache_root);
    }
    // `--indent` only parses 2, 4, or 8.
    if let Some(indentation) = indent.and_then(Indentation::from_spaces) {
        options = options.with_indentation(indentation);
    }
    // Share one single-flight remote-fetch runtime between the pre-flight
    // approval walk and the compose pass so each remote URL is fetched once,
    // not once per stage. Must follow the remote-read-config and cache-root
    // wiring above so the shared runtime inherits both.
    options = options.with_shared_remote_fetch();
    let mut compose_request = prepare_request(options)?;
    let build_options_dur = opts_start.map(|s| s.elapsed()).unwrap_or_default();

    if shell_report {
        // `--shell` reports condition-blind approval candidates: every command
        // that *could* run under any document state, routed through the same
        // pre-flight collector that authorization uses.
        let mut preflight = md.compose_preflight(&compose_request)?;
        if let Some(config) = &lifecycle {
            // Report-only: the stacks are read, never run.
            preflight.entries.extend(lifecycle::report_candidates(config, &lifecycle_source));
        }
        print_shell_command_report(&preflight.entries);
        print_icmp_effect_report(&preflight.icmp_probes);
        drop(options_ctx_ref);
        return Ok(());
    }

    let document = DocumentOutput {
        output,
        show,
        include_frontmatter,
        cli,
        input,
    };

    if let Some(config) = lifecycle {
        drop(options_ctx_ref);
        return lifecycle::run(lifecycle::LifecycleCompose {
            md: &md,
            request: compose_request,
            config,
            source_path: lifecycle_source,
            approval_handler: preflight_handler,
            check_references: is_file_input.then_some(allow),
            document,
            verbose: cli.verbose,
        });
    }

    let compose_start = perf.then(Instant::now);

    // Pre-flight approval lifecycle (v2 design step 4): condition-blind
    // collect every command, validate against blacklist/whitelist policy,
    // and route the remainder through a single batched prompt. The
    // resulting set becomes the execution membership source so the
    // pipeline never prompts per directive and never executes a command
    // before the full approval set is known. Skipped when shell expansion
    // is disabled (nothing to approve) and the per-`::shell` per-shell-block
    // stages never need to gate against an approval set.
    use darkmatter::markdown::compose::ComposeOperation;
    let options = compose_request.options();
    if options.is_enabled(ComposeOperation::ShellExpansion)
        || options.is_enabled(ComposeOperation::ShellBlocks)
        || options.is_enabled(ComposeOperation::FrontmatterShellExpansion)
    {
        match md.compose_preflight_approvals(&compose_request, preflight_handler.clone()) {
            Ok(approvals) => {
                if cli.verbose > 0 {
                    eprintln!(
                        "pre-flight: {} discovered, {} whitelisted, {} approved",
                        approvals.stats.total_discovered,
                        approvals.stats.already_whitelisted,
                        approvals.stats.user_approved
                    );
                }
                // Reuse the graph the preflight walk already resolved so the
                // transclusion stage skips a redundant target-resolution pass
                // (v2 design "reuse the collection walk").
                compose_request = compose_request.map_options(|options| {
                    options
                        .with_pre_approved_commands(approvals.pre_approved_commands)
                        .with_preflight_graph(approvals.preflight_graph)
                });
            }
            Err(e) => {
                return Err(preflight_approval_error(e));
            }
        }
    }

    let (composed, report) = md.compose_with(&compose_request).map_err(compose_error)?;
    let compose_pipeline_dur = compose_start.map(|s| s.elapsed()).unwrap_or_default();

    // Verbose compose summary
    if cli.verbose > 0 {
        use biscuit_terminal::components::renderable::TerminalRenderable;
        use biscuit_terminal::prelude::{Status, StatusState};
        let terminal = term_cell.get_or_init(Terminal::default);
        let status = Status::from_prose(format!(
            "Composed <b>{}</b> transclusions, <b>{}</b> interpolations, <b>{}</b> replacements",
            report.transclusions_applied,
            report.interpolations_applied,
            report.replacements_applied,
        ))
        .state(StatusState::Success);
        eprintln!("{}", status.render(terminal));
    }

    if cli.verbose > 1
        && let Some(perf_report) = &report.perf
    {
        use biscuit_terminal::components::renderable::TerminalRenderable;
        use biscuit_terminal::prelude::Status;
        let terminal = term_cell.get_or_init(Terminal::default);
        for metric in &perf_report.metrics {
            let status = Status::from_prose(format!(
                "<dim>{:20}</dim> {:>8.2}ms",
                metric.stage.to_string(),
                metric.elapsed.as_secs_f64() * 1000.0
            ));
            eprintln!("{}", status.render(terminal));
        }
    }

    document.emit(&composed)?;
    report_compose_diagnostics(&report, deferred_report.as_ref(), &term_cell);

    // Emit perf report to stderr (after all other diagnostics)
    if perf {
        let elapsed = cmd_start.map(|s| s.elapsed()).unwrap_or_default();
        let cli_perf = CliComposePerfReport {
            load_input: load_input_dur,
            resolve_input: resolve_input_dur,
            capture_context: capture_context_dur,
            capture_context_details: options_ctx_ref.capture_timings().to_vec(),
            validate_references: validate_refs_dur,
            build_options: build_options_dur,
            compose_pipeline: compose_pipeline_dur,
            elapsed,
        };
        let rendered = format_compose_perf_report(&cli_perf, report.perf.as_ref());
        eprint!("\n{rendered}");
    }

    // Drop the context reference (only held for perf timings)
    drop(options_ctx_ref);

    Ok(())
}

/// How `md compose` writes the composed document.
pub(crate) struct DocumentOutput<'a> {
    pub(crate) output: OutputFormat,
    pub(crate) show: bool,
    pub(crate) include_frontmatter: bool,
    pub(crate) cli: &'a Cli,
    pub(crate) input: Option<&'a PathBuf>,
}

impl DocumentOutput<'_> {
    /// The composed document in the requested format.
    pub(crate) fn artifact(&self, composed: &Markdown) -> Result<OutputArtifact> {
        match self.output {
            OutputFormat::Auto | OutputFormat::Markdown => {
                let content = if self.include_frontmatter {
                    composed.as_string()
                } else {
                    // Frontmatter drives the pipeline; once composition is complete, discard it.
                    composed.content().to_string()
                };
                Ok(OutputArtifact {
                    content: darkmatter::markdown::render_tree::embedded::without_embedded_metadata(&content),
                    extension: "md",
                    label: "markdown",
                })
            }
            // `markdown-plus` routes the composed document through the
            // MarkdownPlus fold so disclosure blocks emit `<details>` /
            // `<summary>` inline HTML rather than the DSL verbatim.
            OutputFormat::MarkdownPlus => markdown_plus_artifact(composed, self.cli, self.input),
            OutputFormat::Html => html_artifact(composed, self.cli, self.input),
            OutputFormat::Json => json_artifact(composed),
        }
    }

    /// Whether the document is printed: always for Markdown, and for the
    /// other formats only when it is not shown instead.
    pub(crate) fn prints(&self) -> bool {
        matches!(self.output, OutputFormat::Auto | OutputFormat::Markdown) || !self.show
    }

    fn emit(&self, composed: &Markdown) -> Result<()> {
        let artifact = self.artifact(composed)?;
        if self.prints() {
            println!("{}", artifact.content);
        }
        if self.show {
            open_output_artifact(&artifact)?;
        }
        Ok(())
    }
}

/// The outcome of reference validation before composition.
pub(crate) enum ReferenceCheck {
    Clean,
    /// Issues that never fail the run, reported after the content.
    Report(darkmatter::markdown::reference::validate::ReferenceValidationReport),
    /// An error the allow flags do not cover.
    Refused(darkmatter::markdown::reference::validate::ReferenceValidationReport),
}

/// Validates references with the same request composition will use, so
/// interpolation inside transclusion targets resolves as it will compose.
pub(crate) fn check_references(md: &Markdown, request: &ComposeRequest, allow: &ComposeAllowFlags) -> ReferenceCheck {
    use darkmatter::markdown::reference::ReferenceGraphOptions;
    use darkmatter::markdown::reference::validate::{
        ReferenceIssueCode, ReferenceSeverity, ReferenceValidationOptions,
    };

    let val_options = ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(request));
    let Ok(report) = md.validate_references(val_options) else {
        return ReferenceCheck::Clean;
    };
    let has_errors = report.issues.iter().any(|i| i.severity == ReferenceSeverity::Error);
    if has_errors {
        let has_unallowed = report
            .issues
            .iter()
            .any(|i| i.severity == ReferenceSeverity::Error && !allow.is_allowed(i.kind));
        if allow.is_strict() || has_unallowed {
            return ReferenceCheck::Refused(report);
        }
        // All errors are allowed — defer report for stderr after content
        return ReferenceCheck::Report(report);
    }
    // Warnings (such as a missing link inside inserted data) never fail the
    // run; report them after the content.
    if report
        .issues
        .iter()
        .any(|i| i.severity == ReferenceSeverity::Warning && i.code == ReferenceIssueCode::MissingLocalTarget)
    {
        return ReferenceCheck::Report(report);
    }
    ReferenceCheck::Clean
}

pub(crate) fn render_reference_report(
    report: &darkmatter::markdown::reference::validate::ReferenceValidationReport,
    term: &biscuit_terminal::terminal::Terminal,
) -> String {
    use biscuit_terminal::components::renderable::TerminalRenderable as _;
    use darkmatter::markdown::reference::validate::ValidationReportView;
    ValidationReportView::new(report.clone()).render(term)
}

/// Maps a composition failure to the report `md` prints.
pub(crate) fn compose_error(e: darkmatter::markdown::MarkdownError) -> color_eyre::eyre::Report {
    use darkmatter::markdown::MarkdownError::ShellExpansion;
    use darkmatter::markdown::compose::ShellExpansionError;

    match e {
        ShellExpansion(inner) => match *inner {
            ShellExpansionError::ExecutionFailed {
                command,
                code,
                stderr,
                origin,
                ..
            } => {
                let detail = stderr.trim();
                if detail.is_empty() {
                    eyre!("Shell command failed (exit {code}) at {origin}: '{command}'")
                } else {
                    eyre!(
                        "Shell command failed (exit {code}) at {origin}: '{command}'\n{detail}"
                    )
                }
            }
            ShellExpansionError::CommandNotFound { command, origin, .. } => {
                eyre!(
                    "Command not found: '{command}' ({origin})\n\
                     Ensure '{command}' is installed and available on your PATH."
                )
            }
            ShellExpansionError::Timeout {
                command,
                timeout,
                origin,
                ..
            } => {
                eyre!("Shell command timed out after {timeout:?} at {origin}: '{command}'")
            }
            ShellExpansionError::Blacklisted {
                command,
                reason,
                origin,
                ..
            } => {
                eyre!("Blocked command at {origin}: '{command}'\nReason: {reason}")
            }
            ShellExpansionError::Denied { command, origin, .. } => {
                eyre!("Command denied at {origin}: '{command}'")
            }
            ShellExpansionError::ApprovalRequired {
                command,
                whitelist_path,
                suggested_rules,
                ..
            } => approval_required_report(&command, &whitelist_path, &suggested_rules),
            other => eyre!("{other}"),
        },
        other => other.into(),
    }
}

/// The compose warnings and deferred validation issues, on stderr after the
/// content.
pub(crate) fn report_compose_diagnostics(
    report: &darkmatter::markdown::compose::ComposeReport,
    deferred_report: Option<&darkmatter::markdown::reference::validate::ReferenceValidationReport>,
    term_cell: &std::cell::OnceCell<biscuit_terminal::terminal::Terminal>,
) {
    use biscuit_terminal::terminal::Terminal;

    if !report.warnings.is_empty() {
        let term = term_cell.get_or_init(Terminal::default);
        for warning in &report.warnings {
            // A tolerated file-reference failure keeps its stable class row.
            let prose = match warning.resolution_failure {
                Some(failure) => format!(
                    "{}\n<dim>failure:</dim> {}",
                    warning.message,
                    darkmatter::markdown::errors::resolution_failure_name(failure)
                ),
                None => warning.message.clone(),
            };
            eprintln!("{}", warning_rows(&prose, term));
        }
    }

    // Emit deferred validation issues to stderr (allowed but still reported)
    if let Some(report) = deferred_report {
        let term = term_cell.get_or_init(Terminal::default);
        let formatted = render_reference_report(report, term);
        if !formatted.is_empty() {
            eprint!("\n{formatted}");
        }
        // The view lists errors only; a warning (a missing link inside text an
        // expression inserted) is reported line by line.
        use biscuit_terminal::components::prose::Prose;
        use darkmatter::markdown::reference::validate::{ReferenceIssueCode, ReferenceSeverity};
        for issue in report.issues.iter().filter(|i| {
            i.severity == ReferenceSeverity::Warning && i.code == ReferenceIssueCode::MissingLocalTarget
        }) {
            let markup = format!(
                "{} <dim>(line {}, inside inserted text)</dim>",
                Prose::escape_text_outside_code_spans(&issue.message),
                issue.origin.line,
            );
            eprintln!("{}", warning_rows(&markup, term));
        }
    }
}

/// A warning status for `markup`, whose further lines (a `hint:` or
/// `failure:` row) each start a row of their own.
fn warning_rows(markup: &str, term: &biscuit_terminal::terminal::Terminal) -> String {
    use biscuit_terminal::components::prose::LineBreaks;
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::prelude::{Status, StatusState};

    Status::from_prose(markup)
        .with_line_breaks(LineBreaks::Hard)
        .state(StatusState::Warning)
        .render(term)
}

fn apply_compose_baseline_schema(
    options: ComposeOptions,
    baseline_schema: Option<&PathBuf>,
    no_baseline_schema: bool,
    request: &MdRequest,
) -> Result<ComposeOptions> {
    if no_baseline_schema {
        return Ok(options);
    }

    if let Some(path) = baseline_schema {
        let resolved = resolve_file_path(path, request.launch_context()?)?;
        let raw = std::fs::read_to_string(&resolved)
            .wrap_err_with(|| format!("Failed to read baseline schema: {}", resolved.display()))?;
        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&raw).wrap_err_with(|| {
            format!(
                "Failed to parse baseline schema YAML: {}",
                resolved.display()
            )
        })?;
        let schema_value = yaml.get("$schema").unwrap_or(&yaml);
        let schema = darkmatter::markdown::schemas::parse_yaml_schema(schema_value)
            .wrap_err_with(|| format!("Invalid baseline schema: {}", resolved.display()))?;
        return Ok(options.with_baseline_schema(schema));
    }

    if env_disables_baseline_schema() {
        return Ok(options);
    }

    Ok(options.with_darkmatter_baseline_schema())
}

/// Environment variable whose truthy value switches the embedded baseline
/// schema off; see [`disables_baseline_schema`] for the accepted values.
pub(crate) const NO_BASELINE_SCHEMA_ENV: &str = "DARKMATTER_NO_BASELINE_SCHEMA";

/// Reads [`NO_BASELINE_SCHEMA_ENV`] from the live process environment.
///
/// Shared with `md clean`, whose schema resolution is at full compose parity.
/// `md schema validate` reads the variable from its captured request instead
/// and applies [`disables_baseline_schema`] to that value.
pub(crate) fn env_disables_baseline_schema() -> bool {
    disables_baseline_schema(std::env::var(NO_BASELINE_SCHEMA_ENV).ok().as_deref())
}

/// Whether a [`NO_BASELINE_SCHEMA_ENV`] value disables the baseline schema.
///
/// Exactly `1`, `true`, `TRUE`, `yes`, and `YES` disable it; every other
/// value, including other casings and padded spellings, and an absent
/// variable leave it on.
pub(crate) fn disables_baseline_schema(value: Option<&str>) -> bool {
    matches!(value, Some("1" | "true" | "TRUE" | "yes" | "YES"))
}

fn print_shell_command_report(
    commands: &[darkmatter::markdown::compose::shell_expansion::ShellCommandEntry],
) {
    if commands.is_empty() {
        println!("No shell commands discovered.");
        return;
    }

    println!("Shell commands discovered: {}", commands.len());
    println!();
    println!("| Command | Source | Origin |");
    println!("| --- | --- | --- |");

    for command in commands {
        println!(
            "| {} | {} | {} |",
            escape_table_cell(&command.normalized),
            escape_table_cell(&command.source_file.display().to_string()),
            escape_table_cell(&command.origin.to_string()),
        );
    }
}

/// Reports the ICMP probes the document graph could send.
///
/// Printed alongside the shell approval candidates because both answer the
/// same question: which effects would a compose run perform. `Granted` is
/// whether an existing `--allow-host` entry already permits the target; an
/// ungranted probe is the one that needs a new grant.
fn print_icmp_effect_report(probes: &[darkmatter::markdown::compose::PlannedIcmpProbe]) {
    println!();
    if probes.is_empty() {
        println!("No ICMP probes discovered.");
        return;
    }

    println!("ICMP probes discovered: {}", probes.len());
    println!();
    println!("| Function | Target | Timeout (ms) | Attempts | Granted |");
    println!("| --- | --- | --- | --- | --- |");

    for probe in probes {
        println!(
            "| {} | {} | {} | {} | {} |",
            escape_table_cell(&probe.function),
            escape_table_cell(&probe.target.to_string()),
            probe.timeout.as_millis(),
            probe.attempts,
            probe.granted,
        );
    }
}

fn escape_table_cell(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', " ")
}

/// The non-interactive "add one of these" report for a command that needs
/// approval. The rules are the ones its approval prompt would offer (exact
/// bytes, template shape, program prefixes), so each copies into the
/// whitelist as a rule that approves the command.
fn approval_required_report(
    command: &str,
    whitelist_path: &std::path::Path,
    rules: &darkmatter::markdown::compose::shell_expansion::template::SuggestedRules,
) -> color_eyre::eyre::Report {
    // Backslash-escape backslashes and underscores so Prose (used by `main`
    // to render error chains) neither consumes a key's `\\` escapes nor reads
    // `_..._` in paths or command names as italic markdown: a printed rule
    // must copy into the whitelist byte for byte.
    let escape_prose = |s: &str| s.replace('\\', "\\\\").replace('_', "\\_");
    let path = whitelist_path.display().to_string();
    if rules.is_empty() {
        return eyre!("Approval required for '{}'.", escape_prose(command));
    }
    eyre!(
        "Approval required for '{}'.\nTo allow in non-interactive mode, add one of these to {}:\n{}",
        escape_prose(command),
        escape_prose(&path),
        escape_prose(&rules.render()),
    )
}

/// Renders a pre-flight approval lifecycle error as a `color_eyre` report.
///
/// Mirrors the formatting the compose-error mapper applies to the
/// in-stage `ShellExpansionError` so the user sees the same shape
/// whether the failure was caught at pre-flight (this path) or
/// surfaced by `compose_with(...)` (the existing path).
pub(crate) fn preflight_approval_error(
    err: darkmatter::markdown::compose::ShellExpansionError,
) -> color_eyre::eyre::Report {
    use darkmatter::markdown::compose::ShellExpansionError;
    match err {
        // A rich error surfaced during pre-flight (transform parse, ctx merge,
        // schema validation, remote fetch, …). Convert the inner MarkdownError
        // straight to a report so `main`'s BlockError path renders its styled
        // block, identical to the `compose_with` failure path.
        ShellExpansionError::Preflight(inner) => (*inner).into(),
        ShellExpansionError::Blacklisted {
            command,
            reason,
            origin,
            ..
        } => eyre!("Blocked command at {origin}: '{command}'\nReason: {reason}"),
        ShellExpansionError::Denied { command, origin, .. } => {
            eyre!("Command denied at {origin}: '{command}'")
        }
        ShellExpansionError::ApprovalRequired {
            command,
            whitelist_path,
            suggested_rules,
            ..
        } => approval_required_report(&command, &whitelist_path, &suggested_rules),
        other => eyre!("{other}"),
    }
}

// ── Compose performance report ────────────────────────────────────────

/// CLI-envelope timings for the compose command.
struct CliComposePerfReport {
    load_input: std::time::Duration,
    resolve_input: std::time::Duration,
    capture_context: std::time::Duration,
    capture_context_details: Vec<(String, std::time::Duration)>,
    validate_references: std::time::Duration,
    build_options: std::time::Duration,
    compose_pipeline: std::time::Duration,
    elapsed: std::time::Duration,
}

/// Formats a duration as a human-readable string with appropriate units.
///
/// Uses the most readable unit for the magnitude:
/// - Under 1ms: microseconds (e.g., "42µs")
/// - Under 1s: milliseconds with decimals (e.g., "5.2ms")
/// - 1s and above: seconds with decimals (e.g., "2.60s")
fn format_duration(d: std::time::Duration) -> String {
    let micros = d.as_micros();
    if micros < 1_000 {
        format!("{}µs", micros)
    } else if micros < 1_000_000 {
        format!("{:.1}ms", micros as f64 / 1_000.0)
    } else {
        format!("{:.2}s", d.as_secs_f64())
    }
}

/// Formats a single metric line with optional dim/italic for skipped stages.
fn format_metric_line(name: &str, elapsed: std::time::Duration, calls: usize) -> String {
    if calls == 0 {
        format!("<dim><i>{:24}--</i></dim>", format!("{}:", name))
    } else {
        let calls_suffix = if calls > 1 {
            format!(" <dim>({} calls)</dim>", calls)
        } else {
            String::new()
        };
        format!(
            "{:24}{}{}",
            format!("{}:", name),
            format_duration(elapsed),
            calls_suffix,
        )
    }
}

/// Builds the full perf report string for the compose command.
fn format_compose_perf_report(
    cli_perf: &CliComposePerfReport,
    compose_perf: Option<&darkmatter::markdown::compose::ComposePerfReport>,
) -> String {
    use biscuit_terminal::components::block_quote::BlockQuote;
    use biscuit_terminal::components::prose::{LineBreaks, Prose};
    use biscuit_terminal::components::renderable::TerminalRenderable as _;
    use biscuit_terminal::components::two_column::TwoColumn;
    use biscuit_terminal::utils::color::{Color, Tailwind};

    // ── Left column: Command Setup ───────────────────────────────────
    let mut left = format!(
        "<b>Command Setup</b> <dim>(</dim><i>elapsed</i> {}<dim>)</dim>\n",
        format_duration(cli_perf.elapsed)
    );
    let cli_metrics: &[(&str, std::time::Duration)] = &[
        ("load input", cli_perf.load_input),
        ("resolve input", cli_perf.resolve_input),
        ("capture context", cli_perf.capture_context),
    ];
    for (name, dur) in cli_metrics {
        left.push_str(&format!(
            "  {:22}{}\n",
            format!("{}:", name),
            format_duration(*dur)
        ));
    }
    for (name, elapsed) in &cli_perf.capture_context_details {
        left.push_str(&format!(
            "    <dim>{:20}{}</dim>\n",
            format!("{}:", name),
            format_duration(*elapsed),
        ));
    }
    let remaining: &[(&str, std::time::Duration)] = &[
        ("validate references", cli_perf.validate_references),
        ("build options", cli_perf.build_options),
        ("compose pipeline", cli_perf.compose_pipeline),
    ];
    for (name, dur) in remaining {
        left.push_str(&format!(
            "  {:22}{}\n",
            format!("{}:", name),
            format_duration(*dur)
        ));
    }

    // ── Right column: Compose Pipeline ───────────────────────────────
    let right = if let Some(perf) = compose_perf {
        let mut col = format!(
            "<b>Compose Pipeline</b> <dim>(</dim><i>elapsed</i> {}<dim>)</dim>\n",
            format_duration(perf.total)
        );
        for metric in &perf.metrics {
            col.push_str(&format_metric_line(
                &metric.stage.to_string(),
                metric.elapsed,
                metric.calls,
            ));
            col.push('\n');
        }
        col
    } else {
        String::new()
    };

    // ── Title ────────────────────────────────────────────────────────
    let title = Prose::new("<b><yellow>Compose Performance</yellow></b>").render_optimistic(None);

    // ── Two-column layout ────────────────────────────────────────────
    let columns = TwoColumn::new(
        Prose::new(left.trim_end()).with_line_breaks(LineBreaks::Hard),
        Prose::new(right.trim_end()).with_line_breaks(LineBreaks::Hard),
    )
        .with_left_percent(0.5)
        .with_gap(2);

    let body = columns.render_optimistic(None);

    // ── Wrap in a styled block quote ─────────────────────────────────
    let content = format!("{title}\n{body}");
    let mut rendered = BlockQuote::from(content.trim_end())
        .with_left_block_color(Color::Tailwind(Tailwind::Yellow400))
        .with_border("▌ ")
        .render_optimistic(None);
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disables_baseline_schema_accepts_exactly_the_documented_spellings() {
        for value in ["1", "true", "TRUE", "yes", "YES"] {
            assert!(disables_baseline_schema(Some(value)), "{value:?} disables");
        }
        for value in ["", "0", "false", "no", "True", "Yes", " yes", "yes ", "on"] {
            assert!(!disables_baseline_schema(Some(value)), "{value:?} keeps the baseline");
        }
        assert!(!disables_baseline_schema(None), "an absent value keeps the baseline");
    }

    #[test]
    fn warning_rows_put_each_extra_line_on_a_row_of_its_own() {
        let term = biscuit_terminal::terminal::Terminal::builder()
            .width(200)
            .color_depth(biscuit_terminal::discovery::detection::ColorDepth::None)
            .build();
        let rendered = biscuit_terminal::utils::escape_codes::strip_escape_codes(warning_rows(
            "File not found: a.md\nhint: quote it\n<dim>failure:</dim> no-match",
            &term,
        ));
        let rows: Vec<&str> = rendered.lines().collect();
        assert_eq!(rows.len(), 3, "{rendered:?}");
        assert!(rows[0].ends_with("File not found: a.md"), "{rendered:?}");
        assert_eq!(rows[1], "hint: quote it");
        assert_eq!(rows[2], "failure: no-match");
    }

    #[test]
    fn parse_compose_setter_rejects_empty_key() {
        let parsed = parse_compose_setter("=value").expect("expected setter parse result");
        let err = parsed.expect_err("expected empty-key error");
        assert_eq!(err, "setter key must not be empty");
    }

    #[test]
    fn parse_compose_setter_rejects_numeric_leading_key_as_non_setter() {
        assert!(parse_compose_setter("9key=value").is_none());
    }

    #[test]
    fn parse_compose_setter_accepts_hyphenated_and_private_keys() {
        let hyphenated = parse_compose_setter("my-key=value")
            .expect("expected setter")
            .expect("expected valid setter");
        assert_eq!(hyphenated.0, "my-key");
        assert_eq!(hyphenated.1, serde_json::Value::String("value".to_string()));

        let private = parse_compose_setter("_private=true")
            .expect("expected setter")
            .expect("expected valid setter");
        assert_eq!(private.0, "_private");
        assert_eq!(private.1, serde_json::Value::Bool(true));
    }

    #[test]
    fn parse_compose_setter_treats_path_like_keys_as_input_candidates() {
        assert!(parse_compose_setter("path/key=value").is_none());
    }

    #[test]
    fn parse_shorthand_value_parses_primitives_and_falls_back_to_string() {
        assert_eq!(parse_shorthand_value("true"), serde_json::Value::Bool(true));
        assert_eq!(parse_shorthand_value("null"), serde_json::Value::Null);
        assert_eq!(
            parse_shorthand_value("hello"),
            serde_json::Value::String("hello".to_string())
        );
    }

    #[test]
    fn parse_shorthand_value_keeps_a_non_finite_number_as_text_not_null() {
        for raw in ["Infinity", "NaN", "1e400"] {
            assert_eq!(parse_shorthand_value(raw), serde_json::Value::String(raw.to_string()), "{raw}");
        }
    }

    #[test]
    fn parse_compose_positionals_empty_args_have_no_input_or_setters() {
        let parsed = parse_compose_positionals(&[]).expect("expected parse to succeed");
        assert!(parsed.input.is_none());
        assert!(parsed.shorthand_setters.is_empty());
    }

    #[test]
    fn parse_compose_positionals_setter_only_args_keep_input_empty() {
        let args = vec!["iteration=1".to_string(), "draft=false".to_string()];
        let parsed = parse_compose_positionals(&args).expect("expected parse to succeed");
        assert!(parsed.input.is_none());
        assert_eq!(
            parsed.shorthand_setters.get("iteration"),
            Some(&serde_json::json!(1))
        );
        assert_eq!(
            parsed.shorthand_setters.get("draft"),
            Some(&serde_json::json!(false))
        );
    }

    #[test]
    fn parse_compose_positionals_invalid_empty_key_surfaces_error() {
        let args = vec!["=value".to_string()];
        let err = parse_compose_positionals(&args).expect_err("expected parse to fail");
        assert!(err.to_string().contains("Invalid setter '=value'"));
    }

    #[test]
    fn parse_compose_positionals_treats_numeric_leading_key_as_input() {
        let args = vec!["9key=value".to_string()];
        let parsed = parse_compose_positionals(&args).expect("expected parse to succeed");
        assert_eq!(parsed.input, Some(PathBuf::from("9key=value")));
        assert!(parsed.shorthand_setters.is_empty());
    }

    #[test]
    fn format_duration_microseconds() {
        assert_eq!(format_duration(std::time::Duration::from_micros(0)), "0µs");
        assert_eq!(
            format_duration(std::time::Duration::from_micros(42)),
            "42µs"
        );
        assert_eq!(
            format_duration(std::time::Duration::from_micros(999)),
            "999µs"
        );
    }

    #[test]
    fn format_duration_milliseconds() {
        assert_eq!(
            format_duration(std::time::Duration::from_micros(1_000)),
            "1.0ms"
        );
        assert_eq!(
            format_duration(std::time::Duration::from_millis(42)),
            "42.0ms"
        );
        assert_eq!(
            format_duration(std::time::Duration::from_micros(5_200)),
            "5.2ms"
        );
    }

    #[test]
    fn format_duration_seconds() {
        assert_eq!(
            format_duration(std::time::Duration::from_millis(1000)),
            "1.00s"
        );
        assert_eq!(
            format_duration(std::time::Duration::from_millis(2600)),
            "2.60s"
        );
    }

    #[test]
    fn format_compose_perf_report_contains_sections() {
        use darkmatter::markdown::compose::{ComposePerfMetric, ComposePerfReport, ComposeStage};

        let cli_perf = CliComposePerfReport {
            load_input: std::time::Duration::from_millis(8),
            resolve_input: std::time::Duration::from_millis(1),
            capture_context: std::time::Duration::from_millis(50),
            capture_context_details: vec![
                ("git".to_string(), std::time::Duration::from_millis(20)),
                ("repo".to_string(), std::time::Duration::from_millis(15)),
            ],
            validate_references: std::time::Duration::from_millis(100),
            build_options: std::time::Duration::from_millis(2),
            compose_pipeline: std::time::Duration::from_millis(54),
            elapsed: std::time::Duration::from_millis(215),
        };

        let compose_perf = ComposePerfReport {
            total: std::time::Duration::from_millis(54),
            metrics: vec![
                ComposePerfMetric {
                    stage: ComposeStage::Cleanup,
                    elapsed: std::time::Duration::from_millis(3),
                    calls: 1,
                },
                ComposePerfMetric {
                    stage: ComposeStage::Normalization,
                    elapsed: std::time::Duration::from_millis(5),
                    calls: 1,
                },
            ],
            ..Default::default()
        };

        let rendered = format_compose_perf_report(&cli_perf, Some(&compose_perf));

        assert!(rendered.contains("Command Setup"));
        assert!(rendered.contains("Compose Pipeline"));
        assert!(rendered.contains("Compose Performance"));
        assert!(rendered.contains("load input:"));
        assert!(rendered.contains("cleanup:"));
        assert!(rendered.contains("normalization:"));
        // Verify the border is present
        assert!(rendered.contains("▌"));
    }

    #[test]
    fn format_compose_perf_report_without_pipeline() {
        let cli_perf = CliComposePerfReport {
            load_input: std::time::Duration::from_millis(1),
            resolve_input: std::time::Duration::ZERO,
            capture_context: std::time::Duration::ZERO,
            capture_context_details: Vec::new(),
            validate_references: std::time::Duration::ZERO,
            build_options: std::time::Duration::ZERO,
            compose_pipeline: std::time::Duration::ZERO,
            elapsed: std::time::Duration::from_millis(1),
        };

        let rendered = format_compose_perf_report(&cli_perf, None);

        assert!(rendered.contains("Command Setup"));
        assert!(!rendered.contains("Compose Pipeline"));
    }
}
