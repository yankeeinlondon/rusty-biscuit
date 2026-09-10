# Frontmatter Model and Operations

Darkmatter retains YAML frontmatter as structured values plus raw source needed
for source-aware diagnostics. Use this topic for typed library access, merge
policy, and the `md frontmatter` CLI. Use [schema.md](schema.md) for validation
and coercion, and [rendering.md](rendering.md) when a `style:` value is being
lowered into render policy.

## Basic Usage

```rust
use darkmatter::Markdown;

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
use serde_json::json;

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
use serde::Deserialize;

#[derive(Deserialize)]
struct PostMeta {
    title: String,
    date: String,
    tags: Vec<String>,
}

let meta: PostMeta = md.fm_parse()?;
```

## CLI Operations

Use `md frontmatter get`, `set`, and `rm` for structured changes. These commands
parse and serialize frontmatter rather than editing YAML as text. When a
document carries a managed `hash:` property, use the command's normal write
path so Darkmatter can keep the stored hash consistent.

## Text-Preserving Property Restoration

`restore_properties_text(current, snapshot, properties)` sits beside
`apply_hash_save_text` and is the only sanctioned way to put a named set of
frontmatter properties back to their snapshot spelling. A consumer that wants
to overwrite specific keys without reformatting the document must call this
rather than build a second YAML node editor.

It returns `RestoredDocument { text, restored_properties, frontmatter_delta }`.
Every untouched byte survives — block scalars, trailing spaces, four-space
indentation, LF/CRLF, property order, and platform-native paths. Parse
failures, a non-mapping root, and duplicate keys are typed errors and nothing
is written. The `FrontmatterDelta` distinguishes addition, replacement, and
deletion; value-preserving reformatting is **not** a semantic change, and the
restored properties are excluded from the delta.

For "did the body meaningfully change", use the non-strict `Simple` body hash
comparison: leading/trailing whitespace and blank lines are ignored, internal
whitespace stays significant.

## `style:` Frontmatter

`darkmatter::style` owns the document-level `style:` schema and applicators.
It lowers component policy onto render-tree nodes and page-level concerns onto
the `DarkmatterPage` frame. It remains separate from
`renderable::style::Style` on individual nodes.

The supported surface includes:

- page layout, background, color, stylesheet, meta, and code theme
- table, image, block-quote, `ul`, `ol`, `li`, and HR layout/color policy
- `style.hr.*` as the canonical horizontal-rule namespace
- hyperlink style plus local hyperlink/image style overrides

`KnownButInactive` should be empty for valid v1 schema keys. `--strict-style`
promotes unknown and deprecated keys to errors, while valid unsupported
combinations fail through documented `StyleApplyError` variants.

Read [rendering.md](rendering.md) before changing style claims, component
policy, page framing, code-block themes, or target folds.
