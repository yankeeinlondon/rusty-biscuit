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
packages:
    - worktree
    - worktree-cli
    - sniff
    - sniff-cli
    - darkmatter
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
