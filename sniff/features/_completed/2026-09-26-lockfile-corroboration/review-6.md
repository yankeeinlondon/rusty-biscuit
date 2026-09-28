---
$schema: feature-review.yaml
ready: false
findings:
    - priority: high
      title: An npm lockfile without workspace declarations reports a false mismatch
    - priority: high
      title: Shipped CLI fixture tests use a build-host path in archived runs
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-27T11:11:42-07:00
spec: 2026-09-26-lockfile-corroboration/spec.md
implemented: true
next: 2026-09-26-lockfile-corroboration/review-7.md
implemented_by: claude/opus
log: sniff/features/2026-09-26-lockfile-corroboration/implementation-log.md
description: "A **feature** review of `2026-09-26-lockfile-corroboration/spec.md`"
feature: 2026-09-26-lockfile-corroboration/review-6.md
previous: 2026-09-26-lockfile-corroboration/review-5.md
---

# Review 6 — Lockfile corroboration

## Verdict

**Not ready for production.** Review 5's unblocked finding is fixed, and it had no blocked findings. Two newly identified gaps remain: npm can report a definite mismatch from a lockfile that does not establish its workspace members, and the new CLI fixture tests rely on a path from the build host when run from a test archive.

## Previous-review disposition

The Sniff library's [workspace pattern readers](../../lib/src/filesystem/repo/npm.rs), [uv reader](../../lib/src/filesystem/repo/uv.rs), and [Cargo reader](../../lib/src/filesystem/repo/cargo.rs) now distinguish a present member field of the wrong type from an absent declaration. The detectors keep the layer and mark discovery incomplete. The Level 1 [public-result fixture test](../../lib/tests/l1/lockfile_fixtures.rs) checks the lockfile observation and package provenance for wrong-type pnpm, uv, npm, Cargo, and Rush fields. It passes. Review 5 listed no blocked findings, so none needed reclassification.

## Unblocked Findings

### High — An npm lockfile without workspace declarations reports a false mismatch

The Sniff library's [npm lockfile parser](../../lib/src/filesystem/repo/lockfile/npm.rs) treats a missing root `workspaces` field as a complete empty member set when the `packages` object has no non-root records. It then compares that invented empty set with the current manifest and reports `mismatch`, listing every current member as missing. The spec says a missing membership field is not an empty set: the lockfile has not established which workspaces it recorded.

I copied the checked-in npm 11.6.4 workspace fixture to a temporary directory and changed its valid v3 `package-lock.json` to contain only its root package record, without `workspaces`. The shipped `sniff repo structure --json` exited successfully and reported `mismatch` with `.tools/hidden`, `packages/alpha`, and `packages/beta` in `missing`. Its conclusion is unsupported by the lockfile. Preserve the uncertainty as an appropriate `unverifiable` or `unreadable` observation according to the supported-format rule, and add a Level 1 public-result test asserting the status, reason, differences, and unchanged provenance. The parser's current unit corpus has no case for this missing declaration.

### High — Shipped CLI fixture tests use a build-host path in archived runs

The new Sniff CLI [lockfile integration tests](../../cli/tests/l1/lockfile_cli.rs) locate the library's real-tool fixtures with `env!("CARGO_MANIFEST_DIR")`. That value is fixed when the binary is compiled. CI runs Nextest archives on a different host or path, so the test can look for fixtures in the producer's checkout and fail before exercising the shipped CLI. The adjacent Sniff library fixture tests use `biscuit_test_harness::manifest_dir!()`, which honors the remapped runtime directory. Use that same runtime-aware root in the CLI test and keep the path to `lib/tests/fixtures/lockfiles` explicit so the repository-file test index can select it when a fixture changes. Verify with an archived run or the repository's archive-path guard.

## Verification level by requirement

| User-observable requirement | Strongest verification present | Assessment |
|---|---|---|
| Lockfile statuses, member differences, and provenance | Level 1 complete-result and real-tool fixture tests | Appropriate level for repository data. The npm missing-declaration case is absent and gives an incorrect result. |
| Standalone Poetry, PDM, and Composer observations | Level 1 library and shipped CLI tests | Appropriate for repository data. |
| JSON stdout, plain output, stderr separation, and exit status | Level 1 shipped CLI tests | Appropriate level; the new fixture-based tests have an archived-run path defect. |
| Styled lists, wrapping, and status colors | Level 2 capture of the shipped CLI in tmux | Appropriate for real-terminal rendering; the focused test passed. |
| Keyboard, mouse, paste, or input encoding | No such feature requirement | Level 3 is unnecessary. |

## Review checks

I read the spec, review 5, workspace readers, lockfile parsers, public-result fixtures, CLI tests, and declared test targets. The library and CLI test modules are declared, and `just check-tier-coverage sniff` reported no stranded tests. `just test lockfile_` passed all 81 selected Level 1 tests; `just test-l2 level2_lockfile` passed the real-terminal rendering test. I reproduced the npm result through the shipped CLI on a disposable fixture. Cross-OS CI evidence and human approval do not affect this readiness decision, as requested.

## Human review

No human design decision is needed for these fixes. The spec already says that missing membership data cannot establish a complete member set, and the test runner's runtime fixture path is established repository practice.
