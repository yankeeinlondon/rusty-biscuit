---
ready: false
agent: codex
model: ""
---

# Review: Remote Signal

## Findings

### High: signed but invalid Loro payloads are persisted as accepted before the payload is proven applicable

The sync receiver now writes the accepted-envelope row before mutating the session snapshot, which closes the previous "snapshot committed but envelope missing" crash window. The new ordering still has an acceptance-boundary bug: after signature/hash/dedup checks, the receiver saves the envelope as accepted (`claudine/remote-signal/daemon/src/sync.rs:429`, `claudine/remote-signal/daemon/src/sync.rs:446`) and only then imports the payload into Loro (`claudine/remote-signal/daemon/src/sync.rs:452`). `SignedEnvelope::verify` authenticates bytes but does not validate that those bytes are a valid Loro snapshot/delta (`claudine/remote-signal/core/src/envelope.rs:213`). If a paired-but-buggy or malicious peer signs arbitrary bytes for its own chunk, `save_accepted_envelope` durably records the message before `apply_remote_update` can reject the malformed CRDT bytes.

That leaves redb in a poisoned state: the snapshot was not updated, but the duplicate-message/audit table says the message was accepted. On restart, manager construction replays all persisted accepted envelopes (`claudine/remote-signal/daemon/src/session_log.rs:875`, `claudine/remote-signal/daemon/src/session_log.rs:887`, `claudine/remote-signal/daemon/src/session_log.rs:899`), so the same malformed payload can make daemon startup fail through `SessionLogManager::new` (`claudine/remote-signal/daemon/src/server.rs:318`). Even without restart, a later retransmit of the same message ID is now rejected as a duplicate rather than retried or treated as never accepted.

Verification level: Level 1 is appropriate. I found tests for invalid envelope signatures/hash mismatches, duplicate storage, envelope-before-snapshot crash recovery, and happy accepted-envelope replay, but no test for a correctly signed envelope whose payload is not a valid Loro update. Add a Level 1 sync/session-manager test that signs malformed bytes from a paired sender, asserts the update is rejected without an accepted-envelope row, and verifies a restart still succeeds. The fix should make "accepted" mean "verified and successfully applied", either by validating/importing into a temporary Loro doc before saving the envelope or by committing the envelope and snapshot through an atomic state transition that can distinguish pending from accepted.

### Medium: projection idempotence is only covered at the SQL unit layer, not through the remote sync/query path

The previous duplicate-projection issue is plausibly fixed by the new `UNIQUE(chunk_id, sequence)` constraint and `INSERT OR IGNORE` in DuckDB (`claudine/remote-signal/daemon/src/projection.rs:17`, `claudine/remote-signal/daemon/src/projection.rs:124`, `claudine/remote-signal/daemon/src/projection.rs:138`). The test coverage proves that direct duplicate `Projection::append_rows` calls are ignored (`claudine/remote-signal/daemon/src/projection.rs:259`), but it does not exercise the user-observable failure mode: repeated incremental remote sync of the same chunk followed by `QueryProjection`.

Verification level: the existing test is Level 1, but it is not at the right behavioral surface. Add a Level 1 daemon/client integration test that syncs Alice to Bob, appends another Alice entry in the same chunk, syncs again, then calls Bob's `QueryProjection` for Alice's session and asserts exactly one row per sequence. This is not necessarily a code defect now, but it is a coverage gap for a requirement that is only visible through the gRPC projection API.

## Requirement Coverage Notes

- Paired two-node convergence, deterministic chunk IDs/chunk rotation, restart from redb snapshots, discovered-but-unpaired rejection, identity-confirmed invitation pairing, and local write durability before ack have meaningful Level 1 coverage.
- Signed-envelope rejection for bad signatures, hash mismatches, sender/document mismatches, ownership violations, and duplicates has useful Level 1 coverage.
- L2/L3 terminal verification is not applicable. This feature does not define terminal rendering, hotkeys, paste/IME, mouse, or OS keyboard behavior.

## Production Readiness

Not ready for production. The main protocol boundary still allows a signed but structurally invalid CRDT payload to be recorded as accepted before it is actually accepted by the CRDT layer, which can block retries and break restart.

## Verification

Attempted:

- `timeout 60 cargo test --color=never -p remote-signal-daemon envelope_before_snapshot_crash_window_recovers_on_restart -- --nocapture`

The command was stopped by the 60-second timeout while the cold workspace was still compiling dependencies, so I could not verify the suite in this non-interactive review session.
