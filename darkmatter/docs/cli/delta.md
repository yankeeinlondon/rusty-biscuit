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

# JSON report
md delta base.md updated.md --json

# Verbose report (root-level flag)
md -v delta base.md updated.md
```

### Arguments

- `<BASE>`: original markdown document (supports `@` file references).
- `<UPDATED>`: updated markdown document (supports `@` file references).

### Options

- `--json`: emit structured JSON instead of text report.

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

Use `md -v delta ...`.

Verbose output adds:

- extra statistics (bytes, section counts)
- line numbers where available
- visual diff blocks for changed frontmatter/content sections

### JSON Output

JSON includes the same core analysis in machine-readable form, including:

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