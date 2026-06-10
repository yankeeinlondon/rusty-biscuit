use crate::args::{
    Cli, Command as CliCommand, GraphFormat, OutputFormat, SchemaTarget, ValidateOutputFormat,
    ValidateTarget,
};
use crate::output::{
    OutputArtifact, emit_or_show_artifact, html_artifact, json_artifact, markdown_artifact,
    open_output_artifact, print_delta, print_toc_tree, render_terminal_output,
};
use biscuit_hash::xx_hash;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::cleanup::ListSpacingMode;
use darkmatter::markdown::compose::ComposeOptions;
use darkmatter::markdown::hash::{
    ComputedHash, DEFAULT_HASH_PROPERTY, LAST_UPDATED_KEY, MdHashKind, MdHashOptions, StoredHash,
    select_kind,
};
use darkmatter::markdown::highlighting::{
    ColorMode, ThemePair, detect_code_theme, detect_color_mode, detect_prose_theme,
};
use darkmatter::markdown::{Markdown, fs::collect_markdown_files};
use rayon::prelude::*;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;
use tracing::{debug, info, instrument};

pub mod schema;

/// Resolved theme configuration for terminal rendering.
struct ResolvedTheme {
    prose: ThemePair,
    code: ThemePair,
    color_mode: ColorMode,
}

impl ResolvedTheme {
    /// Resolves theme from CLI options, falling back to auto-detection.
    fn from_cli(cli: &Cli) -> Self {
        let prose = cli.theme.unwrap_or_else(detect_prose_theme);
        let code = cli.code_theme.unwrap_or_else(|| detect_code_theme(prose));
        let color_mode = detect_color_mode();
        Self {
            prose,
            code,
            color_mode,
        }
    }
}

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

pub fn validate_subcommand_usage(cli: &Cli) -> Result<()> {
    let mut conflicts = Vec::new();

    if cli.input.is_some() {
        conflicts.push("[INPUT]");
    }
    if cli.output != OutputFormat::Auto {
        conflicts.push("--output");
    }
    if cli.show {
        conflicts.push("--show");
    }
    if cli.save {
        conflicts.push("--save");
    }

    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(eyre!(
            "subcommands cannot be combined with top-level options: {}",
            conflicts.join(", ")
        ))
    }
}

// ── Compose positional classification ─────────────────────────────────

#[derive(Debug)]
struct ParsedComposeArgs {
    input: Option<PathBuf>,
    shorthand_setters: serde_json::Map<String, serde_json::Value>,
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

fn parse_compose_positionals(args: &[String]) -> Result<ParsedComposeArgs> {
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

pub fn run_subcommand(command: CliCommand, cli: &Cli) -> Result<()> {
    match command {
        CliCommand::Render {
            input,
            output,
            show,
            indent,
        } => {
            run_render(input.as_ref(), output, show, indent, cli)?;
        }
        CliCommand::Clean {
            input,
            save,
            indent,
            compact,
            loose,
        } => {
            let mode = resolve_list_spacing(compact, loose);
            run_clean(input.as_ref(), save, indent, mode, cli.verbose > 0)?;
        }
        CliCommand::Compose {
            args,
            state,
            set,
            output,
            show,
            frontmatter,
            compact,
            loose,
            indent,
            allow_missing_hyperlinks,
            allow_missing_image_refs,
            allow_missing_transclusions,
            allow_any_missing_reference,
            allow_ctx_override,
            allow_invalid_frontmatter_assignment,
            allow_reassigned_frontmatter_property,
            timeout,
            allow_shell_timeout,
            shell,
            perf,
            allow_host,
            remote_concurrency,
            remote_ttl,
            remote_refresh,
            remote_freshness,
            cache_root,
        } => {
            let parsed = parse_compose_positionals(&args)?;
            let mode = resolve_list_spacing(compact, loose);
            let allow = ComposeAllowFlags {
                hyperlinks: allow_missing_hyperlinks || allow_any_missing_reference,
                image_refs: allow_missing_image_refs || allow_any_missing_reference,
                transclusions: allow_missing_transclusions || allow_any_missing_reference,
            };
            let remote_config = build_remote_read_config(
                &allow_host,
                remote_concurrency,
                remote_ttl,
                remote_refresh,
                remote_freshness,
            );
            run_compose(
                parsed.input.as_ref(),
                state.as_deref(),
                set.as_deref(),
                parsed.shorthand_setters,
                output,
                show,
                frontmatter,
                mode,
                indent,
                &allow,
                allow_ctx_override,
                allow_invalid_frontmatter_assignment,
                allow_reassigned_frontmatter_property,
                timeout,
                allow_shell_timeout,
                shell,
                perf,
                remote_config,
                cache_root.as_ref(),
                cli,
            )?;
        }
        CliCommand::Toc { input, json } => {
            let md = load_markdown(input.as_ref())?;
            let toc = md.toc();

            if json {
                println!("{}", serde_json::to_string_pretty(&toc)?);
            } else {
                print_toc_tree(&toc, cli.verbose > 0, None);
            }
        }
        CliCommand::Delta {
            base,
            updated,
            json,
        } => {
            let base_md = load_markdown(Some(&base))
                .wrap_err_with(|| format!("Failed to read base file: {:?}", base))?;
            let updated_md = load_markdown(Some(&updated))
                .wrap_err_with(|| format!("Failed to read updated file: {:?}", updated))?;
            let delta = base_md.delta(&updated_md);

            if json {
                println!("{}", serde_json::to_string_pretty(&delta)?);
            } else {
                print_delta(&delta, cli.verbose > 0, &base_md, &updated_md);
            }
        }
        CliCommand::Get {
            input,
            props,
            json5,
            yaml,
            toml,
            raw,
            compact,
        } => {
            run_get(&input, &props, json5, yaml, toml, raw, compact)?;
        }
        CliCommand::Set {
            input,
            prop,
            value,
            save,
        } => {
            run_set(&input, &prop, &value, save)?;
        }
        CliCommand::Rm { input, props, json } => {
            run_rm(&input, &props, json, cli)?;
        }
        CliCommand::Edit { file } => {
            run_edit(&file)?;
        }
        CliCommand::Hash {
            input,
            kind,
            body,
            frontmatter,
            save,
            diff,
            strict,
        } => {
            run_hash(
                input.as_ref(),
                kind.map(Into::into),
                body,
                frontmatter,
                save,
                diff,
                strict,
            )?;
        }
        CliCommand::Validate { target } => {
            run_validate(target)?;
        }
        CliCommand::Graph {
            input,
            follow,
            validate,
            json,
        } => {
            run_graph(&input, follow, validate, json)?;
        }
        CliCommand::Schema { target } => match target {
            SchemaTarget::Validate {
                inputs,
                schema,
                format,
                quiet,
            } => {
                schema::run_validate(&inputs, schema.as_deref(), format, quiet)?;
            }
            SchemaTarget::Detect {
                files,
                format,
                merge,
            } => {
                schema::run_detect(&files, format, merge)?;
            }
        },
    }

    Ok(())
}

/// Clean markdown formatting, optionally saving in place and printing a delta report.
/// Converts CLI flags to a `ListSpacingMode`.
fn resolve_list_spacing(compact: bool, loose: bool) -> ListSpacingMode {
    match (compact, loose) {
        (true, _) => ListSpacingMode::Compact,
        (_, true) => ListSpacingMode::Loose,
        _ => ListSpacingMode::Normal,
    }
}

fn build_remote_read_config(
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

#[instrument(skip_all)]
pub fn run_clean(
    input: Option<&PathBuf>,
    save: bool,
    indent: Option<usize>,
    list_spacing: ListSpacingMode,
    verbose: bool,
) -> Result<()> {
    if !save {
        let mut md = load_markdown(input)?;
        apply_cleanup(&mut md, indent, list_spacing);
        println!("{}", md.as_string());
        return Ok(());
    }

    let input_path = input
        .ok_or_else(|| eyre!("--save requires an input file path (stdin is not supported)"))?;
    if input_path.to_str() == Some("-") {
        return Err(eyre!(
            "--save requires an input file path (stdin is not supported)"
        ));
    }

    let resolved = resolve_file_path(input_path)?;
    let original = load_markdown(Some(input_path))?;
    let mut cleaned = original.clone();
    apply_cleanup(&mut cleaned, indent, list_spacing);

    let delta = original.delta(&cleaned);
    if !delta.is_unchanged() {
        std::fs::write(&resolved, cleaned.as_string())
            .wrap_err_with(|| format!("Failed to write cleaned markdown to {:?}", resolved))?;
    }

    print_delta(&delta, verbose, &original, &cleaned);
    Ok(())
}

fn apply_cleanup(md: &mut Markdown, indent: Option<usize>, mode: ListSpacingMode) {
    match (indent, mode) {
        (Some(size), ListSpacingMode::Compact) => md.cleanup_with_indent_compact(size),
        (Some(size), ListSpacingMode::Loose) => md.cleanup_with_indent_loose(size),
        (Some(size), ListSpacingMode::Normal) => md.cleanup_with_indent(size),
        (None, ListSpacingMode::Compact) => md.cleanup_compact(),
        (None, ListSpacingMode::Loose) => md.cleanup_loose(),
        (None, ListSpacingMode::Normal) => md.cleanup(),
    };
}

/// Shared render logic for both implicit (no subcommand) and explicit `render` subcommand.
#[instrument(skip_all, fields(command = "render"))]
pub fn run_render(
    input: Option<&PathBuf>,
    output: OutputFormat,
    show: bool,
    indent: Option<usize>,
    cli: &Cli,
) -> Result<()> {
    debug!("rendering document");
    let mut md = load_markdown(input)?;

    // Apply cleanup with the specified or default indentation
    let indent_size = indent.unwrap_or(darkmatter::markdown::cleanup::DEFAULT_INDENT);
    md.cleanup_with_indent(indent_size);

    let theme = ResolvedTheme::from_cli(cli);
    let stdout_is_tty = io::stdout().is_terminal();

    match output {
        OutputFormat::Auto => {
            if stdout_is_tty {
                render_terminal_output(&md, input, cli, theme.prose, theme.code, theme.color_mode)?;
                if show {
                    open_output_artifact(&markdown_artifact(&md))?;
                }
            } else {
                emit_or_show_artifact(markdown_artifact(&md), show)?;
            }
        }
        OutputFormat::Markdown => {
            emit_or_show_artifact(markdown_artifact(&md), show)?;
        }
        OutputFormat::Html => {
            let artifact = html_artifact(&md, theme.prose, theme.code, theme.color_mode, cli, input)?;
            emit_or_show_artifact(artifact, show)?;
        }
        OutputFormat::Json => {
            let artifact = json_artifact(&md)?;
            emit_or_show_artifact(artifact, show)?;
        }
    }

    Ok(())
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
    timeout_secs: Option<u64>,
    allow_shell_timeout: bool,
    shell_report: bool,
    perf: bool,
    remote_config: darkmatter::markdown::compose::RemoteReadConfig,
    cache_root: Option<&PathBuf>,
    cli: &Cli,
) -> Result<()> {
    info!("starting compose pipeline");
    use std::time::Instant;

    let cmd_start = perf.then(Instant::now);

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
        use biscuit_terminal::terminal::Terminal;
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
                        let term = Terminal::default();
                        let formatted = format_validation_issues(&report, &term);
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

    // Build shell expansion options
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
        approval_handler: if is_file_input && crate::approval::can_prompt_interactively() {
            Some(Arc::new(crate::approval::CliShellApprovalHandler))
        } else {
            None
        },
        ..Default::default()
    };

    options = options.with_shell(shell_opts);
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
    let build_options_dur = opts_start.map(|s| s.elapsed()).unwrap_or_default();

    if shell_report {
        use darkmatter::markdown::compose::shell_expansion::collect_shell_commands;

        let commands = collect_shell_commands(&md, &options)?;
        print_shell_command_report(&commands);
        drop(options_ctx_ref);
        return Ok(());
    }

    let compose_start = perf.then(Instant::now);
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
        use biscuit_terminal::terminal::Terminal;
        let terminal = Terminal::default();
        let status = Status::from_prose(format!(
            "Composed <b>{}</b> transclusions, <b>{}</b> interpolations, <b>{}</b> replacements",
            report.transclusions_applied,
            report.interpolations_applied,
            report.replacements_applied,
        ))
        .state(StatusState::Success);
        eprintln!("{}", status.render(&terminal));
    }

    if cli.verbose > 1
        && let Some(perf_report) = &report.perf
    {
        use biscuit_terminal::components::renderable::TerminalRenderable;
        use biscuit_terminal::prelude::Status;
        use biscuit_terminal::terminal::Terminal;
        let terminal = Terminal::default();
        for metric in &perf_report.metrics {
            let status = Status::from_prose(format!(
                "<dim>{:20}</dim> {:>8.2}ms",
                metric.stage.to_string(),
                metric.elapsed.as_secs_f64() * 1000.0
            ));
            eprintln!("{}", status.render(&terminal));
        }
    }

    let theme = ResolvedTheme::from_cli(cli);

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
        OutputFormat::Html => {
            let artifact =
                html_artifact(&composed, theme.prose, theme.code, theme.color_mode, cli, input)?;
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
        use biscuit_terminal::terminal::Terminal;
        let term = Terminal::default();
        for warning in &report.warnings {
            let status = Status::from_prose(&warning.message).state(StatusState::Warning);
            eprintln!("{}", status.render(&term));
        }
    }

    // Emit deferred validation issues to stderr (allowed but still reported)
    if let Some(report) = deferred_report {
        use biscuit_terminal::terminal::Terminal;
        let term = Terminal::default();
        let formatted = format_validation_issues(&report, &term);
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

/// Get frontmatter properties from a markdown document.
#[instrument(skip_all)]
pub fn run_get(
    input: &PathBuf,
    props: &[String],
    json5: bool,
    yaml: bool,
    toml: bool,
    raw: bool,
    compact: bool,
) -> Result<()> {
    let md = load_markdown(Some(input))?;
    let fm = md.frontmatter();

    let value = if props.len() == 1 {
        // Single property: return the raw value (or empty string if missing)
        fm.as_map()
            .get(&props[0])
            .cloned()
            .unwrap_or(serde_json::Value::String(String::new()))
    } else {
        // Multiple properties: return a dictionary of requested keys
        let mut map = serde_json::Map::new();
        for prop in props {
            let v = fm
                .as_map()
                .get(prop)
                .cloned()
                .unwrap_or(serde_json::Value::String(String::new()));
            map.insert(prop.clone(), v);
        }
        serde_json::Value::Object(map)
    };

    let output = format_value(&value, json5, yaml, toml, raw, compact)?;
    println!("{output}");

    Ok(())
}

/// Set a frontmatter property on a markdown document.
///
/// By default the modified document is written to stdout without changing the
/// source file. With `--save` the file is updated in place and nothing is
/// printed.
#[instrument(skip_all)]
pub fn run_set(input: &PathBuf, prop: &str, raw_value: &str, save: bool) -> Result<()> {
    let is_stdin = input.to_str() == Some("-");

    if save && is_stdin {
        return Err(eyre!(
            "--save requires an input file path (stdin is not supported)"
        ));
    }

    let mut md = load_markdown(Some(input))?;

    let value: serde_json::Value = serde_json::from_str(raw_value)
        .unwrap_or_else(|_| serde_json::Value::String(raw_value.to_string()));

    md.fm_insert(prop, value)
        .map_err(|e| eyre!("Failed to set frontmatter property: {e}"))?;

    if save {
        let resolved = resolve_file_path(input)?;
        std::fs::write(&resolved, md.as_string())
            .wrap_err_with(|| format!("Failed to write to {:?}", resolved))?;
    } else {
        print!("{}", md.as_string());
    }

    Ok(())
}

/// Remove one or more frontmatter properties from a markdown document.
///
/// Saves the file in place. By default produces no output on success.
/// With `-v`, prints a human-readable summary. With `--json`, outputs
/// structured JSON.
#[instrument(skip_all)]
pub fn run_rm(input: &PathBuf, props: &[String], json: bool, cli: &Cli) -> Result<()> {
    let resolved = resolve_file_path(input)?;
    let mut md = load_markdown(Some(input))?;
    let fm = md.frontmatter_mut().as_map_mut();

    let mut removed = Vec::new();
    let mut not_found = Vec::new();

    for prop in props {
        if fm.shift_remove(prop).is_some() {
            removed.push(prop.clone());
        } else {
            not_found.push(prop.clone());
        }
    }

    if !not_found.is_empty() {
        let missing = not_found.join(", ");
        return Err(eyre!(
            "Property {} not found in frontmatter",
            if not_found.len() == 1 {
                format!("\"{}\"", missing)
            } else {
                format!(
                    "[{}]",
                    not_found
                        .iter()
                        .map(|p| format!("\"{p}\""))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        ));
    }

    std::fs::write(&resolved, md.as_string())
        .wrap_err_with(|| format!("Failed to write to {:?}", resolved))?;

    let remaining: Vec<String> = md.frontmatter().as_map().keys().cloned().collect();

    if json {
        let output = serde_json::json!({
            "removed": removed,
            "remaining": remaining,
            "filename": resolved.to_string_lossy(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if cli.verbose > 0 {
        use biscuit_terminal::components::prose::Prose;
        use biscuit_terminal::components::renderable::TerminalRenderable;
        use biscuit_terminal::terminal::Terminal;
        let props_label = if removed.len() == 1 {
            format!("<b>{}</b> property", removed[0])
        } else {
            format!(
                "<b>{}</b> properties",
                removed
                    .iter()
                    .map(|p| format!("\"{}\"", p))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let remaining_label = remaining.join(", ");
        let terminal = Terminal::default();
        eprintln!(
            "{}",
            Prose::new(format!(
                "- removed the {} from frontmatter (<dim>remaining: <i>{}</i></dim>)",
                props_label, remaining_label
            ))
            .render(&terminal)
        );
    }

    Ok(())
}

/// Format a `serde_json::Value` according to the requested output format.
fn format_value(
    value: &serde_json::Value,
    json5: bool,
    yaml: bool,
    toml: bool,
    raw: bool,
    compact: bool,
) -> Result<String> {
    if raw {
        return Ok(format_raw(value));
    }
    if compact {
        return Ok(serde_json::to_string(value)?);
    }
    if json5 {
        let json_str = serde_json::to_string(value)?;
        let j5 = biscuit_file::Json5::from_str(&json_str)
            .map_err(|e| eyre!("JSON5 conversion failed: {e}"))?;
        Ok(j5.as_json5())
    } else if yaml {
        let json_str = serde_json::to_string(value)?;
        let j5 = biscuit_file::Json5::from_str(&json_str)
            .map_err(|e| eyre!("YAML conversion failed: {e}"))?;
        let mut yaml_str = j5
            .as_yaml()
            .map_err(|e| eyre!("YAML conversion failed: {e}"))?;
        // serde_yaml appends a trailing newline; trim for consistent output
        if yaml_str.ends_with('\n') {
            yaml_str.truncate(yaml_str.len() - 1);
        }
        Ok(yaml_str)
    } else if toml {
        // TOML requires a table at the top level; wrap scalar values
        let toml_value = if value.is_object() {
            value.clone()
        } else {
            serde_json::json!({ "value": value })
        };
        let json_str = serde_json::to_string(&toml_value)?;
        let j5 = biscuit_file::Json5::from_str(&json_str)
            .map_err(|e| eyre!("TOML conversion failed: {e}"))?;
        let mut toml_str = j5
            .as_toml()
            .map_err(|e| eyre!("TOML conversion failed: {e}"))?;
        // Trim trailing newline for consistent output
        if toml_str.ends_with('\n') {
            toml_str.truncate(toml_str.len() - 1);
        }
        Ok(toml_str)
    } else {
        // Default: JSON
        Ok(serde_json::to_string_pretty(value)?)
    }
}

/// Format a JSON value as raw output (no JSON quoting).
///
/// - Strings: printed without quotes
/// - Null: empty string
/// - Booleans/numbers: their natural representation
/// - Arrays: one element per line (each element formatted raw)
/// - Objects: one `key: value` pair per line (values formatted raw)
fn format_raw(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => arr.iter().map(format_raw).collect::<Vec<_>>().join("\n"),
        serde_json::Value::Object(map) => map
            .iter()
            .map(|(k, v)| format!("{k}: {}", format_raw(v)))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Open a file in the user's preferred editor, blocking until the editor exits.
///
/// Resolves the file path using biscuit-file's `FileReference` system. Creates the
/// file if it doesn't exist. After the editor exits, validates that the file exists
/// and is non-empty (after trimming whitespace). Prints the fully qualified path on
/// success.
pub fn run_edit(raw_file: &str) -> Result<()> {
    use biscuit_file::FileReference;

    // --- Resolve the file path ---
    let path = match FileReference::new(raw_file) {
        Ok(file_ref) => {
            let resolved = file_ref
                .resolve()
                .wrap_err("Failed to resolve file reference")?;
            match resolved {
                Some(p) => p,
                None => {
                    // FileReference couldn't resolve it — treat raw input as a
                    // relative path (may not exist yet, which is fine).
                    std::env::current_dir()
                        .wrap_err("Failed to get current directory")?
                        .join(raw_file)
                }
            }
        }
        Err(_) => {
            // Not a valid file reference syntax — treat as plain path.
            let p = PathBuf::from(raw_file);
            if p.is_absolute() {
                p
            } else {
                std::env::current_dir()
                    .wrap_err("Failed to get current directory")?
                    .join(raw_file)
            }
        }
    };

    // Ensure parent directory exists
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent)
            .wrap_err_with(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // Create the file if it doesn't exist
    if !path.exists() {
        std::fs::write(&path, "")
            .wrap_err_with(|| format!("Failed to create file: {}", path.display()))?;
    }

    // --- Launch the editor ---
    let canonical = path
        .canonicalize()
        .wrap_err_with(|| format!("Failed to canonicalize path: {}", path.display()))?;
    darkmatter::editor::launch_editor_on_path(&canonical)?;

    // --- Validate the result ---
    if !canonical.exists() {
        return Err(eyre!(
            "File was deleted during editing: {}",
            canonical.display()
        ));
    }

    let content = std::fs::read_to_string(&canonical)
        .wrap_err_with(|| format!("Failed to read file after editing: {}", canonical.display()))?;

    if content.trim().is_empty() {
        return Err(eyre!(
            "File is empty after editing: {}",
            canonical.display()
        ));
    }

    // Output the fully qualified filename
    println!("{}", canonical.display());

    Ok(())
}

/// Hash a markdown document's frontmatter and/or body.
///
/// The CLI owns environment and flag parsing; the library performs all
/// computation from an explicit [`MdHashOptions`] bundle. Bare invocation prints
/// the computed hash and exits `0`. `--diff` reports differences and exits `2`
/// when any are found; `--save` writes the hash back and exits `0`. Operational
/// errors propagate on the eyre path (exit `1`).
///
/// When the input is a directory, recursively finds all markdown files, hashes
/// each in parallel, and produces a single aggregate hash. Directory mode is
/// bare-hash only: `--save`, `--diff`, and the `structured`/`detailed` kinds are
/// rejected with a usage error.
#[instrument(skip_all)]
pub fn run_hash(
    input: Option<&PathBuf>,
    kind: Option<MdHashKind>,
    body: bool,
    frontmatter: bool,
    save: bool,
    diff: bool,
    strict: bool,
) -> Result<()> {
    let options = resolve_hash_options(kind, body, frontmatter, strict);

    // Directory mode: aggregate hash over all markdown files.
    if let Some(path) = input
        && path.is_dir()
    {
        return run_hash_directory(path, &options, save, diff);
    }

    let md = load_markdown(input)?;
    let stored = parse_stored_hash(&md, &options)?;

    if diff {
        return run_hash_diff(&md, stored.as_ref(), &options);
    }
    if save {
        return run_hash_save(&md, input, stored.as_ref(), &options);
    }

    // Bare mode: select the kind (forced, else stored, else simple) and print.
    let selected = select_kind(stored.map(|s| s.kind), options.forced_kind);
    let computed = md.compute_hash(selected, &options);
    print_computed_hash(&computed)?;
    Ok(())
}

/// Resolves [`MdHashOptions`] from flags and the `HASH_PROPERTY` /
/// `HASH_IGNORE_PROPERTIES` environment variables.
///
/// `--kind` is the single forced-kind input; `--body` / `--frontmatter` are
/// shorthands the CLI maps to `Body` / `Fm`. `HASH_IGNORE_PROPERTIES` is additive
/// and can never un-ignore the active hash property or `last_updated`.
fn resolve_hash_options(
    kind: Option<MdHashKind>,
    body: bool,
    frontmatter: bool,
    strict: bool,
) -> MdHashOptions {
    let property = std::env::var("HASH_PROPERTY")
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|trimmed| !trimmed.is_empty())
        .unwrap_or_else(|| DEFAULT_HASH_PROPERTY.to_string());

    let extra_ignored = std::env::var("HASH_IGNORE_PROPERTIES")
        .ok()
        .map(|raw| parse_ignore_properties(&raw, &property))
        .unwrap_or_default();

    let forced_kind = match (kind, body, frontmatter) {
        (Some(k), _, _) => Some(k),
        (None, true, _) => Some(MdHashKind::Body),
        (None, _, true) => Some(MdHashKind::Fm),
        (None, false, false) => None,
    };

    MdHashOptions {
        property,
        extra_ignored,
        forced_kind,
        strict,
    }
}

/// Parses `HASH_IGNORE_PROPERTIES` as a CSV list of extra ignored properties:
/// trims each entry, drops empties, and excludes the always-managed `property`
/// and `last_updated` keys so they never leak into the stored `ignored` list.
fn parse_ignore_properties(raw: &str, property: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .filter(|entry| *entry != property && *entry != LAST_UPDATED_KEY)
        .map(String::from)
        .collect()
}

/// Reads and parses the document's stored hash property, or `None` when the
/// property is absent or null.
fn parse_stored_hash(md: &Markdown, options: &MdHashOptions) -> Result<Option<StoredHash>> {
    match md.frontmatter().as_map().get(options.property.as_str()) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(value) => Ok(Some(StoredHash::parse(value, &options.property)?)),
    }
}

/// Prints a computed hash: the flat string for `fm`/`body`/`simple`/`structured`,
/// or the nested YAML object for `detailed`.
fn print_computed_hash(computed: &ComputedHash) -> Result<()> {
    match computed.flat_string() {
        Some(flat) => println!("{flat}"),
        None => {
            let ComputedHash::Detailed(detailed) = computed else {
                unreachable!("only detailed hashes lack a flat string form");
            };
            let yaml = serde_yaml_ng::to_string(detailed)
                .wrap_err("Failed to serialize detailed hash")?;
            print!("{yaml}");
        }
    }
    Ok(())
}

/// `md hash --diff`: prints a kind-aware explanation and exits `2` when the
/// document differs from its stored hash (or has none).
fn run_hash_diff(
    md: &Markdown,
    stored: Option<&StoredHash>,
    options: &MdHashOptions,
) -> Result<()> {
    let Some(stored) = stored else {
        println!("No stored hash to compare against");
        std::process::exit(2);
    };

    let comparison = md.compare_hash(stored, options)?;
    let explanation = md.explain_hash_diff(stored, options)?;
    println!("{}", explanation.render());

    if comparison.frontmatter_changed || comparison.body_changed {
        std::process::exit(2);
    }
    Ok(())
}

/// `md hash --save`: writes the computed hash back into the document's
/// frontmatter and prints an explanation of what changed. Always exits `0` on
/// success, whether or not a write was needed.
fn run_hash_save(
    md: &Markdown,
    input: Option<&PathBuf>,
    stored: Option<&StoredHash>,
    options: &MdHashOptions,
) -> Result<()> {
    let input_path = input
        .filter(|p| p.to_str() != Some("-"))
        .ok_or_else(|| eyre!("--save requires an input file path (stdin is not supported)"))?;

    let decision = md.plan_hash_save(stored, options)?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    if let Some(written) = md.apply_hash_save(&decision, options, &today) {
        let resolved = resolve_file_path(input_path)?;
        std::fs::write(&resolved, written)
            .wrap_err_with(|| format!("Failed to write hash to {:?}", resolved))?;
    }

    match stored {
        Some(stored) => {
            let explanation = md.explain_hash_diff(stored, options)?;
            println!("{}", explanation.render());
        }
        None => {
            println!(
                "No stored hash found; wrote initial {} baseline",
                decision.kind
            );
        }
    }
    Ok(())
}

/// Directory-aggregate hashing. Bare-hash only: `--save`, `--diff`, and the
/// `structured`/`detailed` kinds are rejected with a usage error.
fn run_hash_directory(
    path: &std::path::Path,
    options: &MdHashOptions,
    save: bool,
    diff: bool,
) -> Result<()> {
    if save || diff {
        return Err(eyre!(
            "--save and --diff are not supported when hashing a directory"
        ));
    }
    if matches!(
        options.forced_kind,
        Some(MdHashKind::Structured | MdHashKind::Detailed)
    ) {
        return Err(eyre!(
            "--kind structured and --kind detailed are not supported when hashing a directory"
        ));
    }

    let body_only = options.forced_kind == Some(MdHashKind::Body);
    let frontmatter_only = options.forced_kind == Some(MdHashKind::Fm);

    // Directory mode is bare-hash only, so the kind is always one with a flat
    // string form. Computing through `compute_hash` (rather than the legacy
    // `Markdown::hash`) applies the managed-key and `HASH_IGNORE_PROPERTIES`
    // ignore policy, so an aggregate is stable when a file only gains `hash` /
    // `last_updated` and honors extra ignored properties.
    let kind = if body_only {
        MdHashKind::Body
    } else if frontmatter_only {
        MdHashKind::Fm
    } else {
        MdHashKind::Simple
    };

    let mut paths = collect_markdown_files(path)?;
    paths.sort();

    // Propagate per-file load/parse failures with path context rather than
    // hashing an empty document in their place. A single unreadable or
    // malformed file must fail the whole aggregate so `md hash <dir>` honors
    // the operational-error exit-code contract instead of producing a false
    // baseline.
    let per_file_hashes: Vec<String> = paths
        .par_iter()
        .map(|p| {
            let md = Markdown::try_from(p.as_path())
                .wrap_err_with(|| format!("Failed to read file: {}", p.display()))?;
            Ok(md
                .compute_hash(kind, options)
                .flat_string()
                .expect("simple/fm/body always have a flat string form"))
        })
        .collect::<Result<Vec<String>>>()?;

    let combined = per_file_hashes.join("\n");
    let aggregate = xx_hash(&combined);

    if body_only || frontmatter_only {
        println!("{:016x}", aggregate);
    } else {
        // Split each per-file "fm-body" hash, aggregate fm and body separately
        let (fm_parts, body_parts): (Vec<&str>, Vec<&str>) = per_file_hashes
            .iter()
            .filter_map(|h| h.split_once('-'))
            .unzip();

        let fm_aggregate = xx_hash(&fm_parts.join("\n"));
        let body_aggregate = xx_hash(&body_parts.join("\n"));
        println!("{:016x}-{:016x}", fm_aggregate, body_aggregate);
    }

    Ok(())
}

/// Loads markdown from a file path or stdin.
///
/// Paths are resolved through biscuit-file's `FileReference` system, which
/// supports `@`-prefixed magic paths (e.g. `@prompts/feature.md`), vault
/// references, and other reference syntaxes. Plain paths and `-` (stdin)
/// are handled as before.
pub fn load_markdown(path: Option<&PathBuf>) -> Result<Markdown> {
    if let Some(p) = path {
        if p.to_str() == Some("-") {
            // Explicit stdin marker
            read_from_stdin()
        } else {
            let resolved = resolve_file_path(p)?;
            Markdown::try_from(resolved.as_path())
                .wrap_err_with(|| format!("Failed to read file: {:?}", resolved))
        }
    } else {
        // No path provided - check if stdin has data
        if io::stdin().is_terminal() {
            // Interactive terminal - no input available
            Err(eyre!("No input file provided. Use `md --help` for usage."))
        } else {
            // Piped input available
            read_from_stdin()
        }
    }
}

/// Resolves a file path through biscuit-file's `FileReference` system.
///
/// If the path contains `@`-prefixed magic references or other FileReference
/// syntax, it will be resolved accordingly. Plain paths are returned as-is
/// (made absolute if relative).
fn resolve_file_path(raw_path: &PathBuf) -> Result<PathBuf> {
    use biscuit_file::FileReference;

    let raw = raw_path.to_string_lossy();
    match FileReference::new(&raw) {
        Ok(file_ref) => {
            let resolved = file_ref
                .resolve()
                .wrap_err_with(|| format!("Failed to resolve file reference: {:?}", raw_path))?;
            match resolved {
                Some(p) => Ok(p),
                None => {
                    // FileReference couldn't resolve it — fall back to raw path
                    Err(eyre!("Failed to load file: {:?}", raw_path))
                }
            }
        }
        Err(_) => {
            // Not a valid file reference syntax — treat as plain path
            Ok(raw_path.clone())
        }
    }
}

/// Reads markdown content from stdin.
fn read_from_stdin() -> Result<Markdown> {
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .wrap_err("Failed to read from stdin")?;
    Markdown::try_from_content(buffer).map_err(Into::into)
}

#[instrument(skip_all, fields(command = "validate"))]
fn run_validate(target: ValidateTarget) -> Result<()> {
    info!("starting reference validation");
    use darkmatter::markdown::reference::ReferenceGraphOptions;
    use darkmatter::markdown::reference::validate::ReferenceValidationOptions;

    match target {
        ValidateTarget::Refs {
            input,
            remote,
            fragments,
            timeout,
            fail_fast,
            format,
            show_all,
            graph,
        } => {
            let md = Markdown::try_from(input.as_path())
                .wrap_err_with(|| format!("Failed to load {}", input.display()))?;

            // If --graph requested, print graph and exit
            if let Some(graph_format) = graph {
                let graph_options = ReferenceGraphOptions::default();
                let ref_graph = md
                    .reference_graph(graph_options)
                    .wrap_err("Failed to build reference graph")?;

                match graph_format {
                    GraphFormat::Mermaid => println!("{}", ref_graph.to_mermaid()),
                    GraphFormat::Dot => println!("{}", ref_graph.to_dot()),
                }
                return Ok(());
            }

            let options = ReferenceValidationOptions {
                graph: ReferenceGraphOptions::default(),
                validate_remote: remote,
                remote_timeout: std::time::Duration::from_secs(timeout),
                validate_fragments: fragments,
                fail_fast,
            };

            let report = md
                .validate_references(options)
                .wrap_err("Reference validation failed")?;

            match format {
                ValidateOutputFormat::Text => {
                    print_validation_report_text(&report, &input, show_all);
                }
                ValidateOutputFormat::Json => {
                    print_validation_report_json(&report)?;
                }
            }

            if report.is_valid() {
                Ok(())
            } else {
                Err(eyre!("{} error(s) found", report.error_count()))
            }
        }
    }
}

fn print_validation_report_text(
    report: &darkmatter::markdown::reference::validate::ReferenceValidationReport,
    input: &std::path::Path,
    show_all: bool,
) {
    use darkmatter::markdown::reference::validate::ReferenceSeverity;

    println!("References scanned: {}", report.references_scanned);
    println!("Valid: {}", report.references_valid);
    println!("Issues: {}", report.issues.len());

    if !report.issues.is_empty() {
        println!();
    }

    for issue in &report.issues {
        let severity = match issue.severity {
            ReferenceSeverity::Error => "ERROR",
            ReferenceSeverity::Warning => "WARN ",
            ReferenceSeverity::Info => "INFO ",
        };

        let source = match &issue.origin.source {
            darkmatter::markdown::compose::ComposeSource::File(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| input.display().to_string()),
            _ => input.display().to_string(),
        };

        println!(
            "{severity}  {source}:{line}  {message}",
            line = issue.origin.line,
            message = issue.message
        );
    }

    if show_all && !report.warnings.is_empty() {
        println!();
        for warning in &report.warnings {
            println!("WARN   {warning}");
        }
    }
}

fn print_validation_report_json(
    report: &darkmatter::markdown::reference::validate::ReferenceValidationReport,
) -> Result<()> {
    use darkmatter::markdown::reference::validate::ReferenceSeverity;

    let issues: Vec<serde_json::Value> = report
        .issues
        .iter()
        .map(|i| {
            serde_json::json!({
                "code": format!("{:?}", i.code),
                "message": i.message,
                "severity": match i.severity {
                    ReferenceSeverity::Error => "error",
                    ReferenceSeverity::Warning => "warning",
                    ReferenceSeverity::Info => "info",
                },
                "reference_id": i.reference_id,
                "line": i.origin.line,
            })
        })
        .collect();

    let json = serde_json::json!({
        "references_scanned": report.references_scanned,
        "references_valid": report.references_valid,
        "issues": issues,
        "warnings": report.warnings,
        "is_valid": report.is_valid(),
    });

    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

/// Category label for a reference kind used in grouped validation output.
fn reference_kind_category_label(
    kind: darkmatter::markdown::reference::types::ReferenceKind,
) -> &'static str {
    use darkmatter::markdown::reference::types::ReferenceKind;
    match kind {
        ReferenceKind::Hyperlink => "Invalid Hyperlink(s)",
        ReferenceKind::Image => "Invalid Image Reference(s)",
        ReferenceKind::HtmlVideo | ReferenceKind::HtmlAudio | ReferenceKind::HtmlSource => {
            "Invalid Media Reference(s)"
        }
        ReferenceKind::HtmlIframe => "Invalid Iframe(s)",
        ReferenceKind::Transclusion => "Invalid Transclusion Target(s)",
        ReferenceKind::CssImport | ReferenceKind::InlineCss => "Invalid CSS Import(s)",
        ReferenceKind::ScriptImport | ReferenceKind::InlineScript => "Invalid Script Import(s)",
        ReferenceKind::FontImport => "Invalid Font Import(s)",
        ReferenceKind::MetaTag => "Invalid Meta Tag(s)",
    }
}

/// Formats grouped validation issues as a styled string.
///
/// Issues are grouped by [`ReferenceKind`] category, then listed by source
/// document with the broken reference highlighted in red. Returns an empty
/// string if there are no error-severity issues.
fn format_validation_issues(
    report: &darkmatter::markdown::reference::validate::ReferenceValidationReport,
    term: &biscuit_terminal::terminal::Terminal,
) -> String {
    use biscuit_terminal::components::list::UnorderedList;
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::TerminalRenderable as _;
    use darkmatter::markdown::compose::ComposeSource;
    use darkmatter::markdown::reference::types::ReferenceKind;
    use darkmatter::markdown::reference::validate::ReferenceSeverity;
    use std::collections::BTreeMap;

    // Only show error-severity issues in grouped output
    let errors: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.severity == ReferenceSeverity::Error)
        .collect();

    if errors.is_empty() {
        return String::new();
    }

    let mut out = String::new();

    // Group by kind, preserving discovery order within each group
    let mut groups: BTreeMap<u8, (ReferenceKind, Vec<_>)> = BTreeMap::new();
    let kind_order = |k: ReferenceKind| -> u8 {
        match k {
            ReferenceKind::Transclusion => 0,
            ReferenceKind::Hyperlink => 1,
            ReferenceKind::Image => 2,
            ReferenceKind::HtmlVideo | ReferenceKind::HtmlAudio | ReferenceKind::HtmlSource => 3,
            ReferenceKind::HtmlIframe => 4,
            ReferenceKind::CssImport | ReferenceKind::InlineCss => 5,
            ReferenceKind::ScriptImport | ReferenceKind::InlineScript => 6,
            ReferenceKind::FontImport => 7,
            ReferenceKind::MetaTag => 8,
        }
    };

    for issue in &errors {
        let key = kind_order(issue.kind);
        groups
            .entry(key)
            .or_insert_with(|| (issue.kind, Vec::new()))
            .1
            .push(*issue);
    }

    out.push('\n');

    for (kind, issues) in groups.values() {
        let label = reference_kind_category_label(*kind);
        let header = format!("<red-500><b>{label}</b></red-500>");
        out.push_str(&Prose::new(header).render(term));
        out.push('\n');

        let mut list = UnorderedList::empty();
        for issue in issues {
            let source_name = match &issue.origin.source {
                ComposeSource::File(p) => p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                ComposeSource::Url(u) => u.to_string(),
                ComposeSource::Unknown => "unknown".to_string(),
            };

            let source_href = match &issue.origin.source {
                ComposeSource::File(p) => {
                    format!("file://{}", p.display())
                }
                _ => String::new(),
            };

            let item_text = if source_href.is_empty() {
                format!(
                    "the <blue-500>{source_name}</blue-500> reference to <red-500>{ref_display}</red-500> is not valid",
                    ref_display = issue.reference_display,
                )
            } else {
                format!(
                    "the <a href=\"{source_href}\"><blue-500>{source_name}</blue-500></a> reference to <red-500>{ref_display}</red-500> is not valid",
                    ref_display = issue.reference_display,
                )
            };

            list.add(Prose::new(item_text));
        }

        out.push_str(&list.render(term));
        out.push('\n');
    }

    // Summary line
    let issue_count = errors.len();
    let summary = format!(
        "{} references scanned, {} valid, <red-500><b>{} issues</b></red-500>",
        report.references_scanned, report.references_valid, issue_count
    );
    out.push_str(&Prose::new(summary).render(term));
    out.push('\n');

    out
}

// ── Graph JSON serialization ──────────────────────────────────────────

/// Serialize a `ComposeSource` to a JSON-friendly path string.
fn source_to_json(source: &darkmatter::markdown::compose::ComposeSource) -> serde_json::Value {
    use darkmatter::markdown::compose::ComposeSource;
    match source {
        ComposeSource::File(p) => serde_json::Value::String(p.display().to_string()),
        ComposeSource::Url(u) => serde_json::Value::String(u.to_string()),
        ComposeSource::Unknown => serde_json::Value::Null,
    }
}

/// Serialize a `ReferenceKind` to a snake_case string.
fn kind_to_json(kind: darkmatter::markdown::reference::types::ReferenceKind) -> &'static str {
    use darkmatter::markdown::reference::types::ReferenceKind;
    match kind {
        ReferenceKind::Hyperlink => "hyperlink",
        ReferenceKind::Image => "image",
        ReferenceKind::HtmlVideo => "html_video",
        ReferenceKind::HtmlAudio => "html_audio",
        ReferenceKind::HtmlSource => "html_source",
        ReferenceKind::HtmlIframe => "html_iframe",
        ReferenceKind::Transclusion => "transclusion",
        ReferenceKind::CssImport => "css_import",
        ReferenceKind::InlineCss => "inline_css",
        ReferenceKind::ScriptImport => "script_import",
        ReferenceKind::InlineScript => "inline_script",
        ReferenceKind::FontImport => "font_import",
        ReferenceKind::MetaTag => "meta_tag",
    }
}

/// Serialize a `ReferenceTarget` to a JSON object with `type` and `raw`.
fn target_to_json(
    target: &darkmatter::markdown::reference::types::ReferenceTarget,
) -> serde_json::Value {
    use darkmatter::markdown::reference::types::ReferenceTarget;
    match target {
        ReferenceTarget::LocalPath { raw } => {
            serde_json::json!({ "type": "local_path", "raw": raw })
        }
        ReferenceTarget::RemoteUrl { raw } => {
            serde_json::json!({ "type": "remote_url", "raw": raw })
        }
        ReferenceTarget::Fragment { raw } => {
            serde_json::json!({ "type": "fragment", "raw": raw })
        }
        ReferenceTarget::DataUri { raw } => {
            serde_json::json!({ "type": "data_uri", "raw": raw })
        }
        ReferenceTarget::OtherScheme { raw, scheme } => {
            serde_json::json!({ "type": "other_scheme", "raw": raw, "scheme": scheme })
        }
        ReferenceTarget::Inline => {
            serde_json::json!({ "type": "inline" })
        }
    }
}

/// Serialize a `ReferenceSyntax` to a snake_case string.
fn syntax_to_json(syntax: darkmatter::markdown::reference::types::ReferenceSyntax) -> &'static str {
    use darkmatter::markdown::reference::types::ReferenceSyntax;
    match syntax {
        ReferenceSyntax::MarkdownLink => "markdown_link",
        ReferenceSyntax::HtmlAnchor => "html_anchor",
        ReferenceSyntax::MarkdownImage => "markdown_image",
        ReferenceSyntax::HtmlImage => "html_image",
        ReferenceSyntax::HtmlVideoTag => "html_video_tag",
        ReferenceSyntax::HtmlAudioTag => "html_audio_tag",
        ReferenceSyntax::HtmlSourceTag => "html_source_tag",
        ReferenceSyntax::HtmlIframeTag => "html_iframe_tag",
        ReferenceSyntax::DirectiveFile => "directive_file",
        ReferenceSyntax::DirectiveUrl => "directive_url",
        ReferenceSyntax::DirectiveCode => "directive_code",
        ReferenceSyntax::DirectiveTocLinking => "directive_toc_linking",
        ReferenceSyntax::DirectiveFileLinks => "directive_file_links",
        ReferenceSyntax::FrontmatterPrologue => "frontmatter_prologue",
        ReferenceSyntax::FrontmatterEpilogue => "frontmatter_epilogue",
        ReferenceSyntax::HtmlLinkTag => "html_link_tag",
        ReferenceSyntax::HtmlScriptTag => "html_script_tag",
        ReferenceSyntax::HtmlStyleTag => "html_style_tag",
        ReferenceSyntax::CssAtImport => "css_at_import",
        ReferenceSyntax::CssFontFaceSrc => "css_font_face_src",
        ReferenceSyntax::HtmlMetaTag => "html_meta_tag",
    }
}

/// Serialize a transclusion directive kind to a snake_case string.
fn directive_kind_to_json(
    syntax: darkmatter::markdown::reference::types::ReferenceSyntax,
) -> &'static str {
    use darkmatter::markdown::reference::types::ReferenceSyntax;
    match syntax {
        ReferenceSyntax::DirectiveFile => "file",
        ReferenceSyntax::DirectiveUrl => "url",
        ReferenceSyntax::DirectiveCode => "code",
        ReferenceSyntax::DirectiveTocLinking => "toc_linking",
        ReferenceSyntax::DirectiveFileLinks => "file_links",
        ReferenceSyntax::FrontmatterPrologue => "prologue",
        ReferenceSyntax::FrontmatterEpilogue => "epilogue",
        other => syntax_to_json(other),
    }
}

/// Serialize a single `ReferenceRecord` to JSON.
fn reference_record_to_json(
    record: &darkmatter::markdown::reference::types::ReferenceRecord,
) -> serde_json::Value {
    let mut obj = serde_json::json!({
        "id": record.id,
        "kind": kind_to_json(record.kind),
        "target": target_to_json(&record.target),
        "syntax": syntax_to_json(record.origin.syntax),
        "line": record.origin.line,
    });

    if !record.attributes.is_empty() {
        obj["attributes"] = serde_json::Value::Object(record.attributes.clone());
    }

    obj
}

/// Serialize a `ReferenceInsertion` (transclusion) to JSON, optionally expanding child nodes.
fn insertion_to_json(
    insertion: &darkmatter::markdown::reference::types::ReferenceInsertion,
    graph: &darkmatter::markdown::reference::types::ReferenceGraph,
    follow: bool,
) -> serde_json::Value {
    let kind_str = insertion
        .context
        .directive_kind
        .map(directive_kind_to_json)
        .unwrap_or("unknown");

    // Find the target path from the child node
    let child_node = graph.node_by_id(insertion.child_node_id.as_ref());
    let target = child_node
        .map(|n| source_to_json(&n.source))
        .unwrap_or(serde_json::Value::Null);

    let mut obj = serde_json::json!({
        "kind": kind_str,
        "target": target,
        "line": insertion.directive_line,
        "followable": insertion.context.directive_kind
            .map(|s| s.is_followable_transclusion())
            .unwrap_or(false),
    });

    if let Some(ref heading) = insertion.context.section_heading_text {
        obj["section"] = serde_json::Value::String(heading.clone());
    }
    if let Some(level) = insertion.context.section_heading_level {
        obj["section_level"] = serde_json::Value::Number(level.as_u8().into());
    }

    // Recursively expand child node when following
    if follow && let Some(child) = child_node {
        obj["node"] = graph_node_to_json(child, graph, true);
    }

    obj
}

/// Serialize a single graph node to JSON.
fn graph_node_to_json(
    node: &darkmatter::markdown::reference::types::ReferenceGraphNode,
    graph: &darkmatter::markdown::reference::types::ReferenceGraph,
    follow: bool,
) -> serde_json::Value {
    use darkmatter::markdown::compose::ComposeSource;

    let file_name = match &node.source {
        ComposeSource::File(p) => p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        ComposeSource::Url(u) => u.to_string(),
        ComposeSource::Unknown => String::new(),
    };

    let references: Vec<_> = node
        .local_references
        .records
        .iter()
        // Exclude transclusion records (they appear under "transclusions")
        .filter(|r| r.kind != darkmatter::markdown::reference::types::ReferenceKind::Transclusion)
        .map(reference_record_to_json)
        .collect();

    let transclusions: Vec<_> = node
        .child_insertions
        .iter()
        .map(|ins| insertion_to_json(ins, graph, follow))
        .collect();

    serde_json::json!({
        "file": file_name,
        "source": source_to_json(&node.source),
        "references": references,
        "transclusions": transclusions,
    })
}

/// Serialize validation report to JSON.
fn validation_report_to_json(
    report: &darkmatter::markdown::reference::validate::ReferenceValidationReport,
) -> serde_json::Value {
    use darkmatter::markdown::compose::ComposeSource;
    use darkmatter::markdown::reference::validate::ReferenceSeverity;

    let issues: Vec<_> = report
        .issues
        .iter()
        .map(|i| {
            let source_file = match &i.origin.source {
                ComposeSource::File(p) => serde_json::Value::String(p.display().to_string()),
                ComposeSource::Url(u) => serde_json::Value::String(u.to_string()),
                ComposeSource::Unknown => serde_json::Value::Null,
            };
            serde_json::json!({
                "code": format!("{:?}", i.code),
                "message": i.message,
                "severity": match i.severity {
                    ReferenceSeverity::Error => "error",
                    ReferenceSeverity::Warning => "warning",
                    ReferenceSeverity::Info => "info",
                },
                "kind": kind_to_json(i.kind),
                "reference": i.reference_display,
                "line": i.origin.line,
                "source": source_file,
            })
        })
        .collect();

    serde_json::json!({
        "valid": report.is_valid(),
        "references_scanned": report.references_scanned,
        "references_valid": report.references_valid,
        "issues": issues,
        "warnings": report.warnings,
    })
}

#[instrument(skip_all)]
fn run_graph(input: &PathBuf, follow: bool, validate: bool, json: bool) -> Result<()> {
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::terminal::Terminal;
    use darkmatter::markdown::reference::file_tree::FileTree;

    let resolved = resolve_file_path(input)?;
    let mut tree = FileTree::new(&resolved).map_err(|e| eyre!("{e}"))?;

    if follow {
        tree = tree.follow_transclusions();
    }
    if validate {
        tree = tree.validate();
    }

    tree.ensure_built().map_err(|e| eyre!("{e}"))?;

    // ── JSON output ──────────────────────────────────────────────────
    if json {
        if let Some(graph) = tree.graph() {
            let mut root_json = graph_node_to_json(&graph.root, graph, follow);

            if validate && let Some(report) = tree.validation_report() {
                root_json["validation"] = validation_report_to_json(report);
            }

            println!("{}", serde_json::to_string_pretty(&root_json)?);
        }
        // JSON mode always exits 0
        return Ok(());
    }

    // ── Terminal tree output ─────────────────────────────────────────
    let term = Terminal::default();
    print!("{}", tree.display(&term));

    // Validation summary footer
    if validate && let Some(report) = tree.validation_report() {
        use biscuit_terminal::components::prose::Prose;
        use biscuit_terminal::components::renderable::TerminalRenderable as _;

        let formatted = format_validation_issues(report, &term);
        if !formatted.is_empty() {
            println!("{formatted}");
        } else {
            let summary = format!(
                "{} references scanned, {} valid, 0 issues",
                report.references_scanned, report.references_valid,
            );
            println!("\n{}", Prose::new(summary).render(&term));
        }

        // Exit code 2 for validation errors
        if !report.is_valid() {
            std::process::exit(2);
        }
    }

    Ok(())
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
