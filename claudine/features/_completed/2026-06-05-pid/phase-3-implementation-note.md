# Phase 3 — Claudine-Controlled Context Propagation

## Summary

Phase 3 propagates the PID fields captured in Phase 2 through every
Claudine-controlled context surface so that wrapper session lifecycle
records, dispatch metadata, template/expression contexts, and stream
summary events all carry `claudine_pid` and (when available) `agent_pid`.

## Field Placement

### `EnvironmentContext.claudine_pid: Option<u32>`

- Added to `claudine/lib/src/events/environment.rs`.
- Populated by `apply_wrapper_package_context` reading `CLAUDINE_PID`
  from the environment, with fallback to `std::process::id()`.
- `#[serde(default)]` ensures legacy JSONL deserializes as `None`.
- Present on every `EventMeta` via `meta.env.claudine_pid`.

### `EventMeta.agent_pid: Option<u32>`

- Added to `claudine/lib/src/events/event_meta.rs`.
- `#[serde(default, skip_serializing_if = "Option::is_none")]` — raw
  JSONL omits the key entirely when the value is `None`.
- `None` until a successful provider spawn; populated only on
  wrapper-side summary/lifecycle events.
- Hook handler events (separate process) leave it `None` because the
  hook handler has no way to know the agent PID.

## Propagation Paths

### Dispatch preparation (`claudine/lib/src/dispatch/mod.rs`)

`prepare_meta_for_dispatch` mirrors `claudine_pid` (from `meta.env`)
and `agent_pid` (from `meta.agent_pid`) into `meta.extra` as
stringly-typed keys so template and expression contexts can resolve
`{{claudine_pid}}`, `{{agent_pid}}`, `env.claudine_pid`, and
`meta.agent_pid`.

### Stream summary events (`claudine/lib/src/stream/reporting.rs`)

`summary_to_event_meta_with_context` accepts a new `agent_pid:
Option<u32>` parameter and stamps it onto the constructed `EventMeta`.
`claudine_pid` is sourced from the `EnvironmentContext` already
attached to the event.

`semantic_event_to_event_meta` — which converts ordinary provider
stream records — explicitly sets `agent_pid: None`. Per the spec,
ordinary provider stream events do NOT receive PID fields; only
wrapper session lifecycle records do.

### Composition paths

`CompositionStreamResult` (`claudine/cli/src/commands/wrap/composition/mod.rs`)
carries `agent_pid: Option<u32>` from the spawn result. Every emitter
that calls `summary_to_event_meta_with_context` or
`emit_stream_summary_with_context` now threads it through:

- `emit_composition_summary`
- `emit_minimal_composition_summary`
- `emit_stream_summary` / `emit_stream_summary_inner` (policy.rs)
- Direct wrapper path (`wrap/mod.rs`)
- Harness orchestrator (`harness_orch.rs`)
- Composition structured mode (`composition/structured.rs`)
- Inline composition guards (`composition/inline_guards.rs`)

### Kimi wire path (`claudine/cli/src/commands/wrap/exec/wiring.rs`)

The Kimi hook dispatch (`dispatch_hook_request`) builds `EventMeta`
directly and does NOT have access to the wrapper's
`EnvironmentContext` or the (not-yet-known) `agent_pid`. Per the spec,
this is acceptable: `claudine_pid` will be populated by the env
fallback (`std::process::id()`) and `agent_pid` is `None`. The summary
event for the Kimi wire session DOES receive `agent_pid` from
`stream_result.agent_pid` via the standard composition path.

## Test Coverage

Seven new tests added:

| Test | File | What it asserts |
|------|------|-----------------|
| `agent_pid_omitted_when_none` | `event_meta.rs` | JSONL omits `agent_pid` key when value is `None` |
| `agent_pid_serialized_when_some` | `event_meta.rs` | JSONL includes `agent_pid` as a number when populated |
| `agent_pid_defaults_to_none_on_deserialize` | `event_meta.rs` | Legacy JSONL without `agent_pid` deserializes to `None` |
| `wrapper_package_context_reads_claudine_pid_from_env` | `environment.rs` | `CLAUDINE_PID` env var is parsed into `claudine_pid` |
| `wrapper_package_context_falls_back_to_process_id` | `environment.rs` | When env var is unset, falls back to `std::process::id()` |
| `claudine_pid_round_trips_json` | `environment.rs` | `claudine_pid` survives serde round-trip |
| `claudine_pid_defaults_to_none_on_deserialize` | `environment.rs` | Legacy JSONL without `claudine_pid` deserializes to `None` |

Full test suite: **2432 passed, 0 failed** (up from 2425).

## Validation

- `cargo check -p claudine -p claudine-cli` — clean.
- `cargo test -p claudine -p claudine-cli --lib` — 2432 passed.
- `cargo clippy -p claudine -p claudine-cli --all-targets -- -D warnings` — clean.

## Pre-existing issues fixed

Two pre-existing lint/compile issues from earlier phases were blocking
`cargo clippy --all-targets`:

1. `claudine/cli/src/perf.rs` — `mark_substage_with_children` (added
   speculatively in Phase 2 but never called) triggered `dead_code`.
   Added `#[allow(dead_code)]`.
2. `claudine/cli/src/commands/wrap/repo_home.rs` — the triple-tuple
   return type of `build_repo_home_env` (changed in Phase 2 to include
   `RepoHomeTimings`) triggered `clippy::type_complexity`. Added
   `#[allow(clippy::type_complexity)]`.
3. `claudine/cli/src/commands/wrap/mod.rs` tests — two `EnvPlan`
   struct literals missing the `perf_substages` field (added in
   Phase 2). Added `perf_substages: Vec::new()`.

These are surgical, minimal fixes to unblock Phase 3 validation.
