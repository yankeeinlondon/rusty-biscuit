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
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: false
human_review: false
pipeline_step: 6
related:
    - 2026-10-13-named-type-presence-and-local-names
    - 2026-10-21-claudine-schemas
    - 2026-10-23-shell-template-approval
    - 2026-10-28-schema-enhancements
    - 2026-10-31-type-aware-evaluation
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
  (the `unknown[]` arm is an array, which does not admit null). Decided
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

### 3. Null-admitting properties are never required

A property whose type admits null — `null`, `unknown`/`any`, or a
property-level union with a `null`, `unknown`, or `any` arm — is never
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

### 4. Root unions are inclusive; exclusion is explicit

Root-level unions accept a document that matches **one or more** arms. A
property an arm does not declare is unconstrained (`unknown`) in that arm.

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
- **Constraints** after `;`: `min(n)`, `max(n)`, `not-empty`, `pattern(re)`,
  `suggest(…)`, `default(…)`, and the universal `required`, `generated`,
  `eager`. They only narrow. A `default` or `suggest` value must itself be a
  valid value of the type, or the schema fails to load.
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
  list) so JSON consumers can recover the type identity.
- Coercion: native numbers and booleans are not coerced; they are type
  mismatches (an `alphanumeric` value must be a YAML string).

### 6. `shell`

A string type for a value that is a shell command, in the content-format
family beside `expression`.

- Accepts a string that contains at least one non-whitespace character.
  It performs **no shell-syntax check**: Darkmatter's own command grammar
  (frontmatter `$( … )` and `::shell`) is narrower than the system shell that
  runs Claudine's lifecycle `shell` actions (`sh -c` / `cmd /C`), so no single
  parser is correct for every host. Parse-only and passive: never executed,
  never approved, no I/O.
- Lowers to `{"type": "string", "format": "darkmatter-shell"}`.
- Constraint applicability, optional-nullability, coercion of native booleans
  and numbers to their string form, and the `$( … )` / `{{ … }}`
  pending-value deferral rules mirror `expression` exactly: it permits
  `required`, `default(…)` (whose value must itself be a valid `shell` value),
  `generated`, and array constraints with `[]`; string constraints and
  `suggest(…)` are rejected.
- Its job is type identity: a schema can say "this value is a shell command",
  and a union of action types can include or exclude it. For example, a
  lifecycle schema can type `start` as a list of an action union that
  includes `{ shell: shell }` and type `initialize` as a list of a union that
  does not, so a `shell` action under `initialize` fails ordinary schema
  validation in `md schema validate`, composition, and DMLS.

### 7. Subtype relation among string-like types

Darkmatter exposes one passive function that answers "does every value of
type A also satisfy type B?" for the string-like types below, computed from
the typed SimplifiedSchema atoms (not from lowered JSON). Ordinary
constraints (`min`, `max`, `pattern`, …) are ignored; only the type and its
options count. `2026-10-23-shell-template-approval` consumes it to decide
whether a saved approval rule covers a command.

| A | B | A ⊆ B? |
|---|---|---|
| any type | the same type with the same canonical options | yes |
| `alphanumeric(X)` | `alphanumeric(Y)` | yes iff X's allow-options ⊆ Y's allow-options **and** X's narrowing options ⊇ Y's narrowing options |
| `alphanumeric(…)` (any options) | `string` | yes |
| `email` | `string` | yes |
| `string` | `alphanumeric(…)` | no |
| `email` | `alphanumeric(…)` | no |
| any other pair | | no |

Example, with B = `alphanumeric(allow-slashes)`: `alphanumeric(allow-slashes)`
⊆ B (identical); `alphanumeric` ⊆ B (narrower); `string` ⊄ B; `email` ⊄ B.

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
  `claudine/docs/schemas/partials/side-effect.yaml`. A trigger arm whose only
  presence condition was `any(required)` becomes vacuous and is rejected by
  the vacuous-arm lint. Replace each use with a concrete type: the narrowest
  type expression that admits every valid value of that key in the schema the
  trigger or partial targets, with `(required)`. Where the key accepts any
  non-null value (the side-effect `value` fields), write the union of every
  accepted type except `null`, as in Decided 1. Update the existing convert
  test that asserts `any(required)` rejects absence. Later specs that add
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
6. Subtype function: every row of the Decided 7 table, the redundant-option
   canonicalization case (`alphanumeric(block-leading-dash)` and
   `alphanumeric` are mutually subtypes), and that `min`/`pattern`
   constraints do not affect the answer.
7. `shell`: an empty or whitespace-only string is rejected; a command
   containing a pipe or `;` is accepted; a native boolean coerces to its
   string form; a lifecycle-shaped fixture schema whose `initialize` union
   excludes `{ shell: shell }` rejects a `shell` action under `initialize` in
   `md schema validate` and as a DMLS diagnostic, and accepts it under
   `start`.
8. The `any(required)` migration is complete: shipped trigger fixtures and
   partials load and match as before their edit (existing trigger tests green
   with updated fixtures), no shipped file still relies on
   `any(required)` demanding presence, and the side-effect `value` fields
   reject omission and null while accepting every other value.
9. DMLS completion offers the new keywords and `alphanumeric` options; hover
   shows `unknown` with its `any` alias.
10. `md schema about` lists `unknown` (alias `any`), `null`, `alphanumeric`
    (with options), and `shell`, and the descriptor parity tests pass.
11. `just test` and `just lint` pass in `darkmatter/`, plus `just test` in
    `claudine/` for the partial change. Grammar changes include a passive
    shipped-artifact corpus test and an end-to-end test through
    `md schema validate`. Tests are L1, use no terminal or browser windows,
    and are portable to macOS, Linux, Windows, and WSL2. No new CI cells.

## Open questions

None.
