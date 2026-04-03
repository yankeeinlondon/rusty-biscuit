# Inline Compose Frontmatter Rules

## Problem

The `claudine inline-compose` closure currently reconstructs the document using the original frontmatter byte-for-byte, only modifying `last_updated`. This has two gaps:

1. **New properties lost**: If the agent adds a new frontmatter property (e.g., `tags: [research]`), it is silently discarded when the closure overwrites with the original frontmatter.
2. **No visibility into agent modifications**: If the agent modifies an existing property (e.g., changes `prompt`), the closure silently reverts it with no warning.

Additionally, a bug was found where `cleanup_inline_output` passed the entire file (including frontmatter) to `cleanup_content`, corrupting YAML block scalars. This bug has been fixed separately by splitting frontmatter from body before cleanup.

## Business Rules

When inline-compose completes successfully:

1. `last_updated` is set to the current date (local time, `YYYY-MM-DD` format)
2. All frontmatter properties the document started with are preserved at their original values
3. New frontmatter properties added by the agent during execution are merged into the document
4. When an existing property was modified by the agent, the original value is restored and a warning is emitted

## Design

### Library Changes (`claudine/lib/src/composition/closure.rs`)

**New return type:**

```rust
pub struct InlineClosureResult {
    /// Keys that were added by the agent and merged into the document.
    pub new_properties: Vec<String>,
    /// Keys that were modified by the agent and reverted to original values.
    pub reverted_properties: Vec<String>,
}
```

**Modified `apply_inline_closure` signature:**

```rust
pub fn apply_inline_closure(
    plan: &InlineClosurePlan,
    replacement_body: &str,
    target_path: &Path,
    today: &str,
    post_run_frontmatter: Option<&IndexMap<String, serde_json::Value>>,
) -> Result<InlineClosureResult, CompositionError>
```

When `post_run_frontmatter` is `Some`:
- Compare keys against the original frontmatter (parsed from `plan.original_document_text`)
- **New keys** (in post-run, not in original): serialize as YAML lines and pass to `rewrite_inline_document` for injection
- **Modified keys** (in both, values differ): collect into `reverted_properties`
- **Unchanged/removed keys**: no action

When `post_run_frontmatter` is `None`: behave as today (no merge, empty result vectors).

**Modified `rewrite_inline_document` signature:**

```rust
pub fn rewrite_inline_document(
    frontmatter_source: &str,
    body: &str,
    today: &str,
    new_properties: &[(String, String)],  // (key, serialized YAML line(s))
) -> Result<String, String>
```

New properties are injected just before `last_updated` in the raw YAML text. If `last_updated` doesn't exist yet, new properties go at the end of the existing YAML, followed by the appended `last_updated`.

**New helper function:**

```rust
fn serialize_frontmatter_property(key: &str, value: &serde_json::Value) -> String
```

Returns a complete YAML fragment with trailing newline. Simple scalars produce `key: value\n`. Complex types (arrays, objects) delegate to `serde_yaml_ng` for the value portion.

**Modified `upsert_last_updated_in_frontmatter`:**

Gains a `new_properties: &[(String, String)]` parameter. After processing all original lines, injects new property lines before appending/leaving `last_updated` at the end.

### CLI Changes

**Both paths** (non-harness in `composition.rs` and harness in `mod.rs`):

1. After the agent exits, read the post-run file from disk
2. Parse as `Markdown` to extract its frontmatter map
3. Pass the map to `apply_inline_closure`
4. From the `InlineClosureResult`:
   - For each reverted property, emit a warning via `Status::from_prose` with `StatusState::Warning` and `StatusTheme::Circular`
   - For new properties, emit a `check_ok` line mentioning the merged key(s)

**Warning format:**

```rust
Status::from_prose(
    format!("Agent modified frontmatter property <b>\"{key}\"</b> — reverted to original value")
)
.state(StatusState::Warning)
```

**Edge case:** If the post-run file can't be read (deleted, permissions issue), skip comparison and proceed with original closure behavior (no merge, no warnings). Best-effort enhancement.

### Property Positioning

New properties are inserted just before `last_updated`, which always remains the final property in the YAML block. This maintains a consistent convention.

## Files Changed

| File | Change |
|------|--------|
| `claudine/lib/src/composition/closure.rs` | New `InlineClosureResult` type, modified `apply_inline_closure` and `rewrite_inline_document` signatures, new `serialize_frontmatter_property` helper, modified `upsert_last_updated_in_frontmatter` |
| `claudine/cli/src/commands/wrap/composition.rs` | Post-run frontmatter reading and comparison, warning output, pass post-run map to closure |
| `claudine/cli/src/commands/wrap/mod.rs` | Same changes in the harness `try_inline_closure` path |

## Testing

- Unit tests in `closure.rs` for:
  - New properties merged before `last_updated`
  - Modified properties detected and reported (original values preserved)
  - Complex value serialization (arrays, objects)
  - `None` post-run frontmatter behaves as before (backward compatible)
  - Empty new-properties list doesn't modify YAML
- Unit tests in `composition.rs` for:
  - `split_frontmatter_and_body` (already added with the cleanup fix)
