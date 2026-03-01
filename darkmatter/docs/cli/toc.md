## Overview

The `toc` command extracts heading structure from a markdown document and prints it as a tree or JSON.

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

# JSON output
md toc README.md --json

# Verbose tree (verbose is a root flag)
md -v toc README.md
```

### Arguments

- `<INPUT>`: Markdown file path or `-` for stdin.

### Options

- `--json`: output TOC as JSON instead of tree text.

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

Use root verbosity flags, e.g. `md -v toc README.md`.

Verbose mode adds:

- per-heading normalized hashes in the tree
- summary lines on stderr:
    - heading count
    - code block count (if present)
    - internal link counts and broken-link count (if present)
- page hash details on stderr when title metadata exists

The renderer also emits leading/trailing blank lines to stderr for spacing.

### JSON Output

`--json` prints serialized TOC data including:

- document title and page hashes
- preamble content and hash
- heading structure tree
- code block metadata
- internal links and slug index

## Lessons Learned

- `toc` is intentionally lightweight and does not mutate documents.
- Verbose output is split across stdout/stderr so tree output remains redirect-friendly.
- Tab-indented block scalars in frontmatter are handled without polluting heading extraction.

## Issues

- No built-in `--max-depth` or heading-level include/exclude filters yet.
- Verbose mode requires root-level `-v` placement (`md -v toc ...`), not `md toc -v ...`.
