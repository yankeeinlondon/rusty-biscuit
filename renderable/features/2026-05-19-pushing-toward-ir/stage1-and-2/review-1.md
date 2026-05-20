# Aggregate Review — IR Component Migrations (Iteration 1)

> Generated from all `{component}-review-1.md` files in
> `renderable/features/2026-05-19-pushing-toward-ir/`.
> Each section preserves the severity and recommended fix from its source review.

---

## Production Readiness Summary

| Component     | Ready |
|---------------|-------|
| BlockQuote    | ❌    |
| Compose       | ❌    |
| FileSystem    | ❌    |
| OrderedList   | ✅    |
| Progress      | ❌    |
| Section       | ❌    |
| StatusBlock   | ✅    |
| Table         | ❌    |
| TextBlock     | ❌    |
| Todo          | ❌    |
| TwoColumn     | ❌    |
| UnorderedList | ✅    |

**3 of 12 components are production-ready.**

---

## BlockQuote — Suggested Fixes

### 1. Update `renderable/docs/components.md` table [`HIGH`]
- **Fix:** Change BlockQuote row from `Browser ❌, Markdown ❌, IR State = both avail, old renders, bt CLI = bespoke` to `Browser ✅, Markdown ✅, IR State = both avail, tree renders, bt CLI = tree`.

### 2. Honor `NO_COLOR` in the tree renderer path [`HIGH`]
- **Fix:** Either teach `Terminal::new()` / `color_depth()` to downgrade to `ColorDepth::None` when `NO_COLOR` is set, or teach the tree renderer's `apply_style` to skip SGR emission when a `no_color` flag is present on the terminal options. Add an integration test that asserts `NO_COLOR=1 bt quote "text"` emits zero SGR escapes.

### 3. Markdown error handling — log via `tracing::error!` [`MEDIUM`]
- **Fix:** Replace `.unwrap_or_default()` in `render_markdown()` / `render_markdown_plus()` with an explicit `match` that logs via `tracing::error!` (including component name, target dialect, and error) before returning an empty string.

### 4. `test_render_markdown_empty_quote` assertion [`MEDIUM`]
- **Fix:** Assert the output starts with `>` or is exactly `> ` (or minimal output), rather than calling `render_markdown()` with no assertion.

### 5. Missing CLI integration tests for `--example --md` and `--example --html` [`LOW`]
- **Fix:** Add two integration tests that run `bt quote --example --md` and `bt quote --example --html` and assert the respective example command string appears in stdout.

### 6. Unit test for `render_tree()` layout seeding [`LOW`]
- **Fix:** Add a test that builds a quote with a non-default margin or alignment, calls `render_tree()`, and asserts the layout hint is present in serialized `NodeAttrs`.

### 7. `BrowserTreeComponent::fallback_fragment` in-band diagnostic [`LOW`]
- **Fix:** Change `fallback_fragment` to return an empty fragment and emit `tracing::error!` with the error details. This is an adapter-level change.

---

## Compose — Suggested Fixes

### 1. Missing dedicated parity-test file [`HIGH`]
- **Fix:** Create `biscuit-terminal/lib/tests/compose_parity.rs` following the pattern in `section_parity.rs`. It should include:
  1. Structural snapshots (`render_tree_node_produces_root_with_sequence_join_none`, nested Compose hoisting).
  2. Validation gate (`projected_tree_validates_with_no_errors`).
  3. Semantic parity between a retained `render_bespoke` and the tree path.
  4. Width matrix (`PARITY_WIDTHS`).
  5. Markdown / Browser / MarkdownPlus cross-target assertions.
  6. A `render_bespoke` compatibility method on `Compose` so the parity gate has a historical baseline.

### 2. No `KNOWN_DRIFT` ledger [`HIGH`]
- **Fix:** Add a `KNOWN_DRIFT` block either in a new `compose_parity.rs` file or as a doc comment at the top of `compose.rs`. Each entry should name the divergence, explain why it is accepted, and reference the test that gates it.

### 3. Inconsistent cross-target error-handling policy [`MEDIUM`]
- **Fix:** Unify all three paths to `tracing::error!(component = "Compose", target = "...", error = %error)` plus an empty fallback. For `BrowserFragment`, use `BrowserFragment::new().finalize()`. For Markdown, replace `unwrap_or_default()` with an explicit `match` that logs.

### 4. Stale end-user documentation [`MEDIUM`]
- **Fix:** Update `biscuit-terminal/docs/components/compose.md` to document the `bt compose` CLI, its flags, and its part-ordering semantics. Mirror the style of `docs/components/prose.md` or `docs/components/section.md`.

### 5. No Level 2 (real-terminal) verification [`MEDIUM`]
- **Fix:** Add `cli/tests/level2_compose.rs` following the pattern of `level2_render_tree_style.rs` or `level2_prose_styling.rs`. Minimum coverage:
  - `bt compose --prose "<b>bold</b>"` captured via `wezterm cli get-text` or `tmux capture-pane`, asserting glyph presence.
  - `bt compose` with layout flags (`--margin-left`) asserting real-terminal indent width.

### 6. `render_bespoke` not preserved [`LOW`]
- **Fix:** Add a `#[doc(hidden)] pub fn render_bespoke(&self, term: &Terminal) -> String` that performs the old concatenation loop.

---

## FileSystem — Suggested Fixes

### 1. Terminal parity gate is absent [`CRITICAL`]
- **Fix:** Add `biscuit-terminal/lib/tests/filesystem_parity.rs` following the established pattern (`render_bespoke()` vs `TreeComponent::new(fs).render(&term)`). Exercise the spec's 21 variants, with the tree path using `Terminal::new_optimistic(80)` for determinism. Document accepted divergences as `KNOWN_DRIFT` in the test file.

### 2. `TerminalRenderable::render_tree_node` is not implemented [`HIGH`]
- **Fix:** Add `render_tree_node()` that delegates to the same private `fs_render_tree_inner()` helper used by `TreeRenderable::render_tree`.

### 3. Gitignore integration is a stub [`HIGH`]
- **Fix:** Integrate the `ignore` crate (or existing `biscuit-file` / `walkdir` ignore logic) into `build_tree_recursive` so `.gitignore` rules are evaluated per-entry. `is_ignored` should be populated from real filesystem data.

### 4. Permission errors silently swallow directories [`MEDIUM`]
- **Fix:** In the `Err(_) =>` arm of `build_tree_recursive`, create an error-marked `TreeNode::Dir` with `has_error: true` and `children: vec![]` instead of returning an empty vector.

### 5. No `file_links` CLI flag [`MEDIUM`]
- **Fix:** Add `--links` to `DirArgs` and thread it through `render_dir` / `render_dir_alt_target`.

### 6. Root path canonicalization is duplicated [`LOW`]
- **Fix:** Canonicalize once in `ensure_tree_built` (or lazily on first access) and store the canonical base path on `FileSystem` as a private field. Fall back to the raw `root_path` when canonicalization fails.

### 7. `render_bespoke` escape hatch is missing [`LOW`]
- **Fix:** Extract the current `render_nodes` body into `#[doc(hidden)] pub fn render_bespoke`, and have `TerminalRenderable::render` delegate to it.

### 8. `fs_render_tree_inner` empty-Root warning [`LOW`]
- **Fix:** Change `debug!` to `tracing::warn!` when `self.tree.is_none()` so a direct `TreeRenderable::render_tree` caller who forgets `ensure_tree_built()` gets a more visible signal.

---

## OrderedList — Suggested Fixes

### 1. `layout_matrix` omits OrderedList [`MEDIUM`]
- **Fix:** Add an `OrderedList` case to `component_cases()` in `layout_matrix_support.rs`, using a multi-item list that exercises at least one double-digit prefix. Regenerate snapshots with `INSTA_UPDATE=always`.

### 2. `render_markdown` / `render_markdown_plus` silently swallow render errors [`MEDIUM`]
- **Fix:** Replace `unwrap_or_default()` with an explicit `match` that logs the error through `tracing::error!` (component = "OrderedList", dialect = "markdown" / "markdown_plus") before returning `String::new()`.

### 3. No explicit `NO_COLOR` integration test for `bt list --ordered` [`LOW`]
- **Fix:** Add a CLI integration test that spawns `bt list --ordered` with `NO_COLOR=1` and asserts the absence of `\x1b[` bytes.

---

## Progress — Suggested Fixes

### 1. Terminal layout parity is incomplete [`HIGH`]
- **Fix:** Add dedicated parity tests in `progress_parity.rs` that construct a Progress with right margin, top margin, bottom margin, center alignment, and right alignment, call both `render_bespoke` and `render` (the tree path), and assert ANSI-stripped equality.

### 2. `NO_COLOR=1` is not honored by `bt progress` [`MEDIUM`]
- **Fix:** Either teach `detect_terminal_honoring_force_color()` to downgrade `color_depth` to `ColorDepth::None` when `NO_COLOR` is set (preferred), or add a post-render strip step in `bt progress`. Add an integration test asserting `NO_COLOR=1 bt progress 50 --fill-color green` emits no `\x1b` bytes.

### 3. Missing MarkdownPlus layout-no-op test [`MEDIUM`]
- **Fix:** Add a parity test that builds a Progress with `left_margin(4ch)` and `alignment(Center)`, renders `render_markdown_plus()`, and asserts the output contains no `margin-left`, `margin-right`, or `text-align` declarations.

### 4. No explicit `BrowserTreeComponent<Progress>` test [`MEDIUM`]
- **Fix:** Add a test in `progress_parity.rs` that wraps a styled Progress in `BrowserTreeComponent`, renders `render_html_fragment()`, and asserts the HTML contains `role="progressbar"` and the correct `aria-valuenow`.

### 5. Small terminal widths are not stress-tested [`MEDIUM`]
- **Fix:** Add parity tests for widths 10 and 20 that assert both `render_bespoke` and `render` produce deterministic output (no panic) and document any accepted divergence.

### 6. `BrowserTreeComponent` fallback emits in-band sentinel text [`LOW`]
- **Fix:** Change `fallback_fragment` to return an empty fragment and emit `tracing::error!`. This is an adapter change, not a Progress change.

---

## Section — Suggested Fixes

### 1. Incomplete bespoke-vs-tree parity coverage [`HIGH`]
- **Fix:** Add dedicated bespoke-vs-tree parity tests for the missing variants:
  - h3 section with multiple items
  - Section with Prose content (bespoke-vs-tree, not tree-only)
  - Section with nested Component (bespoke-vs-tree, not tree-only)
  - Right/top/bottom margins
  - Alignment (center, right)
  - Narrow width (40 cols), Wide width (120 cols)
  - Width matrix (40/60/80/100/120/160/200)
  Assert on exact bytes where possible and structural facets (indent, blank lines, width, styling offsets) for layout scenarios.

### 2. Vertical-margin CLI asymmetry [`MEDIUM`]
- **Fix:** Either extend the shared `apply_renderable_layout` helper to map vertical margins (and remove `emit_vertical_margins` from the terminal branch), or add explicit runtime warnings when `--margin-top`/`--margin-bottom` are used with `--html`/`--md`/`--md-plus`.

### 3. `render_html_page` is untested [`MEDIUM`]
- **Fix:** Add a one-line unit test asserting that `render_html_page(None).render()` contains the expected `<section>` fragment wrapped in `<html>`.

### 4. `MarkdownRenderable` error fallback diverges from spec [`LOW`]
- **Fix:** Update the spec to match the implementation (empty output + structured log), or add a comment in the source citing the deliberate policy change.

### 5. Unnecessary clones in CLI `run` [`LOW`]
- **Fix:** Refactor `SectionArgs::run` to move `title` and `content` without cloning, e.g. by destructuring or moving fields before layout/flag branches consume `self`.

### 6. Redundant `add_string` builder [`LOW`]
- **Fix:** Deprecate `add_string` with `#[deprecated(note = "use push")]` or remove it in a follow-up.

### 7. No Level-2 (real-terminal) coverage [`LOW`]
- **Fix:** (Optional) Add a minimal Level-2 test for `bt section` if the team wants full coverage parity with other components.

---

## StatusBlock — Suggested Fixes

### 1. Missing `render_tree_node` compatibility hook [`LOW`]
- **Fix:** Add a `render_tree_node` override that returns `Some(self.to_render_node())`. This is a one-line change.

### 2. No dedicated Level-2 real-terminal test [`MEDIUM`]
- **Fix:** Add a `bt status-block` Level-2 case to `level2_render_tree_style.rs` (or a dedicated `level2_status_block.rs`) asserting the thick left border glyph `┃` is visible, header text and icon are visible, color SGR survives on color-capable terminals, and no raw escape garbage leaks into tmux plain capture.

### 3. `renderable/docs/components.md` table is stale [`HIGH — documentation`]
- **Fix:** Update the StatusBlock row in `renderable/docs/components.md` to show Browser ✅, Markdown ✅, Tree ✅, IR State = `tree default + bespoke compatibility fallback`, bt CLI = `tree`.

### 4. Unit test does not assert border color value in projected tree [`LOW`]
- **Fix:** Add a unit test that asserts the serialized `NodeAttrs` JSON contains the expected color value when a custom `border_color` is set.

### 5. `render_html_page` not tested with actual `PageOptions` [`LOW`]
- **Fix:** Add a test that passes `Some(PageOptions::default())` and asserts the returned HTML contains expected page wrappers.

---

## Table — Suggested Fixes

### 1. `uniform_alignment` is silently ignored by the tree renderer [`HIGH`]
- **Fix:** `emit_table` needs to pre-compute `max_content_widths` the same way the bespoke renderer does (`Table::max_content_widths_for_plan`) and thread them into `pad_cell` when `uniform_alignment` is enabled. Add a parity test asserting uniform alignment survives the tree path (spec variant #19).

### 2. Missing dedicated parity tests for several spec variants [`MEDIUM`]
- **Fix:** Add parity tests for variants #8, #10, #11, #14, #19, and #20 to `table_parity.rs`. Each should compare `strip_ansi(table.render(&term))` against `strip_ansi(table.render_bespoke(&term))` on structural invariants.

### 3. Error fallback paths are untested [`MEDIUM`]
- **Fix:** Add a unit test that feeds a structurally-invalid `RenderNode` through `render_terminal_node` (or directly through `Table::render_via_tree` after monkey-patching a bad node) and asserts the result is empty and does not panic. Do the same for Markdown and Browser fallbacks.

### 4. CLI `--example` does not append target flags [`LOW`]
- **Fix:** Build the example command string dynamically, appending `--html`, `--md`, or `--md-plus` when the corresponding flag is set.

---

## TextBlock — Suggested Fixes

### 1. Missing layout-matrix coverage [`HIGH`]
- **Fix:** Add a `ComponentCase` for `TextBlock` to `layout_matrix_support/mod.rs`, generate snapshots, and verify no drift entries are needed.

### 2. Missing legacy parity test: center alignment [`MEDIUM`]
- **Fix:** Add `layout_center_alignment_applied_through_both_paths` to `text_block_parity.rs`.

### 3. Incomplete browser underline-variant coverage [`MEDIUM`]
- **Fix:** Add three tests:
  - `browser_double_underline_lowers_to_text_decoration_double`
  - `browser_dotted_underline_lowers_to_text_decoration_dotted`
  - `browser_dashed_underline_lowers_to_text_decoration_dashed`

### 4. Missing markdown coverage for dim, blink, and HTML-sensitive content [`LOW–MEDIUM`]
- **Fix:** Extend `markdown_renders_plain_text_regardless_of_style` to set dim and blink, and add `markdown_html_sensitive_content_is_escaped`.

### 5. No Level-2 (real-terminal) verification [`MEDIUM`]
- **Fix:** Add a `level2_text_block_style.rs` that drives `bt text-block` through WezTerm/Kitty/tmux and verifies bold, fg color, and underline SGR sequences appear in captured pane text.

### 6. Dead code in bespoke renderer [`LOW`]
- **Fix:** Remove `let _underline = term.underline_support;` from `TextBlock::to_terminal()`.

### 7. `render_bespoke` visibility diverges from spec [`LOW`]
- **Fix:** Either make the method `pub(crate)` or update the spec to match the `#[doc(hidden)] pub` convention.

---

## Todo — Suggested Fixes

### 1. Systematic parity gate is missing [`CRITICAL`]
- **Fix:** Add `Todo` scenarios to `layout_matrix_support.rs` (or the equivalent component list used by `render_comparison.rs`), regenerate snapshots, and populate any resulting `KNOWN_DRIFT` entries.

### 2. Spec parity variants are incomplete [`HIGH`]
- **Fix:** Expand `todo_parity.rs` with missing variants:
  - `use_prose = true` (Prose description tree and parity tests)
  - Nerd Font terminal for InProgress, Blocked, Cancelled
  - No-color terminal for all five states (systematic bespoke-vs-tree)
  - TrueColor terminal for all five states
  - Right margin applied
  - Center alignment
  - Empty description (bespoke-vs-tree)
  - Description with special characters (bespoke-vs-tree)

### 3. CLI integration test omits `todo` [`HIGH`]
- **Fix:** Add `"todo"` to the subcommand array in `test_every_subcommand_help_exposes_example_flag`.

### 4. No Level-2 (real-terminal) tests [`MEDIUM`]
- **Fix:** Add a Level-2 test that runs `bt todo --example` (or explicit state variants) and captures pane text, asserting marker presence.

### 5. `KNOWN_DRIFT` is fragmented and not in the central ledger [`MEDIUM`]
- **Fix:** Once Todo is added to `render_comparison.rs`, record the Cancelled NoColor strikethrough drift in the central ledger. Add a dedicated `KNOWN_DRIFT` comment block in `todo_parity.rs`.

### 6. Bespoke parity assertion is too weak [`MEDIUM`]
- **Fix:** Strengthen `bespoke_and_tree_share_description_in_no_color_terminal` to compare stripped output directly, or at least assert marker presence per state.

### 7. `use_prose` description flattens inline styling [`LOW`]
- **Fix:** Add a tree-render test with `Todo::from_prose("<b>bold</b> task")` asserting the description text is present after ANSI stripping.

---

## TwoColumn — Suggested Fixes

### 1. Missing byte-level Prose SGR guard test [`HIGH`]
- **Fix:** Add to `two_column_parity.rs`:
  ```rust
  #[test]
  fn prose_bold_inline_styling_survives_terminal_tree_render() {
      let left = Prose::new("**bold** left");
      let right = Prose::new("plain right");
      let cols = TwoColumn::new(left, right);
      let term = test_terminal(80);
      let out = cols.render(&term);
      assert!(out.contains("\x1b[1m"), "bold SGR must survive tree path: {out:?}");
      assert!(out.contains("\x1b[22m"), "bold reset must survive tree path: {out:?}");
  }
  ```

### 2. No `render_optimistic` vs `render_bespoke_optimistic` parity test [`MEDIUM`]
- **Fix:** Add a parity test comparing `render_bespoke_optimistic()` against `render_optimistic()`.

### 3. Missing component-level MarkdownPlus emphasis test [`MEDIUM`]
- **Fix:** Add a component-level assertion that `TwoColumn::render_markdown_plus()` preserves `**bold**` or `_italic_` inside columns.

### 4. Stale doc comment in Level-2 test file [`LOW`]
- **Fix:** Update `cli/tests/level2_layout.rs` module doc comment to state that `bt columns` now routes through the canonical tree renderer.

### 5. `render_html_page` is implemented but has no direct test [`LOW`]
- **Fix:** Add a minimal test asserting the returned `HtmlPage` contains the fragment content.

### 6. `render_via_tree_optimistic` error fallback is untested [`LOW`]
- **Fix:** Add a test forcing a tree-render failure on the optimistic path and asserting fallback to `render_bespoke_optimistic`.

---

## UnorderedList — Suggested Fixes

### 1. Browser error fallback emits in-band sentinel [`HIGH`]
- **Fix:** Change `BrowserRenderable::render_html_fragment` to log via `tracing::error!` and return an empty fragment instead of emitting `[render-tree error: …]` text.

### 2. Markdown error fallback silently drops errors [`HIGH`]
- **Fix:** Replace `.unwrap_or_default()` in `render_markdown` / `render_markdown_plus` with an explicit `match` that logs via `tracing::error!` before returning `String::new()`.

### 3. Missing edge-case unit tests for spec variants [`MEDIUM`]
- **Fix:** Add unit tests for:
  - Empty nested UnorderedList
  - Browser: long text item truncation
  - Browser: invalid projected tree fixture fallback
  - Markdown: item containing explicit newline (continuation-line indentation)
  - Markdown: nested UnorderedList / OrderedList structural output
  - Markdown: invalid list structure under `RenderStrictness::Strict`

### 4. MarkdownPlus equality test only exercises plain items [`MEDIUM`]
- **Fix:** Add a test with a styled Prose item (`<b>`, `<red>`) asserting MarkdownPlus preserves the same output as portable Markdown.

### 5. CLI shared-helper doc comments refer exclusively to `<ol>` [`LOW`]
- **Fix:** Update doc comments in `render_html_with_layout` and `wrapper_only_css` to mention both `<ol>` and `<ul>`.

### 6. `render_html_page` is not directly unit-tested [`LOW`]
- **Fix:** (Optional) Add a thin wrapper test for `render_html_page`.

---

## Cross-Cutting Fixes (mentioned in multiple reviews)

1. **BrowserTreeComponent / Browser adapter fallback policy**
   - Several reviews (BlockQuote, Progress, UnorderedList) flag that `BrowserTreeComponent::fallback_fragment` returns in-band `[render-tree error: …]` text. The established policy is `tracing::error!` + empty fragment. Fix once at the adapter level.

2. **Markdown error handling — silent `.unwrap_or_default()`**
   - Several reviews (BlockQuote, OrderedList, UnorderedList, Compose) flag that Markdown paths silently swallow errors. Replace all occurrences with `match + tracing::error! + String::new()`.

3. **`renderable/docs/components.md` table drift**
   - BlockQuote and StatusBlock reviews explicitly call out stale table rows. Audit and update the full table for all 12 flipped components.

4. **`NO_COLOR` not honored by tree renderer**
   - BlockQuote and Progress reviews flag this. A cross-component fix in `detect_terminal_honoring_force_color()` or `Terminal::color_depth()` would resolve it for all commands at once.

5. **Level-2 real-terminal coverage gaps**
   - Compose, TextBlock, Todo, and StatusBlock lack dedicated Level-2 tests. Where the component introduces new SGR, border, or layout behavior, add a minimal Level-2 smoke test.

6. **`render_html_page` untested across multiple components**
   - Section, TwoColumn, and UnorderedList all have thin `render_html_page` wrappers with no direct test. A single shared test helper or per-component one-liner would close the gap.
