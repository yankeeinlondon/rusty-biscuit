---
total_phases: 5
created: 2026-10-02
phase: 1
agent: claude/sonnet
yolo: true
source_files_during_phase_1:
    - worktree/lib/src/pull_requests.rs
    - worktree/lib/src/remote_head.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - worktree
---

# Plan: refresh open pull requests on every `wt list` run

## Summary and definition of done

Today `wt list` waits for the `origin/<default>` check but not for the open-PR query, so PR badges are almost never this run's answer and PR failures are silent. This plan makes the detached worker the **only** PR writer, makes its PR half ask on every run, writes a completion receipt on **every** attempt, and makes the ordinary wait (3 s) cover both halves. The listing then renders head and PR outcomes independently (the §5 table of the spec).

Work splits across two packages:

- `worktree` (lib): `pull_requests.rs` (store format 5, `refresh`, `PrStatus`), the receipt types in `remote_head`.
- `worktree-cli`: `refresh_worker.rs`, `list/wait.rs`, `list.rs`, `list_table.rs`, the hidden `internal-refresh` parser.

Net effect is mostly deletion: `fetch_and_publish`, `LIST_DEADLINE`, `ListSeams::connect`, `origin_pr_source`, `RemoteAnswers::pr_failure`, `Writer`, `force` on `refresh` and on the worker, `RefreshOutcome::AlreadyFresh`, `PrStatus::SkippedFresh`, the post-wait PR-lock probe, and `pr_pending` hint logic.

**Done means** (the spec's Acceptance, made observable):

- [ ] Two sequential `wt list` runs each make one logical PR query even with a fresh store; both halves finishing inside 3 s shows the new answer with no status item and no hint.
- [ ] A failing PR request shows `- PRs as of <age> ago (couldn't refresh)` (or `- couldn't get open PRs` with nothing stored); a stored empty answer is kept as an answer.
- [ ] Head done + PR held: one 3 s wait, completed head caption, refresh hint. PR published + head held: new badges, no PR item, timeout hint.
- [ ] Overlapping listings keep at most one PR query in flight; no wait is extended by contention.
- [ ] `grep` shows no PR-store write outside the worker path.
- [ ] `just test`, `just test-l2`, `just test-perf`, `just lint` pass in `worktree/`; docs and code comments match the new behavior.

Constraints for every task: US English; never run `cargo fmt`; never commit; no agent attribution; the sub-agent tree is non-interactive; L2 tests must not focus any window; on completion the agent stops at "implementation complete, ready for review" (never moves the spec to `_completed`). Read the `worktree`, `rust-testing`, and `biscuit-terminal` skills before starting; load `os` before touching path comparison or Windows cache-path isolation in tests.

## Phase 1: Foundations in the `worktree` library

Goal: the library offers the new store, the new `refresh`, and the new outcome vocabulary, so the CLI can be changed against a stable surface.

### Necessary Rules

Rulings the author should confirm (the plan proceeds on the stated default; none block Phase 1 unless marked):

1. **Receipt format is not versioned.** Receipts live at most 75 s (`ATTEMPT_MAX_AGE`). Removing `PrStatus::SkippedFresh` and adding `Unsupported` changes the serialized `prs` variants; a receipt carrying an unknown variant is "missing" for the wait. Default: no version bump, no compatibility reader.
2. **Store format 5 validation.** Format 5 drops `writer` only. A file that still has `writer` with `format_version: 5`, or any other unknown field, is treated per the matrix below (default: unknown fields are ignored only if the existing store already tolerates them; otherwise a miss). Task 1.1 states which in code and in a test.
3. **"Still running at 3 s" with an answer younger than 60 s.** §5 shows the age item only at 60 s or more, but the refresh hint always prints on timeout. Default: a young answer gets the hint and no age item.
4. **PR success observed through a changed publication id** (before the combined receipt exists, head still held) counts as success and outranks a later missing receipt. A receipt that arrives later with `Failed` for the same attempt does not override an observed new publication id (the store is the authority on whether a publication happened). Default as stated.
5. **Forced wait, second contention.** The one relaunch gets a fresh attempt id and its own receipt; the foreground deletes both receipts at return. Default as stated.
6. **Wider perf measurement is not scheduled.** The spec asks for one quick before/after sample on the development host, with and without authentication. Multi-host or statistical measurement is out of scope; the author decides whether to widen it.
7. **Unsupported detection point.** `PrStatus::Unsupported` is recorded where `PrFailure::from_unavailable` today folds `PrUnavailable::Unsupported` into `Other`. `from_unavailable` keeps its signature for the remaining conditions; the `Unsupported` case moves out of it, so it must never reach `PrFailure`.

No spikes are scheduled. The spec's own figures answer the performance question (cost is `max(head, PR)` capped at 3 s, deliberately accepted), and the one measurement the spec requires is a deliverable in Phase 5, not a pre-implementation risk reducer.

### Input Robustness Matrix

The work changes readers of two on-disk formats: the PR store (`<hash>.prs.json`, format 5) and the completion receipt (`<hash>.refresh-receipt.<id>.json`). Outcomes below are asserted through the public result (`select_cached` / `stored_publication` for the store; the wait result for the receipt), not through the parser's return value.

Store (`pull_requests`), per load-bearing field:

| Shape | `format_version` | `publication` | `origin_digest` | `fetched_at` | answer list | one element |
|---|---|---|---|---|---|---|
| absent | Miss | Miss | Miss | Miss | Miss (not empty) | n/a |
| explicit null | Miss | Miss | Miss | Miss | Miss (not empty) | Miss |
| wrong type, whole field | Miss | Miss | Miss | Miss | Miss | n/a |
| wrong type, one element | n/a | n/a | n/a | n/a | Miss (never filtered) | Miss |
| wrong type, every element | n/a | n/a | n/a | n/a | Miss (not empty) | Miss |
| empty (`[]`, `""`) | `""` Miss | `""` Miss (no usable id) | `""` Miss | n/a | `[]` valid empty answer | n/a |
| duplicate key | Miss | Miss | Miss | Miss | Miss | Miss |
| trailing/invalid content | Miss | Miss | Miss | Miss | Miss | Miss |
| format 4 (`writer` present) | Miss | | | | | |
| future `fetched_at`, other origin | | | | Miss | | |

Receipt, per load-bearing field (`id`, `origin_digest`, `branch`, timestamp, `head`, `prs`): every row above (absent, null, wrong type, empty, duplicate, trailing) is **"missing receipt"**, never an empty or default outcome, and never a PR success. Wrong id, origin, branch, an invalid or pre-attempt timestamp, and an unknown `prs` variant are "missing" too. A `prs` of `Ok` with no store publication is still the receipt's answer, not an invented one.

Rules:

- one test per format walks the whole table from a fixture written by the real writer, one edit per cell, with an unedited control row that gives the positive result;
- before declaring the matrix done, grep the touched readers for `#[serde(default)]` on a load-bearing field, `Option<T>` where absent and null must differ, and `filter_map(.. as_str())`, `unwrap_or_default()`, `.ok()` on a load-bearing parse.

### Wave 1 (sequential; the surface everything else builds on)

- [x] **Store format 5** (`lib/src/pull_requests.rs`)
    - Bump the format to 5 and drop `writer` from the store, `Publication`, and every writer. `Publication` becomes id-only; `stored_publication(store, origin, now)` returns the id under `select_cached`'s rules (same-second and empty answers count; corrupt, other-format, future-dated, wrong-origin do not).
    - Delete `Writer` and the writer-based filters. Keep `publication`, `origin_digest` (via `biscuit-hash`), `fetched_at`, answer fields, validation.
    - Prerequisite: none. Complexity: the Input Robustness Matrix above; implement the store reader so absent, null, and wrong-type never collapse into an empty list.
- [x] **Ask every time** (`lib/src/pull_requests.rs`)
    - `refresh` loses `force` and its freshness recheck: holding the lock means making the request. Remove `RefreshOutcome::AlreadyFresh` and `PrStatus::SkippedFresh`.
    - Keep: nonblocking `<hash>.prs.lock` held from before the request through publication (sidecar never deleted), `Contended` on contention, `REFRESH_DEADLINE`, the origin re-check after the request, "a failure is never stored", `PrStatus::Ignored` for `~/.wt.json`.
    - A successful write stamps a new publication id and `fetched_at` at the request's start.
- [x] **Unsupported outcome** (`lib/src/pull_requests.rs`, `remote_head` receipt types)
    - Add `PrStatus::Unsupported`, recorded by the PR half when sniff reports `PrUnavailable::Unsupported`; take that case out of `PrFailure::from_unavailable`.
    - A local-path or unsupported origin still gets the head check; the PR half makes no request and stores nothing.
- [x] **Remove the foreground request** (`lib/src/pull_requests.rs`)
    - Delete `fetch_and_publish`, `LIST_DEADLINE`, and tests that exercised them or the two-writer ordering (`a_foreground_answer_never_replaces_*`).
    - `FRESHNESS_WINDOW` stays only as the age-display threshold (`PrListing::is_stale_at`).
- [x] **Library tests**
    - `refresh` asks while a fresh answer is stored (replaces the "fresh answer stops the next" half of `concurrent_lists_and_workers_make_one_request_and_a_fresh_answer_stops_the_next`).
    - Contended, Ignored, Unsupported, Failed-not-stored, origin-changed-not-stored; the store matrix test; format-4 file is a miss.
    - Comment/doc pass over every touched symbol (stale claims: "fresh answers skip requests", "listing writer").

Validation checkpoint 1: `just test` and `just lint` pass for the `worktree` package; the CLI crate is expected not to compile yet, so run lib-only (`cargo nextest run -p worktree`).

## Phase 2: Worker, wait, and rendering vocabulary

Goal: the three CLI pieces that sit on the library surface, built in parallel against a fixed contract (the receipt shape and a pure `PrOutcome` fact type).

### Wave 2 (three concurrent tracks; disjoint files)

- [ ] **Worker writes every receipt** (`cli/src/commands/refresh_worker.rs`, `cli/src/main.rs`/clap definition of `internal-refresh`)
    - Write the receipt after both halves join on every attempt; contents unchanged (`head`, `prs`, bound to attempt id, origin digest, branch).
    - Remove `force` from `LaunchArgs` and `--force` from the hidden command and its parser; update all callers.
    - Run `remove_stale_receipts` (older than `ATTEMPT_MAX_AGE`) before writing, for ordinary and forced attempts; never remove another active attempt's receipt.
    - Halves still join separately; a panic in one becomes a recorded failure and never suppresses the other. A receipt write failure must neither hang the wait nor turn a successful store write into an empty PR answer (the store, not the receipt, is the evidence of success).
    - Tests: receipt present with no `--force` carrying `prs: Ok`, `Failed`, `Contended`, `Ignored`, `Unsupported`; panic isolation; sweep of old receipts only.
- [ ] **Ordinary wait covers both halves** (`cli/src/commands/list/wait.rs`; pure over `WaitEnv`)
    - Ends at the first of: head outcome **and** PR result resolved from our receipt; our worker exited without a usable receipt and no matching head attempt is still running; the 3 s `ORDINARY_BUDGET` (one monotonic budget started before launch; adoption, contention, and retries never reset it).
    - Replace the `if !request.force { return … Finished … }` early exit in `Follow::run`; keep forced-versus-ordinary only as budget and retry policy.
    - Contention: record the usable publication id before launch; after the receipt reports `Contended`, wait within the budget for the lock, then reread the id. A nonempty id different from the pre-launch id (same timestamp or empty answer included) proves success. Released lock alone proves nothing. No new id: ordinary reports PR failure; forced relaunches once (fresh id), a second contention is generic `Failed`. Probe the lock only after the worker reported `Contended` or exited.
    - Remove the post-wait refresh-hint probe, keep the contention probe. `WaitEnv::pr_publication` becomes id-only.
    - Adoption: read our own receipt by launched id even when following another head id; a completed holder is followed only when our receipt proves head contention.
    - Independent outcomes: the wait result carries the head outcome and any PR result (receipt, or a changed publication id) even on timeout or when no head attempt was recorded. Missing/malformed receipt: process exit is read before the final store and receipt reads; retain verified head outcome and a new publication; otherwise generic PR failure, no invented credentials reason; never wait past the budget.
    - Cleanup: `discard_receipt` for every launched id (including a retry, not an adopted head's receipt), for ordinary and forced waits.
    - Tests (scripted `WaitEnv`): every bullet in the spec's `wait::wait` list, including that a probe cannot contend with our just-launched worker and that retries share the budget; plus the receipt matrix test.
- [ ] **Status-list rendering** (`cli/src/commands/list_table.rs`, `cli/tests/list_table.rs`)
    - Introduce the pure per-run PR presentation (`PrOutcome` with the §5 rows: observed success, ignored, unsupported, still running, failed with stored answer, failed with nothing stored) and render it through `Prose` and `UnorderedList` (dim, stderr, after graph/legend, before verbose; PR item before hint). No raw escape sequences.
    - `- PRs as of <age> ago` at ≥ 60 s while still running; `(couldn't refresh)` at any age on failure; `- couldn't get open PRs` with nothing stored; pending with nothing stored shows only the hint. Use the existing age formatter.
    - The hint condition becomes "the wait timed out". Caption follows the head half only: completed or failed caption when PRs caused the timeout; "still checking/pulling" only while the followed head is unfinished.
    - A `list_table` snapshot covering every row of the §5 table (integration test file, not a unit test; see the skill's snapshot caveat).

Validation checkpoint 2: each track's own unit tests pass (`just test` for the area compiles only once Phase 3 lands; until then run each module's tests with `cargo nextest run -p worktree-cli <filter>` where the crate compiles, otherwise defer to checkpoint 3).

## Phase 3: Listing flow integration

Goal: `list.rs` consumes the new wait result, the stored answer, and the receipt; one code path for ordinary and forced listings.

### Wave 3 (sequential: touches `list.rs` and its seams)

- [ ] **Remove the foreground request** (`cli/src/commands/list.rs`)
    - `gather_remote` no longer requests on a miss. Delete `ListSeams::connect`, `origin_pr_source`, `RemoteAnswers::pr_failure`; keep `ListSeams { launch, wait_budget, forced_budget }` so tests stub the launched worker.
    - Always launch exactly one worker when there is an `origin` (skip for `--ignore-api`'s provider query only inside the worker); no origin: no worker, PR item, or hint.
- [ ] **Post-wait answer selection**
    - Reread the PR store after the wait on every exit path (timeout, worker failure). A successful empty answer clears badges and counts as success. Keep source-repository filtering for table badges and graph tags.
    - Recheck the current origin before selecting the answer; if it changed or disappeared, discard the old origin's badges and the receipt-derived PR diagnosis and do not start another refresh.
    - Map wait result + store + receipt to `PrOutcome` (§5 table, rulings 3 and 4). Pending PR failure that is unobservable at timeout shows the pending presentation.
- [ ] **Credentials and spinner**
    - `list::credential_line` reads the PR failure from the receipt in every mode (ordinary too); precedence unchanged (confirmed head API condition beats PR condition; ambiguous 404/timeout/unsupported/lock/write never asserts a bad key; never print key values, URLs, or raw provider text). The status item does not repeat the reason.
    - Clear the spinner on every exit path; while only PRs are pending use the generic `updating` text, not a retained fetch/fallback message.
- [ ] **`--ff` and `-r`** keep their 75 s budget and local fast-forward rules; a PR failure never blocks a permitted fast-forward.
- [ ] **Comment/doc pass in code** for `list.rs`, `wait.rs`, `refresh_worker.rs`: remove claims that fresh answers skip requests, that PR requests never delay the foreground, or that only forced attempts write receipts.

Validation checkpoint 3: whole workspace compiles; `just test` and `just lint` pass in `worktree/`; `grep -rn "fetch_and_publish\|LIST_DEADLINE\|AlreadyFresh\|SkippedFresh\|origin_pr_source\|Writer" worktree/` returns nothing outside history/spec text.

## Phase 4: Binary, perf, and L2 tests

Goal: prove the behavior end to end. Tests are independent files, so authors run concurrently. Use loopback fixtures (`ProxyStub`, `FakeGitea`), injected workers, drop guards that release held requests and wait for both locks and the process (`list_prs::ReleaseOnDrop`/`finish_worker`), and the existing cache-path isolation (native Windows ignores `HOME`).

### Wave 4 (three concurrent tracks)

- [ ] **`list_prs` binary tests** (`cli/tests/list_prs.rs`, `cli/tests/perf_support`)
    - Replace `a_fresh_pr_store_makes_no_request_and_shows_its_badges`. Two sequential listings each make one logical PR query (one-page fixture, one HTTP request per query).
    - A PR answer arriving within the wait is shown by the same listing; a failed refresh shows `(couldn't refresh)`; failure with nothing stored shows `couldn't get open PRs`; a held PR request ends at the 3 s budget with the hint; concurrent listings make one PR query at a time; an empty successful answer clears cached badges and has no failure item.
    - No origin: no request or item; ignored repositories make no provider request and suppress even seeded badges; local-path and unsupported origins keep the Git head check and show no PR badges or item; an origin change during the wait never shows the previous repository's badges or credentials warning.
    - `-r` and `--ff` retain budgets; PR failure does not block a permitted fast-forward. Credentials: a 401 on the PR request in an ordinary listing shows the credentials line; head-condition precedence and ambiguous errors unchanged.
    - Update `list_flags` cases that asserted forced-only receipts or the contending-holder relaunch to the new rules.
- [ ] **Perf** (`cli/tests/perf_*`)
    - Remove `perf_list_meets_sla_when_the_pr_request_hits_its_deadline`. Add a held-PR-request test costing only the 3 s wait plus the existing local-work allowance, mirroring `perf_a_held_live_head_check_costs_the_listing_only_its_wait`. `perf_command_sla` keeps its bound with a worker whose halves answer at once.
    - Update the stale-answer and failing-refresh cases (`perf_list_meets_sla_with_a_stale_answer_and_a_failing_refresh`) that relied on a background-only PR request; local gather and render bounds unchanged; check `.config/nextest.toml` overrides still cover any test that now runs longer.
- [ ] **L2 (tmux, no window focus)** (`cli/tests/level2_list_*.rs`)
    - `level2_list_stale_pr_answer_shows_a_dim_age_line_in_tmux` asserts the age item and refresh hint after the 3 s wait.
    - New case: close the PR request with an error and assert the dim `(couldn't refresh)` item; cover PR-only waiting so the spinner clears and the completed head caption does not say "still checking."
    - Use the Test Toolkit and shared harness; run with `BISCUIT_TEST_REQUIRED_BACKENDS=tmux` to avoid silent skips.

Validation checkpoint 4: `just test`, `just test-l2`, `just test-perf`, `just lint` all pass in `worktree/`. Run `just test-l2` and `just test-perf` on this macOS host; per the `os` skill, Windows and WSL2 evidence comes from CI after merge (the pull request proves Linux and macOS), so note in the report that the Windows cache-path isolation was reviewed, not executed.

## Phase 5: Docs, measurement, and final verification

Goal: the `docs/` tree and skills describe current behavior, and the one required measurement is recorded.

### Wave 5 (concurrent doc tracks; they touch different files)

- [ ] **`worktree/docs/cli/list.md`**: rewrite "PR badges", "Status list", and "Checking origin" (one wait for both halves). Audience is a developer new to the repo: lead with what the reader can do, a compact example per rule, a Mermaid diagram for the wait/receipt flow. Never link to or name the feature directory.
- [ ] **`worktree/README.md`**: the `wt list` PR bullet and the ordinary-versus-`--refresh` wait explanation.
- [ ] **`.claude/skills/worktree/SKILL.md`**: the `pull_requests` (format 5, no `writer`, no foreground writer), `remote_head` receipt (every attempt), and `wait` paragraphs; remove the stale `fetch_and_publish`/`Writer`/forced-only claims and the old test names.
- [ ] **`worktree/docs/performance-testing.md`**: update the "PR Request" section and stale-gate notes; label the historical tables as historical.

### Wave 6 (after Wave 5's perf page edit; sequential)

- [ ] **Before/after sample**: on the development host, one quick sample of `wt list` wait time against GitHub with and without authentication, before (checkout of `main`) and after this change; record it in `docs/performance-testing.md` as an observation, not a threshold. No extra hosts or statistics (ruling 6). If GitHub is unreachable or no credentials are available, record that fact instead of a number.

### Wave 7 (final gate, sequential)

- [ ] Run `just test`, `just test-l2`, `just test-perf`, `just lint` in `worktree/` and read the output rather than the exit code.
- [ ] Drift pass: re-read every touched symbol's `///`/`//!` and inline comments; `docs/dependencies.md` unchanged (no crates added or removed; confirm).
- [ ] Set the spec's frontmatter `status` to `implemented` only if the author's process requires it of the implementer; do not move the directory to `_completed` and do not run `just complete`.
- [ ] Report: what changed, any departure from the spec (corrected in docs, recorded in the implementation log), tests not run on Windows/WSL2.
