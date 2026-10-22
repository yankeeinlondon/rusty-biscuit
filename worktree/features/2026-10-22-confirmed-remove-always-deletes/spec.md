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
status: draft-spec
reviewed: false
review_iterations: 0
clarified: false
implemented: false
related:
    - 2026-10-22-remove-large-checkout
    - 2026-10-18-remove-observation-under-load
    - 2026-10-19-macos-drain-interval-comparison
    - 2026-10-05-graceful-file-contention
---

# A confirmed `wt remove` always deletes

## Outcome

A user who types `wt remove <name>` and confirms gets the worktree deleted,
every time, in that one run.

- **Warn, don't block.** Everything `wt` can see that makes the removal risky
  is shown in the confirmation question: processes using the checkout,
  uncommitted files, a `git worktree lock`, contents `wt` treats specially.
  Each warning gives enough context for a user who does not know `wt`'s
  internals to judge it.
- **"Yes" means delete.** After confirmation no internal condition stops the
  removal. The user never sees a hidden `.wt-remove-…` directory, a `Retry:`
  line, a report path, or a watcher, journal, or coverage diagnostic.
- **The only things that end a confirmed removal early:**
    - the user's own Ctrl-C;
    - on Windows, a file another program holds open without delete sharing,
      which cannot be deleted until that program closes it. `wt` deletes
      everything else, names the program in plain terms, and waits in the same
      run until it closes the file (or the user presses Ctrl-C), then finishes.
- **Non-interactive runs** (stdin or stderr not a terminal, or `CI` set)
  cannot show a question. When there is anything to warn about, they refuse
  before anything changes, with exit 3 and a plain message listing what was
  seen. `--force` is the scripted equivalent of answering yes: with it, `wt`
  deletes as an interactive "yes" would.

Example of the interactive question:

```text
$ wt remove feat/modern-config
Remove worktree feat/modern-config (/Volumes/coding/wt/rusty-biscuit/feat-modern-config)?

  ⚠ 2 programs are using files in this worktree:
      cargo build   (pid 4123, started 14:02) — writing target/debug/
      rust-analyzer (pid 3990) — watching src/
    Deleting now may make them fail.
  ⚠ 3 uncommitted files will be lost: src/config.rs, src/lib.rs, notes.md
  • Branch feat/modern-config is merged into main and will be deleted.

Delete it? [y/N] y
Removed feat/modern-config (441,203 entries) and deleted branch feat/modern-config.
```

The exact wording is for the plan. The rule is that each warning names what
was seen and what deleting will do to it.

## Why

The author ruled this on 2026-10-22, while reviewing
`2026-10-22-remove-large-checkout`. Built checkouts of this monorepo
(hundreds of thousands of entries under `target/`) are the normal case, not
the large one. On macOS every such removal stopped partway, often several
times, for reasons the user never agreed to and could not act on:

- fseventsd dropping change notifications under deletion load
  (`CoverageLost::FsEventsDropped`), three of three runs at 500,000 entries;
- before that, the journal's fixed size limit;
- a stalled watcher (`DeliveryStalled`, `DrainCeiling`);
- changes `wt` could not tell apart from its own deletions (`Stop::Uncertain`).

Each stop left a half-deleted checkout under a hidden name and a long report.
The user's only option was to run the command again with no new information,
which made the stop a manual "continue" button rather than a safety check.
Pacing deletion to avoid the drops was measured and gave no benefit it could
detect (`2026-10-19-macos-drain-interval-comparison`).

## What changes

This replaces the rule, in place since `2026-10-05-graceful-file-contention`,
that `wt` stops whenever it cannot prove nothing else touched the tree.

| Today | After this spec |
|---|---|
| A process using the checkout pauses the removal (exit 3, no override) | Warning in the question; yes deletes; non-interactive refuses unless `--force` |
| Submodules refused (exit 3), copying Git's `validate_no_submodules` | Supported. `wt` inspects each submodule and warns about work that exists only there (uncommitted files, commits on no remote), then deletes it with the worktree, including `.git/worktrees/<id>/modules/<name>`. A submodule with nothing at risk adds no warning |
| A locked worktree (`git worktree lock`) refused (exit 3), lock never removed | The most prominent warning, with the lock's reason and its age where Git records them. Yes unlocks and deletes; non-interactive runs refuse without `--force` |
| A WSL checkout under `/mnt/<drive>` refused (exit 4): drvfs rejects `RENAME_NOREPLACE` | Supported. `wt` checks that the hidden name is free, then moves with a plain rename (or deletes in place; for the plan). The question warns that Windows programs using the checkout cannot be seen from WSL and that deletion over drvfs is slow, and suggests removing it from Windows. A file held open on the Windows side is waited on, named by file because the program cannot be named |
| Other refusals before the question: a FIFO, device, or nested mount, a relative `.git` link | Each becomes a warning, or is handled, unless deleting it safely is technically impossible (to settle in planning, case by case) |
| Activity observed during deletion (`Stop::Activity`) | Deletion continues |
| `Stop::Uncertain`, `CoverageLost`, stalls, ceilings | Never reach the user |
| `Stop::Io` (permission denied, read-only entries) | `wt` makes the entry deletable and deletes it |
| `Stop::Io` on Windows from an open file | Wait and name the program (Outcome) |
| The free-space refusal before the move (added by `2026-10-22-remove-large-checkout`) | Removed; recovery bookkeeping must not be a reason to refuse |
| A pending removal from an interrupted or crashed run: hidden directory, `Retry:`, `report.json` path | Finished by the next `wt remove` of that worktree, or by any `wt` command, without asking or mentioning the hidden directory |
| Ctrl-C | Unchanged: ends the run, and the next run finishes the job silently |
| Consent for uncommitted files | Becomes part of the single confirmation question |

Out of scope: branch safety. The branch tiers, the Not-safe branch question,
`--force-branch`, `--force-remote`, and keeping a branch whose tip moved
after it was assessed are about lost commits, not files, and stay as they are.

## Design questions for the plan

The rule makes much of the current machinery unnecessary. The plan should
remove what no longer protects anything, rather than keep it and hide its
output.

1. **The watcher (`remove::observation`).** Once a confirmed removal never
   stops for observed activity, what is the watcher still for? Candidates:
   nothing (remove it), or diagnostics only (identifying a process that wrote
   during deletion, for the final message). Removing it also removes the
   FSEvents, inotify, and `ReadDirectoryChanges` adapters and their L1
   timing constraints.
2. **The quarantine move and per-entry checks.** The move to a hidden sibling
   frees the original path immediately and keeps crash recovery simple. The
   per-entry checks (identity, size, mtime, BLAKE3) exist to stop on change.
   Decide which survive.
3. **The recovery journal.** Its purpose becomes crash recovery only: knowing
   what to finish next time. A file list that already exists on disk (the
   hidden directory itself) may be enough. Size it so that it is never a
   reason to refuse; for a normal built checkout today it needs about 0.5 GB.
4. **`report.json`.** It was 107 MB for one removal. Decide what a completed
   removal keeps, if anything.
5. **Process context.** Sniff's usage report gives a pid, command, and
   mechanism. The warning should make sense to a user: a recognizable program
   name, what it is doing (writing, watching, running from, standing in), and
   where in the tree. Decide what Sniff must add, if anything.
6. **`--force` and the existing flags.** `--force-worktree` already approves
   discarding uncommitted files non-interactively. Decide whether `--force`
   replaces it (one flag for "yes, delete this worktree"), or sits beside it
   and covers only the new warnings. Recommendation: one `--force` for the
   worktree's files and processes, with `--force-branch` and `--force-remote`
   unchanged.
7. **The shell-wrapper handoff.** The second run of a handoff re-checks and
   refuses on change today. Decide how it fits "yes means delete".
8. **Existing pending removals.** Records written by today's `wt` (report
   versions up to 6) must be finished by the new behavior, not stranded.

## Verification

- L1: every row of the table above, using the existing injected observers,
  usage sources, and test seams, and replacing the tests that pin the old
  stops.
- L1: non-interactive refusal without `--force`, and deletion with it.
- L1: a worktree with a submodule holding uncommitted files and an unpushed
  commit warns about both and is deleted, its `modules/<name>` included; a
  clean submodule adds no warning.
- L1: a locked worktree shows the lock's reason, and yes unlocks and deletes
  it.
- WSL2: a checkout under `/mnt/<drive>` warns as specified and is deleted;
  a collision at the hidden name is still detected and nothing is replaced.
- L2: the interactive question with a process warning, answered yes, in a
  tmux pane (no window takes focus).
- Acceptance: one `wt remove` of a disposable 500,000-entry built checkout on
  macOS finishes in one run, with no retry and nothing but the confirmation
  and the final line on screen. Also on Linux and native Windows, under the
  repository's existing evidence rules.
- Windows: a file held open without delete sharing is waited on and named,
  and the removal finishes once it is released.

## Documentation

- `worktree/docs/cli/remove.md` is largely rewritten: the confirmation
  question and its warnings, `--force`, the Windows wait, and silent recovery
  of an interrupted removal. The sections on watching, coverage loss,
  fseventsd, stop reports, and retry shrink or go away, along with the
  **Planned** markers this spec's ruling added.
- `worktree/README.md`: the `wt remove` bullet.
- `.claude/skills/worktree/`: `remove.md`, `lifecycle.md`, `observation.md`.
