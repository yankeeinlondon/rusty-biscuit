---
title: Cross-platform CI refactor — every OS runs every test
status: draft
created: 2026-07-27
supersedes:
  - features/2026-06-07-matrix-testing (WSL1 premise)
builds_on:
  - features/2026-07-24-devops
source_code:
  - .github/ci/areas.json
  - .github/ci/baseline-failures.txt
  - .github/workflows/_area-ci.yml
  - .github/workflows/ci.yml
  - .github/workflows/messenger-desktop-tests.yml
  - .config/nextest.toml
  - just/devops.just
  - scripts/ci/affected_scope.py
  - tools/test-toolkit/src/lib.rs
---

# Cross-platform CI refactor

## Objective

Every package area runs the same L1 and L2 test suite on **Windows, WSL2, Linux,
and macOS**, with results legible as a package-area × OS rollup, and with green
cells that provably executed tests.

Platform-specific exclusions are permitted but must be rare, explicit, and
justified in `areas.json` — not an emergent property of what CI happens to be
able to provision.

## Why now

The repo has been developed ~95% on macOS. Windows and Linux environments now
exist, and WSL2 is available for testing. The current CI encodes decisions made
under constraints that no longer hold:

| Decision | Original constraint | Status |
|---|---|---|
| `soft_os: ["windows-latest"]` | roll Windows in without blocking merges | expired — 14 areas red, permanently muted |
| macOS = compile-check only | runner minutes bill ~10× | void — repo is **public**, standard runners are free |
| L2 = Linux only | tmux is the only headless-provisionable backend | partially void — tmux installs on macOS and WSL |
| `wsl-version: 1` | the only WSL environment available at the time | void — action defaults to `2`; hosted runners have had nested virt since Jan 2024 |

## Prior art — read before starting

- **`features/2026-07-24-devops/`** — the staged, dependency-aware CI architecture
  currently in place. `_area-ci.yml`'s `D1`/`D4`/`D6`/`D7`/`D8`/`D9`/`D11`/`D12`
  markers refer to its decisions. `handoff-remaining-work.md` carries hard-won
  gotchas that still apply (verify every workflow change on a branch PR run;
  `actionlint -shellcheck=`; matrix job outputs are last-writer-wins).
- **`features/2026-07-24-devops/ci-failure-inventory.md`** — 34 failing jobs
  reduced to ~14 root causes. Phase 4 below is the burn-down of that inventory,
  which the devops feature declared an explicit non-goal.
- **`features/2026-06-07-matrix-testing/`** — the original multi-OS spec. Its
  WSL1 premise is superseded here. Its status needs updating so the assumption
  is not re-inherited.

## Constraints

- Public repo → standard runners are free. Optimize **wall-clock and concurrency
  slots**, not cost. Free-plan caps: 20 concurrent jobs, ≤5 concurrent macOS.
  Only full-scope runs approach these, and `canary` already gates those.
- Do **not** tag tests by platform-sensitivity and run subsets. The test nobody
  remembered to tag is the test that never runs. Reduce *areas* (affected-scope
  closure already does this); never reduce *tests within an area*.
- Keep the reverse-dependency closure in `affected_scope.py:339`. Testing only
  directly-changed areas would miss behavioral breaks in downstream consumers,
  which `cargo check` cannot catch.

---

## Phase 0 — Make results legible

Nothing downstream can be triaged until this lands.

### 0.1 Fix the per-package JUnit overwrite

`biscuit-file/justfile:64-66` runs nextest twice (`_test biscuit-file`, then
`_test biscuit-file-cli`); `.config/nextest.toml:228` writes every run to the
same `test-results.xml`. The second invocation overwrites the first, so the
uploaded artifact holds only the last package's results. Every multi-package
area is affected.

`_area-ci.yml`'s upload step already acknowledges this ("captures the last
package's results — full multi-package aggregation is a follow-up"). It is a
prerequisite for 0.2, not a follow-up: the rollup is only as good as its input.

### 0.2 Build the area × OS rollup

Consume the JUnit artifacts already uploaded per area/OS/shard
(`junit-<area>-L1-<os>-<index>`). Emit:

- a package-area × OS × tier grid of `{pass, fail, skip}` counts into
  `GITHUB_STEP_SUMMARY`
- a machine-readable artifact for 1.3 to diff against

Skip counts are first-class (see 1.2). A cell that ran nothing must never render
as PASS.

### 0.3 Decouple lint from test

- Remove `needs: lint` from the `test` job (`_area-ci.yml:122`).
- Delete workflow-level `RUSTFLAGS: "-D warnings"` (`_area-ci.yml` `env:` block).

Both are required. The env var re-couples them through the back door: it applies
to the test job's compilation, so a plain rustc warning fails the build and no
tests run. Nothing is lost by deleting it — `just/devops.just:97` already passes
`-D warnings` to clippy directly, so lint warnings remain errors in the lint job.

**Recovers 5 areas** (`claudine`, `homelab`, `research`, `tree-hugger`,
`worktree`) that currently produce zero test data on any OS because their Ubuntu
lint leg fails. Independently confirmed by the devops handoff: claudine's Windows
Ctrl+C tests "have never run" for this reason.

---

## Phase 1 — Make "green" mean something

Must precede Phase 3, or lighting up new platforms manufactures ~100 meaningless
green cells.

### 1.1 Per-backend L2 requirement

`BISCUIT_TEST_LEVEL_REQUIRED=2` (`tools/test-toolkit/src/lib.rs:124`) converts a
`require_level!` skip into a panic, but it is all-or-nothing: setting it globally
panics the GUI-backed tests a headless runner cannot host, which is why
`_area-ci.yml` verifies `tmux -V` as a proxy instead.

Add per-backend granularity (e.g. `BISCUIT_TEST_REQUIRED_BACKENDS=tmux`) so a
provisioned backend hard-fails when missing while an unhostable one still skips
cleanly.

Without this, `just test-l2` on Windows exits 0 having run nothing:
`TmuxHarness::available()` is `which("tmux")` and tmux has no Windows port;
`AppleTerminalHarness::available()` returns false when `CI=1`
(`apple_terminal.rs:461`); WezTerm and kitty need `WEZTERM_UNIX_SOCKET` /
`KITTY_LISTEN_ON`, i.e. a live GUI session.

### 1.2 Skips are first-class in the rollup

A skip-rate jump reads as loudly as a failure. `require_level!` gating and
harness `available()` checks make silent per-platform skips the default failure
mode of this refactor.

### 1.3 Give `baseline-failures.txt` teeth

The file lists 31 known-red checks and has **zero consumers** — no workflow,
script, or doc references it. Wire it up:

- a failure **not** in the list blocks
- a listed entry that **passes** blocks, forcing cleanup

This preserves full counts while not blocking merges on known backlog. Formalizes
the manual baseline-diff procedure in the devops handoff's "ONE RULE".

### 1.4 Delete `soft_os`

`soft-os` is `continue-on-error` (`_area-ci.yml:128`). It does not merely make a
leg non-blocking — it removes the leg from the run's verdict, which is why 14 red
Windows areas look normal on `main`. Superseded by 1.3, which keeps the signal.

Remove from `areas.json`, `affected_scope.py:43`, `_area-ci.yml`.

---

## Phase 2 — Open decisions

Not blockers for Phase 0/1. Resolve before the corresponding Phase 3 step.

| # | Decision | Notes |
|---|---|---|
| 2.1 | WSL checkout on ext4 or `/mnt/c` | ext4 is where most WSL devs keep repos; `/mnt/c` is 9p with different case-sensitivity, permissions, symlink, and inotify semantics. Recommend ext4 primary + a targeted `/mnt/c` leg for `biscuit-file` |
| 2.2 | `nextest archive` for the WSL leg | Build `x86_64-unknown-linux-gnu` once on `ubuntu-latest`, run inside WSL — no toolchain install, no compile, byte-identical binaries to the Linux leg. Needs `--archive-file` passthrough in `just/devops.just:171,446`. **L1 only** — L2 needs the broker binary and a live tmux server |
| 2.3 | Windows L2 backend | No backend exists. Spike headless `wezterm-mux-server` + `WEZTERM_UNIX_SOCKET`; if that fails, scope a ConPTY-backed harness as separate work. Until then Windows L2 renders as `0 run / N skipped`, never PASS |

**Resolved 2026-07-27:** WSL2 on hosted runners is not an open question.
`Vampire/setup-wsl`'s `wsl-version` defaults to `2`, and GitHub's Windows runners
have supported nested virtualization since the Dadsv5 move in January 2024. The
`wsl-version: 1` pin is an explicit downgrade and should simply be removed.

WSL1 is **not** a fallback. WSLg — GUI, audio, clipboard, D-Bus — is WSL2-only,
so under WSL1 `playa`, `messenger`, `biscuit-clipboard`, `biscuit-visualized`,
and `renderable` all take their no-capability fallback branches. A green WSL1 leg
would misreport those packages for every real (WSL2) user. Networking differs
too: WSL1 shares the Windows stack, WSL2 is NAT'd — material for `sniff`,
`homelab`, and `rendezvous`.

---

## Phase 3 — Platform expansion

Gated on Phases 0 and 1, plus the relevant Phase 2 decision.

| # | Step | Where |
|---|---|---|
| 3.1 | Add `macos-latest` to the `full_os` default; drop `check_os` where a test leg now covers it | `affected_scope.py:40` |
| 3.2 | `brew install tmux` + `tmux -V` verification on the macOS L2 job | `_area-ci.yml` |
| 3.3 | Remove `wsl-version: 1`; promote WSL to a first-class leg; `apt-get install tmux` inside the distro; add a `wsl` key to the `native` map | `messenger-desktop-tests.yml:76`, `_area-ci.yml`, `areas.json` |
| 3.4 | `l2: true` for all 31 areas (currently 5: `biscuit-terminal`, `darkmatter`, `claudine`, `biscuit-icon`, `worktree`) | `areas.json` |

`sniff` is the only area currently testing on macOS (`full_os` override) — and it
is **already failing there**. "macOS is healthy" is an assumption to be measured,
not asserted.

---

## Phase 4 — Burn down

Unblocks the moment Phase 0 lands; runs parallel to 2 and 3. Cross-reference
`features/2026-07-24-devops/ci-failure-inventory.md` for shared root causes —
notably A1, where `NEXTEST_PROFILE: ci` fails in nested workspaces.

| # | Step |
|---|---|
| 4.1 | `biscuit-file` Windows — 24 failures, diagnosed, already gating (`soft_os: []`). Two commits: library normalization, then test POSIX assumptions. See below |
| 4.2 | The other 13 red Windows areas |
| 4.3 | Linux reds: `darkmatter`, `sniff` |
| 4.4 | The 5 lint failures blocking their areas' test legs |
| 4.5 | macOS and WSL reds — unknown until Phase 3 measures them |

### 4.1 detail — `biscuit-file`, run 30323254931

Four root causes behind 24 failures:

1. **Tests hardcode POSIX path literals** (all 8 unit failures).
   `Path::new("/tmp").is_absolute()` is `false` on Windows — rooted, but no drive
   prefix. Test-side only; library correct.
2. **`\\?\` verbatim prefix mismatch** (13 of 14 `implicit_relative` failures).
   Tests build expectations from `TempDir::path().canonicalize()` (verbatim on
   Windows); the git-root leg comes from `gix::discover(...).workdir()` (plain).
   Mostly test-side, but two are **real library defects**: root dedupe fails to
   collapse identical dirs spelled differently, and `diff_paths` finds no common
   prefix and falls back to absolute. Fix by normalizing at the library boundary
   (`dunce::simplified`; `dunce` is already in the dependency graph).
3. **Windows home discovery ignores `$HOME`** (2 tests). Deliberate per
   `context.rs:534` (D11). Tests encode a POSIX-only assumption.
4. **Verbatim prefix + forward slash** (2 `precedence_flip` failures). Win32
   skips normalization for `\\?\` paths, so `/` becomes a literal filename
   character and lookup fails. Genuine library gap in interpolated-path handling.

Split commits: library normalization (2, 4) separately from test POSIX
assumptions (1, 3).

---

## Phase 5 — Release tier

| # | Step |
|---|---|
| 5.1 | Release workflow: full workspace, all four OSes, L1 + L2 + browser, feature matrix, MSRV, doctests, packaging/cross-compile smoke |
| 5.2 | lint as prerequisite + `allow-lint-failure` dispatch input |
| 5.3 | Point `release-plz` at it — today it only waits on `ci` completing (`workflow_run`) |

### 5.2 shape

```yaml
test:
  needs: lint
  if: ${{ always() && (needs.lint.result == 'success' || inputs.allow-lint-failure) }}
```

Not `continue-on-error` on the lint job. That makes the failure stop counting and
the release look clean; this form keeps lint red and reported while allowing a
conscious override. Use a `workflow_dispatch` input rather than a label or commit
trailer so the bypass is recorded in run metadata. The summary job should print a
loud line when it is set.

Caveat: the bypass covers clippy failures, not compile failures. If lint failed
because the crate does not build, bypassing moves the same failure into the test
job minutes later.

---

## Sequencing

```
0.1 → 0.2 ─┐
0.3 ───────┼→ 1.1 1.2 1.3 → 1.4 → 3.1 3.2 3.3 3.4 → 4.5
           └→ 4.1 4.2 4.3 4.4        ↑
                                  2.1 2.2 2.3
```

Critical path: 0 → 1 → 3. Phase 2 resolves alongside. Phase 4 starts after
Phase 0; 4.1 is ready now.

**Biggest risk:** running Phase 3 before Phase 1. Turning on macOS, WSL, and
L2-everywhere without per-backend enforcement and skip visibility yields ~100 new
cells that exit 0 without executing anything — indistinguishable from real passes
in the GitHub UI.

## Success criteria

1. A single view answers "which package areas, on which OS, need attention"
   without hand-written scripts.
2. Every green cell provably executed tests; skipped-but-green is impossible.
3. All four platforms run the same L1 + L2 suite per area; exclusions are
   explicit entries in `areas.json` with a stated reason.
4. A new failure blocks; a fixed failure must be removed from the baseline.
5. `soft_os` no longer exists.

## Local gates

Per `features/2026-07-24-devops/handoff-remaining-work.md`:

```bash
actionlint -shellcheck=
cargo nextest run -p test-toolkit --test ci_workflow_contracts
python3 scripts/ci/test_affected_scope.py && rm -rf scripts/ci/__pycache__
just check-canonical
```

Workflow changes must be verified on a branch PR run — the handoff records five
CI-only bugs that passed every local check.
