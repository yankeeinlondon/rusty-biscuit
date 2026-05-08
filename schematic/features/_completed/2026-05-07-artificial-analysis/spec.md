# Artificial Analysis API Definition Spec

## Overview

- **API Name:** `ArtificialAnalysis`
- **Base URL:** `https://artificialanalysis.ai/api/v2`
- **Docs URL:** `https://artificialanalysis.ai/api-reference`
- **Module slug (kebab):** `artificial-analysis`
- **Module path (Rust):** `artificial_analysis`

The Artificial Analysis API provides independent AI model benchmarks (intelligence
evaluations, speed benchmarks, pricing for LLMs, and Elo ratings for media-generation
models), plus an authenticated CritPt benchmark evaluation endpoint.

> **Attribution requirement.** As required by the Artificial Analysis API terms,
> all usage must include attribution to <https://artificialanalysis.ai/>.

This spec describes a single Artificial Analysis provider that is split into **two**
`RestApi` definitions sharing one Rust module — mirroring the existing `emqx` pattern
(`EmqxBasic` + `EmqxBearer`) and `ollama` pattern (`OllamaNative` + `OllamaOpenAI`).

## Decisions Summary

These are the architectural decisions that shape this spec. They were resolved during
human-in-the-loop review and should not be silently revisited.

- **Two RestApis, one module.** `define_artificial_analysis_data_api()` covers the 6
  free-tier `/data/...` GET endpoints; `define_artificial_analysis_critpt_api()` covers
  the single `/critpt/evaluate` POST endpoint. Both live under
  `schematic/definitions/src/artificial_analysis/`.
- **Both RestApis share one `module_path`** (`"artificial_analysis"`) so generated code
  lives in `schematic_schema::artificial_analysis`.
- **Both RestApis share one env var** (`ARTIFICIAL_ANALYSIS_API_KEY`) with no fallbacks,
  declared via the legacy `auth` + `env_auth` fields *and* the modern `env_mapping`.
- **Auth strategy is the same on both:** API key in the `x-api-key` header.
- **All shared and per-API types live together** in `types.rs`. There is no further
  split into `data_types.rs` / `critpt_types.rs`.
- **`generation_config` and `batch_metadata` are typed as `serde_json::Value`.** This
  is a new precedent in this codebase; both fields derive a permissive `JsonSchema`
  through `serde_json::Value`'s built-in support.
- **`RateLimitError` is documented but not registered.** No endpoint currently emits
  it as a typed response, so it is intentionally **not** included in
  `openapi_registry()`. Keeping the struct around documents the 429 body shape.
- **Numeric integers stay as `i32`** (matching the original spec) even where unsigned
  would be more accurate. See *Notes & Considerations*.
- **Query parameters use the canonical `EndpointParams` API.** No bespoke `Param`,
  `ParamLocation`, or `ParamType` types exist in `schematic-define` — earlier drafts
  that referenced them were broken.

## Authentication

Applies identically to both RestApis.

- **Strategy:** `AuthStrategy::ApiKey { header: "x-api-key" }`
- **Environment variable:** `ARTIFICIAL_ANALYSIS_API_KEY` (no fallback chain)
- **Free-tier rate limit (data API):** 1,000 requests/day
- **CritPt rate limit:** 10 requests / 24-hour window

Each `RestApi` declares all three auth-related fields so the generator's legacy and
modern paths agree:

```rust
auth: AuthStrategy::ApiKey { header: "x-api-key".to_string() },
auth_policy: None,
env_auth: vec!["ARTIFICIAL_ANALYSIS_API_KEY".to_string()],
env_username: None,
env_mapping: Some(EnvMapping {
    api_key: Some(EnvList::single("ARTIFICIAL_ANALYSIS_API_KEY")),
    ..Default::default()
}),
```

## Module Structure

```
schematic/definitions/src/artificial_analysis/
├── mod.rs    # define_artificial_analysis_data_api()
│             # define_artificial_analysis_critpt_api()
│             # openapi_registry()
│             # #[cfg(test)] mod tests
└── types.rs  # All request/response structs (shared + LLM + media + CritPt)
```

The `mod.rs` `//!` doc comment must include the attribution requirement so readers
of the generated rustdoc see it without leaving the source tree.

## Types (`types.rs`)

All structs derive `Debug, Clone, Serialize, Deserialize, JsonSchema`. Doc comments are
required on every public item — match the density of `openai/types.rs`. Field-level
`#[serde(skip_serializing_if = "Option::is_none")]` annotations should be preserved
from the original spec.

### Shared

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A model creator (lab, company, or organization) referenced by the API.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModelCreator {
    /// Stable identifier for the creator.
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// URL-safe slug (may change over time — prefer `id`).
    pub slug: String,
}

/// A per-category breakdown of Elo ratings for a media model.
///
/// Returned only when `include_categories=true` is passed to a media endpoint
/// that supports it.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CategoryBreakdown {
    pub style_category: String,
    pub subject_matter_category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format_category: Option<String>,
    pub elo: i32,
    pub ci95: String,
    pub appearances: i32,
}
```

### LLM endpoint

```rust
/// Benchmark scores for a single LLM model.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmEvaluations {
    pub artificial_analysis_intelligence_index: f64,
    pub artificial_analysis_coding_index: f64,
    pub artificial_analysis_math_index: f64,
    pub mmlu_pro: f64,
    pub gpqa: f64,
    pub hle: f64,
    pub livecodebench: f64,
    pub scicode: f64,
    pub math_500: f64,
    pub aime: f64,
}

/// Pricing for an LLM model in USD per 1M tokens.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmPricing {
    pub price_1m_blended_3_to_1: f64,
    pub price_1m_input_tokens: f64,
    pub price_1m_output_tokens: f64,
}

/// A single LLM model entry returned by `ListLlmModels`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmModel {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub model_creator: ModelCreator,
    pub evaluations: LlmEvaluations,
    pub pricing: LlmPricing,
    pub median_output_tokens_per_second: f64,
    pub median_time_to_first_token_seconds: f64,
}

/// Configuration metadata describing how the LLM benchmarks were collected.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PromptOptions {
    pub parallel_queries: i32,
    pub prompt_length: String,
}

/// Response body for the `ListLlmModels` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmModelsResponse {
    pub status: i32,
    pub prompt_options: PromptOptions,
    pub data: Vec<LlmModel>,
}
```

### Media endpoints (shared shape)

```rust
/// A single media-generation model entry. Used by all `/data/media/*` endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MediaModel {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub model_creator: ModelCreator,
    pub elo: i32,
    pub rank: i32,
    pub ci95: String,
    pub appearances: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<CategoryBreakdown>>,
}

/// Shared response shape for all `/data/media/*` endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MediaModelsResponse {
    pub status: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_categories: Option<bool>,
    pub data: Vec<MediaModel>,
}
```

### CritPt evaluation

```rust
/// A single message in the prompt history sent to the model under test.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CritPtMessage {
    pub role: String,
    pub content: String,
}

/// One submission in a CritPt evaluation batch.
///
/// `generation_config` is an arbitrary JSON object describing the sampler
/// configuration used to produce `generated_code`. Its shape is intentionally
/// open-ended — see *Notes & Considerations*.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CritPtSubmission {
    pub problem_id: String,
    pub generated_code: String,
    pub model: String,
    pub generation_config: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<CritPtMessage>>,
}

/// Request body for the `EvaluateCritPt` endpoint.
///
/// `batch_metadata` is an arbitrary JSON object that callers may populate with
/// run identifiers, environment info, or be left as `{}`. It is skipped during
/// serialization when null so an empty object on the wire is also valid.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct CritPtEvaluateBody {
    pub submissions: Vec<CritPtSubmission>,
    #[serde(skip_serializing_if = "serde_json::Value::is_null", default)]
    pub batch_metadata: serde_json::Value,
}

/// Response body for the `EvaluateCritPt` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CritPtEvaluateResponse {
    pub accuracy: f64,
    pub timeout_rate: f64,
    pub server_timeout_count: i32,
    pub judge_error_count: i32,
}

/// 429 rate-limit error body.
///
/// **Documentation only.** This struct is intentionally **not** registered in
/// `openapi_registry()` because no endpoint currently declares it as its
/// `ApiResponse`. It exists to document the wire shape callers should expect
/// when a 429 is returned (alongside the `Retry-After` header).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RateLimitError {
    pub error: String,
    pub limit: i32,
    pub remaining: i32,
    pub reset: String,
    #[serde(rename = "retryAfter")]
    pub retry_after: i32,
}
```

> **Note on `Default`.** `CritPtEvaluateBody` derives `Default` to satisfy the
> generator's body-type convention. Other body-only types are not currently used —
> the data API has no request bodies.

## API Definition (`mod.rs`)

```rust
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
    ApiRequest, ApiResponse, AuthStrategy, Endpoint, EnvList, EnvMapping, RestApi, RestMethod,
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
        description:
            "Artificial Analysis free data API: LLM and media-model benchmark catalogs."
                .to_string(),
        base_url: "https://artificialanalysis.ai/api/v2".to_string(),
        docs_url: Some("https://artificialanalysis.ai/api-reference".to_string()),
        auth: AuthStrategy::ApiKey {
            header: "x-api-key".to_string(),
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
                description:
                    "Returns primary metrics from independent LLM benchmarks: \
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
            api_key: Some(EnvList::single("ARTIFICIAL_ANALYSIS_API_KEY")),
            ..Default::default()
        }),
    }
}

/// The CritPt benchmark evaluation API: a single POST endpoint.
pub fn define_artificial_analysis_critpt_api() -> RestApi {
    RestApi {
        name: "ArtificialAnalysisCritPt".to_string(),
        description:
            "Artificial Analysis CritPt benchmark: submit code-generation results for \
             evaluation."
                .to_string(),
        base_url: "https://artificialanalysis.ai/api/v2".to_string(),
        docs_url: Some("https://artificialanalysis.ai/api-reference".to_string()),
        auth: AuthStrategy::ApiKey {
            header: "x-api-key".to_string(),
        },
        auth_policy: None,
        env_auth: vec!["ARTIFICIAL_ANALYSIS_API_KEY".to_string()],
        env_username: None,
        headers: vec![],
        endpoints: vec![Endpoint {
            id: "EvaluateCritPt".to_string(),
            method: RestMethod::Post,
            path: "/critpt/evaluate".to_string(),
            description:
                "Submit a batch of code-generation submissions for evaluation against \
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
            api_key: Some(EnvList::single("ARTIFICIAL_ANALYSIS_API_KEY")),
            ..Default::default()
        }),
    }
}
```

## Wiring Required

The following files **must** be edited alongside adding the new module. Earlier drafts
of this spec missed these — every existing definition has the same plumbing, and
omitting any of it produces a working module that nothing actually consumes.

### `schematic/definitions/src/lib.rs`

1. Add the module declaration alongside the others:

   ```rust
   pub mod artificial_analysis;
   ```

2. Add the re-exports next to the other `pub use` lines:

   ```rust
   pub use artificial_analysis::{
       define_artificial_analysis_critpt_api, define_artificial_analysis_data_api,
   };
   ```

3. Extend the `apis_by_module()` `all_apis` vector with both new APIs:

   ```rust
   define_artificial_analysis_data_api(),
   define_artificial_analysis_critpt_api(),
   ```

   Both will group under the `"artificial_analysis"` key (their shared `module_path`),
   matching how `OllamaNative`/`OllamaOpenAI` group under `"ollama"` and
   `EmqxBasic`/`EmqxBearer` group under `"emqx"`.

4. Add a doc-comment example to the crate-level `//!` block alongside the other
   per-API examples:

   ```rust
   //! ```
   //! use schematic_definitions::artificial_analysis::{
   //!     define_artificial_analysis_critpt_api, define_artificial_analysis_data_api,
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
   ```

5. Add a row to the `Available APIs` table in `schematic/definitions/README.md`.

### `schematic/definitions/src/registry.rs`

Both lookup tables in this file must be extended.

1. `get_registry()` — add two arms that resolve to the same `openapi_registry()`
   function (mirrors the `"emqx-basic" | "emqx-bearer"` pattern):

   ```rust
   "artificial-analysis-data" | "artificial-analysis-critpt" => {
       Some(crate::artificial_analysis::openapi_registry())
   }
   ```

2. `registry_key_for()` — map both `RestApi::name` values to their kebab keys:

   ```rust
   "ArtificialAnalysisData" => "artificial-analysis-data".to_string(),
   "ArtificialAnalysisCritPt" => "artificial-analysis-critpt".to_string(),
   ```

3. Extend the `registry_key_for_known_apis_matches_table` test array with both new
   `(api_name, expected_key)` pairs.

### `schematic/definitions/src/prelude.rs`

Match the `lib.rs`-style prelude convention (define functions first, then key types).
Look at the existing prelude — it currently re-exports `define_*_api` functions for
GitHub/Gitea/Bitbucket/OpenAI/UnfoldedCircle and a curated subset of types. For this
provider, re-export both define functions plus the top-level response types most
callers will reach for:

```rust
// API definition functions
pub use crate::artificial_analysis::{
    define_artificial_analysis_critpt_api, define_artificial_analysis_data_api,
};

// Response types for Artificial Analysis (top-level only)
pub use crate::artificial_analysis::{
    CritPtEvaluateBody, CritPtEvaluateResponse, LlmModelsResponse, MediaModelsResponse,
};
```

If any of these names collide with already-re-exported types, prefix them
`ArtificialAnalysis*` exactly the way `bitbucket` types are prefixed `Bitbucket*`.

### `schematic/definitions/Cargo.toml`

If `serde_json` is not already a dependency of `schematic-definitions`, add it (it
will be pulled in transitively by `schemars` already; verify before adding).

## Tests

Add a `#[cfg(test)] mod tests` block at the bottom of `mod.rs` mirroring
`lmstudio/mod.rs` and `openai/mod.rs`. Skeleton:

```rust
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
            AuthStrategy::ApiKey { header } => assert_eq!(header, "x-api-key"),
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
        for id in ["ListTextToImageModels", "ListTextToVideoModels", "ListImageToVideoModels"] {
            let ep = endpoint(id);
            let params = ep.params.as_ref().expect(id);
            assert!(params.query.iter().any(|p| p.name == "include_categories"));
        }
        // Endpoints that should have no params.
        for id in ["ListLlmModels", "ListImageEditingModels", "ListTextToSpeechModels"] {
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
```

## Notes & Considerations

### Rate limiting

- **Data API:** 1,000 requests/day.
- **CritPt API:** 10 requests / 24-hour window.
- 429 responses include `Retry-After` plus a JSON body matching `RateLimitError`.
  This crate documents the body shape but does **not** type it as an `ApiResponse`,
  because the generator currently models a single happy-path response per endpoint.
  If multi-status response modeling is added later, register `RateLimitError` and
  attach it where appropriate.

### Data stability

- `id` fields on `LlmModel`, `MediaModel`, and `ModelCreator` are stable.
- `name` and `slug` may drift over time. Consumers should key by `id`.

### Prompt options

- `LlmModelsResponse.prompt_options.prompt_length` defaults to `"medium"` (~1k input
  tokens) for speed/latency benchmarks. The data API does not currently expose a way
  to request other prompt lengths via query params; if upstream adds that, extend
  `ListLlmModels` with a `prompt_length` query param using
  `EndpointParams::default().with_query_param("prompt_length", QueryParamType::String, ...)`.

### CritPt evaluation requirements

- All submissions for the public problem set must be included in each request.
- Requests may take substantial wall-clock time.
- `generation_config` and `batch_metadata` are typed `serde_json::Value`. This is a
  new precedent in this codebase — every other definition uses concrete struct types.
  Future maintainers may choose to tighten these once the upstream shape stabilizes.

### Numeric type fidelity

- `status`, `elo`, `rank`, `appearances`, `parallel_queries`, `server_timeout_count`,
  `judge_error_count`, and `retry_after` are typed as `i32`. Several of these are
  conceptually unsigned; switching to `u32` (or `u64` for counts) is a possible future
  refinement but is out of scope here. Keeping `i32` matches the existing draft and
  avoids gratuitous churn.

### Future extensions

- **Pagination:** the data endpoints are not currently paginated. If upstream adds
  pagination, model it via `EndpointParams::default().with_pagination(...)`.
- **OpenAPI export:** once both APIs are wired, `schematic-gen` should be able to
  export merged schemas via `get_registries_for_module("artificial_analysis")`.

## Verification Checklist

After implementing, run these in order. Stop and fix at the first failure.

```bash
# 1. Generate the typed client.
cargo run -p schematic-gen -- \
    --api artificial-analysis-data \
    --output schematic/schema/src
cargo run -p schematic-gen -- \
    --api artificial-analysis-critpt \
    --output schematic/schema/src

# 2. Verify the generated module exists at the expected path.
test -f schematic/schema/src/artificial_analysis.rs \
  || ls schematic/schema/src/artificial_analysis/

# 3. Confirm response-method selection looks right (no spurious request_bytes etc.).
grep -n "request_bytes\|request_text\|request_empty" \
    schematic/schema/src/artificial_analysis*.rs || true

# 4. Compile schema crate.
cargo check --manifest-path schematic/schema/Cargo.toml

# 5. Run definitions tests (includes the new tests module above).
cargo test -p schematic-definitions artificial_analysis

# 6. Sanity-check the registry plumbing.
cargo test -p schematic-definitions registry::tests::registry_key_for_known_apis_matches_table
cargo test -p schematic-definitions registry::tests::get_registries_for_module
```

A green run of step 5 plus a successful compile in step 4 is the definition of done
for the definitions side; downstream code-gen consumers must additionally pass their
own `cargo check`.

## Attribution

As required by the API terms, all usage must include attribution to
<https://artificialanalysis.ai/>. This requirement is repeated inside the module-level
`//!` doc comment in `mod.rs` so it surfaces in generated rustdoc.
