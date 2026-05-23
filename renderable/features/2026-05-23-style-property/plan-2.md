---
phases: 3
created: 2026-05-23
start_phase: 1
source_files_during_phase_1:
  - darkmatter/lib/src/style/apply.rs
  - darkmatter/lib/src/style/mod.rs
  - darkmatter/lib/src/style/parse.rs
  - darkmatter/lib/src/style/coverage_tests.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/cli/src/args.rs
  - darkmatter/cli/src/output.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/tests/style_frontmatter.rs
  - darkmatter/cli/tests/cli.rs
docs_updated_during_phase_3:
  - darkmatter/docs/rendering/style.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
  - darkmatter
---

# Plan: Sub-Spec #2 — Page-Level Wiring

This plan wires the `style.page.*` frontmatter subset into the `DarkmatterPage` builder used by the `md` CLI. It introduces CLI-over-frontmatter precedence, `--strict-style` validation, and suppresses `KnownButInactive` warnings for the newly active keys.

## Phase 1: Library Surface & Warning Logic

Prepare the `darkmatter` library for page-level style application and update the parser to recognize Sub-Spec #2 as "active".

- [x] **Task 1: Define `PageStyleOverrides` and `StyleApplyError`**
    - Create `darkmatter/lib/src/style/apply.rs` (or add to `mod.rs`).
    - Define `PageStyleOverrides` struct as specified in Sub-Spec #2.
    - Define `StyleApplyError` enum (e.g., `InvalidCssLength`, `InvalidMaxWidth`).
    - Re-export them in `darkmatter/lib/src/style/mod.rs`.

- [x] **Task 2: Update `ACTIVE_STYLE_WIRING_SUB_SPEC` and Warning Suppression**
    - In `darkmatter/lib/src/style/parse.rs`:
        - Add `const ACTIVE_STYLE_WIRING_SUB_SPEC: u8 = 2;`.
        - Update `emit_known_but_inactive` (or the walk logic) to only emit `KnownButInactive` if `leaf.sub_spec > ACTIVE_STYLE_WIRING_SUB_SPEC`.
    - Verify that `KnownButInactive { sub_spec: 2 }` is no longer emitted for page-level keys.

- [x] **Task 3: Implement `apply_page_style` Logic**
    - In `darkmatter/lib/src/style/apply.rs`:
        - Implement `apply_page_style(page: DarkmatterPage, style: &StyleFrontmatter, overrides: PageStyleOverrides) -> Result<DarkmatterPage, StyleApplyError>`.
        - Implement logic for lowering `Length` to `u16`/`u32` for `DarkmatterPage` builders.
        - Handle `Length::Percent` resolution against terminal width.
        - Handle CLI overrides (if override is true, skip frontmatter field).
        - Implement `page.alignment` broadcasting via `use_alignment_for_all`.

- [x] **Task 4: Unit Tests for `apply_page_style`**
    - Add tests for:
        - `Length` resolution (Zero, Ch, Percent).
        - Error on `Length::Css` for terminal layout.
        - Precedence (CLI override true vs false).
        - Percent `max-width` resolution post-margin/padding.
        - `page.alignment` broadcasting.

## Phase 2: CLI Integration

Wire the library changes into the `md` CLI and add the `--strict-style` flag.

- [x] **Task 5: Add `--strict-style` to CLI Arguments**
    - In `darkmatter/cli/src/args.rs`:
        - Add `strict_style: bool` field to `Cli` struct with `#[arg(long)]`.

- [x] **Task 6: Integrate Style Application in `render_terminal_output`**
    - In `darkmatter/cli/src/output.rs`:
        - Parse style frontmatter using `darkmatter::style::from_frontmatter`.
        - If `cli.strict_style` is true, call `into_strict`.
        - Log non-fatal warnings (using `tracing` or `eprintln!`).
        - Construct `PageStyleOverrides` from `cli` flags (shorthand expansion).
        - Call `apply_page_style` before `page.render(md)`.

- [x] **Task 7: Integrate Style Application in `html_artifact`**
    - Ensure the same parse/strict/apply sequence is used for HTML artifacts.
    - `html_artifact` should also respect `cli.strict_style` and apply page-level styles to `DarkmatterPage` before calling `render_to_browser`.

## Phase 3: Integration & Documentation

Validate the end-to-end behavior and update documentation.

- [x] **Task 8: Update Documentation**
    - Update `darkmatter/docs/rendering/style.md` to reflect that page-level support is live.
    - Document CLI-over-frontmatter precedence.
    - List supported `style.page.*` keys and their expected values.

- [x] **Task 9: Integration Tests for Style Rendering**
    - Extend `darkmatter/lib/tests/style_frontmatter.rs` or add a new integration test.
    - Assert that a `Markdown` with `style:` frontmatter results in a `DarkmatterPage` with the expected margins/padding/alignment when applied.

- [x] **Task 10: Snapshot Validation with `style-prop.md`**
    - Use `darkmatter/example-docs/rendering/style-prop.md` as a test fixture.
    - Verify that `md style-prop.md` produces terminal output with correct margins.
    - Verify that `md --output html style-prop.md` also correctly applies page-level styles.
    - Confirm no `KnownButInactive { sub_spec: 2 }` warnings are emitted.
