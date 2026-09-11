---
status: draft
created: 2026-09-11
area: repo
---

# Run the Level 2 Tier on the WSL2 CI Leg

## Problem

CI's `wsl2-ubuntu` leg runs only the Level 1 tier. For every package that
declares Level 2 tests, the rollup renders the WSL2 Level 2 cell as a
**POLICY GAP** — deliberately, rather than a green cell that ran nothing. The
gap is recorded in `.github/ci/environments.json` (`wsl2-ubuntu` →
`capabilities.tmux`, owner `@yankeeinlondon`, expiry 2026-12-31) with this
reason: the guest runs a prebuilt nextest archive, has no toolchain, cannot
build the `biscuit-harness-broker` binary the tier's default mode uses, and
runs no terminal server.

Ken's standing ruling (2026-09-08) is that this gap is temporary and must be
closed by provisioning, never by narrowing an acceptance criterion. On
2026-09-10/11 the reason stopped applying in practice: the claudine
partial-file family — a real-terminal tmux capture, an `expectrl` PTY suite,
and their Level 1 neighbors — ran **17 of 17 green inside the WSL2 guest from
an archive**, both by hand (with the guest's `~/.config` share down) and
through `just cross-check --os wsl --features terminal-tests` with the share
up. The mechanism works; only the CI leg does not use it.

## Evidence

Measured on ci run `34510204615` (2026-09-10, full scope, 145 min wall clock)
and on the WSL2 build guest (12 cores, 15 GB):

| Fact | Value |
| --- | --- |
| Packages declaring a Level 2 tier | 11 (`claudine-cli`, `darkmatter-cli`, `sniff-cli`, `biscuit-icon-cli`, `claudine-gen`, `dmls`, `biscuit-terminal-cli`, `schematic-gen`, `worktree-cli`, `tree-hugger-cli`, `biscuit-tui-cli`) |
| Their Linux Level 2 jobs, summed, *including* builds | 3,188 s; `claudine-cli` alone 879 s |
| WSL2 leg today: guest test jobs | 66 jobs, 10,377 s summed, longest 682 s (`claudine-cli`) |
| Archive already built with declared features? | Yes — `_wsl-ci.yml`'s archive input is the package selector plus its `[package.metadata.ci.tests] features`, and `claudine-cli` declares `terminal-tests` |
| `claudine-cli` Level 2 tier, parallel self-spawn mode, fast macOS host | 248 tests; 157 executed in 32 s (run cut short by a host-specific failure) — roughly one minute for the tier |
| Same tier's tmux capture inside the WSL2 guest | 3.96 s (22.3 s while the guest's login shell waited on a dead network mount) |
| Other ten packages' whole Linux Level 2 jobs, builds included | 108–318 s each |

Estimate for adding the tier to the leg: **+10–20 runner-minutes per
full-scope run** (≈20–40 Linux-equivalent minutes at `windows-latest`
billing) on top of the 173 the leg already spends; wall clock close to
unchanged, since `claudine-cli`'s guest job would grow from ~11 to ~15 min
against a run whose longest job is 36 min. Per-gate scoping means a typical
PR selects a few of the eleven packages. The first real run replaces this
estimate.

## Required Behavior

1. **The guest runs the `level2_` filterset from the same archive it already
   runs Level 1 from.** No second archive, no toolchain in the guest. Feature
   flags are already baked in; `_archive_drop_build_flags` continues to drop
   them from the run.
2. **`tmux` is provisioned in the guest** during the leg's existing guest
   setup, and verified the way `_package-ci.yml`'s "Provision and verify the
   tmux backend" step does (`tmux -V`; no server, no display, no focus).
3. **The tier runs in parallel self-spawn mode** (`BISCUIT_L2_THREADS=N`),
   where every `shared_or_spawn()` takes its owned-pane fallback and the
   `biscuit-harness-broker` binary is never needed. This is the design choice
   that makes the capability entry's stated blocker moot; the serial broker
   mode is not an option in the guest and is 5–10× slower besides. Choose `N`
   from the guest's cores (the standing guest has 12; hosted guests will
   differ — read `nproc`).
4. **The leg cannot be green with zero Level 2 tests executed.** Set
   `BISCUIT_TEST_REQUIRED_BACKENDS=tmux` for the run so a missing or broken
   backend fails the tier, and carry the backend-execution evidence into the
   rollup the same way the Linux leg's `_test_l2` bracket does. Whether the
   `backend-proof` verifier can run in the guest (it is not a test binary, so
   the archive does not carry it) is the one open implementation question;
   if it cannot, the JUnit manifest must carry an equivalent executed-count
   assertion. A leg that passes by skipping is the non-evidence the 2026-07-24
   devops specification (plan 1.1) rejects, and it fooled this repository
   twice on 2026-09-10.
5. **The capability table and the scope calculator agree.** Flip
   `wsl2-ubuntu` → `capabilities.tmux` to available, and have
   `scripts/ci/affected_scope.py` include the Level 2 tier for `wsl2-ubuntu`
   exactly when a package declares `tiers` containing `L2` and `l2-backends`
   containing `tmux`. Packages whose Level 2 suites drive only GUI backends
   keep rendering POLICY GAP there, as they do on `ubuntu-latest`.
6. **JUnit reports for the tier are staged and uploaded** as
   `junit-<package>-L2-wsl2-ubuntu`, crossing the 9p mount once at the end,
   mirroring the leg's Level 1 staging.

## Non-goals

- Native Windows Level 2 (a separate specification: `windows-l2-ci-leg`).
- Shipping the harness broker into the guest, or a serial shared-pane mode
  there.
- WezTerm, Kitty, or Apple Terminal backends in the guest; those stay POLICY
  GAP on every hosted runner.
- Changing any test, harness, or package-level Level 2 declaration.

## Acceptance Criteria

| Scenario | Required result |
| --- | --- |
| Full-scope run on a branch, `claudine-cli` selected | The `wsl2-ubuntu` Level 2 job executes the package's `level2_` tests from the archive; the rollup shows an executed count, not POLICY GAP; the tmux capture tests in `level2_provided_partial_file_capture.rs` and the PTY tests in `level2_provided_partial_file_pty.rs` are among them and pass. |
| Same run, a package whose Level 2 suite is WezTerm-only | Cell remains POLICY GAP; no job is created for it. |
| `tmux` deliberately absent from the guest | The tier **fails** (required backend), and the failure names the backend. |
| Guest `~/.config` network share unreachable | The leg is unaffected (it never reads it). |
| Duration | The `claudine-cli` guest job's Level 2 phase is under 5 min on the hosted guest; full-scope wall clock within noise of the baseline run. Record the actual numbers in this specification's validation record and replace the estimate above. |
| Three consecutive green runs | Per the repository's evidence standard before the capability flip is merged. |

## Implementation Pointers

- `.github/workflows/_wsl-ci.yml`: the `wsl` job's guest setup (tmux install
  beside the existing native-package step), a new Level 2 run step after the
  Level 1 step (`just _test_l2` equivalent for archive mode, or a direct
  `cargo nextest run --archive-file … -E 'test(/^level2_/)'` with the
  self-spawn variables), and a second JUnit staging/upload pair.
- `.github/ci/environments.json`: the `wsl2-ubuntu` tmux entry.
- `scripts/ci/affected_scope.py` and its tests: the tier/environment cross.
- `scripts/ci-rollup` (or wherever the rollup reads the manifest): the new
  cell must be expected, not merely tolerated.
- The `os` skill's `wsl.md` "Level 2 on WSL" section, once the leg exists.

## Origin

Raised while closing `claudine/fixes/2026-09-10-no-interactive-completion`,
whose Windows and WSL2 interactive evidence had to be produced by hand on the
build rigs because CI could not. The measurements above are in that fix's
`plan.md` and `log.md`.
