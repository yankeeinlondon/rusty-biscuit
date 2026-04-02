---
prompt: |-
        Research the abilities of Gemini CLI to "resume" a session.
        - How is the session ID captured in an interactive session?
        - How is the session ID captured in a non-interactive session?
        - How can the CLI be leveraged to "resume" with a session id?
        - Does the interactive environment provide a slash command or some other means of resuming?
        - Does this Gemini CLI provide hooks which can stop session execution on an interactive/human-in-the-loop prompt and capture the question? 

                - If yes, describe how Gemini CLI could receive interactive prompts (questions, tool call permissions, etc.) during a non-interactive session which would allow Claudine to receive the question, pose the question itself, and then resume with an answer.

        - What quirks or complications does Gemini CLI pose for developers working with the resume functionality?
        - Is the "resumable" content stored locally at all or the only local thing a caller get's a session ID to reference the session state on the server?

        All research and observations should be written to the body of this Markdown document while preserving the Frontmatter data. The Markdown should all be standards based and isomorphic. Tables should be Markdown tables. Links should be Markdown links.

        If any data visuals are thought to be important you should feel free to use Mermaid.js charts by adding in a mermaidjs code block.

        Provide a summary -- a paragraph and some bullet points are an ideal length for the summary -- of this document to STDOUT.
last_updated: 2026-04-02
---

I will use `cli_help` to research Gemini CLI's session management, including session ID capture, resume functionality, and interactive hooks.

I will use `cli_help` to get more details on the `BeforeTool` hook and its interaction with the `ask_user` tool in non-interactive mode, specifically to understand how a wrapper like Claudine could intercept questions and provide answers.

I will search the workspace for files related to `hooks` and `ask_user` to understand their implementation and how they can be used for session interaction and interception.

I will read `claudine/lib/README.md` to understand how it describes Gemini hooks and their interaction with interactive prompts.

I will search for `resume` in the root directory to see if there is any additional Gemini-related code or documentation I may have missed.

I will read `claudine/lib/src/actions/mod.rs` to verify the `HookDecision` variants and understand how the `Ask` decision is implemented.

I will read `claudine/lib/src/actions/hook_response.rs` to examine the `HookDecision` variants and their roles in session interaction.

The Gemini CLI provides a robust session management and resume system designed for both human-in-the-loop and fully automated workflows. Sessions are automatically persisted locally, enabling users to pick up exactly where they left off or allowing external wrappers like Claudine to programmatically handle interruptions.

### Session Identification & Capture

Gemini CLI uniquely identifies every interaction using a UUID-based session ID. The capture method depends on the execution mode:

* **Interactive Sessions**: Users can retrieve the current session ID or browse recent sessions using the `/resume list` slash command or by starting the CLI with the `gemini --list-sessions` flag.
* **Non-Interactive Sessions**: When running in headless mode (e.g., `gemini --prompt "task" --output-format jsonl`), the CLI emits an `init` event as the first object in the JSONL stream. This object contains the `session_id`, allowing a wrapper to store it for future recovery.

### Resuming Sessions

Resuming a session restores the entire conversation history, tool execution results, and metadata. This can be performed via the CLI or within an interactive session:

* **CLI Flags**:

    * `gemini --resume <UUID>`: Directly loads the specific session.
    * `gemini -r <index>`: Loads the $N$th most recent session (e.g., `-r 1` for the last session).

* **Interactive Slash Commands**:

    * `/resume`: Opens the interactive session browser.
    * `/resume <UUID>`: Switches the current chat to the specified session.
    * `/resume list`: Displays a table of recent sessions with their IDs and timestamps.

### Interactive Hooks & Interception

Gemini CLI supports a "blocking hook" architecture that allows external wrappers to intercept events, including interactive prompts.

* **`permission_request` & `human_in_the_loop`**: These events fire when a tool requires explicit user confirmation.
* **`before_tool` Interception**: A hook registered for the `ask_user` tool can intercept the question posed by the agent.

#### Programmable Interaction Workflow (Claudine Integration)

If Gemini CLI is running in a non-interactive environment and needs to "ask" a question, it can be configured to use a blocking hook:

1. **Event Trigger**: The agent calls `ask_user` with a question.
2. **Hook Execution**: Gemini CLI executes the registered `before_tool` hook (Claudine).
3. **Capture**: Claudine receives the JSON payload containing the question.
4. **External Interactivity**: Claudine stops its own execution (or uses its own TTY) to present the question to the user and collect an answer.
5. **Resume**: Claudine returns a `HookResponse` with `HookDecision::Allow` and the user's answer in the `updated_input` or `additional_context` field.
6. **Continuation**: Gemini CLI receives the answer from the hook's stdout and provides it to the model, effectively "resuming" the automated session with human input.

### Storage & Infrastructure

Session data is stored strictly **locally** to ensure privacy and speed.

| Component        | Location                                   | Description                               |
|:-----------------|:-------------------------------------------|:------------------------------------------|
| **Chat History** | `~/.gemini/tmp/<project_hash>/chats/`      | Raw JSON session logs.                    |
| **Checkpoints**  | `~/.gemini/history/<project_hash>`         | Shadow Git repository for code snapshots. |
| **Metadata**     | `~/.gemini/tmp/<project_hash>/checkpoints` | Session state and checkpoint mapping.     |

### Quirks and Limitations

* **Project Scoping**: Sessions are cryptographically tied to the project root directory. A session created in `~/projects/A` cannot be resumed while working in `~/projects/B`.
* **30-Day Retention**: By default, sessions are automatically purged after 30 days. This is configurable via the `general.sessionRetention` setting.
* **Turn Limits**: If the `maxSessionTurns` limit is reached, a resumed session will immediately exit (non-interactive) or pause (interactive).
* **No Cloud Sync**: Since storage is local, session IDs are not portable across different machines without manual synchronization of the `~/.gemini` directory.

### Session Lifecycle State Machine

```mermaid
graph TD
    A[Start Session] --> B{Interactive?}
    B -- Yes --> C[Standard TTY]
    B -- No --> D[JSONL Stream]
    C --> E[Slash /resume list]
    D --> F[init Event: session_id]
    E --> G[Capture ID]
    F --> G
    G --> H[Execution Interrupted]
    H --> I{Resume via CLI?}
    I -- gemini --resume ID --> J[Restore Context]
    J --> K[Continue Execution]
```

Gemini CLI's session management provides a robust foundation for building autonomous agents that can be safely monitored and manually steered when necessary. The combination of local shadow-git checkpointing and JSONL-based session IDs ensures that no work is lost, even in the event of a crash or network failure.

* **Capture**: Session IDs are found in `/resume list` (interactive) or the `init` JSONL event (non-interactive).
* **Resume**: Use `gemini --resume <UUID>` to restore history and checkpoints.
* **Hooks**: `before_tool` can intercept `ask_user` to provide a bridge between non-interactive execution and human intervention.
* **Storage**: All data is stored locally in `~/.gemini/tmp/`, scoped by the project root.
* **Policy**: Sessions expire after 30 days by default.
