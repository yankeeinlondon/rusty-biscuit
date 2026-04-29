# Agent Selection Review 1 — Implementation Plan

## Overview

This plan addresses all issues identified in `review-1.md` across three implementation phases. Each phase is independently completable, includes focused test coverage, and ends with lint validation for the `claudine` package areas.

**Total phases: 3**

---

## Phase 1: Critical Bug Fixes & Ergonomics

**Goal**: Fix the broken sequence review provider selection, eliminate dangerous fallbacks, and address ergonomics issues (redundant provider enumeration, unified config loading).

### 1.1 Fix Sequence Review Provider Selection (CRITICAL)

**File**: `claudine/cli/src/commands/wrap/selection_ui.rs`

**Problem**: `review_sequence` uses `row.get_text("provider")` on a `ChooseOne` cell. `get_text` only matches `CellValue::Text` and `CellValue::StaticText`, so it always returns `None` for `CellValue::ChosenOne`.

**Fix** (lines 135-138):
```rust
// BEFORE:
let provider_slug = row.get_text("provider").unwrap_or_default();
let provider = Provider::fuzzy_match_cli_name(provider_slug).unwrap_or(Provider::Claude);

// AFTER:
let provider = match row.get("provider") {
    Some(CellValue::ChosenOne(Some(slug))) => {
        Provider::parse_cli_name(slug)
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown provider slug in review: {slug}")
            ))?
    }
    Some(CellValue::StaticText(slug)) => {
        Provider::parse_cli_name(slug)
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown provider slug in review: {slug}")
            ))?
    }
    _ => {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "provider cell missing or has unexpected value type"
        ));
    }
};
```

### 1.2 Sequence Review Decoder Robustness

**File**: `claudine/cli/src/commands/wrap/selection_ui.rs`

**Problem**: The decoder uses `Provider::fuzzy_match_cli_name` on a slug that is guaranteed to be valid (it comes from the picker's own option IDs). Using a fuzzy matcher is unnecessary, and the fallback to `Provider::Claude` is dangerous.

**Fix**: Included in 1.1 above. Use `Provider::parse_cli_name` (exact match against canonical slugs/aliases) and return a hard error on `None` instead of silently defaulting to Claude.

### 1.3 Redundant Provider Enumeration

**File**: `claudine/cli/src/commands/wrap/sequence.rs`

**Problem**: Lines 196-208 use a hardcoded provider list that duplicates `PROVIDERS_DISPLAY_ORDER`.

**Fix**:
```rust
// BEFORE: hardcoded array
let installed: Vec<claudine::events::Provider> = [
    claudine::events::Provider::Claude,
    // ... 7 more variants
]
.into_iter()
.filter(|p| clients.path(p.sniff_ai_cli()).is_some())
.collect();

// AFTER: canonical display order
use claudine::events::PROVIDERS_DISPLAY_ORDER;
let installed: Vec<claudine::events::Provider> = PROVIDERS_DISPLAY_ORDER
    .into_iter()
    .filter(|p| clients.path(p.sniff_ai_cli()).is_some())
    .collect();
```

Do the same for `claudine/cli/src/commands/wrap/composition.rs` lines 231-242.

### 1.4 Unified Config Loading

**File**: `claudine/cli/src/commands/wrap/composition.rs`

**Problem**: `load_config_favorite` loads the entire config just to extract `preferred_agent`. The resolution logic now also needs the `models` override map.

**Fix**:

1. Define a new struct in `composition.rs` (or a shared module):
```rust
use std::collections::HashMap;
use claudine::config::claudine_config::ProviderModelOverride;

pub(crate) struct SelectionConfig {
    pub favorite: Option<Provider>,
    pub model_overrides: HashMap<Provider, ProviderModelOverride>,
}
```

2. Replace `load_config_favorite` with `load_selection_config`:
```rust
pub(crate) fn load_selection_config(cwd: &Path) -> Option<SelectionConfig> {
    let repo_root = sniff::filesystem::git::detect_git(cwd, false, 1)
        .ok()
        .flatten()
        .map(|info| info.repo_root);
    let config =
        claudine::dispatch::loader::load_claudine_config(None, repo_root.as_deref()).ok()?;
    Some(SelectionConfig {
        favorite: config.preferred_agent,
        model_overrides: config.models,
    })
}
```

3. Update all call sites in `composition.rs` and `sequence.rs` to use `load_selection_config` and access `.favorite`.

### 1.5 Tests

**File**: `claudine/cli/src/commands/wrap/selection_ui.rs`

Add a `#[cfg(test)]` module with tests for the row decoder logic:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_drafts() -> Vec<SequenceStepDraft> { /* ... */ }

    #[test]
    fn decode_chosen_one_provider_exact_match() {
        // Verify CellValue::ChosenOne(Some("codex")) -> Provider::Codex
    }

    #[test]
    fn decode_static_text_provider_exact_match() {
        // Verify CellValue::StaticText("claude") -> Provider::Claude
    }

    #[test]
    fn decode_unknown_provider_slug_errors() {
        // Verify unknown slug returns Err, not Provider::Claude
    }

    #[test]
    fn decode_missing_provider_errors() {
        // Verify missing/None provider cell returns Err
    }

    #[test]
    fn decode_model_text_empty_becomes_none() {
        // Verify empty model text -> None
    }

    #[test]
    fn decode_model_text_non_empty_preserved() {
        // Verify "gpt-5" -> Some("gpt-5".into())
    }
}
```

**File**: `claudine/cli/src/commands/wrap/composition.rs`

Add tests for `load_selection_config`:
```rust
#[test]
fn load_selection_config_returns_both_favorite_and_overrides() { /* ... */ }

#[test]
fn load_selection_config_handles_missing_config() { /* ... */ }
```

### 1.6 Lint

Run `cargo clippy -p claudine-cli -p claudine-lib -- -D warnings` and fix all issues.

---

## Phase 2: Model Catalog Integration (Overrides & Refresh)

**Goal**: Wire up config overrides into `ModelCatalogService` and trigger catalog refresh in the CLI so dynamic sources are actually utilized.

### 2.1 Load Config Overrides into ModelCatalogService

**File**: `claudine/lib/src/model_catalog/service.rs`

**Problem**: `ModelCatalogService::new()` creates a service with empty overrides. The CLI always calls `new()`, so user-configured model overrides are ignored.

**Fix**: The service already has `with_overrides`. We need the CLI to use it. No changes to `service.rs` are required, but we may add a convenience constructor:

```rust
/// Create a service loading overrides from the given config.
pub fn from_config(config: &crate::config::ClaudineConfig) -> Self {
    Self::with_overrides(config.models.clone())
}
```

### 2.2 Trigger Catalog Refresh

**Files**: `claudine/cli/src/commands/wrap/composition.rs`, `claudine/cli/src/commands/wrap/sequence.rs`

**Problem**: The CLI never calls `refresh()` or `refresh_all()`. Dynamic sources (e.g., `opencode models`) are never utilized unless the user manually populated cache files.

**Fix**: Add a synchronous best-effort refresh helper. The `refresh` method is async, but the CLI is synchronous. Add a blocking wrapper:

```rust
// In a new helper module or inline in composition.rs:
fn refresh_catalog_blocking(catalog: &ModelCatalogService) {
    // Best-effort background refresh; don't fail the command if network/subprocess fails
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return,
    };
    rt.block_on(async {
        let _ = catalog.refresh_all().await;
    });
}
```

**Alternative** (if tokio runtime creation is problematic): Use `std::thread::spawn` with a scoped thread and a channel, or add a `try_refresh_blocking` method to `ModelCatalogService` that encapsulates the runtime logic.

**Decision**: Add `ModelCatalogService::refresh_blocking` that creates a minimal runtime internally. This keeps the async detail inside the library.

```rust
impl ModelCatalogService {
    /// Best-effort blocking refresh of all supported providers.
    ///
    /// Creates a temporary Tokio runtime. Never panics; failures are silently ignored
    /// so that stale cache or static fallback remains available.
    pub fn refresh_blocking(&self) {
        let Ok(rt) = tokio::runtime::Runtime::new() else {
            return;
        };
        rt.block_on(async {
            let _ = self.refresh_all().await;
        });
    }
}
```

Then in `composition.rs` and `sequence.rs`, after creating the service:

```rust
let config = load_selection_config(cwd);
let catalog = match &config {
    Some(cfg) => claudine::model_catalog::ModelCatalogService::with_overrides(cfg.model_overrides.clone()),
    None => claudine::model_catalog::ModelCatalogService::new(),
};
catalog.refresh_blocking();
```

### 2.3 Update Call Sites

**Files**: `claudine/cli/src/commands/wrap/composition.rs`, `claudine/cli/src/commands/wrap/sequence.rs`

Update both files to:
1. Call `load_selection_config` instead of `load_config_favorite`
2. Pass `model_overrides` to `ModelCatalogService::with_overrides`
3. Call `refresh_blocking()` after construction

In `composition.rs` (around line 246-253):
```rust
let selection_config = load_selection_config(source_repo_root.unwrap_or(&launch_cwd));
let catalog = match &selection_config {
    Some(cfg) => claudine::model_catalog::ModelCatalogService::with_overrides(cfg.model_overrides.clone()),
    None => claudine::model_catalog::ModelCatalogService::new(),
};
catalog.refresh_blocking();
let favorite = selection_config.as_ref().and_then(|c| c.favorite);
```

In `sequence.rs` (around line 212-218):
```rust
let selection_config = load_selection_config(
    step_contexts
        .first()
        .and_then(|ctx| ctx.prepared.source_repo_root.as_deref())
        .unwrap_or(std::env::current_dir().ok().as_deref().unwrap_or(std::path::Path::new("."))),
);
let catalog = match &selection_config {
    Some(cfg) => claudine::model_catalog::ModelCatalogService::with_overrides(cfg.model_overrides.clone()),
    None => claudine::model_catalog::ModelCatalogService::new(),
};
catalog.refresh_blocking();
let favorite = selection_config.as_ref().and_then(|c| c.favorite);
```

### 2.4 Tests

**File**: `claudine/lib/src/model_catalog/service.rs`

Add test for `refresh_blocking` (it should not panic and should not fail):
```rust
#[test]
fn refresh_blocking_does_not_panic() {
    let service = ModelCatalogService::new();
    service.refresh_blocking(); // should not panic even if network is down
}
```

**New file**: `claudine/lib/tests/model_catalog_integration.rs`

Add integration tests for config override propagation:

```rust
use std::collections::HashMap;
use claudine::config::claudine_config::{DetailedModelOverride, ModelOverrideMode, ProviderModelOverride};
use claudine::events::Provider;
use claudine::model_catalog::ModelCatalogService;

#[test]
fn config_overrides_propagate_to_catalog_validation() {
    let mut overrides = HashMap::new();
    overrides.insert(
        Provider::Gemini,
        ProviderModelOverride::AddList(vec!["gemini-2.5-pro".into()]),
    );
    let service = ModelCatalogService::with_overrides(overrides);
    
    // Gemini has no static catalog, but override should make this valid
    assert!(service.is_valid(Provider::Gemini, "gemini-2.5-pro"));
}

#[test]
fn config_replace_override_replaces_static_catalog() {
    let mut overrides = HashMap::new();
    overrides.insert(
        Provider::Codex,
        ProviderModelOverride::Detailed(DetailedModelOverride {
            mode: ModelOverrideMode::Replace,
            values: vec!["custom-model".into()],
        }),
    );
    let service = ModelCatalogService::with_overrides(overrides);
    
    assert!(!service.is_valid(Provider::Codex, "o3-mini"));
    assert!(service.is_valid(Provider::Codex, "custom-model"));
}
```

**File**: `claudine/cli/src/commands/wrap/composition.rs`

Add test for catalog initialization with overrides:
```rust
#[test]
fn catalog_initialized_with_config_overrides() {
    // This is an integration-level test; we can verify the wiring by
    // checking that ModelCatalogService::with_overrides is called when
    // config has model overrides.
}
```

### 2.5 Lint

Run `cargo clippy -p claudine-cli -p claudine-lib -- -D warnings` and fix all issues.

---

## Phase 3: Sequence Review UI Polish

**Goal**: Implement the Model column as `ChooseOne` (with catalog fallback to `TextInput`) and implement column locking for explicit CLI flags.

### 3.1 Model Column: ChooseOne vs TextInput

**File**: `claudine/cli/src/commands/wrap/selection_ui.rs`

**Problem**: The Model column is always `TextInput`. It should be `ChooseOne` when a model catalog is available for the resolved provider, falling back to `TextInput` only when no catalog exists.

**Fix**: `review_sequence` needs access to the catalog or per-draft model lists. Change the signature to accept a `ModelCatalogService` reference:

```rust
pub fn review_sequence(
    drafts: Vec<SequenceStepDraft>,
    catalog: &claudine::model_catalog::ModelCatalogService,
) -> io::Result<Vec<ResolvedExecutionTarget>> {
```

Then, when building columns and initial rows, compute per-draft model options:

```rust
fn build_model_column(draft: &SequenceStepDraft, catalog: &ModelCatalogService) -> InputTableColumn {
    let provider = draft.provider_plan.options.get(draft.provider_plan.default_index)
        .map(|o| o.provider);
    
    match provider {
        Some(provider) => {
            let models = catalog.catalog_for(provider);
            if models.is_empty() {
                // Fallback to TextInput when no catalog available
                InputTableColumn::TextInput {
                    id: "model".into(),
                    config: TextInputConfig::default(),
                }
            } else {
                // ChooseOne with catalog models + "(default)" option
                let mut options = vec![ChoiceOption::new("__default__", "(default)", "__default__")];
                for model in models {
                    options.push(ChoiceOption::new(model.clone(), model.clone(), model));
                }
                InputTableColumn::ChooseOne(ChoiceInput::new("model", "Model").with_options(options))
            }
        }
        None => {
            InputTableColumn::TextInput {
                id: "model".into(),
                config: TextInputConfig::default(),
            }
        }
    }
}
```

**Important**: Since each row may have a different provider, each row may need a different column schema. However, `InputTable` requires all rows to share the same column schema. We need to verify whether `InputTable` supports per-row column schemas.

If `InputTable` does not support per-row schemas, we have two options:
1. Use `TextInput` as the common denominator when any row lacks a catalog
2. Use the union of all model options across all providers (not ideal)
3. Revisit the design decision

**Investigation needed**: Check `InputTable` API for per-row column support. If not supported, the pragmatic fix is:
- When ALL drafts have catalog-backed providers, use `ChooseOne` with the union of all models (prefixed by provider)
- Otherwise, use `TextInput`

Actually, looking at the `InputTable` design more carefully, the column schema is shared across all rows. So we need a different approach. The simplest correct approach is:

1. Build a single `ChooseOne` column for Model if at least one draft has a catalog
2. The options include all models from all providers' catalogs, prefixed with provider name
3. When decoding, parse the provider prefix to determine which model was selected

However, this is complex. A simpler approach that matches the tech design's intent:

> "Falls back to TextInput for providers whose catalog is not enumerable"

This suggests that the fallback is per-provider, but since the table columns are shared, the pragmatic implementation is:

- **If the sequence has a single resolved provider** (e.g., all rows default to the same provider, or explicit flag locks one provider): use `ChooseOne` with that provider's catalog
- **Otherwise**: use `TextInput` for all rows

But wait, the review says "Currently, users are forced to type model names manually even for providers like Claude and Codex which have static catalogs." This implies the current behavior is suboptimal even for single-provider sequences.

**Decision**: For Phase 3, implement the following heuristic:
1. If `explicit_provider` is set (all rows have the same provider), use `ChooseOne` with that provider's catalog
2. Else, if ALL drafts have the same default provider and that provider has a catalog, use `ChooseOne`
3. Otherwise, use `TextInput`

This covers the common case (single provider sequence) while avoiding incorrect behavior for multi-provider sequences.

For the initial value in `ChooseOne` mode:
- If `draft.proposed_model` is `Some(model)` and the model is in the catalog, select it
- Otherwise, select `"__default__"`

### 3.2 Column Locking

**File**: `claudine/cli/src/commands/wrap/selection_ui.rs`

**Problem**: Explicit CLI flags (`--claude`, `--model gpt-5`) should lock columns as `StaticText`. Currently not implemented.

**Fix**: When building columns and initial rows, check `draft.provider_locked` and `draft.model_locked`:

```rust
// Provider column
let provider_column = if draft.provider_locked {
    InputTableColumn::StaticText {
        id: "provider".into(),
        text: "Provider".into(),
    }
} else {
    InputTableColumn::ChooseOne(ChoiceInput::new("provider", "Provider").with_options(/* ... */))
};

// Model column (similar logic, but also considering catalog availability)
let model_column = if draft.model_locked {
    InputTableColumn::StaticText {
        id: "model".into(),
        text: "Model".into(),
    }
} else {
    // ChooseOne or TextInput based on catalog availability
    build_model_column(draft, catalog)
};
```

For initial rows:
```rust
let provider_value = if draft.provider_locked {
    CellValue::StaticText(
        draft.provider_plan.options.get(draft.provider_plan.default_index)
            .map(|o| format!("{}", o.provider))
            .unwrap_or_default()
    )
} else {
    CellValue::ChosenOne(
        draft.provider_plan.options.get(draft.provider_plan.default_index)
            .map(|o| o.provider.as_slug().to_string())
    )
};

let model_value = if draft.model_locked {
    CellValue::StaticText(draft.proposed_model.clone().unwrap_or_else(|| "(default)".into()))
} else {
    // For ChooseOne: use model ID or "__default__"
    // For TextInput: use the proposed model string
    match &model_column {
        InputTableColumn::ChooseOne(_) => {
            CellValue::ChosenOne(draft.proposed_model.clone())
        }
        _ => {
            CellValue::Text(draft.proposed_model.clone().unwrap_or_default())
        }
    }
};
```

In the decoder, handle `StaticText` for locked columns:
```rust
let provider = match row.get("provider") {
    Some(CellValue::ChosenOne(Some(slug))) => { /* existing logic */ }
    Some(CellValue::StaticText(text)) => {
        Provider::parse_cli_name(text)
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown provider in locked column: {text}")
            ))?
    }
    // ...
};

let model = match row.get("model") {
    Some(CellValue::Text(s)) | Some(CellValue::StaticText(s)) => {
        if s.is_empty() || s == "(default)" { None } else { Some(s.clone()) }
    }
    Some(CellValue::ChosenOne(Some(id))) => {
        if id == "__default__" { None } else { Some(id.clone()) }
    }
    _ => None,
};
```

### 3.3 Update Callers

**Files**: `claudine/cli/src/commands/wrap/sequence.rs`

Update the call to `review_sequence` to pass the catalog:
```rust
match super::selection_ui::review_sequence(drafts, &catalog) {
    // ...
}
```

### 3.4 Tests

**File**: `claudine/cli/src/commands/wrap/selection_ui.rs`

Add comprehensive unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use claudine::model_catalog::ModelCatalogService;

    fn make_draft(provider: Provider, locked: bool, model: Option<String>) -> SequenceStepDraft {
        SequenceStepDraft {
            step_index: 0,
            step_name: "Test".into(),
            provider_plan: ProviderPickerPlan {
                options: vec![ProviderPickerOption {
                    provider,
                    rank_reason: None,
                }],
                default_index: 0,
            },
            proposed_model: model,
            model_reason: ModelResolutionReason::ProviderDefault,
            provider_locked: locked,
            model_locked: locked,
        }
    }

    #[test]
    fn locked_provider_column_uses_static_text() {
        // Verify that when provider_locked=true, the row uses StaticText
        // and decoder correctly reads it back
    }

    #[test]
    fn locked_model_column_uses_static_text() {
        // Verify that when model_locked=true, the row uses StaticText
    }

    #[test]
    fn unlocked_provider_column_uses_choose_one() {
        // Verify CellValue::ChosenOne for unlocked provider
    }

    #[test]
    fn model_column_choose_one_when_catalog_available() {
        let mut overrides = HashMap::new();
        overrides.insert(Provider::Codex, ProviderModelOverride::AddList(vec!["gpt-5".into()]));
        let catalog = ModelCatalogService::with_overrides(overrides);
        
        let draft = make_draft(Provider::Codex, false, None);
        let column = build_model_column(&draft, &catalog);
        
        // Should be ChooseOne, not TextInput
        assert!(matches!(column, InputTableColumn::ChooseOne(_)));
    }

    #[test]
    fn model_column_text_input_when_catalog_empty() {
        let catalog = ModelCatalogService::new();
        let draft = make_draft(Provider::Gemini, false, None);
        let column = build_model_column(&draft, &catalog);
        
        // Gemini has no static catalog and no overrides
        assert!(matches!(column, InputTableColumn::TextInput { .. }));
    }

    #[test]
    fn decode_model_choose_one_default_option() {
        // Verify "__default__" -> None
    }

    #[test]
    fn decode_model_choose_one_specific_model() {
        // Verify "gpt-5" -> Some("gpt-5")
    }

    #[test]
    fn decode_model_text_input_empty() {
        // Verify "" -> None
    }

    #[test]
    fn multiple_drafts_maintain_row_order() {
        // Verify N drafts -> N rows in same order
    }
}
```

### 3.5 Lint

Run `cargo clippy -p claudine-cli -p claudine-lib -- -D warnings` and fix all issues.

---

## Post-Implementation Validation

After all three phases are complete, run the full validation suite:

```bash
# Run all tests for the claudine package areas
cargo test -p claudine-lib -p claudine-cli

# Run lint checks
cargo clippy -p claudine-lib -p claudine-cli -- -D warnings

# Verify no compilation errors
cargo check -p claudine-lib -p claudine-cli
```

All tests must pass with zero warnings.

---

## Review Issue Traceability

| Review Issue | Phase | Files Modified |
|---|---|---|
| CRITICAL: Sequence Review Provider Selection (`get_text` bug) | 1 | `selection_ui.rs` |
| Sequence Review Decoder Robustness (fuzzy match + Claude fallback) | 1 | `selection_ui.rs` |
| Redundant Provider Enumeration | 1 | `composition.rs`, `sequence.rs` |
| Unified Config Loading (`load_selection_config`) | 1 | `composition.rs`, `sequence.rs` |
| Missing Model Catalog Overrides | 2 | `composition.rs`, `sequence.rs`, `service.rs` |
| Missing Catalog Refresh | 2 | `composition.rs`, `sequence.rs`, `service.rs` |
| Sequence Review Model Column (ChooseOne vs TextInput) | 3 | `selection_ui.rs`, `sequence.rs` |
| Sequence Review Column Locking | 3 | `selection_ui.rs` |
| Interactive UI Testing | 1, 3 | `selection_ui.rs` |
| Model Catalog Integration Testing | 2 | `service.rs`, new integration test file |
