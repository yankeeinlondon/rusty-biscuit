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
- `--indent <#>`: Enforce a nested-list indentation step of 2, 4, or 8 spaces per level. Default: 4. For an eight-space step, cleanup uses CommonMark-valid marker padding on parent items when needed to preserve the same list tree.

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
- `--mermaid`: in terminal output, render Mermaid diagrams in automatic mode: an
  image where the terminal shows images, otherwise a text drawing when the
  diagram has one and it fits, otherwise the highlighted code block (see
  [Mermaid in the terminal](#mermaid-in-the-terminal) below). HTML, Markdown,
  and JSON output ignore it.
- `--feature <NAME>`: turn on a named feature (see [Features](#features) below)

### Features

`--feature <NAME>` turns on an opt-in by name. It is accepted at the top level
(`md --feature NAME doc.md`, before a subcommand) and on `render`
(`md render doc.md --feature NAME`); both lists are combined. The top-level
form also reaches `md --feature NAME compose doc.md --output html`.

- Repeatable and comma-separated: `--feature a --feature b` is the same as
  `--feature a,b`.
- Names are case-insensitive, and repeating a name is harmless.
- An unknown name, including an empty one (`--feature a,,b` or
  `--feature ""`), is a usage error (exit code 2) that lists the valid names.
  Shell completion offers them.

| Name | Effect |
| --- | --- |
| `embed-mermaid-font` | In HTML output, embed the font static-SVG Mermaid diagrams were measured in, once per page (see [Mermaid rendering](../rendering/mermaid.md#embedded-font)). |
| `no-mermaid-fallbacks` | With `--mermaid`, never draw a diagram as text: a terminal without images shows its code block. |
| `no-mermaid-partial-fallbacks` | With `--mermaid`, never draw a diagram as text when the drawing may miss details the image would show. Every diagram family's drawing is partial today, so this currently acts like `no-mermaid-fallbacks`. |
| `no-mermaid-partial-warning` | With `--mermaid`, hide the note that follows a partial text drawing. |
| `no-mermaid-rendering` | Every Mermaid fence stays a highlighted code block, in every output. |

When two fallback features overlap, the more restrictive wins:
`no-mermaid-fallbacks,no-mermaid-partial-fallbacks` never draws text. The
three fallback features act only with `--mermaid`; without it they are
accepted and change nothing.

`embed-mermaid-font` acts only on Mermaid diagrams emitted as static SVG.
`md`'s HTML output does not produce static SVG today: its Mermaid diagrams
are interactive `<pre class="mermaid">` blocks, so the page is byte for byte
the same with and without the feature. Terminal, Markdown, and JSON output
ignore it. No frontmatter key turns it on.

```bash
md --output html --feature embed-mermaid-font notes.md
md render notes.md --output html --feature EMBED-MERMAID-FONT   # same
```

`no-mermaid-rendering` wins over every other Mermaid option on the command:
`--mermaid`, static SVG, and `embed-mermaid-font` (which then has nothing to
embed). Terminal and HTML output show each fence as a code block with
[Mermaid syntax highlighting](../rendering/mermaid.md#syntax-highlighting),
and HTML output loads no Mermaid.js. Markdown and JSON output keep the
` ```mermaid ` fence, as they always do.

```bash
md --mermaid --feature no-mermaid-rendering notes.md   # code blocks, not diagrams
md render notes.md --output html --feature no-mermaid-rendering   # no Mermaid.js
```

### Mermaid in the terminal

`md --mermaid` renders each Mermaid fence in automatic mode:

- **The terminal shows images:** the diagram is drawn as an image.
- **It does not:** a diagram that can be drawn as text (a flowchart or a
  sequence diagram, for example) and fits the width is drawn as a `txt` block,
  followed by a note when the drawing may miss details. Any other diagram
  (a pie chart, for example) stays its highlighted `mermaid` code block.
- **`TERMINAL_IMAGES=0`:** every fence stays a code block.

````text
$ NO_COLOR=1 md --mermaid flow.md          # in a terminal without image support
```txt
┌───────┐     ┌──────┐
│       │     │      │
│ Alpha ├────►│ Beta │
│       │     │      │
└───────┘     └──────┘
```

▐   Note: this textual representation is provided because the current terminal
▐   settings do not allow for a graphical image to be used. …
````

With color, the drawing sits in a highlighted code panel instead of between
fences.

The fallback features above choose whether text may replace the code block
and whether the note is shown. Without a fallback feature, the
`MERMAID_FALLBACK` environment variable applies: `code-block` (any case) means
never draw text. A feature always wins over the variable.

```bash
MERMAID_FALLBACK=code-block md --mermaid flow.md          # source, not a drawing
md --mermaid render flow.md --feature no-mermaid-partial-warning   # drawing, no note
```

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
