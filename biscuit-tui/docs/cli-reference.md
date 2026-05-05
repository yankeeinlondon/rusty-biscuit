# CLI Reference

The `question` CLI exposes every `tui-chrome` component as a subcommand. This page documents the global flags and conventions that apply to **all** subcommands, so individual component docs don't need to repeat them.

## Global Flags

Every subcommand accepts the following flags:

| Flag | Description |
| :--- | :--- |
| `--output <raw\|json\|null>` | Serialisation format for the submitted value. |
| `--height <CELLS_OR_PERCENT>` | Render inline at an explicit height instead of fullscreen. |
| `--show-input-on-exit` | Preserve the rendered input on exit instead of clearing it (fzf default is to clear). |

### `--output`

Controls how the submitted value is written to `stdout`.

- **`raw`** (default for most commands) — emits the value as plain text followed by a newline (`\n`). For multi-value components (`choose-many`), each value is emitted on its own line.
- **`json`** — wraps the value in JSON. Strings are quoted and escaped; booleans are emitted as `true`/`false`; arrays (e.g. `choose-many` selections) are emitted as JSON arrays.
- **`null`** — like `raw`, but terminates the output with a NUL byte (`\0`) instead of a newline. Useful when consuming output from shell scripts where values may contain embedded newlines (particularly `text-area-input`).

### `--height`

By default, `question` takes over the full terminal (alternate screen). The `--height` flag runs the component inline, leaving the existing terminal content visible above the prompt.

Both forms are treated as a **maximum** — when the live terminal is smaller than the requested height, the inline viewport is clamped to the rows that are actually available so the prompt never overflows the screen.

- **Cell count** — e.g. `--height 10` renders the component in up to 10 rows. If the terminal is only 6 rows tall, the prompt occupies all 6.
- **Percentage** — e.g. `--height 50%` resolves the percentage against the current terminal height. Percentages clamp to a floor of 3 rows so the list always has room for a header plus one option, and they are **re-resolved on every terminal resize**: as the terminal grows or shrinks mid-prompt, the inline viewport tracks the requested fraction.

When `--height` is omitted, the component runs fullscreen.

### `--show-input-on-exit`

Controls what happens to the inline viewport once the prompt exits.

- **Omitted (default)** — fzf-style: the inline viewport is cleared on exit and the cursor is parked at the row where the prompt began, so the next shell prompt reuses the space the prompt occupied.
- **Set** — the final frame is left on screen and the cursor is moved to the row immediately below the chrome. Subsequent shell output (the CLI's result line, then the next prompt) follows the rendered border without overlapping it.

Has no effect on fullscreen prompts (i.e. when `--height` is omitted), since fullscreen prompts always revert to the original screen contents on exit.

## Exit Codes

| Code | Meaning |
| :--- | :--- |
| `0` | The user submitted a value successfully. For `choose-one`, pressing `Esc` restores the initial selection and also exits `0`. |
| `130` | The user pressed `Ctrl-C` (SIGINT). Nothing is written to `stdout`. |
| `1` | The user pressed `Esc` to abort (all components except `choose-one`), or a terminal I/O error occurred. Nothing is written to `stdout` on abort. |

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
| `completions` | — | Generate shell completion scripts (bash, zsh, fish, etc.). |

## Option Source Files (`--file`)

`choose-one` and `choose-many` can load their option list from a file via
`--file <PATH>`. The file extension picks the parser:

| Extension(s) | Format |
| :--- | :--- |
| `.json` | JSON array (top-level `[...]`) of strings or objects. |
| `.jsonl`, `.ndjson` | One JSON value per line; each line is one option. |
| `.yaml`, `.yml` | YAML sequence (top-level `- ...`) of strings or maps. |
| `.toml` | TOML table with an `options = [...]` key (see below). |
| `.csv` | First column is the option label/value (one row per option). |

For Markdown frontmatter sources, use `--md <PATH> <PROP>` instead — the
`--file` flag does not parse Markdown.

### TOML convention

Standard TOML cannot represent a top-level bare array (the spec requires
the document root to be a table), so a TOML options file **must** use the
`options = [...]` table form. Files structured with any other top-level
key (for example `colors = [...]`) will fail with
`option file must contain an array`.

Minimal TOML example:

```toml
options = ["Red", "Green", "Blue"]
```

With explicit labels and values:

```toml
[[options]]
label = "Red Delicious"
value = "apple"

[[options]]
label = "Cavendish"
value = "banana"
```

Per-option records may also carry `hotkey` and `disabled` fields, matching
the JSON/YAML object shape.
