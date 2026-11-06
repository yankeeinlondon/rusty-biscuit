## Overview

The `delta` command compares two markdown documents and reports structural and content-level changes.

It analyzes:

- frontmatter differences
- preamble changes
- added/removed/modified/moved sections
- code block changes
- potentially broken internal links

## Reporting

### Usage

```bash
# Human-readable report
md delta base.md updated.md

# Data for a script
md delta base.md updated.md --output json

# Verbose report
md delta base.md updated.md -v
```

### Arguments

- `<BASE>`: original markdown document (supports `@` file references).
- `<UPDATED>`: updated markdown document (supports `@` file references).

### Options

- `--output <terminal|plain|markdown|markdown-plus|html|browser|json|json5|yaml|toml>`: what to print (default: `terminal`).
- `--width <N|N%|auto|source|unlimited>`: accepted like on every command; the report's rows are not wrapped.

### Output

`md delta` accepts both output families. The report is plain text and holds no
escape codes on a pipe. `markdown`, `markdown-plus`, and `html` print the same text
as one code block. A data output prints one value, and writes nothing to stderr. A
failure, such as a file that cannot be read, is status 1: a formatted output writes
the error to stderr, and a data output prints
`{ "outcome": "failure", "diagnostics": [...], "value": null }`. See
[Output and Width](./output-and-width.md) and
[Data Contracts](./data-contracts.md#other-commands).

### Classification Header

Text output starts with a symbol-based classification:

| Symbol | Classification                                |
|--------|-----------------------------------------------|
| `✓`    | No changes                                    |
| `~`    | Whitespace changes only                       |
| `◈`    | Frontmatter only / frontmatter and whitespace |
| `⊕`    | Structural only                               |
| `△`    | Minor changes                                 |
| `◐`    | Moderate changes                              |
| `◉`    | Major changes                                 |
| `★`    | Rewritten                                     |

The header includes a content-change percentage.

### Sections in Text Output

- Frontmatter changes
- Preamble changes
- Added sections
- Removed sections
- Modified sections
- Moved sections
- Code block changes
- Broken internal links
- Whitespace-only changes (grouped near the end)

### Verbose Mode

Use `md delta ... -v` or `md -v delta ...`. It applies to formatted outputs only.

Verbose output adds:

- extra statistics (bytes, section counts)
- line numbers where available
- visual diff blocks for changed frontmatter/content sections

### Data Output

A data output includes the same core analysis in machine-readable form, including:

- `classification`
- `statistics`
- `frontmatter_changed`
- `frontmatter_changes`
- `preamble_changed`
- `added`, `removed`, `modified`, `moved`
- `code_block_changes`
- `broken_links`

### Related Workflow

`md clean FILE.md --save` and `md FILE.md --save` reuse this same delta reporting style after in-place cleanup.

## Lessons Learned

- Section-oriented delta reporting is often more useful than line-only diffs for docs maintenance.
- Broken-link checks catch anchor regressions caused by heading edits.
- Verbose mode is useful for review workflows where traceability matters.

## Issues

- There is no built-in filter to limit report categories (for example, only links or only frontmatter).

## Removed spellings

| Removed | Use instead |
|---|---|
| `--json` | `--output json` |
