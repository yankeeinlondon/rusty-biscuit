---
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
status: implemented
reviewed: false
review_iterations: 0
clarified: true
implemented: true
implemented_by: "claude/opus"
related:
    - 2026-10-23-remove-stopped-by-list
    - 2026-10-18-remove-observation-under-load
    - 2026-10-19-macos-drain-interval-comparison
---

# Once you agree, `wt remove` deletes, and warns instead of stopping

## Outcome

After the user has agreed to a removal, `wt remove` finishes it. What it
sees while doing so (a change or new file in the checkout, a Git command
touching the worktree's record, notifications the system dropped) is
reported as a warning when the removal ends, never a reason to stop.

The questions before the removal are unchanged: they are where the user
learns what will be lost and decides. Everything after the answer carries
out that decision.

**Author decision (2026-10-23):** "warn the user about anything they should
know but if they agree to delete, we delete." Asked what to do with a file
inside the moved checkout that changes or appears during deletion, the
author chose **delete it, then warn**.

## What happened

The same removal of `feat-iterm-graphics` (835,000 entries) ran four times
on 2026-10-23. Two of the stops are fixed elsewhere (`wt list` running
`git status`, and the journal's fixed size limit). The fourth deleted
297,053 entries and stopped because macOS's FSEvents dropped notifications
(`UserDropped`, `MustScanSubDirs`) during the deletion burst. That stop had
been accepted on 2026-10-20 as "a resumable stop, not lost work", and slower
deletion was measured not to prevent it. So every large removal on macOS can
stop several times, and each retry asked the user nothing new; they only
retyped the command.

## Requirements

### What no longer stops a removal

Everything below happens after the user's answer. Each becomes a warning:

| Today | Now |
| --- | --- |
| Watching loses coverage (dropped or overflowed notifications, a stalled or late delivery, a lost watch), before or after the move | One warning naming the reason and the stage; deletion continues without watching |
| A change at the checkout between the answer and the move (an entry modified, created, removed; Git's record changed) | A warning per change; the removal goes ahead |
| The worktree's HEAD, files, or staged changes differ from what the questions showed | A warning; the removal goes ahead. A branch deletion still compares the tip it was approved at and fails safely if the branch moved |
| An entry inside the moved checkout changed, was replaced, or vanished before it was deleted | It is deleted as it is now (a vanished one needs nothing); a warning names it |
| A file or folder appeared inside the moved checkout | It is deleted with its contents; a warning names it |
| A Git command changed the worktree's record (its index or HEAD) | A warning; the worktree's record is removed |
| Something appeared at the original path | It is never touched (it is outside what was agreed); a warning names it, and the removal of the moved checkout and its record still completes |

### Deleting what is there now

- Deletion still walks the approved inventory, children before their
  folders, journaling every approved entry as today.
- An approved entry that no longer matches its record is deleted as it is
  now: a link is removed as a link (never followed), a folder with its
  contents.
- A folder that is not empty when its turn comes (new content appeared) has
  its remaining contents deleted first.
- Deleting unapproved content never follows a link and never crosses into
  another filesystem. A mount point inside the checkout stops the removal
  as an I/O failure, with everything else deleted.
- Unapproved content is not journaled (the journal lists approved entries
  only). The report names each top-level path deleted this way, as
  activity, and counts them.

### What still stops a removal

- An interrupt (Ctrl-C and the other signals): the user's own decision.
- The moved checkout is no longer the directory that was moved (its
  identity differs): `wt` never deletes a directory it did not move.
- An entry that can't be deleted (permission denied, a mount point, a
  Windows handle) and journal or recovery-record failures: as today, with
  everything else left for a retry.
- Before anything changes: a pending removal of the same worktree, and the
  refusals that mean `wt` cannot do the move at all.
- **Not changed by this fix, each a separate decision:** the usage check (a
  process using the checkout pauses the removal with exit 3), a watch that
  cannot start (exit 4), a FIFO, socket, or device in the checkout (exit 3),
  and the shell-wrapper handoff's second run, which refuses when approved
  work changed between its two runs. All four happen after the questions
  and before anything moves.

### What the user sees

A removal that finished with warnings exits 0, prints its usual completion
lines, then:

```text
Warning: 3 changes happened after you agreed; wt deleted them anyway.
  /…/.wt-remove-feat-x-…/target/debug/build/out.o was modified
  /…/.wt-remove-feat-x-…/notes.txt was created
  watching stopped during deletion: macOS could not report every change (UserDropped, MustScanSubDirs; flags 0x3)
Full report: /…/.git/wt-removal/<id>/report.json
```

At most three changes are listed, then a count. The report's `activity`
lists them all, with `outcome: "removed"`.

## Success criteria

- Lifecycle tests, with the injected observer: a coverage loss before the
  move, after the move, and during deletion; a modified approved file; a new
  file in a folder; a new folder; an approved folder replaced by a link (the
  link's target survives); a Git index change; something at the original
  path (kept). Each removal completes, exits 0, and reports its warnings.
- An interrupt, and a quarantine whose identity changed, still stop.
- The existing L1 and L2 tests that asserted a stop for one of the cases
  above now assert completion with a warning.
- `docs/cli/remove.md` and the worktree skill describe the new rule; the
  stop examples that no longer happen are removed.
- `just test` and `just lint` pass in `worktree/`.
