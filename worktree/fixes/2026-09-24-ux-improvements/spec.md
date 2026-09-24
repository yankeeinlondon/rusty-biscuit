---
status: draft
---

# Worktree UX improvements

Throughout this spec, **default branch** means the repository's mainline branch (detected: main/master via the remote's HEAD, falling back to main or master). The short form "default branch" is used thereafter; the library detects it and hands it to the CLI.

This spec is about fixing the user experience rough edges in the `wt` CLI (and library) as well as addressing a few known bugs:

1. Display Issue When Deleting

    When you run `wt remove <worktree> -b` (and possibly without the `-b` too) you'll get a directory tree of files and awkwardly hang off the last line of the directory structure is a confirmation dialog saying that "All file changes will be lost".

    The dialog would be fine if it were BELOW the file directory listing (ideally with a blank line in between)

2. Shell Completions Incomplete

    When you try to autocomplete the name of the worktree you're wanting to switch to or delete the autocomplete options only work when you use the branch name, not the worktree name! Both should work.

    > Note: as an edge case, two worktrees _could_ be on the same branch ... that would be unlikely but it's possible. The same is NOT possible with worktree names. That means the worktree name guarantees uniqueness but not the branch name. Still it is convenient that autocomplete provides both. However, if more than one worktree is on the same branch that a caller is trying to operate on ... this should result in an error that describes and lists the worktrees on that branch.

    Completions (the library's `worktree_names`) will offer, for each non-main worktree, BOTH the branch name and the worktree's dasherized directory basename (deduped when the two are equal); plus `base`. The detected default branch name (e.g. `main`) is ALSO offered as a candidate and resolves to the base checkout: typing or completing `base` or the default branch name brings the user back to the base checkout. Detached worktrees contribute their directory basename only.

    The "dasherized" form of a name is its lowercased form with every character that is not alphanumeric or a dash replaced by a dash; it is the form used for the worktree's directory name.

    Resolution contract (the library's `find_worktree`), applied in this order:

    - `base` and the detected default branch name resolve to the main checkout.
    - A worktree-name match (dasherized directory basename) is unique by construction and wins immediately.
    - A branch-name match resolves to the single worktree on that branch. If MORE THAN ONE worktree is on that branch, the command fails with a new error that describes the ambiguity: it names the branch, lists each worktree's name AND path on that branch, and hints to use the unique worktree name instead.
    - This applies uniformly to `wt go` and `wt remove` (no CWD-based auto-disambiguation).
    - No match remains a not-found error.

3. Default Behavior for `remove` is Poor

    When you are trying to delete a worktree, we currently do NOT delete the branch and require the caller to remember to use the `-b` to make their intent clear that they DO want to remove the branch as well.

    The state facts the command must establish before it does anything:

    - **Dirty:** does the worktree's working tree have any untracked or modified files?
    - **Unique commits:** does the worktree's branch have any commits that are not present on the default branch?
        - "Unique commits" means commits reachable from the worktree's branch but not from the default branch. Commits that are also on ANOTHER local branch are NOT counted as unique, but the report adds a short note when applicable (e.g. "also on `other-branch`") so the user sees that such commits survive the removal.
    - **Ahead/behind:** is the worktree's branch ahead of, behind, or at the same commit as the default branch?
    - **Remote:** does the "remote" have a version of this branch and is it ahead, behind, or at the same commit as the local branch?

    A removal is an important operation, so the first thing the command does -- both with and without CLI flags -- is report the current status before offering any choice or performing any removal. (When the branch is clean and at the same commit as the default branch, the report simply states that the branch is in sync.) The flow then branches on state:

    | Worktree state | Flow |
    |---|---|
    | (a) Clean, at the same commit as the default branch | Look at the remote to see if it carries the same branch. If the branch is on the remote, communicate the ahead/behind relationship between local and remote, then run the branch decision procedure below. If the branch is NOT on the remote, confirm that the user wants to remove the branch as well (defaulting to Yes); if the user chooses NOT to, remove the worktree and keep the branch. |
    | (b) Dirty (untracked or modified files) | Warn the user that there are untracked/dirty files that will be lost. Show the directory list of files so the user is clear (requirement: show staged files as yellow, untracked as red; see Open questions on which coloring axis wins). If there are a large number of files (>10, requirement) then instead provide the NUMBER of files (the number in bold and red) so the terminal is not overwhelmed. Confirm the deletion of the worktree with the default set to "No". If the user chooses "Yes", run the branch decision procedure below. |
    | (c) Clean, but the branch is AHEAD of the default branch (has unique commits) | Follow the SAME flow as case (a): report the unique-commit count and the commit subjects (using the same display convention as the dirty-file list: if there are more than 10, show the bold-red COUNT instead of listing), plus the "also on" note for any unique commit that is also on another local branch, plus the remote state -- then offer the SAME choices as case (a) via the branch decision procedure below. |

    **Branch decision procedure** (shared by cases (a) and (c), and by the dirty flow once the worktree deletion is confirmed):

    1. If the branch is ahead, report the unique commits (count and subjects, with the >10 convention and the "also on" note).
    2. Report the remote state.
    3. Offer the choices:
        - Branch on the remote: the four choices -- remove branch locally / remove branch locally and on remote / remove worktree but keep branch / exit.
        - Branch NOT on the remote: confirm that the user wants to remove the branch as well (defaulting to Yes); choosing No leads to "remove worktree but keep branch".
    4. Loss confirmation. When the user picks a branch-removal choice (either a branch-removal entry from the four choices, or Yes to the "remove the branch as well" confirmation) for a branch that is ahead of the default branch, ONE explicit secondary confirmation fires before anything is deleted: "this will delete N commit(s) not on the default branch -- continue?", defaulting to No. N is the number of commits the chosen action actually orphans: the commits reachable from the branch that will no longer be reachable from any ref after the chosen action, excluding commits still on the remote branch when the remote branch survives. Commits that remain reachable from another local branch are never orphaned and never counted.

        | Chosen action | N counts |
        |---|---|
        | Remove branch locally | The unique commits NOT on the remote branch, which survives and keeps them; all unique commits when there is no remote branch. |
        | Remove branch locally and on remote | All unique commits. |
        | "Remove the branch as well" = Yes (no remote branch) | All unique commits. |
        | Remove worktree but keep branch | -- (no branch deletion, no confirmation) |
        | Exit | -- (nothing is deleted) |

        If N is 0 -- the remote branch survives and keeps every unique commit -- nothing is lost and the confirmation is skipped. A Yes proceeds to a HARD delete (`git branch -D`). A No keeps the branch and the command proceeds with the "remove worktree but keep branch" outcome.
    5. Branch deletion mechanics. When the branch has no unique commits, a soft delete (`git branch -d`) is used. Plain `git branch -d` refuses an ahead branch, so branch removal on an ahead branch always escalates to a hard delete (`git branch -D`) after the explicit confirmation.

    Known limitation: squash-merge / "effectively merged" detection is explicitly OUT OF SCOPE for this spec. A squash-merged branch still counts as "ahead" and goes through the loss-confirmation path.

    **Flag surface.** The flag set of `wt remove` is an EXPLICIT TRIO, each meaning exactly one thing:

    - `--force` (short form `-f`, a plain boolean, NOT a count): never prompt; remove the local worktree regardless of state (dirty files, unique commits). It does NOT touch the local branch or the remote.
    - `--remove-branch`: also hard-delete the local branch (`git branch -D`) -- even with unique commits, since no prompt is allowed when combined with `--force`; interactively without `--force` this flag preselects the matching menu choice in the branch decision procedure, but the loss-confirmation still applies.
    - `--remove-remote`: also delete the remote branch (`git push --delete`) if it exists; if it does not exist, communicate that there was no remote branch with that name.

    All three together = full cleanup (worktree + local branch + remote branch), no prompts.

    **Retirements.**

    - The existing `-b` flag is RETIRED outright (no deprecation period -- the tool has no installed users); passing `-b` is a standard clap "unexpected argument" error.
    - The existing `-f`/`-ff` COUNT matrix and its 10-file bypass limit (FORCE_BYPASS_FILE_LIMIT) are DELETED; `-f` is simply the short form of `--force` from now on.
    - The docs/help text (the AFTER_HELP examples) must be updated accordingly.

    **Flag x interactivity matrix.** "Safe" means no unique commits and no dirty files.

    | | No flags | `--force` | `--remove-branch` | `--remove-remote` | Combinations |
    |---|---|---|---|---|---|
    | Interactive | Report status; decision tree above with its prompts; outcome per the chosen option | No prompt; remove the local worktree only; local branch and remote untouched | Report status; dirty-file confirmation when dirty (no `--force`); branch decision procedure with "remove branch locally" preselected (the user may change the selection); loss confirmation when the branch is ahead; removes the worktree and hard-deletes the local branch | Report status; dirty-file confirmation when dirty (no `--force`); removes the worktree; deletes the remote branch if it exists, otherwise reports that no remote branch with that name exists; local branch kept | `--force` with scope flags: no prompts; removes the worktree plus whatever the scope flags name. `--remove-branch` + `--remove-remote` without `--force`: report status, confirmations as each flag requires, then full cleanup |
    | Non-interactive | Removes the worktree only when safe; otherwise an error describing the state and the reason | No prompt; removes the worktree; local branch and remote untouched | Removes the worktree and hard-deletes the branch when safe; otherwise an error (the required confirmations cannot be shown) | Removes the worktree and deletes the remote branch (or reports that no remote branch with that name exists) when safe; otherwise an error | `--force` with scope flags: no prompts; removes the worktree plus whatever the scope flags name. Scope flags without `--force`: the same "safe" precondition as each flag alone; otherwise an error |

    The non-interactive cells for the scope flags are the most direct reading of the rules above; any residual doubt is recorded in Open questions.

    The non-interactive rule from this spec stands: without the force/scope flags, on a non-interactive terminal, removal happens only when there are no unique commits and no dirty files; in all other cases the command reports an error that describes what state the worktree is in and why it did not remove it. (HOW "non-interactive" is detected and the exit-code contract remain OPEN -- see Open questions.)

4. BUG: Branch not Removed Though Requested

    There are times when -- using the current functionality -- a user calls `wt remove <worktree> -b` (aka, explicitly says they want the branch removed too) and it fails to remove the branch. This is inconsistent because sometimes it DOES remove the branch and it is not clear what determines whether the `-b` flag is respected or not.

    Resolution within the new flow: the `-b` flag is retired (item 3). Its inconsistent behavior came from the soft-delete attempt silently refusing an ahead branch and reporting it as "preserved". In the item 3 flow, a requested branch removal on an ahead branch always goes through the explicit loss confirmation and then a hard delete, so a requested branch removal either happens after confirmation or is explicitly declined; there is no silent refusal path.

5. More Information

    When we run `wt list` (or just `wt` as an alias) we get useful and highly visual information. This is a good starting point. However, there are a few things we need to add:

    - in the table we report with today, we should add a column "From" which would reside between the Branch and Merge columns
    - if the terminal width is less than 120, however, the "From" column will not be rendered to preserve horizontal space (requirement)
    - when it is rendered, its function is to convey which branch the worktree was _forked_ from

    Data source: git records no fork origin. `wt create` will RECORD the fork base (the branch the new branch was forked from) in the library's existing user-cache store -- the versioned JSON store already loaded and saved on every `wt list` -- keyed by branch: { base branch, base sha, created at }. `wt list` reads it for the "From" column.

    - For the reused-existing-branch path (the branch already existed and was checked out as-is), nothing is recorded -- there was no fork -- the column shows the fallback.
    - Worktrees created by plain git (or created before this feature) have no record: the column shows a dim em-dash (—).
    - The main worktree: column blank (consistent with how other columns treat the main row).
    - Stale records (branches deleted) are pruned during the save that `wt list` already performs.

    Note: the fork base becomes an explicit input once item 6's `--branch` switch lands (default: the current branch), so recording it is recording an input.

6. Forking Freedom

    Today when the `wt create` command is used, the new worktree will be forked from the branch the user is currently on. This is a good default but the user should be able to choose a different branch as a base if they want to:

    - we should add the `--branch <branch>` CLI switch to allow a user to explicitly state which branch they want to fork off of (default: the current branch)

## Packages

- `worktree` (library): the name-resolution contract (`find_worktree`, `worktree_names`, including the ambiguity error), default-branch detection, state classification (dirtiness, ahead/behind, unique commits), the unique-commit facts, fork-origin persistence in the user-cache store, remote-branch deletion, and the hard-delete path (`git branch -D`).
- `worktree-cli`: the flag surface, rendering (the list table including the 120-column cutoff, the dirty-file tree, the >10-file count), and the prompts (confirmations and the choice menu).

## Open questions

- How "non-interactive" is detected (stdin TTY? env override?) and the exit-code contract for the refusal path.
- Behavior when the repository has no remote, or the remote is unreachable/offline, in the remove flow and in `--remove-remote`.
- Soft vs hard delete when the user explicitly chooses "remove branch locally" for a branch whose commits are ALSO on the remote and the remote is AHEAD of local (remote-delete safety).
- The edge case of removing the worktree the user is currently standing in (git allows it; the shell is left with a dead CWD) -- refuse, cd-out via the shell wrapper, or document.
- Dirty-file display details: this spec colors by STATE (staged yellow, untracked red) but the current renderer colors by KIND (source red, non-source yellow) -- which axis wins, and what color is modified-unstaged; what the ">10 files" count includes (directories? file entries only?).
- `wt create --branch` (item 6) edge cases: what happens when the target branch already exists AND `--branch` is given; nonexistent base branch; whether the flag should be renamed (`--branch` competes with the positional branch argument; alternatives `--from` / `--base`).
- Whether spec item 1's layout fix is subsumed by the new remove flow's rendering (expected yes -- confirm).
- Acceptance criteria / definition of done per item, and the testing strategy for interactive prompts (scripted stdin vs real-terminal harness).
- Spec structure: this file bundles six changes (some features) under `fixes/` -- keep bundled or split.
- The non-interactive cells for the scope flags in the item 3 matrix (e.g. `--remove-branch` alone, non-interactive, on a dirty or ahead worktree): is the refusal reading correct?
- The truncated NOTE fragment below (recover or delete).

Truncated note from the original draft, preserved verbatim for recovery in a later round:

> NOTE: I do not remember
