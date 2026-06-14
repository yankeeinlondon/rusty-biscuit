# Review 1 Implementation Plan

## Context

The `biscuit-tui` package area contains two workspace packages:

- `biscuit-tui` at `biscuit-tui/lib`
- `biscuit-tui-cli` at `biscuit-tui/cli`, binary `question`

The requested `.claude/skills/biscuit-tui/SKILL.md` file is not present, so this plan follows the repo instructions plus the local Rust/TUI/testing conventions. The root `justfile` does not cover `biscuit-tui`; use `biscuit-tui/justfile` or direct package-scoped `cargo` commands.

Review 1 has two actionable findings:

1. `choose-one` and `choose-many` do not enable fuzzy search in the CLI path, even though the library supports it via `ChoiceInput::with_filter_enabled(true)`.
2. Most CLI integration tests only assert that parsing reaches a no-TTY failure; they do not prove successful output for the new behavior.

## Phase 1: Wire CLI Filter Defaults

Goal: make the CLI meet the spec by enabling fuzzy search for every `choose-one` and `choose-many` option source.

Implementation steps:

- Add a shared `--no-filter` boolean to `ChooseChromeArgs` in `biscuit-tui/cli/src/commands/common_choose.rs` if keeping the compatibility escape hatch from the design is still desired. Default must be filtering enabled.
- In `biscuit-tui/cli/src/commands/choose_one.rs`, after `build_choice_input` applies sorting, call `.with_filter_enabled(true)` before returning the input. If `--no-filter` is added, use `.with_filter_enabled(!args.chrome.no_filter)` instead.
- In `biscuit-tui/cli/src/commands/choose_many.rs`, do the same after sorting and after preserving `SelectionMode::Multiple`.
- Add command unit tests in each file using the existing private `build_choice_input` / `run_with_writer` test modules:
  - default input has `filter_enabled == true` for positional options
  - default input has `filter_enabled == true` for legacy `--options`
  - `--no-filter` sets `filter_enabled == false` if the flag is added
  - `run_with_writer` receives a state whose `input.filter_enabled` matches the CLI flag

Completion criteria:

- Typing an alphanumeric key in a `question choose-*` session opens the filter path by default instead of using the old hotkey path.
- The opt-out, if added, is visible in help and covered by tests.
- No library behavior changes are needed; existing library tests already cover filter-on and filter-off event behavior.

## Phase 2: Add Deterministic CLI Green-Path Tests

Goal: replace no-TTY failure proxies with tests that prove successful output through the real CLI command mapping and writer formatting.

Preferred approach:

- Keep these as in-module command tests in `biscuit-tui/cli/src/commands/choose_one.rs` and `biscuit-tui/cli/src/commands/choose_many.rs`, using the existing `run_with_writer` seam.
- For each test, construct `ChooseOneArgs` / `ChooseManyArgs`, pass an in-memory writer, and use the `run_prompt` closure to inspect the fully built state and return the same value a real prompt would submit.
- This is more deterministic than `assert_cmd` because it avoids a controlling TTY requirement while still exercising arg-to-state mapping and output formatting.

Add or strengthen tests for:

- positional args success: `choose-one alpha beta` writes the active or selected value; `choose-many` writes newline-separated selected values
- delimiter output: `Apple:1` displays label `Apple`, has id/value `1`, and the command writes `1`
- selected defaults: `--selected` matches by value, including delimiter values
- stdin-source behavior at the source-resolution level already needs process stdin, so keep the existing `assert_cmd` no-TTY parsing smoke tests unless a test-only stdin injection seam is introduced; add a direct `resolve_option_strings` test only if the seam is refactored
- filter typing: use the closure to assert `state.input.filter_enabled` is true, and rely on library event tests for the actual key handling
- Esc and Ctrl+C: keep existing `run_with_writer` sentinel tests for `ABORTED_KIND` => `1` and `CANCELLED_KIND` => `130`
- Ctrl+A / Ctrl+D: add command-level tests that call `state.select_all()` / `state.deselect_all()` inside the closure and assert emitted values, while library tests continue to cover the actual key bindings

Optional higher-fidelity approach:

- Add a `#[cfg(test)]` helper in the CLI crate that builds a `TestBackend` terminal and feeds synthetic `crossterm::Event`s through `tui_chrome::drive_event_loop_with_chrome`.
- Use that helper only if the command-level state seam cannot cover a review case. Avoid making production APIs public solely for tests.

Completion criteria:

- Tests prove successful stdout for delimiter values, selected defaults, positional options, bulk selection, and cancellation exit mapping without requiring `QUESTION_INTERACTIVE_PTY=1`.
- The PTY tests may remain as optional smoke tests, but they are no longer the only green-path coverage for interactive behavior.
- The module header in `biscuit-tui/cli/tests/choose_cli.rs` is updated or trimmed if its "reaches event loop then fails" framing no longer describes the production-bar coverage.

## Phase 3: Documentation And Drift Updates

Goal: keep user-facing docs aligned with the behavior the review calls out.

Implementation steps:

- Update `biscuit-tui/cli/README.md` for `choose-one` and `choose-many` to mention:
  - positional args and stdin option sources
  - `--delimiter`
  - `--selected`
  - default fuzzy filtering, plus `--no-filter` if added
  - `Ctrl+A` / `Ctrl+D` for `choose-many`
  - Esc exit code `1` and Ctrl+C exit code `130`
- Update `biscuit-tui/cli/CHANGELOG.md` if adding `--no-filter`; the existing Unreleased entry already mentions inline fuzzy search and the exit-code split.
- No `docs/dependencies.md` update is needed unless the implementation adds or removes crates. The preferred plan adds no crates.

Completion criteria:

- Public behavior changed by Phase 1 and test coverage added by Phase 2 are reflected in CLI docs.
- No architecture or workflow drift requires `.claude/skills/` changes.

## Verification Commands

Run from repo root:

```bash
cd /Users/ken/.claudine/worktrees/rusty-biscuit/biscuit-tui
cargo metadata --no-deps --format-version 1 >/tmp/biscuit-tui-metadata.json
just -f biscuit-tui/justfile check
just -f biscuit-tui/justfile test
just -f biscuit-tui/justfile lint
```

Focused commands while iterating:

```bash
cargo test -p biscuit-tui-cli choose_one
cargo test -p biscuit-tui-cli choose_many
cargo test -p biscuit-tui-cli --test choose_cli
cargo test -p biscuit-tui typing_letter_opens_filter
cargo test -p biscuit-tui select_all
cargo clippy -p biscuit-tui -p biscuit-tui-cli --all-targets -- -D warnings
```

Optional local PTY smoke tests:

```bash
QUESTION_INTERACTIVE_PTY=1 cargo test -p biscuit-tui-cli --test choose_cli
```

## Phase Count

This plan has 3 serial phases. Phase 1 fixes the runtime behavior, Phase 2 raises test confidence, and Phase 3 handles docs/drift.
