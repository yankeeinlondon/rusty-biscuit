# CLI Reference

The `question` CLI exposes every `tui-chrome` component as a subcommand. This page documents the global flags and conventions that apply to **all** subcommands, so individual component docs don't need to repeat them.

## Global Flags

Every subcommand accepts the following flags:

| Flag | Description |
| :--- | :--- |
| `--output <raw\|json\|null>` | Serialisation format for the submitted value. |
| `--height <CELLS_OR_PERCENT>` | Render inline at an explicit height instead of fullscreen. |

### `--output`

Controls how the submitted value is written to `stdout`.

- **`raw`** (default for most commands) — emits the value as plain text followed by a newline (`\n`). For multi-value components (`choose-many`), each value is emitted on its own line.
- **`json`** — wraps the value in JSON. Strings are quoted and escaped; booleans are emitted as `true`/`false`; arrays (e.g. `choose-many` selections) are emitted as JSON arrays.
- **`null`** — like `raw`, but terminates the output with a NUL byte (`\0`) instead of a newline. Useful when consuming output from shell scripts where values may contain embedded newlines (particularly `text-area-input`).

### `--height`

By default, `question` takes over the full terminal (alternate screen). The `--height` flag runs the component inline, leaving the existing terminal content visible above the prompt.

- **Cell count** — e.g. `--height 10` renders the component in exactly 10 rows.
- **Percentage** — e.g. `--height 50%` queries the current terminal size and allocates that proportion of rows. Percentages clamp to a floor of 3 rows so the list always has room for a header plus one option.

When `--height` is omitted, the component runs fullscreen.

## Exit Codes

All subcommands return the same exit codes:

| Code | Meaning |
| :--- | :--- |
| `0` | The user submitted a value successfully. The value was written to `stdout` according to the `--output` flag. |
| `130` | The user pressed `Ctrl-C` (SIGINT). Nothing is written to `stdout`. |
| `1` | The user pressed `Esc` to abort, or a terminal I/O error occurred. Nothing is written to `stdout` on abort. |

These conventions make `question` safe to use in shell pipelines and `$(...)` command substitution:

```bash
# Safe command substitution — abort produces empty string
name=$(question text-input --label "Your name" || true)

# Conditional branching on exit code
if question boolean-switch --label "Continue?"; then
    echo "Proceeding..."
fi
```

## Subcommands

| Subcommand | Component | Description |
| :--- | :--- | :--- |
| `text-input` | [`TextInput`](components/text_input.md) | Single-line text entry. |
| `text-area-input` | [`TextAreaInput`](components/text_area_input.md) | Multi-line scrollable text editor. |
| `boolean-switch` | [`BooleanSwitch`](components/boolean_switch.md) | Binary ON/OFF toggle. |
| `choose-one` | [`ChooseOne`](components/choose_one.md) | Single-selection list. |
| `choose-many` | [`ChooseMany`](components/choose_many.md) | Multi-selection list. |
| `input-table` | [`InputTable`](components/input_table.md) | Grid of heterogeneous editable cells. |
