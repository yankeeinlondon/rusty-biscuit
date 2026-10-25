---
area: darkmatter
status: finalized-spec
created: 2026-10-10
owner: Ken Snyder <ken@ken.net>
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
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-11
review_iterations: 1
completed: true
implemented: true
implemented_by: claude/opus
human_review: false
message_to_agent: |-
    All six phases are implemented; Phase 6 was verification only and
    changed no source, test, doc, or skill file. Read "## Phase 6" in
    implementation-log.md for the final acceptance-to-test map (AC1-AC10).
    Gates green on macOS: darkmatter `just test` 9299 passed / 11 skipped and
    `just lint` exit 0; claudine `just test` 8682 passed / 9 skipped and
    `just lint` exit 0 (only the unrelated macOS `__eh_frame` linker note);
    messenger `just test` 710 passed / 2 skipped. Linux cross-checks passed
    for every feature test file in darkmatter, darkmatter-cli, claudine,
    and claudine-cli; native Windows evidence is from Phase 5 on the same
    tree; WSL2 was not run locally. A reviewer should know: (1) 12
    messenger tests exceed the 5 s L1 budget, but the same tests take the
    same time on the pre-feature commit f1d031f7a, so this is not a
    regression of this feature; (2) the non-blocking observations from
    Phase 5 (conflict messages omit bound values, standalone YAML parse
    errors lose their location, `Markdown::from(String)` drops unparsable
    frontmatter, exponential spike-fixture resolution) remain open and
    unchanged; (3) an inline root union resolves `this` per arm, which is
    the behavior from before this feature and is documented as such (doc
    drift, not a spec departure). Lifecycle movement, commit, and push are
    left to the author.
pipeline_step: 3
canonical_docs:
    - darkmatter/docs/topics/schemas/local-type-references.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/inline/schema-validation.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/completions/shell-completions.md
related:
    - 2026-10-14-union-type-constraints
    - 2026-10-16-core-schema-types
    - 2026-10-17-content-policy-base-schema
---

# Named Types Keep Their Constraints; Local Types by Bare Name

SimplifiedSchema lets a schema define reusable **named types** and reference
them from properties. Two things make that harder than it should be today:

1. When a named type is reused, the resolver silently **strips** its
   top-level `required` and `generated` constraints, so `types: { req:
   expression(required) }` used as `when: req@this` makes `when` optional.
   Authors have to repeat `(required)` at every use site, and forgetting it is
   invisible.
2. A type defined in the same file must be referenced as `name@this`; a bare
   `name` is an "unknown type" error.

After this change a named type carries **all** of its constraints wherever it
is used, a same-file type is referenced by its bare name, and use-site
constraints merge with inherited ones under one predictable rule.

This feature belongs to the Darkmatter package area: its library owns schema
parsing and resolution, and its DMLS language server provides editor support.
Claudine consumes the resolved schemas and file-completion information.

```yaml
kind: schema
types:
    person: "{ name: string(required), email: email }"
    code: string(required; generated)
    err: "{ code: code, author: person, reviewer: person@./people.yaml }"
    # `code` and `person` are same-file types referenced bare (no @this);
    # err.code inherits required and generated; @ is for other files.
```

## Decided

### 1. Presence constraints survive every reference

Decided 2026-10-10 (confirming the owner ruling of 2026-09-19): stripping
`required` and `generated` from a reused named type was an oversight, not a
design decision. Every constraint declared on a named type, including
`required` and `generated`, survives every reference to it: bare local
references, `@this`, cross-file `@./file.yaml` imports, and multi-hop chains
where one named type references another.

An inherited constraint retains its original scope after substitution.
For example, `required` inherited onto `people: person[]` marks `people`
required, the same as writing `person(required)[]` does today, and a
`required` inherited into one arm of a property-level union is hoisted to the
property as `required` on any arm already is.

For example, a named `string(min(3))` used as `label[]` requires each item
to have at least three characters; `label[](min(1))` additionally requires
at least one item. The two minima never merge. Reusing a type that is already
an array preserves its item and container constraints; adding another `[]`
remains an error. Merely requiring `name` inside the object definition of
`person` does **not** require the enclosing `person` property. Presence is
inherited only when declared on the reused value itself.

### 2. No use-site opt-out

There is no syntax to remove an inherited `required` (or `generated`) at a use
site. An omitted postfix never resets the inherited definition. An author who
needs the whole value to be optional will be able to write a union with
`null` after the core-schema-types feature lands:

```yaml
types:
    draft_owner: [person, "null"] # planned: null or omitted, else a person
```

A variant whose inner field is optional is a different type and is declared as
one. The `null` type and the rule that a union containing a `null` arm is never
required land in `2026-10-16-core-schema-types`; the compact
`person | null` spelling lands in `2026-10-28-schema-enhancements`. This spec
does not introduce nullable unions. Until then, declare a separate variant
without top-level `required` or `generated` rather than relying on stripping.
That variant must define its optional value directly: aliasing the required
type would inherit the same presence constraints again. Inner required fields
may remain required when the optional object is supplied.

### 3. Bare local names

A type defined in the same file is referenced by its bare name: `author:
person`. The meaning is identical to `person@this`, which remains supported.
`@` is for types from other files. A bare reference survives renaming the
file, which `@./file.yaml` self-references do not.

Resolution rules:

- **Where local types live.** A bare name resolves in exactly the namespace
  `@this` resolves in today: the named types of the schema where the
  reference is written (the `types` table of a `kind: schema` file; the
  top-level `$schema` entries of a document or plain schema file).
- **Built-in keywords win.** A bare name that is a built-in type keyword
  (`string`, `email`, `file`, …) means the built-in. `string@this` selects a
  local type named `string`.
- **Names retain the existing grammar.** Bare names are case-sensitive and
  use the same identifier characters as explicit named imports. This feature
  does not add quoted names or new punctuation. A misspelled built-in now
  parses as a reference and fails during resolution unless it names a local
  type; the error must still point to the authored name.
- **Unknown names are errors.** A bare name that is neither a built-in nor a
  local type is a typed resolution error naming the type and the schema file.
  It is never treated as `any`, never looked up in other active schemas,
  frontmatter, or runtime globals.
- **Resolution stays in the defining file.** If `address@./contacts.yaml`
  uses a bare `postcode`, that `postcode` comes from `contacts.yaml`, not from
  the importing document.
- **Same forms everywhere.** A bare reference accepts every postfix an
  explicit reference does: `person[]`, `person(required)`, `person[](min(1))`,
  `-> description`, and use inside union arms and inline objects. Forward
  references see the complete local table. These forms remain subject to
  the existing target restrictions: postfixes on a union-typed named type
  remain unsupported until `2026-10-14-union-type-constraints`. A plain
  reference to a union, or use of that reference as a union arm, is supported
  here and preserves inherited presence. Cycles and the existing expansion
  depth limit remain enforced, using `SchemaError::ImportCycle`.
- **No new namespace in a root union.** Inline documents whose `$schema`
  is a root union retain today's empty `@this` namespace; entries from
  separate arms are not combined into a local type table. Whole-file
  reference schemas and raw JSON Schema do not gain named exports.
- **Descriptions.** For supported non-union targets, a use-site description
  replaces the inherited description; omission preserves it. Descriptions
  on union references retain the current rejection until separately supported.

### 4. Constraint merging

When a use site adds constraints to a named type, inherited and use-site
constraints combine. This is a merge within one reused type; it does not
change schema layering, where the winning tier's whole property definition
replaces lower tiers.

| Situation | Result | Example |
|---|---|---|
| Different constraints | Both apply | `label: string(min(3))`, use `label(max(80))` → 3..80 characters |
| Repeated lower bound | The larger (stricter) wins | `min(3)` + `min(5)` → `min(5)` |
| Repeated upper bound | The smaller (stricter) wins | `max(80)` + `max(50)` → `max(50)` |
| Contradictory bounds | Structured schema error | `min(10)` + `max(5)` |
| Flags (`not-empty`, `integer`, `unique`, `eager`, `required`, `generated`) | Idempotent | `required` twice = `required` |
| Repeated `pattern(…)` | All apply (a value must satisfy each) | `pattern(^a)` + `pattern(z$)` requires both |
| Repeated `match(…)` | Intersection: each group keeps its own positive/exclusion list, and a path must satisfy every group | `file(match('*.md'))` used as `doc(match('guide*'))` offers only Markdown files whose names start with `guide` |
| Repeated `scheme(…)` | Intersection; an empty intersection is a contradiction error | `scheme(http,https)` + `scheme(https)` → `scheme(https)` |
| Repeated `default(…)` | Equal values are fine; different values are a contradiction error | |
| Repeated `suggest(…)` | Error, as today (at most one `suggest`) | |
| Repeated `example(…)` | Preserve all artifacts in declaration order, inherited first | Keep each reference's defining-file origin |

Bounds are compared per kind: string length `min`/`max`, number `min`/`max`,
array `min`/`max` (item counts), and `min-keys`/`max-keys`. Array-element
constraints and array-container constraints keep their separate meanings and
merge only with their own kind.

Equal defaults use the converter's existing normalized JSON equality: `3`
and `3.0` compare equal, while `"3"` and `3` differ. This feature does not
introduce a new requirement that every default satisfy all merged constraints.
Existing type-specific default checks remain in force. Enum members and literal
values belong to the named type's definition; a use site cannot replace them
or change the target type. Unsupported constraints are rejected after resolving
the target, just as for explicit imports. Suggestion values are interpreted
using the resolved target type and checked by the existing suggestion lint;
they do not become validation constraints. The one-`suggest` limit applies
across the complete property, including flattened union arms.

**Reader's note:** Keeping two constraints in the parsed representation is
not sufficient if conversion subsequently overwrites a JSON Schema keyword.
Multiple patterns, including the pattern used by `not-empty`, must survive
lowering as conjunctions (for example, separate `allOf` entries), while keeping
the base type and Darkmatter annotations available to validation and editor
consumers. Do not combine regular expressions into a new expression or try to
prove their intersection nonempty. Contradiction detection here is limited to
the listed bounds, empty scheme intersections, and unequal defaults; it is
not a general solver for unsatisfiable schemas. Equal bounds are valid.

**Repeated `match` groups.** Today the Darkmatter library's
[file-match model](../../../darkmatter/lib/src/markdown/schemas/file_match.rs)
exposes one list of patterns: a path must match at least one positive pattern
and none of the exclusions. Claudine uses that list to offer files. In a single
schema these patterns do not reject document values; in a root union they help
select and validate an arm for an existing file. Repeated `match(...)`
constraints on a reused file type combine as an intersection of groups:

- Each `match(...)` keeps its own group, with the existing
  positive/exclusion logic inside that group. Groups are not concatenated,
  because a combined list would widen the positive choices (the example above
  would offer every Markdown file plus every `guide*` file).
- A path satisfies the constraint only when it satisfies every group.
  Completion offers only candidates that satisfy all groups.
- The conjunction is enforced only where `match` already affects validation
  (root-union arm selection and validation). In ordinary single-schema
  validation `match` stays passive and rejects no document value.
- This needs a grouped-pattern representation in place of the single list.
  Darkmatter lowering, root-union selection, and Claudine's completion
  consumers are updated to read it; those changes are in scope here.
- An empty intersection is not proven at schema-parse time: proving it would
  require searching files, and schema parsing never does. Groups whose
  intersection is empty are accepted; completion simply offers no candidates.

A contradiction error is a structured `SchemaError` that names the property,
the conflicting constraints, and where each came from (the named type's
definition and the use site), so the Darkmatter language server (DMLS) can point
at both. Retain typed source locations, not just paths embedded in a message:
the property path, each source file (or inline document identity), and each
authored constraint span. Multi-hop conflicts point to the declarations that
introduced the conflicting values. DMLS reports the consuming reference and
includes related locations for those declarations. Preserve the existing typed
error wrappers when a conflict originates in a referenced schema.

Preserving `generated` does not invent a value provider and does not change
its validation-phase semantics: authored-document validation permits an absent
generated property even when required; effective/runtime validation enforces
required presence and checks supplied values under the existing phase rules.

## Requirements

- In the Darkmatter library's
  [named-type resolver](../../../darkmatter/lib/src/markdown/schemas/resolve.rs),
  delete `strip_top_level_presence`, which currently removes inherited
  presence, and its call in `expand_import`. Update `apply_import_postfix`,
  which attaches use-site constraints, to enforce the merge rules in both
  value and array-container constraints. Check object bounds declared through
  `$constraints` as well as inline object postfixes; the two spellings must
  not bypass contradiction detection. Preserve inherited array constraints
  instead of replacing them. Repeated `match(...)` groups are preserved as
  separate groups (Decided 4) rather than concatenated or overwritten.
- Replace the single pattern list in the file-match model with the
  grouped-pattern representation and update every consumer: Darkmatter
  lowering, root-union selection, and Claudine's file completion. Schema
  parsing performs no file search to detect empty intersections.
- The type-expression parser keeps a bare non-keyword identifier, with its
  authored span, as a local reference; the resolver expands it through the
  same named-import path as `@this`. Bare and explicit references must agree
  for scalar, array, union and constrained types, forward references, missing
  types, cyclic imports, descriptions, and defining-file origins.
- Preserve the existing explicit-import spellings and terminal `@` grammar.
  Constraint applicability and interpretation depend on the resolved target:
  `label(min(5))` uses string length when `label` resolves to a string, and
  numeric bounds when it resolves to a number. Parsing must defer that choice
  for bare references exactly as it does for explicit imports.
- The Darkmatter library's
  [type-expression serializer](../../../darkmatter/lib/src/markdown/schemas/simplified/serialize.rs)
  must round-trip local references without changing their meaning. Canonical
  serialization uses bare names for local references unless a built-in
  collision requires `@this`; cross-file references retain their target.
  Source edits use the existing source-aware facilities rather than rewriting
  whole documents for this spelling change.
- Runtime resolution, validation, generated JSON Schema, and DMLS use the same
  parser and resolver. DMLS completion offers same-file names bare (today it
  offers `Name@this`), and hover / go-to-definition work on bare references.
- DMLS offers a colliding local name as `string@this`, not as bare `string`.
  Unknown names and contradictions are schema-definition errors rather than
  document-value errors. Cursor scanning, reference discovery, suggestion
  checking, and import detection must recognize bare references, including
  files containing no explicit `@` import. Imported definitions retain their
  dependency edges so edits to their files invalidate existing cached results
  and refresh editor assistance. Local references add no external dependency.
- Keep parsing and semantic meta-type checks passive: accepting a bare name
  syntactically does not require a filesystem lookup or expression execution.
  Named imports are expanded only in the existing request-scoped resolution
  stage. Reuse its captured context, file policy, namespace cache, and depth
  limits; do not add schema-root walks for local names.
- **Test-suite audit.** Remove or update every test and comment that requires
  presence constraints to be stripped on reuse.
- **Shipped-schema audit.** Find every shipped schema (under `*/schemas/`,
  `*/docs/schemas/`, the built-in baseline, and inline `$schema` blocks in
  shipped documents) whose named types carry `required` or `generated` and are
  referenced elsewhere. For each, confirm the inherited presence is intended;
  where a reference relied on stripping to be optional, express that
  optionality explicitly in the schema (an optional variant type or, once
  `2026-10-16-core-schema-types` lands, a `null` union). Record the audit
  result (files checked, changes made) in the implementation log.
- **Compatibility audit.** This intentionally changes resolved presence and
  duplicate-constraint behavior. Audit repeated constraints and built-in/local
  name collisions in shipped definitions, including inline document schemas.
  Compare the parsed result, lowered JSON Schema, completion metadata, and
  authored/effective validation outcomes. Update consumers that assume a
  single pattern keyword. Do not mass-rewrite valid `@this` references merely
  to adopt the shorthand. Add a passive shipped-artifact corpus check and an
  end-to-end `md schema validate` fixture through the normal invocation path.
- **Docs.** In `darkmatter/docs/topics/schemas/local-type-references.md` and
  `authoring-schemas.md`, remove the "Implementation status" notes saying bare
  names are unsupported and stripping still happens, and correct the length
  constraint spelling in the examples to the grammar's actual `min(n)` /
  `max(n)` (the docs currently write `min-length` / `max-length`, which the
  grammar does not accept). In `definition.md`, update the Cross-File
  Named-Type Imports section to say constraints merge (with the table above)
  and that same-file types may be referenced bare. Update
  `dmls-schema-support.md` where it says names are offered as `Name@this`.
  Update the `darkmatter` skill's schema notes.
  Document serialization, description precedence, the distinction between
  optional outer objects and required inner fields, generated validation
  phases, and the repeated-`match` intersection behavior. In current topic pages,
  mark nullable-union syntax as planned until implemented rather than presenting
  it as an immediately available workaround. Update the Darkmatter README's
  schema summary if it describes the affected public behavior.

## Out of scope

- `[]`, list constraints, and `required` on a reference to a **union-typed**
  named type: `2026-10-14-union-type-constraints`.
- The `null` type and union-with-`null` optionality:
  `2026-10-16-core-schema-types`.
- The `|` union spelling: `2026-10-28-schema-enhancements`.

## Acceptance criteria

1. Given `types: { code: string(required; generated) }`, the properties
   `a: code`, `b: code@this`, and `c: code@./types.yaml` (a fixture file
   exporting the same definition) all resolve to required-and-generated
   properties, with equivalent property fragments and presence annotations
   after accounting for the different property names. Verify authored and
   effective validation separately, including supplied invalid values.
2. The spike case: `types: { req: expression(required) }` used as
   `when: req` inside a mapping type, and as a top-level property, makes the
   property required in both positions.
3. Coverage: required-only, generated-only, and combined constraints;
   optional (unconstrained) definitions; multi-hop local and cross-file
   chains; nested objects, arrays (`person[]` inherits `required` onto the
   property), and union arms.
4. Merge: additional use-site constraints apply; repeated bounds keep the
   stricter value for string length, number, array item count, and
   `min-keys`/`max-keys`; `min(10)` + `max(5)` yields a structured error naming
   both origins; repeated `pattern` requires both; `scheme` intersects;
   conflicting `default`s error; equal bounds succeed. Distinct patterns and
   `not-empty` remain conjunctive in lowered schemas and actual validation.
   Repeated examples retain their source-relative references; duplicate
   suggestions fail even when inherited through different union arms.
5. Bare names: a bare local reference and its `@this` form produce identical
   resolved schemas for every form in Decided 3; a bare `string` with a local
   type named `string` resolves to the built-in while `string@this` selects
   the local type; an unknown bare name yields the typed resolution error
   naming the type and file; a bare name inside an imported definition
   resolves in its defining file.
6. DMLS: completion offers a same-file type bare; hover on a bare reference
   shows the local definition; a contradiction error is reported as a schema
   diagnostic with related source locations. Go-to-definition selects the
   correct defining file, colliding names complete with `@this`, and editing
   an imported definition refreshes assistance. Missing-name diagnostics point
   to the reference; cycles report the chain for mixed bare/explicit imports.
7. The test-suite and shipped-schema audits are done and recorded; no test
   still asserts stripping.
8. `just test` and `just lint` pass in `darkmatter/`, plus `just test` in any
   package area whose shipped schema the audit changed. Tests are L1, use no
   terminal or browser windows, and are portable to macOS, Linux, Windows, and
   WSL2. No new CI cells.
9. Bare/explicit reference serialization round-trips; both reference forms
   preserve descriptions and target-directed bound interpretation. Shipped
   schemas parse passively, and an L1 `md schema validate` fixture verifies
   inherited presence and constraint conjunction through normal resolution.
10. Existing root-union and already-array target restrictions remain intact;
    unmodified schemas with neither inherited presence nor repeated constraints
    retain equivalent lowered output. Repeated `match` groups intersect:
    given `types: { doc: "file(match('*.md'))" }` and a use site
    `doc(match('guide*'))`, completion offers `guide-intro.md` but neither
    `notes.md` nor `guide.txt`, and root-union selection accepts an arm only
    for a path satisfying both groups. An exclusion in one group does not
    apply to the other group's positives. Groups with an empty intersection
    parse without error and without any file search, and completion offers
    nothing for them. `match` stays passive during ordinary single-schema
    validation. Claudine completion reads the grouped representation.

## Open questions

None.
