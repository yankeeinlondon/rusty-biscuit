# Vector Store Integration

To implement RAG (Retrieval-Augmented Generation) in Rig, you use **Vector Stores**. In this pattern, tools don't just calculate values—they search a mathematical "embedding space" to find relevant context from your own documents.

## The RAG Architecture

A RAG tool in Rig consists of:

1. **Embedding Model**: Converts text queries into vectors (lists of numbers)
2. **Vector Store**: Database holding your documents' vectors (MongoDB, Qdrant, in-memory, etc.)
3. **Vector Store Index**: Bridge that performs similarity search

## Basic RAG Implementation

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

    // In a real app, you'd load these from PDF, database, etc.
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

    // 4. Convert that Index into a Tool
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

## Why Use Dynamic Tools for Vector Stores?

In previous examples, we implemented the `Tool` trait manually. For vector stores, Rig provides a **Dynamic Tool** helper:

- **Automatic Querying**: When the LLM decides to use the search tool, Rig automatically generates an embedding for the query, searches the vector store, and returns the most relevant text snippets
- **Scalability**: While the example uses `InMemoryVectorStore`, Rig supports production stores like MongoDB or LanceDB. You can swap the storage engine without changing agent logic

## Using Dynamic Context (Simpler RAG)

For simpler RAG patterns, use `.dynamic_context()` instead of explicit tools:

```rust
let agent = client
    .agent("gpt-4")
    .preamble("You are a helpful assistant using the provided context.")
    .dynamic_context(3, index)  // Use top 3 similar documents as context
    .build();

let response = agent.prompt("What is Rig?").await?;
```

This automatically injects relevant documents into the prompt without exposing a tool to the LLM.

## Production Vector Stores

### LanceDB (Persistent, Disk-Based)

```rust
use rig_lancedb::{LanceDbVectorStore, Connection};

let db = Connection::open("/path/to/db").await?;
let table = db.create_table("docs", schema).await?;

let vector_store = LanceDbVectorStore::new(table);
let index = vector_store.index(embedding_model).await?;
```

### MongoDB

```rust
use rig_mongodb::MongoDbVectorStore;

let mongo_client = mongodb::Client::with_uri_str("mongodb://localhost:27017").await?;
let db = mongo_client.database("my_db");
let collection = db.collection("documents");

let vector_store = MongoDbVectorStore::new(collection);
let index = vector_store.index(embedding_model).await?;
```

### Qdrant

```rust
use rig_qdrant::QdrantVectorStore;

let qdrant_client = qdrant_client::client::QdrantClient::from_url("http://localhost:6334").build()?;
let vector_store = QdrantVectorStore::new(qdrant_client, "collection_name");
let index = vector_store.index(embedding_model).await?;
```

### SurrealDB

```rust
use rig_surrealdb::SurrealVectorStore;
use surrealdb::Surreal;
use surrealdb::engine::local::Mem;

let db = Surreal::new::<Mem>(()).await?;
db.use_ns("test").use_db("test").await?;

let vector_store = SurrealVectorStore::with_defaults(embedding_model.clone(), db);
vector_store.insert_documents(embeddings).await?;
```

## Working with Custom Document Types

Use the `Embed` derive macro to make custom types embeddable:

```rust
use rig::Embed;
use serde::{Deserialize, Serialize};

#[derive(Embed, Clone, Debug, Serialize, Deserialize)]
struct Document {
    id: String,
    #[embed]  // This field will be embedded
    content: String,
    metadata: String,  // Not embedded, but stored
}

let docs = vec![
    Document {
        id: "1".to_string(),
        content: "Rust is a systems programming language".to_string(),
        metadata: "category: programming".to_string(),
    },
    Document {
        id: "2".to_string(),
        content: "Rig is a Rust library for LLM applications".to_string(),
        metadata: "category: libraries".to_string(),
    },
];

let embeddings = EmbeddingsBuilder::new(model.clone())
    .documents(docs)?
    .build()
    .await?;
```

## Querying the Index

### Top N Similar Documents

```rust
let results = index
    .top_n::<Document>("What is Rig?", 5)
    .await?;

for (score, id, doc) in results {
    println!("Score: {}, ID: {}, Content: {}", score, id, doc.content);
}
```

### Top N from Query Embedding

```rust
let query_embedding = model.embed_text("What is Rust?").await?;

let results = index
    .top_n_from_query(query_embedding.vec, 3)
    .await?;
```

## Comparison: Function vs. RAG Tools

| Feature | Function Tool (e.g., Adder) | RAG Tool (Vector Search) |
|---------|----------------------------|--------------------------|
| **Data Source** | Hardcoded logic or Live API | Unstructured text/documents |
| **Logic** | Deterministic (1+1 is always 2) | Probabilistic (Similarity match) |
| **Best For** | Actions, Calculations, Real-time data | Knowledge bases, FAQs, Research |
| **Setup** | Requires defining a Struct | Requires an Embedding Model |
| **Response Time** | Fast | Depends on vector store size |

## Document Processing Pipeline

For production RAG systems, use a document processing pipeline:

```rust
use rig::pipeline::{Pipeline, Op};

// 1. Load documents from source
async fn load_documents() -> Vec<String> {
    // Load from PDF, web scraping, database, etc.
    vec![]
}

// 2. Chunk documents into smaller pieces
async fn chunk_documents(docs: Vec<String>) -> Vec<String> {
    docs.into_iter()
        .flat_map(|doc| chunk_text(&doc, 512))  // 512 token chunks
        .collect()
}

// 3. Generate embeddings
async fn generate_embeddings(chunks: Vec<String>, model: EmbeddingModel) -> Vec<Embedding> {
    EmbeddingsBuilder::new(model)
        .documents(chunks)?
        .build()
        .await?
}

// 4. Store in vector database
async fn store_in_vector_db(embeddings: Vec<Embedding>, store: &mut VectorStore) {
    store.add_documents(embeddings).await?;
}
```

## Best Practices

### 1. Chunk Size Optimization

```rust
// Too small: loses context
let chunks = chunk_text(document, 100);

// Too large: less precise retrieval
let chunks = chunk_text(document, 2000);

// Optimal: 256-512 tokens for most use cases
let chunks = chunk_text(document, 512);
```

### 2. Metadata Filtering

Use metadata to filter search results:

```rust
#[derive(Embed, Serialize, Deserialize)]
struct Document {
    #[embed]
    content: String,
    category: String,
    date: String,
}

// Only search in specific categories
let results = index
    .filter("category", "technical")
    .top_n("How does it work?", 5)
    .await?;
```

### 3. Hybrid Search

Combine vector search with keyword search for better results:

```rust
// 1. Vector search for semantic similarity
let vector_results = index.top_n("machine learning", 10).await?;

// 2. Keyword search for exact matches
let keyword_results = full_text_search("machine learning");

// 3. Merge and re-rank results
let final_results = merge_results(vector_results, keyword_results);
```

### 4. Embedding Model Selection

```rust
// For production use cases:

// OpenAI text-embedding-3-small: Good balance (1536 dimensions)
let model = client.embedding_model(TEXT_EMBEDDING_3_SMALL);

// OpenAI text-embedding-3-large: Better quality (3072 dimensions)
let model = client.embedding_model(TEXT_EMBEDDING_3_LARGE);

// FastEmbed for local/offline: Fast, runs locally
let model = fastembed_client.embedding_model(AllMiniLML6V2Q);
```

### 5. Index Updates

Handle document updates properly:

```rust
// Add new documents
vector_store.add_documents(new_embeddings).await?;

// Update existing document (delete + add)
vector_store.delete_by_id("doc_123").await?;
vector_store.add_documents(updated_embedding).await?;

// Rebuild index periodically for optimal performance
vector_store.rebuild_index().await?;
```

## Monitoring and Debugging

### Check Embedding Quality

```rust
// Compare similar documents
let doc1_embedding = model.embed_text("Rust is a programming language").await?;
let doc2_embedding = model.embed_text("Python is a programming language").await?;

let similarity = cosine_similarity(&doc1_embedding.vec, &doc2_embedding.vec);
println!("Similarity: {}", similarity);  // Should be high (>0.8)
```

### Track Search Performance

```rust
let start = std::time::Instant::now();
let results = index.top_n("query", 5).await?;
let duration = start.elapsed();

println!("Search took: {:?}", duration);
println!("Found {} results", results.len());
```

## Supported Vector Stores

### Built-in
- **InMemoryVectorStore**: Zero-dependency, RAM-based (development/testing)

### Companion Crates
- **rig-mongodb**: MongoDB integration
- **rig-lancedb**: LanceDB (persistent, disk-based)
- **rig-neo4j**: Neo4j graph database
- **rig-qdrant**: Qdrant vector database
- **rig-surrealdb**: SurrealDB multi-model database
- **rig-sqlite**: SQLite with vector extension
- **rig-milvus**: Milvus vector database
- **rig-scylladb**: ScyllaDB
- **rig-s3vectors**: AWS S3Vectors
- **rig-helixdb**: HelixDB

## Related Topics

- [Embeddings](./embeddings.md) - Working with EmbeddingModel trait and Embed derive
- [Tool Calling](./tool-calling.md) - Creating custom tools
- [Memory & Sessions](./memory.md) - Combining RAG with conversation history
