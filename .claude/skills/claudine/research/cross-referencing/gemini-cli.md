---
homepage: https://geminicli.com/
docs: https://geminicli.com/docs/
skills: https://geminicli.com/docs/cli/skills/
agent: https://geminicli.com/docs/core/subagents/
slash: https://geminicli.com/docs/cli/custom-commands/
scripts: N/A (colocated within skill directories; no dedicated scripts path)
---

# Gemini CLI: Skills, Slash Commands, Agents, Scripts

## Skills

Gemini CLI supports the Agent Skills open standard (a directory containing a `SKILL.md` file). Skills were introduced experimentally in **v0.23.0 (2026-01-07)** and enabled by default starting with **v0.25.0 (2026-01-20)**.

Skills are on-demand expertise modules that the model activates autonomously based on the user's request and the skill's description. Unlike GEMINI.md context files (which are loaded on every prompt), skills are activated only when relevant, conserving context tokens.

### Activation flow

1. **Discovery**: At startup, the CLI scans skill directories and injects skill names + descriptions into the system prompt.
2. **Recognition**: The model identifies a matching skill based on the task and calls the `activate_skill` tool.
3. **Consent & Loading**: The user approves via a confirmation prompt; the full SKILL.md and folder contents are loaded into context; the skill directory gains file-access permissions.

Skills remain active for the duration of the session once loaded.

### Directory discovery (user and repo scope)

Skills are discovered from three tiers, with higher-precedence locations overriding lower ones when names collide:

| Tier | Primary path | Alias path |
|------|-------------|------------|
| **Workspace** (highest) | `.gemini/skills/<name>/SKILL.md` | `.agents/skills/<name>/SKILL.md` |
| **User** | `~/.gemini/skills/<name>/SKILL.md` | `~/.agents/skills/<name>/SKILL.md` |
| **Extension** (lowest) | Bundled within installed extensions | -- |

Within each tier, `.agents/skills/` takes priority over `.gemini/skills/`.

Precedence: **Workspace > User > Extension**.

### Claude Code compatibility

Gemini CLI does **not** read `~/.claude/skills/` or `.claude/skills/` directly. However, the `.agents/skills/` alias path provides a cross-platform bridge: both Gemini CLI and Claude Code can discover skills placed in `.agents/skills/` when configured with symlinks. Community tools like [skill-porter](https://github.com/jduncan-rva/skill-porter) automate conversion between the two platforms.

The SKILL.md format itself (YAML frontmatter + Markdown body) is compatible between Claude Code and Gemini CLI. Both platforms read `name` and `description` from frontmatter.

### Skill directory structure

Minimal:
```
<skill-name>/SKILL.md
```

Recommended layout:
```
<skill-name>/
├── SKILL.md        # Required: entry point with YAML frontmatter
├── scripts/        # Optional: executable scripts
├── references/     # Optional: supporting documentation
└── assets/         # Optional: templates, images, data files
```

### Skill metadata (frontmatter)

`SKILL.md` begins with YAML frontmatter, then Markdown body content.

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Unique identifier; should match directory name; slug format (lowercase, hyphens) |
| `description` | Yes | Single-line string describing what the skill does and when to use it; this is the primary trigger mechanism |

Gemini CLI **only reads `name` and `description`** from frontmatter. Do not include other fields; while they will not cause errors, they are ignored. This differs from some other CLIs (e.g., Kimi Code) that support `license`, `compatibility`, and `metadata` fields.

Example:
```yaml
---
name: code-reviewer
description: "Review code for quality, security, and best practices. Use when asked to review, audit, or check code."
---
## What I do
- Analyze code for common anti-patterns
- Check for security vulnerabilities
- Suggest improvements with explanations
```

### Management commands

- `/skills list` -- show all discovered skills with activation status
- `/skills enable <name>` / `/skills disable <name>` -- toggle individual skills
- `/skills reload` -- re-scan skill directories

### Built-in skills

As of v0.26.0+, Gemini CLI ships with built-in skills:
- **skill-creator**: generates new skill directories with proper SKILL.md frontmatter
- **pr-creator**: assists with pull request creation
- **codebase_investigator**: analyzes codebases (also functions as a built-in sub-agent)

### Key differences from Claude Code

| Aspect | Gemini CLI | Claude Code |
|--------|-----------|-------------|
| **Activation** | Model calls `activate_skill` tool; user confirms | Model reads SKILL.md, selectively follows links |
| **Context strategy** | Loads entire skill folder contents on activation | Traverses tree progressively, reading sub-documents as needed |
| **Context window** | 1M+ tokens; encourages loading full skill content | Smaller context; encourages selective traversal |
| **Discovery paths** | `.gemini/skills/` + `.agents/skills/` | `.claude/skills/` only |
| **Frontmatter fields** | Only `name` and `description` | `description` (primary); `name` optional |
| **User consent** | Required before each skill activation | Automatic (no consent prompt) |

### Gotchas

1. **"Lazy reader" effect**: If you reference linked files from SKILL.md (e.g., `Details`) without the full skill folder being loaded, the model may hallucinate details. Gemini's activation flow loads the entire directory, so ensure all referenced files are within the skill folder.

2. **Description is everything**: Unlike Claude Code where the body of SKILL.md drives selection, Gemini CLI relies heavily on the `description` frontmatter field for deciding when to activate a skill. Write thorough, trigger-phrase-rich descriptions.

3. **No `.claude/skills/` fallback**: Gemini CLI does not scan Claude Code directories. Use `.agents/skills/` symlinks or the extension system to bridge.

---

## Slash Commands

Gemini CLI supports both built-in slash commands (35+) and user-defined custom commands. Custom commands use **TOML format** (`.toml` files), not Markdown.

### Built-in slash commands (selection)

| Command | Alias | Description |
|---------|-------|-------------|
| `/help` | `/?` | Show help and available commands |
| `/quit` | `/exit` | Exit the CLI |
| `/clear` | -- | Clear terminal display |
| `/compress` | -- | Summarize chat context to save tokens |
| `/chat save <tag>` | -- | Checkpoint conversation |
| `/chat resume <tag>` | -- | Restore saved conversation |
| `/resume` | -- | Interactive session browser |
| `/rewind` | -- | Navigate backward through conversation (also Esc x2) |
| `/model` | -- | Choose Gemini model |
| `/settings` | -- | Open settings editor |
| `/memory` | -- | Manage GEMINI.md context files |
| `/skills` | -- | Manage agent skills (list/enable/disable/reload) |
| `/tools` | -- | List available tools |
| `/mcp` | -- | MCP server management |
| `/extensions` | -- | List active extensions |
| `/hooks` | -- | Manage lifecycle hooks |
| `/stats` | -- | Token usage and session statistics |
| `/copy` | -- | Copy last output to clipboard |
| `/init` | -- | Generate tailored GEMINI.md file |
| `/commands reload` | -- | Reload custom commands from disk |
| `/bug` | -- | File a GitHub issue |
| `/vim` | -- | Toggle vim mode |
| `/docs` | -- | Open documentation in browser |

### Custom commands

Custom commands are TOML files placed in:
- **User scope**: `~/.gemini/commands/*.toml`
- **Project scope**: `<project-root>/.gemini/commands/*.toml`

Project commands override user commands when names collide.

#### File format

```toml
description = "Run tests with coverage"
prompt = """
Run the full test suite with coverage. Focus on failing tests and suggest fixes.
Arguments: {{args}}
"""
```

| Field | Required | Description |
|-------|----------|-------------|
| `prompt` | Yes | The prompt text sent to the model; may be multi-line |
| `description` | No | One-line summary shown in `/help`; auto-generated from filename if omitted |

#### Argument handling

- **`{{args}}`**: Replaced with user-provided arguments; automatically shell-escaped inside `!{...}` blocks.
- **No `{{args}}`**: If no placeholder, user arguments are appended to the prompt separated by two newlines.
- **`!{shell command}`**: Executes a shell command and injects its stdout; requires user confirmation.
- **`@{path}`**: Injects file/directory content; processed before shell commands and argument substitution; supports multimodal content (images, PDFs, audio, video).

#### Subdirectory namespacing

Subdirectories create namespaced commands using colon separators:
- `commands/test.toml` -> `/test`
- `commands/git/commit.toml` -> `/git:commit`

### Claude Code compatibility

Gemini CLI does **not** read `.claude/commands/` directories. Claude Code uses Markdown files for custom commands; Gemini CLI uses TOML files. There is no automatic migration path.

### Key differences from Claude Code

| Aspect | Gemini CLI | Claude Code |
|--------|-----------|-------------|
| **File format** | TOML (`.toml`) | Markdown (`.md`) |
| **Directory** | `.gemini/commands/` | `.claude/commands/` |
| **Arguments** | `{{args}}` placeholder | `$ARGUMENTS` placeholder |
| **Shell execution** | `!{command}` syntax in prompt | Not available in command files |
| **File injection** | `@{path}` syntax in prompt | Not available in command files |
| **Hot reload** | `/commands reload` | Requires restart |

### Gotchas

1. **TOML, not Markdown**: A common migration mistake. Claude Code `.md` command files will not work in Gemini CLI; they must be rewritten as `.toml` with `prompt = """..."""` syntax.

2. **Argument escaping**: Inside `!{...}` blocks, `{{args}}` is automatically shell-escaped. Outside those blocks, it is injected raw. Misplacing `{{args}}` can lead to prompt injection or shell escaping issues.

3. **No cross-tool fallback**: Unlike skills (which have `.agents/` aliases), custom commands have no cross-platform discovery path.

---

## Agents / Subagents

Gemini CLI supports sub-agents as an **experimental** feature. Sub-agents are specialized agents that operate within the main session, each with their own system prompt, tool access, and independent context window.

Gemini CLI uses the term **"sub-agent"** (or "subagent") for this concept; Claude Code calls the equivalent mechanism "Task tool" delegation.

### Enabling sub-agents

Sub-agents require explicit opt-in via `settings.json`:
```json
{
  "experimental": {
    "enableAgents": true
  }
}
```

### Directory structure

Sub-agent definitions are Markdown files (`.md`) with YAML frontmatter:
- **Project scope**: `.gemini/agents/*.md`
- **User scope**: `~/.gemini/agents/*.md`

The filename becomes the agent identifier.

### Frontmatter properties

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Unique identifier; slug format (lowercase, hyphens, underscores) |
| `description` | Yes | Short explanation visible to the orchestrator for routing decisions |
| `kind` | No | `"local"` (default) or `"remote"` (for A2A protocol agents) |
| `tools` | No | Array of tool names the agent can access; omit for default tool access |
| `model` | No | Model override (e.g., `gemini-2.5-pro`); defaults to session model |
| `temperature` | No | Sampling temperature (0.0 - 2.0) |
| `max_turns` | No | Conversation turn limit; defaults to 15 |
| `timeout_mins` | No | Execution time limit in minutes; defaults to 5 |

The Markdown body (after frontmatter) becomes the agent's system prompt.

Example:
```yaml
---
name: security-reviewer
description: "Reviews code for security vulnerabilities and suggests fixes"
tools:
  - read_file
  - list_directory
  - grep
model: gemini-2.5-pro
temperature: 0.1
max_turns: 10
---
You are a security expert. Review code for:
- SQL injection, XSS, CSRF
- Authentication and authorization flaws
- Secrets in source code
Report findings in a structured format. Do NOT modify files.
```

### Interaction model

1. **Registration**: Sub-agents are exposed to the main agent as callable tools (tool name = agent name).
2. **Routing**: The orchestrator reads agent descriptions to decide which sub-agent handles which task.
3. **Delegation**: The main agent invokes the sub-agent tool with a task description and prompt.
4. **Execution**: The sub-agent runs in an **isolated context** with its own conversation loop, system prompt, and tool access.
5. **Return**: The sub-agent's final output is returned to the orchestrator as the tool result.

Key properties:
- **Stateless**: Each invocation is independent; no follow-up messages to a running sub-agent.
- **Context isolation**: Sub-agents cannot see parent conversation history; prompts must be self-contained.
- **YOLO mode**: Sub-agents execute tools **without individual user confirmation**. Use restricted `tools` arrays for safety.

### Built-in sub-agents

- **codebase_investigator**: Explores the workspace, analyzes dependencies, and resolves relevant information. Configurable via `settings.json` with `maxNumTurns` and `model` options.
- **cli_help**: Provides expertise on Gemini CLI commands, configuration, and documentation.
- **generalist_agent**: Routes tasks to appropriate specialized sub-agents.

### Remote sub-agents (A2A protocol)

Gemini CLI supports the Agent-to-Agent (A2A) protocol for delegating to remote agents:

```yaml
---
kind: remote
name: my-remote-agent
agent_card_url: https://example.com/agent-card
---
```

Management commands: `/agents list`, `/agents refresh`, `/agents enable <name>`, `/agents disable <name>`.

### Key differences from Claude Code

| Aspect | Gemini CLI | Claude Code |
|--------|-----------|-------------|
| **Vernacular** | "Sub-agent" | "Task tool" / "Sub-agent" |
| **Definition format** | Markdown + YAML frontmatter in `.gemini/agents/` | Markdown in `.claude/agents/` |
| **Feature status** | Experimental (requires `enableAgents`) | Stable (built-in Task tool) |
| **Tool restrictions** | Per-agent `tools` array in frontmatter | Per-agent tool control |
| **User confirmation** | YOLO mode (no per-tool confirmation) | Inherits parent session permissions |
| **Remote agents** | A2A protocol support | Not supported |
| **Model override** | Per-agent `model` field | Per-agent model selection |
| **Turn/timeout limits** | `max_turns` (default 15), `timeout_mins` (default 5) | No built-in limits |

### Gotchas

1. **YOLO mode risk**: Sub-agents execute tools without confirmation. Always restrict the `tools` array for agents with access to destructive operations (e.g., `run_shell_command`, `write_file`).

2. **Context isolation**: Sub-agents see nothing from the parent conversation. Include all necessary context in the delegation prompt; do not assume the sub-agent knows what came before.

3. **Experimental instability**: The feature requires `enableAgents` flag and may have stability issues (e.g., hanging on agent creation has been reported in GitHub issue #18064).

---

## Scripts

Gemini CLI does **not** have a dedicated scripts directory convention (no `.gemini/scripts/` equivalent).

Scripts and executables are expected to be **colocated within skill directories**, following the recommended skill structure:
```
<skill-name>/
├── SKILL.md
└── scripts/
    ├── verify.sh
    └── generate.py
```

The SKILL.md can reference these scripts with relative paths, and the model will suggest executing them via the shell tool when appropriate. Scripts are not auto-executed; user confirmation is required.

For standalone scripts not tied to a skill, Gemini CLI relies on the standard shell tool (`run_shell_command`) and standard project conventions (e.g., `scripts/` at the project root).

### Extensions as script containers

Extensions can bundle scripts alongside MCP servers, commands, and skills. An extension's `gemini-extension.json` manifest provides structured access to bundled executables via MCP tool registration.

---

## Sources

- [Gemini CLI GitHub Repository](https://github.com/google-gemini/gemini-cli)
- [Gemini CLI Documentation](https://geminicli.com/docs/)
- [Agent Skills](https://geminicli.com/docs/cli/skills/)
- [Creating Agent Skills](https://geminicli.com/docs/cli/creating-skills/)
- [Custom Commands](https://geminicli.com/docs/cli/custom-commands/)
- [CLI Commands Reference](https://geminicli.com/docs/cli/commands/)
- [Sub-agents (experimental)](https://geminicli.com/docs/core/subagents/)
- [Remote Subagents (experimental)](https://geminicli.com/docs/core/remote-agents/)
- [Extensions Overview](https://geminicli.com/docs/extensions/)
- [Writing Extensions](https://geminicli.com/docs/extensions/writing-extensions/)
- [Context Files (GEMINI.md)](https://geminicli.com/docs/cli/gemini-md/)
- [Gemini CLI Configuration](https://geminicli.com/docs/get-started/configuration/)
- [Gemini CLI Changelog](https://geminicli.com/docs/changelogs/)
- [Hooks Overview](https://geminicli.com/docs/hooks/)
- [v0.23.0 Weekly Update (Skills Preview)](https://github.com/google-gemini/gemini-cli/discussions/16084)
- [v0.26.0 Weekly Update (Skills + Hooks)](https://github.com/google-gemini/gemini-cli/discussions/17812)
- [skill-porter (Cross-platform Skill Converter)](https://github.com/jduncan-rva/skill-porter)
- [Gemini CLI Skillz MCP Extension](https://github.com/intellectronica/gemini-cli-skillz)
