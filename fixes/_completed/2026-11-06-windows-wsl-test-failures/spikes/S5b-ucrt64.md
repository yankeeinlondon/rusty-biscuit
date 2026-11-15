---
spike: S5b
spec: 2026-11-06-windows-wsl-test-failures
question: Is `ucrt64` now the default Git for Windows layout, and does `wt` behave correctly but slowly without recognizing it?
answer: yes, ucrt64 is the only x86_64 layout since Git for Windows v2.56.0 (2026-09-28); wt is correct but slower when the layout is not recognized (about +70% on a 4-worktree `wt list`); the three CI failures come from PATH naming the binary directly, not from the missing ucrt64 entry
confidence: high (layout, correctness, cause of the three failures); medium (timing magnitude, simulated layout)
date: 2026-11-07
---

## Answer

- **ucrt64 is now the default.** Git for Windows v2.56.0 (2026-09-28)
  removed `mingw64\bin\git.exe` on x86_64, and `ucrt64\bin\git.exe` took its
  role.
  - Every new install or upgrade from v2.56.0 on therefore has a layout that
    `PREFIXES` (`worktree/lib/src/git/program.rs:103`, `mingw64`,
    `clangarm64`, `mingw32`) does not recognize.
  - The hosted runner already has `2.56.0.windows.4` with `ucrt64`.
  - The rig has `2.55.0.windows.3` with `mingw64`, so it is still recognized
    there.
- **`wt` stays correct.** `wt list` printed byte-identical output in all
  three arrangements, cold and warm. When `launcher_layout` returns `None`,
  `command()` falls back to `Command::new("git")` and `EXEPATH` stays unset.
  That costs time only:
  - two process creations per Git call through the launcher;
  - one extra `git --exec-path` per `wt` process, which gix runs.
- **The cost**, as the median of 40 interleaved, warm `wt list` runs over a
  4-worktree repository with no remote:

  | Arrangement on the rig (mingw64 install) | Git processes per listing | Median, run 2 | Median, run 3 |
  |---|---|---|---|
  | A. `cmd\git.exe` launcher first on `PATH`, recognized (the normal case on ≤2.55) | 4 `status` | 123.2 ms | 82.0 ms |
  | B. `<prefix>\bin\git.exe` first on `PATH` (the hosted runner's arrangement) | 1 `--exec-path` + 4 `status` | 165.0 ms | 108.9 ms |
  | C. launcher first, layout not recognized (stands in for a ≥2.56 `ucrt64` install) | 1 `--exec-path` + 4 `status`, each through the launcher | 208.8 ms | 139.5 ms |

  The two runs differ in absolute time because the rig's load varied, but
  the ratios hold: B costs about +33% over A, and C about +70%, roughly
  +57 ms per listing at run 3's load. The cost grows with the number of Git
  calls, so a repository with more worktrees pays more. Any user who
  upgrades to Git for Windows ≥2.56 and keeps the default `cmd` on `PATH`
  is in case C. The module docs' figure for the launcher overhead (2.7 s
  versus 1.6 s per 100 calls) is consistent with this.
- **The three failures in the spec's ucrt64 row are caused by PATH order,
  not by ucrt64 itself.** On the runner, `which git` found
  `C:\Program Files\Git\ucrt64\bin\git.exe`, the binary itself, ahead of
  any launcher. All three failures reproduce on the rig's recognized
  `mingw64` install by putting `mingw64\bin` first on `PATH`, and pass with
  `cmd` first. In that arrangement:
  - `launcher_layout` sees `<prefix>\bin\git.exe`, takes `<prefix>` as the
    top, finds no `<prefix>\<prefix>\bin`, and returns `None`.
  - So **adding `("ucrt64", "UCRT64")` to `PREFIXES` would not turn these
    three tests green on the hosted runner.**

## Evidence

### Release notes

From `git-for-windows/build-extra` `ReleaseNotes.md` (fetched 2026-11-07):

- Under "Changes since Git for Windows v2.55.0(5) (August 20th 2026)", which
  is the v2.56.0 release, quoted verbatim:
  > Following the MSYS2 project, on which Git for Windows is based, Windows
  > 8.1 support was dropped; In doing so, internal paths changed
  > (`/mingw64/bin/git.exe` does not exist anymore, `/ucrt64/bin/git.exe`
  > takes its role; if this breaks your setups, consider switching to
  > `/cmd/git.exe` instead, which is guaranteed to stay stable).
- Under "Changes since Git for Windows v2.56.0 (September 28th 2026)": "after
  Git for Windows v2.56.0's MINGW64 -> UCRT64 migration", and
  `GIT_CONFIG_GLOBAL=NUL` "was broken in v2.56.0 (surprisingly, as a
  consequence of the MINGW64 -> UCRT64 migration)".
- The latest heading is "Changes since Git for Windows v2.56.0(4) (October
  26th 2026)".
- ARM64 keeps `clangarm64`. Nothing found says whether 32-bit (`mingw32`)
  builds still ship.

### Layouts

- **Rig** (`build-win-native`): `git version 2.55.0.windows.3`. `C:\Program
  Files\Git` contains `bin cmd dev etc mingw64 tmp usr`; there is no
  `ucrt64`. `Get-Command git -All` gives `cmd\git.exe`, then
  `mingw64\bin\git.exe`.
- **Hosted runner** (nightly run 42246114963, `worktree` job 128082100327):
  "Getting Git version info" gives `git version 2.56.0.windows.4`.
  `the_direct_program_sees_the_launchers_environment` panicked with
  "expected Git for Windows' launcher layout around `C:\Program
  Files\Git\ucrt64\bin\git.exe`" (`program.rs:310`). That path is what
  `which::which("git")` returned, so the runner's `PATH` names `ucrt64\bin`
  ahead of `cmd`.

### Reproducing the three failures on the rig

The runs used a private clone at `a816f1447` and `just _test`, with
`PathFirst` meaning "prepended to `PATH`".

```text
worktree git::program::   PATH as-is (cmd\git.exe):          7/7 pass
worktree git::program::   mingw64\bin first:                  FAIL the_named_installation_is_the_prefix_git_reports_and_stays_out_of_git_children
                                                              FAIL the_direct_program_sees_the_launchers_environment
git-wt-cli a_listing_never_asks_git_for_its_exec_path
                          PATH as-is:                         pass
                          mingw64\bin first:                  FAIL
```

These are the same three tests and assertions as the nightly:
`program.rs:285`, `program.rs:310`, and `list_output.rs:94` with
`left: 1, right: 0`.

### Timing method

The script is `a3wt.ps1`, run on the rig and removed afterward.

- Each arrangement got its own scratch `HOME`, `XDG_CACHE_HOME`, and
  `LOCALAPPDATA`, with `EXEPATH` and `GIT_EXEC_PATH` unset and
  `NO_COLOR=1`.
- The repository was a temporary local one with three linked worktrees and
  no remote, so the listing does no network work.
- The binary was a debug `wt.exe` from the test build. A release build was
  not timed.
- **Simulating an unrecognized launcher without touching code or the
  install** (arrangement C):
  - A scratch directory got a copy of the real `cmd\git.exe` launcher, plus
    directory junctions `mingw64`, `usr`, and `etc` to the real install.
  - It also got a junction **`mingw32`** pointing at `mingw64`. Two
    `PREFIXES` candidates then exist, so `launcher_layout` returns `None`:
    the same code path a `ucrt64`-only install takes.
  - The copied launcher still starts `mingw64\bin\git.exe` (the trace
    shows ancestry `git.exe` then `wt.exe`, meaning launcher, then wt).
  - The junctions were removed with `cmd /c rmdir` before any recursive
    delete, and the real install was checked intact afterward.
- Git process counts come from `GIT_TRACE2_EVENT`, one file per real Git
  process. Launchers do not write traces.
  - The first cold run in each script run also did merge-state work
    (3 `rev-list` and 3 `merge-tree`). It was cached per repository and
    absent from every later run in every arrangement, so it is a first-run
    effect, not a difference between arrangements.
- **Correctness:** the full `wt list` output, both streams, captured through
  `cmd /c "… 2>&1"`, was identical across A/B/C, cold and warm. It showed a
  4-row table: base repo, `wt1`–`wt3`, all clean.

## What this changes in the spec

- **The known-failures row "3 | `worktree`, `git-wt-cli` | Runner Git at
  `ucrt64\bin\git.exe`, not recognized | Real, small product gap" mixes two
  findings and should be split:**
  1. **The three CI failures:**
     - Cause: the runner's `PATH` names a prefix binary
       (`ucrt64\bin\git.exe`) ahead of the launcher. The two `git::program`
       tests assume a launcher is the `git` on `PATH` (`program.rs:285`,
       `:310`), and `a_listing_never_asks_git_for_its_exec_path` assumes
       `wt` can name the installation.
     - Under that `PATH`, plain `git` is already the real binary (one
       process creation), so the only real cost is the one `--exec-path`
       per process (case B, +33%).
     - Whether this is a test-environment assumption or a product gap
       depends on a decision the spec has not made: should `wt` also
       recognize "`PATH` names `<top>\<prefix>\bin\git.exe` directly" and set
       `EXEPATH` to `<top>`? If yes, it is a product fix, and it also turns
       the `--exec-path` test green. If no, the two program tests need a
       launcher-first `PATH` or a skip condition, and the exec-path test
       is wrong on any host where the binary comes first.
  2. **The ucrt64 gap:**
     - Real, and it affects every user on Git for Windows ≥2.56 with the
       default `cmd` on `PATH`: case C, correct but about +70% `wt list`
       time.
     - Adding `("ucrt64", "UCRT64")` to `PREFIXES` is the likely fix, but it
       is **not verified**. No ≥2.56 install was available to confirm that
       the 2.56 launcher sets `MSYSTEM=UCRT64` and the same `PATH` prefix.
       `the_direct_program_sees_the_launchers_environment` is the test that
       would prove it, on a ≥2.56 host with `cmd` first on `PATH`.
     - The urgency is moderate: performance only, but it is now the default
       configuration for every newly installed user.
- **Outcome, "`wt` handles the `ucrt64` layout, or is shown to be correct
  without it":** shown correct without it. Handling it is a performance fix.

## Surprises

- The ucrt64 PREFIXES entry would not fix the CI cells. The spec's framing
  of the row as one small product gap would have produced a fix that leaves
  the runner red. The PATH-order cause moves this row's seam.
- The rig is one Git release behind the runner: 2.55.0.windows.3 against
  2.56.0.windows.4. The rig therefore cannot show the runner's layout until
  it is upgraded. Upgrading it, an author decision, is the only way to
  verify the ucrt64 fix locally.
- Release notes advise depending on `cmd\git.exe` as "guaranteed to stay
  stable", while `PREFIXES` hard-codes internal prefix names that just
  changed. Any future MSYS2 prefix change will silently fall back again.
