---
phases: 6
created: 2026-05-07
start_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - schematic/definitions/src/artificial_analysis/mod.rs
  - schematic/definitions/src/artificial_analysis/types.rs
  - schematic/definitions/src/lib.rs
  - schematic/definitions/src/registry.rs
  - schematic/definitions/src/prelude.rs
docs_updated_during_phase_2:
  - schematic/definitions/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - schematic/gen/src/codegen/api_struct/mod.rs
  - schematic/gen/src/pipeline.rs
  - schematic/schema/src/artificial_analysis.rs
  - schematic/schema/src/lib.rs
  - schematic/schema/src/prelude.rs
  - schematic/openapi/artificial_analysis.json
  - schematic/postman/artificial_analysis.postman_collection.json
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages:
  - schematic-definitions
  - schematic-gen
  - schematic-schema
---

# Artificial Analysis API Definition Execution Plan

## Phase 1: Baseline Orientation

1. Confirm the current workspace state.
   - Run `git status --short`.
   - Note any existing user changes in files this work may touch.

2. Inspect the existing provider patterns that this feature must mirror.
   - Read `schematic/definitions/src/emqx/` for two `RestApi`s sharing one module.
   - Read `schematic/definitions/src/ollama/` for shared `module_path` and request naming behavior.
   - Read `schematic/definitions/src/lmstudio/mod.rs` or `schematic/definitions/src/openai/mod.rs` for local test style.
   - Read `schematic/definitions/src/lib.rs`, `registry.rs`, and `prelude.rs` before editing wiring.

3. Confirm dependency state.
   - Check `schematic/definitions/Cargo.toml` for an existing direct `serde_json` dependency.
   - If absent, plan to add it because `types.rs` will expose `serde_json::Value` in public structs.

Validation checkpoint:
- The implementer can identify the exact files to edit and the matching local patterns for shared modules, registry lookup, prelude exports, and tests.

Parallelizable:
- Steps 2 and 3 can be performed in parallel because they read disjoint files.

## Phase 2: Add the Artificial Analysis Definition Module

1. Create `schematic/definitions/src/artificial_analysis/mod.rs`.
   - Add the module-level `//!` docs from the spec.
   - Include the attribution requirement to `https://artificialanalysis.ai/`.
   - Declare `mod types;` and `pub use types::*;`.
   - Import `SchemaRegistry`, `schematic_define` primitives, and `params::{EndpointParams, QueryParamType}`.

2. Implement `openapi_registry()`.
   - Register all shared, LLM, media, and CritPt request/response types.
   - Do not register `RateLimitError`.

3. Implement `define_artificial_analysis_data_api()`.
   - Name: `ArtificialAnalysisData`.
   - Base URL: `https://artificialanalysis.ai/api/v2`.
   - Docs URL: `https://artificialanalysis.ai/api-reference`.
   - Auth: `AuthStrategy::ApiKey { header: "x-api-key" }`.
   - Env auth and env mapping: `ARTIFICIAL_ANALYSIS_API_KEY`.
   - `module_path`: `Some("artificial_analysis")`.
   - Add the six `/data/...` GET endpoints exactly as specified.
   - Add `include_categories` query params only to text-to-image, text-to-video, and image-to-video endpoints.

4. Implement `define_artificial_analysis_critpt_api()`.
   - Name: `ArtificialAnalysisCritPt`.
   - Use the same base URL, docs URL, auth, env auth, env mapping, and `module_path`.
   - Add the single `POST /critpt/evaluate` endpoint.
   - Use `CritPtEvaluateBody` as the JSON request body and `CritPtEvaluateResponse` as the JSON response.

5. Add the `#[cfg(test)] mod tests` block to `mod.rs`.
   - Cover metadata, auth, endpoint count, endpoint paths, query param placement, registry completeness, and `RateLimitError` omission.

Validation checkpoint:
- `schematic/definitions/src/artificial_analysis/mod.rs` exists and contains two public `define_*` functions, one shared registry function, and focused tests.

## Phase 3: Add Artificial Analysis Types

1. Create `schematic/definitions/src/artificial_analysis/types.rs`.
   - Import `schemars::JsonSchema`.
   - Import `serde::{Deserialize, Serialize}`.

2. Add shared types.
   - `ModelCreator`.
   - `CategoryBreakdown`.

3. Add LLM response types.
   - `LlmEvaluations`.
   - `LlmPricing`.
   - `LlmModel`.
   - `PromptOptions`.
   - `LlmModelsResponse`.

4. Add media response types.
   - `MediaModel`.
   - `MediaModelsResponse`.

5. Add CritPt request/response and documented error types.
   - `CritPtMessage`.
   - `CritPtSubmission`.
   - `CritPtEvaluateBody`, deriving `Default`.
   - `CritPtEvaluateResponse`.
   - `RateLimitError`, with `#[serde(rename = "retryAfter")]` on `retry_after`.

6. Preserve serialization behavior.
   - Apply `#[serde(skip_serializing_if = "Option::is_none")]` to optional fields from the spec.
   - Apply `#[serde(skip_serializing_if = "serde_json::Value::is_null", default)]` to `CritPtEvaluateBody::batch_metadata`.

7. Add rustdoc comments to every public struct and public field.
   - Keep comments concise and follow the local rustdoc convention.
   - Do not add `# H1` headings inside `///` blocks.

Validation checkpoint:
- Every public type derives `Debug, Clone, Serialize, Deserialize, JsonSchema`.
- `CritPtEvaluateBody` additionally derives `Default`.
- `serde_json::Value` appears only for `generation_config` and `batch_metadata`.

Parallelizable:
- Phase 2 and Phase 3 can be drafted in parallel if writers coordinate on type names and exports. Final compilation depends on both being complete.

## Phase 4: Wire the Provider into schematic-definitions

1. Update `schematic/definitions/src/lib.rs`.
   - Add `pub mod artificial_analysis;`.
   - Re-export `define_artificial_analysis_data_api` and `define_artificial_analysis_critpt_api`.
   - Add both APIs to the `apis_by_module()` `all_apis` vector.
   - Add the crate-level doc example for the provider.

2. Update `schematic/definitions/src/registry.rs`.
   - Add `get_registry()` arms for `artificial-analysis-data` and `artificial-analysis-critpt`, both returning `crate::artificial_analysis::openapi_registry()`.
   - Add `registry_key_for()` mappings for `ArtificialAnalysisData` and `ArtificialAnalysisCritPt`.
   - Extend `registry_key_for_known_apis_matches_table` with both new pairs.

3. Update `schematic/definitions/src/prelude.rs`.
   - Re-export both Artificial Analysis definition functions.
   - Re-export `CritPtEvaluateBody`, `CritPtEvaluateResponse`, `LlmModelsResponse`, and `MediaModelsResponse`.
   - Check for name collisions before finalizing exports.

4. Update `schematic/definitions/README.md`.
   - Add an `Available APIs` table row for Artificial Analysis.
   - Include both API keys if the table distinguishes API names from modules.

5. Update `schematic/definitions/Cargo.toml` if needed.
   - Add direct `serde_json` dependency only if it is not already present.

Validation checkpoint:
- The provider is reachable from the crate root, prelude, registry lookup, and README.
- Both APIs group under the generated module name `artificial_analysis`.

Parallelizable:
- Steps 1, 2, 3, 4, and 5 can be edited independently after Phase 2 names are fixed. Merge carefully because `lib.rs` and `registry.rs` are high-traffic files.

## Phase 5: Generate and Compile the Schema Client

1. Generate the data API client.
   - Run:
     ```bash
     cargo run -p schematic-gen -- \
         --api artificial-analysis-data \
         --output schematic/schema/src
     ```

2. Generate the CritPt API client.
   - Run:
     ```bash
     cargo run -p schematic-gen -- \
         --api artificial-analysis-critpt \
         --output schematic/schema/src
     ```

3. Verify generated output location.
   - Confirm `schematic/schema/src/artificial_analysis.rs` exists, or inspect `schematic/schema/src/artificial_analysis/` if the generator emits a directory.

4. Check response method selection.
   - Run:
     ```bash
     grep -n "request_bytes\|request_text\|request_empty" \
         schematic/schema/src/artificial_analysis*.rs || true
     ```
   - Expected result: no binary, text, or empty request helpers for this provider.

5. Compile the excluded schema crate directly.
   - Run:
     ```bash
     cargo check --manifest-path schematic/schema/Cargo.toml
     ```

Validation checkpoint:
- Generated client code exists under `artificial_analysis`.
- The schema crate compiles without editing generated files manually.

Dependency note:
- Do not rely on workspace-wide commands for `schematic/schema`; it is excluded from the workspace and must use `--manifest-path schematic/schema/Cargo.toml`.

## Phase 6: Test, Document Drift, and Final Review

1. Run targeted definition tests.
   - Run:
     ```bash
     cargo test -p schematic-definitions artificial_analysis
     ```

2. Run targeted registry tests.
   - Run:
     ```bash
     cargo test -p schematic-definitions registry::tests::registry_key_for_known_apis_matches_table
     cargo test -p schematic-definitions registry::tests::get_registries_for_module
     ```

3. Run broader schematic validation if targeted tests pass.
   - Run:
     ```bash
     cargo test -p schematic-define -p schematic-definitions -p schematic-gen
     ```
   - If this is too slow or blocked by unrelated failures, capture the failure and keep the targeted green results as the minimum feature validation.

4. Check formatting and lint-relevant issues.
   - Run `cargo fmt`.
   - Run `cargo check -p schematic-definitions`.
   - If an area justfile has a relevant lint recipe, prefer the existing local command after targeted checks are green.

5. Review drift-sensitive docs.
   - Confirm `schematic/definitions/README.md` is updated.
   - Update dependency docs only if a new direct crate dependency was added.
   - No `.claude/skills/` or `AGENTS.md` update is expected unless implementation uncovers a new workflow or workspace convention.

6. Inspect final diff.
   - Run `git status --short`.
   - Run `git diff -- schematic/definitions schematic/schema`.
   - Confirm no unrelated user changes were reverted.
   - Confirm generated files were produced by the generator and not manually edited.

Final validation checkpoint:
- Targeted tests pass.
- `schematic/schema` compiles via direct manifest path.
- Artificial Analysis is wired into definitions, registry, prelude, docs, and generated schema output.
- Attribution appears in the module rustdoc.
- `RateLimitError` remains documentation-only and unregistered.
