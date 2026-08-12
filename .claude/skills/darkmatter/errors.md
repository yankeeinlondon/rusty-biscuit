---
name: darkmatter-errors
description: Conventions for implementing rich, source-aware diagnostics in darkmatter using `BlockError`, `Vec<Prose>` bodies, and `SourceContext`. Use when adding a new error variant, modifying `status_block` impls, or fixing markup-leak regressions.
---

# darkmatter Error Conventions

## Contents

- Body Contract: Vec<Prose>
- Escape User-Supplied Tokens
- SourceContext for File-Origin Errors
- Standard Structural Layout
- Snapshot Tests
- Adding a New Error Variant
- Anti-Patterns to Avoid
- See Also

Use heading search to jump to the listed subsystem.


User-facing diagnostics in darkmatter are rendered through the `BlockError`
trait from `biscuit-terminal`. Every variant follows the structural rules
below so that rendered errors are consistently rich, actionable, and
visually reviewable.

## Body Contract: `Vec<Prose>`

`StatusBlock::body` accepts a `Vec<Prose>` (or anything that implements
`IntoProseVec`). Each item is rendered through Prose individually and
stacked vertically with one blank line between items. This is the
mechanism that prevents `<dim>`, `<cyan>`, `<inverse>`, and other markup
tags from leaking as literal text — the bug fix that motivated this whole
contract.

Convenience shortcuts:

- `StatusBlock::body_line(prose)` — for a single Prose paragraph.
- `From<&str>` / `From<String>` for `Prose` plus `From<Prose> for Vec<Prose>`
  let `body("plain text")` keep working.

## Escape User-Supplied Tokens

Anything carried in an error variant that is user-supplied (paths,
identifiers, raw values, etc.) must be escaped before being embedded in a
Prose string. Most importantly:

```rust
.body(format!(
    "<dim>Target:</dim> <cyan>{}</cyan>",
    value.replace('_', "\\_"),
))
```

If you forget this step, identifiers like `_self` are reinterpreted by
Prose's markdown subset and rendered as italics or stripped entirely.

## `SourceContext` for File-Origin Errors

Errors that originate in a markdown file MUST carry a
`biscuit_terminal::errors::SourceContext`. The type holds:

- `absolute: PathBuf` — used for OSC 8 hyperlinks.
- `display: PathBuf` — visible label (typically relative to the cwd).
- `content: Arc<str>` — full source, shared cheaply across clones.
- `frontmatter: Option<Range<usize>>` — auto-detected on construction.

Three helper methods produce ready-to-use `Prose` segments:

- `linked_path_prose()` — `<blue><a href=ABS>DISPLAY</a></blue>` for the
  header.
- `frontmatter_prose()` — fenced ```yaml block, or `None` if absent.
- `excerpt_prose(line, context, lang)` — fenced code block with line
  numbers and a `>` gutter on the offending line.

Build `SourceContext` at the parse boundary, where the path and content
are already available. Pass it into the parser entry point and attach it
to error variants as they are constructed.

## Standard Structural Layout

Errors with a file origin follow this layout (rendered from top to
bottom):

1. **Header** — `<error-type>: <one-sentence summary>` with the linked
   path embedded when relevant.
2. **Frontmatter snapshot** (optional) — preceded by a phrase such as
   `The Frontmatter of this document was:`.
3. **Source excerpt** — preceded by a phrase such as
   `The opening page block was found here:`.
4. **Hint** — single sentence suggesting the corrective action.
   Directive tokens are highlighted with `<inverse>`.

Reference implementation:
`PageBlockError::UnterminatedBlock`

## Snapshot Tests

Every `BlockError` variant is exercised by an integration test under
`darkmatter/lib/tests/error_snapshots/`.
Tests:

1. Construct the variant with a representative payload.
2. Render it via `BlockError::report_block_error_optimistic(Some(80))`.
3. Strip ANSI with `biscuit_terminal::utils::escape_codes::strip_escape_codes`.
4. Assert with `insta::assert_snapshot!("variant_name", out)`.
5. Use `assert_contains_all` for invariant tokens (error type, key
   identifiers, hint cues).

Accepting new snapshots locally:

```bash
cd darkmatter
INSTA_UPDATE=always cargo test -p darkmatter --test error_snapshots
```

CI runs `cargo test` without `INSTA_UPDATE`, so any drift in rendered
output fails the build.

## Adding a New Error Variant

1. If the error has a file origin, add a `ctx: SourceContext` (or
   equivalently named) field to the variant.
2. Implement `BlockError::status_block` for the variant. Build the body
   as a `Vec<Prose>`. Use the `SourceContext` helpers for the linked
   header, frontmatter snapshot, and source excerpt.
3. Escape any user-supplied strings (`replace('_', "\\_")` etc.) before
   embedding them in Prose markup.
4. Add a snapshot test under `darkmatter/lib/tests/error_snapshots/`,
   including both `assert_contains_all` and `insta::assert_snapshot!`.
5. Run `INSTA_UPDATE=always cargo test -p darkmatter --test error_snapshots`
   to baseline the new snapshot, then review it.
6. Confirm the rendered output passes `just test` and `just lint` in the
   `darkmatter` package area.

## Anti-Patterns to Avoid

- ❌ `.body(format!("<dim>...</dim>"))` where `body` previously took a
  string — this used to render markup as literal text. The new
  `Vec<Prose>` signature always parses Prose, so this is now correct,
  but it must be reviewed against the escape rule above.
- ❌ Dropping `Arc<str>` cloning by passing `String` everywhere — `Arc<str>`
  is the canonical content type for `SourceContext`.
- ❌ Adding ad-hoc fenced code in error bodies — use
  `SourceContext::excerpt_prose` or `frontmatter_prose` so the gutter and
  frontmatter framing stay consistent.

## See Also

- Error rendering reference
- biscuit-terminal/SKILL.md — Prose grammar
  and `StatusBlock` mechanics.
- features/_completed/2026-05-08-good-errors/spec.md
