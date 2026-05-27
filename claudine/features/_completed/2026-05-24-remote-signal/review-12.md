---
ready: false
agent: codex
model: ""
---

# Review: Remote Signal

## Findings

### High: metadata corruption is still accepted for chunks that already exist locally

The spec requires every session-log chunk document to carry deterministic metadata (`owner_node_id`, `session_id`, `chunk_index`, `created_at`, and `previous_chunk_id`) and defines the signed envelope as the rejection boundary for malformed payloads. The new-chunk path now validates that metadata, but the existing-chunk path still only validates entries.

In `stage_remote_update`, when the chunk already exists in memory, the imported remote payload is checked with `validate_remote_entries` and then persisted using the old in-memory `ChunkMetadata` value ([session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:650), [session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:658), [session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:671)). The metadata validator is only called in the `else` branch for a previously unknown chunk ([session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:681), [session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:682)). Because the committed snapshot comes from the staged Loro document, a signed payload from a paired owner can mutate the document's `metadata` map after the first valid snapshot and that corrupted map becomes durable redb state.

The current metadata rejection tests cover only first-snapshot creation: missing metadata, wrong owner, wrong session, wrong chunk index, invalid timestamp, and wrong previous chunk are all sent to chunks that are not already present locally ([sync.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:1310), [sync.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:1347), [sync.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:1391), [sync.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:1435), [sync.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:1479), [sync.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:1523)). Add tests that first accept a valid snapshot for a chunk, then send a signed update or replacement snapshot that mutates each metadata field, and assert no accepted-envelope row or snapshot mutation is persisted.

Verification level: Level 1 is appropriate for this protocol/schema invariant. L2/L3 terminal verification is not applicable.

## Requirement Coverage Notes

- Level 1 coverage exists for first-snapshot metadata validation, malformed entry payload rejection, invalid signatures, sender mismatch, payload hash mismatch, duplicate message IDs, document ID mismatch, payload-kind mismatch, ownership violation, restart/replay, deterministic chunk rotation, direct paired sync, and invitation pairing.
- Process/IPC integration coverage exists for two-daemon convergence, chunk propagation, restart recovery, one-sided pairing rejection, deferred invitation pairing, and projection idempotence.
- Real mDNS discovery and the unpaired mDNS data-exchange boundary are env-gated behind `REMOTE_SIGNAL_REAL_MDNS=1`, which is appropriate for multicast/network-resource coverage.
- L2/L3 terminal verification is not applicable. The spec does not assert terminal rendering, keyboard encoder behavior, paste/IME, mouse, or TUI interactions.

## Production Readiness

Not ready for production. A paired peer can still get a signed, schema-invalid chunk document persisted after the receiver already has an initial valid copy of that chunk.

## Verification

Attempted:

- `cargo test -p remote-signal-daemon missing_metadata_rejected_without_persistence --color=never`

The command was still compiling after roughly 60 seconds from a cold build, so I terminated it to avoid leaving a long-running process in the non-interactive session. Findings are from source inspection and existing test bodies.
