---
title: CI/CD stabilization — land the three-PR stack and get real Windows evidence
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
  - .github/ci/README.md
  - .github/kache-version
  - .github/ci/areas.json
  - .github/ci/ci-baseline.toml
  - .github/workflows/_area-ci.yml
  - .github/workflows/_wsl-ci.yml
  - .github/workflows/ci.yml
  - .github/workflows/pr-health.yml
  - .gitignore
  - Cargo.lock
  - README.md
  - docs/initialization.md
  - docs/kache-strategy.md
  - justfile
  - scripts/init.ps1
  - scripts/ci/affected_scope.py
  - scripts/ci/test_affected_scope.py
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
---

# CI/CD stabilization

## Objective

Land the three-PR stack in an order that produces **verified** cross-platform
evidence, then burn down the Windows failures against that evidence. Configure
kache to match host filesystem capability instead of forcing one policy on every
host.

Success is not "CI is green". Success is:

1. Every CI-gating area's declared Windows L1 and compile-check cells **report**
   (pass or fail), rather than being blocked, missing, or unscheduled. Declared
   policy gaps remain visible and are not counted as executed evidence.
2. Every Windows fix already written has an applicable cell proving the changed
   behavior, not merely a green job elsewhere in the same area.
3. kache is either earning its keep on a leg or is off on that leg, with the
   decision backed by a measurement rather than an assumption.

## Current state (measured 2026-07-30)

| | PR 19 `docs/cross-platform-ci-plan` | PR 21 `fix/windows-sniff` | PR 22 `fix/unchained-hug-schematic` |
|---|---|---|---|
| Base | `main` | `main` | `fix/windows-sniff` |
| Remote head | `b09b1f50e` | `02a89f149` | `f83c69da5` |
| Files changed | 268 | 195 | 15 |
| Ancestry | 34 commits on `8fc3adc3a` | 27 commits on `8fc3adc3a` | 6 commits on PR 21 |
| CI jobs scheduled | 168 cells | **32 jobs, 0 area jobs** | **0 runs** |
| Failures | 43 (all named) | 2 | unknown |

Ancestry, branch heads, and pull-request metadata were verified through the
GitHub API against a fully unshallowed clone. The Actions job counts and log
conclusions are retained from the named runs below.

### Ancestry is ordinary — an earlier "unrelated history" finding was a clone artifact

An intermediate review of this plan recorded that PR 19 had no common ancestor
with `main` and proposed a repair phase to reconstruct its tree. **That finding
was wrong**, and the repair phase has been removed.

The local clone was shallow. `.git/shallow` contained exactly
`43056c8bcad232ce56228d9dbe086673f0af6c59` — the same commit the finding named as
PR 19's "root". A shallow boundary makes Git treat that commit as parentless
*locally*, so `git merge-base` fails, `git rev-list --max-parents=0` reports it as
a root, and the branch appears to contain only 18 commits.

Verified ancestry, after `git fetch --unshallow`:

| Comparison | Merge base | Relationship |
|---|---|---|
| `main` ↔ PR 19 | `8fc3adc3a` | PR 19 is 34 ahead, 0 behind — `MERGEABLE` |
| `main` ↔ PR 21 | `8fc3adc3a` | PR 21 is 27 ahead, 0 behind |
| PR 19 ↔ PR 21 | `2d6a606d5` | diverged: PR 21 +12, PR 19 +19 |

PR 21 branched from PR 19 at a **real shared commit**, `2d6a606d5`
("docs(devops): record the silently-unscheduled-PR failure mode and its guard").
Ordinary merges and rebases are safe. PR 19's `mergeStateStatus` is `BLOCKED`
because the required `ci-verdict` check is failing, not because of conflicts or
graph problems.

**Operating rule for this repo:** check `.git/shallow` before drawing any
conclusion from `git merge-base`, `git rev-list --count`, or
`git rev-list --max-parents=0`. A shallow clone reports all three in ways that
look like history corruption. Prefer the GitHub compare API
(`gh api repos/<owner>/<repo>/compare/<base>...<head>`) as the authority, since it
reads the full server-side graph.

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

This is the same *class* of defect `pr-health.yml` exists to catch (absence of a
run), but the guard checks only whether GitHub can create a merge ref. Its push
path can pass a mergeable stacked PR and print that CI can be scheduled without
checking whether `ci.yml`'s base-branch filter excludes it. Phase 3 aligns those
contracts.

## Why PR 21's run failed

`fix/windows-sniff` branched from PR 19 at `2d6a606d5`, then added 12 commits of
its own while PR 19 added 19 more. It does not revert those 19 changes — it
simply predates them. Because the branch point is a real shared commit, Git can
identify the earlier CI work as shared history, and the 12-commit range replays
cleanly onto a merged PR 19.

What PR 21 is missing, and what each omission costs:

| PR 19 commit | Missing from PR 21 | Consequence |
|---|---|---|
| `43056c8bc` runtime `cargo_bin` | 56 `cargo_bin!` macro sites remain | **Causes the `biscuit-hash / wsl2` failure** |
| removal of `-D warnings` from `check` | `_area-ci.yml:115` still sets it | **Causes the `playa / check (windows)` failure** |
| `656299926` `node` capability | `areas.json` has no `node: true`; the config validator that made an undeclared pnpm user fail loudly is also gone | homelab's 22 frontend tests silently never run |
| `0ecd4e45d` anchored tier filters | `test(/level2_/)` instead of an anchored tier predicate | Unanchored predicates match substrings anywhere in the path |
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

PR 19 is the dependency of everything else. It is an ordinary descendant of
`main` and merges normally. Its 43 red cells are the honest picture the branch
was built to expose, not a regression.

**Chosen route: one-time admin merge**, with the full failure set, source run,
head SHA, and reason for the override recorded in the PR body. The person
performing the merge must have explicit authority to bypass the required check;
this plan does not turn a draft status into that authorization.

Chosen over widening `ci-baseline.toml` deliberately. Baselining 43 cells to
land the branch that *made them visible* would convert a one-time exception into
43 standing masks, each needing its own retirement later. The admin merge leaves
the failures loudly red, which is the correct signal going into Phase 4.

The baseline stays reserved for failures that are genuinely accepted for a
period, and every such entry still needs `owner`, `reason`, `source_run`, and
`expiry`.

Before the override:

- confirm branch protection still requires only `ci-verdict` — as of 2026-07-30
  ruleset `protect-your-bacon` (id 19747338) lists it as the sole required check,
  so this is a check, not a change;
- confirm the head being merged is the head that was reviewed;
- confirm `Cargo.lock` is tracked and the six stale baseline entries remain
  retired;
- pause unrelated merges until the post-merge `main` run reports, so a second
  change cannot be confused with the accepted failure set.

**Exit criteria:** PR 19 is on `main`; `ci-verdict` is the required check;
`Cargo.lock` is tracked; the post-merge `main` run is linked from the PR.

### Phase 2 — Replay PR 21's own 12 commits onto the new `main`

The branch point is real (`2d6a606d5`), so this is an ordinary rebase of a known
range rather than a reconstruction. Use the range explicitly rather than a plain
`git rebase main`: if PR 19 is squash-merged, its commits lose per-commit patch
identity and Git cannot reliably skip PR 21's copies of the shared CI work.

The PR 21-only range is exactly 12 commits (`47a550084^` resolves to
`2d6a606d5`, the verified branch point):

```text
git rebase --onto origin/main 2d6a606d5 fix/windows-sniff
```

It changes 81 files (1,924 insertions, 201 deletions) before conflict
resolution. PR 19's tree is authoritative when an old policy and a later PR 19
correction overlap. Record every dropped or rewritten hunk; do not resolve
conflicts by taking an entire side.

The replay also implements the kache policy in this plan. In particular, do not
preserve `949e76016`'s tracked, unconditional `.cargo/config.toml` wrapper
merely because it is part of the 12-commit range.

Verify after the replay, before pushing:

- `rg -n 'cargo_bin!' --glob '**/tests/**'` produces no matches
- `rg -n RUSTFLAGS .github/workflows/_area-ci.yml` shows it only on `lint`
- `.github/ci/areas.json` still carries `"node": true` for `homelab`
- `just/devops.just` tier filters are anchored with `(^|::)`
- kache action calls, report steps, reusable-workflow inputs, and per-area
  policy are absent
- `git ls-files --error-unmatch Cargo.lock` succeeds and `.gitignore` does not
  re-ignore it
- `scripts/ci/affected_scope.py` retains `NODE_PROVISIONED_ENVIRONMENTS`, the
  `node` schema validation, and the justfile/pnpm drift guard, together with
  their tests in `test_affected_scope.py`
- `.github/ci/README.md` agrees with `AREA_DEFAULTS`, including
  `wsl2-ubuntu` in the default environment list (the PR 19 README currently
  omits it)
- the final diff from post-PR-19 `main` is the reviewed 12-commit intent plus
  the explicitly listed conflict resolutions and kache-policy changes, with no
  deletion of PR 19's later fixes or specs

Update the PR branch with `--force-with-lease` against recorded head
`02a89f149`. A changed 195-file GitHub count is expected because the PR will no
longer carry the older CI stack; review the resulting inventory instead of
treating the old count as a target.

**Exit criteria:** the `playa` and `biscuit-terminal` Windows `check` cells pass,
the canary stage succeeds, and **the area fan-out actually runs** — the first
Windows evidence for the `sniff` work.

### Phase 3 — Make stacked CI explicit, rebuild PR 22, then retarget

Include in rebased PR 21 a change making `ci.yml` run for pull requests
against any base, rather than only `main`. `affected_scope.py` already uses the
event's base and head SHAs, so a stacked PR naturally validates only its delta.
The modest extra runner cost is the price of evidence before a parent branch
lands.

Update `pr-health.yml` in the same change. Today its `push` path can say a
non-`main` PR "can schedule CI" after checking only mergeability, even though
`ci.yml` filters that PR out. Once all PR bases are supported, the name and
claim become true. Add a workflow-contract test so the two trigger policies
cannot drift apart again.

Rebuild PR 22 onto rebased PR 21 with the safe, genuinely shared range:

```text
02a89f149..f83c69da5
```

Equivalently, after recording both old heads, rebase the six PR 22 commits with
`--onto <new-pr21-head> 02a89f149`. Use `--force-with-lease`.

One known conflict: PR 22's `fa0087bcd fix(research): support symlinks on
Windows` and PR 19's `ef1be90c1 fix(research): create symlinks on Windows
instead of refusing` are independent implementations of the same fix touching
the same three files (`research/lib/src/link/creation.rs`,
`research/lib/src/link/mod.rs`, `research/lib/src/pull.rs`). Resolve to one
implementation; do not merge both.

PR 22's `renderable` (rooted asset paths) and `queue` (native shell) fixes are
**additive** — they close cells PR 19 leaves red.

Run CI while PR 22 still targets rebased PR 21. After PR 21 lands, retarget
PR 22 to `main`. If PR 21 was squash- or rebase-merged and its rebuilt head is
not an ancestor of `main`, rebase the six-child-commit range onto `origin/main`
before retargeting; otherwise the PR diff will reintroduce its parent.

**Exit criteria:** stacked PRs schedule `ci.yml`; `pr-health` and `ci.yml` agree
on applicability; PR 22's `biscuit-file`, `queue`, `renderable`, `research`, and
`unchained-ai` changes receive applicable CI cells before merge; the final
retargeted diff contains only the six-child-commit intent and recorded conflict
resolution.

### Phase 4 — Windows burn-down

Baseline from PR 19's verdict (run 30489327076), the only measured run with full
visibility. Numeric entries are failing-test counts, not job or rollup-cell
counts; textual entries describe missing producer evidence:

| Area | Windows observation | Other environment observations |
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

PR 21 and PR 22 may address much of this. **None of it is verified**, which is
the entire argument for Phases 2 and 3. Re-derive this table from the first
post-replay run before assigning any burn-down work, and separate test
failures from build failures, timeouts, cancellations, missing artifacts, and
policy gaps.

Two items are independent of the Windows product bugs and should be tracked
separately:

- **`claudine` build-time budget.** In the measured run, ~20m45s of a 30-minute
  job was compilation and the library was built three times. Sharding tests
  does not automatically shard compilation. Windows already has
  `Swatinem/rust-cache`; it did not keep this cold/miss path inside the budget.
  Raise `timeout-minutes` first so the next run produces evidence, then profile
  whether invocations can share build work without breaking per-package JUnit
  identity.
- **`darkmatter` Windows timeout.** Diagnose it independently before calling it
  the same class; a timeout alone does not prove duplicate compilation is the
  cause.

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
(`docs/kache-strategy.md`). The measured CI legs are getting essentially
nothing; do not generalize that result to a future backend or action version.

### Why

The logs are explicit:

```
kache: no S3 remote configured — falling back to GitHub Actions cache
GitHub cache key: kache-v0.8.0-linux-x64-49bb78f395f904c1
```

The action's GitHub-cache key is based on kache version, OS, architecture, and
`Cargo.lock`, so it is not literally one key across every OS. It is still one
exact key shared by all same-platform area jobs at the same lockfile state. An
exact GitHub Actions cache entry is immutable, so the first successful save for
that key wins and later jobs cannot accumulate their disjoint stores into it.

Pull-request caches are scoped to the PR merge ref. They can restore from their
base/default branch, but caches they create are available only to reruns of that
PR; they are not immediately deleted when the PR closes. They expire or are
evicted under GitHub's retention and repository-size policy. See GitHub's
[dependency caching reference](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching).

The WSL2 leg also hit `Failed to restore: Cache service responded with 400` and
a failed save. That is consistent with cache-service or quota pressure but does
not prove the cause. Inspect the Actions cache inventory before attributing it
to the default 10 GiB repository limit.

`Swatinem/rust-cache@v2` already persists Cargo artifacts on every native leg
and should remain while kache is disabled. The kache report does not measure
`rust-cache`, so do not claim its hit rate without separate evidence.

### Target configuration

| Target | Decision | Rationale |
|---|---|---|
| **macOS dev** | **Opt in after probe** | Measured 99.6% warm on the current APFS layout. Other store/target layouts must still prove clone support. |
| **Linux dev** | **Opt in after probe** | ext4 is hardlink mode: store ingestion is a second copy and live target links limit reclamation. btrfs / XFS-reflink are stronger candidates. |
| **Windows dev** | **Off by default** | NTFS defaults to copy restore. Opt in only after a ReFS Dev Drive (store and target together) is measured. |
| **WSL2 dev** | **Qualified like Linux** | A normal distro root is commonly ext4 in a VHDX; measure storage and restore behavior. |
| **WSL2 CI guest** | **No** | It executes a prebuilt nextest archive and compiles nothing inside the guest. |
| **CI** | **Off for now** | Keep `rust-cache`. Revisit only with an S3/R2 backend. |

### Actions

**K1 — Move the wrapper from repository policy to host policy.**

PR 21's tracked `.cargo/config.toml` sets
`[build] rustc-wrapper = "kache"` repo-wide and unconditionally. A contributor
who clones and runs Cargo before installing kache gets a hard failure. It also
forces CI and unsupported filesystem layouts to opt out of repository policy.

The kache skill added in PR 22 states the rule directly:

> Separate host policy from repository policy. A tracked Cargo wrapper affects
> every developer OS. Prefer explicit CI activation and host-local opt-in when
> filesystems differ across the team.

Drop the tracked wrapper and do **not** silently activate it from `just init`.
Automatic activation on every Linux host would contradict the filesystem probe
required above, and `kache init` writes to `$CARGO_HOME/config.toml`, affecting
other repositories on that host.

Remove `_ensure-kache` from `just init` and the native Windows initialization
path. Keep it as an explicit pinned installer, then make activation a separate
developer choice after `kache init --check`, `kache doctor`, and a store/target
clone-mode probe. Document both supported activation scopes:

- per shell: `RUSTC_WRAPPER=kache`, the narrowest and easiest to undo;
- host-wide: `kache init`, only with informed consent because it modifies Cargo
  home configuration.

Document rollback (`RUSTC_WRAPPER=""` for a shell, or remove the wrapper from
Cargo home). Do not create an ignored repository `.cargo/config.toml`
automatically; hidden local policy is difficult to diagnose.

**K2 — Remove kache from CI until a remote backend experiment justifies it.**

Keep `Swatinem/rust-cache@v2`, but remove the kache action calls, reporting
steps, reusable-workflow inputs, and per-area `kache` policy rather than leaving
a permanently false dead path. Keep `.github/kache-version` for developer
installation.

If a short experiment is needed before removal, the current action exposes
`cache-key-prefix`; scope it by area and job/tier to stop same-platform writers
colliding, then compare wall time and weighted hit rate against the no-kache
control. Per-area keys trade collision for fragmentation and are not the target
architecture.

**K3 — Keep the version pin at 0.12.0.**

`.github/kache-version` = **0.12.0** is the developer-install authority selected
by PR 21; PR 19 still pins `0.8.0`. `_ensure-kache` must compare the installed
version with the file and install that exact version on mismatch.

Do not attribute Windows block cloning to 0.12.0: the repository's own kache
research records ReFS support as present in 0.8.0. The 0.12.0 pin is still
useful for the newer diagnostics used by this policy, but it must be validated
as the intended release rather than justified by a feature-version claim that
the same documentation contradicts.

**K4 — `cargo binstall` is the canonical developer install path on every OS.**

It fetches a prebuilt binary rather than compiling kache from source. On every
platform, the documented entry point is the explicit root installer recipe. The
recipe, not a Bash-only command substitution in user documentation, resolves
`.github/kache-version` and runs the equivalent of:

```sh
cargo binstall --no-confirm --force --version 0.12.0 kache
```

The root justfile's `_ensure-kache` recipe already performs the pinned install
and should stay as the canonical developer path, but no longer as a dependency
of general initialization. Document it in the kache skill's
`installation.md`, which currently leads with per-OS package managers (mise,
brew, apt, AUR, winget, scoop, choco) — those become fallbacks, not the
recommendation.

This developer decision does not prescribe the future CI installer. The
mutable `kunobi-ninja/kache-action@v1` used by the measured run rejected
`win32-x64`, but its current official documentation now lists Windows x64 and
arm64 as supported and exposes both `version` and `cache-key-prefix`. That drift
is itself a reason not to encode the old limitation as permanent architecture.

The ordering is:

1. Apply K2 — kache off in CI now.
2. If a remote backend lands, compare the official action with an explicit
   installer/backend design. If the action is used, pin it to a reviewed commit
   SHA, pass `.github/kache-version`, assign distinct manifest/namespace keys by
   build variant, and measure Windows rather than assuming support or value.

**K5 — Keep `cargo-sweep` regardless.**

kache's per-crate keying degrades roughly 100× on a large `target/` (~18 s/crate
on a 957k-file `target/deps` versus ~30–170 ms clean). Target hygiene is a
speed requirement, not just a disk one, and is independent of every decision
above.

## Verification scope

Per repo policy, run gates only for the recorded scope — never
`cargo build --workspace` or an unscoped root lifecycle recipe.

- Before implementation gates, refresh GitNexus successfully, run impact
  analysis for every changed symbol, and use `sniff repo packages` plus current
  metadata to record affected packages, package areas, and downstream
  consumers. Stale graph output is not valid dependency evidence.
- Every force-updated branch requires ancestry, range, and changed-file
  inventory checks before a push, and `--force-with-lease` pinned to the
  recorded remote head.
- Confirm the clone is not shallow (`.git/shallow` absent) before reasoning
  about history; see the ancestry note above for why.
- Phase 2 and 3 changes are CI configuration plus per-area product fixes; verify
  each recorded package area with `just build`, `just test`, and `just lint`.
- Cross-platform compile checks on a macOS host use
  `cargo xwin check --target x86_64-pc-windows-msvc <area check_args> --all-targets`,
  with the area's declared `check_args` — not a bare `-p <lib>`, which silently
  skips cfg-gated modules. This is compile evidence only; Windows runtime
  behavior still requires a native Windows CI cell.
- Run `actionlint` for workflow syntax and expression validation.
- `scripts/ci/test_affected_scope.py` and
  `tools/test-toolkit/tests/ci_workflow_contracts.rs` guard the CI configuration
  itself and must pass before any workflow change is pushed.
- Run the `ci-rollup` test suite whenever scope, artifact identity, baseline, or
  verdict behavior changes.

## Non-goals

- Authoring new Windows product fixes in this plan. Phases 2 and 3 integrate the
  fixes already written; Phase 4 sequences any remaining work under area specs.
- Redesigning the canary gate. It behaved correctly — blocking the fan-out on a
  canary failure is the intended contract, and the problem was the failure, not
  the gate.
- Adopting an S3/R2 remote for kache. K2 defers the CI decision until that exists
  as a separate piece of work.
- Changing `ci-verdict` as the required status check.

## Execution log — 2026-07-30

| Phase | State | Evidence |
|---|---|---|
| 1 — land PR 19 | **done** | merge commit `53cfeec00`; admin override inside a restored-on-exit ruleset window; `enforcement=active`, `bypass_actors=0` after |
| 2 — replay PR 21 | **done** | `git rebase --onto main 2d6a606d5`; 11 of 12 clean, one conflict; head `f2f600a9f` |
| 3 — stacked CI + PR 22 | **done** | `ci.yml`/`pr-health.yml` base filters removed; PR 22 head `c3f8ae798`; first-ever `ci` run **30562649123** (112 jobs) |
| 4 — Windows burn-down | **handed to Ken** | targets identified below; owned on a separate branch |
| 5 — remove the blinders | **done** | run **30595280027**: 238 jobs, 151 pass, 55 fail, **0 without a verdict**, 2.4h |

Recovery tags pushed to the remote: `recovery/pr19-pre-merge` (`b09b1f50e`),
`recovery/pr21-pre-rebase` (`02a89f149`), `recovery/pr22-pre-rebase`
(`f83c69da5`).

### Phase 5 — the goal restated, and met

Ken reframed the objective mid-execution, and it was the right correction: the
aim is **a CI/CD process that reports the true state of the software**, not one
where tests pass. Remove what blinds us; fix bugs afterwards. Work had drifted
into fixing individual tests to get past a gate, when the gate was the problem.

Two structural blinders were removed (`161402b6e`), both "fail fast to save
runner minutes" trades that cost exactly the visibility the pipeline exists to
provide:

- **The canary gated the whole area fan-out.** Three consecutive runs produced
  ZERO area evidence because one canary test failed — a Windows compile-check, a
  WSL2 binary path, and a 40 ms timing assertion on macOS. None said anything
  about the other twenty areas, yet each erased them: 31 jobs instead of 168.
  Seven further jobs carried the same clause.
- **L1 gated L2/browser/wsl within each area.** One red L1 deleted the whole
  tier's evidence and the rollup logged MISSING — indistinguishable from a leg
  that was never scheduled.

Both now keep `needs:` for ordering and run on `!cancelled()`. Removing the gate
also made the run *faster* — 2.4 h for 238 jobs versus 4.0 h for 172 — because
areas start immediately instead of queuing behind the canary stage. Wall clock
here is bounded by runner concurrency, not job duration.

Supporting fixes in the same phase:

| Commit | Blinder removed |
|---|---|
| `64837bd21` | `sniff repo --json` aborted on CI's shallow clone (`try_find_object`); added a test owning its own depth-1 fixture |
| `ba2f46229` | 30 min job ceiling killed cells into MISSING → 45 min |
| `3a274b2fa` | 30 s per-test kill failed correct tests → 90 s; claudine's shard had 11 timeouts and **zero** assertion failures |
| `268ab36bf`, `be7e4f613` | per-package invocation resolved features per package, rebuilding crates 3× and hiding feature-gated tests; one resolution per area |
| `7f2bbf7ce` | `progress_resets_stall_clock` had never run in CI; 40 ms budget widened |
| `e5ee7c7f8` | a dead WSL2 guest rendered identically to a tier with no tests; producer now records *why* |

### Remaining blind spots — two, both WSL2 guest instability

Confirmed not to be reporting bugs. Control case: darkmatter shards 1/2/4 had
real test failures, their guests stayed healthy, staging succeeded, and reports
came through. Only a dead guest loses evidence.

1. **claudine wsl2, all four shards** — nextest killed by SIGBUS (signal 7, exit
   135) ~3m45s in, right after `Extracting 153 binaries`. claudine has the
   largest archive (153, vs darkmatter 130, biscuit-terminal 55). Signal 7 is
   what the kernel delivers when a memory-mapped file cannot be backed, so VHDX
   disk or memory exhaustion is the lead. **Diagnose before fixing.**
2. **darkmatter wsl2 shard 3/4** — `Provision the WSL2 guest` failed with
   `wsl.exe` exit 4294967295. A flake; the other three provisioned. Wants a
   bounded retry on that step only — never on the test step, where a retried
   timeout would look healthy.

`claudine/windows-latest/L1` also renders MISSING, but honestly: the crate fails
to BUILD, so the test set is genuinely unknown. That is the rollup working.

### Phase 4 targets — 917 failing tests, two causes explain ~800

macOS and ubuntu are healthy: 14 failures between them. This is Windows (550)
and WSL2 (353).

| Area | macOS | ubuntu | Windows | WSL2 |
|---|---:|---:|---:|---:|
| `darkmatter` | · | 1 | **480** | MISSING |
| `tree-hugger` | · | · | 6 | **150** |
| `biscuit-terminal` | 3 | 2 | 36 | 48 |
| `biscuit-file` | · | · | 4 | 43 |
| `biscuit-speaks` | · | · | · | 34 |
| `schematic` | · | · | 8 | 24 |
| `research` | · | · | 2 | 28 |
| `sniff` | 3 | 1 | 5 | 19 |
| `unchained-ai` | · | · | 5 | 6 |
| others | 1 | 3 | 4 | 1 |

**Target 1 — `md.exe` stack-overflows on Windows (~451 tests, 49% of all
failures).** `code=-1073741571` = `0xC00000FD` = `STATUS_STACK_OVERFLOW`, on
*every* subcommand (`compose`, `clean`, `schema`, `code-block`, `get`, `set`,
`hash`, `graph`, `rm`) — so it is at startup, not in one code path. Windows gives
the main thread 1 MB against 8 MB on Linux/macOS. Candidates: a Windows-scoped
`-C link-arg=/STACK:8388608`, or moving the work onto a thread with an explicit
stack size. Note `.cargo/config.toml` does not exist on any branch — it was
deleted with the kache wrapper — so that file would be created fresh and must be
scoped to `[target.x86_64-pc-windows-msvc]`.

**Target 2 — archived tests resolve fixture paths to the build host (~250–350
tests, 9 areas).** `Io { path: "/home/runner/work/rusty-biscuit/…", NotFound }`
is the *Windows host's* checkout; the guest has the repo at
`/home/runner/rusty-biscuit`. `--workspace-remap` fixes nextest's bookkeeping but
not paths baked into the binary at compile time via `CARGO_MANIFEST_DIR`. Same
class as the `cargo_bin!` bug already fixed in `43056c8bc`; one helper likely
closes all nine areas.

**Both PR 21 canary failures are resolved**, in run 30562547696:
`canary / playa / check (windows-latest)` and
`canary / biscuit-hash / wsl2 / test (wsl2-ubuntu)` both pass, so the area
fan-out is no longer gated.

Conflicts resolved, both predicted by this plan:

- `sniff/{lib,cli}/Cargo.toml` — kept PR 21's `default-features = false` with
  PR 19's fuller comment and range ceiling, dropping PR 19's now-false remark
  that `Cargo.lock` is gitignored.
- `research/lib/src/{link/creation.rs,link/mod.rs,pull.rs}` — the duplicate
  symlink fix. `creation.rs` was functionally identical; `pull.rs` was not.
  PR 22's made any symlink failure fatal, PR 19's treats
  `ERROR_PRIVILEGE_NOT_HELD` as a warning and continues. Kept PR 19's, because a
  default Windows host without Developer Mode cannot create symlinks and the
  framework aliases are non-essential. PR 22's commit dropped as empty.

### Found during execution, not planned for

`lockfiles_stay_gitignored_so_release_checkout_cannot_block` had been failing on
`main` since PR 19 committed the lockfiles — that commit reversed the premise the
test asserts without updating it. Replaced with the current policy.

It exposed a live risk that is **not closed**: release-plz asserts a clean
tracked worktree after release calculation, and a regenerated lock is now
tracked, so that assertion will see it and fail the release. The path runs only
on `main` after a green CI run and cannot be exercised from a branch. Recorded in
`release-plz.yml`. When it fires, choose between committing the regenerated lock
as part of the release commit or scoping the assertion to exclude lockfiles — do
not simply re-ignore lockfiles, which would undo the reproducibility this branch
bought.

## Acceptance criteria

- [x] PR 19 is merged by an authorized admin override; its failure set, source
      run, and head SHA are recorded in the PR body, and no baseline entries were
      added merely to achieve the merge.
- [x] `Cargo.lock` is tracked on `main`.
- [x] PR 21 is replayed from the 12-commit range (`2d6a606d5..02a89f149`) onto
      post-PR-19 `main` and every item in the Phase 2 verification list holds.
- [x] `canary / playa / check (windows-latest)` passes.
- [x] `canary / biscuit-hash / wsl2 / test` passes.
- [x] The area fan-out runs on PR 21 — `matrix.area` is no longer `skipped`.
- [x] `sniff` has Windows L1 and compile-check cells reporting real results.
- [x] `ci.yml` schedules stacked pull requests and `pr-health.yml` has the same
      applicability contract, enforced by a workflow-contract test.
- [~] PR 22 runs CI while stacked (**done**) and the duplicate `research`
      symlink implementation is resolved to one (**done**); retargeting to
      `main` waits on PR 21 landing.
- [x] The Windows failure table is re-derived from a post-replay run,
      with failures, timeouts, cancellations, missing evidence, and policy gaps
      distinguished.
- [x] No tracked Cargo configuration imposes a rustc wrapper on every host, and
      `just init` does not silently activate a host-wide wrapper.
- [x] The pinned `cargo binstall`-backed root recipe is documented as the
      developer install path on every OS, in the kache skill's
      `installation.md` and wherever developer setup is described.
- [x] kache action calls, reports, reusable-workflow inputs, and per-area policy
      are removed from CI; `Swatinem/rust-cache@v2` remains.
- [~] `.github/kache-version` is `0.12.0` and the developer installer verifies
      the installed executable against it — true on the branch; reaches `main`
      when PR 21 lands.
- [ ] Any future kache CI experiment is separately scoped, uses a reviewed
      commit-SHA-pinned action or explicit backend, and reports weighted hit rate
      plus wall-clock comparison against a no-kache control.

### Phase 5 — reporting the true state

- [x] No cell in a completed run lacks a verdict — run 30595280027: 238 jobs,
      zero cancelled, zero timed out.
- [x] A canary failure no longer suppresses the area fan-out, and no job gates on
      canary success. Guarded by a contract test, proven non-vacuous.
- [x] A red L1 no longer suppresses its area's L2, browser, or WSL2 tiers.
- [x] A dead WSL2 guest is distinguishable from a tier that ran no tests.
- [ ] The WSL2 provisioning flake is retried (`wsl.exe` exit 4294967295).
- [ ] The claudine WSL2 SIGBUS has a confirmed cause. Diagnose before fixing.
- [ ] Phase 4 burn-down proceeds against a run in which every cell reported.
