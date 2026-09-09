---
fix: 2026-09-07-faster-claudine-tests
created: 2026-09-08
phase: 10
---

# Results — Faster Claudine tests through complete evaluation and explicit fixtures

The closure record for [spec.md](spec.md), assembled in Phase 10 from the
evidence the earlier phases stored. Nothing here was re-measured; where a
number comes from a stored artifact the artifact is linked, and where the
evidence does not exist yet the row says **pending** rather than passing.
The detailed record stays in its owning document: [`log.md`](log.md) per
phase, [`inventory.md`](inventory.md) for dispositions, overrides and sleep
sites, [`attribution.md`](attribution.md) for the cost attribution,
[`measurement/`](measurement/) for the local runs, and
[`baseline/`](baseline/README.md) / [`candidate/`](candidate/README.md) for the
CI tranche.

## Completion status — three separate claims

| Claim | Status | Evidence |
|---|---|---|
| **Implemented** | **complete** — Phases 2–7 landed every RB1–RB4 item in the plan; Phase 10's two residual edits are import gates only | `plan.md` Phases 2–7 checkpoints; [`log.md`](log.md) §§ Phase 4–7; `git log 9fc5151a0..HEAD -- claudine .config/nextest.toml` |
| **Verified locally** | **L1 and lint green on the current candidate**, minus three identities whose only red input is a *third party's uncommitted edit* to the repository's shipped `prompts/` corpus; L2 stale, three tiers pending on host conditions | § Local verification — current candidate, below |
| **Verified on CI** | **pending** — baseline 1 of 3 runs per leg, candidate 0 of 3 (no push has happened); no budget exists to compare against | [`baseline/README.md`](baseline/README.md) § Status; [`candidate/README.md`](candidate/README.md) § Handoff |

The three claims are deliberately not collapsed. "Implemented" says the code
and documents exist and the local gates that can run are green; it does not
say the performance criteria are met. "Verified on CI" is what the spec's
RB5 and AC6 actually ask for, and it cannot be claimed from this host.

## Local verification — current candidate

This section describes the tree as it stands **now**, not the historical
Phase 10 run. Both were green; they are different trees, and only this one is
the candidate.

**Working-tree state.** Branch `fix/cli-slow-tests` at `fe83e7481`, `main` not
yet merged, committed HEAD plus all four review-3 findings' edits as listed in
[`log.md`](log.md) § Implementation of Review Findings #3. The worktree also
carries uncommitted edits **this fix does not own**: seven modified and three
untracked files under `prompts/`, written by another session while these gates
ran (last write 15:08, mid-cycle).

| Gate | Command | Result |
|---|---|---|
| Canonical L1, **final**, ambient launch identity present | `claudine/just test --no-fail-fast` with `MODEL=opus` exported | **6901 run: 6897 passed, 4 failed, 9 skipped**, 28.98 s runner elapsed, exit 100 |
| Canonical L1, mid-cycle, ambient launch identity **present** | the same with `MODEL`, `YOLO`, `INTERACTIVE`, `AGENT_CWD` all exported | **6901 run: 6898 passed, 3 failed, 9 skipped**, 30.3 s, exit 100 |
| Canonical L1, mid-cycle, ambient launch identity **absent** | the same under `env -u MODEL -u YOLO -u INTERACTIVE -u AGENT_CWD` | **6901 run: 6898 passed, 3 failed, 9 skipped**, 29.1 s, exit 100 — byte-identical failure set |
| Lint | `claudine/just lint` | **exit 0**, zero warnings |
| Real-terminal L2 | `claudine/just test-l2` | `claudine-cli` **218 run: 216 passed, 2 failed**; `claudine-gen` **3 passed**, exit 0 — the two failures are the same `prompts/` corpus |
| Inventory reconciliation | `inventory-reconciler.ts` over captures regenerated at `fe83e7481` | **GATE EXIT=0**, 7,420 runner identities against 7,475 source attributes |
| The `prompts/`-corpus failures, against committed bytes | `--test shipped_prompt_route_drift --test compose_caller_file_provenance` | **18 run: 18 passed, 0 failed** |

The pre-cycle comparison on the same host is **6898 run, 20 failed, 1 timed
out, 48.5 s**. Twenty-one red identities became four, and the four that remain
are the corpus guard described below firing correctly.

The two L1 rows are the point of the fix: the suite now produces the same
result whether or not the shell that launched it exports a Claudine launch
identity. Before this cycle it did not — an ambient `MODEL=opus` reddened six
identities.

**The four remaining failures are not a defect of this branch.** All four read
the repository's own shipped `prompts/` corpus, and that corpus is dirty in
this worktree:

| Failing identity | Reads |
|---|---|
| `shipped_prompt_route_drift::shipped_implement_prompts_have_not_drifted_from_their_fixture` | `prompts/_implement/implement-plan.md` |
| `compose_caller_file_provenance::shipped_implement_router_keeps_the_callers_launch_origin_for_its_lazy_target` | the `implement` route |
| `compose_caller_file_provenance::shipped_implement_router_prefers_an_unimplemented_review_over_the_completed_plan` | the `implement` route |
| `shipped_prompt_contract::feature_review_cli_preserves_numeric_iteration_and_dependent_paths` | `prompts/_reviews/feature-review.md` |

The drift guard names its own cause precisely: the frontmatter hash of
`prompts/_implement/implement-plan.md` is unchanged (`62d70fb16a02592c`) while
its **body** hash moved `56ca8ed9fc5dc007` → `a4e5f2ef36c0395b`. That is the
test doing its job — telling a human that a shipped prompt drifted from the
Level-2 fixture derived from it.

Proven without reverting anyone's work: the modified files were snapshotted,
`HEAD:` bytes were written in their place, both provenance binaries ran
**18 passed / 0 failed**, and every file was restored with a matching checksum.
The fixture pin must **not** be refreshed to make them green — that would bake a
third party's in-progress edit into the corpus. They close when those edits are
committed or reverted by their owner, who then re-derives the fixture with
`CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1`.

**Forbidden shortcuts, checked rather than asserted.** `git diff main --
.config/nextest.toml` is *not* empty, but every non-comment line in it is a
**deletion** of a per-package `slow-timeout = { period = "30s",
terminate-after = 3 }` override (branch commit `0af6cbb50`); the diff contains
no added non-comment line at all. No retry, no tier change, no `#[ignore]`, and
no disabled or weakened assertion was used anywhere in this cycle.

## Measurements

The three costs — build/setup, runner elapsed, summed test duration — are
kept apart in every table because they move independently
([`baseline/README.md`](baseline/README.md) § The three cost columns).

### CI baseline, per leg (predecessor merged, `main` @ `444213eb5`)

Run `34173378609`, `push` to `main`, four legs green, gated by
`junit-metrics.ts` with exit 0 ([`baseline/34173378609/junit-metrics.txt`](baseline/34173378609/junit-metrics.txt)).
**Baseline run 1 of 3.**

| Environment | Build/setup | Runner elapsed | Summed duration | Tests | Failures | Skips |
|---|---:|---:|---:|---:|---:|---:|
| `ubuntu-latest` | 695.1 s | 324.9 s | 324.7 s | 2466 | 0 | 0 |
| `macos-latest` | 832.3 s | 753.7 s | 753.5 s | 2466 | 0 | 0 |
| `windows-latest` | 1012.9 s | 356.1 s | 355.8 s | 2105 | 0 | 0 |
| `wsl2-ubuntu` | 81.3 s | 692.7 s | 692.2 s | 2466 | 0 | 0 |

Run-to-run noise on the identical tree (the PR's own `pull_request` run
`34159725015`, stored as a supplementary sample and counted toward nothing):
matched summed duration PR ÷ `main` is 0.927 / 1.054 / 0.855 / 0.996 on the
four legs. **A candidate ratio inside that 5–15 % bracket on one run is not a
result**, which is why three runs per leg are required.

Runner elapsed equals summed duration on every leg because `claudine-cli`'s
CI profile runs at `max-threads = 1`; on CI the summed column is the floor.
`windows-latest` carries 2105 identities against 2466: 372 Unix-only
(`#![cfg(unix)]` binaries) and 11 Windows-only, declared as platform
exclusions in `baseline/expectations.json` rather than required cross-platform.

### CI candidate, per leg

**None exists.** No push of this branch has happened since the predecessor
merged; the remote branch `fix/cli-slow-tests` was deleted when PR #69 merged,
so the next push recreates it. The handoff — merge `origin/main` (thirteen
conflicting files), re-run the area gates on the merged tree, signed commit,
push, `gh pr create --body-file candidate/pr-body.md` — is written in
[`candidate/README.md`](candidate/README.md). The comparison command, the
within-environment matching, and the additions/removals reporting are ready
and tested (`junit-metrics.ts --baseline`, 57 tests).

`main` moved again at `6504747e2` (PR #70); its run `34232285291` was still in
progress at 15:13 UTC on 2026-09-08 with every `claudine*` native L1 leg
green and the WSL2/L2 legs queued. It is a different source state from the
baseline and is recorded, not collected.

### Local, attribution only (Phase 8, 2026-09-08, 16-core Apple M4 Max)

Local numbers establish no target (spec RB5). They are the attribution the
plan asked for: five alternating warm runs per revision, twice, with the
baseline in a detached worktree at `9fc5151a0` and its own build directory.
Full provenance in [`measurement/provenance.json`](measurement/provenance.json)
and [`log.md` § Phase 8](log.md#phase-8--local-measurement-rb5-first-evidence-tranche).

| `just test` (five crates, one nextest invocation) | Baseline | Candidate | Paired candidate ÷ baseline, min / median / max |
|---|---:|---:|---|
| Build/setup (wall − elapsed), series 2 | 1.5–1.9 s | 1.6–1.7 s | — |
| Runner elapsed, series 2 medians | 52.30 s | **37.89 s** | 0.643 / **0.712** / 0.765 |
| Summed test duration, series 2 medians | 819.42 s | **587.89 s** | 0.637 / **0.707** / 0.758 |
| Runner elapsed, series 1 medians | 53.44 s | 36.01 s | 0.563 / 0.653 / 0.876 |
| Summed test duration, series 1 medians | 836.68 s | 558.03 s | 0.566 / 0.646 / 0.863 |
| Identities run / skipped | 6861 / 11 | 6873 / 9 | +28 / −16 |
| Failed / timed out / leaked / retried | 0 / 0 / 0 / 0 | 0 / 0 / 0 / 0 | every run |

Improved in all ten pairs across both series. The whole-series drift bracket
on this host (42–74 % baseline, 32–57 % candidate) is wider than the effect, so
the strict "median delta clears both brackets" rule reports the suite-level
delta as *not established*; the paired reading, which the alternation exists
to produce, is **0.71**. `just test-rendezvous` is unchanged (paired median
0.96 / 0.95) and no change was claimed there.

Changed cohorts, series 2 medians of summed duration
([`measurement/series-2/report.md`](measurement/series-2/report.md); series 1 agrees in direction on every row):

| Cohort | Tests B → C | Summed B → C | Delta | Established |
|---|---|---:|---:|---|
| lib `composition::sequence::preflight` | 44 → 44 | 45.30 → 4.05 s | −91 % | yes |
| lib `composition::sequence::task` | 106 → 106 | 70.38 → 15.35 s | −78 % | yes |
| lib `composition::schema` | 75 → 75 | 58.95 → 14.43 s | −76 % | yes |
| lib `linking::paths` | 10 → 11 | 14.74 → 2.09 s | −86 % | yes |
| `error_guards` | 18 → 8 | 60.13 → 3.25 s | −95 % | yes |
| `context_command` | 27 → 26 | 43.79 → 12.79 s | −71 % | yes |
| `sequence_overlay_pty` | 7 → 7 | 18.11 → 4.23 s | −77 % | yes |
| Phase 5C context / errors / completion family | 84 → 83 | 52.66 → 21.36 s | −59 % | yes |
| Phase 5D live-child and PTY cohort | 11 → 12 | 20.46 → 6.23 s | −70 % | yes |
| guards, probes, fixture self-tests | 62 → 71 | 69.82 → 21.90 s | −69 % | inside drift |
| Phase 5A compose family | 69 → 69 | 20.54 → 16.34 s | −20 % | inside drift |
| Phase 5B sequence / loop family | 155 → 155 | 49.09 → 46.66 s | −5 % | inside drift |
| `claudine-cli` bin unit tests (untouched control) | 1694 → 1694 | 81.83 → 89.69 s | +10 % | inside drift |
| contract / catalog-types / gen (untouched control) | 223 → 224 | 27.91 → 29.35 s | +5 % | inside drift |

The two untouched controls move by +5–10 % under the same host, which is the
size of the noise; the established rows move by −59 % to −95 %. The 5A/5B
families were migrated for isolation (RB2), not speed, and no timing claim is
made for them.

Every changed timing or concurrency contract ran eleven times on the candidate
under full-suite load (13 targets, 36 identities, 396 executions): 0
non-passing, 0 retries, 0 leaks. One outlier is recorded rather than smoothed
(`level2_pty_provided_partial_single_match_confirms_and_launches`, 4.609 s once
against a 0.81 s median, passed).

### Work counters, independent of timing (Phase 8, [`measurement/sentinels/summary.tsv`](measurement/sentinels/summary.tsv))

| Claim | Counter | Baseline → candidate |
|---|---|---|
| The ambient CWD walk left the redirected library fixtures | lldb entry-location hits on `capture_file_resolution_context`, one process per module | `schema` 73 → 11 · `preflight` 62 → 2 · `task` 84 → 0 · `linking::paths` (`resolve_repo_root`) 10 → 3; every survivor is a test whose subject is real discovery |
| `error_guards` scans production sources once | processes that ran `run_scan` | 12 of 18 → **1** of 8 |
| `context_command` no longer launches from the checkout | `git` shim on `PATH` logging cwd | `rev-parse --show-toplevel` from the checkout 44 → 0; `git init` under `$TMPDIR` 0 → 34, none inside the checkout; `current_dir(repo_root())` sites 23 → 0 |
| No process outlives the suite, no audio on the host | `just test-leaks claudine` | two orphaned `claudine` audio workers before Phase 7 → `no leaked processes detected` |
| Zero raw spawn sites, zero isolation escapes | the two structural gates, executed inside every `just test` run | 36 files / 172 sites across 89 governed → **0 / 0 across 90**; isolation 0 escapes across 37 → **0 across 74** |

### Budgets

**None derived, by construction.** `deriveBudgets` refuses while any leg has
fewer than three consecutive green baseline runs. The ratification procedure is
fixed in `inventory.md` § Budgets — `observedMax × 1.25` per leg and family,
legs never merged, a miss reported with its cause and never closed by adjusting
the budget. No universal speedup percentage is offered; the local paired ratio
is a prior for the `claudine-cli` `max-threads = 1` leg's summed column and
nothing more.

**Update 2026-09-09 — one of the two causes is closed.** The JUnit → family
join now exists (`attribution.ts aggregate`, shared implementation in
`tools/test-audit/src/attribute/aggregate.ts`), so `perLegFamilySummed` is no
longer empty for want of tooling: run against `baseline/34173378609/` it exits 0
with 18 families on each Unix leg and 15 on `windows-latest`, reproducing the
summed-duration column in § CI baseline above. Feeding that output to
`deriveBudgets` still refuses, now with `insufficient-runs`
(`1 green run(s); 3 consecutive are required`) rather than `missing-leg`. The
remaining cause is evidence, not code: two more consecutive green `main` runs
per leg, plus the candidate tranche. `attribution/budgets-pending.json` is left
at `runsPerLeg` 1 with an empty `perLegFamilySummed` on purpose.

## Measurement re-run — deferred, with the blocker named

Review 1's closure criterion 1 ends "regenerate inventory and measurement
evidence". The **inventory** half is done: the sixteen listings were retaken
from the working tree at `78b44a96651e`, the superseded `9fc5151a0` listings
are preserved under
[`enumeration/9fc5151a0/`](enumeration/9fc5151a0/), and
`inventory-reconciler.ts` is back to `GATE EXIT=0` over 7,417 identities
(7,400 before). See [`inventory.md`](inventory.md) § Population.

The **measurement** half is deferred. It is not skipped for convenience: the
protocol cannot legally run here.

- The protocol is [`spec.md`](spec.md) § 5's five alternating warm runs per
  cohort, and `attribution.ts` **rejects a red run by design** — a log whose
  result lines disagree with its `Summary`, or that carries failures, is
  refused rather than averaged in. That refusal is the tool working.
- This branch's suite was red before any of this cycle's work, in two packages.
  Review 3 finding 1 diagnosed and repaired all but three of those identities;
  the table below is the ledger, superseding the earlier "18 pre-existing
  failures" reading, which had misattributed several of them.

| Failing identity | Count | Actual cause | Now |
|---|---:|---|---|
| `claudine-cli::bin/claudine …loop_control::target_launch::tests::*` | 5 | ambient `MODEL` out-ranking each fixture's frontmatter, **not** commit `f0aaa4832` | **fixed** — `LaunchRebuildIntent::env_lookup` seam |
| `claudine-cli::propagated_context_fixtures::isolated_fixture_can_opt_in_to_provider_memory_discovery` | 1 | the same ambient `MODEL` | **fixed** — fixture scrub list |
| `claudine-cli::spawn_inventory::production_spawn_inventory_is_complete_and_governed` | 1 | line-number drift; the site set, functions and governance are unchanged | **fixed** — census refreshed, five integers moved |
| `claudine-cli::wrap_sigint::compose_sigint_during_prep_exits_130_with_notice` | 1 | not reproducible in isolation (5 runs, 0.179–0.186 s each) | **passes** in both full-suite runs |
| `claudine-gen::drift::committed_*` | 5 | **not** the archived baseline (already repointed at `gen/tests/fixtures/`): branch commit `4fb054ed1`'s formatting sweep rewrapped generator-owned sources | **fixed** — regenerated; proven formatting-only |
| `claudine-gen::generate_ux::*` | 5 | the same, surfacing as "the area is not clean" | **fixed** — same regeneration |
| `claudine-cli::shipped_prompt_route_drift` + `compose_caller_file_provenance` | 3 | a third party's uncommitted edits to `prompts/` | **open**, not this fix's — see § Local verification |

**Consequence for the numbers in this fix.** Every **Executed** and
**Summed cost** figure in [`inventory.md`](inventory.md) and every figure in
[`attribution.md`](attribution.md) remains the `9fc5151a0` measurement, and is
labelled as such rather than restated. Six families' identity counts moved
(`inventory.md` § Family index lists them), so for those six the recorded cost
is now a lower bound on the family rather than a measurement of it. No number
was extrapolated, scaled, or invented.

**What closes it.** Fifteen of the eighteen went green in review 3 finding 1;
the remaining three are the `prompts/` corpus rows, which need only their
owner to commit or revert the edits. Once the tree is clean the § 5 protocol
can be re-run and `attribution.md`, `inventory.md`'s two cost columns, and
`measurement/` regenerated together — the tool's refusal to average a red run
no longer blocks it for a reason inside this fix. Owner document:
[`../_unscheduled/test-suite-residuals/spec.md`](../_unscheduled/test-suite-residuals/spec.md)
§ Residual 8.

## Coverage changes

Every test-population change, not netted, with the replacement coverage for
each removal. `just test` moved 6861 → 6873 and `just test-rendezvous`
272 → 273; skips 11 → 9.

| Phase | Additions | Removals | Renames / merges | `just test` after |
|---|---:|---:|---|---:|
| 4 | 9 (fixture self-tests, guard arms) | 0 | — | 6870 |
| 5 | 8 (`contamination_probes`) | 0 | 1 rename (spawn-gate non-vacuity test) | 6878 |
| 6 | 5 (+ the corpus test carrying 12 merged arms) | 2 + 12 merged | 4 renames | 6871 |
| 7 | 3 (two fixture tests, one rendezvous endpoint test) | 0 | 30 `#[serial(pty)]` annotations removed across 7 files | 6873 (+ rendezvous 273) |
| 10 | 0 | 0 | — | 6873 |

**Removals and their replacement coverage** (the sixteen identities in
[`measurement/series-2/report.md`](measurement/series-2/report.md) § Identity changes):

- Twelve `error_guards` scan-backed identities → named arms of `SCAN_GUARDS`,
  evaluated by `production_sources_pass_every_scan_backed_guard`, which reports
  every failing arm under the name it used to carry;
  `a_failing_scan_backed_guard_is_reported_under_its_own_name` is the
  non-vacuity witness (synthetic scan with one planted collapse). Live neuter:
  a stale `transport-allow.toml` entry fails under
  `every_allowlist_entry_still_matches_a_live_site`.
- `stream::stderr::tests::silent_mode_produces_nothing` (a value compared with
  itself) → `render::event_renderer::silent_verbosity_produces_no_render_units`
  (with a Normal-verbosity control) and
  `session_start_updates_the_auth_source_even_when_silent`, which pins an
  ordering the doc comment promised and nothing tested.
- `linking::paths::repo_scope_target_paths_are_absolute` →
  `…_are_rooted_at_the_repository`: `is_absolute()` accepted a bare
  `/.claude/skills`; the replacement asserts equality with the repository root.
- `context_command::context_values_renders_non_null_for_canonical_keys` →
  folded into `context_values_renders_canonical_keys_non_null` as a fourth key
  (one launch instead of two, same assertion).
- `spawn_site_guard::an_empty_allowlist_leaves_every_live_site_unlisted` →
  `the_spawn_gate_reads_a_real_population_and_still_finds_a_planted_site`
  (population > 50 files, zero census, and a planted `cargo_bin("claudine")`
  the same detector still finds).

**Changed assertions**, each with the defect the old one could not see
(`log.md` § Phase 6): `wrap_opencode`'s `error` field is now asserted to carry
the provider's rate-limit text (neuter transcript in the log); three
`linking::paths` tests assert path equality instead of `ends_with`, which a
home/repo root swap satisfied; the two `context_command` width sweeps keep
every launch and assertion but build one repository per sweep instead of
nineteen.

**Moved boundaries and routes**: two stale tier identities entered L1 by
rename (`wrap_sigint::compose_sigint_during_prep_exits_130_with_notice`,
`claudine-gen::signals_validation::shipped_corpus_builds_deterministically`);
`rendezvous-daemon::peer_discovery`'s two `real_*` mDNS identities gained
their first route (`just test-real` inside `claudine/rendezvous`);
`test-real` in `claudine/` moved from `cargo test` onto nextest; the four
PTY binaries and `sequence_overlay_pty` now run concurrently. Review-1
finding 1 then moved those four binaries and their 19 identities from L2 to
L1: they open `/dev/ptmx` and create no emulator session, so the tier claim
was wrong. They are now `level1_dry_run_pty`, `level1_provided_partial_file_pty`,
`level1_pty_wrapper_summary` and `level1_schema_prompt_pty`, carry no
`terminal-tests` gate, and run in `just test` / `just test-cli` (19 of 19,
4.72 s, 2026-09-09). No test moved to a *higher* tier and none was
`#[ignore]`d.

**Narrowed claims** (review-1 closure criterion 2). One identity was renamed
because its name asserted more than its assertions could reach:
`level1_pty_wrapper_summary::level1_pty_wrapper_summary_shows_badges` →
`…::level1_pty_wrapper_summary_text_precedes_child_output`. Every assertion is
unchanged — the same five `expect` calls in the same order — so this is a
statement change, not a coverage change; the test count is unaffected. The old
name and the review's requirement row read as a rendering claim ("badges are
visible as rendered terminal UI"), which an `expectrl` substring match over a
raw `/dev/ptmx` stream cannot establish. **Replacement coverage: none, by
decision.** No `level2_*` binary in `claudine/cli/tests` asserts the wrapper
header row's rendered form (`level2_perf_capture` puts the header on screen but
asserts only the perf tree; `level2_dry_run_metadata_capture` asserts the
`--dry-run` metadata table's `YOLO` cell, a different surface). The badge
constants are `Prose` output, and `biscuit-terminal-cli::level2_prose_styling`
proves that renderer emits bold and fg/bg RGB in real WezTerm and Kitty
sessions — evidence for the primitive, not for this row's composition, order,
or width. Adding an L2 capture was rejected under Rule 2 for a
test-performance fix; the gap is recorded in `inventory.md`
(`cli-l1-pty-interactive` § Disposition) and in the binary's module docs, which
now state what the assertions do and do not establish.

**Runner overrides**: eight `.config/nextest.toml` blocks removed with the
cost each hid (`inventory.md` § Phase 6 disposition); none added. Against
`main` the file's diff is 41 deletions and 16 insertions, every inserted line
a comment, zero non-comment insertions (re-checked in Phase 10).

## Residual findings

**Deferred with an owner document** — evidence, reason and acceptance
criteria for each are in
[`../_unscheduled/test-suite-residuals/spec.md`](../_unscheduled/test-suite-residuals/spec.md):

1. Four zero-byte bench entry points in `claudine/lib/benches/`.
2. The `context` reports have no in-process render seam, so the width matrix
   cannot move below the CLI boundary.
3. `a_failed_ownership_setup_kills_the_spawned_command` holds vacuously; the
   runner would have to expose the direct child's pid.
4. `claudine-contract::real_provider` fails `Unauthorized` on an expired
   credential where its own contract says it skips.
5. `completion_perf::perf_enter_compose_partial_meets_target` (`#[ignore]`d)
   fails on this host before and after the migration.
6. The `claudine-cli-ci-l1` `max-threads = 1` test group is the largest CI
   cost lever and needs the candidate CI evidence before it can be relaxed.
7. Budget derivation needs a JUnit → family aggregator before three runs per
   leg can produce `perLegFamilySummed`.
8. The local cost measurement cannot be re-run while eighteen pre-existing
   failures in `claudine-cli` and `claudine-gen` keep every suite run red;
   § Measurement re-run above names each one.

None of these is generic fixture migration; the spawn allow-list is empty
(AC4's "cannot be deferred" clause is satisfied).

**Host conditions, recorded as pending rather than passing**:

- `just test-l2`: 236 of 237 on every run since Phase 4 — 237 while the 19
  PTY identities were still routed here; the tier is 218 after review-1
  finding 1 reclassified them, and has not been re-run since. The survivor is
  `level2_initialize_proxy_block_auto_detects_osc8_in_wezterm`, where Atuin's
  first-run prompt sits in the spawned WezTerm pane and swallows the exit
  marker. Not in any file this fix touched.
- `just test-real`: 4 of 5 `Unauthorized` (the provider CLI is not
  authenticated on this host), identical under the retired `cargo test` route.
- `just test-l3`: not run — it injects OS keystrokes and steals focus, which a
  non-interactive session must not do.
- `just bench`'s preflight refuses on this host's inflated 1-minute load
  average (AutoMounter/SMB); Phase 6 ran it with `BENCH_YES=1`, exit 0.
- `identityservicesd` at 30–90 % of a core during the Phase 8 series; disclosed
  as noise the paired protocol was chosen to survive.

**Observations recorded, not deferred**:

- `GIT_DIR` / `GIT_WORK_TREE` do not move the *child's* `ctx.repo_root`
  (claudine's discovery walks the filesystem); the scrub's proven hazard is
  the parent-side helper, which is what the Git probe exercises.
- One nextest `LEAK-FAIL` during Phase 7's stabilisation runs whose identity
  was not captured and which did not recur in the 11 consecutive runs after
  it or the 55 Phase 8 runs.
- Nine `#[ignore]`d performance harnesses stay ignored as a recorded decision
  (`inventory.md` § Reachability findings): four are diagnostic printers and
  five assert host-load-sensitive wall-clock budgets.
- The two Windows-only unused-import warnings carried from Phase 1 are closed
  in Phase 10 by gating the imports with `#[cfg(unix)]`; the third, in
  `compose_caller_file_provenance.rs`, had been reported gone in Phase 9 only
  because the warm check replayed a subset of cached diagnostics, and is gated
  the same way.

## Acceptance criteria

| # | Criterion | Status | Evidence |
|---|---|---|---|
| AC1 | Every discovered test/family has a reviewed disposition and a reconciled platform/feature/tier route; none omitted by timing threshold | **verified** | `inventory-reconciler.ts` exit 0 at `9fc5151a0`: 7400 identities / 163 targets, 0 unassigned, 0 double-assigned, 0 stale families; 55 cfg exclusions listed apart; output reproduced in `inventory.md` § Reconciler output |
| AC2 | Zero generic residual spawn exemptions; live-child and ordinary paths share the policy; negative guard tests and Windows proof | **verified** | `SPAWN_ALLOWLIST` empty, gate artifact `files:0 sites:0 governed_files:95` (90 before review-1 finding 1; the guard now classifies by the resource a file owns rather than by its name, so the four PTY binaries joined the governed population instead of escaping it on a `level2_` prefix); one `ChildEnvironment` feeds `build`, `build_std`, `apply_policy_to`, drift-tested by `both_command_surfaces_hand_the_child_the_same_environment`; seven neuter transcripts (`log.md` § Phase 4); `just check-windows` exit 0 with zero warnings (Phase 10); the two Windows console-control tests ran green on `windows-latest` in run `34173378609` |
| AC3 | Inherited-width failures covered; contamination probes cannot alter unrelated results; probes use disposable state | **verified** | `COLUMNS=44` transcripts before/after in `log.md` § Phase 5; eight probes in `contamination_probes.rs`, four neuters each firing exactly the expected probes; the one checkout-adjacent probe uses a directory under gitignored `target/` and removes it on both paths |
| AC4 | Shared-setup, cleanup, assertion and reachability findings resolved; deferrals evidenced and linked; generic fixture migration not deferred | **verified** | four unreachable identities now run; `test-real` on nextest; five metadata blocks declared; seven assertion repairs; leak sweep clean; deferrals 1–7 above with owner document; spawn allow-list empty |
| AC5 | Every pre-existing override justified in the inventory or removed with the cost it hid; none added | **verified** | `inventory.md` § Runner override census and § Phase 6 disposition; `git diff main -- .config/nextest.toml`: 41 deletions, 16 comment-only insertions, 0 non-comment insertions |
| AC6 | Local gates pass; `just check-windows` recorded; platform limitations explicit; budgets have compatible CI evidence | **partially verified — CI half pending** | gate ledger below (all local gates green or recorded pending with cause); `check-windows` exit 0; exclusions declared per leg; **no budget exists and no candidate CI run exists** — see § Budgets. The aggregation tooling shipped 2026-09-09 and the refusal narrowed to `insufficient-runs`, which changes the *reason* AC6 is pending, not its status |
| AC7 | `results.md` complete; docs and skills updated only where workflow or architecture changed | **verified** | this document; skill edits limited to the cross-compile route, the measurement workflow, the rendezvous/builder surfaces Phases 5–7 already repaired, and one falsified Windows claim in `signal-handling.md` (`log.md` § Phase 10) |

## Gate ledger — Phase 10 reconciliation

Rule: a check is credited when its inputs are unchanged since it ran; it is
re-run when Phase 10's edits invalidated it. Phase 10 changed two
`claudine-cli` test files (import gates), which invalidates the `claudine-cli`
L1 suite, the area lint, and the Windows check, and nothing else.

Review 3 finding 1 (2026-09-09) invalidated this ledger's `just test` and
`just lint` rows again — it changed `claudine` and `claudine-cli` sources,
`claudine-cli` tests, and thirteen generator-owned files. Both were re-run;
§ Local verification — current candidate carries the results, and the two rows
below are retained only as the Phase 10 historical record. The remaining
credited rows still hold: no rendezvous, bench, or Windows-arm input moved.

| Gate | Where | Result | Run or credited |
|---|---|---|---|
| `just test` | `claudine/` | **6873 passed / 9 skipped, exit 0** (35.7 s runner elapsed) | **superseded** — Phase 10 historical record; the current candidate's run is in § Local verification |
| `just test-leaks claudine` | repo root | **7146 passed / 11 skipped, `leak-sweep: no leaked processes detected`, exit 0** (39.8 s runner elapsed, 3 m 08 s wall) | **run**, Phase 10, after both edits; `candidate/local-gates/phase10-test-leaks-claudine.log` |
| `just check-windows` | `claudine/` | **exit 0, zero warnings** — warm (0.9 s) and cold in a fresh `CARGO_TARGET_DIR` (1 m 28 s), so the count is not a cached-diagnostic artifact | **run**, Phase 10; `phase10-check-windows.log`, `phase10-check-windows-cold.log` |
| `just lint` | `claudine/` | **exit 0, zero warnings** (5 m 03 s wall, overlapping the cold check) | **superseded** — re-run on the current candidate, **exit 0**; see § Local verification |
| `just doctest` | `claudine/` | **exit 0** — 25 passed / 7 ignored across the four lib crates; `claudine-cli` skipped (no lib target) | **run**, Phase 10; `phase10-just-doctest.log` |
| `just test-rendezvous` | `claudine/` | 273 passed / 2 skipped, exit 0, ×11 | **credited** from Phase 8 — no rendezvous input changed; also inside the Phase 10 leak sweep's population |
| `just test-l2` | `claudine/` | 236 of 237 + `claudine-gen` 3 of 3; the host-condition survivor above | **stale** — credited from Phase 9 (2026-09-08), but review-1 finding 1 removed 19 identities from the tier (237 -> 218) and it has not been re-run; owned by the separate review-1 work unit for the WezTerm survivor |
| `just bench` | `claudine/` | exit 0 (`BENCH_YES=1`) | **credited** from Phase 6 — no bench input changed |
| `just ci-local` | repo root | 147 of 147, 73 packages, exit 0 | **credited** from Phase 9; the same tree minus two import gates |
| `just test-l3` | `claudine/` | — | **pending**, focus-stealing tier not run from a non-interactive session |
| `just test-real` | `claudine/` | 1 passed / 4 `Unauthorized` | **pending**, host credential state (Phase 6) |
| `just test` / `just lint` | `sniff/` | **2599 passed / 23 skipped, exit 0** (52.4 s); lint **exit 0** | **run**, Phase 10 (the session's starting area; no `sniff` file changed); `phase10-sniff-just-test.log`, `phase10-sniff-just-lint.log` |
| `npx tsx --test junit-metrics.test.ts` | fix directory | 57 passed | **credited** from Phase 9; no script changed |
| `git diff main -- .config/nextest.toml` | repo root | 41 deletions, 16 comment insertions, 0 non-comment insertions | **run**, Phase 10 |

## What closes the pending items

All operator actions; none can be taken from this session.

1. Merge `origin/main` into the branch (thirteen conflicting files, listed in
   `candidate/README.md`), re-run `just test` and `just lint` in `claudine/`
   on the merged tree, signed commit, push (the remote branch has to be
   recreated), `gh pr create --base main --body-file candidate/pr-body.md`.
2. Read the first candidate run on every leg for correctness; let normal CI
   accumulate three consecutive green candidate runs per leg; collect each with
   the recipe in `candidate/README.md`, failures included.
3. Either `gh run rerun 34173378609` twice for the two missing baseline samples
   at `444213eb5`, or accept a baseline of one run at that source state.
4. ~~Write the JUnit → family aggregator (residual 7)~~ — **done 2026-09-09**,
   `attribution.ts aggregate`. What remains: point it at the three collected
   runs per leg, replace `attribution/budgets-pending.json` with its output
   (which carries the real `runsPerLeg`), and let `deriveBudgets` produce the
   table beside the baseline in `inventory.md` § Budgets. Then compare, and
   report every miss with its cause.
