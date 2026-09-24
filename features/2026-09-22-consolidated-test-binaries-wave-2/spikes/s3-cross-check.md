---
kind: spike
feature: 2026-09-22-consolidated-test-binaries-wave-2
spike: S3
created: 2026-09-23
rev: 208051f753aa9ce0fcc19622f64464d11d3570d2
---

# S3: what `just cross-check <pkg> --os windows` actually proves

Read from source only; nothing was run. `just cross-check *args` is a thin
wrapper over `./scripts/cross-check.sh` (`justfile:115-117`).

## 1. HEAD or working tree?

**The working tree**: tracked edits plus untracked, non-ignored files, over
the nearest remote base. It is not `HEAD`, and local commits that are not
pushed come along only because their content is in the tree.

- Base: `origin/<branch>` if it exists, else `origin/main`
  (`scripts/cross-check.sh:251-257`). The local `HEAD` is never the base.
- The deciding lines (`scripts/cross-check.sh:332-335`):

  ```bash
  GIT_INDEX_FILE="${index_file}" git read-tree "${base_sha}"
  GIT_INDEX_FILE="${index_file}" git add --all
  tested_tree="$(GIT_INDEX_FILE="${index_file}" git write-tree)"
  ```

  A throwaway index is seeded from the base and then given `git add --all`
  over the working tree. The result is committed as an unsigned synthetic
  commit (`:343-350`) and shipped as a `git bundle` (`:350`). If the tree
  equals the base, nothing is bundled and the base is what gets tested
  (`:338-341`).
- On the host, the clone is reset and cleaned, the bundle is fetched, and the
  clone is checked out detached at exactly that revision (`:560-569` Unix,
  `:942-956` Windows).
- The banner reports what shipped:
  `cross-check: <pkg> @ origin/<branch> (<sha>) + N changed file(s) as <rev>`
  (`:352-353`).
- Tests pin this down: `test_a_dirty_tree_reaches_the_host_as_one_clean_revision`
  and `test_an_ahead_and_dirty_tree_...` (`scripts/ci/test_cross_check.py:671,680`).

## 2. Features, tier filter, extra args

There are two modes, and the only thing that selects between them is whether a
Cargo build flag was passed (`scripts/cross-check.sh:215-229`).

**Archive mode (default, no build flag)**: the features come from
`[package.metadata.ci.tests] features`, not `local-features`.

- The plan is computed locally with `affected_scope.py --all --resolved-plan`
  and no `--event` (`scripts/cross-check.sh:371-372`). With no event, every
  environment gets planned, `windows-latest` included
  (`scripts/ci/affected_scope.py:3361`).
- The planner reads `tests.features` into the record
  (`affected_scope.py:1714`) and turns it into `--features a,b`
  (`feature_args`, `:2485-2489`). That becomes the build identity's
  `features` (`:3083`). `local-features` is only validated (`:1483-1492`) and
  never reaches the build.
- `ci-build produce` hands `identity.features` to `cargo nextest archive`
  (`scripts/ci-build-archive.rs:1059`, and `:1027` for examples).
- Result:
  - **`biscuit-tui-cli`** gets `--features terminal-tests`
    (`biscuit-tui/cli/Cargo.toml:82`). Its `local-features = []` (`:83`) is
    ignored here. So `[[test]] windows_captured_stdout`
    (`required-features = ["terminal-tests"]`, `:41-43`) **is compiled** into
    the Windows archive.
  - **`sniff`** gets `--features remote` (`sniff/lib/Cargo.toml:115-116`). Its
    Windows-only files are auto-discovered and have no `required-features`,
    so they compile anyway:
    - `sniff/lib/tests/windows_app_paths_orphan.rs:7` and
      `windows_find_program_priority.rs:6` use
      `#![cfg(target_os = "windows")]`.
    - Item-level `#[cfg(target_os = "windows")]` tests are in
      `integration.rs:1642,1734,2487,2511,2538` and
      `program_installable.rs:9,26,33`.
    - There is also one `#[cfg(windows)]` in
      `merge_conflict_prediction.rs:511`.

**Tier filter: L1 only.** The host runs `just _test '<pkg>' --no-fail-fast
--archive-file ...` (`scripts/cross-check.sh:663-664`, Windows `:860`).

- `_test` applies `_tier_filter L1` (`just/devops.just:1247`), which is
  `!(level2_|level3_|browser_|real_|slow_)` (`:1069-1072`).
- In archive mode that filter becomes `package(<pkg>) & (...)`
  (`:1266-1272`), and it runs at `:1284`.
- None of the tests in question has a tier prefix:
  - `captured_stdout_receives_only_value_no_tui_bytes`
    (`biscuit-tui/cli/tests/windows_captured_stdout.rs:350`)
  - `path_wins_over_fallbacks_for_cmd`
  - `orphaned_hkcu_entry_is_filtered`

  So all of them are L1 and selected.

**Extra nextest args: yes.** Anything that is not a build flag goes into
`run_args` (`scripts/cross-check.sh:218-225`) and is appended to the nextest
invocation after `-E "$filter"` (`:664`/`:860` → `devops.just:1284`).
Positional args are test-name substrings. The `os` skill warns that `just`
re-splits arguments, so an `-E '...'` containing spaces or `|` has to go
through `./scripts/cross-check.sh` directly
(`.claude/skills/os/build-hosts.md:88-92,108-111`).

**Native mode (any `--features`, `--all-features`, or `--no-default-features`)**
applies to every host and takes a different path:

- It runs `cargo nextest run -p <pkg> --no-fail-fast <all extra args>`
  (`scripts/cross-check.sh:575`, `:873`).
- It applies **no tier filter**, so L2/L3/`real_`/`slow_` tests run as well,
  and it does not stage JUnit.
- It publishes nothing (`:398-399`).
- It is not needed for either package above, because archive mode already
  carries the declared features.

**Skill drift.** `.claude/skills/os/build-hosts.md:86-87` says "feature flags
are routed to the archive build and everything else to the run". The code
does the opposite: a feature flag abandons archive mode for every host
(`scripts/cross-check.sh:68-72,226-229`). The skill line is stale.

## 3. Where per-test results appear

**Only on the live terminal. Nothing per-test is kept locally.**

- **Per-test PASS lines:** each leg's SSH stdout streams to the local
  terminal (`scripts/cross-check.sh:715`, `:988`).
  - The `ci` nextest profile (`NEXTEST_PROFILE` defaults to `ci`, `:610`,
    `:813`) does not override `status-level` (`.config/nextest.toml:174-208`).
  - So nextest's default `status-level = pass` prints one
    `PASS [..] <binary> <test>` line per test.
  - That is the only place a Windows-only test can be *seen* to have run.
- **JUnit:** `_test` → `_stage_junit` copies nextest's
  `target/nextest/ci/test-results.xml` into `$reports/L1/<pkg>.xml`
  (`just/devops.just:1298`, `:630-637`). The remote script then copies it to
  `<clone>/cross-check-<run_id>-report.xml` (`scripts/cross-check.sh:864-866`).
  It is then **deleted without being fetched**:
  - Windows: `:989-991`, plus `$consume` removal at `:968`.
  - Linux/macOS: `discard_remote_report`, `:719,724-727`.
  - Only the WSL leg fetches its report (`:749-750`), and only into
    `$work_dir`.
- **Local persistence: none.** `$work_dir` is a `mktemp -d` that the EXIT
  trap removes (`:328-330`). It holds `wsl.log`, the WSL leg's tee (`:711`,
  WSL only), and the fetched WSL report. There is no log directory.
- **Final summary:** one `pass`/`FAIL` per OS (`:1022-1029`), with no test
  counts.
- **WSL receipt:** the only durable output is the WSL receipt, a git note on
  `refs/notes/ci-local/wsl2-ubuntu` (`:790`). It is published only for an
  exact-head, clean, unfiltered run, and it does not help with Windows.

So cross-check can show Windows per-test evidence only if someone captures the
terminal, for example with `./scripts/cross-check.sh sniff --os windows 2>&1 |
tee <file>`, and then greps for the test names. Given `--no-tests=pass`
(`devops.just:1276-1279`), a Windows-only test binary that compiled to nothing
would not fail the run. Proof therefore needs the named PASS line, not a green
summary.

## Smallest addition that would give durable per-test Windows evidence

Features need no change. For the evidence gap, the smallest change is in
`run_windows`, just before the `Remove-Item` at `scripts/cross-check.sh:990`:
`scp` the report
`${base}/cross-check-${run_id}-report.xml` back to a persistent local path,
for example `target/cross-check/<run_id>/windows-<pkg>.xml`. That is the same
fetch the WSL leg already does at `:749`, pointed outside `$work_dir`, and it
could be generalized to `discard_remote_report` for the Unix legs. The JUnit
file lists every executed `<testcase>`, so a Windows-only module can be shown
to have compiled and run by name. An optional second step is to `tee` every
leg's stream (not only WSL's, `:711`) into the same directory.
