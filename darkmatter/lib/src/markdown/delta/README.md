# Markdown Delta

Structural diff analysis between markdown documents.

## Features

- **Change Classification**: NoChange, WhitespaceOnly, FrontmatterOnly, FrontmatterAndWhitespace, StructuralOnly, ContentMinor, ContentModerate, ContentMajor, Rewritten
- **Content Changes**: Track additions, removals, and modifications at section level
- **Section Movements**: Detect when sections are relocated within the document
- **Frontmatter Changes**: Track YAML frontmatter additions, removals, and modifications
- **Broken Link Detection**: Identify internal links that may be broken after changes
- **Code Block Changes**: Track changes to fenced code blocks
- **Statistics**: Aggregate change counts via `DeltaStatistics`

## Usage

```rust
use darkmatter::markdown::Markdown;

let original: Markdown = old_content.into();
let updated: Markdown = new_content.into();

let delta = original.delta(&updated);

if !delta.is_unchanged() {
    println!("Classification: {:?}", delta.classification);
    println!("{}", delta.summary());
}
```

## Key Types

| Type | Description |
|------|-------------|
| `MarkdownDelta` | Complete diff result |
| `DocumentChange` | Change classification enum (9 variants) |
| `DeltaStatistics` | Numeric change counts |
| `SectionPath` | Type alias for `Vec<String>` representing a section's path |
| `SectionId` | Section identifier with path and title |
| `ContentChange` | Individual content change |
| `ChangeAction` | Change type: Added, Removed, Renamed, Promoted, Demoted, Reordered, MovedSameLevel, MovedDifferentLevel, ContentModified, WhitespaceOnly, PropertyAdded, PropertyRemoved, PropertyUpdated, PropertyReordered |
| `MovedSection` | Section relocation info |
| `FrontmatterChange` | Frontmatter key change |
| `BrokenLink` | Potentially broken internal link |
| `CodeBlockChange` | Code block modification |
