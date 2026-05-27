---
ready: false
agent: codex
model: ""
---

# Review: Remote Signal

## Findings

### High: failed redb writes can leave unacknowledged session data live in memory

The spec requires local session-log writes to be acknowledged only after the corresponding Loro snapshot/delta is durable in redb. The implementation does return an error if the redb write fails, but by then the in-memory source document has already been mutated. `append_entry` creates or rotates the chunk, appends the entry into the live `LoroDoc`, and only then calls `save_snapshot` (`claudine/remote-signal/daemon/src/session_log.rs:392`, `claudine/remote-signal/daemon/src/session_log.rs:409`, `claudine/remote-signal/daemon/src/session_log.rs:451`, `claudine/remote-signal/daemon/src/session_log.rs:457`). `save_snapshot` is the transactional redb boundary (`claudine/remote-signal/daemon/src/storage.rs:183`, `claudine/remote-signal/daemon/src/storage.rs:205`).

If that transaction fails because the database is unavailable, the disk is full, permissions changed, or redb returns a commit/storage error, the caller receives a failed append, but the process still holds the unacknowledged entry in memory. A later successful append to the same chunk can persist a snapshot containing the failed entry, and a sync before restart can export the in-memory chunk to a peer even though the local caller was told the write did not complete. The remote path has the same shape: `apply_remote_update` imports bytes into the live doc before `save_snapshot` (`claudine/remote-signal/daemon/src/session_log.rs:678`, `claudine/remote-signal/daemon/src/session_log.rs:690`, `claudine/remote-signal/daemon/src/session_log.rs:708`), so a storage failure during inbound sync can leave a rejected/not-accepted remote update resident in memory.

Verification level: Level 1 is appropriate. I found durability success tests, restart/replay tests, malformed-payload rejection tests, and accepted-envelope crash-window replay tests, but no failure-injection test for the persistence boundary. Add a Level 1 test with an injectable/failing storage boundary or a small storage trait/fake: make `save_snapshot` fail after the Loro mutation would happen, assert `append_entry` returns an error, then assert `list_chunk_entries`, `list_session_chunks`, subsequent append, and export/sync do not surface the failed entry. The same test shape should cover `apply_remote_update` failing during persistence. A robust fix is to stage mutations in a cloned/fresh `LoroDoc`, persist the resulting snapshot first, and only then swap the staged state into `inner.chunks` and advance cursors.

## Requirement Coverage Notes

- Paired two-node convergence, direct paired-peer sync, deterministic chunk IDs, chunk rotation, restart/replay from redb, explicit pairing, invitation-based identity confirmation, discovered-but-unpaired rejection, accepted-envelope persistence, duplicate message rejection, invalid signature/hash rejection, mismatched sender/document/payload-kind rejection, malformed Loro payload rejection, and DuckDB rebuild/idempotence all have meaningful Level 1 coverage.
- The mDNS unpaired-peer boundary now has a gated real-network test via `REMOTE_SIGNAL_REAL_MDNS=1`, which is the right class of verification for that discovery-specific requirement.
- L2/L3 terminal verification is not applicable. The feature does not specify terminal rendering, keypress behavior, paste/IME, mouse behavior, or terminal-emulator input encoding.
- The remaining gap is not a terminal/user-input level mismatch; it is a storage failure-mode gap at the source-of-truth durability boundary.

## Production Readiness

Not ready for production. The happy-path protocol and prior review fixes look substantially stronger, but the current mutation-before-durability ordering can expose or later persist data from writes that were reported as failed.

## Verification

Attempted:

- `cargo test --color=never -p remote-signal-daemon malformed_loro_payload_rejected_without_envelope_row -- --nocapture`

The command first blocked waiting for Cargo's artifact directory lock and then exceeded the 60-second non-interactive limit, so I stopped it without a test result.
