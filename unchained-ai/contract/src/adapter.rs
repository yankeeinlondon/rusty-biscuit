//! [`UnchainedInferenceAdapter`]: the public [`InferenceAdapter`] implementation
//! backed by the `unchained-ai` provider/model abstraction.

use std::sync::Arc;

use async_trait::async_trait;
use biscuit_contract::inference::{
    InferenceAdapter, InferenceData, InferenceError, InferenceErrorKind, InferenceMetadata,
    InferenceOutput, InferenceRequest, InferenceResponse,
};
use unchained_ai::execution::{
    BackendFactory, CompletionBackend, CompletionOutput, CompletionRequest, RigCompletionBackend,
};
use unchained_ai::models::selection::{EnvView, StdEnvView};
use unchained_ai::rigging::providers::models::ProviderModel;

use crate::error::{classify_provider_error, inference_error};
use crate::profile::{parameters_for_reasoning, resolve_profile_model};
use crate::structured;

/// An [`InferenceAdapter`] that routes requests through the `unchained-ai`
/// execution surface and model resolver.
///
/// Construct with [`new`](Self::new), optionally pin a model with
/// [`with_model`](Self::with_model), and [`build`](Self::build) into an
/// `Arc<dyn InferenceAdapter>`.
///
/// ## Example
///
/// ```no_run
/// use std::sync::Arc;
/// use biscuit_contract::inference::InferenceAdapter;
/// use unchained_ai_contract::UnchainedInferenceAdapter;
///
/// let adapter: Arc<dyn InferenceAdapter> = UnchainedInferenceAdapter::new().build();
/// ```
#[derive(Clone)]
pub struct UnchainedInferenceAdapter {
    env_view: Arc<dyn EnvView>,
    backend_factory: Arc<BackendFactory>,
    pinned_model: Option<ProviderModel>,
}

impl UnchainedInferenceAdapter {
    /// Construct an adapter using the production environment and rig-backed
    /// completion backend.
    pub fn new() -> Self {
        Self {
            env_view: Arc::new(StdEnvView),
            backend_factory: Arc::new(default_backend_factory),
            pinned_model: None,
        }
    }

    /// Pin an explicit model, overriding profile-driven selection.
    #[must_use]
    pub fn with_model(mut self, model: ProviderModel) -> Self {
        self.pinned_model = Some(model);
        self
    }

    /// Finish construction as a shared trait object.
    #[must_use]
    pub fn build(self) -> Arc<dyn InferenceAdapter> {
        Arc::new(self)
    }

    /// Override the environment view (test seam).
    #[cfg(test)]
    pub(crate) fn with_env_view(mut self, env_view: Arc<dyn EnvView>) -> Self {
        self.env_view = env_view;
        self
    }

    /// Override the backend factory (test seam).
    #[cfg(test)]
    pub(crate) fn with_backend_factory(mut self, factory: Arc<BackendFactory>) -> Self {
        self.backend_factory = factory;
        self
    }
}

impl Default for UnchainedInferenceAdapter {
    fn default() -> Self {
        Self::new()
    }
}

fn default_backend_factory(
    model: &ProviderModel,
) -> Result<Arc<dyn CompletionBackend>, unchained_ai::rigging::providers::provider_errors::ProviderError> {
    Ok(Arc::new(RigCompletionBackend::new(model.clone())?))
}

#[async_trait]
impl InferenceAdapter for UnchainedInferenceAdapter {
    async fn infer(
        &self,
        request: InferenceRequest,
    ) -> Result<InferenceResponse, InferenceError> {
        if request.prompt.trim().is_empty() {
            return Err(inference_error(
                InferenceErrorKind::InvalidRequest,
                "prompt is empty",
            ));
        }

        let explicit_model = self.pinned_model.is_some();

        // Validate the schema up front so an invalid schema fails as
        // `InvalidRequest` before any backend is invoked. The schema is then
        // handed to the shared `unchained-ai` structured execution surface,
        // which owns prompt augmentation and JSON extraction; this adapter only
        // re-validates the extracted value against the same schema below. This
        // keeps the adapter and the native `Prompt` structured path on one
        // surface rather than two drifting copies.
        let (validator, schema) = match &request.output {
            InferenceOutput::Structured { schema } => {
                (Some(structured::compile_schema(schema)?), Some(schema.clone()))
            }
            InferenceOutput::Prose => (None, None),
        };

        let model = if let Some(pinned) = &self.pinned_model {
            pinned.clone()
        } else {
            resolve_profile_model(request.profile, self.env_view.as_ref())
                .map_err(|e| classify_provider_error(e, explicit_model))?
        };

        // Keep a clone for metadata reporting after the request consumes the
        // model.
        let model_for_metadata = model.clone();

        let backend = (self.backend_factory)(
            &model,
        )
        .map_err(|e| classify_provider_error(e, explicit_model))?;

        let mut parameters = parameters_for_reasoning(&model, request.profile.reasoning);
        if let Some(metadata) = model.metadata() {
            let mut defaults = unchained_ai::execution::ResolvedParameters::from_metadata(metadata);
            defaults.apply_override(&parameters);
            parameters = defaults;
        }

        let completion_request = CompletionRequest {
            model,
            system_prompt: None,
            prompt: request.prompt.clone(),
            schema,
            parameters: Some(parameters),
        };

        let output = unchained_ai::execution::complete(backend.as_ref(), completion_request)
            .await
            .map_err(|e| classify_provider_error(e, explicit_model))?;

        let model = model_for_metadata;

        let metadata = InferenceMetadata {
            provider: Some(model.provider().display_name().to_string()),
            model: Some(model.wire_id()),
            agent: None,
        };

        match (&request.output, output) {
            (InferenceOutput::Prose, CompletionOutput::Text(text)) => Ok(InferenceResponse {
                data: InferenceData::Prose(text),
                metadata,
            }),
            (InferenceOutput::Structured { .. }, CompletionOutput::Structured(value)) => {
                let validator = validator.expect("validator compiled for structured request");
                structured::validate_instance(&validator, &value)?;
                Ok(InferenceResponse {
                    data: InferenceData::Structured(value),
                    metadata,
                })
            }
            // The execution surface returns `Structured` exactly when a schema
            // was supplied and `Text` otherwise, so request shape and response
            // variant always agree. A mismatch means the surface broke that
            // contract; report it rather than silently coercing.
            (InferenceOutput::Prose, CompletionOutput::Structured(_))
            | (InferenceOutput::Structured { .. }, CompletionOutput::Text(_)) => {
                Err(inference_error(
                    InferenceErrorKind::InvalidResponse,
                    "execution surface returned a response variant that did not match the request",
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_contract::inference::{
        InferenceData, InferenceErrorKind, InferencePriority, InferenceProfile, ReasoningEffort,
    };
    use unchained_ai::execution::FakeCompletionBackend;
    use unchained_ai::models::selection::HashMapEnvView;
    use unchained_ai::rigging::providers::models::openai::ProviderModelOpenAi;

    use unchained_ai::execution::BackendRequest;
    use unchained_ai::rigging::providers::models::anthropic::ProviderModelAnthropic;
    use unchained_ai::rigging::providers::provider_errors::ProviderError;

    fn fake_backend_factory(
        backend: Arc<dyn CompletionBackend>,
    ) -> Arc<BackendFactory> {
        Arc::new(move |_model: &ProviderModel| Ok(backend.clone()))
    }

    /// Backend that records the [`BackendRequest`] it receives and replies with
    /// a fixed body, so tests can assert what the adapter actually handed to the
    /// shared execution surface.
    #[derive(Clone, Default)]
    struct RecordingBackend {
        last: Arc<std::sync::Mutex<Option<BackendRequest>>>,
        reply: String,
    }

    impl RecordingBackend {
        fn new(reply: impl Into<String>) -> Self {
            Self {
                last: Arc::new(std::sync::Mutex::new(None)),
                reply: reply.into(),
            }
        }

        fn recorded(&self) -> BackendRequest {
            self.last.lock().unwrap().clone().expect("request recorded")
        }
    }

    #[async_trait]
    impl CompletionBackend for RecordingBackend {
        async fn complete_text(&self, request: BackendRequest) -> Result<String, ProviderError> {
            *self.last.lock().unwrap() = Some(request);
            Ok(self.reply.clone())
        }
    }

    #[tokio::test]
    async fn structured_request_flows_through_lib_structured_path() {
        // A structured request must let the shared execution surface augment the
        // prompt (rather than the adapter pre-augmenting and passing schema:
        // None). Proven by the lib's schema instruction appearing in the prompt
        // the backend received.
        let backend = Arc::new(RecordingBackend::new(r#"{"answer": 42}"#));
        let adapter = UnchainedInferenceAdapter::new()
            .with_model(ProviderModel::Anthropic(
                ProviderModelAnthropic::Claude__Sonnet__4__5__20250929,
            ))
            .with_backend_factory(fake_backend_factory(backend.clone()));
        let request = InferenceRequest {
            prompt: "What is the answer?".to_string(),
            output: InferenceOutput::Structured {
                schema: serde_json::json!({
                    "type": "object",
                    "properties": { "answer": { "type": "integer" } },
                    "required": ["answer"]
                }),
            },
            profile: InferenceProfile::default(),
        };

        let response = adapter.infer(request).await.expect("structured should succeed");

        let recorded = backend.recorded();
        assert!(
            recorded.user_prompt.contains("conforms to the following schema"),
            "expected the lib structured augmentation in the prompt, got: {}",
            recorded.user_prompt
        );
        match response.data {
            InferenceData::Structured(value) => assert_eq!(value["answer"], 42),
            InferenceData::Prose(_) => panic!("expected structured data"),
        }
    }

    fn test_adapter(backend: Arc<dyn CompletionBackend>) -> UnchainedInferenceAdapter {
        UnchainedInferenceAdapter::new()
            .with_env_view(Arc::new(HashMapEnvView::new([("OPENAI_API_KEY", "sk-test")])))
            .with_backend_factory(fake_backend_factory(backend))
    }

    #[tokio::test]
    async fn prose_request_returns_text() {
        let adapter = test_adapter(Arc::new(FakeCompletionBackend::new("hello world"))).build();
        let request = InferenceRequest {
            prompt: "say hi".to_string(),
            output: InferenceOutput::Prose,
            profile: InferenceProfile::default(),
        };

        let response = adapter.infer(request).await.expect("prose should succeed");
        assert_eq!(
            response.data,
            InferenceData::Prose("hello world".to_string())
        );
        assert_eq!(response.metadata.provider.as_deref(), Some("OpenAI"));
        assert!(response.metadata.model.as_deref().unwrap().contains("openai/"));
    }

    #[tokio::test]
    async fn structured_request_validates_against_schema() {
        let adapter =
            test_adapter(Arc::new(FakeCompletionBackend::new(r#"{"answer": 42}"#))).build();
        let request = InferenceRequest {
            prompt: "What is the answer?".to_string(),
            output: InferenceOutput::Structured {
                schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "answer": { "type": "integer" }
                    },
                    "required": ["answer"]
                }),
            },
            profile: InferenceProfile::default(),
        };

        let response = adapter
            .infer(request)
            .await
            .expect("structured should succeed");
        match response.data {
            InferenceData::Structured(value) => assert_eq!(value["answer"], 42),
            InferenceData::Prose(_) => panic!("expected structured data"),
        }
    }

    #[tokio::test]
    async fn invalid_json_is_invalid_response() {
        let adapter = test_adapter(Arc::new(FakeCompletionBackend::new("not json"))).build();
        let request = InferenceRequest {
            prompt: "give me json".to_string(),
            output: InferenceOutput::Structured {
                schema: serde_json::json!({"type": "object"}),
            },
            profile: InferenceProfile::default(),
        };

        let err = adapter.infer(request).await.unwrap_err();
        assert_eq!(err.kind, InferenceErrorKind::InvalidResponse);
    }

    #[tokio::test]
    async fn schema_violation_is_invalid_response() {
        let adapter =
            test_adapter(Arc::new(FakeCompletionBackend::new(r#"{"answer": "wrong"}"#))).build();
        let request = InferenceRequest {
            prompt: "give me json".to_string(),
            output: InferenceOutput::Structured {
                schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "answer": { "type": "integer" }
                    }
                }),
            },
            profile: InferenceProfile::default(),
        };

        let err = adapter.infer(request).await.unwrap_err();
        assert_eq!(err.kind, InferenceErrorKind::InvalidResponse);
    }

    #[tokio::test]
    async fn empty_prompt_is_invalid_request() {
        let adapter = test_adapter(Arc::new(FakeCompletionBackend::new("ignored"))).build();
        let request = InferenceRequest {
            prompt: "   ".to_string(),
            output: InferenceOutput::Prose,
            profile: InferenceProfile::default(),
        };

        let err = adapter.infer(request).await.unwrap_err();
        assert_eq!(err.kind, InferenceErrorKind::InvalidRequest);
    }

    #[tokio::test]
    async fn with_model_bypasses_selection() {
        let model = ProviderModel::OpenAi(ProviderModelOpenAi::O3);
        let adapter = UnchainedInferenceAdapter::new()
            .with_model(model.clone())
            .with_backend_factory(fake_backend_factory(Arc::new(
                FakeCompletionBackend::new("pinned"),
            )));
        let request = InferenceRequest {
            prompt: "say hi".to_string(),
            output: InferenceOutput::Prose,
            profile: InferenceProfile::default(),
        };

        let response = adapter.infer(request).await.expect("prose should succeed");
        assert_eq!(
            response.data,
            InferenceData::Prose("pinned".to_string())
        );
        assert_eq!(response.metadata.model.as_deref(), Some("openai/o3"));
    }

    fn missing_key_backend_factory() -> Arc<BackendFactory> {
        Arc::new(|_model: &ProviderModel| {
            Err(unchained_ai::rigging::providers::provider_errors::ProviderError::MissingApiKey {
                provider: "openai".to_string(),
                env_vars: vec!["OPENAI_API_KEY".to_string()],
            })
        })
    }

    #[tokio::test]
    async fn pinned_model_missing_key_is_unauthorized() {
        let model = ProviderModel::OpenAi(ProviderModelOpenAi::O3);
        let adapter = UnchainedInferenceAdapter::new()
            .with_model(model.clone())
            .with_backend_factory(missing_key_backend_factory());
        let request = InferenceRequest {
            prompt: "say hi".to_string(),
            output: InferenceOutput::Prose,
            profile: InferenceProfile::default(),
        };

        let err = adapter.infer(request).await.unwrap_err();
        assert_eq!(err.kind, InferenceErrorKind::Unauthorized);
    }

    #[tokio::test]
    async fn object_safety_builds_arc_dyn_adapter() {
        let _: Arc<dyn InferenceAdapter> = UnchainedInferenceAdapter::new().build();
    }

    #[tokio::test]
    async fn profile_priority_quality_selects_smart_model() {
        let env = HashMapEnvView::new([("ANTHROPIC_API_KEY", "sk-test")]);
        let model = resolve_profile_model(
            InferenceProfile {
                priority: InferencePriority::Quality,
                reasoning: ReasoningEffort::None,
            },
            &env,
        )
        .unwrap();
        assert_eq!(model.provider().display_name(), "Anthropic");
    }
}
