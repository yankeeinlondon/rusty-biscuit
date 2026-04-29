---
phases: 9
created: 2026-04-29
start_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase1: []
source_files_during_phase_2:
  - biscuit-tui/lib/src/components/choose.rs
  - biscuit-tui/lib/src/components/mod.rs
  - biscuit-tui/lib/src/core/frame.rs
  - biscuit-tui/lib/src/core/mod.rs
  - biscuit-tui/lib/src/core/sort.rs
  - biscuit-tui/lib/src/core/terminal_style.rs
  - biscuit-tui/lib/src/lib.rs
  - biscuit-tui/lib/src/prelude.rs
  - biscuit-tui/cli/src/commands/common_choose.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2:
  - .claude/skills/biscuit-tui/SKILL.md
source_files_during_phase_3:
  - biscuit-tui/cli/src/commands/common_choose.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase3:
  - .opencode/skill/biscuit-tui/SKILL.md
source_files_during_phase_4:
  - biscuit-tui/lib/src/components/choice_layout.rs
  - biscuit-tui/lib/src/components/choice_render.rs
  - biscuit-tui/lib/src/components/choose_one.rs
  - biscuit-tui/lib/src/components/choose_many.rs
  - biscuit-tui/lib/src/components/mod.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase4: []
source_files_during_phase_5:
  - biscuit-tui/lib/src/components/choose_one.rs
  - biscuit-tui/lib/src/components/choose_many.rs
  - biscuit-tui/lib/src/components/choice_render.rs
  - biscuit-tui/cli/src/commands/choose_one.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase5:
  - .opencode/skill/biscuit-tui/SKILL.md
source_files_during_phase_6:
  - biscuit-tui/lib/src/components/choice_layout.rs
  - biscuit-tui/lib/src/components/choice_render.rs
  - biscuit-tui/lib/src/components/choose_one.rs
  - biscuit-tui/lib/src/components/choose_many.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase6: []
source_files_during_phase_7:
  - biscuit-tui/cli/src/option_sources.rs
  - biscuit-tui/cli/src/choice_normalize.rs
  - biscuit-tui/cli/src/commands/common_choose.rs
  - biscuit-tui/cli/src/commands/choose_one.rs
  - biscuit-tui/cli/src/commands/choose_many.rs
  - biscuit-tui/cli/src/main.rs
  - biscuit-tui/cli/Cargo.toml
  - biscuit-tui/cli/tests/choose_cli.rs
  - biscuit-tui/cli/tests/choose_one_output.rs
  - biscuit-tui/cli/tests/choose_many_output.rs
  - biscuit-tui/cli/tests/exit_codes.rs
docs_updated_during_phase_7: []
docs_created_during_phase_7: []
skills_files_updated_during_phase7:
  - .opencode/skill/biscuit-tui/SKILL.md
source_files_during_phase_8:
  - biscuit-tui/cli/Cargo.toml
  - biscuit-tui/cli/src/main.rs
  - biscuit-tui/cli/tests/help_contract.rs
  - biscuit-tui/cli/tests/completions.rs
docs_updated_during_phase_8: []
docs_created_during_phase_8: []
skills_files_updated_during_phase8:
  - .opencode/skill/biscuit-tui/SKILL.md
packages:
  - tui-chrome
  - tui-chrome-cli
---

# Choose Components Improvements Execution Plan

Sources:

- Functional specification: `biscuit-tui/features/2026-04-28-choose-one-improvements/spec.md`
- Technical design: `biscuit-tui/features/2026-04-28-choose-one-improvements/tech-design.md`

## Phase 1: Baseline And Scope Confirmation

Goal: confirm the current implementation shape, command surface, and tests before modifying shared choice behavior.

1. Run `cargo metadata --no-deps --format-version 1` and confirm the package names for `biscuit-tui/lib` and `biscuit-tui/cli`.
2. Inspect the current public API and tests in:
   - `biscuit-tui/lib/src/components/choose.rs`
   - `biscuit-tui/lib/src/components/choose_one.rs`
   - `biscuit-tui/lib/src/components/choose_many.rs`
   - `biscuit-tui/lib/src/core/frame.rs`
   - `biscuit-tui/lib/src/core/sort.rs`
   - `biscuit-tui/cli/src/commands/common_choose.rs`
   - `biscuit-tui/cli/src/commands/choose_one.rs`
   - `biscuit-tui/cli/src/commands/choose_many.rs`
3. Capture current behavior with focused tests or notes for:
   - `ChooseOne` Enter, Space, Esc, and Ctrl-C outcomes.
   - `ChooseMany` Enter and Space outcomes.
   - Current `FrameChrome` render area shrinking.
   - Current CLI source flags and sort vocabulary.
4. Identify existing docs and README sections that mention the changing behavior.

Validation checkpoint:

- `cargo test -p tui-chrome -p tui-chrome-cli` either passes or failures are recorded as pre-existing.
- A short implementation note lists any existing tests that will need expectation updates.

Parallelizable:

- API/test inventory and docs inventory can be done in parallel.

### Phase 1 Implementation Notes

**Package names confirmed:**
- Library: `tui-chrome` (manifest at `biscuit-tui/lib/Cargo.toml`)
- CLI: `tui-chrome-cli` (manifest at `biscuit-tui/cli/Cargo.toml`)

**Test baseline:**
All tests pass for both packages (`cargo test -p tui-chrome -p tui-chrome-cli` — zero failures). No pre-existing failures to record.

**Current `ChooseOne` behavior:**
- `Enter`: Auto-selects the hovered option if none explicitly selected (fallback submit), then returns `Submitted`.
- `Space`: Selects the hovered option, returns `Consumed`.
- `Esc`: Returns `Cancelled` (standalone runner maps this to exit code `1`).
- `Ctrl-C`: Returns `Cancelled` (standalone runner maps this to exit code `130`).

**Current `ChooseMany` behavior:**
- `Enter`: If zero items selected, auto-selects the hovered option (fallback submit), then returns `Submitted`.
- `Space`: Toggles the hovered option, returns `Consumed`.

**Current `FrameChrome` render area shrinking:**
Margin is subtracted first, then border claims its single-cell perimeter. No interior padding exists. The inner widget receives the remaining rectangle.

**Current CLI source flags:**
- Positional arguments (preferred)
- `--options <CSV>` (legacy)
- `--options-from-file <PATH>` (markdown list)
- `--options-from-dictionary <PATH>` (YAML/JSON mapping)
- Stdin fallback when not a TTY and no other source provided

**Current sort vocabulary:**
Library enum: `SortOrder { Natural, Reverse, Asc, Desc }`
CLI arg: `SortOrderArg { Natural, Reverse, Asc, Desc }`
The spec uses `Inverse` instead of `Reverse`; the CLI will need to expose `inverse` and keep `reverse` as a hidden compatibility alias.

**Existing tests that will need expectation updates in later phases:**
- `lib/src/components/choose_one.rs:981` — `esc_cancels` expects `EventOutcome::Cancelled`; must be updated to `Submitted` after Phase 5.
- `cli/src/commands/choose_one.rs:544` — `run_returns_1_without_output_on_esc` expects status `1`; must be updated to `0` after Phase 5.
- `lib/src/components/choose_many.rs:956` — `fallback_submit_selects_active_when_none_chosen` expects Enter to auto-select hover; must be updated after Phase 5.
- Render tests in both `choose_one.rs` and `choose_many.rs` that assert indicator glyphs (`●`/`○` vs `☑`/`☐`) will need updates after Phase 5 for Nerd Font / radio vs checkbox indicators.
- The current `choose_one.rs` `render_draws_indicator_and_label_per_row` test at line 1046 asserts `●` selected / `○` unselected, which is already the correct fallback radio glyphs, but will need updates if Nerd Font detection changes the default.

**Docs and README sections mentioning behavior that will change:**
- `docs/components/choose_one.md` lines 43-55 (key bindings, auto-selection), lines 139-143 (exit code table claiming `Esc = 1`)
- `docs/components/choose_many.md` lines 56-59 (auto-selection on submit), lines 142-148 (exit code table)
- `docs/cli-reference.md` lines 35-39 (exit code table claiming `Esc = 1`)
- `cli/README.md` lines 226-231 (exit codes), lines 98-109 (choose-one description), lines 134-138 (choose-many description)

## Phase 2: Shared Public Types And Low-Level Primitives

Goal: add foundational types without changing visible behavior except where defaults are explicitly required.

Dependencies: Phase 1.

1. Add shared choice vocabulary in `biscuit-tui/lib/src/components/choose.rs`:
   - `Orientation::{Vertical, Horizontal}` with `Vertical` as default.
   - `HotkeySpec::{Ctrl(char), Alt(char)}`.
   - `HotkeyDisplayMode::{Hidden, CtrlHeld, AltHeld}` with `Hidden` as default.
   - `ActiveChoiceColor::{Grey, Green, Yellow, Red}` if not better housed in theme.
2. Add `hotkey: Option<HotkeySpec>` and `ChoiceOption::with_hotkey`.
3. Add `orientation` and explicit sort field/builders to `ChoiceInput`.
4. Reconcile sort vocabulary:
   - Prefer `OptionSort::{Natural, Inverse, Asc, Desc}`.
   - If keeping existing `SortOrder`, expose CLI `inverse` and keep `reverse` only as a hidden compatibility alias.
5. Add `Padding` to `biscuit-tui/lib/src/core/frame.rs`, mirroring `Margin`.
6. Set `Padding::default()` and `FrameChromeConfig::default().padding` to `Padding::uniform(1)`.
7. Add `core::terminal_style` or equivalent isolated helper with:
   - `TerminalStyle { background, nerd_font }`
   - `TerminalBackground::{Dark, Light, Unknown}`
   - conservative Nerd Font detection from environment.

Validation checkpoint:

- `cargo test -p tui-chrome --lib` compiles.
- Public builders can be used in a minimal unit test without rendering.
- `FrameChromeConfig::default().padding == Padding::uniform(1)`.

Parallelizable:

- Terminal style helper and sort vocabulary can be implemented independently after the public type names are chosen.

## Phase 3: FrameChrome Padding

Goal: implement library-level interior padding and expose it through the CLI.

Dependencies: Phase 2 `Padding`.

1. Update `FrameChrome` rendering order:
   - Apply outer margin.
   - Draw border when configured.
   - Apply padding to the border interior.
   - Render the inner widget in the padded area.
2. Ensure padding applies even when `BorderStyle::None`.
3. Update `FrameChromeConfig::is_empty()` semantics so default padding does not accidentally force wrapping where callers expect a bare widget, or document and test the intended behavior.
4. Add CLI padding flags to the shared chrome args:
   - `--padding <N>` / `-p <N>`
   - `--pt <N>`, `--pb <N>`, `--pl <N>`, `--pr <N>`
5. Merge per-side padding overrides after the uniform value, matching margin precedence.

Validation checkpoint:

- Unit tests prove render area shrinking for border + padding, no-border + padding, and per-side overrides.
- CLI arg tests prove `--padding`, `--pt`, `--pb`, `--pl`, and `--pr` populate `FrameChromeConfig` correctly.

Parallelizable:

- CLI flag parsing can proceed while render tests are being written, once `Padding` exists.

## Phase 4: Shared Choice Layout And Rendering Refactor

Goal: introduce common layout/render helpers while preserving current vertical behavior as much as possible.

Dependencies: Phase 2.

1. Create `biscuit-tui/lib/src/components/choice_layout.rs`.
2. Implement `ChoiceLayout`, `ChoiceItemRect`, and row range tracking.
3. Support vertical layout as one item per row, preserving current scrolling and active-row behavior.
4. Create `biscuit-tui/lib/src/components/choice_render.rs`.
5. Move common choice item rendering into a shared context that accepts:
   - orientation
   - active state
   - selected state
   - disabled state
   - hotkey display state
   - terminal style
6. Render active background only over the visible item width plus one blank cell.
7. Preserve the triangular active pointer in vertical mode only.
8. Export new modules through `components/mod.rs` only as needed.

Validation checkpoint:

- Existing vertical render tests still pass after expectation updates for shared renderer changes.
- New buffer tests cover active background span width and pointer visibility in vertical mode.

Parallelizable:

- Layout helper and render helper can be built in parallel if their shared data structs are agreed first.

## Phase 5: ChoiceOne And ChooseMany Semantics

Goal: implement the required state transition changes before adding horizontal navigation and richer CLI parsing.

Dependencies: Phase 4.

1. Update `ChooseOneState`:
   - Track `initial_selected`.
   - Keep `active`/`hover` distinct from selected state.
   - Add `hotkeys`, `hotkey_display`, and layout cache fields if not already added in Phase 4.
2. Set `initial_selected` from `with_initial_selection` and `with_initial_value`.
3. Implement `ChooseOne::handle_event` ordering:
   - Ctrl-C returns cancellation for exit code `130`.
   - Filter editing keys run before selection shortcuts when filter mode is active.
   - Ctrl/Alt hotkey selects and submits.
   - Enter selects active enabled item and submits.
   - Space selects active enabled item without submitting.
   - Navigation moves active item only.
   - Esc restores `initial_selected` and returns `Submitted`.
4. Update standalone or CLI mapping only if needed so `ChooseOne` Esc exits `0` with the restored/default value while Ctrl-C exits `130`.
5. Update `ChooseMany`:
   - Enter submits selected set exactly as-is.
   - Space remains the exclusive row toggle.
   - Esc cancellation behavior remains unchanged.
6. Apply glyph policy:
   - `ChooseOne`: Nerd Font `\u{f043e}` / `\u{f4aa}`, fallback `●` / `○`.
   - `ChooseMany`: Nerd Font `\u{f14a}` / `\u{f0131}`, fallback `☑` / `☐`.

Validation checkpoint:

- Unit tests cover all specified Enter, Space, Esc, Ctrl-C, and hotkey outcomes.
- Tests prove `ChooseOne` Esc after navigation restores the original selection.
- Tests prove `ChooseOne` Esc after Space restores the original selection.
- Tests prove `ChooseMany` Enter does not toggle or auto-select the active row.
- Glyph tests cover Nerd Font and fallback terminal styles.

Parallelizable:

- `ChooseOne` semantic tests and `ChooseMany` semantic tests can be written independently after Phase 4.

## Phase 6: Horizontal Layout And Navigation

Goal: add horizontal orientation for both choice components using the shared layout cache.

Dependencies: Phases 4 and 5.

1. Implement horizontal item measurement with `unicode_width::UnicodeWidthStr`.
2. Reserve width for:
   - radio or checkbox indicator
   - separator space
   - label
   - one active-background trailing blank
   - hotkey badge when visible
3. Pack options left-to-right and wrap to new rows when the next item would exceed `area.width`.
4. Rebuild layout cache during render.
5. Implement horizontal navigation:
   - Left/Right move to previous/next option in sequential order.
   - Up/Down select the closest column in the adjacent row.
   - If the adjacent row is shorter, choose its last item.
   - If cache is missing or stale, fall back to sequential movement.
6. Ensure horizontal mode removes the triangular pointer and uses active background only.

Validation checkpoint:

- Layout unit tests prove wrapping by measured width and stable option order.
- Navigation tests cover Left, Right, Up, Down, closest-column selection, short-row fallback, and stale-cache fallback.
- Render buffer tests cover horizontal wrapping and absent pointer.

Parallelizable:

- Navigation tests can be prepared from the layout contract while rendering work continues.

## Phase 7: Hotkeys, Source Parsing, And Normalization

Goal: complete the richer CLI option model and hotkey assignment rules.

Dependencies: Phase 2 for `HotkeySpec`; Phase 5 for hotkey behavior.

1. Add or refactor CLI modules as needed:
   - `commands/common_choose.rs` for clap args and completion metadata.
   - `option_sources.rs` for source parsing.
   - `choice_normalize.rs` for hotkeys and label/value transforms.
2. Replace source vocabulary with mutually exclusive sources:
   - positional args
   - `--csv <TEXT>`
   - `--list <TEXT>`
   - `--rows <TEXT>`
   - `--file <PATH>`
   - `--md <PATH> <PROP>`
   - stdin fallback
3. Decide whether to keep `--options` as a hidden compatibility alias for `--csv`; update tests and docs accordingly.
4. Parse `--file` as JSON, JSONL, NDJSON, YAML, CSV, or TOML.
5. Require file and Markdown frontmatter sources to resolve to arrays; return typed CLI errors on invalid shapes.
6. Parse hotkey prefixes:
   - `[CTRL+X]`
   - `[ALT+X]`
   - `[OPT+X]` as alias for Alt.
7. Normalize alpha hotkey characters case-insensitively.
8. Reject duplicate CLI hotkeys with a clear error.
9. Add `--numeric-hot-keys`:
   - first 10 options get Ctrl+1 through Ctrl+9, then Ctrl+0.
   - next 10 options get Alt+1 through Alt+9, then Alt+0.
   - later options receive no numeric hotkey.
   - explicit hotkeys are not overwritten.
10. Add label/value handling:
   - split `Label::Value` on `::`.
   - support `--label <convention>` and `--value <convention>`.
   - conventions: `camel-case`, `pascal-case`, `kebab-case`, `snake-case`, `title-case`, `caps`, `lowercase`.
   - explicit `::` sides take precedence over convention-generated sides.
11. Apply sort after normalization and before `ChoiceInput<String>` construction.
12. Ensure `ChoiceInput` receives clean IDs, labels, values, hotkeys, orientation, and sort.

Validation checkpoint:

- CLI unit tests cover every source form, source conflict detection, stdin fallback, non-array errors, Markdown frontmatter property errors, duplicate hotkeys, numeric hotkey assignment, convention transforms, `::` precedence, and sort order.
- Library duplicate hotkeys use first-wins behavior and do not panic.

Parallelizable:

- Source parsing, convention transforms, and numeric hotkey assignment are independently testable once the normalized option record type exists.

## Phase 8: Completions And CLI Surface Completion

Goal: expose the finished command surface through shell completions.

Dependencies: Phase 7.

1. Add `question completions <shell>` using `clap_complete`.
2. Ensure completions include all top-level subcommands.
3. Ensure generated completions include:
   - `--sort` values: `natural`, `inverse`, `asc`, `desc`.
   - convention values.
   - source flags: `--csv`, `--list`, `--rows`, `--file`, `--md`.
   - padding flags.
   - orientation flag if one is exposed by the CLI.
4. Add hotkey prefix suggestions for `[CTRL+`, `[ALT+`, and `[OPT+` when practical.
5. If prefix-aware completions are too large for this feature, ship static clap completions and record prefix-aware completion as a follow-up.

Validation checkpoint:

- `question completions bash`, `zsh`, and `fish` emit non-empty completion scripts in tests or local smoke checks.
- Completion tests assert the generated scripts mention new sort, convention, source, and padding flags.

Parallelizable:

- Completion command plumbing can start after the clap arg surface from Phase 7 is stable.

## Phase 9: Documentation, Migration, And Final Verification

Goal: align docs with the breaking behavior changes and run package-level validation.

Dependencies: Phases 3 through 8.

1. Update component docs:
   - `biscuit-tui/docs/components/choose_one.md`
   - `biscuit-tui/docs/components/choose_many.md`
   - `biscuit-tui/docs/components/frame_chrome.md`
   - `biscuit-tui/docs/components/index.md`
2. Update CLI docs:
   - `biscuit-tui/docs/cli-reference.md`
   - `biscuit-tui/cli/README.md`
3. Update library docs:
   - `biscuit-tui/lib/README.md`
   - rustdoc examples affected by `Orientation`, `HotkeySpec`, `OptionSort`, padding, and changed key behavior.
4. Update dependency docs if any crates were added or removed:
   - root `docs/dependencies.md` if repo convention requires it.
   - area-specific docs under `biscuit-tui/docs/` if present.
5. Update `.claude/skills/` only if the component architecture or workflow guidance changed materially.
6. Run formatting and linting:
   - `cargo fmt --all`
   - `cargo clippy -p tui-chrome -p tui-chrome-cli --all-targets -- -D warnings`
7. Run focused tests:
   - `cargo test -p tui-chrome -p tui-chrome-cli`
8. Run area validation if available:
   - `just --justfile biscuit-tui/justfile test`
   - `just --justfile biscuit-tui/justfile lint`
9. Smoke test representative CLI commands:
   - `question choose-one Apple Banana Cherry`
   - `question choose-one --csv "Apple,Banana,Cherry" --sort inverse`
   - `question choose-one --rows $'Red::apple\nGreen::pear' --numeric-hot-keys`
   - `question choose-many --padding 2 --pt 0 Red Green Blue`
   - `question completions zsh`

Validation checkpoint:

- Formatting, clippy, and tests pass for `tui-chrome` and `tui-chrome-cli`.
- Docs no longer claim `ChooseOne` Esc exits `1`.
- Docs no longer present `--options` as the primary option source unless it is retained as a compatibility alias.
- Public behavior changes are reflected in examples and exit code tables.

Parallelizable:

- Docs can be updated in parallel with final test hardening after the public API and CLI surface are stable.

## Critical Dependency Order

1. Shared public types must land before state, rendering, CLI parsing, or docs can be finalized.
2. Shared layout/render refactor should land before semantic changes to avoid duplicating behavior in `ChooseOne` and `ChooseMany`.
3. `ChooseOne` and `ChooseMany` event semantics should land before horizontal navigation so tests can isolate behavior changes from geometry changes.
4. CLI source normalization depends on the final hotkey and sort vocabulary.
5. Completions depend on the final clap argument surface.
6. Documentation should be finalized only after behavior and compatibility decisions are settled.

## Risk Controls

- Keep filesystem and parsing errors in `tui-chrome-cli`; do not introduce CLI dependencies into `tui-chrome`.
- Keep terminal capability detection conservative; unknown terminals should use standard Unicode glyphs and dark-background-safe colors.
- Avoid full-screen snapshot tests where narrow buffer assertions cover the behavior.
- Preserve first-wins library hotkey behavior to keep embedded apps from panicking.
- Treat `ChooseOne` Esc behavior as a breaking change and verify every exit-code table and CLI test that mentions Esc.
