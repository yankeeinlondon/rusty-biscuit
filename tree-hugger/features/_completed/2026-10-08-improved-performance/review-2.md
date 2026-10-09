---
$schema: feature-review.yaml
ready: true
findings: []
human_review: true
human_review_items:
  - |-
    Choose how to close the deferred design work. The implemented improvements preserve existing cache policy and public output. Changes to cache storage, advance computation of test/deprecation flags, scope coverage, reference-candidate output, external-linter batching, terminal discovery, cache cleanup, and the release allocator/profile remain deferred.

    - Close this feature around the verified improvements and schedule separately approved follow-ups; recommended.
    - Choose which deferred changes to include and schedule another implementation and review before those changes ship.

    This is the same scope choice recorded in the first review. It does not block the repaired implementation, and the review does not request reaffirmation of an accepted decision.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-09T11:33:47-07:00"
spec: 2026-10-08-improved-performance/spec.md
implemented: false
description: "A **feature** review of `2026-10-08-improved-performance/spec.md`"
feature: 2026-10-08-improved-performance/review-2.md
previous: 2026-10-08-improved-performance/review-1.md
---

# Review 2: Improved performance

The implemented feature is **ready for production**. All three previous code findings are verified fixed across their required instances and consumers. No new blocking defect, repair regression, or missed sibling was found. The existing human scope choice about deferred policy changes remains separate from code readiness.

Reviewed the specification, plan, original implementation log, first review, repair log, and the current implementation, including the uncommitted repair diff. The plan is the supplied design document. `review-1.md` is the only earlier implementation review in this directory. Its frontmatter, three finding bodies, 25 child rows, required repairs, and the repair ledger agree on the blocking inventory. The ledger's additional human-review and recommendation IDs are reconciled below; they were never blocking code findings.

Only this review, the previous review's `next` property, and the spec's review/completion properties were edited. Existing implementation changes and implementer claims were preserved.

## Previous Finding Verification

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
| --- | --- | --- | --- | --- |
| R1-F1 — Failed files bypass flushes and defeat early pipe cancellation | R1-F1.01–12: imports/lint native failures, lint adapter failures, every streaming declaration writer, and collecting report modes | verified fixed | Common write/flush path in tree-hugger-cli [main.rs](../../cli/src/main.rs:1157); all scheduler callbacks inspected; mixed-failure closed-pipe tests and positive controls pass. Per-child checks below. | None. No design approval or prerequisite blocked this repair. |
| R1-F2 — Reference selection duplicates whole-file data for every match | R1-F2.01–07: analyzed index, source snapshot, initial worker snapshot, ID/missing/directional selectors, and prelude reuse | verified fixed | tree-hugger-cli [Selection](../../cli/src/refs.rs:196) stores one state per matching file and lightweight candidates. Public source-retention probe checks allocation identity and exact bytes across every cache branch. Selector controls pass. | None. Public candidate-list policy remains deferred and was not needed to repair ownership. |
| R1-F3 — Delivery panics leave scan workers waiting forever | R1-F3.01–06: delivery/writer/setup unwind, job panic, returned write failure, and empty/normal completion; every ordered-scan consumer | verified fixed | tree-hugger-cli [in_file_order](../../cli/src/scan.rs:139) installs `StopOnDrop` before the first spawn, inside the scoped closure. Deadline-bounded panic regressions and existing exit/order tests pass. | None. No prerequisite or human decision blocked this repair. |
| review-1.md — closure of deferred design work (repair ledger R1-H1) | Human choice about optional policy changes | unresolved | First review explicitly made this external scope choice nonblocking; spec and plan still record deferrals, with no author approval claimed. | Human scope choice carried above; not an unresolved required code repair. |
| review-1.md — keep policy experiments separate (repair ledger R1-N1) | Optional policy follow-ups | unresolved | Existing policies are preserved; no new policy implementation is claimed. | Nonblocking recommendation remains. |
| review-1.md — god-file I/O wording (repair ledger R1-N2) | Distinguish discovery read from shared analysis read | unresolved | Original implementation log and first review distinguish two reads across discovery and analysis. The three-finding repair did not fuse those stages. | Nonblocking documentation recommendation remains; do not claim a single read across the entire command. |
| review-1.md — extend parent indexing to direct APIs only after measurement (repair ledger R1-N3) | Direct library APIs outside the staged query | unresolved | Parent-index scope remains the staged query; fallback outside it is unchanged. | Nonblocking measured follow-up remains; no speedup for direct APIs is claimed. |

No earlier finding was blocked and subsequently unblocked. The accepted compatibility boundary—preserve public results and full cache validation—remained in place before and after repairs. The unanswered policy questions were neither approvals nor prerequisites for these fixes.

### R1-F1 consumer sweep

**Class checked:** every continuing per-file streaming delivery must honor its requested flush, including failed and empty files, before later work is admitted.

The reproduction copies the Rust fixture into a slow successful first file, followed by 120 copies with one invalid UTF-8 byte. With two workers and an eight-file window, closing stdout must produce exit 0 and fewer than eight delivered failures. An open reader must report all 120. The adapter variant uses valid TypeScript files and a local malformed-output oxlint stub; its open-reader control must invoke and fail all 120 files, while closed stdout must stop before eight invocations. These expected results come from the window contract, not cached/uncached agreement.

| Child ID | Site and shape tested | Observed result | Expected result and verification evidence |
| --- | --- | --- | --- |
| R1-F1.01 | Imports native failures; default/plain; closed stdout | Exit 0, fewer than eight errors | Cancellation within the first window; `report::failed_files_still_stop_imports_and_lint_at_a_failing_stdout` passes. Both failure branches now reach `flush.apply`. |
| R1-F1.02 | Lint native failures; default/plain; closed stdout | Same bound and status | Same test, with an explicitly enabled diagnostic ensuring a nonempty head report. |
| R1-F1.03 | Lint adapter failures; default/plain; closed stdout | Fewer than eight errors and adapter runs; open-reader control gives 120 of each | `report::adapter_failures_still_stop_lint_at_a_failing_stdout` passes on this Unix host; failed adapter settlement reaches the common flush. |
| R1-F1.04 | Functions/types/symbols listing error settlement | Functions reproduction passes; shared writer always flushes after settlement | [write_listing](../../cli/src/declarations.rs:685) uses the same unconditional post-settlement flush for all three commands; command choice cannot bypass it. |
| R1-F1.05 | Classes listing error settlement | Closed-pipe reproduction passes | Separate classes branch in `write_listing` also flushes outside successful settlement. |
| R1-F1.06 | Ordinary and complete-index JSON failed deliveries | Both closed-pipe cases pass | [write_json](../../cli/src/declarations.rs:471) flushes for successful, empty, and failed deliveries; an entry serialization/write error breaks immediately. |
| R1-F1.07 | JSON Lines failed/no-record delivery | Closed-pipe case passes | [write_json_lines](../../cli/src/declarations.rs:586) applies the flush after settlement even when no line is written. |
| R1-F1.08 | Table failed-file delivery | Closed-pipe case passes | [write_table](../../cli/src/declarations.rs:768) flushes outside successful settlement. |
| R1-F1.09 | Outline failed-file delivery | Closed-pipe case passes | [write_tree](../../cli/src/declarations.rs:809) applies the common flush after settlement. |
| R1-F1.10 | TSV and ctags failed-file delivery | Both closed-pipe cases pass | [write_export](../../cli/src/declarations.rs:343) flushes after settlement; immediate write errors return `Break`. |
| R1-F1.11 | Markdown and Prose listing delivery | Both closed-pipe cases pass | Shared listing callback retains its flush independently of the layout and worker-rendering choice. |
| R1-F1.12 | Count/statistics, refs, imports/lint JSON; partial failure | Existing totals, ambiguity, empty reports and error/status checks pass | Counts and reference selection explicitly acknowledge `nothing_written`; imports/lint JSON accumulate into an envelope with an empty output buffer during scanning. These modes intentionally complete selection before output and promise no streaming cancellation. |

All production `in_file_order` calls were inspected, including both listing branches. `Flush` is consumed to produce `Flushed`; `nothing_written` remains an explicit escape for collecting modes, so the type alone is not proof that arbitrary callers flush. The actual streaming callers all use the writer path. No remaining production callback acknowledges an unwritten flush after buffered streaming output. Final broken-pipe/other-write-error status mapping remains unchanged.

### R1-F2 ownership and selector sweep

**Class checked:** declaration matches must not each own a whole-file index or byte snapshot; all selection and downstream reporting decisions must still use the original identities and bytes.

| Child ID | Site and shape tested | Observed result | Expected result and verification evidence |
| --- | --- | --- | --- |
| R1-F2.01 | One-file analyzed state; 40 same-named methods | 40 candidates share one retained analysis | `refs::tests::many_same_named_matches_in_one_file_share_its_one_analysis` passes; `Phased` has no `Clone`, and the index is moved once. |
| R1-F2.02 | Multi-file read state; two matching files and one unrelated file | 80 candidates, two retained snapshots, exact source bytes | `only_matching_files_retain_one_byte_snapshot_each` passes; no per-candidate source clone. |
| R1-F2.03 | Initial worker source retention; matching and nonmatching input; all cache outcomes | Source bytes move through analysis; a nonmatching result retains no file state | Library retention regression passes. Independent public API probe preserves the source buffer pointer on miss, hit, kind widening, parsed capability widening, and parse-free capability widening. An on-disk replacement after reading changes neither the retained bytes nor the original `probe` declaration. |
| R1-F2.04 | Unique ID; one/multiple files | Expected declaration and file selected | Unit `an_id_selects_exactly_one_retained_file`, CLI ID/callee controls, and independent fixture-copy ID controls pass. `into_unique` moves the matching file; evidence lookup selects the same ID. |
| R1-F2.05 | Missing name; one/multiple files | Exit 0, null declaration, empty files/errors | New CLI controls and independent `absent` probes pass; no matching state is invented. |
| R1-F2.06 | Ambiguity; all directions, text/JSON, uncached/cold/warm | Exact count, deterministic locations/IDs/order, exit 1 and empty stdout | `refs::many_same_named_methods_in_one_file_stay_one_exact_ambiguity` passes. Independent copy of the Python fixture plus 80 methods and a 1 MB comment gives exactly 80 matches in all 12 one/multiple-file × direction × format probes; qualified-name controls select exactly `sample.py#C17.probe`. |
| R1-F2.07 | Prelude reuse and neighboring whole-file ownership sites | Owned reuse entry removed; no per-match index clone | [declarations.rs](../../cli/src/declarations.rs:241) consumes the reuse map after synthetic export preparation. Prelude tests pass. Other remaining copies are path keys, one selected record, or one synthetic-export vector per reported prelude, not whole-file state per match. |

Tracing includes native file sorting/deduplication, all-kind/all-scope declaration analysis, smart-case/name/qualified/ID filtering, candidate collection, ambiguity before stdout, unique selection, evidence lookup, direction filtering, JSON projection and rendering. Candidate rows retain declaration positions, labels, qualified names and IDs. The loop is shared by all languages; there is no language-specific candidate-ownership branch left unchecked. Existing escaped/normalized/local/partial-error tests pass.

The new tree-hugger library APIs [SourceFile::query_retaining](../../lib/src/file/source_file.rs:230) and [SymbolCache::query_retaining](../../lib/src/cache/symbol_cache.rs:336) give back the exact source snapshot used for analysis. Language, forced-language choice, experimental mode, path and hash are preserved. Ordinary cached queries delegate to this same implementation, so the existing cache/projection/corruption tests also exercise the changed path. Returning retained source does not cause another read or hash.

### R1-F3 exit sweep

**Class checked:** every coordinator exit must stop admission and wake waiting workers before scoped joins, including unwinding.

| Child ID | Exit/site and shape tested | Observed result | Expected result and verification evidence |
| --- | --- | --- | --- |
| R1-F3.01 | Delivery panic on first and middle result, 200 jobs, one/two/eight workers | Original panic propagates within deadline; expected preceding deliveries; admission stops | `scan::tests::a_panicking_delivery_is_propagated_without_waiting_on_workers` passes. Guard drops inside the scope closure before its workers are joined. |
| R1-F3.02 | Writer panic inside delivery | Original writer panic propagates; no continuing work after unwind | `a_writer_panicking_inside_a_delivery_is_propagated` passes; renderer panics traverse the identical guarded delivery call. |
| R1-F3.03 | Thread creation unwind after earlier workers start | Cleanup is structurally guaranteed; actual OS spawn failure not forced | `_stop` is bound before the spawn loop. Any `scope.spawn` unwind drops it before leaving the closure; `Window::stop` sets the stop flag under the wait mutex and notifies all. This checks the precise required invariant without pretending to inject thread exhaustion. |
| R1-F3.04 | Worker/job panic, first/later job | Earlier files delivered in order, original panic propagated | `a_panicking_job_is_propagated_in_file_order` passes; caught job panic is resumed through the same guard. |
| R1-F3.05 | Returned failure or `Break` at first flush | Started work stays within one window, workers finish before return | `a_failed_write_stops_within_one_window` and the CLI closed-pipe tests pass; normal break drops the same guard. |
| R1-F3.06 | Empty input and ordinary completion at multiple worker counts | No empty-input callback; all normal results once and in order | `an_empty_input_delivers_nothing` and `every_file_is_delivered_once_in_file_order_with_any_worker_count` pass. |

Consumers are declaration exports, JSON, JSON Lines, count/statistics, listings/classes, tables, outlines, imports/lint, and reference selection. They all use the same guarded coordinator. The separate library parent-index scope has its own restoration guard and existing panic/nested-tree tests.

The first review requested a deadline-bounded subprocess test. The repair uses a deadline-bounded helper thread inside a nextest-isolated test process instead. A stuck scan fails at the ten-second receive deadline, and process termination removes that thread; passing scenarios catch the original panic only after scoped workers have joined. This fulfills the intended anti-hang verification without a re-execution harness. Fixed sleeps in these tests sample the absence of later work; scoped joining, not elapsed sleep, establishes the lifetime guarantee.

## Verification and requirement coverage

Checks were run against the current working tree, not taken from the implementer's log.

| Check | Result | Requirement and strongest appropriate level |
| --- | --- | --- |
| `just test` in `tree-hugger/` | 2,074 passed, zero skipped (154.448 s) | Level 1: cache acceptance/refusal and projections; stable IDs, spans, ordering and selectors; selective/shared analysis work; scanner identity/literal selection; prelude and god-file results; all new repair regressions. |
| `just lint` in `tree-hugger/` | Passed for both packages | Rust lint checks for the current implementation. |
| `just check-tier-coverage tree-hugger` | Zero stranded tests | New tests are in running tiers. CLI `symbols` declares `refs`/`report`; library `l1` declares `symbols_cache`; scheduler/selection unit tests compile in the `hug` binary. None has a higher-tier marker or an unmet feature gate. |
| `just test-l2` in `tree-hugger/` | Seven entries passed (28.605 s); actual tmux and WezTerm rendering verified | Level 2: actual tmux/WezTerm styled declarations, tables, outlines, statistics, links/wrapping, references and god-file reports, through the live no-focus harness recipe. |
| Independent shipped-CLI probe | Passed all 18 reports | Level 1: exact 80-candidate ambiguity and empty stdout in 12 reports; two missing, two qualified-name, and two ID controls with a 1 MB source comment. Scratch-local HOME/config/temp, two workers, `--no-cache`, fixture copied outside exclusions. |
| Independent public library probe | Passed all five cache outcomes and post-read replacement | Level 1: original byte allocation survives analysis and cache reuse/widening; on-disk replacement cannot substitute declarations or bytes. Rust probe linked against the current library build, in a temporary directory. |

The two Kitty entries are availability-gated; their passing entries are not claimed as actual Kitty rendering evidence. The headless tags-reader entry is a functional reader check, not terminal rendering. The 883 tests excluded by the Level 2 filter are not missing Level 1 coverage.

The original requirement mapping in the first review remains applicable: quadratic cache validation removal, one definition capture, query-scoped parent indexing, shared Rust facts, selective metadata, stable identity/candidate ordering, lint/diagnostic work, worker rendering, package-root/statistics/prelude ownership, god-file analysis consistency, and scanner identity/literal pruning all retain their covered contracts. This iteration independently rechecks the repaired scheduling and reference-ownership rows and runs the existing requirement tests. No new terminal-input behavior is introduced; Level 3 is not required. Cross-OS evidence is left to the normal CI process and is not a readiness blocker.

Measured conditional omissions and unapproved policy changes remain explicitly deferred in the plan and original implementation log. The implementation does not gain credit for compact cache storage, eager flags, scope-pair coverage, candidate truncation/schema changes, batched adapters, terminal-discovery redesign, eviction, or allocator/profile adoption. No new approval is inferred. The prior nonblocking caveats about discovery/analysis reads and direct library parent indexing remain.

## Input robustness

The repair adds source-retaining query APIs and changes scheduling/ownership, not a file-format parser, manifest/config loader, deserializer, or stored schema. Therefore it introduces no new load-bearing field matrix. The cache still calls its existing strict reader before deciding hit or widening, and stores through the existing writer; source reconstruction retains path/hash/text rather than interpreting a new representation.

The feature's earlier validation optimizations remain exercised by the existing [cache outcome tables](../../docs/cache-design.md:411) and [public CLI corruption matrix](../../cli/tests/symbols/cache_robustness.rs:1). They cover absent/null/wrong-type/empty/duplicate/trailing shapes and evidence geometry, ordering, provenance, import inventory and summaries. No permissive parsing change appears in the repair diff. The invalid-UTF-8 and malformed-adapter fixtures verify failure delivery; they do not authorize treating damaged input as empty.

## Unblocked Findings

None. There is no outstanding required repair from the previous review and no new blocking finding.

## Blocked Findings

None. The external scope choice about optional follow-ups is a human-review item, not a blocked defect in the repaired implementation.

## Recurrence

No recurring defect remains. R1-F1, R1-F2 and R1-F3 are verified fixed rather than carried as unchanged reproductions. No repair regression or newly discovered sibling in those classes was found, so `recurrence: false`.

## Nonblocking recommendations

Retain the first review's three follow-ups: keep policy experiments separately approved; describe god-file I/O accurately across discovery and analysis; measure any extension of parent indexing to direct library APIs before adding it. None is a newly required code repair or a claim that deferred work has shipped.
