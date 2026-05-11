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
    /// The style category name.
    pub style_category: String,
    /// The subject matter category name.
    pub subject_matter_category: String,
    /// The format category name, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format_category: Option<String>,
    /// Elo rating for this category.
    pub elo: i32,
    /// 95% confidence interval.
    pub ci95: String,
    /// Number of appearances in this category.
    pub appearances: i32,
}

/// Benchmark scores for a single LLM model.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmEvaluations {
    /// Overall intelligence index.
    pub artificial_analysis_intelligence_index: f64,
    /// Coding benchmark index.
    pub artificial_analysis_coding_index: f64,
    /// Math benchmark index.
    pub artificial_analysis_math_index: f64,
    /// MMLU-Pro score.
    pub mmlu_pro: f64,
    /// GPQA score.
    pub gpqa: f64,
    /// HLE score.
    pub hle: f64,
    /// LiveCodeBench score.
    pub livecodebench: f64,
    /// SciCode score.
    pub scicode: f64,
    /// MATH-500 score.
    pub math_500: f64,
    /// AIME score.
    pub aime: f64,
}

/// Pricing for an LLM model in USD per 1M tokens.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmPricing {
    /// Blended price (3:1 input/output ratio) per 1M tokens.
    pub price_1m_blended_3_to_1: f64,
    /// Input token price per 1M tokens.
    pub price_1m_input_tokens: f64,
    /// Output token price per 1M tokens.
    pub price_1m_output_tokens: f64,
}

/// A single LLM model entry returned by `ListLlmModels`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmModel {
    /// Stable identifier for the model.
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// URL-safe slug (may change over time — prefer `id`).
    pub slug: String,
    /// The creator of this model.
    pub model_creator: ModelCreator,
    /// Benchmark evaluation scores.
    pub evaluations: LlmEvaluations,
    /// Pricing information.
    pub pricing: LlmPricing,
    /// Median output tokens per second.
    pub median_output_tokens_per_second: f64,
    /// Median time to first token in seconds.
    pub median_time_to_first_token_seconds: f64,
}

/// Configuration metadata describing how the LLM benchmarks were collected.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PromptOptions {
    /// Number of parallel queries used during benchmarking.
    pub parallel_queries: i32,
    /// Length of prompts used (`"short"`, `"medium"`, etc.).
    pub prompt_length: String,
}

/// Response body for the `ListLlmModels` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmModelsResponse {
    /// HTTP status code echoed by the API.
    pub status: i32,
    /// Prompt configuration used for the benchmark.
    pub prompt_options: PromptOptions,
    /// List of LLM models with their benchmarks.
    pub data: Vec<LlmModel>,
}

/// A single media-generation model entry. Used by all `/data/media/*` endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MediaModel {
    /// Stable identifier for the model.
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// URL-safe slug (may change over time — prefer `id`).
    pub slug: String,
    /// The creator of this model.
    pub model_creator: ModelCreator,
    /// Elo rating.
    pub elo: i32,
    /// Rank position.
    pub rank: i32,
    /// 95% confidence interval.
    pub ci95: String,
    /// Number of appearances.
    pub appearances: i32,
    /// Release date, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    /// Per-category breakdown, when requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<CategoryBreakdown>>,
}

/// Shared response shape for all `/data/media/*` endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MediaModelsResponse {
    /// HTTP status code echoed by the API.
    pub status: i32,
    /// Whether category breakdown was included.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_categories: Option<bool>,
    /// List of media models with their Elo ratings.
    pub data: Vec<MediaModel>,
}

/// A single message in the prompt history sent to the model under test.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CritPtMessage {
    /// Role of the message sender (e.g., `"user"`, `"assistant"`).
    pub role: String,
    /// Message content.
    pub content: String,
}

/// One submission in a CritPt evaluation batch.
///
/// `generation_config` is an arbitrary JSON object describing the sampler
/// configuration used to produce `generated_code`. Its shape is intentionally
/// open-ended.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CritPtSubmission {
    /// Identifier of the problem being evaluated.
    pub problem_id: String,
    /// The generated code submission.
    pub generated_code: String,
    /// Name of the model used.
    pub model: String,
    /// Sampler/generation configuration as an arbitrary JSON object.
    pub generation_config: serde_json::Value,
    /// Optional prompt history.
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
    /// List of code-generation submissions to evaluate.
    pub submissions: Vec<CritPtSubmission>,
    /// Optional metadata about the evaluation batch.
    #[serde(skip_serializing_if = "serde_json::Value::is_null", default)]
    pub batch_metadata: serde_json::Value,
}

/// Response body for the `EvaluateCritPt` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CritPtEvaluateResponse {
    /// Overall accuracy score.
    pub accuracy: f64,
    /// Rate of timeout responses.
    pub timeout_rate: f64,
    /// Number of server-side timeouts.
    pub server_timeout_count: i32,
    /// Number of judge evaluation errors.
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
    /// Error message.
    pub error: String,
    /// Request limit.
    pub limit: i32,
    /// Remaining requests.
    pub remaining: i32,
    /// Reset timestamp.
    pub reset: String,
    /// Seconds until the rate limit resets.
    #[serde(rename = "retryAfter")]
    pub retry_after: i32,
}
