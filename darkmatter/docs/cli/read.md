## Overview

The `read` command renders markdown documents for terminal viewing or export formats.

`read` is functionally equivalent to running `md` with no subcommand:

- `md read README.md`
- `md README.md`

Use `read` when you want explicit command intent in scripts and docs.

## Reporting

### Usage

```bash
# Explicit read command
md read README.md

# Equivalent implicit form
md README.md

# Read from stdin
md read -
cat README.md | md read

# Select output format
md read README.md --output markdown
md read README.md --output html
md read README.md --output json

# Open selected output via temp artifact
md read README.md --output html --show
```

### Arguments

- `[INPUT]`: Markdown file path. Use `-` for stdin. If omitted, reads stdin when piped; otherwise errors.

### Options

- `--output <auto|markdown|text|html|json|ast>`: Output format (default: `auto`).
- `--show`: Write output to a temp file and open it with the system default app.

### Global Flags Relevant to `read`

These are top-level flags (not read-specific) but affect `read` behavior:

- `--theme <NAME>`: prose theme
- `--code-theme <NAME>`: code theme override
- `--line-numbers`: include code line numbers in terminal rendering
- `--mermaid`: render mermaid diagrams as images in terminal mode when supported

### Output Behavior

**`--output auto` (default)**

- If stdout is a TTY: renders styled terminal output.
- If stdout is not a TTY: emits markdown text.

**`--output markdown|html|json`**

- Emits selected format to stdout.

**`--show` behavior**

- In explicit format modes, output is opened from a temp file; the artifact is not printed to stdout.
- In `auto` mode on a TTY, terminal rendering is still printed, and markdown artifact is opened.
- `MD_DRY_RUN=1` writes the temp artifact but skips launching the viewer.

## Lessons Learned

- `auto` mode gives good defaults for both interactive terminals and pipelines.
- `read` and implicit `md [INPUT]` share the same core path, so behavior remains aligned.
- Theme and rendering controls are intentionally global CLI flags.

## Issues

- `read` has no command-local theme flags; theming is configured at root CLI level.
- `--show` behavior differs slightly in `auto` TTY mode versus explicit format modes.
