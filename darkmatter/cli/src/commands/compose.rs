//! `md compose` subcommand implementation: positional argument parsing, the
//! compose pipeline runner, the shell-command report, and the compose
//! performance report.

use crate::args::{Cli, OutputFormat};
use crate::artifact::{
    OutputArtifact, emit_or_show_artifact, html_artifact, json_artifact, markdown_plus_artifact,
    open_output_artifact,
};
use crate::io::{load_markdown, resolve_file_path};
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::cleanup::ListSpacingMode;
use darkmatter::markdown::compose::ComposeOptions;
use std::path::PathBuf;
use tracing::{info, instrument};

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
        Ok(parsed) => parsed.value().clone(),
        Err(_) => serde_json::Value::String(raw.to_string()),
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
    list_spacing: ListSpacingMode,
    indent: Option<usize>,
    allow: &ComposeAllowFlags,
    allow_ctx_override: bool,
    allow_invalid_frontmatter_assignment: bool,
    allow_reassigned_frontmatter_property: bool,
    baseline_schema: Option<&PathBuf>,
    no_baseline_schema: bool,
    no_trigger_schemas: bool,
    timeout_secs: Option<u64>,
    allow_shell_timeout: bool,
    shell_report: bool,
    perf: bool,
    remote_config: darkmatter::markdown::compose::RemoteReadConfig,
    cache_root: Option<&PathBuf>,
    cli: &Cli,
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

    // Resolve the input path once through FileReference (handles @-prefixed paths)
    // and reuse for both loading and source_file/policy_root.
    let resolve_start = perf.then(Instant::now);
    let resolved_input = if let Some(path) = input
        && path.to_str() != Some("-")
    {
        Some(resolve_file_path(path)?)
    } else {
        None
    };
    let resolve_input_dur = resolve_start.map(|s| s.elapsed()).unwrap_or_default();

    let load_start = perf.then(Instant::now);
    let md = if let Some(ref resolved) = resolved_input {
        Markdown::try_from(resolved.as_path())
            .wrap_err_with(|| format!("Failed to load file: {:?}", resolved))?
    } else {
        load_markdown(None)?
    };
    let load_input_dur = load_start.map(|s| s.elapsed()).unwrap_or_default();

    // Demand-driven context capture: scan document (body + frontmatter values)
    // for ctx.* references and only capture the groups actually needed.
    // Shared between validation and compose.
    let ctx_start = perf.then(Instant::now);
    let shared_context = {
        let base_dir = std::env::current_dir().unwrap_or_default();
        darkmatter::markdown::compose::ComposeContext::capture_for_document(&base_dir, &md)
    };
    let capture_context_dur = ctx_start.map(|s| s.elapsed()).unwrap_or_default();
    // Keep a cheap Arc clone for perf report context timings
    let options_ctx_ref = shared_context.clone();

    // ── Parse --state and --set early ────────────────────────────────────
    // These must be available before reference validation so that
    // interpolation inside transclusion targets (e.g., `::file @{{ctx.pkg}}/{{plan}}`)
    // can resolve user-provided variables during the validation pass.
    let opts_start = perf.then(Instant::now);
    let mut options = ComposeOptions::new_with_context(shared_context);
    options = apply_compose_baseline_schema(options, baseline_schema, no_baseline_schema)?;
    options = options.with_trigger_schemas(!no_trigger_schemas);

    // Parse --state as JSON or JSON5
    if let Some(json_str) = state_json {
        let parsed = biscuit_file::Json5::from_str(json_str)
            .wrap_err("Invalid JSON/JSON5 in --state argument")?;
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
        options = options.with_set_overrides(serde_json::Value::Object(override_map));
    }

    // ── Reference validation ───────────────────────────────────────────
    // Validate before composing so broken references are caught early.
    // Uses the same options (including --state/--set) so interpolation
    // inside transclusion targets resolves correctly.
    let val_start = perf.then(Instant::now);
    let deferred_report = if resolved_input.is_some() {
        use darkmatter::markdown::reference::ReferenceGraphOptions;
        use darkmatter::markdown::reference::validate::{
            ReferenceSeverity, ReferenceValidationOptions,
        };

        let val_options = ReferenceValidationOptions::with_graph(
            ReferenceGraphOptions::with_compose(options.clone()),
        );

        match md.validate_references(val_options) {
            Ok(report) => {
                let has_errors = report
                    .issues
                    .iter()
                    .any(|i| i.severity == ReferenceSeverity::Error);

                if has_errors {
                    let has_unallowed = report.issues.iter().any(|i| {
                        i.severity == ReferenceSeverity::Error && !allow.is_allowed(i.kind)
                    });

                    if allow.is_strict() || has_unallowed {
                        // Strict mode or unallowed errors: show issues and exit
                        use biscuit_terminal::components::renderable::TerminalRenderable as _;
                        use darkmatter::markdown::reference::validate::ValidationReportView;
                        let term = term_cell.get_or_init(Terminal::default);
                        let formatted = ValidationReportView::new(report.clone()).render(term);
                        eprint!("{formatted}");
                        std::process::exit(2);
                    }

                    // All errors are allowed — defer report for stderr after content
                    Some(report)
                } else {
                    None
                }
            }
            Err(_) => None,
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
        policy_root: resolved_input.as_ref().and_then(|p| {
            p.parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .map(|parent| parent.to_path_buf())
        }),
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
    if let Some(size) = indent {
        options = options.with_indent_size(size);
    }
    // Share one single-flight remote-fetch runtime between the pre-flight
    // approval walk and the compose pass so each remote URL is fetched once,
    // not once per stage. Must follow the remote-read-config and cache-root
    // wiring above so the shared runtime inherits both.
    options = options.with_shared_remote_fetch();
    let build_options_dur = opts_start.map(|s| s.elapsed()).unwrap_or_default();

    if shell_report {
        // `--shell` reports condition-blind approval candidates: every command
        // that *could* run under any document state, routed through the same
        // pre-flight collector that authorization uses.
        let preflight = md.compose_preflight(&options)?;
        print_shell_command_report(&preflight.entries);
        drop(options_ctx_ref);
        return Ok(());
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
    if options.is_enabled(ComposeOperation::ShellExpansion)
        || options.is_enabled(ComposeOperation::ShellBlocks)
        || options.is_enabled(ComposeOperation::FrontmatterShellExpansion)
    {
        match md.compose_preflight_approvals(&options, preflight_handler.clone()) {
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
                options = options
                    .with_pre_approved_commands(approvals.pre_approved_commands)
                    .with_preflight_graph(approvals.preflight_graph);
            }
            Err(e) => {
                return Err(preflight_approval_error(e));
            }
        }
    }

    let (composed, report) = md.compose_with(options).map_err(|e| {
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
                    origin: _,
                    ..
                } => {
                    let executable = command.split_whitespace().next().unwrap_or(&command);
                    // Backslash-escape underscores so Prose (used by `main` to
                    // render error chains) does not mistake `_..._` patterns in
                    // paths or command names for italic markdown.
                    let escape_prose = |s: &str| s.replace('_', "\\_");
                    let path = whitelist_path.display().to_string();
                    eyre!(
                        "Approval required for '{}'.\nTo allow in non-interactive mode, add one of these to {}:\n  exact {}\n  prefix {}",
                        escape_prose(&command),
                        escape_prose(&path),
                        escape_prose(&command),
                        escape_prose(executable),
                    )
                }
                other => eyre!("{other}"),
            },
            other => other.into(),
        }
    })?;
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

    match output {
        OutputFormat::Auto | OutputFormat::Markdown => {
            let content = if include_frontmatter {
                composed.as_string()
            } else {
                // Frontmatter drives the pipeline; once composition is complete, discard it.
                composed.content().to_string()
            };
            if show {
                let artifact = OutputArtifact {
                    content: content.clone(),
                    extension: "md",
                    label: "markdown",
                };
                println!("{content}");
                open_output_artifact(&artifact)?;
            } else {
                println!("{content}");
            }
        }
        OutputFormat::MarkdownPlus => {
            // `markdown-plus` routes the composed document through the
            // MarkdownPlus fold so disclosure blocks emit `<details>` /
            // `<summary>` inline HTML rather than the DSL verbatim.
            let artifact = markdown_plus_artifact(
                &composed,
                cli,
                input,
            )?;
            emit_or_show_artifact(artifact, show)?;
        }
        OutputFormat::Html => {
            let artifact =
                html_artifact(&composed, cli, input)?;
            emit_or_show_artifact(artifact, show)?;
        }
        OutputFormat::Json => {
            let artifact = json_artifact(&composed)?;
            emit_or_show_artifact(artifact, show)?;
        }
    }

    // Emit compose warnings to stderr
    if !report.warnings.is_empty() {
        use biscuit_terminal::components::renderable::TerminalRenderable;
        use biscuit_terminal::prelude::{Status, StatusState};
        let term = term_cell.get_or_init(Terminal::default);
        for warning in &report.warnings {
            let status = Status::from_prose(&warning.message).state(StatusState::Warning);
            eprintln!("{}", status.render(term));
        }
    }

    // Emit deferred validation issues to stderr (allowed but still reported)
    if let Some(report) = deferred_report {
        use biscuit_terminal::components::renderable::TerminalRenderable as _;
        use darkmatter::markdown::reference::validate::ValidationReportView;
        let term = term_cell.get_or_init(Terminal::default);
        let formatted = ValidationReportView::new(report).render(term);
        if !formatted.is_empty() {
            eprint!("\n{formatted}");
        }
    }

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

fn apply_compose_baseline_schema(
    options: ComposeOptions,
    baseline_schema: Option<&PathBuf>,
    no_baseline_schema: bool,
) -> Result<ComposeOptions> {
    if no_baseline_schema {
        return Ok(options);
    }

    if let Some(path) = baseline_schema {
        let resolved = resolve_file_path(path)?;
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

fn env_disables_baseline_schema() -> bool {
    matches!(
        std::env::var("DARKMATTER_NO_BASELINE_SCHEMA")
            .ok()
            .as_deref(),
        Some("1" | "true" | "TRUE" | "yes" | "YES")
    )
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

fn escape_table_cell(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', " ")
}

/// Renders a pre-flight approval lifecycle error as a `color_eyre` report.
///
/// Mirrors the formatting the compose-error mapper applies to the
/// in-stage `ShellExpansionError` so the user sees the same shape
/// whether the failure was caught at pre-flight (this path) or
/// surfaced by `compose_with(...)` (the existing path).
fn preflight_approval_error(
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
            origin: _,
            ..
        } => {
            let executable = command.split_whitespace().next().unwrap_or(&command);
            let escape_prose = |s: &str| s.replace('_', "\\_");
            let path = whitelist_path.display().to_string();
            eyre!(
                "Approval required for '{}'.\nTo allow in non-interactive mode, add one of these to {}:\n  exact {}\n  prefix {}",
                escape_prose(&command),
                escape_prose(&path),
                escape_prose(&command),
                escape_prose(executable),
            )
        }
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
    use biscuit_terminal::components::prose::Prose;
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
    let columns = TwoColumn::new(Prose::new(left.trim_end()), Prose::new(right.trim_end()))
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
