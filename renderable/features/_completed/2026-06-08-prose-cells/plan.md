---
phases: 5
created: 2026-06-09
start_phase: 1
packages:
  - biscuit-terminal
source_files_during_phase_1:
  - biscuit-terminal/lib/src/components/table/cell.rs
  - biscuit-terminal/lib/src/components/table/table.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - biscuit-terminal/lib/src/components/table/table.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
  - renderable/src/tree/attrs.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
  - biscuit-terminal
  - renderable
source_files_during_phase_3:
  - biscuit-terminal/lib/src/components/table/table.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - biscuit-terminal/lib/tests/prose_cells_parity.rs
  - biscuit-terminal/lib/tests/filesystem_parity.rs
  - biscuit-terminal/lib/src/components/filesystem/mod.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - biscuit-terminal/lib/src/components/table/table.rs
docs_updated_during_phase_5:
  - biscuit-terminal/lib/src/components/table/README.md
  - biscuit-terminal/docs/components/prose.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/biscuit-terminal/components.md
packages_during_phase_5:
  - biscuit-terminal
source_code:
  - biscuit-terminal/lib/src/components/table/cell.rs
  - biscuit-terminal/lib/src/components/table/table.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
  - biscuit-terminal/lib/src/components/filesystem/mod.rs
  - biscuit-terminal/lib/tests/prose_cells_parity.rs
  - biscuit-terminal/lib/tests/filesystem_parity.rs
  - renderable/src/tree/attrs.rs
documentation:
  - biscuit-terminal/lib/src/components/table/README.md
  - biscuit-terminal/docs/components/prose.md
  - .claude/skills/biscuit-terminal/components.md
packages:
  - biscuit-terminal
  - renderable
---

# Prose Table Cells — Execution Plan

Adds `StyledProse(Box<Prose>)` to `TableCellContent` so callers can embed
styled, capability-aware inline content in table cells without pre-rendering
to terminal bytes during construction.

> **Payload note (review-1).** The spec wrote the variant as
> `StyledProse(Prose)`; the implementation boxes it (`StyledProse(Box<Prose>)`).
> This is an intentional, approved decision: `Prose` embeds a full `Layout`, so
> an inline payload would make `TableCellContent` an order of magnitude larger
> than its other (≤24-byte) variants and trip `clippy::large_enum_variant` for
> every cell in the `Vec<Vec<TableCellContent>>` grid. `From<Prose>` boxes for
> callers, so the allocation is invisible at the construction site.

Source: `renderable/features/2026-06-08-prose-cells/spec.md`

---

## Phase 1 — Enum Extension

Add the `StyledProse` variant and `From<Prose>` conversion so the codebase
compiles with the new variant. Every exhaustive match site gains a
compile-failing arm that must be resolved in this phase or Phase 2.

### Tasks

- [x] Add `StyledProse(Box<Prose>)` variant to `TableCellContent` in
  `biscuit-terminal/lib/src/components/table/cell.rs` (boxed — see payload note
  above)
- [x] Add `From<Prose> for TableCellContent` in `cell.rs`, producing
  `StyledProse`
- [x] Update `Display for TableCellContent` — `StyledProse` uses
  `Prose::render_optimistic(None)` as the compatibility fallback
- [x] Update `cell_content_kind` in `table.rs:1828` — return
  `"styled_prose"` for the new variant
- [x] Update `cell_content_raw_value` in `table.rs:1838` — return
  `serde_json::Value::Null` for `StyledProse`
- [x] Update doc comment on the `TableCellContent` enum to list the new
  variant

### Validation

- [x] `cargo check -p biscuit-terminal` passes (no remaining match errors)
- [x] Existing Table tests compile and pass

---

## Phase 2 — Canonical Tree Projection

Update `to_render_tree_node` so `StyledProse` projects its inline
`RenderNode` children directly into the `TableCell`, with fenced-code
degradation. Update `reconstruct_cell` to handle the `"styled_prose"` hint.

### Tasks

- [x] In `Table::to_render_tree_node` (`table.rs:1464-1491`), branch on the
  new variant:
  - `StyledProse(prose)` → call `prose.to_render_nodes()`, degrade any
    top-level `NodeKind::Code` children to escaped literal text, pass the
    remaining inline nodes as the cell's children (instead of the current
    single `RenderNode::text(content.to_string())`)
  - All other variants keep their existing single-text-child projection
- [x] In `reconstruct_cell` (`render.rs:2421-2451`), handle
  `kind == "styled_prose"` by returning
  `TableCellContent::Text(rendered_text)` — the structured children have
  already been resolved for the active terminal
- [x] Update `TableCellHints::kind` doc in
  `renderable/src/tree/attrs.rs:645` to include `"styled_prose"`
- [x] Implement the fenced-code degradation: when a top-level child from
  `to_render_nodes()` is `NodeKind::Code`, replace it with a `Text` node
  containing the code body as escaped literal text

### Validation

- [x] `cargo check -p biscuit-terminal -p renderable` passes
- [x] Existing table render-tree tests pass
- [x] `cargo check` across the full workspace (no downstream breakage from
  `reconstruct_cell` changes)

---

## Phase 3 — Terminal Bespoke Path Resolution

Ensure both bespoke rendering paths (`render_content` and
`render_with_cursor_positioning`) resolve every `StyledProse` cell into
`Text` **once**, before any width planning, so the ANSI-aware table
machinery sees a uniform `Vec<Vec<TableCellContent>>` of resolved content.

### Tasks

- [x] Add a private helper `resolve_data_for_bespoke(&self, term: &Terminal)
  -> Vec<Vec<TableCellContent>>` that clones `self.data` and replaces every
  `StyledProse` cell with
  `TableCellContent::Text(prose.render(term))`
- [x] Update `render_bespoke` (`table.rs:1585-1609`) to call
  `resolve_data_for_bespoke` at entry, then pass the resolved data to
  `render_content` / `render_with_cursor_positioning`
- [x] Update `render_content` (`table.rs:914`) and
  `render_with_cursor_positioning` (`table.rs:1159`) to accept the resolved
  data as a parameter instead of reading `self.data` directly, OR
  alternatively, restructure `render_bespoke` to build a temporary `Table`
  with resolved data and delegate to its rendering methods (whichever
  involves less refactoring)
- [x] Ensure `render_optimistic` follows the same single-resolution rule
  through its optimistic `Terminal`
- [x] Verify that `StyledProse` cells render identically via both
  `render_content` (space-padded) and `render_with_cursor_positioning`
  (cursor-positioning) bespoke paths

### Validation

- [x] `cargo test -p biscuit-terminal` passes
- [x] A hand-checked terminal rendering of a table mixing `StyledProse`,
  `Integer`, and `Text` cells produces correct visible output with no style
  bleeding into borders/padding

---

## Phase 4 — Verification Tests

Unit tests covering every acceptance criterion from the spec.

### Tasks

- [x] **API test** — `Prose::new("...").into()` produces
  `TableCellContent::StyledProse(_)`
- [x] **Projection test** — a styled cell projects semantic
  `Strong`/`Emphasis`/`Delete`/`Link`/`Span` children rather than a flat
  text node containing Prose source or ANSI
- [x] **Hint test** — projected cell has `kind == "styled_prose"` and
  `raw_value == null`
- [x] **Fenced-code degradation test** — Prose containing a fenced code block
  produces degraded text nodes; the projected table passes render-tree
  validation
- [x] **Layout isolation test** — Prose's outer `Layout` does not become
  nested cell layout
- [x] **Terminal no-color test** — using an explicitly constructed
  `ColorDepth::None` terminal profile, styled cells emit no color/style SGR
- [x] **Terminal multiline/wrap test** — styled cells in a wrapped multiline
  table are measured by visible width and style does not bleed into padding,
  borders, or adjacent rows
- [x] **Mixed-type row test** — a table with `StyledProse`, `Integer`,
  `Float`, and `Currency` cells retains typed formatting and alignment
- [x] **Both bespoke paths test** — standard tree rendering and the
  cursor-alignment bespoke path produce equivalent visible content
- [x] **Single-resolution test** — instrumented/focused test verifying the
  bespoke path resolves each Prose cell exactly once before planning
- [x] **Browser render test** — browser output preserves semantic emphasis,
  links, and supported style attributes inside `<td>`
- [x] **Markdown render test** — portable Markdown matches standalone Prose
  semantics for bold, italic, strikethrough, links, and escaping; pipe
  characters and line breaks do not corrupt the GFM table structure
- [x] **MarkdownPlus render test** — matches standalone Prose's richer style
  behavior
- [x] **Compatibility regression test** — all existing Table and Prose tests
  pass unchanged

### Validation

- [x] `cargo nextest run -p biscuit-terminal -p renderable` — all tests green
- [x] No test mutates global state (`NO_COLOR`, env vars); all terminal
  profiles are explicitly constructed

---

## Phase 5 — Documentation

Update public docs, skill docs, and code comments.

### Tasks *(parallelizable with Phase 4)*

- [x] Update `biscuit-terminal/lib/src/components/table/README.md` with the
  `StyledProse` variant and a two-column Prose-cell example
- [x] Update `.claude/skills/biscuit-terminal/components.md` to mention Prose
  cells
- [x] Update `biscuit-terminal/docs/components/prose.md` to document inline
  Prose in table cells and the table-owned layout rule
- [x] Review rustdoc on `TableCellContent`, `cell_content_kind`,
  `reconstruct_cell`, and the `to_render_tree_node` doc comment for drift
  from the old "every cell is one text child" contract; correct in the same
  change
- [x] Verify `AGENTS.md` and skill docs remain consistent (no action needed
  if the feature is fully internal to biscuit-terminal + renderable)

### Validation

- [x] `cargo doc -p biscuit-terminal --no-deps` produces no new warnings
- [x] All updated documentation accurately describes the implemented contract

---

## Review 1 Resolutions

All findings from `review-1.md` addressed:

- **High — no-color SGR.** `emphasis_sgr` now returns empty under
  `ColorDepth::None` (style escapes suppressed alongside color, per spec line
  214), and a new `style::appearance_close` makes every inline close path emit a
  reset *only* when something was opened — so no stray `ESC` survives. The
  no-color test now asserts the full output (both the standard tree path and the
  cursor-alignment bespoke path) contains no escape character at all. Two prose
  underline tests that pinned the old stray-reset behavior were corrected.
- **High — Level 2 verification.** Added `bt table --prose-row` and
  `--cursor-align` CLI surface and `biscuit-terminal/cli/tests/level2_prose_cells.rs`
  (WezTerm/Kitty/tmux): styled-cell SGR survival, independent per-row styling,
  border/geometry integrity, and the cursor-alignment path emitting CSI
  column-moves with styling intact.
- **High — browser computed style.** Added real headless-Chrome computed-style
  tests (`browser_prose_cell_{color,background,underline}_computes`) asserting
  the styled run inside the `<td>` computes its color/background/underline.
- **Medium — resolve-once measurement.** `render_bespoke` resolves Prose
  in-place once and returns the count via `render_bespoke_instrumented`; the test
  asserts the real path resolves each `StyledProse` cell exactly once before
  planning (typed/text cells excluded).
- **Medium — Markdown/MarkdownPlus parity.** A shared corpus now compares the
  rendered cell body byte-for-byte against standalone Prose for both dialects
  (emphasis, links with significant URL chars, literal sigils, color, background,
  underline). This surfaced and fixed a real divergence: a cell link destination
  now applies CommonMark destination escaping (parens/backslashes) in addition
  to GFM pipe/newline safety, matching standalone Prose.
- **Medium — enum payload shape.** `StyledProse(Box<Prose>)` documented as
  intentional (see payload note above).
- **Recommendation — double clone.** `render_bespoke` resolves Prose cells in
  place on a single clone instead of cloning the whole table twice.
- **Recommendation — filesystem edits.** The `filesystem/mod.rs` +
  `filesystem_parity.rs` edits in this worktree are **load-bearing**, not stray:
  the committed `GitignoreMatcher` wiring (`f015bf6c5`) makes the scanner emit
  dim for ignored entries, so the test had to be updated to match. They are left
  in place (reverting would break `cargo test`) and should be committed
  separately for scope hygiene.

### Bleed fix (surfaced by the L2 work)

The multiline-cell L2/L1 coverage exposed a real defect the ANSI-stripping L1
test could not see: a multi-line `StyledProse` cell's SGR run was reset only at
the end of the *last* line, so bold/color bled into the right border, the next
row's left border, and padding. `wrap_cell_content` now runs
`sanitize_wrapped_lines` on the `WordWrap::None` (explicit-newline) path too, and
`active_ansi_state` was generalized from foreground-only to a full SGR-state
tracker (emphasis + foreground + background), so every wrapped/multiline cell
line is independently balanced. This also fixes color/bold bleed in standalone
Prose wrapping.
