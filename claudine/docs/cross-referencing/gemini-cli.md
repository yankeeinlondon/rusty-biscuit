---
prompt: |-
  Do a deep dive on Gemini CLI's support for both "slash commands", "skills", and "agents/subagents" (if supported).

  - Describe the directory structure conventions for each standard (both User scope and Repo scope)
  - What metadata is supported/used/required in:
      - "skill" documents
      - "slash command" documents
      - "agent/subagent" documents
  - Where does Gemini CLI suggest that "scripts" or "executables" be saved
  - What "built-in" slash commands come with Gemini CLI
  - What are some important differences that exist between Gemini CLI's skills and Anthropic/Claude Code's skills?
      - Are there any common gotchas that a developer should be aware of? How can these gotcha's be navigated around?
  - What are some important difference that exist between Gemini CLI's slash commands and Anthropic/Claude Code's slash commands?
      - Are there any common gotchas that a developer should be aware of? How can these gotcha's be navigated around?
  - If agents/subagents are supported:
      - What is the interaction model between the orchestrator and the subagent?
      - What are some best practices in leveraging concurrency and subagents in Gemini CLI?

  ## What is a Skill?

 When we refer to "skills" we're talking about the _semi-standard_ which Anthropic created with a base `SKILL.md` file linking to a tree of other resources (docs, scripts) which allows the Agent to selectively choose what content to explore.
---


### **1. The "Skill" Standard in Gemini CLI**

Unlike Anthropic's ecosystem, which often treats a "Skill" as a distinct entity to be "loaded" to save context tokens, Gemini CLI does not have a reserved `SKILL.md` parser hardcoded into its binary.

However, because Gemini 1.5 Pro/Flash has a massive (1M-2M) token window, the implementation of this standard shifts from **"Selective Navigation"** (Claude) to **"Holistic Ingestion"** (Gemini).

#### **Directory Structure Conventions**

To implement the `SKILL.md` standard effectively in Gemini CLI, the recommended structure organizes skills as "Context Modules" rather than executable packages.

**Repo Scope:**

```text
.gemini/
├── modules/ (or "skills/")
│   ├── react-best-practices/
│   │   ├── SKILL.md            # Entry point / Map
│   │   ├── components.md       # Specific guidelines
│   │   └── hooks.md            # Specific guidelines
│   └── database-migration/
│       ├── SKILL.md
│       └── scripts/
│           └── verify_schema.sh

```

**User Scope (`~/.gemini/`):**

* Global skills are typically stored in `~/.gemini/docs/` or `~/.gemini/knowledge/`.
* Gemini CLI users often alias these directories using a system environment variable or a `.gemini/settings.json` path to allow quick references like `@db-skill`.

---

### **2. Metadata & Documents**

In the "Skill Tree" pattern, the document acts as the interface between the human's intent and the agent's execution.

#### **A. "Skill" Documents (`SKILL.md`)**

In Gemini CLI, this file acts as a **Context Anchor**.

* **Supported Metadata:** Gemini does not enforce YAML frontmatter for logic, but it *heavily utilizes* standard Markdown links for relationship understanding.
* **Required Content:**
* **High-Level Goal:** "This skill helps you generate clean React components."
* **Map of Resources:** A list of relative links to child documents.
* **Trigger Phrases:** (Implicit) Keywords that help the model decide *when* to use this info.



**Example `SKILL.md` for Gemini:**

```markdown
# React Component Skill

Use this skill when the user asks to "scaffold a component" or "refactor UI".

## Resources
- [Component Structure](./structure.md): Rules for folder organization.
- [Styling Guidelines](./styling.md): Tailwind vs. CSS Modules rules.

## Associated Scripts
- `scripts/scaffold.sh`: Run this to generate the file boilerplate.

```

#### **B. "Slash Command" Documents (.toml)**

In this pattern, the Slash Command is merely a **pointer** or **shortcut** to load the Skill Tree.

* **Metadata:**
* `prompt`: Instead of containing the logic, the prompt simply injects the skill file.



**Example `commands/react.toml`:**

```toml
description = "Loads React expertise"
prompt = """
I am loading the React Skill Tree. Please review the following context and assist the user:
{{@skills/react-best-practices/SKILL.md}}
"""

```

---

### **3. Scripts & Executables Location**

When using the Skill Tree standard, Gemini CLI conventions suggest keeping executable logic **colocated with the documentation** rather than in a hidden global bin.

* **Location:** Inside the specific skill folder, typically in a `./scripts/` subdirectory relative to the `SKILL.md` file.
* **Reasoning:** This allows the `SKILL.md` to reference the script using a relative path (`./scripts/migrate.sh`), which Gemini can resolve easily when it reads the file.
* **Execution:** The Agent does not "auto-run" these. It reads the `SKILL.md`, sees the reference to the script, and then suggests: *"I see a script at `skills/db/scripts/migrate.sh`. Would you like me to run that using the shell tool?"*

---

### **4. Gemini CLI vs. Anthropic/Claude Code: Skills**

This is where the divergence in interaction models is most distinct.

| Feature | Gemini CLI | Anthropic / Claude Code |
| --- | --- | --- |
| **Discovery** | **Explicit.** You generally must `@mention` the skill file or folder to bring it into focus. | **exploratory.** Claude is often set up to "crawl" or "ls" directories to find relevant skill files autonomously. |
| **Context Strategy** | **"Ingest All."** Due to the 1M+ token window, the standard Gemini pattern is to load the *entire* skill folder (`@skills/react/`) at once. | **"Traverse Tree."** Designed to read the root `SKILL.md`, pick *one* relevant sub-link, and read only that to save tokens. |
| **File Formats** | **Agnostic.** reads `.md`, `.py`, `.sh` equally well as text context. | **Markdown-Centric.** Heavily optimized for structured Markdown with XML tags. |

#### **Common Gotchas & Workarounds**

* **Gotcha 1: The "Lazy Reader" Effect**
* *Issue:* If you provide a `SKILL.md` with links like `[Details](./details.md)` but don't explicitly tell Gemini to "read the linked files," it might only read the top-level file and hallucinate the details.
* *Workaround:* Use the **`@dir`** syntax (e.g., `@skills/my-skill/`) instead of just referencing the single file. This forces the CLI to load the directory content recursively, ensuring all child nodes of the skill tree are in context immediately.


* **Gotcha 2: Relative Path Confusion**
* *Issue:* If the `SKILL.md` references a script at `./scripts/run.sh`, but the CLI is running from the project root, the model might try to run `run.sh` without the full path and fail.
* *Workaround:* In your `SKILL.md`, instruct the model explicitly: *"Always resolve script paths relative to the project root, or navigate to this directory before running scripts."*



---

### **5. Gemini CLI vs. Anthropic/Claude Code: Slash Commands**

| Feature | Gemini CLI | Anthropic / Claude Code |
| --- | --- | --- |
| **Definition** | **Configuration (`.toml`).** Rigid structure defined in `commands/`. | **Prompt-based.** Often defined loosely in system prompts or `CLAUDE.md`. |
| **Flexibility** | **Static.** The command template is fixed until you edit the `.toml`. | **Dynamic.** Can often be tweaked on the fly by changing the system prompt text. |
| **Parameters** | **`{{args}}` Injection.** Simple string replacement. | **Natural Language Parsing.** Can interpret "run tests for the api" and map it to a command intelligently. |

#### **Common Gotchas & Workarounds**

* **Gotcha: Argument Rigidness**
* *Issue:* Gemini's `.toml` commands blindly inject `{{args}}`. If a user types `/refactor please help me`, the prompt receives "please help me" as the code argument, potentially confusing the model.
* *Workaround:* Write robust prompt wrappers in your `.toml` file.
* *Bad:* `Refactor this: {{args}}`
* *Good:* `The user has provided the following input arguments: "{{args}}". If this looks like a filename, read it. If it is a request, interpret it.`





---

### **6. Agents & Subagents (Revised)**

Since Gemini CLI relies on Tool Delegation rather than persistent sub-agent processes:

#### **Interaction Model**

1. **Orchestrator (Main Context):** The user prompt + loaded `SKILL.md` defines the "Persona".
2. **Execution (Ephemeral Subagent):** When the model decides to run a script defined in a Skill, it effectively delegates control to that script.
3. **Return:** The script's `stdout` is piped back into the Orchestrator's context.

#### **Concurrency Best Practices**

* **Do not rely on chat-based subagents.** Gemini CLI cannot "chat with itself" in a background thread easily.
* **"Skill as a Script":** The most robust way to create a "subagent" in Gemini CLI is to write a Python script using the Google GenAI SDK, place it in `skills/my-agent/run.py`, and have the main CLI trigger it. This allows the sub-script to have its own independent loop and context, returning only the final result to the main CLI.

