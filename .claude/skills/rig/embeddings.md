# Vector Embeddings and the EmbeddingModel Trait

Vector embeddings are numerical representations of data in a continuous, low-dimensional vector space where semantically similar items are mapped to nearby points. Rig provides a comprehensive framework for working with embeddings in Rust.

## Core Components

### The EmbeddingModel Trait

The heart of Rig's embedding system:

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
    ) -> impl std::future::Future<Output = Result<Embedding, EmbeddingError>> + WasmCompatSend {
        async {
            Ok(self
                .embed_texts(vec![text.to_string()])
                .await?
                .pop()
                .expect("There should be at least one embedding"))
        }
    }
}
```

**Key aspects:**
- **MAX_DOCUMENTS**: Provider's batch size limit
- **ndims()**: Dimensionality of vectors (e.g., 1536 for OpenAI's text-embedding-3-small)
- **embed_texts()**: Batch embedding method
- **embed_text()**: Single document embedding (delegates to batch method)

### The Embedding Struct

```rust
#[derive(Clone, Default, Deserialize, Serialize, Debug)]
pub struct Embedding {
    /// The document that was embedded (for debugging)
    pub document: String,
    /// The embedding vector
    pub vec: Vec<f64>,
}
```

## Making Types Embeddable

### Using the Derive Macro

```rust
use rig::Embed;
use serde::{Deserialize, Serialize};

#[derive(Embed, Serialize, Clone)]
struct WordDefinition {
    word: String,
    #[embed]  // Mark field to embed
    definition: String,
}
```

### Manual Implementation

For custom logic:

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

### Embedding Multiple Fields

```rust
#[derive(Embed, Clone, Deserialize, Debug, Serialize)]
struct WordDefinition {
    id: String,
    word: String,
    #[embed]
    definitions: Vec<String>,  // All definitions will be embedded
}
```

## The EmbeddingsBuilder

Handles batch embedding operations efficiently:

```rust
use rig::embeddings::EmbeddingsBuilder;

let documents = vec![
    WordDefinition { /* ... */ },
    WordDefinition { /* ... */ },
];

let embeddings = EmbeddingsBuilder::new(embedding_model)
    .documents(documents)?
    .build()
    .await?;
```

The builder:
- Respects provider batch size limits automatically
- Handles concurrent processing
- Returns iterator over `(Embedding, T)` tuples
- Provides error handling for the entire batch

## Provider Integrations

### OpenAI

```rust
use rig::{embeddings::EmbeddingsBuilder, providers::openai};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client from environment (OPENAI_API_KEY)
    let client = openai::Client::from_env();

    // Create embedding model instance
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

    // Generate embeddings
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
use serde::{Deserialize, Serialize};

#[derive(Embed, Clone, Deserialize, Debug, Serialize)]
struct WordDefinition {
    id: String,
    word: String,
    #[embed]
    definitions: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Create FastEmbed client
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

    // Create in-memory vector store
    let vector_store = rig::vector_store::in_memory_store::InMemoryVectorStore::from_documents_with_id_f(
        embeddings,
        |doc| doc.id.clone()
    );

    // Create searchable index
    let index = vector_store.index(embedding_model);

    // Query the index
    let results = index
        .top_n::<WordDefinition>(
            "I need to buy something in a fictional universe. What type of money can I use?",
            1
        )
        .await?;

    for (score, id, doc) in results {
        println!("Score: {}, ID: {}, Word: {}", score, id, doc.word);
    }

    Ok(())
}
```

## Implementing a Custom EmbeddingModel Provider

```rust
use rig::embeddings::{EmbeddingModel, Embedding, EmbeddingError};

// Your custom client struct
pub struct MyEmbeddingClient {
    api_key: String,
    base_url: String,
    http_client: reqwest::Client,
}

// Your custom embedding model
pub struct MyEmbeddingModel {
    client: MyEmbeddingClient,
    model_name: String,
    ndims: usize,
}

impl EmbeddingModel for MyEmbeddingModel {
    const MAX_DOCUMENTS: usize = 100; // Provider-specific limit

    type Client = MyEmbeddingClient;

    fn make(client: &Self::Client, model: impl Into<String>, dims: Option<usize>) -> Self {
        let model_name = model.into();
        let ndims = dims.unwrap_or(512); // Default dimensionality

        Self {
            client: client.clone(),
            model_name,
            ndims,
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

            // Make API call to your provider
            let response = self.client.http_client
                .post(&format!("{}/embed", self.client.base_url))
                .header("Authorization", &format!("Bearer {}", self.client.api_key))
                .json(&serde_json::json!({
                    "model": self.model_name,
                    "input": texts
                }))
                .send()
                .await
                .map_err(EmbeddingError::HttpError)?;

            // Parse response and convert to Embeddings
            let embeddings: Vec<Vec<f64>> = response.json().await
                .map_err(EmbeddingError::JsonError)?;

            Ok(texts.into_iter().zip(embeddings)
                .map(|(document, vec)| Embedding { document, vec })
                .collect())
        }
    }
}
```

**Key implementation notes:**
1. **Wasm Compatibility**: Trait bounds ensure WebAssembly compatibility
2. **Error Handling**: Convert provider errors to `EmbeddingError` variants
3. **Batching**: Respect `MAX_DOCUMENTS` to avoid rate limits
4. **Async**: All operations are asynchronous

## Integration with Vector Stores

### In-Memory Store

```rust
use rig::vector_store::in_memory_store::InMemoryVectorStore;

let vector_store = InMemoryVectorStore::default();
vector_store.add_documents(embeddings).await?;

let index = vector_store.index(embedding_model);
let results = index.top_n_from_query("What is Rust?", 5).await?;
```

### SurrealDB

```rust
use rig_surrealdb::SurrealVectorStore;
use surrealdb::Surreal;
use surrealdb::engine::local::Mem;

let db = Surreal::new::<Mem>(()).await?;
db.use_ns("ns").use_db("db").await?;

let vector_store = SurrealVectorStore::with_defaults(model.clone(), db);
vector_store.insert_documents(embeddings).await?;
```

### RAG with Dynamic Context

```rust
// Create a RAG agent that uses the vector store for context
let rag_agent = client
    .agent(openai::GPT_4_1_NANO)
    .preamble("You are an expert assistant. Use the provided context.")
    .dynamic_context(3, vector_store) // Use top 3 relevant docs
    .build();

let response = rag_agent
    .prompt("When was Rust's first stable release?")
    .await?;
```

## Best Practices

### 1. Document Preparation

- **Clean text**: Remove irrelevant content and normalize whitespace
- **Chunking**: Split large documents into smaller chunks for better retrieval
- **Batch processing**: Use `EmbeddingsBuilder` for multiple documents

### 2. Error Handling

```rust
match embeddings_builder.build().await {
    Ok(docs) => { /* Process documents */ },
    Err(EmbeddingError::HttpError(e)) => {
        eprintln!("Network error: {}", e);
        // Implement retry logic
    },
    Err(EmbeddingError::ProviderError(msg)) => {
        eprintln!("Provider error: {}", msg);
        // Check API key, rate limits, etc.
    },
    Err(e) => eprintln!("Embedding failed: {}", e),
}
```

### 3. Provider Selection

- **OpenAI text-embedding-3-small**: Cost savings, good quality (1536 dims)
- **OpenAI text-embedding-3-large**: Better quality (3072 dims)
- **FastEmbed**: Use quantized models (e.g., `AllMiniLML6V2Q`) for local inference

### 4. Batch Size Management

```rust
// Respect MAX_DOCUMENTS to avoid failures
let batch_size = embedding_model.MAX_DOCUMENTS;

for chunk in documents.chunks(batch_size) {
    let embeddings = EmbeddingsBuilder::new(model.clone())
        .documents(chunk.to_vec())?
        .build()
        .await?;

    vector_store.add_documents(embeddings).await?;
}
```

### 5. Type Safety with Derive Macros

Always enable the `derive` feature:

```toml
[dependencies]
rig-core = { version = "0.5", features = ["derive"] }
```

## Performance Considerations

### Concurrent Processing

EmbeddingsBuilder handles concurrency automatically:

```rust
// Processes documents in batches respecting MAX_DOCUMENTS
// Utilizes async runtime for parallel execution
let embeddings = EmbeddingsBuilder::new(model)
    .documents(large_document_set)?
    .build()
    .await?;
```

### Caching Embeddings

For frequently queried documents:

```rust
use std::collections::HashMap;

struct EmbeddingCache {
    cache: HashMap<String, Embedding>,
}

impl EmbeddingCache {
    async fn get_or_create(
        &mut self,
        text: &str,
        model: &EmbeddingModel
    ) -> Result<Embedding, EmbeddingError> {
        if let Some(embedding) = self.cache.get(text) {
            return Ok(embedding.clone());
        }

        let embedding = model.embed_text(text).await?;
        self.cache.insert(text.to_string(), embedding.clone());
        Ok(embedding)
    }
}
```

## Summary

The `EmbeddingModel` trait in Rig provides:

- **Modularity**: Easy to implement custom providers
- **Type Safety**: Strong typing via `Embed` trait and derive macros
- **Performance**: Built-in batching and async support
- **Ecosystem**: Seamless integration with vector stores and RAG pipelines

## Related Topics

- [Vector Store Integration](./vector-stores.md) - Using embeddings for RAG
- [Tool Calling](./tool-calling.md) - Creating RAG-enabled tools
- [Custom Providers](./custom-providers.md) - Implementing custom embedding providers
