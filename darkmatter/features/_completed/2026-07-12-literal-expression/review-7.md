---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-07-13T18:57:59-07:00
spec: 2026-07-12-literal-expression/spec.md
implemented: false
description: "A **feature** review of `2026-07-12-literal-expression/spec.md`"
feature: 2026-07-12-literal-expression/review-7.md
---

# Review 7 — Literal Expression

## Verdict

Ready for production. The Review 6 blocker is fixed: trigger matching now compares integer JSON
numbers exactly and crosses the integer/float representation boundary only when the float is
integral and converts to the same integer. The `2^53 + 1`, `i64::MAX`, `i64::MAX + 1`, and
`u64::MAX` regressions pass, including rejection of their adjacent integer values. Focused
Darkmatter, CLI, DMLS, and real-tmux tests also pass.

No correctness, ergonomics, portability, or performance issue remains that should block release.
One non-blocking integration-test depth gap remains in the trigger regression.

## Findings

### Medium — The large-integer trigger regression bypasses authored trigger loading

The new matcher unit test and
`schemas_literal_expression::trigger_matcher_large_integer_literal_is_exact` both construct a
`MatchExpr::Property` and `PropertyAtom` directly. They verify the corrected equality function and
the public `triggers::matches` entry point, but they do not pass an authored
`version: literal(...)` condition through `parse_match_arms`, a trigger envelope/registry, or
`md schema triggers`. Review 6 specifically requested a public trigger-registry or CLI case in
addition to matcher coverage.

This is not evidence of a remaining implementation defect: large-integer `literal(...)` parsing
is independently covered through the shared type-expression parser, and trigger grammar delegates
directly to that parser. It is nevertheless weaker integration protection than requested and
would not catch a future disconnect between trigger-envelope parsing and matcher evaluation.

Add one Level 1 authored-trigger integration fixture that loads a trigger schema and verifies that
an exact boundary integer activates it while its adjacent integer does not. Covering the four
existing boundaries in a table-driven registry test is sufficient; a second CLI assertion is
optional if the registry test exercises the same discovery/load path.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| AC1 — Literal grammar, typed scalars, canonical round trip, and typed `const` | Level 1 grammar/conversion/integration tests, including four large-integer boundaries | Appropriate and passing |
| AC2 — Optional/required equality and self-validating defaults | Level 1 validation tests, including exact boundary defaults | Appropriate and passing |
| AC3 — Literal coercion with pending and validation guards | Level 1 compose/coercion tests, including exact large-integer string coercion | Appropriate and passing |
| AC4 — Mixed literal/property unions | Level 1 library validation tests | Appropriate and passing |
| AC5 — Expression either-dialect parsing, format failure, pending behavior, and passivity | Level 1 library and DMLS no-side-effects tests | Appropriate and passing; no renderer or external input encoder is involved |
| AC6 — Expression scalar coercion and mapping/sequence rejection | Level 1 library tests | Appropriate and passing |
| AC7 — Trigger matching | Level 1 matcher and public `matches` tests | Correct tier and behavior; authored envelope/registry integration remains the medium-depth gap above |
| AC8 — `schema about` rows and legacy validation output | Level 1 spawned-binary tests plus Level 2 real-tmux rendering tests | Appropriate and passing |
| AC9 — DMLS completion, hover, diagnostics, code action, and decoded scalar ranges | Level 1 provider, diagnostic, and LSP tests | Appropriate and passing; this is protocol behavior, not terminal rendering |
| AC10 — Unambiguous discriminant selection and unresolved-union behavior | Level 1 selector, validation, and DMLS tests | Appropriate and passing |
| AC11 — Complete area L1/L2 gates | Focused L1 suites and the feature-relevant L2 slice passed; the full L1 recipe was bounded by the non-interactive ceiling | Feature-scoped evidence is green; a complete current area run was not re-established in this review |

No requirement concerns keyboard, mouse, paste, IME, or terminal input encoding, so Level 3 is not
applicable. Level 2 is needed only for the user-visible `schema about` terminal presentation and is
present through the real-tmux capture suite.

## Verification performed for this review

- Inspected the complete specification, Reviews 1–6, the current Review 6 fix, the trigger grammar
  and matcher, the feature acceptance suite, CLI compatibility fixtures, and DMLS feature paths.
- `cargo nextest run -p darkmatter --test schemas_literal_expression --color never`: 44 passed.
- Focused Darkmatter matcher/conversion filter: 5 passed, including the exact large-integer and
  integer/float-equivalence cases.
- `cargo nextest run -p darkmatter-cli --test schema_validate_baseline --test schema_about
  --color never`: 14 passed. One leaked-handle retry did not reproduce in three subsequent
  retry-disabled runs.
- `cargo nextest run -p dmls -E
  'test(/root_union|literal|expression|discriminated|unresolved_union/)' --color never`: 84 passed,
  485 skipped by the focused filter.
- `just _test_l2 darkmatter-cli --test level2_schema_about --color never`: 3 passed in real tmux.
- Full `just test --color never` was terminated at the non-interactive command ceiling after
  2,123 of 5,602 Darkmatter tests passed with no failures; the CLI and DMLS packages were not
  reached, so no complete current area L1 result is claimed.
- `git diff --check` passed for the requested spec/review outputs. Review-frontmatter validation
  remains blocked because `schemas/feature-review.yaml` is rejected as a standalone tagged schema:
  it contains unsupported `description` and `$schema` keys. The requested frontmatter is retained
  exactly.
- GitNexus identified the relevant matcher and DMLS symbols, but its worktree index predates the
  fix; current source inspection and executable tests were used for the final assessment.
