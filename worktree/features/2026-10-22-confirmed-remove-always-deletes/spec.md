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
- **The only thing a confirmed removal leaves behind** is a filesystem mounted
  inside the checkout, whose data lives elsewhere (R7).
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

## Rulings

All decided by the author on 2026-10-22. This replaces the rule, in place
since `2026-10-05-graceful-file-contention`, that `wt` stops whenever it
cannot prove nothing else touched the tree. The plan removes machinery that
no longer protects anything; it does not keep that machinery and hide its
output.

### What a removal does in each case

| # | Today | After this spec |
|---|---|---|
| R1 | A process using the checkout pauses the removal (exit 3, no override) | A warning in the question, with the process context of R27. Yes deletes; non-interactive runs refuse unless `--force` |
| R2 | Submodules refused (exit 3), copying Git's `validate_no_submodules` | Supported. `wt` inspects each submodule and warns about work that exists only there (uncommitted files, commits on no remote), then deletes it with the worktree, including `.git/worktrees/<id>/modules/<name>`. A submodule with nothing at risk adds no warning |
| R3 | A locked worktree (`git worktree lock`) refused (exit 3), lock never removed | The most prominent warning, with the lock's reason and its age where Git records them. Yes unlocks and deletes; non-interactive runs refuse without `--force`. The same applies to a locked worktree whose directory is gone |
| R4 | A WSL checkout under `/mnt/<drive>` refused (exit 4): drvfs rejects `RENAME_NOREPLACE` | Supported. `wt` checks that the hidden name is free, then moves with a plain rename (or deletes in place; for the plan). The question warns that Windows programs using the checkout cannot be seen from WSL and that deletion over drvfs is slow, and suggests removing it from Windows. A file held open on the Windows side is waited on, named by file because the program cannot be named |
| R5 | A FIFO, socket, or device inside the checkout refused (exit 3) | Deleted without a warning. Unlinking never opens it; a socket in use by a process is covered by R1 |
| R6 | A Windows reparse point other than a link (cloud placeholder, deduplicated file) refused (exit 3) | Deleted. When the checkout is inside a synced folder (OneDrive), the question warns that deleting removes the cloud copies too |
| R7 | A nested mount (another filesystem mounted inside the checkout) refused (exit 3) | A warning naming the mount and its device; `wt` never deletes across it. Everything else is deleted, the mount point is left, and the final line says to unmount it, after which `wt remove` again finishes silently. This is the one case in which a confirmed removal leaves something behind, because the mounted data lives elsewhere |
| R8 | A relative `.git` link with `..` steps after the first non-`..` component refused (exit 3): it would break after the move | Handled internally: `wt` rewrites the `.git` file for the move, or skips the move (for the plan). No warning |
| R9 | A link (symlink, or on Windows a junction or other reparse point) in place of the worktree directory refused (exit 3) | When the target is this worktree's own checkout (its `.git` leads back to this worktree's record), the question says the path is a link to the worktree's real location, and yes deletes both. Otherwise the question says the target is not touched, and yes deletes only the link and Git's record |
| R10 | Git cannot read the worktree (broken `.git`, a repair that cannot be verified) refused (exit 3), no flag helps | The question warns that Git cannot read it (with Git's reason), so uncommitted files cannot be listed, and that everything in it will be deleted. Yes deletes it and Git's record; non-interactive runs need `--force` |
| R11 | A file where the worktree directory should be, or a path that cannot be inspected, refused (exit 3) | As R10: the question says what is there; yes deletes it and Git's record |
| R12 | Directory gone, index missing or unreadable, refused (exit 3) | The question warns that staged changes cannot be checked; yes removes the record |
| R13 | Files `wt` could not check (a failed `git status`, a permission error) refused (exit 3), no flag helps | The question gives the count and the reason; yes deletes them. `wt` makes read-only entries deletable |
| R14 | Files changed or staged while the question was open: refused (exit 3), "run it again" | Deleted. The final line names what changed after the question was shown, for example `also deleted 1 file changed after you answered: src/new.rs` |
| R15 | Your shell is standing in the worktree without the shell wrapper: refused (exit 4) | Unix: the question warns that your shell is inside the worktree and says to `cd` out afterward; yes deletes. Windows: refused before the question with one plain line naming the shell, because the shell's working directory blocks deleting that folder, and the shell is the one running `wt` |
| R16 | Another `wt remove` of the same worktree is running: refused (exit 4) | `wt` names it, waits for it to finish, and reports its result |
| R17 | The checkout vanished during the usage check: refused (exit 3), "run again" | Treated as already removed; `wt` finishes the record and branch steps |
| R18 | The handoff's second run refuses after a minute, or when anything changed between its two runs (exit 3 or 4) | The second run carries out what the first run approved without re-checking. The handoff token stays single-use; its expiry rises to 10 minutes |
| R19 | A pending removal from an interrupted or crashed run: hidden directory, `Retry:`, `report.json` path, consent asked again | Finished by the next `wt remove` of that name using the earlier approvals, asking nothing and never mentioning the hidden directory. Records written by today's `wt` (report versions up to 6) are finished the same way |
| R20 | Activity observed during deletion (`Stop::Activity`) | Deletion continues. A directory that keeps refilling is handled by R24 |
| R21 | `Stop::Uncertain`, `CoverageLost`, stalls, ceilings | Gone with the watcher (R23) |
| R22 | `Stop::Io` on Windows from an open file | `wt` deletes everything else, names the program, and waits in the same run until it closes the file or the user presses Ctrl-C |
| — | The free-space refusal before the move (added by `2026-10-22-remove-large-checkout`) | Removed with the journal (R25) |
| — | Ctrl-C | Unchanged: ends the run; the next run finishes silently (R19) |
| — | Consent for uncommitted files | Part of the single confirmation question |
| — | A name that matches more than one worktree | Unchanged: an input error before any question |
| — | Branch safety: the branch tiers, the Not-safe branch question, `--force-branch`, `--force-remote`, a branch kept because its tip moved after it was assessed | Unchanged and out of scope: it protects commits, not files |

### Machinery

- **R23. The watcher is removed on every OS**, with its FSEvents, inotify,
  and `ReadDirectoryChanges` adapters and their test groups. Nothing it
  reports changes the outcome any more.
- **R24. A program that keeps writing during deletion** makes a final
  directory removal fail as not empty. `wt` lists what is there again and
  deletes it. When the directory keeps refilling, `wt` runs the usage check,
  names the program as in R1, and waits as in R22.
- **R25. The quarantine move stays; the per-entry checks and the journal
  go.** The move to a hidden sibling frees the original path at once and
  gives an interrupted removal one place to finish. The per-entry checks
  (identity, size, modification time, permission bits, BLAKE3) existed only
  to stop on change and are removed. The journal is replaced by a small
  record: operation id, original path, hidden path, and the approved branch
  steps. The hidden directory itself is the list of what is left. A normal
  built checkout's bookkeeping drops from about 0.5 GB to a few kilobytes,
  and the free-space refusal goes away.
- **R26. `report.json`.** A finished removal keeps nothing. An interrupted
  one keeps only R25's record.
- **R27. Process context.** Sniff's usage report adds, for each process, a
  recognizable name (the app name on macOS, the program's description on
  Windows), its start time, and its parent chain up to something the user
  knows (a terminal, an editor, an agent), alongside what it is doing
  (writing, watching, running from, standing in) and where. Example:
  `cargo build (pid 4123, started 14:02, run from VS Code's terminal), writing target/debug/`.
- **R28. One `--force`** is the scripted equivalent of yes for everything
  about the worktree: files, processes, a lock, files `wt` could not check.
  `--force-worktree` is removed. `--force-branch` and `--force-remote` are
  unchanged.

## Verification

- L1: every ruling R1–R28, using injected usage sources and test seams, and
  replacing the tests that pin the old stops. The watcher's tests are removed
  with it (R23).
- L1: non-interactive refusal without `--force`, and deletion with it.
- L1: a worktree with a submodule holding uncommitted files and an unpushed
  commit warns about both and is deleted, its `modules/<name>` included; a
  clean submodule adds no warning.
- L1: a locked worktree shows the lock's reason, and yes unlocks and deletes
  it.
- L1: a nested mount is warned about, nothing on it is deleted, and a second
  `wt remove` after it is unmounted finishes silently (R7; on Linux, a tmpfs
  or bind mount where the test can create one, otherwise a skip with a
  reason).
- L1: a link to this worktree's own checkout deletes both; a link to anything
  else deletes only the link and Git's record, and the target is untouched
  (R9).
- L1: a file changed after the question is deleted and named in the final
  line (R14).
- L1: an interrupted removal, including one recorded by today's report
  version 6, is finished by the next `wt remove` without any question (R19).
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
