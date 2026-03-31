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
last_updated: 2026-03-30
---

To research the OpenCode CLI's handling of the system prompt, I analyzed the project's internal documentation, expert research from the `claudine` and `acp` skills, and current web-based technical references.

OpenCode CLI (v1.2.x) employs a multi-layered approach to system prompt composition, balancing project-specific rules with core agent instructions.

### **System Prompt Manipulation Mechanisms**

OpenCode provides three primary methods for influencing the system prompt, each with distinct behaviors regarding whether they append to or replace the existing instructions.

#### **1. CLI Switches**

The `opencode` binary includes flags specifically for non-interactive (headless) and session-start manipulation.

| Switch                 | Type     | Behavior        | Description                                                                                                                                                                                    |
|:-----------------------|:---------|:----------------|:-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `--system <text\|file>` | Override | **Replace**     | Completely replaces the default system prompt for the current session.                                                                                                                         |
| `--agent <name>`       | Select   | **Replace**     | Switches to a custom agent profile, using that agent's unique prompt instead of the default "Build" prompt.                                                                                    |
| `--prompt <text>`      | Input    | **User Prompt** | Standard user input. Note: In some legacy versions (v0.x), this occasionally acted as a system prompt in specific headless modes, but in v1.x, it is consistently treated as the user message. |
| `-s`                   | Alias    | **Session**     | **Caution:** `-s` is frequently an alias for `--session`, not `--system`. Use the long-form flag for prompt overrides to avoid ambiguity.                                                      |

#### **2. File-Based Configuration (The "Project Layer")**

OpenCode automatically discovers specific files in the project root to refine the agent's behavior.

* **`AGENTS.md` (Append):** This is the standard location for project-wide rules. Instructions found here are **appended** to the system prompt. It is the preferred way to enforce coding standards (e.g., "Always use tabs") without losing the agent's core capabilities.
* **`CONTEXT.md` (Injected Context):** Content here is injected into the conversation context as high-level project documentation. While not technically part of the system prompt "instructions," it acts as persistent context.
* **`.opencode/agents/*.md` (Replace):** Defining a new agent in this directory allows for a completely distinct system prompt. When invoked via `opencode --agent <name>`, the agent's Markdown body **replaces** the default system instructions.

#### **3. Programmatic Manipulation (Plugins)**

The OpenCode plugin system (located in `.opencode/plugins/`) provides the most granular control.

* **`experimental.chat.system.transform`:** This hook receives an array of system prompt strings (`output.system`).

    * **To Append:** `output.system.push("New rule")`.
    * **To Replace:** Clear the array and add a new string. However, OpenCode implements a **safety mechanism**: if the array is returned entirely empty, it silently restores the original default prompt to prevent the agent from becoming "un-instructioned."

---

### **Agent and Subagent Distinctness**

OpenCode supports a robust hierarchical prompt model where subagents are isolated from the primary orchestrator.

* **Isolation:** When a primary agent invokes a subagent (via the `task` tool), a fresh child session is created.
* **Distinct Prompts:** The subagent uses its own system prompt defined in its specific agent file (e.g., `.opencode/agents/rust-developer.md`). It does **not** inherit the parent's `AGENTS.md` content unless specifically configured to do so, ensuring that the subagent remains focused on its specialized domain (e.g., testing or security) without the "noise" of the general orchestrator's rules.

---

### **Prompt Composition Hierarchy**

The following diagram illustrates how OpenCode assembles the final system prompt sent to the LLM.

```mermaid
graph TD
    A[Core Provider Header] --> B[Environment Info]
    B --> C[Core Agent Instructions]
    C --> D{Source Selected?}
    D -- Default --> E[Build Agent Prompt]
    D -- --agent flag --> F[Custom Agent Prompt]
    E --> G[AGENTS.md Content]
    F --> G
    G --> H[Plugin Transforms]
    H --> I[Final System Prompt]
    
    subgraph "Append Layer"
    G
    end
    
    subgraph "Replace Layer"
    E
    F
    end
```

---

### **Best Formats for System Prompts**

Research suggests a **hybrid approach** is most effective for modern models (especially Claude 3.5/4/4.5, which OpenCode frequently targets).

| Format            | Best Use Case                      | Rationale                                                                                                                                                                |
|:------------------|:-----------------------------------|:-------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Pure Markdown** | **Appending** (`AGENTS.md`)        | High readability for human developers; aligns with project documentation standards.                                                                                      |
| **XML Wrappers**  | **Replacing** (Specialized Agents) | Models like Claude demonstrate significantly higher "instruction following" and "data isolation" when constraints are wrapped in tags like `<rules>` or `<constraints>`. |

**Format Recommendation:**

* **Structure:** Use Markdown headings (`# Heading`) for general organization.
* **Segmentation:** Use XML-style tags (`<project_standards>...</project_standards>`) within the Markdown to encapsulate critical rules or few-shot examples. This prevents "instruction leakage" where the model confuses a rule with the code it is analyzing.

---

### **Quirks, Workarounds, and Recent Changes**

#### **Common Quirks**

* **Prompt Bloat:** OpenCode's default "Build" prompt is notoriously verbose. Developers often "work around" this by creating a minimal custom agent (`minimal.md`) and running the CLI with `--agent minimal`.
* **The Safety Mechanism:** As noted, if a plugin clears the system prompt array, the original is restored. To achieve a "true" replacement via plugins, you must ensure the array contains at least one non-empty string.
* **`AGENTS.md` vs `CLAUDE.md`:** While OpenCode natively supports `AGENTS.md`, it does **not** natively read `CLAUDE.md` unless the `claudine` wrapper or a specific compatibility plugin is active.

#### **Recent Changes (Late 2025 - Early 2026)**

* **v1.0.190 (Dec 2025):** Introduction of the **Skill System**. This fundamentally changed prompt composition. Skills are no longer part of the initial system prompt; instead, they are "paged in" via a `skill` tool call, reducing initial token "bloat" while effectively appending the `SKILL.md` content to the session context on-demand.
* **v1.2.x (Feb 2026):** Stabilization of the `experimental.chat.system.transform` hook, allowing for the first time truly dynamic, multi-plugin prompt manipulation.

---

### **Summary of Findings**

OpenCode CLI provides a sophisticated, layered system for prompt management that prioritizes project-level rules while allowing for total overrides when necessary.

* **Manipulation:** Use `--system` to replace, `AGENTS.md` to append, and the `system.transform` hook for programmatic logic.
* **Distinctness:** Subagents are isolated and carry their own unique system prompts, preventing context pollution between the orchestrator and specialized workers.
* **Format:** A hybrid approach using Markdown for structure and XML tags for data encapsulation provides the best balance of human readability and machine precision.
* **Recent Evolution:** The move toward on-demand "Skill" loading has shifted the paradigm from massive static system prompts to dynamic, tool-assisted context injection.
