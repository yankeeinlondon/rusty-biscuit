# Darkmatter CLI

Binary: `md`

A themed markdown renderer for the terminal and browser with syntax highlighting, image support, and document analysis tools.

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
# Render markdown to terminal with syntax highlighting
md README.md

# Pipe from stdin
cat README.md | md
echo "# Hello\n\nWorld" | md

# Explicit stdin
md -
```

### Output Formats

```bash
# HTML output (standalone with embedded styles)
md README.md --html > output.html

# Open in browser
md README.md --show-html

# MDAST JSON (abstract syntax tree)
md README.md --ast

# Table of contents (tree format)
md README.md --toc

# Table of contents with filename
md README.md --toc-filename

# Table of contents as JSON
md README.md --toc --json
```

### Document Cleanup

```bash
# Normalize formatting (output to stdout)
md README.md --clean

# Clean and save back to file
md README.md --clean-save
```

Cleanup operations:

- Inject blank lines between block elements
- Align table columns
- Normalize whitespace

### Document Comparison

```bash
# Show structural diff between files
md original.md --delta updated.md

# JSON format for programmatic use
md original.md --delta updated.md --json

# Verbose with visual diff
md original.md --delta updated.md -v
```

Delta analysis includes:

- Change classification (identical, minor edit, major rewrite, etc.)
- Content additions/removals/modifications
- Section movements
- Frontmatter changes
- Broken link detection

### Theming

```bash
# List available themes
md --list-themes

# Apply theme (affects both prose and code)
md README.md --theme dracula

# Separate prose and code themes
md README.md --theme nord --code-theme monokai
```

Available themes: `github`, `one-half`, `base16-ocean`, `gruvbox`, `solarized`, `nord`, `dracula`, `monokai`, `vs-dark`

### Frontmatter Manipulation

```bash
# Merge JSON into frontmatter (JSON wins on conflicts)
md README.md --fm-merge-with '{"version": "2.0"}'

# Set default values (document wins on conflicts)
md README.md --fm-defaults '{"draft": false}'
```

### Rendering Options

```bash
# Line numbers in code blocks
md README.md --line-numbers

# Disable image rendering (show placeholders)
md README.md --no-images

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

Run `md --completions <SHELL>` to see the setup command for your shell.

## All Options

```
Usage: md [OPTIONS] [INPUT]

Arguments:
  [INPUT]  Input file path (reads from stdin if not provided)

Options:
      --theme <THEME>           Theme for prose content
      --code-theme <THEME>      Theme for code blocks
      --list-themes             List available themes
      --clean                   Clean up markdown formatting (stdout)
      --clean-save              Clean up and save back to file
      --html                    Output as HTML
      --show-html               Generate HTML and open in browser
      --ast                     Output MDAST JSON
      --toc                     Show table of contents
      --toc-filename            Show TOC with filename in header
      --delta <FILE>            Compare with another markdown file
      --json                    Output as JSON (for --toc and --delta)
      --fm-merge-with <JSON>    Merge JSON into frontmatter
      --fm-defaults <JSON>      Set default frontmatter values
      --line-numbers            Include line numbers in code blocks
      --no-images               Disable image rendering
      --mermaid                 Render mermaid diagrams as images
      --completions <SHELL>     Generate shell completions setup command
  -v, --verbose...              Increase verbosity
  -h, --help                    Print help
  -V, --version                 Print version
```

## Features

| Feature | Description |
|---------|-------------|
| **Syntax highlighting** | Language-aware code blocks with 200+ grammars |
| **Theme support** | 9 theme pairs with automatic light/dark detection |
| **Terminal images** | Inline images in Kitty, iTerm2, and sixel terminals |
| **Mermaid diagrams** | Render flowcharts, sequences, etc. as images |
| **GFM tables** | GitHub-flavored tables with box-drawing |
| **Hyperlinks** | OSC 8 terminal hyperlinks in supported terminals |
| **Document diffing** | Structural comparison with change analysis |
| **TOC extraction** | Hierarchical heading structure |
| **Cleanup tools** | Normalize formatting, align tables |
| **Shell completions** | Bash, Zsh, Fish, PowerShell with markdown file filtering |

## Mermaid Support

When `--mermaid` is enabled:

1. Validates diagram size (max 10KB)
2. Executes local `mmdc` CLI with dark theme
3. Displays PNG image via viuer
4. Falls back to syntax-highlighted code block on failure

**Requirements**: Install mermaid CLI globally:

```bash
npm install -g @mermaid-js/mermaid-cli
```

Or let the tool use `npx` to install temporarily (slower first run).

## Library

For programmatic access, see the [darkmatter-lib](../lib/) library.

```rust
use darkmatter_lib::markdown::{Markdown, output::{TerminalOptions, write_terminal}};

let md: Markdown = "# Hello\n\nWorld".into();
write_terminal(&mut std::io::stdout(), &md, TerminalOptions::default())?;
```
