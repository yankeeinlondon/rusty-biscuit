---
$schema: "@.claudine/schemas/review.yaml"
ready: false
agent: codex/default
created: 2026-07-11T09:27:25
implemented: true
---

# Review 3: Schema Triggers

## Findings

### High: a symlinked `schemas/` root is followed

The specification says discovery does not follow directory or file symlinks.
File entries inside a schema root are checked with `DirEntry::file_type`, but
`schema_roots` recognizes the root itself with `Path::is_dir`. That call follows
symlinks. Consequently, an ancestor entry such as `repo/schemas ->
/outside/schemas` is accepted as a schema root, and `read_dir` loads trigger
envelopes and payloads from outside the configured discovery boundary.

This breaks the project-contained discovery guarantee and makes the checked-out
project's effective schema depend on external filesystem state. The existing
`enumerate_excludes_subdirectories` test does not cover this case: it verifies
that enumeration is non-recursive, not that the discovered `schemas/` root is a
real directory.

References:

- `darkmatter/lib/src/markdown/schemas/triggers/discovery.rs`:
  `schema_roots` uses `schemas_dir.is_dir()` before adding a root.
- The same file's symlink test covers only a symlinked YAML file inside a real
  schema root.

Suggested fix: inspect each candidate root with `symlink_metadata` and accept it
only when the candidate itself is a non-symlink directory. Add a Level-1
filesystem regression test for a symlinked `schemas/` root. Keep the production
implementation and test portable: Windows symlink creation may need to skip
when the host lacks the required privilege, while the root check itself must
behave consistently on macOS, Windows, and Linux.

### High: DMLS retains last-good trigger state but drops the required load diagnostic

The specification requires transactional DMLS behavior to do both parts of the
contract: retain the last-good registry and publish a schema-load diagnostic on
the trigger envelope when a trigger load fails. The new eager payload loading
correctly makes invalid payloads fail the registry scan, and DMLS correctly
retains the prior registry. However, `OverlayCache::trigger_registry` only logs
the scan error and returns the cached registry. The error is not retained in
overlay state or exposed to diagnostics, so a missing payload, invalid payload,
or other rescan failure cannot produce the required diagnostic on the envelope.

`SuggestionState::TriggerError` does not close this gap. It is created only
while building an overlay for an open YAML buffer whose envelope parser itself
returns an error. Payload resolution errors arise during registry scanning, and
missing payloads have no payload buffer on which to diagnose. The new
`trigger_payload_edit_retains_last_good_registry` test verifies retained schema
behavior but does not assert that a diagnostic is published.

References:

- `darkmatter/dmls/src/overlay/mod.rs`: `trigger_registry` logs and discards the
  `scan_triggers` error.
- `darkmatter/dmls/src/overlay/mod.rs`: `SuggestionState::TriggerError` is
  populated only by `parse_trigger_envelope_from_str(text)` on the authoring
  buffer path.
- `darkmatter/dmls/src/diagnostics/frontmatter.rs`: trigger diagnostics can only
  consume `SuggestionState::TriggerError`; no registry-load error reaches it.

Suggested fix: retain the failed scan's error and responsible envelope path as
transactional registry state, and route it through the diagnostics publisher
for that file without replacing the last-good registry. Add a Level-1 LSP or
diagnostics integration test that edits or removes a payload, verifies the
consumer document keeps its prior effective schema, and verifies a schema-load
diagnostic is published on the envelope.

## Review-2 Closure

The review-2 payload transactionality finding is fixed. Discovery now resolves
every unshadowed trigger payload before returning a registry, including
unmatched triggers; effective assembly reuses the resolved payloads; and DMLS
retains the last-good activation when a valid payload becomes non-mergeable.

## Test Rigor

This feature's observable behavior is filesystem discovery, schema parsing and
matching, CLI behavior, and DMLS state/diagnostic transitions. Level 1 is the
appropriate tier; no requirement depends on terminal rendering, terminal input
encoding, keyboard events, mouse/paste/IME behavior, or scrolling, so Level 2
and Level 3 are not required.

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Match grammar, combinators, arms, path matching, and vacuous lint | Level 1 unit tests | Appropriate |
| Ancestor discovery, ordering, shadowing, extensions, file-symlink exclusion, and case collisions | Level 1 filesystem tests | Appropriate for the covered cases |
| Do not follow a symlinked `schemas/` directory | No test; behavior is broken | Gap |
| CLI compose/validate activation, assignment and shell re-resolution, raw mode, and trace output | Level 1 binary integration tests | Appropriate |
| CLI/DMLS dialect activation parity | Level 1 fixture-based integration tests | Appropriate |
| Invalid payloads fail transactionally during registry loading, including when unmatched | Level 1 unit tests | Appropriate |
| Invalid payload edits retain DMLS last-good effective behavior | Level 1 overlay test | Appropriate |
| Failed DMLS trigger rescans publish a schema-load diagnostic on the envelope | No integration test; behavior is missing | Gap |

## Verification

Targeted nextest runs completed successfully:

- `darkmatter`: 4 passed, covering unmatched missing/non-mergeable payloads,
  transactional scan failure, and prebuilt-registry assembly.
- `dmls`: 1 passed, covering valid-to-invalid payload editing with last-good
  activation retention.

These are Level-1 results. A full package-area test and lint run was not repeated
for this review iteration.

## Production Readiness

Not ready for production. The review-2 transactionality defect is repaired, but
the discovery boundary can still be escaped through a symlinked schema root and
DMLS does not publish the required diagnostic when transactional trigger loading
fails.
