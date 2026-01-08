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
                let is_whitespace_only =
                    orig_node.own_content_hash_trimmed == upd_node.own_content_hash_trimmed;

                let action = if is_whitespace_only {
                    ChangeAction::WhitespaceOnly
                } else {
                    ChangeAction::ContentModified
                };

                delta.modified.push(ContentChange::new(
                    action,
                    Some(orig_path.clone()),
                    Some(upd_path.clone()),
                    Some(orig_node.level),
                    Some(upd_node.level),
                    Some(orig_node.line_range.0),
                    Some(upd_node.line_range.0),
                    format!(
                        "{} section '{}'",
                        if is_whitespace_only {
                            "Whitespace changes in"
                        } else {
                            "Modified content in"
                        },
                        orig_node.title
                    ),
                ));

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

/// Compares code blocks between two documents.
fn compare_code_blocks(
    original_toc: &MarkdownToc,
    updated_toc: &MarkdownToc,
    delta: &mut MarkdownDelta,
) {
    let mut original_by_hash: HashMap<u64, &crate::markdown::toc::CodeBlockInfo> = HashMap::new();
    for cb in &original_toc.code_blocks {
        original_by_hash.insert(cb.content_hash, cb);
    }

    let mut updated_by_hash: HashMap<u64, &crate::markdown::toc::CodeBlockInfo> = HashMap::new();
    for cb in &updated_toc.code_blocks {
        updated_by_hash.insert(cb.content_hash, cb);
    }

    // Find added code blocks
    for cb in &updated_toc.code_blocks {
        if !original_by_hash.contains_key(&cb.content_hash) {
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
        }
    }

    // Find removed code blocks
    for cb in &original_toc.code_blocks {
        if !updated_by_hash.contains_key(&cb.content_hash) {
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

    // Calculate bytes changed (rough estimate)
    stats.bytes_changed = (stats.original_bytes as i64 - stats.new_bytes as i64).unsigned_abs() as usize;

    // Calculate content change ratio
    let max_bytes = stats.original_bytes.max(stats.new_bytes);
    if max_bytes > 0 {
        // Factor in section changes
        let section_change_impact =
            stats.sections_added + stats.sections_removed + stats.sections_modified;
        let total_sections = stats.original_section_count.max(stats.new_section_count);

        if total_sections > 0 {
            stats.content_change_ratio = section_change_impact as f32 / total_sections as f32;
        }
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
