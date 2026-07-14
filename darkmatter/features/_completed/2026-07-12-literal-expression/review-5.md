---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-13T17:06:04-07:00
spec: 2026-07-12-literal-expression/spec.md
implemented: true
description: "A **feature** review of `2026-07-12-literal-expression/spec.md`"
feature: 2026-07-12-literal-expression/review-5.md
---

# Review 5 — Literal Expression

## Verdict

Not ready for production. Both Review 4 blockers are fixed: unresolved root-union shared
properties now merge every arm rather than selecting the last arm, and the spawned
`md schema about` test verifies every public type descriptor plus explicit `literal` and
`expression` rows. The focused Level 1 library, CLI, and DMLS suites for those fixes pass.

Two high-severity gaps remain. Large integer literals lose precision while compiling to JSON
Schema, breaking the defining exact-value contract. In addition, the current `build_problems`
change alters diagnostics for legacy nested unions that do not use either new type, violating the
specification's byte-identical compatibility requirement. The checked-in compatibility fixtures
do not cover that union shape, so they remain green while the public output drifts.

## Findings

### High — Large integer literals are rounded before becoming JSON Schema `const` values

The grammar correctly parses a bare literal number through `serde_json::Number::from_str`, which
can preserve an integer beyond JavaScript's exact `f64` range. `literal_fragment` then passes that
exact `Number` through `normalize_json_number`. That helper calls `as_f64()` for every number and,
when the rounded float looks integral, replaces the original number with `f as i64`. This changes
the value the schema author wrote.

A current-binary reproduction with:

```yaml
$schema:
  version: literal(9007199254740993; required)
version: 9007199254740993
```

fails validation with `9007199254740992 was expected`. The exact authored document value therefore
does not satisfy its own literal schema. Values around and above `i64::MAX` are also exposed to
float rounding and saturating float-to-integer casts. This breaks acceptance criteria 1 and 2,
can prevent criterion 3 coercion from committing the intended value, and makes large numeric
discriminants disagree between DMLS's retained SimplifiedSchema AST and library validation.

Preserve `serde_json::Number` values that are already `i64` or `u64`; only canonicalize an actual
floating representation when it is safe and necessary. Add Level 1 boundary cases for `2^53 + 1`,
`i64::MAX`, `i64::MAX + 1`, and `u64::MAX` covering parse/serialize/reparse, emitted `const`, direct
validation, string coercion, and an equal `default(...)`.

### High — General `anyOf` diagnostic flattening breaks the promised legacy-output compatibility

The current `build_problems` change restricts recursive child-error extraction to the two-arm
optional-null wrapper. Previously, every failing `anyOf` recursively surfaced errors below the
union property's path. This is a global behavior change, not a literal-discriminant-only narrowing
change.

For example, this legacy schema contains neither `literal` nor `expression`:

```yaml
$schema:
  event:
    - "{ a: string(required) }"
    - "{ b: number(required) }"
event:
  a: 1
  b: nope
```

The pre-change mapper surfaced the deeper arm failures under `/event/a` and `/event/b`. The current
binary instead emits one parent-level problem at `/event`: `{"a":1,"b":"nope"} is not valid under
any of the schemas listed in the 'anyOf' keyword`. That violates acceptance criterion 8's
byte-identical existing-schema output and criterion 10's instruction to preserve existing union
behavior when narrowing is unavailable.

Do not redefine general union diagnostics to implement D3. Keep the pre-feature `build_problems`
behavior for an unselected union; only replace problems after
`select_literal_discriminant_arm` returns one unambiguous arm. Add this nested inline-object union
to the executable pretty and JSON compatibility fixtures. The existing seven fixtures cover a
primitive property union and root unions, but no property union whose arm errors occur below the
union path.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| AC1 — Literal grammar, typed scalar, canonical round trip, and typed `const` | Level 1 grammar/conversion tests | Gap: ordinary values pass, but no integer precision boundaries; a direct current-binary reproduction fails |
| AC2 — Optional/required equality and self-validating default | Level 1 library acceptance tests | Gap: common values pass, but the compiled large-integer identity is wrong and equal boundary defaults are untested |
| AC3 — Literal coercion with pending and validation guards | Level 1 library/coercion tests | Mostly appropriate; common number/boolean cases pass, but large exact integers are unverified and inherit the incorrect `const` |
| AC4 — Mixed Literal/property unions | Level 1 library validation tests | Appropriate and passing |
| AC5 — Expression either-dialect parsing, format failure, pending behavior, and passivity | Level 1 library plus DMLS no-side-effects tests | Appropriate; focused tests pass. One pending-shell test hit leaked-handle detection once, then passed on retry and in an isolated no-retry run |
| AC6 — Expression scalar coercion and mapping/sequence rejection | Level 1 library tests | Appropriate and passing |
| AC7 — Trigger matching | Level 1 matcher tests | Appropriate; Literal equality and Expression's YAML/JSON-like string shape are covered |
| AC8 — `schema about` rows and legacy validate output | Level 1 spawned-binary tests | `schema about` is now directly and exhaustively covered; the seven compatibility fixtures pass, but omit the legacy nested-union shape whose output currently drifts |
| AC9 — DMLS completion, hover, diagnostics, code action, and decoded scalar ranges | Level 1 provider/diagnostic/LSP tests | Appropriate for non-rendering LSP behavior; the focused feature selection passes |
| AC10 — Unambiguous discriminant selection and unresolved union behavior | Level 1 selector, validation, and DMLS provider tests | Root shared-property merging is fixed in both arm orders and unresolved states; library diagnostic preservation still fails for nested legacy unions |
| AC11 — Complete area L1/L2 gates | Canonical recipes attempted | Not established in this review: `just test` and `just lint` exceeded the non-interactive command window during cold compilation; no feature-specific terminal or OS-input behavior requires Level 2/3 |

## Verification performed for this review

- Inspected the complete specification, Reviews 1–4, current uncommitted Review 4 fixes, relevant
  feature commits, and the schema/DMLS execution paths located through GitNexus.
- `cargo nextest run -p darkmatter --test schemas_literal_expression --color never`: 38 passed;
  one leaked-handle retry, followed by an isolated no-retry pass for that test.
- `cargo nextest run -p darkmatter-cli --test schema_validate_baseline --test schema_about
  --color never`: 14 passed.
- `cargo nextest run -p dmls -E 'test(/root_union|literal|expression|discriminated|unresolved_union/)'
  --color never`: 84 passed, 485 skipped by the focused filter.
- Direct `md schema validate --format json` reproductions confirmed the large-integer rounding and
  parent-level legacy nested-union diagnostic described above.
- `just test`: terminated during a cold dependency build to honor the non-interactive command-time
  limit; it did not reach a complete area result.
- `just lint`: likewise did not complete within the non-interactive command-time limit; no lint
  result is claimed.
- `git diff --check`: passed.
- Review-frontmatter schema validation could not run because the repository's
  `schemas/feature-review.yaml` is itself rejected by the current standalone-schema parser: its
  tagged form contains unsupported `description` and `$schema` keys. The requested review
  frontmatter was retained exactly.

No feature requirement concerns terminal rendering, keyboard, mouse, paste, or IME behavior, so
there is no feature-specific Level 2 or Level 3 verification requirement.
