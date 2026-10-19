---
$schema: feature-review.yaml
ready: false
findings:
    - "R2-F1 (high): Shared rows can change after validation and silently change answers"
    - "R2-F2 (high): A cache failure during a request returns an incomplete answer"
observations:
    - "The root test recipe fails before nextest starts"
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-18T06:37:58-07:00"
spec: 2026-10-17-dmls-index-store/spec.md
implemented: true
description: "A **feature** review of `2026-10-17-dmls-index-store/spec.md`"
feature: 2026-10-17-dmls-index-store/review-2.md
previous: 2026-10-17-dmls-index-store/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/feat-dmls-indexing/darkmatter/features/2026-10-17-dmls-index-store/log.md
implemented_by: claude/opus
next: 2026-10-17-dmls-index-store/review-3.md
---

# DMLS index store review, iteration 2

The feature is **not production ready**. Two high findings affect the completeness of references, rename edits, and workspace symbols. The ordinary restart, request-equivalence, and fallback-at-open tests pass; the defects occur after a process has reached ready.

This review covers the specification, plan, implementation log, previous review, repair ledger, store implementation, router consumers, external wiki universe, cache-directory helper, and corresponding tests. The accepted decision to answer individual requests on the dispatch thread remains accepted.

## Previous Finding Verification

The previous review explicitly assigned **no code findings**. Its repair ledger labels the author-decision item `R1-H1`; it does not turn that item into a code defect. The following row verifies that decision and its sole required child. The ledger's optional follow-ups (`R1-O1`) were observations, so they are not carried as findings.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-H1 — where per-request store work runs (decision item, not a code finding) | R1-H1.1: document dispatch-thread queries and temporary graph construction | verified fixed | The author decision in `review-1.md` accepts this implementation. `docs/lsp/indexing.md`, “Answering a request from the store,” describes it. In package `dmls`, `Router::needs_index_build` queues requests during a build; `router/facts.rs` performs request graph and symbol queries synchronously. The documentation's approximate timings agree with the recorded benchmark samples. | Nothing; no new approval or worker implementation is required. |

The two findings below are newly found defects, not partial repairs or siblings of a previous code finding. No earlier code findings remain open.

## Unblocked Findings

### R2-F1 (high): Shared rows can change after validation and silently change answers

**Authority: spec.** The multiple-writer requirement states: “A process uses another's row only when its hash equals its own read of the bytes: shared rows save parsing, never decide correctness.” The readiness requirement likewise says every listed row matches bytes read **in this process**. The current implementation checks hashes during a validation pass, then trusts subsequent shared-store queries without enforcing that condition.

**Defect class:** validation of mutable shared cache data expires before the decisions that consume it.

In package `dmls`, [WorkspaceFacts](../../dmls/src/index_store/workspace.rs) remembers verified hashes, but its request readers do not compare retrieved rows with them. Candidate queries and heading scans do not even return the hashes needed for that comparison. A second process can publish an older row after this process validates newer bytes. This is a normal interleaving: one editor starts a fill, a file is saved and another editor validates it, then the first editor commits its already-computed batch. SQLite makes each write atomic; it does not ensure the later writer read newer bytes.

**Reproduction through the public API.** Copy `tests/fixtures/index_store_corpus` with the existing `Corpus::materialize` helper. Fill `WorkspaceFacts` and mark it ready. Verify its answers against the corpus oracle. Open a second `IndexStore` for the same root and upsert the facts for an earlier `notes/wiki.md` containing only `# Older\n`, representing a batch computed before the current fixture bytes were saved. Leave disk unchanged. Query candidates, documents, symbols, references, heading rename, and file rename through the first instance. It still reports ready, but serves the foreign hash, loses the wiki referrer, and disagrees with the disk oracle. Repeat with the earlier version of `docs/compose.md` to exercise DSL transclusions. Both reproductions were executed with nextest.

The sweep follows the verified hash into every request decision; it includes clean sibling paths:

| Child ID | Site in package `dmls` | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F1.1 | `WorkspaceFacts::is_ready` and `candidates_for` | A late foreign row removes a current wiki link | Still ready; `notes/wiki.md` disappears from candidates of `docs/b.md` | Candidate completeness must survive foreign writes; do not accept an unverified row's absence of a key |
| R2-F1.2 | `request_documents` → `IndexStore::load` | Same foreign row, different hash | Returns the older hash and headings despite the remembered current hash | Use facts matching this process's byte read, or refresh before answering |
| R2-F1.3 | `symbol_documents` → `scan_headings` | Same row | Symbols differ from the unchanged disk oracle | Scan only facts verified for the answer |
| R2-F1.4 | `request_graph` → references, including heading, Markdown-link, and wiki probes | Same row; every corpus probe checked | All three probe families have answers differing from the oracle | Preserve the oracle's locations |
| R2-F1.5 | `request_graph` → DSL “transcluded by” | Earlier `docs/compose.md` has no `::file` | DSL references differ from the oracle | Preserve the oracle's transclusion referrers |
| R2-F1.6 | Heading rename and its candidate refresh | Wiki row loses `[[b#Some Heading]]` | Rename omits the wiki edit; refresh cannot discover the lost candidate | Include the current referrer before computing edits |
| R2-F1.7 | File rename and its candidate refresh | Wiki row loses `[[b#Some Heading]]` | File rename omits its wiki spelling edit | Include the current referrer before computing edits |
| R2-F1.8 | `revalidate` and `refresh` | A mismatching row is present before the pass | Clean: the pass hashes bytes and replaces a mismatching row; existing restart and live-refresh tests pass | Retain this behavior; cover a writer publishing after that check as well |
| R2-F1.9 | Open-buffer overlay in document and symbol readers | A buffer differs from the disk row | Clean: explicit buffer insertion replaces the row; `open_buffers_replace_rows_and_never_write_one` passes | Preserve the buffer override |
| R2-F1.10 | `MemoryFacts` | Process-local backend | Clean by ownership: another process cannot overwrite its map | Preserve the same answers without shared-row races |

**Repair boundary.** Enforce the existing byte/hash invariant across candidate selection, document loading, and heading scans, and ensure a request consumes a consistent view. Checking only the hash of loaded candidates cannot fix this: an overwritten row can remove a candidate before it is loaded. Add deterministic tests for a late batch from another writer and exercise all four requests, including DSL and wiki reference projections. The existing foreign-row test manually replaces a row before consumption; it does not test a ready `WorkspaceFacts` instance after a foreign write.

### R2-F2 (high): A cache failure during a request returns an incomplete answer

**Authority: spec.** The failure-fallback requirement says “A cache problem never fails an editor request,” and acceptance criterion 8 requires all four requests to “still answer correctly from memory” when the store is unusable. Switching the backend after constructing an incomplete answer does not meet that contract.

**Defect class:** recovery invalidates cached state without retrying the operation that already consumed incomplete recovery data.

In package `dmls`, [RootFacts::fall_back](../../dmls/src/index_store/source.rs) attempts to salvage all rows. If any row cannot be decoded, salvage fails and memory can be empty. The request-facing methods then return that partial memory result. [WorkspaceFacts::take_switched](../../dmls/src/index_store/workspace.rs) withdraws readiness, but the [router request handlers](../../dmls/src/router.rs) immediately serialize the result; they do not revalidate and retry the current request. Only a later request gets the rebuild. A rename can therefore return partial edits, and a symbol search or references request can report missing results as a successful answer.

**Reproduction through the public API.** Start with the same materialized corpus and a ready store, with positive oracle comparisons before mutation. Use another SQLite connection to damage the cache after readiness. For candidate decoding, replace the path of a row selected by `cand.key = 'b'` with invalid UTF-8 text. For document loading and heading scans, set stored heading levels to `9`. The fixture files remain valid and readable. Invoke each request reader as the first failing operation. The affected root switches to memory, its first answer is incomplete, and readiness becomes false. Explicit revalidation restores correct symbols. The document-load case also directly reproduced incorrect references and heading/file rename results; follow-on comparisons after each initial failure confirmed all four consumers can see incomplete memory.

These mutations simulate unreadable cache data, which the spec explicitly requires handling. The finding does not ask for additional validation of arbitrary adversarial Markdown or a stronger corruption-repair policy.

| Child ID | Site in package `dmls` | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F2.1 | `RootFacts::candidates_for` → `WorkspaceFacts::candidates_for` | Invalid stored path on a matching candidate row after ready | Failed salvage yields an incomplete candidate set; readiness is withdrawn afterward | Refill memory before a successful request consumes the set |
| R2-F2.2 | `RootFacts::load` → `request_documents` / `request_graph` | Invalid heading level after ready | Failed salvage yields a graph missing readable workspace files | Rebuild facts and retry the current request |
| R2-F2.3 | `RootFacts::scan_headings` → `symbol_documents` | Same invalid heading level, scan is the first reader | First symbol result omits files from the affected root | Return the complete memory-backed result |
| R2-F2.4 | Router references handler; ordinary, wiki, and DSL providers share its request graph | A request graph lost its root's facts | Handler emits success with the incomplete locations; direct heading-reference comparison differs from the oracle | Preserve all reference projections when retrying |
| R2-F2.5 | Heading rename, including pre-graph refresh | Failure while refreshing or loading its candidates | No post-refresh/post-read readiness gate; direct comparison loses edits or refuses a rename the oracle can perform | Revalidate before returning edits or a rename refusal caused by missing facts |
| R2-F2.6 | File rename, including pre-graph refresh | Same failed root | Direct comparison omits the wiki referrer edit; handler returns its result directly | Revalidate before returning the file's edits |
| R2-F2.7 | Router workspace-symbol handler | Heading-scan failure | Handler emits success with incomplete symbols | Retry the current search after memory is ready |
| R2-F2.8 | Failure during a build, before requests proceed | Mid-fill write failure | Clean: `finish_index_round` sees `PassReport::switched`, starts a full pass, then releases waiters; fault-injection tests pass | Retain this reconciliation for build-time failures |
| R2-F2.9 | `RootFacts` salvage when reads still work; failure at open | Write-only failure, or corrupt/off/unavailable store before first fill | Clean: readable rows are salvaged, or memory is populated before ready; existing fallback and request-equivalence tests pass | Preserve these successful recovery paths |

**Repair boundary.** Treat a read-side backend switch as invalidating the in-flight answer. Complete a memory revalidation and retry that request before sending success. Cover candidate lookup, document loading, heading scans, and rename's pre-query refresh; the current after-ready fault test exercises a refresh followed by an explicit revalidation, so it never asserts what the first editor request receives. This is a bounded extension of the existing rebuild mechanism, not a request for a new recovery framework.

## Blocked Findings

None. Both findings can be repaired without another design decision.

## Recurrence

None. `review-1.md` contains no code findings, and its accepted dispatch-thread decision is not being reopened.

## Verification and requirement coverage

I used the darkmatter and rust-testing skills, with their DMLS and test-design references, and consulted the cache/path and hashing skills for the relevant helpers. No terminal or browser rendering or keyboard behavior is added by this feature; Level 1 is the appropriate verification boundary for its protocol, persistence, and filesystem requirements.

| Specification behavior | Strongest relevant evidence checked | Assessment |
| --- | --- | --- |
| Unchanged restart parses zero; one changed file parses once (acceptance 1–2) | Level 1 public store and server-session parse-counter tests in `index_store_live` and `workspace_tests` | Passing |
| Never-open referrers participate after restart; multi-root and nested-root results (acceptance 3, 6) | Level 1 `index_store_sessions`, including cross-root edits and deduplication | Passing ordinary cases |
| Four requests, wiki ambiguity/spellings, DSL references, candidate superset (acceptance 4–5) | Level 1 corpus oracle comparisons in `index_store_requests`, `index_store_keys`, `wiki_universe_equivalence`; additional late-writer probes | Existing tests pass; R2-F1 covers the missing shared-writer case |
| Unsaved buffers and rename freshness/watch modes (open-buffer requirement; acceptance 7) | Level 1 `index_store_live` and `workspace_tests`; fallback rename discovers an unannounced referrer | Passing ordinary cases; both findings extend to rename consumers |
| Off/read-only/corrupt/locked stores and recovery (acceptance 8) | Level 1 in-process oracle comparisons, fault injection, compiled-binary stdio tests; additional failures after ready | Open-time cases pass; R2-F2 covers in-flight recovery |
| Shared fills and liveness (acceptance 9–10) | Level 1 two-process cold fill, shared-lock/sweep/repair tests, idempotent row writes | Passing tested interleavings; R2-F1 identifies a different late-write interleaving |
| Warm latency and memory; benchmark report (acceptance 11 and benchmark requirement) | Recorded repository measurements in `implementation-log.md`: 109–122 ms excluding discovery, 256–285 ms including it, 42.9–46.2 MiB added; Level 1 benchmark shape/child-process tests | Recorded values meet targets; no new performance run claimed |
| Full production graph build removed (acceptance 12) | Source inspection and passing `full_index_removal_guard` tests | Passing |
| Persistence version, row round trips, cache location/permissions, logs | Level 1 golden/round-trip/open/sweep/log tests; 15 cache-directory helper tests | Passing |

Executed on this macOS host:

- `cargo nextest run -p dmls --features effects-instrumentation --no-fail-fast`: **1,202 passed, 6 skipped**, before temporary review probes were compiled. No failing or slow test was reported.
- Temporary public-API probes compiled inside the declared `l1` binary and selected with `test(review2_)`; all **3 final probes passed by asserting the observed defects**. They exercise the real copied corpus, real SQLite connections, and the normal provider APIs. They were removed after verification; no implementation or test changes are part of this review.
- `cargo nextest run -p biscuit-file -E 'test(user_cache_dir)' --no-fail-fast`: **15 passed**.
- `just check-tier-coverage darkmatter`: **0 stranded tests**. The store tests are declared by `tests/l1/main.rs`; their names keep them in Level 1.

Cross-OS results are left to CI as requested. The documented optional benchmark reruns and global storage-maintenance skill edit remain outside this review's repair inventory.

## Input Robustness Matrix

The added reader is SQLite, not a new JSON/YAML/TOML configuration loader. The default for a structurally invalid store or undecodable fact is a typed store failure followed by the specified recovery ladder, with a complete editor answer. R2-F2 concerns the last step of that ladder.

Required scalar types and nullability are enforced by `STRICT` tables, `NOT NULL`, primary keys, and typed row reads. Optional wiki heading/alias fields preserve SQL null as absence; absent child rows are valid empty fact collections. Numeric validators cover heading levels, nonnegative positions/lines, ordered spans, paired nullable heading-span endpoints, and the boolean unsupported flag. Existing public `IndexStore` tests exercise a valid control, missing tables/stamps, format/root mismatches, a wrong SQL field type, invalid UTF-8, invalid heading/flag values, inverted spans, half a nullable span, and negative line numbers. Corrupt-file tests exercise SQLite's document loader. This review adds first-reader controls after readiness for candidate, document, and heading readers rather than multiplying every column by every request.

The requested smell search found no permissive `serde(default)` or parse-error-to-empty conversion in the store decoders. `MemoryFacts::get_all` intentionally filters missing map entries; it becomes harmful when an incomplete fallback map is consumed as a finished answer, covered by R2-F2. Cache override absent/empty/off/relative/file/directory shapes and the injected per-OS cache-directory rules have explicit tests. No broader arbitrary-corruption hardening is requested.

## Observations

**The root test recipe fails before nextest starts.** `just test dmls` exits with “a value is required for '--package <PACKAGES>' but none was supplied.” Running nextest directly succeeds. This runner defect is outside the index-store changes and is not a finding or a repair requirement for this cycle.
