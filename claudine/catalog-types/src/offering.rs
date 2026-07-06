//! `&'static` row types for the generated expected-offering records.
//!
//! `claudine-gen` compiles the agent-models `default_models[]` research
//! (classified and joined to the unchained-ai models-catalog artifact)
//! into `ProviderInfo.expected_offerings`, and the model-config
//! `local_runners[]` research into `ProviderInfo.offering_sources`.
//! Plan endpoints and local runners get first-class records here — the
//! unchained catalog stays model-API-only by design (F4 ruling,
//! design/model-catalog-boundary.md).

use serde::Serialize;
use strum::{EnumIter, IntoStaticStr, VariantNames};

/// Classification of one model offering (or offering-source namespace).
///
/// The strum introspection derives back the generator's schema↔catalog
/// compatibility gate, mirroring [`crate::ModelCatalogSource`]:
/// snake_case member strings are the serde wire form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum OfferingClass {
    /// A model served by the vendor's own model API.
    VendorApi,
    /// A managed subscription/plan endpoint (e.g. `kimi-for-coding`) —
    /// absent from the unchained-ai catalog by design.
    PlanEndpoint,
    /// A gateway offering fronting other vendors' models (e.g. the
    /// `opencode/` Zen gateway ids).
    Aggregator,
    /// A local-runner namespace (ollama, omlx, ...).
    LocalRunner,
}

/// How a local runner integrates with a provider (schema-compat gate
/// pattern: the model-config sidecar's `integration` enum members must be
/// a subset of these snake_case wire forms).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum LocalRunnerIntegration {
    /// The provider ships explicit runner integration.
    FirstClass,
    /// Works by pointing a compatible client at the runner's endpoint.
    BaseUrlOverride,
    /// Needs a translation shim between API standards.
    ProxyRequired,
    /// No integration path exists.
    Unsupported,
}

/// How a record's id/alias resolves to a concrete model at answer time
/// (schema-compat gate pattern: snake_case member strings are the serde
/// wire form).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIter, IntoStaticStr, VariantNames)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ResolvesVia {
    /// Resolve through the vendored family index's `latest` entry.
    FamilyLatest,
}

/// One expected offering — a model id the provider is known to serve out
/// of the box (agent-models research, generated).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ExpectedOffering {
    /// The exact id the provider CLI/config accepts.
    pub id: &'static str,
    /// Rolling alias accepted alongside `id` (e.g. `opus`, `kimi-code`).
    pub alias: Option<&'static str>,
    /// Whether this is the provider's default model.
    pub is_default: bool,
    /// Context window in tokens, when the research records one.
    pub context_window: Option<u32>,
    pub class: OfferingClass,
    /// Identity key joining the unchained-ai models-catalog artifact
    /// (`vendor/family[@version](+variant)*` grammar, derived by
    /// unchained-ai's parser — Claudine joins by lookup only). `None`
    /// means no confident join: plan endpoints by design, drifted
    /// spellings or artifact coverage gaps otherwise.
    pub catalog_id: Option<&'static str>,
    /// Marks a record whose id/alias is a rolling pointer the provider
    /// re-targets over releases; consumers answer "what does it mean
    /// right now" through the vendored family index instead of trusting
    /// the researched pin.
    pub resolves: Option<ResolvesVia>,
}

/// One offering-source namespace — an id prefix under which a user can
/// route offerings beyond the expected set (local runners today).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct OfferingSource {
    /// The id-prefix namespace (the model-config `runner` member, e.g.
    /// `ollama` for `ollama/*` ids).
    pub prefix: &'static str,
    pub class: OfferingClass,
    /// API standard the integration path rides on (model-config
    /// `standard` member), when recorded.
    pub api_standard: Option<&'static str>,
    /// Integration level, when recorded ([`OfferingClass::LocalRunner`]
    /// rows always carry one — the sidecar requires it).
    pub integration: Option<LocalRunnerIntegration>,
}

#[cfg(test)]
mod tests {
    use strum::VariantNames;

    use super::*;

    /// The sidecar enum-subset gate compares against these snake_case
    /// lists.
    #[test]
    fn variant_names_are_snake_case_wire_forms() {
        assert_eq!(
            OfferingClass::VARIANTS,
            &["vendor_api", "plan_endpoint", "aggregator", "local_runner"]
        );
        assert_eq!(
            LocalRunnerIntegration::VARIANTS,
            &[
                "first_class",
                "base_url_override",
                "proxy_required",
                "unsupported"
            ]
        );
        assert_eq!(ResolvesVia::VARIANTS, &["family_latest"]);
    }

    /// The serde wire form IS the catalog shape (`catalog.json` value /
    /// override `value:` authoring form), so this pins the contract.
    #[test]
    fn serde_wire_forms_match_catalog_shape() {
        let offering = ExpectedOffering {
            id: "kimi-for-coding",
            alias: Some("kimi-code"),
            is_default: true,
            context_window: Some(262144),
            class: OfferingClass::PlanEndpoint,
            catalog_id: None,
            resolves: None,
        };
        assert_eq!(
            serde_json::to_value(offering).unwrap(),
            serde_json::json!({
                "id": "kimi-for-coding",
                "alias": "kimi-code",
                "is_default": true,
                "context_window": 262144,
                "class": "plan_endpoint",
                "catalog_id": null,
                "resolves": null,
            })
        );
        assert_eq!(
            serde_json::to_value(ResolvesVia::FamilyLatest).unwrap(),
            serde_json::json!("family_latest")
        );

        let source = OfferingSource {
            prefix: "ollama",
            class: OfferingClass::LocalRunner,
            api_standard: Some("openai_compatible"),
            integration: Some(LocalRunnerIntegration::FirstClass),
        };
        assert_eq!(
            serde_json::to_value(source).unwrap(),
            serde_json::json!({
                "prefix": "ollama",
                "class": "local_runner",
                "api_standard": "openai_compatible",
                "integration": "first_class",
            })
        );
    }
}
