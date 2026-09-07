---
status: implemented
created: 2026-08-30
area: repo
packages: []
---

# CI preflight parity: local scope self-test + a cross-OS smoke recipe

## Summary

PR #66 turned three CI cells red (preflight × macOS/Linux/Windows) on a
single Python assertion, and five more cells red on Windows-only failures
the macOS development host could not surface. Both gaps are closable with
tooling the repo already has: the preflight self-tests run in ~1s and can
run locally, and the `build-linux`/`build-win-native` SSH hosts can run real
Linux/Windows suites before a push — they were used exactly that way, by
hand, to fix PR #66.

This fix delivers three things, all implemented alongside this spec:

1. `just ci-local` now runs preflight's CI-infra self-test suite as its
   first gate.
2. The exact-dependents scope fixture fails with a paste-ready correction.
3. `just cross-check <pkg>` runs a package's L1 suite on standing Linux and
   native-Windows clones against the local tree.

## The defect

### The preflight fixture had no local signal

`scripts/ci/test_affected_scope.py::test_sniff_change_selects_exact_direct_dependents`
hardcodes the list of sniff's direct dependents. PR #66 added a
`claudine-gen → sniff` edge; the scope computation picked it up correctly,
but the fixture lagged and three preflight cells went red. The fixture is
*deliberate* friction — a human must acknowledge dependency-graph changes —
but the author's first signal was CI, and the failure message required
reverse-engineering which side of the `assertEqual` was the computed truth.

### Cross-OS evidence required ad-hoc plumbing

The Windows-only failure class (path spellings — see
`biscuit-file/fixes/2026-08-30-path-spelling/spec.md`) is invisible to macOS
L1. The macOS host CAN produce real evidence: during the PR #66 session,
`build-win-native` ran the failing suites via a hand-rolled
clone + `git apply` + `cargo nextest` loop over SSH. Nothing captured that
workflow, so the next path-sensitive change would rebuild it from scratch or
skip it.

## Delivered behavior

### D1 — `just ci-local` runs the CI-infra self-test

`just/ci-local.just` runs `python3 scripts/ci/test_affected_scope.py` as the
first gate of every non-dry-run invocation (including `--lint-only` /
`--test-only`), quiet on success, full unittest output on failure, and
reported in the summary as `preflight ci-infra self-test`. It is ~1s, so it
is unconditional rather than scope-gated.

### D2 — paste-ready fixture failure

The exact-dependents test now compares against the computed list and, on
mismatch, appends a message naming the file and test and printing the
computed list formatted exactly as the fixture's source lines, ready to
paste. The list stays hand-written on purpose (see "deliberate friction"
above); only the cost of acknowledging a change drops.

### D3 — `just cross-check <pkg> [--host linux|windows|all] [nextest args]`

`scripts/cross-check.sh`, wrapped by a root-justfile recipe:

- **Sync model:** the remote standing clone is reset to the nearest commit
  the remote can fetch (`origin/<current-branch>` if it exists, else
  `origin/main`), then the local tree's entire difference from that commit —
  tracked and untracked, committed-but-unpushed included — ships as one
  binary-safe patch and is `git apply`'d. No local commit or push is
  required.
- **Standing clones** (persist between runs so compile caches stay warm):
  `build-linux:~/ci-verification/rusty-biscuit` and
  `build-win-native:W:\ci-verification\rusty-biscuit`. Both bootstrap
  themselves on first use.
- **Unattended:** every ssh/scp uses `-o BatchMode=yes` (host rule:
  authentication must fail, never prompt). The Windows remote shell is
  PowerShell 5, so the script avoids `&&` and propagates
  `$LASTEXITCODE` explicitly.
- **Result:** per-OS pass/FAIL summary; non-zero exit when any selected
  host fails.

Documented (in the recipe) as the expected pre-push step for changes
touching path semantics, process spawning, or terminal behavior.

## Design decisions

- **Patch-over-fetch, not push-to-test.** Requiring a push to get cross-OS
  signal inverts the point (the signal should come *before* the push) and
  would spam the remote with WIP commits. The patch model was proven during
  the PR #66 session.
- **Standing clones, not per-run temp dirs.** A cold Windows build of a CLI
  package is tens of minutes; a warm one is seconds-to-minutes. The W:
  location follows the established `W:\ci-verification` convention, and the
  named, documented directory keeps the target dir visible to storage
  sweeps rather than orphaned (see the Windows storage-strategy notes).
- **Sequential hosts.** Output stays readable and the common case is
  `--host windows` anyway; parallelism can come later if the wait hurts.
- **Self-test unconditional in ci-local.** Scope-gating a 1-second suite
  buys nothing and adds a path where the guard silently doesn't run.

## Verification

- `python3 scripts/ci/test_affected_scope.py` — 65/65 pass; with
  `claudine-gen` temporarily removed from the fixture the run goes red and
  prints the paste-ready replacement list naming file and test (non-vacuity
  proven, then restored).
- `just ci-local --lint-only biscuit-hash` — summary shows
  `✅ preflight ci-infra self-test` ahead of the lint gates.
- `just cross-check biscuit-hash` — bootstraps both standing clones, applies
  the local diff, runs the suite on real Linux and native Windows, and
  prints the per-OS summary.

## Out of scope

- Wiring `cross-check` into the pre-push git hook. It costs remote compile
  time and needs the build hosts reachable; it stays a deliberate manual
  step until the hosts' availability story is settled.
- The path-spelling hardening itself —
  `biscuit-file/fixes/2026-08-30-path-spelling/spec.md`.
