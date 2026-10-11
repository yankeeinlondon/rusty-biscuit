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
source_files_during_phase_2:
    - worktree/lib/src/pull_requests.rs
    - worktree/lib/src/list/wait/tests.rs
    - worktree/cli/src/commands/list/tests.rs
    - worktree/cli/src/commands/git_graph/tests.rs
    - worktree/cli/tests/list_table.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/worktree/list-remote.md
packages:
    - sniff
    - worktree
    - worktree-cli
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

## Phase 2

All in `worktree/lib/src/pull_requests.rs` (the other touched files only gained the new `status` field in `OpenPullRequest` test literals).

- `OpenPullRequest::status: PrCheckStatus` (sniff's enum, re-exported from `worktree::pull_requests`); stored as `failing|running|passed|unknown` via a `status_form` serde module (`status_name()` exposes the string). `SniffOpenPrSource` now calls `open_pull_requests_with_status` and copies `summary.status`.
- Format 7 (`PR_STORE_FORMAT_VERSION` 6 -> 7): all fields written explicitly. Formats 5 and 6 read through status-free structs (`StatuslessStoreFile` new for 6, `UncredentialedStoreFile` for 5, both using `StatuslessPullRequest`), so any `status` there is ignored and reads `unknown`. <=4 and >=8 remain misses.
- Leniency: `#[serde(default, with = "status_form")]` reads the value as a JSON `Value`; absent/null/wrong type/unrecognized -> `unknown` for that PR only. A repeated `status` key is still a miss because serde's derive rejects duplicate fields (tested). The `#[serde(default)]` smell is intentional here and is the only one on this struct.
- Worker/publish path: `refresh` is unchanged; it already publishes list + statuses in one atomic write, publishes empty lists, writes nothing on failure, and rechecks origin. Enrichment failures inside sniff yield `unknown` statuses with a complete list, so they publish. Verified by `statuses_round_trip_and_a_later_answer_replaces_them` (replace, failed refresh keeps old, empty clears). No lock/deadline/publication-ID change.
- Duplicate numbers: the store accepts two records with one number (matrix cell kept); dedupe is Phase 3's coverage job.

Requirement -> test (all `worktree pull_requests::tests::`):

- format 7 matrix incl. control row, format_version 7 valid / 8 miss, every status shape (absent, null, number, bool, `[]`, `["passed"]`, `{}`, `""`, bogus, wrong case, each valid name, duplicate key = miss, one wrong element among valid, every element wrong): `the_store_reader_walks_the_input_robustness_matrix`
- formats 5/6 ignore any status and keep their other rules (bad element, duplicate key, trailing, missing credentials, anonymous credentials kept): `formats_5_and_6_read_as_unknown_status_whatever_a_pr_spells`
- format 5 stray credentials / format 8 miss: `a_format_5_answer_is_served_with_unknown_credentials_and_a_future_format_is_a_miss`
- read/write/read round trip and replacement: `statuses_round_trip_and_a_later_answer_replaces_them`

Gates: `just test` (1099 passed) and `just lint` in `worktree/` pass. The production `SniffOpenPrSource` mapping is a field copy and is not exercised against a live provider (no network in tests). Smell grep: no `unwrap_or_default()`/`filter_map(as_str)` added on the identity path. Skill updated: `.claude/skills/worktree/list-remote.md`. L2 not run this phase (no L2 test touches the store format directly beyond `PR_STORE_FORMAT_VERSION` constants).
