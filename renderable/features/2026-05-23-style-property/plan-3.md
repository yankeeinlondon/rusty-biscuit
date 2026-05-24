---
phases: 6
created: 2026-05-23
start_phase: 1
---

# Execution Plan: Sub-Spec #3 — Existing-Component Wiring

> Derived from `spec-3.md`. Wires `style.table.*`, `style.images.*`, and `style.block-quote.*` to their existing `PageComponent` variants via `DarkmatterPage` builders.

---

## Phase 1: Types and Lowering Helpers

*Goal: Establish the types and conversion utilities that subsequent phases depend on.*

- [ ] Add `ComponentStyleOverrides` struct to `darkmatter::style` with six boolean fields (`tables_alignment`, `tables_fill`, `images_alignment`, `images_fill`, `block_quotes_alignment`, `block_quotes_fill`)
- [ ] Derive `Debug`, `Clone`, `Copy`, `Default`, `PartialEq`, `Eq` for `ComponentStyleOverrides`
- [ ] Add `StyleApplyError` variant for width/max-width exclusivity conflict (e.g., ``style.{bucket}.width and style.{bucket}.max-width are mutually exclusive``)
- [ ] Add `StyleApplyError` variant for invalid `Length::Css(_)` in component fill context (e.g., ``style.{bucket}.{field} uses CSS length which is not supported for component fill``)
- [ ] Implement internal `lower_length_to_fill(length: &Length) -> Result<PageFill, StyleApplyError>` helper in `darkmatter::style`
  - `Length::Zero` and `Length::Ch(n)` → `WidthUnit::Fixed(u16)` via saturating cast
  - `Length::Percent(p)` → `WidthUnit::Percent(p)`
  - `Length::Css(_)` → return the new `StyleApplyError` variant
- [ ] Write unit tests for `lower_length_to_fill` covering all four `Length` variants and edge cases (overflow cast, zero)
- [ ] **Checkpoint:** `cargo test` in `darkmatter/` passes for new unit tests

*Parallelizable:* Yes — the error-variant additions and the lowering helper can be developed in parallel as long as the `StyleApplyError` enum location is agreed upon.

---

## Phase 2: Core Style Application (`apply_component_style`)

*Goal: Implement the function that applies parsed component style to a `DarkmatterPage` builder.*

- [ ] Implement `apply_component_style(page, style, overrides) -> Result<DarkmatterPage, StyleApplyError>` in `darkmatter::style`
- [ ] Implement internal helper `apply_common_style(page, bucket_name, common_style, overrides_for_bucket)` that:
  1. Validates `width` and `max-width` are not both present; returns exclusivity error if so
  2. Chooses `width` → `PageFill::Explicit(...)` or `max-width` → `PageFill::Max(...)` via `lower_length_to_fill`
  3. Applies fill to the page builder if the corresponding CLI fill flag is `false`
  4. Applies alignment to the page builder via `use_alignment(component, alignment)` if the corresponding CLI alignment flag is `false`
- [ ] Wire helper three times: for `style.table`, `style.images`, `style.block_quote`
- [ ] Ensure bucket names in errors use kebab-case (`style.block-quote.max-width`, not snake_case)
- [ ] Preserve page-broadcast precedence: component frontmatter overrides `style.page.alignment` when no CLI claim exists for that component
- [ ] Write unit tests for `apply_component_style` covering:
  - Table alignment applied
  - Table max-width applied
  - Table width applied
  - Image alignment and fill applied
  - Block-quote max-width applied
  - Width + max-width exclusivity rejection
  - CLI override suppression (alignment and fill)
  - Page broadcast overridden by component frontmatter
- [ ] **Checkpoint:** All new unit tests pass; `cargo test` in `darkmatter/lib` is green

*Dependencies:* Phase 1 (types and lowering helpers must exist).

*Parallelizable:* No — sequential within phase, but the three bucket implementations can be done bucket-by-bucket if desired.

---

## Phase 3: CLI Integration and Render Pipeline

*Goal: Parse component-level CLI flags and wire `apply_component_style` into the terminal and HTML render paths.*

- [ ] Add CLI argument definitions for component-specific flags (or verify they exist from sub-spec #2):
  - `--align-tables`, `--align-images`, `--align-block-quotes`
  - `--fill-tables`, `--fill-images`, `--fill-block-quotes`
- [ ] Implement `ComponentStyleOverrides` construction from parsed `Cli` in `darkmatter-cli`
  - Global `--alignment` sets all three `*_alignment` fields to `true`
  - Global `--fill` sets all three `*_fill` fields to `true`
  - Component-specific flags set only their field to `true`
- [ ] Wire `apply_component_style` into `darkmatter/cli/src/output.rs:render_terminal_output` after `apply_page_style`
- [ ] Wire `apply_component_style` into `darkmatter/cli/src/output.rs:html_artifact` after `apply_page_style`
- [ ] Ensure helper call order is: `new` → `apply_cli_layout_flags` → `apply_page_style` → `apply_component_style` → render
- [ ] Add CLI integration tests for:
  - `--align-tables right` overriding frontmatter `style.table.alignment: left`
  - `--fill max=60` overriding frontmatter `style.table.max-width: 50%` for all components
- [ ] **Checkpoint:** CLI builds and integration tests pass; `md` and `md --output html` run without panics on documents with and without `style:` frontmatter

*Dependencies:* Phase 2 (`apply_component_style` must be implemented).

*Parallelizable:* Terminal and HTML wiring can be done in parallel once the function signature is stable.

---

## Phase 4: Active Wiring Phase Advancement

*Goal: Advance the parser's active sub-spec phase so wired keys no longer emit `KnownButInactive { sub_spec: 3 }` warnings.*

- [ ] Locate the active wiring phase constant / configuration (established in sub-spec #2)
- [ ] Advance active phase to `3`
- [ ] Verify that `table.width`, `table.max-width`, `table.alignment` no longer emit `KnownButInactive { sub_spec: 3 }`
- [ ] Verify that `images.width`, `images.max-width`, `images.alignment` no longer emit `KnownButInactive { sub_spec: 3 }`
- [ ] Verify that `block-quote.width`, `block-quote.max-width`, `block-quote.alignment` no longer emit `KnownButInactive { sub_spec: 3 }`
- [ ] Verify that `table.color`, `table.bg-color`, `images.color`, `images.bg-color`, `block-quote.color`, `block-quote.bg-color` **still** emit `KnownButInactive { sub_spec: 5 }`
- [ ] Add acceptance test asserting warning suppression for the newly active keys and continued inactivity for color keys
- [ ] **Checkpoint:** Parser warning tests pass; no `KnownButInactive { sub_spec: 3 }` for wired keys

*Dependencies:* Phase 3 (render pipeline must be wired so we know exactly which keys are live).

*Parallelizable:* Yes — the schema/config change is independent of the render pipeline wiring as long as the key set is fixed, but safest to do after.

---

## Phase 5: Integration and Regression Testing

*Goal: Validate end-to-end behavior on the fixture document and guard against regressions.*

- [ ] Run `md darkmatter/example-docs/rendering/style-prop.md` and visually verify the table is right-aligned and capped at 50% max width
- [ ] Run `md --output html darkmatter/example-docs/rendering/style-prop.md` and verify matching layout CSS for table (and image/block-quote where applicable)
- [ ] Confirm all sub-spec #1 and #2 tests continue passing (`cargo test` in `darkmatter/`)
- [ ] Confirm `apply_cli_layout_flags` behavior is unchanged for documents without `style:` frontmatter
- [ ] Add focused regression test for block-quote terminal rendering path that inspects visible width and wrapping for top-level block quotes with `style.block-quote.max-width`
- [ ] Add integration test for `style-prop.md` fixture that asserts on structural output (alignment and width constraints) without asserting unstable ANSI details
- [ ] **Checkpoint:** Full `darkmatter` test suite passes; fixture renders correctly in both terminal and HTML modes

*Dependencies:* Phases 3 and 4 (full implementation must be in place).

*Parallelizable:* Terminal fixture verification and HTML fixture verification can be done in parallel.

---

## Phase 6: Documentation Update

*Goal: Keep authoritative docs in sync with implementation.*

- [ ] Update `darkmatter/docs/rendering/style.md`:
  - Mark `table.width`, `table.max-width`, `table.alignment` as live
  - Mark `images.width`, `images.max-width`, `images.alignment` as live
  - Mark `block-quote.width`, `block-quote.max-width`, `block-quote.alignment` as live
  - Document the width/max-width exclusivity rule for these three buckets
  - Document CLI-over-frontmatter precedence for component fields
  - Use kebab-case spelling consistently (`style.block-quote.*`)
  - Note that color and background-color remain inactive (sub-spec #5)
- [ ] Update `docs/dependencies.md` or per-area `darkmatter/docs/dependencies.md` if new crates were added (unlikely for this sub-spec)
- [ ] Verify no stale references to `KnownButInactive { sub_spec: 3 }` for the wired keys remain in docs
- [ ] **Checkpoint:** Documentation review passes; doc examples match implemented behavior

*Dependencies:* Phase 5 (behavior is finalized and verified).

*Parallelizable:* Yes — docs can be drafted during Phase 5 and finalized after verification.

---

## Cross-Phase Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Width-vs-fill semantic ambiguity | Reject width/max-width combinations in this phase; document the limitation in `style.md` |
| Block-quote rendering path differs from tables/images | Add focused regression test inspecting visible width and wrapping for top-level block quotes |
| CLI precedence regression | Construct `ComponentStyleOverrides` after shorthand expansion; add global-plus-component override tests |
| Warning lifecycle drift | Acceptance tests assert warning suppression for phase 3 and continued inactivity for color keys |

---

## Task Dependency Graph

```text
Phase 1 (Types/Helpers)
    │
    ▼
Phase 2 (apply_component_style)
    │
    ├──► Phase 3 (CLI Integration)
    │        │
    │        ├──► Phase 4 (Active Phase Advancement)
    │        │        │
    │        │        └──► Phase 5 (Integration Tests)
    │        │                 │
    │        │                 └──► Phase 6 (Docs)
    │        │
    │        └──► Phase 5 can start once Phase 3 is complete
    │
    └──► Phase 6 can be drafted in parallel with Phase 5
```
