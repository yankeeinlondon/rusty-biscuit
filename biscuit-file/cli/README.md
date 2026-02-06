# bf

Command-line interface for `biscuit-file`. Converts between JSON, YAML, and TOML, extracts content from PDFs, and reads frontmatter from Markdown files.

## Installation

```sh
cargo install --path .
```

## Usage

```
bf [OPTIONS] [FILE]
```

When `FILE` is omitted or `-`, reads from STDIN (requires `--input-format`).

## Output Format Flags

Exactly one output flag may be specified. If none is given, the default depends on the input type.

| Flag | Description | Default for |
|--------|------------------------------|-------------|
| `--json` | Output as JSON (pretty-printed) | TOML, YAML, JSON, Markdown |
| `--yaml` | Output as YAML | |
| `--toml` | Output as TOML | |
| `--text` | Output as plain text | PDF |
| `--md` | Output as Markdown | |

Passing more than one output flag produces an error:

```
error: the argument '--json' cannot be used with '--yaml'
```

## Input Format Detection

File type is detected automatically from extension:

| Extensions | Detected As |
|------------|-------------|
| `.toml` | TOML |
| `.yaml`, `.yml` | YAML |
| `.json` | JSON |
| `.md`, `.markdown`, `.mdx` | Markdown |
| `.pdf` | PDF |

PDF files are also detected by magic bytes (`%PDF-`), regardless of extension.

Use `--input-format` to override detection:

```sh
bf data.txt --input-format yaml --json
```

## Conversion Matrix

| Input \ Output | `--json` | `--yaml` | `--toml` | `--text` | `--md` |
|----------------|:--------:|:--------:|:--------:|:--------:|:------:|
| TOML | yes | yes | yes | -- | -- |
| YAML | yes | yes | yes | -- | -- |
| JSON | yes | yes | yes | -- | -- |
| Markdown | yes | yes | yes | -- | -- |
| PDF | -- | -- | -- | yes | yes |

## STDIN Support

Omit the file argument or pass `-` to read from STDIN. The `--input-format` flag is required since there is no file extension to detect from.

```sh
# Omit file argument
cat config.yaml | bf --input-format yaml --json

# Explicit dash
echo '{"key": "value"}' | bf - --input-format json --toml

# Pipe between bf invocations
bf config.toml --json | bf --input-format json --yaml
```

## Markdown Frontmatter

When the input is a Markdown file, `bf` extracts the frontmatter block and converts it. The body content is ignored.

Two frontmatter formats are supported:

**YAML frontmatter** (delimited by `---`):

```markdown
---
title: My Post
date: 2026-01-15
tags:
  - rust
  - cli
---

Body content here...
```

**TOML frontmatter** (delimited by `+++`):

```markdown
+++
title = "My Post"
date = 2026-01-15
draft = true
+++

Body content here...
```

The frontmatter format is detected automatically from the delimiter. The extracted data is then converted to whatever output format is requested.

```sh
bf post.md --json    # frontmatter as JSON
bf post.md --toml    # frontmatter as TOML
bf post.md --yaml    # frontmatter as YAML
bf post.md           # defaults to JSON
```

## Examples

```sh
# Format conversions
bf Cargo.toml --json
bf docker-compose.yml --toml
bf package.json --yaml

# Markdown frontmatter
bf README.md --json
bf content/post.md --toml

# PDF extraction
bf report.pdf              # plain text (default)
bf report.pdf --md         # as Markdown

# STDIN piping
curl -s api.example.com/config.json | bf --input-format json --yaml
bf Cargo.toml --json | bf --input-format json --yaml
```
