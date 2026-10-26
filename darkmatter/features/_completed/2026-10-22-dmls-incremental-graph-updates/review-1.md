---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-24T07:16:18-07:00"
spec: 2026-10-22-dmls-incremental-graph-updates/spec.md
feature: 2026-10-22-dmls-incremental-graph-updates/review-1.md
implemented: true
description: "A **feature** review of `2026-10-22-dmls-incremental-graph-updates/spec.md`"
ready: false
findings: []
recurrence: false
human_in_the_loop:
    - |-
        Decide whether this delivery should repair the deferred behavior when an already referenced Markdown file is opened with unchanged text. The implementation log reports that the published graph still treats the file as a reference-only dependency until another rebuild. Such dependencies are excluded from wiki-link candidates. For example, if an open document references an unlisted `notes.md`, DMLS can load its headings for navigation; opening that same file without editing it does not immediately make it eligible for wiki lookup.

        The implementation preserved the existing unchanged-text shortcut, citing the plan's instruction to retain publication behavior. Opening the file nevertheless changes its role, so the choice affects which files wiki links can find, rather than performance alone.

        - **Include the small repair (recommended):** rebuild when opening a file changes it from reference-only to open, even if its bytes are identical. Keep the unchanged-text shortcut when the file's role is unchanged. This makes wiki eligibility reflect the open buffers immediately and adds one rebuild for this transition.
        - **Explicitly retain the existing behavior for this delivery:** preserve the shortcut and accept delayed wiki eligibility until another rebuild. This avoids expanding the change but leaves the reported inconsistency in place.

        Record the selected behavior under Human rulings before the technical review resumes. The behavior and proposed repair are reported by the implementation log; this paused review has not independently reproduced them.
human_resolved: false
human_review: true
human_review_items:
    - |-
        Select how opening an unchanged, previously referenced-only file should affect wiki lookup, and record the choice under Human rulings.

        - **Recommended:** include the role-transition repair so the newly opened file becomes eligible immediately; unchanged edits to an already open file remain a no-op.
        - **Alternative:** retain the reported delay until another rebuild and explicitly accept that limitation for this delivery.

        This is the same decision recorded in human_in_the_loop, not a second approval request.
has_blocked_findings: false
blocked: false
log: /Volumes/coding/wt/rusty-biscuit/feat-merman/darkmatter/features/2026-10-22-dmls-incremental-graph-updates/log.md
implemented_by: claude/opus
next: 2026-10-22-dmls-incremental-graph-updates/review-2.md
---

# Review paused for the deferred file-opening decision

The initial document review found a deferred, user-visible behavior that needs an author decision before this review continues. Your first-cycle instructions require stopping when implementation work was deferred. No implementation-to-spec comparison, reproduction, or test execution was performed in this cycle.

Production readiness has **not been assessed**. The `ready: false` value records the incomplete review, rather than a technical rejection. The findings list is empty because this is the human-decision branch of the review, not a completed technical review. The specification is not marked completed.

## Decision needed

The [implementation log](implementation-log.md:58) reports that a Markdown file loaded only because another document references it remains classified as a dependency when the user opens it with identical text. Dependencies deliberately do not participate in wiki-link matching. The graph retains that classification until a later rebuild, even though the file is now open.

For example, an open `guide.md` can reference an unlisted `notes.md`, causing DMLS to load its headings for navigation. Opening `notes.md` without changing it leaves it excluded from wiki candidates until another action rebuilds the graph. This example illustrates the logged behavior; it is not a reproduction performed by this review.

The implementation explicitly left this behavior unchanged because the plan preserves the shortcut for unchanged content. That raises a scope decision: should that shortcut also suppress publication when a file's role changes?

- **Include the repair, recommended.** Rebuild specifically when opening the file changes its role from referenced-only to open. This updates wiki eligibility immediately while preserving no-op handling when both text and role are unchanged. The log describes this as a small change; implementation review should verify that assessment and cover the transition.
- **Retain the behavior explicitly.** Accept delayed wiki eligibility for this delivery. This keeps the existing publication behavior but leaves the reported inconsistency visible to users.

The decision is needed because the implementation knowingly deferred this behavior; this review does not treat every internal implementation choice as requiring approval.

## Other implementation decisions

The two requested sub-agent audits reviewed the implementation log and the specification/plan. They found no other unresolved product or architecture decision:

- **Lookup ownership:** the shared table caches wiki filename lookup facts; frontmatter keys are rebuilt from content. The plan already clarifies the specification's ambiguous “frontmatter-key lookup” wording. Editing a frontmatter key therefore still needs to update its references.
- **Request-graph path facts:** a document whose supplied wiki slot deliberately cannot match receives real path facts separately. The log says this preserves existing navigation and collision behavior.
- **Internal API and visibility:** the reuse builder accepts the shared universe directly, and assembly helpers remain private. These are implementation details with no reported change to editor behavior.
- **Phase sequencing:** the final implementation phase completed the previously missing equivalence and server-session tests. The work was completed rather than deferred.
- **Publication verification:** the log assigns the no-new-snapshot assertion to an in-crate test, with the server session checking zero graph work and unchanged diagnostics. Whether that evidence satisfies the contract remains for technical review.
- **Remaining performance work:** further optimization of candidate-input comparison was intentionally excluded by the agreed scope. The reported final sample met the decision aids; neither outcome authorized another optimization. This is not unfinished required work.

These are document-audit conclusions, not independent verification of the code or the reported tests and performance figures.

## Findings

None recorded. The deferred behavior above is an author scope decision at this stage, not an independently verified defect finding. There are no previous review findings to reconcile in iteration 1.

## Human rulings

Pending. Record the selected file-opening behavior here and set `human_resolved: true` before resuming technical review.
