use crate::args::Cli;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Context, Result};
use darkmatter::layout::{DarkmatterPage, PageComponent};
use darkmatter::markdown::highlighting::{ColorMode, ThemePair};
use darkmatter::markdown::output::MermaidMode;
use darkmatter::markdown::output::terminal::TerminalImageMode;
use darkmatter::markdown::{Markdown, MarkdownDelta, MarkdownToc, MarkdownTocNode};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct OutputArtifact {
    pub content: String,
    pub extension: &'static str,
    pub label: &'static str,
}

pub fn render_terminal_output(
    md: &Markdown,
    input_path: Option<&PathBuf>,
    cli: &Cli,
    prose_theme: ThemePair,
    code_theme: ThemePair,
    color_mode: ColorMode,
) -> Result<()> {
    let term = Terminal::new();
    let mut page = DarkmatterPage::new(&term)
        .with_prose_theme(prose_theme.kebab_name())
        .with_code_theme(code_theme.kebab_name())
        .with_color_mode(color_mode)
        .with_image_mode(terminal_image_mode_from_env())
        .with_mermaid_mode(if cli.mermaid {
            MermaidMode::Image
        } else {
            MermaidMode::Off
        });

    if let Some(path) = input_path
        && path.to_str() != Some("-")
    {
        page = page.with_base_path(path.parent().map(|p| p.to_path_buf()).unwrap_or_default());
    }

    // Apply layout flags from CLI.
    page = apply_cli_layout_flags(page, cli);

    // Handle line numbers: CLI flag overrides default.
    if let Some(on) = cli.line_numbers {
        page = page.with_line_numbers(on);
    }

    let output = page
        .render(md)
        .context("Failed to render markdown for terminal")?;

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_all(output.as_bytes())
        .context("Failed to write terminal output")?;

    Ok(())
}

/// Apply CLI layout flags to a [`DarkmatterPage`].
///
/// Precedence: margin shorthand → axis → side-specific.
/// Same for padding. Alignment: global → component-specific.
/// Fill: global → component-specific.
pub fn apply_cli_layout_flags(page: DarkmatterPage, cli: &Cli) -> DarkmatterPage {
    let mut page = page;

    // Margin precedence: all > axis > side
    if let Some(n) = cli.margin {
        page = page.with_margin(n);
    }
    if let Some(n) = cli.mx {
        page = page.with_margin_x(n);
    }
    if let Some(n) = cli.my {
        page = page.with_margin_y(n);
    }
    if let Some(n) = cli.mt {
        page = page.with_margin_top(n);
    }
    if let Some(n) = cli.mb {
        page = page.with_margin_bottom(n);
    }
    if let Some(n) = cli.ml {
        page = page.with_margin_left(n);
    }
    if let Some(n) = cli.mr {
        page = page.with_margin_right(n);
    }

    // Padding precedence: all > axis > side
    if let Some(n) = cli.padding {
        page = page.with_padding(n);
    }
    if let Some(n) = cli.px {
        page = page.with_padding_x(n);
    }
    if let Some(n) = cli.py {
        page = page.with_padding_y(n);
    }
    if let Some(n) = cli.pt {
        page = page.with_padding_top(n);
    }
    if let Some(n) = cli.pb {
        page = page.with_padding_bottom(n);
    }
    if let Some(n) = cli.pl {
        page = page.with_padding_left(n);
    }
    if let Some(n) = cli.pr {
        page = page.with_padding_right(n);
    }

    // Page background
    if let Some(bg) = cli.page_bg {
        page = page.with_page_background(bg.into());
    }

    // Max width
    if let Some(n) = cli.max_width {
        page = page.with_max_width(n);
    }

    // Alignment precedence: global > component-specific
    if let Some(align) = cli.alignment {
        page = page.use_alignment_for_all(align.into());
    }
    if let Some(align) = cli.align_images {
        page = page.use_alignment(PageComponent::Images, align.into());
    }
    if let Some(align) = cli.align_lists {
        page = page.use_alignment(PageComponent::Lists, align.into());
    }
    if let Some(align) = cli.align_block_quotes {
        page = page.use_alignment(PageComponent::BlockQuotes, align.into());
    }
    if let Some(align) = cli.align_tables {
        page = page.use_alignment(PageComponent::Tables, align.into());
    }
    if let Some(align) = cli.align_code_blocks {
        page = page.use_alignment(PageComponent::CodeBlocks, align.into());
    }

    // Fill precedence: global > component-specific
    if let Some(fill) = cli.fill {
        page = page.with_fill_for_all(fill);
    }
    if let Some(fill) = cli.fill_images {
        page = page.with_fill(PageComponent::Images, fill);
    }
    if let Some(fill) = cli.fill_lists {
        page = page.with_fill(PageComponent::Lists, fill);
    }
    if let Some(fill) = cli.fill_block_quotes {
        page = page.with_fill(PageComponent::BlockQuotes, fill);
    }
    if let Some(fill) = cli.fill_tables {
        page = page.with_fill(PageComponent::Tables, fill);
    }
    if let Some(fill) = cli.fill_code_blocks {
        page = page.with_fill(PageComponent::CodeBlocks, fill);
    }

    page
}

pub fn markdown_artifact(md: &Markdown) -> OutputArtifact {
    OutputArtifact {
        content: md.as_string(),
        extension: "md",
        label: "markdown",
    }
}

pub fn html_artifact(
    md: &Markdown,
    prose_theme: ThemePair,
    code_theme: ThemePair,
    color_mode: ColorMode,
    cli: &Cli,
) -> Result<OutputArtifact> {
    let term = Terminal::new_optimistic(120);
    let page = apply_cli_layout_flags(
        DarkmatterPage::new(&term)
            .with_prose_theme(prose_theme.kebab_name())
            .with_code_theme(code_theme.kebab_name())
            .with_color_mode(color_mode),
        cli,
    );

    let content = page
        .render_to_browser(md)
        .context("Failed to convert to HTML")?;

    Ok(OutputArtifact {
        content,
        extension: "html",
        label: "html",
    })
}

pub fn json_artifact(md: &Markdown) -> Result<OutputArtifact> {
    let ast = md.as_ast().context("Failed to generate AST")?;
    let content = serde_json::to_string_pretty(&ast)?;
    Ok(OutputArtifact {
        content,
        extension: "json",
        label: "json",
    })
}

pub fn emit_or_show_artifact(artifact: OutputArtifact, show: bool) -> Result<()> {
    if show {
        open_output_artifact(&artifact)
    } else {
        println!("{}", artifact.content);
        Ok(())
    }
}

pub fn open_output_artifact(artifact: &OutputArtifact) -> Result<()> {
    let temp_path = write_output_artifact_file(artifact)?;

    // MD_DRY_RUN=1 writes the temp file but skips launching the viewer.
    // Useful in tests and CI where opening a GUI app is undesirable.
    if std::env::var("MD_DRY_RUN").is_ok_and(|v| !v.is_empty()) {
        return Ok(());
    }

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

pub fn terminal_image_mode_from_env() -> TerminalImageMode {
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

pub fn parse_bool_env(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" | "y" => Some(true),
        "0" | "false" | "no" | "off" | "n" => Some(false),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TOC Tree Output
// ─────────────────────────────────────────────────────────────────────────────

/// Prints the table of contents as a text-based tree.
///
/// If `filename` is provided, it will be displayed in bold after the document icon.
pub fn print_toc_tree(toc: &MarkdownToc, verbose: bool, filename: Option<&str>) {
    // ANSI escape codes
    const BOLD: &str = "\x1b[1m";
    const RESET: &str = "\x1b[0m";

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
pub fn format_code_block_change(lang: &str, section_path: &str, description: &str) -> String {
    // Parse the description to determine change type and format accordingly
    if let Some(rest) = description.strip_prefix("Language: ") {
        // Language change: "Language: none → text"
        format!(
            "{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET} 
             changed its {BOLD}language{RESET} setting: {rest}"
        )
    } else if let Some(rest) = description.strip_prefix("'") {
        // Property change: "'title': "old" → "new"" -> "title property: "old" → "new""
        if let Some((prop_name, value_part)) = rest.split_once("':") {
            format!(
                "{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET} 
                 changed its {BOLD}{prop_name}{RESET} property:{value_part}"
            )
        } else {
            // Fallback if parsing fails
            format!(
                "{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET} 
                 changed: {description}"
            )
        }
    } else if description.starts_with("Modified") {
        // Content modified
        format!(
            "{INVERSE}{lang}{RESET} code block in {BOLD}{section_path}{RESET} 
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
pub fn print_delta(delta: &MarkdownDelta, verbose: bool, original: &Markdown, updated: &Markdown) {
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
