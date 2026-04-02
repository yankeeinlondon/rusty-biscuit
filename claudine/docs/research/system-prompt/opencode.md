---
prompt: |-
        Claudine wants to be able to provide a consistent and universal way to either _append to_ or _replace_ the system prompt. Your job is to research in detail how OpenCode CLI handles the system prompt. You should be able to at least answer the following questions:
        - What CLI switches are involved in effecting the system prompt? What does each switch do?
        - What other ways, other than via a CLI switch, can you manipulate what the system prompt will be?
        - Can Agents or Subagents have their own system prompt which is distinct from an orchestrator?
        - what quirks and workarounds do developers talk about with regard to OpenCode CLI and system prompts?
        - have there been any recent changes to how system prompts can be manipulated? If so, when?
        - what format works best when appending to the system prompt? what format works best when replacing the system prompt?

                - pure markdown? XML wrapper of markdown? Other?

        All research and observations should be written to the body of this Markdown document while preserving the Frontmatter data. The Markdown should all be standards based and isomorphic. Tables should be Markdown tables. Links should be Markdown links.

        If any data visuals are thought to be important you should feel free to use Mermaid.js charts by adding in a mermaidjs code block.

        Provide a summary -- a paragraph and some bullet points are an ideal length for the summary -- of this document to STDOUT.
last_updated: 2026-04-02
---

# OpenCode CLI System Prompt Research

OpenCode CLI (by Anomaly) provides a highly flexible, layered architecture for manipulating the system prompt. Since the release of **v1.3.0** in early 2026, the platform has shifted toward a "crisp" prompting philosophy, reducing default instruction volume to improve model responsiveness and reduce "token burn."

## CLI Switches and Usage

The OpenCode CLI primarily uses two switches for direct system prompt manipulation in non-interactive mode.

### Primary Switches

| Switch     | Type                | Description                                                                                                                                                                                                        |
|:-----------|:--------------------|:-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `--system` | **Override/Append** | Injects custom instructions or a persona for the session. By default, this appends to the "base" provider instructions (e.g., "You are an AI assistant...") but overrides other project-level custom instructions. |
| `--prompt` | **Task**            | Specifies the actual request or user query. In non-interactive mode (`opencode run`), this is required if no positional argument is provided.                                                                      |

### Execution Examples

```bash
# Append specific coding standards for a single run
opencode run "Refactor auth.ts" --system "Use functional patterns only; no classes."

# Use a local file to define a temporary persona
opencode run "Analyze security" --system "$(cat security-guidelines.md)"
```

## System Prompt Manipulation Methods

Beyond CLI switches, OpenCode supports several layered mechanisms for system prompt control, ordered here from lowest to highest precedence.

### 1. Project-wide `AGENTS.md`

If an `AGENTS.md` file is found in the project root, OpenCode automatically discovers it and **appends** its content to the system prompt for all sessions in that project. This is the recommended way to enforce project-specific rules (e.g., "Always run `just test` before committing").

### 2. Configuration Files (`opencode.json`)

Instructions can be defined globally (`~/.config/opencode/opencode.json`) or at the project level (`opencode.json`) using the `instructions` array or the `agent` configuration object.

- **`instructions`**: An array of strings concatenated into the final prompt.
- **`agent`**: Specific configuration for built-in or custom agents.

### 3. Custom Agent Definitions (`.opencode/agents/*.md`)

Developers can create custom agents by placing Markdown files in the `agents/` directory. Each file can contain a `prompt` field in its YAML frontmatter or use the file body as the prompt. These are distinct from the primary "Build" agent.

### 4. Plugin Hooks (`experimental.chat.system.transform`)

Introduced in **v1.2.x**, this hook allows programmatic, real-time transformation of the system prompt array immediately before it is sent to the LLM. It is often used for dynamic context injection (e.g., inserting current build status or linting errors).

### 5. Environment Variables

- `OPENCODE_CONFIG_CONTENT`: Allows passing a raw JSON configuration string, which can include `instructions` or `agent` overrides, useful for CI/CD pipelines.

## Agents and Subagents

OpenCode supports a hierarchical prompt structure where **Agents** and **Subagents** have their own distinct system prompts.

- **Primary Agents**: Switchable via the TUI (using `Tab`). Each (e.g., Build, Plan) has a dedicated instruction set.
- **Subagents**: Invoked via the `task` tool or `@mention` syntax. They operate in **isolated child sessions** with a fresh context and a system prompt defined by their specific `.md` or JSON definition. They do not inherit the parent session's history, ensuring they focus strictly on the delegated task.

## Quirks and Workarounds

### The "Bloated Prompt" Problem

A common developer complaint in late 2025 was "prompt bloat," where default instructions from multiple providers (Claude, Gemini, etc.) conflicted and caused models to become overly verbose or cautious.

- **Workaround**: Many developers now use the `experimental.chat.system.transform` hook to "strip" default instructions or replace them with condensed versions to improve model "crispiness."

### Hook Safety Mechanism

If the `experimental.chat.system.transform` hook returns an empty `system` array, OpenCode **automatically restores the original system prompt**.

- **Workaround**: To effectively "replace" the prompt, developers must clear the array and then push at least one non-empty string of their own instructions.

### Hook Sequence

Because plugins run sequentially, a downstream plugin might accidentally overwrite or append to a prompt string added by an upstream plugin.

- **Workaround**: Developers use unique XML-style tags (e.g., `<plugin-context>`) to wrap their injections, making it easier for subsequent hooks to identify and modify specific blocks.

## Recent Changes (2026)

The **v1.3.0 (March 2026)** release introduced significant changes:

- **Auto Compact**: Automatically summarizes long conversation histories into the system prompt to stay within token limits while preserving state.
- **Prompt Slots**: A new feature allowing the injection of project-specific instructions into pre-defined "slots" in the system prompt without needing to override the entire configuration.
- **Provider-Specific Refactoring**: Prompts were refactored into "Beast" (GPT-4/o1), "Codex" (GPT-5.4), and "Anthropic" (Claude 3.5+) variants to optimize performance for different reasoning architectures.

## Best Formatting Practices

OpenCode is optimized for **Markdown**, but the strategy differs depending on whether you are appending or replacing.

### Appending to the System Prompt

**XML-Wrapped Markdown** is the most effective format for appending. Wrapping instructions in tags helps the model distinguish between "Base Instructions," "Project Context," and "Session-Specific Rules."

```markdown
<project-rules>
- Use the `just` runner for all tasks.
- Return errors in `thiserror` format.
</project-rules>
```

### Replacing the System Prompt

**Pure Markdown** works best for total replacement. Since you have full control over the context, the model does not need delimiters to separate your content from original instructions. Standard Markdown headers (`## Instructions`, `## Standards`) provide sufficient structure.

## Summary

OpenCode CLI handles system prompts through a layered approach, combining provider defaults with project-specific `AGENTS.md` files, configuration settings, and real-time plugin transformations.

- **Key Switches**: `--system` for ad-hoc instructions and `--prompt` for the specific task.
- **2026 Shifts**: v1.3.0 introduced Auto Compact, Prompt Caching, and specialized prompts (e.g., `PROMPT_BEAST`) to improve performance and reduce token costs.
- **Isolation**: Agents and Subagents maintain their own distinct system prompts and isolated contexts.
- **Best Practice**: Use XML-wrapped Markdown for appending to ensure clarity, while pure Markdown is preferred for complete replacements.
