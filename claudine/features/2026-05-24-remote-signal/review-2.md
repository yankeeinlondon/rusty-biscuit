---
ready: false
agent: codex
model: ""
---

# Review: Remote Signal

## Findings

### High: Message IDs are not monotonic per sender across daemon lifetime

The spec requires a monotonic per-sender message ID and durable duplicate-message rejection. The new `EnvelopeSealer` issues IDs from an in-memory counter that always starts at zero (`claudine/remote-signal/core/src/envelope.rs:132`, `claudine/remote-signal/core/src/envelope.rs:140`, `claudine/remote-signal/core/src/envelope.rs:158`). The daemon also creates separate sealers for the same node identity in `SessionLogManager` and `SyncService` (`claudine/remote-signal/daemon/src/session_log.rs:272`, `claudine/remote-signal/daemon/src/sync.rs:146`), so a single sender can produce duplicate ID `0` from different code paths. After daemon restart, the sync sealer resets and starts reusing old message IDs again.

The durable duplicate check is keyed only by `message_id_hex` (`claudine/remote-signal/daemon/src/storage.rs:333`, `claudine/remote-signal/daemon/src/storage.rs:361`, `claudine/remote-signal/daemon/src/sync.rs:402`). Because every sender starts at the same counter, Bob's first accepted envelope and Carol's first accepted envelope have the same message ID. A future 3-peer sync can falsely reject a valid envelope from a different paired sender, while a restarted sender can also collide with its own old IDs.

Verification level: current coverage is Level 1 only, which is appropriate for this protocol requirement, but the needed cases are missing. Add L1 tests for two different senders both sending message ID `0`, for restart continuing the outbound counter, and for duplicate rejection scoped by `(sender, message_id)` rather than message ID alone.

### High: Persisted accepted envelopes are only metadata and are not atomic with redb mutation

The spec says accepted network payloads are represented as persisted signed envelopes, including sender identity, document identity, payload kind, per-sender message ID, payload hash, and payload bytes. It also says restart/replay, deduplication, and rejection behavior are defined by the same data accepted from the network. The persisted `AcceptedEnvelope` omits the signature and payload bytes entirely (`claudine/remote-signal/daemon/src/storage.rs:410`), so redb does not contain the accepted signed envelope and cannot replay or audit the exact network payload that was verified.

The persistence order also leaves a consistency hole. `run_session` verifies and applies the payload first (`claudine/remote-signal/daemon/src/sync.rs:408`), and `apply_remote_update` persists the resulting Loro snapshot to redb (`claudine/remote-signal/daemon/src/session_log.rs:683`). Only after that does `run_session` save the accepted-envelope record (`claudine/remote-signal/daemon/src/sync.rs:414`). A crash or storage error between those steps leaves redb mutated by a network payload that is not represented in the accepted-envelope table.

Verification level: Level 1 protocol/storage tests are appropriate, but current tests only round-trip metadata rows. Add a daemon-level test that asserts the full signed envelope bytes are persisted before or atomically with the snapshot mutation, plus a failure-path test showing that an envelope persistence failure does not leave an applied snapshot behind.

### Medium: Real mDNS gating uses an undocumented hard-fail variable value

The mDNS test was correctly moved to a `real_` test name and skips by default (`claudine/remote-signal/daemon/tests/peer_discovery.rs:101`). However, the guard checks `BISCUIT_TEST_LEVEL_REQUIRED=real` (`claudine/remote-signal/daemon/tests/peer_discovery.rs:103`), while the repo testing guide reserves `BISCUIT_TEST_LEVEL_REQUIRED` for numeric Level 2/3 terminal tests and says real-resource tests should use per-package environment variables. This is a small ergonomics mismatch, but it will confuse anyone trying to run the real tier through the standard recipes.

Verification level: this is not an L1/L2/L3 terminal mismatch. Keep the `real_` naming and skip behavior, but switch to a remote-signal-specific opt-in such as `REMOTE_SIGNAL_REAL_MDNS=1`, or document the exact variable in the remote-signal justfile/docs.

## Requirement Coverage Notes

- Paired local nodes converge through direct QUIC sync: Level 1 integration coverage exists.
- Deterministic chunk IDs and chunk rotation: Level 1 unit/integration coverage exists.
- Discovered but unpaired peers cannot exchange session-log deltas: Level 1 integration coverage exists for the gRPC-triggered sync path; real mDNS discovery is gated as a real-resource test.
- Invalid signature, hash mismatch, sender/document/kind mismatch: Level 1 unit/protocol coverage exists for envelope verification and sync-path checks.
- Duplicate message IDs: Level 1 coverage exists only for a single in-process inbox and metadata storage; durable per-sender behavior across multiple senders/restarts is missing.
- Persisted accepted signed envelopes: not implemented as specified because payload bytes and signatures are not persisted, and persistence is not atomic with snapshot mutation.
- Local write ack after redb durability: implemented by saving snapshots before returning append success.
- Restart/replay of Loro state: Level 1 integration coverage exists for source-of-truth reads and subsequent sync.
- DuckDB rebuild from redb: Level 1 coverage was added for rebuilding an empty projection from redb snapshots.
- L2/L3 terminal verification: not applicable to this feature because no terminal rendering or OS keyboard behavior is specified.

## Production Readiness

Not ready for production. Iteration 2 closes several earlier gaps, especially document/kind binding and DuckDB rebuild, but the monotonic per-sender replay boundary and persisted signed-envelope contract are still incomplete.
