# Darkmatter CLI

Binary: `md`

A themed markdown renderer for the terminal and browser.

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
# Render markdown to terminal
md README.md

# Pipe from stdin
cat README.md | md
echo "# Hello\n\nWorld" | md
```

### Output Formats

```bash
# HTML output
md README.md --html > output.html

# Open in browser
md README.md --show-html

# MDAST JSON (abstract syntax tree)
md README.md --ast

# Table of contents
md README.md --toc
md README.md --toc --json
```

### Document Cleanup

```bash
# Normalize formatting (stdout)
md README.md --clean

# Clean and save back
md README.md --clean-save
```

### Document Comparison

```bash
# Show structural diff
md original.md --delta updated.md
md original.md --delta updated.md --json
```

### Theming

```bash
# List themes
md --list-themes

# Apply theme
md README.md --theme dracula

# Separate prose and code themes
md README.md --theme nord --code-theme monokai
```

### Rendering Options

```bash
# Line numbers in code blocks
md README.md --line-numbers

# Disable images
md README.md --no-images

# Render mermaid diagrams as images
md README.md --mermaid

# Verbose output
md README.md -v    # INFO
md README.md -vv   # DEBUG
md README.md -vvv  # TRACE
```

## All Options

Run `md --help` for the complete option list.

## Library

For programmatic access, see the [darkmatter-lib](../lib/) library.
