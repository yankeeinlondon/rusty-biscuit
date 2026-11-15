# Presence, `null`, and defaults: code map

Loaded from the `simplified-schema` skill. The user-facing record is
[validation.md](../../../simplified-schema/docs/topics/validation.md)
("Missing keys, `null`, and `required`" and "Applying defaults"); this file
says where the code is and which traps to avoid.

## The contract in one table

A missing key and a written `null` are the same value. `required` alone asks
whether the key was written; the type then judges the value.

| Declaration | Missing | `n: null` |
|---|---|---|
| `n: number` | valid | valid |
| `n: number(default(3))` | `3` applied | `3` applied |
| `n: number(required)` | `MissingRequired` at the parent | `TypeMismatch` at `/n` |
| `n: number(required; default(3))` | `3` applied | `3` applied |
| `n: null(required)` / `unknown(required)` / `[number(required), null]` | `MissingRequired` | valid |

Tests: `tests/l1/required_presence.rs` (presence, no defaults) and
`tests/l1/required_defaults.rs` (`presence_table`, `null_and_falsey`,
`parents`, `unions`, `raw_schemas`, `raw_schema_inputs`, `input_shapes`).

## Presence

- `PropertyDef::is_required` / `PropertyAtom::is_required` (`simplified/types.rs`)
  are the one presence calculation (validation, lowering, queries, trigger
  matching, completion, composition). Written, inherited, and hoisted markers
  all count; `admits_null` never cancels them. Details and the phase
  projection: [declaration-narrowing.md](declaration-narrowing.md).
- Function arguments and tuple positions are positional: optional means
  "may be omitted", a written `null` needs a null-admitting type, and default
  preparation never inserts a position.

## Default application (`defaults.rs`)

- Two steps, kept separate: `prepare_defaults` is the only code that sets a
  value; validators judge the completed candidate and set nothing.
- `DefaultPlan::compile(Arc<Value>, &ValidatorCache)` reads a lowered or raw
  schema once into object locations: a location merges every node that can
  describe it (`$ref` target, `allOf`, every `anyOf`/`oneOf` arm), never
  choosing an arm; memoized by sorted node pointers, so recursion is finite.
  `DefaultPlan::for_identity` memoizes per `SchemaIdentity` (capacity 64,
  cleared by `reset_process_caches`).
- `prepare_defaults(&plan, Cow<Value>, &NestedTemplates)` fills a missing or
  `null` property from its default, required or not. A `null` default fills
  only a missing key of a property some schema at that location lists as
  `required`. Supplied non-null values (`0`, `false`, `""`, `[]`, `{}`) win.
  A missing/`null` parent without a default is left alone; a parent filled
  from its default is checked, then descended into. Values at or below a
  nested template are pending and skipped.
- A used default is checked strictly (structural validator, root `$defs` in
  scope, memoized per declaration) against **every** definition declaring
  it → `InvalidDefault { path, pointer, message }`. Unused defaults are never
  judged, so an optional property can expose a broken default only when a
  value leaves it unset.
- Errors: `ConflictingDefaults { pointer, property, declarations }` at
  compile time (object arms, raw schemas); a property-level union's
  differing arm defaults are still a `Convert` error from hoisting in
  `convert.rs`. `RecursiveDefault { path: None }` for a `$ref` loop without
  descent (so `Prepared::new` refuses it); `RecursiveDefault { path: Some }`
  for a default that keeps recreating a defaulted parent. All reach callers
  as `ValidationFailure::Unprepared`. The runtime `AmbiguousDefaults` and the
  per-arm "already valid wins" logic are gone; do not reintroduce them.
- `lower_document` runs `defaults::check_declarations`, so
  `to_json_schema`, `to_json_schema_with_definitions`, phase lowering, and
  the facade export refuse conflicting object-arm defaults too.
- `Prepared` holds a plan (compiled in `compiled_through`). `validate_strict`
  prepares, judges, and returns the input `Cow::Borrowed` with `defaults`;
  `validate_soft` prepares, returns at once if the candidate is valid, else
  converts and prepares again (late defaults deduplicated by path). There is
  no "accepted as written, skip defaults" shortcut anywhere in the library.
- Never write a second defaulting walk in a host. Darkmatter's
  `EffectiveSchema` holds a plan (`default_plan()`); see the `darkmatter`
  skill's [schema.md](../darkmatter/schema.md).
- Known gap: a default under an empty nested key (`o.""`) is neither applied
  nor reported, because `AppliedDefault.path` cannot spell it.

## Export

Exported JSON Schema keeps `required` and writes `default` as an annotation.
A plain validator rejects `{}` for `n: number(required; default(3))` and
leaves an optional `n: null` as `null`; pinned by
`a_plain_validator_still_rejects_an_absent_defaulted_required_property`.
Never relax exported `required` to hide this.

## `not-empty`, `yaml`/`json`, and numbers

- `yaml(not-empty)` / `json(not-empty)` judge the **text**: `"{}"` and `"[]"`
  have content, and soft turns a native `{}` into `"{}"` and accepts it
  (`not_empty_values.rs::yaml_and_json_text_must_still_parse`). Lowering per
  type: [declaration-narrowing.md](declaration-narrowing.md).
- An overflowing YAML number (`1e400`) loads as the text `"1e400"` (pinned by
  `setter_values::spellings_yaml_reads_as_strings_are_never_rounded`).
  `SimplifiedSchema::try_from` reads raw JSON text through YAML, so a raw
  `"default": 1e400` loads and fails later as `InvalidDefault`; the `.json`
  file loaders refuse it at load ("out of range"). A fix that refused it in
  the admitted-YAML loader broke the pinned test and was reverted; changing
  it means re-pinning those tests.
