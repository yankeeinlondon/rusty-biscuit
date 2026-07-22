# Darkmatter CLI

Binary: `md`

A themed markdown renderer for terminal and browser workflows with markdown, markdown-plus, HTML, and AST JSON output modes.

## Binary Overview

The user-facing CLI surface is defined in `src/args/`: `cli.rs` holds global
flags, `command.rs` holds subcommands, `target.rs` holds command targets,
`enums.rs` holds output/format enums, `wrappers.rs` holds CLI conversion types,
`parsers.rs` holds value parsers, and `completion.rs` holds shell completion
helpers.

Runtime dispatch lives in `src/commands/mod.rs`, with command-specific
implementations in `commands/render.rs`, `commands/clean.rs`,
`commands/validate.rs`, `commands/graph.rs`, `commands/compose.rs`,
`commands/frontmatter.rs`, `commands/hash.rs`, and `commands/code_block.rs`.
Shared input loading is in `src/io/`, output artifact handling is in
`src/artifact.rs`, terminal rendering setup is in `src/render.rs`, and CLI flag
precedence is lowered to `darkmatter::style::CliStyleClaims` in
`src/style_claims.rs`.

## Installation

```bash
# From source
cargo install --path .

# Or with just
just -f darkmatter/justfile install
```

## Usage

### Basic Rendering

```bash
# Render markdown using auto mode
md README.md

# Pipe from stdin
cat README.md | md
echo "# Hello\n\nWorld" | md

# Explicit stdin
md -
```

### Output Modes

Use a single `--output <OUTPUT>` switch for render format selection:

- `auto` (default): render ANSI terminal output on TTY, markdown text on non-TTY
- `markdown` (alias: `text`)
- `markdown-plus`: markdown with disclosure blocks emitted as inline HTML `<details>`/`<summary>`
- `html` (alias: `browser`)
- `json` (alias: `ast`)

```bash
md README.md --output markdown
md README.md --output text
md README.md --output markdown-plus
md README.md --output html > output.html
md README.md --output browser > output.html
md README.md --output json
md README.md --output ast
```

### Show Rendered Output

`--show` writes the selected output into a temp file and opens it with the system default app.

```bash
md README.md --output html --show
md README.md --output markdown --show
md README.md --output json --show
```

In `--output auto` mode on a TTY, `md` renders ANSI output to the terminal and also opens markdown in a temp file.

### TOC and Delta Subcommands

```bash
# Table of contents
md toc README.md
md toc README.md --json

# Document comparison
md delta original.md updated.md
md delta original.md updated.md --json
md -v delta original.md updated.md
```

### Dependency Graph

```bash
# Visualize a file's dependency graph
md graph README.md

# Recursively follow transclusions
md graph README.md --follow

# Validate references inline
md graph README.md --validate

# Both together
md graph README.md --follow --validate
```

Exit codes: `0` success/valid, `1` runtime error, `2` validation found errors.

### Compose Pipeline

```bash
# Compose a document through the markdown pipeline
md compose README.md

# Provide default values (fills null/missing keys, preserves existing)
md compose README.md --state '{"name":"Alice","env":"prod"}'

# JSON5 is also accepted (unquoted keys, trailing commas)
md compose README.md --state '{name: "Alice", env: "prod"}'

# Override values with shorthand setters
md compose README.md iteration=1 draft=false name=Alice

# Shorthand setters can appear before the input path too
md compose iteration=1 README.md

# Include frontmatter in output
md compose README.md --fm

# Compose from stdin
echo "# Hello {{ name }}" | md compose - --state '{"name":"Alice"}'

# Render compose output as HTML or JSON
md compose README.md --output html
md compose README.md --output json

# Adjust shell command timeouts during compose
md compose README.md --timeout 3

# Convert timed out shell commands into empty strings and warnings
md compose README.md --timeout 3 --allow-shell-timeout

# Report compose-tree shell commands without executing them
md compose README.md --shell

# Allow non-object ctx frontmatter (downgrades error to warning)
md compose README.md --allow-ctx-override

# Emit structured performance report to stderr
md compose README.md --perf
```

During `compose`, Darkmatter supports both body `::shell ...` directives and top-level frontmatter `$(...)` expressions. Both use the same whitelist/blacklist and approval flow. Frontmatter shell expansion stores trimmed `stdout` only; body shell expansion stores combined `stdout` + `stderr`. Use `--shell` to inspect the shell commands discovered across the compose tree before approving or executing them.

### Frontmatter Set

```bash
# Set a property (outputs modified document to stdout, file unchanged)
md set doc.md title "New Title"

# Save in place (no output)
md set doc.md title "New Title" --save

# Chain via pipes
md set doc.md title "New Title" | md set - version 2

# Compose and set in a pipeline
md compose "@prompts/feature.md" --fm | md set - feature "auth" | md set - base_dir "./features/auth"
```

### Frontmatter Remove

```bash
# Remove a property (saves in place, silent on success)
md rm doc.md draft

# Remove multiple properties
md rm doc.md draft wip temp

# Verbose output
md -v rm doc.md draft

# JSON output
md rm doc.md draft --json
```

### Frontmatter Get

```bash
# Single property
md get doc.md title

# Multiple properties
md get doc.md title author tags

# Output formats
md get doc.md title --yaml
md get doc.md title --json5
```

### Document Cleanup

```bash
# Normalize formatting (output to stdout)
md clean README.md

# Normalize formatting and collapse editor/LLM fixed-column prose wrapping
md clean README.md

# Collapse incidental wrapping, then re-wrap prose to 80 display columns
md clean README.md --fixed-width 80

# Preserve source single newlines while keeping the older cleanup behavior
md clean README.md --ignore-incidental-newlines

# Normalize formatting and force 4-space nested list indentation
md clean README.md --indent 4

# Clean from stdin
cat README.md | md clean

# Emit the v1 diagnostic envelope instead of Markdown
md clean README.md --json

# Control the effective frontmatter schema
md clean README.md --schema docs.schema.yaml
md clean README.md --baseline-schema project-baseline.yaml
md clean README.md --no-baseline-schema
md clean README.md --no-trigger-schemas

# Save cleaned file in place and report delta-style changes
md clean README.md --save

# Include visual diff output in save mode
md clean README.md --save -v

# Shorthand: top-level clean-and-save
md README.md --save
```

By default, `md clean` collapses incidental single newlines in prose before
running the rest of cleanup. Blank lines, fenced and indented code blocks,
tables, HTML blocks, transclusion directives, list markers, and blockquote
prefixes are preserved. Prose includes paragraphs inside ordered, unordered,
and task-list items at every nesting depth: default cleanup removes source-only
continuation indentation, while `--fixed-width <#>` unwraps the complete item
paragraph and emits list-aware hanging continuation prefixes. For lists inside
blockquotes, those prefixes retain both the quote and list containers. Use
`--fixed-width <#>` when you want canonical cleanup followed by prose wrapping
to a target display width, or
`--ignore-incidental-newlines` when source line breaks must remain unchanged.
`--fixed-width` and `--ignore-incidental-newlines` conflict because fixed-width
reflow first needs the incidental source wrapping removed.

Deterministic YAML frontmatter repairs are also enabled by default. Analysis is
limited to the frontmatter block; YAML fences in the body are never inspected.
Without `--save`, the repaired document is printed and the input file remains
unchanged. With `--save`, accepted repairs are written in place. Report-only
findings are rendered as suggestions on stderr and still exit `0`; there is no
`md clean --strict` in v1.

Schema precedence is baseline → matching repository triggers → document
`$schema`. `--baseline-schema` replaces the default Darkmatter baseline,
`--no-baseline-schema` removes it, `--no-trigger-schemas` disables trigger
discovery, and `--schema` replaces the document `$schema` layer. Stdin has no
document path, so trigger discovery is always inert for stdin, while explicit
schema and baseline flags remain active.

`--json` writes the version-1 envelope as the sole stdout payload: `version`,
structured `source` and `frontmatter`, document-position `diagnostics`, the
repairs actually `applied`, and whole-document `changed`. It suppresses the
cleaned Markdown, delta report, and human stderr suggestions. Success is exit
`0`; unrepaired invalid YAML remains exit `1` but still emits the envelope with
a `yaml.parse` diagnostic on stdout, leaves stderr empty, and does not modify a
save target. Diagnostic offsets are zero-based, end-exclusive document byte
offsets; lines and byte columns are one-based. See the
[clean command guide](../docs/cli/clean.md#version-1-json-envelope) for the
exact JSON example and full flag behavior.

Frontmatter manipulation is available through `md set` (modify individual properties) and `md compose --fm` (output frontmatter with composed content). The `--state` flag on `compose` fills in null/missing frontmatter keys with default values.

### Theming and Rendering Options

```bash
# List available themes
md --list-themes

# Apply theme (affects both prose and code)
md README.md --theme dracula

# Separate prose and code themes
md README.md --theme nord --code-theme monokai

# Control the code block's light/dark variant (default: inverse)
md README.md --code-block dark      # always a dark code panel
md README.md --code-block light     # always a light code panel
md README.md --code-block same      # match the terminal's mode

# Line numbers in code blocks
md README.md --line-numbers

# Render mermaid diagrams as images
md README.md --mermaid
```

A theme name (`--theme`, `--code-theme`) is mode-agnostic — the concrete
light/dark variant is chosen from the terminal color mode. By default code blocks
use the *inverted* mode (`--code-block inverse`): a light code panel on a dark
page, and vice versa, so the code contrasts against the page; prose follows the
real mode. This inversion default applies to **both terminal and HTML** output
(`--output html`), so the two targets agree. `--code-block <inverse|dark|light|same>`
overrides the variant for **terminal** rendering — `dark`/`light` pin the
variant, `same` matches the terminal (HTML currently always uses the inverse
default). The variant is derived from the terminal (the same source as the page),
so it is consistent regardless of environment color detection.
Every `ThemePair` is a (light theme, dark theme) couple, so the inversion applies
to all of them. See
[Code Highlighting](../docs/rendering/code-highlighting.md).

### Verbosity

```bash
md README.md -v      # INFO level
md README.md -vv     # DEBUG level
md README.md -vvv    # TRACE level
md README.md -vvvv   # TRACE with file/line info
```

### Shell Completions

Enable tab completions for markdown files (`.md`, `.dm`) and directory traversal:

```bash
# Bash (add to ~/.bashrc)
source <(COMPLETE=bash md)

# Zsh (add to ~/.zshrc)
source <(COMPLETE=zsh md)

# Fish (add to ~/.config/fish/config.fish)
COMPLETE=fish md | source

# PowerShell (add to $PROFILE)
$env:COMPLETE = "powershell"; md | Out-String | Invoke-Expression; Remove-Item Env:\COMPLETE
```

Run `md --completions <SHELL>` to print the setup command for your shell.

## Notes

- `TERMINAL_IMAGES` controls terminal image behavior:
  - truthy (`true`, `1`, `yes`, `on`) forces protocol image output attempts
  - falsy (`false`, `0`, `no`, `off`) disables image protocol output
  - unset/invalid uses capability auto-detection
- Removed legacy flags: `--html`, `--show-html`, `--ast`, top-level `--json`, `--no-images`, top-level `--toc`, and top-level `--delta`.

## Library

For programmatic access, see [darkmatter-lib](../lib/).

```rust
use darkmatter::markdown::{Markdown, output::{TerminalOptions, write_terminal}};

let md: Markdown = "# Hello\n\nWorld".into();
write_terminal(&mut std::io::stdout(), &md, TerminalOptions::default())?;
```
