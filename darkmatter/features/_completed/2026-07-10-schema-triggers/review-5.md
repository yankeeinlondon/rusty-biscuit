---
$schema: "@.claudine/schemas/review.yaml"
ready: false
implemented: true
agent: codex/default
created: 2026-07-11T10:31:36
---

# Review 5: Schema Triggers

## Findings

### High: open envelopes can retain stale trigger-load diagnostics

The new transition model correctly records that a repaired envelope must clear, or that failure ownership moved from envelope A to envelope B. The router, however, discards every transition whose envelope is open. This is unsafe because `refresh_all_diagnostics` refreshes open documents before consuming transitions, and the open-document iteration order does not establish that an envelope is refreshed after the consumer scan which creates the transition.

For example, with envelope A and a consumer open, A can be refreshed first while its old load error still exists. A's stale diagnostic is published. Refreshing the consumer then successfully repairs A or transfers failure ownership to B and queues an A-clear transition. The router sees that A is open and drops the clear, leaving the obsolete diagnostic published until another unrelated refresh. The same ordering issue can delay a newly assigned diagnostic on an open envelope.

This leaves review 4's ownership-transfer requirement incomplete at the LSP publication layer. The new state test proves only that transitions are generated, while the new LSP tests deliberately leave the envelope closed and therefore exercise only the branch that publishes transitions directly.

References:

- `darkmatter/dmls/src/router.rs:296-317` refreshes all open documents first, then unconditionally skips transitions for open URIs.
- `darkmatter/dmls/src/overlay/mod.rs:337-365` creates clear/set transitions during a later scan, after an earlier open-envelope refresh may already have published stale state.
- `darkmatter/dmls/src/overlay/mod.rs:798-866` validates transition generation but not router publication.
- `darkmatter/dmls/tests/lsp_session.rs:622-709` validates closed-envelope publication and repair only.

Suggested fix: coalesce transitions by envelope URI after all scans, then republish every affected envelope. For an open envelope, recompute its versioned diagnostics after the final registry state is settled; for a closed envelope, publish the transition's file-level diagnostics with `version: None`. Do not silently discard open-envelope transitions. Add a Level-1 LSP regression with A and a consumer both open that exercises A-invalid → repair A/B-invalid and asserts A clears and B receives the diagnostic immediately, independent of open-document iteration order. A simpler repair-only case with one open envelope should also assert immediate clearing.

## Review-4 Closure

Closed trigger envelopes now receive and clear load diagnostics in both watcher modes, and the consumer retains its last-good effective schema. The prior append-only error map has also been replaced by per-scope failure state that produces explicit ownership transitions.

The ownership-transfer finding is only partially closed. State transitions are correct, but the router discards them for open envelopes, so the client-visible diagnostic can remain stale.

## Test Rigor

This feature's observable behavior is filesystem discovery, schema parsing and matching, CLI output, and LSP state/diagnostic publication. Level 1 is appropriate; no requirement depends on terminal rendering, terminal input encoding, or OS keyboard injection, so Levels 2 and 3 are not required.

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Match grammar, combinators, arms, path matching, and vacuous lint | Level 1 unit tests | Appropriate |
| Ancestor discovery, ordering, shadowing, extensions, symlink exclusion, and case collisions | Level 1 filesystem tests | Appropriate |
| CLI compose/validate activation, assignment and shell re-resolution, raw mode, and trace output | Level 1 binary integration tests | Appropriate |
| CLI/DMLS dialect activation parity | Level 1 fixture-based integration tests | Appropriate |
| Transactional invalid-payload behavior and last-good registry retention | Level 1 unit and LSP tests | Appropriate |
| Failed scans publish and clear diagnostics for closed envelopes in both watcher modes | Level 1 LSP integration tests | Appropriate |
| Consecutive failed scans generate A-clear/B-set ownership transitions | Level 1 state test | Necessary but insufficient for client-visible behavior |
| Open-envelope repair and ownership transfer publish the final diagnostic state immediately | No LSP test; router behavior is ordering-dependent | Gap |

## Verification

The following targeted Level-1 nextest tests passed:

- `overlay::tests::consecutive_failed_scans_transfer_diagnostic_ownership`
- `server_rescan_publishes_and_clears_unopened_trigger_envelope_diagnostic`
- `client_watcher_publishes_and_clears_unopened_trigger_envelope_diagnostic`

No Level-2 or Level-3 run was warranted because this feature has no real-terminal rendering or OS-input requirements. A full package-area test or lint run was not performed in this review iteration.

## Production Readiness

Not ready for production. Closed-envelope diagnostics and transition generation are repaired, but open envelopes can still retain obsolete trigger-load diagnostics because the router drops their post-scan transitions.
