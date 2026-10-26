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

## Formatting Is Preserved

Only the property being set is rewritten. Every other byte of the document
stays exactly as authored: indentation, comments, quoting, block scalars,
line endings, and the body. A new property is appended as the last
frontmatter entry; an existing one is replaced where it stands.

```yaml
# before                         # after: md set doc.md start_phase 6 --save
phase: 1  # first phase          phase: 1  # first phase
files:                           files:
    - a.rs                           - a.rs
                                 start_phase: 6
```

The new value itself is written in Darkmatter's standard YAML style. When the
frontmatter cannot be edited in place (for example, its root mapping is
indented), `set` falls back to rewriting the whole frontmatter block in the
standard style; `md clean --save` fixes the indentation so later edits stay
in place.

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
- Re-serializing the whole frontmatter on every `set` turned a one-property change into a diff of every list in the file; editing only the changed property keeps scripted edits reviewable.
