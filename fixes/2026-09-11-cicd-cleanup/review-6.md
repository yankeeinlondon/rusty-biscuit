---
$schema: feature-review.yaml
ready: false
findings:
  - title: Raw producer failures bypass the area's baseline decision at the merge gate
    priority: high
  - title: Complete strict failures still withhold remote validation evidence
    priority: high
  - title: Local validation replans and reruns cells already covered by accepted evidence
    priority: high
  - title: The approved dependent compile step is absent
    priority: high
  - title: Validation-verifier failure aborts scope instead of retaining normal execution
    priority: high
  - title: The receipt fast path still initializes Rust and omits required diagnostics
    priority: high
  - title: Real workflow presentation remains unverified at the required boundary
    priority: high
  - title: Absorbed audit work and active documentation remain unresolved
    priority: medium
human_review: true
human_review_items:
  - |-
    After the implementation and its demonstration pass, approve changing this repository's required merge check from `ci-verdict` to `ci-gate`. This controls whether pull requests may merge. Confirm that a failed selected area blocks merging and an unselected area does not leave a pending requirement; then authorize the coordinated branch-protection change. The implementation must first resolve the baseline-handling finding in this review.
reviewed_by: codex/gpt-6-astra
created: "2026-09-12T02:39:40-07:00"
spec: 2026-09-11-cicd-cleanup/spec.md
implemented: true
implemented_by: claude/fable
log: fixes/2026-09-11-cicd-cleanup/log.md
description: "A **fix** review of `2026-09-11-cicd-cleanup/spec.md`"
fix: 2026-09-11-cicd-cleanup/review-6.md
previous: 2026-09-11-cicd-cleanup/review-5.md
next: 2026-09-11-cicd-cleanup/review-7.md
---

# Review 6

**Not production-ready.** The simultaneous-update constraint defect and persistent-store discovery are fixed. The policy-free gate and neutral-gap publisher now exist. However, the implementation does not satisfy several explicit September 12 rulings, and the required hosted presentation verification remains incomplete. Passing tests include assertions that enforce behavior the rulings explicitly rejected.

Reviewed HEAD `a8dbd1a61b16d7fb353ef508f4080e7849d06a66` plus the existing working-tree implementation. The scope includes the September 10 specification absorbed by ruling B0, its audit, and rulings OQ1–OQ4/D1/D2. This review changes only three review/specification documents. Missing cross-OS results and the eventual human approval are not readiness findings.

## Findings

### High — Raw producer failures bypass the area's baseline decision at the merge gate

`.github/workflows/ci.yml:518` makes `ci-gate` depend on the reusable `area-ci` job, not solely on its policy-evaluated rollup. Inside that reusable workflow, `_area-ci.yml:57` calls the package producers and `:166` runs the rollup after them. A failed L1 command at `_package-ci.yml:362` remains a failed job: it has no error normalization after recording its result. A successful later rollup does not erase that producer failure from the reusable workflow's outcome.

Consequently, a complete hosted test failure that the area's exact, valid baseline accepts still reaches the outer fold as a failed `area-ci`. The same failure supplied only by local evidence can be accepted because it has no failed hosted producer. This contradicts area-owned policy and equivalent treatment of local/CI outcomes. GitHub distinguishes continuing dependent jobs from suppressing a failed job's effect on workflow success; this conclusion follows from the workflow wiring and the documented [job failure semantics](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#jobsjob_idcontinue-on-error). It was not observed in a new hosted run during this review.

The L1 Rust test `messenger_l1_failure_requires_an_exact_package_keyed_baseline` (`scripts/ci-rollup-tests.rs:1478`) proves baseline acceptance inside the evaluator. `WorkflowGateStepTests` supplies manufactured job-result strings, so it cannot establish what the nested producer/rollup workflow returns. The scratch fixture uses simplified area jobs and does not cover a red producer followed by an accepting rollup.

Make the area's policy decision authoritative while preserving genuine setup, upload, cancellation, missing-result, and unaccepted-test failures. Do not simply ignore the entire area workflow. Add a reduced hosted regression with a baselined producer failure, the same failure without acceptance, a missing report, and an equivalent reused failure. The baseline case needs the hosted workflow boundary; L1 evaluator/fold tests alone are insufficient.

### High — Complete strict failures still withhold remote validation evidence

`.githooks/pre-push:650` records cells, but `:657` explicitly avoids `push_note` when strict validation fails. A later `--no-verify` push therefore does not make that locally retained note available to CI. CI repeats the known failure unless the note reached the remote through some separate action.

Ruling D1 explicitly requires remote publication before blocking and removal of this branch. The test `test_a_blocked_strict_push_records_locally_but_publishes_nothing` at `.githooks/tests/test-pre-push.sh:741` still greps for the opposite behavior and passes. This is an implementation defect and an invalid regression contract, not an unresolved design choice.

Publish complete outcomes before returning the strict failure. Replace the grep with an L1 hook fixture that stages a real failing report, checks the nonzero hook result, reads the validation note from the disposable remote, and proves subsequent evidence verification retains the failed outcome without scheduling its execution. Add the complete warn-failure publication case too (audit T1/T2).

### High — Local validation replans and reruns cells already covered by accepted evidence

The hook reviews an evidence-overlaid committed plan at `.githooks/pre-push:526`, but then calls plain `just pre-push` at `:588`. `just/ci-local.just:149` computes scope again from the working tree against its own merge base. Its evidence overlay at `:178` runs only for `show_plan == 1`; the adjacent comment explicitly says ordinary runs have no reason to consume evidence. Validation recording also still resolves `origin/main` at `.githooks/pre-push:602` instead of using the reviewed trigger context.

A follow-up commit that changes only documentation can therefore rerun the source packages changed earlier in the PR even when their prior passing cells qualify under input equivalence. Stacked PRs also retain separate trigger-review and local-validation scope/base paths. D2/W14 explicitly require local reuse, and audit W1 requires binding the validation plan to the reviewed scope. The current behavior defeats the governing requirement to test only changed execution inputs.

Feed the reviewed scope into clean local validation, overlay qualifying prior passing cells for actual execution, and record only cells actually run. Preserve intentional wider dirty-tree feedback without publishing it as exact-tree evidence. Add L1 real-recipe/hook fixtures that count planner and gate invocations for equivalent prior evidence, changed inputs, prior failure, and a non-main PR target; `--plan` rendering coverage does not prove ordinary execution.

### High — The approved dependent compile step is absent

OQ1 selects an Ubuntu compile step for unchanged direct consumers inside the changed package's owned check work, with dependent details and no new consumer area/cell. `scripts/ci/affected_scope.py:1751` computes direct dependents but only reports their names; its comment still treats the decision as open. `check_arguments` at `:740` builds arguments only for the changed package's uncovered examples/benches. Neither the package projection nor `_package-ci.yml` carries or executes a dependent compile list.

A public API change can pass all selected-package tests while breaking an unchanged consumer. Packages without examples/benches have no check cell at all, so adding a step only to currently scheduled check jobs would still miss those packages.

Implement the ruled owner-attributed Linux compile work, including the no-example/no-bench case, exclude already-selected consumers, and report the dependent count and identities. Use L1 static Cargo fixtures with an intentionally broken consumer to prove failure belongs to the changed area and no consumer area is scheduled. Existing planner tests prove exclusion of consumer jobs, not preservation of the ruled compile coverage.

### High — Validation-verifier failure aborts scope instead of retaining normal execution

`.github/workflows/ci.yml:165` invokes `verify --cells` unguarded under `set -euo pipefail`; its `--apply-to` invocation is likewise unguarded. A process/import failure exits the scope step before writing matrix outputs, preventing the remaining package work. Absorbed R8 and audit W4 require failed evidence processing to retain ordinary execution with an explanation, rather than aborting the evidence-independent plan.

Confirmed with the existing `WorkflowScopeStepTests` temporary-repository harness: replace only the validation-verifier invocation in the extracted shell with a process exiting 42. The step exits **42**, writes **no matrix outputs**, and emits no fallback summary. The planner and other evidence paths remain real. This is an L1 subprocess probe, not a hosted experiment.

Keep a valid unmodified plan until the overlay succeeds. On verifier/overlay failure, use that plan with no accepted cells and a specific diagnostic. Add permanent L1 workflow-step regressions for both failures. Do not relax trigger-side constraint enforcement or accept unverifiable evidence.

### High — The receipt fast path still initializes Rust and omits required diagnostics

`.github/workflows/ci.yml:87` executes `rustup show` before checking the scope receipt. On a runner without the pinned toolchain this can initialize/install it even when the receipt is valid. Absorbed R9 explicitly requires the valid-receipt path to avoid toolchain setup unless needed to verify the receipt; the Python receipt path does not establish that need.

The scope summary at `ci.yml:233` reports scope source, selection, and job estimate, but omits matching validation environments, reused passing/failing cells, and cells retained because evidence is incomplete. Rejections are only passed into the overlay when the accepted list is nonempty (`:171`), so the all-rejected case also bypasses that diagnostic attachment path.

Defer toolchain setup to work that needs it, preserve rejection information even when no cell qualifies, and render the required summary fields. Extend the L1 workflow harness across the setup/receipt boundary with a toolchain invocation counter and summary assertions for pass, fail, and all-rejected evidence. Current tests extract only the calculation step and cannot prove the preceding setup was avoided. These are W3/W5 and absorbed AC15 gaps, not missing cross-OS evidence.

### High — Real workflow presentation remains unverified at the required boundary

The neutral-gap publisher is implemented and has L1 payload/subprocess tests. The recorded scratch experiment establishes the chosen fixed gate and neutral-check semantics. It explicitly leaves nested reusable-workflow display and mixed-cell labels to a later fixture (`fixtures/scratch-2026-09-12.md`, final section). The only fixture record present is that scratch record; `rollout-2026-09-11.md` still says `status: not-triggered`, and the latest log at `:526` lists the owed observations.

No inspected evidence proves the actual nested area's labels, a reused cell with no producer but a visible result, gap/no-gap publisher routing and artifact download, or mixed reused/executing/failed/unselected presentation through the shipped workflow shape. Source-string tests and synthetic API payload tests cannot prove these GitHub behaviors. AC2/4/8/9/10 and the final integration chain in AC12 remain below the required verification boundary.

Complete the reduced hosted fixture prescribed by B5, record actual checks, labels, result slices, and conclusions, and include the baseline-propagation case above. No full workspace matrix or WSL execution is needed for this proof. This is an integration-verification gap; it is neither a demand for human testing nor a missing cross-OS result. The log's “4 fixed, 0 deferred” classification does not close this part of the previous finding.

### Medium — Absorbed audit work and active documentation remain unresolved

B0 makes the audit's W1–W14/T1–T5 explicit scope, to be closed or explicitly deferred. Several remaining examples are still present without a completed disposition:

- **W2:** `record_scope` (`scripts/ci/local_evidence.py:464`) rejects non-ancestor bases, and the advanced-target hook test (`:1531`) still pins withholding a newly calculated exact-pair receipt.
- **W6/W7:** scope receipts still carry both `plan` and legacy `scope`; lint cells at `affected_scope.py:1470` do not explicitly declare themselves non-reusable, unlike check cells. The audit identified these as duplicate-state and policy-contract cleanup.
- **W8/W10:** the hook has only EXIT cleanup, no explicit interruption policy, and the combined publication guard silently withholds evidence for several failures. Add behavioral coverage that an interrupted/partial run cannot become complete reusable evidence; mere presence of guard text is insufficient.
- **W9/W11/W12:** browser-reuse scope remains undocumented against the absorbed exclusion, the unknown-mode diagnostic still lists `off, warn, or strict`, and the legacy single-environment helper remains. These need the audit's requested disposition.
- **W13:** actionlint still exits 1 with five SC2086 findings at `_package-ci.yml:168,259,466,626,792`.
- **R10/AC14:** `README.md:117` still describes pass-only whole-environment reuse and consumer checks; `:129` says `off` skips validation entirely. `docs/testing-strategy.md:601,639,672` retains the old model and calls the already-absorbed work “in flight.” Current skills claim strict and warn both publish, contrary to the branch identified above.

Close these items with the smallest relevant behavioral checks or explicitly record justified deferrals as B0 allows. Update active documentation to the final implementation. W1/W3/W4/W5/W14 and T1/T2 are addressed by the higher-severity findings above; T3–T5 still need the audit's behavioral override, dispatch-with-validation, and malformed-scope cases. Do not treat a source grep or a test that enforces a superseded rule as verification of the new requirement.

## Previous-review disposition

| Review-5 finding | Review-6 assessment |
|---|---|
| Simultaneous target/head updates bypass constraints | Fixed: current and incoming target states are checked; both update orders, passing control, and target deletion have hook coverage. |
| Standalone global policy verdict | Removed; the replacement fold is policy-free. Producer-to-area propagation still bypasses baseline acceptance. Live ruleset migration remains a separate approved-release step. |
| Gap publication and hosted verification | Publisher implemented; hosted mixed/nested presentation remains open. |
| Constraints disappear in a fresh session | Fixed: persistent default, environment override, fresh-session checker/preview/hook coverage. |

## Requirement-to-verification map

Here **L1** means in-process or hermetic subprocess/Git/filesystem execution. The appropriate higher boundary for Actions labels, nested workflow conclusions, and merge behavior is **hosted integration**. Terminal-IPC L2 and OS-keyboard L3 do not prove these requirements and are not required: this specification adds no terminal rendering or keyboard interaction contract.

| Requirement | Strongest evidence inspected | Assessment |
|---|---|---|
| AC1: impacted areas; no consumer area jobs | L1 planner fixtures | Selection covered; ruled consumer compile work absent. |
| AC2: area and nested-area identities | L1 workflow/mapping contracts | Hosted verification missing. |
| AC3: relevant compile targets and environments | L1 planner/command fixtures | Core target selection covered; OQ1 work missing. |
| AC4: visible completed local results without rerun | L1 evidence/rollup fixtures | Hosted visibility missing; ordinary local reuse absent. |
| AC5: combined macOS/WSL and cross-check receipts | L1 real-Git/evidence fixtures | Appropriate logic boundary. |
| AC6: seven reused macOS cells never become MISSING | L1 Rust regression | Appropriate; passed. |
| AC7: rejected evidence, input equivalence, constrained triggers | L1 evidence/constraint/hook fixtures | Trigger cases covered; CI verifier crash fallback defective. |
| AC8: failures attributed to their area; independent work completes | L1 Rust and workflow contracts | Baseline propagation defect; hosted mixed-case proof missing. |
| AC9: immediate explained gaps; expiry/revocation/cancellation | L1 publisher/planner/rollup plus scratch API experiment | API semantics proven by record; shipped integration still missing. |
| AC10: resolved labels and environment-qualified lint | L1 workflow-source contracts | Actual nested labels unverified. |
| AC11: area-owned merge authority | L1 fold/evaluator; simplified hosted gate fixture | Baseline-accepted producer case missing and wiring defective. |
| AC12: successful validation reuse; invalid-result refusal | L1 reuse fixtures and recorded scratch conclusions | Final workflow integration remains incomplete. |
| AC13: local/CI workers, L2 forwarding, overrides | L1 recipe/argument fixtures | Appropriate; passed. |
| AC14: accurate active documentation | Source review | Stale authorities remain. |
| AC15: area mapping agrees with sniff | Conditional L1 subprocess drift test | Appropriate where sniff is present. |
| AC16: v1 exact identity and unrecorded measurements | L1 real-Git/Rust fixtures | Appropriate; passed. |
| AC17: persisted restrictions and reviewable plans | L1 store/recipe/hook/renderer | Fresh-session and simultaneous-update cases covered. |
| Absorbed R1–R3/AC1–4: committed authoritative scope | L1 receipt and real workflow-step fixtures | Core scope reuse covered; duplicate local planning and advanced-target receipt work remain. |
| Absorbed R4–R7/AC5–8,12–13: complete outcomes and independent work | L1 evidence/rollup; hook coverage incomplete | Strict publication wrong; failure/interrupt boundary tests incomplete. D2 supersedes cross-note conflict rejection. |
| Absorbed R5/AC9–11: scope-only, off, no-verify | L1 hook/evidence fixtures | Mode execution covered; diagnostic/documentation drift. |
| Absorbed R8–R9/AC3,14–15: fallback, dispatch, cheap bootstrap, diagnostics | L1 extracted workflow step | Crash fallback, setup boundary, summary, and dispatch-validation case incomplete. |
| Absorbed R10/AC16: docs and verification | Source review and focused suites | Audit dispositions, missing behavioral cases, and actionlint remain open. |

## Verification performed

- `python3 -m unittest discover -s scripts/ci -p 'test_*.py'`: **432 passed**, 78.521 seconds.
- `env -u CDPATH GIT_TERMINAL_PROMPT=0 /bin/bash ./.githooks/tests/test-pre-push.sh </dev/null`: **57 passed**, 0 failed.
- `just test test-toolkit`: **156 passed**, 2 skipped.
- `cargo nextest run --manifest-path scripts/Cargo.toml --no-default-features --bin ci-rollup`: **186 passed**.
- `cargo nextest run --manifest-path scripts/Cargo.toml --bin ci-plan`: **9 passed**.
- `shellcheck .githooks/pre-push`: passed.
- `actionlint` on `ci.yml`, `_area-ci.yml`, `_package-ci.yml`, and `ci-infra-retry.yml`: **exit 1**, the five SC2086 findings listed above.
- Temporary verifier-crash probe through the extracted real scope step: **exit 42 and no matrix outputs**, confirming the missing fallback. An initial probe reporting statement used the wrong `StepRun` attribute; the corrected probe asserted the exit code and empty outputs.

These totals do not imply requirement completeness; the verification map and findings identify what the assertions actually establish. No full-workspace tests, WSL execution, terminal windows, hosted workflow triggers, repository commits, pushes, or ruleset changes were performed. Hook/workflow fixtures make commits and publish notes only in disposable local repositories.

GitNexus query/context and pre-edit document impacts were consulted. The index reports **94 commits behind HEAD** and document impact **UNKNOWN**, with no resolved callers/processes; this was not treated as proof of safety. Current source and review-chain text were inspected directly. No code symbol was edited.

The requested previous-review reference under `prompts/_reviews/fixes/` returns no match through `bf reference` (biscuit-file's FileReference CLI). The existing review resolves under `fixes/2026-09-11-cicd-cleanup/review-5.md`; its `implemented: true` is preserved and its `next` now points here. The specification records `review_iterations: 6`.
