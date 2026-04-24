---
ready: true
---

# Review 1

## Findings

1. High: `InputTable` does not expose the typed row-based API described in the spec/design. `InputTableState::new` takes a row count instead of caller-provided initial rows, `values()` / `StandaloneState::value()` return `Vec<Vec<String>>`, and cell extraction flattens booleans and multi-selects into strings. That loses both column ids and cell types before the CLI serializes the result, so library callers cannot retrieve `&[Row]` or typed row objects as designed. References: `biscuit-tui/lib/src/components/input_table/table.rs:52-70`, `biscuit-tui/lib/src/components/input_table/table.rs:149-156`, `biscuit-tui/lib/src/components/input_table/cell.rs:103-133`, `biscuit-tui/cli/src/commands/input_table.rs:378-409`.

2. High: configurable key bindings are effectively unimplemented. `KeyBindings` exists as shared core API, but components do not store or consume it; instead they hardcode event matches directly in each `handle_event`. That leaves the designed configurable navigation / submit / cancel model missing for `TextInput`, `BooleanSwitch`, `ChooseOne`, `ChooseMany`, and most of `InputTable`. References: `biscuit-tui/lib/src/core/keybindings.rs:1-69`, `biscuit-tui/lib/src/components/text_input.rs:207-220`, `biscuit-tui/lib/src/components/boolean_switch.rs:218-225`, `biscuit-tui/lib/src/components/choose_one.rs:230-270`, `biscuit-tui/lib/src/components/choose_many.rs:262-302`, `biscuit-tui/lib/src/components/input_table/table.rs:246-393`.

3. Medium: the choice components are still string-only end to end, so the generic `ChoiceInput<V>` / `ChoiceOption::map_value` contract is not actually usable through the widget API. `ChoiceInput<V>` is generic, but both `ChooseOneState` and `ChooseManyState` only accept `ChoiceInput<String>`. That means typed option values cannot flow through the public component states as designed, which is a real ergonomics regression for library consumers. References: `biscuit-tui/lib/src/components/choose.rs:91-125`, `biscuit-tui/lib/src/components/choose_one.rs:41-65`, `biscuit-tui/lib/src/components/choose_many.rs:47-70`.

4. Medium: the choice-list rendering misses some of the designed UX affordances. `ComponentTheme.focus_indicator` is defined but never rendered, and the list viewport does not paint `▲` / `▼` overflow indicators when content is scrolled. The current implementation only changes style and selection glyphs, so the hovered row and off-screen content are less explicit than the design called for. References: `biscuit-tui/lib/src/core/theme.rs:18-24`, `biscuit-tui/lib/src/components/choose_one.rs:359-403`, `biscuit-tui/lib/src/components/choose_many.rs:386-427`.

5. Medium: the CLI/testing surface is still short of the spec. `question` does not expose `--height` as a global flag; it is duplicated on each subcommand instead. I also found no CLI integration tests using the already-declared `assert_cmd` dependency, so exit codes and output modes are only covered by narrow unit tests rather than end-to-end command execution. References: `biscuit-tui/cli/src/main.rs:22-32`, `biscuit-tui/cli/src/commands/text_input.rs:16-39`, `biscuit-tui/cli/src/commands/choose_one.rs:22-57`, `biscuit-tui/cli/Cargo.toml:16-18`.

## Testing Notes

- I could not run `cargo test --manifest-path biscuit-tui/lib/Cargo.toml` because the workspace currently references a missing member at `tui/Cargo.toml` from the root `Cargo.toml`. That blocks direct verification from this worktree until the workspace manifest is repaired.

## Conclusion

This feature is not ready for production yet. The remaining gaps are still public-contract issues around `InputTable`, key binding configurability, and the typed choice API, plus missing end-to-end CLI coverage.
