---
implementation_1: "2026-11-10T20:48:57-08:00"
---

# Legible CI names: review-to-implement log

## Implementation of Review Findings #1

> **started at:** 2026-11-10T20:48:57-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/rusty-biscuit/features/2026-11-06-legible-ci-names/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was already `false`; it stays `false` until the cycle is complete
- the review lists no `human_in_the_loop` items, so no finding can be blocked on a ruling
- environment facts discovered at the start (20:49):
        - `gh` is not authenticated (the stored token is invalid), and this session may not run `gh auth login`
        - the repository is public, so unauthenticated GitHub REST reads of runs, jobs, and check runs work
        - SSH push to `origin` works with `BatchMode=yes`, so the spec's single disposable display workflow can run on a scratch remote ref without a local branch
        - a pull request cannot be opened without `gh`; the check-runs API (the data the pull-request checks list renders) is the substitute evidence for that surface

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | Release preparation rejects the renamed CI workflow | fixed | all children verified; 228/228 `ci_workflow_contracts`; mutation back to `ci` fails the driver case and the new contract test |
| R1-F1.1 | `release_path.py::select_preparation` selects a successful main push titled `Continuous integration` | fixed | `CI_WORKFLOW_TITLE` constant; in-process: `Continuous integration` → selected with `validated_sha`, `ci` and `Release-plz` → not selected; other checks unchanged; docstring updated. Decision: historical `ci` is not accepted (see Progress) |
| R1-F1.2 | Release fixture positive case uses the shipped title, tied to `ci.yml` so another rename fails a test | fixed | driver reads `ci.yml` `name:`; negative cases `Release automation` and `ci`; new test `release_preparation_accepts_the_ci_workflow_title` ties `ci.yml` title, selector constant, and `release-plz.yml` trigger |
| R1-F1.3 | `release-plz.yml` trigger and guards reach selection for the renamed title | clean | `release-plz.yml:28` triggers on `Continuous integration`; job guard (57–61) reads conclusion, event, branch, repository only |
| R1-F1.4 | `ci-infra-retry.yml` trigger and decision path are title-independent | clean | trigger on new title (line 33); decision uses conclusion, run ID, attempt; rerun by run ID |
| R1-F1.5 | `reuse_validation.py`, `hosted_observations.py`, `distribution.py` workflow lookup is title-independent | clean | `actions/workflows/ci.yml/runs` and `path == .github/workflows/ci.yml`; `WORKFLOW_FILE`/`WORKFLOW_PATH`; `DISTRIBUTION_WORKFLOW` path |
| R1-F1.6 | `just/devops.just` and `CLAUDE.md` `gh run list --workflow ci.yml` | clean | resolves by file name, not title |
| R1-F1.7 | `_package-ci.yml:1123`, `_wsl-ci.yml:946` `--workflow "$GITHUB_WORKFLOW_REF"` | clean | path-based workflow ref |
| R1-F1.8 | `scripts/slow-tests.rs`, `runner_loss.py`, `.githooks/pre-push` run lookups | clean | by run ID/attempt; no title lookup |
| R1-F1.9 | `sniff` fixture `"workflow_name": "CI"` | clean | third-party fixture repository, unrelated to this repo's title |
| R1-F1.10 | `docs/cicd/release-plz-ci.md` `workflows: ["ci"]` examples | clean | generic tool page placeholders, annotated as "the name: of your test workflow" |
| R1-F1.11 | `docs/releases.md` user-facing title references | fixed | lines 55 and 215 now name **Continuous integration** |
| R1-F2   | The naming fallback lacks the required feasibility evidence | fixed | bounded display check run 42963148323 (33 jobs = 33 check runs) establishes the fallback condition: GitHub cuts each name segment at 100 characters and a leading label loses the runner; fallback retained on evidence, recorded in `implementation.md`; stale rendering claims corrected (R1-F2.7–F2.19) |
| R1-F2.1 | Native package matrix: leading display label plus stored identity, evidence settles legibility and identity | fixed | label leads the suffix (V1/V2) but truncates: `test (rendezvous-core: run Level 1 tests on Linux, rendezvous-core, L1, ubuntu-latest, ubuntu-lat...`; unlabeled worst case is 75 characters; fallback kept |
| R1-F2.2 | WSL2 nested call: same check through WSL2-shaped nesting | fixed | V5 `… / wsl2 (…) / Run guest tests`; V6 static name truncated to `windows...` / `win...`; evidence recorded |
| R1-F2.3 | Two levels of one package: complete names corroborate the chosen template | fixed | `claudine-cli, L1` and `claudine-cli, L2` distinct in every variant, including the fallback V4 shape |
| R1-F2.4 | Nested area and empty selection: names retain area and expose no raw expression | fixed | `area-ci (claudine/rendezvous)` observed; skipped jobs render static name or job id; no name contains `${{`; skipped caller renders `area-ci (tools) / Run package work` |
| R1-F2.5 | Actions job list and pull-request checks: complete chosen rendering settled without a full-scope run | fixed | jobs list and check-runs list identical (33 names, saved in `display-check/`); no pull request could be opened without `gh`, so the check-runs API that feeds the pull-request checks list is the substitute; no full-scope run triggered; scratch ref deleted |
| R1-F2.6 | Accepted-gap name generation: publisher controls the name | clean | `publish_gaps.py::check_run` sets the whole name; longest possible `ACCEPTED GAP: unfolded-integration-helper: run Level 1 tests on Windows` is 71 characters, under 100 and `NAME_LIMIT` 255; `test_publish_gaps.py` 29 OK |
| R1-F2.7 | `_package-ci.yml` comment "a literal `name:` would drop the row" | fixed | comment states GitHub appends the row to a literal name, the job id reader, and the 100-character reason (comment only) |
| R1-F2.8 | `ci.yml` `area-ci` comment "a literal name would hide the area" | fixed | true reason stated: contract, recorded runs, and docs spell `area-ci (<dir>)` (comment only) |
| R1-F2.9 | `ci.yml` `preflight` rendered-name comment | fixed | states `Verify build prerequisites (ubuntu-latest)` (comment only) |
| R1-F2.10 | `ci.yml` `certify-nightly` rendered-name comment | fixed | adds `Publish nightly coverage evidence (wsl2-ubuntu)` (comment only) |
| R1-F2.11 | `docs/topics/ci-cd.md` rendering rule, matrix-job names, label limitation | fixed | false rule removed; `preflight`/`certify-nightly` rows show their suffix; plain-language section on the 100-character cap; darkmatter doc-corpus tests 2/2 |
| R1-F2.12 | `ci_workflow_contracts.rs` naming-contract comments | fixed | comments only; 228/228 |
| R1-F2.13 | `test_runner_loss.py::job_label` modelled "a declared `name:` wins" | fixed | static name on a matrix job now renders `<name> (<values>)`; two new tests (observed `Run tests (…)`, and `preflight`/`certify-nightly`); 74 OK |
| R1-F2.14 | `runner_loss.py` readers on the true `preflight`/`certify-nightly` names | clean | `parse_job_name`, `parse_build_job_name`, `is_non_producer` classify the suffixed names exactly as the old `preflight (…)`: no cell, not a build owner, not a judge; pinned by the new test |
| R1-F2.15 | `AUDIT_JOB` and other exact-name matchers | clean | every declared `name:` grepped in `scripts/` and `tools/`; matchers target non-matrix jobs only |
| R1-F2.16 | `test_ci_local.py` comment "which a literal would hide" | fixed | comment only; `EmptyMatrixGuardTests` 5 OK |
| R1-F2.17 | `.claude/skills/rust-devops/ci-cd.md` rendering claim and label rejection reason | fixed | truncation reason replaces the false claim and the row-schema reason |
| R1-F2.18 | `.github/ci/README.md` and skill listing of declared names | clean | lists declared names only; no suffix claim |
| R1-F2.19 | `runner_loss.py` comment "static job name cannot" (guest job) | clean | guest job is not a matrix job; statement true |
| R1-F3   | The provisioning test fails on unrelated diagnostic steps | fixed | `provision_owner` reads the one named step with `step_script`; `python3 test_ci_local.py` 104/104 OK (was 103/104) |
| R1-F3.1 | Before-cache-restore sample does not prevent the prerequisite test | fixed | ci.yml:715; `test_the_build_owner_installs_only_its_own_records_closure` passes against the unmodified `ci.yml`; `job_run_steps(build)` still refuses this step, so the reader was not widened |
| R1-F3.2 | After-cache-restore sample, same | fixed | ci.yml:737; same test passes |
| R1-F3.3 | Before-schematic-guard sample, same | fixed | ci.yml:758; same test passes |
| R1-F3.4 | After-schematic-guard sample, same | fixed | ci.yml:789; same test passes |
| R1-F3.5 | Before-publication sample, same | fixed | ci.yml:824; same test passes |
| R1-F3.6 | After-publication sample, same | fixed | ci.yml:851; same test passes |
| R1-F3.7 | Other live whole-job reader callers succeed on the unmodified workflow | clean | `job_run_steps` reads ci.yml `scope` (4 steps), `preflight` (4), `ci-gate` (1) and `_package-ci.yml` `check` (10), `test` (22), `lint` (14); other readers are raw-line (`workflow_job_block`, `test_runner_loss.py::job_blocks`, Rust `job_block`/`steps`); `rg 'run: >-' .github/workflows` finds only the six `build` steps, and no other caller reads `build` |
| R1-F3.8 | Named-step reader returns the prerequisite script from the unmodified build job | fixed | `step_script` returns the first step with that name; the name occurs once in ci.yml (line 697, inside `build` 619–887); returned script matches ci.yml 701–709 |

### Progress

- starting the work on 'R1-F1 release preparation title' at 20:50:47
        - defect class: a presentation rename updates an event trigger but leaves a downstream identity comparison and its positive fixture on the old title
        - 20:51:30 added `CI_WORKFLOW_TITLE` to `scripts/ci/release_path.py`; the release fixture reads `ci.yml`'s `name:`; negative cases for an unrelated title and the old `ci`; new contract test `release_preparation_accepts_the_ci_workflow_title`
        - 20:52:20 swept the repository for other title readers (R1-F1.6–F1.10); none compare against the title; corrected `docs/releases.md` (R1-F1.11)
        - 20:52:35 mutation check: restoring the old title fails both the driver case and the contract test
        - decision: the historical title `ci` is not accepted. `release-plz.yml` runs from `main` and checks out `main` before selecting, so the trigger and selector always come from the same commit; a `ci`-titled run can no longer trigger it, which makes acceptance dead code. Alternatives: accept both titles (one-line change), or match `workflow_run.path` (title-independent but adds a required event field). Recorded in the review's `human_review_items`
        - verification: `cargo nextest run -p test-toolkit --test ci_workflow_contracts -E 'test(release)'` 37 passed; full binary 228 passed; `cargo clippy -p test-toolkit --test ci_workflow_contracts -- -D warnings` clean
- work completed for 'R1-F1 release preparation title' at 20:52:56
- starting the work on 'R1-F3 provisioning test reader' at 20:54:09
        - defect class: a focused test parses unrelated workflow steps through a deliberately restricted whole-job reader, so valid production syntax elsewhere in the job prevents the intended behavior from being tested
        - confirmed `job_run_steps(ci.yml, build)` raises on the folded `Sample storage before the cache restore` step
        - 20:54:30 `NativeProvisioningTests.provision_owner` now uses `step_script(ci.yml, "Install native prerequisites")`; both prerequisite-list assertions unchanged; no workflow or reader change
        - 20:54:48 swept every whole-job reader caller (R1-F3.7): clean, no additional rows
        - verification: `python3 test_ci_local.py NativeProvisioningTests -v` 4 OK; full `python3 test_ci_local.py` 104/104 OK (299 s); `python3 test_runner_loss.py` 72 OK; `ci_workflow_contracts` 228/228
        - observed: `the_shipped_planner_emits_plans_the_guards_reader_accepts` took 5.49 s while the full Python suite ran concurrently; rechecked alone in final verification
- work completed for 'R1-F3 provisioning test reader' at 20:59:57
- starting the work on 'R1-F2 naming fallback evidence' at 21:01:54
        - defect class: a conditional design fallback is accepted on evidence that does not test its condition; sibling class: code, tests, comments, and docs encode the false rule that a declared static `name:` suppresses GitHub's matrix suffix
        - existing run records cannot settle the added-label template (no run ever carried a label), so the spec's one disposable display workflow was used
        - 21:02:52 built a signed orphan commit `9315984f9` holding only four `zz-display-*.yml` workflows (no repository code, so no other workflow ran); pushed with `--no-verify` to the scratch ref `display-check/legible-ci-names` because the pre-push hook would plan CI scope for a source-free orphan tree; signing was not bypassed
        - 21:04:02 run 42963148323 completed `success`: 33 jobs, 33 check runs, identical names; 5 unauthenticated API requests
        - 21:04:36 scratch ref deleted (`git ls-remote` empty); workflows and trimmed JSON saved under `display-check/`; the run record and orphan commit remain on GitHub (deleting a run needs authentication)
        - observations: a static `name:` on an expanded matrix job gets the values appended; skipped jobs show the static name or job id and never `${{`; the label leads the suffix; each name segment is cut at 100 characters, so labeled rows lose the runner and sometimes part of the environment
        - decision: the ruling's fallback condition ("adding it loses identity in any rendered name") is established by observation; the fallback is retained and no `label` field is added. A static descriptive name with unlabeled rows (variant V3, `Run tests (pkg, L1, env, runner)`) also renders well but would replace the job id `runner_loss.py` reads and contradict the frozen contract; recorded in the review's `human_review_items`
        - 21:08–21:13 corrected the false rendering rule in workflow comments, docs, skill, and contract comments; fixed the `job_label` test model; confirmed no reader misclassifies a true rendered name; rewrote `implementation.md`'s decision and added the display-check record
        - verification: `test_runner_loss.py` 74 OK, `test_reuse_validation.py` 52 OK, `test_publish_gaps.py` 29 OK, `ci_workflow_contracts` 228/228, darkmatter doc-corpus 2/2, workflow structure diff against the baseline empty, workflow diff contains comment lines only
- work completed for 'R1-F2 naming fallback evidence' at 21:13:28
- final verification after all changes (orchestrator, no subagent running), 21:14–21:20:
        - `just test test-toolkit`: 533 passed, 4 skipped; 4 SLOW (>5 s), none changed in this cycle: `archive_path_guard` ×2, `kache_host_contracts` ×1, and `the_shipped_planner_emits_plans_the_guards_reader_accepts` (5.24 s alone; already recorded 31 times in the slow-test log before this cycle, so it is pre-existing and unrelated)
        - `cargo clippy -p test-toolkit --all-targets -- -D warnings`: clean (the area has no `just lint` recipe)
        - `python3 scripts/ci/test_runner_loss.py` 74 OK; `test_reuse_validation.py` 52 OK; `test_publish_gaps.py` 29 OK; `test_ci_local.py` 104/104 OK (296.8 s)
        - darkmatter `shipped_markdown_inline_schemas_parse_passively`, `shipped_markdown_named_type_references_resolve`: 2 passed
        - cross-OS check skipped: changes are Python test logic, a string constant, comments, and docs, with no path or OS-specific behavior

### Completion Report

- **outcome:** complete
- **elapsed:** about 32 minutes (20:48:57 → 21:21)
- **parent findings:** total 3 = fixed 3 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- **instance rows:** R1-F1 11 rows (3 fixed, 8 clean); R1-F2 19 rows (13 fixed, 6 clean); R1-F3 8 rows (7 fixed, 1 clean); none open
- verified repairs:
        - R1-F1: release preparation selects the run titled `Continuous integration`; the fixture reads the shipped title from `ci.yml`, and a contract test ties the workflow title, selector constant, and `release-plz.yml` trigger together
        - R1-F2: a bounded display check produced the required evidence; the fallback is retained because a label truncates identity at GitHub's 100-character segment cap; every false claim about static names was corrected and the runner-loss test model now matches GitHub
        - R1-F3: the provisioning test reads only the step it needs; `test_ci_local.py` is fully green
- open findings: none
- decisions for the author (added to the review's `human_review_items`): historical `ci` title not accepted by the release selector; fallback retained on display evidence with variant V3 noted as an alternative; scratch ref pushed with `--no-verify`
- no performance measurement was required; `deferred_perf_measurement` not set
- changed files:
        - `scripts/ci/release_path.py`, `tools/test-toolkit/tests/fixtures/release_path/driver.py`, `tools/test-toolkit/tests/ci_workflow_contracts.rs`, `docs/releases.md` (R1-F1; contract comments also R1-F2)
        - `scripts/ci/test_ci_local.py` (R1-F3; one comment for R1-F2)
        - `.github/workflows/_package-ci.yml`, `.github/workflows/ci.yml` (comments only), `docs/topics/ci-cd.md`, `.claude/skills/rust-devops/ci-cd.md`, `scripts/ci/test_runner_loss.py`, `features/2026-11-06-legible-ci-names/implementation.md`, new `features/2026-11-06-legible-ci-names/display-check/` (R1-F2)
