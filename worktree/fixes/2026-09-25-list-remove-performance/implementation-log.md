---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-wt-ux/worktree/fixes/2026-09-25-list-remove-performance/spec.md"
plan: "worktree/fixes/2026-09-25-list-remove-performance/plan.md"
implemented_by: "claude/default"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
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
