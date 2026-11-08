## Overview

`md list-themes` lists the themes a style can name, in their canonical order,
each with a short description. Use a name as the value of `style.page.theme`
(the prose theme) or `style.page.code.theme` (the code theme):

```sh
md list-themes
md README.md style.page.theme=github style.page.code.theme=dracula
```

Theme values are matched without regard to case, and a few aliases are
accepted, but the names listed here are the canonical ones that shell
completion offers.

## Usage

```sh
# A list for reading
md list-themes

# Data for a script
md list-themes --output json
```

### Options

- `--output <terminal|plain|markdown|markdown-plus|html|browser|json|json5|yaml|toml>`: What to print (default: `terminal`).
- `--width <N|N%|auto|source|unlimited>`: Width the list is wrapped to (see [Output and Width](./output-and-width.md#choosing-a-width)).

The command takes no input file.

## Output

The formatted outputs list each theme and end with a line naming the two
style setters that select one.

A data output is a bare array of objects, one per theme, in canonical order:

```json
[
  { "name": "base16-ocean", "description": "…" },
  { "name": "github", "description": "…" }
]
```

| Field | Meaning |
|---|---|
| `name` | the kebab-case name a style value takes |
| `description` | the dark variant's description |

TOML has no top-level array, so it writes the list under `value`. A run that
carries a diagnostic (an unusable `--width`, for example) prints
`{ outcome, diagnostics, value }` instead. The schema is
`cli/schemas/output/list-themes.yaml`.

The status is 0 on success and 2 for a usage error.
