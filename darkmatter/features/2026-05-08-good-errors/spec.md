# Specification: Good Errors

This document defines a structural fix for darkmatter's error rendering so that user-facing diagnostics are consistently rich, properly styled, and actionable. It targets two coupled defects: (1) Prose markup leaks through `StatusBlock` bodies as literal text, and (2) error variants lack the source context required to render the error a reader can act on.

## 1. Problem Statement

A representative bad output observed in the wild:

```
┃ <dim>Opened at line:</dim> 14
┃ <dim>Opening directive:</dim> ::block when="x"
┃ <dim>File ends at line:</dim> 50
```

Three faults are stacked here:

1. **Bare markup leak.** `<dim>...</dim>` is rendered literally instead of as styled text. The body content was passed as a `String`, which `BlockQuote` renders as plain text without invoking `Prose`.
2. **Line numbers without a code excerpt are noise.** A bare line number is meaningless when the document was composed (transcluded), and even for a non-composed document the reader has to switch context to a file viewer to interpret it.
3. **Missing actionable context.** No file path (and therefore no link), no frontmatter snapshot, no excerpt of the offending region.

The root causes are:

- `StatusBlock::body()` accepts `impl Into<RenderableContent>`. A `String` becomes `RenderableContent::String`, which `BlockQuote` (`biscuit-terminal/lib/src/components/block_quote.rs:185`) emits unchanged. Prose grammar is never parsed for the body. The same footgun exists for any caller that hands `body()` a string with markup tags.
- darkmatter error variants (e.g. `PageBlockError::UnterminatedBlock`) carry only `{ line, opening_text, file_ends_at_line }`. They have no source path and no source content, so even a corrected renderer cannot produce a useful excerpt.
- Existing tests assert `out.contains("unterminated")`, which does not catch tag leaks. Regressions slip through silently.

This pattern recurs in **6 darkmatter error files** (`page_blocks/types.rs`, `transclusion/types.rs`, `reference/errors.rs`, `render/stylesheet.rs`, `render/image_ref.rs`, `render/link.rs`). Fixing one variant in isolation will not stop the bug from reappearing.

## 2. Goals and Non-Goals

### 2.1 Goals

- Make it impossible for an error body to leak Prose markup as literal text.
- Provide a reusable mechanism for rendering source excerpts (file link header + frontmatter snapshot + N lines around the offending line) in error blocks.
- Establish a snapshot-test pattern that keeps rendered errors visually reviewable.
- Migrate `PageBlockError::UnterminatedBlock` end-to-end as the canonical reference, then sweep the remaining 5 darkmatter error files.

### 2.2 Non-Goals

- Source-mapping for composed (transcluded) documents. Line numbers reported from a virtual document remain potentially misleading. This spec acknowledges the gap and treats it as a follow-up feature; this work does not introduce a `(virtual_line -> source_file:line)` map.
- Migrating other crates (`claudine`, `schematic`, `sniff`) to the new pattern. The reusable helpers will live in `biscuit-terminal` so other crates can adopt them, but adoption outside darkmatter is not part of this feature.
- Replacing `thiserror` or restructuring darkmatter's error enum hierarchy.

## 3. Requirements

### 3.1 Output Quality Bar

Every error rendered through `StatusBlock` MUST satisfy the following on a typical 80-column terminal:

1. **No bare markup.** No `<...>` tag ever appears as literal output unless it is the actual user content being displayed (e.g. a malformed HTML attribute the parser quoted back).
2. **Linked file path** when the error has a source location. The header includes a `<blue><a href={absolute}>{relative}</a></blue>` segment using OSC 8 hyperlinks supplied by Prose.
3. **Frontmatter snapshot** when the error originated in a document with frontmatter, rendered as a fenced `yaml` code block.
4. **Source excerpt** when the error has a line location, showing the offending line with up to two lines of context above and below, rendered as a fenced code block whose language tag matches the source file (`md` for Markdown).
5. **Hint** below the body, written as Prose, restating the corrective action in one sentence.

### 3.2 API Contract: `StatusBlock::body`

The `StatusBlock::body` method is changed to accept a vector of `Prose` items that stack vertically with a single blank line between them.

- Old signature: `fn body(self, body: impl Into<RenderableContent>) -> Self`
- New signature: `fn body(self, body: impl Into<Vec<Prose>>) -> Self`

Convenience conversions are provided so simple call sites stay ergonomic:

- `From<Prose> for Vec<Prose>` (single paragraph)
- `From<&str> for Prose` and `From<String> for Prose` already exist; combined with the conversion above, `body("plain text")` continues to compile and now correctly Prose-parses any markup.

A separate single-item shortcut is provided for the common case where the error body is a single styled line:

- `fn body_line(self, line: impl Into<Prose>) -> Self` — wraps the provided Prose into `vec![prose]` and calls `body`.

This is a breaking change. All call sites must be migrated. The migration is mechanical because the existing footgun makes the breakage immediately visible (literal tags become rendered styles).

### 3.3 Source-Aware Error Trait

`biscuit-terminal` introduces a new `SourceContext` value type and a companion `SourceExcerpt` Prose-builder helper:

```rust
// biscuit-terminal/lib/src/errors/source_context.rs

use std::path::PathBuf;
use std::sync::Arc;

/// Resolved source context for an error that originates in a file.
#[derive(Debug, Clone)]
pub struct SourceContext {
    /// Absolute path used for OSC 8 hyperlinks.
    pub absolute: PathBuf,
    /// Display path (typically relative to repo or cwd) for the visible label.
    pub display: PathBuf,
    /// Full source content. Shared via Arc to keep error variants cheap to clone.
    pub content: Arc<str>,
    /// Byte range of frontmatter in `content`, if present.
    pub frontmatter: Option<std::ops::Range<usize>>,
}
```

`SourceContext` is purely data. Rendering helpers live alongside it but are pure functions producing `Prose`:

```rust
impl SourceContext {
    /// Render a `<blue><a href=ABSOLUTE>RELATIVE</a></blue>` Prose segment
    /// for use in error headers.
    pub fn linked_path_prose(&self) -> Prose;

    /// Render the frontmatter as a fenced ```yaml block, or None if absent.
    pub fn frontmatter_prose(&self) -> Option<Prose>;

    /// Render an excerpt centered on `line` (1-based), with `context` lines
    /// above and below, as a fenced code block tagged with `lang`.
    /// The offending line is marked with a leading `>` gutter.
    pub fn excerpt_prose(&self, line: usize, context: usize, lang: &str) -> Prose;
}
```

Prose gains a minimal fenced code block grammar for these helpers. See §4.2.

### 3.4 Enriched Error Variants

Error variants that have a file origin gain a `source: SourceContext` field. The reference migration is `PageBlockError::UnterminatedBlock`:

```rust
PageBlockError::UnterminatedBlock {
    source: SourceContext,
    opening_line: usize,
    opening_text: String,
}
```

`file_ends_at_line` is dropped — it is derivable from `source.content` and the rendered excerpt makes it obvious where the file ends. `line` is renamed to `opening_line` for clarity.

The full sweep migrates the other five files in Phase 2 (§5).

### 3.5 Reference Render Output

The `BlockError::status_block` impl for `UnterminatedBlock` produces the following structure (rendered shape, not literal source):

- **Header**: `error PageBlockError: The file <linked path> has an unterminated ::block / ::end-block block definition.`
- **Body**, as `Vec<Prose>`:
  1. "The Frontmatter of this document was:"
  2. The frontmatter as a fenced `yaml` code block (omitted entirely when absent).
  3. "The opening page block was found here:"
  4. The source excerpt as a fenced `md` code block, centered on `opening_line` with 2 lines of context above and below, with the offending line gutter-marked.
- **Hint**: `Add a matching <inverse>::end-block</inverse> directive to close the region.`

`<inverse>` is used in the hint for the literal directive token, matching the user's stated preference for highlighting directive names.

### 3.6 Snapshot Tests

Snapshot testing is added using the `insta` crate (already in workspace use elsewhere). Each `BlockError` variant gains a snapshot test that:

1. Constructs the error with a representative payload.
2. Renders it via `report_block_error_optimistic(Some(80))`.
3. Strips ANSI escape codes (use existing `biscuit_terminal::utils::escape_codes::strip_escape_codes`).
4. Asserts the result matches a checked-in snapshot.

This is the gate that prevents the bare-markup regression. Existing `assert!(out.contains(...))` tests stay as smoke checks but the snapshot is the authority.

### 3.7 Documentation Update

A new authoritative document is added at `darkmatter/docs/errors/README.md` describing:

- The body-is-`Vec<Prose>` contract.
- The `SourceContext` requirement for any error with a file origin.
- The standard structure (linked header → frontmatter → excerpt → hint).
- The snapshot test requirement.

The existing skill at `.claude/skills/schematic-define/SKILL.md` is not affected; a new skill stub at `.claude/skills/darkmatter/errors.md` (or amendment of an existing darkmatter skill) is created in Phase 2.

## 4. Technical Design

### 4.1 `StatusBlock::body` Change

`StatusBlock`'s field changes from `body: Option<RenderableContent>` to `body: Vec<Prose>`. The `render` impl iterates the vector and emits each Prose item inside a `BlockQuote`, separated by a single blank line wrapped with the same border glyph so the visual block stays continuous.

```rust
pub struct StatusBlock {
    severity: StatusState,
    header: Option<String>,
    body: Vec<Prose>,           // was: Option<RenderableContent>
    hint: Option<String>,
    border_color: Option<Color>,
    border: String,
    layout: Layout,
}
```

Render flow for the body:

```rust
if !self.body.is_empty() {
    let composed = self.body.iter()
        .map(|p| p.render(term))
        .collect::<Vec<_>>()
        .join("\n\n");
    let mut block = BlockQuote::new(RenderableContent::String(composed), None::<&str>)
        .with_left_block_color(self.resolved_border_color())
        .with_border(&self.border);
    // ... apply layout
    parts.push(block.render(term));
}
```

Rendering each Prose individually before stacking guarantees the markup is parsed. The `BlockQuote` then receives an already-rendered string and only handles wrapping and the gutter glyph.

### 4.2 Prose Fenced Code Block Grammar

`Prose` currently supports inline grammars only (`<bold>`, `**bold**`, `[label](url)`). A minimal block-level fenced code grammar is added:

- Opening fence: ` ```LANG\n ` at the start of a line.
- Closing fence: ` ``` ` at the start of a line.
- Body is rendered with line preservation, using the existing TextBlock with a dim foreground color and a 2-space indent. Syntax highlighting is **not** required in this feature — language tag is stored but unused for now (open question in §7).

Rationale: the user's `Vec<Prose>` answer commits the body to a homogeneous Prose vector. Without fenced code support, frontmatter and source excerpts would have to live as a separate component type. Adding the grammar is the cheaper path and stays inside Prose's existing markdown-subset story.

### 4.3 `SourceContext` Construction Sites

`SourceContext` is built at the parse boundary, where the file path and full content are already in hand:

- Page block parser: `darkmatter/lib/src/markdown/compose/page_blocks/parser.rs` — already receives the source text. Threading the `PathBuf` through the parser entry point is required.
- Transclusion engine: `darkmatter/lib/src/markdown/compose/transclusion/engine.rs` — already tracks file paths through the include chain.
- Reference parser, image-ref parser, link parser, stylesheet parser — each receives a path and content; threading is mechanical.

The full content is wrapped in `Arc<str>` so cloning into error variants is cheap.

### 4.4 Frontmatter Range Extraction

`SourceContext::frontmatter` is populated by reusing darkmatter's existing frontmatter detection (`darkmatter/lib/src/markdown/frontmatter`). The byte range, not the parsed value, is stored. This keeps the rendered output verbatim with the source and avoids any re-serialization rounding.

### 4.5 Excerpt Rendering Algorithm

```rust
fn excerpt_prose(&self, line: usize, context: usize, lang: &str) -> Prose {
    let lines: Vec<&str> = self.content.lines().collect();
    let total = lines.len();
    let start = line.saturating_sub(context + 1);
    let end = (line + context).min(total);
    let width = end.to_string().len();

    let mut buf = String::from("```");
    buf.push_str(lang);
    buf.push('\n');
    for (idx, l) in lines[start..end].iter().enumerate() {
        let n = start + idx + 1;
        let gutter = if n == line { ">" } else { " " };
        writeln!(buf, "{gutter} {n:>width$} │ {l}", width = width).unwrap();
    }
    buf.push_str("```");
    Prose::new(buf)
}
```

The `>` gutter on the offending line provides a visual anchor without relying on color (which may be disabled).

### 4.6 Test Strategy

- **Unit (existing)**: `assert!(out.contains(...))` smoke checks remain.
- **Snapshot (new)**: `insta` snapshots per `BlockError` variant, stored under `darkmatter/lib/tests/snapshots/`.
- **Coverage gate**: The Phase 2 sweep is not complete until every variant has a snapshot.
- **CI**: `cargo insta test --review` is part of the developer loop; CI runs `cargo insta test` (no `--accept`).

## 5. Migration Plan

### Phase 1 — Reference Implementation

In scope:

1. `StatusBlock::body` API change in `biscuit-terminal`.
2. `SourceContext` type and its three Prose-builder helpers in `biscuit-terminal`.
3. Prose fenced code block grammar in `biscuit-terminal`.
4. `PageBlockError::UnterminatedBlock` migrated end-to-end (variant, parser threading, render impl, snapshot test).
5. `darkmatter/docs/errors/README.md` authored.
6. All other `StatusBlock::body` call sites in the workspace updated to compile against the new signature. For non-darkmatter call sites, the conversion is mechanical: replace `body(format!(...))` with `body_line(format!(...))` (the format string still goes through Prose, so any pre-existing markup tags now render correctly — this is intentional improvement, not regression).

Out of scope for Phase 1:

- Migrating other darkmatter `BlockError` variants to use `SourceContext`. Their existing render impls are updated only as much as needed to compile against the new `body` signature — typically just wrapping the format string in `Prose::new`.

Phase 1 ships when:

- `cargo test -p biscuit-terminal` and `cargo test -p darkmatter` pass.
- The `UnterminatedBlock` snapshot is checked in and visually approved.
- No call site in the workspace emits literal `<dim>` or `<cyan>` for a known input.

### Phase 2 — Sweep

In scope:

1. The five remaining darkmatter error files migrated to carry `SourceContext` where applicable:
   - `darkmatter/lib/src/markdown/compose/transclusion/types.rs`
   - `darkmatter/lib/src/markdown/reference/errors.rs`
   - `darkmatter/lib/src/render/stylesheet.rs`
   - `darkmatter/lib/src/render/image_ref.rs`
   - `darkmatter/lib/src/render/link.rs`
2. Snapshot tests for every variant in those files.
3. The remaining `PageBlockError` variants (`ParseDirective`, `UnmatchedEnd`, `Condition`).
4. Skill update at `.claude/skills/darkmatter/errors.md` documenting the contract for future contributors.

Phase 2 is its own plan and may ship in a separate PR. It does not block Phase 1.

## 6. Open Questions

1. **Syntax highlighting in code blocks.** Current spec dims the excerpt body without highlighting. `two-face` and `syntect` are already on the dependency tree elsewhere. Adding minimal highlighting (yaml + md only, gated behind a feature flag) is a small follow-up. Decision deferred until Phase 1 ships and the visual baseline is established.

2. **Composed-document line-number caveat phrasing.** When an error originates from a transcluded document, the spec currently shows the line number from the enclosing parser without indicating it is a virtual line. Until the source-map feature lands, should the rendered excerpt prefix include a marker like `(composed: line N — source file may differ)`? Phase 1 omits this; revisit after real usage.

3. **`SourceContext` in non-file-origin errors.** Errors like `MarkdownError::Transform("pipeline stalled")` have no source file. They keep their current single-line `body_line` rendering. No `SourceContext` field is added.

4. **Frontmatter visibility threshold.** Long frontmatter (e.g. 200 lines) makes the error block unreadable. Spec currently emits the full frontmatter verbatim. A truncation rule (e.g. "first 30 lines + … + last 5") may be needed; deferred until a real document forces the issue.

## 7. Drift-Maintenance Checklist

When this feature ships, update:

- `darkmatter/README.md` — link to the new `docs/errors/README.md`.
- `darkmatter/docs/errors/README.md` — authored in this feature.
- `biscuit-terminal/README.md` — note the `StatusBlock::body` signature change and the new `SourceContext` type.
- `.claude/skills/darkmatter/errors.md` — Phase 2 deliverable.
- `darkmatter/CLAUDE.md` (if present) — error-rendering convention summary.
