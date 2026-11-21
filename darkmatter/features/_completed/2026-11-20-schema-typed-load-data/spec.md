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
status: finalized-spec
area: darkmatter
reviewed: true
reviewed_by: "codex/gpt-6.2-sol"
reviewed_on: "2026-11-20"
review_iterations: 2
completed: true
needs_rulings: false
required_rulings:
clarified: false
implemented: true
implemented_by: "claude/opus"
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

Allow `load_data(file)` without a quoted type argument. A document can declare
the destination type once in `$schema`, with the loaded data flowing through
ordinary expressions such as `map`, `filter`, and conditionals. The Darkmatter
language server (DMLS) can inspect approved data to catch mistakes while the
author edits. Compose judges the values from its own run.

This feature also adds object spread, as in `{ ...item, color: 'blue' }`, so
an author can extend a row without repeating its fields.

## Example

```yaml
$schema:
    direct: "{ name: string(required), age: number }[]"
    additive: "{ name: string(required), age: number, color: string }[]"
    conditional:
        - "{ name: string(required), age: number }[]"
        - "literal(foobar)"
direct: "{{ load_data('data.yaml') }}"
additive: "{{ map(load_data('data.yaml'), i => { ...i, color: 'blue' }) }}"
conditional: "{{ file_exists('data.yaml') ? load_data('data.yaml') : 'foobar' }}"
```

Save the following as `data.yaml`.

```yaml
- name: Ada
  age: 36
- name: Grace
  age: 85
```

All three properties pass in compose and in DMLS once the editor has an
approved, current result. If Grace's age is changed to `"old"`, all three
properties fail with the file present. The direct error points at the
`direct` expression and the value on line 4 of `data.yaml`. The `additive`
error points at the expression that built the new row. The conditional
uses the fallback only when the file is absent.

## Existing contracts

- The Darkmatter [function catalog](../../schemas/partials/functions.yaml)
  requires a container type in the second argument. The Darkmatter
  [`expression_type` checker](../../lib/src/markdown/compose/expression/call_check.rs),
  which supplies types for passive expression checks and editor features,
  already follows callbacks and branches. Its special handling for
  `load_data` uses a literal contract and accounts for a nullable file
  argument. That checker stays independent of file access.
- The [Loading Data guide](../../docs/topics/loading-data.md) defines
  contract conversion, input parsing, positional and object rows, and
  readable error locations. This feature extends that behavior by making
  the contract optional, wrapping scalar roots, and changing named
  tabular keys as described below.
- [Darkmatter schema validation](../../docs/topics/schemas/definition.md)
  judges the effective schema, which can include built-in rules, matching
  triggers, and imported or resolved types. It already applies defaults
  and converts values. These rules also apply to expression results.
- The [Simplified Schema expression parser](../../../simplified-schema/lib/src/expression/parser.rs)
  reads object fields and field shorthand. Adding spread affects this
  parser and the consumers that evaluate or inspect the resulting expressions.

## Requirements

### Optional contracts and return values

The catalog signature becomes
`load_data(file: file | null, as?: type-definition(container) | null, options?: …)`.
Calling `load_data(file)` and `load_data(file, null)` has the same meaning.
Use `load_data(file, null, options)` to set options without a contract.

- A supplied contract keeps its existing role. It selects an allowed
  tabular row shape and validates and converts the parsed value before the
  call returns. Preserve literal-contract typing, including the null-file
  case, and the existing treatment of computed or imported contracts.
- Omitted or null `as` skips call-level schema validation and conversion.
  Strict parsing, option checks, error handling, and read tracking still
  apply. A CSV cell like `"1000"` is a string inside the surrounding
  expression. A destination declared as `number` can convert it when
  frontmatter is validated. Supply `as` when a callback needs converted
  values before that stage.
- After parsing, keep array and object roots, wrap any non-null scalar
  root in a one-element array, and turn a null root into `[]`. Apply this
  before a supplied contract is judged, so a YAML file containing `5`
  can satisfy `number[]` and can fail a tuple that needs two elements. An
  empty YAML file follows its parsed null value. Empty or malformed input
  still follows each format's parser. For example, an empty JSON file
  is malformed and JSON containing `null` produces `[]`.
- A bound null file argument still short-circuits to null, before contract
  imports, file resolution, or reads. Preserve the passive check for
  provably invalid authored arguments, even in an unselected branch.
- Preserve the existing resolution context, permissions, read set, and
  failure order. A missing file is still a recoverable absence for
  `try(…)`. Malformed data, invalid options, and permission failures
  keep their current failure behavior. Ordinary compose reads use the
  existing loader and memo, including their refresh after an executed
  shell step.

### Checking expression results against the schema

Check top-level frontmatter properties declared by the effective schema whose
entire authored value is one `{{ … }}` expression. Embedded text such as
`"prefix {{ … }}"` and nested expression sites keep their existing
compose-time checks. Use the effective declaration for that document and
its original resolution context. Defer a finding if a schema slot, a root
union arm, or another part of the applicable declaration cannot yet be
resolved well enough to prove the problem. For example, an error under
one possible root union arm does not establish an error under the union.

Use the exact observed values when available. Preserve literal leaves,
array positions, object keys, and nulls. For member lookup and hover, this
behaves like a tuple for an array and a known set of fields for an object.
The implementation can retain the parsed value and derive shapes as needed.
Keep the declared type visible in hover. Summarize large inferred shapes by
field, element, or alternative count and show only a bounded preview. Build
that preview without first formatting the entire value.

Compose uses the actual expression result and its normal frontmatter
validation stage. DMLS checks a copy when an approved background evaluation
has a concrete result. Use the schema engine's existing conversion,
defaulting, presence, and constraint rules. For example, a missing field
with a default can pass, `"1000"` can convert to a number, and an unexpected
field still fails in a closed object. If a rule needs a file check or
other input the editor cannot verify under its read policy, defer that
finding. Treat strings returned by evaluation as data, even if they contain
template syntax.

> **Reader's note.** The draft proposed a new type relation to predict what
> validation would accept. Checking observed values through the existing
> validator keeps conversion, defaults, and union selection consistent with
> compose. An inferred shape helps the editor explain a value. A firm schema
> error requires evidence the existing validation rules can judge. The
> editor reports no schema error from types alone; see
> [Resolved decisions](#resolved-decisions).

For an unavailable uncontracted load, the type is a container of unknown
shape, plus null if the file argument may be null. A literal null file has
type null. An unknown result is provisional. It supplies no property-level
verdict and must also avoid new missing-member errors based solely on
unobserved fields. For `header: false` without `as`, publish
`unknown[][]`; DMLS skips cell-by-cell schema checking through that
uncontracted read. Compose still validates every realized cell covered by
the destination schema.

### Editor reads and refresh

Keep passive call checking available without reading inputs. Schedule new
background reads for whole property expressions that use uncontracted
`load_data`. Reuse the read authorization, function admission,
work limits, cancellation, and current-result installation used for editor
schema slots. Those rules already permit pure transforms and approved reads,
and prevent effectful functions from running. Use the document's resolution
context and frontmatter values known at editing time. Defer when the
expression needs runtime-only values, unresolved dependencies, or an
unsupported operation. Evaluation follows ordinary branch selection,
short-circuiting, and `try` recovery. A missing file in an unselected branch
does not produce a warning.

- Allow local content and existence reads only under the configured
  file-read policy or an applicable in-process approval. The worker raises
  no prompt. An unapproved or unavailable read leaves the result pending.
- Remote reads for this check may use a current cached response only when
  the URL is allowed for the request, with authorization checked before
  cache access. A miss, expired entry, or denied URL leaves the result
  pending. The editor check never fetches or revalidates over the network.
  Expiry also makes an installed result based on that response stale.
- Feed completion, hover, and diagnostics from the installed snapshot.
  Track reads and missing candidates, and refresh on relevant disk changes,
  unsaved changes to dependent buffers, configuration or approval changes,
  and schema invalidation. Discard results from outdated jobs. While a
  result is stale, withhold its findings and inferred members. Follow the
  existing [DMLS schema slot worker](../../docs/topics/schemas/schema-slots.md#in-the-editor)
  for path watches, missing-file creation, and clients without file watches.
- Warn at an executed, unrecovered missing local read that the editor can
  prove under its read policy. The same missing file remains a compose
  error. Preserve the distinction between a missing file and a file the
  worker is unable to inspect.

Reuse the parsed input across eligible properties in the same observation.
Build source positions only for reported problems and stop when the
background work allowance is exhausted. Compose needs neither an extra
input read nor a separate inference pass to judge the completed property.

### Error locations

Keep the authored Markdown expression as the primary location. For a
direct load, including one wrapped in parentheses or selected unchanged by
a conditional, add a data-file location to a property schema error when
the failing value can be traced to the data. A result built by a callback
or object spread gets the expression location only. Call-level malformed
input and explicit-contract failures keep their existing data-file details
regardless of how the result would have been used.

Reuse Darkmatter's [`DataLocation` record](../../lib/src/markdown/compose/expression/diagnostics.rs)
and the loader's [position lookup](../../lib/src/markdown/compose/expression/data/locate.rs)
so the CLI and editor point to the same source. Store source identity and
the mapping needed to locate a failure. Locate the original input, including
the source root for a wrapped scalar and the original cell for a renamed
header. Preserve the existing rules for absent members, unknown line or
column, and optional byte offsets. Never guess a position. Extend the
diagnostic data contract to allow `data_location` on these property schema
failures as well as the existing data errors. Use existing schema error
codes for the property failure so a call's own contract violation remains
distinguishable from a destination schema violation.

### Object spread

```text
{ ...base, color: 'blue' }
{ ...defaults, ...overrides }
```

- Evaluate entries once each in source order. For each key the last entry
  providing it wins. Two explicitly written entries with the same key
  remain a parse error, including shorthand and duplicates separated by
  a spread.
- Spread accepts an object or null. Null contributes no fields. A known
  non-object, non-null operand is a passive type error. An operand whose
  type cannot establish the result is checked when evaluated. Report the
  error at the spread operand.
- Type inference uses the same order. Known later fields replace earlier
  fields. A field that might be supplied by a later spread can widen the
  earlier field's type. A spread with unknown keys leaves an open result
  until more is known, while a subsequent explicit field sets the type
  of that field.
- The Simplified Schema [expression tree](../../../simplified-schema/lib/src/expression/)
  keeps written fields and spread operands distinct, with source spans for
  editor ranges. Update every consumer that handles object literals,
  including dependency and permission scans, passive diagnostics, editing
  support, and evaluation across Simplified Schema, Darkmatter, DMLS, and
  Claudine. Scans must visit spread operands, including ones whose keys
  are later overwritten. Preserve the existing interpolation closing
  rule and the literal-only grammar used for catalog examples.

Array spread is out of scope.

### CSV and TSV rows and keys

`header` accepts `boolean | string[]`; omitted or null uses `true`.
With no `as` contract, it determines the shape.

| `header` | First input record | Row shape |
|---|---|---|
| `true` (default) | Header | Object keyed by camelCase names |
| `['label', 'years']` | Data | Object with exactly the supplied column names |
| `false` | Data | Array of cells in column order |

With `as`, retain the existing rule that the contract selects object or
positional rows and all applicable contract arms agree:

- `header: true` supports either shape. Positional rows skip the header
  record without inspecting its names. Object rows use camelCase keys with
  or without `as`.
- `header: false` requires a positional-row contract. A list of names
  requires an object-row contract. Report an incompatible contract as
  `dm.data.row_shape_conflict` before reading the data, using the existing
  passive check when the format, header mode, and contract are known.
- Use explicit names verbatim after normal argument binding. An empty
  name, duplicate name, or empty name list is `dm.data.invalid_options`
  on the options argument and fails before any data read. For a nonempty
  file, the name count must equal the first data record's width. A
  mismatch is `dm.data.malformed` at the first data record and names both
  counts. With no data records, return `[]` and apply any supplied
  contract. Keep the normal width check on every later record.

For example, `load_data('export.csv', null, { header: ['label', 'years'] })`
reads a first record of `First Name,Age` as
`{label: "First Name", years: "Age"}`. The second record `Ada,36` reads as
`{label: "Ada", years: "36"}`. The same file under `header: true` yields
one row, `{firstName: "Ada", age: "36"}`.

Use the same case conversion as Darkmatter's
[`camel_case` expression function](../../lib/src/markdown/compose/expression/functions/strings.rs)
to make header-derived keys predictable in expressions. A normalized empty
key or a collision such as `First Name` and `first_name` is
`dm.data.malformed` on the header record. Name the offending original
header or both colliding headers. Preserve the original row and cell
positions and the read cache's distinction between different row shapes
or explicit name lists. A tabular `header` value other than null stays
invalid on non-tabular formats. Format, delimiter, and strict parsing
rules continue to apply. Header detection from data is out of scope.

> **Reader's note.** Header normalization intentionally changes existing
> object-row keys, including with `as`. Update contracts, member accesses,
> and any consumers of emitted objects that use the old spellings. Callers
> who need verbatim names can supply the names explicitly when their input
> has no header record. Document this before-and-after migration with a
> header such as `First Name`. Review shipped callers and examples for
> affected keys. Positional rows still use cell order.

## Resolved decisions

The author decided on 2026-11-20 that a null or empty data value loads as
`[]`; a tabular file without a contract gets camelCase-keyed objects from
header rows and `unknown[][]` from headerless rows; object-row header keys
are normalized even when `as` is supplied; and explicit names are verbatim,
unique, and match the column count, with every record treated as data.
The property check covers whole-value expressions. Header detection is
deferred.

The author ruled on 2026-11-20 that the editor reports new property schema
errors only from concrete results it can evaluate, judged by the same
validation rules as compose. When it cannot evaluate a property (runtime-only
values, a function it cannot run, an unapproved read, or a missing file), it
reports no schema verdict, and compose remains the check. For example, a
missing file gets the missing-file warning but no error for a property
declared as a non-container type; that mismatch appears once the file exists
or at compose. Proving mismatches from types alone is out of scope for this
feature.

## Documentation

Update [Loading Data](../../docs/topics/loading-data.md), the generated
[function page](../../docs/functions/load-data.md) via `just regen-expr-doc`,
and the Darkmatter expression, frontmatter interpolation, and DMLS schema
guides. Document optional contracts, when conversion happens, a header-key
migration example, and what the editor can check. Update
[Data Contracts](../../docs/cli/data-contracts.md) and its output schema for
the additional use of `data_location`. Document object spread in
[Simplified Schema expression syntax](../../../simplified-schema/docs/topics/expression-syntax.md).
Update the Darkmatter overview and caller skill where they introduce
`load_data`, then publish the caller skill with `just publish-skills`.

## Acceptance

- The worked example passes for valid data. With `age: "old"` on line 4,
  all three expressions have property errors in DMLS when the file is
  present and approved. `direct` and the selected direct conditional arm
  also carry the original data location. Verify each property through
  compose in isolation so failure order cannot hide another case.
- A property declared `enum(macOS, Linux)[]` accepts `- macOS` and rejects
  `- BeOS` when observed. A numeric string and a missing field with a
  declared default agree with compose. An unexpected member in a closed
  object fails. Verify effective schema selection with an imported type
  and a trigger declaration.
- Cover omitted and explicit null `as`, literal and computed null file
  arguments, scalar wrapping, empty YAML, JSON `null`, and malformed
  empty JSON. Verify that a supplied container contract is applied after
  normalization and that equivalent calls share the existing read memo.
- Cover all three header modes with and without compatible contracts,
  including retained positional rows for `header: true`. Check camelCase
  collisions and empty normalized names, invalid explicit names before
  reads, width mismatches and their source lines, and `header: null` on
  a non-tabular format. Verify that explicit names preserve cell positions
  and that different lists do not share a result of the wrong shape.
- An unguarded, missing local file gets an editor warning when provable
  and fails compose. Test the guarded and recovered cases and verify
  that creating the missing file refreshes the editor. Cover an approved
  unsaved dependent buffer, a stale or superseded job, a disallowed
  local read, and a remote cache miss without a network request.
- Spread tests cover ordering, multiple spreads, null, wrong-type
  operands, explicit duplicates, nested interpolation, and member typing
  after a spread with unknown keys. Verify that reads in spread operands
  are found and subject to the same permission checks.
- Run the passive shipped-artifact corpus and the end-to-end cases through
  normal Darkmatter and DMLS entry points. Check Claudine's expression
  consumers after the shared parser change.
