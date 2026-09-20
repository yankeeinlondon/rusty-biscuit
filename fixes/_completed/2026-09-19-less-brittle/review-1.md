---
$schema: feature-review.yaml
ready: false
findings:
  - title: The twelve known archive-path violations remain unfixed
    priority: high
  - title: The guard still performs unsafe text matching with file-wide exemptions
    priority: high
  - title: The planner does not select or provide the guard's scan scope
    priority: high
  - title: Guard execution is not integrated into CI ownership, reporting, or local validation
    priority: high
  - title: Required regression coverage and verification boundaries are absent
    priority: high
human_review: false
reviewed_by: codex/gpt-5.6-luna
created: "2026-09-19T15:57:26-07:00"
spec: 2026-09-19-less-brittle/spec.md
log: fixes/2026-09-19-less-brittle/implementation-log.md
implemented: true
implemented_by: claude/opus
description: "A **fix** review of `2026-09-19-less-brittle/spec.md`"
fix: 2026-09-19-less-brittle/review-1.md
next: 2026-09-19-less-brittle/review-2.md
---

# Review 1

**Not production-ready.** The checkout contains the specification and the
pre-existing archive-path guard, but no implementation for this fix. The
working tree has no implementation changes; the guard still fails on all
twelve known sites. The requested change therefore cannot be considered
implemented or ready for release.

## Findings

### High — The twelve known archive-path violations remain unfixed

`cargo nextest run -p test-toolkit --test archive_path_guard` fails with all
twelve paths named by the specification: the five Claudine/DMLS sites and the
seven Messenger sites still contain `env!("CARGO_MANIFEST_DIR")`. The shared
`biscuit-test-harness` macros have not been adopted at those call sites, and
`messenger/lib/Cargo.toml` still has no direct `biscuit-test-harness`
dev-dependency. This leaves the original relocation failure in place and also
means the empty-runtime-value behavior is still inconsistent with the shared
helper.

Migrate every listed site, preserve the relevant path ownership, remove stale
imports/comments, add the direct dependency, and add the requested relocation
coverage. The guard must pass with no new allowlist entries.

### High — The guard still performs unsafe text matching with file-wide exemptions

`tools/test-toolkit/tests/archive_path_guard.rs` searches source lines for
literal strings after blanking only `//` comments. It does not distinguish
Rust tokens from strings, block comments, multiline expressions, or a real
macro invocation. Its `ALLOWED` set exempts an entire file rather than the
specific safe fallback expression. It also validates allowlist entries using
the filtered scan, so safe-fallback recognition cannot be independent from
stale-exemption detection as required by the specification. The scanner
silently ignores unreadable directories/files, and self-exclusion is based on
the basename suffix rather than the actual guard implementation path.

Replace this with the specified small token-aware matcher and separate raw
live-form scan. Add fixtures for runtime fallback, empty values, unrelated
runtime reads, binary-name and precedence mismatches, multiline forms,
comments, strings, a second unsafe occurrence, and the independent hosted-root
check. Read failures, duplicate entries, and stale/renamed entries must fail
with actionable diagnostics.

### High — The planner does not select or provide the guard's scan scope

`SUITE_REGISTRY` contains only the existing `test-toolkit-l1` and companion
suites; there is no guard-only selection. `change_inventory` records general
change buckets, but no guard-owned trigger policy, removed/renamed-file
handling, selected normalized path set, scan mode, or malformed-plan contract
exists. The current guard always discovers the whole repository itself, so it
cannot implement changed-file pull-request scans, full-tree push/manual scans,
or the explicit-empty versus unavailable-diff distinction from the spec.

Extend the canonical resolved plan and all validators/readers/projections as a
single contract. Use the same eligible-file policy for planner selection and
scanning, select exactly one Linux guard execution for the specified inputs,
and cover package-area/out-of-member Rust files, deletions, renames, skipped
directories, guard-owned non-Rust inputs, documentation-only changes, events,
absent diffs, explicit empty inventories, and proven-environment reuse.

### High — Guard execution is not integrated into CI ownership, reporting, or local validation

No workflow, recipe, registry, coverage projection, receipt identity, or
local-plan path invokes a separately selected archive guard. Adding the guard
to the existing test suite would not satisfy the requirement because ordinary
`test-toolkit` L1 selection remains package-driven and would run unrelated
suites. Conversely, a source scan performed inside the existing test is not
evidence for a separate planner cell. There is also no implementation of the
required guard-only behavior that avoids Clippy/L1/companion work, blocks the
merge gate on failure, reports missing execution through the owning area audit,
and avoids presenting a partial scan as full-tree evidence.

Implement the selected suite-only integration (or document and implement a
deliberately equivalent package/gate design), including package/environment/
gate-keyed results, scan inputs in evidence identity, canonical recipe
execution, failure/cancellation propagation, coverage-audit enforcement, and
the all-supported-OS standalone full-tree default. Update the CI README,
recipe documentation, and CI skill with the actual behavior.

### High — Required regression coverage and verification boundaries are absent

The only relevant guard tests are the two existing L1 tests; one passes only
because the allowlist entries are live, and the other fails on the known
violations. There are no planner fixtures, matcher fixtures, scan fixtures,
exemption-mode fixtures, workflow/reporting contracts, relocation tests for
the migrated sites, or focused Messenger research-feature runs required by
the acceptance criteria.

For this specification, terminal L2/L3 testing is not applicable: it adds no
terminal rendering or keyboard behavior. The required minimum evidence is L1
for the matcher, filesystem scanner, planner, recipes, and migrated tests,
plus the real CI/workflow boundary for selection, missing execution, result
identity, and merge blocking. Source-string assertions or an ordinary
`test-toolkit` run cannot prove those workflow requirements. The implementation
must provide the acceptance tests and run the canonical full guard recipe,
`just test test-toolkit`, the focused affected package recipes with Messenger's
`research` feature, compact CI contract tests, and `actionlint` when workflows
change.

## Verification performed

- `cargo nextest run -p test-toolkit --test archive_path_guard`: **1 passed,
  1 failed**; the failing test reports all twelve known violations.
- Direct inspection confirms no implementation diff is present in the
  checkout; only the new specification directory is untracked.
- GitNexus impact queries could not resolve the test file and returned
  `risk: UNKNOWN`; this was not treated as evidence of safety. The index also
  reports the checkout as stale, and `detect-changes --scope all` reports no
  tracked changes because the implementation is absent.

No cross-OS claim is needed for these findings, and no terminal/browser window
was launched.
