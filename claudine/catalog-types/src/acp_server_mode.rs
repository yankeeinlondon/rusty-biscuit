//! Typed [`AcpServerMode`] descriptor for the provider catalog.

use serde::Serialize;
use strum::{EnumIter, IntoStaticStr, VariantNames};

/// ACP server posture the provider itself offers.
///
/// Mirrors the `docs/research/acp/_schema.yaml` `support` vocabulary
/// verbatim — the sidecar is the vocabulary authority (Ken, 2026-07-05).
/// This records what the provider offers, NOT whether Claudine integrates
/// with it (Claudine does not use ACP today).
///
/// The strum introspection drives the generator's schema↔catalog
/// compatibility gate: the acp research sidecar's `support` enum must
/// declare members that are a subset of [`AcpServerMode::VARIANTS`]
/// (snake_case, matching the serde wire form).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AcpServerMode {
    /// The provider CLI exposes ACP as a first-class server mode.
    Native,
    /// ACP is available through a separate adapter package.
    Adapter,
    /// Partial or incomplete ACP support.
    Partial,
    /// No ACP support.
    None,
    /// ACP support has not been verified.
    Unknown,
}

#[cfg(test)]
mod tests {
    use strum::VariantNames;

    use super::*;

    /// The sidecar enum-subset gate compares against this snake_case list.
    #[test]
    fn variant_names_are_snake_case_wire_forms() {
        assert_eq!(
            AcpServerMode::VARIANTS,
            &["native", "adapter", "partial", "none", "unknown"]
        );
    }

    /// Members serialize as bare snake_case strings — the catalog shape
    /// (`catalog.json` value / override `value:` authoring form).
    #[test]
    fn serde_wire_forms_match_catalog_shape() {
        let value = serde_json::to_value(AcpServerMode::Adapter).unwrap();
        assert_eq!(value, serde_json::json!("adapter"));
    }
}
