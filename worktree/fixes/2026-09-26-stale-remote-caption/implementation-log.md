---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-wt-ux/worktree/fixes/2026-09-26-stale-remote-caption/spec.md"
plan: "worktree/fixes/2026-09-26-stale-remote-caption/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages: []
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - worktree/fixes/2026-09-26-stale-remote-caption/implementation-log.md
skills_files_updated_during_phase_1: []
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
