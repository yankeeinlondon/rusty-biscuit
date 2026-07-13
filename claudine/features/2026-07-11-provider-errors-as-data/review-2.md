---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-13T06:43:01-07:00
spec: 2026-07-11-provider-errors-as-data/spec.md
implemented: true
description: A **feature** review of `2026-07-11-provider-errors-as-data/spec.md`
feature: 2026-07-11-provider-errors-as-data/review-2.md
---

# Review 2: Provider Errors as Data

## Verdict

The feature is **not ready for production**. The prior review's source-graduation, outcome-reporting, seed-identity, and Kilo-discrimination findings have been substantially addressed. However, exhausted remediation attempts can still convert a known-bad provider document into a successful fleet result, and the exact Codex capacity incident that motivated the feature remains unclassified by the intended API-remote bucket.

## Findings

### High: exhausted remediation still allows a successful fleet result

The durable outcome report fixes the earlier ambiguity between a clean run and a checker failure, but it does not enforce the specification's terminal condition after the resume budget is exhausted.

The fleet stack resumes when the report has `status: findings`, with `max_attempts: 2`. Once that budget is exhausted, `ControlDispatch::Exhausted` becomes `TerminalControlAction::Fallthrough`; the terminal recovery path then returns `Completed`. There is no finalize guard that re-reads the outcome report and fails when findings remain. Consequently, a provider document that fails all three checker runs can leave a machine-readable findings report while the fleet sequence itself reports success. This directly violates D10's requirement that a known-bad document must not become a successful fleet result.

Add a finalize lifecycle stack that rejects a remaining `findings` or otherwise non-clean outcome after fallthrough. Verify the full lifecycle at Level 1 by keeping a research document invalid across the initial run and both resumes, then asserting both a non-successful fleet result and a preserved findings report.

### High: the motivating Codex capacity incident remains unclassified

The accepted Codex delta adds `overloaded`, and the tests demonstrate classification for a manufactured “selected model is overloaded” string. The specification's motivating incident is instead `Selected model is at capacity`. That phrase matches none of the generated Codex message needles and, without a structured `error_kind`, remains `AgentNative` rather than `ApiRemote`.

The research document acknowledges that the exact `at capacity` phrase was withheld, but also claims the accepted `overloaded` needle closes the motivating incident class. Those statements are not equivalent: the sibling wording is classified, while the cited production wording is not.

Either source-pin and add a sufficiently narrow capacity needle, with a parser-level positive fixture and a negative collision fixture, or explicitly narrow the feature's claim and leave the motivating incident as an unresolved blocker. The current helper-level test for `overloaded` does not verify the actual Codex event shape or the exact incident.

### Medium: closeout documents contradict the graduated implementation

The implementation declares research as the authoritative source, removes facts keys, generates research-backed vocabulary, and records an accepted delta. The feature's closeout documents still describe C1 as pending or blocked, state that runtime remains facts-backed, and leave the specification in draft status. This conflicts with the implemented state and leaves C3 incomplete.

Reconcile `plan.md`, `_fleet-review.md`, and the specification frontmatter with the actual checkpoint disposition. Any deliberately unresolved scope, including the exact capacity phrase, should be recorded as such rather than describing both the graduation and its motivating case as complete.

## Requirement Verification

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| Research documents are the sole vocabulary source and generated artifacts remain deterministic | Level 1: generator unit/integration tests and `claudine-gen check` | Meets the required level. |
| Generated projection preserves ordering, bucket routing, and Kimi multi-target behavior | Level 1: generator projection and vocabulary tests | Meets the required level. |
| Gate output distinguishes clean, findings, and checker failure with durable atomic reports | Level 1: gate and fleet-shape tests | Partial. The report states are covered, but the exhausted-resume fleet outcome is not. |
| Existing seed rows retain full branch, bucket, kind, item, and value identity | Level 1: table-driven generator tests against archived seeds | Meets the required level. |
| Kilo runtime identity is discriminated and invalid identities are rejected | Level 1: parser unit tests with injected vocabulary | Meets the required level. |
| Accepted Codex overload vocabulary classifies the motivating capacity incident | Level 1 helper test for `overloaded`; no parser fixture for `Selected model is at capacity` | Gap. The exact user-visible behavior is neither implemented nor verified. |
| Failed remediation remains a failed fleet result after the retry budget | No end-to-end lifecycle verification | Gap. Level 1 is required and sufficient for this workflow behavior. |
| Graduation and onboarding documentation describe the shipped source of truth | Document inspection | Gap. Several closeout documents describe the pre-graduation state. |

This feature does not expose terminal rendering, terminal input encoding, keyboard, paste, IME, mouse, or scrolling behavior. Level 2 and Level 3 tests are therefore not applicable; the appropriate rigor for its observable behavior is Level 1 process and lifecycle testing.

## Verification Performed

- `just lint` passed for all five Claudine packages and repository guards.
- Focused generator tests passed: 34 tests.
- Focused Claudine classification and fleet tests passed: 12 tests.
- `claudine-cli` dispatch inventory passed: 12 tests.
- `cargo run -q -p claudine-gen -- check` reported all generated artifacts clean.
- The broad Claudine library suite passed 3,403 tests with 7 skipped and one successful retry.
- `claudine-contract` passed 47 tests with 5 skipped; `claudine-catalog-types` passed 21 tests.
- The broad `claudine-cli` run was stopped at the non-interactive 60-second ceiling after 678 of 1,903 tests passed with no observed test failure. The remaining 1,225 tests were not executed in that run.
- `git diff --check` passed.
