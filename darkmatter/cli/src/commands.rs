use crate::args::{
    Cli, Command as CliCommand, GraphFormat, OutputFormat, ValidateOutputFormat, ValidateTarget,
};
use crate::output::{
    OutputArtifact, emit_or_show_artifact, html_artifact, json_artifact, markdown_artifact,
    open_output_artifact, print_delta, print_toc_tree, render_terminal_output,
};
use biscuit_hash::xx_hash;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::cleanup::ListSpacingMode;
use darkmatter::markdown::compose::ComposeOptions;
use darkmatter::markdown::highlighting::{
    detect_code_theme, detect_color_mode, detect_prose_theme,
};
use darkmatter::markdown::{Markdown, fs::collect_markdown_files};
use rayon::prelude::*;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;
use std::process::Command;
use tracing::{debug, info, instrument};

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
            input,
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
            perf,
        } => {
            let mode = resolve_list_spacing(compact, loose);
            let allow = ComposeAllowFlags {
                hyperlinks: allow_missing_hyperlinks || allow_any_missing_reference,
                image_refs: allow_missing_image_refs || allow_any_missing_reference,
                transclusions: allow_missing_transclusions || allow_any_missing_reference,
            };
            run_compose(
                input.as_ref(),
                state.as_deref(),
                set.as_deref(),
                output,
                show,
                frontmatter,
                mode,
                indent,
                &allow,
                allow_ctx_override,
                perf,
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
            body,
            frontmatter,
            strict,
        } => {
            run_hash(input.as_ref(), body, frontmatter, strict)?;
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

    let prose_theme = cli.theme.unwrap_or_else(detect_prose_theme);
    let code_theme = cli
        .code_theme
        .unwrap_or_else(|| detect_code_theme(prose_theme));
    let color_mode = detect_color_mode();
    let stdout_is_tty = io::stdout().is_terminal();

    match output {
        OutputFormat::Auto => {
            if stdout_is_tty {
                render_terminal_output(&md, input, cli, prose_theme, code_theme, color_mode)?;
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
            let artifact = html_artifact(&md, prose_theme, code_theme, color_mode)?;
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
    output: OutputFormat,
    show: bool,
    include_frontmatter: bool,
    list_spacing: ListSpacingMode,
    indent: Option<usize>,
    allow: &ComposeAllowFlags,
    allow_ctx_override: bool,
    perf: bool,
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
    if let Some(json_str) = set_json {
        let parsed = biscuit_file::Json5::from_str(json_str)
            .wrap_err("Invalid JSON/JSON5 in --set argument")?;
        let set = parsed.value().clone();
        if !set.is_object() {
            return Err(eyre!(
                "Invalid --set argument: expected a JSON object like {{\"name\":\"Alice\"}}"
            ));
        }
        options = options.with_set_overrides(set);
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
    options = options.with_list_spacing(list_spacing);
    options = options.with_allow_ctx_override(allow_ctx_override);
    options = options.with_perf(perf);
    if let Some(size) = indent {
        options = options.with_indent_size(size);
    }
    let build_options_dur = opts_start.map(|s| s.elapsed()).unwrap_or_default();

    let compose_start = perf.then(Instant::now);
    let (composed, report) = md.compose_with(options).map_err(|e| {
        use darkmatter::markdown::MarkdownError::ShellExpansion;
        use darkmatter::markdown::compose::ShellExpansionError;

        match &e {
            ShellExpansion(ShellExpansionError::ApprovalRequired {
                command,
                whitelist_path,
                ..
            }) => {
                let executable = command.split_whitespace().next().unwrap_or(command);
                eyre!(
                    "Approval required for '{command}'.\n\
                     To allow in non-interactive mode, add one of these to {}:\n  \
                     exact {command}\n  \
                     prefix {executable}",
                    whitelist_path.display(),
                )
            }
            ShellExpansion(ShellExpansionError::ExecutionFailed {
                command,
                code,
                stderr,
                line,
                ..
            }) => {
                let detail = stderr.trim();
                if detail.is_empty() {
                    eyre!("Shell command failed (exit {code}) on line {line}: '{command}'")
                } else {
                    eyre!(
                        "Shell command failed (exit {code}) on line {line}: '{command}'\n{detail}"
                    )
                }
            }
            ShellExpansion(ShellExpansionError::CommandNotFound { command, line }) => {
                eyre!(
                    "Command not found: '{command}' (line {line})\n\
                     Ensure '{command}' is installed and available on your PATH."
                )
            }
            ShellExpansion(ShellExpansionError::Timeout {
                command,
                timeout,
                line,
            }) => {
                eyre!("Shell command timed out after {timeout:?} on line {line}: '{command}'")
            }
            ShellExpansion(ShellExpansionError::Blacklisted {
                command,
                reason,
                line,
            }) => {
                eyre!("Blocked command on line {line}: '{command}'\nReason: {reason}")
            }
            ShellExpansion(ShellExpansionError::Denied { command, line }) => {
                eyre!("Command denied on line {line}: '{command}'")
            }
            _ => eyre!("{e}"),
        }
    })?;
    let compose_pipeline_dur = compose_start.map(|s| s.elapsed()).unwrap_or_default();

    // Verbose compose summary
    if cli.verbose > 0 {
        use biscuit_terminal::components::renderable::Renderable;
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
        use biscuit_terminal::components::renderable::Renderable;
        use biscuit_terminal::prelude::Status;
        use biscuit_terminal::terminal::Terminal;
        let terminal = Terminal::default();
        for metric in &perf_report.metrics {
            let status = Status::from_prose(format!(
                "<dim>{:20}</dim> {:>8.2}ms",
                metric.name,
                metric.elapsed.as_secs_f64() * 1000.0
            ));
            eprintln!("{}", status.render(&terminal));
        }
    }

    let prose_theme = cli.theme.unwrap_or_else(detect_prose_theme);
    let code_theme = cli
        .code_theme
        .unwrap_or_else(|| detect_code_theme(prose_theme));
    let color_mode = detect_color_mode();

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
            let artifact = html_artifact(&composed, prose_theme, code_theme, color_mode)?;
            emit_or_show_artifact(artifact, show)?;
        }
        OutputFormat::Json => {
            let artifact = json_artifact(&composed)?;
            emit_or_show_artifact(artifact, show)?;
        }
    }

    // Emit compose warnings to stderr
    if !report.warnings.is_empty() {
        use biscuit_terminal::components::renderable::Renderable;
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
        use biscuit_terminal::components::renderable::Renderable;
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

/// Default editor priority when neither `$EDITOR` nor `$VISUAL` resolve to an
/// installed binary. Ordered from most capable/modern to most basic.
const DEFAULT_EDITOR_PRIORITY: &[sniff::programs::Editor] = &[
    sniff::programs::Editor::Neovim,
    sniff::programs::Editor::Helix,
    sniff::programs::Editor::Vim,
    sniff::programs::Editor::Zed,
    sniff::programs::Editor::VSCode,
    sniff::programs::Editor::VSCodium,
    sniff::programs::Editor::Sublime,
    sniff::programs::Editor::Micro,
    sniff::programs::Editor::Kakoune,
    sniff::programs::Editor::Emacs,
    sniff::programs::Editor::Lapce,
    sniff::programs::Editor::TextMate,
    sniff::programs::Editor::BBEdit,
    sniff::programs::Editor::Kate,
    sniff::programs::Editor::Geany,
    sniff::programs::Editor::Nano,
    sniff::programs::Editor::Vi,
    sniff::programs::Editor::Amp,
    sniff::programs::Editor::XEmacs,
    sniff::programs::Editor::PhpStorm,
    sniff::programs::Editor::IntellijIdea,
    sniff::programs::Editor::PyCharm,
    sniff::programs::Editor::WebStorm,
    sniff::programs::Editor::CLion,
    sniff::programs::Editor::GoLand,
    sniff::programs::Editor::Rider,
];

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

    // --- Select the editor ---
    let editor_cmd = resolve_editor_command()?;

    // --- Launch the editor ---
    let canonical = path
        .canonicalize()
        .wrap_err_with(|| format!("Failed to canonicalize path: {}", path.display()))?;

    // Split editor command into binary + any pre-existing args (e.g., "code --new-window")
    let mut parts = editor_cmd.split_whitespace();
    let editor_bin = parts.next().unwrap_or(&editor_cmd);
    let mut cmd = Command::new(editor_bin);
    for arg in parts {
        cmd.arg(arg);
    }

    // GUI editors exit immediately unless told to wait — inject the appropriate flag.
    for flag in wait_args_for_editor(editor_bin) {
        cmd.arg(flag);
    }

    cmd.arg(&canonical);

    let status = cmd
        .status()
        .wrap_err_with(|| format!("Failed to launch editor: {}", editor_cmd))?;

    if !status.success() {
        return Err(eyre!(
            "Editor exited with non-zero status: {}",
            status.code().unwrap_or(-1)
        ));
    }

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

/// Resolve the editor command to use, checking (in order):
/// 1. `$EDITOR` environment variable
/// 2. `$VISUAL` environment variable
/// 3. First installed editor from `DEFAULT_EDITOR_PRIORITY`
fn resolve_editor_command() -> Result<String> {
    use sniff::programs::{ProgramMetadata, find_program};

    // Check $EDITOR
    if let Ok(editor) = std::env::var("EDITOR") {
        let cmd = editor.split_whitespace().next().unwrap_or(&editor);
        if find_program(cmd).is_some() {
            return Ok(editor);
        }
    }

    // Check $VISUAL
    if let Ok(visual) = std::env::var("VISUAL") {
        let cmd = visual.split_whitespace().next().unwrap_or(&visual);
        if find_program(cmd).is_some() {
            return Ok(visual);
        }
    }

    // Fall back to default priority list
    let editors = sniff::programs::InstalledEditors::new();
    for &editor in DEFAULT_EDITOR_PRIORITY {
        if editors.is_installed(editor) {
            return Ok(editor.binary_name().to_string());
        }
    }

    Err(eyre!(
        "No editor found. Set $EDITOR or $VISUAL, or install one of: nvim, vim, code, nano"
    ))
}

/// Returns the CLI flags needed to make a GUI editor block until the file is closed.
///
/// Terminal editors (vim, neovim, helix, nano, etc.) naturally block because they
/// run in the foreground. GUI editors use a client-server model where the CLI wrapper
/// exits immediately after handing off to the running instance. The `--wait` flag
/// (or equivalent) tells the CLI wrapper to stay alive until the file tab is closed.
///
/// Returns an empty slice for terminal editors or unrecognized binaries.
fn wait_args_for_editor(binary: &str) -> &'static [&'static str] {
    match binary {
        // VS Code / VS Codium
        "code" | "codium" | "code-insiders" => &["--wait"],
        // Sublime Text
        "subl" => &["--wait"],
        // Zed
        "zed" => &["--wait"],
        // TextMate
        "mate" => &["--wait"],
        // BBEdit
        "bbedit" => &["--wait"],
        // Kate
        "kate" => &["--block"],
        // JetBrains IDEs
        "phpstorm" | "idea" | "pycharm" | "webstorm" | "clion" | "goland" | "rider" => &["--wait"],
        _ => &[],
    }
}

/// Hash a markdown document's frontmatter and/or body.
///
/// When the input is a directory, recursively finds all markdown files,
/// hashes each in parallel, concatenates the per-file hashes, and
/// produces a single aggregate hash.
#[instrument(skip_all)]
pub fn run_hash(
    input: Option<&PathBuf>,
    body_only: bool,
    frontmatter_only: bool,
    strict: bool,
) -> Result<()> {
    // Directory mode: aggregate hash over all markdown files
    if let Some(path) = input
        && path.is_dir()
    {
        let mut paths = collect_markdown_files(path)?;
        paths.sort();

        let per_file_hashes: Vec<String> = paths
            .par_iter()
            .map(|p| {
                let md = Markdown::try_from(p.as_path())
                    .unwrap_or_else(|_| Markdown::from(String::new()));
                md.hash(body_only, frontmatter_only, strict)
            })
            .collect();

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

        return Ok(());
    }

    // Single file / stdin mode
    let md = load_markdown(input)?;
    println!("{}", md.hash(body_only, frontmatter_only, strict));
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
    Ok(buffer.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_args_vscode_returns_wait() {
        assert_eq!(wait_args_for_editor("code"), &["--wait"]);
    }

    #[test]
    fn wait_args_codium_returns_wait() {
        assert_eq!(wait_args_for_editor("codium"), &["--wait"]);
    }

    #[test]
    fn wait_args_code_insiders_returns_wait() {
        assert_eq!(wait_args_for_editor("code-insiders"), &["--wait"]);
    }

    #[test]
    fn wait_args_sublime_returns_wait() {
        assert_eq!(wait_args_for_editor("subl"), &["--wait"]);
    }

    #[test]
    fn wait_args_zed_returns_wait() {
        assert_eq!(wait_args_for_editor("zed"), &["--wait"]);
    }

    #[test]
    fn wait_args_kate_returns_block() {
        assert_eq!(wait_args_for_editor("kate"), &["--block"]);
    }

    #[test]
    fn wait_args_jetbrains_ides_return_wait() {
        for ide in [
            "phpstorm", "idea", "pycharm", "webstorm", "clion", "goland", "rider",
        ] {
            assert_eq!(
                wait_args_for_editor(ide),
                &["--wait"],
                "expected --wait for {ide}"
            );
        }
    }

    #[test]
    fn wait_args_textmate_returns_wait() {
        assert_eq!(wait_args_for_editor("mate"), &["--wait"]);
    }

    #[test]
    fn wait_args_bbedit_returns_wait() {
        assert_eq!(wait_args_for_editor("bbedit"), &["--wait"]);
    }

    #[test]
    fn wait_args_terminal_editors_return_empty() {
        for editor in ["nvim", "vim", "vi", "hx", "nano", "micro", "kak", "emacs"] {
            assert!(
                wait_args_for_editor(editor).is_empty(),
                "expected no wait args for {editor}"
            );
        }
    }

    #[test]
    fn wait_args_unknown_binary_returns_empty() {
        assert!(wait_args_for_editor("my-custom-editor").is_empty());
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
        use darkmatter::markdown::compose::{ComposePerfMetric, ComposePerfReport};

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
                    name: "cleanup".to_string(),
                    elapsed: std::time::Duration::from_millis(3),
                    calls: 1,
                },
                ComposePerfMetric {
                    name: "normalization".to_string(),
                    elapsed: std::time::Duration::from_millis(5),
                    calls: 1,
                },
            ],
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
    use biscuit_terminal::components::renderable::Renderable as _;
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
            ReferenceKind::CssImport | ReferenceKind::InlineCss => 3,
            ReferenceKind::ScriptImport | ReferenceKind::InlineScript => 4,
            ReferenceKind::FontImport => 5,
            ReferenceKind::MetaTag => 6,
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
        ReferenceSyntax::DirectiveFile => "directive_file",
        ReferenceSyntax::DirectiveUrl => "directive_url",
        ReferenceSyntax::DirectiveCode => "directive_code",
        ReferenceSyntax::DirectiveTocLinking => "directive_toc_linking",
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
    let child_node = graph.node_by_id(&insertion.child_node_id);
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
        obj["section_level"] = serde_json::Value::Number(level.into());
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
    use biscuit_terminal::components::renderable::Renderable;
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
        use biscuit_terminal::components::renderable::Renderable as _;

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
    use biscuit_terminal::components::renderable::Renderable as _;
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
                &metric.name,
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
