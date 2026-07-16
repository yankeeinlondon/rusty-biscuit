---
$schema: "@.claudine/schemas/review.yaml"
ready: true
agent: unknown/default
created: 2026-07-11T21:26:12-07:00
---

# Review: Real Error Messages (Iteration 3)

## Verdict

Ready for production. No implementation or test-coverage findings remain for
this feature. The iteration-2 dispatch-inventory blocker is fixed, and the
feature-specific Level 1 integration test plus the library unit matrix verify
the complete structured-error propagation and message-building behavior.

## Findings

None.

## Prior Findings Rechecked

The committed provider dispatch inventory now matches the generated inventory;
both dispatch guard tests pass. The pure `failure_message` builder still applies
one final hygiene pass to every cascade branch, reserves space for retry suffixes,
and returns a single escape-free line capped at 240 characters. Structured
provider errors are copied from `StreamExecutionSummary` into `AttemptOutcome`
and reach both `failure.err.msg` and failed-`finalize.err.msg` through the real
wrapper subprocess seam.

The implementation remains appropriately centralized in the library. Both the
lifecycle error payload and the unhandled-failure banner consume the same built
message, while `AttemptOutcome.error_kind` documents provider-semantic kinds as
well as wrapper guard and timeout labels.

## Requirement-to-Verification Matrix

| User-facing requirement | Strongest relevant verification | Assessment |
|---|---:|---|
| Structured provider text becomes `failure.err.msg` and failed-`finalize.err.msg` | Level 1 real subprocess integration | Appropriate and passing |
| Guard trips describe the guard and key parameters | Level 1 table-driven library tests | Appropriate and passing |
| Provider message, guard, timeout, stderr, and fallback precedence | Level 1 table-driven library tests | Appropriate and passing |
| Attempt suffix is absent on attempt 1 and present after retry for every cascade source | Level 1 table-driven library tests | Appropriate and passing |
| Every message is single-line, escape-free, and at most 240 characters | Level 1 focused hygiene tests across externally sourced branches | Appropriate and passing |
| Generic fallback retains the exit code | Level 1 unit tests and existing Level 2 lifecycle capture | Appropriate and passing |
| Provider dispatch inventory remains synchronized | Level 1 drift-guard integration tests | Appropriate and passing |

No requirement in this feature depends on terminal-emulator rendering or input
encoding. Level 2 and Level 3 testing is therefore not required for the new
message-data behavior. Existing Level 2 lifecycle coverage is supplementary;
there is no keyboard, paste, IME, mouse, glyph-width, SGR, or scrolling contract
in the specification.

## Verification Performed

- `cargo nextest run -p claudine-cli --test dispatch_inventory --test level1_structured_error_message --color never`: 13 passed, including both dispatch inventory guards and the structured-error end-to-end regression.
- `just test` completed the `claudine-catalog-types` suite (21 passed), `claudine` suite (3,395 passed, 7 skipped), and `claudine-contract` suite (47 passed, 5 skipped).
- The full CLI portion of `just test` was bounded in this non-interactive review session: 289 tests passed before the run was intentionally interrupted, with no test assertion failures. The feature-specific CLI tests and the prior release blocker were run separately to completion as reported above.

Static review also confirmed every `AttemptOutcome` construction path initializes
the new fields, structured execution threads `error_message`, timeout duration is
selected from the rule that fired, and capture/interactive paths intentionally
leave stream-only data absent.
