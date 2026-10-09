---
$schema: feature-review.yaml
ready: false
findings:
  - title: "R1-F1 — Failed files bypass flushes and defeat early pipe cancellation"
    priority: high
  - title: "R1-F2 — Reference selection duplicates whole-file data for every match"
    priority: high
  - title: "R1-F3 — Delivery panics leave scan workers waiting forever"
    priority: medium
human_review: true
human_review_items:
  - |-
    Decide how to close the explicitly deferred design work after the code findings are repaired. The implementation preserves existing cache policy and public output; it does not implement the optional changes to cache storage, eager test/deprecation flags, scope coverage, reference-candidate output, external-linter batching, terminal discovery, cache cleanup, or the release allocator/profile.

    - Close this feature around the implemented improvements and schedule separately approved follow-ups. This keeps each public or storage change reviewable on its own; recommended.
    - Choose which deferred changes should be included before closing this feature, using the options and measurements already recorded in the specification. This expands the remaining implementation and review work.

    No choice is needed to repair the three findings in this review. This item concerns the unfinished design choices, not approval of the existing performance improvements.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-09T10:50:57-07:00"
spec: 2026-10-08-improved-performance/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-08-improved-performance/log.md
description: "A **feature** review of `2026-10-08-improved-performance/spec.md`"
feature: 2026-10-08-improved-performance/review-1.md
next: 2026-10-08-improved-performance/review-2.md
---

# Review 1: Improved performance

The feature is **not ready for production**. The main algorithmic improvements are implemented and the existing verification passes, but failure paths can continue scanning after stdout closes, reference selection still multiplies whole-file allocations, and a delivery panic can hang the new scheduler. All three findings can be repaired without changing public contracts or making a new design choice.

There is no previous implementation review for this feature. There are no carried findings, partially repaired previous findings, or newly discovered siblings of a previous review finding to reconcile. The specification's earlier design review is not an implementation repair ledger.

The specification, plan, implementation log, relevant current topic documentation, and implementation changes from `350cec8a2` through `2cdc7d368` were reviewed. Unrelated repository CI planning changes in that range were excluded. The plan supplies the implementation design; there is no separate design document. Only this review and the spec's `review_iterations` were changed.

Production symbols linked in the findings belong to **tree-hugger-cli**. Library symbols in the coverage discussion belong to **tree-hugger**.

## Verification

| Check | Result | What it establishes |
| --- | --- | --- |
| `just test` in `tree-hugger/` | 2,064 passed, 0 skipped | Level 1 library, binary, and CLI integration checks, including cache corruption, projections, reference matrices, literal-path selection, exports, ordering, and pipe handling |
| `just lint` in `tree-hugger/` | Passed for both packages | Existing lint checks accept the implementation |
| `just check-tier-coverage tree-hugger` | No stranded tests | Tier markers have live recipes |
| `just test-l2` in `tree-hugger/` | Seven test entries passed | Actual tmux and WezTerm checks exercised styled declarations, wrapping, tables, outlines, statistics, reference reports, and god-file rendering; the headless tags reader also passed |
| Additional shipped-CLI probes | Two defect classes reproduced | Mixed-success closed-pipe scans and large ambiguous reference selections, detailed below |
| Extracted scheduler probe | Delivery panic hung until killed | Exact production scheduler body exercised independently, detailed under R1-F3 |

The fast Kitty test entries are availability-gated; their passing entries are not claimed as actual Kitty rendering evidence. Level 2 used the area's existing no-focus harness recipe. There are no keyboard, mouse, paste, or input-encoder requirements introduced by this feature, so Level 3 is not required. Missing cross-OS receipts do not affect this readiness decision.

The CLI probes used the current `target/debug/hug`, scratch directories outside fixture exclusions, isolated `HOME` and temporary-cache directories, `RAYON_NUM_THREADS=2`, and `--no-cache`. Expected outcomes were independently stated from the contract: an eight-file admission window, exactly 80 declarations for the generated ambiguous selector, a uniquely selected declaration for an ID selector, and an empty report for a missing selector. Agreement between cached and uncached reports was not used as the correctness oracle.

New library integration tests are included by the declared `l1` target's modules. New CLI checks are included by `tests/symbols/main.rs`; scheduler checks are compiled into the `hug` binary's unit tests. Their names keep them in Level 1. The existing Level 2 targets require `terminal-tests`, enabled by both the live recipe and package CI metadata. No test target or tier-placement gap was found.

## R1-F1 — High: Failed files bypass flushes and defeat early pipe cancellation

**Defect class:** early returns from failed per-file deliveries acknowledge a requested flush without actually flushing buffered output, admitting later work after the reader has closed.

The [main delivery callback](../../cli/src/main.rs:1160), which streams `imports` and `lint`, returns `Continue` immediately after a native analysis error at line 1166 and an external-adapter error at line 1191. Neither branch applies its `Flush`. The successful text branch does apply it at line 1219. The scheduler's [in_file_order](../../cli/src/scan.rs:175), which bounds admitted files, advances its window when a delivery returns after being told to flush; it cannot distinguish these skipped flushes from completed ones.

The consequence is more than delayed error reporting. Buffered output from an earlier successful file stays unwritten while every later failing file is read, and external tools can keep launching. Only the final flush notices the closed pipe. Exit 0 is correct for the closed pipe, but the promised bound on unnecessary work is broken.

Reproduction: copy `lib/tests/fixtures/symbols/rust.rs` to scratch `a.rs`, append a valid function containing 2,000 `let x = 1;` statements to make it the slow first file, and create `b000.rs` through `b119.rs` from the same fixture with just one appended invalid UTF-8 byte, `0xff`. Add a root Cargo manifest. Close the subprocess stdout pipe's read end immediately after spawning `hug --verbose imports --plain --no-cache`, then drain stderr and wait. With two workers, no more than eight files should be started before the first failed flush; at most seven later file errors should therefore be delivered. **All 120 file errors were delivered**, followed by exit 0. `lint --plain` reproduced the same failure. Default text modes reproduced it too.

For the adapter sibling, keep that tree and add 120 unedited copies of the TypeScript fixture. Install a scratch-local `node_modules/.bin/oxlint` stub that returns a version for `--version` and `invalid-json` for analysis. The same closed-pipe `lint --plain` run delivered **240 errors, including all 120 adapter parse failures**. The expected result is cancellation at the first required flush, before reaching those later files.

The full decision chain is native analysis → optional import selection → summary creation → optional adapter → error return or text rendering → requested flush → window admission → final write-status mapping. Both failure returns bypass the only text flush. JSON intentionally accumulates one envelope and is outside the text-streaming early-stop promise; it should retain its existing errors and exit semantics.

| Child ID | Site | Shape tested | Observed result | Expected result | Verification evidence |
| --- | --- | --- | --- | --- | --- |
| R1-F1.01 | `imports`, native-error branch | Slow successful head, 120 invalid-UTF-8 siblings, closed stdout | 120 errors, exit 0 | At most seven later errors at two workers; exit 0 | Shipped CLI, default and plain text |
| R1-F1.02 | `lint`, native-error branch | Same fixture edit and closed stdout | 120 errors, exit 0 | Same cancellation bound | Shipped CLI, default and plain text |
| R1-F1.03 | `lint`, adapter-error branch | Earlier buffered Rust report, malformed adapter output on later TS files | 120 adapter errors plus 120 source errors | Stop before later adapter invocations | Shipped CLI with scratch-local stub |
| R1-F1.04 | `write_listing`, functions/types/symbols | Same source fixture and closed stdout | At most seven errors | Flush even when a file fails | CLI reproduction; callback applies `flush` after `settle` |
| R1-F1.05 | `write_listing`, classes | Same source fixture and closed stdout | Seven errors | Same bound | CLI reproduction; shared error settlement does not return before flush |
| R1-F1.06 | `write_json` | Same tree, ordinary and complete-index JSON | Ordinary JSON stopped with zero errors | Every failed/no-record delivery still flushes the started envelope | Ordinary JSON CLI reproduction; both forms use the same unconditional `report.flush(flush)` at line 478 |
| R1-F1.07 | `write_json_lines` | Same source fixture and closed stdout | Zero errors | Flush after a failed/no-record delivery | CLI reproduction and callback invariant |
| R1-F1.08 | `write_table` | Same source fixture and closed stdout | Zero errors | Same cancellation bound | CLI reproduction and unconditional final callback flush |
| R1-F1.09 | `write_tree` | Same source fixture and closed stdout | Seven errors | Same cancellation bound | CLI reproduction and unconditional final callback flush |
| R1-F1.10 | `write_export`, TSV and ctags | Same source fixture and closed stdout | Seven errors in each format | Same cancellation bound | CLI reproduction; both exports flush outside successful-file settlement |
| R1-F1.11 | Markdown and Prose listing layouts | Same source fixture and closed stdout | Seven and zero errors, respectively | Same cancellation bound for per-file units | CLI reproduction; shared listing callback |
| R1-F1.12 | `tally`, `refs`, imports/lint JSON | Failure beside valid source | Intentional scan-before-output paths | Preserve count/statistics totals, ambiguity decisions, partial-result errors, and final write status | Precise invariant: these consumers collect decisions before their first stdout write; they do not claim per-file text streaming |

Cross-file listing units remain allowed to buffer their current directory/package by the existing contract; this finding concerns an already buffered report whose callback fails to flush it.

**Repair:** make every text delivery, including native and adapter failures, pass through one common flush/error-to-`Break` path. Keep JSON accumulation separate. Extend the existing closed-pipe CLI tests with mixed success/failure input and adapter failure; the current new tests use successful files and miss both branches. Assert attempted/delivered work as well as exit status.

## R1-F2 — High: Reference selection duplicates whole-file data for every match

**Defect class:** each declaration candidate owns a deep copy of its file's evidence or source snapshot, making selection memory grow with the number of matches times the size of the file's data.

[Phased](../../cli/src/refs.rs:151), which retains either source bytes or an analyzed evidence index for later selection, derives `Clone`. The [candidate collection loop](../../cli/src/refs.rs:229) calls `kept.clone()` once per matching record. In a one-file scan, that clones the entire `FileSymbolIndex`: every record, occurrence, candidate list, and coverage region. In a multi-file scan, it clones `SourceFile`, including its full source `String`, for each match. Even one unique match performs one unnecessary deep clone. Additionally, the worker clones source bytes for every multi-file input before knowing whether that input matches.

This directly misses the spec's requirement that the chosen index be moved instead of cloned per match. It is independent of the deferred choice about changing public reference-candidate lists: the same lists and source snapshot can be retained once per file without changing results.

Reproduction: copy `lib/tests/fixtures/symbols/python.py` to scratch `sample.py` and append 20, 40, then 80 distinct classes, each with this method:

```python
class C0:
    def probe(self):
        return self.value
```

Change the class number for each appended class. Add `other.py` containing only an unrelated function. Measure the shipped CLI with `/usr/bin/time -l`, `--no-cache`, and two workers. `hug refs probe sample.py` must report exactly the generated number of declarations, exit 1, and write no stdout; no evidence copies are needed to produce that ambiguity report.

| Matching methods | One-file `refs probe sample.py` peak resident bytes | Multi-file `refs probe '*.py'` peak resident bytes | Complete-index output size for `sample.py` |
| --- | --- | --- | --- |
| 20 | 37,945,344 | 29,409,280 | 335,128 bytes |
| 40 | 66,568,192 | 29,655,040 | 630,308 bytes |
| 80 | 230,440,960 | 30,965,760 | 1,513,468 bytes |

These are descriptive debug-build samples, not performance thresholds. The source explains the growth: an increasingly large evidence index is copied 80 times. The multi-file branch has a sibling problem rather than being clean: appending a one-million-character ordinary comment to the same 80-method file raised the multi-file ambiguity run to **111,312,896 resident bytes**, because the source snapshot was copied for each candidate.

The decision chain was checked through file identity sorting/deduplication, all-kind/all-scope declarations, name/ID matching, candidate collection, ambiguity stopping, evidence analysis from the retained bytes, the selected record lookup, and report-direction filtering. There is no erroneous selector result in these probes. Name lookup still sees all 80 matches, ID lookup sees one, and a missing selector sees none; the defect is the unnecessary copies made before those decisions finish.

| Child ID | Site/consumer | Shape tested | Observed result | Expected result | Verification evidence |
| --- | --- | --- | --- | --- | --- |
| R1-F2.01 | One-file `Phased::Analyzed` | 20/40/80 matching method names | Entire evidence index cloned per match; about 230 MB at 80 | One retained index, lightweight candidate records | CLI memory samples and `kept.clone()` invariant |
| R1-F2.02 | Multi-file `Phased::Read` | Same matches, unrelated second file, then 1 MB comment | Entire source cloned per match; about 111 MB with the comment | One retained byte snapshot per file | CLI memory sample and `SourceFile`'s owned `String` |
| R1-F2.03 | Multi-file worker's initial snapshot | Matching and nonmatching files | `source.clone()` runs before matching for every file | Avoid unnecessary source-byte copies; preserve the selected file's exact bytes | Line 209 executes before line 213's selector; no-match output independently verified |
| R1-F2.04 | `--id` selection, one and multiple files | ID obtained from an actual `probe` declaration | Correct unique declaration and report, but `kept.clone()` still runs once | Move/reuse the one file state; same declaration and evidence | Both shipped CLI ID reports succeeded and selected `probe` |
| R1-F2.05 | Missing name, one and multiple files | `absent` | Exit 0, null declaration, empty evidence files | No invented or missing matched result | Both shipped CLI empty-report controls passed; no candidate-loop clone occurs |
| R1-F2.06 | References, callers, and callees; text and JSON | 80 ambiguous methods | All directions returned 1, empty stdout, and 80 matches; JSON ambiguity also kept stdout empty | Identical ambiguity decisions using one stored state per file | CLI direction/JSON probes; direction filtering occurs after candidate selection |
| R1-F2.07 | Prelude analysis reuse | Reused per-file analyses | Scan removes the owned analysis from its map | No whole-index clone for reuse | `declarations::run` line 241 uses `remove`; the map is consumed after synthetic exports are prepared; existing prelude tests passed |

The collection loop is language-independent, so both affected state variants apply to every supported language and to qualified/file-prefixed name selectors as well. There is no separate language-specific ownership path to repair. Public candidate construction in the resolver deliberately remains owned under the existing output contract; changing that representation is a deferred design choice, not an additional instance of this finding.

**Repair:** retain file state once and let candidate records refer to it, or retain lightweight ambiguity records and only one movable file state until uniqueness is known. Move the selected index/snapshot into the evidence phase. Preserve all kinds/scopes, deterministic ambiguity ordering, exact IDs, and analysis of the same source bytes. Add a stable ownership/work assertion for many same-named declarations rather than a timing or resident-memory threshold.

## R1-F3 — Medium: Delivery panics leave scan workers waiting forever

**Defect class:** cancellation is performed only on explicit normal exits, so unwinding the delivery thread leaves scoped workers blocked on window admission.

[in_file_order](../../cli/src/scan.rs:136), the common ordered scan scheduler, catches panics from `work`, but invokes `deliver` at line 176 without a cancellation guard. [Window::stop](../../cli/src/scan.rs:239), which sets the stop flag and wakes waiting workers, is called only at line 186 after the loop. If delivery panics, that call is skipped. Dropping the result receiver does not wake a worker blocked in `Window::admit`'s condition-variable wait, and `thread::scope` waits for all those workers before finishing the unwind. The process can hang instead of exposing the original panic.

Reproduction used the exact scheduler definitions copied from `scan.rs`, compiled with `rustc --edition=2024`. A two-thread shim supplied only `rayon::current_num_threads()`; the scheduler, window, and flush bodies were unchanged. The probe supplied 200 paths, immediate successful work, and this delivery callback:

```rust
let _ = in_file_order::<_, ()>(
    &files,
    |_| (),
    |_, _, _| panic!("delivery panic"),
);
```

**Expected:** workers are canceled and joined, then the panic escapes promptly. **Observed:** the panic message appeared, but the process remained alive and had to be killed after three seconds. This is an internal scheduler reproduction, not a claim that a particular ordinary CLI input presently triggers a renderer panic. Severity is medium because it concerns unwinding robustness. The spec explicitly requires workers to wake and join on all exits.

| Child ID | Exit/site | Shape tested | Observed result | Expected result | Verification evidence |
| --- | --- | --- | --- | --- | --- |
| R1-F3.01 | Delivery callback panic | Two workers, 200 jobs, panic on first delivery | Process hung | Stop admission, wake/join workers, propagate panic | Exact-body extracted scheduler probe |
| R1-F3.02 | Panic from a delivery-side writer/renderer | Write or formatting call unwinds inside `deliver` | Same unguarded unwind path | Same cleanup | Precise invariant: every call inside `deliver` exits through line 176; none reaches line 186 on unwind |
| R1-F3.03 | Thread creation failure after some workers start | `scope.spawn` unwinds | Same missing cleanup guard | Wake already-started workers before scope joins | Precise invariant: the spawning loop is inside the same scope, before unconditional cleanup; no guard exists |
| R1-F3.04 | Worker/job panic | First and later panicking jobs | Correctly delivered in order and propagated | Cancel/join workers before propagation | Existing `a_panicking_job_is_propagated_in_file_order` passed; it reaches `stop` before `resume_unwind` |
| R1-F3.05 | Returned write failure / `Break` | First requested flush fails | Correct cancellation within one window | Stop, wake, join, return failure | Existing `a_failed_write_stops_within_one_window` and CLI pipe tests passed |
| R1-F3.06 | Empty input and normal completion | Zero jobs; ordered jobs at several worker counts | Correct completion | No spurious callback, orphan worker, or missing result | Existing empty-input and ordered-delivery tests passed |

All consumers share this scheduler: declaration exports, JSON, JSON Lines, tally, listings/classes, tables, outlines; imports/lint; and reference candidate collection. None installs its own unwind cancellation guard. Each therefore inherits R1-F3.01–03 when its delivery or setup unwinds. The separate library parent-index scope does restore its state on panic through a `Drop` guard; its nested-tree and panic-restoration tests passed.

**Repair:** install cancellation cleanup before starting workers, inside the scope where it is dropped before scoped joins. It must stop the window and wake waiters during unwinding as well as on explicit breaks. Preserve the worker-panic ordering already tested. Add a deadline-bounded subprocess regression that panics in delivery and confirms exit instead of hanging; keep it in the existing Level 1 binary target.

## Requirement coverage and recorded departures

The plan deliberately distinguishes mandatory algorithmic work, measured conditional work, and changes requiring an author choice. Unapproved optional policies are not silently treated as implemented, and are not blocking code findings here. The pending closure choice is recorded in frontmatter.

| Requirement | Implementation and strongest verification | Review outcome |
| --- | --- | --- |
| Remove quadratic cache membership and summary-order checks | Indexed memberships, normalized name keys, monotonic containment cursor; Level 1 corruption, normalized-identity, ordering, projection and cache tests | Implemented; no acceptance/refusal regression found |
| Share source-line geometry and avoid cloning import records for inventory hashing | One line-start index, borrowed stored-import view; Level 1 digest pins and import geometry checks | Implemented; hashing still materializes JSON text, which is a remaining low-cost optimization |
| One definition-query execution supplies declarations/imports/exports/diagnostics | Shared definition capture and import nodes; Level 1 direct-import equality, query counters, fixtures, lint and cache checks | Implemented |
| Constant-time indexed parent access and equivalent scope/export decisions | Query-scoped parent index, node-identity context sets; Level 1 fixture-wide parent equality, anonymous/aliased/missing/error nodes, generated-file fallback counter, nested scope and panic restoration | Implemented for the staged query path |
| Share Rust pattern placement and relation walks | Retargeted placed bindings and keyed shared lexical facts; Level 1 pattern identity, owner identity, relations, work counters and shared-vs-walked evidence | Implemented; no incorrect extra bindings or missing proven bindings found |
| Skip unused widened-member record facets while preserving retained type/member data | Original-kind selection distinct from completion kinds; Level 1 field/variant lists and narrow/full type-record comparisons | Implemented; recorded overflow-error change for unselected member generics is consistent with skipping that unused work |
| Stable record keys, IDs, candidate order and first-wins behavior | Final-key constructor, one normalized path, sorted name buckets; Level 1 literal ID pins, repeated declaration ordinals, candidate capture permutations and cache checks | Implemented |
| Shared lint registry, one policy application, linear source-context/dead-code scans, safe ignore prefilter | Level 1 policy idempotence, registry equivalence, LF/CRLF/no-final-newline/Unicode line cases, diagnostics and ignore controls | Implemented |
| Worker-side per-file rendering and JSON projection | Shared renderer/capability snapshot; Level 1 text/export/JSON goldens and deterministic ordering, plus Level 2 actual tmux/WezTerm rendering and links | Implemented for terminal/Prose per-file units and JSON; Markdown and cross-file units intentionally remain on delivery |
| Bounded ordered scheduling, early broken-pipe stop, no surviving workers | Level 1 order/slow-head/job-panic/write-failure tests and shipped mixed-failure probes | **Incomplete: R1-F1 and R1-F3** |
| Two-phase references, ambiguity before output, exact-byte reuse, no per-match index clone | Level 1 local/qualified/ID/ambiguous/missing/partial-error reports and per-file cache coverage; extra ownership probes | Functional selection preserved; **allocation contract incomplete: R1-F2** |
| Run-scoped package-root memo, enum-keyed statistics, owned prelude reuse | Level 1 grouping, counts, map spelling/order, prelude filters and exports | Implemented; scan-root and test-file manifest memoization explicitly deferred after small measured cost |
| God-file read/count/parse consistency and conservative byte-size screen | Analysis count/parse share bytes and borrowed source; Level 1 threshold, CRLF/final-line/non-UTF-8 and file-identity tests; Level 2 report checks | Implemented within analysis; discovery still performs its separate read |
| Scanner identity reuse and conservative literal-path pruning | Native identity retained; walker still applies overrides/ignores; Level 1 explicit/glob/alias/cache-key equivalence, literal-vs-bracketed-walker matrix, changed-file ownership cases | Implemented; direct `--file`, forced-language files, and ordinary patterns remain distinct |
| Optional sibling/doc caches, bind hash/index changes, counters, parser reuse and smaller relation cleanups | Implementation log records profiling results and conditional deferrals | No new public result contract assumed; defer unmeasured complexity |
| Cache precomputation/storage/scope policies, candidate-output changes, linter batches, terminal discovery, cache clear, allocator/profile | Spikes and explicit deferrals; existing contracts retained | Await author scope choice; these were not authorized by an unanswered question |

The public direct library methods `symbols`, `symbol_records`, imports/exports/locals, and direct lint/reference helpers still run outside the query-installed parent index and use its documented fallback. Public-API probes on the generated 40/120-entry fixture returned the expected declaration counts (47/127); those small samples did not establish a worthwhile timing gain from widening the index scope. This is a nonblocking follow-up, not evidence that those methods gained the staged query's optimization.

## Input robustness

The changed persistent-cache reader validates the existing JSON format; no manifest/configuration parser or new storage format was added. Its load-bearing entry, record, import, occurrence, coverage, provenance, and summary fields retain their existing shape contracts. The borrowed import writer retains field order and optional values; digest pins guard the persisted witness.

The existing [cache robustness tables](../../docs/cache-design.md:411) state outcomes across absent, explicit null, whole-field wrong type, one/every wrong element, empty, duplicate-key, and trailing-content shapes. The [CLI robustness matrix](../../cli/tests/symbols/cache_robustness.rs:1) starts from entries produced by the shipped CLI, includes positive controls, edits one cell at a time, and proves serving/recomputation through public output and a sentinel. Its field/nested-member/geometry/evidence/list-order tests passed in this review. Optional-null and genuinely empty controls retain their documented outcomes; populated import inventory, named-owner evidence, covered IDs, source paths and edge summaries continue to be checked before projection.

The changed containment cursor was also checked against exhaustive span containment, including touching/overlapping spans; it does not merge two spans into a false witness. Normalized-name membership uses the same key function as prior equality. No new robustness finding was identified. The invalid-UTF-8 fixtures in R1-F1 test source-read failure delivery, not a newly permissive cache parser.

## Nonblocking recommendations

- Keep the remaining policy experiments as separately approved changes. Their documented estimates are not established production speedups, and restoring full cache validation is already the implemented baseline.
- Describe god-file I/O as two reads for a retained candidate across discovery plus analysis, with one shared read for count/parse within analysis. The implementation improves the previous three-read path, but does not make the whole command a single-read operation. Consider a fused path only if its benefit outweighs retaining large source snapshots.
- A future parent-index extension to direct library APIs should use actual large-file measurements and preserve nested calls and tree lifetimes. The staged-query performance proof currently does not cover those entry points.

The repair inventory is exactly R1-F1, R1-F2, and R1-F3. Passing existing gates does not close them; each needs the focused regression described in its repair section, followed by the affected package checks. Human review is needed only to settle closure of the deferred design work, and does not block these repairs.
