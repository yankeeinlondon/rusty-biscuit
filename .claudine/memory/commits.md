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
- `git commit -m "message" -- path` WITHOUT `--only` commits ALL staged files, not just the named paths. The paths after `--` are purely to disambiguate `-m` from a path argument; they do not scope the commit. Always use `--only` when committing specific staged files:
  - `git commit --only -m "message" -- path1 path2` — commits only path1 and path2
  - `git commit -m "message" -- path1 path2` — commits ALL staged files (the paths are informational only)
- Quote paths that contain spaces when passing them to `git commit`.
- Be careful with renames. When files are staged as renamed (R100), `git commit --only` with only the new path can produce incomplete commits that record only the deletion of the old path, not the addition of the new path. To preserve a rename atomically, either commit without path-limiting (let git infer the paths) or include both old and new paths explicitly.
  - **Recovery (rename commit only recorded deletions):** If a rename commit (R100) only recorded deletions of the old path (missing new file additions), the fix is:
    1. `git reset --soft HEAD~1` — this brings the deletion commit's tree into the index, which combines with any staged additions to restore proper R100 rename entries
    2. `git commit -m "message"` (no paths) — commit without path-limiting so git handles the rename atomically
  - **Recovery (blob extraction fallback):** Use `git ls-files --stage` to find blob hashes, `git show <hash> > path` to restore content, then `git add` and `git commit` normally.
  - **Better approach for multi-agent workflows:** Avoid using `--only` with renamed files. Instead, have the orchestrator stage files one group at a time and have subagents commit sequentially, rather than pre-staging all groups and using concurrent `--only` commits.
- `git commit --only -m "message" -- path` also works for a newly added file, as long as the file has already been staged.

## History and Verification

- `git log --oneline -n <count>` can miss a relevant commit either because the count is too small or because the commit is reachable on the branch but not on the current ancestry path. Increase `-n` or use `git log --all --oneline` when needed.
- Do not assume `HEAD` still points to your new commit in a concurrent workflow. Capture the commit hash from `git commit` output and verify that object directly.
- A commit may be reachable from a branch without being an ancestor of the current `HEAD`. Use `git branch --contains <commit>` when you need reachability, not just local history order.
- Lost commits can usually be found in `git reflog`. Treat reflog lookup as the recovery starting point instead of memorizing one rigid reset sequence.

## Multi-Agent Workflow

- **Verify all staged files are assigned before dispatching.** After organizing into semantic groups, cross-check that every staged file appears in exactly one group's file list. A missed file will silently remain staged after all subagents finish, requiring a follow-up dispatch. Use `git status --short` after all subagents report completion to confirm a clean index.
- Subagents may see a different staged set than the prompt implies. Always verify the actual index state before committing.
- In a shared worktree, concurrent agents share the same index. `git reset HEAD` without paths resets the entire staged set for everyone.
- **Orchestrator staging discipline:** When committing from the same worktree with multiple subagents, two strategies work:
  1. **Sequential:** Stage each group's files and have the subagent commit BEFORE staging the next group. This is the safest approach.
  2. **Concurrent with `--only`:** Pre-stage all files upfront, then launch concurrent subagents each using `git commit --only -m "..." -- <paths>`. Because `--only` limits the commit to only the named paths, it does NOT unstage other files — so concurrent commits from the same worktree can succeed safely when all agents use this form.
    - Without `--only`, concurrent commits are unsafe because `git commit path` commits the path AND removes it from the staging area, racing against other agents.
    - With `--only`, concurrent commits are safe because only the specified paths are committed; other staged files remain staged.
- If another worker already committed some assigned files, a later commit may legitimately report `nothing to commit`. That does not mean the earlier commit was missing.
- **Staged file overlap with concurrent agents:** When using concurrent subagents with pre-staged files, if ANY subagent uses `git commit` without `--only` (e.g., to handle renames atomically), it will commit ALL currently staged files, potentially including files assigned to other semantic groups. In this session, the kickoff docs commit (handling R100 renames without `--only`) inadvertently included the spec files that were semantically group 6. Either stage files one-group-at-a-time sequentially, or ensure no subagent needs to bypass `--only` for rename handling when running concurrent agents against the same pre-staged set.
- Auto-formatting workflows (e.g., rustfmt on save) may pre-commit files before an orchestrator assigns them. If a subagent finds no staged changes for an assigned file, it was likely auto-committed by a formatting hook.
- Commit message hooks may rewrite the submitted message after `git commit` returns. A subagent reported the resulting message differed from what was attempted. If message fidelity is critical, verify the actual commit with `git show <hash>` rather than trusting the submitted message text.
- **Concurrent `git commit` invocations can hit `.git/index.lock: File exists` (or `refs/heads/<branch>.lock`)** even when each uses `--only`. Git's locks are fail-fast, not queuing. This is not corruption — wait 1–3 seconds and retry the same `git commit --only ...` command. Up to 5 retries with short backoff is a reasonable budget. Always brief subagents on this retry policy when dispatching parallel commits.
- **`git log -1` after a concurrent commit may show a sibling subagent's commit, not yours.** Verify your own commit landed by capturing the hash from the `git commit` output and using `git show <hash>` or `git log --oneline -N` (with N large enough to span the parallel batch), not by assuming HEAD points at your work.
- **`--only` does not make concurrent commits safe when semantic groups have inter-file type dependencies.** The "concurrent with `--only`" rule above assumes each group's commit is self-contained at the source level. If group A introduces a new struct/field/method that group B's files reference, and both run concurrently with `--only`, the resulting commits become siblings sharing the same parent (the pre-A HEAD). Group B's tree has its new files referencing an identifier that lives only in A's commit (not in B's parent), so the commit does not compile and the history contains a broken tree. Detect this by scanning whether any group's files reference identifiers introduced by another group. When inter-group dependencies exist, run subagents sequentially — each commit's parent becomes the previous commit's HEAD, producing a clean linear chain with self-consistent trees at every step. Use `git commit --only` in each sequential step to keep the remaining staged files intact; do NOT unstage/restage between steps (the task explicitly forbids it, and the lessons above say not to "group commits by unstaging and restaging"). Dependents first, dependents-consumers after.
- **Merge-in-progress state blocks `--only` partial commits.** When the repo is in a merge state (conflicts resolved, merge not yet committed), `git commit --only` cannot do partial commits — Git requires a full merge commit to conclude the merge. Subagents should detect this with `git status` (look for "All conflicts fixed but are still committing") and either complete the merge first or stage files one-group-at-a-time sequentially. The orchestrator should verify no merge is in progress before launching concurrent subagents.

## Git Path Handling in Workspaces

- When the worktree is a Cargo workspace member (e.g., running from `darkmatter/darkmatter/`), git interprets relative paths as relative to the current working directory, not the repo root. Use paths relative to the workspace member directory when committing from within a package subdirectory, not paths from the repo root like `darkmatter/lib/src/...`.
- **Never `cd` out of the current git worktree to run inspection commands.** The wrapper has already placed you at the worktree root. Specifically, do NOT prefix `sniff repo` (or any other diagnostic) with `cd ~/.claudine/worktrees/<repo>/` — that path is the *parent* directory containing all linked worktrees of `<repo>` and is OUTSIDE the active worktree. OpenCode's permission engine treats it as `external_directory` and surfaces a permission ask (auto-allowed under `--dangerously-skip-permissions` but noise either way). All sniff and git subcommands are already worktree-aware; run them plainly from the inherited cwd.

## Shell Gotchas

- Literal backticks inside a double-quoted shell command trigger command substitution in `zsh`. If a commit message contains Markdown code spans, escape the backticks or build the message with a single-quoted heredoc first.
- When generating temporary files for use in shell commands (e.g., as input for `git commit -F`), they must be placed inside the project's temporary directory (`.gemini/tmp/...`) rather than the global `/tmp` directory, which may be outside the allowed workspace scope.
- Function names, identifiers with underscores, or code-like strings in commit messages can be interpreted as commands by zsh if they match shell functions or aliases. Prefer writing commit message bodies in prose that describes the feature rather than naming implementation details, or use single-quoted strings to prevent expansion.
- In scripts that run `git add .`, use `git add . || exit 0` to prevent CI failure when there is nothing to commit.
- In `zsh`, `$status` is a read-only special variable (alias of `$?`). When wrapping `git commit` in a retry loop, capture the exit code into a non-reserved name immediately after the command (e.g. `rc=$?`) — assigning to `status=$?` silently fails and breaks success/failure detection. Verify the result with `git show <hash>` rather than re-parsing captured stdout.
- For complex commit messages with bullet points and special characters (like backticks or underscores), some agents prefer committing with a simple placeholder message first and then using `git commit --amend` (or passing the message via a temporary file) to avoid shell expansion and command-injection false positives in the `-m` argument.
- **`git commit --amend` without `--only` also commits ALL staged files.** Like `git commit` without `--only`, `git commit --amend` replaces the current commit with whatever is currently staged, not just the files that were part of the original commit. To amend while preserving only some staged files, you must use `git commit --only --amend -m "message" -- <paths>` or stage files one-at-a-time sequentially before amending.

## Rust Idioms

- Prefer `sort_by_key` over `sort_by(|a, b| key(a).cmp(&key(b)))` for single-key sorts — it is more idiomatic and slightly more efficient.
- Prefer guard clauses (`if condition =>`) in pattern matches over nested `if` blocks inside match arms.

## Testing Terminal-Rendering CLIs

- When writing tests for CLI table output that uses terminal rendering, be aware that non-TTY test contexts default to 80-column width, which may be too narrow for some tables. Tests should gracefully accept the "could not be rendered" (or equivalent) error message when the terminal is too narrow, rather than only asserting on successful table render. This makes tests more robust across different CI environments and execution contexts.
