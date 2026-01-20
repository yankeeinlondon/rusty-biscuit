# Native API

Ollama's native API provides full control over model lifecycle and runtime parameters through `/api/*` endpoints.

## Core Endpoints

### Chat Completion
```http
POST /api/chat
```

**Request:**
```json
{
  "model": "llama3",
  "messages": [
    {"role": "user", "content": "Hello!"}
  ],
  "stream": false,
  "options": {
    "temperature": 0.7,
    "num_ctx": 4096,
    "num_predict": 100
  }
}
```

**Response:**
```json
{
  "model": "llama3",
  "created_at": "2024-01-01T00:00:00Z",
  "message": {
    "role": "assistant",
    "content": "Hello! How can I help you today?"
  },
  "done": true,
  "total_duration": 123456789,
  "prompt_eval_count": 10,
  "eval_count": 50
}
```

### Raw Generation
```http
POST /api/generate
```

For single-shot completions without conversation history:
```json
{
  "model": "llama3",
  "prompt": "Once upon a time",
  "stream": false,
  "options": {
    "num_predict": 50
  }
}
```

## Model Management

### List Models
```http
GET /api/tags
```

**Response:**
```json
{
  "models": [
    {
      "name": "llama3:latest",
      "size": 4150000000,
      "digest": "365c0bd3c000",
      "details": {
        "families": ["llama"],
        "parameter_size": "8B",
        "quantization_level": "Q4_0"
      }
    }
  ]
}
```

### Pull Model
```http
POST /api/pull
```

**Stream model download:**
```json
{
  "name": "mistral",
  "stream": true
}
```

The response is a stream of progress updates:
```json
{"status": "pulling manifest"}
{"status": "downloading sha256:abc123...", "completed": 1000, "total": 5000}
{"status": "verifying sha256 digest"}
{"status": "success"}
```

### Delete Model
```http
DELETE /api/delete
```

```json
{
  "name": "mistral:latest"
}
```

### Show Model Info
```http
POST /api/show
```

```json
{
  "name": "llama3"
}
```

Returns detailed model information including the Modelfile configuration.

## Performance Parameters

The `options` object supports Ollama-specific parameters not found in OpenAI:

### Context and Memory
- **`num_ctx`** (default: 2048): Context window size
- **`num_gpu`** (default: -1): Number of layers to offload to GPU (-1 = all)
- **`num_thread`** (default: auto): CPU threads to use

### Generation Control
- **`num_predict`** (default: 128): Maximum tokens to generate (like OpenAI's `max_tokens`)
- **`temperature`** (default: 0.8): Randomness (0.0-2.0)
- **`top_k`** (default: 40): Limit token selection pool
- **`top_p`** (default: 0.9): Nucleus sampling
- **`repeat_penalty`** (default: 1.1): Penalize repetition
- **`stop`**: Array of stop sequences

### Advanced Sampling
- **`mirostat`** (default: 0): Enable Mirostat sampling (0, 1, or 2)
- **`mirostat_eta`** (default: 0.1): Learning rate for Mirostat
- **`mirostat_tau`** (default: 5.0): Target perplexity

### Model Loading
- **`keep_alive`** (default: "5m"): Keep model in memory ("5m", "-1" for forever, "0" to unload immediately)

## Streaming Responses

Native streaming uses Server-Sent Events with a different format than OpenAI:

### Rust Streaming Example
```rust
use futures::stream::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
struct StreamChunk {
    message: Option<MessageContent>,
    done: bool,
    // Performance metrics in final chunk
    total_duration: Option<u64>,
    eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct MessageContent {
    content: String,
    role: String,
}

async fn stream_chat() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let body = json!({
        "model": "llama3",
        "messages": [{"role": "user", "content": "Tell me a story"}],
        "stream": true,
        "options": {
            "temperature": 0.8,
            "num_ctx": 4096
        }
    });

    let mut stream = client
        .post("http://localhost:11434/api/chat")
        .json(&body)
        .send()
        .await?
        .bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item?;
        let text = String::from_utf8_lossy(&chunk);

        // Parse each line as JSON
        for line in text.lines() {
            if line.is_empty() { continue; }

            if let Ok(chunk) = serde_json::from_str::<StreamChunk>(line) {
                if let Some(msg) = chunk.message {
                    print!("{}", msg.content);
                    std::io::stdout().flush()?;
                }

                if chunk.done {
                    println!("\n\nPerformance:");
                    if let Some(duration) = chunk.total_duration {
                        println!("Total duration: {}ms", duration / 1_000_000);
                    }
                    if let Some(tokens) = chunk.eval_count {
                        println!("Tokens generated: {}", tokens);
                    }
                }
            }
        }
    }

    Ok(())
}
```

## Embeddings

Generate embeddings for text:

```http
POST /api/embeddings
```

```json
{
  "model": "llama3",
  "prompt": "The capital of France is Paris"
}
```

**Response:**
```json
{
  "embedding": [0.123, -0.456, 0.789, ...] // Float array
}
```

## Error Handling

Common error responses:

### Model Not Found
```json
{
  "error": "model 'nonexistent' not found, try pulling it first"
}
```

### Invalid Parameters
```json
{
  "error": "num_ctx must be a positive integer"
}
```

### Server Overloaded
```json
{
  "error": "server busy, too many parallel requests"
}
```

## Best Practices

1. **Always specify `num_ctx`** based on your use case and available VRAM
2. **Use `keep_alive: "0"`** after requests if running multiple models
3. **Set `num_gpu`** appropriately for partial offloading on limited VRAM
4. **Handle streaming properly** - Native format differs from OpenAI
5. **Check model availability** with `/api/tags` before making requests
6. **Use appropriate sampling** - Mirostat for creative tasks, low temperature for factual

## Example: Model Lifecycle Management

```rust
use reqwest::Client;
use serde_json::json;

async fn ensure_model_available(model_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let client = Client::new();

    // 1. Check if model exists
    let response = client
        .get("http://localhost:11434/api/tags")
        .send()
        .await?;

    let tags: serde_json::Value = response.json().await?;
    let model_exists = tags["models"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .any(|m| m["name"].as_str() == Some(model_name));

    if !model_exists {
        println!("Model {} not found, pulling...", model_name);

        // 2. Pull the model
        let pull_body = json!({
            "name": model_name,
            "stream": false // Simplified for example
        });

        client
            .post("http://localhost:11434/api/pull")
            .json(&pull_body)
            .send()
            .await?;
    }

    Ok(true)
}
```