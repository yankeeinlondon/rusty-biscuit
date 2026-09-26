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
reviewed: true
reviewed_by: codex/gpt-6-sol
reviewed_on: 2026-09-24
review_iterations: 10
completed: true
implemented: true
implemented_by: claude/opus
human_review: false
message_to_agent: |-
    All six phases are implemented; the spec is ready for review and is NOT moved to `_completed` (the author closes it). Phase 6 details are in "## Phase 6" of implementation-log.md. For the reviewer:
    - Phase 6 changed no behavior. Docs: `worktree/docs/cli/list.md` rewritten for the new table/graph, `worktree/README.md`'s `wt list` and cache sections, `docs/dependencies.md`, and one `os` skill fact (`dirs::cache_dir()` ignores HOME on Windows). Source edits are a comment-only quality pass over 9 files (two drifted comments fixed: `pr_age_markup`/`PrListing::age_minutes` and `safety::Evidence::DefaultBranch`).
    - No per-command docs were added for remove/go/create: `docs/cli/` holds only `list.md` and the README documents those commands in full.
    - Cross-OS: L1 green on macOS, Linux, native Windows, and WSL2; worktree-cli L2 green on macOS, Linux, and WSL2 with tmux required. Unmet, with provisioning as the required change: a Windows real-console L2 harness for PowerShell prompts, fish on build-linux and WSL2, zsh on WSL2, and Kitty on the Mac and build-linux.
    - Linux cross-check still needs `--no-default-features` (native path) because the standing clone on build-linux has stale read-only `librenderable-*.rmeta` links that break archive mode.
    - One failing test OUTSIDE this work: `biscuit-terminal-cli::level2 level2_prose_styling::level2_columns_word_wrap_in_pane` fails deterministically on this Mac because the shell's echo of the long `bt prose "aaa…"` command wraps into a row of only `a`s, which the test mistakes for output. The Prose output itself wraps correctly. Nothing in this branch reaches that code.
    - The author confirmed Decisions 20–33 and kept the narrow-terminal graph (Decision 34) on 2026-09-25; Decision 21 is amended by review 3's remote-repository finding, review 4's URL-rewrite finding, and review 5's remote-name finding.
---

# Worktree UX improvements

Throughout this spec, **default branch** means the repository's mainline branch (detected: main/master via the remote's HEAD, falling back to main or master). The short form "default branch" is used thereafter; the library detects it and hands it to the CLI.

Ahead/behind and merge-conflict comparisons use one **default-branch target**: whichever of the local default-branch tip and `origin/<default>` contains the other. If they diverge, use `origin/<default>` and say so in the caption. `wt` never fetches; a remote-tracking ref is only as current as the last fetch. With no remote-tracking ref, use the local tip. This target applies to the `wt list` table's `-> {default}` column (item 5) and to the ahead/behind facts in `wt remove`'s status report (item 3). The safety tiers separately inspect both refs.

This spec is about fixing the user experience rough edges in the `wt` CLI (and library) as well as addressing a few known bugs:

1. Display Issue When Deleting

    When you run `wt remove <worktree> -b` (and possibly without the `-b` too) you'll get a directory tree of files and awkwardly hang off the last line of the directory structure is a confirmation dialog saying that "All file changes will be lost".

    The dialog would be fine if it were BELOW the file directory listing (ideally with a blank line in between)

    Resolution: item 3's new flow prints its whole status report, including the dirty-file list, before any question, and each question starts after one blank line.

2. Shell Completions Incomplete

    When you try to autocomplete the name of the worktree you're wanting to switch to or delete the autocomplete options only work when you use the branch name, not the worktree name! Both should work.

    Git normally prevents one branch from being checked out in two worktrees, but forced or externally modified worktrees can still produce ambiguous metadata. Name resolution must handle that state instead of choosing a worktree arbitrarily.

    Completions (the library's `worktree_names`) will offer, for each non-main worktree, BOTH the branch name and the worktree's dasherized directory basename (deduped when the two are equal); plus `base`. They also offer the base checkout's actual branch name, if attached. The detected default branch name (e.g. `main`) appears only when that branch is checked out in a worktree. Detached worktrees contribute their directory basename only.

    The "dasherized" form of a name is its lowercased form with every character that is not alphanumeric or a dash replaced by a dash; it is the form used for the worktree's directory name.

    Resolution contract (the library's `find_worktree`):

    - `base` resolves to the main checkout, whatever branch it has checked out.
    - For every other input, collect exact branch-name matches and exact directory-basename matches. One distinct worktree resolves. Multiple distinct worktrees fail as ambiguous, whether the collision is between branches, basenames, or a branch and another worktree's basename. The error lists each worktree's branch (if attached), basename, and path. A basename is unique only within its parent directory, not across the repository.
    - This applies uniformly to `wt go` and `wt remove` (no CWD-based auto-disambiguation).
    - No match remains a not-found error. The base checkout cannot be removed; `wt remove base` and any name that resolves to the base checkout refuse before showing any removal prompt.

    Reader's note: mapping the default branch name directly to `base` would send `wt go main` to the wrong checkout when the base checkout is on another branch and `main` is checked out elsewhere. Branch names therefore resolve by actual checkout; `base` is the stable way to reach the main checkout.

3. Default Behavior for `remove` is Poor

    When you are trying to delete a worktree, we currently do NOT delete the branch and require the caller to remember to use the `-b` to make their intent clear that they DO want to remove the branch as well.

    The rewrite (ruled 2026-09-24) starts from one question: **would removing this lose work?** `wt remove <name>` removes the worktree AND its local branch whenever the safety checks show that work remains elsewhere, never deletes the remote branch unless told to, and loses work only after an explicit `--force-*` flag or an interactive answer.

    **Safety tiers.** The worktree's safety depends on its dirty files; the branch's safety depends on where its last commit can be found. Checking the last commit is enough: every earlier commit on the branch is part of its history, so wherever the last commit is found, all of them are.

    | Tier | The branch's last commit is found on... | Interactive | Non-interactive |
    |---|---|---|---|
    | **Safe** | the default branch (local or `origin/<default>`), or the head of an open or merged PR on origin | Remove; no prompt | Remove |
    | **Pretty safe** | any other local branch, any `origin/*` branch (including this branch's own origin copy), or a tag | Remove; no prompt; one line names where the work still lives (e.g. "your commits also live on `feat/theme`, which is not merged into main yet") | Remove |
    | **Not safe** | nowhere else | Show what would be lost; ask (see below) | Keep the branch (see below) |

    - The worktree is safe to remove when it has no dirty files (modified, staged, or untracked) and no ignored entries. Git's ignore rules keep files out of version control; they do not establish that their contents are disposable, so a local `.env`, notes, or build output all need consent (ruled 2026-09-24, Decisions 18).
    - "Found on" means the last commit is part of that ref's history. Local refs and tags are checked directly; an `origin/*` ref is only an observation from the last fetch. The report labels remote-tracking evidence "as of your last fetch". Before using a remote-tracking ref other than `origin/<default>` to justify automatic local-branch deletion, apply the live check below.
    - An open PR counts as Safe only when the provider reports that its source head is the exact current local branch tip; then the provider still has those commits. A merged PR counts as Safe only when its recorded source head is the exact current local branch tip, so commits made after the merge cannot be mistaken for merged work. This also covers squash and rebase merges, whose resulting commits have different SHAs. A PR for another source repository must match that repository's identity as well as the branch name. If the provider cannot supply the source head, source repository, or definitive open/merged state, the PR grants no Safe evidence. An unavailable answer (offline or no credentials) falls through to the other refs; a pushed branch can still be Pretty safe.
    - A remote-tracking ref is evidence only of the last observed state. Before deleting the local branch automatically on the strength of an `origin/*` ref other than `origin/<default>`, query that live remote branch head. It qualifies when the live SHA equals the local tip, or when it equals the remote-tracking SHA and that local ref contains the local tip. Otherwise the live remote might have moved or have unobserved commits; keep the local branch and report that its remote copy could not be verified. Local refs and tags remain usable offline. This check does not fetch or mutate local refs. `origin/<default>` is exempt: in a PR-driven workflow it moves constantly, so its live head rarely equals the last-fetched one, and requiring that would keep nearly every merged branch; the default branch is also the ref least likely to be rewritten or deleted.
    - A detached worktree has no branch; only its dirty files matter.

    **Flags.** Three flags, one per object, each authorizing removal without a safety prompt whatever the tier. They cannot override git errors, a failed shell handoff, a held Windows directory, or remote protection. There are no short forms: "force" is meant to make the caller stop and think, and full cleanup of unsafe work takes all three.

    | Flag | Guarantees | Git equivalent |
    |---|---|---|
    | `--force-worktree` | The worktree is removed, dirty files included | `git worktree remove --force` |
    | `--force-branch` | The local branch is removed, even if its commits exist nowhere else | `git branch -D` |
    | `--force-remote` | The branch on origin is removed, if there is one; if there is none, the report says so | `git push origin --delete` |

    Without flags the remote branch is never deleted, even when that would be harmless: deleting from origin affects other people and closes any open PR on the branch.

    **When `--force-remote` is given**, the branch's tier is computed without its own origin copy and without its open PR, since the command is about to delete both. A branch that was Pretty safe only because it was pushed becomes Not safe, and without `--force-branch` the local branch is kept, so the work survives locally. The remote destination comes from the branch's configured upstream, if it is on `origin`, or otherwise from `origin/<branch>`; never infer a remote branch from a PR in a different repository. If neither exists, report that no matching remote branch exists. Before deletion, query the live remote head. If its commits are unavailable locally, say that their number and contents are unknown rather than claiming there are none; otherwise show remote-only commits. Use a lease against the observed SHA for deletion, so a push between the report and deletion causes a failure rather than deleting new work. A protected-branch refusal is an ordinary remote failure. Do not claim success from a stale remote-tracking ref.

    **Flow.**

    1. **Report first.** Before asking anything or removing anything, `wt remove` prints the status:
        - The worktree's dirty files: up to 10 are listed as a tree, colored by kind with the table's dot palette (source files orange, other files yellow); above 10, a bold red count of every dirty path (modified, staged, and untracked) replaces the list. Today's 50-file cap and its "...and N more" line are deleted.
        - The ignored entries removal would also delete (top-level matches from `git status --ignored=matching`, which does not descend into ignored folders, e.g. ".env, notes.md, target/"), stating that their contents will be deleted.
        - The branch's tier and where its work lives.
        - Its ahead/behind against the selected default-branch target, named in the text (e.g. "3 ahead, 12 behind origin/main").
        - Its origin copy (as of the last fetch) and any PR.
    2. **Worktree.** Clean with no ignored entries, or `--force-worktree`: removed. Dirty files or ignored entries without the flag: interactive asks "discard these files and remove the worktree?", naming the dirty files and the ignored entries (default No; No removes nothing); non-interactive removes nothing and exits with an error naming them and `--force-worktree`.
    3. **Local branch** (only once the worktree is removed; git cannot delete a branch a worktree has checked out). Safe, Pretty safe, or `--force-branch`: deleted. Not safe without the flag: interactive shows the commits that would be lost (subjects, or a bold red count when there are more than 10) and offers "keep the branch" (default) or "delete it and lose N commits"; non-interactive keeps the branch, prints a warning naming it and why, and still succeeds (exit 0). `wt remove` removes everything it safely can.
    4. **Remote branch.** Only with `--force-remote`.

    Classify safety only after resolving the actual target and collecting the refs. If the ref check or git command needed to establish safety fails, keep the branch and report the failure; an unknown result is never Safe or Pretty safe. `--force-branch` explicitly overrides that uncertainty. The status report distinguishes local commits from remote-only commits, and names the exact ref used as evidence for each tier.

    `--force-branch` on a dirty worktree, or one with ignored entries, without `--force-worktree` is a conflict: deleting the branch requires removing the worktree and its files. Non-interactive: an error before anything is removed ("`--force-branch` needs the worktree removed first, and it has uncommitted files; add `--force-worktree` to discard them"). Interactive: step 2's question about the files is asked instead.

    **Removing the worktree you are standing in** (ruled 2026-09-24). `wt remove` removes it and moves the shell out through the wrapper's `cd:` protocol, the same one `wt go` uses:

    - It lands in the fork parent's worktree when the branch has a fork-origin record and that parent branch has a worktree; otherwise in the base repo. Either way it keeps the current subdirectory when that path exists there (`wt go`'s rule) and says where the caller landed.
    - **Move first, then remove, on every OS** (spike-confirmed on Windows, 2026-09-24). On Windows a directory that is a process's current directory cannot be deleted, so the shell must leave before the removal runs; using the same sequence everywhere keeps one code path that every OS tests:
        1. The first `wt remove` run does every check and asks every question, but removes nothing. It records the selected worktree's canonical path, its current HEAD and branch tip, the dirty-state fingerprint, the intended landing path, the approved force choices, and, with `--force-remote`, the resolved push endpoint (Decision 21) in a handoff record in the user-cache store. The fingerprint covers each modified, staged, and untracked path, its status, and its working content, plus Git's logical index entries (mode, object ID, and conflict stage per path, not the raw index file), with `biscuit-hash` BLAKE3, so re-staging different content under an unchanged status still refuses (review 3). The fingerprint also covers the set of ignored top-level entries, so a new ignored entry appearing between the runs makes the second run refuse. The record expires after one minute and is keyed by a random, hard-to-guess token. Standard output contains a `cd:` line and a fixed `remove-handoff:` token. This is a protocol token, not a shell command; the wrapper never evaluates output as shell code.
        2. The wrapper changes directory. Only after a successful `cd` does it invoke `wt remove --handoff <token>`; a failed `cd` returns an error and leaves the worktree intact. The wrapper passes the token as one quoted argument and never interpolates it into a command string.
        3. The second run checks that the token identifies the same repository, worktree path, HEAD, branch tip, dirty state, and landing destination, and that the caller is outside the target. It re-checks the safety tiers without prompts. Any state change or new risk refuses with nothing removed and asks the caller to start again. Consume the token before mutation so it cannot be replayed. An expired or missing token also refuses without mutation.
      The caller is only moved once every question is answered; a refusal or a cancelled prompt in the first run leaves the shell where it is.
    - `wt` runs git against the base repo (`git -C`), never from inside the target. The second run starts from the landing directory after the wrapper moves; a direct removal from another worktree also runs from outside the target.
    - When the shell wrapper is not active, so `wt` cannot move the shell, `wt remove` refuses (exit as a refusal, nothing removed) with the same "shell wrapper not active" help `wt go` shows, plus the option of running the command from another directory.

    **Windows** (ruled 2026-09-24 from measurements on the Windows build host; the details are in the `os` skill's `windows.md`, "Current-directory locks"):

    - Which shells hold the lock: cmd.exe after `cd`, and any window launched inside the directory, do; PowerShell after `Set-Location` and Git Bash after `cd` do not.
    - `git worktree remove` on a held directory fails late: it deletes every file and unregisters the worktree, then exits 255 leaving an empty directory. So before calling it, `wt remove` checks for the lock by renaming the directory to a sibling name and straight back. If it is held (another cmd window, a terminal tab opened there, an open file), `wt remove` refuses with nothing removed and says the folder is in use by another program. This check runs for every removal on Windows, not only the worktree the caller is standing in.
    - The PowerShell wrapper (item 7) moves both the location (`Set-Location`) and the process's current directory (`[Environment]::CurrentDirectory`); moving only the location leaves a window that was launched inside the worktree still holding it.
    - cmd.exe cannot have a wrapper, so it follows the "wrapper not active" rule: `wt remove` refuses to remove the worktree the caller is standing in.

    **Interactive or not** (ruled 2026-09-24). A run is interactive when stdin and stderr are both terminals and the `CI` environment variable is not set. Standard output is not part of the test: the shell wrapper always captures it.

    **Exit codes** (ruled 2026-09-24). 0, 1, and 2 mean the same for every `wt` command; 3 and 4 are used where the command refuses.

    | Code | Meaning | `wt remove` cases |
    |---|---|---|
    | 0 | Done | Everything requested was removed; the branch was kept with a warning (Not safe, no `--force-branch`); the caller cancelled at a prompt; the first run of move-first handed off to the wrapper (the wrapper stops on a non-zero exit) |
    | 1 | Something failed | A git command failed; origin could not be reached during `--force-remote` (the local removal has already happened, and the report lists what was removed and the command that finishes the job, `git push origin --delete <branch>`) |
    | 2 | Invalid arguments | clap's existing behavior, e.g. the retired `-b` |
    | 3 | Refused to avoid losing work; nothing removed | A dirty worktree, or one with ignored entries, without `--force-worktree`; `--force-branch` on a dirty worktree without `--force-worktree`; the second run of move-first found a risk it was not given a flag for |
    | 4 | Blocked by the environment; nothing removed, and no `--force-*` flag helps | Standing in the worktree without the shell wrapper; the folder is in use by another program (the Windows lock check); an expired or missing handoff token (start again) |

    Codes 3 and 4 are separate because the fix differs: for 3, a `--force-*` flag if the loss is acceptable; for 4, move elsewhere or close the program. Elsewhere, `wt go` without the wrapper exits 4 (today it exits 0 although nothing happened); `wt create` without the wrapper still creates the worktree and exits 0 with a message that it could not move the shell.

    **Lost-commit count.** Where the flow names a number of lost commits, it is the commits on the branch that are not part of any other local branch, `origin/*` branch, or tag (leaving out this branch's own origin copy when `--force-remote` is given).

    **Examples.**

    | Command | State | Result |
    |---|---|---|
    | `wt remove old-cleanup` | Clean; branch merged into main | Worktree and branch removed |
    | `wt remove feat-dark-fixes` | Clean; branch pushed, no PR | Worktree and branch removed; note that the commits live on `origin/feat/dark-fixes` |
    | `wt remove spike-parser` (no terminal) | Clean; 4 commits found nowhere else | Worktree removed; branch kept with a warning; exit 0 |
    | `wt remove spike-parser --force-branch` | Same | Worktree and branch removed; 4 commits lost |
    | `wt remove fix-wt-ux` (no terminal) | 3 dirty files | Error; nothing removed |
    | `wt remove fix-wt-ux --force-worktree --force-branch --force-remote` | Anything | Worktree, local branch, and `origin/fix/wt-ux` removed; PR #99 closes |

    **Retirements.** No deprecation period (the tool has no installed users); retired flags become standard clap "unexpected argument" errors.

    - `-b`, `-f`/`-ff` (the count matrix and its 10-file bypass limit, `FORCE_BYPASS_FILE_LIMIT`), and the earlier proposed `--force`, `--remove-branch`, and `--remove-remote` are gone.
    - The four-choice branch menu, the flag-by-interactivity matrix, and the "unique commits" gate of the earlier draft are replaced by the tiers above.
    - The docs and help text (the AFTER_HELP examples) are updated accordingly.

4. BUG: Branch not Removed Though Requested

    There are times when -- using the current functionality -- a user calls `wt remove <worktree> -b` (aka, explicitly says they want the branch removed too) and it fails to remove the branch. This is inconsistent because sometimes it DOES remove the branch and it is not clear what determines whether the `-b` flag is respected or not.

    Resolution within the new flow: the `-b` flag is retired (item 3). Its inconsistent behavior came from the soft-delete attempt (`git branch -d`) silently refusing a branch git considered unmerged and reporting it as "preserved". In the item 3 flow, `wt` decides branch deletion itself from the safety tiers and deletes with `git branch -D`: the branch is deleted when it is Safe or Pretty safe or `--force-branch` is given, and is otherwise kept with an explicit warning (or kept or deleted by the user's answer, interactively). There is no silent refusal path.

    Example: 

    ```sh
    Removed worktree fix/ci-build-feature-divergence at /Volumes/coding/wt/rusty-biscuit/feat-dark-fixes
    Warning: branch fix/ci-build-feature-divergence was preserved: warning: not deleting branch 'fix/ci-build-feature-divergence' that is not yet merged to
             'refs/remotes/origin/fix/ci-build-feature-divergence', even though it is merged to HEAD
    error: the branch 'fix/ci-build-feature-divergence' is not fully merged
    hint: If you are sure you want to delete it, run 'git branch -D fix/ci-build-feature-divergence'
    hint: Disable this message with "git config set advice.forceDeleteBranch false"
      Run git branch -D fix/ci-build-feature-divergence if you want to force-delete it.
    ```

5. More Complete Information

    `wt list` should show whether each branch can merge into the selected default-branch target and, where known, its fork parent. This describes merge readiness; item 3's `wt remove` report makes the separate decision about what deletion would lose. A remote-tracking ref is local, last-fetched information; a network outage does not change the comparison target unless that ref is absent.

    ### Table Design

    The current design looks like this:

    ```txt
    ┌──────────┬───────────────────────┬───────────┬───────┬─────────┐
    │ Worktree │ Worktree Name         │ Branch    │ Merge │ Commits │
    ├──────────┼───────────────────────┼───────────┼───────┼─────────┤
    │  Dirty   │ main::(rusty-biscuit) │ main      │       │         │
    │  Dirty   │ fix-wt-ux             │ fix/wt-ux │ clean │      +1 │
    └──────────┴───────────────────────┴───────────┴───────┴─────────┘
    ```

    The redesigned table (agreed 2026-09-24 through terminal prototypes):

    ```txt
     [main] is 7 commits behind [origin/main]

    ╭───────────────────┬───────────────────────┬──────────────────┬────────────────╮
    │ Worktree          │ Branch                │ -> [origin/main] │ -> parent      │
    ├───────────────────┼───────────────────────┼──────────────────┼────────────────┤
    │ ○ base repo       │ [main]                │ —                │ —              │
    │ ● fix-wt-ux       │ ├─ fix/wt-ux          │ clean [PR #99]   │ —              │
    │ ● feat-theme      │ ├─ feat/theme         │ clean            │ —              │
    │ ○ feat-dark-fixes │ │  └─ feat/dark-fixes │ clean            │ conflicts      │
    │ ○ old-cleanup     │ └─ chore/old-cleanup  │ already in       │ —              │
    │                   │ experiments (deleted) │                  │                │
    │ ● spike-parser    │ └┄ spike/parser       │ conflicts        │ parent deleted │
    │ ● bisect          │ detached @ a1b2c3d    │ —                │ —              │
    ╰───────────────────┴───────────────────────┴──────────────────┴────────────────╯

     Worktree   ○ clean    ● uncommitted files    ● uncommitted source files
     Branch     ├─ merges cleanly into parent    ├─ conflicts with parent    └┄ parent deleted
    ```

    `[name]` marks a badge (a colored background with a space of padding on each side). The prototype that renders this with real color is `wt_list_prototypes.py` (session scratchpad; not checked in).

    Every column after Branch answers one question: **what happens if this row's branch merges into the column's target?** The table deliberately does not show a branch's relationship to its own `origin/*` copy. For a non-default branch that relationship is rarely the one that matters (the remote copy often does not exist until a PR is raised); the targets that matter are the default branch and the fork parent.

    **Caption line** -- above the table, one line states the default branch's relationship to its origin peer, the only branch for which that relationship matters:

    - `[main] is N commits behind [origin/main]` (the usual PR-driven state), `[main] is N commits ahead of [origin/main]` (Gitflow-style local merges), or `[main] is in sync with [origin/main]`.
    - Only the count ("N commits") is colored (yellow); the rest of the sentence uses the default foreground.
    - No caption when there is no remote or no remote tracking ref.

    **Worktree column** -- `{dot} {name}`. The dot encodes the working tree's dirty state (the library's existing `DirtyStatus`):

    - `○` dim ring -- clean
    - `●` yellow -- uncommitted files, none of them source code
    - `●` orange -- uncommitted source files

    The dots are single-column text glyphs colored by ANSI, not emoji. The base-checkout row shows `<i>base repo</i>` (dim italic) in the name position instead of today's `main::{repo}`. The current worktree's name is bold.

    **Branch column** -- the fork lineage drawn as a tree, rooted at the default branch. The tree decides the row order.

    - The default branch is shown as a local-branch badge (`[main]`).
    - Each branch sits under its fork parent (from the fork-origin record). The connector's color says whether the branch still merges cleanly into its parent: gray = clean, red = conflicts. This deliberately repeats the `-> parent` column (see Decisions 14).
    - A parent branch that has no worktree still gets a tree row, with an empty Worktree cell and empty target cells. This covers the default branch itself when the base checkout sits on another branch, which replaces the earlier "prepended default-branch row" rule.
    - A fork parent that has since been deleted gets a row showing its name dim, italic, and struck through, followed by `(deleted)`; its children hang from it with a dotted connector (`└┄`) in a neutral color.
    - A branch with no fork-origin record (a plain-git worktree, or one created before this feature) sits at the root level with no connector.
    - A detached worktree shows `detached @ {short sha}` dim italic at the root level.

    The fork-origin record supplies the parent name. Kept from the previously-ruled T2 mechanism: `wt create` persists { base branch, base sha, created at } in the library's user-cache store keyed by branch. The reused-existing-branch path records nothing. Stale records (branches deleted) are pruned during the save that `wt list` already performs.

    **`-> {default}` column** -- the header names the selected default-branch target as a remote badge (`-> [origin/main]`), or local `-> [main]` when the local tip contains the remote tip or there is no remote. Cells:

    - `already in` (dim italic) -- every commit on the branch is already on the target.
    - `clean` (dim italic) -- the branch has commits the target lacks, and merging them would not conflict.
    - `conflicts` (red) -- merging would conflict.
    - `—` (dim) -- not applicable: the default-branch row and detached worktrees.

    **`-> parent` column** -- the same vocabulary against the branch's fork parent. `—` when the parent is the default branch (the previous column already answers it), when there is no fork-origin record, and on the default-branch and detached rows. `parent deleted` (neutral color) when the recorded parent no longer exists.

    **PR badge** -- an open PR is a proposed merge into a specific base branch, so its badge (`[PR #99]`, a link to the PR) follows the cell of the column whose target is the PR's base: the `-> parent` column when the PR targets the fork parent, or the `-> {default}` column when it targets the selected default branch. If it targets neither, show `PR #99 → {target}` beside the branch name; placing it under either target column would imply a merge destination the PR does not use. The badge is a link only in terminals that support OSC 8 links; elsewhere it shows the PR number with no visible URL (see Dropped by design). No badge when there is no open PR; the absence of a PR is not a problem to flag, since many workflows never use PRs.

    **Badges** -- three roles with distinct background colors: a local branch (`[main]`), a remote branch (`[origin/main]`), and a PR (`[PR #99]`). Badge text is padded with one space on each side and needs no special font.

    **Legend** -- two lines under the table, each starting with the column it explains (`Worktree`, `Branch`) in the default foreground, followed by the glyph samples and their meanings in dim.

    **Row emphasis** -- the current worktree's row gets a very subtle background color. Feasibility fact (a dependency): biscuit-terminal's Table component has NO per-row highlight API today (only even-row striping and per-cell styling); this requires ADDING a per-row highlight slot to the Table component -- see Cross-package dependencies.

    **Dropped by design** --

    - Per-branch ahead/behind numbers. Only the caption line keeps a count.
    - A branch's relationship to its own `origin/*` copy (pushed, ahead, behind). Whether removing a worktree loses work is answered by `wt remove`'s status report (item 3), which establishes the remote state itself; the list table does not answer it.
    - The earlier emoji proposals (👍/📙/📕 status glyphs, ✔/💥 merge icons, 🌏/📦 remote icons) and a separate Loss column.
    - The old 120-terminal-column suppression rule is RETIRED with the old layout. No width/capability suppression rules are defined for the new table; rules may be added later if needed.
    - A visible PR URL on terminals without OSC 8 links (ruled 2026-09-25, review 2). There the PR badge shows its number only; Prose's usual degradation to a visible `[text](url)` is not used for the badge, because a URL is awkward in this layout and `Table` never breaks inside a word, so one URL could leave the table no width to render in.

    **PR detection** -- the PR badge and its link require each open PR's number, URL, source repository, source branch, and target branch; git has no concept of PRs. Ruled: the **sniff library provides this cross-platform**. Sniff's `remote` module (feature-gated) already has normalized provider clients for GitHub, GitLab, Gitea, and Bitbucket, credential handling, `GitRemote::from_url` provider detection, the `commit_links` identity authority, and a `list_pull_requests(owner, repo, state)` provider method whose `PullRequestInfo` carries `number`, `html_url`, `source_branch`, and `target_branch`. `wt list` makes ONE repository-wide "list open PRs" request and matches PRs to worktree branches by both source repository and branch name; a same-named branch in a fork must not receive the wrong badge. `wt remove` is the exception: its Safe tier needs the open or merged PR for the one branch being removed, including the PR's recorded source head SHA, so it makes one per-branch request (same deadline; an unavailable answer falls through to the lower tiers). No new network dependencies or auth plumbing in the worktree area -- see Cross-package dependencies.

    Note: the fork base becomes an explicit input once item 6's `--from` flag lands (default: the current branch), so recording it is recording an input.

    ### Data Gathering

    The table must stay within the ratified `wt list` performance targets (`worktree/docs/performance-testing.md`). All git data is local and cacheable; the PR badges are the only network-dependent data, and the network never holds the table up.

    | Data | Source | Cost versus today |
    |---|---|---|
    | Default-branch tip, `origin/<default>` tip, fork-parent tips, and whether a recorded parent still exists | One `git for-each-ref refs/heads refs/remotes`, replacing today's `rev-parse {default}` | About the same |
    | Caption line (default branch vs its origin peer) | One `rev-list --left-right --count {default}...origin/{default}`, cached by SHA pair | One call, usually a cache hit |
    | `-> {default}` column | Today's `rev-list` + speculative `merge-tree`, aimed at the selected default-branch target; `already in` is "ahead = 0" | No new calls |
    | `-> parent` column | The same `rev-list` + `merge-tree` pair against the fork parent's tip, only for branches whose parent is not the default branch | Two calls per such branch, cached |
    | Branch tree | The fork-origin records in the user-cache store; built in memory | No git calls |
    | Worktree dot | Today's live `git status` per worktree (never cached) | Unchanged |
    | PR badges | One repository-wide open-PR request through sniff | Network; see below |

    **Comparison cache** -- the existing SHA-pair cache generalizes from (default tip, branch tip) to (target tip, branch tip) so one cache serves both target columns and the caption. The key shape changes, so `CACHE_FORMAT_VERSION` is bumped.

    **Furthest-ahead target** -- the `-> {default}` column compares against whichever of the local default branch and `origin/{default}` contains the other. When they have diverged (each has commits the other lacks), it uses `origin/{default}`, the copy everyone shares; the caption line already reports the divergence.

    **PR freshness: deadline plus cache** (ruled 2026-09-24) --

    - The open-PR request runs in parallel with the git work under a strict deadline (about 300 ms).
    - Results are stored per repository in the user-cache store with their fetch time. When the stored results are younger than a freshness window (about 60 s), `wt list` skips the network request entirely.
    - When the request fails or misses the deadline (offline, slow network, no credentials), the table shows the stored badges and a dim line under the legend stating their age (e.g. "PRs as of 12 min ago"). With nothing stored, the table shows no PR badges. Neither case is an error.
    - An authentication or permission error is an unavailable answer, not an empty list. Without credentials for a private repository, show cached badges with their age or omit them; never cache the error as "no open PRs".

    **Sync and async** -- sniff's provider clients are async and the worktree library is thread-based. Sniff provides a blocking entry point for the open-PR list so synchronous callers (the worktree library and others) do not each build their own runtime.

    ### Graph View

    Until now the graph showed only two branches (the current branch and the default branch) and only local commits; `wt list` from the default branch showed every worktree branch. The redesign (ruled 2026-09-24, "Option B") adds the default branch's origin peer, the fork parent, and open PRs without adding lanes for things that are not separate lines of work. The example below was drafted through biscuit-terminal's `MermaidDiagram`, the component `wt list` uses, so it stays within what the renderer can draw.

    **Lanes and tags** --

    - A **lane** is a line of work whose commits are not on any other drawn lane: the default branch, the fork parent when it is not the default branch, and the current branch. In the base view (from the default branch), every worktree branch that still has commits of its own gets a lane.
    - A ref whose tip sits on a drawn lane is a **tag** on that commit, not a lane: the local default branch, `origin/<default>`, and any branch that is already in the default branch. This is how `git log --decorate`, Jujutsu, and Sapling draw refs.
    - `origin/<default>` gets its own lane only when it has diverged from the local default branch. Otherwise the distance between the two tags shows the gap: `origin/main` ahead of `main` in a PR-driven workflow, the reverse in a Gitflow-style one.
    - An open PR is a tag on its head branch's tip naming its target: `PR #99 → main`, `PR #104 → feat/theme`.
    - The `+N` elision square keeps today's meaning: commits the graph leaves out.

    ```mermaid
    gitGraph
        commit id: "4c2e4d5"
        commit id: "2596f1d" tag: "main"
        branch feat/theme
        checkout feat/theme
        commit id: "7a1b2c3"
        commit id: "8b2c3d4"
        branch feat/dark-fixes
        checkout feat/dark-fixes
        commit id: "9c3d4e5"
        commit id: "0d4e5f6" tag: "PR #104 → feat/theme"
        checkout feat/theme
        commit id: "1e5f6a7"
        checkout main
        commit id: "+4" type: HIGHLIGHT
        commit id: "d44f301"
        commit id: "e55a412"
        commit id: "f66b523" tag: "origin/main"
    ```

    (Standing in `feat-dark-fixes`, forked from `feat/theme`; local `main` is behind `origin/main`.)

    **Renderer limits** (`mermaid-rs-renderer`; present in 0.2.1, 0.3.1, and upstream `main` as of 2026-09-24) --

    - The `branch` and `merge` statements take everything after the keyword as the branch name, attributes included. `branch feat order: 1` creates a lane named `feat order: 1`, and a later `checkout feat` creates a second, disconnected lane; `merge feat id: "m"` looks up a branch named `feat id: "m"`, finds none, and draws the merge commit without the line from `feat`. Until fixed, the component emits no attributes on those two statements: lane order comes from branch creation order, and merge commits get the renderer's generated IDs. Once the 0.3.1 upgrade lands, the bug is reported upstream as an issue with a minimal reproduction plus a small PR that strips attributes from the branch name in both statements; both are drafted for the author's review before anything is filed.
    - There is no per-lane or per-edge styling, so the graph cannot highlight the current branch or color a conflict; the table carries those facts.

    **Ownership** -- a new biscuit-terminal `GitGraph` component owns the lane/tag rule, lane ordering, elision markers, and the renderer workarounds above. The worktree CLI gathers the git facts (commits per line with full SHAs, ref tips, open PRs) and hands the component typed data; it no longer builds Mermaid text itself. The component runs no git commands. It implements `TerminalRenderable`, `BrowserRenderable` (the renderer's SVG as a raw-HTML island, as `GraphExpression` does), and `TreeRenderable`. See Cross-package dependencies.

    **Graph sizing** (ruled 2026-09-24) -- nodes and text must be the same size in every diagram, whatever its commit count. Today `wt` picks a width in terminal columns from the commit count (`default_graph_width` in `worktree-cli`), `MermaidDiagram` converts columns to pixels, and resvg rasterizes the SVG to exactly that width, so node size is a side effect. Measured on the drafts, the lookup table approximates about 5.4 SVG units per column, but width is also driven by the branch-name label column and tag lengths, which commit count does not predict. `default_graph_width` is deleted.

    - **Scale, not width.** biscuit-terminal gains a scale-based image width (e.g. `ImageWidth::Scale(f32)`). Pixels per SVG unit = scale × cell height ÷ 16, so at 100% the diagram's 16-unit body text is exactly one terminal line tall. Columns = SVG width × pixels per unit ÷ cell width, rounded up. Cell sizes are real pixels, so the result is resolution-independent (sharper on high-DPI screens, same visual size).
    - **Natural size.** The SVG's natural width comes from the renderer: `render_svg_with_dimensions` in `mermaid-rs-renderer` 0.3 (see Cross-package dependencies).
    - **Default scale.** `MermaidDiagram` defaults to 100%. `GitGraph` defaults to 125% (ruled 2026-09-24 after comparing 100/125/150% in a simulated 120-column terminal): gitGraph's commit IDs are drawn at 10 units and branch labels at 14, so they need more than the 100% body-text reference to read comfortably. On the drafts, 125% gives 83 × 13 (columns × rows) standing in `fix-wt-ux` and 104 × 19 standing in `feat-dark-fixes`.
    - **Cap.** The width is clamped to the available columns minus margins. Only then does a diagram shrink, so a very narrow terminal shows a smaller graph.
    - **Height cap (base view).** Height grows by about 80 SVG units per lane (nine lanes need about 52 rows at 125%), and trimming commits does not reduce it. The graph is capped at about half the terminal's rows; past the cap, `GitGraph` shows fewer lanes, most recently active first, followed by a line such as "4 more worktrees not shown". The table above already lists every worktree, so the graph does not need to be complete. Views from a feature branch have at most four lanes and are not affected.
    - **Unknown cell size.** Falls back to the existing 8×16 assumption: sizes stop matching the terminal font but stay consistent across diagrams.
    - **Fit by trimming, not shrinking.** `GitGraph` knows the scale and the cap before rendering. When the graph would exceed the cap, it shows fewer commits (a larger `+N` elision) rather than shrinking nodes; shrinking remains only for terminals too narrow to fit even the minimum graph.

6. Forking Freedom

    Today when the `wt create` command is used, the new worktree will be forked from the branch the user is currently on. This is a good default but the user should be able to choose a different branch as a base if they want to:

    - we should add the `--from <base>` flag to allow a user to explicitly state which branch they want to fork off of (default: the current branch): `wt create fix/x --from feat/theme`. (Ruled 2026-09-24: `--from`, because `wt create <branch>` already names the NEW branch, so `--branch` would name two different branches in one command.)
    - If the new branch already exists and `--from` is given, `wt create` fails instead of silently ignoring the flag: "`feat/theme` already exists, so `--from main` would be ignored. Drop `--from` to reuse it." Without `--from`, today's reuse-with-warning behavior stays.
    - A `--from` branch that does not exist is an error naming it.
    - `--from` names an existing local branch. A detached checkout has no current branch to record as a fork parent, so creating a new branch from it requires `--from`; the error explains that choice. This prevents a detached HEAD SHA from being written as a branch name in the fork-origin record.
    - The fork-origin record (item 5) stores the `--from` branch, or the current branch by default.

7. Shell Wrapper Detection

    `wt go`, `wt create`, and `wt remove` move the caller's shell by printing `cd:<path>` on standard output, which the shell wrapper (a shell function) acts on after `wt` exits. Today `wt` decides the wrapper is active when standard output is not a terminal. That is also true for scripts, `just` recipes, editor task runners, and AI agents, which capture standard output but never act on `cd:`. Under item 3's rule for removing the worktree you are standing in, an agent could be left in a deleted directory.

    Ruled 2026-09-24: the wrapper announces itself. Every wrapper sets `WT_SHELL_WRAPPER=1` for the one `wt` invocation it makes (e.g. `WT_SHELL_WRAPPER=1 command wt "$@"`), and `wt` treats only that variable as proof it can move the shell.

    - With the variable: `cd:<path>` is printed as today.
    - Without it: `wt go` and `wt create` print the existing "shell wrapper not active" help instead of `cd:`; `wt remove` of the worktree the caller is standing in refuses (item 3).
    - The wrappers are generated by `wt --completions <shell>` (bash, zsh, fish, in `worktree-cli`'s `main.rs`), which becomes their only source: the hand-written copies in `worktree/shell/` (`wt.sh`, `wt.fish`) are deleted and the docs point to `wt --completions` (ruled 2026-09-24).
    - A PowerShell wrapper is added (`wt --completions powershell`), so native Windows users get `wt go`, `wt create`, and item 3's move-first removal. It sets `[Environment]::CurrentDirectory` along with `Set-Location` (item 3's Windows notes). There is no wrapper for cmd.exe.
    - Every wrapper understands item 3's `remove-handoff:` token as well as `cd:`: after a successful `cd` it runs the fixed command `wt remove --handoff <token>`, passing the token as one quoted argument, and never evaluates `wt`'s output as shell code.

## Acceptance criteria and testing

Testing follows the `rust-testing` skill's levels: L1 for logic against temporary repositories, L2 for real-terminal behavior through `biscuit-test-harness`, never taking window focus. Every criterion holds on macOS, Linux, native Windows, and WSL2; where a Windows or WSL2 L2 cell is not provisioned in CI, the criterion is recorded as unmet with provisioning as the required change, never narrowed.

1. **Display.** Every question `wt remove` asks starts after one blank line below the report (L2 capture).
2. **Completions.** `worktree_names` offers each worktree's branch name and directory name (deduped), plus `base`; the default branch name appears when it is checked out. `find_worktree` resolves in the ruled order, including when the base checkout is on a non-default branch. Two worktrees on one branch or with the same basename produce an ambiguity error listing their paths (L1).
3. **Remove.**
    - Every combination of tier (Safe, Pretty safe, Not safe, including a merged PR and a tag), dirty state, ignored entries (none, a file such as `.env`, a folder such as `target/`), `--force-*` flags, and interactive/non-interactive produces the documented result and exit code (L1; PR answers from a stub, including "unavailable").
    - The prompts, and move-first through each wrapper (zsh, bash, fish, PowerShell), work in a real terminal, land in the fork parent's worktree or the base repo, and say where. A failed `cd`, an expired token, or a changed worktree or branch between the two runs leaves the worktree intact; wrapper output is never evaluated as shell code (L1 and L2).
    - An open or merged PR counts as Safe only for the same source repository and exact branch tip. A stale or deleted `origin/*` ref cannot by itself justify automatic deletion; remote unavailability preserves the local branch. A remote push between preflight and `--force-remote` deletion fails the lease, preserving the new remote head (L1 with local bare remotes and stubbed provider answers).
    - On Windows, a held directory is detected before `git worktree remove` and produces exit 4 with nothing removed; move-first removes a worktree the PowerShell window was launched inside (L2 on Windows).
4. **Branch removal.** The original bug's case (a branch merged into HEAD but not into its upstream) is deleted or kept exactly as the tiers say, never silently preserved (L1).
5. **List and graph.**
    - Table cells, caption variants, badges, and legend render as ruled (L1 snapshots).
    - `list gather` stays within the ratified targets (warm 120 ms, cold 300 ms; full non-image `wt list` 1 s), including with the network down and a slow PR request hitting its deadline; stored PR results within the freshness window skip the network (L1 and the existing performance gates).
    - `GitGraph` applies the lane/tag rule, trims commits to fit the width cap and lanes to fit the base view's height cap, and sizes images from the scale rule (L1 on the generated Mermaid text and computed sizes).
    - The `mermaid-rs-renderer` 0.3.1 upgrade passes `biscuit-visualized`'s suite, including the fixed pie-contrast test.
6. **`--from`.** A worktree forks from the named local base and records it; `--from` with an existing destination branch, a nonexistent or nonlocal base, and a detached checkout without `--from` fail with the ruled messages (L1).
7. **Wrapper detection.** `cd:` and the `remove-handoff:` token are printed only when `WT_SHELL_WRAPPER=1` is set; `wt go` without it exits 4; every generated wrapper, including PowerShell, sets the variable, handles both protocol lines, checks the `cd` result, and invokes the fixed `wt remove --handoff` command (L1 for the output, L2 per shell).

## Sequencing

Fixes first (ruled 2026-09-24):

1. **Fixes:** shell wrapper detection and the PowerShell wrapper (item 7), completions (item 2), the remove flow (items 1, 3, 4), `wt create --from` (item 6), and sniff's PR-for-one-branch entry point. Without that entry point, the Safe tier's PR check falls through to the lower tiers, so the remove flow can land first.
2. **Dependencies for the table and graph:** the `mermaid-rs-renderer` 0.3.1 upgrade, biscuit-terminal's row highlight, scale-based width, and `GitGraph`, and sniff's open-PR list.
3. **List and graph:** item 5.

## Packages

- `worktree` (library): the name-resolution contract (`find_worktree`, `worktree_names`, including ambiguity errors), default-branch detection, state classification (dirtiness, ahead/behind), the remove flow's safety tiers and lost-commit count, default-branch target selection (local or last-fetched remote tip), per-branch mergeability against the default branch and fork parent, live verification of remote safety evidence, open-PR state with its deadline and freshness cache (via sniff's `remote` feature), fork-origin persistence and the short-lived remove handoff in the user-cache store, `biscuit-hash` content fingerprinting for that handoff, lease-protected remote-branch deletion, and branch deletion (`git branch -D`, decided by the safety tiers).
- `worktree-cli`: the shell wrappers (`--completions` output only; `worktree/shell/` is deleted) and wrapper detection (item 7), the flag surface, rendering (the redesigned list table with its caption and legend, the dirty-file tree, the >10-file count), the prompts (confirmations and the choice menu), and gathering the graph's git facts for `GitGraph` (deleting `default_graph_width`).
- `biscuit-terminal`, `biscuit-visualized`, `sniff`: the changes listed under Cross-package dependencies. This spec is deliberately cross-package (ruled 2026-09-24).

## Cross-package dependencies

1. `biscuit-terminal` 
    - Table component: add a per-row highlight API (a subtle background on one designated row); no such API exists today; judged worth adding for reuse.
    - `GitGraph` component (new): typed input (lines of commits, ref tips, open PRs, elision counts) rendered through `MermaidDiagram`; owns item 5's lane/tag rule, lane ordering, the `mermaid-rs-renderer` workaround, and fit-by-trimming; implements `TerminalRenderable`, `BrowserRenderable`, and `TreeRenderable`. Today `MermaidDiagram` itself implements only `TerminalRenderable` and `TreeRenderable` (its browser output through the tree is an empty-URL placeholder), although it already has `render_to_svg()`.
    - Scale-based image width (item 5's Graph sizing): an `ImageWidth` variant sized from the SVG's natural width, the terminal cell size, and a scale relative to terminal text, capped at the available columns; used by `MermaidDiagram` and therefore by `GitGraph`.
2. `biscuit-visualized` -- upgrade `mermaid-rs-renderer` from 0.2 (locked at 0.2.1) to 0.3.1 (ruled 2026-09-24):
    - Wanted for the 0.3 layout and theme fixes, for `render_svg_with_dimensions` (Graph sizing), and so the upstream report of the attribute-parsing bug is made against the latest release.
    - A trial upgrade on 2026-09-24 compiled with no code changes (`resvg`/`usvg` move to 0.47, adding `tiny-skia` 0.12); the gitGraph drafts rendered byte-identical; biscuit-terminal's Mermaid, diagram, and parity tests (1,088) and Darkmatter's Mermaid tests (61) passed.
    - One `biscuit-visualized` test fails: 0.3 changed the default third pie color from `#FFFFFF` to `hsl(0, 0%, 60%)`, and `fix_pie_text_contrast` reads only hex colors. Fix: parse `hsl()` in the contrast helper and update `mermaid_pie_chart_init_directive_applies_custom_colors`, whose comment assumes a white slice.
    - Update the version noted in `biscuit-terminal/docs/data-visualization/visualizing-graph-expressions.md`.
3. `sniff` -- `remote` module: blocking entry points, built on the existing async `list_pull_requests` provider method, credential handling, and `commit_links` identity authority, for (a) a repository's open PRs (number, URL, source repository, source branch, target branch) for `wt list`, and (b) the open or merged PR for one source repository and branch, including its recorded source head commit SHA and definitive state, for `wt remove`'s Safe tier. `PullRequestInfo` does not carry all of these identity and head fields today. An authentication failure must remain distinguishable from an empty PR list. Both entry points work for all four providers sniff supports (GitHub, GitLab, Gitea, Bitbucket) at launch (ruled 2026-09-24).

These changes are part of this spec (ruled 2026-09-24: the spec is cross-package); the order they land in is under Sequencing.

## Decisions

1. Ruled 2026-09-24: interactive means stdin and stderr are both terminals and `CI` is not set, and the exit codes are 0 done, 1 failure, 2 invalid arguments, 3 refused to avoid losing work, 4 blocked by the environment (see item 3).
2. Ruled 2026-09-24: when origin cannot be reached during `--force-remote`, the local removal has already happened; `wt remove` exits 1 and the report lists what was removed and the `git push origin --delete <branch>` command that finishes the job. With no remote at all, `--force-remote` reports that there is no remote branch. Local refs, tags, and `origin/<default>` need no network; evidence from other `origin/*` refs is verified live and, when the remote cannot be reached, does not count, so the local branch is kept. An unavailable PR answer falls through to the lower tiers (item 3); the list table's offline behavior is ruled under item 5's Data Gathering.
3. Ruled 2026-09-24 (by item 3's rewrite): remote deletion only happens with `--force-remote`, which is the caller's explicit acceptance; the status report printed first shows any commits that exist only on the remote branch.
4. Ruled 2026-09-24: "unique commits" is replaced by item 3's safety tiers (where the branch's last commit is found) and its lost-commit count.
5. Ruled 2026-09-24: the remove report uses the same default-branch target as the list table: the descendant tip, or `origin/<default>` when the tips diverge. The safety tiers inspect both refs independently.
6. Ruled 2026-09-24: remove it and move the shell to the fork parent's worktree, or the base repo (see item 3). Refuses when the shell wrapper is not active; wrapper detection is ruled under item 7. Windows is ruled too: move first, then remove, on every OS, with a lock check before `git worktree remove` on Windows (spike-confirmed on the Windows build host, 2026-09-24).
7. Ruled 2026-09-24: the dirty-file list uses kind-based coloring (source orange, other yellow), counts every modified, staged, and untracked path above 10, and replaces the old 50-file cap. Item 1's spacing is part of the new remove report.
8. Ruled 2026-09-24: the flag is `--from <base>`; `--from` with an existing branch is an error, as is a `--from` branch that does not exist (item 6).
9. Ruled 2026-09-24: L1 tests for the logic, L2 real-terminal tests for prompts and the shell wrappers; per-item acceptance criteria are under Acceptance criteria and testing.
10. Ruled 2026-09-24: one spec, kept in `worktree/fixes/`; the items reference each other too heavily to split.
11. Ruled 2026-09-24: non-interactive behavior follows item 3's tiers and `--force-*` flags (a dirty worktree without `--force-worktree` is an error and removes nothing; a Not safe branch without `--force-branch` is kept with a warning, exit 0).
12. Ruled 2026-09-24: the cross-package changes are folded into this spec and land fixes first (see Sequencing).
13. Ruled 2026-09-24: sniff's PR entry points cover all four providers (GitHub, GitLab, Gitea, Bitbucket) at launch.
14. Ruled 2026-09-24: the Branch column keeps its gray/red connector colors even though they repeat the `-> parent` column's answer; the redundancy is deliberate, for clarity.
15. Ruled 2026-09-24: the truncated note from the original draft ("NOTE: I do not remember") is deleted.
16. Ruled 2026-09-24: `GitGraph` works around the `mermaid-rs-renderer` attribute-parsing bug, and the bug is reported upstream against 0.3.1 as an issue plus a small PR, drafted for review before filing (item 5's Graph View).
17. Ruled 2026-09-24: the scale reference is 100% = diagram body text one terminal line tall, and `GitGraph` defaults to 125%. The base view's maximum height is ruled too; see item 5's Graph sizing.
18. Ruled 2026-09-24 (revised after review): ignored entries require consent like dirty files: interactive asks (default No), non-interactive needs `--force-worktree`, and the handoff fingerprint covers the ignored-entry set (item 3). The earlier "mention, don't block" ruling could delete an ignored `.env` or notes file without consent. Accepted cost: worktrees with build output such as `target/` prompt, and scripts pass `--force-worktree`. **Superseded 2026-09-25 by `2026-09-25-worktree-file`:** only new or changed files named by `.worktreeinclude` need consent.
19. Ruled 2026-09-24: `wt --completions <shell>` is the only source of the shell wrappers; `worktree/shell/` is deleted (item 7).

### Rulings from Phase 1

Phase 1 of `plan.md` recorded one recommendation per open question (R1–R11) and ran five spikes (`spike-s1.md` … `spike-s5.md` in this directory). The author confirmed all of them as written on 2026-09-25, with the amendment noted on Decision 21.

20. **R1, handoff token randomness** (ruled 2026-09-25): 128 bits from the OS CSPRNG through `getrandom` 0.4, encoded as 32 lowercase hex characters. `getrandom` 0.4 is already a workspace dependency (Darkmatter's `ctx.id`), so `worktree` adds no new crate to the graph; `docs/dependencies.md` gains a `worktree` row.
21. **R2, live remote-head check** (ruled 2026-09-25, amended by S2 and by reviews 3, 4, and 5): run `git -C <base repo> -c credential.interactive=never ls-remote origin refs/heads/<branch>` with `GIT_TERMINAL_PROMPT=0`, `GCM_INTERACTIVE=never`, and `GIT_SSH_COMMAND` set to the user's SSH command (`$GIT_SSH_COMMAND`, else `core.sshCommand`, else `ssh`) plus `-o BatchMode=yes`, under a 3 s deadline. At the deadline kill the whole process tree (a process group on Unix; `taskkill /T /F` on Windows, since a Job Object cannot be nested inside an SSH session's Job): S2 showed that killing `git` alone leaves `ssh`/`git-remote-https` holding the output pipe on every OS and, on Windows, leaves the real `git.exe` running and holding its working directory. A timeout or failure is "unavailable": the local branch is kept, and `--force-remote` exits 1 (Decisions 2). **Amendment (review 3):** `origin` is a mutable nickname, and `ls-remote origin` reads the fetch URL while deletion follows the push URL. The approval therefore records the resolved deletion endpoint; the live check and the deletion both use that endpoint, and the handoff refuses (exit 3, nothing removed) when it has changed. A configuration with separate or multiple push URLs has each destination checked, or is refused before anything is removed. **Amendment (review 4):** git rewrites a URL argument again (`ls-remote` by `insteadOf`, `push` by `pushInsteadOf`, else `insteadOf`), and no switch disables that for one command. The resolved endpoint is therefore used only when no configured `url.<base>.insteadOf` or `url.<base>.pushInsteadOf` value is a prefix of it; otherwise `--force-remote` refuses (exit 3, nothing removed), on the first run and on the handoff's second run alike. **Amendment (review 5):** git also reads a repository argument that names a configured remote as that remote, whose fetch and push URLs can differ, so a relative endpoint such as `approved` is not literal whenever a remote `approved` exists. The endpoint is therefore used only when, in addition, no `remote.<endpoint>.*` key exists in any configuration scope (including `-c`) and no legacy `$GIT_DIR/remotes/<endpoint>` or `$GIT_DIR/branches/<endpoint>` file exists; otherwise `--force-remote` refuses the same way, the report names the key or file that defines the colliding remote, and the deletion re-checks both conditions before pushing. `git remote get-url` is not used for this, since it ignores remotes defined outside the repository's own configuration.
22. **R3, PR deadline for `wt remove`** (ruled 2026-09-25): 2 s for `wt remove`'s per-branch request, 300 ms for `wt list`. This replaces item 5's "same deadline" wording for `wt remove`.
23. **R4, `--handoff` surface** (ruled 2026-09-25): `--handoff <token>` is hidden, conflicts with `<name>` and every `--force-*` flag (the approved choices come from the record), and `<name>` is optional only when `--handoff` is present.
24. **R5, failed rename-back in the Windows lock probe** (ruled 2026-09-25): retry the rename back three times, 50 ms apart; if it still fails, exit 1 naming both paths and the `move` command that restores the worktree, and never proceed to removal. S4 measured the probe at about 1.5 ms per rename pair and showed a held directory fails on the first rename with nothing changed, so this guards a narrow race.
25. **R6, "`CI` is set"** (ruled 2026-09-25): the variable is present and not empty.
26. **R7, exit-code plumbing** (ruled 2026-09-25): a CLI-side exit-code classification (0/1/3/4; clap keeps 2); `WorktreeError::RefusedToLoseWork` and `WorktreeError::BlockedByEnvironment`, each carrying its report; `Cancelled` maps to 0; `main.rs` owns the mapping; `wt go` without a wrapper returns `BlockedByEnvironment`.
27. **R8, abbreviated PR source-head SHAs** (ruled 2026-09-25, amended by S1): when the provider's SHA is shorter than the local tip's full object ID (40 for SHA-1, 64 for SHA-256), it counts only if it is at least 7 lowercase hex characters, the local tip starts with it, and it resolves to exactly one object in the local repository (`git rev-parse --disambiguate`). Otherwise the PR grants no Safe evidence. Only Bitbucket Cloud abbreviates (12 characters in practice). For Gitea and Forgejo, a merged PR's head SHA is taken from the list endpoint only, because the single-PR endpoint reports the live branch tip (S1).
28. **R9, where the new records live** (ruled 2026-09-25): beside the comparison cache, as sibling files of `cache::cache_path` (`<user cache>/worktree/<repo hash>.json`) keyed by the same repository hash, each with its own format version and written with `atomic_write`: `<repo hash>.fork-origins.json`, `<repo hash>.prs.json`, and one `<repo hash>.handoff-<token>.json` per pending handoff. The repository hash is taken from the main worktree's path so every worktree of a repository shares the records.
29. **R10, terminal rows for the graph height cap** (ruled 2026-09-25, confirmed by S5): use biscuit-terminal's existing size detection, which falls through stdout, stderr, and stdin, so it still works when the wrapper captures stdout. When every stream is redirected it reports 24 rows (cap about 12).
30. **R11, landing directory for a detached worktree** (ruled 2026-09-25): the base repo.
31. **Ignored-entry display** (ruled 2026-09-25, from S4): `git status --ignored=matching` lists a directory once only when a directory pattern matches it. This repository's own `**/target/*` lists `target/debug/`, `target/CACHEDIR.TAG`, and so on, and a `*.log` pattern lists every file. The handoff fingerprint hashes the full list; the report groups entries by their first path component (`target/ (6 entries)`) and applies the dirty list's 10-item cap and bold red total. Item 3's "e.g. `.env, notes.md, target/`" is then the grouped display, not the raw list.
32. **Graph natural width** (ruled 2026-09-25, correction from S5): the SVG's natural width comes from `mermaid-rs-renderer` 0.3.1's `measure_svg_dimensions` (`SvgDimensions::viewbox_width`), not `render_svg_with_dimensions`, which returns the SVG string. The measurement must use biscuit's theme, because layout depends on its font size. gitGraph branch labels use the theme's body size: 16 in the Default and Forest themes and 14 in Dark and Neutral; commit IDs are 10 in all. Item 5's and Cross-package dependencies' mentions of `render_svg_with_dimensions` read as this function.
33. **PowerShell wrapper encoding** (ruled 2026-09-25, from S3): the PowerShell wrapper sets `[Console]::OutputEncoding` to UTF-8 (no BOM) for the `wt` call and restores it afterwards. Windows PowerShell 5.1 otherwise decodes `wt`'s output with the console code page (IBM437 on the build host) and corrupts non-ASCII `cd:` paths.
34. **Graph in narrow terminals** (ruled 2026-09-25): the graph is shown at every terminal width. The old rule that hid it below 80 columns is removed, and a narrow terminal gets the smaller graph from item 5's Graph sizing (trim commits, then shrink).
