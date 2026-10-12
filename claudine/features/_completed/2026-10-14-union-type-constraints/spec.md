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
review_iterations: 2
completed: true
human_review: false
implemented: true
message_to_agent: |-
    All five phases are implemented. Read the implementation log's "## Phase 5" section first.

    - Phase 5 fixed two review defects, each test first:
        - `resolve.rs` now wraps `SchemaError::UnionReference` in `ReferencedSchema`, as it does the other named-type errors.
        - DMLS adds the use site as related information when the site is in another file.
        - DMLS now declares `darkmatter/lib/tests/union_fixtures/mod.rs` in `source-inputs`.
    - Phase 5 also added a round-trip and anonymous-syntax test for authored `U[]` references.
    - Byte identity is unchanged: 347/347 entries. The shipped Content Policy schema is unchanged.
    - Environment trap on the author's Mac: a plain `just lint` (or `cargo test --no-run`) sometimes compiled `darkmatter` from `/Volumes/coding/personal/rusty-biscuit`. That checkout holds broken uncommitted work, so the run failed with E0432/E0599 errors that are not in this worktree. Rerun with `KACHE_DISABLED=1`. The kache root cause is unproven.
    - Review nits deliberately left in place are listed in the log: duplicate flattening helpers, an unused `PropertyDef::alternatives()`, an unreachable `ItemMatch::Unique` arm, and "flattened" wording on `SchemaUnionRejection` arm indexes.
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
expressions). Explicit named references work today; bare local references
are supplied by the prerequisite named-type feature. When a use site adds
a postfix or description, the resolver refuses: `policy[]@file`, `policy[](min(1))@file`, and
even `short_form(required)` fail with "cannot apply `[]`/constraints to the
union-typed named type". So an author cannot say "a non-empty list whose
entries are each a rule string or a rule mapping", and cannot make a
union-typed field required.

After this change `[]`, the array constraints, and the presence markers apply
to a reference to a union-typed named type. A list of a union lowers to JSON
Schema `items: { anyOf: [...] }`, and schema consumers can inspect the
alternatives for each list item.
Validation and editor reporting may choose an arm to explain a failure;
coercion and rewriting retain their stricter rules before changing a value.

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
| `U(required)` / `U(generated)` / `U(eager)` / combinations of these | The union, with presence, generation, or validation timing applied to the **property as a whole**, following existing property-union rules. `eager` does not imply `required`. |
| `U -> text` | The union with the property description `text`. |
| `U[]` | An array whose every item is one of `U`'s arms. |
| `U[](min(n); max(n); unique; required; eager; generated; default([...]))` | That array with the usual array constraints. |

Value-level constraints that would have to be pushed into every arm
(`U(max(10))`, `U(pattern(…))`, `U(suggest(…))`, `U(default(x))` without
`[]`, …) remain rejected, with a typed error that names the constraint and
says it cannot apply to a union-typed type. `[]` on a union where any arm is
itself an array is rejected, mirroring today's "already-array named type"
error. Check this after resolving and flattening imported arms, including
multi-hop references. The prohibition concerns an arm whose outer value is an
array; arrays nested inside an object arm remain allowed.

Presence markers inherited from the arms follow the prerequisite
`2026-10-13-named-type-presence-and-local-names`: they survive and hoist to
the containing property, including when that property is an array. Constraints
such as an arm's string pattern remain on that item arm; a string `min(3)`
must not become the array's `minItems: 3`. Nested object fields retain their
own presence and timing. `generated` retains the existing annotation and
static-required suppression; it does not supply a value.

Universal `eager` is accepted before or after `[]` as property timing. An
inherited `file(eager)` arm still owns its file-existence check; applying
`eager` to the union or array does not turn lazy file arms into eager files.
Unsupported pre-array value constraints are rejected rather than distributed
across arms. Post-array constraints use the existing array applicability and
merge rules, including contradictory-bound errors.

Inherited union defaults must not become scalar defaults on an array.
If a lifted definition carries a default that cannot be an array default,
reject the declaration with a structured error identifying the definition
and use site; do not discard it or wrap it in a one-item list. Compatible
array defaults retain the prerequisite's equal-value merge rule; conflicting
defaults fail preparation. This does not introduce runtime default insertion.
A use-site description belongs to the containing union or array and does not
replace the descriptions of its arms.

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
inlining a union arm does today; arm descriptions are kept per arm. Items
receive no optional-property null wrapper (the same as items of any other
array); `[null]` is invalid for this example even though a missing or
null optional `content_policy` is allowed. `rule:
short_form(required)` lowers to the union with `rule` in the parent's
`required` list.

### 3. One representation: an array whose item is a union

The typed model gains a way to say "an array of a union". Today `[]` binds
per arm (`PropertyAtom.is_array`), so no such shape exists. The implementer
chooses the Rust spelling (for example a `TypeExpr` variant that holds union
arms, used with `is_array: true`), subject to:

- it is the **only** representation of an array of a union; there is no
  second, parallel encoding;
- authored references such as `U[]` round-trip through the passive parser
  and resolver with their defining named-type table and origins retained.
  An expanded anonymous union need not serialize as a standalone expression:
  `(A | B)[]` is outside this spec. `md schema about` and the descriptor
  catalog explain the array and all its item alternatives without promising
  unsupported parseable syntax;
- discriminated-union narrowing, closest-arm reporting, and the
  nullable-wrapper unwrapping apply to an item union exactly as they apply to
  a property-level union.

Later specs build on this representation and must not introduce their own:
`2026-10-28-schema-enhancements` builds the `(A | B)[]` spelling on it, and
`2026-10-29-recursive-schema-types` adds references for cyclic types; it
reuses this array-of-union representation rather than replacing it.

### 4. Consumers inspect alternatives without guessing

A consumer that walks into `items` must inspect `items.anyOf` rather than
reading `properties` directly from the union wrapper. Preserve these distinct
contracts:

- Validation retains `anyOf` acceptance: at least one arm must validate.
  The shared literal-discriminant selector narrows error reporting only when
  the authored tags select one arm unambiguously. Missing, unknown, duplicate,
  or conflicting tags retain ordinary union reporting. Closest-arm scoring
  explains invalid data; it is not proof that a transformation is safe.
- Coercion reuses the existing property-union candidate rule for each item:
  evaluate each candidate independently and commit only when exactly one
  arm's candidate validates. Zero or multiple matches leave that item
  unchanged. Keep specialized `boolish` and `numberlike` recognition ahead
  of generic union handling; they already lower to `anyOf` internally.
- Eager-file rewriting retains the property-union rule: only an unambiguous
  eligible eager-file arm may rewrite a scalar item. Object-arm traversal
  must preserve that rule for its nested file fields. Ambiguous alternatives
  must not acquire filesystem effects merely because they score closest.
- Cleanup never repairs inside a genuine union. The `darkmatter` library's
  cleanup code already refuses to infer a single string type from a genuine
  union, and that conservative behavior extends to union items: an item that
  matches no arm is left unchanged and its problems are reported, even when a
  literal tag would identify the intended arm. Cleanup must not delete keys
  based on the closest invalid arm or on any other arm selection; for
  example, an object with an extra key that fails every closed object arm
  keeps that key. Cleanup still descends through the array wrapper and still
  repairs non-union parts of the document as it does today.
- Completion combines applicable alternatives for incomplete items; an
  unambiguous literal tag narrows the choices. Hover shows a matching arm
  when unique, and the alternatives when the value is missing, invalid, or
  matches more than one arm.

**Reader's note:** the original draft required every consumer to select the
closest arm. This would let a reporting heuristic change stored data. The
revised contract reuses each consumer's existing safety rule and reserves
closest-arm selection for explanations.

JSON Schema consumers in the `darkmatter` library:

- [validate.rs](../../../darkmatter/lib/src/markdown/schemas/validate.rs) —
  description and node lookup by instance path, including nullable wrappers;
- [clean.rs](../../../darkmatter/lib/src/markdown/schemas/clean.rs) —
  schema descents used to authorize repairs;
- [rewrite.rs](../../../darkmatter/lib/src/markdown/schemas/rewrite.rs) —
  eager-file normalization;
- [coerce.rs](../../../darkmatter/lib/src/markdown/schemas/coerce.rs) —
  item coercion and independent union candidates;
- [discriminant.rs](../../../darkmatter/lib/src/markdown/schemas/discriminant.rs) —
  the shared literal-tag selector used by library and editor reporting.

Typed-tree consumers: `schemas/completion.rs`, `schemas/frontmatter_shape.rs`,
`schemas/about.rs`, `schemas/phase.rs`, `schemas/detect.rs`,
`schemas/triggers/matcher.rs`, `compose/context/catalog.rs`, the CLI's
`commands/schema/assignment.rs`, and in DMLS `overlay/schema.rs` and
`providers/frontmatter.rs`. Each handles the new representation (completion
offers every arm's keys/values for a list item, narrowed by discriminant
where one applies; hover shows the arm that matches).

## Requirements

- Extend the `darkmatter` library's
  [apply_import_postfix](../../../darkmatter/lib/src/markdown/schemas/resolve.rs),
  which applies use-site constraints to expanded named types, with the allowed
  union operations above. Bare, `@this`, and cross-file forms must agree.
  Keep definition origins, dependency tracking, import-cycle errors, and
  existing expansion limits; no new file-resolution or effectful parser path.
- Extend [convert.rs](../../../darkmatter/lib/src/markdown/schemas/simplified/convert.rs),
  the `darkmatter` library's JSON Schema lowering, to emit the array container
  separately from its item union. Preserve flattened arm order and metadata.
- Unsupported union operations produce a structured schema error identifying
  the property, named type, rejected constraint or array arm, and available
  definition/use-site provenance. DMLS projects the same error; do not add
  an editor-only interpretation. Item diagnostic source positioning remains
  outside this spec.
- Keep validation-only APIs passive and read-only. Composition alone performs
  existing coercion and successful eager-file normalization, in its existing
  stage order and captured request context.
- Docs: revise the array-of-union limitation in
  `darkmatter/docs/topics/schemas/definition.md` to allow named `U[]` while
  retaining the restriction on anonymous union array syntax. Document the
  allowed use-site operations under Cross-File Named-Type Imports and Unions, with the
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
   - accepts missing or null `content_policy` because it is optional;
   - rejects `[null]` and a scalar `ValidFor(3mo)`;
   - rejects `[]` (min items);
   - rejects `[Duration(3mo)]` (no arm matches);
   - rejects `[{action: archive}]` and `[{rule: null, action: archive}]`
     (`rule` required);
   - rejects `[{rule: ValidFor(3mo), action: delete}]`.
2. `U(required)` makes the property required; `U(max(10))` without `[]` and
   `[]` over a union with an array arm fail with the typed errors from
   the allowed use-site operations section.
3. The lowered JSON Schema for the example in the Lowering section has `type: array`,
   `minItems: 1`, and `items.anyOf` with one entry per flattened arm.
4. Per-walker tests through a union item, each failing if the walker reads the
   `anyOf` wrapper blindly: validate (problem description / node lookup for
   `/content_policy/1/rule`), clean (an untagged object item with an extra
   key and a literal-tagged object item with an extra key both fail every
   arm; cleanup leaves both items unchanged, deletes no key, and reports
   their problems, while a repairable non-union sibling property is still
   repaired), rewrite
   (an unambiguous eager `file` arm inside a union item is rewritten),
   coerce (a `boolish`/`numberlike` arm inside a union item coerces),
   discriminant (a literal-tagged item union narrows to one arm's problems).
5. Typed-tree tests: completion inside a list item offers keys from the
   object arms and values from the scalar arms; DMLS hover on an item shows
   the matching arm; `md schema about` renders the array-of-union type.
6. Against the tree with the prerequisite already implemented, every existing
   schema that does not use the new forms lowers to byte-identical JSON Schema.
   Include the passive shipped-schema corpus and resolver fixtures. Do not
   compare against pre-prerequisite presence stripping, which intentionally
   changes lowering.
7. Additional boundary coverage: all three reference forms and multi-hop
   imports; inherited required/generated/eager; use-site descriptions and
   arm descriptions; pre-array versus container constraints; `min`/`max`
   boundaries, contradictory bounds, `unique`, compatible and incompatible
   defaults; an array arm hidden behind an import; an object arm containing
   a nested array; and import cycles. Include overlapping union arms, invalid
   and conflicting literal tags, independent items selecting different arms,
   and ambiguous coercion/rewrite cases that remain unchanged. Compare each
   item with the same union used as a property wherever behavior is shared.
8. Exercise `md schema validate`, `md schema about`, and normal composition
   through the CLI with a hermetic fixture. Assert validation leaves input
   unchanged, composition retains the documented coercion rules, and repeated
   normalization is idempotent. Include DMLS parser, completion, hover, and
   schema-error tests; resolving a local named reference for display does not
   require the deferred anonymous-union grammar.
9. `just test` and `just lint` pass in `darkmatter/`. Tests are L1, use no
   terminal or browser windows, and are portable to macOS, Linux, Windows, and
   WSL2. No new CI cells.

## Open questions

None.
