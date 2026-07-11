//! Typed [`ResumeSupport`] descriptor for the provider catalog.

use serde::Serialize;
use strum::{EnumIter, IntoStaticStr, VariantNames};

/// Overall resume support level for a provider's session continuation.
///
/// Deliberately the support LEVEL only (2026-07-04 resume-parity ruling):
/// session-id capture and resume argv mechanics stay out of the catalog
/// and live with the wrapper profiles.
///
/// The strum introspection derives back the generator's schema↔catalog
/// compatibility gate: the resume research sidecar's `support` enum must
/// declare members that are a subset of [`ResumeSupport::VARIANTS`]
/// (snake_case, matching the serde wire form).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ResumeSupport {
    /// Resumable on both interactive and non-interactive surfaces with a
    /// stable session handle.
    FirstClass,
    /// Resumable, but with material caveats on at least one surface.
    Partial,
    /// Resume exists only on the interactive surface.
    InteractiveOnly,
    /// Resume exists only on the non-interactive surface.
    NonInteractiveOnly,
    /// No resume support.
    None,
    /// Resume behavior has not been verified.
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
            ResumeSupport::VARIANTS,
            &[
                "first_class",
                "partial",
                "interactive_only",
                "non_interactive_only",
                "none",
                "unknown"
            ]
        );
    }

    /// Members serialize as bare snake_case strings — the catalog shape
    /// (`catalog.json` value / override `value:` authoring form).
    #[test]
    fn serde_wire_forms_match_catalog_shape() {
        let value = serde_json::to_value(ResumeSupport::FirstClass).unwrap();
        assert_eq!(value, serde_json::json!("first_class"));
    }
}
