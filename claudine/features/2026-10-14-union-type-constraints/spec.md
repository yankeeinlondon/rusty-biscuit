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
pipeline_step: 4
depends-on:
    - 2026-10-13-named-type-presence-and-local-names
related:
    - 2026-10-17-content-policy-base-schema
    - 2026-10-28-schema-enhancements
    - 2026-10-29-recursive-schema-types
---

# Constraints on References to Union-Typed Named Types

A named type can be a union (a YAML sequence of alternative type
expressions), and it can be used bare today. But the moment a use site adds
anything, the resolver refuses: `policy[]@file`, `policy[](min(1))@file`, and
even `short_form(required)` fail with "cannot apply `[]`/constraints to the
union-typed named type". So an author cannot say "a non-empty list whose
entries are each a rule string or a rule mapping", and cannot make a
union-typed field required.

After this change `[]`, the array constraints, and the presence markers apply
to a reference to a union-typed named type. A list of a union lowers to JSON
Schema `items: { anyOf: [...] }`, and every validator, cleaner, coercer,
rewriter, and editor feature selects the matching arm for each list item.

This is schema groundwork. Content Policy's editor schema
(`2026-10-17-content-policy-base-schema`) needs both shapes; Claudine's
lifecycle lists (`- stop` beside object items) need the list form.

## Decided

Decided 2026-10-10: `[]`, `min(…)`/`max(…)` list constraints, and `required`
apply to a reference to a union type, and a list of a union lowers to
`items: { anyOf }`.

### 1. What a use site may add to a union-typed reference

Given a union-typed named type `U` (bare local, `@this`, or cross-file):

| Use site | Meaning |
|---|---|
| `U` | Unchanged: the union, inlined (today's behavior). |
| `U(required)` / `U(generated)` / `U(required; generated)` | The union, with the presence marker applied to the **property as a whole**, exactly as `required` on any arm of a property-level union is hoisted today. |
| `U -> text` | The union with the property description `text`. |
| `U[]` | An array whose every item is one of `U`'s arms. |
| `U[](min(n); max(n); unique; required; default([...]))` | That array with the usual array constraints. |

Value-level constraints that would have to be pushed into every arm
(`U(max(10))`, `U(pattern(…))`, `U(suggest(…))`, `U(default(x))` without
`[]`, …) remain rejected, with a typed error that names the constraint and
says it cannot apply to a union-typed type. `[]` on a union where any arm is
itself an array is rejected, mirroring today's "already-array named type"
error. Presence markers inherited from the arms follow
`2026-10-13-named-type-presence-and-local-names` (they survive and hoist).

### 2. Lowering

```yaml
# policy.yaml
kind: schema
types:
    short_form: [string(pattern(^Evergreen$)), string(pattern(^TimeSensitive$))]
    long_form: "{ rule: short_form(required), action: enum(refresh, archive; required) }"
    policy: [short_form, long_form]
```

```yaml
# a document's frontmatter
$schema:
    content_policy: policy[](min(1))@./policy.yaml
```

`content_policy` lowers to (optional-property wrapping as for any optional
array):

```json
{
  "type": "array",
  "minItems": 1,
  "items": { "anyOf": [ { "type": "string", "pattern": "^Evergreen$" },
                        { "type": "string", "pattern": "^TimeSensitive$" },
                        { "type": "object", "properties": { "rule": { "anyOf": [ … ] }, "action": { … } },
                          "required": ["rule", "action"], "additionalProperties": false } ] }
}
```

A union nested inside a union is flattened into one `anyOf` exactly as
inlining a union arm does today; arm descriptions are kept per arm. Items are
not null-wrapped (the same as items of any other array). `rule:
short_form(required)` lowers to the union with `rule` in the parent's
`required` list.

### 3. One representation: an array whose item is a union

The typed model gains a way to say "an array of a union". Today `[]` binds
per arm (`PropertyAtom.is_array`), so no such shape exists. The implementer
chooses the Rust spelling (for example a `TypeExpr` variant that holds union
arms, used with `is_array: true`), subject to:

- it is the **only** representation of an array of a union; there is no
  second, parallel encoding;
- it serializes back to SimplifiedSchema text, round-trips through the
  passive parser, and appears in `md schema about` / the descriptor catalog
  where array types do;
- discriminated-union narrowing, closest-arm reporting, and the
  nullable-wrapper unwrapping apply to an item union exactly as they apply to
  a property-level union.

Later specs build on this representation and must not introduce their own:
`2026-10-28-schema-enhancements` builds the `(A | B)[]` spelling on it, and
`2026-10-29-recursive-schema-types` no longer needs its own array-of-union
item (its design item 3 is satisfied here; it adds only the `$ref` form for
cyclic types).

### 4. Every consumer selects arms

A consumer that walks into `items` must handle `items: { anyOf }` by selecting
the arm that matches the item (discriminant first, then the closest arm, as
property-level unions do), never by reading `properties` off the `anyOf`
wrapper. A walker that misses this degrades **silently** (it finds no
`properties` and returns nothing), so each one gets a targeted test.

JSON-schema walkers (file paths as of 2026-10-10):

- `darkmatter/lib/src/markdown/schemas/validate.rs` — description and node
  lookup by instance path (~:1050-1054) and `unwrap_nullable_arm`
  (~:1133-1142);
- `clean.rs` — both `items` descents (~:491-494, ~:636-639);
- `rewrite.rs` — eager-file array rewrite (~:263-266);
- `coerce.rs` — array coercion target (~:195-198);
- `discriminant.rs` — arm selection.

Typed-tree consumers: `schemas/completion.rs`, `schemas/frontmatter_shape.rs`,
`schemas/about.rs`, `schemas/phase.rs`, `schemas/detect.rs`,
`schemas/triggers/matcher.rs`, `compose/context/catalog.rs`, the CLI's
`commands/schema/assignment.rs`, and in DMLS `overlay/schema.rs` and
`providers/frontmatter.rs`. Each handles the new representation (completion
offers every arm's keys/values for a list item, narrowed by discriminant
where one applies; hover shows the arm that matches).

## Requirements

- Lift the union rejection in `apply_import_postfix`
  (`darkmatter/lib/src/markdown/schemas/resolve.rs`) per Decided 1.
- Lowering in `simplified/convert.rs` per Decided 2.
- Docs: remove the "No arrays of ordinary denoted-value unions" limitation in
  `darkmatter/docs/topics/schemas/definition.md` (~:1683) and document the
  table in Decided 1 under Cross-File Named-Type Imports and Unions, with the
  Content Policy shape as the example. Update `dmls-schema-support.md`'s
  matrix and the `darkmatter` skill.

## Out of scope

- The `(A | B)[]` and `A | B` spellings: `2026-10-28-schema-enhancements`.
- `$defs`/`$ref` and recursion: `2026-10-29-recursive-schema-types`.
- Content Policy's baseline wiring and `rule` becoming required in its shipped
  schema: `2026-10-17-content-policy-base-schema`.
- Diagnostic line/column at the offending list item:
  `2026-10-15-diagnostic-node-positions`.

## Acceptance criteria

1. The Content Policy shapes load and validate. With a copy of
   `content-policy/schemas/content-policy.yaml` as a test fixture in which
   `long_form.rule` is `short_form(required)` (bare local name) and same-file
   references are bare, a document property typed
   `policy[](min(1))@<fixture path>`:
   - accepts `[ValidFor(3mo), {rule: "ValidUntil(2027-01-01)", action: archive}]`;
   - rejects `[]` (min items);
   - rejects `[Duration(3mo)]` (no arm matches);
   - rejects `[{action: archive}]` and `[{rule: null, action: archive}]`
     (`rule` required);
   - rejects `[{rule: ValidFor(3mo), action: delete}]`.
2. `U(required)` makes the property required; `U(max(10))` without `[]` and
   `[]` over a union with an array arm fail with the typed errors from
   Decided 1.
3. The lowered JSON Schema for the example in Decided 2 has `type: array`,
   `minItems: 1`, and `items.anyOf` with one entry per flattened arm.
4. Per-walker tests through a union item, each failing if the walker reads the
   `anyOf` wrapper blindly: validate (problem description / node lookup for
   `/content_policy/1/rule`), clean (an unknown key inside a long-form item is
   cleaned), rewrite (an eager `file` arm inside a union item is rewritten),
   coerce (a `boolish`/`numberlike` arm inside a union item coerces),
   discriminant (a literal-tagged item union narrows to one arm's problems).
5. Typed-tree tests: completion inside a list item offers keys from the
   object arms and values from the scalar arms; DMLS hover on an item shows
   the matching arm; `md schema about` renders the array-of-union type.
6. Every existing schema that does not use the new forms lowers to
   byte-identical JSON Schema (shipped corpus and resolver fixtures green).
7. `just test` and `just lint` pass in `darkmatter/`. Tests are L1, use no
   terminal or browser windows, and are portable to macOS, Linux, Windows, and
   WSL2. No new CI cells.

## Open questions

None.
