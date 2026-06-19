---
ready: false
agent: codex
model: ""
created: "2026-06-19T13:50:30"
---

# Review Findings

Assumption: `@claudine/./design.md` does not exist in this worktree, so I reviewed against `claudine/features/2026-06-19-review-findings/plan.md` as the technical design artifact.

## Findings

### High — Concurrent append/remote-commit can still lose durable data on restart

`claudine/rendezvous/daemon/src/session_log.rs:447` computes a staged snapshot, drops the manager lock, persists that staged snapshot at `:455`, and only then re-acquires the lock and imports into the live in-memory document at `:460-464`. The remote commit path has the same shape: it persists `staged.state.snapshot_bytes()` at `:711-713`, then imports it into the live doc at `:718-724`.

That fixes the in-memory overwrite, but not the durable race. Two concurrent appends against the same base can each persist `N+A` and `N+B`; after both imports, memory may contain `N+A+B`, but redb contains whichever stale staged snapshot was written last. A restart then drops the other accepted entry. The same race applies to two inbound remote updates staged from the same base.

This means spec acceptance criteria 3 is not met: the staging→commit race is not actually closed for durable state. The existing tests exercise restart and crash replay separately, but I did not find a restart-after-concurrent-append or restart-after-concurrent-remote-commit test that would catch this.

Recommended fix: after re-acquiring the lock and merging/importing, export the merged live document and persist that merged snapshot, or serialize per-chunk commit through a per-chunk lock so the persisted snapshot is the exact state being published. Add a regression that forces two concurrent writes to one chunk, verifies both are visible in memory, restarts the manager/daemon, and verifies both remain visible.

### High — Sync still performs redb writes directly on the Tokio worker thread

`claudine/rendezvous/daemon/src/sync.rs:474-487` seals an outbound delta and immediately calls `self.storage.save_outbound_counter(...)` inside the async sync loop. `save_outbound_counter` opens a redb write transaction and commits it synchronously in `claudine/rendezvous/daemon/src/storage.rs:467-477`.

The design and spec explicitly require synchronous redb/DuckDB I/O to be moved off Tokio worker threads. Other RPC paths were wrapped in `spawn_blocking`, but this sync initiator path was missed. Under a slow fsync, syncing one peer can park a runtime worker and degrade unrelated async work.

Recommended fix: move this counter persistence into `spawn_blocking` or the same blocking persistence actor used for the rest of daemon storage. Add a test seam with a deliberately slow counter write and assert the async runtime can continue servicing another lightweight task/RPC while the write is in progress.

## Test Rigor

The reviewed gaps are not terminal-rendering or keyboard-encoder requirements, so Level 2/Level 3 terminal verification is not the relevant bar. The missing coverage is Level 1/integration-style durability and async scheduling coverage: concurrent write plus restart, and slow redb write while the async runtime remains responsive.

## Summary

The protect, UTF-8 truncation, lifecycle ternary, contract-crate, and most wrapper hardening work are implemented in the expected shape. The feature should not be marked production-ready until the rendezvous durability race and remaining async blocking write are fixed and covered.
