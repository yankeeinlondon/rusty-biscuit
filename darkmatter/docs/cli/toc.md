## Overview

The `toc` command extracts heading structure from a markdown document and prints it as a tree, a linked list, or data.

It is useful for:

- quick structure checks on long docs
- heading hierarchy validation workflows
- machine-readable TOC extraction in scripts

## Reporting

### Usage

```bash
# Tree output
md toc README.md
md toc -

# A nested list of links, or data for a script
md toc README.md --output markdown
md toc README.md --output json

# Verbose tree
md toc README.md -v
```

### Arguments

- `<INPUT>`: Markdown file path (supports `@` file references) or `-` for stdin.

### Options

- `--output <terminal|plain|markdown|markdown-plus|html|browser|json|json5|yaml|toml>`: what to print (default: `terminal`).
- `--width <N|N%|auto|source|unlimited>`: accepted like on every command; tree rows are not wrapped.

### Output

`md toc` accepts both output families. Everything it prints goes to stdout, and on
a failure a formatted output writes the error to stderr, while a data output writes
one value to stdout. See [Output and Width](./output-and-width.md) and
[Data Contracts](./data-contracts.md#other-commands).

| `--output` | What you get |
|---|---|
| `terminal` (default), `plain` | the heading tree below; `terminal` falls back to the `plain` bytes on a pipe |
| `markdown`, `markdown-plus`, `html` | a nested list of links to the headings |
| `json`, `json5`, `yaml` | the TOC data |
| `toml` | the TOC data, with each hash as decimal text because the unsigned 64-bit values do not fit a TOML integer |

### Tree Output (Default)

- Printed to stdout.
- Uses box-drawing structure (`├──`, `└──`, `│`).
- Shows a document icon (`📄`) when TOC title metadata is present.

Example shape:

```text
📄
├── Introduction
│   └── Background
└── Usage
```

### Verbose Mode

Use `-v`, before or after the subcommand: `md toc README.md -v`. It applies to
formatted outputs only.

Verbose mode adds, on stdout:

- per-heading normalized hashes in the tree
- summary lines:
    - heading count
    - code block count (if present)
    - internal link counts and broken-link count (if present)
- page hash details when title metadata exists

### Data Output

A data output prints the serialized TOC data, including:

- document title and page hashes
- preamble content and hash
- heading structure tree
- code block metadata
- internal links and slug index

## Lessons Learned

- `toc` is intentionally lightweight and does not mutate documents.
- Tab-indented block scalars in frontmatter are handled without polluting heading extraction.

## Issues

- No built-in `--max-depth` or heading-level include/exclude filters yet.

## Removed spellings

| Removed | Use instead |
|---|---|
| `--json` | `--output json` |
