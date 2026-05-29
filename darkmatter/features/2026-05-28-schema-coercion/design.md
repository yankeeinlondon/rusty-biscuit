# Phase 1 — Coercion API Design (code-level decisions)

This is the Phase 1 deliverable: the API surface, recognized JSON Schema
fragment shapes, the union algorithm, and the `$(...)` skip rule, fixed in
code-level terms so Phases 2–4 can be implemented against a stable contract.

## Module placement

New file `darkmatter/lib/src/markdown/schemas/coerce.rs`, registered as
`pub mod coerce;` in `schemas/mod.rs` (after `pub mod completion;`, keeping
alphabetical-ish ordering already present).

## Public API surface (in `coerce.rs`)

```rust
/// The conversion a recognized property schema asks for.
pub enum CoercionTarget {
    ToBoolean,
    ToNumber,
    ToString,
    Array(Box<CoercionTarget>),
}

/// Result of coercing a whole instance against a schema.
pub struct CoercionOutcome {
    pub value: serde_json::Value,
    pub changed: bool,
}

/// Maps a single property's JSON Schema fragment to its coercion target,
/// or `None` when the fragment is outside the coercion matrix.
pub fn coercion_target(property_schema: &serde_json::Value) -> Option<CoercionTarget>;

/// Single source of truth. Pure: builds a coerced copy of `instance` against
/// `json_schema` and reports whether anything changed. Never mutates inputs.
pub fn coerce_frontmatter(
    json_schema: &serde_json::Value,
    instance: &serde_json::Value,
) -> CoercionOutcome;
```

`coerce_frontmatter` is pure and self-contained. For the **root-union** case it
must strict-validate per-arm candidates; it builds the arm validators itself via
the existing `validate::build_validator` helper (which must be exposed as
`pub(super)` / `pub(crate)`). We deliberately do **not** thread the
`ValidatorCache` through this signature — coercion runs once per validate call,
not in a hot loop, and keeping the documented `(json_schema, instance)` signature
makes it trivially reusable by both the library validate path and the compose
write-back. (Optimization to reuse the cache is a possible later refinement, not
v1.)

## Recognizer: fragment shapes → target

`convert.rs` is the authoritative producer of these fragments. The recognizer
matches exactly the shapes it emits:

| Fragment | Target |
|---|---|
| `{"type":"boolean"}` | `ToBoolean` |
| `{"anyOf":[…]}` where one arm is `{"type":"boolean"}` **and** one arm has an `"enum"` key | `ToBoolean` (boolish) |
| `{"type":"number"}` or `{"type":"integer"}` | `ToNumber` |
| `{"anyOf":[…]}` where one arm is `{"type":"number"}` **and** one arm is `{"type":"string"}` carrying a `"pattern"` | `ToNumber` (numberlike) |
| `{"type":"string"}` (with or without `format`/`pattern`/`minLength`/…) | `ToString` |
| `{"type":"array","items":T}` | `Array(coercion_target(T))` — `None` if `T` unrecognized |
| `{"type":"object"}`, `{}` (any), bare `{"enum":[…]}` with no `type`, or anything else | `None` |

Recognition notes:

- **Distinguishing numberlike from a plain `string|number` property union:** the
  numberlike string arm *carries a `pattern`*; a generic `string|number` union
  arm does not. Requiring the `pattern` on the string arm prevents false
  positives. A generic property-level union (e.g. `["string","number"]`) yields
  `None`, so it is left untouched (matches the spec: only the specific
  boolish/numberlike shapes coerce).
- **boolish** is recognized by `{type:boolean}` arm + an arm with an `enum` key.
- Arm order is not assumed; match by scanning the `anyOf` array.
- `type` may legitimately be a JSON array in raw JSON Schema (e.g.
  `{"type":["string","null"]}`). Treat such multi-type fragments as **not
  recognized** (`None`) — they are outside the matrix and must be left to the
  validator. Only a single string `type` is recognized.

## Scalar coercion rules

Operate on a `&Value`, return `Option<Value>` (Some = coerced replacement, None =
leave the original element/value untouched):

- **`ToBoolean`:** `Value::String(s)` where `s ∈ BOOLISH_VALUES`
  (`{true,false,True,False,TRUE,FALSE}`) → `Value::Bool` (`*TRUE*`/`*True*`/`true`
  → `true`; the `*FALSE*` family → `false`). Anything else (incl. `"yes"`,
  `"1"`, non-string) → untouched.
- **`ToNumber`:** `Value::String(s)` where `s` matches `^-?\d+(\.\d+)?$` →
  parsed JSON number via `serde_json::from_str::<serde_json::Number>` so the
  result matches serde_json's own number model (integral strings → `i64`/`u64`,
  preserving values above `i64::MAX`; decimals → `f64`). The only literals left
  untouched are those too large even for `f64`, which then fail validation as
  strings. Non-matching string / non-string → untouched.
- **`ToString`:** `Value::Number(n)` → `Value::String(n.to_string())`
  (`42`→`"42"`, `3.14`→`"3.14"`); `Value::Bool(b)` → `"true"`/`"false"`.
  `Value::String` (already a string), `Null`, `Array`, `Object` → untouched.
- **`Array(inner)`:** `Value::Array(items)` → element-wise apply the scalar rule
  for `inner` (recursing for nested `Array`); elements that don't coerce are kept
  as-is (the validator reports them). Non-array value → untouched.

`null`, arrays-as-scalars, and objects are never coerced toward a scalar target.

## Shared constants (single source of truth)

Extract from `convert.rs` and reuse in both `convert.rs` and `coerce.rs`:

```rust
// in convert.rs (pub(super))
pub(super) const BOOLISH_VALUES: [&str; 6] =
    ["true", "false", "True", "False", "TRUE", "FALSE"];
pub(super) const NUMBERLIKE_PATTERN: &str = r"^-?\d+(\.\d+)?$";
```

`boolish_fragment` builds its `enum` array from `BOOLISH_VALUES`;
`numberlike_fragment` uses `NUMBERLIKE_PATTERN` for the string arm pattern.
`coerce.rs` imports both. For the numberlike *match test* in coercion, a small
hand-rolled `is_numberlike(&str)` that mirrors `^-?\d+(\.\d+)?$` exactly is
preferred over compiling a `regex::Regex` at runtime (the `regex` crate is a
dependency, but a once-cell regex is unnecessary for this trivial grammar). The
shared `NUMBERLIKE_PATTERN` constant remains the source for the *schema* the
validator compiles.

## Union algorithm (`coerce_frontmatter`)

1. If `json_schema.get("anyOf")` is an array (root union):
   - For each arm **in index order**: `wrap_arm_as_root_schema(arm)`; build a
     coerced candidate by applying that arm's per-property targets (read from the
     arm's `properties`) to the instance via the non-union object pass; strict-
     validate the candidate with `build_validator(&wrapped)`.
   - The **first arm whose candidate validates wins**: return
     `{ value: candidate, changed: candidate != instance }`.
   - If no arm validates post-coercion: return `{ value: instance.clone(),
     changed: false }` and let existing union error reporting run.
2. Else (single object schema): run the non-union object pass against
   `json_schema.properties` and return the result.

**Non-union object pass:** for each `(name, prop_schema)` in
`json_schema["properties"]`, if the instance has property `name` and
`coercion_target(prop_schema)` is `Some(target)`, attempt coercion; on success
replace the stored value and mark changed. Properties absent from the instance,
or with no recognized target, are left untouched. Properties in the instance but
not in the schema are never touched.

**Idempotence:** coercing an already-correctly-typed value is a no-op (the scalar
rules only fire on a *mismatched* JSON type), so the pass reports `changed:
false` for an already-valid instance, and re-running after a write-back does not
double-convert.

## Where coercion runs (Phases 3–4 contract)

- **Library (`EffectiveSchema::validate_with_positions`)** — coerce a *working
  copy* of the instance using `self.json_schema`, then validate the coerced copy.
  No document mutation. The position-less `validate` inherits this via
  `validate_with_positions`. Because it reads `json_schema` (post baseline
  merge), baseline-merged fields and raw-JSON-Schema documents (where
  `simplified` is `None`) coerce with no AST consultation.
- **Compose (`schema_validation::run`)** — signature becomes
  `run(&mut Markdown, …)`. After resolving the effective schema, build the
  instance, call `coerce_frontmatter`, and write coerced **top-level** properties
  back into `markdown.frontmatter_mut().as_map_mut()` (an
  `IndexMap<String, Value>`), so real types flow to every later stage and the
  composed output.

## `$(...)` skip rule (pre-shell deferral, preserved)

The compose write-back must **skip** any top-level value that still contains a
`$(...)` shell expression (reuse / align with the existing
`value_needs_shell_expansion`): such a value is neither coerced nor written back
nor errored at the pre-shell stage. Its real type is resolved later, at the
post-shell re-validation point, which goes through
`EffectiveSchema::validate*` and therefore coerces via the **same**
`coerce_frontmatter` helper. A value that is *not* shell-pending (the resolved
`has_*` strings) is coerced and written back at this stage.

Concretely: when building the instance to write back, exclude shell-pending
top-level keys from the coercion/write-back so their literal `$(...)` form
survives untouched into shell expansion. The existing deferral filter on
*reported problems* stays as-is.
