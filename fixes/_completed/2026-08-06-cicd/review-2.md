---
title: Per-package test resolution — review 2
status: findings delivered
created: 2026-08-07
spec: fixes/2026-08-06-cicd/spec.md
plan: fixes/2026-08-06-cicd/plan.md
implementation: fixes/2026-08-06-cicd/implementation-notes.md
previous: fixes/2026-08-06-cicd/review-1.md
reviewed_state: working tree on top of e844fc2e9 (fix round, ~1,600 insertions, 24 files)
---

# Review 2 — verification of the review-1 fixes

## Verdict

**All 28 review-1 findings (5 blockers, 7 majors, 16 minors) are fixed and
verified**, along with every named test-coverage gap. The fixes are correct at
the level review-1 worried about — I specifically checked the failure modes a
plausible-but-wrong fix would have introduced, and found none (details below).
No new blockers or majors were introduced by the fix round.

The branch is **ready for the Phase 4 bridging run**. What remains is what was
always gated on CI: the bridging run to verify the candidate baseline, and
Phase 6 measurement (now explicitly including full-scope cache-hit rates and
the WSL surface's billed minutes). Four decisions the developer made are
flagged for ratification at the end.

Every disposition below was verified against the actual diff, not the
developer's notes; the suites were re-run locally.

---

## Blockers — all fixed

**B1 (claudine-gen undeclared L2) — FIXED.**
`claudine/gen/Cargo.toml` declares `tiers = ["L1", "L2"]`,
`l2-backends = ["tmux"]`, with a comment naming the owned tests. The claudine
area's local `test-l2` now runs `_test_l2 claudine-gen` after the parallel
claudine-cli pass — deliberately on the serial path, since these tests are
`#[serial]`-chained (correct call; the parallel self-spawn mode is claudine-cli
policy, and claudine-gen declares no runner tools so CI also takes the serial
default). **Verified end-to-end:** `cargo nextest list -p claudine-gen` with
the canonical L2 filter lists exactly the 3 `level2_report_*` tests —
the declared tier is non-vacuous.

**B2 (guard blind to default-policy packages) — FIXED.**
`package_policies()` now maps every workspace member, synthesizing an
empty-object default record when no metadata block exists (with a comment
naming this exact failure). `an_undeclared_browser_tier_is_rejected` added,
symmetric with L2. `nextest_test_count` now panics on spawn failure and
asserts on listing failure — a degraded environment fails loudly instead of
converting the rejection guard into a no-op. The nextest slow-timeout
overrides were widened accordingly (60s × 20) in **both** the default and ci
profiles, since the rejection sweeps now build every gating package's test
binaries.

**B3 (WSL guest jq) — FIXED.** `jq` added to `additional-packages` on both
provision attempts, with a comment stating why it is not optional.

**B4 (l1-include-slow not forwarded to WSL) — FIXED.** Input declared in both
`workflow_call` and `workflow_dispatch` blocks of `_wsl-ci.yml`, forwarded
from `_package-ci.yml:771`, exported inside the guest L1 step. Darkmatter's
L1 contract is now environment-uniform.

**B5 (empty-matrix crash) — FIXED.** `has_packages` derives from
`.matrix | length > 0` with a comment explaining the gates-false-only case;
the old derivation is additionally *forbidden* by the new contract test
`the_fan_out_gate_derives_from_the_matrix_not_the_impacted_list`, which
asserts both the presence of the new expression and the absence of the old.

## Majors — all fixed

**M1 (skipped companion invisible) — FIXED, and correctly scoped.**
`companion_suites` now travels in the rollup-facing policy
(`policy_record`); the producer status records the companion step outcome
(`success`/`failure`/`skipped`, defaulting to `skipped` when empty) on every
run of a declaring package; the rollup downgrades a green-reading cell
(Pass/NothingToRun/Skip) whose declared companion produced no success
evidence. **The false-downgrade hazard is handled:** expected cells carry
companion expectations only for the L1 tier on **node-capable** environments
(`expected_cells`), so green windows/macos L1 cells — where the node steps
are rightly skipped — are not downgraded. The lint-cell variant lacks that
environment scoping but is safe because the lint job is pinned to
ubuntu-latest (nit N1 below). Both directions test-proven
(`a_skipped_companion_downgrades_a_green_report`,
`a_companion_with_no_reported_outcome_downgrades_a_green_report`,
`a_successful_companion_keeps_the_cell_green`,
`companion_suites_are_expected_only_on_node_capable_environments`, plus the
lint variant).

**M2 (browser tier vanished) — FIXED per the spec's letter (ruling 1).**
The three `headless_browser` absences in `environments.json` are now governed
records (owner/reason/expiry — Windows and macOS 2027-01-31, WSL2
2026-12-31), and `expected_cells` renders browser POLICY GAP cells on
non-capable environments exactly like L2, with per-tier gap messaging in
`classify_one`. The old `continue` is gone.

**M3 (non-atomic history) — RESOLVED BY DECISION, not by rebase.** The
cutover will land as a **squash merge**, so the broken intermediate tree
(`c52eea5b3` deleting `_area-ci.yml` early) never reaches `main`; recorded in
the implementation notes with the rationale that `e844fc2e9` stays
addressable during review. Acceptable — but this is now a **merge-time
obligation**, not a repo state; see the ratification list.

**M4 (guards guard nothing in CI) — FIXED (with a documented residual).**
`affected_scope.py` maps `scripts/` and `.github/ci/` to a `ci_tooling` flag;
a new `ci-tooling` leg in `ci.yml` runs the scope tests and the rollup nextest
suite, wired into the rollup and final-summary `needs`. Contract-tested
(`ci_tooling_changes_schedule_the_tooling_leg`). Residual, correctly recorded:
the R11 **contract** suite itself still runs in no CI job until
`tools/test-toolkit` promotes (`gates = false`, expiry 2026-10-31).

**M5 (l2-backends advisory) — FIXED per ruling 2.** The L2 leg exports
`BISCUIT_TEST_REQUIRED_BACKENDS` as declared ∩ provisioned (`tmux`), making
`_test_l2`'s backend-proof bracket live in CI; contract test added; the
skill and `tools/test-toolkit/src/backend.rs` docs corrected to describe the
intersection rather than "passes it through unmodified". (All 8 L2-declaring
packages include tmux, so no leg ends up with an empty requirement.)

**M6 (AC4 hardcoded list) — FIXED.** The guard now globs
`.github/workflows`, `just/`, `scripts/` (rs/py/sh), the root justfile, and
every area justfile, with a `> 30 files` non-vacuity floor on the glob
itself.

**M7 (docs teach the retired system) — FIXED.** The sharding paragraph in
`rust-testing/SKILL.md` is replaced with the removal rationale and the
required-backends wiring; frontmatter `hash`/`last_updated` regenerated.
`docs/testing-strategy.md` rewritten around the per-package fan-out (spot
checks: sharding text gone, `BISCUIT_TEST_REQUIRED_BACKENDS` described).

## Minors — all 16 fixed

Verified individually; the notable ones:

1. Baseline `schema_version` is now a required field (default removed), with
   `a_version_less_baseline_is_refused`.
2. `FailureEntry` is `deny_unknown_fields`, with
   `a_baseline_entry_with_a_stale_shard_key_is_refused`.
3. `gates = false` without exclusion metadata renders NOT SCHEDULED via an
   explicit "ungoverned" backstop record instead of vanishing; test added.
4. `estimate_jobs` counts WSL as 2 and the docstring no longer claims
   exactness; implementation notes corrected to ~460.
5. Phase 6 scope now names full-scope cache-hit measurement against the
   10 GB quota and the WSL billed-minutes surface.
6. messenger selection is prefix-matched in the scope step
   (`startswith("messenger")`), exported as a proper `messenger` output;
   `ORCHESTRATED` contract updated.
7. The native-union/feature coupling is documented on `build_closure` and
   pinned by `test_biscuit_speaks_closure_still_contains_playa` against the
   real metadata.
8. `biscuit-visualized` records `all-features = true` with a
   promotion-can't-drop-it comment.
9. biscuit-clipboard's three exclusions now carry genuinely package-specific
   reasons (~101 lib / ~17 cli / ~79 service, each with its own blocking
   rationale).
10. Companion-recipe check requires a recipe **definition**
    (`^recipe(:|\s)`, MULTILINE); watch-recipe and parameterized-recipe tests
    added.
11. `backend_hostable()` gives every declared backend its own capability
    axis in both the scope script and the rollup's expected cells; a backend
    with no capability entry is hostable nowhere (tested).
12. `native` arrays sorted — scope.json is byte-stable.
13. Baseline header corrected to "claudine CLI".
14. Stale area/shard/N-A prose fixed in `ci-rollup.rs`, `devops.just`,
    `dmls/Cargo.toml`, and `memory/just.md` (causal sentence restored).
15. README documents the real cache-key scheme and the `wsl` producer name
    (diff reviewed; spot-checked).
16. md-fixture probe also accepts `target/debug/md.exe`.

## Test-coverage gaps — closed

All the review-1 gaps have named tests, verified present in the diffs:
gates-false path, non-propagation, `build_closure` dev-dep edges, the three
`Cargo.lock` branches, top-level-directory fallback, MATRIX_LIMIT overflow,
`estimate_jobs`, `ci_tooling` flag, companion-recipe definition, L2 backend
axis (scope); `status_cells` mapping + exclusions, lint-tier baseline
excusal, synthetic identities out-of-scope **by name**, NotScheduled in the
blocking-states rstest, stray-key and version-less refusals, the four
companion-evidence directions (rollup); B2's enumeration, B5's gate
derivation, the comment-proof companion assertion (contracts).

## Verification performed this round

- Read the full fix diff (all 24 files; workflows, scope script, rollup, and
  contract diffs line-by-line, docs by targeted greps).
- `python3 scripts/ci/test_affected_scope.py` — **53 pass** (was 35).
- `cargo nextest run … --bin ci-rollup` — **133 pass** (was 114).
- `cargo nextest run -p test-toolkit --test ci_workflow_contracts` excluding
  the slow sweeps — **53 pass, 4 skipped**. The 4 slow tier sweeps were not
  re-run here (they build every gating package's test binaries; the developer
  reports 57/57); instead I proved the load-bearing instance directly:
  `cargo nextest list -p claudine-gen` under the canonical L2 filter → exactly
  the 3 declared tests.
- `actionlint` on the three workflows — pre-existing SC2086 infos only.

## New observations (none rise above nit)

- **N1** — the lint-cell companion downgrade in `status_cells` checks the
  declaration but not the environment's node capability; safe solely because
  the lint job hardcodes `ENVIRONMENT: ubuntu-latest`. If lint ever gains
  another leg, scope it like the L1 path.
- **N2** — the two rejection sweeps now cost a near-workspace build inside
  single tests (accepted via the 20-interval slow-timeout in both nextest
  profiles). Until test-toolkit promotes, they run only on developer
  machines — the M4 residual, already recorded.
- **N3** — `ci-tooling` runs the scope and rollup suites but not the
  contract suite; intentional (the contract suite needs `cargo nextest list`
  builds), covered by the same M4 residual.

## For ratification (decisions made during the fix round)

1. **Browser POLICY GAP** — implemented per the spec's letter; the three
   governed `headless_browser` absences carry Ken-attributed owner/reason and
   2026-12-31/2027-01-31 expiries. Confirm the owner and dates.
2. **Squash merge at cutover** — M3 is resolved only if the merge actually
   squashes. This is now a checklist item on whoever lands the branch.
3. **`BISCUIT_TEST_REQUIRED_BACKENDS` wiring** — live in CI as declared ∩
   {tmux}. Confirm this is the intended enforcement posture.
4. **`l2-parallel-self-spawn`** — ratified into spec § survivors (text
   already updated in this round).

## Remaining (unchanged, gated on CI)

- **Phase 4 bridging run** — full-scope, package-keyed; verify/correct the
  candidate baseline before the cutover merge. B3/B4 fixes mean it should now
  produce clean signal on the WSL legs.
- **Phase 6 measurement** — narrow-change before/after plus full-scope
  cache-hit rates and billed minutes (WSL surface included); closes the
  cache-key decision.
- Pre-existing, out of scope: the 6 `junit_staging_contracts` failures
  (broken identically on clean main; `_storage_preflight` temp-dir issue).
