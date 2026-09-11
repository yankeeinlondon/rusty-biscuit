---
status: draft
created: 2026-09-11
area: repo
---

# Run the Level 2 Tier on the Native Windows CI Leg

## Problem

CI's `windows-latest` leg runs only the Level 1 tier. Every Level 2 cell for
that environment is a **POLICY GAP**, recorded in
`.github/ci/environments.json` (owner `@yankeeinlondon`, expiry 2027-01-31)
with this reason: tmux has no Windows port, and WezTerm, Kitty, and Apple
Terminal need a live desktop session no hosted runner provides; the "headless
`wezterm-mux-server` spike (plan 2.3)" that would test the alternative was
deferred.

Ken's standing ruling (2026-09-08) is that the gap is temporary and closes by
provisioning, never by narrowing an acceptance criterion. On 2026-09-10 the
spike was effectively performed on the native Windows build rig: a headless
`wezterm-mux-server`, addressed through `WEZTERM_UNIX_SOCKET`, hosted a real
`cmd.exe` pane, and the first native-Windows Level 2 test in the repository —
`level2_windows_provided_partial_file_capture.rs` — passed in 4.4 s, twice,
with the required-backend gate set. That run also found a production defect
on Windows the non-interactive tests could not see. Headless WezTerm works;
CI does not use it yet.

## Evidence

From the build rig work recorded in
`claudine/fixes/2026-09-10-no-interactive-completion/plan.md`:

- **A headless mux server hosts Level 2 panes.** `wezterm-mux-server.exe`
  started with `Start-Process -WindowStyle Hidden` and stdio redirected,
  reachable at `WEZTERM_UNIX_SOCKET=C:\Users\<user>\.local\share\wezterm\sock`,
  answered `wezterm cli list`, spawned `cmd.exe` panes, accepted `send-text`,
  and returned `get-text` with styling intact.
- **The client evaluates the developer's `wezterm.lua` before `spawn`.** On
  the rig that config `dofile`s an unreachable UNC path and every spawn
  blocked 21 s, exceeding the harness's 15 s `SPAWN_TIMEOUT`. Fixed in
  `biscuit-test-harness` (`wezterm_command()`): every harness client runs
  under an empty `WEZTERM_CONFIG_FILE` unless the caller set one. A hosted
  runner has no such config, but the isolation is what makes the leg
  independent of one.
- **Availability gating already exists.** `WezTermHarness::available()`
  requires `WEZTERM_UNIX_SOCKET`, `wezterm` on `PATH`, and a live
  `wezterm cli list`; `BISCUIT_TEST_REQUIRED_BACKENDS=wezterm` turns an
  unavailable backend into a failure instead of a ~0.02 s skip that nextest
  prints as PASS. That skip masqueraded as evidence twice on 2026-09-10.
- **The tier is small today.** Almost every Level 2 test in the repository is
  `#![cfg(unix)]`; on Windows the executable set is the Windows twins written
  explicitly for it (one in claudine as of this writing, plus a Level 3 test
  that this leg does not run). The leg's value is the platform it makes
  testable, and it grows as twins are written. Its cost is correspondingly
  small: a WezTerm install, a server start, and seconds of execution.
- **Never blanket-stop mux servers on a shared host.** A diagnostic that ran
  `Get-Process wezterm-mux-server | Stop-Process -Force` killed the session
  the developer was working in, twice. A hosted runner has no such session,
  but the CI step must still stop only the server it started.

## Required Behavior

1. **WezTerm is provisioned on the `windows-latest` runner** at a pinned
   version (`winget` or `choco`, whichever the runner image supports
   unattended), in a named step that fails the job if `wezterm --version`
   does not run. Cache the installer if the image does not carry it.
2. **A headless mux server is started by the job and owned by the job.**
   Start `wezterm-mux-server.exe` hidden with stdio redirected, export
   `WEZTERM_UNIX_SOCKET`, and poll `wezterm cli list --format json` until it
   answers (bounded; the rig needed under 5 s). Stop that process — by the
   pid the step recorded, never by name — in a step that runs even on
   failure. No GUI, no desktop session, no focus.
3. **The tier runs with the backend required.** `BISCUIT_TEST_REQUIRED_BACKENDS=wezterm`
   for the run, and the backend-execution evidence reaches the rollup as it
   does for tmux on Linux. A green cell must mean executed tests.
4. **Only the WezTerm backend is claimed.** The environment's capability
   entry gains `wezterm: true`; tmux, Kitty, and Apple Terminal stay
   unavailable there. The scope calculator includes the Level 2 tier for
   `windows-latest` exactly when a package declares `tiers` containing `L2`
   and `l2-backends` containing `wezterm`.
5. **Tests spawn `cmd.exe`, not a POSIX shell.** `TerminalHarness::spawn_shell`
   is Unix-shaped; Windows twins use `spawn_program("cmd.exe")` as the
   existing twin does. This specification does not change the harness's
   shell resolution; that is a separate decision if a `spawn_shell` for
   Windows is ever wanted.
6. **The Level 3 tier stays off.** `level3_windows_*` needs OS keyboard
   injection and a focused window; it remains attended-only.

## Non-goals

- WSL2 Level 2 (a separate specification: `wsl2-l2-ci-leg`).
- Converting the Unix Level 2 suites to run on Windows; twins are written per
  test where the platform matters.
- Provisioning a persistent mux server on the native Windows build rig for
  unattended `cross-check` runs (a small, separate host task: a user-level
  `WEZTERM_UNIX_SOCKET` and a logon-triggered scheduled task).
- Level 3 on any hosted runner.

## Acceptance Criteria

| Scenario | Required result |
| --- | --- |
| Full-scope run on a branch, `claudine-cli` selected | The `windows-latest` Level 2 job provisions WezTerm, starts and later stops its own mux server, executes `level2_wezterm_windows_review_router_partial_confirms_before_initialize`, and passes; the rollup shows an executed count for the cell. |
| WezTerm install step fails | The job fails at that named step; no tier runs; the rollup shows a failure, not POLICY GAP or PASS. |
| Mux server never answers `wezterm cli list` | The readiness poll fails the job within its bound; the tier does not run; nothing is left running on the runner. |
| Package declaring Level 2 with tmux only | No Windows Level 2 job is created; the cell renders POLICY GAP as today. |
| Duration | The job's Level 2 phase, install included, under 5 min; record the actual numbers in this specification's validation record. |
| Three consecutive green runs | Per the repository's evidence standard before the capability flip is merged. |

## Implementation Pointers

- `.github/workflows/_package-ci.yml`: the `l2` job's "Provision and verify
  the tmux backend" step gains a `windows-latest` arm that installs WezTerm
  instead of tmux; a new step starts the mux server and exports the socket;
  a post step stops it. `BISCUIT_TEST_REQUIRED_BACKENDS` is derived from the
  package's declared backends intersected with the environment's provisioned
  set, as it already is for tmux.
- `.github/ci/environments.json`: the `windows-latest` `wezterm` entry.
- `scripts/ci/affected_scope.py` and its tests: the tier/environment cross.
- `biscuit-test-harness/src/wezterm.rs`: no change expected; verify
  `wezterm_command()`'s empty-config isolation holds on the runner and that
  the pane cleanup (`cleanup_stale_wezterm_panes`) is inert with no stale
  panes.
- The `os` skill's `windows.md` "WezTerm on `build-win`" section, generalized
  once the hosted leg exists.

## Origin

Raised while closing `claudine/fixes/2026-09-10-no-interactive-completion`,
where the Windows interactive evidence had to be produced by hand on the
build rig. The rig facts above are recorded in that fix's `plan.md` and
`log.md` and in the `os` skill.
