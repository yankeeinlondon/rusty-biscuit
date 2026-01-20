# OpenAI Compatibility Layer

Ollama provides an OpenAI-compatible API at `/v1/*` endpoints, allowing seamless migration of existing applications.

## Quick Setup

Simply point your OpenAI client to Ollama:

```bash
# Environment variables
export OPENAI_BASE_URL=http://localhost:11434/v1
export OPENAI_API_KEY=ollama  # Can be anything, Ollama ignores it
```

## Supported Endpoints

### Chat Completions
```http
POST /v1/chat/completions
```

Standard OpenAI request format:
```json
{
  "model": "llama3",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello!"}
  ],
  "temperature": 0.7,
  "max_tokens": 100,
  "stream": false
}
```

### Models List
```http
GET /v1/models
```

Returns OpenAI-compatible model list:
```json
{
  "object": "list",
  "data": [
    {
      "id": "llama3",
      "object": "model",
      "created": 1686935002,
      "owned_by": "library"
    }
  ]
}
```

## Rust Integration with async-openai

### Basic Setup
```toml
# Cargo.toml
[dependencies]
async-openai = "0.23"
tokio = { version = "1", features = ["full"] }
```

### Complete Example
```rust
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure client for Ollama
    let config = OpenAIConfig::new()
        .with_api_base("http://localhost:11434/v1")
        .with_api_key("ollama"); // Required by library, ignored by Ollama

    let client = Client::with_config(config);

    // Build messages
    let messages = vec![
        ChatCompletionRequestMessage::System(
            ChatCompletionRequestSystemMessageArgs::default()
                .content("You are a Rust expert.")
                .build()?
        ),
        ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessageArgs::default()
                .content("Explain ownership in Rust")
                .build()?
        ),
    ];

    // Create request
    let request = CreateChatCompletionRequestArgs::default()
        .model("llama3")
        .messages(messages)
        .temperature(0.7)
        .max_tokens(200)
        .build()?;

    // Get response
    let response = client.chat().create(request).await?;

    // Extract content
    if let Some(choice) = response.choices.first() {
        println!("{}", choice.message.content.as_ref().unwrap());
    }

    Ok(())
}
```

### Streaming Example
```rust
use async_openai::types::CreateChatCompletionStreamResponse;
use futures::StreamExt;

async fn stream_chat() -> Result<(), Box<dyn std::error::Error>> {
    let config = OpenAIConfig::new()
        .with_api_base("http://localhost:11434/v1")
        .with_api_key("ollama");

    let client = Client::with_config(config);

    let request = CreateChatCompletionRequestArgs::default()
        .model("llama3")
        .messages([
            ChatCompletionRequestUserMessageArgs::default()
                .content("Write a haiku about Rust")
                .build()?
                .into()
        ])
        .stream(true)
        .build()?;

    let mut stream = client.chat().create_stream(request).await?;

    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                if let Some(choice) = response.choices.first() {
                    if let Some(content) = &choice.delta.content {
                        print!("{}", content);
                        std::io::stdout().flush()?;
                    }
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    println!(); // New line after streaming
    Ok(())
}
```

## Parameter Mapping

OpenAI parameters are automatically mapped to Ollama equivalents:

| OpenAI Parameter | Ollama Mapping | Notes |
|------------------|----------------|--------|
| `max_tokens` | `num_predict` | Maximum tokens to generate |
| `temperature` | `temperature` | Same range (0.0-2.0) |
| `top_p` | `top_p` | Nucleus sampling |
| `stop` | `stop` | Stop sequences |
| `presence_penalty` | Approximated via `repeat_penalty` | Not exact mapping |
| `frequency_penalty` | Approximated via `repeat_penalty` | Not exact mapping |
| `n` | Not supported | Always returns 1 choice |
| `logprobs` | Not supported | No token probabilities |

## Limitations

The compatibility layer has some limitations:

### Not Supported
- Function calling / Tools
- Response format (JSON mode)
- Multiple choices (`n` parameter)
- Token usage details in response
- Logprobs
- Fine-tuning endpoints

### Behavioral Differences
- Authentication is ignored (any API key works)
- Model names must match locally available models
- No token usage tracking
- Streaming format matches OpenAI but performance metrics are omitted

## Migration Examples

### From OpenAI Python
```python
# Before (OpenAI)
import openai
client = openai.OpenAI(api_key="sk-...")
response = client.chat.completions.create(
    model="gpt-3.5-turbo",
    messages=[{"role": "user", "content": "Hello"}]
)

# After (Ollama)
import openai
client = openai.OpenAI(
    api_key="ollama",
    base_url="http://localhost:11434/v1"
)
response = client.chat.completions.create(
    model="llama3",  # Use local model name
    messages=[{"role": "user", "content": "Hello"}]
)
```

### From LangChain
```python
# Before
from langchain_openai import ChatOpenAI
llm = ChatOpenAI(model="gpt-3.5-turbo")

# After
from langchain_openai import ChatOpenAI
llm = ChatOpenAI(
    model="llama3",
    base_url="http://localhost:11434/v1",
    api_key="ollama"
)
```

## Advanced Usage

### Custom Headers and Timeout
```rust
use async_openai::config::{OpenAIConfig, OPENAI_API_BASE};
use std::time::Duration;

let config = OpenAIConfig::new()
    .with_api_base("http://localhost:11434/v1")
    .with_api_key("ollama")
    .with_timeout(Duration::from_secs(60)); // Longer timeout for large models

let client = Client::with_config(config);
```

### Error Handling
```rust
use async_openai::error::OpenAIError;

match client.chat().create(request).await {
    Ok(response) => {
        // Process response
    }
    Err(OpenAIError::ApiError(err)) => {
        eprintln!("API Error: {} (Code: {:?})", err.message, err.code);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

## Best Practices

1. **Always check model availability** - The model must be pulled locally first
2. **Use appropriate timeouts** - Local inference can be slower than cloud APIs
3. **Handle streaming properly** - Useful for better UX with slower models
4. **Set reasonable max_tokens** - Prevents runaway generation
5. **Test parameter mappings** - Some OpenAI parameters approximate differently

## Debugging Tips

### Enable Request Logging
```rust
use tracing_subscriber;

// Enable debug logging
tracing_subscriber::fmt::init();

// Now async-openai will log requests/responses
```

### Check Ollama Logs
```bash
# View Ollama server logs
journalctl -u ollama -f  # Linux systemd
ollama serve 2>&1 | tee ollama.log  # Manual run
```

### Common Issues

**Model not found:**
```json
{
  "error": {
    "message": "model 'gpt-3.5-turbo' not found",
    "type": "invalid_request_error"
  }
}
```
Solution: Use `ollama list` to see available models

**Timeout errors:**
- Increase client timeout for larger models
- Consider streaming for long responses
- Check if model is already loaded (`keep_alive` parameter)