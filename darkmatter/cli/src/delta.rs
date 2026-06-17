//! Delta comparison output formatting for the `md` CLI.
//!
//! Formats a [`MarkdownDelta`] as styled terminal output. This is the
//! rendering counterpart to [`darkmatter::markdown::Markdown::delta`].

use darkmatter::markdown::{Markdown, MarkdownDelta};
use std::io::{self, Write};

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
