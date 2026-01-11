use clap::Parser;
use color_eyre::eyre::{eyre, Context, Result};
use darkmatter::Cli;
use shared::markdown::highlighting::{
    detect_code_theme, detect_color_mode, detect_prose_theme, ColorMode, ThemePair,
};
use shared::markdown::output::{HtmlOptions, MermaidMode, TerminalOptions, write_terminal};
use shared::markdown::{Markdown, MarkdownDelta, MarkdownToc, MarkdownTocNode};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

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
            1 => "info,md=info,shared=info".to_string(),
            // -vv: Show DEBUG for tool arguments and requests
            2 => "info,md=debug,shared=debug".to_string(),
            // -vvv+: Show TRACE for detailed debugging
            _ => "debug,md=trace,shared=trace".to_string(),
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
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    // Handle --list-themes first (no input needed)
    if cli.list_themes {
        list_themes();
        return Ok(());
    }

    // Load markdown from input or stdin
    let mut md = load_markdown(cli.input.as_ref())?;

    // Handle frontmatter operations
    if let Some(ref json) = cli.fm_merge_with {
        let data: serde_json::Value =
            serde_json::from_str(json).wrap_err("Invalid JSON in --fm-merge-with argument")?;
        // TODO: Implement fm_merge_with when Markdown API is available
        eprintln!("Frontmatter merge: {:?}", data);
        return Ok(());
    }

    if let Some(ref json) = cli.fm_defaults {
        let data: serde_json::Value =
            serde_json::from_str(json).wrap_err("Invalid JSON in --fm-defaults argument")?;
        // TODO: Implement fm_defaults when Markdown API is available
        eprintln!("Frontmatter defaults: {:?}", data);
        return Ok(());
    }

    // Handle clean operations
    if cli.clean {
        md.cleanup();
        println!("{}", md.as_string());
        return Ok(());
    }

    if cli.clean_save {
        let path = cli
            .input
            .ok_or_else(|| eyre!("--clean-save requires a file path, not stdin"))?;
        md.cleanup();
        std::fs::write(&path, md.as_string())
            .wrap_err_with(|| format!("Failed to write to {:?}", path))?;
        eprintln!("Saved cleaned content to {:?}", path);
        return Ok(());
    }

    // Resolve themes
    let prose_theme = cli.theme.unwrap_or_else(detect_prose_theme);
    let code_theme = cli
        .code_theme
        .unwrap_or_else(|| detect_code_theme(prose_theme));
    let color_mode = detect_color_mode();

    // Handle output modes
    if cli.ast {
        let ast = md.as_ast().context("Failed to generate AST")?;
        println!("{}", serde_json::to_string_pretty(&ast)?);
        return Ok(());
    }

    // Handle --toc mode
    if cli.toc {
        let toc = md.toc();
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&toc)?);
        } else {
            print_toc_tree(&toc, cli.verbose > 0);
        }
        return Ok(());
    }

    // Handle --delta mode
    if let Some(ref other_path) = cli.delta {
        let other_md = Markdown::try_from(other_path.as_path())
            .wrap_err_with(|| format!("Failed to read comparison file: {:?}", other_path))?;
        let delta = md.delta(&other_md);
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&delta)?);
        } else {
            print_delta(&delta, cli.verbose > 0, &md, &other_md);
        }
        return Ok(());
    }

    if cli.html {
        let mut options = HtmlOptions::default();
        options.prose_theme = prose_theme;
        options.code_theme = code_theme;
        options.color_mode = color_mode;
        // For HTML output, default to interactive mermaid diagrams
        // (browsers can render them natively via mermaid.js)
        options.mermaid_mode = MermaidMode::Image;

        let html = md.as_html(options).context("Failed to convert to HTML")?;
        println!("{}", html);
        return Ok(());
    }

    if cli.show_html {
        let mut options = HtmlOptions::default();
        options.prose_theme = prose_theme;
        options.code_theme = code_theme;
        options.color_mode = color_mode;
        // For HTML output, default to interactive mermaid diagrams
        options.mermaid_mode = MermaidMode::Image;

        let html = md.as_html(options).context("Failed to convert to HTML")?;
        let temp_path = std::env::temp_dir().join("md-preview.html");
        std::fs::write(&temp_path, &html)
            .wrap_err("Failed to write temp HTML file")?;

        // Non-blocking open, graceful error handling
        if let Err(e) = open::that(&temp_path) {
            eprintln!("Failed to open browser: {}", e);
            eprintln!("Preview available at: {}", temp_path.display());
        }
        return Ok(());
    }

    // Default: render to terminal
    let mut options = TerminalOptions::default();
    options.prose_theme = prose_theme;
    options.code_theme = code_theme;
    options.color_mode = color_mode;
    options.include_line_numbers = cli.line_numbers;
    options.color_depth = None; // Auto-detect
    options.render_images = !cli.no_images;
    options.mermaid_mode = if cli.mermaid {
        MermaidMode::Image
    } else {
        MermaidMode::Off
    };

    // Derive base_path from input file for relative image resolution
    if let Some(ref path) = cli.input
        && path.to_str() != Some("-")
    {
        options.base_path = path.parent().map(|p| p.to_path_buf());
    }

    // Use write_terminal with stdout for proper image rendering
    // (viuer requires direct stdout access for graphics protocols)
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    write_terminal(&mut handle, &md, options).context("Failed to render markdown for terminal")?;

    Ok(())
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
        if atty::is(atty::Stream::Stdin) {
            // Interactive terminal - no input available
            Err(eyre!(
                "No input file provided. Use `md --help` for usage."
            ))
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

// ─────────────────────────────────────────────────────────────────────────────
// TOC Tree Output
// ─────────────────────────────────────────────────────────────────────────────

/// Prints the table of contents as a text-based tree.
fn print_toc_tree(toc: &MarkdownToc, verbose: bool) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Print title if available
    if let Some(ref title) = toc.title {
        writeln!(handle, "📄 {}", title).ok();
        if verbose {
            writeln!(
                handle,
                "   Page hash: {:016x} (trimmed: {:016x})",
                toc.page_hash, toc.page_hash_trimmed
            )
            .ok();
        }
        writeln!(handle).ok();
    }

    // Print the tree structure
    for (i, node) in toc.structure.iter().enumerate() {
        let is_last = i == toc.structure.len() - 1;
        print_toc_node(&mut handle, node, "", is_last, verbose);
    }

    // Print summary
    writeln!(handle).ok();
    writeln!(
        handle,
        "Total: {} heading{}",
        toc.heading_count(),
        if toc.heading_count() == 1 { "" } else { "s" }
    )
    .ok();

    if !toc.code_blocks.is_empty() {
        writeln!(handle, "Code blocks: {}", toc.code_blocks.len()).ok();
    }

    if !toc.internal_links.is_empty() {
        let broken_count = toc.broken_links().len();
        if broken_count > 0 {
            writeln!(
                handle,
                "Internal links: {} ({} broken)",
                toc.internal_links.len(),
                broken_count
            )
            .ok();
        } else {
            writeln!(handle, "Internal links: {}", toc.internal_links.len()).ok();
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
            prefix, connector, node.title, node.prelude_hash_normalized()
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
        format!(
            "{INVERSE}{lang}{RESET} code block added in {BOLD}{section_path}{RESET}"
        )
    } else if description.starts_with("Removed") {
        // Removed code block
        format!(
            "{INVERSE}{lang}{RESET} code block removed from {BOLD}{section_path}{RESET}"
        )
    } else {
        // Fallback for other descriptions
        format!(
            "{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET}: {description}"
        )
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
        shared::markdown::DocumentChange::NoChange => ("✓", "No changes"),
        shared::markdown::DocumentChange::WhitespaceOnly => ("~", "Whitespace changes only"),
        shared::markdown::DocumentChange::FrontmatterOnly => ("◈", "Frontmatter only"),
        shared::markdown::DocumentChange::FrontmatterAndWhitespace => {
            ("◈", "Frontmatter and whitespace")
        }
        shared::markdown::DocumentChange::StructuralOnly => ("⊕", "Structural only"),
        shared::markdown::DocumentChange::ContentMinor => ("△", "Minor changes"),
        shared::markdown::DocumentChange::ContentModerate => ("◐", "Moderate changes"),
        shared::markdown::DocumentChange::ContentMajor => ("◉", "Major changes"),
        shared::markdown::DocumentChange::Rewritten => ("★", "Rewritten"),
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
                    shared::markdown::ChangeAction::PropertyAdded => "+",
                    shared::markdown::ChangeAction::PropertyRemoved => "-",
                    shared::markdown::ChangeAction::PropertyUpdated => "~",
                    _ => "?",
                };
                writeln!(handle, "  {} {}: {}", symbol, change.key, change.description).ok();
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
        .filter(|c| !matches!(c.action, shared::markdown::ChangeAction::WhitespaceOnly))
        .collect();
    let whitespace_changes: Vec<_> = delta
        .modified
        .iter()
        .filter(|c| matches!(c.action, shared::markdown::ChangeAction::WhitespaceOnly))
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
                writeln!(
                    handle,
                    " → did you mean #{}?",
                    suggestion
                )
                .ok();
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
            writeln!(handle, "  - {}: {ITALIC}{}{RESET}", path_str, change.description).ok();
        }
        // Dim italic note after the list
        writeln!(handle).ok();
        writeln!(handle, "  \x1b[2m\x1b[3mwhitespace only changes have no visual effect when rendered\x1b[0m").ok();
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
        use shared::markdown::delta::visual::{render_visual_diff, VisualDiffOptions};

        let options = VisualDiffOptions::default();

        // Frontmatter visual diff (if changed)
        if delta.frontmatter_changed && !delta.frontmatter_formatting_only {
            let fm_orig = serde_yaml::to_string(original.frontmatter().as_map()).unwrap_or_default();
            let fm_upd = serde_yaml::to_string(updated.frontmatter().as_map()).unwrap_or_default();

            if !fm_orig.is_empty() || !fm_upd.is_empty() {
                writeln!(handle, "{BOLD}Frontmatter Visual Diff:{RESET}").ok();
                writeln!(
                    handle,
                    "{}",
                    render_visual_diff(&fm_orig, &fm_upd, "original", "updated", &options)
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
                    original.content(),
                    updated.content(),
                    "original",
                    "updated",
                    &options
                )
            )
            .ok();
        }
    }
}
