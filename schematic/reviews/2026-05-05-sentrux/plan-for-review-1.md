---
phases: 6
start_phase: 2
source_files_during_phase_2:
  - schematic/gen/src/commands.rs
  - schematic/gen/src/export/postman.rs
  - schematic/gen/src/export/openapi.rs
  - schematic/gen/src/export/mod.rs
  - schematic/gen/src/codegen/request_structs/mod.rs
  - schematic/gen/src/codegen/request_structs/shared.rs
  - schematic/gen/src/codegen/request_structs/single.rs
  - schematic/gen/src/codegen/request_structs/body.rs
  - schematic/gen/src/codegen/request_structs/multipart.rs
  - schematic/gen/src/codegen/request_structs/urlencoded.rs
  - schematic/gen/src/codegen/request_structs/paginated.rs
  - schematic/gen/src/main.rs
  - schematic/gen/src/lib.rs
  - schematic/gen/src/postman_output.rs
  - schematic/gen/src/openapi_output.rs
  - schematic/gen/src/output/ws_modules.rs
  - schematic/define/src/headers/builder.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
  - schematic-gen
  - schematic-define
---

# Implementation Plan: Schematic Sentrux Review 1

Addresses all 23 suggestions from `review-1.md`. Phases are ordered by dependency: `define` refactoring first (downstream crates depend on it), then `gen` critical path (prerequisite for schema output changes), then `definitions` and `schema` improvements, and finally low-priority cleanup.

---

## Phase 1: schematic-define Structural Refactoring

**Review items addressed:** 5 (3 urgent, 1 important, 1 nice-to-have)

No downstream API changes. All public re-exports preserved byte-identical. Run `cargo test -p schematic-define` after each sub-step.

### 1.1 Split `headers.rs` (2 005 LOC) into submodule `[urgent]`

**Files:**
- `schematic/define/src/headers.rs` → delete
- `schematic/define/src/headers/mod.rs` → new (re-exports all public items)
- `schematic/define/src/headers/sensitive.rs` → new (`SensitiveString`)
- `schematic/define/src/headers/env.rs` → new (`EnvList`, `ApiKeyEnv`, `EnvMapping`)
- `schematic/define/src/headers/builder.rs` → new (`Headers` + impl blocks)
- `schematic/define/src/headers/error.rs` → new (`HeaderError`)
- `schematic/define/src/lib.rs` → unchanged (already `pub mod headers;` + `pub use headers::{...}`)

**Steps:**
1. Create `schematic/define/src/headers/` directory.
2. Move `SensitiveString` to `sensitive.rs` with its `impl` blocks and tests.
3. Move `EnvList`, `ApiKeyEnv`, `EnvMapping` to `env.rs`.
4. Move `Headers` struct + all `impl Headers`, `impl Default for Headers` blocks to `builder.rs`.
5. Move `HeaderError` to `error.rs`.
6. Write `mod.rs` with `mod sensitive; mod env; mod builder; mod error;` plus `pub use` for each public symbol — mirroring the current `pub use headers::{...}` in `lib.rs`.
7. Update internal `use crate::headers::*` in `auth.rs`, `types.rs`, `request.rs`, `oauth.rs` — these resolve through `mod.rs` and need no change.
8. Run `cargo test -p schematic-define`.

**Internal imports to verify:**
- `oauth.rs:184` — `use crate::headers::{EnvList, EnvMapping}`
- `oauth.rs:203` — `use crate::headers::EnvMapping`
- `types.rs:13` — `use crate::headers::{EnvList, EnvMapping}`
- `types.rs:326` — `crate::headers::ApiKeyEnv { ... }`
- `types.rs:672` — `use crate::headers::ApiKeyEnv`

**Test gates:**
- `cargo test -p schematic-define` (unit + integration)
- `cargo test -p schematic-define --test headers_integration`

### 1.2 Split `params.rs` (1 561 LOC) — extract pagination `[urgent]`

**Files:**
- `schematic/define/src/params.rs` → keep (parameter types only)
- `schematic/define/src/pagination.rs` → new (`PaginationStyle`, `PaginationResponse`)
- `schematic/define/src/lib.rs` → add `pub mod pagination;` + `pub use pagination::{PaginationStyle, PaginationResponse};`

**Steps:**
1. Identify and extract `PaginationStyle` and `PaginationResponse` (plus all their impl blocks and tests) from `params.rs` into `pagination.rs`.
2. Add `pub mod pagination;` to `lib.rs`.
3. Re-export `PaginationStyle`, `PaginationResponse` from crate root so `schematic_define::PaginationStyle` still resolves.
4. Update `pub use` in `lib.rs` line 162 to include pagination re-exports.
5. Update `prelude.rs` line 50 — already `pub use crate::params::{...PaginationStyle}`; add `pub use crate::pagination::{PaginationStyle, PaginationResponse}` instead.
6. Run `cargo test -p schematic-define`.

**Downstream consumers to verify:**
- `schematic/definitions/src/gitea/mod.rs:796` — `use schematic_define::params::PaginationStyle`
- `schematic/definitions/src/gitlab/mod.rs:888` — `use schematic_define::params::PaginationStyle`
- These can remain as-is since `params.rs` can re-export from `pagination`, or migrate to `schematic_define::pagination::PaginationStyle`.

### 1.3 Reorganise `lib.rs` re-exports into layered groups `[urgent]`

**Files:**
- `schematic/define/src/lib.rs` → add `pub mod core`, `pub mod transport`, `pub mod model` group modules

**Steps:**
1. After the existing `pub mod` declarations (lines 135–148), add layered re-export modules:
   ```rust
   pub mod core {
       pub use crate::auth::*;
       pub use crate::types::*;
       pub use crate::request::*;
       pub use crate::response::*;
       pub use crate::schema::*;
   }
   pub mod transport {
       pub use crate::headers::*;
       pub use crate::params::*;
       pub use crate::pagination::*;
       pub use crate::websocket::*;
   }
   pub mod model {
       pub use crate::models::*;
   }
   ```
2. Keep all existing flat `pub use` re-exports (lines 150–171) for backward compatibility.
3. Run `cargo test -p schematic-define`.
4. Migrate internal consumers in `gen` and `definitions` to use layered imports in a follow-up pass (non-breaking).

### 1.4 Add acyclicity guardrail between `auth`, `headers`, `oauth` `[important]`

**Files:**
- `schematic/.sentrux/rules.toml` → create or append

**Steps:**
1. Create/edit `schematic/.sentrux/rules.toml`:
   ```toml
   [[layer]]
   name = "define-internal"
   order = ["headers", "auth", "oauth"]
   ```
2. Add a doc-comment in `schematic/define/src/oauth.rs` header documenting the intended layer order: `headers (lowest) → auth → oauth (highest)`.
3. This is documentation/configuration only — no code change.

### 1.5 Compact `openapi/import/mappings.rs` (1 284 LOC) `[nice-to-have]`

**Files:**
- `schematic/define/src/openapi/import/mappings.rs` → delete
- `schematic/define/src/openapi/import/mappings/` → new directory
- `schematic/define/src/openapi/import/mappings/mod.rs` → new
- `schematic/define/src/openapi/import/mappings/schema.rs` → new
- `schematic/define/src/openapi/import/mappings/parameters.rs` → new
- `schematic/define/src/openapi/import/mappings/responses.rs` → new

**Steps:**
1. Analyze `mappings.rs` to identify logical groupings (schema mapping, parameter mapping, response mapping, etc.).
2. Split into per-OpenAPI-element files under `mappings/` subdirectory.
3. `mod.rs` re-exports all public symbols.
4. No public API change — `openapi::import::mappings::*` still resolves.
5. Run `cargo test -p schematic-define --features openapi`.

**Test gates:** `cargo test -p schematic-define --features openapi --test openapi_tests`

---

## Phase 2: schematic-gen Critical Pipeline Refactoring

**Review items addressed:** 4 (1 critical, 3 urgent)

This phase is a prerequisite for Phase 5 (schema output changes). Run `cargo test -p schematic-gen` and all integration tests after each sub-step.

### 2.1 Carve `output.rs` (2 166 LOC) into a pipeline `[critical]`

**Files:**
- `schematic/gen/src/output.rs` → delete
- `schematic/gen/src/output/` → new directory
- `schematic/gen/src/output/mod.rs` → new (re-exports + entry point `write_api_module`)
- `schematic/gen/src/output/options.rs` → new (`OutputOptions`)
- `schematic/gen/src/output/ws_modules.rs` → new (WS_DEFINITION_MODULES table + helpers)
- `schematic/gen/src/output/assemble.rs` → new (`generate_module_tokens`)
- `schematic/gen/src/output/format.rs` → new (syn validate + prettyplease formatting)
- `schematic/gen/src/output/write.rs` → new (atomic temp-file write)
- `schematic/gen/src/lib.rs` → update `pub mod output;` (remains)
- `schematic/gen/src/main.rs` → update imports (lines 34–35)

**Steps:**
1. Create `output/` directory.
2. Extract `OutputOptions` struct + impl into `options.rs`.
3. Extract WS_DEFINITION_MODULES map + snake_case helper into `ws_modules.rs`.
4. Extract the token assembly logic (`generate_module_tokens` or equivalent) into `assemble.rs`.
5. Extract syn validation + prettyplease formatting into `format.rs`.
6. Extract the atomic write logic into `write.rs`.
7. Write `mod.rs` with the public entry point:
   ```rust
   pub use options::OutputOptions;

   pub fn generate_and_write(api: &RestApi, dir: &Path, dry_run: bool)
       -> Result<String, GeneratorError>
   { ... }

   pub fn generate_and_write_all(apis: &[RestApi], dir: &Path, dry_run: bool)
       -> Result<(), GeneratorError>
   { ... }
   ```
8. Update `main.rs` imports: `use schematic_gen::output::{generate_and_write, generate_and_write_all};` still resolves.
9. Update `lib.rs` if needed.

**Test gates:**
- `cargo test -p schematic-gen`
- `cargo test -p schematic-gen --test e2e_generation`
- `cargo test -p schematic-gen --test artifact_drift`

### 2.2 Slim `main.rs` (1 185 LOC) — move non-CLI logic into library `[urgent]`

**Files:**
- `schematic/gen/src/main.rs` → strip to CLI parsing + dispatch
- `schematic/gen/src/lib.rs` → re-export new submodules
- `schematic/gen/src/commands.rs` → new (dispatch logic)
- `schematic/gen/src/pipeline.rs` → new (orchestration logic extracted from main)

**Steps:**
1. Identify all non-CLI functions in `main.rs` (anything not clap-related or `fn main()`).
2. Move orchestration functions into `pipeline.rs` in the library.
3. Move command dispatch logic into `commands.rs`.
4. Leave `main.rs` with: clap arg parsing + `commands::dispatch(args)` call.
5. Re-export from `lib.rs`: `pub mod commands; pub mod pipeline;`.
6. Run `cargo test -p schematic-gen`.

### 2.3 Break `codegen/request_structs.rs` (2 172 LOC) by struct shape `[urgent]`

**Files:**
- `schematic/gen/src/codegen/request_structs.rs` → delete
- `schematic/gen/src/codegen/request_structs/` → new directory
- `schematic/gen/src/codegen/request_structs/mod.rs` → new (dispatch entry point)
- `schematic/gen/src/codegen/request_structs/single.rs` → new (single-param ergonomics)
- `schematic/gen/src/codegen/request_structs/body.rs` → new (body-only)
- `schematic/gen/src/codegen/request_structs/multipart.rs` → new (FormData/file fields)
- `schematic/gen/src/codegen/request_structs/urlencoded.rs` → new
- `schematic/gen/src/codegen/request_structs/paginated.rs` → new
- `schematic/gen/src/codegen/request_structs/shared.rs` → new (common helpers)

**Steps:**
1. Analyze the match arms / code paths in `request_structs.rs` to identify per-shape generators.
2. Extract each shape into its own file.
3. `mod.rs` contains the public function (`generate_request_struct_with_options` or equivalent) that dispatches to the appropriate shape handler.
4. Update `codegen/mod.rs` to declare `pub mod request_structs;` (unchanged since directory module).
5. Run `cargo test -p schematic-gen`.

### 2.4 Split `postman_output.rs` (1 951 LOC) + unify with `openapi_output.rs` `[urgent]`

**Files:**
- `schematic/gen/src/postman_output.rs` → delete or reduce to thin shim
- `schematic/gen/src/openapi_output.rs` → refactor
- `schematic/gen/src/export/postman.rs` → expand (per-resource emitters)
- `schematic/gen/src/export/openapi.rs` → expand (per-resource emitters)
- `schematic/gen/src/export/mod.rs` → add `ExportFormat` trait + dispatch
- `schematic/gen/src/export/body.rs` → expand
- `schematic/gen/src/export/auth.rs` → expand
- `schematic/gen/src/export/path_params.rs` → expand
- `schematic/gen/src/lib.rs` → update re-exports if needed

**Steps:**
1. Define `ExportFormat` trait in `export/mod.rs`:
   ```rust
   pub trait ExportFormat {
       fn render_collection(api: &RestApi) -> Result<String, GeneratorError>;
       fn extension() -> &'static str;
   }
   ```
2. Move Postman collection assembly into `export/postman.rs` implementing `ExportFormat`.
3. Move OpenAPI assembly into `export/openapi.rs` implementing `ExportFormat`.
4. Move shared emitters (request body, auth block, path params) into the existing small `export/` files (`body.rs`, `auth.rs`, `path_params.rs`).
5. Keep backward-compatible re-exports in `lib.rs`: `pub use postman_output::{write_postman, write_postman_grouped};` → shim that delegates to `export::postman`.
6. Update `main.rs` imports.
7. Run `cargo test -p schematic-gen`.

**Test gates:**
- `cargo test -p schematic-gen --test postman_golden`
- `cargo test -p schematic-gen --test postman_artifact_validation`
- `cargo test -p schematic-gen --test postman_var_consistency`
- `cargo test -p schematic-gen --test postman_schema`

---

## Phase 3: schematic-gen Secondary Refactoring

**Review items addressed:** 4 (2 important, 1 important/cross-crate, 1 nice-to-have)

Depends on Phase 2 being complete. Further reduces `complex_fn_count` and Gini inequality.

### 3.1 Promote `codegen/api_struct.rs` (1 381 LOC) to sub-module `[important]`

**Files:**
- `schematic/gen/src/codegen/api_struct.rs` → delete
- `schematic/gen/src/codegen/api_struct/` → new directory
- `schematic/gen/src/codegen/api_struct/mod.rs` → new (entry point `generate_api_struct`)
- Per-emitter files as needed (e.g. `struct_def.rs`, `impl_blocks.rs`, `helpers.rs`)

**Steps:**
1. Analyze `api_struct.rs` to identify natural split points (struct definition generation vs impl blocks vs helpers).
2. Split into sub-files under `api_struct/`.
3. `mod.rs` re-exports the public function `generate_api_struct`.
4. Update `codegen/mod.rs` if needed (directory module auto-detected).
5. Run `cargo test -p schematic-gen`.

### 3.2 Promote `codegen/client.rs` (1 320 LOC) to sub-module `[important]`

**Files:**
- `schematic/gen/src/codegen/client.rs` → delete
- `schematic/gen/src/codegen/client/` → new directory
- `schematic/gen/src/codegen/client/mod.rs` → new
- Per-emitter files (e.g. `methods.rs`, `builders.rs`, `variants.rs`)

**Steps:**
1. Analyze `client.rs` to identify natural split points.
2. Split into sub-files under `client/`.
3. `mod.rs` re-exports the public function `generate_client_impl` (or equivalent).
4. Run `cargo test -p schematic-gen`.

### 3.3 Wire or delete `SchematicSchemaExtension` `[important]`

**Files:**
- `schematic/define/src/openapi/extensions.rs` → modify
- `schematic/gen/src/openapi_output.rs` (or `export/openapi.rs` after Phase 2) → add emission if wiring

**Steps:**
1. Determine if `SchematicSchemaExtension` has a consumer in the generator.
2. **Option A (preferred):** Wire it into the OpenAPI exporter so it emits the schema-level extension in generated specs.
3. **Option B:** Delete the unused struct and supporting types; add a `TODO` comment for when it's needed.
4. Track decision in `schematic/docs/io/openapi-extensions.md`.
5. Run `cargo test -p schematic-define --features openapi`.

### 3.4 Document the import_pipeline → output data flow `[nice-to-have]`

**Files:**
- `schematic/gen/src/lib.rs` → add ASCII pipeline diagram to module doc-comment

**Steps:**
1. Add to the `//!` block in `lib.rs`:
   ```text
   //! ## Pipeline
   //!
   //! ```text
   //! import_pipeline → parser → inference → model_gen → codegen/* → output → disk
   //! ```
   ```
2. Documentation only — no code change.

---

## Phase 4: schematic-definitions Provider Refactoring

**Review items addressed:** 4 (1 urgent, 2 important, 1 nice-to-have)

Independent of Phases 2–3. Can run in parallel with Phase 2 if desired. Run `cargo test -p schematic-definitions` after each sub-step.

### 4.1 Split large provider `types.rs` files `[urgent]`

**Target files (in priority order):**
| Provider | LOC | Priority |
|----------|-----|----------|
| `huggingface/types.rs` | 2 587 | P0 |
| `elevenlabs/types.rs` | 2 035 | P0 |
| `gitlab/types.rs` | 1 411 | P1 |
| `bitbucket/types.rs` | 1 365 | P1 |
| `github/types.rs` | 1 151 | P2 |
| `ollama/types.rs` | 1 149 | P2 |
| `anthropic/types.rs` | 1 069 | P2 |
| `emqx/types.rs` | 1 012 | P3 |

**Steps for each provider:**
1. Create `definitions/src/<provider>/types/` directory.
2. Split by API resource (e.g., for HuggingFace: `models.rs`, `datasets.rs`, `repos.rs`, `spaces.rs`, `shared.rs`).
3. Write `types/mod.rs` with `pub use` for all types — preserving the `types::*` path.
4. The provider `mod.rs` already does `pub mod types;` — no change needed.
5. Run `cargo test -p schematic-definitions` after each provider split.
6. Run `cargo test -p schematic-gen --test e2e_generation` to verify generated output unchanged.

### 4.2 Make `registry.rs` (1 216 LOC) data-driven `[important]`

**Files:**
- `schematic/definitions/src/registry.rs` → refactor
- Each `definitions/src/<provider>/mod.rs` → add registration call

**Steps:**
1. Analyze current enumeration pattern in `registry.rs` (likely a match on provider name).
2. Replace explicit enumeration with either:
   - **Option A:** `inventory::submit!` — each provider self-registers at compile time.
   - **Option B:** A `phf::Map` or `HashMap` built from a `register()` call in each provider's `mod.rs`.
3. Add `register()` to each provider sub-module.
4. `registry.rs` becomes a thin lookup facade.
5. Run `cargo test -p schematic-definitions`.

**Dependency:** May need `inventory` or `phf` added to `definitions/Cargo.toml`.

### 4.3 Lift duplicated provider scaffolding `[important]`

**Files:**
- All `definitions/src/<provider>/mod.rs`
- `schematic/define/src/` (potential new builder helper)

**Steps:**
1. Survey the `define_api()` function shape across all 13 providers.
2. Identify the common pattern (typically: `RestApi { name, description, base_url, ... }` with inline endpoint declarations).
3. Add a builder to `schematic-define` (e.g., `RestApi::builder("Name").base_url("...").auth(...).build()`).
4. Migrate 2–3 providers as a proof of concept.
5. Run `cargo test -p schematic-definitions`.
6. Migrate remaining providers in a follow-up.

### 4.4 Add `lib.rs` re-export policy doc-comment `[nice-to-have]`

**Files:**
- `schematic/definitions/src/lib.rs` → add `//!` policy block

**Steps:**
1. Add module-level doc-comment:
   ```rust
   //! # Re-export Policy
   //!
   //! Only types backed by an OpenAPI or Postman source appear at the crate root.
   //! Provider-internal helpers remain behind their `<provider>::` path.
   ```
2. Documentation only.

---

## Phase 5: Generator Output Improvements (schema crate)

**Review items addressed:** 3 (1 urgent, 2 important)

Depends on Phase 2 (output.rs pipeline) being complete. Changes are in the generator — the schema crate is regenerated after each change. Verify with `cargo check --manifest-path schematic/schema/Cargo.toml`.

### 5.1 Generator emit per-resource sub-modules `[urgent]`

**Files:**
- `schematic/gen/src/output/assemble.rs` (from Phase 2) → modify
- `schematic/gen/src/codegen/client/` (from Phase 3) → modify
- `schematic/schema/src/<api>/` → regenerated output

**Steps:**
1. Modify the output assembly to emit per-API directory modules instead of single files:
   ```text
   schema/src/emqx/
   ├── mod.rs            # client struct + method dispatch + re-exports
   ├── requests.rs       # request structs
   ├── responses.rs      # response types
   └── auth.rs           # variant() / variant_with() builders
   ```
2. `mod.rs` re-exports everything currently re-exported from the flat file.
3. `schema/src/lib.rs` already has `pub mod emqx;` — works with both file and directory.
4. Regenerate: `cargo run -p schematic-gen -- generate --api all --output schematic/schema/src`.
5. Verify: `cargo check --manifest-path schematic/schema/Cargo.toml`.
6. Run full test suite: `cargo test -p schematic-gen --test e2e_generation`.

### 5.2 Group sibling modules by transport `[important]`

**Files:**
- `schematic/gen/src/output/assemble.rs` → modify `lib.rs` emission
- `schematic/schema/src/lib.rs` → regenerated

**Steps:**
1. Modify the generator's `lib.rs` emitter to produce:
   ```rust
   pub mod rest {
       pub mod anthropic;
       pub mod openai;
       // ...
   }
   pub mod ws {
       pub mod elevenlabs_ws;
       pub mod unfolded_circle_core_ws;
       // ...
   }
   // backward-compat re-exports:
   pub use rest::*;
   pub use ws::*;
   ```
2. Regenerate and verify: `cargo check --manifest-path schematic/schema/Cargo.toml`.
3. Verify downstream consumers in the monorepo still compile.

### 5.3 Factor cross-API patterns into `shared.rs` / `ws_shared.rs` `[important]`

**Files:**
- `schematic/gen/src/codegen/api_struct/` → modify to emit trait impls instead of inlined patterns
- `schematic/gen/src/codegen/request_structs/` → modify similarly
- `schematic/schema/src/shared.rs` → regenerated with new traits

**Steps:**
1. Define shared traits in the generator's output for `shared.rs`:
   ```rust
   pub trait DocsUrl { const DOCS_URL: &'static str; }
   pub trait Paginated { type Cursor; fn cursor(&self) -> Option<Self::Cursor>; }
   ```
2. Modify the codegen to emit `impl DocsUrl for OpenAI { ... }` instead of inlining `DOCS_URL` constants.
3. Similarly for pagination iterators and `From<&str>` impls.
4. Regenerate and verify: `cargo check --manifest-path schematic/schema/Cargo.toml`.

---

## Phase 6: schematic-oauth + Remaining Cleanup

**Review items addressed:** 3 (1 important, 2 nice-to-have)

Independent of all other phases. Can run at any time.

### 6.1 Hide `oauth2` crate behind internal module `[important]`

**Files:**
- `schematic/oauth/src/external.rs` → new
- `schematic/oauth/src/manager.rs` → update imports
- `schematic/oauth/src/types.rs` → update imports if needed
- `schematic/oauth/src/lib.rs` → add `mod external;` (private)

**Steps:**
1. Create `external.rs` that owns every `use oauth2::*` import.
2. Re-publish only the types that `manager.rs` and `types.rs` need.
3. Update `manager.rs` and `types.rs` to import from `crate::external` instead of `oauth2` directly.
4. Run `cargo test -p schematic-oauth`.

### 6.2 Move `OAuth2RuntimeConfig` validation into constructor `[nice-to-have]`

**Files:**
- `schematic/oauth/src/types.rs` → refactor struct to private fields + `new()` constructor

**Steps:**
1. Replace `pub struct OAuth2RuntimeConfig { pub fields }` with private fields.
2. Add `OAuth2RuntimeConfig::new(...) -> Result<Self, OAuthError>` that validates URL well-formedness and scope non-empty.
3. Update callers in `manager.rs` and any tests.
4. Run `cargo test -p schematic-oauth`.

### 6.3 Add `#[allow(clippy::too_many_lines)]` at module scope in generated code `[nice-to-have]`

**Files:**
- `schematic/gen/src/codegen/*` → modify emission

**Steps:**
1. After Phases 2–3 reduce function sizes, audit remaining clippy suppressions in generated output.
2. Move any per-function `#[allow(clippy::too_many_lines)]` to module-level in the generated `mod.rs`.
3. Or remove them entirely if functions are now short enough.
4. Regenerate and verify: `cargo check --manifest-path schematic/schema/Cargo.toml`.

---

## Verification Checklist

After all phases:

1. `cargo test -p schematic-define -p schematic-definitions -p schematic-gen -p schematic-oauth`
2. `cargo test -p schematic-gen --test e2e_generation`
3. `cargo test -p schematic-gen --test artifact_drift`
4. `cargo check --manifest-path schematic/schema/Cargo.toml`
5. `cargo clippy --workspace --all-targets --color=never`
6. `just -f schematic/justfile generate` (regenerate all schemas)
7. `cargo test -p schematic-gen --test postman_golden`
8. Re-run `sentrux scan` — target `complex_fn_count < 25` and improved Newman modularity

## Summary Table

| Phase | Crates | Items | Priority Range | Dependency |
|-------|--------|-------|----------------|------------|
| 1 | define | 5 | urgent → nice-to-have | None |
| 2 | gen | 4 | critical → urgent | None (parallel w/ P1) |
| 3 | gen | 4 | important → nice-to-have | Phase 2 |
| 4 | definitions | 4 | urgent → nice-to-have | None (parallel w/ P2) |
| 5 | gen → schema | 3 | urgent → important | Phase 2 |
| 6 | oauth + gen | 3 | important → nice-to-have | None |
| **Total** | | **23** | | |
