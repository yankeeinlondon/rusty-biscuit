//! Integration tests for model catalog config override propagation.

use std::collections::HashMap;

use claudine::config::claudine_config::{
    DetailedModelOverride, ModelOverrideMode, ProviderModelOverride,
};
use claudine::model_catalog::ModelCatalogService;
use claudine::provider::Provider;

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
