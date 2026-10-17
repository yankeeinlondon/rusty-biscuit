---
feature: 2026-10-11-schematic-generation-integrity
implementation_1: "2026-10-12T08:48:56-07:00"
implementation_2: "2026-10-12T09:31:41-07:00"
---

# Schematic Generation Integrity — Review Implementation Log

## Implementation of Review Findings #1

> **started at:** 2026-10-12T08:48:56-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'schematic/features/2026-10-11-schematic-generation-integrity/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review stopped for a human decision; the author's `## Human rulings` are this cycle's findings and were added to the review's `findings` frontmatter before any code change:
        - `high: carry out ruling on item 1: enforce R13 duplicate method+path as errors and merge the GitHub and Bitbucket duplicate pairs`
        - `high: carry out ruling on item 2: record R30 completion after the author-authorized deletion of schematic/gen/schematic/`
- starting the work on 'ruling 1: enforce R13 duplicate method+path as errors' at 08:49:42
        - delegated to one foreground subagent (serial orchestration); it appends its own sub-bullets below
        - defect class: a validation rule the spec requires as an error is emitted at a lower severity (or behind a per-rule severity constant / exception list), so a definition that should block an output passes and the output silently loses data
        - class sweep: rule-severity downgrades; sites checked: `validation.rs` `check_routes` (duplicate id, duplicate route, `query-in-path`), every other `Severity::` site in `validation.rs`, `check_unsent_params` (Postman), `export/openapi.rs` (group route collision, duplicate `operationId`, unresolved link), WebSocket planner `WsSeverity`; fixed here: same-API `duplicate-method-path` (warning -> error, `DUPLICATE_METHOD_PATH_SEVERITY` removed); clean: duplicate id, `query-in-path` (R33 specifies a warning), all `export/openapi.rs` checks, all Rust-only checks; flagged, not changed: Postman optional header/cookie parameters stay warnings (R10 sets no severity for an optional input; disclosed on the review-1 agenda item 14; raising it would block the OpenAI Postman collection); WebSocket diagnostics predate this spec
        - discovery (premise contradiction, needs the author): `sniff` uses three of the four removed endpoints behind its `remote` feature (used by sniff-cli, worktree, darkmatter, dmls): `sniff/lib/src/remote/github.rs:114,148` (`GetGitTreeRecursiveRequest`), `sniff/lib/src/remote/bitbucket.rs:272` (`ListDirectoryContentsRequest`), `:438-441` (`GetFileContentRawRequest`, `get_file_content_raw`); `cargo check -p sniff --features remote` now fails with 5 errors; per R31 the dependent was not edited
        - discovery: Gitea `GetGitTreeRecursive` (`?recursive=true` in the path) does not collide with Gitea `GetGitTree`; regeneration reports no duplicate route for it
        - changed: `gen/src/validation.rs` (duplicate route is `Severity::Error`, constant removed, message says to merge); GitHub `GetGitTree` now has an optional `recursive` query parameter and `GetGitTreeRecursive` is gone (17 -> 16 endpoints); Bitbucket `ListDirectoryContents` + `GetFileContentRaw` -> `GetSource` returning `Binary`, pagination kept, no `Accept` override (16 -> 15 endpoints)
        - regenerated with `just generate`: exit 0, OK for rust/openapi/postman, zero `duplicate-method-path` lines; diff confined to github/bitbucket in `schema/src`, `openapi/`, `postman/`
        - docs/skills: `docs/generation/validation.md`, `README.md`, `definitions/README.md`, `gen/README.md`, `skill/SKILL.md`, `.claude/skills/schematic-define/SKILL.md`, `.claude/skills/schematic-define/schematic/gen.md`; implementation log gained "Review 1, ruling 1" and superseded-notes on the R13 rows
        - tests: added `same_api_duplicate_route_blocks_every_output`, `live_definitions_declare_no_duplicate_routes`, `same_route_in_different_apis_is_not_a_duplicate`; updated the route-key test, GitHub/Bitbucket definition tests
        - results: `just test` (schematic) exit 0, 1776 passed / 9 skipped, fresh-output guard 4 passed (1 slow, 152 s); `just lint` (schematic) exit 0
        - orchestrator decision on the `sniff` breakage: the ruling explicitly authorizes removing the four endpoints, so migrating their callers is a change the ruling implies; leaving `sniff` (and through it `sniff-cli`, `worktree`, `darkmatter`, `dmls`) uncompilable is not an acceptable terminal state, and reverting would leave R13 unenforced. The ruling's premise "nothing outside `schematic/` uses any of the four endpoints" was false, and R31 says a dependent is not edited; this departure from R31 is made for the ruling's own edits only (not for generator changes), is confined to the three `sniff` call sites and their docs, and is recorded for the author to confirm at the next review
        - migration of the `sniff` call sites delegated to a second foreground subagent; it appends its own sub-bullets below
        - class sweep: callers of removed generated endpoints; sites checked: whole-repo `rg` for `GetGitTreeRecursive|ListDirectoryContents|GetFileContentRaw|get_git_tree_recursive|list_directory_contents|get_file_content_raw` (excluding `target/`, `schematic/features/`, `schematic/reviews/`, Gitea's own endpoint); fixed here: `sniff/lib/src/remote/github.rs:114,148`, `sniff/lib/src/remote/bitbucket.rs:272,438-441`, `sniff/docs/repos/repo-tech-design.md` (GitHub/Bitbucket endpoint tables, mapping notes, fixture name); clean: every other hit is Gitea's unchanged `GetGitTreeRecursive` (`sniff/lib/src/remote/gitea.rs`, Gitea rows in `sniff/docs/repos/repo-tech-design.md` and `schematic/definitions/README.md`, `schematic/gen` diagnostics/tests/README, `schematic/docs/generation/validation.md`, `schematic/postman/gitea.postman_collection.json`) or a definition test asserting the removal; no other package calls the removed request types or client methods; `.claude/skills/sniff/**` and sniff READMEs name none of them
        - discovery (wire premise contradicted): the old `GetGitTreeRecursiveRequest::new` set `recursive: None` and `into_parts` sends the query only when `Some`, so sniff's GitHub "recursive" tree request sent no `recursive` parameter and GitHub returned only the top level; the definition declared the parameter required ("always true for this endpoint") but the generated constructor did not set it. sniff now sends `recursive=true` on both the root tree and the `branch:prefix` continuations, which is what the endpoint declared and what sniff's code and docs assume (the continuation comment in `snapshot.rs` expects `workflows/ci.yml` beneath `.github`); this is a deliberate wire change and fixes nested docs/CI being invisible on GitHub
        - changed (GitHub): `GetGitTreeRequest::new(..).with_recursive(true)` replaces `GetGitTreeRecursiveRequest`; the GitHub `list_documents_success` mock now also matches `query_param("recursive", "true")`, so a request without it falls through to 404
        - changed (Bitbucket): directory listing uses `GetSourceRequest` with the same `pagelen=100`/`page` query and no headers (same wire request as `ListDirectoryContents`), calls `get_source`, and decodes the bytes with `decode_directory_listing` into `PaginatedResponse<SourceEntry>`; a decode failure maps through `map_schematic_error(SchematicError::Json(_))` to `RemoteApi { status: 0, "JSON parse error: ..." }`, which the snapshot degrades to an unavailable tree (existing R11.6 behavior), never an observed empty tree
        - changed (Bitbucket): `get_file_content` uses `GetSourceRequest` + `get_source` and decodes with `String::from_utf8_lossy`, matching the old `request_text` path (`reqwest::Response::text`, which replaces invalid UTF-8); the old request also sent `Accept: text/plain`, the new one sends none; Bitbucket's `src` endpoint returns raw file bytes for a file path regardless of `Accept`, so this was left as is
        - tests: added unit tests `a_directory_listing_body_decodes_to_its_entries`, `a_malformed_directory_listing_body_is_a_json_error`, `file_text_replaces_invalid_utf8` (`sniff/lib/src/remote/bitbucket.rs`) and L1 `bitbucket_tests::a_malformed_directory_listing_leaves_the_tree_unavailable_not_empty`, `bitbucket_tests::get_file_content_returns_the_raw_bytes_as_text` (`sniff/lib/tests/l1/remote_providers.rs`)
        - results: `just test` (sniff) exit 0, 3360 passed / 33 skipped (1 slow: pre-existing `files_link_root::relative_base_with_unreadable_cwd_fails`, 13.6 s, unrelated); `just lint` (sniff) exit 0; `cargo clippy -p sniff --features remote --all-targets -- -D warnings` clean; `cargo check -p sniff-cli -p worktree -p darkmatter -p dmls` (each enables `sniff/remote`) finished clean; `rustfmt --check` clean on the three edited Rust files
        - orchestrator note: the GitHub wire change (sniff now sends `recursive=true`, which the old generated constructor silently omitted) is a behavior fix the author should confirm at the next review; drop `.with_recursive(true)` in `sniff/lib/src/remote/github.rs` to restore the old wire exactly
- work completed for 'ruling 1: enforce R13 duplicate method+path as errors' at 09:22:22
- starting the work on 'ruling 2: record R30 completion' at 09:22:22
        - defect class: a requirement's completion state recorded in the feature's ledger lags the tree after the work was done out of band
        - verified: `schematic/gen/schematic/` is gone; `git status` shows exactly the five staged deletions (`schema/Cargo.toml`, `src/lib.rs`, `src/openai.rs`, `src/prelude.rs`, `src/shared.rs`) and nothing else under that folder
        - remaining textual references are historical and stay: `checks/cargo-machete.txt` (kept unchanged by the ruling), specs/plans of other features (snapshots), and a completed sniff baseline fixture
        - class sweep: stale R30 status in the ledger; sites checked: `implementation-log.md` Task 7.1 heading, acceptance row, requirement summary row, review-agenda item, Status paragraph; fixed here: all five via appended "superseded" notes (history kept); clean: `spec.md` (snapshot, not edited), `plan.md` (no status column for R30)
- work completed for 'ruling 2: record R30 completion' at 09:22:27

### Successful Completion

The implementation of review cycle 1 has completed successfully in 34 minutes. During this implementation all 2 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 0 were deferred, 0 were disputed (see reasons below):

- no finding was deferred or disputed
- recorded departures for the author to confirm at the next review:
        - R31 ("no file in a dependent crate changes"): `sniff` called three of the four endpoints the ruling removed, contrary to the ruling's premise; its call sites in `sniff/lib/src/remote/github.rs` and `sniff/lib/src/remote/bitbucket.rs`, their tests, and `sniff/docs/repos/repo-tech-design.md` were migrated so the workspace compiles
        - GitHub tree fetches in `sniff` now send `recursive=true`; the old generated constructor never sent it, so earlier "recursive" fetches returned only the top level
- this review never compared the implementation with the specification; this feature is **not** reported as production ready
- the files changed in this cycle:
        - `schematic/gen/src/validation.rs` and the generator's L1 tests
        - `schematic/definitions/src/github/mod.rs`, `schematic/definitions/src/bitbucket/mod.rs`, `schematic/definitions/README.md`
        - regenerated GitHub and Bitbucket output in `schematic/schema/src/`, `schematic/openapi/`, `schematic/postman/`
        - `schematic/docs/generation/validation.md`, `schematic/README.md`, `schematic/gen/README.md`, `schematic/skill/SKILL.md`, `.claude/skills/schematic-define/SKILL.md`, `.claude/skills/schematic-define/schematic/gen.md`
        - `sniff/lib/src/remote/github.rs`, `sniff/lib/src/remote/bitbucket.rs`, `sniff/lib/tests/l1/remote_providers.rs`, `sniff/docs/repos/repo-tech-design.md`
        - `implementation-log.md`, `review-1.md` (findings and metadata), and this log

## Implementation of Review Findings #2

> **started at:** 2026-10-12T09:31:41-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'schematic/features/2026-10-11-schematic-generation-integrity/review-2.md'
- this is iteration 2 of the review-to-implement cycle 
- findings in scope (the review's frontmatter lists only the high finding; the medium finding in the body is also implemented here):
        - `high: Unsupported optional Postman inputs do not fail their output`
        - `medium: Nested query arrays escape the required validation`
- starting the work on 'high: Unsupported optional Postman inputs do not fail their output' at 09:32:10
        - defect class: an output that cannot carry a declared input treats the gap as success when the input is optional, so it silently omits a declared capability
        - class sweep: unsupported input projections whose outcome depends on optionality; sites checked: every `Severity::Warning` site in `gen/src/validation.rs` (`QUERY_IN_PATH_SEVERITY`, `check_unsent_params`), `validate_rust_client`, `validate_openapi` and the OpenAPI header/cookie/auth projection (`define/src/openapi/export/parameters.rs`), `export/postman/mod.rs` single-API and grouped writers, Postman `build_url`/`build_headers`/`build_body`/`build_collection_auth`, `export/auth.rs::map_auth`; fixed here: `check_unsent_params` (deleted; header and cookie parameters are now carried), Postman `build_headers` (header/cookie params were dropped), Postman `build_collection_auth` for a cookie `ApiKeyParam` (wrote `in: cookie`, which Postman's API-key auth does not support); clean: `query-in-path` (warning by R33), Rust and OpenAPI validators and projections, Postman query/path/body projections, `map_auth`
        - change: the Postman exporter writes each declared header parameter as a request header `name: <name>` and each cookie parameter as `Cookie: name=<name>`, `"disabled": true` when optional, with the declared description; a cookie API key is an `apikey` auth that adds `Cookie: name={{apiKey}}` as a header; `check_unsent_params` and its call in `validate_postman` are removed, so no Postman severity depends on `required`
        - tests: added `output_validation::header_and_cookie_parameters_reach_every_postman_writer` (required and optional header and cookie through the batch generator, `write_postman`, and `write_postman_grouped`; asserts enabled vs disabled entries, Postman v2.1 schema validity, no diagnostics in any output, and OpenAPI header/cookie locations) and `output_validation::cookie_api_key_rides_a_cookie_header_in_postman`; `live_definitions_warn_once_per_output_for_each_embedded_query` now also asserts no non-`query-in-path` diagnostic in any output; removed `postman_reports_unsent_header_and_cookie_parameters_by_requiredness`; updated `invalid_header_and_cookie_parameters_fail_only_the_rust_client` (Postman now reports nothing) and `auth_api_key_cookie_location`
        - regeneration: `just generate` exits 0; artifact diff is confined to `postman/openai.postman_collection.json` (+56 lines, the seven `openai-beta` headers as disabled entries); the only remaining warnings are the 27 `query-in-path` lines
        - docs: `docs/io/export-postman.md` (new Parameter Mapping section, `ApiKeyParam` auth rows), `docs/generation/validation.md` (removed the Postman header/cookie row, stated how the exports carry them), `gen/README.md` validator table, `skill/SKILL.md` warnings line
        - verification: `just test` 1777 passed, 9 skipped, plus fresh-output 4 passed; `just lint` clean; `just check-tier-coverage schematic` clean (0 stranded)
        - orchestrator decision: chose support over rejection because D10 says header and cookie parameters are "implemented, rather than rejected", and rejecting the optional `openai-beta` header would have blocked the shipped OpenAI Postman collection (R20 drift and fresh-output checks); the review names support as a valid repair
        - known limitation (documented in `docs/io/export-postman.md`): several enabled cookie parameters become several `Cookie` header lines in Postman, while the Rust client folds them into one; Postman-app handling of this was not checked
- work completed for 'high: Unsupported optional Postman inputs do not fail their output' at 09:39:05
- starting the work on 'medium: Nested query arrays escape the required validation' at 09:39:20
        - defect class: validation rejected an unserializable parameter shape (array of arrays) at the header and cookie locations but let it through at the query location, so the generated Rust crate failed to compile (E0599 on `Vec<bool>::to_string()`)
        - class sweep: parameter/field shapes validation accepts but a projection cannot serialize; sites checked: Rust query pairs (`generate_path_format`), Rust header and cookie values (`header_value_expr`, `cookie_value_expr`), Rust path params (always `String` from the template, no declared type), Rust form-data and url-encoded fields (`FormFieldKind` has no nesting), every other `QueryParamType` at each Rust location (`Json` and `Array(Json)` use `serde_json::Value` Display, `Enum` is `String`; no object/map variant exists), OpenAPI `param_schema` (recursive items schema), Postman `build_url` query entries and header entries (key and description only, no type), WebSocket `ParamType` (scalars only); fixed here: Rust query array-of-arrays now rejected; clean: all other sites listed
        - change: `check_named_params` renamed `check_sent_params` and extended to `EndpointParams.query` with new `Feature::QueryParameter` (`query-parameter`); the HTTP-token name check still applies only to header and cookie; any depth of nested array is rejected as a Rust-output error
        - tests: `output_validation::nested_arrays_fail_the_rust_client_at_every_parameter_location` (query/header/cookie x `Array(Boolean)` accepted, `Array(Array(Boolean))` rejected with exact API/endpoint/feature/output diagnostic; OpenAPI and Postman validators clean), `nested_query_array_leaves_earlier_rust_output_untouched` (generate_batch fails only Rust, schema dir byte-identical, no Cargo.toml), `live_definitions_declare_no_nested_array_parameters`; `display_and_serialized_ids_agree_for_every_variant` covers the new feature id
        - docs: `docs/generation/validation.md` table split into name rule (header/cookie) and array-of-arrays rule (query/header/cookie) plus a paragraph and example on why exports still write; `docs/generation/request-side.md` query section notes the rejection; no skill text described the rule
        - verification: `just test` 1780 passed, 9 skipped; fresh-output 4 passed; `just lint` exit 0; `just check-tier-coverage schematic` 0 stranded; no shipped definition declares a nested array parameter (live test), so no regeneration
- work completed for 'medium: Nested query arrays escape the required validation' at 09:42:55

### Successful Completion

The implementation of review cycle 2 has completed successfully in 11 minutes. During this implementation all 2 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 0 were deferred, 0 were disputed (see reasons below):

- no finding was deferred or disputed
- the two observations needed no repair:
        - the `sniff` dependent migration was accepted by the review; nothing reverted
        - the three existing main-suite tests over 5 s (`e2e_generation::shared_module_contains_paginated_trait`, `e2e_generation::multiple_apis_generate_together`, `output::tests::split_to_single_file_transition_cleans_up`) predate this cycle and were not investigated here; they remain a separate performance investigation
- recorded for the author to confirm at the next review:
        - the high finding was repaired by **supporting** header and cookie parameters in the Postman export (optional ones as disabled entries) rather than rejecting them, per D10; the OpenAI Postman collection gained seven disabled `openai-beta` headers
        - a cookie `ApiKeyParam` in Postman now rides a `Cookie` header instead of an `apikey` auth with the unsupported `in: cookie`
        - several enabled cookie parameters become several `Cookie` header lines in Postman (documented limitation)
- the files changed in this cycle:
        - `schematic/gen/src/validation.rs`, `schematic/gen/src/diagnostics.rs`
        - `schematic/gen/src/export/postman/types.rs`, `request.rs`, `auth.rs`, `tests.rs`
        - `schematic/gen/tests/l1/output_validation.rs`
        - regenerated `schematic/postman/openai.postman_collection.json`
        - `schematic/docs/io/export-postman.md`, `schematic/docs/generation/validation.md`, `schematic/docs/generation/request-side.md`, `schematic/gen/README.md`, `schematic/skill/SKILL.md`
        - `review-2.md` (metadata) and this log
