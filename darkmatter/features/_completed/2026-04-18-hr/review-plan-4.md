---
plan_for: review-4.md
feature: horizontal-rule
packages:
  - biscuit-terminal
  - darkmatter
phases: 3
tdd: true
---

# Implementation Plan - Review 4 (Horizontal Rule)

This plan addresses every remaining recommendation in
[`review-4.md`](./review-4.md). The feature behavior is considered
complete by the review; the work here is to restore deterministic test
baselines, refresh stale snapshots, and prove that the `darkmatter`
package area and its rendering dependency are warning-free.

Target: green tests for the horizontal-rule surface, green package-area
tests, and zero lint warnings/errors for `biscuit-terminal`,
`biscuit-terminal-cli`, `darkmatter`, and `darkmatter-cli`.

## Source Context

Before editing, read:

- `darkmatter/features/2026-04-18-hr/spec.md`
- `darkmatter/features/2026-04-18-hr/tech-design.md`
- `darkmatter/features/2026-04-18-hr/review-4.md`
- `biscuit-terminal/lib/src/components/horizontal_rule.rs`
- `darkmatter/lib/src/markdown/output/terminal.rs`
- `darkmatter/lib/tests/horizontal_rule_snapshots.rs`

Relevant skills: `darkmatter`, `biscuit-terminal`, `rust`, and
`rust-testing`.

## Execution Rules

- Work one phase at a time.
- Do not use interactive snapshot review. Use `INSTA_UPDATE=always cargo
  test ...`, then inspect the resulting diff.
- Do not update snapshots until the test harness explicitly selects the
  intended baseline.
- Keep Tier 1 image rendering covered by explicit image-tier tests.
  Text/Unicode/ASCII tests must opt out of image rendering explicitly.
- Do not change production HR behavior unless needed to make test
  intent selectable through existing options.

## Phase 1 - Fix `biscuit-terminal` HR Test Regressions (A1)

**Goal:** All text-mode `HorizontalRule` tests run against a terminal
that cannot enter Tier 1, while Tier 1 remains tested separately.

**Files to inspect/change:**

- `biscuit-terminal/lib/src/components/horizontal_rule.rs`
- `biscuit-terminal/lib/src/components/mod.rs`
- `biscuit-terminal/lib/src/components/horizontal_rule_snapshot.rs`
- `biscuit-terminal/lib/src/components/horizontal_rule_test.rs`

**Steps:**

1. Confirm which files compile. `components/mod.rs` should only include
   `pub mod horizontal_rule;`; if `horizontal_rule_snapshot.rs` and
   `horizontal_rule_test.rs` remain unreferenced legacy files, do not
   chase them for review-4 unless the compiler includes them.

   ```bash
   rg -n 'horizontal_rule_snapshot|horizontal_rule_test|mod horizontal_rule' \
     biscuit-terminal/lib/src
   ```

2. Reproduce the failure class:

   ```bash
   cargo test -p biscuit-terminal horizontal_rule --lib
   ```

   Expected before the fix in a Kitty/iTerm-capable test environment:
   text assertions or terminal snapshots receive image escape output
   such as `\x1b_G`.

3. In `biscuit-terminal/lib/src/components/horizontal_rule.rs`, audit
   every test that asserts Unicode, ASCII, exact padding, visible width,
   text color, or a terminal snapshot. These tests must use one of the
   existing text helpers:

   - `text_terminal()`
   - `text_terminal_with_width(width)`
   - `text_terminal_with_color_depth(color_depth)`
   - `text_terminal_with_width_and_color_depth(width, color_depth)`
   - `text_terminal_with_width_and_unicode(width, supports_unicode)`

   These helpers set `term.is_tty = false` and
   `term.image_support = ImageSupport::None`, which is the correct
   baseline for Tier 2/Tier 3 tests.

4. Replace any remaining `Terminal::default()`, `Terminal::new()`,
   `Terminal::new_optimistic(...)`, or `Terminal::builder().build()`
   in text-mode tests with a helper from step 3. If a builder is needed
   for a special case, set both:

   ```rust
   .is_tty(false)
   .image_support(ImageSupport::None)
   ```

5. Keep these positive Tier 1 tests explicit and isolated:

   - Kitty image support: `is_tty(true)` +
     `image_support(ImageSupport::Kitty)` and assert `\x1b_G`.
   - iTerm image support: `is_tty(true)` +
     `image_support(ImageSupport::ITerm)` and assert the expected
     image protocol escape.
   - Rasterization failure fallback: construct the failure case
     directly and assert it falls through to text without removing
     the image-tier branch.

6. Add or update one regression test in the inline test module:

   - Suggested name:
     `test_text_terminal_helper_disables_image_tier`.
   - Build a rule that would normally prefer Tier 1, render it with
     `text_terminal()`, and assert the output does not contain `\x1b_G`
     or iTerm image escapes and does contain the expected text fallback.

7. Run the targeted suite:

   ```bash
   cargo test -p biscuit-terminal horizontal_rule --lib
   ```

8. If any HR snapshots changed because they were previously recorded
   from Tier 1 output, re-record only after the test has been forced to
   text mode:

   ```bash
   INSTA_UPDATE=always cargo test -p biscuit-terminal horizontal_rule --lib
   git diff -- biscuit-terminal/lib/src/components/snapshots/
   ```

   Accept only diffs that restore the intended Unicode/ASCII/text
   baseline. Do not accept image escape sequences in text snapshots.

**Required coverage after this phase:**

- Text Unicode tests prove `ImageSupport::None` cannot enter Tier 1.
- Text ASCII tests prove `supports_unicode(false)` or non-UTF-8 locale
  reaches Tier 3.
- Dedicated Kitty/iTerm tests prove Tier 1 still activates when
  explicitly requested.

## Phase 2 - Stabilize `darkmatter` HR Snapshots (A2)

**Goal:** Darkmatter terminal snapshots are recorded against a stable
Unicode/text baseline, stale terminology is refreshed, and image output
is covered separately instead of leaking into text snapshots.

**Files to inspect/change:**

- `darkmatter/lib/src/markdown/output/terminal.rs`
- `darkmatter/lib/tests/horizontal_rule_snapshots.rs`
- `darkmatter/lib/tests/horizontal_rule_integration.rs`
- `darkmatter/lib/tests/snapshots/horizontal_rule_snapshots__tests__*.snap`

**Steps:**

1. Confirm current snapshot failures:

   ```bash
   cargo test -p darkmatter --test horizontal_rule_snapshots
   ```

   Expected before the fix: stale `Placement`/`Alignment` headings
   and/or image escape output where Unicode text snapshots are expected.

2. Make the terminal rendering pipeline honor the existing image
   disable switch for component rendering. In
   `darkmatter/lib/src/markdown/output/terminal.rs`, update construction
   of the shared `render_terminal` so `TerminalImageMode::Never`
   forces:

   ```rust
   .is_tty(false)
   .image_support(ImageSupport::None)
   ```

   Keep `TerminalImageMode::Auto` capability-driven. For
   `TerminalImageMode::Force`, preserve the existing force behavior used
   by image rendering; if needed, set `is_tty(true)` and ensure
   `ImageSupport::None` is upgraded to `ImageSupport::Kitty` only in the
   force path.

3. Add a focused integration test in
   `darkmatter/lib/tests/horizontal_rule_integration.rs`:

   - Suggested name:
     `test_terminal_image_mode_never_disables_hr_image_tier`.
   - Use markdown such as
     `--- { style: waves, alignment: centered, color: "red" }`.
   - Set `TerminalOptions { image_mode: TerminalImageMode::Never,
     max_width: Some(40), color_depth: Some(ColorDepth::TrueColor),
     ..Default::default() }`.
   - Assert the terminal output does not contain image protocol escapes
     (`\x1b_G` for Kitty and the iTerm image introducer if present in
     the component tests) and does contain the text fallback (`≋` or
     `~`).

4. Add or preserve a separate positive image-path test. If no
   darkmatter-level test can force image output without broad API
   changes, rely on the biscuit-terminal Tier 1 tests from Phase 1 and
   document that darkmatter's integration contract is option routing,
   not rasterization internals. Do not let image escapes become the
   terminal snapshot baseline.

5. Update `darkmatter/lib/tests/horizontal_rule_snapshots.rs` so
   terminal snapshot tests use a local helper instead of
   `TerminalOptions::default()`:

   ```rust
   fn terminal_snapshot_options() -> TerminalOptions {
       let mut options = TerminalOptions::default();
       options.image_mode = TerminalImageMode::Never;
       options.max_width = Some(80);
       options.color_depth = Some(ColorDepth::TrueColor);
       options
   }
   ```

   Use the helper for every call to `for_terminal(...)` in this file.
   Leave HTML snapshots unchanged.

6. Re-record snapshots non-interactively:

   ```bash
   INSTA_UPDATE=always cargo test -p darkmatter --test horizontal_rule_snapshots
   ```

7. Inspect the diff:

   ```bash
   git diff -- darkmatter/lib/tests/snapshots/
   find darkmatter biscuit-terminal -name '*.snap.new' -print
   ```

   Required diff properties:

   - No `.snap.new` files remain.
   - The complex document snapshots say `Alignment Options`, not
     `Placement Options`.
   - Terminal HR snapshots contain Unicode/text output, not image
     protocol escapes.
   - HTML snapshots only change where the source fixture terminology
     changed.

8. Run the darkmatter HR integration tests:

   ```bash
   cargo test -p darkmatter --test horizontal_rule_integration
   cargo test -p darkmatter --test horizontal_rule_snapshots
   ```

**Required coverage after this phase:**

- Page-level HR defaults and per-rule overrides remain covered by
  integration tests.
- Blockquote HR behavior remains covered by the existing review-3
  regression test.
- Snapshot tests pin the desired text baseline.
- Image-tier behavior remains covered at the component level, and
  darkmatter covers the option that disables image output for
  deterministic text rendering.

## Phase 3 - Package-Area Verification and Lint Cleanup

**Goal:** Prove the review-4 fixes did not introduce warnings,
regressions, or snapshot drift in either affected area.

**Files to inspect/change if verification fails:**

- `biscuit-terminal/lib/src/components/horizontal_rule.rs`
- `darkmatter/lib/src/markdown/output/terminal.rs`
- `darkmatter/lib/tests/horizontal_rule_integration.rs`
- `darkmatter/lib/tests/horizontal_rule_snapshots.rs`
- Any file named by compiler, clippy, rustfmt, or insta output.

**Steps:**

1. Format the affected Rust packages:

   ```bash
   cargo fmt -p biscuit-terminal -p biscuit-terminal-cli -p darkmatter -p darkmatter-cli
   ```

2. Run targeted HR tests first:

   ```bash
   cargo test -p biscuit-terminal horizontal_rule --lib
   cargo test -p darkmatter --test horizontal_rule_integration
   cargo test -p darkmatter --test horizontal_rule_snapshots
   ```

3. Run full tests for the affected packages:

   ```bash
   cargo test -p biscuit-terminal
   cargo test -p biscuit-terminal-cli
   cargo test -p darkmatter
   cargo test -p darkmatter-cli
   ```

4. Run lint with warnings denied:

   ```bash
   cargo clippy -p biscuit-terminal -p biscuit-terminal-cli --all-targets -- -D warnings
   cargo clippy -p darkmatter -p darkmatter-cli --all-targets -- -D warnings
   ```

   The area justfiles may also be used as a cross-check:

   ```bash
   just -f biscuit-terminal/justfile lint
   just -f darkmatter/justfile lint
   ```

5. Check for snapshot leftovers and unintended files:

   ```bash
   find biscuit-terminal darkmatter -name '*.snap.new' -print
   git diff --stat
   ```

6. If any verification command fails:

   - Fix the smallest relevant cause.
   - Re-run the failing command.
   - Re-run the targeted HR tests from step 2.
   - If a lint fix changes behavior, re-run the full package tests from
     step 3.

## Acceptance Checklist

- [ ] A1 fixed: `biscuit-terminal` text-mode HR tests explicitly use
      `ImageSupport::None` or `is_tty(false)`.
- [ ] A1 preserved: explicit Kitty/iTerm Tier 1 tests still assert image
      protocol output.
- [ ] A2 fixed: darkmatter HR snapshots use `Alignment` terminology and
      a deterministic text baseline.
- [ ] A2 covered: `TerminalImageMode::Never` disables HR image output in
      the darkmatter terminal pipeline.
- [ ] No text snapshot contains Kitty/iTerm image escape sequences.
- [ ] No `.snap.new` files remain.
- [ ] `cargo test -p biscuit-terminal horizontal_rule --lib` passes.
- [ ] `cargo test -p darkmatter --test horizontal_rule_integration`
      passes.
- [ ] `cargo test -p darkmatter --test horizontal_rule_snapshots`
      passes.
- [ ] Full tests pass for `biscuit-terminal`, `biscuit-terminal-cli`,
      `darkmatter`, and `darkmatter-cli`.
- [ ] Clippy passes with `-D warnings` for both package areas.
