---
$schema: feature-review.yaml
ready: false
findings:
  - 'R1-F1 (high): Release preparation rejects the renamed CI workflow'
  - 'R1-F2 (high): The naming fallback lacks the required feasibility evidence'
  - 'R1-F3 (high): The provisioning test fails on unrelated diagnostic steps'
human_review: true
human_review_items:
  - 'For later review: exact descriptive titles, job names, and step names were selected by the implementer; retain them unless different wording is preferred.'
  - 'For later review: Phase 1 temporarily accepted old and new names; the finished tests accept only the new names. No action is needed.'
  - 'For later review: workflow structure is compared with a feature-local Python script and saved baseline instead of a permanent snapshot test; retain this review aid.'
  - 'For later review: naming evidence lives in implementation.md and progress in implementation-log.md; retain the split.'
  - 'For later review: two downstream workflow triggers were updated to preserve automation after the title change; retain these necessary compatibility edits and complete the release-reader repair in R1-F1.'
  - 'For later review: renamed compilation and companion-test steps now receive accurate diagnostic stage categories; retain the categories, which do not decide retry eligibility.'
  - 'For later review: accepted-gap explanation text retains the unchanged coverage-audit job identifier; retain it to preserve the other payload fields.'
  - 'For later review: the existing topic page is the canonical naming reference, with links from the README and skill; retain this organization.'
  - 'For later review: historical parser fixtures remain, unknown gap labels are refused before publication, and nearby naming comments were corrected; retain these choices.'
  - 'For later review: the baseline included concurrent package-workflow edits and was checked again after they were committed; retain that comparison boundary.'
  - 'For later review: darkmatter verification covered its two affected documentation tests, and additional OS runs were deferred to CI; no extra local run is requested.'
  - 'For later review: automatic matrix names were chosen and renamed-name corroboration was deferred; R1-F2 requires evidence before that choice can be accepted.'
  - 'For later review: the existing provisioning failure was deferred to another owner; R1-F3 recommends a small test-only repair in this cycle.'
  - 'For later review: replacement of the two job-name readers remains assigned to their Rust port, as the spec directs; no work is added here.'
  - 'Implementer decision (R1-F1): the release selector accepts only the current title `Continuous integration`, not the historical `ci`, because release-plz.yml runs from main and can no longer be triggered by a `ci`-titled run; accepting both or matching `workflow_run.path` were the alternatives.'
  - 'Implementer decision (R1-F2): display check run 42963148323 showed GitHub cuts each job-name segment at 100 characters, so a leading label loses the runner; the fallback was retained. A static descriptive name over unlabeled rows (`Run tests (pkg, L1, env, runner)`) also renders well but would replace the job id runner_loss.py reads; confirm or request that change.'
  - 'Implementer decision (R1-F2): the disposable display workflow ran from a signed orphan commit pushed with `--no-verify` to a scratch ref, deleted afterwards; the run record remains in Actions history.'
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-10T20:44:07-08:00
spec: 2026-11-06-legible-ci-names/spec.md
implemented: true
description: 'A **feature** review of `2026-11-06-legible-ci-names/spec.md`'
feature: 2026-11-06-legible-ci-names/review-1.md
log: /Volumes/coding/rusty-biscuit/features/2026-11-06-legible-ci-names/log.md
implemented_by: claude/opus
next: 2026-11-06-legible-ci-names/review-2.md
---

# Review 1

The feature is not production ready. The title rename prevents release preparation, the fallback's required display evidence is incomplete, and a known provisioning test still fails. None requires a human decision before repair.

## Implementation decisions (informational)

Two independent readers inspected the spec and implementation records in parallel. Their inventories agree. The following choices are reasonable, reversible, or already authorized:

- Exact descriptive titles and step names were selected within the naming convention.
- Temporary old-or-new assertions enabled Phase 1 verification; final assertions now require the new names.
- A feature-local script and saved structure baseline avoid permanently freezing workflow execution details. The baseline included concurrent edits and was rechecked after those edits were committed.
- Naming evidence and progress occupy separate files. The canonical naming reference is a section of the existing CI topic page, with links from other documentation and skills.
- Updating the two downstream title-based triggers preserves existing automation. This is necessary compatibility maintenance, although the release decision reader was missed; see R1-F1.
- Compilation and companion-test steps now receive more accurate diagnostic categories. These categories do not determine retry eligibility.
- Accepted-gap explanation text retains the unchanged job identifier, preserving every payload field except the check name.
- Historical parser fixtures remain; unknown gap gates and environments are refused before posting; adjacent stale naming comments were corrected.
- Darkmatter verification was narrowed to its two affected document-corpus tests. Additional cross-OS execution was left to CI, which does not block this review.
- Replacing the existing job-name readers remains out of scope and belongs to their Rust port, as already decided.

Two deferred items become ordinary findings: the fallback and post-change display corroboration in R1-F2, and the provisioning failure in R1-F3. The implementer's statement that another owner must choose a provisioning repair is unnecessary: the existing named-step helper permits a small test-only repair. No change of ownership or scope is needed.

## Findings

### R1-F1 (high): Release preparation rejects the renamed CI workflow

**Authority: regression and spec acceptance item 1**, “Preserve machine lookup behavior; where a reader uses the old display name, update it with the rename and verify that it still finds the same jobs.” A successful push run on `main` now has title `Continuous integration`, but [the release preparation selector](../../../scripts/ci/release_path.py:574) still requires `workflow_run.name == "ci"`. The updated trigger starts the release workflow, then its selection step returns a successful no-op. No release proposal is prepared.

Defect class: a presentation rename updates an event trigger but leaves a downstream identity comparison and its positive fixture on the old title.

Reproduction: use `workflow_run_event()` from [the existing release fixture](../../../tools/test-toolkit/tests/fixtures/release_path/driver.py:85), keeping every field unchanged except `name`. Call `select_preparation("workflow_run", CANONICAL, event)` in process. The control `ci` selects and returns `validated_sha`; `Continuous integration` returns `selected=False`, reason `workflow 'Continuous integration' is not ci`. The fixture's existing positive case still passes, explaining why the contract suite misses this regression.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F1.1 | `release_path.py::select_preparation` | Existing successful-main-push fixture with renamed title | Rejects | Select |
| R1-F1.2 | Release fixture positive case | Same event, old title | Passes without testing shipped title | Positive case exercises shipped title |
| R1-F1.3 | `release-plz.yml` trigger and guards | Renamed title, successful push on main | Trigger matches; guards do not compare title | Clean; reach selection |
| R1-F1.4 | `ci-infra-retry.yml` trigger and decision path | Renamed title, failed first attempt | Trigger matches; decision uses run ID and attempt, not title | Clean |
| R1-F1.5 | `reuse_validation.py`, `hosted_observations.py`, `distribution.py` workflow lookup | Same workflow file with renamed title | Identity uses file path or workflow reference | Clean; title-independent |

The sweep searched workflow-name comparisons and followed the release selection's `selected` output to the guarded release-tool setup, token creation, and proposal creation steps: all are skipped for the renamed event.

Recommended repair: recognize the current title, update the real-shaped fixture, and connect its positive title to the shipped workflow so another rename cannot silently break it. Keep unrelated workflow rejection and repository, branch, status, and conclusion checks. Historical-title support may remain for already-created runs. Correct the selector's documentation alongside its behavior. This is an existing workflow-title reader, not a new job-display-name reader.

### R1-F2 (high): The naming fallback lacks the required feasibility evidence

**Authority: the bounded feasibility check and the author's fallback ruling.** The spec permits fallback “if the display check shows that the label cannot render legibly, or that adding it loses identity in any rendered name,” and requires complete observed names in the Actions job list and pull-request checks list. [The evidence record](implementation.md) explicitly says the fallback was chosen without a display experiment and all renamed names remain projections. Its sources show historical four-value rows, not the proposed added label.

Defect class: a conditional design fallback is accepted using evidence that does not test the condition.

The evidence that a static name hides a suffix addresses one possible template. It does not establish that a leading display-only label in an automatic suffix fails, and the existing strict row schema is not a reason to reject the preferred design: the spec expressly permits updating a strict reader for that additive field. The record also leaves static-name suffix behavior uncertain. Consequently the projections test internal string composition, not the rendering decision that justified leaving `L1` and runner labels in executing names.

| Instance | Site | Shape checked | Observed evidence | Expected |
|---|---|---|---|---|
| R1-F2.1 | Native package matrix | Leading display label plus stored identity | Only historical unlabeled names and projected renamed names | Evidence settling legibility and retained identity |
| R1-F2.2 | WSL2 nested call | Same label through delegation and guest | Historical unlabeled nesting and projection | Same check through WSL2-shaped nesting |
| R1-F2.3 | Two levels of one package | Level 1 and Level 2 | Historical levels distinguish; renamed strings manufactured in test | Complete naming examples corroborating chosen template |
| R1-F2.4 | Nested area and empty selection | Nested directory; skipped matrix | Historical examples exist; renamed names projected | Chosen names retain area and expose no raw expression |
| R1-F2.5 | Actions list and PR checks projection | Complete names in both surfaces | Pre-rename records only; new checks prefix explicitly uncertain | Settle complete chosen rendering without a full-scope run |
| R1-F2.6 | Accepted-gap name generation | Known gate/environment mappings | Publisher public-result tests pass | Clean; this name is directly controlled by the publisher |

Recommended repair: first seek existing records that actually settle the added-label template. If they do not, use the spec's one small disposable display workflow, including representative native, nested, and skipped shapes; record complete names and the resulting decision. Retain the fallback if that check establishes its condition. Otherwise implement the authorized additive label and its reader/schema tests. Do not restructure production workflows or trigger full repository CI to answer this question. No terminal keyboard or real-terminal test is relevant; the missing boundary is GitHub's rendering service.

### R1-F3 (high): The provisioning test fails on unrelated diagnostic steps

**Authority: defect**, reproduced with the checked-in workflow, not synthetic configuration. [The build provisioning test](../../../scripts/ci/test_ci_local.py:2923) calls `job_run_steps` for the entire build job just to select `Install native prerequisites`. [The workflow reader](../../../scripts/ci/workflow_reading.py:252) rejects `run: >-`, so the test raises before executing its intended prerequisite assertions. The implementation records this existing failure and leaves it red.

Defect class: a focused test parses unrelated workflow steps using a deliberately restricted reader, so supported production workflow syntax prevents the intended behavior from being tested.

The whole build-job read raises on `Sample storage before the cache restore`. To sweep siblings, each of the six unchanged diagnostic step blocks was copied from `ci.yml` into an otherwise minimal build job and passed through public `job_run_steps`; each raises the same error. The focused `step_script` helper successfully reads `Install native prerequisites` from the unmodified workflow.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F3.1 | Before cache restore sample | Checked-in folded run block | Whole-job reader raises | Must not prevent prerequisite test |
| R1-F3.2 | After cache restore sample | Checked-in folded run block | Same | Same |
| R1-F3.3 | Before schematic guard sample | Checked-in folded run block | Same | Same |
| R1-F3.4 | After schematic guard sample | Checked-in folded run block | Same | Same |
| R1-F3.5 | Before publication sample | Checked-in folded run block | Same | Same |
| R1-F3.6 | After publication sample | Checked-in folded run block | Same | Same |
| R1-F3.7 | Other live reader callers: scope, preflight, ci-gate, package check/test/lint, WSL guest, area audit | Unmodified workflow jobs | All public whole-job reads succeed | Clean |
| R1-F3.8 | Named prerequisite reader | Unmodified build job | `step_script` returns intended script | Clean; usable repair |

Recommended smallest repair: use the existing named-step reader in `provision_owner` and keep both prerequisite-list assertions. This avoids changing production commands, expanding the deliberately narrow YAML reader, or introducing a new protection layer. Verify the previously failing test and its neighboring provisioning cases. The feature's protected-command comparison can remain unchanged.

## Verification and limits

- `cargo nextest run -p test-toolkit --test ci_workflow_contracts`: 227 passed, none skipped. These tests are compiled by the package's normal integration-test discovery and selected as Level 1; the new naming tests have no higher-tier marker.
- `python3 scripts/ci/test_runner_loss.py`: 72 passed.
- `python3 scripts/ci/test_reuse_validation.py`: 52 passed.
- `python3 scripts/ci/test_publish_gaps.py`: 29 passed.
- Existing release-fixture positive case passed; changing only its title reproduced R1-F1 through the public selector.
- `python3 scripts/ci/test_ci_local.py`: 104 tests, 103 passed and one failed in 245.218 seconds. The sole failure is the provisioning-reader failure reported in R1-F3. Live workflow-reader calls and the six copied diagnostic blocks independently reproduced it.
- Reviewed the workflow diff: job/step identifiers, commands, guards, dependencies, runners, permissions, and artifact fields remain unchanged; the two additional workflow edits only change their title triggers. The optional baseline script could not run with this session's `python3` because PyYAML is absent; direct diff inspection supplied the alternative check.

No configuration parser or field validator was added or changed by this implementation, so the input robustness matrix does not apply. The newly edited readers consume job-display text; the gap publisher maps already-planned gate/environment values. R1-F3 concerns an existing intentionally restricted workflow reader; the recommended repair changes its test caller, not its accepted configuration language.

Level 1 is appropriate for literal workflow contracts, job-name parsing, audit discovery, and publisher payloads. GitHub-rendered-name feasibility needs the bounded service evidence described in R1-F2. Cross-OS evidence is left to CI and is not a finding. This is the first implementation review; there are no previous finding outcomes.
