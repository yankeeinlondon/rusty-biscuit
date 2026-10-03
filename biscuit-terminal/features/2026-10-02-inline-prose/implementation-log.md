---
spec: /Volumes/coding/wt/rusty-biscuit/fix-magic-globs/biscuit-terminal/features/2026-10-02-inline-prose/spec.md
plan: biscuit-terminal/features/2026-10-02-inline-prose/plan.md
implemented_by: claude/opus
started_phase: 1
---

# Implementation Log for 2026-10-02-inline-prose (8 phases)

## Phase 1

Phase 1 is rulings, spikes, and a baseline. It changes no source code.

### Rulings (R1–R10)

The plan records rulings R1–R10 as yolo defaults the author may overturn at
review. Phase 1 adopts them as written; the spikes below add these
refinements:

- **R1 — Sentinel safety.** Adopted as written. Facts for Phase 3: the
  current parser (`components/prose/markdown.rs`) already lifts two kinds
  of content behind C0 control-character sentinels, not private-use ones:
  `HREF_PLACEHOLDER_MARK = '\u{0001}'` (`\u{0001}HREF<n>\u{0001}`, link
  targets) and `CODE_BLOCK_PLACEHOLDER_MARK = '\u{0002}'`
  (`\u{0002}CODE<n>\u{0002}`, fenced blocks). There is no pre-pass today:
  a malformed placeholder is treated as literal text
  (`tokens.rs::take_code_placeholder`), but a well-formed one with an in-range
  index typed by the user *is* resolved, so input can impersonate a reference now. Phase 3
  must add the R1 pre-pass for every sentinel the parser uses (including
  the new code-span one), so there is one escape mechanism, not one per
  lift.
- **R2 — `\` before a blank line.** Adopted as written: literal backslash;
  `Soft` → one soft break per run; `Hard` → one hard break per `\n` of the
  run (`a\n\nb` → two hard breaks).
- **R3 — Layout transfer.** Adopted. S2b (below) shows every non-inline
  kind accepts `Layout`, so `ListItem`, `BlockQuote`, `TableCell`, and
  `Paragraph` all qualify and no new neutral block kind is needed. A nested
  `Root` is rejected by the validator ("Root node may appear only as the
  top-level node"), which confirms the "never a nested `Root`" clause.
- **R4 — `StyledProse` → `StyledInlineProse`.** Adopted as written.
- **R5 — Inline vs block classification.** Adopted as written; the S1
  ledger applies it.
- **R6 — Fence helper lives in `renderable`.** Adopted as written.
- **R7 — No new `bt prose` flags.** Adopted as written.
- **R8 — Perf.** Adopted. `biscuit-terminal/lib/tests/l1/perf_gate.rs`
  exists and is part of the baseline below.
- **R9 — Known-red ledger.** Adopted; see "Known-Red Ledger".
- **R10 — Spec `packages:`.** Adopted; extra consumers found by S1 are
  recorded under "Scope Record", not edited into the spec.

### S2 — Terminal Renderer Feasibility

Read-only, macOS.

#### S2a — Appearance restore and the backtick-fallback signal

- **Restore is supported; no new style stack is needed.**
  `render_tree/render.rs` passes the inherited text appearance down as the
  `effective: &Style` argument of `render_inline` / `render_inline_node`.
  Every styled inline kind (`Emphasis`, `Strong`, `Delete`, `Span` with a
  style, `InlineCode`, `Extended` mark/dim, class-driven styling via
  `apply_classes`) computes `child_effective`, opens with
  `style::text_appearance_sgr(&child_effective, term)`, and closes with
  `style::appearance_close(&open, effective, term)`, which emits a reset
  and then re-applies the *parent* appearance. The recursion is the stack.
  The `InlineCode` arm (render.rs, `NodeKind::InlineCode { value }`)
  already follows this pattern: themed background/foreground when the
  context carries `inline_code_background`, otherwise `dim`.
- **The "no styling emitted" signal needs a comparison, not `open.is_empty()`.**
  `text_appearance_sgr` returns the *full* effective appearance, so `open`
  is non-empty whenever an ancestor is styled, even if the code span itself
  adds nothing visible. Example: inside an underlined run on a
  `ColorDepth::None` terminal with underline support, `dim` is suppressed
  but `open` still carries the inherited underline. The Phase 3 fallback
  should therefore keep the backticks when
  `text_appearance_sgr(&child_effective, term) == text_appearance_sgr(effective, term)`
  (the code span changed nothing). On a plain terminal both sides are
  empty, which is the common case.
- **OSC 8 does not count.** Hyperlink escapes are added in the `Link` arm
  around the already-styled label; they never pass through
  `text_appearance_sgr`, so a code span inside a link is judged on its SGR
  alone, as the spec requires.

#### S2b — Which `NodeKind`s accept `Layout`

`renderable/src/tree/validate.rs` rejects a `Layout` only on inline kinds
(`is_inline_kind`); every other kind accepts it. The terminal renderer
applies it in `render_with_layout` for any non-inline node and drops it
on inline nodes.

| Accepts `Layout` | Rejects `Layout` (inline) |
| --- | --- |
| `Root`, `Heading`, `Section`, `Paragraph`, `BlockQuote`, `List`, `ListItem`, `Code`, `ThematicBreak`, `Table`, `TableRow`, `TableCell`, `FootnoteDefinition`, `Disclosure`, `Html`, `Unsupported` | `Text`, `Emphasis`, `Strong`, `Delete`, `Span`, `Extended`, `InlineCode`, `Link`, `Image`, `FootnoteReference`, `SoftBreak`, `HardBreak` |

Other placement constraints that matter for R3:

- `Root` is valid only at the top level, so an embedded `Prose` must give
  up its `Root` and hand its children (and its `Layout`) to the container.
- A `sequence-join` policy is `Root`-only.
- `TextLayoutHints` (pad/truncate) are allowed only on `Link`, `Image`,
  and `ListItem`; they are not a substitute for `Layout`.

### Scope Record

(S1 ledger — see below.)

### Known-Red Ledger

(Filled from the baseline.)

### Snapshot Review

(Ledger of snapshot directories likely to move — see below.)

### Drift Notes

None found in Phase 1 (read-only).

### Departures

None in Phase 1.
