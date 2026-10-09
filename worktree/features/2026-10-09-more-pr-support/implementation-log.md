---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-modern-config/worktree/features/2026-10-09-more-pr-support/spec.md"
plan: "worktree/features/2026-10-09-more-pr-support/plan.md"
implemented_by: "claude/sonnet"
started_phase: 1
source_files_during_phase_1:
    - sniff/lib/src/remote/check_status.rs
    - sniff/lib/src/remote/focused.rs
    - sniff/lib/src/remote/blocking.rs
    - sniff/lib/src/remote/mod.rs
    - sniff/lib/tests/l1/pr_check_status.rs
    - sniff/lib/tests/l1/main.rs
    - sniff/lib/tests/fixtures/remote/graphql_check_status/success.json
    - sniff/lib/tests/fixtures/remote/graphql_check_status/passed.json
    - sniff/lib/tests/fixtures/remote/graphql_check_status/partial_errors.json
    - sniff/lib/tests/fixtures/remote/graphql_check_status/request_wide_errors.json
    - sniff/lib/tests/fixtures/remote/graphql_check_status/permission_failure.json
    - sniff/lib/tests/fixtures/remote/graphql_check_status/rate_limit.json
    - sniff/lib/tests/fixtures/remote/graphql_check_status/changed_head.json
    - sniff/lib/tests/fixtures/remote/graphql_check_status/null_rollup.json
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - sniff
---

# Implementation Log for 2026-10-09-more-pr-support (4 phases)

## Phase 1

Rulings: R1-R10 taken at the plan's stated defaults (no author objection recorded; headless run).

Implemented in `sniff` (no worktree code touched; `PrSummary` gaining a field compiles unchanged there):

- `PrCheckStatus { Failing, Running, Passed, Unknown }` and `PrSummary::status` in `remote/blocking.rs`.
- `remote/check_status.rs`: pure rollup-count mapping (`status_from_rollup`), batch query/variables builder, and a strict (duplicate-key-rejecting) response reader.
- `FocusedProviderClient::check_statuses` (GitHub + selected token + nonempty list; <=100 aliased `pullRequest(number:)` per POST; head SHA must equal the REST `source_head_sha`; any failure stops later batches, never errors) and `graphql_endpoint` (`api.github.com/graphql`, or `<origin>/api/graphql` for `/api/v3/`; always the REST base's host).
- `fetch_json` split into `send` (GET or POST, same policy, deadline, credential selection and `SentLog` evidence) plus JSON parsing; REST behavior unchanged.
- Opt-in public API: `open_pull_requests_with_status` / `open_pull_requests_with_status_with`. `open_pull_requests[_with]` are unchanged in cost and report `Unknown`.
- One `Instant` deadline (existing `run_with_deadline`) is shared by REST pages and GraphQL; enrichment errors are swallowed inside the operation, so the complete REST list is returned.

Requirement -> test mapping (all in `sniff/lib/tests/l1/pr_check_status.rs` unless noted; fixtures in `sniff/lib/tests/fixtures/remote/graphql_check_status/`):

- every evidence-table row / unknown case: `check_status::tests::*` (unit)
- 1 / 100 / 101 PRs -> 1 / 1 / 2 requests: `requests_are_one_batch_up_to_100_prs_and_two_for_101`
- anonymous / unsupported provider / empty list -> no request: `an_empty_list_anonymous_access_and_other_providers_send_no_graphql`
- ordinary query unchanged: `the_ordinary_query_sends_no_graphql_and_reports_unknown`
- request-wide failure stops later batches, earlier statuses kept: `a_request_wide_failure_stops_later_batches_and_keeps_earlier_statuses`; failure kinds (403 permission, 429, RATE_LIMITED body, null repository, 500, non-JSON, trailing content, duplicate key): `failures_leave_the_complete_list_with_unknown_statuses`
- per-PR errors, changed head, null rollup: `per_pr_errors_*`, `a_changed_head_or_null_rollup_is_unknown`
- deadline after a complete list keeps it: `a_deadline_after_the_complete_list_keeps_the_list`; failed REST list still an error: `a_failed_list_is_still_an_error_*`
- no provider text in query, bearer token, variables: `the_query_carries_no_provider_text_*`; credential folding: `each_pr_gets_the_status_*`
- GraphQL endpoint derivation (github.com vs self-hosted, never cross-host): `focused::graphql_endpoint_tests`
- input robustness matrix with control row: `the_graphql_reader_walks_the_input_robustness_matrix`

Gates: `just test` and `just lint` in `sniff/` pass (3355 tests). Tests are in the declared `l1` binary under the `remote` feature (already enabled by CI for this binary's neighbors).

Open item: the GraphQL field names (R7) were written from knowledge of GitHub's schema and could not be checked against the live reference from this headless session; the saved fixtures follow the same names. Worth a doc-read confirmation before Phase 4 docs.

Departure: a duplicate key or trailing content in a GraphQL body is treated as request-wide (stops later batches) rather than "that PR unknown"; the list is kept either way.
