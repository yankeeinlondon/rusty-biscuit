---
plan_file: features/2026-07-24-devops/plan.md
phases: 5
status: "phase 1 complete (local gates green; CI-only ACs pending a branch run)"
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
