---
prompt: |-
        Claudine wants to be able to provide a consistent and universal way to either _append to_ or _replace_ the system prompt. Your job is to research in detail how Qwen CLI handles the system prompt. You should be able to at least answer the following questions:
        - What CLI switches are involved in effecting the system prompt? What does each switch do?
        - What other ways, other than via a CLI switch, can you manipulate what the system prompt will be?
        - Can Agents or Subagents have their own system prompt which is distinct from an orchestrator?
        - what quirks and workarounds do developers talk about with regard to Qwen CLI and system prompts?
        - have there been any recent changes to how system prompts can be manipulated? If so, when?
        - what format works best when appending to the system prompt? what format works best when replacing the system prompt?

                - pure markdown? XML wrapper of markdown? Other?

        All research and observations should be written to the body of this Markdown document while preserving the Frontmatter data. The Markdown should all be standards based and isomorphic. Tables should be Markdown tables. Links should be Markdown links.

        If any data visuals are thought to be important you should feel free to use Mermaid.js charts by adding in a mermaidjs code block.

        Provide a summary -- a paragraph and some bullet points are an ideal length for the summary -- of this document to STDOUT.
last_updated: 2026-03-30
---

I will begin by investigating the `.qwen` directory and searching the codebase for references to Qwen to understand if it refers to a specific internal implementation or an external tool.I will read the `QwenCliAgent` implementation and any related documentation to understand how it handles system prompts and identifies relevant CLI switches.I will examine the `QwenCliAgent` implementation, the Qwen adapter, and the configuration file to understand CLI switches, prompt manipulation, and system prompt handling.I will investigate the `.qwen` directory for any documentation or configuration files and use `web_fetch` to gather more information from the official Qwen Code CLI documentation, specifically regarding system prompt support.I will read `cli-developer.md` in the `.qwen/agents/` directory to check if subagents have their own system prompts.I will check for frontmatter in `.qwen/agents/cli-developer.md` using `head`, and search for a `QWEN.md` file in the root to investigate the "replacing" versus "appending" of system prompts.I will search for `QWEN.md` across the entire repository to confirm its location and role as a system prompt source.I will check `claudine/lib/src/config/qwen.rs` for configuration details, perform a Google search to find information on Qwen CLI system prompts and command-line arguments, and search for "system prompt" in `claudine/lib` to better understand its implementation.I will use `web_fetch` to explore the documentation at `https://qwenlm.github.io/qwen-code-docs/` and search for "qwen-code" on the internet to find more details about its features and usage.I will use `web_fetch` to access the documentation at the specified URLs for Qwen Code CLI commands, agents, and skills to gather more information on system prompt support and other features.I will check for a `.qwen/settings.json` file in the current directory and explore the `.qwen` directory for any other configuration files to understand how system prompts and agent settings are defined.I'll search for `QWEN.md` again to ensure I didn't miss it, and then I'll search for "system prompt" across the codebase to identify any related configurations or documentation.I will read the three identified research files to gather comprehensive information on Qwen CLI's system prompt behavior, subagent support, and context file configuration.I have gathered extensive details from the research files and can now answer most of the user's questions. I will verify the subagent format by checking `claudine/docs/research/cross-referencing/qwen-cli.md` to confirm if it uses Markdown with YAML frontmatter or another format.Qwen Code CLI (package `@qwen-code/qwen-code`) manages its system prompt through a hierarchical context system and specialized entity definitions rather than traditional CLI flags. Unlike other agents that allow full replacement via a single switch, Qwen prioritizes a "supplemental" model where project-specific and user-specific instructions are layered on top of a foundational, model-optimized base prompt.

### CLI Switches and System Prompt Manipulation

The Qwen CLI does not provide a direct `--system-prompt` flag for overriding or appending content. Instead, it relies on the following mechanisms:

* **Hierarchical Context Files:** The primary method for manipulating the system prompt is through `QWEN.md` files (the filename is configurable via `context.fileName` in `settings.json`). Qwen automatically searches for and concatenates these files from:

    1. `~/.qwen/QWEN.md` (Global user instructions)
    2. `QWEN.md` in the current directory and all parent directories up to the project root or home directory.

* **Import Syntax:** Within any context file, you can use the `@path/to/file.md` or `::file @path` syntax to modularize instructions and import external Markdown documents.
* **Runtime Commands:** Users can manipulate the loaded context during a session using `/memory show` to view the combined prompt and `/memory refresh` to force a re-scan of the hierarchy.

### Agents and Subagents

Qwen CLI supports **Subagents**, which are specialized AI assistants that possess their own distinct system prompts, toolsets, and isolated conversation contexts.

* **Definition Format:** Subagents are defined using **Markdown files with YAML frontmatter** located in `.qwen/agents/` or `~/.qwen/agents/`.
* **System Prompt Isolation:** The body content of the subagent's Markdown file (everything after the frontmatter) serves as its unique system prompt.
* **Orchestration:** The main orchestrator delegates tasks to these subagents using the `Task` tool, providing them with a specific instruction set that does not conflict with the orchestrator's own system prompt.

### Quirks and Developer Observations

* **Supplemental Nature:** A common "gotcha" for developers is that `QWEN.md` content is strictly **supplemental**. It is appended to the built-in system prompt; there is no native CLI way to fully replace the core instructions that guide the model's tool-use behavior.
* **Tool Conflicts:** Project-level restrictions in `settings.json` (specifically the `coreTools` property) can override a subagent's individual tool declarations, leading to unexpected "tool not found" errors even if the subagent's prompt specifies access.
* **ACP Limitations:** When operating via the Agent Client Protocol (ACP), certain features like "Plan Mode" may fail to correctly filter tools, and subagent progress was historically difficult to monitor due to missing stream updates (largely resolved in v0.9.0+).

### Recent Changes

* **Flag Graduation:** The `--acp` flag was graduated from experimental status in January 2026.
* **Skills Integration:** The Agent Skills standard was introduced in late 2025 (v0.6.0), allowing the model to dynamically load context from `SKILL.md` files when a task matches the skill's description.

### Recommended Formats

| Operation        | Best Format                 | Rationale                                                                |
|:-----------------|:----------------------------|:-------------------------------------------------------------------------|
| **Appending**    | Pure Markdown (`QWEN.md`)   | Native support for hierarchical concatenation and easy version control.  |
| **Specializing** | Markdown + YAML Frontmatter | Required for Subagents and Custom Commands to define metadata/tools.     |
| **Modularizing** | `@path` Imports             | Allows for clean separation of concerns without duplicating prompt text. |

Qwen CLI does not utilize XML wrappers for its system prompt components; it adheres to a "Markdown-first" philosophy for both configuration and instructions.

---

**Summary of Qwen CLI System Prompt Research**

Qwen CLI employs a hierarchical, supplemental system prompt model centered around `QWEN.md` context files rather than direct CLI switches. While it lacks a native "full replacement" flag, it provides robust support for specialized subagents with isolated prompts and an on-demand "Skills" system.

* **Hierarchical Layering:** Instructions are gathered from global and project-level `QWEN.md` files and appended to the base prompt.
* **Subagent Independence:** Subagents use Markdown with YAML frontmatter to define distinct roles, tools, and system prompts.
* **Pure Markdown Focus:** No XML wrappers are used; all prompt manipulation relies on standard Markdown and specific import syntaxes.
* **Configuration over Flags:** Prompt behavior is largely governed by `settings.json` and file-system discovery rather than ephemeral CLI arguments.
