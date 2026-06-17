//! Provider-neutral inference contract.
//!
//! Reaper, Darkmatter, Claudine, and Unchained AI agree on the data shapes and
//! the [`InferenceAdapter`] trait defined here. Provider implementations live
//! in their own crates; consumer wiring lives in Reaper and Darkmatter.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use thiserror::Error;

/// Which trade-off an adapter should optimize for when it has a choice.
///
/// Adapters may approximate these preferences; they translate the priority
/// into the provider's own model catalog and inference settings. `Balanced`
/// defers the trade-off to the adapter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InferencePriority {
    /// Optimize for the lowest cost.
    Cost,
    /// Optimize for the lowest latency.
    Latency,
    /// Optimize for the highest output quality.
    Quality,
    /// Defer the cost/latency/quality trade-off to the adapter.
    #[default]
    Balanced,
}

/// Desired reasoning effort for a request.
///
/// `None` requests no deliberate reasoning mode; it does not assert that a
/// provider performs no internal reasoning.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ReasoningEffort {
    /// No deliberate reasoning mode requested.
    None,
    /// Light reasoning effort.
    Low,
    /// Standard reasoning effort.
    #[default]
    Medium,
    /// Maximum reasoning effort.
    High,
}

/// Best-effort provider-neutral preferences for a single inference call.
///
/// Preferences are not service-level guarantees; an adapter may approximate a
/// profile when its provider has no exact mapping and only returns
/// [`InferenceErrorKind::Unsupported`] when it cannot perform the operation
/// at all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InferenceProfile {
    /// Which trade-off to optimize.
    pub priority: InferencePriority,
    /// Desired reasoning effort.
    pub reasoning: ReasoningEffort,
}

/// The shape of the value an adapter must produce for a request.
#[derive(Debug, Clone, PartialEq)]
pub enum InferenceOutput {
    /// Free-form natural-language text.
    Prose,
    /// A JSON value that must validate against `schema` (JSON Schema
    /// Draft 2020-12). A successful response is
    /// [`InferenceData::Structured`] and satisfies the schema.
    Structured {
        /// The JSON Schema describing the expected value.
        schema: Value,
    },
}

/// The payload returned by an adapter for a single inference call.
#[derive(Debug, Clone, PartialEq)]
pub enum InferenceData {
    /// Natural-language text produced for an [`InferenceOutput::Prose`]
    /// request.
    Prose(String),
    /// A JSON value produced for an [`InferenceOutput::Structured`] request.
    /// The adapter is responsible for confirming the value satisfies the
    /// supplied schema before returning success.
    Structured(Value),
}

/// Diagnostic metadata about the adapter that produced a response.
///
/// Each field is optional because not every backend exposes all three
/// identities. These fields are diagnostic only and must not be used by
/// consumers to change domain semantics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InferenceMetadata {
    /// Provider identifier (e.g. `openai`, `anthropic`), when known.
    pub provider: Option<String>,
    /// Concrete model identifier (e.g. `gpt-4o-mini`), when known.
    pub model: Option<String>,
    /// Agent identifier (e.g. an agent slug), when known.
    pub agent: Option<String>,
}

/// Owned request data for one inference call.
///
/// The request owns its data so the returned future is independent of caller
/// lifetimes and can be dispatched or queued by an adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct InferenceRequest {
    /// The text prompt to send to the adapter.
    pub prompt: String,
    /// The shape of value the caller expects in return.
    pub output: InferenceOutput,
    /// Best-effort provider-neutral preferences for this call.
    pub profile: InferenceProfile,
}

/// The result of one inference call.
#[derive(Debug, Clone, PartialEq)]
pub struct InferenceResponse {
    /// The payload returned by the adapter.
    pub data: InferenceData,
    /// Diagnostic metadata about the adapter that produced `data`.
    pub metadata: InferenceMetadata,
}

/// Stable error category for [`InferenceError`].
///
/// Consumers can switch on `kind` without depending on provider-specific
/// error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceErrorKind {
    /// The request was rejected before reaching the provider (for example, an
    /// invalid schema).
    InvalidRequest,
    /// The adapter cannot perform the operation at all.
    Unsupported,
    /// The provider is temporarily unavailable.
    Unavailable,
    /// The caller is not authorized to use the provider.
    Unauthorized,
    /// The caller has been rate-limited by the provider.
    RateLimited,
    /// The provider did not respond within an adapter-internal deadline.
    Timeout,
    /// The provider returned a response that did not satisfy the contract
    /// (invalid JSON, schema mismatch, or a response variant that differs
    /// from the request).
    InvalidResponse,
    /// A provider-specific failure that does not fit another category.
    Provider,
}

/// Concrete error type returned by [`InferenceAdapter::infer`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct InferenceError {
    /// The stable error category.
    pub kind: InferenceErrorKind,
    /// Human-readable diagnostic message. Must not contain secrets, API keys,
    /// authorization headers, or full provider payloads.
    pub message: String,
    /// Suggested delay before retrying, populated only when the provider
    /// supplies a meaningful value (normally for
    /// [`InferenceErrorKind::RateLimited`] or
    /// [`InferenceErrorKind::Unavailable`]). The contract does not retry.
    pub retry_after: Option<Duration>,
}

impl InferenceError {
    /// Builds a new `InferenceError` with no `retry_after` hint.
    pub fn new(kind: InferenceErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            retry_after: None,
        }
    }

    /// Sets `retry_after` on `self` and returns `self` for builder-style use.
    #[must_use]
    pub fn with_retry_after(mut self, retry_after: Duration) -> Self {
        self.retry_after = Some(retry_after);
        self
    }
}

/// Provider-neutral interface for one text-inference operation.
///
/// Implementations validate a structured response against the supplied JSON
/// Schema before returning success. They populate
/// [`InferenceMetadata`](InferenceMetadata) with every identity they can
/// report reliably.
///
/// Object-safe and `Send + Sync` so consumers can store and inject
/// `Arc<dyn InferenceAdapter>` across ordinary multi-threaded runtimes.
///
/// ## Examples
///
/// ```no_run
/// use std::sync::Arc;
/// use biscuit_contract::inference::InferenceAdapter;
///
/// fn _accepts(_: Arc<dyn InferenceAdapter>) {}
/// ```
#[async_trait]
pub trait InferenceAdapter: Send + Sync {
    /// Performs one inference call.
    ///
    /// `request` is owned so the returned future is independent of caller
    /// lifetimes; the adapter may dispatch or queue it as it sees fit. The
    /// future is `Send` and best-effort cancellable: dropping it requests
    /// cancellation, but an upstream provider may already be processing the
    /// request. Callers own end-to-end deadlines and may wrap `infer` with
    /// `tokio::time::timeout` or an equivalent runtime mechanism.
    async fn infer(
        &self,
        request: InferenceRequest,
    ) -> Result<InferenceResponse, InferenceError>;
}

// Compile-time proof that the trait is object-safe: a function that takes
// `Arc<dyn InferenceAdapter>` by value only type-checks if the trait object
// is constructible. Kept hidden so it does not appear in the public API.
#[doc(hidden)]
#[allow(dead_code)]
fn _assert_inference_adapter_is_object_safe() {
    fn _accepts(_: Arc<dyn InferenceAdapter>) {}
}

#[cfg(test)]
mod tests {
    //! Deterministic L1 tests for the inference contract.
    //!
    //! These tests verify object-safety of the trait, round-trip behavior
    //! for prose and structured outputs, profile defaults, every error
    //! category, and the contract's `InvalidResponse` semantics for
    //! response-variant and schema mismatches.

    use async_trait::async_trait;
    use serde_json::{json, Value};
    use std::sync::Arc;

    use super::{
        InferenceAdapter, InferenceData, InferenceError, InferenceErrorKind,
        InferenceMetadata, InferenceOutput, InferencePriority, InferenceProfile,
        InferenceRequest, InferenceResponse, ReasoningEffort,
    };

    /// Returns prose for prose requests and rejects structured requests.
    /// Populates every metadata field so the round-trip test can assert
    /// each one end-to-end.
    struct FakeProseAdapter;

    #[async_trait]
    impl InferenceAdapter for FakeProseAdapter {
        async fn infer(
            &self,
            request: InferenceRequest,
        ) -> Result<InferenceResponse, InferenceError> {
            match request.output {
                InferenceOutput::Prose => Ok(InferenceResponse {
                    data: InferenceData::Prose(format!("echo: {}", request.prompt)),
                    metadata: InferenceMetadata {
                        provider: Some("fake-prose".to_string()),
                        model: Some("fake-prose-v1".to_string()),
                        agent: Some("fake-prose-agent".to_string()),
                    },
                }),
                InferenceOutput::Structured { .. } => Err(InferenceError::new(
                    InferenceErrorKind::InvalidRequest,
                    "fake prose adapter only accepts Prose requests",
                )),
            }
        }
    }

    /// Validates that a structured request's schema is a non-null object
    /// schema and returns a structured response that echoes the request.
    struct FakeStructuredAdapter;

    #[async_trait]
    impl InferenceAdapter for FakeStructuredAdapter {
        async fn infer(
            &self,
            request: InferenceRequest,
        ) -> Result<InferenceResponse, InferenceError> {
            match request.output {
                InferenceOutput::Structured { schema } => {
                    if !is_object_schema(&schema) {
                        return Err(InferenceError::new(
                            InferenceErrorKind::InvalidRequest,
                            "schema must be a non-null object schema",
                        ));
                    }
                    Ok(InferenceResponse {
                        data: InferenceData::Structured(json!({
                            "received_prompt": request.prompt,
                            "ok": true,
                        })),
                        metadata: InferenceMetadata {
                            provider: Some("fake-structured".to_string()),
                            model: Some("fake-structured-v1".to_string()),
                            agent: None,
                        },
                    })
                }
                InferenceOutput::Prose => Err(InferenceError::new(
                    InferenceErrorKind::InvalidRequest,
                    "fake structured adapter only accepts Structured requests",
                )),
            }
        }
    }

    /// True when `schema` is a JSON object that names `type: "object"`.
    /// The contract crate deliberately does not perform full schema
    /// validation; this is the minimum shape a structured adapter is
    /// expected to enforce before validating against a real schema
    /// engine.
    fn is_object_schema(schema: &Value) -> bool {
        schema.is_object()
            && schema
                .get("type")
                .and_then(Value::as_str)
                .is_some_and(|t| t == "object")
    }

    /// Simulates a response-variant mismatch: the adapter produced prose
    /// for a structured request and reports the mismatch as
    /// `InvalidResponse` before returning.
    struct FakeVariantMismatchAdapter;

    #[async_trait]
    impl InferenceAdapter for FakeVariantMismatchAdapter {
        async fn infer(
            &self,
            _request: InferenceRequest,
        ) -> Result<InferenceResponse, InferenceError> {
            Err(InferenceError::new(
                InferenceErrorKind::InvalidResponse,
                "response variant does not match the request output",
            ))
        }
    }

    /// Simulates a schema violation: structured data would not satisfy
    /// the supplied schema. Reports the violation as `InvalidResponse`.
    struct FakeSchemaViolationAdapter;

    #[async_trait]
    impl InferenceAdapter for FakeSchemaViolationAdapter {
        async fn infer(
            &self,
            request: InferenceRequest,
        ) -> Result<InferenceResponse, InferenceError> {
            match request.output {
                InferenceOutput::Structured { schema: _ } => Err(InferenceError::new(
                    InferenceErrorKind::InvalidResponse,
                    "response does not satisfy the supplied schema",
                )),
                InferenceOutput::Prose => Err(InferenceError::new(
                    InferenceErrorKind::InvalidRequest,
                    "schema-violation fake only accepts Structured requests",
                )),
            }
        }
    }

    #[test]
    fn inference_profile_defaults_are_balanced_and_medium() {
        let profile = InferenceProfile::default();
        assert_eq!(profile.priority, InferencePriority::Balanced);
        assert_eq!(profile.reasoning, ReasoningEffort::Medium);
        assert_eq!(InferencePriority::default(), InferencePriority::Balanced);
        assert_eq!(ReasoningEffort::default(), ReasoningEffort::Medium);
    }

    #[test]
    fn inference_error_kind_variants_construct_and_match() {
        // Construct every variant, then exhaustively pattern-match to
        // guarantee that future enum additions or removals are caught
        // by the test suite.
        let kinds = [
            InferenceErrorKind::InvalidRequest,
            InferenceErrorKind::Unsupported,
            InferenceErrorKind::Unavailable,
            InferenceErrorKind::Unauthorized,
            InferenceErrorKind::RateLimited,
            InferenceErrorKind::Timeout,
            InferenceErrorKind::InvalidResponse,
            InferenceErrorKind::Provider,
        ];

        let mut observed = Vec::new();
        for kind in kinds {
            match kind {
                InferenceErrorKind::InvalidRequest => observed.push("invalid_request"),
                InferenceErrorKind::Unsupported => observed.push("unsupported"),
                InferenceErrorKind::Unavailable => observed.push("unavailable"),
                InferenceErrorKind::Unauthorized => observed.push("unauthorized"),
                InferenceErrorKind::RateLimited => observed.push("rate_limited"),
                InferenceErrorKind::Timeout => observed.push("timeout"),
                InferenceErrorKind::InvalidResponse => observed.push("invalid_response"),
                InferenceErrorKind::Provider => observed.push("provider"),
            }
        }
        assert_eq!(observed.len(), 8);

        // Equality and inequality are derivable, but pin them down so
        // `InferenceErrorKind` cannot drift to a non-`Eq` enum without
        // surfacing a compile error here.
        assert_eq!(InferenceErrorKind::InvalidRequest, InferenceErrorKind::InvalidRequest);
        assert_ne!(InferenceErrorKind::InvalidRequest, InferenceErrorKind::Provider);
    }

    #[tokio::test]
    async fn fake_adapter_stores_in_arc_dyn_inference_adapter() {
        let adapter: Arc<dyn InferenceAdapter> = Arc::new(FakeProseAdapter);
        let request = InferenceRequest {
            prompt: "ping".to_string(),
            output: InferenceOutput::Prose,
            profile: InferenceProfile::default(),
        };

        let response = adapter.infer(request).await.expect("prose should succeed");

        match response.data {
            InferenceData::Prose(text) => assert_eq!(text, "echo: ping"),
            InferenceData::Structured(_) => panic!("expected prose response"),
        }
    }

    #[tokio::test]
    async fn prose_request_round_trip_via_trait_object() {
        let adapter: Arc<dyn InferenceAdapter> = Arc::new(FakeProseAdapter);
        let request = InferenceRequest {
            prompt: "hello".to_string(),
            output: InferenceOutput::Prose,
            profile: InferenceProfile {
                priority: InferencePriority::Quality,
                reasoning: ReasoningEffort::High,
            },
        };

        let response = adapter.infer(request).await.expect("prose should succeed");

        assert_eq!(
            response.data,
            InferenceData::Prose("echo: hello".to_string())
        );
        assert_eq!(response.metadata.provider.as_deref(), Some("fake-prose"));
        assert_eq!(response.metadata.model.as_deref(), Some("fake-prose-v1"));
        assert_eq!(
            response.metadata.agent.as_deref(),
            Some("fake-prose-agent")
        );
    }

    #[tokio::test]
    async fn structured_request_round_trip_via_trait_object() {
        let adapter: Arc<dyn InferenceAdapter> = Arc::new(FakeStructuredAdapter);
        let schema = json!({
            "type": "object",
            "properties": {
                "ok": { "type": "boolean" },
                "received_prompt": { "type": "string" }
            }
        });
        let request = InferenceRequest {
            prompt: "give me a result".to_string(),
            output: InferenceOutput::Structured { schema },
            profile: InferenceProfile::default(),
        };

        let response = adapter
            .infer(request)
            .await
            .expect("structured should succeed");

        match response.data {
            InferenceData::Structured(value) => {
                assert_eq!(value.get("ok").and_then(Value::as_bool), Some(true));
                assert_eq!(
                    value.get("received_prompt").and_then(Value::as_str),
                    Some("give me a result")
                );
            }
            InferenceData::Prose(_) => panic!("expected structured response"),
        }
        assert_eq!(
            response.metadata.provider.as_deref(),
            Some("fake-structured")
        );
        assert_eq!(
            response.metadata.model.as_deref(),
            Some("fake-structured-v1")
        );
        assert_eq!(response.metadata.agent, None);
    }

    #[tokio::test]
    async fn response_variant_mismatch_is_invalid_response() {
        let adapter: Arc<dyn InferenceAdapter> = Arc::new(FakeVariantMismatchAdapter);
        let request = InferenceRequest {
            prompt: "structured please".to_string(),
            output: InferenceOutput::Structured {
                schema: json!({"type": "object"}),
            },
            profile: InferenceProfile::default(),
        };

        let err = adapter
            .infer(request)
            .await
            .expect_err("variant mismatch should be reported as an error");

        assert_eq!(err.kind, InferenceErrorKind::InvalidResponse);
        assert!(!err.message.is_empty());
    }

    #[tokio::test]
    async fn schema_violation_is_invalid_response() {
        let adapter: Arc<dyn InferenceAdapter> = Arc::new(FakeSchemaViolationAdapter);
        let request = InferenceRequest {
            prompt: "give me json".to_string(),
            output: InferenceOutput::Structured {
                schema: json!({"type": "object"}),
            },
            profile: InferenceProfile::default(),
        };

        let err = adapter
            .infer(request)
            .await
            .expect_err("schema violation should be reported as an error");

        assert_eq!(err.kind, InferenceErrorKind::InvalidResponse);
        assert!(!err.message.is_empty());
    }
}
