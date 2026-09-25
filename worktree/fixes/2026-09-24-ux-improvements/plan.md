---
total_phases: 6
created: 2026-09-24
phase: 4
agent: claude/opus
yolo: false
source_files_during_phase_1: []
docs_updated_during_phase_1:
    - worktree/fixes/2026-09-24-ux-improvements/spec.md
    - worktree/fixes/2026-09-24-ux-improvements/plan.md
docs_created_during_phase_1:
    - worktree/fixes/2026-09-24-ux-improvements/spike-s1.md
    - worktree/fixes/2026-09-24-ux-improvements/spike-s2.md
    - worktree/fixes/2026-09-24-ux-improvements/spike-s3.md
    - worktree/fixes/2026-09-24-ux-improvements/spike-s4.md
    - worktree/fixes/2026-09-24-ux-improvements/spike-s5.md
    - worktree/fixes/2026-09-24-ux-improvements/implementation-log.md
skills_files_updated_during_phase_1:
    - .claude/skills/os/windows.md
    - .claude/skills/os/build-hosts.md
source_files_during_phase_2:
    - Cargo.lock
    - worktree/lib/src/error.rs
    - worktree/lib/src/lib.rs
    - worktree/lib/src/cache.rs
    - worktree/lib/src/worktree.rs
    - worktree/lib/src/fork_origin.rs
    - worktree/cli/Cargo.toml
    - worktree/cli/src/main.rs
    - worktree/cli/src/lib.rs
    - worktree/cli/src/args.rs
    - worktree/cli/src/env.rs
    - worktree/cli/src/exit.rs
    - worktree/cli/src/shell_integration.rs
    - worktree/cli/src/commands/go.rs
    - worktree/cli/src/commands/create.rs
    - worktree/cli/src/commands/remove.rs
    - worktree/cli/tests/wrapper_protocol.rs
    - worktree/cli/tests/shell_wrapper_exec.rs
    - worktree/cli/tests/snapshots/wrapper_protocol__bash.snap
    - worktree/cli/tests/snapshots/wrapper_protocol__zsh.snap
    - worktree/cli/tests/snapshots/wrapper_protocol__fish.snap
    - worktree/cli/tests/snapshots/wrapper_protocol__powershell.snap
    - worktree/shell/wt.sh (deleted)
    - worktree/shell/wt.fish (deleted)
    - sniff/lib/src/remote/blocking.rs
    - sniff/lib/src/remote/types.rs
    - sniff/lib/src/remote/focused.rs
    - sniff/lib/src/remote/github.rs
    - sniff/lib/src/remote/gitlab.rs
    - sniff/lib/src/remote/gitea.rs
    - sniff/lib/src/remote/bitbucket.rs
    - sniff/lib/src/remote/provider.rs
    - sniff/lib/src/remote/mod.rs
    - sniff/lib/tests/l1/main.rs
    - sniff/lib/tests/l1/pr_for_branch.rs
    - sniff/cli/src/output/remote.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/pull_requests.rs
docs_updated_during_phase_2:
    - worktree/README.md
    - docs/dependencies.md
    - sniff/lib/README.md
    - sniff/lib/CHANGELOG.md
    - worktree/fixes/2026-09-24-ux-improvements/plan.md
    - worktree/fixes/2026-09-24-ux-improvements/implementation-log.md
    - worktree/fixes/2026-09-24-ux-improvements/spec.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/worktree/SKILL.md
    - .claude/skills/sniff/SKILL.md
    - .claude/skills/sniff/remote-and-repository.md
source_files_during_phase_3:
    - Cargo.lock
    - worktree/lib/Cargo.toml
    - worktree/lib/src/lib.rs
    - worktree/lib/src/error.rs
    - worktree/lib/src/git.rs
    - worktree/lib/src/worktree.rs
    - worktree/lib/src/default_target.rs
    - worktree/lib/src/remove/mod.rs
    - worktree/lib/src/remove/inventory.rs
    - worktree/lib/src/remove/safety.rs
    - worktree/lib/src/remove/live_remote.rs
    - worktree/lib/src/remove/remote.rs
    - worktree/lib/src/remove/handoff.rs
    - worktree/lib/src/remove/test_support.rs
    - worktree/cli/Cargo.toml
    - worktree/cli/src/args.rs
    - worktree/cli/src/main.rs
    - worktree/cli/src/exit.rs
    - worktree/cli/src/commands/mod.rs
    - worktree/cli/src/commands/dirty_tree.rs
    - worktree/cli/src/commands/remove.rs (moved to remove/mod.rs)
    - worktree/cli/src/commands/remove/mod.rs
    - worktree/cli/src/commands/remove/policy.rs
    - worktree/cli/src/commands/remove/report.rs
    - worktree/cli/tests/remove.rs
    - worktree/cli/tests/level2_remove.rs
    - worktree/cli/tests/level2_dirty_tree.rs
    - worktree/cli/tests/powershell_wrapper_exec.rs
docs_updated_during_phase_3:
    - worktree/README.md
    - docs/dependencies.md
    - worktree/fixes/2026-09-24-ux-improvements/plan.md
    - worktree/fixes/2026-09-24-ux-improvements/implementation-log.md
    - worktree/fixes/2026-09-24-ux-improvements/spec.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/worktree/SKILL.md
    - .claude/skills/os/windows.md
source_files_during_phase_4:
    - Cargo.lock
    - biscuit-visualized/src/Cargo.toml
    - biscuit-visualized/src/src/mermaid/mod.rs
    - biscuit-visualized/src/src/mermaid/render.rs
    - biscuit-visualized/src/src/tests/mermaid_tests.rs
    - biscuit-terminal/lib/src/components/git_graph.rs
    - biscuit-terminal/lib/src/components/git_graph/tests.rs
    - biscuit-terminal/lib/src/components/mod.rs
    - biscuit-terminal/lib/src/components/mermaid.rs
    - biscuit-terminal/lib/src/components/table/table.rs
    - biscuit-terminal/lib/src/components/table/types.rs
    - biscuit-terminal/lib/src/components/terminal_image/iterm.rs
    - biscuit-terminal/lib/src/components/terminal_image/kitty.rs
    - biscuit-terminal/lib/src/components/terminal_image/mod.rs
    - biscuit-terminal/lib/src/components/terminal_image/protocol.rs
    - biscuit-terminal/lib/src/components/terminal_image/tests.rs
    - biscuit-terminal/lib/src/components/terminal_image/width.rs
    - biscuit-terminal/lib/src/discovery/fonts/types.rs
    - biscuit-terminal/lib/src/prelude.rs
    - biscuit-terminal/lib/src/render_tree/render.rs
    - biscuit-terminal/lib/src/terminal.rs
    - biscuit-terminal/lib/tests/l1/table_parity.rs
    - biscuit-terminal/lib/tests/l1/snapshots/l1__table_parity__table_highlight_row_with_striping_snapshot.snap
    - biscuit-terminal/cli/src/commands/shared.rs
    - renderable/src/tree/attrs.rs
    - renderable/src/tree/mod.rs
    - sniff/lib/src/remote/blocking.rs
    - sniff/lib/src/remote/focused.rs
    - sniff/lib/tests/l1/main.rs
    - sniff/lib/tests/l1/open_pull_requests.rs
    - sniff/lib/tests/l1/pr_for_branch.rs
    - worktree/cli/src/commands/list.rs
docs_updated_during_phase_4:
    - docs/dependencies.md
    - biscuit-terminal/docs/data-visualization/visualizing-graph-expressions.md
    - biscuit-terminal/docs/components/index.md
    - biscuit-terminal/docs/components/mermaid_diagram.md
    - biscuit-terminal/docs/components/table.md
    - biscuit-terminal/docs/components/terminal_image.md
    - biscuit-terminal/lib/src/components/table/README.md
    - biscuit-terminal/cli/README.md
    - sniff/lib/README.md
    - sniff/lib/CHANGELOG.md
    - worktree/fixes/2026-09-24-ux-improvements/plan.md
    - worktree/fixes/2026-09-24-ux-improvements/implementation-log.md
    - worktree/fixes/2026-09-24-ux-improvements/spec.md
docs_created_during_phase_4:
    - biscuit-terminal/docs/components/git_graph.md
    - worktree/fixes/2026-09-24-ux-improvements/upstream-issue.md
    - worktree/fixes/2026-09-24-ux-improvements/upstream-pr.patch
skills_files_updated_during_phase_4:
    - .claude/skills/biscuit-terminal/components.md
    - .claude/skills/biscuit-terminal/image-rendering.md
    - .claude/skills/renderable/tree.md
    - .claude/skills/sniff/SKILL.md
    - .claude/skills/sniff/remote-and-repository.md
    - .claude/skills/os/macos.md
    - .claude/skills/os/SKILL.md
    - .claude/skills/os/build-hosts.md
packages:
    - worktree
    - worktree-cli
    - sniff
    - sniff-cli
    - darkmatter
    - biscuit-visualized
    - biscuit-terminal
    - biscuit-terminal-cli
    - renderable
---

# Plan: Worktree UX improvements

Implements `2026-09-24-ux-improvements` (`spec.md` in this directory). Item numbers (item 1 … item 7) refer to that spec.

## Summary and Success Criteria

### The work

The spec touches four package areas. The work falls into three bodies that land in the ruled order (fixes first):

1. **Fixes (worktree, worktree-cli, sniff)**
    - Wrapper detection through `WT_SHELL_WRAPPER=1`, and a PowerShell wrapper (item 7).
    - A resolution contract that is aware of ambiguity (item 2).
    - `wt create --from` with a new fork-origin record (item 6).
    - The rewritten `wt remove` (items 1, 3, 4): safety tiers, `--force-worktree`, `--force-branch` and `--force-remote`, the report-first flow, move-first removal through a handoff token, the Windows lock check, and exit codes 0–4.
    - sniff's blocking entry point that returns the PR for one branch.
2. **Dependencies for the list and graph (biscuit-visualized, biscuit-terminal, sniff)**
    - The `mermaid-rs-renderer` 0.3.1 upgrade.
    - A per-row highlight on `Table`.
    - `ImageWidth::Scale`.
    - A new `GitGraph` component.
    - sniff's blocking entry point that returns a repository's open PRs.
3. **List and graph (item 5)**
    - The redesigned `wt list` table: caption, fork tree, target columns, badges, legend, and row emphasis.
    - The PR deadline and freshness cache, and the generalized comparison cache.
    - Graph data gathering handed to `GitGraph`. `default_graph_width` is deleted.

### Starting state (verified against the tree on 2026-09-24)

| Fact | Where | Consequence |
|---|---|---|
| `find_worktree` returns the first match, and `worktree_names` offers a branch name *or* a directory name | `worktree/lib/src/worktree.rs:544`, `:576` | Item 2 rewrites both |
| `remove` uses the `-f` count matrix, `FORCE_BYPASS_FILE_LIMIT`, and `git branch -d` through `delete_branch` / `DeleteBranchOutcome` | `worktree/cli/src/commands/remove.rs`, `worktree/lib/src/worktree.rs:603-640` | Replaced wholesale |
| The wrapper counts as active when `stdout` is not a TTY; `wt go` without a wrapper exits 0 | `worktree/cli/src/commands/go.rs:71` | Item 7 |
| Every error exits 1, including `WorktreeError::Cancelled` | `worktree/cli/src/main.rs:17-22` | An exit-code mapping is needed |
| The wrappers are written inline in `print_completions` (bash, zsh, fish), and hand-written copies exist in `worktree/shell/` | `worktree/cli/src/main.rs:52`, `worktree/shell/` | Item 7 |
| **No fork-origin record exists yet** | — | Built new in Phase 2 |
| The cache key is `(default_tip, branch_tip, CACHE_FORMAT_VERSION = 1)` | `worktree/lib/src/cache.rs:18` | Generalized in Phase 5 |
| `PullRequestInfo` lacks the source repository and the source head SHA | `sniff/lib/src/remote/types.rs:259` | Phase 2 extends it |
| sniff has no blocking wrapper; `tokio` (`rt`) is optional behind `network` | `sniff/lib/Cargo.toml:50,60` | Phase 2 |
| `Table` has striping but no per-row highlight | `biscuit-terminal/lib/src/components/table/table.rs` | Phase 4 |
| `ImageWidth` offers `Fill`, `Percent`, and `Characters` | `biscuit-terminal/lib/src/components/terminal_image/width.rs:44` | Phase 4 |
| `MermaidDiagram` implements Terminal and Tree rendering but not `BrowserRenderable` | `biscuit-terminal/lib/src/components/mermaid.rs:628,658` | `GitGraph` must supply browser output itself |
| `mermaid-rs-renderer = "0.2"` and `resvg = "0.45"` | `biscuit-visualized/src/Cargo.toml:20,24` | Phase 4 |
| `default_graph_width` and Mermaid text are built in the CLI | `worktree/cli/src/commands/list.rs:257`, `git_graph.rs` | Moved into `GitGraph` |

### Definition of done

The spec's "Acceptance criteria and testing" section is the contract. Concretely:

- [ ] Each of the spec's seven acceptance criteria has at least one named test that proves it:
    - L1 in the owning package.
    - L2 through `biscuit-test-harness`, never taking window focus.
- [ ] `just test`, `just test-l2`, and `just lint` pass in `worktree`, `biscuit-terminal`, `biscuit-visualized`, and `sniff`.
- [ ] `list gather` stays within the ratified targets in `worktree/docs/performance-testing.md` (warm 120 ms, cold 300 ms, full non-image `wt list` 1 s), including with the network down and a PR request that hits its deadline.
- [ ] The following are either green or recorded as **unmet with provisioning as the required change**. They are never narrowed.
    - Every criterion on macOS, Linux, native Windows, and WSL2 (see the `os` skill).
    - The Windows L2 cells: PowerShell move-first and the held-directory exit 4.
- [ ] No references remain to `-b`, `-f`/`-ff`, `FORCE_BYPASS_FILE_LIMIT`, `DeleteBranchOutcome`, `default_graph_width`, or `worktree/shell/`. The completed-spec archives are the exception.
- [ ] Docs are updated with the behavior:
    - `worktree/README.md`, `worktree/docs/cli/*`, `worktree/docs/git-graph.md`, and the `AFTER_HELP` text.
    - `.claude/skills/worktree/SKILL.md`, whose cache-key sentence changes.
    - The `os` skill, for new Windows findings.
    - `docs/dependencies.md` for any added crate.
    - The biscuit-terminal docs for `Table`, `ImageWidth`, and `GitGraph`.
- [ ] The upstream `mermaid-rs-renderer` issue and PR are **drafted for the author's review**, not filed.
- [ ] The spec is left for the author to close. No agent moves it to `_completed` or runs `just complete`.

## Phase 1 — Rulings and risk-reducing spikes

Goal: every open question has a ruling and the riskiest external behaviors have evidence before any production code changes.

### Necessary Rules

These points are unclear or unstated in the spec. Each has a recommended answer; the author confirms or overrides it before Phase 2 starts, and the ruling is written back into `spec.md` under Decisions.

- [x] **R1 — The handoff token's randomness source.**
    - "Random, hard to guess" needs an OS CSPRNG. `biscuit-hash` is a hash, not a random source.
    - *Recommendation:* add `getrandom` to `worktree` (small, supports every target OS). Encode 128 bits as hex, and record it in `docs/dependencies.md`.
- [x] **R2 — The deadline and credential behavior of the live remote-head check.**
    - Item 3's `origin/*` verification and `--force-remote` query the live remote. The spec gives no timeout and does not say how to keep `git ls-remote` from prompting.
    - *Recommendation:*
        - Run `git ls-remote origin refs/heads/<b>` with `GIT_TERMINAL_PROMPT=0` and `GIT_SSH_COMMAND` extended with `-o BatchMode=yes`.
        - Apply a 3 s deadline.
        - Treat a timeout as "unavailable": the local branch is kept, and `--force-remote` exits 1 as Decisions 2 rules.
- [x] **R3 — The PR-query deadline for `wt remove`.**
    - The spec says "same deadline" (about 300 ms, the list's figure). A blocking remove can afford more, and 300 ms would often downgrade a merged branch to Pretty safe or Not safe.
    - *Recommendation:* 2 s for `wt remove`, 300 ms for `wt list`.
- [x] **R4 — The `--handoff` argument surface.**
    - *Recommendation:*
        - `--handoff <token>` is hidden.
        - It conflicts with `<name>` and with all `--force-*` flags, because the approved force choices come from the record.
        - `<name>` becomes optional only when `--handoff` is present.
- [x] **R5 — A failed rename-back during the Windows lock probe.**
    - The probe renames the directory to a sibling name and straight back. If the second rename fails, the worktree is left under a temporary name.
    - *Recommendation:* retry the rename-back briefly (3 × 50 ms). If it still fails, exit 1 and name both paths plus the manual `move` command. Never proceed to removal.
- [x] **R6 — What "`CI` is set" means.**
    - *Recommendation:* the variable is present and not empty, matching common CI conventions.
- [x] **R7 — Exit-code plumbing.**
    - *Recommendation:*
        - Add a CLI-side `ExitCode` classification of 0/1/3/4. clap keeps 2.
        - Add the refusal kinds as `WorktreeError` variants (`RefusedToLoseWork`, `BlockedByEnvironment`), each carrying its report.
        - `Cancelled` maps to 0.
    - `main.rs` owns the mapping, and the `wt go` wrapper-less path returns `BlockedByEnvironment`.
- [x] **R8 — Matching a truncated source head SHA.**
    - Some providers may return an abbreviated source head (Bitbucket returns 12 characters in some payloads). Spike S1 establishes the facts.
    - *Recommendation:* when the provider's SHA is shorter than 40 characters, require a unique prefix match against the local tip. Otherwise the PR grants no Safe evidence.
- [x] **R9 — Where the fork-origin record and PR cache live.**
    - *Recommendation:* both sit beside the comparison cache under the per-repository user-cache path from `cache::cache_path`, as separate files with their own format versions, written with `atomic_write`. The handoff record goes in the same directory.
- [x] **R10 — The rows available to the graph height cap when stdout is not a TTY.**
    - *Recommendation:* use the terminal size from stderr/`/dev/tty` detection that `biscuit-terminal` already performs. When unknown, assume 24 rows (cap about 12).
- [x] **R11 — The landing directory for a detached worktree.**
    - The spec implies the base repo, because there is no branch and therefore no fork record.
    - *Recommendation:* confirm the base repo.

### Spikes

Wave 1 runs these spikes in parallel. Each is time-boxed and produces a short findings note in this directory (`spike-s{n}.md`). Findings that change the spec go back to the author as rulings.

- [x] **S1 — PR identity fields across providers**
    - For GitHub, GitLab, Gitea, and Bitbucket, confirm the API field for each of these on **open and merged** PRs:
        - the source repository identity (fork versus same repository)
        - the source head SHA (full or abbreviated)
        - a definitive state
        - a per-branch filter (for example GitHub's `head=owner:branch`)
    - Confirm that authentication and permission failures are distinguishable from an empty list in sniff's current error types.
    - Output: a field-mapping table feeding task P2-sniff and R8.
- [x] **S2 — Non-interactive `git ls-remote` on every OS**
    - Show that the R2 environment prevents any prompt (HTTPS credential helper, SSH passphrase) and that the deadline kills the child process cleanly on macOS, Linux, and Windows.
    - Record the Windows behavior in the `os` skill if it is non-obvious.
- [x] **S3 — Prompts through the captured-stdout wrappers**
    - Confirm that `inquire` renders to stderr and works when zsh, bash, fish, and PowerShell capture `wt`'s stdout.
    - Confirm how to scope `WT_SHELL_WRAPPER=1` to exactly one invocation in each shell:
        - `VAR=1 command wt` in bash and zsh
        - `env WT_SHELL_WRAPPER=1 wt` or `set -lx` in fish
        - set/restore in a `try/finally`, or `Start-Process`-free invocation, in PowerShell
    - Check whether PowerShell's native-command stdout capture preserves the `cd:` / `remove-handoff:` lines exactly, and does not re-encode non-ASCII paths.
- [x] **S4 — The shape of `git status --porcelain --ignored=matching`**
    - Confirm that the output lists ignored top-level directories as `dir/` without descending, and that nested ignored entries under tracked directories appear.
    - Measure its cost on a worktree with a large `target/`, to confirm the remove report stays fast.
    - Confirm the rename lock probe's cost on Windows (ties to R5).
- [x] **S5 — The mermaid-rs-renderer 0.3.1 API and cell pixel size**
    - Confirm the exact signature and units of `render_svg_with_dimensions`.
    - Confirm that biscuit-terminal already exposes the cell pixel size (or where to add it), with the 8×16 fallback.
    - Output: the concrete formula inputs for `ImageWidth::Scale`.

### Validation checkpoint

- [ ] R1–R11 are ruled and recorded in `spec.md`'s Decisions. The spike notes exist and any spec deltas are ruled.
    - *Status 2026-09-24 (Phase 1 agent):* the recommendations, amended by the spikes, are recorded as **proposed** rulings (Decisions 20–33), and `spike-s1.md` … `spike-s5.md` exist. This box stays open until the author confirms or overrides Decisions 20–33; the spec's `human_review_items` explain each choice.

## Phase 2 — Fixes: shell wrapper, resolution, `--from`, sniff PR-for-one-branch

Covers items 2, 6, and 7, plus the sniff entry point that the remove flow's Safe tier uses.

### Wave 2 — CLI foundation (sequential, blocks Wave 3)

- [x] **Exit-code plumbing** (R7)
    - Add the `RefusedToLoseWork` and `BlockedByEnvironment` variants to `worktree/lib/src/error.rs`.
    - Map errors to exit codes 0/1/3/4 in `worktree/cli/src/main.rs`. `Cancelled` exits 0.
    - L1: an `assert_cmd` test per code.
- [x] **Interactivity and wrapper detection**
    - Add one CLI module (for example `worktree/cli/src/env.rs`) with two functions:
        - `is_interactive()`: stdin and stderr are TTYs, and `CI` is unset or empty (R6).
        - `shell_wrapper_active()`: `WT_SHELL_WRAPPER == "1"`.
    - Delete the `stdout().is_terminal()` test in `go.rs`.
    - L1: table-driven tests with the environment injected, never mutating the process environment.

### Wave 3 — Parallel

- [x] **Generated wrappers** (item 7)
    - Rewrite the bash, zsh, and fish wrappers in `print_completions` to:
        - set `WT_SHELL_WRAPPER=1` for the one invocation, per S3
        - handle `cd:` and `remove-handoff:` lines
        - check the `cd` result
        - run the fixed `command wt remove --handoff "$token"`
        - never evaluate output
    - Add `Shell::PowerShell`, which calls `Set-Location` and sets `[Environment]::CurrentDirectory`.
    - Delete `worktree/shell/`, and point the README and docs at `wt --completions`.
    - Update `AFTER_HELP`'s shell-integration block to include PowerShell.
    - L1: snapshot each generated script, and assert the variable, both protocol lines, the `cd` check, and the absence of `eval`, `Invoke-Expression`, and `iex`.
- [x] **`wt go` / `wt create` without a wrapper**
    - `wt go` returns `BlockedByEnvironment` (exit 4) with the existing help, which becomes shell-appropriate by naming all four `--completions` shells.
    - `wt create` without a wrapper creates the worktree, prints the could-not-move message, and exits 0.
    - L1: tests with and without the variable.
- [x] **Resolution and completions** (item 2)
    - Rewrite `find_worktree` in `worktree/lib/src/worktree.rs`:
        - `base` resolves to the main checkout.
        - Otherwise collect exact branch matches and exact basename matches (the input is dasherized for the basename comparison), then dedupe by worktree path.
        - One match resolves; several fail with a new `WorktreeError::AmbiguousWorktree` that lists branch, basename, and path; none is not-found.
    - Rewrite `worktree_names`:
        - For each non-main worktree: its branch and its basename, deduped.
        - Also `base`, plus the base checkout's branch when it is attached.
        - The default branch appears only through the checkout rule.
        - Detached worktrees contribute their basename only.
    - `remove` refuses `base`, and any name that resolves to the main checkout, before any prompt.
    - Pass the porcelain text in so the matching logic is a pure function and testable.
    - L1 (pure):
        - two worktrees on one branch
        - basename collisions across parent directories
        - a branch-to-basename collision
        - a base checkout on a non-default branch while `main` is checked out elsewhere
        - detached worktrees
        - deduping
- [x] **Fork-origin store and `--from`** (item 6)
    - Add a new `worktree/lib/src/fork_origin.rs`:
        - `{ base_branch, base_sha, created_at }` keyed by branch
        - its own format version, stored per R9
        - load, save, and prune-stale APIs (pruning is called from `wt list` in Phase 5)
    - `create_worktree` takes an optional `from` argument:
        - It validates that `from` is an existing **local** branch, with an error naming the branch.
        - It fails when the destination exists and `--from` was given, using the ruled message.
        - It fails on a detached HEAD without `--from`, and explains the choice.
        - It records the fork origin only for new branches.
    - Add `--from <base>` to `Commands::Create`, with completion over local branches.
    - L1 against temporary repositories, covering each ruled message.
- [x] **sniff: PR for one branch** (spike S1 must be complete)
    - Extend `PullRequestInfo`, or add a richer record, with the source repository identity and the source head SHA. Fill them in all four providers.
    - Add a `blocking` module behind the `remote` feature:
        - `pull_request_for_branch(remote_url, source_repo, branch, deadline) -> Result<Option<PrEvidence>, PrUnavailable>`
        - It builds a current-thread Tokio runtime internally. Its docs state that it must not be called from inside an existing runtime.
        - It returns an auth/permission error as `Unavailable`, never as `None`.
    - Update the sniff skill and docs.
    - L1 with `wiremock` per provider:
        - open PR, merged PR, fork source, missing head SHA, auth failure, timeout

### Validation checkpoint

- [x] `just test` and `just lint` pass in `worktree` and `sniff`. The new tests exist for acceptance criteria 2, 6, and 7 (output side).
- [x] Manual smoke in zsh:
    - `source <(wt --completions zsh)`; `wt go <basename>` and `wt go <branch>` both work.
    - `command wt go x` exits 4.

## Phase 3 — Fixes: the `wt remove` rewrite (items 1, 3, 4)

### Wave 4 — Parallel, in the library (`worktree/lib/src/remove/` as a new module)

- [x] **Removal inventory**
    - Collect the dirty entries from `git status --porcelain` (modified, staged, untracked), with a kind classification that reuses `has_source` logic.
    - Collect the ignored top-level entries from `--ignored=matching` (S4).
    - All git calls run through `git -C <base repo>` or `git -C <target>` for read-only status. Nothing runs with the target as the working directory.
- [x] **Safety evidence and tiers**
    - Select the default-branch target: the descendant of local and `origin/<default>`; `origin/<default>` when they diverge; local when there is no remote-tracking ref. This becomes a shared function that Phase 5 reuses.
    - Pass 1 — Safe: the tip is reachable from local `<default>` or `origin/<default>`, **or** the PR evidence (from sniff) matches the source repository and the exact tip (R8).
    - Pass 2 — Pretty safe: the tip is reachable from another local branch, an `origin/*` ref, or a tag. `origin/*` evidence other than `origin/<default>` must pass the live check (R2): the live SHA equals the local tip, or the live SHA equals the tracking SHA and the tracking ref contains the tip.
    - `--force-remote` excludes the branch's own origin copy and its open PR from the computation.
    - Any failure yields `Unknown`, which is treated as Not safe with the reason attached.
    - Compute the lost-commit list: the commits on the branch that no other local branch, `origin/*` ref, or tag contains.
    - Compute ahead/behind against the selected target.
    - Name the exact evidence ref for each tier. Label remote-tracking evidence "as of your last fetch".
    - L1 with local bare remotes and a PR-evidence stub trait:
        - every tier: merged PR, open PR, fork PR, tag, other branch, own origin copy, stale origin ref, deleted remote branch, remote unreachable
        - the original bug case (item 4): merged into HEAD, not into its upstream
- [x] **Handoff record**
    - The record holds:
        - a token from R1
        - repository identity, canonical target path, HEAD, branch tip, landing path, approved force choices, and created-at time
    - The dirty fingerprint uses `biscuit-hash` BLAKE3 over each dirty path's status and content, plus the set of ignored entries.
    - Write the record with `atomic_write`. It expires after 60 s.
    - `consume(token)` deletes the record before returning it, which prevents replay.
    - The check function compares the stored state with fresh state and requires the current directory to be outside the target. Use a path comparison that follows the `os` skill's Windows spelling guidance.
    - L1:
        - expiry
        - replay
        - each field changing, including a new ignored entry and a content-only edit
        - a caller still inside the target
- [x] **Removal primitives**
    - `remove_worktree` gains a `#[cfg(windows)]` lock probe before `git worktree remove`: rename to a sibling and back, following R5. A held directory returns `BlockedByEnvironment`.
    - Branch deletion: `git branch -D` only, decided by the caller. Delete `delete_branch` and `DeleteBranchOutcome`.
    - Remote deletion:
        - Resolve the destination: the configured upstream when it is on origin; otherwise `origin/<branch>`; never a PR from another repository.
        - Query the live head (R2).
        - Report the remote-only commits, or "unknown" when they are absent locally.
        - Push with `--force-with-lease=refs/heads/<b>:<observed sha> origin :refs/heads/<b>`.
    - L1 with a bare remote: a push between preflight and deletion fails the lease and preserves the new head.
    - L2 on Windows: a held directory exits 4 with nothing removed.

### Wave 5 — CLI flow (depends on Wave 4)

- [x] **Flag surface**
    - Replace `force: u8` and `branch: bool` with `--force-worktree`, `--force-branch`, `--force-remote`, and the hidden `--handoff` (R4). No short forms.
    - Rewrite `AFTER_HELP` with the spec's examples.
    - Retired flags produce clap's own error (exit 2). Covered by L1.
- [x] **Report renderer**
    - Render with `Prose` and `UnorderedList`:
        - The dirty tree: at most 10 entries, colored by kind with the table's dot palette. Above 10, a bold red total count. The 50-file cap and "…and N more" are deleted from `dirty_tree.rs`.
        - The ignored entries, with the contents-deleted wording.
        - The tier and its evidence ref.
        - Ahead/behind against the named target.
        - The origin copy, "as of your last fetch".
        - The PR.
    - Each question starts after exactly one blank line (item 1).
- [x] **Decision flow**
    - Order: report, then the worktree question, then the branch question, then the remote step.
    - Covers:
        - the `--force-branch`-without-`--force-worktree` conflict: an error in non-interactive mode, and the files question in interactive mode
        - a Not safe branch, where the prompt defaults to "keep"
        - non-interactive keep with a warning (exit 0)
        - a `--force-remote` failure after local removal (exit 1, with the finishing `git push origin --delete` command)
    - Keep the policy as a pure function, from (inventory, tier, flags, interactivity, answers) to actions and an exit code, so the L1 matrix does not need a terminal.
- [x] **Move-first removal from inside the target**
    - Detect that the current directory is inside the target.
    - Without a wrapper, exit 4 with the `wt go` help plus "run it from another directory".
    - With a wrapper:
        - The first run asks every question, writes the handoff record, and prints `cd:<landing>` followed by `remove-handoff:<token>`.
        - The landing directory is the fork parent's worktree when the fork record names a parent that has a worktree; otherwise the base repo (R11). The current subdirectory is kept when it exists there.
        - The second run consumes the token, re-verifies state and tiers without prompts, and either removes or refuses: exit 3 for a new risk, exit 4 for an expired or missing token.
    - The same sequence runs on every OS.

### Wave 6 — Remove test matrix (parallel; depends on Wave 5)

- [x] **L1 policy matrix**
    - Cover this cross-product: tier × dirty state × ignored entries (none, `.env`, `target/`) × each flag subset × interactive or not.
    - Assert the resulting actions and exit codes, including every row of the spec's Examples table.
    - PR answers come from the stub, including "unavailable".
- [x] **L2 real-terminal tests** (`worktree/cli/tests/level2_remove.rs`)
    - The blank line before each prompt.
    - The Not safe menu.
    - Move-first through zsh, bash, and fish wrappers under tmux:
        - It lands in the fork parent or the base repo, and says which.
        - A failed `cd` (landing directory removed between runs), an expired token, and a changed branch tip each leave the worktree intact.
    - Never take focus. Gate with `require_level!`.
- [x] **Windows L2**
    - A PowerShell wrapper, launched with the target as its working directory, removes the target.
    - The held-directory check exits 4.
    - If CI has no Windows L2 cell, record the criterion as unmet with provisioning as the required change. Run it on the Windows build host per the `os` skill.
    - *Status 2026-09-24 (Phase 3 agent):* both scenarios ask no question, so they run as Windows-only **L1** tests, `cli/tests/powershell_wrapper_exec.rs` plus a `cfg(windows)` lib test. CI's Windows L1 cell runs them, and they passed on `build-win-native` through `just cross-check`. No Windows L2 (real console) cell exists for the interactive PowerShell prompts, so that part is recorded as unmet, with provisioning a Windows L2 harness as the required change.

### Validation checkpoint

- [x] `just test`, `just test-l2`, and `just lint` pass in `worktree`. Acceptance criteria 1, 3, and 4 have named passing tests (see the Phase 3 mapping in `implementation-log.md`).
- [x] Code search finds no `-ff`, `FORCE_BYPASS_FILE_LIMIT`, `delete_branch`, or `DeleteBranchOutcome`. The one `-ff` left is in `retired_flags_are_clap_errors`, which asserts clap rejects it.
- [x] The Phase 1–3 changes form a coherent landing point: the fixes ship before the list work, as sequenced.

## Phase 4 — Dependencies for the table and graph

### Wave 7 — Parallel

- [x] **mermaid-rs-renderer 0.3.1**
    - Bump the dependency in `biscuit-visualized/src/Cargo.toml` (resvg and usvg move to 0.47, adding tiny-skia 0.12).
    - Teach `fix_pie_text_contrast` to parse `hsl()`, and update `mermaid_pie_chart_init_directive_applies_custom_colors` and its white-slice comment.
    - Update the version in `biscuit-terminal/docs/data-visualization/visualizing-graph-expressions.md` and in `docs/dependencies.md`.
    - Validate: `biscuit-visualized`'s suite, biscuit-terminal's Mermaid, diagram, and parity tests, and Darkmatter's Mermaid tests.
- [x] **Per-row highlight on `Table`**
    - Add a typed style slot on `TableStyle` plus a builder method (for example `highlight_row(index, color)`) that composes with striping; the highlight wins on its row.
    - Update the component docs.
    - L1 snapshot.
- [x] **sniff: a repository's open PRs**
    - Add a blocking `open_pull_requests(remote_url, deadline) -> Result<Vec<PrSummary>, PrUnavailable>` returning number, URL, source repository, source branch, and target branch. It reuses the Phase 2 runtime helper and the extended record.
    - All four providers.
    - L1 with `wiremock`, including auth failure and timeout distinguished from an empty list.

### Wave 8 — Parallel (depends on the mermaid upgrade)

- [x] **`ImageWidth::Scale(f32)`**
    - Pixels per SVG unit = scale × cell height ÷ 16.
    - Columns = ceil(SVG width × pixels per unit ÷ cell width).
    - Clamp to the available columns minus the margins.
    - Fall back to an 8×16 cell size.
    - Take the natural width from `render_svg_with_dimensions` (S5).
    - `MermaidDiagram` defaults to `Scale(1.0)`. Audit the existing `MermaidDiagram` callers, including Darkmatter, for the changed default.
    - L1 on the computed sizes.
- [x] **`GitGraph` core**
    - A new `biscuit-terminal/lib/src/components/git_graph.rs`.
    - Typed input: lines of commits with full SHAs, ref tips, open PRs, elision counts, and the current branch.
    - Lane/tag rule:
        - Lanes: the default branch, the non-default fork parent, and the current branch; in the base view, every branch with commits of its own.
        - `origin/<default>` gets a lane only when it has diverged.
        - PRs are tags of the form `PR #n → target`.
    - Lane ordering by branch creation order.
    - Workaround: emit no attributes on `branch` or `merge` statements.
    - Implements `TerminalRenderable`, `TreeRenderable`, and `BrowserRenderable` (SVG as a raw-HTML island, as `GraphExpression` does).
    - L1 on the generated Mermaid text for the spec's example (standing in `feat-dark-fixes`).

### Wave 9 — `GitGraph` sizing and upstream drafts (depends on Wave 8)

- [x] **Fit by trimming**
    - The default scale is 125%.
    - The width cap trims commits (a larger `+N`) before any shrinking.
    - The base-view height cap is about half the terminal rows (R10). Past it, show fewer lanes, most recently active first, followed by "N more worktrees not shown".
    - L1 on the trimming decisions and the computed sizes.
- [x] **Upstream drafts**
    - Write a minimal-reproduction issue and a small PR that strips attributes from the branch name in `branch` and `merge`.
    - Save both as drafts in this directory (`upstream-issue.md`, `upstream-pr.patch`) for the author. **Do not file them.**

### Validation checkpoint

- [x] `just test` and `just lint` pass in `biscuit-visualized`, `biscuit-terminal`, `sniff`, and `darkmatter`, plus `just test` in `worktree`, which exercises the existing graph path.
- [x] The acceptance criterion 5 bullets for `GitGraph` and the renderer upgrade are green.

## Phase 5 — `wt list` table and graph (item 5)

### Wave 10 — Parallel, in the library

- [ ] **Comparison-cache generalization**
    - The key becomes `(target_tip, branch_tip, version)`.
    - Bump `CACHE_FORMAT_VERSION`.
    - Serves the `-> {default}` column, the `-> parent` column, and the caption (`rev-list --left-right --count`).
    - Update the worktree skill's cache sentence.
- [ ] **List data model**
    - Read the default tip, the `origin/<default>` tip, the fork-parent tips, and whether each parent exists from one `git for-each-ref refs/heads refs/remotes`, replacing `default_tip_sha`.
    - Use the Phase 3 target-selection function.
    - Per branch, compute `already in`, `clean`, or `conflicts` against the target and, when the parent is not the default branch, against the parent.
    - Compute the caption state.
    - Build the fork tree from the fork-origin store: parent rows without worktrees, deleted parents, root-level branches with no record, and detached rows.
    - Prune stale fork records during the existing save.
    - Keep the `parse_worktree_state` / `fill_worktree_statuses` seam intact.
- [ ] **PR cache and deadline**
    - Store per-repository PR results with their fetch time (R9).
    - Skip the network within 60 s of the last fetch.
    - Otherwise run sniff's open-PR request on a thread in parallel with the git work, under a 300 ms deadline.
    - On failure or timeout, use the stored results with their age. Never cache "unavailable" as an empty list.
    - Match PRs to branches by source repository **and** branch name.
    - Place each badge by the PR's target: the parent column, the default column, or beside the branch as `PR #n → target`.

### Wave 11 — CLI rendering (depends on Wave 10)

- [ ] **The table**
    - Rewrite the table rendering in `worktree/cli/src/commands/list.rs`:
        - the caption with the count in yellow
        - the Worktree column with dot glyphs and `base repo` in dim italic; the current worktree's name in bold
        - the Branch column tree with gray or red connectors, deleted parents struck through, and `└┄`
        - the two target columns
        - three badge roles, with PR badges as OSC 8 links through `Prose`
        - a two-line legend
        - the "PRs as of N min ago" line
        - the current row highlighted through the new `Table` API
    - Retire the 120-column suppression rule.
    - L1 snapshots for every caption variant, cell, badge placement, and the legend.
- [ ] **Graph handoff**
    - `git_graph.rs` gathers typed facts only: commits per line with full SHAs, ref tips, and open PRs.
    - Build a `GitGraph` from those facts.
    - Delete `default_graph_width` and the CLI's Mermaid text building.
    - `--width` overrides the scale-derived width.
    - Update `worktree/docs/git-graph.md`.

### Wave 12 — Performance and L2 (parallel; depends on Wave 11)

- [ ] **Performance gates**
    - Run the existing `perf_command_sla`, `cache_warm_path`, and `cache_cold_path` tests and the `list_status` bench against the ratified targets.
    - Add cases for:
        - the network down
        - a stubbed slow PR request that hits the deadline
        - a fresh PR cache that makes no request (assert through a call counter)
- [ ] **L2 list**
    - Update `level2_list_verbose.rs` and `list_output.rs` for the new table and graph in a real terminal.

### Validation checkpoint

- [ ] `just test`, `just test-l2`, and `just lint` pass in `worktree`. All of acceptance criterion 5 is green.

## Phase 6 — Documentation, cross-OS evidence, and hand-off

### Wave 13 — Parallel

- [ ] **Docs drift pass**
    - `worktree/README.md` and `worktree/docs/cli/list.md`, plus new `docs/cli/remove.md`, `go.md`, and `create.md` if the area documents commands per file.
    - `worktree/docs/git-graph.md` and `worktree/docs/performance-testing.md`, if the gather steps changed.
    - The skills: `.claude/skills/worktree/SKILL.md` (fork-origin store, handoff, exit codes, cache key), the sniff skill, the biscuit-terminal skill (`GitGraph`, `ImageWidth::Scale`, row highlight), and the `os` skill (Windows lock probe and PowerShell wrapper facts learned).
    - `docs/dependencies.md`.
    - Apply the comment-quality pass from `CLAUDE.md` to every changed symbol.
- [ ] **Cross-OS evidence**
    - Follow the `os` skill: run the L1 suites and the L2 wrapper tests on macOS, Linux, native Windows (PowerShell), and WSL2.
    - Record every criterion without evidence as **unmet with provisioning as the required change**.
    - Run `just ci-local --plan` before any push.

### Final checkpoint

- [ ] Walk the Definition of done in the first section item by item.
- [ ] Update `spec.md` frontmatter to `status: implemented`.
- [ ] Report "implementation complete, ready for review". The author closes the spec.
