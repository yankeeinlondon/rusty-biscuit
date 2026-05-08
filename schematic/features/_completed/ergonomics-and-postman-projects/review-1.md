---
ready: "no"
agent: ""
model: ""
---

# Implementation Review: Ergonomics and Postman Collections

## Summary

The feature successfully introduces Postman collection generation and a normalization layer for artifact export. However, it fails to deliver the designed **"grouped" OpenAPI export**, which is a critical requirement for APIs sharing modules (e.g., Ollama, EMQX). Additionally, the implementation of test rigor falls short of the **Level 2** requirement for CLI-facing output.

## Findings

### 1. OpenAPI Grouping Regression (Severity: High)

**Requirement:** Shared-module handling: "We should emit one OpenAPI document per generated module... The document should contain the union of operations for all `RestApi` definitions that resolve to that module." (Tech Design)

**Implementation Gap:** `schematic-gen` currently exports OpenAPI documents individually for every `RestApi` using `api.name.to_lowercase()`.

- **File:** `schematic/gen/src/main.rs` (lines 538-593)
- **Impact:** APIs like `OllamaNative` and `OllamaOpenAI` produce separate files (`ollamanative.json`, `ollamaopenai.json`) instead of a single, unified `ollama.json`. This breaks the design goal of aligning OpenAPI artifacts with the generated Rust module layout.

### 2. OpenAPI Filename Mismatch (Severity: Medium)

**Requirement:** "File naming: `<module>.json` or `<module>.yaml`. The file should be named from the resolved module path, not directly from `api.name`" (Tech Design)

**Implementation Gap:** `write_openapi` in `openapi_output.rs` and the loop in `main.rs` use `api.name.to_lowercase()` as the base for the filename.

- **Impact:** Inconsistent naming between generated Rust modules (which use `module_path`) and OpenAPI specs.

### 3. Lack of Level 2 Test Rigor (Severity: High)

**Requirement:** "A feature MAY be marked production-ready only when each user-observable requirement has at minimum the level of verification appropriate for it." (Review Mandate)

**Finding:** The strongest verification for CLI output is **Level 1**.

- **User-Observable Behavior:** Colored validation results, generation progress reporting, and error diagnostics in the `schematic-gen` CLI.
- **Verification Present:** Unit tests and in-process PTY-like tests in `artifact_drift.rs` and `e2e_generation.rs`.
- **Gap:** No Level 2 tests (real terminal capture via `tmux` or `wezterm cli`) verify that these logs and reports render correctly through an actual terminal emulator's SGR styling and layout engine.

### 4. Strict Registry Enforcement (Severity: Low)

**Finding:** `main.rs` enforces a hard failure if a registry is missing for an API during `generate --api all`.

- **Tech Design Reference:** "Migration behavior can remain warn-and-skip until registry coverage is complete."
- **Observation:** While most registries are currently implemented, this strictness deviates from the designed migration path. It is acceptable given the current high coverage but should be noted.

## Positive Observations

- **Postman Implementation:** The Postman export is robust, correctly implements grouping via `write_postman_grouped`, and handles foldering and auth mapping exactly as specified.
- **Normalization Layer:** The introduction of `schematic/gen/src/export/` provides a clean, reusable abstraction for different export formats.
- **Registry Coverage:** The team exceeded the initial goal by providing schema registries for nearly every REST API in the monorepo.
- **Artifact Drift Detection:** The `artifact_drift.rs` test suite is a high-quality addition for maintaining artifact integrity over time.

## Recommended Changes

1. **Refactor OpenAPI Export for Grouping:**
    - Add `write_openapi_grouped` to `openapi_output.rs` (similar to `write_postman_grouped`).
    - Update `run_generate_all` in `main.rs` to group APIs using `apis_by_module()` for OpenAPI export, ensuring filenames use the module name and contents are merged.
2. **Implement Level 2 Verification:**
    - Add a test case that spawns `schematic-gen validate --api openai` inside a `tmux` session and captures the pane output to verify the coloring and formatting of the validation report.
3. **Align OpenAPI Filenames:**
    - Ensure all OpenAPI filenames are derived from `schematic_gen::export::resolve_module_name(api)`.

## Closure

This feature provides excellent Postman support and a strong foundation for artifact export, but the failure to implement OpenAPI grouping and the gap in test rigor prevent it from being marked as production-ready.
