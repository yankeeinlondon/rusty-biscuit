---
ready: true
agent: codex/default
created: 2026-06-28T08:48:06
---

# Review 2 — OpenCode Log Fix

## Findings

No blocking findings.

The prior review's two issues appear addressed:

- The 1.17.8 `message="stream error"` cap line now has wrapper-path coverage in `claudine/cli/tests/wrap_opencode.rs` via `opencode_stderr_stream_error_cap_1_17_8_forces_early_termination`. The fake child emits the captured line and then sleeps; the test passed in 1.428s, so it verifies early process termination rather than merely parser classification.
- The repeated stream-error backstop now tracks a stable fingerprint (`providerID`, `modelID`, `session.id`, and normalized provider error text), resets on fingerprint changes, and includes tests for distinct errors and mid-run fingerprint changes. This matches the spec's "identical stream error" condition.

## Coverage Map

- Captured OpenCode 1.17.8 `message="stream error"` + `error.error` classification: Level 1 unit, present (`classifies_1178_stream_error_usage_cap`).
- `reset_at`, `provider_id`, `model_id`, and `provider_error` extraction: Level 1 unit, present in the same classification test.
- `message="stream"` call-start regression: Level 1 unit, present (`classifies_1178_stream_start_as_llm_call`).
- Legacy `service=llm error={JSON}` usage-cap fixtures: Level 1 unit, present.
- Semantic bridge terminal event and early-termination signal for the new shape: Level 1 unit, present (`opencode_1178_stream_error_usage_cap_terminates`).
- Wrapper process terminates on the first new-shape cap error: Level 1 process integration, present (`opencode_stderr_stream_error_cap_1_17_8_forces_early_termination`).
- Repeated unrecognized stream-error backstop: Level 1 unit, present for identical repeats, step-advance reset, distinct errors, and fingerprint-change reset.

No Level 2 or Level 3 verification is required for these requirements: the observable behavior under review is parser classification, semantic bridge emission, and process termination. It does not depend on terminal rendering, terminal input encoding, or OS keyboard injection.

## Verification Run

Ran:

```sh
cargo nextest run -p claudine stream::logs::opencode --color=never
cargo nextest run -p claudine-cli opencode_stderr_stream_error_cap_1_17_8_forces_early_termination --color=never
```

Results:

- `145` OpenCode log tests passed, `2865` skipped.
- `1` wrapper integration test passed, `1903` skipped.

## Production Readiness

Ready for production. The implemented behavior satisfies the spec acceptance criteria at the appropriate verification level.
