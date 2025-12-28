# Agentic Loops in Rig

In Rig, tool chaining is handled by the **Agentic Loop**. Instead of a single request-response cycle, the agent enters a loop where it can call a tool, observe the result, and decide if it needs to call another tool or provide a final answer.

## The Execution Loop

When you use a high-level `Agent` in Rig, the loop is largely managed for you. If the agent determines that the output of Tool A is necessary to fulfill the requirements of Tool B, it will execute them sequentially.

## Sequential Tool Dependencies

The agentic loop automatically handles sequential dependencies when one tool's output is required as input for another tool.

### Example: Search Then Details

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
            description: "Get user details by their ID".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(GetDetailsArgs)).unwrap(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(format!("Details for {}: Email is john@example.com", args.user_id))
    }
}
```

### Execution Sequence

When you ask "What is John Doe's email?", the agent follows this sequence:

1. **Call `search_users`** with `{"query": "John Doe"}`
2. **Receive** `"user_123"`
3. **Call `get_user_details`** with `{"user_id": "user_123"}`
4. **Final Answer**: "John Doe's email is john@example.com."

The LLM recognizes the dependency automatically based on tool descriptions and the user's request.

## Preamble Tuning for Complex Workflows

Use the agent's preamble to explain multi-step workflows:

```rust
let agent = client
    .agent("gpt-4o")
    .preamble("You are a helpful assistant. When looking up user information, first use search_users to find their ID, then use get_user_details to get their full information.")
    .tool(UserSearch)
    .tool(UserDetails)
    .build();
```

### Good Preamble Examples

**For Research Tasks:**
```rust
.preamble("You are a research assistant. First search for relevant documents, then analyze them, and finally summarize findings.")
```

**For Data Processing:**
```rust
.preamble("You process data in stages: first validate the input, then transform it, and finally store the result.")
```

**For Customer Support:**
```rust
.preamble("You help customers by first looking up their account, then checking for issues, and finally providing solutions.")
```

## Error Handling in Agentic Loops

When a tool in the chain fails, the agent receives the error and can decide how to proceed:

```rust
impl Tool for UserDetails {
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Simulate user not found
        if args.user_id == "user_999" {
            return Err("User not found in database".into());
        }

        Ok(format!("Details for {}: ...", args.user_id))
    }
}

// When the chain fails:
// 1. search_users returns "user_999"
// 2. get_user_details fails with "User not found"
// 3. LLM sees the error as an "observation"
// 4. LLM may try an alternative approach or report to user
```

## Structured Output for Complex Chains

For chains that need to return data in specific formats, use the `completion` API:

```rust
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct UserReport {
    user_id: String,
    email: String,
    status: String,
}

// After the tool chain completes, extract structured data
let completion = agent.completion("Get John Doe's information").await?;
let report: UserReport = serde_json::from_str(&completion.text)?;
```

## Advanced: Manual Loop Control

For complex scenarios, you can maintain state across interactions:

```rust
use rig::agent::Session;

let agent = client
    .agent("gpt-4o")
    .tool(UserSearch)
    .tool(UserDetails)
    .build();

let mut session = agent.context(vec![]);

// Step 1: Search
let res1 = session.prompt("Find user John Doe").await?;
println!("Search result: {}", res1);

// Step 2: Get details (agent remembers previous search)
let res2 = session.prompt("Now get their email address").await?;
println!("Email: {}", res2);

// The session maintains the conversation history,
// so the agent knows which user ID was found
```

## Loop Optimization Strategies

### 1. Minimize Tool Calls

Design tools to return comprehensive data rather than requiring multiple calls:

**Better:**
```rust
// Returns full user object with all details
async fn call(&self, args: Self::Args) -> Result<UserData, Self::Error> {
    Ok(UserData {
        id: "user_123",
        email: "john@example.com",
        name: "John Doe",
        status: "active",
    })
}
```

**Worse:**
```rust
// Requires separate calls for each field
get_user_email(), get_user_name(), get_user_status()
```

### 2. Use Dynamic Context for Knowledge

Instead of tool chaining for knowledge lookup, use vector store dynamic context:

```rust
let agent = client
    .agent("gpt-4o")
    .dynamic_context(5, vector_store)  // Top 5 relevant docs
    .tool(ActionTool)  // Only tools for actions, not knowledge
    .build();
```

### 3. Batch Operations

When possible, design tools to handle batches:

```rust
#[derive(Deserialize, JsonSchema, Serialize)]
pub struct BatchDetailsArgs {
    pub user_ids: Vec<String>,  // Multiple IDs at once
}
```

## Summary Table: Tool Calling Mechanics

| Concept | Rig Implementation |
|---------|-------------------|
| **Schema Generation** | `schemars::JsonSchema` on the Args struct |
| **Tool Registry** | The `.tool()` method on Agent builder |
| **Execution** | Automatic via agent's internal loop |
| **Error Feedback** | Errors from `call()` sent back to LLM as "observations" |
| **State Management** | Via `Session` for multi-turn conversations |

## Related Topics

- [Tool Calling](./tool-calling.md) - Basic tool implementation
- [Multi-Tool Agents](./multi-tool-agents.md) - Using multiple tools together
- [Memory & Sessions](./memory.md) - Maintaining context across loops
