# Ergonomics and Postman Projects — Implementation Plan

## Overview

This plan implements the [spec](spec.md) and [tech design](tech-design.md) across three phases. Each phase is self-contained and delivers testable value.

**Current state summary:**

- OpenAPI export exists (`schematic-define::openapi::export`) but is opt-in via `--openapi-out`
- Schema registries exist only for `openai` and `samsung_smart_tv`
- No Postman export code exists
- No shared export helpers exist
- No grouped/module-centric export path exists
- 16 REST APIs defined in `schematic-definitions`

---

## Phase 1: Postman Exporter and Artifact Plumbing

**Goal:** Add Postman collection generation, shared export helpers, and wire both OpenAPI and Postman into the generation pipeline with CLI flags.

### 1.1 — Shared Export Helpers

Create `schematic/gen/src/export/` module with normalized representations that both OpenAPI and Postman writers consume.

**Files to create:**

| File | Purpose |
|------|---------|
| `export/mod.rs` | Module root, re-exports |
| `export/http.rs` | `ExportEndpoint` — normalized HTTP request: method, path, description, folder key |
| `export/naming.rs` | Module name resolution from `RestApi` (lowercase name or `module_path` override) |
| `export/path_params.rs` | Extract `{param}` segments from path strings (reuse logic from `parser.rs`) |
| `export/auth.rs` | `ExportAuth` — normalized auth metadata: strategy kind, variable names, header names |
| `export/body.rs` | `ExportBody` — normalized request body: content type, mode (json/form/urlencoded/text/binary) |

**Key types:**

```rust
// export/http.rs
pub struct ExportEndpoint {
    pub id: String,
    pub method: RestMethod,
    pub path: String,
    pub description: String,
    pub folder_key: Option<String>,     // inferred from path
    pub path_params: Vec<String>,
    pub query_params: Vec<ExportParam>,
    pub headers: Vec<(String, String)>,
    pub body: Option<ExportBody>,
    pub auth_override: Option<ExportAuth>, // when per-endpoint auth differs
}

// export/naming.rs
pub fn resolve_module_name(api: &RestApi) -> String;

// export/auth.rs
pub enum ExportAuth {
    Bearer { variable: String },
    ApiKey { header: String, variable: String },
    Basic { username_var: String, password_var: String },
    None,
}
pub fn map_auth(strategy: &AuthStrategy) -> ExportAuth;

// export/body.rs
pub enum ExportBody {
    Json { content_type: String },
    FormData { fields: Vec<FormField> },
    UrlEncoded { fields: Vec<FormField> },
    Text { content_type: String },
    Binary,
}

// export/path_params.rs
pub fn extract_folder_key(path: &str) -> Option<String>;
```

**Foldering algorithm** (from tech design §Foldering strategy):

1. Strip leading `/`
2. Ignore path variables `{...}`
3. Ignore version prefixes (`v1`, `v2`, etc.)
4. First remaining segment = folder key
5. No stable segment → `None` (collection root)

**Tests:**

- Unit tests for each helper in `export/` submodules
- Folder key extraction: `/models` → `models`, `/repos/{owner}/{repo}/issues` → `repos`, `/v1/audio/speech` → `audio`, `/` → `None`
- Auth mapping for each `AuthStrategy` variant
- Body mapping for each `ApiRequest` variant
- Module name resolution with and without `module_path` override

### 1.2 — Postman Writer

Create `schematic/gen/src/postman_output.rs` with strongly typed serde structs for Postman Collection v2.1.0.

**Serde types:**

```rust
#[derive(Serialize)]
pub struct PostmanCollection {
    pub info: PostmanInfo,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub variable: Vec<PostmanVariable>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<PostmanAuth>,
    pub item: Vec<PostmanItem>,
}

#[derive(Serialize)]
pub struct PostmanInfo {
    pub name: String,
    pub description: String,
    pub schema: String, // always "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum PostmanItem {
    Folder { name: String, item: Vec<PostmanItem> },
    Request { name: String, request: PostmanRequest },
}

#[derive(Serialize)]
pub struct PostmanRequest {
    pub method: String,
    pub url: PostmanUrl,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<PostmanAuth>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub header: Vec<PostmanHeader>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<PostmanBody>,
}

#[derive(Serialize)]
pub struct PostmanUrl {
    pub raw: String,
    pub host: Vec<String>,
    pub path: Vec<PostmanPathSegment>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub query: Vec<PostmanQuery>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub variable: Vec<PostmanVariable>,
}

#[derive(Serialize)]
pub struct PostmanAuth { /* type + fields per auth kind */ }
#[derive(Serialize)]
pub struct PostmanVariable { pub key: String, pub value: String, pub description: Option<String> }
#[derive(Serialize)]
pub struct PostmanHeader { pub key: String, pub value: String }
#[derive(Serialize)]
pub struct PostmanBody { pub mode: String, /* raw/formdata/urlencoded/file */ }
#[derive(Serialize)]
pub struct PostmanQuery { pub key: String, pub value: String, pub description: Option<String> }
```

**Public API:**

```rust
/// Build a Postman collection from a single RestApi.
pub fn build_postman_collection(api: &RestApi) -> PostmanCollection;

/// Build a Postman collection from grouped RestApis (shared module).
pub fn build_postman_collection_grouped(apis: &[&RestApi], module_name: &str) -> PostmanCollection;

/// Write a Postman collection to disk.
pub fn write_postman(api: &RestApi, dir: &Path, dry_run: bool) -> Result<PathBuf, GeneratorError>;

/// Write a grouped Postman collection to disk.
pub fn write_postman_grouped(apis: &[&RestApi], module_name: &str, dir: &Path, dry_run: bool) -> Result<PathBuf, GeneratorError>;
```

**File naming:** `<module_name>.postman_collection.json`

**Auth mapping** (from tech design §Auth mapping):

| Schematic | Postman | Variable |
|-----------|---------|----------|
| `BearerToken { header: None }` | `bearer` | `{{bearerToken}}` |
| `BearerToken { header: Some(h) }` | `apikey` (header) | `{{apiKey}}` |
| `ApiKey { header }` | `apikey` (header) | `{{apiKey}}` |
| `Basic` | `basic` | `{{username}}`, `{{password}}` |
| `None` | `noauth` | — |

**Body mapping** (from tech design §Body mapping):

| Schematic | Postman mode | Content-Type |
|-----------|-------------|--------------|
| `Json(Schema)` | `raw` | `application/json` |
| `FormData` | `formdata` | — |
| `UrlEncoded` | `urlencoded` | — |
| `Text` | `raw` | declared content type |
| `Binary` | `file` | — |

**Tests:**

- Unit tests for each auth mapping
- Unit tests for each body mapping
- Unit tests for foldering behavior
- Unit tests for URL construction with path params and query params
- Snapshot test: build collection for a minimal `RestApi` fixture, assert JSON structure

### 1.3 — CLI Flag Extensions

Modify `schematic/gen/src/main.rs` to add Postman flags alongside existing OpenAPI flags.

**New CLI flags on `generate` command:**

```
--postman-out <DIR>    Directory for Postman collection output
--no-openapi           Skip OpenAPI generation
--no-postman           Skip Postman generation
```

**Default resolution** (from tech design §CLI flags):

- When output is `schematic/schema/src` and no explicit `--openapi-out` / `--postman-out`:
  - OpenAPI defaults to `schematic/openapi`
  - Postman defaults to `schematic/postman`
- `--no-openapi` and `--no-postman` suppress the respective outputs

**Implementation:**

Add fields to the existing `Generate` variant in the CLI args enum:

```rust
#[arg(long, value_name = "DIR")]
postman_out: Option<PathBuf>,

#[arg(long)]
no_openapi: bool,

#[arg(long)]
no_postman: bool,
```

Update `run_generate()` and `run_generate_all()` to call `write_postman()` when Postman output is enabled.

### 1.4 — Output Directory Setup

**Directories to create (if not present):**

- `schematic/openapi/` — already exists (has `specs/` subdirectory)
- `schematic/postman/` — new directory, created on first generation

**`.gitignore` considerations:** Generated artifacts should be committed (they are the deliverable). No gitignore changes needed.

### 1.5 — Justfile Updates

Update `schematic/justfile`:

```just
# Existing (modify)
generate:
    cargo run -p schematic-gen -- generate --api all --output schema/src

# Update to include artifact generation
generate:
    cargo run -p schematic-gen -- generate --api all --output schema/src \
        --openapi-out openapi --postman-out postman

generate-one api:
    cargo run -p schematic-gen -- generate --api {{api}} --output schema/src \
        --openapi-out openapi --postman-out postman

# New helpers
generate-openapi:
    cargo run -p schematic-gen -- generate --api all --output schema/src \
        --openapi-out openapi --no-postman

generate-postman:
    cargo run -p schematic-gen -- generate --api all --output schema/src \
        --postman-out postman --no-openapi
```

### 1.6 — Phase 1 Testing

**Unit tests** (in respective modules):

- `export/` helpers: ~20 tests covering normalization edge cases
- `postman_output.rs`: ~15 tests covering struct construction, auth, body, folders

**Golden artifact tests:**

Create `schematic/gen/tests/fixtures/postman/` with:

- `openai.postman_collection.json` — expected Postman output for OpenAI API
- A minimal synthetic API fixture for edge cases

Add `schematic/gen/tests/postman_generation.rs`:

- Build collection from OpenAI definition → compare against golden file
- Build collection from synthetic fixture → compare against golden file

**Structural validation:**

- Vendor the Postman v2.1.0 JSON Schema at `schematic/gen/tests/fixtures/postman/postman-collection-v2.1.0-schema.json`
- Add a test that validates generated collections against the vendored schema using `jsonschema` crate (dev dependency)

### Phase 1 Deliverables

- [ ] `schematic/gen/src/export/` module (5 files)
- [ ] `schematic/gen/src/postman_output.rs`
- [ ] CLI flags: `--postman-out`, `--no-openapi`, `--no-postman`
- [ ] Updated justfile recipes
- [ ] `schematic/postman/` directory with initial generated collections
- [ ] Unit tests for export helpers and Postman writer
- [ ] Golden artifact tests for Postman
- [ ] Structural validation test against v2.1.0 schema

---

## Phase 2: Complete OpenAPI Registry Coverage and Grouped Export

**Goal:** Add schema registries for all REST APIs, implement module-grouped OpenAPI export, and make OpenAPI output comprehensive.

### 2.1 — Registry Expansion

Add `openapi_registry()` functions to each API module in `schematic-definitions`.

**APIs needing registries (14 remaining):**

| API | Module | Complexity | Notes |
|-----|--------|------------|-------|
| `anthropic` | `anthropic` | Medium | Message types, content blocks |
| `bitbucket` | `bitbucket` | Medium | Repository, PR types |
| `elevenlabs` | `elevenlabs` | Low | Voice, generation types |
| `emqx` (Basic) | `emqx` | Medium | Shared module with Bearer |
| `emqx` (Bearer) | `emqx` | Medium | Shared module with Basic |
| `eversolo` | `eversolo` | Low | Device control types |
| `gitea` | `gitea` | Medium | Repository types |
| `github` | `github` | High | Many endpoint response types |
| `gitlab` | `gitlab` | Medium | Project, MR types |
| `huggingface` | `huggingface` | Medium | Model, dataset types |
| `lmstudio` | `lmstudio` | Low | Model, completion types |
| `ollama` (Native) | `ollama` | Medium | Shared module with OpenAI |
| `ollama` (OpenAI) | `ollama` | Medium | Shared module with Native |
| `unfolded_circle` | `unfolded_circle` | Medium | Multiple sub-APIs |

**Pattern for each:** Add `#[derive(JsonSchema)]` to response types and create registry:

```rust
use schemars::JsonSchema;
use schematic_define::registry::SchemaRegistry;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ModelResponse { /* fields */ }

pub fn openapi_registry() -> SchemaRegistry {
    let mut registry = SchemaRegistry::new();
    registry.register::<ModelResponse>("ModelResponse");
    // ... more types
    registry
}
```

**Approach:** This is the largest scope item. Tackle APIs in order of complexity (low → high) to build momentum. Some APIs may have response types defined only as `ApiResponse::Json(Schema::Any)` — these need concrete types added.

### 2.2 — Grouped Registry Lookup

Extend `schematic-definitions` to support grouped registry queries.

**Add to `schematic-definitions/src/lib.rs`:**

```rust
/// Returns a combined registry for all APIs that share a module.
pub fn grouped_registry(module_name: &str) -> Option<SchemaRegistry>;

/// Returns all RestApi definitions grouped by resolved module name.
pub fn apis_by_module() -> IndexMap<String, Vec<&'static RestApi>>;
```

**Shared-module groups identified:**

- `ollama` → `OllamaNative` + `OllamaOpenAI`
- `emqx` → `EmqxBasic` + `EmqxBearer`

### 2.3 — Grouped OpenAPI Writer

Add `write_openapi_grouped()` to `schematic/gen/src/openapi_output.rs`.

**Behavior:**

- Accept `&[&RestApi]` for a module group
- Merge endpoints from all APIs into one OpenAPI document
- Merge security schemes from all APIs (different auth → multiple schemes)
- Per-operation security requirements reflect the originating API's auth
- Single output file per module: `<module>.json` or `<module>.yaml`

**New function in `schematic-define::openapi::export`:**

```rust
pub fn export_grouped<R: SchemaRegistryLike>(
    apis: &[&RestApi],
    module_name: &str,
    registries: &[&R],
    options: &ExportOptions,
) -> Result<openapiv3::OpenAPI, OpenApiError>;
```

**Merge rules:**

- `info.title` = module name (title case)
- `info.description` = concatenation or first non-empty
- `servers` = deduplicated union
- `paths` = union (error on collision)
- `components.securitySchemes` = union keyed by scheme name
- Per-operation `security` = from the originating `RestApi.auth`

### 2.4 — Refactor `run_generate_all()`

Update the batch generation path in `schematic/gen/src/main.rs`:

1. Collect all `RestApi` definitions
2. Group by resolved module name (using `export::naming::resolve_module_name`)
3. For each group:
   - Generate Rust client code (existing behavior)
   - Export OpenAPI document (grouped if >1 API)
   - Export Postman collection (grouped if >1 API)
4. Report results

**Migration behavior:** During Phase 2, APIs without registries emit a warning and skip OpenAPI export but still generate Postman collections.

### 2.5 — OpenAPI Versioning

Replace hardcoded `1.0.0` in OpenAPI export (tech design §Versioning):

1. CLI `--openapi-version` override → highest priority
2. `RestApi.version` field → if set
3. Fallback `"0.1.0"`

**Changes:**

- Add `--openapi-version <VERSION>` to CLI
- Update `ExportOptions` to accept version override
- Update `export()` to use the priority chain

### 2.6 — Phase 2 Testing

**Unit tests:**

- Grouped export: merge two APIs, verify paths/security union
- Grouped registry: query `ollama`, get combined schemas
- Version resolution priority chain

**Golden artifact tests:**

Add to `schematic/gen/tests/fixtures/`:

- `openapi/ollama.json` — expected grouped OpenAPI for Ollama
- `openapi/emqx.json` — expected grouped OpenAPI for EMQX
- `openapi/openai.json` — expected single-API OpenAPI

Add `schematic/gen/tests/openapi_generation.rs`:

- Generate OpenAPI for `openai` → compare against golden file
- Generate grouped OpenAPI for `ollama` → compare against golden file
- Generate grouped OpenAPI for `emqx` → compare against golden file

**Structural validation:**

- Parse emitted JSON back into `openapiv3::OpenAPI` → assert required fields
- Verify all `$ref` targets resolve within the document

**End-to-end tests:**

Extend `schematic/gen/tests/e2e_generation.rs`:

- `--api openai` generates Rust + OpenAPI + Postman
- `--api all` generates complete artifact set
- `ollama` and `emqx` produce grouped artifacts
- Dry-run produces no files
- Verify expected filenames and parsability

### Phase 2 Deliverables

- [ ] `#[derive(JsonSchema)]` and `openapi_registry()` for all 16 APIs
- [ ] `apis_by_module()` and `grouped_registry()` in definitions
- [ ] `export_grouped()` in `schematic-define::openapi::export`
- [ ] `write_openapi_grouped()` in `schematic-gen::openapi_output`
- [ ] Refactored `run_generate_all()` with module grouping
- [ ] `--openapi-version` CLI flag
- [ ] `schematic/openapi/` populated with all API specs
- [ ] Golden artifact tests for OpenAPI
- [ ] Structural validation tests for OpenAPI
- [ ] Extended E2E tests

---

## Phase 3: Make Artifacts Default and Strict

**Goal:** Flip generation to always emit all artifacts, remove warn-and-skip, add drift detection.

### 3.1 — Default-On Behavior

Remove the requirement for explicit `--openapi-out` / `--postman-out` flags.

**Change:** When `generate` runs without explicit output dir overrides and the Rust output is `schematic/schema/src`:

- OpenAPI defaults to `schematic/openapi/`
- Postman defaults to `schematic/postman/`
- `--no-openapi` and `--no-postman` still suppress

This means `just generate` and `just generate-one` produce all three artifacts by default with no flag changes needed (the justfile already passes the dirs from Phase 1).

### 3.2 — Strict Failure Policy

Remove warn-and-skip behavior for missing registries.

**Change:** `run_generate_all()` fails with a clear error when:

- A REST API lacks a schema registry (unless `--no-openapi` is set)
- OpenAPI serialization fails
- Postman serialization fails

**Error format:**

```
Error: Missing schema registry for API "Gitea" (module: gitea)
  → Add `openapi_registry()` to schematic-definitions/src/gitea/mod.rs
  → Or skip with --no-openapi
```

### 3.3 — Artifact Drift Detection

Add a test that verifies committed artifacts match what generation would produce.

**Implementation:**

Add `schematic/gen/tests/artifact_drift.rs`:

```rust
#[test]
#[ignore] // run explicitly or in CI
fn generated_artifacts_are_up_to_date() {
    // 1. Run generation to a temp dir
    // 2. Compare each file against committed version
    // 3. Fail with diff on mismatch
}
```

**Justfile recipe:**

```just
check-drift:
    cargo test -p schematic-gen artifact_drift -- --ignored
```

### 3.4 — Stale Artifact Cleanup

When regenerating, remove files in the output directories that no longer correspond to any API.

**Implementation:** Before writing, collect the set of expected filenames. After writing, delete any existing files in the output directory not in that set. Log deletions.

### 3.5 — Documentation Updates

Update in the same changeset:

| Document | Changes |
|----------|---------|
| `schematic/README.md` | Add OpenAPI and Postman sections, output layout |
| `schematic/gen/README.md` | Document new CLI flags, generation workflow |
| `schematic/docs/io/export-openapi.md` | Update with grouped export, versioning |
| `schematic/docs/io/export-postman.md` | New file documenting Postman generation |
| `schematic/docs/postman.md` | Add implementation notes referencing the generator |

### Phase 3 Deliverables

- [ ] Default-on artifact generation (no explicit flags needed)
- [ ] Strict failure on missing registries
- [ ] Artifact drift detection test
- [ ] Stale artifact cleanup
- [ ] Documentation updates
- [ ] `just check-drift` recipe

---

## Dependency Changes

### `schematic-gen` (`Cargo.toml`)

**New dependencies:**

```toml
[dev-dependencies]
jsonschema = "0.29"   # Postman schema validation in tests
```

No new runtime dependencies expected — `serde_json` is already present.

### `schematic-definitions` (`Cargo.toml`)

Already depends on `schemars` and `openapiv3`. No changes needed for Phase 1. Phase 2 may require `schemars` derives on new types.

### `schematic-define` (`Cargo.toml`)

No changes expected. The `openapi` feature already provides everything needed.

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| **Shared-module export complexity** | Phase 1 works per-API; grouping deferred to Phase 2 with dedicated tests |
| **Schema registry scope** | Phase 2 is ordered low→high complexity; Postman works without registries |
| **Artifact churn / noisy diffs** | All serialization uses `serde_json::to_string_pretty` with stable key ordering; BTreeMap for maps |
| **Postman format correctness** | Vendored JSON Schema validation in tests catches format regressions |
| **Breaking existing generation** | Phase 1 is additive (new flags, new outputs); existing behavior unchanged until Phase 3 |

---

## Execution Order

```
Phase 1.1  Export helpers         ─┐
Phase 1.2  Postman writer         ─┤── Can develop in parallel
Phase 1.3  CLI flags              ─┘
Phase 1.4  Output directories      ← After 1.3
Phase 1.5  Justfile updates        ← After 1.3
Phase 1.6  Phase 1 tests          ← After 1.1 + 1.2

Phase 2.1  Registry expansion     ─┐
Phase 2.2  Grouped registry       ─┤── Parallel tracks
Phase 2.3  Grouped OpenAPI writer ─┘
Phase 2.4  Refactor run_generate_all  ← After 2.1-2.3
Phase 2.5  Versioning                 ← After 2.3
Phase 2.6  Phase 2 tests             ← After 2.4

Phase 3.1  Default-on             ← After Phase 2 complete
Phase 3.2  Strict failure         ← After 3.1
Phase 3.3  Drift detection        ← After 3.1
Phase 3.4  Stale cleanup          ← After 3.1
Phase 3.5  Documentation          ← After 3.1-3.4
```
