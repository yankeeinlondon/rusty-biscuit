---
description: A record of novel things learned about how to best perform commits and work with the conventional commits standard.
---
# Knowledge Learned about Committing

> Consolidated commit guidance for this repo and this multi-agent workflow. Keep this focused on rules that were actually observed or verified.

## Repo Conventions

- Commit exactly what the caller already staged. Do not stage extra files to make a commit "work," and do not unstage or second-guess the staged set — the caller chose it intentionally. If nothing is staged, stop and report it.
- Conventional commit subjects use lowercase after the colon, e.g. `docs(darkmatter): update parser notes`. For `.claude/` or skill-file restructuring, prefer `docs(<area>): ...`.
- View history with `git log` directly. `sniff git commits` is not a command in this repo.
- **Misfiled planning artifacts are deleted, not moved to `_completed/`.** When a `plan.md`/`spec.md` lives in a package area's `features/` or `fixes/` directory but its frontmatter `area:` field names a *different* package (e.g. a `biscuit-test-harness` plan sitting under `renderable/fixes/`), clean it up as a single-file deletion with a `chore(<host-area>): remove misfiled <area> <artifact>` message — do NOT move it into the host area's `_completed/`, because that would canonize the misfile. The `_completed/` move is reserved for plans/specs whose `area:` matches their directory.

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
  - **Recovery (rename commit only recorded deletions):** `git reset --soft HEAD~1` brings the deletion commit's tree into the index, where it combines with any staged additions to restore proper R100 rename entries; then `git commit` (no paths) so git handles the rename atomically.
  - **Recovery (blob extraction fallback):** `git ls-files --stage` to find blob hashes, `git show <hash> > path` to restore content, then `git add` and `git commit` normally.

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
- **`git show --stat <hash>` is the cleanest post-commit verification of the file set.** After a `--only` commit in a multi-agent worktree, run `git show --stat <hash>` and compare the file list / line counts against the pre-commit `git diff --staged --stat` for the assigned paths — if a sibling subagent's files leaked in, the stat will not match. This is faster and less error-prone than re-running `git diff` or parsing the `git commit` output.
- **`--only` does not make concurrent commits safe when semantic groups have inter-file type dependencies.** The "concurrent with `--only`" rule above assumes each group's commit is self-contained at the source level. If group A introduces a new struct/field/method that group B's files reference, and both run concurrently with `--only`, the resulting commits become siblings sharing the same parent (the pre-A HEAD). Group B's tree has its new files referencing an identifier that lives only in A's commit (not in B's parent), so the commit does not compile and the history contains a broken tree. Detect this by scanning whether any group's files reference identifiers introduced by another group — including **new dev-dependency entries in `Cargo.toml`/`package.json`**, which make downstream test files compile. When inter-group dependencies exist, run subagents sequentially — each commit's parent becomes the previous commit's HEAD, producing a clean linear chain with self-consistent trees at every step. Use `git commit --only` in each sequential step to keep the remaining staged files intact; do NOT unstage/restage between steps (the task explicitly forbids it, and the lessons above say not to "group commits by unstaging and restaging"). Dependents first, dependents-consumers after.
- **Merge-in-progress state blocks `--only` partial commits.** When the repo is in a merge state (conflicts resolved, merge not yet committed), `git commit --only` cannot do partial commits — Git requires a full merge commit to conclude the merge. Subagents should detect this with `git status` (look for "All conflicts fixed but are still committing") and either complete the merge first or stage files one-group-at-a-time sequentially. The orchestrator should verify no merge is in progress before launching concurrent subagents.
- **A single file's diff can span multiple plan phases and cannot be split across commits.** When the staged set was produced by squashing several plan phases (e.g. one file touches Phase 1, Phase 2, and Phase 5 of a feature), the file must be committed as a single unit. `--only -- <path>` always commits the full current content for that path — calling it twice commits the same file content twice. Splitting the underlying logical concerns would require `git add`/`git reset` to restage subsets, which is forbidden in this workflow. Organize commit groups by what is *splittable* in the actual staged set, not by ideal logical granularity, and accept that some commits will bundle several phases' worth of work on a single file.
- **A file with both staged and unstaged changes (`MM` in `git status --short`) commits only the staged portion under `git commit --only`.** When a developer (or an auto-format hook) is actively editing a file while you are committing, the file may show as `MM` (the staged region reflects an earlier snapshot, the working tree has since drifted). `git commit --only -- <path>` is safe in this state: it captures only the staged diff for the file, leaving the working-tree modifications untouched for the next agent or follow-up commit. Always verify with `git show --stat <hash>` that the committed line range matches the *pre-commit* `git diff --staged --stat` for the path, not the working-tree state. If the two diverge by exactly the unstaged portion, you committed the right thing.
- **The pre-commit `git diff --staged --stat` figure is a snapshot, not a contract.** In a multi-agent worktree the staged blob for a path can drift between the orchestrator's read and the subagent's commit: a developer editing the same path, an auto-format hook re-staging, or a sibling subagent that touched a shared file can all advance the staged snapshot. The subagent's prompt-side stat reflects the state at the moment the prompt was written — not at commit time. `git show --stat <hash>` is the only authoritative post-commit check. If the committed stat differs from the prompt's stat, the commit captured whatever was staged at commit time; the prompt's stat was simply out of date. Flag the drift to the orchestrator rather than treating it as a verification failure.
- **A `mod foo;` declaration in a parent module file forces that parent file to be paired with the new child submodule's commit.** When a new submodule is created, the `mod foo;` line that wires it in must land in the same commit, otherwise the build breaks (`mod foo;` references a non-existent module). In Rust, the parent module file (often `mod.rs`) often accumulates multiple concerns — re-exports of pre-existing sibling modules, a new `mod inherit;` line, and a new `pub use inherit::Foo;` re-export. The parent file has to be paired with whichever group introduces the new submodule (so the `mod` declaration lands with the new file), and other concerns landing in the same commit is acceptable. Plan the assignment of `mod.rs`-style files by asking: "which group introduces the new submodule this file references?" — pair the parent with that group, not with the group that defined the types the parent re-exports.

## Git Path Handling in Workspaces

- When the worktree is a Cargo workspace member (e.g., running from `darkmatter/darkmatter/`), git interprets relative paths as relative to the current working directory, not the repo root. Use paths relative to the workspace member directory when committing from within a package subdirectory, not paths from the repo root like `darkmatter/lib/src/...`.
- **Never `cd` out of the current git worktree to run inspection commands.** The wrapper has already placed you at the worktree root. Specifically, do NOT prefix `sniff repo` (or any other diagnostic) with `cd ~/.claudine/worktrees/<repo>/` — that path is the *parent* directory containing all linked worktrees of `<repo>` and is OUTSIDE the active worktree. OpenCode's permission engine treats it as `external_directory` and surfaces a permission ask (auto-allowed under `--dangerously-skip-permissions` but noise either way). All sniff and git subcommands are already worktree-aware; run them plainly from the inherited cwd.

## Shell Gotchas

- Use a single-quoted heredoc (`<<'COMMIT_MSG'`) for any message containing backticks, `$`, `_`, or code-like identifiers — otherwise zsh triggers command substitution or alias expansion on them.
- In zsh, `$status` is read-only (an alias of `$?`). In a retry loop, capture the exit code immediately into another name (`rc=$?`); `status=$?` silently fails and breaks success/failure detection.
- Temp files for `git commit -F` must live inside the workspace temp dir, not global `/tmp`, which may be outside the allowed scope.
- **`git commit --amend` without `--only` also commits ALL staged files** — it replaces the current commit with whatever is currently staged, not just the files originally in it. To amend while preserving only some staged files, use `git commit --only --amend -F - -- <paths>` or stage files one-at-a-time before amending.

## Rust Idioms

- Prefer `sort_by_key` over `sort_by(|a, b| key(a).cmp(&key(b)))` for single-key sorts.
- Prefer guard clauses (`if condition =>`) in match arms over nested `if` blocks.

## Testing Terminal-Rendering CLIs

- Non-TTY test contexts default to 80-column width, which can be too narrow for some tables. Tests should also accept the "could not be rendered" (or equivalent) error rather than only asserting a successful render, so they stay robust across CI environments.
- For biscuit-terminal table layouts that switch between single and split (two-column) views based on terminal dimensions, the established pattern in `biscuit-icon/cli/tests/cli.rs` is to drive a deterministic `Terminal` by passing `BISCUIT_TERM_WIDTH` and `BISCUIT_TERM_HEIGHT` env vars to the child process (parsed as `u32` inside the CLI), and to detect the layout by counting or scanning table-separator lines in the rendered output (e.g. `stdout.lines().filter(|l| l.starts_with('├')).count()` for the number of tables, or `any(|l| l.contains('┤') && l.contains('├'))` for the split case). This avoids snapshotting unstable ANSI styling while still asserting the layout branch.
