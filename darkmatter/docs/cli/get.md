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

# Alternate output formats
md get doc.md title --yaml
md get doc.md title --json5
md get doc.md title count --toml
```

### Arguments

- `<INPUT>`: Markdown file path or `-` for stdin.
- `<PROP>...`: One or more frontmatter property names (required).

### Options

- `--json5`: output as JSON5.
- `--yaml`: output as YAML.
- `--toml`: output as TOML.

### Output Behavior

**Single property request**

- Returns the raw property value.
- Missing property returns an empty string.

**Multiple property request**

- Returns an object/map containing requested keys.
- Missing keys are included with empty-string values.

**Default format**

- Pretty-printed JSON.

### Format Selection Notes

- If multiple format flags are passed, precedence is:
  - `--json5` first
  - then `--yaml`
  - then `--toml`
  - otherwise JSON
- TOML output wraps scalar single-property values under `value = ...` because TOML requires a table root.

### Property Lookup Scope

- `get` looks up top-level frontmatter keys by exact key name.
- It does not evaluate dotted paths or expressions.

## Lessons Learned

- Returning empty strings for missing keys keeps shell usage simple and predictable.
- Multi-property output shape is stable for script consumption.
- JSON5/YAML/TOML output options make interop with config tooling easier.

## Issues

- Missing and intentionally-empty string values are indistinguishable in CLI output.
