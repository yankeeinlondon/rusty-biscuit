---
ready: false
agent: codex
model: ""
---

# Review: Remote Signal

## Findings

### High: accepted-envelope persistence failure leaves remote data durable without its signed envelope

`SyncService::receive_delta` verifies the envelope and then calls `apply_remote_update` before saving the `AcceptedEnvelope` row (`claudine/remote-signal/daemon/src/sync.rs:264`, `claudine/remote-signal/daemon/src/sync.rs:266`, `claudine/remote-signal/daemon/src/sync.rs:295`). `apply_remote_update` persists the resulting Loro snapshot to redb before mutating memory (`claudine/remote-signal/daemon/src/session_log.rs:706`, `claudine/remote-signal/daemon/src/session_log.rs:709`). If `save_accepted_envelope` then fails, the sync call returns an error, but the received remote data is already durable and visible in the snapshots table without the signed envelope record the spec requires.

That violates two acceptance criteria: accepted network payloads must be represented as persisted signed envelopes, and rejected/failing network payloads must not mutate local redb state. It also weakens restart/replay semantics because the authoritative snapshot can contain network-originated CRDT state that has no durable envelope audit row and cannot be deduplicated from the persisted envelope table.

The right shape is a three-step staged path: verify envelope, import the payload into a temporary/staged Loro state without saving the snapshot, persist the accepted envelope, then persist the staged snapshot. If the envelope write fails, no snapshot should exist. If the snapshot write fails after the envelope write, startup replay can recover from the persisted envelope.

Verification level: Level 1 is appropriate. Add a sync-path test using `Storage::inject_accepted_envelope_failure()` through `receive_delta`, not just direct storage calls, and assert both `snapshot_count` and `accepted_envelope_count` are unchanged after the failure.

### High: signed remote snapshots are not validated against the session-log document schema before persistence

The receiver authenticates the envelope and checks ownership, document ID, duplicate message ID, and payload kind, but it does not validate that the imported Loro document contains the required deterministic metadata or only well-formed append-only `Entry` values. For a new remote chunk, `apply_remote_update` imports arbitrary Loro bytes, counts only values that happen to deserialize as `Entry`, fabricates metadata from the chunk path and local clock, and saves the snapshot (`claudine/remote-signal/daemon/src/session_log.rs:667`, `claudine/remote-signal/daemon/src/session_log.rs:672`, `claudine/remote-signal/daemon/src/session_log.rs:680`, `claudine/remote-signal/daemon/src/session_log.rs:706`, `claudine/remote-signal/daemon/src/session_log.rs:709`). Projection rebuild and live projection similarly skip malformed entries instead of rejecting the accepted document (`claudine/remote-signal/daemon/src/session_log.rs:759`, `claudine/remote-signal/daemon/src/session_log.rs:795`).

The spec requires each chunk document to include deterministic metadata and an append-only `entries` list with the defined entry schema. A paired peer can currently send a signed snapshot for its own namespace with missing or inconsistent metadata, non-string entries, invalid entry JSON, duplicate or non-monotonic sequences, or non-append-only edits, and the snapshot can still become redb state. Signature verification proves origin, not document validity.

Verification level: Level 1 is appropriate. Add validation before persistence for metadata shape/values, entry decoding, sequence ordering, and append-only behavior for updates over existing chunks. Tests should construct signed but schema-invalid Loro snapshots/deltas and assert no snapshot or accepted-envelope row is written.

### Medium: local appends can be reported as failed after redb has already accepted the write

`append_entry` saves the snapshot to redb, swaps the staged state into memory, advances the session cursor, and only then submits the row to the DuckDB projection batcher (`claudine/remote-signal/daemon/src/session_log.rs:442`, `claudine/remote-signal/daemon/src/session_log.rs:444`, `claudine/remote-signal/daemon/src/session_log.rs:447`, `claudine/remote-signal/daemon/src/session_log.rs:466`, `claudine/remote-signal/daemon/src/session_log.rs:484`). If `batcher.submit` fails, the caller receives an error even though the write is already durable in redb.

That is awkward for a command API: a client that retries after the error will create a second entry, even though the first write was already acknowledged by the storage layer. The spec says DuckDB may lag and redb is authoritative, so projection queue failure should not determine whether the local write is reported as accepted. Prefer returning success after redb durability and surfacing projection failures through logs/health state, or explicitly model the response as "write accepted, projection enqueue failed."

Verification level: Level 1 is appropriate. Add a test that closes the batcher, appends, and verifies the API behavior matches the intended contract without creating retry ambiguity.

## Requirement Coverage Notes

- Level 1 integration coverage exists for two-node convergence, direct paired-peer sync, manual invitation, explicit pairing, deterministic chunk rotation, restart/replay from redb snapshots, DuckDB rebuild, projection idempotence, and the one-sided-pairing rejection path.
- The mDNS behavior is covered by real-network tests gated behind `REMOTE_SIGNAL_REAL_MDNS=1`. That is an appropriate non-default verification class for multicast discovery, but it should be documented in the package test strategy if this feature becomes a release gate.
- L2/L3 terminal verification is not applicable. The spec does not assert terminal rendering, keyboard encoder behavior, paste/IME, mouse, or TUI interactions.
- The remaining blockers are storage/protocol invariants, so Level 1 tests are sufficient once they exercise the actual sync/redb boundary.

## Production Readiness

Not ready for production. The paired sync path can persist remote CRDT state without the required accepted-envelope row if envelope persistence fails, and authenticated remote CRDT payloads are not yet validated against the session-log schema before becoming durable state.

## Verification

Attempted:

- `cargo test -p remote-signal-core -p remote-signal-daemon -p remote-signal-client --color=never`

The command was still compiling after roughly 60 seconds from a cold build, so I stopped it to avoid hanging the non-interactive session. Findings are from source inspection and existing test bodies.
