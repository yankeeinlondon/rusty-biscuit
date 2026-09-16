---
$schema: feature-review.yaml
ready: true
findings:
  - title: ci-tooling checked out at depth 1, so the baseline-revision suite reported green without running
    priority: high
  - title: The workflow reader's unbounded forward search could compare the wrong revision pair and pass
    priority: high
  - title: 23 of 55 plan-validation rules are pinned by no assertion anywhere
    priority: high
  - title: Area-drift enforcement is scheduled nowhere because ci-tooling's path gate excludes sniff
    priority: high
  - title: The source review's require_tool() recommendation would redden macOS and Windows preflight
    priority: high
  - title: An artifact-collision refusal in the plan validator is unreachable
    priority: medium
  - title: The published code for a plan-structure problem is assigned outside schema.py
    priority: medium
  - title: The self-test suite list is coupled across four files and fails loudly only by accident
    priority: medium
human_review: false
reviewed_by: claude/opus-5
created: "2026-09-15T17:15:52-07:00"
spec: 2026-09-12-single-os-compile/spec.md
implemented: true
description: "Findings from the four spikes raised by the CI Python test-suite review"
fix: 2026-09-12-single-os-compile/review-4.md
previous: 2026-09-12-single-os-compile/review-3.md
log: fixes/2026-09-12-single-os-compile/log.md
---

# Review 4 — Spike Findings

**Review 3's readiness verdict stands.** No spike found a defect in this fix's
implementation of the specification. The two high-severity defects the spikes
did find — and fixed — were in the *test and CI harness around* the fix, and
both had the same shape: **a suite reporting success for work it never
performed.** That shape matters here because AC1 and AC9 rest on those suites.

This review records the four spikes proposed by
`reviews/2026-09-15-python-test-code/review.md` §6, all executed on 2026-09-15.
It is not a re-adjudication of the implementation; `human_review: true` is set
for two design rulings the spikes deliberately left open rather than decide
alone.

Each spike's full measurements live beside its plan:
[spike 1](../../reviews/2026-09-15-python-test-code/spike-1-results.md),
[spike 2](../../reviews/2026-09-15-python-test-code/spike-2-results.md),
[spike 3](../../reviews/2026-09-15-python-test-code/spike-3-results.md),
[spike 4](../../reviews/2026-09-15-python-test-code/spike-4-results.md).

## Orchestration

Spikes 1 and 2 ran concurrently under partitioned file ownership; spikes 3 and
4 ran serially and alone, because each is decided by an absolute wall-clock
threshold (30s/3min and ~2min respectively) that background load would move
across a bucket boundary. Isolated git worktrees were not available: the fix's
own files (`scripts/ci/build_key.py`, `test_build_key.py`, `plan_fixtures.py`,
and others) are untracked in this worktree, so a fresh checkout from `HEAD`
would not contain the code under investigation.

## Rulings

| Spike | Question | Ruling | Selected by |
|---|---|---|---|
| 1 | PyYAML dependency for the workflow readers? | **No — keep the heuristic** | 0 silent misreads after guards; `yaml` absent on 3 of 5 measured interpreters |
| 2 | Add `PLAN_REJECTIONS`, subdivide `malformed-receipt`? | **No — change nothing** | 1 collision (the rule's 1–2 band); step-2 gate tripped independently |
| 3 | Where does `test_build_key.py` get wired in? | **`ci-tooling` unconditionally** | 26.4s cold, under the 30s threshold |
| 4 | Where is AC15 enforced? | **Off the merge path, new `area-drift.yml`** | 264.6s cold sniff build vs a ~2min budget |
| 4 | Query semantics A / B / C? | **A — per-directory detection** | `SNIFF_SELF_INCONSISTENT` re-measures non-empty |

Three of the four rulings are the cheap outcome the plans predicted. Only spike
4 moved work, and it moved it *out* of the merge path rather than in.

## High-severity findings

### `ci-tooling` checked out at depth 1, so a suite reported green without running

`.github/workflows/ci.yml:729` used the default shallow checkout, where
`BASE_REVISION` does not exist. `test_build_baseline_revision.py` would have
reported **11 tests green having actually run 3** — proven against a real
`--depth 1` clone, not inferred. Fixed with `fetch-depth: 0` (spike 3).

This is the clearest instance of the pattern above: the suite guards AC8's
measurement baseline, and a shallow checkout silenced it without failing.

### An unbounded forward search could compare the wrong revision pair and pass

`test_build_baseline_revision.py:238`'s `_run_body` searched forward for
`^        run:` with no stop condition, so a `uses:`-only gate step made it
return **the next step's** script. Its only job is a byte-identity comparison
between two revisions; it would have compared the wrong pair and passed. 96 of
the 100 new loud failures the consolidated reader introduced are this case
(spike 1).

### 23 of 55 plan-validation rules are pinned by no assertion anywhere

Spike 2 went looking for brittleness and found a completeness gap instead. All
of `_dependent_seam` (`scripts/ci/schema.py:951`, `:959`, `:963`) and half of
`_cell_consistency` (`:972`, `:975`, `:989`, `:998`, `:1006`) can refuse a plan
that no test exercises. This belongs in the source review's §1 (completeness),
not §5 (brittleness), and it is a larger risk to AC1 than the coarse rejection
codes the spike was commissioned to investigate.

### Area-drift enforcement is scheduled nowhere

`ci_tooling` is path-gated and the gate excludes `sniff/**`
(`scripts/ci/affected_scope.py:120-128`), so even with `sniff` installed in
that job, a change to sniff itself would schedule no area-drift check. Spike 4's
`area-drift.yml` therefore carries its own `pull_request` trigger on `sniff/**`.
Adding `sniff/**` to the path gate is the merge-path alternative; it was not
taken because that file belongs to the planner, not the spike.

### The source review's `require_tool()` recommendation would redden two OSes

`reviews/2026-09-15-python-test-code/review.md` §1.2 recommends a shared guard
that *fails* under `CI` rather than skipping. Spike 4 found that
`test_resolved_plan.py` also runs in `preflight` on up to three OSes every push
(`.github/workflows/ci.yml:489`), where `sniff` is likewise absent — so applying
that recommendation as written would turn macOS and Windows red on every push.
The recommendation is still right in principle; it needs the per-job scoping
spike 4 shipped (`BISCUIT_REQUIRE_SNIFF=1` in the job that owns the contract,
an honest skip message naming that job everywhere else).

**This finding modifies a prioritized action in the source review and should be
read before anyone implements it.**

## Medium and low findings

| Sev | Anchor | Finding |
|---|---|---|
| M | `scripts/ci/schema.py:802-806` | Unreachable: `expected_artifact` derives from `{package, producer, key}`, so an artifact collision implies a key collision already caught at `:759-765`. `test_an_artifact_name_collision_is_refused` (`test_schema.py:521`) does not reach it — its fixture trips the mismatch rule at `:799`. Needs a behavior decision (delete, or admit a hand-spelled artifact). |
| M | `scripts/ci/local_evidence.py:257-261` | Assigns the `malformed-receipt` code a plan-structure problem carries into CI. `schema.py` does not own it, which is why a fourth vocabulary there could not have changed what any consumer sees. |
| M | `scripts/ci/test_resolved_plan.py:543` | Asserts an exact coded literal with `assertNotIn` — silently vacuous after any code change. |
| M | `just/ci-local.just:449` | The self-test list is coupled to `test_ci_local.py:130`/`:884` and `.githooks/tests/test-pre-push.sh:2099`. Editing the recipe alone produced **10 measured failures**, so source review §3.6 is [M], not [L] — it fails loudly only because the fixtures stub by name. |
| M | `scripts/ci/test_ci_local.py:1057` | Read `needs:` sequence items as steps; `ci.yml:861` (`ci-gate`'s `needs:`) yielded 9 phantom steps, silently dropped. Correct answers were accidental. |
| M | `~/.cargo/config.toml:7` | A global `rustc-wrapper = "kache"` makes any un-neutralized build timing on this host 3–6× optimistic. Both build spikes neutralized it and verified the neutralization. |
| M | `.github/workflows/sniff-performance.yml:121` | Already builds sniff nightly on three OSes and on `sniff/**` PRs — the cheapest *blocking* home for the area-drift contract. Left alone, but it is the first place to look if the nightly proves too weak. |
| L | `scripts/ci/build_key.py:59-62`, `just/devops.just:59` | Hardcode `scripts/target` while `just/ci-local.just:160` redirects `CARGO_TARGET_DIR`; not verified end-to-end. This is also why `CARGO_TARGET_DIR` could not be used to fake a cold build. |
| L | `scripts/ci/build_key.py:153` | The planner's own `subprocess.run` is unbounded, like the one spike 3 fixed in the test. |
| L | `scripts/ci/schema.py:1128` | Rewrites a code with `str.replace` because `_sha`/`_str_list` lack the `code` parameter `_keys`/`_member` have. |
| L | `scripts/ci/workflow_reading.py:191` vs `:214` | `job_run_steps` tolerates step-key reorder, `step_script` does not. Both loud, but the asymmetry is a trap. |
| L | `scripts/ci/test_affected_scope.py:2482` | Asserts `any("native" in error ...)` — one word, and a field name. |
| L | — | No workflow runs `setup-python` or `pip install`, so any future third-party Python dependency needs edits to `ci.yml`'s `preflight` **and** `ci-tooling`, plus a `just init` path. |
| L | `os` skill | `BUILD_WIN` has **no usable `python3`**: it resolves to a Cygwin shim pointing at a deleted `Python313\python.exe`; a working 3.13.14 exists only as `py`. |

## Corrections to the source review

The spikes refuted four claims in `reviews/2026-09-15-python-test-code/review.md`.
Recorded because that review is otherwise accurate and will be read again.

- **§5.1 / Spike 2's premise.** `test_schema.py:594` and `:613` do **not** pin two
  different rules. Both reach one rule at `schema.py:913` with a disjunctive
  condition; no code subdivision could separate them. The claimed collision that
  motivated the spike does not exist.
- **§5.1 counts.** The emission surface is **55 sites, 46 coarse**, not 53/45 —
  the original count matched only `problems.append`.
- **§5.6 tally.** **8 of 12** `test_build_key.py` tests need the helper binary,
  not 6; and `:76` needs no binary only while `cargo` is on `PATH`.
- **Spike 4's own hypothesis.** The `ThreadPoolExecutor` is **not** the cost
  driver. System time is flat at ~70s across `max_workers` 1/4/8/16 (wall
  10.9/7.1/6.2/6.1s), so the "one-line fix worth taking whatever else is
  decided" would cost 4.7s and save nothing. Shipped as a comment instead.

Two of spike 1's own mutants were also vacuous as specified: `anchor` and
`flow-mapping` sit inside a `uses:` step's `with:` block, which every reader
skips. They were replaced by `flow-step` and `alias-run`, which turned out to be
2 of the 4 real silent misreads.

## Measurements

Build timings are on an M4 with `CARGO_BUILD_JOBS=4`, a fresh `CARGO_HOME`, and
the `kache` wrapper neutralized and verified absent from a full `cargo build -v`
log.

| Measurement | Value |
|---|---|
| `test_build_key.py` cold / warm | **26.4s** (207 crates, 1.0G target) / **0.10–0.12s** |
| …with `kache` left active | 4.3s — *not cold*, recorded to show the trap |
| …as positioned in `ci-tooling` | **0.1s**, after the step that already builds `ci-build` |
| `sniff-cli --release` cold / warm | **264.6s** (556 crates, 1.7G) / **99.5s** |
| `ci-tooling` wall, six real hosted runs | **5m54s – 6m43s** (20-minute timeout) |
| Area contract, `max_workers` 1/4/8/16 | wall 10.9 / 7.1 / 6.2 / 6.1s; **sys flat at ~70s** |
| Inverted (cheap) area query | 2.7s, 33 calls — but **72 of 73** packages |
| Workflow reader consolidation | −115 lines across 3 suites → one 315-line module |
| Differential over 14 shipped workflows | 584 probes: identical 484, **differing 0**, new-only-raise 100 |
| Python suite baseline | **607 → 611 tests**, 0 failures, 0 skips, 168.8s → ~145s |

`yaml` availability, measured: macOS `/usr/bin/python3` 3.9.6 **absent**;
macOS Homebrew 3.14.7 present; `BUILD_LINUX` 3.13.5 **absent**; `BUILD_WSL`
3.14.4 present; `BUILD_WIN` `py` 3.13.14 **absent**.

## What shipped

| File | State |
|---|---|
| `scripts/ci/workflow_reading.py` | new — one reader replacing four, loud on every layout it cannot read |
| `scripts/ci/fixtures/workflow_mutants/` | new — 11 mutants + provenance SHAs, a permanent regression corpus |
| `.github/workflows/area-drift.yml` | new — schedule + dispatch + `sniff/**` PR; `BISCUIT_REQUIRE_SNIFF=1` makes absence fail, not skip |
| `.github/workflows/ci.yml` | `fetch-depth: 0`; the two orphaned suites wired into `ci-tooling` after the step that builds `ci-build` |
| `scripts/ci/test_build_key.py` | `timeout=` on the previously unbounded subprocess |
| `scripts/ci/test_schema.py` | the one real collision disambiguated (3 methods, +19 lines) |
| `scripts/ci/test_resolved_plan.py` | honest skip messages naming the job that does enforce AC15 |
| `scripts/ci/test_ci_local.py`, `test_runner_loss.py`, `test_build_baseline_revision.py` | ported onto the shared reader |
| `just/ci-local.just`, `.githooks/tests/test-pre-push.sh` | `test_build_key.py` added to all four coupled suite lists |

`REJECTIONS`, `SCOPE_REJECTIONS`, and `.github/ci/schemas/contract.json` were
verified **byte-identical** (contract SHA-256 `d5a5f61c…` before and after
regeneration — confirmed a no-op). No workflow YAML was edited by spike 1, which
reads workflows but does not modify them.

## Fidelity and what could not be measured here

- **No hosted runner was measured for build cost.** Cold figures are a faithful
  *lower bound* from this host; they cannot correct for per-core speed.
  Single-factor scaling puts `test_build_key.py` near **40–70s** on
  `ubuntu-latest` — an estimate, explicitly not a measurement. Spike 3's ruling
  does not depend on it, because the chosen step position costs 0.1s on either
  side of the 30s boundary; only the bucket *label* would change.
- **`ci-tooling` with sniff installed was never run**, so its post-change wall
  time (estimated 12.5–18min cold) is unverified. That estimate is what moved
  the work off the merge path.
- **`area-drift.yml` has never executed.** The Linux build is unproven: sniff
  declares no `[package.metadata.ci.native]`, so the workflow adds no apt step,
  while `sniff-performance.yml` installs `pkg-config libssl-dev libgit2-dev`
  anyway. That is the first thing to try on failure.
- **PyYAML availability on `ubuntu-latest` and `windows-latest`** was not
  measured; it would require editing and pushing a workflow. Moot for the
  ruling, which turns on the developer hosts the pre-push hook targets.

Per the repository's `CI/CD Test-scope Discipline`, no speculative CI run was
dispatched to obtain any of the above.

## Human review — resolved 2026-09-15

Both open questions were ruled on by the author the same day, and both rulings
are implemented in this branch.

### 1. Area-drift enforcement — ruled: put it on the merge path

The question was whether a nightly nobody watches is adequate enforcement for
the contract that keeps the planner's area derivation honest. **It is not.** The
ruling adds an `area_drift` flag to the planner and a job in `ci.yml` that builds
`sniff-cli`, runs the contract class with the requirement declared, and is folded
by `ci-gate` like every other blocking job. No second required status context was
introduced, and no policy was added to the gate — `skipped` already passes the
fold, so a pull request that cannot cause drift pays nothing.

Two consequences worth recording:

- **The flag follows any `Cargo.toml` at any depth**, not just `sniff/**` and the
  two planner files. An area is a function of *where* a manifest sits, and the
  planner's only input is `git diff --name-only` with no add/move/delete status,
  so "a manifest moved" is not separable from "a dependency was bumped" without
  a second Git read that `--all` runs have no base for. Over-triggering costs one
  parallel job the gate folds; the miss it prevents is silent. Fixture manifests
  are included deliberately — a manifest in an unusual location is precisely the
  input that separates sniff's rule from the planner's.
- **`area-drift.yml` lost its `pull_request` trigger** and kept its schedule and
  dispatch. Running both on one pull request is the duplication the repository's
  CI/CD test-scope discipline warns against. The nightly remains as the backstop
  for the gate's one structural gap: the planner schedules the job that validates
  the planner, so a scope defect could skip its own check.

The rejected alternative, for the record: installing sniff in `ci-tooling` costs
1m39s warm on a job that already runs ~6 minutes and fires on every `scripts/` or
`.github/` change, and it reddens the CI-infrastructure gate for a sniff compile
error.

### 2. The unreachable artifact-collision refusal — ruled: delete

Deleted, with the `artifacts` dict that existed only to feed it (8 lines). The
property it appeared to protect — one artifact, one build record — is enforced by
construction: the name is derived from `{package, producer, key}` rather than
supplied, and key uniqueness is checked upstream. That is a stronger guarantee
than a runtime check, not a weaker one.

The misnamed test is now
`test_an_artifact_that_copies_a_sibling_records_name_is_refused` and asserts the
exact mismatch message it actually exercises, with its deliberate overlap with
`test_an_artifact_that_is_not_package_keyed_is_refused` named in the comment.

## Next steps

All three follow-ups this review opened are **done**, in this branch.

1. ~~Pin the unasserted plan-validation rules.~~ **Done — 23 unpinned to zero**,
   via 23 new tests. Verified by mutation rather than inspection: an AST-tagged
   copy of `schema.py` suppressed each of the 54 reachable emission sites in turn
   and confirmed every one fails a named test. That sweep also found **two rules
   the spike had recorded as pinned but which were not** — `schema.py:696` was
   satisfied by a build record's message through a `startswith` assertion, and
   `:677` was pinned only indirectly from another suite. Both now have exact
   tests. Every new assertion compares full messages rather than substrings, so
   the coverage fix introduces no new collisions.
2. ~~Prove the area-drift contract runs.~~ **Superseded by ruling 1**: it is now
   a merge-path job, so the pull request for this branch exercises it directly.
   `workflow_dispatch` would not have proven much from a feature branch, since
   GitHub only surfaces it once the workflow is on the default branch.
3. ~~Implement `require_tool()` with per-job scoping.~~ **Done** —
   `scripts/ci/tool_guard.py`, applied to all 13 tool guards. `enforced_by` is
   keyword-only with no default, so a guard that cannot name where its contract
   runs will not construct. `ArchiveInventoryClosureTests` now fails on a planner
   that cannot resolve a full-scope plan instead of reporting the class green.

Remaining, unaffected by the spikes and able to proceed independently: extending
`plan_fixtures.py` to own the whole plan document, extracting `CiLocalSandbox`,
and adding `ruff`. Spike 1 removed 115 lines of the duplication the source review
identified in the workflow readers; the plan-fixture duplication is untouched.

Two items were explicitly held back as separate concerns: the `conftest.py` /
package restructuring that would let a discovery-based runner import all 13
suites, and the two suites the source review found were executed by nothing
(both are now wired in, but only through enumerated lists).

Nothing in this review blocks the specification's acceptance criteria. The
performance comparison remains deferred exactly as Review 3 and
`deferred-performance-measurements.md` record it; no spike produced hosted
measurements, and none was chartered to.

## Verification performed

Run after all four spikes and the suite-list wiring, on macOS:

- All 13 Python suites individually: **611 tests, 0 failures, 0 skips**.
- `.githooks/tests/test-pre-push.sh`: **66 passed, 0 failed**.
- `scripts/ci/test_ci_local.py` after the four-list edit: **66 passed**.
- `cargo nextest run -p test-toolkit --test ci_workflow_contracts`: **119 passed**.
- `actionlint` on `ci.yml` and `area-drift.yml`: clean.
- `test_build_key.py` with `BISCUIT_CI_BUILD_BIN` set: **12/12**; with no binary
  and no `cargo`: 4 pass, 8 fail loudly, **nothing hangs**.
- `test_resolved_plan.py`: 64 passed with sniff present; with sniff off `PATH`,
  61 passed and 3 skipped with messages naming `area-drift`; with
  `BISCUIT_REQUIRE_SNIFF=1` and sniff absent, 3 **failures** — the guard works
  in both directions.
- Spike 1's three ported suites under `/usr/bin/python3` 3.9.6: 66 + 43 + 11.
- The `scripts/target` cache moved aside for spike 3's cold measurement was
  **restored and confirmed** (7.7G, original mtime, warm suite green). No other
  build cache was moved; scratch build directories were deleted.

Nothing was committed.
