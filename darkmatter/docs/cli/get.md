## Overview

The `get` command extracts frontmatter values from a markdown document, naming each with a [property path](./index.md#property-paths).

It is intended for metadata-oriented workflows in scripts and pipelines.

## Reporting

### Usage

```bash
# Single property
md get doc.md title

# Multiple properties
md get doc.md title author tags

# A nested value, an array element, a key with a dot in its name
md get doc.md style.page.max-width
md get doc.md items.1.name
md get doc.md 'a\.b'

# Several paths give a flat object, keyed as you wrote them
md get doc.md author.name title

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
- `<PROP>...`: One or more [property paths](./index.md#property-paths) (required). A malformed path such as `a..b` is a usage error with exit status 2, before the document is read.

### Options

- `--output <terminal|plain|markdown|markdown-plus|html|browser|json|json5|yaml|toml>`: what to print (default: `json`). See [Output Behavior](#output-behavior).
- `--compact`: print arrays and objects on a single line. Valid only with `--output json` or `--output json5`; with any other output it is a usage error (exit status 2).
- `--width <N|N%|auto|source|unlimited>`: accepted like on every command. It has no effect, because `md get` prints no prose.

### Output Behavior

**Single property request**

- Returns the raw property value.
- A missing path returns an empty string. That includes a path that cannot be followed: an index out of range, or a path through text or `null`.
- An explicit `null` stays `null` in data outputs.

**Multiple property request**

- Returns a flat object with one entry per argument, keyed as you wrote it. It is not rebuilt into a nested object.
- Missing paths are included with empty-string values.

```text
$ md get doc.md author.name title
{
  "author.name": "Alice",
  "title": "Notes"
}
```

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

- `get` reads the frontmatter as written, with no schema. It evaluates no expression, so `{{ title }}` comes back as that text.
- `.` always separates keys. `md get doc.md a.b` reads `b` inside `a`, even when the document has a top-level key named `a.b`; write `md get doc.md 'a\.b'` for that key. See [Property Paths](./index.md#property-paths).

## Lessons Learned

- Returning empty strings for missing keys keeps shell usage simple and predictable.
- Multi-property output shape is stable for script consumption.
- JSON5, YAML, and TOML outputs make interop with config tooling easier.

## Issues

- Missing and intentionally-empty string values are indistinguishable in CLI output.

## Migrating older calls

| If you relied on | Now |
|---|---|
| `md get doc.md a.b` reading a top-level key whose name holds a dot | `md get doc.md 'a\.b'`; `a.b` reads `b` inside `a` |
| a key name holding `\\` or `\.` | write each backslash as `\\`: the key `x\.y` is `'x\\\.y'` |
| `md get doc.md a..b` printing `""` | it is now a usage error, because a key cannot be empty in a longer path |

An ordinary call such as `md get doc.md title` is unchanged.

## Removed spellings

| Removed | Use instead |
|---|---|
| `--raw` | `--output plain` |
| `--json5`, `--yaml`, `--toml` | `--output json5`, `--output yaml`, `--output toml` |
