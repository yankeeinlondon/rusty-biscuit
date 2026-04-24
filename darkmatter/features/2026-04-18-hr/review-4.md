---
review: 4
reviewed: 2026-04-24
reviewer: gemini-cli
feature: horizontal-rule
ready: false
packages:
  - biscuit-terminal
  - darkmatter
test_status: failing (43 failures in biscuit-terminal, 2 in darkmatter)
---

# Review 4 — Horizontal Rule Component

The implementation has reached a high level of technical maturity, and the "hr" feature now correctly implements the three-tier progressive enhancement strategy, frontmatter page defaults, and per-rule attribute overrides as specified. All 19 suggestions from Review 3 (A2, B1–B5, C1–C5, D1–D7) have been implemented.

However, the feature is **not production-ready** because the test suite is currently in a broken state. Enabling Tier 1 image rendering for Kitty and ITerm terminals has caused 43 regression failures in `biscuit-terminal` and stale snapshots in `darkmatter`.

## `ready` verdict

**Not ready.** Although the code is high quality, the test suite must be fixed to ensure long-term maintainability and CI stability. Addressing the test environment configuration and updating the snapshots will move this to READY.

---

## Category A — Blockers

### A1. Broken Test Suite in `biscuit-terminal` (43 failures)

The activation of Tier 1 image rendering by default (via `ImageSupport::Kitty` or `ImageSupport::ITerm` detection) causes `HorizontalRule::render` to return Kitty graphics escape sequences. The existing unit tests assert on the presence of Unicode or ASCII characters and specific text-based alignment padding, which are skipped when Tier 1 is active.

**Impact**: 43 tests fail in an environment that supports Kitty/ITerm graphics.

**Fix**: Update the unit tests in `biscuit-terminal/lib/src/components/horizontal_rule.rs` to use a `Terminal` instance specifically configured with `.image_support(ImageSupport::None)` or `.is_tty(false)` to force the text-rendering tiers under test.

### A2. Stale Snapshots in `darkmatter`

The integration snapshots in `darkmatter/lib/tests/horizontal_rule_snapshots.rs` are failing for two reasons:
1.  **Terminology drift**: The move from `Placement` to `Alignment` in section headers has not been reflected in the `.snap` files.
2.  **Output format drift**: The snapshots were recorded with Unicode dashes (`╌`) but the current environment produces Kitty images.

**Fix**: Run `cargo insta review` and accept the new snapshots. Ensure that the snapshots are recorded in an environment that represents the desired baseline (ideally Unicode for text snapshots and a separate dedicated test for image output).

---

## Category B — Implementation Verification

All previous "Category B" (Spec-level gaps) have been closed:
- **B1 & B2**: `resolve_width` now supports `px` units and emits a `tracing::warn!` on unrecognized input.
- **B3**: `hr_builder.rs` correctly coerces numeric and boolean frontmatter values to strings, ensuring sibling keys are preserved.
- **B4**: The hidden 10-char minimum width has been removed in favor of a 1-column floor.
- **B5**: `ImageSupport::ITerm` now correctly triggers Tier 1 rendering.

---

## Category C — Test Coverage Verification

All previous "Category C" (Coverage gaps) have been closed:
- **C1**: `test_resolve_width_invalid_warns_and_falls_back` added.
- **C2**: `hr_defaults_from_frontmatter_numeric_width_preserves_siblings` added.
- **C3**: `test_render_image_tier_falls_through_on_rasterization_failure` added.
- **C4**: `test_render_curtain_rod_thick_right_has_brackets_and_heavy_line` added.
- **C5**: `test_blockquote_hr_renders_with_frontmatter_defaults` added and spec updated.

---

## Category D — Polish & Ergonomics Verification

All previous "Category D" (Polish) have been closed:
- **D1**: `HorizontalRule` now derives `PartialEq`.
- **D2**: `Terminal` now has a `supports_unicode` capability consulted by the renderer.
- **D3**: `HtmlOptions.hr_css_variables` is now a bare `HashMap`.
- **D4**: `Margin::Offset` correctly emits `calc()` for CSS compatibility.
- **D5**: Tier 1 rendering uses `DEFAULT_CELL_WIDTH/HEIGHT` constants.
- **D7**: `parse_basic_color` now uses `eq_ignore_ascii_case` to avoid allocations.

---

## Summary

| # | Item | Status |
|---|------|--------|
| A1 | `biscuit-terminal` test regression (43 fails) | **Blocker** |
| A2 | `darkmatter` stale snapshots | **Blocker** |
| B1-B5 | Spec gaps closed | Verified |
| C1-C5 | Coverage gaps closed | Verified |
| D1-D7 | Polish items closed | Verified |
