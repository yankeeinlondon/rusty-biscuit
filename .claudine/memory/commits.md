---
description: A record of novel things learned about how to best perform commits and work with the conventional commits standard.
---
# Knowledge Learned about Committing

> Consolidated commit guidance for this repo and this multi-agent workflow. Keep this focused on rules that were actually observed or verified.

## Repo Conventions

- Do not stage additional files just to make a commit work. Commit exactly what the caller already staged. If nothing is staged, stop and report that.
- Do not second-guess the staged set. The caller chose those files intentionally.
- When viewing history, use `git log` directly. `sniff git commits` is not a valid command in this repo.
- Conventional commit scopes and subjects in this repo use lowercase after the colon, for example `docs(darkmatter): update parser notes`.
- For documentation restructuring in `.claude/` or skill files, prefer a `docs(<area>): ...` prefix.

## Check What Will Be Committed

- Use `git status --short` first to see what is staged, unstaged, untracked, or renamed.
- Use `git diff --staged --name-status` to confirm the exact staged file set before committing.
- Use `git diff --staged --stat` to see the change size for each file before organizing groups. When diff output is truncated, it is easy to miss that certain files have actual changes.
- Review staged source changes with `git diff --staged` before committing. Git will happily commit unresolved conflict markers if they are present in the staged content.
- For new files, `git diff --staged` shows the file as a diff against `/dev/null`. That is normal; confirm the staging state with `git status --short`.
- A file may appear in the staged list but have no actual content changes (e.g., auto-formatting that matched existing formatting). If `git diff --staged` shows no diff for a file, it has no actual changes and committing it is harmless but unnecessary.

## Path-Limited Commits

- `git commit --only -m "message" -- path1 path2` is the recommended form when committing a specific subset of staged files. Always use `--only` to explicitly limit the commit to only the named paths.
- All options must come before the `--` pathspec separator. `git commit --only -- path -m "message"` is wrong because `-m` is then parsed as a path.
- For committing a single file with path-limiting, use `git commit -m "message" -- path` without `--only`. The `--only` flag is mutually exclusive with a single pathspec argument in git.
- Quote paths that contain spaces when passing them to `git commit`.
- Be careful with renames. Committing only the new path records an add and leaves the delete staged. Committing only the old path records the delete but leaves the new file staged as an add. To preserve a rename atomically, either commit without path-limiting (let git infer the paths) or include both old and new paths explicitly.
- `git commit --only -m "message" -- path` also works for a newly added file, as long as the file has already been staged.

## History and Verification

- `git log --oneline -n <count>` can miss a relevant commit either because the count is too small or because the commit is reachable on the branch but not on the current ancestry path. Increase `-n` or use `git log --all --oneline` when needed.
- Do not assume `HEAD` still points to your new commit in a concurrent workflow. Capture the commit hash from `git commit` output and verify that object directly.
- A commit may be reachable from a branch without being an ancestor of the current `HEAD`. Use `git branch --contains <commit>` when you need reachability, not just local history order.
- Lost commits can usually be found in `git reflog`. Treat reflog lookup as the recovery starting point instead of memorizing one rigid reset sequence.

## Multi-Agent Workflow

- Subagents may see a different staged set than the prompt implies. Always verify the actual index state before committing.
- In a shared worktree, concurrent agents share the same index. `git reset HEAD` without paths resets the entire staged set for everyone.
- **Orchestrator staging discipline:** When committing from the same worktree with multiple subagents, two strategies work:
  1. **Sequential:** Stage each group's files and have the subagent commit BEFORE staging the next group. This is the safest approach.
  2. **Concurrent with `--only`:** Pre-stage all files upfront, then launch concurrent subagents each using `git commit --only -m "..." -- <paths>`. Because `--only` limits the commit to only the named paths, it does NOT unstage other files — so concurrent commits from the same worktree can succeed safely when all agents use this form.
    - Without `--only`, concurrent commits are unsafe because `git commit path` commits the path AND removes it from the staging area, racing against other agents.
    - With `--only`, concurrent commits are safe because only the specified paths are committed; other staged files remain staged.
- If another worker already committed some assigned files, a later commit may legitimately report `nothing to commit`. That does not mean the earlier commit was missing.
- Auto-formatting workflows (e.g., rustfmt on save) may pre-commit files before an orchestrator assigns them. If a subagent finds no staged changes for an assigned file, it was likely auto-committed by a formatting hook.

## Shell Gotchas

- Literal backticks inside a double-quoted shell command trigger command substitution in `zsh`. If a commit message contains Markdown code spans, escape the backticks or build the message with a single-quoted heredoc first.
- Function names, identifiers with underscores, or code-like strings in commit messages can be interpreted as commands by zsh if they match shell functions or aliases. Prefer writing commit message bodies in prose that describes the feature rather than naming implementation details, or use single-quoted strings to prevent expansion.
- In scripts that run `git add .`, use `git add . || exit 0` to prevent CI failure when there is nothing to commit.

## Rust Idioms

- Prefer `sort_by_key` over `sort_by(|a, b| key(a).cmp(&key(b)))` for single-key sorts — it is more idiomatic and slightly more efficient.
- Prefer guard clauses (`if condition =>`) in pattern matches over nested `if` blocks inside match arms.
