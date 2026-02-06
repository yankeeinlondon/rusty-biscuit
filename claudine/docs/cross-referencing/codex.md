---
prompt: |-
  Do a deep dive on Codex CLI's support for both "slash commands", "skills", and "agents/subagents" (if supported).
  - Describe the directory structure conventions for each standard (both User scope and Repo scope) - What metadata is supported/used/required in:
      - "skill" documents
      - "slash command" documents
      - "agent/subagent" documents
  - Where does Codex CLI suggest that "scripts" or "executables" be saved - What "built-in" slash commands come with Codex CLI - What are some important differences that exist between Codex CLI's skills and Anthropic/Claude Code's skills?
      - Are there any common gotchas that a developer should be aware of? How can these gotcha's be navigated around?
  - What are some important difference that exist between Codex CLI's slash commands and Anthropic/Claude Code's slash commands?
      - Are there any common gotchas that a developer should be aware of? How can these gotcha's be navigated around?
  - If agents/subagents are supported:
      - What is the interaction model between the orchestrator and the subagent?
      - What are some best practices in leveraging concurrency and subagents in Codex CLI?
---
## OpenCode Deep Dive: Slash Commands, Skills, and Agents

### 1. Directory Structure Conventions

OpenCode supports three main extensibility mechanisms, each with specific directory structure conventions at both **User scope** (global) and **Repo scope** (project-level):

| Mechanism            | User Scope (Global)                                                           | Repo Scope (Project)                                               | Claude-Compatible Fallback            |
|----------------------|-------------------------------------------------------------------------------|--------------------------------------------------------------------|---------------------------------------|
| **Skills**           | `~/.config/opencode/skills/<name>/SKILL.md`<br>`~/.claude/skills/<name>/SKILL.md` | `.opencode/skills/<name>/SKILL.md`<br>`.claude/skills/<name>/SKILL.md` | `.agents/skills/<name>/SKILL.md`      |
| **Slash Commands**   | `~/.config/opencode/commands/*.md`                                            | `.opencode/commands/*.md`                                          | ❌ Not supported (GitHub issue #6985) |
| **Agents/Subagents** | `~/.config/opencode/agents/*.md`                                              | `.opencode/agents/*.md`                                            | ❌ Not applicable                     |

**Key Structure Details:**

- **Skills**: Each skill lives in its own folder with a `SKILL.md` file inside. The folder name must match the `name` field in the frontmatter
- **Commands**: Markdown files directly in the `commands/` folder; filename becomes the command name (e.g., `test.md` → `/test`)
- **Agents**: Markdown files directly in the `agents/` folder; filename becomes the agent name (e.g., `review.md` → `@review` agent)

**Scripts/Executables Location:**
OpenCode suggests storing helper scripts in `.opencode/scripts/` . This is a convention for project-scoped executable utilities that skills or agents can invoke via the `bash` tool. Example:

```bash
mkdir -p .opencode/scripts
chmod +x .opencode/scripts/opencode_image_gen.py
```

---

### 2. Metadata Support by Document Type

#### **Skill Documents (SKILL.md)**

Required and supported frontmatter fields :

| Field           | Required | Description                                                                       |
|-----------------|----------|-----------------------------------------------------------------------------------|
| `name`          | ✅ Yes   | 1–64 chars, lowercase alphanumeric with single hyphens. Must match directory name |
| `description`   | ✅ Yes   | 1–1024 chars, used by agent to choose when to load the skill                      |
| `license`       | ❌ No    | SPDX license identifier (e.g., `MIT`)                                             |
| `compatibility` | ❌ No    | Target platform (e.g., `opencode`)                                                |
| `metadata`      | ❌ No    | Free-form key-value map (strings only) for extensibility                          |

**Example SKILL.md:**

```yaml
---
name: git-release
description: Create consistent releases and changelogs
license: MIT
compatibility: opencode
metadata:
  audience: maintainers
  workflow: github
---
## What I do
- Draft release notes from merged PRs
- Propose a version bump
...
```

#### **Slash Command Documents (*.md)**

Supported frontmatter fields :

| Field         | Required | Description                                    |
|---------------|----------|------------------------------------------------|
| `description` | ❌ No    | Shown in TUI command completion                |
| `agent`       | ❌ No    | Which agent to use (e.g., `build`, `plan`)     |
| `model`       | ❌ No    | Specific model ID to use for this command      |
| `template`    | ❌ No    | Alternative to body content (JSON config only) |

The **body content** of the markdown file becomes the prompt template executed when the command is invoked.

**Example Command (`test.md`):**

```yaml
---
description: Run tests with coverage
agent: build
model: anthropic/claude-3-5-sonnet-20241022
---
Run the full test suite with coverage report and show any failures.
Focus on the failing tests and suggest fixes.
```

#### **Agent/Subagent Documents (*.md)**

Supported frontmatter fields :

| Field         | Required | Description                                                               |
|---------------|----------|---------------------------------------------------------------------------|
| `name`        | ❌ No    | Agent identifier (defaults to filename)                                   |
| `description` | ❌ No    | Shown in available agents list                                            |
| `mode`        | ❌ No    | `primary`, `subagent`, or `all` (default: `all`)                          |
| `model`       | ❌ No    | Model ID string (e.g., `anthropic/claude-sonnet-4-20250514`)              |
| `temperature` | ❌ No    | Sampling temperature (0.0–1.0)                                            |
| `tools`       | ❌ No    | Object mapping tool names to boolean (e.g., `write: true`, `bash: false`) |
| `permission`  | ❌ No    | Fine-grained permissions (e.g., `skill: {"internal-*": "deny"}`)          |
| `hidden`      | ❌ No    | Boolean to hide from `@` autocomplete (for internal subagents)            |
| `color`       | ❌ No    | Hex color or theme name for UI display                                    |

**Example Agent:**

```yaml
---
description: Reviews code for quality and best practices
mode: subagent
model: anthropic/claude-sonnet-4-20250514
temperature: 0.1
tools:
  write: false
  edit: false
  bash: false
permission:
  task:
    "*": "deny"
    "code-reviewer": "allow"
hidden: false
color: "#ff6b6b"
---
You are in code review mode. Focus on:
- Code quality and best practices
- Security considerations
Provide constructive feedback without making direct changes.
```

---

### 3. Built-in Slash Commands

OpenCode comes with the following built-in slash commands available in the TUI :

| Command     | Alias                  | Description                                     | Keybind    |
|-------------|------------------------|-------------------------------------------------|------------|
| `/compact`  | `/summarize`           | Compact the current session (summarize history) | `ctrl+x c` |
| `/commands` | —                      | Show all available commands                     | —          |
| `/models`   | —                      | List available models                           | `ctrl+x m` |
| `/agents`   | —                      | List available agents                           | —          |
| `/status`   | —                      | Show session configuration and token usage      | —          |
| `/mcp`      | —                      | Show MCP server status                          | —          |
| `/init`     | —                      | Create or update `AGENTS.md` file               | `ctrl+x i` |
| `/connect`  | —                      | Add a provider to OpenCode                      | —          |
| `/new`      | `/clear`               | Start a new session                             | `ctrl+x n` |
| `/sessions` | `/resume`, `/continue` | List and switch between sessions                | `ctrl+x l` |
| `/share`    | —                      | Create shareable conversation link              | `ctrl+x s` |
| `/undo`     | —                      | Undo recent changes (requires Git)              | —          |
| `/redo`     | —                      | Redo previously undone changes                  | `ctrl+x r` |
| `/export`   | —                      | Export conversation to Markdown                 | `ctrl+x x` |
| `/editor`   | —                      | Open external editor for composing              | `ctrl+x e` |
| `/details`  | —                      | Toggle tool execution details                   | `ctrl+x d` |
| `/themes`   | —                      | List available themes                           | `ctrl+x t` |
| `/help`     | —                      | Show help dialog                                | `ctrl+x h` |
| `/exit`     | `/quit`, `/q`          | Exit OpenCode                                   | `ctrl+x q` |

---

### 4. OpenCode Skills vs. Claude Code Skills: Key Differences

| Aspect                | OpenCode                                                                                             | Claude Code                    | Gotcha/Navigation                                                                                                                                                                                                                                                                                                                                                 |
|-----------------------|------------------------------------------------------------------------------------------------------|--------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Directory Naming**  | `skill/` (singular)                                                                                  | `skills/` (plural)             | ⚠️ **Gotcha**: OpenCode's glob pattern looks for `skill/**/SKILL.md` but Claude uses `skills/`. OpenCode has compatibility support but the singular/plural mismatch caused discovery issues in earlier versions (fixed in recent versions to check both). **Workaround**: Use symlinks if needed: `ln -s ~/.claude/skills/<name> ~/.config/opencode/skill/<name>` |
| **Discovery Scope**   | Loads from `.opencode/`, `.claude/`, and `.agents/` directories with upward traversal until git root | Loads from `.claude/` only     | ✅ OpenCode is more flexible for monorepos                                                                                                                                                                                                                                                                                                                        |
| **Loading Mechanism** | Explicit `skill` tool call required; progressive disclosure (metadata first, full content on demand) | Similar progressive disclosure | ⚠️ **Gotcha**: Skills aren't automatically loaded just by being present; the agent must choose to call `skill({name: "..."})`. Ensure descriptions are clear so agents know when to use them                                                                                                                                                                      |
| **Permissions**       | Fine-grained pattern-based permissions in `opencode.json` (`allow`/`deny`/`ask`)                     | Simpler allowlist              | ✅ OpenCode offers more granular control                                                                                                                                                                                                                                                                                                                          |
| **Metadata Support**  | `metadata` map in frontmatter                                                                        | Similar                        | ⚠️ Both use the open standard, but verify specific keys are supported                                                                                                                                                                                                                                                                                             |

**Common Gotchas:**

1. **Skill Not Showing Up**: Verify `SKILL.md` is ALL CAPS, frontmatter has `name` and `description`, and the directory name matches the `name` field exactly
1. **Claude Skills Not Found**: If migrating from Claude Code, ensure skills are in `~/.claude/skills/` (plural), which OpenCode now supports as a fallback
1. **Permissions Denied**: Check `opencode.json` permission patterns—skills with `deny` are hidden from the agent entirely

---

### 5. OpenCode Slash Commands vs. Claude Code Slash Commands

| Aspect              | OpenCode                                                                                            | Claude Code                    | Gotcha/Navigation                                                                                                                                          |
|---------------------|-----------------------------------------------------------------------------------------------------|--------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **File Location**   | `.opencode/commands/*.md`                                                                           | `.claude/commands/*.md`        | ⚠️ **Gotcha**: OpenCode does NOT support `.claude/commands/` compatibility (GitHub issue #6985). You must manually copy or symlink commands when migrating |
| **Namespacing**     | Supports subdirectories: `.opencode/commands/frontend/component.md` → `/project:frontend:component` | Similar project/user namespace | ✅ Feature parity here                                                                                                                                     |
| **Arguments**       | Use `$ARGUMENTS` placeholder in content                                                             | `$ARGUMENTS` placeholder       | ✅ Compatible syntax                                                                                                                                       |
| **Bash Execution**  | Use `!` prefix: `!git status`                                                                       | `!` prefix for bash            | ✅ Compatible syntax                                                                                                                                       |
| **File References** | Use `@` prefix: `@src/utils.js`                                                                     | `@` prefix for files           | ✅ Compatible syntax                                                                                                                                       |
| **Configuration**   | JSON in `opencode.json` OR markdown files                                                           | Primarily markdown files       | ⚠️ **Gotcha**: OpenCode allows JSON config for commands, which Claude doesn't support. Don't mix both for the same command                                 |

**Common Gotchas:**

1. **Migration Friction**: Unlike skills, commands in `~/.claude/commands/` won't be automatically discovered. **Workaround**: `cp -r ~/.claude/commands/* ~/.config/opencode/commands/`
1. **Command Not Appearing**: Ensure the `.md` file has valid frontmatter (if any) and the filename matches the intended command name
1. **Namespace Conflicts**: Project commands take precedence over user commands; use `/project:` or `/user:` prefixes to disambiguate

---

### 6. Agents/Subagents Support

#### **Interaction Model: Orchestrator ↔ Subagent**

OpenCode supports a hierarchical agent architecture using the **`task`** tool :

1. **Orchestrator (Primary Agent)**: Receives user request, analyzes intent, decides on delegation strategy
1. **Delegation**: Orchestrator calls `task` tool with:
   - `subagent_type`: Target agent name
   - `description`: Short task summary (3-5 words)
   - `prompt`: Detailed instructions for the subagent
1. **Subagent Execution**: OpenCode spins up a **new isolated session** (child session) with:
   - Fresh context (no access to parent conversation history unless included in prompt)
   - Subagent's specific system prompt, tools, and model
   - Independent tool permissions
1. **Result Return**: Subagent returns final text output → Parent receives it as the `task` tool return value
1. **Continuation**: Parent integrates results and continues or delegates further

**Key Architectural Points:**

- **Statelessness**: Each subagent invocation is stateless—you cannot send follow-up messages to a running subagent
- **Context Isolation**: Subagents don't see parent conversation; prompts must be self-contained
- **Tool Restrictions**: Subagents can have different tool access (e.g., read-only reviewers vs. full-access builders)

**Orchestrator Agent Template:**

```yaml
---
description: Central dispatch system for routing requests
mode: primary
model: anthropic/claude-haiku-4-20250514
temperature: 0.1
tools:
  read: true
  list: true
  glob: true
  grep: true
  task: true
  write: false
  edit: false
  bash: false
permission:
  edit: deny
  bash:
    "*": deny
---
You are The Orchestrator. You NEVER execute tasks yourself. You ALWAYS delegate to subagents.
## Agent Capability Map
| Agent | Capability | Triggers |
|-------|------------|----------|
| @dev | Implementation | "create", "build", "implement" |
| @review | Code review | "review", "audit", "check" |
| @explore | Codebase search | "find", "locate", "search" |
## Routing Rules
1. Explicit requests: Obey direct agent mentions
2. Research first: Chain @explore -> @dev for vague requests
3. Parallelize: Use multiple task calls for independent tasks
```

#### **Concurrency and Best Practices**

OpenCode supports **parallel subagent execution** by issuing multiple `task` tool calls in a single assistant message :

```markdown
### Delegation
[Tool call 1: task(subagent_type="code-review", prompt="Review auth.ts for security...")]
[Tool call 2: task(subagent_type="writer", prompt="Update API docs based on auth.ts...")]
```

**Best Practices for Concurrency:**

1. **Parallelize Independent Tasks**: Delegate to multiple subagents simultaneously when tasks don't depend on each other (e.g., security review + documentation update)

1. **Chain Dependent Tasks**: Use sequential delegation when later steps need earlier results:

   ```text
   @explore (find files) → @dev (implement changes)
   ```

1. **Keep Chains Short**: Maximum 3 agents in a chain unless explicitly requested

1. **Prompt Engineering for Subagents**:

   - Include all necessary context in the prompt (they can't see parent history)
   - Specify expected return format explicitly
   - State clearly whether they should write code or just research
1. **Avoid Git Conflicts**: Don't have multiple parallel subagents edit the same files simultaneously

1. **Resource Awareness**: Each subagent consumes API tokens; monitor costs with parallel execution

1. **Use Manifests for Long-Running Work**: For pipelines, use JSON manifests to track state across subagent invocations, enabling resume-from-failure

1. **Mode Configuration**: Set `mode: subagent` for workers, `mode: primary` for orchestrators. Use `hidden: true` for internal helpers

**Concurrency Limitations:**

- Currently, subagent delegation is synchronous from the orchestrator's perspective—it waits for results
- True "fire-and-forget" background execution is a requested feature (GitHub issue #5886) but not yet implemented
- Each subagent runs in its own session; there's no shared state between concurrent subagents

---

### 7. Key References

| Resource                               | Description                                     | Link                                                                                                                                         |
|----------------------------------------|-------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------|
| **OpenCode Official Documentation**    | Main docs portal                                | [https://opencode.ai/docs/](https://opencode.ai/docs/)                                                                                       |
| **Agent Skills Reference**             | SKILL.md specification and metadata             | [https://opencode.ai/docs/skills/](https://opencode.ai/docs/skills/)                                                                         |
| **Custom Commands Guide**              | Slash command creation and frontmatter          | [https://opencode.ai/docs/commands/](https://opencode.ai/docs/commands/)                                                                     |
| **Agents Configuration**               | Agent/subagent setup, modes, and permissions    | [https://opencode.ai/docs/agents/](https://opencode.ai/docs/agents/)                                                                         |
| **TUI Commands Reference**             | Built-in slash commands and keybindings         | [https://opencode.ai/docs/tui/](https://opencode.ai/docs/tui/)                                                                               |
| **Rules & AGENTS.md**                  | Project rules and Claude compatibility          | [https://opencode.ai/docs/rules/](https://opencode.ai/docs/rules/)                                                                           |
| **Orchestrator Guide**                 | Best practices for agent delegation and routing | [https://gist.github.com/gc-victor/1d3eeb46ddfda5257c08744972e0fc4c](https://gist.github.com/gc-victor/1d3eeb46ddfda5257c08744972e0fc4c)     |
| **GitHub Issue #6177**                 | Skill discovery path compatibility discussion   | [https://github.com/anomalyco/opencode/issues/6177](https://github.com/anomalyco/opencode/issues/6177)                                       |
| **GitHub Issue #6985**                 | Claude commands/ compatibility request          | [https://github.com/anomalyco/opencode/issues/6985](https://github.com/anomalyco/opencode/issues/6985)                                       |
| **Agent Skills Open Standard**         | Cross-platform skill specification              | [https://agentskills.io/](https://agentskills.io/)                                                                                           |
| **OpenCode vs Claude Code Comparison** | Feature and architecture differences            | [https://www.builder.io/blog/opencode-vs-claude-code](https://www.builder.io/blog/opencode-vs-claude-code)                                   |
| **OpenCode Deep Dive**                 | Internal architecture and tool system           | [https://cefboud.com/posts/coding-agents-internals-opencode-deepdive/](https://cefboud.com/posts/coding-agents-internals-opencode-deepdive/) |
