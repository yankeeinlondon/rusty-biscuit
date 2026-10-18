---
feature: 2026-10-17-dmls-index-store
implementation_1: "2026-10-18T06:36:59-07:00"
implementation_2: "2026-10-18T06:46:14-07:00"
---

# Review-to-Implement Log: DMLS Index Store

## Implementation of Review Findings #1

> **started at:** 2026-10-18T06:36:59-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-dmls-indexing/darkmatter/features/2026-10-17-dmls-index-store/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- set the review's `implemented` flag to `false` before starting (it was already `false`)
- inventory of `review-1.md`:
        - frontmatter `findings: []` and `observations: []`; `recurrence: false`; `has_blocked_findings: false`; `blocked: false`
        - the body states "No code findings have been assigned"; it is a human-review-branch review, and no implementation-versus-spec review or test run was performed
        - the one item is a `human_in_the_loop` decision: whether per-request store queries and temporary graph construction may run on the dispatch thread
        - "Deferred follow-ups" (quiet-host benchmark rerun, cold-disk and other-OS samples, the user-global `storage-strategy` skill edit) are stated by the reviewer to be optional author follow-ups, explicitly not findings
        - the "Author decision (2026-10-18)" section accepts the current design and states that `docs/lsp/indexing.md` now documents it

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-H1   | Human decision: where per-request store queries and graph construction run | already fixed | Author decision (2026-10-18) in `review-1.md` accepts the dispatch-thread design; no code change is called for |
| R1-H1.1 | `docs/lsp/indexing.md` "Answering a request from the store" states that these requests answer on the dispatch thread | already fixed | Uncommitted author edit present (`git diff darkmatter/docs/lsp/indexing.md`, +9 lines); its claims checked against code: `Router::needs_index_build` (`dmls/src/router.rs:1892`), `request_graph` / `symbol_documents` (`dmls/src/router/facts.rs:205,229`); link target `## The index build` exists (`docs/lsp/indexing.md:510`); quoted timings (~45 ms references, ~50 ms workspace symbols) match run 2 in `implementation-log.md:857` (43 ms / 51 ms) |
| R1-O1   | Deferred follow-ups: quiet-host and cold-disk `--bench-index` reruns, extra OS samples, user-global `storage-strategy` skill entry | deferred (nonblocking) | Reviewer states these are optional author follow-ups and not findings; the skill edit is outside this repository |

- starting the work on 'R1-H1 dispatch-thread decision' at 06:38:10
        - defect class: none; this is a design decision, not a defect, and the review assigns no code finding
        - no sibling sweep applies: the decision is accepted as built, so no request handler changes
        - verified the author's documentation edit against the code it describes (see ledger row R1-H1.1)
        - checked that no Rust test reads `docs/lsp/indexing.md` (`grep -rln 'indexing\.md' --include='*.rs' darkmatter/` returns nothing), so the doc edit schedules no test
- work completed for 'R1-H1 dispatch-thread decision' at 06:39:30

### Completion Report

- **outcome: complete**; elapsed about 4 minutes (06:36:59 to about 06:41)
- parent findings: total 0 code findings in `review-1.md` (`findings: []`); 0 fixed + 0 already fixed + 0 blocked + 0 deferred + 0 unresolved + 0 disputed
- decision items: 1 (R1-H1), **already fixed** by the author's accepted decision and documentation edit; its one child (R1-H1.1) verified against the code
- nonblocking follow-ups: R1-O1 left as **deferred** to the author, as the reviewer stated; these are not findings
- open finding IDs: none
- changed files in this cycle:
        - `darkmatter/features/2026-10-17-dmls-index-store/log.md` (this log)
        - `darkmatter/features/2026-10-17-dmls-index-store/review-1.md` (frontmatter `log`, `implemented`, `implemented_by` only)
        - no source, test, or doc file changed; the `docs/lsp/indexing.md` edit was the author's and was already in the tree
- verification:
        - `just lint` in `darkmatter/`: exit 0 (all packages, plus the `zed-dmls` wasm32-wasip2 check)
        - `just test` in `darkmatter/` not rerun: no Rust source or test-read input changed since the Phase 7 close, which recorded 10454 passed, 0 failed (`implementation-log.md:915`); no Rust test reads `docs/lsp/indexing.md`
        - cross-OS check not run: no code change, so no OS risk
- the next review can resume the implementation-versus-spec review that `review-1.md` deferred pending this decision

## Implementation of Review Findings #2

> **started at:** 2026-10-18T06:46:14-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-dmls-indexing/darkmatter/features/2026-10-17-dmls-index-store/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- the review's `implemented` flag was already `false`; left `false` before repairs
- inventory of `review-2.md`:
        - frontmatter findings: R2-F1 (high), R2-F2 (high); `recurrence: false`; `blocked: false`
        - body: R2-F1 with children R2-F1.1–R2-F1.10 (three marked clean by the reviewer), R2-F2 with children R2-F2.1–R2-F2.9 (two marked clean)
        - observation (not a finding): the root `just test dmls` recipe fails before nextest starts; outside this spec's code, not repaired here
        - no blocked findings; no recurrence

### Finding Ledger

| ID       | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| -------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R2-F1    | Shared rows can change after validation and silently change answers | fixed | every child below fixed or clean; mechanism `WorkspaceFacts::verified_view` + `FactsSource::view` read transaction; mutation check fails 4 new tests |
| R2-F1.1  | `is_ready` / `candidates_for`: a late foreign row must not remove a current candidate | fixed | `a_late_older_wiki_row_from_another_writer_changes_no_answer`: `notes/wiki.md` stays a candidate of `docs/b.md` after the late commit |
| R2-F1.2  | `request_documents` → load: facts must match this process's byte read | fixed | same test: `request_documents` returns hash equal to `verified_hash` and the current headings |
| R2-F1.3  | `symbol_documents` → heading scan: only verified facts | fixed | both late-row tests: workspace symbols equal the oracle after the late commit |
| R2-F1.4  | references (heading, Markdown link, wiki) equal the oracle after a late foreign write | fixed | both late-row tests: references at every heading, Markdown-link, and wiki probe equal the oracle |
| R2-F1.5  | DSL "transcluded by" equals the oracle after a late foreign write | fixed | `a_late_older_transcluding_row_from_another_writer_changes_no_answer`: DSL transcluded-by equals the oracle, `docs/compose.md` stays a candidate |
| R2-F1.6  | heading rename and its candidate refresh include the current referrer | fixed | wiki late-row test: heading rename equals the oracle, which edits `notes/wiki.md`; `refresh_rename_referrers` uses the checked `candidates_for` |
| R2-F1.7  | file rename and its candidate refresh include the current wiki spelling edit | fixed | wiki late-row test: `will_rename_files` of `docs/b.md` and every file rename equal the oracle |
| R2-F1.8  | `revalidate` / `refresh` replace mismatching rows (clean per reviewer; add late-writer coverage) | clean | refresh replaces a mismatching row (`reparsed == 1`); new `a_row_committed_after_a_refresh_checked_it_changes_no_answer` covers a commit after the refresh's check |
| R2-F1.9  | open-buffer overlay replaces rows (clean per reviewer) | clean | the overlay is applied after `verified_view`; `open_buffers_replace_rows_and_never_write_one` passes |
| R2-F1.10 | `MemoryFacts` is process-local (clean per reviewer) | clean | process-local; salvaged foreign rows now checked too: `a_salvaged_older_row_in_memory_decides_no_answer` |
| R2-F2    | A cache failure during a request returns an incomplete answer | fixed | repaired by the R2-F1 `verified_view` change (rows missing after a failed salvage are unverified and re-read); 12 new first-answer tests; mutation check fails 10 of 11 plus the build test |
| R2-F2.1  | candidate lookup failure: refill memory before the request consumes candidates | fixed | `an_undecodable_candidate_row_after_ready_leaves_the_first_candidates_complete` |
| R2-F2.2  | document load failure: rebuild facts and retry the request | fixed | `undecodable_headings_after_ready_leave_the_first_request_documents_complete`, `..._first_request_graph_complete` |
| R2-F2.3  | heading-scan failure: complete memory-backed symbols | fixed | `undecodable_headings_after_ready_leave_the_first_workspace_symbols_complete` |
| R2-F2.4  | references handler (ordinary, wiki, DSL) retries after switch | fixed | `after_ready_damage_the_first_{heading,wiki,transcluded_by}_references_equal_memory` (LSP sessions vs `DMLS_CACHE_DIR=off`) |
| R2-F2.5  | heading rename including pre-graph refresh retries | fixed | `after_ready_damage_the_first_heading_rename_equals_memory`; `undecodable_headings_first_read_by_a_rename_refresh_leave_the_renames_complete` |
| R2-F2.6  | file rename including pre-graph refresh retries | fixed | `after_ready_damage_the_first_file_rename_equals_memory` (wiki spelling edit present); the same facts-level refresh test |
| R2-F2.7  | workspace-symbol handler retries | fixed | `after_ready_damage_the_first_workspace_symbols_equal_memory` |
| R2-F2.8  | build-time failure reconciliation (clean per reviewer) | clean | existing fault tests pass; new `a_store_failing_during_the_build_gets_a_full_round_before_any_answer` fails when the `switched → FactsPlan::Full` branch is disabled |
| R2-F2.9  | salvage when reads work; failure at open (clean per reviewer) | clean | `a_readable_store_that_fails_a_write_is_salvaged_into_memory`, `a_corrupted_cell_read_through_root_facts_falls_back_to_memory`, off/read-only/corrupt-store session tests pass |
| R2-F2.10 | discovered: comments and docs claimed salvage keeps every readable row (one undecodable row salvages nothing) | fixed | `source.rs` fallback table and `fall_back` doc, `workspace.rs` module doc, `docs/lsp/indexing.md` fallback table corrected |
| R2-F2.11 | discovered sibling sites: `facts_changed`/`facts_saved`, `prepare_rename`, `RootFacts::view` memory `unwrap_or_default`, `request_documents` per-path check, `RootFacts::candidates_for`/`scan_headings` | clean | no client answer, snapshot-only, or memory that cannot fail; `request_graph` checks every listed file; the two `RootFacts` readers are not on any request path |

- starting the work on 'R2-F1 shared rows change after validation' at 06:47:21
        - defect class: validation of mutable shared cache data expires before the decisions that consume it (a check passes once, later reads trust whatever the shared backend now holds)
        - delegated to a foreground `rust-developer` subagent; its log items are recorded below when it returns
        - subagent (rust-developer) report, R2-F1:
                - design: `FactsSource::view(&ViewRead)` returns every row hash together with the request's candidates, facts, or headings from one state; `StoreFacts` takes them inside one SQLite read transaction (`IndexStore::begin_read` / `end_read`, a WAL snapshot); `MemoryFacts` uses the default
                - `WorkspaceFacts::verified_view` (used by `candidates_for`, `request_documents`, `request_graph`, `symbol_documents`) treats a checked path as unverified when its row in any owning root is missing or its hash differs from `verified`; an unverified row is dropped and the file is re-read, re-hashed, re-parsed (`reread` → `pass`, which rewrites the row), and the fresh parse decides, including whether it is a candidate
                - no retry loop: one view plus fresh parses has a fixed cost and cannot loop against a writer that keeps committing
                - sibling sweep: `refresh_rename_referrers` and `bench.rs` go through the checked readers; `refresh`'s `load` and `revalidate`'s `hashes` are write-side checks and stay as they were, since a later commit is caught at request time
                - new tests: `a_late_older_wiki_row_from_another_writer_changes_no_answer`, `a_late_older_transcluding_row_from_another_writer_changes_no_answer`, `a_row_committed_after_a_refresh_checked_it_changes_no_answer` (`tests/l1/index_store_requests.rs`); `a_commit_during_a_read_transaction_is_invisible_until_it_ends` (`index_store_concurrency.rs`); `a_salvaged_older_row_in_memory_decides_no_answer` (`workspace_tests.rs`)
                - mutation check: with `verified_view`'s re-read disabled, all four of those tests fail
                - two unit tests that pinned the old partial answer after a store failure were updated to expect the complete answer
                - docs: `darkmatter/docs/lsp/indexing.md` sections "Answering a request from the store", "Several DMLS processes", "When a store cannot be used"
                - scoped runs: `-E 'test(index_store) | test(wiki_universe) | test(bench_index) | test(full_index_removal) | test(facts)'` 121 passed; `--lib -E 'test(index_store)'` 34 passed; `-E 'test(rename) | test(references) | test(workspace_symbol) | test(index_build) | test(lsp_session)'` 194 passed; `just lint` exit 0
                - one breach of instructions: the subagent used `--no-fail-fast` on one 3-test mutation run (since reverted); no wider effect
        - orchestrator check: read the `workspace.rs`, `source.rs`, `store.rs` diff; the readers drop unverified rows before use and the fresh parse replaces them in candidates, facts, and headings. Because a failed salvage leaves rows missing, which counts as unverified, the same path should also complete R2-F2's in-request answers; R2-F2 is verified separately
- work completed for 'R2-F1 shared rows change after validation' at 06:56:44
- starting the work on 'R2-F2 cache failure during a request' at 06:56:44
        - defect class: recovery invalidates cached state without redoing the operation that already consumed the incomplete recovery data
        - delegated to a foreground `rust-developer` subagent
        - subagent (rust-developer) report, R2-F2:
                - sweep result: the R2-F1 repair already closes every child. A failed salvage leaves memory empty or partial; the missing hashes mark every checked file unverified, `reread` parses them from disk, and the first answer is complete. `take_switched` withdraws ready, so the router's next request queues a full revalidation that refills memory
                - no behavior change was needed; comments and docs that said salvage keeps "every row the store can still read" were corrected (one undecodable row salvages nothing): the `source.rs` fallback table and `fall_back` doc, the `workspace.rs` module doc, and one new row in `docs/lsp/indexing.md` "When a store cannot be used"
                - new facts-level tests (`tests/l1/index_store_requests.rs`): `an_undecodable_candidate_row_after_ready_leaves_the_first_candidates_complete`, `undecodable_headings_after_ready_leave_the_first_request_documents_complete`, `undecodable_headings_after_ready_leave_the_first_request_graph_complete`, `undecodable_headings_after_ready_leave_the_first_workspace_symbols_complete`, `undecodable_headings_first_read_by_a_rename_refresh_leave_the_renames_complete`
                - new router-level session tests (`tests/l1/index_store_live.rs`), compared against a `DMLS_CACHE_DIR=off` session; each damages the store by path and by heading level, then checks the first answer, the withdrawn ready (6 of 6 files read on the next request, 0 after), and every request again: `after_ready_damage_the_first_{heading,wiki,transcluded_by}_references_equal_memory`, `after_ready_damage_the_first_heading_rename_equals_memory`, `after_ready_damage_the_first_file_rename_equals_memory`, `after_ready_damage_the_first_workspace_symbols_equal_memory`, `a_store_failing_during_the_build_gets_a_full_round_before_any_answer`
                - mutation checks: with the unverified filter narrowed to "present rows with a different hash", 10 of 11 first-answer tests failed (the rename-refresh facts test passes either way because the refresh writes the candidates into memory); with the `switched → FactsPlan::Full` branch disabled, the build test failed (`reads: 6`). Both temporary changes were reverted
                - siblings: `facts_changed` / `facts_saved` (no client answer; a switch withdraws ready) clean; `prepare_rename` reads the snapshot, not the facts, so it is clean; the `RootFacts::view` `unwrap_or_default()` runs on memory only, which cannot fail, so it is clean; `request_documents` checks only its given paths, which is clean because production goes through `request_graph`, which checks every listed file; `RootFacts::candidates_for` / `scan_headings` are no longer on any request path (tests only)
                - scoped runs: `-E 'test(index_store) | test(wiki_universe) | test(facts) | test(fallback) | test(fault)'` 151 passed; `-E 'test(rename) | test(references) | test(workspace_symbol) | test(index_build) | test(lsp_session)'` 202 passed; `just lint` exit 0; each new test under 0.35 s
- work completed for 'R2-F2 cache failure during a request' at 07:04:12
- starting phase verification (`just test`, `just lint`, `just check-tier-coverage darkmatter` in `darkmatter/`) at 07:04:12
        - `just test` (darkmatter/): 10471 passed, 0 failed, 10 skipped, 36 s; a second run reported no SLOW, FAIL, or TIMEOUT line
        - `just lint` (darkmatter/): exit 0, including the `zed-dmls` wasm32-wasip2 check
        - `just check-tier-coverage darkmatter` (repo root): 0 stranded tests
        - cross-OS check not run: the change adds an SQLite read transaction and request-side re-reads with no new path, process, or OS-specific code; the new tests use temp dirs and the existing session harness. CI covers the other OSes
- phase verification completed at 07:06:28

### Completion Report

- **outcome: complete**; elapsed about 20 minutes (06:46:14 to 07:06)
- parent findings: total 2 = 2 fixed + 0 already fixed + 0 blocked + 0 deferred + 0 unresolved + 0 disputed
        - R2-F1 **fixed**: children F1.1–F1.7 fixed, F1.8–F1.10 clean with new coverage
        - R2-F2 **fixed** by the same mechanism, with no further code change: children F2.1–F2.7 fixed, F2.8–F2.9 clean, discovered F2.10 fixed (doc drift), F2.11 clean
- verified repairs:
        - every request reader (`candidates_for`, `request_documents`, `request_graph`, `symbol_documents`) reads one consistent state per root (SQLite read transaction) with all row hashes, and uses a row only when its hash equals this process's last read; any other listed file is re-read and its fresh parse decides, including candidacy
        - a mid-request switch to memory therefore still answers in full: rows missing after a failed salvage are re-read; ready is withdrawn and the next request rebuilds
- open finding IDs: none
- observation not repaired (not a finding): the root `just test dmls` recipe fails before nextest starts; outside this spec's code
- not measured: the per-request row-hash scan's latency on this repository (about one hash per stored file); no review finding requires a new benchmark, so `deferred_perf_measurement` is not set
- comment drift detected and resolved: salvage was documented as keeping every readable row; the code salvages all rows or none; docs and comments now say so
- changed files:
        - `darkmatter/dmls/src/index_store/store.rs` (`begin_read` / `end_read`): R2-F1
        - `darkmatter/dmls/src/index_store/source.rs` (`ViewRead`, `View`, `FactsSource::view`, `RootFacts::view`, docs): R2-F1, R2-F2.10
        - `darkmatter/dmls/src/index_store/mod.rs` (re-exports): R2-F1
        - `darkmatter/dmls/src/index_store/workspace.rs` (`verified_view`, `reread`, readers, docs): R2-F1, R2-F2
        - `darkmatter/dmls/src/index_store/workspace_tests.rs`: R2-F1.10, updated partial-answer tests
        - `darkmatter/dmls/tests/l1/index_store_requests.rs`: R2-F1.1–F1.8, R2-F2.1–F2.3, F2.5–F2.6
        - `darkmatter/dmls/tests/l1/index_store_live.rs`: R2-F2.4–F2.8
        - `darkmatter/dmls/tests/l1/index_store_concurrency.rs`: read-transaction isolation (R2-F1)
        - `darkmatter/docs/lsp/indexing.md`: R2-F1, R2-F2.10
- final verification: `just test` 10471 passed / 0 failed; `just lint` exit 0; `just check-tier-coverage darkmatter` 0 stranded; scoped subagent runs listed above
