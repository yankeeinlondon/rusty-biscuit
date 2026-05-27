---
ready: false
agent: codex
model: ""
---

# Review: Remote Signal

## Findings

### High: accepted network payloads can be applied without durable envelope state

The spec makes the signed envelope the acceptance boundary for network sync: accepted payloads must be persisted as signed envelopes, restart/replay and deduplication must be defined by that persisted data, and duplicate message IDs must be rejected. The inbound sync path still applies the verified payload to the session log before saving the accepted-envelope row. After `EnvelopeInbox::accept` verifies the envelope, `run_session` calls `apply_remote_update`, which persists the resulting Loro snapshot to redb, and only then calls `save_accepted_envelope` (`claudine/remote-signal/daemon/src/sync.rs:434`, `claudine/remote-signal/daemon/src/sync.rs:436`, `claudine/remote-signal/daemon/src/sync.rs:462`). `apply_remote_update` saves the snapshot before returning (`claudine/remote-signal/daemon/src/session_log.rs:706`, `claudine/remote-signal/daemon/src/session_log.rs:709`, `claudine/remote-signal/daemon/src/session_log.rs:712`).

That leaves a crash/failure window where the receiver has durable session-log state but no durable accepted-envelope record. If the process crashes after `save_snapshot` and before `save_accepted_envelope`, restart will show the replicated entries but `has_accepted_envelope(sender, message_id)` will be false, so the same envelope can be accepted again instead of being rejected as a duplicate. If `save_accepted_envelope` returns an error, the sync reports failure, but the remote payload has already been committed to the source-of-truth snapshot. This is the same class of boundary issue that was fixed for local `append_entry`, just one layer later in the signed-envelope path.

Verification level: Level 1 is appropriate. The suite now has Level 1 tests for staged local append failure, staged remote snapshot failure, envelope-before-snapshot replay, persisted accepted-envelope fields, scoped duplicate checks, and malformed Loro rejection. I did not find a test that injects failure after the remote snapshot write but before accepted-envelope persistence, nor a restart test that proves a previously applied envelope is rejected as a duplicate after that crash window. Add a Level 1 failure-injection test around `save_accepted_envelope` or a small sync-storage abstraction: accept a valid envelope, force accepted-envelope persistence to fail, assert the receiver does not expose the remote entry and does not persist a snapshot without the envelope; then cover the crash ordering by persisting the envelope before the snapshot and relying on startup replay. A robust implementation would stage/validate the Loro import first, persist the accepted envelope, then persist/swap the snapshot so durable replay/dedup state exists before the payload becomes visible.

## Requirement Coverage Notes

- Paired two-node convergence, direct sync, deterministic chunk IDs, chunk rotation, restart/replay from snapshots, explicit pairing, invitation identity confirmation, discovered-but-unpaired rejection, invalid signature/hash rejection, mismatched sender/document/payload-kind rejection, duplicate message storage checks, malformed Loro payload rejection, and DuckDB projection rebuild/idempotence all have meaningful Level 1 coverage.
- The previous mutation-before-`save_snapshot` gap is fixed for both local appends and `apply_remote_update`; the code stages Loro mutations and the new failure-injection tests exercise that boundary.
- The mDNS unpaired-peer boundary has a gated real-network test via `REMOTE_SIGNAL_REAL_MDNS=1`, which is the right class of verification for discovery-specific behavior.
- L2/L3 terminal verification is not applicable. This feature does not specify terminal rendering, keypress behavior, paste/IME, mouse behavior, or terminal-emulator input encoding.
- The remaining gap is a storage/order-of-acceptance issue in the network envelope path, not a terminal verification mismatch.

## Production Readiness

Not ready for production. The core POC is close, but accepted network payloads can still become durable without the signed-envelope record that defines replay and duplicate rejection.

## Verification

Attempted:

- `GIT_TERMINAL_PROMPT=0 cargo test --color=never -p remote-signal-daemon failed_persist -- --nocapture`

The command blocked waiting for Cargo's artifact directory lock, so I stopped it rather than exceeding the non-interactive session limit. Review findings are from source inspection.
