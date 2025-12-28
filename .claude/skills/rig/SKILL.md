---
name: rig
description: Expert knowledge for building LLM-powered applications with Rig, a Rust library that provides type-safe agents, tool calling, RAG patterns, vector store integration, and unified interfaces for 20+ model providers including OpenAI, Anthropic, Cohere, and Gemini
last_updated: 2025-12-19T00:00:00Z
hash: d6bb0fc1848ff7a5
---

# Rig: Rust LLM Application Framework

Rig is a Rust library for building scalable, modular, and ergonomic LLM-powered applications. It provides unified interfaces for working with 20+ model providers and 10+ vector stores, with a focus on type safety, performance, and minimal boilerplate.

## Core Philosophy

- **Type Safety**: Leverage Rust's type system for compile-time correctness in LLM interactions
- **Unified API**: Consistent interface across providers to reduce vendor lock-in
- **Modular Design**: Compose agents, vector stores, and tools in a flexible pipeline architecture
- **Performance**: Async-first design with zero-cost abstractions
- **WASM Compatibility**: Core library works in WebAssembly environments

## Quick Start

```rust
use rig::{completion::Prompt, providers::openai};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let openai_client = openai::Client::from_env();
    let gpt4 = openai_client.agent("gpt-4").build();

    let response = gpt4.prompt("Who are you?").await?;
    println!("GPT-4: {}", response);
    Ok(())
}
```

## Installation

```bash
# Core library with derive macros
cargo add rig-core --features derive

# Add Tokio runtime
cargo add tokio --features macros,rt-multi-thread

# Add specific provider
cargo add rig-core -F openai

# For blockchain functionality
cargo add rig-onchain-kit -F solana
```

## Core Concepts

### Provider Clients
Each LLM provider (OpenAI, Anthropic, Cohere, etc.) has a `Client` struct that serves as a factory for creating completion and embedding models.

### Model Traits
- **`CompletionModel`**: For text generation and chat completions
- **`EmbeddingModel`**: For generating vector embeddings

### Agents
High-level abstractions combining models with:
- System prompts (preamble)
- Configuration (temperature, max tokens)
- Context management (static & dynamic)
- Tool integration
- Multi-turn conversation support

### Vector Stores
Common interface via `VectorStoreIndex` trait for similarity search and retrieval, enabling RAG (Retrieval-Augmented Generation) patterns.

### Tools
Extend agent capabilities with structured function calling via the `Tool` trait.

## Topics

### Agent & Tool Patterns

- [Tool Calling](./tool-calling.md) - Implementing type-safe tools with the Tool trait
- [Multi-Tool Agents](./multi-tool-agents.md) - Orchestrating agents with multiple tools
- [Agentic Loops](./agentic-loops.md) - Tool chaining and sequential dependencies
- [Memory & Sessions](./memory.md) - Managing conversation history and context

### RAG & Vector Search

- [Vector Store Integration](./vector-stores.md) - RAG architecture and vector database usage
- [Embeddings](./embeddings.md) - Working with the EmbeddingModel trait and Embed derive macro

### Extensibility

- [Custom Providers](./custom-providers.md) - Creating OpenAI-compatible API providers

## Common Patterns

### Agent with Tools

```rust
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, JsonSchema, Serialize)]
pub struct AddArgs {
    pub a: i32,
    pub b: i32,
}

pub struct Adder;

impl Tool for Adder {
    const NAME: &'static str = "add_numbers";
    type Error = std::io::Error;
    type Args = AddArgs;
    type Output = i32;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Adds two integers together".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(AddArgs)).unwrap(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(args.a + args.b)
    }
}

// Use the tool
let agent = client
    .agent("gpt-4o")
    .tool(Adder)
    .build();
```

### RAG with Vector Store

```rust
use rig::embeddings::EmbeddingsBuilder;
use rig::vector_store::in_memory_store::InMemoryVectorStore;

let model = client.embedding_model("text-embedding-ada-002");
let mut store = InMemoryVectorStore::default();
let index = store.index(model.clone()).await?;

// Add documents
index.add_documents(docs).await?;

// Create RAG agent
let agent = client
    .agent("gpt-4")
    .dynamic_context(3, index)  // Top 3 similar docs
    .build();
```

## Supported Providers

### Native (in rig-core)
OpenAI, Anthropic, Cohere, Perplexity, Google Gemini, xAI, DeepSeek

### Companion Crates
- **rig-bedrock**: AWS Bedrock
- **rig-eternalai**: Decentralized inference
- **rig-vertexai**: Google Vertex AI
- **rig-fastembed**: Local embedding models

## Supported Vector Stores

### Built-in
- **InMemoryVectorStore**: Zero-dependency, RAM-based (dev/testing)

### Companion Crates
- **rig-mongodb**, **rig-lancedb**, **rig-neo4j**, **rig-qdrant**, **rig-surrealdb**, **rig-sqlite**, **rig-milvus**, **rig-scylladb**, **rig-s3vectors**, **rig-helixdb**

## Best Practices

1. **Context Management**: Keep static context minimal; use dynamic context for large knowledge bases
2. **Tool Design**: Implement proper error handling; prefer static tools for core functionality
3. **Performance**: Configure appropriate chunk sizes; monitor token usage; use batch operations
4. **Error Handling**: Use `anyhow` for flexibility; Rig provides specific error types per module
5. **Testing**: Start with `InMemoryVectorStore` for development; use feature flags to conditionally compile provider code

## Resources

- [Official Documentation](https://docs.rig.rs)
- [GitHub Repository](https://github.com/0xPlaygrounds/rig)
- [API Reference](https://docs.rs/rig-core/latest/rig/)
