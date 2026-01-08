//! Markdown document delta comparison.
//!
//! This module provides functionality to compare two markdown documents and
//! generate a detailed analysis of changes, including:
//! - Content additions, removals, and modifications
//! - Section movements and reordering
//! - Frontmatter changes
//! - Broken link detection
//!
//! ## Examples
//!
//! ```rust
//! use shared::markdown::Markdown;
//!
//! let original: Markdown = "# Hello\n\nWorld".into();
//! let updated: Markdown = "# Hello\n\nUniverse".into();
//!
//! let delta = original.delta(&updated);
//!
//! if delta.is_unchanged() {
//!     println!("No changes detected");
//! } else {
//!     println!("{}", delta.summary());
//! }
//! ```

mod types;

pub use types::{
    BrokenLink, ChangeAction, CodeBlockChange, ContentChange, DeltaStatistics, DocumentChange,
    FrontmatterChange, MarkdownDelta, MovedSection, SectionId, SectionPath,
};

use crate::markdown::toc::{MarkdownToc, MarkdownTocNode};
use crate::markdown::Markdown;
use std::collections::{HashMap, HashSet};

/// Extracts all headings from a TOC into a flat list with their paths.
fn extract_headings_with_paths(toc: &MarkdownToc) -> Vec<(Vec<String>, &MarkdownTocNode)> {
    fn collect_recursive<'a>(
        node: &'a MarkdownTocNode,
        path: Vec<String>,
        result: &mut Vec<(Vec<String>, &'a MarkdownTocNode)>,
    ) {
        let mut current_path = path;
        current_path.push(node.title.clone());
        result.push((current_path.clone(), node));

        for child in &node.children {
            collect_recursive(child, current_path.clone(), result);
        }
    }

    let mut result = Vec::new();
    for node in &toc.structure {
        collect_recursive(node, Vec::new(), &mut result);
    }
    result
}

/// Compares two markdown documents and returns detailed change analysis.
pub fn compute_delta(original: &Markdown, updated: &Markdown) -> MarkdownDelta {
    let original_toc = original.toc();
    let updated_toc = updated.toc();

    let mut delta = MarkdownDelta::new();

    // Set byte statistics
    delta.statistics.original_bytes = original.content().len();
    delta.statistics.new_bytes = updated.content().len();

    // Compare frontmatter
    compare_frontmatter(original, updated, &mut delta);

    // Compare preamble
    compare_preamble(&original_toc, &updated_toc, &mut delta);

    // Compare sections
    compare_sections(&original_toc, &updated_toc, &mut delta);

    // Compare code blocks
    compare_code_blocks(&original_toc, &updated_toc, &mut delta);

    // Detect broken links
    detect_broken_links(&original_toc, &updated_toc, &mut delta);

    // Calculate final statistics
    calculate_statistics(&mut delta);

    // Determine classification
    delta.classification = DocumentChange::from_statistics(&delta.statistics);

    delta
}

/// Compares frontmatter between two documents.
fn compare_frontmatter(original: &Markdown, updated: &Markdown, delta: &mut MarkdownDelta) {
    let original_fm = original.frontmatter().as_map();
    let updated_fm = updated.frontmatter().as_map();

    let original_keys: HashSet<_> = original_fm.keys().collect();
    let updated_keys: HashSet<_> = updated_fm.keys().collect();

    // Check for added keys
    for key in updated_keys.difference(&original_keys) {
        if let Some(value) = updated_fm.get(*key) {
            delta
                .frontmatter_changes
                .push(FrontmatterChange::added((*key).clone(), value.clone()));
        }
    }

    // Check for removed keys
    for key in original_keys.difference(&updated_keys) {
        if let Some(value) = original_fm.get(*key) {
            delta
                .frontmatter_changes
                .push(FrontmatterChange::removed((*key).clone(), value.clone()));
        }
    }

    // Check for modified keys
    for key in original_keys.intersection(&updated_keys) {
        let original_value = original_fm.get(*key);
        let updated_value = updated_fm.get(*key);

        if original_value != updated_value {
            if let (Some(ov), Some(uv)) = (original_value, updated_value) {
                delta
                    .frontmatter_changes
                    .push(FrontmatterChange::updated((*key).clone(), ov.clone(), uv.clone()));
            }
        }
    }

    delta.frontmatter_changed = !delta.frontmatter_changes.is_empty();

    // Check if changes are formatting-only (same normalized hash)
    if delta.frontmatter_changed {
        let original_toc = original.toc();
        let updated_toc = updated.toc();
        delta.frontmatter_formatting_only =
            original_toc.frontmatter_hash_normalized == updated_toc.frontmatter_hash_normalized;
    }
}

/// Compares preamble (content before first heading).
fn compare_preamble(original_toc: &MarkdownToc, updated_toc: &MarkdownToc, delta: &mut MarkdownDelta) {
    delta.preamble_changed = original_toc.preamble_hash != updated_toc.preamble_hash;

    if delta.preamble_changed {
        delta.preamble_whitespace_only =
            original_toc.preamble_hash_trimmed == updated_toc.preamble_hash_trimmed;
    }
}

/// Compares sections between two documents.
fn compare_sections(original_toc: &MarkdownToc, updated_toc: &MarkdownToc, delta: &mut MarkdownDelta) {
    let original_headings = extract_headings_with_paths(original_toc);
    let updated_headings = extract_headings_with_paths(updated_toc);

    delta.statistics.original_section_count = original_headings.len();
    delta.statistics.new_section_count = updated_headings.len();

    // Build indexes for matching
    let mut original_by_content: HashMap<u64, Vec<(Vec<String>, &MarkdownTocNode)>> = HashMap::new();
    for (path, node) in &original_headings {
        original_by_content
            .entry(node.own_content_hash)
            .or_default()
            .push((path.clone(), *node));
    }

    let mut updated_by_content: HashMap<u64, Vec<(Vec<String>, &MarkdownTocNode)>> = HashMap::new();
    for (path, node) in &updated_headings {
        updated_by_content
            .entry(node.own_content_hash)
            .or_default()
            .push((path.clone(), *node));
    }

    // Track matched sections
    let mut matched_original: HashSet<usize> = HashSet::new();
    let mut matched_updated: HashSet<usize> = HashSet::new();

    // First pass: exact matches (same path and content)
    for (i, (orig_path, orig_node)) in original_headings.iter().enumerate() {
        for (j, (upd_path, upd_node)) in updated_headings.iter().enumerate() {
            if matched_updated.contains(&j) {
                continue;
            }

            if orig_path == upd_path && orig_node.own_content_hash == upd_node.own_content_hash {
                // Perfect match - unchanged
                matched_original.insert(i);
                matched_updated.insert(j);
                delta.statistics.sections_unchanged += 1;
                break;
            }
        }
    }

    // Second pass: content matches with different paths (moves)
    for (i, (orig_path, orig_node)) in original_headings.iter().enumerate() {
        if matched_original.contains(&i) {
            continue;
        }

        for (j, (upd_path, upd_node)) in updated_headings.iter().enumerate() {
            if matched_updated.contains(&j) {
                continue;
            }

            if orig_node.own_content_hash == upd_node.own_content_hash
                && orig_node.own_content_hash != 0
            {
                // Same content, different path = moved
                let level_delta = upd_node.level as i8 - orig_node.level as i8;

                delta.moved.push(MovedSection::new(
                    orig_node.own_content_hash,
                    orig_path.clone(),
                    upd_path.clone(),
                    level_delta,
                    i,
                    j,
                ));

                matched_original.insert(i);
                matched_updated.insert(j);
                delta.statistics.sections_moved += 1;
                break;
            }
        }
    }

    // Third pass: same path, different content (modifications)
    for (i, (orig_path, orig_node)) in original_headings.iter().enumerate() {
        if matched_original.contains(&i) {
            continue;
        }

        for (j, (upd_path, upd_node)) in updated_headings.iter().enumerate() {
            if matched_updated.contains(&j) {
                continue;
            }

            if orig_path == upd_path {
                // Same path, different content = modified
                // Compare alphanumeric content only to detect formatting-only changes
                // This catches: whitespace, table padding, separator dashes, emphasis markers
                // Also strip code fence lines since code blocks are compared separately
                let orig_content = orig_node.own_content.as_deref().unwrap_or("");
                let upd_content = upd_node.own_content.as_deref().unwrap_or("");

                let orig_content_hash =
                    crate::hashing::xx_hash_alphanumeric(&strip_code_fences(orig_content));
                let upd_content_hash =
                    crate::hashing::xx_hash_alphanumeric(&strip_code_fences(upd_content));
                let is_whitespace_only = orig_content_hash == upd_content_hash;

                let action = if is_whitespace_only {
                    ChangeAction::WhitespaceOnly
                } else {
                    ChangeAction::ContentModified
                };
                let edit_distance = levenshtein_distance(orig_content, upd_content);

                // Generate a meaningful description
                let description = if is_whitespace_only {
                    describe_whitespace_change(orig_content, upd_content)
                } else {
                    describe_content_change(orig_content, upd_content, &orig_node.title)
                };

                delta.modified.push(ContentChange::new(
                    action,
                    Some(orig_path.clone()),
                    Some(upd_path.clone()),
                    Some(orig_node.level),
                    Some(upd_node.level),
                    Some(orig_node.line_range.0),
                    Some(upd_node.line_range.0),
                    description,
                ));

                // Track byte-level changes (only for non-whitespace changes)
                if !is_whitespace_only {
                    delta.statistics.bytes_modified += edit_distance;
                }

                matched_original.insert(i);
                matched_updated.insert(j);
                delta.statistics.sections_modified += 1;

                if is_whitespace_only {
                    delta.statistics.whitespace_only_changes += 1;
                } else {
                    delta.statistics.content_only_changes += 1;
                }
                break;
            }
        }
    }

    // Remaining unmatched original = removed
    for (i, (path, node)) in original_headings.iter().enumerate() {
        if !matched_original.contains(&i) {
            let content_len = node.own_content.as_ref().map(|c| c.len()).unwrap_or(0);
            delta.statistics.bytes_removed += content_len;

            delta.removed.push(ContentChange::removed(
                path.clone(),
                node.level,
                node.line_range.0,
                &node.title,
            ));
            delta.statistics.sections_removed += 1;
        }
    }

    // Remaining unmatched updated = added
    for (j, (path, node)) in updated_headings.iter().enumerate() {
        if !matched_updated.contains(&j) {
            let content_len = node.own_content.as_ref().map(|c| c.len()).unwrap_or(0);
            delta.statistics.bytes_added += content_len;

            delta.added.push(ContentChange::added(
                path.clone(),
                node.level,
                node.line_range.0,
                &node.title,
            ));
            delta.statistics.sections_added += 1;
        }
    }
}

/// Describes changes between two code block info strings.
///
/// Parses the info strings to identify specific property changes rather than
/// showing the full strings.
fn describe_info_string_change(old_info: &str, new_info: &str) -> String {
    let (old_lang, old_props) = parse_info_string(old_info);
    let (new_lang, new_props) = parse_info_string(new_info);

    let lang = new_lang.unwrap_or("plain text");

    // Check if language changed
    if old_lang != new_lang {
        let old = old_lang.unwrap_or("none");
        let new = new_lang.unwrap_or("none");
        return format!("Language: {} → {}", old, new);
    }

    // Find changed properties
    let mut changes = Vec::new();

    // Check for modified or removed properties
    for (key, old_val) in &old_props {
        match new_props.get(key) {
            Some(new_val) if new_val != old_val => {
                changes.push(format!("'{}': \"{}\" → \"{}\"", key, old_val, new_val));
            }
            None => {
                changes.push(format!("'{}' removed", key));
            }
            _ => {}
        }
    }

    // Check for added properties
    for (key, new_val) in &new_props {
        if !old_props.contains_key(key) {
            changes.push(format!("'{}' added: \"{}\"", key, new_val));
        }
    }

    if changes.is_empty() {
        // Fallback if we couldn't parse the difference
        format!("Info changed")
    } else if changes.len() == 1 {
        format!("{} ({})", lang, changes[0])
    } else {
        format!("{} ({} properties changed)", lang, changes.len())
    }
}

/// Parses a code block info string into language and properties.
///
/// Format: `language key="value" key2="value2"` or `language key=value`
fn parse_info_string(info: &str) -> (Option<&str>, HashMap<String, String>) {
    let mut props = HashMap::new();
    let info = info.trim();

    if info.is_empty() {
        return (None, props);
    }

    let mut parts = info.splitn(2, char::is_whitespace);
    let lang = parts.next().filter(|s| !s.is_empty());
    let rest = parts.next().unwrap_or("");

    // Parse key="value" or key=value patterns
    let mut chars = rest.chars().peekable();
    while chars.peek().is_some() {
        // Skip whitespace
        while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
            chars.next();
        }

        // Read key
        let key: String = chars
            .by_ref()
            .take_while(|&c| c != '=' && !c.is_whitespace())
            .collect();

        if key.is_empty() {
            break;
        }

        // Check for =
        if chars.peek() == Some(&'=') {
            chars.next(); // consume '='

            // Read value (possibly quoted)
            let value = if chars.peek() == Some(&'"') {
                chars.next(); // consume opening quote
                let v: String = chars.by_ref().take_while(|&c| c != '"').collect();
                v
            } else {
                chars
                    .by_ref()
                    .take_while(|&c| !c.is_whitespace())
                    .collect()
            };

            props.insert(key, value);
        }
    }

    (lang, props)
}

/// Strips code fence lines from content for comparison.
///
/// Removes lines that start with ``` (code block delimiters) since
/// code blocks are compared separately. This prevents language tag
/// changes from being double-counted as section content changes.
fn strip_code_fences(content: &str) -> String {
    content
        .lines()
        .filter(|line| !line.trim_start().starts_with("```"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Describes whitespace changes between two content strings.
fn describe_whitespace_change(original: &str, updated: &str) -> String {
    let orig_lines: Vec<&str> = original.lines().collect();
    let upd_lines: Vec<&str> = updated.lines().collect();

    let orig_blank = orig_lines.iter().filter(|l| l.trim().is_empty()).count();
    let upd_blank = upd_lines.iter().filter(|l| l.trim().is_empty()).count();

    if orig_blank != upd_blank {
        let diff = upd_blank as i32 - orig_blank as i32;
        if diff > 0 {
            return format!("Added {} blank line{}", diff, if diff > 1 { "s" } else { "" });
        } else {
            return format!(
                "Removed {} blank line{}",
                -diff,
                if -diff > 1 { "s" } else { "" }
            );
        }
    }

    // Check for trailing whitespace changes
    let orig_trailing: usize = orig_lines
        .iter()
        .map(|l| l.len() - l.trim_end().len())
        .sum();
    let upd_trailing: usize = upd_lines
        .iter()
        .map(|l| l.len() - l.trim_end().len())
        .sum();

    if orig_trailing != upd_trailing {
        return "Trailing whitespace changes".to_string();
    }

    // Check for indentation changes
    let orig_indent: usize = orig_lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .sum();
    let upd_indent: usize = upd_lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .sum();

    if orig_indent != upd_indent {
        return "Indentation changes".to_string();
    }

    "Whitespace formatting changes".to_string()
}

/// Describes content changes between two strings.
fn describe_content_change(original: &str, updated: &str, title: &str) -> String {
    let orig_len = original.len();
    let upd_len = updated.len();
    let char_diff = upd_len as i32 - orig_len as i32;

    // Count non-blank lines (actual content lines)
    let orig_content_lines = original.lines().filter(|l| !l.trim().is_empty()).count();
    let upd_content_lines = updated.lines().filter(|l| !l.trim().is_empty()).count();
    let content_line_diff = upd_content_lines as i32 - orig_content_lines as i32;

    // Describe based on what changed
    if content_line_diff != 0 {
        // Content lines added or removed
        let sign = if content_line_diff > 0 { "+" } else { "" };
        format!(
            "{}: {}{} content line{}",
            title,
            sign,
            content_line_diff,
            if content_line_diff.abs() > 1 { "s" } else { "" }
        )
    } else if char_diff != 0 {
        // Same content line count, but characters changed (reformatting, word changes)
        let sign = if char_diff > 0 { "+" } else { "" };
        format!("{}: {}{} chars", title, sign, char_diff)
    } else {
        // Same size but different content (word substitutions)
        format!("{}: text edited", title)
    }
}

/// Compares code blocks between two documents.
///
/// Matches code blocks by parent section path and position, then compares
/// both language and content to report accurate changes.
fn compare_code_blocks(
    original_toc: &MarkdownToc,
    updated_toc: &MarkdownToc,
    delta: &mut MarkdownDelta,
) {
    // Group code blocks by parent section path (ignoring language for matching)
    let mut original_by_section: HashMap<Vec<String>, Vec<&crate::markdown::toc::CodeBlockInfo>> =
        HashMap::new();
    for cb in &original_toc.code_blocks {
        original_by_section
            .entry(cb.parent_section_path.clone())
            .or_default()
            .push(cb);
    }

    let mut updated_by_section: HashMap<Vec<String>, Vec<&crate::markdown::toc::CodeBlockInfo>> =
        HashMap::new();
    for cb in &updated_toc.code_blocks {
        updated_by_section
            .entry(cb.parent_section_path.clone())
            .or_default()
            .push(cb);
    }

    // Track which blocks have been matched
    let mut matched_original: HashSet<(Vec<String>, usize)> = HashSet::new();
    let mut matched_updated: HashSet<(Vec<String>, usize)> = HashSet::new();

    // Match blocks by section and position
    for (section, orig_blocks) in &original_by_section {
        if let Some(upd_blocks) = updated_by_section.get(section) {
            // Match blocks in order within each section
            for (i, orig_cb) in orig_blocks.iter().enumerate() {
                if let Some(upd_cb) = upd_blocks.get(i) {
                    matched_original.insert((section.clone(), i));
                    matched_updated.insert((section.clone(), i));

                    // Compare info string (includes language + metadata) and content
                    let info_changed = orig_cb.info_string != upd_cb.info_string;
                    let content_changed =
                        orig_cb.content_hash_trimmed != upd_cb.content_hash_trimmed;

                    if info_changed || content_changed {
                        let description = if content_changed && info_changed {
                            // Both changed
                            format!(
                                "Modified code block ({})",
                                upd_cb.language.as_deref().unwrap_or("plain text")
                            )
                        } else if content_changed {
                            // Only content changed
                            format!(
                                "Modified {} code block",
                                orig_cb.language.as_deref().unwrap_or("plain text")
                            )
                        } else {
                            // Only info string changed - provide detailed diff
                            describe_info_string_change(
                                &orig_cb.info_string,
                                &upd_cb.info_string,
                            )
                        };

                        let action = if content_changed {
                            ChangeAction::ContentModified
                        } else {
                            ChangeAction::Renamed // Info string change only
                        };

                        delta.code_block_changes.push(CodeBlockChange::new(
                            action,
                            upd_cb.language.clone(),
                            orig_cb.parent_section_path.clone(),
                            Some(orig_cb.line_range.0),
                            Some(upd_cb.line_range.0),
                            description,
                        ));

                        if content_changed {
                            delta.statistics.code_blocks_modified += 1;
                            // Track byte-level changes for content modifications
                            let edit_dist =
                                levenshtein_distance(&orig_cb.content, &upd_cb.content);
                            delta.statistics.bytes_modified += edit_dist;
                        }
                        if info_changed && !content_changed {
                            delta.statistics.code_blocks_language_changed += 1;
                            // Info string change counts as a modification
                            let edit_dist = levenshtein_distance(
                                &orig_cb.info_string,
                                &upd_cb.info_string,
                            );
                            delta.statistics.bytes_modified += edit_dist.max(1);
                        }
                    }
                    // else: completely unchanged, don't report
                }
            }
        }
    }

    // Find unmatched original blocks (removed)
    for (section, blocks) in &original_by_section {
        for (i, cb) in blocks.iter().enumerate() {
            if !matched_original.contains(&(section.clone(), i)) {
                delta.code_block_changes.push(CodeBlockChange::new(
                    ChangeAction::Removed,
                    cb.language.clone(),
                    cb.parent_section_path.clone(),
                    Some(cb.line_range.0),
                    None,
                    format!(
                        "Removed {} code block",
                        cb.language.as_deref().unwrap_or("plain text")
                    ),
                ));
                delta.statistics.code_blocks_removed += 1;
                delta.statistics.bytes_removed += cb.content.len();
            }
        }
    }

    // Find unmatched updated blocks (added)
    for (section, blocks) in &updated_by_section {
        for (i, cb) in blocks.iter().enumerate() {
            if !matched_updated.contains(&(section.clone(), i)) {
                delta.code_block_changes.push(CodeBlockChange::new(
                    ChangeAction::Added,
                    cb.language.clone(),
                    cb.parent_section_path.clone(),
                    None,
                    Some(cb.line_range.0),
                    format!(
                        "Added {} code block",
                        cb.language.as_deref().unwrap_or("plain text")
                    ),
                ));
                delta.statistics.code_blocks_added += 1;
                delta.statistics.bytes_added += cb.content.len();
            }
        }
    }
}

/// Detects internal links that would break due to heading changes.
fn detect_broken_links(
    _original_toc: &MarkdownToc,
    updated_toc: &MarkdownToc,
    delta: &mut MarkdownDelta,
) {
    // Check each internal link in the updated document
    for link in &updated_toc.internal_links {
        if !updated_toc.slug_index.contains_key(&link.target_slug) {
            let mut broken = BrokenLink::new(
                link.target_slug.clone(),
                link.link_text.clone(),
                link.line_number,
            );

            // Try to find a similar slug (simple Levenshtein-based suggestion)
            if let Some((suggested, confidence)) =
                find_similar_slug(&link.target_slug, &updated_toc.slug_index)
            {
                broken = broken.with_suggestion(suggested, confidence);
            }

            delta.broken_links.push(broken);
        }
    }

    delta.statistics.broken_links_count = delta.broken_links.len();
}

/// Finds a similar slug in the index (simple similarity matching).
fn find_similar_slug(
    target: &str,
    slug_index: &HashMap<String, Vec<(Vec<String>, usize)>>,
) -> Option<(String, f32)> {
    let mut best_match: Option<(String, f32)> = None;

    for slug in slug_index.keys() {
        let distance = levenshtein_distance(target, slug);
        let max_len = target.len().max(slug.len());
        let similarity = 1.0 - (distance as f32 / max_len as f32);

        if similarity > 0.5 {
            if best_match.is_none() || similarity > best_match.as_ref().unwrap().1 {
                best_match = Some((slug.clone(), similarity));
            }
        }
    }

    best_match
}

/// Simple Levenshtein distance calculation.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut dp = vec![vec![0; n + 1]; m + 1];

    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[m][n]
}

/// Calculates final statistics.
fn calculate_statistics(delta: &mut MarkdownDelta) {
    let stats = &mut delta.statistics;

    // Calculate net bytes changed
    stats.bytes_changed =
        (stats.original_bytes as i64 - stats.new_bytes as i64).unsigned_abs() as usize;

    // Calculate content change ratio based on actual byte changes
    // This is more accurate than counting sections as binary changed/unchanged
    let total_bytes = stats.original_bytes.max(stats.new_bytes);
    if total_bytes > 0 {
        let total_change = stats.bytes_added + stats.bytes_removed + stats.bytes_modified;
        stats.content_change_ratio = total_change as f32 / total_bytes as f32;
        // Cap at 1.0 (can exceed if there are many edits)
        stats.content_change_ratio = stats.content_change_ratio.min(1.0);
    }

    // Calculate structural changes
    stats.structural_changes = stats.sections_moved + delta.moved.len();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_identical_documents() {
        let original: Markdown = "# Hello\n\nWorld".into();
        let updated: Markdown = "# Hello\n\nWorld".into();

        let delta = compute_delta(&original, &updated);
        assert!(delta.is_unchanged());
        assert_eq!(delta.statistics.sections_unchanged, 1);
    }

    #[test]
    fn test_delta_content_modified() {
        let original: Markdown = "# Hello\n\nWorld".into();
        let updated: Markdown = "# Hello\n\nUniverse".into();

        let delta = compute_delta(&original, &updated);
        assert!(!delta.is_unchanged());
        assert_eq!(delta.modified.len(), 1);
    }

    #[test]
    fn test_delta_section_added() {
        let original: Markdown = "# Hello".into();
        let updated: Markdown = "# Hello\n\n## New Section\n\nContent".into();

        let delta = compute_delta(&original, &updated);
        assert_eq!(delta.added.len(), 1);
        assert_eq!(delta.statistics.sections_added, 1);
    }

    #[test]
    fn test_delta_section_removed() {
        let original: Markdown = "# Hello\n\n## Old Section\n\nContent".into();
        let updated: Markdown = "# Hello".into();

        let delta = compute_delta(&original, &updated);
        assert_eq!(delta.removed.len(), 1);
        assert_eq!(delta.statistics.sections_removed, 1);
    }

    #[test]
    fn test_delta_frontmatter_changed() {
        let original: Markdown = "---\ntitle: Old\n---\n# Hello".into();
        let updated: Markdown = "---\ntitle: New\n---\n# Hello".into();

        let delta = compute_delta(&original, &updated);
        assert!(delta.frontmatter_changed);
        assert_eq!(delta.frontmatter_changes.len(), 1);
    }

    #[test]
    fn test_delta_frontmatter_added() {
        let original: Markdown = "---\ntitle: Test\n---\n# Hello".into();
        let updated: Markdown = "---\ntitle: Test\nauthor: Alice\n---\n# Hello".into();

        let delta = compute_delta(&original, &updated);
        assert!(delta.frontmatter_changed);

        let added_change = delta
            .frontmatter_changes
            .iter()
            .find(|c| matches!(c.action, ChangeAction::PropertyAdded));
        assert!(added_change.is_some());
    }

    #[test]
    fn test_delta_preamble_changed() {
        let original: Markdown = "Some intro\n\n# Hello".into();
        let updated: Markdown = "Different intro\n\n# Hello".into();

        let delta = compute_delta(&original, &updated);
        assert!(delta.preamble_changed);
    }

    #[test]
    fn test_delta_broken_links() {
        let original: Markdown = "# Hello\n\n## Section\n\nSee [link](#section)".into();
        let updated: Markdown = "# Hello\n\n## Renamed\n\nSee [link](#section)".into();

        let delta = compute_delta(&original, &updated);
        assert!(delta.has_broken_links());
        assert_eq!(delta.broken_links.len(), 1);
    }

    #[test]
    fn test_delta_summary() {
        let original: Markdown = "# Hello".into();
        let updated: Markdown = "# Hello\n\n## New".into();

        let delta = compute_delta(&original, &updated);
        let summary = delta.summary();
        assert!(summary.contains("1 added"));
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
        assert_eq!(levenshtein_distance("hello", "hallo"), 1);
        assert_eq!(levenshtein_distance("hello", ""), 5);
        assert_eq!(levenshtein_distance("", "hello"), 5);
    }
}
