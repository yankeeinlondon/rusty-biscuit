---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-17T17:23:11-07:00"
spec: "2026-10-25-loops-and-flow-control/spec.md"
feature: "2026-10-25-loops-and-flow-control/review-1.md"
implemented: false
description: "A **feature** review of `2026-10-25-loops-and-flow-control/spec.md`"
# No technical findings were gathered: this cycle stopped at the decision audit.
# The nonempty human_in_the_loop list pauses review regardless of ready.
ready: true
readiness_assessed: false
findings: []
observations: []
recurrence: false
human_review: true
human_resolved: false
has_blocked_findings: false
blocked: false
human_in_the_loop:
  - |-
      Decide when a sequence must obtain approval for shell commands in helpers called with `prep`. The spec requires advance inspection of every helper whose target is already known, including helpers it calls. The implementation log says each helper instead obtains approval when reached. Earlier steps could therefore change files before a later helper is refused approval.

      - Keep advance inspection: preserves the promised opportunity to approve the sequence's known shell work before execution, but requires connecting helper discovery to the parent's approval inventory. Recommended.
      - Accept approval at each call: retains the current implementation and reduces this release's scope, but permits approval failures after earlier work has run. Record this as an intentional scope reduction in the implementation log and current docs.

      Record which approval contract this release should provide. Sources: spec section 5, “Preparation,” and implementation-log.md lines 991–998.
  - |-
      Decide whether `prep` must support standalone group files in this release. The spec permits documents, sequences, and files declaring `kind: group`. The implementation log says group targets fail. For example, `prep: "@checks.md"` cannot call a standalone group of parallel checks.

      - Implement group targets now: preserves the same helper-call interface for all three executable primitives. Recommended because this uniformity is a central feature goal.
      - Defer group targets: retain document and sequence calls, document the restriction and its error, and record the reduced scope. This avoids adding a group entry point in this cycle.

      Record whether group targets remain required now or are explicitly deferred. Sources: spec section 5's target parameter and implementation-log.md lines 999–1002.
  - |-
      Clarify whether helper calls need a shared total time budget. The spec says a helper inherits the caller's remaining wall-clock deadline without resetting it. The implementation log says no overall deadline exists: each provider child receives the inherited timeout settings separately. For illustration, three children with a one-minute limit could together take about three minutes; that setting is not a one-minute budget for the whole helper.

      - Retain per-child limits for this release: matches the existing timeout model and avoids introducing an overall timeout system solely for helper calls. Recommended unless a total invocation budget is an intended feature requirement. Document this departure explicitly.
      - Require a shared remaining deadline now: provides the stated total-budget guarantee, but requires defining where that deadline starts and carrying it through caller and helper execution while preserving tighter child limits.

      Record the intended timeout contract; this is a scope question, not evidence that cancellation is broken. Sources: spec section 5, “Shared limits,” and implementation-log.md lines 1003–1007.
human_review_items:
  - "Choose advance approval or approval at helper entry for known prep targets; see the first human_in_the_loop item for consequences and recommendation."
  - "Choose whether standalone group files must be callable with prep now or are deferred; see the second human_in_the_loop item."
  - "Choose per-child timeout limits or a shared total deadline for helper calls; see the third human_in_the_loop item."
---

# Review 1: human decisions required

This review pauses for three meaningful scope decisions. **Production readiness has not been assessed.** Two independent document reviewers examined the spec and implementation log; both identified the same unresolved departures. No implementation comparison, defect sweep, robustness matrix, or test execution was performed, as required for this course of action. The statements below describe what the implementation log reports, rather than independently verified runtime behavior.

The `ready` field is true only under the requested rule that it is false exactly when there is a high finding; this document contains no technical findings. It is **not production approval**. `readiness_assessed: false` and the unresolved `human_in_the_loop` list distinguish this decision audit from a completed implementation review. The spec's `completed` field follows the requested mechanical update when `ready` is true; the unresolved human decisions still pause the review cycle.

## Decisions requiring review

### Approve helper commands before execution or at each call

Claudine's `prep` action calls a helper and then resumes its caller. The [spec's preparation contract](spec.md:426) says a sequence inspects all statically known helper targets transitively before execution. The [implementation log](implementation-log.md:991) reports that this advance inventory was omitted; the helper instead undergoes its own approval when called, using the caller's approval flags and policy.

For example, a sequence could modify source files and then call a helper that runs a shell check. Under advance inspection, the known check belongs to the initial approval request. Under the logged behavior, approval for that helper is handled after the source edits. This changes when a sequence can discover that it cannot finish. The log does not establish an approval bypass; the meaningful choice is the timing and scope of approval.

**Recommendation:** retain advance inspection. Alternatively, explicitly accept approval at helper entry as a scope reduction. Record the chosen contract before technical review resumes.

### Include standalone groups as callable helpers or defer them

The [target parameter](spec.md:418) accepts a document, a sequence, or a standalone `kind: group` file. The [implementation log](implementation-log.md:999) says the group target is rejected, and an attempted synthetic sequence wrapper encounters a self-reference check.

For example, a user cannot directly call a group of parallel checks with `prep: "@checks.md"` when that file declares `kind: group`. **Recommendation:** support group targets now to preserve a consistent call interface across the feature's three primitives. Deferring them is a possible release-scope decision, but needs explicit authorization and clear documentation of the restriction.

### Define whether time limits apply per child or to the entire call

The [shared-limits contract](spec.md:460) requires the caller's remaining deadline to pass into the helper. The [implementation log](implementation-log.md:1003) reports that the existing timeout settings apply per provider child and that no invocation-wide deadline was added. Its [final acceptance inventory](implementation-log.md:1314) still identifies this contract and advance approval as uncovered.

These models give different guarantees: three children can each receive a full timeout under the existing model, whereas a shared deadline limits their combined time to the caller's remaining budget. **Recommendation:** explicitly retain the existing per-child model unless a total invocation budget is intended now. Adding a new overall deadline solely to satisfy wording would be a larger design commitment; if chosen, its start point and interaction with tighter limits must be defined.

## Decisions already settled and deferred scope

The [author rulings](implementation-log.md:666) already confirm document-only rate-limit handling, file-probe error and null behavior, recursive-path misses, interpolated break reasons, and cap-override precedence. These need no renewed approval. In-process re-entry through existing command entry points is an implementation choice, not another product decision.

The same rulings explicitly defer migration of the retired `review-loop.md` prompt and its rehearsal acceptance criterion; `just fix` remains the workflow. This is authorized deferred scope, not a new question. `defer` and persisted checkpoint/resume are already [out of scope](spec.md:764).

The log also records incomplete native-Windows behavior-test coverage for prompt-driven loop, break, prep, and dry-run-shell paths, and absent summaries for completed or exhausted loops. Those are inventory for the subsequent technical review, not additional design questions or findings in this document. The pre-existing nested-inline-sequence and stdout backslash limitations likewise receive no defect classification here.

## Findings

None gathered. After the three choices are recorded, resume the implementation review against the spec and the authorized departures, including the appropriate public-API input checks and requirement-to-test mapping.

## Observations

None gathered.
