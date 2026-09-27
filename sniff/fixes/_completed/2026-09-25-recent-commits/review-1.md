---
$schema: feature-review.yaml
ready: false
findings:
    - priority: high
      title: Files in overlapping package and area directories miss the nested area
human_review: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-26T17:33:42-07:00
spec: 2026-09-25-recent-commits/spec.md
implemented: true
next: 2026-09-25-recent-commits/review-2.md
implemented_by: claude/default
log: sniff/fixes/2026-09-25-recent-commits/implementation-log.md
description: "A **fix** review of `2026-09-25-recent-commits/spec.md`"
fix: 2026-09-25-recent-commits/review-1.md
---

# Review 1 — Recent commits package-area filtering

## Verdict

**Not production ready.** The fix handles the reported `worktree/fixes/` case, fills the requested count with matching commits, and attributes shared files without inventing a package. One path-based area rule remains broken for directories that are both a package and a nested area.

## Findings

### High — Files in overlapping package and area directories miss the nested area

The specification says area membership follows file location and that an area selector includes any file under its directory. In the Sniff library, [`PackageCatalog::resolve`](../../../lib/src/filesystem/git/recent_commits/collect.rs) instead takes an owning package's declared area before checking the file's location. For example, `darkmatter/dmls` is both the `dmls` package directory and a package area for a nested package. A commit changing only `darkmatter/dmls/README.md` is assigned to `darkmatter` and is excluded by `--package-area darkmatter/dmls`, although that file is inside the selected directory. The same shape exists at `tabby/` and in the library fixture at `crates/alpha/`.

This affects both filtering and JSON attribution. The Sniff library's [overlap test](../../../lib/tests/l1/recent_commits.rs) explicitly expects `crates/alpha/README.md` to be absent from `--package-area crates/alpha`, so the test currently locks in the behavior that conflicts with the spec. Resolve the file's area from the deepest containing area directory independently of package ownership, while continuing to use the deepest owning package for the `--package` filter. Update the overlap test and the [recent-commits documentation](../../../docs/topics/repo/recent-commits.md) to state the resulting rule. A CLI regression for the same overlapping directory would verify the shipped command.

## Requirement and verification level

| User-observable requirement | Strongest present verification | Assessment |
|---|---|---|
| Shared planning files, reviews, and area-root README files match their area but no package | Level 1 disposable Git repositories in the Sniff library and spawned CLI | Appropriate level; focused tests pass. |
| Library and CLI return matching commit identities in order and fill the requested count with area matches | Level 1 exact hash sequences from disposable histories | Appropriate level; focused tests pass. |
| Outside files, similarly named siblings, and misleading commit scopes do not match | Level 1 library and CLI fixtures | Appropriate level; focused tests pass. |
| JSON attributes shared files to an area without inventing a package; filtered records keep all changed files | Level 1 library JSON assertions and spawned CLI JSON assertions | Appropriate level; focused tests pass. |
| Every file under a selected area matches by location, including a file in a directory that is both a package and an area | Level 1 overlap test expects exclusion; no CLI regression for this case | High finding above. |
| The same file boundaries work on macOS, Linux, native Windows, and WSL2 | Level 1 cross-platform code path and fixture tests; implementation log records local cross-check runs | Functional Level 1 is appropriate. CI remains the separate operating-system proof and does not determine this verdict. |

These requirements involve repository paths, Git history, and command output. They do not depend on terminal rendering or keyboard input, so Level 2 and Level 3 tests are not required.

## Review verification

- Ran the Sniff library's `recent_commits::` Level 1 tests with the `remote` feature: 44 passed.
- Ran the two new Sniff CLI Level 1 tests for shared-area filtering and aggregate attribution: both passed.
- Checked that both test files are declared by their packages' `l1` targets and selected by the Level 1 tier. The relevant tests have no tier-excluding name segment.
- Reviewed the library resolver, existing overlap test, CLI fixture, documentation, and the implementation plan. The plan calls the package-first overlap rule an ambiguity; the specification's location rule supplies the review criterion.
