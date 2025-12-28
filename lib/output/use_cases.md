`rig` (often referred via its core crate `rig-core`) is a Rust library designed to build LLM-powered applications with a focus on modularity, type safety, and common patterns like RAG and Agents.

Here are five common use cases where `rig-core` excels, along with code examples.

---

### 1. Retrieval-Augmented Generation (RAG) Systems

**Use Case:** You have a large corpus of private documentation (PDFs, Markdown, or Wikis) and want an LLM to answer questions based strictly on that data.

**Benefit:** `rig-core` provides a unified abstraction for vector databases and embedding models. You can swap out a local Qdrant instance for a cloud-based MongoDB Atlas vector search without rewriting your business logic. It handles the "context assembly" (joining retrieved chunks into a prompt) automatically.

**Code Example:**

````rust
use rig::{providers::openai, vector_store::VectorStoreIndex};

// Initialize an OpenAI client and a Vector Store index
let client = openai::Client::from_env();
let model = client.embedding_model("text-embedding-3-small");
let index = my_vector_db.index(model); // Abstracted vector store

// Create a RAG agent
let rag_agent = client.agent("gpt-4o")
    .preamble("You are a helpful assistant that answers questions based on the provided docs.")
    .dynamic_context(2, index) // Automatically fetch 2 relevant chunks
    .build();

let response = rag_agent.prompt("How do I configure the production server?").await?;
````

---

### 2. Autonomous Agents with Tool Use (Function Calling)

**Use Case:** Building a virtual assistant that doesn't just talk but *acts*—such as checking the weather, querying a SQL database, or interacting with a GitHub API.

**Benefit:** `rig-core` uses Rust's type system and `serde` to make tool-calling type-safe. You define your tools as simple Rust structs/functions, and `rig` handles the JSON schema generation and the execution loop where the LLM decides which tool to call.

**Code Example:**

````rust
use rig::tool::Tool;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, JsonSchema)]
struct CalculatorArgs { x: i32, y: i32 }

// Define a tool
struct AddTool;
impl Tool for AddTool {
    type Args = CalculatorArgs;
    type Output = i32;
    async fn call(&self, args: Self::Args) -> Result<Self::Output, ToolError> {
        Ok(args.x + args.y)
    }
}

// Attach tool to agent
let agent = client.agent("gpt-4o")
    .tool(AddTool)
    .build();

let res = agent.prompt("What is 1234 + 5678?").await?;
````

---

### 3. Model-Agnostic LLM Pipelines

**Use Case:** You are building an enterprise application and want to avoid vendor lock-in. You might want to use GPT-4 for complex reasoning but switch to a local Llama-3 (via Ollama) for sensitive data or cost-saving.

**Benefit:** `rig-core` provides a standard `CompletionModel` trait. Your core logic depends on the trait, not the specific provider (OpenAI, Anthropic, Cohere, etc.). Switching providers becomes a single line of configuration change.

**Code Example:**

````rust
async fn run_pipeline(model: impl rig::completion::CompletionModel) {
    let response = model.completion("Summarize this report...").await;
    // ... logic remains same regardless of provider
}

// Easily switchable:
let openai_model = openai_client.completion_model("gpt-4");
let anthropic_model = anthropic_client.completion_model("claude-3-5-sonnet");

run_pipeline(openai_model).await;
run_pipeline(anthropic_model).await;
````

---

### 4. Structured Data Extraction

**Use Case:** Converting unstructured text (like emails, medical notes, or invoices) into structured JSON objects for a database.

**Benefit:** Instead of manual parsing or regex, `rig-core` supports "structured outputs." By leveraging Rust's `JsonSchema` and `Deserialize` traits, it ensures the LLM's response maps perfectly to your Rust structs, throwing compile-time or runtime errors if the schema is violated.

**Code Example:**

````rust
#[derive(Deserialize, JsonSchema)]
struct InvoiceDetails {
    amount: f64,
    vendor: String,
    items: Vec<String>,
}

let extractor = client.extractor::<InvoiceDetails>("gpt-4o")
    .preamble("Extract invoice details from the text.")
    .build();

let invoice: InvoiceDetails = extractor.extract("I paid $50 to Acme Corp for Hammers.").await?;
println!("Vendor: {}, Cost: {}", invoice.vendor, invoice.amount);
````

---

### 5. Multi-Agent Orchestration (Workflows)

**Use Case:** Complex tasks where one LLM acts as a "Manager" that delegates sub-tasks to specialized "Worker" agents (e.g., a "Researcher" agent and a "Writer" agent).

**Benefit:** Because `rig-core` agents are lightweight and share a common interface, they can be nested. You can treat one agent as a "Tool" for another agent, creating a hierarchy of AI logic.

**Code Example:**

````rust
// Researcher Agent
let researcher = client.agent("gpt-4o").preamble("You find facts.").build();

// Manager Agent that has the researcher as a tool
let manager = client.agent("gpt-4o")
    .preamble("You manage tasks and use the researcher when you need facts.")
    .tool(researcher.to_tool("research_tool", "Use this to look up facts"))
    .build();

let final_report = manager.prompt("Write a report on the 2024 tech trends.").await?;
````

### Summary of Benefits for Rust Developers:

1. **Type Safety:** Uses Rust's type system to prevent common LLM integration errors (like passing the wrong JSON keys).
1. **Concurrency:** Leverages Rust's `async/await` for high-performance, parallel LLM calls.
1. **Low Boilerplate:** Standardizes the "Prompt -> Call -> Parse" loop into a builder pattern.
1. **Extensibility:** The Trait-based architecture allows you to write custom providers or vector stores easily.