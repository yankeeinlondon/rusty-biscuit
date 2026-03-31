---
prompt: |-
        Claudine wants to be able to provide a consistent and universal way to either _append to_ or _replace_ the system prompt. Your job is to research in detail how Claude Code handles the system prompt. You should be able to at least answer the following questions:
        - What CLI switches are involved in effecting the system prompt? What does each switch do?
        - What other ways, other than via a CLI switch, can you manipulate what the system prompt will be?
        - Can Agents or Subagents have their own system prompt which is distinct from an orchestrator?
        - what quirks and workarounds do developers talk about with regard to Claude Code and system prompts?
        - have there been any recent changes to how system prompts can be manipulated? If so, when?
        - what format works best when appending to the system prompt? what format works best when replacing the system prompt?

                - pure markdown? XML wrapper of markdown? Other?

        All research and observations should be written to the body of this Markdown document while preserving the Frontmatter data. The Markdown should all be standards based and isomorphic. Tables should be Markdown tables. Links should be Markdown links.

        If any data visuals are thought to be important you should feel free to use Mermaid.js charts by adding in a mermaidjs code block.

        Provide a summary -- a paragraph and some bullet points are an ideal length for the summary -- of this document to STDOUT.
last_updated: 2026-03-30
---

I will begin by researching Claude Code's system prompt handling, focusing on CLI switches, configuration methods, and best practices for prompt manipulation.

I will now investigate the local research documents for Claude Code and conduct a targeted search for its CLI flags to better understand how to manipulate the system prompt.

I will list the `claudine` directory to verify the correct path for the research documents and ensure I'm looking in the right place.

I will list `claudine/docs` to search for the `research` directory mentioned in the skill instructions.

I will list `claudine/docs/research` to confirm the contents and locate the relevant research documents.

I will list `claudine/docs/research/system-prompt` and the other relevant directories to explore the available information on system prompts and agent CLIs.

I will read `claudine/docs/research/system-prompt/claude.md` to identify any existing information and avoid duplication.

I will search for Claude Code's CLI switches, specifically for any flags related to replacing or appending to the system prompt, as well as the preferred format for these prompts.

I will search for any additional quirks or workarounds regarding Claude Code's system prompt, specifically looking for information on its size, context window interaction, and whether an XML wrapper is used for the system prompt.

I will now verify the release date of Claude Code version 2.1.63 and check for any additional recent changes to system prompt manipulation, including the best formatting practices for appending and replacing prompts.

Claude Code provides a robust and layered system for managing its system prompt, ranging from project-wide persistent files to session-specific CLI overrides.

### CLI Switches for System Prompt Manipulation

Claude Code uses three primary switches to manipulate the system prompt at runtime. These switches are **temporary** and apply only to the current session.

| Switch                        | Effect            | Description                                                                                                                                                                                      |
|:------------------------------|:------------------|:-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `--append-system-prompt`      | **Append**        | Injects custom instructions directly above the tool definitions. This is the safest way to add rules without breaking the core agentic behavior.                                                 |
| `--append-system-prompt-file` | **Append (File)** | Loads text from a local file and appends it to the system prompt. Useful for sharing large sets of rules across different sessions or environments.                                              |
| `--system-prompt`             | **Replace**       | The "Nuclear Option." Replaces the entire default "constitution" (except for tool definitions and a basic ID line). Requires providing a complete set of instructions for the agent to function. |

### Non-CLI Manipulation Methods

Beyond CLI switches, Claude Code relies on a hierarchy of files and configurations to determine its behavior:

* **`CLAUDE.md`**: The primary project-level system prompt. Claude reads this file at the start of every session to understand project conventions, build commands, and testing patterns.
* **Subagent Files**: Custom agents defined as `.md` files (typically in `.claude/agents/`) use YAML frontmatter to specify their own models, toolsets, and system instructions, allowing for specialized "experts" within a larger project.
* **`settings.json`**: Global (`~/.claude/`) and project-scoped (`.claude/`) settings can configure "Memory" and "Hooks" (e.g., `onFileEdit`), which act as procedural extensions to the system prompt by enforcing shell-based checks.

### Agents and Subagents

Agents and subagents (now officially referred to as **Agents** as of v2.1.63) have **entirely distinct system prompts** from the main orchestrator. They operate in isolated context windows, which prevents "context bloat" in the main conversation. When an agent is spawned via the `Agent` tool (formerly `Task`), it receives its own specific instructions and returns only a summary to the orchestrator.

### Quirks and Workarounds

* **Placement Priority**: Appended prompts are injected *above* the tool definitions. This is a strategic placement to ensure the model prioritizes user-defined rules before processing the large list of available tools.
* **Token Consumption**: The default "constitution" is roughly **3,100 tokens**. Customizing the prompt or adding large `CLAUDE.md` files can significantly impact the context window, leading developers to use subagents for high-volume research tasks to keep the main history lean.
* **Output Style Conflict**: If a user has a `/output-style` configured, those instructions are placed *before* any instructions from `--append-system-prompt`.
* **The "Agent" Rename**: Developers should note that the `Task` tool was renamed to `Agent` in February 2026. Older documentation or prompts referring to "Task" may need updating for full compatibility with v2.1.63+.

### Recent Changes (v2.1.63 - Feb 28, 2026)

* **HTTP Hooks**: A shift from local shell-only hooks to HTTP-based hooks, allowing the system prompt's enforcement layer to interact with remote APIs.
* **Git Worktree Support**: `CLAUDE.md` and "auto-memory" are now shared across all worktrees, ensuring a consistent system prompt experience regardless of which branch is active.
* **Template Variables**: The system prompt now uses variables like `{{OPUS_ID}}` instead of hardcoded model names, facilitating smoother transitions between different Claude models.

### Recommended Formats

| Action        | Recommended Format       | Rationale                                                                                                                                                                                       |
|:--------------|:-------------------------|:------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Appending** | **Pure Markdown**        | Since it is added to an existing structured prompt, simple Markdown lists and headers are typically sufficient for Claude to integrate the new rules.                                           |
| **Replacing** | **XML-Wrapped Markdown** | When replacing the default prompt, you must provide the structure yourself. XML tags (e.g., `<rules>`, `<context>`) are critical for Claude to distinguish between different types of guidance. |

```mermaid
graph TD
    A[Start Session] --> B{Check CLI Flags}
    B -- --system-prompt --> C[Replace Default Constitution]
    B -- --append-system-prompt --> D[Combine with Default]
    B -- None --> E[Load Default Constitution]
    C --> F[Load CLAUDE.md]
    D --> F
    E --> F
    F --> G[Inject Subagent Context if active]
    G --> H[Final System Prompt Assembly]
```

**Summary of Research**
Claude Code provides a sophisticated system for prompt manipulation that balances ease of use (via `CLAUDE.md`) with total control (via `--system-prompt`).

* **Control Switches:** Use `--append-system-prompt` for additive rules and `--system-prompt` for a full persona reset.
* **Layered Context:** Instructions flow from the default constitution, through the project-specific `CLAUDE.md`, to session-specific flags.
* **Agent Isolation:** Specialized agents maintain their own prompts and context windows, optimizing performance and reducing "hallucinations" caused by context overload.
* **Recent Maturity:** The v2.1.63 release (Feb 2026) solidified the "Agent" terminology and expanded prompt-driven capabilities into parallel batch processing and HTTP-integrated hooks.
