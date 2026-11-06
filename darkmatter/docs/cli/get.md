## Overview

The `get` command extracts frontmatter properties from a markdown document.

It is intended for metadata-oriented workflows in scripts and pipelines.

## Reporting

### Usage

```bash
# Single property
md get doc.md title

# Multiple properties
md get doc.md title author tags

# Read from stdin
cat doc.md | md get - title

# Raw values for the shell: strings unquoted, null as empty
TITLE=$(md get doc.md title --output plain)

# Alternate data formats
md get doc.md title --output yaml
md get doc.md title --output json5
md get doc.md title count --output toml

# Arrays and objects on one line
md get doc.md tags --compact
```

### Arguments

- `<INPUT>`: Markdown file path (supports `@` file references) or `-` for stdin.
- `<PROP>...`: One or more frontmatter property names (required).

### Options

- `--output <terminal|plain|markdown|markdown-plus|html|browser|json|json5|yaml|toml>`: what to print (default: `json`). See [Output Behavior](#output-behavior).
- `--compact`: print arrays and objects on a single line. Valid only with `--output json` or `--output json5`; with any other output it is a usage error (exit status 2).
- `--width <N|N%|auto|source|unlimited>`: accepted like on every command. It has no effect, because `md get` prints no prose.

### Output Behavior

**Single property request**

- Returns the raw property value.
- Missing property returns an empty string.

**Multiple property request**

- Returns an object/map containing requested keys.
- Missing keys are included with empty-string values.

**Default output**

- Pretty-printed JSON, on a terminal and on a pipe.

**`--output plain`**

- The raw value, for shell use: a string unquoted, `null` as empty, a list as one
  item per line, an object as `key: value` lines. `terminal` prints the same text.
  `markdown`, `markdown-plus`, and `html` carry the same text in that form.

```text
$ md get doc.md tags
[
  "a",
  "b"
]

$ md get doc.md tags --output plain
a
b

$ md get doc.md title count --output plain
title: Notes
count: 3
```

**Data outputs**

- A clean success is the bare value, because the keys are yours and a property
  called `outcome` must not collide with anything. A TOML output wraps a value that
  is not an object as `value = ...`, because TOML requires a table root.
- A run with something to say wraps the value. A failure has `value: null` and a
  warning keeps the value:

```text
$ md get doc.md title --width 72.9
{
  "outcome": "success",
  "diagnostics": [{ "code": "dm.output.fractional_width", "...": "..." }],
  "value": "Notes"
}
```

A failure is status 1. The default output prints that wrapped value on stdout, and a
formatted output writes the error to stderr. See
[Output and Width](./output-and-width.md) and
[Data Contracts](./data-contracts.md#other-commands).

### Property Lookup Scope

- `get` looks up top-level frontmatter keys by exact key name.
- It does not evaluate dotted paths or expressions.

## Lessons Learned

- Returning empty strings for missing keys keeps shell usage simple and predictable.
- Multi-property output shape is stable for script consumption.
- JSON5, YAML, and TOML outputs make interop with config tooling easier.

## Issues

- Missing and intentionally-empty string values are indistinguishable in CLI output.

## Removed spellings

| Removed | Use instead |
|---|---|
| `--raw` | `--output plain` |
| `--json5`, `--yaml`, `--toml` | `--output json5`, `--output yaml`, `--output toml` |
