---
phases: 5
created: 2026-05-04
start_phase: 5
source_files_during_phase_1:
  - unchained-ai/lib/src/models/model_pricing.rs
  - unchained-ai/lib/src/models/model_default_parameters.rs
  - unchained-ai/lib/src/models/model_metadata.rs
  - unchained-ai/lib/src/models/mod.rs
  - unchained-ai/lib/src/rigging/providers/models/mod.rs
  - unchained-ai/lib/src/rigging/providers/models/metadata_generated.rs
  - unchained-ai/cli/src/commands/models.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - unchained-ai
  - unchained-ai-cli
source_files_during_phase_2:
  - unchained-ai/lib/src/api/openai_api.rs
  - unchained-ai/gen/src/main.rs
  - unchained-ai/lib/src/rigging/providers/models/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - unchained-ai/gen/src/provider_metadata/openrouter.rs
  - unchained-ai/gen/src/provider_metadata/mod.rs
  - unchained-ai/gen/src/main.rs
  - unchained-ai/lib/src/models/model_pricing.rs
  - unchained-ai/lib/src/rigging/providers/models/mod.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - unchained-ai/gen/src/metadata_generator.rs
  - unchained-ai/gen/src/main.rs
  - unchained-ai/gen/src/provider_metadata/openrouter.rs
  - unchained-ai/gen/src/provider_metadata/mod.rs
  - unchained-ai/lib/src/rigging/providers/models/mod.rs
  - unchained-ai/lib/src/rigging/providers/models/metadata_openrouter_generated.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/unchained-ai/SKILL.md
source_files_during_phase_5:
  - unchained-ai/cli/src/commands/models.rs
docs_updated_during_phase_5:
  - unchained-ai/README.md
  - unchained-ai/cli/README.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .opencode/skill/unchained-ai/SKILL.md
  - .opencode/skill/unchained-ai/providers-and-models.md
  - .opencode/skill/unchained-ai/model-generator.md
  - .claude/skills/unchained-ai/SKILL.md
  - .claude/skills/unchained-ai/providers-and-models.md
  - .claude/skills/unchained-ai/model-generator.md
---

# Execution Plan - Better Model Metadata

This plan outlines the implementation of rich provider-native metadata capture and merging for the `unchained-ai` package, as specified in the [Functional Specification](./spec.md) and [Technical Design](./tech-design.md).

## Summary

The goal is to stop discarding rich model metadata (pricing, architecture, parameters) from provider APIs (specifically OpenRouter) and merge it with Parsera data to provide a more complete model specification in the generated Rust code.

---

## Phase 1: Core Data Models (unchained-ai-lib)
**Goal**: Define the new metadata structures and refactor the existing metadata container.

- [x] **Step 1.1: Create `ModelPricing` struct**
    - Create `unchained-ai/lib/src/models/model_pricing.rs`.
    - Implement `ModelPricing` with `Option<f64>` fields for `prompt_per_token`, `completion_per_token`, `web_search_per_request`, and `input_cache_read_per_token`.
    - Add custom `serde` deserializer to handle string-to-f64 conversion from OpenRouter JSON.
- [x] **Step 1.2: Create `ModelDefaultParameters` struct**
    - Create `unchained-ai/lib/src/models/model_default_parameters.rs`.
    - Implement `ModelDefaultParameters` with fields for temperature, top_p, top_k, etc.
- [x] **Step 1.3: Update module exports**
    - Update `unchained-ai/lib/src/models/mod.rs` to export the new modules.
- [x] **Step 1.4: Refactor `ModelMetadata` to `ProviderModelMetadata`**
    - Modify `unchained-ai/lib/src/models/model_metadata.rs`.
    - Rename `ModelMetadata` to `ProviderModelMetadata`.
    - Add new fields: `description`, `pricing`, `supported_parameters`, `default_parameters`, `knowledge_cutoff`, `created`.
    - Add `#[deprecated]` type alias: `pub type ModelMetadata = ProviderModelMetadata;`.
    - Implement `Default` for `ProviderModelMetadata`.

**Validation**: `cargo check -p unchained-ai` passes.

---

## Phase 2: Preserving Raw Provider Data (unchained-ai-lib)
**Goal**: Update the API layer to preserve full JSON responses from provider `/v1/models` endpoints.

- [x] **Step 2.1: Update `openai_api.rs` types**
    - Modify `unchained-ai/lib/src/api/openai_api.rs`.
    - Define `ProviderModelsResponse` and `ProviderModelEntry` to capture `serde_json::Value` for each model.
- [x] **Step 2.2: Update `get_provider_models_from_api`**
    - Change return type to `Result<Vec<ProviderModelEntry>, ...>`.
    - Update implementation to preserve the raw metadata object.
- [x] **Step 2.3: Update internal library consumers**
    - Ensure any internal callers of the updated API function are fixed.

**Validation**: `cargo test -p unchained-ai` (specifically `api::openai_api` tests).

---

## Phase 3: Provider-Specific Metadata Parsing (unchained-ai-gen)
**Goal**: Implement the logic to parse and route provider-native metadata.

- [x] **Step 3.1: Create OpenRouter parser**
    - Create `unchained-ai/gen/src/provider_metadata/openrouter.rs`.
    - Implement `parse_openrouter_model(&serde_json::Value) -> ProviderModelMetadata`.
- [x] **Step 3.2: Create metadata dispatcher**
    - Create `unchained-ai/gen/src/provider_metadata/mod.rs`.
    - Implement a dispatcher that routes to the OpenRouter parser based on `Provider` type.
- [x] **Step 3.3: Update generator `main.rs`**
    - Update `ProviderResult` struct to include a `HashMap<String, serde_json::Value>` for raw metadata.
    - Update the fetch loop to store raw metadata alongside model IDs.

**Validation**: Unit tests for `parse_openrouter_model` using a JSON fixture.

---

## Phase 4: Metadata Merging and Code Generation (unchained-ai-gen)
**Goal**: Merge data sources and emit the expanded Rust files.

- [x] **Step 4.1: Implement `merge_metadata` logic**
    - Update `unchained-ai/gen/src/metadata_generator.rs`.
    - Implement the merging logic with priority: Provider-Native > Parsera.
- [x] **Step 4.2: Expand compact metadata generation**
    - Update `generate_entry` to emit the new `ProviderModelMetadata` fields (using `..Default::default()` for brevity).
- [x] **Step 4.3: Implement rich OpenRouter metadata generation**
    - Add `generate_openrouter_entry` to `metadata_generator.rs`.
    - Create the structure for `metadata_openrouter_generated.rs` (LazyLock HashMap).
- [x] **Step 4.4: Wire up dual file output**
    - Update `unchained-ai/gen/src/main.rs` to write both `metadata_generated.rs` and `metadata_openrouter_generated.rs`.

**Validation**: Run `just generate-models` and verify both files are produced and compile.

---

## Phase 5: Verification and Documentation
**Goal**: Finalize the feature and update project documentation.

- [x] **Step 5.1: Full regeneration and smoke test**
    - Run `just generate-models` for all providers.
    - Verify that `unchained-ai` CLI (`models` command) still works and ideally displays some of the new metadata if updated.
- [x] **Step 5.2: Update documentation**
    - Update `.opencode/skill/unchained-ai/*.md` if applicable.
    - Update `README.md` if public API changes are notable.

**Validation**: `just lint` and `just test` pass for the entire package.
