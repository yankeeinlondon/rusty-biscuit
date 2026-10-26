---
$schema: feature-review.yaml
ready: true
findings: []
observations:
    - Fingerprint helpers bypass the repository hashing library
human_in_the_loop: null
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-26T14:47:44-07:00"
spec: 2026-10-26-dmls-frontmatter-typing-allocation/spec.md
implemented: false
description: "A **fix** review of `2026-10-26-dmls-frontmatter-typing-allocation/spec.md`"
fix: 2026-10-26-dmls-frontmatter-typing-allocation/review-1.md
---

# Review 1: DMLS frontmatter typing allocation

The fix is production ready against its specification. There are no blocking
findings, deferred requirements, or decisions requiring human review.

## Implementation decisions

Two independent reviewers read the specification and implementation log before
the implementation comparison. Both found `decisions_for_author: []` and no
requested work awaiting an author decision. The implementation's choices are
settled; they do not require reaffirmation.

- Schema nodes use addresses within retained owners as snapshot-local identities.
  A safe append-only store keeps borrowed values stable. Merged properties with
  one source retain that source's identity, allowing recursive reuse.
- Each cursor request classifies its scalar without initializing the inventory.
  Diagnostics and document links lazily classify every scalar once, including
  negative results, and later requests reuse that classification.
- The overlay cache owns the current snapshot. Its identity includes buffer
  revision and text, tree freshness, and prepared schema inputs; retained old
  requests can finish without making an old snapshot current again.
- The implementation also repaired dependency tracking after malformed imported
  schemas and removed remaining hover allocations in declaration parsing,
  lifecycle discovery, alternative validation, and array-item synthesis.
- Prepared alternative validators and synthesized item definitions survive
  document edits in bounded, content-keyed server stores. Each store holds at
  most 256 entries before clearing; snapshots retain their own item owners.

The review's interpretations and alternatives are recorded under **Review 1 /
Design Decisions** in [the implementation log](implementation-log.md). In
particular, the allocation criterion applies to the existing benchmark's
measured edit loops. It does not promise that the first-ever hover over unseen
alternatives stays below 1 MB: the log explicitly reports about 16.8 MB of
one-time preparation. The existing missing-import creation limitation is also
disclosed; following existing invalidation does not introduce a new missing-file
discovery requirement.

## Findings

None. This is the first implementation review, so there are no previous
findings to reconcile.

## Requirement verification

| Spec requirement | Verification and result |
| --- | --- |
| Cursor-only expression and lifecycle lookup, before diagnostics | L1 `sequence_tests::cold_*` exercises the provider registry on 20 and 200 items: no typing or stack walk, at most one scalar classification, and bounded constructions. Includes expression, verb, key, completion, and definition requests. |
| Shared root, merged shapes, item definitions, and recursive identities | L1 `the_root_is_assembled_once_and_shared`, `identical_items_do_not_multiply_shared_constructions`, `recursive_depths_and_same_arm_items_share_one_view`, and `item_definitions_are_synthesized_once_for_every_item`; diagnostics construction counts remain constant when item counts increase. |
| Lazy inventory, cached misses, and reuse through fresh request overlays | L1 sequence tests check point-first and inventory-first classification, unresolved and non-expression results, repeated diagnostics, and document-link reuse. Snapshot tests use one long-lived `OverlayState` and compare responses with fresh states. |
| Invalidation and malformed-buffer recovery | L1 `every_input_change_starts_a_snapshot_that_answers_from_its_own_inputs` covers revisions, edits, imported schema changes and repairs, configuration, registry, context, malformed buffers, retained old requests, and close/reopen. |
| Unchanged responses and selection rules | Twelve frozen-input response tests pin hover text/ranges, completion order/edits, diagnostics codes/ranges/messages/severities, and navigation. Focused L1 tests cover sibling selections, missing/unknown/duplicate/conflicting discriminants, literal types, recursive references, both array-crossing modes, and plain/quoted/block/tagged values. Existing protocol and L2 suites pass. |
| Passive cache construction | Snapshot tests assert no extra reads, hashes, parses, enumerations, registry scans, context capture, preparation, effect-engine construction, or network attempts during their requests. File-target resolution remains outside the typing cache. |
| Content-keyed reuse preserves answers | L1 snapshot tests compare cached item hovers with fresh-state hovers after edits and schema changes. Library tests check prepared matcher behavior, recursive dependency fingerprints, and lifecycle catalog transitions. |
| Measured allocation target | Re-ran `cargo bench -p dmls --bench pipeline_work_a3`: all 24 rows exactly match the recorded Phase 5 work, allocation-count, allocated-byte, and peak-live-byte columns. Every measured expression hover allocates 105,025 bytes and verb hover 58,763 bytes at both sizes, warm and cold. |

The benchmark retains **overlay → diagnostics → completion → verb hover →
expression hover**, its initial warm-up, and five timed repetitions. Warm
diagnostics allocate 437,251,200 → 472,521,047 bytes for the two sizes; cold
diagnostics allocate 840,132,995 → 875,402,842 bytes. Scalar classifications grow
66 → 453 while root and merge construction counts stay constant. This supports
the fixed-schema typing mechanism; it does not establish arbitrary-schema
scaling or remove the separately identified lifecycle call-check work.

These changes concern in-process analysis and protocol responses, appropriately
verified at L1. They introduce no keyboard-encoder or terminal-rendering behavior
requiring L3 or new L2 cases. The existing L2 suites were rerun unchanged. The
configuration input matrix does not apply: no accepted file/configuration shapes
or validation rules changed; lifecycle catalog classification is unchanged and
its cache now also retains negative classifications.

## Verification commands

- `just test` in `darkmatter/`: 12,571 passed, 10 skipped. The first run
  reported one slow test; a follow-up with slow-test reporting also passed all
  12,571 tests and reported no slow tests. No over-five-second test reproduced
  in that follow-up.
- `just test-l2` in `darkmatter/`: 18 library, 69 CLI, and 3 DMLS tests passed;
  the existing harnesses ran without window activation.
- `just lint` in `darkmatter/`: passed, including the Zed WASM compilation check.
- `just check-tier-coverage darkmatter`: zero stranded tests. New modules are
  included in declared L1 targets and use enabled features.
- `cargo bench -p dmls --bench pipeline_work_a3`: passed; allocation and work
  columns match `phase5-a3.tsv` exactly.

The root-level `just test dmls` attempt failed before test execution because its
generated command supplied an empty package argument. The package-area command
above succeeded. Cross-OS evidence remains CI's responsibility and is not a
readiness condition in this review.

## Observations

### Fingerprint helpers bypass the repository hashing library

This is a repository-convention issue, with no demonstrated editor-response
defect. The new `DebugHasher` implementations in the `dmls` package's
[`overlay/analysis.rs`](../../dmls/src/overlay/analysis.rs) and the `darkmatter`
library's [`simplified/query.rs`](../../lib/src/markdown/schemas/simplified/query.rs)
use `std::collections::hash_map::DefaultHasher`. The session instructions require
hashing through `biscuit-hash`. The log explains the streaming requirement and
the lack of a streaming API in that library, but an existing use elsewhere does
not waive that instruction. Keep streaming to avoid rebuilding large strings;
route the hashing through a shared streaming facility in `biscuit-hash`. This
has neither a violated spec acceptance criterion nor a demonstrated response
regression, so it is an observation under this review's authority rules.
