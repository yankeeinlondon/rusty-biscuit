//! Typed [`BillingModel`] descriptor for the provider catalog.

use serde::Serialize;
use strum::{EnumIter, IntoStaticStr, VariantNames};

/// How a provider charges for usage.
///
/// Vocabulary recovered from the retired legacy `AgentCapabilities` tree
/// (Phase D wave 1). A provider may offer several models at once; the
/// catalog carries the full list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum BillingModel {
    /// Flat-rate subscription plan.
    Subscription,
    /// Metered pay-per-token API billing.
    PerToken,
    /// Prepaid credit packs drawn down by usage.
    PrepaidCredits,
    /// The tool itself bills nothing; costs accrue with whichever upstream
    /// provider the user configures (aggregators).
    ProviderOnly,
}

#[cfg(test)]
mod tests {
    use strum::VariantNames;

    use super::*;

    /// Facts lists are authored in these snake_case wire forms.
    #[test]
    fn variant_names_are_snake_case_wire_forms() {
        assert_eq!(
            BillingModel::VARIANTS,
            &["subscription", "per_token", "prepaid_credits", "provider_only"]
        );
    }

    #[test]
    fn serde_wire_forms_match_catalog_shape() {
        let value = serde_json::to_value(BillingModel::PerToken).unwrap();
        assert_eq!(value, serde_json::json!("per_token"));
    }
}
