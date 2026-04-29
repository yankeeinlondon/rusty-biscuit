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

**Options:**

- Positional arguments — option strings, used when no `--options*` flag is set
- `--options <CSV>` — comma-separated list of option values
- `--options-from-file <PATH>` — read options from a markdown list (ordered or unordered)
- `--options-from-dictionary <PATH>` — read from YAML/JSON file (keys = labels, values = values)
- `--delimiter <CHAR>` — split each option on the first delimiter into `label` and returned `value`
- `--label <TEXT>` — label text
- `--selected <VALUE>` — pre-select the option whose value matches
- `--required` — submission is blocked if no selection made
- `--no-filter` — disable the default fuzzy filter and use legacy first-letter shortcuts
- `--sort {natural|reverse|asc|desc}` — order options before rendering
- `--border`, `--border-label <TEXT>`, `--border-style <STYLE>` — add border chrome
- `--margin <N>`, `--mt <N>`, `--mb <N>`, `--ml <N>`, `--mr <N>` — add outer spacing

**Output:** the selected value (raw string).

When neither `--options`, `--options-from-file`, nor
`--options-from-dictionary` is provided, `choose-one` reads options
from positional arguments first, then from piped stdin. Without
`--delimiter`, each option's label and value are the same. With
`--delimiter ":"`, `question choose-one "Apple:1"` displays `Apple`
and returns `1`.

Typing alphanumeric characters opens the fuzzy filter by default. Use
Up/Down to move the active row, Space to select, and Enter to submit.
If Enter is pressed before an explicit Space selection, the active row
is submitted.

`--height` accepts a percentage suffix — see **Global Flags**.

### choose-many

Multi-selection list.

```bash
question choose-many Red Green Blue --min-selections 1 --max-selections 2
printf "%s\n" "Red" "Green" "Blue" | question choose-many
```

**Options:**

- Same as `choose-one`, plus:
- `--selected <VALUE>` — pre-select values; repeat the flag to pre-select multiple (`--selected foo --selected bar`). Comma-splitting is **not** applied — if you need CSV semantics, use the deprecated `--initial` flag.
- `--min-selections <N>` — minimum number of selections required (submit-time validation)
- `--max-selections <N>` — maximum number of selections allowed (keystroke-time cap)

**Output (raw mode):** one value per line (newline-separated, matches `grep` and `sort` conventions).

**Output (json mode):** JSON array of selected values.

**Output (null mode):** NUL-separated list (for `xargs -0`).

Use Up/Down to move the active row, Space to toggle it, Enter to
submit, `Ctrl+A` to select all enabled options, and `Ctrl+D` to clear
the selection. Like `choose-one`, the active row is submitted if Enter
is pressed before any explicit selection.

`--height` accepts a percentage suffix — see **Global Flags**.

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

- `0` — user submitted a value (stdout contains the result)
- `1` — user pressed Esc (no output written to stdout)
- `130` — user pressed Ctrl-C / SIGINT (no output written to stdout)
- Non-zero (other) — argument parsing error or invalid configuration

## Shell Integration

### Capture scalar values

```bash
NAME=$(question text-input --label "Name")
ACTIVE=$(question boolean-switch --labels "Yes,No")
ROLE=$(question choose-one --options "Admin,User,Guest")
```

### Process multi-selection list

```bash
question choose-many --options "Red,Green,Blue" | while read -r color; do
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
