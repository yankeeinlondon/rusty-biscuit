---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-13T18:19:59-07:00
spec: 2026-07-12-literal-expression/spec.md
implemented: true
description: "A **feature** review of `2026-07-12-literal-expression/spec.md`"
feature: 2026-07-12-literal-expression/review-6.md
---

# Review 6 — Literal Expression

## Verdict

Not ready for production. Both Review 5 blockers are fixed: JSON Schema `const` emission now
preserves large integers exactly, and legacy nested property unions again surface their deeper
arm errors. The new executable CLI compatibility fixture confirms the restored pretty and JSON
output, and the focused library, CLI, DMLS, and schema-specific Level 2 tests pass.

One high-severity correctness gap remains. Trigger matching still falls back to `f64` equality
for JSON numbers, so adjacent large integers can compare equal and activate the wrong trigger.
This violates the exact-value contract of `literal(value)` and acceptance criterion 7.

## Findings

### High — Large integer literal triggers can match a different integer

`triggers::matcher::literal_value_matches` first checks exact `serde_json::Value` equality, but
when that fails for two numbers it compares `a.as_f64() == b.as_f64()`. IEEE-754 cannot represent
every integer beyond `2^53`; for example, `9007199254740992` and `9007199254740993` convert to the
same `f64`. A trigger authored with `version: literal(9007199254740993)` can therefore activate for
a document whose version is `9007199254740992` (and the reverse pairing has the same risk).

This is not only a diagnostic discrepancy. A false match can merge the wrong trigger schema into
the effective schema, changing validation, coercion, completion, and DMLS diagnostics for that
document. It also leaves the feature internally inconsistent: JSON Schema validation and
discriminated-union selection now preserve exact large integers, while trigger selection does not.

Preserve exact integer comparison for `i64`/`u64` values and permit integer-versus-float equality
only when the floating value represents that integer exactly. Add Level 1 matcher and public
trigger-registry/CLI cases around `2^53 + 1`, `i64::MAX`, `i64::MAX + 1`, and `u64::MAX`, including
neighbor rejection. The current trigger test covers only `2`, `2.0`, and the string `'2'`, so it
cannot expose this boundary.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| AC1 — Literal grammar, typed scalar, canonical round trip, and typed `const` | Level 1 grammar/conversion/integration tests, including four large-integer boundaries | Appropriate and passing |
| AC2 — Optional/required equality and self-validating default | Level 1 library tests, including exact large-integer defaults | Appropriate and passing |
| AC3 — Literal coercion with pending and validation guards | Level 1 library/coercion tests, including exact large-integer string coercion | Appropriate and passing |
| AC4 — Mixed literal/property unions | Level 1 library validation tests | Appropriate and passing |
| AC5 — Expression either-dialect parsing, format failure, pending behavior, and passivity | Level 1 library plus DMLS no-side-effects coverage | Appropriate and passing; no external encoder or renderer is involved |
| AC6 — Expression scalar coercion and mapping/sequence rejection | Level 1 library tests | Appropriate and passing |
| AC7 — Trigger matching | Level 1 matcher tests | **Gap:** the tier is appropriate, but large-integer neighbor rejection is absent and the implementation is incorrect |
| AC8 — `schema about` rows and legacy validation output | Level 1 spawned-binary tests plus Level 2 real-tmux rendering tests | Appropriate and passing; the new nested-union pretty/JSON fixture restores the missing compatibility shape |
| AC9 — DMLS completion, hover, diagnostics, code action, and decoded scalar ranges | Level 1 provider/diagnostic/LSP tests | Appropriate and passing for protocol behavior; no terminal rendering or OS input encoder is involved |
| AC10 — Unambiguous discriminant selection and unresolved union behavior | Level 1 selector, validation, and DMLS provider tests | Appropriate and passing; the Review 5 compatibility regression is fixed |
| AC11 — Complete area L1/L2 gates | Canonical recipes attempted; focused suites completed | Not fully re-established in this review: the full recipes exceeded the non-interactive 60-second ceiling, though the relevant focused suites and schema-about L2 slice passed |

No requirement involves keyboard, mouse, paste, or IME input, so Level 3 verification is not
applicable. No additional ergonomics or performance concern was found that should block release;
the remaining blocker is exact trigger correctness and its missing boundary coverage.

## Verification performed for this review

- Inspected the complete specification, Reviews 1–5, the Review 5 fixes, current uncommitted
  compatibility fixture, schema conversion/coercion/reporting paths, trigger matching, and DMLS
  feature surfaces.
- `cargo nextest run -p darkmatter --test schemas_literal_expression -E 'all()' --color never`:
  43 passed.
- Focused Darkmatter trigger matcher selection: 3 passed. These tests demonstrate the coverage
  gap because they exercise only small numeric values.
- `cargo nextest run -p darkmatter-cli --test schema_validate_baseline --test schema_about
  --color never`: 14 passed, including the new legacy nested-union fixture.
- `cargo nextest run -p dmls -E
  'test(/root_union|literal|expression|discriminated|unresolved_union/)' --color never`: 84 passed,
  485 skipped by the focused filter.
- `just _test_l2 darkmatter-cli --test level2_schema_about --color never`: 3 passed in real tmux.
- Full `just test` was terminated during a cold dependency build at the 60-second non-interactive
  ceiling; no full-area L1 result is claimed.
- Full `just test-l2` completed Darkmatter's 19 tests and 11 CLI tests before the same ceiling; it
  was interrupted before the remaining CLI and DMLS tests, so no complete area L2 result is
  claimed.
- `git diff --check` is currently blocked by pre-existing trailing whitespace in
  `prompts/merge-conflicts.md`, outside this feature and untouched by this review.
- Review-frontmatter validation could not run because `schemas/feature-review.yaml` is rejected
  by the current standalone-schema parser: its tagged form contains unsupported `description`
  and `$schema` keys. The requested review frontmatter was retained exactly.
- GitNexus index refresh was attempted because the worktree index was stale, but it made no
  progress within the non-interactive command ceiling and was terminated. Source inspection and
  executable tests were used rather than treating stale graph omissions as evidence.
