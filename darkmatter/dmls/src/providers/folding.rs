//! Folding ranges: frontmatter, heading sections, fenced code, block quotes,
//! lists, and tables.
//!
//! Sections come from the heading authority (graph); everything else is a
//! line-oriented scan that tracks fenced-code state so a `>` / `|` / `-` line
//! inside a fence is never mistaken for a quote, table, or list. Every fold is
//! line-only (no character offsets), so the Neovim `lineFoldingOnly` gate needs
//! no special case; Helix (no folding) short-circuits to empty.

use lsp_types::FoldingRange;

use super::DocumentContext;
use darkmatter::markdown::extract_frontmatter_block;

/// All folding ranges for the current document.
pub fn folding_ranges(ctx: &DocumentContext) -> Vec<FoldingRange> {
    if !ctx.profile.supports_folding {
        return Vec::new();
    }
    let text = ctx.text;
    let lines: Vec<&str> = text.split('\n').collect();
    let last_line = if text.ends_with('\n') {
        lines.len().saturating_sub(2)
    } else {
        lines.len().saturating_sub(1)
    };

    let mut folds: Vec<FoldingRange> = Vec::new();

    // Frontmatter block (always at document start, line 0).
    let frontmatter_end = match extract_frontmatter_block(text) {
        Ok(Some(extraction)) => {
            let end = closing_line(text, extraction.block_span.end);
            if end > 0 {
                folds.push(fold(0, end));
            }
            end
        }
        _ => 0,
    };

    section_folds(ctx, last_line, &mut folds);
    block_folds(&lines, frontmatter_end, last_line, &mut folds);
    folds
}

/// Heading-section folds: each heading down to the line before the next
/// same-or-higher heading (else end of document).
fn section_folds(ctx: &DocumentContext, last_line: usize, folds: &mut Vec<FoldingRange>) {
    let Some(doc_id) = ctx.doc_id else {
        return;
    };
    let headings: Vec<(u8, usize)> = ctx
        .graph
        .headings(doc_id)
        .filter_map(|(_, node)| node.as_heading().map(|h| (h.level, h.line)))
        .collect();
    for (position, &(level, line)) in headings.iter().enumerate() {
        let start = line.saturating_sub(1);
        let end = headings
            .iter()
            .skip(position + 1)
            .find(|(next_level, _)| *next_level <= level)
            .map(|(_, next_line)| next_line.saturating_sub(2))
            .unwrap_or(last_line);
        if end > start {
            folds.push(fold(start, end));
        }
    }
}

/// Fenced-code, block-quote, list, and table folds via a single line scan.
fn block_folds(lines: &[&str], start_line: usize, last_line: usize, folds: &mut Vec<FoldingRange>) {
    let mut index = if start_line == 0 { 0 } else { start_line + 1 };
    let mut fence: Option<(usize, char)> = None;
    while index <= last_line && index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim_start();

        if let Some((fence_start, marker)) = fence {
            if is_fence_close(trimmed, marker) {
                folds.push(fold(fence_start, index));
                fence = None;
            }
            index += 1;
            continue;
        }
        if let Some(marker) = fence_open_marker(trimmed) {
            fence = Some((index, marker));
            index += 1;
            continue;
        }
        if trimmed.starts_with('>') {
            let start = index;
            while index <= last_line && lines[index].trim_start().starts_with('>') {
                index += 1;
            }
            if index - 1 > start {
                folds.push(fold(start, index - 1));
            }
            continue;
        }
        if is_table_row(line) && index < last_line && is_table_delimiter(lines[index + 1]) {
            let start = index;
            index += 1;
            while index <= last_line && is_table_row(lines[index]) {
                index += 1;
            }
            folds.push(fold(start, index - 1));
            continue;
        }
        if is_list_item(trimmed) {
            let start = index;
            while index <= last_line && is_list_item(lines[index].trim_start()) {
                index += 1;
            }
            if index - 1 > start {
                folds.push(fold(start, index - 1));
            }
            continue;
        }
        index += 1;
    }
    // An unclosed fence folds to the last line.
    if let Some((fence_start, _)) = fence
        && last_line > fence_start
    {
        folds.push(fold(fence_start, last_line));
    }
}

/// The 0-based line of a block's closing delimiter, given the block's end
/// byte offset (which sits just past the closing delimiter's terminator).
fn closing_line(text: &str, block_end: usize) -> usize {
    let prefix = &text[..block_end.min(text.len())];
    let newlines = prefix.bytes().filter(|&b| b == b'\n').count();
    if prefix.ends_with('\n') {
        newlines.saturating_sub(1)
    } else {
        newlines
    }
}

fn fold(start_line: usize, end_line: usize) -> FoldingRange {
    FoldingRange {
        start_line: start_line as u32,
        start_character: None,
        end_line: end_line as u32,
        end_character: None,
        kind: None,
        collapsed_text: None,
    }
}

fn fence_open_marker(trimmed: &str) -> Option<char> {
    if trimmed.starts_with("```") {
        Some('`')
    } else if trimmed.starts_with("~~~") {
        Some('~')
    } else {
        None
    }
}

fn is_fence_close(trimmed: &str, marker: char) -> bool {
    let fence = trimmed.trim_end();
    fence.len() >= 3 && fence.chars().all(|c| c == marker)
}

fn is_list_item(trimmed: &str) -> bool {
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        return true;
    }
    // Ordered list: leading digits then `.`/`)` then a space.
    let digits: String = trimmed.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return false;
    }
    let rest = &trimmed[digits.len()..];
    (rest.starts_with(". ") || rest.starts_with(") ")) && !digits.is_empty()
}

fn is_table_row(line: &str) -> bool {
    line.contains('|') && !line.trim().is_empty()
}

fn is_table_delimiter(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.contains('-')
        && trimmed
            .chars()
            .all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fence_markers() {
        assert_eq!(fence_open_marker("```rust"), Some('`'));
        assert_eq!(fence_open_marker("~~~"), Some('~'));
        assert_eq!(fence_open_marker("text"), None);
        assert!(is_fence_close("```", '`'));
        assert!(!is_fence_close("```rust", '`'));
    }

    #[test]
    fn test_list_and_table_detection() {
        assert!(is_list_item("- item"));
        assert!(is_list_item("1. item"));
        assert!(is_list_item("12) item"));
        assert!(!is_list_item("not a list"));
        assert!(is_table_row("| a | b |"));
        assert!(is_table_delimiter("|---|:--|"));
        assert!(!is_table_delimiter("| a | b |"));
    }
}
