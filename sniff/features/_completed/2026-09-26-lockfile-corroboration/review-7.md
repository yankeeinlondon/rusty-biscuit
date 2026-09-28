---
$schema: feature-review.yaml
ready: false
findings:
    - priority: high
      title: An explicit null npm workspace declaration is treated as missing evidence
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-27T11:33:22-07:00
spec: 2026-09-26-lockfile-corroboration/spec.md
implemented: true
next: 2026-09-26-lockfile-corroboration/review-8.md
implemented_by: claude/opus
log: sniff/features/2026-09-26-lockfile-corroboration/implementation-log.md
description: "A **feature** review of `2026-09-26-lockfile-corroboration/spec.md`"
feature: 2026-09-26-lockfile-corroboration/review-7.md
previous: 2026-09-26-lockfile-corroboration/review-6.md
---

# Review 7 — Lockfile corroboration

## Verdict

**Not ready for production.** Both unblocked findings from review 6 are implemented, and that review had no blocked findings. One new parser defect remains: an explicitly malformed npm workspace declaration receives the status reserved for incomplete evidence instead of the status for an invalid lockfile.

## Previous-review disposition

The Sniff library's [npm lockfile parser](../../lib/src/filesystem/repo/lockfile/npm.rs) now reports `unverifiable` with `no_membership_data` when the root package record omits `workspaces` and records no local packages. The [Level 1 public-result test](../../lib/tests/l1/lockfile_fixtures.rs) checks the full observation, empty differences, unchanged package provenance, and parse counter. This resolves review 6's false mismatch.

The Sniff CLI's [real-tool fixture tests](../../cli/tests/l1/lockfile_cli.rs) now find the library fixtures through `biscuit_test_harness::manifest_dir!()`, which uses the runtime manifest directory supplied to archived tests. The full-tree archive-path guard passed. This resolves review 6's build-host path finding. Review 6 had no blocked findings to reassess.

## Unblocked Findings

### High — An explicit null npm workspace declaration is treated as missing evidence

The Sniff library's [npm lockfile parser](../../lib/src/filesystem/repo/lockfile/npm.rs) stores the root record's `workspaces` field as `Option<Declarations>`. Serde maps both an omitted field and an explicit JSON `null` to `None`. The parser therefore treats a present field of the wrong type as missing membership data. The spec requires a supported-format lockfile with an invalid required membership field to report `unreadable` with `parse_failed`. Consumers need that distinction to tell a malformed lockfile from one that simply does not record workspace membership.

I copied the npm 11.6.4 workspace fixture to a temporary repository, changed only `packages[""].workspaces` to `null`, and ran the shipped `sniff repo structure --json`. It exited successfully, left provenance unchanged, and reported `unverifiable` with `ambiguous_membership`, empty differences, and `package-lock.json` as its path. The same malformed field in a root-only lockfile would report `no_membership_data`. Both results violate the required `unreadable`/`parse_failed` classification. Distinguish absence from explicit `null` in the typed root record, then add a Level 1 parser test and a public-result fixture test for this boundary.

## Verification level by requirement

| User-observable requirement | Strongest verification present | Assessment |
|---|---|---|
| Lockfile statuses, member differences, and provenance | Level 1 parser and complete-result fixture tests | The right level for repository data. The explicit-null npm case is missing and currently produces the wrong status and reason. |
| Standalone Poetry, PDM, and Composer observations | Level 1 library and shipped CLI tests | Appropriate for repository data. |
| JSON stdout, plain output, stderr separation, and success exit status | Level 1 shipped CLI tests | Appropriate for these noninteractive commands. |
| Styled lists, wrapping, and status colors | Level 2 capture of the shipped CLI in tmux | Appropriate for real-terminal rendering; the focused test passed. |
| Keyboard, mouse, paste, or input encoding | No such feature requirement | Level 3 is unnecessary. |

## Review checks

I read the spec, review 6, the lockfile source table and parsers, repository observation code, complete-result and CLI tests, and the declared test targets. `just check-tier-coverage sniff` found no stranded tests. `just test lockfile_` passed 82 Level 1 tests; `just test-l2 level2_lockfile` passed the tmux rendering test; and `just archive-path-guard` passed its full-tree scan. I also reproduced the npm null-field result through the shipped CLI on a disposable copy of the real-tool fixture. Cross-OS CI evidence and human approval are excluded from the readiness decision as requested.

## Human review

No human decision is needed. The spec already defines how invalid required membership fields must be classified.
