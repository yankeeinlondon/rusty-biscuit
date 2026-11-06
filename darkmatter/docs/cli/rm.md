## Overview

The `rm` command removes one or more frontmatter properties from a markdown document and saves the result in place.

## Usage

```bash
# Remove a single property
md rm doc.md draft

# Remove multiple properties
md rm doc.md draft wip temp

# Verbose output (to stderr)
md rm doc.md draft -v

# Report the result as data (to stdout)
md rm doc.md draft --output json
```

## Arguments

- `<INPUT>`: Markdown file path (supports `@` file references). `md rm` edits a file in place, so it does not read stdin.
- `<PROP>...`: One or more frontmatter property names to remove (required).

## Options

- `--output <terminal|plain|markdown|markdown-plus|html|browser|json|json5|yaml|toml>`: What to print (default: `terminal`). A data output reports the `removed`, `remaining`, and `filename` fields.
- `--width <N|N%|auto|source|unlimited>`: Accepted like on every command. It has no effect, because `md rm` prints no prose.

## Output Behavior

`md rm` accepts both output families. See
[Output and Width](./output-and-width.md).

**`terminal` (default) and `plain`: silent**

- No output on success. Exit status 0.
- If any requested property is not found, returns exit status 1 with an error
  message on stderr.

**`markdown`, `markdown-plus`, `html`: one sentence**

- Asking for a document form by name prints one sentence on stdout:

  ```text
  $ md rm doc.md wip --output markdown
  Removed `wip` from the frontmatter. Remaining: `title`, `draft`.
  ```

**Verbose (`-v`)**

- In a formatted output, also prints a human-readable summary to stderr:
  `- removed the wip property from frontmatter (remaining: title, draft)`

**Data outputs (`json`, `json5`, `yaml`, `toml`)**

- Print one value to stdout and nothing to stderr:

```json
{
  "removed": ["draft"],
  "remaining": ["title", "author"],
  "filename": "/path/to/doc.md"
}
```

The object gains `outcome` and `diagnostics` only when the run has a warning. A
failure has no result, so it is
`{ "outcome": "failure", "diagnostics": [...], "value": null }`. See
[Data Contracts](./data-contracts.md#other-commands).

## Validation and Errors

- If any requested property does not exist in frontmatter, the command fails with exit status 1 and reports the missing properties.
- The file is only written if all requested properties are found and removed.

## Formatting Is Preserved

Only the removed properties' lines change. Every other byte of the document
stays exactly as authored: indentation, comments, quoting, block scalars,
line endings, and the body. When the frontmatter cannot be edited in place
(for example, its root mapping is indented), `rm` falls back to rewriting the
whole frontmatter block in Darkmatter's standard YAML style.

## Lessons Learned

- Silent-by-default keeps `rm` well-behaved in scripts.
- Data outputs support programmatic consumption in pipelines and CI.
- Key order is preserved in the resulting file thanks to insertion-order frontmatter storage.

## Removed spellings

| Removed | Use instead |
|---|---|
| `--json` | `--output json` |
