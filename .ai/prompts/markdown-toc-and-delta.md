# Markdown TOC, Delta, Normalization, and Re-leveling

- This plan involves work in the Shared Library (`./shared`) package
- And minor enhancements in the `md` CLI (`./md`) to expose this new capability built into the `Markdown` struct

Save plan to `.ai/plans/2026-0107. Markdown-TOC.md`

## Preamble

Currently we have provided a utility function for the **xxHash** function in @shared/src/mermaid/hash.rs this is the wrong location for it. It should service the Mermaid functionality but it is much more general purpose and will used more widely (include Markdown files in the next sections).

- Move the implementation from it's current location to @shared/src/hashing/xx_hash.rs and rename the function to `xx_hash()`.
    - Make sure that all tests are still passing
    - Make sure all callers within the shared library are referencing the new symbol location
- Add the `blake3` crate to the shared library and then provide a public utility function `blake3()` in the @shared/src/hashing/blake3.rs file.

## Table of Contents

The `Markdown` struct will have a method `toc()` attached to its public interface which will return a Table of Contents (`MarkdownToc`). This TOC will provide both headings and hash values.

### MarkdownTocNode

Each node in the TOC represents a heading and its associated content:

```rust
/// A node in the Table of Contents for a Markdown page.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkdownTocNode {
    /// The level of the heading node (1-6)
    pub level: u8,

    /// The text content of this heading, minus the leading `#` markers.
    /// Preserves inline formatting (bold, code, etc.) as raw markdown.
    pub title: String,

    /// An xxHash of the title text
    pub title_hash: u64,

    /// An xxHash of the title text after trimming whitespace
    pub title_hash_trimmed: u64,

    /// The generated anchor slug for this heading (e.g., "my-heading" for "## My Heading").
    /// Used for internal link detection and TOC link generation.
    pub slug: String,

    // ─────────────────────────────────────────────────────────────
    // Location Information
    // ─────────────────────────────────────────────────────────────

    /// Byte offset range in the source document [start, end).
    /// Covers from the heading line through all content until the next
    /// heading at the same or higher level (or end of document).
    pub source_span: (usize, usize),

    /// Line number range [start_line, end_line) (1-indexed).
    pub line_range: (usize, usize),

    // ─────────────────────────────────────────────────────────────
    // Own Content (this section only, excluding children)
    // ─────────────────────────────────────────────────────────────

    /// The textual content of THIS section only.
    /// Spans from after the heading line to either:
    /// - The first child heading, OR
    /// - The next sibling/parent heading, OR
    /// - End of document
    /// Does NOT include child section content.
    pub own_content: Option<String>,

    /// An xxHash of `own_content`
    pub own_content_hash: u64,

    /// An xxHash of `own_content` after trimming whitespace
    pub own_content_hash_trimmed: u64,

    // ─────────────────────────────────────────────────────────────
    // Subtree Hashes (for quick "has anything changed?" checks)
    // ─────────────────────────────────────────────────────────────

    /// An xxHash combining this node's content AND all descendant content.
    /// Useful for quick subtree change detection without traversing children.
    pub subtree_hash: u64,

    /// An xxHash of the subtree content after trimming all whitespace.
    pub subtree_hash_trimmed: u64,

    // ─────────────────────────────────────────────────────────────
    // Hierarchy
    // ─────────────────────────────────────────────────────────────

    /// Child nodes (headings at a deeper level under this heading)
    pub children: Vec<MarkdownTocNode>,
}
```

### CodeBlockInfo

Code blocks are tracked separately for precise change detection:

```rust
/// Information about a fenced code block in the document.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeBlockInfo {
    /// The language identifier (e.g., "rust", "javascript"), if specified.
    pub language: Option<String>,

    /// The raw content of the code block (excluding fence markers).
    pub content: String,

    /// An xxHash of the code block content.
    pub content_hash: u64,

    /// Line number range [start_line, end_line) (1-indexed).
    pub line_range: (usize, usize),

    /// Path to the containing section (e.g., ["Introduction", "Setup"]).
    pub parent_section_path: Vec<String>,
}
```

### InternalLinkInfo

Internal links are tracked to detect broken links after heading renames:

```rust
/// Information about an internal link (anchor reference) in the document.
#[derive(Debug, Clone, PartialEq)]
pub struct InternalLinkInfo {
    /// The target anchor (e.g., "my-heading" from `[text](#my-heading)`).
    pub target_slug: String,

    /// The link text.
    pub link_text: String,

    /// Line number where this link appears (1-indexed).
    pub line_number: usize,

    /// Byte offset of the link in the source document.
    pub byte_offset: usize,

    /// Path to the containing section.
    pub parent_section_path: Vec<String>,
}
```

### MarkdownToc

The top-level TOC structure for a document:

```rust
/// Table of Contents for a Markdown document.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkdownToc {
    // ─────────────────────────────────────────────────────────────
    // Document Identity
    // ─────────────────────────────────────────────────────────────

    /// The document title, determined by:
    /// 1. The `title` frontmatter property (if set), OR
    /// 2. The text of the single H1 heading (if exactly one H1 exists), OR
    /// 3. None
    pub title: Option<String>,

    /// An xxHash of the entire page content (excluding frontmatter).
    pub page_hash: u64,

    /// An xxHash of the page content after trimming (excluding frontmatter).
    pub page_hash_trimmed: u64,

    // ─────────────────────────────────────────────────────────────
    // Frontmatter
    // ─────────────────────────────────────────────────────────────

    /// An xxHash of the raw frontmatter YAML string.
    /// Hashes the raw text to detect formatting/ordering changes.
    pub frontmatter_hash: u64,

    /// An xxHash of the frontmatter after serializing to canonical form.
    /// Ignores key ordering and whitespace differences.
    pub frontmatter_hash_normalized: u64,

    // ─────────────────────────────────────────────────────────────
    // Preamble (content before first heading)
    // ─────────────────────────────────────────────────────────────

    /// The text before the first heading tag is encountered.
    pub preamble: String,

    /// An xxHash of the preamble text.
    pub preamble_hash: u64,

    /// An xxHash of the preamble after trimming whitespace.
    pub preamble_hash_trimmed: u64,

    // ─────────────────────────────────────────────────────────────
    // Structure
    // ─────────────────────────────────────────────────────────────

    /// The hierarchical structure of headings in the document.
    ///
    /// ## Fragment Rules
    ///
    /// The `structure` vector contains one or more "fragments". A new fragment
    /// starts when:
    ///
    /// 1. At the first heading in the document
    /// 2. When encountering a heading at a level ≤ the current fragment's root level
    /// 3. When encountering an H1 (always starts a new fragment)
    ///
    /// ## Examples
    ///
    /// Document: `H2 -> H3 -> H3 -> H4 -> H3 -> H2 -> H3`
    /// Fragments:
    /// - Fragment 1 (root level 2): `H2 -> H3 -> H3 -> H4 -> H3`
    /// - Fragment 2 (root level 2): `H2 -> H3`
    ///
    /// Document: `H1 -> H2 -> H1 -> H2`
    /// Fragments:
    /// - Fragment 1 (root level 1): `H1 -> H2`
    /// - Fragment 2 (root level 1): `H1 -> H2`
    ///
    /// Document: `H3 -> H4 -> H2 -> H3`
    /// Fragments:
    /// - Fragment 1 (root level 3): `H3 -> H4`
    /// - Fragment 2 (root level 2): `H2 -> H3`
    pub structure: Vec<MarkdownTocNode>,

    // ─────────────────────────────────────────────────────────────
    // Additional Tracking
    // ─────────────────────────────────────────────────────────────

    /// All code blocks in the document, in order of appearance.
    pub code_blocks: Vec<CodeBlockInfo>,

    /// All internal links (anchor references) in the document.
    pub internal_links: Vec<InternalLinkInfo>,

    /// All heading slugs in the document (for quick lookup).
    /// Maps slug -> Vec<(section_path, line_number)> to handle duplicates.
    pub slug_index: HashMap<String, Vec<(Vec<String>, usize)>>,
}
```

### Implementation Notes

- The `MarkdownToc` should implement `From<&Markdown>` and `From<Markdown>`
- Implementation should use `pulldown_cmark` for performance (streaming parser)
    - The shared library already uses both `markdown_rs` and `pulldown_cmark`
    - `pulldown_cmark` is preferred here because:
        1. TOC extraction is a streaming operation (no need for full AST)
        2. Better performance for large documents
        3. Built-in support for heading extraction events
    - Use `markdown_rs` only if AST manipulation is required later

## Delta

The `Markdown` struct will expose a public function called `delta()` which takes another `Markdown` struct and provides a detailed analysis of what has changed.

### Design Principles

1. **Hash-based detection**: Use xxHash values for O(1) change detection
2. **Bidirectional tracking**: Clearly separate additions, removals, and modifications
3. **Whitespace awareness**: Distinguish substantive changes from formatting changes
4. **Hierarchical context**: Track changes within their structural context
5. **Quantifiable metrics**: Provide statistics for programmatic threshold decisions

### SectionPath

A unique identifier for locating a section within the document hierarchy:

```rust
/// Path to a section in the document hierarchy.
/// Example: ["Getting Started", "Installation", "Linux"]
pub type SectionPath = Vec<String>;

/// Unique identifier for a section, handling duplicate titles.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SectionId {
    /// Path to the section (heading titles from root to this node).
    pub path: SectionPath,

    /// Hash of the section's own content (for disambiguation).
    pub content_hash: u64,

    /// Occurrence index when multiple sections have identical paths.
    /// (e.g., two "## Examples" sections under the same parent)
    pub occurrence: usize,
}
```

### ChangeAction

Granular classification of change types:

```rust
/// The type of change detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
```

### Change Item Structs

Detailed change records with full context:

```rust
/// A change to a frontmatter property.
#[derive(Debug, Clone, PartialEq)]
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

/// A change to document content (sections, preamble).
#[derive(Debug, Clone, PartialEq)]
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

/// A section that moved without content changes.
#[derive(Debug, Clone, PartialEq)]
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

/// A link that would break due to heading changes.
#[derive(Debug, Clone, PartialEq)]
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

/// A change to a code block.
#[derive(Debug, Clone, PartialEq)]
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
```

### DeltaStatistics

Quantifiable metrics for programmatic decision-making:

```rust
/// Statistics about changes between two documents.
#[derive(Debug, Clone, PartialEq, Default)]
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
```

### DocumentChange

High-level classification with defined thresholds:

```rust
/// High-level classification of document changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
```

### MarkdownDelta

The complete delta result:

```rust
/// Complete analysis of changes between two Markdown documents.
#[derive(Debug, Clone, PartialEq)]
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

impl MarkdownDelta {
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
}
```

### Usage Example

```rust
use shared::markdown::{Markdown, MarkdownDelta, DocumentChange};

let original: Markdown = std::fs::read_to_string("doc_v1.md")?.into();
let updated: Markdown = std::fs::read_to_string("doc_v2.md")?.into();

let delta = original.delta(&updated);

// Quick checks
if delta.is_unchanged() {
    println!("No changes detected");
    return Ok(());
}

if delta.is_cosmetic_only() {
    println!("Only whitespace/formatting changes");
}

// Check for breaking changes
if delta.has_broken_links() {
    println!("Warning: {} internal links would break!", delta.broken_links.len());
    for link in &delta.broken_links {
        println!("  Line {}: [{}](#{}) - target removed",
            link.line_number, link.link_text, link.target_slug);
        if let Some(suggestion) = &link.suggested_replacement {
            println!("    Suggestion: #{} (confidence: {:.0}%)",
                suggestion, link.suggestion_confidence.unwrap_or(0.0) * 100.0);
        }
    }
}

// Detailed analysis
match delta.classification {
    DocumentChange::ContentMinor => {
        println!("Minor edits in {} sections", delta.statistics.sections_modified);
    }
    DocumentChange::Rewritten => {
        println!("Document substantially rewritten ({:.0}% changed)",
            delta.statistics.content_change_ratio * 100.0);
    }
    _ => {}
}

// Iterate specific changes
for change in &delta.added {
    println!("+ Added: {:?}", change.new_path);
}

for change in &delta.removed {
    println!("- Removed: {:?}", change.original_path);
}

for moved in &delta.moved {
    println!("↔ Moved: {:?} → {:?} (level {})",
        moved.original_path, moved.new_path,
        if moved.level_delta < 0 { "up" }
        else if moved.level_delta > 0 { "down" }
        else { "same" });
}
```

## Implementation Notes

### Section Matching Algorithm

When comparing documents, sections are matched using this priority:

1. **Exact match**: Same path AND same content hash
2. **Content match**: Same content hash, different path (indicates move)
3. **Title match**: Same path, different content hash (indicates edit)
4. **Fuzzy match**: Similar title (Levenshtein distance), similar content hash position

### Hash Collision Handling

While xxHash collisions are extremely rare (1 in 2^64), the implementation should:

1. Use `SectionId` compound keys (path + hash + occurrence) for identification
2. Fall back to content comparison when hashes match but paths differ
3. Log warnings if identical hashes are found for different content

### Performance Considerations

1. **Lazy TOC generation**: Only compute TOC when `toc()` or `delta()` is called
2. **Hash caching**: Store computed hashes in the `Markdown` struct (behind `OnceCell`)
3. **Streaming comparison**: For large documents, compare section-by-section rather than loading full TOCs into memory

## Normalization and Re-leveling

The `Markdown` struct will provide functions for validating document structure and normalizing heading levels to ensure well-formed hierarchy.

### Document Structure Concepts

**Well-Formed Document**: A markdown document where heading levels form a proper hierarchy:
- A "complete" document has a single H1 as the root, with all other headings nested beneath it
- A "fragment" document starts at a higher level (H2-H6) but maintains consistent hierarchy from that root level

**Root Level**: The heading level of the first heading in the document. All other headings should be at this level or deeper (higher numbers).

**Hierarchy Violation**: When a heading appears at a level shallower (lower number) than the document's root level. For example, an H2 appearing after the document started with H3.

### HeadingLevel

A type-safe representation of markdown heading levels:

```rust
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
        Self::new(self.0 - 1)
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
```

### StructureIssue

Issues detected during document structure validation:

```rust
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
```

### StructureValidation

Result of validating a document's heading structure:

```rust
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

impl StructureValidation {
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
}
```

### NormalizationReport

Details about what changed during normalization:

```rust
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

impl NormalizationReport {
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
            let direction = if self.level_adjustment > 0 { "demoted" } else { "promoted" };
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
}
```

### Markdown Methods

New methods on the `Markdown` struct:

```rust
impl Markdown {
    /// Validates the document's heading structure.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use shared::markdown::Markdown;
    ///
    /// let doc: Markdown = "## Intro\n### Details\n## Conclusion".into();
    /// let validation = doc.validate_structure();
    ///
    /// assert!(validation.is_well_formed());
    /// assert_eq!(validation.root_level, Some(HeadingLevel::H2));
    /// assert_eq!(validation.heading_count, 3);
    /// ```
    pub fn validate_structure(&self) -> StructureValidation;

    /// Normalizes the document's heading levels.
    ///
    /// ## Parameters
    ///
    /// - `target`: The desired root level. If `None`, uses the current root level
    ///   (effectively just fixing hierarchy violations without changing depth).
    ///
    /// ## Behavior
    ///
    /// 1. **Level Adjustment**: All headings are shifted so the root level matches
    ///    the target. For example, normalizing an H3-rooted document to H1 promotes
    ///    all headings by 2 levels.
    ///
    /// 2. **Hierarchy Violation Correction**: If any heading appears at a level
    ///    shallower than the document's root, it is demoted to match the root level,
    ///    and its children are adjusted proportionally.
    ///
    /// ## Returns
    ///
    /// A tuple of the normalized `Markdown` document and a `NormalizationReport`
    /// describing all changes made.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The document has no headings (nothing to normalize)
    /// - Re-leveling would push headings beyond H6
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use shared::markdown::{Markdown, HeadingLevel};
    ///
    /// // Promote an H3-rooted document to H1
    /// let doc: Markdown = "### Intro\n#### Details".into();
    /// let (normalized, report) = doc.normalize(Some(HeadingLevel::H1))?;
    ///
    /// assert_eq!(normalized.content(), "# Intro\n## Details");
    /// assert_eq!(report.level_adjustment, -2); // Promoted by 2 levels
    /// ```
    pub fn normalize(
        &self,
        target: Option<HeadingLevel>,
    ) -> Result<(Markdown, NormalizationReport), NormalizationError>;

    /// Normalizes the document in place, returning only the report.
    ///
    /// This is a convenience method that modifies `self` instead of returning
    /// a new `Markdown` instance.
    pub fn normalize_mut(
        &mut self,
        target: Option<HeadingLevel>,
    ) -> Result<NormalizationReport, NormalizationError>;

    /// Re-levels the document to a specific target level.
    ///
    /// This is a simpler operation than `normalize()` - it only shifts all
    /// heading levels uniformly without correcting hierarchy violations.
    ///
    /// ## Parameters
    ///
    /// - `target`: The desired root level for the document.
    ///
    /// ## Returns
    ///
    /// A tuple of the re-leveled `Markdown` document and the level adjustment
    /// that was applied.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The document has no headings
    /// - Re-leveling would push headings beyond H6
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use shared::markdown::{Markdown, HeadingLevel};
    ///
    /// // Demote an H1-rooted document to H2 (for embedding as a subsection)
    /// let doc: Markdown = "# Main\n## Sub\n### Detail".into();
    /// let (releveled, adjustment) = doc.relevel(HeadingLevel::H2)?;
    ///
    /// assert_eq!(releveled.content(), "## Main\n### Sub\n#### Detail");
    /// assert_eq!(adjustment, 1); // Demoted by 1 level
    /// ```
    pub fn relevel(
        &self,
        target: HeadingLevel,
    ) -> Result<(Markdown, i8), NormalizationError>;
}
```

### NormalizationError

Errors that can occur during normalization:

```rust
/// Errors that can occur during document normalization.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum NormalizationError {
    /// The document has no headings to normalize.
    #[error("Document has no headings")]
    NoHeadings,

    /// Re-leveling would push some headings beyond H6.
    #[error(
        "Cannot re-level to {target}: would push {affected_count} heading(s) beyond H6 \
        (deepest heading at {deepest_title:?} would become H{would_become})"
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
```

### Normalization Examples

#### Example 1: Simple Re-leveling (Promotion)

```markdown
// Before: H3-rooted document
### Introduction
Some intro text.
#### Background
Background info.
#### Goals
The goals.
### Implementation
Implementation details.
```

```rust
let (normalized, report) = doc.normalize(Some(HeadingLevel::H1))?;
```

```markdown
// After: H1-rooted document
# Introduction
Some intro text.
## Background
Background info.
## Goals
The goals.
# Implementation
Implementation details.
```

```rust
assert_eq!(report.level_adjustment, -2);  // Promoted by 2 levels
assert_eq!(report.headings_adjusted, 5);
assert!(report.violations_corrected.is_empty());
```

#### Example 2: Simple Re-leveling (Demotion)

```markdown
// Before: H1-rooted document (to be embedded as subsection)
# API Reference
## Endpoints
### GET /users
### POST /users
```

```rust
let (normalized, report) = doc.normalize(Some(HeadingLevel::H3))?;
```

```markdown
// After: H3-rooted document
### API Reference
#### Endpoints
##### GET /users
##### POST /users
```

#### Example 3: Hierarchy Violation Correction

```markdown
// Before: Malformed document (H2 appears after H3 root)
### Getting Started
#### Prerequisites
## Installation        <-- VIOLATION: H2 is shallower than root H3
### Basic Usage
```

```rust
let (normalized, report) = doc.normalize(None)?;  // Keep root level, just fix violations
```

```markdown
// After: Well-formed document
### Getting Started
#### Prerequisites
### Installation        <-- Corrected to H3 (matches root level)
### Basic Usage
```

```rust
assert_eq!(report.violations_corrected.len(), 1);
assert_eq!(report.violations_corrected[0].original_level, HeadingLevel::H2);
assert_eq!(report.violations_corrected[0].corrected_level, HeadingLevel::H3);
```

#### Example 4: Violation Correction with Children

```markdown
// Before: Violation with children that need adjustment
### Root Section
#### Subsection
# Misplaced Section    <-- VIOLATION: H1 in H3-rooted document
## Child One           <-- Must be adjusted relative to corrected parent
### Grandchild
### Another Root
```

```rust
let (normalized, report) = doc.normalize(None)?;
```

```markdown
// After: All violations corrected with proportional child adjustment
### Root Section
#### Subsection
### Misplaced Section  <-- H1 → H3 (delta: +2)
#### Child One         <-- H2 → H4 (delta: +2, maintains relationship)
##### Grandchild       <-- H3 → H5 (delta: +2, maintains relationship)
### Another Root
```

```rust
assert_eq!(report.violations_corrected[0].children_adjusted, 2);
```

#### Example 5: Level Overflow Error

```markdown
// Document with deep nesting
#### Level 4
##### Level 5
###### Level 6
```

```rust
let result = doc.normalize(Some(HeadingLevel::H3));
assert!(matches!(
    result,
    Err(NormalizationError::LevelOverflow { would_become: 7, .. })
));
```

### Implementation Notes

#### Heading Detection

Use `pulldown_cmark` events to detect headings:

```rust
use pulldown_cmark::{Event, HeadingLevel as PulldownLevel, Parser, Tag, TagEnd};

fn detect_headings(content: &str) -> Vec<(HeadingLevel, String, usize)> {
    let parser = Parser::new(content);
    let mut headings = vec![];
    let mut current_level = None;
    let mut current_text = String::new();

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current_level = Some(level);
                current_text.clear();
            }
            Event::Text(text) if current_level.is_some() => {
                current_text.push_str(&text);
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(level) = current_level.take() {
                    let line = content[..range.start].lines().count() + 1;
                    headings.push((
                        pulldown_to_heading_level(level),
                        std::mem::take(&mut current_text),
                        line,
                    ));
                }
            }
            _ => {}
        }
    }

    headings
}
```

#### Heading Replacement Strategy

When adjusting heading levels, use byte-offset replacement to preserve all other content:

1. Collect all heading positions and their byte ranges
2. Sort by position (descending) to avoid offset invalidation
3. Replace each heading's `#` prefix with the correct number
4. Preserve the rest of the line (title, trailing hashes, attributes)

```rust
fn replace_heading_level(
    content: &str,
    line_start: usize,
    original_level: HeadingLevel,
    new_level: HeadingLevel,
) -> String {
    let prefix_end = line_start + original_level.hash_count();
    let new_prefix = "#".repeat(new_level.hash_count());

    format!(
        "{}{}{}",
        &content[..line_start],
        new_prefix,
        &content[prefix_end..]
    )
}
```

#### Violation Correction Algorithm

When correcting a hierarchy violation:

1. Identify the violating heading and calculate the required adjustment
2. Find all "child" headings (subsequent headings until one at or shallower than the corrected level)
3. Apply the same delta to all children to maintain relative hierarchy
4. Record the correction for the report

```rust
fn correct_violation(
    headings: &mut [(HeadingLevel, String, usize)],
    violation_idx: usize,
    root_level: HeadingLevel,
) -> ViolationCorrection {
    let (original_level, title, line) = &headings[violation_idx];
    let delta = root_level.as_u8() as i8 - original_level.as_u8() as i8;

    // Adjust the violating heading
    headings[violation_idx].0 = root_level;

    // Find and adjust children
    let mut children_adjusted = 0;
    for i in (violation_idx + 1)..headings.len() {
        let child_level = headings[i].0;

        // Stop when we hit a heading at or shallower than root
        if child_level.as_u8() <= root_level.as_u8() {
            break;
        }

        // Apply same delta to maintain relative hierarchy
        if let Some(new_level) = HeadingLevel::new((child_level.as_u8() as i8 + delta) as u8) {
            headings[i].0 = new_level;
            children_adjusted += 1;
        }
    }

    ViolationCorrection {
        title: title.clone(),
        line_number: *line,
        original_level: *original_level,
        corrected_level: root_level,
        children_adjusted,
        description: format!(
            "Demoted {} from {} to {} (root level), adjusted {} children",
            title, original_level, root_level, children_adjusted
        ),
    }
}
