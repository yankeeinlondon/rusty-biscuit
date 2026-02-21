---
prompt: |-
    Do a deep dive on the JSON-RPC API that Kimi Code CLI provides.

    After completing the deep dive, provide the following additional sections which cover writing code examples with Rust:

    1. Show how a Rust client can interact programmatically with the Kimi Code Agent using ACP
    2. Show explicitly how to handle "Reverse Requests" where Kimi asks the client to fulfill a tool request, a file read, etc. (as a Kimi is not allowed to do this directly when operating via ACP)
    3. dfd
    4. Show how the Rust client we've created can use things like `mpsc` channels to send Kimi's streaming text to a desktop desktop app framework like Tauri or iced
---

## Kimi Code's JSON-RPC API

Here is a deep dive into the JSON-RPC API provided by Kimi Code CLI.

To understand Kimi Code CLI’s API, you have to look at the **Agent Client Protocol (ACP)**. Rather than inventing a proprietary API, Moonshot AI built Kimi Code CLI to natively implement ACP—an open-source standard (spearheaded by Zed and JetBrains) that does for AI coding agents what the Language Server Protocol (LSP) did for language tooling.

When you run Kimi Code CLI in its server mode (`kimi acp`), it acts as an ACP Agent communicating with an ACP Client (like an IDE or a custom script) using **JSON-RPC 2.0 over standard input/output (stdio)**.

Here is a technical breakdown of how the API is structured, the core lifecycle, and the payloads involved.

---

### 1. Transport and Architecture

* **Protocol:** JSON-RPC 2.0.
* **Transport Layer:** Newline-delimited JSON over `stdin` and `stdout`.
* **Communication Model:** It is **bidirectional** and **stateful**. The client sends requests to the agent to process prompts, but the agent also sends requests *back* to the client to read files or execute terminal commands, alongside streaming notifications for real-time UI updates.
* *Note on limits:* Because it relies on `stdio` streams, standard buffer limits apply. Recent updates to Kimi Code CLI bumped the internal `asyncio` StreamReader buffer limit to 100MB to prevent `LimitOverrunError` crashes when handling massive JSON-RPC payloads (like reading massive files in a single line).

### 2. The Core JSON-RPC Lifecycle

A standard conversation via the Kimi Code CLI JSON-RPC API follows a strict phase-by-phase lifecycle.

#### Phase 1: Initialization

The client (your IDE or script) boots up the Kimi Code CLI subprocess and sends an `initialize` request to negotiate capabilities.

**Client Request:**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": 1,
    "clientInfo": { "name": "MyCustomEditor", "version": "1.0.0" },
    "clientCapabilities": {
      "fs": { "readTextFile": true, "writeTextFile": true },
      "terminal": true
    }
  }
}

```

**Kimi Code CLI Response:**
Kimi will respond detailing what it supports, such as loading existing sessions or accepting image prompts (powered by Kimi K2.5's native multi-modality).

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": 1,
    "agentCapabilities": {
      "loadSession": true,
      "promptCapabilities": { "image": true }
    }
  }
}

```

#### Phase 2: Session Management

Because coding tasks require heavy context, Kimi Code CLI maintains stateful sessions. You must create or load a session before prompting.

**Client Request (`session/new`):**

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "session/new",
  "params": {
    "cwd": "/path/to/your/project",
    "mcpServers": []
  }
}

```

*Tip:* You can pass Model Context Protocol (MCP) server configurations here, allowing Kimi to dynamically access external APIs, databases, or enterprise tools during this specific session.

#### Phase 3: The Prompt Loop and Streaming Updates

When you send a prompt, you do not wait for a single massive JSON response. Instead, Kimi acknowledges the request and streams back asynchronous notifications (`session/update`) containing its internal reasoning (Deep Think traces), tool calls, and text chunks.

**Client Request (`session/prompt`):**

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "session/prompt",
  "params": {
    "sessionId": "sess_abc123",
    "content": [
      { "type": "text", "text": "Refactor the auth middleware to use JWT." }
    ]
  }
}

```

**Kimi Code CLI Notifications (Streaming):**
Kimi streams its progress using one-way JSON-RPC notifications (no `id` required).

```json
{
  "jsonrpc": "2.0",
  "method": "session/update",
  "params": {
    "sessionId": "sess_abc123",
    "update": {
      "kind": "message",
      "content": "I'll start by checking the current middleware implementation..."
    }
  }
}

```

#### Phase 4: Reverse Requests (Agent to Client)

Kimi Code CLI doesn't magically edit files on its own when running via ACP. To maintain security and sandbox constraints, **the Agent asks the Client to execute actions**.

If Kimi needs to read a file, it sends a JSON-RPC request to your client, and your client must fulfill it.

**Kimi Request to Client:**

```json
{
  "jsonrpc": "2.0",
  "id": 101,
  "method": "fs/readTextFile",
  "params": {
    "path": "/path/to/your/project/src/middleware.ts"
  }
}

```

Your client script or IDE must read that file and return a standard JSON-RPC `result` payload with the file contents back over `stdin`.

### 3. Integrating with the API

Because it adheres strictly to ACP, you don't actually need to write the raw JSON-RPC boilerplate from scratch unless you want to.

* **Using standard IDEs:** You can plug Kimi directly into Zed, JetBrains IDEs, or VS Code by mapping the custom agent to the command `kimi acp` in their respective configuration files (e.g., `settings.json` or `acp.json`).
* **Building Custom Clients:** You can use the official `@agentclientprotocol/sdk` (TypeScript), `acp-kotlin-sdk` (JVM), or standard Python/Rust libraries to wrap the JSON-RPC lifecycle. You just spawn the `kimi acp` process and attach the SDK to its `stdio` streams.

## Interacting with Kimi via ACP

Here is a practical example of how to build a custom ACP Client in Rust that spawns and controls Kimi Code CLI programmatically.

While you *could* use the official `agent-client-protocol` or `sacp` Rust crates, the best way to understand how ACP actually works under the hood is to write a raw asynchronous JSON-RPC client using `tokio`.

This example demonstrates how to spawn the `kimi acp` subprocess, attach to its standard input/output streams, and send the required JSON-RPC payloads to initialize the agent and start a session.

### Prerequisites

Add the following dependencies to your `Cargo.toml`:

```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"

```

### The Rust Implementation

```rust
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Spawn Kimi Code CLI in ACP server mode
    println!("-> Spawning 'kimi acp' subprocess...");
    let mut child = Command::new("kimi")
        .arg("acp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn kimi acp. Is Kimi CLI installed and in your PATH?");

    // 2. Capture the stdio streams
    let mut stdin = child.stdin.take().expect("Failed to capture stdin");
    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let mut reader = BufReader::new(stdout).lines();

    // 3. Helper closure to send JSON-RPC requests
    let mut send_request = |req: Value| async {
        let mut payload = req.to_string();
        payload.push('\n'); // ACP requires newline-delimited JSON
        stdin.write_all(payload.as_bytes()).await.unwrap();
        stdin.flush().await.unwrap();
    };

    // 4. Send Phase 1: Initialize
    println!("-> Sending 'initialize' request...");
    send_request(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": 1,
            "clientInfo": { "name": "RustDemoClient", "version": "0.1.0" },
            "clientCapabilities": {}
        }
    })).await;

    // 5. Read the Initialization Response
    if let Some(line) = reader.next_line().await? {
        println!("<- Received: {}", line);

        // 6. Send Phase 2: Create a Session (Assuming Init was successful)
        println!("-> Sending 'session/new' request...");
        send_request(json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "session/new",
            "params": {
                // Pass the current working directory so Kimi knows where it is
                "cwd": std::env::current_dir()?.to_string_lossy().to_string(),
                "mcpServers": []
            }
        })).await;
    }

    // 7. Listen to the stream for further responses (Session ID, Prompts, etc.)
    // In a real application, you would parse the JSON here and route it to your UI or state manager.
    while let Some(line) = reader.next_line().await? {
        let parsed: Value = serde_json::from_str(&line)?;

        if let Some(method) = parsed.get("method") {
            println!("<- Kimi Notification [{}]: {}", method, parsed["params"]);
        } else if let Some(id) = parsed.get("id") {
            println!("<- Kimi Response to ID {}: {}", id, parsed["result"]);
            // For the sake of this demo, we exit after getting the session/new response
            if id.as_i64() == Some(2) {
                println!("-> Session established. Exiting demo.");
                break;
            }
        }
    }

    // Clean up the subprocess
    child.kill().await?;
    Ok(())
}

```

### How This Works in Practice

* **Subprocess Management:** We use `tokio::process::Command` so that we do not block the main thread while waiting for Kimi to "think" or generate responses.
* **Newline Delimitation:** You will notice `.push('\n')` before sending payloads to `stdin`. Because ACP communicates over raw byte streams, it uses `\n` to distinguish where one JSON payload ends and the next begins.
* **The Read Loop:** The `while let Some(line) = reader.next_line().await?` loop is the heart of an ACP client. Because the protocol is bidirectional, you must constantly listen to `stdout`. Sometimes it will be a response to a prompt, and sometimes it will be a *reverse request* (e.g., Kimi asking your Rust program to read a file for it).

If you plan to build a production-grade tool, I highly recommend looking into the official `agent-client-protocol` crate on crates.io, which provides strongly typed structs for all the payloads (like `PromptRequest`, `SessionCapabilities`, etc.) so you don't have to manually write `json!` macros.

### Handling Streaming Responses

Handling a prompt stream in the Agent Client Protocol (ACP) is where the architecture really shines. Because Kimi Code CLI is built to emulate the responsiveness of human typing, you don't wait for a single monolithic JSON response. Instead, you process a continuous stream of events over `stdout`.

To do this successfully, your Rust client needs to handle three distinct phases of a prompt turn:

1. **The Request:** Sending the `session/prompt` method with a specific `id`.
2. **The Stream:** Catching asynchronous `session/update` notifications (which have no `id`) to render Kimi's text generation and internal reasoning in real-time.
3. **The Conclusion:** Catching the final response payload that matches your original prompt `id`, signaling that Kimi is done.

Here is how you implement that stream-parsing logic in your Rust `tokio` loop.

### The Rust Implementation: Parsing the Stream

Assuming you have already established a session and extracted the `sessionId` (as shown in the previous example), here is how you send a prompt and process the resulting JSON-RPC stream:

```rust
use std::io::Write;

// ... continuing from the previous initialization and session setup ...

// 1. Send the Prompt Request
let prompt_id = 3;
let session_id = "sess_abc123def456"; // In a real app, parse this from Phase 2

println!("-> Sending 'session/prompt' request...");
send_request(json!({
    "jsonrpc": "2.0",
    "id": prompt_id,
    "method": "session/prompt",
    "params": {
        "sessionId": session_id,
        "content": [
            { "type": "text", "text": "Write a fast fibonacci function in Rust." }
        ]
    }
})).await;

// 2. Parse the Streaming Response
println!("-> Waiting for Kimi's response stream...\n");

while let Some(line) = reader.next_line().await? {
    let parsed: Value = serde_json::from_str(&line)?;

    // Handle One-Way Streaming Notifications
    if let Some(method) = parsed.get("method").and_then(|m| m.as_str()) {
        if method == "session/update" {
            let update = &parsed["params"]["update"];
            let update_type = update["sessionUpdate"].as_str().unwrap_or("unknown");

            match update_type {
                "agent_thought_chunk" => {
                    // Kimi's internal reasoning (Deep Think traces)
                    if let Some(thought) = update["content"]["thought"].as_str() {
                        // Print thoughts to the terminal in grey text
                        print!("\x1b[90m{}\x1b[0m", thought);
                    }
                }
                "agent_message_chunk" => {
                    // Actual response content (text, code, etc.)
                    if let Some(text) = update["content"]["text"].as_str() {
                        // Print the actual response normally
                        print!("{}", text);
                    }
                }
                _ => {
                    // Other updates (e.g., tool calls, mode changes) happen here
                }
            }
            // Flush stdout immediately so chunks appear character-by-character
            std::io::stdout().flush().unwrap();
        }
    }
    // Handle the Final Completion Response
    else if let Some(id) = parsed.get("id").and_then(|i| i.as_i64()) {
        if id == prompt_id {
            let stop_reason = parsed["result"]["stopReason"]
                .as_str()
                .unwrap_or("unknown");

            println!("\n\n-> Turn complete. Stop reason: {}", stop_reason);
            break; // Exit the loop now that the prompt turn is finished
        }
    }
    // Handle Errors
    else if let Some(error) = parsed.get("error") {
        eprintln!("\n-> Kimi Error: {}", error);
        break;
    }
}

```

### Key Concepts to Watch Out For

* **Differentiating Thoughts vs. Messages:** Kimi often streams `agent_thought_chunk` payloads before `agent_message_chunk` payloads. The "thoughts" represent the model's internal reasoning and planning (like "I need to write a recursive function, but an iterative one is faster..."). You typically want to render these differently in your UI—perhaps collapsable or colored gray.
* **The Power of `flush()`:** Because JSON payloads are parsed line-by-line using Tokio, but the text chunks inside them might not end with newline characters, you must explicitly call `std::io::stdout().flush()` in Rust. If you forget this, your terminal will buffer the chunks and output them all at once, ruining the streaming effect.
* **The Final Stop Reason:** The stream is officially over only when the agent sends a JSON-RPC response with the original `id` (in our case, `3`). The `stopReason` will usually say `"done"`. If Kimi is interrupted, it might say `"cancelled"`.


## Handling Reverse Requests

Handling "Reverse Requests" is the most crucial part of building an ACP Client. By design, Kimi Code CLI operates in a sandbox when running as an ACP agent. It cannot actually edit your files or run shell commands directly. Instead, when it decides it needs to look at your code, it sends a JSON-RPC request *to you*, and your Rust program acts as the executor.

While Kimi is waiting for your response, its prompt stream pauses. It will only resume generating text or code after you hand back the result of the action.

Here is how you intercept these requests, execute the local file system operation, and return the data Kimi needs.

### The Rust Implementation: Fulfilling Agent Requests

You need to expand the main read loop we built earlier to detect when an incoming JSON payload contains both an `id` and a `method` (which defines it as a Request rather than a Notification or a Response).

```rust
use std::fs;

// ... inside your `while let Some(line) = reader.next_line().await?` loop ...

let parsed: Value = serde_json::from_str(&line)?;

// 1. Detect Reverse Requests: Has an 'id' and a 'method'
if let (Some(id), Some(method)) = (
    parsed.get("id"),
    parsed.get("method").and_then(|m| m.as_str())
) {
    println!("\n<- Kimi requested action: {} (ID: {})", method, id);

    // 2. Route the request based on the method name
    match method {
        "fs/readTextFile" => {
            // Extract the parameters
            let path = parsed["params"]["path"].as_str().unwrap_or("");
            println!("-> Client executing local read on: {}", path);

            // Perform the actual local file system operation
            let result_payload = match fs::read_to_string(path) {
                Ok(content) => {
                    // Success: return the file content
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": { "content": content }
                    })
                }
                Err(e) => {
                    // Failure: return a formal JSON-RPC error
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32603,
                            "message": format!("Failed to read file: {}", e)
                        }
                    })
                }
            };

            // 3. Send the response back to Kimi's stdin
            let mut payload = result_payload.to_string();
            payload.push('\n');
            stdin.write_all(payload.as_bytes()).await.unwrap();
            stdin.flush().await.unwrap();

            println!("-> Client fulfilled request {}", id);
        }

        "terminal/executeCommand" => {
            // Here you would implement logic to run terminal commands (e.g., `cargo check`)
            // and return the stdout/stderr.
            println!("-> Kimi wants to run a terminal command.");
        }

        // 4. Handle unsupported capabilities gracefully
        _ => {
            let error_payload = json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": format!("Method not found or unsupported by this client: {}", method)
                }
            });
            let mut payload = error_payload.to_string();
            payload.push('\n');
            stdin.write_all(payload.as_bytes()).await.unwrap();
            stdin.flush().await.unwrap();
        }
    }
}
// ... fallback to handling Notifications (session/update) and Agent Responses as shown before ...

```

### Why This Architecture Matters

* **Security and Control:** Because your Rust program handles the actual `std::fs::read_to_string` or `std::process::Command` calls, *you* control the permissions. If Kimi asks to run `rm -rf /`, your client can simply reject it and return an error payload. This is why IDEs love the ACP model.
* **Error Handling:** Notice the JSON-RPC error block (`code: -32603`). If you fail to read the file (e.g., it doesn't exist), you must send an error back. If you simply ignore the request, Kimi Code CLI will hang indefinitely waiting for a response to that specific `id`.
* **Synchronous Illusion:** From the user's perspective, Kimi is seamlessly reading files and writing code. Under the hood, Kimi stream-generates a request, pauses, waits for your client's lightning-fast filesystem response, reads it into its context window, and immediately resumes streaming its output.

## Executing Commands for Kimi

Implementing `terminal/executeCommand` is a massive upgrade for your custom client. It takes Kimi from being just a "smart auto-complete" to a fully autonomous agent that can write code, run the compiler, read the errors, and fix its own bugs.

Because we are already using `tokio`, we can leverage `tokio::process::Command` to execute these shell commands asynchronously. This is critical: if you use the standard library's `std::process::Command`, your entire client will block while the command runs, meaning you won't be able to process any incoming JSON-RPC notifications or heartbeat pings from Kimi.

Here is how you parse the payload, spawn the subprocess, capture the streams, and return the execution results to Kimi.

### The Rust Implementation: Asynchronous Command Execution

We will replace the placeholder for `terminal/executeCommand` inside our `match` block from the previous example. Kimi typically sends the base `command` and an array of `arguments` in the `params`.

```rust
use tokio::process::Command;

// ... inside your match block for Reverse Requests ...

"terminal/executeCommand" => {
    // 1. Extract the command and its arguments
    let command_str = parsed["params"]["command"].as_str().unwrap_or("");
    let args_val = parsed["params"]["arguments"].as_array();

    let mut args = vec![];
    if let Some(arr) = args_val {
        for arg in arr {
            if let Some(s) = arg.as_str() {
                args.push(s);
            }
        }
    }

    println!("-> Kimi executing: {} {:?}", command_str, args);

    // 2. Spawn the process asynchronously using tokio
    // Note: `.output().await` waits for the process to finish and
    // captures stdout and stderr automatically into memory.
    let output_result = Command::new(command_str)
        .args(&args)
        .output()
        .await;

    // 3. Format the JSON-RPC Response
    let result_payload = match output_result {
        Ok(output) => {
            // Convert the raw byte streams into UTF-8 strings
            let stdout_str = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr_str = String::from_utf8_lossy(&output.stderr).into_owned();
            let exit_code = output.status.code().unwrap_or(-1);

            // Return the execution results to Kimi
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "stdout": stdout_str,
                    "stderr": stderr_str,
                    "exitCode": exit_code
                }
            })
        }
        Err(e) => {
            // This happens if the command itself fails to spawn (e.g., binary not found)
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32603,
                    "message": format!("Failed to spawn process: {}", e)
                }
            })
        }
    };

    // 4. Send the payload back over Kimi's stdin
    let mut payload = result_payload.to_string();
    payload.push('\n');
    stdin.write_all(payload.as_bytes()).await.unwrap();
    stdin.flush().await.unwrap();

    println!("-> Terminal execution complete. Results sent to Kimi.");
}

```

### Important Design Considerations

* **Timeouts:** In the code above, `.output().await` will wait indefinitely for the command to finish. If Kimi accidentally runs a server (`cargo run`) instead of a test command (`cargo test`), your client will hang forever. In a production client, you should wrap the `.output().await` call in `tokio::time::timeout(Duration::from_secs(60), ...)` to forcibly kill commands that run too long and return a timeout error to Kimi.
* **Working Directory:** By default, `tokio::process::Command` executes in the directory where your Rust client was started. If Kimi needs to run commands in a specific subdirectory, you should check for a `cwd` field in the `params` and apply it using `.current_dir(path)`.
* **Security & Sandboxing:** Giving an AI raw terminal access is incredibly powerful but obviously risky. A robust client will implement a whitelist of allowed commands (e.g., `cargo check`, `git diff`, `npm run test`) or ask the human user for confirmation via the UI before executing destructive commands.


## Hooking up a Frontend UI

