# Memory & Sessions in Rig

To make an agent truly useful in production, it needs **Memory**. Without it, every tool call and conversation starts from a blank slate. In Rig, memory is handled via the `ChatHistory` trait and the `Session` abstraction.

## Why Memory Matters

Without memory, tool calling becomes brittle:

- **Context Loss**: If the user says "Search for that again," the agent won't know what "that" refers to
- **Redundant Calls**: The agent might call an expensive API tool multiple times for the same information
- **No Correction**: If a tool call fails, the agent needs to see that failure in history to try a different approach

## The Interaction Flow with Memory

When an agent uses memory, the sequence includes **Context Injection**:

1. User sends prompt
2. **Agent reviews previous messages** in history
3. Agent decides if answer exists in context or needs new tool call
4. If tool call needed, executes tool
5. **Stores tool request and result in history**
6. Returns final answer

## Using Sessions

A `Session` is a stateful wrapper around an agent that maintains conversation history:

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

    // 2. Create a Session with empty history
    let mut session = agent.context(vec![]);

    // 3. First interaction: Agent calls tool and remembers it
    let res1 = session.prompt("Add 5 and 10 for me.").await?;
    println!("Response 1: {}", res1);

    // 4. Second interaction: Agent knows the previous result
    // No need to call tool again because history contains the answer
    let res2 = session.prompt("Now multiply that result by 2.").await?;
    println!("Response 2: {}", res2);

    Ok(())
}
```

## Persisting History

For production applications (web apps, chatbots, support agents), you'll want to persist history to a database:

```rust
use rig::agent::Session;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

async fn handle_user_message(
    session_id: String,
    user_message: String,
) -> Result<String, Box<dyn std::error::Error>> {
    // 1. Load history from database
    let history: Vec<ChatMessage> = load_from_db(&session_id).await?;

    // 2. Create session with loaded history
    let mut session = agent.context(history);

    // 3. Process new message
    let response = session.prompt(&user_message).await?;

    // 4. Save updated history back to database
    let updated_history = session.history();
    save_to_db(&session_id, updated_history).await?;

    Ok(response)
}
```

## Memory Strategies

### 1. No Memory (Stateless)

**Use Case**: Simple one-off commands

```rust
let agent = client.agent("gpt-4o").build();
let response = agent.prompt("Translate 'hello' to Spanish").await?;
```

**Pros**: Fastest, simplest
**Cons**: No context between calls

### 2. In-Memory Session

**Use Case**: CLI tools, short-lived scripts

```rust
let mut session = agent.context(vec![]);
// Session lives for duration of program
```

**Pros**: Fast, no database overhead
**Cons**: Lost on restart

### 3. Database-Backed History (JSON)

**Use Case**: Web apps, Slack bots, customer support

```rust
// Persist as JSON in PostgreSQL, MongoDB, etc.
let history = load_from_db(session_id).await?;
let mut session = agent.context(history);
// ... handle interaction ...
save_to_db(session_id, session.history()).await?;
```

**Pros**: Permanent, accessible across sessions
**Cons**: Slower due to database I/O

### 4. Vector-Based Long-Term Memory

**Use Case**: Agents that need to recall things from weeks ago

```rust
// Store old conversations in vector store
// Retrieve relevant past conversations as dynamic context
let agent = client
    .agent("gpt-4o")
    .dynamic_context(3, memory_vector_store)
    .build();
```

**Pros**: Can recall distant context
**Cons**: Most complex, slowest

## Comparison of Memory Types

| Strategy | Performance | Persistence | Best Use Case |
|----------|-------------|-------------|---------------|
| **No Memory** | Fastest | None | Simple one-off commands |
| **In-Memory** | Fast | Lost on Restart | CLI tools, scripts |
| **Database (JSON)** | Slower | Permanent | Web apps, bots, support |
| **Vector-based** | Slowest | Permanent | Long-term memory, research |

## Managing History Size

Long conversations can exceed token limits. Strategies to manage this:

### 1. Sliding Window

Keep only the most recent N messages:

```rust
fn truncate_history(history: Vec<ChatMessage>, max_size: usize) -> Vec<ChatMessage> {
    if history.len() > max_size {
        history[history.len() - max_size..].to_vec()
    } else {
        history
    }
}

let history = load_from_db(session_id).await?;
let truncated = truncate_history(history, 20);  // Keep last 20 messages
let mut session = agent.context(truncated);
```

### 2. Summarization

Periodically summarize old messages:

```rust
// When history gets long:
// 1. Use LLM to summarize first N messages
// 2. Replace them with summary
// 3. Keep recent messages as-is
let summary = summarize_agent
    .prompt("Summarize this conversation: ...")
    .await?;

let new_history = vec![
    ChatMessage { role: "system", content: summary },
    // ... recent messages ...
];
```

### 3. Selective Retention

Keep only messages with tool calls or important context:

```rust
fn filter_important(history: Vec<ChatMessage>) -> Vec<ChatMessage> {
    history
        .into_iter()
        .filter(|msg| {
            msg.contains_tool_call() ||
            msg.role == "system" ||
            msg.is_recent()
        })
        .collect()
}
```

## Extracting and Inspecting History

```rust
let mut session = agent.context(vec![]);

session.prompt("Add 10 and 20").await?;
session.prompt("Multiply the result by 3").await?;

// Get full conversation history
let history = session.history();

// Inspect messages
for message in history {
    println!("Role: {}", message.role);
    println!("Content: {}", message.content);
    if let Some(tool_calls) = message.tool_calls {
        println!("Tool calls: {:?}", tool_calls);
    }
}
```

## Best Practices

### 1. Always Persist in Production

Never rely on in-memory sessions for production user-facing apps:

```rust
// Bad for production
static mut SESSION: Option<Session> = None;

// Good for production
async fn get_session(session_id: &str) -> Session {
    let history = db.load_history(session_id).await?;
    agent.context(history)
}
```

### 2. Include System Context

Add important context as system messages:

```rust
let history = vec![
    ChatMessage {
        role: "system".into(),
        content: "User's timezone: UTC-8, Subscription: Premium".into(),
    },
    // ... conversation history ...
];
let session = agent.context(history);
```

### 3. Monitor Token Usage

Track token consumption to avoid hitting limits:

```rust
fn estimate_tokens(history: &[ChatMessage]) -> usize {
    history.iter()
        .map(|msg| msg.content.split_whitespace().count())
        .sum::<usize>() * 2  // Rough estimate
}

if estimate_tokens(&history) > 6000 {
    // Implement truncation or summarization
}
```

### 4. Handle Concurrent Updates

For multi-user systems, use proper locking:

```rust
// Use database transactions or distributed locks
let lock = redis.lock(format!("session:{}", session_id)).await?;
let history = load_from_db(&session_id).await?;
let mut session = agent.context(history);
let response = session.prompt(user_msg).await?;
save_to_db(&session_id, session.history()).await?;
lock.release().await?;
```

## Related Topics

- [Agentic Loops](./agentic-loops.md) - How memory affects tool chaining
- [Multi-Tool Agents](./multi-tool-agents.md) - Using memory with multiple tools
- [Vector Store Integration](./vector-stores.md) - Vector-based long-term memory
