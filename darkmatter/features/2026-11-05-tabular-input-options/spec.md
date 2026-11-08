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
status: finalized-spec
area: darkmatter
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-07
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
    - 2026-11-05-dynamic-schema-types
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
whose path has no extension. Today these require preprocessing the input or
changing its filename; a
headerless file can also be silently misread because its first row is
treated as the header.

## Current behavior

Verified against the code on 2026-11-07:

- **Two arguments only.** The catalog entry is
  `load_data(file: file | null, as: type-definition(container))`
  ([function catalog](../../schemas/partials/functions.yaml)); a third
  argument is a binding error.
- **The extension picks the format.** Darkmatter's
  [format selector](../../lib/src/markdown/compose/expression/data/load.rs) passes the
  local path, or a URL's path with its query and fragment dropped, to
  biscuit-file's [extension detector](../../../biscuit-file/lib/src/detect.rs).
  YAML, JSON, JSONL/NDJSON, CSV, TSV,
  TOML, and JSON5 are read; any other extension, or none, fails with
  `dm.data.unsupported_format`, naming the extension found.
- **The format picks the delimiter.** Darkmatter's
  [data parser](../../lib/src/markdown/compose/expression/data/parse.rs) reads CSV
  with `b','` and TSV with `b'\t'`. biscuit-file's
  [read_delimited](../../../biscuit-file/lib/src/delimited.rs)
  already accepts any ASCII delimiter except `"`, CR, and LF, so the reader
  needs no change.
- **The first row is always a header.** In object mode the header names each
  row's keys and must be non-empty and unique; in list mode (a `tuple([…])[]`
  or `string[][]` contract) the header row is read and then skipped. A
  headerless file therefore loses its first data row in list mode, and in
  object mode the first data row becomes the keys.
- **The memo is keyed by the file alone.** The parsed-source memo
  in Darkmatter's [data runtime](../../lib/src/markdown/compose/expression/data/runtime.rs)
  is keyed by the canonical path or normalized URL, and
  the row mode is applied at bind time per contract.

## Scope and call contract

Support both `load_data(file, as)` and `load_data(file, as, options)`.
`as` remains a quoted SimplifiedSchema container type; options change how
input is interpreted, not the type the call returns.

```md
{{ load_data("rows.csv", "tuple([string, number])[]", {header: false}) }}
{{ load_data("rows.txt", "{ name: string, port: number }[]", {format: "csv", delimiter: ";"}) }}
{{ load_data("https://example.com/export?id=7", "string[]", {format: "json"}) }}
```

The third argument is a closed object: only the following keys are allowed.
Omission, `null`, and `{}` all mean the existing defaults. Optional keys
accept missing or `null` values as absent, following the repository's schema
contract. For example, `{header: null}` uses the default header behavior.

| Key | Bound value | Meaning when absent |
| --- | --- | --- |
| `format` | `yaml` (or `yml`), `json`, `jsonl` (or `ndjson`), `csv`, `tsv`, `toml`, or `json5` | Detect from the extension as today. |
| `delimiter` | For CSV: a string containing exactly one ASCII character except `"`, CR, or LF. For TSV: only `"\t"`. | Comma for CSV, tab for TSV. |
| `header` | Boolean | `true` for CSV and TSV. |

Use the shared function binder and coercion engine for this object, including
normal conversions to its declared field types. A value that cannot bind is
an error; do not introduce a second, stricter native-type rule in the handler.
The delimiter constraint is checked on the bound string. A tab is written as
`"\t"`; empty strings, multiple characters, non-ASCII characters, quote, CR,
and LF are invalid. Format names are case-sensitive. `yml` and `ndjson` are
accepted as aliases, matching the filename extensions the loader already
recognizes, and normalize to `yaml` and `jsonl`: diagnostics and memo keys use
the canonical name, so `{format: "yml"}` and `{format: "yaml"}` share entries.
Do not trim or lowercase option values.

A non-null `format` replaces extension detection completely, including for
a misleading supported extension. CSV with a tab delimiter remains format
`csv` in diagnostics; choosing a delimiter does not rename the format. TSV is
tab-separated by definition: on `tsv`, a `delimiter` other than `"\t"` is
`dm.data.invalid_options` (an explicit `"\t"` is the default spelled out and is
accepted). Data separated by another character is read as `csv` with that
delimiter.
Neither contents nor HTTP `Content-Type` select a format. A non-null
`delimiter` or `header` is invalid for a non-tabular format, even
`header: true`; `{header: null}` is absent and therefore permitted.

**Reader's note:** headerless rows and explicit format selection intentionally
extend the fixed rules of 2026-11-04-loading-data. Strict parsing, cell
conversion, container contracts, and remote access policy retain their
existing meanings. This is not permission to infer a dialect from content.

## Headerless rows and source positions

The element type of `as` continues to select object rows or positional rows,
using the existing rule that all non-null contract arms must agree.
`header: false` requires positional rows, such as `"string[][]"` or
`"tuple([string, number])[]"`. An object-row contract fails with
`dm.data.row_shape_conflict`, suggesting a positional contract or an input
with a header. No `columns` option is introduced.

The existing biscuit-file reader stores the first nonblank record in
`headers` even though it has not interpreted the names. Darkmatter must use
that record followed by `rows` when `header` is false. Keep the parsed table
immutable so another call can still use it with `header: true`. Binding and
diagnostic lookup must share the same logical row view: with no header,
result pointer `/0/1` refers to the second cell of the first input record.
Do not prepend a synthetic header or rewrite the text to achieve this;
that would shift the source positions reported for failures.

| Input with `header: false` | Result |
| --- | --- |
| Empty or blank lines only | `[]`, then judged against `as`. |
| One record | One data row, including empty or repeated cell values. |
| Leading or intervening blank lines | Skipped, with original source positions retained. |
| A later record with a different cell count | `dm.data.malformed`, at that record; the first record determines the width. |
| A quoted delimiter, newline, or doubled quote | The same strict CSV-style parsing as today. |
| Invalid quoting | `dm.data.malformed`; no partial result. |

With `header: true`, the first record is skipped for positional rows and
names the keys for object rows. Header-name validation remains specific to
object rows. No changes to biscuit-file's public reader are required.

For contract failures, retain the existing raw field offsets and convert
them with biscuit-file's source-position helper. Positions are one-based
lines and character columns in BOM-stripped text, including for quoted,
multiline, multibyte, and CRLF input. Missing cells or members retain the
existing nearest-parent location rule. A malformed record need not have a
JSON Pointer. Invalid UTF-8 retains the current original-byte-offset rule.

## Passive checks, execution, and diagnostics

The library owns option validation; composition and DMLS use the same
judgment. The passive check never opens a file, resolves an on-disk type
import, fetches a URL, or evaluates a computed expression.

- Check literal option values even when another argument is computed.
  Reject unknown keys and invalid field values without needing a file.
- A literal format override determines the format even when `file` is
  computed. Otherwise use the existing passive extension check when the
  reference is literal. If the format is not provable, defer format-specific
  checks. In particular, a computed options object might override a literal
  filename's extension; do not reject its contract using that extension alone.
- Check `header: false` against `as` when the selected format is known to be
  tabular and the contract can be classified without I/O. Imported contracts
  that need a file remain provisional until execution.
- Preserve the existing rejection of provably invalid calls in dead branches.
  Literal mistakes are still rejected when `file` is `null`. After normal
  argument binding, a null file returns null before semantic option checks,
  contract imports, resolution, or reading; computed options do not trigger
  those later operations for a null file.

For a non-null file, validate bound option values first, then prepare the
contract, resolve and record the source, choose the effective format, check
option applicability and row shape, enforce remote policy, read and parse,
and bind the result. No parse or fetch occurs after a known option or row
shape error. Resolution failures retain their existing precedence over
format-dependent checks that require the resolved source.

Binding failures keep the shared binder's existing diagnostics. Introduce
`dm.data.invalid_options` for option constraints not expressed by binding
(such as delimiter length and format applicability), with the same code and
message in passive checking and execution. Locate it at the third argument;
include the offending key and explain the accepted values. A headerless
object-row conflict retains `dm.data.row_shape_conflict` at `as`, with a
message explaining the `header: false` conflict. Neither error claims a
position in an unread data file. Malformed input and contract failures retain
`dm.data.malformed` and `dm.data.contract_mismatch`, with the selected format
and existing separate data-file location fields. Register the new error
code wherever the library's data-error catalog is consumed; it is an error,
not a governed warning.

The inferred result type remains the literal `as` type for both arities,
even when options are computed. Preserve null-file typing, callback member
completion, and the existing limits for computed contracts and named types.
Options never require DMLS to execute a load.

## Reuse within a command

**Sequencing (2026-11-08).** 2026-11-05-dynamic-schema-types is implemented
first and changes the same memo and loader: it stores read observations in the
memo entry, checks its slot read gate before the memo lookup, and adds a
memo-generation counter. This feature builds on that code. Observations belong
on the raw-source layer (where bytes are read), and the slot gate stays ahead of
the first memo layer consulted.

A **memo** is the command's in-memory store of earlier reads, parses, and
bound results. Option differences must separate interpretations without
re-reading one source. Use these effective keys rather than serializing the
authored options object:

| Stored result | Key |
| --- | --- |
| Raw source bytes or fetched text | Existing canonical source identity. |
| Parsed source | Source identity, effective format, and effective delimiter (absent for non-tabular formats). |
| Bound value | Parsed-source key, existing prepared-contract key, and effective header setting for tabular formats. |

**Reader's note:** merely adding options to the existing source key would
also repeat local I/O. A separate raw-source entry preserves the prerequisite's
one-read promise while allowing multiple interpretations. Share the retained
bytes/text rather than copying it for each format. Keep format-dependent
UTF-8 errors in the interpretation layer so a cached error names the format
requested by that call.

Omitted defaults and explicit defaults share entries: for a CSV file,
`{}`, `{format: "csv"}`, and `{delimiter: ",", header: true}` reuse the same
parse and bind. Changing only `header` reuses the parse but binds separately.
Changing format or delimiter reuses the raw source but parses separately.
Different contracts reuse the parse and bind separately, as today.

Keep one computation per key for concurrent callers, including failures;
never hold a map lock while reading or parsing. Preserve call-site-specific
projection of cached errors. Existing shell and lifecycle invalidation
clears every memo layer together, with the generation check preventing an
older in-flight read from filling the new maps. It never clears the read set.
This applies in every cache mode. The read set contains source identities,
not one entry per interpretation, and still records resolved sources before
reading or format-dependent failures.

Remote host authorization must happen before returning any cached remote
body or interpretation. Explicit format selection grants no network access,
changes no URL identity, and cannot bypass redirect policy. Preserve the
reference check's remote deferral: lack of remote permission there is not
cached as a failure that would block composition later.

Keep Darkmatter's [DataRuntime::stats()](../../lib/src/markdown/compose/expression/data/runtime.rs)
useful for deterministic verification: `reads`
counts raw source loads, `parses` counts successful parses as today, and
`binds` counts contract binding attempts. Keep `source_hits` as parsed-source
hits and `bind_hits` as bound-result hits, preserving existing two-argument
counts; describe raw-read reuse through `reads`. No timing benchmark or performance
spike is required; work counts answer the reuse question directly.

## Non-goals and resolved design choices

- Other CSV dialect settings (quote character, escape character, comment
  lines, trimming). Quoting stays strict and CSV-style for every delimiter.
- Encodings other than UTF-8, content sniffing, and HTTP `Content-Type`
  inference. An explicit format gives reproducible behavior for both local
  files and URLs without changing transport metadata contracts.
- A `columns` list for headerless object rows. Positional contracts already
  solve headerless input; adding names and validating their interaction with
  headers would be a separate feature. Rejecting object rows avoids silently
  inventing keys.
- Changes to `bf`, which still does not convert CSV, TSV, or JSONL.
- New CLI flags, file-size limits, persistent cache formats, expression
  grammar, or new dependencies.

## Documentation and affected surfaces

Implementation updates the library's function catalog, shared passive checks,
result typing, loader, memo, and tabular diagnostic lookup. DMLS completion
and hover describe the third argument, its allowed keys and values, and its
defaults; nested option completion must use the declared object shape rather
than a separate editor-only list.

Update the generated `load_data` function page and its catalog examples,
[Loading Data](../../docs/topics/loading-data.md), the relevant memo section
of [Caching](../../docs/topics/caching.md), and
[CLI data contracts](../../docs/cli/data-contracts.md). Include headerless,
custom-delimiter, and extensionless examples, the new error code, and the
read/parse distinction. Update the Darkmatter skill's signature and memo
rules alongside implementation. The README remains an overview; no new
reference material belongs there. Documentation describes current behavior
when implementation lands and never links back to this spec.

## Open questions

None remain for this scope. Author decisions on 2026-11-08: implement after
2026-11-05-dynamic-schema-types; accept `yml` and `ndjson` as format aliases;
restrict TSV's delimiter to tab. The decisions above keep explicit format
selection and positional headerless rows; named columns and response-header
inference are excluded, rather than left as implementation choices.

## Acceptance criteria and verification

Use bounded L1 fixtures and the normal command path. These requirements add
coverage to the existing suite rather than replacing it.

- A two-record CSV with no header, read using
  `load_data("rows.csv", "tuple([string, number])[]", {header: false})`,
  returns both rows. Cover TSV and a one-record file too.
- A headerless object-row contract fails with
  `dm.data.row_shape_conflict`, passively when the relevant values are
  literal and at execution for computed values or on-disk contract imports.
- `load_data("rows.txt", "{ a: string }[]", {format: "csv", delimiter: ";"})`
  reads semicolon-separated input. Also cover overriding a supported but
  incorrect extension, choosing TSV's tab default, and using a pipe delimiter.
  `{format: "yml"}` and `{format: "ndjson"}` behave as `yaml` and `jsonl` and
  share memo entries with them. `{format: "tsv", delimiter: ","}` fails with
  `dm.data.invalid_options` before reading; `{format: "tsv", delimiter: "\t"}`
  is accepted.
- Invalid option objects cover unknown keys, unbindable field types, invalid
  format names, and empty, multi-character, non-ASCII, quote, CR, and LF
  delimiters. Non-null `header` and `delimiter` on JSON fail before reading.
  Assert the shared diagnostic and third-argument location in library and
  DMLS tests; literal rejection performs no I/O.
- Computed options are validated at execution. A computed override does not
  cause a false passive rejection based on a literal file's extension.
  Null options and null fields follow the defaults above; valid null-file
  calls return null with no reads or contract imports.
- Empty files, blank lines, ragged rows, strict quoting, and empty cell text
  follow the headerless table above. A contract failure in the first
  headerless row reports pointer `/0/1` and that field's raw position. Include
  a later row and a quoted multiline field, with multibyte and CRLF content,
  so diagnostic lookup cannot silently remain shifted by one row.
- Two different successful delimiter interpretations of one source produce
  `reads = 1` and `parses = 2`. Identical calls and equivalent default spellings
  parse once; changing only `header` parses once and binds twice. Distinct
  explicit formats do not share parsed results or format-specific failures.
  Verify call ordering cannot change the result.
- A header-name failure in object mode does not poison positional mode over
  the same parsed source. A failure reused at another call names that call's
  authored position. Existing invalidation and concurrent-reuse tests cover
  all memo layers; no elapsed-time threshold is needed.
- An extensionless URL can load JSON using `{format: "json"}`. Use a local
  HTTP mock for allowed and denied hosts, including a warmed memo, and prove
  the reference check can defer the read for composition. No live endpoint
  or window focus is needed.
- Both call arities retain inferred callback types and nullability in DMLS.
  All existing two-argument results, diagnostic fields, read sets, and work
  counts remain unchanged.
- Catalog examples pass the passive shipped-artifact corpus check. Through
  the normal `md compose` invocation, test one headerless success and one
  option failure with JSON output: exit 1 for failure, empty stderr, a
  located diagnostic, and output conforming to the existing envelope schema.
- Run `just test` and `just lint` in `darkmatter/` for implementation. Use
  the repository test toolkit for portable fixtures and headless editor
  tests. No real-terminal or browser test is required for these semantics.

The spec review itself changes only this document; these checks are the
implementation's completion requirements.
