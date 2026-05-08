---
ready: false
agent: codex
model: ""
---

# Review 1

## Findings

### High: Artificial Analysis attribution is missing from generated/public artifacts

The spec calls out the API terms requirement that all usage include attribution to
<https://artificialanalysis.ai/>, and repeats that this should surface in rustdoc.
The definition module includes the attribution text in
`schematic/definitions/src/artificial_analysis/mod.rs`, but the generated client
module does not carry it forward. `schematic/schema/src/artificial_analysis.rs`
starts with only the generated client headings and descriptions for
`ArtificialAnalysisData` and `ArtificialAnalysisCritPt`; there is no attribution
notice in the generated rustdoc. The committed OpenAPI/Postman artifacts likewise
describe the APIs but do not include attribution text.

This means the public generated crate and exported API artifacts can be consumed
without ever exposing the required attribution notice. For a terms-driven
requirement, that is a production blocker even though the source definition module
has the text.

Suggested fix: add an attribution field to the API description path that is emitted
into grouped generated module docs and exported artifacts, or include the
attribution sentence directly in the `RestApi.description` values for both
Artificial Analysis definitions if that is the current generator-supported path.
Then regenerate `schematic/schema/src/artificial_analysis.rs`,
`schematic/openapi/artificial_analysis.json`, and
`schematic/postman/artificial_analysis.postman_collection.json`.

Verification level: documentation/public artifact requirement. Current strongest
verification is manual source inspection; no test asserts the generated rustdoc or
artifacts contain the attribution. Add an artifact/content regression test.

### High: Generated client behavior has no provider-specific integration coverage

The Artificial Analysis implementation has good definition-level unit tests in
`schematic/definitions/src/artificial_analysis/mod.rs`, but those tests stop at
metadata: endpoint counts, paths, auth declaration, query-param presence, and
registry completeness. They do not exercise the generated client at runtime.

The user-observable behavior of this feature is the generated HTTP client:

- Data API requests must require `ARTIFICIAL_ANALYSIS_API_KEY` or explicit API key
  auth and send it as `x-api-key`.
- `include_categories` endpoints must serialize the expected query string.
- `EvaluateCritPtRequest` must serialize `CritPtEvaluateBody`, including arbitrary
  `generation_config` and optional/null-skipped `batch_metadata`.
- Both generated clients must call the expected paths against the shared base URL.

The generated code appears to implement these behaviors, but there is no
Artificial Analysis integration test under `schematic/schema/tests/` or
`schematic/gen/tests/` that runs the generated client against a mock HTTP server
and asserts the actual method, path, query, headers, and JSON body. Existing
generator-wide query/auth tests reduce risk, but they are not tied to this
provider's two shared-module clients.

Suggested fix: add a provider-specific generated-client integration test, similar
in spirit to `schematic/schema/tests/huggingface_client.rs`, using a local mock
server. Cover at least one data endpoint with `include_categories=true`, one data
endpoint without params, missing-auth error behavior, explicit `.api_key(...)`
header injection, env fallback for `ARTIFICIAL_ANALYSIS_API_KEY`, and
`EvaluateCritPtRequest` body serialization with nested JSON values.

Verification level: Level 1 in-process/runtime integration is appropriate for this
REST-client feature. Current strongest provider-specific verification is Level 1
definition metadata only, not runtime HTTP behavior. The terminal Level 2/Level 3
categories are not applicable because this feature has no terminal rendering or OS
keyboard behavior.

## Coverage Notes

The implementation does include:

- Two `RestApi` definitions sharing `module_path = "artificial_analysis"`.
- The expected 6 data endpoints and 1 CritPt endpoint.
- `x-api-key` auth with `ARTIFICIAL_ANALYSIS_API_KEY` in both legacy `env_auth`
  and modern `env_mapping`.
- `RateLimitError` as documentation-only and not registered in the OpenAPI registry.
- Generated schema, OpenAPI, and Postman artifacts for the shared module.

Remaining ergonomic/performance notes:

- The generated `ArtificialAnalysisData` and `ArtificialAnalysisCritPt` clients
  duplicate a large amount of auth/helper code in the shared module. This follows
  current generator output, so I would not block this provider on it, but the
  generator could eventually reduce code size by factoring shared auth helpers for
  multi-API modules.
- The OpenAPI grouped document uses the first API's description as the document
  description, so the top-level `artificial_analysis.json` description only
  describes the data API. That is less severe than the missing attribution, but it
  makes the grouped artifact less ergonomic for users discovering the CritPt API.

## Verification Run

- `cargo check --manifest-path schematic/schema/Cargo.toml` passed.
- `cargo test -p schematic-definitions artificial_analysis` passed: 10 tests.
- `cargo test -p schematic-definitions registry::tests::registry_key_for_known_apis_matches_table` passed.
- `cargo test -p schematic-definitions registry::tests::get_registries_for_module` passed: 5 tests.
- `cargo test -p schematic-gen --test artifact_drift` passed: 4 tests.
- One attempted combined registry command failed because `cargo test` accepts only
  one test-name filter; I reran the filters separately.
