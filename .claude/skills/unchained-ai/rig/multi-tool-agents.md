# Multi-Tool Agents

A multi-tool agent in Rig can intelligently choose which tool to use based on the user's request. This is where Rig truly shines, providing seamless orchestration across multiple tools.

## Building a Multi-Tool Agent

When you build an agent, simply chain multiple `.tool()` calls. Rig constructs a prompt for the LLM that includes descriptions of all available tools.

```rust
use rig::providers::openai;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = openai::Client::from_env();

    let agent = client
        .agent("gpt-4o")
        .preamble("You are a helpful research assistant.")
        .tool(Adder)
        .tool(WeatherTool { api_key: "abc-123".into() })
        .build();

    // The LLM will choose 'add_numbers'
    let math_res = agent.prompt("What is 50 + 50?").await?;
    println!("Math: {}", math_res);

    // The LLM will choose 'get_weather'
    let weather_res = agent.prompt("Should I wear a jacket in London?").await?;
    println!("Weather: {}", weather_res);

    Ok(())
}
```

## How Tool Selection Works

1. Agent receives user prompt
2. Rig sends prompt + all tool definitions to LLM
3. LLM analyzes prompt and tool descriptions
4. LLM chooses appropriate tool based on semantic match
5. Rig executes chosen tool
6. Result is sent back to LLM for natural language response

## Example: Search + Details Pattern

A common pattern is having one tool to find an identifier and another to fetch details:

```rust
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, JsonSchema, Serialize)]
pub struct SearchArgs {
    pub query: String
}

#[derive(Deserialize, JsonSchema, Serialize)]
pub struct GetDetailsArgs {
    pub user_id: String
}

// Tool 1: Searches for a user and returns an ID
pub struct UserSearch;

impl Tool for UserSearch {
    const NAME: &'static str = "search_users";
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Args = SearchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search for a user by name and return their ID".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SearchArgs)).unwrap(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Logic: "John Doe" -> "user_123"
        Ok("user_123".to_string())
    }
}

// Tool 2: Takes an ID and returns specific details
pub struct UserDetails;

impl Tool for UserDetails {
    const NAME: &'static str = "get_user_details";
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Args = GetDetailsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get detailed information for a user by their ID".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(GetDetailsArgs)).unwrap(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(format!("Details for {}: Email is john@example.com", args.user_id))
    }
}

// Register both tools
let agent = client
    .agent("gpt-4o")
    .preamble("You help users find information. First search, then get details.")
    .tool(UserSearch)
    .tool(UserDetails)
    .build();

// The agent will automatically chain the tools
let response = agent.prompt("What is John Doe's email?").await?;
// Execution: search_users("John Doe") -> "user_123" -> get_user_details("user_123")
```

## Error Handling in Multi-Tool Scenarios

One of the "hidden" complexities of tool calling is when the LLM hallucinates arguments (e.g., passing a string where an integer is expected).

### Rig's Built-in Validation

Because Rig uses `serde` and `schemars`, it validates the LLM's JSON input before your tool's `call` method is even invoked.

### Execution Errors

If your `call` method returns an `Err`, Rig will pass that error message back to the LLM. Often, the LLM will see the error (e.g., "City not found") and try to "self-correct" by asking for clarification or trying a different tool.

Example error handling:

```rust
impl Tool for WeatherTool {
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let city = &args.city;

        // Simulate API call failure
        if city == "Unknown City" {
            return Err(format!("City '{}' not found in database", city).into());
        }

        Ok(format!("Weather in {}: 22°C and sunny", city))
    }
}

// When the LLM tries to get weather for "Unknown City":
// 1. Tool returns error
// 2. LLM receives error message
// 3. LLM may ask user for clarification or suggest valid cities
```

## Optimizing Tool Selection

### Clear, Specific Descriptions

Tool descriptions are critical for proper selection:

**Good:**
```rust
description: "Get the current weather for a specific city using real-time weather API data"
```

**Bad:**
```rust
description: "Get weather"
```

### Preamble Guidance

Use the agent's preamble to guide tool usage:

```rust
let agent = client
    .agent("gpt-4o")
    .preamble("You are a helpful assistant. When users ask about weather, use the get_weather tool. For calculations, use the calculator tool.")
    .tool(WeatherTool { api_key: key })
    .tool(Calculator)
    .build();
```

### Tool Naming Conventions

Use descriptive, verb-based names:
- `search_users` instead of `users`
- `calculate_total` instead of `total`
- `get_weather` instead of `weather`

## Comparison: Rig vs. Manual Implementation

| Feature | Manual Implementation | Using Rig |
|---------|----------------------|-----------|
| **Schema Generation** | Hand-written JSON strings | Auto-generated via `JsonSchema` |
| **Type Safety** | Manual parsing of JSON strings | Automated via `serde` |
| **Tool Selection** | Complex `if/else` or `match` logic | Handled by Agent builder |
| **Async Support** | Requires manual boilerplate | Natively `async` |
| **Error Feedback** | Manual error propagation | Automatic error-to-LLM feedback |

## Related Topics

- [Tool Calling](./tool-calling.md) - Basic tool implementation
- [Agentic Loops](./agentic-loops.md) - Sequential tool dependencies
- [Memory & Sessions](./memory.md) - Maintaining context across tool calls
