## Overview

The `render` command renders markdown documents for terminal viewing or export formats.

`render` is functionally equivalent to running `md` with no subcommand:

- `md render README.md`
- `md README.md`

Use `render` when you want explicit command intent in scripts and docs.

## Reporting

### Usage

```bash
# Explicit render command
md render README.md

# Equivalent implicit form
md README.md

# Render from stdin
md render -
cat README.md | md render

# Select output format
md render README.md --output markdown
md render README.md --output html
md render README.md --output json

# Open selected output via temp artifact
md render README.md --output html --show

# Override list indentation
md render README.md --indent 2

# Choose how code blocks pick their light/dark variant
md render README.md --code-block dark    # always a dark code panel
md render README.md --code-block same    # match the terminal's mode
```

### Arguments

- `[INPUT]`: Markdown file path (supports `@` file references). Use `-` for stdin. If omitted, reads stdin when piped; otherwise errors.

### Options

- `--output <auto|markdown|text|html|json|ast>`: Output format (default: `auto`).
- `--show`: Write output to a temp file and open it with the system default app.
- `--indent <#>`: Normalize nested list indentation width (2 or 4 spaces per level). Default: 4. `8` is rejected because eight-space nesting is not CommonMark-portable for narrow markers (`-`, `*`, `+`, single-digit ordered).

### Global Flags Relevant to `render`

These are top-level flags (not render-specific) but affect `render` behavior:

- `--theme <NAME>`: prose theme
- `--code-theme <NAME>`: code theme override (selects *which* theme pair, e.g. `dracula`)
- `--code-block <inverse|dark|light|same>`: selects *which variant* of the code
  theme a code block uses relative to the page color mode (default `inverse`):
    - `inverse` (default): the opposite variant from the terminal, so the code
      panel contrasts with the page (a dark terminal gets a light panel, and a
      light terminal gets a dark panel).
    - `dark`: always the dark variant.
    - `light`: always the light variant.
    - `same`: match the terminal's own mode.
  The panel's light/dark choice is derived from the terminal (the same source as
  the page), so it stays consistent regardless of environment color detection.
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
- `render` and implicit `md [INPUT]` share the same core path, so behavior remains aligned.
- Theme and rendering controls are intentionally global CLI flags.

## Issues

- `render` has no command-local theme flags; theming is configured at root CLI level.
- `--show` behavior differs slightly in `auto` TTY mode versus explicit format modes.
