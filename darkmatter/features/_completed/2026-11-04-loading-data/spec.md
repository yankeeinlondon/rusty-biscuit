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
created: 2026-11-04
status: finalized-spec
area: darkmatter
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-06
clarified: true
clarified_by: claude/opus
needs_rulings: false
implemented: true
review_iterations: 2
completed: true
related:
    - 2026-10-28-extract-simplified-schema
    - 2026-11-04-composition-errors
depends-on:
    - 2026-11-04-composition-errors
    - 2026-11-06-dotted-paths
next_spec: 2026-11-05-render-surface
references:
    spikes/sp1-scanner-corpus-diff.md: Compares today's double-brace expression scanner with several quote-aware variants over 6,909 repository files and recommends the balanced, parse-gated rule adopted in R11. Also lists every other double-brace detector and whether it must move to the shared scanner.
    spikes/sp2-line-column-per-format.md: Tests a JSON Pointer to line/column locator for each supported format and fixes the per-format locators, their owning crates, the line-break rule, and the one pointer-only case (YAML with aliases or multi-line flow collections) used in R3 and R5.
    spikes/sp3-union-failure-flattening.md: Shows that contract-mismatch records must come from the coercion engine's `ConversionError` rather than `validate_soft`, and defines the union-flattening rule, leaf vocabulary, and ordering used in R5.
implemented_by: "claude/opus"
---

# Loading Data

Add a `load_data(file, as)` expression function that reads a structured data
file (YAML, JSON, JSONL, CSV, TSV, TOML, or JSON5) and returns its contents
as a typed container value. The caller states the expected shape with `as`, a
quoted type definition that must denote a **container** type. Data that does
not satisfy that contract is an error, never a partially typed value.

This spec is **finalized**. The decisions taken in the 2026-11-05 clarification
sessions, and those that follow from the three spikes, are recorded under
[Resolved decisions](#resolved-decisions). The spikes are complete; see
[Spikes and risks](#spikes-and-risks). Work that implementation may do
without pausing for approval is listed under
[Pre-authorizations](#pre-authorizations).

The inline review on 2026-11-06 preserves the author's settled choices,
including unlimited input size.
It makes failure locations, memo lifetime, null typing, and dependency
ordering explicit. Review notes below explain corrections to conflicting
requirements; they do not authorize unrelated implementation work.

This spec builds on 2026-11-04-composition-errors, which is complete. That fix
removed `md compose --output report-json` in favor of `md compose --output json`
(the **compose data envelope**), added optional fields to the diagnostic
record, and documents the data outputs on `docs/cli/data-contracts.md` (the
**data-contract page**). This spec uses those names throughout.

## Motivation

The expression language has no way to read a data file today. The function
catalog (`schemas/partials/functions.yaml`) reads paths, globs, and Markdown,
but nothing parses YAML or JSON into a list or dictionary:

- `as_json`, `as_csv`, and siblings go the other way (list → text).
- `try_frontmatter` is the closest neighbor, but it only reads a Markdown
  file's `---` block. On a plain YAML file it returns `{ok: true, value: {}}`
  — a silent empty mapping — and it can never return a top-level sequence.

So a single source of truth such as `claudine/docs/os.yaml` (a list of
supported operating systems) cannot feed a document, a `::map` region, or —
once 2026-11-05-dynamic-schema-types lands — a schema's `enum(...)`.

## Example

```yaml
# claudine/docs/os.yaml
- macOS
- Linux
- Windows
```

```markdown
Supported: {{ as_csv(load_data("claudine/docs/os.yaml", "string[]")) }}
```

renders `Supported: macOS, Linux, Windows`. With a mismatching contract:

```markdown
{{ load_data("claudine/docs/os.yaml", "number[]") }}
```

composition fails. The diagnostic points at the `load_data` call in the
Markdown document and names the data file, the location inside the data
(`/0`, line 1, column 3), the expected type (`number`), and the value found
(`"macOS"`). Line and column are 1-based and the column counts characters,
not bytes: `macOS` starts at the third character of `- macOS`.

## Dependencies

- **2026-11-04-composition-errors (met 2026-11-06).** This spec adds fields to
  the diagnostic record and the compose data envelope that fix defines, and
  documents its codes on the data-contract page. Requirements below were
  revised on 2026-11-06 against what that fix actually built; see
  [Resolved decisions](#resolved-decisions).
- **2026-11-06-dotted-paths (prerequisite).** This is the other prerequisite
  declared in frontmatter. It establishes nested frontmatter access and
  compose setters before this feature's CLI integration. Do not claim all
  prerequisites are complete merely because composition errors is complete.
  Expression member access and JSON Pointers retain their own existing
  grammars; this feature does not reinterpret them as CLI dotted paths.

## Terms

- **Type definition** — a SimplifiedSchema type written as text, such as
  `string[]` or `{ name: string }`. The catalog's `type-definition`
  parameter type accepts such text and checks that it parses.
- **Denoted type** — the type that the text describes (for `"string[]"`, an
  array of strings), as opposed to the carrier, which is always a string.
- **Container** — a denoted type that holds other values: an array, a tuple,
  an object, or a pattern dictionary (`{ <string>: T }`).
- **Passive call check** — the first composition stage. It inspects every
  function call without running anything (no file I/O, no network) and
  rejects a call that is provably wrong, with the span of the bad argument.
- **Coercion engine** — `simplified_schema::coercion`, the one place that
  converts a `serde_json::Value` toward a declared type (for example the text
  `"8080"` to the number `8080`) and validates the result. Its entry point in
  the simplified-schema library is
  [coercion::bind](../../../simplified-schema/lib/src/coercion/mod.rs);
  on failure it returns a `ConversionError`, a tree of
  every failing location.
- **JSON Pointer** — a path into a data value, such as `/3/port` (fourth
  element, its `port` field). The empty pointer `""` means the whole value.
- **Locator** — code that maps a JSON Pointer to a byte offset in the
  source text of one data format, so a diagnostic can report a line and
  column.
- **Compose data envelope** — the one object `md compose --output json`
  prints: `{file, outcome, document, diagnostics, summary, perf,
  lifecycle_output}`. Its shape is defined in
  `darkmatter/cli/schemas/output/compose.yaml` and enforced by a conformance
  test (both introduced by 2026-11-04-composition-errors).
- **Diagnostic record** — one entry of `diagnostics` (the darkmatter library's
  [Diagnostic](../../lib/src/markdown/compose/expression/diagnostics.rs)). Its
  `code` is a stable `dm.<family>.<kind>` string shared with DMLS; its
  `source` is the position in the Markdown document.
- **Read set** — the list of files and URLs that expressions read during one
  command (R8).
- **Reference check** — the pass `md compose` runs before composition (its
  `validate references` stage) to find broken links, images, and `::file`
  targets. A link target can be computed, so the pass evaluates expressions on
  a throwaway copy of the document. It runs no shell command and has no
  permission to read a remote URL. `md graph` and `md validate refs` prepare
  documents the same way.
- **Options-level shared slot** — a value held behind an `Arc` on
  `ComposeOptions`, so every part of one command (the reference check, the
  pre-flight walk, composition, both expression-context builders, every
  document, every nested compose) sees the same instance. The existing
  `RemoteFetchRuntime` works this way: a caller supplies one to share across
  passes, and a run with none builds its own. R7 and R8 use the pattern.

## Requirements

### R1. Signature and catalog entry

**Requirement**

```text
load_data(file: file | null, as: type-definition(container)) -> any | null   (fallible)
```

- `file` is a `FileReference` resolved in the calling document's context,
  exactly like every other file-taking function. Null passes through to null,
  matching the catalog's existing null convention.
  Use the context derived from the darkmatter library's existing
  [ComposeRequest](../../lib/src/markdown/compose/context/request.rs);
  never recapture the process directory, environment, home, or repository.
  Null still undergoes ordinary argument binding and the passive call check,
  but execution returns before resolving imports or touching a data file.
- `as` is a **quoted string** containing a type definition:
  `load_data("claudine/docs/os.yaml", "string[]")`. Named type imports are
  allowed: `"os-list@./schemas/os.yaml"` and `"Name@darkmatter:<stem>"`.
  `Name@this` is **not** allowed (see below).
- The return value is the parsed data, coerced and validated against `as`.
  The catalog signature says `any`; the static type the checker actually
  uses is narrower when `as` is a literal (R10).
- The function is listed in `schemas/partials/functions.yaml` (category
  `Filesystem`), so DMLS hover, completion, and the generated function docs
  come from the catalog; nothing is hand-maintained in a second place.
- DMLS hover and completion for type definitions work inside the `as` string
  of a `load_data` call, as they already do for a frontmatter value whose
  schema type is `type-definition`.

**Design notes**

- Why quoted: unquoted type syntax does not survive the expression parser
  today. `string[]` parses as the variable `string` followed by an invalid
  index (`simplified-schema/lib/src/expression/parser.rs`), `{ … }` parses
  as an object literal, and `number(…)` calls the existing `number()`
  function. Backticks and a bare type syntax were considered and rejected for
  now.
- The function-signature translator already validates a `type-definition`
  parameter through simplified-schema's `judge_type_definition`
  (`darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs`),
  and the passive call check already judges literal arguments with their
  span. The new work is the `container` option (R2).
- DMLS today offers type-definition hover/completion only from frontmatter
  (`dmls/src/providers/frontmatter.rs`); extend it to any string argument
  whose catalog parameter is a `type-definition`.
- A named import is accepted **provisionally** by the passive check, without
  reading the schema file — the same precedent as the parse-only `forms(…)`
  check, which lets named references pass. `load_data` resolves the import
  when it executes and then applies the container check (R2). A
  `Name@darkmatter:<stem>` import needs no I/O and can be judged passively.
- `Name@this` means "a type declared in this schema document". An
  expression argument has no schema document of its own, so the reference has
  no clear target. It is rejected with `dm.data.this_import_rejected`
  (R5): passively when `as` is a literal, at execution otherwise.
  This restriction applies anywhere in the argument's own type expression.
  It does not prohibit `Name@this` inside an imported schema: there, `this`
  names that schema under the existing import resolver's rules.
- Resolve file imports through the existing schema selector and the calling
  document's schema roots. Imports inside that schema retain the schema's
  own origin. Resolve the complete definition graph, including recursive
  definitions, before checking containers or choosing a tabular row shape.
  A broken or missing import is an expression evaluation failure with the
  existing typed schema cause; it is not malformed data or a mismatch with
  data that has not yet been read.

**Acceptance criteria**

- `load_data("os.yaml", "string[]")` returns `["macOS", "Linux", "Windows"]`
  for the example file.
- `load_data(null, "string[]")` returns null without reading anything.
- `load_data("os.yaml", string[])` (unquoted) is a parse error.
- The generated function docs and DMLS hover for `load_data` come from the
  catalog entry.
- DMLS hover over `string` inside `"string[]"` in a `load_data` call shows
  the type-definition hover; completion inside the string offers type names.
- `load_data("os.yaml", "os-list@./schemas/os.yaml")` passes the passive check
  without the schema file being opened, and executes correctly when the
  named type is a container.
- `load_data("os.yaml", "Os@this")` is rejected by the passive check with
  `dm.data.this_import_rejected` and the `as` span; no file is read.

### R2. `as` must denote a container

**Requirement**

The denoted type of `as` must be a container:

| Denoted type | Example `as` | Accepted |
|---|---|---|
| array | `"string[]"`, `"{ name: string, port: number }[]"` | yes |
| tuple | `"tuple([string, number])"` | yes |
| object | `"{ name: string, tags: string[] }"` | yes |
| pattern dictionary | `"{ <string>: number }"` | yes |
| scalar | `"string"`, `"number"`, `"enum(a, b)"` | **no** |
| union | every non-null arm a container | yes (see below) |

Note that `[string, number]` written as a type definition is a **union**
(`string | number`), not a tuple; the tuple spelling is
`tuple([string, number])`.

A union is accepted when every non-null arm denotes a container. `X | null`
is allowed because an empty YAML file parses to null. `unknown`/`any` alone
is not a container. At least one non-null container arm is required:
`"null"` alone is rejected. An unresolved named reference is provisional,
not proof that its eventual definition is a container.

| `as` | Container? | Notes |
|---|---|---|
| `string[] \| null` | yes | empty YAML file → null |
| `unknown[]` | yes | rejected over CSV/TSV (R4) |
| `unknown` | no | |
| `object` | yes | rejected over CSV/TSV (R4) |
| `{a: string}[] \| {b: number}[]` | yes | CSV/TSV: object rows |
| `string[] \| {a: string}` | yes | rejected over CSV/TSV (object arm) |
| `literal([…])` | — | not expressible: `literal` takes a scalar |

A non-container `as` is a static error, `dm.data.not_a_container` (R5). When
`as` is a literal, the passive call check rejects it **before anything
runs**, with the span of the `as` argument. A computed `as` is judged when
the call executes.

**Design notes**

- Add a `container` option to the type-definition format check in
  simplified-schema (`simplified-schema/lib/src/format.rs`, beside the
  existing `forms(…)` and `declared-required` options) so the catalog can
  declare `as: type-definition(container)`. It is judged on the literal text
  only; no I/O.
- The predicate (for example `denotes_container`) lives in simplified-schema
  next to `narrowing::families`
  (`simplified-schema/lib/src/simplified/narrowing.rs`) and walks union arms
  with `query::flattened_alternatives`.
- The predicate and R5's failure-flattening rule share the notion "drop null
  arms"; cover that shared rule with nullable-container tests rather than
  reciprocal per-function documentation links.

**Acceptance criteria**

- Each "yes" row in both tables passes the passive check (ignoring the
  CSV/TSV notes, which R4 covers); each "no" row is rejected by it with
  `dm.data.not_a_container` and the `as` argument's span, and no file is
  read.
- `"[string, number]"` is rejected (a union of scalars).
- A computed scalar `as` (for example `load_data(f, t)` with `t = "string"`)
  fails at execution with the same diagnostic code.

### R3. Formats, ownership, and malformed input

**Requirement**

The format is chosen from the extension of the file path, or of a URL's
**path** (any `?query` or `#fragment` is ignored). Matching is
case-insensitive, so `DATA.JSON` is JSON. Neither the HTTP `Content-Type`
header nor the file's content is consulted, and there is no way to override
the format in this spec (see [Follow-ups](#follow-ups)).

| Extension | Parsed as |
|---|---|
| `.yaml`, `.yml` | one YAML document, any top-level shape |
| `.json` | one JSON value, any top-level shape |
| `.jsonl`, `.ndjson` | one JSON value per line → array |
| `.csv` | RFC 4180 rows → array (see R4) |
| `.tsv` | tab-separated rows, CSV-style quoting → array (see R4) |
| `.toml` | one TOML document (always a table) |
| `.json5` | one JSON5 value, any top-level shape |

An unknown extension is `dm.data.unsupported_format`. Input a format cannot
parse is `dm.data.malformed`; the rules below say exactly what counts as
malformed. Every such error names the file and the position: a line and
character column, computed the same way as in R5 (or a byte offset where a
line is meaningless, such as invalid UTF-8).

| Input | Result |
|---|---|
| a byte-order mark at the start | stripped, for every format |
| CRLF line endings | accepted, for every format |
| local text that is not valid UTF-8 | error naming the file and the byte offset (a URL's is a transport failure, R5) |
| a zero-byte file | YAML → `null`; JSON and JSON5 → error; JSONL → `[]`; CSV and TSV → `[]`; TOML → `{}` |
| YAML stream with more than one document | error |
| YAML anchors and aliases | aliases are expanded |
| YAML custom tags (`!foo`) | error |
| YAML duplicate keys or non-string mapping keys | error, matching the shared whole-document YAML loader |
| YAML merge keys (`<<`) | unchanged shared-loader behavior; no new merge-key expansion |
| JSON object with a duplicate key | the last value wins (documented) |
| JSON5, TOML: NaN or infinity | error |
| JSONL: a whitespace-only line | skipped |
| JSONL: a line that is not valid JSON | error with that line's number |

CSV- and TSV-specific rules are in R4.

**Design notes — who owns what**

`DataFormat` in `biscuit-file/lib/src/format.rs` today only names formats; it
parses nothing. TOML and JSON5 parsing exist as separate wrapper types
(`Toml`, `Json5`). Extension detection already exists as
`detect_from_extension` in `biscuit-file/lib/src/detect.rs`. The work divides
as follows:

- **biscuit-file**
  - Make `detect_from_extension` public and extend it to recognize JSONL,
    CSV, and TSV, rather than adding a second extension table such as a
    `DataFormat::from_extension`. Add `DataFormat` variants for JSONL, CSV,
    and TSV.
  - Add a reader for JSON lines and a reader for delimited text (CSV/TSV,
    the tab delimiter for TSV). The delimited reader sits behind a new
    **opt-in** Cargo feature (not in `default`), so biscuit-file's other
    consumers do not pay for it. Each record carries its start line **and
    each field's raw byte offset** in the source text.
  - Why raw offsets need `csv-core`: the `csv` crate's field ranges index the
    record's *unescaped* buffer, so after a doubled quote they no longer
    match the source (for `a,"x""y,z",q` field 2 reports `6..7`, but it
    starts at byte 11). `csv_core::Reader::read_field` reports how many input
    bytes it consumed, and a running total gives each field's true start.
  - New dependencies: `csv` (`csv = "1.3"`, already in the workspace lock
    through biscuit-tui's CLI) and `csv-core` (0.1.13, already in the lock as
    a dependency of `csv`). Record both in `docs/dependencies.md`.
  - The per-format locators and the shared position helper of R5.
- **darkmatter** — strips the byte-order mark, checks UTF-8, and routes by
  format:
  - YAML through the existing whole-YAML-file loader `load_yaml_document`
    (`darkmatter/lib/src/markdown/schemas/yaml_documents/load.rs`);
  - JSON through `serde_json` plus simplified-schema's
    `admit_json_text_numbers`;
  - TOML through biscuit-file's `Toml`, converted to a JSON value;
  - JSON5 through biscuit-file's `Json5` plus `reject_non_finite_json5`;
  - JSONL, CSV, and TSV through the new biscuit-file readers.
- **Numbers** enter only through `simplified_schema::numeric`, as everywhere
  else. biscuit-file sits below simplified-schema in the dependency order and
  cannot call it, so number admission happens in darkmatter after the
  biscuit-file reader returns.
  Admit JSONL numbers from each original record's text before numeric spelling
  is lost; YAML uses the admitted YAML loader. JSON5 and TOML likewise must
  reject inadmissible numeric tokens before a lossy conversion rounds them.
  Rejecting non-finite numbers alone is insufficient. Test the same
  non-exact integer in every numeric format, including JSONL and JSON5.
- **TOML numbers need new work.** biscuit-file's `toml_to_json`
  (`biscuit-file/lib/src/toml_impl/types.rs`) silently turns NaN and
  infinity into `null` and passes integers through without admission. The
  TOML path must reject non-finite floats (as JSON5 does) and admit every
  number through `simplified_schema::numeric`, so an integer outside the
  exact-`f64` range is rejected as it is for JSON. The rejection may live in
  biscuit-file or in a darkmatter-side conversion, but other `toml_to_json`
  callers must not silently change behavior: either add a separate rejecting
  conversion, or check every existing caller and record the change in the
  implementation log.
- **Parser-error positions.** Parsers disagree on what a column is
  (`serde_json` reports a byte column, `json-five` a character column,
  `toml` a byte range, `serde_yaml_ng` and `csv` a byte index). Every
  parser error is first turned into a byte offset in the source text (for
  `serde_json`, the line's start plus `column - 1` bytes) and then into a
  line and character column by the shared helper (R5). A parser's own column
  is never passed through.
- Loaded values remain data. Strings containing `{{ ... }}`, `$(...)`,
  directives, `kind`, or `$schema` are not composed, executed, or treated
  as schema-selection instructions. YAML uses the parsing loader, not the
  whole-document kind-validation entry point. Only `as` supplies the contract.
  Reject custom YAML tags on this data-loading path without changing the
  shared loader's behavior for its other callers.

**Acceptance criteria**

- Each extension in the table loads a fixture into the expected value, and
  an upper-case extension loads the same way.
- A URL whose path ends in `.json` and carries `?v=2#top` loads as JSON.
- `.ndjson` and `.jsonl` behave identically.
- Each row of the malformed-input table has a fixture with the stated
  result; every error names the file and its line and character column, or
  its byte offset for invalid UTF-8.
- A malformed JSON fixture with a multibyte character before the bad token
  on the same line reports the character column, not `serde_json`'s byte
  column.
- An unknown extension fails with `dm.data.unsupported_format`.
- A JSON number, or a TOML integer, outside the exact-`f64` range is rejected
  the same way it is for frontmatter; a TOML `nan` or `inf` is rejected.
- A loaded string containing an expression or shell syntax stays unchanged,
  and a YAML mapping containing `kind` or `$schema` causes no schema discovery
  or extra read. Numeric admission also covers JSONL and JSON5 tokens.
- biscuit-file built without the new feature does not depend on `csv` or
  `csv-core`.
- `docs/dependencies.md` lists `csv` and `csv-core` for biscuit-file.

### R4. Tabular shape follows `as`

**Requirement**

CSV and TSV carry no types. The first row is a header. The row shape is
chosen from the element type of `as`. Every non-null arm of `as` must be an
array or tuple, and all their element types must agree:

- every element is an **object** (object-row mode) → each row becomes
  `{header: cell, …}`;
- every element is a **tuple or array** (list-row mode) → each row becomes a
  list of cells and the header row is skipped.

Cells are text. Turning `"8080"` into the number `8080` is the normal job of
the coercion engine when it applies `as`; no CSV-specific number guessing is
added.

The rules are strict:

| Case | Result |
|---|---|
| arms disagree (one object element, one list element) | `dm.data.row_shape_conflict`, naming the conflicting arms |
| a scalar or `unknown` element (`"string[]"`, `"unknown[]"`) | `dm.data.row_shape_conflict`, suggesting `"{ <string>: string }[]"` or `"tuple([…])[]"` |
| a top-level object or dictionary `as` (`"object"`, `"{ <string>: string }"`) | the same diagnostic and suggestion |
| a file with no header row | not supported; the first row is always a header (see [Follow-ups](#follow-ups)) |
| a duplicate or empty header, object-row mode | `dm.data.malformed`, naming the columns, on the header's source line |
| a duplicate or empty header, list-row mode | allowed (the header is skipped) |
| a row with more or fewer cells than the header (ragged) | `dm.data.malformed`, with that row's line |
| an empty cell | the empty string `""`; the coercion engine decides whether that satisfies `as` |
| a quoted cell containing a newline | allowed; record errors use the record start, cell errors use that field's raw start |
| a quoted cell containing the delimiter | allowed |
| a header-only file | `[]` |

LF and CRLF terminate records; CSV-style quotes may enclose either delimiter
or a line break, and doubled quotes escape a quote. Reject an unterminated
quoted field, a quote inside an unquoted field, or text between a closing
quote and the next delimiter or record end as `dm.data.malformed`. The reader
must enforce these rules even if its underlying CSV parser accepts them.
Do not trim cells or headers. A blank physical line outside a quoted field
is skipped, matching the CSV reader; it does not become an empty header or
row. A delimiter-only line is a record with empty cells. Header diagnostics
use the header's actual source line, which need not be line 1.

**Cell conversion.** The coercion engine converts cell text with the same
rules it uses for frontmatter. This spec pins that behavior with a fixture;
it does not change it:

| Cell | as `number` | as `boolean` | as `string` |
|---|---|---|---|
| `" 42 "` | `42` (surrounding whitespace trimmed) | — | kept |
| `"1_000"` | `1000` | — | kept |
| `"yes"`, `"on"`, `"1"` | — | `true` | kept |
| `""` | fails | fails | `""` |
| `"1,000"`, `"NaN"`, `"0x10"` | fails | — | kept |
| `"9007199254740993"` | fails (not exactly representable) | — | kept |

The boolean words follow a YAML-1.1-like vocabulary, which a CSV author may
not expect (`"1"` reads as `true`), so the docs state it (R12).

**Design notes**

- The row-shape rule is CSV/TSV-specific and lives in darkmatter, not in the
  simplified-schema container predicate.
- When both `file` and `as` are literals, the passive check knows the
  extension and applies the row-shape rule before anything runs; otherwise
  it is applied at execution.
  An unresolved file import defers this check too: knowing the extension
  does not supply the imported element type. Walk element unions and all
  fixed tuple slots; they must select the same row mode. An empty top-level
  tuple selects no mode and is rejected. Null top-level arms do not select
  a mode; nullable or unknown row elements do not describe a row and are
  rejected.
- TSV uses the same quoting as CSV (a tab delimiter), not the "no quoting,
  escape sequences" TSV variant.
- The cell-conversion fixture exists so that a later change to the shared
  engine shows up as a visible behavior change for CSV, not a silent one.

**Acceptance criteria**

- `"{ name: string, port: number }[]"` over a CSV with header `name,port`
  yields objects with numeric `port`.
- `"tuple([string, number])[]"` over the same file yields lists and skips the
  header.
- `"{a: string}[] | tuple([string])[]"` over a CSV is
  `dm.data.row_shape_conflict` naming both arms, from the passive check when
  both arguments are literals.
- `"string[]"`, `"unknown[]"`, and `"object"` over a CSV are each
  `dm.data.row_shape_conflict` with the suggestion.
- Each other row of the strict table has a CSV fixture (and a TSV fixture for
  quoting) with the stated result and line.
- One fixture pins every row of the cell-conversion table: an empty cell
  against `number` or `boolean` is a contract mismatch at that cell and is
  accepted as `""` against `string`; `" 42 "` → `42`; `"1_000"` → `1000`;
  `"yes"`, `"on"`, `"1"` → `true`; `"1,000"`, `"NaN"`, `"0x10"`, and
  `"9007199254740993"` fail against `number`.

### R5. Contract enforcement and diagnostics

**Requirement**

After parsing, the value is converted by the coercion engine
(`coercion::bind` over the bare shape of `as`), the same engine frontmatter
uses. Any failure is an error (never a warning, never a partial value). The
failure records come from the engine's `ConversionError` through a
simplified-schema flattening helper, **never** from `validate_soft`'s JSON
Schema problems (see the design notes for why).
This is bare-value binding, with no frontmatter pending-value deferral.
Preserve the existing function binder's passive refinement semantics: for
example, a `file(eager)` leaf is checked as file-reference syntax, without
probing or fetching the leaf. Reading the data source does not authorize
effects from values inside it. Do not claim these leaves establish file
existence; a future context-aware data-validation API would be separate work.

**Codes.** Each static or data error has its own diagnostic code, all with
severity `error`:

| Code | When | Raised by |
|---|---|---|
| `dm.data.not_a_container` | `as` is not a container | R2 |
| `dm.data.row_shape_conflict` | `as` cannot pick a CSV/TSV row shape | R4 |
| `dm.data.this_import_rejected` | `as` uses `Name@this` | R1 |
| `dm.data.unsupported_format` | the extension names no supported format | R3 |
| `dm.data.malformed` | the content cannot be parsed under its format's rules | R3, R4 |
| `dm.data.contract_mismatch` | the parsed data does not satisfy `as` | this section |

A failure to resolve or read the file — a missing file, a denied remote host,
a failed fetch — is **not** a new code. It stays
`dm.expression.evaluation_failure`, and gains the fields a reference record
already has, so a script can tell what went wrong without parsing the message.
The rule covers **every read-side expression function**, not only `load_data`.
Before this spec only a `MarkdownError::Reference` filled these fields, so
`::file` had them and `frontmatter()` did not.

```json
{
  "code": "dm.expression.evaluation_failure",
  "reason": "file not found",
  "construct": "frontmatter()",
  "reference": "missing.md",
  "failure": "no-match"
}
```

- `construct` is the function's name followed by `()`.
- `reference` is the argument as written.
- `failure` is the `FileReferenceDiagnostic`'s resolution-failure class.
- `included_via` follows its existing rule: omitted when no one reference or
  trail is known.

The code is deliberately not a `dm.reference.*` one: those codes are the
missing-reference diagnostics that `--allow` and `--deny` govern. Human stderr
and each `message` are unchanged.

**Where things point.** A diagnostic has two locations:

- `source` stays the **call site**: the `load_data` call in the Markdown
  document, under that document's position rules.
- A new optional `data_location` field points **into the data**:

  | Field | Value |
  |---|---|
  | `file` | the resolved data file (portable path) or URL |
  | `pointer` | the JSON Pointer of the offending value, for example `/0` |
  | `line`, `column` | 1-based; the column counts characters, not bytes; unaffected by CRLF line endings (the same convention as `source`) |
  | `byte_offset` | zero-based offset into the original bytes, only for invalid UTF-8; no line or column accompanies it |

  `data_location` is present for `dm.data.malformed` and
  `dm.data.contract_mismatch`, and absent (not `null`-filled) otherwise.
  Data loaded from a URL gets line and column too, computed from the fetched
  text.

  **Malformed input has no value tree.** Its `pointer` is absent unless
  the parser proves one; do not invent an empty pointer to mean "unknown."
  Parser-proven line and column remain available even for YAML containing
  aliases or multi-line flow collections. That YAML limitation concerns
  locating contract violations in a successfully parsed tree, not parser
  errors. A transport failure, including UTF-8 rejection by the existing
  remote runtime before text reaches this function, retains its expression
  evaluation failure and has no `data_location`.

  **Line breaks** are LF or CRLF. A lone CR (not followed by LF) is not a
  line break.

  **Pointer only.** Every format reports a line and column, with one
  exception: a YAML file that contains an alias (`*name`) or a flow
  collection spanning more than one line (`servers: [` … `]`). For such a
  file `data_location` carries `file` and `pointer`, and `line` and `column`
  are absent. A guessed position is never reported in their place.

  For a missing member or tuple slot, keep the missing value's pointer but
  locate the nearest existing parent container. Locate an unexpected key at
  its value. If the format cannot prove that parent's position, omit line
  and column. For CSV/TSV, a field position is its raw start, including an
  opening quote; a row-level failure points to the record start. Columns
  are counted on the physical line containing that start.

  **Review note:** these rules separate syntax errors, which may have no
  pointer, from contract errors, which always have one. They also replace
  the earlier conflicting claims that all CSV positions use the record's
  start line and that cell positions use raw field offsets.

**Expected and found values.** `expected_type` and `actual_type`, which the
record already has, are filled structurally (for example `number` and
`string`). The found value itself (`"macOS"`) appears only in the human
`message`; no field is added for it.

**Many violations.** A contract mismatch is still **one** composition
failure, but its diagnostics list every violating location, one record
each, up to a cap of **20**. If there are more,
one further record of the same code says how many were not listed. Records
are in **file order** — array indexes ascending, object keys in the order
the file writes them, union arms in the order `as` declares them — so the
cap keeps the first 20 violations in the file.
Preserve authored mapping order separately from the JSON value if the parser
or conversion sorts keys. Missing members have no source order: list them
after present members at the same parent, in contract declaration order.
The overflow record points at the call site, omits `data_location` and
expected/found types, and is not counted as another violating location.

**From engine failures to records.** The engine reports a tree: a value
that matches no arm of a union carries one failure list per arm. The helper
flattens that tree into one record per location:

1. **A failure that is not a union failure** becomes one record at its path.
2. **A union failure:**
   1. drop the `null` arms (so a `null` arm never appears in
      `expected_type`);
   2. an arm is **reachable** unless it failed with "no conversion" *at the
      union's own path*, which means the value's kind cannot become that
      arm's top-level kind at all;
   3. if exactly one arm is reachable, descend into that arm's failures;
   4. otherwise (no reachable arm, or two or more), emit **one** record at
      the union's location whose `expected_type` lists the remaining arms.

For example, `"string[] | {a: string}"` over the YAML object `{a: [1]}`:
the `string[]` arm cannot accept an object at all, so only the object arm is
reachable, and the record is `/a`, expected `string`, actual `array`.

**Field vocabulary.**

| Situation | `expected_type` | `actual_type` |
|---|---|---|
| a required value is missing (for example a short tuple row) | the missing slot's type | `missing` |
| an unexpected key or extra element | `absent` | the found value's kind |
| any other leaf | the engine's expected type | the found value's kind |
| any record at pointer `""` | the `as` argument as written | the found value's kind |

Value kinds use the schema vocabulary: `string`, `number`, `boolean`,
`null`, `array`, `object` (the engine's `text` becomes `string` and `list`
becomes `array`).

**Known limit.** When two union arms are containers of the same kind
(`{a: string}[] | {b: number}[]`, or `number[] | boolean[]`), both are
reachable, so data that fits neither gives one record at pointer `""`, not
one per row. Nothing in the data says which arm the author meant; blaming
rows against the arm with the fewest failures was considered and rejected.
The record's `message` carries the engine's per-arm text, cut to a few
lines.

**Tracing.** Each call opens a tracing span recording the resolved path or
URL, the format, the byte count, the row or item count, and whether the
per-run memo (R7) answered it. Nothing extra is printed: in a data output
(`--output json`) the CLI's tracing is already disabled, and in other modes
the span is visible only at the usual verbosity.

**Design notes — records**

- Build every `dm.data.*` record in `compose/report_diagnostics.rs`, like
  every other compose diagnostic. These codes are errors only, so they are
  not added to `COMPOSE_DIAGNOSTIC_CODES`
  (`darkmatter/lib/src/markdown/compose/policy.rs`), which lists the
  warnings `--deny` can make blocking. A code is added there only if it is
  ever emitted as a warning.
- Add `data_location` to `Diagnostic` and to the `diagnostic` type of **every
  output schema a `load_data` failure can reach** (pre-authorization (d)).
  Seventeen files under `darkmatter/cli/schemas/output/` declare their own
  closed copy of that type. `compose.yaml` and `render.yaml` are the obvious
  two, and `graph.yaml` and `validate-refs.yaml` follow because those commands
  evaluate expressions while preparing documents. Check the rest while
  implementing; a closed type rejects a field it does not list.
- **Why not `validate_soft`.** When the coercion engine cannot build a
  partial copy of a value (it cannot when any key or tuple element is
  undeclared), `validate_soft` runs the JSON Schema validator over the
  *unconverted* value. Its problems then include false positives: a CSV
  with one undeclared column reports every numeric cell in the file
  (`"8080"`, `"1"`) as "not of type number". For CSV an undeclared header
  column is the common case. See [Defects found](#defects-found).
- **Ownership of the flattening.** simplified-schema owns it, as a helper
  such as `ConversionError::located_failures()` returning each location's
  path, expected text, value kind, and failure in engine order. It merges
  with the private `coerce.rs::flatten_failures`, which already looks
  through a union whose only non-null arm failed, so frontmatter annotation
  and `load_data` use one rule. It reuses an existing JSON Pointer encoder
  (`validation.rs::JsonPointer` or `coerce.rs::pointer`) rather than adding
  a third. Darkmatter owns policy: the code, the cap and overflow record,
  `data_location`, the `as` text at pointer `""`, and the message wording.
  The schema-ownership guard
  (`darkmatter/lib/tests/l1/schema_language_ownership_guard.rs`) keeps
  coercion interpretation out of darkmatter.

**Design notes — locators**

Each locator turns a JSON Pointer into a byte offset in the source text (the
text after the byte-order mark is stripped). One shared helper turns a byte
offset into a 1-based line and character column; it lives in biscuit-file
and has no feature gate. Darkmatter selects the locator by format, converts
its offset with the helper, and fills `data_location`; a locator that
returns nothing means pointer only.

| Format | Locator | Owner |
|---|---|---|
| YAML | `PositionMap::project` (`simplified-schema/lib/src/positions.rs`), relaxed to accept a list or scalar root; the start of `PositionMap::byte_range` converted by the shared helper | simplified-schema |
| JSON | walk the pointer with `serde_json::value::RawValue`; the value's offset is its slice's address minus the source's (the technique `numeric/ingress.rs` already uses); for a duplicate key, the last occurrence | biscuit-file |
| JSONL | the JSON locator on the record's physical line, plus that line's start offset; blank lines are skipped but keep line numbering | biscuit-file |
| CSV, TSV | the delimited reader's raw field offsets (R3) | biscuit-file |
| TOML | `toml::de::DeTable::parse`, whose keys and values carry byte spans | biscuit-file |
| JSON5 | a recursive descent over the byte spans from `json_five::tokenize_str` | biscuit-file |

- **YAML columns are bytes in `PositionMap`.** `ValidationProblem.column`
  counts UTF-8 bytes, so `data_location` never copies it; it converts the
  start of `byte_range` instead.
- **Relaxing the root check** affects only `project`. Frontmatter whose root
  is not a mapping is rejected before validation, so frontmatter behavior is
  unchanged; the existing `positions.rs` tests plus a list-root and a
  scalar-root case confirm it.
- **The pointer-only case.** For a YAML file with an alias or a multi-line
  flow collection, `project` refuses the whole file. Its fallback,
  `PositionMap::top_level`, reports top-level keys at column 1; that guess is
  never reported as `data_location`.
- **JSON does not go through the YAML locator.** The YAML locator is
  line-based and fails on pretty-printed JSON, tab indentation, duplicate
  keys, and surrogate-pair escapes.

**Acceptance criteria**

- The `number[]` example fails with three `dm.data.contract_mismatch`
  records, one for each operating system. The first has:
  `source` at the call, `data_location` `{file: …/os.yaml, pointer: "/0",
  line: 1, column: 3}`, `expected_type` `number`, `actual_type` `string`,
  and `"macOS"` in the message.
- One contract-failure fixture per format reports the correct pointer, line,
  and column: YAML, JSON (pretty-printed), JSONL (with a blank line before
  the bad record), CSV, TSV, TOML (inside an array of tables), and JSON5.
- A YAML fixture with an alias, and one with a multi-line flow sequence, each
  report `file` and `pointer` with no `line` or `column`.
- A JSON fixture with a duplicate key reports the position of the last
  occurrence.
- A CSV fixture with a quoted field containing the delimiter, a doubled
  quote, and a newline before the bad cell reports the cell's raw column.
- A fixture with a multibyte character before the bad value, and a CRLF
  fixture, report the same column as the LF/ASCII equivalent, counted in
  characters.
- `"{ name: string, port: number }[]"` over a CSV with an undeclared column
  yields exactly one record per row at `/i/<column>` with `expected_type`
  `absent`, and **no** record for a `port` cell that converts.
- Several bad rows yield one record each in file order (`/1/port`,
  `/2/port`, …); a 25-row fixture with a bad cell in every row yields
  records for rows 0–19 plus one record reporting 5 more.
- `"number[] | null"` over `["macOS"]` reports `/0`, `number`, `string`; the
  null arm never appears in `expected_type`.
- `"{a: string}[] | {b: number}[]"` over data that fits neither arm yields
  **one** record at pointer `""` with `expected_type` equal to `as` as
  written and `actual_type` `array`.
- `"string[] | {a: string}"` over the YAML object `{a: [1]}` reports `/a`,
  `string`, `array`.
- A missing tuple slot reports `actual_type` `missing`; an extra element or
  key reports `expected_type` `absent`.
- A URL-loaded fixture reports `data_location` with the URL, pointer, line,
  and column.
- A missing file, and a URL from a denied host, each produce
  `dm.expression.evaluation_failure` with `reason`, `construct`, `reference`,
  and `failure` set.
- `frontmatter`, `markdown_title`, `markdown_body_empty`, and `validate_schema`
  on a missing file report the same four fields, each naming its own function
  in `construct`.
- The human stderr report of a missing file is byte-identical to before.
- Each `dm.data.*` code in the table has at least one test.
- Invalid local UTF-8 reports `data_location.file` and `byte_offset`;
  malformed JSON has a parser position without an invented pointer. Missing
  members retain their missing pointer and locate the existing parent.
- Keys containing `/`, `~`, or an empty string round-trip through RFC 6901
  pointers and their locators. Two bad object fields written in reverse
  alphabetical order are reported in authored order.

### R6. Remote reads

**Requirement**

`load_data` is a read-side remote function. An HTTP(S) argument is fetched
only under `--allow host=HOST`, through `biscuit_file::FetchPolicy`, exactly
like `try_frontmatter`. Deny-all stays the default. A URL computed at run
time (not a string literal) behaves exactly as it does for `frontmatter()`
today. The format comes from the URL path's extension (R3).

**Design notes**

Add `load_data` to `REMOTE_READ_FUNCTIONS` in
`darkmatter/lib/src/markdown/compose/remote.rs`.
Authorization is checked before consulting the data memo, as well as before
transport-cache access. A shared slot never grants a less privileged context
access to an earlier result. Retain the remote runtime's redirect, freshness,
and response-cache rules; the data memo adds no independent fetch path and
persists no parsed result. Named schema imports retain their existing local
and shipped-schema resolution rules; allowing a data host does not authorize
remote schema imports.

**Acceptance criteria**

- A URL with no `--allow` fails without a network request, with the
  file-reference fields (R5).
- A URL with `--allow host=HOST` is fetched once and loaded.

### R7. Repeated reads within one command

**Requirement**

Within one command (for `md compose`: the reference check, the pre-flight walk,
and composition together), the same resolved file (canonical path or URL)
loaded with the same **resolved** `as` is read, parsed, and validated
**once**; later calls reuse the result. There is no file-size limit.

**Anything that may have changed a file ends the reuse.** The memo is emptied
each time a command has run: a frontmatter `$(...)`, a `::shell` directive, a
shell block, or a lifecycle step. The reference check runs no command, so
without this rule a file a frontmatter command regenerates would be served
from the check's earlier read.

```markdown
---
made: "$(./regenerate-hosts)"
---
{{ load_data("hosts.yaml", "string[]") }}
```

Here the check reads the old `hosts.yaml`, the command runs, and composition
reads the new one: two reads, and the correct data. Lifecycle events behave the
same way: `initialize` runs after the frontmatter bootstrap, so a read in the
bootstrap is not reused after it.

A result that only means "not possible yet" is never remembered. The reference
check has no remote permission, so a remote read there defers to composition
rather than storing a failure composition would inherit.

"Resolved `as`" matters for named imports: `"Os@./schemas/os.yaml"` written
in two documents in different folders can name two different files, so the
memo key uses the resolved type, not the text as written.

Why: a `{{ load_data(…) }}` inside a `::map` body is evaluated once per item
(only the collection source itself is evaluated once), so without a memo a
100-item map reads the same file 100 times. The author's decision is to add
no file-size cap. Iteration limits bound later collection traversal and output;
they do not bound the bytes fetched, parser allocations, alias expansion, or
the number of distinct memo entries. Document that distinction and retain
existing parser and coercion depth limits; this feature promises no fixed
memory bound for an arbitrarily large input. No additional performance spike
is needed to establish that limitation.

> **Decided (2026-11-06). Read once.** A file is read and parsed once per command, and
> validated once per contract, **in every cache access mode** (`CacheAccessMode`, a
> library-only setting). `Refresh` and `Off` govern the compose cache, not this memo: I/O
> is not cheap, and repeating a read within one command answers no question. The
> re-read after a command, above, is the one exception, because the file may have changed.

Two levels make this hold: the source (read, decode, parse) is keyed by the resolved
file or URL, and the bound result by that source and the resolved `as`. So
`load_data(f, "a[]")` and `load_data(f, "b[]")` read `f` once and validate twice.

**Design notes**

- The memo is **one options-level shared slot for the whole command**: an
  `Arc` held on `ComposeOptions`, cloned into both expression-context builders
  and shared by every pass, on the pattern of `remote_fetch_runtime`. A library
  caller that supplies none gets a fresh one per root run, so separate
  compositions never share results. Nothing persists.
- It does **not** follow the `ResolutionContext.provider_queries` pattern
  (`darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs`). That
  slot is created per `ResolutionContext` unless a remote runtime exists, so
  a memo there would be per evaluation surface, and a `::map` callback and
  the body could each read the file.
- Each memo entry is single-flight through a `OnceLock`: the first caller
  loads, concurrent callers for the same key wait for that result. Loading
  data never evaluates another `load_data` call, so there is no re-entrancy
  to guard against.
- Reuse identity includes the resolved data source, format, resolved contract
  and complete definition graph, and the context needed to interpret that
  contract. Equal type text is insufficient when imports or context-sensitive
  constraints differ. Use existing identity helpers; any new fingerprint uses
  biscuit-hash. Keep native path identity separate from portable display
  text, using biscuit-file's canonicalization and portable-path helpers.
- Cache source-independent values or typed causes, never a fully located
  diagnostic. Each caller supplies its own call span, document, iteration
  path, and authored `as` text, even when another caller populated the entry.
  Preflight-only deferrals and failures that grant no usable result are not
  memoized; successful results are the required reuse unit.
- Invalidation advances a generation as it removes entries. A load already
  in progress may finish for its original caller but cannot populate the new
  generation. Invalidate after each executed command attempt, including a
  failed command that may have written files, before any next expression.
  Skipped commands do not invalidate. A lifecycle invalidation is shared by
  every request participating in that command. Concurrent external writers
  are outside this guarantee.
  The generation is a counter separating reads before and after a command;
  it prevents a late completion from restoring data the command invalidated.
- Local invalidation does not discard the existing remote runtime's URL
  snapshot. A command that changes a remote resource does not force a second
  fetch in this feature; a new root command gets a new remote snapshot.

**Acceptance criteria**

- A `::map` over three items whose body calls `load_data` on one file reads
  that file once (observed through a work counter or read-count hook).
- A call in a `::map` callback and a call in the body of the same document,
  on the same file and `as`, read the file once.
- The same file with a different `as` is read once and validated separately.
- The same relative named import written in documents in two folders, naming
  different schema files, is validated separately.
- Under each of `ReadWrite`, `ReadOnly`, `Refresh`, and `Off`, the same `::map` reads
  the file once and validates it once; a URL is fetched once.
- `md compose` on a file-backed document with a `load_data` call and a reference
  check reads the data file once across both passes.
- A frontmatter command that rewrites the data file before a body `load_data`
  call yields the rewritten data, with the read count showing the second read.
- A shell step inside a `::map` body causes the next item's call to read again.
- A URL read under `--allow host=HOST` succeeds when the reference check ran
  first and could not read it.
- Two library compositions with no caller-supplied slot do not share results.
- Repeated failures at two call sites retain each site's provenance; neither
  a data memo hit nor a prior allowed-host read bypasses current policy.
- An in-flight old-generation load cannot be reused after invalidation, and
  a failed shell attempt that writes a file also ends reuse.

### R8. Read set

**Requirement**

Each command keeps a **read set**: the resolved file paths and normalized URLs
that expressions read. `load_data` and every existing read-side expression
function (every member of `REMOTE_READ_FUNCTIONS`: `frontmatter`,
`try_frontmatter`, `file_exists`, `markdown_title`, `markdown_body_empty`,
`validate_schema`) report into it, on every surface where expressions are
evaluated. A call that fails after resolving its file still reports it.

The read set covers the whole command. A read in the reference check counts, and
emptying the R7 memo never removes an entry, so a failure in the check reports
the file it failed on even though composition never started.

The read set is a **library** feature. It is available on the compose
result and appears in the compose data envelope (`md compose --output
json`). DMLS does not use it in this spec.

Why: no expression function records what it reads today.
`referenced_files` exists only in schema resolution
(`darkmatter/lib/src/markdown/schemas/mod.rs`), and the composed-result cache
is per run, so there is nothing persistent to invalidate. DMLS is out of
scope because it has no stale result to fix yet: it never evaluates
expressions, it re-checks every open document on any watched change, and it
watches only `.md` and `.yaml` files. Using the read set for DMLS
invalidation belongs to 2026-11-05-dynamic-schema-types (its spikes S2 and
S3), which introduces evaluated results in DMLS.

**Design notes**

- The read set uses the same options-level shared slot as the R7 memo: an
  `Arc` sink on `ComposeOptions`, cloned into every expression context and
  copied onto the compose result when the command ends, whichever pass stopped
  it.
- **Coverage risk.** About 120 sites construct `ResolutionContext::new`
  directly. Many build a context with a default sink — for example the
  `ShortcutLookup` in
  `darkmatter/lib/src/markdown/compose/transclusion/conditions.rs` — so a
  `file_exists` inside a `when=` condition would go unrecorded if the sink
  were attached per context. Every construction site that evaluates
  expressions must receive the shared sink; a test per evaluation surface
  (below) proves it.
- Add a field for it to the envelope and to
  `darkmatter/cli/schemas/output/compose.yaml`; the compose conformance test
  covers it. The field is `read_set: string[]`, always present (an empty
  array when there were no reads), on both successful and failed compose
  envelopes. Like `data_location`, this extends an existing output contract.
- Store a deduplicated set and serialize it in stable lexical order, not
  concurrent completion order. Local entries use biscuit-file's canonical
  native identity internally and portable absolute path text at the output
  boundary. URL identity uses the existing remote runtime's normalization,
  retaining distinctions such as query parameters; do not invent a second
  normalization rule. The field is command output, never a new persisted
  remote manifest; existing persistence redaction remains unchanged.
- `file_exists` is an observation even when false. Record concrete local
  candidates actually probed when no file exists, using normalized absolute
  candidate paths because canonicalization cannot succeed. Record an
  existing resolved source before opening or parsing it, so permission and
  parse failures are included. An invalid reference with no concrete target
  contributes no invented path. A remote observation records the normalized
  URL even if policy denies the fetch; recording it does not authorize I/O.
- Record schema files actually read while resolving a `load_data` contract
  or running `validate_schema`, including transitive imports, using existing
  resolver provenance. This keeps the set faithful to the function's
  dependencies. Embedded schemas add no filesystem entry. Memo hits retain
  the dependencies of their original load. This remains an expression-read
  set, not an inventory of unrelated links, transclusions, or every file
  consulted anywhere in composition.

**Acceptance criteria**

- After a run that calls `load_data("a.yaml", …)`, `frontmatter("b.md")`,
  and `file_exists("c.md")`, the read set contains all three resolved files.
- One test per evaluation surface shows a read-side call recorded: body
  interpolation, frontmatter interpolation, conditions (`::block` and
  `when=`), `::map`/`::filter` callbacks, transclusion directive targets,
  and lifecycle expressions where they can call a read-side function.
- A URL read under `--allow host=HOST` appears normalized in the read set.
- A call that fails after resolving its file still records that file.
- A malformed data file that fails during the reference check appears in the
  failed envelope's read set.
- `md compose --output json` includes the read set, and the record passes
  the compose conformance test.
- DMLS behavior is unchanged; no DMLS test is added for the read set.
- Repeated reads contribute one entry; false `file_exists` probes, a denied
  URL, imported schema dependencies, and a command that fails before compose
  returns all obey the rules above. Empty and failed envelopes retain the
  same `read_set` field shape and ordering.

### R9. Optional data files

**Requirement**

There is no `try_load_data`. A malformed but present file always fails.
For a data file that may be absent, the documented pattern is:

```markdown
{{ file_exists(f) ? load_data(f, "string[]") : [] }}
```

Note that `load_data(f, "string[]") || []` does **not** recover: `||` does
not absorb errors (`darkmatter/lib/src/markdown/compose/expression/mod.rs`;
documented in `darkmatter/docs/topics/darkmatter-expressions.md`).

The same pattern applies to a file a frontmatter command creates. The passes
that run ahead of composition evaluate expressions before any command runs, so
an unguarded call on a file that does not exist yet fails before the command
can create it. `frontmatter()` already behaves this way.

The guard handles absence only. If an old file exists but is malformed or
fails `as`, the reference check stops before a command can replace it. A
computed import that is missing behaves the same way. Generate or repair
such inputs before invoking `md compose`; this feature does not postpone
known failures until after commands run. The reference check's throwaway
document prevents its old values from becoming final frontmatter state;
after an executed command, composition evaluates against the new memo
generation. Cover this ordering with a regression test.

**Acceptance criteria**

- The `file_exists` pattern yields `[]` for a missing file and the data for a
  present one.
- `load_data(missing, "string[]") || []` fails composition.
- The `file_exists` pattern over a present but malformed file fails.

### R10. Typed result from a literal `as`

**Requirement**

When `as` is a literal string, the static checks treat the result of
`load_data` as having the type `as` denotes, instead of `any`. When `as` is
computed, the result is `any`. The typed result flows into inline callbacks:

```markdown
{{ map(load_data("hosts.yaml", "{ name: string, port: number }[]"), h => h.name + ":" + h.port) }}
```

Here the passive check knows `h` is `{ name: string, port: number }`, and
DMLS offers `name` and `port` when completing after `h.`.

The same data can also reach a `::map` region through frontmatter:

```markdown
---
hosts: "{{ load_data('hosts.yaml', '{ name: string, port: number }[]') }}"
---
::map hosts
- {{ i.name }}:{{ i.port }}
::end-map
```

The typing of `i` there comes from the document's schema for `hosts`, not
from this requirement. (`::map` takes an identifier or dotted path as its
source, not an expression.)

**Limits**

- **Named imports.** The static type (`Shape`) carries no table of named
  definitions, so a reference inside `as` — a `Name@darkmatter:<stem>`
  import, a file import, or a recursive type — projects to unknown unless
  its definition is written inline. A file import is unknown to the passive
  check anyway, matching its provisional acceptance (R1).
- **No member-level undeclared check.** A misspelled member such as
  `h.nmae` is **not** reported. No such check exists anywhere today: for a
  schema-typed frontmatter value, `{{ server.nmae }}` is silent, while only
  an undeclared top-level name such as `{{ sever.name }}` warns. A member
  check is a follow-up spec (see [Follow-ups](#follow-ups)).

**Design notes**

- Add a `load_data` arm to `expression_type`
  (`darkmatter/lib/src/markdown/compose/expression/call_check.rs`), which
  already special-cases `map` and `filter`. The arm parses the literal `as`
  and returns its shape; `map`/`filter` then give the callback parameter the
  element type.
- DMLS completion reads the same typing, so no DMLS-specific typing is
  added.
- `X | null` from an `as` such as `"string[] | null"` keeps the null arm, so
  the existing null-handling rules apply.
- The file argument's nullability also matters. A definitely null file gives
  static type `null`; a possibly null file gives the denoted type unioned
  with `null`; a proven non-null file gives the denoted type alone. A literal
  contract cannot erase the signature's null passthrough. Invalid literal
  contracts remain errors even for null files under the passive call check.

**Acceptance criteria**

- In the inline-callback example, DMLS completion after `h.` offers `name`
  and `port`.
- With a computed `as`, completion after `h.` offers nothing from `load_data`
  and nothing is reported (the result is `any`).
- `expression_type` of `load_data(f, "{ a: string }[]")` is an array of
  `{ a: string }` when `f` is non-null, and that array unioned with null
  when `f` may be null. A named file import projects to unknown.
- A definitely null file has result type null and opens neither the data
  file nor any imported contract file. No static type inspection reads one.

### R11. Balanced, parse-gated `{{ }}` scanning

**Requirement**

Today `scan_legacy_expression`
(`simplified-schema/lib/src/expression/scan.rs`) ends an expression at the
first unnested `}}` anywhere, so

```markdown
{{ load_data(f, "{ a: { b: string }}") }}
```

is cut off after `string }}` and fails, and so is the object literal
`{{ {a: {b: 1}} }}`, whose `}}` is not inside a string at all.

The new rule:

1. While looking for the closing `}}`, the scanner skips single- and
   double-quoted strings using the expression lexer's escape rule
   (`read_string` in `simplified-schema/lib/src/expression/lexer.rs`: `\`
   consumes the next character, and strings may span lines).
2. A `}}` closes the expression only when every single `{` opened inside it
   is closed.
3. Preserve today's span first if its text parses. Otherwise the scanner
   uses the new span only when it differs from today's span **and** its text
   parses as an expression. Otherwise — no closing `}}`, an
   unterminated string, an unbalanced `{`, or a parse failure — it uses
   today's span unchanged.

The invariant: **no span that parses today changes, and a changed span
always parses.**

A purely quote-aware scan (rule 1 alone) was **rejected**. In prose, an
apostrophe opens a "string" that can run for many lines and swallow working
expressions. For

```markdown
{{ don't }} it's fine {{ x }} and more }}
```

today's scan finds `don't` and `x`; a quote-aware scan finds one span,
`don't }} it's fine {{ x }} and more`. The parse gate keeps today's spans.

The rule reaches every place that finds expressions: composition,
frontmatter, DMLS, and claudine's sequence sources. Several of those places
have their own `{{`/`}}` detection today rather than calling the scanner;
they are routed through it (design notes). This touches claudine, a second
package area.

**Design notes**

- **Corpus result (SP1).** Over 6,909 repository files holding 10,399
  expressions, only 6 files change, and every change turns a span that did
  not parse into one that does. No file under any `prompts/`,
  `tests/fixtures/`, or `docs/` folder changes.
- The parse gate makes the scanner call `expression::parse`, which lives in
  the same module, so no layering changes. Update the module doc in
  `scan.rs`, which says the scan "is purely lexical". The parse runs only
  where spans differ. Use the parser's existing recursion limits and avoid
  repeatedly rescanning a suffix from each closing-delimiter candidate.
  The completed corpus check is sufficient; no new performance study is
  required.
- Unchanged: `{{{ … }}}` literals still close at the first `}}}`, even inside
  quotes, and `{{{a: 1}}}` with no space is a literal, not an object
  expression.
- Trade-off: an expression that has a real syntax error **and** a `}}`
  inside a string still truncates at that `}}`, so its error describes the
  truncated text. That is today's behavior.
- **Detectors routed through the shared scanner** (for example through a
  helper exported from `scan.rs` that says whether an offset is inside an
  unclosed `{{`), never through a second quote rule:

  | Site | Today | Priority |
  |---|---|---|
  | DMLS `completion_partial` (`darkmatter/dmls/src/overlay/expressions.rs`) | treats any `}}` before the cursor as closing, so no completion after a `}}` inside a string | must |
  | `mask_interpolations` (`darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs`) | closes at the first `}}`, which corrupts the shell-quote tracking of `$(cmd {{ f("}}") }})` | must |
  | claudine `whole_value_span` (`claudine/lib/src/composition/sequence/grammar.rs`) | treats `sequence: "{{ … }}"` as an expression only if the inner text has no `}}`, so a nested type is misread as a file reference | must |
  | DMLS `rfind("{{")` in `darkmatter/dmls/src/providers/dsl.rs` and `darkmatter/dmls/src/providers/frontmatter.rs` | a quoted `{{` earlier in the expression moves the scope start into the string | should |
  | `protect_interpolation_expressions` (`darkmatter/lib/src/markdown/frontmatter.rs`) | a second copy of the legacy closing rule, used only when YAML parsing fails | should |

  Presence checks such as `contains("{{")` only decide whether text may hold
  an expression, not where it ends, and are unaffected.

  Editor completion also needs an unfinished expression, which cannot pass
  the complete-expression parse gate. The shared scanner must expose cursor
  state separately: determine whether the cursor is within a quoted string,
  nested brace, or unfinished expression using the same lexical rules,
  without accepting unfinished text as a valid composition expression.
  This is how type completion remains possible before the final `}}` exists.

**Acceptance criteria**

- The `load_data` example above composes, and the same text in DMLS gets
  diagnostics and hover for the whole call.
- `{{ {a: {b: 1}} }}` yields the object `{a: {b: 1}}`.
- `{{ "}}" }}` yields the string `}}`; `{{ '{{' }}` yields `{{`; an escaped
  quote inside a string (`"a\"}}"`) does not end the string.
- A string spanning lines, `{{ f("multi⏎line }}") }}`, yields the whole call.
- `{{ don't }} it's fine {{ x }} and more }}` finds `don't` and `x`, exactly
  as today; so does `{{ "open }} then {{ y }} and " more }}` (finding
  `"open` and `y`).
- Prose containing `` `contains("{{")` `` followed further down by valid
  expressions still finds those expressions.
- A `{{` with an unbalanced quote and no closing `}}` after it scans exactly
  as it does today.
- DMLS completion works with the cursor after
  `load_data(f, "{a: {b: string}}") ` inside an open `{{`.
- `$(cmd {{ f("}}") }})` in frontmatter shell expansion keeps the command's
  shape.
- In claudine, `sequence: "{{ load_data(f, \"{a: {b: string}}\") }}"` is an
  expression source, not a file reference.
- Bounded L1 regression fixtures preserve the changed expression forms from
  the completed corpus check and the prose counterexamples above. Assert
  that every changed span parses and its old span did not, and preserve any
  old span that already parses. Read only explicitly declared shipped inputs
  or frozen test fixtures; do not rescan the working repository or include
  internal `prompts/` trees in ordinary tests. The completed spike remains
  the record of the broad corpus comparison.

### R12. Documentation

**Requirement**

- A function page generated from the catalog, plus a short topic section
  showing each format, the R3 and R4 input rules (including the CSV cell
  vocabulary: `yes`/`on`/`1` read as `true`, `1_000` as `1000`), the R9
  optional-file pattern, the R10 typed result and its limits, and that `as`
  is a quoted string.
- **Rewrite the `type-definition` documentation.** The current material in
  `docs/topics/schemas/definition.md` (Semantic Meta-Types) explains carrier
  vs. denoted types but never shows a newcomer which spellings are accepted,
  which constraints apply (stated far below, under Semantic Arrays), or how to
  tell a container from a scalar. `load_data` makes `type-definition` a
  user-facing argument type, so the page leads with what a reader can write,
  gives one compact example per form (including `tuple([…])` versus a bracket
  union), and adds a "container types" section that R2 links to.
- The `dm.data.*` codes, the `data_location` field (including the YAML
  pointer-only case and the line-break rule), the file-reference fields on
  expression failures, and the read-set field are documented on
  `docs/cli/data-contracts.md`. The stopping rule for a data failure, including
  that it can stop a command in the reference check, is on
  `docs/errors/composition-problems.md`.
- The read set (R8) and the per-command memo, including that it holds in every cache
  access mode, and its emptying rule (R7), are described on the relevant compose topic
  page.
- The expression-syntax page states the R11 rule in plain terms: a `}}`
  inside a quoted string, or inside a nested `{ … }`, does not end an
  expression; and, next to the object-literal example, that `{{{ … }}}` is
  still a literal.

**Acceptance criteria**

- The function page renders from the catalog with at least one example per
  format family.
- The `type-definition` page has a "container types" section matching R2.
- The data-contract page lists every `dm.data.*` code and the
  `data_location` and read-set fields.
- No `docs/` page names this spec.

## Pre-authorizations

Approved by the author on 2026-11-05. Implementation may do each of these
without pausing for approval.

- **(a) biscuit-file.** Add `csv` and `csv-core` behind a new opt-in feature
  (not in `default`) and record both in `docs/dependencies.md`; make
  extension detection public; reject NaN and infinity in the TOML-to-JSON
  path. The TOML change needs care: other `toml_to_json` callers must not
  change behavior silently (R3).
- **(b) Shared simplified-schema edits.** Relax `PositionMap::project` to
  list and scalar roots; add the `container` option to `type-definition`;
  the parse-gated scanner (R11), including routing the darkmatter and
  claudine detectors through it; the union-failure flattening helper (R5).
- **(c) Other specs.** Edit 2026-11-05-dynamic-schema-types to the quoted
  `as` form. File these follow-up specs as dated active directories under
  `darkmatter/features/`: a general recovery construct (`try(...)`);
  headerless CSV/TSV with an options or format override; a member-level
  undeclared-property check; and YAML locator hardening (see
  [Follow-ups](#follow-ups)).
- **(d) Shared record shape.** Add `data_location` to the `Diagnostic` record
  and the read-set field to the envelope, in every output schema they can
  reach, in their conformance tests, and on the data-contract page. Fill the
  file-reference fields on expression failures for every read-side function
  (R5).

## Defects found

- **`validate_soft` false positives for nested containers.** When a value
  has an undeclared key or an extra tuple element anywhere inside a nested
  container, the coercion engine cannot build its partial copy, and
  `validate_soft` judges the whole unconverted value with the JSON Schema
  validator. It then reports convertible text (such as `"8080"` against
  `number`) as a type error. This affects frontmatter with nested
  containers today. `load_data` avoids it by building records from the
  engine (R5). Implementation fixes this defect in the shared failure
  projection while preserving the existing successful-conversion behavior.
  A regression test combines an undeclared nested key with convertible
  numeric text and verifies that only the actual violation is reported.
  Filing the known defect does not satisfy implementation completion.

## Spikes and risks

All three spikes are done (2026-11-05):

- **SP1. Scanner corpus diff (R11)** — the quote-aware scan as first
  written regresses prose; the balanced, parse-gated rule changes only 6 of
  6,909 files, all from non-parsing to parsing spans. Notes:
  [sp1-scanner-corpus-diff.md](spikes/sp1-scanner-corpus-diff.md).
- **SP2. Line/column per format (R3, R5)** — every format can be located;
  the only pointer-only case is YAML with an alias or a multi-line flow
  collection. Notes:
  [sp2-line-column-per-format.md](spikes/sp2-line-column-per-format.md).
- **SP3. Union failure flattening (R5)** — records must come from the
  coercion engine's `ConversionError`, flattened by a new simplified-schema
  helper, not from `validate_soft`. Notes:
  [sp3-union-failure-flattening.md](spikes/sp3-union-failure-flattening.md).

Remaining risks:

- **Shared simplified-schema code.** The relaxed `PositionMap::project`
  root, the scanner, and the merged flattening helper are all used by
  frontmatter. Existing tests plus the new list-root, scalar-root, and
  bounded scanner regression fixtures guard them.
- **Cross-package scanner change.** R11 changes claudine's sequence grammar.
  Run claudine's `just test` alongside darkmatter's.
- **Read-set coverage.** A context built without the shared sink silently
  drops reads (R8); the per-surface tests are the guard.

## Testing

- **Formats (L1):** one success fixture per extension in R3, including an
  upper-case extension, an empty YAML file (→ null, accepted by
  `"string[] | null"`), and a URL path with a query and fragment.
- **Malformed input (L1):** every row of the R3 malformed-input table, with
  character-column positions.
- **Container rule (L1, simplified-schema):** every row of both R2 tables
  through `denotes_container` and the `type-definition(container)` format
  check, with the bracket-union case.
- **Passive check (L1):** a scalar `as`, a disagreeing CSV union, a scalar
  CSV element, and `Name@this` are each rejected with their code, the `as`
  span, and no file read; a file named import passes provisionally without
  I/O.
- **Strict CSV/TSV (L1):** every row of the R4 table, including ragged rows,
  duplicate and empty headers in both modes, empty cells, quoted newlines
  (record and field positions), invalid quoting, leading blank lines, TSV
  quoting, and the cell-conversion fixture.
- **Locators (L1, biscuit-file and simplified-schema):** per-format pointer
  to position, including duplicate JSON keys, CSV quoting, a TOML array of
  tables, multibyte and CRLF twins, YAML list and scalar roots, and the two
  YAML pointer-only cases.
- **Diagnostics (L1):** one contract failure per format asserting
  `data_location` with pointer, line, and column; the two YAML pointer-only
  cases; the union-flattening criteria of R5; the 25-violation cap case; and
  a URL-loaded fixture with positions.
- **Memoization (L1):** the R7 `::map` read-count test, the callback-plus-body
  case, the different-`as` and different-folder import cases, every cache access
  mode reading once, one read across the reference check and composition, and the memo
  emptied after a frontmatter command, a `::shell`, and a lifecycle step.
- **Read set (L1):** the R8 criteria for `load_data` and the other
  read-side functions, one test per evaluation surface, a URL, and a failure in
  the reference check.
- **Typed result (L1 + DMLS):** the R10 criteria, including one DMLS
  completion test after a typed callback parameter.
- **Scanner (L1, simplified-schema, darkmatter, DMLS, claudine):** the R11
  criteria and bounded old-versus-new regression fixtures; no working-tree
  or internal-prompt scan.
- **Optional files (L1):** the R9 criteria, including regeneration ordering
  and a malformed old file that fails before a command runs. Shell fixtures
  use the shared test toolkit and portable helper executables, not a POSIX
  shell command assumed to exist on native Windows. Editor tests stay
  headless and never activate terminal or browser windows.
- **End to end:** `md compose` through the normal invocation path for one
  success and one contract failure. The failure case runs
  `md compose --output json` and asserts exit status 1, `outcome:
  "failure"`, nothing on stderr, a `dm.data.contract_mismatch` record with
  `data_location`, and that the output passes the compose conformance test.
  The default (Markdown) mode asserts the rendered diagnostic on stderr.
- **Remote:** wiremock tests for a denied host and an allowed host.
- **Shipped corpus:** the catalog entry passes the passive shipped-artifact
  corpus test.

## Definition of done

- Every acceptance criterion in R1–R12 has a passing test.
- The implementation log cites the completed scanner corpus comparison and
  records the bounded regression results. It names the known
  pointer-only case — YAML files containing aliases or multi-line flow
  collections — which the docs name too (hardening is a follow-up).
- `just test` and `just lint` pass in `darkmatter/`, `simplified-schema/`,
  `biscuit-file/`, and `claudine/`. The shipped-schema corpus carves out
  `sandboxing.yaml`, whose enum slot 2026-11-05-dynamic-schema-types delivers;
  that test fails if the file starts to validate, so the carve-out cannot
  outlive the feature.
- `compose.yaml` and its conformance test include `data_location` and the
  read set; every other output schema a `load_data` failure can reach includes
  `data_location`.
- The catalog entry, generated function page, topic section, data-contract
  page, and rewritten `type-definition` page are in place;
  `docs/dependencies.md` records `csv` and `csv-core`.
- The `validate_soft` defect is fixed with its regression test (see
  [Defects found](#defects-found)), and the follow-up specs in
  pre-authorization (c) are filed.
- The `darkmatter`, `simplified-schema`, and `biscuit-file` skills mention
  `load_data` and the `dm.data.*` codes; the `container` option, the
  parse-gated scanner, and the flattening helper; and the new `DataFormat`
  variants, the delimited reader, and the locators, respectively.

## Resolved decisions

Author decisions were made on 2026-11-05 and 2026-11-06. The inline-review
clarifications below are dated separately; they preserve those decisions.

- **How is `as` written?** As a quoted string, `"string[]"`. Unquoted type
  syntax collides with variables, indexing, object literals, and the
  `number()` function in today's expression grammar.
- **(old Q1) TOML and JSON5?** Supported, aiming for full line/column
  reporting. biscuit-file already parses both, and consistency across
  formats matters more than the extra locator work.
- **(old Q2) A non-failing `try_load_data`?** No. `file_exists(f) ? … : []`
  covers the absent-file case, and a general recovery construct belongs in
  its own spec rather than one function at a time.
- **(old Q3) Unions of containers?** Allowed when every non-null arm is a
  container; for CSV/TSV all arms must agree on row shape. This keeps
  `X | null` usable for empty files without making R4 ambiguous.
- **(old Q4) Named type imports?** Allowed; the passive check accepts them
  provisionally and execution resolves them. Reusable contracts are worth it,
  and the `forms(…)` check already sets the provisional precedent.
- **(old Q5) File-size limit?** No limit; repeated reads are memoized per run
  instead. Existing iteration limits already bound downstream work, and the
  real cost was re-reading inside `::map`.
- **Read set (R8): build now or later?** Build now, covering the existing
  reading functions as well as `load_data`.
- **(O5) Read-set scope.** A library read set only: resolved paths and
  normalized URLs, reported by `load_data` and every `REMOTE_READ_FUNCTIONS`
  member, exposed on the compose result and the data envelope; DMLS
  unchanged. DMLS evaluates no expressions and re-checks every open document
  on any watched change, so it has no stale result to fix yet.
- **(O1, O2) CSV/TSV and malformed input.** Strict rules (R3 and R4 tables);
  headerless files are not supported. Silent guessing hides data errors, and
  every strict rule can be relaxed later without breaking a document.
- **(O3) The `}}` scanner hazard.** Fixed in the scanner rather than
  documented as a workaround, which would surprise every user who writes a
  nested object type. The rule is settled by SP1 (next entry).
- **(SP1) Scanner rule.** Balanced and parse-gated (R11). The purely
  quote-aware form is rejected because an apostrophe in prose swallows
  working expressions; the gate guarantees that no span that parses today
  changes.
- **(SP1) Detectors.** Route the darkmatter and claudine `{{`/`}}`
  detectors through the shared scanner rather than adding a second quote
  rule, so one closing rule governs every surface.
- **(O6, SP2) Line/column feasibility.** Every format gets a locator (R5).
  Only YAML with an alias or a multi-line flow collection reports the
  pointer alone, and a guessed position is never reported instead.
- **(SP2) Locator ownership.** biscuit-file owns the JSON, JSONL, CSV/TSV,
  TOML, and JSON5 locators and the shared byte-offset-to-character-column
  helper; simplified-schema owns YAML; darkmatter selects and fills
  `data_location`. Each locator sits beside the parser its format already
  uses.
- **(SP2) Line breaks.** LF or CRLF; a lone CR is not a break. Such files
  are rare, and this matches the existing helpers.
- **(SP3) Source of contract records.** The coercion engine's
  `ConversionError` through a simplified-schema flattening helper, not
  `validate_soft`, which reports false positives when any key or element is
  undeclared.
- **(SP3) Flattening, vocabulary, and order.** As stated in R5. Same-kind
  container unions give one root record; picking an arm by fewest failures
  was rejected because it can blame rows for an arm the author never meant.
- **CSV cell conversion.** The shared engine's existing rules, pinned by a
  fixture and documented, not changed (R4). One conversion rule for
  frontmatter and data files.
- **Result type.** Typed from a literal `as`; `any` from a computed one
  (R10). The caller already wrote the type, so completion should use it.
- **(R10) Where typing applies.** To inline callbacks through a `load_data`
  arm in `expression_type`; DMLS completes members after a typed callback
  parameter. `::map` sources are identifiers or dotted paths, so a
  `::map` over `load_data` goes through frontmatter and the document
  schema. A member-level undeclared check does not exist today and is a
  follow-up, not part of this spec.
- **(R7) Memo placement.** One options-level shared slot per command,
  single-flight per entry. A per-`ResolutionContext` memo would be per
  evaluation surface and miss the reuse R7 exists for.
- **(R8) Read-set placement.** The same options-level pattern, because
  contexts built directly with a default sink would drop reads.
- **(2026-11-06) File failures.** Every read-side expression function fills
  `reason`, `construct`, `reference`, and `failure` (R5), keeping the
  `dm.expression.evaluation_failure` code. The earlier text assumed these
  functions already did. Filling them for `load_data` alone would leave two
  behaviors for one failure.
- **(2026-11-06) One memo and read set for the whole command.** The reference
  check, added by 2026-11-04-composition-errors, evaluates expressions before
  composition. Sharing the slot across passes keeps "one `md compose` reads a
  file once" true, and keeps a file that fails in the check in the read set.
- **(2026-11-06) Commands end reuse.** The memo empties after any command or
  lifecycle step, because the check runs no shell and would otherwise serve a
  regenerated file's old contents to composition. The shared slot was chosen
  knowing this: reads repeat only where a command could have changed the data.
- **(2026-11-06) Names and limits.** The `dm.data.*` spellings, the violation
  cap of 20, and the `read_set` field name are confirmed as proposed.
- **(2026-11-06) Read once.** I/O is not cheap, so a file is read and parsed once per
  command in every cache access mode, and validated once per contract. The only re-read
  follows a command, which may have changed the file.
- **Sequencing.** Implementation starts after both declared prerequisites,
  composition errors and dotted property paths, are complete (see
  [Dependencies](#dependencies)). This remains an unimplemented spec; it
  does not assert that implementation has already begun.
- **Diagnostics.** Typed `dm.data.*` codes, structured fields, a new
  `data_location`, a cap on listed violations, and a tracing span per call
  (R5). Scripts and DMLS need the data position separately from the call
  site, and one failure with a capped list is readable without hiding
  problems.
- **Format detection.** From the path's or URL path's extension,
  case-insensitive, through biscuit-file's existing `detect_from_extension`;
  no `Content-Type`, no sniffing, no override (R3). One extension table,
  and the same rule for files and URLs.
- **(O4) `Name@this` inside `as`.** Rejected with a diagnostic (R1). An
  expression argument has no schema document for "this" to name.
- **TSV quoting.** CSV-style quoting with a tab delimiter (R4). One reader
  with one set of rules for both formats.
- **YAML locator hardening.** Deferred to a follow-up spec. A YAML data file
  containing an alias or a multi-line flow collection reports a JSON Pointer
  without line/column (R5). Both cases are uncommon in data files, the
  fallback is already defined, and the shared simplified-schema change
  (about 1.5 days for both) would grow an already-large spec.
- **Inline review (2026-11-06): consistency corrections.** Malformed input
  need not have a pointer, missing values locate their existing parent,
  CSV cell positions use raw field starts, and null file arguments remain
  null in static typing. The read set has a stable field name, ordering,
  and failure-path coverage. The existing shared conversion defect must be
  fixed rather than merely filed, following the repository's rule to fix
  discovered defects during implementation. Scanner regression tests use
  bounded fixtures; the completed broad spike is not rerun in ordinary tests.

## Open questions

No design questions remain open.

## Follow-ups

Implementation must file these as dated active specs under the existing
pre-authorization for other specs. They are not asserted to exist yet:

- **General expression-level recovery.** A construct such as
  `try(expr, fallback)` that absorbs an error.
- **Headerless CSV/TSV and a format override.** A possible options argument
  to `load_data` for a headerless file, a different delimiter, or an
  explicit format when the extension is missing or wrong.
- **Member-level undeclared-property check.** Report `h.nmae` when `h` has a
  known object type, for frontmatter values and typed `load_data` results
  alike.
- **YAML locator hardening.** Let `PositionMap` locate values below an
  alias (about half a day) and flow collections that span lines (about one
  day, which also gives frontmatter positions for such values), removing the
  pointer-only case from R5.

Cross-spec impact:

- **2026-11-05-dynamic-schema-types:**
  - it writes the `as` argument unquoted
    (`load_data("claudine/docs/os.yaml", string[])`) and must adopt the
    quoted form, `"string[]"` (pre-authorization (c));
  - it owns DMLS invalidation from the read set (its spikes S2 and S3); this
    spec provides the library read set only.
- **2026-11-04-composition-errors:** this spec adds the optional
  `data_location` field to `Diagnostic` and the output schemas, adds a read-set
  field to the compose data envelope, fills the file-reference fields on
  expression failures, and documents the `dm.data.*` codes on the data-contract
  page (pre-authorization (d)).
