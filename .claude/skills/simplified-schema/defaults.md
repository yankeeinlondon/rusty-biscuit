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
  choosing an arm; memoized by its sorted nodes, so recursion is finite.
  Each node is `Reached { pointer, arm }`: `arm` lists the `anyOf`/`oneOf`
  members on its route from the root (one per union, a recursive re-entry
  replacing the earlier choice; a union with at most one member admitting
  non-null is no choice, since a described value exists). `$ref`, `allOf`,
  selectors and array items keep the arm.
  `DefaultPlan::for_identity` memoizes per `SchemaIdentity` (capacity 64,
  cleared by `reset_process_caches`). `Prepared::new` takes its plan from
  it (keyed by the shared cache's canonical digest), so a large schema
  prepared once per file compiles its defaults once per process;
  `Prepared::compiled_through` (private caches, the envelopes) compiles
  its own.
- Key selection follows JSON Schema, and every schema selecting one key is
  that key's one property definition (`Entry`: child location + one
  `Declared`): its `properties` entries, every matching `patternProperties`
  schema, and each node's `additionalProperties` when *that* node neither
  names nor matches the key (`Scope::catches`, per node, so another `allOf`
  member's catch-all still describes a literally named key). A literal
  name's definitions merge at compile time (`Property`), so conflicts there
  are compile-time. Any other key's selection is known only from the key:
  `Fill::select` gathers the matching `KeyPattern`/`CatchAll` entries (and,
  where a location names the key, that location's `Property`) across every
  location describing the object, and `Fill::fill` applies the ordinary
  rules to the merged selection: one default (different values are the one
  run-time `ConflictingDefaults`, per key, only when needed), `required`
  from any location, the default checked (below) before any child, then
  `visit(&children, …)` over all selected child locations at once, so a
  parent default from one selector and a child default from another land in
  one pass. Only literal names, names in `required`, and keys present in the value are
  prepared (a missing `required` name takes its selecting pattern's or catch-all's default);
  a selector never creates a key. Patterns compile once into `KeyPattern`
  (`coercion::TextPattern`, `fancy_regex`, the engine that already handles
  the lowered literal-exclusion lookahead). `if`/`then`/`else`,
  `dependentSchemas`, `not`, and `unevaluated*` are deliberately not followed.
  Tests: `tests/l1/pattern_defaults.rs`. `check_declarations` remembers
  accepted documents by the XXH64 of their JSON text in written key order
  (`written_digest`, streamed, no sorted copy; `CHECKED`, capacity 256),
  because hosts lower the same baseline repeatedly and the pattern subtrees
  more than doubled its locations. The key is not a canonical identity:
  every lowering pays it, a registered trigger payload of half a megabyte
  included, so do not switch it to `xx_hash_json`. It also compiles without arms
  (`Compiler::new(schema, false)`: one location per node set instead of one
  per arm, about 13x fewer on the lifecycle baseline), since arms only
  matter to `Fill::check`; a refusal or an expansion loop
  (`Compiler::loops`) is re-decided by the full compile, so its verdict
  and error are the arm-aware ones. `Compiler` interns pointers and arms as
  `u32` ids (`Reached` is `Copy`); keep its hot maps on `IdMap`/`IdSet`,
  because SipHash over those ids dominated the debug-build test time.
- `prepare_defaults(&plan, Cow<Value>, &NestedTemplates)` fills a missing or
  `null` property from its default, required or not. A `null` default fills
  only a missing key of a property some schema at that location lists as
  `required`. Supplied non-null values (`0`, `false`, `""`, `[]`, `{}`) win.
  A missing/`null` parent without a default is left alone; a parent filled
  from its default is checked, then descended into. Values at or below a
  nested template are pending and skipped.
- A used default is checked strictly (structural validator, root `$defs` in
  scope, memoized per `Definition`) before its children are filled, against
  **every** definition in the key's selection (`Entry::definitions`, all
  selectors, `allOf` members, `$ref` targets and joined item schemas, not
  only those repeating `default`), judged by `Fill::accepts` from each
  declaring definition's arm (`Declared::arms`) → `InvalidDefault { path,
  pointer, message }`. Within that context arm: every definition whose arm
  lies inside it must accept; a definition on a sibling member of a union
  the context chooses is skipped (`conflicts`: the one-arm policy); and each
  union the context does not choose is a group, needing one member that
  accepts, judged recursively with that member chosen (so a nested union is
  judged inside its member). A member with no definition for the key
  accepts only if `admits_key` says it can hold an object with that key:
  its `type`/`const`/`enum` allow an object, it is not `false`, and an
  `additionalProperties: false` does not deny the key (followed through
  local `$ref` and `allOf`; a nested `anyOf`/`oneOf` needs one admitting
  member). It is a static reading, never a validation of the unfinished
  parent. Independent groups are each
  required. Each definition is validated at most once per check. So a
  default object is never repaired by another selector's required child,
  even when that child is required by every alternative of an independent
  union, and a text default is never converted to fit one. Unused defaults
  are never judged, so an optional property can expose a broken default
  only when a value leaves it unset. Tests:
  `pattern_defaults::{complete_default_checks, independent_union_checks}`,
  `required_defaults::unions`.
- Errors: `ConflictingDefaults { pointer, property, declarations }` at
  compile time (object arms, raw schemas), or from `Fill::fill` per key for
  an unnamed key's disagreeing selectors (above); a property-level union's
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
