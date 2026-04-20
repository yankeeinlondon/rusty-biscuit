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
- `--height <N>` — render inline in `N` rows below the cursor instead of fullscreen

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
question choose-one --options "Red,Green,Blue"
```

**Options:**

- `--options <CSV>` — comma-separated list of option values
- `--options-from-file <PATH>` — read options from a markdown list (ordered or unordered)
- `--options-from-dictionary <PATH>` — read from YAML/JSON file (keys = labels, values = values)
- `--label <TEXT>` — label text
- `--required` — submission is blocked if no selection made

**Output:** the selected value (raw string).

### choose-many

Multi-selection list.

```bash
question choose-many --options "Red,Green,Blue" --min-selections 1 --max-selections 2
```

**Options:**

- Same as `choose-one`, plus:
- `--min-selections <N>` — minimum number of selections required (submit-time validation)
- `--max-selections <N>` — maximum number of selections allowed (keystroke-time cap)

**Output (raw mode):** one value per line (newline-separated, matches `grep` and `sort` conventions).

**Output (json mode):** JSON array of selected values.

**Output (null mode):** NUL-separated list (for `xargs -0`).

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
- `130` — user cancelled (Esc or Ctrl-C; no output written to stdout)
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
