---
ready: false
agent: codex
model: ""
---

# Review: Remote Signal

## Findings

### High: existing chunk entries are not enforced as append-only

The spec defines each session-log chunk as deterministic metadata plus an append-only `entries` list. For an existing chunk, the receiver currently validates only that the post-import list is not shorter than the previous list and that decoded entry sequence numbers are strictly increasing ([session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:658), [session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:1121)). It does not compare the already-accepted prefix against the previous snapshot.

That leaves a rewrite path: a paired owner can send a signed update or replacement snapshot that changes an existing entry's `message`, `level`, `source`, timestamp, or metadata while keeping the same sequence ordering and length. `validate_remote_entries` accepts that shape, `commit_staged_update` persists the staged snapshot, and redb now contains rewritten log history ([session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:724)). This breaks the POC's append-only document model.

Add a validator for existing chunks that decodes the prior entries and the staged entries, requires `staged[..existing_entry_count] == previous_entries[..]`, and only allows new entries to be appended after that prefix. Add Level 1 tests that first accept a valid snapshot, then attempt signed mutations of an existing entry field, deletion+replacement with the same sequence, and reordering. Each case should assert no new accepted-envelope row and no snapshot mutation.

Verification level: Level 1 is appropriate because this is an in-process CRDT/schema invariant. No L2/L3 terminal coverage is applicable.

### High: `previous_chunk_id` is not rejected on part 0

The spec requires deterministic chunk metadata, including `previous_chunk_id`. For part 0, that value should be absent/none. The first-snapshot validator does not inspect `previous_chunk_id` at all when `expected_chunk_index == 0`; it returns `None` even if the signed snapshot includes a bogus previous chunk ([session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:1017)). The existing-chunk validator rejects a non-empty string for part 0, but silently accepts non-string values because the `None` branch only checks the value when it is a string ([session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:1101)).

The current tests cover a wrong `previous_chunk_id` for chunk 1 and a string mutation for an existing chunk 0, but they do not cover a first snapshot for chunk 0 with an unexpected previous pointer or a non-string previous pointer on an existing chunk ([sync.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:1524), [sync.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/sync.rs:1732)). A signed malformed part-0 snapshot can therefore become durable redb state.

Make the metadata rule explicit: for `chunk_index == 0`, reject any present `previous_chunk_id` unless the on-disk schema intentionally represents none in a single canonical way. Add Level 1 tests for first-snapshot and existing-chunk variants, including non-string values.

Verification level: Level 1 is appropriate for deterministic metadata validation. L2/L3 terminal coverage is not applicable.

### High: startup replay applies persisted accepted-envelope payloads without re-verifying the signed envelope

The spec says network sync and replay use the canonical signed envelope, and that restart/replay rejection behavior is defined by the same data accepted from the network. The implementation persists enough fields for that audit record: sender, message ID, document ID, payload kind, content hash, signature, and payload bytes ([storage.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/storage.rs:503)). However, startup replay discards the envelope semantics and re-applies only `document_id` plus `payload_bytes` ([session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:910), [session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:923)).

That means replay does not re-check the content hash, signature, sender identity, payload kind, or message ID shape before mutating in-memory state and potentially re-saving a snapshot. The tests prove crash-window recovery from a valid accepted envelope and tolerant skipping of malformed Loro bytes, but they do not prove that a tampered accepted-envelope row with valid Loro bytes and an invalid hash/signature is rejected during replay ([session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:1731), [session_log.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/remote-signal/daemon/src/session_log.rs:1904)).

Reconstruct `SignedEnvelope` from `AcceptedEnvelope` during replay, verify it with the same envelope verifier used on the live network path, check the document and payload kind before applying, and then replay the verified payload. Add Level 1 tests for invalid signature, hash mismatch, malformed sender/message/signature hex, and payload-kind/document mismatch in persisted accepted-envelope rows.

Verification level: Level 1 is appropriate for replay/security invariants. L2/L3 terminal coverage is not applicable.

## Requirement Coverage Notes

- The review-12 metadata mutation issue has been partially addressed: existing chunks now call `validate_metadata_unchanged`, and there are Level 1 tests for mutating each main metadata field.
- Level 1 coverage exists for invalid live-network signatures, payload hash mismatch, sender mismatch, duplicate message IDs, document ID mismatch, payload-kind mismatch, foreign namespace rejection, malformed Loro payloads, first-snapshot metadata validation, deterministic chunk rotation, restart/replay, and projection rebuild/idempotence.
- Process/gRPC integration coverage exists for two-daemon convergence, manual invitation pairing, one-sided pairing rejection, chunk propagation, restart recovery, accepted-envelope crash-window recovery, and projection idempotence.
- Real mDNS discovery and the mDNS unpaired data-exchange boundary are env-gated behind `REMOTE_SIGNAL_REAL_MDNS=1`. That is appropriate for a real multicast/network-resource test.
- L2/L3 terminal verification is not applicable to this feature as specified. The spec does not assert terminal rendering, keyboard encoder behavior, paste/IME, mouse handling, or TUI interactions.

## Production Readiness

Not ready for production. The remaining gaps are protocol/data-integrity issues: accepted chunks can still violate append-only and metadata invariants, and persisted envelope replay does not enforce the same signed-envelope boundary as live network ingestion.

## Verification

Attempted:

- `cargo test -p remote-signal-daemon metadata_owner_mutation_on_existing_chunk_rejected --color=never`

The command was still compiling dependencies after roughly 60 seconds from a cold build, so I terminated it to avoid leaving a long-running process in the non-interactive session. Findings are from source inspection and existing test bodies.
