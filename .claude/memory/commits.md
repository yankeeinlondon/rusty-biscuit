# Commit Lessons Learned

## Never Reset or Rebase Commits

**Do NOT use `git reset` or `git rebase` in this monorepo worktree.** Staging and unstaging files can have unexpected consequences when developers are actively working on the codebase.

## Commit File Grouping

**Do NOT group commits by unstaging and restaging files one group at a time.** This risks worktree corruption and unexpected state changes from concurrent development.

Instead: commit file groups **explicitly** using `git commit --only -m "message" -- <file1> <file2> ...`.

The `--only` flag ensures git commits exactly the specified files, regardless of what else might be staged.

## Lock Contention

When multiple subagents commit in parallel against the same worktree, `git commit` can fail with:

```
fatal: Unable to create '.git/index.lock': File exists.
```

or the equivalent `refs/heads/<branch>.lock` variant. This is **not** corruption — git's locks are fail-fast, not queuing. Retry the same `git commit --only …` command after a 1–3 second backoff. Retry up to 5 times before giving up.

## Path Resolution in This Worktree

The git repo root is `/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter`. Staged files are specified relative to this root:

- Files in `darkmatter/` package area use `darkmatter/<path>` prefix (e.g., `darkmatter/lib/src/layout/page.rs`)
- Files outside the darkmatter package area (e.g., `prompts/`) are at the repo root and may require `../prompts/` prefix when the subagent's working directory differs from the git repo root

When in doubt, run `git status` to see the actual staged file paths and use those exact paths in `git commit --only`.

## Zsh Backtick Expansion in Commit Messages

Literal backticks inside a double-quoted `-m` argument trigger command substitution in `zsh`. When a commit message contains Markdown code spans (backticks), escape them or split the message across multiple `-m` flags:

```bash
# Wrong — backticks are interpreted as command substitution
git commit -m "feat(foo): add `bar` helper"

# Works — separate -m flags are concatenated by git
git commit -m "feat(foo): add" -m "`bar` helper"
```

The multi-flag approach avoids shell escaping entirely and keeps the message intact. Writing the full message (subject + blank line + bullets) to a temp file and feeding it via `-F /tmp/msg.md -- <paths>` is even safer — no shell evaluation runs against the body at all, and the file is reusable across retry attempts under lock contention. Keep the filename scoped (e.g. `/tmp/commit_msgs/<n>-<op>-<scope>.md`) so concurrent subagents do not overwrite each other.

## Atomic Multi-Site Contract Changes

A contract change that is enforced at multiple sites — e.g. parser rejection plus executor backstop plus bootstrap clearing plus runner substitution — must ship in **one** commit. Splitting the parser from the executor leaves a window where a programmatically built stack passes parsing but reaches the executor (or vice versa), violating the contract. The shell-free initialization ruling for `2026-09-15-initialize-after-proxy` is the canonical example: `parse_lifecycle_config` rejects `initialize` shell actions, `run_shell_action` short-circuits with `ShellRunError::BeforePreflight` for `LifecycleSignal::Initialize`, the lifecycle guard swaps in `DisabledShellRunner` until `start`, and the staged boot / harness boot drop the narrow-shell-approval calls. All 30 files (lib + cli + tests) landed in one `feat(claudine):` commit because any split would have left the contract half-enforced. The companion `docs(claudine):`, `chore(skills):`, `chore(darkmatter):`, `fix(prompts):`, and `planning(claudine):` commits are downstream documentation and planning artifacts — they describe the new contract but do not enforce it — so they are safe to ship in their own commits after the enforcement commit lands.

## `--only` Includes Untracked New Files

`git commit --only -- <path>` commits the working-tree content of `<path>` even when the file is untracked (status `??` in `git status` or `A` after staging sibling changes in the same batch). The staging it does to enable the commit is implicit — there is no need to `git add` first when the file is on disk with the desired content. This is useful for split-batch patterns where the developer staged an unrelated set of changes and you are committing a slice that includes new files (e.g. a new `implementation-log.md` alongside modified `spec.md` and `plan.md` for a planning close). Pair the staging-only-on-working-tree hint with a post-commit `git status --short -- <path>` to confirm the path was consumed; if `git show --name-status <hash>` lists the path with `A`, the working-tree content was what shipped.

## Spike Bundle Atomicity (markdown cross-ref + `#[path]` compile dep)

A `planning(<area>):` commit for a spike-driven feature must bundle spec + spike report + transition table + shared spike code (`features/<date>/spike/model.rs`, `scenarios.rs`) + the test fixtures (`lib/tests/<name>_spike.rs`) that consume the spike code via `#[path = "../../features/<date>/spike/model.rs"]`. The spike report describes the fixtures through markdown `[Fixture: path]` links (markdown cross-reference) AND the test fixtures import the spike code through `#[path]` attributes that fail at compile time when the spike code is missing (compile dependency). Both couplings force the bundle to ship together: splitting the spike code from its consumers leaves the test files unable to compile, and splitting the report from the fixtures leaves dangling markdown references. Cross-area test fixtures (e.g. `claudine/lib/tests/...` and `darkmatter/lib/tests/...` both consuming `darkmatter/features/.../spike/`) are normal for this pattern — keep the scope on the primary feature area (the spec's `area:` frontmatter) even though the diff crosses package areas. The `39135dd73` lifecycle-events commit (10 files, 2 package areas, 1989 insertions) is the canonical example.
