---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-17T23:29:23-07:00"
spec: 2026-10-17-dmls-index-store/spec.md
feature: 2026-10-17-dmls-index-store/review-1.md
implemented: true
description: "A **feature** review of `2026-10-17-dmls-index-store/spec.md`"
findings: []
observations: []
recurrence: false
human_review: true
has_blocked_findings: false
blocked: false
human_in_the_loop:
    - |-
        Decide whether DMLS may perform per-request database queries and temporary graph construction on the thread that handles editor messages. The plan placed store work on a worker; the implementation deliberately keeps this part inside the request handler to avoid adding asynchronous responses for four request types. Parsing and whole-workspace revalidation still use a worker.

        The implementation log reports about 10 ms for references and 54 ms for workspace symbols in its initial sample. Those measurements support the simpler implementation, but do not establish how promptly the server can handle another editor message while a slower query is running. For example, an edit or cancellation arriving during a workspace-symbol query must wait for the message-handling thread to become available.

        Choose one:

        - Accept the current design and its tradeoff: less response-management code, with other editor messages waiting while this request work runs.
        - Retain the planned worker design: move per-request database queries and graph construction off the message-handling thread before continuing the implementation review.
human_review_items:
    - |-
        Decide where DMLS performs per-request database queries and temporary graph construction. The implementation log explicitly departs from the plan by doing this on the thread that handles editor messages. Its initial sample reports 10 ms for references and 54 ms for workspace symbols, but other messages must wait while that thread is occupied.

        - Accept the current implementation to keep response management simpler.
        - Require the planned worker implementation so this work does not occupy the message-handling thread.
log: /Volumes/coding/wt/rusty-biscuit/feat-dmls-indexing/darkmatter/features/2026-10-17-dmls-index-store/log.md
implemented_by: claude/opus
next: 2026-10-17-dmls-index-store/review-2.md
---

# Human review needed before implementation review

The two requested document reviews are complete. One meaningful implementation decision needs the author's direction: where DMLS runs the database queries and temporary graph construction used to answer editor requests. No implementation-versus-spec review or test execution was performed in this iteration, as required by the human-review branch. Production readiness has therefore **not been assessed**; `ready` is intentionally unset rather than recording an unverified production-readiness verdict. No code findings have been assigned.

## Decision: where request work runs

The [plan's concurrency model](plan.md:416) puts database access on a worker. The [implementation log](implementation-log.md:653) explicitly records a departure: whole-workspace parsing and revalidation remain on a worker, but the database queries and temporary graph construction for individual requests run inside the editor-message handler.

This is a responsiveness-versus-complexity choice. The log reports initial measurements of roughly 10 ms for references and 54 ms for workspace symbols and concludes that adding asynchronous response management for four request types is not worthwhile at those times. That is a reasonable rationale, but a quick request measurement does not establish responsiveness when a request takes longer. For example, an edit or cancellation arriving during a workspace-symbol query waits until the message-handling thread is available. This is a consequence of the recorded design, not a reproduced code defect.

The author should choose whether to accept that tradeoff or require the planned worker implementation. This review does not impose a new response-time requirement or classify the departure as a blocking code finding.

## Decisions that do not need another approval

The plan calls several choices recommendations awaiting confirmation, and the log says the agent adopted them during non-interactive implementation. However, the [finalized spec](spec.md:48) explicitly says its decisions are author-confirmed. Its current requirements already include the warm-restart performance limits, conservative candidate matching, lock-file protection, multiple-root deduplication, and memory fallback. Those choices are not being reopened here.

The remaining recorded implementation choices are bounded details: no guessed Windows cache directory when `LOCALAPPDATA` is unavailable; conservative cache-override handling; hourly store touches; refreshing rename inputs before computing edits; and a fresh child process for the warm-memory benchmark. The log also limits automatic recreation to opening a store and switches later failures to memory, preserving readable or newly computed facts. These are disclosed for context and do not independently warrant stopping this review.

## Deferred follow-ups

The [final measurements](implementation-log.md:847) report warm references responses of 109–122 ms excluding discovery, 256–285 ms including discovery, and 42.9–46.2 MiB additional memory. Those reported results meet the spec's limits. The host was busy; a quiet-host rerun, cold-disk measurements, and additional operating-system performance samples remain optional author follow-ups. Their absence is not a code finding or a request to weaken the targets.

The [global storage-maintenance skill update](implementation-log.md:881) remains outside this repository change. The log supplies a cache-directory entry for the author to add to `~/.claude/skills/storage-strategy`. This is a small deferred documentation step, not an unresolved design question.

Earlier temporary deferrals, including server-level restart parse counting, are recorded as completed in later phases. Existing link-resolution and folder-rename limitations are explicitly unchanged or outside the agreed scope. Neither category is being reported as a finding.

Implementation review should resume after the author decides where per-request store work runs. No production-readiness conclusion or spec completion is recorded by this preliminary decision review.

## Author decision (2026-10-18)

**Accepted: the per-request store work stays on the dispatch thread.** Every DMLS request is answered synchronously on that thread (`router.rs`, `CancelLedger`), so giving these four requests a worker would make them the only ones answered asynchronously. The build's lock on the store is never contended by a request: `needs_index_build` makes a request wait while a build runs or the store is not ready, so `request_graph` and `symbol_documents` lock `facts` only when no worker holds it. If request latency needs a fix, it belongs in the server-wide `2026-10-11-dmls-request-timeouts`. `docs/lsp/indexing.md` ("Answering a request from the store") now states that these requests answer on the dispatch thread. The implementation review can resume.
