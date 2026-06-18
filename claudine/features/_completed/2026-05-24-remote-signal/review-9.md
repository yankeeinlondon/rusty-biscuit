---
ready: false
agent: codex
model: ""
---

# Review: Remote Signal

## Findings

### High: malformed but signed CRDT payloads are persisted as accepted envelopes before structural validation

The signed-envelope path now persists the accepted-envelope row before applying the CRDT payload, which fixes the previous crash window. However, the implementation persists the envelope immediately after signature/hash/identity checks and before proving that the `payload_bytes` are valid Loro data for the target chunk. In `run_session`, the receiver builds `AcceptedEnvelope` and calls `save_accepted_envelope` before `apply_remote_update` imports the payload (`claudine/remote-signal/daemon/src/sync.rs:434`, `claudine/remote-signal/daemon/src/sync.rs:440`, `claudine/remote-signal/daemon/src/sync.rs:451`, `claudine/remote-signal/daemon/src/sync.rs:457`). If the import then fails, the envelope remains in redb as an accepted network payload even though the CRDT state rejected it.

That durable poison is replayed on every restart. `replay_accepted_envelopes_on_startup` iterates all accepted envelopes and feeds their payloads back into `apply_remote_update` (`claudine/remote-signal/daemon/src/session_log.rs:889`, `claudine/remote-signal/daemon/src/session_log.rs:891`, `claudine/remote-signal/daemon/src/session_log.rs:902`). The current unit test `malformed_loro_payload_in_accepted_envelope_breaks_restart` codifies the failure: a signed envelope with invalid Loro bytes in `accepted_envelopes` makes `SessionLogManager::with_clock` return an error on restart (`claudine/remote-signal/daemon/src/session_log.rs:1615`).

This violates the spec's rejection boundary in practice. The spec requires rejected network payloads to not mutate local redb state, and the signed envelope is supposed to define accepted replay data. A structurally invalid CRDT payload should not become an accepted envelope. The safer shape is to stage-import the payload into a temporary Loro doc first, then persist the accepted envelope and the resulting snapshot in the intended crash-recovery order. If the staged import fails, return `MalformedPayload` without writing either `accepted_envelopes` or snapshots.

Verification level: Level 1 is appropriate for this protocol/storage invariant. There is already a Level 1 test proving the restart failure once a malformed accepted envelope exists, but the implementation needs a sync-path test that sends or constructs a validly signed malformed payload and asserts no accepted-envelope row and no snapshot are persisted.

### Medium: envelope rejection is mostly unit-tested, not integration-tested at the sync/redb boundary

The spec explicitly requires invalid signatures, mismatched sender identity, unknown/unpaired senders, duplicate message IDs, and payload hash mismatches to be rejected without mutating redb. The core envelope unit tests cover hash/signature/document/kind tampering and in-memory duplicate detection, and the daemon tests cover unpaired peers and normal duplicate storage behavior. I did not find a sync-layer integration test that drives malformed `SyncDelta` frames through `run_session` and asserts redb stays unchanged for each rejection case.

This matters because the implementation has several checks outside `SignedEnvelope::verify`: hello sender matching, `document_id` versus `delta.chunk_id`, owner namespace enforcement, durable duplicate lookup, and accepted-envelope persistence. Those are the boundaries most likely to regress during protocol work.

Verification level: Level 1 integration is enough. Add tests around the direct sync frame handler, or a small test harness that can inject `SyncDelta` frames, for invalid signature, payload hash mismatch, mismatched hello sender, duplicate `(sender, message_id)`, foreign namespace, and payload-kind mismatch. Each test should assert both the returned error and unchanged `snapshot_count` / `accepted_envelope_count`.

## Requirement Coverage Notes

- Paired two-node convergence, deterministic chunk IDs, chunk rotation, restart/replay from snapshots, direct paired-peer sync, manual invitation, explicit pairing, and DuckDB rebuild/idempotence have meaningful Level 1 integration coverage.
- The mDNS discovery and mDNS unpaired-data boundary tests are real-network/env-gated behind `REMOTE_SIGNAL_REAL_MDNS=1`, which is the right class of verification for mDNS behavior but not part of the default fast suite.
- L2/L3 terminal verification is not applicable here. The feature does not specify terminal rendering, keyboard input, paste/IME, mouse behavior, or terminal-emulator encoder behavior.
- The remaining blocker is not test tier mismatch; it is the durable acceptance of malformed CRDT payloads.

## Production Readiness

Not ready for production. The main protocol is close, but a paired peer can persist a signed malformed CRDT payload as an accepted envelope and make the receiver fail to restart.

## Verification

Attempted:

- `GIT_TERMINAL_PROMPT=0 cargo test --color=never -p remote-signal-daemon malformed_loro_payload_in_accepted_envelope_breaks_restart -- --nocapture`

The command was still compiling after roughly 60 seconds from a cold build, so I killed it to respect the non-interactive session limit. Review findings are from source inspection and the existing test bodies.
