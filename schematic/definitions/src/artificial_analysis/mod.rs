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

use crate::registry::SchemaRegistry;
use schematic_define::{
    ApiKeyEnv, ApiRequest, ApiResponse, AuthStrategy, Endpoint, EnvList, EnvMapping, RestApi,
    RestMethod,
    params::{EndpointParams, QueryParamType},
};

/// Creates a schema registry containing every Artificial Analysis response and
/// request type registered against both RestApis.
///
/// `RateLimitError` is intentionally omitted — it is documentation-only and is
/// not the typed response of any endpoint.
#[must_use]
pub fn openapi_registry() -> SchemaRegistry {
    SchemaRegistry::new()
        // Shared
        .register::<ModelCreator>("ModelCreator")
        .register::<CategoryBreakdown>("CategoryBreakdown")
        // LLM
        .register::<LlmEvaluations>("LlmEvaluations")
        .register::<LlmPricing>("LlmPricing")
        .register::<LlmModel>("LlmModel")
        .register::<PromptOptions>("PromptOptions")
        .register::<LlmModelsResponse>("LlmModelsResponse")
        // Media
        .register::<MediaModel>("MediaModel")
        .register::<MediaModelsResponse>("MediaModelsResponse")
        // CritPt
        .register::<CritPtMessage>("CritPtMessage")
        .register::<CritPtSubmission>("CritPtSubmission")
        .register::<CritPtEvaluateBody>("CritPtEvaluateBody")
        .register::<CritPtEvaluateResponse>("CritPtEvaluateResponse")
}

/// The free-tier data API: 6 GET endpoints for LLM and media-model benchmarks.
pub fn define_artificial_analysis_data_api() -> RestApi {
    RestApi {
        name: "ArtificialAnalysisData".to_string(),
        description: "Artificial Analysis free data API: LLM and media-model benchmark \
             catalogs. Attribution: as required by the Artificial Analysis API terms, all \
             usage must include attribution to https://artificialanalysis.ai/."
            .to_string(),
        base_url: "https://artificialanalysis.ai/api/v2".to_string(),
        docs_url: Some("https://artificialanalysis.ai/api-reference".to_string()),
        auth: AuthStrategy::ApiKey {
            header: "x-api-key".to_string(),
            value_prefix: None,
        },
        auth_policy: None,
        env_auth: vec!["ARTIFICIAL_ANALYSIS_API_KEY".to_string()],
        env_username: None,
        headers: vec![],
        endpoints: vec![
            Endpoint {
                id: "ListLlmModels".to_string(),
                method: RestMethod::Get,
                path: "/data/llms/models".to_string(),
                description: "Returns primary metrics from independent LLM benchmarks: \
                     intelligence, speed, and pricing."
                    .to_string(),
                request: None,
                response: ApiResponse::json_type("LlmModelsResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "ListTextToImageModels".to_string(),
                method: RestMethod::Get,
                path: "/data/media/text-to-image".to_string(),
                description: "Returns Elo ratings for text-to-image models.".to_string(),
                request: None,
                response: ApiResponse::json_type("MediaModelsResponse"),
                headers: vec![],
                params: Some(EndpointParams::default().with_query_param(
                    "include_categories",
                    QueryParamType::Boolean,
                    false,
                    Some("Include a breakdown of Elo scores by category."),
                )),
                oauth_scopes: None,
            },
            Endpoint {
                id: "ListImageEditingModels".to_string(),
                method: RestMethod::Get,
                path: "/data/media/image-editing".to_string(),
                description: "Returns Elo ratings for image-editing models.".to_string(),
                request: None,
                response: ApiResponse::json_type("MediaModelsResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "ListTextToSpeechModels".to_string(),
                method: RestMethod::Get,
                path: "/data/media/text-to-speech".to_string(),
                description: "Returns Elo ratings for text-to-speech models.".to_string(),
                request: None,
                response: ApiResponse::json_type("MediaModelsResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "ListTextToVideoModels".to_string(),
                method: RestMethod::Get,
                path: "/data/media/text-to-video".to_string(),
                description: "Returns Elo ratings for text-to-video models.".to_string(),
                request: None,
                response: ApiResponse::json_type("MediaModelsResponse"),
                headers: vec![],
                params: Some(EndpointParams::default().with_query_param(
                    "include_categories",
                    QueryParamType::Boolean,
                    false,
                    Some("Include a breakdown of Elo scores by category."),
                )),
                oauth_scopes: None,
            },
            Endpoint {
                id: "ListImageToVideoModels".to_string(),
                method: RestMethod::Get,
                path: "/data/media/image-to-video".to_string(),
                description: "Returns Elo ratings for image-to-video models.".to_string(),
                request: None,
                response: ApiResponse::json_type("MediaModelsResponse"),
                headers: vec![],
                params: Some(EndpointParams::default().with_query_param(
                    "include_categories",
                    QueryParamType::Boolean,
                    false,
                    Some("Include a breakdown of Elo scores by category."),
                )),
                oauth_scopes: None,
            },
        ],
        module_path: Some("artificial_analysis".to_string()),
        request_suffix: None,
        version: None,
        env_mapping: Some(EnvMapping {
            api_key: Some(ApiKeyEnv {
                names: EnvList::single("ARTIFICIAL_ANALYSIS_API_KEY"),
                header: "x-api-key".to_string(),
            }),
            ..Default::default()
        }),
    }
}

/// The CritPt benchmark evaluation API: a single POST endpoint.
pub fn define_artificial_analysis_critpt_api() -> RestApi {
    RestApi {
        name: "ArtificialAnalysisCritPt".to_string(),
        description: "Artificial Analysis CritPt benchmark: submit code-generation results for \
             evaluation. Attribution: as required by the Artificial Analysis API terms, all \
             usage must include attribution to https://artificialanalysis.ai/."
            .to_string(),
        base_url: "https://artificialanalysis.ai/api/v2".to_string(),
        docs_url: Some("https://artificialanalysis.ai/api-reference".to_string()),
        auth: AuthStrategy::ApiKey {
            header: "x-api-key".to_string(),
            value_prefix: None,
        },
        auth_policy: None,
        env_auth: vec!["ARTIFICIAL_ANALYSIS_API_KEY".to_string()],
        env_username: None,
        headers: vec![],
        endpoints: vec![Endpoint {
            id: "EvaluateCritPt".to_string(),
            method: RestMethod::Post,
            path: "/critpt/evaluate".to_string(),
            description: "Submit a batch of code-generation submissions for evaluation against \
                 the CritPt benchmark."
                .to_string(),
            request: Some(ApiRequest::json_type("CritPtEvaluateBody")),
            response: ApiResponse::json_type("CritPtEvaluateResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        }],
        module_path: Some("artificial_analysis".to_string()),
        request_suffix: None,
        version: None,
        env_mapping: Some(EnvMapping {
            api_key: Some(ApiKeyEnv {
                names: EnvList::single("ARTIFICIAL_ANALYSIS_API_KEY"),
                header: "x-api-key".to_string(),
            }),
            ..Default::default()
        }),
    }
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
}
