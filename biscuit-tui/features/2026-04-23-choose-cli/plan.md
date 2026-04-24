---
phases: 12
created: 2026-04-23
start_phase: 1
memory: choose-cli
source_files_during_phase_1:
  - biscuit-tui/lib/Cargo.toml
  - biscuit-tui/lib/src/core/sort.rs
  - biscuit-tui/lib/src/core/frame.rs
  - biscuit-tui/lib/src/core/mod.rs
  - biscuit-tui/lib/src/lib.rs
  - biscuit-tui/lib/src/prelude.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase1: []
source_files_during_phase_2:
  - biscuit-tui/lib/src/core/standalone.rs
  - biscuit-tui/lib/src/core/mod.rs
  - biscuit-tui/lib/src/lib.rs
  - biscuit-tui/lib/src/prelude.rs
  - biscuit-tui/cli/src/commands/text_input.rs
  - biscuit-tui/cli/src/commands/text_area_input.rs
  - biscuit-tui/cli/src/commands/boolean_switch.rs
  - biscuit-tui/cli/src/commands/choose_one.rs
  - biscuit-tui/cli/src/commands/choose_many.rs
  - biscuit-tui/cli/src/commands/input_table.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
source_files_during_phase_3:
  - biscuit-tui/cli/src/commands/common_choose.rs
  - biscuit-tui/cli/src/commands/mod.rs
  - biscuit-tui/cli/src/commands/choose_one.rs
  - biscuit-tui/cli/src/commands/choose_many.rs
  - biscuit-tui/cli/tests/choose_cli.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase3: []
source_files_during_phase_4:
  - biscuit-tui/lib/src/components/choose_one.rs
  - biscuit-tui/lib/src/components/choose_many.rs
  - biscuit-tui/cli/src/commands/common_choose.rs
  - biscuit-tui/cli/src/commands/choose_one.rs
  - biscuit-tui/cli/src/commands/choose_many.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase4: []
source_files_during_phase_5:
  - biscuit-tui/lib/src/components/choose_one.rs
  - biscuit-tui/lib/src/components/choose_many.rs
  - biscuit-tui/lib/src/components/input_table/cell.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase5: []
source_files_during_phase_6:
  - biscuit-tui/lib/src/core/keybindings.rs
  - biscuit-tui/lib/src/components/choose_many.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase6: []
source_files_during_phase_7:
  - biscuit-tui/lib/src/core/fuzzy.rs
  - biscuit-tui/lib/src/core/mod.rs
  - biscuit-tui/lib/src/lib.rs
  - biscuit-tui/lib/src/prelude.rs
docs_updated_during_phase_7: []
docs_created_during_phase_7: []
skills_files_updated_during_phase7: []
packages:
  - tui-chrome
  - tui-chrome-cli
---

# Choose CLI Enhancements — Execution Plan

Derived from:
- [Functional Specification](./spec.md)
- [Technical Design](./tech-design.md)

This plan translates the design into concrete, observable steps ordered by dependency. Phases are sequential unless marked **PARALLEL**.

---

## Phase 1 — Core Frame & Sort Primitives

**Goal:** Introduce the new library primitives that later phases depend on.

**Steps:**

1. Add `nucleo-matcher = "0.3"` to `tui-chrome/Cargo.toml` (feature-gated or direct; confirm with maintainer).
2. Create `lib/src/core/sort.rs` — define `SortOrder` enum and `apply()` method.
3. Create `lib/src/core/frame.rs` — define `Margin`, `HeightSpec`, `BorderStyle`, `FrameChrome`.
4. Add `mod sort; mod frame;` to `lib/src/core/mod.rs`.
5. Re-export new types from `lib/src/lib.rs` and `lib/src/prelude.rs`.
6. Wire unit tests for `sort.rs` and `frame.rs` (see tech-design §10.1).

**Validation Checkpoint:**
- `cargo test -p tui-chrome core::sort core::frame` passes.
- New types are reachable from `tui_chrome::` root.

---

## Phase 2 — Event Loop Exit Codes

**Goal:** Distinguish Esc from Ctrl-C so the CLI can return different exit codes.

**Steps:**

1. Introduce `LoopExit<V>` enum in `lib/src/core/standalone.rs`.
2. Define `CANCELLED_KIND` and `ABORTED_KIND` sentinel constants.
3. Update `drive_event_loop_with_hint` to return `LoopExit<V>` instead of `Result<V, io::Error>`.
4. Update `run_standalone` to map `LoopExit` variants back to `Ok(V)` / `Err(...)`.
5. Add unit tests for `loop_exit_distinguishes_esc_from_ctrl_c`.

**Validation Checkpoint:**
- `cargo test -p tui-chrome core::standalone` passes.
- Existing component tests still compile (may need mechanical `?` -> `match` updates).

---

## Phase 3 — CLI Source Resolution (STDIN, Positional, Delimiter)

**Goal:** Allow `choose-one` / `choose-many` to read options from STDIN, positional args, and split label/value.

**Steps:**

1. Create `cli/src/commands/common_choose.rs`.
2. Define `ChooseChromeArgs` struct with clap derives.
3. Implement `resolve_option_strings()` (stdin vs positional vs legacy).
4. Implement `parse_label_value()` per tech-design §4.3.
5. Implement `build_options()` and `build_chrome()` helpers.
6. Refactor `cli/src/commands/choose_one.rs` and `choose_many.rs` to flatten `ChooseChromeArgs`.
7. Deprecate `--initial` in favour of `--selected` (hide from help, warn on stderr).
8. Add integration test scaffold in `cli/tests/choose_cli.rs`.

**Validation Checkpoint:**
- `cargo test -p tui-chrome-cli --test choose_cli choose_one_reads_from_stdin` passes.
- `cargo test -p tui-chrome-cli --test choose_cli delimiter_separates_label_and_value` passes.
- `cargo clippy -p tui-chrome-cli` is clean.

---

## Phase 4 — Pre-Selection & Sort Wiring

**Goal:** Wire `--selected` and `--sort` through to state construction.

**Steps:**

1. Add `with_initial_value(&str)` to `ChooseOneState`.
2. Add `with_initial_values(&[&str])` to `ChooseManyState`.
3. Update CLI commands to call the new pre-selection methods.
4. Apply `SortOrder::apply()` after `build_options()` and before state construction.
5. Add unit tests for pre-selection and sort in `components::choose_one` / `choose_many`.

**Validation Checkpoint:**
- `cargo test -p tui-chrome components::choose_one::initial_value_pre_selects` passes.
- `cargo test -p tui-chrome components::choose_many::initial_values_pre_select_by_value` passes.
- `cargo test -p tui-chrome core::sort` passes.

---

## Phase 5 — Fallback Submit on Active

**Goal:** If Enter is pressed with no explicit selection, submit the currently active (hovered) item.

**Steps:**

1. Update `ChooseOneState::submit` to promote hover -> selected when `selected.is_none()`.
2. Update `ChooseManyState::submit` to select the active item when `selected_count() == 0`.
3. Ensure disabled items are skipped during fallback.
4. Add unit tests for fallback in both components.

**Validation Checkpoint:**
- `cargo test -p tui-chrome components::choose_one::fallback_submit_promotes_hover` passes.
- `cargo test -p tui-chrome components::choose_many::fallback_submit_selects_active_when_none_chosen` passes.

---

## Phase 6 — Ctrl+A / Ctrl+D (choose-many only)

**Goal:** Add bulk select / deselect hotkeys.

**Steps:**

1. Add `select_all` and `deselect_all` fields to `KeyBindings`.
2. Implement `ChooseManyState::select_all()` / `deselect_all()` (skip disabled on select_all).
3. Update `ChooseMany::handle_event` to match the new bindings.
4. Add unit tests for bulk operations.

**Validation Checkpoint:**
- `cargo test -p tui-chrome components::choose_many::ctrl_a_selects_all_enabled_options` passes.
- `cargo test -p tui-chrome components::choose_many::ctrl_d_clears_all` passes.

---

## Phase 7 — FuzzyFilter Library

**Goal:** Build the standalone `FuzzyFilter` that powers search-on-type.

**Steps:**

1. Create `lib/src/core/fuzzy.rs`.
2. Implement `FuzzyFilter` using `nucleo_matcher` per tech-design §6.2.
3. Expose `pattern`, `set_pattern`, `push_char`, `pop_char`, `clear`, `visible`, `is_active`.
4. Add unit tests for filtering, scoring, empty pattern, and mutation.

**Validation Checkpoint:**
- `cargo test -p tui-chrome core::fuzzy` passes.
- `FuzzyFilter` is re-exported from `lib.rs`.

---

## Phase 8 — Search Prompt Rendering & State Plumbing

**Goal:** Integrate `FuzzyFilter` into the choose components and render the search prompt.

**Steps:**

1. Add `filter: FuzzyFilter` and `filter_visible: bool` to `ChooseOneState` and `ChooseManyState`.
2. Add `with_filter_enabled(bool)` to `ChoiceInput`.
3. Update event routing so alphanumeric input opens the filter when enabled.
4. Update rendering to show the search prompt row when `filter_visible` is true.
5. Highlight matched characters using `nucleo_matcher::pattern::Pattern::indices()`.
6. Handle empty-filter state: show `(no matches)` and block submit.
7. Handle Esc behaviour: clear filter first, then abort on second Esc.
8. Update navigation (Up/Down/Space) to walk `visible_indices()`.
9. Add unit tests for search open, filter, navigation, and Esc clear.

**Validation Checkpoint:**
- `cargo test -p tui-chrome components::choose_one::typing_letter_opens_filter` passes.
- `cargo test -p tui-chrome components::choose_many::submit_blocked_when_filter_hides_everything` passes.
- Manual QA: run `printf 'a\nb\nc' | question choose-one`, type "b", only "b" remains visible.

---

## Phase 9 — Border CLI Flags

**Goal:** Wire `--border`, `--border-label`, and `--border-style` through to `FrameChrome`.

**Steps:**

1. Add `BorderStyleArg` clap value enum to `common_choose.rs`.
2. Implement `BorderStyle -> (Borders, BorderType)` mapping per tech-design §7.1.
3. Update `build_chrome()` to construct `FrameChromeConfig` from parsed args.
4. Update `run_standalone_with_chrome` call sites in `choose_one.rs` and `choose_many.rs`.
5. Add integration test for border rendering.

**Validation Checkpoint:**
- `cargo test -p tui-chrome-cli --test choose_cli` border tests pass.
- Manual QA: `question choose-one a b c --border --border-label "Pick"` draws a labelled border.

---

## Phase 10 — Margin CLI Flags

**Goal:** Wire `--margin` and per-side overrides.

**Steps:**

1. Parse `--margin`, `--mt`, `--mb`, `--ml`, `--mr` in `common_choose.rs`.
2. Apply per-side overrides in `build_chrome()`.
3. Add integration test for margin geometry.

**Validation Checkpoint:**
- `cargo test -p tui-chrome-cli --test choose_cli` margin tests pass.
- Manual QA: `question choose-one a b c --margin 2 --mt 0` shows correct spacing.

---

## Phase 11 — Height CLI Flag

**Goal:** Support `--height <cells | %>`.

**Steps:**

1. Add `HeightSpecArg` value parser in `common_choose.rs`.
2. Resolve `HeightSpec` against terminal rows in `run_standalone_with_chrome`.
3. Ensure `Percent` clamps to a floor of 3 rows.
4. Add integration tests for cell and percent heights.

**Validation Checkpoint:**
- `cargo test -p tui-chrome-cli --test choose_cli` height tests pass.
- Manual QA: `question choose-one a b c --height 50%` renders at half terminal height.

---

## Phase 12 — Integration Tests, Manual QA & CHANGELOG

**Goal:** Lock in behaviour and ship.

**Steps:**

1. Write remaining integration tests in `cli/tests/choose_cli.rs`:
   - `choose_one_positional_args`
   - `choose_many_ctrl_a_then_submit_writes_all_values`
   - `esc_exits_with_code_1`
   - `ctrl_c_exits_with_code_130`
2. Run the full manual QA checklist from tech-design §10.4.
3. Update `CHANGELOG.md` with breaking change notice for Esc exit code.
4. Run `cargo test -p tui-chrome -p tui-chrome-cli` and ensure everything passes.
5. Run `cargo clippy -p tui-chrome -p tui-chrome-cli -- -D warnings`.
6. Run `cargo fmt --all --check`.

**Validation Checkpoint:**
- All unit and integration tests pass.
- Manual QA checklist fully checked off.
- CHANGELOG entry merged.
- CI is green.

---

## Dependency Graph

```
Phase 1 (core primitives)
   │
   ▼
Phase 2 (event loop exits)
   │
   ▼
Phase 3 (CLI source resolution)
   │
   ▼
Phase 4 (pre-selection + sort)
   │
   ▼
Phase 5 (fallback submit)
   │
   ▼
Phase 6 (Ctrl+A / Ctrl+D)
   │
   ▼
Phase 7 (FuzzyFilter lib)
   │
   ▼
Phase 8 (search prompt + plumbing)
   │
   ▼
Phase 9 (border flags)
   │
   ▼
Phase 10 (margin flags)
   │
   ▼
Phase 11 (height flag)
   │
   ▼
Phase 12 (tests + QA + CHANGELOG)
```

**Parallel work opportunities:**
- Phase 1 and Phase 2 are independent of each other and can be done in either order, but both must finish before Phase 3.
- Phase 5 and Phase 6 are independent once Phase 4 is done, but the design orders them sequentially for simplicity.
- Phase 9, 10, and 11 are all `FrameChrome`-dependent and ordered by complexity, but a single engineer can knock them out back-to-back quickly.

---

## Risk Register

| Risk | Mitigation |
|---|---|
| `nucleo-matcher` API changes between 0.3 and next patch | Pin exact version `"=0.3"` in Cargo.toml; add smoke test that exercises matcher surface. |
| Crossterm `/dev/tty` assumption fails in CI | Add `ensure_controlling_tty()` check and skip TTY-dependent integration tests when `CI=true`. |
| Esc exit code breaks existing scripts | Document in CHANGELOG; bump minor version of `tui-chrome-cli` to signal breaking change. |
| Legacy `--initial` callers confused | Keep `--initial` hidden for one release; print deprecation warning to stderr. |

---

## Exit Criteria

This plan is complete when:

1. All 12 phases are implemented.
2. Every validation checkpoint is satisfied.
3. The manual QA checklist from the technical design (§10.4) is fully executed and signed off.
4. `CHANGELOG.md` contains a "Breaking" section documenting the Esc exit-code change.
5. A PR is opened with a green CI run.
