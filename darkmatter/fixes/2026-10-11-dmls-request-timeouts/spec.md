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
message_to_agent: |-
    Placeholder draft. It records the evidence observed on 2026-10-11 and the
    questions to answer first. Do not plan or implement from it until the
    investigation section is completed and the author has fleshed out the
    expected behavior.
implemented: false
---

# DMLS Request Timeouts

## Problem

In Zed on 2026-10-11, DMLS repeatedly failed to answer ordinary editor
requests within Zed's 120-second LSP request timeout. The only evidence so far
is Zed's log (`~/Library/Logs/Zed/Zed.log` and `Zed.log.old`), which covers
09:39–14:24 that day.

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

Every timed-out request was a cheap, cursor-driven query (highlights, code
actions) or a per-document query (tokens, folding, links). When these stall,
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

## Investigation (to do)

1. Reproduce with a current `dmls` build in this worktree, and record which
   document was open and what triggered the stall (opening the file, editing,
   saving, or workspace indexing).
2. Find out whether DMLS handles requests one at a time, and whether any
   handler or notification, such as `didOpen`/`didChange` analysis, workspace
   scanning, transclusion resolution, or schema/trigger discovery, can run long
   enough to block the queue.
3. Check whether any work does unbounded filesystem traversal or network
   access, or runs `::shell` content, while answering an editor request.
4. Check whether DMLS honors `$/cancelRequest`, so that requests Zed has
   abandoned don't keep using the server.

## Expected Behavior (to be fleshed out)

- Interactive requests (highlights, code actions, hover, completion) answer
  quickly, or come back empty or cancelled. They never wait behind long
  analysis.
- Long-running analysis runs off the request path, can be cancelled, and is
  superseded by newer document versions.
- Cancelled or superseded requests stop consuming work.

## Acceptance Criteria

To be written once the investigation identifies the cause.
