---
kind: fix
created: 2026-11-14
status: human-in-the-loop
area: simplified-schema
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-14-edit-action
depends-on:
    - 2026-11-14-edit-action
next_spec: 2026-11-14-lifecycle-mutation-root
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
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-14
review_iterations: 0
needs_rulings: false
---

# A missing key and `null` are the same value

## Why this exists

A YAML mapping can omit a key or contain that key with a `null` value. Those
are different representations; YAML itself does not make them equal.
Darkmatter's expression reads already return `null` for a missing document
property. This fix makes SimplifiedSchema's property values follow that same
rule, while preserving key presence for the `required` contract.

SimplifiedSchema currently distinguishes omission from explicit `null` in
[validation](../../docs/topics/validation.md#required-properties-with-a-default)
and [conversion](../../docs/topics/conversion.md#conversion-rules):

| Declaration | `{}` today | `{ "n": null }` today |
|---|---|---|
| `n: number(default(3))` | passes; default is metadata only | passes; no replacement |
| `n: number(required; default(3))` | passes with `n = 3` reported | rejected |
| `n: number(non-null)` | passes | rejected |

The review of `2026-11-14-edit-action` exposed a related mismatch:
Darkmatter's lifecycle parser refuses `no_error: null`, while the action's
`no_error: boolean` schema accepts it. That feature also added `non-null`
markers to make `edit`'s schema match its parser. The author has ruled that
these surfaces should instead share the value rules below.

## Author rulings (2026-11-14)

- **A missing key and an explicit `null` are the same value, `null`,** for
  reading, defaults, and type checks.
- **`required` is the one place they differ, and only for the required
  contract:** it asks whether the key was written. `n: null` meets that
  contract; a missing `n` does not, unless a default supplies it. The value
  must then satisfy the property's type.
- **A default replaces a `null` value,** from a missing key or an explicit
  `null`, on optional and required properties alike. A default also meets
  `required` for a missing key.
- **`null` fails a type that does not admit it.** `n: number(required)` with
  `n: null` meets `required` but fails the number type check. Ordinary
  optional properties retain their existing acceptance of `null`.
- **One implementation and consistent guidance:** the `simplified-schema`
  crate owns these rules; its hosts, docs, and skills use the same contract.

The following are validation outcomes after default preparation. They do
not mean that strict validation returns an edited value; see the validation
contract below.

| Declaration | Key missing | `n: null` |
|---|---|---|
| `n: number` | valid; reads as `null` | valid; stays `null` |
| `n: number(default(3))` | valid with default `3` | valid with default `3` |
| `n: number(required)` | error: required key missing | error: `null` is not a number |
| `n: number(required; default(3))` | valid with default `3` | valid with default `3` |
| `n: null(required)` | error: required key missing | valid |
| `n: unknown(required)` | error: required key missing | valid |
| `n: ["number(required)", null]` | error: required key missing | valid |

**Reader's note — intentional compatibility changes** (the table's null-admitting rows and this note confirmed by the author, 2026-11-14). Today a type that
admits `null` cancels `required`, including a marker inherited from a named
type or another union arm. The author's presence-only definition of
`required` replaces that rule: null admission controls the value, never
whether the key must exist. Removing `non-null` is also an intentional
language change. It is not a spelling alias for `required`: replacing an
optional `string(non-null)` with `string(required)` would additionally
forbid omission. The migration must preserve intended optionality.


### Applying defaults and validating are separate steps (author ruling, 2026-11-14)

1. **Applying defaults** fills a missing or `null` value with the declared
   default. It is the only step that sets a value.
2. **Validation** judges the resulting value against the schema. It never
   sets a value. (It may know that a default exists, as ruling 3 requires
   for `required`, but it does not fill it.)

Describe the two steps separately in docs and API docs; where the library
today does both inside one call (soft validation converts values and
inserts defaults), keep the call's behavior but document it as both steps.

**One rule for both schema forms.** Applying defaults treats a raw JSON
Schema `default` exactly like a SimplifiedSchema `default(...)`: an optional
property in a raw schema with `"default": 3` that a document leaves unset
(or sets to `null`) becomes `3`, as `n: number(default(3))` does. Raw
schemas' assertions and `required` arrays are otherwise unchanged. A
schema's meaning never depends on which form it was written in.

### Defaults and unions (author ruling, 2026-11-14)

1. **A default is a runtime fill-in for a property, never part of matching.**
   `retries: number(default(3))` makes the effective type `number` (without
   the default it would be `number | null`). It does not make the property
   required; when `retries` has no value at runtime, the value is `3`.
2. **The same in a union as anywhere else.** Whether a property is declared
   alone or in one arm of a union, "no value means the default" applies,
   whichever arm matched.
3. **Matching uses the actual values.** An optional property with no value
   fits any arm, so arm selection only matters where values are present or
   an arm declares a required or excluded (`null`) property. Exclusive arms
   are written by excluding the other arm's keys:

   ```yaml
   $schema:
     - timeout: number(default(3))
       retries: null
     - retries: number
       timeout: null
   ```

4. **At most one default per property across a union's arms.** Differing
   defaults for the same property on different arms are a compile-time
   schema error, never resolved by order. This extends the existing
   property-level rule ("Differing `default(...)` values on multiple arms is
   a compile-time error", `docs/topics/language/unions.md`) to root-union
   (object) arms. A default on one arm is fine:

   ```yaml
   $schema:
     - timeout: number(default(3))   # design time: number | string
     - timeout: string               # runtime: 3 when timeout is null
   ```

Consequences for the plan: union arm selection no longer depends on
defaults, so the "already valid as written wins" preference and
`SchemaError::AmbiguousDefaults` (`docs/topics/validation.md`, "Unions are
judged per arm") should become unreachable; confirm and remove them rather
than leave dead code. Scan the shipped schemas' root unions for properties
with defaults on more than one arm, and for an arm (for example a closed one)
that would reject a default another arm supplies.

## Scope and boundaries

This fix covers declared object properties, including nested objects, list
items that are objects, resolved named types, root and property unions,
triggers, and YAML schema-document envelopes. Darkmatter frontmatter,
composition, lifecycle actions, DMLS, and Claudine's schema-backed readers
must consume the library's rule. Include whole-YAML validation as well as
Markdown frontmatter.

**Declared is not written (author ruling, 2026-11-14).** A property
declared in the schema is never an undeclared name, whether or not the
author wrote it. Reading a declared property that has no value and no
default gives `null`, which interpolates as empty text, with no warning;
this outcome is today's and does not change. But no key is added to the
data to achieve it: composition records which names the schema declares
separately, and stops writing `null` into its working frontmatter for
absent optional properties (`materialize_optional_document_bindings`,
documented in `darkmatter/docs/inline/schema-validation.md`, "Optional
parameter bindings"). Everything downstream sees only what was actually
written: a transcluded child, `--frontmatter` output, lifecycle actions, and
`required` checks.

Why: once `required` distinguishes a missing key from a written `null`
(ruling 2), a synthetic `null` would satisfy a child's `required` contract
although the caller passed nothing:

```text
parent.md   $schema: { title: string }              (title optional, not written)
  ::file child.md
child.md    $schema: { title: unknown(required) }
```

The child must report that `title` is required and was not provided.

Likewise, with no applicable default, `{}` stays `{}` and `{ n: null }`
stays `{ n: null }`: validation never adds keys to the data it is given.
Unknown keys remain unknown even when their value is `null`; closed objects
still reject them. Do not globally delete null-valued keys. Source
positions, authored key order, and the original input remain available to
diagnostics and presence checks.

Function arguments and tuple positions have an existing positional contract:
optional means that an argument or trailing item may be omitted; explicit
`null` needs a null-admitting type. Preserve that contract and argument counts;
this fix concerns mapping keys, not synthesizing positional arguments or
list items. Nested object properties inside those values follow this fix.
Ordinary scalar coercion, file-reference rules, subtype comparisons, and
which lifecycle events execute remain unchanged.

## Validation and default preparation

Use the existing shared default engine, not a second walk in a host. In the
`simplified-schema` package,
[`prepare_defaults`](../../lib/src/defaults.rs) builds the candidate a
validator judges, and
[`Prepared`](../../lib/src/prepared.rs) exposes strict and soft validation.
Expand that engine from missing required keys to missing or null-valued
properties with defaults. All entry points must prepare defaults even when
the unmodified input already passes: the current early-success checks would
otherwise skip every newly active optional default.

- **Strict validation** judges the completed candidate without scalar
  coercion, returns the original borrowed value, and reports applied defaults
  in [`ValidationSuccess::defaults`](../../lib/src/outcome.rs), the
  `simplified-schema` package's successful-validation report. Thus both `{}` and `{ n: null }` pass
  `n: number(default(3))`, but their returned values remain as written and
  their default report names `n = 3`.
- **Soft validation** returns a copy containing replacements and insertions,
  as well as scalar conversions. Default `3` replaces `null` even though the
  optional property would otherwise accept it. A non-null supplied value
  takes precedence: `0`, `false`, `""`, `[]`, and `{}` are not null values.
  Defaults apply before conversion and again where conversion makes a nested
  default reachable. A second soft validation of the completed result makes
  no further changes.
- **No input or source mutation.** Validation, editor diagnostics, schema
  loading, and trigger matching stay passive. Composition may write the
  successful completed copy into its output document, never back into the
  authored file. Failed conversion retains the existing diagnostic-copy
  behavior and never commits partially completed data.
- **A child needs its parent.** A missing or null-valued object without its
  own default is not created to reach a child's default. Inside an existing
  object, including an object in a list, child defaults apply. An object
  supplied by a default must already satisfy its complete required-child
  contract; defaults do not repair an invalid default object.
  After checking that default, complete reachable child defaults in the
  same preparation so the result is stable on a second validation. Recursive
  type references remain legal, but defaults that repeatedly create another
  defaulted object must fail with a recursive-default error instead of
  expanding indefinitely.
- **Default validity and reporting.** Preserve strict checking of a used
  default against the complete property definition, without conversion or
  file access. Preserve invalid-default and recursive-reference errors (the
  union `AmbiguousDefaults` error is retired; see "Defaults and unions"), stable declaration/index order, and one report per path. A default
  needed by an optional property can now expose an invalid schema that was
  previously accepted because its default was never used.
- **Null defaults terminate.** An optional `default(null)` is a no-op on an
  absent or null value and creates no key or applied-default report. For a
  required null-admitting property, it inserts a missing key with `null` and
  reports that insertion; it does nothing to an already present `null`.
  A required number with a null default fails default validation. Do not
  repeatedly try to replace a null default with itself.
- **Pending values stay pending.** Neither a deferred expression nor a
  template descendant is a null value to replace. Unresolved schema slots
  still prevent validation until explicitly resolved. Default preparation
  never evaluates expressions or grants file or network access.

Within an object, presence is checked after default preparation and then
values and other constraints are judged. Existing object key-count
constraints see the completed candidate, so adding optional defaults may
change their outcome. Do not change the key-count definition to hide that
effect; audit schemas that combine defaults with key counts.

Unions follow "Defaults and unions" in the author rulings: arms are matched
on the actual values, a property's one default fills it whichever arm
matched, and differing defaults for one property across arms are a
compile-time error. Preserve discriminant checks and root sibling
assertions.

## Presence, lowering, and JSON Schema

In the `simplified-schema` package,
[`PropertyDef::is_required`](../../lib/src/simplified/types.rs) is the shared
presence calculation used by validation, lowering, queries, trigger matching,
completion, and composition. Update that calculation and every consumer so
that written, inherited, or hoisted `required` survives a null-admitting arm.
Resolve named types before making that decision; unresolved template types
retain their pending state.

Trigger conditions retain their restricted grammar: defaults, imports, and
generated values remain forbidden in `match:`. The changed presence rule
applies to those conditions, while default preparation applies to the
trigger's selected schema payload, not to trigger discovery itself.

Lower ordinary required properties into the parent's `required` array.
Their value schemas admit `null` only when the declared type does. Ordinary
optional properties retain their null-admitting wrapper when needed.
Defaults remain `default` annotations in exported Draft 2020-12 JSON Schema.

**Reader's note — export cannot perform default application.** JSON Schema's
`required` checks key presence, but its `default` keyword does not fill or
replace values. A third-party validator rejects `{}` against exported
`n: number(required; default(3))`; this library accepts it using its shared
default preparation. A third-party validator also leaves an optional
`n: null` unchanged. Sharing the meaning of `required` does not make the two
validation procedures equivalent. Document this limitation instead of
relaxing exported `required` or inventing a standard defaulting behavior.

Preserve the existing timing and ownership roles of `eager`, `generated`,
and phase projections. They decide when a host demands a property or who
supplies it; they must not restore the old rule that a null-admitting type
cancels `required`. In particular, a launch-time projection that temporarily
allows an absent required property must retain the declared value type for
any value supplied. Test these phases separately from the ordinary table.

Raw JSON Schema assertions must not acquire SimplifiedSchema's optional-null
wrappers or have their authored `required` arrays rewritten. Default application uses raw JSON Schema `default` keywords exactly as it
uses SimplifiedSchema defaults (see "Applying defaults and validating are
separate steps").

## Removing `non-null` and migrating schemas

Remove the constraint from grammar, the parsed type model, resolution,
serialization, export, validation, error/help catalogs, declaration checks,
and completion. Old constraint spellings, including
`$constraints: { non-null: true }`, become unsupported with a useful schema
error; do not silently ignore them. Negative tests may retain the old spelling.

**`not-empty` replaces `non-null` (author ruling, 2026-11-14).** `not-empty`
means "a written value must have content". It never rejects `null`: on an
optional property `null` (or a missing key) is always "not set". Today it
applies to strings only; this fix extends it:

| Type | `not-empty` rejects | Still accepted |
|---|---|---|
| `string` | `""` and whitespace-only text (unchanged) | `null`, text with content |
| `yaml`, `json` | empty or whitespace-only text, e.g. an interpolation expected to produce YAML that produced nothing | an explicit `null`, a non-empty definition |
| lists (`x[]`) | `[]` (the same check as `min(1)`; compile both to one rule) | `null`, a non-empty list |
| objects and mappings (`{ … }`, `schema(mapping)`) | `{}` | `null`, a non-empty mapping |
| `number`, `boolean`, `null`, `unknown` | schema error: "empty" has no meaning for these types | — |

**Migration:** replace each `non-null` with `not-empty` where the type has an
"empty"; drop it where the type has none. Keep any `default(...)`, and use
`required` only where a key truly must be written.

| Before | After | Newly rejected |
|---|---|---|
| `description: string(non-null)` | `string(not-empty)` | `description: ""` |
| `session_start: action[](non-null)` | `action[](not-empty)` | `session_start: []` |
| `types: schema(mapping; non-null)` | `schema(mapping; not-empty)` | `types: {}` |
| `prewarm: boolean(non-null; default(true))` | `boolean(default(true))` | nothing |

An override that inherits when it is left out still inherits when it is
`null`, because `not-empty` never rejects `null`.

For example, schema `description`, `categories`, optional `types`, optional
trigger payloads, and repository configuration overrides must not become
mandatory. Update schema-envelope readers as well as their admission schemas:
a null optional `types` table is absent, and a tagged schema with
`$schema: null` and a valid `types` mapping behaves like one with only `types`.
A document with neither a usable export nor a types table remains invalid.
An export-required `kind: yaml-schema` still rejects a null `$schema`.
The schema value `n: null`, which declares a null-typed property inside a
payload, remains a valid type declaration; it is not an absent payload.

Audit configuration readers and layering before adding defaults. An override
that inherits when absent must also inherit when null; inventing `false`,
`[]`, or `{}` as a default could disable an inherited setting. Record migration
choices and changed completed outputs in the implementation log.

## Lifecycle integration and dependency

The prerequisite `2026-11-14-edit-action` is already implemented. This fix
owns the still-pending review ruling **“`edit` follows coercion”** recorded
in that feature's implementation log. Implement the schema and lifecycle
parser changes together; neither half may pass review or be committed alone.
The following spec, `2026-11-14-lifecycle-mutation-root`, changes write
boundaries and remains outside this fix.

- Wherever an action accepts `no_error`, declare
  `boolean(default(false))`, including supported sibling/bundle flags and
  long forms. Preserve the existing exception: `set` has no `no_error` flag.
  `edit` declares `create_new: boolean(default(false))` and `gui: boolean`.
  Missing or null `gui` means no editor preference.
- Use the shared conversion engine for flag values and `edit` operands:
  boolean-like text such as `"yes"` becomes `true`; native numbers do not
  become booleans. `edit: 5` names file `5`. Strict validation still rejects
  values that require coercion; soft validation and execution agree after
  binding.
- Keep blank paths, unknown keys, invalid argument counts, and missing/null
  mandatory `file` operands refused. Removing the outer `non-null` marker
  does not make `edit: null` an executable action: it supplies no target.
  Keep the shared passive action-shape/argument-count check used by schema
  validation and the lifecycle parser. Do not mark `edit` required in the
  shared verb dictionary, which would require it beside unrelated actions.
- Inserted `no_error: false` must not cause duplicate sibling/inner flag
  errors, fabricate an action, change verb counts, or give `set` a flag.
  Preserve authored flag origin separately from completed values when
  checking duplicates; a null flag counts as omitted. Explicit conflicting
  placements remain invalid. Include `{ edit: { file: notes.md } }` as a
  regression case because both enclosing and inner schemas may add a flag.
- Update the generator in the `darkmatter` package,
  [`schema_projection`](../../../darkmatter/lib/src/lifecycle/schema_projection.rs),
  which creates lifecycle schemas from the typed catalogs, then regenerate
  its regions. Its current special alternatives for optional operands and
  `no_error` depend on required markers and key counts; hand-editing generated
  YAML alone would leave those contracts inconsistent.

## Inventory and documentation

Treat counts from the draft as search hints, not acceptance criteria. Actual
constraint uses include SimplifiedSchema's three envelope/trigger definitions,
Claudine's and Darkmatter's configuration schemas, and Darkmatter's lifecycle partial. Many
matches in Claudine trigger schemas and Darkmatter's benchmark fixtures are
only prose meaning “a value other than null”; that meaning remains valid.
Do not rewrite frozen benchmark inputs just to eliminate the word.

Audit optional defaults in `darkmatter/schemas`, `claudine/schemas`, and
SimplifiedSchema's fixtures, including null defaults in `fm.yaml` and defaults
nested in style options. Include all required null-admitting declarations and
all host readers that consume completed values. Also inspect raw JSON Schema
inputs according to the author ruling.

Update the affected topic pages: SimplifiedSchema validation, conversion,
[types](../../docs/topics/language/types.md), unions, schema documents, and
custom vocabulary; Darkmatter schema and lifecycle docs; Claudine
configuration and lifecycle docs. Update overview README claims only where
behavior is summarized there. Docs must explain the final rules without
linking to this spec.

Update repo skills under `.claude/skills/` and canonical caller skills under
each area's `docs/use-skills/`, then use `just publish-skills` in the affected
areas. Never edit installed user-scope copies directly. Remove live guidance
that null cancels `required` or that optional defaults are metadata only;
retain the accurate statement that defaults are annotations in exported
JSON Schema and the positional-call distinction described above.

Lowering and shipped schema changes require updating each affected owner's
`PREPARATION_REVISION` for code-only preparation changes and regenerating all
three shipped artifacts with one root `just shipped-schemas` run. Ensure
freshness tests declare any newly relevant source inputs under the repository's
test-input rules. Cached/prepared and uncached schemas must produce the same
result. Do not hand-edit generated artifacts.

## Verification

The implementation is complete when the following behavior is covered and
package checks pass. This is the implementation's verification plan; this
inline review changes only this specification.

- Exercise every outcome in the author-ruling table through strict and soft
  library validation. Check borrowed versus owned results, default reports,
  unchanged inputs, and repeat-soft-validation stability. Include false,
  zero, empty values, optional and required null defaults, and unused versus
  newly used invalid defaults.
- Cover existing/missing/null parents, arrays of objects, recursive named
  types, imported and hoisted required markers, null-admitting unions, and
  pending templates. Cover discriminated and overlapping unions according
  to "Defaults and unions": an input that already fits an arm still gets a
  property's default; differing defaults across arms fail at compile time;
  a default on one arm applies whichever arm matched. Preserve invalid-default and recursive-reference errors.
- Test exported JSON Schema with a plain validator separately from the
  library's preparation path; test raw JSON Schema inputs according to the
  author ruling. Verify trigger matching and launch/completion/generated
  presence behavior without running lifecycle effects.
- Cover schema-envelope admission and parsing with omitted/null metadata,
  `types`-only documents, null optional carriers, missing mandatory exports,
  and a null type declaration inside a valid payload. Cover Claudine's
  omitted/null override inheritance and key-count-constrained schemas.
- Exercise `md schema validate`, composition, whole-YAML validation, and DMLS
  with representative fixtures. Extend the existing
  [lifecycle parity tests](../../../darkmatter/lib/tests/l1/lifecycle_edit_schema_parity.rs)
  for omitted/null/text/native flags and mandatory operands, in both schema
  and parser paths. Include generated actions, bundles, duplicate flag
  placement, `set`, and defaults visible to composition's subsequent reads.
  Use passive checks or fake services; no editor or browser takes focus.
- `not-empty` per type: for `string`, `yaml`/`json`, a list, and a mapping,
  the empty value is rejected while `null`, a missing key, and a non-empty
  value are accepted; `not-empty` on `number`, `boolean`, `null`, or
  `unknown` is a schema error; on a list it behaves exactly like `min(1)`.
  Cover a `yaml` property fed by an interpolation that produces nothing.
- Declared is not written, through `md compose` with a parent that declares
  `title` as optional and a transcluded child that declares
  `title: unknown(required)`:

  | Parent writes | Parent renders `{{ title }}` | Child result |
  |---|---|---|
  | nothing | empty, no warning | error: `title` is required and was not provided |
  | `title: null` | empty, no warning | accepted (`null` was written) |
  | `title: "Hi"` | `Hi` | accepted |

  Repeat with the child declaring `title: string(required)`: the first row
  reports the missing `title` (not "`null` is not a string"). Check that
  `--frontmatter` output and lifecycle actions in the parent see no
  `title` key in the first row.
- Search active implementation, schemas, help, and docs for stale constraint
  support and old contract claims. Historical specifications, rejection
  tests, and descriptive uses of “non-null” are legitimate matches.
- Run `just test` and `just lint` in `simplified-schema/`, `darkmatter/`, and
  `claudine/`, using the repository's nextest recipes. Run relevant existing
  integration tests only where host write-back or execution behavior needs
  them; use the Test Toolkit and keep windows from taking focus. Changes
  must remain portable across macOS, Linux, native Windows, and WSL2.

No new performance spike or benchmark matrix is required. Keep default work
in the shared preparation/conversion machinery, reuse compiled validators,
and avoid copying the input when no default or conversion applies. If the
chosen union policy reveals a concrete performance concern, record it for
the author before expanding measurement scope.

## Open Questions

None.
