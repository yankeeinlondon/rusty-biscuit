---
ready: false
agent: codex
model: ""
---

# Review: Remote Signal

## Findings

### High: Signed envelopes do not bind the document ID or payload kind

The spec requires the persisted signed envelope to include `document_id`, `payload_kind`, a monotonic per-sender message ID, the payload hash, and payload bytes. The implementation signs only `(sender || message_id || content_hash || payload)` and stores only `sender`, `message_id`, `content_hash`, `signature`, and `payload` in `SignedEnvelope` (`claudine/remote-signal/core/src/envelope.rs:67`). The sync frame carries `chunk_id` and `is_snapshot` outside the envelope (`claudine/remote-signal/daemon/src/sync.rs:305`, `claudine/remote-signal/daemon/src/sync.rs:334`, `claudine/remote-signal/daemon/src/sync.rs:365`).

That means the authenticity boundary does not cover the document target or whether the payload is a snapshot versus delta. A paired peer or corrupted transport path can replay a valid signed payload under a different `chunk_id`; `apply_remote_update` will import it and persist the resulting snapshot under the frame's chunk key (`claudine/remote-signal/daemon/src/session_log.rs:584`, `claudine/remote-signal/daemon/src/session_log.rs:671`). This violates the acceptance criteria for signed envelopes and leaves a concrete document-confusion bug.

Verification level: current coverage is Level 1 only, and there is no negative test that tampers `chunk_id` or `payload_kind` while preserving a valid envelope. For this daemon/network feature, Level 1 protocol tests are appropriate, but the required cases are missing.

### High: Accepted network envelopes and replay state are not persisted in redb

The spec says accepted network payloads are represented as persisted signed envelopes and that restart/replay, deduplication, and rejection behavior are defined by the same data accepted from the network. Storage currently has tables only for snapshots, session chunk catalogs, and pairings (`claudine/remote-signal/daemon/src/storage.rs:19`, `claudine/remote-signal/daemon/src/storage.rs:25`, `claudine/remote-signal/daemon/src/storage.rs:30`). Accepted deltas/snapshots are reduced to the latest Loro snapshot via `save_snapshot` (`claudine/remote-signal/daemon/src/session_log.rs:665`).

Replay protection is an in-memory `EnvelopeInbox` allocated inside each sync session (`claudine/remote-signal/daemon/src/sync.rs:359`). It is discarded after the session and is not rebuilt from redb after restart. The implementation can therefore accept the same signed message ID again in a later session or after daemon restart, and it cannot audit or replay the exact accepted envelopes. This is a direct gap against the durable signed-envelope and duplicate-message rejection criteria.

Verification level: Level 1 tests cover in-process duplicate detection inside `EnvelopeInbox`, but not daemon-level duplicate rejection across sync sessions or restart. Add tests that resend the same signed frame in a new sync session and after restart, then assert rejection and unchanged redb state.

### High: Message IDs are content-derived, not monotonic per sender

The envelope `message_id` is derived from `content_hash || sender` (`claudine/remote-signal/core/src/envelope.rs:70`, `claudine/remote-signal/core/src/envelope.rs:90`). The spec requires a monotonic per-sender message ID. Content-derived IDs make two identical payloads from the same sender indistinguishable from a replay, while different payloads do not provide any ordering or durable high-water mark. This design cannot support the specified per-sender replay boundary.

Verification level: Level 1 tests currently assert deterministic content-derived IDs, which locks in behavior that conflicts with the spec. Replace that with tests for monotonic IDs, durable high-water tracking, duplicate/old-ID rejection, and allowed later IDs.

### High: DuckDB is not rebuilt from redb on restart

The spec says DuckDB is disposable and rebuildable from redb, and tests should demonstrate that redb is authoritative for sync/replay. Startup opens DuckDB and starts the batcher (`claudine/remote-signal/daemon/src/server.rs:305`), while `SessionLogManager::rehydrate_from_storage` rebuilds only the in-memory Loro/session cursor state from snapshots (`claudine/remote-signal/daemon/src/session_log.rs:706`). There is no projection truncation/rebuild path and no code that replays snapshot entries into DuckDB after restart.

The existing restart test uses an in-memory projection and validates source-of-truth reads, but it never queries the projection after restart. A daemon restarted with an empty or deleted DuckDB projection will serve empty analytical rows despite redb containing acknowledged entries. That is a functional gap against the CQRS acceptance criteria.

Verification level: Level 1 integration coverage is present for initial append-to-DuckDB lag, but missing for restart/rebuild. Add a restart test that deletes or uses an empty projection, boots from existing redb, waits for rebuild, and verifies `QueryProjection` matches source-of-truth entries.

### Medium: The mDNS discovery test is not environment-gated

`two_daemons_discover_each_other_via_mdns` intentionally fails loudly when multicast is unavailable (`claudine/remote-signal/daemon/tests/peer_discovery.rs:90`). Per the repo testing taxonomy, real network/multicast availability is an external resource and should be a `real_` test or should skip unless the environment declares it required. As written, the default test suite can fail on CI runners, VMs, or locked-down networks for environmental reasons unrelated to the implementation.

Verification level: this is not a Level 2/Level 3 terminal mismatch; it is a resource-gating mismatch. Keep deterministic unit tests for TXT parsing/registry behavior in Level 1, and move real multicast discovery behind the `real_` tier or an explicit env guard.

## Requirement Coverage Notes

- Paired local nodes converge through direct QUIC sync: Level 1 integration coverage exists.
- Deterministic chunk IDs and chunk rotation: Level 1 unit/integration coverage exists.
- Unpaired peers cannot sync: Level 1 integration coverage exists for the gRPC-triggered sync path.
- Invalid envelope signature/hash: Level 1 unit coverage exists for `SignedEnvelope`, but daemon-level rejection without redb mutation is not covered.
- Mismatched sender identity: Level 1 sync-path coverage exists for sender-vs-hello mismatch in code, but no targeted test exercises it.
- Duplicate message IDs: only in-process `EnvelopeInbox` unit coverage exists; durable daemon behavior is missing.
- Persisted accepted envelopes: not implemented.
- Local write ack after redb durability: implemented by saving the snapshot before returning append success.
- Restart/replay of Loro state: Level 1 integration coverage exists for source-of-truth reads and subsequent sync.
- DuckDB rebuild from redb: not implemented or verified.

## Production Readiness

Not ready for production. The POC has a useful skeleton and meaningful L1 integration tests, but the signed-envelope contract and durable replay/rejection semantics are central to the feature's security model and are not implemented as specified.
