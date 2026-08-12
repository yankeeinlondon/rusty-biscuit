---
status: ready for planning and implementation
reviewed: true
review_iterations: 4
inputs:
  - ../../lib/src/markdown/schemas/simplified/types.rs
  - ../../lib/src/markdown/schemas/simplified/grammar.rs
  - ../../lib/src/markdown/schemas/resolve.rs
  - ./example.yaml
  - ./types.yaml
  - ./today-example.yaml
  - ./as_unordered_list-example.yaml
related:
  - ../2026-07-08-single-sourcing-schema/spec.md
---

# SimplifiedSchema Composition Primitives

**Status:** Reviewed and ready for planning and implementation. This spec defines
four additions that turn SimplifiedSchema from a flat single-file validator into
a **composable type system**. Consumed first by
[single-sourcing-schema](../2026-07-08-single-sourcing-schema/spec.md), which uses
them to model context-variable examples as validated YAML artifacts (its "E3"
option) instead of a Rust sidecar.

The features are independent in grammar but cohesive in intent:

- **A — `example()` constraint**: attach one or more example YAML files to a
  property.
- **B — `@` cross-file named-type import** (+ `this`): reference a named type
  defined in another schema file.
- **C — pattern/dictionary keys**: type objects whose keys follow a pattern
  rather than a fixed literal set.
- **D — content-format string types**: accept and validate embedded YAML/JSON
  content, including native YAML values that are coerced to strings when a
  composing caller asks for write-back.

## Why (single-sourcing's E3)

single-sourcing needs a home for context-variable examples that (a) is not the
Rust catalog and (b) does not bloat the validation grammar. The answer: examples
are **their own schema-validated YAML files**, referenced from a property via an
`example()` constraint. This keeps the base schema pure, makes examples
first-class data validated by SimplifiedSchema itself, and needs the two general
composition primitives (B, C) plus the content-format type (D) to express the
example/parameter schemas cleanly.

## Feature A — `example()` constraint

Attaches example artifacts to a property.

```yaml
today: "date(generated; required; example(./ctx/today-example.yaml)) -> Local date…"
```

- New `Constraint::Example(Vec<FileReference>)` (sibling to the existing
  file-referencing `Match`). Comma-separated file refs:
  `example(./a.yaml, ./b.yaml)`.
- **Non-validating for the annotated document** — examples are documentation, like
  a richer `description`. They do not constrain the property's value.
- **Each referenced file is itself validated at schema-load time**; a malformed
  example is a schema error (fail loud — the point is trustworthy examples).
  Validation has two layers: the common artifact envelope validates against
  `example.yaml`, then target-specific fields such as `parameters` validate
  against the annotated property or typed expression-function signature.
- Resolution: `biscuit_file::FileReference` + magic paths, relative to the
  **referencing schema file** (mirrors `$schema` file-ref resolution in
  `resolve.rs`), plus the new `this` (Feature B).
- JSON Schema mapping: `x-darkmatter-example` carrying the resolved example
  objects, so downstream tools (`md schema about`, DMLS hover) read them without
  re-reading disk.

### The example artifact schema (`example.yaml`)

The corrected authored schema:

```yaml
$schema:
    kind: enum(example; required)
    invocation:
        - string(required)
        - { frontmatter: yaml }
    returns: any
    description: string
```

Notes:

- `type` and parameter **type declarations** are intentionally **not** fields in
  `example.yaml`. They are inherited from the annotated property or typed
  expression-function catalog (O-A1). This prevents example artifacts from
  drifting away from the thing they demonstrate.
- `parameters` still exists in example **instances** as an optional runtime-input
  block, but its shape is inherited from the target. For expression functions it
  is an array of single-key maps, each binding a variable **name → its initial
  value** (O-A4), so `parameter` is `{ "<string>": any }` with exact one-key arity
  (Feature C). The `any` value holds both a native array (`[1,2,3]`) and a
  list-string (`"- 1\n- 2\n- 3"`) — the two representations
  `as_unordered_list-example{,-2}.yaml` show. Example input data has three
  sources: `parameters`, a frontmatter-invocation property, or a static literal in
  the invocation.
- `invocation` is a union: a plain expression string, or a frontmatter block
  (typed `yaml` with coercion — O-A3).
- The existing `today-example.yaml` and `as_unordered_list-example*.yaml` fixtures
  are part of the acceptance corpus, but must be migrated with this spec: remove
  redundant `type` fields from all examples, fix `parameters: null` to either omit
  the field or validate as nullable, and correct the stale prose in the
  `as_unordered_list` fixtures where a native array is described as a CSV string.

## Feature B — `@` cross-file named-type import

`Name@fileref` **inlines the definition** of the named type `Name` from the
referenced file at the use site (structural substitution, not a persistent
reference).

```yaml
type:       type@./types.yaml        # inlines types.yaml's `type` definition
parameters: parameter[]@./types.yaml # array of the inlined `parameter` map
value:      type@this                # the `type` defined in *this* file
```

Worked expansions (the ratified intent):

- `type@./types.yaml`
  → `enum(string,number,boolean,date,time,datetime,any; required)`
- `parameter[]@./types.yaml`
  → `{ "<string>": any }[]` with `min-keys(1)` / `max-keys(1)`

Semantics:

- **Left of `@`** = the named type to inline. **Right of `@`** = a file reference;
  **`this`** = the current file (self-reference).
- A schema file's **top-level `$schema:` entries are its named types.** Each is a
  full SimplifiedSchema definition — scalar, enum, union, or an inline object with
  literal *or* pattern keys. `Name@file` substitutes that definition verbatim.
- **Postfix composes outward.** `Name[]@file` = array-of-(inlined `Name`);
  `Name(constraints)@file` applies constraints to the inlined type. Grammar:
  `type_ref := ident ('[]')? ('(' constraints ')')? '@' fileref`. (Precedence
  pinned in O-B1.)
- **Expansion is eager, bounded, and cycle-checked.** Named types form a DAG; a
  type that (transitively) references itself is a recursion error in v1 (mirrors
  the existing inline-object depth cap). True recursive/reference types are
  deferred.
- File resolution uses standard Darkmatter magic-path rules (`resolve.rs` /
  `biscuit_file::FileReference`), with `this` added as the self-target.
- The parser should represent imported types explicitly until the resolver runs
  (for example `TypeExpr::Imported { name, reference, postfix, constraints }`).
  `to_json_schema` must reject unresolved imports with a conversion error, just as
  it rejects unresolved root-union `FileRef` arms today.
- **Cache invalidation**: each `@`-import is a dependency edge; the base-schema
  LRU cache and DMLS live indexing invalidate a schema when any imported file
  changes (DMLS already content-hashes for this).

Because a named type can be *any* definition and `@` inlines it, SimplifiedSchema
needs **no** separate "type expression" meta-primitive — composition (named types
+ `@` + `[]` + unions + pattern keys) *is* the type system. See O-C2. This is
SimplifiedSchema's ergonomic answer to JSON Schema `$ref`/`$defs`.

## Feature C — pattern / dictionary keys

Type objects whose keys follow a pattern instead of a fixed literal set.

```yaml
parameter:
    "<string>": any                     # any string key → any value (one pair; see arity)
```

Key forms (O-C1 resolved — ergonomic sugar for *literal* prefix/suffix, raw regex
for anything more):

- `<string>` — any string key. → JSON Schema `additionalProperties: <valueType>`.
- `<starting::PREFIX>` — keys beginning with the **literal** string `PREFIX`.
  Desugars to anchored `^PREFIX`.
- `<ending::SUFFIX>` — keys ending with the **literal** string `SUFFIX`. Desugars
  to `SUFFIX$`.
- `<pattern::RE>` — a raw ECMA-262 regex escape hatch for sets/alternation/etc.,
  reusing `Constraint::Pattern` directly (e.g. `<pattern::[0-9_]$>`).

The non-catch-all pattern forms compile to `patternProperties`; `<string>`
compiles to `additionalProperties: <valueType>`. There is **no** custom set
mini-language — the earlier `<ending::[0-9,_]>` sketch becomes
`<pattern::[0-9_]$>`, drawing the line exactly where the shorthand got ambiguous
with real regex char classes.

Rules:

- **Literal keys win** over pattern keys when both could match a key. JSON Schema
  does not provide that precedence by default because `patternProperties` also
  applies to explicitly declared `properties`. The converter must therefore
  subtract literal names from each pattern, by wrapping emitted ECMA-262 patterns
  with a negative lookahead for the literal key set. Example:
  `"<starting::x-">` plus literal key `x-kind` emits a pattern equivalent to
  `^(?!(?:x-kind)$)x-`. If wrapping would make a user-supplied `<pattern::RE>`
  invalid, conversion fails loudly instead of silently double-validating the
  literal property.
- Multiple pattern keys are allowed (maps to multiple `patternProperties`).
- A pattern-keyed object defaults to closed-object semantics:
  `additionalProperties: false` unless it has a `<string>` catch-all, in which
  case the catch-all lowers to `additionalProperties: <valueType>`. This matches
  existing inline-object behavior while making dictionary schemas precise.
- Pattern-keyed objects with a `@`-imported value type sidestep the one-level
  inline-object nesting cap — cross-file named types are the escape hatch for
  depth.

### Object arity (property-count) constraints

Pattern-keyed objects match 0..N keys, but some shapes need a bounded count — a
`parameter` must be **exactly one** key/value pair, yet `{ "<string>": any }`
permits any number. Add object **property-count constraints** — the direct
object-analog of the existing array `min-items(n)` / `max-items(n)`:

- `min-keys(n)` / `max-keys(n)` → JSON Schema `minProperties` / `maxProperties`
  (kebab-case canonical, matching `min-items` / `min-len`).
- **Attach via a reserved `$constraints` metadata key** on the block object — the
  ergonomic alternative to the ugly quoted-inline `{ … }(…)` string, keeping the
  readable block form (and scaling to object arrays). It desugars to the *same*
  canonical `Constraint`s the postfix form produces — **one model, two surfaces**:

  ```yaml
  parameter:
      "<string>": any
      $constraints:
          min-keys: 1
          max-keys: 1
  ```

  `$constraints` (dollar — mirrors the `$schema` convention) is **reserved only
  inside authored schema objects**: it is stripped before shape assembly and never
  participates in literal/pattern key matching. This does not forbid user
  frontmatter data from having a `$constraints` key unless a schema object chooses
  to model that key literally. Flag constraints (`required`, `not-empty`, …) are
  written `name: true`. The inline `{ … }(min-keys(1); max-keys(1))` form remains
  valid and equivalent.

Reserving `$constraints` globally in user data would be an accidental
compatibility break; only the schema-authoring language needs the sentinel.

No `keys(n)` exact-count shorthand — arrays have no `items(n)` sugar either, so
this stays symmetric.

## Feature D — content-format string types (`yaml`, `json`)

Two new primitive types for strings whose **content** must parse as a structured
document:

- `yaml` — a string that is valid YAML.
- `json` — a string that is valid, strict JSON.

```yaml
invocation:
    - string(required)
    - { frontmatter: yaml }     # a frontmatter block expressed as a YAML string
```

These join the existing **string-with-format** family (`date`, `datetime`,
`time`, `email`, `url`, `file`) — not a new mechanism. Design:

- **Compile to** `{ "type": "string", "format": "darkmatter-yaml" | "darkmatter-json" }`,
  registered through the same custom-format seam `file` already uses
  (`format.rs` `DARKMATTER_FILE_*`, wired in `validate.rs`).
- **Validator reuses biscuit-file** — the value is parsed with biscuit-file's YAML
  / JSON parsing (`biscuit_file::{Yaml, Json}` / `DataFormat`); a parse failure is
  a validation error. Near-zero new validation code.
- **String *or* native, with coercion (O-A3).** The value may be a YAML/JSON
  **string** *or* a **native** structure (mapping/sequence/scalar). A native value
  is **coerced** to its YAML/JSON string serialization (the same write-back model
  as existing scalar coercion); coercion fails — a validation error — when the
  native value cannot be represented in the target format. So `frontmatter: yaml`
  accepts both `"title: Foo\ntags: [a, b]"` and a native `{ title: Foo, tags: [a, b] }`.
- **Validation must work without mutating the caller's value.** The implementation
  should reuse the existing schema coercion/normalization path before the
  `jsonschema` validator runs: composing callers can receive the serialized
  write-back, while validation-only APIs validate against a transient coerced copy
  and leave the supplied frontmatter unchanged. A plain `format` validator is not
  enough because Draft JSON Schema formats only see strings.
- **`json` is strict; `yaml` is a superset.** JSON is valid YAML, so `yaml`
  accepts JSON; `json` rejects YAML-only syntax. Two distinct validators.
- **Constraints deferred.** `yaml(schema(<ref>))` / `json(schema(<ref>))` — validate
  the embedded content against a sub-schema — is a natural future extension, out of
  v1 scope.

Driver: `example.yaml`'s `invocation` union `{ frontmatter: yaml }`. Per O-A3 the
`-fm` instance's native mapping is valid via coercion, so the driver stands and
Feature D ships in v1.

## Type domains and the `error` type

Typing the function catalog (single-sourcing, O-A1) and prohibiting function-typed
frontmatter partition the vocabulary into three domains:

- **Data types** — frontmatter property values *and* function parameters. The
  `SimplifiedType` set (`string`, `number`, `boolean`, `date`, `datetime`, `time`,
  `object`, arrays, `file`, `url`, `email`, `yaml`, `json`, `any`). **No `error`,
  no function types.**
- **Return types** — function return positions. Data types **plus `error`**, as a
  union member: a fallible function returns `<success> | error` (mirrors Rust
  `Result<T, error>`), e.g. `as_csv(list: any[]) -> string | error`. Infallible
  functions just return their success type.
- **Function types** — signatures `(params) -> return`. A **catalog-domain**
  concept only.

Rules:

- **Frontmatter properties are data-only.** A schema property may never be typed as
  a function or as `error`; those keywords are not accepted in property-type
  position — a **structural** exclusion, not a runtime check. `SimplifiedType` (the
  frontmatter validator) is untouched by function typing.
- **`error`** is a first-class type but appears only in **return** position (via a
  union). It is the type-system anchor for typed expression errors (the future
  real-errors story).
- The authored `types.yaml` fixture is not a frontmatter schema extension. Its
  `type` enum and `parameter` dictionary are reusable catalog/example helper
  types only; they must not leak `error` or function signatures into
  `SimplifiedType`.

## Existing-code anchors

- `Constraint` enum (`simplified/types.rs`) already has `Pattern(String)` (ECMA
  regex), `Match(Vec<String>)` (globbed file paths), `Eager` — so a file-ref
  `Example` variant and pattern-key regex compilation both have precedent.
- `resolve.rs` already resolves whole-schema file refs via
  `biscuit_file::FileReference::resolve_from(base_dir)` — Feature B reuses this,
  adding named-type lookup within the target and the `this` target.
- Inline objects (`grammar.rs`) parse literal identifier keys into
  `SchemaShape { properties: IndexMap<..> }` with a nesting-depth cap — Feature C
  extends key parsing to `<...>` pattern keys.

## Migration / compatibility notes

- Existing SimplifiedSchema documents without `example()`, `@`, pattern keys,
  `$constraints`, `yaml`, or `json` must parse and compile byte-equivalently.
- The new `yaml` / `json` keywords are intentional grammar additions. Any existing
  schema that used those words as named helper types must rename those helpers;
  primitive keywords stay reserved in type-expression position.
- The schema-plus fixture files under this feature directory are draft artifacts,
  not authoritative runtime docs. Updating them to match O-A1 is in scope for
  implementation and should happen before acceptance tests are added.
- DMLS cache invalidation is documented here only as a required dependency edge;
  DMLS provider behavior remains downstream and out of scope.

## Open decisions

- **O-A1 — Example ↔ target consistency.** ✅ Resolved: **Solution 2 (inherit)** —
  example files omit `type` and parameter type declarations; those derive from the
  target, so drift is impossible. Example instances may still provide
  `parameters` as runtime input values, and those values validate against the
  inherited target signature. **Gated on Option A:** typing the expression-function
  catalog (return type incl. `error`, param types), tracked in single-sourcing,
  since functions have no declared types today. ctx-variable examples inherit from
  the base schema; function examples inherit from the typed catalog. **Follow-up:**
  `example.yaml` drops the `type` field and any parameter-type declarations;
  `types.yaml`'s `type` enum / `parameter` are repurposed as the
  function-signature vocabulary. File edits deferred until the function catalog is
  typed.
- **O-A2 — When are example files validated?** ✅ Resolved: **at schema load**
  (eager, fail-loud), in the same resolution pass as `$schema` file refs, with
  content-hash caching so warm reloads skip unchanged files. Lazy validation
  rejected — it lets malformed examples lurk until viewed.
- **O-A3 — Frontmatter-invocation shape.** ✅ Resolved: keep `{ frontmatter: yaml }`.
  The `yaml` type accepts **either** a YAML string **or** a native YAML structure,
  **coercing** native → its YAML-string serialization (like existing scalar
  coercion), and errors if the native value cannot be represented as YAML. So the
  `-fm` instance's native mapping is valid *and* stays authorable as a string.
  Keeps Feature D (no longer deferred) with a real driver.
- **O-A4 — What `parameters` holds.** ✅ Resolved: `parameters` bind a variable's
  **initial value** (option A). `parameter` is `{ "<string>": any }` (value type
  `any`, holding native arrays *or* strings) with exact one-key arity
  (`min-keys(1)` + `max-keys(1)`). The scalar `type` enum is used only for the
  return `type`. Example input data has three sources: `parameters`, a
  frontmatter-invocation property, or a static literal in the invocation
  (`as_unordered_list("1,2,3")` — best for string params).
- **O-C3 — Object arity + block-form constraints.** ✅ Resolved: add
  `min-keys(n)` / `max-keys(n)` (object-analog of `min-items` / `max-items` →
  `minProperties` / `maxProperties`), authored via a reserved **`$constraints`**
  metadata key on schema block objects (desugars to the canonical `Constraint`s;
  excluded from key matching; flags as `name: true`). The inline `{ … }(…)` form
  stays equivalent. `parameter` = `{ "<string>": any }` + `$constraints: { min-keys:
  1, max-keys: 1 }`. `$constraints` is reserved in schema-authoring position only,
  not as a global user-data key.
- **O-C4 — `$as_array` block-form array marker.** ⏸️ Deferred. A `$as_array: true`
  key to wrap a block object in an array. Held off: cardinality already belongs at
  the *use site* via the `[]` postfix (`parameter[]`), baking it into a named type
  couples cardinality to the type, and `$as_array` + `$constraints` is ambiguous
  (element- vs array-level). If a block-form *inline* object-array need appears,
  prefer a `$array:` *scope* (presence ⇒ array; body carries array constraints)
  over a boolean with a dead `false` no-op.
- **O-B1 — `@` grammar precedence.** ✅ Resolved (Solution A): postfix `[]`/`()`
  bind to the type name; `@fileref` is the **terminal** suffix (runs to the end of
  the type expression — unambiguous, no filename-delimiter problem). Grammar:
  `type_ref := base ('[]')? ('(' constraints ')')? '@' fileref`. `@` may appear in
  any type-expression position (union arm, inline-object value, array element),
  always terminal within that sub-expression.
- **O-B2 — What is importable.** ✅ Resolved (Solution 1): `Name@file` resolves only
  among a file's **top-level `$schema:` named types** (flat per-file namespace;
  mirrors JSON Schema `$defs`/`$ref`). Reusing a nested shape means promoting it to
  a named top-level type — intentional reuse, stable against internal reorg. `@this`
  covers intra-file composition. No dotted-path imports.
- **O-B3 — Recursive/self types.** ✅ Resolved (Feature B): named types form a DAG;
  a self-referential type is a recursion error in v1 (bounded expansion, mirrors
  the inline-object depth cap). `this` is allowed for non-recursive cross-references
  within a file. True recursive/reference types deferred.
- **O-C1 — Pattern-key matching.** ✅ Resolved (Solution 3): `<string>` (any),
  literal-only `<starting::PREFIX>` / `<ending::SUFFIX>` sugar (desugars to anchored
  `^PREFIX` / `SUFFIX$`), and a raw-regex `<pattern::RE>` escape hatch for
  sets/alternation (reuses `Constraint::Pattern`). No custom set mini-language;
  non-catch-all patterns lower to `patternProperties`, while `<string>` lowers to
  `additionalProperties`. `<ending::[0-9,_]>` → `<pattern::[0-9_]$>`.
- **O-C2 — Type vocabulary.** ✅ Resolved. `@`-import is **structural inline
  expansion** (Feature B); named types are ordinary definitions, so **no**
  recursive `type-expr` meta-primitive is introduced. The scalar enum `type` is
  used **only for the top-level return `type` field**. `parameters` bind initial
  **values** (O-A4), so `parameter` is `{ "<string>": any }` — the `any[]` concern
  is moot (values are `any`). A `type-expr` string primitive remains a deferred
  escape hatch if precise param typing is ever wanted — out of v1 scope.
- **O-D1 — Scope split.** ✅ Resolved: schema-plus is its own feature; both
  single-sourcing and modal-and-autocomplete depend on it (confirmed when the specs
  were split).
- **O-D2 — Content-format family breadth.** ✅ Resolved (Solution 3): ship `yaml` +
  `json` now (the formats with concrete drivers); `toml` / `json5` are
  **pre-blessed fast-follows** added when a driver appears. The format seam is
  general, but their coercion nuances (TOML can't represent `null` / top-level
  arrays; JSON5 is input-only) are best designed against a real use case.

## Acceptance criteria

1. `example(<file>, …)` parses as a `Constraint`, resolves via magic paths +
   `this`, validates each example against the corrected `example.yaml` envelope at
   load, and validates a target-typed `returns` value against the annotated
   property's compiled type. Target-specific `parameters` validate only against
   the generic `parameter[]` shape (array of single-key maps, O-A4); validating
   them against a **typed expression-function signature** (param names, arity,
   per-param types) is **deferred to single-sourcing D7** per O-A1 — the function
   catalog is untyped today, so there is no signature to inherit. When that
   catalog is typed, this criterion extends to signature-aware parameter checks.
2. `Name@fileref` and `Name@this` resolve named types across files, with cycle
   detection, unresolved-import conversion errors, and cache invalidation on
   referenced-file change.
3. `<string>`, `<starting::…>`, `<ending::…>` key patterns parse and compile to
   `additionalProperties`/`patternProperties`; literal keys take precedence in the
   emitted JSON Schema instead of being double-validated.
4. `@`-import inline-expands named types (incl. `[]`/constraint postfix) with no
   `type-expr` meta-primitive; array-valued params are expressible as `any`
   parameter values (O-C2/O-A4).
5. `example.yaml` (corrected) validates `today-example.yaml` and all
   `as_unordered_list-example*.yaml` fixtures after removing inherited `type`
   fields and fixing stale fixture prose.
6. single-sourcing can express context-variable examples entirely as referenced
   YAML files (no Rust example sidecar).
7. `yaml` and `json` types parse, compile to `format: darkmatter-yaml|darkmatter-json`,
   validate via biscuit-file (`json` strict; `yaml` accepts JSON), and accept
   native YAML values through transient coercion without mutating validation-only
   callers. The `invocation` union in `example.yaml` validates.
8. Builds and passes on macOS, Windows, Linux.

## Out of scope

- Any DMLS or catalog change (those are downstream specs).
- General JSON Schema `$ref` compatibility beyond what `@` import needs.
