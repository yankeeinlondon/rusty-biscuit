---
title: Cross-platform CI refactor — every supported environment runs every applicable test
status: draft
created: 2026-07-27
supersedes:
  - features/2026-06-07-matrix-testing (deleted 2026-07-27; last at 7d4125fa0)
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

Every CI-enabled package area runs the same canonical L1 suite on **Windows,
WSL2, Linux, and macOS**. Every area that owns L2 tests also runs every
applicable L2 test on each environment with a provisioned backend. Results are
legible as a package-area × environment rollup, and green cells provably
executed tests.

Windows, macOS, and Linux are operating systems; WSL2 is a distinct supported
Linux environment hosted by Windows. The rollup and policy schema should call
this dimension `environment`, even if GitHub runner labels remain named `os`.
Platform `#[cfg]` gates and declared backend applicability mean the discovered
test set can differ by environment; "same suite" means the same canonical
recipe and no ad hoc filtering within that recipe.

Platform-specific exclusions are permitted but must be rare, explicit, and
justified in `areas.json`. A tier with no tests is `N/A`, not PASS; a tier with
tests but no provisioned compatible backend is a policy gap, not an acceptable
green skip.

`areas.json` currently has 31 area records: 21 are matrix-enabled and 10 are
explicitly `ci: false`. This refactor does not silently treat those 10 as
covered. Each must either be promoted after gaining the canonical recipe set or
remain an explicit exclusion with an owner and follow-up.

## Why now

The repo has been developed ~95% on macOS. Windows and Linux environments now
exist, and WSL2 is available for testing. The current CI encodes decisions made
under constraints that no longer hold:

| Decision | Original constraint | Status |
|---|---|---|
| `soft_os: ["windows-latest"]` | roll Windows in without blocking merges | expired — 14 areas red in run 30323254931, permanently muted |
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
- **`features/2026-06-07-matrix-testing/`** — the original multi-OS spec.
  Superseded by this plan and **deleted 2026-07-27**; its last living copy is at
  commit `7d4125fa0` (`git show 7d4125fa0:features/2026-06-07-matrix-testing/spec.md`).
  Its salvageable content is carried below, so this section — not git history —
  is the reference.

### Salvaged from the superseded matrix-testing spec

**Keep — its D2 filesystem decision.** That spec already resolved the WSL
checkout question: clone into the WSL guest filesystem
(`git clone "${GITHUB_WORKSPACE}" /home/runner/rusty-biscuit`) rather than
building over `/mnt/c`. Its reasoning was the WSL1 translation-layer penalty;
under WSL2 the penalty is the 9p boundary, which is worse, so the conclusion
strengthens rather than weakens under the version change. Adopted into 2.1.

**Keep — docs-only changes must not spin up the matrix.** Its AC1 required that
modifying a non-code file not trigger the multi-OS suite. Carried into the
success criteria; verify against `affected_scope.py` rather than assuming.

**Keep — `homelab` as the worked example of a legitimate exclusion.** It targets
physical IoT hardware, so it is excluded on capability grounds, not because it is
backlog. Useful as the reference case when resolving the 10 `ci: false` records
in 3.5: a real exclusion looks like this; the rest need promotion.

**Reject — D3's branch gating** (Linux-only on feature branches, full matrix only
on `main`). Its stated rationale was minimizing Actions credit consumption, which
is void for a public repo. More importantly it detects cross-platform breakage
only *after* the offending commit reaches `main`, forfeiting cheap bisection and
recreating the deferral pattern that produced the current backlog.

**Reject — its macOS exclusion premise** ("the primary developer develops and
thoroughly tests on macOS locally"). Refuted by measurement: `sniff` is the only
area whose tests run on macOS in CI, and it fails there today.

**Superseded — its WSL provisioning mechanic** (install rustup and `just` inside
the guest, then compile). Its own AC2 worried about GHA timeouts from exactly
this. The `nextest archive` approach in 2.2 removes the toolchain install and the
compile from the guest entirely.

**Superseded — `just ci-changed-areas` and git-range change detection.**
`scripts/ci/affected_scope.py` does strictly more: reverse-dependency closure,
area-schema validation, and shadow-workspace detection.

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

Make the shared test runner preserve one XML document per nextest invocation
under a staging directory keyed by `{tier}/{package}`. Copy the report
immediately after each invocation, including a failing invocation, before the
next invocation can overwrite it. The area recipe must continue through its
remaining packages and return a combined non-zero status afterward; otherwise
an early package failure suppresses both later tests and their reports. Upload
the whole staging directory, not only `target/nextest/ci/test-results.xml`.

This second defect is already live and compounds the first. `just` aborts a
recipe at its first failing line, so when `_test biscuit-file` fails,
`_test biscuit-file-cli` never executes. In run 30323254931 the `biscuit-file`
lib failed on Windows, which means **`biscuit-file-cli` was never tested on
Windows at all** — and the run reports no evidence of that gap. Overwrite loses
the earlier package's report; abort loses the later package's tests entirely.

Do not concatenate XML text. Either retain the set of valid JUnit documents for
the rollup consumer or merge suites with an XML-aware tool while recomputing
counts and durations. Add contract tests for two-package success, first-package
failure, sharded runs, and package names containing hyphens.

### 0.2 Build the area × environment rollup

Consume the JUnit artifacts already uploaded per area/OS/shard
(`junit-<area>-L1-<os>-<index>`). Emit:

- a package-area × environment × tier grid of `{pass, fail, skip}` counts into
  `GITHUB_STEP_SUMMARY`
- a machine-readable artifact for 1.3 to diff against

Skip counts are first-class (see 1.2). The rollup must distinguish PASS, FAIL,
SKIP, N/A, MISSING, and NOT SCHEDULED. A scheduled job that produces no valid
report is MISSING and fails the summary gate; it must never render as PASS.
Preserve shard and package identities in the machine-readable schema so counts
can be traced back without parsing display names.

### 0.3 Decouple lint from test

- Remove `needs: lint` from the `test` job (`_area-ci.yml:122`).
- Delete workflow-level `RUSTFLAGS: "-D warnings"` (`_area-ci.yml` `env:` block).

Both are required. The env var re-couples them through the back door: it applies
to the test job's compilation, so a plain rustc warning fails the build and no
tests run. Nothing is lost by deleting it — `just/devops.just:97` already passes
`-D warnings` to clippy directly, so lint warnings remain errors in the lint job.

Any lint failure prevents every L1 matrix leg for that area, because `test`
needs `lint`. Remeasure before implementing — but note that the checked-in
baseline and the observed run already disagree, which is itself evidence for
1.3:

| Source | Lint state |
|---|---|
| `baseline-failures.txt` | 7 entries: `biscuit-speaks`, `biscuit-terminal`, `claudine`, `homelab`, `research`, `tree-hugger`, `worktree` |
| Run 30323254931 (measured) | 4 failed (`homelab`, `research`, `tree-hugger`, `worktree`) + 1 **cancelled** (`claudine`) = 5 areas blocked |

`biscuit-speaks` and `biscuit-terminal` lint now **pass**; the baseline is stale
on both. So the live blocked count is 5, not 7 — the checked-in file overstates
it. Treat neither number as durable; the point is that an unenforced,
hand-maintained baseline drifts in both directions, which is why 1.3 replaces it
with something derived from actual results.

`claudine` being *cancelled* rather than *failed* is a worked example of the
taxonomy in 0.2: it blocked the same test legs as a failure, but a naive
pass/fail reading records neither. The devops handoff independently confirms
claudine's Windows Ctrl+C tests did not run under the staged graph.

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
provisioned backend hard-fails when missing while an inapplicable one still
skips cleanly. Parse a normalized, comma-separated set and reject unknown or
empty entries; do not use substring matching. `require_level!` needs a stable
backend identifier separate from its human-readable diagnostic label.

The contract must also prove execution, not only availability: for every
required backend, record at least one executed test or fail the tier. This can
be derived from backend-tagged JUnit properties or a small machine-readable
manifest emitted by the gating helper. An installed `tmux` plus zero tmux tests
is not evidence.

Without this, `just test-l2` on Windows exits 0 having run nothing:
`TmuxHarness::available()` is `which("tmux")` and tmux has no Windows port;
`AppleTerminalHarness::available()` returns false when `CI=1`
(`apple_terminal.rs:461`); WezTerm and kitty need `WEZTERM_UNIX_SOCKET` /
`KITTY_LISTEN_ON`, i.e. a live GUI session.

### 1.2 Skips are first-class in the rollup

A skip-rate jump reads as loudly as a failure. `require_level!` gating and
harness `available()` checks make silent per-platform skips the default failure
mode of this refactor.

Store an approved skip budget/reason by area, environment, tier, and backend.
Compare exact test identities as well as counts so one removed skip cannot hide
one newly skipped test. New skips block; resolved skips force baseline cleanup.
Compile-time `#[cfg]` absence is N/A and must not be inferred from JUnit alone;
generate the expected-test manifest on the target environment.

### 1.3 Give `baseline-failures.txt` teeth

The file has **31 lines**, but they are not all homogeneous check names (for
example, it also contains generator/coverage labels), and it has zero consumers.
Replace it with a validated machine-readable baseline keyed by stable
`{area, environment, tier, shard}` identity plus an owner, reason, source run,
and optional expiry. Do not key policy to mutable GitHub display names.

- a failure **not** in the list blocks
- a listed entry that is scheduled and **passes** blocks, forcing cleanup
- an entry outside the affected scope is ignored, not treated as a pass
- a scheduled entry that is cancelled, missing, or emits no result remains
  blocking and cannot be accepted as a known test failure

This preserves full counts while not blocking merges on known backlog. Formalizes
the manual baseline-diff procedure in the devops handoff's "ONE RULE".

This requires a deliberate job graph: a downstream `ci-verdict` job downloads
all result/status artifacts, performs the baseline comparison, and is the single
required branch-protection check. Expected-red producer jobs must still run and
remain visibly red, but cannot themselves be required checks or their failure
will bypass the baseline verdict. Use `if: always()` and explicit producer
status artifacts so a failed job cannot prevent the verdict from running.

### 1.4 Delete `soft_os`

`soft-os` is `continue-on-error` (`_area-ci.yml:128`). It does not merely make a
leg non-blocking — it removes the leg from the run's verdict, which is why 14 red
Windows areas (measured, run 30323254931) look normal on `main`. Superseded by
1.3, which keeps the signal.

Remove from `areas.json`, `affected_scope.py:43`, `_area-ci.yml`.

---

## Phase 2 — Open decisions

Not blockers for Phase 0/1. Resolve before the corresponding Phase 3 step.

| # | Decision | Notes |
|---|---|---|
| 2.1 | WSL checkout on ext4 or `/mnt/c` | **Largely pre-decided** — see "Salvaged" below. Clone into the WSL guest filesystem (`git clone "${GITHUB_WORKSPACE}" /home/runner/rusty-biscuit`), not `/mnt/c`. ext4 is also where most WSL devs keep repos. Remaining decision is only whether to add a targeted `/mnt/c` leg for `biscuit-file`, where the differing case-sensitivity, permission, symlink, and inotify semantics are the point rather than an obstacle |
| 2.2 | `nextest archive` for the WSL leg | Build `x86_64-unknown-linux-gnu` once on `ubuntu-latest`, run inside WSL — no toolchain install, no compile, byte-identical binaries to the Linux leg. Needs `--archive-file` passthrough in `just/devops.just:171,446`. **L1 only** — L2 needs the broker binary and a live tmux server |
| 2.3 | Windows L2 backend | No current backend is proven. Spike headless `wezterm-mux-server` + `WEZTERM_UNIX_SOCKET`; if that fails, scope a ConPTY-backed harness as separate work. Until one works, Windows L2 is BLOCKED/POLICY GAP for areas with applicable L2 tests, never a green `0 run / N skipped` cell |

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
| 3.3 | Remove `wsl-version: 1`; promote WSL to a first-class environment; `apt-get install tmux` inside the distro; extend schema, native-package lookup, cache keys, artifact names, and rollup identities for `wsl2-ubuntu` | `messenger-desktop-tests.yml:76`, `_area-ci.yml`, `areas.json`, `affected_scope.py` |
| 3.4 | Inventory L2 tests/backend tags for all 31 declared areas. Enable L2 for every promoted area that owns L2 tests; record `N/A` for areas with none. Today 5 areas set `l2: true` (`biscuit-terminal`, `darkmatter`, `claudine`, `biscuit-icon`, `worktree`) | `areas.json`, area `justfile`s |
| 3.5 | Resolve the 10 `ci: false` records: promote areas with real packages and canonical recipes, or retain a time-bounded exclusion. Specialized workflows must emit the same result schema and feed the same verdict | `areas.json`, `ci.yml`, specialized workflows |

`sniff` is the only area currently testing on macOS (`full_os` override) — and it
is **already failing there**. "macOS is healthy" is an assumption to be measured,
not asserted.

WSL cannot be added to the existing `runs-on` matrix as another runner label:
the job still runs on `windows-latest` and commands execute through `wsl-bash`.
Give it a separate reusable job (or an explicit environment strategy) so native
dependency installation, paths, shells, caching, and artifact collection cannot
accidentally use the Windows branch.

---

## Phase 4 — Burn down

Unblocks the moment Phase 0 lands; runs parallel to 2 and 3. Treat
`features/2026-07-24-devops/ci-failure-inventory.md` as a dated snapshot, not a
live backlog. In particular, its A1 nested-workspace diagnosis is now resolved:
the shadow workspaces were removed, and
`affected_scope.py::validate_no_shadow_workspaces` prevents recurrence.
Reproduce each remaining failure against the current tree before changing code.

| # | Step |
|---|---|
| 4.1 | `biscuit-file` Windows — 24 failures, diagnosed, already gating (`soft_os: []`). Two commits: library normalization, then test POSIX assumptions. See below |
| 4.2 | Remaining current Windows failures after 4.1 — 13 areas as measured in run 30323254931, but derive the live set from the unified result artifact rather than carrying forward the inventory's job/cause counts |
| 4.3 | Linux reds: `darkmatter`, `sniff` |
| 4.4 | The lint failures blocking their areas' test legs — 5 areas measured, 7 stale entries in the checked-in baseline (see 0.3); remeasure before implementation |
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

1. A single view answers "which package areas, in which environments, need attention"
   without hand-written scripts.
2. Every green scheduled cell has a valid report and at least one executed test;
   zero-test tiers are explicitly N/A, and missing evidence is blocking.
3. All four environments run the same canonical L1 recipe per promoted area;
   areas with L2 tests run every applicable backend test where that backend is
   provisioned. Policy gaps and exclusions are explicit entries in `areas.json`
   with an owner and reason.
4. A new failure blocks; a fixed failure must be removed from the baseline.
5. `soft_os` no longer exists.
6. All 31 declared areas are either represented in the unified verdict or have
   an explicit, time-bounded `ci: false` exclusion.
7. A docs-only change does not schedule any test leg on any environment
   (carried from the superseded matrix-testing spec's AC1; verify against
   `affected_scope.py`, do not assume).

## Executing on another host

This plan is written to be picked up on a different machine. Prerequisites:

1. **Fetch the branch.** The plan lives on `docs/cross-platform-ci-plan`; push it
   to `origin` before handing off, or it exists only on the authoring host.
2. **Bootstrap the toolchain** with `just init` — it installs `cargo-nextest`
   and the pinned toolchain from `rust-toolchain.toml`. Additionally needed for
   this work specifically: `actionlint`, `python3`, `jq`, and an authenticated
   `gh` (the rollup and baseline work both query the Actions API).
3. **Re-measure before acting.** Every count in this plan is tagged with run
   30323254931 (2026-07-27). GitHub retains logs and artifacts for a limited
   window, and `main` moves. Regenerate the area × environment table from a
   current full-scope run before starting any phase; treat the numbers here as
   the shape of the problem, not its current state.
4. **Platform reality.** Phases 0, 1, and 5 are host-agnostic — they are CI
   config, `just` recipes, and a leaf crate. Phase 4 burn-down is not: fixing
   Windows failures wants a Windows host, and macOS/WSL failures likewise.
   Sequence the burn-down to whichever hosts are actually available.

   One exception worth planning around: **1.1 is best done on a macOS desktop.**
   It is the only environment where all four L2 backends exist — WezTerm, kitty,
   Apple Terminal, and tmux. CI can never host more than tmux, so the
   backend-identifier contract and the per-backend requirement parsing are far
   easier to get right where every backend can actually be exercised. Observe the
   repo rule that L2 tests must not steal window focus.
5. **Verify workflow changes on a branch PR run.** The devops handoff records
   five CI-only bugs that passed every local check. Local gates are necessary
   and not sufficient.

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
