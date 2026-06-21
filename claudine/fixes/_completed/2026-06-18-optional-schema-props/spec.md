---
status: ready for planning and implementation
created: 2026-06-18
severity: bug
provider: all
reviewed: true
related_features:
  - darkmatter/features/_completed/2026-05-11-schemas
related_docs:
  - claudine/docs/topics/composition.md
  - darkmatter/features/_completed/2026-05-11-schemas/spec.md
  - darkmatter/docs/topics/schema-definition.md
---

# Optional Schema Properties Should Accept `null` as "Absent"

## The Problem We Are Solving

A composition document declares an optional `$schema` property whose frontmatter value resolves to `null` after Darkmatter composition (a very common pattern when a templated lookup fails). Schema validation rejects the resolved document even though the property is explicitly optional.

### Observed Incident

`claudine/prompts/review-feature.md` declares:

```yaml
$schema:
    spec: string(required)
    design: string
    iteration: number
description: "Reviews a _feature specification_ …"
dir: "$(dirname '{{spec || design}}')"
design: "{{ file_exists(dir + '/design.md') ? dir + '/design.md' : null }}"
```

When no `design.md` exists next to the spec, the `design` template evaluates to `null`. Claudine compose aborts before launch with:

```
CompositionError: schema validation failed
…
type design: null is not of type "string" at 9:1

Correct the frontmatter so it satisfies the declared $schema (or baseline schema).
```

The diagnostic is **wrong**. The schema says `design: string` — a property that is **not** `required`. By the SimplifiedSchema contract ([spec], "Optional by default"), `design` may be absent. A frontmatter slot whose value resolved to `null` is the same logical state as "absent," and validation must accept it.

[spec]: ../../../darkmatter/features/_completed/2026-05-11-schemas/spec.md

### Root Cause

`darkmatter/lib/src/markdown/schemas/simplified/convert.rs::atom_to_schema` already recognizes the "optional absent sentinel" pattern, but only for the `file` type, and only for the empty-string sentinel:

```rust
// Decision A: a non-`required` scalar `file` field treats an empty string
// as "absent" so a ternary like `spec: "{{ ... ? path : '' }}"` validates
// when the optional file is missing. …
let optional_empty_file =
    matches!(&atom.ty, TypeExpr::Primitive(SimplifiedType::File)) && !atom.is_array && !required;
…
} else if optional_empty_file {
    json!({ "anyOf": [ { "const": "" }, inner ] })
}
```

There is **no** equivalent wrap for any other optional type, and **no** wrap that admits JSON `null`. The generated JSON Schema for `design: string` is therefore the bare fragment `{"type":"string"}`, which by Draft 2020-12 semantics rejects `null` even though the property is optional. The validator is correctly applying the schema it was handed; the schema is wrong.

The same gap exists for every other typed property (`number`, `boolean`, `date`, `enum`, `url`, `email`, …), for `object` / inline objects, and for both scalar and array forms. Optional means "may be absent," and absent is what `null` says — everywhere, not just for `file("")`.

## What We Are Building

### Goal

Make SimplifiedSchema's "optional" mean the same thing everywhere: **an optional property accepts `null` as a sentinel for "absent," exactly as it accepts the property being entirely missing.** A present-but-`null` value validates against every optional type.

### The Semantic Rule (single source of truth)

> A typed property is **nullable** if and only if it is **not required**.

This rule is uniform across:

- Primitive scalars (`string`, `number`, `boolean`, `boolish`, `numberlike`, `date`, `datetime`, `time`, `url`, `email`, `file`, `enum`).
- `any`, with one important distinction: `any` already accepts every JSON value, including `null`, so `any(required)` remains a presence-only contract rather than growing a new `not null` rule in this bug fix.
- `object` and inline object literals (`{ foo: string }`).
- Arrays of any of the above (`string[]`, `file(match('*.md'))[]`, …).
- Property-level unions (`[string, number]`) and arms of root-level unions.
- Constrained atoms (`string(not-empty)`, `number(min(0))`, `file(match('*.png'))`, …) — `null` bypasses inner constraints because the typed arm is never reached.

Required typed atoms (`string(required)`, `number(required)`, `object(required)`, etc.) continue to reject `null` because their typed fragments still reject it. The `any(required)` exception is deliberate and preserves the established SimplifiedSchema contract that `any` means "anything." This avoids widening the bug fix into a breaking change for callers that use `any(required)` to require presence while accepting arbitrary frontmatter values.

For property-level unions, `required` is still hoisted from any arm. Therefore a union property is nullable only when **none** of its arms declares `required`.

> **Reader's note:** an earlier version of this draft treated every required atom, including `any(required)`, as non-null. That would require changing `any` from `{}` to a `not null` schema when required, which is a broader compatibility break than this bug needs. This spec keeps the fix focused on optional typed properties accepting `null`.

### Optional `file` Behavior (preserved, extended)

Decision A's existing empty-string tolerance stays. An optional scalar `file` field now accepts all three of:

1. Absent (the property key is missing) — already valid.
2. `""` (empty string) — already valid via Decision A's `{"const": ""}` arm.
3. `null` — newly valid via the general nullable wrap.

A required scalar `file` field still rejects both `""` and `null`.

### Effect on Claudine Composition

With the darkmatter fix in place, claudine's `claudine/lib/src/composition/schema_validation.rs` paths all do the right thing without further changes:

- `pre_validate_schema` runs jsonschema validation against the raw frontmatter (with `set_overrides` applied). A present `null` will now pass instead of being flagged as `ValidationProblemKind::Type`. The existing `value_needs_composition` deferral for template strings is unaffected — it still defers template-bearing values to the post-compose validator.
- `prepare_direct_with_schema` / `prepare_inline_with_schema` invoke Darkmatter's compose pipeline, which runs schema validation against the composed (post-interpolation, pre-shell-expansion) frontmatter. A template that resolves to `null` now validates.
- `post_shell_validate` runs jsonschema validation against the post-shell-expansion effective frontmatter. A `$(...)` shell expression that emits `null` (rare, but possible via something like `echo null`) now validates for optional properties.

The `categorize_problems` / `is_required` helpers in claudine need **no** changes: they read `Constraint::Required` from the `PropertyAtom`, which the converter still emits unchanged for required atoms. The JSON Schema output change is invisible to claudine's categorization logic.

### What Must Not Change

- The SimplifiedSchema authoring grammar. No new constraint keyword (no `nullable`, no `non-null`). Users who want non-null behavior already have it via `required`.
- The existing Decision A wrap for optional `file` and `""` — preserved verbatim.
- The shape of fragments emitted for **required typed** atoms — they remain bare typed fragments, never null-wrapped.
- `convert.rs`'s public output stability for the `required` typed case — required typed atoms remain byte-for-byte identical so existing snapshots and external consumers are unaffected. `any(required)` also remains byte-for-byte identical (`{}` at the fragment level plus the parent `required` entry).
- Claudine's typed schema error categorization (`is_required`, `categorize_problems`, `interactive_shape_for_atom`). These read the `PropertyAtom`, not the converted JSON Schema.
- The public coercion semantics. Existing scalar coercions must still happen for non-null values behind a nullable wrapper; `null` itself must pass through untouched.

## Proposed Implementation

The schema-shape change starts in `darkmatter/lib/src/markdown/schemas/simplified/convert.rs::atom_to_schema`. The existing `optional_empty_file` block is generalized into a two-stage wrap:

```rust
// Decision A (generalized): an optional property accepts `null` as an
// "absent" sentinel, mirroring how an optional property may be entirely
// missing from the frontmatter. Required atoms keep the bare fragment.
// Optional `file` atoms additionally accept `""` (the original Decision A
// surface) so templates that produce empty strings for missing files
// still validate.
let is_optional = !required;
let is_optional_scalar = is_optional && !atom.is_array;
let is_optional_file =
    is_optional_scalar && matches!(&atom.ty, TypeExpr::Primitive(SimplifiedType::File));

let mut schema = if atom.is_array {
    // Optional arrays: wrap the entire array fragment in `anyOf: [null, ...]`
    // so a templated null on `tags: string[]` validates. The inner fragment
    // (array type + items + array constraints) is unchanged.
    let mut arr = Map::new();
    arr.insert("type".into(), Value::String("array".into()));
    arr.insert("items".into(), inner);
    apply_array_constraints(name, &mut arr, &atom.array_constraints)?;
    if required {
        Value::Object(arr)
    } else {
        wrap_optional_null(Value::Object(arr))
    }
} else if is_optional_file {
    // Optional file: accept null OR empty-string OR the file fragment.
    wrap_optional_null_with_empty_file(inner)
} else if is_optional_scalar {
    // Any other optional scalar/object/inline-object/any: accept null OR
    // the typed fragment.
    wrap_optional_null(inner)
} else {
    // Required scalar: bare fragment, byte-for-byte unchanged.
    inner
};
```

`wrap_optional_null` should produce the Draft 2020-12 null arm as `{"type":"null"}`. For `any`, the wrapper is technically redundant (`{}` already accepts `null`), but keeping the shape uniform for optional properties is useful for schema introspection and for documenting that `null` is intentionally allowed.

The `default` and `description` attachment block that follows (`if let Value::Object(map) = &mut schema { ... }`) should keep attaching annotations to the **outer property schema**. This is the correct JSON Schema annotation location for the property as a whole, including the `null` branch. Do not move annotations exclusively onto the typed arm; doing so would make annotations disappear for instances that match the `null` branch and would diverge from the existing optional-`file` wrapper behavior.

> **Reader's note:** the pre-review draft suggested descending into the typed arm for `default(...)` and `-> description`. That was corrected here. If `md schema about` or a future IDE hover has trouble reading wrapped schemas, fix the reader to understand Darkmatter's nullable wrapper rather than relocating property annotations away from the property schema.

### Property-Level Unions

`union_property_to_schema` calls `atom_to_schema` per arm and wraps the results in an outer `anyOf`. With this fix, an optional property-level union (`[string, number]` without `required` on any arm) would produce a nested `anyOf` shape if every arm were independently nullable:

```json
{ "anyOf": [
    { "anyOf": [ { "type": "null" }, { "type": "string" } ] },
    { "anyOf": [ { "type": "null" }, { "type": "number" } ] }
] }
```

This is correct for validation (jsonschema flattens nested `anyOf` semantically) but ugly and harder for coercion/introspection to recognize. Lift the `null` wrap out of `atom_to_schema`'s union path and apply it **once** at the property level when `required` is false. Concretely, `union_property_to_schema` should wrap its emitted `anyOf` in `{ "anyOf": [ { "type": "null" }, <union anyOf> ] }` when `!required`, and `atom_to_schema` should be callable in a "union arm" mode where it does not apply the general null wrap.

Preserve the optional `file` empty-string sentinel inside union arms. For example, an optional `[file(match('*.md')), number]` property should accept `null` at the property level and should still accept `""` through the file arm. A simple way to model this is:

- General optional-null wrapping happens once at the property level for a union.
- The scalar optional-file empty-string arm remains part of the file atom fragment when the containing union is optional.
- Required unions (`required` on any arm) receive neither the property-level `null` arm nor optional-file empty-string tolerance.

The clean-output approach is required for snapshot stability, downstream readability, and coercion.

### Inline-Object Atoms

`atom_to_schema` already delegates inline objects to `inline_object_fragment` and returns the resulting `Value::Object`. The generalized wrap above covers them transparently: an optional inline object atom `{ foo: string }` emits `{"anyOf": [ {"type":"null"}, {"type":"object", "additionalProperties": false, "properties": {"foo": ...}} ]}`. No special handling required.

### Coercion Impact

This fix cannot live only in `convert.rs`. Darkmatter's coercion layer is driven from the generated JSON Schema, and `coercion_target` currently returns `None` for unrecognized `anyOf` shapes. If optional `string` changes from `{"type":"string"}` to `{"anyOf":[{"type":"null"},{"type":"string"}]}` without a coercion update, existing compose-time coercion for non-null optional strings stops working.

Update `darkmatter/lib/src/markdown/schemas/coerce.rs` so nullable wrappers are transparent to non-null values:

- Recognize Darkmatter's optional-null wrapper shape and recurse into the non-null typed arm.
- Recognize the optional-file wrapper shape (`null`, `""`, typed file arm) and recurse into the typed file arm.
- Recognize property-level nullable union wrappers and continue using the existing per-arm union coercion path for non-null values.
- Leave `Value::Null` untouched; it should validate against the null arm, not be coerced.
- Keep boolish and numberlike exact-shape matching strict. Nullable wrappers should unwrap to those existing exact shapes; they should not make arbitrary user-authored `anyOf` schemas coercible.

## Files Most Likely to Change

- `darkmatter/lib/src/markdown/schemas/simplified/convert.rs` — `atom_to_schema` (the wrap), `union_property_to_schema` (lift the null wrap to the property level for clean output while preserving optional-file empty-string tolerance), and small wrapper helpers.
- `darkmatter/lib/src/markdown/schemas/simplified/convert.rs` `#[cfg(test)] mod tests` — new tests covering every optional type's null tolerance, plus regression tests that required atoms still reject null.
- `darkmatter/lib/src/markdown/schemas/coerce.rs` — teach coercion recognition to unwrap Darkmatter's nullable wrappers for non-null values.
- `darkmatter/lib/src/markdown/schemas/coerce.rs` `#[cfg(test)] mod tests` — regression tests proving optional nullable wrappers preserve existing non-null coercions and leave `null` untouched.
- `darkmatter/features/_completed/2026-05-11-schemas/spec.md` — add a callout under "Optional by default" stating that optional properties accept `null` as the absent sentinel, with a worked example.
- `darkmatter/docs/topics/schema-definition.md` — update the canonical SimplifiedSchema topic docs in the same way; this is the user-facing documentation, not only historical feature-spec maintenance.
- `claudine/docs/topics/composition.md` — under "Required vs Optional," add a row noting that a present `null` validates for optional properties (paralleling how a missing key validates).
- `claudine/lib/src/composition/schema_validation.rs` `#[cfg(test)] mod tests` — add an end-to-end test that reproduces the review-feature.md incident (a `string` schema property whose template resolves to `null`) and asserts `prepare_direct_with_schema` succeeds.

## Gotchas Worth Repeating Up Front

- **`default` and `description` must survive the wrap.** Today they are attached to the outermost property schema, including the existing optional-file `anyOf` wrapper. Keep that behavior. Schema readers must learn to look through the nullable wrapper for type details, but property annotations belong on the wrapper.
- **Array `items` and array constraints must stay inside the array arm.** `apply_array_constraints` writes `minItems` / `maxItems` / `uniqueItems` to the array fragment. The null wrap must wrap the *finished* array fragment, not its `items` schema.
- **Snapshot drift.** The existing `v1_scalar_atoms_are_byte_for_byte_unchanged` test (convert.rs lines ~1093–1129) asserts the exact JSON keys for scalar atoms. With this change, **optional** scalar atoms gain an `anyOf` wrap and the snapshot must be updated; **required** scalar atoms must remain byte-for-byte unchanged. Split the test into required-vs-optional variants rather than relaxing it.
- **Property-level union output shape.** The naive per-arm wrap produces nested `anyOf`. Lift the wrap to the property level so `union_property_to_schema`'s output stays a single-level `anyOf` plus a null arm, while keeping optional-file `""` acceptance inside the file arm.
- **Root-level unions are unaffected.** Each arm is an object schema; null tolerance for optional properties inside an arm is handled by the per-property wrap. The root `anyOf` itself is unchanged (a root document instance is never `null`).
- **Coercion must not fire on null, but must still fire on non-null values.** `coercion_target` currently returns `None` for `anyOf` shapes it does not recognize (`target_from_any_of` only matches boolish and numberlike). That must change for Darkmatter's own nullable wrappers, or optional string/number/boolean coercion regresses. A `string` value against an optional `string` must still coerce as `ToString`; a `null` value against the same property passes through to validation and matches the null arm.
- **No new authoring grammar.** Resist the temptation to add `nullable` / `non-null` constraints in this fix. The semantics "optional typed properties accept `null`" is sufficient; users who want non-null typed behavior already have `required`. A future extension can revisit if a real need arises.
- **JSON Schema `type: "null"`.** Draft 2020-12 spells the null arm as `{"type": "null"}`, not `{"type": null}`. The wrap must use the string form.
- **The `review-feature.md` reproduction is the smoke test.** The incident that surfaced this bug — a `string` schema property whose template resolves to `null` — is the simplest end-to-end verification. Add it as a claudine composition test so this specific failure mode never silently regresses.

## Required Documentation Updates (Do Not Skip)

- `darkmatter/features/_completed/2026-05-11-schemas/spec.md` — under the "Optional by default" paragraph (around line 63), add: "An optional property accepts `null` as a sentinel for absent. A frontmatter slot whose value resolves to `null` (e.g. from a Darkmatter ternary `{{ x ? y : null }}`) validates the same way as a missing key." Include a short example.
- `darkmatter/docs/topics/schema-definition.md` — make the same update under the public "Optional by default" docs, and add a short note in the type table that `any(required)` remains presence-only because `any` includes `null`.
- `claudine/docs/topics/composition.md` — under "Required vs Optional" (the table around line 322), add a row or note clarifying that a *present* `null` validates for optional properties. Today's table only documents Missing vs Present-and-valid vs Present-but-invalid; the spec change means `null` is in the "Present and valid" column for optional properties.
- `.claude/skills/darkmatter/SKILL.md` (or sibling) — if the SimplifiedSchema section mentions type validity, add a one-liner: optional = nullable.
- `darkmatter/CLI` schema-explain output (if it surfaces type info) — verify it still reads cleanly through the new wrap. No code change expected, but worth a manual check.

## Acceptance Criteria

A. **Optional primitives accept `null`.** For every typed primitive SimplifiedType (`string`, `number`, `boolean`, `boolish`, `numberlike`, `date`, `datetime`, `time`, `url`, `email`, `enum`), the schema generated from `$schema: { opt: <type> }` validates each of:

- `{}` (absent)
- `{"opt": null}` (present null)
- a well-typed non-null value (e.g. `{"opt": "x"}` for `string`)

Required typed variants (`<type>(required)`) reject `{"opt": null}` with a Type problem.

B. **Optional `file` accepts both `null` and `""`.** `$schema: { opt: file }` validates each of `{}`, `{"opt": null}`, `{"opt": ""}`, and `{"opt": "@existing.md"}`. The existing Decision A test (`optional_file_accepts_empty_string_as_absent`) continues to pass, plus a new sibling test for `null`.

C. **Optional `object` and inline objects accept `null`.** `$schema: { opt: object }` and `$schema: { opt: "{ foo: string }" }` each validate `{}` and `{"opt": null}`.

D. **Optional arrays accept `null`.** `$schema: { opt: string[] }` validates `{}`, `{"opt": null}`, `{"opt": []}`, and `{"opt": ["a"]}`.

E. **Optional property-level unions accept `null`.** `$schema: { opt: [string, number] }` validates `{}`, `{"opt": null}`, `{"opt": "x"}`, `{"opt": 3}`.

F. **Constraints on optional atoms are bypassed by `null`.** `$schema: { opt: "string(not-empty; min(5))" }` validates `{"opt": null}` (constraint checks never fire against the null arm) while still rejecting `{"opt": ""}` and `{"opt": "ab"}`.

G. **Required typed atoms still reject `null`.** `$schema: { req: "string(required)" }` rejects `{"req": null}` with a Type problem. Existing required typed-atom behavior is unchanged.

H. **`any` behavior is explicit.** `$schema: { opt: any }` validates `{}`, `{"opt": null}`, and any non-null value. `$schema: { req: "any(required)" }` rejects `{}` but accepts `{"req": null}` because `any` is presence-only when required. This is documented as an intentional compatibility preservation.

I. **`default` and `description` survive the wrap.** `$schema: { opt: "string(default(hello)) -> A greeting" }` produces a schema whose outer property schema carries `default: "hello"` and `description: "A greeting"`. A regression test asserts the annotations are present on the outer `anyOf` wrapper, preserving existing optional-file annotation behavior.

J. **Snapshot stability for required atoms.** The JSON Schema emitted for every **required** scalar atom is byte-for-byte identical to today's output. Update the existing `v1_scalar_atoms_are_byte_for_byte_unchanged` test to assert required-atom stability separately from optional-atom snapshots.

K. **End-to-end claudine test.** A test in `claudine/lib/src/composition/schema_validation.rs` reproduces the review-feature.md incident: a composition source whose `$schema` declares an optional `string` property and whose frontmatter value resolves (via Darkmatter template) to `null`. `prepare_direct_with_schema` succeeds and produces a `PreparedComposition` whose `effective_frontmatter` retains the resolved `null` (the value is **not** silently dropped as an invalid optional).

L. **Coercion regression.** Existing coercion tests in `darkmatter/lib/src/markdown/schemas/coerce.rs` continue to pass. New tests assert that nullable optional wrappers preserve non-null coercion for `string`, `number`, `boolean`, `boolish`, `numberlike`, arrays, inline objects, and property-level unions, while `null` itself is left untouched.

M. **No claudine code changes required.** The behavior fix is entirely in darkmatter. If implementation reveals that claudine's `categorize_problems` or `is_required` needs touching, that is a defect in the design and the spec must be revised before proceeding.

## Out of Scope (Non-Goals)

- **No `nullable` / `non-null` authoring keywords.** Users who need non-null semantics use `required`. A future spec can revisit if a concrete use case emerges (e.g. "required but nullable for serialization compatibility with an external system").
- **No change to YAML serialization of `null`.** How Darkmatter writes `null` back to disk on inline-compose closure is a separate concern.
- **No change to the Interactive Mode collection flow.** A `null` value is not "missing," so the interactive prompt loop does not fire for it. This is already the correct behavior and is unchanged.
- **No change to root-level union handling.** Root-level unions validate the entire document; null tolerance for their inner optional properties is covered by the per-property wrap.
- **No change to claudine's drop-invalid-optionals logic.** A present `null` no longer qualifies as "invalid" once the schema accepts it, so the drop path is simply not reached. No code change.

## References

- Observed incident: `claudine compose @prompts/review-feature.md` against a feature folder without a `design.md`, producing `CompositionError: schema validation failed — type design: null is not of type "string" at 9:1`.
- Source: `claudine/prompts/review-feature.md:1-11` — the document whose frontmatter declares the optional `design: string` schema property and the templated `design:` value that resolves to `null`.
- Source: `darkmatter/lib/src/markdown/schemas/simplified/convert.rs:199-218` — Decision A, the existing optional-`file` empty-string wrap that this fix generalizes.
- Source: `darkmatter/lib/src/markdown/schemas/simplified/convert.rs:180-232` — `atom_to_schema`, the function where the wrap is added.
- Source: `darkmatter/lib/src/markdown/schemas/simplified/convert.rs:134-176` — `union_property_to_schema`, where the null wrap is lifted to the property level for clean output.
- Source: `claudine/lib/src/composition/schema_validation.rs:558-652` — `categorize_problems` and `is_required`, which read `Constraint::Required` from the `PropertyAtom` and are unaffected by the JSON Schema shape change.
- Source: `darkmatter/features/_completed/2026-05-11-schemas/spec.md:63` — "Optional by default" contract that this fix makes literally true.
- Related: `darkmatter/lib/src/markdown/schemas/coerce.rs:76-101` — `coercion_target`, which must learn to unwrap Darkmatter's nullable `anyOf` shapes for non-null coercion while leaving null values untouched.
