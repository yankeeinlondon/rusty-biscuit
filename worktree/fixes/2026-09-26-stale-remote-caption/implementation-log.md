---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-wt-ux/worktree/fixes/2026-09-26-stale-remote-caption/spec.md"
plan: "worktree/fixes/2026-09-26-stale-remote-caption/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages:
    - worktree
    - worktree-cli
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - worktree/fixes/2026-09-26-stale-remote-caption/implementation-log.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - worktree/lib/src/live_remote.rs
    - worktree/lib/src/remove/live_remote.rs
    - worktree/lib/src/remote_head.rs
    - worktree/lib/src/lib.rs
    - worktree/lib/src/remove/mod.rs
    - worktree/lib/src/remove/remote.rs
    - worktree/lib/src/remove/safety.rs
    - worktree/lib/src/listing.rs
    - worktree/lib/src/worktree.rs
    - worktree/lib/src/cache.rs
    - worktree/lib/src/pull_requests.rs
    - worktree/cli/src/commands/remove/mod.rs
    - worktree/cli/tests/list_table.rs
docs_updated_during_phase_2:
    - worktree/fixes/2026-09-26-stale-remote-caption/plan.md
    - worktree/fixes/2026-09-26-stale-remote-caption/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/worktree/SKILL.md
source_files_during_phase_3:
    - worktree/cli/src/args.rs
    - worktree/cli/src/main.rs
    - worktree/cli/src/commands/mod.rs
    - worktree/cli/src/commands/pr_refresh.rs
    - worktree/cli/src/commands/refresh_worker.rs
    - worktree/cli/src/commands/list.rs
    - worktree/cli/src/commands/list/tests.rs
    - worktree/cli/src/commands/list_table.rs
    - worktree/cli/tests/list_table.rs
    - worktree/cli/tests/list_prs.rs
    - worktree/cli/tests/perf_pr_request.rs
    - worktree/cli/tests/perf_support/mod.rs
    - worktree/cli/tests/level2_list_verbose.rs
    - worktree/cli/tests/snapshots/list_table__the_spec_example_renders_as_ruled.snap
    - worktree/cli/tests/snapshots/list_table__the_table_at_100_columns_shows_counts.snap
    - worktree/cli/tests/snapshots/list_table__the_table_at_99_columns_shows_no_counts.snap
docs_updated_during_phase_3:
    - worktree/fixes/2026-09-26-stale-remote-caption/plan.md
    - worktree/fixes/2026-09-26-stale-remote-caption/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/worktree/SKILL.md
    - .claude/skills/os/SKILL.md
source_files_during_phase_4:
    - worktree/cli/Cargo.toml
    - worktree/cli/tests/level2_list_verbose.rs
    - worktree/cli/tests/list_prs.rs
    - worktree/cli/tests/list_remote_head.rs
    - worktree/cli/tests/list_table.rs
    - worktree/cli/tests/perf_pr_request.rs
    - worktree/cli/tests/perf_support/mod.rs
    - worktree/cli/tests/snapshots/list_table__caption_observation_rows.snap
    - worktree/cli/tests/snapshots/list_table__caption_comparison_states.snap
    - worktree/cli/tests/snapshots/list_table__caption_fresh_and_stale_answers.snap
    - worktree/cli/tests/snapshots/list_table__caption_missing_refs_and_failed_comparison.snap
    - worktree/cli/tests/snapshots/list_table__caption_trunk_default_branch.snap
    - worktree/cli/tests/snapshots/list_table__caption_age_boundaries_and_future_answers.snap
    - worktree/lib/src/live_remote.rs
    - worktree/lib/src/remote_head.rs
docs_updated_during_phase_4:
    - worktree/fixes/2026-09-26-stale-remote-caption/plan.md
    - worktree/fixes/2026-09-26-stale-remote-caption/implementation-log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
    - worktree/lib/src/listing.rs
    - worktree/lib/src/live_remote.rs
    - worktree/lib/src/worktree.rs
    - worktree/cli/src/commands/list_table.rs
docs_updated_during_phase_5:
    - worktree/README.md
    - worktree/docs/cli/list.md
    - worktree/docs/performance-testing.md
    - docs/dependencies.md
    - worktree/fixes/2026-09-26-stale-remote-caption/plan.md
    - worktree/fixes/2026-09-26-stale-remote-caption/implementation-log.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/worktree/SKILL.md
source_code:
    - worktree/lib/src/live_remote.rs
    - worktree/lib/src/remove/live_remote.rs
    - worktree/lib/src/remote_head.rs
    - worktree/lib/src/lib.rs
    - worktree/lib/src/remove/mod.rs
    - worktree/lib/src/remove/remote.rs
    - worktree/lib/src/remove/safety.rs
    - worktree/lib/src/listing.rs
    - worktree/lib/src/worktree.rs
    - worktree/lib/src/cache.rs
    - worktree/lib/src/pull_requests.rs
    - worktree/cli/Cargo.toml
    - worktree/cli/src/args.rs
    - worktree/cli/src/main.rs
    - worktree/cli/src/commands/mod.rs
    - worktree/cli/src/commands/pr_refresh.rs
    - worktree/cli/src/commands/refresh_worker.rs
    - worktree/cli/src/commands/list.rs
    - worktree/cli/src/commands/list/tests.rs
    - worktree/cli/src/commands/list_table.rs
    - worktree/cli/src/commands/remove/mod.rs
    - worktree/cli/tests/list_table.rs
    - worktree/cli/tests/list_prs.rs
    - worktree/cli/tests/list_remote_head.rs
    - worktree/cli/tests/perf_pr_request.rs
    - worktree/cli/tests/perf_support/mod.rs
    - worktree/cli/tests/level2_list_verbose.rs
    - worktree/cli/tests/snapshots/list_table__the_spec_example_renders_as_ruled.snap
    - worktree/cli/tests/snapshots/list_table__the_table_at_100_columns_shows_counts.snap
    - worktree/cli/tests/snapshots/list_table__the_table_at_99_columns_shows_no_counts.snap
    - worktree/cli/tests/snapshots/list_table__caption_observation_rows.snap
    - worktree/cli/tests/snapshots/list_table__caption_comparison_states.snap
    - worktree/cli/tests/snapshots/list_table__caption_fresh_and_stale_answers.snap
    - worktree/cli/tests/snapshots/list_table__caption_missing_refs_and_failed_comparison.snap
    - worktree/cli/tests/snapshots/list_table__caption_trunk_default_branch.snap
    - worktree/cli/tests/snapshots/list_table__caption_age_boundaries_and_future_answers.snap
documentation:
    - worktree/README.md
    - worktree/docs/cli/list.md
    - worktree/docs/performance-testing.md
    - docs/dependencies.md
    - .claude/skills/worktree/SKILL.md
    - .claude/skills/os/SKILL.md
    - worktree/fixes/2026-09-26-stale-remote-caption/plan.md
    - worktree/fixes/2026-09-26-stale-remote-caption/implementation-log.md
completed_phase: 5
implemented: true
---

# Implementation Log for 2026-09-26-stale-remote-caption (5 phases)

## Phase 1

Phase 1 is rulings, spikes, and baseline only. No product source, test, doc, or skill file changed.

### Rulings accepted

Rules 1–14 in `plan.md` are accepted as written. No spike contradicted any of them. The spike results below confirm Rule 7 (the complete-output contract), Rule 13 (a raw loopback listener as the live-hold origin), and Rule 14 (a raw `401` responder).

Rule 7 note, recorded as the plan requires: making `drain` return `Result` and rejecting malformed `ls-remote` lines tightens removal only by turning a would-be false "absent" into "could not reach". That is conservative for removal, so it preserves removal's behavior.

### Baseline (unmodified branch, macOS, 2026-09-26)

- `just test` in `worktree/`: **466 passed, 18 skipped, 0 failed**, across 18 binaries (`worktree` 206, `worktree-cli` lib 56, `bin/wt` 56, `remove` 83, `list_table` 19, `wrapper_protocol` 15, `list_prs` 12, and others).
- `just lint` in `worktree/`: clean for `worktree` and `worktree-cli`.
- No pre-existing failures.

### S1 — Git HTTP hold and 401 on loopback (macOS, git 2.55.0)

The harness was a throwaway Python script outside the repo. It reproduced `run_noninteractive` exactly: `git -C <repo> -c credential.interactive=never ls-remote <url> refs/heads/main`, with stdin null, `GIT_TERMINAL_PROMPT=0`, `GCM_INTERACTIVE=never`, every `*_proxy` removed, `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=<empty file>`, its own process group, and `kill -KILL -<pgid>` at the deadline.

- **Hold.** A raw `TcpListener` that accepts and never replies is enough. Git connected within about 20 ms and sent `GET /r.git/info/refs?service=git-upload-pack HTTP/1.1`. At the 500 ms deadline the group kill returned after **0.504 s**, and the listener saw EOF **0.000 s** after the kill. That is one connection, and the tree was killed.
- **Kill-tree matters.** Killing only the `git` pid (a plain `Child::kill`) left the connection open for the full 4 s observation window, because `git-remote-http` kept it. This confirms the skill's note that `Child::kill` is insufficient. The deadline test in Phase 4 should assert that the listener observes the close.
- **401.** A raw responder that writes the bytes below and closes makes git exit **128 in 0.024 s** with `fatal: unable to get password from user`. It opened one connection and showed no prompt.

  ```text
  HTTP/1.1 401 Unauthorized\r\n
  WWW-Authenticate: Basic realm="r"\r\n
  Content-Length: 0\r\n
  Connection: close\r\n
  \r\n
  ```

- **Decision.** The Rule 13 and Rule 14 fixtures need nothing beyond a raw `std::net::TcpListener`: no HTTP library and no git `http-backend`.
- **Windows.** Not run in Phase 1. `just cross-check` runs cargo tests, not ad-hoc scripts, and there is no Rust test yet to carry. Phase 4's optional Windows cross-check of the transport and deadline tests (Checkpoint 4) covers it.

### S3 — Absent-branch exit semantics (git 2.55.0)

The fixture was a bare origin with branches `main` and `foo/main` at the same commit. HTTP was served by `git http-backend` behind a small Python server.

| Command | Local bare origin | Smart HTTP |
|---|---|---|
| `ls-remote origin refs/heads/missing` | exit 0, empty stdout | exit 0, empty stdout |
| `ls-remote origin refs/heads/main` | exit 0, one line (`refs/heads/main` only) | exit 0, one line |
| `ls-remote origin main` (bare name) | two lines: `refs/heads/foo/main` **and** `refs/heads/main` | — |

- A fully qualified `refs/heads/<b>` pattern does not match `refs/heads/foo/<b>`. The bare-name pattern does tail-match, so `LsRemote` must keep passing the full refname and keep its exact-name comparison.
- Rule 7's "exit 0 with no line for the exact ref is the only `Ok(None)`" matches git's behavior on both transports.

### S2 — Inventory for Phase 4

**Key finding.** `git ls-remote` honors the proxy variables the fixtures set. This was verified with the same harness: `HTTPS_PROXY` to a stub received `CONNECT github.com:443`, and `HTTP_PROXY` to a stub received `GET http://gitea.test/o/r.git/info/refs?service=git-upload-pack`. So once the live-head half exists, both `ProxyStub` (GitHub origin) and `FakeGitea` (gitea origin) receive the worker's `ls-remote` as an extra connection or request. `FakeGitea` counts every request regardless of path, and while held it holds git too. Any test that counts connections or requests, or that waits for `waiting()`, must seed a fresh remote-head store (Rule 12) or it will count the live half.

**Rule 8 consequence.** A PR `Miss` with a non-`Fresh` live head now launches, because `origin.is_some() && head != Fresh`. Tests that assert "a miss never starts a worker" hold only with a fresh remote-head store seeded.

#### Tests that run `wt list`, the worker, or `gather` with an origin

| Test file / test | Origin | Asserts | Phase 4 action |
|---|---|---|---|
| `cli/src/commands/list/tests.rs` `NO_PRS` users (all pipeline tests) | none | `no_launch` panics on launch | none: no origin means no launch (Rule 8) |
| `list/tests.rs` `a_stale_answer_is_shown_at_once_and_refreshed_in_the_background` | `example.invalid` | `git_calls == origin_lookup()`, one launch | launch count unchanged (the PR stale half already launches). Recheck `git_calls` if selection adds a git call (it should not: origin is read once and the default comes from `list`) |
| `list/tests.rs` `a_stale_empty_answer_is_still_an_answer` | same | one launch | unchanged (still one launch, Rule 8's single decision) |
| `list/tests.rs` `a_fresh_answer_neither_requests_nor_refreshes` | same | no launch, one git call | **seed a fresh remote-head store**, otherwise it launches |
| `list/tests.rs` `a_miss_requests_in_the_foreground_and_stores_the_answer` | same | no launch | **seed a fresh remote-head store** |
| `list/tests.rs` `a_failed_miss_shows_no_badges_and_stores_nothing` | same | no launch | **seed a fresh remote-head store** |
| `list/tests.rs` `a_changed_origin_never_shows_the_old_badges` | changed | no launch | **seed fresh for the new origin** |
| `list/tests.rs` `without_an_origin_a_stored_answer_is_ignored_and_nothing_is_requested` | removed | no launch | none: add a leftover `refs/remotes/origin/main` variant for the no-origin suppression (acceptance) |
| `list/tests.rs` `PrSeams` fixture (`fn gather`, ~L568) | — | — | rename to `ListSeams` (Rule 8) |
| `cli/tests/list_prs.rs` `a_fresh_pr_store_makes_no_request_and_shows_its_badges` | GitHub via `ProxyStub` | `connections() == 0` | **seed fresh remote-head** |
| `list_prs.rs` `a_stale_store_shows_its_badges_at_once_and_a_detached_worker_makes_the_request` | GitHub via `ProxyStub` | `connections() == 1`, `Contended` | **seed fresh remote-head**. `finish_worker` must also wait for process exit and the remote-head lock |
| `list_prs.rs` `a_changed_origin_hides_the_stored_badges_and_starts_no_worker` | GitHub, changed | no worker | **seed fresh remote-head for the new origin**, otherwise it launches |
| `list_prs.rs` `the_worker_command_prints_nothing_and_ignores_a_linked_worktree` | GitHub | worker via `internal-refresh-prs` argv | rename argv to `internal-refresh`. Extend it with a subdirectory and a missing path (acceptance 7) |
| `list_prs.rs` `the_worker_command_is_hidden_from_help_and_completion` | — | help and completions lack `internal-refresh-prs` | assert on `internal-refresh` |
| `list_prs.rs` `with_the_network_down_the_table_shows_without_badges_and_nothing_is_stored` | GitHub, refused proxy | PR lock probe loop | the worker's live half also runs; wait for the process and both locks |
| `list_prs.rs` `a_detached_workers_answer_replaces_the_stale_one_on_the_next_list` | gitea | `gitea.requests() == 1`, no worker | **seed fresh remote-head** |
| `list_prs.rs` `concurrent_lists_and_workers_make_one_request_and_a_fresh_answer_stops_the_next` | gitea | `requests() == 1`, one worker | **seed fresh remote-head** |
| `list_prs.rs` `a_killed_worker_releases_its_lock_and_a_later_worker_refreshes` | gitea | `requests() == 2` | **seed fresh remote-head** |
| `list_prs.rs` `a_failed_or_unauthorized_refresh_keeps_the_stored_answer` | gitea | `requests() >= 1` | seed fresh (tolerant as is, but seeding isolates the PR half) |
| `list_prs.rs` `an_origin_change_during_a_workers_request_discards_its_answer` | gitea, changed | `requests() == 1` | **seed fresh remote-head** (the worker is run directly, so the head half still runs) |
| `list_prs.rs` `WorkerGuard`/`finish_worker` helpers (L54–112) | — | PR lock and `wait_for_refresh_workers(…, 0)` | wait for both locks and process exit (Rule 12) |
| `cli/tests/perf_pr_request.rs` `perf_list_meets_sla_with_the_network_down` | GitHub, refused | timing | seed fresh remote-head, or the worker leaks past the fixture |
| `perf_pr_request.rs` `perf_list_meets_sla_when_the_pr_request_hits_its_deadline` | GitHub, hanging | `connections() >= runs` | tolerant, but a PR miss plus a head miss now launches workers: seed fresh remote-head and clean up |
| `perf_pr_request.rs` `perf_list_meets_sla_with_a_stale_answer_and_a_blocked_refresh` | GitHub | `connections() == 0` on fresh, PR lock wait | **seed fresh remote-head**. Keep the test's bound unchanged (plan). Add the `remote select` < 300 ms stage assertion |
| `perf_pr_request.rs` `best_full_command_with_store` / `pr_gather_with_store` | GitHub | timing | seed fresh remote-head alongside the PR store |
| `cli/tests/level2_list_verbose.rs` `DesignFixture` (L298–440, GitHub origin) and every `list_*` user | GitHub via proxy | snapshot and style frames | seed fresh remote-head in `seed_stores`. The caption wording changes the frames (update assertions to the new caption) |
| `level2_list_verbose.rs` `level2_list_stale_pr_answer_shows_a_dim_age_line_in_tmux` | GitHub, hanging proxy | `connections() == 1` | **seed fresh remote-head**. Clean up by waiting for both locks |
| `cli/tests/perf_support/mod.rs` `refresh_worker_via_gitea` (L171–176), `RefreshWorker` / `refresh_workers` (L333–370, argv match at L358) | — | — | argv `internal-refresh`. Add `seed_remote_head_store(age, sha)` and `remote_head_store()` beside `seed_pr_store` |
| `cache_cold_path.rs`, `cache_warm_path.rs`, `perf_command_sla.rs`, `perf_flag.rs`, `list_output.rs`, `level2_dirty_tree.rs`, `level2_graph_in_kitty.rs` | none | — | none: no origin, so no worker |
| `cli/tests/remove.rs`, `level2_remove.rs`, `level2_powershell_remove.rs` | local bare origin | removal only, no `wt list` | none (regression run only) |
| `cli/src/commands/git_graph.rs` tests | local refs only | graph facts | none: no `wt list` launch |

#### Active references to `internal-refresh-prs`, `pr_refresh`, or `remove::live_remote`

Historical specs under `_completed/**` and this fix's own directory are excluded.

| File | Line(s) | What |
|---|---|---|
| `worktree/cli/src/commands/pr_refresh.rs` | 2, 13 | module doc and `SUBCOMMAND` (rename the module to `refresh_worker.rs`) |
| `worktree/cli/src/commands/mod.rs` | 7 | `pub mod pr_refresh;` |
| `worktree/cli/src/commands/list.rs` | 47 | `launch: super::pr_refresh::launch` |
| `worktree/cli/src/args.rs` | 98–99 | `pr_refresh::SUBCOMMAND`, `InternalRefreshPrs` |
| `worktree/cli/src/main.rs` | 55–56 | `Commands::InternalRefreshPrs`, `pr_refresh::run` |
| `worktree/cli/Cargo.toml` | 35 | comment on `sysinfo` |
| `worktree/cli/tests/list_prs.rs` | 4, 209, 228, 239 | module doc, argv, help/completion asserts |
| `worktree/cli/tests/perf_support/mod.rs` | 171, 175, 333, 358 | worker command and argv match |
| `worktree/lib/src/remove/mod.rs` | 3, 13 | module doc link and `pub mod live_remote;` |
| `worktree/lib/src/remove/remote.rs` | 23, 311 | `super::live_remote::…`, test import |
| `worktree/lib/src/remove/safety.rs` | 26, 534 | `super::live_remote::RemoteHeads`, test import |
| `worktree/cli/src/commands/remove/mod.rs` | 29 | `worktree::remove::live_remote::{…}` |
| `worktree/docs/performance-testing.md` | 33 | worker name |
| `docs/dependencies.md` (repo root) | 212 | `sysinfo` rationale names `wt internal-refresh-prs` |
| `.claude/skills/worktree/SKILL.md` | 52, 53, 67 | `live_remote::…`, worker name, `pr_refresh.rs` |

`docs/dependencies.md` at the repo root is outside `worktree/` and is not in Phase 5's drift grep (`worktree .claude`). Phase 5 should update it too.

### Checkpoint 1

- Spikes S1–S3 are recorded above. No ruling is contradicted.
- The baseline is known and green.

## Phase 2

Phase 2 is the library foundations: the moved and tightened transport, `Caption.tracking_sha`, the shared sidecar lock, `default_branch_in`, and the `worktree::remote_head` store. Nothing in the CLI's behavior changed. The only CLI edits are the transport import in `commands/remove/mod.rs` and the new `Caption` field in `cli/tests/list_table.rs`.

### Wave 1

- **Transport move (Rule 1).** `lib/src/remove/live_remote.rs` is now `lib/src/live_remote.rs` (`pub mod live_remote` in `lib.rs`). `remove::live_remote` is gone, not re-exported. Callers updated: `remove/remote.rs`, `remove/safety.rs` (including both test modules), and `cli/src/commands/remove/mod.rs`. The `remove` module's `//!` now points at `crate::live_remote`. `git mv` staged the rename, and this phase must not stage anything, so it was unstaged at once with `git reset -- <both paths>`. The working tree shows a deletion plus a new file; Git will pair them as a rename when the commit step stages them.
- **Complete-output contract (Rule 7).** `drain` now sends `Result<String, String>`: a `read_to_end` error is `Err`. A new `collect(receiver, grace)` turns a grace-period timeout or a disconnected channel into `Err`. `run_noninteractive` returns `collect(&stdout, …)` on success, so an incomplete stdout can no longer be `Ok("")`. Stderr keeps its fallback (`collect(&stderr, …).unwrap_or_default()`, then "git exited with …"). `LsRemote::live_head` delegates to `parse_live_head`: every non-empty line must be `<oid>\t<non-empty refname>` or the whole answer is `Err`. Lines for other refs are skipped, and only the exact `refs/heads/<b>` counts. `is_object_id` (40 or 64 lowercase hex) is public, because `remote_head` validates stored SHAs with it. `LIVE_CHECK_DEADLINE`, `PUSH_DEADLINE`, `kill_tree`, and `batch_ssh_command` are unchanged. As Phase 1 recorded, the only effect on removal is conservative: a would-be false "absent" becomes "could not reach".
- **Transport tests added** (in `live_remote::tests`): `only_the_exact_ref_counts_and_an_empty_answer_is_absence` (`refs/heads/x/main` is not `refs/heads/main`, and a SHA-256 OID is accepted), `a_malformed_line_is_an_error_not_an_absent_branch` (no tab, empty refname, a truncated line, a good line next to a bad one), `a_wrong_length_or_uppercase_object_id_is_an_error` (39, 41, 63, uppercase, non-hex), and `unreadable_or_unfinished_output_is_an_error` (a reader that errors, a missing pipe, a sender that never finishes, and a complete-read control). The three moved tests pass unchanged.
- **Caption tracking SHA.** `Caption` gained `tracking_sha: String`, filled from the same `remote_tip` (one `RefTips` snapshot) that produced the counts. The docs on `Caption` and on `WorktreeList.caption` now say that the counts are against the **local tracking ref**, as of the last fetch, and never the live remote. `listing::tests::the_caption_and_target_follow_origin_in_every_direction` now asserts `tracking_sha` at every step, and adds the reported failure at library level: after another clone pushes and nothing has been fetched, the caption is still `InSync` at the old tracking tip. After the fetch it is `Behind(1)` at the pushed SHA. The struct literals in the `listing` unit test and in `cli/tests/list_table.rs` compile with the new field.
- **Shared lock (Rule 2).** `pull_requests::try_lock` moved to `cache::try_lock_sidecar` (`pub(crate)`), with the "never unlink" contract in its doc. The PR tests pass unchanged, apart from the one test that called `try_lock` by name.
- **`default_branch_in(repo)` (Rule 6).** This runs the same algorithm (`origin/HEAD`, then `main`/`master`) through `git_from(repo, repo, …)`. `default_branch()` delegates with `Path::new(".")` rather than `current_dir()`, which adds no new failure mode. The recorded git args are unchanged, so the subprocess-count tests still hold. New test: `default_branch_in_answers_for_a_repository_other_than_the_cwd`, with a `trunk` repo via `origin/HEAD` and a `master` repo while the cwd is a third repo on `main`.

### Wave 2 — `worktree::remote_head`

- The schema is `{ format_version: 1, origin_digest, branch, sha, checked_at }`. `sha` is `Option<String>` with `#[serde(deserialize_with = "Option::deserialize")]`. That makes the field **required**: plain serde would read a document without `sha` as `None`, a verified absence, from a corrupt file. `load` also rejects an empty `branch` and any `sha` that fails `is_object_id`.
- `remote_head_store_path(main)` gives `repo_cache_file(main, "remote-head.json")`, and `remote_head_lock_path(store)` gives `store.with_extension("lock")`, which is `<hash>.remote-head.lock`.
- `select_cached_head(store, origin, default_branch, now)` returns `Fresh | Stale | Miss`, per Rule 4. The freshness window is `pull_requests::FRESHNESS_WINDOW` (60 s), reused rather than duplicated. `RemoteHead::{is_stale_at, is_future_at}` are for render-time evaluation.
- `refresh_remote_head(store, main, clock, heads)` follows the Rule 5 sequence exactly: lock (contended means no request), `origin_url`, `default_branch_in`, `checked_at = clock()`, freshness recheck, `heads.live_head("origin", &branch)`, re-read of both origin and default branch, then publication under the lock. `REMOTE_HEAD_REFRESH_DEADLINE` = 10 s. Only `origin_digest` is stored; the URL is never stored or logged.
- **Deviation from Rule 5 (small).** Rule 5 said to reuse `pull_requests::RefreshOutcome` and add a `NoDefaultBranch` variant. I also added `DefaultBranchChanged`, rather than reporting a default-branch change as `OriginChanged`, because that name would be wrong and the tests assert the two cases separately. Nothing in the CLI matches `RefreshOutcome` exhaustively, so the extra variant touches no other code.

#### Store tests (acceptance 1), `remote_head::tests`

| Requirement | Test |
|---|---|
| Fresh at 59 s, stale at 60 s; render-time re-evaluation; future is not stale | `an_answer_is_fresh_below_60_seconds_and_stale_from_60` |
| Verified absence is `Fresh`/`Stale`, not a miss | `a_verified_absence_is_an_answer_not_a_miss` |
| Changed origin, changed branch, no origin, no default, future `checked_at` are misses | `another_origin_another_branch_no_origin_and_a_future_answer_are_misses` |
| Corrupt JSON, format 0/2, bad OIDs (39/41 chars, uppercase, short, non-hex, non-string), empty branch, every missing field (incl. `sha`), absent file are misses; SHA-256 and `null` accepted | `corrupt_other_format_invalid_and_incomplete_documents_are_misses` |
| Requests `origin` by name for the default branch, stamps the start time, read/write/read round trip | `a_refresh_asks_origin_by_name_for_the_default_branch_and_stamps_its_start` |
| Non-`main` default branch (`origin/HEAD` → `trunk`) | `a_refresh_follows_a_non_main_default_branch` |
| Only a digest of the origin is stored | `the_store_records_only_a_digest_of_the_origin` |
| No origin / no default branch: no request, no file | `without_an_origin_or_a_default_branch_no_request_is_made` |
| Fresh answer skips the request, bytes unchanged | `a_fresh_answer_skips_the_request` |
| Request failure and invalid-output `Err` preserve bytes; a failure with no previous answer stores nothing | `failures_leave_the_previous_bytes_untouched` |
| Publication failure (store path is a directory) and lock failure | `a_publication_or_lock_failure_is_reported_and_changes_nothing` |
| Origin change during a blocked request is discarded | `an_origin_change_during_the_request_discards_the_answer` |
| Default-branch change during a blocked request is discarded | `a_default_branch_change_during_the_request_discards_the_answer` |
| Competing refreshes: one `Contended` with no request; then `AlreadyFresh` with no request; sidecar persists | `a_competing_refresh_is_contended_and_a_published_answer_stops_the_next` |
| Real `LsRemote` against a local bare origin: present, pushed elsewhere (tracking ref untouched), absent after deletion | `git_ls_remote_stores_present_pushed_and_absent_heads` |
| Store and lock file names beside the comparison cache | `store_and_lock_sit_beside_the_comparison_cache` |

The blocked-request tests use an `mpsc` readiness channel and a release channel, each bounded by `recv_timeout(10 s)`. No test uses a fixed sleep.

### Verification

- `cargo nextest run -p worktree`: **227 passed** (baseline 206; 5 new in Wave 1, 16 in `remote_head`).
- `just test` in `worktree/`: **487 passed, 18 skipped, 0 failed** (baseline 466/18). That includes every `remove::` transport, safety, and remote test and `cli/tests/remove.rs`.
- `just lint` in `worktree/`: clean after one fix (`clippy::useless_format` in a new transport test).
- `grep -rn "remove::live_remote" worktree/` matches only this fix's `plan.md` and `implementation-log.md`, not any source file.
- **Cross-OS:**
  - `just cross-check worktree --os windows`: 208 passed. A filtered re-run showed all 16 `remote_head` tests and `default_branch_in_answers_for_a_repository_other_than_the_cwd` passing on native Windows.
  - `--os wsl`: 228 passed.
  - `--os linux` (build-linux) **failed before compiling any of our code**: `target/release/deps/*.rmeta is not writeable -- check its permissions` on that host. This is a rig permission problem, not a test failure. WSL2 stands in as the Linux evidence for this phase.

### Skill

`.claude/skills/worktree/SKILL.md`: the `run_noninteractive` bullet now names `worktree::live_remote` as a top-level module shared with `worktree::remote_head`, and states the complete-output contract. The `wt list` description (store, worker, caption) is left to Phase 5, as the plan says.

### Checkpoint 2

Met: `just test` and `just lint` pass in `worktree/`, and no source file names `remove::live_remote`.

## Phase 3

Phase 3 covers the worker, the caption rendering, and the list orchestration. To keep `just test` green once the command is renamed and the worker gains its live-head half, it also does most of Phase 4's **Migrate existing tests** task. That task stays unchecked in the plan, because what is left of it is listed under "Left for Phase 4" below.

### Wave 1 — shared worker (`cli/src/commands/refresh_worker.rs`)

- `pr_refresh.rs` is now `refresh_worker.rs`: `SUBCOMMAND = "internal-refresh"` and `Commands::InternalRefresh { repo }`, still `hide = true`. As in Phase 2, the file was `git mv`'d and then unstaged, so the working tree shows a deletion plus a new file. The launcher is unchanged: `main_checkout` validation, cwd = main, null stdio, `WT_SHELL_WRAPPER`/`COMPLETE` removed, `configure_detached_child`, and silent failure.
- `run(repo)` → `run_halves(main, pr_half, head_half)`. Each half runs on its own `std::thread::scope` thread. Each is joined separately and its panic is discarded. The PR half is the unchanged `pull_requests::refresh`. The head half is `refresh_remote_head` with `LsRemote { base: main, deadline: REMOTE_HEAD_REFRESH_DEADLINE }`.
- **Deviation from Rule 9 (small).** `run_halves` takes `impl FnOnce(&Path) + Send`, not `fn(&Path)`. The tests need closures that capture `mpsc` channels and a fixture's store paths, which a plain `fn` pointer cannot do without statics. Production passes the two `fn`s unchanged.
- Tests. All are in the module, so they run under both the lib and bin targets. They use real stores in a temp dir, a real repo with an `origin`, and stub sources. The blocked tests wait on channels bounded by `recv_timeout(10 s)` and never sleep a fixed time.

| Requirement (acceptance 4, worker side) | Test |
|---|---|
| Head half publishes while the PR half is blocked; a sequential worker would time out | `the_head_half_publishes_while_the_pr_half_is_blocked` |
| PR half publishes while the head half is blocked | `the_pr_half_publishes_while_the_head_half_is_blocked` |
| A failing or unsupported-provider PR half still lets the head half publish | `a_failing_or_unsupported_pr_half_leaves_the_head_half_publishing` |
| A contended PR half (another refresh holds the lock mid-request) still lets the head half publish | `a_contended_pr_half_leaves_the_head_half_publishing` |
| A failing head half still lets the PR half publish, and stores nothing | `a_failing_head_half_leaves_the_pr_half_publishing` |
| A panicking half, either one, does not stop the other | `a_panicking_half_does_not_stop_the_other` |
| Kept: main-checkout validation and spawn failure | `only_the_top_level_of_a_main_checkout_is_accepted`, `a_worker_that_cannot_start_is_an_error_launch_discards` |

### Wave 1 — caption rendering (`cli/src/commands/list_table.rs`)

- `RemoteFacts { default_branch, tracking_tip: Option<&str>, answer: Option<&RemoteHead> }` is `Copy`. `TableFacts` gains `remote: Option<RemoteFacts>`. `TableFacts::from_list(list, prs, remote)` sets `caption` to `None` whenever `remote` is `None`, so with no origin, leftover `origin/*` refs never render.
- **Deviation from Rule 10 (small).** There is no `tracking_ref` field. The name is always `origin/<default>` (it is how the library builds `Caption.remote`), so `RemoteFacts::tracking_ref()` derives it. A stored field could only disagree with the name.
- `caption_markup` now reads "… with/behind/ahead of/diverged from local tracking ref `origin/main`." and ends with a period. The count is still the only yellow text.
- `observation_markup(remote, now)` covers the spec's five rows plus the two no-tracking-ref variants. Branch names use the existing badges, and the rest is `<dim>`. An answer is ignored when it is future-dated at `now` or names another branch. Every observation is past tense.
- **Wording decision (not in the spec table).** An answer of absence when there is no local tracking ref reads "No local tracking ref `origin/main`; the remote branch was absent when checked N ago." This is the spec's no-tracking-ref sentence form with the absent row's fact. It never claims the branch is still deleted.
- `age_text(seconds)` is shared: "less than 1 min", then `N min`, `N h`, and `N days`, using the existing bands. `pr_age_markup` calls it with `minutes * 60`, and its output is unchanged. (PR ages are at least 1 min, because the line appears only when the answer is stale.)
- `render` prints the caption paragraph (comparison, then observation, in one `Prose`) when either part exists.
- **Wrapping change (found by the L2 run).** The caption `Prose` had no word-wrap policy. While the caption was one short sentence that did no harm. With the observation added, a 120-column tmux pane broke the line mid-word ("1 m" / "in ago"). The caption paragraph now uses `WordWrap::WrapProse(None, Some(1))`: it breaks between words, and continuation lines keep the caption's one-space indent. The 99-column snapshot now shows the caption on two lines. The spec asks to "preserve wrapping". The old paragraph never wrapped only because it never needed to, so this does not change any existing short caption.
- Module and function docs were updated for the new contract (the authoring discipline).

### Wave 2 — list orchestration (`cli/src/commands/list.rs`)

- `PrSeams` is now `ListSeams { connect, launch }` (`PrLaunch` → `RefreshLaunch`). `gather_prs` is now `gather_remote(Stores { prs, head }, main, default_branch, seams) -> RemoteAnswers { prs, head: Option<CachedRemoteHead>, origin_present, pr_gather, remote_select }`, per Rule 8:
    - read `origin_url(main)` once, then select the PR store
    - on a PR miss with an origin, finish the 300 ms foreground request
    - select the live head with `list.default_branch`
    - launch once if `pr_stale || (origin && head != Fresh)`
- The `pr gather` perf stage now covers only the origin lookup, the PR selection, and any foreground request. The new `remote select` stage covers the live-head selection plus the launch, for Phase 4's < 300 ms stage gate. Previously a stale-PR launch was counted inside `pr gather`, so that stage can only get smaller.
- `run_pipeline` builds `RemoteFacts` only when an origin exists. `tracking_tip` is `caption.tracking_sha`, falling back to `list.refs().remote("origin/<default>")` (the same snapshot). The library already exposed `WorktreeList::refs()`, so this phase changed no library code.
- Unit tests (`commands::list::tests::gather`). They record the order of every seam call (`Connect`, `Fetch`, `Launch`) and seed the head store by writing its JSON format:

| Requirement (acceptance 4, launch side) | Test |
|---|---|
| Stale PR plus missing head: exactly one launch | `a_stale_pr_answer_and_a_missing_head_launch_exactly_once` |
| Both fresh: no connect and no launch; the only git call is the origin lookup | `two_fresh_answers_neither_request_nor_refresh` |
| No origin: nothing requested or launched; `head` is `None` | `without_an_origin_stored_answers_are_ignored_and_nothing_is_requested_or_launched` |
| PR miss plus fresh head: one foreground request, no launch (unchanged) | `a_pr_miss_with_a_fresh_head_requests_in_the_foreground_and_launches_nothing` |
| PR miss plus missing or stale head: `Connect`, `Fetch`, then `Launch`, so the two never overlap | `a_pr_miss_settles_before_the_worker_is_launched` |
| Missing or stale head with a fresh PR answer: launch, no connect, no foreground `ls-remote` (the git calls are only the origin lookup) | `a_missing_or_stale_head_launches_without_any_foreground_request` |
| Existing PR behavior kept (stale shown at once, empty answer, failed miss, changed origin) | `a_stale_answer_is_shown_at_once_and_refreshed_in_the_background`, `a_stale_empty_answer_is_still_an_answer`, `a_failed_miss_shows_no_badges_and_stores_nothing`, `a_changed_origin_never_shows_the_old_answers` |

The `NO_PRS` pipeline tests still pass: their repositories have no origin.

### Test migration pulled forward from Phase 4 (needed for green)

Renaming the command and adding a live-head half broke 7 `list_prs` tests. They failed for two reasons: the old argv, and git's HTTP transport honoring `HTTPS_PROXY`/`HTTP_PROXY`. The worker's `ls-remote` to the `github.com`/`gitea.test` origins reached `ProxyStub`/`FakeGitea` and was counted as a PR request.

- `perf_support`:
    - `refresh_workers` and `refresh_worker_via_gitea` now match `internal-refresh`.
    - `wt_command_without_network` refuses git's own transports with `GIT_CONFIG_COUNT=2`, `protocol.http.allow=never`, and `protocol.https.allow=never`. With that, only PR requests reach the stand-ins.
    - New helpers: `remote_head_store()`, `seed_remote_head_store(age, sha)`, and `probe_head_refresh()`.
    - `NoRequest` also implements `RemoteHeads`.
    - `wait_until_unlocked` now requires both locks to be free **and** no `internal-refresh` process (Rule 12).
    - `RemoveOnDrop` also removes the head store and its lock (Windows real cache).
- **Linux-only finding.** On WSL2, `refresh_workers` saw two "workers" for one process. `sysinfo` on Linux lists every thread (task) with its process's argv, and the worker now has two threads. It now filters `thread_kind().is_none()`. I added this to the `os` skill's standing rules.
- `list_prs.rs`: every fixture that counts requests or asserts that no worker runs seeds a fresh head. Two assertions were added: two fresh answers start no worker, and a PR miss alone starts no worker. The help and completion check now looks for `internal-refresh`. The network-down test now waits for the worker's exit and both locks.
    - A detail I found along the way: listing reads `origin` once, before the PR request (Rule 8). So in the foreground origin-change test, the head is selected with the old origin, and the fresh head seeded for the old origin is what keeps a worker from racing the assertions.
- `perf_pr_request.rs`: all three tests seed a fresh head and hold a `RemoveOnDrop`, so no sample launches a worker for the live head alone.
- `level2_list_verbose.rs` (L2, tmux): the fixture seeds a fresh head that matches `origin/main`, and the caption assertion now checks the new wording plus the observation, whitespace-normalized because it wraps.
- `list_table.rs`:
    - `Example` gains a stored head (`origin/main`'s tip, 2 min old).
    - The two existing caption tests are updated.
    - The three snapshots change only in the caption lines.
    - New plain-text tests: `every_remote_observation_reads_as_ruled` (every row, the no-tracking-ref variants, future-dated, other branch), `the_observation_follows_the_comparison_in_one_paragraph`, `a_stale_answer_keeps_past_tense_and_shows_its_age` (5 h, 3 days), `ages_use_the_pr_age_units` (0, 59, 60, 3599, 3600, 2 days − 1 min, and 2 days in seconds), `without_an_origin_leftover_tracking_refs_show_no_caption`, `a_failed_comparison_still_shows_the_observation`, and `a_long_caption_wraps_between_words_within_the_terminal`.

#### Left for Phase 4

- The full acceptance-3 **snapshot** matrix. The plain-text tests above cover the rows but not snapshots of each state.
- `seed_remote_head_store` always writes branch `main`, because every current fixture uses `main`. A `trunk` fixture needs a parameter.
- The Rule 13 live-hold fixture. Remember that `MixedFixture`'s network-free command now sets `GIT_CONFIG_COUNT`, so a live-head test that needs git's HTTP transport must `env_remove("GIT_CONFIG_COUNT")` (or build its own command).
- The Rule 14 `401` test, the `remote select` < 300 ms assertion, and the real-Git detection file.

### Verification

- `just test` in `worktree/`: **512 passed, 18 skipped, 0 failed** (Phase 2 ended at 487/18).
- `just lint` in `worktree/`: clean after one fix (an overlapping `0` / `..=59` match range in `age_text`).
- L2: `BISCUIT_TEST_REQUIRED_BACKENDS=tmux cargo nextest run -p worktree-cli --features terminal-tests -E 'binary(level2_list_verbose)'` gave **9/9 passed**.
- Cross-OS (`just cross-check worktree-cli`):
    - WSL2: first run 2 failures (the `sysinfo` thread over-count above); after the fix, **285 passed**.
    - Native Windows: **271 passed** on the pre-wrap tree, and **272 passed** on the final tree.
- Checkpoint 3 manual smoke, in this checkout (`NO_COLOR=1 target/debug/wt list`, origin `git@github.com:…`):
    1. First run: "main is in sync with local tracking ref origin/main. Remote state has not been verified."
    2. About 5 s later: "… origin/main matched the remote when checked less than 1 min ago." (`git ls-remote` confirmed `69b207ee3`, equal to local `origin/main`).
    3. `ps` about 15 s later showed no `internal-refresh` process.
- New tests and tiers: every new test is L1. None has a tier marker in its path, and all sit in declared targets: the `refresh_worker` and `list` unit modules, and `tests/list_table.rs`. `worktree-cli` does not set `autotests = false`. The only L2 edit is inside the existing `level2_list_verbose` binary.

### Checkpoint 3

Met: `just test` passes, the smoke run shows the qualified caption and "Remote state has not been verified." first and an aged observation next, and no worker lingers.

## Phase 4

Phase 4 adds the integration tests and finishes migrating the existing tests. No product code changed. The only non-test edit is a comment in `cli/Cargo.toml` that still named `internal-refresh-prs`. Two subagents wrote the isolated files (the `list_table.rs` snapshot matrix and the library transport tests). I reviewed both results and read the snapshots.

### Migrate existing tests (finishing Phase 3's pull-forward)

- Checked against the S2 inventory: every entry is done. The pieces left over from Phase 3 were:
    - `level2_list_verbose::level2_list_stale_pr_answer_shows_a_dim_age_line_in_tmux`: after the worker exits, it now also asserts that the live-head lock is free (new `probe_head_refresh`). Before, only the PR lock and the process were checked (Rule 12).
    - `cli/Cargo.toml`: the `sysinfo` comment now names `wt internal-refresh`.
- `perf_support` additions, which the new tests use:
    - `HoldingOrigin`, the Rule 13 loopback origin. It records each connection's request line and holds the connection until `close_held`.
    - `MixedFixture::with_origin(url)`.
    - `MixedFixture::wt_command_direct()`: no proxy variables, `GIT_CONFIG_COUNT` removed, `GIT_CONFIG_NOSYSTEM=1`, and `GIT_CONFIG_GLOBAL` set to an empty file.
    - `MixedFixture::seed_empty_pr_store(age)`: an answer for an origin no provider recognizes.
    - `isolated_cache_file(home, xdg, real)`, now shared with `MixedFixture::pr_store`.
- Still left for Phase 5, which owns docs: `docs/dependencies.md` (repo root, line 212) and `worktree/docs/performance-testing.md` (line 33) still say `internal-refresh-prs`.

### Requirement-to-test mapping

| Requirement | Test(s) |
|---|---|
| Acc. 2: a push elsewhere reads as a difference until the fetch. This is the reported bug: before the fix the caption said "in sync". | `list_remote_head::a_push_elsewhere_reads_as_a_difference_until_the_fetch_then_as_behind_and_matched` |
| Acc. 2: after `git fetch`, the caption shows "1 commit behind" plus "matched", and listing makes no request (store bytes unchanged, no worker) | same test, second half |
| Acc. 2: a fetch newer than the observation reads "differs", never "moved" or "advanced" | `list_remote_head::a_fetch_newer_than_the_observation_is_a_difference_never_a_move` |
| Acc. 2: remote deletion reads absent; after `fetch --prune`, "No local tracking ref …; … was absent"; after the branch is recreated and refreshed, present | `list_remote_head::a_deleted_then_recreated_remote_branch_is_reported_absent_then_present` |
| Acc. 2: the main checkout and a linked worktree resolve one `remote-head.json` (a worker launched from the linked worktree records it) | `list_remote_head::the_main_checkout_and_a_linked_worktree_share_one_live_head_store` |
| Acc. 2 / no origin: leftover `refs/remotes/origin/main` shows no caption and starts no worker (no lock file created) | `list_remote_head::without_an_origin_leftover_tracking_refs_show_no_caption_and_start_no_worker` |
| Acc. 3: snapshot matrix (5 rows, 4 comparison states, fresh/stale × match/differ/absent, missing default or tracking tip, failed comparison, `trunk`, age boundaries 59/60/3599/3600/2 d − 1 min/2 d, future-dated) | `list_table::caption_snapshot_observation_rows`, `…_comparison_states`, `…_fresh_and_stale_answers`, `…_missing_refs_and_failed_comparison`, `…_trunk_default_branch`, `…_age_boundaries_and_future_answers` (6 `.snap` files) |
| Acc. 5: a missing or stale live head never holds up `wt list`. `.output()` returns while `HoldingOrigin` still holds the worker's `ls-remote`, the head lock is `Contended`, there is exactly one worker and one request (git's `GET /r.git/info/refs…`), no PR request, and the store is unchanged | `list_prs::a_missing_or_stale_live_head_never_holds_up_the_listing` (both cases, with a drop guard `ReleaseOnDrop`) |
| Acc. 5: timing. Each sample's `remote select` stays under 300 ms with the refresh blocked; the best full command stays under 1 s | `perf_pr_request::perf_remote_select_stays_under_the_deadline_with_a_blocked_live_head_refresh` (macOS: 0.3–0.9 ms per sample, 75 ms full). This is a `perf_` test, so it runs under `just test-perf`, not `just test`. |
| Acc. 6: a `401` fails fast with no prompt | `live_remote::tests::an_unauthorized_origin_fails_fast_without_a_prompt` |
| Acc. 6: a `401` leaves the store bytes unchanged and creates nothing when no store existed | `remote_head::tests::an_unauthorized_origin_fails_fast_and_leaves_the_store_alone` |
| Acc. 6: the deadline kills the HTTP transport tree, and the listener sees the close (all OSes, no `cfg(unix)`) | `live_remote::tests::the_deadline_kills_the_http_transport_and_closes_its_connection` |
| Acc. 6: a reader that errors or never finishes, and malformed lines, give `Err` and preserve the store | already covered in Phase 2: `unreadable_or_unfinished_output_is_an_error`, `a_malformed_line_is_an_error_not_an_absent_branch`, `failures_leave_the_previous_bytes_untouched` |
| Acc. 7: the worker ignores a linked worktree, a subdirectory, and a missing path (no lock, no output); on the main checkout it takes both locks and a failure stores nothing in either store | `list_prs::the_worker_command_prints_nothing_and_ignores_anything_but_a_main_checkout` (renamed from `…_ignores_a_linked_worktree`) |
| Acc. 7: removal is unchanged | `cli/tests/remove.rs` and the lib `remove::` suites (inside `just test`), and `level2_remove` in tmux (9/9) |

Library-test fixture notes (from the subagent):
- `live_remote::tests` is now `pub(crate) mod tests`, so `remote_head` tests can reuse its `Loopback` server.
- Each test repository sets an empty `http.proxy`, which turns off proxying and overrides `*_proxy`, and an empty `credential.helper`. No test mutates the process environment.
- Known gap: a user's global `url.*.insteadOf` rule matching `http://127.0.0.1` would still apply. No build host or CI runner has one.

Findings while writing the tests (test-side only, no product change):
- In `NO_COLOR` output a branch badge keeps its padding (`origin/main .`). `list_remote_head` normalizes the text before asserting, and the snapshots record it as rendered. The spacing is not new; the existing snapshots already had it.
- Any stored answer 60 s or older is stale, so it launches a worker. A test that asserts "no request from listing" must keep the answer fresh, not aged.

### Verification

- `just test` in `worktree/`: **527 passed, 21 skipped, 0 failed** (Phase 3: 512/18). The 3 new skips are the `perf_` test and the two `perf_support::tests` copies in the new `list_remote_head` binary, which run under `just test-perf`.
- `just test-perf` in `worktree/`: **21/21 passed**.
- `just lint` in `worktree/`: clean. `cargo clippy -p worktree-cli --tests --all-features -- -D warnings`: clean.
- L2 (tmux), `BISCUIT_TEST_REQUIRED_BACKENDS=tmux cargo nextest run -p worktree-cli --features terminal-tests -E 'binary(level2_list_verbose) | binary(level2_remove)'`: **18/18 passed**.
- `just check-tier-coverage worktree`: nothing stranded. Every new test is L1, except `perf_remote_select_…`, which is in the perf tier. All sit in declared targets (`worktree-cli` has no `autotests = false`, so `tests/list_remote_head.rs` is its own binary).
- After the runs, `ps` shows no `internal-refresh` process.
- Cross-OS:
    - `just cross-check worktree-cli --os windows`: **284 passed** (Phase 3: 272; +12 new).
    - `--os wsl`: **297 passed** (Phase 3: 285; +12).
    - `just cross-check worktree`, run by the subagent: Windows **211**, WSL2 **231**, macOS 230.
    - build-linux was not retried, because of the rig permission problem recorded in Phase 2. WSL2 stands in as the Linux evidence.
    - The WSL cross-check published no receipt, because the tree is uncommitted.

### Checkpoint 4

Met: `just test` and `just lint` pass, the L2 runs for `level2_list_verbose` and `level2_remove` pass, no worker lingers, and the optional Windows cross-check was run for both packages.

## Phase 5

Phase 5 covers documentation, drift, and final validation. No behavior changed. The only source edits are to `///` doc comments.

### Documentation

- `.claude/skills/worktree/SKILL.md`, `wt list` section:
    - New `remote_head` bullet: the store's shape and format, `select_cached_head` states, the refresh lock and its recheck-through-publication rule, `checked_at` taken before the request, the origin/default re-read, the "differs, never moved" rule, and `observation_markup` / `TableFacts::from_list` no-origin suppression.
    - New live-head test-helper bullet: `HoldingOrigin`, `MixedFixture::with_origin`, `wt_command_direct`, `seed_empty_pr_store`, `isolated_cache_file`, the `ReleaseOnDrop` cleanup rule, `list_remote_head.rs`, and the reuse of `live_remote::tests::Loopback`.
    - The `wt remove` section already named `worktree::live_remote` and the complete-output contract (Phase 2/3). The worker, launch-decision, and caption bullets were also written in Phase 3.
- `worktree/docs/cli/list.md`:
    - The example caption now uses the new wording.
    - The "Caption" section now covers the local-tracking-ref comparison, the five-row observation table, age units, the no-origin rule, and the background check.
    - The PR section says the same worker also checks the live head. This file was drifted: it still described the old unqualified caption. The plan did not list it.
- `worktree/docs/performance-testing.md`:
    - `internal-refresh-prs` became `internal-refresh`.
    - The caption bullet now says "local tracking ref".
    - The `pr gather` stage no longer claims the stale-answer spawn; since Phase 3 that is in `remote select`.
    - New "Live Remote Head" section: the store, the 10 s background-only request, the single launch decision, the two locks, and the `remote select` stage with its proving tests.
    - The frontmatter `hash` was refreshed with `md hash`. It was current before the edit.
- `worktree/README.md`: the `wt list` caption bullet now describes the local tracking ref, the aged observation, the background check, and "no origin, no caption".
- `docs/dependencies.md` (repo root): the `sysinfo` note now names `wt internal-refresh`.
- No crate was added in any phase. `git diff main -- '*Cargo.toml' Cargo.lock` shows only the Phase 4 comment change, so no dependency docs needed updating beyond that name.

### Drift sweep

- `grep -rn "internal-refresh-prs\|pr_refresh\|remove::live_remote\|PrSeams\|InternalRefreshPrs" worktree .claude docs` now matches only `fixes/_completed/2026-09-25-list-remove-performance/**` and this spec's own directory, both historical.
- Doc drift found and fixed (the code is correct; the comments were wrong):
    - `listing::Caption`: "the one line above the table". The caption is now a wrapped paragraph that also carries the observation. It now says "the comparison that opens the caption above the table".
    - `live_remote::LIVE_CHECK_DEADLINE` / `PUSH_DEADLINE`: now in a shared module, so they are marked as removal's deadlines, with a link to `REMOTE_HEAD_REFRESH_DEADLINE` for the list refresh. `RemoteHeads`' "tier logic" note now also names the live-head refresh.
    - `worktree::default_branch`: said "(main or master)", but resolution tries `origin/HEAD` first. It now points to `default_branch_in`.
    - `list_table::TableFacts`: said "Everything the table shows", but it also feeds the caption, legend, and PR age line.
- Reviewed with no drift: `caption_markup`, `observation_markup`, `RemoteFacts`, `ListSeams`, `gather_remote`, `run_noninteractive`, `LsRemote`, `refresh_worker` module and `launch`, and `args::InternalRefresh`.

### Final validation

- `just test` (worktree/): **527 passed, 21 skipped, 0 failed**, the same as Phase 4. The skips are the perf-tier and level-2 tests, which run under their own recipes.
- `just lint` (worktree/): clean.
- `just test worktree` / `just test worktree-cli` from the root were not run separately. They select the same nextest packages as the area's `just test`, which already covers both.
- `cargo doc -p worktree -p worktree-cli --no-deps` gives 3 warnings, all pre-existing and from other work on this branch or on main, none added by this spec: `worktree::config::resolve_base_dir` unresolved, `table` → private `METRICS_MIN_WIDTH`, and `run_handoff` → private `reconfirm_branch`. The new links (`REMOTE_HEAD_REFRESH_DEADLINE`, `default_branch_in`, `render`) resolve.
- Manual smoke test (Checkpoint 3), `NO_COLOR=1 wt list` in this checkout:
    - The first run shows "main is in sync with local tracking ref origin/main. origin/main matched the remote when checked 33 min ago." That stale answer launches the worker.
    - 15 s later the same line reads "less than 1 min ago".
    - `ps` shows no `internal-refresh` process afterwards.
- Cross-OS: not re-run. This phase changed only comments and Markdown. The Phase 4 Windows and WSL2 cross-checks stand.

### Definition of Done and acceptance walk-through

| Item | Where it is proven |
|---|---|
| DoD 1 / Acc. 5: `wt list` makes no foreground wait for the live head; a PR miss keeps its 300 ms request | `list_prs::a_missing_or_stale_live_head_never_holds_up_the_listing`, `perf_pr_request::perf_remote_select_stays_under_the_deadline_with_a_blocked_live_head_refresh`, `perf_pr_request::perf_list_meets_sla_with_a_stale_answer_and_a_blocked_refresh`, `commands::list::tests::a_pr_miss_with_a_fresh_head_requests_in_the_foreground_and_launches_nothing` |
| DoD 2 / Acc. 3: "local tracking ref" wording, five rows, age units | `list_table::caption_snapshot_*` (6 snapshots), `caption_variants_read_as_ruled`, `only_the_caption_count_is_colored_yellow` |
| DoD 3: no origin means no comparison or observation | `list_remote_head::without_an_origin_leftover_tracking_refs_show_no_caption_and_start_no_worker`, `list_table` missing-origin case |
| DoD 4 / Acc. 4: at most one launch; the halves are independent | `commands::list::tests::a_stale_pr_answer_and_a_missing_head_launch_exactly_once`, `a_missing_or_stale_head_launches_without_any_foreground_request`, `a_pr_miss_with_a_fresh_head_requests_in_the_foreground_and_launches_nothing`, `a_pr_miss_settles_before_the_worker_is_launched`; `commands::refresh_worker::tests::the_pr_half_publishes_while_the_head_half_is_blocked`, `a_panicking_half_does_not_stop_the_other`, `a_failing_or_unsupported_pr_half_leaves_the_head_half_publishing`, `the_head_half_publishes_while_the_pr_half_is_blocked` |
| DoD 5 / Acc. 1 and 6: failures preserve bytes; only complete output proves absence | `remote_head::tests` (freshness boundaries, origin/default change mid-request, contention, `failures_leave_the_previous_bytes_untouched`, `an_unauthorized_origin_fails_fast_and_leaves_the_store_alone`); `live_remote::tests` (`unreadable_or_unfinished_output_is_an_error`, `a_malformed_line_is_an_error_not_an_absent_branch`, `the_deadline_kills_the_http_transport_and_closes_its_connection`, `an_unauthorized_origin_fails_fast_without_a_prompt`) |
| DoD 6 / Acc. 7: `internal-refresh` is hidden and validates its path; no active `internal-refresh-prs` | `list_prs::the_worker_command_is_hidden_from_help_and_completion`, `list_prs::the_worker_command_prints_nothing_and_ignores_anything_but_a_main_checkout`, `refresh_worker::tests::only_the_top_level_of_a_main_checkout_is_accepted`; the drift grep above |
| DoD 7 / Acc. 7: removal deadlines and safety unchanged | `LIVE_CHECK_DEADLINE` 3 s and `PUSH_DEADLINE` 30 s unchanged; `cli/tests/remove.rs`, lib `remove::` suites (in `just test`), `level2_remove` 9/9 (Phase 4) |
| DoD 8: `just test` and `just lint` pass; no worker survives | this phase's run; `MixedFixture::wait_until_unlocked`, `ReleaseOnDrop`, smoke `ps` check |
| DoD 9: skill, performance doc, and README updated | this phase (plus `docs/cli/list.md`) |
| DoD 10: spec left for the author | spec not moved; no commit |
| Acc. 2: real-Git detection | `list_remote_head::*` (5 tests; see the Phase 4 table) |

**State: implementation complete, ready for review.**
