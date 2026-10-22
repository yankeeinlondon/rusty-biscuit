---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-21T20:38:25-07:00"
spec: 2026-10-27-darkmatter-lifecycle-ownership/spec.md
feature: 2026-10-27-darkmatter-lifecycle-ownership/review-1.md
implemented: false
description: "A **feature** review of `2026-10-27-darkmatter-lifecycle-ownership/spec.md`"
ready: true
review_scope: implementation-decision-triage
technical_review_pending: true
findings: []
observations: []
recurrence: false
human_resolved: false
human_review: true
has_blocked_findings: false
blocked: false
human_review_items:
    - Resolve the implementation choices documented in human_in_the_loop before the technical review proceeds. Production readiness has not been assessed.
human_in_the_loop:
    - |-
        Decide whether shared lifecycle decisions are sufficient ownership, or whether every Claudine run must also use Darkmatter's request/reply protocol.

        The implementation log reports that Darkmatter now owns the shared accounting and decision functions, but Claudine still drives execution through its existing direct-call interfaces. Only the md host uses the request/reply protocol. That protocol is the spec's mechanism for rejecting stale, duplicate, foreign, or wrong-kind replies, particularly after cancellation. Sharing decision functions does not by itself establish that every caller receives those protocol guarantees.

        - Accept the shared decision functions and existing Claudine drivers. This avoids the log's estimated 15,000 lines of driver changes and about 200 test-call changes, but explicitly narrows the planned common protocol boundary.
        - Require every driver to use the protocol. This follows the engine design and warrants allowing necessary test setup changes while preserving behavioral assertions.

        Recommendation: retain the planned protocol boundary unless the author deliberately accepts a narrower ownership goal. This is an architectural decision, not a routine choice of Rust signatures. Source: implementation log, lines 1697–1753 and 2249–2258.
    - |-
        Choose whether early agent-resolution and schema-validation failures should fire lifecycle cleanup events.

        The implementation preserves existing Claudine behavior: failures classified as occurring before the run fire neither blocked nor finalize. For example, an invalid agent selection can exit without running an authored finalize action. The spec's preflight section and acceptance criterion 9 instead require blocked followed by finalize. The log explicitly chooses behavior preservation over that requirement. It also reports that most checks open separate one-check runs, so their combined order is not enforced by one shared preflight run; the sequence path is the exception.

        - Preserve the existing no-event behavior for these early failures and explicitly exempt them from the event requirement. This avoids changing existing cleanup behavior and test assertions.
        - Require blocked and finalize for all specified host-check failures. This follows the stated preflight contract but changes existing Claudine behavior and needs corresponding behavioral test updates.

        Recommendation: preserve the early no-event behavior as an explicit exception. Also decide whether the shared preflight process must enforce the complete check order, or whether the existing drivers may continue to enforce it. Source: implementation log, lines 1757–1786 and 1863–1915.
    - |-
        Decide whether reusing preflight discovery results remains required for this implementation.

        The spec explicitly requires reuse of the approved discovery graph. The log says Claudine still composes twice because preflight and final composition use different options; it treats reuse as a performance follow-up without completing it here. Repeated work remains on the normal execution path, even though it preserves previous behavior. Remote-fetch reuse is separately reported as inapplicable because Claudine does not enable remote reads.

        - Complete discovery reuse now, preserving the different validation phases and existing approval checks.
        - Accept repeated composition as a deliberate scope reduction and state that the reuse requirement is deferred.

        Recommendation: retain the explicit reuse requirement unless investigation demonstrates that safely sharing the graph requires disproportionate changes. This triage has not verified the log's implementation-cost explanation. Source: implementation log, lines 1898–1915 and 2259–2262.
    - |-
        Approve or replace the new public schema-reference syntax Name@darkmatter:<stem>.

        The log reports a reserved namespace that always resolves a type from Darkmatter's shipped schema definitions. It lets Claudine's schema references work in the checkout, embedded bundles, and isolated repositories containing only Claudine's schemas. For example, a reference using this namespace does not depend on a neighboring Darkmatter directory. This solves the packaging problem but establishes a public syntax and resolution rule beyond the spec's instruction to share action definitions.

        - Accept the reserved namespace and its rule that shipped definitions are authoritative. This supports all three environments with one reference spelling.
        - Keep public references unchanged and provide embedded resolution through another mechanism. This avoids the new syntax but requires a replacement design for files used outside the checkout.

        Recommendation: accept the reserved namespace; its packaging purpose is concrete. Source: implementation log, lines 2567–2582.
    - |-
        Decide whether the recorded md execution limits are acceptable for the initial lifecycle host.

        The log says an interrupt during composition waits for composition to return and then discards its result. A lifecycle command already starting may finish; a second interrupt force-exits. Lifecycle shell output is captured and replayed rather than inherited directly from the terminal. Lifecycle-enabled composition also omits the existing performance and verbose summaries. These choices affect cancellation responsiveness, command output, and troubleshooting.

        The log additionally reports that every err field except msg is null under md because no diagnostic classification catalog is supplied. A handler inspecting err.code therefore cannot distinguish failure categories there, even though the design describes a structured error envelope.

        - Accept these limits explicitly for the initial md host, with the current documentation describing them.
        - Require cancellation responsiveness, output/reporting parity, and structured error classification before accepting the host. The author can select these individually; they need not move together.

        Recommendation: accept cancellation after in-progress work returns and captured output as initial limits, provided the later technical review establishes that cancelled work cannot publish a document. Retain existing reporting options and meaningful error classifications unless explicitly removed from scope. Source: implementation log, lines 2334–2353, 2920–2936, and 3101–3106.
    - |-
        Choose whether editor replacement guidance must appear in the diagnostic itself.

        The spec requires a Claudine document using composed instead of start to receive a schema error naming start. The log reports that DMLS instead provides that replacement guidance only in hover; adding it to the diagnostic was reverted to preserve existing exact-message assertions. A developer reading only the Problems panel therefore does not receive the requested replacement guidance.

        - Keep guidance in hover and accept the weaker diagnostic requirement.
        - Include the replacement in the diagnostic and permit the necessary message-assertion changes.

        Recommendation: retain the requested diagnostic guidance. The no-behavior-change requirement specifically protects Claudine behavior; preserving all existing editor message assertions should not silently override a new editor requirement. Source: implementation log, lines 2723–2729; spec acceptance criterion 13 and the host-names section.
---

# Review paused for implementation decisions

Two independent agents inspected the specification and implementation log. The specification records no open questions. The log nevertheless records consequential architectural departures, a conflict between preserving existing behavior and changing preflight failure events, and unfinished discovery reuse. These need author decisions before the implementation can be reviewed against an agreed contract.

The six entries in `human_in_the_loop` above describe the choices, consequences, examples, recommendations, and source locations. They are also the requested prose report: the review stops here, without comparing implementation code against the specification.

## Readiness and review scope

Production readiness is **not assessed**. No code review, robustness sweep, test-tier audit, or test execution was performed in this decision-only review. There are no technical findings yet, and no previous findings to resolve in iteration 1.

`ready: true` records only the requested rule that readiness is false exactly when findings exist; it is not a production-readiness endorsement in this paused review. `technical_review_pending: true` and the unresolved `human_in_the_loop` list identify the remaining review work. The spec's `completed` field follows the requested mechanical update when `ready` is true; it likewise does not resolve these choices or substitute for technical review.

## Other recorded decisions and scope

The log reports that Claudine retains its existing diagnostic error variants through an exhaustive adapter with parity checks. This is consistent with preserving its public behavior and does not independently require a human decision. Rust naming, re-exports, and placing the lifecycle module inside the existing Darkmatter crate are similarly ordinary implementation choices.

The log also records no new `md compose --dry-run` flag, missing `then`/`else` completion under the md lifecycle schemas, runtime-only enforcement of required `then` in that projection, no production invocation of the event-position checker, and an additional roughly 72–87 ms of debug schema-validation cost. These are disclosed for the subsequent technical review, not classified as verified defects or approved scope changes here. Report-only `--shell` is the passive path cited by the implementation.

Differences between the captured recovery behavior and the spec's descriptive recovery table do not independently require renewed approval: the spec explicitly makes the starting traces authoritative when those disagree. The log says the missing type-narrowing prerequisite was completed following an earlier author ruling, and the earlier trace and input-matrix gaps were subsequently addressed. Those historical blockers are not presented as unresolved work.

## Findings

None recorded. Technical review is pending resolution of the implementation decisions.

## Observations

None recorded; the implementation has not been inspected for defects in this review.
