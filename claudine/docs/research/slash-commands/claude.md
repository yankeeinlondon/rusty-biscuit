---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7
homepage: https://code.claude.com/
docs: https://code.claude.com/docs/en/overview
slash_docs: https://code.claude.com/docs/en/skills
support: first_class
locations:
  - os: macos
    scope: user
    path: ~/.claude/skills/<name>/SKILL.md
    notes: Personal skills; directory name becomes the / command. Also ~/.claude/commands/<name>.md for legacy custom commands. Loaded in every project.
  - os: macos
    scope: user
    path: ~/.claude/commands/<name>.md
    notes: Legacy custom command files. Still supported; skills take precedence if the same name exists in ~/.claude/skills/.
  - os: linux
    scope: user
    path: ~/.claude/skills/<name>/SKILL.md
    notes: Same behavior as macOS; ~/.claude resolves to $HOME/.claude.
  - os: linux
    scope: user
    path: ~/.claude/commands/<name>.md
    notes: Legacy custom command files.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\skills\\<name>\\SKILL.md"
    notes: Personal skills on Windows.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\commands\\<name>.md"
    notes: Legacy custom command files on Windows.
  - os: macos
    scope: repo
    path: .claude/skills/<name>/SKILL.md
    notes: Project skills. Discovered from the starting directory and every parent up to the repo root. Also from nested .claude/skills/ directories on demand. Require workspace trust.
  - os: macos
    scope: repo
    path: .claude/commands/<name>.md
    notes: Legacy project custom commands. Require workspace trust.
  - os: linux
    scope: repo
    path: .claude/skills/<name>/SKILL.md
    notes: Same as macOS.
  - os: linux
    scope: repo
    path: .claude/commands/<name>.md
    notes: Same as macOS.
  - os: windows
    scope: repo
    path: .claude\\skills\\<name>\\SKILL.md
    notes: Project skills on Windows.
  - os: windows
    scope: repo
    path: .claude\\commands\\<name>.md
    notes: Legacy project custom commands on Windows.
  - os: macos
    scope: system
    path: /Library/Application Support/ClaudeCode/
    notes: File-based managed settings and managed skills directory. Also delivered via MDM plist at com.anthropics.claudecode.
  - os: linux
    scope: system
    path: /etc/claude-code/
    notes: File-based managed settings and managed skills directory.
  - os: windows
    scope: system
    path: "C:\\Program Files\\ClaudeCode\\"
    notes: File-based managed settings and managed skills directory. Also delivered via HKLM\\SOFTWARE\\Policies\\ClaudeCode registry.
  - os: macos
    scope: extension
    path: <plugin>/skills/<name>/SKILL.md
    notes: Plugin skills. Namespaced as plugin-name:name. Plugin root SKILL.md uses frontmatter name field for the command.
format:
  file_names:
    - "SKILL.md"
    - "*.md"
  frontmatter: true
  required_fields: []
  optional_fields:
    - name
    - description
    - when_to_use
    - argument-hint
    - arguments
    - disable-model-invocation
    - user-invocable
    - allowed-tools
    - disallowed-tools
    - model
    - effort
    - context
    - agent
    - hooks
    - paths
    - shell
  argument_syntax: |
    $ARGUMENTS is replaced with the raw argument string after the command name.
    $ARGUMENTS[N] accesses the Nth 0-based positional argument; $N is shorthand for $ARGUMENTS[N].
    Named arguments declared in the frontmatter arguments: list substitute as $name.
    Multi-word arguments must be quoted with double quotes: /cmd "hello world" second.
    To type a literal $ before a digit or argument name, escape with a single backslash: \$1.00.
  body_format: markdown
  notes: |
    Skills are directories containing SKILL.md. Legacy custom commands are single .md files under .claude/commands/.
    The body supports dynamic context injection: a line starting with !`command` runs the shell command and replaces the line with its output before Claude sees the content.
    Multi-line shell injection uses a fenced code block opened with ```!.
    Shell injection is disabled by the disableSkillShellExecution setting in any scope; it does not affect bundled or managed skills.
command_model:
  invocation: |
    Type /name in an interactive session, e.g. /commit or /research:publish.
    Nested skills use a directory-qualified name such as apps/web:deploy.
    Plugin skills use plugin-name:name.
    Up to six user-invocable skills can be stacked at the start of one message (v2.1.199+), e.g. /code-review /fix-issue 123.
  namespacing: |
    Built-in slash commands and user-defined skills/commands share one namespace.
    If a skill and a legacy command share the same name, the skill wins.
    Enterprise/managed skills override personal skills, which override project skills.
    Nested skills with clashing names stay available under a qualified path:name.
    Plugin skills are namespaced and cannot collide with non-plugin names.
  arguments: |
    Everything after the command name is passed as one raw argument string to $ARGUMENTS.
    Individual arguments are available with $0, $1, etc. or $ARGUMENTS[0], $ARGUMENTS[1].
    Named arguments from arguments: [foo, bar] map as $foo to the first argument and $bar to the second.
    Double quotes group multi-word values into one argument.
    Default arguments are not supported.
  output_handling: |
    The rendered SKILL.md or command .md body is inserted into the conversation as a user prompt.
    Dynamic context commands (!`cmd` and ```! blocks) execute during rendering and are replaced by their stdout.
    If $ARGUMENTS is absent from the body, Claude Code appends ARGUMENTS: <input> to the end.
  disabled_mechanism: |
    Remove or rename the file/folder.
    Set disable-model-invocation: true to hide from Claude's automatic Skill tool use (still user-invokable).
    Set user-invocable: false to hide from the / menu (still auto-invokable by Claude).
    Use the skillOverrides setting with values on, name-only, user-invocable-only, or off.
    Pass --disable-slash-commands to disable all skills and commands for the session.
    Pass --bare or --safe-mode to skip auto-discovery of user/project skills and commands.
    Set disableBundledSkills: true or CLAUDE_CODE_DISABLE_BUNDLED_SKILLS=1 to remove only bundled skills/workflows.
  notes: |
    Project skills and commands require accepting a workspace trust dialog before their allowed-tools or other permissions take effect.
    Live change detection watches ~/.claude/skills/, project .claude/skills/, and --add-dir .claude/skills/ directories; edits apply within the session.
    Skills support model and effort overrides for the current turn only.
    A skill with context: fork runs in a subagent; agent: names a built-in or custom subagent type.
portability:
  portable: false
  non_portable_assets:
    - "$ARGUMENTS / $N / $ARGUMENTS[N] placeholders"
    - "Named $arg placeholders declared in arguments: frontmatter"
    - "!`command` and ```! dynamic shell injection"
    - "Frontmatter fields: allowed-tools, disallowed-tools, model, effort, context, agent, hooks, paths, shell"
    - "Nested path: namespace syntax and plugin: namespace syntax"
    - "${CLAUDE_SKILL_DIR}, ${CLAUDE_PROJECT_DIR}, ${CLAUDE_SESSION_ID}, ${CLAUDE_EFFORT} substitutions"
    - "Workspace trust gating and Claude-specific tool names"
  rewrite_needed: true
  notes: |
    The Markdown prose body is largely portable, but every execution-facing and namespacing construct must be rewritten for another provider.
    Placeholders differ across providers (some use {{args}}, some $ARGUMENTS, some positional $1).
    Tool permission models and namespace prefixes are provider-specific.
    Dynamic shell injection has no direct equivalent in most providers and should be expanded or removed.
cli_params:
  - flag: --disable-slash-commands
    description: Disable all skills and commands for the session.
    example: claude --disable-slash-commands
  - flag: --bare
    description: Skip auto-discovery of hooks, skills, plugins, MCP servers, auto memory, and CLAUDE.md. Sets CLAUDE_CODE_SIMPLE. User/project skills and commands do not load.
    example: claude --bare -p "query"
  - flag: --safe-mode
    description: Start in safe mode; CLAUDE.md, skills, plugins, hooks, MCP servers, custom commands and agents, output styles, workflows, custom themes, custom keybindings, status line and file-suggestion commands, LSP servers, and auto-memory do not load.
    example: claude --safe-mode
  - flag: --add-dir
    description: Add additional working directories. .claude/skills/ inside an added directory is loaded automatically; other .claude/ config such as commands is not loaded from added dirs.
    example: claude --add-dir ../apps ../lib
  - flag: --plugin-dir
    description: Load plugins from the specified directory, making their skills available namespaced by plugin name.
    example: claude --plugin-dir ./my-plugin
  - flag: --allowed-tools, --allowed-tools
    description: Tools that execute without prompting. Affects permission behavior while skills run, not discovery.
    example: claude --allowed-tools "Bash(git *)" Read
  - flag: --dangerously-skip-permissions
    description: Start in bypassPermissions mode, which also affects how skill tool calls are gated.
    example: claude --dangerously-skip-permissions
  - flag: --permission-mode
    description: Set the initial permission mode (ask, auto, bypassPermissions, plan).
    example: claude --permission-mode auto
env_vars:
  - name: CLAUDE_CODE_DISABLE_BUNDLED_SKILLS
    effect: Set to 1 to remove bundled skills/workflows from the session. Built-in slash commands stay typable but are hidden from the model. User/project/plugin skills and commands are unaffected.
  - name: CLAUDE_CODE_DISABLE_POLICY_SKILLS
    effect: Set to 1 to skip loading skills from the system-wide managed skills directory. Useful for container or CI sessions.
  - name: CLAUDE_CODE_SAFE_MODE
    effect: Set to 1 to start in safe mode; skips loading user/project skills, commands, plugins, hooks, MCP servers, etc.
  - name: CLAUDE_CODE_SIMPLE
    effect: Set to 1 by --bare. Runs with a minimal system prompt and skips auto-discovery of hooks, skills, plugins, MCP servers, auto memory, and CLAUDE.md.
  - name: CLAUDE_CODE_SYNC_SKILLS
    effect: Set to 1 to download enabled claude.ai skills into ~/.claude/skills/ before the first query in non-interactive (-p) mode, and resync every 10 minutes.
  - name: CLAUDE_CODE_SYNC_SKILLS_INSTALL_TIMEOUT_MS
    effect: Timeout for mid-session skills resync when CLAUDE_CODE_SYNC_SKILLS is set (default 30000).
  - name: CLAUDE_CODE_SYNC_SKILLS_WAIT_TIMEOUT_MS
    effect: Timeout for the first query to wait on initial skills sync (default 5000).
  - name: CLAUDE_CODE_USE_NATIVE_FILE_SEARCH
    effect: Set to 1 to discover custom commands, subagents, and output styles using Node.js file APIs instead of ripgrep.
  - name: SLASH_COMMAND_TOOL_CHAR_BUDGET
    effect: Legacy override for the character budget of skill metadata shown to the Skill tool.
changes: []
requires_claudine_update: false
reason: Research confirms Claude Code's first-class skill/command model; existing portability classification (non-portable with required rewrites) remains accurate.
---

# Claude Code Slash Commands and Skills

## Overview

Claude Code calls user-defined reusable commands **skills**. The legacy term **custom slash commands** still works: a file at `.claude/commands/deploy.md` and a directory at `.claude/skills/deploy/SKILL.md` both create `/deploy` and behave the same way. Anthropic now treats skills as the primary format because they support extra features such as supporting files, richer frontmatter, and optional automatic invocation.

Support is **first class**: users can define commands at user, project, plugin, and managed/enterprise scopes; invoke them by name with `/`; pass arguments; control whether Claude can invoke them automatically; and pre-approve tools. Built-in commands like `/help` and bundled skills like `/code-review` share the same `/` namespace.

## Locations

Claude Code discovers command resources from several scopes. On Windows, `~/.claude` resolves to `%USERPROFILE%\.claude`.

| OS | Scope | Path | Notes |
| :- | :---- | :--- | :---- |
| macOS / Linux | User | `~/.claude/skills/<name>/SKILL.md` | Personal skills available in every project. |
| macOS / Linux | User | `~/.claude/commands/<name>.md` | Legacy personal custom commands. |
| Windows | User | `%USERPROFILE%\.claude\skills\<name>\SKILL.md` | Personal skills. |
| Windows | User | `%USERPROFILE%\.claude\commands\<name>.md` | Legacy personal custom commands. |
| macOS / Linux / Windows | Repo | `.claude/skills/<name>/SKILL.md` | Project skills. Loaded from the start directory and every parent up to the repo root; also discovered on demand from nested `.claude/skills/` directories. |
| macOS / Linux / Windows | Repo | `.claude/commands/<name>.md` | Legacy project custom commands. |
| macOS | System / managed | `/Library/Application Support/ClaudeCode/` | File-based managed settings and managed skills. Also delivered via MDM plist `com.anthropics.claudecode`. |
| Linux / WSL | System / managed | `/etc/claude-code/` | File-based managed settings and managed skills. |
| Windows | System / managed | `C:\Program Files\ClaudeCode\` | File-based managed settings and managed skills. Also delivered via `HKLM\SOFTWARE\Policies\ClaudeCode` registry. |
| All | Extension | `<plugin>/skills/<name>/SKILL.md` | Plugin skills. A plugin root `SKILL.md` uses its frontmatter `name` for the command. |

### Local observations

On this machine, `~/.claude/commands/` exists and contains Markdown files such as `commit.md`, `research/integrate-new-research.md`, and `meta/create-command.md`. Each file has YAML frontmatter with `description`, `argument-hint`, and sometimes `allowed-tools`, `model`, and `name`. The repository also has project-level commands in `.claude/commands/` including `code-review.md` and `drift.md`. `~/.claude/skills/` contains skill directories (e.g. `axum/`, `clap/`, `darkmatter/`) each with a `SKILL.md` entry point.

## File Format

### Skills

A skill is a directory whose name becomes the command name. The required entry point is `SKILL.md`:

```text
my-skill/
├── SKILL.md
├── reference.md
├── examples.md
└── scripts/
    └── helper.sh
```

### Legacy custom commands

A single Markdown file under `.claude/commands/` or `~/.claude/commands/`:

```text
.claude/commands/
└── deploy.md
```

### Frontmatter

Both skills and legacy command files use YAML frontmatter between `---` markers. All fields are optional except that `description` is recommended.

| Field | Purpose | Example |
| :---- | :------ | :------ |
| `name` | Display name in listings. For skills, the directory sets the typed command name, not this field. | `name: Deploy Staging` |
| `description` | Shown in `/` menu and help; used by Claude to decide auto-invocation. | `description: Deploy to staging` |
| `when_to_use` | Extra trigger hints; appended to `description` (1,536 char cap). | `when_to_use: Use when CI is green` |
| `argument-hint` | Autocomplete hint for expected arguments. | `argument-hint: "[branch]"` |
| `arguments` | Named positional args as YAML list or space-separated string. | `arguments: [issue, branch]` |
| `disable-model-invocation` | If `true`, only the user can invoke it; Claude's Skill tool cannot. | `disable-model-invocation: true` |
| `user-invocable` | If `false`, hidden from the `/` menu; Claude can still auto-invoke. | `user-invocable: false` |
| `allowed-tools` | Tools pre-approved while the skill is active. | `allowed-tools: "Bash(git *)" Read` |
| `disallowed-tools` | Tools removed from the pool while the skill is active. | `disallowed-tools: AskUserQuestion` |
| `model` | Model override for the current turn. | `model: claude-sonnet-4-5-20250929` |
| `effort` | Effort override (`low`, `medium`, `high`, `xhigh`, `max`). | `effort: high` |
| `context` | `fork` to run in a subagent context. | `context: fork` |
| `agent` | Subagent type for `context: fork`. | `agent: Explore` |
| `hooks` | Skill-scoped lifecycle hooks. | See Hooks docs |
| `paths` | Glob patterns limiting auto-activation. | `paths: "src/**/*.rs"` |
| `shell` | Shell for `!` commands (`bash` or `powershell`). | `shell: bash` |

### Argument substitution

The body supports these substitutions:

| Token | Meaning |
| :---- | :------ |
| `$ARGUMENTS` | The entire raw argument string after the command name. |
| `$ARGUMENTS[N]` | The Nth positional argument, 0-based. |
| `$N` | Shorthand for `$ARGUMENTS[N]`. |
| `$name` | A named argument declared in `arguments:`. |
| `${CLAUDE_SESSION_ID}` | Current session ID. |
| `${CLAUDE_EFFORT}` | Current effort level. |
| `${CLAUDE_SKILL_DIR}` | Directory containing the skill's `SKILL.md`. |
| `${CLAUDE_PROJECT_DIR}` | Project root directory. |

Multi-word arguments are grouped with double quotes: `/migrate "search bar" React Vue` makes `$0` = `search bar`, `$1` = `React`, `$2` = `Vue`. Escape a literal dollar with a single backslash: `\$1.00`.

### Dynamic context injection

Lines starting with `` !`command` `` or fenced blocks opened with ` ```! ` execute as shell commands during rendering. Their stdout replaces the placeholder in the prompt before Claude sees it. This is preprocessing, not a tool call Claude makes.

Example command file:

```markdown
---
description: Summarize the current diff
argument-hint: "[files]"
disable-model-invocation: true
allowed-tools: Bash(git *)
---

Summarize these changes:

!`git diff HEAD`

Focus on: $ARGUMENTS
```

### Body format

The body is Markdown. It becomes the user prompt when the command runs. There is no separate "executable" body format; instructions are prose that may include placeholders and dynamic injection markers.

## Invocation Model

### How commands are invoked

In an interactive session, type `/` followed by the command name. Examples:

- `/commit`
- `/research:publish`
- `/apps/web:deploy`
- `/skill-creator:eval-viewer`

A command is only recognized at the start of a message. Skills are the exception: up to six user-invocable skills can be stacked at the start of one message (v2.1.199+), e.g. `/code-review /fix-issue 123`.

### Namespacing and precedence

Built-in slash commands and user-defined skills/commands share a single `/` namespace. Resolution rules:

1. If a skill and a legacy `.claude/commands/<name>.md` file share the same name, the skill wins.
2. Enterprise/managed skills override personal skills; personal skills override project skills.
3. A skill at any level overrides a bundled skill with the same name.
4. Nested skills with clashing names remain available under a qualified name such as `apps/web:deploy`.
5. Plugin skills are namespaced as `plugin-name:skill-name` and cannot collide with non-plugin names.

### Arguments

Everything after the command name is passed as one raw string to `$ARGUMENTS`. Positional and named placeholders split it into individual arguments. There are no default arguments. Quoting uses shell-style double quotes. If the body does not contain `$ARGUMENTS`, Claude Code appends `ARGUMENTS: <input>` to the end of the rendered prompt.

### Output handling

The rendered Markdown is inserted into the conversation as a user prompt. Dynamic shell commands execute during rendering and their stdout replaces the markers. The skill content then stays in context for the rest of the session (subject to compaction budgets).

### Disable mechanisms

- Delete, rename, or move the file/folder.
- `disable-model-invocation: true` — hides from Claude's automatic Skill tool use; the user can still type `/name`.
- `user-invocable: false` — hides from the `/` menu; Claude can still auto-invoke.
- `skillOverrides` in settings — per-skill states: `"on"`, `"name-only"`, `"user-invocable-only"`, `"off"`.
- `--disable-slash-commands` — disables all skills and commands for the session.
- `--bare` / `--safe-mode` / `CLAUDE_CODE_SAFE_MODE=1` — skip user/project skills and commands discovery.
- `disableBundledSkills: true` or `CLAUDE_CODE_DISABLE_BUNDLED_SKILLS=1` — removes only bundled skills/workflows.
- `disableSkillShellExecution: true` in settings — disables `!` shell injection for user/project/plugin skills and commands.

### Trust and permissions

Project-level skills and commands require accepting a workspace trust dialog. Their `allowed-tools` and other permission-related effects only apply after trust is granted. Managed settings can enforce `strictPluginOnlyCustomization` to block user/project skills entirely.

### Live reload

Adding, editing, or removing a skill under `~/.claude/skills/`, the project `.claude/skills/`, or a `.claude/skills/` inside an `--add-dir` directory takes effect within the current session. Creating a top-level skills directory that did not exist at session start requires restarting Claude Code.

## Portability

Claude Code skills and commands are **not portable** to other agentic CLIs without rewriting.

What can be linked with rewrites:

- The prose Markdown body, after placeholder substitution is mapped.

What is provider-specific and must be rewritten or removed:

- `$ARGUMENTS`, `$N`, `$ARGUMENTS[N]`, and named `$arg` placeholders.
- `` !`command` `` and ` ```! ` dynamic shell injection.
- Frontmatter execution hints: `allowed-tools`, `disallowed-tools`, `model`, `effort`, `context`, `agent`, `hooks`, `paths`, `shell`.
- Nested `path:name` and plugin `plugin:name` namespace syntax.
- `${CLAUDE_SKILL_DIR}`, `${CLAUDE_PROJECT_DIR}`, `${CLAUDE_SESSION_ID}`, `${CLAUDE_EFFORT}` substitutions.
- Workspace trust gating and Claude-specific tool names.

Because the command model, placeholder grammar, and permission system are Claude-specific, Claudine should classify these assets as **rewrite needed** rather than linkable as-is.

## Claudine Linking Notes

- Classify Claude Code as **first-class slash/skill support** with **non-portable** commands.
- Do not symlink Claude Code command files directly to another provider; map the Markdown body and rewrite placeholders/frontmatter.
- For cross-provider sync, extract the body prose and convert `$ARGUMENTS` / `$N` to the target provider's argument grammar.
- Strip or expand Claude-specific dynamic shell injection for providers that do not support it.
- Map frontmatter fields individually; many (model, effort, context, agent, allowed-tools) have no universal equivalent.
- Trust gating is Claude-specific; other providers may need their own opt-in mechanisms for repo-level commands.
- The command namespace collision rules (skill beats legacy command, managed > user > project, plugin namespacing) should be preserved if Claudine builds a unified command index.

## Sources

- [Claude Code overview](https://code.claude.com/docs/en/overview)
- [Skills and custom commands documentation](https://code.claude.com/docs/en/skills)
- [Commands reference](https://code.claude.com/docs/en/commands)
- [Claude Code settings](https://code.claude.com/docs/en/settings)
- [CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Environment variables](https://code.claude.com/docs/en/env-vars)
- Local inspection of `~/.claude/commands/`, `~/.claude/skills/`, and `/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/.claude/commands/`
