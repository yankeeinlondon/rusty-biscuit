---
phases: 5
start_phase: 1
source_files_during_phase_1:
  - schematic/gen/src/postman_output.rs
  - schematic/gen/src/export/auth.rs
  - schematic/gen/tests/fixtures/postman/golden/file_upload_elevenlabs.json
  - schematic/define/src/openapi/export.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2: []
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - schematic/definitions/src/registry.rs
  - schematic/definitions/src/bitbucket/mod.rs
  - schematic/definitions/src/gitea/mod.rs
  - schematic/definitions/src/github/mod.rs
  - schematic/definitions/src/gitlab/mod.rs
  - schematic/definitions/src/ollama/mod.rs
  - schematic/definitions/src/anthropic/mod.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - schematic/definitions/src/registry.rs
  - schematic/define/src/openapi/export.rs
  - schematic/define/src/openapi/error.rs
  - schematic/gen/tests/openapi_strict_completeness.rs
  - schematic/definitions/src/anthropic/mod.rs
  - schematic/definitions/src/bitbucket/mod.rs
  - schematic/definitions/src/gitea/mod.rs
  - schematic/definitions/src/github/mod.rs
  - schematic/definitions/src/gitlab/mod.rs
  - schematic/definitions/src/ollama/mod.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages:
  - schematic-gen
  - schematic-define
  - schematic-definitions
---
# Review-3 Implementation Plan: Ergonomics and Postman Collections

## Overview

This plan addresses all four findings from `review-3.md`:

1. **OpenAPI artifacts contain unresolved component references** (High)
2. **Postman API-key auth serializes header name and secret variable backwards** (High)
3. **Grouped Postman base URL aliases are declared but not used** (High)
4. **API-key parameter auth loses location metadata** (Medium)

Each phase is self-contained, has clear test requirements, and ends with passing `cargo test` and `cargo clippy` for the affected schematic packages.

---

## Phase 1: Postman API-Key Auth Key/Value Fix

### Goal
Swap the reversed `key`/`value` fields in Postman `apikey` auth blocks so that `key` holds the header/parameter name and `value` holds the secret variable reference.

### Files to Change

1. **`schematic/gen/src/postman_output.rs`**
   - In `build_collection_auth()`, `ExportAuth::ApiKey` arm (lines 322-343):
     - Change `key: "key"` → `value: Some(header.clone())`
     - Change `key: "value"` → `value: Some(format!("{{{{{}}}}}"`, variable))`
     - Keep `key: "in"` → `value: Some("header".to_string())` (will be updated in Phase 4 for query/cookie)

2. **`schematic/gen/src/postman_output.rs` (tests)**
   - Update `auth_api_key` test (lines 1069-1087) to assert the corrected mapping:
     - `apikey[0].key == "key"` && `apikey[0].value == Some("X-API-Key".to_string())`
     - `apikey[1].key == "value"` && `apikey[1].value == Some("{{apiKey}}".to_string())`
     - `apikey[2].key == "in"` && `apikey[2].value == Some("header".to_string())`

3. **`schematic/gen/tests/fixtures/postman/golden/file_upload_elevenlabs.json`**
   - Regenerate via `BLESS_POSTMAN_GOLDEN=1 cargo test -p schematic-gen --test postman_golden` after code fix.

### Test Verification
- `cargo test -p schematic-gen --test postman_golden` passes (fixture re-blessed)
- `cargo test -p schematic-gen postman_output::tests::auth_api_key` passes with corrected assertions
- `cargo clippy -p schematic-gen` clean

### Estimated Complexity
Small — two field swaps, one test update, one fixture regeneration.

---

## Phase 2: Grouped Postman Base URL Variable Routing

### Goal
Ensure requests from grouped APIs with distinct base URLs reference the correct `baseUrl` variable (`baseUrl`, `baseUrl2`, etc.) instead of always using `{{baseUrl}}`.

### Files to Change

1. **`schematic/gen/src/postman_output.rs`**
   - Modify `build_request_item()` signature to accept `base_url_var: &str`:
     ```rust
     fn build_request_item(
         endpoint: &Endpoint,
         api: &RestApi,
         owning_auth: &ExportAuth,
         emit_request_level_auth: bool,
         disambiguate_with_api_name: bool,
         base_url_var: &str,  // NEW
     ) -> PostmanItem
     ```
   - Pass `base_url_var` into `build_url()`:
     ```rust
     let url = build_url(&api.base_url, &endpoint.path, &path_params, endpoint, base_url_var);
     ```

   - Modify `build_url()` signature to accept `base_url_var: &str`:
     ```rust
     fn build_url(
         _base_url: &str,
         path: &str,
         path_params: &[&str],
         endpoint: &Endpoint,
         base_url_var: &str,  // NEW
     ) -> PostmanUrl
     ```
   - Replace hard-coded `"{{baseUrl}}"` with `format!("{{{{{}}}}}"`, base_url_var)` in `raw` URL and `host`.

   - In `build_postman_collection_grouped()` (lines 728-860):
     - Build a `BTreeMap<String, String>` or `HashMap<&str, &str>` that maps each API's `base_url` to its assigned variable name (`baseUrl`, `baseUrl2`, ...).
     - Pass the correct variable name when calling `build_request_item()` for each endpoint.

   - In `build_postman_collection()` (single-API path, line 246-307):
     - Pass `"baseUrl"` as the `base_url_var` when calling `build_request_item()`.

   - Update all call sites of `build_request_item()` and `build_url()` in tests.

2. **`schematic/gen/src/postman_output.rs` (tests)**
   - Update `url_with_path_params` test to pass `"baseUrl"`.
   - Update any other unit tests calling `build_url()` directly.

3. **`schematic/gen/tests/fixtures/postman/golden/grouped_module_ollama.json`**
   - The `OllamaOpenAI` request (`ListModels`) must now have:
     - `"raw": "{{baseUrl2}}/models"`
     - `"host": ["{{baseUrl2}}"]`
   - Regenerate via `BLESS_POSTMAN_GOLDEN=1 cargo test -p schematic-gen --test postman_golden`.

4. **`schematic/gen/tests/postman_golden.rs`**
   - Add behavioural assertion to `golden_grouped_module_ollama()`:
     ```rust
     // Verify OllamaOpenAI's ListModels uses baseUrl2, not baseUrl
     let list_models_req = find_request_by_name(&value, "ListModels")
         .expect("ListModels request must exist");
     let raw_url = list_models_req.pointer("/url/raw").and_then(Value::as_str);
     assert_eq!(raw_url, Some("{{baseUrl2}}/models"), "OllamaOpenAI request must use baseUrl2");
     ```

### Test Verification
- `cargo test -p schematic-gen --test postman_golden` passes (fixture re-blessed + new assertion)
- `cargo test -p schematic-gen postman_output::tests` passes
- `cargo clippy -p schematic-gen` clean

### Estimated Complexity
Medium — signature changes propagate through multiple functions, tests, and fixtures. The core logic is straightforward (map base_url → var name, pass it through).

---

## Phase 3: API-Key Parameter Auth Location Metadata

### Goal
Preserve `ApiKeyParam.location` (Query/Cookie) through the export pipeline instead of hard-coding `"header"`.

### Files to Change

1. **`schematic/gen/src/export/auth.rs`**
   - Extend `ExportAuth::ApiKey` with an `in`/`location` field:
     ```rust
     pub enum ExportAuth {
         // ...
         ApiKey {
             header: String,
             variable: String,
             location: ApiKeyLocation,  // NEW
         },
         // ...
     }
     ```
   - Update `map_auth()`:
     - `AuthStrategy::ApiKey { header }` → `location: ApiKeyLocation::Header`
     - `AuthStrategy::BearerToken { header: Some(h) }` → `location: ApiKeyLocation::Header`
     - `AuthStrategy::ApiKeyParam { name, location }` → map `location` directly
   - Update all `ExportAuth::ApiKey { ... }` instantiations in tests.

2. **`schematic/gen/src/postman_output.rs`**
   - In `build_collection_auth()`, `ExportAuth::ApiKey` arm:
     - Map `ApiKeyLocation::Header` → `"header"`
     - Map `ApiKeyLocation::Query` → `"query"`
     - Map `ApiKeyLocation::Cookie` → `"cookie"`

3. **`schematic/gen/src/postman_output.rs` (tests)**
   - Update `auth_api_key` test to include `location: ApiKeyLocation::Header`.
   - Add new tests:
     - `auth_api_key_query_location()` — verifies `"in": "query"` for `ApiKeyParam` with `Query`
     - `auth_api_key_cookie_location()` — verifies `"in": "cookie"` for `ApiKeyParam` with `Cookie`
     - `map_auth_api_key_param_query()` in `export/auth.rs` tests

4. **`schematic/gen/tests/postman_golden.rs`**
   - Add a new golden scenario `build_api_key_query_auth()` that creates an API with `AuthStrategy::ApiKeyParam { name: "api_key", location: ApiKeyLocation::Query }` and one endpoint.
   - Add `golden_api_key_query_auth()` test that asserts:
     - `auth.apikey` contains `"in": "query"`
     - `auth.apikey` contains `"key": "api_key"`
   - Commit the generated fixture to `tests/fixtures/postman/golden/api_key_query_auth.json`.

### Test Verification
- `cargo test -p schematic-gen --test postman_golden` passes (new fixture + existing)
- `cargo test -p schematic-gen export::auth::tests` passes
- `cargo test -p schematic-gen postman_output::tests` passes
- `cargo clippy -p schematic-gen` clean

### Estimated Complexity
Small-to-medium — adding a field to an enum variant touches several pattern matches, but the logic is simple.

---

## Phase 4: OpenAPI $ref Closure Validation

### Goal
Prevent OpenAPI artifacts from containing dangling `$ref` references by:
1. Adding a validation pass that walks every emitted `$ref` and fails if the target is missing from `components.schemas`.
2. Extending `validate_completeness()` to also check JSON request body schemas and nested `$defs` from registered response types.

### Files to Change

#### 4a. Extend `validate_completeness()` to cover request schemas

1. **`schematic/definitions/src/registry.rs`**
   - In `validate_completeness()` (lines 180-203), add request schema checking:
     ```rust
     for endpoint in &api.endpoints {
         // Existing: check response schemas
         if let ApiResponse::Json(schema) = &endpoint.response {
             // ... existing logic ...
         }
         // NEW: check request schemas
         if let Some(ApiRequest::Json(schema)) = &endpoint.request {
             let type_name = &schema.type_name;
             if !self.types.contains_key(type_name) {
                 missing.push(type_name.clone());
             }
         }
     }
     ```
   - Add test: `validate_completeness_fails_for_missing_request_types()`.

#### 4b. Add `$ref` closure validation to OpenAPI export

2. **`schematic/define/src/openapi/export.rs`**
   - Add a new function `validate_ref_closure(openapi: &OpenAPI) -> Result<(), Vec<String>>` that:
     - Collects all schema names present in `components.schemas`.
     - Recursively walks every `ReferenceOr::Reference { reference }` under `paths` and `components`.
     - Extracts the schema name from `#/components/schemas/<Name>` references.
     - Returns `Err(vec![...])` with any missing schema names.
   - Call `validate_ref_closure()` at the end of `export()` before returning `Ok(OpenAPI)`.
   - Update `export()` return type or wrap the error appropriately. Since `export()` already returns `Result<OpenAPI, OpenApiError>`, add a new `OpenApiError::UnresolvedRefs(Vec<String>)` variant (or similar).

3. **`schematic/define/src/openapi/error.rs`** (or wherever `OpenApiError` is defined)
   - Add error variant for unresolved refs if not already present.

4. **`schematic/define/src/openapi/export.rs` (tests)**
   - Add test `export_fails_on_unresolved_request_schema_ref()`:
     - Create an API with `ApiRequest::Json(Schema::new("MissingBody"))`.
     - Use an empty `TestRegistry`.
     - Assert `export()` returns an error containing `"MissingBody"`.
   - Add test `export_fails_on_unresolved_response_schema_ref()`:
     - Similar, but with `ApiResponse::Json(Schema::new("MissingResponse"))`.
   - Add test `export_succeeds_when_all_refs_are_resolved()`:
     - Register both request and response schemas in `TestRegistry`.
     - Assert `export()` succeeds.

#### 4c. Nested `$defs` / nested schema inclusion (optional but recommended)

5. **`schematic/definitions/src/registry.rs`**
   - The `convert_schema_to_openapi()` function already rewrites `#/$defs/` → `#/components/schemas/`. However, those `$defs` are not extracted and registered separately.
   - **Recommended approach**: In the schemars `Schema` → OpenAPI conversion, when a `$ref` to `#/components/schemas/<Name>` is emitted but `<Name>` is NOT in the top-level registry, we have two options:
     - **Option A**: Include all `$defs` from every registered schema into `components.schemas` during `to_openapi_schemas()`. This requires walking the schemars JSON tree for each registered schema, finding `$defs`, and converting them too.
     - **Option B**: Change `map_request_body()` to emit inline schemas instead of `$ref` when the schema is not in the registry.
   - **Decision**: Use **Option A** for correctness. In `to_openapi_schemas()`:
     - After converting the top-level schema, also walk the original schemars JSON for `$defs` entries.
     - Convert each `$def` to an `openapiv3::Schema` and include it in the returned `IndexMap` under the `$def` key name.
   - This is the cleanest fix because it makes the registry truly complete.

   However, since this is a significant change, a simpler **Option C** is acceptable for this plan:
   - **Option C**: Keep the `$ref` closure validation (4b) as the primary guardrail. For request body schemas, ensure every API registers its request body types in the registry. For nested `$defs`, update each API's `openapi_registry()` to also `.register::<T>()` all nested types that appear in `$defs`.
   - Since the review says "fix registry generation/conversion so request-body types and nested `$defs` are included in `components.schemas`", we should:
     1. Extend each API's `openapi_registry()` to include request body types.
     2. For nested `$defs`, add a helper `SchemaRegistry::extract_and_register_defs()` or modify `to_openapi_schemas()` to include them.

   **Final approach for this plan**:
   - Modify `SchemaRegistry::to_openapi_schemas()` to also extract `$defs` from each registered schemars schema and convert them:
     ```rust
     pub fn to_openapi_schemas(&self) -> IndexMap<String, openapiv3::Schema> {
         let mut result = IndexMap::new();
         for (name, schema) in &self.types {
             let openapi_schema = convert_schema_to_openapi(schema);
             result.insert(name.clone(), openapi_schema);
             // Extract $defs and add them too
             if let Some(defs) = schema.as_value().get("$defs").and_then(|v| v.as_object()) {
                 for (def_name, def_schema) in defs {
                     if !result.contains_key(def_name) {
                         result.insert(def_name.clone(), convert_json_schema_to_openapi(def_schema));
                     }
                 }
             }
         }
         result
     }
     ```
   - Update `validate_completeness()` to also check request body schemas (as in 4a).
   - Per-API registries may need to be updated to register request body types. The plan should include running `cargo test -p schematic-definitions` to identify which APIs now fail `validate_completeness` after adding request-body checks, then updating those registries.

#### 4d. Registry updates for request body schemas

6. **Per-API registry files** (`schematic/definitions/src/*/mod.rs` or wherever `openapi_registry()` is defined)
   - Run tests to identify which APIs are missing request body schema registrations.
   - Add `.register::<T>("BodyTypeName")` entries for every `ApiRequest::Json` schema referenced by endpoints.
   - Examples from the review:
     - `anthropic`: `CreateMessageBody`, `CountTokensBody`, `ContentBlock`
     - `elevenlabs`, `emqx`, `ollama`, `unfolded_circle_core_rest`: similar missing types

#### 4e. Update existing OpenAPI strict completeness tests

7. **`schematic/gen/tests/openapi_strict_completeness.rs`**
   - The existing tests should continue to pass after registry updates.
   - Add a new test `request_body_types_must_be_registered()`:
     - Create a synthetic API with `ApiRequest::Json(Schema::new("SomeBody"))`.
     - Use an empty registry.
     - Assert `validate_completeness()` returns `Err` containing `"SomeBody"`.

### Test Verification
- `cargo test -p schematic-define` passes (new `$ref` closure tests)
- `cargo test -p schematic-definitions` passes (registry completeness tests + any updated registries)
- `cargo test -p schematic-gen --test openapi_strict_completeness` passes
- `cargo test -p schematic-gen --test artifact_drift` passes (regenerate OpenAPI artifacts if needed)
- `cargo clippy -p schematic-define -p schematic-definitions -p schematic-gen` clean

### Estimated Complexity
Large — this is the most involved fix. It requires:
- Adding a new validation pass to the OpenAPI exporter
- Extending registry completeness checks
- Potentially updating many per-API registry definitions
- Extracting `$defs` from schemars schemas
- Regenerating committed OpenAPI artifacts

---

## Phase 5: Integration, Regeneration, and Final Verification

### Goal
Ensure all changes work together, all tests pass, no lint warnings, and committed artifacts are regenerated.

### Steps

1. **Regenerate committed artifacts**:
   ```bash
   just -f schematic/justfile generate
   ```
   This will update `schematic/openapi/*.json` and `schematic/postman/*.postman_collection.json`.

2. **Verify OpenAPI $ref closure manually**:
   ```bash
   # Quick sanity check: no dangling $refs in committed OpenAPI files
   for f in schematic/openapi/*.json; do
     echo "Checking $f..."
     # Extract all $ref values under paths and components
     refs=$(jq -r '.. | ."$ref"? | select(. != null)' "$f" | grep '#/components/schemas/' | sed 's|#/components/schemas/||' | sort -u)
     schemas=$(jq -r '.components.schemas | keys[]' "$f" | sort -u)
     for ref in $refs; do
       if ! echo "$schemas" | grep -qx "$ref"; then
         echo "  MISSING: $ref"
       fi
     done
   done
   ```

3. **Run full test suite**:
   ```bash
   cargo test -p schematic-define -p schematic-definitions -p schematic-gen
   ```

4. **Run lint checks**:
   ```bash
   cargo clippy -p schematic-gen -p schematic-define -p schematic-definitions -- -D warnings
   ```

5. **Verify Postman golden fixtures**:
   ```bash
   cargo test -p schematic-gen --test postman_golden
   cargo test -p schematic-gen --test postman_artifact_validation
   cargo test -p schematic-gen --test postman_var_consistency
   ```

6. **Update `review-3.md`**:
   - Change `ready: false` → `ready: true` after all tests pass.

### Test Verification
- All unit tests pass
- All integration tests pass
- All golden fixtures are regenerated and committed
- No clippy warnings
- Manual `$ref` check shows zero dangling references in committed OpenAPI artifacts

### Estimated Complexity
Medium — mostly verification and regeneration. Could reveal issues from earlier phases that need minor fixes.

---

## Summary Table

| Phase | Focus | Severity | Files Touched | Complexity |
|-------|-------|----------|---------------|------------|
| 1 | Postman API-key auth key/value swap | High | `postman_output.rs`, golden fixture | Small |
| 2 | Grouped Postman base URL variable routing | High | `postman_output.rs`, golden fixture, tests | Medium |
| 3 | API-key parameter location metadata | Medium | `export/auth.rs`, `postman_output.rs`, new golden fixture | Small-Medium |
| 4 | OpenAPI `$ref` closure validation + registry completeness | High | `registry.rs`, `export.rs`, per-API registries, tests | Large |
| 5 | Integration, artifact regeneration, final verification | — | All generated artifacts, review doc | Medium |

---

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| Phase 4 registry updates are large (many APIs missing request body types) | Start by running tests to get an exact list of missing types; update one API at a time |
| `$defs` extraction from schemars schemas is complex | Implement incrementally: first extract top-level `$defs`, then nested ones if needed |
| Golden fixture regeneration masks unintended changes | Review diffs carefully before committing; use `git diff` on fixture files |
| Existing tests rely on old (buggy) auth shape | Update unit tests in the same commit as the code fix to keep atomicity |
| `build_url()` signature change breaks other callers | Search codebase for all `build_url` call sites; only used in `postman_output.rs` and its tests |

---

## Completion Criteria

- [ ] Phase 1: Postman API-key auth emits `key` = header name, `value` = `{{apiKey}}`
- [ ] Phase 2: Grouped Postman requests use `baseUrl2`, `baseUrl3`, etc. when appropriate
- [ ] Phase 3: `ApiKeyParam` with `Query`/`Cookie` produces `"in": "query"`/`"cookie"` in Postman
- [ ] Phase 4: OpenAPI export validates `$ref` closure and fails on dangling references
- [ ] Phase 4: `validate_completeness()` checks both request and response JSON schemas
- [ ] Phase 4: All committed OpenAPI artifacts have zero dangling `$ref`s
- [ ] Phase 5: `cargo test -p schematic-define -p schematic-definitions -p schematic-gen` passes
- [ ] Phase 5: `cargo clippy -p schematic-gen -p schematic-define -p schematic-definitions -- -D warnings` passes
- [ ] Phase 5: All regenerated artifacts are committed
- [ ] Phase 5: `review-3.md` updated to `ready: true`
