---
spec: /Volumes/coding/wt/rusty-biscuit/fix-wt-ux/worktree/fixes/2026-09-24-ux-improvements/spec.md
plan: worktree/fixes/2026-09-24-ux-improvements/plan.md
implemented_by: claude/opus
started_phase: 1
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
source_files_during_phase_5:
    - worktree/lib/src/cache.rs
    - worktree/lib/src/default_target.rs
    - worktree/lib/src/lib.rs
    - worktree/lib/src/listing.rs
    - worktree/lib/src/pull_requests.rs
    - worktree/lib/src/worktree.rs
    - worktree/cli/src/commands/mod.rs
    - worktree/cli/src/commands/list.rs
    - worktree/cli/src/commands/list/tests.rs
    - worktree/cli/src/commands/list_table.rs
    - worktree/cli/src/commands/git_graph.rs
    - worktree/cli/src/commands/git_graph/tests.rs
    - worktree/cli/tests/list_output.rs
    - worktree/cli/tests/list_table.rs
    - worktree/cli/tests/list_prs.rs
    - worktree/cli/tests/perf_pr_request.rs
    - worktree/cli/tests/perf_support/mod.rs
    - worktree/cli/tests/level2_list_verbose.rs
    - worktree/cli/tests/snapshots/list_table__the_spec_example_renders_as_ruled.snap
docs_updated_during_phase_5:
    - worktree/docs/git-graph.md
    - worktree/docs/performance-testing.md
    - worktree/fixes/2026-09-24-ux-improvements/plan.md
    - worktree/fixes/2026-09-24-ux-improvements/implementation-log.md
    - worktree/fixes/2026-09-24-ux-improvements/spec.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/worktree/SKILL.md
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

# Implementation Log for 2026-09-24-ux-improvements (6 phases)

## Phase 1

Phase 1 is rulings and spikes only; no production code changes.

### Rulings (R1–R11)

- Recorded as **proposed** Decisions 20–30 in `spec.md`, amended where a spike changed the answer. Three further proposals came from the spikes: 31 (ignored-entry display), 32 (graph natural-width API), and 33 (PowerShell output encoding).
- The spec is set to `human_review: true`. The plan's validation checkpoint stays unchecked until the author confirms Decisions 20–33.
- R1: `getrandom` 0.4 is already in the workspace graph (Darkmatter), so `worktree` adds no new crate.
- R9: `cache::cache_path` returns a file, not a directory, so the new records are sibling files keyed by the same repository hash. Callers already prefer the main worktree's path (`cache.rs:53`).

### Spikes

- **S1** (subagent, `spike-s1.md`): every provider exposes the source repository, head SHA, and state. Only Bitbucket Cloud abbreviates SHAs (12 characters).
  - R8 is confirmed with three amendments: compare against the local object-ID length, require at least 7 hex characters, and check the prefix is unique locally.
  - Blocking defect for Phase 2: sniff's focused client turns a list-endpoint 404 into an empty list.
  - Gitea's single-PR endpoint reports the live branch tip, even after merge.
- **S2** (`spike-s2.md`, run on macOS, `$BUILD_LINUX`, and `$BUILD_WIN`):
  - No prompt appeared in any case with `GIT_TERMINAL_PROMPT=0`, `BatchMode=yes`, and `credential.interactive=never`. The cases covered HTTPS without credentials, an unknown host key, and a passphrase key without an agent.
  - A plain kill leaves the transport grandchild holding stderr on every OS. On Windows the real `git.exe` is also orphaned and holds its working directory.
  - Killing the process group (Unix) or `taskkill /T /F` (Windows) released everything at the 3 s deadline.
- **S3** (`spike-s3.md`): an `inquire` probe ran under zsh, bash, and fish wrappers in detached tmux sessions.
  - Prompts render on stderr, the variable is scoped to one invocation, and non-ASCII paths with spaces arrive exact.
  - The token reaches `wt` as one literal argument; a `$(…)` payload inside it never ran.
  - A failed `cd` stops the wrapper before the handoff.
  - PowerShell 5.1 on `$BUILD_WIN` corrupts non-ASCII captured output unless `[Console]::OutputEncoding` is UTF-8.
  - POSIX wrappers must act after the read loop, so the handoff call keeps the terminal's stdin.
  - fish 4.9.3 was installed on this macOS host with Homebrew for the spike.
- **S4** (`spike-s4.md`):
  - `--ignored=matching` collapses only directories matched by a directory pattern. This repository's `**/target/*` lists the children of `target/`, which drives proposed Decision 31.
  - Cost: 0.07 s against 0.03 s for a plain status on this checkout. `traditional` takes 0.20 s.
  - The Windows rename probe costs about 1.5 ms per pair, and a held directory fails on the first rename in 2 ms.
- **S5** (subagent, `spike-s5.md`):
  - The sizing API is `measure_svg_dimensions` (`viewbox_width`), not `render_svg_with_dimensions`.
  - `Terminal::cell_size()` exists; on native Windows it is always `None`.
  - Rows fall through stdout, stderr, and stdin, which confirms R10.
  - 0.3.1 rejects Mermaid input with no header, which is a Phase 4 fixture risk.

### Skill updates

- `.claude/skills/os/windows.md`: Windows `git` deadline kills need a tree kill; PowerShell 5.1 capture encoding; lock-probe cost.
- `.claude/skills/os/build-hosts.md`: `tar` on `$BUILD_WIN` is Cygwin's; use System32 `tar.exe`.
- The `worktree` skill is unchanged, because no worktree behavior changed in this phase.

### Tests and gates

- No test was added or changed: Phase 1 has no behavior change. Its evidence is the spike notes, and each probe was a throwaway crate outside the repository, now deleted.
- `just test` (worktree): 149 passed, 11 skipped. `just lint` (worktree): clean.
- Scratch probes were removed from `$BUILD_LINUX` and `$BUILD_WIN` (`B:\scratch-s2probe`, `B:\scratch-s4probe`).

## Phase 2

Phase 2 covers the shell wrapper (item 7), name resolution (item 2), `wt create --from` with the fork-origin store (item 6), the exit-code plumbing, and sniff's blocking PR-for-one-branch lookup.

### Starting conditions

- The spec still carried `human_review: true` for proposed Decisions 20–33. The author launched Phase 2 anyway, so this phase implemented the proposed rulings it touches: 25 (`CI` present and not empty), 26 (the exit-code plumbing), 27 (the PR head SHA stored exactly as received), 28 (record location), and 33 (PowerShell UTF-8). The confirmation item stays open in the spec, because Phase 3 depends on 20–24 and 31.
- The sniff task ran in a subagent in parallel. It touched only sniff, one darkmatter test literal, and the sniff skill. I ran every other task myself and re-ran the sniff gates.

### Exit codes and interactivity (R7, R6)

- `WorktreeError` gains `RefusedToLoseWork(String)` and `BlockedByEnvironment(String)`, each carrying Prose markup that `main.rs` prints as-is. It also gains `AmbiguousWorktree { name, candidates }`, `FromWithExistingBranch`, `FromBranchNotFound`, and `DetachedHeadWithoutFrom`.
- `cli/src/exit.rs` maps `Cancelled` to 0, `RefusedToLoseWork` to 3, `BlockedByEnvironment` to 4, and everything else to 1; clap keeps 2. `main.rs` exits with that code. `Cancelled` now prints a dim "Cancelled." instead of an error.
- Other errors now pass through `Prose::escape_text`, because the new messages contain `--from <branch>`, which Prose would otherwise parse as a tag.
- `cli/src/env.rs` holds `is_interactive()` (stdin and stderr are TTYs, and `CI` is unset or empty) and `shell_wrapper_active()` (`WT_SHELL_WRAPPER` is exactly `1`). The pure `*_from` helpers take injected values, so the tests never mutate the process environment.
- For a real code-3 producer before Phase 3, the current `wt remove` refuses a dirty worktree with exit 3 when it is not interactive. Before this change the same case failed with exit 1 on inquire's "not a TTY" error. Phase 3 replaces the whole flow.

### Shell wrappers (item 7)

- `cli/src/shell_integration.rs` generates the wrapper plus clap's completion registration **in-process**, through `clap_complete::env::Shells::builtins().completer(..).write_registration(..)`. The old scripts sourced `COMPLETE=<shell> command wt` at shell start. The in-process registration drops that second process and keeps the protocol wrapper free of any evaluation. clap's own PowerShell completer still uses `Invoke-Expression` on the command line being completed, never on `wt`'s output, so the no-eval assertion targets the wrapper function.
- bash and zsh share one bash-3.2-compatible POSIX wrapper, shaped as spike S3 found. fish uses `WT_SHELL_WRAPPER=1 command wt` (fish 3.1+).
- The PowerShell wrapper embeds the absolute executable path from `current_exe()` at generation. **Finding:** Windows Terminal installs its own `wt.exe` app alias, so a bare `wt` can resolve to Windows Terminal. The wrapper also switches the console to UTF-8 for the call (Decision 33), sets both `Set-Location` and `[Environment]::CurrentDirectory`, restores the variable and the encoding in `finally`, and sets `$global:LASTEXITCODE` on failure.
- `--completions` accepts only bash, zsh, fish, and powershell. Anything else, including elvish, is a clap error (exit 2); it previously printed a message and exited 0.
- `worktree/shell/` is deleted. The README gains "Shell Integration" and "Exit Codes" sections, and `AFTER_HELP` lists all four shells.
- The fish install line changes from `source (wt --completions fish | psub)` to `wt --completions fish | source`.

### `wt go` / `wt create` without the wrapper

- `wt go` returns `BlockedByEnvironment` (exit 4) with help naming all four `--completions` lines. The "already in this worktree" case still exits 0 before the wrapper check.
- `wt create` without the wrapper creates the worktree, says it could not move the shell (with the same help), and exits 0. The "you have been moved" line appears only when a `cd:` line is printed. `--stay` prints neither.

### Resolution and completions (item 2)

- `resolve_worktree(entries, name)` and `completion_names(entries)` are pure functions over parsed porcelain. `find_worktree` and `worktree_names` are thin wrappers around them.
- **Assumption (stated per Rule 1):** basenames match on linked worktrees only. The main checkout's basename is the repository name, which completions never offered, and `base` is its stable name. The input matches the basename raw **or** dasherized, so a plain-git directory such as `Foo_Bar` still resolves by the exact name completions offer.
- `wt remove base` and `wt remove main` (the base checkout's branch) both hit the existing main-checkout refusal before any prompt; a test now covers it.

### Fork-origin store and `--from` (item 6)

- `lib/src/fork_origin.rs` stores `{ base_branch, base_sha, created_at (Unix seconds) }` per branch in `<repo hash>.fork-origins.json`, format version 1. `cache::repo_cache_file(repo_root, suffix)` now builds both that path and `cache_path`. `prune(live_branches)` keeps a record whose parent was deleted, because Phase 5 lists that parent as deleted.
- `create_worktree(branch, base, from)`:
  - `from` must resolve under `refs/heads/`, so a remote-only branch or a tag is rejected with the not-a-local-branch message.
  - An existing destination branch plus `--from` fails with the ruled message.
  - A detached HEAD without `--from` fails.
  - With `--from`, the start point is `refs/heads/<from>`, so a same-named tag cannot shadow it. Without `--from`, no start point is passed, which keeps today's behavior, and the recorded base is `symbolic-ref HEAD`.
- The record write is best-effort after the worktree exists, because the store is a user cache. The CLI prints "(forked from X)".
- `--from` completes local branches through `local_branches()`.

### sniff: PR for one branch (subagent)

- `PullRequestInfo` gains `source_repo`, `source_repo_is_target`, and `source_head_sha`, filled in both the Stage-1 mappers and the focused normalizer.
- `sniff::remote::blocking::pull_request_for_branch(remote_url, source_repo, branch, deadline)` and `pull_request_for_branch_with(client, ..)` return `Result<Option<PrEvidence>, PrUnavailable>`. `PrState` is `{ Open, Merged }`, and `PrUnavailable` is `{ Timeout, Network, Auth, NotFoundOrNotPermitted, RateLimited, Unsupported, Other }`.
- The lookup runs on a current-thread runtime. A call from inside a runtime returns `Other` instead of panicking. The deadline covers the whole lookup.
- Choice among matches: open beats merged, then the most recent wins. **No local-tip parameter exists**, so if an older merged PR matches the tip but a newer one does not, the Safe evidence is lost. That fails safe: the branch falls to a lower tier. See the message to the Phase 3 agent.
- `pull_request_for_branch` supports only hosts identifiable from the URL (github.com, gitlab.com, bitbucket.org, `gitea.*`, `forgejo.*`, codeberg.org). Self-hosted servers get `Unsupported`, which falls through to the lower tiers.
- Existing behavior changed on the way: the Gitea/Forgejo focused normalizer reads the branch from `head.label`, and a body-read timeout is now reported as unreachable rather than "malformed JSON".
- **Not fixed (spike S1 drift items, off this path):** Stage-1 Bitbucket All/Closed returns OPEN only; Gitea `has_merged` lacks its serde rename; the shared focused pagination sends `per_page` to Gitea; the GitHub request docs claim a default of 100. The old `query_pull_requests` still reads a list 404 as "no more pages". Each needs its own commit.
- The subagent ran `rustfmt` on individual files it changed: its two new files and `github.rs`/`gitlab.rs`. The diffs in the existing files are limited to the mapper closures it rewrote (their bodies re-indent inside a new block); nothing else in those files moved.

### Requirement-to-test mapping

| Requirement | Tests |
|---|---|
| Exit codes 0/1/2/3/4, one `assert_cmd` test each | `wrapper_protocol::exit_0_for_completions`, `exit_1_for_an_unknown_worktree`, `exit_2_for_invalid_arguments`, `exit_3_when_removal_would_lose_files_and_nobody_can_confirm` (CI unset and `CI=true`), `exit_4_for_go_without_the_shell_wrapper` (unset, `0`, empty) |
| `Cancelled` → 0 and the full mapping | `exit::tests::refusals_and_cancel_map_to_their_codes`, `every_other_error_is_a_failure` |
| Interactive / wrapper detection, table-driven | `env::tests::interactive_requires_both_terminals_and_no_ci`, `wrapper_is_active_only_for_exact_one` |
| AC7: `cd:` only with `WT_SHELL_WRAPPER=1` | `exit_4_for_go_without_the_shell_wrapper` (stdout empty), `go_prints_cd_only_with_the_wrapper_and_resolves_branch_and_basename`, `create_without_the_wrapper_creates_but_cannot_move`, `create_with_the_wrapper_moves_and_stay_does_not` |
| AC7: every wrapper sets the variable, handles both lines, checks `cd`, runs the fixed handoff, never evaluates | `shell_integration::tests::*` (7 tests), `wrapper_protocol::wrapper_snapshots` (4 insta snapshots) |
| AC7 by behavior (bash on Unix, zsh on macOS), stub `wt` on `PATH` | `shell_wrapper_exec::wrapper_changes_directory_then_runs_the_handoff_with_the_literal_token` (non-ASCII path with a space; token holding `$(touch …)` and a backquoted command arrives as one literal argument, and the file is never created), `wrapper_stops_before_the_handoff_when_cd_fails`, `wrapper_passes_a_failure_through_without_acting_on_protocol_lines`, `wrapper_with_no_protocol_lines_prints_output_and_stays`. A mutation check (dropping `\|\| return 1` and the token quoting) turned two of these red. |
| AC2: resolution and completions | `worktree::tests::resolve_by_branch_and_by_basename_reach_the_same_worktree`, `resolve_unknown_name_is_not_found`, `two_worktrees_on_one_branch_are_ambiguous`, `basename_collision_across_parent_directories_is_ambiguous`, `branch_name_colliding_with_another_basename_is_ambiguous`, `ambiguity_error_lists_branch_basename_and_path`, `base_on_another_branch_while_default_is_checked_out_elsewhere`, `detached_worktrees_resolve_and_complete_by_basename_only`, `completion_names_offer_branch_and_basename_deduplicated`, `find_worktree_and_worktree_names_read_the_real_repository`; CLI: `go_with_an_ambiguous_name_lists_every_match_and_moves_nowhere`, `completions_offer_branch_and_directory_names`, `remove_refuses_the_base_checkout_by_any_name_before_prompting` |
| AC6: `--from` | lib: `create_from_forks_the_named_branch_and_records_it`, `create_without_from_forks_and_records_the_current_branch`, `create_from_with_existing_branch_fails_and_creates_nothing`, `create_reusing_an_existing_branch_records_nothing`, `create_from_a_missing_or_remote_only_branch_names_it` (missing, remote-only, `origin/…`, tag), `create_on_detached_head_requires_from`; CLI: `create_from_forks_the_named_base_and_records_it`, `create_from_errors_use_the_ruled_messages_and_create_nothing`, `create_from_completes_local_branches` |
| Fork-origin persistence (round trip, versions, prune, location) | `fork_origin::tests::repeated_round_trip_preserves_records` (write/read/write/read), `record_replaces_an_existing_entry`, `missing_corrupt_and_other_version_files_load_empty`, `prune_drops_deleted_branches_and_keeps_orphaned_children`, `store_file_sits_beside_the_comparison_cache` |
| sniff PR-for-one-branch, per provider | `sniff/lib/tests/l1/pr_for_branch.rs` (15 tests: open, merged, open over merged, closed/declined never count, fork mismatch, fork match, missing head SHA, auth 401/403, list 404, GitLab fork-project 404, timeout, server-side filters, Gitea `limit` paging, Bitbucket 12-character hash, Gitea merged SHA from the list) and 5 unit tests in `remote::blocking::tests` |

All new tests are L1: no tier-marker segment appears in any path, and every file is auto-discovered (`worktree-cli` does not set `autotests = false`). `shell_wrapper_exec.rs` is `#![cfg(unix)]`, zsh runs on macOS only, and fish and PowerShell execution are left to Phase 3's L2 wrapper tests.

### Gates

- `just test` (worktree): 213 passed, 11 skipped. The same 11 were skipped at the Phase 1 baseline.
- `just lint` (worktree): clean.
- `just test` (sniff): 2846 passed, 31 skipped. `just lint` (sniff): exit 0, no warnings.
- `cargo check -p darkmatter --tests`: passes after the test-literal fix.
- Manual zsh smoke (`zsh -f`, built `wt` on `PATH`): `wt go theme-dir`, `wt go base`, and `wt go feat/theme` land correctly; `command wt go base` exits 4. The only noise is `compdef: command not found`, because `zsh -f` does not run `compinit`; a real `.zshrc` does.
- No `cargo fmt` was run.

### Observations (not fixed; out of scope)

- `is_current_worktree` uses a prefix check, so with a linked worktree *nested inside* the base checkout, standing in the linked worktree also counts as standing in base. `wt go base` then says "already in base". One test first tripped on this; it now places its worktree in a sibling directory. `wt create` never nests worktrees.

## Phase 3

Phase 3 is the `wt remove` rewrite (items 1, 3, and 4): the safety tiers, the three `--force-*` flags, the report-first flow, move-first removal through a handoff token, the Windows lock check, and exit codes 0–4.

### Starting conditions

- Decisions 20–33 were still **proposed** (`human_review: true`). As in Phase 2, the phase ran on them: 20 (handoff token), 21 (live remote check), 22 (2 s PR deadline), 23 (`--handoff` surface), 24 (lock probe rename-back), 27 (abbreviated PR heads), 28 (record location), 30 (detached landing), and 31 (grouped ignored entries). Each is implemented as proposed.
- I did all the work myself, without subagents, because the library and CLI halves share types throughout.

### Library: `worktree::remove` (Wave 4)

New module `worktree/lib/src/remove/`, plus `worktree/lib/src/default_target.rs`.

- **`default_target::select_default_target(base, default)`**: returns local `<default>` or `origin/<default>`, whichever contains the other. When they have diverged it returns `origin/<default>` with `diverged: true`. Without a remote-tracking ref it returns the local tip. Phase 5 reuses it.
- **`inventory`**: runs one `git status --porcelain=v1 -z --untracked-files=all --ignored=matching`.
  - `-z` handles spaces and renames, whose original path is the next record.
  - `-uall` lists every untracked file, so the >10 count covers every path.
  - `ignored_groups()` groups entries by first path component (Decision 31).
  - `fingerprint()` is a BLAKE3 digest of each dirty entry (status, path, and a streamed content hash; a symlink hashes its target) plus the sorted ignored set.
- **`safety::assess`**: classifies the branch from one `git for-each-ref --contains <tip> refs/heads refs/remotes/origin refs/tags`, while the PR lookup runs on a scoped thread.
  - Pass 1, Safe: local or `origin/<default>` contains the tip, or a PR is the tip's own. The PR must be open (unless `--force-remote`) or merged, come from origin's `owner/repo` (compared case-insensitively), and have a head that equals the tip or is a Decision 27 prefix. A prefix must be at least 7 lowercase hex characters, begin the tip, and be unique under `git rev-parse --disambiguate`.
  - Pass 2, Pretty safe: another local branch, then a tag (no network needed), then each `origin/*` ref after the live check. The branch's own copy is checked first. The live SHA must equal the tip, or equal the tracking SHA. After the first unreachable answer the remaining remote refs are skipped.
  - `--force-remote` leaves out the own origin copy (from both the tier and the lost-commit list) and an open PR.
  - Any evidence that was considered and rejected becomes a plain-words `note`.
  - Lost commits: `git log <tip> --not --exclude=<branch> --branches [--exclude=origin/<dest>] --remotes=origin --tags`.
- **`live_remote`**: the only network path in removal code (Decision 21).
  - Runs `git -C <base> -c credential.interactive=never …` with `GIT_TERMINAL_PROMPT=0`, `GCM_INTERACTIVE=never`, and `GIT_SSH_COMMAND` set to the user's command plus `-o BatchMode=yes`.
  - Unix spawns in a new process group and sends `kill -KILL -- -<pgid>`. Windows uses `taskkill /T /F`.
  - Output is drained on detached threads and read with a timeout, so an orphan holding a pipe cannot hang `wt`.
- **`remote`**:
  - `remote_destination`: the branch's upstream when it is on origin, else the same name; `None` without origin.
  - `preflight_remote_deletion`: returns `NoRemote`, `Absent`, `Present { sha, remote_only }`, or `Unavailable`. `remote_only` is `Unknown` when the live head is not in the local object store.
  - `delete_remote_branch`: `git push --force-with-lease=refs/heads/<b>:<observed> origin :refs/heads/<b>`.
- **`handoff`**:
  - `new_token()` returns 128 bits from `getrandom` 0.4 as 32 hex characters.
  - `handoff_path` rejects any other token shape, so a token cannot reach outside the cache directory.
  - `consume` deletes the file *before* judging version and age. A record older than 60 s, or dated in the future, is `Expired`.
  - `verify` checks "caller inside the target" first (exit 4), then compares repository, target, head, branch, fingerprint, and landing (exit 3). Paths are compared after `biscuit_file::canonicalize_simplified`.
- **Primitives**:
  - `remove::remove_worktree(base, path, force)` runs `check_not_in_use` on Windows first. That function renames the directory to a sibling and back, and retries the rename-back 3 × 50 ms (Decision 24).
  - `remove::remove_local_branch` is `git branch -D`.
  - The old `worktree::remove_worktree`, `delete_branch`, `DeleteBranchOutcome`, `DirtyFiles`, and `list_dirty_files` are deleted. `DirtyFiles`' rename-parsing test became `porcelain_path_reads_plain_and_renamed_entries`.
- **New errors**:
  - `WorktreeError::DirectoryInUse(path)` exits 4 (added to `exit.rs`).
  - `WorktreeError::LockProbeRenameBack { original, temporary }` exits 1, and its message names the `move` command that restores the worktree.
- **`git::git_from(base, dir, args)`** runs `git -C <dir>` with `base` as the working directory. Every removal call goes through it.
- **Cargo**:
  - `worktree/lib` gains `getrandom` 0.4 and `biscuit-file` (no default features), and turns on the `blake3` feature of `biscuit-hash` and the `remote` feature of `sniff`.
  - `worktree/cli` gains `serde_json` as a dev-dependency, for the expired-token test.
  - No crate is new to the workspace graph. `docs/dependencies.md` is updated.

### CLI (Wave 5)

`cli/src/commands/remove.rs` became `cli/src/commands/remove/{mod.rs, policy.rs, report.rs}`.

- **Flags** (`args.rs`):
  - `name: Option<String>` with `required_unless_present = "handoff"`.
  - `--force-worktree`, `--force-branch`, and `--force-remote`, with no short forms.
  - A hidden `--handoff <TOKEN>` that conflicts with `name` and every force flag.
  - `-b`, `-f`, `-ff`, and `--branch` are now clap's "unexpected argument" (exit 2).
  - `AFTER_HELP` shows the spec's examples.
- **`policy::decide(situation, flags, ask)`** is pure. The order is worktree question, then branch question, then remote step. Outcomes:
  - `Refuse(FilesNeedForce | ForceBranchNeedsWorktree)` exits 3.
  - `Cancelled` exits 0.
  - `Proceed(Actions)` carries a branch step: `Delete { approved }`, `KeepWithWarning`, or `Keep`.
  - An interrupted prompt maps to `Cancelled`.
- **`report`**: Prose and `UnorderedList` output.
  - The dirty tree shows up to 10 entries; above 10 the report shows a bold red count. The ignored groups say their contents will be deleted.
  - The tier line names its evidence ref. Remote-tracking evidence is labeled "(as of your last fetch)", and a live-verified ref "(checked on origin just now)".
  - The report also shows ahead/behind against the named target, the origin copy, the PR, and the notes. The `--force-remote` block lists remote-only commits, or says they are unknown.
  - The report ends without a trailing newline. Each question prints exactly one blank line first; the Not safe question also shows the lost commits and a blank line before the menu.
- **`dirty_tree.rs`**: the 50-file cap and "…and N more" are deleted. Source files are `<orange>` (the list's dot palette) instead of `<red>`, and names are escaped.
- **Flow (`run`)**: `find_worktree`, then refuse the main checkout, then `set_current_dir(base)`, so neither `wt` nor a git child holds the worktree. Then:
  1. If the caller is inside the target without the wrapper, exit 4 with `wrapper_setup_help()` plus "run it from another directory".
  2. Gather the facts and print the report.
  3. Decide.
  4. If the caller is inside the target, hand off; otherwise execute.
- **Execute**:
  1. Remove the worktree (`--force` when it has consented files).
  2. Handle the branch: `-D`, which appends "(N commits lost)" when it overrode an unsafe tier; the warning; or "Kept branch".
  3. Handle origin. A lease failure or an unreachable origin returns exit 1 with "removed worktree X and branch Y, but origin/Z was not deleted … Finish with: git push origin --delete Z".
- **Hand-off (first run)**:
  - The landing directory is the fork parent's worktree when the fork-origin record names a parent that has a worktree other than the target; otherwise the base repo. The caller's subdirectory is kept when it exists there.
  - The record stores approvals: `discard_files`; `BranchAction::{Delete, DeleteIfSafe, Keep}`; and `RemoteApproval { destination, observed_sha }`.
  - stdout gets `cd:<landing>` and then `remove-handoff:<token>`.
- **`run_handoff` (second run)**:
  1. Consume the token. A missing or expired token exits 4.
  2. Find the worktree by canonical path. If it is gone, exit 3.
  3. Gather fresh facts, using the cwd as the landing, and run `verify`.
  4. `DeleteIfSafe` requires the branch to be Safe or Pretty safe still; otherwise exit 3.
  5. A remote head that moved away from the observed SHA exits 3.
  6. Execute, then print "You are now in …".

### Decisions made during implementation (not in the spec)

- **The PR lookup always runs** (in parallel, capped at 2 s), even when the default branch already makes the branch Safe, because the report shows "any PR". Against an unsupported host it answers at once. On github.com, a merged branch's removal can take up to the PR round-trip.
- **A state change between the runs exits 3, not 4.** The table lists "the second run of move-first found a risk" under 3; a changed tip or file set is such a risk. An expired or missing token and a caller still inside exit 4.
- **The origin destination going absent between the runs is accepted** (nothing to delete, no loss). A destination that *appears*, or moves to another SHA, refuses.
- **Windows evidence is Windows-only L1, not L2.** Neither scenario asks a question, so neither needs a terminal:
  - `cli/tests/powershell_wrapper_exec.rs` runs the generated wrapper in `powershell.exe` launched *inside* the worktree, which is the case that holds the directory. It also runs the held-directory exit-4 case through the real binary.
  - `lib … a_held_directory_is_in_use_and_nothing_is_removed` covers the held directory at the library level.
  - These run in CI's Windows L1 cell (push to `main`), which is stronger than an L2 cell CI does not provision. There is still no Windows L2 (real console) cell for the interactive prompts in PowerShell; that part of acceptance criterion 3 is **unmet, with provisioning a Windows L2 harness as the required change**.

### Requirement-to-test mapping

| Requirement | Tests |
|---|---|
| AC1: each question after exactly one blank line (L2) | `level2_remove::level2_remove_files_question_follows_one_blank_line`, `level2_remove_not_safe_menu_defaults_to_keeping_the_branch` (both the lost-commit block and the menu), and inside `level2_move_first_through_each_wrapper_lands_in_the_fork_parent`; unit `report::tests::the_rendered_report_has_no_trailing_blank_line`. Mutation check: dropping the `eprintln!()` before the files question turned the first test red. |
| AC3 policy matrix: tier (none, Safe default, Safe merged PR, Pretty safe tag, Pretty safe remote, Not safe, Unknown) × contents (clean, dirty, `.env`, `target/`) × all 8 flag subsets × mode (non-interactive, three answer scripts) | `policy::tests::policy_matrix_follows_the_rules` (896 cases checked against the rules), `spec_examples` (every Examples row), `an_interrupted_prompt_propagates` |
| AC3 Examples table through the binary | `remove::example_clean_and_merged_removes_worktree_and_branch`, `example_pushed_without_a_pr_is_pretty_safe_and_names_the_origin_copy`, `example_unique_commits_without_a_terminal_keep_the_branch_with_a_warning`, `example_force_branch_deletes_unique_commits`, `example_dirty_files_without_a_terminal_refuse_with_nothing_removed` (CI unset and `CI=true`), `example_all_three_flags_remove_worktree_branch_and_origin_branch` |
| AC3 ignored entries need consent; conflict; remote failure; detached | `ignored_entries_need_consent_like_dirty_files` (`.env`, `target/`), `force_branch_on_a_dirty_worktree_without_force_worktree_is_a_conflict`, `force_remote_with_an_unreachable_origin_removes_locally_then_exits_1`, `force_remote_without_a_remote_branch_says_so`, `force_remote_keeps_a_branch_whose_only_other_copy_it_deletes`, `a_detached_worktree_depends_only_on_its_files`, `the_report_names_ahead_behind_against_the_selected_target` |
| AC3 flag surface and retirements | `retired_flags_are_clap_errors` (`-b`, `--branch`, `-f`, `-ff`, `--force`, `--remove-branch`, `--remove-remote`), `help_lists_the_three_force_flags_and_hides_handoff`, `handoff_conflicts_with_a_name_and_every_force_flag`, `an_unknown_name_fails_and_the_base_checkout_is_refused` |
| AC3 PR evidence: same repository and exact tip only; unavailable falls through | `safety::tests::a_pr_counts_only_for_the_same_repository_and_exact_tip` (merged, open, case-insensitive repository, fork mismatch, post-merge commits, missing head, 12-character prefix, too-short and uppercase prefixes), `an_unavailable_pr_answer_falls_through_to_lower_tiers`, `force_remote_ignores_the_open_pr_but_not_a_merged_one` |
| AC3 stale or deleted `origin/*`, and an unreachable remote, never justify deletion | `own_origin_copy_is_pretty_safe_after_the_live_check`, `a_remote_copy_that_moved_on_with_the_tip_in_it_still_qualifies_only_if_tracked`, `a_stale_or_deleted_origin_ref_never_justifies_deletion`, `an_unreachable_remote_keeps_the_branch`, `another_origin_branch_is_verified_live`, `force_remote_excludes_the_own_origin_copy_from_tier_and_lost_commits`, `merged_into_origin_main_only_is_safe_without_a_live_check`, `another_local_branch_or_tag_is_pretty_safe_offline`, `unique_commits_are_not_safe_and_listed`, `a_ref_search_failure_is_unknown` |
| AC3 lease: a push between preflight and deletion fails and keeps the new head | `remote::tests::a_push_between_preflight_and_deletion_fails_the_lease_and_keeps_the_new_head`, `deletion_succeeds_under_the_observed_lease`, `preflight_reports_absent_present_and_remote_only_commits`, `destination_is_the_origin_upstream_else_the_branch_name`, `an_unreachable_origin_is_unavailable` |
| AC3 live check never prompts and dies at its deadline | `live_remote::tests::reads_live_heads_and_reports_absent_branches`, `a_missing_origin_is_an_error_not_an_absent_branch`, `the_deadline_kills_a_hung_transport` (Unix: an SSH command that sleeps 30 s is killed at 0.5 s) |
| AC3 move-first: binary (L1) | `standing_inside_without_the_wrapper_exits_4_and_removes_nothing`, `standing_inside_with_the_wrapper_hands_off_then_finishes_from_the_landing` (keeps the subdirectory; a replay exits 4), `the_handoff_lands_in_the_fork_parent_worktree`, `a_changed_branch_tip_between_the_runs_refuses_with_nothing_removed`, `a_new_ignored_entry_between_the_runs_refuses`, `an_expired_token_refuses_with_exit_4`, `a_caller_still_inside_the_target_is_refused_with_exit_4`, `a_missing_or_malformed_token_refuses_with_exit_4`, `the_handoff_carries_force_flags_to_the_second_run`, `a_branch_that_stops_being_safe_between_the_runs_refuses` |
| AC3 move-first through real bash, zsh, and fish wrappers (L2, tmux) | `level2_move_first_through_each_wrapper_lands_in_the_fork_parent` (dirty worktree: the prompt works while stdout is captured, the shell lands in `feat-theme/docs`, and the output says so), `level2_move_first_lands_in_the_base_repo_without_a_fork_record`, `level2_move_first_failures_leave_the_worktree_intact` (failed `cd`, expired token, and changed tip, each in all three shells) |
| AC3 Windows: held directory exits 4 with nothing removed; PowerShell launched inside the worktree | `remove::tests::a_held_directory_is_in_use_and_nothing_is_removed` (lib, `cfg(windows)`), `powershell_wrapper_exec::a_directory_held_by_another_program_exits_4_with_nothing_removed`, `move_first_removes_the_worktree_powershell_was_launched_inside`. Portable: `an_unheld_directory_passes_the_lock_check_untouched`, `a_missing_directory_is_reported_as_in_use_before_git_runs` |
| AC4: merged into HEAD but not its upstream | `safety::tests::merged_into_head_but_not_its_upstream_is_safe`, `remove::a_branch_merged_into_head_but_not_its_upstream_is_deleted` (asserts no "preserved") |
| Inventory and fingerprint | `inventory::tests::parses_every_status_kind_and_skips_rename_origins`, `clean_output_needs_no_consent`, `ignored_entries_group_by_first_component`, `collects_dirty_and_ignored_entries_from_a_real_worktree` (a directory pattern and a `**/build/*` pattern), `fingerprint_changes_with_content_status_new_paths_and_ignored_entries` |
| Handoff record | `handoff::tests::tokens_are_32_random_hex_characters`, `a_malformed_token_never_names_a_file`, `round_trip_then_replay_is_missing` (write/read/write/read), `an_expired_or_future_record_is_refused_and_still_deleted` (the TTL boundary), `a_corrupt_or_other_version_record_is_missing`, `every_changed_field_refuses`, `a_caller_still_inside_the_target_is_refused` (including a sibling whose name extends the target's), `paths_compare_canonical` |
| Default-branch target | `default_target::tests::local_only_selects_the_local_tip`, `missing_default_branch_selects_nothing`, `the_descendant_wins_in_both_directions`, `diverged_tips_select_origin_and_say_so` |
| Report text | `report::tests::*` (7 tests), `dirty_tree::tests::every_path_is_rendered_and_markup_in_names_is_escaped`, `exit::tests::*` (DirectoryInUse → 4, LockProbeRenameBack → 1) |

Placement:

- Every L1 test sits in a lib or bin unit module, or in an auto-discovered `cli/tests/*.rs`. `worktree-cli` does not set `autotests = false`.
- `level2_remove.rs` is declared as a `[[test]]` with `required-features = ["terminal-tests"]`, which is in `[package.metadata.ci.tests] features`. Every function in it is named `level2_*`.
- `just check-tier-coverage worktree` reports 0 stranded tests.
- `powershell_wrapper_exec.rs` is `#![cfg(windows)]` and L1.
- The obsolete `level2_dirty_tree::level2_remove_dirty_worktree_shows_tree_and_prompt` asserted the old "source code files" wording. It is deleted, and `level2_remove` supersedes it.

### Gates

- `just test` (worktree): 264 passed, 11 skipped. The same 11 were skipped in Phase 2. This is the final run, after the dead `DirtyFiles` tests were removed.
- `just test-l2` (worktree): 9 passed, including all 5 `level2_remove` tests. The Kitty image test skips here because Kitty is not installed. With `BISCUIT_TEST_LEVEL_REQUIRED=2` it fails for that reason, which is a provisioning matter and predates this phase.
- `just lint` (worktree): clean. `cargo clippy -p worktree-cli --tests --features terminal-tests -- -D warnings`: clean.
- `just cross-check worktree --os linux`: pass, 97 tests. `just cross-check worktree-cli --os linux`: pass, 167 passed, 20 skipped.
- `just cross-check worktree-cli --os windows`: pass, 165 passed. Both PowerShell tests and all 28 `remove` tests ran on native Windows.
  - That first run exposed a slow holder: `cmd /C ping` left `ping` alive for 30 s after the kill. Both Windows tests now spawn `ping` directly with null stdio.
  - Re-run after the fix: `just cross-check worktree --os windows` passed (95 tests, including `a_held_directory_is_in_use_and_nothing_is_removed` in 1.5 s), and `just cross-check worktree-cli --os windows` passed (165 passed, 15 skipped; the held-directory test now takes 2.0 s).
  - The fact is recorded in the `os` skill's `windows.md` under "Current-directory locks".
- WSL2 was not cross-checked. Nothing here is WSL-specific beyond Linux, which passed, and the nightly schedule covers WSL2.
- No `cargo fmt` was run.

### Code search (validation checkpoint)

`delete_branch`, `DeleteBranchOutcome`, and `FORCE_BYPASS_FILE_LIMIT` no longer appear under `worktree/`, in the worktree skill, or in `docs/`. The branch primitive is named `remove_local_branch` so that a substring search stays clean. `-ff` appears only in `retired_flags_are_clap_errors`, which asserts that clap rejects it. `--no-ff` and `--ff-only` are git arguments in test fixtures.

### Docs and skill

- `worktree/README.md`: the `wt remove` section is rewritten for the tiers, the flags, report-first, move-first, and the Windows check.
- `docs/dependencies.md`: the worktree note now covers the handoff records, `getrandom`, the `blake3` feature, `sniff/remote`, and `biscuit-file`. The `getrandom` catalog entry names the new use.
- `.claude/skills/worktree/SKILL.md`: new "`wt remove`" section, and `DirectoryInUse` added under exit code 4.
- `.claude/skills/os/windows.md`: a lock-holder test must spawn the holder directly, never through `cmd /C`.

## Phase 4

Dependencies for the table and graph: the `mermaid-rs-renderer` 0.3.1 upgrade, `Table::highlight_row`, sniff's `open_pull_requests`, `ImageWidth::Scale`, the `GitGraph` component with fit-by-trimming, and the upstream drafts. The sniff entry point and the `Table` highlight were built by two parallel subagents. Everything else was done in the main session.

### Starting conditions

- Decisions 20–33 are still **proposed** (the spec's `human_review` from Phase 3 is unanswered). This phase builds on Decision 32 as proposed: the natural width comes from `measure_svg_dimensions` (`viewbox_width`), measured with biscuit's theme.
- `just test` in `worktree` was green at the end of Phase 3.

### mermaid-rs-renderer 0.3.1 (Wave 7)

- `biscuit-visualized/src/Cargo.toml`: `mermaid-rs-renderer = { version = "0.3.1", default-features = false }`.
  - **Deviation from the plan's wording.** The plan expected resvg/usvg to move to 0.47 and tiny-skia 0.12 to be added. Those are the crate's `png` feature. biscuit rasterizes with its own resvg 0.45 and never used the crate's PNG path (the `cli` feature pulls clap too). With default features off, the lockfile **drops** resvg 0.46, usvg 0.46, kurbo 0.13, svgtypes 0.16, roxmltree 0.21, and imagesize 0.14 (the duplicates 0.2.1 already carried), and nothing new is added. Spike S5 had recommended considering this.
- **Pie contrast.** 0.3 derives the default pie palette as `hsl(…)`; the third default slice is `hsl(0, 0%, 60%)`, a mid gray.
  - `fix_pie_text_contrast` now parses `#rgb`, `#rrggbb`, and `hsl(h, s%, l%)` (`parse_css_color`, `hsl_to_rgb`).
  - **Rule change.** Parsing alone would have put light text on the gray at 2.3:1 contrast (the old rule was "luminance > 0.4 means dark text"). Each label now takes whichever of the two label colors has the higher WCAG contrast ratio against its slice. This flips one existing expectation: the TypeScript blue `#3178C6` is a near tie (3.9:1 dark against 3.6:1 light) and now gets dark text. `mermaid_pie_chart_init_directive_applies_custom_colors` asserts the new result, and its white-slice comment is replaced.
  - **Doc drift fixed.** The helper's doc said "> 0.5" while the code used 0.4. The doc now describes the contrast rule.
- `MermaidDiagram::natural_size()` (biscuit-visualized) returns `NaturalSize { width, height }` in SVG units from `measure_svg_dimensions(...).viewbox_{width,height}`, using the same parse, theme, `%%{init}%%` overrides, and layout as `render`. `render_svg` and `natural_size` share the new `compute_layout`.
- The 0.3 parser now rejects input without a diagram header and malformed `%%{init}%%` lines. No fixture in biscuit-visualized, biscuit-terminal, or darkmatter tripped on this.
- Docs: `docs/dependencies.md` (a note and a catalog entry under Image Processing) and `visualizing-graph-expressions.md` (0.3.1, the `png` feature's 0.47 deps, and the header rule).

### `Table::highlight_row` (Wave 7, subagent)

- `Table::highlight_row(row, color)` with the typed slot `TableStyle::highlight_row: Option<TableRowHighlight>`.
  - The index is the 0-based body row. An out-of-range index is a no-op, and a second call replaces the first.
  - On its row the highlight beats both stripe colors. Cell foreground and bold are kept, and the background is restored after each SGR reset inside a cell.
  - The background covers padding and inner separators but not the outer borders. It is degraded to 256 and 16 colors like stripes, and dropped when the terminal has no color.
  - Browser and Markdown output ignore it, as they ignore striping.
- **`renderable` is touched too.** The default `Table::render` goes through the render tree, and striping reaches it through `renderable::tree::TableTerminalHints`. `TableRowHighlight` and a `highlight_row` hint field (serde default, skipped when `None`) follow the same path.
- **Doc drift fixed.** The striping docs in `table.rs` and `types.rs` said "even data rows"; the code stripes 0-indexed rows 1, 3, 5. The docs now say "every second data row (0-indexed rows 1, 3, 5, ...)". The subagent also reported that `lib/src/components/table/README.md`'s field table describes the `alternate_*` fields as `bool`s that need true color. That is left for a docs pass.

### sniff: `open_pull_requests` (Wave 7, subagent)

- `sniff::remote::blocking::open_pull_requests(remote_url, deadline) -> Result<Vec<PrSummary>, PrUnavailable>`, plus `open_pull_requests_with(&FocusedProviderClient, deadline)`.
  - `PrSummary { number, html_url, source_repo, source_branch, target_branch }` is `#[non_exhaustive]`. `source_repo` is `owner/repo` (the GitLab project path), and `None` when the provider cannot name it.
  - It reuses Phase 2's `PrUnavailable`, `run_with_deadline`, `client_for_url`, and `classify`, and refuses to run inside a Tokio runtime.
- **Errors, never an empty list:** 401/403 is `Auth`; a 404 on the list (GitHub's private-repository case) is `NotFoundOrNotPermitted`; a missed deadline is `Timeout`; more than `MAX_PAGES` (20) pages is `Other`, rather than a shortened list.
- Paging follows each provider's next-page rule. Rows are filtered to open state again locally.
- **Deviation from the spec's wording, as in Phase 2.** The spec says to build on the async `list_pull_requests` provider method. Like Phase 2's `pull_request_for_branch`, this uses `FocusedProviderClient` instead, because `list_pull_requests` reads one page and folds a list 404 into an empty list (spike S1).
- GitLab fork MRs name their source project only by ID, so each distinct fork costs one `GET projects/{id}`. A 404 there leaves `source_repo: None`, and any other error fails the list.
- The paging loop moved from `branch_pull_requests` into a shared private `pr_list_rows`. All `pr_for_branch` tests still pass. The `pr_for_branch.rs` fixture helpers became `pub(super)` so both test modules can use them.

### `ImageWidth::Scale(f32)` (Wave 8)

- `ImageWidth::scaled_columns(scale, natural_width, cell)` implements the formula: pixels per unit = scale × cell height ÷ `SCALE_REFERENCE_TEXT_UNITS` (16), and columns = ⌈natural width × pixels per unit ÷ cell width⌉, at least 1. A zero cell uses `CellSize::FALLBACK`. NaN and negative values give 1.
- `TerminalImage::resolve_scaled_dimensions_for(width, layout, term_width, natural_width, cell)` clamps to the available columns after margins. `resolve_dimensions_for` delegates to it with no natural width, so `Scale` there behaves like `Fill`.
- **One fallback constant.** `CellSize::FALLBACK` (8×16) replaces the `unwrap_or((8u32, 16u32))` literals in the files this phase touched (`kitty.rs`, `iterm.rs`, `protocol.rs`, and `mermaid.rs`). The `graph_expression.rs` literal is left alone, since that file is otherwise untouched.
- **Rasters.** The Kitty, iTerm, and inline paths now load the image before resolving, so `Scale` on a raster uses its pixel width. iTerm's width parameter sends the resolved cell count for `Scale`.
- **`MermaidDiagram`** now defaults to `Scale(1.0)` and gains `resolve_dimensions(term_width, cell)`, which measures the SVG only for `Scale`. The PNG is shown with `ImageWidth::Characters(columns)`, because re-resolving a scale against the raster's pixel width would give a different size.
  - `MermaidRenderer::natural_size()` wraps the biscuit-visualized call. `terminal_theme()` is the extracted color-mode theme choice, now shared with `GitGraph`.
- **Audit of the changed default** (callers relying on `MermaidDiagram::new`'s width):
  - `darkmatter::mermaid::render_terminal::render_for_terminal`, render-tree Mermaid promotion (`render_tree/render.rs`), and every `bt` diagram command without `--width` now draw at body-text size instead of 50% width. This is the ruled behavior.
  - `wt list` passes an explicit width, so it is unchanged until Phase 5.
  - `darkmatter`'s `render_to_svg` and `fallback_code_block` callers are unaffected.
  - `biscuit-terminal/cli/README.md` said "(default: 50%)" and is corrected.
- Two exhaustive matches needed an arm. In `worktree/cli/src/commands/list.rs`, `Scale` fits like `Characters`; `--width` never produces `Scale`, and Phase 5 rewrites this code. In `bt`'s `parse_column_width`, `Scale` is rejected like `Fill`.
- **Doc drift fixed.** `Terminal::cell_size()`'s doc said the fallback was a CSI 14t query. The code tries the `TIOCGWINSZ` pixel fields first, then CSI 14t, and always returns `None` on native Windows.
- `parse_width_spec` has no string form for `Scale` (not required).

### `GitGraph` (Waves 8–9)

`biscuit-terminal/lib/src/components/git_graph.rs` (behind the `image` feature, re-exported from the prelude) and `git_graph/tests.rs`.

- **Typed input:**
  - `GitGraph::new(default_branch, Vec<LaneEntry>)`, where a `LaneEntry` is `Commit(full sha)` or `Elided(n)`.
  - `GraphLine { branch, parent, fork_sha, entries, created_at, last_active }`.
  - `with_ref(name, sha)`, `GraphPullRequest { number, source_branch, target_branch }`, and `with_current_branch`.
  - `with_scale` (default `DEFAULT_GIT_GRAPH_SCALE` = 1.25), `with_width`, and `with_theme`.
- **Lane/tag rule:**
  - Lanes are the default branch, the current branch, its non-default fork parent, and `origin/<default>` when it is passed as a line with commits of its own. In the base view (the default branch checked out, or no current branch), every line with commits of its own gets a lane.
  - Every other drawn ref tip is a tag. A lane's own tip is not tagged with its own name, and lines with no commits of their own tag their fork commit.
  - A PR is a tag `PR #n → target` on its source branch's tip.
- **Lane order.** Siblings forking at the same commit follow `created_at` (lines without one come last), then input order. There are no `order:` attributes.
- **Renderer facts found here** (in 0.3.1's parser, not in the spec):
  - The parser always names the first lane `main`; `mainBranchName` is honored only when there are no branches. So the default branch is always that lane, and its real name appears only as a tag. A non-default branch named `main` is drawn as `main~` (`~` cannot occur in a git branch name).
  - Parents are looked up by commit ID, and IDs are labels. Short IDs are therefore lengthened past 7 characters where two SHAs share a prefix, and a repeated `+N` gets trailing spaces, which do not show. Before this, `wt`'s graph could emit duplicate `+N` IDs.
- **Fitting** (`plan` / `plan_with(viewport, measure)`):
  - Height (base view only): past `rows / 2`, keep the default lane (and a diverged `origin/<default>`), then add lanes most recently active first, each with its drawn ancestors, until the next would not fit. `hidden_lanes` produces the dim "N more worktree(s) not shown" line.
  - Width: while columns exceed the viewport, fold one unpinned commit (not a lane tip, fork point, or tagged commit) into a `+N` square. A commit beside an existing square goes first, then the oldest commit on the lane showing the most commits; ties go to the earlier lane. When nothing can be trimmed, the image shrinks through `MermaidDiagram`'s clamp.
  - An explicit non-scale `with_width` is never trimmed to. `with_width(Scale(s))` replaces the scale.
  - **A bug the fake measurer caught.** The first version always took the oldest commit on the longest lane. Converting a lone commit into a `+1` square saves only its label's width, and ties then spread `+1` squares across lanes (4 trims where 2 merges were enough). The merge-first rule fixed it.
- **Measurement is injected.** `plan` measures with biscuit-visualized's `natural_size` in the render theme. Text widths come from system fonts (`fontdb`), so exact sizes vary by OS. L1 tests pin exact decisions with a fake measurer, and one test checks relations only with the real renderer.
- **Rendering:**
  - Terminal: the fitted plan through `MermaidDiagram` at `Scale(scale)` (or the override), with the layout copied, followed by the hidden-lanes note.
  - Tree: `MermaidDiagram`'s projection of the untrimmed text.
  - Browser: the untrimmed SVG as a raw-HTML island in the Default theme unless one is set, with a `<pre><code class="language-mermaid">` fallback.
- **Visual check** (a throwaway example, deleted afterwards): the spec example rendered to PNG shows three lanes in creation order, the `main` and `origin/main` tags, `PR #104 → feat/theme` on `feat/dark-fixes`, and the `+4` square. Its natural size is 626 × 234 units. At 125% with an 8×16 cell that is 98 × 19; the spec estimated 104 × 19. The fully trimmed version (three `+1` squares with distinct IDs, plus `+6`) also connected correctly and narrowed to 542 units.
- `worktree/docs/git-graph.md` is an older, never-built `GitGraph` design. Phase 5 rewrites it, as the plan says. It was not edited here.

### Upstream drafts (Wave 9)

- `upstream-issue.md`: a minimal reproduction, expected and actual results, the cause, the fix, and a lower-priority note about the hard-coded `main` lane.
- `upstream-pr.patch`: a diff against the published 0.3.1 source (`src/parser.rs` only). It adds `gitgraph_branch_name`, which takes the first token or a quoted string, applied to `branch`, `checkout`/`switch`, and `merge`, plus two parser tests.
  - Verified on a scratch copy in `/tmp`, since deleted. The published crate omits fixture files its own lib tests `include_str!`, so empty placeholders were added there. The two new tests fail on 0.3.1's behavior and pass with the patch, and the existing `parse_gitgraph_basic` still passes. The "Actual" output quoted in the issue was captured from 0.3.1, not inferred.
  - **Nothing was filed.**

### Requirement-to-test mapping

| Requirement | Tests (all L1) |
|---|---|
| 0.3.1 upgrade passes biscuit-visualized's suite, including the fixed pie-contrast test | biscuit-visualized `just test` (77); `tests::mermaid_tests::mermaid_pie_chart_init_directive_applies_custom_colors` (the original failing input: default third slice `hsl(0, 0%, 60%)`) |
| `hsl()` parsing and the contrast choice, including malformed input | `mermaid::render::tests::parses_hex_and_hsl_colors`, `rejects_malformed_colors`, `label_color_follows_the_higher_contrast_ratio` |
| Natural size equals the rendered `viewBox` (Decision 32), for both the 16-unit and 14-unit themes | `tests::mermaid_tests::mermaid_natural_size_matches_the_rendered_viewbox`, `mermaid_natural_size_rejects_unparseable_input`, `mermaid_natural_size_grows_with_commits` |
| Downstream Mermaid, diagram, and parity tests | biscuit-terminal filter `scale\|scaled\|mermaid\|diagram\|parity\|width`: 1,270 passed. darkmatter `test(~mermaid)`: 73 passed |
| `ImageWidth::Scale` computed sizes | `terminal_image::tests::scaled_columns_follow_the_cell_height_rule` (8×16, 10×20, 16×32, 125% rounding), `scaled_columns_never_drop_below_one`, `scaled_columns_treat_a_zero_cell_as_the_fallback`, `scale_resolves_from_the_natural_width_and_clamps_to_the_available_columns` (including margins), `scale_without_a_cell_size_uses_the_8x16_fallback`, `scale_without_a_natural_width_fills_the_available_columns`, `other_widths_ignore_the_natural_width`, `legacy_display_dimensions_scale_the_pixel_width` |
| `MermaidDiagram` defaults to `Scale(1.0)` and sizes from the measured SVG | `mermaid::tests::diagram_defaults_to_scale_one`, `scaled_columns_come_from_the_measured_svg_width`, `a_narrow_terminal_caps_the_scaled_width`, `a_scaled_diagram_that_does_not_parse_is_an_error` |
| Row highlight | `table::table::tests::highlight_row_*` (9 tests) and snapshot `l1::table_parity::table_highlight_row_with_striping_snapshot`; renderable `table_terminal_hints_round_trip`, `table_terminal_hints_omit_absent_highlight_when_serialized` |
| Open-PR list on all four providers; auth failure and timeout distinct from empty | `open_pull_requests::*` (11 tests; see the subagent report above) |
| `GitGraph` lane/tag rule on the spec's example (standing in `feat-dark-fixes`) | `git_graph::tests::the_spec_example_emits_the_spec_text` (exact text), `branch_and_checkout_statements_carry_no_attributes`, `a_focused_view_draws_no_lane_for_an_unrelated_branch`, `a_branch_already_in_the_default_branch_is_a_tag`, `the_base_view_gives_every_branch_with_commits_a_lane`, `origin_default_is_a_tag_until_it_diverges`, `a_default_branch_not_named_main_is_the_root_lane_and_a_tag`, `pull_requests_tag_their_source_tip_and_skip_undrawn_branches`, `lanes_forking_at_one_commit_follow_creation_order`, `a_fork_point_outside_the_drawn_commits_hangs_from_the_lane_start`, `ids_are_unique_even_when_short_shas_or_elisions_repeat`, `quotes_in_ref_names_cannot_break_a_tag`, `a_default_lane_without_commits_draws_nothing`, `every_emitted_graph_parses` |
| Trimming decisions and computed sizes | `a_graph_that_fits_is_not_trimmed_and_sizes_from_the_scale`, `the_width_cap_trims_commits_before_anything_shrinks`, `trimming_stops_when_only_pinned_commits_remain`, `an_explicit_width_is_never_trimmed_to`, `a_scale_width_replaces_the_scale`, `the_base_view_height_cap_keeps_the_most_recently_active_lanes`, `the_height_cap_adds_lanes_in_activity_order_until_one_does_not_fit`, `a_focused_view_is_never_cut_by_the_height_cap`, `a_failed_measurement_trims_nothing`, `the_viewport_comes_from_the_terminal_after_margins`, `measured_sizes_trim_to_the_width_cap` (real renderer) |
| Terminal, tree, and browser outputs | `without_image_support_the_terminal_gets_the_code_block_and_the_lane_note`, `the_tree_projection_carries_the_untrimmed_source`, `the_browser_output_is_an_svg_island` |

Placement: every new biscuit-terminal and biscuit-visualized test is a lib unit test (`#[cfg(test)]`, compiled by `--features image` / `--all-features`, as each justfile runs them). The sniff tests sit in the declared `tests/l1/` binary under `#[cfg(feature = "remote")]`, which is in sniff's CI features. The table snapshot is in the existing `l1::table_parity`. No test name has a tier marker. A draft name, `real_measurements_…`, would have left L1 and was renamed to `measured_sizes_trim_to_the_width_cap` before it ever ran.

### Gates

- **The first gate run tested the wrong tree.** The gate script looped `cd "$area" && just test`. This host's `CDPATH` lists the main checkout and has no leading `.`, so Bash sent every `cd` to `/Users/ken/coding/personal/rusty-biscuit/<area>`, and all 12 gates "passed" there. The first log line and the test counts gave it away (biscuit-visualized 71 instead of 77). The rerun used `unset CDPATH` and absolute paths, and each log starts with the worktree path. The trap is recorded in the `os` skill (`macos.md`, plus an index line in `SKILL.md`).
- The final run is all green, in this worktree:

| Area | `just test` | `just lint` |
|---|---|---|
| biscuit-visualized | 77 passed | clean |
| biscuit-terminal (lib + CLI) | 3314 passed, 55 skipped | clean |
| sniff (lib + CLI) | 2857 passed, 31 skipped | clean |
| renderable | 546 passed | clean |
| worktree (lib + CLI) | 264 passed, 11 skipped (the same 11 as Phases 2–3) | clean |
| darkmatter | 8498 passed, 12 skipped | clean |

- `just check-tier-coverage` reports 0 stranded tests for biscuit-terminal, biscuit-visualized, sniff, renderable, and worktree.
- **Cross-OS** (`just cross-check`):
  - biscuit-terminal on native Windows: 2973 passed, including all 28 `git_graph::tests` (the real-renderer sizing test among them). That host has no cell size, so this exercises the 8×16 fallback.
  - biscuit-terminal on Linux: archive mode failed before any test ran. Its release `ci-build` compile hit a read-only `librenderable-*.rmeta` in the standing clone, the kache hardlink trap already described in `build-hosts.md`. `--features image` (the native path) then passed: 3043, including the 28 `git_graph` tests. The stale links remain in that clone. This is noted in `build-hosts.md`, but not cleared, because that needs manual deletion on the shared host.
  - biscuit-visualized needs a build flag for cross-check (the plan names no ubuntu-latest build for it). With `--all-features`: 77 passed on both Linux and native Windows.
  - WSL2 was not cross-checked. Nothing here is WSL-specific, and the nightly schedule covers WSL2.
- No `cargo fmt` was run. A formatting hook in this environment reformatted `git_graph.rs` and `git_graph/tests.rs` after they were written; that was not a `cargo fmt` invocation by this session.

### Docs and skills

- New: `biscuit-terminal/docs/components/git_graph.md`, and an index row in `docs/components/index.md`.
- Updated: `mermaid_diagram.md` (the `Scale(1.0)` default and a Sizing section), `terminal_image.md` (`Scale`, `scaled_columns`, `resolve_scaled_dimensions_for`), `table.md` and the table README (subagent), `biscuit-terminal/cli/README.md` (the default diagram width), `visualizing-graph-expressions.md`, `docs/dependencies.md`, and sniff's README and CHANGELOG (subagent).
- Skills:
  - `biscuit-terminal/components.md` (the `GitGraph` row and `MermaidDiagram`'s default) and `image-rendering.md` (`Scale` and `CellSize::FALLBACK`).
  - `renderable/tree.md` (the table highlight hint), and `sniff/SKILL.md` plus `remote-and-repository.md` (open-PR listing).
  - `os/macos.md` and `os/SKILL.md` (`CDPATH`), and `os/build-hosts.md` (the stale kache links in the standing clone).
- **The worktree skill is unchanged.** `wt` does not use `GitGraph` or `open_pull_requests` yet (that is Phase 5). The one `wt` source change is a match arm.
- **Observed and not fixed** (out of scope): `.claude/skills/biscuit-terminal/mermaid-diagrams.md` still describes rendering through the `mmdc` CLI and recommends installing `@mermaid-js/mermaid-cli`. Rendering has been pure Rust through biscuit-visualized for some time.

## Phase 5

The `wt list` table and graph (item 5): the generalized comparison cache, the list data model, the PR store, the redesigned table, and the handoff to `GitGraph`.

### Starting conditions

- Decisions 20–33 are still **proposed**; this phase uses Decision 29 (terminal rows) through `GitGraph`'s existing `GraphViewport::for_terminal`.
- Shell scripts `unset CDPATH` and use absolute paths (the Phase 4 trap).

### Library (Wave 10)

- `cache.rs`: `CacheKey { target_tip_sha, branch_tip_sha, version }`, `CACHE_FORMAT_VERSION = 2`. A version-1 file (the old `default_tip_sha` key) loads empty (`cache_written_with_the_version_one_key_shape_loads_empty`).
- `default_target.rs`: the rule moved into the pure `choose_default_target(default, local, remote, contains)`; `select_default_target` (used by `wt remove`) calls it with `merge-base --is-ancestor`. `wt list` answers `contains` from the cached caption counts, so a warm list runs no `merge-base`.
- New `listing.rs`:
  - `RefTips` from one `for-each-ref --format=%(objectname) %(refname) refs/heads refs/remotes` (symbolic `origin/HEAD` skipped). It replaces `default_tip_sha`, which is deleted.
  - `compare_cached(cache, target_sha, branch_sha)`: `rev-list --left-right --count` and a speculative `merge-tree` on full SHAs, in parallel. A `rev-list` failure is `None` and is not cached (the old code turned it into "0 ahead, 0 behind, clean").
  - `Comparison::merge_state` (`AlreadyIn` when ahead = 0, else `Clean`/`Conflicts`), `Caption`/`CaptionState` (in sync, ahead, behind, diverged), `ParentComparison` (`NotApplicable`, `Deleted`, `Compared`).
  - `build_tree` (pure): default branch first, records nest branches under parents, parent rows without worktrees, deleted parents as roots with dotted children, no-record branches at the root in worktree order, detached rows last. Siblings follow `created_at`, then name. A record cycle is cut so no branch disappears; two worktrees on one branch give two rows.
- `worktree.rs`: `WorktreeStatus` is now `{ entry, dirty }`; `WorktreeList` gains `target`, `caption`, `tree`, `comparisons` (by branch), `refs()`, and `fork_origins()`. `fill_worktree_statuses` spawns the dirty walks first, then the caption, the target, and one thread per branch comparison. The `parse_worktree_state` / `fill_worktree_statuses` seam is kept.
  - Fork records of deleted branches are pruned in the same pass, **only after a successful `for-each-ref`** (an empty ref set would make every record look deleted).
- New `pull_requests.rs`: `<repo hash>.prs.json` (format 1, `fetched_at`, `source_repo`, PRs). `open_pull_requests(store, now, connect)` returns stored results inside the 60 s window without calling `connect` (so no git call either); otherwise one request under 300 ms; on failure the stored results with `stale = true`; a failure is never written. `PrListing::for_branch` matches source repository (ASCII case-insensitive) **and** branch; a PR without a source repository matches nothing. `placement(target, parent, default)` picks the parent column, the default column, or beside the branch. `SniffOpenPrSource` wraps `sniff::remote::blocking::open_pull_requests`.
- Skill: the worktree skill's cache paragraph is replaced by a `wt list` section (cache key, seam, prune guard, PR store).

### CLI rendering (Wave 11)

- New `cli/src/commands/list_table.rs` (`pub`, so `tests/list_table.rs` can snapshot it; snapshots cannot live in the shared modules' unit tests, which compile twice). It is pure over `TableFacts` (from `WorktreeList` plus the `PrListing`):
  - Caption: `[main] is N commits behind/ahead of [origin/main]`, `is in sync with`, and `has diverged from [origin/main]: N commits ahead, M commits behind`; only the counts are yellow; singular "commit"; no caption without both refs.
  - Worktree column: `○` dim, `●` yellow (other files), `●` orange (source); `base repo` dim italic (bold italic when current); the current name bold.
  - Branch column: guides and `├─`/`└─` connectors from the tree rows, gray when the branch merges cleanly into its parent (the target comparison under the default branch), red when it conflicts, dim `├┄`/`└┄` under a deleted parent; the default branch as a local badge; a deleted parent dim, italic, struck through, plus `(deleted)`; `detached @ <sha>` dim italic.
  - Target columns: the header is `-> ` plus a remote badge (`origin/…`) or a local badge; cells `already in`/`clean` (dim italic), `conflicts` (red), `—` (dim) on the default and detached rows, `parent deleted` (gray), empty for a parent row without a worktree, and `?` when git failed (never a fake answer).
  - Badges: local blue-800, remote violet-800, PR emerald-800, one space of padding. PR badges follow `placement`.
  - Legend (two lines) and the dim `PRs as of N min ago` line (only for stale stored results; `less than a minute`, `N h`, `N days` variants).
  - The current row is highlighted with `Table::highlight_row` (rgb 38,42,54 dark / 234,238,246 light).
- `list.rs`: `run_pipeline` spawns the PR thread (`pr gather` stage) and the graph/verbose thread right after `parse_worktree_state`, runs `fill_worktree_statuses`, then renders caption/table/legend, the graph, and the verbose section. The PR source is injected (`PrConnect`); unit tests pass `no_prs`.
- `git_graph.rs` rewritten: `GatherInput::from_list`, `gather(input, needs_graph, needs_verbose) -> (Option<GraphFacts>, Option<VerboseData>)`, and `GraphFacts::to_git_graph(prs, width)`. Deleted: `CommitId`, `display_sha`, `BranchGraphData`, `BaseGraphData`, `worktree_graph`, `base_graph`, `elision_commit`, `default_graph_width`, `MIN_GRAPH_TERMINAL_WIDTH`, `graph_eligible`. Verbose helpers (`format_commit` and friends) are unchanged.
  - The default lane ends at the descendant default tip (one `merge-base` when local and origin differ); a diverged `origin/<default>` is a line of its own; both tips are refs. The focused view adds a line for a recorded non-default fork parent; the base view nests a line under its recorded parent when the parent is drawn. `last_active` is the tip's commit time (from `%ct` in the same `log`), `created_at` from the fork record. A `+N` square is counted only when a line fills its 5-commit window.
  - PRs reach `GitGraph` only through `PrListing::for_branch`, because `GitGraph` matches by branch name alone.
- `worktree/docs/git-graph.md` is rewritten for this design (the old text described a component that was never built). `worktree/docs/performance-testing.md` is corrected where it named deleted functions and the old cache key, and gains a "PR Request" section.

### Decisions made during implementation (not in the spec)

- **No minimum terminal width for the graph.** The old 80-column gate (`MIN_GRAPH_TERMINAL_WIDTH`) is removed: the spec's sizing rule says a narrow terminal shows a smaller graph, which `GitGraph` does by trimming and then clamping. `run_pipeline_gathers_the_graph_on_a_narrow_image_terminal` replaces the gate's tests.
- **PR badges are links only where the terminal shows OSC 8 links.** The spec keeps Prose's degradation of links to visible `[text](url)` on other terminals. With it, `Table` (which never breaks inside a word) needed 128 content columns for the spec example and printed "Table could not be rendered in 120 columns" instead of the table. The badge text stays; only the URL is dropped (`pr_badges_link_through_osc_8_and_print_no_url_without_it`).
- **Square table corners.** The spec's mock-up shows rounded corners; `Table` has no rounded border style, and the corners are not among the ruled cell/caption/badge/legend facts. Not changed.
- **Verbose follows the current branch, not the checkout.** `wt list -v` shows the verbose section whenever the current branch is not the default branch (previously: whenever the current worktree was not the main checkout). The base view is likewise "the default branch is checked out".
- **Tree sibling order** is the fork record's `created_at` (whole seconds), then name; branches created in the same second sort by name.

### Performance and L2 (Wave 12)

- **PR requests in tests never leave the host.** sniff's reqwest client honors `HTTPS_PROXY` (verified: the stub received `CONNECT api.github.com:443`), and provider tokens come only from environment variables (`GH_TOKEN`, `GITHUB_TOKEN`, …), which the tests remove. `perf_support` gains `ProxyStub::hanging()` (accepts, never answers, counts connections) and `ProxyStub::refusing()` (a closed port: the network down), `MixedFixture::with_github_origin`, `wt_command_via(proxy)`, `pr_store()`, `seed_pr_store()`, and `stage_from_perf()`.
  - On Windows the user cache directory does not follow `HOME`, so `pr_store()` (and `list_output.rs`'s `fork_store()`) use the real per-user path there, keyed by the temporary repository, and the tests delete what they seed.
- New L1 behavior tests through the real binary (`tests/list_prs.rs`): a fresh store makes **zero** connections and shows its badge; a stalled request stops at the deadline, shows the stored badge with "PRs as of 12 min ago", and leaves the store byte-for-byte unchanged; a refused connection shows the table with no badges and creates no store, and with a store it shows the stored badge with its age.
- New perf gates (`tests/perf_pr_request.rs`, `perf_` prefix, so they run in `just test-perf` like the existing gates) and the re-measured table are in `worktree/docs/performance-testing.md`. Every `pr gather` in the stalled case was 309–317 ms (the 300 ms deadline plus runtime setup).
- `level2_list_verbose.rs`: both tmux tests now assert the redesigned table (`-> parent` header, `base repo`, the feature row's `○` and `clean`, both legend lines). `list_output.rs` asserts the new table byte for byte (with an isolated home) and adds `create_from_records_the_parent_that_list_draws`: `wt create --from` writes the record, `wt list` nests the branch, answers `clean` / `conflicts`, and prunes a deleted branch's record on disk.

### Requirement-to-test mapping (acceptance criterion 5 and this phase's tasks)

| Requirement | Tests | Level |
|---|---|---|
| Cache key `(target_tip, branch_tip, version)`, version bumped; an old-shape file is not reused | `cache::tests::cache_written_with_the_version_one_key_shape_loads_empty` (the exact version-1 JSON), `cache_round_trip_atomic`, `cache_wrong_version_returns_empty` | L1 |
| One cache serves the caption and both target columns; a warm run has no `rev-list` or `merge-base` | `listing::repo_tests::one_cache_serves_the_caption_and_both_target_columns`, `worktree::tests::list_worktrees_warm_run_*`, `*_tip_advance_invalidates_cache_entry` | L1 |
| Tips from one `for-each-ref`, replacing `default_tip_sha` | `worktree::tests::list_worktrees_reads_tips_from_one_for_each_ref`, `listing::tests::ref_tips_parse_local_and_remote_branches_and_skip_symbolic_heads` | L1 |
| Target selection (Phase 3 rule) and every caption state, with a real bare origin | `listing::repo_tests::the_caption_and_target_follow_origin_in_every_direction` (in sync, behind, diverged, ahead), `default_target::tests::choosing_from_tips_follows_the_same_rule`, `listing::tests::caption_states_cover_every_direction` | L1 |
| `already in` / `clean` / `conflicts` against the target | `listing::repo_tests::the_target_column_reports_already_in_clean_and_conflicts`, `listing::tests::merge_state_reads_ahead_first` | L1 |
| `-> parent`, deleted parents, tree rows, prune during the save | `listing::repo_tests::the_parent_column_and_tree_follow_the_fork_records_and_prune_stale_ones`; end to end through `wt create --from`: `list_output::create_from_records_the_parent_that_list_draws` | L1 |
| Fork tree: spec example, parent rows without worktrees, default row when the base is elsewhere, roots, sibling order, guides, cycles, duplicates, detached | `listing::tests::*` (11 tests; `the_spec_example_builds_the_spec_tree` is the spec's table row for row) | L1 |
| PR store: 60 s window skips the request (call counter), 300 ms deadline, stale results with age, never cache a failure, corrupt/other-version/future files | `pull_requests::tests::*` (8 tests); through the binary: `list_prs::a_fresh_pr_store_makes_no_request_and_shows_its_badges` (0 connections), `a_stalled_pr_request_stops_at_its_deadline_and_shows_stored_badges_with_their_age`, `with_the_network_down_the_table_shows_without_badges_and_nothing_is_stored` | L1 |
| Match by source repository **and** branch; badge placement by target | `pull_requests::tests::prs_match_by_source_repository_and_branch`, `badges_follow_the_prs_target`; rendered: `list_table::pr_badges_follow_the_prs_target_and_skip_forks`; graph: `git_graph::tests::the_git_graph_tags_own_prs_only_and_takes_the_width_override` | L1 |
| Table cells, caption variants, badges, legend, row emphasis (L1 snapshots) | `list_table::the_spec_example_renders_as_ruled` (snapshot), `caption_variants_read_as_ruled` (all four states plus singular and none), `only_the_caption_count_is_colored_yellow`, `every_cell_kind_renders_as_ruled`, `an_unknown_comparison_is_a_question_mark_not_an_answer`, `the_target_header_names_a_remote_or_local_target`, `styles_follow_the_design` (highlight, bold, red connector, strikethrough, dot colors), `pr_badges_link_through_osc_8_and_print_no_url_without_it`, `the_legend_explains_both_columns`, `the_pr_age_line_appears_only_for_stored_results`; real binary: `list_output::list_output_is_the_redesigned_table` | L1 |
| Graph handoff: typed facts with full SHAs, fork parents, origin ahead/diverged, elision, criss-cross, shared merge-base, `--width` override | `git_graph::tests::*` (9 tests) | L1 |
| Pipeline: PR stage beside the git work, graph gathered on a narrow image terminal, no graph work without image support | `list::tests::run_pipeline_gathers_the_graph_on_a_narrow_image_terminal`, `run_pipeline_without_image_support_or_verbose_gathers_no_graph`, `run_pipeline_graph_git_calls_begin_before_list_gather_completes`, and the kept stage tests | L1 |
| `list gather` within warm 120 / cold 300 ms and full `wt list` within 1 s, including the network down and a PR request at its deadline | `perf_cache_warm_list_gather_meets_sla`, `perf_cache_cold_list_gather_meets_sla`, `perf_full_command_non_image_meets_sla`, `perf_pr_request::perf_list_meets_sla_with_the_network_down`, `perf_list_meets_sla_when_the_pr_request_hits_its_deadline`; bench `list_status` | perf |
| Table and graph in a real terminal | `level2_list_verbose::level2_list_verbose_renders_table_and_verbose_in_tmux`, `level2_list_verbose_renders_with_graph_path_active` (both assert the new table); `level2_graph_emits_image_protocol_bytes_in_kitty` | L2 |

Placement: every new test is compiled by a declared target (lib unit tests, `commands::*::tests` in both CLI targets, and auto-discovered `cli/tests/*.rs`; worktree-cli does not set `autotests = false`). No behavior test carries a tier marker. The two new timing gates start with `perf_`, like the existing gates, so `just test` leaves them to `just test-perf`. `just check-tier-coverage worktree`: 0 stranded. No feature gates were added. No test reads a repository file.

### Gates

| Gate | Result |
|---|---|
| `just test` (worktree) | 286 passed, 17 skipped. The skips are the 7 `perf_` gates plus `perf_support`'s own parser tests (its module name starts with `perf_`) in each of the 5 binaries that include it. |
| `just test-l2` (worktree) | 9 passed. `level2_graph_emits_image_protocol_bytes_in_kitty` skips itself here because Kitty is not installed (known since Phase 3). |
| `just test-perf` (worktree) | all green: warm 12.8 ms, cold 23.6 ms, full 55.7 ms, network down 21.0 / 10.5 / 52.9 ms, stalled PR 11.6 ms warm and 359.7 ms full |
| `just lint` (worktree), plus `cargo clippy --all-targets` | clean |
| `cargo bench --bench list_status` (short window) | `list_status/warm` 79 ms on this checkout |

### Cross-OS

- **Native Windows** (`just cross-check worktree-cli --os windows`, CI's L1 archive mode): 160 passed, 21 skipped (the `perf_` family). Every new test ran, including `list_prs` (the proxy stub and the Windows cache-path branch) and `list_output::create_from_records_the_parent_that_list_draws`. `worktree` lib: 122 passed (the 2 `cfg(unix)` tests do not compile there).
- **Linux**: archive mode failed before any test, on the stale read-only `librenderable-*.rmeta` link in the standing clone (the kache trap recorded in `build-hosts.md` in Phase 4). `--no-default-features` (worktree-cli has no default features) takes the native path: 179 passed, with no tier filter, so the perf gates ran too. `worktree` lib: 124 passed.
- **WSL2** was not cross-checked. Nothing here is WSL-specific, and the nightly schedule covers WSL2.

### Docs and skills

- Rewritten: `worktree/docs/git-graph.md`. Updated: `worktree/docs/performance-testing.md` (deleted function names, the cache key, a "PR Request" section, and the re-measured targets).
- Skill: `.claude/skills/worktree/SKILL.md` gains a `wt list` section (cache key, the parse/fill seam, the prune guard, the PR store, `list_table`, the graph handoff, and the proxy-stub test technique).
- **Left for Phase 6 (the plan's docs drift pass):** `worktree/README.md` and `worktree/docs/cli/list.md` still describe the old table, the automatic width table, and the 80-column graph cutoff.
- No `cargo fmt` was run. No crate was added, so `docs/dependencies.md` and `Cargo.lock` are unchanged.
