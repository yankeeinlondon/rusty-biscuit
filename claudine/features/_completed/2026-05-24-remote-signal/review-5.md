---
ready: false
agent: codex
model: ""
---

# Review: Remote Signal

## Findings

### High: repeated remote syncs can duplicate DuckDB projection rows

The live sync path now queues remote entries into DuckDB, which fixes the prior "remote entries never reach the projection" gap. The implementation does it by submitting every entry in the whole chunk whenever a remote update advances (`claudine/remote-signal/daemon/src/sync.rs:413`, `claudine/remote-signal/daemon/src/sync.rs:416`, `claudine/remote-signal/daemon/src/session_log.rs:741`, `claudine/remote-signal/daemon/src/session_log.rs:754`). DuckDB append is plain insert into a table with no uniqueness constraint or upsert key (`claudine/remote-signal/daemon/src/projection.rs:17`, `claudine/remote-signal/daemon/src/projection.rs:122`).

That means an incremental sync into an existing chunk re-enqueues already-projected entries. A concrete flow is: Alice syncs one entry to Bob, Bob projects row 0; Alice appends row 1 to the same chunk and syncs again; Bob imports the delta, then `submit_chunk_to_projection` inserts row 0 again plus row 1. `QueryProjection` can now return duplicate historical entries even though the source Loro chunk and redb snapshot are correct. This violates the CQRS contract that DuckDB is a lagging projection of redb, not an accumulating duplicate log.

Verification level: Level 1 daemon/client integration is the right level, but I did not find coverage for this scenario. Current projection tests cover local append projection (`claudine/remote-signal/client/tests/session_log_round_trip.rs:92`) and startup rebuild (`claudine/remote-signal/daemon/src/session_log.rs:1171`), while sync convergence tests read Loro entries/chunk catalogs rather than `QueryProjection` after repeated incremental syncs (`claudine/remote-signal/daemon/tests/phase6_integration.rs:212`, `claudine/remote-signal/daemon/tests/phase6_integration.rs:250`). Add a Level 1 test that syncs the same chunk twice with a new entry on the second sync and asserts the receiver projection has exactly one row per `(chunk_id, sequence)`. Fix either by projecting only newly materialized entries or by making the projection idempotent with a uniqueness key/upsert.

### High: signed envelopes are persisted after mutating redb, so the acceptance boundary is not crash-safe

The spec says accepted network payloads are represented as persisted signed envelopes and that the signed envelope defines replay/deduplication behavior. In the sync receiver, the daemon verifies and accepts the envelope in memory, applies the payload to the session log, and `apply_remote_update` saves the merged snapshot to redb (`claudine/remote-signal/daemon/src/sync.rs:411`, `claudine/remote-signal/daemon/src/sync.rs:413`, `claudine/remote-signal/daemon/src/session_log.rs:702`, `claudine/remote-signal/daemon/src/session_log.rs:708`). Only after that does it write the accepted-envelope row (`claudine/remote-signal/daemon/src/sync.rs:419`, `claudine/remote-signal/daemon/src/sync.rs:434`).

If the process crashes, is killed, or `save_accepted_envelope` fails in that window, redb can contain the remote mutation without the envelope record that makes the mutation auditable and replay-protected. After restart, the duplicate message ID is not known to `has_accepted_envelope` (`claudine/remote-signal/daemon/src/storage.rs:393`), so the same network payload can be accepted again rather than rejected at the signed-envelope boundary. Loro may make the CRDT bytes idempotent, but the specified duplicate-message rejection and persisted-envelope audit trail are still broken.

Verification level: Level 1 storage/session-manager coverage is appropriate, but the existing accepted-envelope replay test constructs the happy crash-recovery state manually: it applies a payload, saves an accepted envelope, removes the snapshot, and restarts (`claudine/remote-signal/daemon/src/session_log.rs:1361`, `claudine/remote-signal/daemon/src/session_log.rs:1375`, `claudine/remote-signal/daemon/src/session_log.rs:1383`). It does not cover the actual receiver ordering or the failure window where snapshot persistence succeeds and envelope persistence is lost. Persist the accepted envelope and snapshot in one redb transaction, or persist the envelope before applying and make startup reconcile pending accepted envelopes. Add a Level 1 test that simulates the crash window and verifies duplicate rejection after restart.

### Medium: manual invitation records pairing before proving the remote identity

The spec's manual invitation flow says the daemon records pairing if the remote identity matches the invitation. The implementation records the pairing immediately after a permissive QUIC connection succeeds (`claudine/remote-signal/daemon/src/service.rs:247`, `claudine/remote-signal/daemon/src/service.rs:251`, `claudine/remote-signal/daemon/src/service.rs:253`). QUIC server certificates are intentionally accepted without authenticating the peer (`claudine/remote-signal/daemon/src/quic.rs:6`, `claudine/remote-signal/daemon/src/quic.rs:230`), and the sync-level identity check happens later in `SyncHello` (`claudine/remote-signal/daemon/src/sync.rs:237`, `claudine/remote-signal/daemon/src/sync.rs:246`).

This means `ConnectToPeer` can persist an authorization record for the invitation's node ID even when the endpoint at that address has not yet proven it owns that node ID. Data exchange should still fail on the later hello mismatch, but the pairing table has already been mutated based on transport reachability rather than identity verification. That is weaker than the explicit-pairing trust boundary described by the spec.

Verification level: Level 1 integration coverage is appropriate. Existing tests assert invite-created one-sided pairing plus responder rejection (`claudine/remote-signal/daemon/tests/phase6_integration.rs:425`, `claudine/remote-signal/daemon/tests/phase6_integration.rs:456`), but not "invitation public key does not match connected daemon identity, so no pairing is recorded." Move the pairing write after a successful identity-confirming handshake, or store an untrusted peer record until sync/bootstrap proves the invited public key.

## Requirement Coverage Notes

- Paired two-node convergence, deterministic chunk IDs/chunk rotation, mDNS discovered-but-unpaired rejection, local write durability before ack, restart from redb snapshots, accepted-envelope-only replay, and invalid envelope checks have meaningful Level 1 or real-resource coverage.
- DuckDB projection correctness for repeated network-applied deltas is not adequately implemented or verified.
- Durable signed-envelope acceptance is not atomic with the redb mutation it authorizes, leaving a crash/replay gap.
- Manual-invitation pairing is implemented, but identity proof happens after pairing state is persisted.
- L2/L3 terminal verification is not applicable. This feature does not define terminal rendering, hotkeys, paste/IME, mouse, or OS keyboard behavior.

## Production Readiness

Not ready for production. Iteration 5 closes important prior gaps, but the projection can become user-visibly wrong after normal incremental sync, and the signed-envelope persistence boundary still does not meet the spec's replay/deduplication contract.

## Verification

Attempted targeted tests:

- `cargo test --color=never -p remote-signal-daemon accepted_envelope_only_replay_recovers_missing_snapshot -- --nocapture`
- `cargo test --color=never -p remote-signal-daemon sync_fails_when_only_one_side_is_paired --test phase6_integration -- --nocapture`

Both were stopped after roughly 60 seconds because the workspace was still cold-compiling and contending on Cargo locks in this non-interactive session.
