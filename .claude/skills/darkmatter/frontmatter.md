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
closure). Advancing `last_updated` renews every content policy whose baseline
is `@last_updated`; see the `content-policy` skill. Do not call
`plan_hash_save` plus `apply_hash_save_text` directly in a new writer.

## Text-Preserving Property Restoration

`restore_properties_text(current, snapshot, properties)` sits beside
`apply_hash_save_text` and is the only sanctioned way to put a named set of
frontmatter properties back to their snapshot spelling. A consumer that wants
to overwrite specific keys without reformatting the document must call this
rather than build a second YAML node editor.

It returns `RestoredDocument { text, restored_properties, frontmatter_delta }`.
Every untouched byte survives — block scalars, trailing spaces, four-space
indentation, LF/CRLF, property order, and platform-native paths. Parse
failures, a non-mapping root, duplicate keys, and a restoration that would
change the parsed value of a property outside `properties` (a restored node
adds or drops an anchor that an unowned alias resolves to) are typed errors and
nothing is written. The `FrontmatterDelta` distinguishes addition, replacement, and
deletion; value-preserving reformatting is **not** a semantic change, and the
restored properties are excluded from the delta.

For "did the body meaningfully change", use the non-strict `Simple` body hash
comparison: leading/trailing whitespace and blank lines are ignored, internal
whitespace stays significant.

`restore_properties_text` needs parseable YAML. A consumer repairing
malformed YAML (Claudine's inline closure) repairs first and restores second.

Restore still picks **one** newline for the whole file (`detect_newline`) and
prepends a new block before a leading BOM; only the hash writer below has the
per-line rules.

## Text-Preserving Hash Save

`apply_hash_save_text` (behind `md hash --save` and Claudine's closure
write-back) changes only the managed `hash` node and, on a bump,
`last_updated`. Its terminator rules are per line, never per file:

- replacement `hash` line *i* takes original node line *i*'s terminator; extra
  lines repeat the previous line's;
- an inserted property takes the terminator of the line before it (the opening
  `---` for empty frontmatter);
- a new block goes after a leading BOM and uses the first line's terminator, or
  LF;
- an empty `last_updated:` becomes `last_updated: {today}`, keeping a comment
  (one space is supplied before `#` when none was authored); `~` and `null`
  are ordinary values;
- a scalar date replaces only its value bytes, after the colon or alone on
  the first following non-blank, non-comment line; indented comment and blank
  lines in the node are kept. The parsed value type, never the node's line
  count, decides whether `last_updated` is a collection (`rewrite_date_scalar`
  takes the parsed value). A quote is quoting syntax only as a scalar's first
  character (`quoted_scalar_end`): in `yesterday's date   # why` the `#` still
  starts the kept comment, and a key like `author's note:` still has its
  colon found. A key's separator is the `:` YAML reads as a value indicator
  (followed by whitespace or line end in block context), so `a:b: c` has the
  key `a:b`, not `a`. `mapping_colon` delegates to the public
  `schemas::mapping_separator`, which nested keys, flow entries, and the
  position maps (`validate::build_position_map`, `style` positions, DMLS,
  Claudine excerpts) share; do not split a YAML key line at its first `:`.

It returns `MarkdownError::FrontmatterTextEdit` and no text when a bump meets a
`last_updated` with an anchor, alias, or tag (the shared
`leading_node_property` predicate, also used by `locate_frontmatter_leaves`),
a value that parses to a sequence or mapping, or a scalar it cannot rewrite
on one line (a block scalar, or a plain or quoted value continued across
lines, detected when the value's bytes parsed alone differ from the parsed
value); when the rewritten frontmatter fails
`parse_text_frontmatter` (for example, a replaced `hash` orphans an alias); and
when any property other than the managed hash and a bumped `last_updated`
parses to a different value than before (`ensure_unmanaged_values_kept`).
Parsing alone is not enough: YAML permits reusing an anchor name, so removing
the `hash` node's `&h` can re-point a later `*h` at an earlier `&h` and still
parse. A decision with no new stored hash returns `None` without parsing.

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
- A quoted flow item is located whatever its plain siblings or keys hold:
  `[don't, "{{x}}"]`, `{a: say "hi, b: '{{x}}'}`, `[a:'b, "{{x}}"]`, and
  `{a:'b: "{{x}}"}` split as YAML does. Raw YAML flow text goes through
  `flow_collection_end` and `split_flow_entries` in
  `schemas/simplified/source.rs`, both built on `scan_raw_yaml`: a quote opens
  only at a scalar's start, and a `:` starts a new scalar only as a value
  indicator (followed by whitespace or a flow indicator, or right after a
  quoted or collection key as in `{"a":b}`). Inside a plain scalar such as
  `a:'b` or `http://x` the colon, and the quote after it, are content. Only
  `[`/`{` nest: a parenthesis is plain-scalar content, so `[a(b, "{{x}}"]`
  has two entries and `[a(b, c)d, "{{x}}"]` three. The schema-declaration
  cursor splits its opaque file-reference arms with `split_flow_entries` too.
- Type-expression text is read in the grammar's lexical modes, never by
  delimiter depth. "Inside parentheses" is not "inside an argument list": a
  description after `->` and a file reference after `@` can hold parentheses.
  `grammar.rs` states each mode's boundary once (`quoted_end`,
  `inline_description_end`, `file_reference_end`, `pattern_key_end`), and the
  parser, `scan_expression` in `source.rs`, and the type-definition cursor all
  call them. Only an argument list gives quotes meaning (`\` escapes in either
  style; `[`/`{` are argument text). Description prose ignores quotes and
  brackets and, inside an inline object, nests only `()`/`{}` before its `,`
  or `}`; a top-level description runs to the end. A file reference is opaque
  up to `,`, `}`, or `->`, so `Name@./a{b.yaml -> d` references `./a{b.yaml`.
  A `<…>` pattern key is opaque to its first `>`. `scan_expression` takes an
  `ExpressionContext` (`Expression`, `FlowArm`, `ObjectBody`, `Arguments`)
  and visits only type text and argument text outside strings, with an
  `ExpressionLevel { objects, groups }`. `find_top_level_*`,
  `split_top_level`, `inline_object_end`, `scan_constraint_groups`, and
  `constraint_call_arguments` use it on decoded expressions; never point them
  at raw YAML. The type-definition cursor splits flow arms in two layers: a
  quote at an arm's start is a YAML quoted scalar (`['enum("a)b", c)', str`),
  and the rest is scanned as `FlowArm`, where YAML ends a top-level
  description at `,`/`]`, so `[enum('a)b', c), str` and
  `[string -> (it's fine), s` both have two arms. `scan_scalar` resumes type
  context after an inline description's `,` and keeps the import-reference
  role across filename punctuation. Test source maps with
  `SchemaSourceMap::entries()` so a silently merged property fails.
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
