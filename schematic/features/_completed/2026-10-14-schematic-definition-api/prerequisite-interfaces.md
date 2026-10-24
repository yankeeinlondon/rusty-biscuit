---
phase: 1
---

# Prerequisite interface map

Refreshed against execution-tree source on 2026-10-14. No prerequisite hold remains. The generation-integrity, runtime-envelope, and pagination predecessors are delivered; pagination review 2 reports production readiness within its scope with no findings. Its observation about copying traversal history is retained as a performance observation, not reopened design work.

## Review outcomes that govern migration

- Generation-integrity review 1 author ruling: identity remains `(method, path)`; no selectors or fixed-value query parameters. Merge the Gitea recursive route and seven OpenAI beta routes into their twins with optional caller query controls, after the OpenAI schema comparison. Reject GET request bodies when parameter-location validation lands.
- Runtime-envelope's delivered surface is `EndpointSpec`, API `cross_origin`, `request_id_header`, and `secret_names`; `ParamDef::secret` was not delivered. ElevenLabs ListVoices page_size/page_token inputs are part of the baseline.
- Pagination review 1 Human ruling 1: reject first-page-opened sessions with partitions; retain separately opened shared sessions with rotating handles and unpartitioned first-page sessions.
- Pagination review 1 Human ruling 2: paging applies the global JSON callback before item/continuation extraction and skips the typed endpoint callback; ordinary sends apply both.
- Pagination review 1 Human ruling 3: complete failure evidence survives repeated calls without network work; optional explicit retry is available for every error category; failed retries replace the evidence without redelivering earlier pages. Automatic retry is outside this migration.

## Concrete interface map

| Concern | Delivered source and public interface |
| --- | --- |
| Output validation | `gen/src/validation.rs`: `validate_rust_client`, `validate_openapi`, `validate_openapi_registry`, `validate_postman`; endpoint uniqueness uses `EndpointUniquenessKey` / `endpoint_uniqueness_key`. `QUERY_IN_PATH_SEVERITY` remains Warning until all migrations succeed. |
| Diagnostics | `gen/src/diagnostics.rs`: `ValidationDiagnostic`, `Severity`, `Output`, `Feature`, `has_errors`; preserve structured output-specific errors. |
| Runtime | `runtime/src/endpoint.rs`: `Api`, `EndpointSpec`, `ErrorBody`, `RequestParts`, `RequestBody`, `FormPart`; `runtime/src/lib.rs`: `ABI = 4`. |
| Origin policy | `define/src/cross_origin.rs`: `CrossOrigin`, `danger_allow_any`; API policy lowers to runtime `OriginPolicy` (`runtime/src/origin.rs`). Preserve exact-origin validation and tighten-only caller behavior. |
| Redaction/request ID | `define/src/types.rs`: `RestApi::{request_id_header,secret_names}`. No parameter secret field; preserve existing lists and runtime metadata redaction. |
| Pagination model | `define/src/pagination/{collection,strategy}.rs`: `Collection`, `Pagination`, `PaginationException`, `Strategy`, `PageStrategy`, named four-kind strategy structs. `RestApi::{default_pagination,pagination_exceptions}`. |
| Resolution | `define/src/pagination/resolve.rs`: `resolve(&RestApi,&Endpoint)->Resolution`, `Resolution::{NotAList,Unpaginated,Paginated}`, `ResolutionOrigin::{Exception,ApiDefault,Unauthored}`, `caller_knobs`, `strategy_knobs`. Only GET explicit Vec descriptors infer a list. |
| Adoption | Same module: `param_adoptions`, `adopted_params`, `ParamAdoption`, `AdoptionOutcome`, `ManagedRole`, `ParamLocation`. Preserve distinctions between absent knobs and required managed inputs without defaults. |
| Validation | `define/src/pagination/validate.rs`: `validate_strategy`, `validate_pagination`, `PaginationDiagnostic`, `PaginationError`, including `PagerManagedRequiredWithoutDefault`, `AdoptionWithoutKnob`, and `partitioned-first-page-session`. |
| Pager API | `runtime/src/pager/`: `Paginated`, `Pager`, `Pages`, `SessionScope`, `AdjustablePageSize`, `Sessioned`, `Sessionless`, `Renewable`. `Paginated::Failure: Sync`, no Clone bound; `Pages::retry` and `Items::retry`. `fetch_page` returns `Page<Item>`. |
| Import overlay | `gen/src/import_pipeline.rs`: `ImportOptions::overlay`; CLI `import --overlay <RUST_PATH>` requires `--definitions-out`; `gen/src/definition_gen.rs::parse_overlay_path` validates a plain Rust path and emits generic `fn(RestApi)->RestApi` call. `definitions/src/openai_pagination.rs::apply` stays outside importer ownership. |
| Isolated compilation | `gen/tests/common/fresh_output.rs`: `FreshCrate::{generate,generate_with_runtime,add_test_file,cargo_check,cargo_check_tests,nextest_run}`; compiler artifacts live outside checkout and commands are offline. `gen/tests/l1/fresh_output_compile.rs` exercises generated output and guards child-test registration. Use existing separate `just test-fresh-output` guard. |
| Registry | `definitions/src/registry/mod.rs`: ordered IndexMap-backed `SchemaRegistry::register`, OpenAPI 3 settings and nested-schema extraction. `registry/lookup.rs`: `get_registry`, `registry_key_for`, `get_registries_for_module`; module pairs share the registry today. |

## Six R27 retirement entries

| R27 entry | Actual existing evidence | Required replacement/disposition |
| --- | --- | --- |
| 1. Cross-origin/no-GraphQL surface | `define/tests/cross_origin_policy.rs::endpoints_have_no_cross_origin_override_and_no_graphql_export` | Replace override assertion with R14 coverage; preserve no-GraphQL assertion until the later GraphQL feature. |
| 2. Endpoint literal | `define/tests/endpoint_literal_surface.rs::endpoint_and_params_literals_compile_unchanged_and_behave_the_same` | Required-fact builder compile coverage plus R9/R10 equality. |
| 3. EndpointParams literal | Same test (one function intentionally covers both literal shapes) | Update field set when legacy metadata is removed; preserve `Default` / `new()` allow-list coverage. |
| 4. Unchanged literal lines | Pagination predecessor implementation-log comparison, rather than a live named test | Retire constraint only when builder migration starts; replace with R24 definition/artifact baselines. |
| 5. Legacy inert metadata | `define/tests/pagination_tables.rs::legacy_pagination_metadata_does_not_change_resolution`; `gen/src/codegen/pagination/tests.rs::legacy_pagination_metadata_does_not_change_the_output`; `gen/tests/l1/e2e_generation.rs::legacy_pagination_metadata_emits_no_pager` | Retire with legacy fields/types; retain resolution and emitted-control evidence. |
| 6. Exceptions tables | `define/tests/pagination_tables.rs`, `define/tests/pagination_partitions.rs`, `gen/tests/l1/pagination_validation.rs`; `definitions/tests/pagination_coverage/{main,forges,openai,others}.rs` | Replace table mechanics with endpoint authorship while retaining exhaustive resolutions, caller knobs, invalid siblings, and the exact 32-item verification-task list. |

Additional safety nets retained: `gen/tests/l1/import_overlay.rs::reimporting_the_vendored_openai_document_reproduces_the_committed_definition`, generic overlay validation tests, `schema/tests/{pagination,compile_fail}.rs`, runtime retry/session/hook tests, and generated fixture child registration guard. The canonical area test recipe runs separate fresh-output and runtime compile-fail guards; doctests require `just doctest`. `check-drift` already uses nextest; `test-e2e` must not be invoked unchanged because it uses cargo test.
