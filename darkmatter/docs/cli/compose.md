## Overview

The `compose` command runs markdown through the transform pipeline and outputs the composed result.

It is used for document assembly workflows like:

- interpolation (`{{ ... }}`)
- replacement transforms
- transclusion (`::file`, `::code`, and related directives)

Unlike `read`, `compose` focuses on transformed content, not display rendering defaults.

## Reporting

### Usage

```bash
# Compose a file
md compose doc.md

# Compose stdin
cat doc.md | md compose
md compose -

# Provide default values as JSON or JSON5
md compose doc.md --state '{"name":"Alice","env":"prod"}'
md compose doc.md --state '{name: "Alice", env: "prod"}'

# Include frontmatter in output
md compose doc.md --frontmatter
md compose doc.md --fm

# Render composed output as HTML or JSON
md compose doc.md --output html
md compose doc.md --output json

# Show output via temp artifact
md compose doc.md --show
```

### Arguments

- `[INPUT]`: Markdown file path (supports `@` file references). Use `-` for stdin. If omitted, reads stdin when piped; otherwise errors.

### Options

- `--state <JSON>`: default values as JSON or JSON5; fills in null/missing frontmatter keys without overriding existing values.
- `--frontmatter` / `--fm`: include frontmatter in the output (default: body only).
- `--output <markdown|text|html|json|ast|auto>`: output format (default: `markdown`).
- `--show`: open output via temp artifact.

### Output Behavior

**Default (`--output markdown`)**

- Prints composed markdown content.
- Frontmatter is consumed as pipeline input and stripped from output unless `--frontmatter` is specified.

**`--frontmatter` / `--fm`**

- Includes frontmatter in the markdown output, reflecting any values filled in by `--state`.
- Key order is preserved from the source document; new keys appear at the end.

**`--output auto`**

- Treated the same as markdown for compose.

**`--output html|json`**

- Emits HTML or AST JSON from the composed document.

**`--show` behavior**

- For markdown/auto: prints composed content and opens markdown artifact.
- For html/json: opens artifact instead of printing to stdout.

### State Merge Behavior

The `--state` flag provides **default values** for the document's frontmatter:

- Null or missing frontmatter keys are filled in from `--state`.
- Existing non-null frontmatter values are preserved (document wins).
- Accepts both JSON and JSON5 (unquoted keys, trailing commas, comments).

```bash
# Given frontmatter: { stage: "plan", feature: null }
md compose doc.md --state '{feature: "auth", stage: "build"}'
# Result: stage stays "plan" (existing), feature becomes "auth" (was null)
```

### Transform Context

- If `[INPUT]` is a file path, compose sets source-file context for relative transclusion resolution.
- If input is stdin (`-` or piped with no input arg), source-file-relative path resolution is not available.
- All file path arguments support `@`-prefixed file references (resolved from git root).

### Validation and Errors

- Invalid JSON/JSON5 in `--state` returns an error.
- Non-object JSON (array/string/number/etc.) in `--state` returns an error.
- Transform failures return non-zero exit with error details.

## Lessons Learned

- `compose` defaults to markdown because composed document output is the primary workflow.
- Frontmatter acts as transform configuration and is intentionally stripped unless `--fm` is used.
- Explicit `--state` is best for script-driven parameterization; it fills gaps without overwriting intent.
- JSON5 support for `--state` makes shell usage more ergonomic (no quoting keys).

## Issues

- CLI currently discards the detailed transform report that library callers can capture.
