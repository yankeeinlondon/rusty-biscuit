---
ready: false
agent: codex
model: ""
---

# Review: Remote Signal

## Findings

### High: accepted-envelope durability can block replay of an update that was never applied

The spec says the signed envelope is the acceptance boundary for restart/replay and deduplication, and that accepted payloads are persisted with their CRDT payload bytes. Iteration 3 now persists the signature and payload bytes, which closes part of the prior finding, but the accept sequence is still not crash-safe. During inbound sync, the daemon checks durable duplicate state, verifies the envelope, writes the accepted-envelope row, and only then imports the Loro payload into the source-of-truth snapshot (`claudine/remote-signal/daemon/src/sync.rs:416`, `claudine/remote-signal/daemon/src/sync.rs:421`, `claudine/remote-signal/daemon/src/sync.rs:438`, `claudine/remote-signal/daemon/src/sync.rs:444`; `claudine/remote-signal/daemon/src/session_log.rs:621`, `claudine/remote-signal/daemon/src/session_log.rs:708`).

If the process crashes, is killed, or hits an import/storage error after `save_accepted_envelope` but before `save_snapshot`, redb contains a durable duplicate marker for an update whose document snapshot never advanced. On restart, `rehydrate_from_storage` rebuilds only from snapshots (`claudine/remote-signal/daemon/src/session_log.rs:780`), and `iter_accepted_envelopes` is not used anywhere outside storage tests (`claudine/remote-signal/daemon/src/storage.rs:389`). A retry of the same envelope is then rejected as a duplicate even though the CRDT state is missing it.

Verification level: Level 1 protocol/storage coverage is appropriate, but missing. Add a test that simulates "accepted envelope persisted, snapshot absent" and proves restart either replays the persisted payload into the snapshot or does not mark the envelope accepted until the snapshot mutation is durable. The implementation needs one atomic accept operation, or startup replay from accepted envelopes that can repair snapshots before duplicate rejection is enforced.

### High: paired peers can still write documents owned by another node

The spec's ownership model says the local daemon/client may write local session documents, while remote mesh peers are read-only listeners. The current sync path only verifies that the envelope sender matches the peer's sync hello and that the document ID matches the delta frame (`claudine/remote-signal/daemon/src/sync.rs:392`, `claudine/remote-signal/daemon/src/sync.rs:397`). It does not verify that the document owner namespace is authorized for that sender before applying the update (`claudine/remote-signal/daemon/src/sync.rs:444`).

Separately, the local IPC API accepts any caller-supplied `owner_node_id` (`claudine/remote-signal/daemon/src/service.rs:123`, `claudine/remote-signal/daemon/src/service.rs:132`). Combined with unrestricted inbound apply, a paired node can send a validly signed delta for `session/<this-node-id>/...` or any arbitrary owner namespace and mutate the receiver's local redb snapshot. That makes pairing equivalent to write access for every document namespace, not read-only replication.

Verification level: Level 1 is appropriate for this protocol authorization invariant, but coverage is missing. Add negative sync tests where a paired peer attempts to push a chunk whose `owner_node_id` is not the sender's node ID and assert the receiver's redb snapshot/catalog is unchanged. Also decide whether local append should derive the owner from daemon identity instead of accepting it from clients.

### High: mDNS-discovered unpaired peers are not verified at the required path

The acceptance criteria explicitly call out that a discovered but unpaired mDNS peer cannot exchange session-log deltas. Current coverage proves manual-invitation sync is rejected without pairing (`claudine/remote-signal/daemon/tests/phase6_integration.rs:397`) and separately proves real mDNS discovery can populate the peer list when `REMOTE_SIGNAL_REAL_MDNS=1` (`claudine/remote-signal/daemon/tests/peer_discovery.rs:101`). There is no test that combines the actual mDNS discovery path with a session-log exchange attempt before approval.

Verification level: this is a real-network/resource requirement rather than terminal L1/L2/L3 behavior. The strongest current coverage is Level 1 for the manual path plus a real-resource discovery-only test, which does not verify the user-facing mDNS trust boundary. Add a `real_` mDNS test that discovers a peer, attempts any available data-exchange path before approval, verifies failure/no redb mutation, then approves and verifies sync begins.

## Requirement Coverage Notes

- Paired local nodes converge through direct QUIC sync: Level 1 integration coverage exists.
- Deterministic chunk IDs, restart/replay from redb snapshots, and chunk rotation: Level 1 coverage exists.
- Invalid signature, payload hash mismatch, sender mismatch, document mismatch, payload-kind mismatch, and duplicate IDs: Level 1 unit/protocol coverage exists for the normal rejection paths.
- Persisted signed envelopes: signature and payload bytes are now persisted, but replay/duplicate behavior is still inconsistent if the envelope row and snapshot mutation diverge.
- Remote read-only ownership model: not implemented as specified and lacks Level 1 negative coverage.
- mDNS unpaired data-exchange boundary: not verified on the real mDNS path.
- L2/L3 terminal verification: not applicable; the spec does not define terminal rendering or OS keyboard behavior.

## Production Readiness

Not ready for production. Iteration 3 fixes the prior message-ID durability/scope issue and persists full envelope data, but the accepted-envelope replay boundary and remote ownership authorization remain blockers.

## Verification

Attempted `cargo test --color=never -p remote-signal-core -p remote-signal-daemon -p remote-signal-client`, but stopped it after a cold dependency build ran into native dependency compilation for longer than the non-interactive session budget allowed.
