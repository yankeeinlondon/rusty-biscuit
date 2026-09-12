---
title: CI cleanup closure checklist
kind: closure-checklist
created: 2026-09-12
spec: fixes/2026-09-11-cicd-cleanup/spec.md
review: fixes/2026-09-11-cicd-cleanup/review-7.md
candidate_base: f9de1ea42a36c5653b24e33e0198578bca1d7c49
status: in-progress
---

# Closure checklist

This pass follows the current spec's rulings, review 7, and the absorption
audit. The process-review recommendations remain separate work. Previously
established evidence stands unless this pass changes its inputs or exposes a
regression. A prepared fixture is not hosted evidence.

| ID | Owner | Closing condition | State |
|---|---|---|---|
| C1 / review 7.1 | native | Consumer and transitive native prerequisites reach only the owning Ubuntu check; planner, projection, and shipped-step regressions pass. | Complete; local regressions pass |
| C2 / review 7.2 / W2 | evidence | Advanced non-main target retains the exact reviewed plan, skips proven passing cells without reselection, and publishes exact-base receipts; another base is rejected. | Complete; local regressions pass |
| C3 / W8 / T3 | evidence | Signals and interrupted gate exit codes publish no complete validation; an explicit area override publishes no validation note. | Complete; local regressions pass |
| C4 / T4 / T5 | audit | Real scope step ignores validation notes on dispatch and falls back on malformed scope notes. | Complete; local regressions pass |
| C5 / W6 / W7 / W9 / W12 | evidence, native, audit | Each contract discrepancy has an implementation or explicit justified B0 disposition. | Complete; local regressions pass |
| C6 / W11 / R10 / AC14 | audit, evidence | README, testing strategy, active skills, and mode diagnostic describe the implementation. | Complete; local regressions pass |
| C7 / W13 | native | Required actionlint check passes. | Complete; local regressions pass |
| C8 / review 7.3 | orchestrator | Candidate hosted record proves nested labels, reuse without producers, gap/no-gap routing, artifact access, mixed cells, and accepted/unaccepted/missing-report/reused failure outcomes. | Prepared and locally verified; hosted execution blocked on authentication |
| C9 | orchestrator | Focused combined validation and independent delta review cover the completed candidate and checklist. | Local validation and independent delta review complete; final readiness awaits C8 |
| C10 | Ken, orchestrator | After hosted proof and the required candidate run, separately approve and verify the required-check migration. | Waiting for proof |

## Execution boundaries

- No WSL rerun, direct or indirectly triggered; no full-workspace validation.
- Review every triggered comparison, including bootstrap pushes and open PR
  bases. The old fixture's two WSL executions are prohibited, and a provisional
  comparison against main can select the full workspace.
- No fabricated validation receipts. Any fixture-only inputs/results must be
  clearly distinguished from product test evidence.
- Existing unrelated edits at entry: `prompts/_agent-skills.md`,
  `darkmatter/docs/inline/looping.md`, `darkmatter/docs/iterables.md`.
- GitHub CLI authentication is currently unavailable. Default HOME is
  `/Users/ken/.claudine`; the account under `/Users/ken/.config/gh` reports an
  invalid token. The connected GitHub app supports read-only inspection, but
  does not satisfy the authenticated CLI requirement in the pre-push hook.
- The remote currently has only the macOS notes ref. No WSL receipt was found
  in `git ls-remote origin refs/notes/ci-local/*` on this pass.
- The WSL prohibition is now recorded under both
  `/Users/ken/.claudine/.rusty-biscuit/ci-constraints/` and
  `/Users/ken/.rusty-biscuit/ci-constraints/`, repository-scoped, with a
  non-expiring sentinel date (`9999-12-31`). It requires an explicit superseding
  instruction to remove; session-home differences must not lose the restriction.

## Explicit B0 dispositions

These are bounded compatibility/policy follow-ups, not missing correctness
evidence. Owner: repository CI maintainers (Ken Snyder). B0 permits explicit
deferral; none of these entries invents a user ruling.

| Item | Disposition and reason | Closing condition |
|---|---|---|
| W6 | Defer removing the legacy scope projection from scope receipts. Both CI and the scope-verify CLI now derive their actual projection from the canonical plan; a poisoned compatibility field cannot suppress scheduling. Its stale schema comment is corrected now. | Migrate the receipt schema and compatibility fixtures together, proving old receipts either safely project or explicitly miss. |
| W9 | Retain existing measured, same-environment browser-cell reuse; defer alignment with the absorbed spec's browser-reuse exclusion. The existing implementation verifies completion and gate inputs and this pass introduces no browser execution. | Explicitly resolve the exclusion in the governing spec, preserving existing tested semantics or changing policy and its regressions together. |
| W12 | Defer removal of the legacy `verified_environment` CLI path. Current hook and workflow callers use per-cell verification; this compatibility API is not the active scheduling authority. | Remove the legacy command, helper, and its fixtures together after confirming all callers; retain supported v1-receipt verification through the per-cell path. |

## Verification record

The implementation remains uncommitted on candidate base
`f9de1ea42a36c5653b24e33e0198578bca1d7c49`. Unrelated user edits are excluded
from the prepared candidate patch. No commit, push, dispatch, PR creation, or
ruleset write occurred in this pass.

| Check | Result |
|---|---|
| `python3 -m unittest discover -s scripts/ci -p 'test_*.py'` | 464 passed |
| `.githooks/tests/test-pre-push.sh` with noninteractive Git and CDPATH unset | 66 passed, zero failed |
| `cargo nextest run -p test-toolkit --test ci_workflow_contracts` | 91 passed, zero skipped |
| actionlint on all five production CI workflows | Passed |
| ShellCheck on the pre-push hook | Passed |
| Scoped whitespace check | Passed; unrelated user edits excluded |
| GitNexus working-tree change analysis | Medium risk; 44 changed symbols, three affected processes; no partial/truncated result |
| Reduced fixture local Bash/Rust outcome verification | All seven expected outcomes passed |
| Generated fixture workflow actionlint | Passed |
| Every fixture plan against both constraint stores and `just ci-local --plan --plan-in` | Passed; Ubuntu-only execution, zero WSL cells |

After the combined suites, a focused reproduction exposed a malformed trailing
manifest row that could conceal an interruption after an earlier passing
report. The recipe now marks completion as indeterminate and withholds
publication when manifest parsing fails. The real-recipe regression passed
for that case, raw exit 130, and a tier wrapper normalizing exit 143 to 1.
The combined suite counts above precede this final narrow correction.

Independent review covered the evidence changes, native provisioning, and
fixture generator. It also caught an all-reused empty matrix condition and a
normalized interruption exit; both were corrected and covered by regressions.

Prepared artifacts live outside the working tree at
`/Users/ken/.claudine/.rusty-biscuit/ci-evidence/cicd-cleanup-review7/`:

- `candidate.patch` and `candidate-paths.txt`: scoped implementation snapshot.
- `hosted-overlay/`: generated workflows, measured local fixture reports,
  candidate Git blob identities, and local verification results.
- `fixture-checkout/`: isolated full candidate clone with the reduced overlay
  applied and source identity checks passed; no fixture commits exist yet.
- `plans/`: seven resolved fixture plans and their local previews.
- `logs/`: combined local verification output.

The fixture runbook is [hosted-fixture.md](fixtures/hosted-fixture.md).
Its seven dispatch cases and dedicated PR case still need real hosted results.
Local simulation does not establish GitHub scheduling, artifact permissions,
neutral-check placement on the PR head, or hosted producer omission.

## Remaining closure sequence

1. Restore authenticated GitHub CLI access and explicitly authorize the scoped,
   signed commits needed to publish only the isolated fixture branches.
2. Review the actual outgoing triggers against every active constraint, then
   execute the seven bounded cases and dedicated fixture PR. Record run URLs,
   candidate identities, artifacts, and expected per-cell outcomes.
3. Complete the focused readiness review from those records. The ordinary
   `feat/unifi` push remains prohibited while its plan selects seven WSL cells;
   fixture evidence does not supply product WSL receipts.
4. Obtain the required green candidate run through a constraint-compatible
   route, then seek Ken's separately required approval for the ruleset switch.
   Verify the required context after the approved change. Until then, neither
   the spec nor this closure record is complete.
