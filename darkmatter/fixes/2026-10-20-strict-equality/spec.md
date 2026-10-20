---
kind: fix
area: darkmatter
status: draft-spec
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
    implemented: boolean -> indicates whether this spec's plan has been implemented
created: 2026-10-20
owner: Ken Snyder <ken@ken.net>
packages:
    - darkmatter
    - claudine
related:
    - 2026-10-16-type-narrowing
    - 2026-10-28-schema-enhancements
---

# Strict equality for `==` and `!=`

Darkmatter's expression evaluator compares `==` and `!=` as rendered text.
Both sides are turned into strings first (`scalar_string`, where `null`
renders as `""`), with one special case: when both sides are `null`, both
operators return `false`
([`expression/mod.rs`](../../lib/src/markdown/compose/expression/mod.rs),
the `Expr::Comparison` arm). That produces results an author does not expect:

| Expression | Today |
| --- | --- |
| `5 == "5"` | `true` |
| `null == ''`, `'' == null` | `true` |
| `null == null` | `false` |
| `null != null` | `false` |

The author ruled on 2026-10-20, while reviewing `2026-10-16-type-narrowing`,
that equality is strict: two values are equal only when they have the same
type and the same value. Conversion is explicit (`number("5")`). A missing
value is `null`; `null` is equal to itself and to nothing else.

## Decision

| Expression | Result |
| --- | --- |
| `5 != "5"` | `true` |
| `5 == number("5")` | `true` |
| `null == null` | `true` (identity) |
| `null != ''`, `'' != null` | `true` |
| `true == "true"` | `false` |

`!=` is always the exact complement of `==`.

## Consequences

- **The evaluator.** The `ComparisonOp::Equal`/`NotEqual` arms stop comparing
  `scalar_string` text and drop the `both_null` special case.
- **Type narrowing follows automatically.** `TypeScope`'s equality facts
  (`expression/scope/facts.rs`, `compare_with`) mirror the evaluator and are
  tested against it by the differential module in
  `lib/tests/l1/expression_type_scope.rs`. Under strict equality:
    - `x == null` proves `x` is `null` on its true side and present on its
      false side; `x != null` the reverse.
    - `x != ''` no longer proves presence (a `null` `x` passes it). Its true
      side proves nothing about presence; its false side proves `x` is `''`.
    - `x == 'a'` proves `x` is the string `'a'`, not merely present.
- **Docs.** The guard table in `docs/topics/darkmatter-expressions.md` (rows
  for `x != ''`, `x != null`, `x == ''`, `x == y`) and every page that
  describes `==` as text comparison are rewritten to the new rule.

## Migration

Presence checks written as text comparison change meaning silently, so they
are rewritten, not left to drift:

- `x != ''` used to mean "present and nonempty" becomes `!is_empty(x)`.
- `x == ''` used to mean "missing or empty" becomes `is_empty(x)`.
- `x != null` / `x == null` keep their spelling; under strict equality they
  mean exactly "present" / "missing".

Audit every shipped prompt, skill, schema example, test fixture, and doc in
the repository for `==`/`!=` against `''`, `""`, `null`, or a number written
as a string, and rewrite each one to what its author meant. The frozen prompt
copies under `claudine/cli/tests/fixtures/frozen_prompts/` are migrated with
their originals.

## Open Questions

The author has not ruled on these yet. Each lists the recommended answer
first.

1. **Numbers that differ only in spelling.** Recommended: `5 == 5.0` is
   `true`; both are the number five. The alternative, comparing JSON
   spelling, would make the result depend on how YAML parsed the value.
2. **Arrays and objects.** Recommended: structural equality, so
   `[1, 2] == [1, 2]` and `{a: 1} == {a: 1}` are `true`, element by element
   and key by key under the same strict rule, with object key order ignored.
   The alternative is that arrays and objects are never equal to anything,
   which makes `==` useless for them without making it safer.
3. **Ordering operators.** Recommended: `<`, `>`, `<=`, `>=` keep today's
   numeric conversion; this fix changes `==`/`!=` only. The alternative,
   strict ordering (a type error or `false` across types), is a separate
   decision with its own migration.

## Acceptance

1. Every row of the Decision table holds through the public expression API,
   in `when:` conditions, ternaries, and `{{ }}` interpolation.
2. `!=` is the complement of `==` for every pair of the differential sample
   values in `lib/tests/l1/expression_type_scope.rs`.
3. The differential narrowing tests pass unchanged in structure: every guard
   side admits every value the evaluator sends there, with `x == null`,
   `x != null`, `x == ''`, and `x != ''` rows asserting the facts listed under
   Consequences.
4. The motivating commit-message prompt
   (`prompts/_implement/implement-plan.md` and its frozen copy) still passes
   pre-flight with its guard, launches nothing on the false branch, and
   passes one argument on the selected branch.
5. The migration audit is recorded in the implementation log with every
   rewritten site, and no shipped prompt, skill, or doc still uses `!= ''` or
   `== ''` as a presence check.
6. `just test` and `just lint` pass in `darkmatter/` and `claudine/`.
