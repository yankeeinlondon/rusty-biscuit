---
ready: false
agent: codex
model: ""
---

# Review: PID Capture for Wrapped Agentic CLIs

## Findings

### High: Kimi wire hook/action dispatch still loses both PID fields after spawn

The spec requires wrapper session lifecycle records and Claudine-controlled hook, action, log, and report contexts to include `claudine_pid` when captured, and `agent_pid` after successful provider spawn. Iteration 2 fixes this for the live semantic sink and the summary/report query surfaces, but the Kimi wire hook-dispatch path still builds a fresh `EventMeta` without the wrapper environment context or captured child PID.

Evidence:

- `run_kimi_wire_session` receives `config.env_context`, but currently discards it with a reserved-for-later binding before the child is spawned: `claudine/cli/src/commands/wrap/exec/wiring.rs:587`.
- After spawn, `captured_pid` is available, but the stdout reader calls `handle_request_dispatch` without passing either `config.env_context` or `captured_pid`: `claudine/cli/src/commands/wrap/exec/wiring.rs:676`.
- `handle_request_dispatch` forwards Kimi `HookRequest`s to `dispatch_hook_request` without any PID/context argument: `claudine/cli/src/commands/wrap/exec/wiring.rs:813` and `claudine/cli/src/commands/wrap/exec/wiring.rs:848`.
- `dispatch_hook_request` constructs `EventMeta::new(Provider::KimiCode, canonical_event)` and only copies request context fields. It never sets `meta.env` from the wrapper `EnvironmentContext` and never sets `meta.agent_pid`: `claudine/cli/src/commands/wrap/exec/wiring.rs:394` and `claudine/cli/src/commands/wrap/exec/wiring.rs:419`.

Impact: Kimi wire `HookRequest` dispatches after successful spawn do not expose `claudine_pid`, `agent_pid`, or their `extra` mirrors to templates, expressions, hook actions, dispatch JSONL, or reporting ingest. This is a Claudine-controlled hook/action context, not a raw provider stream record, so it falls under the acceptance criteria.

Expected fix: thread `EnvironmentContext` and `Some(captured_pid)` into `handle_request_dispatch` / `dispatch_hook_request`, set `meta.env = env_context.clone()`, and set `meta.agent_pid = agent_pid` before calling `dispatch_event_meta_with_runtime`. Add a Level 1 test that drives a Kimi `HookRequest` through this path with a runtime config that records the dispatched `EventMeta`, asserting both typed fields and the `extra` mirrors are present.

Verification level present: none for this requirement. Existing Level 1 tests cover Kimi request classification and fallback responses, but no test verifies PID propagation for Kimi wire hook/action dispatch.

## Test Rigor

PID capture and structured record propagation are not terminal rendering or keyboard-encoder behavior, so Level 1 is the appropriate verification level. Current Level 1 coverage now passes for provider env injection, spawned-child PID capture, live semantic JSONL records, raw-log omission when unavailable, nullable report DTOs, and reporting query projection. The remaining Kimi wire hook/action requirement has no matching Level 1 test.

I ran:

```sh
cargo test -p claudine-cli --test wrap_commands pid --color=never
cargo test -p claudine pid --color=never
```

Results: passed. The first command ran 4 tests. The second command ran 12 PID-filtered `claudine` tests.

## Production Readiness

Not ready for production. The main wrapper and reporting paths are much closer after iteration 2, but a provider-specific Claudine-controlled hook/action path still violates the PID propagation contract.
