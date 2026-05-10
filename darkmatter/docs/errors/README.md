# Error Rendering Conventions

Darkmatter renders user-facing diagnostics through the `BlockError` trait
defined in `biscuit-terminal`. The structural rules below are enforced for
every variant so that errors are consistently rich, actionable, and
visually reviewable.

## Body Contract: `Vec<Prose>`

`StatusBlock::body` accepts a `Vec<Prose>` (or anything that implements
`IntoProseVec`). Each item is rendered through Prose individually and
stacked vertically with one blank line between items. This guarantees that
markup tags like `<dim>`, `<cyan>`, and `<inverse>` are always parsed —
never leaked as literal text.

Two ergonomic shortcuts exist:

- `StatusBlock::body_line(prose)` — convenience for a single Prose
  paragraph.
- `From<&str>` / `From<String>` for `Prose`, combined with
  `From<Prose> for Vec<Prose>`, lets `body("plain text")` keep working.

Anything user-supplied that may contain `_`, `*`, or other markdown
sentinels must be escaped (e.g. `value.replace('_', "\\_")`) before being
embedded in a Prose string, otherwise it will be reinterpreted as
formatting markup.

## `SourceContext` for File-Origin Errors

Every error variant whose origin is a markdown file carries a
`biscuit_terminal::errors::SourceContext`. The type holds:

- `absolute: PathBuf` — used for OSC 8 hyperlinks.
- `display: PathBuf` — visible label (typically relative to the cwd).
- `content: Arc<str>` — the full source, shared cheaply across clones.
- `frontmatter: Option<Range<usize>>` — auto-detected on construction.

Three helper methods produce ready-to-use `Prose` segments:

- `linked_path_prose()` — `<blue><a href=ABS>DISPLAY</a></blue>` for the
  header.
- `frontmatter_prose()` — fenced ```yaml block for the body, or `None` if
  the source has no frontmatter.
- `excerpt_prose(line, context, lang)` — fenced code block with line
  numbers and a `>` gutter on the offending line.

Threading: build `SourceContext` at the parse boundary where the path and
content are already in hand. Pass it into the parser entry point and
attach it to error variants as they are constructed.

## Standard Structural Layout

Errors with a file origin follow this layout (rendered from top to
bottom):

1. **Header** — `<error-type>: <one-sentence summary>` with the linked
   path embedded when relevant.
2. **Frontmatter snapshot** (optional) — preceded by the line
   `The Frontmatter of this document was:`.
3. **Source excerpt** — preceded by a phrase such as
   `The opening page block was found here:`.
4. **Hint** — single sentence suggesting the corrective action.
   Directive tokens are highlighted with `<inverse>`.

The reference implementation is `PageBlockError::UnterminatedBlock` in
`darkmatter/lib/src/markdown/compose/page_blocks/types.rs`.

## Snapshot Tests

Every `BlockError` variant is exercised by an integration test under
`darkmatter/lib/tests/error_snapshots/`. The tests:

1. Construct the variant with a representative payload.
2. Render it via `BlockError::report_block_error_optimistic(Some(80))`.
3. Strip ANSI with `biscuit_terminal::utils::escape_codes::strip_escape_codes`.
4. Assert the result against a checked-in `insta` snapshot when the
   variant has structural body content.
5. Use `assert_contains_all` for invariant tokens (error type, key
   identifiers, hint cues).

To accept new snapshots locally:

```bash
cd darkmatter
INSTA_UPDATE=always cargo test -p darkmatter --test error_snapshots
```

CI runs `cargo test` without `INSTA_UPDATE`, so any drift in rendered
output fails the build.

## Adding a New Error Variant

1. Add a `source: SourceContext` field to the variant when the error
   originates from a file.
2. Implement `BlockError::status_block` for the variant. Build the body
   as a `Vec<Prose>`. Use the `SourceContext` helpers for the linked
   header, frontmatter snapshot, and source excerpt.
3. Escape any user-supplied strings (`replace('_', "\\_")` etc.) before
   embedding them in Prose markup.
4. Add a snapshot test under `darkmatter/lib/tests/error_snapshots/`.
5. Confirm the rendered output passes `just test` and `just lint` in the
   `darkmatter` package area.

## See Also

- [biscuit-terminal/lib/src/errors/source_context.rs](../../../biscuit-terminal/lib/src/errors/source_context.rs)
- [biscuit-terminal/lib/src/components/status_block.rs](../../../biscuit-terminal/lib/src/components/status_block.rs)
- [features/2026-05-08-good-errors/spec.md](../../features/2026-05-08-good-errors/spec.md)
