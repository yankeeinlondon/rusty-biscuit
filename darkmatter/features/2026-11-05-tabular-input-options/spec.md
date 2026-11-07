---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
kind: feature
created: 2026-11-07
status: draft-spec
area: darkmatter
reviewed: false
clarified: false
implemented: false
review_iterations: 0
packages:
    - darkmatter
    - darkmatter-lsp
related:
    - 2026-11-04-loading-data
depends-on:
    - 2026-11-04-loading-data
---

# Tabular Input Options

Give `load_data` an optional third argument, an options object, so a caller
can read a CSV or TSV file that has no header row, read delimited text with a
delimiter other than a comma or tab, and name a file's format when its
extension is missing or wrong.

## Motivation

2026-11-04-loading-data fixed every reading choice to the file's extension
and to a header row, which keeps the two-argument call simple. Real inputs
break those assumptions: exported tables without a header, semicolon- or
pipe-separated files, and URLs such as `https://example.com/export?id=7`
whose path has no extension. Today none of these can be read at all.

## Current behavior

Verified against the code on 2026-11-07:

- **Two arguments only.** The catalog entry is
  `load_data(file: file | null, as: type-definition(container))`
  (`schemas/partials/functions.yaml`); a third argument is a binding error.
- **The extension picks the format.** `data/load.rs::format_of` passes the
  local path, or a URL's path with its query and fragment dropped, to
  `biscuit_file::detect_from_extension`. YAML, JSON, JSONL/NDJSON, CSV, TSV,
  TOML, and JSON5 are read; any other extension, or none, fails with
  `dm.data.unsupported_format`, naming the extension found.
- **The format picks the delimiter.** `data/parse.rs` reads CSV with `b','`
  and TSV with `b'\t'`. biscuit-file's `read_delimited(text, delimiter)`
  already accepts any ASCII delimiter except `"`, CR, and LF, so the reader
  needs no change.
- **The first row is always a header.** In object mode the header names each
  row's keys and must be non-empty and unique; in list mode (a `tuple([…])[]`
  or `string[][]` contract) the header row is read and then skipped. A
  headerless file therefore loses its first data row in list mode, and in
  object mode the first data row becomes the keys.
- **The memo is keyed by the file alone.** The parsed-source memo
  (`data/runtime.rs`) is keyed by the canonical path or normalized URL, and
  the row mode is applied at bind time per contract.

## Scope

1. An optional third argument `options`, an object with three optional keys:
   - `format`: one of `yaml`, `json`, `jsonl`, `csv`, `tsv`, `toml`, `json5`.
     It replaces the extension's choice; the extension is not consulted.
   - `delimiter`: one ASCII character other than `"`, CR, or LF. Allowed only
     when the format (chosen either way) is CSV or TSV.
   - `header`: a boolean, default `true`. With `false`, every row is data,
     and the contract must read rows by position (list mode); an object-row
     contract fails with `dm.data.row_shape_conflict`, because there are no
     names to key by.
2. Unknown keys, wrong value types, a delimiter on a non-tabular format, and
   `header` on a non-tabular format are errors. A literal options object is
   checked passively, like a literal `as`; a computed one is checked at
   execution.
3. The memo stays correct: the parsed-source key includes the chosen format
   and delimiter, and the bound-result key includes `header`, so two calls
   that read one file differently never share a parse they should not.
4. The two-argument call is unchanged, byte for byte, in its results,
   diagnostics, and read set.
5. Passive typing, DMLS completion and hover for the option keys, the
   catalog examples, and the `darkmatter/docs/` loading-data page cover the
   new argument.

## Non-goals

- Other CSV dialect settings (quote character, escape character, comment
  lines, trimming). Quoting stays strict and CSV-style for every delimiter.
- Encodings other than UTF-8.
- Changing `bf`, which still does not convert CSV, TSV, or JSONL.

## Open questions

- Whether `header: false` with an object-row contract should instead accept
  a `columns: [...]` list that names the keys.
- Whether a `format` override on a URL should also be inferable from the
  response's `Content-Type`. The current remote path returns text only, so
  this spec assumes not.

## Acceptance criteria

- `load_data("rows.csv", "tuple([string, number])[]", {header: false})`
  returns every row, the first included.
- `load_data("rows.csv", "{ name: string }[]", {header: false})` fails with
  `dm.data.row_shape_conflict`, passively when both arguments are literal.
- `load_data("rows.txt", "{ a: string }[]", {format: "csv", delimiter: ";"})`
  reads a semicolon-separated file, and a contract violation reports the raw
  cell's line and column.
- `load_data("https://example.com/export?id=7", "string[]", {format: "json"})`
  reads a JSON body served from an extensionless URL.
- `{delimiter: ";"}` with a `.json` file, an unknown key, and
  `{delimiter: "\""}` each fail with a located diagnostic at the options
  argument.
- Two calls over one file, one with `{delimiter: ";"}` and one without,
  parse it twice; two identical calls parse it once
  (`DataRuntime::stats()`).
- Every existing two-argument `load_data` test passes unchanged.
