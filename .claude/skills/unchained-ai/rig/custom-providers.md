# Creating Custom Providers

Rig makes it straightforward to create OpenAI-compatible API providers. This is valuable for vendor flexibility, cost optimization, redundancy, and accessing specialized models while maintaining a consistent API.

## Why Custom Providers?

- **Vendor flexibility**: Switch between providers without rewriting client code
- **Cost optimization**: Choose the most cost-effective option
- **Redundancy**: Have backup providers when one is unavailable
- **Specialized models**: Access unique models while maintaining a consistent API

## Understanding Provider Architecture

Custom providers in Rig typically implement:

1. **Client struct**: Factory for creating models
2. **CompletionModel trait**: For text generation
3. **EmbeddingModel trait**: For vector embeddings
4. **Provider-specific types**: Request/response structures

## Basic Provider Structure

```rust
use rig::{
    completion::{CompletionModel, CompletionRequest, CompletionResponse},
    embeddings::{EmbeddingModel, Embedding, EmbeddingError},
};
use reqwest::Client as HttpClient;

pub struct CustomClient {
    api_key: String,
    base_url: String,
    http_client: HttpClient,
}

impl CustomClient {
    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
            http_client: HttpClient::new(),
        }
    }

    pub fn from_env() -> Self {
        let api_key = std::env::var("CUSTOM_API_KEY")
            .expect("CUSTOM_API_KEY must be set");
        let base_url = std::env::var("CUSTOM_BASE_URL")
            .unwrap_or_else(|_| "https://api.custom.com".to_string());

        Self::new(api_key, base_url)
    }

    pub fn completion_model(&self, model: &str) -> CustomCompletionModel {
        CustomCompletionModel {
            client: self.clone(),
            model_name: model.to_string(),
        }
    }

    pub fn embedding_model(&self, model: &str) -> CustomEmbeddingModel {
        CustomEmbeddingModel {
            client: self.clone(),
            model_name: model.to_string(),
            ndims: 1536, // Default, adjust per model
        }
    }
}
```

## Implementing CompletionModel

```rust
use async_trait::async_trait;
use rig::completion::{CompletionModel, CompletionRequest, CompletionResponse, Choice};

pub struct CustomCompletionModel {
    client: CustomClient,
    model_name: String,
}

#[async_trait]
impl CompletionModel for CustomCompletionModel {
    type Response = CompletionResponse;

    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<Self::Response, Box<dyn std::error::Error + Send + Sync>> {
        // Transform Rig's CompletionRequest to your provider's format
        let provider_request = serde_json::json!({
            "model": self.model_name,
            "messages": request.messages,
            "temperature": request.temperature.unwrap_or(0.7),
            "max_tokens": request.max_tokens,
            "stream": false,
        });

        // Send request to your provider
        let response = self.client.http_client
            .post(&format!("{}/v1/chat/completions", self.client.base_url))
            .header("Authorization", &format!("Bearer {}", self.client.api_key))
            .json(&provider_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(format!("API request failed ({}): {}", status, error_text).into());
        }

        let provider_response: serde_json::Value = response.json().await?;

        // Transform provider's response to Rig's CompletionResponse
        Ok(CompletionResponse {
            id: provider_response["id"].as_str().unwrap_or("unknown").to_string(),
            object: "chat.completion".to_string(),
            created: provider_response["created"].as_u64().unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }),
            model: self.model_name.clone(),
            choices: vec![Choice {
                index: 0,
                message: provider_response["choices"][0]["message"].clone(),
                finish_reason: provider_response["choices"][0]["finish_reason"]
                    .as_str()
                    .map(|s| s.to_string()),
            }],
            usage: provider_response["usage"].clone(),
        })
    }
}
```

## Implementing EmbeddingModel

```rust
pub struct CustomEmbeddingModel {
    client: CustomClient,
    model_name: String,
    ndims: usize,
}

impl EmbeddingModel for CustomEmbeddingModel {
    const MAX_DOCUMENTS: usize = 100; // Provider-specific limit

    type Client = CustomClient;

    fn make(client: &Self::Client, model: impl Into<String>, dims: Option<usize>) -> Self {
        Self {
            client: client.clone(),
            model_name: model.into(),
            ndims: dims.unwrap_or(1536),
        }
    }

    fn ndims(&self) -> usize {
        self.ndims
    }

    fn embed_texts(
        &self,
        texts: impl IntoIterator<Item = String>,
    ) -> impl std::future::Future<Output = Result<Vec<Embedding>, EmbeddingError>> + Send {
        async move {
            let texts: Vec<String> = texts.into_iter().collect();

            if texts.len() > Self::MAX_DOCUMENTS {
                return Err(EmbeddingError::ProviderError(
                    format!("Batch size {} exceeds maximum {}", texts.len(), Self::MAX_DOCUMENTS)
                ));
            }

            let request = serde_json::json!({
                "model": self.model_name,
                "input": texts,
            });

            let response = self.client.http_client
                .post(&format!("{}/v1/embeddings", self.client.base_url))
                .header("Authorization", &format!("Bearer {}", self.client.api_key))
                .json(&request)
                .send()
                .await
                .map_err(|e| EmbeddingError::ProviderError(e.to_string()))?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                return Err(EmbeddingError::ProviderError(
                    format!("API request failed ({}): {}", status, error_text)
                ));
            }

            let response_json: serde_json::Value = response.json().await
                .map_err(|e| EmbeddingError::ProviderError(e.to_string()))?;

            let data = response_json["data"].as_array()
                .ok_or_else(|| EmbeddingError::ProviderError("Missing data field".to_string()))?;

            let embeddings: Result<Vec<Embedding>, EmbeddingError> = texts
                .into_iter()
                .enumerate()
                .map(|(i, document)| {
                    let vec = data[i]["embedding"]
                        .as_array()
                        .ok_or_else(|| EmbeddingError::ProviderError("Missing embedding".to_string()))?
                        .iter()
                        .map(|v| v.as_f64().unwrap_or(0.0))
                        .collect();

                    Ok(Embedding { document, vec })
                })
                .collect();

            embeddings
        }
    }
}
```

## Using Your Custom Provider

```rust
use rig::completion::Prompt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize your custom client
    let client = CustomClient::from_env();

    // Create an agent using your provider
    let agent = client
        .completion_model("custom-model-v1")
        .agent()
        .preamble("You are a helpful assistant.")
        .build();

    let response = agent.prompt("What is Rust?").await?;
    println!("Response: {}", response);

    // Use embeddings
    let embedding_model = client.embedding_model("custom-embeddings-v1");
    let embedding = embedding_model.embed_text("Hello, world!").await?;
    println!("Embedding dimensions: {}", embedding.vec.len());

    Ok(())
}
```

## OpenAI-Compatible Providers

Many providers offer OpenAI-compatible APIs. For these, you can often reuse Rig's OpenAI client with a custom base URL:

```rust
use rig::providers::openai;

// Example: Using DeepSeek with OpenAI client
let client = openai::Client::new(
    "your-deepseek-api-key",
    "https://api.deepseek.com/v1"
);

let agent = client
    .agent("deepseek-chat")
    .build();

let response = agent.prompt("Hello!").await?;
```

## Error Handling

Implement comprehensive error handling:

```rust
#[derive(Debug)]
pub enum CustomProviderError {
    HttpError(reqwest::Error),
    ApiError { status: u16, message: String },
    ParseError(serde_json::Error),
    InvalidResponse(String),
}

impl std::fmt::Display for CustomProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HttpError(e) => write!(f, "HTTP error: {}", e),
            Self::ApiError { status, message } => {
                write!(f, "API error ({}): {}", status, message)
            }
            Self::ParseError(e) => write!(f, "Parse error: {}", e),
            Self::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
        }
    }
}

impl std::error::Error for CustomProviderError {}
```

## Rate Limiting

Implement client-side rate limiting:

```rust
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct RateLimitedClient {
    client: CustomClient,
    semaphore: Arc<Semaphore>,
}

impl RateLimitedClient {
    pub fn new(client: CustomClient, max_concurrent: usize) -> Self {
        Self {
            client,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    pub async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, Box<dyn std::error::Error + Send + Sync>> {
        let _permit = self.semaphore.acquire().await?;
        self.client.completion_model("model").complete(request).await
    }
}
```

## Retry Logic

Add automatic retries for transient failures:

```rust
use tokio::time::{sleep, Duration};

async fn complete_with_retry(
    model: &CustomCompletionModel,
    request: CompletionRequest,
    max_retries: usize,
) -> Result<CompletionResponse, Box<dyn std::error::Error + Send + Sync>> {
    let mut attempts = 0;

    loop {
        match model.complete(request.clone()).await {
            Ok(response) => return Ok(response),
            Err(e) if attempts < max_retries => {
                attempts += 1;
                let backoff = Duration::from_secs(2_u64.pow(attempts as u32));
                eprintln!("Retry {} after {:?}: {}", attempts, backoff, e);
                sleep(backoff).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

## Testing Your Provider

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_completion() {
        let client = CustomClient::new(
            "test-key".to_string(),
            "https://api.test.com".to_string(),
        );

        let model = client.completion_model("test-model");

        let request = CompletionRequest {
            messages: vec![],
            temperature: Some(0.7),
            max_tokens: Some(100),
            ..Default::default()
        };

        let result = model.complete(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_embeddings() {
        let client = CustomClient::from_env();
        let model = client.embedding_model("test-embeddings");

        let embedding = model.embed_text("test").await;
        assert!(embedding.is_ok());
        assert_eq!(embedding.unwrap().vec.len(), model.ndims());
    }
}
```

## Deployment Considerations

### Environment Configuration

```rust
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ProviderConfig {
    pub api_key: String,
    pub base_url: String,
    pub max_retries: usize,
    pub timeout_secs: u64,
    pub max_concurrent_requests: usize,
}

impl ProviderConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            api_key: std::env::var("CUSTOM_API_KEY")?,
            base_url: std::env::var("CUSTOM_BASE_URL")
                .unwrap_or_else(|_| "https://api.custom.com".to_string()),
            max_retries: std::env::var("MAX_RETRIES")
                .unwrap_or_else(|_| "3".to_string())
                .parse()?,
            timeout_secs: std::env::var("TIMEOUT_SECS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()?,
            max_concurrent_requests: std::env::var("MAX_CONCURRENT")
                .unwrap_or_else(|_| "10".to_string())
                .parse()?,
        })
    }
}
```

### Monitoring

Add logging and metrics:

```rust
use tracing::{info, warn, error};

impl CustomCompletionModel {
    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, Box<dyn std::error::Error + Send + Sync>> {
        let start = std::time::Instant::now();

        info!(
            model = %self.model_name,
            "Starting completion request"
        );

        let result = self.do_complete(request).await;

        let duration = start.elapsed();

        match &result {
            Ok(_) => {
                info!(
                    model = %self.model_name,
                    duration_ms = duration.as_millis(),
                    "Completion succeeded"
                );
            }
            Err(e) => {
                error!(
                    model = %self.model_name,
                    duration_ms = duration.as_millis(),
                    error = %e,
                    "Completion failed"
                );
            }
        }

        result
    }
}
```

## Best Practices

1. **Type Safety**: Use strong types for requests/responses
2. **Error Handling**: Provide detailed error messages
3. **Rate Limiting**: Implement client-side rate limiting
4. **Retries**: Add automatic retry logic with exponential backoff
5. **Testing**: Write comprehensive unit and integration tests
6. **Monitoring**: Add logging and metrics collection
7. **Configuration**: Use environment variables for configuration
8. **Documentation**: Document API differences from OpenAI

## Related Topics

- [Tool Calling](./tool-calling.md) - Using tools with custom providers
- [Embeddings](./embeddings.md) - Implementing custom embedding models
- [Vector Store Integration](./vector-stores.md) - Using custom embeddings for RAG
