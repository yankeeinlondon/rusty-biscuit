//! Typed [`ModelCatalogSource`] descriptor for the provider catalog.
//!
//! Centralizes how each provider sources its model catalog so the
//! `model_catalog::provider_sources` module can dispatch on a typed flag
//! instead of `match Provider { ... }`.

use serde::Serialize;
use strum::{EnumIter, IntoStaticStr, VariantNames};

/// Source of a provider's model catalog.
///
/// Mechanism-only by design (2026-07-05 ruling): variants describe HOW a
/// catalog is obtained, never WHICH provider obtains it. Model ground
/// truth joins arrive generate-time via the committed unchained-ai
/// artifact (Phase F), not through this enum.
///
/// The strum introspection derives back the generator's schema↔catalog
/// compatibility gate: research sidecars that feed this enum must declare
/// `enum(...)` members that are a subset of [`ModelCatalogSource::VARIANTS`]
/// (snake_case, matching the serde wire form). `ShellCommand` carries
/// data, so its catalog/override shape is the externally tagged object
/// `{"shell_command": {"program": ..., "args": [...]}}` rather than the
/// bare member string the unit variants use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ModelCatalogSource {
    /// No dynamic listing; validation relies solely on the generated
    /// `expected_offerings` baseline.
    None,
    /// Programmatic listing: shells out to `program` with `args`; stdout
    /// yields the model list, consumed as the drift channel.
    ShellCommand {
        /// Executable looked up on `PATH`.
        program: &'static str,
        /// Arguments passed to `program`.
        args: &'static [&'static str],
    },
}

#[cfg(test)]
mod tests {
    use strum::VariantNames;

    use super::*;

    /// The sidecar enum-subset gate compares against this snake_case list.
    #[test]
    fn variant_names_are_snake_case_wire_forms() {
        assert_eq!(ModelCatalogSource::VARIANTS, &["none", "shell_command"]);
    }

    /// Unit variants serialize as bare member strings; the struct variant
    /// serializes as an externally tagged object. Both forms ARE the
    /// catalog shape (`catalog.json` value / override `value:` authoring
    /// form), so this pins the wire contract.
    #[test]
    fn serde_wire_forms_match_catalog_shape() {
        let unit = serde_json::to_value(ModelCatalogSource::None).unwrap();
        assert_eq!(unit, serde_json::json!("none"));

        let shell = serde_json::to_value(ModelCatalogSource::ShellCommand {
            program: "opencode",
            args: &["models"],
        })
        .unwrap();
        assert_eq!(
            shell,
            serde_json::json!({"shell_command": {"program": "opencode", "args": ["models"]}})
        );
    }

    #[test]
    fn into_static_str_covers_the_data_variant() {
        let shell = ModelCatalogSource::ShellCommand {
            program: "opencode",
            args: &["models"],
        };
        assert_eq!(<&'static str>::from(shell), "shell_command");
        assert_eq!(<&'static str>::from(ModelCatalogSource::None), "none");
    }
}
