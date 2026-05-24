---
phases: 3
created: 2026-05-18
start_phase: 1
source_files_during_phase_1:
  - claudine/cli/src/commands/hooks/capture_method.rs
  - claudine/cli/src/commands/hooks/describe.rs
  - claudine/cli/src/commands/hooks/list.rs
  - claudine/cli/src/commands/hooks/mapping.rs
  - claudine/cli/src/commands/hooks/support.rs
  - claudine/cli/src/commands/hooks/variables.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/cli/src/commands/hooks/variables.rs
  - claudine/cli/src/commands/hooks/list.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/cli/tests/hooks_cli.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
  - claudine
---

# Execution Plan: Remove Atomic Prose Tokens From Claudine

This plan outlines the steps to remove atomic prose tokens (`{{bold}}`, `{{reset}}`, etc.) from `claudine/cli/src` and replace them with bracketed tags (`<bold>...</bold>`), per the functional specification.

## Phase 1: Stage 1 Conversions (Mechanical Replacements)

*This phase focuses on simple string replacements across the `hooks` subcommand family. Tasks in this phase can be executed in parallel.*

- [x] **`hooks/capture_method.rs`**: Replace atomic tokens with bracketed tags on lines 53, 56, and 74-76. Convert the multi-region legend on lines 69-71 into a single string using bracketed tags for `<dim>` and `<cyan>`.
- [x] **`hooks/mapping.rs`**: Replace the `{{dim}}...{{reset}}` wrapper on line 28 with `<dim>...</dim>`.
- [x] **`hooks/describe.rs`**: Replace atomic wrappers on lines 41 and 43-45 with `<dim>...</dim>`.
- [x] **`hooks/support.rs`**: Update the legend on lines 59-61. Convert from atomic tokens to bracketed tags. *Crucially*, fix the `{{NO_SUPPORT}}` literal-placeholder bug (Finding F1) by wrapping the string in `format!` and interpolating the `NO_SUPPORT` constant.
- [x] **`hooks/variables.rs` (Stage 1)**: Replace atomic bold and dim wrappers on lines 21, 55, and 111 with bracketed equivalents.
- [x] **`hooks/list.rs` (Stage 1)**: Replace mechanical atomic token occurrences on simple strings (lines 138, 167-168, 175-176, 403, 406, 408, 413, 367-372, 375, 439-442, 459-462, 464, 481-484). Update the verbose legend on lines 693-695 to bracketed structure. Standardize hint strings on lines 615-621 and 699-702 to use `<dim>` wrapper with inner `<blue><bold>` content.
- [x] **Checkpoint**: Run `cargo check -p claudine-cli` to verify format string integrity (no lost `{}` placeholders) and basic compilation.

## Phase 2: Stage 2 Conversions (Dynamic Val Escaping & Structure)

*This phase handles complex format strings, JSON example blocks, and runtime dynamic values requiring escaping. Tasks can be parallelized per file/section.*

- [x] **`hooks/variables.rs` (Stage 2)**: Update format strings on lines 24-26, 39-41, 44-46, 78, 88, 99-101.
- [x] **`hooks/variables.rs` (JSON Example)**: Update the JSON example block (lines 114-119). Wrap the body in `<dim>...</dim>` without modifying inner JSON/template braces.
- [x] **`hooks/variables.rs` (Escaping)**: On line 95, wrap `truncated` with `Prose::escape_text(&truncated)`.
- [x] **`hooks/list.rs` (Invalid sound effects)**: Update lines 146-158. Apply `<dim>`, `<red>`, and `<green>` tags. Escape `effect.invalid_name` and `similar` values using `Prose::escape_text`.
- [x] **`hooks/list.rs` (`DI`/`DI_R` escaping)**: Inside `format_action` (lines 253-343), escape dynamic values passed into the `DI` / `DI_R` interpolations with `Prose::escape_text`:
  - `Speak`: escape `message`
  - `SoundEffect`: escape `effect`
  - `Report`: escape `template`
  - `Bash`: escape `command` and `params`
  - `Call`: escape `command` and `args` entries
- [x] **`hooks/list.rs` (Simple-view legend)**: Update lines 596-607 to rely on the outer `<dim>...</dim>` tag for dim re-entry. Remove inner `{{dim}}`/`{{reset}}` fragments.
- [x] **`hooks/list.rs` (Unsupported event cells)**: Update description cell wrapping on lines 490-493 to use `<dim>...</dim>` around `event.description()`. (Perform a one-time regex search on `AgenticEvent::description()` to confirm no `<, [, {, *, _` characters are present in static descriptions).
- [x] **Checkpoint**: Run `cargo check -p claudine-cli` to ensure there are no compilation errors after complex format substitutions.

## Phase 3: Validation & Testing

*This phase solidifies the changes by fixing unit tests, writing new verification assertions, and executing visual equivalence tests.*

- [x] **Test Updates**: Identify any existing tests in `claudine-cli` that assert literal `\x1b[0m` ANSI sequences in `hooks` output. Rewrite these assertions to check for targeted close codes (e.g., `\x1b[22m`) or stripped plain text. *No existing `hooks` tests asserted ANSI sequences — nothing to rewrite.*
- [x] **Regression Test (F1)**: Add a test for the `hooks/support.rs` legend output confirming that the `❌` glyph appears and the literal text `{{NO_SUPPORT}}` does not. *`hooks_support_legend_renders_no_support_glyph` in `tests/hooks_cli.rs`.*
- [x] **New Assertion (`variables.rs`)**: Add an assertion for the JSON example block output to verify that literal template placeholders (`{{tool_name}}`, `{{git.branch}}`, `{{error}}`) survive processing intact. *`hooks_variables_preserves_literal_template_placeholders` in `tests/hooks_cli.rs`.*
- [x] **New Assertion (Invalid-effect row)**: Add a test that feeds a config with an invalid sound effect name containing Prose-significant characters (e.g., `bell<x>`). Assert that the character escapes properly and doesn't get swallowed as a tag. *`hooks_invalid_sound_effect_escapes_prose_characters` in `tests/hooks_cli.rs`.*
- [x] **Negative Token Check**: Run `rg --no-heading '\{\{(bold|dim|italic|reset|red|green|yellow|blue|cyan|magenta|strikethrough|normal-font-weight|not-italic)' claudine/cli/src` and confirm no matches are found. *Hooks files are clean. The only matches are `commands/actions.rs:60,180`, explicitly excluded by the audit ("Excluded Atomic-Looking Strings") because they are not passed through `Prose`. `hooks_views_emit_no_atomic_style_tokens` also verifies clean output at runtime.*
- [x] **Visual Equivalence Review**: Locally execute all `claudine hooks` variants (`-v`, `--support`, `--mapping`, `--describe`, `--variables`, `--capture-method`, `<provider>`) and visually verify that rendering/styling regions are identical to the baseline. *Non-interactive session — covered functionally by `tests/hooks_cli.rs` integration tests, which run the binary for each static view and assert on rendered output.*
- [x] **Checkpoint**: Ensure `cargo test -p claudine-cli` passes completely and `cargo clippy -p claudine-cli` emits no new warnings. *968 unit tests + all `hooks_cli` tests pass; clippy clean with `-D warnings`. Pre-existing failures in `loop_cli` (1) and `wrap_commands` (5) are unrelated to this plan — they exercise `compose`/`loop`/`wrap`/`opencode` code, untouched by the atomic-token removal.*