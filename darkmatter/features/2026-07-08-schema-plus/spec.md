---
status: spike
review_iterations: 0
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

**Status:** Spike — designing three additions that turn SimplifiedSchema from a
flat single-file validator into a **composable type system**. Consumed first by
[single-sourcing-schema](../2026-07-08-single-sourcing-schema/spec.md), which uses
them to model context-variable examples as validated YAML artifacts (its "E3"
option) instead of a Rust sidecar.

The three features are independent in grammar but cohesive in intent:

- **A — `example()` constraint**: attach one or more example YAML files to a
  property.
- **B — `@` cross-file named-type import** (+ `this`): reference a named type
  defined in another schema file.
- **C — pattern/dictionary keys**: type objects whose keys follow a pattern
  rather than a fixed literal set.

## Why (single-sourcing's E3)

single-sourcing needs a home for context-variable examples that (a) is not the
Rust catalog and (b) does not bloat the validation grammar. The answer: examples
are **their own schema-validated YAML files**, referenced from a property via an
`example()` constraint. This keeps the base schema pure, makes examples
first-class data validated by SimplifiedSchema itself, and needs the two general
primitives (B, C) to express the example/parameter schemas cleanly.

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
- **Each referenced file is itself validated against `example.yaml`** at
  schema-load time; a malformed example is a schema error (fail loud — the point
  is trustworthy examples).
- Resolution: `biscuit_file::FileReference` + magic paths, relative to the
  **referencing schema file** (mirrors `$schema` file-ref resolution in
  `resolve.rs`), plus the new `this` (Feature B).
- JSON Schema mapping: `x-darkmatter-example` carrying the resolved example
  objects, so downstream tools (`md schema about`, DMLS hover) read them without
  re-reading disk.

### The example artifact schema (`example.yaml`)

The authored schema (matching the current file):

```yaml
$schema:
    kind: enum(example; required)
    type: type@./types.yaml
    parameters: parameter[]@./types.yaml
    invocation:
        - string(required)
        - { frontmatter: yaml }
    returns: any
    description: string
```

Notes:

- `type` (the **return type**) imports the scalar enum
  `enum(string,number,boolean,date,time,datetime,any; required)` from `types.yaml`.
  This is the enum's *only* use site.
- `parameters` is an **array** of single-key maps, each binding a variable
  **name → its initial value** (O-A4), so `parameter` is `{ "<string>": any }` with
  `max-keys(1)` arity (Feature C). The `any` value holds both a native array
  (`[1,2,3]`) and a list-string (`"- 1\n- 2\n- 3"`) — the two representations
  `as_unordered_list-example{,-2}.yaml` show. Example input data has three sources:
  `parameters`, a frontmatter-invocation property, or a static literal in the
  invocation.
- `invocation` is a union: a plain expression string, or a frontmatter block
  (typed `yaml` with coercion — O-A3).
- The three `as_unordered_list-example*.yaml` files are the acceptance corpus
  (Acceptance 5).

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
  → `{ "<string>": enum(string,number,boolean,date,time,datetime,any; required) }[]`

Semantics:

- **Left of `@`** = the named type to inline. **Right of `@`** = a file reference;
  **`this`** = the current file (self-reference).
- A schema file's **top-level `$schema:` entries are its named types.** Each is a
  full SimplifiedSchema definition — scalar, enum, union, or an inline object with
  literal *or* pattern keys. `Name@file` substitutes that definition verbatim.
- **Postfix composes outward.** `Name[]@file` = array-of-(inlined `Name`);
  `Name(constraints)@file` applies constraints to the inlined type. Grammar:
  `type_ref := ident ('[]')? '@' fileref`. (Precedence pinned in O-B1.)
- **Expansion is eager, bounded, and cycle-checked.** Named types form a DAG; a
  type that (transitively) references itself is a recursion error in v1 (mirrors
  the existing inline-object depth cap). True recursive/reference types are
  deferred.
- File resolution uses standard Darkmatter magic-path rules (`resolve.rs` /
  `biscuit_file::FileReference`), with `this` added as the self-target.
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

Key forms:

- `<string>` — any string key. → JSON Schema `additionalProperties: <valueType>`.
- `<starting::PREFIX>` — keys beginning with `PREFIX`.
- `<ending::SET>` — keys ending with a char in `SET`.

→ `<starting::>`/`<ending::>` compile to anchored `patternProperties` regexes.
The `SET` mini-syntax (`[0-9,_]` = "a digit or `_`") is comma-separated members
with ranges (`0-9`) and literals (`_`), compiled to an anchored regex char class
(`[0-9_]$`). See O-C1 — we may instead accept raw ECMA-262 patterns, since
`Constraint::Pattern` already exists.

Rules:

- **Literal keys win** over pattern keys when both could match a key.
- Multiple pattern keys are allowed (maps to multiple `patternProperties`).
- Pattern-keyed objects with a `@`-imported value type sidestep the one-level
  inline-object nesting cap — cross-file named types are the escape hatch for
  depth.

### Object arity (property-count) constraints

Pattern-keyed objects match 0..N keys, but some shapes need a bounded count — a
`parameter` must be **exactly one** key/value pair, yet `{ "<string>": any }`
currently permits any number of keys. Add object **property-count constraints**:

- `min-keys(n)` / `max-keys(n)` → JSON Schema `minProperties` / `maxProperties`.
- `parameter` becomes an exactly-one-pair map: `{ "<string>": any }` with
  `min-keys(1); max-keys(1)`.

**O-C3** (open): how a constraint attaches to a *block-form* object value in YAML.
The inline `{ … }(constraints)` postfix already exists in the grammar, but authors
write `parameter` as a nested block mapping. Options: require the inline form for
constrained objects, a dedicated count marker on the pattern key, or a metadata key.

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
- **`json` is strict; `yaml` is a superset.** JSON is valid YAML, so `yaml`
  accepts JSON; `json` rejects YAML-only syntax. Two distinct validators.
- **Constraints deferred.** `yaml(schema(<ref>))` / `json(schema(<ref>))` — validate
  the embedded content against a sub-schema — is a natural future extension, out of
  v1 scope.

Driver: `example.yaml`'s `invocation` union `{ frontmatter: yaml }`. Per O-A3 the
`-fm` instance's native mapping is valid via coercion, so the driver stands and
Feature D ships in v1.

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

## Open decisions

- **O-A1 — Example ↔ target consistency.** An example declares `type`/`parameters`
  that duplicate the real signature of the annotated variable/function
  (`today-example` says `type: date`; the schema already says `today: date`).
  Options: (a) example omits type/params and inherits from the target; (b)
  example declares them and they are **cross-checked** against the target at
  load (drift guard); (c) purely documentary. **Leaning:** (b) — cheap drift
  protection, keeps examples self-contained.
- **O-A2 — When are example files validated?** At schema load (fail-loud) vs.
  lazily on first read. **Leaning:** at load, behind the same resolution pass as
  `$schema` refs.
- **O-A3 — Frontmatter-invocation shape.** ✅ Resolved: keep `{ frontmatter: yaml }`.
  The `yaml` type accepts **either** a YAML string **or** a native YAML structure,
  **coercing** native → its YAML-string serialization (like existing scalar
  coercion), and errors if the native value cannot be represented as YAML. So the
  `-fm` instance's native mapping is valid *and* stays authorable as a string.
  Keeps Feature D (no longer deferred) with a real driver.
- **O-A4 — What `parameters` holds.** ✅ Resolved: `parameters` bind a variable's
  **initial value** (option A). `parameter` is `{ "<string>": any }` (value type
  `any`, holding native arrays *or* strings) with `max-keys(1)` arity. The scalar
  `type` enum is used only for the return `type`. Example input data has three
  sources: `parameters`, a frontmatter-invocation property, or a static literal in
  the invocation (`as_unordered_list("1,2,3")` — best for string params).
- **O-C3 — Arity constraint attach-syntax.** How `min-keys`/`max-keys` attach to a
  block-form object value (Feature C § Object arity). Property-count constraints
  recommended; exact author-facing surface open.
- **O-B1 — `@` grammar precedence** of `[]`/`()` vs `@`
  (`(parameter[])@file` vs `parameter@(file...)`) and whether `@` is allowed in
  arbitrary type positions or only top-level property types.
- **O-B2 — What is importable.** Only top-level `$schema:` entries, or nested
  named types too? Recommend top-level only for v1.
- **O-B3 — Recursive/self types.** ✅ Resolved (Feature B): named types form a DAG;
  a self-referential type is a recursion error in v1 (bounded expansion, mirrors
  the inline-object depth cap). `this` is allowed for non-recursive cross-references
  within a file. True recursive/reference types deferred.
- **O-C1 — Pattern mini-syntax vs raw regex.** Adopt the `[0-9,_]` set shorthand,
  or accept ECMA-262 patterns directly (reusing `Constraint::Pattern`), or both.
- **O-C2 — Type vocabulary.** ✅ Resolved. `@`-import is **structural inline
  expansion** (Feature B); named types are ordinary definitions, so **no**
  recursive `type-expr` meta-primitive is introduced. The scalar enum `type` is
  used **only for the top-level return `type` field**. `parameters` bind initial
  **values** (O-A4), so `parameter` is `{ "<string>": any }` — the `any[]` concern
  is moot (values are `any`). A `type-expr` string primitive remains a deferred
  escape hatch if precise param typing is ever wanted — out of v1 scope.
- **O-D1 — Scope split.** Confirm this is its own feature with single-sourcing
  depending on it (recommended), vs. folded into single-sourcing.
- **O-D2 — Content-format family breadth (Feature D).** Ship `yaml` + `json` only
  (concrete driver), or the full biscuit-file `DataFormat` set (`toml`, `json5`)
  for symmetry? **Leaning:** `yaml` + `json` now; others on demand (each is a
  one-line format registration).

## Acceptance criteria (draft)

1. `example(<file>, …)` parses as a `Constraint`, resolves via magic paths +
   `this`, and validates each target against `example.yaml` at load.
2. `Name@fileref` and `Name@this` resolve named types across files, with cycle
   detection and cache invalidation on referenced-file change.
3. `<string>`, `<starting::…>`, `<ending::…>` key patterns parse and compile to
   `additionalProperties`/`patternProperties`; literal keys take precedence.
4. `@`-import inline-expands named types (incl. `[]`/constraint postfix) with no
   `type-expr` meta-primitive; array-valued params are expressible via enum
   array-members (O-C2).
5. `example.yaml` (corrected) validates both provided example instances.
6. single-sourcing can express context-variable examples entirely as referenced
   YAML files (no Rust example sidecar).
7. `yaml` and `json` types parse, compile to `format: darkmatter-yaml|darkmatter-json`,
   and validate via biscuit-file (`json` strict; `yaml` accepts JSON). The
   `invocation` union in `example.yaml` validates.
8. Builds and passes on macOS, Windows, Linux.

## Out of scope

- Any DMLS or catalog change (those are downstream specs).
- General JSON Schema `$ref` compatibility beyond what `@` import needs.
