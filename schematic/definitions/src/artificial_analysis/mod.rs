//! Artificial Analysis API definitions.
//!
//! This module provides two `RestApi` definitions that share one provider:
//!
//! - [`define_artificial_analysis_data_api`] — 6 free-tier GET endpoints under
//!   `/data/...` for LLM and media-model benchmarks.
//! - [`define_artificial_analysis_critpt_api`] — 1 POST endpoint at
//!   `/critpt/evaluate` for the CritPt code-generation benchmark.
//!
//! Both APIs use the same auth (`x-api-key` header), the same env var
//! (`ARTIFICIAL_ANALYSIS_API_KEY`), and the same Rust module path
//! (`artificial_analysis`), so generated code lives in
//! `schematic_schema::artificial_analysis`.
//!
//! ## Attribution
//!
//! As required by the Artificial Analysis API terms, **all usage must include
//! attribution to <https://artificialanalysis.ai/>**.
//!
//! ## Examples
//!
//! ```rust
//! use schematic_definitions::artificial_analysis::{
//!     define_artificial_analysis_data_api,
//!     define_artificial_analysis_critpt_api,
//! };
//!
//! let data = define_artificial_analysis_data_api();
//! assert_eq!(data.name, "ArtificialAnalysisData");
//! assert_eq!(data.endpoints.len(), 6);
//!
//! let critpt = define_artificial_analysis_critpt_api();
//! assert_eq!(critpt.name, "ArtificialAnalysisCritPt");
//! assert_eq!(critpt.endpoints.len(), 1);
//! ```

mod types;

pub use types::*;

use crate::registry::{SchemaRegistry, ModuleGroup, assemble_module_group};
use schematic_define::{DefinitionContext, DefinitionError};
#[cfg(test)]
use schematic_define::RestMethod;
use schematic_define::{
    ApiKeyEnv, AuthStrategy, Endpoint, EnvList, EnvMapping,
    RestApi,
    params::{EndpointParams, QueryParamType},
};

/// Creates a schema registry containing every Artificial Analysis response and
/// request type registered against both RestApis.
///
/// `RateLimitError` is intentionally omitted — it is documentation-only and is
/// not the typed response of any endpoint.
#[must_use]
pub fn openapi_registry() -> SchemaRegistry {
    module_group().expect("valid artificial_analysis definitions").registry
}

/// The free-tier data API: 6 GET endpoints for LLM and media-model benchmarks.
pub fn define_artificial_analysis_data_api() -> RestApi {
    module_group().expect("valid artificial_analysis definitions").apis.remove(0)
}

fn build_artificial_analysis_data_api(ctx: &mut DefinitionContext) -> Result<RestApi, DefinitionError> {
    let endpoints = vec![
            Endpoint::get("ListLlmModels", "/data/llms/models")
                .describes("Returns primary metrics from independent LLM benchmarks: \
                     intelligence, speed, and pricing.")
                .returns_json::<LlmModelsResponse>(ctx)
                .build()?,
            Endpoint::get("ListTextToImageModels", "/data/media/text-to-image")
                .describes("Returns Elo ratings for text-to-image models.")
                .returns_json::<MediaModelsResponse>(ctx)
                .params(EndpointParams::default().with_query_param(
                    "include_categories",
                    QueryParamType::Boolean,
                    false,
                    Some("Include a breakdown of Elo scores by category."),
                ))
                .build()?,
            Endpoint::get("ListImageEditingModels", "/data/media/image-editing")
                .describes("Returns Elo ratings for image-editing models.")
                .returns_json::<MediaModelsResponse>(ctx)
                .build()?,
            Endpoint::get("ListTextToSpeechModels", "/data/media/text-to-speech")
                .describes("Returns Elo ratings for text-to-speech models.")
                .returns_json::<MediaModelsResponse>(ctx)
                .build()?,
            Endpoint::get("ListTextToVideoModels", "/data/media/text-to-video")
                .describes("Returns Elo ratings for text-to-video models.")
                .returns_json::<MediaModelsResponse>(ctx)
                .params(EndpointParams::default().with_query_param(
                    "include_categories",
                    QueryParamType::Boolean,
                    false,
                    Some("Include a breakdown of Elo scores by category."),
                ))
                .build()?,
            Endpoint::get("ListImageToVideoModels", "/data/media/image-to-video")
                .describes("Returns Elo ratings for image-to-video models.")
                .returns_json::<MediaModelsResponse>(ctx)
                .params(EndpointParams::default().with_query_param(
                    "include_categories",
                    QueryParamType::Boolean,
                    false,
                    Some("Include a breakdown of Elo scores by category."),
                ))
                .build()?,
        ];
    RestApi::builder(ctx, "ArtificialAnalysisData", "https://artificialanalysis.ai/api/v2")
        .describes("Artificial Analysis free data API: LLM and media-model benchmark \
             catalogs. Attribution: as required by the Artificial Analysis API terms, all \
             usage must include attribution to https://artificialanalysis.ai/.")
        .authentication(AuthStrategy::ApiKey {
            header: "x-api-key".to_string(),
            value_prefix: None,
        }, vec!["ARTIFICIAL_ANALYSIS_API_KEY".to_string()])
        .docs_url("https://artificialanalysis.ai/api-reference")
        .module_path("artificial_analysis")
        .env_mapping(EnvMapping {
            api_key: Some(ApiKeyEnv {
                names: EnvList::single("ARTIFICIAL_ANALYSIS_API_KEY"),
                header: "x-api-key".to_string(),
            }),
            ..Default::default()
        })
        .endpoints(endpoints)
        .build()
}

/// The CritPt benchmark evaluation API: a single POST endpoint.
pub fn define_artificial_analysis_critpt_api() -> RestApi {
    module_group().expect("valid artificial_analysis definitions").apis.remove(1)
}

fn build_artificial_analysis_critpt_api(ctx: &mut DefinitionContext) -> Result<RestApi, DefinitionError> {
    let endpoints = vec![Endpoint::post("EvaluateCritPt", "/critpt/evaluate")
                .describes("Submit a batch of code-generation submissions for evaluation against \
                 the CritPt benchmark.")
                .request_json::<CritPtEvaluateBody>(ctx)
                .returns_json::<CritPtEvaluateResponse>(ctx)
                .build()?];
    RestApi::builder(ctx, "ArtificialAnalysisCritPt", "https://artificialanalysis.ai/api/v2")
        .describes("Artificial Analysis CritPt benchmark: submit code-generation results for \
             evaluation. Attribution: as required by the Artificial Analysis API terms, all \
             usage must include attribution to https://artificialanalysis.ai/.")
        .authentication(AuthStrategy::ApiKey {
            header: "x-api-key".to_string(),
            value_prefix: None,
        }, vec!["ARTIFICIAL_ANALYSIS_API_KEY".to_string()])
        .docs_url("https://artificialanalysis.ai/api-reference")
        .module_path("artificial_analysis")
        .env_mapping(EnvMapping {
            api_key: Some(ApiKeyEnv {
                names: EnvList::single("ARTIFICIAL_ANALYSIS_API_KEY"),
                header: "x-api-key".to_string(),
            }),
            ..Default::default()
        })
        .endpoints(endpoints)
        .build()
}

/// Assemble sibling APIs and their ordered nominal schemas in one context.
pub fn module_group() -> Result<ModuleGroup, DefinitionError> {
    assemble_module_group("::schematic_definitions::artificial_analysis", |ctx| {
        ctx.register::<ModelCreator>()?;
        ctx.register::<CategoryBreakdown>()?;
        ctx.register::<LlmEvaluations>()?;
        ctx.register::<LlmPricing>()?;
        ctx.register::<LlmModel>()?;
        ctx.register::<PromptOptions>()?;
        ctx.register::<LlmModelsResponse>()?;
        ctx.register::<MediaModel>()?;
        ctx.register::<MediaModelsResponse>()?;
        ctx.register::<CritPtMessage>()?;
        ctx.register::<CritPtSubmission>()?;
        ctx.register::<CritPtEvaluateBody>()?;
        ctx.register::<CritPtEvaluateResponse>()?;
        Ok(vec![
            build_artificial_analysis_data_api(ctx)?,
            build_artificial_analysis_critpt_api(ctx)?,
        ])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Data API ----------------------------------------------------------

    #[test]
    fn data_api_has_correct_metadata() {
        let api = define_artificial_analysis_data_api();
        assert_eq!(api.name, "ArtificialAnalysisData");
        assert_eq!(api.base_url, "https://artificialanalysis.ai/api/v2");
        assert_eq!(api.module_path.as_deref(), Some("artificial_analysis"));
        assert!(api.docs_url.is_some());
    }

    #[test]
    fn data_api_uses_apikey_header_auth() {
        let api = define_artificial_analysis_data_api();
        match &api.auth {
            AuthStrategy::ApiKey { header, .. } => assert_eq!(header, "x-api-key"),
            other => panic!("expected ApiKey, got {other:?}"),
        }
        assert_eq!(api.env_auth, vec!["ARTIFICIAL_ANALYSIS_API_KEY"]);
    }

    #[test]
    fn data_api_has_six_endpoints() {
        let api = define_artificial_analysis_data_api();
        assert_eq!(api.endpoints.len(), 6);
    }

    #[test]
    fn data_api_endpoint_paths() {
        let api = define_artificial_analysis_data_api();
        let paths: Vec<_> = api.endpoints.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"/data/llms/models"));
        assert!(paths.contains(&"/data/media/text-to-image"));
        assert!(paths.contains(&"/data/media/image-editing"));
        assert!(paths.contains(&"/data/media/text-to-speech"));
        assert!(paths.contains(&"/data/media/text-to-video"));
        assert!(paths.contains(&"/data/media/image-to-video"));
    }

    #[test]
    fn data_api_include_categories_present_only_where_expected() {
        // Endpoints with the include_categories query param.
        for id in [
            "ListTextToImageModels",
            "ListTextToVideoModels",
            "ListImageToVideoModels",
        ] {
            let ep = endpoint(id);
            let params = ep.params.as_ref().expect(id);
            assert!(params.query.iter().any(|p| p.name == "include_categories"));
        }
        // Endpoints that should have no params.
        for id in [
            "ListLlmModels",
            "ListImageEditingModels",
            "ListTextToSpeechModels",
        ] {
            let ep = endpoint(id);
            assert!(ep.params.is_none(), "{id} should have no params");
        }
    }

    fn endpoint(id: &str) -> schematic_define::Endpoint {
        define_artificial_analysis_data_api()
            .endpoints
            .into_iter()
            .find(|e| e.id == id)
            .unwrap_or_else(|| panic!("endpoint {id} missing"))
    }

    // -- CritPt API --------------------------------------------------------

    #[test]
    fn critpt_api_has_correct_metadata() {
        let api = define_artificial_analysis_critpt_api();
        assert_eq!(api.name, "ArtificialAnalysisCritPt");
        assert_eq!(api.module_path.as_deref(), Some("artificial_analysis"));
    }

    #[test]
    fn critpt_api_has_one_endpoint() {
        let api = define_artificial_analysis_critpt_api();
        assert_eq!(api.endpoints.len(), 1);
        let ep = &api.endpoints[0];
        assert_eq!(ep.id, "EvaluateCritPt");
        assert_eq!(ep.method, RestMethod::Post);
        assert_eq!(ep.path, "/critpt/evaluate");
        assert!(ep.request.is_some());
    }

    #[test]
    fn critpt_api_uses_same_env_var() {
        let api = define_artificial_analysis_critpt_api();
        assert_eq!(api.env_auth, vec!["ARTIFICIAL_ANALYSIS_API_KEY"]);
    }

    // -- Registry ----------------------------------------------------------

    #[test]
    fn registry_validates_against_both_apis() {
        let registry = openapi_registry();
        registry
            .validate_completeness(&define_artificial_analysis_data_api())
            .expect("data API registry should be complete");
        registry
            .validate_completeness(&define_artificial_analysis_critpt_api())
            .expect("critpt API registry should be complete");
    }

    #[test]
    fn registry_does_not_register_rate_limit_error() {
        // RateLimitError is documentation-only.
        let registry = openapi_registry();
        assert!(registry.get("RateLimitError").is_none());
    }

    #[test]
    fn module_group_preserves_endpoint_and_registry_inventory() {
        let group = module_group().unwrap();
        assert_eq!(
            group.apis.iter().flat_map(|api| api.endpoints.iter().map(|endpoint| endpoint.id.as_str())).collect::<Vec<_>>(),
            vec!["ListLlmModels", "ListTextToImageModels", "ListImageEditingModels", "ListTextToSpeechModels", "ListTextToVideoModels", "ListImageToVideoModels", "EvaluateCritPt"]
        );
        assert_eq!(group.registry.names().collect::<Vec<_>>(), vec!["ModelCreator", "CategoryBreakdown", "LlmEvaluations", "LlmPricing", "LlmModel", "PromptOptions", "LlmModelsResponse", "MediaModel", "MediaModelsResponse", "CritPtMessage", "CritPtSubmission", "CritPtEvaluateBody", "CritPtEvaluateResponse"]);
        for api in &group.apis {
            assert!(group.registry.validate_completeness(api).is_ok());
        }
    }

}
