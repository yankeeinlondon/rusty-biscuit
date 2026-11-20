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
created: 2026-11-20
status: draft-spec
area: darkmatter
reviewed: false
clarified: false
implemented: false
packages:
    - darkmatter
    - darkmatter-lsp
    - simplified-schema
related:
    - 2026-11-04-loading-data
    - 2026-11-05-tabular-input-options
    - 2026-11-05-dynamic-schema-types
depends-on:
    - 2026-11-04-loading-data
---

# Schema-Typed `load_data`

Let `load_data(file)` be called without a type. The type of the loaded data
is **inferred from the file's contents**, carried through the surrounding
expression, and checked against the frontmatter property's declared
`$schema` type. A document states its types once, in `$schema`, instead of
repeating them as a quoted `as` string inside each call.

This spec also adds **object spread** (`{ ...item, color: "blue" }`) to the
expression language, which the motivating example needs and which does not
exist today.

## Motivation

`load_data` requires `as`, a quoted type definition, on every call. When the
call fills a frontmatter property, the document already declares that
property's type in `$schema`, so the author writes the type twice, in two
syntaxes, and the two can drift. The common case is also verbose:

```yaml
apps: "{{ load_data('../../../terminal-apps.yaml', '{sequence: {name: string, url: string, os: string[]}[]}') }}"
```

The type checker can already follow a value through `map`, `filter`,
ternaries, and `||` (`expression_type` in
[call_check.rs](../../lib/src/markdown/compose/expression/call_check.rs)). The only missing
input is the type of the loaded data itself. If that type comes from the
file, every property built from loaded data can be checked against its
declared type, at design time in DMLS and at compose time.

## Example

```yaml
$schema:
    direct: "{ name: string(required), age: number }[]"
    additive: "{ name: string(required), age: number, color: string }[]"
    conditional:
        - "{ name: string(required), age: number }[]"
        - "literal(foobar)"
direct: "{{ load_data('data.yaml') }}"
additive: "{{ map(load_data('data.yaml'), i -> { ...i, color: 'blue' }) }}"
conditional: "{{ file_exists('data.yaml') ? load_data('data.yaml') : 'foobar' }}"
```

```yaml
# data.yaml
- name: Ada
  age: 36
- name: Grace
  age: 85
```

All three properties pass, at design time and at compose time. If `data.yaml`
gave Grace's age as `"old"`, `direct` and `additive` would fail. The error for
`direct` would point at `data.yaml` line 4. The error for `additive` would
point at the property, because the value passed through a transformation.

## Current behavior

Verified against the code on 2026-11-20:

- **`as` is required.** The catalog signature is
  `load_data(file: file | null, as: type-definition(container), options?: …)`
  ([function catalog](../../schemas/partials/functions.yaml)). A one-argument
  call fails with "no signature takes 1 argument".
- **The checker gets `load_data`'s type only from `as`.** `load_data_type` in
  `call_check.rs` parses the literal `as` text. Without it, the call's type
  falls back to the catalog return type, `unknown | null`.
- **Data must be a container.** The `as` contract must denote an array, tuple,
  object, or dictionary, so a file holding a single scalar cannot satisfy any
  contract.
- **No property-level type check.** Nothing compares the type the checker
  infers for a frontmatter property's expression with that property's
  `$schema` type. Mismatches are found only when compose validates the
  evaluated frontmatter.
- **`is_subtype` is too narrow to do that comparison.** `simplified_schema::is_subtype`
  ([subtyping](../../../simplified-schema/docs/topics/language/subtyping.md))
  compares arrays only by identity, has no structural rule for inline objects,
  and ignores presence (`required`) and conversion. It was built for approval
  rules, not for checking values.
- **There is no spread syntax.** The expression parser's object literal
  (`parse_object_literal` in
  [parser.rs](../../../simplified-schema/lib/src/expression/parser.rs)) accepts
  only `key: value` entries and the `{a}` shorthand; `...` is not a token.

## Requirements

### R1 — `as` becomes optional

The catalog signature becomes
`load_data(file: file | null, as?: type-definition(container) | null, options?: …)`.

- With `as`, `as` is the contract and the call's type is the type `as`
  denotes, as today. R2 and R8 change what the data looks like before the
  contract is applied.
- Without `as` (or with `as: null`, so `options` can be passed positionally),
  the data is not checked by the call. The call's type is the type
  **inferred** from the data (R3), or `container` when the data cannot be read
  at check time (R5).

### R2 — loaded data is always a container

A file whose top-level value is a scalar loads as a one-element array:
`5` → `[5]`, `"macOS"` → `["macOS"]`. A file whose top-level value is `null`,
including a zero-byte YAML file (which loads as `null` today), loads as `[]`.
This applies with or without `as`.

`null` passed as the `file` argument still returns `null` without reading
anything; this rule is about the contents of a file that was read.

The data-contract page and the `load_data` catalog entry document this.

### R3 — the inferred type is exact

The inferred type of loaded data records the data exactly, with no
generalizing:

| Data | Inferred type |
|---|---|
| a string, number, or boolean | that literal (`"macOS"` → `literal(macOS)`) |
| `null` | `null` |
| an array | a tuple of each element's inferred type |
| an object | an inline object with each key present and typed exactly |

Because nothing is generalized, an inferred `literal(macOS)` fits
`enum(macOS, Linux, Windows)`, and it also fits `string`.

CSV and TSV cells are text, so they infer as string literals. The one
exception to exact typing is a CSV or TSV file read with `header: false`,
which is typed `unknown[][]` (R8). The fit relation
(R4) applies text-to-number and text-to-boolean conversion the same way
validation does, so `"1000"` still fits `number`.

Each inferred literal remembers where it came from: the data file and the
JSON Pointer to that value. R6 uses this.

**Display.** Hover and diagnostics never print a large inferred type in full.
Past a threshold chosen in planning, a tuple is shown as an array of the union
of its elements, and a long union of literals is shown by its base type and a
count (for example `string (42 distinct values)`).

### R4 — a property's inferred type must fit its declared type

Add a **fit** relation, "every value of type `a` is accepted by declared type
`b`", in simplified-schema beside `is_subtype`. It differs from `is_subtype`
in the ways that checking a value needs:

- arrays are covariant: `A[]` fits `B[]` when `A` fits `B`;
- a tuple fits `B[]` when every position fits `B`;
- an inline object fits a declared object when each declared field the value
  has fits that field's type, each `required` field is present, and any extra
  fields are allowed wherever validation allows them;
- a union fits when every arm fits; a value fits a union when one arm accepts
  it;
- conversion is applied as in
  [conversion rules](../../../simplified-schema/docs/topics/conversion.md):
  a text literal fits `number` when it converts;
- constraints (`min`, `pattern`, …) are checked against literal types, since
  their values are known, and ignored for non-literal types;
- `unknown` and `container` fit only `unknown` and container-compatible
  declarations; they never cause a diagnostic by themselves.

Using it, check every frontmatter property whose value is a single `{{ … }}`
expression and whose name `$schema` declares: compute `expression_type` for
the expression and report a mismatch when it does not fit.

### R5 — when data is read for type inference

- **Compose** reads the data as it does today; the inferred type is available
  to every later check.
- **DMLS** reads local data files off the request path, under the same rules
  as the schema-slot worker
  ([slot_jobs.rs](../../dmls/src/slot_jobs.rs)): configured read permissions
  and in-process approvals decide each read, no prompt is raised, and a change
  to a file that was read marks the result stale.
- **Remote URLs** are not read at design time unless the read is already
  approved and cached. Their type is `container`, which produces no
  diagnostic.
- **A local file that does not exist** is a design-time **warning** on the
  call, and the call's type is `container`. At compose time it remains an
  error, unless the call is inside `try(…)`, as today.

### R6 — diagnostics name the data location

When the value that fails the fit check is a literal inferred from a data file
(the property's value passed through unchanged, as in `direct`), the
diagnostic carries the existing `data_location` fields (file, JSON Pointer,
line, column) in addition to the property's location. When the value passed
through a transformation, the diagnostic names the property only.

The compose-time check against `$schema` stays authoritative: the data file
may change between design time and compose time.

### R7 — object spread

Add `...expr` as an object-literal entry:

```text
{ ...base, color: "blue" }
{ ...defaults, ...overrides }
```

- Entries apply left to right; a later key replaces an earlier one, whether
  it came from a spread or was written.
- Spreading `null` contributes nothing.
- Spreading anything other than an object or `null` is a type error, reported
  by the passive check when the type is known and at evaluation otherwise.
- Two **written** entries with the same key remain a parse error, as today.
  A written key may replace a spread key.
- `expression_type` merges object shapes in the same order, so
  `{ ...i, color: "blue" }` over `{name: string, age: number}` has type
  `{name: string, age: number, color: literal(blue)}`.

The AST change is in simplified-schema (`Expr::ObjectLiteral` and
`SpannedExprKind::ObjectLiteral` gain a spread entry). About 18 files in
darkmatter, darkmatter-lsp, simplified-schema, and claudine match on
`ObjectLiteral` and must handle it. Each one is updated, not given a wildcard
arm.

Array spread (`[...a, b]`) is out of scope.

### R8 — CSV and TSV row shapes and header keys

The `header` option's type widens from `boolean` to `boolean | string[]`; it
still defaults to `true`. Without `as`, `header` alone decides each row's
shape:

| `header` | First row of the file | Each row | Rows from a file `First Name,Age` / `Ada,36` |
|---|---|---|---|
| `true` (default) | the header | an object keyed by the camelCased header | `{firstName: "Ada", age: "36"}` |
| `[name, …]` | data | an object keyed by the given names | with `["label", "years"]`: `{label: "First Name", years: "Age"}`, `{label: "Ada", years: "36"}` |
| `false` | data | a list of cells, by position | `["First Name", "Age"]`, `["Ada", "36"]` |

```yaml
rows: "{{ load_data('export.csv', null, { header: ['name', 'age'] }) }}"
```

**A list of names means the file has no header row.** Every record,
including the first, is data, and the names become the keys:

- names are used exactly as written; they are not camelCased;
- each name must be non-empty and unique, or the call fails as a bad
  argument, before the file is read;
- the number of names must equal the number of columns, or the call fails
  with `dm.data.malformed`, naming both counts, on the first record's line;
- rows are typed exactly, per R3, and an `as` contract names the given keys.

Detecting whether a file's first row is a header (for example, by comparing
its cell types with the second row's) is out of scope.

`header` applies only to CSV and TSV. On YAML, TOML, JSON, JSON5, or JSONL it
stays an error, as `delimiter` and `header` are today.

With `header: false` the data's inferred type is the wide `unknown[][]`, not
the exact type of R3: a headerless file has no named columns to check, so its
rows are not checked value by value. An `as` contract still checks them, and
the existing rule that `header: false` needs a list-row contract applies only
when `as` is given.

**With `header: true`, header keys are camelCased** with the same conversion as the `camel_case`
expression function, so keys are readable in expressions
(`row.firstName`). This applies with or without `as`: an
`as` contract names the camelCased keys. Existing contracts keyed by a
header that is already camelCase (such as `name` or `stars`) are unaffected.

Two headers that become the same key after conversion (`First Name` and
`first_name`) are `dm.data.malformed`, naming both original headers, on the
header's source line. This replaces the existing duplicate-header rule in
object-row mode.

## Resolved decisions

Decided by the author on 2026-11-20:

- An empty or `null` data file loads as `[]` (R2).
- Without `as`, CSV/TSV rows are camelCase-keyed objects with a header row and
  lists of cells without one; a headerless file's type is `unknown[][]` (R8).
- Header keys are camelCased with or without `as`; two headers that convert
  to the same key are `dm.data.malformed` (R8).
- `header` accepts `boolean | string[]`. A list of names means the file has
  no header row; the names are used verbatim, must be unique, and must match
  the column count. Header detection is deferred (R8).
- The property check covers only properties whose whole value is one
  `{{ … }}` expression; text such as `"prefix {{ … }}"` is left to
  compose-time validation for now (R4).

## Documentation

- `darkmatter/docs/functions/load-data.md` is regenerated from the catalog
  (`just regen-expr-doc`).
- The interpolation and frontmatter-interpolation pages describe the
  property-level check and inference.
- `simplified-schema/docs/topics/expression-syntax.md` documents spread.
- The `simplified-schema` subtyping page documents the fit relation beside
  `is_subtype`.
- The `use-darkmatter` caller skill is updated where it shows `load_data`, then
  `just publish-skills`.

## Acceptance

- All three properties in [Example](#example) produce no diagnostics in DMLS
  or `md compose`. Changing Grace's age to `"old"` produces a DMLS error on
  `direct` (with `data_location` at `data.yaml` line 4) and on `additive`
  (without it), and compose fails on both.
- `load_data('x.yaml')` where `x.yaml` holds `5` composes to `[5]`; an empty
  `x.yaml` composes to `[]`.
- A CSV with header `First Name,Age` loads rows as `{firstName, age}`
  objects, and `row.firstName` resolves; with `header: false` it loads lists
  of cells. Headers `First Name` and `first_name` together are
  `dm.data.malformed`.
- The same file with `header: ['label', 'years']` loads two rows,
  `{label: "First Name", years: "Age"}` and `{label: "Ada", years: "36"}`.
  Three names for two columns is `dm.data.malformed`; `['a', 'a']` and
  `['']` fail before the file is read; `header` on a YAML file is an error.
- A property declared `enum(macOS, Linux)[]` loaded from a file containing
  `- macOS` passes; one containing `- BeOS` fails at design time.
- A missing local file is a DMLS warning and a compose error.
- Every existing `load_data` call with `as` behaves as before, except where
  R2 (scalar and empty files) or R8 (camelCased headers) apply.
- `{ ...a, k: 1 }`, `{ ...a, ...b }`, and `{ ...null }` evaluate and type as
  in R7; `{ ...5 }` is a type error; `{ k: 1, k: 2 }` is still a parse error.
