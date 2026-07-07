//! Override merging for model catalogs.
//!
//! Combines dynamically-fetched model lists with user-supplied overrides
//! from [`ClaudineConfig.models`] using additive or replace semantics.

use std::collections::HashSet;

use crate::config::claudine_config::{ModelOverrideMode, ProviderModelOverride};
use crate::provider::Provider;
/// Merge a base catalog with user overrides.
///
/// ## Returns
///
/// A de-duplicated list of model identifiers.
pub fn merge_overrides(
    _provider: Provider,
    base: &[String],
    override_entry: Option<&ProviderModelOverride>,
) -> Vec<String> {
    match override_entry {
        None => base.to_vec(),
        Some(entry) => match entry.mode() {
            ModelOverrideMode::Add => {
                let mut set = HashSet::new();
                for m in base {
                    set.insert(m.clone());
                }
                for m in entry.values() {
                    set.insert(m.to_string());
                }
                set.into_iter().collect()
            }
            ModelOverrideMode::Replace => {
                entry.values().into_iter().map(str::to_string).collect()
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::claudine_config::{DetailedModelOverride, ModelOverrideMode};

    #[test]
    fn additive_merge_adds_user_models() {
        let base = vec!["gpt-4o".into(), "o3-mini".into()];
        let override_entry = ProviderModelOverride::AddList(vec!["gpt-5".into()]);

        let merged = merge_overrides(Provider::Codex, &base, Some(&override_entry));
        assert!(merged.contains(&"gpt-4o".to_string()));
        assert!(merged.contains(&"o3-mini".to_string()));
        assert!(merged.contains(&"gpt-5".to_string()));
    }

    #[test]
    fn additive_merge_deduplicates() {
        let base = vec!["gpt-4o".into(), "o3-mini".into()];
        let override_entry = ProviderModelOverride::AddList(vec!["gpt-4o".into()]);

        let merged = merge_overrides(Provider::Codex, &base, Some(&override_entry));
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn replace_override_replaces_base() {
        let base = vec!["gpt-4o".into(), "o3-mini".into()];
        let override_entry = ProviderModelOverride::Detailed(DetailedModelOverride {
            mode: ModelOverrideMode::Replace,
            values: vec!["custom-model".into()],
        });

        let merged = merge_overrides(Provider::Codex, &base, Some(&override_entry));
        assert_eq!(merged, vec!["custom-model"]);
    }

    #[test]
    fn no_override_returns_base() {
        let base = vec!["gpt-4o".into()];
        let merged = merge_overrides(Provider::Codex, &base, None);
        assert_eq!(merged, base);
    }

    /// Object-form values (with or without a `catalog_id`) merge by their
    /// `id` exactly like bare strings — the catalog identity never leaks
    /// into the merged id list.
    #[test]
    fn object_values_merge_by_id_like_strings() {
        use crate::config::claudine_config::{ModelOverrideEntry, ModelOverrideValue};

        let base = vec!["gpt-4o".into()];
        let override_entry = ProviderModelOverride::Detailed(DetailedModelOverride {
            mode: ModelOverrideMode::Add,
            values: vec![
                "gpt-5".into(),
                ModelOverrideValue::Entry(ModelOverrideEntry {
                    id: "gpt-4o".into(),
                    catalog_id: Some("openai/gpt-4o@1".into()),
                }),
            ],
        });

        let merged = merge_overrides(Provider::Codex, &base, Some(&override_entry));
        let mut sorted = merged.clone();
        sorted.sort();
        assert_eq!(sorted, vec!["gpt-4o", "gpt-5"]);
    }

    #[test]
    fn empty_base_with_additive_override() {
        let base: Vec<String> = vec![];
        let override_entry = ProviderModelOverride::AddList(vec!["gpt-5".into()]);

        let merged = merge_overrides(Provider::Codex, &base, Some(&override_entry));
        assert_eq!(merged, vec!["gpt-5"]);
    }
}
