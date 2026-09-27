---
total_phases: 5
created: 2026-09-26
phase: 2
agent: claude/opus
yolo: true
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

# Plan: `wt list` caption trusts a stale `origin/<default>`

Spec: `2026-09-26-stale-remote-caption` (`spec.md` in this directory).

## Summary and Definition of Done

### Work required

`wt list` prints "main is in sync with origin/main" from a remote-tracking ref that may be hours old. The fix has three parts:

1. **Library (`worktree`).**
    - Add a live-head store, `<repo hash>.remote-head.json`, with its lock `<repo hash>.remote-head.lock`. It records the answer of a background `git ls-remote origin refs/heads/<default>`.
    - Move the non-interactive transport from `worktree::remove::live_remote` to `worktree::live_remote`, and make it reject incomplete output.
    - Add the tracking-tip SHA to `Caption`.
2. **CLI (`worktree-cli`).**
    - Replace `wt internal-refresh-prs` with `wt internal-refresh`, which runs the PR half and the live-head half concurrently.
    - Make one launch decision per `wt list`, after any foreground PR request has settled.
    - Render the comparison as being against the **local tracking ref**, followed by an aged remote observation.
3. **Tests and docs.**
    - Tests: store and selection, real-Git detection, caption snapshots, worker orchestration, the no-foreground-wait proof, failure and cleanup, and the command/removal regression.
    - Docs: the worktree skill, `worktree/docs/performance-testing.md`, and the README.

Code locations this plan relies on (verified 2026-09-26):

| Concern | Location |
|---|---|
| PR store, select, refresh, lock | `worktree/lib/src/pull_requests.rs` (`select_cached`, `refresh`, `try_lock`, `save`, `unix_now`) |
| Transport to move | `worktree/lib/src/remove/live_remote.rs` (`LsRemote`, `run_noninteractive`, `drain`, `kill_tree`) |
| Transport callers | `lib/src/remove/{mod.rs,remote.rs,safety.rs}`, `cli/src/commands/remove/mod.rs` |
| Caption built from one `RefTips` snapshot | `lib/src/worktree.rs` `fill_worktree_statuses` (~L296–305), `lib/src/listing.rs` `Caption` |
| Default-branch resolution (cwd-based) | `lib/src/worktree.rs` `default_branch()` |
| Worker | `cli/src/commands/pr_refresh.rs`, `cli/src/args.rs` (`InternalRefreshPrs`), `cli/src/main.rs` |
| List orchestration | `cli/src/commands/list.rs` (`PrSeams`, `gather_prs`, `run_pipeline`), `cli/src/commands/list/tests.rs` |
| Rendering | `cli/src/commands/list_table.rs` (`TableFacts`, `caption_markup`, `pr_age_markup`) |
| Tests to extend | `cli/tests/list_table.rs`, `cli/tests/list_prs.rs`, `cli/tests/perf_pr_request.rs`, `cli/tests/perf_support/mod.rs`, `cli/tests/level2_list_verbose.rs` |

### Definition of done

- [ ] `wt list` makes **no** foreground network wait for the live head. A PR miss keeps its existing 300 ms foreground request.
- [ ] Each comparison sentence names the "local tracking ref". Remote-status text follows the spec's five-row table and uses the PR-age units ("less than 1 min ago", N min, N h, N days).
- [ ] With no `origin`, neither a comparison nor an observation is shown, even when leftover `origin/*` refs exist.
- [ ] One `wt list` launches at most one `wt internal-refresh <main>`. The worker's two halves publish independently, and neither suppresses the other.
- [ ] Failures, deadline expiry, truncated or malformed output, and origin/default changes during a request all leave the previous store bytes unchanged. Only a complete, successful response without the exact ref is stored as absence.
- [ ] `internal-refresh` is hidden from help and completions. No active code, test, doc, or skill still uses `internal-refresh-prs`. Historical specs are not rewritten.
- [ ] Removal's deadlines (`LIVE_CHECK_DEADLINE` 3 s, `PUSH_DEADLINE` 30 s) and safety rules are unchanged, and all existing removal tests pass.
- [ ] `just test` and `just lint` in `worktree/` pass. No detached worker survives any test fixture.
- [ ] The skill, `docs/performance-testing.md`, and the README describe the store, worker, and caption.
- [ ] The spec's frontmatter is left for the author. The terminal state is "implementation complete, ready for review".

## Phase 1 — Rulings, Spikes, and Baseline

### Necessary Rules

These rulings resolve ambiguities in the spec. Implementers follow them as written. If a spike contradicts one, stop and record the conflict in the implementation log before changing course.

1. **Module layout.** The store and its refresh go in a new module, `worktree::remote_head` (`lib/src/remote_head.rs`). The transport moves to `worktree::live_remote` (`lib/src/live_remote.rs`), and `remove::live_remote` is deleted, not re-exported. The CLI worker module is renamed `pr_refresh.rs` → `refresh_worker.rs`: `SUBCOMMAND = "internal-refresh"`, `Commands::InternalRefresh { repo }`.
2. **Shared store plumbing.** Move `try_lock` (the nonblocking `fs4` sidecar lock) out of `pull_requests.rs` into `cache.rs` as `pub(crate) fn try_lock_sidecar`, beside `repo_cache_file`. `unix_now` stays in `pull_requests` and is imported by `remote_head`. Both stores use the same `atomic_write`. Do not create a generic store abstraction; two stores do not justify one (Rule 2).
3. **Store schema.** The file is `{ format_version: 1, origin_digest, branch, sha: Option<String>, checked_at: u64 }`, with `sha: null` meaning verified absence.
    - `REMOTE_HEAD_FORMAT_VERSION = 1`.
    - `remote_head_store_path(main)` returns `repo_cache_file(main, "remote-head.json")`. The lock is `store.with_extension("lock")`, which gives `<hash>.remote-head.lock`.
    - An object ID is valid only if it is 40 or 64 lowercase hex characters (SHA-1 or SHA-256). Anything else in the file is a miss. Anything else in `ls-remote` output is invalid output.
4. **Selection API.** `select_cached_head(store, origin: Option<&str>, default_branch: Option<&str>, now) -> CachedRemoteHead`, where `CachedRemoteHead` is `Fresh(RemoteHead) | Stale(RemoteHead) | Miss`.
    - `RemoteHead { branch, sha: Option<String>, checked_at }`.
    - Fresh means `now - checked_at < 60`. Stale means 60 s or more. A future `checked_at`, digest mismatch, branch mismatch, or missing origin/default is a `Miss`.
    - `RemoteHead::is_stale_at(now)` and `is_future_at(now)` are re-evaluated by the renderer at render time.
5. **Refresh API.** `refresh_remote_head(store, main, clock, heads: &dyn RemoteHeads) -> RefreshOutcome`.
    - Reuse `pull_requests::RefreshOutcome`, adding a `NoDefaultBranch` variant.
    - Sequence: lock (contended means no request), resolve origin and default, recheck freshness (fresh means `AlreadyFresh`), `checked_at = clock()`, request, re-resolve **both** origin and default and discard on any change, then publish while still holding the lock.
    - The worker uses `LsRemote { base: main, deadline: REMOTE_HEAD_REFRESH_DEADLINE }`, with `REMOTE_HEAD_REFRESH_DEADLINE = 10 s`, and addresses the remote by the name `"origin"`.
6. **Path-explicit default branch.** Add `worktree::worktree::default_branch_in(repo: &Path)`, which runs the same algorithm through `git_from`. `default_branch()` delegates to it with the cwd. The worker and the refresh re-read use `default_branch_in(main)`, so tests never depend on the process cwd.
7. **Complete-output contract (transport audit).** `drain` returns `Result<String, String>`.
    - A `read_to_end` error, a grace-period timeout, or a disconnected channel on **stdout** makes `run_noninteractive` return `Err`, never `Ok("")`. A missing stderr keeps its current fallback message.
    - `LsRemote::live_head` returns `Err` when any non-empty output line is not `<valid oid>\t<refname>`.
    - An exit of 0 with no line for the exact ref is the only `Ok(None)`.
    - This tightens removal only by turning a would-be false "absent" into "could not reach". That is conservative for removal, so it counts as preserving removal's behavior. Record it in the implementation log.
8. **Launch decision (CLI).**
    - The PR thread in `run_pipeline` does, in order: read `origin_url(main)` once; select the PR store; on a PR `Miss` with an origin, run the existing 300 ms foreground request to completion; select the live-head store with `list.default_branch`; call `launch` **once** if `pr_stale || (origin.is_some() && head != Fresh)`.
    - A PR miss alone still does not launch, which matches current behavior.
    - `PrSeams` becomes `ListSeams { connect, launch }`. There is no live-head seam in listing because listing never asks the network.
    - The thread returns `(PrListing, Option<CachedRemoteHead>, origin_present: bool)`.
9. **Worker concurrency.**
    - `refresh_worker::run(repo)` validates the main checkout, then calls `run_halves(main, pr_half, head_half)` with two `fn(&Path)` parameters. Each runs in its own `std::thread::scope` thread, and both are joined before the worker exits. The worker still has no foreground joiner.
    - Tests inject blocking halves to prove independence.
    - A panic in one half must not stop the other: each half is joined separately and panics are discarded.
10. **Caption facts.**
    - `Caption` gains `tracking_sha: String`, taken from the same `RefTips` snapshot as its counts.
    - The CLI builds `RemoteFacts { default_branch, tracking_ref, tracking_tip: Option<&str>, answer: Option<&RemoteHead> }`. `tracking_tip` is `caption.tracking_sha` when a caption exists, otherwise `list.refs.remote("origin/<default>")`, from the same snapshot.
    - `TableFacts` gains `remote: Option<RemoteFacts>`, which is `None` when origin is absent. When origin is absent, `TableFacts.caption` is also forced to `None` (no comparison from leftover tracking refs).
    - `TableFacts::from_list(list, prs, remote)` is the agreed signature for Phase 3's parallel wave.
11. **Caption text and styling.**
    - The comparison sentence ends with a period.
    - The observation follows in the **same** `Prose` paragraph, so wrapping stays as it is today. Branch names use the existing `local_badge` and `remote_badge`, and the rest of the observation text is `<dim>` (the PR-age-line convention).
    - Extract the age text into one shared `age_text(seconds)` used by both `pr_age_markup` and the caption. It returns `less than 1 min` for under 60 s, then the existing `min`/`h`/`days` bands. `pr_age_markup` output must not change.
    - An answer that is future-dated at render time counts as unusable: "Remote state has not been verified."
12. **Tests must not leak workers.**
    - Any CLI test that runs `wt list` with an origin now launches a worker unless both stores are fresh.
    - Tests that assert "no worker" or count PR requests seed a fresh remote-head store through a new `MixedFixture::seed_remote_head_store(age, sha)` helper, which isolates the live half.
    - Cleanup waits for the **process** to exit (`refresh_workers(main)` empty, matching argv `internal-refresh`) **and** for both locks to be free. The PR lock alone is no longer proof.
13. **Live-hold fixture.** For the no-foreground-wait proof, the origin URL is `http://127.0.0.1:<port>/r.git`, pointing at a loopback listener that accepts and holds the connection. Git's HTTP transport reaches it directly, with no proxy assumption.
    - The fixture clears the `*_proxy` variables for the child and sets `GIT_CONFIG_NOSYSTEM=1` and `GIT_CONFIG_GLOBAL` to an empty temp file, so no user `insteadOf` or `http.proxy` applies.
    - The PR store is seeded fresh for that same origin URL, so the PR half makes no request.
14. **Credential test.** A loopback server that answers `401` with `WWW-Authenticate: Basic` proves that `GIT_TERMINAL_PROMPT=0` and `credential.interactive=never` fail without a prompt (stdin is null), and that nothing is stored.

### Spikes

- [x] **S1 — Git HTTP hold and 401 on loopback** (about 30 min, macOS; Windows via `./scripts/cross-check.sh` if the host is reachable; load the `os` skill first)
    - Confirm that `git ls-remote http://127.0.0.1:<port>/r.git refs/heads/main` connects to a holding listener and blocks until killed, and that `run_noninteractive` with a 500 ms deadline returns `Err` quickly and the listener sees the connection close (process-tree kill of `git-remote-http`).
    - Confirm a `401` responder produces a fast `Err` with no prompt under the transport's env.
    - Output: a short note in the implementation log with elapsed times and the minimal HTTP response bytes that work. This decides whether the fixtures in Rules 13 and 14 need anything beyond a raw `TcpListener`.
- [x] **S2 — Inventory of affected tests** (about 20 min, read-only)
    - List every test that runs `wt list` or the worker with an origin configured: `list_prs.rs`, `perf_pr_request.rs`, `level2_list_verbose.rs`, `perf_support/mod.rs`, and any `MixedFixture::with_*_origin` user.
    - For each, record whether it asserts worker count, request count, or "no launch", and whether it needs `seed_remote_head_store` or a cleanup change.
    - Also list every `internal-refresh-prs` or `pr_refresh` reference in active code, tests, docs, and skills. Exclude `fixes/_completed/**` and other historical specs.
    - Output: a table in the implementation log that Phase 4 works through.
- [x] **S3 — Absent-branch exit semantics** (about 10 min)
    - Confirm that `git ls-remote origin refs/heads/<missing>` exits 0 with empty stdout on a local bare origin and over HTTP. Confirm that `refs/heads/main` does not also match `refs/heads/foo/main`, since the parser keeps its exact-name comparison.

### Tasks

- [x] **Baseline green**
    - Run `just test` and `just lint` in `worktree/` on the unmodified branch. Record any pre-existing failures in `implementation-log.md` so they are not attributed to this fix.
- [x] **Log setup**
    - Create `implementation-log.md` in this fix directory, containing the rulings accepted and the spike results.

**Checkpoint 1:** the spikes are recorded, no ruling is contradicted (or each conflict is written up), and the baseline is known.

## Phase 2 — Library Foundations

### Wave 1 (parallel; disjoint files)

- [x] **Move transport** (`lib/src/remove/live_remote.rs` → `lib/src/live_remote.rs`)
    - `git mv` the file, declare `pub mod live_remote` in `lib/src/lib.rs`, and remove it from `remove/mod.rs` (fix that module's `//!` link text).
    - Update imports in `remove/remote.rs`, `remove/safety.rs` (including test modules), and `cli/src/commands/remove/mod.rs`.
    - Apply Rule 7: `drain` returns a `Result`, stdout failure or timeout becomes `Err`, and malformed `ls-remote` lines become `Err`. Update the module docs to state the complete-output contract.
    - Keep `LIVE_CHECK_DEADLINE`, `PUSH_DEADLINE`, `kill_tree`, and `batch_ssh_command` unchanged.
    - The existing transport tests move with the file and still pass. Add unit tests: a malformed line gives `Err`, a wrong-length OID gives `Err`, and exact-ref matching ignores `refs/heads/x/main`.
- [x] **Caption tracking SHA** (`lib/src/listing.rs`, `lib/src/worktree.rs`)
    - Add `tracking_sha: String` to `Caption` and fill it from the `remote_tip` already read in `fill_worktree_statuses`.
    - Reword the field docs: `ahead`/`behind` count against the **local tracking ref**, not the live remote. Fix the `Caption` and `remote` docs to match.
    - Update the struct literals in the unit tests and in `cli/tests/list_table.rs::Example` (compile only; behavior asserts come in Phase 3).
- [x] **Shared lock and default branch** (`lib/src/cache.rs`, `lib/src/pull_requests.rs`, `lib/src/worktree.rs`)
    - Move `try_lock` to `cache::try_lock_sidecar` (Rule 2) and update `pull_requests::refresh`. The PR tests must pass unchanged.
    - Add `default_branch_in(repo)` (Rule 6), make `default_branch()` delegate to it, and add a unit test that it answers for a path other than the cwd.

### Wave 2 (after Wave 1)

- [x] **Remote-head store** (`lib/src/remote_head.rs`)
    - Implement the schema, `remote_head_store_path`, `select_cached_head`, `RemoteHead` helpers, `refresh_remote_head`, and `REMOTE_HEAD_REFRESH_DEADLINE` per Rules 3–5.
    - Never store or log the origin URL; only `origin_digest`.
    - Module `//!` docs state the contracts: `checked_at` is taken before the request; a mismatch is not chronology; the lock is persistent and never unlinked; readers need no lock.
- [x] **Store tests (acceptance 1)** (unit tests in `remote_head.rs`, using `remove::test_support::TestRepo`, an injected clock, and a stub `RemoteHeads`)
    - Fresh at 59 s and stale at 60 s. Missing or changed origin, changed default branch, and future `checked_at` are misses.
    - Corrupt JSON, format 0/2, bad OID (short, uppercase, wrong length), and a missing field are misses. SHA-256-length OIDs are accepted. A verified absence (`sha: null`) is `Fresh` or `Stale`, not a miss.
    - Request failure, `Err` from invalid output, and publication failure (store path is a directory) leave the previous bytes byte-for-byte identical.
    - An origin change **and**, separately, a default-branch change made while a stub request is blocked are discarded. Use `std::sync::mpsc` readiness and release channels with `recv_timeout` bounds, never fixed sleeps.
    - Two competing refreshes, with the first blocked in its request, give one `Contended` and no second request. After a successful publication the next refresh returns `AlreadyFresh` with no request.
    - Real `LsRemote` against the local bare origin: present, absent, and pushed-elsewhere cases store the right `sha`.

**Checkpoint 2:** `just test` passes in `worktree/`, including all `remove::` tests (transport, safety, remote) and `cli/tests/remove.rs`, and `just lint` is clean. `grep -rn "remove::live_remote" worktree/` returns nothing.

## Phase 3 — Worker, Rendering, and Orchestration

### Wave 1 (parallel; disjoint files)

- [ ] **Shared worker** (`cli/src/commands/refresh_worker.rs`, `commands/mod.rs`, `args.rs`, `main.rs`)
    - Rename the module and the hidden command (Rule 1). Keep `main_checkout` validation, cwd = main, null stdio, the `WT_SHELL_WRAPPER`/`COMPLETE` env removal, `configure_detached_child`, and silent failure.
    - Implement `run_halves` (Rule 9). The PR half is the existing `pull_requests::refresh`. The head half is `refresh_remote_head` with `LsRemote` at `REMOTE_HEAD_REFRESH_DEADLINE`.
    - Update the module docs to describe both halves.
    - Unit tests (acceptance 4, worker side): with the PR half blocked on a channel, the head half publishes, and the reverse. A PR half that returns `Contended`, fails, or has an unsupported provider still lets the head half run. A failing head half still lets the PR half run. A panicking half does not stop the other. Keep the existing `main_checkout` and spawn-failure tests.
- [ ] **Caption rendering** (`cli/src/commands/list_table.rs`)
    - Add `RemoteFacts` and `TableFacts.remote`, and change the `from_list` signature (Rule 10). Extract `age_text` (Rule 11).
    - Rewrite `caption_markup` for the four comparison states with "local tracking ref". Add `observation_markup(remote, now)` covering the five spec rows, the "no tracking ref and no usable answer" sentence, and the stale-absent wording (past tense only).
    - `render` shows the caption paragraph when a comparison **or** an observation exists. No raw URL and no new protocol line; output stays on stderr.
    - Update the module and function docs (the authoring discipline).

### Wave 2 (after Wave 1)

- [ ] **List orchestration** (`cli/src/commands/list.rs`, `cli/src/commands/list/tests.rs`)
    - Implement Rule 8: `ListSeams`, a single launch after the foreground PR request settles, and a return of `(prs, head, origin_present)` from the PR thread. Pass `RemoteFacts` into `TableFacts::from_list`.
    - Keep the `pr gather` perf stage. Add a `remote select` perf stage that covers live-head selection plus the launch call, for the Phase 4 stage measurement.
    - Unit tests (acceptance 4, launch side), using counting `connect`/`launch` seams and a seeded store directory:
        - A stale PR plus a missing head launches exactly once.
        - Both fresh: no launch.
        - No origin: no launch, and no connect.
        - A PR miss plus a fresh head: foreground connect once and no launch, which is unchanged behavior.
        - A PR miss plus a stale head: `connect` returns before `launch` is called (record call order), so there is never a simultaneous foreground and worker PR request.
        - A live-head miss never calls `connect` for a live request (there is no such seam), so the foreground makes no live request.
    - The existing `NO_PRS` tests keep passing, since their repositories have no origin.

**Checkpoint 3:** `just test` passes. `wt list` in this checkout (manual smoke, `NO_COLOR=1`) shows the qualified caption and "Remote state has not been verified." on the first run, then an aged observation on a later run. `ps` shows no lingering `internal-refresh` process after about 15 s.

## Phase 4 — Integration Tests and Existing-Test Migration

### Wave 1 (parallel; separate test files; each task owns the `perf_support` helpers it adds, which are listed here to avoid collisions)

- [ ] **Migrate existing tests** (`cli/tests/perf_support/mod.rs`, `list_prs.rs`, `perf_pr_request.rs`, `level2_list_verbose.rs`; driven by S2's inventory)
    - Rename the argv matching to `internal-refresh` in `refresh_workers` and `refresh_worker_via_gitea`, and the help/completion assertions in `the_worker_command_is_hidden_from_help_and_completion`.
    - Add `seed_remote_head_store` and `remote_head_store()` (Rule 12). Seed a fresh head in every test that asserts no worker, a worker count, or a request count, so the PR behavior under test is isolated.
    - Change `finish_worker` and the L2 stale-PR cleanup to wait for process exit plus both locks (Rule 12).
    - Update module docs that name the old command.
- [ ] **Snapshot matrix (acceptance 3)** (`cli/tests/list_table.rs`)
    - Cover:
        - each of the five observation rows
        - all four comparison states
        - fresh and stale × matching, differing, and absent
        - missing local default with a tracking tip present
        - missing tracking tip with and without a usable answer
        - missing origin with leftover tracking refs (nothing rendered)
        - failed comparison (`caption: None`) with an observation
        - a `trunk` default name
        - the age boundaries 59 s, 60 s, 3599 s, 3600 s, 2 days − 1 min, and 2 days
        - future-dated at render time
    - Update `caption_variants_read_as_ruled` and `only_the_caption_count_is_colored_yellow` to the new wording while keeping the yellow-count assertion. Snapshots stay in this integration test (the skill's double-compile rule).
- [ ] **Real-Git detection (acceptance 2)** (new `cli/tests/list_remote_head.rs`, isolated `HOME`/`XDG_CACHE_HOME`, a local bare origin plus a pusher clone)
    - The pusher advances main. Run `wt internal-refresh <main>` synchronously as a direct child and wait for it, then `wt list`: the caption says "differs … run git fetch origin".
    - `git fetch` in the listed checkout, then `wt list`: shows "1 commit behind local tracking ref" **and** "matched the remote", with no new live request (store bytes unchanged, and the `checked_at` age still shown).
    - Remote deletion: "was absent on origin". Then `git fetch --prune`: "No local tracking ref origin/main; the remote branch …"/absent wording as the rows specify. Recreate the branch and refresh again: present.
    - A fetch newer than the cached observation: the caption says "differs" and never "moved" or "advanced".
    - Invoking from the main checkout and from a linked worktree resolves the same `remote-head.json` path.
    - A missing origin with leftover `refs/remotes/origin/main` shows no caption and starts no worker.
- [ ] **No foreground wait (acceptance 5)** (extend `cli/tests/list_prs.rs`; timing in `perf_pr_request.rs`)
    - Use the Rule 13 holding listener as the origin and seed a fresh PR store for it. Test two cases: live-head miss and live-head stale.
    - The captured `.output()` of `wt list` returns while the listener still holds the worker's connection. Assert one connection (the request count) and zero PR requests.
    - Clean up by closing the held connection, then waiting for the worker's process exit and both locks.
    - Add to `perf_pr_request.rs` a per-sample assertion that the `remote select` stage stays under 300 ms. Keep the existing full-command bound and `perf_list_meets_sla_with_a_stale_answer_and_a_blocked_refresh` unchanged.
- [ ] **Failure and cleanup (acceptance 6)** (lib tests in `live_remote.rs` and `remote_head.rs`, plus loopback servers from S1)
    - The Rule 14 `401` server gives a fast `Err`, no store change, and no prompt (stdin is null, and the test runs under nextest with no TTY).
    - A holding HTTP server with a 500 ms deadline gives `Err` within the bound, and the listener observes the close (tree kill). This runs on all OSes, unlike the existing Unix-only `hang.sh` test.
    - A stub reader that errors or times out gives `Err` from `run_noninteractive`, never an absence. A malformed line gives `Err` and preserves the store.
    - Every fixture that can spawn a worker uses a drop guard that closes held connections and waits for process exit, so assertion-failure paths also clean up.
- [ ] **Command and removal regression (acceptance 7)**
    - `wt internal-refresh` ignores a linked worktree, a subdirectory, and a missing path, and prints nothing (extend the existing `list_prs` worker test).
    - Run `cli/tests/remove.rs`, `cli/tests/level2_remove.rs` (tmux, `BISCUIT_TEST_REQUIRED_BACKENDS=tmux cargo nextest run -p worktree-cli --features terminal-tests -E 'binary(level2_remove)'`), and the lib `remove::` suites unchanged.

**Checkpoint 4:** `just test` and `just lint` pass in `worktree/`. Run `just test-l2` for `level2_list_verbose` (tmux) and `level2_remove`. Afterwards, `ps` shows no `internal-refresh` processes and no temp fixture directories remain. Optional but recommended, because the transport moved and changed: `./scripts/cross-check.sh --os windows worktree` for the transport, deadline, and worker tests (see the `os` skill). Record the result or state why it was skipped.

## Phase 5 — Documentation, Drift, and Final Validation

### Wave 1 (parallel)

- [ ] **Skill update** (`.claude/skills/worktree/SKILL.md`)
    - `wt list` section: the `remote_head` store and lock, the `internal-refresh` worker with two concurrent halves, the single launch decision, the caption wording rules, the no-origin suppression, and the new test helpers and cleanup rule.
    - `wt remove` section: the transport path is now `worktree::live_remote`, plus the complete-output contract.
    - Replace every `internal-refresh-prs` and `pr_refresh.rs` reference.
- [ ] **Docs** (`worktree/docs/performance-testing.md`, the worktree README's `wt list` description)
    - Describe the new stage measurement, the live-head refresh (background only, 10 s deadline), and the caption semantics.
    - Confirm that no new crate was added. If one was, update `docs/dependencies.md` and the area's `docs/dependencies.md`.

### Wave 2

- [ ] **Drift sweep**
    - `grep -rn "internal-refresh-prs\|pr_refresh\|remove::live_remote" worktree .claude` finds matches only under `fixes/_completed/**` and other historical specs.
    - Review `///` and `//!` docs on every changed symbol (`Caption`, `TableFacts`, `caption_markup`, `PrSeams`→`ListSeams`, `run_noninteractive`, `LsRemote`, `default_branch`) for drift, and record any drift that was fixed.
- [ ] **Final validation**
    - Run `just test` and `just lint` in `worktree/`, and `just test worktree` / `just test worktree-cli` from the root if those recipes apply.
    - Walk through every Definition of Done item and every acceptance criterion (1–7), recording where each is proven (test name) in `implementation-log.md`.
    - Set the log's final state to "implementation complete, ready for review". Do not move the spec to `_completed` and do not commit unless told to.
