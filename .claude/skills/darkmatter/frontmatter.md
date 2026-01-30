# Frontmatter Operations

YAML frontmatter parsing with typed access and merge strategies.

## Basic Usage

```rust
let mut md: Markdown = content.into();

// Typed access
let title: Option<String> = md.fm_get("title")?;

// Insert values
md.fm_insert("version", "1.0")?;

// Check existence
if md.has_frontmatter() {
    // ...
}
```

## Merge Strategies

```rust
use darkmatter_lib::markdown::frontmatter::MergeStrategy;

// Merge with strategy
md.fm_merge_with(json!({"tags": ["rust"]}), MergeStrategy::ErrorOnConflict)?;

// Set defaults (document wins on conflict)
md.fm_set_defaults(json!({"draft": false}))?;

// Override (new values win)
md.fm_merge_with(json!({"status": "published"}), MergeStrategy::Override)?;
```

## Available Strategies

| Strategy | Behavior |
|----------|----------|
| `MergeStrategy::ErrorOnConflict` | Fail if keys conflict |
| `MergeStrategy::Override` | New values win |
| `MergeStrategy::Preserve` | Existing values win |
| `MergeStrategy::Append` | Append arrays, merge objects |

## Typed Extraction

```rust
#[derive(Deserialize)]
struct PostMeta {
    title: String,
    date: String,
    tags: Vec<String>,
}

let meta: PostMeta = md.fm_parse()?;
```
