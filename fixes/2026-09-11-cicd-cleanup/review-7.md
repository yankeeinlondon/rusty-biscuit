---
$schema: feature-review.yaml
ready: false
findings:
  - title: Dependent compilation omits the consumers' native prerequisites
    priority: high
  - title: An advanced PR target still disables authoritative local planning and reuse
    priority: high
  - title: Nested workflow behavior remains unverified at the required boundary
    priority: high
  - title: Deferred audit coverage and active documentation remain incomplete
    priority: medium
human_review: true
human_review_items:
  - |-
    After the implementation and its hosted demonstration pass, approve replacing the repository's required merge check, `ci-verdict`, with `ci-gate`. This setting controls whether pull requests can merge. Review the demonstration showing that an unaccepted failure blocks merging, an explicitly accepted failure follows the recorded exception, and an unselected area does not leave a pending check; then authorize the coordinated branch-protection change required by the specification.
reviewed_by: codex/gpt-6-astra
created: "2026-09-12T11:20:55-07:00"
spec: 2026-09-11-cicd-cleanup/spec.md
implemented: false
description: "A **fix** review of `2026-09-11-cicd-cleanup/spec.md`"
fix: 2026-09-11-cicd-cleanup/review-7.md
previous: 2026-09-11-cicd-cleanup/review-6.md
---

# Review 7

**Not production-ready.** Strict-failure publication, evidence-error fallback, receipt bootstrap, and ancestor-based local reuse now have meaningful behavioral regressions. The dependent compile step exists, but omits native prerequisites for the additional packages it builds. Local reuse still falls back to the old path when the PR target advances. Hosted verification and part of the absorbed audit remain explicitly deferred.

Reviewed the current working-tree implementation against the September 11 specification, its September 12 rulings, the absorbed September 10 specification, and the absorption audit. The implementation log reports six findings fixed and two deferred; it does not claim all eight were closed. This review changes only the review chain and specification metadata. Missing cross-OS execution results and the separately required human approval do not determine `ready`.

## Findings

### High — Dependent compilation omits the consumers' native prerequisites

`scripts/ci/affected_scope.py:1918` populates `native` using only `native_closure(package_id, ...)` for the changed package. Its new `dependent_seam` is attached separately at line 1924. `_package-ci.yml:193` installs that original native list, then line 225 runs `cargo check` for the unchanged consumers. A reverse dependent and its other dependencies are outside the changed package's build closure.

Consequently, a valid API-compatible consumer requiring an additional system development package can fail in the changed area's check job because CI never provisioned its prerequisites. The failure is then described as a consumer/public-API compile failure, obscuring the setup defect. This affects OQ1 and required behavior 1.2/1.3, not merely cross-OS proof.

**Reproduction (L1, executed):** instantiate `DependentSeamTests`, set the fixture policy for `gamma-lib` to `native = {"ubuntu-latest": ["libfixture-dev"]}`, and plan `alpha/lib/src/lib.rs`. The resulting seam contains `beta-app` and `gamma-lib`, with command `-p beta-app -p gamma-lib --lib --bins --tests`, while the changed package's provisioned `native` remains `{}`. The fixture already includes a consumer's independent dependency, so it can also prove transitive prerequisite collection. A read-only scan of current Cargo metadata found 49 changed-package/consumer edges with additional declared Ubuntu prerequisites; examples include `biscuit-hash` → `playa` and `biscuit-terminal` → `messenger-cli`. That count describes declared policy omissions, not 49 observed hosted failures; optional features and runner preinstalls affect which fail in practice.

Carry the union of native requirements for the actual dependent build closures into the owning Ubuntu check's provisioning. Keep that additional setup specific to the check that compiles them; unrelated test and platform jobs need not install it. Add L1 planner/projection and workflow-step assertions for a consumer with its own native requirement and one inherited through another dependency. Retain the existing real Cargo broken-consumer fixture, which correctly proves API failure attribution but uses only Rust/std and cannot expose this setup omission.

### High — An advanced PR target still disables authoritative local planning and reuse

`.githooks/pre-push:423` returns before saving `HEAD_PLAN_FILE`, `HEAD_REVIEW_FILE`, or `HEAD_PLAN_BASE` when the freshly fetched PR target is not an ancestor of HEAD. That is an ordinary branch shape after another change lands on the target. The trigger review has already calculated and checked the exact two-dot comparison, but the gate path at lines 620–633 then falls back to working-tree planning. `just/ci-local.just:178` computes its own merge base; its evidence overlay is limited to `--plan`, so the ordinary fallback run reruns cells with qualifying prior passing evidence. The publication guard also withholds the new validation receipt because `HEAD_PLAN_BASE` is empty.

The ancestor restriction remains in `record_scope` (`scripts/ci/local_evidence.py:473`) and `record_cells`. Thus the W1/W14/D2 correction is conditional on ancestry, even for a clean checkout and an exact reviewed comparison. W2 was deferred, but coupling that deferral to plan handoff leaves the claimed local-reuse fix incomplete too. This causes repeated local and hosted work, rather than silently accepting an invalid result.

**Evidence:** the passing L1 test `test_an_advanced_target_branch_is_read_from_the_remote_and_records_no_receipt` (`.githooks/tests/test-pre-push.sh:1680`) manufactures this branch shape and explicitly requires withholding the receipt. It runs in `scope-only`, so it does not verify ordinary gate execution. New reuse fixtures exercise documentation-only follow-ups, changed inputs, prior failure, and a stacked ancestor target; they do not close this case.

Support newly calculated exact `{base, head, tree}` receipts for an advanced target, while continuing to reject a receipt recorded against a different base. Feed the reviewed plan to clean local validation independently of receipt-publication success. Extend the real hook/recipe fixture with an advanced non-main target, an older passing cell whose inputs remain identical, and counters proving no second selection or rerun of that cell. Assert the newly recorded comparison and remotely available outcomes. This needs L1 Git/subprocess evidence, not a hosted or cross-OS experiment.

### High — Nested workflow behavior remains unverified at the required boundary

The producer normalization changes the relevant commands to step-level `continue-on-error`, folds their outcomes into status artifacts, and leaves area evaluation responsible for accepted test failures. The new L1 workflow-step and Rust tests exercise those folds and baseline decisions. This addresses the source-level defect from review 6, but does not establish the outcome GitHub gives the complete nested workflow.

`fixtures/throwaway-branch-plan-2026-09-12.md` is explicitly a plan with `status: awaiting-push-authorization`. `rollout-2026-09-11.md` remains `not-triggered`. The scratch record covers simplified gate/neutral-check semantics, not the shipped nesting and mixed-cell behavior. The implementation log explicitly defers this hosted half of review-6 finding 1 along with finding 7.

AC2/4/8/9/10/11/12 still need the prescribed hosted integration assertions: nested-area labels; visible reused results with no corresponding producer; gap/no-gap publisher routing and artifact access; mixed local/executing/failed cells; and the four normalization cases—accepted producer failure, unaccepted failure, missing report/setup failure, and equivalent reused failure. L1 source contracts and manufactured step outcomes are below the required boundary for these GitHub-observable behaviors. They are not production-readiness evidence for that integration.

Complete and record the controlled hosted demonstration using the candidate implementation, with check names, conclusions, result slices, and gate outcomes. Reconcile its execution plan with every standing restriction first. In particular, the new plan admits it schedules two WSL cells, while this specification expressly authorizes no WSL rerun. Do not treat the fixture's general authorization or an empty constraint directory as overriding that restriction: obtain qualifying evidence or design a smaller fixture that preserves the workflow boundary without prohibited execution. No hosted work was triggered by this review.

### Medium — Deferred audit coverage and active documentation remain incomplete

Review-6 finding 8 is honestly deferred in `log.md:724`, but several requirements remain unfinished. The deferral records work ownership; it does not supply the missing behavior or tests.

- **Incomplete-run/publication coverage (W8, T3–T5):** the hook has EXIT cleanup but no explicit interruption policy or behavioral fixture proving an interrupted hook cannot publish complete evidence. The override publication test at `.githooks/tests/test-pre-push.sh:746` still checks source strings. The workflow suite has a dispatch case without validation evidence and invalid-scope unit coverage, but still lacks the audit's dispatched-run-with-validation-note and malformed-scope-through-the-real-step cases. Add the smallest L1 hook/step fixtures at these boundaries; neither terminal IPC nor keyboard injection is needed. T1/T2 are now closed by real disposable-remote failure publication tests.
- **Contract disposition (W6/W7/W9/W12):** scope receipts still store both `plan` and its legacy `scope` projection; lint cells still inherit `reusable: true` although the recorder refuses lint; browser receipt reuse still needs an explicit disposition against the absorbed exclusion; `verified_environment` remains alongside per-cell verification. Close or justify these individually under B0. These are not all independently demonstrated runtime defects.
- **Documentation (R10, AC14, W11):** `README.md:117` still says local validation compile-checks reverse dependencies and publishes successful whole-environment evidence, and the table at line 129 omits `scope-only`. `docs/testing-strategy.md:601` and its later reuse/in-flight sections retain the prior model. The unknown-mode diagnostic still lists only `off, warn, or strict`. Treat current code as the authority and update these explanations; do not regress code to match stale prose.
- **Workflow validation (W13):** actionlint still returns 1 with SC2086 diagnostics at `_package-ci.yml:193,326,543,716,892` and `_wsl-ci.yml:179`. These predate this cycle, but cleaning them is explicitly absorbed scope. Use a correctly constructed argument array, or a narrowly justified suppression where splitting is intentional, and make the required check pass.

## Previous-review disposition

| Review-6 finding | Review-7 assessment |
|---|---|
| Raw producer failures bypass area baselines | Source-level correction and L1 regressions present; hosted behavior remains unverified. |
| Complete strict failures withhold evidence | Fixed: strict blocks after note publication; a fresh clone verifies the failed outcome. Warn publication is also covered. |
| Local validation replans and reruns proven cells | Fixed for clean ancestor-based comparisons, including stacked targets; advanced-target path remains open above. |
| Dependent compile step absent | Implemented with real Cargo failure attribution; native provisioning is incomplete. |
| Verifier crash aborts scope | Fixed: verifier/overlay failures retain the original plan and emit diagnostics; focused step regressions pass. |
| Receipt fast path initializes Rust and omits diagnostics | Fixed: the hit path avoids `rustup`; pass/fail/rejection summary assertions pass. |
| Real workflow presentation unverified | Deferred; a detailed plan is present, no execution record. |
| Absorbed audit and documentation | Deferred; some publication items were closed by other findings, remaining items listed above. |

## Requirement-to-verification map

L1 below includes in-process tests and hermetic subprocess/Git/Cargo fixtures. This specification introduces no keyboard, mouse, or terminal-emulator rendering contract. Terminal-IPC L2 and OS-keyboard L3 are therefore inapplicable. Hosted Actions integration is the appropriate additional boundary for GitHub labels, nested outcomes, and check presentation; it is not interchangeable with terminal L2/L3.

| Requirement | Strongest evidence inspected | Assessment |
|---|---|---|
| AC1: source-owned areas, no consumer areas | L1 planner and static Cargo fixture | Appropriate selection coverage. |
| AC2: one identity per area, nested areas distinct | L1 mapping/workflow contracts | Hosted display proof missing; high finding. |
| AC3/OQ1: explicit compile targets and required environments | L1 planner, command, real Cargo fixture | Native setup defect above; no cross-OS-results finding. |
| AC4: visible reused results without rerun | L1 evidence/rollup and hook/recipe fixtures | Hosted visibility missing; advanced-target local fallback remains. |
| AC5: combined environment and cross-check receipts | L1 Git/evidence fixtures | Appropriate logic boundary; no WSL execution requested. |
| AC6: seven macOS results never become MISSING | L1 Rust regression | Appropriate; rollup suite passed. |
| AC7: rejected evidence, equivalence, constrained triggers | L1 evidence, constraint, hook, and workflow-step fixtures | Rejection/fallback logic covered; advanced-target production incomplete. |
| AC8: local/CI failures owned by their area; independent work | L1 Rust and normalized status scripts | Nested workflow propagation needs hosted proof. |
| AC9: explained gaps, expiry/revocation/cancellation | L1 publisher/planner/rollup; recorded scratch experiment | Basic neutral semantics recorded; shipped publisher integration pending. |
| AC10: resolved labels and environment-qualified lint | L1 workflow contracts | Hosted label proof missing. |
| AC11: area-owned decision and policy-free merge gate | L1 evaluator/fold; simplified scratch fixture | Normalized producer-to-gate hosted cases pending. Human ruleset approval separate. |
| AC12: successful prior validation reuse, invalid-result refusal | L1 reuse suite and recorded scratch outcomes | Logic covered; final workflow integration pending. |
| AC13: worker budgets, L2 forwarding, explicit overrides | L1 recipe and argument fixtures | Appropriate for scheduling arguments; passed. |
| AC14: current documentation and skills | Direct source review | Active authorities still stale. |
| AC15: area mapping matches sniff | Conditional L1 subprocess drift contract | Appropriate where sniff is installed; no substitute aggregate claim. |
| AC16: v1 exact identity and unrecorded measurements | L1 Git/schema/Rust fixtures | Appropriate logic coverage. |
| AC17: visible plans and persistent restrictions | L1 store, renderer, recipe, and hook fixtures | Appropriate; trigger tests passed. |
| Absorbed R1–R3 / AC1–4: authoritative committed scope | L1 scope receipts and extracted real step | Core hit/miss proven; advanced-base recording/handoff open. |
| Absorbed R4–R7 / AC5–8,12–13: complete outcomes | L1 real remote notes, verifier, recipe, rollup | Strict/warn fixed; interruption/override boundary coverage unfinished. D2 governs recency. |
| Absorbed R5 / AC9–11: scope-only/off/no-verify | L1 hook and fresh-clone reuse fixture | Core paths covered; mode documentation incomplete. |
| Absorbed R8–R9 / AC3,14–15: fallback/bootstrap/dispatch | L1 real workflow-step suite | Crash fallback and cheap hit proven; audit T4/T5 remain. |
| Absorbed R10 / AC16: docs and validation | Direct review, focused suites, actionlint | Documentation and six lint diagnostics remain. |

## Verification performed

- `python3 -m unittest discover -s scripts/ci -p 'test_*.py'`: **455 passed**, 97.540 seconds.
- `env -u CDPATH GIT_TERMINAL_PROMPT=0 /bin/bash ./.githooks/tests/test-pre-push.sh </dev/null`: **62 passed**, 0 failed.
- `cargo nextest run -p test-toolkit --test ci_workflow_contracts`: **90 passed**, 0 skipped.
- `cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features --bin ci-rollup`: **192 passed**, 0 skipped.
- `shellcheck .githooks/pre-push`: passed.
- Actionlint on `ci.yml`, `_area-ci.yml`, `_package-ci.yml`, `_wsl-ci.yml`, and `ci-infra-retry.yml`: **failed**, six SC2086 diagnostics above.
- Temporary static-policy probe: confirmed the dependent command includes a consumer whose declared native prerequisite is absent from provisioning. Read-only Cargo metadata scan corroborated the same policy shape in the actual workspace. Neither probe installed packages or ran consumer builds.

The test totals identify the suites executed, not requirement completeness. No full workspace suite, WSL rerun, terminal window, hosted workflow trigger, repository commit, push, or ruleset change was performed. Hook fixtures create commits and publish notes only in disposable local repositories.

GitNexus query/context was consulted for the CI flow and `native_closure`; the index is 94 commits behind HEAD. Document impact reported UNKNOWN (no resolved callers/processes for the spec; review 6 absent from the index), so it was not treated as a safety clearance. Current source and review-chain references were checked directly. No implementation symbol was edited.

Biscuit-file's `bf reference` found no file for `@prompts/_reviews/fixes/2026-09-11-cicd-cleanup/review-6.md` and resolved the existing review under `fixes/2026-09-11-cicd-cleanup/review-6.md`. That file retains `implemented: true` and now points `next` here. The specification records `review_iterations: 7`.
