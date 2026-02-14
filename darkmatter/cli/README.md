# Darkmatter CLI

Binary: `md`

A themed markdown renderer for terminal and browser workflows with markdown, HTML, and AST JSON output modes.

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
- `html`
- `json` (alias: `ast`)

```bash
md README.md --output markdown
md README.md --output text
md README.md --output html > output.html
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
md delta original.md updated.md -v
```

### Document Cleanup

```bash
# Normalize formatting (output to stdout)
md README.md --clean

# Clean and save back to file
md README.md --clean-save
```

### Frontmatter Manipulation

```bash
# Merge JSON into frontmatter (JSON wins on conflicts)
md README.md --fm-merge-with '{"version": "2.0"}'

# Set default values (document wins on conflicts)
md README.md --fm-defaults '{"draft": false}'
```

### Theming and Rendering Options

```bash
# List available themes
md --list-themes

# Apply theme (affects both prose and code)
md README.md --theme dracula

# Separate prose and code themes
md README.md --theme nord --code-theme monokai

# Line numbers in code blocks
md README.md --line-numbers

# Render mermaid diagrams as images
md README.md --mermaid
```

### Verbosity

```bash
md README.md -v      # INFO level
md README.md -vv     # DEBUG level
md README.md -vvv    # TRACE level
md README.md -vvvv   # TRACE with file/line info
```

### Shell Completions

Enable tab completions that filter to `.md` and `.dm` files (including one directory level deep):

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
