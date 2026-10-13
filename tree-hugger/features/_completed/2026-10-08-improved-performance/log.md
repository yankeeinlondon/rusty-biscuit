---
kind: log
feature: 2026-10-08-improved-performance
implementation_1: "2026-10-09T11:04:03-07:00"
deferred_perf_measurement: false
---

# Improved Performance: Review Implementation Log

## Implementation of Review Findings #1

> **started at:** 2026-10-09T11:04:03-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'tree-hugger/features/2026-10-08-improved-performance/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs started

### Finding Ledger

| ID       | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
|----------|-----------------------------------------------------------|-------------|----------------------------------|
| R1-F1    | Failed files bypass flushes and defeat early pipe cancellation | fixed | all child rows verified; `Flush`→`Flushed` proof makes a skipped flush a compile error; `just test` 2074/2074 |
| R1-F1.01 | `imports` native-error branch: flush, stop within one window on closed stdout; exit 0 | fixed | `report::failed_files_still_stop_imports_and_lint_at_a_failing_stdout` (default + `--plain` imports: <8 errors, exit 0; fails on pre-fix source with 120) |
| R1-F1.02 | `lint` native-error branch: same cancellation bound | fixed | same test, `lint` default + `--plain` (`--warn unwrap-call`) |
| R1-F1.03 | `lint` adapter-error branch: stop before later adapter invocations | fixed | `report::adapter_failures_still_stop_lint_at_a_failing_stdout` (cfg(unix) oxlint stub: <8 errors, <8 stub runs; control 120/120) |
| R1-F1.04 | `write_listing` functions/types/symbols: flush on failed file | already fixed | already flushed on every path; now type-enforced via `as_flow(io::Result<Flushed>)`; pinned by `--plain functions` case |
| R1-F1.05 | `write_listing` classes: flush on failed file | already fixed | type-enforced; pinned by `--plain classes` case |
| R1-F1.06 | `write_json` ordinary and complete-index: failed delivery still flushes | already fixed | `JsonReport::flush` returns `Flushed`; pinned by `--json` and `--json-full` cases |
| R1-F1.07 | `write_json_lines`: flush after failed delivery | already fixed | type-enforced; pinned by `--jsonl` case |
| R1-F1.08 | `write_table`: same bound | already fixed | type-enforced; pinned by `--format table` case |
| R1-F1.09 | `write_tree`: same bound | already fixed | type-enforced; pinned by `--tree` case |
| R1-F1.10 | `write_export` TSV and ctags: same bound | already fixed | type-enforced; pinned by `--format tsv` and `--format ctags` cases |
| R1-F1.11 | Markdown and Prose listing layouts: same bound | already fixed | type-enforced; pinned by `--format markdown` and `--format prose` cases |
| R1-F1.12 | `tally`, `refs`, imports/lint JSON: scan-before-output semantics preserved | clean | scan-before-output: `tally` and `refs` phase 1 return `Flush::nothing_written()`; imports/lint JSON pass through the common path with an empty buffer; existing tally/refs/JSON tests pass |
| R1-F2    | Reference selection duplicates whole-file data for every match | fixed | all child rows verified; `Phased` no longer `Clone`; RSS at 80 matches 230.4 MB → 31.0 MB (descriptive) |
| R1-F2.01 | One-file `Phased::Analyzed`: one retained index, no per-match clone | fixed | `refs::tests::many_same_named_matches_in_one_file_share_its_one_analysis` (40 matches, 1 retained state) |
| R1-F2.02 | Multi-file `Phased::Read`: one retained snapshot per file | fixed | `refs::tests::only_matching_files_retain_one_byte_snapshot_each` (80 matches, 2 states, exact bytes); 1 MB-comment case 111.3 → 29.2 MB |
| R1-F2.03 | Multi-file worker's initial snapshot: no unconditional source copy | fixed | `SourceFile::query_retaining`/`SymbolCache::query_retaining` move bytes back; lib `symbols_cache::retaining_queries_give_back_the_source_they_analyzed_on_every_lookup`; nonmatching file retains nothing (unit test above) |
| R1-F2.04 | `--id` selection (one/multi-file): move file state, same report | fixed | `refs::tests::an_id_selects_exactly_one_retained_file`; CLI `--id` one-file/multi-file and `--callees` |
| R1-F2.05 | Missing name (one/multi-file): empty report unchanged | clean | CLI test: `absent` → null declaration, no files/errors, exit 0, both scan shapes |
| R1-F2.06 | References/callers/callees text and JSON ambiguity decisions unchanged | clean | CLI `refs::many_same_named_methods_in_one_file_stay_one_exact_ambiguity`: references/callers/callees, text+JSON, cold+warm: exit 1, empty stdout, exactly 40 rows, stable IDs/order |
| R1-F2.07 | Prelude analysis reuse: owned removal, no whole-index clone | clean | `declarations::run` takes reuse entries with `remove` (no clone); other sweep hits (`prelude.rs` PathBuf key, one-time `exports.clone()`, single reported record) are not per-match copies |
| R1-F3    | Delivery panics leave scan workers waiting forever | fixed | all child rows verified; `StopOnDrop` guard installed before spawning |
| R1-F3.01 | Delivery callback panic: stop, wake, join, propagate | fixed | `scan::tests::a_panicking_delivery_is_propagated_without_waiting_on_workers` (200 files, 1/2/8 workers, first + mid-stream panic; 10 s deadline; fails at deadline pre-fix) |
| R1-F3.02 | Writer/renderer panic inside `deliver`: same cleanup | fixed | `scan::tests::a_writer_panicking_inside_a_delivery_is_propagated` |
| R1-F3.03 | `scope.spawn` failure after some workers start: wake started workers | fixed | invariant: guard bound before the first `scope.spawn`, so a spawn unwind drops it before the scope joins; thread-creation failure not injectable without a test hook |
| R1-F3.04 | Worker/job panic: ordering preserved | clean | `a_panicking_job_is_propagated_in_file_order` passes; stop now runs during unwind, before join |
| R1-F3.05 | Returned write failure / `Break`: stop within one window | clean | `a_failed_write_stops_within_one_window` + closed-pipe CLI tests pass |
| R1-F3.06 | Empty input and normal completion | clean | empty-input and ordered-delivery tests pass |
| R1-H1    | Human review item: closure of deferred design work (nonblocking, author decision) | deferred | human decision on closure of deferred design work; author choice, nonblocking per review |
| R1-N1    | Nonblocking: keep policy experiments as separately approved changes | deferred | nonblocking recommendation; no code change requested |
| R1-N2    | Nonblocking: describe god-file I/O as two reads across discovery + analysis | deferred | nonblocking docs wording recommendation; outside the three-finding repair inventory |
| R1-N3    | Nonblocking: parent-index extension to direct library APIs needs measurement | deferred | nonblocking; requires large-file measurements and an author decision |

### Progress

- starting the work on 'R1-F1 — Failed files bypass flushes' at 11:05:02
        - defect class: a per-file delivery that is asked to flush could return `Continue` without flushing, and `scan::in_file_order` counted it as flushed and advanced the admission window
        - design: the scheduler now requires proof; `Flush` is no longer `Copy`/`Clone`, `Flush::apply` returns a private-constructible `scan::Flushed`, and `deliver` must return `ControlFlow<B, Flushed>`; scans that write nothing until the end acknowledge with the explicitly named `Flush::nothing_written`; a skipped flush is now a compile error rather than a silent window advance
        - `main.rs` imports/lint delivery restructured: native-analysis and adapter failures `break` out of a settle block and fall through to the one write + `flush.apply` path (JSON accumulation also passes through it; flushing its empty buffer writes nothing, so JSON keeps its envelope, errors and exit semantics)
        - sites swept (`grep -rn in_file_order cli/src`): `main.rs` imports/lint text (fixed: both error branches now flush), imports/lint `--json` (unchanged semantics, same path); `declarations.rs` `write_export` TSV + ctags, `write_json` (ordinary and `--json-full`, via `JsonReport::flush` now returning `Flushed`), `write_json_lines`, `write_listing` (functions/types/symbols and classes; terminal, Prose, Markdown), `write_table`, `write_tree` (already flushed on every path; now type-enforced through `as_flow(io::Result<Flushed>)`); `tally` (count/stats) and `refs.rs` phase one (scan-before-output: `nothing_written`, totals/ambiguity/partial errors unchanged)
        - docs: `in_file_order`/`Flush`/`Flushed` doc comments; `docs/symbol-reports.md` (failed files take their turn at the flush; no external linter runs past the window; which modes write only after the scan); tree-hugger skill line about the flush contract
        - tests: `scan::tests::a_flush_reaches_the_writer_only_when_asked` (unit); `report::failed_files_still_stop_imports_and_lint_at_a_failing_stdout` (slow valid head + 120 non-UTF-8 siblings, closed stdout: imports/lint default and `--plain`, plus functions/classes listings, `--json`, `--json-full`, `--jsonl`, table, `--tree`, TSV, ctags, Markdown, Prose; asserts exit 0 and fewer than 8 delivered errors; open-pipe control delivers all 120); `report::adapter_failures_still_stop_lint_at_a_failing_stdout` (`cfg(unix)`: shell-script `node_modules/.bin/oxlint` stub printing `invalid-json`; asserts exit 0, fewer than 8 errors and fewer than 8 adapter runs; control: 120 runs/errors with a reader)
        - both new CLI tests fail on the pre-fix sources (120 delivered errors in each) and pass after
        - checks: `just test` 2067 passed, 0 skipped; `just lint` clean
- work completed for 'R1-F1 — Failed files bypass flushes' at 11:12:40
- starting the work on 'R1-F2 — Reference selection duplicates whole-file data' at 11:13:09
        - defect class: each declaration candidate owned a deep copy of its file's evidence index (one-file scan) or source snapshot (multi-file scan), so selection memory grew with matches × file size; the multi-file worker also copied every file's bytes before knowing whether it matched
        - sites swept (`grep -rn '\.clone()' cli/src` over `SourceFile`/`FileSymbolIndex`/`SymbolQueryResult`/`Phased`/analysis results): `refs.rs` candidate loop `kept.clone()` (R1-F2.01/.02/.04, fixed), `refs.rs` worker `source.clone()` (R1-F2.03, fixed), `refs.rs` phase-two record lookup clones one record (kept: one record, not file state); `prelude.rs:68` `candidate.clone()` clones a `PathBuf` key only; `declarations.rs` reuse map consumed with `remove` (R1-F2.07 confirmed, no change); `declarations.rs:916` `index.symbols = exports.clone()` copies a prelude file's synthetic export records once, when that prelude file is reported (records only, not per match; outside this defect class; no change); no other per-match whole-file copies in `cli/src`
        - design: `refs::select` builds a `Selection` of one `MatchedFile` per matching file (display, native path, first record, `Phased` state) plus a lightweight `Candidate` per match (file position, start point, label, qualified name, ID); `Phased` no longer derives `Clone`; when exactly one candidate remains, `Selection::into_unique` moves that file's state into the evidence phase and the reported record is looked up there by ID (falling back to the kept first record, as before); the ambiguity message reads the same fields, so its text and order are unchanged
        - R1-F2.03: new library API `SourceFile::query_retaining` and `SymbolCache::query_retaining` return the result and the `SourceFile` (text moved through `TreeFile::into_source`, never copied; `SymbolCache::query` delegates to it, crate-private `SourceFile::analyze` became `analyze_retaining`); CLI `Analyzer::analyze_retaining` (`analyze_source` delegates); `refs` keeps the returned bytes only for a matching file in a multi-file scan, so a nonmatching file keeps nothing and the selected file's evidence is analyzed from the exact bytes phase one read
        - docs: `docs/references.md#hug-refs` (memory per matching file, not per match), tree-hugger skill `hug refs` line, `api-reference.md` (`_retaining` forms, `analyze_retaining`); `refs.rs` type docs rewritten
        - tests: `refs::tests::many_same_named_matches_in_one_file_share_its_one_analysis` (40 matches, 1 retained `Analyzed` state, source order), `only_matching_files_retain_one_byte_snapshot_each` (2×40 matches + unrelated file: 80 candidates, 2 `Read` states with the exact bytes, none for `other.py`), `an_id_selects_exactly_one_retained_file`; CLI `refs::many_same_named_methods_in_one_file_stay_one_exact_ambiguity` (one-file and multi-file scans × references/callers/callees × text/JSON × cold/warm: exit 1, empty stdout, exactly 40 listed rows with location/label/qualified name, identical IDs; `--id` of the 18th selects it in both scans and for `--callees`; `absent` is an empty report); library `symbols_cache::retaining_queries_give_back_the_source_they_analyzed_on_every_lookup` (Miss, Hit, WidenedKinds, WidenedCapabilities{parsed} and uncached forced-language/experimental: result equals `query`, source identity unchanged)
        - the unit tests do not compile against the pre-fix sources (no per-file state structure); the CLI test is a behavior-preservation control and passes before and after; removing `Clone` from `Phased` makes per-match copies a compile error
        - measurements (debug build, `--no-cache`, `RAYON_NUM_THREADS=2`, `/usr/bin/time -l`, reviewer's fixture): one-file 20/40/80 matches 29.6/30.4/31.0 MB (review: 37.9/66.6/230.4 MB); multi-file 29.2/29.5/30.6 MB (review: 29.4/29.7/31.0 MB); multi-file 80 matches + 1 MB comment 29.2 MB (review: 111.3 MB); descriptive only
        - checks: `just test` 2072 passed, 0 skipped; `just lint` clean (after fixing two clippy findings in the new code)
- work completed for 'R1-F2 — Reference selection duplicates whole-file data' at 11:22:26
- starting the work on 'R1-F3 — Delivery panics leave scan workers waiting' at 11:23:43
        - defect class: cancellation (`Window::stop`) ran only on the explicit post-loop path of `scan::in_file_order`, so any unwind of the coordinating thread (delivery panic, writer/renderer panic inside `deliver`, `scope.spawn` panic after some workers started) skipped it; workers blocked in `Window::admit`'s Condvar wait never woke and `thread::scope` hung joining them
        - design: `struct StopOnDrop<'a>(&'a Window)` whose `Drop` calls `stop()` (lock, set flag, `notify_all`); bound as `_stop` first thing inside the `thread::scope` closure, before the spawn loop, so it drops before the scope joins on normal return, `Break`, the job-panic `resume_unwind` and any other unwind; the explicit post-loop `window.stop()` was removed (the guard replaces it); the receiver is still dropped explicitly and workers never block on the unbounded `mpsc` send; `in_file_order` doc comment now states no worker outlives the call on any exit, unwinding included
        - sites swept (`grep -rn 'thread::scope\|Condvar\|scope.spawn\|thread::spawn' tree-hugger/lib/src tree-hugger/cli/src`): `cli/src/scan.rs::in_file_order` (the only scoped-thread/Condvar coordinator; fixed); `lib/src/file/parents.rs::with_parent_index` (thread-local index restored by its `Restore` Drop guard on panic; confirmed, no change); `lib/src/file/extract/complexity/tests.rs` `thread::Builder::spawn` (test-only, joined, no coordination; no change); every `in_file_order` consumer (exports, JSON, JSON Lines, tally, listings/classes, tables, outlines, imports/lint, `refs` phase one) inherits the guard, none needed its own
        - dispositions: R1-F3.01 fixed (test below); R1-F3.02 fixed (writer-panic test); R1-F3.03 fixed by invariant: the guard is constructed before the first `scope.spawn`, so a spawn panic unwinds through its drop and wakes already-started workers before the join (not induced in a test: thread-creation failure cannot be injected without a seam); R1-F3.04 preserved (`a_panicking_job_is_propagated_in_file_order` passes; stop now happens in the unwind, still before the join); R1-F3.05 preserved (`a_failed_write_stops_within_one_window`); R1-F3.06 preserved (empty input returns before the scope; ordered-delivery test passes)
        - tests (L1, `hug` binary unit tests): `scan::tests::a_panicking_delivery_is_propagated_without_waiting_on_workers` (200 files, 1/2/8 workers, panic on delivery 0 and mid-stream 37: original payload, exact delivered count, admission held, no work after unwind), `a_writer_panicking_inside_a_delivery_is_propagated` (flush of a panicking writer inside `deliver`); both run the scan on a spawned thread under a 10 s `recv_timeout` deadline, so a regression fails instead of hanging (nextest runs each test in its own process, so the stuck thread ends with it; a re-exec subprocess was not needed)
        - both new tests fail at the 10 s deadline against the pre-fix scheduler (guard removed, post-loop `stop` restored) and pass after
        - checks: `cargo nextest run -p tree-hugger-cli --bin hug scan::` 8/8; `just test` 2074 passed, 0 skipped; `just lint` clean
        - docs: tree-hugger skill `in_file_order` line updated; `docs/symbol-reports.md` makes no worker-lifetime claim, so unchanged
- work completed for 'R1-F3 — Delivery panics leave scan workers waiting' at 11:29:01
- orchestrator reconciliation at 11:33:04
        - inspected the combined diff (`scan.rs` `Flush`/`Flushed`/`StopOnDrop`, `main.rs` imports/lint settle path, `refs.rs` `Selection`/`MatchedFile`, library `query_retaining`)
        - re-ran `just test` in `tree-hugger/`: 2074 passed, 0 skipped
        - re-ran `just lint` in `tree-hugger/`: clean for both packages
        - `just check-tier-coverage tree-hugger`: 0 stranded tests
        - OS review: no `#[cfg]`-specific or path-comparison production code changed; the adapter stub test is `#[cfg(unix)]` with its Unix-only import inside the test body, and the shared helpers are used by the cross-platform test, so Windows builds have no dead code; Windows/WSL2 are left to the scheduled CI legs (no local cross-check run)

### Completion Report

- **outcome:** complete
- **elapsed:** about 30 minutes (11:04:03 → 11:34)
- **parent findings:** 3 total = 3 fixed + 0 already fixed + 0 blocked + 0 deferred + 0 unresolved
        - instance rows: R1-F1 12 rows (3 fixed, 8 already fixed and now type-enforced/pinned, 1 clean); R1-F2 7 rows (4 fixed, 3 clean); R1-F3 6 rows (3 fixed, 3 clean)
        - nonblocking items outside the repair inventory (R1-H1, R1-N1, R1-N2, R1-N3) are deferred to the author; the review states none blocks these repairs
- **verified repairs**
        - R1-F1: every `in_file_order` delivery must return a `Flushed` proof only `Flush` can produce; imports/lint failures now share the one write-and-flush path, so a closed stdout stops within one window and external adapters are not run past it
        - R1-F2: `hug refs` keeps one state per matching file plus lightweight candidates; `Phased` is not `Clone`; nonmatching files no longer retain source copies (bytes are moved back through the new `query_retaining` library API)
        - R1-F3: a `StopOnDrop` guard stops and wakes the window on every exit of the scheduler, including unwinding, before the scope joins
- **open finding IDs:** none among R1-F1..R1-F3; deferred nonblocking items R1-H1 (author scope decision), R1-N1, R1-N2, R1-N3
- **performance measurement:** not deferred; the R1-F2 debug-build RSS samples were taken (descriptive, not thresholds)
- **changed files**
        - `tree-hugger/cli/src/scan.rs` (R1-F1, R1-F3), `tree-hugger/cli/src/main.rs`, `declarations.rs`, `report.rs` (R1-F1), `tree-hugger/cli/src/refs.rs` (R1-F1, R1-F2)
        - `tree-hugger/lib/src/file/source_file.rs`, `lib/src/file/tree_file.rs`, `lib/src/cache/symbol_cache.rs` (R1-F2)
        - tests: `tree-hugger/cli/tests/symbols/report.rs` (R1-F1), `tree-hugger/cli/tests/symbols/refs.rs` (R1-F2), `tree-hugger/lib/tests/l1/symbols_cache.rs` (R1-F2), unit tests in `scan.rs`/`refs.rs`
        - docs: `tree-hugger/docs/symbol-reports.md` (R1-F1), `tree-hugger/docs/references.md` (R1-F2), `.claude/skills/tree-hugger/SKILL.md` (all), `.claude/skills/tree-hugger/api-reference.md` (R1-F2)
- **final verification**
        - `just test` (tree-hugger): 2074/2074 passed, 0 skipped (all findings)
        - `just lint` (tree-hugger): clean (all findings)
        - `just check-tier-coverage tree-hugger`: 0 stranded (new tests are L1 in declared targets)
        - pre-fix failure checks reported by subagents: R1-F1 CLI tests delivered 120 errors on old sources; R1-F3 unit tests hit their 10 s deadline on the old scheduler
