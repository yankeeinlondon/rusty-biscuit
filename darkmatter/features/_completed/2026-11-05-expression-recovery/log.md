---
kind: log
feature: 2026-11-05-expression-recovery
implementation_1: "2026-11-09T00:19:08-08:00"
implementation_2: "2026-11-09T01:04:52-08:00"
---

# Expression recovery implementation log

## Implementation of Review Findings #1

> **started at:** 2026-11-09T00:19:08-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-expression-recovery/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs began
- the log file was empty at the start of this iteration, so this section starts the file

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | Missing sources bypass invalid-request checks | fixed | shared `plan_read` used by the present and missing paths (`check_absent_request`); all children below; CLI `compose_keeps_invalid_requests_fatal::computed_invalid_options_for_missing_source` |
| R1-F1.1 | YAML + computed `delimiter: ";"`: invalid options whether missing or present | fixed | `expression_recovery_data::invalid_request_fails_whether_or_not_present::yaml_delimiter::{missing,present}` |
| R1-F1.2 | JSON + computed delimiter: invalid options in both | fixed | `expression_recovery_data::invalid_request_fails_whether_or_not_present::json_delimiter::{missing,present}` |
| R1-F1.3 | JSON Lines + computed delimiter: invalid options in both | fixed | `expression_recovery_data::invalid_request_fails_whether_or_not_present::jsonl_delimiter::{missing,present}` |
| R1-F1.4 | JSON5 + computed delimiter: invalid options in both | fixed | `expression_recovery_data::invalid_request_fails_whether_or_not_present::json5_delimiter::{missing,present}` |
| R1-F1.5 | TOML + computed delimiter, object contract: invalid options in both | fixed | `expression_recovery_data::invalid_request_fails_whether_or_not_present::toml_delimiter::{missing,present}` |
| R1-F1.6 | YAML + computed `header: false`: invalid options in both | fixed | `expression_recovery_data::invalid_request_fails_whether_or_not_present::yaml_header::{missing,present}` |
| R1-F1.7 | TSV + computed semicolon delimiter: invalid options in both | fixed | `expression_recovery_data::invalid_request_fails_whether_or_not_present::tsv_semicolon::{missing,present}` |
| R1-F1.8 | CSV object-row contract + `header: false`: row-shape conflict in both | fixed | `expression_recovery_data::invalid_request_fails_whether_or_not_present::csv_headerless_object_rows::{missing,present}` |
| R1-F1.9 | CSV + computed `string[]` contract: row-shape conflict in both | fixed | `expression_recovery_data::invalid_request_fails_whether_or_not_present::csv_string_rows::{missing,present}` |
| R1-F1.10 | TSV + computed `string[]` contract: row-shape conflict in both | fixed | `expression_recovery_data::invalid_request_fails_whether_or_not_present::tsv_string_rows::{missing,present}` |
| R1-F1.11 | `.xyz` + `format: yaml` + semicolon: invalid options in both | fixed | `expression_recovery_data::invalid_request_fails_whether_or_not_present::explicit_format::{missing,present}`; unit `a_format_option_judges_every_candidate_by_that_format` |
| R1-F1.12 | Recursive `%optional.xyz` without format: unsupported-format error | fixed | `expression_recovery_data::invalid_request_fails_whether_or_not_present::recursive_unsupported_extension::{missing,present}`; unit `an_authored_candidate_names_the_reference_as_written` |
| R1-F1.13 | Nonrecursive `optional.xyz`: unsupported format in both (clean control) | clean | existing `missing_source_with_unsupported_extension_is_not_recovered` and `unsupported_format_is_not_recovered` pass |
| R1-F1.14 | Recursive `%optional.yaml` miss: fallback (clean control) | clean | `valid_request_recovers_only_a_miss::recursive_supported_extension::{missing,present}` |
| R1-F1.15 | Delimiter value validator `";;"`: invalid options in both (clean control) | clean | `DataOptions::from_bound` runs before resolution; CLI `computed_invalid_options` passes |
| R1-F1.16 | Scalar contract admission: not-a-container in both (clean control) | clean | contract prepared before resolution; `not_a_container_is_not_recovered` passes |
| R1-F1.17 | `Name@this` contract: this-import-rejected in both (clean control) | clean | `this_import_is_not_recovered` passes |
| R1-F1.18 | Missing contract file: contract-import error in both (clean control) | clean | `missing_contract_import_is_not_recovered` and CLI `missing_contract_import` pass |
| R1-F1.19 | Valid request controls: missing → fallback, present → values (clean control) | clean | `valid_request_recovers_only_a_miss::{yaml_list,csv_tuple_rows,explicit_format}::{missing,present}`; CLI `compose_keeps_invalid_requests_fatal::control` |
| R1-F1.20 | Passive call checking of literal invalid options/rows (clean control) | clean | call check untouched; `just test call_check` 112/112 |
| R1-F1.21 | Recovery classifier and both executors unchanged (clean control) | clean | classifier and executors untouched; `just test try` 568/568 |
| R1-F2 | Stale remote cache hides a denied redirect | fixed | stale arm serves stale content only for `RemoteReadKind::Transport`; unit `remote_fetch::persistent_cache_tests::fallback_serves_stale_only_over_a_transport_failure`; regression proven red by reverting the condition |
| R1-F2.1 | `load_data` typed adapter, warm cache + denied redirect: policy refusal propagates | fixed | `expression_recovery_data::warm_cache_never_hides_a_refused_redirect` (Policy kind, no target request, fallback not run); CLI `test_compose_remote_fallback_never_serves_stale_cache_over_a_refused_redirect` |
| R1-F2.2 | `file_exists` typed adapter: policy refusal propagates | fixed | shared path `get_content_typed` → `fetch_with_cache`; manual `md` probe exits 1 with "blocked by fetch policy" |
| R1-F2.3 | `frontmatter` via `load_markdown`: policy refusal propagates | fixed | shared path `load_markdown` → `fetch_remote_text` → `fetch_with_cache`; manual probe exits 1 |
| R1-F2.4 | `markdown_title` via `load_markdown`: policy refusal propagates | fixed | same path as R1-F2.3; manual probe exits 1 |
| R1-F2.5 | `markdown_body_empty` via `load_markdown`: policy refusal propagates | fixed | same path as R1-F2.3; manual probe exits 1 |
| R1-F2.6 | `validate_schema` via `load_markdown`: policy refusal propagates | fixed | same path as R1-F2.3; manual probe exits 1 |
| R1-F2.7 | `try_frontmatter`: receives refusal, keeps its error-as-data behavior | fixed | same path; probe returns `{ok:false, error:"…blocked by fetch policy"}`, error-as-data contract unchanged |
| R1-F2.8 | `::file` string adapter: reports failed remote read | fixed | transclusion engine → `get_content` → `fetch_with_cache`; manual probe exits 1 |
| R1-F2.9 | `::code` string adapter: reports failed remote read | fixed | same engine path; manual probe exits 1 with `TransclusionError … blocked by fetch policy` |
| R1-F2.10 | Approval discovery remote child reader: reports refused child read | fixed | `preflight/collect.rs::fetch_remote_child_body` → `get_content`; `md compose --shell` probe exits 1 |
| R1-F2.11 | Cold-cache denied redirect: policy error (clean control) | clean | `FetchFresh` arm unchanged; `redirect_to_denied_host_is_fatal` |
| R1-F2.12 | Strict revalidation branch: policy error (clean control) | clean | non-Fallback modes return the error; `strict_failure_without_fallback_errors` |
| R1-F2.13 | Mandatory revalidation (`no-cache`): error (clean control) | clean | `fallback_does_not_serve_a_no_cache_entry_after_failed_revalidation` |
| R1-F2.14 | Initial host admission: error (clean control) | clean | `check_allowed` precedes the store read; `denied_host_never_reads_a_fresh_cached_entry`, `denied_host_is_fatal_with_warm_cache_and_memo` |
| R1-F2.15 | Allowed origin HTTP 500 with warm cache: stale fallback succeeds (clean control) | clean | new `warm_cache_covers_a_transport_failure` plus the HTTP 500 half of the unit test; existing `network_failure_serves_stale_under_fallback` |
| R1-F3 | Value-only `try` calls return a generic handler error | fixed | `call_entry` no-handler branch returns `ExpressionError::ContractViolation` with the original message |
| R1-F3.1 | `call_function("try", …)` rejects with a typed contract error, nonrecoverable | fixed | `direct_value_call_of_try_is_rejected` asserts ContractViolation, not recoverable, fatal |
| R1-F3.2 | Other lazy functions through the same value-only branch: consistent classification | fixed | `direct_value_call_of_evaluator_owned_functions_is_a_contract_violation` covers `and`, `or`, `map`, `filter` (the only other lazy/callback bindings) |
| R1-F1.22 | Discovered: existing test asserted a missing file outranks an explicit-format applicability error | fixed | rewritten as `load_data_options::applicability_outranks_a_missing_file` (computed and literal input, plus a valid-request NotFound control) |
| R1-F1.23 | Sibling sweep: remote path, present-file raw read, other read functions (`frontmatter`, `file_exists`, …) | clean | remote checks precede the fetch; other readers take no format, options, or row contract |
| R1-F2.16 | Discovered: revalidation 200 with non-UTF-8 body | clean | returns a Transport error, never stale content; docs corrected to say so |
| R1-F2.17 | Sibling: `fetch_remote_text` first-probe `if let Ok` | clean | a failure re-reads the same failed slot and keeps its kind |
| R1-F2.18 | Sibling: `effects::http_post` | clean | uncached; nothing can be served after an error |
| R1-F2.19 | Sibling: `DataRuntime` source memo | clean | memoizes only successful reads; sweep found `fetch_with_cache` is the only serve-after-error site in the workspace |
| R1-F3.3 | Discovered: evaluator path for `map`/`filter` without an inline callback | fixed | `callback_function_without_inline_callback_is_a_contract_violation`; composition already rejects at call check (`md compose` output unchanged) |
| R1-F3.4 | Discovered: lifecycle `call_with_operand_values` for `map`/`filter` | fixed | `lifecycle::executor` `callback_function_with_template_operand_is_a_contract_violation` |
| R1-F3.5 | Sibling: `call_host` host function without a handler | clean | a different class ("no implementation in this process"); host registration already refuses lazy functions |

### Progress
- starting the work on 'R1-F1 missing sources bypass invalid-request checks' at 00:19:44
        - delegated to a foreground rust-developer subagent that owns the progress sub-bullets for this finding
        - defect class: an invalid request is reported as a recoverable source miss because request validation runs only after a source is located
        - sibling sweep (every path the feature changed that can return a recoverable miss before request validation):
            - `load_data_fn` local miss (`resolve_arg` → `None`): defective; fixed (R1-F1.1–R1-F1.11)
            - `unreadable_candidates` skipped recursive (`%`) references: defective; fixed (R1-F1.12); the helper is removed, its rule folded into the shared check
            - `load_data_fn` remote path (`RemoteNotEnabled`, fetch/transport errors): clean; format, applicability, and row checks already run before the fetch
            - `load_data_fn` raw-read failure (unreadable present file, recoverable): clean; it runs after the request checks
            - `call_check.rs::load_data_problem` (passive `DataOptions`/`require_positional_rows`/`data_format_of` consumer): clean; it never looks for the file (R1-F1.20)
            - `frontmatter`, `try_frontmatter`, `markdown_title`, `markdown_body_empty`, `validate_schema` (via `load_markdown`), `file_exists`, `relative`/`absolute`: clean; they take no format, options, or row contract, so there is no request check a miss could bypass
            - recovery classifier (`is_recoverable_read_failure`) and both executors: unchanged (R1-F1.21)
            - `load_data_options::missing_file_keeps_precedence_over_applicability` pinned the defect (missing file outranked an explicit-format applicability error): new row R1-F1.22, test rewritten as `applicability_outranks_a_missing_file` (the tabular spec kept resolution precedence only for checks "that require the resolved source"; an explicit format does not)
        - repair: one helper `load.rs::plan_read` (format choice, `check_applicability`, effective delimiter/header, row mode, `require_positional_rows`) is called by the present path and by `check_absent_request` on the miss path; slot-approval ordering, `record_probed_candidates`, and the valid-miss result are unchanged
            - decision: absent candidates are the nonrecursive resolution plan's paths; a recursive reference (whose plan lists search roots, not files) or a plan that cannot be listed uses the authored reference text; no resolved identity is invented and nothing extra is read or canonicalized (a planned path is canonicalized only to name it in an unsupported-format error, as before)
            - decision: when candidates map to different formats, the miss stands if ANY candidate accepts the request; only when every candidate refuses is the first refusal (resolution order) reported; an unreadable extension counts as a refusal, which preserves the earlier unsupported-format rule
            - decision: a recursive unsupported-format error names the authored reference (`%optional.xyz`) as its identity text
        - tests: `expression_recovery_data::invalid_request_fails_whether_or_not_present::{12 cases}::{missing,present}`, `valid_request_recovers_only_a_miss::{4 cases}::{missing,present}`, `load.rs` unit tests for the multi-candidate verdict (4), CLI `compose_keeps_invalid_requests_fatal::computed_invalid_options_for_missing_source`; reverting the miss-path call made 11 `::missing` cases fail (fail-fast stopped the 12th) while every `::present` passed
        - docs: `loading-data.md` "Files that may be missing" and `darkmatter-expressions.md` recovery section; `darkmatter` skill `expressions.md` call order
        - verification: `just test expression_recovery` 126/126, `just test load_data` 505/505, `just test try` 568/568, `just test data::load::` 8/8, `just test expression::data::` 101/101, `just test call_check` 112/112, doc-reading guards pass, `just lint` clean
        - R1-F1 implementation complete, ready for review
- work completed for 'R1-F1 missing sources bypass invalid-request checks' at 00:32:43
        - orchestrator inspected the load.rs diff: one plan_read helper now serves both the present path and the missing path (check_absent_request), so the two paths cannot drift
- starting the work on 'R1-F2 stale remote cache hides a denied redirect' at 00:32:43
        - delegated to a foreground rust-developer subagent that owns the progress sub-bullets for this finding
        - defect class: a failed revalidation is treated as a transport outage without checking its typed cause, so a refusal becomes a successful stale read
        - sibling sweep (every site that can serve cached or memoized content after a fetch error; grep `StaleServed`, `RemoteFreshnessMode::Fallback`, `stale`, `fetch_with_cache`, `get_content`, direct `fetch(`/`PolicyClient` uses across darkmatter lib/cli/dmls, biscuit-file, claudine, simplified-schema):
            - `cache/remote_cache.rs::fetch_with_cache` revalidation `Err(_)` arm: defective; fixed (R1-F2, R1-F2.1–R1-F2.10 share it); the only stale-serving branch in the workspace
            - `fetch_with_cache` no-store and `FetchFresh` arms: clean; an error returns through `RemoteFetchFailure::from` with no cached body (R1-F2.11)
            - `fetch_with_cache` revalidation `Ok(resp)` with a non-UTF-8 body: clean for this class; it returns a `Transport` error, never stale content (new row R1-F2.16)
            - `RemoteFetchRuntime::register_and_fetch` host admission: clean; `check_allowed` runs before the store is read (R1-F2.14)
            - `resolve_ctx.rs::fetch_remote_text` first probe (`if let Ok(Some(body))`): clean; an `Err` re-reads the same failed slot and keeps its typed kind (new row R1-F2.17)
            - `effects/verbs.rs::http_post`: clean; no cache, so nothing can be served after an error (new row R1-F2.18)
            - `DataRuntime` source memo: clean; only successful reads are memoized and a failed remote slot is never replaced (new row R1-F2.19; `denied_host_is_fatal_with_warm_cache_and_memo` already pins it)
            - `preflight/collect.rs::fetch_remote_child_body` and `transclusion/engine.rs` `::file`/`::code`: consumers of the shared slot; they turn any slot error into a failure, so the cache arm was the only defect on their path
        - repair: the arm now converts the error with `RemoteFetchFailure::from` and serves stale only when `kind == RemoteReadKind::Transport` (and, as before, `Fallback` and not `no-cache`); otherwise it returns that typed failure
            - decision: classify with the existing `From<FetchError>` mapping rather than matching `FetchError` variants in the cache, so one table decides both recovery and stale-serving
            - decision: the CLI mock HTTP server (`cli/tests/common/mod.rs`) learned `302`, with `body` sent as `Location`, rather than adding a `location` field to all 27 `MockHttpResponse` literals
        - tests: unit `remote_fetch::persistent_cache_tests::fallback_serves_stale_only_over_a_transport_failure` (refused redirect → `Policy`, HTTP 500 → stale); library `expression_recovery_data::{warm_cache_never_hides_a_refused_redirect, warm_cache_covers_a_transport_failure}`; CLI `compose_remote_caching::test_compose_remote_fallback_never_serves_stale_cache_over_a_refused_redirect`; reverting the kind check made all three regressions fail
        - ad-hoc `md compose` probe (local Python server, temp dirs and cache roots): warm-up succeeds, then the refused redirect fails with "blocked by fetch policy" for `load_data`, `file_exists`, `frontmatter`, `markdown_title`, `markdown_body_empty`, `validate_schema`, `::file`, `::code`, and `--shell` discovery; `try_frontmatter` returns `{ok: false, error: "…blocked by fetch policy"}`
        - docs: `caching.md` Freshness, `remote-url-references.md`, `docs/cli/compose.md` `--remote-freshness`, `darkmatter-dsl.md` cache note, `RemoteFreshnessMode::Fallback` and CLI `RemoteFreshness::Fallback` docs, `remote_cache.rs` module and `## Errors` docs, `darkmatter` skill `remote-cache.md`
        - verification: `just test remote_cache` 20/20, `just test expression_recovery` 128/128, `just test remote` 260/260, `just test compose_remote_caching` 21/21, `just test remote_fetch` 50/50, other `mock_http_server` users (`path_completion::` 87, `policy_verbs::` 25, `render_style_wiring::` 23), doc-reading guards pass, `just lint` clean
        - R1-F2 implementation complete, ready for review
- work completed for 'R1-F2 stale remote cache hides a denied redirect' at 00:42:35
        - orchestrator inspected the remote_cache.rs diff: the stale arm now converts once through RemoteFetchFailure::from and serves stale content only for RemoteReadKind::Transport
- starting the work on 'R1-F3 value-only try calls return a generic handler error' at 00:42:35
        - delegated to a foreground rust-developer subagent that owns the progress sub-bullets for this finding
        - defect class: an evaluator-owned function invoked through a value-only entry point is reported as a generic handler failure instead of a call-contract violation
        - sweep (grep `EvaluationMode::Lazy`, `handler: None`, `call_function`, "called only by the evaluator", `lazy_function`; Claudine, DMLS, and CLI have no `call_function` caller and never match `ExpressionError::Other`/`ContractViolation`):
            - `functions/mod.rs::call_entry` no-handler branch: defective; fixed (R1-F3, R1-F3.1, R1-F3.2); the only evaluator-owned bindings are lazy `and`, `or`, `try` and callback `map`, `filter`
            - `expression/mod.rs::evaluate_function`: lazy names dispatch to the lazy evaluator first, but a callback function spelled without an inline callback (`map([1], 1)`) parses as `FunctionCall` and reached the branch as `Other`; fixed by the shared branch (new row R1-F3.3); in composition the passive call check already rejects it first as `dm.expression.incompatible_call` (probed with `md compose`, literal and frontmatter-valued second argument, before and after), so composed output is unchanged
            - `lifecycle/executor.rs::call_with_operand_values` (a template operand): lazy names are handled before the value call; `map`/`filter` reached the branch as `Other`; fixed by the shared branch (new row R1-F3.4)
            - `functions/mod.rs::call_host` `handler: None`: clean for this class; it is a host function without an implementation in this process, not an evaluator-owned one, and `HostFunctionError::LazyFunction` refuses lazy host registrations (new row R1-F3.5)
            - `test_hooks::call`: clean; test functions are always eager (`lazy_function` returns `None` for them)
            - `FunctionRegistration::bind_call`/`lazy_call` (public binding API): clean; binds only and invokes no handler
        - repair: the branch returns `ExpressionError::ContractViolation` with `function` set to the canonical name (no lazy or callback binding has aliases, so the text is unchanged) and the same explanatory messages, for lazy and callback bindings alike
            - projection: renders identically to `Other` (`{function}(): {message}`); `is_authoring_fatal` true (never demoted to a lenient `dm.expression.evaluation_failure` warning); `is_recoverable_read_failure` false, so `try` never recovers it; no catalog, Claudine, or DMLS code branches on either variant
            - decision: one shared branch for lazy and callback bindings rather than keeping callback as `Other`; no reason was found to keep callback generic, and every authored path that reaches it is already fatal through the call check
            - decision: no new variant; `ContractViolation` already means "the call breaks the function's contract and is never demoted"
        - tests: `functions::registration_tests::direct_value_call_of_try_is_rejected` now asserts `ContractViolation`, nonrecoverable, fatal; new `direct_value_call_of_evaluator_owned_functions_is_a_contract_violation` (`and`, `or`, `map`, `filter`); new `expression::tests::error_enrichment::callback_function_without_inline_callback_is_a_contract_violation` (R1-F3.3); new `lifecycle::executor::tests::callback_function_with_template_operand_is_a_contract_violation` (R1-F3.4)
        - docs: `call_function` `## Errors` and an inline WHY comment in `call_entry`; `docs/topics/expression-engine.md` lazy-function paragraph now names the contract violation and the callback case; no other docs/ or skill page mentions the old classification
        - verification: `just test direct_value_call` 2/2, `just test try` 568/568, `just test expression_recovery` 128/128, `just test functions::` 299/299, `just test lifecycle::executor::` 16/16, `just test error_enrichment` 4/4, `just test fatality` 21/21, `just test lazy` 86/86, `just test callback` 120/120, `just lint` clean
        - R1-F3 implementation complete, ready for review
- work completed for 'R1-F3 value-only try calls return a generic handler error' at 00:49:49

### Completion Report

- **outcome: complete**; elapsed about 35 minutes (00:19:08 → 00:55:03)
- parent findings: total 3 = fixed 3 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - R1-F1 (high), fixed: one shared `plan_read` request check now runs before a missing source's recoverable miss as well as before a present read. That covers recursive references, judged by their authored extension. 21 review children plus 2 discovered rows were resolved (12 fixed, 11 clean controls)
        - R1-F2 (medium), fixed: `fetch_with_cache` serves a stale body only when the typed failure kind is Transport, so refusals propagate to every reader. 15 review children plus 4 discovered sibling rows were resolved (10 fixed, 9 clean)
        - R1-F3 (low), fixed: value-only invocation of lazy and callback bindings is now `ExpressionError::ContractViolation`, which is nonrecoverable. 2 review children plus 3 discovered rows were resolved (4 fixed, 1 clean)
- open finding or child IDs: none
- repair decisions are recorded in the review's `human_review_items` for author confirmation: R1-F1 mixed-format candidates, R1-F1 precedence reversal, R1-F2 kind-table gating, R1-F3 callback functions
- cross-OS: no Windows cross-check was run. The repairs add no OS-specific code, and the new tests use temp dirs and the existing mock HTTP servers. Path handling in `absent_candidates` reads only extensions from plan paths or authored text, so CI's nightly Windows and WSL2 legs cover it

#### Changed files

- R1-F1: `lib/src/markdown/compose/expression/data/load.rs`, `lib/tests/l1/expression_recovery_data.rs`, `lib/tests/l1/load_data_options.rs`, `cli/tests/l1/expression_recovery.rs`, `docs/topics/loading-data.md`, `docs/topics/darkmatter-expressions.md`, `.claude/skills/darkmatter/expressions.md`
- R1-F2: `lib/src/markdown/compose/cache/remote_cache.rs`, `lib/src/markdown/compose/remote_fetch.rs`, `lib/src/markdown/compose/remote.rs`, `lib/tests/l1/expression_recovery_data.rs`, `cli/tests/common/mod.rs`, `cli/tests/l1/compose_remote_caching.rs`, `cli/src/args/enums.rs`, `docs/topics/caching.md`, `docs/topics/remote-url-references.md`, `docs/cli/compose.md`, `docs/darkmatter-dsl.md`, `.claude/skills/darkmatter/remote-cache.md`
- R1-F3: `lib/src/markdown/compose/expression/functions/mod.rs`, `lib/src/markdown/compose/expression/mod.rs`, `lib/src/lifecycle/executor/tests.rs`, `docs/topics/expression-engine.md`

#### Final verification (run by the orchestrator after all changes)

| Command | Result | Verifies |
| --- | --- | --- |
| `just test` (darkmatter) | 16103 passed, 0 failed, 10 skipped | all findings, no regressions |
| `just lint` (darkmatter) | exit 0 | all findings |
| root `just check-tier-coverage darkmatter` | 0 stranded | new tests are selected by L1 |
| `just test lifecycle` (claudine) | 1271 passed | R1-F3.4 lifecycle consumer |

## Implementation of Review Findings #2

> **started at:** 2026-11-09T01:04:52-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-expression-recovery/review-2.md'
- this is iteration 2 of the review-to-implement cycle 
- the review's `implemented` flag was confirmed `false` before repairs began
- the review has one finding (R2-F1, medium), no blocked findings, and no `human_in_the_loop` items
- defect class for R2-F1: a request's validity is judged from reference text before the reference's environment placeholders are expanded, so the judged filename or extension can differ from the one resolution would use

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R2-F1 | Recursive missing-source format checks use unexpanded environment references | fixed | every child row below is fixed or clean; full `just test` (darkmatter, 16118 passed) and `just lint` green |
| R2-F1.1 | Recursive `%{{DATA_FILE}}` (`optional.yaml`): missing → `[]`; present → loaded array | fixed | `expression_recovery_data::env_reference_missing_and_present::recursive_env_file::{missing,present}`; CLI `recursive_env_reference_recovers_only_a_miss`; orchestrator's debug-CLI repro: missing → exit 0, `[]`, no diagnostics; present → `["loaded"]` |
| R2-F1.2 | Recursive `%optional.{{DATA_EXT}}` (`yaml`): missing → `[]`; present → loaded array | fixed | `expression_recovery_data::env_reference_missing_and_present::recursive_env_extension::{missing,present}` |
| R2-F1.3 | Recursive `%{{DATA_FILE}}`, YAML, `delimiter: ";"`: invalid options whether missing or present | fixed | `expression_recovery_data::env_reference_missing_and_present::recursive_env_file_delimiter::{missing,present}`: `dm.data.invalid_options` in both, fallback not read |
| R2-F1.4 | Recursive `%{{DATA_DIR}}/optional.yaml`: missing → `[]`; present → loaded (clean control) | clean | `expression_recovery_data::env_reference_missing_and_present::recursive_env_dir::{missing,present}` |
| R2-F1.5 | Nonrecursive `{{DATA_FILE}}`: missing → `[]`; present → loaded (clean control) | clean | `expression_recovery_data::env_reference_missing_and_present::env_file::{missing,present}` |
| R2-F1.6 | Nonrecursive `optional.{{DATA_EXT}}`: missing → `[]`; present → loaded (clean control) | clean | `expression_recovery_data::env_reference_missing_and_present::env_extension::{missing,present}` |
| R2-F1.7 | Recursive `%{{DATA_FILE}}` with `format: "yaml"`: missing → `[]`; present → loaded (clean control) | clean | `expression_recovery_data::env_reference_missing_and_present::recursive_env_file_explicit_format::{missing,present}` |
| R2-F1.8 | Literal `%optional.yaml`: missing → `[]`; present → loaded (clean control) | clean | existing `valid_request_recovers_only_a_miss::recursive_supported_extension::{missing,present}` passes |
| R2-F1.9 | Literal `%optional.xyz`: unsupported format in both (author ruling ER-2 retained) | clean | existing `invalid_request_fails_whether_or_not_present::recursive_unsupported_extension::{missing,present}` still fails with `dm.data.unsupported_format`; unit `an_authored_candidate_names_the_reference_as_written`; orchestrator CLI repro of `%optional.xyz` exits 1 |
| R2-F1.10 | Present-source format selection for rows 1–9 uses the resolved path | clean | present branch uses `plan_read(.., extension_format(&identity))` with the resolved-path identity; all 7 present cases load |
| R2-F1.11 | Passive call check: placeholder references reach execution, no early format rejection, no input reads | clean | `call_check.rs` `provable_format`/`format_of_path` is text-only and reads no input; a placeholder makes the extension unknown (`None`), so nothing is judged early; present cases reach execution and load |
| R2-F1.12 | Ordinary evaluator recovery classifier unchanged; repair is in the loader | clean | evaluator classifier unchanged; repair confined to the loader (`absent_candidates`) |
| R2-F1.13 | Lifecycle recovery consumes the same loader result; no independent format derivation | clean | `lifecycle/executor.rs` calls `is_recoverable_read_failure()` on the same loader error; no format logic in `lifecycle/` |
| R2-F1.14 | Missing-source diagnostic identity stays the authored reference for recursive refs (no invented resolved path) | fixed | `AbsentCandidate { path, authored }`: errors name the raw reference for recursive/unplanned refs; format comes from the interpolated payload |
| R2-F1.15 | `absent_candidates` raw-extension comment corrected | fixed | `absent_candidates` doc comment rewritten to state judgment by interpolated payload |
| R2-F1.16 | CLI regression for the recursive environment-interpolated missing source | fixed | `darkmatter-cli` L1 `recursive_env_reference_recovers_only_a_miss`; a mutation check that restored the raw-text path failed it and the 3 missing-side lib tests |
| R2-F1.S1 | Sweep: passive call check `format_of_path`/`provable_format` judging format from authored text | clean | text-only; never rejects a placeholder reference early (see R2-F1.11). Corner case: an empty variable in `{{X}}.yaml` reads as YAML statically but as no extension at runtime; both fail, so recovery is unaffected |
| R2-F1.S2 | Sweep: present-local read branch in `load.rs` | clean | format from the resolved path |
| R2-F1.S3 | Sweep: remote branch (`is_remote_url`, `fetch_remote_text`) | clean | format and fetched URL derive from the same URL text and cannot disagree |
| R2-F1.S4 | Sweep: `functions/fs.rs` (`absolute_fn`, probe recording), `schema_validation.rs`, `markdown/fs.rs` branches on `class().recursive` | clean | they make no format or applicability decision |
| R2-F1.S5 | Sweep: `functions/mod.rs` `file_extension` (dead code), `resolve_ctx.rs` raw text used only in a message | clean | no format decision |
| R2-F1.S6 | biscuit-file: a non-probing public way to read a reference's environment-expanded target text | fixed | new `FileReference::interpolated_payload`, which reuses the internal `interpolate`/`interpolated_url_string` (no second interpolator); 6 L1 tests in `resolution_context::interpolated_payload::*` and a doctest |

### Progress

- starting the work on 'R2-F1: Recursive missing-source format checks use unexpanded environment references' at 01:07:30
        - delegated to one foreground `rust-developer` subagent with an orchestrator-chosen design: expand the reference through `FileReference` and the captured context, not through a second interpolator
        - added `FileReference::interpolated_payload(&FileResolutionContext)` to **biscuit-file**. It is additive and non-probing, and it reuses the existing interpolation, so URLs keep an unset variable verbatim as their resolution does
        - `absent_candidates` now judges a recursive or unplannable missing reference by its interpolated payload and names the authored reference in errors. It falls back to the raw text only if interpolation fails, which cannot happen there because resolution would already have failed
        - added tests: a 7-case × missing/present matrix (`env_reference_missing_and_present`, 14 tests), one CLI regression, and 6 biscuit-file tests plus a doctest. A mutation check that restored the raw-text path failed the 3 missing-side bug cases and the CLI test
        - updated docs: `darkmatter/docs/topics/loading-data.md` (the extension is read after `{{VAR}}` expansion), `biscuit-file/docs/topics/file-references.md` (method table row), and `.claude/skills/biscuit-file/references/file-references.md`
        - the orchestrator reviewed the diff and reproduced the review's CLI scenario with the debug `md` binary: missing → exit 0, `[]`, no diagnostics; present → `["loaded"]`; `%optional.xyz` → exit 1
- work completed for 'R2-F1: Recursive missing-source format checks use unexpanded environment references' at 01:28:18

### Completion Report

- outcome: **complete**. Elapsed: about 24 minutes (01:04:52 → 01:29)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - R2-F1 (medium), fixed: 7 child rows fixed, 9 clean, plus 6 sweep rows (1 fixed: the biscuit-file API; 5 clean)
- open finding or child IDs: none
- no decision needed author confirmation: the review recommended this repair, so `human_review` is unchanged
- cross-OS: no Windows cross-check was run. The change only reads an extension from interpolated text using `Path`, adds no OS-specific code, and its tests use temp dirs and an injected env snapshot. CI's nightly Windows and WSL2 legs cover it

#### Changed files

- R2-F1: `biscuit-file/lib/src/file_reference/mod.rs`, `biscuit-file/lib/src/file_reference/resolve.rs`, `biscuit-file/lib/tests/l1/resolution_context.rs`, `biscuit-file/docs/topics/file-references.md`, `.claude/skills/biscuit-file/references/file-references.md`, `darkmatter/lib/src/markdown/compose/expression/data/load.rs`, `darkmatter/lib/tests/l1/expression_recovery_data.rs`, `darkmatter/cli/tests/l1/expression_recovery.rs`, `darkmatter/docs/topics/loading-data.md`

#### Final verification (run by the orchestrator after all changes)

| Command | Result | Verifies |
| --- | --- | --- |
| `just test` (darkmatter) | 16118 passed, 0 failed, 10 skipped; none ≥ 5 s | R2-F1, no regressions |
| `just lint` (darkmatter) | exit 0 | R2-F1 |
| `just test` (biscuit-file) | 1279 passed | R2-F1.S6 |
| `just lint` (biscuit-file) | exit 0 | R2-F1.S6 |
| root `just check-tier-coverage darkmatter` / `biscuit-file` | 0 stranded each | new tests are selected by L1 |
| debug `md compose` repro (temp dir) | missing `[]` exit 0; present `["loaded"]`; `%optional.xyz` exit 1 | R2-F1.1, R2-F1.9 |
