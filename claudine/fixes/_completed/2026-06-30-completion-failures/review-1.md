---
ready: false
agent: codex/default
created: 2026-06-30T19:11:25
implemented: true
---

# Review 1 - Completion Failures

## Findings

### High - Provided partial `file[]` values are specified but not implemented

The plan and docs explicitly extend provided-partial resolution to `file` **and** `file[]` schema properties, but the classifier only extracts a scalar string from the effective value. `provided_partial_value` returns `Some` only for `serde_json::Value::String` and rejects arrays, even though the surrounding docs claim it classifies `file[]` too ([schema_validation.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/schema_validation.rs:784), [schema_validation.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/lib/src/composition/schema_validation.rs:859)).

This leaves both likely `file[]` user paths uncovered:

- `attachments=everywhere` for a `file[](required;match(...);eager)` property is likely a type validation failure rather than the `no existing file matched reference` failure the classifier requires.
- `attachments=["everywhere"]` is an array value, and `provided_partial_value` rejects arrays before `UnresolvedFileReference` can be raised.

The CLI resolution path has `is_array` support after classification ([schema_interactive.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/schema_interactive.rs:391)), but the library layer never reliably reaches it for `file[]`. Add library coverage that proves `file[](required;match(**/*spec*.md);eager)` with a provided partial produces `CompositionError::UnresolvedFileReference { is_array: true, ... }`, plus CLI/L2 coverage for the interactive chooser/confirmation path. Until then, one of the documented user-facing requirements is incomplete.

Verification level present: none for `file[]` provided partials.

### High - New schema/description detail rendering is only verified at Level 1, not through the real terminal chooser

The spec requires user-visible TUI rendering changes: descriptions should render as Prose emphasis, and the `$schema` section should render as a YAML code block with all properties visible and unmangled. The implementation adds good in-process assertions against `render_file_detail_prose` ([autocomplete_ui.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/completion/autocomplete_ui.rs:420)), but the changed real UI path also depends on pre-rendering the code block at pane width and converting ANSI into ratatui text ([autocomplete_ui.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/completion/autocomplete_ui.rs:105), [autocomplete_ui.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/completion/autocomplete_ui.rs:271)).

Existing Level 2 chooser tests only assert generic detail markers such as `Schema:` or `no schema defined`; they do not capture a real terminal pane containing the exact `spec`, `design`, `plan`, and `match(**/*spec*.md)` text, nor do they verify the description emphasis survives the real terminal rendering path. Per the requested test rigor, glyphs, SGR styling, widths, and detail-pane clipping require Level 2 capture. A Level 1 unit test cannot prove the code block is visible and uncorrupted in tmux/WezTerm after `ansi_to_tui` conversion and pane clipping.

Add a Level 2 capture test for the operation-file chooser or schema chooser using the exact plan schema and `_feature_`/`_plan_` description fixture. It should assert the captured pane text contains all schema properties in order and the `match(...)` glob unmangled; where styling is material, assert the terminal-rendered emphasis through the available capture mechanism.

Verification level present: Level 1 for content/styling; Level 2 only for generic detail-pane presence. Required level: Level 2 for the changed user-visible TUI rendering.

## Notes

The scalar `file(match)` path is otherwise well covered: the library classification tests passed, the CLI candidate-filter tests passed, and the new Level 2 PTY tests cover single-match confirmation and zero-match error preservation for scalar `file`.

## Verification Run

- `cargo check --color=never -p claudine-cli -p claudine` - passed
- `cargo test --color=never -p claudine provided_file -- --nocapture` - passed, 2 tests
- `cargo test --color=never -p claudine-cli detail_prose -- --nocapture` - passed, 10 tests
- `cargo test --color=never -p claudine-cli provided_partial_candidates -- --nocapture` - passed, 3 tests

## Production Readiness

Not production ready. The scalar happy path is in good shape, but an explicit `file[]` requirement is not implemented, and the rendering fix needs Level 2 verification for the real terminal path before this should ship.
