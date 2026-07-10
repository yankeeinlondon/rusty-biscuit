//! Typed [`PlatformKind`] descriptor for the provider catalog.

use serde::Serialize;
use strum::{EnumIter, IntoStaticStr, VariantNames};

/// How a provider's CLI product is positioned.
///
/// Vocabulary ratified at Checkpoint D (2026-07-04, question 4): every
/// provider is exactly one of these two kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PlatformKind {
    /// Predominantly fronts the vendor's own models, where rolling-alias
    /// handling matters.
    VendorPlatform,
    /// Model-agnostic front-end where provider/model pair selection is
    /// the central UX.
    AgentAggregator,
}

#[cfg(test)]
mod tests {
    use strum::VariantNames;

    use super::*;

    /// Facts values are authored in these snake_case wire forms.
    #[test]
    fn variant_names_are_snake_case_wire_forms() {
        assert_eq!(
            PlatformKind::VARIANTS,
            &["vendor_platform", "agent_aggregator"]
        );
    }

    #[test]
    fn serde_wire_forms_match_catalog_shape() {
        let value = serde_json::to_value(PlatformKind::VendorPlatform).unwrap();
        assert_eq!(value, serde_json::json!("vendor_platform"));
    }
}
