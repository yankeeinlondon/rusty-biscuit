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
