---
homepage: https://roocode.com/
docs: https://docs.roocode.com/
skills: https://docs.roocode.com/features/skills
agent: https://docs.roocode.com/features/boomerang-tasks
slash: https://docs.roocode.com/features/slash-commands
scripts: N/A (custom tools in `.roo/tools/`; scripts also colocated within skill directories)
---

# Roo Code: Skills, Slash Commands, Agents, Scripts

Roo Code is an open-source (Apache 2.0), AI-powered coding assistant that runs primarily as a VS Code extension. It supports multiple LLM providers (Anthropic, OpenAI, local models, etc.) and is built around a **modes** architecture where specialized personas with distinct tool permissions handle different tasks. Configuration lives under `.roo/` directories at both project and user scope.

Repository: https://github.com/RooCodeInc/Roo-Code

---

## Skills

Roo Code supports the Agent Skills open standard (a directory containing a `SKILL.md` file). Skills were introduced in **v3.38.0 (2025-12-27)**.

Skills are on-demand instruction packages that Roo loads when a user's request matches the skill's description. Unlike custom instructions (`.roo/rules/`) which are always appended to the system prompt, skills activate only when relevant, keeping the base prompt lean.

### Activation flow (progressive disclosure)

1. **Discovery**: At startup, Roo indexes `SKILL.md` files and extracts `name` and `description` from frontmatter. File watchers detect changes during development.
2. **Matching**: When a user request aligns with a skill's description, the full `SKILL.md` instructions are loaded via the `read_file` tool (or the built-in `skill` tool).
3. **Resource access**: Referenced bundled files (scripts, templates, data) load on-demand during execution.

Skills do not require user consent to activate; Roo decides autonomously based on the description match.

### Directory discovery (user and repo scope)

Skills are discovered from two scopes. Within each scope, mode-specific directories take priority over generic ones.

| Scope | Path |
|-------|------|
| **Project generic** | `<project-root>/.roo/skills/<name>/SKILL.md` |
| **Project mode-specific** | `<project-root>/.roo/skills-<modeSlug>/<name>/SKILL.md` |
| **User generic** | `~/.roo/skills/<name>/SKILL.md` |
| **User mode-specific** | `~/.roo/skills-<modeSlug>/<name>/SKILL.md` |

Override priority (highest to lowest):
1. Project mode-specific (`.roo/skills-code/my-skill/`)
2. Project generic (`.roo/skills/my-skill/`)
3. Global mode-specific (`~/.roo/skills-code/my-skill/`)
4. Global generic (`~/.roo/skills/my-skill/`)

Symbolic links are fully supported (resolved up to 5 levels deep). Symlinked directory names become the skill identifier.

### Claude Code compatibility

Roo Code does **not** read `~/.claude/skills/` or `.claude/skills/` directories. Skills must be placed in `.roo/skills/` paths. To share skills between Claude Code and Roo Code, use symlinks:

```bash
# Symlink a Claude Code skill into the Roo Code user skills directory
ln -s ~/.claude/skills/my-skill ~/.roo/skills/my-skill
```

There is no `.agents/skills/` alias path (unlike Gemini CLI and Codex). Cross-platform bridging requires explicit symlinks or community tooling like [skillport](https://github.com/gotalab/skillport).

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
| `name` | Yes | Must exactly match directory name; 1-64 lowercase alphanumeric characters and hyphens (no leading/trailing/consecutive hyphens) |
| `description` | Yes | Specific purpose statement; 1-1024 characters (trimmed); this is the primary trigger mechanism |

No other frontmatter fields are documented as supported. The Agent Skills standard defines additional optional fields (`license`, `compatibility`, `metadata`) but Roo Code only reads `name` and `description`.

Example:
```yaml
---
name: pdf-processing
description: Extract text and tables from PDF files using Python libraries. Use when asked to parse, extract, or analyze PDF content.
---
## Instructions
- Use `pdfplumber` for table extraction
- Use `PyPDF2` for text extraction
- Output extracted data as structured JSON
```

### Mode-specific skills

Skills placed in `skills-{modeSlug}/` directories activate only when Roo is operating in the matching mode. For example, a skill in `.roo/skills-code/` is available in Code mode but not in Architect or Ask modes.

### Key differences from Claude Code

| Aspect | Roo Code | Claude Code |
|--------|----------|-------------|
| **Platform** | VS Code extension (GUI) | Terminal CLI |
| **Discovery paths** | `.roo/skills/` + `.roo/skills-{mode}/` | `.claude/skills/` |
| **Activation** | Model loads via `read_file` or `skill` tool | Model reads SKILL.md, selectively follows links |
| **Mode targeting** | Skills scoped to specific modes via directory name | No mode concept; skills are universal |
| **Required frontmatter** | `name` + `description` | `description` (primary); `name` optional |
| **User consent** | Automatic (no consent prompt) | Automatic (no consent prompt) |
| **File watchers** | Live change detection during session | Requires restart for changes |

### Gotchas

1. **Name must match directory**: Unlike some platforms, Roo Code enforces that the `name` field in frontmatter exactly matches the directory name. Mismatches cause silent discovery failures.

2. **Mode-specific scoping**: If you place a skill in `.roo/skills-code/` but try to use it in Architect mode, it will not be discovered. Use the generic `.roo/skills/` path for universally available skills.

3. **No `.agents/skills/` fallback**: Roo Code does not scan `.agents/skills/` or `.claude/skills/`. Cross-platform sharing requires symlinks.

---

## Slash Commands

Roo Code supports both built-in slash commands and user-defined custom commands. Custom commands are **Markdown files** with optional YAML frontmatter, sharing the same file format as Claude Code commands.

### Built-in slash commands

| Command | Description |
|---------|-------------|
| `/init` | Multi-phase codebase analysis; generates tailored AGENTS.md configuration |
| `/code` | Switch to Code mode |
| `/architect` | Switch to Architect mode |
| `/ask` | Switch to Ask mode |
| `/debug` | Switch to Debug mode |
| `/<custom-slug>` | Switch to any custom mode (e.g., `/docs-writer`) |
| `/help` | Show available commands |

All custom modes automatically register as slash commands using their slug (e.g., a mode with slug `reviewer` becomes `/reviewer`).

### Custom commands

Custom commands are Markdown files placed in:
- **Project scope**: `<project-root>/.roo/commands/*.md`
- **User scope**: `~/.roo/commands/*.md`

Project commands override global commands when names collide.

#### File format

The filename (without `.md` extension) becomes the command name. Names are normalized: converted to lowercase, spaces replaced with dashes, special characters removed.

```markdown
---
description: Generate a new REST API endpoint with best practices
argument-hint: <endpoint-name> <http-method>
mode: code
---
Create a new REST API endpoint named `{{endpoint-name}}` using the `{{http-method}}` HTTP method.

Follow the project's existing patterns for:
- Route definition
- Request validation
- Error handling
- Response formatting
```

#### Frontmatter properties

| Field | Required | Description |
|-------|----------|-------------|
| `description` | No | One-line summary shown in the command menu; auto-generated from filename if omitted |
| `argument-hint` | No | Placeholder guidance shown as gray hint text next to the command (e.g., `<file-path>`) |
| `mode` | No | Mode slug to activate before executing the command (added in v3.38.0) |

#### Subdirectory support

The documentation mentions grouping related commands in subdirectories as a best practice, but specific namespacing behavior (e.g., whether `commands/git/commit.md` becomes `/git:commit` or `/git-commit`) is not documented in detail.

#### Programmatic execution (experimental)

The `run_slash_command` tool allows the AI to execute slash commands programmatically without user trigger. This is an experimental feature requiring explicit enablement in settings. Parameters:
- **command** (required): Command name without leading slash
- **args** (optional): Additional context or arguments

The tool searches three levels: project commands, global commands, then built-in commands.

### Claude Code compatibility

Roo Code does **not** read `.claude/commands/` or `~/.claude/commands/` directories. However, since both platforms use Markdown files with YAML frontmatter, commands are format-compatible. The main differences are directory location and placeholder syntax.

### Key differences from Claude Code

| Aspect | Roo Code | Claude Code |
|--------|----------|-------------|
| **Directory** | `.roo/commands/` | `.claude/commands/` |
| **File format** | Markdown (`.md`) | Markdown (`.md`) |
| **Arguments** | `argument-hint` for display guidance | `$ARGUMENTS` placeholder for substitution |
| **Mode switching** | `mode` frontmatter field auto-switches mode | Not applicable |
| **Programmatic exec** | `run_slash_command` tool (experimental) | Not available |
| **Hot reload** | File watchers detect changes | Requires restart |

### Gotchas

1. **Argument hints are display-only**: Unlike Claude Code's `$ARGUMENTS` substitution, Roo Code's `argument-hint` provides visual guidance in the menu but does not auto-insert into the command body.

2. **Experimental flag**: The `run_slash_command` tool requires explicit enablement. Without it, the AI cannot programmatically trigger custom commands.

3. **No cross-tool fallback**: Roo Code does not scan Claude Code or Gemini CLI command directories. Manual migration is required.

---

## Agents / Subagents

Roo Code implements agent orchestration through its **modes** system and the **Boomerang Tasks** (subtask delegation) pattern. There is no separate "agents" or "subagents" directory; instead, modes serve as the agent abstraction, and the Orchestrator mode coordinates multi-step workflows by spawning subtasks in different modes.

Roo Code uses the terms **"modes"** and **"Boomerang Tasks"** for this concept. The Orchestrator was originally a community-created custom mode called "Boomerang Mode" before becoming a built-in mode in **v3.14.3 (2025-04-25)**.

### Built-in modes (agents)

| Mode | Slug | Tool Access | Purpose |
|------|------|-------------|---------|
| Code | `code` | Full (read, edit, browser, command, mcp) | Everyday coding, file operations, implementation |
| Architect | `architect` | Read, browser, MCP, markdown-only edit | System design, architecture planning |
| Ask | `ask` | Read, browser, MCP | Learning, explanations, documentation queries |
| Debug | `debug` | Full | Systematic troubleshooting and diagnostics |
| Orchestrator | `orchestrator` | `new_task` only (no direct file/command access) | Workflow orchestration via subtask delegation |

### Custom modes (custom agents)

Custom modes are defined in YAML (preferred) or JSON:
- **Global**: `custom_modes.yaml` (or `custom_modes.json`) in user settings directory
- **Project**: `.roomodes` file in workspace root

Project-level modes completely override global modes with the same slug (no property merging).

#### Mode configuration properties

| Property | Required | Description |
|----------|----------|-------------|
| `slug` | Yes | Unique identifier; lowercase alphanumeric with hyphens (`/^[a-zA-Z0-9-]+$/`) |
| `name` | Yes | Display name shown in UI (can include emojis and spaces) |
| `roleDefinition` | Yes | Detailed role description placed at the beginning of the system prompt |
| `description` | No | Short summary displayed below mode name in selector UI |
| `whenToUse` | No | Guidance for the Orchestrator on when to select this mode for delegation |
| `customInstructions` | No | Additional behavioral guidelines added to system prompt |
| `groups` | Yes | Array of allowed tool categories with optional file restrictions |

#### Tool groups

Available groups: `read`, `edit`, `browser`, `command`, `mcp`

File restrictions can be applied per-group:
```yaml
groups:
  - read
  - - edit
    - fileRegex: "\\.md$"
      description: "Markdown files only"
  - command
```

#### Example custom mode

```yaml
# custom_modes.yaml
customModes:
  - slug: docs-writer
    name: "Documentation Writer"
    roleDefinition: "You are a technical writer specializing in clear, accurate documentation."
    whenToUse: "Use this mode for writing and editing documentation, README files, and API references."
    customInstructions: "Focus on clarity. Use active voice. Include code examples."
    groups:
      - read
      - - edit
        - fileRegex: "\\.(md|mdx|txt)$"
          description: "Documentation files only"
      - browser
```

#### Mode-specific instructions (rules)

Each mode can have additional rules loaded from the filesystem:
- **Directory method** (preferred): `.roo/rules-{slug}/` (files read recursively, alphabetically)
- **File fallback**: `.roorules-{slug}` (used only if directory is empty or missing)
- **Legacy fallback**: `.clinerules-{slug}` (backward compatibility, not recommended)

### Orchestrator mode and Boomerang Tasks

The Orchestrator mode is Roo Code's answer to sub-agent delegation. It uses the `new_task` tool to spawn subtasks in specialized modes.

#### How delegation works

1. **Task analysis**: The Orchestrator breaks a complex request into discrete subtasks.
2. **Spawning**: For each subtask, it calls `new_task` with a target mode and instructions.
3. **Isolation**: Each subtask runs in its own conversation context with separate history.
4. **Completion**: The subtask calls `attempt_completion` with a summary via the `result` parameter.
5. **Return**: Only the completion summary "boomerangs" back to the Orchestrator; detailed execution steps remain isolated.
6. **Continuation**: The Orchestrator decides the next step based on the returned summary.

#### `new_task` tool parameters

| Parameter | Required | Description |
|-----------|----------|-------------|
| `mode` | Yes | Slug of the mode for the subtask (e.g., `code`, `architect`, `debug`) |
| `message` | Yes | Comprehensive instructions including all necessary context |
| `todos` | No | Initial todo list in markdown checklist format |

#### Context isolation

- Subtasks do **not** inherit parent conversation history
- Information flows **down** via the `message` parameter and **up** via `attempt_completion`'s `result` parameter
- The parent task is paused during subtask execution
- Token usage and task history are tracked per-subtask

#### Approval behavior

By default, users must approve both subtask creation and completion. This can be automated via Roo Code's auto-approve settings.

### Key differences from Claude Code

| Aspect | Roo Code | Claude Code |
|--------|----------|-------------|
| **Vernacular** | "Modes" + "Boomerang Tasks" | "Task tool" / "Sub-agents" |
| **Agent definition** | YAML in `custom_modes.yaml` or `.roomodes` | Markdown in `.claude/agents/` |
| **Orchestration** | Built-in Orchestrator mode with `new_task` tool | Manual Task tool calls in agent instructions |
| **Tool restrictions** | Per-mode `groups` array with file regex patterns | Per-agent tool control |
| **Mode-specific instructions** | `.roo/rules-{slug}/` directories | No per-agent instruction directories |
| **Context isolation** | Full isolation; summary-only return | Task tool provides similar isolation |
| **Approval** | Configurable auto-approve for subtask creation | Inherits parent session permissions |
| **Nested delegation** | Theoretically possible (subtask in Orchestrator mode) | Supported via recursive Task tool calls |
| **Sticky models** | Each mode remembers its last-used LLM model | Single model per session |
| **Feature status** | Stable (built-in since v3.14.3) | Stable (built-in Task tool) |

### Gotchas

1. **Orchestrator has no tools**: The Orchestrator mode intentionally cannot read files, write files, or run commands. It can only delegate via `new_task`. If you need an orchestrator that can also read files, create a custom mode with both `new_task` access and `read` group permissions.

2. **No `.claude/agents/` equivalent**: Roo Code does not have a directory of agent definition files that get loaded as callable sub-agents. Instead, agents are defined as modes in YAML config files. This is a fundamentally different architecture.

3. **Context isolation is strict**: Subtasks see nothing from the parent. All required context must be passed in the `message` parameter. Do not assume a subtask knows what the parent was working on.

4. **Project vs global precedence**: When `.roomodes` and global `custom_modes.yaml` both define a mode with the same slug, the project version completely replaces the global one (no merging of individual properties).

---

## Scripts

Roo Code does not have a dedicated scripts directory analogous to a `.roo/scripts/` path. Instead, it provides two mechanisms for executable automation:

### 1. Custom tools (`.roo/tools/`)

Custom tools are TypeScript or JavaScript files that extend Roo Code's tool catalog:

- **Project scope**: `.roo/tools/*.ts` or `.roo/tools/*.js`
- **Global scope**: `~/.roo/tools/*.ts` or `~/.roo/tools/*.js`

Project tools override global tools with the same name.

Custom tools are dynamically loaded and transpiled with esbuild. They use the `defineCustomTool()` API from `@roo-code/types` with Zod schema validation for parameters:

```typescript
import { defineCustomTool, parametersSchema as z } from "@roo-code/types";

export default defineCustomTool({
  name: "run-migrations",
  description: "Execute database migrations for the current project",
  parameters: z.object({
    direction: z.enum(["up", "down"]).describe("Migration direction"),
    count: z.number().optional().describe("Number of migrations to run"),
  }),
  async execute({ direction, count }) {
    // Tool logic here
    return `Ran ${count ?? "all"} migrations ${direction}`;
  },
});
```

Key properties:
- Custom tools are **automatically approved** when enabled (no per-invocation confirmation)
- NPM dependencies can be installed in the tool directory and imported normally
- `.env` and `.env.*` files from the tool directory are copied to the cache folder for runtime access
- Tools appear alongside built-in tools like `read_file` and `execute_command`

### 2. Scripts within skill directories

Scripts can be colocated within skill directories under a `scripts/` subdirectory:

```
my-skill/
├── SKILL.md
└── scripts/
    ├── validate.sh
    └── generate.py
```

The `SKILL.md` references these scripts, and Roo executes them via the `execute_command` tool when instructed by the skill's content. Scripts are not auto-executed; user approval is required (unless auto-approve is configured).

### MCP servers as script containers

MCP (Model Context Protocol) servers provide another mechanism for external tool integration:
- **Global**: `mcp_settings.json` in VS Code settings
- **Project**: `.roo/mcp.json` in workspace root

MCP servers support STDIO, Streamable HTTP, and SSE transports.

### Custom instructions (not scripts, but related)

Roo Code's instruction system provides non-executable automation:

| Method | Location | Scope |
|--------|----------|-------|
| Rules directory | `.roo/rules/` | Project-wide, always loaded |
| Mode-specific rules | `.roo/rules-{slug}/` | Per-mode, always loaded |
| AGENTS.md | Workspace root | Project-wide (enabled by default via `roo-cline.useAgentRules`) |
| `.roorules` | Workspace root | Legacy fallback |
| `.clinerules` | Workspace root | Legacy fallback (from Cline heritage) |

---

## Sources

- [Roo Code Homepage](https://roocode.com/)
- [Roo Code Documentation](https://docs.roocode.com/)
- [Roo Code GitHub Repository](https://github.com/RooCodeInc/Roo-Code)
- [Skills Documentation](https://docs.roocode.com/features/skills)
- [Slash Commands Documentation](https://docs.roocode.com/features/slash-commands)
- [run_slash_command Tool](https://docs.roocode.com/advanced-usage/available-tools/run-slash-command)
- [Custom Modes Documentation](https://docs.roocode.com/features/custom-modes)
- [Using Modes](https://docs.roocode.com/basic-usage/using-modes)
- [Boomerang Tasks (Subtask Orchestration)](https://docs.roocode.com/features/boomerang-tasks)
- [Custom Instructions](https://docs.roocode.com/features/custom-instructions)
- [Custom Tools (Experimental)](https://docs.roocode.com/features/experimental/custom-tools)
- [new_task Tool](https://docs.roocode.com/advanced-usage/available-tools/new-task)
- [attempt_completion Tool](https://docs.roocode.com/advanced-usage/available-tools/attempt-completion)
- [Tool Use Overview](https://docs.roocode.com/advanced-usage/available-tools/tool-use-overview)
- [MCP in Roo Code](https://docs.roocode.com/features/mcp/using-mcp-in-roo)
- [v3.38.0 Release Notes (Skills Introduction)](https://docs.roocode.com/update-notes/v3.38.0)
- [v3.38 Combined Release Notes](https://docs.roocode.com/update-notes/v3.38)
- [v3.17.0 Release Notes (whenToUse Field)](https://docs.roocode.com/update-notes/v3.17.0)
- [v3.14.3 Release Notes (Orchestrator Built-in)](https://docs.roocode.com/update-notes/v3.14.3)
- [AGENTS.md Support Issue](https://github.com/RooCodeInc/Roo-Code/issues/5966)
- [AGENTS.md Discussion](https://github.com/RooCodeInc/Roo-Code/discussions/5967)
- [Global .roo/rules Directory Support Issue](https://github.com/RooCodeInc/Roo-Code/issues/4978)
- [CLI/Headless Execution Support Issue](https://github.com/RooCodeInc/Roo-Code/issues/3835)
- [Slash Commands Feature Request](https://github.com/RooCodeInc/Roo-Code/issues/4183)
- [Roo Code VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=RooVeterinaryInc.roo-cline)
- [skillport (Cross-platform Skill Tool)](https://github.com/gotalab/skillport)
- [Agent Skills Open Standard](https://agentskills.io/)
- [Roo Code on X (Skills Announcement)](https://x.com/roocode/status/2005064260850946508)
- [Roo Code on X (AGENTS.md Support)](https://x.com/roo_code/status/1950232276379291736)
- [DeepWiki: Custom Instructions and Rules](https://deepwiki.com/RooCodeInc/Roo-Code/9.4-custom-instructions-and-rules)
