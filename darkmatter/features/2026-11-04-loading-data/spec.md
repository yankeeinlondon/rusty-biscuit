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
file (CSV, TSV, JSON, JSONL, or YAML) and returns its contents as a typed
container value. The caller states the expected shape with `as`, a
`type-definition` that must denote a **container** type. Data that does not
satisfy that contract is an error, never a partially typed value.

This is a **draft**. The open questions at the end must be settled before
the spec is finalized.

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
Supported: {{ as_csv(load_data("claudine/docs/os.yaml", string[])) }}
```

renders `Supported: macOS, Linux, Windows`. With a mismatching contract:

```markdown
{{ load_data("claudine/docs/os.yaml", number[]) }}
```

composition fails, naming the file, the location inside the data (`/0`), the
expected type (`number`), and the value found (`"macOS"`).

## Requirements

### R1. Signature and catalog entry

```text
load_data(file: file | null, as: type-definition) -> any | null   (fallible)
```

- `file` is a `FileReference` resolved in the calling document's context,
  exactly like every other file-taking function. Null passes through to null,
  matching the catalog's existing null convention.
- The return value is the parsed data, coerced and validated against `as`.
- The function is added to `schemas/partials/functions.yaml` (category
  `Filesystem`) so DMLS hover, completion, and the generated function docs
  pick it up from the catalog; nothing is hand-maintained in a second place.

### R2. `as` must denote a container

`as` is a `type-definition` value. Its **denoted type** (see the
`type-definition` docs) must be a container:

| Denoted type | Example `as` | Accepted |
|---|---|---|
| array | `string[]`, `{ name: string, port: number }[]` | yes |
| tuple | `[string, number]` (tuple form) | yes |
| object | `{ name: string, tags: string[] }` | yes |
| dictionary (pattern dictionary) | dictionary of `number` | yes |
| scalar | `string`, `number`, `enum(a, b)` | **no** |
| union | `string[] \| object` | open question Q3 |

A non-container `as` is a static error. Because `as` is usually a literal,
the existing passive call check (the first pipeline stage) must reject it
**before anything runs**, with the span of the `as` argument. A dynamically
computed `as` is checked when the call executes.

### R3. Formats

The format is chosen from the file extension. Content sniffing is not used.

| Extension | Parsed as |
|---|---|
| `.yaml`, `.yml` | one YAML document, any top-level shape |
| `.json` | one JSON value, any top-level shape |
| `.jsonl`, `.ndjson` | one JSON value per non-empty line → array |
| `.csv` | RFC 4180 rows → array (see R4) |
| `.tsv` | tab-separated rows → array (see R4) |

An unknown extension, or a multi-document YAML stream, is an error.

**Format parsing belongs in `biscuit-file`**, not Darkmatter. `DataFormat`
today covers TOML, YAML, JSON, JSON5, Markdown, PDF, and Text. Add the
tabular and line-delimited formats there so other packages reuse them, and
record any new crate (for example `csv`) in `docs/dependencies.md`.

### R4. Tabular shape follows `as`

CSV and TSV carry no types. The first row is a header. The shape of each row
is chosen from the element type of `as`:

- element is an **object** → each row becomes `{header: cell, …}`;
- element is a **tuple or array** → each row becomes a list of cells and the
  header row is skipped.

Cells are text. Turning `"8080"` into the number `8080` is the normal job of
the single coercion engine (`markdown::coercion`) when it applies `as`; no
CSV-specific number guessing is added.

### R5. Contract enforcement

After parsing, the value is converted and validated against `as` by the same
coercion and validation path that frontmatter uses. Any failure is an error
(never a warning, never a partial value). The error must report:

- the resolved file (portable path or URL);
- the location inside the data as a JSON Pointer, plus line/column for YAML
  and JSON and row/column for CSV/TSV/JSONL where the parser exposes them;
- the expected type and the value found.

Errors use the existing typed file-reference diagnostics for resolution and
read failures (`FileReferenceDiagnostic`), so a missing file reads the same as
it does for `try_frontmatter` or `::file`.

### R6. Remote reads and caching

- `load_data` is a read-side remote function. An HTTP(S) argument is fetched
  only under `--allow host=HOST`, through `biscuit_file::FetchPolicy`, exactly
  like `try_frontmatter` (add it to the read-side list in
  `compose/remote.rs`). Deny-all stays the default.
- Every file `load_data` reads is recorded in the document's referenced files
  so composition caches and DMLS invalidate when the data changes.

### R7. Documentation

- A function page generated from the catalog, plus a short topic section
  showing each format and the R4 tabular rule.
- **Rewrite the `type-definition` documentation.** The current material in
  `docs/topics/schemas/definition.md` (Semantic Meta-Types) explains carrier
  vs. denoted types but never shows a newcomer which spellings are accepted,
  which constraints apply (stated far below, under Semantic Arrays), or how to
  tell a container from a scalar. `load_data` makes `type-definition` a
  user-facing argument type, so the page must lead with what a reader can
  write, give one compact example per form, and add a "container types"
  section that R2 can link to.

## Testing

- L1 unit tests per format, per R4 shape, and per R5 failure class, including
  an empty file, a header-only CSV, and a quoted CSV cell containing the
  delimiter.
- A passive check test: a scalar `as` is rejected by the call check, and no
  file is read.
- An end-to-end `md compose` test through the normal invocation path for one
  success and one contract failure, asserting the rendered diagnostic.
- A remote test (wiremock) for both denied and allowed hosts.

## Open questions

- **Q1.** Should TOML and JSON5 be supported too? `biscuit-file` already parses
  both, so the cost is low, but they were not in the original request.
- **Q2.** Should there be a non-failing `try_load_data` returning
  `{ok, value, error}`, matching `try_frontmatter`?
- **Q3.** May `as` be a union of containers (`string[] | { … }`)? Allowing it
  makes R4's tabular rule ambiguous when arms disagree on row shape.
- **Q4.** May `as` be a named type import (`os-list@./schemas/os.yaml`)? This
  makes the contract reusable but makes the passive call check resolve a schema
  file.
- **Q5.** Is a file-size limit needed (one large file read per expression in a
  long `::map`)?
- **Q6.** Should a headerless CSV be expressible (for example, a tuple `as`
  meaning "no header row")?
