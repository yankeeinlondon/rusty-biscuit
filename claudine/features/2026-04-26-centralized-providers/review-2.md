---
agent: ""
model: ""
ready: false
---

# Feature Review: Centralized Providers

This review covers the implementation of the centralized provider model in the `claudine` package, as defined in the 2026-04-26-centralized-providers feature specification and design documents.

## Critical Findings

### 1. Broken Implementation (Does Not Compile)
The implementation in `claudine/lib/src/provider/methods.rs` contains a critical type mismatch that prevents the library from compiling.
- **Location:** `claudine/lib/src/provider/methods.rs:145`
- **Issue:** The code uses `if let Some(_event) = provider_info(provider).adapter.detect(raw)` but `AdapterBehavior::detect` returns a `bool`, not an `Option`.
- **Impact:** The library is unusable in its current state as it cannot be built.

### 2. Incomplete Migration (Missed Dispatch Points)
The primary goal of this feature was to eliminate scattered `match provider` blocks in the library. Several significant dispatch points were missed:
- **`permissions::query::is_cli_sensitive`**: Still uses a manual `match provider` block instead of consulting `provider_info(provider).cli_sensitive_axes`.
- **`composition::select::provider_env_vars`**: Still uses a manual `match provider` block instead of consulting `provider_info(provider).model_env_vars`.
- **`model_catalog::provider_sources`**: Both `static_catalog_for_provider` and `fetch_provider_catalog` still use `match provider` instead of using the new `static_models` and `dynamic_source` fields in `ProviderInfo`.

### 3. Incomplete Wrapper Profile Thinning
Phase 6 of the migration intended to thin out the `WrapperProfile` implementations in the CLI by using default implementations that consume the centralized catalog.
- **Observation:** `ClaudeWrapper`, `KimiWrapper`, and others still have hardcoded `--yolo` flags and other logic in their `apply_yolo`, `has_supported_yolo`, and `reject_direct_yolo` methods.
- **Suggestion:** Move these into default implementations on the `WrapperProfile` trait that consume `provider_info(self.provider()).yolo`.

## Functional Gaps

### 1. Simplified EventSupportLevel
The implementation of `EventSupportLevel` is significantly simpler than what was proposed in the design.
- **Design:** Proposed data-carrying variants like `StreamParse { protocol, native_name }` and `WireProxy { mode, native_name }`.
- **Implementation:** Uses unit variants `Hook`, `NonHook`, `Acp`, `NotSupported`.
- **Impact:** While functional, it loses the "strongly typed surface" advantage for identifying the specific capture mechanism used by non-hook events.

### 2. AdapterBehavior Discrepancy
The `AdapterBehavior` trait implementation in `behavior.rs` lacks the `parse_payload` method proposed in the spec, which was intended to return `Option<AgenticEvent>`. Instead, it only provides `detect` and `provider_adapter`.

## Test Coverage

- **Exhaustiveness Tests:** While the registry exhaustiveness tests are well-written, they failed to catch the compile error in `detect_from_payload`, indicating that these tests are either not being run as part of the validation suite or they don't exercise the broken code path.
- **Integration Tests:** Stronger integration tests are needed for the `detect_from_payload` logic to ensure that the table-driven detection actually works for all providers.

## Ergonomics and Performance

- **Registry Implementation:** The design suggested using an array-backed `OnceLock` for O(1) lookup in `provider_info`. The current implementation uses a `match` block. While safe and exhaustive, it deviates from the architectural preference stated in the design.
- **Model Catalog Sourcing:** `static_catalog_for_provider` should be refactored to return a slice from `ProviderInfo` instead of creating a new `Vec<String>` via a match block.

## Conclusion

**Ready for Production: NO**

The feature is not ready for production due to the critical compile error and the significant number of incomplete migration points. While the overall architecture is sound and the bulk of the work (including `AgentId` removal) is complete, the remaining gaps undermine the primary goal of the refactor: creating a single, authoritative, and drift-free provider system.
