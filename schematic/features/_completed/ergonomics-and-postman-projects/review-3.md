---
ready: true
agent: codex
model: ""
---

# Implementation Review: Ergonomics and Postman Collections

## Summary

The review-2 blockers have mostly been addressed: grouped Postman auth, declared auth variables, file form fields, strict response-registry checks, and Postman schema validation now have implementation and tests.

The feature is still not production-ready. The remaining blockers are semantic correctness issues that the current tests do not catch: committed OpenAPI artifacts contain dangling `$ref`s, Postman API-key auth is serialized with the key/value fields reversed, and grouped Postman base URL aliases are declared but not used by requests.

## Findings

### 1. OpenAPI artifacts contain unresolved component references (Severity: High)

Requirement: the spec asks for OpenAPI schemas to be generated as first-class artifacts and tested thoroughly to ensure they are correct. The technical design also makes artifact trustworthiness the reason for strict final-state generation.

Implementation: `map_request_body()` emits JSON request bodies as `$ref: "#/components/schemas/<Type>"` (`schematic/define/src/openapi/export.rs:324-330`), and nested schemars `$ref`s are also rewritten to `#/components/schemas/...`. However, `SchemaRegistry::validate_completeness()` only checks `ApiResponse::Json` response schemas (`schematic/definitions/src/registry.rs:180-192`). It does not validate JSON request schemas or nested referenced schemas from registered response types.

Impact: many committed OpenAPI files currently contain dangling references while all added strictness tests still pass. For example, `schematic/openapi/anthropic.json` references `#/components/schemas/CreateMessageBody`, `#/components/schemas/CountTokensBody`, and `#/components/schemas/ContentBlock`, but those names are absent from `components.schemas`. I verified the same class of missing refs across multiple committed artifacts, including `elevenlabs.json`, `emqx.json`, `ollama.json`, and `unfolded_circle_core_rest.json`.

Verification level: Level 1 parse tests and registry-completeness tests are present, but they validate only JSON syntax/OpenAPI struct parsing and response schema names. They do not validate `$ref` closure, so they are the wrong Level 1 assertions for this requirement. Level 2/3 are not applicable to static OpenAPI artifacts.

Recommended fix: add an OpenAPI artifact validation pass that walks every emitted `$ref` under `#/components/schemas/*` and fails if the target component is missing. Then fix registry generation/conversion so request-body types and nested `$defs` are included in `components.schemas`, or stop emitting refs for schemas that are not registered.

### 2. Postman API-key auth serializes header name and secret variable backwards (Severity: High)

Requirement: Postman auth for `ApiKey { header }` and `BearerToken { header: Some(h) }` should let imported collections send the configured header name with a variable secret value.

Implementation: `build_collection_auth()` currently emits Postman `apikey` entries as `key = "{{apiKey}}"` and `value = "<header-name>"` (`schematic/gen/src/postman_output.rs:322-338`). The committed ElevenLabs collection shows the bug directly: `key` is `{{apiKey}}` and `value` is `xi-api-key` (`schematic/postman/elevenlabs.postman_collection.json:19-32`). Postman expects the API-key auth property named `key` to hold the header/query parameter name and the property named `value` to hold the secret value.

Impact: imported API-key collections will send the wrong credential shape. This affects current generated collections such as Anthropic, ElevenLabs, GitLab, and Gitea.

Verification level: Level 1 tests exist, but they assert the wrong behavior; the `file_upload_elevenlabs` golden fixture also enshrines the reversed shape. Postman JSON Schema validation cannot catch this because the document is structurally valid but semantically wrong.

Recommended fix: swap the API-key auth mapping to emit `{"key":"key","value": header}`, `{"key":"value","value":"{{apiKey}}"}`, and `{"key":"in","value":"header"}`. Add a focused test for a real API-key provider and, if `ApiKeyParam` remains supported by the exporter, preserve/query-test its `location` as well.

### 3. Grouped Postman base URL aliases are declared but not used (Severity: High)

Requirement: grouped module collections should produce usable requests for every member API, including modules whose members have distinct base URLs.

Implementation: `build_postman_collection_grouped()` declares `baseUrl`, `baseUrl2`, etc. for distinct API base URLs (`schematic/gen/src/postman_output.rs:747-764`), but `build_request_item()` always calls `build_url()` with no base-variable choice, and `build_url()` always emits `{{baseUrl}}` (`schematic/gen/src/postman_output.rs:452-460`). The golden test creates a synthetic grouped Ollama case where `OllamaOpenAI` has `http://localhost:11434/v1`, but the expected fixture still emits `{{baseUrl}}/models` while declaring `baseUrl2 = http://localhost:11434/v1` (`schematic/gen/tests/fixtures/postman/golden/grouped_module_ollama.json:36-65`).

Impact: any grouped collection with distinct base URLs will route later APIs through the first API's base URL. The current test checks that `baseUrl2` exists, but not that requests from the second API use it.

Verification level: Level 1 golden coverage exists but asserts only variable declaration, not request behavior. Level 2/3 terminal verification is not applicable.

Recommended fix: carry the selected base URL variable name alongside each grouped API, pass it into URL construction, and assert that each request references the variable matching its owning API. The existing synthetic grouped Ollama fixture is already a good regression target once its expected URL is corrected to `{{baseUrl2}}/models`.

### 4. API-key parameter auth loses location metadata (Severity: Medium)

Requirement: `AuthStrategy::ApiKeyParam` supports query and cookie locations in `schematic-define`, and OpenAPI import can produce that strategy.

Implementation: `map_auth()` collapses `ApiKeyParam { name, location }` into `ExportAuth::ApiKey { header: name, ... }`, dropping `location`, and Postman auth hard-codes `"in": "header"` (`schematic/gen/src/export/auth.rs:69-72`, `schematic/gen/src/postman_output.rs:336-338`).

Impact: no current committed REST definition appears to use `ApiKeyParam`, so this is not breaking today's artifacts. It is still incomplete exporter behavior for imported or future definitions and will silently turn query/cookie API-key auth into header auth.

Verification level: no Level 1 Postman tests cover `ApiKeyParam` location. Level 2/3 are not applicable.

Recommended fix: extend `ExportAuth::ApiKey` with an `in`/location field, map `Query` and `Cookie` distinctly, and add unit/golden coverage for query API-key auth at minimum.

## Test Rigor Classification

- OpenAPI artifact generation: Level 1 parse/drift/completeness tests are present, but missing `$ref` closure validation means the strongest test is mismatched for correctness.
- Postman artifact generation: Level 1 schema/golden/drift tests are present. They catch structural JSON validity but miss semantic API-key auth and grouped base URL routing.
- CLI terminal output: Level 2 tmux capture exists for grouped artifact filenames. That is sufficient for the terminal-rendering requirement in this feature.
- Level 3: not applicable; this feature has no OS keyboard-event requirement.

## Verification Performed

- Read `spec.md` and `tech-design.md`.
- Reviewed the generator, OpenAPI writer, Postman writer, registry strictness, justfile/docs wiring, committed artifacts, and added tests.
- Ran `cargo test -p schematic-gen --test postman_golden --test postman_artifact_validation --test postman_var_consistency --test openapi_strict_completeness` successfully.
- Used `jq` to compare every committed OpenAPI `$ref` against `components.schemas`, confirming unresolved references in multiple generated artifacts.

## Closure

Set `ready: false`. The feature should not be marked production-ready until OpenAPI `$ref` closure is enforced and fixed, Postman API-key auth sends the correct header/secret mapping, and grouped Postman requests use the base URL variable for their owning API.
