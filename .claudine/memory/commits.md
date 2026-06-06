---
description: A record of novel things learned about how to best perform commits and work with the conventional commits standard.
---
# Knowledge Learned about Committing

> Consolidated commit guidance for this repo and this multi-agent workflow. Keep this focused on rules that were actually observed or verified.

## Repo Conventions

- Commit exactly what the caller already staged. Do not stage extra files to make a commit "work," and do not unstage or second-guess the staged set — the caller chose it intentionally. If nothing is staged, stop and report it.
- Conventional commit subjects use lowercase after the colon, e.g. `docs(darkmatter): update parser notes`. For `.claude/` or skill-file restructuring, prefer `docs(<area>): ...`.
- View history with `git log` directly. `sniff git commits` is not a command in this repo.

## Inspect Before Committing

- `git status --short` shows staged / unstaged / untracked / renamed at a glance.
- `git diff --staged --stat` (change size per file) and `git diff --staged --name-status` (exact staged set) before grouping. Truncated diff output makes it easy to miss that a file has real changes.
- A staged file may have no actual diff (e.g. formatting that already matched). Committing it is harmless but unnecessary.
- New files show as a diff against `/dev/null` under `git diff --staged` — that is normal; confirm staging with `git status --short`.
- Review staged content with `git diff --staged` before committing — git will happily commit unresolved conflict markers.
- `git diff --staged --find-renames --name-status -- <source-path>` (pathspec restricted to only the source side) shows only `D` (deletion) entries for staged renames, **not** `R`. Rename detection reconstructs pairs only when *both* sides are in the diff set; restricting the pathspec to the source half hides the rename. To verify that a staged move is actually a rename (vs. a delete + add), run the unfiltered `git diff --staged --find-renames --name-status` or include both old and new paths in the pathspec. A subagent who only sees `D` here may incorrectly conclude a rename is a pure deletion.

## Path-Limited Commits

- Use `git commit --only -F - -- <paths>` to commit a specific staged subset. `--only` is **mandatory**: without it, `git commit -- <paths>` commits the ENTIRE staged set (the paths after `--` only disambiguate the message flag from pathspecs; they do not scope the commit).
- All options must precede the `--` pathspec separator.
- Feed the message on stdin via a **single-quoted heredoc** (`<<'COMMIT_MSG'`), never inline `-m "..."`. Commit bodies contain backticks, `$`, `_`, and code spans; inside a double-quoted string the shell expands them (backticks are command substitution even when double-quoted), corrupting the message and making OpenCode's snapshot layer try to `git add` the extracted tokens as pathspecs (`fatal: pathspec '::end-block' did not match any files`). The `'COMMIT_MSG'` delimiter must be single-quoted — that is what disables expansion.

  ```
  git commit --only -F - -- path1 path2 <<'COMMIT_MSG'
  refactor(darkmatter): scope block-quote support to ::shell-block

  - Add `quoted` field to stack entries
  - `::end-block` only closes matching quoted openers
  COMMIT_MSG
  ```

- Renames: `--only` with only the new path can record just the deletion of the old path. To keep a rename atomic, commit without path-limiting (let git infer both paths) or list old AND new paths explicitly. Do not mix `--only` and rename handling in the same concurrent batch (see Multi-Agent Workflow).

## A Successful Commit Is Final — Do Not Verify-Loop

This is the single biggest source of wasted time and outright hangs in this workflow.

- **A zero exit from `git commit` is authoritative.** Capture the hash from its output; at most run `git show <hash> --stat` once. Then STOP and report success.
- **Do NOT diff `HEAD:<file>` against the working-tree `<file>`.** When the caller staged a *partial* change, or a developer is still editing, the working tree legitimately differs from what you committed. A non-empty `git diff HEAD -- <file>` (or differing `wc -l`) is EXPECTED, not a failed commit. Treating it as failure sends the agent into an endless forensic loop — `reflog`, blob extraction, `branch --contains`, `.git/` spelunking — that burns the entire wall-clock budget. This is a confirmed cause of 15-minute hangs.
- **Do NOT trust `git log -1` / `HEAD` to confirm YOUR commit** in a concurrent batch — it may show a sibling's commit. Use the hash captured from your own `git commit` output. This is the *only* re-check you need, and only if a check is genuinely warranted.
- **`git reflog` and `.git/` inspection are last-resort recovery tools for an actually-reported failure, never a routine verification step.** If a commit genuinely fails, the evidence is the non-zero exit code and stderr from `git commit` itself — report THAT. Do not go hunting for proof that a *successful* commit "really" landed.

## Never Leave the Worktree (Hard Deadlock)

- **Never read, `ls`, `find`, or `cd` into any absolute path outside the active worktree root** — including `.git/worktrees/<name>/`, `~/.claudine/worktrees/<repo>/`, or the repo's parent directory. The wrapper already placed you at the worktree root; every `git` and `sniff` command is worktree-aware from there.
- Touching an external path triggers OpenCode's `external_directory: ask` permission. In a non-interactive session nobody can answer it, so the agent **blocks until the wall-clock timeout kills it**. A verification spiral that wanders into `.git/worktrees/…` to "check the index" is the exact path that produced a real 15-minute hang.
- Run diagnostics plainly from the inherited cwd: `git status`, `git log`, `sniff repo`. Never prefix with `cd <path>`.

## Multi-Agent Workflow

- **Assign every staged file to exactly one group before dispatching.** After all subagents report done, `git status --short` should show a clean index; a leftover file means a missed assignment and needs a follow-up dispatch.
- **Concurrent `--only` commits against one worktree are safe** — each commits only its own paths and leaves the rest staged. But if any subagent commits WITHOUT `--only` (e.g. to handle a rename atomically), it sweeps in ALL currently-staged files, stealing other groups' files. Keep every subagent on `--only`, or stage one group at a time sequentially.
- **Lock contention is not corruption.** Concurrent `git commit` can fail fast with `.git/index.lock: File exists` (or `refs/heads/<branch>.lock`). Git's locks are fail-fast, not queuing — wait 1–3s and retry the same command, up to ~5 times with short backoff. Brief subagents on this policy.
- **A merge in progress blocks `--only` partial commits** (git requires a full merge commit). Detect with `git status` ("All conflicts fixed but are still committing"); the orchestrator should confirm no merge is in progress before launching concurrent subagents.
- Auto-format-on-save hooks may commit or alter an assigned file before a subagent acts, so "nothing to commit" can be legitimate. Commit-message hooks may rewrite the submitted message after `git commit` returns.

## Shell Gotchas

- Use a single-quoted heredoc (`<<'COMMIT_MSG'`) for any message containing backticks, `$`, `_`, or code-like identifiers — otherwise zsh triggers command substitution or alias expansion on them.
- In zsh, `$status` is read-only (an alias of `$?`). In a retry loop, capture the exit code immediately into another name (`rc=$?`); `status=$?` silently fails and breaks success/failure detection.
- Temp files for `git commit -F` must live inside the workspace temp dir, not global `/tmp`, which may be outside the allowed scope.

## Rust Idioms

- Prefer `sort_by_key` over `sort_by(|a, b| key(a).cmp(&key(b)))` for single-key sorts.
- Prefer guard clauses (`if condition =>`) in match arms over nested `if` blocks.

## Testing Terminal-Rendering CLIs

- Non-TTY test contexts default to 80-column width, which can be too narrow for some tables. Tests should also accept the "could not be rendered" (or equivalent) error rather than only asserting a successful render, so they stay robust across CI environments.
