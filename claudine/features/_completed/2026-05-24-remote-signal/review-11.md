---
ready: false
agent: codex
model: ""
---

# Review: Remote Signal

## Findings

### High: signed remote chunk documents can omit or lie about the required metadata and still become durable state

The spec requires each session-log chunk document to include deterministic metadata: `owner_node_id`, `session_id`, `chunk_index`, `created_at`, and `previous_chunk_id`. The receiver still validates only the `entries` list. `stage_remote_update` imports the remote Loro payload and calls `validate_remote_entries`, but for a new remote chunk it then fabricates `ChunkMetadata` from the chunk path and local clock rather than reading and validating the document's `metadata` map ([session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:679), [session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:682), [session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:695)). `validate_remote_entries` only decodes list elements and checks monotonic sequence ordering ([session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:983)).

The tests encode the same gap: `make_valid_loro_snapshot` creates only an `entries` list, with no `metadata` container, and `valid_delta_persists_accepted_envelope_and_snapshot` expects that payload to be accepted ([sync.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:677), [sync.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:701)). A paired sender can therefore sign and persist a document whose in-document metadata is missing or inconsistent with the envelope/document ID. That leaves restart/replay and remote audit semantics dependent on locally fabricated metadata rather than the CRDT document shape the spec calls authoritative.

Verification level: Level 1 is appropriate. Add sync-path tests that sign snapshots with missing metadata, wrong owner, wrong session, wrong chunk index, wrong `previous_chunk_id`, and missing/invalid `created_at`, and assert no snapshot or accepted-envelope row is written.

### Medium: converged peers keep resending full snapshots because remote advertisements for our namespace are discarded

During sync, each side advertises only chunks it owns ([sync.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:377)). When reading the peer's advertisements, the code keeps only chunks whose owner is the peer ([sync.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:408), [sync.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:416)). But the next phase uses `remote_state.get(chunk)` for chunks owned by the local node to decide what the peer is missing ([sync.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:438), [sync.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:439)). That lookup can never succeed for local-owned chunks, so `export_updates_since` is called with `None` and exports another full snapshot each sync.

This does not usually break convergence because Loro imports are idempotent, and the projection has a repeated-sync test. It does break the protocol contract that peers exchange state vectors and push only missing deltas. It also causes unnecessary network traffic, new signed envelopes, accepted-envelope rows, and snapshot writes on every sync after convergence.

Verification level: Level 1 is appropriate. Add a two-node direct-sync test that runs a second sync after convergence and asserts no chunk advances, no extra accepted-envelope row is created for already-converged chunks, and the sender does not emit snapshot payloads for chunks the peer already advertised.

## Requirement Coverage Notes

- Level 1 coverage exists for two-node convergence, restart/replay, deterministic chunk rotation, direct paired sync, invitation pairing, invalid signatures, sender mismatch, payload hash mismatch, duplicate message IDs, and malformed entry payload rejection.
- Real mDNS discovery and the unpaired mDNS data-exchange boundary are gated behind `REMOTE_SIGNAL_REAL_MDNS=1`, which is appropriate for multicast/network-resource coverage.
- L2/L3 terminal verification is not applicable. The spec does not assert terminal rendering, keyboard encoder behavior, paste/IME, mouse, or TUI interactions.

## Production Readiness

Not ready for production. The remaining blockers are protocol/data invariants: accepted remote documents are not validated against the required metadata schema, and the direct sync state-vector path keeps resending snapshots after convergence.

## Verification

Attempted:

- `cargo test -p remote-signal-daemon projection_is_idempotent_across_repeated_syncs --color=never`

The command was still compiling after roughly 60 seconds from a cold build, so I stopped it to avoid hanging the non-interactive session. Findings are from source inspection and existing test bodies.
