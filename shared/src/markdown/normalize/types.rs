//! Type definitions for Markdown normalization and re-leveling.

use thiserror::Error;

/// Represents a valid markdown heading level (1-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HeadingLevel(u8);

impl HeadingLevel {
    /// H1 - Top level heading
    pub const H1: Self = Self(1);
    /// H2 - Second level heading
    pub const H2: Self = Self(2);
    /// H3 - Third level heading
    pub const H3: Self = Self(3);
    /// H4 - Fourth level heading
    pub const H4: Self = Self(4);
    /// H5 - Fifth level heading
    pub const H5: Self = Self(5);
    /// H6 - Sixth level heading (deepest)
    pub const H6: Self = Self(6);

    /// Creates a new heading level from a u8 value.
    ///
    /// ## Errors
    ///
    /// Returns `None` if the value is not in the range 1-6.
    pub fn new(level: u8) -> Option<Self> {
        if (1..=6).contains(&level) {
            Some(Self(level))
        } else {
            None
        }
    }

    /// Returns the numeric value (1-6).
    pub fn as_u8(&self) -> u8 {
        self.0
    }

    /// Returns the number of `#` characters for this level.
    pub fn hash_count(&self) -> usize {
        self.0 as usize
    }

    /// Returns the next deeper level, or None if already at H6.
    pub fn deeper(&self) -> Option<Self> {
        Self::new(self.0 + 1)
    }

    /// Returns the next shallower level, or None if already at H1.
    pub fn shallower(&self) -> Option<Self> {
        Self::new(self.0.saturating_sub(1))
    }

    /// Calculates the delta to another level.
    /// Positive = other is deeper, Negative = other is shallower.
    pub fn delta_to(&self, other: Self) -> i8 {
        other.0 as i8 - self.0 as i8
    }
}

impl std::fmt::Display for HeadingLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "H{}", self.0)
    }
}

impl TryFrom<u8> for HeadingLevel {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(())
    }
}

/// Types of structural issues in a markdown document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureIssueKind {
    /// A heading appears at a level shallower than the document's root level.
    /// Example: Document starts with H3, then an H2 appears later.
    HierarchyViolation,

    /// A heading skips levels in the hierarchy.
    /// Example: H2 followed directly by H4 (skipping H3).
    SkippedLevel,

    /// Multiple H1 headings in a document that should have only one.
    MultipleH1,

    /// The document has no headings at all.
    NoHeadings,

    /// A heading would overflow H6 after re-leveling.
    /// Example: Re-leveling an H5 document to H3 would push H6 children to H8.
    LevelOverflow,
}

impl std::fmt::Display for StructureIssueKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HierarchyViolation => write!(f, "hierarchy violation"),
            Self::SkippedLevel => write!(f, "skipped level"),
            Self::MultipleH1 => write!(f, "multiple H1 headings"),
            Self::NoHeadings => write!(f, "no headings"),
            Self::LevelOverflow => write!(f, "level overflow"),
        }
    }
}

/// A structural issue found in a markdown document.
#[derive(Debug, Clone, PartialEq)]
pub struct StructureIssue {
    /// The type of structural issue.
    pub kind: StructureIssueKind,

    /// The heading title where the issue was found.
    pub heading_title: String,

    /// Line number where the issue occurs (1-indexed).
    pub line_number: usize,

    /// Human-readable description of the issue.
    pub description: String,

    /// Suggested fix (if applicable).
    pub suggestion: Option<String>,
}

impl StructureIssue {
    /// Creates a new structure issue.
    pub fn new(
        kind: StructureIssueKind,
        heading_title: String,
        line_number: usize,
        description: String,
    ) -> Self {
        Self {
            kind,
            heading_title,
            line_number,
            description,
            suggestion: None,
        }
    }

    /// Adds a suggestion to this issue.
    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }
}

/// Result of validating a markdown document's structure.
#[derive(Debug, Clone, PartialEq)]
pub struct StructureValidation {
    /// Whether the document structure is valid (no issues found).
    pub is_valid: bool,

    /// The detected root level of the document (level of first heading).
    pub root_level: Option<HeadingLevel>,

    /// The minimum (shallowest) heading level found in the document.
    pub min_level: Option<HeadingLevel>,

    /// The maximum (deepest) heading level found in the document.
    pub max_level: Option<HeadingLevel>,

    /// Total number of headings in the document.
    pub heading_count: usize,

    /// List of structural issues found.
    pub issues: Vec<StructureIssue>,
}

impl Default for StructureValidation {
    fn default() -> Self {
        Self::new()
    }
}

impl StructureValidation {
    /// Creates a new empty validation result.
    pub fn new() -> Self {
        Self {
            is_valid: true,
            root_level: None,
            min_level: None,
            max_level: None,
            heading_count: 0,
            issues: Vec::new(),
        }
    }

    /// Returns true if the document has no structural issues.
    pub fn is_well_formed(&self) -> bool {
        self.issues.is_empty()
    }

    /// Returns true if the document can be safely re-leveled to the target.
    ///
    /// A document can be re-leveled if:
    /// 1. It has at least one heading
    /// 2. Re-leveling won't push any heading beyond H6
    pub fn can_relevel_to(&self, target: HeadingLevel) -> bool {
        let Some(max_level) = self.max_level else {
            return false; // No headings
        };
        let Some(root_level) = self.root_level else {
            return false;
        };

        // Calculate how deep the deepest heading is relative to root
        let depth = max_level.as_u8() - root_level.as_u8();

        // Check if target + depth exceeds H6
        target.as_u8() + depth <= 6
    }

    /// Returns issues of a specific kind.
    pub fn issues_of_kind(&self, kind: StructureIssueKind) -> Vec<&StructureIssue> {
        self.issues.iter().filter(|i| i.kind == kind).collect()
    }

    /// Adds an issue to the validation result.
    pub fn add_issue(&mut self, issue: StructureIssue) {
        self.issues.push(issue);
        self.is_valid = false;
    }
}

/// Details of a single heading level adjustment.
#[derive(Debug, Clone, PartialEq)]
pub struct HeadingAdjustment {
    /// The heading title.
    pub title: String,

    /// Line number (1-indexed).
    pub line_number: usize,

    /// Original heading level.
    pub original_level: HeadingLevel,

    /// New heading level after adjustment.
    pub new_level: HeadingLevel,
}

impl HeadingAdjustment {
    /// Creates a new heading adjustment.
    pub fn new(
        title: String,
        line_number: usize,
        original_level: HeadingLevel,
        new_level: HeadingLevel,
    ) -> Self {
        Self {
            title,
            line_number,
            original_level,
            new_level,
        }
    }
}

/// Details of a hierarchy violation that was corrected.
#[derive(Debug, Clone, PartialEq)]
pub struct ViolationCorrection {
    /// The heading title that violated hierarchy.
    pub title: String,

    /// Line number (1-indexed).
    pub line_number: usize,

    /// The original (violating) level.
    pub original_level: HeadingLevel,

    /// The corrected level.
    pub corrected_level: HeadingLevel,

    /// How child headings were affected.
    pub children_adjusted: usize,

    /// Description of the correction.
    pub description: String,
}

impl ViolationCorrection {
    /// Creates a new violation correction.
    pub fn new(
        title: String,
        line_number: usize,
        original_level: HeadingLevel,
        corrected_level: HeadingLevel,
        children_adjusted: usize,
    ) -> Self {
        let description = format!(
            "Demoted '{}' from {} to {} (root level), adjusted {} children",
            title, original_level, corrected_level, children_adjusted
        );

        Self {
            title,
            line_number,
            original_level,
            corrected_level,
            children_adjusted,
            description,
        }
    }
}

/// Report of changes made during document normalization.
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizationReport {
    /// The original root level before normalization.
    pub original_root_level: Option<HeadingLevel>,

    /// The target root level after normalization.
    pub target_root_level: HeadingLevel,

    /// The level adjustment applied (positive = demoted, negative = promoted).
    pub level_adjustment: i8,

    /// Number of headings that were adjusted.
    pub headings_adjusted: usize,

    /// Details of each heading adjustment.
    pub adjustments: Vec<HeadingAdjustment>,

    /// Hierarchy violations that were corrected.
    pub violations_corrected: Vec<ViolationCorrection>,
}

impl NormalizationReport {
    /// Creates a new normalization report.
    pub fn new(
        original_root_level: Option<HeadingLevel>,
        target_root_level: HeadingLevel,
        level_adjustment: i8,
    ) -> Self {
        Self {
            original_root_level,
            target_root_level,
            level_adjustment,
            headings_adjusted: 0,
            adjustments: Vec::new(),
            violations_corrected: Vec::new(),
        }
    }

    /// Returns true if any changes were made.
    pub fn has_changes(&self) -> bool {
        self.headings_adjusted > 0 || !self.violations_corrected.is_empty()
    }

    /// Returns a human-readable summary.
    pub fn summary(&self) -> String {
        if !self.has_changes() {
            return "No changes needed".to_string();
        }

        let mut parts = vec![];

        if self.level_adjustment != 0 {
            let direction = if self.level_adjustment > 0 {
                "demoted"
            } else {
                "promoted"
            };
            parts.push(format!(
                "{} headings {} by {} level(s)",
                self.headings_adjusted,
                direction,
                self.level_adjustment.abs()
            ));
        }

        if !self.violations_corrected.is_empty() {
            parts.push(format!(
                "{} hierarchy violation(s) corrected",
                self.violations_corrected.len()
            ));
        }

        parts.join("; ")
    }

    /// Adds a heading adjustment.
    pub fn add_adjustment(&mut self, adjustment: HeadingAdjustment) {
        self.adjustments.push(adjustment);
        self.headings_adjusted += 1;
    }

    /// Adds a violation correction.
    pub fn add_violation(&mut self, correction: ViolationCorrection) {
        self.violations_corrected.push(correction);
    }
}

/// Errors that can occur during document normalization.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum NormalizationError {
    /// The document has no headings to normalize.
    #[error("Document has no headings")]
    NoHeadings,

    /// Re-leveling would push some headings beyond H6.
    #[error(
        "Cannot re-level to {target}: would push {affected_count} heading(s) beyond H6 \
        (deepest heading at \"{deepest_title}\" would become H{would_become})"
    )]
    LevelOverflow {
        target: HeadingLevel,
        affected_count: usize,
        deepest_title: String,
        would_become: u8,
    },

    /// A custom validation rule was violated.
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading_level_constants() {
        assert_eq!(HeadingLevel::H1.as_u8(), 1);
        assert_eq!(HeadingLevel::H6.as_u8(), 6);
    }

    #[test]
    fn test_heading_level_new_valid() {
        assert!(HeadingLevel::new(1).is_some());
        assert!(HeadingLevel::new(6).is_some());
    }

    #[test]
    fn test_heading_level_new_invalid() {
        assert!(HeadingLevel::new(0).is_none());
        assert!(HeadingLevel::new(7).is_none());
    }

    #[test]
    fn test_heading_level_hash_count() {
        assert_eq!(HeadingLevel::H1.hash_count(), 1);
        assert_eq!(HeadingLevel::H3.hash_count(), 3);
    }

    #[test]
    fn test_heading_level_deeper() {
        assert_eq!(HeadingLevel::H1.deeper(), Some(HeadingLevel::H2));
        assert!(HeadingLevel::H6.deeper().is_none());
    }

    #[test]
    fn test_heading_level_shallower() {
        assert_eq!(HeadingLevel::H2.shallower(), Some(HeadingLevel::H1));
        assert!(HeadingLevel::H1.shallower().is_none());
    }

    #[test]
    fn test_heading_level_delta_to() {
        assert_eq!(HeadingLevel::H1.delta_to(HeadingLevel::H3), 2);
        assert_eq!(HeadingLevel::H3.delta_to(HeadingLevel::H1), -2);
        assert_eq!(HeadingLevel::H2.delta_to(HeadingLevel::H2), 0);
    }

    #[test]
    fn test_heading_level_display() {
        assert_eq!(format!("{}", HeadingLevel::H1), "H1");
        assert_eq!(format!("{}", HeadingLevel::H6), "H6");
    }

    #[test]
    fn test_heading_level_try_from() {
        assert_eq!(HeadingLevel::try_from(3), Ok(HeadingLevel::H3));
        assert!(HeadingLevel::try_from(7).is_err());
    }

    #[test]
    fn test_structure_issue() {
        let issue = StructureIssue::new(
            StructureIssueKind::HierarchyViolation,
            "Test Heading".to_string(),
            10,
            "A violation occurred".to_string(),
        );
        assert_eq!(issue.kind, StructureIssueKind::HierarchyViolation);
        assert!(issue.suggestion.is_none());
    }

    #[test]
    fn test_structure_issue_with_suggestion() {
        let issue = StructureIssue::new(
            StructureIssueKind::SkippedLevel,
            "Test".to_string(),
            5,
            "Skipped H3".to_string(),
        )
        .with_suggestion("Add an H3 before this heading".to_string());

        assert!(issue.suggestion.is_some());
    }

    #[test]
    fn test_structure_validation_new() {
        let validation = StructureValidation::new();
        assert!(validation.is_valid);
        assert!(validation.is_well_formed());
        assert_eq!(validation.heading_count, 0);
    }

    #[test]
    fn test_structure_validation_add_issue() {
        let mut validation = StructureValidation::new();
        validation.add_issue(StructureIssue::new(
            StructureIssueKind::NoHeadings,
            String::new(),
            0,
            "No headings found".to_string(),
        ));

        assert!(!validation.is_valid);
        assert!(!validation.is_well_formed());
    }

    #[test]
    fn test_structure_validation_can_relevel() {
        let mut validation = StructureValidation::new();
        validation.root_level = Some(HeadingLevel::H2);
        validation.max_level = Some(HeadingLevel::H4);

        // H4 is 2 levels deep from H2. Moving to H3 would make deepest H5 - OK
        assert!(validation.can_relevel_to(HeadingLevel::H3));

        // Moving to H5 would make deepest H7 - NOT OK
        assert!(!validation.can_relevel_to(HeadingLevel::H5));
    }

    #[test]
    fn test_normalization_report_new() {
        let report = NormalizationReport::new(Some(HeadingLevel::H2), HeadingLevel::H1, -1);
        assert!(!report.has_changes());
    }

    #[test]
    fn test_normalization_report_summary() {
        let mut report = NormalizationReport::new(Some(HeadingLevel::H3), HeadingLevel::H1, -2);
        report.add_adjustment(HeadingAdjustment::new(
            "Test".to_string(),
            1,
            HeadingLevel::H3,
            HeadingLevel::H1,
        ));

        let summary = report.summary();
        assert!(summary.contains("promoted"));
        assert!(summary.contains("2 level(s)"));
    }

    #[test]
    fn test_heading_adjustment() {
        let adj = HeadingAdjustment::new(
            "Section".to_string(),
            10,
            HeadingLevel::H3,
            HeadingLevel::H2,
        );
        assert_eq!(adj.original_level, HeadingLevel::H3);
        assert_eq!(adj.new_level, HeadingLevel::H2);
    }

    #[test]
    fn test_violation_correction() {
        let correction = ViolationCorrection::new(
            "Bad Section".to_string(),
            15,
            HeadingLevel::H1,
            HeadingLevel::H3,
            2,
        );

        assert_eq!(correction.children_adjusted, 2);
        assert!(correction.description.contains("Demoted"));
    }

    #[test]
    fn test_normalization_error_no_headings() {
        let err = NormalizationError::NoHeadings;
        assert_eq!(format!("{}", err), "Document has no headings");
    }

    #[test]
    fn test_normalization_error_level_overflow() {
        let err = NormalizationError::LevelOverflow {
            target: HeadingLevel::H4,
            affected_count: 3,
            deepest_title: "Deep Section".to_string(),
            would_become: 8,
        };

        let msg = format!("{}", err);
        assert!(msg.contains("H4"));
        assert!(msg.contains("3 heading(s)"));
        assert!(msg.contains("H8"));
    }
}
