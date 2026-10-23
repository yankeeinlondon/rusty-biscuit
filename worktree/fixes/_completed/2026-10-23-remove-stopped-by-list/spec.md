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
clarified: false
implemented: true
implemented_by: "claude/opus"
related:
    - 2026-10-18-remove-observation-under-load
    - 2026-10-01-remove-busy-folder
---

# `wt list` stops a running `wt remove`, and the stop reports don't say what to do

## Outcome

`wt list` never writes into a checkout that a `wt remove` has started and
not finished, so running it in another terminal can't stop a removal. The
listing shows such a checkout as being removed and names the command that
finishes it.

When a removal is refused or stops, its report says what happened in terms
the user can act on, what that means for their files, and the steps that
recover, in order. A report never asks the user to "fix what is named
above" or "inspect both locations" when the retry already re-checks
everything.

## What happened

On 2026-10-23 `wt remove fix/dmls-rebuild-and-schema-reuse` (worktree
`feat-iterm-graphics`, about 835,000 entries, macOS) failed twice.

| Run | Result | Cause |
| --- | --- | --- |
| 1 | Refused before the move. Nothing changed. | `just install` of `darkmatter` in another terminal was writing `target/release/deps/…/full.rmeta` in that worktree |
| 2 | Moved aside, deleted 18,000 entries, stopped | `.git/worktrees/feat-iterm-graphics/index.lock` was modified 4.9 s after the move |

The user had made sure nothing was building before run 2, so the stop was
unexpected. Shell history and file times explain it:

| Local time | Terminal | Event |
| --- | --- | --- |
| 10:49:27 | A | `wt remove fix/dmls-rebuild-and-schema-reuse` |
| 10:49:47.1 | — | checkout moved to `.wt-remove-feat-iterm-graphics-…`; Git's record now points there |
| 10:49:51 | B | `wt` (that is, `wt list`) |
| 10:49:52.1 | — | `index.lock` event |
| 10:49:52.97 | — | the worktree's `index` mtime |

`wt list` runs `git -c core.untrackedCache=true status --porcelain` in
every registered worktree. After the move, Git registers the removal's
quarantine, so `wt list` ran `git status` there, Git refreshed and rewrote
that worktree's index, and the removal's watcher saw a write it had not
made. **`wt remove` was stopped by `wt list`.**

Nothing of value was lost. The checkout was clean and its branch was merged
(PR #137). Of the 18,000 deleted entries, 16,993 were under `target/` and
the rest were committed files.

### What the reports got wrong

Run 1:

```text
Nothing was removed. wt stopped before moving worktree feat-iterm-graphics: /…/target/release/deps/rmeta2E6L2M/full.rmeta changed while the removal was being prepared: it appeared after the inventory was taken.
  Nothing was moved. Fix what is named above, then run wt remove fix/dmls-rebuild-and-schema-reuse again.
```

- "Fix what is named above" names a build artifact. Nothing needs fixing:
  something is still writing into the worktree, and the user should let it
  finish.
- "changed while …: it appeared after the inventory was taken" chains two
  colons and uses the internal word "inventory".

Run 2 (abridged):

```text
Removal stopped: Git's record of the worktree changed.
…
Observed: /…/.git/worktrees/feat-iterm-graphics/index.lock was modified (reported at 17:49:52 UTC)
…
A write that landed just before its entry was deleted was deleted with it; the saved report lists every deleted entry.
Before the move: Discovery could not run; FSEvents subscriptions and polling watchers cannot be listed on this system; …; the check ran out of time.
After the stop, at /…/.wt-remove-…: Discovery could not run; FSEvents subscriptions and polling watchers cannot be listed on this system; …; the check ran out of time.

Stop any tasks using either path. Inspect both locations and save new work.
Retry: wt remove fix/dmls-rebuild-and-schema-reuse
  Run it from the base checkout at /Volumes/coding/personal/rusty-biscuit.
```

- "Git's record of the worktree changed" hides the one useful fact: a
  change to `index.lock` means **a Git command ran in this worktree**.
- "A write that landed just before its entry was deleted was deleted with
  it" reads as a data-loss warning. What the user needs first is that only
  what they approved was deleted.
- The two process-check lines are identical and repeat each other.
- "Stop any tasks using either path. Inspect both locations and save new
  work." is the same text for every stop, whatever the cause. The retry
  already lists what is left and asks again about any changed or new file,
  so "inspect both locations" is work the tool does.
- "Run it from the base checkout" reads as a requirement. A retry works
  from anywhere outside the moved directory; the user was already in
  another worktree.

## Requirements

### 1. `wt list` leaves a checkout under removal alone

- A checkout is **under removal** when a pending recovery record (in
  `<git-common-dir>/wt-removal/`) identifies it: the record's quarantine,
  or the unmoved checkout at the record's original path with the recorded
  identity. This is the same match `wt remove` uses to resolve a retry, so
  the two never disagree.
- `wt list` runs no `git status` in a checkout under removal: not in the
  listing, and not in the refresh after `--ff` moves a checkout.
- `wt list --ff` does not fast-forward through a holder under removal; it
  refuses with a note naming the holder and the command that finishes the
  removal, and changes no ref.
- The row shows a distinct marker, and a closing note says the checkout is
  being removed and shows `wt remove <name>`, with the name the removal was
  started with, as the way to finish it. The legend explains the marker
  only when a row shows it.
- Reading the store is best effort: a missing store costs nothing, and a
  store or record that can't be read marks nothing under removal. (While an
  unreadable record exists, `wt remove` refuses to start any removal, so no
  removal is running for it to protect.)
- No new Git process: the common directory is read in-process.

### 2. Refusal before the move: something is still writing

When the inventory sees an entry appear, change, vanish, or be replaced
while the removal is prepared, the refusal says:

```text
Nothing was removed. wt stopped before moving worktree feat-iterm-graphics: something is still writing into it (/…/full.rmeta was created while wt was preparing the removal).
  Let it finish, or stop it, then run wt remove fix/dmls-rebuild-and-schema-reuse again.
```

Other inventory refusals (an entry that can't be inspected or can't be
removed safely) keep "Nothing was moved. Fix what is named above, …",
because there the named entry is what needs fixing.

### 3. A stop says what happened, what it means, and what to do

The report keeps its facts (both locations, counts, observed changes, Git's
registration, branches, errors, the saved report path) and changes:

- **Headline for a Git command.** When the first observed change is in
  Git's record of the worktree and is the index (`index`, `index.lock`):
  `Removal stopped: a Git command ran in this worktree while it was being
  removed.` When it is `HEAD` or `HEAD.lock`: `Removal stopped: a Git
  command changed this worktree's HEAD while it was being removed.` Any
  other change there keeps `Git's record of the worktree changed`.
- **What it means for your files.** After the deleted count, when anything
  was deleted: `Only entries you approved before the move were deleted.`
  The concurrent-write caveat follows it, reworded: `A file written in the
  instant before wt deleted its path would have been deleted too; the saved
  report lists every deleted path.`
- **Process checks.** Lines whose note is identical are merged into one,
  their contexts joined (`Before the move and after the stop, at <path>:
  …`).
- **What to do**, numbered, chosen by what stopped the removal:

  | Stop | Step before the retry |
  | --- | --- |
  | a Git command (index or HEAD) | `Let that Git command finish. A shell prompt that shows Git status, an editor's Git integration, or a git command in another terminal can start one; don't run Git in the moved directory until the removal finishes.` |
  | something new at the original path | `Move or save what is now at <original>; wt never deletes it, and the retry refuses until it is gone.` |
  | any other observed change | `Stop whatever is writing to <path of the first observed change>.` |
  | anything else (interrupt, a failed step, lost watching) | none |

  followed by the retry, always:

  ```text
  Retry: wt remove <name>
    Run it from anywhere outside the moved directory, such as the base checkout at <base>. It lists what is left and asks again about anything new or changed before deleting more.
  ```

  A report for a removal whose checkout is gone and whose branch step
  failed keeps its current retry line alone.

### Not in this fix

- **The process check runs out of time on a large checkout.** Sniff walks
  the whole tree (835,000 entries) inside the check's 2 s budget before it
  lists processes, so the processes are never listed. It did not matter
  here: `wt list` started 4 s after the move, so no check before the move
  could have seen it. Changing the order of Sniff's query is a Sniff
  change of its own.
- **Other programs that run `git status`** (shell prompts, editors) can
  still stop a removal; `wt` can only tell the user that this is what
  happened, which requirement 3 does.

## Success criteria

- An L1 test lists a repository whose linked worktree has a pending
  removal record at its quarantine and proves `wt list` ran no `git status`
  there and marks it under removal.
- `--ff` refuses a holder under removal without changing a ref.
- The stop and refusal report tests cover each new headline, the "what to
  do" step for each row of the table, the merged process-check line, and
  the refusal for an entry that appeared during preparation; snapshots are
  updated and reviewed.
- `docs/cli/remove.md` and `docs/cli/list.md` show the new output.
- `just test` and `just lint` pass in `worktree/`.
