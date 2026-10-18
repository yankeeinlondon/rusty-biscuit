---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-18T07:06:46-07:00"
spec: 2026-10-17-dmls-index-store/spec.md
implemented: false
description: "A **feature** review of `2026-10-17-dmls-index-store/spec.md`"
feature: 2026-10-17-dmls-index-store/review-3.md
previous: 2026-10-17-dmls-index-store/review-2.md
---

# DMLS index store review, iteration 3

The feature is **production ready** within the specification's scope. Both previous high findings are verified fixed, including every instance in the repair's Finding Ledger. No new findings or observations were identified.

I reviewed the specification, plan, earlier reviews, original implementation log, repair ledger, current store and request implementation, and relevant tests. The accepted decision to perform individual requests' store queries and temporary graph construction on the message-handling thread remains accepted; this review requires no further human decision.

## Previous Finding Verification

Evidence below refers to package `dmls`. All named tests ran successfully in this review's `just test` run. Source inspection independently checked the repair mechanism; ledger completion flags and reported mutation tests were not treated as proof.

The key implementation is [WorkspaceFacts::verified_view](../../dmls/src/index_store/workspace.rs): it checks every listed file against the row hashes from the same database snapshot used to select candidates and read facts or headings. A missing or mismatching row causes a fresh disk read and parse, which replaces that row's contribution to the current answer. [StoreFacts::view](../../dmls/src/index_store/source.rs) encloses these reads in a transaction. Consequently, an older foreign row cannot silently remove a candidate, and an empty fallback map cannot become a successful incomplete answer.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R2-F1 — shared rows can change after validation and silently change answers | R2-F1.1–R2-F1.10, each checked below | verified fixed | Consistent per-root reads plus fresh facts for every unverified listed path; late-writer tests compare all request families with the full-graph oracle. | Nothing. |
| R2-F1.1 — readiness and candidate completeness | A late foreign wiki row removes a current link | verified fixed | `a_late_older_wiki_row_from_another_writer_changes_no_answer` checks that readiness remains usable and the wiki referrer remains a candidate after the foreign commit. Candidate absence is checked against all listed hashes, before narrowing. | Nothing. |
| R2-F1.2 — document loading | Foreign facts differ from this process's verified hash | verified fixed | The same test publishes the older row again before `request_documents`, then checks the returned hash and current heading title. | Nothing. |
| R2-F1.3 — heading scans and symbols | A foreign row replaces current headings | verified fixed | Both late-row tests compare workspace-symbol results with the oracle. The headings and hashes are read in one transaction; unverified headings are replaced. | Nothing. |
| R2-F1.4 — ordinary and wiki references | Heading, Markdown-link, and wiki probes after a late commit | verified fixed | `check_every_request_after_a_late_row` checks every corpus probe, with and without declarations, against the oracle. A foreign row is published before each graph query. | Nothing. |
| R2-F1.5 — transclusion references | An older composing file loses its `::file` reference | verified fixed | `a_late_older_transcluding_row_from_another_writer_changes_no_answer` checks candidacy and a nonempty transcluded-by answer containing the closed composing file, then all request comparisons. | Nothing. |
| R2-F1.6 — heading rename | A foreign row removes the wiki heading referrer | verified fixed | The wiki late-row test checks the oracle edits the wiki file and compares heading rename after another commit. The router's pre-rename refresh also calls the checked candidate reader. | Nothing. |
| R2-F1.7 — file rename | A foreign row removes a wiki spelling edit | verified fixed | The wiki late-row test checks that the oracle edits the wiki file, compares that rename, and exercises every corpus file rename, including collisions and changed directories. | Nothing. |
| R2-F1.8 — revalidation and refresh | Mismatch before a pass, and a foreign commit after a successful refresh | verified fixed | `a_row_committed_after_a_refresh_checked_it_changes_no_answer` verifies one reparse for the mismatch, zero for the next unchanged refresh, then all answers after late commits. Existing restart and refresh tests pass. | Nothing. |
| R2-F1.9 — open buffers | Saved and unsaved buffers replace disk rows without writing them | verified fixed | `open_buffers_replace_rows_and_never_write_one` passes. Document, graph, and symbol readers apply the buffer overlay after verification. | Nothing. |
| R2-F1.10 — memory backend | Process-local facts, including a salvaged foreign row | verified fixed | `MemoryFacts` owns its map; `a_salvaged_older_row_in_memory_decides_no_answer` checks candidates, document headings, and symbols after salvage of an older foreign row. | Nothing. |
| R2-F2 — a cache failure during a request returns an incomplete answer | R2-F2.1–R2-F2.11, including the two ledger additions | verified fixed | First-reader API tests and six server-session request tests verify the answer that encounters the failure, subsequent revalidation, and later answers. | Nothing. |
| R2-F2.1 — candidate lookup failure | Undecodable stored candidate path after ready | verified fixed | `an_undecodable_candidate_row_after_ready_leaves_the_first_candidates_complete` checks the first set equals its positive control and contains all oracle referrers; readiness is withdrawn. | Nothing. |
| R2-F2.2 — document loading failure | Undecodable heading after ready | verified fixed | `undecodable_headings_after_ready_leave_the_first_request_documents_complete` checks every requested file, hash, and heading against disk. The corresponding first-request-graph test checks references and all later request families. | Nothing. |
| R2-F2.3 — heading-scan failure | Symbol query is the first reader of damaged headings | verified fixed | `undecodable_headings_after_ready_leave_the_first_workspace_symbols_complete` compares the first scan with the oracle for every corpus query. | Nothing. |
| R2-F2.4 — references handlers | Ordinary heading, wiki, and DSL references encounter damage first | verified fixed | The three `after_ready_damage_the_first_*_references_equal_memory` server-session tests compare each first answer against a memory-backed control, for both path and heading damage. Controls require a closed referrer in the answer. | Nothing. |
| R2-F2.5 — heading rename and pre-query refresh | Damage encountered while refreshing or building the graph | verified fixed | `after_ready_damage_the_first_heading_rename_equals_memory` and `undecodable_headings_first_read_by_a_rename_refresh_leave_the_renames_complete` verify complete first edits. | Nothing. |
| R2-F2.6 — file rename and pre-query refresh | Same failure during file rename, including wiki edits | verified fixed | `after_ready_damage_the_first_file_rename_equals_memory` checks the first protocol answer; the facts-level refresh test also compares file edits with the full-graph oracle and requires the wiki referrer. | Nothing. |
| R2-F2.7 — workspace-symbol handler | Damage encountered by the first symbol search | verified fixed | `after_ready_damage_the_first_workspace_symbols_equal_memory` checks first and later protocol answers. All six session tests assert that the next request rereads all six files and the following request rereads none. | Nothing. |
| R2-F2.8 — build-time reconciliation | Failure before releasing waiting requests | verified fixed | `a_store_failing_during_the_build_gets_a_full_round_before_any_answer` checks two full read rounds and complete answers. `Router::finish_index_round` selects a full pass after a backend switch, before marking ready. | Nothing. |
| R2-F2.9 — successful salvage and open-time fallback | Write-only failure; corrupt, disabled, or unavailable store | verified fixed | `a_readable_store_that_fails_a_write_is_salvaged_into_memory`, source fault tests, and off/read-only/corrupt-held-store oracle tests pass. Newly computed facts survive write failure. | Nothing. |
| R2-F2.10 — salvage documentation | Comments and docs incorrectly promised preservation of every individually readable row | verified fixed | The fallback table and `fall_back` documentation in `source.rs`, module documentation in `workspace.rs`, and “When a store cannot be used” in `docs/lsp/indexing.md` now describe all-or-none salvage and fresh reads for missing facts. | Nothing. |
| R2-F2.11 — additional consumers and defaults | Change/save notifications, prepare-rename, memory view, document checks, old root readers | verified fixed | Notifications produce no client answer and withdraw readiness on a switch; prepare-rename uses the live snapshot. The infallible memory view cannot produce a decode failure. Production graph queries check every listed file; direct document reads check every requested path. Old root candidate/heading readers are not used by production request handlers. | Nothing. |

No earlier code finding remains open. Review 1 assigned no code findings; its request-thread decision was accepted by the author and verified in review 2. It is not reopened here.

The repairs are complete repairs of R2-F1 and R2-F2. The additional ledger rows concern corrected salvage documentation and checked sibling consumers; they introduce no remaining defect or newly discovered sibling finding.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

None. The earlier defect classes are repaired, and this review identifies no repeated or new finding.

## Verification and requirement coverage

I used the darkmatter and rust-testing skills and consulted the just, biscuit-file, and biscuit-hash guidance for the relevant commands and helpers. This feature changes persistence, filesystem processing, and language-server responses. Level 1 public-API and protocol tests are appropriate; it adds no terminal rendering, keyboard, mouse, or browser behavior requiring Level 2 or Level 3 evidence.

| Specification behavior | Verification level and evidence | Result |
| --- | --- | --- |
| Unchanged restart parses zero; one changed file parses once | Level 1 workspace and live server-session parse counters; current repository benchmark | Pass; benchmark reread 5,712 files and reparsed zero. |
| Never-open referrers participate in heading and file rename after restart | Level 1 `index_store_sessions` restart tests | Pass. |
| References, DSL and wiki projections, heading/file rename, and workspace symbols equal the full graph | Level 1 `index_store_requests` oracle comparisons; `wiki_universe_equivalence`; first-answer recovery sessions | Pass, including wiki ambiguity and rename spellings. |
| Candidate sets contain every true referrer, including bare repository-root paths and unusual forms | Level 1 `index_store_keys` and request corpus tests | Pass. Existing explicit-relative and broad-name controls preserve the narrowing contract. |
| Multiple and nested roots preserve cross-root results and remove duplicates | Level 1 request corpus and `index_store_sessions` | Pass. Source inspection also confirms any owning root's mismatching row causes fresh facts to replace that path's merged contribution. |
| Open buffers override rows; events and saves update the prescribed files; rename reads current referrers | Level 1 workspace and `index_store_live` tests, including unannounced fallback-mode changes | Pass. |
| Off, read-only, corrupt, locked, or failed stores still produce complete requests from memory | Level 1 source fault injection, public-API oracle comparisons, live protocol sessions, and compiled-binary stdio tests | Pass, including the first request after readiness-time failure. |
| Shared cold fills converge; live stores are not swept or recreated | Level 1 `index_store_concurrency`, `index_store_sweep`, source/open tests; read-transaction isolation test | Pass on this host. Other operating-system results remain CI's responsibility. |
| Stored facts round-trip; extraction changes require a version bump | Level 1 repeated persistence round trips and golden snapshot test in `index_store_facts` | Pass. No extraction or version semantics changed in the repair. |
| Cache location, permissions, root identity, versioned files, and logs | Level 1 open/location/log tests and biscuit-file cache-directory tests | Pass on this host; injected cache-platform cases cover each convention. |
| Warm latency and resident-memory limits | Release `dmls --bench-index` over the current repository | Pass; current measurements below. |
| Full production in-memory build removed; full graph retained only as a test oracle | Source inspection and Level 1 `full_index_removal_guard` | Pass. |

Executed on this macOS host:

- `just test` in `darkmatter/`: **10,471 passed, 10 skipped**, 36.250 seconds. No `SLOW`, `FAIL`, or `TIMEOUT` line. The run includes the repaired store tests and the surrounding library, CLI, language-server, and extension-helper Level 1 suites.
- `just lint` in `darkmatter/`: **passed**, including the Zed extension's `wasm32-wasip2` check.
- `just check-tier-coverage darkmatter`: **zero stranded tests**. Store integration modules are declared by `tests/l1/main.rs`; their names select Level 1, and the area's recipe enables `effects-instrumentation` for DMLS.
- `just test user_cache_dir` in `biscuit-file/`: **15 cache-directory tests passed** through nextest; the recipe's additional minimal-feature path checks also passed.
- `cargo build --color=never --release -p dmls`, followed by `cargo run --color=never --release -p dmls -- --bench-index /Volumes/coding/wt/rusty-biscuit/feat-dmls-indexing --json`: **passed**.

The release benchmark covered **5,712 Markdown files** and used a fresh child process for the warm half. It measured the repaired implementation, including the per-request row-hash scan:

| Measurement | Current result | Specification limit |
| --- | --- | --- |
| Warm first references, excluding file-list walk | 142.85 ms | 500 ms |
| Warm first references, including walk | 309.08 ms | 1,000 ms |
| Added peak resident memory while serving references, rename, and symbols | 38.39 MiB | 64 MiB |
| Warm revalidation | 66.80 ms; 5,712 unchanged, zero reparsed | Zero reparses for unchanged files |
| Cold fill | 4,415.60 ms; all 5,712 parsed; 622.55 MiB peak RSS | Recorded only; no target |

This is one current-host sample, not a cross-OS performance claim or a cold-disk measurement. The default benchmark target was the repository README: 29 candidates, zero incoming locations at its first heading. Nonempty reference and rename answers are independently verified by the corpus and protocol tests.

No production code or tests were changed for this review. Existing deterministic regression tests reproduce the earlier inputs through public APIs and protocol sessions, so no temporary implementation probes were necessary.

## Input Robustness Matrix

The feature adds a SQLite fact reader and an environment-based cache-location choice; it does not add a JSON, YAML, TOML, or JSONL configuration loader. A structurally invalid store or undecodable fact produces a typed store failure, followed by the specified recreation-or-memory recovery. If the failure occurs during a request, the request obtains fresh facts for missing or unverified listed files before producing its answer.

SQLite `STRICT` tables and `NOT NULL` enforce required field types and nullability. Primary keys prevent duplicate identities. Missing fact children represent valid empty collections, including empty documents. Optional wiki heading/alias values intentionally use SQL null for absence; paired heading-span endpoints are validated together. Shared numeric decoders reject out-of-range heading levels, negative positions and lines, inverted spans, invalid boolean flags, and half-present optional spans.

Public `IndexStore` tests include positive controls, empty facts and titles, missing tables, incompatible stamps and roots, wrong SQL types, invalid UTF-8, and one invalid decoded value in each fact table. The first-answer tests additionally check candidate, document, and heading readers after readiness, then follow those results through references, both renames, and workspace symbols. SQLite's document boundary is exercised with garbage and truncated database files; text-format trailing-content and multi-document rules do not apply to this database reader.

The requested smell search found no load-bearing `serde(default)` or parse-error suppression. `MemoryFacts::get_all` filters absent map entries by contract; request verification now detects missing rows before they can silently omit listed files. `RootFacts::view` applies `unwrap_or_default` only to the process-local `MemoryFacts` implementation, whose methods return no errors. Environment tests cover unset, empty, disabled, relative, file, and directory overrides and the injected platform-specific cache rules. No new arbitrary-corruption protection layer is warranted.

## Observations

None.
