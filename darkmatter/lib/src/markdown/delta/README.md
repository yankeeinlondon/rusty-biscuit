# Markdown Delta

Structural diff analysis between markdown documents.

## Features

- **Change Classification**: Identical, WhitespaceOnly, MinorEdit, ContentUpdate, StructuralChange, MajorRewrite
- **Content Changes**: Track additions, removals, and modifications at section level
- **Section Movements**: Detect when sections are relocated within the document
- **Frontmatter Changes**: Track YAML frontmatter additions, removals, and modifications
- **Broken Link Detection**: Identify internal links that may be broken after changes
- **Code Block Changes**: Track changes to fenced code blocks
- **Visual Diff**: Unified and side-by-side diff output for terminal

## Usage

```rust
use darkmatter_lib::markdown::Markdown;

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
| `DocumentChange` | Change classification enum |
| `DeltaStatistics` | Numeric change counts |
| `ContentChange` | Individual content change |
| `ChangeAction` | Added, Removed, Modified |
| `MovedSection` | Section relocation info |
| `FrontmatterChange` | Frontmatter key change |
| `BrokenLink` | Potentially broken internal link |
| `CodeBlockChange` | Code block modification |
