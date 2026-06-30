---
agent: gemini
phases: 4
start_phase: 1
created: '2026-05-05T12:16:49'
source_review: review-1.md
source_files_during_phase_1:
  - ../biscuit-test-harness/src/lib.rs
  - ../biscuit-test-harness/src/tmux.rs
  - lib/src/components/choice_state.rs
  - lib/src/components/choose_many.rs
  - lib/src/components/choose_one.rs
  - lib/src/components/mod.rs
  - lib/src/lib.rs
  - lib/src/prelude.rs
docs_updated_during_phase_1:
  - reviews/2026-05-04-sentrux/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - ../.claude/skills/biscuit-tui/SKILL.md
source_files_during_phase_2:
  - cli/src/commands/input_table/columns.rs
  - cli/src/commands/input_table/mod.rs
  - cli/src/commands/input_table/tests.rs
  - lib/src/components/choice_render/badge.rs
  - lib/src/components/choice_render/highlight.rs
  - lib/src/components/choice_render/horizontal.rs
  - lib/src/components/choice_render/mod.rs
  - lib/src/components/choice_render/tests.rs
  - lib/src/components/choice_render/vertical.rs
  - lib/src/components/choose_many.rs
  - lib/src/components/choose_many/tests.rs
  - lib/src/components/choose_one.rs
  - lib/src/components/choose_one/tests.rs
  - lib/src/components/input_table/table.rs
  - lib/src/components/input_table/table/tests.rs
  - lib/src/core/standalone/inline_viewport.rs
  - lib/src/core/standalone/loop_driver.rs
  - lib/src/core/standalone/mod.rs
  - lib/src/core/standalone/terminal_lifecycle.rs
  - lib/src/core/standalone/tests.rs
docs_updated_during_phase_2:
  - reviews/2026-05-04-sentrux/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - ../.claude/skills/biscuit-tui/SKILL.md
source_files_during_phase_3:
  - cli/src/choice_normalize.rs
  - cli/src/main_tmp.rs
  - cli/src/option_sources.rs
  - lib/src/components/choice_layout.rs
  - lib/src/components/choose_many.rs
  - lib/src/components/choose_one.rs
  - lib/src/helpers/choice_builders.rs
  - lib/src/helpers/mod.rs
  - lib/src/prelude.rs
docs_updated_during_phase_3:
  - ../docs/dependencies.md
  - reviews/2026-05-04-sentrux/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - ../.claude/skills/biscuit-tui/SKILL.md
source_files_during_phase_4:
  - cli/src/commands/choose_many.rs
  - cli/src/commands/choose_one.rs
  - cli/src/commands/common_choose.rs
  - cli/tests/completions_shell.rs
docs_updated_during_phase_4:
  - reviews/2026-05-04-sentrux/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - ../.claude/skills/biscuit-tui/SKILL.md
packages:
  - biscuit-tui-cli
---

# biscuit-tui Refactor Plan

This plan operationalizes the 15 suggestions in [`review-1.md`](./review-1.md). The refactor targets modularity, code equality (reducing LOC concentration), and removing redundancy across the `biscuit-tui` library and `biscuit-tui-cli`.

## Phase 1: Critical Modularity & State Consolidation
*Goal: Fix the primary peer coupling and reduce massive duplication in choice components.*

- [x] **Decouple ChooseOne/ChooseMany:**
    - Create `lib/src/components/choice_state.rs`.
    - Move shared helpers (`HOTKEY_DISPLAY_FALLBACK`, `build_effective_hotkeys`, etc.) from `choose_one.rs` to the new module.
    - Update both `choose_one.rs` and `choose_many.rs` to import from the new module.
- [x] **Extract `ChoiceCommonState<V>`:**
    - Define the shared state struct in `choice_state.rs`.
    - Refactor `ChooseOneState` and `ChooseManyState` to use this common substructure.
    - Consolidate builder methods (`with_label`, `with_theme`, etc.) using a trait or macro.
- [x] **Centralize Re-exports:**
    - Update `lib/src/prelude.rs` to be the sole authority for public types.
    - Refactor `lib/src/lib.rs` to glob-re-export `prelude`.
    - Re-export `helpers` functions in the prelude for better ergonomics.

## Phase 2: Code Equality & File Decomposition
*Goal: Break up the five "god files" (>1500 LOC) by extracting tests and submodules.*

- [x] **Extract Test Modules:**
    - Move tests from `choose_one.rs`, `choose_many.rs`, `standalone.rs`, `choice_render.rs`, and `input_table/table.rs` into sibling `tests.rs` or directory-based modules.
- [x] **Decompose `choice_render`:**
    - Split `choice_render.rs` into a directory module: `choice_render/{mod.rs, vertical.rs, horizontal.rs, badge.rs, highlight.rs}`.
    - Flatten complex rendering functions into smaller, focused helpers.
- [x] **Decompose `core::standalone`:**
    - Split `standalone.rs` into `core/standalone/{mod.rs, loop_driver.rs, terminal_lifecycle.rs, inline_viewport.rs}`.
- [x] **Decompose `input_table` CLI:**
    - Split `cli/src/commands/input_table.rs` into `input_table/{mod.rs, columns.rs, tests.rs}`.

## Phase 3: Redundancy Elimination
*Goal: Remove duplicated logic and orphaned files across the package area.*

- [x] **Cleanup CLI Artifacts:**
    - Delete `cli/src/main_tmp.rs`.
- [x] **Consolidate Choice Parsing:**
    - Promote markdown/dictionary/CSV parsers in `lib/src/helpers/choice_builders.rs` to canonical status.
    - Refactor `cli/src/option_sources.rs` to call these library helpers.
- [x] **Leverage `heck` crate:**
    - Replace hand-rolled case conversion in `cli/src/choice_normalize.rs` with `heck`.
- [x] **Refactor Choice Layout:**
    - Move `navigate_row` to be a method on `ChoiceLayout`.

## Phase 4: CLI Argument Ergonomics
*Goal: Consolidate duplicated clap argument structures.*

- [x] **Lift `ChooseSourceArgs`:**
    - Create a shared struct in `cli/src/commands/common_choose.rs` for source-resolution flags.
    - Update `ChooseOneArgs` and `ChooseManyArgs` to flatten this shared struct.
- [x] **Generic Command Run Plumbing:**
    - Extract shared control flow (resolve -> normalize -> build -> run -> format) into `common_choose.rs`.
    - Collapse `run()` implementations in `choose_one.rs` and `choose_many.rs`.
- [x] **Final Validation:**
    - Run `just lint` and `just test` across the entire package area.
    - Verify `quality_signal` improvement via Sentrux (if available).
