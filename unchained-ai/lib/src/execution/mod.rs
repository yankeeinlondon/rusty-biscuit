use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use crate::models::model_default_parameters::ModelDefaultParameters;
use crate::models::model_metadata::ProviderModelMetadata;
use crate::rigging::providers::client_adaptors::{zai, zenmux};
use crate::rigging::providers::models::ProviderModel;
use crate::rigging::providers::provider_errors::ProviderError;
use crate::rigging::providers::Provider;
use rig::client::CompletionClient;
use rig::completion::{CompletionModel, Prompt};

/// A request to execute a single-turn completion.
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    /// The concrete model to route the request to.
    pub model: ProviderModel,

    /// Optional system prompt / preamble.
    pub system_prompt: Option<String>,

    /// The user prompt text.
    pub prompt: String,

    /// Optional JSON Schema for structured output. When set, the model is
    /// instructed to return JSON matching the schema and the raw parsed
    /// `serde_json::Value` is returned without validation.
    pub schema: Option<Value>,

    /// Optional generation parameters. When omitted, defaults are derived from
    /// the model's metadata.
    pub parameters: Option<ResolvedParameters>,
}

/// The result of a successful completion.
#[derive(Debug, Clone)]
pub enum CompletionOutput {
    /// Plain prose response.
    Text(String),

    /// Raw structured response parsed from the model's text output.
    /// Schema validation is the caller's responsibility.
    Structured(Value),
}

/// Generation parameters resolved from model metadata defaults and caller
/// overrides.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResolvedParameters {
    /// Sampling temperature.
    pub temperature: Option<f32>,

    /// Nucleus sampling probability mass.
    pub top_p: Option<f32>,

    /// Top-k sampling limit.
    pub top_k: Option<u32>,

    /// Frequency penalty.
    pub frequency_penalty: Option<f32>,

    /// Presence penalty.
    pub presence_penalty: Option<f32>,

    /// Maximum tokens to generate.
    pub max_tokens: Option<u64>,

    /// Provider-specific parameters that the execution surface does not
    /// interpret itself (e.g. OpenAI `reasoning_effort`, Anthropic `thinking`).
    /// Values are passed through to the backend unchanged.
    pub extra: Option<serde_json::Map<String, serde_json::Value>>,
}

impl ResolvedParameters {
    /// Creates an empty parameters instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Seeds parameters from the model metadata defaults, if any.
    #[must_use]
    pub fn from_metadata(metadata: &ProviderModelMetadata) -> Self {
        metadata.default_parameters.as_ref().map_or_else(
            Self::new,
            |defaults| Self {
                temperature: defaults.temperature,
                top_p: defaults.top_p,
                top_k: defaults.top_k,
                frequency_penalty: defaults.frequency_penalty,
                presence_penalty: defaults.presence_penalty,
                max_tokens: None,
                extra: None,
            },
        )
    }

    /// Applies caller overrides on top of these defaults. `None` values in
    /// `overrides` leave the existing value unchanged.
    pub fn apply_override(&mut self, overrides: &Self) {
        if overrides.temperature.is_some() {
            self.temperature = overrides.temperature;
        }
        if overrides.top_p.is_some() {
            self.top_p = overrides.top_p;
        }
        if overrides.top_k.is_some() {
            self.top_k = overrides.top_k;
        }
        if overrides.frequency_penalty.is_some() {
            self.frequency_penalty = overrides.frequency_penalty;
        }
        if overrides.presence_penalty.is_some() {
            self.presence_penalty = overrides.presence_penalty;
        }
        if overrides.max_tokens.is_some() {
            self.max_tokens = overrides.max_tokens;
        }
        if overrides.extra.is_some() {
            self.extra = overrides.extra.clone();
        }
    }
}

impl From<&ModelDefaultParameters> for ResolvedParameters {
    fn from(value: &ModelDefaultParameters) -> Self {
        Self {
            temperature: value.temperature,
            top_p: value.top_p,
            top_k: value.top_k,
            frequency_penalty: value.frequency_penalty,
            presence_penalty: value.presence_penalty,
            max_tokens: None,
            extra: None,
        }
    }
}

/// Request passed to a [`CompletionBackend`].
#[derive(Debug, Clone)]
pub struct BackendRequest {
    /// Optional system prompt / preamble.
    pub system_prompt: Option<String>,

    /// The user prompt text (already augmented with any structured-output
    /// instructions).
    pub user_prompt: String,

    /// Generation parameters to apply.
    pub parameters: ResolvedParameters,
}

/// A seam for executing raw text completions. Production code and tests each
/// provide an implementation so the higher-level [`complete`] logic is
/// unit-testable without a network.
#[async_trait]
pub trait CompletionBackend: Send + Sync {
    /// Execute a single-turn text completion.
    async fn complete_text(&self, request: BackendRequest) -> Result<String, ProviderError>;
}

/// Executes a [`CompletionRequest`] through the provided backend.
///
/// For prose requests, the model's text output is returned as
/// [`CompletionOutput::Text`]. For structured requests, the prompt is
/// augmented with a JSON-only instruction, the model's text output is parsed
/// into a raw [`serde_json::Value`], and returned as
/// [`CompletionOutput::Structured`]. No schema validation is performed here.
pub async fn complete(
    backend: &dyn CompletionBackend,
    request: CompletionRequest,
) -> Result<CompletionOutput, ProviderError> {
    let parameters = resolve_parameters(&request);
    let user_prompt = build_user_prompt(&request);

    let backend_request = BackendRequest {
        system_prompt: request.system_prompt,
        user_prompt,
        parameters,
    };

    let text = backend.complete_text(backend_request).await?;

    if request.schema.is_some() {
        let value = extract_json(&text)?;
        Ok(CompletionOutput::Structured(value))
    } else {
        Ok(CompletionOutput::Text(text))
    }
}

fn resolve_parameters(request: &CompletionRequest) -> ResolvedParameters {
    let mut parameters = request
        .model
        .metadata()
        .map_or_else(ResolvedParameters::new, ResolvedParameters::from_metadata);

    if let Some(overrides) = &request.parameters {
        parameters.apply_override(overrides);
    }

    parameters
}

fn build_user_prompt(request: &CompletionRequest) -> String {
    match &request.schema {
        Some(schema) => format!(
            "{}\n\nRespond with a single JSON object that conforms to the following schema. Do not include any prose, markdown formatting, or explanatory text outside the JSON object.\n\n{}",
            request.prompt,
            serde_json::to_string_pretty(schema).unwrap_or_default()
        ),
        None => request.prompt.clone(),
    }
}

fn extract_json(text: &str) -> Result<Value, ProviderError> {
    let trimmed = text.trim();

    // If the response is wrapped in a markdown code fence, strip it.
    let cleaned = if let Some(inner) = trimmed.strip_prefix("```json") {
        inner
            .trim_end()
            .strip_suffix("```")
            .unwrap_or(inner)
            .trim()
    } else if let Some(inner) = trimmed.strip_prefix("```") {
        inner
            .trim_end()
            .strip_suffix("```")
            .unwrap_or(inner)
            .trim()
    } else {
        trimmed
    };

    serde_json::from_str(cleaned).map_err(|e| ProviderError::ExecutionFailed {
        provider: "execution".to_string(),
        reason: format!("structured response was not valid JSON: {e}"),
    })
}

/// Production backend that builds a rig-core client from a [`ProviderModel`]
/// and executes the completion through it.
pub struct RigCompletionBackend {
    model: ProviderModel,
}

impl RigCompletionBackend {
    /// Creates a backend for the given model, resolving API credentials from
    /// the process environment.
    ///
    /// Returns [`ProviderError::MissingApiKey`] when the provider requires an
    /// API key and none of its configured environment variables are set or
    /// non-empty.
    pub fn new(model: ProviderModel) -> Result<Self, ProviderError> {
        // Validate credentials eagerly so callers get a clear error before any
        // async work.
        let _ = resolve_api_key(&model)?;
        Ok(Self { model })
    }
}

#[async_trait]
impl CompletionBackend for RigCompletionBackend {
    async fn complete_text(&self, request: BackendRequest) -> Result<String, ProviderError> {
        let model_id = self.model.model_id().to_string();
        let provider = self.model.provider();

        match provider {
            Provider::Anthropic => {
                let key = resolve_api_key(&self.model)?;
                let client: rig::providers::anthropic::Client =
                    rig::providers::anthropic::Client::new(key).map_err(|e| {
                        ProviderError::ClientBuildFailed {
                            provider: provider.display_name().to_string(),
                            reason: e.to_string(),
                        }
                    })?;
                prompt_with_agent_builder(client.agent(&model_id), request).await
            }
            Provider::OpenAi => {
                let key = resolve_api_key(&self.model)?;
                let client: rig::providers::openai::Client =
                    rig::providers::openai::Client::new(key).map_err(|e| {
                        ProviderError::ClientBuildFailed {
                            provider: provider.display_name().to_string(),
                            reason: e.to_string(),
                        }
                    })?;
                prompt_with_agent_builder(client.agent(&model_id), request).await
            }
            Provider::Gemini => {
                let key = resolve_api_key(&self.model)?;
                let client: rig::providers::gemini::Client =
                    rig::providers::gemini::Client::new(key).map_err(|e| {
                        ProviderError::ClientBuildFailed {
                            provider: provider.display_name().to_string(),
                            reason: e.to_string(),
                        }
                    })?;
                prompt_with_agent_builder(client.agent(&model_id), request).await
            }
            Provider::Groq => {
                let key = resolve_api_key(&self.model)?;
                let client: rig::providers::groq::Client =
                    rig::providers::groq::Client::new(key).map_err(|e| {
                        ProviderError::ClientBuildFailed {
                            provider: provider.display_name().to_string(),
                            reason: e.to_string(),
                        }
                    })?;
                prompt_with_agent_builder(client.agent(&model_id), request).await
            }
            Provider::Mistral => {
                let key = resolve_api_key(&self.model)?;
                let client: rig::providers::mistral::Client =
                    rig::providers::mistral::Client::new(key).map_err(|e| {
                        ProviderError::ClientBuildFailed {
                            provider: provider.display_name().to_string(),
                            reason: e.to_string(),
                        }
                    })?;
                prompt_with_agent_builder(client.agent(&model_id), request).await
            }
            Provider::Deepseek => {
                let key = resolve_api_key(&self.model)?;
                let client: rig::providers::deepseek::Client =
                    rig::providers::deepseek::Client::new(key).map_err(|e| {
                        ProviderError::ClientBuildFailed {
                            provider: provider.display_name().to_string(),
                            reason: e.to_string(),
                        }
                    })?;
                prompt_with_agent_builder(client.agent(&model_id), request).await
            }
            Provider::Xai => {
                let key = resolve_api_key(&self.model)?;
                let client: rig::providers::xai::Client =
                    rig::providers::xai::Client::new(key).map_err(|e| {
                        ProviderError::ClientBuildFailed {
                            provider: provider.display_name().to_string(),
                            reason: e.to_string(),
                        }
                    })?;
                prompt_with_agent_builder(client.agent(&model_id), request).await
            }
            Provider::MoonshotAi => {
                let key = resolve_api_key(&self.model)?;
                let client: rig::providers::moonshot::Client =
                    rig::providers::moonshot::Client::new(key).map_err(|e| {
                        ProviderError::ClientBuildFailed {
                            provider: provider.display_name().to_string(),
                            reason: e.to_string(),
                        }
                    })?;
                prompt_with_agent_builder(client.agent(&model_id), request).await
            }
            Provider::OpenRouter => {
                let key = resolve_api_key(&self.model)?;
                let client: rig::providers::openrouter::Client =
                    rig::providers::openrouter::Client::new(key).map_err(|e| {
                        ProviderError::ClientBuildFailed {
                            provider: provider.display_name().to_string(),
                            reason: e.to_string(),
                        }
                    })?;
                prompt_with_agent_builder(client.agent(&model_id), request).await
            }
            Provider::Ollama => {
                let client: rig::providers::ollama::Client =
                    rig::providers::ollama::Client::new(rig::client::Nothing).map_err(|e| {
                        ProviderError::ClientBuildFailed {
                            provider: provider.display_name().to_string(),
                            reason: e.to_string(),
                        }
                    })?;
                prompt_with_agent_builder(client.agent(&model_id), request).await
            }
            Provider::Zai => {
                let client = zai::Client::from_env()?;
                prompt_with_agent_builder(client.agent(&model_id), request).await
            }
            Provider::ZenMux => {
                let client = zenmux::Client::from_env()?;
                prompt_with_agent_builder(client.agent(&model_id), request).await
            }
            Provider::HuggingFace => Err(ProviderError::ExecutionFailed {
                provider: provider.display_name().to_string(),
                reason: "HuggingFace is not supported for text completion".to_string(),
            }),
        }
    }
}

async fn prompt_with_agent_builder<M>(
    mut builder: rig::agent::AgentBuilder<M>,
    request: BackendRequest,
) -> Result<String, ProviderError>
where
    M: CompletionModel,
{

    if let Some(system) = request.system_prompt {
        builder = builder.preamble(&system);
    }
    if let Some(temperature) = request.parameters.temperature {
        builder = builder.temperature(f64::from(temperature));
    }
    if let Some(max_tokens) = request.parameters.max_tokens {
        builder = builder.max_tokens(max_tokens);
    }

    let agent = builder.build();
    agent
        .prompt(request.user_prompt)
        .await
        .map_err(|e| ProviderError::ExecutionFailed {
            provider: "LLM".to_string(),
            reason: e.to_string(),
        })
}

fn resolve_api_key(model: &ProviderModel) -> Result<String, ProviderError> {
    let provider = model.provider();
    let config = provider.config();

    if config.is_local {
        return Ok(String::new());
    }

    for var in config.env_vars {
        if let Ok(value) = std::env::var(var) && !value.trim().is_empty() {
            return Ok(value);
        }
    }

    Err(ProviderError::MissingApiKey {
        provider: provider.display_name().to_string(),
        env_vars: config.env_vars.iter().map(ToString::to_string).collect(),
    })
}

/// Factory that creates a [`CompletionBackend`] for a given model.
pub type BackendFactory =
    dyn Fn(&ProviderModel) -> Result<Arc<dyn CompletionBackend>, ProviderError> + Send + Sync;

static TEST_BACKEND_FACTORY: std::sync::Mutex<Option<Arc<BackendFactory>>> =
    std::sync::Mutex::new(None);

fn backend_for(model: &ProviderModel) -> Result<Arc<dyn CompletionBackend>, ProviderError> {
    if let Some(factory) = TEST_BACKEND_FACTORY
        .lock()
        .expect("backend factory lock poisoned")
        .as_ref()
    {
        return factory(model);
    }

    Ok(Arc::new(RigCompletionBackend::new(model.clone())?))
}

/// Executes a [`CompletionRequest`] synchronously by running the async
/// [`complete`] path on a dedicated current-thread Tokio runtime.
///
/// ## Runtime discipline
///
/// This function is safe to call both from outside any async runtime and from
/// inside an already-running multi-thread Tokio runtime. It detects the current
/// runtime with [`tokio::runtime::Handle::try_current`]. When already inside a
/// runtime, it spawns a dedicated worker thread and runs a fresh current-thread
/// runtime there, avoiding the panic that would occur from calling
/// [`Handle::block_on`][tokio::runtime::Handle::block_on] recursively.
///
/// Production code uses a [`RigCompletionBackend`] built from the request's
/// [`ProviderModel`]. Tests may install a fake backend via
/// [`install_test_backend`].
pub fn complete_blocking(request: CompletionRequest) -> Result<CompletionOutput, ProviderError> {
    let backend = backend_for(&request.model)?;

    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::scope(|scope| {
            let handle = scope.spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to build current-thread runtime");
                runtime.block_on(complete(backend.as_ref(), request))
            });

            match handle.join() {
                Ok(result) => result,
                Err(_) => Err(ProviderError::ExecutionFailed {
                    provider: "execution".to_string(),
                    reason: "completion worker thread panicked".to_string(),
                }),
            }
        })
    } else {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build current-thread runtime");
        runtime.block_on(complete(backend.as_ref(), request))
    }
}

/// Reference implementation of [`CompletionBackend`] for tests.
#[derive(Debug, Default, Clone)]
pub struct FakeCompletionBackend {
    response: Arc<std::sync::Mutex<String>>,
}

impl FakeCompletionBackend {
    /// Creates a fake backend that always returns the given text.
    #[must_use]
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            response: Arc::new(std::sync::Mutex::new(response.into())),
        }
    }

    /// Replaces the response the fake backend will return.
    pub fn set_response(&self, response: impl Into<String>) {
        let mut guard = self.response.lock().expect("fake backend lock poisoned");
        *guard = response.into();
    }
}

#[async_trait]
impl CompletionBackend for FakeCompletionBackend {
    async fn complete_text(&self, _request: BackendRequest) -> Result<String, ProviderError> {
        let guard = self.response.lock().expect("fake backend lock poisoned");
        Ok(guard.clone())
    }
}

/// Installs a factory used by [`complete_blocking`] to create backends in
/// tests. Production code never calls this function.
#[cfg(test)]
pub fn install_test_backend_factory(factory: Arc<BackendFactory>) {
    *TEST_BACKEND_FACTORY
        .lock()
        .expect("backend factory lock poisoned") = Some(factory);
}

/// Installs a single fake backend for all models in tests.
#[cfg(test)]
pub fn install_test_backend(backend: Arc<dyn CompletionBackend>) {
    install_test_backend_factory(Arc::new(move |_model: &ProviderModel| Ok(backend.clone())));
}

/// Clears any backend factory installed by [`install_test_backend_factory`].
#[cfg(test)]
pub fn clear_test_backend_factory() {
    *TEST_BACKEND_FACTORY.lock().expect("backend factory lock poisoned") = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rigging::providers::models::openai::ProviderModelOpenAi;

    #[tokio::test]
    async fn test_complete_prose_returns_text() {
        let backend = FakeCompletionBackend::new("hello world");
        let request = CompletionRequest {
            model: ProviderModel::OpenAi(ProviderModelOpenAi::O3),
            system_prompt: Some("You are helpful".to_string()),
            prompt: "say hi".to_string(),
            schema: None,
            parameters: None,
        };

        let result = complete(&backend, request).await.unwrap();
        assert!(matches!(result, CompletionOutput::Text(text) if text == "hello world"));
    }

    #[tokio::test]
    async fn test_complete_structured_returns_raw_json() {
        let backend = FakeCompletionBackend::new(r#"{"answer": 42}"#);
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "answer": { "type": "integer" }
            }
        });
        let request = CompletionRequest {
            model: ProviderModel::OpenAi(ProviderModelOpenAi::O3),
            system_prompt: None,
            prompt: "What is the answer?".to_string(),
            schema: Some(schema),
            parameters: None,
        };

        let result = complete(&backend, request).await.unwrap();
        match result {
            CompletionOutput::Structured(value) => {
                assert_eq!(value["answer"], 42);
            }
            _ => panic!("expected structured output"),
        }
    }

    #[tokio::test]
    async fn test_complete_structured_strips_markdown_fence() {
        let backend = FakeCompletionBackend::new("```json\n{\"answer\": 42}\n```");
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "answer": { "type": "integer" }
            }
        });
        let request = CompletionRequest {
            model: ProviderModel::OpenAi(ProviderModelOpenAi::O3),
            system_prompt: None,
            prompt: "What is the answer?".to_string(),
            schema: Some(schema),
            parameters: None,
        };

        let result = complete(&backend, request).await.unwrap();
        match result {
            CompletionOutput::Structured(value) => {
                assert_eq!(value["answer"], 42);
            }
            _ => panic!("expected structured output"),
        }
    }

    #[tokio::test]
    async fn test_complete_structured_invalid_json_returns_error() {
        let backend = FakeCompletionBackend::new("not json");
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "answer": { "type": "integer" }
            }
        });
        let request = CompletionRequest {
            model: ProviderModel::OpenAi(ProviderModelOpenAi::O3),
            system_prompt: None,
            prompt: "What is the answer?".to_string(),
            schema: Some(schema),
            parameters: None,
        };

        let result = complete(&backend, request).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_resolved_parameters_apply_override() {
        let mut params = ResolvedParameters {
            temperature: Some(0.7),
            max_tokens: Some(100),
            ..ResolvedParameters::default()
        };
        let overrides = ResolvedParameters {
            temperature: Some(0.2),
            top_p: Some(0.9),
            ..ResolvedParameters::default()
        };
        params.apply_override(&overrides);

        assert_eq!(params.temperature, Some(0.2));
        assert_eq!(params.top_p, Some(0.9));
        assert_eq!(params.max_tokens, Some(100));
    }

    #[test]
    #[serial_test::serial]
    fn test_complete_blocking_prose_outside_runtime() {
        install_test_backend(Arc::new(FakeCompletionBackend::new("blocking prose")));
        let request = CompletionRequest {
            model: ProviderModel::OpenAi(ProviderModelOpenAi::O3),
            system_prompt: None,
            prompt: "say hi".to_string(),
            schema: None,
            parameters: None,
        };

        let result = complete_blocking(request);
        clear_test_backend_factory();

        assert!(matches!(result, Ok(CompletionOutput::Text(text)) if text == "blocking prose"));
    }

    #[test]
    #[serial_test::serial]
    fn test_complete_blocking_structured_outside_runtime() {
        install_test_backend(Arc::new(FakeCompletionBackend::new(r#"{"value": 1}"#)));
        let request = CompletionRequest {
            model: ProviderModel::OpenAi(ProviderModelOpenAi::O3),
            system_prompt: None,
            prompt: "return json".to_string(),
            schema: Some(serde_json::json!({"type": "object"})),
            parameters: None,
        };

        let result = complete_blocking(request);
        clear_test_backend_factory();

        assert!(
            matches!(result, Ok(CompletionOutput::Structured(ref value)) if value["value"] == 1)
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_complete_blocking_inside_runtime_does_not_panic() {
        install_test_backend(Arc::new(FakeCompletionBackend::new("runtime safe")));
        let request = CompletionRequest {
            model: ProviderModel::OpenAi(ProviderModelOpenAi::O3),
            system_prompt: None,
            prompt: "say hi".to_string(),
            schema: None,
            parameters: None,
        };

        let result = complete_blocking(request);
        clear_test_backend_factory();

        assert!(matches!(result, Ok(CompletionOutput::Text(text)) if text == "runtime safe"));
    }
}
