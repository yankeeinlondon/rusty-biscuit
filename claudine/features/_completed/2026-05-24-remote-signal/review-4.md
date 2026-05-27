---
ready: false
agent: codex
model: ""
---

# Review: Remote Signal

## Findings

### High: synced remote entries never reach the live DuckDB projection

The spec's ingestion pipeline says network-applied CRDT deltas should update the source-of-truth Loro state and then enqueue JSON state for the DuckDB micro-batcher. Local appends do that: after redb durability, `append_entry` submits a `ProjectionRow` to the batcher (`claudine/remote-signal/daemon/src/session_log.rs:455`, `claudine/remote-signal/daemon/src/session_log.rs:477`). The inbound sync path does not. `SyncService` applies a verified remote envelope with `apply_remote_update` (`claudine/remote-signal/daemon/src/sync.rs:411`, `claudine/remote-signal/daemon/src/sync.rs:413`), and `apply_remote_update` imports the Loro payload, saves the snapshot to redb, and updates cursors, but never submits any newly materialized entries to the batcher (`claudine/remote-signal/daemon/src/session_log.rs:621`, `claudine/remote-signal/daemon/src/session_log.rs:702`, `claudine/remote-signal/daemon/src/session_log.rs:708`, `claudine/remote-signal/daemon/src/session_log.rs:725`).

The result is that `ListChunkEntries` can show replicated data immediately after sync, while `QueryProjection` on the same receiver remains stale until a daemon restart rebuilds DuckDB from redb (`claudine/remote-signal/daemon/src/session_log.rs:740`). That is stronger than allowed lag; it is a missing live ingestion path for network data.

Verification level: Level 1 integration coverage is appropriate, but missing. Current projection tests cover local appends (`claudine/remote-signal/client/tests/session_log_round_trip.rs:92`) and restart rebuild (`claudine/remote-signal/daemon/src/session_log.rs:1132`), not "sync remote entry, wait for batcher, query projection on receiver." Add that test and queue deduplicated projection rows from `apply_remote_update`.

### High: manual invitations do not establish pairing, so one specified authorization path is absent

The spec says data exchange begins after "manual invitation or explicit local approval," and the manual-invitation flow says `ConnectToPeer(invite_string)` dials the endpoint and records pairing if the remote identity matches the invitation. The implementation only records a QUIC peer record in `connect_to_peer`; it does not verify the remote sync identity or persist a pairing (`claudine/remote-signal/daemon/src/service.rs:231`, `claudine/remote-signal/daemon/src/service.rs:247`, `claudine/remote-signal/daemon/src/service.rs:251`). Pairing is only persisted through the separate `ApprovePeer` RPC (`claudine/remote-signal/daemon/src/service.rs:269`, `claudine/remote-signal/daemon/src/service.rs:276`), and sync still rejects invite-only peers via `assert_paired` (`claudine/remote-signal/daemon/src/sync.rs:180`, `claudine/remote-signal/daemon/src/sync.rs:200`).

The tests encode the narrower implementation rather than the spec: successful sync helpers approve both sides before connecting (`claudine/remote-signal/daemon/tests/phase6_integration.rs:77`, `claudine/remote-signal/daemon/tests/phase6_integration.rs:92`), while invite-only sync is expected to fail (`claudine/remote-signal/daemon/tests/pairing_and_sync.rs:164`, `claudine/remote-signal/daemon/tests/pairing_and_sync.rs:179`). That leaves no tested flow where a manual invitation alone authorizes the joiner.

Verification level: Level 1 daemon/client integration is appropriate, but absent for the spec behavior. Either implement invitation-confirmed pairing with identity verification during connect/sync bootstrap, or narrow the spec to say manual invitations only create transport connections and explicit approval is always required.

### Medium: the accepted-envelope replay test does not exercise accepted-envelope crash recovery

Iteration 4 added startup replay from `iter_accepted_envelopes` (`claudine/remote-signal/daemon/src/session_log.rs:837`, `claudine/remote-signal/daemon/src/session_log.rs:847`), which addresses the design direction from review 3. The coverage still does not prove the crash boundary it names. `crash_recovery_replays_accepted_envelope` syncs Alice to Bob, then restarts Alice and verifies Alice's own locally appended snapshot survived (`claudine/remote-signal/daemon/tests/phase6_integration.rs:521`, `claudine/remote-signal/daemon/tests/phase6_integration.rs:550`, `claudine/remote-signal/daemon/tests/phase6_integration.rs:558`). It does not persist an accepted-envelope row without a matching snapshot, restart the receiver, and verify the payload is replayed before duplicate rejection.

Verification level: Level 1 storage/session-manager coverage is appropriate. Add a focused test that writes a real accepted envelope payload into redb, withholds or removes the corresponding snapshot, constructs a new `SessionLogManager`, and asserts the chunk state is recovered from the accepted envelope.

## Requirement Coverage Notes

- Paired two-node direct sync, deterministic chunking, restart from redb snapshots, ownership checks, invalid signatures/hash/sender/document/kind, duplicate IDs, and mDNS unpaired rejection now have Level 1 or real-resource coverage in the implementation.
- Live DuckDB projection for network-applied CRDT deltas is not implemented or verified.
- Manual-invitation-as-pairing is not implemented or verified.
- Accepted-envelope startup replay is implemented, but the crash-recovery test does not actually cover the accepted-envelope-only replay case.
- L2/L3 terminal verification is not applicable; this spec does not define terminal rendering, hotkeys, paste, mouse, or OS keyboard behavior.

## Production Readiness

Not ready for production. The sync/storage security fixes from the prior review are materially better, but the live analytical projection and manual invitation semantics still do not match the specification.

## Verification

Attempted `cargo test --color=never -p remote-signal-core -p remote-signal-daemon -p remote-signal-client`. I stopped it after roughly 60 seconds because the workspace was still compiling cold dependencies in this non-interactive session.
