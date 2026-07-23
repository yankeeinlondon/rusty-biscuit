---
title: Change Notes — Rendezvous Local IPC
phase_recorded: 1
recorded: 2026-07-16
tool: GitNexus impact (upstream, includeTests=true, repo=rusty-biscuit @ claudine worktree)
---

# Change Notes

Pre-implementation blast-radius record required by Phase 1. Re-run
`impact({target, direction: "upstream"})` before editing each symbol in the
phase that actually touches it.

## `default_socket_path` — `claudine/rendezvous/core/src/socket.rs`

- **Risk: CRITICAL** (epistemic: exact). 23 impacted symbols; 6 direct callers.
- Direct callers (depth 1):
  - `claudine/cli/src/commands/dashboard/mod.rs:run`
  - `claudine/cli/src/commands/handle.rs:report_session_status`
  - `claudine/cli/src/commands/wrap/harness_orch/loop_control/requeue.rs:enqueue_requeue_entry_async`
  - `claudine/cli/src/commands/wrap/session_report.rs:StatusReporter::report`
  - `claudine/cli/src/commands/wrap/session_report.rs:report_status`
  - `claudine/cli/src/commands/wrap/session_report.rs:block_report`
- Affected processes: `dashboard::run` (7 flows, earliest broken step 1),
  `wrap::harness_orch::attempt::execute_harness_attempt` (6 flows, step 1).
- Affected modules: Session_report (9), Wrap (5), Tests (3), Commands (3),
  Dashboard (1), Live_semantic_sink (1, indirect).
- These are exactly the Phase 6 migration call-site groups.

## `spawn_uds_server` — `claudine/rendezvous/daemon/src/server.rs`

- **Risk: CRITICAL** (epistemic: exact). 403 impacted symbols; 15 direct callers.
- Depth counts: 15 / 152 / 236. Modules: Tests (218 hits, direct),
  Session_report (4, direct); the remainder is indirect fan-out through the
  generic `main`/`run` process hubs of unrelated areas (research, homelab,
  biscuit-clipboard, model-citizen, sniff CLI) — call-graph hub noise, not a
  real coupling to Rendezvous.
- The genuine surface is the daemon-spawning test fixtures plus session
  reporting. Phase 4 replaces this with `spawn_local_server`.

## `rendezvous_client::connect` — `claudine/rendezvous/client/src/lib.rs`

- **Risk: LOW** (epistemic: exact). 25 impacted symbols; 1 direct caller.
- All 24 downstream hits are in the Tests module. Phase 3 can change the
  signature to take `LocalEndpoint` with a small production blast radius.

## Daemon data-root selection — `claudine/rendezvous/daemon/src/main.rs`

- Not a named symbol; selection is inline at `main.rs:87`:
  `.unwrap_or_else(|| std::env::temp_dir().join("rendezvous-data"))`, documented
  at `main.rs:33`. No callers to migrate; Phase 4 replaces this expression with
  `default_data_dir()` under the platform-local data directory.

## Phase 1 assessment

Phase 1 adds `sniff::os::user` only. It edits none of the CRITICAL symbols
above, so no blast radius is realized in this phase. The record exists so
Phases 2–6 start from a measured baseline rather than re-deriving it.
