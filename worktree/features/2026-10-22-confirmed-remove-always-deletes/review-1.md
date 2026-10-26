---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-23T17:48:13-07:00
spec: 2026-10-22-confirmed-remove-always-deletes/spec.md
feature: 2026-10-22-confirmed-remove-always-deletes/review-1.md
implemented: false
ready: false
description: "A **feature** review of `2026-10-22-confirmed-remove-always-deletes/spec.md`"
findings: []
observations: []
recurrence: false
human_resolved: false
human_review: true
has_blocked_findings: false
blocked: false
human_in_the_loop:
    - |-
        **Choose how long to spend checking which programs use a large checkout.**

        The implementation log reports that the two-second discovery allowance expired before any programs were checked on a checkout with 500,000 entries. Walking the files alone took about 3.5 seconds with a larger allowance. The confirmation therefore warned that some programs could not be checked. A script without a terminal refused removal unless given `--force`, even if no program was actually using the checkout. Deletion itself succeeded.

        This matters because built checkouts of this size are a normal use case for this feature. The spec permits incomplete discovery within a budget; the decision is whether this predictable loss of process information is acceptable in that normal case.

        - Keep two seconds: preserves the current allowance, but large checkouts can routinely lack process information and require `--force` in scripts.
        - Increase the allowance, for example to ten seconds: a small implementation change that gives the measured checkout time to reach process discovery, at the cost of a longer possible wait before confirmation. It does not guarantee complete discovery on every checkout.
        - Change the shared Sniff library so process discovery does not first require walking every checkout entry: potentially improves both speed and information, but requires a broader implementation and testing effort.

        Recommendation: increase the allowance now and defer a shared-library redesign until measurements justify it. Record the chosen allowance and align the documentation's “Which programs are using the checkout” section with it.
    - |-
        **Decide which permission protection is acceptable for saved removal approvals on Windows drives accessed through WSL.**

        An interrupted removal keeps an approval file that allows a later invocation to finish deleting the selected checkout without asking again. The implementation normally rejects storage whose Unix permission bits allow other users access. On the tested WSL Windows drive (`/mnt/c`, using drvfs without its metadata option), entries reported permissions equivalent to everyone having full access, and changing those bits had no effect. Every removal failed before deletion.

        The agent changed the check on drvfs to verify ownership while accepting these permission bits. Protection then depends on Windows permissions that this check does not inspect. This enables the spec's required WSL support, but changes what the program trusts for an approval that can authorize deletion.

        - Retain the exception: supports these drives, relying on their Windows access controls in addition to the retained ownership and target checks.
        - Store approvals in a suitably protected location outside drvfs: preserves support with a different storage design, but needs implementation work and validation of recovery and repository association.
        - Refuse this environment and direct users to native Windows: avoids accepting permissions the check cannot verify, but requires explicitly changing the spec's WSL support requirement.

        Recommendation: retain the exception only if relying on the Windows access controls is an accepted trust boundary; otherwise pursue protected storage. Ownership alone is not evidence that other users cannot alter the approval. Update “The saved approval” documentation to match the decision.
human_review_items:
    - Confirm the time allowance for checking programs on large checkouts, choosing among the options described above.
    - Accept the Windows-permission dependency for saved approvals on WSL Windows drives, or select protected storage or an explicit change to supported environments.
log: /Volumes/coding/wt/rusty-biscuit/fix-remove-large-checkout/worktree/features/2026-10-22-confirmed-remove-always-deletes/log.md
implemented_by: claude/opus
next: 2026-10-22-confirmed-remove-always-deletes/review-2.md
---

# Review paused for two human decisions

Production readiness has **not been assessed**. This first review follows the requested human-decision route: `ready: false` records that pause, rather than a blocking code finding. No implementation-versus-spec review or test execution was performed, and there are no finding IDs yet.

Two independent agents read the spec and implementation log. The spec says its design was accepted and has no open questions. The implementation log, however, explicitly leaves the process-discovery allowance and WSL approval-storage permission policy open in its final handoff. The two frontmatter items explain the choices, consequences, and recommendations.

## Other decisions recorded during implementation

The agent adopted the plan's proposed implementation rules without human confirmation. Routine choices included record version 2, three unsuccessful directory-refill passes before waiting, an interruptible retry delay starting at 50 milliseconds and capped at two seconds, conservative treatment of unknown filesystem errors, and using an operation identifier in the ten-minute single-use handoff. These fill implementation details and do not independently justify stopping this review.

Other rules define how local submodule commits are identified without fetching remote data, which incomplete usage checks trigger a warning, and which exit codes describe incomplete removal. These should be checked against the accepted contract during the resumed code review; they are not new requirements imposed by this review.

The Phase 5 log also records departures worth retaining in the review handoff: special targets such as missing checkouts and root links used no durable approval and could ask again after a crash; checkout identity was bound after confirmation; and an approved unlock could remain after a refused move. Those are historical phase notes, not verified statements about the final implementation. The later review must reconcile them with the final code and the spec's save-before-unlock, target-binding, and silent-resume requirements. They must not be treated as approved exceptions merely because they appear in the log.

## Deferred work and verification

The final documentation phase explicitly did not verify what happens when a Windows program holds a file open in a checkout removed through WSL's Windows-drive access. The spec requires waiting for that file to be released. The log notes that an access-denied error might instead be treated as permanent and omits the waiting claim from the documentation. This is deferred behavior verification, not a request for the author to waive the requirement. After the decisions above, the code review should establish the behavior and request a repair if the accepted contract is unmet. Cross-platform test evidence by itself is not a readiness finding.

Earlier notes also leave long-path display hyphenation in the shared terminal library and a load-sensitive process-discovery allowance without a test override. A note about cancellation between confirmation and movement cites machinery removed later; it needs reconciliation rather than being presented as a confirmed current defect. Documentation cleanup deferred in earlier phases was reported finished in the final phase.

## Next review

Record the author's choices under `## Human rulings` and set `human_resolved: true` when both decisions are settled. Then resume the implementation review, including the recorded departures and deferred behavior above. No prior review findings exist to carry forward.
