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
status: draft-spec
area: darkmatter
reviewed: false
clarified: false
implemented: false
review_iterations: 0
related:
    - 2026-10-28-extract-simplified-schema
next_spec: 2026-11-05-dynamic-schema-types
---

# Loading Data

Add a `load_data(file, as)` expression function that reads a structured data
file (YAML, JSON, JSONL, CSV, TSV, TOML, or JSON5) and returns its contents
as a typed container value. The caller states the expected shape with `as`, a
quoted type definition that must denote a **container** type. Data that does
not satisfy that contract is an error, never a partially typed value.

This is a **draft**. The decisions taken in the 2026-11-05 clarification
session are recorded under [Resolved decisions](#resolved-decisions); the
items under [Open questions](#open-questions) must still be settled before the
spec is finalized.

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

composition fails, naming the file, the location inside the data (`/0`, line
1, column 3), the expected type (`number`), and the value found (`"macOS"`).

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
  `"8080"` to the number `8080`) and validates the result.
- **JSON Pointer** — a path into a data value, such as `/3/port` (fourth
  element, its `port` field).

## Requirements

### R1. Signature and catalog entry

**Requirement**

```text
load_data(file: file | null, as: type-definition(container)) -> any | null   (fallible)
```

- `file` is a `FileReference` resolved in the calling document's context,
  exactly like every other file-taking function. Null passes through to null,
  matching the catalog's existing null convention.
- `as` is a **quoted string** containing a type definition:
  `load_data("claudine/docs/os.yaml", "string[]")`. Named type imports are
  allowed: `"os-list@./schemas/os.yaml"` and `"Name@darkmatter:<stem>"`.
- The return value is the parsed data, coerced and validated against `as`.
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
is not a container.

| `as` | Container? | Notes |
|---|---|---|
| `string[] \| null` | yes | empty YAML file → null |
| `unknown[]` | yes | CSV/TSV cannot pick a row shape (see O1) |
| `unknown` | no | |
| `object` | yes | not valid for CSV/TSV (R4; diagnostic wording O1) |
| `{a: string}[] \| {b: number}[]` | yes | CSV/TSV: object rows |
| `string[] \| {a: string}` | yes | not valid for CSV/TSV (object arm) |
| `literal([…])` | — | not expressible: `literal` takes a scalar |

A non-container `as` is a static error. When `as` is a literal, the passive
call check rejects it **before anything runs**, with the span of the `as`
argument. A computed `as` is judged when the call executes.

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

**Acceptance criteria**

- Each "yes" row in both tables passes the passive check; each "no" row is
  rejected by it with the `as` argument's span, and no file is read.
- `"[string, number]"` is rejected (a union of scalars).
- A computed scalar `as` (for example `load_data(f, t)` with `t = "string"`)
  fails at execution with the same diagnostic.

### R3. Formats and ownership

**Requirement**

The format is chosen from the file extension. Content sniffing is not used.

| Extension | Parsed as |
|---|---|
| `.yaml`, `.yml` | one YAML document, any top-level shape |
| `.json` | one JSON value, any top-level shape |
| `.jsonl`, `.ndjson` | one JSON value per non-empty line → array |
| `.csv` | RFC 4180 rows → array (see R4) |
| `.tsv` | tab-separated rows → array (see R4) |
| `.toml` | one TOML document (always a table) |
| `.json5` | one JSON5 value, any top-level shape |

An unknown extension, or a multi-document YAML stream, is an error.

**Design notes — who owns what**

`DataFormat` in `biscuit-file/lib/src/format.rs` today only names formats; it
parses nothing. TOML and JSON5 parsing exist as separate wrapper types
(`Toml`, `Json5`). The work divides as follows:

- **biscuit-file** — add `DataFormat` variants for JSONL, CSV, and TSV and a
  `DataFormat::from_extension`; add reusable readers for delimited text
  (CSV/TSV, each row carrying its source line from the `csv` crate's
  positions) and for JSON lines. New dependency: `csv` (already in the
  workspace lock through biscuit-tui's CLI, `csv = "1.3"`); record it in
  `docs/dependencies.md`.
- **darkmatter** — routes by format:
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

**Acceptance criteria**

- Each extension in the table loads a fixture into the expected value.
- `.ndjson` and `.jsonl` behave identically; blank lines are skipped.
- An unknown extension and a two-document YAML stream each fail with a
  diagnostic naming the file.
- A JSON number outside the exact-`f64` range is rejected the same way it is
  for frontmatter.
- `docs/dependencies.md` lists `csv` for biscuit-file.

### R4. Tabular shape follows `as`

**Requirement**

CSV and TSV carry no types. The first row is a header. The row shape is
chosen from the element type of `as`. Every non-null arm of `as` must be an
array or tuple, and all their element types must agree:

- every element is an **object** → each row becomes `{header: cell, …}`;
- every element is a **tuple or array** → each row becomes a list of cells
  and the header row is skipped.

Arms that disagree (one object element, one list element), or an arm that is
not an array/tuple, are a static error naming the conflicting arms.

Cells are text. Turning `"8080"` into the number `8080` is the normal job of
the coercion engine (`simplified_schema::coercion`) when it applies `as`; no
CSV-specific number guessing is added.

**Design notes**

- The row-shape rule is CSV/TSV-specific and lives in darkmatter, not in the
  simplified-schema container predicate.
- When both `file` and `as` are literals, the passive check knows the
  extension and can apply the row-shape rule before anything runs; otherwise
  it is applied at execution.

**Acceptance criteria**

- `"{ name: string, port: number }[]"` over a CSV with header `name,port`
  yields objects with numeric `port`.
- `"tuple([string, number])[]"` over the same file yields lists and skips the
  header.
- `"{a: string}[] | tuple([string])[]"` over a CSV is a static error naming
  both arms.
- A quoted cell containing the delimiter, and a header-only file (→ `[]`),
  parse correctly.

### R5. Contract enforcement and error reporting

**Requirement**

After parsing, the value is converted and validated against `as` by the same
coercion and validation path that frontmatter uses. Any failure is an error
(never a warning, never a partial value). The error reports:

- the resolved file (portable path or URL);
- the location inside the data as a JSON Pointer **and** a line/column, for
  every format (YAML, JSON, JSONL, CSV, TSV, TOML, JSON5);
- the expected type and the value found.

Resolution and read failures use the existing typed file-reference
diagnostics (`FileReferenceDiagnostic`), so a missing file reads the same as
it does for `try_frontmatter` or `::file`.

**Design notes**

- Line/column for YAML comes from simplified-schema's `PositionMap`.
  `PositionMap::project` (`simplified-schema/lib/src/positions.rs`) must
  accept a top-level list root; today the spec's own `os.yaml` example gets
  no line/column.
- JSONL and CSV/TSV map naturally: the record's source line plus the column
  within it.
- Multi-line JSON, TOML, and JSON5 are **risk items**, not decided designs;
  see [Risks](#risks).

**Acceptance criteria**

- The `number[]` example fails with the file, `/0`, line 1 column 3,
  `number`, and `"macOS"`.
- For each format, one contract-failure fixture reports the correct line and
  column of the offending value.
- A missing file produces the same diagnostic class as `try_frontmatter` on a
  missing file.

### R6. Remote reads

**Requirement**

`load_data` is a read-side remote function. An HTTP(S) argument is fetched
only under `--allow host=HOST`, through `biscuit_file::FetchPolicy`, exactly
like `try_frontmatter`. Deny-all stays the default.

**Design notes**

Add `load_data` to the read-side list in `compose/remote.rs`. The format is
still taken from the URL path's extension.

**Acceptance criteria**

- A URL with no `--allow` fails without a network request.
- A URL with `--allow host=HOST` is fetched once and loaded.

### R7. Repeated reads within one run

**Requirement**

Within one compose run, the same resolved file (canonical path or URL) with
the same `as` text is read, parsed, and validated **once**; later calls reuse
the result. When the request turns caching off, every call reads again. There
is no file-size limit.

Why: a `{{ load_data(…) }}` inside a `::map` body is evaluated once per item
(only the collection source itself is evaluated once), so without a memo a
100-item map reads the same file 100 times. File size needs no separate cap
because the existing request-wide iteration limits already bound the work
done with the data.

**Design notes**

Follow the existing per-run single-flight pattern
(`ResolutionContext.provider_queries` in
`darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs`). The memo
lives for one run only; nothing persists.

**Acceptance criteria**

- A `::map` over three items whose body calls `load_data` on one file reads
  that file once (observed through a work counter or read-count hook).
- The same file with a different `as` text is validated separately.
- With caching off, the same `::map` reads the file three times.

### R8. Read set

**Requirement**

Each compose run keeps a **read set**: the files that expressions read.
`load_data` and the existing reading expression functions — at minimum
`frontmatter` and `try_frontmatter`, plus the other read-side functions —
report every file they read into it. DMLS uses the read set to know when a
document's expression results are stale.

Why: no expression function records what it reads today.
`referenced_files` exists only in schema resolution
(`darkmatter/lib/src/markdown/schemas/mod.rs`), the composed-result cache is
per-run (there is nothing persistent to invalidate), and DMLS does not track
expression reads at all. So editing `os.yaml` leaves DMLS showing results
computed from the old content.

The exact scope is not decided; see O5.

**Acceptance criteria**

- After a run that calls `load_data("a.yaml", …)` and
  `frontmatter("b.md")`, the read set contains both resolved files.
- A call that fails after resolving its file still records that file.
- In DMLS, editing a file in an open document's read set causes that
  document's expression-derived results (diagnostics, hover values) to be
  recomputed.

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

**Acceptance criteria**

- The `file_exists` pattern yields `[]` for a missing file and the data for a
  present one.
- `load_data(missing, "string[]") || []` fails composition.
- The `file_exists` pattern over a present but malformed file fails.

### R10. Documentation

**Requirement**

- A function page generated from the catalog, plus a short topic section
  showing each format, the R4 tabular rule, the R9 optional-file pattern,
  and that `as` is a quoted string.
- **Rewrite the `type-definition` documentation.** The current material in
  `docs/topics/schemas/definition.md` (Semantic Meta-Types) explains carrier
  vs. denoted types but never shows a newcomer which spellings are accepted,
  which constraints apply (stated far below, under Semantic Arrays), or how to
  tell a container from a scalar. `load_data` makes `type-definition` a
  user-facing argument type, so the page leads with what a reader can write,
  gives one compact example per form (including `tuple([…])` versus a bracket
  union), and adds a "container types" section that R2 links to.
- The read set (R8) and the per-run memo (R7) are described on the relevant
  compose topic page.

**Acceptance criteria**

- The function page renders from the catalog with at least one example per
  format family.
- The `type-definition` page has a "container types" section matching R2.
- No `docs/` page names this spec.

## Risks

These are unproven and are candidates for short spikes before planning:

- **Line/column for multi-line JSON.** The current locator is YAML-oriented.
  Pretty-printed JSON is valid YAML flow syntax, so it may work unchanged,
  but that is unverified.
- **Line/column for TOML and JSON5.** No locator exists for either. Each
  needs a span-preserving parse (or a second positional pass) that maps a
  JSON Pointer back to source.
- **Top-level list roots** in `PositionMap::project` (R5) touch shared
  simplified-schema code used by frontmatter validation.

## Testing

- **Formats (L1):** one success fixture per extension in R3, including an
  empty YAML file (→ null, accepted by `"string[] | null"`), a blank-line
  JSONL file, a header-only CSV, and a quoted CSV cell containing the
  delimiter.
- **Container rule (L1, simplified-schema):** every row of both R2 tables
  through `denotes_container` and the `type-definition(container)` format
  check, with the bracket-union case.
- **Passive check (L1):** a scalar `as` and a disagreeing CSV union are each
  rejected with the `as` span and no file read; a named import passes
  provisionally without I/O.
- **Row shape (L1):** each R4 acceptance criterion.
- **Error reporting (L1):** one contract failure per format asserting JSON
  Pointer, line, and column; a top-level-list YAML root included.
- **Memoization (L1):** the R7 `::map` read-count test, the different-`as`
  case, and the caching-off case.
- **Read set (L1 + DMLS):** the R8 criteria for `load_data`, `frontmatter`,
  and `try_frontmatter`, and one DMLS test that edits a read file and observes
  recomputed results.
- **Optional files (L1):** the three R9 criteria.
- **End to end:** `md compose` through the normal invocation path for one
  success and one contract failure, asserting the rendered diagnostic and the
  `report-json` record.
- **Remote:** wiremock tests for a denied host and an allowed host.
- **Shipped corpus:** the catalog entry passes the passive shipped-artifact
  corpus test.

## Definition of done

- Every acceptance criterion in R1–R10 has a passing test.
- `just test` and `just lint` pass in `darkmatter/`, `simplified-schema/`, and
  `biscuit-file/`.
- The catalog entry, generated function page, topic section, and rewritten
  `type-definition` page are in place; `docs/dependencies.md` records `csv`.
- The `darkmatter`, `simplified-schema`, and `biscuit-file` skills mention
  `load_data`, the `container` option, and the new `DataFormat` variants
  respectively.
- Each risk item is either resolved or reported in the implementation log
  with what was delivered instead.

## Resolved decisions

All decided on 2026-11-05.

- **How is `as` written?** As a quoted string, `"string[]"`. Unquoted type
  syntax collides with variables, indexing, object literals, and the
  `number()` function in today's expression grammar.
- **(old Q1) TOML and JSON5?** Supported, with full line/column reporting.
  biscuit-file already parses both, and consistency across formats matters
  more than the extra locator work.
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
  reading functions as well as `load_data`. Without it DMLS cannot tell when
  any expression result is stale.

## Open questions

- **O1. Scalar elements and objects over CSV/TSV.** R4 has no rule for a
  scalar or `unknown` element type over a CSV/TSV file (for example
  `"string[]"`), nor a decided diagnostic for a top-level object or
  dictionary `as`. Research suggests a static error recommending
  `"{ <string>: string }[]"` or `"tuple([…])[]"`; this is not decided.
- **O2. Headerless CSV** (old Q6). Should a file with no header row be
  expressible, for example through a tuple `as` or an option?
- **O3. The `}}` scanner hazard.** The `{{ … }}` scanner ends an expression
  at the first unnested `}}`, even inside a string
  (`darkmatter` `scan.rs`). An `as` such as `"{a: {b: string}}"` therefore
  breaks the enclosing expression unless written `"{a: {b: string} }"`.
  Should the scanner respect quotes, or is the workaround documented?
- **O4. `Name@this` inside an expression.** It is unclear what "this"
  refers to in an expression argument (the calling document's schema?), or
  whether it should be rejected.
- **O5. Read-set scope.** Which functions report into it beyond
  `frontmatter` and `try_frontmatter`; how DMLS consumes it; whether it
  appears in `--output report-json`; and whether remote URLs are included.
- **O6. Line/column feasibility** for JSON, TOML, and JSON5 (see
  [Risks](#risks)). If a locator proves impractical, is a JSON Pointer alone
  acceptable for that format?

## Follow-ups

- **General expression-level recovery.** A construct such as
  `try(expr, fallback)` that absorbs an error, designed in a separate
  follow-up spec (not yet created).
- **Cross-spec impact:** 2026-11-05-dynamic-schema-types writes the `as`
  argument unquoted (`load_data("claudine/docs/os.yaml", string[])`). It must
  adopt the quoted form, `"string[]"`.
