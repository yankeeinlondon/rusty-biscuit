---
kind: fix
created: 2026-11-14
status: draft-spec
area: simplified-schema
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-14-edit-action
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
reviewed: false
review_iterations: 0
---

# A missing key and `null` are the same value

## Why this exists

In YAML a key that is not written reads as `null`. SimplifiedSchema treats
the two differently today
([validation](../../docs/topics/validation.md#required-properties-with-a-default),
[conversion](../../docs/topics/conversion.md#conversion-rules)):

| Declaration | `{}` today | `{ "n": null }` today |
|---|---|---|
| `n: number(default(3))` (optional) | passes; nothing filled, "the default stays metadata" | passes |
| `n: number(required; default(3))` | `n = 3` filled | rejected |
| `n: number(non-null)` (optional) | passes | rejected |

Composition, meanwhile, sometimes materializes an absent optional property
as `null`, so the system already treats them as the same in one place and
as different in others.

The review of `2026-11-14-edit-action` surfaced it: Darkmatter's lifecycle
parser refuses `no_error: null` while every action's schema
(`no_error: boolean`) accepts it, and Phase 6 of that feature added
`non-null` markers to `edit`'s flags to make the schema refuse `null` too.

## Author rulings (2026-11-14)

- **A missing key and an explicit `null` are the same value, `null`,**
  for reading, defaults, and type checks.
- **`required` is the one place they differ, and only for the required
  contract:** the contract asks whether the key was written. `n: null`
  meets it; a missing `n` does not (unless a default supplies it). Once the
  contract is met, the value is judged by its type like any other.
- **A default replaces a `null` value,** from a missing key or an explicit
  `null`, on optional and required properties alike. A default also meets
  `required` for a missing key.
- **`null` fails a type that does not admit it.** `n: number(required)` with
  `n: null` meets `required` and fails `number`.

| Declaration | key missing | `key: null` |
|---|---|---|
| `n: number` | `null`, valid | `null`, valid |
| `n: number(default(3))` | `3` | `3` |
| `n: number(required)` | error: required key missing | error: `null` is not a number |
| `n: number(required; default(3))` | `3` | `3` |

- **DRY and no contradictory guidance:** one implementation of the rule in
  the `simplified-schema` crate; every skill and doc that describes defaults,
  `required`, `null`, or `non-null` says the same thing after the change.

## Requirements

1. Strict and soft validation, conversion, and every consumer (Darkmatter
   frontmatter, lifecycle schemas, trigger payloads, DMLS) apply the table
   above. Soft validation fills an optional property's default; check every
   consumer that writes back a converted copy (composition) for the effect
   of newly filled defaults.
2. `non-null` is removed: under this rule it has no meaning distinct from
   `required`. Migrate each use (schema YAML in `simplified-schema/schemas`,
   `claudine/schemas`, `darkmatter/schemas`, bench fixtures, and the Rust
   that parses or reports it) to `required` or to a default, whichever states
   the intent, and record each choice in the implementation log.
3. JSON Schema lowering expresses the rule: `required` lists the key; a type
   that does not admit `null` rejects it; an optional property admits `null`;
   a default is carried as `default`. Raw JSON Schema documents keep the
   standard's own meaning; both agree because JSON Schema's `required` is
   also only about the key being present.
4. Darkmatter's lifecycle parser follows the same rule for its own keys:
   `no_error: boolean(default(false))` on every action, `edit`'s
   `create_new: boolean(default(false))`, and `gui: boolean` (absent or
   `null` means no preference). Boolean-like text coerces (`"yes"` →
   `true`), per the conversion rules. Remove `edit`'s `non-null` markers.

## Inventory (2026-11-14)

- `non-null` appears in schema YAML under `simplified-schema/schemas/`,
  `claudine/schemas/` (`inline-compose`, `sequence`, `claudine`,
  `claudine-config`), `darkmatter/schemas/partials/lifecycle.yaml`, and the
  Claudine bench fixtures in `darkmatter/lib/benches/fixtures/`; 18 Rust files
  in `simplified-schema/lib/src` reference it.
- 17 optional properties with a `default(...)` in `darkmatter/schemas`,
  `claudine/schemas`, and `simplified-schema` change from metadata-only to
  filled.
- Docs: `simplified-schema/docs/topics/validation.md`, `conversion.md`,
  `language/types.md` (`non-null` properties); skills: `simplified-schema`,
  `darkmatter` (`schema.md`), and any user-scope skill that describes these.

## Verification

- The table above, for strict and soft validation, through `md schema
  validate`, composition, and DMLS.
- The lifecycle parser and schema validation accept and refuse the same
  `no_error`, `create_new`, and `gui` values (extend
  `lifecycle_edit_schema_parity.rs`).
- A repository search finds no `non-null` and no doc or skill describing a
  default as metadata-only or `null` as distinct from a missing key outside
  the `required` contract.
