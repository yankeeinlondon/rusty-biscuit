# Knowledge Learned about Committing

> This document is a bulleted list of things learned about commits which may not be intuitive and obvious with just a knowledge of git and this repo.

- I am not responsible for running tests, this will already have been done before i'm handed the job of making git commits

- Do not stage additional files. The caller has already determined what files they want staged and adding more is a mistake! If there are NO files staged then simply exit with a message about no files being staged.

- When viewing commit history, use `git log --oneline -n` directly — `sniff git commits` is not a valid command in this repo

- The repo uses lowercase after the colon in conventional commits (e.g., `docs(darkmatter):` not `Docs(Darkmatter):`)

- When committing agent/skill files in `.claude/`, use `docs(<area>)` as the prefix for documentation restructuring changes

- Subagents may see a different set of staged files than what the user specifies in the prompt (due to concurrent work or a filtered list). When this happens, verify with `git status` and if more files are staged than assigned, use `git reset HEAD` to unstage everything, then `git add` only the assigned files. This is the only way to commit only specific files when other unrelated files are also staged. Never use `git reset *` (with a glob) as that can corrupt staged state. **Never use `git reset --hard`** — it wipes both the staging area AND working tree changes, destroying all uncommitted work. Use `git reset HEAD` (without `--hard`) to only unstage.

- When multiple related files are staged together (e.g., a directory rename like `transform/` → `compose/`), git commits them as an atomic unit. In such cases, subagents will not be able to split them into separate granular commits even if semantically distinct groups were planned - the files must be committed together as they were staged

- Using `git commit -- path/to/file` does NOT limit the commit to only those paths if other files are also staged. Git will commit ALL staged files. To commit only specific files, ensure ONLY those files are staged (not using `git add` broadly) or use `git commit -m "message" -- path1 path2` with explicit paths when you are certain only those files are staged

- When files are renamed and staged as renames (e.g., `old.md -> new.md`), specifying explicit paths to `git commit -m "..." -- new.md` only commits the "new" side; the "deleted" side remains staged even when rename detection is complete (R100). Always verify both sides of a rename are committed with `git diff --staged --name-status`

- When committing files in directories with spaces in their names (e.g., `darkmatter/features/2026-03-24. reference-validation/`), paths must be properly quoted when passed to `git commit -m "..." -- "path with spaces/file.md"`. Failure to quote results in git treating each space-separated token as a separate path argument.

- Do not second guess the files which were staged; the user intentionally chose which files they were interested in committing.

- Files can only be committed once. When multiple subagent groups are committing related files, later groups will get "nothing to commit" for files already committed by earlier groups, but can still commit any remaining files assigned to them.

- When using `git commit -m "message" -- path1 path2`, the `--` separator before paths combined with `-m` can cause git to source the commit message from a cached staged template instead of the inline message. To avoid this, place paths before `-m` (e.g., `git commit file1 file2 -m "message"`) or use `git commit --only -- path1 path2 -m "message"` for explicit path-limited commits with a custom message.

- When multiple subagents commit concurrently to the same branch, they may create overlapping or duplicate commits. The orchestrator can use `git reset --soft <before-subagents>` to restore all changes to staging, then re-commit them properly in separate atomic commits. Always verify the final state with `git log --oneline` and `git diff <parent> --stat` before assuming the job is complete.

- A commit can be "on" a branch (reachable from it via `git branch --contains`) but NOT an ancestor of the current HEAD. This happens when HEAD has moved forward after the branch diverged. Subagents using `git log --oneline -n` only see ancestry-path commits and will miss reachable-but-not-ancestor commits. To see all commits on a branch regardless of ancestry, use `git log --all --oneline | head -n` or `git branch -v --contains <commit>` to check if a specific commit is reachable.

- Lost commits can be recovered from the reflog. After `git reset` moves HEAD, the old commits remain in `git reflog` with timestamps. Use `git reflog | head -20` to find the hash, then `git cherry-pick -n <hash>` to replay the changes as unstaged working-tree modifications, then re-stage and commit properly. This is the rescue path when a subagent accidentally resets a commit that contained staged changes.

- When a subagent accidentally commits files it shouldn't (e.g., because `git reset HEAD` unstaged only some files while working-tree changes were simultaneously modified), the recovery sequence is: `git reset HEAD~1` to undo the bad commit, then `git checkout -- .` to restore the working tree to match the previous commit, then re-`git add` only the intended files. This restores both the staging area and working tree to a clean state before retrying.

- `git log --oneline -n` may not show a relevant commit if it's further back than `n`. Use a larger `-n` value or `--all` to ensure you see the full picture. A commit can exist on the branch but be deeper in history than a small `-n` count suggests.

- When subagents operate concurrently on the same branch, their working trees share the same staging area. A `git reset HEAD` by one subagent affects what all other subagents see as "staged." Each subagent should verify staging state independently before committing, and use explicit `git add` of assigned files rather than relying on pre-existing staged state.

- In this Claudine sandbox environment, a writable worktree can still have its git metadata stored outside the writable roots (for example under `/Volumes/.../.git/worktrees/...`). When that happens, `git commit` fails trying to create `index.lock` even though editing files in the worktree succeeds. Check the actual gitdir path before assuming commits are possible from the session.

- When multiple subagents commit concurrently from a shared staging area, `git reset HEAD` (without paths) resets the ENTIRE index, destroying all staged changes from ALL subagents. This means if Subagent A does `git reset HEAD` to isolate its files, Subagent B's staged changes are also wiped out. The orchestrator should NOT have subagents use `git reset HEAD` when operating concurrently — instead, each subagent should verify what is actually staged and work with that state, or the orchestrator should stage files for each subagent sequentially rather than having all subagents manipulate the shared index simultaneously.
