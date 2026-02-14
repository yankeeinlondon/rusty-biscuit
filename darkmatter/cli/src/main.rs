use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::highlighting::{
    ColorMode, ThemePair, detect_code_theme, detect_color_mode, detect_prose_theme,
};
use darkmatter::markdown::output::terminal::TerminalImageMode;
use darkmatter::markdown::output::{HtmlOptions, MermaidMode, TerminalOptions, write_terminal};
use darkmatter::markdown::transform::TransformOptions;
use darkmatter::markdown::{Markdown, MarkdownDelta, MarkdownToc, MarkdownTocNode};
use darkmatter_cli::{Cli, CliCommand, OutputFormat};
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug)]
struct OutputArtifact {
    content: String,
    extension: &'static str,
    label: &'static str,
}

/// Initialize tracing subscriber based on verbosity level.
///
/// Verbosity levels:
/// - 0 (default): WARN only (errors and warnings)
/// - 1 (-v): INFO (tool calls, phase transitions)
/// - 2 (-vv): DEBUG (tool arguments, API requests)
/// - 3 (-vvv): TRACE (request/response bodies)
/// - 4+ (-vvvv): TRACE with file/line numbers
fn init_tracing(verbose: u8) {
    // Only initialize if verbose mode is enabled
    if verbose == 0 {
        return;
    }

    let base_filter = match std::env::var("RUST_LOG") {
        Ok(filter) => filter,
        Err(_) => match verbose {
            // -v: Show INFO for progress and tool calls
            1 => "info,md=info,darkmatter=info".to_string(),
            // -vv: Show DEBUG for tool arguments and requests
            2 => "info,md=debug,darkmatter=debug".to_string(),
            // -vvv+: Show TRACE for detailed debugging
            _ => "debug,md=trace,darkmatter=trace".to_string(),
        },
    };

    let filter = EnvFilter::try_new(&base_filter).unwrap_or_else(|_| EnvFilter::new("warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_thread_ids(false)
                .with_file(verbose >= 4)
                .with_line_number(verbose >= 4)
                .with_writer(std::io::stderr)
                .compact(),
        )
        .init();
}

fn main() -> Result<()> {
    color_eyre::install()?;

    // Handle dynamic shell completions (invoked by shell completion scripts)
    CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();
    init_tracing(cli.verbose);

    // Handle --completions first (no input needed)
    if let Some(shell) = cli.completions {
        print_completions(shell);
        return Ok(());
    }

    // Handle --list-themes (no input needed)
    if cli.list_themes {
        list_themes();
        return Ok(());
    }

    if let Some(command) = cli.command.clone() {
        validate_subcommand_usage(&cli)?;
        run_subcommand(command, &cli)?;
        return Ok(());
    }

    // No subcommand given — if no input and interactive terminal, show help
    if cli.input.is_none() && io::stdin().is_terminal() {
        Cli::command().print_help()?;
        return Ok(());
    }

    // Run as implicit read using top-level args
    run_read(cli.input.as_ref(), cli.output, cli.show, &cli)?;

    Ok(())
}

fn validate_subcommand_usage(cli: &Cli) -> Result<()> {
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

    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(eyre!(
            "subcommands cannot be combined with top-level render options: {}",
            conflicts.join(", ")
        ))
    }
}

fn run_subcommand(command: CliCommand, cli: &Cli) -> Result<()> {
    match command {
        CliCommand::Read {
            input,
            output,
            show,
        } => {
            run_read(input.as_ref(), output, show, cli)?;
        }
        CliCommand::Clean { input } => {
            let mut md = load_markdown(input.as_ref())?;
            md.cleanup();
            println!("{}", md.as_string());
        }
        CliCommand::Compose {
            input,
            state,
            output,
            show,
        } => {
            run_compose(input.as_ref(), state.as_deref(), output, show, cli)?;
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
    }

    Ok(())
}

/// Shared read/render logic for both implicit (no subcommand) and explicit `read` subcommand.
fn run_read(
    input: Option<&PathBuf>,
    output: OutputFormat,
    show: bool,
    cli: &Cli,
) -> Result<()> {
    let md = load_markdown(input)?;

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

/// Run the compose (transform) pipeline.
fn run_compose(
    input: Option<&PathBuf>,
    state_json: Option<&str>,
    output: OutputFormat,
    show: bool,
    cli: &Cli,
) -> Result<()> {
    let md = load_markdown(input)?;

    let mut options = TransformOptions::new();

    // Parse --state as JSON if provided
    if let Some(json_str) = state_json {
        let state: serde_json::Value =
            serde_json::from_str(json_str).wrap_err("Invalid JSON in --state argument")?;
        options = options.with_external_state(state);
    }

    // Set source file for relative transclusion resolution
    if let Some(path) = input
        && path.to_str() != Some("-")
    {
        options = options.with_source_file(path);
    }

    let (transformed, _report) = md
        .transform_with(options)
        .map_err(|e| eyre!("Transform failed: {}", e))?;

    let prose_theme = cli.theme.unwrap_or_else(detect_prose_theme);
    let code_theme = cli
        .code_theme
        .unwrap_or_else(|| detect_code_theme(prose_theme));
    let color_mode = detect_color_mode();

    match output {
        OutputFormat::Auto | OutputFormat::Markdown => {
            // Frontmatter drives the pipeline; once composition is complete, discard it.
            let content = transformed.content().to_string();
            if show {
                let artifact = OutputArtifact {
                    content: content.clone(),
                    extension: "md",
                    label: "markdown",
                };
                print!("{content}");
                open_output_artifact(&artifact)?;
            } else {
                print!("{content}");
            }
        }
        OutputFormat::Html => {
            let artifact = html_artifact(&transformed, prose_theme, code_theme, color_mode)?;
            emit_or_show_artifact(artifact, show)?;
        }
        OutputFormat::Json => {
            let artifact = json_artifact(&transformed)?;
            emit_or_show_artifact(artifact, show)?;
        }
    }

    Ok(())
}

fn render_terminal_output(
    md: &Markdown,
    input_path: Option<&PathBuf>,
    cli: &Cli,
    prose_theme: ThemePair,
    code_theme: ThemePair,
    color_mode: ColorMode,
) -> Result<()> {
    let mut options = TerminalOptions::default();
    options.prose_theme = prose_theme;
    options.code_theme = code_theme;
    options.color_mode = color_mode;
    options.include_line_numbers = cli.line_numbers;
    options.color_depth = None; // Auto-detect
    options.image_mode = terminal_image_mode_from_env();
    options.mermaid_mode = if cli.mermaid {
        MermaidMode::Image
    } else {
        MermaidMode::Off
    };

    // Derive base_path from input file for relative image resolution
    if let Some(path) = input_path
        && path.to_str() != Some("-")
    {
        options.base_path = path.parent().map(|p| p.to_path_buf());
    }

    // Use write_terminal with stdout for proper image rendering
    // (viuer requires direct stdout access for graphics protocols)
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    write_terminal(&mut handle, md, options).context("Failed to render markdown for terminal")?;

    Ok(())
}

fn markdown_artifact(md: &Markdown) -> OutputArtifact {
    OutputArtifact {
        content: md.as_string(),
        extension: "md",
        label: "markdown",
    }
}

fn html_artifact(
    md: &Markdown,
    prose_theme: ThemePair,
    code_theme: ThemePair,
    color_mode: ColorMode,
) -> Result<OutputArtifact> {
    let mut options = HtmlOptions::default();
    options.prose_theme = prose_theme;
    options.code_theme = code_theme;
    options.color_mode = color_mode;
    options.mermaid_mode = MermaidMode::Image;

    let content = md.as_html(options).context("Failed to convert to HTML")?;
    Ok(OutputArtifact {
        content,
        extension: "html",
        label: "html",
    })
}

fn json_artifact(md: &Markdown) -> Result<OutputArtifact> {
    let ast = md.as_ast().context("Failed to generate AST")?;
    let content = serde_json::to_string_pretty(&ast)?;
    Ok(OutputArtifact {
        content,
        extension: "json",
        label: "json",
    })
}

fn emit_or_show_artifact(artifact: OutputArtifact, show: bool) -> Result<()> {
    if show {
        open_output_artifact(&artifact)
    } else {
        print!("{}", artifact.content);
        Ok(())
    }
}

fn open_output_artifact(artifact: &OutputArtifact) -> Result<()> {
    let temp_path = write_output_artifact_file(artifact)?;

    // Non-blocking open, graceful error handling
    if let Err(error) = open::that(&temp_path) {
        eprintln!("Failed to open {} output: {}", artifact.label, error);
        eprintln!("Preview available at: {}", temp_path.display());
    }

    Ok(())
}

fn write_output_artifact_file(artifact: &OutputArtifact) -> Result<PathBuf> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let filename = format!(
        "md-output-{}-{}.{}",
        std::process::id(),
        timestamp,
        artifact.extension
    );
    let path = std::env::temp_dir().join(filename);

    std::fs::write(&path, &artifact.content)
        .wrap_err_with(|| format!("Failed to write {} output file", artifact.label))?;

    Ok(path)
}

fn terminal_image_mode_from_env() -> TerminalImageMode {
    let Ok(raw) = std::env::var("TERMINAL_IMAGES") else {
        return TerminalImageMode::Auto;
    };

    match parse_bool_env(&raw) {
        Some(true) => TerminalImageMode::Force,
        Some(false) => TerminalImageMode::Never,
        None => {
            tracing::warn!(value = %raw, "Invalid TERMINAL_IMAGES value; falling back to auto mode");
            TerminalImageMode::Auto
        }
    }
}

fn parse_bool_env(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" | "y" => Some(true),
        "0" | "false" | "no" | "off" | "n" => Some(false),
        _ => None,
    }
}

/// Loads markdown from a file path or stdin.
fn load_markdown(path: Option<&PathBuf>) -> Result<Markdown> {
    if let Some(p) = path {
        if p.to_str() == Some("-") {
            // Explicit stdin marker
            read_from_stdin()
        } else {
            Markdown::try_from(p.as_path())
                .wrap_err_with(|| format!("Failed to read file: {:?}", p))
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

/// Reads markdown content from stdin.
fn read_from_stdin() -> Result<Markdown> {
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .wrap_err("Failed to read from stdin")?;
    Ok(buffer.into())
}

/// Lists all available themes with descriptions.
fn list_themes() {
    println!("Available themes:\n");
    for theme_pair in ThemePair::all() {
        println!(
            "  {:20} {}",
            theme_pair.kebab_name(),
            theme_pair.description(ColorMode::Dark)
        );
    }
    println!("\nUse --theme <name> to set prose theme");
    println!("Use --code-theme <name> to override code theme");
}

/// Prints shell completions setup instructions.
///
/// With dynamic completions, the shell sources a command that calls back to the CLI.
/// This outputs the appropriate setup command for each shell.
fn print_completions(shell: clap_complete::Shell) {
    use clap_complete::Shell;

    let (setup_cmd, config_file) = match shell {
        Shell::Bash => (r#"source <(COMPLETE=bash md)"#, "~/.bashrc"),
        Shell::Zsh => (r#"source <(COMPLETE=zsh md)"#, "~/.zshrc"),
        Shell::Fish => (r#"COMPLETE=fish md | source"#, "~/.config/fish/config.fish"),
        Shell::PowerShell => (
            r#"$env:COMPLETE = "powershell"; md | Out-String | Invoke-Expression; Remove-Item Env:\COMPLETE"#,
            "$PROFILE",
        ),
        Shell::Elvish => (r#"eval (E:COMPLETE=elvish md | slurp)"#, "~/.elvish/rc.elv"),
        _ => {
            eprintln!("Shell {:?} is not supported for dynamic completions", shell);
            return;
        }
    };

    println!("# Add this line to {}:", config_file);
    println!("{}", setup_cmd);
}

// ─────────────────────────────────────────────────────────────────────────────
// TOC Tree Output
// ─────────────────────────────────────────────────────────────────────────────

/// Prints the table of contents as a text-based tree.
///
/// If `filename` is provided, it will be displayed in bold after the document icon.
fn print_toc_tree(toc: &MarkdownToc, verbose: bool, filename: Option<&str>) {
    let stdout = io::stdout();
    let stderr = io::stderr();
    let mut out = stdout.lock();
    let mut err = stderr.lock();

    // Breathing room: blank line to stderr before TOC
    writeln!(err).ok();

    // Print document icon, optionally with filename in bold
    if toc.title.is_some() {
        if let Some(name) = filename {
            writeln!(out, "📄 {BOLD}{name}{RESET}").ok();
        } else {
            writeln!(out, "📄").ok();
        }
        if verbose {
            writeln!(
                err,
                "   Page hash: {:016x} (trimmed: {:016x})",
                toc.page_hash, toc.page_hash_trimmed
            )
            .ok();
        }
    }

    // Print the tree structure
    for (i, node) in toc.structure.iter().enumerate() {
        let is_last = i == toc.structure.len() - 1;
        print_toc_node(&mut out, node, "", is_last, verbose);
    }

    // Breathing room: blank line to stderr after TOC
    writeln!(err).ok();

    // Print summary only in verbose mode, to stderr
    if verbose {
        writeln!(
            err,
            "Total: {} heading{}",
            toc.heading_count(),
            if toc.heading_count() == 1 { "" } else { "s" }
        )
        .ok();

        if !toc.code_blocks.is_empty() {
            writeln!(err, "Code blocks: {}", toc.code_blocks.len()).ok();
        }

        if !toc.internal_links.is_empty() {
            let broken_count = toc.broken_links().len();
            if broken_count > 0 {
                writeln!(
                    err,
                    "Internal links: {} ({} broken)",
                    toc.internal_links.len(),
                    broken_count
                )
                .ok();
            } else {
                writeln!(err, "Internal links: {}", toc.internal_links.len()).ok();
            }
        }
    }
}

/// Recursively prints a TOC node with tree characters.
fn print_toc_node<W: Write>(
    out: &mut W,
    node: &MarkdownTocNode,
    prefix: &str,
    is_last: bool,
    verbose: bool,
) {
    // Tree connector characters
    let connector = if is_last { "└── " } else { "├── " };
    let child_prefix = if is_last { "    " } else { "│   " };

    if verbose {
        // Show semantic content hash (used for whitespace-insensitive comparison)
        writeln!(
            out,
            "{}{}{} ({:016x})",
            prefix,
            connector,
            node.title,
            node.prelude_hash_normalized()
        )
        .ok();
    } else {
        writeln!(out, "{}{}{}", prefix, connector, node.title).ok();
    }

    // Print children
    let new_prefix = format!("{}{}", prefix, child_prefix);
    for (i, child) in node.children.iter().enumerate() {
        let child_is_last = i == node.children.len() - 1;
        print_toc_node(out, child, &new_prefix, child_is_last, verbose);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Delta Output
// ─────────────────────────────────────────────────────────────────────────────

// ANSI escape codes
const INVERSE: &str = "\x1b[7m";
const BOLD: &str = "\x1b[1m";
const ITALIC: &str = "\x1b[3m";
const RESET: &str = "\x1b[0m";

/// Formats a code block change with ANSI styling.
///
/// Format: `{inverse}lang{reset} code block in {bold}section{reset} {description}`
fn format_code_block_change(lang: &str, section_path: &str, description: &str) -> String {
    // Parse the description to determine change type and format accordingly
    if let Some(rest) = description.strip_prefix("Language: ") {
        // Language change: "Language: none → text"
        format!(
            "{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET} \
             changed its {BOLD}language{RESET} setting: {rest}"
        )
    } else if let Some(rest) = description.strip_prefix("'") {
        // Property change: "'title': \"old\" → \"new\"" -> "title property: \"old\" → \"new\""
        if let Some((prop_name, value_part)) = rest.split_once("':") {
            format!(
                "{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET} \
                 changed its {BOLD}{prop_name}{RESET} property:{value_part}"
            )
        } else {
            // Fallback if parsing fails
            format!(
                "{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET} \
                 changed: {description}"
            )
        }
    } else if description.starts_with("Modified") {
        // Content modified
        format!(
            "{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET} \
             was {BOLD}modified{RESET}"
        )
    } else if description.starts_with("Added") {
        // Added code block
        format!("{INVERSE}{lang}{RESET} code block added in {BOLD}{section_path}{RESET}")
    } else if description.starts_with("Removed") {
        // Removed code block
        format!("{INVERSE}{lang}{RESET} code block removed from {BOLD}{section_path}{RESET}")
    } else {
        // Fallback for other descriptions
        format!("{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET}: {description}")
    }
}

/// Prints the delta comparison results.
fn print_delta(delta: &MarkdownDelta, verbose: bool, original: &Markdown, updated: &Markdown) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Blank line before output for visual separation
    writeln!(handle).ok();

    // Print classification header
    let (classification_symbol, classification_name) = match delta.classification {
        darkmatter::markdown::DocumentChange::NoChange => ("✓", "No changes"),
        darkmatter::markdown::DocumentChange::WhitespaceOnly => ("~", "Whitespace changes only"),
        darkmatter::markdown::DocumentChange::FrontmatterOnly => ("◈", "Frontmatter only"),
        darkmatter::markdown::DocumentChange::FrontmatterAndWhitespace => {
            ("◈", "Frontmatter and whitespace")
        }
        darkmatter::markdown::DocumentChange::StructuralOnly => ("⊕", "Structural only"),
        darkmatter::markdown::DocumentChange::ContentMinor => ("△", "Minor changes"),
        darkmatter::markdown::DocumentChange::ContentModerate => ("◐", "Moderate changes"),
        darkmatter::markdown::DocumentChange::ContentMajor => ("◉", "Major changes"),
        darkmatter::markdown::DocumentChange::Rewritten => ("★", "Rewritten"),
    };

    writeln!(
        handle,
        "{} {} ({:.1}% changed)",
        classification_symbol,
        classification_name,
        delta.statistics.content_change_ratio * 100.0
    )
    .ok();
    writeln!(handle).ok();

    // Print frontmatter changes
    if delta.frontmatter_changed {
        writeln!(handle, "Frontmatter:").ok();
        if delta.frontmatter_formatting_only {
            writeln!(handle, "  (formatting changes only)").ok();
        } else {
            for change in &delta.frontmatter_changes {
                let symbol = match change.action {
                    darkmatter::markdown::ChangeAction::PropertyAdded => "+",
                    darkmatter::markdown::ChangeAction::PropertyRemoved => "-",
                    darkmatter::markdown::ChangeAction::PropertyUpdated => "~",
                    _ => "?",
                };
                writeln!(
                    handle,
                    "  {} {}: {}",
                    symbol, change.key, change.description
                )
                .ok();
            }
        }
        writeln!(handle).ok();
    }

    // Print preamble changes
    if delta.preamble_changed {
        if delta.preamble_whitespace_only {
            writeln!(handle, "Preamble: whitespace changes only").ok();
        } else {
            writeln!(handle, "Preamble: modified").ok();
        }
        writeln!(handle).ok();
    }

    // Print added sections
    if !delta.added.is_empty() {
        writeln!(handle, "Added ({}):", delta.added.len()).ok();
        for change in &delta.added {
            let path_str = change
                .new_path
                .as_ref()
                .map(|p| p.join(" > "))
                .unwrap_or_default();
            if verbose {
                writeln!(
                    handle,
                    "  + {} (line {})",
                    path_str,
                    change.new_line.unwrap_or(0)
                )
                .ok();
            } else {
                writeln!(handle, "  + {}", path_str).ok();
            }
        }
        writeln!(handle).ok();
    }

    // Print removed sections
    if !delta.removed.is_empty() {
        writeln!(handle, "Removed ({}):", delta.removed.len()).ok();
        for change in &delta.removed {
            let path_str = change
                .original_path
                .as_ref()
                .map(|p| p.join(" > "))
                .unwrap_or_default();
            if verbose {
                writeln!(
                    handle,
                    "  - {} (was line {})",
                    path_str,
                    change.original_line.unwrap_or(0)
                )
                .ok();
            } else {
                writeln!(handle, "  - {}", path_str).ok();
            }
        }
        writeln!(handle).ok();
    }

    // Separate content changes from whitespace-only changes
    let content_changes: Vec<_> = delta
        .modified
        .iter()
        .filter(|c| !matches!(c.action, darkmatter::markdown::ChangeAction::WhitespaceOnly))
        .collect();
    let whitespace_changes: Vec<_> = delta
        .modified
        .iter()
        .filter(|c| matches!(c.action, darkmatter::markdown::ChangeAction::WhitespaceOnly))
        .collect();

    // Print content modifications (the important ones)
    if !content_changes.is_empty() {
        writeln!(handle, "Modified ({}):", content_changes.len()).ok();
        for change in &content_changes {
            writeln!(handle, "  - {}", change.description).ok();
        }
        writeln!(handle).ok();
    }

    // Print moved sections
    if !delta.moved.is_empty() {
        writeln!(handle, "Moved ({}):", delta.moved.len()).ok();
        for moved in &delta.moved {
            let from = moved.original_path.join(" > ");
            let to = moved.new_path.join(" > ");
            let level_change = if moved.level_delta < 0 {
                format!(" (promoted by {})", -moved.level_delta)
            } else if moved.level_delta > 0 {
                format!(" (demoted by {})", moved.level_delta)
            } else {
                String::new()
            };
            writeln!(handle, "  ↷ {} → {}{}", from, to, level_change).ok();
        }
        writeln!(handle).ok();
    }

    // Print code block changes (always show, not just verbose)
    if !delta.code_block_changes.is_empty() {
        writeln!(handle, "Code blocks:").ok();
        for change in &delta.code_block_changes {
            let lang = change.language.as_deref().unwrap_or("plain");
            // Skip H1 in section path (start from index 1 if it exists)
            let section_path = if change.section_path.len() > 1 {
                change.section_path[1..].join(" > ")
            } else if !change.section_path.is_empty() {
                change.section_path[0].clone()
            } else {
                String::from("(preamble)")
            };

            // Format with ANSI styling based on change type
            // inverse=\x1b[7m, bold=\x1b[1m, italic=\x1b[3m, reset=\x1b[0m
            let formatted = format_code_block_change(lang, &section_path, &change.description);
            writeln!(handle, "  - {}", formatted).ok();
        }
        writeln!(handle).ok();
    }

    // Print broken links
    if !delta.broken_links.is_empty() {
        writeln!(handle, "⚠ Broken links ({}):", delta.broken_links.len()).ok();
        for link in &delta.broken_links {
            write!(
                handle,
                "  ✗ #{} at line {}",
                link.target_slug, link.line_number
            )
            .ok();
            if let Some(ref suggestion) = link.suggested_replacement {
                writeln!(handle, " → did you mean #{}?", suggestion).ok();
            } else {
                writeln!(handle).ok();
            }
        }
        writeln!(handle).ok();
    }

    // Print whitespace-only changes at the end (less important)
    if !whitespace_changes.is_empty() {
        writeln!(handle, "Whitespace only ({}):", whitespace_changes.len()).ok();
        for change in &whitespace_changes {
            // Skip H1 in section path (start from index 1 if it exists)
            let path_str = change
                .original_path
                .as_ref()
                .map(|p| {
                    if p.len() > 1 {
                        p[1..].join(" > ")
                    } else if !p.is_empty() {
                        p[0].clone()
                    } else {
                        String::from("(preamble)")
                    }
                })
                .unwrap_or_default();
            // description contains the whitespace type(s) - show in italics
            writeln!(
                handle,
                "  - {}: {ITALIC}{}{RESET}",
                path_str, change.description
            )
            .ok();
        }
        // Dim italic note after the list
        writeln!(handle).ok();
        writeln!(
            handle,
            "  \x1b[2m\x1b[3mwhitespace only changes have no visual effect when rendered\x1b[0m"
        )
        .ok();
        writeln!(handle).ok();
    }

    // Print summary statistics if verbose
    if verbose {
        let stats = &delta.statistics;
        writeln!(handle, "Statistics:").ok();
        writeln!(
            handle,
            "  Bytes: {} → {} ({} changed)",
            stats.original_bytes, stats.new_bytes, stats.bytes_changed
        )
        .ok();
        writeln!(
            handle,
            "  Sections: {} → {} ({} unchanged)",
            stats.original_section_count, stats.new_section_count, stats.sections_unchanged
        )
        .ok();
        writeln!(handle).ok();

        // Visual diff output
        use darkmatter::diff::visual::{VisualDiffInput, VisualDiffOptions, render_visual_diff};

        let options = VisualDiffOptions::default();

        // Frontmatter visual diff (if changed)
        if delta.frontmatter_changed && !delta.frontmatter_formatting_only {
            let fm_orig =
                serde_yaml::to_string(original.frontmatter().as_map()).unwrap_or_default();
            let fm_upd = serde_yaml::to_string(updated.frontmatter().as_map()).unwrap_or_default();

            if !fm_orig.is_empty() || !fm_upd.is_empty() {
                writeln!(handle, "{BOLD}Frontmatter Visual Diff:{RESET}").ok();
                writeln!(
                    handle,
                    "{}",
                    render_visual_diff(
                        VisualDiffInput {
                            original: &fm_orig,
                            updated: &fm_upd,
                            label_original: "original",
                            label_updated: "updated",
                        },
                        &options,
                    )
                    .rendered
                )
                .ok();
            }
        }

        // Content body visual diff (if has content changes)
        let has_content_changes = !delta.added.is_empty()
            || !delta.removed.is_empty()
            || !delta.modified.is_empty()
            || delta.preamble_changed;

        if has_content_changes {
            writeln!(handle, "{BOLD}Content Visual Diff:{RESET}").ok();
            writeln!(
                handle,
                "{}",
                render_visual_diff(
                    VisualDiffInput {
                        original: original.content(),
                        updated: updated.content(),
                        label_original: "original",
                        label_updated: "updated",
                    },
                    &options,
                )
                .rendered
            )
            .ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_bool_env;

    #[test]
    fn parse_bool_env_supports_truthy_values() {
        for value in ["1", "true", "TRUE", "yes", "on", "y"] {
            assert_eq!(parse_bool_env(value), Some(true), "value: {value}");
        }
    }

    #[test]
    fn parse_bool_env_supports_falsy_values() {
        for value in ["0", "false", "FALSE", "no", "off", "n"] {
            assert_eq!(parse_bool_env(value), Some(false), "value: {value}");
        }
    }

    #[test]
    fn parse_bool_env_rejects_unknown_values() {
        for value in ["", "maybe", "2", "enable", "disable"] {
            assert_eq!(parse_bool_env(value), None, "value: {value}");
        }
    }
}
