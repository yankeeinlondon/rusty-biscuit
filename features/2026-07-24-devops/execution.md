---
plan_file: features/2026-07-24-devops/plan.md
phases: 5
status: "phases 1-5 implemented; PRs #5/#7/#8/#9 stacked for review; #6 merged"
stop_reason: ""
started: 24 July 2026
scope: Phase 1 — Restore Bootstrap and Release Signal
---

# DevOps Plan Execution

## Summary

| Metric | Value |
|--------|-------|
| Plan | features/2026-07-24-devops/plan.md |
| Phase in progress | 1 of 5 (Restore Bootstrap and Release Signal) |
| Nature | CI/CD infrastructure: GitHub Actions YAML, Python scope calc, TOML/justfile config, one Rust contract test |
| Local-validation ceiling | Python scope tests, Rust contract tests, `actionlint`, `just` parse, `cargo metadata` without kache. Multi-OS preflight, live `workflow_run` release gating, and real kache-absent CI runs require GitHub Actions and are NOT locally verifiable. |

## Load-bearing decisions confirmed with user

- **OQ3 / Task 1.4 lockfile policy:** Keep all `Cargo.lock` gitignored (repo policy since 2026-04-08). The plan's baseline ("keep schematic/Cargo.lock tracked") was premised on a stale assumption. Chosen: OQ3 **Option C** — gitignore-isolated + clean-tracked-worktree assertions in release-plz. No lockfile is committed; no literal disposable worktree is needed because the ignored file cannot dirty tracked state or block checkout.

## Task 1.1: Verification scope (impact analysis + sniff)

- The Phase 1 changes are almost entirely **non-Rust** infrastructure files (GitHub Actions YAML, `scripts/ci/*.py`, `.cargo/config.toml`, `justfile`, `.github/kache-version`). GitNexus tracks Rust symbols; these files have no symbol graph, so upstream impact analysis is N/A for them.
- The single Rust file changed is a **test file** (`tools/test-toolkit/tests/phase7_ci_workflows.rs` → renamed). Test files have no downstream Rust consumers; blast radius = the `test-toolkit` test binary only. `sniff repo packages` confirms `test-toolkit` is a leaf dev-crate.
- **Recorded verification scope (locally runnable):**
  - `python3 scripts/ci/test_affected_scope.py` (scope calc unit tests)
  - `cargo nextest run -p test-toolkit --test ci_workflow_contracts` (workflow contract tests)
  - `actionlint` on all changed `.github/workflows/*.yml` + the composite action
  - `just --evaluate` / `just check-canonical` (justfile parses after `KACHE_VERSION` sourcing change)
  - `RUSTC_WRAPPER= cargo metadata --no-deps --format-version 1` (clean-checkout regression)
- **Not locally runnable (requires GH Actions):** ACs 1–5 multi-OS bootstrap, AC 28 live release ordering, AC 34 one-failure-per-OS fan-out suppression. These are asserted structurally by the contract tests instead.

## Progress

- [x] Pre-execution baseline: git clean (only CLAUDE.md + spec.md pre-modified, outside blast radius); python scope tests green (6/6); `cargo metadata` OK without wrapper; `phase7_ci_workflows.rs` already RED (4/5, references deleted workflows).
- [x] Task 1.2: kache optional + single version authority
- [x] Task 1.3: dependency-aware bootstrap preflight + scope OS classification
- [x] Task 1.4: release-plz workflow_run gating + clean-tree assertions
- [x] Task 1.5: bootstrap + release workflow contract tests (Python scope + Rust YAML contracts)
- [x] Validation checkpoint 1 (local subset) — all locally-runnable gates green

## Validation checkpoint 1 results

Locally runnable (all GREEN):
- `python3 scripts/ci/test_affected_scope.py` — 11/11 (added windows-path, package-local OS union, global 3-OS, kache-version full-scope, doc-only host-only).
- `cargo nextest run -p test-toolkit --test ci_workflow_contracts` — 9/9.
- `actionlint` (all workflows + composite action) — exit 0, no findings.
- `just --evaluate` → `KACHE_VERSION := "0.8.0"` sourced from `.github/kache-version`; `just check-canonical` 18/18.
- Clean-checkout regression: `RUSTC_WRAPPER="" cargo metadata --no-deps --format-version 1` succeeds with `.cargo/config.toml` removed.
- Non-vacuity proof: reintroduced raw `kunobi-ninja/kache-action` → `area_ci_activates_kache_through_the_verified_composite_action` went RED (TRY 4 FAIL), restored → GREEN.

Requires live GitHub Actions (NOT locally verifiable — asserted structurally by contract tests instead):
- ACs 1–5 real multi-OS bootstrap on fresh runners; AC 4 simulated missing/invalid kache failing a named step end-to-end.
- AC 28 real `workflow_run` ordering (release starts only after CI success on main).
- AC 34 real fan-out suppression (one bootstrap failure per OS, no dependent area jobs).
- These should be exercised on a branch push / PR before this phase is considered fully proven in CI.

## Lockfile-policy direction change (documented 2026-07-24)

The plan's baseline ("keep `schematic/Cargo.lock` tracked + `--locked`") assumed a
tracked lockfile; `git check-ignore` confirms `**/Cargo.lock` is gitignored and
neither lockfile is tracked. Adequacy assessment: the ignore policy itself
neutralizes the original checkout-block failure (a regenerated lockfile is
invisible to `git status --porcelain`/`git checkout`); our design adds
`workflow_run` gating (kills the race), `head_sha` checkout, and clean-*tracked*-
worktree assertions. Residual: the "after" assertion strictness needs a live
release run to confirm; the excluded crate's unpinned resolution is the
pre-existing repo posture (unchanged, not regressed).

Documented in:
- `spec.md` → "Release failure" Update note + OQ3 "Resolution" block (with scope change + AC27 rationale).
- `plan.md` → Planning Baseline bullet corrected with pointer to the spec resolution.
- New guard test `lockfiles_stay_gitignored_so_release_checkout_cannot_block` protects the premise against a future force-add. Contract suite now 10/10.

## Branch CI run — learnings (PR #2, branch `devops-phase-1-bootstrap-release`)

Three CI runs on the branch (`pull_request` event → full scope, since this PR changes global inputs):

**Validated working in real CI (the CI-only ACs I could not check locally):**
- `scope` job classifies the global change as `full` and emits a **3-OS preflight matrix** (AC33 mechanics).
- `preflight` passes on **ubuntu + windows + macos** — the cross-platform shell assumptions I flagged (`just check-canonical` bash/grep on Windows, `python3` on Windows, wrapper-free `cargo metadata`) all hold (AC1, AC34 mechanics).
- `needs: [scope, preflight]` gating works — area jobs start only after preflight is green.
- After fixes, **0 kache failures** and **0 infra failures**; remaining reds are pre-existing product/lint (`biscuit-speaks` unused var, `playa` lint) = Phase 2/3, exactly as the spec's Evidence predicted.

**Two bugs the branch run caught that ALL local checks (actionlint + text contract tests) missed:**
1. **Composite `if:` load error.** A step with both `if: ${{ inputs.kache }}` and `uses: <local composite>` fails at workflow-prep: *"Unrecognized named-value: 'inputs'"*. Broke every area job at the kache step. Fix: gate inside the composite via a declared `enabled` input; caller passes `enabled: ${{ ... }}`. (commit `3eaf5e50`)
2. **kache-action@v1 is Windows-incompatible.** `kunobi-ninja/kache-action@v1` errors *"Unsupported platform: win32-x64"*. Pre-existing (old workflow ran it on Windows unconditionally) but blocking for `soft_os: []` areas like `biscuit-file`. Fix: `enabled: ${{ inputs.kache && runner.os != 'Windows' }}` — kache is Linux/macOS-only; Windows builds without the cache (D1). (commit `73baba87`)

**Doc follow-up (Phase 2/4):** `.claude/skills/rust-devops/kache.md` says "Windows … Supported" — true for the kache *binary*, but the *kache-action@v1* rejects win32-x64. Worth a note when kache docs are updated.

Contract test updated both times so the text checks now encode the corrected patterns. Local suite: 10/10 Rust, 11/11 Python, actionlint clean.

## Phase 2 progress (in branch, incremental)

Decision confirmed with user (OQ1): pin **exactly `1.97.1`** (verified current latest stable via `rustup check`; CI already resolves it). Landing Phase 2 in safe, CI-verifiable increments rather than rewiring 12 workflows at once.

- [x] **Task 2.2 (core): controlled required toolchain + latest-stable advisory.**
  - `rust-toolchain.toml`: `channel = "stable"` → `"1.97.1"`; components `["clippy", "rustfmt"]`. Effective immediately — the toml wins over floating `@stable` defaults, so CI keeps building on 1.97.1 while local dev converges off 1.96.0. Verified locally: `rustup` auto-installed 1.97.1 and the contract suite compiled+passed under it.
  - New `.github/workflows/rust-latest-stable.yml`: scheduled (weekly) + manual advisory, overrides the pin via `RUSTUP_TOOLCHAIN=stable`, runs `cargo fmt --all --check` + representative area gates, non-required.
  - Contract tests: `required_ci_pins_an_exact_rust_toolchain`, `latest_stable_advisory_is_separate_and_non_required`. Suite now 12/12; actionlint clean.
- [x] **Task 2.2 (remainder): required path honors the file.** ci.yml + _area-ci.yml no longer carry `dtolnay/rust-toolchain@stable`; each toolchain step is `rustup show` (materializes the pinned 1.97.1 from rust-toolchain.toml). clippy+rustfmt come from the file; the coverage job adds `llvm-tools-preview` via `rustc --version` + `rustup component add`. Contract test `required_ci_honors_the_toolchain_file_without_stable_override`. (The other 10 specialized/scheduled workflows keep @stable for now — Phase 4 consolidation; the pin still wins for cargo there.) **NOTE:** local `actionlint` hangs on ci.yml via its shellcheck integration (concurrency/stdin deadlock) — native `actionlint -shellcheck=` passes and the run-scripts are shellcheck-clean; real proof is the branch run.
- [x] **Task 2.3: explicit CI nextest profile + no blanket retries.** `.config/nextest.toml [profile.ci] retries = 3 → 0` (kills the 4×-run hazard); scoped `retries = 2` overrides kept ONLY for `test(/level2_/)` and `test(/browser_/)` (documented resource contention); JUnit already emitted. `_area-ci.yml` sets `NEXTEST_PROFILE: ci` so every area recipe's `cargo nextest` selects+logs the profile (no recipe churn). Verification: `ci_profile_retry_policy_is_scoped_not_blanket` + `area_ci_selects_the_ci_nextest_profile_explicitly`. Suite 15/15; nextest parses the ci profile.
- [x] **Task 2.4: shard recalc from measured cold-run durations (run 30130238314, cold Linux, profile ci).**
  - **Measurements:** Claudine main suite logged `Starting 3964 tests` but aborted at `402/3964` in **162s** because `1 timed out` tripped fail-fast → extrapolated **~27 min unsharded**, right at the 30-min budget → shard it. Darkmatter's 4 shards ran **6.7–8.2 min each** → comfortably within budget → keep 4 (no evidence to change).
  - **Changes:** `areas.json` claudine `shards: ["1/4".."4/4"]` (≈7 min/shard); darkmatter unchanged. `_area-ci.yml` L1 step now `--no-fail-fast` (D6/D7 — a shard must run ALL its tests; the fail-fast is exactly what hid 3562 claudine tests). Per-tier JUnit upload with collision-free `junit-<area>-<gate>-<os>-<job-index>` names.
  - **Also confirmed from the same run:** `nextest profile: ci` is logged (Task 2.3 / AC15 ✅); 30/30 toolchain steps + all kache/preflight green (Task 2.2 ✅).
  - **Caveats (follow-ups, not Phase 2 blockers):** (a) recipes that run nextest once per package overwrite `test-results.xml`, so per-shard JUnit currently captures the last package — full aggregation is a follow-up; (b) the `1 timed out` claudine test is a real product/test defect (Phase 2/3 area-owned), now visible instead of masked.
- [x] **Task 2.5** staged `_area-ci.yml`: lint -> test (L1) -> optional l2/browser via `needs:`. A build/lint failure now skips the test tiers (fixes the biscuit-speaks lint+test redundancy); expensive L2/browser run only after L1. actionlint validates the DAG; contract test `area_test_tiers_are_staged_after_build_and_lint`.
- [x] **Task 2.6** docs updated (subagent, verified): docs/testing-strategy.md + rust-testing SKILL.md (hash regenerated) + rust-devops/kache.md — pinned toolchain, explicit ci profile/retries, shard evidence, staged gates, kache opt-in + Windows caveat. Fixed drifted `_area-ci.yml` header comment ("kache on every runner" -> Linux/macOS only).

## Phase 3 progress (in branch)

- **Task 3.1 scope:** Phase 3 touches the L2 terminal-backend harness (`biscuit-test-harness`), `_area-ci.yml` L2/browser provisioning, area `just test-l2` recipes, `areas.json` policy schema, `affected_scope.py` validation, and native-dep areas (Playa ALSA). A discovery agent is mapping the exact backend-selection + native-prereq mechanics (the first agent died in a power cut; re-launched).
- [x] **Task 3.2 (foundation): strict capability-policy schema.** `affected_scope.py` now validates every `areas.json` record via `validate_area_schema` (D10): required fields (`area`, `check_args`), unknown-field rejection, supported-runner-OS enforcement on `full_os`/`check_os`/`soft_os`/`native` keys, known-backend vocabulary (`tmux`/`wezterm`/`kitty`/`apple-terminal`), `native` OS→packages typing, and boolean-flag typing. New optional fields `backends`, `native`, `canary` added with empty defaults. Documented every field in new `.github/ci/README.md`. Tests: 18/18 (added 7 schema cases). Real `areas.json` validates clean.
- [x] **Task 3.2 (remainder):** declared `backends` (biscuit-terminal: tmux/wezterm/kitty/apple-terminal; darkmatter/claudine: tmux/wezterm) and Playa `native` ({ubuntu-latest: libasound2-dev, libpulse-dev}) in areas.json.
- [x] **Task 3.3/3.4 (C+, user-approved):** tmux is the ONLY CI-provisionable backend (WezTerm/Kitty/Apple need a live GUI session, impossible on headless runners). L2 job now provisions AND verifies tmux (`tmux -V` = the hard-require, D8) and drops the global `BISCUIT_TEST_LEVEL_REQUIRED=2` so GUI-backend tests skip cleanly instead of panicking. tmux is headless (no focus); L3 stays off. `backends` metadata documents each area's usage. Contract test `l2_provisions_and_verifies_only_the_ci_capable_backend`.
- [x] **Task 3.5** native prereqs: new `install-native` composite (apt/brew, named failure, no-op when empty) wired into all 5 _area-ci jobs before build/test; ci.yml passes `native`; Playa Linux ALSA/Pulse now provisioned; created playa/docs/dependencies.md. Contract test + shellcheck-clean. CI-caught bug: `` bash idiom yields `{}}` (jq parse error) breaking every job; fixed by defaulting before expansion (proven locally against the exact failure).
- [x] **Task 3.6** regression coverage satisfied: python schema tests (unknown backend, invalid OS, native typing, non-bool flag) + contract tests (L2 backend policy, native provisioning). 18 contract + 18 python green.

## Phase 4 progress (in branch)

- **Task 4.1 scope:** 72 workspace members; 52 owned by the 18 curated areas; **20 orphans** in 13 dirs (rendezvous/homelab-integrations are NOT orphans — they nest under claudine/homelab).
- [x] **Task 4.2: complete, unique workspace ownership (user-decided classification).**
  - **Promoted 3 CI-ready orphans to curated areas:** `renderable`, `worktree`, `biscuit-icon` (added its missing `bench` recipe). All have the full 12 canonical recipes (`check-canonical` green). biscuit-icon + worktree get `l2:true` + backends (tmux + wezterm/kitty).
  - **Exempted 15 packages** in new `.github/ci/exemptions.json` with reasons: test-infra crates (test-toolkit, biscuit-test-harness, biscuit-browser-harness), experimental (agent-sandbox-cli, tabby, ui), real-areas-pending-canonical-recipes (biscuit-visualized, messenger*), and not-fully-working (visualizer, biscuit-clipboard*, reaper*).
  - `affected_scope.py` gains `validate_ownership` (D10): every member owned-by-area OR exempt; fails by name on unmapped member, owned-and-exempt contradiction, or stale exemption. `load_exemptions` rejects duplicates/empty reasons. Documented in `.github/ci/README.md`.
  - Tests: **24/24 python** (6 new ownership cases). Real workspace validates clean.
  - **User note:** biscuit-visualized/messenger/biscuit-icon(done)/etc. "all eventually need inclusion"; the exemption reasons track the exact blocker (complete canonical recipes) so promotion is a tracked follow-up.
- [x] **Task 4.3** canaries: biscuit-hash (pure-Rust) + playa (native) + darkmatter (heavy) flagged `canary:true`; affected_scope emits `canaries`; ci.yml runs a canary stage first on full scope (canary_matrix) and gates the non-canary fan-out + specialized jobs on canary success/skip (a canary failure blocks fan-out). Contract test.
- [~] **Task 4.4 (in progress)** specialized-workflow consolidation. Pattern proven + orchestrated for 2 of 6: claudine-windows-ctrl-c (single-job) + rendezvous-tests (3-OS matrix) are now reusable (workflow_call), no longer self-trigger, and are called by ci.yml gated on scope (area_names output added). Remaining: biscuit-tui-captured-stdout, playa-windows, messenger-desktop (matrix+WSL+exempt-package gating), build-integrations (release-triggered - leave), failure-class summary (D15 pt2). Needs CI verification per orchestration.
<!-- 44progress -->
- [x] **Task 4.5 (scope part)** scope job writes an actionable `## CI scope` summary to $GITHUB_STEP_SUMMARY (event, change class, full-scope+reason, preflight OS, canaries, areas, package count, toolchain, kache). Failure-class summary is part of the 4.4 remainder.
- [x] **Task 4.6** stale counts fixed (subagent): CLAUDE.md/AGENTS.md 48->72 members; matrix-testing spec 17->21 areas + source-of-truth corrected to areas.json; rust/monorepos skills (hashes regenerated) + 2 more. Non-essential counts point to cargo metadata/areas.json.

## Files Changed

- `.cargo/config.toml` — **deleted** (removed mandatory global `rustc-wrapper = "kache"`; `.cargo/` now empty/gone). D1.
- `.github/kache-version` — **new**, single kache version authority (`0.8.0`). D2.
- `.github/actions/enable-kache/action.yml` — **new** composite: reads the pin, activates via `kache-action`, verifies the active version before any Cargo step (named bootstrap failure on mismatch/missing). D2.
- `justfile` — `KACHE_VERSION` now `trim(\`cat .github/kache-version\`)` instead of a literal. D2.
- `.github/workflows/_area-ci.yml` — five `Enable kache` blocks → `uses: ./.github/actions/enable-kache` (opt-in, verified, no duplicate literal). D1/D2.
- `scripts/ci/affected_scope.py` — hoisted `AREA_DEFAULTS`; added `.github/kache-version` (global path) + `.github/actions/` (global prefix); new `global_trigger()` + `classify_preflight()`; scope output now carries `change_class`, `preflight_os`, `preflight_reason`. D3.
- `.github/workflows/ci.yml` — scope job emits preflight outputs (moved canonical-recipe + scope-calc guards out of `scope`); new matrix `preflight` job (toolchain/tooling verify, wrapper-free `cargo metadata`, guards) whose breadth = `preflight_os`; `area-ci` + coverage + claudine-generator + darkmatter-no-color now `needs: [scope, preflight]`. D3.
- `.github/workflows/release-plz.yml` — trigger changed from `push: main` to `workflow_run` of `ci` (success + `head_branch == main` + repo guards); checks out the validated `head_sha`; clean-tracked-worktree assertions before/after release calc; documents gitignored-lockfile isolation (OQ3 Option C). D13.
- `scripts/ci/test_affected_scope.py` — +5 scope tests (windows-path, package-local OS, global 3-OS, kache-version full scope, doc-only host-only).
- `tools/test-toolkit/tests/phase7_ci_workflows.rs` — **deleted** (stale; read deleted per-area workflow files).
- `tools/test-toolkit/tests/ci_workflow_contracts.rs` — **new**, 9 durable contract tests for the current architecture.

## Notes / Lessons

- `phase7_ci_workflows.rs` was already broken before this work: it reads per-area workflow files (`test.yml`, `darkmatter-tests.yml`, `biscuit-file-tests.yml`, `claudine-tests.yml`) deleted when CI moved to the matrix-driven `ci.yml` + `_area-ci.yml`. Task 1.5's rename/rewrite fixes this stale test.
- kache/fan-out/release YAML behavior is asserted in the **Rust** contract test (it inspects YAML text); `test_affected_scope.py` covers only the pure `calculate_scope` function (Windows-path, package-local OS union, global 3-OS, doc-only). Allocation documented here since the plan listed them together.

## Session 2026-07-26/27 — native libs, 4.4 completion, canaries, Phase 5

Branch/PR map (stacked, each rebased onto the one below):

| PR | Branch | Content |
|---|---|---|
| #5 | `devops-native-libs` | one native-library installer, run before every build |
| #7 | `devops-phase-4-orchestration` | Task 4.4 remainder + failure-class summary |
| #8 | `devops-phase-5-scheduled` | Phase 5 scheduled automation |
| #9 | `devops-ci-commit` | CI-aware `just commit` |
| #6 | `devops-canary-set` | **merged** — canary set trimmed to green areas |

### Native libraries — one installer (spec "Native libraries", D9)

`_ensure-native-libs` now takes an optional area: `just _ensure-native-libs <area>`
scopes to one area (CI), no argument covers every area (`just init`, and the
affected-coverage job, which instruments the whole workspace). An unknown area
name fails loudly instead of silently installing nothing. The `install-native`
composite and the now-dead `native` workflow input are deleted — which also
retires the `${VAR:-{}}` jq-parse hazard the composite carried.

Every building job provisions before it builds: check, test, lint, l2, browser,
plus affected-coverage. Contract test `native_prerequisites_are_installed_before_
anything_is_built` splits each workflow's `jobs:` section and asserts the install
step precedes every build command; proven non-vacuous by reordering the lint job
and observing red.

**CI evidence (run 30230445139):** "Install native prerequisites" succeeded on
ubuntu-latest, macos-latest, AND windows-latest. The bash-shebang recipe runs
under Git Bash on Windows; `uname -s` there yields `MINGW64_NT-*`, which maps to
the `windows-latest` key, where no area declares packages, so it no-ops cleanly.

### Canaries must be green (merged, PR #6)

`darkmatter` dropped from the canary set; `biscuit-hash` + `playa` retained.
Directly observed on both sides:

- **Before** (PR #5, run 30230445139): all 8 `canary / darkmatter / test` shards
  failed and the entire fan-out was SKIPPED — no area result was visible at all.
  The darkmatter failure is pre-existing product/test debt: `1573 tests run:
  1567 passed, 6 timed out`.
- **After** (PR #6): canaries green, fan-out started (41 jobs).

Also confirmed: a `soft_os` leg failing does **not** block the canary gate.
`canary / playa / test (windows-latest)` failed and fan-out proceeded. That
failure is also pre-existing and NOT playa's: `biscuit-terminal` has three
Windows-only dead-code items (`parse_cpr_response`, `parse_csi_14t_response`,
`OSC_QUERY_ATTEMPT_TARGET`) that `RUSTFLAGS: -D warnings` turns into errors.
Owned by biscuit-terminal; spec non-goal here.

### Task 4.4 remainder + failure-class summary (D12/D15)

All five specialized runtime workflows are now reusable and orchestrated:
rendezvous, claudine-windows-ctrl-c, biscuit-tui-captured-stdout, playa-windows,
messenger-desktop. `build-integrations` stays release-triggered.

`messenger` is EXEMPT from area ownership so it never appears in `area_names`;
it is selected from the affected PACKAGE list instead, and its exemption reason
now records that this workflow IS its CI ownership.

Failure-class summary is split across two levels because a reusable workflow's
**matrix legs cannot report outputs back to the caller** (a matrix job's outputs
are last-writer-wins). So `_area-ci.yml` gains an `if: failure()` classifier that
writes its own gate-level line (build/lint/L1/L2/browser) to the shared run
summary, and `ci.yml` classifies the stage. Classification reads job RESULTS, not
log text, so a Node deprecation warning or cache collision can never outrank the
real root cause (D15 explicitly requires this).

Incidental fix: the messenger WSL job's last command was `cargo test -p
messenger-cli"` — a stray quote making it unparseable.

### Phase 5 — scheduled automation

`bench-nightly`: push trigger removed; execution separated from Bencher upload
(previously the whole step was `continue-on-error`, so a bench that failed to
COMPILE was invisible — the inverse of the AC31 hazard); budget 30 → 90 min.

**Measured evidence:** six consecutive warm scheduled runs took 14, 16, 17, 18,
18, 18 min (run 29971775038: ~6 min compile + ~9 min Criterion). Four runs were
cancelled at exactly 30 min — i.e. **the cold duration has never been observed,
only bounded below**. AC30 asks for a budget above the measured *cold* duration;
90 min is above the measured *warm* duration with margin and is explicitly
provisional. The job now records its own duration, runner image, and toolchain,
so the next cold run produces the number needed to tighten it (or to justify
splitting the 16 bench targets). **This is the one acceptance criterion not fully
satisfiable from existing evidence.**

`coverage` moved 04:00 → 05:00 (it collided with `sniff-performance`), honors the
pinned toolchain, provisions native libs. `fuzz-nightly` gained a summary job and
a note on why it deliberately overrides the pin. No duplicated L1 work was found
in either to remove (plan 5.3) — neither repeats area CI.

New `maintenance-audit.yml`: weekly, advisory, read-only permissions, always
succeeds. Reports upstream movement in Rust/kache/nextest/action versions/runner
image. The Rust lookup was verified locally against
`static.rust-lang.org/dist/channel-rust-stable.toml` (returns 1.97.1 = the pin).

### Docs drift corrected

`docs/topics/ci-cd.md` and `docs/testing-strategy.md` still described the
pre-Phase-1 world: floating `stable` toolchain, release-plz on push to main,
path-triggered specialized workflows, `dtolnay/rust-toolchain` as the house rule.
Corrected, plus a written "Advancing a pinned value" procedure to pair with the
maintenance audit.

### Learnings

- **A red canary hides everything.** Worth re-stating as an operational rule: the
  canary stage is a serial gate, so canary membership costs the whole fan-out's
  visibility when the area is not otherwise green.
- **Text contract tests need job-boundary awareness.** The first version of the
  before-build ordering test failed on an input's `description:` prose that
  merely *quoted* `cargo check --all-targets`. Splitting at the `jobs:` section
  fixed it — worth remembering for any future text-matching contract.
- **`gh run list` + `gh api .../jobs/<id>` is the cheapest way to get real timing
  evidence** for budget decisions, and it distinguishes "slow" from "truncated".

### Failure-set baseline (measured 2026-07-27) — read this before judging a CI run

A full-scope run cannot be green, and could not before this work either. Judge a
CI branch by **diffing** its failure set against a baseline, never by counting
reds:

```bash
gh pr checks <pr> --json name,state \
  | jq -r '.[]|select(.state=="FAILURE")|.name' | sed 's/^canary \/ //' | sort -u
comm -13 baseline.txt candidate.txt   # only NEW failures
```

Baseline = PR #6 (canary change only, no `_area-ci.yml` edits): **31 failures.**
Lint: biscuit-speaks, biscuit-terminal, claudine, homelab, research, tree-hugger,
worktree. Tests: darkmatter (8 shards), sniff (3 OSes), schematic (2), unchained-ai
(2), and Windows legs of biscuit-file, biscuit-icon, biscuit-tui, model-citizen,
queue, renderable. Plus claudine-generator drift and affected coverage.

Measured deltas:

- **PR #5 (native libs): 0 new failures.** The one-installer change broke nothing.
- **PRs #7/#8/#9: exactly 3 new**, all inherited down the stack from #7:
  `rendezvous / native (windows-latest)`, `claudine-windows-ctrl-c`, and
  `messenger-desktop / WSL2 ubuntu`.

Those three are **pre-existing failures newly made visible**, not regressions:

- `gh run list --workflow=<wf> --branch main` shows all three red on `main`
  continuously — claudine-windows-ctrl-c since 2026-06-25, messenger-desktop
  since 2026-06-17, rendezvous since 2026-07-23.
- Their failing steps are product/environment steps this work never touched: the
  Ctrl+C test itself, the Rendezvous suite, and `Vampire/setup-wsl@v4`. Every
  step the 4.4 conversion DID change — `rustup show`, `Install just`, `Install
  native prerequisites`, protoc, cache, compile-check — passed.

They were previously invisible because each self-triggered only on its own path
filter, so a change to `.github/workflows/**` never selected them. Orchestration
(D12) now selects them from affected scope, which is the intended behavior and
also means **they now gate merges**. Each belongs to its owning area.

Two visibility unlocks in one session therefore account for the entire apparent
explosion of red: the canary fix let the fan-out run at all, and orchestration
pulled in three more workflows. This is G1 working, not a regression.

### Failure-class diagnostics verified on a real runner (PR #7)

- `Failure-class summary` in `ci.yml`: **success** — hyphenated `needs.<job>.result`
  references (`needs.area-ci`, `needs.claudine-generator-signals`, …) resolve
  correctly in GitHub expressions.
- Per-area `failure class` jobs behaved as designed: **success** (i.e. ran and
  classified) for areas with a hard failure, **skipped** for areas that passed —
  and also skipped for areas whose only failure was a `soft_os`
  `continue-on-error` leg (biscuit-icon, biscuit-tui, model-citizen, queue,
  renderable), because `failure()` does not fire for a tolerated leg. Correct: an
  advisory leg is not an actionable failure class.

### Scheduled-workflow shell blocks were dry-run locally

Scheduled workflows get no run from a PR, and `maintenance-audit.yml` cannot even
be dispatched until it is on `main`. Extracting and executing their `run:` blocks
locally caught three defects that would have produced silently wrong output on an
unwatched nightly run:

1. `cargo nextest --version` prints a multi-line build block, so `awk '{print $2}'`
   emitted one value per line and broke the audit's markdown table row.
2. `grep -m1` closes curl's pipe early, which curl reports as exit 56 — making the
   step's exit status meaningless.
3. A benchmark that failed before recording its duration printed a bare `"s"`.

Worth repeating for any future scheduled workflow: **extract the `run:` blocks and
execute them with `GITHUB_STEP_SUMMARY` pointed at a temp file** before merging.

## Validation checkpoints 4 and 5 closed (2026-07-28)

Both checkpoints were satisfied after #7, #8, #9, and #13 landed on `main`.

### Checkpoint 4 — scope and orchestration

- `python3 scripts/ci/test_affected_scope.py` — 26 tests, all passing.
- `cargo nextest run -p test-toolkit --test ci_workflow_contracts` — 28 passing.
- `just check-canonical` — 21 areas, 0 failures.
- Named scenarios map to fixtures: package-local
  (`test_package_local_change_derives_area_preflight_os`), shared dependency
  (`test_shared_test_change_includes_consuming_areas`), documentation-only
  (`test_unrelated_documentation_change_has_empty_scope`), unmapped package
  (`test_package_under_no_declared_area_fails_by_name`), invalid policy (the
  six `*_is_rejected` cases). Global canary success and failure were observed
  live rather than as fixtures: the canary gates fan-out on every global-scope
  run in this session.
- **Duplicate ownership is no longer a reachable state.** `owner_area` maps a
  manifest's top-level directory to one declared area or `None`, and a package
  lives in exactly one directory, so nothing can be claimed twice. The old
  `areas + exemptions` pair could double-claim a package; #13 removed the
  class rather than adding a test for it. This checkpoint predates #13.
- Specialized contracts run only when selected: on the workflow-only PRs #14 and
  #15 every specialized job reported `SKIPPED`, while full-scope runs selected
  and ran them.

### Checkpoint 5 — independent scheduled signals

All four dispatched from `main`:

| workflow | run | result |
|---|---|---|
| `maintenance-audit` | 30315637796 | success |
| `bench-nightly` | 30318777650 | success, 34.7 min |
| `fuzz-nightly` | 30318779675 | success |
| `coverage` | 30318778635 | failure — product test, infrastructure green |

Each reports under a distinct name with its own summary and artifacts, and
`coverage`'s failure did not affect the other three.

The "successful benchmark, failed upload" requirement is held by contract test
`benchmark_upload_failure_cannot_erase_a_successful_measurement` (AC31), which
is stronger than a one-off simulation: it asserts execution precedes upload,
that `continue-on-error: true` is bound to the upload step alone, that benchmark
*execution* is not `continue-on-error`, and that the upload is gated on
`steps.bench.outcome == 'success'`.

### AC30 resolved — the cold bench duration has now been observed

The first cold `bench-nightly` run took **34.7 min against the 90-min budget**.
That also explains the truncations: every earlier cold run died under the old
30-min ceiling, which 34.7 min exceeds. Warm runs measured 14–18 min. The budget
has ~2.6x headroom on a cold run and can be tightened without splitting the 16
bench targets.

### A local dry-run is necessary but not sufficient

The three defects caught by dry-running the scheduled `run:` blocks (above) were
real, but two more survived that method and only a live dispatch found them:

1. **#14** — `grep -m1` closed the pipe and `printf` took an EPIPE, fatal under
   GitHub's `bash -e -o pipefail`. The dry-run missed it because the earlier
   `curl` fix had been read as closing the whole class; the early-closing reader
   simply moved to the next writer.
2. **#15** — `cargo nextest --version` exits 101 on the audit runner, which
   installs only the toolchain. Every local check passed because a developer
   host has cargo-nextest installed.

Both had to wait for #8 to merge: a new scheduled workflow gets no PR run and
cannot be dispatched until it is on `main`. The third dispatch was green end to
end, including `Report third-party action versions in use`, which had never
executed on a runner before.

When dry-running a scheduled block, also run it with the tool under test removed
from `PATH` — that is the runner's real state.

### A3 fixed: workspace coverage can build again

`_ensure-native-libs` was left half-migrated when `exemptions.json` was deleted:
the unscoped branch still passed the now-undefined `$exemptions_json` to `jq`, so
under `set -u` the assignment aborted and `declared` came back empty. `just init`
and the workspace coverage job silently installed nothing on Linux.

**A baseline failure-set diff could not have caught this.** The only job that
exercises the unscoped path is "Coverage for affected packages", already red for
an unrelated reason, so #13's clean zero-new-failures delta was consistent with
the bug. Two red jobs can hide each other; a delta of zero proves no *new* job
broke, not that a changed code path works.

After the fix, `gdk-sys` compiles on the coverage runner instead of failing its
build script, which closes root cause A3 in `ci-failure-inventory.md`. Coverage
now fails much later on genuine product tests.

### Coverage runs L2 tests that `just test` excludes

Both coverage runs failed on `level2_*` targets (`biscuit-icon-cli --test
level2_terminal`, and a `biscuit-clipboard-service` failure in the ci.yml job).
`cargo llvm-cov --workspace` invokes `cargo test --tests --workspace` directly,
bypassing the nextest filterset the `just test` recipes use to exclude `level2_`.
So real-terminal tests run headless under coverage. Not addressed here; it
belongs with the L2-on-Linux work in #16.
