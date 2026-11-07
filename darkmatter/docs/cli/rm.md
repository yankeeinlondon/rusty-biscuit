## Overview

The `rm` command removes one or more frontmatter values from a markdown document, each named by a [property path](./index.md#property-paths), and saves the result in place.

## Usage

```bash
# Remove a single property
md rm doc.md draft

# Remove multiple properties
md rm doc.md draft wip temp

# Remove a nested value, an array element, a key with a dot in its name
md rm doc.md style.page.max-width
md rm doc.md items.0
md rm doc.md 'a\.b'

# Verbose output (to stderr)
md rm doc.md draft -v

# Report the result as data (to stdout)
md rm doc.md draft --output json
```

## Arguments

- `<INPUT>`: Markdown file path (supports `@` file references). `md rm` edits a file in place, so it does not read stdin.
- `<PROP>...`: One or more [property paths](./index.md#property-paths) to remove (required). A malformed path such as `a..b` is a usage error with exit status 2, before the document is read.

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

`removed` holds the paths as you wrote them, in the order you wrote them.
`remaining` lists the surviving **top-level** keys in document order, whatever
depth was removed.

The object gains `outcome` and `diagnostics` only when the run has a warning. A
failure has no result, so it is
`{ "outcome": "failure", "diagnostics": [...], "value": null }`. See
[Data Contracts](./data-contracts.md#other-commands).

## Validation and Errors

`md rm` plans every removal against the original document, then removes. If anything is wrong with any path, the whole command is canceled with exit status 1 and the file is not written:

- A requested path does not exist. Every missing path is reported.
- A path has an index out of range, or goes through text or `null`.
- Two paths name the same value, or one contains the other (`md rm doc.md author author.name`).

`md rm` stays structural: it reads no schema, so it may remove a required key.

**Array indexes refer to the original array**, so the order you write them in does not matter:

```sh
# on items: [a, b, c, d], both leave [b, d]
md rm doc.md items.0 items.2
md rm doc.md items.2 items.0
```

**Removing the last child leaves its parent**, because a path command removes only what you named: `md rm doc.md style.page.max-width` can leave `style: { page: {} }`, and removing the last element of an array leaves `[]`. Name the parent to remove it too.

## Formatting Is Preserved

Only the removed values' lines change. Every other byte of the document
stays exactly as authored: indentation, comments, quoting, block scalars,
line endings, and the body. When the frontmatter cannot be edited in place
(its root mapping is indented, or the path goes inside a non-empty flow
collection), `rm` falls back to rewriting the whole frontmatter block in
Darkmatter's standard YAML style. See
[Editing nested values](../topics/frontmatter.md#editing-nested-values).

## Lessons Learned

- Silent-by-default keeps `rm` well-behaved in scripts.
- Data outputs support programmatic consumption in pipelines and CI.
- Key order is preserved in the resulting file thanks to insertion-order frontmatter storage.

## Migrating older calls

An ordinary call such as `md rm doc.md draft` is unchanged. These differ:

| Call | Before | Now |
|---|---|---|
| `md rm doc.md a.b` on a top-level key named `a.b` | removed it | `md rm doc.md 'a\.b'`; `a.b` means `b` inside `a` |
| `md rm doc.md draft draft` | removed `draft` | canceled: both paths name the same value |
| a key name holding `\\` or `\.` | taken literally | write each backslash as `\\`, as in [Spelling a path](./index.md#spelling-a-path) |

## Removed spellings

| Removed | Use instead |
|---|---|
| `--json` | `--output json` |
