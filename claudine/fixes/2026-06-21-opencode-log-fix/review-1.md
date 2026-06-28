---
ready: false
agent: codex/default
created: 2026-06-28T08:27:16
implemented: true
---

# Review 1 — OpenCode Log Fix

## Findings

### High — Missing wrapper-level proof that the new cap line terminates the child process

The implementation has good Level 1 unit coverage for parsing and bridge emission: `classifies_1178_stream_error_usage_cap` proves the new line shape classifies as `UsageCap`, and `opencode_1178_stream_error_usage_cap_terminates` proves `OpenCodeLogBridge::ingest` emits a terminal semantic error plus an early-termination channel signal. That covers the parser/bridge contract, but it does not verify the user-facing acceptance criterion that “the wrapper terminates on the first cap error” (`spec.md` acceptance criterion 2).

The missing piece is a Level 1 process integration test with a fake `opencode` executable that emits the captured 1.17.8 stderr line, then continues sleeping or retrying. The test should run the real wrapper path and assert the process exits promptly/non-zero with `error_kind = usage_limit_reached` or equivalent summary evidence, rather than waiting for retries. Existing tests stop at the bridge channel in `claudine/lib/src/stream/logs/opencode/reasoning.rs:1355`; the wait-loop mapping exists in `claudine/cli/src/commands/wrap/exec/termination.rs`, but this specific new line shape is not exercised end-to-end.

Verification level: strongest present for this requirement is Level 1 unit/bridge. Required is Level 1 process integration because the observable behavior is wrapper termination, not terminal rendering or keyboard input.

### Medium — Backstop does not enforce the spec’s “identical stream error” condition

The spec calls for a guard over “consecutive identical `message=\"stream error\"` records” so retry-driven repeats cannot keep resetting silence forever (`spec.md:151`). The implementation increments a single counter for any stream-error record, regardless of whether the provider error text, provider/model, session, agent, or raw line changed (`reasoning.rs:317`).

That means five distinct non-terminal stream errors in one step can trip `RepeatedStreamError`, even though the specified hang pattern is repeated identical failures under OpenCode backoff. If the intended behavior is broader than the spec, update the spec and tests to say “consecutive stream errors” rather than “identical.” If the spec is authoritative, track the last stream-error fingerprint and reset the counter when it changes. The current test only repeats one constant line (`reasoning.rs:1386`), so it does not catch this distinction.

Verification level: Level 1 unit coverage exists for repeated identical synthetic errors and step-reset behavior, but not for mixed stream-error sequences.

## Coverage Map

- Captured 1.17.8 `message="stream error"` + `error.error` classification: Level 1 unit, present (`errors.rs:902`).
- `reset_at`, `provider_id`, `model_id`, `provider_error` extraction: Level 1 unit, present (`errors.rs:907`).
- `message="stream"` call-start regression: Level 1 unit, present (`errors.rs:936`).
- Legacy `service=llm error={JSON}` usage-cap fixtures: Level 1 unit, present.
- Semantic bridge terminal event and early-termination signal for new shape: Level 1 unit, present (`reasoning.rs:1355`).
- Wrapper process terminates on first new-shape cap error: not directly verified; needs Level 1 process integration.
- Repeated unrecognized stream-error backstop: Level 1 unit, present for identical lines only (`reasoning.rs:1389`).

## Verification Run

Ran:

```sh
cargo nextest run -p claudine stream::logs::opencode --color=never
```

Result: 143 tests passed, 2865 skipped.

## Production Readiness

Not ready for production yet. The core parser and bridge fix looks sound, but acceptance criterion 2 needs a real wrapper-path test for the new line shape, and the repeated-error backstop should either match the spec’s “identical” condition or the spec should be adjusted to the broader implemented behavior.
