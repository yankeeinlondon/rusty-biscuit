---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_review_items: []
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-11-10T21:21:04-08:00"
spec: 2026-11-06-legible-ci-names/spec.md
implemented: false
description: 'A **feature** review of `2026-11-06-legible-ci-names/spec.md`'
feature: 2026-11-06-legible-ci-names/review-2.md
previous: 2026-11-06-legible-ci-names/review-1.md
---

# Review 2

The feature is production ready within the spec's approved fallback. All three previous findings are verified fixed, including every child instance in the repair ledger. No partial repair, regression, newly found sibling, or blocked finding remains.

The review checked the spec and its author decisions, review-1.md, the repair log's Finding Ledger, the original implementation commit, the current repair diff, and the naming evidence. The previous review contains no Human rulings section. Its informational implementation decisions do not need reaffirmation.

## Previous Finding Verification

Every row below has outcome **verified fixed**; a previously clean instance means its required behavior was checked again and remains correct. Nothing remains in any row. Child identifiers correspond to the repair log's Finding Ledger.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: Release preparation rejects the renamed CI workflow | R1-F1.1–R1-F1.11, detailed below | verified fixed | Selector, fixture, triggers, lookup consumers, and release documentation checked; all 228 workflow contracts pass. | Nothing |
| R1-F1.1: Release selector | Successful main-push event with current title | verified fixed | `release_path.py::select_preparation` compares with `CI_WORKFLOW_TITLE = "Continuous integration"`; the in-process release fixture runs through the passing Rust contract suite. | Nothing |
| R1-F1.2: Positive fixture and rename guard | Shipped title, unrelated title, historical title | verified fixed | Driver reads `ci.yml`'s title; negative cases retain rejection checks. `release_preparation_accepts_the_ci_workflow_title` ties the selector constant and trigger to that title. | Nothing |
| R1-F1.3: Release workflow | Trigger and downstream selection guards | verified fixed | `release-plz.yml` waits for the current title; guards preserve successful push, main branch, and canonical repository requirements. Positive selection reaches the existing output contract. | Nothing |
| R1-F1.4: Infrastructure retry | Renamed trigger and retry identity | verified fixed | `ci-infra-retry.yml` waits for the current title and uses run ID and attempt for retry decisions. Reader tests pass. | Nothing |
| R1-F1.5: Other workflow lookups | Reuse, hosted observations, distribution | verified fixed | Lookups use workflow file paths or references, not the changed title; reuse suite passes. | Nothing |
| R1-F1.6: Developer run listings | Just recipes and repository instructions | verified fixed | `gh run list --workflow ci.yml` resolves the file name. | Nothing |
| R1-F1.7: Producer workflow references | Native package and WSL guest publication | verified fixed | Both producers pass `GITHUB_WORKFLOW_REF`, preserving file-based identity. | Nothing |
| R1-F1.8: Remaining run readers | Slow-test tooling, runner loss, pre-push hook | verified fixed | These consumers identify runs by run ID/attempt rather than workflow title. | Nothing |
| R1-F1.9: Sniff fixtures | Third-party workflow title | verified fixed | `workflow_name: CI` belongs to an unrelated fixture repository; it is not a title comparison for this pipeline. | Nothing |
| R1-F1.10: Generic release documentation | Example `workflows: ["ci"]` | verified fixed | Tool-level examples describe the reader's own workflow title; they do not configure this repository. | Nothing |
| R1-F1.11: Repository release guide | Current title references | verified fixed | `docs/releases.md` now identifies Continuous integration. | Nothing |
| R1-F2: The naming fallback lacks the required feasibility evidence | R1-F2.1–R1-F2.19, detailed below | verified fixed | Live GitHub jobs and check-run APIs match the saved 33 names; observed truncation establishes the author-approved fallback condition. | Nothing |
| R1-F2.1: Native matrix | Leading label, stored identity, unlabeled control | verified fixed | Display variants show the label leads but truncates the runner for `rendezvous-core`; unlabeled control retains all four values. | Nothing |
| R1-F2.2: WSL2 nesting | Labeled delegation through guest workflow | verified fixed | Saved and live names show `wsl2 (…) / Run guest tests`; static labeled delegation truncates identity. | Nothing |
| R1-F2.3: Distinct test levels | One package, Level 1 and Level 2 | verified fixed | Observed `claudine-cli` names distinguish L1/L2 in the automatic unlabeled variant; workflow projection and reader tests pass. | Nothing |
| R1-F2.4: Nested area and empty selection | `claudine/rendezvous`, empty `tools` rows | verified fixed | Recorded names preserve the nested directory; skipped variants retain static text. None of the 33 live names contains a raw expression. | Nothing |
| R1-F2.5: Both naming surfaces | Actions jobs and commit check runs | verified fixed | Independently fetched both GitHub API lists: 33 records each, names equal to their saved records. Check-run records are the documented substitute for a pull-request checks list; no new production run was needed. | Nothing |
| R1-F2.6: Accepted-gap publisher | Gate/environment wording and unchanged payload fields | verified fixed | All 29 publisher tests pass, including public payload checks; only display-name construction changes the successful payload. | Nothing |
| R1-F2.7: Package workflow comment | Static name and matrix suffix | verified fixed | Comment now correctly says a literal keeps the row and changes the job-id prefix; it explains truncation. | Nothing |
| R1-F2.8: Area fan-out comment | Static name and area suffix | verified fixed | Correctly states a static name retains the expanded area; keeping the automatic spelling is a compatibility choice. | Nothing |
| R1-F2.9: Preflight comment | Expanded static matrix name | verified fixed | Correctly shows `Verify build prerequisites (ubuntu-latest)`. | Nothing |
| R1-F2.10: Nightly certification comment | Expanded static matrix name | verified fixed | Correctly shows `Publish nightly coverage evidence (wsl2-ubuntu)`. | Nothing |
| R1-F2.11: Current CI documentation | Rendering, fallback, environments, skipped states | verified fixed | Topic page matches observed suffix behavior and explains truncation, test levels, WSL2, neutral gaps, reuse, and missing results. | Nothing |
| R1-F2.12: Rust contract comments | Static and automatic matrix names | verified fixed | Comments now match observed rendering; all 228 contracts pass. | Nothing |
| R1-F2.13: Python name model | Static named matrix and declared matrix keys | verified fixed | `job_label` appends the suffix for a literal name; new observed-name control and shipped static-matrix cases pass in the 74-test reader suite. | Nothing |
| R1-F2.14: Readers of static matrix jobs | Preflight and nightly certification, expanded/skipped | verified fixed | Tests verify no producer cell or build owner is inferred, and neither is incorrectly ignored as an advisory judge. | Nothing |
| R1-F2.15: Audit and exact-name consumers | Renamed/historical audit names and matrix suffixes | verified fixed | Audit matcher accepts both names; 52 reuse tests pass. Exact-name targets are non-matrix jobs. | Nothing |
| R1-F2.16: Empty-matrix test comment | Automatic area suffix | verified fixed | Comment corrected; all five EmptyMatrixGuardTests pass. | Nothing |
| R1-F2.17: CI skill guidance | Rendering rule and fallback justification | verified fixed | Guidance now cites observed truncation rather than the false claim that static names suppress values. | Nothing |
| R1-F2.18: CI overview and declared-name lists | Declared names versus expanded names | verified fixed | Lists describe declared names without claiming their matrix suffix disappears. | Nothing |
| R1-F2.19: Guest-name comment | Non-matrix guest job | verified fixed | Guest job has no matrix; its static name carries no identity, so attribution correctly comes from the enclosing row. | Nothing |
| R1-F3: The provisioning test fails on unrelated diagnostic steps | R1-F3.1–R1-F3.8, detailed below | verified fixed | All four NativeProvisioningTests pass against the shipped workflow; restricted reader remains restricted. | Nothing |
| R1-F3.1: Before-cache diagnostic | Folded run block before prerequisite assertions | verified fixed | Focused named-step read bypasses this unrelated block; build-owner prerequisite test passes. | Nothing |
| R1-F3.2: After-cache diagnostic | Same | verified fixed | Same passing test uses the complete unchanged workflow containing this block. | Nothing |
| R1-F3.3: Before-guard diagnostic | Same | verified fixed | Same passing test; block does not enter the named prerequisite script. | Nothing |
| R1-F3.4: After-guard diagnostic | Same | verified fixed | Same passing test; no production command changed. | Nothing |
| R1-F3.5: Before-publication diagnostic | Same | verified fixed | Same passing test; no reader expansion was introduced. | Nothing |
| R1-F3.6: After-publication diagnostic | Same | verified fixed | Same passing test; all six folded blocks remain present. | Nothing |
| R1-F3.7: Other whole-job callers | Scope, preflight, gate, native check/test/lint, WSL guest, area audit | verified fixed | Called public `job_run_steps` for all eight jobs; all succeed. Whole-build read still rejects the first folded diagnostic, as intended. | Nothing |
| R1-F3.8: Named prerequisite reader | Unique prerequisite step in build job | verified fixed | `step_script` returns the intended script; `provision_owner` uses it and retains both prerequisite-list assertions. | Nothing |

## Unblocked Findings

None.

## Blocked Findings

None. No decision is needed before repair or release review.

## Spec coverage and verification

| Requirement | Appropriate verification | Evidence and outcome |
|---|---|---|
| Inventory and preserve existing readers | Level 1 public-result and workflow contracts | Current, historical, and delegated WSL2 names, advisory classification, audit discovery, title triggers, and release selection pass. |
| Apply feasible templates without changing execution | Source comparison plus GitHub rendering evidence | Compared all four workflows with the parent of implementation commit `d46efacca`, excluding presentation names, full-line comments, and blank lines: all remaining content is identical. IDs, commands, guards, matrices, permissions, artifacts, and dependencies remain intact. |
| Explain names and skipped states | Documentation inspection | CI topic page defines environments and levels, describes retained prefixes, distinguishes reuse/deferred/gap/missing states, and explains compilation of unchanged dependents. |
| Cover examples, empty matrices, levels, and required gate | Level 1 contracts plus bounded service check | 228 workflow contracts pass; observed native, nested, WSL2-shaped, two-level, and empty-selection examples corroborate the templates. `ci-gate` remains the exact required context. |
| Preserve grandfathered readers | Level 1 public-result tests | Runner-loss suite: 74 passed. Reuse suite: 52 passed. No label field ships under the approved fallback, so label validation is not a new planner obligation. |
| Publish truthful accepted-gap checks | Level 1 publisher payload tests | 29 passed; neutral conclusion, governance content, identities, and explanation remain intact. |
| Repair focused provisioning verification | Level 1 subprocess tests | NativeProvisioningTests plus EmptyMatrixGuardTests: 9 passed in about three seconds. |

Commands rerun during this review:

- `cargo nextest run -p test-toolkit --test ci_workflow_contracts`: 228 passed, zero skipped, about 6.5 seconds total; no individual test exceeded five seconds.
- `python3 scripts/ci/test_runner_loss.py`: 74 passed.
- `python3 scripts/ci/test_reuse_validation.py`: 52 passed.
- `python3 scripts/ci/test_publish_gaps.py`: 29 passed.
- `python3 scripts/ci/test_ci_local.py NativeProvisioningTests EmptyMatrixGuardTests`: 9 passed.

The Rust tests are compiled in the existing integration-test target and have ordinary Level 1 names. The Python suites remain selected by the existing repository companion-suite wiring. The focused reruns are sufficient for this repair; the implementation log's full-suite claims were not used as substitute evidence.

The optional YAML structure script could not run because this session lacks PyYAML. The direct protected-content comparison and workflow diff inspection supplied an independent check without installing dependencies. GitHub's live jobs and check-run records for [display run 42963148323](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42963148323) matched the saved evidence, including five truncated names and no raw expressions. This corroborates the approved fallback; it does not claim a production run or an actual pull request displayed the renamed workflow.

No configuration loader or field validator was added or changed. The input robustness matrix therefore does not apply: the changed publisher builds output, the grandfathered readers consume display text, and the release selector changes one accepted title. Existing positive and negative event cases exercise that selector through its public result.

No terminal rendering, keyboard handling, or browser interaction changed. Level 2 or Level 3 terminal tests would not verify GitHub's naming behavior; the bounded GitHub service check supplies that evidence. Cross-OS execution remains CI's responsibility and does not block readiness.

## Recurrence

None. The earlier defect classes are repaired throughout the required instances; no unchanged instance, repair regression, or missed sibling was found.
