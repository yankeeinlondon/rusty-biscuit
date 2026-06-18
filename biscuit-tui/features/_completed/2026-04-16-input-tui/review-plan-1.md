# Review 1 Follow-Up Plan

This plan closes every actionable item from [`review-1.md`](./review-1.md) for the `biscuit-tui` package area.

Scope guardrails:

- Keep the work inside `biscuit-tui/lib` and `biscuit-tui/cli`.
- Do not add v2-only features from the spec.
- Do not touch unrelated workspace layout or root manifests.
- Finish with passing tests and zero clippy warnings/errors for both crates.

Preferred implementation workflow:

- Use the local area `justfile` for broad validation.
- Use `cargo nextest run` while iterating on tests.
- Keep `cargo clippy ... -D warnings` as the lint gate.

## Review Items To Close

1. `InputTable` must preserve typed row data end-to-end instead of flattening to `Vec<Vec<String>>`.
2. Configurable `KeyBindings` must actually drive component event handling.
3. `ChoiceInput<V>` must remain usable generically through `ChooseOne` and `ChooseMany`.
4. Choice-list rendering must keep the intended focus and scroll affordances.
5. The CLI must expose the documented global flags and have real `assert_cmd` coverage.

## Phase 1: Restore And Lock Down Public Contracts

Goal: make the library surface match the spec and review recommendations before spending time on test expansion.

Implementation work:

- `InputTable`
  - Ensure `InputTableState::new(columns, initial_rows)` accepts caller-provided typed rows, not a row count.
  - Preserve column identifiers and typed cell values in the stored row model.
  - Make `state.value()` return the typed row slice required by the spec, not a string matrix.
  - Keep submit-time validation focused on the first failing cell and preserve the cell-level error.
- Key bindings
  - Thread `KeyBindings` through every input state that currently hardcodes submit/cancel/navigation keys.
  - Use `KeyBindings::matches` in `handle_event` so navigation, submit, cancel, and toggle behavior remain configurable.
  - Keep the current defaults, including vim-compatible navigation and `Ctrl-S` submit where the spec calls for it.
- Choice generics
  - Keep `ChoiceInput<V>` and `ChoiceOption::map_value`.
  - Make `ChooseOneState<V>` and `ChooseManyState<V>` carry typed values through to `value()` / `values()`.
  - Leave the CLI on `V = String`, but avoid collapsing the library API to strings.
- Choice UX
  - Render the focus prefix and overflow indicators consistently in `ChooseOne` and `ChooseMany`.
  - Preserve disabled-option styling and the selected-vs-hovered distinction.
- CLI flags
  - Keep `--height` and `--output` as top-level `question` flags.
  - Ensure subcommands do not reintroduce per-command copies of those global flags.

Exit criteria:

- The public API matches the spec for the five review items above.
- Any local refactor keeps the existing help text and component semantics intact.

## Phase 2: Add Targeted Unit Coverage

Goal: prove the contract changes at the component level before widening to subprocess coverage.

Tests to add or extend:

- `lib/src/components/input_table/table.rs`
  - Typed row preservation and row ordering.
  - First-failing-cell focus on submit-time validation.
  - `Ctrl-S` submit behavior and cancel handling through `KeyBindings`.
- `lib/src/components/choose_one.rs`
  - Generic typed values flow through `selected_value()`.
  - Required-submit validation and validation-error clearing.
  - Focus indicator and overflow rendering.
- `lib/src/components/choose_many.rs`
  - Typed `values()` flow through multiple selections.
  - `max_selections` keystroke rejection.
  - `required` / `min_selections` submit-time validation.
  - Focus indicator and overflow rendering.
- `lib/src/components/text_input.rs`, `text_area_input.rs`, `boolean_switch.rs`
  - Confirm they consume configurable key bindings instead of hardcoded matches.
  - Confirm their submit/cancel paths remain stable.

Test style:

- Prefer same-module unit tests with `#[cfg(test)]`.
- Keep assertions narrow and behavior-focused.
- Add regression tests in the same file as the code they guard.

Exit criteria:

- Every review-driven behavior change has at least one direct unit test.
- The new tests fail for the old behavior and pass for the new behavior.

## Phase 3: Strengthen CLI And Integration Coverage

Goal: cover the actual `question` binary and its output contract with `assert_cmd`.

Implementation work:

- Expand the existing `cli/tests/*.rs` coverage so each subcommand has an end-to-end subprocess test.
- Verify the global `--height` flag is accepted before the subcommand token.
- Verify `--output` is honored at the CLI boundary for raw, JSON, and NUL-separated modes where applicable.
- Keep exit-code checks explicit:
  - `0` for successful submission paths covered by unit seams.
  - `130` for cancel paths.
  - `1` for parse/runtime failures surfaced by subprocess execution.

Suggested coverage shape:

- One `assert_cmd` smoke test per subcommand that proves the real binary is wired correctly.
- One CLI help/flag-position test for the global flag surface.
- One exit-code test module that distinguishes cancel from runtime failure.

Notes:

- Do not add a test-only CLI flag just to force submission.
- Keep the existing unit-test seams for success-path serialization; use subprocess tests to cover the real binary contract and flag handling.

Exit criteria:

- The `question` binary is exercised in subprocess tests for every subcommand.
- The CLI contract is covered at the boundary, not only through internal helpers.

## Phase 4: Verify And Clean Up

Goal: finish with green tests and a clean lint pass for the package area.

Required verification:

- `cargo nextest run -p biscuit-tui -p biscuit-tui-cli`
- `cargo clippy -p biscuit-tui -p biscuit-tui-cli --all-targets --all-features -- -D warnings`

Area-level convenience checks:

- `just -f biscuit-tui/justfile test`
- `just -f biscuit-tui/justfile lint`

Final exit criteria:

- All tests pass for `biscuit-tui` and `biscuit-tui-cli`.
- `clippy` reports zero warnings/errors for the `biscuit-tui` package area.
- The code and tests cover every recommendation from `review-1.md`.
