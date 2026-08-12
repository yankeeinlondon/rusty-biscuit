# Biscuit File

A Rust library and CLI for working with files and file formats.

## Functional Overview

- **Convert data formats** -- Move data freely between JSON, JSON5, YAML, and TOML
- **Extract PDF content** -- Pull text or Markdown from PDF documents
- **Read Markdown frontmatter** -- Extract and convert the YAML or TOML metadata block from Markdown files
- [**Analyze and repair YAML source**](./lib/README.md#yaml-source-analysis-and-repair) -- produce span-aware diagnostics, inspect certainty, and safely apply deterministic edits
- **Detect file types** -- Automatically identify files using extensions and magic bytes
- [**File Resolution**](./docs/topics/file-references.md) -- resolves the file path of a passed in file using a set of smart and consistent path based logic
- [**Portable path text**](#portable-path-text) -- render a `Path` as forward-slash text without breaking Windows verbatim, UNC, or device paths

## Packages

| Package                   | Description                                               |
|---------------------------|-----------------------------------------------------------|
| `biscuit-file` (lib)      | Core library with parsers, converters, and file detection |
| `biscuit-file-cli` (`bf`) | Command-line interface for all conversions                |

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

## Portable Path Text

Turning a `Path` into text is a domain boundary, and `.replace('\\', "/")` is
the wrong tool for it: applied to `\\?\C:\CON` it yields `//?/C:/CON`, which is
neither a path nor a URL. Two unfeatured functions own the policy — available
even with `--no-default-features`:

```rust
use std::path::Path;
use biscuit_file::{to_portable_string, try_portable_string};

// Separators are normalized; a safely reducible `\\?\` prefix is removed.
assert_eq!(to_portable_string(Path::new(r"docs\file.md")), "docs/file.md");
```

`to_portable_string` renders portable text when a faithful slash-separated
spelling exists and otherwise returns the **native** spelling unchanged.
`try_portable_string` is the same function with the fallback exposed as `None`
so a caller can act on it.

Reach for `try_portable_string` when a native spelling would be wrong output —
a Markdown link destination, for instance, where CommonMark's backslash escapes
mean `\\server\share\f.md` does not survive a parse. Reach for
`to_portable_string` when native text is still correct for the consumer:
diagnostics, completions, YAML scalars.

A path is declined when it is a Windows UNC, device-namespace, or verbatim path
that `dunce::simplified` would not reduce — including reserved DOS names,
trailing dots or spaces, over-`MAX_PATH` paths, and paths whose `.` or `..`
components are literal filenames under `\\?\`. Nothing is collapsed lexically;
`dunce`'s refusal is authoritative.

Two conversions are lossy and deliberate: non-Unicode path data becomes U+FFFD
via `Path::to_string_lossy`, and on Unix a literal `\` in a filename is rendered
as `/`.

## Supported Formats

| Format                 | Read | Write |
|------------------------|:----:|:-----:|
| JSON                   | yes  | yes   |
| JSON5                  | yes  | yes   |
| YAML                   | yes  | yes   |
| TOML                   | yes  | yes   |
| Markdown (frontmatter) | yes  | --    |
| PDF                    | yes  | --    |
