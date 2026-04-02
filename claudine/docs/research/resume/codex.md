---
prompt: |-
        Research the abilities of Codex CLI to "resume" a session.
        - How is the session ID captured in an interactive session?
        - How is the session ID captured in a non-interactive session?
        - How can the CLI be leveraged to "resume" with a session id?
        - Does the interactive environment provide a slash command or some other means of resuming?
        - Does this Codex CLI provide hooks which can stop session execution on an interactive/human-in-the-loop prompt and capture the question? 

                - If yes, describe how Codex CLI could receive interactive prompts (questions, tool call permissions, etc.) during a non-interactive session which would allow Claudine to receive the question, pose the question itself, and then resume with an answer.

        - What quirks or complications does Codex CLI pose for developers working with the resume functionality?
        - Is the "resumable" content stored locally at all or the only local thing a caller get's a session ID to reference the session state on the server?

        All research and observations should be written to the body of this Markdown document while preserving the Frontmatter data. The Markdown should all be standards based and isomorphic. Tables should be Markdown tables. Links should be Markdown links.

        If any data visuals are thought to be important you should feel free to use Mermaid.js charts by adding in a mermaidjs code block.

        Provide a summary -- a paragraph and some bullet points are an ideal length for the summary -- of this document to STDOUT.
last_updated: 2026-04-02
---

I will begin by activating the `claudine` skill to leverage its specialized knowledge of agentic CLI platforms, including Codex CLI, and then search the codebase for any existing research or documentation related to Codex's session management.

I will read the Codex-specific research files and then perform a broad search for "Codex" and "resume" or "session" across the codebase to uncover any additional details.

I will examine the `claudine` source code, specifically looking for Codex-related implementation details in `claudine/lib/src/agents/`, and I will also read the Codex research files identified earlier to understand its session management.

I'll list the contents of the `claudine` directory to confirm the correct file paths.

I will read the Codex research file from the skill's specified path in `.gemini/skills/claudine/`.

The initial research in `claudine/research/hooks/codex.md` identified `thread-id` as the session identifier, present in both `notify` hook payloads and `--json` streams. While it's established as a correlation key, the specific mechanism for "resuming" a session remains unclear. I will now examine `claudine/research/cross-referencing/codex.md` and search the codebase for `codex` command usage, specifically looking for flags like `--thread` or `--session` that might facilitate session resumption.

The research has pinpointed `/resume` as the key slash command for reloading conversations. My next steps are to determine its CLI-equivalent for non-interactive sessions, identify any associated flags like `--thread` or `--session`, and locate where these conversations are stored. I'll search the `claudine` source code and documentation for references to `/resume` or `resume` to see if this functionality is already integrated or documented.

The `claudine` grep results are promising, with `claudine/lib/src/agents/codex.rs` explicitly mentioning `"codex exec resume"` and setting `resume_supported: true`. I'll now examine `claudine/lib/src/agents/codex.rs` to understand the resume command and flag definitions.

`claudine/lib/src/agents/codex.rs` explicitly lists `"codex exec resume"` as an entrypoint and confirms session storage in `~/.codex/sessions/` and `~/.codex/history.jsonl`. This points to `codex exec resume` as the potential CLI equivalent to the `/resume` slash command, likely taking a session ID and an optional new prompt. I'll now search for other occurrences of `"codex exec resume"` in the `claudine` directory to confirm its usage and syntax.

The grep results confirm `codex exec resume <session_id> [prompt]` and `codex exec resume --last` for non-interactive session resumption. I now need to determine how session IDs are captured across interactive and non-interactive sessions, and if hooks exist to capture questions from interactive prompts. I'll start by reading `claudine/docs/research/agent-cli/codex.md` to find these details and confirm session storage in `~/.codex/sessions/` and `~/.codex/history.jsonl`.

The Codex CLI provides a robust session management system centered around **Thread IDs** (the internal term for session IDs). These identifiers are consistently exposed across both interactive and non-interactive modes, enabling sophisticated "resume" workflows and external orchestration.

### Session ID Capture

#### Interactive Sessions

In interactive TUI mode, the session ID is primarily captured through the **`notify` hook**. When configured in `~/.codex/config.toml`, Codex executes an external command after each turn, passing a JSON payload as an argument.

* **Payload Field:** `thread-id` (kebab-case UUID).
* **Alternative:** Session transcripts are written to `~/.codex/sessions/<YYYY>/<MM>/<DD>/<session-id>/` and indexed in `~/.codex/history.jsonl`.

#### Non-interactive Sessions

When running in automation mode via `codex exec --json`, the session ID is emitted early in the stream.

* **Event Type:** `thread.started`
* **Payload Field:** `thread_id` (snake_case UUID).

### Resuming Sessions

Codex allows resuming sessions through both CLI subcommands and internal slash commands.

| Mode                | Command                             | Description                                                           |
|:--------------------|:------------------------------------|:----------------------------------------------------------------------|
| **Interactive**     | `codex resume [ID]`                 | Opens the TUI. If ID is omitted, it shows a session picker.           |
| **Interactive**     | `codex resume --last`               | Resumes the most recent session directly.                             |
| **Non-interactive** | `codex exec resume <ID> [prompt]`   | Resumes session ID in `exec` mode, optionally providing a new prompt. |
| **Non-interactive** | `codex exec resume --last [prompt]` | Resumes the most recent session non-interactively.                    |
| **Slash Command**   | `/resume`                           | Within the TUI, reloads a previously saved conversation.              |

### Human-in-the-Loop Orchestration

The Codex CLI does **not** provide native "blocking" hooks (e.g., hooks that can pause for approval or modify tool calls). The `notify` hook is fire-and-forget. However, Claudine can leverage the `resume` functionality to achieve human-in-the-loop control:

1. **Initial Run:** Claudine launches `codex exec --json "task"`.
2. **Detection:** Claudine parses the JSONL stream. If the model emits an `agent_message` that is a question (or if a tool fails requiring clarification), the turn completes.
3. **Capture:** Claudine extracts the `thread_id` and the question.
4. **Interaction:** Claudine presents the question to the user and collects an answer.
5. **Resume:** Claudine calls `codex exec resume <thread_id> "<user_answer>"` to continue the work.

### Storage and Persistence

Resumable content is stored **locally on the host machine**. The session ID serves as a reference to a specific directory in the user's home folder.

* **Location:** `~/.codex/sessions/<YYYY>/<MM>/<DD>/<session-uuid>/`
* **Metadata:** `~/.codex/history.jsonl` (contains summaries and correlation IDs).
* **Persistence Policy:** Configurable via `[history] persistence = "save-all"`.

### Quirks and Complications

* **Experimental JSON:** The `--json` stream (and its alias `--experimental-json`) is subject to schema changes.
* **Notify Hook Delivery:** The `notify` payload is passed as a command-line argument, not via `stdin`, which can cause shell escaping issues or character limit truncations for very large messages.
* **Exec Approval Policy:** In `exec` mode, the approval policy is effectively locked to `never`. Direct approval prompting is only available in the interactive TUI.
* **Kebab vs. Snake:** The `notify` hook uses kebab-case (`thread-id`), while the `--json` stream uses snake_case (`thread_id`).

### Session Lifecycle and Resume Flow

```mermaid
graph TD
    A[Start: codex exec --json] --> B[Capture thread_id from thread.started]
    B --> C{Agent turn complete?}
    C -->|Question Asked| D[Claudine traps turn.completed]
    D --> E[User provides answer]
    E --> F[codex exec resume ID 'answer']
    F --> B
    C -->|Task Finished| G[Final Output]
    C -->|Error/Failure| H[Handle via Resume or Retry]
```
