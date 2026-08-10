---
title: Per-package test resolution — review 1
status: findings implemented
created: 2026-08-06
spec: fixes/2026-08-06-cicd/spec.md
plan: fixes/2026-08-06-cicd/plan.md
implementation: fixes/2026-08-06-cicd/implementation-notes.md
reviewed_commit: e844fc2e9
---

# Review 1 — implementation of per-package test resolution

## Verdict

The implementation is faithful to the spec and high quality where it counts:
selection semantics, the reverse-dependency closure, the native union, the
result schema, the state machine, and the baseline re-key all verified correct
in detail (see "Verified correct" at the end). The developer's summary claims
held up under adversarial checking almost everywhere.

It is **not ready for the cutover merge**. Four blockers exist, three of which
the pending bridging run would surface only by burning a full cycle (and one of
which it cannot surface at all, because the guard that should catch it is
structurally blind). Fix B1–B4 before the bridging run, not after it.

Findings are numbered; severity is blocker / major / minor. File references
are to the reviewed commit `e844fc2e9`.

---

## Blockers

### B1 — `claudine-gen` owns three real L2 tests and declares no L2 tier

`claudine/gen/tests/level2_report_terminal.rs:240,300` contains
`require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux)` tests
(plus `level2_report_probe` at :109), and the package's dev-deps are even
commented "Level-2 real-terminal report tests." `claudine/gen/Cargo.toml` has
**no** `[package.metadata.ci]` block, so the package defaults to L1-only. The
anchored L1 filter excludes `level2_*`, and no L2 job selects the package —
these tests run in **no tier**, in CI or locally (the claudine area `test-l2`
runs only `claudine-cli`).

This is exactly the situation AC12 exists to reject, one package over from the
`dmls` case the implementation did catch and fix. Violates R4, R7, AC3, AC12.

**Fix:** declare `tiers = ["L1", "L2"]` + `l2-backends = ["tmux"]` on
`claudine-gen` — after fixing B2, which is why this one slipped through.

### B2 — the AC12 undeclared-tier guard is structurally blind to packages without a metadata block

`tools/test-toolkit/tests/ci_workflow_contracts.rs:98`:
`let ci = package["metadata"]["ci"].as_object()?;` inside a `filter_map` drops
every package that has no `[package.metadata.ci]` block from enumeration. Only
28 of 72 members have one, so `an_undeclared_l2_tier_is_rejected` (:1764)
never inspects the other 44 — which are precisely the default-policy packages
the guard exists for. `dmls` was caught only because Phase 1 had already given
it a block; `claudine-gen` (B1) was not. A default-policy package can add
`level2_*` tests today and all 53 contract tests stay green while the tests
run nowhere.

Two adjacent holes in the same guard:

- **No browser-direction rejection exists at all.** Only
  declared-but-empty is covered for browser
  (`every_declared_browser_tier_is_non_vacuous`, :1744); there is no
  `an_undeclared_browser_tier_is_rejected`. AC12 covers both non-default
  tiers.
- **`nextest_test_count` fails open** (:1679–1693): any spawn failure or
  non-zero `cargo nextest list` returns 0, which makes the *rejection* test
  pass vacuously in a degraded environment while the *non-vacuity* tests fail
  spuriously — the opposite of the inline "skip rather than fail" comment.

Violates R11's non-vacuity requirement in effect. **Fix:** enumerate all
workspace members with an empty-object default policy; add the browser
rejection test; make listing failure loud (or an explicit skip), never a
silent 0.

### B3 — the WSL guest step runs `jq` before anything installs it

`.github/workflows/_wsl-ci.yml:395-404`: "Install native prerequisites in the
guest" runs inside the guest under `set -euo pipefail` and unconditionally
pipes `$NATIVE` through `jq` — for every package, even one with an empty
native list. The guest is provisioned with only
`additional-packages: ca-certificates curl git xz-utils` (:290, :302). The old
path survived because the guest ran `just _ensure-native-libs <area>`, whose
recipe detected a missing `jq` and apt-installed it inside the guest; the new
recipe only bootstraps jq in its **no-arg** branch, which this step never
uses. If jq is absent from the guest image — and the old recipe's guest-side
jq-install branch is strong evidence it is — **all ~63 wsl2-ubuntu legs die
before running a single test**, and the bridging run is wasted.

**Fix (either):** add `jq` to `additional-packages`, or move the JSON parse
out of the guest (parse on the host, pass a plain list).

### B4 — `l1-include-slow` is not forwarded to the WSL leg

`_package-ci.yml:732-747` passes only `package`, `check-args`, `test-args`,
`native` to `_wsl-ci.yml`; `_wsl-ci.yml` declares no `l1-include-slow` input
and its guest L1 step never exports `BISCUIT_L1_INCLUDE_SLOW` (the native legs
do, `_package-ci.yml:227`). `darkmatter` and `darkmatter-cli` declare
`l1-include-slow = true`, and the old guest ran the area recipe whose
`BISCUIT_TEST_FILTER` kept `slow_`. Result: darkmatter's L1 contract now
differs by environment — `slow_` tests silently dropped on wsl2-ubuntu only.

This is the exact "silently narrows the suite" failure mode the plan's
sharding-removal risk names, and it undermines the re-keyed darkmatter wsl2
baseline entries (whose failure counts were recorded with slow tests
included) — the bridging run would produce confusing baseline mismatches
rather than a clean signal. Violates spec §survivors ("forwarded …
consistently"), R4, AC13.

**Fix:** add the input to `_wsl-ci.yml`, forward it, export it in the guest
L1 step.

### B5 — a change touching only `gates = false` packages crashes the run (empty matrix)

`.github/workflows/ci.yml:116` computes
`has_packages = jq '.packages | length > 0'` from the **impacted** list, which
includes non-gating packages; `affected_scope.py:922-924` excludes them from
`.matrix`. A change touching only, e.g., `messenger/cli/` or
`biscuit-clipboard/service/` (both `gates = false`, zero dependents) yields
`has_packages == 'true'` with `package_matrix == {"include":[]}` — GitHub
fails the `package-ci` job at strategy evaluation. The old workflow guarded
this exact case by deriving the flag from the matrix
(old ci.yml:104); the guard was lost in the rewrite, and no contract test
covers it.

**Fix:** compute `has_packages` from `.matrix | length > 0` (and consider a
contract test asserting the gate variable derives from the matrix).

---

## Majors

### M1 — a *skipped* companion suite is invisible; only a *failed* one downgrades

Spec (R12 / §survivors): "a green Rust JUnit report must not hide a failed
**or skipped** companion suite." The failed path is solid end-to-end (step
failure → job failure → `status.json` detail → rollup downgrade at
`ci-rollup.rs:1309-1326`, test-proven). The skipped path is not: the frontend
step's `if:` gates on `node-environments` (`_package-ci.yml:338`), so a
scope-derivation bug or a capability flip makes the step outcome `skipped`,
`status.json` records success, and the cell renders PASS. The rollup
*structurally cannot* catch this because `policy_record`
(`affected_scope.py:872-882`) omits `companion-suites` from the rollup-facing
policy — the rollup never learns the package declares one. All existing
mitigations are static YAML/JSON pins; `BISCUIT_FRONTEND_REQUIRED` is `''` in
exactly the skipped case.

**Fix:** include `companion-suites` in the rollup policy and treat a declared
companion with no evidence as MISSING/downgrade (mirroring how MISSING already
works for producers).

### M2 — browser tier disappears from the grid instead of rendering POLICY GAP

`scripts/ci-rollup.rs:634-641`: for `Tier::Browser`, a non-capable environment
`continue`s — the expected cell is never created, so a browser-owning package
shows no browser cell on windows/macos/wsl2. The spec's letter: "an
unsupported required tier becomes an explicit `POLICY GAP` rather than
disappearing from the result grid." Consistent with the implementation's
choice, `environments.json` records `headless_browser: false` as plain,
ungoverned false — so if gap cells *were* rendered they would hard-block
(ungoverned absence is never excused).

This is a deliberate, commented design deviation (browser treated as a
Linux-only tier by definition, mirroring pre-existing behavior) — but it
contradicts the ratified spec and is inconsistent with how tmux absence is
governed. **Needs Ken's ruling:** either govern the browser absences in
`environments.json` and render the gap cells, or amend the spec to define the
browser tier as Linux-hosted and record why.

### M3 — the cutover is not atomic in history: `_area-ci.yml` was deleted in an image commit

Commit `c52eea5b3` ("chore(claudine): improved claudine standing image")
deletes `.github/workflows/_area-ci.yml` (679 lines) alongside two image
assets. At that commit, `ci.yml` still referenced `_area-ci.yml` — a broken
intermediate tree, and the cutover commit's message claims a rename that
actually happened elsewhere. Violates spec §Migration atomicity and commit
hygiene. The branch is unmerged: **fix history** (move the deletion into
`e844fc2e9`, or plan a squash-merge and say so).

### M4 — the guards guard nothing in CI: rollup and contract suites run in no CI job

Two compounding pre-existing gaps this rewrite resurfaced and should have
caught:

- A change to `scripts/ci-rollup.rs` — the merge-gate binary — classifies as
  `documentation` (`scripts/**` is not in `GLOBAL_PATHS`, and scripts are not
  packages), schedules zero packages, and no workflow runs the rollup's 114
  tests. A `ci-baseline.toml` edit similarly exercises nothing.
- Because `tools/test-toolkit` is `gates = false`
  (promotion-pending, expiry 2026-10-31), the entire R11 contract suite —
  including the AC12 tier contracts — runs in **no CI job**. The guards fire
  only when a developer runs them locally.

**Fix (minimum):** add `scripts/ci-rollup*.rs`, `scripts/ci/`, and
`.github/ci/` to the scope triggers with a leg that runs the rollup tests and
scope tests; treat test-toolkit's promotion as the durable fix for the
contract suite.

### M5 — declared `l2-backends` are advisory: `BISCUIT_TEST_REQUIRED_BACKENDS` is never set

No workflow sets `BISCUIT_TEST_REQUIRED_BACKENDS`, so the `backend-proof`
reset/verify bracket in `_test_l2` is inert in CI: an installed tmux with zero
executed L2 tests renders a green cell (the availability-is-not-execution
problem). Pre-existing parity — the old CI never set it either — but the
per-package conversion removed the old blocker (per-package `reset` erasing
sibling evidence no longer applies, since each L2 job runs one package), so
this was the natural moment to wire `BISCUIT_TEST_REQUIRED_BACKENDS=tmux`
(declared ∩ provisioned). Worse, `rust-testing/SKILL.md:93-96` and
`tools/test-toolkit/README.md:118-121` — both edited in this commit — still
claim "CI passes it through unmodified," which is false.

### M6 — AC4's no-reader guard checks a hardcoded 7-file list

`ci_workflow_contracts.rs:1601-1623` asserts `areas.json` absence plus no
mention in exactly 7 named files. It does not glob `.github/workflows/*`,
`scripts/*`, or area justfiles — and `homelab/justfile` carried `areas.json`
text until this very commit while not being on the list. The repo is clean
today (verified by repo-wide grep), but the guard does not keep it clean; a
reader added to `coverage.yml` passes. Contrast `no_workflow_passes_partition`,
which globs correctly. **Fix:** glob workflows + justfiles + scripts.

### M7 — shipped documentation still teaches the retired system

Both files were edited in this same commit yet contradict it:

- `.claude/skills/rust-testing/SKILL.md:205-216` still says "Heavy areas shard
  their L1 run via nextest `--partition count:i/N` … (darkmatter and claudine
  both use 4 shards)" and "within each area" — directly contradicts R7a/AC11.
  Its frontmatter `hash:`/`last_updated` were also not regenerated (repo rule:
  `md hash` after skill edits).
- `docs/testing-strategy.md:202-247` still describes the area fan-out, "check
  (macOS compile)" (check now runs on windows-latest,
  `affected_scope.py:92`), "L1 shards", "macOS is compile-checked, not
  full-tested" (macOS now runs full L1), and `BISCUIT_TEST_LEVEL_REQUIRED=2`
  on the Linux leg (a contract test now asserts CI must NOT set it).

Violates the repo Drift Maintenance rule. Fix in the cutover branch, not
later.

---

## Minors

1. **Schema-default bypass** — `ci-rollup.rs:1451-1461`: absent
   `schema_version` defaults to the *current* version, so a version-less file
   skips the migration-error path (dies as a parse error today; silently
   self-upgrades on the next bump). Make the field required.
2. **v2 baseline entries accept stale keys** — `FailureEntry`
   (`ci-rollup.rs:1464-1474`) lacks `deny_unknown_fields`; an entry carrying
   `shard = "1/4"` or `area = …` parses cleanly. AC11's spirit wants rejection.
3. **`gates = false` without a parseable exclusion renders nothing** —
   `ci-rollup.rs:599-618` skips the package entirely instead of NOT SCHEDULED.
   Unreachable from a valid scope artifact (upstream validation), but the
   rollup has no backstop and no test.
4. **`estimate_jobs` undercounts** — counts `wsl` as 1 job; `_wsl-ci.yml`
   spawns 2 (archive + guest). Full-scope ≈460 actual vs 402 recorded; also
   the docstring says "exact." The enforced limit (`len(matrix) ≤ 256`) is the
   right one and holds.
5. **Cache quota pressure** — ~5 keys × 63 packages ≈ 315 caches vs GitHub's
   10 GB repo quota → eviction thrash on full scope. Phase 6 must actually
   measure full-scope hit rates before closing the key decision (the L2/browser
   reuse-the-test-key design is good).
6. **`messenger-desktop` selection misses `messenger-cli`-only changes** —
   `ci.yml:384` uses exact-member `contains(...,'messenger')`; a cli-only
   change doesn't trigger the only workflow that covers messenger (and per B5
   the run errors instead). Pre-existing, now compounding.
7. **Native-union/feature coupling is latent** — the closure comes from the
   workspace-unified `cargo metadata` resolve, not the job's declared
   features. Correct today only because every optional edge (e.g.
   biscuit-speaks→playa) is enabled by some other member's hard dep; if that
   enabling edge disappears, the union silently loses packages. Nothing
   guards this.
8. **`biscuit-visualized`'s `--all-features` test contract not recorded** —
   its justfile documents 56 feature-gated tests; the manifest records only
   the exclusion. On promotion it silently drops them (messenger, in the same
   position, recorded its policy explicitly).
9. **Recycled area-level exclusion reasons** — biscuit-clipboard's three
   records share one verbatim "201 tests … across three packages" reason;
   AC15 wants package-specific justification.
10. **Companion-recipe existence check is a substring test**
    (`affected_scope.py:429`) — `test-frontend-watch:` would satisfy it.
11. **`l2_environments` derives solely from the `tmux` capability**
    (`affected_scope.py:830-833`); `kitty`/`apple-terminal` have no
    environment axis. Same hardcoding in the rollup's expected-cells
    (`ci-rollup.rs:623-632`).
12. **scope.json native array order is PYTHONHASHSEED-unstable** (set
    iteration, `affected_scope.py:932`) — nothing consumes the order, but
    byte-diffs will be noisy.
13. **Baseline header drift** — `.github/ci/ci-baseline.toml:293-295` header
    says "claudine **library**" over a `claudine-cli` entry (entry is
    correct).
14. **Stale area/shard prose in shipped code** — `ci-rollup.rs:1355`
    (`{area, environment, tier}` doc), `:2002` (user-facing "An area that
    produced nothing"), `:1544` (`area:` param name);
    `just/devops.just:800-819` (`_expected_manifest` comments: "N/A",
    "shards"). Also `darkmatter/dmls/Cargo.toml`'s ci comment describes a
    pre-HEAD justfile state, and `memory/just.md:106-110` lost its causal
    sentence mid-edit.
15. **README simplifications** — `.github/ci/README.md` states one cache-key
    scheme (actual: check/lint differ, L2/browser reuse the test key —
    deliberate and good, but undocumented) and lists a `wsl2` producer (job is
    `wsl`).
16. **md-fixture probe misses `md.exe` on Windows**
    (`_package-ci.yml:290`) → rebuilds every Windows run. Inherited verbatim
    from `_ensure-md`; equivalence preserved, wart included.

---

## Test-coverage gaps worth closing

The suites are genuinely behavioral (35 scope + 114 rollup + 53 contract, all
verified passing), but the untested branches cluster exactly where silent
mis-scoping would live:

- **Scope:** the `gates = false` path (no test asserts excluded-from-matrix +
  present-in-policy); non-propagation of tiers/tools/companions from
  dependencies; dev-dep edge rules of `build_closure`; the `Cargo.lock`
  branches through `calculate_scope`; the top-level-directory fallback;
  `MATRIX_LIMIT` overflow; `estimate_jobs` (untested entirely).
- **Rollup:** `status_cells` mapping (entirely untested); a `tier = "lint"`
  baseline entry excusing a lint FAIL (the path the synthetic entries will
  eventually ride); pinning `claudine-gen-drift`/`coverage` to
  `baseline-out-of-scope` by name; `NotScheduled` missing from the
  blocking-states rstest; stray-key rejection (minor 2); version-less baseline
  (minor 1).
- **Contracts:** everything under B2; a test that `has_packages` derives from
  the matrix (B5); comment-satisfiable `contains()` assertions — the
  companion-downgrade check matches any line containing `COMPANION` and
  `failure`, which a YAML comment satisfies.

---

## Needs Ken's ruling

1. **M2** — browser tier: govern + render POLICY GAP per spec, or amend the
   spec to a Linux-hosted browser tier.
2. **M5** — wire `BISCUIT_TEST_REQUIRED_BACKENDS` on the per-package L2 legs
   now, or explicitly defer with the skill/README text corrected.
3. **`l2-parallel-self-spawn`** — the runner-tool vocabulary grew a fifth
   entry beyond the spec's initial four. Justified (preserves claudine-cli's
   measured parallel L2 mode); ratify it into the spec's list.
4. **WSL surface** — `wsl: true` for every gating package means full scope
   provisions ~63 WSL legs (each 2 jobs). Consistent with R1/R6, but confirm
   the cost was priced; Phase 6's billed-minutes measurement will show it.

---

## Verified correct (condensed)

Checked in detail and confirmed, beyond the developer's claims:

- **Selection/closure (R1, R2):** transitive reverse-dep walk, seeds included;
  live run reproduces the exact R2 closure; deepest-package path mapping;
  `Cargo.lock` scoped from its own diff with fail-open widening.
- **Native union (R5):** forward build-closure walk (seed dev-deps in,
  transitive dev-deps out), union of `native` only; live-verified
  biscuit-speaks → espeak + playa's ALSA/PulseAudio; direction is test-proven.
  Non-propagation of tiers/tools/companions confirmed in code.
- **Validation:** all eight spec-listed rules present, loud, and each with a
  firing test; sensible beyond-spec extras (expiry classes, runner-label
  checks, archive-only hosting).
- **Manifests:** 28 blocks, schema vocabulary exact; native 4/4, backends 8/8
  grep-verified against real `level2_` files (no false declarations);
  exclusion audit per AC15 (time-bounded → gating, visualizer keeps native);
  feature contracts migrated incl. justfile-only facts (sniff-cli
  `test-fixtures`); claudine stub/fixture narrowing to claudine-cli verified
  at source level.
- **Workflows (R1, R9):** one package per result-producing job across name,
  `-p`, artifacts, status; canonical recipes exactly as specced; `_sanity`
  absent; feature args single-sourced to check/archive/test; runner tools
  byte-equivalent to the old steps; companion failure path end-to-end;
  sharding gone everywhere; artifact names unique; matrix/nesting within
  platform limits; actionlint clean.
- **Rollup (R3, R10):** CellKey through all six paths; schema v2 stamped once,
  v1 and future versions rejected both directions with explicit migration
  errors; NOTHING TO RUN vs NOT SCHEDULED vs MISSING distinct with only
  MISSING blocking; empty-JUnit reachability empirically confirmed with
  `--no-tests=pass`; manifest-only identity (no-name-parse test-proven).
- **Baseline:** old 32 = 13 shard-keyed + 2 synthetic + 17; new 24 = 32 − 13
  + 5 collapses, mapped entry-by-entry with nothing dropped; the darkmatter
  wsl2 2/4 → library split is real; synthetic entries carried un-renamed,
  out-of-scope, neither blocking nor passing; all governance fields present
  and load-tested.
- **POLICY GAP (L2):** governed absence renders a gap with owner/reason/expiry
  travelling in results; ungoverned false hard-blocks; expiry validated.
- **Local behavior (R8/AC9):** `test`/`_test`/`_test_all`/`_test_l2` and
  `BISCUIT_*` handling untouched; `BISCUIT_L1_INCLUDE_SLOW` is opt-in (unset =
  identical filter); homelab's local `test` still runs the frontend.
- **Dev-notes accuracy:** claims verified except the "exact" job count
  (minor 4) and the WSL slow-forwarding (B4); the notes are silent on
  B1/B3/B5.

## Reviewer's note on process

The five findings B1–B5 share a signature: each lives at a seam between two
components (manifest↔guard, host↔guest, caller↔reusable workflow, scope↔matrix
gate) that no single test file owns. The bridging run remains mandatory and
would have caught B3/B4/B5 at the cost of a cycle — but B1/B2 it could never
catch, because the missing tests are the very evidence the run would be judged
by. Worth a contract-test pass focused on seams before the bridging run.
