# Schema-driven frontmatter type coercion

When a frontmatter property is constrained by a `$schema` (or baseline schema) and the
value supplied for that property is *trivially* of the wrong JSON type but unambiguously
convertible to the declared type, Darkmatter should **coerce the stored value** to the
declared type and accept it — rather than failing validation. Coercion is default-on and
requires no opt-in.

## Motivating failure

The following real invocation fails today:

```sh
claudine compose prompts/implement.md spec="features/2026-05-28-darkmatter-hashing/plan.md" -y total_phases=8 phase=1 --claude
```

```text
MarkdownError: schema validation failed
type has_plan: "false" is not of type "boolean" at 31:1 (schema arm 1)
type has_review: "false" is not of type "boolean" at 32:1 (schema arm 1)
type has_spec: "true" is not of type "boolean" at 30:1 (schema arm 1)
```

The offending frontmatter is computed by interpolation expressions:

```yaml
has_spec:  "{{spec ? true : false}}"
has_plan:  "{{plan ? true : false}}"
has_review:"{{review ? true : false}}"
```

The ternary's `true`/`false` literals render **into a quoted scalar**, so the stored value
is unavoidably the *string* `"true"` / `"false"`. The schema arm types these properties as
strict `boolean`, and strict JSON Schema validation rejects a string. The conversion
`"true"` → `true` is completely unambiguous — there is exactly one boolean a value like
`"true"` can mean. The author should not have to fight the type system, and the same class
of mismatch arises from any value source: shell expansion (`$(...)` always yields a
string), interpolation expressions, ternaries, and quoted CLI key/value arguments.

### Correctness, not just ergonomics

These properties are not inert. They feed downstream pipeline conditions:

```yaml
init:
  stack:
    - when: has_plan        # init-stack condition
```

```markdown
::block when="has_spec"     # page-block condition
::block when="!has_spec"
```

If the stored value remains the **string** `"false"`, condition evaluators commonly treat
any non-empty string as truthy, so `!has_spec` can misfire — a latent logic bug. Coercing
the *stored value* to a real boolean (not merely widening what validation accepts) fixes
both the validation error and this latent bug at once. This is why coercion mutates the
stored frontmatter value rather than only relaxing the validator.

## What coercion does

Coercion is driven by each property's **declared type**, read from the authoritative
compiled JSON Schema (the merged document `$schema` + baseline). When the instance value's
JSON type does not match the declared type, Darkmatter attempts a single, unambiguous
conversion. If the conversion succeeds, the converted value replaces the original; if it
fails, the value is left untouched and the **existing strict type error is reported as it
is today**.

### Coercion matrix

| Declared type (JSON Schema fragment) | Incoming value | Coerced result |
|---|---|---|
| `{"type":"boolean"}` | string in the boolish set `{true,false,True,False,TRUE,FALSE}` | real boolean |
| boolish `{"anyOf":[{"type":"boolean"},{"enum":["true","false",…]}]}` | same boolish set | real boolean (normalized) |
| `{"type":"number"}` / `{"type":"integer"}` | string matching `^-?\d+(\.\d+)?$` | real number |
| numberlike `{"anyOf":[{"type":"number"},{"type":"string","pattern":…}]}` | numeric string (same regex) | real number (normalized) |
| `{"type":"string"}` — including `date` / `datetime` / `time` / `url` / `email` / `file` (all compile to `type:string` + `format`/`pattern`) | a `number` or `boolean` scalar | its canonical string (`42` → `"42"`, `3.14` → `"3.14"`, `true` → `"true"`) |
| `{"type":"array","items":T}` | an array | each element coerced by `T`'s rule (recursively) |
| `{"type":"object"}`, `any` (`{}`), bare `{"enum":[…]}` without a `type`, or a `null` value | — | untouched |

Two things follow from the matrix:

- **The lenient SimplifiedSchema types `boolish` and `numberlike` now *normalize*** the
  stored value to the canonical JSON type, where previously they only *accepted* the string
  form and left it as a string. After this feature, a `boolish` field holding `"true"`
  becomes a real `true`, just like a strict `boolean` field would. This removes the latent
  truthiness bug for those types too.
- **Coercion toward `string` is reverse-direction and always unambiguous** — every scalar
  has exactly one canonical string form. It targets `string` only; we never parse a string
  *into* a date/url/email/etc. A number landing in a `date` field coerces to its string form
  and then fails the `date` format check normally — coercion never produces a false accept.

### What is never coerced (ambiguous)

The following are **not** coerced and continue to fail strict validation when the declared
type does not match:

- `"yes"` / `"no"` / `"on"` / `"off"` → boolean (not in the boolish set; ambiguous)
- `"1"` / `"0"` → boolean (ambiguous: these are equally valid as numbers)
- any string that does not match the numeric regex → number
- arrays, objects, or `null` → any scalar type
- a string → object, or an object → string

Anything outside the matrix falls through unchanged to the existing validator, which
reports the same `Type` problem it does today. Coercion only ever *adds* acceptances for
unambiguous cases; it never changes the outcome of a value that is already valid, and never
masks a genuinely-wrong value.

## Approach: single JSON-Schema-driven path

Coercion is derived by walking the **merged compiled JSON Schema** (`EffectiveSchema.json_schema`),
which is always present and is the authoritative result of merging the document `$schema`
with any baseline. This is deliberately chosen over the SimplifiedSchema AST: the AST is
`None` for raw-JSON-Schema documents and does **not** include baseline-merged properties,
so an AST-driven approach would silently skip coercion for baseline-declared fields and
raw-JSON-Schema schemas. One JSON-Schema-driven path covers every case — inline `$schema`,
baseline-merged fields, raw JSON Schema, and root unions — with no second code path to keep
in sync.

A small recognizer maps a property's schema fragment to a coercion target:

```text
{type: boolean}                         -> ToBoolean
{anyOf:[{type:boolean},{enum:[…]}]}     -> ToBoolean   (boolish)
{type: number} | {type: integer}        -> ToNumber
{anyOf:[{type:number},{type:string,p}]} -> ToNumber    (numberlike)
{type: string} (with/without format)    -> ToString
{type: array, items: T}                 -> element-wise target of T
everything else                         -> no coercion
```

The numberlike / boolish `anyOf` shapes are produced by Darkmatter's own
`simplified/convert.rs` and are internally stable; recognizing them is a small, well-defined
helper.

### Root unions (the `anyOf` arm model)

The motivating schema is a three-arm root union (`{"anyOf":[arm0, arm1, arm2]}`). Coercion
must make the instance satisfy *at least one* arm, and different arms may type the same
property differently. The algorithm:

1. For each arm, **in index order**, build a coerced candidate by applying that arm's
   per-property coercion targets to the instance.
2. Strict-validate the candidate against that arm.
3. The **first arm that validates post-coercion wins**; its coerced candidate is committed.
4. If no arm validates post-coercion, return the instance unchanged and let the existing
   union error reporting run (which already attributes problems to the closest-matching arm
   and carries its `arm_index`).

This mirrors the existing `collect_root_union_problems` arm model and the `(schema arm N)`
attribution already shown in error output. For non-union schemas there is a single implicit
arm and the algorithm reduces to "coerce, then validate".

## Where coercion runs

### Library validation (`EffectiveSchema::validate*`)

`EffectiveSchema::validate_with_positions` coerces a **working copy** of the instance first,
then validates the coerced copy. The returned `ValidationReport` therefore reflects
post-coercion validity. This keeps `md schema validate`, the library `DarkmatterSchemas::validate`
API, and the compose pipeline in agreement about what is valid. Because this path operates
on a copy, it performs **no document mutation** — it is purely a check. A new pure helper,

```text
coerce_frontmatter(json_schema: &Value, instance: &Value) -> { value: Value, changed: bool }
```

(living in a new `schemas/coerce.rs`) is the single source of truth, used by both the
library validation path and the compose write-back.

### Compose pipeline write-back (`schema_validation::run`)

The compose Schema Validation stage is where the stored value is actually mutated so the
real type flows downstream. The stage runs after frontmatter interpolation and before
frontmatter shell expansion — exactly where the `has_*` ternary values are already resolved
to `"true"`/`"false"` strings. The stage:

1. Resolves the effective schema (document `$schema` merged with `ComposeOptions::baseline_schema`).
2. Builds the frontmatter instance and runs `coerce_frontmatter`.
3. **Writes coerced top-level properties back** into `frontmatter_mut().as_map_mut()`, so
   the real boolean/number/string is visible to every later stage (shell expansion, page
   blocks, body interpolation, init-stack conditions) and to the composed output.
4. Validates (now idempotent — re-coercing already-coerced values is a no-op).

The stage's signature changes from `run(&Markdown, …)` to `run(&mut Markdown, …)`; the call
site at `compose/mod.rs` passes `self`, which is already `&mut`.

### Shell-deferral contract is preserved

The stage today **defers** any problem on a value still containing `$(...)`, because shell
expansion runs afterward and the consumer (e.g. claudine's `prepare_*_with_schema`)
re-validates the post-shell frontmatter. Coercion honors this contract: a value still
holding `$(...)` is **skipped** by coercion (not coerced, not errored) at this stage. Its
real type is resolved later, at the post-shell re-validation point, which uses the same
`coerce_frontmatter` helper and so coerces consistently. A value that is *not* shell-pending
(like the resolved `has_*` strings) is coerced and written back here.

## Edge cases

- **Properties not declared in the schema** are never touched.
- **Nested object properties** are not deeply coerced — coercion applies to top-level
  properties and to the elements of top-level typed arrays. SimplifiedSchema's `object`
  type is opaque (it declares no sub-property types), so there is nothing to coerce against
  inside an object.
- **`integer` fields** (only reachable via raw JSON Schema baselines): a numeric string
  coerces to a JSON number; `"42"` → `42` validates as integer, while `"3.14"` → `3.14`
  fails the integer check normally.
- **`format`/`pattern`-constrained strings** (`date`, `email`, …): a scalar coerces to its
  string form and then the format/pattern check runs unchanged — coercion never bypasses it.
- **Idempotence**: coercing an already-correctly-typed value is a no-op, so running the
  compose stage's write-back and then validating does not double-convert.

## Scope and non-goals

- **No opt-out flag** (e.g. `--no-coerce` / strict-types). Coercion is unambiguous by
  construction and default-on; a strict-mode escape hatch is explicitly out of scope for v1
  and can be added later if a real need appears.
- **No `md schema validate --write`** to persist coerced values to disk. The library check
  path reports post-coercion validity but does not rewrite files; only the compose pipeline
  mutates the (in-memory) document it is composing.
- **No new coercions beyond the matrix** — in particular no `"yes"/"no"/"1"/"0"` → boolean,
  no string-parsing into date/url/email, and no object/array coercions.
- **Error-message wording is unchanged.** Values that remain uncoercible produce the same
  `Type` problem text as today. Enhancing those messages (e.g. "recognized booleans are
  true/false") is a possible future improvement, not part of this feature.

## Success criteria

- The motivating `claudine compose prompts/implement.md …` invocation succeeds, with
  `has_spec` / `has_plan` / `has_review` stored as real booleans in the composed document.
- A strict `boolean` field holding `"true"`/`"false"` (and the boolish case spellings)
  validates and is stored as a real boolean; a strict `number` field holding a numeric
  string validates and is stored as a real number.
- A `string` field holding a number or boolean validates and is stored as the canonical
  string form.
- `boolish` and `numberlike` fields normalize their stored value to the canonical JSON type.
- Typed arrays coerce element-wise.
- Root-union schemas coerce against the first arm that validates post-coercion.
- Ambiguous values (`"yes"`, `"1"` → boolean) and out-of-matrix values still fail with the
  existing strict type error.
- Values still holding `$(...)` are not coerced at the pre-shell stage and remain deferred.
- `md schema validate` and the compose pipeline agree on validity for the same document.
