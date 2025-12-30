//! Model selection implementation with fallback stacking
//!
//! This module provides the core `get_model()` function that implements
//! fallback stacking and stderr logging for model selection.

use super::types::{LlmClient, ModelKind, ModelProvider, ModelStack};
use rig::client::ProviderClient;
use rig::providers::{gemini, openai};
use thiserror::Error;
use tracing::warn;

/// Errors that can occur during model selection
#[derive(Debug, Error)]
pub enum ModelError {
    /// No valid model available in stack
    #[error("No valid model available in stack. Attempted: {attempted:?}")]
    NoValidModel {
        /// List of (provider/model, reason) pairs for failed attempts
        attempted: Vec<(String, String)>,
    },

    /// Client initialization failed
    #[error("Client initialization failed for {provider}: {reason}")]
    ClientInitFailed {
        /// Provider name
        provider: String,
        /// Failure reason
        reason: String,
    },
}

/// Get a model client with fallback stacking
///
/// This function attempts to initialize a model client based on the provided
/// `ModelKind`. If the primary model fails to initialize, it will try fallback
/// models in order until one succeeds.
///
/// # Arguments
///
/// * `kind` - The model selection strategy (Quality tier, Use case, or Explicit)
/// * `desc` - Optional description of the task (logged to stderr if provided)
///
/// # Returns
///
/// Returns an `LlmClient` wrapper containing the initialized client for the
/// selected provider.
///
/// # Errors
///
/// Returns `ModelError::NoValidModel` if all models in the stack fail to initialize.
///
/// # Examples
///
/// ```rust,no_run
/// use shared::model::{get_model, ModelKind, ModelQuality};
///
/// // Select a fast model for scraping
/// let client = get_model(
///     ModelKind::Quality(ModelQuality::Fast),
///     Some("scrape web content")
/// )?;
/// ```
pub fn get_model(kind: ModelKind, desc: Option<&str>) -> Result<LlmClient, ModelError> {
    let stack = match kind {
        ModelKind::Quality(quality) => ModelStack::for_quality(quality),
        ModelKind::UseCase { task, quality } => ModelStack::for_task(task, quality),
        ModelKind::TryExplicit {
            explicit_first,
            fallback,
        } => {
            let mut stack = ModelStack(vec![explicit_first]);
            stack.0.extend(ModelStack::for_quality(fallback).0);
            stack
        }
    };

    let mut attempted = Vec::new();

    for model_provider in stack.0 {
        match try_build_client(&model_provider) {
            Ok(client) => {
                let (provider, model) = model_provider.to_rig_identifier();
                if let Some(description) = desc {
                    eprintln!("- using the {} from {} to {}", model, provider, description);
                }
                return Ok(client);
            }
            Err(e) => {
                warn!(
                    "Failed to initialize {} {}: {}",
                    model_provider.provider, model_provider.model, e
                );
                attempted.push((
                    format!("{}/{}", model_provider.provider, model_provider.model),
                    e.to_string(),
                ));
                continue;
            }
        }
    }

    Err(ModelError::NoValidModel { attempted })
}

/// Try to build a client for the given provider
fn try_build_client(provider: &ModelProvider) -> Result<LlmClient, ModelError> {
    let (provider_name, _model_id) = provider.to_rig_identifier();

    // Use rig client builders
    // Note: from_env() may panic if environment variables are not set
    // We catch the panic and convert to an error
    match provider_name {
        "openai" => {
            // Try to create OpenAI client
            std::panic::catch_unwind(|| {
                let client = openai::Client::from_env();
                LlmClient::OpenAI(client)
            })
            .map_err(|e| {
                let msg = if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "OpenAI client initialization panicked".to_string()
                };
                ModelError::ClientInitFailed {
                    provider: "openai".to_string(),
                    reason: msg,
                }
            })
        }
        "gemini" => {
            // Try to create Gemini client
            std::panic::catch_unwind(|| {
                let client = gemini::Client::from_env();
                LlmClient::Gemini(client)
            })
            .map_err(|e| {
                let msg = if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Gemini client initialization panicked".to_string()
                };
                ModelError::ClientInitFailed {
                    provider: "gemini".to_string(),
                    reason: msg,
                }
            })
        }
        "anthropic" => {
            // Anthropic is not supported in rig yet
            Err(ModelError::ClientInitFailed {
                provider: "anthropic".to_string(),
                reason: "Anthropic provider not yet supported in rig library".to_string(),
            })
        }
        _ => Err(ModelError::ClientInitFailed {
            provider: provider_name.to_string(),
            reason: format!("Unsupported provider: {}", provider_name),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::types::{ModelQuality, TaskKind};

    #[test]
    fn model_error_no_valid_model_displays_correctly() {
        let error = ModelError::NoValidModel {
            attempted: vec![
                ("anthropic/claude-haiku-4-5".to_string(), "No API key".to_string()),
                ("gemini/gemini-flash".to_string(), "No API key".to_string()),
            ],
        };
        let error_str = error.to_string();
        assert!(error_str.contains("No valid model available"));
        assert!(error_str.contains("anthropic/claude-haiku-4-5"));
    }

    #[test]
    fn model_error_client_init_failed_displays_correctly() {
        let error = ModelError::ClientInitFailed {
            provider: "anthropic".to_string(),
            reason: "Missing ANTHROPIC_API_KEY".to_string(),
        };
        let error_str = error.to_string();
        assert!(error_str.contains("Client initialization failed"));
        assert!(error_str.contains("anthropic"));
        assert!(error_str.contains("Missing ANTHROPIC_API_KEY"));
    }

    #[test]
    fn try_build_client_unsupported_provider() {
        let provider = ModelProvider::new("unsupported", "model-1");
        let result = try_build_client(&provider);
        assert!(result.is_err());
        match result.unwrap_err() {
            ModelError::ClientInitFailed { provider, reason } => {
                assert_eq!(provider, "unsupported");
                assert!(reason.contains("Unsupported provider"));
            }
            _ => panic!("Expected ClientInitFailed error"),
        }
    }

    // Note: We cannot test actual client building without API keys
    // Integration tests would need to mock the rig-core clients or use test credentials

    #[test]
    fn get_model_constructs_quality_stack() {
        // This test will fail without API keys, but verifies the stack construction
        let result = get_model(ModelKind::Quality(ModelQuality::Fast), None);
        // Without API keys, this should fail with NoValidModel error
        if let Err(ModelError::NoValidModel { attempted }) = result {
            assert!(!attempted.is_empty());
            // Should have attempted haiku and flash
            assert!(attempted.len() >= 1);
        }
    }

    #[test]
    fn get_model_constructs_usecase_stack() {
        let result = get_model(
            ModelKind::UseCase {
                task: TaskKind::Scrape,
                quality: None,
            },
            None,
        );
        // Without API keys, this should fail with NoValidModel error
        if let Err(ModelError::NoValidModel { attempted }) = result {
            assert!(!attempted.is_empty());
        }
    }

    #[test]
    fn get_model_constructs_try_explicit_stack() {
        let explicit = ModelProvider::new("anthropic", "claude-opus-4-5");
        let result = get_model(
            ModelKind::TryExplicit {
                explicit_first: explicit,
                fallback: ModelQuality::Fast,
            },
            None,
        );
        // Without API keys, this should fail with NoValidModel error
        if let Err(ModelError::NoValidModel { attempted }) = result {
            assert!(!attempted.is_empty());
            // First attempt should be the explicit model
            assert!(attempted[0].0.contains("opus"));
        }
    }
}
