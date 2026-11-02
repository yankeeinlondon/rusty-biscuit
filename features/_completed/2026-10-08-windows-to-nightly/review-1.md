---
$schema: feature-review.yaml
ready: false
findings:
  - "[high] Publication order can turn an older pass into a retry of a newer failure"
  - "[high] Nightly anchors inherit coverage without checking the required cell contract"
  - "[high] Completed hosted observations depend on a later workflow-wide publisher"
observations:
  - Existing source-input audit repeatedly scans the checkout
human_review: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-11T18:54:46-07:00"
spec: 2026-10-08-windows-to-nightly/spec.md
implemented: true
next: 2026-10-08-windows-to-nightly/review-2.md
implemented_by: claude/opus
log: features/2026-10-08-windows-to-nightly/log.md
description: "A **feature** review of `2026-10-08-windows-to-nightly/spec.md`"
feature: 2026-10-08-windows-to-nightly/review-1.md
---

# Review: Windows to Nightly

The feature is **not production ready**. The event cadence is implemented, but three defects affect which cells the nightly considers proved or repeats after interruption. A separate medium finding concerns seeding when policy requires a tier beyond Level 1.

A cell is one package's required check or test tier on one environment. Reuse means a prior qualifying result satisfies that cell, so CI does not execute it again.

The review compares implementation commits through `c30690c79` with `bc0d9a09d`, reads the specification and implementation log, and checks the changed planner, evidence readers and publishers, seed transport, workflow wiring, reporting, and tests. The optional Windows-risk selection interface and release enforcement are explicitly deferred and are not missing implementation findings. Cross-OS execution evidence and human sign-off do not determine this review's readiness.

## Findings

### High — Publication order can turn an older pass into a retry of a newer failure

**Authority: spec.** Evidence integrity requires that “a concurrent pass must not silently erase a failure for equivalent inputs,” and the selected precedence design in the implementation log says “Timestamps, fetch order, and publication order are never used.” The acceptance criteria require fixtures for “unordered conflicting outcomes” and require that “a newer failure cannot be hidden by older passing evidence.”

**Defect class:** publication invents execution lineage from the order in which observations reach the local notes collection.

In the `repo-deps` package, [merge_observations](../../scripts/ci/observation_notes.py:196), which combines independently produced cell results, automatically adds `retry_of` whenever exactly one existing observation has no successor. [publish](../../scripts/ci/observation_notes.py:338) enables this by default. It checks neither that the incoming execution started after the existing one nor that it actually retried it. The verifier correctly honors explicit lineage, so the invented link makes the incoming pass suppress the existing failure.

A realistic sequence is an earlier passing run whose publication was delayed, followed by a failing run at the same commit, followed by recovery/publication of the earlier pass. This is particularly plausible with the newly added hosted recovery command and separate rig runs. The reproduction generated the pass first, published the failure first, then published the old pass. Public verification changed from no credit to a reused passing cell.

| Site swept | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Hook observations through `observation_notes.publish` | Same cell/commit, independent failure published before delayed pass | Pass is linked to failure and credited | Unordered conflict; execute |
| Rig observations through the same publisher | Same sequence, retained report and proof documents | Pass is linked to failure and credited | Unordered conflict; execute |
| Seed-kind observations through the same publisher | Same sequence, valid seed identity and retained proof | Pass is linked to failure and credited | Unordered conflict; execute |
| Hosted `emit` → `publish` → `verify_cells` | Passing run `100` generated before failing run `101`; publish `101`, then `100` | `alpha/ubuntu-latest/L1` is credited | Unordered conflict; execute |
| Remote-only collection merge | Same delayed pass, but publishing clone has not previously fetched the failure | Results remain unlinked; no passing credit | Same; clean |
| Verifier with explicit retry links | Passing retry explicitly names the failing observation | Correctly credits retry | Credit verified retry |
| Verifier with unlinked conflicting results | Independent pass/failure stored without an invented link | Correctly requires execution | Execute |

Reproduction used `PublicationTests` in [test_evidence_reuse.py](../../scripts/ci/test_evidence_reuse.py:2483), with its real repository, bare remote, generated observations, and `accepted_keys()` verification. The hosted case used `HostedFixture.finish_cell`, `publish_run`, and `credited` in [test_hosted_observations.py](../../scripts/ci/test_hosted_observations.py:109). Existing tests for explicit links and unordered conflicts pass; they bypass the unsafe inference or assume the second publication really is a retry.

Require execution-established retry lineage. For hosted reruns, the workflow run and attempt can establish a relationship; rig retries need an explicit predecessor known when the retry starts. Leave independent or delayed publications unlinked. Add the delayed-publication case through the publisher and verifier, rather than testing only manually linked collections.

### High — Nightly anchors inherit coverage without checking the required cell contract

**Authority: spec.** Required cadence says “an expired gap, a newly declared gate, or a newly required backend cannot be treated as proved by an older successful baseline” and that “the planner adds newly required coverage even when the source diff is empty.” The acceptance criteria require affected scope “plus any newly required coverage under current policy.”

**Defect class:** baseline inheritance substitutes the current coverage contract for an older contract that was never proved.

In the `repo-deps` package, [apply_policy_delta](../../scripts/ci/affected_scope.py:3967), which adds nightly work the comparison baseline does not cover, compares certified coverage only by package and gate. The [certification_document](../../scripts/ci/reuse_validation.py:442) writer retains only those names in `covered_cells`. Adding a backend to an existing Level-2 gate therefore changes neither identity, and the planner inherits that cell without execution or receipt verification. Changes to the features selecting tests have the same problem. A package manifest edit alone does not select its source suite under the existing scope rules, so the ordinary diff does not rescue this case.

The legacy anchor path is broader: when `covered_cells` is absent by the legacy contract, it assumes the old run covered every cell currently required. A newly required gate is immediately inherited. This defeats the migration safeguard for old WSL2 coverage.

| Site swept | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Certified anchor planner | Unchanged required coverage, empty diff | No cells; prior coverage inherited | Same; clean control |
| Certified anchor planner | Existing Level-2 gate gains a required backend; only package manifest changed | No cells; Level 2 inherited | Select changed coverage, then verify evidence |
| Certified anchor planner | Existing Level-1 gate gains required test features; only package manifest changed | No cells; Level 1 inherited | Select changed coverage, then verify evidence |
| Certified anchor planner | New browser gate absent from `covered_cells`, with browser capability available | New browser cell selected | Same; clean |
| Legacy anchor planner | Same new required browser gate | New WSL2 browser gate inherited, not selected | Select unproved new gate |
| Per-observation verifier | Observation lacks a backend required by an already selected cell | Observation refused | Same; clean, but cannot protect a cell omitted by the planner |

Reproduction used the existing [CoverageAnchorTests](../../scripts/ci/test_affected_scope.py:9852) fixture and its public `calculate_scope` path. The unchanged certified control selected zero cells. Adding `wezterm` alongside `tmux` to the required Level-2 backends also selected zero cells and retained `alpha-core/L2` in WSL2 `inherited_cells`. Adding a required test feature to Level 1 selected zero cells as well. For the newly enabled browser gate, the certified anchor selected the WSL2 cell, while the legacy anchor inherited it. These were valid resolved plans, not malformed configuration inputs.

Preserve enough coverage-contract identity to compare what an anchor proved with what policy now requires. For legacy runs, derive historical required coverage from their committed policy or conservatively select coverage that cannot be established. Keep the correct no-work behavior for unchanged contracts. Test changed contracts within an existing gate as well as newly named gates; the current policy-delta test only exercises a missing package/gate pair.

### High — Completed hosted observations depend on a later workflow-wide publisher

**Authority: spec.** Evidence integrity requires “Hosted producers publish proof as each cell completes, before area-wide or workflow-wide success is known.” Nightly recovery says “Run artifacts alone do not satisfy this requirement unless discovery and verification can consume them.” The acceptance criteria require that a failed or canceled nightly retain independently published completed hosted cells so retries do not repeat them while evidence still qualifies.

**Defect class:** durable cell artifacts are disconnected from automatic evidence discovery until a later aggregate job runs.

The native and WSL2 producers upload `observation-*` artifacts immediately, but the trusted [publish-observations job](../../.github/workflows/ci.yml:1040) waits for the entire `area-ci` matrix before writing discoverable notes. The next nightly fetches notes and calls [verify_cells](../../scripts/ci/local_evidence.py:377); its artifact probe verifies observations already discovered in notes and does not discover orphaned observation artifacts. If the collector never runs or is interrupted, previously completed cells are invisible to reuse even though their observation, report, and completion artifacts are intact. The workflow itself documents this loss and requires manual recovery.

This is a recovery-path implementation gap, not a request to prove GitHub permissions or to run a hosted experiment. The dependency graph and reader path establish the delay, and the existing fixture demonstrates the missing connection.

| Site swept | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Native hosted producer → nightly verifier | Completed cell uploads observation, report, and completion; aggregate collector has not run | No reuse credit | Completed proof is discoverable before unrelated areas finish |
| WSL2 hosted producer → nightly verifier | Same completed uploads, collector absent | No reuse credit | Same per-cell recovery guarantee |
| Aggregate collector → nightly verifier | Publish the very same retained native and WSL2 artifacts | Both cells receive reuse credit | Same; clean |
| Rig/seed producer | Individual finished leg records and publishes its own observation | Discoverable independently of subsequent legs | Same; clean |

Reproduction used `HostedFixture.finish_cell` for `alpha` on Ubuntu and WSL2, followed by `credited()` before and after `publish_run()`. Before the collector, the result was `[]`; afterward it was `['alpha/ubuntu-latest/L1', 'alpha/wsl2-ubuntu/L1']`. The native Windows producer shares the native workflow's observation steps. Its wiring was checked alongside [the WSL2 steps](../../.github/workflows/_wsl-ci.yml:885); no new credential is needed in PR producers to fix this.

Make completed hosted observations automatically discoverable independently of the end-of-run collector, either through trusted per-cell publication or a bounded recovery discovery path. Keep the existing restriction that PR code receives no new write credential. Add a fixture where artifacts exist but collection never occurred; current recovery fixtures start with notes already written or explicitly call the collector.

### Medium — Seeding cannot execute an unproved tier beyond Level 1

**Authority: spec.** Required seeding includes “declared gates and features” and says “On resume, reverify inputs and run only unproven cells.”

**Defect class:** the seed executor collapses gate-specific pending work into a package/environment run that always executes Level 1.

In the `repo-deps` package, [run_cross_check](../../scripts/ci/seed.py:224), which launches a pending seed leg, passes only package and OS to the shell transport. Both archive consumers in [cross-check.sh](../../scripts/cross-check.sh:699) hardcode the Level-1 listing and test recipe, and publication defaults to Level 1. If Level 1 is already proved and Level 2 becomes required, the seed reruns Level 1, leaves Level 2 unproved, and repeats this behavior on resume.

| Site swept | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Native Windows seed/consumer | Level 1 proved; Level 2 pending | Package leg invoked; only Level-1 transport exists; Level 2 remains unproved | Execute and publish Level 2 only |
| WSL2 seed/consumer | Same pending gate shape | Same result | Same gate-specific execution |
| Both seed environments | Level 1 pending; higher tiers accepted gaps | Level 1 proved; gaps preserved | Same; clean |
| Generic rig recorder | Explicit non-Level-1 `gate` argument | Recorder can address that gate | Clean capability; shell/seed never supply the gate |

Reproduction changed the existing `test_seed.PLAN` fixture to make `alpha/L2` pending, retained `alpha/L1` proof, and ran public `Pass.refresh()` and `seed.run()` with the existing `Rigs` transport fixture. Both environments invoked the package leg and left `alpha/<environment>/L2` unproved. Inspection of both generated consumer scripts confirms that the production transport runs only Level 1. The existing seed suite uses accepted gaps for higher tiers and therefore does not exercise this case.

This is medium because the shipped full-workspace schedule currently has no pending non-Level-1 cells; higher tiers are governed gaps. Before enabling such coverage, carry the pending gate and canonical plan into transport execution. If that extension is deferred, document the seed command's Level-1 limitation and its inability to seed newly enabled higher tiers.

## Verification

The requirements here concern planning, filesystem/Git evidence, and workflow orchestration. Level-1 in-process and isolated subprocess fixtures are appropriate; no terminal rendering or OS keyboard behavior is added, so Level-2/Level-3 terminal evidence is unnecessary.

| Requirement | Verification checked | Assessment |
| --- | --- | --- |
| PR/push cadence, opt-ins, empty proven push, demanded WSL2 producer | Planner event tests and shipped workflow contracts | Level 1 is appropriate |
| Bounded wait for matching unfinished PR validation | Public validation reader with API responses and bounded waiter tests | Level 1 is appropriate |
| Separate nightly anchors, migration, unchanged empty plans, added coverage | Anchor reader and public planner fixtures | Level 1 is appropriate; changed-contract cases are missing |
| Per-cell publication, concurrent notes updates, conflicting outcomes | Real isolated Git repositories/remotes plus public publication/verification | Level 1 is appropriate; delayed publication exposes the first finding |
| Hosted completed-cell recovery | Generated observation/report/completion files and simulated artifact service | Level 1 is appropriate; collector-absent case exposes the third finding |
| Seed pinning, unavailable rigs, interruption, resume | Seed public orchestration fixtures and cross-check transport contracts | Level 1 is appropriate; pending higher-tier case is missing |
| Malformed JSON/configuration | Existing field-matrix tests for collections, hosted envelopes, certifications, anchors, completion inputs, and seed state | Reviewed public-result coverage and strict loading; no separate shape finding |

Focused runs passed: 63 seed/validation tests, 44 anchor/event/collection tests, and 22 observation-collection/rig-publication contract tests. `actionlint` and `git diff --check` passed. The broad combined run did not complete: its initial seed import lacked `PYTHONPATH` (the corrected seed run passed), and the remaining run was interrupted after more than 13 minutes while scanning repository test inputs. Its partial progress is not claimed as a passing suite. No Rust suite or cross-OS run was performed during this review. No hosted workflow was dispatched, no terminal window was opened, and no implementation code was changed.

## Observations

**Existing source-input audit repeatedly scans the checkout.** The unchanged `repo-deps` test [test_every_declared_source_input_is_read_by_its_declarer](../../scripts/ci/test_affected_scope.py:8886) rebuilds a full plan for each declaration and repeatedly scans repository test sources. After the broad review run had been running for more than 13 minutes, it spent over a minute without output in this test; interruption located it in the test-input scanner's token matching. The method exists unchanged on the base branch, so this is an observation, not a readiness finding. Reusing the scan within the audit would be a potential separate test-performance improvement; no repair is requested in this review cycle.
