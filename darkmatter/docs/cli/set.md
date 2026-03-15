## Overview

The `set` command modifies a frontmatter property on a markdown document.

By default the modified document is written to stdout without changing the source file, making it suitable for piped workflows. The `--save` flag writes the change back to the file in place.

## Usage

```bash
# Set a property and output to stdout (file is not modified)
md set doc.md title "New Title"

# Set a property and save in place (no output)
md set doc.md title "New Title" --save

# Set from stdin (pipe-friendly)
cat doc.md | md set - title "New Title"

# Chain multiple set operations via pipes
md set doc.md title "New Title" | md set - version 2 | md set - draft true

# JSON values are auto-detected
md set doc.md tags '["rust","markdown"]'
md set doc.md count 42
md set doc.md draft false
```

## Arguments

- `<INPUT>`: Markdown file path (supports `@` file references) or `-` for stdin.
- `<PROP>`: Frontmatter property name to set.
- `<VALUE>`: Value to assign. Parsed as JSON if valid, otherwise treated as a string.

## Options

- `--save`: Write the change back to the source file. Produces no output on success. Not compatible with stdin input.

## Output Behavior

**Default (no `--save`)**

- Prints the full document (frontmatter + body) with the property updated to stdout.
- The source file is not modified.
- Frontmatter key order is preserved from the original document.

**With `--save`**

- Updates the source file in place.
- Produces no output on success (exit code 0).
- Returns a non-zero exit code on failure.

**Stdin (`-`)**

- Modified document is always written to stdout.
- `--save` is not supported with stdin and returns an error.

## Lessons Learned

- Default-to-stdout makes `set` composable in shell pipelines without side effects.
- `--save` is explicit opt-in for mutation, avoiding accidental file changes.
- Key ordering is preserved using an insertion-order map, so frontmatter reads back in the same order it was written.
