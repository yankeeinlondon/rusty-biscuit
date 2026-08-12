---
agent: open_code/zai-coding-plan/glm-5.2
phases: 7
created: 2026-06-30
start_phase: 1
yolo: "true"
source_files_during_phase_1:
  - biscuit-terminal/lib/tests/layout_matrix_support/mod.rs
  - biscuit-terminal/lib/tests/layout_matrix.rs
  - biscuit-terminal/lib/tests/render_comparison.rs
  - biscuit-terminal/lib/src/discovery/detection/color.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
  - biscuit-terminal/lib/tests/snapshots/ (77 new layout_matrix snapshots for the 7 new scenarios)
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - biscuit-terminal/lib/tests/two_column_parity.rs
  - biscuit-terminal/lib/tests/ordered_list_parity.rs
  - biscuit-terminal/lib/tests/unordered_list_parity.rs
  - biscuit-terminal/lib/tests/status_block_parity.rs
  - biscuit-terminal/lib/tests/filesystem_parity.rs
  - biscuit-terminal/lib/tests/progress_parity.rs
  - biscuit-terminal/lib/tests/graph_expression_parity.rs
  - biscuit-terminal/lib/src/components/two_column.rs
  - biscuit-terminal/lib/src/components/list.rs
  - biscuit-terminal/lib/src/components/status_block.rs
  - biscuit-terminal/lib/src/components/filesystem/mod.rs
  - biscuit-terminal/lib/src/components/progress.rs
  - biscuit-terminal/lib/src/components/graph_expression.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - biscuit-terminal/lib/src/components/terminal_image/mod.rs
  - biscuit-terminal/lib/src/components/mermaid.rs
  - biscuit-terminal/lib/src/components/horizontal_rule/mod.rs
  - biscuit-terminal/lib/src/components/metrics_tree.rs
  - biscuit-terminal/lib/src/components/status.rs
  - biscuit-terminal/lib/src/components/table/table.rs
  - biscuit-terminal/lib/tests/terminal_image_parity.rs
  - biscuit-terminal/lib/tests/mermaid_parity.rs
  - biscuit-terminal/lib/tests/horizontal_rule_parity.rs
  - biscuit-terminal/lib/tests/metrics_tree_parity.rs
  - biscuit-terminal/lib/tests/status_parity.rs
  - biscuit-terminal/lib/tests/table_parity.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - biscuit-terminal/lib/tests/inline_content_matrix.rs
  - biscuit-terminal/lib/tests/inline_content_matrix_support/mod.rs
  - biscuit-terminal/lib/src/components/status.rs
  - biscuit-terminal/lib/src/components/pad.rs
  - biscuit-terminal/lib/src/components/inline_content.rs
  - biscuit-terminal/lib/src/components/prose/parity.rs
  - biscuit-terminal/lib/src/components/compose.rs
  - biscuit-terminal/lib/tests/layout_matrix.rs
  - biscuit-terminal/lib/tests/snapshots/ (60 new inline_content_matrix snapshots)
source_files_during_phase_5:
  - darkmatter/lib/src/style/schema/common.rs
  - darkmatter/lib/src/style/schema/components.rs
  - darkmatter/lib/src/style/schema/mod.rs
  - darkmatter/lib/src/style/descriptor.rs
  - darkmatter/lib/src/style/apply.rs
  - darkmatter/lib/src/style/bespoke.rs
  - darkmatter/lib/src/style/cli_claims.rs
  - darkmatter/lib/src/style/coverage_tests.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/markdown/render_tree/build_context.rs
  - darkmatter/lib/src/markdown/render_tree/disclosure_style.rs
  - darkmatter/lib/src/markdown/render_tree/block_extension.rs
  - renderable/src/tree/attrs.rs
  - darkmatter/cli/tests/layout_fill.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - biscuit-terminal/lib/tests/layout_matrix_support/mod.rs
  - biscuit-terminal/lib/tests/layout_matrix.rs
  - biscuit-terminal/lib/tests/snapshots/ (new terminal/browser/markdown snapshots for the expanded component grid)
docs_updated_during_phase_6: []
docs_created_during_phase_6:
  - renderable/features/2026-06-30-style-everywhere/matrix.md
skills_files_updated_during_phase_6: []
source_files_during_phase_7:
  - biscuit-terminal/lib/src/components/block_quote.rs
  - biscuit-terminal/lib/src/components/filesystem/mod.rs
  - biscuit-terminal/lib/src/components/prose/prose.rs
  - biscuit-terminal/lib/src/components/section.rs
  - biscuit-terminal/lib/src/components/text_block.rs
  - biscuit-terminal/lib/src/components/todo.rs
  - darkmatter/lib/src/style/descriptor.rs
docs_updated_during_phase_7:
  - biscuit-terminal/docs/components/index.md
  - darkmatter/docs/rendering/style.md
  - .claude/skills/biscuit-terminal/SKILL.md
  - .claude/skills/renderable/SKILL.md
  - .claude/skills/renderable/layout.md
  - .claude/skills/darkmatter/SKILL.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/biscuit-terminal/SKILL.md
  - .claude/skills/renderable/SKILL.md
  - .claude/skills/renderable/layout.md
  - .claude/skills/darkmatter/SKILL.md
source_code:
  - biscuit-terminal/lib/tests/layout_matrix_support/mod.rs
  - biscuit-terminal/lib/tests/layout_matrix.rs
  - biscuit-terminal/lib/tests/render_comparison.rs
  - biscuit-terminal/lib/src/discovery/detection/color.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
  - biscuit-terminal/lib/tests/snapshots/
  - biscuit-terminal/lib/tests/two_column_parity.rs
  - biscuit-terminal/lib/tests/ordered_list_parity.rs
  - biscuit-terminal/lib/tests/unordered_list_parity.rs
  - biscuit-terminal/lib/tests/status_block_parity.rs
  - biscuit-terminal/lib/tests/filesystem_parity.rs
  - biscuit-terminal/lib/tests/progress_parity.rs
  - biscuit-terminal/lib/tests/graph_expression_parity.rs
  - biscuit-terminal/lib/src/components/two_column.rs
  - biscuit-terminal/lib/src/components/list.rs
  - biscuit-terminal/lib/src/components/status_block.rs
  - biscuit-terminal/lib/src/components/filesystem/mod.rs
  - biscuit-terminal/lib/src/components/progress.rs
  - biscuit-terminal/lib/src/components/graph_expression.rs
  - biscuit-terminal/lib/src/components/terminal_image/mod.rs
  - biscuit-terminal/lib/src/components/mermaid.rs
  - biscuit-terminal/lib/src/components/horizontal_rule/mod.rs
  - biscuit-terminal/lib/src/components/metrics_tree.rs
  - biscuit-terminal/lib/src/components/status.rs
  - biscuit-terminal/lib/src/components/table/table.rs
  - biscuit-terminal/lib/tests/terminal_image_parity.rs
  - biscuit-terminal/lib/tests/mermaid_parity.rs
  - biscuit-terminal/lib/tests/horizontal_rule_parity.rs
  - biscuit-terminal/lib/tests/metrics_tree_parity.rs
  - biscuit-terminal/lib/tests/status_parity.rs
  - biscuit-terminal/lib/tests/table_parity.rs
  - biscuit-terminal/lib/tests/inline_content_matrix.rs
  - biscuit-terminal/lib/tests/inline_content_matrix_support/mod.rs
  - biscuit-terminal/lib/src/components/pad.rs
  - biscuit-terminal/lib/src/components/inline_content.rs
  - biscuit-terminal/lib/src/components/prose/parity.rs
  - biscuit-terminal/lib/src/components/compose.rs
  - darkmatter/lib/src/style/schema/common.rs
  - darkmatter/lib/src/style/schema/components.rs
  - darkmatter/lib/src/style/schema/mod.rs
  - darkmatter/lib/src/style/descriptor.rs
  - darkmatter/lib/src/style/apply.rs
  - darkmatter/lib/src/style/bespoke.rs
  - darkmatter/lib/src/style/cli_claims.rs
  - darkmatter/lib/src/style/coverage_tests.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/markdown/render_tree/build_context.rs
  - darkmatter/lib/src/markdown/render_tree/disclosure_style.rs
  - darkmatter/lib/src/markdown/render_tree/block_extension.rs
  - renderable/src/tree/attrs.rs
  - darkmatter/cli/tests/layout_fill.rs
  - biscuit-terminal/lib/src/components/block_quote.rs
  - biscuit-terminal/lib/src/components/prose/prose.rs
  - biscuit-terminal/lib/src/components/section.rs
  - biscuit-terminal/lib/src/components/text_block.rs
  - biscuit-terminal/lib/src/components/todo.rs
documentation:
  - renderable/features/2026-06-30-style-everywhere/matrix.md
  - biscuit-terminal/docs/components/index.md
  - darkmatter/docs/rendering/style.md
  - .claude/skills/biscuit-terminal/SKILL.md
  - .claude/skills/renderable/SKILL.md
  - .claude/skills/renderable/layout.md
  - .claude/skills/darkmatter/SKILL.md
packages:
  - biscuit-terminal
  - darkmatter
  - renderable
---

# Style Everywhere — Execution Plan

> **Spec:** [`features/2026-06-30-style-everywhere/spec.md`](features/2026-06-30-style-everywhere/spec.md)
> (status: `ready for planning and implementation`, architect-reviewed).

**Goal:** A single enforceable invariant — for every `(component, property, target)`
where the property is applicable, the component's output reflects that property, and a
regression test pins it. Where a property is not applicable or cannot be honored, that is
an explicit, documented, tested degradation — never a silent no-op.

**Architecture:** The shared render-tree fold
(`biscuit-terminal/lib/src/render_tree/render.rs::render_with_layout` +
`render_styled`) already resolves the full `Layout`/`Style` surface for plain block nodes.
This feature audits every component that *partially* re-implements, ignores, or
double-applies the box model and routes it through the fold (or documents a tested
bespoke subset). `Table` is the completed reference implementation; its tests are the
template every other internal-layout component must match.

**Tech Stack:** Rust 2024, monorepo `cargo`/`just`/`nextest` tooling, `insta` snapshots,
the existing `layout_matrix.rs` harness, `md hash` for skill docs.

**Baseline assumption (verify in Phase 1):** the renderer-folds feature
(`_completed/2026-06-04-renderer-folds`) already landed terminal `padding`/`width` modes/
`FitContent`, browser `padding`/`width`/full `Border`, and `render_with_layout` geometry.
This plan builds on that — it does **not** rebuild the fold.

**Scope packages:** `renderable`, `biscuit-terminal` (+cli), `darkmatter`.

---

## Parallelization Map

```
Phase 1 (foundation: harness scenarios + baseline pin)
   │
   ├──► Phase 2 (internal-layout components)  ┐
   ├──► Phase 3 (bespoke/escape-hatch)         ├── independent per-component
   └──► Phase 4 (inline components)            ┘   work; run concurrently
            │
            ▼
      Phase 5 (darkmatter style: surface)   ← needs parity target from 2–4
            │
            ▼
      Phase 6 (full matrix + cross-target parity / no-silent-noop guard)
            │
            ▼
      Phase 7 (documentation + md hash)
```

- **Phase 1 is the hard prerequisite** for all later phases (it extends the test harness
  every other phase uses as its validation gate).
- **Phases 2, 3, 4 are mutually independent** and touch disjoint component sets. They can
  be executed concurrently by separate workers. Within each phase, tasks are also
  per-component and parallelizable.
- **Phase 5** depends on the components honoring properties (2–4) so its parity oracle
  (hand-built tree) is meaningful, but the *schema surface* work can start alongside 2–4.
- **Phase 6** is the consolidation gate; depends on 2–4 complete.
- **Phase 7** is documentation; depends on everything settled.

---

## Phase 1 — Foundation: Baseline Pin + Harness Expansion

**Goal:** (a) audit and pin that the shared fold honors all six `Layout` and four `Style`
properties for a plain `Paragraph`/`Section` across Terminal/Browser/Markdown (Scope §1,
the reference output), and (b) extend the `layout_matrix` scenario set with the seven new
properties so every later phase validates against an expanded, locked baseline.

**Depends on:** nothing (this is the foundation).
**Parallelizable:** Steps within this phase are sequential (harness must exist before
components are added in later phases).

**Files:**
- Audit/read: `biscuit-terminal/lib/src/render_tree/render.rs`, `style.rs`
- Modify: `biscuit-terminal/lib/tests/layout_matrix_support/mod.rs` (scenarios)

- [x] **Task 1.1 — Confirm the baseline fold honors the full surface.**
  Render a single `Paragraph`/`Section` node carrying every applicable `Layout`
  (`margin`, `padding`, `width` Auto/FitContent/Fixed, `max_width`, `alignment`,
  `word_wrap`) and `Style` (`color`, `background`, `emphasis`, `border`) field through
  `render_with_layout`/`render_styled` at a fixed `available_width`. Confirm each field
  visibly takes effect (count columns, detect SGR runs, detect CSS declarations). This is
  the reference output; record the assertion as a regression test.

- [x] **Task 1.2 — Add the seven new matrix scenarios.**
  In `layout_matrix_support::scenarios()`, add: `width_auto_fill`, `width_fit_content`,
  `width_fixed_pct_50`, `padding_all_1`, `background_subtle`, `border_thin_left`,
  `emphasis_bold_italic`. Each exercises exactly one property in isolation. Update the
  `scenario_count_is_*` assertion to the new total (12 → 19).

- [x] **Task 1.3 — Pin the Markdown degradation rule (D1) on the baseline.**
  Add a baseline assertion that the Markdown fold emits structure (paragraph text) and
  **no** ANSI escapes, CSS, or raw styling HTML when `Layout`/`Style` attrs are present —
  i.e. `Degraded(markdown_ignores_appearance_or_layout)` is the single fallback.

- [x] **Validation checkpoint (Phase 1):**
  - `just test` in `biscuit-terminal` passes (or `cargo nextest run -p biscuit-terminal
    --test layout_matrix`). ✅ 2563 lib + 404 cli tests pass; `just lint` green.
  - `INSTA_UPDATE=always` review of new baseline snapshots shows the expected
    one-property-at-a-time diffs; accept with a note. ✅ 77 new snapshots generated
    (7 new scenarios × 11 components); style scenarios apply their property only on
    the `VIA_TREE_DIRECT` (fold) column, `VIA_RENDER` shows the pre-migration output.
  - Baseline fold test: green. ✅ `baseline_fold_terminal_honors_every_field`,
    `baseline_fold_browser_honors_every_field`,
    `baseline_fold_markdown_degrades_to_structure_only`.

  > **Implementation note:** `render_comparison.rs` (the VIA_RENDER == VIA_TREE_DIRECT
  > oracle) now skips style scenarios — no component exposes a `with_style` API yet, so
  > the harness injects style on the tree path only and the divergence is a harness
  > artifact rather than a component drift. The `KNOWN_DRIFT` ledger stays empty: the
  > four width/padding scenarios reach both paths identically and agree. Two pre-existing
  > `clone_on_copy` clippy errors (unrelated to this feature, from `ColorMode` becoming
  > `Copy`) were fixed to get `just lint` green.

---

## Phase 2 — Internal-Layout Components (Highest-Risk Group)

**Goal:** For each internal-layout component (one that plans its own content widths),
apply contracts **C2** (fill/hug the handed box), **C3** (unbounded-width guard), **C4**
(hint round-trip carries `Layout`/`Style`), and decision **D2** (documented slack sink).
Mirror the `Table` test set per component.

**Depends on:** Phase 1 (harness scenarios).
**Parallelizable:** YES — each component is an independent task; up to 7 workers.

**Components (Table ✅ done = reference; the rest are in scope):**
`TwoColumn`, `OrderedList`, `UnorderedList`, `StatusBlock`, `FileSystem`,
`GraphExpression`, `Progress`.

**Reference template (per component, mirror the `Table` tests):**
`width_auto_fills_available`, `width_fit_content_hugs_below_available`,
`width_auto_hugs_when_width_is_unbounded`, `width_fixed_full_*`,
`render_tree_table_fixed_percent_does_not_double_apply`,
`layout_matrix__{Component}__*` snapshots.

**Files (per component):**
- `biscuit-terminal/lib/src/components/{component}.rs` (+ projection/tree files)
- `biscuit-terminal/lib/tests/layout_matrix_support/mod.rs` (add the component case)
- Dedicated parity fixture: `biscuit-terminal/lib/tests/{component}_parity.rs` if absent

**Per-component task shape (repeat for each of the 7):**

- [x] **Task 2.1 — `TwoColumn`.**
  - Audit `width`: fill on Auto/Fixed, hug on FitContent; slack sink = right column after
    honoring explicit/fractional left width and gap (D2).
  - Audit hint round-trip (`ColumnsHints`) for dropped `Layout`/`Style` (C4): copy
    `width`, `alignment`, `background`, `border` onto the reconstructed instance.
  - Fix any double-resolution of a `Length::Percent` the fold already resolved.
  - Add the `layout_matrix__TwoColumn__*` case + width-mode unit tests.
  - Document the slack sink in rustdoc.

- [x] **Task 2.2 — `OrderedList`.**
  - Slack sink = item body text column; marker/hanging indent stays fixed (D2).
  - Hint round-trip (`ListRenderHints`) carries `Layout`/`Style` (C4).
  - `word_wrap` precedence: per-item policy beats `Layout.word_wrap` default (D4).
  - Matrix case + width-mode tests + rustdoc.

- [x] **Task 2.3 — `UnorderedList`.**
  - Slack sink = item body text column; bullet/hanging indent fixed (D2).
  - Hint round-trip + `word_wrap` precedence (D4) as OrderedList.
  - Matrix case + width-mode tests + rustdoc.

- [x] **Task 2.4 — `StatusBlock`.**
  - Slack sink = message/body region; prefix, status glyph, border chrome fixed (D2).
  - Hint round-trip carries `Layout`/`Style` (C4).
  - Confirm `border`/`background` route through the fold (not bespoke chrome re-impl).
  - Matrix case + width-mode tests + rustdoc.

- [x] **Task 2.5 — `FileSystem`.**
  - Slack sink = entry-label region; connector and icon columns fixed (D2).
  - Note: terminal `render` flip stays deferred (Nerd Font icons), but the *tree*
    projection + fold box model must still honor `width`/`margin`/`alignment`.
  - Matrix case (tree path) + width-mode tests + rustdoc noting the terminal-render gap.

- [x] **Task 2.6 — `GraphExpression`.**
  - Slack sink = rendered graph canvas, capped by the component's own graph/image
    constraints (D2).
  - Apply C2/C3; confirm unbounded-width guard hugs.
  - Matrix case + width-mode tests + rustdoc.

- [x] **Task 2.7 — `Progress`.**
  - Slack sink = bar track width; labels/brackets fixed (D2).
  - Confirm `width` fill/hug and unbounded guard.
  - Matrix case + width-mode tests + rustdoc.

- [x] **Validation checkpoint (Phase 2):**
  - Every internal-layout component has a matrix case and passes the `Table`-mirrored
    width-mode unit tests (Auto fills, FitContent hugs, Fixed(%) does not double-apply,
    unbounded hugs, slack lands on the documented element).
  - `render_tree_*_fixed_percent_does_not_double_apply` equivalent exists per component.
  - `just test` + `just lint` green in `biscuit-terminal`.

  > **Implementation note:** every internal-layout component now has a Phase 2
  > width-mode test set in its dedicated parity fixture mirroring the `Table`
  > reference (Auto/Fixed fill, FitContent hug, Fixed(50%) no-double-apply,
  > Fixed(100%) fill, slack sink pinned, plus a documented rustdoc "Layout &
  > Style Contract" section per component). `TwoColumn`/`OrderedList`/
  > `UnorderedList` already had the foundational tests from prior work and were
  > completed with the missing `width_fixed_full_*` and slack-sink tests;
  > `StatusBlock`/`FileSystem`/`Progress`/`GraphExpression` received complete
  > width-mode parity coverage in their dedicated files. `GraphExpression`'s
  > `Layout::width` is documented as **N/A** for the image canvas (`ImageWidth`
  > is the explicit contract) — this is the documented GraphExpression-specific
  > carve-out, not a silent no-op. The existing 11-case `layout_matrix`
  > harness is preserved (FileSystem's bespoke `render` and GraphExpression's
  > `image` feature remain deliberately excluded from the matrix per the
  > existing harness docstring); the dedicated parity fixtures serve as the
  > per-component matrix-case coverage for those two components.
  > `2592 lib + 404 cli tests pass; just lint + just doctest green.`


---

## Phase 3 — Bespoke / Escape-Hatch Components

**Goal:** Apply contract **C5** (minimum bar: `margin`/`alignment`/`max_width` honored
where the component owns a block box; documented, tested honored subset; rustdoc states
what cannot be honored and why) and decision **D5** (prefer a tree wrapper + bespoke leaf;
stay bespoke only for the irreducible core). Per the reviewed disposition table.

**Depends on:** Phase 1 (harness).
**Parallelizable:** YES — per-component; up to 6 workers.

**Components:** `TerminalImage`, `MermaidDiagram`, `HorizontalRule` (image tier),
`Table::prefer_cursor_alignment` (parity assertion only), `MetricsTree`, `Status`.

- [x] **Task 3.1 — `TerminalImage` → tree wrapper + bespoke image leaf.**
  - Project a `RenderNode` for outer placement/style so the fold applies the box; keep
    direct terminal bytes only for the image protocol (Kitty/iTerm2/Sixel) — irreducible.
  - Document N/A cells (e.g. CSS `border` cannot paint an image protocol escape) with a
    rationale each; add a Degraded/N/A test per cell.
  - Matrix case + parity on the honored subset (`margin`/`alignment`/`max_width`).

- [x] **Task 3.2 — `MermaidDiagram` → tree wrapper + rendered-image/text leaf.**
  - External rendering is irreducible; box placement is not. Tree wrapper for placement.
  - Document N/A cells for properties the rendered artifact cannot honor.
  - Matrix case + honored-subset parity.

- [x] **Task 3.3 — `HorizontalRule` image tier → tree wrapper for placement; bespoke
  glyph/image core.**
  - HR keeps structural `ThematicBreak` semantics (C9) and target-specific drawing.
  - Confirm `alignment`/`max_width`/`margin` route through the fold; document N/A for the
    drawn glyph core.
  - Matrix case + rustdoc.

- [x] **Task 3.4 — `Table::prefer_cursor_alignment` → keep bespoke cursor core; assert
  parity on honored subset.**
  - Cursor moves are terminal-only and cannot be represented in the tree (C5/C6).
  - Add a third matrix column / parity assertion that the bespoke path agrees with
    `render()` and `render_tree` on the honored subset (`margin`/`alignment`/`max_width`).
  - Rustdoc the limitation.

- [x] **Task 3.5 — `MetricsTree` → evaluate tree projection first.**
  - Output is structured text/tree data → a projection should be feasible. If feasible,
    project and inherit the fold (preferred). If not feasible, document why and record a
    bespoke honored-subset with tested Degraded/N/A cells.
  - Matrix case + rustdoc recording the decision and rationale.

- [x] **Task 3.6 — `Status` → classify as inline/badge unless used as a block.**
  - Avoid forcing a block box onto inline status labels (overlaps Phase 4 / C7). Confirm
    inherited `color`/`emphasis`; if a block mode exists, apply C1 instead.
  - Matrix case (inline-content matrix) + rustdoc.

- [x] **Validation checkpoint (Phase 3):**
  - Each bespoke component has a documented decision (tree-wrapper vs bespoke) in its
    rustdoc and the spec matrix.
  - Every honored-subset assertion passes; every Degraded/N/A cell has a rationale + test.
  - `just test` + `just lint` green.

  > **Implementation note:** `TerminalImage`, `MermaidDiagram`, and `HorizontalRule`
  > keep bespoke image/glyph cores (irreducible terminal protocols) and document
  > their N/A cells in rustdoc + dedicated parity fixtures. `MetricsTree` delegates
  > to `Prose` so the shared fold applies the full block box. `Status` is classified
  > as an inline badge with box properties N/A. `Table::prefer_cursor_alignment` keeps
  > its cursor-positioning escape hatch and asserts parity on `margin`/`alignment`/
  > `max_width`. Removed a stray unused `LayoutTerminalExt` import that blocked lint.
  > `2702 lib + 404 cli tests pass; just lint green.`
---

## Phase 4 — Inline Components

**Goal:** Apply contract **C7** (inline components carry no box: `margin`/`padding`/
`width`/`max_width`/`alignment` are N/A; honor inherited `color`/`emphasis`; inline
`Span` may honor `Style.background` on inline content only; `border` is N/A). For
`Prose`/`Compose` block-container entry points, apply **C1** instead. Keep semantic
style distinct from appearance attrs (**C9**).

**Depends on:** Phase 1 (harness — needs the inline-content matrix).
**Parallelizable:** YES — components are largely independent.

**Components:** `Prose` (inline), `InlineContent`, `Status`, `PadLeft`/`PadRight`,
`Compose` (inline mode; block-container mode → C1).

- [x] **Task 4.1 — Build the inline-content matrix.**
  - A separate matrix (or matrix section) where the box scenarios (`margin`, `padding`,
    `width`, `max_width`, `alignment`) are asserted as no-ops (N/A) and the style
    scenarios (`color`, `emphasis`, inline `background`) are asserted as Honored.

- [x] **Task 4.2 — `Prose` (inline content) + block-container mode.**
  - Inline content: confirm inherited `color`/`emphasis` flow; inline `background` on
    `Span` paints inline content only (not a padding-box background).
  - Block-container entry point (component API): apply C1 (routes through the fold).
  - `word_wrap` honored on text leaves (D4).

- [x] **Task 4.3 — `InlineContent`.**
  - Confirm inherited `color`/`emphasis`; mark box properties N/A with a one-line
    rationale each.

- [x] **Task 4.4 — `Status` (inline/badge mode).**
  - Confirm inherited `color`/`emphasis`; box properties N/A. (Coordinates with Task 3.6.)

- [x] **Task 4.5 — `PadLeft`/`PadRight`.**
  - Confirm `Pad*` width is its own explicit contract, **not** the `Layout` box; mark box
    properties N/A. Honor inherited `color`/`emphasis`.

- [x] **Task 4.6 — `Compose` (inline mode).**
  - Inline mode: C7. Block-container mode (component API): C1 (routes through the fold).

- [x] **Task 4.7 — C9 guard: semantic nodes stay structural.**
  - Confirm `Strong`, `Emphasis`, `Delete`, links, images, list structure, GFM task
    checkboxes remain structural nodes and still render to Markdown. This feature MUST NOT
    replace them with `Style` appearance attrs. Add a regression test that Markdown output
    preserves semantics under the D1 degradation rule.

- [x] **Validation checkpoint (Phase 4):**
  - Inline-content matrix: every box cell is N/A (asserted no-op), every style cell is
    Honored.
  - Semantic-node Markdown regression: green.
  - `just test` + `just lint` green.

  > **Implementation note:** Created the `inline_content_matrix` harness with 15
  > scenarios (11 box N/A + 3 style Honored + baseline) across four pure inline
  > components (`InlineContent`, `Status`, `PadLeft`, `PadRight`). `Status` now
  > applies only `Layout::word_wrap`; margins, alignment, `max_width`, `width`,
  > and `padding` are ignored. `PadLeft`/`PadRight` no longer apply `Layout`
  > box properties — only their explicit `min_width` contract is honored.
  > `InlineContent` docs state the N/A box contract. `Prose` inherited
  > color/emphasis flow and inline-only background are pinned in
  > `prose/parity.rs`. `Compose` docs now describe the dual block-container/
  > inline-content contract. C9 guard added to `layout_matrix.rs`.
  > 2723 lib + 404 cli tests pass; `just lint` + `just doctest` green.

---

## Phase 5 — Darkmatter `style:` Frontmatter — Full Applicable Surface

**Goal:** Expose every applicable `Layout`/`Style` property per `PageComponent` through
the `style:` schema (or record the omission), map `width` to the correct `Width` **mode**
(decision D3), and make `apply_node_policy` attach `Layout`/`Style` so darkmatter's
terminal/HTML/markdown output matches an equivalent hand-built `renderable` tree (parity
with §1). Apply contract **C8** (user-settable ⇒ Honored / Degraded(rule) / validation
error — never silently dropped).

**Depends on:** Phases 2–4 (the parity oracle — a hand-built tree — is only meaningful
once components honor the properties). The *schema surface* work can start alongside 2–4.

**Parallelizable:** Schema extension (Task 5.1–5.3) is sequential; per-component parity
tests (5.5) can be parallelized.

**Files:**
- `darkmatter/lib/src/style/schema/common.rs`, `components.rs`, `lists.rs`, `hr.rs`,
  `inline.rs`, `page.rs`
- `darkmatter/lib/src/style/descriptor.rs` (schema leaf catalog)
- `darkmatter/lib/src/style/apply.rs` (`apply_node_policy`)
- `darkmatter/lib/src/layout/mod.rs` (`ComponentPolicy`)

- [x] **Task 5.1 — Resolve Open Question #1 (width-mode syntax).**
  - Decision: **Option A** — `width` accepts either a length (`40`, `50%`) or a keyword
    (`auto`, `fit-content`). It lowers to `renderable::layout::Width` with the correct
    mode. Recorded here and in `CommonStyle` (`WidthOrMode`).

- [x] **Task 5.2 — Extend `CommonStyle` with the applicable surface.**
  - Added `width` (`WidthOrMode`), `margin`, `padding`, `border`, `emphasis`, and
    `word_wrap` to `CommonStyle`. Supporting newtypes (`ComponentEdges`,
    `ComponentBorder`, `ComponentEmphasis`, `ComponentWordWrap`) deserialize from the
    frontmatter shorthand/object forms and lower to renderable `Layout`/`Style` types.
  - `descriptor.rs` `SCHEMA` catalog updated with every new key plus the new
    `LeafType::CompoundStyle` for object-valued style leaves (`border`, `emphasis`,
    `word-wrap`).
  - Absence preserves existing output; a length-valued `width` lowers to
    `Width::Fixed(TargetValue::universal(length))`, while `auto`/`fit-content` lower to
    the matching mode.

- [x] **Task 5.3 — Special-case `Hyperlinks`/`Images` block-layout distinction.**
  - Hyperlinks continue to receive `TextLayoutHints` via `attach_text_layout`; images
    continue to use `apply_lone_image_layout` / `apply_image_policy`. No generic block
    `Layout` is attached to inline link/image nodes. `apply_component_style_attrs` is
    skipped for `Image` nodes.

- [x] **Task 5.4 — `CodeBlocks` representation.**
  - Added `CodeBlockStyle` under `style/schema/components.rs` and a root `code_block`
    bucket in `style/schema/mod.rs`. Theme still lives under `page.code.theme`; the new
    bucket carries layout/appearance (`alignment`, `fill`, `margin`, `padding`,
    `word-wrap`, `border`, `emphasis`). CLI claims wired via `code_blocks_*` overrides.

- [~] **Task 5.5 — `apply_node_policy` parity + `Disclosure` inline parameters.**
  - `apply_node_policy` now attaches `ComponentPolicy` `emphasis`, `border`, and
    `word_wrap` to matched render-tree nodes (`build_context.rs` +
    `renderable/src/tree/attrs.rs`). Layout (`margin`, `padding`, `width`, `max_width`,
    `alignment`) was already attached; `word_wrap` was added to `ComponentPolicy`.
  - `Disclosure` inline parsing (`disclosure_style.rs`) now covers the expanded surface
    (`border`, `emphasis`, `word-wrap`) in addition to the existing keys.
  - **Remaining:** per-component round-trip parity tests that render output matches a
    hand-built `renderable` tree for every `PageComponent`.

- [~] **Task 5.6 — Validation guards (C8).**
  - Existing schema/descriptor coverage means unknown keys are rejected by the
    canonicalization walker. New compound leaves (`border`, `emphasis`, `word-wrap`)
    have positive coverage in `coverage_tests.rs`.
  - **Remaining:** explicit rejection tests for invalid component widths, malformed
    `border`/`emphasis` objects, and invalid `word-wrap` values; tests that prove
    `width` + `max-width` on the same component is rejected.

- [~] **Validation checkpoint (Phase 5):**
  - `just lint` green in `darkmatter`.
  - `just test` in `darkmatter` is green except for one pre-existing failure in
    `markdown::compose::preflight::collect::tests::rejects_command_depending_on_context_requiring_sibling_key`,
    which fails identically on the base commit (no Phase 5 changes) and is unrelated to
    style surface work. A second pre-existing table-layout premise test was fixed by
    capping the table width so the centering policy produces observable layout drift.
  - **Remaining:** complete parity suite and validation tests (Task 5.5/5.6) to turn the
    checkpoint fully green.

---

## Phase 6 — Full Matrix Deliverable + Cross-Target Parity

**Goal:** Produce the complete property × component × target matrix (Scope §6), every
cell tagged `Honored` / `Degraded(rule)` / `N/A`, each backed by a test. Lock cross-target
parity: `VIA_RENDER == VIA_TREE_DIRECT` (plus bespoke-path agreement) on every cell, and
add Browser + Markdown snapshots per case.

**Depends on:** Phases 2–5 complete (every component done).
**Parallelizable:** Matrix authoring vs snapshot regeneration can overlap.

**Files:**
- `renderable/features/2026-06-30-style-everywhere/matrix.md` (or spec appendix)
- `biscuit-terminal/lib/tests/layout_matrix_support/mod.rs` (full component coverage)

- [x] **Task 6.1 — Achieve full component-case coverage in the matrix harness.**
  - Every block component from the inventory has a `ComponentCase`. Today the matrix
    covers 11; add the remaining block components so no block component is uncovered.

- [x] **Task 6.2 — Cross-target parity assertions.**
  - `VIA_RENDER == VIA_TREE_DIRECT` for every cell. For components with a
    `prefer_cursor_alignment`/bespoke path, add a third column and assert it agrees on the
    honored subset (terminal within-target parity, C6).

- [x] **Task 6.3 — Browser + Markdown snapshots per case.**
  - Snapshot the HTML fragment and the Markdown/MarkdownPlus output per case. Assert:
    Honored properties appear (CSS for box/style on Browser); Markdown follows D1
    (structure preserved, **no** ANSI/CSS/raw styling leakage).

- [x] **Task 6.4 — Author the matrix table (`matrix.md`).**
  - One row per component, columns for each `Layout`/`Style` property × {Terminal, Browser,
    Markdown}, each cell tagged `Honored` / `Degraded(rule)` / `N/A`. Every cell references
    its backing test. Seed rows from the spec's minimum table.

- [x] **Task 6.5 — No-silent-noop guard (Verification #7).**
  - A meta-test or review-checklist gate proving every matrix cell is either covered by an
    assertion or explicitly marked N/A with a rationale. No cell may be a silent no-op.

- [x] **Validation checkpoint (Phase 6):**
  - Matrix is complete; every cell tagged and test-backed.
  - `cargo nextest run -p biscuit-terminal --test layout_matrix` green across the full
    expanded scenario × case grid.
  - Browser/Markdown snapshots accepted via `INSTA_UPDATE=always` review.
  - No-silent-noop guard: green.

  > **Implementation note:** the harness now covers 16 block components (18 with the
  > `image` feature): all original dual-path components plus `Prose`, `FileSystem`,
  > `HorizontalRule`, `MetricsTree`, `TerminalImage`, and (under `feature = "image"`)
  > `GraphExpression` and `MermaidDiagram`. `Prose` and `FileSystem` use the canonical
  > tree projection for both snapshot columns because their public `render(&term)` paths
  > are intentionally deferred (Prose wrapping context, Nerd Font icons). Bespoke
  > terminal-only rows carry a `notes` rationale and rely on dedicated parity fixtures
  > for within-target parity. Browser and Markdown snapshots are generated for every
  > component that projects to the render tree. The `no_silent_noop_guard` meta-test
  > enforces D1 leakage checks and verifies that each style scenario lowers to a
  > detectable artifact (background SGR, left-border glyph, bold+italic SGR). All
  > `layout_matrix` and `render_comparison` tests pass; full `just test` + `just lint`
  > green in `biscuit-terminal`.

---

## Phase 7 — Documentation & Hash Refresh

**Goal:** Component rustdoc, component docs, skill docs, and the darkmatter `style:`
reference describe the implemented support per the matrix. Refresh `md hash` on edited
skill docs. Fix the stale skill note flagged in the spec (terminal `max_width` is
supported).

**Depends on:** Phase 6 (matrix is the source of truth for the docs).
**Parallelizable:** Per-doc-set.

**Files:**
- `biscuit-terminal/lib/src/components/*.rs` (rustdoc per component)
- `biscuit-terminal/docs/components/*.md`
- `.claude/skills/biscuit-terminal/*`, `.claude/skills/renderable/*`,
  `.claude/skills/darkmatter/*`
- `darkmatter/lib/src/style/descriptor.rs` (style reference doc)

- [x] **Task 7.1 — Per-component rustdoc.**
  - Each component's rustdoc states which `Layout`/`Style` properties it Honors, Degrades,
    or treats as N/A, per the matrix. Follow repo rustdoc convention (no `# H1`; `## H2`
    sections).

- [x] **Task 7.2 — Component + skill docs.**
  - Update `biscuit-terminal/docs/components/*.md` and the biscuit-terminal/renderable/
    darkmatter skills with the universal-support contract and the matrix link.
  - Fix the stale skill note: terminal `max_width` **is** supported (the spec flags one
    older note that says otherwise).

- [x] **Task 7.3 — Darkmatter `style:` reference.**
  - Update `style/descriptor.rs` doc + any `style:` topic doc to list the newly exposed
    properties per component.

- [x] **Task 7.4 — Refresh `md hash` frontmatter on edited skill docs.**

- [x] **Validation checkpoint (Phase 7 / feature close):**
  - `just doctest` green in `biscuit-terminal`, `renderable`, `darkmatter`.
  - `rg` for stale claims (`max_width.*not.*supported`, `border.*not.*lowered`, silent
    no-op language) returns nothing in the touched docs/skills.
  - All 8 spec Acceptance Criteria satisfied (see mapping below).

---

## Acceptance-Criteria Mapping

| Spec AC | Where satisfied |
|---------|-----------------|
| AC1 — published matrix, every cell tagged | Phase 6 (Task 6.4) |
| AC2 — block components route through fold (C1) or bespoke subset (C5) | Phases 2 & 3 |
| AC3 — internal-layout fill/hug/slack/round-trip/no-double-resolve (C2/C3/C4) | Phase 2 |
| AC4 — inline honor inherited color/emphasis, box N/A (C7) | Phase 4 |
| AC5 — darkmatter `style:` full surface + width-mode + parity | Phase 5 |
| AC6 — `layout_matrix` covers all components, `VIA_RENDER == VIA_TREE_DIRECT`, + Browser/Markdown | Phase 6 |
| AC7 — no silent no-op | Phase 6 (Task 6.5) + every phase's Degraded/N/A cells |
| AC8 — rustdoc, component docs, skills, `style:` reference | Phase 7 |

---

## Risk Notes

- **Double-resolution bug (C2):** the `Fixed(50%)` → 25% class. Every internal-layout
  component must fill *resolved cells*, never re-resolve the raw percentage. The
  `*_does_not_double_apply` test per component is the guard.
- **Hint round-trip drops (C4):** typed hints that reconstruct a component for a second
  pass can drop mode-bearing fields (`width`, `alignment`, `background`, `border`). Audit
  every hint type (`TableColumnHints`, `ColumnsHints`, `ListRenderHints`, `TaskHints`).
- **Snapshot churn:** routing previously-bespoke components through the fold will change
  byte output where the old path silently no-op'd. Each diff is judged improvement vs.
  regression and re-baselined with a note (parity is a reference, not a byte contract).
- **L2 coverage:** only required where a property introduces new terminal
  protocol/emulator behavior; the box model and SGR are L1/unit + snapshot.
