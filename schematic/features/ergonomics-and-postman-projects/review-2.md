---
ready: false
agent: ""
model: ""
---

# Implementation Review: Ergonomics and Postman Collections

## Summary

The previous OpenAPI grouping and CLI terminal-capture gaps have largely been addressed: `generate --api all` now writes module-named OpenAPI and Postman artifacts, there are committed artifact drift checks, and CLI SGR output has Level 2 tmux capture coverage.

This feature is still not ready for production. The remaining blockers are in Postman collection correctness for real users, plus one strictness gap in OpenAPI generation that can let broken `$ref`s ship.

## Findings

### 1. Grouped Postman auth is wrong for mixed-auth modules (Severity: High)

Requirement: the tech design says that if APIs in one grouped module have different auth strategies, collection-level auth should be omitted and each request should get request-level auth.

Implementation: `build_postman_collection_grouped()` always takes auth from the first API and installs it as collection auth (`schematic/gen/src/postman_output.rs:613-616`, `schematic/gen/src/postman_output.rs:680-693`). `build_request_item()` then always writes `auth: None` for every request (`schematic/gen/src/postman_output.rs:380-386`).

Impact: `emqx.postman_collection.json` groups `EmqxBasic` and `EmqxBearer`, but the collection gets Basic auth from the first API and every request inherits it. Bearer-only requests, including bearer login/logout and duplicated management endpoints, are exported with the wrong auth semantics. The emitted requests are also duplicated with indistinguishable names such as repeated `ListAlarms` entries, because the collection gives no request-level indication of which auth variant each request belongs to.

Verification level: the strongest relevant test is Level 1, but it does not cover this requirement. The Postman unit tests cover individual auth mapping only (`schematic/gen/src/postman_output.rs:889-951`), and there is no grouped mixed-auth assertion.

Recommended fix: detect whether all grouped APIs map to the same `ExportAuth`. If they do, use collection-level auth. If not, set collection auth to `None` and assign request-level auth from each endpoint's owning API. For duplicate endpoint names in mixed-auth groups, include the API/auth variant in the request or folder name.

### 2. Postman auth variables are referenced but not declared (Severity: High)

Requirement: generated Postman auth should use collection variables such as `{{bearerToken}}`, `{{apiKey}}`, `{{username}}`, and `{{password}}`, and the collection `variable` list should include auth-related variables.

Implementation: single-API collections only declare `baseUrl` (`schematic/gen/src/postman_output.rs:249-297`). Grouped collections only declare base URL variables (`schematic/gen/src/postman_output.rs:618-635`, `schematic/gen/src/postman_output.rs:680-693`). The auth objects reference variables in `build_collection_auth()` (`schematic/gen/src/postman_output.rs:305-355`), but those variables are never added to the collection variables.

Impact: for example, `schematic/postman/openai.postman_collection.json` declares only `baseUrl` while its bearer auth references `{{bearerToken}}`. Users importing the collection do not get the expected variable scaffold and have to infer missing variable names manually.

Verification level: Level 1 unit tests assert that auth objects contain `{{bearerToken}}`/`{{apiKey}}`, but no test pairs those references with collection variables.

Recommended fix: build collection variables from the auth metadata as well as base URLs, dedupe them, and add tests that every `{{...}}` variable referenced by auth/url/header/body fields is declared or intentionally external.

### 3. Multipart file uploads are exported as text fields (Severity: High)

Requirement: Postman body mapping should preserve `ApiRequest::FormData` as form data. For actual file upload endpoints, the Postman form-data entry must be a `file` field, not `text`.

Implementation: `export::body::FormField` drops the original `schematic_define::request::FormField` kind (`schematic/gen/src/export/body.rs:24-32`, `schematic/gen/src/export/body.rs:75-81`). `build_form_param()` then hard-codes every form-data and urlencoded field to `type: "text"` (`schematic/gen/src/postman_output.rs:532-539`).

Impact: real file-upload APIs are broken in Postman. The committed artifacts show this today: ElevenLabs sample upload emits `audio` as `"type": "text"`, and Unfolded Circle image/icon uploads emit `file` as `"type": "text"`.

Verification level: Level 1 tests exist for form-data shape, but they assert key/description only and do not assert file-vs-text type (`schematic/gen/src/postman_output.rs:966-991`).

Recommended fix: carry a field kind through `export::body::FormField`, map `FormField::file*`/`files*` to Postman `type: "file"`, and add tests against at least one real ElevenLabs or Unfolded Circle endpoint.

### 4. OpenAPI strict registry completeness is not enforced in the generation path (Severity: High)

Requirement: final-state generation should fail when a grouped API references a JSON response schema that is not registered, and the error should list missing type names and the owning API/module.

Implementation: `SchemaRegistry::validate_completeness()` exists (`schematic/definitions/src/registry.rs:180-202`), but `run_generate()`, `run_generate_all()`, and `write_openapi_grouped()` never call it. `schematic_define::openapi::export()` maps JSON responses directly to `#/components/schemas/<Type>` references (`schematic/define/src/openapi/export.rs:499-505`) and builds components only from whatever the registry contains (`schematic/define/src/openapi/export.rs:52-62`).

Impact: a missing registry entry can produce an OpenAPI document with dangling schema references while `schematic-gen generate` still succeeds. Per-definition unit tests reduce the risk for current APIs, but the production generation path itself does not enforce the designed contract.

Verification level: Level 1 tests validate some registries directly, but no generation-level test injects a missing referenced schema and asserts that generation/export fails with the module and type name.

Recommended fix: before exporting each grouped module, call `validate_completeness()` for every member API against the merged registry and fail with a message including module, API, and missing type names. Add a regression test that proves a dangling JSON response ref cannot be emitted.

### 5. Postman artifact validation is lighter than the design requires (Severity: Medium)

Requirement: the tech design calls for validating emitted Postman JSON against the vendored Postman Collection v2.1.0 JSON Schema and maintaining fixture-based golden artifact tests.

Implementation: there is no vendored Postman schema fixture under `schematic/gen/tests/fixtures/`, and no `jsonschema`-style validation in the tests. Current Postman artifact tests parse JSON and compare committed artifacts against the current generator output, but artifact drift tests cannot catch generator-shaped-invalid output.

Verification level: Level 1 exists for serialization and drift, but structural validation against the format contract is absent.

Recommended fix: vendor the v2.1.0 collection schema, validate every generated Postman artifact against it, and add targeted golden fixtures for mixed auth, auth variables, file uploads, path variables, query params, and grouped modules.

## Test Rigor Classification

- OpenAPI grouped filenames and CLI reporting: Level 1 plus Level 2 tmux capture are present for user-visible CLI output.
- OpenAPI artifact validity: Level 1 parse/drift tests are present, but missing-reference strictness is not verified.
- Postman artifact generation: Level 1 serialization/drift tests are present, but schema validation and several user-facing collection semantics are missing.
- Level 3: not applicable to this feature; there is no OS keyboard-event UX requirement.

## Verification Performed

- Read `spec.md` and `tech-design.md`.
- Inspected generator, registry, Postman/OpenAPI output code, docs, justfile wiring, committed artifacts, and tests.
- Ran `cargo test -p schematic-gen postman_output --lib` successfully: 18 passed.
- Queried committed Postman artifacts with `jq` to confirm missing auth variables and file fields emitted as `text`.

## Closure

Set `ready: false`. The feature should not be marked production-ready until grouped Postman auth, auth variable declaration, multipart file fields, and OpenAPI missing-schema enforcement are fixed and covered by focused tests.
