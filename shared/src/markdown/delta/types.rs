//! Type definitions for Markdown document delta comparison.

use serde::Serialize;

/// Path to a section in the document hierarchy.
/// Example: ["Getting Started", "Installation", "Linux"]
pub type SectionPath = Vec<String>;

/// Unique identifier for a section, handling duplicate titles.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SectionId {
    /// Path to the section (heading titles from root to this node).
    pub path: SectionPath,

    /// Hash of the section's own content (for disambiguation).
    pub content_hash: u64,

    /// Occurrence index when multiple sections have identical paths.
    /// (e.g., two "## Examples" sections under the same parent)
    pub occurrence: usize,
}

impl SectionId {
    /// Creates a new section identifier.
    pub fn new(path: SectionPath, content_hash: u64, occurrence: usize) -> Self {
        Self {
            path,
            content_hash,
            occurrence,
        }
    }
}

/// The type of change detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ChangeAction {
    // ─────────────────────────────────────────────────────────────
    // Structural Changes
    // ─────────────────────────────────────────────────────────────
    /// A new section/property was added.
    Added,

    /// A section/property was removed.
    Removed,

    /// A section/property was renamed (content hash unchanged, title changed).
    Renamed,

    /// A section moved to a higher level (e.g., H3 → H2).
    Promoted,

    /// A section moved to a lower level (e.g., H2 → H3).
    Demoted,

    /// A section's position changed among its siblings (same parent, different order).
    Reordered,

    /// A section moved to a different parent at the same heading level.
    MovedSameLevel,

    /// A section moved to a different parent AND different heading level.
    MovedDifferentLevel,

    // ─────────────────────────────────────────────────────────────
    // Content Changes
    // ─────────────────────────────────────────────────────────────
    /// Content was modified (substantive text changes).
    ContentModified,

    /// Only whitespace changed (trimmed hash unchanged).
    /// These changes won't affect rendered output.
    WhitespaceOnly,

    // ─────────────────────────────────────────────────────────────
    // Frontmatter-Specific
    // ─────────────────────────────────────────────────────────────
    /// A frontmatter property was added.
    PropertyAdded,

    /// A frontmatter property was removed.
    PropertyRemoved,

    /// A frontmatter property value changed.
    PropertyUpdated,

    /// Frontmatter key ordering or formatting changed (values unchanged).
    PropertyReordered,
}

/// A change to a frontmatter property.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FrontmatterChange {
    /// The type of change.
    pub action: ChangeAction,

    /// The property key that changed.
    pub key: String,

    /// The original value (if applicable).
    pub original_value: Option<serde_json::Value>,

    /// The new value (if applicable).
    pub new_value: Option<serde_json::Value>,

    /// Human-readable description of the change.
    pub description: String,
}

impl FrontmatterChange {
    /// Creates a new frontmatter change record.
    pub fn new(
        action: ChangeAction,
        key: String,
        original_value: Option<serde_json::Value>,
        new_value: Option<serde_json::Value>,
        description: String,
    ) -> Self {
        Self {
            action,
            key,
            original_value,
            new_value,
            description,
        }
    }

    /// Creates an "added" change.
    pub fn added(key: String, value: serde_json::Value) -> Self {
        Self::new(
            ChangeAction::PropertyAdded,
            key.clone(),
            None,
            Some(value),
            format!("Added frontmatter property '{}'", key),
        )
    }

    /// Creates a "removed" change.
    pub fn removed(key: String, value: serde_json::Value) -> Self {
        Self::new(
            ChangeAction::PropertyRemoved,
            key.clone(),
            Some(value),
            None,
            format!("Removed frontmatter property '{}'", key),
        )
    }

    /// Creates an "updated" change.
    pub fn updated(key: String, old_value: serde_json::Value, new_value: serde_json::Value) -> Self {
        Self::new(
            ChangeAction::PropertyUpdated,
            key.clone(),
            Some(old_value),
            Some(new_value),
            format!("Updated frontmatter property '{}'", key),
        )
    }
}

/// A change to document content (sections, preamble).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContentChange {
    /// The type of change.
    pub action: ChangeAction,

    /// Path to the section in the ORIGINAL document (for removals, modifications).
    pub original_path: Option<SectionPath>,

    /// Path to the section in the NEW document (for additions, modifications).
    pub new_path: Option<SectionPath>,

    /// Original section identifier (for correlation).
    pub original_id: Option<SectionId>,

    /// New section identifier.
    pub new_id: Option<SectionId>,

    /// The heading level in the original document.
    pub original_level: Option<u8>,

    /// The heading level in the new document.
    pub new_level: Option<u8>,

    /// Line number in original document.
    pub original_line: Option<usize>,

    /// Line number in new document.
    pub new_line: Option<usize>,

    /// Human-readable description of the change.
    pub description: String,
}

impl ContentChange {
    /// Creates a new content change record.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        action: ChangeAction,
        original_path: Option<SectionPath>,
        new_path: Option<SectionPath>,
        original_level: Option<u8>,
        new_level: Option<u8>,
        original_line: Option<usize>,
        new_line: Option<usize>,
        description: String,
    ) -> Self {
        Self {
            action,
            original_path,
            new_path,
            original_id: None,
            new_id: None,
            original_level,
            new_level,
            original_line,
            new_line,
            description,
        }
    }

    /// Creates an "added" change for a section.
    pub fn added(path: SectionPath, level: u8, line: usize, title: &str) -> Self {
        Self::new(
            ChangeAction::Added,
            None,
            Some(path),
            None,
            Some(level),
            None,
            Some(line),
            format!("Added section '{}'", title),
        )
    }

    /// Creates a "removed" change for a section.
    pub fn removed(path: SectionPath, level: u8, line: usize, title: &str) -> Self {
        Self::new(
            ChangeAction::Removed,
            Some(path),
            None,
            Some(level),
            None,
            Some(line),
            None,
            format!("Removed section '{}'", title),
        )
    }

    /// Creates a "modified" change for a section.
    pub fn modified(
        original_path: SectionPath,
        new_path: SectionPath,
        level: u8,
        original_line: usize,
        new_line: usize,
        title: &str,
    ) -> Self {
        Self::new(
            ChangeAction::ContentModified,
            Some(original_path),
            Some(new_path),
            Some(level),
            Some(level),
            Some(original_line),
            Some(new_line),
            format!("Modified content in section '{}'", title),
        )
    }
}

/// A section that moved without content changes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MovedSection {
    /// The content hash (unchanged between documents).
    pub content_hash: u64,

    /// Path in the original document.
    pub original_path: SectionPath,

    /// Path in the new document.
    pub new_path: SectionPath,

    /// Level change: negative = promoted, 0 = same, positive = demoted.
    pub level_delta: i8,

    /// Original sibling index (position among siblings).
    pub original_sibling_index: usize,

    /// New sibling index.
    pub new_sibling_index: usize,
}

impl MovedSection {
    /// Creates a new moved section record.
    pub fn new(
        content_hash: u64,
        original_path: SectionPath,
        new_path: SectionPath,
        level_delta: i8,
        original_sibling_index: usize,
        new_sibling_index: usize,
    ) -> Self {
        Self {
            content_hash,
            original_path,
            new_path,
            level_delta,
            original_sibling_index,
            new_sibling_index,
        }
    }

    /// Returns true if the section was promoted (moved to higher level).
    pub fn was_promoted(&self) -> bool {
        self.level_delta < 0
    }

    /// Returns true if the section was demoted (moved to lower level).
    pub fn was_demoted(&self) -> bool {
        self.level_delta > 0
    }

    /// Returns true if only the position changed (same level).
    pub fn was_reordered(&self) -> bool {
        self.level_delta == 0
    }
}

/// A link that would break due to heading changes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BrokenLink {
    /// The target slug that no longer exists.
    pub target_slug: String,

    /// The link text.
    pub link_text: String,

    /// Line number where this link appears.
    pub line_number: usize,

    /// Suggested replacement slug (if a similar heading was found).
    pub suggested_replacement: Option<String>,

    /// Confidence score for the suggestion (0.0 - 1.0).
    pub suggestion_confidence: Option<f32>,
}

impl BrokenLink {
    /// Creates a new broken link record.
    pub fn new(target_slug: String, link_text: String, line_number: usize) -> Self {
        Self {
            target_slug,
            link_text,
            line_number,
            suggested_replacement: None,
            suggestion_confidence: None,
        }
    }

    /// Sets a suggested replacement.
    pub fn with_suggestion(mut self, slug: String, confidence: f32) -> Self {
        self.suggested_replacement = Some(slug);
        self.suggestion_confidence = Some(confidence);
        self
    }
}

/// A change to a code block.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CodeBlockChange {
    /// The type of change.
    pub action: ChangeAction,

    /// Language identifier (if specified).
    pub language: Option<String>,

    /// Path to the containing section.
    pub section_path: SectionPath,

    /// Line number in original document.
    pub original_line: Option<usize>,

    /// Line number in new document.
    pub new_line: Option<usize>,

    /// Human-readable description.
    pub description: String,
}

impl CodeBlockChange {
    /// Creates a new code block change.
    pub fn new(
        action: ChangeAction,
        language: Option<String>,
        section_path: SectionPath,
        original_line: Option<usize>,
        new_line: Option<usize>,
        description: String,
    ) -> Self {
        Self {
            action,
            language,
            section_path,
            original_line,
            new_line,
            description,
        }
    }
}

/// Statistics about changes between two documents.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct DeltaStatistics {
    // ─────────────────────────────────────────────────────────────
    // Content Metrics
    // ─────────────────────────────────────────────────────────────
    /// Total bytes in original document (excluding frontmatter).
    pub original_bytes: usize,

    /// Total bytes in new document (excluding frontmatter).
    pub new_bytes: usize,

    /// Estimated bytes changed (added + removed).
    pub bytes_changed: usize,

    /// Ratio of content that changed (0.0 - 1.0).
    /// Calculated as: bytes_changed / max(original_bytes, new_bytes)
    pub content_change_ratio: f32,

    // ─────────────────────────────────────────────────────────────
    // Section Metrics
    // ─────────────────────────────────────────────────────────────
    /// Number of sections in original document.
    pub original_section_count: usize,

    /// Number of sections in new document.
    pub new_section_count: usize,

    /// Sections that were added.
    pub sections_added: usize,

    /// Sections that were removed.
    pub sections_removed: usize,

    /// Sections with content modifications.
    pub sections_modified: usize,

    /// Sections that moved (same content, different location).
    pub sections_moved: usize,

    /// Sections that are completely unchanged.
    pub sections_unchanged: usize,

    // ─────────────────────────────────────────────────────────────
    // Change Type Breakdown
    // ─────────────────────────────────────────────────────────────
    /// Number of structural changes (moves, level changes, reorders).
    pub structural_changes: usize,

    /// Number of content-only changes (text edits within sections).
    pub content_only_changes: usize,

    /// Number of whitespace-only changes.
    pub whitespace_only_changes: usize,

    // ─────────────────────────────────────────────────────────────
    // Code Block Metrics
    // ─────────────────────────────────────────────────────────────
    /// Code blocks added.
    pub code_blocks_added: usize,

    /// Code blocks removed.
    pub code_blocks_removed: usize,

    /// Code blocks modified.
    pub code_blocks_modified: usize,

    // ─────────────────────────────────────────────────────────────
    // Link Metrics
    // ─────────────────────────────────────────────────────────────
    /// Internal links that would break.
    pub broken_links_count: usize,
}

/// High-level classification of document changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DocumentChange {
    /// No changes detected (all hashes match).
    NoChange,

    /// Only whitespace changed (trimmed hashes match).
    /// Rendered output would be identical.
    WhitespaceOnly,

    /// Only frontmatter changed, body is identical.
    FrontmatterOnly,

    /// Frontmatter and whitespace changed, but no substantive body changes.
    FrontmatterAndWhitespace,

    /// Only structural changes (sections moved/reordered, content unchanged).
    StructuralOnly,

    /// Minor content changes (< 10% of content changed).
    ContentMinor,

    /// Moderate content changes (10-40% of content changed).
    ContentModerate,

    /// Major content changes (40-80% of content changed).
    ContentMajor,

    /// Document substantially rewritten (> 80% of content changed).
    Rewritten,
}

impl DocumentChange {
    /// Determine the change classification from statistics.
    pub fn from_statistics(stats: &DeltaStatistics) -> Self {
        // Check for no changes first
        if stats.content_change_ratio == 0.0
            && stats.sections_added == 0
            && stats.sections_removed == 0
            && stats.sections_moved == 0
        {
            return DocumentChange::NoChange;
        }

        // Check for whitespace-only
        if stats.content_only_changes == 0
            && stats.structural_changes == 0
            && stats.whitespace_only_changes > 0
        {
            return DocumentChange::WhitespaceOnly;
        }

        // Check for structural-only
        if stats.content_only_changes == 0 && stats.structural_changes > 0 {
            return DocumentChange::StructuralOnly;
        }

        // Classify by content change ratio
        match stats.content_change_ratio {
            r if r < 0.10 => DocumentChange::ContentMinor,
            r if r < 0.40 => DocumentChange::ContentModerate,
            r if r < 0.80 => DocumentChange::ContentMajor,
            _ => DocumentChange::Rewritten,
        }
    }
}

/// Complete analysis of changes between two Markdown documents.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MarkdownDelta {
    // ─────────────────────────────────────────────────────────────
    // Summary
    // ─────────────────────────────────────────────────────────────
    /// High-level classification of the change.
    pub classification: DocumentChange,

    /// Quantifiable statistics about the changes.
    pub statistics: DeltaStatistics,

    // ─────────────────────────────────────────────────────────────
    // Frontmatter Changes
    // ─────────────────────────────────────────────────────────────
    /// Whether frontmatter changed at all.
    pub frontmatter_changed: bool,

    /// Whether frontmatter changes are formatting-only (key order, whitespace).
    pub frontmatter_formatting_only: bool,

    /// Detailed frontmatter property changes.
    pub frontmatter_changes: Vec<FrontmatterChange>,

    // ─────────────────────────────────────────────────────────────
    // Preamble Changes
    // ─────────────────────────────────────────────────────────────
    /// Whether the preamble (content before first heading) changed.
    pub preamble_changed: bool,

    /// Whether preamble changes are whitespace-only.
    pub preamble_whitespace_only: bool,

    // ─────────────────────────────────────────────────────────────
    // Content Changes (Bidirectional)
    // ─────────────────────────────────────────────────────────────
    /// Sections that exist in NEW but not in ORIGINAL (additions).
    pub added: Vec<ContentChange>,

    /// Sections that exist in ORIGINAL but not in NEW (removals).
    pub removed: Vec<ContentChange>,

    /// Sections that exist in both but with different content.
    pub modified: Vec<ContentChange>,

    /// Sections that moved without content changes.
    pub moved: Vec<MovedSection>,

    // ─────────────────────────────────────────────────────────────
    // Code Block Changes
    // ─────────────────────────────────────────────────────────────
    /// Changes to code blocks specifically.
    pub code_block_changes: Vec<CodeBlockChange>,

    // ─────────────────────────────────────────────────────────────
    // Link Impact
    // ─────────────────────────────────────────────────────────────
    /// Internal links that would break due to heading changes.
    pub broken_links: Vec<BrokenLink>,
}

impl Default for MarkdownDelta {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownDelta {
    /// Creates a new empty delta (no changes).
    pub fn new() -> Self {
        Self {
            classification: DocumentChange::NoChange,
            statistics: DeltaStatistics::default(),
            frontmatter_changed: false,
            frontmatter_formatting_only: false,
            frontmatter_changes: Vec::new(),
            preamble_changed: false,
            preamble_whitespace_only: false,
            added: Vec::new(),
            removed: Vec::new(),
            modified: Vec::new(),
            moved: Vec::new(),
            code_block_changes: Vec::new(),
            broken_links: Vec::new(),
        }
    }

    /// Returns true if there are no substantive changes.
    pub fn is_unchanged(&self) -> bool {
        matches!(self.classification, DocumentChange::NoChange)
    }

    /// Returns true if changes are cosmetic only (whitespace/formatting).
    pub fn is_cosmetic_only(&self) -> bool {
        matches!(
            self.classification,
            DocumentChange::NoChange
                | DocumentChange::WhitespaceOnly
                | DocumentChange::FrontmatterAndWhitespace
        )
    }

    /// Returns true if any internal links would break.
    pub fn has_broken_links(&self) -> bool {
        !self.broken_links.is_empty()
    }

    /// Returns a human-readable summary of the changes.
    pub fn summary(&self) -> String {
        let stats = &self.statistics;
        format!(
            "{:?}: {} added, {} removed, {} modified, {} moved ({:.1}% changed)",
            self.classification,
            stats.sections_added,
            stats.sections_removed,
            stats.sections_modified,
            stats.sections_moved,
            stats.content_change_ratio * 100.0
        )
    }

    /// Returns total number of changes.
    pub fn change_count(&self) -> usize {
        self.added.len()
            + self.removed.len()
            + self.modified.len()
            + self.moved.len()
            + self.frontmatter_changes.len()
            + self.code_block_changes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_id() {
        let id = SectionId::new(vec!["Root".to_string(), "Child".to_string()], 12345, 0);
        assert_eq!(id.path.len(), 2);
        assert_eq!(id.content_hash, 12345);
    }

    #[test]
    fn test_frontmatter_change_added() {
        let change = FrontmatterChange::added("title".to_string(), serde_json::json!("Hello"));
        assert!(matches!(change.action, ChangeAction::PropertyAdded));
        assert!(change.original_value.is_none());
        assert!(change.new_value.is_some());
    }

    #[test]
    fn test_frontmatter_change_removed() {
        let change = FrontmatterChange::removed("title".to_string(), serde_json::json!("Hello"));
        assert!(matches!(change.action, ChangeAction::PropertyRemoved));
        assert!(change.original_value.is_some());
        assert!(change.new_value.is_none());
    }

    #[test]
    fn test_frontmatter_change_updated() {
        let change = FrontmatterChange::updated(
            "title".to_string(),
            serde_json::json!("Old"),
            serde_json::json!("New"),
        );
        assert!(matches!(change.action, ChangeAction::PropertyUpdated));
        assert!(change.original_value.is_some());
        assert!(change.new_value.is_some());
    }

    #[test]
    fn test_content_change_added() {
        let change = ContentChange::added(
            vec!["New Section".to_string()],
            2,
            10,
            "New Section",
        );
        assert!(matches!(change.action, ChangeAction::Added));
        assert!(change.original_path.is_none());
        assert!(change.new_path.is_some());
    }

    #[test]
    fn test_content_change_removed() {
        let change = ContentChange::removed(
            vec!["Old Section".to_string()],
            2,
            5,
            "Old Section",
        );
        assert!(matches!(change.action, ChangeAction::Removed));
        assert!(change.original_path.is_some());
        assert!(change.new_path.is_none());
    }

    #[test]
    fn test_moved_section_promoted() {
        let moved = MovedSection::new(12345, vec!["A".to_string()], vec!["A".to_string()], -1, 0, 0);
        assert!(moved.was_promoted());
        assert!(!moved.was_demoted());
    }

    #[test]
    fn test_moved_section_demoted() {
        let moved = MovedSection::new(12345, vec!["A".to_string()], vec!["A".to_string()], 1, 0, 0);
        assert!(!moved.was_promoted());
        assert!(moved.was_demoted());
    }

    #[test]
    fn test_moved_section_reordered() {
        let moved = MovedSection::new(12345, vec!["A".to_string()], vec!["A".to_string()], 0, 0, 1);
        assert!(moved.was_reordered());
    }

    #[test]
    fn test_broken_link() {
        let link = BrokenLink::new("old-section".to_string(), "Link Text".to_string(), 10);
        assert_eq!(link.target_slug, "old-section");
        assert!(link.suggested_replacement.is_none());
    }

    #[test]
    fn test_broken_link_with_suggestion() {
        let link = BrokenLink::new("old-section".to_string(), "Link Text".to_string(), 10)
            .with_suggestion("new-section".to_string(), 0.9);
        assert_eq!(link.suggested_replacement, Some("new-section".to_string()));
        assert_eq!(link.suggestion_confidence, Some(0.9));
    }

    #[test]
    fn test_document_change_no_change() {
        let stats = DeltaStatistics::default();
        assert!(matches!(
            DocumentChange::from_statistics(&stats),
            DocumentChange::NoChange
        ));
    }

    #[test]
    fn test_document_change_content_minor() {
        let mut stats = DeltaStatistics::default();
        stats.content_change_ratio = 0.05;
        stats.content_only_changes = 1;
        assert!(matches!(
            DocumentChange::from_statistics(&stats),
            DocumentChange::ContentMinor
        ));
    }

    #[test]
    fn test_document_change_content_moderate() {
        let mut stats = DeltaStatistics::default();
        stats.content_change_ratio = 0.25;
        stats.content_only_changes = 1;
        assert!(matches!(
            DocumentChange::from_statistics(&stats),
            DocumentChange::ContentModerate
        ));
    }

    #[test]
    fn test_document_change_rewritten() {
        let mut stats = DeltaStatistics::default();
        stats.content_change_ratio = 0.85;
        stats.content_only_changes = 1;
        assert!(matches!(
            DocumentChange::from_statistics(&stats),
            DocumentChange::Rewritten
        ));
    }

    #[test]
    fn test_markdown_delta_new() {
        let delta = MarkdownDelta::new();
        assert!(delta.is_unchanged());
        assert!(delta.is_cosmetic_only());
        assert!(!delta.has_broken_links());
        assert_eq!(delta.change_count(), 0);
    }

    #[test]
    fn test_markdown_delta_summary() {
        let delta = MarkdownDelta::new();
        let summary = delta.summary();
        assert!(summary.contains("NoChange"));
    }
}
