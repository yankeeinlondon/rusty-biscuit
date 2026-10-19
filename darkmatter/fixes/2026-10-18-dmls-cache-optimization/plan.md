---
total_phases: 6
created: 2026-10-18
phase: 1
agent: claude/sonnet
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
    - dmls
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/payload_memo_tests.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - darkmatter/dmls/src/registry_jobs.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/l1/registry_preparation.rs
    - darkmatter/dmls/tests/l1/repository_contexts.rs
    - darkmatter/dmls/tests/l1/workspace_schema_scopes.rs
docs_updated_during_phase_3:
    - darkmatter/docs/lsp/indexing.md
    - darkmatter/docs/lsp/configuration.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_4:
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/registry_jobs.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/work_counters.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/startup_preparation.rs
    - darkmatter/dmls/tests/l1/workspace_file_list.rs
docs_updated_during_phase_4:
    - darkmatter/docs/lsp/indexing.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_5:
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/request_timing.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/registry_observability.rs
docs_updated_during_phase_5:
    - darkmatter/docs/lsp/indexing.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_6: []
docs_updated_during_phase_6:
    - darkmatter/docs/lsp/indexing.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/darkmatter/dmls.md
source_code:
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/payload_memo_tests.rs
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/registry_jobs.rs
    - darkmatter/dmls/src/request_timing.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/work_counters.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/registry_observability.rs
    - darkmatter/dmls/tests/l1/registry_preparation.rs
    - darkmatter/dmls/tests/l1/repository_contexts.rs
    - darkmatter/dmls/tests/l1/startup_preparation.rs
    - darkmatter/dmls/tests/l1/workspace_file_list.rs
    - darkmatter/dmls/tests/l1/workspace_schema_scopes.rs
documentation:
    - darkmatter/docs/lsp/indexing.md
    - darkmatter/docs/lsp/configuration.md
    - .claude/skills/darkmatter/dmls.md
completed_phase: 6
implemented: true
---

# Plan: DMLS Trigger-Registry Cache Optimization

## Summary and Definition of Done

Opening a document right after DMLS starts stalls every schema-dependent
request for over a second, because workspace trigger registries are
re-prepared (a routine `didChangeConfiguration` invalidates all of them) and
the trigger-payload memo misses on nearly every lookup. The work is four
fixes plus visibility, spread over two crates:

| Req | Crate | Where | Change |
| --- | --- | --- | --- |
| R1 | `darkmatter` lib | `lib/src/markdown/schemas/triggers/assemble.rs` (`file_payload_memo`) | Key by what resolution reads, not the requester's folder; capacity 32; true LRU |
| R2 | `dmls` | `router.rs` (`update_config`) | Compare effective config first; unchanged invalidates nothing and queues nothing |
| R3 | `dmls` | `registry_jobs.rs`, `router.rs` (`spawn_registry_worker`) | Document jobs first; workspace jobs merge; running pass yields between probes |
| R4 | `dmls` | `router.rs` (`spawn_startup`), `overlay/mod.rs` | Registry worker owns all preparation; discovery runs concurrently; file list published first |
| R5 | both | `request_timing.rs`, worker | Pass-local debug line; `trigger_registry_wait_ms` on the slow-request event |

**Done when** (spec acceptance criteria 1–6 all hold):

- [x] An unchanged config notification adds no workspace pass and no blanket invalidation
- [x] A cold scan of the three-trigger input set records 3 payload misses; a second forced scan records 0
- [x] Multiple workspace schedules merge; schedules during a running pass yield exactly one follow-up (including the worker-completion/dispatch boundary)
- [x] Document completion releases waiters before the remaining probes
- [x] No two server preparations run concurrently; relevant changes during preparation leave the scope stale
- [x] Resolved registries, diagnostics, and trigger summaries are identical on `darkmatter/tests/fixtures/schema-triggers/`
- [x] `just test` and `just lint` pass in `darkmatter/`; docs updated; before/after repro sample recorded in the implementation log

Input Robustness Matrix: **not applicable.** No parser, loader, or
deserializer is added or changed; the memo key and job scheduling are internal
state.

Conventions for every phase: US English; do not run `cargo fmt`; do not
commit; use nextest via `just test`; use the Test Toolkit; hermetic L1 tests
only (no terminal or browser focus); update `///`/`//!` and inline comments
for every behavior change (the old memo/`update_config` docs describe the
behavior being removed).

---

## Phase 1: Rulings, Baseline, and Test Seams

Goal: settle the open design points, record the "before" sample, and add the
seams later phases test against.

### Necessary Rules

The spec leaves these to the implementer; the plan rules as follows.
Departures found during implementation go in the implementation log, not the
spec.

1. **Memo key shape (R1).** Key = `(source, payload, schema_roots, K)` where
   `K` is the ordered list of full `FileResolutionContext`s actually used by
   resolution: the context derived with `for_cwd(source.parent())`, then one
   `for_trusted_external_cwd(root)` per schema root derived **directly from the
   original** context. Compare contexts by full equality. Execution still uses
   the original context. If any branch (imports, delegated schemas, examples,
   inline payloads) reads original state not captured in `K`, add the original
   context (or that state) to the key for that branch. When in doubt keep the
   field: a miss costs time, a false hit serves a wrong schema.
2. **Hit/miss seam for tests and R5.** Counters are **scoped to the calling
   thread** (a thread-local collector that a caller installs for the span of a
   pass), not process-global deltas, so a document job that interrupts a
   workspace pass on the same worker thread is excluded by swapping the
   collector for the duration of that job. The library exposes the minimum
   needed surface for `dmls` and tests; no other new public context API.
   A hit is fingerprint-validated reuse; a miss is an attempted uncached
   resolution; fresh scopes record neither.
3. **Capacity and eviction.** 32 entries, bounded and fixed; hits refresh
   recency; the least recently used entry is evicted at the bound.
4. **Worker queue model (R3).** Replace the unbounded FIFO with two lanes
   (document queue, a workspace slot) behind one worker thread. Workspace
   state is a small state machine: `Idle → Queued → Running → (Running +
   RequestedAgain) → Queued`. `Queued` is cleared atomically by the worker
   when the pass starts. All schedules during a pass collapse into one
   follow-up using the latest roots/contexts/repository set (read when the
   follow-up starts, not when scheduled).
5. **Suspended pass accounting.** `outstanding` counts one workspace job
   (queued, running, or suspended) plus each queued document job, never
   probes or merged schedule calls. One completion message per executed job.
6. **Startup moves onto the worker (R4).** Startup submits two phase jobs
   (initial scopes; newly discovered scopes) to the same worker. Discovery
   stays on its own thread. The second phase is submitted by dispatch only
   after it has seen **both** the discovery result and the initial-phase
   completion. The `schema registry ready` summary and observer completion
   follow the final phase.
7. **Yield granularity.** A workspace pass checks for waiting document jobs
   before the first probe (including probe-list construction, which moves
   off the dispatch thread) and between probes. No interruption inside a
   payload resolution or filesystem call.
8. **Stale document job.** A queued document job reads the current context
   when it executes; if the captured context is outdated it reschedules
   itself before releasing waiters.
9. **Wider measurement (for the author).** The spec limits performance
   evidence to one before/after sample on one quiet host. A multi-host or
   repeated measurement is **not** scheduled; listing it here only so the
   author can request it.

### Spikes

None. The spec's profile and miss counts already answer where the cost is
(see ruling 9); the context-key equivalence checks are required correctness
work in Phase 2, not a spike.

### Tasks

**Wave 1** (parallel):

- [x] **Before sample**
    - Build current `dmls`; run the stdio repro from the spec (`initialize`,
      `initialized`, `didOpen` of a darkmatter document, then
      `semanticTokens/full`), once with no `didChangeConfiguration` and once
      with `{"settings": {}}`, on this host.
    - Record both timings and `waited_for_trigger_registry` in the
      implementation log. Use a throwaway harness; do not commit it unless
      it is the hermetic stdio test from Phase 4.
- [x] **Read the memo's inputs**
    - Read `FileResolutionContext::derive`, `for_cwd`,
      `for_trusted_external_cwd`, and every context use in
      `resolve_yaml_schema_with_roots` / `resolve_file_payload`.
    - Write the list of request-side fields that survive derivation and
      the resolution branches that read the original context. Record in the
      implementation log; this fixes `K` for Phase 2.
- [x] **Read the worker plumbing**
    - Read `registry_jobs.rs`, `spawn_registry_worker`, `spawn_startup`,
      `Internal::{RegistryPrepared,RegistryReady,FileList}`,
      `TriggerRegistries::{begin,is_fresh}`, and the existing `StartupEvent`
      observer and gate usage in `dmls/tests/l1/{registry_preparation,startup_log_lines}.rs`.
    - List every test that counts duplicate schedules or asserts
      file-list-after-registry-ready order; these are updated in Phases 3–4.

**Checkpoint 1:** the key field set, test-update list, and baseline numbers are
recorded in the implementation log. `just test` is green before any change.

---

## Phase 2: Payload Memo (R1)

Library only (`darkmatter/lib`). Independent of the `dmls` phases except for
the counter seam that Phase 5 consumes.

**Wave 2** (single file; sequential within):

- [x] **New key**
    - Replace `folder_context` keying with the `K` from ruling 1. Build `K`
      per lookup (cheap derivations) and compare by full equality.
    - Remove the old doc comment's "request context" wording; document the
      new key and why the requester's folder is excluded.
- [x] **Equivalence checks**
    - For relative payload files, inline payloads/imports/examples, a
      delegated schema, and a trusted external root: confirm a hit returns
      what a fresh resolution would. Where direct derivation from the original
      differs from derivation from a normalized context, keep the original
      state in the key (ruling 1). Never clear a validation failure or widen
      a trust boundary to get a hit.
- [x] **LRU at 32**
    - Capacity constant 32; a hit moves the entry to most recent; eviction
      removes the least recently used. Keep the existing fingerprint check
      (dependency bytes and searched folder listings) and "only successes
      kept" rule unchanged.
- [x] **Counter seam**
    - Add the thread-scoped hit/miss collector (ruling 2) and record at the
      exact points: hit after a passing fingerprint, miss on each attempted
      uncached resolution.

**Wave 3** (tests; parallel, `feature-tester-rust`):

- [x] Two documents in different package folders of one repo, same schema roots: the second resolution is a hit and returns the same schema `Arc`
- [x] Incompatibility matrix, one edit per cell from a real fixture, with a control row proving the unedited fixture hits: different repository/request boundary, launch `@` scope, home, environment, explicit tree control, inherited failure. Cover a trusted external root and inline imports/examples as well as relative payload files
- [x] 18 genuinely distinct full keys then a forced repeat scan: 0 misses. LRU: a hit protects an entry; the least recently used is evicted at 32
- [x] A changed dependency file, and a file added to a schema root, still miss (fingerprint path)
- [x] Existing `schema-triggers` fixtures: registries, diagnostics, and trigger summaries unchanged (criterion 6)

**Checkpoint 2:** `just test` and `just lint` in `darkmatter/`; code smell
grep: no new `#[serde(default)]`, `unwrap_or_default()`, or `.ok()` on a
load-bearing parse in the touched code.

---

## Phase 3: Config Reload and Job Queue (R2, R3)

Both touch `dmls/src/router.rs`; R3's queue core lives in `registry_jobs.rs`
and the worker. Run Wave 4 in parallel with Phase 2.

**Wave 4** (parallel; separate files until the worker integration):

- [x] **Unchanged reload (R2)**
    - In `update_config`, snapshot `before`, apply `update`, compare effective
      config **first**. Unchanged: no `invalidate_all_trigger_registries`,
      no `schedule_workspace`; keep `contexts.clear()`, and the existing
      `relink` + `refresh_all_diagnostics` when contexts were dropped.
    - Changed: keep blanket invalidation and queue one merged workspace pass
      **after** the new config applies. Keep graph relinking, config-error
      diagnostics, semantic-token refresh decisions, and open-document
      diagnostics updates.
    - Verify `is_fresh`'s `scope.context == context.for_source(path)` makes a
      differently rebuilt context re-prepare on demand.
    - Rewrite the `reload_config`/`update_config` doc comments ("even when the
      effective config is unchanged" is now false).
- [x] **Queue core (R3)**
    - In `registry_jobs.rs` implement the two-lane queue and the workspace
      state machine from rulings 4–5: a document job always dequeues before a
      workspace job; `schedule_workspace` is a no-op while queued; a schedule
      during a running pass records `RequestedAgain` and yields exactly one
      follow-up; queued state is cleared by the worker atomically at pass
      start (shared state under one lock, not by the dispatch-side handler).
    - Fix `outstanding` accounting (ruling 5); one completion per executed
      job.

**Wave 5** (sequential, after Wave 4):

- [x] **Worker integration**
    - Rewrite `spawn_registry_worker` on the new queue. Move probe-list
      construction into the worker. Between probes (and before the first),
      run any waiting document job, post its completion immediately, then
      continue the suspended pass. Stale-context document jobs reschedule
      (ruling 8).
    - Update tests that counted duplicate schedules (list from Phase 1).

**Wave 6** (tests; parallel):

- [x] R2: unchanged `didChangeConfiguration` leaves every registry fresh and queues no job; an effective change (for example workspace discovery settings) invalidates; unrelated file/config diagnostics and refresh behavior intact
- [x] R2: a context that rebuilds differently after a reload re-prepares on next need without blanket invalidation
- [x] R3 (worker gated): Workspace, Workspace, Document queued → document first, exactly one workspace pass
- [x] R3: a document job queued during a gated pass runs after the current probe and before the next, and its completion is posted before the pass resumes
- [x] R3: a workspace schedule during a running pass yields exactly one follow-up
- [x] R3: schedule after worker completion but before dispatch handles it → exactly one new pass
- [x] Gates have bounded cleanup (timeouts) so a regression fails instead of hanging

**Checkpoint 3:** `just test` in `darkmatter/` green for `dmls`; existing
change-replay and failure-transition tests pass unchanged (criterion 5).

---

## Phase 4: Startup on the Worker (R4)

Depends on Phase 3. Single owner of `spawn_startup` / dispatch handling.

**Wave 7** (sequential):

- [x] **Phase jobs**
    - Add startup phase jobs to the worker (initial scopes; newly discovered
      scopes), preparing without holding the overlay lock across I/O, and
      preserving the existing change/epoch replay.
- [x] **Discovery concurrency**
    - `spawn_startup` now only runs discovery (starts at once, concurrent
      with the initial phase) and sends the result; it never calls
      preparation. Dispatch installs and publishes the file list as soon as
      discovery returns, with no wait on any registry phase.
    - Dispatch submits the second phase only after both discovery result and
      initial-phase completion, using completion messages, never blocking.
    - Send the single `schema registry ready` summary and observer completion
      after the final phase; elapsed excludes the file walk and queue waits;
      the summary counts every startup scope. Account `outstanding` so
      observers cannot see startup complete between discovery and the final
      phase.
- [x] **Lifecycle**
    - Cancellation removes only that request's waiter (shared jobs remain for
      other waiters). Shutdown answers all waiters once, stops taking jobs,
      and releases startup coordination without waiting for a completion
      that cannot arrive.
    - Update docs/tests asserting file-list-after-registry-ready order.

**Wave 8** (tests; parallel):

- [x] Startup and refresh scheduled together: preparations never overlap (instrumented overlap detector); fresh probes skipped; discovered repositories prepared before startup completion
- [x] A change recorded during the single in-flight preparation leaves the scope stale
- [x] Concurrent discovery: with the initial phase gated, the file list is installed and published before the gate releases; the second phase starts only after both; the ready summary follows and counts every startup scope
- [x] Lifecycle: waiters share a document job; cancel one, answer the other once; shutdown during either startup phase releases waiters and coordination; a context change during queued work cannot strand waiters
- [x] Hermetic stdio session on `LspFixture` (startup request ordering): gate an unrelated workspace probe, send unchanged configuration, open a document, request semantic tokens; release the current probe and observe the response before the next probe completes (a cold document may report `waited_for_trigger_registry=true`)
- [x] Criterion 3: unchanged notification causes zero extra preparations vs. the same startup without it

**Checkpoint 4:** all of criteria 1, 3, 4, 5 hold; `just lint` clean.

---

## Phase 5: Observability (R5)

Depends on Phases 2 and 4 (counter seam and worker loop).

**Wave 9** (parallel, different files):

- [x] **Pass log**
    - At `debug`, one line per workspace pass: `trigger registries prepared probes=… prepared=… fresh=… payload_hits=… payload_misses=… elapsed_ms=… workspace=…`.
      Install the thread-scoped collector (ruling 2) for the pass and swap it
      out for interrupting document jobs. Count failed probes separately from
      fresh and prepared. Preserve existing info-level startup summaries and
      failure/recovery transitions. Never log payload contents, environment
      values, or schema text.
- [x] **Wait duration**
    - In `request_timing.rs` add `trigger_registry_wait_ms` to the existing
      `slow request` completion event (no second event), using the injected
      server clock. Sum separate parked intervals, each ending at resume,
      cancellation, or shutdown; exclude handler work and index waits. Emit
      for canceled requests too.

**Wave 10** (tests; parallel):

- [x] Captured `tracing`: pass-local counts exclude interrupting document jobs; failed probes counted separately; a fresh scope records neither hit nor miss
- [x] Injected clock: summed waits with resume/requeue, cancellation, and shutdown, all on one completion event
- [x] Log-hygiene check: no payload/env/schema text in the new lines

**Checkpoint 5:** `just test` green.

---

## Phase 6: Docs, Lint, and Close-out

**Wave 11** (parallel docs):

- [x] `darkmatter/docs/lsp/indexing.md`: job ordering, workspace-job merging, when a config reload invalidates, memo key, file list published before registry-ready; include a Mermaid sequence "request parked on a document job while a workspace pass runs". Audience is a developer new to the repo; lead with what the reader can do, no links to the feature/fix
- [x] `darkmatter/docs/lsp/configuration.md`: unchanged effective reload (with unchanged context inputs) does not invalidate; every effective change keeps conservative invalidation
- [x] `.claude/skills/darkmatter/dmls.md`: worker priority rule and memo key
- [x] Doc comments: `file_payload_memo`, `update_config`, `spawn_startup`, `spawn_registry_worker`, registry-jobs module; delete drifted comments and record the drift in the implementation log
- [x] README only if public behavior changed (expected: no)

**Wave 12** (sequential):

- [x] Find consumers of any changed public lib surface (Claudine) with Sniff/GitNexus; fix breaks
- [x] `just lint` and `just test` in `darkmatter/`; no workspace-wide gates
- [x] After sample: rerun the stdio repro from Phase 1 on the same host; record before/after, the remaining wait and its cause, against the non-gating ≤ 250 ms target
- [x] Write the implementation log: key field set, any departures from the spec, doc/test updates, drift resolved
- [x] Set spec status to `implemented` per repo convention. Do not move the fix to `_completed` and do not run `just complete`; the terminal state is "implementation complete, ready for review"

**Checkpoint 6 (final):** every Done-when item above checked, with test names
recorded in the implementation log.
