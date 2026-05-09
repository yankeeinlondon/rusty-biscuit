---
ready: false
agent: codex
model: ""
---

# Review 2

## Findings

### High: Generated-client tests bypass the typed response contract

The new integration tests cover important request-side behavior, but they call
`client.request(...)` as `Result<serde_json::Value, _>` instead of using the
endpoint response structs declared by the spec. That leaves the generated typed
response contract effectively unverified for `LlmModelsResponse`,
`MediaModelsResponse`, and `CritPtEvaluateResponse`.

This is visible in `schematic/schema/tests/artificial_analysis_client.rs`: the
LLM test returns only `{"status": 200, "data": []}` and still passes because it
deserializes to `serde_json::Value`, even though `LlmModelsResponse` requires
`prompt_options` (`schematic/definitions/src/artificial_analysis/types.rs`). The
CritPt tests return `{"accuracy": 0.0, "results": []}` and also pass as
`serde_json::Value`, even though `CritPtEvaluateResponse` requires
`accuracy`, `timeout_rate`, `server_timeout_count`, and `judge_error_count`.

Requirement mapping:

- Data endpoints return typed `LlmModelsResponse` / `MediaModelsResponse`:
  strongest current provider-specific verification is Level 1 request-path/query
  integration only; typed response deserialization is not verified.
- CritPt endpoint returns typed `CritPtEvaluateResponse`: strongest current
  provider-specific verification is Level 1 request-body integration only; typed
  response deserialization is not verified.
- Terminal Level 2 / Level 3 categories are not applicable because this feature
  has no terminal rendering, hotkeys, paste, mouse, IME, or OS keyboard behavior.

Suggested fix: update the mock success payloads so they match the declared
response structs, and deserialize at least one representative request per response
shape as its concrete type:

- `client.request::<LlmModelsResponse>(...)` with `prompt_options`.
- `client.request::<MediaModelsResponse>(...)` for a media endpoint, including
  `include_categories`.
- `client.request::<CritPtEvaluateResponse>(...)` with all required fields.

Keep the existing `serde_json::Value` checks only where the test is intentionally
about raw request serialization or hook behavior.

## Coverage Notes

The prior review blockers are addressed: attribution now appears in generated
schema/OpenAPI/Postman artifacts with an artifact regression test, and there are
provider-specific generated-client tests for auth, query serialization, missing
credentials, explicit API key override, env fallback, and CritPt request body
serialization.

Definition-level coverage is also solid for the designed shape: two shared-module
`RestApi` definitions, six data endpoints, one CritPt endpoint, `x-api-key` auth
with `ARTIFICIAL_ANALYSIS_API_KEY`, query-param placement, registry completeness,
and intentional omission of `RateLimitError`.

I did not find an implementation gap in the API definitions themselves. The
remaining issue is test rigor for the generated typed response behavior.

## Verification Run

- `cargo test -p schematic-definitions artificial_analysis` passed: 10 tests.
- `cargo test --manifest-path schematic/schema/Cargo.toml --test artificial_analysis_client` passed: 7 tests.
- `cargo test -p schematic-gen --test artifact_drift artificial_analysis_attribution_present_in_all_artifacts` passed: 1 test.
- `cargo test -p schematic-definitions registry::tests::registry_key_for_known_apis_matches_table` passed: 1 test.
