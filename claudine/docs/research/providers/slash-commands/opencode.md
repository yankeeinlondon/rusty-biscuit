---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7
homepage: https://opencode.ai
docs: https://opencode.ai/docs/commands
slash_docs: https://opencode.ai/docs/commands
support: first_class
locations:
  - os: macos
    scope: user
    path: ~/.config/opencode/commands/<name>.md
    notes: Global custom-command files. OpenCode uses xdg-basedir, so the path follows $XDG_CONFIG_HOME/opencode/commands when that env var is set.
  - os: linux
    scope: user
    path: ~/.config/opencode/commands/<name>.md
    notes: Same as macOS; resolves through $XDG_CONFIG_HOME, defaulting to ~/.config/opencode/commands.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\commands\\<name>.md"
    notes: Windows follows the same xdg-basedir logic because the source uses xdg-basedir unconditionally; default is a .config folder under the user profile unless XDG_CONFIG_HOME is set.
  - os: macos
    scope: user
    path: ~/.config/opencode/opencode.json
    notes: Global JSON config defining commands under the "command" key.
  - os: linux
    scope: user
    path: ~/.config/opencode/opencode.json
    notes: Same as macOS.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\opencode.json"
    notes: Same Windows xdg-basedir behavior as the commands directory.
  - os: macos
    scope: repo
    path: .opencode/commands/<name>.md
    notes: Project custom-command files. OpenCode walks up from the current directory to the git worktree root and discovers every .opencode/commands directory along the way. The singular .opencode/command/<name>.md is also accepted.
  - os: linux
    scope: repo
    path: .opencode/commands/<name>.md
    notes: Same as macOS.
  - os: windows
    scope: repo
    path: ".opencode\\commands\\<name>.md"
    notes: Same as macOS.
  - os: macos
    scope: repo
    path: opencode.json
    notes: Project JSON config defining commands under the "command" key. OpenCode also reads opencode.jsonc and .opencode/opencode.json(c) discovered along the path.
  - os: linux
    scope: repo
    path: opencode.json
    notes: Same as macOS.
  - os: windows
    scope: repo
    path: opencode.json
    notes: Same as macOS.
  - os: macos
    scope: system
    path: /Library/Application Support/opencode/opencode.json
    notes: File-based managed settings can define commands under the "command" key.
  - os: linux
    scope: system
    path: /etc/opencode/opencode.json
    notes: File-based managed settings can define commands under the "command" key.
  - os: windows
    scope: system
    path: "%ProgramData%\\opencode\\opencode.json"
    notes: File-based managed settings can define commands under the "command" key.
format:
  file_names:
    - "*.md"
  frontmatter: true
  required_fields: []
  optional_fields:
    - description
    - agent
    - model
    - variant
    - subtask
  argument_syntax: |
    $ARGUMENTS is replaced with the raw argument string after the command name.
    $1, $2, ... access positional arguments. The highest-numbered $N used in the template receives all remaining arguments joined by a space.
    Arguments are tokenized with shell-style quote awareness: double or single quotes group multi-word values. There are no named argument placeholders.
  body_format: markdown
  notes: |
    The Markdown body below the frontmatter becomes the command template. It is inserted into the conversation as a prompt after placeholder substitution.
    Inline shell output can be injected with !`command`; the command runs in the project root and its stdout replaces the marker.
    File references can be inlined with @path/to/file; the file is read and attached to the prompt.
    JSON config uses the same optional fields plus a required "template" string; the "command" object is keyed by command name.
    Nested directories under commands/ map to slash names: .opencode/commands/review/code.md becomes the command review/code.
command_model:
  invocation: |
    In the TUI, type /name at the start of a message, e.g. /test or /review/code.
    From the CLI, run opencode run --command <name> [args...].
  namespacing: |
    Built-in commands and user-defined commands share one namespace. A user command with the same name as a built-in overrides the built-in.
    Project definitions override global definitions. Markdown files discovered later in the walk from cwd to repo root override earlier ones.
    A JSON "command" entry and a Markdown file with the same name in the same scope are merged; the Markdown template normally wins because the directory scan is applied after the JSON file.
  arguments: |
    Everything after the command name is passed as a raw string to $ARGUMENTS. The same string is tokenized into positional arguments for $1, $2, etc.
    Quotes preserve whitespace: /create-file "my file" src makes $1 = my file and $2 = src.
    If the template contains no $ARGUMENTS and no $N placeholders, the user's argument string is appended to the end of the rendered prompt.
  output_handling: |
    The rendered Markdown body is sent as a user prompt (or as a subtask prompt when the target agent is a subagent or subtask is true).
    !`command` markers are executed during rendering and replaced with stdout. @path references are resolved to file attachments.
  disabled_mechanism: |
    Remove, rename, or delete the Markdown file; remove the key from the JSON "command" object. There is no per-command disable frontmatter or flag.
  notes: |
    Config is loaded once at startup; changes require restarting OpenCode. Project commands do not require a separate trust dialog because OpenCode defaults to allowing operations and gates actions through its permission model.
    Commands can force a subagent invocation with subtask: true or by setting agent to a subagent name. Setting subtask: false prevents a subagent from being spawned for that command.
portability:
  portable: false
  non_portable_assets:
    - "$ARGUMENTS and $N positional placeholders (grammar varies by provider)"
    - "Frontmatter fields: agent, model, variant, subtask"
    - "Inline !`command` shell injection and @path file references"
    - "Directory-to-slash name mapping for nested commands"
    - "JSON inline command definitions keyed under command"
    - "OpenCode agent names, model IDs, and subtask behavior"
  rewrite_needed: true
  notes: |
    The Markdown prose body is mostly portable, but the frontmatter execution hints, placeholders, and file-reference syntax must be mapped to the target provider.
    JSON command objects are provider-specific and should be converted to the target's command file format rather than linked directly.
    Claude Code command files can often be dropped into .opencode/commands/ with minimal changes because the $ARGUMENTS/$N placeholders and !`command` syntax overlap, but Claude-specific frontmatter (allowed-tools, effort, context, etc.) is ignored by OpenCode and should be reviewed.
cli_params:
  - flag: --command <name>
    description: Run a named slash command in non-interactive mode; remaining positional arguments become the command arguments.
    example: opencode run --command test "--coverage"
  - flag: --pure
    description: Run without external plugins. Does not disable commands defined in config or Markdown files.
    example: opencode --pure
  - flag: --model <provider/model>
    description: Override the model used when a command does not specify its own model.
    example: opencode run --command review --model anthropic/claude-sonnet-4-5
  - flag: --agent <name>
    description: Override the agent used when a command does not specify its own agent.
    example: opencode run --command plan --agent plan
env_vars:
  - name: OPENCODE_CONFIG
    effect: Path to an additional JSON config file loaded between global and project configs; can define commands under the "command" key.
  - name: OPENCODE_CONFIG_DIR
    effect: Path to a custom config directory scanned like .opencode for config files and command definitions.
  - name: OPENCODE_CONFIG_CONTENT
    effect: Inline JSON config content merged as a runtime override; can define or override commands.
  - name: OPENCODE_DISABLE_PROJECT_CONFIG
    effect: Skip project opencode.json/opencode.jsonc files. Does not skip .opencode command directories.
  - name: OPENCODE_PURE
    effect: Equivalent to --pure; skips external plugins, not config-defined commands.
  - name: XDG_CONFIG_HOME
    effect: Overrides the default ~/.config base path used to resolve the global opencode config and commands directory.
changes: []
requires_claudine_update: false
reason: |
  This research document records OpenCode's first-class custom command support. It does not require Claudine code or schema changes; any linking logic should treat OpenCode command files as rewrite-needed rather than portable.
---

# OpenCode CLI Custom Commands

## Overview

OpenCode CLI calls user-defined reusable commands **custom commands**. They are surfaced as slash commands in the interactive TUI and can also be invoked from `opencode run`. The feature is distinct from OpenCode's **Agent Skills**, which are loaded on-demand through the `skill` tool rather than invoked with `/`.

Support is **first class**: users can define commands at global, project, managed-system, and runtime-inline scopes; invoke them by name with `/`; pass arguments; override the model or agent; and force subagent execution. Built-in commands such as `/init` and `/review` live in the same namespace and can be overridden by user-defined commands with the same name.

## Locations

Custom commands come from two shapes: Markdown files in `commands/` (or `command/`) directories, and the `"command"` object inside `opencode.json` / `opencode.jsonc` files. OpenCode discovers both from the current working directory up to the git worktree root, plus a global user directory.

| OS | Scope | Path | Notes |
| :- | :---- | :--- | :---- |
| macOS / Linux | User | `~/.config/opencode/commands/<name>.md` | Global Markdown commands. The `commands/` folder may be singular (`command/`). |
| Windows | User | `%USERPROFILE%\.config\opencode\commands\<name>.md` | Default follows xdg-basedir unless `XDG_CONFIG_HOME` is set. |
| macOS / Linux | User | `~/.config/opencode/opencode.json` | Global JSON config; commands live under `"command"`. |
| Windows | User | `%USERPROFILE%\.config\opencode\opencode.json` | Same xdg-basedir behavior. |
| macOS / Linux | Repo | `.opencode/commands/<name>.md` | Project Markdown commands. OpenCode walks from cwd to the worktree root and loads every matching directory. |
| Windows | Repo | `.opencode\commands\<name>.md` | Same behavior. |
| macOS / Linux | Repo | `opencode.json` / `opencode.jsonc` | Project JSON config. OpenCode also reads `.opencode/opencode.json(c)` discovered along the path. |
| Windows | Repo | `opencode.json` / `opencode.jsonc` | Same behavior. |
| macOS | System / managed | `/Library/Application Support/opencode/opencode.json` | Managed JSON config can define commands. |
| Linux | System / managed | `/etc/opencode/opencode.json` | Managed JSON config can define commands. |
| Windows | System / managed | `%ProgramData%\opencode\opencode.json` | Managed JSON config can define commands. |

### Local observations

On the machine used for this research, `~/.config/opencode/commands/` exists and contains mostly **symlinks to Claude Code command files** under `~/.claude/commands/` (for example `commit.md`, `clarify.md`, `implement-feature.md`). Nested directories such as `vue/`, `review/`, and `local/` are also present, which OpenCode maps to slash names like `/vue/add-component` and `/review/code-review`. The global OpenCode config at `~/.config/opencode/opencode.json` does not define any commands. This repository (`/Users/ken/.claudine/worktrees/rusty-biscuit/claudine`) has a `.opencode` directory but no `commands/` subdirectory.

## File Format

### Markdown commands

A command is a single Markdown file named after the command. It lives directly inside the `command(s)/` folder:

```text
.opencode/commands/
├── test.md
├── component.md
└── review/
    └── code.md
```

The file name (including subdirectories) becomes the command name after stripping the `command(s)/` prefix and `.md` extension. The example above yields `test`, `component`, and `review/code`.

#### Frontmatter

YAML frontmatter is optional. Recognized fields:

| Field | Purpose | Example |
| :---- | :------ | :------ |
| `description` | Shown in the TUI command picker and help. | `description: Run tests with coverage` |
| `agent` | Agent that executes the command. | `agent: build` |
| `model` | Model override for this command. | `model: anthropic/claude-sonnet-4-5` |
| `variant` | Model variant for this command. | `variant: high` |
| `subtask` | Force (`true`) or prevent (`false`) subagent execution. | `subtask: true` |

The Markdown body below the frontmatter is the command **template**.

#### Argument substitution

| Token | Meaning |
| :---- | :------ |
| `$ARGUMENTS` | The entire raw argument string after the command name. |
| `$1`, `$2`, ... | Positional arguments. The highest `$N` used receives all remaining arguments joined by a space. |

Arguments are tokenized with shell-style quoting: `/create-file "my file" src` produces `$1 = my file` and `$2 = src`.

#### Inline shell and file references

- `!` `` `command` `` runs the shell command and inserts its stdout into the template.
- `@path/to/file` resolves the file and attaches it to the prompt.

Example command file:

```markdown
---
description: Summarize recent commits
agent: plan
---

Summarize these recent commits:

!`git log --oneline -10`

Focus on: $ARGUMENTS
```

### JSON commands

Commands can also be defined inline in any `opencode.json` / `opencode.jsonc` file:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "command": {
    "test": {
      "template": "Run the full test suite with coverage and show failures.",
      "description": "Run tests with coverage",
      "agent": "build",
      "model": "anthropic/claude-sonnet-4-5"
    }
  }
}
```

The `"template"` field is required; all other fields are optional and match the Markdown frontmatter fields.

## Invocation Model

### How commands are invoked

In the TUI, type `/` followed by the command name at the start of a message:

- `/test`
- `/component Button`
- `/review/code src/lib.rs`

From the CLI, use the `--command` flag:

```bash
opencode run --command test --coverage
```

For nested commands, the slash is part of the name: `/review/code`.

### Namespacing and precedence

Built-in commands and user-defined commands share a single namespace. Resolution rules:

1. User-defined commands override built-in commands of the same name.
2. Project-scoped definitions override global-scoped definitions.
3. When walking from cwd to the repo root, closer directories override farther ones.
4. Markdown files normally override JSON entries of the same name in the same scope because the directory scan is applied after JSON config files.
5. Managed/system config layers are loaded last and can override user/project settings.

There is no separate namespacing prefix such as `user:` or `project:`.

### Arguments

Everything after the command name is passed as one raw string to `$ARGUMENTS`. The same string is split into positional tokens for `$1`, `$2`, etc. There are no named arguments and no default-argument mechanism.

If the template does not contain `$ARGUMENTS` or any `$N` placeholder, OpenCode appends the raw argument string to the end of the rendered prompt.

### Output handling

The rendered Markdown template becomes the user prompt. During rendering:

1. `$ARGUMENTS` and `$N` placeholders are substituted.
2. `!` `` `command` `` markers are executed and replaced with their stdout.
3. `@path` references are resolved to file attachments.

The resulting prompt is sent to the conversation. If the command targets a subagent or has `subtask: true`, the prompt is sent as a subtask instead.

### Disable mechanisms

- Delete, rename, or move the Markdown file.
- Remove the key from the JSON `"command"` object.
- Override a built-in by defining a command with the same name.

There is no per-command `disable` frontmatter, no `--disable-commands` flag, and no safe mode that suppresses commands.

### Trust and permissions

Project-level commands do not require a separate workspace-trust dialog. OpenCode defaults to allowing operations and gates individual actions through its permission model (`permission` in `opencode.json`). A command file itself is treated as trusted config once it is discovered.

## Portability

OpenCode custom commands are **not portable** to other agentic CLIs without rewriting.

What can be linked with rewrites:

- The Markdown prose body, after placeholder substitution is mapped.
- Simple command files that only use `$ARGUMENTS` and `$1`/`$2` can often be reused in Claude Code with minimal changes because the placeholder syntax overlaps.

What is provider-specific and must be rewritten or removed:

- `$ARGUMENTS` / `$N` placeholders (other providers may use `{{args}}`, `$ARGUMENTS[N]`, or named variables).
- `!` `` `command` `` shell injection and `@path` file references.
- Frontmatter fields `agent`, `model`, `variant`, and `subtask`.
- Directory-to-slash name mapping for nested commands.
- JSON inline command definitions under `"command"`.
- OpenCode-specific agent names, model IDs, and subtask behavior.

Because the command model, placeholder grammar, and permission system are OpenCode-specific, Claudine should classify these assets as **rewrite needed** rather than linkable as-is.

## Claudine Linking Notes

- Classify OpenCode as **first-class custom command support** with **non-portable** command files.
- Do not symlink OpenCode command files directly to another provider; extract the Markdown body and rewrite placeholders and frontmatter.
- For cross-provider sync, map `$ARGUMENTS` / `$N` to the target provider's argument grammar and strip or expand OpenCode-specific shell/file-reference syntax.
- Map frontmatter fields individually; many (`agent`, `model`, `variant`, `subtask`) have no universal equivalent.
- When linking Claude Code commands into OpenCode, place them in `.opencode/commands/` or `~/.config/opencode/commands/`. Claude-specific frontmatter such as `allowed-tools` is ignored by OpenCode, so review whether tool permissions need to be expressed through OpenCode's `permission` config instead.
- Preserve the directory-to-slash naming rule if the target provider supports nested command names; otherwise flatten nested directories into prefixed names.

## Sources

- [OpenCode homepage](https://opencode.ai)
- [OpenCode commands documentation](https://opencode.ai/docs/commands)
- [OpenCode config documentation](https://opencode.ai/docs/config)
- [OpenCode CLI documentation](https://opencode.ai/docs/cli)
- [OpenCode TUI documentation](https://opencode.ai/docs/tui)
- [OpenCode agent skills documentation](https://opencode.ai/docs/skills)
- [OpenCode published config schema](https://opencode.ai/config.json)
- [OpenCode GitHub repository](https://github.com/anomalyco/opencode)
- OpenCode source: `packages/core/src/config/plugin/command.ts`, `packages/core/src/config/command.ts`, `packages/core/src/config.ts`, `packages/core/src/global.ts`, `packages/opencode/src/session/prompt.ts`, `packages/opencode/src/config/markdown.ts`, `packages/core/src/plugin/internal.ts`, `packages/core/src/plugin/command.ts`, `packages/core/src/plugin/skill/customize-opencode.md`
- Local inspection of `~/.config/opencode/commands/`, `~/.config/opencode/opencode.json`, and `/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/.opencode/`
