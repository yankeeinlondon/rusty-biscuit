---
prompt: |-
    Research the abilities of Qwen CLI to "resume" a session.
    - How is the session ID captured in an interactive session?
    - How is the session ID captured in a non-interactive session?
    - How can the CLI be leveraged to "resume" with a session id?
    - Does the interactive environment provide a slash command or some other means of resuming?
    - Does this Qwen CLI provide hooks which can stop session execution on an interactive/human-in-the-loop prompt and capture the question? 

                    - If yes, describe how Qwen CLI could receive interactive prompts (questions, tool call permissions, etc.) during a non-interactive session which would allow Claudine to receive the question, pose the question itself, and then resume with an answer.

    - What quirks or complications does Qwen CLI pose for developers working with the resume functionality?
    - Is the "resumable" content stored locally at all or the only local thing a caller get's a session ID to reference the session state on the server?

    All research and observations should be written to the body of this Markdown document while preserving the Frontmatter data. The Markdown should all be standards based and isomorphic. Tables should be Markdown tables. Links should be Markdown links.

    If any data visuals are thought to be important you should feel free to use Mermaid.js charts by adding in a mermaidjs code block.

    Provide a summary -- a paragraph and some bullet points are an ideal length for the summary -- of this document to STDOUT.
last_updated: 2026-04-02
---

# Qwen CLI Session Resumption and Interactivity

Qwen Code (binary: `qwen`) provides robust session management and resumption capabilities, largely derived from its Gemini CLI lineage but with significant architectural differences in how it handles hooks and interactivity.

## Session ID Capture

Session IDs are the primary keys for resumption and are surfaced differently depending on the engagement mode.

| Mode                            | Capture Mechanism | Location / Event                                           |
|:--------------------------------|:------------------|:-----------------------------------------------------------|
| **Interactive**                 | File System       | `~/.qwen/projects/<project-hash>/chats/` (JSONL filenames) |
| **Non-interactive**             | `stream-json`     | `system` event with `subtype: "session_start"`             |
| **ACP (Agent Client Protocol)** | JSON-RPC          | Response to `session/new` request                          |

### Interactive Session ID

In a standard TUI session, the ID is not explicitly displayed to the user by default. However, it is captured in the project-scoped history files. Running `qwen -r` (without an ID) launches a TUI-based session picker that lists recent sessions by timestamp and summary, allowing users to select an ID visually.

### Non-interactive Session ID

When running in headless mode with `--output-format stream-json`, the first event emitted is an initialization payload:

```json
{
    "type": "system",
    "subtype": "session_start",
    "session_id": "b5b53246-4d23-42e5-9adb-71ccaefd09ba",
    "model": "qwen3-coder-plus"
}
```

## Resuming Sessions

The Qwen CLI offers two primary flags for resumption from the terminal:

- **`--continue` / `-c`**: Resumes the most recent session for the current project directory.
- **`--resume [id]` / `-r [id]`**: Resumes a specific session by ID. If the ID is omitted, it opens the interactive session picker.

### Resumption Lifecycle

Resuming a session loads the full conversation history from the local JSONL transcript. The agent maintains state by re-playing this history to the model provider.

**Note:** Resumption requires that `--chat-recording` is enabled (it is `true` by default). If recording is disabled, sessions are ephemeral and cannot be resumed.

## Interactive Environment Commands

While inside an active TUI session, Qwen CLI supports several "slash commands," though it lacks a dedicated `/resume` command (resumption is handled at startup).

- `/model`: Switch models mid-session.
- `/auth`: Manage authentication (OAuth or API keys).
- `/think` / `/no_think`: Toggle chain-of-thought reasoning (for supported models).
- `/memory show`: Display currently loaded context from `QWEN.md` files.
- `/memory refresh`: Reload hierarchical context files.
- `Shift+Tab`: Cycle through approval modes (`plan`, `default`, `auto-edit`, `yolo`).

## Hooks and Human-in-the-Loop Interactivity

Qwen CLI does **not** provide a unified, user-configurable hook system like Claude Code (e.g., there is no `UserPromptSubmit` hook). Instead, developers must leverage specific integration surfaces to achieve interactivity during non-interactive sessions.

### Interception via ACP Mode (Recommended)

The most effective way for Claudine to receive interactive prompts and questions during a "non-interactive" session is to run Qwen CLI in **ACP Mode** (`qwen --acp`).

In ACP mode:

1. The Agent cannot execute tools or read files directly; it sends **Reverse Requests** to the client (Claudine).
2. When the Agent needs user input or permission, it sends a `session/request_permission` JSON-RPC message.
3. **Claudine Strategy**: Claudine acts as the ACP client, receives the permission/tool request, pauses its own execution to prompt the human user, and then forwards the answer back to Qwen to resume the turn.

### Interception via SDK Callback

If using the Qwen SDK instead of the CLI binary, developers can implement the `canUseTool` callback. This is the **only blocking event** in the Qwen ecosystem that allows for:

- Denying a tool call (with a message to the model).
- Approving a tool call with modified inputs.
- Halting the session.

### Sequence for Interactivity in Headless Mode

```mermaid
sequence_tree
    participant User
    participant Claudine
    participant Qwen_CLI (ACP)
    
    User->>Claudine: Runs non-interactive task
    Claudine->>Qwen_CLI: Start session (qwen --acp)
    Qwen_CLI->>Claudine: session/update (Streaming text)
    Qwen_CLI->>Claudine: session/request_permission (Tool: execute_shell)
    Note over Claudine: Execution Paused
    Claudine->>User: "Agent wants to run 'rm -rf'. Allow?"
    User->>Claudine: "No, explain why first."
    Claudine->>Qwen_CLI: permission response (Deny + Message)
    Qwen_CLI->>Claudine: session/update (Agent explains)
```

## Quirks and Complications

- **Fragmented Surfaces**: Unlike Claude Code's unified `settings.json` hooks, Qwen's visibility is split between SDK callbacks, internal subagent hooks, and headless stream events.
- **60-Second Timeout**: The `canUseTool` permission callback has a hard-coded 60-second deadline. If the human-in-the-loop (via Claudine) doesn't respond in time, the tool is auto-denied.
- **No Prompt Interception**: There is no native hook to intercept the user's initial or subsequent text prompts before they reach the model.
- **ACP Versioning**: Qwen currently implements ACP v1. Integration with clients requiring ACP v2 (like JetBrains 2025.3+) may require a bridge or compatibility layer.
- **OAuth Limits**: Qwen OAuth authentication cannot be performed in headless/CI environments. Non-interactive sessions must use API keys via `OPENAI_API_KEY` or `DASHSCOPE_API_KEY`.

## Storage and Persistence

"Resumable" content is stored **exclusively on the local file system**.

- **Path**: `~/.qwen/projects/<cwd-hash>/chats/`
- **Format**: Line-delimited JSON (JSONL).
- **Security**: Content is not encrypted by default.
- **Server State**: While the model provider (e.g., DashScope) may log turns for safety/training, the CLI's "session state" is reconstructed entirely from the local history files. Deleting these files makes a session ID unresumable.

## Summary

Qwen CLI supports session resumption through local JSONL history files indexed by session IDs. While it lacks a unified hook system for intercepting user prompts, it provides a powerful **ACP (Agent Client Protocol) mode** that transforms the Agent into a subordinate process.

* **Resumption**: Use `--continue` for the last session or `--resume <id>` for specific ones.
* **Discovery**: Capture IDs from the `system` event in `stream-json` output or the `~/.qwen/projects/` directory.
* **Interactivity**: Leverage **ACP mode** to intercept tool and permission requests, allowing a wrapper like Claudine to inject human-in-the-loop decisions.
* **Storage**: All session data is local; there is no server-side "resume by ID" if local history is cleared.
