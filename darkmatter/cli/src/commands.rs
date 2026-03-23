use crate::args::{Cli, Command as CliCommand, OutputFormat};
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
        } => {
            let mode = resolve_list_spacing(compact, loose);
            run_compose(
                input.as_ref(),
                state.as_deref(),
                set.as_deref(),
                output,
                show,
                frontmatter,
                mode,
                indent,
                cli,
            )?;
        }
        CliCommand::Toc { input, json } => {
            let md = load_markdown(Some(&input))?;
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
pub fn run_render(
    input: Option<&PathBuf>,
    output: OutputFormat,
    show: bool,
    indent: Option<usize>,
    cli: &Cli,
) -> Result<()> {
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
pub fn run_compose(
    input: Option<&PathBuf>,
    state_json: Option<&str>,
    set_json: Option<&str>,
    output: OutputFormat,
    show: bool,
    include_frontmatter: bool,
    list_spacing: ListSpacingMode,
    indent: Option<usize>,
    cli: &Cli,
) -> Result<()> {
    let md = load_markdown(input)?;

    // Resolve the input path through FileReference (handles @-prefixed paths)
    // so that source_file and policy_root use the real filesystem path.
    let resolved_input = if let Some(path) = input
        && path.to_str() != Some("-")
    {
        Some(resolve_file_path(path)?)
    } else {
        None
    };

    let mut options = ComposeOptions::new();

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
    if let Some(size) = indent {
        options = options.with_indent_size(size);
    }

    let (composed, _report) = md.compose_with(options).map_err(|e| {
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

    Ok(())
}

/// Get frontmatter properties from a markdown document.
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
        let props_label = if removed.len() == 1 {
            format!("\"{}\" property", removed[0])
        } else {
            format!(
                "[{}] properties",
                removed
                    .iter()
                    .map(|p| format!("\"{p}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let remaining_label = remaining.join(", ");
        eprintln!(
            "- removed the {} from frontmatter (\x1b[2mremaining: \x1b[3m{}\x1b[0m\x1b[2m)\x1b[0m",
            props_label, remaining_label
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
}
