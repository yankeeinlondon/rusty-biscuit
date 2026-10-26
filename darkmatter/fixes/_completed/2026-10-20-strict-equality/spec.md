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
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-10-20
review_iterations: 1
completed: true
created: 2026-10-20
owner: Ken Snyder <ken@ken.net>
packages:
    - darkmatter
    - claudine
related:
    - 2026-10-16-type-narrowing
    - 2026-10-28-schema-enhancements
implemented: true
implemented_by: "claude/opus"
---

# Strict equality for `==` and `!=`

Darkmatter's expression evaluator currently compares `==` and `!=` as
rendered text. Both sides are turned into strings first, with `null`
rendering as `""`. When both sides are `null`, both operators return `false`.
The implementation is the comparison branch in the darkmatter package's
[expression evaluator](../../lib/src/markdown/compose/expression/mod.rs).
This produces surprising results:

| Expression | Today | After this fix |
| --- | --- | --- |
| `5 == "5"` | `true` | `false` |
| `null == ''`, `'' == null` | `true` | `false` |
| `null == null` | `false` | `true` |
| `null != null` | `false` | `false` |
| `[] == '[]'` | `true` | `false` |
| `true == "true"` | `true` | `false` |

The author ruled on 2026-10-20, while reviewing `2026-10-16-type-narrowing`,
that equality is strict: two values are equal only when they have the same
runtime type and the same value. Conversion is explicit (`number("5")`).
A missing document value resolves to `null`; `null` equals itself and
nothing else.

## Decision

Equality operates on evaluated values, before rendering them as text. Runtime
types here are null, boolean, number, string, array, and object; schema types
such as `email` remain strings at runtime. Schema coercion already completed
before a comparison remains effective: strict equality adds no coercion of
its own.

| Expression | Result |
| --- | --- |
| `5 != "5"` | `true` |
| `5 == number("5")` | `true` |
| `5 == 5.0` | `true` |
| `null == null` | `true` |
| `null != ''`, `'' != null` | `true` |
| `0 == false` | `false` |
| `'A' == 'a'` | `false` |
| `' a ' == 'a'` | `false` |
| `[1, null] == [1.0, null]` | `true` |
| `[1, 2] == [2, 1]` | `false` |
| `{a: 1, b: null} == {b: null, a: 1.0}` | `true` |
| `{a: null} == {}` | `false` |
| `[] == {}` | `false` |

For successfully evaluated operands, `!=` is always the exact complement of
`==`. Operand evaluation order and failures remain unchanged. An unavailable
host binding, malformed expression, or failed function call remains an error;
it does not become a missing value or an unequal comparison.

### Numbers and containers

**Review decision:** adopt numeric value equality and recursive container
equality. These resolve the draft's spelling and container questions as part
of this proposed design; they are not additional historical author rulings.

- Integer and floating-point storage represent one runtime number type.
  Compare the represented numeric values exactly, without an approximate
  tolerance. `5` and `5.0` match; positive and negative zero match.
- Compare signed and unsigned integers without loss. For integer/float pairs,
  equality requires the floating-point value to represent that exact integer
  within its range. Do not convert all numbers to floating point: that could
  make the distinct supplied integers `9007199254740992` and
  `9007199254740993` equal.
- Compare strings exactly, without trimming, case folding, or Unicode
  normalization. Booleans compare only to booleans.
- Arrays require equal lengths and equal corresponding elements. Objects
  require the same keys and equal corresponding values, independent of key
  order. Apply the same strict rules recursively, including numeric equality.
  A missing object key is different from a key containing `null`.

The current darkmatter parser stores numeric literals as floating point, and
its evaluator converts integral literals to signed integers. This fix does
not redesign number parsing or arithmetic: equality compares the values those
stages actually produced, not the original spelling. Test large numbers
through supplied typed values as well as ordinary literals so parser rounding
cannot hide a comparison defect. Non-finite numbers are not an additional
supported value type.

**Reader's note:** raw `serde_json::Value` equality alone does not establish
this contract: integer and float storage may differ even when the intended
numeric value is equal. Text serialization also cannot establish it. Keep
one typed comparison implementation in darkmatter, reused by the evaluator
and finite-value analysis; do not add a separate Claudine or editor evaluator.
Structural comparison should stop at the first mismatch and avoid allocating
serialized copies. No performance spike is required for this change.

### Scope boundaries

Ordering operators (`<`, `>`, `<=`, `>=`) retain their existing numeric
conversion behavior. Arithmetic, truthiness, fallback, rendering, schema
literal validation, and the grammar are unchanged. Consequently, ordering
does not imply strict equality: `5 <= "5"` and `5 >= "5"` can both hold while
`5 == "5"` is false. Document that distinction explicitly.

This intentionally replaces the existing expression equality contract; it
does not replace every use of equality in the repository. Do not change Rust
comparisons, JSON Schema `const`/`enum` validation, or unrelated template
languages during the migration.

## Evaluator and type analysis

In the darkmatter package, update the equality branches of the
[expression evaluator](../../lib/src/markdown/compose/expression/mod.rs)
and remove their both-null exception. Keep text rendering available for
interpolation and other existing consumers.

Type narrowing does **not** follow automatically from changing the evaluator.
The darkmatter package's [guard analysis](../../lib/src/markdown/compose/expression/scope/facts.rs)
selects the types a value can have on each branch; its
[type shapes](../../lib/src/markdown/compose/expression/scope/shape.rs)
currently narrow by rendered text. Replace those equality rules together,
including comparisons with a finite set of possible values on the other side.
For an otherwise unconstrained `x`, the required facts are:

| Guard | True side | False side |
| --- | --- | --- |
| `x == null` | null only | present (non-null) |
| `x != null` | present (non-null) | null only |
| `x == ''` | empty string | no general presence proof |
| `x != ''` | no general presence proof | empty string |
| `x == 'a'` | the string `'a'` | no general presence proof |
| `x != 'a'` | no general presence proof | the string `'a'` |

Intersect these facts with the declared type and preserve its constraints.
Reversed operands and negated guards have the same corresponding facts.
For `x == y`, retain every declared alternative that could strictly equal a
possible value on the other side; a singleton `null` is now a usable finite
set too. Remove all special routes introduced only for the old both-null
exception. Unknown or non-finite other-side types may remain conservative.

When equality holds against an array or object expression, the subject is
present and has that container family. Full container-value narrowing is not
required. Never evaluate a function, read a file, or fetch data to obtain a
static fact; a computed operand may remain unknown. Literal-number analysis
must model the evaluator's actual literal value, including its integral
conversion, rather than a different storage representation.

The analyzer must admit every live value the evaluator sends down a branch.
In particular, if a numeric literal shape would reject an equivalent numeric
storage representation under existing schema validation, retain a broader
number shape instead of changing schema validation or rejecting the live
value. Preserve the existing conservative treatment of contradictory guards;
this fix does not add dead-code elimination.

The same shared analysis must reach shell preflight, conditional body blocks,
transclusion targets, absence warnings, Claudine lifecycle and sequence
conditions, and DMLS (the Markdown language server) diagnostics and hover.
For example, `x != ''` must no longer excuse a missing shell placeholder or
nullable transclusion target. `x != null` must provide that presence evidence.
Existing runtime checks still apply when a value changes after a guard.

## Migration

This is an intentional breaking behavior change, without a compatibility
mode. Audit authored Darkmatter expressions throughout shipped prompts,
skills, schema examples, fixtures, and documentation. Include variable-to-
variable comparisons, booleans against strings, and containers against JSON
text, not just comparisons against empty strings and null.

Choose replacements from the intended behavior and declared type:

| Intended check | Expression after the fix |
| --- | --- |
| Missing or explicitly null | `is_null(x)` or `x == null` |
| Present, including empty values | `!is_null(x)` or `x != null` |
| Exactly an empty string | `x == ''` |
| Missing or an empty string, but not an empty container | `is_null(x) || x == ''` |
| Present and not an empty string, allowing empty containers | `x != null && x != ''` |
| Null or any empty string, array, or object | `is_empty(x)` |
| Neither null nor any empty string, array, or object | `!is_empty(x)` |

For a string-or-null input, `is_empty(x)` and `!is_empty(x)` are convenient
migrations for the old empty-text checks. They are not general replacements:
the darkmatter package's [is_empty](../../lib/src/markdown/compose/expression/functions/mod.rs)
predicate also treats `[]` and `{}` as empty, whereas the old equality
compared their nonempty JSON text. Whitespace-only strings remain nonempty.
The old `x != null` also rejected empty strings, and the old `x == null`
matched empty strings but rejected null; audit their intent rather than
assuming the spelling can stay unchanged.

For intentional rendered-text comparison, use explicit existing string
concatenation, for example `('' + x) == ('' + y)`. This deliberately restores
text comparison for that site, including null rendering as empty text; it
does not restore the old both-null exception. For intentional numeric
comparison against text, convert explicitly with `number(...)`, preserving
its failure behavior. Retain `== ''` and `!= ''` where exact string comparison
is intended.

Migrate corresponding frozen prompt copies together with their originals,
including both copies of `_implement/implement-plan.md` under the Claudine
CLI's `tests/fixtures/frozen_prompts/`. The current commit-message guard is
`ctx.dirty_files && commit_message`; it already checks truthiness and does
not require an equality rewrite. Preserve it as a regression case rather
than inventing a change to that prompt.

Record inspected locations, rewritten sites, and intentionally retained
comparisons with their reasons in the implementation log. Historical specs
remain snapshots; do not rewrite them as part of the audit.

## Documentation and skill updates

Update the darkmatter package's
[expression reference](../../docs/topics/darkmatter-expressions.md), including
the equality section, guard table, and nullable-target examples, and its
[expression engine guide](../../docs/topics/expression-engine.md).
Update the Darkmatter [expression skill](../../../.claude/skills/darkmatter/expressions.md)
and any affected preflight, Claudine lifecycle/sequence, and DMLS hover pages
that teach the former rules. Check README examples and update any that
present changed behavior; keep detailed rules in topic pages.

Show strict type examples, recursive equality, explicit conversions, changed
presence checks, and unchanged ordering conversion. Current docs describe
the implemented behavior and never link back to this spec. Update them with
the implementation, not during this draft review.

## Open Questions

None remain in this reviewed proposal. Numeric value equality, structural
container equality, and unchanged ordering are explicit review decisions
above, ready for the author's review of the draft.

## Acceptance

1. All comparison examples above hold through the public expression API.
   Include missing versus explicit null; both operand orders; booleans,
   numbers, strings, empty and nested containers; object key order; missing
   keys versus null values; integer/float pairs; signed/unsigned boundaries;
   negative zero; and distinct large integers supplied without parser rounding.
   Check reflexivity, symmetry, and the exact `!=` complement for successful
   comparisons over the sample pairs.
2. Add expected-result assertions independent of the comparator. The
   darkmatter package's [differential scope tests](../../lib/tests/l1/expression_type_scope.rs)
   must still verify that every runtime branch admits its actual values;
   update expectations and extend samples for the new semantics. Check both
   sides of each presence guard above, finite other-side values including
   null, unknown subjects, and container-family refinement. Sharing a helper
   must not let the evaluator and analyzer agree on the same wrong answer.
3. Extend the existing shared guard cases and applicable consumer tests to
   prove that `x != ''` cannot authorize a missing placeholder or target,
   while `x != null` can. DMLS must report and display the same facts without
   executing conditions. Preserve scope invalidation after writes and the
   established handling of undeclared-property warnings.
4. Verify representative changed comparisons through ordinary composition:
   frontmatter and body interpolation, ternaries, `::block when`, and
   conditional transclusion. Verify Claudine lifecycle and sequence conditions
   through their existing normal invocation tests. Preserve operand errors
   and skipped-branch behavior. Use frozen fixtures and the test toolkit;
   tests must not launch an agent, commit, stage files, or activate a window.
5. The motivating commit-message prompt and both frozen copies still pass
   preflight. With dirty files, null/empty messages select the generated-message
   route and a nonempty message selects the supplied-message route. With no
   dirty files neither commit route runs. A harmless command substitute
   confirms that the selected supplied-message command receives the message
   as one argument; a false branch launches no command from that branch.
6. Record the migration audit and update the affected current docs and skills.
   Cover changed shipped artifacts passively and through normal invocation;
   prompt-specific corpus checks stay opt-in through `just test-prompts`,
   never newly added to CI. Retained exact empty-string comparisons are valid.
7. For implementation verification, run `just test` and `just lint` in
   `darkmatter/` and `claudine/`, plus the affected opt-in prompt checks locally.
   These semantic cases belong in L1 and require no new terminal/browser tier,
   CI matrix, benchmark suite, or OS-specific comparison behavior. This draft
   review itself changes only this specification and requires no Rust build.
