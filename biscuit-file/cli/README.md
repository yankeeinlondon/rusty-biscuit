# bf

Command-line interface for `biscuit-file`. Converts between JSON, JSON5, YAML, and TOML, extracts content from PDFs, and reads frontmatter from Markdown files.

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
| `--json` | Output as JSON (pretty-printed) | TOML, YAML, JSON, JSON5, Markdown |
| `--json5` | Output as JSON5 (unquoted keys, single-quoted strings) | |
| `--yaml` | Output as YAML | |
| `--toml` | Output as TOML | |
| `--text` | Output as plain text | PDF |
| `--md` | Output as Markdown | |

Passing more than one output flag produces an error:

```
error: the argument '--json' cannot be used with '--yaml'
```

## Compact Mode

Use `--compact` with `--json` or `--json5` for single-line output:

```sh
bf data.json --compact                 # compact JSON (default format)
bf data.json --json5 --compact         # compact JSON5
bf config.toml --json --compact        # any input format works
```

`--compact` is ignored for YAML, TOML, text, and Markdown output.

## Input Format Detection

File type is detected automatically from extension:

| Extensions | Detected As |
|------------|-------------|
| `.toml` | TOML |
| `.yaml`, `.yml` | YAML |
| `.json` | JSON |
| `.json5` | JSON5 |
| `.md`, `.markdown`, `.mdx` | Markdown |
| `.pdf` | PDF |

PDF files are also detected by magic bytes (`%PDF-`), regardless of extension.

Use `--input-format` to override detection:

```sh
bf data.txt --input-format yaml --json
```

## Conversion Matrix

| Input \ Output | `--json` | `--json5` | `--yaml` | `--toml` | `--text` | `--md` |
|----------------|:--------:|:---------:|:--------:|:--------:|:--------:|:------:|
| TOML | yes | yes | yes | yes | -- | -- |
| YAML | yes | yes | yes | yes | -- | -- |
| JSON | yes | yes | yes | yes | -- | -- |
| JSON5 | yes | yes | yes | yes | -- | -- |
| Markdown | yes | yes | yes | yes | -- | -- |
| PDF | -- | -- | -- | -- | yes | yes |

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

## File Reference Resolution

The `reference` subcommand (alias `ref`) resolves file reference strings to filesystem paths.

```sh
bf reference @docs/spec.md           # magic: searches repo root, then HOME
bf reference !README.md              # package: resolves from Cargo workspace area
bf reference %foo.md                 # recursive: walks directories for a match
bf ref ./Cargo.toml                  # alias for 'reference'
```

### Output spelling

Resolved paths print as portable text through
[`biscuit_file::to_portable_string`](../README.md), so `bf ref ./Cargo.toml`
prints `C:/repo/Cargo.toml` on Windows rather than `C:\repo\Cargo.toml`. A
script that captures this output behaves the same on every host.

A Windows UNC, device, or unreducible verbatim path has no faithful
`/`-separated spelling and prints natively instead, because a path the caller
cannot open would be worse than an inconsistent separator.

### Flags

| Flag | Description |
|------|-------------|
| `--relative` | Output path relative to CWD |
| `--relative-cwd` | Output path relative to CWD |
| `--add-vault` / `-v` | Add a vault root for `vault:` references |

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | File found and path printed |
| `1` | Well-formed reference but no matching file |
| `2` | Error (invalid syntax, missing env var, etc.) |

### Examples

```sh
# Resolve a magic reference
bf reference @CLAUDE.md

# Resolve with relative output
bf reference --relative-cwd @docs/spec.md

# Resolve a vault reference with custom root
bf reference --add-vault ~/my-vault vault:notes/today.md
bf reference -v ~/vault1 -v ~/vault2 vault:note.md
```

## Examples

```sh
# Format conversions
bf Cargo.toml --json
bf docker-compose.yml --toml
bf package.json --yaml
bf config.json5 --json

# JSON5 output
bf package.json --json5           # pretty JSON5 with unquoted keys
bf package.json --json5 --compact # single-line JSON5

# Compact output
bf data.json --compact            # single-line JSON
bf data.json --json5 --compact    # single-line JSON5

# Markdown frontmatter
bf README.md --json
bf content/post.md --toml

# PDF extraction
bf report.pdf              # plain text (default)
bf report.pdf --md         # as Markdown

# STDIN piping
curl -s api.example.com/config.json | bf --input-format json --yaml
bf Cargo.toml --json | bf --input-format json --yaml
echo "{ key: 'value' }" | bf --input-format json5 --json
```
