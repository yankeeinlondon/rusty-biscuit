---
area: darkmatter
status: draft-spec
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: false
review_iterations: 0
created: 2026-10-11
owner: Ken Snyder <ken@ken.net>
packages:
    - dmls
related:
    - 2026-10-11-clean-fidelity
human_review: true
references:
    investigation.md: Current-build headless investigation, including conditions, findings, limitations, and reproduction instructions. It demonstrates present queue blockage without establishing the historical incident's cause.
    measurements.json: Sanitized per-request measurements, timing notes, and process resource observations from the investigation. These are baseline evidence rather than future acceptance guarantees.
    probe.py: Bounded headless client exercising the server through its normal protocol. It records request and edit-send durations without retaining document bodies.
message_to_agent: |-
    Current-build investigation and two clarification batches establish
    behavior requirements and reference response targets. Do not begin
    implementation until remaining resource, operational, and completion
    rulings are resolved. No replacement architecture is selected.
implemented: false
---

# DMLS Request Timeouts

## Problem

The original incident report records that, in Zed on 2026-10-11, DMLS
repeatedly failed to answer ordinary editor requests within Zed's 120-second
Language Server Protocol request timeout. Its historical evidence is Zed's
log (`~/Library/Logs/Zed/Zed.log` and `Zed.log.old`), covering 09:39–14:24
that day. The counts and observations below are preserved from that report;
they have not been independently verified during clarification.

| Request | Timeouts |
| --- | --- |
| `textDocument/codeAction` | 18+ |
| `textDocument/documentHighlight` | 13+ |
| `textDocument/semanticTokens/full` | at least 1 |
| `textDocument/foldingRange` | at least 1 |
| `textDocument/documentLink` | at least 1 |

Failures were logged at 09:44, 09:45, 10:12, 10:16, 10:25, and 10:29. At
10:29:17, five different request types failed within the same second. That
pattern suggests requests were queued behind one blocked operation, rather than
each handler being slow on its own. This is a hypothesis.

The affected methods support cursor-driven queries (highlights, code actions)
or per-document queries (tokens, folding, links). Their actual cost in this
incident is unknown. When these stall,
the editor shows no Darkmatter feedback for minutes at a time, and every
cursor move queues another request.

### Context that may matter

- The installed binary (`~/.cargo/bin/dmls`, reports `dmls 0.1.0`) was built
  on 2026-09-24. Source has changed considerably since then, so the first step
  is to reproduce on a current build.
- Zed started DMLS three times in that log window. Each time, the working
  directory was the `feat-modern-config` worktree root, which is a large
  monorepo.
- Zed recorded hang traces (`~/Library/Application Support/Zed/hang_traces/`,
  10:19, 10:38, 11:52). These may or may not be related.
- The log shows no DMLS crash or panic.

## Goals and Boundaries

The fix must keep editor requests responsive while analysis is pending, prevent
obsolete work from publishing results, and bound queued and running work.
The author has approved reference response targets below. Concrete resource
and stopping limits still require explicit approval before implementation
planning.

`dmls` owns the investigation and the server's request handling, document state,
workspace analysis, cancellation, and publication. Changes to `darkmatter`,
which owns parsing and validation, are in scope only when evidence implicates
those operations. Changes to `zed-dmls`, which launches and configures the
server for Zed, are in scope only when evidence implicates launch or
configuration behavior. No scheduling or concurrency architecture has been
selected by these requirements.

The existing cross-platform baseline remains macOS, Linux, native Windows, and
WSL2. This fix does not authorize changing that baseline. Editor analysis must
remain passive: no shell execution, expression evaluation effects, remote
fetches, credential lookup, document mutation, or implicit network access.

## Investigation Authorization and Outputs

The author has authorized a bounded investigation of the current build through
the normal editor protocol, using a headless client and local builds. Replacing
the installed binary and launching, restarting, or controlling Zed are not
authorized. The authorized investigation has completed; its findings are
recorded in the adjacent evidence files listed in frontmatter.

Use the current monorepo and existing representative fixtures. Record the open
document, trigger (open, edit, save, or indexing), document bytes, workspace
file count, request mix, edit frequency, build mode, and host conditions for
each measurement. Report request latency and work behavior under those
conditions rather than presenting measurements as universal guarantees.

The investigation must:

1. Attempt to reproduce the incident with the current `dmls` build through its
   normal protocol path, distinguishing current behavior from the historical
   installed binary.
2. Locate operations that delay unrelated requests, including document
   notifications, workspace scanning, file resolution, and schema discovery.
3. Check for unbounded filesystem traversal and violations of passive
   analysis, including network access and execution of `::shell` content.
4. Measure cancellation, supersession by edits, and the amount of queued and
   running work; identify operations that cannot stop immediately.
5. Produce reproducible observations, candidate latency and workload limits,
   and the evidence supporting or rejecting the blocked-queue hypothesis.

If the historical incident is not reproduced, record that outcome and obtain
agreement on the remaining responsiveness requirements before proposing a
redesign. The investigation is limited to two hours of elapsed work, including
builds and measurement, and 150 seconds per server scenario. Start with small
existing fixtures before representative worktree documents. Stop early when
evidence is sufficient to propose targets, or report what blocked progress
within the budget. The completed investigation stayed within these limits.

Save sanitized findings and reproduction instructions beside this
specification, including timings, workload counts, build details, and relevant
host characteristics. Do not copy document bodies, full editor logs,
credentials, environment values, or personal absolute paths into evidence.

### Current Source Observations

Source review during clarification found synchronous request dispatch and
synchronous indexing/diagnostics on document changes. In `dmls`, `CancelLedger`
(the record of cancelled request identifiers) is checked before dispatch, and
`apply_change` (the document-change handler) performs indexing and diagnostics
synchronously. Generation stamps identify obsolete results but do not stop
the work that produces them.

Further inspection found whole-workspace graph rebuilding on each change and
a configured diagnostics debounce interval that is not currently applied.
These source observations do not prove the cause of the historical incident
or determine the replacement architecture.

### Current-build Findings

The measured release build did not reproduce a 120-second timeout. It did
demonstrate unrelated requests waiting behind document changes: approximately
0.79–1.25 seconds after a single edit in the populated workspace. An isolated
20-edit burst took 5.299 seconds from first edit send to the unrelated response,
including 3.532 seconds spent sending edits because of pipe backpressure.
Selected-document handlers subsequently took approximately 3–9 milliseconds
in the first populated-workspace run.

The isolated confirmation reached approximately 861 MiB peak process memory.
That lifetime high-water mark is not a queue-memory measurement. The limited
sample count, basic cursor positions, and host variability cannot establish a
performance distribution, cancellation stopping time, or resource budget.
The evidence supports fixing the demonstrated current queue blockage; the
historical timeout's exact cause remains unknown.

## Accepted Behavior

### Responsiveness and Measurement

The approved future maximum response targets are 250 milliseconds for
highlights, code actions, hover, and completion, and one second for full
semantic tokens, folding ranges, and document links. They apply under the
reference measurement conditions below, including unrelated long analysis;
they are not claims about the current build. A controlled regression must
also show unrelated requests completing while analysis is deliberately paused.

The reference is a release build on the recorded macOS Apple M4 Max host
(16 cores, 128 GiB memory), two open buffers, selected documents up to
12,242 bytes, the recorded 5,308-document corpus, and 20 successive full-text
edits. Cover both startup overlapping requests and a populated index. Record
the corpus and build revision, document bytes, workspace counts, request mix,
edit-send pattern, and host conditions for each run. The current corpus may
change; record the corpus actually tested rather than assuming it is immutable.

Run at least ten independent batches per scenario, exercising relevant methods
and phases. Report observed maxima against these targets, not inferred
percentile guarantees. Measure requests with a monotonic clock from immediately
before client framing/write to decoding the matching response, including pipe
and server queue delays. Also report edit-send duration and first-edit-to-first
response duration; no separate deadline for those edit measurements has been
approved.

This is a measurement envelope, not a hard support cap. Beyond it, the bounded
work and overload behavior below still apply, but numerical response guarantees
are not established. Numerical performance on other operating systems has not
been established; deterministic correctness remains required on every supported
operating system.

### Results While Analysis Is Pending

Answer from current text or analysis verified for the current document version.
When a feature needs analysis that is unavailable for that version, omit that
feature temporarily. Never present an earlier version's ranges or fixes as
current. Verify the document version immediately before publishing a result.

Clear obsolete diagnostics or retain only the subset verified as current, and
push current diagnostics when analysis is ready. Request-based features become
available on the next request, or through an applicable refresh mechanism when
the client has negotiated support. The server does not promise immediate
redisplay in every editor.

Known changes to a schema, referenced document, configuration, or resolution
context invalidate affected results, rather than invalidating unrelated
documents. Open buffers are authoritative; closed files use disk observations
adopted by the server. This does not require a simultaneous snapshot of every
workspace file. A result must not claim freshness against a known changed
dependency merely because the open document's version is unchanged.

### Cancellation and Obsolete Work

Use cooperative stopping at safe checkpoints. After cancellation is handled,
perform no further request-specific work beyond reaching a safe stopping
checkpoint. Work shared with another request or still needed for the current
document may continue.

A newer edit replaces queued analysis for older versions. Running obsolete
analysis stops at safe checkpoints and cannot publish. Queued and running work
must have explicit limits. Work that cannot be interrupted may finish, but it
must neither block the interactive request path nor accumulate without bound.

Every accepted request receives exactly one terminal response. If the server
observes cancellation before the request completes, return the protocol
cancellation response. A completed request does not acquire a second response
when a later cancellation arrives.

Numerical stopping and resource limits require measurement and human approval.
The meaning of a safe checkpoint remains to be specified. These rules define
required behavior rather than a choice of threads, tasks, or queues.

### Work Limits and Overload

Bound queued work and running work separately. A newer document version replaces
older queued analysis while retaining the fact that the document still needs
analysis. Preserve shared work that remains useful. Schedule documents fairly
so that, after an edit burst ends, each document's latest version is eventually
analyzed; dropping old queued versions must not permanently drop analysis.

When analysis is unavailable or a work limit is reached, requests return current
results where possible, otherwise a protocol-valid empty or unavailable result
promptly. They must not wait for space in an analysis queue. Exact response
encoding remains method-specific and must preserve protocol validity.
Concrete queued/running limits and stopping budgets are still unresolved.

## Acceptance Criteria

The following behavior and reference response targets are approved. Resource
and stopping thresholds remain unresolved; final completion criteria are pending.

| Requirement | Validation |
| --- | --- |
| Approved response maxima | At least ten independent batches per reference scenario; every measured relevant request meets its class's 250 ms or one-second maximum using the approved clock |
| Isolation from long analysis | Pause one document's analysis through an implementation-time control and verify unrelated requests complete before release; use the approved maxima on the reference host |
| Current results only | Edit while analysis is paused, release it, and verify no obsolete ranges, fixes, or results publish; clear stale diagnostics or preserve only a verified current subset |
| Dependency freshness | Change a known schema, referenced document, configuration, or context; verify affected results invalidate and unrelated documents remain usable, including authoritative open buffers |
| Feature recovery | Verify current diagnostics push when ready; request-based features recover on the next request and applicable negotiated refresh is honored |
| Cancellation responses | Observe cancellation before completion and verify one cancellation response; verify later cancellation produces no duplicate response |
| Cancellation stopping | Measure client-send-to-server-receipt and receipt-to-stop intervals separately, observing request-specific work and shared work; the stopping threshold remains pending |
| Supersession and fairness | Burst edits on multiple documents with paused analysis; verify old queued versions are replaced, obsolete running work cannot publish, shared useful work remains, and latest versions are analyzed after the burst |
| Bounded overload | Observe queued and running work separately, including uninterruptible operations; requests return current or protocol-valid empty/unavailable results without waiting for analysis capacity; numerical limits remain pending |
| Passive analysis | Retain sentinel-based coverage through the normal protocol; parsing/validation changes also require library coverage |
| Cross-platform correctness | Run deterministic protocol and work-management checks on macOS, Linux, native Windows, and WSL2; reference numerical performance applies only to the approved macOS conditions |

The current baseline probe does not implement pause controls, stopping events,
or work counters. Those validation controls will be needed during implementation;
they must not be described as evidence already gathered.

## Remaining Human Rulings

- Concrete limits for queued and running work, safe-checkpoint meaning, and
  numerical cancellation/obsolete-work stopping budgets.
- Required operational evidence, diagnostics/logging and privacy boundaries,
  rollout/fallback, and final acceptance criteria and definition of done.

Implementation planning remains blocked on these rulings. The author owns the
decisions. Clarification and the investigation do not authorize implementation
or imply that the historical cause is known.
