---
$schema: "@.claudine/schemas/review.yaml"
ready: false
implemented: true
agent: codex/default
created: 2026-07-11T09:56:46
---

# Review 4: Schema Triggers

## Findings

### High: trigger-load diagnostics are still limited to open envelope buffers

The specification requires DMLS to publish a schema-load diagnostic on the trigger envelope when a transactional registry scan fails. The new state retains the error by envelope path, but the only path from that state to `publishDiagnostics` is `OverlayState::for_document`, which is called by `RouterState::refresh_diagnostics` only for documents in the open-document store. `refresh_all_diagnostics` likewise iterates only open URIs.

Consequently, the normal background cases remain silent: a watched payload/import/example is changed or removed while its envelope is closed, or a watcher-less client saves an unrelated open document and the rescan discovers the failure. The consumer documents retain their last-good effective schemas, but DMLS never publishes the required diagnostic for the responsible closed envelope. The new LSP test does not cover this contract because it explicitly opens the envelope before removing the payload.

References:

- `darkmatter/dmls/src/overlay/mod.rs:207-239` exposes a retained load error only while building an overlay for that envelope document.
- `darkmatter/dmls/src/router.rs:268-300` publishes and refreshes diagnostics only for open documents.
- `darkmatter/dmls/src/router.rs:891-897` and `:961-977` rescan/refresh after saves and watcher events but have no path-targeted diagnostic publication for failed trigger envelopes.
- `darkmatter/dmls/tests/lsp_session.rs:544-612` opens the envelope at line 564, so it proves only the open-buffer case.

Suggested fix: make a trigger-registry refresh return a transition containing the current failed envelope URI/error and any envelope diagnostics that must be cleared. The router can then publish `dm.schema.prepare` directly for those file URIs, with `version: None`, regardless of open state, while open buffers remain authoritative for their text. Add Level-1 LSP tests for both watcher modes with the envelope unopened: remove or corrupt a payload, assert a diagnostic is published on the envelope URI and the consumer retains its prior schema, then repair the registry and assert that diagnostic is cleared.

### High: a later failed scan leaves stale diagnostics on previously responsible envelopes

`trigger_load_errors` accumulates errors on every failed scan and clears them only after a completely successful scan. If envelope A causes a failure, then A is repaired while envelope B becomes the first failing trigger, the next scan inserts B's error without removing A's. An open A therefore continues to report its obsolete load error even though the current transactional failure is attributable to B. The same stale state persists across every unsuccessful scan.

This violates the requirement that the diagnostic identify the responsible envelope and makes the editor report failures that no longer exist. It also complicates the closed-file publication fix above because stale envelope markers need explicit clearing as responsibility moves.

References:

- `darkmatter/dmls/src/overlay/mod.rs:307-323` removes root-scoped errors only in the `Ok(registry)` branch; the `Err(error)` branch only inserts or replaces the newly reported path.
- `darkmatter/dmls/src/overlay/mod.rs:225-236` reports any retained error keyed to the currently opened envelope, including stale entries.

Suggested fix: model scan status per registry scope/boundary rather than as an append-only global path map. Each scan attempt should atomically replace that scope's current failure with exactly the new responsible envelope, preserving the last-good registry separately. Return the old/new error-path transition so diagnostics for a formerly responsible envelope are cleared even when the replacement scan also fails. Add a Level-1 state or LSP regression covering A-invalid → repair A and invalidate B → B-invalid, asserting A clears and B receives the diagnostic without losing the last-good consumer schema.

## Review-3 Closure

The symlinked-root finding is fixed. `schema_roots` now uses `symlink_metadata` and accepts only a non-symlink directory, with a portable Level-1 regression test that handles Windows symlink privilege restrictions.

The DMLS diagnostic finding is only partially fixed. The implementation now retains payload-resolution scan errors and the new LSP test proves publication for an open envelope while preserving the consumer's last-good schema. It does not publish for closed envelopes and does not replace stale error ownership across consecutive failed scans.

## Test Rigor

This feature's observable behavior is filesystem discovery, schema parsing and matching, CLI behavior, and LSP state/diagnostic transitions. Level 1 is appropriate; none of these requirements depend on terminal rendering or a terminal emulator's input encoder, so Levels 2 and 3 are not required.

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Match grammar, combinators, arms, path matching, and vacuous lint | Level 1 unit tests | Appropriate |
| Ancestor discovery, ordering, shadowing, extensions, symlink exclusion, and case collisions | Level 1 filesystem tests | Appropriate |
| CLI compose/validate activation, assignment and shell re-resolution, raw mode, and trace output | Level 1 binary integration tests | Appropriate |
| CLI/DMLS dialect activation parity | Level 1 fixture-based integration tests | Appropriate |
| Transactional invalid-payload behavior and last-good registry retention | Level 1 unit and LSP tests | Appropriate |
| Failed DMLS scans diagnose an open envelope | Level 1 LSP integration test | Appropriate for the open-buffer case |
| Failed DMLS scans diagnose a closed envelope and clear it after repair | No test; behavior is missing | Gap |
| Consecutive failed scans transfer diagnostic ownership from repaired A to failing B | No test; behavior is broken | Gap |

## Verification

Targeted Level-1 nextest runs passed:

- `darkmatter`: `ancestor_walk_excludes_symlinked_schemas_root`.
- `dmls`: `trigger_payload_failure_retains_effective_schema_and_diagnoses_envelope`.

The package-area `just test darkmatter` run reached 5,233 passing tests before it was stopped at the non-interactive runtime limit, with 210 tests not run. One unrelated cache test failed its first attempt and passed its configured retry. Because the run was interrupted, it is not a clean full-suite result. Lint was not run in this iteration.

## Production Readiness

Not ready for production. The filesystem-boundary escape is repaired and open-envelope diagnostics work, but DMLS still fails to publish required trigger-load diagnostics for closed envelopes and can retain obsolete diagnostics across consecutive failed scans.
