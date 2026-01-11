---
name: rig
description: Comprehensive guide to the Rig crate for building LLM-powered applications in Rust
created: 2025-12-19
last_updated: 2025-12-19T00:00:00Z
hash: 80c159883a80a75d
tags:
  - rust
  - llm
  - ai
  - agents
  - rag
  - embeddings
  - vector-stores
---

# Rig: Building LLM-Powered Applications in Rust

## Overview

**Rig** is a Rust library for building scalable, modular, and ergonomic LLM-powered applications. It provides a unified interface for working with 20+ model providers and 10+ vector stores, with a focus on type safety, performance, and minimal boilerplate. The core crate is `rig-core`, with a growing ecosystem of companion crates for specific integrations.

### Core Philosophy

- **Type Safety**: Leverage Rust's type system for compile-time correctness in LLM interactions
- **Unified API**: Consistent interface across different providers to reduce vendor lock-in
- **Modular Design**: Compose agents, vector stores, and tools in a flexible pipeline architecture
- **Performance**: Async-first design with zero-cost abstractions
- **WASM Compatibility**: Core library works in WebAssembly environments

### Why Rig?

Rig excels at handling the complexities of LLM-powered applications by providing:

- **Tool calling with full type safety**: Automatic JSON schema generation via `schemars`, eliminating hand-written schemas
- **RAG (Retrieval-Augmented Generation)**: Seamless integration with vector stores for context-aware responses
- **Multi-turn conversations**: Built-in memory management via `ChatHistory` trait
- **Agentic workflows**: Support for complex tool chaining and autonomous decision-making
- **Provider flexibility**: Switch between OpenAI, Anthropic, Cohere, and others with minimal code changes

---

## Table of Contents

1. [Installation & Setup](#installation--setup)
2. [Core Architecture](#core-architecture)
3. [Building Your First Agent](#building-your-first-agent)
4. [Tool Calling](#tool-calling)
5. [Vector Embeddings](#vector-embeddings)
6. [RAG (Retrieval-Augmented Generation)](#rag-retrieval-augmented-generation)
7. [Memory & Conversation State](#memory--conversation-state)
8. [Agentic Loops & Tool Chaining](#agentic-loops--tool-chaining)
9. [Provider Integrations](#provider-integrations)
10. [Vector Store Integrations](#vector-store-integrations)
11. [Advanced Features](#advanced-features)
12. [Creating Custom Providers](#creating-custom-providers)
13. [Best Practices](#best-practices)
14. [Production Use Cases](#production-use-cases)

---

## Installation & Setup

### Basic Installation

```bash
# Core library with derive macros
cargo add rig-core --features derive

# Add Tokio runtime
cargo add tokio --features macros,rt-multi-thread

# Add specific provider (e.g., OpenAI)
cargo add rig-core -F openai
```

### Setting Up API Keys

```bash
# For OpenAI
export OPENAI_API_KEY="sk-..."

# For Anthropic
export ANTHROPIC_API_KEY="sk-ant-..."

# For Cohere
export COHERE_API_KEY="..."
```

### Minimal Example

```rust
use rig::{completion::Prompt, providers::openai};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let client = openai::Client::from_env();
    let gpt4 = client.agent("gpt-4").build();

    let response = gpt4.prompt("Who are you?").await?;
    println!("GPT-4: {}", response);

    Ok(())
}
```

---

## Core Architecture

### 1. Provider Clients

Each LLM provider (OpenAI, Anthropic, Cohere, etc.) has a `Client` struct that serves as a factory for creating completion and embedding models.

```rust
let openai_client = openai::Client::from_env();
let anthropic_client = anthropic::Client::from_env();
```

### 2. Model Traits

Two fundamental traits provide low-level interfaces:

- **`CompletionModel`**: For text generation and chat completions
- **`EmbeddingModel`**: For generating vector embeddings

These traits define contracts that all providers must implement, ensuring consistency across the ecosystem.

### 3. Agents

Agents are high-level abstractions that combine models with:

- **Preamble**: System prompts that define agent behavior
- **Configuration**: Temperature, max tokens, and other generation parameters
- **Context Management**: Static and dynamic context injection
- **Tool Integration**: Function calling capabilities
- **Memory**: Multi-turn conversation support via `ChatHistory`

### 4. Vector Stores

Common interface via `VectorStoreIndex` trait for similarity search and retrieval, enabling RAG patterns. Rig supports both in-memory stores for development and production-grade databases like MongoDB, Qdrant, and LanceDB.

### 5. Tools

Extend agent capabilities with structured function calling. Tools can be:

- **Static Functions**: Deterministic operations (e.g., calculators, formatters)
- **RAG-Enabled**: Retrieved from vector stores
- **Dynamically Loaded**: From MCP (Model Context Protocol) servers
- **External APIs**: Weather, databases, or any HTTP service

---

## Building Your First Agent

### Simple Agent with Preamble

```rust
use rig::providers::openai;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let client = openai::Client::from_env();

    let agent = client
        .agent("gpt-4o")
        .preamble("You are a helpful assistant that speaks like a pirate.")
        .build();

    let response = agent.prompt("Tell me about Rust").await?;
    println!("{}", response);

    Ok(())
}
```

### Agent with Configuration

```rust
let agent = client
    .agent("gpt-4o")
    .preamble("You are a technical writing assistant.")
    .temperature(0.3)  // Lower temperature for more consistent output
    .max_tokens(500)   // Limit response length
    .build();
```

### Streaming Responses

```rust
use rig::streaming::{StreamingPrompt, StreamingCompletionModel};

let model = client.completion_model("gpt-4").await?;

let mut stream = model
    .stream_prompt("Write a short story about Rust")
    .await?;

while let Some(chunk) = stream.next().await {
    print!("{}", chunk?);
}
```

---

## Tool Calling

Tool calling is where Rig truly shines, leveraging Rust's type system to ensure that the LLM receives well-structured schemas and returns valid data.

### How Tool Calling Works

In Rig, a tool is defined by implementing the `Tool` trait, which requires:

1. **A Definition**: Name and description for the LLM
2. **An Input Type**: A struct that implements `serde::Deserialize` and `JsonSchema`

### Creating a Simple Tool

```rust
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// 1. Define the arguments
#[derive(Deserialize, JsonSchema, Serialize)]
pub struct AddArgs {
    /// The first number to add
    pub a: i32,
    /// The second number to add
    pub b: i32,
}

// 2. Define the Tool
pub struct Adder;

impl Tool for Adder {
    const NAME: &'static str = "add_numbers";

    type Error = std::io::Error;
    type Args = AddArgs;
    type Output = i32;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Adds two integers together".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(AddArgs)).unwrap(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(args.a + args.b)
    }
}
```

### Using the Tool with an Agent

```rust
let agent = client
    .agent("gpt-4o")
    .preamble("You are a helpful assistant that can perform math.")
    .tool(Adder)
    .build();

let response = agent.prompt("What is 1234 plus 5678?").await?;
println!("{}", response);  // The LLM calls the tool and returns the result
```

### Key Tool Calling Concepts

- **JsonSchema Trait**: Critical for automatic schema generation. Without it, Rig cannot tell the LLM what the "shape" of your data looks like
- **Descriptions Matter**: Docstrings (e.g., `/// The first number`) and the `description` field guide the LLM on when to call your tool
- **Type Safety**: If the LLM tries to send invalid data (e.g., a string where an integer is expected), deserialization fails before the `call` method executes, preventing runtime errors

### Tools with External State

Often, tools need access to API keys or database connections:

```rust
#[derive(Deserialize, JsonSchema, Serialize)]
pub struct WeatherArgs {
    /// The city to get the weather for
    pub city: String,
}

pub struct WeatherTool {
    pub api_key: String,
}

impl Tool for WeatherTool {
    const NAME: &'static str = "get_weather";

    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Args = WeatherArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get the current weather for a specific city".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(WeatherArgs)).unwrap(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // In production, use reqwest to call a weather API
        Ok(format!("The weather in {} is currently 22°C and sunny.", args.city))
    }
}
```

### Multi-Tool Orchestration

When you build an agent with multiple tools, Rig constructs a prompt that includes descriptions of all available tools. The LLM then intelligently chooses which tool to use:

```rust
let agent = client
    .agent("gpt-4o")
    .preamble("You are a helpful research assistant.")
    .tool(Adder)
    .tool(WeatherTool { api_key: "abc-123".into() })
    .build();

// The LLM chooses 'add_numbers'
let math_res = agent.prompt("What is 50 + 50?").await?;

// The LLM chooses 'get_weather'
let weather_res = agent.prompt("Should I wear a jacket in London?").await?;
```

### Error Handling and Validation

One hidden complexity of tool calling is when the LLM hallucinates arguments:

- **Rig's Validation**: Because Rig uses `serde` and `schemars`, it validates the LLM's JSON input before your tool's `call` method is invoked
- **Execution Errors**: If your `call` method returns an `Err`, Rig passes that error message back to the LLM, which often self-corrects by asking for clarification or trying a different tool

### Comparison: Rig vs. Manual Tool Calling

| Feature | Manual Implementation | Using Rig |
|---------|---------------------|-----------|
| **Schema Generation** | Hand-written JSON strings | Auto-generated via `JsonSchema` |
| **Type Safety** | Manual parsing of JSON | Automated via `serde` |
| **Tool Selection** | Complex `if/else` logic | Handled by Agent builder |
| **Async Support** | Manual boilerplate | Natively `async` |

---

## Vector Embeddings

Vector embeddings are numerical representations of data in a continuous, low-dimensional vector space where semantically similar items are mapped to nearby points. Rig provides a comprehensive framework for working with embeddings.

### The `EmbeddingModel` Trait

The heart of Rig's embedding system is the `EmbeddingModel` trait:

```rust
pub trait EmbeddingModel: WasmCompatSend + WasmCompatSync {
    /// The maximum number of documents that can be embedded in a single request
    const MAX_DOCUMENTS: usize;

    type Client;

    fn make(client: &Self::Client, model: impl Into<String>, dims: Option<usize>) -> Self;

    /// The number of dimensions in the embedding vector
    fn ndims(&self) -> usize;

    /// Embed multiple text documents in a single request
    fn embed_texts(
        &self,
        texts: impl IntoIterator<Item = String> + WasmCompatSend,
    ) -> impl std::future::Future<Output = Result<Vec<Embedding>, EmbeddingError>> + WasmCompatSend;

    /// Embed a single text document
    fn embed_text(
        &self,
        text: &str,
    ) -> impl std::future::Future<Output = Result<Embedding, EmbeddingError>> + WasmCompatSend;
}
```

**Key aspects:**
- **MAX_DOCUMENTS**: Provider's batch size limit
- **ndims()**: Returns dimensionality of vectors (e.g., 1536 for OpenAI's text-embedding-3-small)
- **embed_texts()**: Batch embedding method
- **embed_text()**: Single document embedding

### The `Embedding` Struct

```rust
#[derive(Clone, Default, Deserialize, Serialize, Debug)]
pub struct Embedding {
    /// The document that was embedded (for debugging)
    pub document: String,
    /// The embedding vector
    pub vec: Vec<f64>,
}
```

### The `Embed` Trait

To make custom types embeddable, implement the `Embed` trait. Rig provides a derive macro:

```rust
use rig::Embed;

#[derive(Embed, Serialize, Clone)]
struct WordDefinition {
    word: String,
    #[embed]  // This field will be embedded
    definition: String,
}
```

**Manual implementation** (for custom logic):

```rust
use std::fmt;

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Record {
    first_name: String,
    last_name: String,
    email: String,
    role: String,
    salary: u32,
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "First name: {}\nLast name: {}\nEmail: {}\nRole: {}\nSalary: {}",
            self.first_name, self.last_name, self.email, self.role, self.salary
        )
    }
}

impl rig::embeddings::Embed for Record {
    fn embed(
        &self,
        embedder: &mut rig::embeddings::TextEmbedder,
    ) -> Result<(), rig::embeddings::EmbedError> {
        Ok(embedder.embed(self.to_string()))
    }
}
```

**Multiple embedded fields**:

```rust
#[derive(Embed, Clone, Deserialize, Debug, Serialize)]
struct WordDefinition {
    id: String,
    word: String,
    #[embed]
    definitions: Vec<String>, // All definitions will be embedded
}
```

### Working with OpenAI Embeddings

```rust
use rig::{embeddings::EmbeddingsBuilder, providers::openai};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = openai::Client::from_env();
    let model = client.embedding_model(openai::TEXT_EMBEDDING_3_SMALL);

    println!("Embedding dimensions: {}", model.ndims()); // 1536

    #[derive(rig::Embed, serde::Serialize)]
    struct WordDefinition {
        word: String,
        #[embed]
        definition: String,
    }

    let words = vec![
        WordDefinition {
            word: "flurbo".to_string(),
            definition: "A fictional currency from Rick and Morty.".to_string(),
        },
    ];

    let embeddings = EmbeddingsBuilder::new(model.clone())
        .documents(words)?
        .build()
        .await?;

    Ok(())
}
```

### Local Embeddings with FastEmbed

For offline/self-hosted embeddings:

```rust
use rig::Embed;
use rig_fastembed::FastembedModel;

#[derive(Embed, Clone, serde::Deserialize, Debug, serde::Serialize)]
struct WordDefinition {
    id: String,
    word: String,
    #[embed]
    definitions: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let fastembed_client = rig_fastembed::Client::new();

    // Use quantized model for efficiency
    let embedding_model = fastembed_client.embedding_model(
        &FastembedModel::AllMiniLML6V2Q
    );

    let documents = vec![
        WordDefinition {
            id: "doc0".to_string(),
            word: "flurbo".to_string(),
            definitions: vec![
                "A green alien that lives on cold planets.".to_string(),
                "A fictional digital currency from Rick and Morty.".to_string()
            ]
        },
    ];

    let embeddings = EmbeddingsBuilder::new(embedding_model.clone())
        .documents(documents)?
        .build()
        .await?;

    Ok(())
}
```

### The `EmbeddingsBuilder`

The `EmbeddingsBuilder` handles batch embedding operations efficiently:

```rust
let embeddings = EmbeddingsBuilder::new(embedding_model)
    .documents(documents)?
    .build()
    .await?;
```

The builder:
- Respects provider batch size limits automatically
- Handles concurrent processing
- Returns an iterator over `(Embedding, T)` tuples
- Provides error handling for the entire batch

---

## RAG (Retrieval-Augmented Generation)

RAG moves beyond simple function calling into the world of **Vector Stores**. In this pattern, the tool searches a mathematical "embedding space" to find relevant context from your documents.

### RAG Architecture in Rig

A RAG tool consists of:

1. **An Embedding Model**: Converts text queries into vectors
2. **A Vector Store**: Database holding document vectors (MongoDB, Qdrant, or in-memory)
3. **A Vector Store Index**: Performs similarity search

### Basic RAG Implementation

```rust
use rig::providers::openai;
use rig::vector_store::{in_memory_store::InMemoryVectorStore, VectorStoreIndex};
use rig::embeddings::Embed;
use serde::{Deserialize, Serialize};

#[derive(Embed, Clone, Debug, Serialize, Deserialize)]
struct Document {
    id: String,
    #[embed]
    content: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let openai_client = openai::Client::from_env();

    // Create embedding model
    let embedding_model = openai_client.embedding_model("text-embedding-ada-002");

    // Create vector store
    let mut store = InMemoryVectorStore::default();

    // Create index
    let index = store.index(embedding_model).await?;

    // Add documents
    let docs = vec![
        Document {
            id: "1".to_string(),
            content: "Rust is a systems programming language".to_string(),
        },
        Document {
            id: "2".to_string(),
            content: "Rig is a Rust library for LLM applications".to_string(),
        },
    ];

    index.add_documents(docs).await?;

    // Create RAG agent
    let agent = openai_client
        .agent("gpt-4")
        .preamble("You are a helpful assistant using the provided context.")
        .dynamic_context(3, index)  // Use top 3 similar documents as context
        .build();

    let response = agent.prompt("What is Rig?").await?;
    println!("Response: {}", response);

    Ok(())
}
```

### Building a Search Tool

```rust
use rig::{
    embeddings::EmbeddingsBuilder,
    providers::openai::{Client, TEXT_EMBEDDING_3_SMALL},
    vector_store::in_memory_store::InMemoryVectorStore,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env();

    // 1. Initialize the embedding model
    let model = client.embedding_model(TEXT_EMBEDDING_3_SMALL);

    // 2. Create and populate an in-memory vector store
    let mut vector_store = InMemoryVectorStore::default();

    let entries = vec![
        "The company's vacation policy allows for 25 days of PTO.",
        "The office is closed on all bank holidays.",
    ];

    let embeddings = EmbeddingsBuilder::new(model.clone())
        .documents(entries)?
        .build()
        .await?;

    vector_store.add_documents(embeddings).await?;

    // 3. Create an Index from the store
    let index = vector_store.index(model);

    // 4. Convert the Index into a Tool
    let search_tool = index.definition(
        "policy_search",
        "Search the company employee handbook for HR policies"
    ).await;

    // 5. Build the Agent
    let agent = client
        .agent("gpt-4o")
        .preamble("You are an HR assistant. Use the search tool to answer questions.")
        .dynamic_tool(search_tool)
        .build();

    let response = agent.prompt("How many vacation days do I get?").await?;
    println!("HR Bot: {}", response);

    Ok(())
}
```

### Why Use Dynamic Tools for Vector Stores?

- **Automatic Querying**: When the LLM decides to use the search tool, Rig automatically generates an embedding, searches the vector store, and returns the most relevant text snippets
- **Scalability**: While the example uses `InMemoryVectorStore`, Rig supports production-grade stores like MongoDB or LanceDB. You can swap the storage engine without changing agent logic

### Comparison: Function vs. RAG Tools

| Feature | Function Tool (e.g., Calculator) | RAG Tool (Vector Search) |
|---------|----------------------------------|-------------------------|
| **Data Source** | Hardcoded logic or Live API | Unstructured text/documents |
| **Logic** | Deterministic (1+1 is always 2) | Probabilistic (similarity match) |
| **Best For** | Actions, Calculations, Real-time data | Knowledge bases, FAQs, Research |
| **Setup** | Requires defining a Struct | Requires an Embedding Model |

---

## Memory & Conversation State

To make an agent truly useful in production, it needs **Memory**. Without it, every tool call and conversation starts from a blank slate.

### The ChatHistory Trait

In Rig, memory is handled via the `ChatHistory` trait. While the agent manages tool calling logic, the history stores the "transcript" of the conversation, including tool requests and results.

### Interaction Flow with Memory

When an agent uses memory, the sequence includes **Context Injection**. Before the LLM decides which tool to call, it reviews previous messages to see if the answer already exists or if it needs to follow up on a previous tool result.

### Implementing Sessions

```rust
use rig::agent::Session;
use rig::providers::openai;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = openai::Client::from_env();

    // 1. Initialize agent with tools
    let agent = client
        .agent("gpt-4o")
        .preamble("You are a helpful assistant.")
        .tool(Adder)
        .build();

    // 2. Create a Session
    // In a web app, load the history for a specific 'session_id' from your DB
    let mut session = agent.context(vec![]);

    // 3. First interaction: Agent calls a tool and remembers it
    let res1 = session.prompt("Add 5 and 10 for me.").await?;
    println!("Response 1: {}", res1);

    // 4. Second interaction: The agent knows the previous result
    // It doesn't need to call the tool again
    let res2 = session.prompt("Now multiply that result by 2.").await?;
    println!("Response 2: {}", res2);

    // 5. To persist, extract the history and save it to your DB
    let history = session.history();
    // save_to_db(history).await?;

    Ok(())
}
```

### Why Memory is Crucial for Tool Calling

Without memory, tool calling becomes brittle:

- **Context Loss**: If the user says "Search for that again," the agent won't know what "that" refers to
- **Redundant Calls**: The agent might call an expensive API multiple times for the same information
- **Correction**: If a tool call fails (e.g., database timeout), the agent needs to see that failure in its history to try a different approach

### Memory Strategy Comparison

| Strategy | Performance | Persistence | Best Use Case |
|----------|------------|-------------|---------------|
| **No Memory** | Fastest | None | Simple one-off commands (e.g., "Translate this") |
| **In-Memory** | Fast | Lost on Restart | CLI tools or short-lived scripts |
| **Database (JSON)** | Slower | Permanent | Web apps, Slack bots, customer support agents |
| **Vector-based Memory** | Slowest | Permanent | "Long-term" memory where agent recalls things from weeks ago |

---

## Agentic Loops & Tool Chaining

In Rig, tool chaining is handled by the **Agentic Loop**. Instead of a single request-response cycle, the agent enters a loop where it can call a tool, observe the result, and decide if it needs to call another tool or provide a final answer.

### The Execution Loop

When you use a high-level `Agent` in Rig, the loop is largely managed for you. If the agent determines that the output of Tool A is necessary to fulfill the requirements of Tool B, it will execute them sequentially.

### Sequential Tool Dependencies

```rust
#[derive(Deserialize, JsonSchema, Serialize)]
pub struct SearchArgs { pub query: String }

#[derive(Deserialize, JsonSchema, Serialize)]
pub struct GetDetailsArgs { pub user_id: String }

// Tool 1: Searches for a user and returns an ID
pub struct UserSearch;
impl Tool for UserSearch {
    /* ... definition ... */
    async fn call(&self, args: SearchArgs) -> Result<String, Self::Error> {
        // Logic: "John Doe" -> "user_123"
        Ok("user_123".to_string())
    }
}

// Tool 2: Takes an ID and returns specific details
pub struct UserDetails;
impl Tool for UserDetails {
    /* ... definition ... */
    async fn call(&self, args: GetDetailsArgs) -> Result<String, Self::Error> {
        Ok(format!("Details for {}: Email is john@example.com", args.user_id))
    }
}
```

### Running the Chain

When you provide both tools to the agent, the LLM recognizes the dependency. If you ask, *"What is John Doe's email?"*, the agent follows this sequence:

1. **Call `UserSearch`** with `{"query": "John Doe"}`
2. **Receive** `"user_123"`
3. **Call `UserDetails`** with `{"user_id": "user_123"}`
4. **Final Answer**: "John Doe's email is john@example.com."

### Key Strategies for Chaining

- **Preamble Tuning**: Use the agent's preamble to explain the workflow. Example: "First find the ID using search, then fetch details."
- **Structured Output**: Use the `completion` API if you need the agent to return data in a specific JSON format after the chain finishes

### Tool Calling Summary

| Concept | Rig Implementation |
|---------|-------------------|
| **Schema Generation** | `schemars::JsonSchema` on the Args struct |
| **Tool Registry** | The `.tool()` method on the Agent builder |
| **Execution** | Automatic via the agent's internal loop |
| **Error Feedback** | Errors from `call()` are sent back to the LLM as "observations" |

---

## Provider Integrations

### Native Providers (in `rig-core`)

- **OpenAI**: GPT models, embeddings
- **Anthropic**: Claude models
- **Cohere**: Command models
- **Perplexity**: Llama models
- **Google Gemini**: Gemini models
- **xAI**: Grok models
- **DeepSeek**: DeepSeek models

### Companion Provider Crates

- **`rig-bedrock`**: AWS Bedrock integration
- **`rig-eternalai`**: Eternal AI (decentralized inference)
- **`rig-vertexai`**: Google Vertex AI
- **`rig-fastembed`**: Local embedding models via FastEmbed

### Using Anthropic

```rust
use rig::providers::anthropic;

let client = anthropic::Client::from_env();
let model = client.completion_model("claude-3-opus-20240229").await?;
let response = model.prompt("Hello!").await?;
```

### Switching Providers

One of Rig's strengths is the ability to switch providers with minimal code changes:

```rust
// OpenAI
let openai_client = openai::Client::from_env();
let agent = openai_client.agent("gpt-4").build();

// Anthropic (same agent interface!)
let anthropic_client = anthropic::Client::from_env();
let agent = anthropic_client.agent("claude-3-opus-20240229").build();
```

---

## Vector Store Integrations

### In-Memory (Built-in)

- **`InMemoryVectorStore`**: Zero-dependency, RAM-based storage
- Ideal for development/testing
- Thread-safe for reads, exclusive access for writes

```rust
use rig::vector_store::in_memory_store::InMemoryVectorStore;

let vector_store = InMemoryVectorStore::default();
vector_store.add_documents(embeddings).await?;

let index = vector_store.index(embedding_model);
let results = index.top_n_from_query("What is Rust?", 5).await?;
```

### Companion Vector Store Crates

- **`rig-mongodb`**: MongoDB integration
- **`rig-lancedb`**: LanceDB (persistent, disk-based)
- **`rig-neo4j`**: Neo4j graph database
- **`rig-qdrant`**: Qdrant vector database
- **`rig-surrealdb`**: SurrealDB multi-model database
- **`rig-sqlite`**: SQLite with vector extension
- **`rig-milvus`**: Milvus vector database
- **`rig-scylladb`**: ScyllaDB
- **`rig-s3vectors`**: AWS S3Vectors
- **`rig-helixdb`**: HelixDB

### Using LanceDB

```rust
use rig_lancedb::{LanceDbVectorStore, Connection};

let db = Connection::open("/path/to/db").await?;
let table = db.create_table("docs", schema).await?;

let vector_store = LanceDbVectorStore::new(table);
let index = vector_store.index(embedding_model).await?;
```

### Using SurrealDB

```rust
use rig_surrealdb::{SurrealVectorStore, Mem};
use surrealdb::Surreal;

let db = Surreal::new::<Mem>(()).await?;
db.use_ns("ns").use_db("db").await?;

let vector_store = SurrealVectorStore::with_defaults(model.clone(), db);
vector_store.insert_documents(documents).await?;
```

---

## Advanced Features

### Pipeline API

```rust
use rig::pipeline::{Pipeline, Op};

let pipeline = Pipeline::new()
    .add(Op::from(load_documents))
    .add(Op::from(chunk_documents))
    .add(Op::from(generate_embeddings))
    .add(Op::from(store_in_vector_db));
```

### Multi-Agent Orchestration

```rust
use rig::agent::AgentSet;

let agents = AgentSet::new()
    .add("researcher", researcher_agent)
    .add("writer", writer_agent);

let result = agents
    .route("Analyze this data and write a report")
    .await?;
```

### MCP (Model Context Protocol) Integration

Enable with `rig-core/mcp` feature:

```rust
// Connect to MCP server
let transport = ClientSseTransportBuilder::new("http://localhost:3000/sse").build();
let mcp_client = ClientBuilder::new(transport).build();
mcp_client.open().await?;

// Add MCP tools to agent
let tools = mcp_client.list_tools(None, None).await?;
let agent = openai_client
    .agent("gpt-4")
    .mcp_tools(tools, mcp_client)
    .build();
```

### On-Chain Integration (`rig-onchain-kit`)

Execute blockchain operations via natural language:

```rust
use rig_onchain_kit::agent::create_solana_agent;
use rig_onchain_kit::signer::SignerContext;

SignerContext::with_signer(signer, async {
    let agent = create_solana_agent();
    let response = agent.prompt("Swap 0.1 SOL to USDC on Jupiter").await?;
}).await;
```

### Document Extraction

Extract structured data from unstructured text:

```rust
use rig::extractor::Extractor;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Person {
    name: String,
    age: u32,
}

let extractor = Extractor::new(openai_client, "gpt-4");
let text = "John Doe is 30 years old and works as a software engineer.";

let person: Person = extractor
    .extract("Extract person information", text)
    .await?;

println!("Name: {}, Age: {}", person.name, person.age);
```

---

## Creating Custom Providers

> **Note**: The following example demonstrates creating an OpenAI-compatible API provider. This is distinct from implementing the `EmbeddingModel` or `CompletionModel` traits for a new LLM provider. The code below shows how to build a server that exposes an OpenAI-compatible API using Rig's abstractions.

For API providers that offer OpenAI-compatible endpoints (like DeepSeek), you can create custom integrations. This is valuable for:

- **Vendor flexibility**: Switch between providers without rewriting client code
- **Cost optimization**: Choose the most cost-effective option
- **Redundancy**: Have backup providers when one is unavailable
- **Specialized models**: Access unique models while maintaining a consistent API

### Project Setup

```toml
# Cargo.toml
[package]
name = "my-openai-provider"
version = "0.1.0"
edition = "2024"

[dependencies]
rig = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
warp = "0.3"
reqwest = { version = "0.11", features = ["json"] }
anyhow = "1.0"
```

### Basic Server Architecture

```rust
use rig::{ProviderBuilder, OpenAIApi};
use warp::Filter;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider_config = ProviderConfig::from_env()?;

    let provider = ProviderBuilder::new()
        .with_config(provider_config)
        .with_backend(MyBackend::new())
        .build();

    let routes = OpenAIApi::new(provider)
        .routes()
        .with(warp::log("api"));

    println!("Starting server on 0.0.0.0:8080");
    warp::serve(routes)
        .run(([0, 0, 0, 0], 8080))
        .await;

    Ok(())
}
```

### Configuration Management

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    pub api_key: String,
    pub model_name: String,
    pub backend_url: String,
    pub max_tokens: u32,
    pub rate_limit: u32,
}

impl ProviderConfig {
    pub fn from_env() -> Result<Self> {
        Ok(ProviderConfig {
            api_key: std::env::var("PROVIDER_API_KEY")?,
            model_name: std::env::var("MODEL_NAME").unwrap_or_else(|_| "my-model".to_string()),
            backend_url: std::env::var("BACKEND_URL")?,
            max_tokens: std::env::var("MAX_TOKENS")
                .unwrap_or_else(|_| "2048".to_string())
                .parse()?,
            rate_limit: std::env::var("RATE_LIMIT")
                .unwrap_or_else(|_| "60".to_string())
                .parse()?,
        })
    }
}
```

### Implementing the Backend

```rust
use rig::{Backend, CompletionRequest, CompletionResponse};
use reqwest::Client;

pub struct MyBackend {
    client: Client,
    api_url: String,
    model_name: String,
}

#[async_trait::async_trait]
impl Backend for MyBackend {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        // Transform the OpenAI request to your backend's format
        let backend_request = json!({
            "prompt": request.prompt,
            "max_tokens": request.max_tokens.unwrap_or(100),
            "temperature": request.temperature.unwrap_or(0.7),
            "model": self.model_name
        });

        // Send the request to your backend
        let response = self.client
            .post(&format!("{}/complete", self.api_url))
            .json(&backend_request)
            .send()
            .await?;

        // Transform the backend response to OpenAI format
        // ... (see full example in source documents)

        Ok(completion_response)
    }
}
```

### Authentication Middleware

```rust
use warp::{Filter, Rejection};

pub fn with_auth(
    config: Arc<ProviderConfig>,
) -> impl Filter<Extract = (), Error = Rejection> + Clone {
    warp::header::optional::<String>("authorization")
        .and(warp::any().map(move || config.clone()))
        .and_then(|auth_header: Option<String>, config: Arc<ProviderConfig>| async move {
            match auth_header {
                Some(header) if header.starts_with("Bearer ") => {
                    let token = header.strip_prefix("Bearer ").unwrap();
                    if token == config.api_key {
                        Ok(())
                    } else {
                        Err(warp::reject::custom(AuthError::InvalidToken))
                    }
                }
                _ => Err(warp::reject::custom(AuthError::MissingToken)),
            }
        })
}
```

### Implementing a Custom EmbeddingModel

For integrating a new embedding provider directly with Rig:

```rust
use rig::embeddings::{EmbeddingModel, Embedding, EmbeddingError};

pub struct MyEmbeddingModel {
    client: MyEmbeddingClient,
    model_name: String,
    ndims: usize,
}

impl EmbeddingModel for MyEmbeddingModel {
    const MAX_DOCUMENTS: usize = 100;

    type Client = MyEmbeddingClient;

    fn make(client: &Self::Client, model: impl Into<String>, dims: Option<usize>) -> Self {
        Self {
            client: client.clone(),
            model_name: model.into(),
            ndims: dims.unwrap_or(512),
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

            // Validate batch size
            if texts.len() > Self::MAX_DOCUMENTS {
                return Err(EmbeddingError::ProviderError(
                    format!("Batch size {} exceeds maximum {}", texts.len(), Self::MAX_DOCUMENTS)
                ));
            }

            // Make API call and convert to Embeddings
            // ... (implementation details)

            Ok(embeddings)
        }
    }
}
```

---

## Best Practices

### 1. Context Management

- **Keep static context minimal**: Only include essential information that applies to all queries
- **Use dynamic context for large knowledge bases**: Let vector stores provide relevant context on-demand
- **Monitor token usage**: Context injection can quickly consume your token budget

### 2. Tool Design

- **Implement proper error handling**: Return descriptive error messages that the LLM can use to self-correct
- **Prefer static tools for core functionality**: Deterministic operations should use the `Tool` trait
- **Use descriptive names and documentation**: The LLM relies on these to choose tools correctly

### 3. Performance

- **Configure appropriate chunk sizes**: When processing documents, balance retrieval granularity with context window limits
- **Monitor token usage**: Track input/output tokens to optimize costs
- **Use batch operations**: `EmbeddingsBuilder` automatically batches requests efficiently

### 4. Error Handling

- **Use `anyhow` for flexibility**: Rig's error types integrate well with `anyhow::Error`
- **Rig provides specific error types**: Each module has its own error type (e.g., `EmbeddingError`, `CompletionError`)

### 5. Testing

- **Start with `InMemoryVectorStore` for development**: Simplifies setup and teardown
- **Use feature flags**: Conditionally compile provider code to reduce dependencies

### 6. Document Preparation

- **Clean text**: Remove irrelevant content and normalize whitespace before embedding
- **Chunking**: Split large documents into smaller chunks for better retrieval
- **Batch processing**: Use `EmbeddingsBuilder` for multiple documents

### 7. Provider-Specific Optimizations

- **OpenAI**: Use smaller models like `text-embedding-3-small` for cost savings
- **FastEmbed**: Use quantized models (e.g., `AllMiniLML6V2Q`) for faster local inference
- **Batch size**: Respect `EmbeddingModel::MAX_DOCUMENTS` to avoid failures

### 8. Type Safety with Derive Macros

Always enable the `derive` feature for cleaner code:

```toml
[dependencies]
rig-core = { version = "0.5", features = ["derive"] }
```

---

## Production Use Cases

Companies using Rig in production:

- **St Jude**: Genomics visualization chatbot
- **Coral Protocol**: Rust SDK components
- **VT Code**: Terminal coding agent
- **Dria**: Decentralized AI network compute nodes
- **Nethermind**: Neural Interconnected Nodes Engine
- **Neon**: App.build V2 platform
- **Listen**: AI portfolio management
- **Cairnify**: Intelligent document search

These applications demonstrate Rig's versatility across domains:

- Healthcare (genomics analysis)
- Developer tools (terminal agents)
- Blockchain (decentralized AI, smart contracts)
- Finance (portfolio management)
- Enterprise search (document intelligence)

---

## Module Reference

Based on the [docs.rs documentation](https://docs.rs/rig-core/latest/rig/), the crate exposes these key modules:

```rust
pub mod agent          // Agent implementation and builder
pub mod cli_chatbot    // CLI chatbot utilities
pub mod client         // Provider client traits
pub mod completion     // Completion request/response types
pub mod embeddings     // Embedding functionality and Embed trait
pub mod extractor      // Structured data extraction from text
pub mod loaders        // File loading and preprocessing
pub mod one_or_many    // Utility type for handling single or multiple values
pub mod pipeline       // Flexible pipeline API for operation sequences
pub mod prelude        // Common imports
pub mod providers      // LLM provider implementations
pub mod streaming      // Streaming completion support
pub mod tool           // Tool traits and structs
pub mod transcription  // Audio transcription models
pub mod vector_store   // Vector store interfaces
```

### Key Re-exports

```rust
pub use completion::message;      // Message types for completions
pub use embeddings::Embed;        // Trait for embeddable types
pub use one_or_many::{EmptyListError, OneOrMany}; // Utility types
```

---

## Companion Crates Ecosystem

### Core

- **`rig-core`**: Main library with base functionality

### Provider Extensions

- **`rig-bedrock`**: AWS Bedrock
- **`rig-eternalai`**: Decentralized AI inference
- **`rig-vertexai`**: Google Vertex AI
- **`rig-fastembed`**: Local embeddings

### Vector Stores

- **`rig-mongodb`**, **`rig-lancedb`**, **`rig-neo4j`**, **`rig-qdrant`**, **`rig-surrealdb`**, **`rig-sqlite`**, **`rig-milvus`**, **`rig-scylladb`**, **`rig-s3vectors`**, **`rig-helixdb`**

### Specialized

- **`rig-onchain-kit`**: Blockchain interactions (Solana/EVM)
- **`rig-tool-macro`**: Derive macros for tool creation

---

## Resources

- **Official Documentation**: [docs.rig.rs](https://docs.rig.rs)
- **GitHub Repository**: [github.com/0xPlaygrounds/rig](https://github.com/0xPlaygrounds/rig)
- **API Reference**: [docs.rs/rig-core](https://docs.rs/rig-core/latest/rig/)

Rig is actively developed with breaking changes expected as the API evolves. Check the official documentation and GitHub repository for the latest updates and migration guides.
