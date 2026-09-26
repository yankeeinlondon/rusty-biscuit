---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-wt-ux/worktree/fixes/2026-09-25-list-remove-performance/spec.md"
plan: "worktree/fixes/2026-09-25-list-remove-performance/plan.md"
implemented_by: "claude/default"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - sniff/lib/src/process.rs
    - sniff/lib/src/lib.rs
    - sniff/lib/Cargo.toml
    - playa/lib/src/detached/mod.rs
    - playa/lib/src/detached/tests.rs
    - playa/lib/Cargo.toml
    - biscuit-speaks/lib/src/detached.rs
    - worktree/lib/Cargo.toml
    - worktree/lib/src/pull_requests.rs
    - worktree/lib/src/remove/safety.rs
    - worktree/cli/src/commands/list.rs
    - worktree/cli/src/commands/list/tests.rs
    - worktree/cli/src/commands/list_table.rs
    - worktree/cli/src/commands/git_graph/tests.rs
    - worktree/cli/tests/list_table.rs
    - worktree/cli/tests/perf_support/mod.rs
    - worktree/cli/tests/level2_list_verbose.rs
    - Cargo.lock
docs_updated_during_phase_2:
    - docs/dependencies.md
    - sniff/docs/dependencies.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/worktree/SKILL.md
    - .claude/skills/sniff/architecture.md
    - .claude/skills/os/windows.md
    - .claude/skills/os/build-hosts.md
source_files_during_phase_3:
    - worktree/cli/src/args.rs
    - worktree/cli/src/main.rs
    - worktree/cli/src/commands/mod.rs
    - worktree/cli/src/commands/pr_refresh.rs
    - worktree/cli/src/commands/list.rs
    - worktree/cli/src/commands/list/tests.rs
    - worktree/cli/src/commands/remove/mod.rs
    - worktree/cli/tests/list_prs.rs
    - worktree/cli/tests/list_table.rs
    - worktree/cli/tests/perf_support/mod.rs
    - worktree/cli/tests/remove.rs
    - worktree/lib/src/remove/handoff.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/worktree/SKILL.md
packages:
    - sniff
    - playa
    - biscuit-speaks
    - worktree
    - worktree-cli
source_files_during_phase_5: []
docs_updated_during_phase_5:
    - worktree/README.md
    - worktree/docs/cli/list.md
    - worktree/docs/performance-testing.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/worktree/SKILL.md
source_code:
    - sniff/lib/src/process.rs
    - sniff/lib/src/lib.rs
    - sniff/lib/Cargo.toml
    - playa/lib/src/detached/mod.rs
    - playa/lib/src/detached/tests.rs
    - playa/lib/Cargo.toml
    - biscuit-speaks/lib/src/detached.rs
    - worktree/lib/Cargo.toml
    - worktree/lib/src/pull_requests.rs
    - worktree/lib/src/remove/safety.rs
    - worktree/lib/src/remove/handoff.rs
    - worktree/lib/src/remove/inventory.rs
    - worktree/cli/Cargo.toml
    - worktree/cli/src/args.rs
    - worktree/cli/src/main.rs
    - worktree/cli/src/commands/mod.rs
    - worktree/cli/src/commands/pr_refresh.rs
    - worktree/cli/src/commands/list.rs
    - worktree/cli/src/commands/list/tests.rs
    - worktree/cli/src/commands/list_table.rs
    - worktree/cli/src/commands/git_graph/tests.rs
    - worktree/cli/src/commands/remove/mod.rs
    - worktree/cli/tests/list_table.rs
    - worktree/cli/tests/list_prs.rs
    - worktree/cli/tests/remove.rs
    - worktree/cli/tests/perf_pr_request.rs
    - worktree/cli/tests/perf_support/mod.rs
    - worktree/cli/tests/level2_list_verbose.rs
    - Cargo.lock
documentation:
    - docs/dependencies.md
    - sniff/docs/dependencies.md
    - worktree/README.md
    - worktree/docs/cli/list.md
    - worktree/docs/performance-testing.md
completed_phase: 5
implemented: true
implementation_1: "2026-09-26T15:31:55-07:00"
---

# Implementation Log for 2026-09-25-list-remove-performance (5 phases)

## Phase 1

Phase 1 is investigation only: interface decisions, the approved-action matrix, an executable process/lock test approach, and a baseline. No production source was changed. A disposable spike test was added to `playa`, run on macOS and native Windows, and deleted; `git status` afterward shows only this log and the plan as untracked.

### 1. Current state (what the code does today)

- `pull_requests::open_pull_requests(store, now, connect)` (`worktree/lib/src/pull_requests.rs`): store format v1 `{format_version, fetched_at, source_repo, pull_requests}`. Fresh (`age < 60`, not future) returns without calling `connect`. Otherwise it calls `connect()`, which runs `SniffOpenPrSource::for_origin()`. That uses **`git_command` (cwd-relative)**, not a repository-explicit call. It fetches under 300 ms, saves on success, and returns stored results with `stale: true` on failure. The store carries no origin binding.
- `list.rs::run_pipeline` runs the PR work on a scoped thread beside `fill_worktree_statuses` and **joins it** before rendering, so a stale store always waits up to the deadline.
- `list_table::pr_age_markup` shows the age line only when `prs.stale` (request failed). Under the new contract it must depend on the age at render time.
- `run_handoff` (`worktree/cli/src/commands/remove/mod.rs`) calls the same `Facts::gather` as the first run. That does a full inventory, `assess` (PR lookup 2 s on a thread plus live `ls-remote` checks, 3 s each), and `preflight_remote_deletion` when a remote approval exists. It then verifies state, re-checks `DeleteIfSafe` against the full tier, and compares remote destination/endpoint/SHA.

### 2. Confirmed spec/implementation gaps and their regression cases

**Gap 1: `origin/<default>` counts without a live check.** `safety::classify` pass 1 returns `Safe(DefaultBranch("origin/main"))` straight from `for-each-ref`. The second run's local shortcut must admit only `refs/heads/<default>`, other `refs/heads/*` (not the branch itself), and `refs/tags/*`. Every `refs/remotes/origin/*`, the remote default included, goes through `RemoteHeads::live_head`. First-run classification stays as it is.

- Regression (lib, `safety.rs`): tip contained only by `refs/remotes/origin/main` (local `main` older, no other branch or tag). With a counting `StubHeads` answering `Ok(Some(<other sha>))`, the result does not allow deletion, and exactly one live call names `origin main`. With `Err(..)` it does not allow deletion. With `Ok(Some(tracking sha))` it allows deletion. With local `main` containing the tip, zero PR and zero live calls.
- Regression (CLI, `cli/tests/remove.rs`, local bare origin plus `pusher` clone): first run approves `DeleteIfSafe` on `origin/main`-only proof. Between runs the pusher force-pushes `main` back so it no longer contains the tip, while the local tracking ref is unchanged. Expect exit 3 before mutation, with the worktree directory, its registration, and the branch all intact.

**Gap 2: remote absence is treated as "unchanged".** `run_handoff` compares SHAs only when the fresh answer is `Present` (`now_sha.is_some() && …`). So `Present(X)` → `Absent` passes, and `→ Unavailable` passes and then fails the push *after* local removal (partial result).

- New rule (second run with a remote approval): the fresh live answer must be verified, meaning `Present` or `Absent`. `Unavailable` refuses (exit 3) before any mutation. Allowed pairs are approved `Some(X)` → `Present(X)`, and approved `None` → `Absent` (a no-op remote deletion, as today). `Some(X)` → `Absent`, `Some(X)` → `Present(Y)`, and `None` → `Present(_)` refuse. `MultiplePushUrls` and `ReinterpretedEndpoint` keep their existing refusals. The destination and endpoint comparisons are unchanged.
- Regressions (CLI, local bare origin): (a) approve with the destination present, delete it on the bare remote between runs → exit 3, nothing removed. (b) Approve present, then rename the bare origin's directory between runs, so the endpoint's spelling is unchanged but `ls-remote` fails → `Unavailable`, exit 3, nothing removed. (c) Approved absent → still absent → proceeds, "Nothing to delete on origin". (d) Existing: a moved head refuses, and a push after preflight fails the lease with the partial-result message.

**Handoff record version: no bump.** `RemoteApproval.observed_sha: None` already covers both "verified absent" and "unavailable at approval". Under the rule above, both accept only a verified `Absent` in the second run, which deletes nothing remotely. So the distinction never changes an outcome, and the serialized contract is unchanged. Phase 3 should document this meaning on the `observed_sha` field.

### 3. Approved-action matrix for the second run

The first run is unchanged: full `Facts::gather`, full report. The second run always consumes the token, checks the 60 s expiry, verifies the caller left, re-reads entry/head/branch/rules/baseline/full fingerprint, and releases the target cwd, all before anything below.

| `approvals.branch` | Remote approval | Local-branch safety facts | PR lookups | Live `ls-remote` for safety | Remote preflight (1 live head plus config reads) |
|---|---|---|---|---|---|
| `None` (detached) | none | none | 0 | 0 | 0 |
| `None` (detached) | some (all-`None` fields, no branch) | none | 0 | 0 | 0 (`facts.remote` is `None` for detached, as today) |
| `Keep` (includes first-run `KeepWithWarning`) | none | none | 0 | 0 | 0 |
| `Keep` | some | none | 0 | 0 | 1 |
| `Delete` (explicit: answer or `--force-branch`) | none | none | 0 | 0 | 0 |
| `Delete` | some | none | 0 | 0 | 1 |
| `DeleteIfSafe` | none | reconfirm: local proof → stop | 0 with local proof, else ≤ 1 | 0 with local proof or PR proof, else one per candidate `origin/*` ref until the first verified or the first failure | 0 |
| `DeleteIfSafe` | some | reconfirm with `force_remote = true` (destination excluded from every tier, open PR excluded) | as above | as above | 1 |

What downstream code actually needs:

- `execute` reads `facts.safety` **only** for the `Delete` "(N lost)" suffix (`unsafe_deleted && lost_commit_count() > 0`). Decision: when safety was not assessed, print `Deleted branch X` without the suffix. The first run already showed the lost commits and the caller confirmed. Do not invent a tier. Do not compute `lost_commits` locally for the suffix, since a PR-proved branch deleted with `--force-branch` would then be mislabelled as losing commits.
- `delete_on_origin` reads `facts.remote` and must still receive the fresh `RemoteState::Present { endpoint, sha, .. }` for the lease. The lease SHA stays the approved one (equal by the rule above).
- `render_report` currently renders `(Some(branch), None)` as "Detached HEAD". The second run's refusal paths must not claim that. Phase 3 either renders the branch heading with a dim "safety not rechecked" note, or prints only the refusal reason. Recommendation: print only the refusal reason (and the reconfirm notes for `DeleteIfSafe`), not the full first-run report.
- `hand_off`/`landing_root` are first-run only.

### 4. Interface decisions

**PR store (`worktree/lib/src/pull_requests.rs`)**

- `PR_STORE_FORMAT_VERSION = 2`. `StoreFile` gains `origin_digest: String` = `biscuit_hash::blake3_hash(&origin_url)` (hex BLAKE3 of the exact string `git_from(root, root, ["remote","get-url","origin"])` returns). The raw URL is never persisted and never passed on the worker's command line.
- `pub fn origin_url(repo_root: &Path) -> Option<String>`: repository-explicit (`git_from(root, root, …)`). Parent and worker both call it with the **main checkout** path. Missing or failed means no reusable answer.
- `pub enum CachedPrs { Fresh(PrListing), Stale(PrListing), Miss }` from `pub fn select_cached(store, origin: Option<&str>, now) -> CachedPrs`. `Miss` covers: no or corrupt file, other version, future `fetched_at`, digest mismatch, and no origin. A valid empty `pull_requests` is an answer (`Fresh`/`Stale`), never a miss. Boundary: `now - fetched_at < 60` is fresh; exactly 60 is stale (the existing convention and test).
- `pub fn fetch_and_publish(store, repo_root, origin: &str, now, source: &dyn OpenPrSource) -> PrListing` (foreground miss): fetch; on success re-read `origin_url(repo_root)` and publish only if it equals `origin`. Return the listing either way on success. On failure return `PrListing::default()` (no badges) and leave the store untouched.
- `pub fn refresh(store, repo_root, clock: impl Fn() -> u64, connect: impl FnOnce(&str) -> Box<dyn OpenPrSource>) -> RefreshOutcome` (worker). Steps: open or create the sidecar `<store>.lock` (`store.with_extension("json.lock")` or `repo_cache_file(root, "prs.lock")`, a persistent file that is never unlinked); `fs4::fs_std::FileExt::try_lock_exclusive` → `Ok(false)` gives `Contended`, `Err` gives `LockFailed`, and neither touches the store. Under the lock: re-read origin (missing → `NoOrigin`), `select_cached` → `Fresh` gives `AlreadyFresh` with no request. Otherwise take `started = clock()`, fetch, re-read origin (changed → `OriginChanged`, discard), and publish with `fetched_at = started`. Failure gives `Failed` with the store untouched. Hold the lock through publication, and release by dropping the `File`.
- Timestamp rule for both writers: `fetched_at` = the time the request **started** (conservative age).
- Remove `PrListing::stale`. Age-line visibility and minutes come from `fetched_at` versus the render-time `now`: show when `now - fetched_at >= 60`. A fresh selection that crosses 60 s before rendering then shows the line naturally.
- `OpenPrSource::source_repo()` stays. `SniffOpenPrSource { remote_url, deadline }` is built from the origin string: `LIST_DEADLINE` (300 ms) for foreground, and a new `REFRESH_DEADLINE` (10 s) for the worker, which nobody waits on.
- Windows note: `cache::atomic_write` has no retry. A transient `ERROR_ACCESS_DENIED`/`SHARING_VIOLATION` on replace (see `os/windows.md`) makes the worker's publish fail, which leaves the old store in place, and the next stale list relaunches. That is acceptable and needs no retry loop.
- fs4 gotcha: Rust std (1.89+) has an inherent `File::unlock`/`File::try_lock`, which shadow the trait methods of the same name. Release by dropping the file, or call `fs4::fs_std::FileExt::unlock(&file)` explicitly. `try_lock_exclusive` has no std twin.

**CLI list wiring (Phase 3)**

- `PrConnect` becomes `fn(&str) -> Box<dyn OpenPrSource>` (origin URL in). Add a launch seam `fn(&Path /*main checkout*/)` so the injected pipeline tests count launches without spawning.
- PR thread: `origin = origin_url(main)`, then `match select_cached(..)`: `Fresh` → listing; `Stale` → listing plus `launch_refresh(main)` (spawn only, never wait); `Miss` with origin → `fetch_and_publish`; `Miss` without origin → default.
- Hidden subcommand: `#[command(hide = true)] RefreshPrs { repo: PathBuf }` (spelled `wt internal-refresh-prs <main checkout>`), dispatched before any list work and printing nothing. Spawn: `Command::new(current_exe()).arg("internal-refresh-prs").arg(main).current_dir(main)`, with `Stdio::null()` ×3 and `sniff::process::configure_detached_child`, then `spawn()` and drop the `Child`. A spawn error is ignored. Do not set `WT_SHELL_WRAPPER`.

**Local proof (`worktree/lib/src/remove/safety.rs`)**

- `pub fn reconfirm(input: &SafetyInput, prs: &dyn PrSource, heads: &dyn RemoteHeads) -> Reconfirmation`, where `Reconfirmation { tier: Tier, notes: Vec<String> }`. Steps: one `for-each-ref --contains` with the same exclusions as `classify` (own local ref, `origin/HEAD`, the doomed destination when `force_remote`). A git failure gives `Tier::Unknown`. Local proof (`refs/heads/<default>` → `Safe(DefaultBranch)`, another `refs/heads/*` → `PrettySafe(LocalBranch)`, `refs/tags/*` → `PrettySafe(Tag)`) returns **before** `prs.lookup` or any `heads.live_head`. Otherwise run `prs.lookup` → `pr_evidence` (same exact-tip, source-repo, and open-PR-excluded-under-`force_remote` rules). Then every remaining `refs/remotes/origin/*`, **including `origin/<default>`**, goes through the existing live-check loop (live == tip or live == tracking). Otherwise `NotSafe`. Reuse `classify`'s loop by extracting it rather than copying it.
- `SniffPrSource::for_origin(base)` does only a local git call and no network, so constructing it eagerly is fine. The counting stubs prove zero `lookup`/`live_head` calls.

**`sniff::process` public API (Phase 2)**

- `sniff/lib/src/lib.rs`: `pub(crate) mod process` → `pub mod process`. Every existing item stays `pub(crate)`. The one new public item is `pub fn configure_detached_child(command: &mut std::process::Command)`, moved unchanged in behavior: Unix `process_group(0)`; Windows clears `HANDLE_FLAG_INHERIT` on this process's std handles, then sets `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW` (`0x0800_0208`).
- Windows binding: sniff uses `windows` 0.62, not `windows-sys`. Port the two calls to `windows::Win32::System::Console::{GetStdHandle, STD_*_HANDLE}` and `windows::Win32::Foundation::{SetHandleInformation, HANDLE_FLAG_INHERIT, HANDLE_FLAGS}`, adding the **`Win32_System_Console`** feature to sniff's `windows` dependency (`Win32_Foundation` is already on). This avoids adding `windows-sys` to sniff.
- Callers to migrate: `playa/lib/src/detached/mod.rs:684` (spool spawn), `playa/lib/src/detached/tests.rs:646`, and **`biscuit-speaks/lib/src/detached.rs:192`** (a consumer outside playa; biscuit-speaks already depends on sniff). Move the tests `detached_process_uses_a_new_process_group` (unix) and `detached_process_combines_all_required_windows_flags` to sniff. Check whether playa's `windows-sys` `Win32_System_Console` feature is still used after the move. Update the `.claude/skills/os/windows.md` reference, and sniff's module doc claim that "every child process sniff spawns goes through `run_with_timeout`". That stays true of sniff's own spawns, but the doc must describe the public helper as configuring a caller's intentionally detached, unbounded child.

### 5. Process lifecycle and lock spike (executed, then deleted)

A disposable test `playa/lib/src/detached/spike_lifecycle.rs` re-executed its own test binary in `parent`/`worker` roles and used `configure_detached_child` plus `fs4`:

1. A parent spawned with captured stdout/stderr (`.output()`) starts a detached worker (null stdio, cwd = a separate "main" directory) and exits. `.output()` returned while the worker still held the lock and was blocked on a release marker.
2. A second worker's `try_lock_exclusive` returned `Ok(false)` while the first held it. It exited without work.
3. On release, the holder replaced `prs.json` by write-temp + `rename` **while the sidecar stayed locked**. The store held the new bytes.
4. A lock holder killed with `Child::kill` released the lock, and the next worker acquired it and published.

Results:

- macOS (local): passed. Parent `.output()` returned in 4.6 ms, and the worker's cwd was the given main directory.
- Native Windows (`./scripts/cross-check.sh --os windows playa spike_`, build-win-native, over SSH, so inside the session Job Object): `spike_detached_lifecycle` PASS in 0.123 s (2 run, 143 skipped). Every step above held, including step 1 with the inherited-pipe fix. The worker's `Child::kill` crash release works on Windows file locks too.
- Linux and WSL2 were not run for the spike. The primitives (`process_group(0)`, `flock` via fs4, close-on-exec fds) are the same Unix path as macOS, and Phase 4/5 run the real tests on all four environments.

Real tests (Phase 4) follow the same shape: readiness and release via marker files polled with a bounded deadline, never fixed sleeps. Each test owns and reaps its children, including on assertion failure (a drop guard that writes the release marker and kills or waits).

### 6. Injecting a blocking local PR source without a test flag (proven on macOS)

sniff's `client_for_url` accepts `http://` origins, maps a host starting with `gitea.` to the Gitea flavor, keeps the scheme for the API base, and reqwest honors `HTTP_PROXY`. So with origin `http://gitea.test/o/r.git` and `HTTP_PROXY=http://127.0.0.1:<port>` (unset `NO_PROXY`/`ALL_PROXY`), the request arrives at a local plain-HTTP listener as `GET http://gitea.test/api/v1/repos/o/r/pulls?state=open&page=1&limit=50`, with no DNS and no TLS. A JSON body such as `[{"number":7,"html_url":…,"state":"open","head":{"ref":"feat/x","sha":…,"repo":{"full_name":"o/r"}},"base":{"ref":"main","repo":{"full_name":"o/r"}}}]` produced `PR #7` in a real `wt list` table, and the store was written.

- Phase 4 adds a `FakeGitea` helper beside `ProxyStub` in `cli/tests/perf_support/`. It counts requests, signals "request arrived" (a condvar or channel in-process, since the worker's request reaches the test's listener thread), holds the response until released, and then answers with success or 5xx/401. This covers blocking, success, failure, and authentication without contacting a provider or adding a user-facing flag.
- The same proxy counts git smart-HTTP traffic (`…/info/refs?service=git-upload-pack`) if a handoff test uses an `http://gitea.test/…` origin. That gives a binary-level "zero network requests in the second run" assertion for the keep and explicit-delete rows. Positive remote verification stays on the local bare origin (lib-level `RemoteHeads`/`LsRemote`, CLI `TestRepo`-style fixtures).
- The worker test sets `HOME`/`XDG_CACHE_HOME`, plus the real per-user path on Windows as `MixedFixture::pr_store` already does. The env is inherited by the detached worker because the spawn does not clear it.

### 7. Baseline (macOS dev host, 2026-09-26, `cd worktree && just test-perf`, 17 passed)

| Gate | Result |
|---|---|
| `perf_cache_cold_list_gather_meets_sla` | cold `list gather` 24.2 ms |
| `perf_cache_warm_list_gather_meets_sla` | warm 12.8 ms (cold reference 25.0 ms) |
| `perf_full_command_non_image_meets_sla` (fresh-equivalent, no PR store) | 56.7 ms |
| `perf_list_meets_sla_with_the_network_down` | cold 24.2 / warm 12.9 / full 55.1 ms |
| `perf_list_meets_sla_when_the_pr_request_hits_its_deadline` (today's stale behavior) | warm 12.7 ms, **full 359.6 ms**, `pr gather` 308–324 ms |
| `perf_subprocess_counts_meet_sla` | `list_worktrees` 6 git calls, base view 3 |

Note: `just -d worktree test-perf` from the repo root printed nothing on this host. `cd worktree && just test-perf` works.

Existing PR tests to update in Phase 3/4: `list_prs.rs` (`a_fresh_pr_store_makes_no_request_and_shows_its_badges`, `a_stalled_pr_request_stops_at_its_deadline_and_shows_stored_badges_with_their_age`, `with_the_network_down_the_table_shows_without_badges_and_nothing_is_stored`), `perf_pr_request.rs` (both gates), `perf_support::MixedFixture::seed_pr_store` (writes format v1, so it must write v2 plus the digest of the fixture's origin), and the unit tests in `pull_requests.rs`. The stalled-deadline gate's premise ("every run made the request") changes: with a seeded store, runs become stale hits. Keep a **miss** variant (no store) for the 300 ms foreground-deadline assertion.

### 8. Stale full-command gate method (fixed before implementation)

- Fixture: `MixedFixture::new().with_github_origin()` (or the `gitea.test` origin plus `FakeGitea` in blocking mode). Warm the untracked cache and the comparison cache with one warm-up run.
- Before **each** of 5 timed samples, reseed a matching store (v2, digest of the fixture's origin) aged well past 60 s (for example 12 min), so a worker's success can never turn a sample into a fresh-cache run. Images disabled (`TERM_PROGRAM`/`KITTY_WINDOW_ID` removed), stdout/stderr null for timing.
- The source blocks every request until the test ends (hanging proxy or `FakeGitea` held). Assert best-of-5 full command < 1 s, **and** that the rendered stderr of a non-timed run contains the cached badge and the "PRs as of 12 min ago" line.
- Deterministic no-wait proof (separate L1 test, not a timing gate): with the source held (request observed as arrived), the parent's `.output()` returns with exit 0 and the badge and age line before release. Then release, and the next list shows the refreshed answer. A 1 s ceiling alone would not catch a reintroduced 300 ms wait.
- Record fresh (seeded store age 0) versus stale best-of-5 and `pr gather` on the same host. 80 ms is an observation, not a threshold.
- Also retain the miss-path deadline gate, the network-down gate, and warm/cold `list gather`. Update `perf_subprocess_counts_meet_sla` for the added `remote get-url` on cache hits only if its scenario reaches the PR thread.

### 9. Conservative rules for remaining uncertainty

- Worker deadline: `REFRESH_DEADLINE = 10 s`. If a test needs a shorter bound it releases the fake source; there is no env override.
- Lock sidecar name: `<repo hash>.prs.lock` via `repo_cache_file`, never deleted.
- Hidden command: rejects a path that is not a Git main checkout by doing nothing (exit 0, no output). A failed worker never prints.
- Any second-run safety uncertainty (git failure, live `Unavailable`, PR `Unavailable` with no other proof) refuses `DeleteIfSafe` before mutation (exit 3), as today.

### Checkpoint

The interface notes (§4), action matrix (§3), gap regressions (§2), the executable process/lock and fake-source test approach (§5, §6), the baseline (§7), and the gate method (§8) are recorded. No destructive-action policy is unresolved. `just test` and `just lint` in `worktree/` are recorded below.

### Gates run at the end of Phase 1

- `cd worktree && just test`: 392 passed, 17 skipped (the skips are the `perf_` tier, which `test` filters out). No failures, pre-existing or new.
- `cd worktree && just lint`: passed for `worktree` and `worktree-cli`. The recipe does not invoke `cargo fmt`.
- `cd worktree && just test-perf`: 17 passed (baseline in §7).
- `./scripts/cross-check.sh --os windows playa spike_`: PASS on build-win-native (the disposable spike, since deleted).
- No tests were added in this phase. The requirement-to-test mapping for the phases that add behavior is §2 (gap regressions), §3 (per-row network counts), §6, and §8.

## Phase 2

The three foundation tracks are implemented, and every consumer compiles. The local-proof track (`safety.rs`) was delegated to one subagent. The helper move and the PR storage were done in the main session. The main session also reviewed the subagent's diff.

### 1. `configure_detached_child` moved to `sniff::process`

- `sniff/lib/src/lib.rs`: `pub mod process`. The only public item is `configure_detached_child`, and every existing item stays `pub(crate)`. Its behavior is unchanged: Unix `process_group(0)`; Windows clears `HANDLE_FLAG_INHERIT` on this process's std handles, then sets `0x0800_0208`.
- The Windows half was ported from `windows-sys` 0.61 to sniff's `windows` 0.62. `GetStdHandle` returns `Result<HANDLE>` there, and an `Err` is skipped, as is a null handle. `SetHandleInformation(handle, HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0))`. sniff gained the `Win32_System_Console` feature, and playa's `windows-sys` dropped it (nothing else in playa used it).
- Callers migrated: `playa::detached::spawn_scheduler` and `biscuit-speaks/lib/src/detached.rs::spawn_preparation`. The old playa function and its constant were deleted. `detached_process_uses_a_new_process_group` (unix) and `detached_process_combines_all_required_windows_flags` moved to `sniff::process::tests`.
- Doc drift fixed:
  - sniff's module doc now distinguishes its own bounded spawns from the public detached helper.
  - Two intra-doc links to private items became code spans, because the module is now rendered.
  - `.claude/skills/os/windows.md` names the new path.
  - `.claude/skills/sniff/architecture.md` describes the public helper.

### 2. PR storage (`worktree/lib/src/pull_requests.rs`)

This implements the Phase 1 §4 interface as specified, with these details:

- `fetch_and_publish(store, repo_root, origin, now, source) -> Option<PrListing>`. It returns `None` on failure, not `PrListing::default()`, so a caller can fall back to a stale listing. A success whose origin changed mid-request is returned but not stored.
- `refresh(store, repo_root, clock, connect: FnOnce(&str) -> Box<dyn OpenPrSource>) -> RefreshOutcome`. The outcomes are `Refreshed`, `AlreadyFresh`, `Contended`, `LockFailed`, `NoOrigin`, `OriginChanged`, `Failed`, and `PublishFailed`. The lock is `pr_lock_path(store)` = `store.with_extension("lock")`, which is `<repo hash>.prs.lock`. It is taken with `fs4::fs_std::FileExt::try_lock_exclusive` and released by drop. `clock` is read once, before the freshness recheck and the request.
- `origin_url(repo_root)` uses `git_from(root, root, ["remote","get-url","origin"])`, and an empty value counts as none. `origin_digest(origin)` is public so test fixtures can write a v2 store without hard-coding the algorithm.
- `PrListing::stale` was removed. `PrListing::is_stale_at(now)` is the single definition of the boundary (`fetched <= now && now - fetched >= 60`). `select_cached` and the age line both use it.
- `SniffOpenPrSource::for_origin()` was removed; callers build the struct from the origin string. `REFRESH_DEADLINE = 10 s` was added.
- Test origins use `https://prs.example.invalid/...`, because a developer's global `url.*.insteadOf` rule for `github.com` would rewrite what `git remote get-url` returns. No existing fixture isolates git config.

### 3. Local proof (`worktree/lib/src/remove/safety.rs`, subagent)

- `pub fn reconfirm(input, prs, heads) -> Reconfirmation { tier, notes }`, in this order:
  1. local `refs/heads/<default>` gives `Safe(DefaultBranch)`;
  2. another `refs/heads/*` gives `PrettySafe(LocalBranch)`, and `refs/tags/*` gives `PrettySafe(Tag)`. Steps 1 and 2 make zero calls;
  3. then `pr_evidence`;
  4. then `verified_remote_copy` over every `origin/*` ref, `origin/<default>` included;
  5. otherwise `NotSafe`.

  A git failure gives `Unknown` with no lookup.
- `classify` was refactored onto shared helpers (`containing_refs`, `local_branch_or_tag`, and `verified_remote_copy`, which is the old loop moved unchanged). Its first-run semantics are unchanged, including trusting `origin/<default>` without a live check, and every existing `assess` test passes. The test stubs now count calls: `StubPr::new(..).calls()`, and `StubHeads` records `"<remote> <branch>"`.

### 4. Interim CLI adaptation (Phase 3 replaces it)

Removing `stale`, `open_pull_requests`, and `for_origin` forced minimal CLI changes so that everything compiles and the suite passes:

- `list.rs`: `PrConnect = fn(&str) -> Box<dyn OpenPrSource>`. The new `gather_prs(store, main, connect)` does `origin_url(main)` → `select_cached`. `Fresh` returns at once; `Stale` and `Miss` (with an origin) run `fetch_and_publish` in the foreground under 300 ms, and a stale entry is the fallback on failure. **That is today's blocking behavior on a stale store, kept on purpose until Phase 3 swaps the stale arm for the detached launch.** The `list::tests` `no_prs` connector now panics, since those repositories have no origin and must never ask.
- `list_table::pr_age_markup` shows the line when `prs.is_stale_at(now)`. The unreachable "less than a minute" arm was folded into `..=59 => "{m} min"`, since a stale age is at least 1 minute.
- `tests/list_table.rs`: `Example`'s fetch time is now `NOW`, so the snapshots stay byte-identical. The age-line test was renamed to `the_pr_age_line_appears_once_the_badges_are_60_seconds_old`, and it covers 59 s, 60 s, future, and unknown.
- `perf_support::seed_pr_store` and `level2_list_verbose`'s seed now write format 2 with `origin_digest(origin_url(main))`.

### Requirement-to-test mapping (Phase 2 scope)

| Requirement | Test(s) |
|---|---|
| Freshness boundary: 59 s fresh, 60 s stale, stale still shows badges | `a_stored_answer_is_fresh_below_60_seconds_and_stale_from_60`, `staleness_needs_a_known_past_fetch_time` |
| Age decided at render time | `a_fresh_selection_that_crosses_the_window_before_rendering_reads_stale`, `the_pr_age_line_appears_once_the_badges_are_60_seconds_old` (cli) |
| Changed or missing origin is a miss even when fresh | `an_answer_for_another_or_no_origin_is_a_miss_even_when_fresh`, `origin_url_is_none_without_an_origin` |
| Empty answer is an answer | `a_stored_empty_answer_is_an_answer` |
| Corrupt, old (the exact v1 shape), other version, and future stores are misses | `corrupt_old_format_other_version_and_future_stores_are_misses` |
| Only a digest is stored, never the URL or credentials | `the_store_records_only_a_digest_of_the_origin` |
| Foreground failure stores nothing and preserves bytes, auth error included | `a_failed_foreground_request_leaves_the_store_untouched` |
| Origin change during foreground fetch is not stored | `a_foreground_answer_is_not_stored_when_origin_changed_during_the_request` |
| Refresh replaces stale, stamped at start, next read fresh (read/write/read twice) | `a_refresh_replaces_a_stale_answer_stamped_at_its_start` |
| Refresh fills a miss and rebinds to a new origin | `a_refresh_fills_a_miss_including_an_answer_for_another_origin` |
| Under-lock freshness recheck skips the request | `a_refresh_skips_the_request_when_the_answer_is_already_fresh` |
| Contention makes no request; release lets the next run; sidecar persists | `a_refresh_makes_no_request_while_another_holds_the_lock` |
| Lock open failure makes no request and leaves the store untouched | `a_refresh_that_cannot_open_its_lock_makes_no_request` |
| Failed/auth refresh preserves bytes and the stale answer | `a_failed_refresh_leaves_the_stored_answer_untouched` |
| Origin change during refresh discards the result | `a_refresh_discards_its_answer_when_origin_changed_during_the_request` |
| No origin: no request | `a_refresh_without_an_origin_makes_no_request` |
| Sidecar name beside the store | `store_file_sits_beside_the_comparison_cache` (extended) |
| Local proof makes zero PR or live calls (default, other branch, tag) | `reconfirm_accepts_local_proof_without_pr_or_network` |
| Gap 1: `origin/<default>`-only proof is live-checked (moved, error, gone refuse; tracking match accepts; exactly one `origin main` call) | `reconfirm_verifies_origin_default_live` |
| PR proof before live checks; excluded under `force_remote`; fork PR does not count | `reconfirm_accepts_an_exact_pr_before_any_live_check` |
| Destination, own ref, and `origin/HEAD` never count | `reconfirm_never_counts_the_destination_the_own_ref_or_origin_head` |
| Git failure is `Unknown` | `reconfirm_is_unknown_when_git_fails` |
| Detached helper keeps Unix process group and Windows flags | `sniff process::tests::detached_process_uses_a_new_process_group`, `detached_process_combines_all_required_windows_flags` |

All the new tests are L1 in `worktree`'s lib unit-test target, which `just test` runs. No name has a tier marker. The cross-process lock is proven here with two handles in one process (flock and LockFileEx both conflict per open file). The real two-process, crash-release, and captured-parent proofs belong to Phase 4, as planned.

### Gates run

- `cd worktree && just test`: 410 passed, 17 skipped (the `perf_` tier). `just lint`: clean after one `clippy::type_complexity` fix, a test type alias. `just test-perf`: 17 passed. `perf_list_meets_sla_when_the_pr_request_hits_its_deadline` still passes, because the interim stale path still waits.
- `cd sniff && just test`: 2858 passed. `cd playa && just test`: 194 passed. biscuit-speaks detached tests: 5 passed. `just lint` is clean in sniff, playa, and biscuit-speaks. None of these recipes invokes `cargo fmt`.
- `cargo check --target x86_64-pc-windows-gnu -p sniff -p playa --tests`: compiles. The warnings are pre-existing, in files this phase did not touch.
- Cross-OS (`./scripts/cross-check.sh`):

| OS | Package | Filter | Result |
|---|---|---|---|
| native Windows | `worktree` | `pull_requests` | 21/21 passed |
| native Windows | `worktree` | `reconfirm` | 5/5 passed |
| native Windows | `playa-cli` | `publication` | 2/2 passed; the `.output()` parent with a detached worker returned in 0.89 s |
| native Windows | `sniff` | `detached_process` | 1/1 passed |
| WSL2 | `worktree` | `pull_requests` | 21/21 passed |
| WSL2 | `worktree` | `reconfirm` | 5/5 passed |
| Linux | `worktree` | `pull_requests` | 21/21 passed |
| Linux | `worktree` | `reconfirm` | 5/5 passed |

  - A filter containing `|` breaks the remote shell (`syntax error near unexpected token '|'`), so pass one substring per run.
  - Linux archive mode failed twice with `target/release/deps/*.rmeta is not writeable`. These are the stale kache hardlinks in the `fix-wt-ux` clone already recorded in `os/build-hosts.md`. `--all-features` (the native path) ran green, and `build-hosts.md` now records that the links were still present on 2026-09-26.
- No pre-existing failures were encountered.

## Phase 3

Both command paths are wired. The list/refresh track was done in the main session. The handoff track was delegated to one subagent, and its diff was reviewed in the main session. Nothing was committed.

### 1. `wt list` → detached refresh

- **New `cli/src/commands/pr_refresh.rs`**, declared in `commands/mod.rs`, so both CLI targets compile it:
  - `launch(main)` spawns `current_exe() internal-refresh-prs <main>` with the main checkout as its working directory, null stdin, stdout, and stderr, and `WT_SHELL_WRAPPER`/`COMPLETE` removed. It applies `sniff::process::configure_detached_child` and drops the `Child`, so it never waits and never kills. A spawn error is ignored.
  - `run(repo)` is the worker. It runs only when `repo` is the top level of a main checkout: `rev-parse --path-format=absolute --show-toplevel --git-dir --git-common-dir`, with the toplevel equal to the canonical `repo` and the git dir equal to the common dir. It then calls `pull_requests::refresh(store, main, unix_now, …)` with `REFRESH_DEADLINE`. It prints nothing, and its outcome is discarded.
- **`args.rs`/`main.rs`**: a hidden `InternalRefreshPrs { repo }` subcommand, named by `pr_refresh::SUBCOMMAND`, dispatches straight to `pr_refresh::run` and never reaches list code. It does not appear in `--help` or in dynamic completion (tested).
- **`list.rs`**:
  - `gather_prs(store, main, PrSeams)`: `Fresh` returns the listing. `Stale` returns the listing and calls `launch(main)`. `Miss` with an origin runs `fetch_and_publish` under 300 ms, and a failure returns `PrListing::default()`. Without an origin it returns the default.
  - `PrSeams { connect, launch }` replaced the separate `PrConnect` argument, keeping `run_pipeline` at 7 parameters for clippy.
  - `--perf`'s "pr gather" now measures the origin lookup, plus the spawn when a launch happens.
- **`list_table.rs`** needed no change: Phase 2 already decides the age line from `is_stale_at(now)` at render time. A stored empty answer that is stale shows no badges and still shows its age; an unavailable answer shows neither (new test).
- **Test support**: `ProxyStub` holds its accepted streams in a shared list, gains `close_held()` and `wait_for_connections(n, limit)`, and counts a connection only after storing it. `MixedFixture::worktrees()` was added.

### 2. Handoff second run (subagent)

- `Facts::gather` split into `Facts::local`, which makes no network request, and `Facts::assess`, which runs the tiers and the `--force-remote` preflight. The first run calls both, so it is unchanged.
- `run_handoff`:
  1. Unchanged: token consumption, expiry, the caller-left check, re-reading the entry, tip, branch, rules, baseline, and full fingerprint, and the cwd release.
  2. `DeleteIfSafe` → `reconfirm_branch`, which is `safety::reconfirm` with `force_remote = remote approval present`. Anything that does not allow deletion refuses with exit 3 before mutation.
  3. A remote approval → one `preflight_remote_deletion` → `remote_changed`. That compares the destination and the endpoint, runs `unprovable_remote` (multiple push URLs or reinterpretation), and then requires `Present(sha) == approved` or `Absent` with approved `None`. `Unavailable` refuses (Gap 2).
- Second-run refusals print only the reason, never the first-run report, which is what removes the misleading "Detached HEAD" heading. `DeleteIfSafe` refusals add reconfirm's notes.
- `execute`'s "(N lost)" suffix requires assessed safety, so an explicit delete in the second run prints `Deleted branch X` without it (Phase 1 §3 decision).
- `lib/src/remove/handoff.rs`: a doc comment only, on `RemoteApproval.observed_sha` (Phase 1 §2 "no record version bump").
- How the zero-network proof works: origin is `http://gitea.test/o/r.git`, and every proxy variable points at a local listener that counts connections and closes them unanswered. Each test asserts that the first run reached the listener, so the proxy is honored, and that the second run added zero connections. A file-path bare origin could not prove this, because it never reaches the PR lookup.

### 3. Wave 4 contract check

- The hidden command dispatches only to `pr_refresh::run`. It has no list code path and no terminal: null stdio, and `CREATE_NO_WINDOW` on Windows through the helper.
- `git diff` touches no fingerprint (`inventory.rs`), included-file observation (`compare.rs`), or git status code.
- Action-matrix rows are covered below. Every refusal test asserts that the worktree directory, its registration, and the branch are intact.

### Requirement-to-test mapping (Phase 3)

| Requirement | Test(s) |
|---|---|
| Stale matching answer shown at once, one launch for the main checkout, no foreground request, parent writes nothing | `list::tests::gather::a_stale_answer_is_shown_at_once_and_refreshed_in_the_background` |
| Stale empty answer is an answer (no miss request, refresh launched) | `gather::a_stale_empty_answer_is_still_an_answer`; render: `list_table::a_stored_empty_answer_shows_no_badges_but_keeps_its_age` |
| Fresh: no request, no launch | `gather::a_fresh_answer_neither_requests_nor_refreshes`, `list_prs::a_fresh_pr_store_makes_no_request_and_shows_its_badges` |
| Miss: foreground request, stored, next run reads it, never launches | `gather::a_miss_requests_in_the_foreground_and_stores_the_answer` |
| Failed miss: no badges or age, nothing stored | `gather::a_failed_miss_shows_no_badges_and_stores_nothing`, `list_prs::with_the_network_down_the_table_shows_without_badges_and_nothing_is_stored` |
| Changed/missing origin never shows old badges | `gather::a_changed_origin_never_shows_the_old_badges`, `gather::without_an_origin_a_stored_answer_is_ignored_and_nothing_is_requested`, `list_prs::a_changed_origin_hides_the_stored_badges_and_starts_no_worker` |
| Real binary: captured `.output()` returns while the worker is blocked (the worker holds the lock), with badges, age line, and pr gather < 300 ms; one request; failed refresh leaves bytes; next stale list retries | `list_prs::a_stale_store_shows_its_badges_at_once_and_a_detached_worker_makes_the_request` |
| Worker prints nothing; ignores a linked worktree; failed refresh preserves the store | `list_prs::the_worker_command_prints_nothing_and_ignores_a_linked_worktree`, `pr_refresh::tests::only_the_top_level_of_a_main_checkout_is_accepted` |
| Hidden from help and completion | `list_prs::the_worker_command_is_hidden_from_help_and_completion` |
| Gap 1 (origin/<default>-only proof re-verified live) | `remove::a_branch_proved_only_by_origin_default_refuses_once_origin_drops_the_tip`, `…_is_deleted_while_origin_still_has_it` |
| Gap 2 (absent / unavailable / created / moved) | `remove::an_origin_branch_deleted_between_the_runs_refuses_with_nothing_removed`, `an_unreachable_origin_in_the_second_run_refuses_force_remote_with_nothing_removed`, `an_origin_branch_created_between_the_runs_refuses_with_nothing_removed`, `an_origin_branch_moved_between_the_runs_refuses_with_nothing_removed`, `an_origin_branch_still_absent_in_the_second_run_leaves_nothing_to_delete` |
| Lease failure after preflight → partial result | `remove::a_push_after_the_second_run_preflight_fails_the_lease_with_a_partial_result` |
| Keep / explicit delete / local proof make no second-run network request | `remove::keeping_the_branch_makes_no_network_request_in_the_second_run`, `an_explicitly_deleted_branch_makes_no_network_request_in_the_second_run`, `automatic_deletion_with_local_proof_makes_no_network_request_in_the_second_run` |
| No "(N lost)" suffix without assessed safety | `remove::the_handoff_carries_force_flags_to_the_second_run` (updated), `an_explicitly_deleted_branch_…` |

Tests that failed against the pre-phase code:
- the stale `gather` test (it made a foreground request);
- the `list_prs` stale test (pr gather took ≥ 300 ms against the hanging proxy);
- the subagent confirmed 7 `remove.rs` tests failed on the old handoff: both gaps and the zero-network rows.

All the new tests are L1. No segment carries a tier marker; `list_prs.rs`, `list_table.rs`, and `remove.rs` are auto-discovered test targets, and `worktree-cli` does not set `autotests = false`.

### Gates run

- macOS, `cd worktree`:
  - `just test`: 441 passed, 17 skipped (the `perf_` tier).
  - `just lint`: clean. The recipe runs clippy only, not `cargo fmt`.
  - `just test-perf`: 17 passed. The stalled-request gate is now a miss-path gate, because no store is seeded (pr gather 309–322 ms, full 371 ms). The network-down gate measured cold 24.3 ms, warm 12.8 ms, and full 64.3 ms.
- Native Windows, `./scripts/cross-check.sh --os windows worktree-cli <filter>`, one filter per run (`a_stale_store`, `worker_command`, `changed_origin`, `second_run`, `between_the_runs`, `origin_default`, `only_the_top_level`, `handoff`): 44 test runs, all passed. This includes the captured-parent stale test (4.7 s, a real run, not a skip), the `pre-push`-hook lease test, and the proxy-based zero-network tests.
  - A filter on a binary name (`list_prs`) matches 0 tests, because the filter is a test-name substring.
- Linux, `--os linux worktree-cli --all-features`: 271 passed.
- WSL2, `--os wsl worktree-cli`: 238 passed, 33 skipped (tiers).
- No pre-existing failures.

## Phase 5

Validation and documentation only; no source code changed in this phase. The main session ran every gate itself (one test coordinator, so timing gates never overlapped other builds) and wrote the docs. Nothing was committed or staged.

### Note on Phase 4

The log has no `## Phase 4` section: Phase 4 landed as commits `ae03b5381` (handoff safety), `143bb2c50` (refresh lifecycle), `156978301` (stale latency) and `c43a286d2` (`sysinfo` dev dependency), and `b76e3893a` ticked two of its three tasks. "Prove handoff safety" was left unchecked for verification. This phase verified it (every row below passes on all four OSes) and ticked it.

### Documentation

- `worktree/README.md`: the PR badge bullet now describes immediate stored answers bound to `origin`, the age line after 60 s, the background refresh, and the foreground 300 ms request only on a first run. A new sub-bullet under "standing in the worktree" describes the conditional handoff speedup without implying every removal is network-free, and states that dirty files are still read in full.
- `worktree/docs/cli/list.md`: the "PR badges" paragraph was drift. It said a stored answer is used only for 60 s and that the request always runs otherwise. Rewritten for stale-while-revalidate, `origin` binding, and single-request refresh.
- `worktree/docs/performance-testing.md`:
  - The "PR Request" section was drift: it claimed a fresh answer runs without the `git remote get-url` call. It now states the origin lookup on every hit, the detached worker and its lock, and the miss-only foreground request.
  - Added the 2026-09-26 measurement table including the new stale gate, its method (reseeded stale store, stays-stale check, per-sample `pr gather` < 300 ms), the deterministic no-join tests, and the fresh/stale comparison.
  - Added "`git status` Cost (Investigated, Not Changed)" with the spec's §3 findings and the no-watcher ruling.
  - `last_updated` and the `md hash` value were refreshed (the hash is stable on recompute).
- `docs/dependencies.md` (`worktree -> fs4`, `sysinfo` dev) and `sniff/docs/dependencies.md` (the `Win32_System_Console` feature) were already current from Phases 2 and 4; no change needed.
- The sniff, playa, and os skills already name `sniff::process::configure_detached_child` (Phase 2); a grep found no remaining reference to the old playa helper.
- `.claude/skills/worktree/SKILL.md`:
  - names the stale timing gate and the deterministic no-join proof;
  - records a new trap: `just test-l2 <substring>` matches test names only, and `just test-l2 -E '<filterset>'` fails with a bash syntax error inside `_test_l2`. The skill gives the direct `cargo nextest … -E 'binary(level2_remove)'` form with `BISCUIT_TEST_REQUIRED_BACKENDS=tmux`.
- Code-comment drift audit: `pull_requests.rs` module and constant docs, `list.rs::gather_prs`, `pr_refresh.rs`, and the `perf_pr_request.rs` docs all match the code. No code comment needed a change.

### Gates run

macOS (dev host):

| Command | Result |
|---|---|
| `cd worktree && just test` | 456 passed, 18 skipped (`perf_` tier) |
| `cd worktree && just test-perf` (alone, `-j 1`) | 18 passed |
| `cd worktree && just lint` | clean (clippy only; the recipe runs no `cargo fmt`) |
| `cd sniff && just test` | 2858 passed, 31 skipped |
| `cd playa && just test` | 194 passed, 8 skipped |
| `cargo nextest run -p biscuit-speaks detached` | 1 passed |
| `just lint` in sniff, playa, biscuit-speaks | clean |
| `BISCUIT_TEST_REQUIRED_BACKENDS=tmux BISCUIT_TEST_LEVEL_REQUIRED=2 cargo nextest run -p worktree-cli --features terminal-tests -E 'binary(level2_remove)'` | 9 passed, 1.3–11.2 s each (real tmux runs, no focus) |

sniff, playa, and biscuit-speaks have had no commits since Phase 2, whose native Windows `detached_process` / `publication` evidence still qualifies for them.

Cross-OS (`./scripts/cross-check.sh`, local tree including this phase's uncommitted docs):

| OS | Package | Args | Result |
|---|---|---|---|
| Linux (build-linux) | `worktree-cli` | `--all-features` | 285/285 passed. This includes the perf tier and the tmux `level2_remove` tests (0.6–3.1 s, real runs). |
| Linux | `worktree` | `--all-features` | 206/206 passed |
| native Windows (build-win-native) | `worktree-cli` | (L1) | 238 passed, 28 skipped (tiers); every `list_prs` lifecycle test ran 6.6–12 s; the handoff rows ran 5–13 s |
| native Windows | `worktree` | (L1) | 186/186 passed |
| native Windows | `worktree-cli` | `--all-features --no-capture perf_list_meets_sla` | 3/3 passed (serial) |
| WSL2 (build-win, archive mode) | `worktree-cli` | (L1) | 251 passed, 34 skipped; relocatable nextest archive extracted and run |
| WSL2 | `worktree` | (L1) | 206/206 passed |
| WSL2 | `worktree-cli` | `--all-features --no-capture perf_list_meets_sla` | 3/3 passed |

Same-host fresh-versus-stale full-command measurements (best of 5, from `perf_list_meets_sla_with_a_stale_answer_and_a_blocked_refresh`):

| Host | Fresh full / `pr gather` | Stale (blocked refresh) full / `pr gather` range |
|---|---|---|
| macOS | 63.3 ms / 6.9 ms | 62.4 ms / 6.7–11.0 ms |
| native Windows | 244.6 ms / 38.2 ms | 242.7 ms / 45.5–57.3 ms |
| WSL2 | 21.4 ms / 2.3 ms | 21.6 ms / 2.7–6.0 ms |

**One failure, from running the gates in parallel, then a clean serial rerun.** The first native-Windows perf run (`--all-features perf_list_meets_sla`, without `--no-capture`) failed `perf_list_meets_sla_when_the_pr_request_hits_its_deadline`: warm `list gather` was 120.7 ms against its 120 ms bound, with `pr gather` 351–452 ms.

- nextest ran the three perf tests at once. It runs each test in its own process, so `#[serial]` does not serialize them.
- The serial rerun (`--no-capture` forces one at a time) passed. It measured warm `list gather` 95.3 ms and `pr gather` 347–367 ms.
- That test and the `list gather` stage are not changed by this fix.
- Windows' serial warm `list gather` (92–95 ms) leaves little headroom under 120 ms. That is a pre-existing Windows margin, recorded here for review. It is not a regression from this fix.
- Run Windows perf gates only serially (`just test-perf` does `-j 1`; through cross-check pass `--no-capture` or `-j 1`).

No other failures, pre-existing or new. No leaked `internal-refresh-prs` workers were observed; every lifecycle test reaps its worker, and the `Reaper`/`finish_worker` guards run on failure too.

### Acceptance audit (spec "Acceptance criteria and testing")

| Spec criterion | Tests | Platforms that executed them |
|---|---|---|
| 1a. Stale answer renders at once, age line, detached refresh | `list_prs::a_stale_store_shows_its_badges_at_once_and_a_detached_worker_makes_the_request`, `list::tests::gather::a_stale_answer_is_shown_at_once_and_refreshed_in_the_background` | macOS, Linux, native Windows, WSL2 |
| 1a. Concurrent lists make at most one request; recheck under lock | `list_prs::concurrent_lists_and_workers_make_one_request_and_a_fresh_answer_stops_the_next`, lib `a_refresh_skips_the_request_when_the_answer_is_already_fresh` | all four |
| 1a. Worker crash releases the lock | `list_prs::a_killed_worker_releases_its_lock_and_a_later_worker_refreshes` | all four |
| 1a. Successful refresh visible on next list | `list_prs::a_detached_workers_answer_replaces_the_stale_one_on_the_next_list` | all four |
| 1b. Miss / other / missing origin → foreground request; changed origin never shows old badges; old format a miss | `gather::a_miss_requests_in_the_foreground_and_stores_the_answer`, `gather::a_changed_origin_never_shows_the_old_badges`, `list_prs::a_changed_origin_hides_the_stored_badges_and_starts_no_worker`, lib `corrupt_old_format_other_version_and_future_stores_are_misses` | all four |
| 1c. Failed refresh / origin change during request leaves store; future timestamp not fresh | `list_prs::a_failed_or_unauthorized_refresh_keeps_the_stored_answer`, `list_prs::an_origin_change_during_a_workers_request_discards_its_answer`, lib `staleness_needs_a_known_past_fetch_time` | all four |
| 1d. No wait on any platform, including Windows handle inheritance; worker cwd is main checkout, linked worktree not held | captured `.output()` tests above; `a_detached_workers_answer_replaces_the_stale_one_on_the_next_list` (cwd check and rename of the linked worktree); `the_worker_command_prints_nothing_and_ignores_a_linked_worktree` | all four, including native Windows |
| 1e. Stale full-command performance gate plus deterministic no-join proof | `perf_pr_request::perf_list_meets_sla_with_a_stale_answer_and_a_blocked_refresh` plus the `list_prs` captured-output tests | macOS, Linux, native Windows, WSL2 |
| 2. Keep / explicit delete make no second-run request | `remove::keeping_the_branch_makes_no_network_request_in_the_second_run`, `…an_explicitly_deleted_branch_makes_no_network_request_in_the_second_run` | all four |
| 2. Local default / other branch / tag proof make none | `remove::automatic_deletion_with_local_proof_makes_no_network_request_in_the_second_run`, lib `reconfirm_accepts_local_proof_without_pr_or_network` | all four |
| 2. PR-only / remote-only proof is rechecked and refuses when lost (Gap 1) | `remove::automatic_deletion_proved_only_by_a_pr_asks_the_provider_again_in_the_second_run`, `…proved_only_by_another_origin_branch_asks_origin_again…`, `a_branch_proved_only_by_origin_default_refuses_once_origin_drops_the_tip`, lib `reconfirm_verifies_origin_default_live`, `reconfirm_verifies_another_origin_branch_live` | all four |
| 2. `--force-remote` refuses when the live head changes, is absent, or is unavailable (Gap 2); only the preflight request; lease failure is a partial result | `remove::an_origin_branch_{deleted,moved,created}_between_the_runs_…`, `an_unreachable_origin_in_the_second_run_refuses_force_remote_with_nothing_removed`, `a_force_remote_second_run_makes_only_the_preflight_request`, `a_push_after_the_second_run_preflight_fails_the_lease_with_a_partial_result`, plus the endpoint and reinterpretation rows | all four |
| 3. Same-size, same-mtime edit of a large file and inside a nested repository refuses; read error refuses | `remove::a_same_size_same_mtime_edit_between_the_runs_refuses_and_keeps_the_edit`, `an_unreadable_dirty_file_refuses_the_handoff_with_exit_4_and_is_kept`, `an_unreadable_file_inside_a_nested_repo_refuses_the_handoff_with_exit_4`, Windows-only `inventory::fingerprint_fails_when_a_dirty_file_is_held_open_exclusively` | all four (the held-open test on Windows only) |

Plan Wave 7 checks:
- no raw origin URL in the store (`StoreFile` holds `origin_digest` only; `the_store_records_only_a_digest_of_the_origin`) or the worker's arguments (`internal-refresh-prs <main checkout>` only);
- no removal code reads `pull_requests` or the `.prs.json` store (grep of `lib/src/remove` and `cli/src/commands/remove`); the second run's PR proof is `SniffPrSource::for_origin`, a live request;
- no metadata fingerprint shortcut and no daemon; `inventory.rs` gained only the Windows-only test.

### Remaining evidence limitations

- Linux's perf numbers were not printed, because that run had no `--no-capture`. It passed with every tier in parallel, which is a stricter condition than the serial gate.
- The Windows-only held-open read-failure test and the Linux-only non-UTF-8 name tests run only where their platform allows, as designed.

Implementation complete, ready for review. The fix directory was left in place; `just complete` was not run.

## Implementation of Review Findings #1

> **started at:** 2026-09-26T15:31:55-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-wt-ux/worktree/fixes/2026-09-25-list-remove-performance/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'A foreground PR request can show badges from a previous origin' at 15:32:09
        - discovered: `fetch_and_publish` (`worktree/lib/src/pull_requests.rs`) declined to save but still returned the old listing after an `origin` change; `gather_prs` rendered it
        - fixed: `fetch_and_publish` now returns `None` when `origin` no longer matches after the request (store untouched); doc comment updated; `gather_prs` (`worktree/cli/src/commands/list.rs`) already maps `None` to no badges, so only its doc changed
        - unit test renamed to `a_foreground_answer_is_discarded_when_origin_changed_during_the_request` and now asserts `None`, one request, and no store
        - discovered: the 300 ms `LIST_DEADLINE` makes a hold/change/release sequence from the test thread racy
                - added `FakeGitea::before_reply` in `worktree/cli/tests/perf_support/mod.rs`, so the stub server changes `origin` while the request waits, then replies with a PR that would produce a badge
        - added L1 regression `list_prs::an_origin_change_during_a_foreground_request_shows_no_badges_from_the_old_origin`
                - a control run first requires `PR #7` to appear, so the later absence of badges cannot be a timeout
                - asserts two requests, no `PR #`, no "PRs as of" line, and no store
                - negative proof: this test failed when the old `fetch_and_publish` body was temporarily restored
        - drift fix: the `pull_requests` bullet in `.claude/skills/worktree/SKILL.md` now states this contract
        - results: `just test` (worktree) 457 passed, 18 skipped; `just lint` clean; `just check-tier-coverage worktree` 0 stranded
- work completed for 'A foreground PR request can show badges from a previous origin' at 15:35:24
- starting the work on 'The stale PR age line has no real-terminal style verification' at 15:35:24
        - discovered: the L2 tmux pane passes env as an inline `K='v' cmd` prefix, so the detached `internal-refresh-prs` worker inherits `HTTPS_PROXY`, `HOME`, and `XDG_CACHE_HOME`, and its lock sidecar lands in the fixture's cache
        - added L2 test `level2_list_verbose::level2_list_stale_pr_answer_shows_a_dim_age_line_in_tmux` (`worktree/cli/tests/level2_list_verbose.rs`)
                - seeds a stale answer (12 min 5 s) bound to the current origin; asserts the `PR #99` badge still shows, "PRs as of 12 min ago" is the row directly after the legend, and every cell of it is dim (SGR)
                - the worker's request goes to `ProxyStub::hanging`; the test closes it, waits for the lock to be free and for no worker process, then asserts one connection and an unchanged store
                - fixture: `DesignFixture::with_pr_age`, `seed_stores(pr_age)`, `list_until`, and `probe_refresh`; the file now includes `mod perf_support`
        - extended `level2_list_styles_follow_the_design_in_tmux` to assert no age line for a fresh answer (control)
        - negative proof: with the dim tags removed from `pr_age_markup`, the new test failed on the dim assertion; `list_table.rs` was restored and has no diff
        - drift fix: one sentence added to the `wt list` styling bullet in `.claude/skills/worktree/SKILL.md`
        - results:
                - L2 binary with `BISCUIT_TEST_REQUIRED_BACKENDS=tmux`: 6/6 passed across 4 runs
                - `just test` (worktree): 457 passed, 18 skipped
                - `just lint`: clean; `cargo clippy -p worktree-cli --features terminal-tests --all-targets -- -D warnings`: clean
                - `just check-tier-coverage worktree`: 0 stranded
- work completed for 'The stale PR age line has no real-terminal style verification' at 15:41:51
- cross-OS check of the new L1 regression (`just cross-check` with the name filter `foreground`)
        - native Windows: 3/3 passed, including `an_origin_change_during_a_foreground_request_shows_no_badges_from_the_old_origin` (5.4 s), which uses the real per-user cache path there
        - the first Windows attempt used the binary name `list_prs` as the filter and ran 0 tests; the filter matches test names only
        - Linux (build-linux): blocked by the environment, twice; rustc reported that files under `target/release/deps` in the standing clone are not writable, so no test ran; this needs a manual fix on that host and is unrelated to this change; CI's Linux leg will cover it
        - the new L2 test uses tmux and skips on Windows, as designed

### Successful Completion

The implementation of review cycle 1 has completed successfully in 10 minutes. During this implementation all 2 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 0 were deferred (see reasons below):

- no findings were deferred
- no performance measurement was deferred, so `deferred_perf_measurement` stays unset
- the files changed in this cycle:
        - `worktree/lib/src/pull_requests.rs`
        - `worktree/cli/src/commands/list.rs` (doc only)
        - `worktree/cli/tests/list_prs.rs`
        - `worktree/cli/tests/perf_support/mod.rs`
        - `worktree/cli/tests/level2_list_verbose.rs`
        - `.claude/skills/worktree/SKILL.md`
