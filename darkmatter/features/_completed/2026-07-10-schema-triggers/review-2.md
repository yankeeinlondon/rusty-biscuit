---
$schema: "@.claudine/schemas/review.yaml"
ready: false
agent: codex/default
created: 2026-07-11T08:19:03
implemented: true
---

# Review 2: Schema Triggers

## Findings

### High: trigger payload failures are neither load-time nor transactional

The specification requires discovery to treat an opted-in trigger with an
unresolvable, cyclic, or non-mergeable payload as a hard load error. Registry
installation must be transactional, and DMLS must retain its last-good
registry after a malformed payload edit.

The implementation validates only the envelope during `triggers::scan`.
Payload resolution and its mergeability/cycle checks occur later in
`DarkmatterSchemas::effective_for`, after matching. This has three observable
consequences:

- An invalid payload is not reported at all when its trigger does not match the
  current document, even though the trigger registry is invalid by contract.
- CLI and library callers can install a registry containing an invalid trigger
  and discover the error only for documents that activate it.
- In DMLS, changing a previously valid payload to invalid does not make the
  registry rescan fail. A matching document receives `SchemaOutcome::Failed`
  instead of continuing with the last-good registry/effective schema, so the
  document flaps to a schema-preparation error.

References:

- `darkmatter/lib/src/markdown/schemas/triggers/discovery.rs`: `scan` parses
  envelopes but never resolves or validates their payloads.
- `darkmatter/lib/src/markdown/schemas/triggers/assemble.rs`: payload
  resolution, mergeability checks, and cycle checks run only after a trigger
  has matched.
- `darkmatter/dmls/src/overlay/mod.rs`: `trigger_registry` retains last-good
  state only when `scan_triggers` returns an error; payload failures instead
  arise later from `schema::assemble` and are cached as `SchemaOutcome::Failed`.
- The DMLS transactional regression test edits only the envelope to
  `match: {}`; it does not edit the payload to an invalid schema.

Suggested fix: make registry construction validate every unshadowed trigger's
payload before installing the registry, independent of whether any current
document matches. Store enough resolved payload metadata in the registry to
avoid resolving it twice, or add a separate transactional registry-loading
stage above filesystem enumeration. DMLS should retain the last-good loaded
registry and publish the load diagnostic against the responsible envelope or
payload file. Add Level 1 tests for an unmatched trigger with a missing payload,
and for a valid-to-invalid payload edit retaining the prior DMLS behavior.

## Test Rigor

This feature concerns filesystem discovery, parsing, matching, schema
resolution, CLI exit/output behavior, and DMLS state transitions. Level 1 is
the appropriate verification tier for all specified user-observable behavior;
the feature does not depend on terminal-emulator rendering, terminal input
encoding, keyboard events, mouse/paste/IME handling, or scrolling, so Level 2
and Level 3 are not required.

Requirement-to-level assessment:

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Match grammar, combinators, arms, path matching, and vacuous lint | Level 1 unit tests | Appropriate |
| Ancestor discovery, ordering, shadowing, extensions, symlink exclusion, and case collisions | Level 1 filesystem tests | Appropriate |
| CLI compose/validate activation, assignment re-resolution, shell re-resolution, raw mode, and trace output | Level 1 binary integration tests | Appropriate |
| CLI/DMLS dialect activation parity | Level 1 fixture-based integration tests | Appropriate |
| Envelope-edit transactionality and frontmatter hysteresis in DMLS | Level 1 overlay tests | Appropriate |
| Invalid payloads fail during registry loading, including when unmatched | No test; behavior is missing | Gap |
| Invalid payload edits retain DMLS last-good registry/effective behavior | No test; behavior is broken | Gap |

## Verification

`just test` completed successfully from the Darkmatter package area:

- `darkmatter`: 5,439 passed, 111 skipped
- `darkmatter-cli`: 552 passed, 71 skipped
- `dmls`: 405 passed, 0 skipped

These are Level 1 results. The passing suite does not cover the payload-loading
transaction identified above.

## Production Readiness

Not ready for production. The envelope and activation paths are broadly
implemented and well exercised at the correct test level, but malformed
trigger payloads bypass the specified transactional load boundary and can
silently escape detection or make DMLS abandon last-good behavior.
