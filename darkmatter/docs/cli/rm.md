## Overview

The `rm` command removes one or more frontmatter properties from a markdown document and saves the result in place.

## Usage

```bash
# Remove a single property
md rm doc.md draft

# Remove multiple properties
md rm doc.md draft wip temp

# Verbose output (to stderr)
md -v rm doc.md draft

# JSON output (to stdout)
md rm doc.md draft --json
```

## Arguments

- `<INPUT>`: Markdown file path (supports `@` file references).
- `<PROP>...`: One or more frontmatter property names to remove (required).

## Options

- `--json`: Output result as JSON with `removed`, `remaining`, and `filename` fields.

## Output Behavior

**Default (silent)**

- No output on success. Exit code 0.
- If any requested property is not found, returns exit code 1 with an error message to stderr.

**Verbose (`-v`)**

- Prints a human-readable summary to stderr:
  `- removed the "prop" property from frontmatter (remaining: key1, key2)`

**JSON (`--json`)**

- Outputs structured JSON to stdout:

```json
{
  "removed": ["draft"],
  "remaining": ["title", "author"],
  "filename": "/path/to/doc.md"
}
```

## Validation and Errors

- If any requested property does not exist in frontmatter, the command fails with a non-zero exit code and reports the missing properties.
- The file is only written if all requested properties are found and removed.

## Lessons Learned

- Silent-by-default keeps `rm` well-behaved in scripts.
- `--json` output supports programmatic consumption in pipelines and CI.
- Key order is preserved in the resulting file thanks to insertion-order frontmatter storage.
