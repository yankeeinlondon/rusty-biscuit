# Darkmatter Code-Block Rendering Failures — Fix Spec

**Date:** 2026-05-22
**Area:** `darkmatter` (terminal rendering of Markdown code blocks under page layout)
**Reproduction:** `md blocks.md --ml 4 --mr 4 --mt 1 --mb 1 --code-theme dracula`
**Fixture:** `renderable/blocks.md`

This spec covers three discrete stages:

1. **Stage 1 — Defects:** enumerate every observed rendering error, root-cause each
   one against the current code, and define the fix.
2. **Stage 2 — Test gaps:** explain *why the existing tests passed despite these
   obvious defects*, then specify the tests to add (both to reproduce the known
   defects and to surface additional ones).
3. **Stage 3 — Execution:** TDD loop — land the failing tests first, then fix
   until green, with the success criteria stated up front.

---

## Stage 1 — Defects and Root Causes

### Reproduction context

For `--ml 4 --mr 4 --mt 1 --mb 1`, `DarkmatterPage::render` (`darkmatter/lib/src/layout/page.rs:464`):

- builds a `LayoutContext` whose `effective_width = terminal_width − margin_x − padding_x = terminal_width − 8`;
- sets `options.max_width = Some(effective_width)` (`page.rs:484`);
- delegates to the bespoke terminal renderer with `Some(&ctx)`;
- wraps the rendered body in `apply_row_decoration` (`page.rs:650`), which prepends
  `margin_left` (4 spaces) and appends `margin_right` (4 spaces) to **every** body line.

The crucial consequence: inside the renderer, `terminal_width` is derived from
`options.max_width` (`terminal.rs:890`), so **`terminal_width == effective_width`**
during a page-decorated render. Several width comparisons downstream were written
assuming `terminal_width` is the *physical* terminal width; under decoration it is
not, and they misfire.

---

### Defect #0 — Code blocks do not invert their theme for contrast (and the theme abstraction is undocumented)

**Symptom.** In a dark terminal the code block renders with a *dark* syntax theme,
so its background is nearly indistinguishable from the page background. There is no
contrast lift separating the code panel from prose. Every other defect below is
made far harder to see because of this.

**Intended behavior.** A code block should render with the **opposite** light/dark
variant of the page: a *light* code theme in a dark terminal, and a *dark* code
theme in a light terminal. The contrast is what visually lifts the code panel off
the page. **This inversion applies to code blocks only** — prose, headings, tables,
and the page background continue to follow the terminal's real light/dark mode so
body text stays readable. (Confirmed design decision: "Code blocks only".)

**Root cause.**

- `CodeHighlighter` is built with the *same* color mode as prose:
  - Bespoke path: `terminal.rs:895` — `CodeHighlighter::new(options.code_theme, options.color_mode)`.
  - Render-tree path: `markdown/render_tree/code_renderer.rs:96-98`.
  - YamlBlock path: `markdown/yaml_block.rs:177`.
  - HTML path: `markdown/output/html.rs` (via `render_html_code_block`).
- There is no inversion of the color mode for code; `options.color_mode`
  (= `ctx.render_color_mode`, the terminal's mode) flows straight into the code
  theme resolution.

**The theme abstraction (must be documented).** `ThemePair` is a *user-facing,
mode-agnostic* theme name (e.g. `github`, `dracula`). `ThemePair::resolve(ColorMode)`
(`markdown/highlighting/themes.rs:97`) maps the abstract name **plus** a color mode
to a concrete `Theme` variant — e.g. `(Github, Dark) → GithubDark`,
`(Github, Light) → GithubLight`. **The literal light/dark choice is already encoded
in this mapping.** A few pairs are intentionally single-variant and ignore the mode
entirely — `(Dracula, _) → Dracula`, `(Nord, _) → Nord`, `(Monokai, _) → MonokaiExtended`,
`(VisualStudioDark, _) → VisualStudioDark` (`themes.rs:109-112`). For those, inversion
is a no-op *by design* — they have no opposite variant, and that is correct, not a
bug. The screenshot used `--code-theme dracula`, which is exactly one of these
single-variant themes, so it cannot lighten regardless; that is expected once the
abstraction is understood.

**Fix.**

1. Add an explicit inversion at the point where the **code** highlighter (and only
   the code highlighter) is constructed: resolve the code `ThemePair` against
   `options.color_mode.inverted()` rather than `options.color_mode`. Prose continues
   to use `options.color_mode`. Apply consistently in all four construction sites
   (bespoke, render-tree, YamlBlock, HTML) so terminal and browser agree.
2. The same inverted mode must be threaded into `render_terminal_code_block`'s
   `color_mode` argument (used by `compute_highlight_bg` / `adjust_background`,
   `code_block.rs:371`, `terminal.rs:2909`) so highlight-line background math matches
   the inverted theme.
3. **Document the abstraction** in at least:
   - `markdown/highlighting/themes.rs` (doc comment on `ThemePair`, `resolve`, and
     the single-variant arms — state that single-variant pairs deliberately ignore mode);
   - the `darkmatter` skill (`.claude/skills/darkmatter/`), terminal/highlighting topic;
   - `darkmatter/lib`/CLI README where `--code-theme` is described;
   - a code comment at each code-highlighter construction site explaining the
     deliberate inversion ("code blocks contrast against the page; prose does not").

> **Note.** Because fixing #0 makes the code panel's background clearly distinct,
> the top/bottom code-block *padding rows* (which are part of the panel, not blank
> lines) will read as panel, not as empty space. This materially changes the
> *perception* of Defect #3 and must be accounted for when counting "blank lines".

---

### Defect #1 — Code-block background is mis-painted; a 4-column unstyled gap appears after the last code character

**Symptom.** The syntax-highlighted code text is followed by ~4 columns with **no**
theme background, after which the theme background resumes and runs to the screen
edge. The right margin is not honored as a boundary on the code body.

**Root cause (this is the single linchpin defect; #2 is the same cause).**

The code **body** clears to the physical terminal edge with `\x1b[K` instead of
padding to the content width with spaces:

- `render_terminal_code_block` pads each line to an explicit width **only when
  `target_width = Some(w)`**; otherwise it emits `\x1b[K` (`code_block.rs:143-155`,
  bottom padding `:162-178`, top padding `emit_padding_row :385`).
- The caller decides `target_width` with this guard (`terminal.rs:1212-1216`):
  ```rust
  let body_width = if code_width < terminal_width { Some(code_width) } else { None };
  ```
- Under page decoration, `terminal_width == effective_width` (see context) **and**
  `code_width = resolve_component_render_width(CodeBlocks, terminal_width, Some(ctx))`
  resolves (for the default `PageFill::Full`) to `effective_width`, then
  `terminal_width.min(effective_width) = effective_width` (`terminal.rs:2873`,
  `layout/context.rs:resolve_component_width`).
- Therefore `code_width < terminal_width` is `effective_width < effective_width` →
  **false** → `body_width = None` → the body uses `\x1b[K`.

`\x1b[K` (EL, Erase in Line) erases from the cursor to the physical end of the line
using the active background, **without moving the cursor**. The render then runs
`apply_row_decoration`, which has *already* been bypassed for background painting
(the page background is `Transparent` here) but still appends `margin_right` (4
literal, unstyled spaces) immediately after the body line. Sequence per code line as
the terminal executes it:

1. 4 left-margin spaces (cursor at col 4).
2. set code bg, print code text (cursor at col `4 + visible`).
3. `\x1b[K` erases `4+visible … edge` with code bg; **cursor stays at `4+visible`**.
4. `\x1b[0m` reset.
5. 4 right-margin spaces printed **at col `4+visible`** — with no background —
   overwriting the first 4 cells `\x1b[K` had just painted.

Result: code text → **4 unstyled cells** (the misplaced right margin) → theme bg to
the edge. That is precisely the artifact in the screenshot, and the "4" is exactly
`--mr 4`.

**Fix.** Never use `\x1b[K` for the code body when a layout context is present (i.e.
when row decoration will run). Pad the body to the resolved content width with
background-colored spaces, exactly as the header pill already does. Concretely:

```rust
let body_width = if layout_ctx.is_some() || code_width < terminal_width {
    Some(code_width)
} else {
    None // true zero-config path: code spans the physical terminal from col 0
};
```

This mirrors the already-fixed `YamlBlock`/render-tree behavior ("pad the body to
that width rather than clearing to the physical edge with `\x1b[K`", recorded in
`tests/render_comparison.rs` KNOWN_DRIFT notes and recent commits
`8e5e7427` / `46c2f5eb`). The bespoke Markdown code-fence path was simply never
switched over.

`None` (→ `\x1b[K`) must remain only for the genuine zero-config path (`layout_ctx`
is `None`, content starts at column 0), to preserve byte-for-byte equivalence with
`for_terminal(&md, TerminalOptions::default())`.

---

### Defect #2 — Language pill respects the right margin, code body does not (same cause as #1)

**Symptom.** The language pill (`rust`, `ts`) sits correctly at the right-margin
boundary, while the code body's right edge does not — making the pill *look*
misplaced when in fact it is correct and the body is wrong.

**Root cause.** `format_header_row` (`terminal.rs:2209`) right-aligns the pill by
emitting **explicit spaces** up to `code_width` (`:2238`), so the header spans
exactly the content rectangle and lands on the right-margin boundary. The body, per
Defect #1, uses `\x1b[K` and never pads to `code_width`. The two paths disagree on
how they reach the right edge.

**Fix.** Same as Defect #1: pad the body with explicit spaces to `code_width`. Once
the body pads identically to the header, the pill and the body share the same right
boundary. No change to `format_header_row` is required.

---

### Defect #3 — Extra blank lines after blocks; Markdown's "no 2+ consecutive blank lines" invariant is violated

**Symptom.** Rendering adds two blank lines where the bottom margin is 0, and three
where it is 1 — i.e. a constant **+2** trailing-blank offset on top of a correct
per-row margin. More generally, output can contain runs of 2+ blank lines, which
Markdown semantics forbid: in Markdown, any run of ≥2 newlines is a single paragraph
break; 2 blank lines and 99 blank lines must render identically.

**Root causes (several contributors; #0 changes how they are *perceived*).**

1. **Hardcoded double newline after every code block** (`terminal.rs:1234`):
   `wrapper.push_with_newlines("\n\n")`. One `\n` terminates the bottom padding row;
   the second is an unconditional separator. It does not consult any layout margin
   and stacks with whatever follows.
2. **Code-block padding rows masquerade as blank lines.** The block emits a top and
   a bottom padding row (`code_block.rs:75`, `:162-178`). With Defect #0 unfixed the
   theme bg ≈ terminal bg, so each padding row *looks* blank — visually inflating the
   apparent blank-line count by up to 2 around every block. Fixing #0 reclassifies
   these as panel, not blank space.
3. **No global "collapse consecutive blank lines" invariant.** The renderer guards
   against doubling only ad hoc — e.g. the heading handler checks
   `!output.ends_with("\n\n")` before inserting a blank (`terminal.rs:1247`). There
   is no single normalization pass guaranteeing the Markdown invariant across all
   block boundaries and at the document tail, so the constant +2 offset survives.

**Fix.**

1. Replace the hardcoded `"\n\n"` block separator with the same single-blank-line
   rhythm used elsewhere (cf. `write_horizontal_rule`, `terminal.rs:2799-2811`,
   which already standardized one trailing blank line for default layout).
2. Apply a **final normalization** on the rendered terminal body that collapses any
   run of ≥2 blank lines to exactly one and trims trailing blank lines, *before*
   page decoration adds the configured top/bottom margins. Margins (`mt`/`mb`) are
   then the *only* source of leading/trailing blank rows, so `mb=0 → 0`, `mb=1 → 1`,
   matching expectations. This normalization must run on the inner body so it does
   not eat the deliberate page margin rows.
3. Confirm the exact pre-fix trailing count with a failing test (Stage 2) rather
   than assuming it; the fix target is "blank-line count is a pure function of the
   configured margins, with no constant offset, and no interior run exceeds one".

> The normalization must treat a code-block padding row (a bg-filled line) as
> **non-blank** — it is content. "Blank" means a line whose visible width is zero
> after ANSI stripping *and* which carries no background fill. Care is needed so the
> collapse pass does not delete padding rows.

---

## Stage 1 — Additional defects found while tracing

| # | Defect | Location | Notes |
|---|--------|----------|-------|
| A | Right-margin spaces can push a line past the physical width when the body still clears to edge | `page.rs:719` + `code_block.rs:153` | Resolved transitively by the Defect #1 fix; add an explicit "no rendered line exceeds terminal width" invariant test to lock it. |
| B | `ThemePair` abstraction & single-variant arms undocumented | `themes.rs:97-114` | Directly caused the #0 confusion; documentation is part of the #0 fix. |
| C | Two different color-mode sources | `themes.rs:detect_color_mode` (YamlBlock path) vs `terminal.color_mode → ctx.render_color_mode` (page path) | Inversion must be applied consistently regardless of which source feeds a given path; verify both. |
| D | HTML code blocks do not invert for contrast | `render_html_code_block`, `code_block.rs:201` | Apply the same code-blocks-only inversion in the browser path for cross-target consistency. |
| E | `render_comparison.rs` is a *parity* harness, not a *correctness* harness | `tests/render_comparison.rs` | See Stage 2 — this is the main reason the defects shipped. |

---

## Stage 2 — Why the tests missed this, and what to add

### Why the existing suite passed despite obvious breakage

1. **Parity, not ground truth.** `tests/render_comparison.rs` compares the *bespoke*
   renderer against the *render-tree* renderer across six facets. It asserts the two
   implementations **agree**, never that either is **correct**. Worse, its matrix
   cells use the `YamlBlock` component — which was *already migrated* to pad-to-width
   — so the buggy bespoke Markdown-fence path under page decoration is **not in the
   matrix at all**. The KNOWN_DRIFT ledger even documents the pad-vs-`\x1b[K` fix for
   the migrated paths while the un-migrated fence path stayed untested.
2. **Snapshots bless whatever renders.** `tests/layout_snapshots.rs` uses
   `insta::assert_snapshot!` on full output. A snapshot captures the *current* bytes —
   including bugs — and passes until a human re-reviews. Snapshots encode "what it
   does", never "what it should do".
3. **Unit tests assert only weak existence properties.** `code_block.rs` tests assert
   `output.contains("\x1b[")` or `plain.contains("foo: 1")`. None assert background
   coverage, right-edge position, line width, margin placement, blank-line counts, or
   theme contrast. The one width-padding test
   (`test_render_terminal_code_block_with_target_width_pads_spaces`) only exercises
   the `Some(w)` branch — exactly the branch the bug never takes.
4. **The buggy branch is never exercised with `PageFill::Full` + margins.** Page
   tests like `render_code_block_with_pad_fill` use `Pad`, which makes
   `code_width < terminal_width` *true*, so they take the `Some(w)` branch and look
   fine. The CLI default is `Full`, which makes `code_width == terminal_width` and
   takes the `None` (`\x1b[K`) branch — untested.
5. **No color-mode/contrast assertions anywhere.** Nothing asserts that the code
   theme differs from the prose/page mode, so the missing inversion was invisible.

### Generalize the defects into matrix-wide rendering invariants (the real goal)

Catching *these* bugs is the floor, not the ceiling. Every defect here is a
*specific* breach of a *general* property that must hold for **every block-level
shape under every layout**. The high-leverage move — and the one that pays off
**today**, because a bug caught now is worth 10× one caught later — is to encode
those general properties as **ground-truth invariants** and sweep them across the
full `components × scenarios` matrix in one shot. That sweep is expected to *fail in
several cells we have not yet inspected*; each failure is a bug found today rather
than shipped.

The defects map to a small, reusable invariant set. Each invariant is a predicate
`fn(rendered: &str, expect: &LayoutExpectation) -> Result<(), Violation>`, where
`LayoutExpectation` carries the scenario's resolved `left`, `right`, `top`, `bottom`,
`effective_width`, physical `width`, `color_mode`, and per-component class (code vs
non-code):

| Inv | Property (must hold for every block shape) | Generalizes |
|-----|--------------------------------------------|-------------|
| **I1 Containment** | No rendered line's visible width exceeds the physical terminal width, post-decoration. | #1/#2, A |
| **I2 BackgroundRectangle** | A line carrying an SGR background opens it after exactly `left` cells and ends the fill at `width − right`; there is **no `\x1b[K`** when a layout context is active, and **no unstyled gap** inside the content rectangle. | #1 |
| **I3 RightBoundaryCoherence** | Within one block, every background-bearing line (chrome + body, e.g. header pill + code body) ends its fill at the **same** column. | #2 |
| **I4 LeftOffsetUniformity** | Leading-space count equals the resolved left offset (margin/alignment), identically on every line of the block. | #1/#2 (reuses `indent_profile`) |
| **I5 VerticalRhythm** | No run of ≥2 consecutive *blank* lines in the body (a bg-filled padding row is **not** blank); leading blank rows == `top`, trailing == `bottom`, with **no constant offset**. | #3 |
| **I6 BlankLineIdempotence** | Output is invariant to the number of blank lines (≥1) separating blocks in the source: 1 vs 5 vs 99 blanks render identically. | #3 |
| **I7 ColorModeContract** | Code surfaces resolve their theme against the **inverted** terminal mode; non-code surfaces (prose, headings, lists, tables, page bg) follow the terminal mode. | #0, C, D |

**Two structural changes turn this from code-block-specific into broad coverage:**

1. **Widen the component matrix.** `tests/layout_matrix_support` currently exposes a
   single `ComponentCase` (`YamlBlock`) rendered directly via `Layout`. Extend
   `component_cases()` to every block shape, **rendered through `DarkmatterPage`**
   (the path the reported bugs live in): heading, paragraph/prose (wrapping), ordered
   & unordered list, blockquote, table, **Markdown code fence** (paired theme *and*
   single-variant theme), `YamlBlock`, horizontal rule, image placeholder. Each cell
   knows its component class for I7.
2. **Widen the scenario matrix to the combinations that actually broke.** Add the
   page-level scenarios the current matrix omits — most importantly **`Full` fill with
   left *and* right margin set together** (the CLI default that hides the `\x1b[K`
   bug), plus padding + page-bg, `max_width`, and the literal repro
   (`ml4 mr4 mt1 mb1`). Keep the one-dimension-at-a-time scenarios too.

**New harness, alongside the old one.** Keep `render_comparison.rs` (parity:
"the two renderers disagree") and add `tests/render_invariants.rs` (correctness:
"the output is wrong, regardless of agreement"). The reported defects shipped
precisely because only parity existed *and* the buggy shape was absent from the
matrix; parity can never catch a fault both renderers share, and ground-truth
invariants run on a single renderer so they need no second opinion. The invariant
sweep iterates `component_cases() × scenarios()`, runs each applicable invariant, and
reports **all** violations as a ledgered set (same regression/fixed protocol as
`render_comparison.rs`) so newly-discovered breakage is captured rather than silently
re-baselined.

**Reusability beyond darkmatter.** The invariant predicates operate on rendered ANSI
strings plus a `LayoutExpectation`; they have no darkmatter-specific dependency.
Factor them so `biscuit-terminal` (or a shared test-support crate) can reuse the same
checks against *its* renderable components — the same `Layout`, `Margin`, and
background-fill model underpins both. This makes the invariant set the standing
contract for "what correct layout output looks like" across the whole rendering
stack.

### Seed tests (concrete reproductions of the known defects)

These pin the specific reported failures. They are the **first** cells of the
invariant matrix above, written explicitly so the root cause of each is unambiguous;
the matrix sweep then generalizes them across all shapes. All assert **ground truth**,
not parity, and prefer invariant predicates over byte snapshots.

**A. Code-body width & background (Defects #1, #2, A)** — `code_block.rs` unit +
`page.rs` integration:

- Under a layout context with `--ml 4 --mr 4` and default `Full` fill, every code
  body line and both padding rows have visible width **exactly `effective_width`**
  after ANSI stripping (no `\x1b[K`, no short lines, no overflow).
- The rendered output contains **no `\x1b[K`** on any code-block line when a layout
  context is present.
- After full page decoration, **no rendered line exceeds the physical terminal
  width** (locks Defect A).
- The header pill's right edge and the body's right edge sit at the **same column**.
- A regression test asserting the specific failure shape pre-fix: there is **no run
  of unstyled cells between the last code glyph and the right margin** (the 4-col gap).

**B. Margin / blank-line invariants (Defect #3)** — `page.rs` integration:

- For a document ending in a code block (and separately a table), the number of
  trailing blank lines equals `margin_bottom` exactly, for `mb ∈ {0, 1, 2}` — i.e.
  no constant offset.
- The rendered body (pre-decoration) contains **no run of ≥2 consecutive blank
  lines** (Markdown invariant), where "blank" excludes bg-filled padding rows.
- Two source documents that differ only in the *number* of blank lines between
  blocks (e.g. 1 vs 5 blank lines) produce **identical** rendered output.
- A "padding rows are preserved" test: the collapse pass does not delete code-block
  top/bottom padding rows.

**C. Theme contrast / inversion (Defect #0, B, C, D)** — `highlighting` + `code_block.rs` + `html`:

- Code highlighter resolves against the **inverted** mode: with terminal mode
  `Dark` and a *paired* theme (`github`), the code block background equals
  `GithubLight`'s background, while prose uses `GithubDark`. And the mirror for
  terminal mode `Light`.
- Single-variant themes (`dracula`, `nord`, `monokai`, `vs-dark`) resolve to the
  same `Theme` under both modes — assert the documented no-op so the abstraction is
  pinned and a future "fix" can't silently change it.
- `ThemePair::resolve` doc-test demonstrating the abstraction (paired vs
  single-variant), so the contract is executable documentation.
- HTML code block uses the inverted mode for its `<span>` colors and background,
  matching terminal.

**D. CLI end-to-end (the actual repro)** — `darkmatter/cli` integration test:

- Run the equivalent of `md blocks.md --ml 4 --mr 4 --mt 1 --mb 1 --code-theme dracula`
  through the library API (`DarkmatterPage`) and assert the invariants from A and B
  hold on the real fixture.

### Additional defects the matrix sweep is expected to surface (today, not later)

The invariant sweep is the primary discovery mechanism, and it must be run **before**
any fix so we harvest the cheap bugs while they are cheap. Running I1–I7 across the
widened `components × scenarios` matrix is expected to flag, at minimum: the HTML
contrast gap (D), the dual color-mode-source inconsistency (C), and any table / list /
blockquote / image block that also relies on `\x1b[K` or a hardcoded `"\n\n"`
separator under decoration. The static audit (every `\x1b[K` and every literal
`"\n\n"` push in `terminal.rs`) feeds the same list. Each surfaced violation is
recorded in the invariant ledger as a found-today bug, triaged, and either fixed in
this effort or split out with a tracking note — none are silently re-baselined.

---

## Stage 3 — Execution (TDD)

Use the systematic-debugging + TDD discipline: **write the failing test first, watch
it fail for the expected reason, then fix.**

### Order of operations

Discovery first, fixes second — the value of each defect decays the longer it sits.

1. **Build the invariant harness early.** Implement I1–I7 as predicates plus the
   widened `component_cases()` (all block shapes through `DarkmatterPage`) and the
   widened `scenarios()` (incl. `Full` fill + left&right margins, and the literal
   repro). This is the single highest-leverage step.
2. **Run the sweep and harvest.** Execute the invariant matrix and the static audit
   (every `\x1b[K` / `"\n\n"` in `terminal.rs`) **before any fix**. Catalogue every
   violation into the invariant ledger — this is the "bugs caught today" deliverable.
   Confirm the seed defects (#0–#3, A–E) appear, and triage anything new.
3. **Land the seed tests** (Stage 2 groups A–D) as explicit, root-cause-pinned
   reproductions; confirm each fails for the expected reason, and record the exact
   pre-fix trailing-blank count for #3.
4. **Fix Defect #0** (theme inversion, code-blocks-only) across the four construction
   sites + documentation. Re-run group C + I7.
5. **Fix Defects #1/#2** (pad body to `code_width`; drop `\x1b[K` under layout) at
   `terminal.rs:1212`. Re-run group A + I1–I4.
6. **Fix Defect #3** (replace hardcoded `"\n\n"`; collapse blank-line runs on the
   inner body before decoration). Re-run group B + I5/I6.
7. **Burn down the rest of the ledger.** Address every other violation the sweep
   surfaced, or split it out with an explicit tracking note. The invariant matrix must
   reach an empty (or fully-explained) ledger.
8. **Re-run the full darkmatter suite**, including `render_comparison.rs` and
   `layout_snapshots.rs`. Intentional snapshot changes are re-blessed *only after*
   visual review confirms the new output is correct (not just different). Update the
   KNOWN_DRIFT ledger per its regeneration protocol if parity shifts.
9. **Manual verification** against the real repro command in a dark terminal
   (Level-2 style): code panel visibly contrasts (light theme on dark page), bg spans
   the full content rectangle, pill and body share the right boundary, no stray 4-col
   gap, blank-line count tracks margins with no constant offset.
10. **Drift maintenance:** update READMEs, the `darkmatter` skill, and any `docs/`
    affected by the theme-abstraction documentation and the spacing change.

### Success criteria

- The invariant matrix (I1–I7 across all shapes × scenarios) passes with an empty or
  fully-explained ledger — no shape/scenario cell silently re-baselined.
- All seed tests (A–D) pass.
- Full `cargo test -p darkmatter` (and `cargo nextest`) green; any re-blessed
  snapshot visually reviewed.
- The repro command renders: contrasting code theme; code-theme background covering
  exactly columns `[margin_left … width − margin_right]` on every code line and
  padding row; language pill and code body sharing the right-margin boundary; no
  unstyled gap; trailing blank lines equal to `mb`; no interior run of ≥2 blank lines.
- `ThemePair`'s mode abstraction (and the deliberate single-variant no-op) is
  documented in code, skill, and README, with an executable doc-test.

### Out of scope

- Re-architecting the bespoke renderer onto the render tree (tracked separately).
- New themes or new `PageFill` modes.
- Browser layout beyond the code-blocks-only contrast inversion (the inversion
  itself is **now in scope and implemented** — see Stage 4, Defect D).

---

## Stage 4 — Scope expansion (2026-05-22 follow-up)

The initial fix (commits `9c364244` / `5366b66e`) landed Defects #0–#3 with the
seed tests and a partial invariant suite (I1, I2-as-`no-clear`, I5, I5b, I7).
This stage completes the spec's full ambition. All work is TDD and lands green.

### 4.1 Invariant predicates factored for cross-crate reuse (Additional defect E)

The invariant predicates now live in
`biscuit-test-harness/src/layout_invariants.rs` as a `pub` module, operating on
a rendered ANSI string plus a `LayoutExpectation { width, left, right, top,
bottom }`. `biscuit-test-harness` is `publish = false` and already a
dev-dependency of both **darkmatter** and **biscuit-terminal**, so the same
contract is reusable against biscuit-terminal's own renderable components.
Each predicate returns `Result<(), String>` and carries its own unit tests
(SGR-aware background-extent tracking via `bg_extent`).

### 4.2 Full invariant set (I1–I6) with the right-aligned-pill refinement

The harness now implements the complete set the Stage 2 table specified:

| Predicate | Spec inv | Applies to | Notes |
|-----------|----------|------------|-------|
| `containment` | I1 | every shape | unchanged |
| `no_clear_to_eol` | I2a | every shape | `\x1b[K` ban |
| `background_rectangle` | I2b | code panels | fill ends at `width - right` |
| `right_boundary_coherence` | I3 | code panels | all bg lines end at one column |
| `left_offset_uniformity` | I4 | code panels | **full-bleed** lines open at `left` |
| `vertical_rhythm` | I5 | every shape | no ≥2 interior blanks |
| `margin_blanks` | I5b | every shape | leading/trailing == `top`/`bottom` |
| `blank_line_idempotent` | I6 | every shape | 1 vs 5 vs 99 blanks render identically |

**Refinement discovered by the sweep.** The spec's I2/I4 wording ("a line
carrying a background opens it after exactly `left` cells") does **not** hold
for a code block's **language pill**: the pill is right-aligned chrome whose
(narrow) background legitimately opens mid-line and ends on the right boundary.
Per Defect #2 ("No change to `format_header_row` is required"), the pill is
correct as-is. So the rectangle family was scoped accordingly:

- **I2b** and **I3** check the *right* edge (`width - right`), which the pill
  satisfies — this is the real Defect #2 cross-coherence check.
- **I4** is scoped to *full-bleed* lines (fill == content width); narrow chrome
  is exempt.

This is the spec's "the sweep is expected to surface things we have not
inspected" working as intended — it surfaced a mis-generalization in the
invariant wording, not a renderer bug.

### 4.3 Ledger protocol for the invariant sweep (Additional defect E)

`darkmatter/lib/tests/render_invariants.rs` was converted from
panic-collect-all to the same regression/fixed ledger protocol as
`render_comparison.rs`: a committed `KNOWN_VIOLATIONS` const, a live-vs-known
diff that reports regressions and fixes separately, and a
`RECORD_INVARIANTS=1` regeneration mode. The sweep runs `shapes() × scenarios()`
(12 block shapes × 5 layouts, code-panel invariants gated on `is_code`) and
currently lands an **empty ledger** — every applicable invariant holds in every
cell. I6 (idempotence) and I7 (theme inversion) are dedicated tests alongside.

### 4.4 Defect D — HTML code blocks now invert (promoted from deferred)

HTML code blocks resolve their `ThemePair` against `color_mode.inverted()`,
matching the terminal for cross-target parity. The `color_mode` is read as the
caller-declared **page** mode, so a dark page emits a light code panel exactly
as the terminal does. Applied at **both** HTML construction sites:

- `markdown/output/html.rs::as_html` (drives `.code-block` / `.code-block-title`
  CSS and the highlighted `<span>` colors), and
- `markdown/yaml_block.rs::render_browser_html` (so a Markdown ` ```yaml ` fence
  and a `YamlBlock` stay byte-identical — locked by
  `test_browser_render_parity_with_markdown_yaml_fence`).

Verified by `tests/html_inversion.rs` (deterministic: dark page → github-light
`#ffffff`, light page → github-dark `#111b27`, single-variant dracula
mode-invariant) and the re-blessed `browser_render.rs` computed-style assertion
(`rgb(255, 255, 255)`). Nine horizontal-rule HTML snapshots were re-blessed —
the only delta is the documented `.code-block` background inverting
`#111b27 → #ffffff` and `.code-block-title` `#07111d → #f5f5f5`. Docs updated:
`darkmatter/cli/README.md`, `darkmatter/docs/rendering/{code-highlighting,style}.md`,
`darkmatter/docs/darkmatter-rendering-pipeline.md`, and the `darkmatter` skill
(`SKILL.md`, `terminal.md`).

### 4.5 Stage 4 success criteria (met)

- Invariant predicates live in `biscuit-test-harness::layout_invariants` with
  unit tests; reusable by biscuit-terminal.
- I1–I6 implemented; the `render_invariants` sweep passes with an empty ledger.
- Defect D implemented at both HTML sites; HTML and terminal agree; full
  `cargo test -p darkmatter` and `-p darkmatter-cli` green; snapshots re-blessed
  after review; docs and skill updated.
