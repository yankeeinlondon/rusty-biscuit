The `rig-core` crate is a Rust framework designed for building LLM-powered applications, such as RAG (Retrieval-Augmented Generation) systems and AI agents. Because it is highly modular and built on the Rust async ecosystem, it is almost always paired with specific libraries to handle execution, data serialization, and schema definition.

Here are the three libraries most commonly integrated with `rig-core`.

---

### 1. Tokio

**Why they are used together:**
`rig-core` is an entirely asynchronous library. Every interaction with an LLM (OpenAI, Anthropic) or a vector database involves network I/O. **Tokio** provides the multi-threaded async runtime required to execute these non-blocking calls. Without Tokio (or a similar runtime), you cannot poll the futures returned by Rig’s agents and extractors.

**Code Example:**

````rust
use rig::providers::openai;
use rig::completion::Prompt;

#[tokio::main] // Standard Tokio macro to initialize the runtime
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the OpenAI client through Rig
    let client = openai::Client::from_env();

    // Build a simple agent
    let agent = client.agent("gpt-4o")
        .preamble("You are a helpful assistant.")
        .build();

    // The .prompt() method returns a Future, which Tokio executes
    let response = agent.prompt("What is the capital of France?").await?;

    println!("Response: {}", response);
    Ok(())
}
````

---

### 2. Serde (with `serde_json`)

**Why they are used together:**
One of Rig’s most powerful features is **Structured Output** and **Tool Use (Function Calling)**.

* **Tool Use:** When an LLM decides to call a tool, it returns a JSON string. `serde` is used to deserialize that JSON into a type-safe Rust struct.
* **Structured Output:** If you want an LLM to return a specific data format (like a list of tasks), Rig uses `serde` to map the LLM’s response directly into your Rust models.

**Code Example:**

````rust
use rig::completion::CompletionModel;
use serde::{Deserialize, Serialize};

// Define the structure you want the LLM to return
#[derive(Serialize, Deserialize, Debug)]
struct Translation {
    original_text: String,
    translated_text: String,
    language: String,
}

// Rig uses Serde to ensure the LLM output matches this struct
async fn get_translation(agent: rig::agent::Agent<openai::CompletionModel>) {
    // Rig's structured output relies on Serde under the hood
    let result: Translation = agent
        .typed_prompt("Translate 'Hello' to Spanish")
        .await
        .unwrap();
    
    println!("Translated: {}", result.translated_text);
}
````

---

### 3. Schemars

**Why they are used together:**
While `serde` handles the *conversion* of data, **Schemars** handles the *description* of data. To perform Tool Use or Structured Output, Rig must send a **JSON Schema** to the LLM (e.g., OpenAI) so the model knows exactly what fields and types to generate.

In Rig, you typically derive `JsonSchema` for your structs. This allows Rig to automatically generate the documentation that tells the LLM: "This tool requires an integer called 'count' and a string called 'name'."

**Code Example:**

````rust
use rig::tool::Tool;
use schemars::JsonSchema; // Essential for Rig Tools
use serde::{Deserialize, Serialize};

// Define arguments for a tool
#[derive(Deserialize, Serialize, JsonSchema)]
struct CalculatorArgs {
    x: i32,
    y: i32,
}

// Define the tool
struct AddTool;

impl Tool for AddTool {
    type Args = CalculatorArgs; // Rig uses Schemars here to tell the LLM the schema
    type Output = i32;
    const NAME: &'static str = "add_numbers";

    async fn definition(&self, _prompt: String) -> rig::tool::ToolDefinition {
        rig::tool::ToolDefinition {
            name: Self::NAME.into(),
            description: "Adds two numbers together".into(),
            // Rig uses Schemars to generate the JSON schema for CalculatorArgs
            parameters: schemars::schema_for!(CalculatorArgs).into(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, rig::tool::ToolError> {
        Ok(args.x + args.y)
    }
}
````

### Summary Table

|Library|Role with `rig-core`|Primary Use Case|
|:------|:-------------------|:---------------|
|**Tokio**|Runtime|Executing async LLM requests and vector searches.|
|**Serde**|Data Bridge|Converting LLM JSON strings into type-safe Rust structs.|
|**Schemars**|Schema Provider|Telling the LLM the expected format for tool arguments.|