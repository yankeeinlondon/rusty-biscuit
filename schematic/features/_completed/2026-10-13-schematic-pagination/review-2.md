---
$schema: feature-review.yaml
ready: true
findings: []
observations:
    - Traversal checkpoints copy accumulated loop history
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-14T16:18:20-07:00"
spec: 2026-10-13-schematic-pagination/spec.md
implemented: false
description: "A **feature** review of `2026-10-13-schematic-pagination/spec.md`"
feature: 2026-10-13-schematic-pagination/review-2.md
previous: 2026-10-13-schematic-pagination/review-1.md
refresh: refreshed
refresh_changes:
    - Replaced the next spec and plan's draft-pagination assumptions with delivered model, runtime, generator, and test interfaces.
    - Updated the endpoint-literal estimate and recorded the distinction between 35 legacy declarations and 94 generated pagers, including Bitbucket's binary GetSource endpoint.
    - Carried forward the author's session-partition, paging-callback, and optional manual-retry rulings as migration invariants.
    - Named the frozen tests and import-persistence checks the builder migration must preserve or replace, and corrected the next plan's stale check-drift runner advice.
---

# Review 2: Schematic pagination

The feature is **production ready within the specification's stated scope**. No high, medium, or low findings remain. All three findings from review 1 are implemented, including the author's revised retry ruling. The remaining observation does not require another repair cycle.

This review examined the working-tree implementation, not just the repair log: the definition model and resolution/validation, runtime traversal and request construction, retry checkpoints, sessions and partitions, generated forwards and their tests, provider authoring, importer overlay, documentation, and skills. Existing implementation-log results are identified separately from commands rerun here. No implementation source was changed.

## Previous findings and human rulings

The authority for each repair is **review-1.md, item 1, 2, or 3**, under its **Human rulings** heading. Those rulings govern over the original spec where they differ. In particular, the author rejected the original recommendation to keep `AlreadyFailed`; the original spec's exclusion of recovery and unconditional terminal-failure rule no longer apply.

| Previous finding and authority | Sites and shapes checked | Result |
| --- | --- | --- |
| Reject partitioned first-page sessions; review-1.md, item 1 | schematic-define's [strategy validation](../../define/src/pagination/validate.rs) and [partition tests](../../define/tests/pagination_partitions.rs): API default, endpoint override, and alternate; each with first-page opening plus partitions, first-page opening without partitions, and separately opened shared sessions. schematic-gen's [validation tests](../../gen/tests/l1/pagination_validation.rs) check the emitted diagnostic. | Fixed. The forbidden combination reports `partitioned-first-page-session`. The supported siblings validate. Runtime partition tests retain concurrent requests, independent histories, latest-observed shared handles, and one close. Public docs and both Schematic skills state the restriction. |
| Publish and verify paging callbacks; review-1.md, item 2 | schematic-runtime's [page decoder](../../runtime/src/pager/request.rs), client hook contract, [hook guide](../../docs/runtime/hooks.md), and schematic-gen's [fresh generated-client tests](../../gen/tests/l1/fresh_output_compile.rs). The generated test exercises `pages`, `fetch_page`, `collect_all`, and an ordinary send with both callbacks installed. | Fixed. Paging applies the global JSON callback before item and continuation extraction and skips the typed endpoint callback. Ordinary sending applies both in order. Assertions check transformed items, continuation requests, callback counts, and exhaustion without an extra request. |
| Retain complete errors and allow optional explicit retry; review-1.md, item 3 | schematic-runtime's [error type](../../runtime/src/pager/error.rs), [Pages and Items](../../runtime/src/pager/pages.rs), [session state](../../runtime/src/pager/session.rs), and [recovery tests](../../runtime/tests/l1/pager_retry.rs); freshly generated-client recovery test. Checked API errors, malformed items, missing/invalid/repeated continuations, boundary stalls, empty pages claiming more, invalid arguments, expired deadlines, rejected links, session opening/missing handles, backward traversal, and partitions. | Fixed. `AlreadyFailed` is removed. Repeated calls retain the specific error and evidence and send nothing. Explicit retry remains callable for every category; a failed attempt replaces the evidence. Retry preserves position, direction, partition isolation, inclusive identities, and current adjustable size. Continuation refresh does not redeliver its source items. Buffered recovered items count only when delivered. Provider errors need `Sync`, without requiring `Clone`. |

The paging guide, runtime README, affected rustdocs, implementation-log addendum, and Schematic skills agree with these repairs. Consuming `fetch_page` and `collect_all` remain fail-fast; callers retain `Pages` or `Items` when they want recovery. The ruling does not require a recovery handle in a consuming helper's error.

## Specification coverage and verification fidelity

The labels below describe what was actually observed. A locally scripted HTTP response remains **F1 evidence about the provider's input format**, even when a compiled generated client consumes it. Compiling generated output and exercising it against a local server is **F2 evidence about the generated client's behavior**. These are different claims.

| User-observable requirement | Boundary and strongest implementation evidence | Assessment |
| --- | --- | --- |
| Model, builders, serde, validation, defaults/exceptions, list recognition, alternates, and adoption; requirements 1–8 and 26 | The authoring model is our own API: F1 round trips, valid/invalid siblings, resolution tables, exhaustive shipped-definition coverage, and item-type/schema checks. Rust compilation provides F2 for accepted literal/type paths and generated request omissions. | Meets the authored-model contract. GET-only explicit `Vec<T>` inference is preserved; wrappers and POST lists need explicit entries. Legacy metadata remains inert. |
| Only authored controls compile; thin generated forwards; iteration, item types, and customized single-page fetch; requirements 9–11a and 25 | Rust compiler and generated client: F2 through runtime trybuild, fresh isolated generated crates, committed-schema compile-fail cases, and generated forge/cursor/link HTTP tests. | Meets the public surface contract. Tests cover unavailable controls and outside-scope session iteration, plus positive page, fetch, and collection behavior. |
| End detection, size bounds/defaults, loops, composite/inclusive cursors, follow-up requests, and nested item extraction; requirements 12–17 | HTTP transport and typed decoding: local-server runtime tests exercise all four kinds through the public API; F1 for manufactured response shapes, supplemented by F2 generated-client execution for page-number, cursor, and next-link strategies. Offset emission also has generator assertions; no shipped definition currently emits an offset pager. | Explicit false signals beat stale cursors; missing/invalid signals remain errors; source-record counts drive offsets; malformed pages contribute no items; required defaults and caller overrides reach requests. Follow-up tests cover query/header/body destinations, body carry forms, replacement paths/methods, and exact link queries. |
| Origin policy, shared deadlines, page envelopes, complete-or-incomplete collection, and privacy; requirements 18–21 and 36 | Real HTTP clients and loopback servers, including refused ports, cut-off bodies, redirects, two origins, recorded request counts, and timed cancellation. Generated-schema tests add F2 execution of the emitted controls. | Tests retain status/body/metadata, refuse disallowed sends, stop at the shared deadline, preserve partial items only in `Incomplete`, inspect the final allowed page before declaring a limit, and keep secrets out of pager-created diagnostics. Raw response evidence retains the existing envelope limitations. |
| Scoped sessions, bounded close, renewal, rotating handles, and partitions; requirements 22, 22a, and 23 | Local session servers and Rust compiler: runtime public-API behavior plus F2 fresh generated-session execution and compile-fail scope checks. Response content is synthetic. | Close is attempted once on ordinary success/error and unwind with a usable handle, under its separate grace; failures retain closure outcomes. Cancellation relies on the server lease. Partition requests remain caller-driven and concurrent. The item 1 ruling narrows the supported opening combination. No shipped provider definition authors sessions or partitions. |
| Provider authoring, unverified opt-outs, all inventoried mechanisms and outside patterns; requirements 27, 28, 30, and 33 | Exhaustive definition resolution/knob tables and local-server mechanism tests: F1 about provider behavior. Vendor-schema-derived output also crosses the Rust compiler boundary. | 18 REST APIs, 94 authored pagers, and the exact 32-endpoint verification task are pinned. Six recalled outside patterns remain explicitly `unverified_*`; they prove mechanisms only. See the evidence limitation below. |
| Export stability, dependents, importer persistence, docs/skills, and decode-path decision; requirements 24, 29, 32, 34, and 35 | Regeneration comparisons, real Rust compilation/execution, vendored OpenAPI import, and document review. The current re-import test compares both emitted OpenAI source files byte for byte with the files compiled by the workspace; coverage tests then evaluate their overlaid definitions. | Current tests pass. The original implementation log records the export-baseline comparison, dependent test runs, and the permitted decision to retain `from_value` after the size experiment. This review did not repeat the release measurement or all dependent suites. |

**Provider evidence limitation and accepted assumption.** No live provider call or dated HTTP-response capture was added or run; this review does not claim F3 proof of live pagination behavior. The spec's **Authoring table** explicitly accepts `V-def` and `V-notes` evidence, distinguishes them from independent server verification, and excludes live credentialed requests. The implementation log's Phase 6 records the actual sources: the OpenAI vendor document retrieved 2026-07-31; forge research under `sniff/docs/repos/` dated 2026-02-15 and its 2026-10-09 inventory; ElevenLabs research dated 2026-05-07. The scoped acceptance assumption is that these documented contracts remain accurate. Calls to paid or credentialed counterparties are outside this review's authorized test boundary. Unknown or contradictory rows remain unpaginated, rather than being promoted by mock responses. This is the spec's explicit evidence limitation, not a claim that scripted responses supply native provider evidence.

**Input robustness matrix.** No definition/configuration file loader was introduced by pagination. Its serde surface is a model round-trip facility; [pagination_serde.rs](../../define/tests/pagination_serde.rs) covers unknown/missing fields, wrong whole/element types, explicit empty values, duplicates, trailing content, and the declared absent/null equivalence of optional dimensions. The provider JSON decoder reads a protocol response, not configuration; its collection, continuation, signal, and total shapes are covered by the rule tests. The preexisting OpenAPI reader is not newly generalized by the overlay hook. The configuration-loader matrix therefore does not apply to this change.

## Verification performed for this review

All commands below ran against the current working tree on macOS. No terminal or browser windows were opened.

| Command | Result |
| --- | --- |
| `cd schematic && just test` | Passed: 2,230 ordinary L1 tests; six fresh-output guards; one runtime compile-fail guard. |
| `cd schematic && just test-schema` | Passed: 55 tests, zero skipped, including generated-client paging and compile-fail diagnostics. |
| `cd schematic && just lint` | Passed for all five area packages. |
| `cd schematic && just lint-schema` | Passed for the excluded generated crate. |
| `just check-tier-coverage schematic` | Passed: zero stranded tests. |
| `git diff --check` | Passed before review edits; repeated after final document updates. |

The ordinary L1 invocation reported 12 exclusions. Seven are the slow guards subsequently run by the same recipe; the remaining five are preexisting ignored generation/opt-in registry tests. They are not claimed as passing evidence. New retry tests are declared in the runtime's consolidated L1 target; generated child tests are checked by their registration guard. No new test is hidden behind an unavailable tier or feature. Cross-platform execution remains CI's responsibility and does not block this review.

## Findings

None. The three earlier findings are closed; no additional repair is requested.

## Observations

### Traversal checkpoints copy accumulated loop history

schematic-runtime's [page traversal](../../runtime/src/pager/pages.rs) clones its [engine](../../runtime/src/pager/engine.rs), including the accumulated continuation sets, to preserve rollback and retry state. Total copying therefore grows quadratically with the number of pages in a long uninterrupted traversal. This is a source-level cost observation, not a measured performance regression or an unmet acceptance criterion. The default collection limit bounds ordinary collection to 100 attempts, and the snapshots protect important recovery guarantees. Profile a real large traversal before replacing them with a more complicated checkpoint scheme; no optimization is requested in this cycle.

## Next-spec refresh

Refreshed `2026-10-14-schematic-definition-api` and its plan with the delivered interfaces, ABI 4, actual literal count, pagination coverage and exclusions, the frozen-test map, and all three settled review rulings. The plan now uses the existing nextest-based drift recipe. Historical author decisions and pipeline order were preserved. No contested scope change or new human decision was needed.
