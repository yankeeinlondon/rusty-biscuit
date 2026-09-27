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
