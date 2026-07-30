---
title: CI/CD stabilization — land three divergent branches and get real Windows evidence
status: draft
created: 2026-07-30
builds_on:
  - fixes/2026-07-27-refactor
pull_requests:
  - "#19 docs/cross-platform-ci-plan -> main"
  - "#21 fix/windows-sniff -> main"
  - "#22 fix/unchained-hug-schematic -> fix/windows-sniff"
source_code:
  - .cargo/config.toml
  - .claude/skills/kache/installation.md
  - .github/actions/enable-kache/action.yml
  - .github/actions/report-kache/action.yml
  - .github/kache-version
  - .github/ci/areas.json
  - .github/ci/ci-baseline.toml
  - .github/workflows/_area-ci.yml
  - .github/workflows/ci.yml
  - scripts/ci/affected_scope.py
---

# CI/CD stabilization

## Objective

Get the three open branches merged in an order that produces **verified**
cross-platform evidence, then burn down the Windows failures against that
evidence. Configure kache to match the OS support policy instead of forcing one
policy on every host.

Success is not "CI is green". Success is:

1. Every area's Windows cell **reports** (pass or fail), rather than being
   blocked, missing, or unscheduled.
2. Every Windows fix already written has a cell proving it works.
3. kache is either earning its keep on a leg or is off on that leg, with the
   decision backed by a measurement rather than an assumption.

## Current state (measured 2026-07-30)

| | PR 19 `docs/cross-platform-ci-plan` | PR 21 `fix/windows-sniff` | PR 22 `fix/unchained-hug-schematic` |
|---|---|---|---|
| Base | `main` | `main` | `fix/windows-sniff` |
| Files changed | — | 195 | 15 |
| CI jobs scheduled | 168 cells | **32 jobs, 0 area jobs** | **0 runs** |
| Failures | 43 (all named) | 2 | unknown |

### PR 21 has not tested its own content

Its run (30524532660) scheduled 32 jobs and **zero area jobs** — `matrix.area`
concluded `skipped`. This is the canary gate behaving as designed: `area-ci`
carries

```yaml
if: >-
  !cancelled() &&
  needs.scope.outputs.has_areas == 'true' &&
  (needs.canary.result == 'success' || needs.canary.result == 'skipped')
```

and `canary / playa / check (windows-latest)` failed, so the fan-out never
started. `sniff` — the headline of the PR — was never built or tested on any
operating system.

### PR 22 gets no CI by construction

`ci.yml` declares `on: pull_request: branches: [main]`. PR 22 targets
`fix/windows-sniff`, so no `ci` run is created at all; only `pr-health` and
Socket report. Five product fixes are unverified.

This is the same *class* of defect `pr-health.yml` exists to catch (a PR that
schedules no run), but not the same instance — `pr-health` checks that a run was
created for PRs it observes, and a stacked PR legitimately has no `ci` workflow
to create.

## Root cause of both PR 21 failures

`fix/windows-sniff` is a **rebase of the CI branch taken at an earlier point**,
plus new work. It reverts nothing in git terms — it simply predates PR 19's last
18 commits. Because both branches share `main` (`8fc3adc3a`) as their merge
base, these are one-sided changes and git will resolve nearly all of them in
PR 19's favour automatically.

What PR 21 is missing, and what each omission costs:

| PR 19 commit | Missing from PR 21 | Consequence |
|---|---|---|
| `43056c8bc` runtime `cargo_bin` | 56 `cargo_bin!` macro sites remain | **Causes the `biscuit-hash / wsl2` failure** |
| removal of `-D warnings` from `check` | `_area-ci.yml:115` still sets it | **Causes the `playa / check (windows)` failure** |
| `656299926` `node` capability | `areas.json` has no `node: true`; the config validator that made an undeclared pnpm user fail loudly is also gone | homelab's 22 frontend tests silently never run |
| `0ecd4e45d` anchored tier filters | `test(/level2_/)` instead of `test(/(^|::)level2_/)` | Unanchored predicates match substrings anywhere in the path |
| `9a01ba383` `report-kache` action | absent | No cache-effectiveness signal at all |
| `61466f94e` committed `Cargo.lock` | `.gitignore:68` re-ignores `**/Cargo.lock` | CI re-resolves every dependency every run; a past run cannot be reproduced |
| `eca67d517` baseline pruning | 6 retired entries restored | Now-passing cells stay masked |
| `ae5525845` ci-rollup fix | passing evidence from unscheduled cells still erased | Rollup misreports |

### Defect 1 — `playa / check (windows-latest)` is `-D warnings`, not a compile error

Reproduced locally with `cargo xwin` at CI's exact package scope:

```sh
RUSTFLAGS=""            cargo xwin check --target x86_64-pc-windows-msvc \
                        -p playa -p playa-cli --all-targets   # 6 warnings, exit 0
RUSTFLAGS="-D warnings" cargo xwin check --target x86_64-pc-windows-msvc \
                        -p playa -p playa-cli --all-targets   # could not compile
```

The six warnings map 1:1 onto CI's six errors — three `unsafe_op_in_unsafe_fn`
(`warning[E0133]`, a warn-by-default lint even in edition 2024) and three
dead-code findings, all in `playa/lib/src/windows_com.rs`.

**The package scope is load-bearing.** `windows_com` is gated behind
`#[cfg(all(target_os = "windows", any(feature = "sfx-native-audio", feature =
"audio-ducking-windows")))]`, and only `playa-cli`'s default features enable it.
A check of `-p playa` alone compiles none of it and reports clean — which is how
this was initially misdiagnosed as dependency drift. Always use the area's
declared `check_args`.

It is also broader than `playa`: under `-D warnings` the build dies first in
**`biscuit-terminal`** (three dead-code warnings for cfg'd-out Unix helpers).
That cell was never scheduled, so the failure is latent rather than absent.

### Defect 2 — `biscuit-hash / wsl2` is compile-time binary-path baking

```
Failed to spawn ".../target/x86_64-unknown-linux-gnu/debug/bh": No such file or directory
```

`cargo_bin!` is a macro that expands to `env!("CARGO_BIN_EXE_…")`, baking an
absolute path at compile time. The WSL2 leg runs a **prebuilt nextest archive**,
so the guest looks for the binary where the builder had it. The free function
`assert_cmd::cargo::cargo_bin` reads `env::var_os` at runtime and works in both
modes.

## Plan

### Phase 1 — Land PR 19

PR 19 is the dependency of everything else. Its 43 red cells are the honest
picture the branch was built to expose, not a regression.

**Approved route: one-time admin merge**, with the full failure set recorded in
the PR body so it is recoverable without re-running CI.

Chosen over widening `ci-baseline.toml` deliberately. Baselining 43 cells to
land the branch that *made them visible* would convert a one-time exception into
43 standing masks, each needing its own retirement later. The admin merge leaves
the failures loudly red, which is the correct signal going into Phase 4.

The baseline stays reserved for failures that are genuinely accepted for a
period, and every such entry still needs `owner`, `reason`, `source_run`, and
`expiry`.

**Exit criteria:** PR 19 is on `main`; `ci-verdict` is the required check;
`Cargo.lock` is tracked.

### Phase 2 — Rebase PR 21 onto the new `main`

Expect a mostly clean rebase: the eighteen missing commits are one-sided
relative to the shared base.

Verify after rebasing, before pushing:

- `git grep -c 'cargo_bin!' -- '*/tests/*'` returns **0**
- `grep -n RUSTFLAGS .github/workflows/_area-ci.yml` shows it only on `lint`
- `.github/ci/areas.json` still carries `"node": true` for `homelab`
- `just/devops.just` tier filters are anchored with `(^|::)`
- `.github/actions/report-kache/action.yml` exists
- root `Cargo.lock` is tracked and `.gitignore` does not re-ignore it
- `scripts/ci/affected_scope.py` retains `NODE_PROVISIONED_ENVIRONMENTS`, the
  `node` schema validation, and the justfile/pnpm drift guard, together with
  their tests in `test_affected_scope.py`

**Exit criteria:** the `playa` and `biscuit-terminal` Windows `check` cells pass,
the canary stage succeeds, and **the area fan-out actually runs** — the first
Windows evidence for the `sniff` work.

### Phase 3 — Retarget PR 22 to `main`

Once PR 21 lands, retarget so PR 22 gets a `ci` run.

One known conflict: PR 22's `fa0087bcd fix(research): support symlinks on
Windows` and PR 19's `ef1be90c1 fix(research): create symlinks on Windows
instead of refusing` are independent implementations of the same fix touching
the same three files (`research/lib/src/link/creation.rs`,
`research/lib/src/link/mod.rs`, `research/lib/src/pull.rs`). Resolve to one
implementation; do not merge both.

PR 22's `renderable` (rooted asset paths) and `queue` (native shell) fixes are
**additive** — they close cells PR 19 leaves red.

Separately, decide whether `ci.yml` should trigger on non-`main` bases. A
stacked PR that silently runs no CI is a visibility gap regardless of how this
particular stack resolves.

**Exit criteria:** PR 22's changes have cells.

### Phase 4 — Windows burn-down

Baseline from PR 19's verdict (run 30489327076), the only run with full
visibility:

| Area | Windows | Other environments |
|---|---|---|
| `sniff` | **394** | 5 ubuntu, 4 macOS |
| `biscuit-terminal` | 36 | 2 ubuntu, 3 macOS |
| `schematic` | 8 | |
| `tree-hugger` | 6 | |
| `unchained-ai` | 5 | |
| `biscuit-file` | 4 | |
| `queue` | 2 | |
| `research` | 2 | |
| `renderable` | 1 | |
| `biscuit-tui` | 1 | |
| `biscuit-icon` | — | 1 wsl2 |
| `biscuit-speaks` | — | 1 ubuntu, 1 macOS |
| `claudine` | build failure + missing shards | 15+ ubuntu |
| `darkmatter` | MISSING (timeout) | 26 ubuntu, 1 macOS |

PR 21 and PR 22 plausibly address most of this. **None of it is verified**, which
is the entire argument for Phases 2 and 3. Re-derive this table from the first
post-rebase run before assigning any burn-down work.

Two items are independent of the Windows product bugs and should be tracked
separately:

- **`claudine` build-time budget.** ~20m45s of a 30-minute job is compilation;
  the lib is rebuilt three times per job; sharding tests does not shard the
  build; Windows has no compile cache. Options: raise `timeout-minutes`,
  collapse the triple rebuild into one nextest invocation, or get a cache onto
  Windows. The middle option is the real fix but interacts with per-package
  JUnit staging.
- **`darkmatter` Windows timeout.** Same class, different area.

Already specced, to run after these merges:

- `claudine/fixes/2026-07-29-windows-paths/spec.md` — the real path-separator
  fix, replacing the interim one-time `tracing::warn!`
- `features/2026-07-29-reclassify-browser-tests/spec.md` — retire the Browser
  tier into L2
- `worktree/fixes/2026-07-29-inconsistent-checks/spec.md` — `wt remove -b` merge
  authority

## kache

### Measured reality

From `report-kache` in run 30489327076:

| Leg | Hit rate | Weighted by compile cost | Time saved |
|---|---|---|---|
| `biscuit-terminal` ubuntu | 3.8% (26 hit / 666 miss) | **0.4%** | ~2s |
| `biscuit-terminal` macOS | 6.2% (44 / 664) | **2.3%** | ~15s |
| `playa` ubuntu | 0% (0 / 17) | 0% | n/a |
| `playa` macOS | 0% (0 / 17) | 0% | n/a |
| `biscuit-hash` ubuntu | 0% (0 / 2) | 0% | n/a |
| `biscuit-hash` macOS | 100% (2 / 0) | 100% | 437ms |

Against the local macOS measurement of **99.6% warm, 35.1 → 18.3 min**
(`docs/kache-strategy.md`). CI is getting essentially nothing.

### Why

The logs are explicit:

```
kache: no S3 remote configured — falling back to GitHub Actions cache
GitHub cache key: kache-v0.8.0-linux-x64-49bb78f395f904c1
```

**One key for the entire repository.** GitHub Actions cache entries are
immutable and branch-scoped: the first job to finish wins the write, every other
leg's save is discarded, and a PR branch's writes are invisible to other
branches and deleted with the PR. The store cannot accumulate. The WSL2 leg also
hit `Failed to restore: Cache service responded with 400` and a failed save,
suggesting pressure against the 10 GB/repo quota.

Meanwhile `Swatinem/rust-cache@v2` runs alongside kache on every leg and is doing
the caching that actually works.

### Target configuration

| Target | Decision | Rationale |
|---|---|---|
| **macOS dev** | **Yes** | APFS reflink; measured 99.6% warm. kache's best case. |
| **Linux dev** | **Yes, qualified** | ext4 is hardlink mode — the store is a genuine second copy and `gc` cannot reclaim blobs a live `target/` still links. Unqualified yes on btrfs / XFS-reflink. |
| **Windows dev** | **No** | NTFS is copy-mode. Only a ReFS Dev Drive makes it worthwhile. |
| **WSL2** | **No** | The WSL2 CI leg runs a prebuilt archive and compiles nothing. |
| **CI** | **Off for now** | Keep `rust-cache`. Revisit only with an S3/R2 backend. |

### Actions

**K1 — Move the wrapper from repository policy to host policy.**

The tracked `.cargo/config.toml` sets `[build] rustc-wrapper = "kache"`
repo-wide and unconditionally, which forces kache onto every OS including
Windows, where the answer is no. A Windows contributor who clones and runs
`cargo build` before `just init` gets `failed to run 'kache'` on every command.
It is also why `enable-kache` must *neutralize* the wrapper on five separate
legs and why every other workflow sets `RUSTC_WRAPPER: ""` at workflow level —
a lot of machinery to undo a decision the config file made too broadly.

The kache skill added in PR 22 states the rule directly:

> Separate host policy from repository policy. A tracked Cargo wrapper affects
> every developer OS. Prefer explicit CI activation and host-local opt-in when
> filesystems differ across the team.

Seed a host-local config from `just init` on macOS and Linux; drop the tracked
wrapper. Cargo config has no conditionals, so the environment variable is the
only mechanism that can express a per-host decision.

**K2 — Disable kache in CI until a remote backend exists.**

The GitHub-cache fallback structurally cannot work with one shared key. If it
is kept in the interim, give `kache-action` a per-area cache key so legs stop
colliding.

**K3 — Keep the version pin at 0.12.0.**

`.github/kache-version` = **0.12.0** is the single authority and is correct.
PR 19 still pins `0.8.0`; the merge must land on 0.12.0. The `enable-kache`
verify step compares `kache --version` against the file and fails the bootstrap
on a mismatch, so a stale pin surfaces loudly rather than silently building
through an unexpected wrapper.

0.12.0 is where `doctor` reports the store filesystem and where Windows block
cloning exists, so the upgrade matters beyond the version string.

**K4 — `cargo binstall kache` is the install path on every OS.**

Regardless of platform, install kache with:

```sh
cargo binstall kache
```

It fetches a prebuilt binary rather than compiling from source, and it honours
an exact version, so it satisfies the single-authority requirement directly:

```sh
cargo binstall kache@"$(tr -d '[:space:]' < .github/kache-version)"
```

The root justfile's `_ensure-kache` recipe already does this and should stay as
the canonical developer path. Document it in the kache skill's
`installation.md`, which currently leads with per-OS package managers (mise,
brew, apt, AUR, winget, scoop, choco) — those become fallbacks, not the
recommendation.

This has a direct consequence for CI. `kunobi-ninja/kache-action@v1` is the
**only** component that rejects `win32-x64`, and it is also what installed
`0.8.0` against the older pin in run 30489327076. Replacing it with binstall
would remove the Windows exclusion and guarantee the pinned version on every
leg.

It would not, however, make kache worth running in CI on its own: the action
also supplies the GitHub-cache-backed store persistence, and that backend is
precisely what K2 finds worthless (one repo-wide key, immutable and
branch-scoped entries). So the ordering is:

1. Apply K2 — kache off in CI now.
2. If and when a remote backend lands, re-enable CI kache as
   **binstall + explicit `actions/cache` or S3**, not `kache-action@v1`. At that
   point Windows CI becomes a policy decision (measure it) rather than a
   platform limitation, and `docs/kache-strategy.md`'s "Option B" collapses to
   the install step described here.

**K5 — Keep `cargo-sweep` regardless.**

kache's per-crate keying degrades roughly 100× on a large `target/` (~18 s/crate
on a 957k-file `target/deps` versus ~30–170 ms clean). Target hygiene is a
speed requirement, not just a disk one, and is independent of every decision
above.

## Verification scope

Per repo policy, run gates only for the recorded scope — never
`cargo build --workspace` or an unscoped root lifecycle recipe.

- Phase 2 and 3 changes are CI configuration plus per-area product fixes; verify
  with each affected area's `just build`, `just test`, `just lint`.
- Cross-platform compile checks on a macOS host use
  `cargo xwin check --target x86_64-pc-windows-msvc <area check_args> --all-targets`,
  with the area's declared `check_args` — not a bare `-p <lib>`, which silently
  skips cfg-gated modules.
- `scripts/ci/test_affected_scope.py` and
  `tools/test-toolkit/tests/ci_workflow_contracts.rs` guard the CI configuration
  itself and must pass before any workflow change is pushed.

## Non-goals

- Fixing the Windows product bugs in this plan. Phase 4 sequences them; the
  fixes belong to their area specs.
- Redesigning the canary gate. It behaved correctly — blocking the fan-out on a
  canary failure is the intended contract, and the problem was the failure, not
  the gate.
- Adopting an S3/R2 remote for kache. K2 defers the CI decision until that exists
  as a separate piece of work.
- Changing `ci-verdict` as the required status check.

## Acceptance criteria

- [ ] PR 19 is merged by admin override; its failure set is recorded in the PR
      body, and no baseline entries were added to achieve the merge.
- [ ] `Cargo.lock` is tracked on `main`.
- [ ] PR 21 is rebased and every item in the Phase 2 verification list holds.
- [ ] `canary / playa / check (windows-latest)` passes.
- [ ] `canary / biscuit-hash / wsl2 / test` passes.
- [ ] The area fan-out runs on PR 21 — `matrix.area` is no longer `skipped`.
- [ ] `sniff` has a Windows cell reporting a real result.
- [ ] PR 22 targets `main` and has CI cells; the duplicate `research` symlink
      implementation is resolved to one.
- [ ] The Windows failure table is re-derived from a post-rebase run.
- [ ] `.cargo/config.toml` no longer imposes a rustc wrapper on every host.
- [ ] `cargo binstall kache` is documented as the install path on every OS, in
      the kache skill's `installation.md` and wherever developer setup is
      described.
- [ ] kache is off in CI, or reports a weighted hit rate that justifies keeping
      it on.
- [ ] `.github/kache-version` is `0.12.0` on `main`, and the installed wrapper
      matches it on every leg that installs kache.
