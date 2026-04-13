# Technical Design: Claudine Stream Fusion (Hook-to-Stream Correlation)

## Overview

Some agent CLIs (OpenCode, Qwen, Gemini) filter their `stdout` or emit incomplete structured streams. However, they all support hooks that provide rich event payloads via a side-channel. Currently, the `claudine` wrapper only sees the `stdout` stream, while `claudine handle` (the hook callback) runs in a separate, isolated process.

This design introduces **Stream Fusion**: an IPC bridge between `claudine handle` and the active `claudine` wrapper process. This allows the wrapper to "fuse" events from both sources into a single, deduplicated, and authoritative event stream.

## Architecture

### 1. IPC Bridge (Unix Domain Sockets)

- **Wrapper Side:** When `claudine <provider>` (the wrapper) starts, it creates a Unix Domain Socket (UDS) at a random path (e.g., `/tmp/claudine-ipc-<random>.sock`).
- **Environment Variable:** The wrapper injects the `CLAUDINE_IPC_PATH` environment variable into the agent CLI subprocess.
- **Hook Side:** When `claudine handle` is invoked by the agent CLI, it checks for `CLAUDINE_IPC_PATH`.
- **Forwarding:** If the path is present, `claudine handle` connects to the socket and forwards the event payload as NDJSON.
- **Blocking Support:** For blocking hooks (e.g., `BeforeTool`, `PermissionRequest`), `claudine handle` waits for a response from the socket before proceeding.

### 2. Event Fusing & Deduplication

The wrapper's execution loop (`run_child_stream`) will be extended to consume events from two concurrent sources:
1.  **Stdout Stream:** Parsed by the provider-specific `StreamParser`.
2.  **IPC Hooks:** Received via the UDS listener.

Both sources feed into a **Deduplicating Sink**.

### 3. Correlation Strategy

To deduplicate events appearing on both channels, we use provider-specific correlation IDs:

| Provider | Correlation ID(s) |
| :--- | :--- |
| **OpenCode** | `sessionID` + `callID` (Tools) \| `requestID` (Permissions/Questions) |
| **Claude** | `session_id` + `tool_use_id` |
| **Generic** | `AgenticEvent` type + Hash of payload fields (excluding timestamp) |

The `DeduplicatingSink` maintains a short-lived LRU cache of recently processed IDs. If a duplicate arrives (usually the stdout line following a hook), it is silently ignored if the hook has already been processed.

## Implementation Details

### 1. New Library Module: `claudine::ipc`

A new module in `claudine` library to handle UDS client/server logic.

```rust
// claudine/lib/src/ipc/mod.rs
pub struct IpcMessage {
    pub event: AgenticEvent,
    pub payload: Value,
    pub is_blocking: bool,
}

pub struct IpcResponse {
    pub decision: Option<HookDecision>,
    pub updated_input: Option<Value>,
    // ...
}
```

### 2. `claudine handle` Updates

Modify `claudine/cli/src/commands/handle.rs`:

```rust
pub async fn run(args: HandleArgs) -> Result<()> {
    let payload = read_stdin()?;
    if let Ok(ipc_path) = std::env::var("CLAUDINE_IPC_PATH") {
        // 1. Connect to UDS
        // 2. Send payload
        // 3. If blocking, wait for response and print to stdout
        // 4. Return
    }
    // Fallback: execute actions locally if no IPC bridge
    dispatch_canonical(&payload, ...).await
}
```

### 3. Wrapper Updates (`exec.rs`)

Modify `claudine/cli/src/commands/wrap/exec.rs`:

- In `run_child_stream`, spawn a `tokio` task (or thread) to listen on the UDS.
- The listener task receives `IpcMessage`, converts them to `EventMeta`, and calls `sink.on_event(...)`.
- If the message is blocking, the listener waits for the sink/dispatcher to produce a result, then sends it back over the socket.

### 4. `DeduplicatingSink`

A wrapper around `StreamEventSink` that prevents double-triggering of actions.

```rust
pub struct DeduplicatingSink {
    inner: Box<dyn StreamEventSink>,
    cache: LruCache<CorrelationId, Instant>,
}

impl StreamEventSink for DeduplicatingSink {
    fn on_before_tool(&mut self, meta: &EventMeta) {
        let id = extract_id(meta);
        if self.cache.contains(&id) { return; }
        self.cache.put(id, Instant::now());
        self.inner.on_before_tool(meta);
    }
    // ...
}
```

## Sequence Diagram

```mermaid
sequence_diagram
    participant User
    participant Wrapper as claudine (Wrapper)
    participant CLI as Agent CLI (e.g. OpenCode)
    participant Hook as claudine handle (Hook)

    User->>Wrapper: claudine opencode "do X"
    Wrapper->>Wrapper: Start IPC Server (UDS)
    Wrapper->>CLI: Spawn with CLAUDINE_IPC_PATH=...
    CLI->>Hook: Fire Hook (BeforeTool)
    Hook->>Wrapper: IPC: BeforeTool { callID: "123", ... }
    Wrapper->>Wrapper: Dispatch Actions (e.g. Play Sound)
    Wrapper->>Wrapper: Update TUI: "Running Tool..."
    Wrapper-->>Hook: IPC Response: { decision: "allow" }
    Hook->>CLI: Exit 0 (Allow)
    CLI->>CLI: Execute Tool
    CLI->>Wrapper: Stdout: { type: "tool_use", callID: "123", ... }
    Wrapper->>Wrapper: DeduplicatingSink: Already seen "123", ignore.
    CLI->>Wrapper: Stdout: Tool Output...
    Wrapper->>User: Display Result
```

## Benefits

1.  **OpenCode Visibility:** Enables real-time visibility into OpenCode's hidden permission prompts and tool executions.
2.  **Centralized UI:** All TUI updates, sounds, and TTS are managed by the persistent wrapper process, ensuring a smooth and coordinated user experience.
3.  **Reliability:** Even if a provider filters `stdout` for "safety" or "cleanliness", Claudine still receives the full event stream via the hook side-channel.
4.  **No Double-Triggering:** Correct correlation ensures that sounds and actions fire exactly once per logical event.

## Risks & Considerations

- **Socket Cleanup:** The wrapper must ensure the UDS file is deleted on exit (even on crash).
- **Timeouts:** IPC calls between `handle` and the wrapper should have strict timeouts to prevent hanging the agent CLI if the wrapper becomes unresponsive.
- **Race Conditions:** An event might arrive on `stdout` *milliseconds before* the hook IPC message. The deduplication logic must handle both arrival orders.
