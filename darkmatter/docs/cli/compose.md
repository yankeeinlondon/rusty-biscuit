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

# Provide initial state (must be a JSON object)
md compose doc.md --state '{"name":"Alice","env":"prod"}'

# Render composed output as HTML or JSON
md compose doc.md --output html
md compose doc.md --output json

# Show output via temp artifact
md compose doc.md --show
```

### Arguments

- `[INPUT]`: Markdown file path. Use `-` for stdin. If omitted, reads stdin when piped; otherwise errors.

### Options

- `--state <JSON>`: external state merged into transform state; must parse as a JSON object.
- `--output <markdown|text|html|json|ast|auto>`: output format (default: `markdown`).
- `--show`: open output via temp artifact.

### Output Behavior

**Default (`--output markdown`)**

- Prints composed markdown content.
- Frontmatter is consumed as pipeline input and not included in composed markdown output.

**`--output auto`**

- Treated the same as markdown for compose.

**`--output html|json`**

- Emits HTML or AST JSON from the composed document.

**`--show` behavior**

- For markdown/auto: prints composed content and opens markdown artifact.
- For html/json: opens artifact instead of printing to stdout.

### Transform Context

- If `[INPUT]` is a file path, compose sets source-file context for relative transclusion resolution.
- If input is stdin (`-` or piped with no input arg), source-file-relative path resolution is not available.

### Validation and Errors

- Invalid JSON in `--state` returns an error.
- Non-object JSON (array/string/number/etc.) in `--state` returns an error.
- Transform failures return non-zero exit with error details.

## Lessons Learned

- `compose` defaults to markdown because composed document output is the primary workflow.
- Frontmatter acts as transform configuration and is intentionally stripped from markdown compose output.
- Explicit `--state` is best for script-driven parameterization.

## Issues

- CLI currently discards the detailed transform report that library callers can capture.
