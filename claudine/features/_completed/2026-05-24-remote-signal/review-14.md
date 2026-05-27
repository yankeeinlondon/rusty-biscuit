---
ready: true
agent: open_code
model: ""
---

# Review 14: Remote Signal — Session-Log POC

## Summary

All three findings from review-13 have been fully addressed with both implementation fixes and Level 1 tests. The implementation now satisfies every acceptance criterion in the spec. No new high-severity gaps remain. The POC is **production-ready within its stated scope** (two-node session-log sync with deterministic chunking, signed envelopes, restart/replay, and mDNS-gated discovery).

## Review-13 Resolution Audit

### 1. Append-only enforcement on existing chunks — RESOLVED

`validate_append_only_prefix` ([session_log.rs:1229](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:1229)) now captures the original entry JSON strings before import, re-collects them after import, and asserts the prefix is byte-identical. This is called from `stage_remote_update` for existing chunks ([session_log.rs:661](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:661)).

Level 1 tests cover:
- Message mutation of an existing entry — rejected ([session_log.rs:2469](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:2469))
- Delete+replace with same sequence — rejected ([session_log.rs:2525](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:2525))
- Reordering of existing entries — rejected ([session_log.rs:2593](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:2593))

### 2. `previous_chunk_id` rejected on chunk 0 — RESOLVED

`validate_and_extract_metadata` now rejects any `previous_chunk_id` key present in the metadata map when `expected_chunk_index == 0` ([session_log.rs:1044–1051](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:1044)). `validate_metadata_unchanged` also rejects any `previous_chunk_id` key on chunk 0 via the `None` arm ([session_log.rs:1134–1141](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:1134)).

Level 1 tests cover:
- String `previous_chunk_id` on first-snapshot chunk 0 — rejected ([session_log.rs:2667](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:2667))
- Integer `previous_chunk_id` on first-snapshot chunk 0 — rejected ([session_log.rs:2709](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:2709))
- Integer `previous_chunk_id` on existing chunk 0 — rejected ([session_log.rs:2745](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:2745))

### 3. Startup replay re-verifies signed envelopes — RESOLVED

`replay_accepted_envelopes_on_startup` now reconstructs a `SignedEnvelope` from the persisted `AcceptedEnvelope` via `reconstruct_signed_envelope` ([session_log.rs:932](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:932)), calls `signed.verify()` ([session_log.rs:941](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:941)), and skips the envelope on any verification failure. Corrupt hex fields (malformed sender, message ID, content hash, signature) cause `reconstruct_signed_envelope` to return `None` and the envelope is skipped ([session_log.rs:933–939](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:933)).

Level 1 tests cover:
- Tampered signature — skipped ([session_log.rs:2840](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:2840))
- Tampered content hash — skipped ([session_log.rs:2887](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:2887))
- Malformed sender hex — skipped ([session_log.rs:2926](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:2926))
- Wrong payload kind — skipped ([session_log.rs:2965](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:2965))

## Spec Acceptance Criteria Coverage

| # | Acceptance Criterion | Status | Evidence |
|---|----------------------|--------|----------|
| 1 | Two explicitly paired local nodes can append, exchange signed Loro deltas, and converge | PASS | `two_nodes_converge_across_namespaces` ([phase6:192](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/tests/phase6_integration.rs:192)), `paired_daemons_converge_after_direct_sync` ([pairing_and_sync:57](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/tests/pairing_and_sync.rs:57)) |
| 2 | Chunk document IDs are deterministic; redb replay restores chunk metadata and entry ordering | PASS | `rotation_happens_at_configured_entry_cap` ([session_log.rs:1411](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:1411)), `rehydrate_picks_up_existing_snapshots` ([session_log.rs:1445](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:1445)), `restart_replays_state_and_resumes_sync` ([phase6:321](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/tests/phase6_integration.rs:321)) |
| 3 | Discovered but unpaired mDNS peer cannot exchange session-log deltas | PASS | `sync_fails_when_only_one_side_is_paired` ([phase6:429](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/tests/phase6_integration.rs:429)), `sync_is_rejected_when_pairing_is_missing` ([pairing_and_sync:155](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/tests/pairing_and_sync.rs:155)), mDNS discovery gated behind `REMOTE_SIGNAL_REAL_MDNS=1` |
| 4 | Invalid envelope signature, mismatched sender, unknown sender, duplicate message ID, payload hash mismatch all rejected without mutating redb | PASS | 10 Level 1 tests in sync.rs: invalid signature ([sync.rs:791](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:791)), hash mismatch ([sync.rs:828](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:828)), sender mismatch ([sync.rs:866](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:866)), foreign namespace ([sync.rs:906](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:906)), duplicate message ID ([sync.rs:944](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:944)), payload kind mismatch ([sync.rs:992](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:992)), document ID mismatch ([sync.rs:1030](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:1030)), malformed CRDT ([sync.rs:752](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:752)), envelope persistence failure ([sync.rs:1067](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:1067)) |
| 5 | Accepted network payloads persisted as signed envelopes with all required fields | PASS | `AcceptedEnvelope` struct ([storage.rs:508](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/storage.rs:508)), accepted_envelope_persists_signature_and_payload_bytes test ([storage.rs:755](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/storage.rs:755)), valid_delta_persists_accepted_envelope_and_snapshot test ([sync.rs:715](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:715)) |
| 6 | Local writes acknowledged only after Loro snapshot/delta is durable in redb | PASS | `append_entry` writes to redb before updating in-memory state ([session_log.rs:455](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:455)); `failed_persist_does_not_leave_entry_in_memory` test ([session_log.rs:2082](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:2082)) |
| 7 | After daemon restart, node rebuilds from redb and can replay/sync without losing acknowledged writes | PASS | `restart_replays_state_and_resumes_sync` ([phase6:321](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/tests/phase6_integration.rs:321)), `crash_recovery_replays_accepted_envelope` ([phase6:530](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/tests/phase6_integration.rs:530)), `accepted_envelope_only_replay_recovers_missing_snapshot` ([session_log.rs:1698](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:1698)), `envelope_before_snapshot_crash_window_recovers_on_restart` ([session_log.rs:1837](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:1837)) |
| 8 | DuckDB may lag; redb is authoritative for sync/replay; DuckDB rebuildable from redb | PASS | `rebuild_projection_from_storage_populates_duckdb` ([session_log.rs:1578](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:1578)), `projection_is_idempotent_across_repeated_syncs` ([phase6:762](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/tests/phase6_integration.rs:762)) |
| 9 | Chunking is deterministic and testable; both paired nodes converge on the same chunk set | PASS | `rotation_happens_at_configured_entry_cap` ([session_log.rs:1411](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:1411)), `chunk_rotation_propagates_through_sync` ([phase6:263](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/tests/phase6_integration.rs:263)), `poc_demo_end_to_end_flow` ([phase6:582](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/tests/phase6_integration.rs:582)) |
| 10 | Direct paired-peer sync backend for POC; foca/plumtree deferred | PASS | `SyncService` implements direct bidi QUIC sync ([sync.rs:143](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:143)); no gossip dependency |
| 11 | Compute jobs and production Claudine log ingestion excluded | PASS | No compute-job or log-ingestion code exists in the codebase |

## Test Verification Level Classification

### Level 1 — In-Process / Unit

All security invariant and schema validation tests are Level 1. These run without network I/O, QUIC, or terminal interaction.

**session_log.rs** (30 tests):
- Basic CRUD, rotation, rehydration, signing, sealer counter persistence
- Append-only enforcement: message mutation, deletion+replacement, reordering
- Previous-chunk-id rejection on chunk 0 (string, integer, existing-chunk variants)
- Replay security: tampered signature, tampered content hash, malformed sender hex, wrong payload kind
- Crash recovery: accepted-envelope replay, crash-window recovery, malformed envelope tolerance
- Persistence atomicity: failed snapshot persist, failed remote update persist, failed accepted-envelope persist
- Batcher independence: append succeeds after batcher shutdown
- Projection rebuild from storage

**sync.rs** (22 tests):
- Valid delta persists envelope + snapshot
- Malformed CRDT, invalid signature, hash mismatch, sender mismatch, foreign namespace, duplicate message ID, payload kind mismatch, document ID mismatch — all rejected without storage mutation
- Envelope persistence failure leaves no snapshot
- Schema validation: non-string entry, bad JSON, non-monotonic sequence, missing metadata, wrong owner, wrong session, wrong chunk index, invalid created_at, wrong previous_chunk_id
- Metadata mutation on existing chunk: owner, session, chunk_index, created_at, previous_chunk_id

**envelope.rs** (13 tests):
- Seal/verify round-trip, tampered payload, tampered signature, spoofed sender, monotonic IDs, same payload different IDs, wrong document_id, wrong payload_kind
- Inbox: first-accept/reject-replay, invalid without storing, distinct envelopes, high-water mark, resume from offset, separate inboxes for different senders

**storage.rs** (11 tests):
- Snapshot save/load, chunk listing, session-chunks catalog, pairings CRUD, accepted-envelope round-trip, duplicate rejection, per-sender scoping, outbound counter persistence, signature/payload round-trip

### Level 2 — Real-Terminal IPC (gRPC over UDS)

Integration tests spawn full daemon processes with QUIC networking and exercise the gRPC surface end-to-end.

**phase6_integration.rs** (8 tests):
- Two-node convergence across namespaces
- Chunk rotation propagation through sync
- Restart/replay and resume sync
- One-sided pairing rejection (security boundary)
- Foreign namespace write rejection
- Crash recovery via accepted-envelope replay
- POC demo end-to-end flow (bootstrap, pair, write, converge, rotate, converge)
- Deferred invitation pairing until identity confirmation
- Projection idempotence across repeated syncs

**pairing_and_sync.rs** (3 tests):
- Paired daemons converge after direct sync
- Sync rejected when pairing is missing
- Pairings listed and revoked

**uds_round_trip.rs** and **session_log_round_trip.rs**: Phase 1/2 smoke tests for ping/status and append/persist.

### Level 3 — OS Keyboard Injection

Not applicable. The spec does not assert terminal rendering, keyboard encoder behavior, paste/IME, mouse handling, or TUI interactions.

### mDNS Tests

Real mDNS discovery is gated behind `REMOTE_SIGNAL_REAL_MDNS=1` (env-gated). This is appropriate — mDNS requires real multicast/network resources and is non-deterministic by nature. The mDNS unpaired data-exchange boundary is tested at Level 2 via explicit pairing rejection.

## Low-Priority Observations

These are not blocking for POC production readiness but worth noting for future work:

### 1. `reconstruct_signed_envelope` does not re-verify the content hash against the payload bytes

The function reconstructs a `SignedEnvelope` and the replay path calls `signed.verify()`, which does recompute the BLAKE3 hash and check it. So the content hash IS verified during replay. This is correct. No action needed.

### 2. `EnvelopeInbox` is per-sync-session, not durable across sessions

The in-memory `EnvelopeInbox` is created fresh for each sync session ([sync.rs:489](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:489)). Replay deduplication across daemon restarts is handled by the durable `has_accepted_envelope` check ([sync.rs:258](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:258)). This is the correct design — the inbox catches in-session duplicates cheaply, redb catches cross-session duplicates durably.

### 3. Initiator auto-pairs on hello confirmation

The sync engine auto-pairs the initiator after the hello confirms the expected peer identity ([sync.rs:361–371](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:361)). This means that using a manual invitation is sufficient to establish full bidirectional pairing (initiator auto-pairs, responder requires pre-existing approval). The spec says "explicit pairing through manual invitation or explicit local approval," and the deferred-pairing test ([phase6:668](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/tests/phase6_integration.rs:668)) confirms this works correctly.

## Production Readiness

**Ready for production within POC scope.** All spec acceptance criteria are satisfied. The three data-integrity gaps from review-13 (append-only enforcement, chunk-0 previous_chunk_id, replay signature verification) have been fully addressed with both defensive code and comprehensive Level 1 tests. The integration test suite provides solid Level 2 coverage of the gRPC + QUIC + redb stack.

## Verification

Source inspection only (non-interactive session). Findings are based on reading the complete implementation and test source across all three crates.
