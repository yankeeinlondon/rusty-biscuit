---
ready: false
agent: codex
model: ""
---

# Remote Signal Review - Iteration 1

## Findings

### High: paired peers can write documents owned by another node

The spec says session-log data is locally owned: the local daemon/client may write local session documents, while remote mesh peers are read-only listeners. The implementation does not enforce that boundary. `AppendEntryRequest` accepts any `owner_node_id` from a local client (`claudine/remote-signal/daemon/src/service.rs:123`), and inbound sync accepts any `delta.chunk_id` from a paired peer, verifies only that the envelope sender matches the sync hello, then persists the update under that chunk (`claudine/remote-signal/daemon/src/sync.rs:386`, `claudine/remote-signal/daemon/src/sync.rs:392`, `claudine/remote-signal/daemon/src/sync.rs:444`; `claudine/remote-signal/daemon/src/session_log.rs:621`, `claudine/remote-signal/daemon/src/session_log.rs:708`).

A paired remote can therefore send `session/<this-node-id>/...` and mutate a locally owned session namespace. This violates the ownership model and makes "paired" equivalent to "can write arbitrary document IDs."

Verification level: Level 1 is appropriate for this protocol invariant, but coverage is missing. Add tests that a paired peer cannot sync a document whose `owner_node_id` is not the sender's node id, and that the rejected payload leaves redb unchanged. Also decide whether the local IPC API should default or restrict `owner_node_id` to the daemon identity.

### High: accepted envelopes are recorded before the CRDT payload is successfully applied

During inbound sync, the daemon checks durable duplicate state, verifies the envelope, immediately saves the accepted-envelope audit row, and only then imports the Loro payload (`claudine/remote-signal/daemon/src/sync.rs:416`, `claudine/remote-signal/daemon/src/sync.rs:421`, `claudine/remote-signal/daemon/src/sync.rs:438`, `claudine/remote-signal/daemon/src/sync.rs:444`). If the payload is signed by a paired sender but malformed as a Loro update, `apply_remote_update` returns an error after the message ID has already been persisted as accepted.

On retry, the same payload is rejected as a duplicate even though it never mutated the source-of-truth snapshot. That breaks the acceptance criterion that rejection behavior and replay are defined by the same data that was actually accepted from the network.

Verification level: Level 1 is appropriate and missing. Add a test that sends a validly signed but invalid Loro payload, asserts no accepted-envelope row is committed, and asserts a later valid retry with the same message ID is not incorrectly blocked. The fix should make envelope persistence and snapshot mutation one atomic accept operation, or persist only after successful import.

### High: mDNS unpaired-peer data-exchange behavior is not verified

The acceptance criteria specifically say a discovered but unpaired mDNS peer cannot exchange session-log deltas. Current tests cover manual invitation without pairing (`claudine/remote-signal/daemon/tests/phase6_integration.rs:397`) and real mDNS discovery only (`claudine/remote-signal/daemon/tests/peer_discovery.rs:101`), but there is no test where two nodes discover each other over mDNS and then fail to exchange session-log data until approval.

Verification level: this is a real-network behavior, not a pure in-process protocol property. The strongest current coverage is Level 1/manual-QUIC plus a gated real-resource discovery test, which is below the requirement. Add a real-resource mDNS test gated by `REMOTE_SIGNAL_REAL_MDNS=1` that discovers peers, attempts sync before pairing, asserts `FailedPrecondition`/no data leak, then approves and verifies sync succeeds.

### Medium: the reviewed design document is empty

The requested technical design file, `claudine/docs/remote-signal/design.md`, is zero bytes. The implementation can be reviewed against `claudine/features/2026-05-24-remote-signal/spec.md` and `plan.md`, but the canonical docs path named in the review request does not preserve the architecture, acceptance criteria, or the decisions made during implementation.

This is a production-readiness issue for maintainability rather than a runtime defect. Copy or consolidate the accepted design into `claudine/docs/remote-signal/design.md`, including the final decisions that differ from the original stack notes.

## Verification Matrix

- Paired two-node direct sync convergence: Level 1 integration coverage exists in `pairing_and_sync.rs` and `phase6_integration.rs`.
- Deterministic chunk IDs and chunk rotation: Level 1 unit/integration coverage exists.
- redb durability, restart, and replay: Level 1 integration coverage exists for source-of-truth reads and resumed sync.
- DuckDB projection rebuild from redb: Level 1 coverage exists in `SessionLogManager` tests.
- Envelope signature, payload hash, document ID, payload kind, sender mismatch, and duplicate storage: Level 1 unit/protocol coverage exists for the core pieces.
- Remote ownership/read-only peer model: missing Level 1 negative coverage and likely not implemented.
- Malformed but signed CRDT payload rejection without durable false-accept: missing Level 1 negative coverage and likely broken.
- mDNS-discovered unpaired peer cannot exchange data: current coverage is below the required real-network level.

## Production Readiness

Not ready for production. The POC has meaningful coverage for happy-path sync, durability, and signed-envelope basics, but the remaining ownership and acceptance-order bugs are security/data-consistency blockers.
