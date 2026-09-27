---
$schema: feature-review.yaml
ready: true
human_review: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-26T17:45:31-07:00
spec: 2026-09-25-recent-commits/spec.md
implemented: false
description: "A **fix** review of `2026-09-25-recent-commits/spec.md`"
fix: 2026-09-25-recent-commits/review-2.md
previous: 2026-09-25-recent-commits/review-1.md
---

# Review 2 — Recent commits package-area filtering

## Verdict

**Production ready.** The package-area filter now uses each changed file's location, including when a package directory is also a nested area. Package filtering still uses package ownership. The library and shipped CLI agree on the matching commits and JSON attribution.

## Previous review findings

### Unblocked Findings

The first review's high-priority finding, **Files in overlapping package and area directories miss the nested area**, is resolved. In the Sniff library, [`PackageCatalog::resolve`](../../lib/src/filesystem/git/recent_commits/collect.rs) now looks up the deepest area containing a changed file independently of its owning package. Its overlap regression expects `crates/alpha/README.md` to belong to the `crates/alpha` area while remaining owned by the `alpha` package. The Sniff CLI regression exercises the same arrangement through the shipped command, checks commit identities and order, and checks that JSON attribution agrees with filtering. The [recent-commits topic](../../docs/topics/repo/recent-commits.md) and [schema](../../docs/topics/repo/recent-commits-schema.md) describe the rule.

### Blocked Findings

The first review listed no blocked findings, so there was nothing to unblock before this implementation.

## New Findings

None. The implementation meets the spec's location, package ownership, count, ordering, whole-commit, and attribution requirements. The source change is confined to the Sniff library; the CLI reports its results.

## Requirement and verification level

| User-observable requirement | Strongest verification | Assessment |
|---|---|---|
| Shared planning files, reviews, features, and area-root README files match their area without gaining a package | Level 1 disposable Git histories in the Sniff library and spawned CLI | Appropriate; both check exact commit hashes and JSON attribution. |
| A file owned by a package inside a nested area matches that area, while package filters retain the owner | Level 1 library and spawned CLI overlap regressions | Appropriate; both test a package directory that is also an area, including a similarly named sibling outside the nested area. |
| A requested count is filled with matching commits in newest-first order, and a retained commit keeps all its changed files | Level 1 library and spawned CLI histories with intervening nonmatches and mixed-area commits | Appropriate; tests compare ordered hashes and complete records. |
| Misleading commit scopes and paths outside the area do not match | Level 1 library and spawned CLI histories | Appropriate; location is checked independently of commit text. |
| Aggregate JSON and focused JSON attribute shared files consistently | Level 1 spawned CLI regression | Appropriate; it compares the complete `recent_commits` arrays and checks a documentation projection. |
| Area matching works on macOS, Linux, native Windows, and WSL2 | Shared cross-platform path code and Level 1 fixtures | Appropriate functional level. CI provides the separate operating-system evidence and does not affect this verdict. |

These behaviors depend on Git paths and command data, not terminal rendering or keyboard input. Level 2 and Level 3 tests are therefore unnecessary for this fix.

## Review verification

- The Sniff library's `recent_commits::` Level 1 suite passed: 44 tests.
- The two relevant Sniff CLI filtering regressions and its aggregate-attribution regression passed: 3 tests.
- The actual commit `1634e6e` appeared once in `sniff repo recent-commits 300 --package-area worktree --json` against this repository, with `packages: []` and `package_areas: ["worktree"]`.
- Both test files are declared by their packages' `l1` targets, and their names have no segment that removes them from Level 1. The Sniff library's Level 1 feature selection enables `remote`.
- `git diff --check` found no whitespace errors.
