---
total_phases: 9
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2: []
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - worktree/lib/src/pull_requests.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - worktree/lib/src/listing.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - worktree/cli/src/commands/git_graph/tests.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - worktree/cli/src/commands/list/tests.rs
  - worktree/cli/src/commands/list/tests/pipeline.rs
  - worktree/cli/tests/list_prs.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/worktree/testing.md
packages:
  - sniff
  - worktree
  - worktree-cli
created: 2026-10-03
phase: 1
agent: claude/sonnet
yolo: true
---

# Plan: `wt list` overlap and the keyless notice

Spec: `2026-10-03-list-overlap-and-keyless-notice` (`spec.md`, review `review-1.md`).
Skills to load before work: `worktree` (topics `list`, `list-remote`, `git-graph`,
`testing`), `rust-testing`, `sniff`, `os` (scoped threads, detached worker,
`#[cfg(windows)]`), `biscuit-hash` only if a digest is touched (none expected).

## Summary and definition of done

Two independent changes share one listing:

1. **Overlap.** `run_pipeline` (`cli/src/commands/list.rs`) currently runs
   `gather_remote` to completion, then `--ff`, then `reread_refs`, then the
   graph/list scope. After the change the wait stays on the calling thread while
   scoped threads gather dirtiness, comparisons, graph and verbose history from
   the initial `RefTips`. After the wait and `--ff` the refs are reread; equal
   successful snapshots accept the first gather, anything else discards the
   ref-dependent results and gathers once more. Persistent writes (comparison
   cache, fork-origin prune, copy-record prune) move to one commit step after
   acceptance. `--perf` reports the concurrent span as one nested group.
2. **Keyless notice.** `sniff` returns which credential a successful
   branch-head / open-PR request used; the worker records it (head attempt in
   remote-head format 3, PR publication in PR-store format 6); `wt list` prints
   one dim line after the caption when an observed successful API answer was
   anonymous, below existing confirmed warnings in precedence.

Done when every acceptance bullet of the spec holds:

- [ ] bounded-synchronization L1 test proves local gathers start before a held worker outcome is released
- [ ] unchanged refs: gather, status and comparison work run once; changed refs: all output describes final tips; only a `--ff`-moved checkout reruns dirtiness
- [ ] cache save, fork prune, copy-record prune run once, after acceptance
- [ ] one keyless line, correct precedence, none for cached/ignored/unsupported/changed-origin/unknown evidence
- [ ] old stores migrate per spec; malformed auth metadata is never read as anonymous; no token value is ever stored or printed
- [ ] no new network request or wait is introduced
- [ ] `just test`, `just test-l2`, `just test-perf`, `just lint` pass in `worktree/`; affected `sniff` tests and lint pass
- [ ] docs listed in the spec updated, without naming this fix; agent stops at "implementation complete, ready for review" (no `just complete`, no move to `_completed`, no commit unless asked)

## Wave schedule (overview)

Tasks in the same wave are independent and may run on concurrent subagents.
Each wave ends at a validation checkpoint before the next starts.

| Wave | Phases / tasks | Depends on |
| ---- | -------------- | ---------- |
| 1 | P1 spikes and baseline | — |
| 2 | P2 sniff metadata · P4 lib gather split · P7 perf collector groups | Wave 1 |
| 3 | P3 stores and worker · P5 snapshot-addressed graph/verbose | Wave 2 (P3←P2, P5←P4) |
| 4 | P6 overlap pipeline · P8 keyless notice | Wave 3 (P6 needs P4, P5, P7; P8 needs P3). P6 and P8 both edit `run_pipeline`/`list.rs`: land P6's reorder first, or keep P8 to `credential_line`/`RemoteAnswers`/`list_table.rs` and merge sequentially |
| 5 | P9 docs, perf sample, full verification | Wave 4 |

## Phase 1: Rulings, spikes, and baseline

### Necessary Rules

Rulings the implementer must treat as decided (each fills a gap the spec leaves
to "implementation"; the author can overturn any in review):

1. **Credential evidence type.** Three states, serialized as a tagged object, never
   `Option`: `anonymous`, `keyed { variables: [name, ...] }` (non-empty,
   each a validated env-var name: `[A-Za-z_][A-Za-z0-9_]*`, never a value), and
   `unknown`. `unknown` is the only state a migrated or absent source yields and
   it is never rendered as anonymous. A paginated PR answer folds to `anonymous`
   only if every request was anonymous; any keyed page makes it `keyed`; any
   unknown page makes it `unknown`.
2. **Where evidence lives.** Head: a new field on the `attempt` (survives
   `checking → fetching`, fetch failure, fetch timeout; a later `source: fetch`
   answer never overwrites it). PR: a new required field on the format-6
   publication, written in the same atomic write as the answer and `publication` id.
3. **Format-3 head with missing/malformed evidence on an attempt that reports API
   success:** drop the `attempt` (as format 2 does), keep the `answer`. This satisfies
   "bad attempt metadata cannot invalidate a good answer". Format-6 PR with
   missing/malformed evidence: whole publication is a miss (spec: reject malformed).
4. **Receipt format is unchanged** (no serialized shape change).
5. **Perf naming.** The group is `remote wait ‖ local gather` (local-only run:
   `local gather`). Children: `remote wait`, `list gather`, `graph gather` or
   `verbose gather`. Top-level `pr gather` keeps only the **pre-launch** portion
   (origin lookup); the **post-wait** PR reread and origin recheck run inside the
   group's wall-clock when they overlap local tasks and are shown as a non-additive
   diagnostic child `pr reread`. Existing tests that read `pr gather` are updated to
   expect the pre-launch meaning (confirm in review).
6. **Snapshot addressing.** Every history/comparison git call in the graph,
   verbose, caption and comparison paths uses captured object IDs from `RefTips`
   (never a branch name). Verbose ref labels (`%D` replacement) come from the
   accepted snapshot (spike S1 picks the mechanism).
7. **Pruning timestamp.** `refs_read_at` for fork pruning is the final accepted
   read's time; a record created after it is protected exactly as today.
8. **Spinner/terminal output.** Scoped local tasks never print; only the caller emits.
9. **Out-of-scope reminder.** No wider performance matrix; if the author wants more
   hosts or repetitions, that is a new ruling. Not scheduled here.

### Spikes (each runs once, before the work it informs)

- [x] **S1: verbose labels from a snapshot** (informs P5). Read `DETAIL_FMT` (`%h %s %at %D`) use in `git_graph.rs` and the tag/HEAD labeling tests. Decide, with a 30-line throwaway, whether to (a) drop `%D` and rebuild decorations from `RefTips` + `git for-each-ref refs/tags` / HEAD, or (b) keep `%D` but filter its refs through the snapshot. Output: a ruling line appended to Rule 6 naming the chosen mechanism and how tag/HEAD behavior is preserved.
- [x] **S2: carrying the credential selection** (informs P2). Read `sniff/lib/src/remote/focused.rs` (`FocusedProviderClient::credential`, `credential_key`, host-bound `SNIFF_*_TOKEN`) and the `blocking.rs` entry points `branch_head*`, `open_pull_requests*`. Output: the exact place where the selection is made per request and the minimal signature change (new result wrapper vs `*_with_evidence` entry points) so pagination folds per Rule 1.
- [x] No timing spike: the spec's measurements already justify overlap (recorded as a ruling; do not add one).

### Baseline

- [x] Run `just test`, `just lint` in `worktree/` and the affected `sniff` recipes; record failures that pre-exist so they are not attributed later.
- [x] Locate and list every consumer of the stage names `pr gather`, `remote wait`, `list gather`, `graph gather`, `verbose gather` (`cli/tests/perf_*.rs`, `perf_support/mod.rs`, `cache_*_path.rs`, `list/tests.rs`).

**Checkpoint 1:** rulings 1–9 and S1/S2 outputs are written at the top of this file's "Implementation log" section (add it); baseline recorded.

## Phase 2: `sniff` credential metadata

Wave 2. Package: `sniff`.

- [x] **Evidence type.** Add the Rule 1 type to `sniff/lib/src/remote` (public, `serde` where the worker needs it is decided in P3, so keep it plain data here). Variable names only.
- [x] **Selection from the client.** Make `FocusedProviderClient` report the selection it made for each request (provider variable, host-bound `SNIFF_*_TOKEN`, empty variable treated as absent, none) without a second lookup and without extra requests. Pre-reqs: S2.
- [x] **Blocking entry points.** Return the evidence beside the existing result for `branch_head*` and `open_pull_requests*` (fold per Rule 1 across pages). Preserve deadlines, error classification, pagination, request counts. Failures stay distinct and carry no success evidence.
- [x] **Callers compile.** Update every `sniff` and workspace caller of changed signatures (grep `branch_head(`, `open_pull_requests(`); prefer additive entry points if the churn is wide.
- [x] **Tests (L1, sniff).** Provider token, host-bound token, empty variable, no token; paginated PR (all anonymous / mixed / all keyed); token values absent from `Debug`/serialization of the evidence; request count unchanged.

**Checkpoint 2:** `sniff` area tests and lint pass.

## Phase 3: Stores, worker, and wait capture

Wave 3. Depends on P2. Package: `worktree` lib + CLI wait.

### Input Robustness Matrix (new load-bearing fields)

Load-bearing: **PR `credentials`** (format 6) and **head `attempt.credentials`** (format 3). Outcome per shape; one test per store walks every row from a real-writer fixture with one edit per cell, asserting through the public result (`select_cached`, `stored_publication`, `select_attempt` / wait result, and the final notice), plus a control row proving the unedited fixture yields the positive result.

| Shape | PR `credentials` (v6) | Head `attempt.credentials` (v3) |
| ----- | --------------------- | ------------------------------- |
| absent | whole publication = miss | attempt dropped, `answer` kept |
| explicit null | miss (not conflated with absent; unknown is the tagged value, not null) | attempt dropped, answer kept |
| wrong type, whole field | miss | attempt dropped, answer kept |
| wrong type, one variable element (`["A", 1]`) | miss | attempt dropped, answer kept |
| wrong type, every element (`[1]`) | miss | attempt dropped, answer kept |
| empty (`keyed` with `[]`, `{}` object) | miss (keyed needs ≥1 name) | attempt dropped, answer kept |
| unknown state tag / invalid variable name / name that looks like a value | miss | attempt dropped, answer kept |
| duplicate key (in object or in the envelope) | miss | strict reader rejects the file as today (answer rules unchanged) |
| trailing / invalid content | miss | file miss as today |
| `unknown` tag, valid | cached data usable, can never produce a notice | same |

Cross-version rows: head v1 (migrate as today) / v2 (keep `answer`, drop `attempt`) / v3; PR v5 (valid answer + publication readable, evidence `unknown`, never "this listing's request"), v6, v4 and older (miss), v7+ (miss).

Smells to grep before closing: `#[serde(default)]` on the new fields; `Option<T>` where absent vs null must differ; `filter_map(.. as_str())`, `unwrap_or_default()`, `.ok()` on the credentials parse.

### Tasks

- [x] **Serialized evidence.** Strict (duplicate/missing/type/trailing) serde for the P2 type through the existing `strict_json` reader; persist state and validated names only.
- [x] **PR store format 6.** Bump 5→6 in `pull_requests.rs`; write evidence atomically with `publication`; read v5 as `unknown`, older/newer = miss. `refresh` passes the P2 evidence from the request outcome; an empty PR list stores evidence too.
- [x] **Head store format 3.** Bump 2→3 in `remote_head.rs`; record evidence on the attempt when the API check succeeds (`remote_update::run_attempt`), retain it through `set_phase`/`finish_attempt` and across the fetch, including fetch failure and timeout; `publish_answer` (`source: fetch`) must not erase it. v1 migration unchanged; v2 drops `attempt`.
- [x] **Wait capture.** In `list/wait.rs`, return evidence from the **same new publication** the wait accepted (including a lock-holder's publication); never from an older cached answer or a later unrelated publication; no budget reset, no wait for the receipt. For adopted head work use the followed attempt's evidence. PR failure stays the listing's own receipt.
- [x] **Tests (L1).** Matrix tests above per store; publication-before-receipt; lock contention publication; adoption with a different worker environment; superseded later publication ignored; token values absent from files.

**Checkpoint 3:** `just test` in `worktree/` for lib and wait tests; `rg 'serde\(default\)'` smell check clean for the new fields.

## Phase 4: Library gather separation

Wave 2 (independent of P2/P3; may run concurrently with them). Package: `worktree` lib.

- [x] **Split `fill_worktree_statuses`** (`lib/src/worktree.rs`) into: (a) dirtiness gather, (b) ref-dependent gather (caption, target, tree, comparisons) over a given `RefTips`, returning results plus the SHA-pair cache entries, and (c) a commit step (save cache once, fork prune, copy-record prune). No persistent write in (a)/(b).
- [x] **Compose in `list_worktrees`.** Same observable behavior and subprocess counts for library callers (existing tests `list_worktrees_*` stay green).
- [x] **In-memory cache.** Valid SHA-pair entries from a discarded gather are kept and saved once; failed comparisons stay uncached; the cache is seedable into the final gather.
- [x] **Targeted dirtiness.** Expose a function that refreshes one entry's dirtiness (for the `--ff` moved checkout) and replaces that result only.
- [x] **Snapshot comparison.** Add `RefTips` equality over the full local and remote maps (additions/deletions count) and a type that records whether each read succeeded; an unsuccessful read never equals anything.
- [x] **Prune guard.** Fork pruning requires a successful final read; reload the store before pruning; keep the `refs_read_at` protection; copy-record pruning keeps live-worktree checks.
- [x] **Docs on touched symbols** updated in the same change (comment drift rule).
- [x] **Tests (L1).** Persistence happens only in the commit step; speculative cache entries survive; concurrent `wt create` record survives; failed read never prunes; read-failure never equal.

**Checkpoint 4:** `just test` in `worktree/` (lib) with unchanged warm/cold subprocess-count tests.

## Phase 5: Snapshot-addressed graph and verbose gathering

Wave 3. Depends on P4 and spike S1. Package: `worktree-cli` (`git_graph.rs`).

- [x] **Take tips as input.** `GatherInput` is built from the initial or final `RefTips` plus entries, not from a filled `WorktreeList`, so it can be built before the caption/tree exist (derive what it needs from `build_tree`, which is pure).
- [x] **Object IDs only.** Replace branch names with captured SHAs in every `git log`/`rev-list`/`merge-base`/`merge-tree` call the graph and verbose gatherers issue.
- [x] **Verbose labels** from the accepted snapshot per S1, preserving tag and HEAD labeling.
- [x] **Preserve** graph `incomplete` behavior and existing conditions for the graph (image-capable path) and verbose rows.
- [x] **Tests (L1).** Existing git-graph tests pass; new tests: a ref moved after capture does not change gathered output; `%D`-equivalent labels match the snapshot, with tag/HEAD cases.

**Checkpoint 5:** `git_graph` and `perf_graph_stages` tests pass.

## Phase 6: Overlapped pipeline and acceptance

Wave 4. Depends on P4, P5, P7 (group recording) and, for `RemoteAnswers`, nothing from P3.

- [x] **Reorder `run_pipeline`.** After `parse_worktree_state`: record `--ignore-api` and prepare the remote request (preference-write error still exits with no listing or launch). Then in `std::thread::scope`: scoped threads run dirtiness (parallel per worktree), the ref-dependent gather, and graph/verbose gather from the initial snapshot; the calling thread runs `gather_remote`/the wait. Keep list∥graph and per-worktree parallelism. The monotonic budget still starts before the first launch.
- [x] **Join, then `--ff`.** Local tasks are joined before `fast_forward_default`; the detached worker is never joined or killed. A timeout limits waiting, not local computation.
- [x] **Accept or regather.** Reread refs when `remote.waited.is_some()` or `--ff` ran (as today); equal successful snapshots accept; changed or failed read → one regather of caption, target, tree, comparisons, graph facts, verbose from the final snapshot (no retry loop); failed final read keeps degraded behavior and skips pruning. A fetch observed by the final read invalidates the first gather even after a timeout.
- [x] **Dirtiness reuse.** Keep initial results; if `FfResult` is a successful move with a checkout path, refresh only that checkout; other outcomes need no status walk.
- [x] **Commit once.** Save cache, prune forks and copy records once on the accepted results; then render once (caption, table, graph, verbose) with the PR answer applied at render time.
- [x] **Exit paths.** Scoped tasks joined and the spinner cleared on every exit (local gather error included); no output from scoped tasks; no-origin, ignored-API, captured-output and non-image paths unchanged; origin-changed handling unchanged.
- [x] **Test seam.** Extend `tests::overlap` (list-versus-graph only today) so a held worker outcome releases only after both local gathers arrived; bounded waits, no sleeps, released on failure, no leaked processes.
- [x] **Tests (L1)** per spec: overlap; unchanged refs run status/comparison once; remote advance/rewind/add/delete reflect final tips in caption/target/tree/counts/graph/verbose; `--ff` without holder / up-to-date / refused / moved-with-holder reruns only that checkout; failed initial or final read; local gather error cleanup; persistence-once; timeout (budget not exceeded by the wait, first gather reused when tips match, regather when a fetch was observed, pending status and hint kept, worker not joined, long local gather allowed to finish).
- [x] **Existing suites.** `list/tests.rs`, `list_prs.rs`, `cache_*_path.rs`, `remote_fixture` L1/L2 stay green; update only what the reorder changes.

**Checkpoint 6:** `just test` and `just test-l2 list` green in `worktree/`.

## Phase 7: `--perf` nested groups

Wave 2 (independent; may run concurrently with P2/P4). Package: `worktree-cli` (`perf.rs`).

- [ ] **Group API.** `PerfCollector` records a named group with measured elapsed (not a child sum) and ordered children without percentages; `recorded_stages` keeps working for flat stages and flattens group children by name so existing readers resolve `list gather` etc.
- [ ] **Reconciliation.** Top-level children (including `unattributed`) sum to total; overlapping children are excluded from the top-level sum; no saturating subtraction hiding overlap.
- [ ] **Rendering.** Biscuit-terminal metrics tree as in the spec example; `remote wait ‖ local gather`, local-only `local gather` without `remote wait`; `regather` group only when needed; `fast-forward` and the affected-checkout status refresh as separate top-level rows; conditional `graph gather`/`verbose gather`.
- [ ] **Callers.** Wire the group into `run_pipeline` in P6 (this phase only provides and tests the API); update `perf_support/mod.rs` and stage parsers for nested rows.
- [ ] **Tests (L1).** `perf.rs` unit tests (not snapshot in the double-compiled module; use an integration test for snapshots): reconciliation with nested overlap, conditional rows, local-only, regather only when needed.

**Checkpoint 7:** `perf.rs` tests and updated `perf_*` integration tests pass.

## Phase 8: Keyless notice

Wave 4. Depends on P3 (evidence in `WaitEnd`). Package: `worktree-cli` (`list.rs`, `list_table.rs`).

- [ ] **Evidence in `RemoteAnswers`.** Surface head evidence (from `followed_attempt`) and PR evidence (from the accepted publication) with "observed in this listing" vs "cached" distinguished; unknown, cached-only, and post-selection arrivals are not observed-anonymous.
- [ ] **Choose one line.** Precedence: confirmed head warning, confirmed PR warning, keyless success from either half. A keyed success never hides an anonymous success in the other half; a generic failure in one half does not hide anonymous success in the other; a confirmed warning always outranks.
- [ ] **Suppression.** None for ignored, unsupported, local-path, changed origin (including successful notices for the old origin), cached-only, unknown. No notice for an anonymous API failure alone.
- [ ] **Text.** Provider name and variable names from `sniff::credential_env`; "higher rate limits" wording only for providers where the advantage is established, otherwise "set {variables} to authenticate API requests"; names only, no values or raw URLs. Rendered through the existing credentials-line renderer, dim, immediately below the caption.
- [ ] **Fallback coexistence.** The closing `fallback_notice` is unchanged and may appear with the new line.
- [ ] **Tests (L1).** Head-only, PR-only, empty PR answer, mixed keyed/anonymous, success during fetch, adoption with different environments, publication before receipt, PR lock contention; warning precedence; cached/unknown/ignored/changed-origin suppression; coexistence with fallback; snapshot of text per provider kind in `cli/tests/list_table.rs` (integration test, per double-compile rule).
- [ ] **Test (L2).** Extend the existing credentials-line terminal test (dim, below caption, after spinner cleanup) on a windowless backend; no new visible window, no focus.

**Checkpoint 8:** `just test` and `just test-l2` credential tests green.

## Phase 9: Docs, sample, and full verification

Wave 5. Depends on all phases.

- [ ] **Docs** (current behavior only; never name this fix or another dated spec): `worktree/README.md`, `worktree/docs/cli/list.md`, `worktree/docs/performance-testing.md` (append the new sample; keep dated history), `worktree/docs/git-graph.md`, `.claude/skills/worktree/list.md`, `list-remote.md` (formats 3 and 6, metadata), `testing.md`; relevant `sniff` remote docs and skill topic page. Use Mermaid for the pipeline and accept/regather flow; target reader is a developer new to the repo. Fix the stale "run after the wait" statement in `list.md`.
- [ ] **Comment drift pass** over every touched symbol; report any drift found and how it was resolved.
- [ ] **Quick sample.** One release build on the dev host, same listing shape when available (9 worktrees, authenticated); record overlap and any regather cost in `performance-testing.md`. No extra hosts, repetitions, or thresholds.
- [ ] **Full verification.** In `worktree/`: `just test`, `just test-l2`, `just test-perf`, `just lint`. In `sniff/`: its test and lint recipes. Confirm no leftover background processes from tests, and OS notes (macOS, Linux, native Windows, WSL2) for scoped-thread and detached-process behavior; use the `os` skill for any `cfg(windows)` or path-comparison edits.
- [ ] **Smell greps** from the matrix section clean; confirm no token appears in any serialized file or rendered output.
- [ ] **Implementation log** in the fix directory records every departure from the spec and any ruling the author should revisit. Leave `spec.md` as decided. Stop at "implementation complete, ready for review".

**Checkpoint 9 (exit):** all Definition-of-done boxes above are checked.

## Implementation log

(Filled during implementation: ruling confirmations, spike outcomes, departures.)

### Checkpoint 1 (Phase 1)

Context: review cycle 1 (`implementation-log.md`, "Implementation of Review
Findings #1") already built phases 2–8 and most of phase 9 against this plan,
and ran both spikes as part of it. Phase 1 therefore records the rulings as they
were actually applied, consolidates the spike outcomes, and takes a fresh
baseline of the tree as it now stands. Checkboxes in phases 2–9 are left for
their own phases to verify and tick.

**Rulings 1–9, as applied:**

1. Credential evidence: confirmed with one tightening. Three tagged states
   (`anonymous`, `keyed { variables }`, `unknown`), never `Option`; in `sniff`
   it is `RequestCredentials`, in `worktree` `remote_head::CredentialEvidence`
   (`{"state": …}`). Stored names must match `[A-Z_][A-Z0-9_]*` (upper case),
   stricter than the rule's `[A-Za-z_][A-Za-z0-9_]*`, because a mixed-case
   GitHub token (`ghp_…`) matches the looser pattern. The pagination fold is as
   ruled, and a fold over zero requests is `unknown`.
2. Where evidence lives: confirmed. Head `Attempt::credentials` is set when the
   API check succeeds, before the answer is published, and survives
   fetching, fetch failure, timeout, and a `source: fetch` answer. PR
   `credentials` is written in the same atomic write as the answer and
   publication id.
3. Bad v3 head evidence drops the attempt and keeps the answer; bad v6 PR
   evidence makes the publication a miss. Confirmed.
4. Receipt format unchanged. Confirmed.
5. Perf naming: confirmed, with one note. The post-wait PR reread always runs
   inside the group's span, so it is always shown as the child `pr reread`.
   The regather group is `regather`, with children `list regather` and
   `graph regather` / `verbose regather`. `pr gather` is the pre-launch
   portion only.
6. Snapshot addressing: confirmed. The graph, comparisons, and target ancestry
   were already SHA-addressed; verbose `%D` was the only live-ref read.
   **S1 ruling (appended):** read `%H` plus `%D` filtered with
   `--decorate-refs-exclude=HEAD|refs/heads/|refs/remotes/`, so only tags and
   other non-branch decorations come from Git. Then rebuild `HEAD -> <current>`,
   local branches, remote-tracking names, and symbolic `origin/HEAD` from the
   snapshot's `RefTips` (which now records `remote_heads` via `%(symref)`), in
   Git's reverse-refname order. An include pattern (`--decorate-refs=`) was
   rejected because it drops tags. When nothing moves, the output matches Git's
   own `%D`.
7. `refs_read_at` for fork pruning is the final accepted read's time. Confirmed
   (`RefSnapshot` carries the time; `WorktreeList::commit` reloads the store
   and protects newer records).
8. Scoped local tasks never print. Confirmed.
9. Out-of-scope reminder. Confirmed; nothing scheduled.

**S2 ruling (carrying the credential selection):** `FocusedProviderClient::fetch_json`
selects the credential per request via `credential()`: provider variables for a
URL-built client, the host-bound `SNIFF_{PROVIDER}_{HOST}_TOKEN` only for a
discovered `ProviderAndHost` client. The minimal change is an optional per-client
`SentLog` (`with_sent_log`) appended from that same `credential()` call. It is
installed and folded by `blocking::run_with_deadline`. The only changed
signatures are `BranchHead` (gains `credentials`) and `open_pull_requests{,_with}`
(returns `OpenPullRequests { pull_requests, credentials }`). This was chosen
over `*_with_evidence` entry points because the only workspace callers are
`worktree` and sniff's own tests. It adds no request and changes no deadline,
pagination, or error classification.

**No timing spike**, per the plan.

**Stage-name consumers** (`pr gather`, `pr reread`, `remote wait`,
`list gather`, `graph gather`, `verbose gather`, and the group names
`remote wait ‖ local gather` / `local gather`):

- producers: `cli/src/commands/list.rs` (records them), `cli/src/perf.rs` (collector, groups, and its unit tests)
- unit tests: `cli/src/commands/list/tests.rs`, `cli/src/commands/list/tests/pipeline.rs`
- stage parser: `cli/tests/perf_support/mod.rs` (`stage_from_perf`, `list_gather_from_perf`; whole-label match at any depth)
- integration readers: `cli/tests/perf_flag.rs`, `cli/tests/perf_pr_request.rs`, `cli/tests/perf_graph_stages.rs`, `cli/tests/cache_cold_path.rs` and `cli/tests/cache_warm_path.rs` (through `perf_support`'s `list_gather_duration`)
- the plan's `list_prs.rs` names no stage now; its comment naming `pr gather` was corrected in review cycle 1

**Baseline:** see `implementation-log.md`, `## Phase 1`.
