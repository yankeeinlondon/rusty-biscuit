---
phases: 6
created: 2026-05-23
start_phase: 1
source_files_during_phase_1:
  - darkmatter/lib/src/layout/types.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/layout/context.rs
  - darkmatter/lib/src/markdown/output/terminal.rs
  - darkmatter/lib/src/style/apply.rs
  - darkmatter/cli/src/output.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/style/apply.rs
  - darkmatter/lib/src/style/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
  - darkmatter
  - darkmatter-cli
---

# Execution Plan: Sub-Spec #4 — `ul` / `ol` / `li` Split + Wiring

> Derived from `spec-4.md`. Splits the monolithic `PageComponent::Lists` into three concrete variants (`Ul`, `Ol`, `Li`), wires `style.{ul,ol,li}.*` frontmatter through a new list-indent channel, and preserves backward compatibility for the deprecated `Lists` variant.

---

## Phase 1: Core Type Split and Backward-Compat

*Goal: Establish the split `PageComponent` variants, the independent list-indent storage, and fallback behavior so existing consumers keep compiling.*

- [ ] **Task 1.1: Split `PageComponent` enum**
  - In `darkmatter/lib/src/layout/types.rs`, add `Ul`, `Ol`, `Li` variants after `CodeBlocks`
  - Mark `Lists` with `#[deprecated(note = "use PageComponent::{Ul, Ol, Li}")]`
  - Update `PageComponent::ALL` to `[Images, BlockQuotes, Tables, CodeBlocks, Ul, Ol, Li]`
  - Add `PageComponent::LISTS: [PageComponent; 3] = [Self::Ul, Self::Ol, Self::Li]`
  - Update `page_component_all_covers_every_variant` test to assert `len() == 7`
  - Add test asserting `PageComponent::LISTS` contains exactly the three concrete list variants

- [ ] **Task 1.2: Add list-indent channel to `DarkmatterPage`**
  - Add `list_left_margins: HashMap<PageComponent, WidthUnit>` field to `DarkmatterPage`
  - Add `with_list_left_margin(self, component: PageComponent, margin: WidthUnit) -> Self` builder
    - Returns clear error / panics (documented) when `component` is not `PageComponent::Ul`
  - Add `list_left_margin_for(&self, component: PageComponent) -> Option<WidthUnit>` accessor
  - Update `is_default_layout()` to include `&& self.list_left_margins.is_empty()`

- [ ] **Task 1.3: Backward-compat fallback in `DarkmatterPage`**
  - Update `alignment_for(&self, component)` to fall back to `PageComponent::Lists` when the concrete component has no explicit entry
  - Update `fill_for(&self, component)` to fall back to `PageComponent::Lists` when the concrete component has no explicit entry
  - Add unit tests: `alignment_for_ul_falls_back_to_lists`, `fill_for_ol_falls_back_to_lists`

- [ ] **Task 1.4: Update `LayoutContext` for list left margins and fallback**
  - Add `list_left_margins: HashMap<PageComponent, WidthUnit>` field
  - Add `list_left_margin(&self, component: PageComponent) -> Option<WidthUnit>` accessor
  - Update `from_page` signature to accept `list_left_margins` and store it
  - Update `component_alignment()` to fall back to `PageComponent::Lists` for concrete list variants
  - Update `component_fill()` to fall back to `PageComponent::Lists` for concrete list variants
  - Update `LayoutContext` test constructors and existing tests to compile with the new field

- [ ] **Task 1.5: Thread list left margins through render paths**
  - In `DarkmatterPage::render`, pass `self.list_left_margins.clone()` into `LayoutContext::from_page`
  - In `DarkmatterPage::render_to_browser`, pass `self.list_left_margins.clone()` into `LayoutContext::from_page`

- [ ] **Checkpoint:** `cargo test -p darkmatter` compiles and passes for `layout::types`, `layout::page`, and `layout::context` test modules.

*Parallelizable:* Tasks 1.2–1.4 can be drafted in parallel once 1.1 establishes the new variants, but they must integrate before the checkpoint.

---

## Phase 2: Style Lowering and Application

*Goal: Implement `apply_list_style` that lowers parsed `StyleFrontmatter` list buckets onto the `DarkmatterPage` builder, including the new `ul.left-margin` channel and width/max-width exclusivity.*

- [ ] **Task 2.1: Extend `StyleApplyError` for list wiring**
  - Add `WidthMaxWidthConflict { bucket: &'static str }` variant with message: ``style.{bucket}.width and style.{bucket}.max-width are mutually exclusive``
  - Add `InvalidListLeftMarginComponent` variant with message describing that only `Ul` is accepted in this sub-spec

- [ ] **Task 2.2: Add `ListStyleOverrides` struct**
  - In `darkmatter/lib/src/style/apply.rs`, add `ListStyleOverrides` with nine `bool` fields:
    `ul_alignment`, `ul_fill`, `ul_left_margin`, `ol_alignment`, `ol_fill`, `li_alignment`, `li_fill`
  - Derive `Debug`, `Clone`, `Copy`, `Default`, `PartialEq`, `Eq`

- [ ] **Task 2.3: Implement lowering helpers**
  - Reuse / extend the `lower_length_to_fill` helper from sub-spec #3 (or implement if missing):
    - `Length::Zero` / `Length::Ch(n)` → `WidthUnit::Fixed(u16)`
    - `Length::Percent(p)` → `WidthUnit::Percent(p)`
    - `Length::Css(_)` → `StyleApplyError::InvalidCssLength`
  - Add `lower_length_to_width_unit(length: &Length) -> Result<WidthUnit, StyleApplyError>` with the same mapping but returning `WidthUnit` directly

- [ ] **Task 2.4: Implement `apply_list_style`**
  - Signature: `apply_list_style(page: DarkmatterPage, style: &StyleFrontmatter, overrides: ListStyleOverrides) -> Result<DarkmatterPage, StyleApplyError>`
  - For each bucket (`ul`, `ol`, `li`):
    - If both `width` and `max_width` are `Some`, return `WidthMaxWidthConflict`
    - If `width` is `Some`, lower to `PageFill::Explicit(...)` and apply via `with_fill(component, ...)` unless fill override is `true`
    - If `max_width` is `Some`, lower to `PageFill::Max(...)` and apply via `with_fill(component, ...)` unless fill override is `true`
    - If `alignment` is `Some`, map to `PageAlignment` and apply via `use_alignment(component, ...)` unless alignment override is `true`
  - For `ul.left_margin`:
    - Lower via `lower_length_to_width_unit`
    - Apply via `with_list_left_margin(PageComponent::Ul, ...)` unless `ul_left_margin` override is `true`

- [ ] **Task 2.5: Add unit tests for `apply_list_style`**
  - `ul_left_margin_applied`: `style.ul.left_margin = Some(Length::Ch(4))` → `page.list_left_margin_for(Ul) == Some(WidthUnit::Fixed(4))`
  - `ol_alignment_applied`: `style.ol.alignment = Some(Alignment::Right)` → `page.alignment_for(Ol) == PageAlignment::Right`
  - `li_width_applied`: `style.li.width = Some(Length::Ch(30))` → `page.fill_for(Li) == PageFill::Explicit(WidthUnit::Fixed(30))`
  - `ul_width_max_width_conflict`: both set → `WidthMaxWidthConflict { bucket: "ul" }`
  - `ol_width_max_width_conflict`: both set → `WidthMaxWidthConflict { bucket: "ol" }`
  - `li_width_max_width_conflict`: both set → `WidthMaxWidthConflict { bucket: "li" }`
  - `ul_left_margin_css_rejected`: `Length::Css(...)` → `InvalidCssLength`
  - `overrides_suppress_frontmatter`: each override `true` skips its field

- [ ] **Checkpoint:** `cargo test -p darkmatter` passes for `style::apply` tests.

*Parallelizable:* Task 2.1 and 2.2 are independent and can be done before 2.3/2.4.

---

## Phase 3: CLI Granularity and Broadcast

*Goal: Add granular list CLI flags and make broadcast flags (`--align-lists`, `--fill-lists`) write all three concrete variants.*

- [ ] **Task 3.1: Add granular CLI arguments**
  - In `darkmatter/cli/src/args.rs`, add:
    - `--align-ul <ALIGN>`, `--align-ol <ALIGN>`, `--align-li <ALIGN>`
    - `--fill-ul <FILL>`, `--fill-ol <FILL>`, `--fill-li <FILL>`
  - Place them in the same CLI argument group as the existing alignment/fill flags

- [ ] **Task 3.2: Update `apply_cli_layout_flags` for broadcast behavior**
  - Change `--align-lists` to call `use_alignment` for `Ul`, `Ol`, and `Li` (instead of `Lists`)
  - Change `--fill-lists` to call `with_fill` for `Ul`, `Ol`, and `Li` (instead of `Lists`)
  - Add handlers for the six new granular flags
  - Precedence rule: granular flag overrides the broadcast `--align-lists` / `--fill-lists` for its component
  - Global `--alignment` / `--fill` still claim all components (including the three list variants via `PageComponent::ALL`)

- [ ] **Task 3.3: Update existing CLI precedence tests**
  - In `darkmatter/cli/tests/cli.rs`, update `layout_resolved_fill_global_then_component_specific`:
    - `fill_for(PageComponent::Lists)` assertions become `fill_for(Ul)`, `fill_for(Ol)`, `fill_for(Li)`
  - Update `layout_resolved_alignment_global_then_component_specific` similarly
  - Add `#[allow(deprecated)]` on the old `Lists` fallback test or add a new dedicated backward-compat test in Phase 5

- [ ] **Task 3.4: Add granular CLI precedence tests**
  - `--align-lists right --align-ul left` → `Ul` is `Left`, `Ol` and `Li` are `Right`
  - `--fill-lists max=40 --fill-ol max=30` → `Ol` is `Max(30)`, `Ul` and `Li` are `Max(40)`

- [ ] **Checkpoint:** `cargo test -p darkmatter-cli` passes for layout precedence tests.

*Parallelizable:* Task 3.1 can be done before 3.2 (the binary won't compile until both are done, but the arg definitions are independent of the apply logic).

---

## Phase 4: Renderer Wiring (Terminal + Browser)

*Goal: Make the terminal renderer choose the correct list component and apply `ul.left-margin` stacking; make the HTML renderer emit split selectors.*

- [ ] **Task 4.1: Terminal renderer — split list component selection**
  - In `darkmatter/lib/src/markdown/output/terminal.rs`, around the `Tag::List(start_num)` handler:
    - `Tag::List(None)` → `PageComponent::Ul`
    - `Tag::List(Some(_))` → `PageComponent::Ol`
  - Replace the single `PageComponent::Lists` usage with the conditional selection above

- [ ] **Task 4.2: Terminal renderer — `ul.left-margin` stacking**
  - On top-level `Tag::List(None)` when `layout_ctx` is present:
    - Resolve `list_left_margin = ctx.list_left_margin(PageComponent::Ul)`
    - Compute `body_width = ctx.resolve_component_width(PageComponent::Ul)` capped by remaining space after left-margin
    - Push `body_width` onto the wrapper
    - Set `wrapper.alignment_offset = left_margin_cells + alignment_padding_for_body`
    - Add test: fixture `style.ul.left-margin: 4ch` + `style.ul.max-width: 40` renders unordered list items with a 4-cell left offset and body wrapping at ≤40 cells

- [ ] **Task 4.3: Terminal renderer — `li` body overrides**
  - On `Tag::Item` start:
    - After marker emission, if `layout_ctx` is present and `Li` has explicit alignment or fill:
      - Push `Li` component width
      - Adjust `alignment_offset` for `Li` alignment
  - On `TagEnd::Item`:
    - Pop any `Li`-scoped width/alignment overrides before newline
  - Add test: `style.li.alignment: right` aligns item body while preserving marker prefix placement

- [ ] **Task 4.4: HTML renderer — split `component_selectors`**
  - In `darkmatter/lib/src/layout/page.rs`, update `component_selectors`:
    - `PageComponent::Ul` → `"ul"`
    - `PageComponent::Ol` → `"ol"`
    - `PageComponent::Li` → `"li"`
    - `PageComponent::Lists` → `"ul, ol"` (preserved for backward compat)

- [ ] **Task 4.5: HTML renderer — CSS generation order and left-margin**
  - In `build_component_css`:
    - Emit deprecated `Lists` rules first (if `Lists` has explicit alignment/fill)
    - Then iterate `PageComponent::ALL` as before
    - For `PageComponent::Ul`, if `ctx.list_left_margin(Ul)` is `Some`, emit `margin-left: {N}ch;`
    - Ensure concrete `Ul`/`Ol`/`Li` rules override the deprecated `Lists` rule via normal cascade (more specific selector, emitted later)
  - Add test verifying generated CSS contains `ul {`, `ol {`, `li {` separately, and `ul, ol {` only when deprecated `Lists` is set

- [ ] **Task 4.6: Add render integration tests**
  - `render_ul_left_margin`: fixture's `style.ul.left-margin: 4ch` → item bodies start 4 cells further right
  - `render_ul_max_width`: fixture's `style.ul.max-width: 40` → bullet body wraps at 40 cells
  - `render_ul_left_margin_and_max_width`: both apply simultaneously
  - `render_ol_alignment_right`: ordered list contents right-aligned
  - `render_li_body_alignment`: `style.li.alignment: right` preserves marker prefix

- [ ] **Checkpoint:** `cargo test -p darkmatter` passes for terminal render and HTML wrapper tests.

*Parallelizable:* Tasks 4.1–4.3 (terminal) and 4.4–4.5 (browser) are independent and can be developed in parallel.

---

## Phase 5: Warning Suppression and Comprehensive Tests

*Goal: Advance the active wiring constant, suppress `KnownButInactive { sub_spec: 4 }` for wired list keys, and validate every acceptance criterion.*

- [ ] **Task 5.1: Advance `ACTIVE_STYLE_WIRING_SUB_SPEC`**
  - In `darkmatter/lib/src/style/parse.rs`, change `ACTIVE_STYLE_WIRING_SUB_SPEC` from `2` to `4`
  - If sub-spec #3 has not yet landed in the branch being worked on, advance to `3` first (or land #3 before #4), ensuring `table`, `block-quote`, and `images` keys are wired before the constant reaches `4`

- [ ] **Task 5.2: Update parser tests for suppressed warnings**
  - In `parse.rs` tests:
    - Update `test_doc_all_known_but_inactive`: the 6 inactive warnings become 0 (all list keys are now wired)
    - Update `matches_test_doc_acceptance_criteria`: no behavioral change expected, but verify no `KnownButInactive` warnings
  - In `coverage_tests.rs`: update expectations for `ul.*`, `ol.*`, `li.*` canonical and alias paths to expect 0 `KnownButInactive` warnings

- [ ] **Task 5.3: Add spec acceptance tests**
  - `ul_left_margin_render`: fixture's `style.ul.left-margin: 4ch` → 4-cell indent observable in rendered output
  - `ul_max_width_render`: fixture's `style.ul.max-width: 40` → body wraps at 40 cells
  - `ul_left_margin_plus_max_width`: both coexist, neither overwrites the other
  - `ol_alignment_render`: `style.ol.alignment: right` → ordered list right-aligned
  - `li_body_alignment`: `style.li.alignment: right` aligns item body, marker stays put
  - `li_independent_of_ul_ol`: `style.li.color` (when #5 lands) applies regardless of list type; for now test that `li.alignment` is independent
  - `align_lists_broadcast`: `--align-lists right` applies to `Ul`, `Ol`, and `Li`
  - `align_ul_granular`: `--align-ul right` overrides only `Ul`
  - `deprecated_lists_fallback`: `page.use_alignment(PageComponent::Lists, Right)` still affects both `Ul` and `Ol` when no concrete override exists
  - `width_max_width_exclusivity_ul`, `_ol`, `_li`: each returns `StyleApplyError`
  - `browser_selectors_split`: generated CSS uses separate `ul`, `ol`, `li` selectors
  - `active_wiring_warnings`: `ul.width`, `ul.max-width`, `ul.alignment`, `ul.left-margin`, `ol.width`, `ol.max-width`, `ol.alignment`, `li.width`, `li.max-width`, and `li.alignment` produce no `KnownButInactive`; list `color`/`bg-color` keys still produce `KnownButInactive { sub_spec: 5 }`

- [ ] **Task 5.4: Add backward-compat compile test**
  - A library test that uses `#[allow(deprecated)]` and calls `page.use_alignment(PageComponent::Lists, Right)` followed by `page.alignment_for(PageComponent::Ul)` asserting `Right`

- [ ] **Checkpoint:** All new tests pass; `cargo test -p darkmatter` and `cargo test -p darkmatter-cli` are green.

*Parallelizable:* Task 5.2 (parser test updates) and 5.3 (acceptance tests) can be written in parallel once 5.1 advances the constant.

---

## Phase 6: Documentation and Final Validation

*Goal: Update user-facing docs and run the fixture through both terminal and HTML output to confirm acceptance.*

- [ ] **Task 6.1: Update `darkmatter/docs/rendering/style.md`**
  - Change list key examples to canonical kebab-case (`left-margin`, `max-width`, etc.)
  - Document `ul.left-margin` stacking behavior: it resolves first, then `width`/`max-width` applies to the remaining body width
  - Document that `--align-lists` is a broadcast flag applying to all three list types, with granular `--align-ul` / `--align-ol` / `--align-li` available
  - Add a note that `PageComponent::Lists` is deprecated in favor of `Ul` / `Ol` / `Li`

- [ ] **Task 6.2: Validate the fixture end-to-end**
  - Run `md darkmatter/example-docs/rendering/style-prop.md` and visually inspect:
    - Unordered list items have a 4ch left indent
    - Unordered list body wraps at 40 cells
    - Ordered list is right-aligned
  - Run `md --output html darkmatter/example-docs/rendering/style-prop.md` and inspect generated CSS:
    - Contains separate `ul { ... }`, `ol { ... }`, `li { ... }` rules
    - Does not rely solely on `ul, ol { ... }` for the concrete variants

- [ ] **Task 6.3: Full workspace test run**
  - Run `cargo test` across the darkmatter workspace member (`darkmatter/lib`, `darkmatter/cli`)
  - Run `cargo clippy` and resolve any new deprecation warnings in workspace code
  - Run `cargo doc` and verify no broken intra-doc links

- [ ] **Task 6.4: Check downstream consumers**
  - Search the workspace for exhaustive `match` on `PageComponent` and add the three new variants (or `#[allow(deprecated)]` if matching `Lists`)
  - Files known to reference `PageComponent::Lists`:
    - `darkmatter/cli/src/output.rs`
    - `darkmatter/cli/tests/cli.rs`
    - `darkmatter/lib/src/layout/page.rs`
    - `darkmatter/lib/src/markdown/output/terminal.rs`
    - `darkmatter/lib/src/layout/types.rs`

- [ ] **Final Acceptance Checkpoint:**
  - Fixture renders with `ul` left-margin (`4ch`), `ul` max-width (`40`), and `ol` right-alignment.
  - Existing `PageComponent::Lists` code compiles with deprecation warning and works as fallback.
  - `PageComponent::ALL` contains only concrete variants.
  - `style.ul.left-margin` coexists with `style.ul.width` / `style.ul.max-width`.
  - `width` + `max-width` conflict detection exists for `ul`, `ol`, `li`.
  - All previous sub-spec tests still pass.
  - `KnownButInactive { sub_spec: 4 }` is suppressed for wired keys.
  - HTML output uses split list selectors.
  - Documentation is updated.

*Parallelizable:* Task 6.1 (docs) can be drafted in parallel with earlier phases, but should be finalized after the behavior is locked.
