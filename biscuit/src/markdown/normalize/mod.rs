//! Markdown document structure validation and normalization.
//!
//! This module provides functionality to:
//! - Validate heading structure for well-formedness
//! - Normalize heading levels to a target root level
//! - Re-level documents for embedding as subsections
//! - Correct hierarchy violations
//!
//! ## Examples
//!
//! ```rust
//! use shared::markdown::Markdown;
//! use shared::markdown::normalize::HeadingLevel;
//!
//! // Validate a document's structure
//! let doc: Markdown = "## Intro\n### Details\n## Conclusion".into();
//! let validation = doc.validate_structure();
//!
//! assert!(validation.is_well_formed());
//! assert_eq!(validation.root_level, Some(HeadingLevel::H2));
//! ```

mod types;

pub use types::{
    HeadingAdjustment, HeadingLevel, NormalizationError, NormalizationReport, StructureIssue,
    StructureIssueKind, StructureValidation, ViolationCorrection,
};

use pulldown_cmark::{Event, HeadingLevel as PulldownLevel, Parser, Tag, TagEnd};

/// Information about a heading extracted during parsing.
#[derive(Debug, Clone)]
struct HeadingInfo {
    level: HeadingLevel,
    title: String,
    line_number: usize,
    byte_start: usize,
    byte_end: usize,
}

/// Converts pulldown_cmark HeadingLevel to our HeadingLevel.
fn pulldown_to_heading_level(level: PulldownLevel) -> HeadingLevel {
    match level {
        PulldownLevel::H1 => HeadingLevel::H1,
        PulldownLevel::H2 => HeadingLevel::H2,
        PulldownLevel::H3 => HeadingLevel::H3,
        PulldownLevel::H4 => HeadingLevel::H4,
        PulldownLevel::H5 => HeadingLevel::H5,
        PulldownLevel::H6 => HeadingLevel::H6,
    }
}

/// Extracts heading information from markdown content.
fn extract_headings(content: &str) -> Vec<HeadingInfo> {
    let parser = Parser::new(content);

    let mut headings = Vec::new();
    let mut current_heading: Option<(PulldownLevel, String, usize)> = None;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current_heading = Some((level, String::new(), range.start));
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, ref mut title, _)) = current_heading {
                    title.push_str(&text);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, title, start)) = current_heading.take() {
                    let line_number = content[..start].lines().count() + 1;
                    headings.push(HeadingInfo {
                        level: pulldown_to_heading_level(level),
                        title,
                        line_number,
                        byte_start: start,
                        byte_end: range.end,
                    });
                }
            }
            _ => {}
        }
    }

    headings
}

/// Validates the structure of a markdown document.
pub fn validate_structure(content: &str) -> StructureValidation {
    let headings = extract_headings(content);
    let mut validation = StructureValidation::new();

    if headings.is_empty() {
        validation.add_issue(StructureIssue::new(
            StructureIssueKind::NoHeadings,
            String::new(),
            0,
            "Document has no headings".to_string(),
        ));
        return validation;
    }

    validation.heading_count = headings.len();
    validation.root_level = Some(headings[0].level);

    // Find min and max levels
    let mut min_level = headings[0].level;
    let mut max_level = headings[0].level;

    for heading in &headings {
        if heading.level < min_level {
            min_level = heading.level;
        }
        if heading.level > max_level {
            max_level = heading.level;
        }
    }

    validation.min_level = Some(min_level);
    validation.max_level = Some(max_level);

    let root_level = headings[0].level;

    // Check for hierarchy violations
    for heading in &headings[1..] {
        if heading.level < root_level {
            validation.add_issue(
                StructureIssue::new(
                    StructureIssueKind::HierarchyViolation,
                    heading.title.clone(),
                    heading.line_number,
                    format!(
                        "Heading '{}' at {} is shallower than document root level {}",
                        heading.title, heading.level, root_level
                    ),
                )
                .with_suggestion(format!("Change to {} or deeper", root_level)),
            );
        }
    }

    // Check for multiple H1s
    let h1_count = headings
        .iter()
        .filter(|h| h.level == HeadingLevel::H1)
        .count();
    if h1_count > 1 {
        validation.add_issue(StructureIssue::new(
            StructureIssueKind::MultipleH1,
            String::new(),
            0,
            format!("Document has {} H1 headings (expected at most 1)", h1_count),
        ));
    }

    // Check for skipped levels
    let mut prev_level = root_level;
    for heading in &headings[1..] {
        let level_jump = heading.level.as_u8() as i8 - prev_level.as_u8() as i8;
        if level_jump > 1 {
            validation.add_issue(
                StructureIssue::new(
                    StructureIssueKind::SkippedLevel,
                    heading.title.clone(),
                    heading.line_number,
                    format!(
                        "Heading '{}' skips from {} to {} (skipped {} level(s))",
                        heading.title,
                        prev_level,
                        heading.level,
                        level_jump - 1
                    ),
                )
                .with_suggestion(format!(
                    "Add intermediate heading(s) at {}",
                    HeadingLevel::new(prev_level.as_u8() + 1)
                        .map(|l| l.to_string())
                        .unwrap_or_default()
                )),
            );
        }
        prev_level = heading.level;
    }

    validation
}

/// Normalizes heading levels in markdown content.
pub fn normalize(
    content: &str,
    target: Option<HeadingLevel>,
) -> Result<(String, NormalizationReport), NormalizationError> {
    let headings = extract_headings(content);

    if headings.is_empty() {
        return Err(NormalizationError::NoHeadings);
    }

    let root_level = headings[0].level;
    let target_level = target.unwrap_or(root_level);

    // Calculate the level adjustment
    let level_adjustment = target_level.as_u8() as i8 - root_level.as_u8() as i8;

    // Check for overflow
    let max_level = headings.iter().map(|h| h.level).max().unwrap();
    let depth = max_level.as_u8() - root_level.as_u8();
    if target_level.as_u8() + depth > 6 {
        let deepest_heading = headings.iter().find(|h| h.level == max_level).unwrap();
        return Err(NormalizationError::LevelOverflow {
            target: target_level,
            affected_count: headings
                .iter()
                .filter(|h| h.level.as_u8() as i8 + level_adjustment > 6)
                .count(),
            deepest_title: deepest_heading.title.clone(),
            would_become: target_level.as_u8() + depth,
        });
    }

    let mut report = NormalizationReport::new(Some(root_level), target_level, level_adjustment);

    // If no adjustment needed
    if level_adjustment == 0 {
        return Ok((content.to_string(), report));
    }

    // Collect all replacements (sorted by position descending to avoid offset issues)
    let mut replacements: Vec<(usize, usize, HeadingLevel, HeadingLevel)> = Vec::new();

    for heading in &headings {
        let new_level_raw = heading.level.as_u8() as i8 + level_adjustment;
        if let Some(new_level) = HeadingLevel::new(new_level_raw as u8) {
            replacements.push((
                heading.byte_start,
                heading.byte_end,
                heading.level,
                new_level,
            ));
            report.add_adjustment(HeadingAdjustment::new(
                heading.title.clone(),
                heading.line_number,
                heading.level,
                new_level,
            ));
        }
    }

    // Sort by position descending
    replacements.sort_by(|a, b| b.0.cmp(&a.0));

    // Apply replacements
    let mut result = content.to_string();

    for (start, _end, old_level, new_level) in replacements {
        // Find the end of the # sequence
        let prefix_end = start + old_level.hash_count();
        let new_prefix = "#".repeat(new_level.hash_count());

        result = format!(
            "{}{}{}",
            &result[..start],
            new_prefix,
            &result[prefix_end..]
        );
    }

    Ok((result, report))
}

/// Re-levels a document to a specific target level (simple uniform shift).
pub fn relevel(content: &str, target: HeadingLevel) -> Result<(String, i8), NormalizationError> {
    let (result, report) = normalize(content, Some(target))?;
    Ok((result, report.level_adjustment))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_headings() {
        let content = "# Hello\n\n## World\n\n### Nested";
        let headings = extract_headings(content);

        assert_eq!(headings.len(), 3);
        assert_eq!(headings[0].level, HeadingLevel::H1);
        assert_eq!(headings[0].title, "Hello");
        assert_eq!(headings[1].level, HeadingLevel::H2);
        assert_eq!(headings[2].level, HeadingLevel::H3);
    }

    #[test]
    fn test_validate_structure_well_formed() {
        let content = "## Intro\n\n### Details\n\n## Conclusion";
        let validation = validate_structure(content);

        assert!(validation.is_well_formed());
        assert_eq!(validation.root_level, Some(HeadingLevel::H2));
        assert_eq!(validation.heading_count, 3);
    }

    #[test]
    fn test_validate_structure_hierarchy_violation() {
        let content = "### Start\n\n## Violation";
        let validation = validate_structure(content);

        assert!(!validation.is_well_formed());
        let violations = validation.issues_of_kind(StructureIssueKind::HierarchyViolation);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_validate_structure_multiple_h1() {
        let content = "# First\n\n# Second";
        let validation = validate_structure(content);

        let issues = validation.issues_of_kind(StructureIssueKind::MultipleH1);
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn test_validate_structure_skipped_level() {
        let content = "## Start\n\n#### Skipped";
        let validation = validate_structure(content);

        let issues = validation.issues_of_kind(StructureIssueKind::SkippedLevel);
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn test_validate_structure_no_headings() {
        let content = "Just some text without headings.";
        let validation = validate_structure(content);

        assert!(!validation.is_well_formed());
        let issues = validation.issues_of_kind(StructureIssueKind::NoHeadings);
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn test_validate_can_relevel() {
        let content = "## Root\n\n### Child\n\n#### Deep";
        let validation = validate_structure(content);

        // Can relevel to H1 (max would become H3)
        assert!(validation.can_relevel_to(HeadingLevel::H1));

        // Can relevel to H4 (max would become H6)
        assert!(validation.can_relevel_to(HeadingLevel::H4));

        // Cannot relevel to H5 (max would become H7)
        assert!(!validation.can_relevel_to(HeadingLevel::H5));
    }

    #[test]
    fn test_normalize_promotion() {
        let content = "### Intro\n\n#### Details";
        let (result, report) = normalize(content, Some(HeadingLevel::H1)).unwrap();

        assert!(result.starts_with("# Intro"));
        assert!(result.contains("## Details"));
        assert_eq!(report.level_adjustment, -2);
        assert_eq!(report.headings_adjusted, 2);
    }

    #[test]
    fn test_normalize_demotion() {
        let content = "# Root\n\n## Child";
        let (result, report) = normalize(content, Some(HeadingLevel::H3)).unwrap();

        assert!(result.starts_with("### Root"));
        assert!(result.contains("#### Child"));
        assert_eq!(report.level_adjustment, 2);
    }

    #[test]
    fn test_normalize_no_change() {
        let content = "## Root\n\n### Child";
        let (result, report) = normalize(content, Some(HeadingLevel::H2)).unwrap();

        assert_eq!(result, content);
        assert_eq!(report.level_adjustment, 0);
        assert!(!report.has_changes());
    }

    #[test]
    fn test_normalize_overflow_error() {
        // H4 with depth 2 (H4->H5->H6), target H5 would make deepest H7
        let content = "#### Level 4\n\n##### Level 5\n\n###### Level 6";
        let result = normalize(content, Some(HeadingLevel::H5));

        assert!(matches!(
            result,
            Err(NormalizationError::LevelOverflow { .. })
        ));
    }

    #[test]
    fn test_normalize_no_headings() {
        let content = "No headings here";
        let result = normalize(content, Some(HeadingLevel::H1));

        assert!(matches!(result, Err(NormalizationError::NoHeadings)));
    }

    #[test]
    fn test_relevel() {
        let content = "## Start\n\n### Details";
        let (result, adjustment) = relevel(content, HeadingLevel::H1).unwrap();

        assert!(result.starts_with("# Start"));
        assert_eq!(adjustment, -1);
    }

    #[test]
    fn test_normalize_preserves_content() {
        let content = "### Title with **bold** and `code`\n\nParagraph text.\n\n#### Subsection\n\nMore content.";
        let (result, _) = normalize(content, Some(HeadingLevel::H1)).unwrap();

        assert!(result.contains("**bold**"));
        assert!(result.contains("`code`"));
        assert!(result.contains("Paragraph text."));
        assert!(result.contains("More content."));
    }

    #[test]
    fn test_normalize_report_summary() {
        let content = "### A\n\n### B\n\n### C";
        let (_, report) = normalize(content, Some(HeadingLevel::H1)).unwrap();

        let summary = report.summary();
        assert!(summary.contains("3 headings"));
        assert!(summary.contains("promoted"));
        assert!(summary.contains("2 level(s)"));
    }
}
