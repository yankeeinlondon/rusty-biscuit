# question

A CLI for the `tui-chrome` library. Exposes each input component as a subcommand, making them directly usable in shell scripts and pipelines.

## Installation

```bash
cargo install --path .
```

The binary is named `question`.

## Global Flags

- `--output {raw|json|null}` — controls output serialization (default: `raw`)
  - `raw`: plain text, one value per line (default for scalars and multi-selects)
  - `json`: JSON encoding (strings are quoted, arrays for multi-value outputs)
  - `null`: NUL-byte separated (useful when values may contain newlines; pairs with `xargs -0`)
- `--height <N|PCT%>` — render inline in the given number of rows below the cursor instead of fullscreen. Accepts either an absolute cell count (e.g. `12`) or a percentage (e.g. `50%`); percentages are resolved against the current terminal rows at render time and clamped to a floor of 3 rows.

## Subcommands

### text-input

Single-line text input.

```bash
question text-input --label "Enter your name" --max-length 50
```

**Options:**

- `--label <TEXT>` — label text
- `--label-position {above|below|left|right}` — where the label renders (default: `above`)
- `--max-length <N>` — maximum input length (hard cap enforced at keystroke time)

**Output:** raw string followed by newline.

### text-area-input

Multi-line text editor.

```bash
question text-area-input --width 80 --scrollbar
```

**Options:**

- `--label <TEXT>` — label text
- `--width <N>` — editor width in columns (default: 60)
- `--scrollbar` — show vertical scrollbar when content exceeds height
- `--initial <TEXT>` — initial buffer contents (newlines in the argument become line breaks)

**Output:** raw text (newlines preserved).

### boolean-switch

Toggle switch.

```bash
question boolean-switch --labels "YES,NO" --initial true
```

**Options:**

- `--label <TEXT>` — label text
- `--labels <ON,OFF>` — custom on/off captions (default: `"true,false"`)
- `--initial {true|false}` — initial checked value (default: `false`)

**Output:** `true` or `false`.

### choose-one

Single-selection list.

```bash
question choose-one Red Green Blue
printf "%s\n" "Red" "Green" "Blue" | question choose-one
```

**Option sources (mutually exclusive):**

- Positional arguments — used when no explicit source flag is set
- `--csv <TEXT>` — comma-separated list (alias: `--options` for backward compatibility)
- `--list <TEXT>` — newline-separated list
- `--rows <TEXT>` — newline-separated `label::value` pairs
- `--file <PATH>` — JSON, JSONL, NDJSON, YAML, TOML, or CSV file containing an array
- `--md <PATH> <PROP>` — YAML frontmatter array property from a Markdown file
- Piped stdin (automatic when stdin is not a TTY)

**Selection & filtering:**

- `--delimiter <CHAR>` — split each option on the first delimiter into `label` and returned `value`
- `--label <TEXT>` — label text
- `--label-position {above|below|left|right}` — where the label renders (default: `above`)
- `--selected <VALUE>` — pre-select the option whose value matches
- `--required` — submission is blocked if no selection made
- `--no-filter` — disable the default fuzzy filter and use legacy first-letter shortcuts
- `--sort {natural|inverse|asc|desc}` — order options before rendering (`reverse` is a hidden alias for `inverse`)

**Hotkeys & normalization:**

- `--numeric-hot-keys` — auto-assign Ctrl+1..9,0 then Alt+1..9,0 to the first 20 options
- `--label-convention <caps|lowercase|camel-case|pascal-case|kebab-case|snake-case|title-case>` — transform option labels
- `--value-convention <caps|lowercase|camel-case|pascal-case|kebab-case|snake-case|title-case>` — transform option values
- `::` delimiter in option text splits `label::value` (takes precedence over conventions)
- `[CTRL+X]`, `[ALT+X]`, `[OPT+X]` prefixes in option text assign explicit hotkeys

**Chrome:**

- `--border`, `--border-label <TEXT>`, `--border-style <STYLE>` — add border chrome
- `--margin <N>`, `--mt <N>`, `--mb <N>`, `--ml <N>`, `--mr <N>` — outer margin
- `--padding <N>` / `-p <N>`, `--pt <N>`, `--pb <N>`, `--pl <N>`, `--pr <N>` — inner padding
- `--active-color {grey|green|yellow|red}` — background colour for the active row (default `grey`); the renderer picks a contrasting foreground based on the detected terminal background and only paints the focus indicator + label + one trailing blank cell

**Output:** the selected value (raw string).

When no explicit source flag is provided, `choose-one` reads options
from positional arguments first, then from piped stdin. Without
`--delimiter`, each option's label and value are the same. With
`--delimiter ":"`, `question choose-one "Apple:1"` displays `Apple`
and returns `1`.

Typing alphanumeric characters opens the fuzzy filter by default. Use
Up/Down (or j/k) to move the active row, Space to select, and Enter to
submit. `Esc` restores the initial selection and submits (exit `0`).
Ctrl/Alt hotkeys select and submit immediately.

`--height` accepts a percentage suffix — see **Global Flags**.

### choose-many

Multi-selection list.

```bash
question choose-many Red Green Blue --min-selections 1 --max-selections 2
printf "%s\n" "Red" "Green" "Blue" | question choose-many
```

**Option sources:** Same set as `choose-one` (positional, `--csv`, `--list`, `--rows`, `--file`, `--md`, stdin).

**Selection & filtering:**

- `--selected <VALUE>` — pre-select values; repeat the flag to pre-select multiple (`--selected foo --selected bar`). Comma-splitting is **not** applied — if you need CSV semantics, use the deprecated `--initial` flag.
- `--required` — fail if no items are selected.
- `--min-selections <N>` — minimum number of selections required (submit-time validation)
- `--max-selections <N>` — maximum number of selections allowed (keystroke-time cap)
- `--delimiter <CHAR>` — split each option on the first delimiter into `label` and `value`
- `--no-filter` — disable fuzzy filter
- `--sort {natural|inverse|asc|desc}` — order options before rendering

**Hotkeys, normalization, and chrome:** Same flags as `choose-one`.

**Output (raw mode):** one value per line (newline-separated, matches `grep` and `sort` conventions).

**Output (json mode):** JSON array of selected values.

**Output (null mode):** NUL-separated list (for `xargs -0`).

Use Up/Down to move the active row, Space to toggle it, Enter to
submit the current selection exactly as-is, `Ctrl+A` to select all
enabled options, and `Ctrl+D` to clear the selection.

`--height` accepts a percentage suffix — see **Global Flags**.

### completions

Generate shell completion scripts.

```bash
question completions bash > /usr/share/bash-completion/completions/question
question completions zsh > /usr/share/zsh/site-functions/_question
question completions fish > ~/.config/fish/completions/question.fish
```

**Options:**

- `bash`, `zsh`, `fish`, `elvish`, `powershell` — target shell

### input-table

Grid of mixed input cells.

```bash
question input-table --columns '[
  {"type": "static", "id": "name", "text": "Alice"},
  {"type": "boolean", "id": "active"}
]'
```

**Options:**

- `--columns <JSON>` — array of column definitions. Each object has:
  - `type` — one of: `static`, `boolean`, `text`, `textarea`, `choose-one`, `choose-many`
  - `id` — column identifier (becomes JSON key in output)
  - type-specific fields (e.g. `text` for static, `options` for choice columns, `max_length` for text)
- `--rows <JSON>` — optional initial row data (array of arrays matching column order)

**Column spec schema:**

```json
{
  "type": "static",
  "id": "name",
  "text": "Display text"
}

{
  "type": "boolean",
  "id": "active",
  "initial": true
}

{
  "type": "text",
  "id": "email",
  "max_length": 100
}

{
  "type": "textarea",
  "id": "notes",
  "width": 40
}

{
  "type": "choose-one",
  "id": "role",
  "options": ["Admin", "User", "Guest"],
  "required": true
}

{
  "type": "choose-many",
  "id": "tags",
  "options": ["urgent", "bug", "feature"],
  "min_selections": 1,
  "max_selections": 3
}
```

**Output (raw mode):** JSON array of row objects. Each object is keyed by `column_id`:

```json
[
  {
    "name": "Alice",
    "active": true,
    "role": "Admin",
    "tags": ["urgent", "bug"]
  },
  {
    "name": "Bob",
    "active": false,
    "role": "User",
    "tags": []
  }
]
```

Note that boolean cells emit JSON booleans (`true`/`false`), and `choose-many` cells emit JSON arrays. This typed output was introduced in Phase 5.

**Output (null mode):** `key=value` pairs separated by NUL bytes. Multi-selection cells emit one `key=value` pair per selected value.

## Exit Codes

- `0` — user submitted a value (stdout contains the result). For `choose-one`, pressing `Esc` restores the initial selection and also exits `0`.
- `1` — user pressed Esc to abort (all components except `choose-one`), or a terminal I/O error occurred. No output written to stdout on abort.
- `130` — user pressed Ctrl-C / SIGINT (no output written to stdout)
- Non-zero (other) — argument parsing error or invalid configuration

## Shell Integration

### Capture scalar values

```bash
NAME=$(question text-input --label "Name")
ACTIVE=$(question boolean-switch --labels "Yes,No")
ROLE=$(question choose-one Admin User Guest)
```

### Process multi-selection list

```bash
question choose-many Red Green Blue | while read -r color; do
  echo "Selected: $color"
done
```

### Handle NUL-separated values (when values may contain newlines)

```bash
question choose-many --output null --options "Line one,Line\ntwo" | xargs -0 -I {} echo "Item: {}"
```

### Parse JSON table output

```bash
question input-table --columns '[{"type":"text","id":"name"},{"type":"boolean","id":"active"}]' \
  | jq '.[] | select(.active == true) | .name'
```

## Documentation

For library documentation and design details, see:

- [tui-chrome library README](../lib/README.md)
- [spec.md](../features/2026-04-16-input-tui/spec.md)
- [tech-design.md](../features/2026-04-16-input-tui/tech-design.md)
