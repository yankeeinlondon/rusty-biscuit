---
ready: true
agent: codex
model: ""
---

# Review: PID Capture for Wrapped Agentic CLIs

## Findings

No blocking findings.

Iteration 3 closes the remaining Kimi wire gap from review 2. `run_kimi_wire_session` now clones the wrapper `EnvironmentContext` after spawn, passes it with `Some(captured_pid)` through `handle_request_dispatch`, and `dispatch_hook_request` builds Kimi hook `EventMeta` with both typed PID fields and `extra` mirrors. Evidence: `claudine/cli/src/commands/wrap/exec/wiring.rs:395`, `claudine/cli/src/commands/wrap/exec/wiring.rs:422`, `claudine/cli/src/commands/wrap/exec/wiring.rs:687`, `claudine/cli/src/commands/wrap/exec/wiring.rs:891`.

## Verification Level Review

PID capture, child-env injection, JSONL emission, dispatch metadata, and report-query projection are process/data contracts. They do not depend on terminal rendering, terminal input encoders, OS keyboard injection, glyph widths, or SGR behavior, so Level 1 is the appropriate verification level for this feature.

- `CLAUDINE_PID` is injected into the provider env before spawn, and `AGENT_PID` is not fabricated into provider env: Level 1 integration test at `claudine/cli/tests/wrap_commands.rs:1555`.
- Successful provider spawn captures `agent_pid`: Level 1 spawn tests in `claudine/cli/src/commands/wrap/exec/spawn.rs` plus wrapper JSONL coverage.
- Wrapper summary and live semantic JSONL records include `env.claudine_pid` and post-spawn `agent_pid`: Level 1 integration test at `claudine/cli/tests/wrap_commands.rs:1612`; live sink unit coverage at `claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs:2198`.
- Kimi wire hook/action dispatch includes `env.claudine_pid`, `agent_pid`, and `extra` mirrors after spawn: Level 1 unit coverage at `claudine/cli/src/commands/wrap/exec/wiring.rs:1452` and unavailable-PID omission coverage at `claudine/cli/src/commands/wrap/exec/wiring.rs:1489`.
- Raw structured logs omit unavailable `agent_pid`: Level 1 serde coverage in `EventMeta`, plus dry-run and failed-binary wrapper tests.
- Report/query output exposes stable nullable PID fields: Level 1 serde tests at `claudine/lib/src/reporting/types.rs:408` and query projection tests at `claudine/lib/src/reporting/mod.rs:229`.

No Level 2 or Level 3 tests are required for this PID feature under the provided rubric.

## Tests Run

```sh
cargo test -p claudine pid --color=never
cargo test -p claudine-cli build_hook_event_meta --color=never
cargo test -p claudine-cli --test wrap_commands pid --color=never
```

Results: all passed. The commands ran 12 PID-filtered `claudine` tests, 2 Kimi hook meta-builder tests, and 4 wrapper integration PID tests.

## Production Readiness

Ready for production. The implementation now satisfies the spec's wrapper env, post-spawn child PID, Claudine-controlled context, raw-log omission, and stable nullable report/query contracts with appropriate Level 1 verification.
