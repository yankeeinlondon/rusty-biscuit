# Implementation Plan — Review-2 Closure

Closes the five findings in
`schematic/features/ergonomics-and-postman-projects/review-2.md`. The plan is
broken into six cohesive phases. Each phase ends with green tests, zero
clippy warnings on the touched crates, and (when generator behaviour changes)
regenerated artifacts under `schematic/openapi/` and `schematic/postman/`
that match the new generator output.

## Conventions Used Throughout

- Workspace-aware test command:
  ```bash
  cargo test -p schematic-define -p schematic-definitions -p schematic-gen
  ```
- Workspace-aware clippy command:
  ```bash
  cargo clippy --all-targets -p schematic-define -p schematic-definitions -p schematic-gen -- -D warnings
  ```
- Whenever generator output changes, regenerate and verify the schema crate
  builds:
  ```bash
  just -f schematic/justfile generate
  cargo check -p schematic-schema --manifest-path schematic/schema/Cargo.toml
  ```
  (`schematic/schema` is excluded from the workspace, hence the explicit
  `--manifest-path`.)
- Test-rigor language used below:
  - **Level 1** = unit-level Rust assertions (`#[test]` in the crate that
    owns the code).
  - **Level 2** = integration tests under `schematic/gen/tests/*` exercising
    the public API or fixture artifacts.

## Phase Overview

| Phase | Focus | Touches generator output? |
|-------|-------|---------------------------|
| 0 | Vendor Postman v2.1.0 schema + helper for fixture validation | no (test infra only) |
| 1 | Carry `FormFieldKind` through `ExportBody`; emit `type: "file"` for file uploads (Finding 3) | yes |
| 2 | Declare auth variables in collection `variable` list (Finding 2) | yes |
| 3 | Mixed-auth handling for grouped Postman collections (Finding 1) | yes |
| 4 | Enforce `validate_completeness()` in the generation path (Finding 4) | no (error-path only; no shape change) |
| 5 | Postman JSON Schema validation + golden fixtures (Finding 5) | no |
| 6 | Repository-wide verification: tests, clippy, regenerate, drift | yes — final regenerate + commit |

The fix-order is deliberate: shape-changing fixes (1-3) come first so that
re-runs of `just generate` produce stable artifacts. Strictness (4) lands
next so it cannot mask a regression introduced by 1-3. Validation
infrastructure (0, 5) is plumbed in early (0) and exercised at the end (5)
against the now-correct artifacts.

---

## Phase 0 — Test Infrastructure: Vendor Postman v2.1.0 Schema

### Goal

Land the JSON Schema fixture and a tiny validation helper used by
phases 1-5 so that fixture-shaped regressions cannot hide. No generator
behaviour changes in this phase.

### Files

- **New**: `schematic/gen/tests/fixtures/postman/v2.1.0-collection.schema.json`
  — vendored copy of
  `https://schema.getpostman.com/json/collection/v2.1.0/collection.json`.
  The file contains the Postman-published JSON Schema as-is (no edits).
- **New**: `schematic/gen/tests/postman_schema.rs` — thin Level 2 test
  harness that:
  - Loads the vendored schema once via a `OnceLock<jsonschema::Validator>`.
  - Exposes `pub(crate) fn validate_postman_json(value: &serde_json::Value)`
    that returns `Result<(), Vec<String>>`.
  - Includes two sanity tests:
    1. The vendored schema itself parses as a valid JSON Schema.
    2. A handcrafted minimal `{ "info": { "name": "x", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json" }, "item": [] }` validates `Ok`.
- **Modify**: `schematic/gen/Cargo.toml` — add `jsonschema = "0.30"` (or the
  workspace pinned version if already present elsewhere; pick a version that
  builds against the existing `serde_json` 1.0). Place under
  `[dev-dependencies]` because validation only runs in tests.

### Tests

- Level 2: the two sanity tests above. Document that this harness is the
  validator used by phases 1, 2, 3, and 5.
- The vendored schema file is treated as a frozen artifact — drop a
  one-line `README.md` next to it noting the upstream URL and date pulled.

### Verification

```bash
cargo test -p schematic-gen --test postman_schema
cargo clippy --all-targets -p schematic-gen -- -D warnings
```

### Done When

- New `postman_schema.rs` integration test file passes both sanity tests.
- `cargo build -p schematic-gen --tests` succeeds with the new dep.
- No production-code or generator behaviour changes shipped in this phase.

---

## Phase 1 — File-vs-Text in Multipart Bodies (Finding 3)

### Goal

Carry the original `schematic_define::request::FormFieldKind` through the
`ExportBody`/`FormField` pipeline and emit Postman `type: "file"` for file
upload fields. No more silent flattening of file uploads to `text`.

### Files

- **Modify**: `schematic/gen/src/export/body.rs`
  - Extend `ExportBody::FormField` (line 33-41) with a `kind` field:
    ```rust
    pub struct FormField {
        pub name: String,
        pub required: bool,
        pub description: Option<String>,
        pub kind: FormFieldExportKind, // new
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum FormFieldExportKind {
        Text,
        File { accept: Vec<String> },
        Files { accept: Vec<String>, min: Option<u32>, max: Option<u32> },
        Json,
    }
    ```
  - Update `map_form_field` (line 75-81) to translate
    `schematic_define::FormFieldKind` → `FormFieldExportKind`. `Json` is
    serialized as a text field at the Postman layer (Postman has no native
    JSON-part concept) but we preserve the discriminator so callers can
    pivot later.
- **Modify**: `schematic/gen/src/postman_output.rs`
  - In `build_form_param` (line 533-540), branch on
    `field.kind`:
    - `FormFieldExportKind::File { .. } | Files { .. }` → `type_field: "file"`,
      and set `value: None` (Postman expects `src` array for files; `value`
      is irrelevant).
    - Other variants → `type_field: "text"`, value left as today.
  - Update tests `body_form_data` (line 967-991) and `body_url_encoded`
    (line 994-1009) to set the new `kind` field and assert the right
    Postman type emerges.
- **Modify**: every test/call-site in this crate that builds an
  `export::body::FormField` literal — search for `FormField {` inside
  `schematic/gen/src` and update to include `kind`.

### Tests

- **Level 1, `schematic/gen/src/export/body.rs`**:
  - `map_body_form_data_preserves_file_kind` — feed
    `ApiRequest::FormData` containing `FormField::file("audio")` and assert
    the resulting `ExportBody::FormData` field has
    `kind: FormFieldExportKind::File { accept: vec![] }`.
  - `map_body_form_data_preserves_files_constraints` — same idea for
    `FormField::files_with_constraints`.
  - `map_body_form_data_preserves_text_for_text` — round-trip Text.
- **Level 1, `schematic/gen/src/postman_output.rs`**:
  - `form_param_file_emits_type_file` — assert `build_form_param` emits
    `"file"` for `FormFieldExportKind::File`.
  - `form_param_text_emits_type_text` — same for Text.
  - `body_form_data_real_elevenlabs_upload` — build a representative
    ElevenLabs sample-upload `ApiRequest` (mirror what's in
    `schematic_definitions::elevenlabs`), generate the Postman body, and
    assert the `audio` field has `type == "file"`.

### Verification

```bash
cargo test -p schematic-define -p schematic-definitions -p schematic-gen
just -f schematic/justfile generate
cargo check -p schematic-schema --manifest-path schematic/schema/Cargo.toml
```

After regeneration:

```bash
jq '.item[] | select(.name | test("upload"; "i")) | .. | objects | select(.formdata?) | .formdata' \
   schematic/postman/elevenlabs.postman_collection.json
```

Manual sanity: confirm the previously-broken file fields now emit
`"type": "file"`. Commit the regenerated artifacts in this phase
(updated artifacts are part of "done").

### Done When

- All new Level 1 tests pass.
- `cargo clippy --all-targets -p schematic-define -p schematic-definitions -p schematic-gen -- -D warnings` passes.
- Regenerated `schematic/postman/elevenlabs.postman_collection.json` and
  `schematic/postman/unfolded_circle_core_rest.postman_collection.json`
  emit `type: "file"` on at least one form field each.

---

## Phase 2 — Declare Postman Auth Variables (Finding 2)

### Goal

When a collection's auth references `{{bearerToken}}`, `{{apiKey}}`,
`{{username}}`, or `{{password}}`, those variables must be declared in the
collection's `variable` list. Dedupe with base-URL variables.

### Files

- **Modify**: `schematic/gen/src/postman_output.rs`
  - Add an internal helper:
    ```rust
    fn auth_variables(auth: &ExportAuth) -> Vec<PostmanVariable>;
    ```
    Returns the variable definitions implied by the given `ExportAuth`:
    - `Bearer { variable }` → one `PostmanVariable` keyed `"bearerToken"`,
      empty `value`, description `"Bearer token for Authorization header"`.
    - `ApiKey { variable, .. }` → one keyed `"apiKey"`, description
      `"API key value"`.
    - `Basic { username_var, password_var }` → two variables
      (`"username"`, `"password"`).
    - `None` → empty.
  - In `build_postman_collection` (line 246-300), append the result of
    `auth_variables(&auth)` to the collection's `variable` vector after
    the base-URL var. Dedupe by `key` (the base-URL var is `"baseUrl"`,
    which never collides with auth variable keys, but keep the dedupe step
    defensive).
  - In `build_postman_collection_grouped` (line 608-695), the auth shape
    will change in Phase 3. For now, append `auth_variables` for the
    collection-level auth using the same logic, but design the helper so
    Phase 3 can call it once per request-level auth too (i.e. accumulate
    a `BTreeMap<String, PostmanVariable>` and emit a sorted, deduped
    `Vec` at the end).
- **Modify**: existing tests in `postman_output.rs`:
  - `build_minimal_collection` (line 832-848) — `variable.len()` will
    still be 1 (auth is `None`), but assert the collection still validates.
  - `postman_collection_json_structure` (line 1168-1211) — use
    `BearerToken`; now assert the JSON contains
    `"variable":[..., { "key": "bearerToken", ... }]`.

### Tests

- **Level 1**:
  - `auth_variables_for_bearer_returns_one_variable_named_bearerToken`.
  - `auth_variables_for_api_key_returns_apiKey_variable`.
  - `auth_variables_for_basic_returns_username_and_password`.
  - `auth_variables_for_none_returns_empty`.
  - `build_postman_collection_bearer_declares_bearerToken` — build a
    collection with bearer auth, assert collection.variable contains a
    var with `key == "bearerToken"`.
  - `build_postman_collection_basic_declares_username_and_password`.
- **Level 2 (new in `schematic/gen/tests/postman_schema.rs` or a new
  file `schematic/gen/tests/postman_var_consistency.rs`)**:
  - `every_referenced_variable_is_declared_for_all_apis` — for each
    committed `schematic/postman/*.postman_collection.json`, parse it,
    walk the JSON looking for `{{name}}` references inside auth, url,
    header, query, and body fields, and assert every referenced name is
    either:
    - declared in `collection.variable[*].key`, **or**
    - is `baseUrl` / `baseUrl<N>` (always declared above), **or**
    - is on a small allowlist of intentionally-external vars (initially
      empty; widened only with explicit justification).

### Verification

```bash
cargo test -p schematic-define -p schematic-definitions -p schematic-gen
just -f schematic/justfile generate
cargo check -p schematic-schema --manifest-path schematic/schema/Cargo.toml
```

Confirm with `jq`:

```bash
jq '.variable[].key' schematic/postman/openai.postman_collection.json
# expect: "baseUrl" and "bearerToken"
```

### Done When

- All new Level 1 + Level 2 tests pass.
- Clippy clean for the three crates.
- Regenerated artifacts include the auth-variable declarations.

---

## Phase 3 — Mixed-Auth Handling for Grouped Collections (Finding 1)

### Goal

Detect mixed auth across grouped APIs. When uniform → keep collection-level
auth (current behaviour). When mixed → set collection auth to `None`,
attach per-request auth from the owning API, and disambiguate duplicate
request names by suffixing with the API name. Auth variables for *all*
auth modes used in the group are declared once at the collection level
(builds on Phase 2's helper).

### Files

- **Modify**: `schematic/gen/src/postman_output.rs`
  - Refactor `build_request_item` (line 367-395) to accept the request's
    owning auth instead of the collection-wide one:
    ```rust
    fn build_request_item(
        endpoint: &schematic_define::Endpoint,
        api: &RestApi,
        owning_auth: &ExportAuth,
        emit_request_level_auth: bool,
        disambiguate_with_api_name: bool,
    ) -> PostmanItem;
    ```
    - When `emit_request_level_auth` is `true`, set
      `request.auth = build_collection_auth(owning_auth)`.
    - When `disambiguate_with_api_name` is `true`, set
      `name = format!("{} ({})", endpoint.id, api.name)`.
  - Refactor `build_postman_collection_grouped` (line 608-695):
    1. Compute `let auths: Vec<ExportAuth> = apis.iter().map(|a| map_auth(&a.auth)).collect();`.
    2. `let uniform = auths.iter().all(|a| a == &auths[0]);`
    3. If `uniform`:
       - Keep current behaviour: collection-level auth from `auths[0]`,
         each request gets `auth: None` (inherits collection auth). Names
         do NOT need disambiguation.
    4. If `!uniform`:
       - `collection_auth = None` (omit collection-level auth).
       - For each request, set `auth = Some(build_collection_auth(...))`
         using its owning API's auth.
       - Detect duplicate `endpoint.id` across grouped APIs (build a
         `HashMap<String, usize>` first); for entries with `count > 1`,
         disambiguate by suffixing with the owning API name. Apply the
         suffix only to the duplicate occurrences (preserve unique names
         as-is) — keeps single-API cases tidy and only renames the
         actually-conflicting entries.
    5. Build collection-level `variable` list:
       - Start with base-URL vars (existing logic).
       - For uniform groups: add `auth_variables(&auths[0])`.
       - For mixed groups: union `auth_variables` across every distinct
         `ExportAuth` in `auths`, deduplicated by key. (The Phase-2
         helper signature already supports per-auth lookup; the new
         caller just iterates.)
  - Update the doctest on `build_postman_collection_grouped` (line 567-606)
    to reflect new return shape; or keep the doctest narrow and add a
    second doctest illustrating mixed auth.
- **Modify**: existing grouped tests in `postman_output.rs` if any rely
  on the old "first API wins" behaviour. Inspection of the file shows
  none currently assert mixed-auth shape; only the
  `postman_collection_json_structure` test exists for single-API.

### Tests

- **Level 1, `schematic/gen/src/postman_output.rs`**:
  - `grouped_uniform_auth_uses_collection_auth_and_no_request_auth` —
    two APIs both `BearerToken`; assert `collection.auth` is
    `Some(Bearer)` and every `request.auth` is `None`.
  - `grouped_mixed_auth_omits_collection_auth_and_emits_per_request` —
    one Basic + one Bearer API; assert `collection.auth` is `None`,
    every request has `Some(auth)` matching its owning API.
  - `grouped_mixed_auth_declares_both_auth_variable_sets` — assert the
    merged `variable` list contains `username`, `password`, *and*
    `bearerToken` (i.e. union of both auth families).
  - `grouped_mixed_auth_disambiguates_duplicate_request_names` — two
    APIs both expose `ListAlarms`; assert the resulting items have
    distinct names (`"ListAlarms (EmqxBasic)"` and
    `"ListAlarms (EmqxBearer)"` or equivalent) and no other names are
    rewritten.
  - `grouped_mixed_auth_preserves_unique_request_names` — names that
    only appear in one API stay verbatim.
- **Level 2, `schematic/gen/tests/postman_schema.rs`** (depends on
  Phase 0 helper):
  - `emqx_grouped_collection_validates_against_postman_schema` — load
    the regenerated `schematic/postman/emqx.postman_collection.json`
    and call `validate_postman_json()`; assert `Ok`.

### Verification

```bash
cargo test -p schematic-define -p schematic-definitions -p schematic-gen
just -f schematic/justfile generate
cargo check -p schematic-schema --manifest-path schematic/schema/Cargo.toml
```

`jq` spot-checks on the regenerated EMQX collection:

```bash
jq '.auth' schematic/postman/emqx.postman_collection.json
# expect: null

jq '[.item[] | .. | objects | select(.request?) | .request.auth.type] | unique' \
   schematic/postman/emqx.postman_collection.json
# expect: ["basic", "bearer"]

jq '[.item[] | .. | objects | select(.request?) | .name]
    | group_by(.) | map(select(length > 1))' \
   schematic/postman/emqx.postman_collection.json
# expect: []  (no duplicates remain)
```

### Done When

- All new Level 1 + Level 2 tests pass.
- Clippy clean for the three crates.
- Regenerated `schematic/postman/emqx.postman_collection.json` shows
  `auth: null`, mixed per-request auth types, and unique request names.

---

## Phase 4 — Enforce `validate_completeness()` in Generation Path (Finding 4)

### Goal

Before grouped OpenAPI export, validate every member API against the
merged registry and fail fast with a clear error listing the module, the
API, and missing schema names. No more dangling `$ref`s.

### Files

- **Modify**: `schematic/gen/src/openapi_output.rs`
  - In `write_openapi_grouped` (line 148+), immediately after the
    directory check and before the per-API export loop, add:
    ```rust
    // Strict completeness check: every member API must have all of its
    // referenced JSON response schemas registered in `registry`.
    // schematic_definitions::registry::SchemaRegistry exposes
    // validate_completeness, but `R: SchemaRegistryLike` does not. We
    // therefore add a `validate_completeness_for_apis` helper on the
    // schematic-definitions side so write_openapi_grouped can stay generic.
    ```
  - Because `write_openapi_grouped` is generic over
    `R: SchemaRegistryLike` (a trait in `schematic-define`), we cannot
    call `validate_completeness` directly on `R`. Two options:
    1. Extend `SchemaRegistryLike` in `schematic-define` with a default
       method `fn validate_completeness(api) -> Result<...>` that returns
       `Ok(())` by default.
    2. Move strict validation up into the **callers**
       (`run_generate`, `run_generate_all`) where the concrete
       `schematic_definitions::registry::SchemaRegistry` is available.

    **Decision**: option 2. It keeps `schematic-define` agnostic of any
    particular registry implementation and concentrates strictness in the
    generator binary, where errors are surfaced with the most context
    (module name, verbose-level reporting, exit codes). Document this
    decision in `tech-design.md` if the design doc mentioned moving
    the check into the writer.

- **Modify**: `schematic/gen/src/main.rs`
  - In `run_generate` (line 404-507), after resolving `registry` and
    before calling `run_openapi_export_grouped` (line 480-497), add:
    ```rust
    if let Err(missing) = registry.validate_completeness(&api) {
        return Err(GeneratorError::ConfigError(format!(
            "OpenAPI registry incomplete for module \"{module_name}\" \
             (API \"{api_name}\"): missing schema(s) {missing:?}. \
             Add JsonSchema derive + register::<T>(\"{first_missing}\") \
             entries in schematic-definitions, or skip with --no-openapi.",
            module_name = module_name,
            api_name = api.name,
            missing = missing,
            first_missing = missing.first().map(|s| s.as_str()).unwrap_or("…"),
        )));
    }
    ```
  - In `run_generate_all` (line 624-811), inside the per-module loop
    (line 705-735), after fetching `registry`, iterate every member API:
    ```rust
    for member in module_apis {
        if let Err(missing) = registry.validate_completeness(member) {
            return Err(GeneratorError::ConfigError(format!(
                "OpenAPI registry incomplete for module \"{module_name}\" \
                 (API \"{api_name}\"): missing schema(s) {missing:?}.",
                module_name = module_name,
                api_name = member.name,
                missing = missing,
            )));
        }
    }
    ```

- **Modify**: `schematic/gen/src/errors.rs` (only if the existing
  `ConfigError` variant feels semantically wrong). Prefer to reuse
  `ConfigError` to minimize churn — Phase 4 is correctness, not error
  taxonomy.

### Tests

- **Level 2, new file `schematic/gen/tests/openapi_strict_completeness.rs`**:
  - `single_api_with_missing_schema_fails_generation` — build a
    synthetic `RestApi` referencing `ApiResponse::json_type("Missing")`
    plus an empty `SchemaRegistry`. Call a function exposed for testing
    (or replicate the relevant slice of `run_generate` logic by directly
    calling `registry.validate_completeness(&api)`) and assert the
    returned error is `Err` containing `"Missing"`.
    *Implementation note*: since `run_generate` uses
    `schematic_definitions::registry::get_registries_for_module`, the
    cleanest test surface is to call `validate_completeness` directly
    and additionally invoke the generator binary with
    `assert_cmd` against a temporary working API. If `assert_cmd` is too
    heavy, restrict to direct `validate_completeness` regression tests
    (Level 1 in `schematic-definitions/src/registry.rs`).
  - `grouped_module_with_missing_schema_in_one_member_fails` — build
    two synthetic APIs sharing a module; one references a missing type.
    Assert that the strict check rejects the module before any file is
    written.
- **Level 1, `schematic/definitions/src/registry.rs`**: the existing
  `validate_completeness_*` tests already cover the registry side. Add a
  new test that confirms the error message (or returned `Vec<String>`)
  is suitable for embedding in the generator-level error
  (`Vec<String>` already covers this; no change needed in
  schematic-definitions).

### Verification

```bash
cargo test -p schematic-define -p schematic-definitions -p schematic-gen
# Should pass — current registries are complete.

just -f schematic/justfile generate
cargo check -p schematic-schema --manifest-path schematic/schema/Cargo.toml
# Should pass unchanged.
```

Manual regression check: temporarily comment out one
`.register::<Foo>("Foo")` line in any schematic-definitions module,
re-run `cargo run -p schematic-gen -- generate --api all`, confirm it
exits non-zero with the expected error message, then revert.

### Done When

- New Level 2 tests pass.
- Clippy clean.
- Manual regression check above produces the expected error.
- No artifact diffs in `schematic/openapi/` (this phase only changes
  the failure path, not the success shape).

---

## Phase 5 — Postman JSON Schema Validation + Golden Fixtures (Finding 5)

### Goal

Use the helper from Phase 0 to validate every committed Postman artifact
against Postman's published v2.1.0 schema. Add focused golden fixtures
that pin down the exact behaviour fixed in Phases 1-3.

### Files

- **Modify**: `schematic/gen/tests/postman_schema.rs` (or split into
  `postman_artifact_validation.rs`):
  - Add a single Level 2 test that walks every
    `schematic/postman/*.postman_collection.json`, parses to
    `serde_json::Value`, calls `validate_postman_json`, and reports any
    failures with the file name + JSON pointer location for fast triage.
- **New**: `schematic/gen/tests/fixtures/postman/golden/` directory
  containing golden fixtures committed to git:
  - `mixed_auth_emqx.json` — synthesized small fixture (handcrafted
    expected output, ~50 lines) covering: collection.auth = null,
    per-request mixed auth, declared `username`/`password`/`bearerToken`,
    disambiguated names.
  - `auth_variables_openai.json` — minimal single-API bearer collection
    asserting `bearerToken` is declared.
  - `file_upload_elevenlabs.json` — minimal collection with one
    multipart endpoint whose `audio` field is `type: "file"`.
  - `path_query_params_github.json` — minimal collection with one
    `/repos/{owner}/{repo}/issues?state=open` endpoint asserting
    Postman path variables and query entries are correct.
  - `grouped_module_ollama.json` — minimal grouped collection covering
    base-URL variable naming (`baseUrl`, `baseUrl2`) and merged folders.
- **New tests, `schematic/gen/tests/postman_golden.rs`**:
  - For each golden fixture, build the corresponding collection
    in-memory using the same input that produced the committed
    artifact, serialize, and `assert_eq!` against the committed JSON
    (use `pretty_assertions::assert_eq` if already available; otherwise
    plain `assert_eq` plus `serde_json::Value` round-tripping for
    deterministic ordering).
  - Each golden test also calls `validate_postman_json` on the produced
    JSON, so fixtures stay schema-valid even if hand-edited.

### Tests

All tests added in this phase are Level 2 (integration).

### Verification

```bash
cargo test -p schematic-define -p schematic-definitions -p schematic-gen
cargo clippy --all-targets -p schematic-define -p schematic-definitions -p schematic-gen -- -D warnings
```

Expect the entire suite (artifact validation + 5 golden fixtures + every
existing test) to pass against the artifacts already regenerated in
phases 1-3. No new generation pass needed in this phase — but if a
fixture comparison fails because of an unrelated artifact diff, fix the
fixture (not the generator).

### Done When

- Every committed artifact passes `validate_postman_json`.
- All five golden fixtures pass strict equality.
- Clippy clean.

---

## Phase 6 — Repository-Wide Schematic Verification

### Goal

Final pass: regenerate everything, confirm artifacts are stable, run the
full test + clippy matrix, and ensure committed artifacts in
`schematic/openapi/` and `schematic/postman/` are exactly what the
generator produces today.

### Files

- No production source changes.
- Possible doc updates:
  - `schematic/docs/io/export-postman.md` — note the new behaviour for
    mixed-auth groups, declared auth variables, and file fields.
  - `schematic/README.md` and `schematic/gen/README.md` if they document
    Postman semantics.
  - `schematic/features/ergonomics-and-postman-projects/review-2.md` —
    flip frontmatter `ready: true` once verification is green (only
    after a real reviewer pass — leave this for the human).

### Verification (run all in order)

```bash
# 1. Full test matrix.
cargo test -p schematic-define -p schematic-definitions -p schematic-gen

# 2. Workspace-wide clippy on the touched crates.
cargo clippy --all-targets -p schematic-define -p schematic-definitions -p schematic-gen -- -D warnings

# 3. Regenerate artifacts and verify schema crate compiles.
just -f schematic/justfile generate
cargo check -p schematic-schema --manifest-path schematic/schema/Cargo.toml

# 4. Confirm zero git drift after regeneration.
git status schematic/openapi schematic/postman
# Expect: nothing to commit (artifacts already up to date).

# 5. Drift test (existing).
cargo test -p schematic-gen artifact_drift -- --ignored

# 6. Schema and golden fixture validation.
cargo test -p schematic-gen --test postman_schema
cargo test -p schematic-gen --test postman_golden  # if separated
```

### Done When

- All commands above succeed.
- `git status` shows no unintended diffs in `schematic/openapi/` or
  `schematic/postman/` after `just generate`.
- `schematic/features/ergonomics-and-postman-projects/review-2.md`
  findings 1-5 each have at least one passing test in this plan that
  would have caught the original regression.

---

## Risk Notes

- **`jsonschema` crate version**: pin to a draft-07-compatible version
  (the Postman schema declares `draft-07`). If the workspace already
  uses `jsonschema`, reuse that version. Otherwise add the smallest
  feasible dep — Phase 0 lives entirely in `[dev-dependencies]` so it
  cannot leak into downstream consumers of `schematic-gen`.
- **Artifact churn**: phases 1-3 each regenerate artifacts. Land them in
  the order given so reviewers see one shape change per phase.
- **`validate_completeness` location**: keeping the strict check inside
  `main.rs` (Phase 4) instead of inside `write_openapi_grouped` deviates
  from the literal wording of the tech design. The plan documents the
  rationale (registry-trait genericity) so the design doc and the code
  match intent.
- **Mixed-auth name disambiguation**: the suffix style
  `"<Id> (<ApiName>)"` is chosen to keep the original `Id` searchable
  and to avoid breaking anyone scripting against unique-id assumptions.
  If reviewers prefer a different format, change it in one place
  (`build_request_item`'s name builder).

