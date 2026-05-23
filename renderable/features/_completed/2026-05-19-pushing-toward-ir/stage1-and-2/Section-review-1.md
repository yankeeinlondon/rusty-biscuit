---
ready: false
---

# Section Component — Review 1

**Scope**
- `biscuit-terminal/lib/src/components/section.rs`
- `biscuit-terminal/cli/src/commands/section.rs`
- `biscuit-terminal/lib/tests/section_parity.rs`
- `biscuit-terminal/cli/tests/integration_test.rs` (Section subcommand coverage)

## Summary

The Section component has been fully flipped to the canonical render-tree path. All four target traits — `TerminalRenderable`, `TreeRenderable`, `MarkdownRenderable`, and `BrowserRenderable` — are implemented and delegate to a single private projection helper (`to_render_node`). The CLI command (`bt section`) exists with all required flags (`--html`, `--md`, `--md-plus`, `--level`, `--content`, `--example`). The `render_comparison.rs` drift ledger is clean (no `Section` entries), and all existing tests pass.

The implementation follows the Stage-2 migration recipe established by the earlier component flips: one projection helper, `render_via_tree` with `tracing::error!` + empty-string fallback, a retained `#[doc(hidden)] pub fn render_bespoke` parity surface, and direct cross-target trait impls.

## Findings

### 1. Incomplete bespoke-vs-tree parity coverage — **High**

The spec lists **14 critical parity variants** that must be covered before the flip is considered hardened. The current `section_parity.rs` suite covers only about half of them with a genuine bespoke-vs-tree comparison:

| Variant | Status |
|---|---|
| h1 section, no content | ✅ `empty_section_bespoke_and_tree_emit_identical_output` (exact bytes) |
| h2 section with string content | ✅ `render_via_tree_matches_render_bespoke_ansi_stripped` |
| h3 section with multiple items | ❌ No bespoke-vs-tree test; `sample_section` is h2 |
| h4 / h5 / h6 sections | ✅ `bespoke_and_tree_agree_for_every_heading_level` (width 80 only) |
| Section with Prose content | ❌ Tree-only (`prose_child_inline_emphasis_survives_terminal_render`); no bespoke comparison |
| Section with nested Component | ❌ Tree-only (`nested_block_component_renders_across_all_targets`); no bespoke comparison |
| Layout margins (left/right/top/bottom) | ⚠️ `left_margin_is_honored_through_tree_path` is tree-only; right/top/bottom margins have **no bespoke-vs-tree coverage** |
| Layout alignment | ❌ Not tested at all |
| Narrow width (40 cols) | ❌ `tree_output_renders_at_all_parity_widths` is tree-only |
| Wide width (120 cols) | ❌ Same gap |
| Width matrix (40/60/80/100/120/160/200) | ❌ No bespoke-vs-tree width matrix |

Most existing parity assertions use `split_whitespace` token equality after ANSI stripping. This is too coarse to catch the structural facet divergences the spec calls out (blank-line positions, indentation, maximum visible width, SGR styling offsets). The `render_comparison.rs` matrix *does* assert these facets, but after the IR flip both halves of the matrix route through the tree renderer, so the matrix is tautological for Section and cannot exercise the retained bespoke path.

**Impact:** The bespoke renderer is still a `pub` parity surface. Without structural facet coverage across layout scenarios, a future change to the bespoke path (or to the tree renderer's layout application) could drift silently.

**Recommendation:** Add dedicated bespoke-vs-tree parity tests for the missing variants, asserting on exact bytes where possible (empty section) and on structural facets (indent, blank lines, width, styling offsets) for layout scenarios. The width matrix should cover at least 40, 80, and 120 columns.

---

### 2. Vertical-margin CLI asymmetry — **Medium**

The spec's CLI section states that `apply_section_layout()` should map **all** `LayoutArgs` fields onto the component, including `margin_top` and `margin_bottom`, and that `emit_vertical_margins()` should *not* be called to avoid double-application.

The actual CLI uses the shared `apply_renderable_layout` helper, which only maps `margin_left`, `margin_right`, and `alignment`. It then calls `emit_vertical_margins` for the terminal branch. This means:
- **Terminal:** vertical margins are applied via `emit_vertical_margins`, not via the tree renderer. This works but is inconsistent with the spec's design.
- **HTML / Markdown:** `--margin-top` and `--margin-bottom` are **silently ignored** because `emit_vertical_margins` does not run on those branches and the shared helper never seeds them onto the node.

The doc comment on `SectionArgs.layout` documents this asymmetry, which follows the project's surgical-change discipline, but it is still a functional gap for non-terminal targets.

**Recommendation:** Either extend the shared helper to map vertical margins (and remove `emit_vertical_margins` from the terminal branch so the tree renderer handles them uniformly), or add explicit runtime warnings when `--margin-top`/`--margin-bottom` are used with `--html`/`--md`/`--md-plus`.

---

### 3. `render_html_page` is untested — **Medium**

`BrowserRenderable` requires both `render_html_fragment` and `render_html_page`. The Section impl provides the latter, but no test — unit, parity, or CLI — exercises it. A regression in `HtmlPage::from` or `apply_page_options` would not be caught by Section's test suite.

**Recommendation:** Add a one-line unit test asserting that `render_html_page(None).render()` contains the expected `<section>` fragment wrapped in `<html>`.

---

### 4. `MarkdownRenderable` error fallback diverges from spec — **Low**

The spec shows the Markdown fallback as `unwrap_or_else(|_| self.title.clone())`. The implementation returns `String::new()` and logs via `tracing::error!`. This follows the evolved project pattern (empty output + structured log) and is preferable to in-band sentinel text, but it is a divergence from the written spec.

**Recommendation:** Update the spec to match the implementation, or add a comment in the source citing the deliberate policy change.

---

### 5. Unnecessary clones in CLI `run` — **Low**

`SectionArgs::run` clones `self.title` and `self.content` because `self` is needed later for flags and layout. Since `run` takes `self` by value, the clones are avoidable by destructuring or by moving the fields out before referencing the remaining flags.

**Recommendation:** Refactor to move `title` and `content` without cloning, e.g. by extracting them into local variables before the layout/flag branches consume `self`.

---

### 6. Redundant `add_string` builder — **Low**

`Section::add_string` duplicates `Section::push`, which already accepts any `T: Into<RenderableTerminalContent>` (including `String` and `&str`). The redundant method adds API surface with no additional behavior.

**Recommendation:** Deprecate `add_string` with `#[deprecated(note = "use push")]` or remove it in a follow-up.

---

### 7. No Level-2 (real-terminal) coverage for Section — **Low**

Section has no dedicated Level-2 tests (WezTerm/Kitty/tmux capture). The tree renderer's style and layout lowering *are* covered at Level-2 by `level2_render_tree_style.rs` and `level2_layout.rs` for other components, so Section rides the same shared path. The risk of a Section-specific rendering bug in a real terminal is low because Section emits no custom escape sequences beyond the standard SGR and layout prefixes that the shared renderer handles.

**Note:** This is acceptable given the component's simplicity, but worth recording because the test-rigor snippet asks for it.

---

## Ergonomics & Performance

- **Performance:** No hot-path allocations or clones stand out. `to_render_node` allocates a `Vec` for children, which is necessary. The bespoke `render_content` path is retained only for parity and is not on the default hot path.
- **Ergonomics:** The projection helper (`to_render_node`) is cleanly factored and well-documented. The CLI's `wrapper_only_css` correctly avoids double-application of layout margins in HTML, following the pattern established by `bt list`.

---

## Production Readiness

**Judgment: Not production ready.**

The implementation is **functionally correct** and follows the established Stage-2 migration pattern. The tree projection, trait wiring, and CLI surface are all complete. However, the component fails the "strong unit and integration testing for everything" bar because the bespoke-vs-tree parity suite is incomplete relative to the spec's own critical-variant list. High-severity gaps include:

- No bespoke-vs-tree coverage for Prose content, nested components, right/top/bottom margins, alignment, or the width matrix.
- No structural-facet assertions (indent, blank lines, width, styling offsets) beyond the empty-section case.
- The `render_comparison.rs` matrix, which used to provide facet-based coverage, is now tautological for Section because both halves route through the tree renderer.

Until these parity gaps are closed, the retained bespoke path is inadequately validated, and the component does not meet the acceptance criteria written in its own specification. Once the missing parity variants are added (and `render_html_page` receives a basic smoke test), the component will be production ready.
