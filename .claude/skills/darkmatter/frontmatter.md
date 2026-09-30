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

Parsing one YAML frontmatter document rejects duplicate keys at every mapping
depth, including mappings nested in arrays. Duplicate-looking text inside an
expression string remains scalar content. This replaces YAML last-value-wins
behavior for authored duplicates; it does not change explicit merging between
separate documents, which still follows the selected strategy below.

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

## Stamping a Baseline

`Markdown::stamp_baseline(source, &opts, now, change)` is the single entry point
for writing a document's `hash` and `last_updated` together. It parses the
stored hash (`Markdown::stored_hash`), plans the save, applies the `Change`
policy, and stamps the UTC date of `now`. `Change::Detect` bumps `last_updated`
only when the hash moved (`md hash --save`); `Change::Known` bumps it because
the caller just edited the content (effects auto-rehash, Claudine's inline
closure). Do not call `plan_hash_save` plus `apply_hash_save_text` directly in
a new writer.

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

`restore_properties_text` needs parseable YAML. A consumer repairing
malformed YAML (Claudine's inline closure) repairs first and restores second.

## Literal Tokens

A frontmatter string an operation produced (agent output, an effect write) is
persisted as **data** so a later compose does not read it as a template or
shell command. The on-disk form is one whole-leaf, double-quoted scalar:

```yaml
summary: "{{!data:v1:Zml4ZWQge3sgYXJlYSB9fSBwYXJzaW5n}}"  # "fixed {{ area }} parsing"
```

The payload is the value's UTF-8 bytes in unpadded URL-safe base64. Decoding
is canonical, and `{{!data:v1:}}` is the empty string. The API lives in
`darkmatter::markdown::literal_token`:

| Item | Use |
| ---- | --- |
| `encode(&str)` / `encode_yaml_scalar(&str)` | Bare token or the always-double-quoted YAML scalar. Round-trips every Unicode string. |
| `decode(&str)` | One bare token to its text; `TokenError` otherwise. |
| `decode_leaf(&str)` | `None` (no token), `Some(Ok(text))` (whole-leaf token), or `Some(Err(..))` (malformed or `Embedded`). |
| `decode_literal_tokens(&Value)` | Decodes a loaded tree; `LiteralTokenError` carries the dotted path. |
| `Frontmatter::decoded_literal_tokens()` | The per-document convenience. |
| `holds_pending_syntax(&str)` | Lexical "still a template?" (`{{` or `$(` and not a whole valid token). A caller that already knows a value is data must not ask. |
| `TOKEN_PREFIX`, `TOKEN_VERSION` | `{{!data:` and `v1`. |

Rules consumers depend on:

- **Loaders keep tokens encoded, and readers must decode.** `fm_get`,
  `fm_parse`, and the raw YAML all show the token. Schema checks, reports, and
  handoffs call `decode_literal_tokens`. Never feed decoded values back to
  composition as authored text; use `ComposeOptions::with_data_overrides`
  instead (see [compose.md](compose.md#inserted-text-is-data)). A tool outside
  Darkmatter and Claudine that reads the YAML directly sees the token.
- **The cross-document expression readers decode.** `frontmatter(path[,
  prop])` and `markdown_title(path)` return a stored token as its text
  (data, like any expression result); a malformed token comes back raw rather
  than failing the expression.
- **Composition decodes once.** Frontmatter pass 1 decodes each whole
  authored leaf, and the text is data from then on. A malformed or embedded
  token fails with `ExpressionError::MalformedLiteralToken` and its line and
  column. It never falls through to expression or shell parsing.
- **Encode by origin, never by appearance.** A raw string that already looks
  like a token is encoded again. An unchanged stored token is left alone.
- **Hand edits.** Replacing the token with plain text makes the value an
  authored template again. To keep it as data, decode, edit, and re-encode
  it. An author writes the token spelling as literal text with
  `{{{!data:…}}}`.

### Locating a Leaf for an In-Place Edit

`hash::locate_frontmatter_leaves(document, &[Vec<FrontmatterPathSegment>])`
returns one `LeafSpan { path, range, decoded }` per requested string leaf.
`range` is the scalar's absolute byte range: quotes and a block scalar's
header are included, and the final line break is excluded. Splicing an
encoded scalar into exactly that range leaves every other byte and line
ending intact. This, with `restore_properties_text`, is the sanctioned
text-preserving writer. Do not build another YAML editor.

- Each path is located within its own top-level property. An unmodeled
  construct elsewhere never blocks it.
- A span is returned only when its decoded text equals what compose reads.
  For a clipped block scalar ending the frontmatter, `decoded` therefore lacks
  the trailing newline.
- Failure is all-or-nothing. `LeafLocateError::Document` means the
  frontmatter is not a parseable block mapping. `LeafLocateError::Unlocated`
  is the first path that failed, as an `UnlocatedLeaf { path, line, reason }`.
  Its `UnlocatedLeafReason` is `Missing`, `NotAScalar`, `NodeProperties`
  (anchor, alias, tag, or `<<` merge), or `UnsupportedShape` (a plain
  flow-collection item, a multi-line flow collection, or a nested sequence).
  Report the line rather than falling back to re-serializing the document.

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
