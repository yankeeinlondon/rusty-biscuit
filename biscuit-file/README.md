# Biscuit File

A Rust toolkit for converting between common file formats and extracting content from documents.

## What It Does

- **Convert data formats** -- Move data freely between JSON, JSON5, YAML, and TOML
- **Extract PDF content** -- Pull text or Markdown from PDF documents
- **Read Markdown frontmatter** -- Extract and convert the YAML or TOML metadata block from Markdown files
- **Detect file types** -- Automatically identify files using extensions and magic bytes

## Packages

| Package | Description |
|---------|-------------|
| `biscuit-file` (lib) | Core library with parsers, converters, and file detection |
| `biscuit-file-cli` (`bf`) | Command-line interface for all conversions |

## Quick Start

```sh
# Install the CLI
cargo install --path cli

# Convert between formats
bf config.toml --yaml
bf data.yaml --json
bf settings.json --toml
bf config.json5 --json

# JSON5 output (unquoted keys, single-quoted strings, trailing commas)
bf data.json --json5

# Compact single-line output (JSON and JSON5)
bf data.json --compact
bf data.json --json5 --compact

# Extract frontmatter from Markdown
bf README.md --json
bf post.md --toml

# Extract text from PDFs
bf document.pdf
bf document.pdf --md

# Pipe through STDIN
cat data.json | bf --input-format json --yaml
```

## Supported Formats

| Format | Read | Write |
|--------|:----:|:-----:|
| JSON | yes | yes |
| JSON5 | yes | yes |
| YAML | yes | yes |
| TOML | yes | yes |
| Markdown (frontmatter) | yes | -- |
| PDF | yes | -- |
