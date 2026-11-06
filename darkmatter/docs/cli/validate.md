---
kind: canonical-documentation
---

# `md validate refs`

`md validate refs` checks every reference in a document (links, images,
transclusions, CSS and script imports) and tells you which ones are broken and
exactly where each is written. It reads the document and any documents it
transcludes, and runs nothing.

```sh
md validate refs notes.md               # the report
md validate refs notes.md --output json # the same result as data, for CI
md validate refs notes.md --fragments   # also check #heading links
```

```text
$ md validate refs broken.md
Broken references in broken.md

broken.md
  file not found in hyperlink  `./missing.md`  broken.md:3:5

child.md (transcluded at broken.md:5)
  file not found in image  `./nope.png`  child.md:2:1

Checked 4 references across 2 documents: 2 ok, 2 broken
```

It exits with status 0 when nothing is broken and 1 when something is. Scripts
should read the status, not the report's text.

## Options

| Option | What it does |
|---|---|
| `--fragments` | also check that a `#heading` fragment names a real heading |
| `--remote` | also check remote URLs over the network; without it, a remote URL is listed as not verified |
| `--timeout <SECONDS>` | how long a remote check may take (default: 10) |
| `--fail-fast` | stop at the first broken reference |
| `--show-all` | list every reference, not only the broken ones |
| `--graph <mermaid\|dot>` | print the transclusion graph instead of validating |
| `--output <TARGET>` | what to print; see [Output](#output) |
| `--width <WIDTH>` | see [Output and Width](./output-and-width.md#choosing-a-width) |

### `--fail-fast`

The report stops at the first broken reference, and its summary counts the rest of
what was scanned as ok. The exit status is still 1.

```text
$ md validate refs broken.md --fail-fast
Broken references in broken.md

broken.md
  file not found in hyperlink  `./missing.md`  broken.md:3:5

Checked 4 references across 2 documents: 3 ok, 1 broken
```

### `--show-all`

A section lists every reference with its location, followed by any warnings and the
summary. In a data output the same list is the `references` field.

```text
$ md validate refs notes.md --show-all
All references
  in hyperlink  `https://example.com/docs`  notes.md:8:61
  in image      `./diagram.png`             notes.md:15:1

Remote URL not verified: https://example.com/docs

Checked 2 references across 1 document: all ok
```

### `--graph`

`--graph` prints a diagram of which documents transclude which, in Mermaid or DOT
notation. It is diagram-only: it skips validation, so a document with broken
references still exits 0, and `--show-all` and `--fail-fast` have no effect beside it.

```text
$ md validate refs broken.md --graph mermaid
flowchart TD
    _work_broken_md["broken.md<br/>3 refs"]
    _work_child_md["child.md<br/>1 refs"]
    _work_broken_md --> _work_child_md
```

`--graph` names the diagram's notation, and `--output` still chooses the output:
`markdown` wraps the diagram in a fenced code block, and a data output is
`{ "graph": { "format": "dot", "source": "digraph ..." } }` alone.

## The reference report

This report is shared: `md validate refs` prints it to stdout, `md graph --validate`
prints it after the tree, and `md compose` prints it on stderr when references stop
it. It covers reference problems only.

Problems are grouped under the document that contains them, because that is where
the fix goes: the named document first, then transcluded documents depth-first. A
transcluded document's heading says how it was reached, outermost first:
`(transcluded at types.md:22 → intro.md:4)`.

Each row has a reason, a construct, the reference as written, and a location.

| Reason | Meaning |
|---|---|
| `file not found` | a local target does not exist |
| `no matching heading` | a `#fragment` names no heading in its document |
| `malformed URL` | the URL does not parse |
| `unreachable` | a remote check failed |
| `unsupported scheme` | the URL's scheme cannot be checked |
| `no document context` | a relative reference has no document to resolve against |
| `remote check disabled` | the remote check is turned off |

The construct says what wrote the reference: *in hyperlink*, *in image*, *in
`::file`*, *in `::code`*, *in `::url`*, *in `::toc-linking`*, *in `::file-links`*,
*in `prologue:`*, *in `epilogue:`*, and the HTML forms such as *in `<video>`*.
A detail a reason does not state, such as a remote status or a glob hint, goes on
a dim line under its row.

A summary closes the report: the scope and result, then a tally by reason when there
is more than one, then any warnings and notices.

```text
Checked 3 references across 1 document: 0 ok, 3 broken
  2 × file not found · 1 × no matching heading
```

When nothing is broken, the report is one line, never empty:

```text
Checked 3 references across 1 document: all ok
```

The report is laid out in columns. A reference and a location are never split, since
you may want to copy them. At a width too narrow to hold a row, such as `--width 30`
or a very long reference at `--width 60`, that group's rows stack instead:

```text
d.md
  file not found in hyperlink
    `./missing.md`
    d.md:5:5
```

### Locations

A location always names the source document holding the problem, never a position in
the composed output, so transclusion cannot shift it. Each output carries the same
three facts (path, heading, line and column) in the form it supports:

| Output | A location becomes |
|---|---|
| `terminal` with OSC 8 links | `broken.md:3:5`, linked to the bare `file://` path, with `line`, `column`, and `heading` as link parameters |
| `terminal` without them, and `plain` | the text `broken.md:3:5`; with `-v`, `broken.md:3:5 <file:///work/broken.md>` |
| `markdown`, `markdown-plus` | `[broken.md:3:5](/work/broken.md#broken '{ qp: { line: 3, column: 5 } }')` |
| `html` | `<a href="/work/broken.md?heading=broken&line=3&column=5">` |
| a data output | the diagnostic's `source` fields |

Lines and columns are one-based, and a column counts characters, not bytes. Displayed
relative paths use `/` on every OS.

## Output

`md validate refs` accepts both output families, and its report is the deliverable,
so a formatted output writes it to stdout.

| `--output` | stdout | stderr |
|---|---|---|
| `terminal` (default), `plain`, `markdown`, `markdown-plus`, `html` | the report | only an error that stops the run, such as a file that cannot be read |
| `json`, `json5`, `yaml`, `toml` | one value | nothing |

`terminal` falls back to the `plain` bytes on a pipe. A failed run prints the report
and nothing else: no separate error count follows it.

```json
{
  "valid": false,
  "references_scanned": 1,
  "references_valid": 0,
  "issues": [
    {
      "code": "MissingLocalTarget",
      "message": "Missing local target: ./nope.png",
      "severity": "error",
      "kind": "image",
      "reference": "./nope.png",
      "line": 2,
      "column": 1,
      "section": "child",
      "inserted": false,
      "source": "/work/child.md",
      "included_via": []
    }
  ],
  "warnings": [],
  "outcome": "failure",
  "diagnostics": [
    { "code": "dm.reference.missing_image", "severity": "error", "...": "..." }
  ]
}
```

A success has no `outcome` or `diagnostics`. A failure adds one
[diagnostic](./data-contracts.md#diagnostics) per problem. A file that cannot be read
has no result to report, so it is
`{ "outcome": "failure", "diagnostics": [...], "value": null }`. See
[Data Contracts](./data-contracts.md#other-commands).

## Removed spellings

These are usage errors (exit status 2).

| Removed | Use instead |
|---|---|
| `--format` | `--output` |
