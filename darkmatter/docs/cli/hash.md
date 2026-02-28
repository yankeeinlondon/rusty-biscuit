## Overview

The `hash` command computes deterministic xxHash64 values for markdown frontmatter and body content.

It supports single-file/stdin hashing and aggregate directory hashing.

## Reporting

### Usage

```bash
# Hash frontmatter + body
md hash doc.md
md hash -

# Hash only body or only frontmatter
md hash doc.md --body
md hash doc.md --frontmatter

# Strict mode
md hash doc.md --strict

# Directory aggregate hashing
md hash docs/
md hash docs/ --body
```

### Arguments

- `[INPUT]`: file path, directory path, or `-` for stdin. If omitted, reads stdin when piped; otherwise errors.

### Options

- `--body`: output only body hash.
- `--frontmatter`: output only frontmatter hash.
- `--strict`: disable normalization and hash raw serialized content.

### Flag Precedence

- If both `--body` and `--frontmatter` are passed, `--body` takes precedence.

### Output Format

**Single file/stdin**

- Default: `<frontmatter_hash>-<body_hash>`
- `--body`: `<hash>`
- `--frontmatter`: `<hash>`

All hashes are 16-char lowercase hex values.

**Directory input**

- Recursively collects `.md` and `.dm` files.
- Skips hidden directories.
- Ignores non-markdown files.
- Sorts paths before aggregation for deterministic output.

Directory output forms:

- Default: aggregate `<frontmatter_hash>-<body_hash>`
- `--body` or `--frontmatter`: single aggregate hash

### Normalization Behavior

**Non-strict mode**

- Frontmatter: canonicalized by sorted keys and JSON-serialized values.
- Body: whitespace normalization variants are applied before hashing.

**Strict mode**

- Frontmatter: hashes YAML serialization without canonical key sorting.
- Body: hashes raw body bytes.

## Lessons Learned

- Non-strict mode is best for change detection where formatting-only edits should often collapse.
- Strict mode is best when exact serialized content differences matter.
- Directory mode is deterministic and optimized for larger doc trees.

## Issues

- There is no structured (`--json`) output mode; hash output is plain text only.
