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
clarified: false
created: 2026-10-22
owner: Ken Snyder <ken@ken.net>
packages:
    - dmls
human_review: true
needs_rulings: false
---

# DMLS: a flood of watched-file creations pins a core

## Problem

Three DMLS processes launched by Zed (one per worktree) each ran at about
100% of a core for 22 hours to 2 days (330–560 CPU-minutes each) on
2026-10-22, while the developer was not typing. A 3-second `sample` of one
put 86% of the main thread's samples in
`Router::dispatch_notification` → `WorkspaceIndex::disk_changed`, and most
of that in `WorkspaceIndex::rebuild` (`WorkspaceGraph::build_listed`, the
`WikiUniverse`, and the Unicode-normalizing `canonical_segments` /
`portability_key` over every listed path), with `regex_automata` DFA
construction (ignore-glob compilation) beside it.

The installed binary predated two cache commits on `main`, so the defect
was reproduced on `main` at `3884a0743`: one
`workspace/didChangeWatchedFiles` notification creating 200 Markdown files
under a `.gitignore`d `target/` in a 3,000-document workspace kept the
main thread busy for **8.3–8.9 s** (debug build), about 42 ms per created
file the walk never lists. A build or test run that writes Markdown under
the workspace produces such events continuously.

## Cause

Each created path in a notification is applied on its own
(`Router::apply_disk_change`), and three costs compound:

1. **An unlisted Markdown creation rebuilds the whole graph.**
   `WorkspaceIndex::unlisted_changed` rebuilds whenever a created Markdown
   path is not already parsed, so that a link that failed to resolve can
   now resolve. It does not ask whether any link was waiting for that
   path, so an ignored file nothing references costs a full rebuild.
2. **One rebuild per path, not per notification.** A notification of *n*
   creations runs up to *n* rebuilds, each proportional to the workspace.
3. **Classifying a path rebuilds ignore matchers per directory level.**
   `listed_kind` builds a fresh one-level `ignore` walker for every
   directory between the root and the path, for every event, compiling
   the ignore globs each time.

## Required behavior

- **R1.** A created Markdown file the walk does not list rebuilds the
  graph only when an existing link's planned target is that path. A link
  whose target is created still resolves, exactly as today.
- **R2.** One notification rebuilds the graph at most once, after every
  change in it is applied, whatever the number of changes.
- **R3.** Classifying the paths of one notification reads each
  directory's ignore rules at most once.
- **R4.** No change to what is listed, indexed, diagnosed, or answered:
  the existing DMLS suites pass unchanged.

## Acceptance

- **A1 (R1, R2).** An L1 test sends one notification creating 200 ignored,
  unreferenced Markdown files and asserts the index generation did not
  advance; a second creates a file an existing link names and asserts the
  link resolves after one rebuild.
- **A2 (R2).** An L1 test sends one notification creating several listed
  Markdown files and asserts exactly one rebuild.
- **A3 (performance).** The reproduction above (200 ignored creations,
  3,000 documents) completes in well under a second in a debug build; the
  measured before and after are recorded in the implementation log.
- **A4 (R4).** `cargo nextest run -p dmls` and the darkmatter area's L1
  suites pass.
