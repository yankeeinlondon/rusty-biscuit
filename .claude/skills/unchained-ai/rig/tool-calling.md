# Tool Calling with Rig

Rig provides type-safe tool calling by leveraging Rust's type system and JSON schema generation. The `Tool` trait ensures that LLMs receive well-structured schemas and return valid data.

## How Tool Calling Works

A tool in Rig requires two main components:

1. **Definition**: Name and description that tells the LLM what the tool does
2. **Input Type**: Struct implementing `serde::Deserialize` and `JsonSchema` for automatic schema generation

## The Tool Trait

```rust
pub trait Tool {
    const NAME: &'static str;
    type Error: std::error::Error + Send + Sync;
    type Args: serde::Deserialize + JsonSchema;
    type Output: serde::Serialize;

    async fn definition(&self, prompt: String) -> ToolDefinition;
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error>;
}
```

## Basic Example

```rust
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// 1. Define the arguments the LLM will provide
#[derive(Deserialize, JsonSchema, Serialize)]
pub struct AddArgs {
    /// The first number to add
    pub a: i32,
    /// The second number to add
    pub b: i32,
}

// 2. Define the Tool struct
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
```

## Integrating Tools into Agents

```rust
use rig::providers::openai;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = openai::Client::from_env();

    let agent = client
        .agent("gpt-4o")
        .preamble("You are a helpful assistant that can perform math.")
        .tool(Adder)
        .build();

    let response = agent.prompt("What is 1234 plus 5678?").await?;
    println!("Agent response: {}", response);

    Ok(())
}
```

## Execution Flow

When you call `agent.prompt()`:

1. Agent sends prompt + tool definitions to LLM
2. LLM decides if it needs to call `add_numbers`
3. LLM returns JSON arguments matching `AddArgs` schema
4. Rig deserializes and validates the arguments
5. Rig executes `call()` method with validated arguments
6. Result is sent back to LLM for natural language response

## Tools with External State

Tools can include API keys, database connections, or other state:

```rust
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get the current weather for a specific city".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(WeatherArgs)).unwrap(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // In a real app, use reqwest to call weather API
        Ok(format!("The weather in {} is currently 22°C and sunny.", args.city))
    }
}
```

## Key Concepts

### JsonSchema Trait
This is crucial. Without it, Rig cannot tell the LLM the "shape" of your data (e.g., that `a` and `b` must be integers).

### Descriptions Matter
The docstrings (e.g., `/// The first number`) and the `description` field in `ToolDefinition` are used by the LLM to decide **when** to call your tool. Be descriptive!

### Type Safety
If the LLM tries to send a string where an integer is expected, deserialization fails before the `call` method is reached, preventing runtime logic errors.

## Error Handling and Retries

Rig validates LLM's JSON input before your tool's `call` method is invoked using `serde` and `schemars`. If your `call` method returns an `Err`, Rig passes that error message back to the LLM. Often, the LLM will see the error and try to self-correct by asking for clarification or trying a different tool.

## Comparison: Rig vs. Manual Tool Calling

| Feature | Manual Implementation | Using Rig |
|---------|----------------------|-----------|
| Schema Generation | Hand-written JSON strings | Auto-generated via `JsonSchema` |
| Type Safety | Manual parsing of JSON strings | Automated via `serde` |
| Tool Selection | Complex `if/else` or `match` logic | Handled by Agent builder |
| Async Support | Requires manual boilerplate | Natively `async` |

## Related Topics

- [Multi-Tool Agents](./multi-tool-agents.md) - Using multiple tools in one agent
- [Agentic Loops](./agentic-loops.md) - Tool chaining patterns
- [Vector Store Integration](./vector-stores.md) - RAG-based dynamic tools
