---
ready: false
agent: gemini
---

# Feature Review: Agent Selection (Review 1)

This review covers the implementation of the Agent Selection feature for `claudine compose`, `inline-compose`, and `sequence`. While the core resolution logic in `claudine-lib` is well-implemented and follows the resolution chains defined in the specification, there are several critical bugs and functional gaps in the CLI wiring and interactive UI components that prevent this feature from being production-ready.

## Gaps in Functionality

### 1. Sequence Review Model Column
The `Model` column in the sequence review table (`claudine/cli/src/commands/wrap/selection_ui.rs`) is implemented as a simple `TextInput`. The design specifically required this to be a `ChooseOne` component when a model catalog is available for the resolved provider, falling back to `TextInput` only when no catalog exists. Currently, users are forced to type model names manually even for providers like Claude and Codex which have static catalogs.

### 2. Sequence Review Column Locking
The design required that explicit CLI flags (e.g., `--claude` or `--model gpt-5`) should "lock" the corresponding columns in the sequence review screen, rendering them as `StaticText` rather than editable widgets. The current implementation in `selection_ui.rs` does not implement this locking logic; it always presents editable columns, which could lead to users attempting to change a provider that was explicitly locked via a CLI flag.

### 3. Missing Model Catalog Overrides
The `ModelCatalogService` is initialized in both `composition.rs` and `sequence.rs` using `ModelCatalogService::new()`. This constructor does not load user-configured model overrides from the Claudine config. As a result, the "User override" feature for model enumeration (allowing users to add or replace models in the catalog via `config.json`) is non-functional in the current CLI implementation.

### 4. Missing Catalog Refresh
The CLI never triggers a `refresh()` or `refresh_all()` on the `ModelCatalogService`. This means dynamic sources (like shelling out to `opencode models`) are never utilized unless the user has manually populated the cache files. The service should likely perform a background refresh or a lazy refresh when a catalog-aware resolution is requested.

## Broken or Incomplete Features

### 1. CRITICAL: Sequence Review Provider Selection
In `claudine/cli/src/commands/wrap/selection_ui.rs`, the `review_sequence` function incorrectly uses `row.get_text("provider")` to read back the user's choice from a `ChooseOne` cell. Because `tui-chrome` returns a `CellValue::ChosenOne` for such cells, `get_text` always returns `None`. This causes the provider to always default to `Provider::Claude` (via the `unwrap_or(Provider::Claude)` fallback) regardless of what the user selected in the UI.

### 2. Sequence Review Decoder Robustness
The decoder in `review_sequence` uses `Provider::fuzzy_match_cli_name` on a slug that is guaranteed to be valid (since it comes from the picker's own option IDs). Using a fuzzy matcher here is unnecessary, and the default fallback to `Provider::Claude` is dangerous. It should ideally use an exact match or handle the `None` case as a hard error/abort.

## Test Coverage

### 1. Interactive UI Testing
There are no automated tests for the logic in `claudine/cli/src/commands/wrap/selection_ui.rs`. While testing TUI components is difficult, the data transformation logic (mapping drafts to table rows and decoding table rows back to execution targets) is pure and should be covered by unit tests to prevent regressions like the `get_text` bug identified above.

### 2. Model Catalog Integration
There are no integration tests verifying the end-to-end propagation of model overrides from `ClaudineConfig` through to the resolution logic. The library has unit tests for `merge_overrides`, but the CLI's failure to initialize the service with these overrides remains untested.

## Ergonomics and Performance

### 1. Redundant Provider Enumeration
In `claudine/cli/src/commands/wrap/sequence.rs`, a hardcoded list of providers is used to check for installation. This list is redundant with `claudine::events::PROVIDERS_DISPLAY_ORDER` and will likely drift as new providers are added. It should be refactored to use the canonical display order list.

### 2. Unified Config Loading
`load_config_favorite` in `composition.rs` loads the entire Claudine config just to extract the `preferred_agent` field. Since the resolution logic now also requires the `models` override map, this should be refactored into a `load_selection_config` helper that returns all selection-related settings in one pass.

## Conclusion

The core library logic for agent and model resolution is solid and well-tested. However, the interactive sequence review screen is currently broken (always resolving to Claude) and several key design requirements (model pickers, column locking, and config overrides) are missing from the CLI implementation.

**Status: Not Ready**
