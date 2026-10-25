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
human_review: false
pipeline_step: 6
canonical_docs:
    - darkmatter/docs/topics/schemas/schema-types.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/topics/schemas/local-type-references.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/inline/doc-list.md
    - darkmatter/dmls/docs/autocomplete.md
    - darkmatter/dmls/docs/hover.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/state-management/side-effects.md
related:
    - 2026-10-13-named-type-presence-and-local-names
    - 2026-10-21-claudine-schemas
    - 2026-10-23-shell-template-approval
    - 2026-10-28-schema-enhancements
    - 2026-10-31-type-aware-evaluation
implemented: true
completed: true
---

# Core Schema Types: `unknown`, `null`, `alphanumeric`, `shell`

This is Phase A of Darkmatter's type system: four SimplifiedSchema types that
later work depends on.

- **`unknown`** (with `any` as an equivalent alias) names "every value,
  including null", so the type system has an honest name for an untyped
  variable.
- **`null`** lets a schema declare a property as deliberately absent, which
  makes exclusive unions (a spec file **or** a review file, not both) and
  optional whole values (`[person, null]`) expressible.
- **`alphanumeric`** is a string type made of ASCII letters and digits, with
  options that widen or narrow the character set. Typed shell-command approval
  (`2026-10-23-shell-template-approval`) stores rules such as
  `{{ alphanumeric(allow-slashes) }}`, and uses this spec's subtype relation
  to decide whether a saved rule covers a later command.
- **`shell`** names a value that is a shell command, so Claudine's lifecycle
  event types can include it (`start`) or exclude it (`initialize`) through
  ordinary schema validation (`2026-10-21-claudine-schemas`).

Authors gain exact schemas for these cases, with validation, generated JSON
Schema, `md schema about`, and DMLS completion/hover all agreeing.

## Decided

### 1. `unknown` and its alias `any`

- `unknown` accepts every value, including `null`, and omission. It lowers to
  JSON Schema `{}`.
- `any` is an equivalent spelling: it parses to the same type, lowers
  identically, and accepts the same values. `unknown` is the canonical name:
  `md schema about`, DMLS completion and hover, serialization back to
  SimplifiedSchema, and documentation use `unknown` and mention `any` as its
  alias.
- `required` has no effect on `unknown` or `any`: `spec: unknown(required)`
  accepts null and omission. (This changes today's `any(required)`, which
  demands key presence.) A schema or trigger that needs a value to be present
  uses a concrete type, for example `agent: string(required)`. "Anything
  except null" is written as a union of every accepted type except `null`,
  for example `value: [string(required), number, boolean, object, "unknown[]"]`
  (the `unknown[]` arm's underlying type is an array; the union's required
  presence removes the ordinary optional-null wrapper). Decided
  2026-10-11.
- An unschematized expression variable's type is `unknown`. Every place in the
  docs or diagnostics that describes an undeclared name's type as `any` says
  `unknown` instead. This rename does not add static checks: existing runtime
  errors for invalid operations on an unknown-typed value remain errors.

### 2. `null`

- `null` accepts only an explicit null value or an omitted property. Any
  non-null value fails validation.
- It lowers to JSON Schema `{"type": "null"}`, and the property is never
  placed in the parent's `required` list (absence is allowed).
- `required` has no effect on `null`: `foobar: null(required)` is legal and
  accepts explicit null and omission.
- Spelling: at a type position the YAML null scalar (`null`, `Null`, `NULL`,
  `~`) and the string `"null"` both mean the `null` type, so `review: null`
  and `[person, null]` work unquoted. `null(required)` is an ordinary type
  expression string.
- A `default(…)` on `null` must be null; any other default is a schema-load
  error.
- This adds null only at a **property type position**, including a union arm
  or a named type's definition. It does not make an empty root `$schema:`,
  a null root-union arm, or `literal(null)` legal. Serialization emits the
  explicit type string `"null"`, not an empty YAML scalar. Meta-type validation
  uses the same rule as ordinary schema parsing.

### 3. Null-admitting properties are never required

A property whose underlying value type explicitly admits null — scalar
`null`, scalar `unknown`/`any`, or a property-level union with one of those
scalar arms — is never
required. A `required` written on it, inherited from a named type
(`2026-10-13-named-type-presence-and-local-names`), or hoisted from another
union arm has no effect. This is how a whole value is made optional when its
type carries `required`:

```yaml
kind: schema
types:
    code: string(required; min(3))
    record: "{ code: code }"        # `code` is required (inherited)
    draft_code: [code, null]        # optional: null, omitted, or a valid code
```

The compact `code | null` spelling arrives with
`2026-10-28-schema-enhancements` and means the same thing.

Determine this after named references have been resolved, before adding the
ordinary optional-property null wrapper. Otherwise every optional concrete
type would accidentally lose its ability to become required. This concerns
the **whole property**, not its children: `unknown[](required)` and
`null[](required)` require a non-null array, whose items respectively accept
any value or only null. A required object with a nullable child still requires
the object. Existing rules for `required` before and after `[]` remain intact.

One shared presence calculation must govern lowering, trigger matching,
composition's optional-property materialization, and launch/completion phase
validation. Neither `eager` nor `generated` restores presence for an explicitly
nullable whole value. Constraints on other arms still apply to their non-null
values; `[string(required; min(3)), null]` rejects `"ab"`.
Raw JSON Schema retains its authored `required` list, including a required
property whose schema is `{}` or `{"type":"null"}`. This is an intentional
SimplifiedSchema change, not a change to raw JSON Schema.

### 4. Root unions are inclusive; exclusion is explicit

Root-level unions accept a document that matches **one or more** arms.
Ordinary root mappings are open, so a property an arm does not declare is
unconstrained in that arm. Existing pattern-key dictionary restrictions and
explicit raw JSON Schema restrictions still apply; this feature does not
make closed objects open.

```yaml
$schema:
  - schema: file(required)
  - review: file(required)
```

accepts a `schema` file, a `review` file, or both; it rejects neither. The XOR
idiom excludes overlap with `null`:

```yaml
$schema:
  - schema: file(required)
    review: null
  - review: file(required)
    schema: null
```

accepts exactly one file (the other omitted or explicitly null) and rejects
both or neither. Existing coercion and arm-selection behavior is unchanged.

### 5. `alphanumeric`

A new string type (like `email`), not a constraint on `string`.

**Base set:** ASCII letters and digits only, `[A-Za-z0-9]`. No option is on by
default; it is a general type, not a directory-name type.

**Type options** are part of the type's identity. They are written before `;`,
the way `enum` members are, and ordinary constraints follow the `;`:

```yaml
$schema:
    slug:   alphanumeric(allow-dashes; min(1); max(40))
    path:   alphanumeric(allow-slashes, allow-periods, block-double-periods, block-leading-slash)
    name:   alphanumeric(leading-alpha, allow-underscores)
    code:   alphanumeric(min(1))       # no options; constraints only
```

| Option | Effect |
|---|---|
| `allow-dashes` | also allows `-` |
| `allow-underscores` | also allows `_` |
| `allow-slashes` | also allows `/` |
| `allow-periods` | also allows `.` |
| `allow-whitespace` | also allows the ASCII space character (U+0020) |
| `block-double-periods` | the value may not contain `..` |
| `block-leading-dash` | the first character may not be `-` |
| `block-leading-slash` | the first character may not be `/` |
| `leading-alpha` | the first character must be an ASCII letter |

- Options are a closed keyword set. An unknown option, a repeated option, or
  an option written after the `;` is a grammar error. When the parenthesized
  list has no `;` and contains no option keyword, it is read entirely as
  constraints (so `alphanumeric(min(1))` and `alphanumeric(required)` work);
  a list without `;` that mixes options and constraints is a grammar error
  whose message shows the `;` form.
- The empty string is accepted unless a constraint excludes it (`min(1)` or
  `not-empty`). The leading-character options constrain the first character
  only when there is one.
- A narrowing option whose character is not allowed (`block-leading-dash`
  without `allow-dashes`, `block-double-periods` without `allow-periods`,
  `block-leading-slash` without `allow-slashes`) is accepted and redundant.
  The type's canonical form drops redundant options; identity, the subtype
  relation (Decided 7), serialization, and `md schema about` use the
  canonical form.
- `leading-alpha` also makes `block-leading-dash` and
  `block-leading-slash` redundant. Canonical option order follows the table
  above regardless of input order; compare named option sets after
  canonicalization. Equivalent spellings do not cause a use-site conflict.
- **Constraints** after `;`: `min(n)`, `max(n)`, `not-empty`, `pattern(re)`,
  `suggest(…)`, `default(…)`, and the universal `required`, `generated`,
  `eager`, plus the existing `example(…)` annotation. Value constraints only
  narrow; suggestions, defaults, examples, and timing/ownership annotations
  retain their existing roles. A `default` or `suggest` value must itself be a
  valid value of the type, or the schema fails to load.
  This stricter suggestion check applies to `alphanumeric` only; it does not
  turn other types' advisory suggestions into validation constraints.
- **Named types:** constraint merging from
  `2026-10-13-named-type-presence-and-local-names` applies only to the
  constraints after `;`. A use site may repeat the named type's exact option
  set; writing a different option set at a use site (or merging two different
  option sets for one named type by any route) is a schema error, never a
  widening:

  ```yaml
  kind: schema
  types:
      slug: alphanumeric(allow-dashes)
      a: slug(max(10))                     # ok: constraints only
      b: slug(allow-dashes; max(10))       # ok: same option set
      c: slug(allow-periods)               # error: different option set
  ```

- **Lowering:** `{"type": "string", "pattern": <regex>}` where the regex
  encodes exactly the canonical option set, using only features the
  validator's linear-time regex engine supports (no lookaround). A
  user-written `pattern(…)` must hold as well (both patterns apply). The
  lowered fragment also carries an annotation recording the type and its
  canonical options (`"x-darkmatter-type": "alphanumeric"` plus the option
  list under `"x-darkmatter-options"`) so JSON consumers can recover the type
  identity. Emit an array of canonical keyword strings, empty for the base
  type. These annotations do not perform validation. Preserve the base
  pattern, each user pattern, and `not-empty` together using `allOf` where
  necessary; none may overwrite another. The base pattern must check the
  entire string, including leading/trailing newlines, and encode forbidden
  consecutive periods without switching to a backtracking engine.
- Coercion: native numbers and booleans are not coerced; they are type
  mismatches (an `alphanumeric` value must be a YAML string).
- Options are independent of the host. `allow-slashes` allows `/`, never
  `\`; `allow-whitespace` allows space, never tabs, newlines, or other Unicode
  whitespace. This type does not establish that a value is a safe path or a
  safe shell argument; quoting and path policy belong to their consumers.

### 6. `shell`

A string type for a value that is a shell command, in the content-format
family beside `expression`.

- Accepts a string that contains at least one non-whitespace character.
  It performs **no shell-syntax check**: Darkmatter's own command grammar
  (frontmatter `$( … )` and `::shell`) is narrower than the system shell that
  runs Claudine's lifecycle `shell` actions (`sh -c` / `cmd /C`), so no single
  parser is correct for every host. Validation is passive: never executed,
  never approved, no I/O.
- Lowers to `{"type": "string", "format": "darkmatter-shell"}`. Register
  the format in every Darkmatter validator builder and use the existing
  literal-token decoding boundary. Whitespace follows Rust's
  `char::is_whitespace`; custom-format enforcement is required for consumers
  of exported JSON Schema. A validator that ignores unknown formats cannot
  enforce this check.
- Constraint applicability, optional-nullability, coercion of native booleans
  and numbers to their string form, and the `$( … )` / `{{ … }}`
  pending-value deferral rules mirror `expression` exactly: it permits
  `required`, `default(…)` (whose value must itself be a valid `shell` value),
  `generated`, `eager`, `example(…)`, and array constraints with `[]`; string
  constraints and `suggest(…)` are rejected. Scalar boolean/number defaults
  use the same text conversion as supplied values; structured defaults fail
  to load.
- Its job is type identity: a schema can say "this value is a shell command",
  and a union of action types can include or exclude it. For example, a
  lifecycle schema can type `start` as a list of an action union that
  includes `{ shell: shell(required) }` and type `initialize` as a list of a
  union that does not, so a `shell` action under `initialize` fails ordinary schema
  validation in `md schema validate`, composition, and DMLS.
- Action object alternatives must be closed inline shapes, as existing
  SimplifiedSchema inline objects are, with a required action key. An opaque
  `object` or `unknown` alternative would accept a shell action regardless of
  this type's exclusion. Reject both a shell-only object and an object mixing
  an allowed action with a `shell` key. This type adds authoring validation;
  Claudine's existing parser and runtime prohibition on shells during
  `initialize` remain necessary, including shell-valued `set` and bootstrap
  `$(...)` forms, which are not covered merely by excluding a `shell` key.

### 7. Subtype relation among string-like types

Darkmatter exposes one passive function that answers "does every non-null
string accepted by the underlying type A also satisfy the underlying type
B?" for the types in the table below, computed from
the typed SimplifiedSchema atoms (not from lowered JSON). Ordinary
constraints (`min`, `max`, `pattern`, …) are ignored; only the type and its
options count. Property absence, optional-null wrappers, scalar coercion,
array containers, unresolved references, and ordinary constraints are outside
this relation. Resolve named types first. Two identical types (the same type
with the same canonical options) are always subtypes of each other, whatever
the type. `2026-10-23-shell-template-approval` consumes it to decide
whether a saved approval rule covers a command.

| A | B | A ⊆ B? |
|---|---|---|
| any type | the same type with the same canonical options | yes |
| `alphanumeric(X)` | `alphanumeric(Y)` | yes iff X's allowed character set and allowed first-character set are subsets of Y's corresponding sets, and any prohibition on `..` in Y is guaranteed by X |
| any always-text type: `alphanumeric(…)` (any options), `url`, `enum(…)`, `date`, `datetime`, `time`, `email`, `shell`, `expression`, and every other type whose accepted values are always strings | `string` | yes |
| `string` | `alphanumeric(…)` | no |
| `email` | `alphanumeric(…)` | no |
| any other pair | | no |

Example, with B = `alphanumeric(allow-slashes)`: `alphanumeric(allow-slashes)`
⊆ B (identical); `alphanumeric` ⊆ B (narrower); `string` ⊄ B; `email` ⊄ B.

The first-character set is the allowed character set with the leading
restrictions applied; all option sets still accept the empty string. X
guarantees a prohibition on `..` when it either blocks double periods or
does not allow periods at all. For example, base `alphanumeric` is a subtype
of `alphanumeric(allow-periods, block-double-periods)`, and
`alphanumeric(leading-alpha, allow-dashes)` is a subtype of
`alphanumeric(allow-dashes, block-leading-dash)`.

**Reader's note:** Comparing keyword sets alone misses both examples. Compare
what the options permit, so an approval is not refused just because a narrower
type guarantees a restriction without spelling its keyword. This is bounded
reasoning over the listed options, not inclusion checking for user regexes.
The later shell-approval design additionally says a `string` rule covers
every scalar after conversion; that conversion rule is distinct from this
function and must not make `unknown`, numbers, or arrays subtypes of `string`
here. Runtime value checks and shell quoting remain mandatory.

## Requirements

- Add the types to the SimplifiedSchema grammar, model, lowering, coercion,
  serializer, and the `md schema about` descriptor catalog
  (`schema_type_descriptors` and its parity tests). Internal Rust names and
  the migration of existing `Any` users are implementation choices.
- **`any(required)` migration (a task of this spec).** This spec is the first
  to change what `any(required)` means, so it migrates every shipped use in
  the same change. Because `required` no longer has an effect on `any`, each
  use changes meaning: the trigger fixtures under
  `darkmatter/tests/fixtures/schema-triggers/schemas/` (`claudine.trigger.yaml`,
  `inline-compose.trigger.yaml`, `sequence.trigger.yaml`, each with
  `steps: any(required)`), the trigger examples in
  `darkmatter/docs/topics/schemas/definition.md`, and the two
  `value: any(required)` fields in
  `claudine/docs/schemas/partials/side-effect.yaml`. These are the known sites,
  not an exhaustive list: audit shipped schemas, embedded baselines, inline
  schemas, tests, and documentation, and record the results in the
  implementation log. A trigger arm whose only
  presence condition was `any(required)` becomes vacuous and is rejected by
  the vacuous-arm lint. Replace each use with a concrete type: the narrowest
  type expression that admits every valid value of that key in the schema the
  trigger targets, with `(required)`. Preserve positive and negative matching,
  including `none` conditions: validate the fixture's actual input domain
  rather than inferring it from a target schema that does not declare `steps`.
  Replacing a presence check with a concrete type also stops matching
  malformed non-null values; record that intentional difference and test that
  it cannot silently select an unrelated trigger.
  **Side-effect `value` fields: null acceptance is decided per field.**
  Rejecting `null` is the rule, but a field accepts `null` where null has a
  defined meaning for that action; otherwise it becomes a union of every
  accepted type except `null`, with `(required)`. Today's `any(required)`
  accepts explicit null, so each field is decided on that basis:
  `set_frontmatter`'s `value` accepts null, because the operation stores a
  supplied null value today
  ([`set_frontmatter`](../../../darkmatter/lib/src/effects/verbs.rs)); its
  type is the union of every accepted type including `null` (a
  null-admitting union, so per Decided 3 `required` has no effect on it).
  `merge_frontmatter` has no `value` field after the correction below. Any
  further migrated `any(required)` side-effect field found by the audit
  applies the same rule, and the implementation log records the decision for
  each. Correct the `merge_frontmatter` schema shape to
  its existing two-argument operation (file plus object); it has no `prop`
  argument and does not accept arbitrary scalar values. This corrects schema
  drift; it does not change the operation.
  Update the existing convert
  test that asserts `any(required)` rejects absence, while retaining its
  explicit-null acceptance case. Later specs that add
  trigger files (`2026-10-21-claudine-schemas`,
  `2026-10-27-darkmatter-lifecycle-ownership`) write concrete types from the
  start.
- **Keyword collisions.** `unknown`, `null`, `alphanumeric`, and `shell`
  become built-in keywords, which win over a same-named local type referenced
  bare. Audit shipped schemas for local types with those names and switch any
  bare reference to `@this`.
- **Validation docs.** Reconcile `definition.md` text that treats a required
  null as absence ("Optional by default", the `any` row of the Types table,
  Universal Constraints and the phase table's "Missing and explicit `null`
  both count as absence") with Decided 1-3: those rules hold for every type
  except null-admitting ones, for which `required` has no effect. Add Types
  table rows and sections for `unknown`, `null`, `alphanumeric`, and `shell`;
  document the inclusive root-union rule and the XOR idiom under Root-Level
  Unions; document the subtype relation. Update the `darkmatter` skill.
- **Tests that expected rejection.** Any test or snapshot that asserts
  `unknown` or `null` is rejected by the grammar is updated to the new
  behavior; such expectations were never permanent requirements.
- **DMLS.** Completion offers `unknown`, `any`, `null`, `alphanumeric`, and
  `shell` wherever type keywords are offered, and offers the `alphanumeric`
  option names inside its parentheses before `;`. Hover describes each type
  (and, for `alphanumeric`, the effective character set of its canonical
  options). Grammar errors for options (unknown, repeated, after `;`, mixed
  without `;`, conflicting option sets) appear as schema diagnostics.
- **Consumer parity.** DMLS and `md` use the library's parsed type options and
  canonicalizer, rather than reconstructing them from display strings. Preserve
  authored spans for native YAML null and option errors, including related
  locations when a named definition conflicts with a use site. Update the
  Darkmatter README's public type summary and the current schema topic pages,
  including editor support. The implementation must not change composition
  stage order or treat inserted data containing `$(...)` or `{{...}}` as newly
  executable or pending syntax.

## Out of scope

- Function signatures, `|` unions, tuples, and the expression-variable type
  projection (`A | null` for optional properties): `2026-10-28-schema-enhancements`.
- Type-aware evaluation, narrowing, and diagnostics:
  `2026-10-31-type-aware-evaluation`.
- Approval rules, matching, and run-time type checks of shell templates:
  `2026-10-23-shell-template-approval`.
- Claudine's action-type schemas that use `shell`:
  `2026-10-21-claudine-schemas`.

## Acceptance criteria

1. Phase A table, through the schema parser and validator (and
   `md schema validate` end to end):

   | Declaration or schema | Input | Expected |
   |---|---|---|
   | `file(required)` | valid filepath | accepted |
   | `file(required)` | null or omitted (final validation) | rejected |
   | `null` or `null(required)` | null or omitted | accepted |
   | `null` or `null(required)` | any non-null value | rejected |
   | `unknown`, `any`, or either with `(required)` | any value, null, or omitted | accepted |
   | inclusive root union (Decided 4) | either file, or both | accepted |
   | inclusive root union | neither | rejected |
   | XOR root union (Decided 4) | one file, the other null or omitted | accepted |
   | XOR root union | both, or neither | rejected |

2. Lowering: `null` → `{"type":"null"}` with the key absent from `required`;
   `unknown` and `any` → `{}`; `null(required)` and `unknown(required)` never
   add the key to `required`; `shell` → `format: darkmatter-shell`.
3. `[person, null]` where `person` carries `required` (written or inherited)
   accepts null and omission; `[string(required), unknown]` likewise.
4. `review: null` and `review: ~` unquoted parse as the `null` type.
5. `alphanumeric`: for each option, a positive and a negative value; the empty
   string accepted without constraints and rejected with `min(1)`;
   `alphanumeric(allow-periods, block-double-periods)` rejects `a..b` and
   accepts `a.b`; `leading-alpha` rejects `1abc`; a non-ASCII letter (`é`) is
   rejected; grammar errors for unknown, repeated, post-`;`, and mixed
   options; the named-type option-set conflict error; a native number value is
   a type mismatch; the generated regex compiles in the validator.
6. Subtype function: every row of the string-like subtype table above, the
   redundant-option
   canonicalization case (`alphanumeric(block-leading-dash)` and
   `alphanumeric` are mutually subtypes), and that `min`/`pattern`
   constraints do not affect the answer. Cover option order, restrictions
   implied by `leading-alpha` or an absent allow-option, identical
   non-string-like types (`number` ⊆ `number`, an identical array type)
   returning true, each always-text type (`url`, `enum(…)`, `date`,
   `datetime`, `time`, `email`, `shell`, `expression`, `alphanumeric(…)`)
   ⊆ `string` returning true, and `unknown`, `number`, or an array type
   against `string` returning false.
7. `shell`: an empty or whitespace-only string is rejected; a command
   containing a pipe or `;` is accepted; a native boolean coerces to its
   string form; a lifecycle-shaped fixture schema whose `initialize` union
   excludes `{ shell: shell(required) }` rejects a `shell` action under
   `initialize` in `md schema validate` and as a DMLS diagnostic, and accepts it under
   `start`. Also reject a mixed allowed-action-plus-shell object and an empty
   action object. Existing initialization parser/runtime tests remain green.
8. The `any(required)` migration is complete: shipped trigger fixtures and
   partials load, valid trigger inputs retain their selection, and malformed
   inputs follow the documented migration policy (existing trigger tests green
   with updated fixtures). No shipped file still relies on
   `any(required)` demanding presence. `set_frontmatter`'s `value` is
   the null-admitting union and accepts explicit null and every other
   accepted type; any
   other migrated side-effect field accepts null only where the
   implementation log records a defined meaning for it, and otherwise
   rejects null. The `merge_frontmatter` definition accepts an object without
   `prop` and rejects non-object input.
9. DMLS completion offers the new keywords and `alphanumeric` options; hover
   shows `unknown` with its `any` alias.
10. `md schema about` lists `unknown` (alias `any`), `null`, `alphanumeric`
    (with options), and `shell`, and the descriptor parity tests pass.
11. `just test` and `just lint` pass in `darkmatter/`, plus `just test` in
    `claudine/` for the partial change. Grammar changes include a passive
    shipped-artifact corpus test and an end-to-end test through
    `md schema validate`. Tests are L1, use no terminal or browser windows,
    and are portable to macOS, Linux, Windows, and WSL2. No new CI cells.
12. Presence agrees across ordinary validation, trigger matching, composition,
    and launch/completion validation: required nullable whole values remain
    optional, required arrays with nullable items remain required, and a
    nullable child's presence does not change its parent. Raw JSON Schema
    `required` continues to require nullable properties. Ordinary optional
    concrete types can still become required; generated/eager phase behavior
    for non-nullable types is unchanged.
13. Generated schemas and Darkmatter's typed validation agree for
    `alphanumeric`, including newlines at either edge, tabs, non-ASCII
    whitespace, overlapping constraints, and canonical annotations. Optional
    null is accepted while required null is rejected. Coercion preserves
    numeric/boolean inputs unchanged when checking `alphanumeric`, including
    array items and candidate union arms; other union arms retain their own
    coercion rules. A round trip through the SimplifiedSchema serializer preserves
    meaning and emits canonical options, `unknown` for `any`, and an explicit
    `"null"` type spelling.
14. `null` works as a native YAML property definition, a property-union arm,
    and a named definition through local and imported references. Null root
    declarations/arms and `literal(null)` remain errors. Valid null defaults
    load; non-null defaults for scalar `null` fail with a source location.
    `unknown` values retain their native scalar/object/array types and nested
    nulls; they undergo no string coercion. Schema meta-types, DMLS, and the
    CLI agree on these cases.

## Open questions

None.
