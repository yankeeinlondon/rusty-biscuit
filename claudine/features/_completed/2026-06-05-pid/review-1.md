---
ready: false
agent: codex
model: ""
---

# Review: PID Capture for Wrapped Agentic CLIs

## Findings

### High: Live hook/action/log contexts do not receive `agent_pid` after spawn

The spec requires Claudine-controlled hook, action, log, and report contexts to include `agent_pid` after a successful wrapped-provider spawn. The implementation captures the child PID in the spawn layer and returns it in `ProcessResult`, but live semantic event dispatch/logging happens before that `ProcessResult` exists.

Evidence:

- `semantic_event_to_event_meta` always constructs semantic-event records without an `agent_pid` input, and its `EventMeta` construction has no way to populate the field: `claudine/lib/src/stream/reporting.rs:192`.
- `LiveSemanticSink::dispatch_meta` calls that helper and only overrides `event` and `cwd`: `claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs:608`.
- `on_semantic_event` logs and dispatches that same metadata for every live semantic event: `claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs:790`.
- The child PID is captured in the spawn function before streaming, but is only returned at the end as `ProcessResult.agent_pid`: `claudine/cli/src/commands/wrap/exec/spawn.rs:601`, `claudine/cli/src/commands/wrap/exec/spawn.rs:971`.

Impact: lifecycle summary records get `agent_pid`, but streamed Claudine-controlled records and hook/action contexts emitted during the wrapped session do not. This misses an acceptance criterion and makes per-event correlation incomplete for exactly the records operators are likely to inspect during a session.

Verification level present: Level 1 only, and it only verifies the final wrapper summary JSONL record (`wrapper_structured_summary_includes_pids_in_jsonl`). There is no Level 1 test asserting a dispatched/logged semantic event or hook/action context carries `agent_pid` after spawn.

### High: Report JSON omits nullable `agent_pid` instead of exposing a stable null field

The spec says report and query outputs must expose stable nullable `agent_pid` fields or columns, where `agent_pid: null` means no child PID was available. `SessionInfo` defines `agent_pid: Option<u32>`, but it is annotated with `#[serde(skip_serializing_if = "Option::is_none")]`, so JSON reports omit the field entirely when null: `claudine/lib/src/reporting/types.rs:187`.

Impact: machine consumers cannot distinguish "old client/schema without this field" from "new schema, no agent PID available", which is the stability guarantee the spec explicitly asks for.

Verification level present: Level 1 database ingest tests verify SQL `NULL` storage, but no report/CLI JSON test verifies `agent_pid: null` is emitted. The current serde annotation would fail that test.

### High: Non-session report/query DTOs do not expose PID fields

The implementation adds `claudine_pid` and `agent_pid` to the SQL `events` and `sessions` tables, and to `SessionInfo`, but the event-shaped report DTOs do not project those columns. `ErrorRecord` has no PID fields: `claudine/lib/src/reporting/types.rs:203`. The errors queries select neither `e.claudine_pid` nor `e.agent_pid`: `claudine/lib/src/reporting/queries/errors.rs:24` and `claudine/lib/src/reporting/queries/sessions.rs:374`.

This conflicts with the plan/spec language for report/query outputs and with the requirement that Claudine-controlled log/report contexts include PID information. At minimum, event/detail surfaces that expose individual event rows, such as errors and session detail events, should surface nullable PID fields when the underlying event table has them.

Verification level present: Level 1 tests cover event/session table persistence and session aggregation only. I found no query-specific tests for errors, today/week/month, or session-detail event rows projecting PID fields, despite the plan marking that work complete.

## Test Rigor

PID capture is not a terminal rendering or keyboard-encoder feature, so Level 1 tests are appropriate for most requirements here. The current Level 1 coverage verifies provider env injection, spawn-result PID capture, summary JSONL PID fields, and SQL ingest/aggregation. It does not verify live semantic dispatch/log records, hook/action contexts, or stable nullable JSON report output.

I ran:

```sh
cargo test -p claudine-cli --test wrap_commands pid --color=never
```

Result: passed, 4 tests.

## Production Readiness

Not ready for production. The child PID is captured, but it is not propagated to all required Claudine-controlled contexts, and report/query output does not yet meet the stable nullable `agent_pid` contract.
