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
use darkmatter::markdown::frontmatter::MergeStrategy;

// Error on conflict (strict)
md.fm_merge_with(json!({"tags": ["rust"]}), MergeStrategy::ErrorOnConflict)?;

// External values win on conflict
md.fm_merge_with(json!({"status": "published"}), MergeStrategy::PreferExternal)?;

// Document values win on conflict (set defaults)
md.fm_set_defaults(json!({"draft": false}))?;
```

## Available Strategies

| Strategy | Behavior |
|----------|----------|
| `ErrorOnConflict` | Fail if keys exist in both |
| `PreferExternal` | Incoming values win on conflict |
| `PreferDocument` | Existing document values win |

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
