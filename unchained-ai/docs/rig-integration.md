# Rig Integration

Under the surface, we are using the `rig` crate (and ecosystem) to provide the details of implementation where we can.

Rig provides the following primitives:

## Clients

Provides an API surface to allow interaction with a given _provider_ (e.g., OpenAI, Anthropic, etc.).

  > Note: the `Client` was designed to be extensible to allow for you to create your own client providers.

**Rig** interacts with the following providers out-of-the-box:

- Primary Providers
    - Anthropic
    - Azure
    - Cohere
    - Deepseek
    - Google Gemini
    - Groq
    - Mira
    - Mistral
    - Moonshot
    - OpenAI
    - Perplexity
    - Together
    - Voyage AI
    - xAI
- Aggregators
    - HuggingFace
    - OpenRouter

In addition to this we have added the following providers:

- Primary Providers
    - Z.ai
- Aggregators
    - ZenMux


### Client Features

A Client can register support for any of the following (although text completion may be required):

1. **Text Completion** - able to call a model hosted by client provider for text completion
2. **Embeddings** - able to encode/decode via the client provider's embedding models
3. **Tool Calling** - allows for formal LLM tool calling which is

The table below shows the support provided by the built-in Clients:

| Provider       | Text Completion |   Embeddings | Tool Calling |
| -------------- | --------------: | -----------: | -----------: |
| OpenAI         |               ✅ |            ✅ |            ✅ |
| Azure OpenAI   |               ✅ |            ✅ |            ✅ |
| Anthropic      |               ✅ | ❌ (commonly) |            ✅ |
| Cohere         |               ✅ |            ✅ |            ✅ |
| Gemini         |               ✅ | ❌ (commonly) |            ✅ |
| Perplexity     |               ✅ |            ❌ |            ❌ |
| DeepSeek / xAI |               ✅ |            ❌ |            ❌ |

> For embeddings, if your provider doesn’t support them, you can pair it with another embedding provider or local embeddings (`rig-fastembed`).

## MCP

Rig can consume tools exposed by MCP servers (via `rmcp`) and register them with agents for tool calling.

**Note:** Rig does not “expose MCP servers as tools.” Rather:

- MCP servers expose tools, and
- Rig imports/bridges those MCP tools into the agent’s tool registry.


## Modalities

- The rig crate started by focusing on text input and text output
- but with modern versions there is growing support for both multi-modal inputs as well as additional output modalities
- `rig_core` as of version 0.28
    - **multi-modal inputs** are now available from some provider implementations
        - it's important to have good metadata which can tell what modalities are available for input
    - is now natively supporting image outputs
    - text-to-speech has support through the `rig::audio_generation` module which abstracts over a number of different providers
    - audio transcription (e.g., speech-to-text) is supported via the `rig::transcription` module.
    - the feature flag's `audio` and `image` are larger marker/organization currently but this is likely to change in future releases

### Example of Sending an Image to Anthropic

```rust
use rig::prelude::*;
use rig::completion::{Prompt, message::Image};
use rig::message::{ImageMediaType, DocumentSourceKind};
use rig::providers::anthropic;
use base64::{Engine as _, prelude::BASE64_STANDARD};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let client = anthropic::Client::from_env();

    let agent = client
        .agent(anthropic::completion::CLAUDE_3_5_SONNET)
        .preamble("You are a vision assistant. Describe images in detail.")
        .build();

    let image_bytes = reqwest::get("https://example.com/image.jpg").await?.bytes().await?;
    let b64_data = BASE64_STANDARD.encode(image_bytes);

    let image = Image {
        data: DocumentSourceKind::base64(b64_data),
        media_type: Some(ImageMediaType::JPEG),
        ..Default::default()
    };

    let response = agent.prompt(image).await?;
    println!("Description: {}", response);

    Ok(())
}
```

#### Example of sending both Text and Image in a single message

```rust
use rig::message::{Message, UserContent, Image, DocumentSourceKind, ImageMediaType};
use rig::{OneOrMany, providers::openai};

// ... inside async context with an `agent` ...
let response = agent
    .prompt(Message::User {
        content: OneOrMany::many(vec![
            UserContent::text("What is in this image?"),
            UserContent::Image(Image {
                data: DocumentSourceKind::url("https://example.com/cat.jpg".into()),
                media_type: Some(ImageMediaType::JPEG),
                ..Default::default()
            }),
        ])?
    })
    .await?;
```
