---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7
homepage: https://kilo.ai/
docs: https://kilo.ai/docs
slash_docs: https://kilo.ai/docs/customize/workflows
support: first_class
locations:
  - os: macos
    scope: user
    path: ~/.config/kilo/commands/<name>.md
    notes: Modern global Markdown commands. The directory may be singular (`command/`). Also loaded from legacy `~/.kilocode/commands/` and `~/.opencode/commands/`.
  - os: linux
    scope: user
    path: ~/.config/kilo/commands/<name>.md
    notes: Same as macOS. Resolves through `$XDG_CONFIG_HOME`, defaulting to `~/.config/kilo/commands/`.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\kilo\\commands\\<name>.md"
    notes: Same xdg-basedir resolution as macOS/Linux unless `XDG_CONFIG_HOME` is set. Legacy `%USERPROFILE%\.kilocode\commands\` and `%USERPROFILE%\.opencode\commands\` are also scanned.
  - os: macos
    scope: user
    path: ~/.config/kilo/kilo.json
    notes: Global JSON config defining commands under the `command` key. `kilo.jsonc`, legacy `opencode.json`, and `opencode.jsonc` are also accepted.
  - os: linux
    scope: user
    path: ~/.config/kilo/kilo.json
    notes: Same as macOS.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\kilo\\kilo.json"
    notes: Same xdg-basedir behavior.
  - os: macos
    scope: repo
    path: .kilo/commands/<name>.md
    notes: Project Markdown commands. Kilo walks from the current directory up to the git worktree root and loads every `.kilo/`, `.kilocode/`, and `.opencode/` commands directory. The singular `command/` form is also accepted.
  - os: linux
    scope: repo
    path: .kilo/commands/<name>.md
    notes: Same as macOS.
  - os: windows
    scope: repo
    path: ".kilo\\commands\\<name>.md"
    notes: Same walk-up behavior; legacy `.kilocode\commands\` and `.opencode\commands\` are also scanned.
  - os: macos
    scope: repo
    path: kilo.json
    notes: Project JSON config. Also reads `.kilo/kilo.json(c)`, `.kilocode/kilo.json(c)`, and legacy `opencode.json(c)` discovered along the path.
  - os: linux
    scope: repo
    path: kilo.json
    notes: Same as macOS.
  - os: windows
    scope: repo
    path: kilo.json
    notes: Same as macOS.
  - os: macos
    scope: system
    path: /Library/Application Support/kilo/kilo.json
    notes: Managed/enterprise JSON config can define commands under the `command` key.
  - os: linux
    scope: system
    path: /etc/kilo/kilo.json
    notes: Managed/enterprise JSON config.
  - os: windows
    scope: system
    path: "%ProgramData%\\kilo\\kilo.json"
    notes: Managed/enterprise JSON config.
  - os: macos
    scope: other
    path: "$KILO_CONFIG_DIR/commands/<name>.md"
    notes: Extra config directory appended to the search list when `KILO_CONFIG_DIR` is set.
  - os: linux
    scope: other
    path: "$KILO_CONFIG_DIR/commands/<name>.md"
    notes: Same as macOS.
  - os: windows
    scope: other
    path: "%KILO_CONFIG_DIR%\\commands\\<name>.md"
    notes: Same as macOS.
format:
  file_names:
    - "*.md"
  frontmatter: true
  required_fields: []
  optional_fields:
    - description
    - agent
    - model
    - subtask
  argument_syntax: |
    $ARGUMENTS is replaced with the raw argument string after the command name.
    $1, $2, ... access positional arguments. The highest-numbered $N used in the template receives all remaining arguments joined by a space.
    Arguments are tokenized with shell-style quote awareness: double or single quotes group multi-word values. There are no named argument placeholders.
  body_format: markdown
  notes: |
    The Markdown body below the optional frontmatter becomes the command template. It is inserted into the conversation as a prompt after placeholder substitution.
    Inline shell output can be injected with !`command`; the command runs and its stdout replaces the marker.
    File references can be inlined with @path/to/file; the file is read and attached to the prompt.
    JSON config commands use the same optional fields plus a required "template" string; the "command" object is keyed by command name.
    Nested directories under commands/ map to slash names: `.kilo/commands/review/code.md` becomes `/review/code`.
command_model:
  invocation: |
    In the TUI, type /name at the start of a message, e.g. /submit-pr or /review/code.
    From the CLI, run kilo run --command <name> [args...].
  namespacing: |
    Built-in commands and user-defined commands share one namespace. A user command with the same name as a built-in overrides the built-in.
    Project definitions override global definitions. Markdown files discovered later in the walk from cwd to repo root override earlier ones.
    A JSON "command" entry and a Markdown file with the same name in the same scope are merged; the Markdown template normally wins because the directory scan is applied after the JSON file.
    There is no separate namespacing prefix such as user: or project:.
  arguments: |
    Everything after the command name is passed as a raw string to $ARGUMENTS. The same string is tokenized into positional arguments for $1, $2, etc.
    Quotes preserve whitespace: /create-file "my file" src makes $1 = my file and $2 = src.
    If the template contains no $ARGUMENTS and no $N placeholders, the user's argument string is appended to the end of the rendered prompt.
  output_handling: |
    The rendered Markdown body is sent as a user prompt (or as a subtask prompt when the target agent is a subagent or subtask: true).
    !`command` markers are executed during rendering and replaced with stdout. @path references are resolved to file attachments.
  disabled_mechanism: |
    Remove, rename, or delete the Markdown file; remove the key from the JSON "command" object. There is no per-command disable frontmatter or flag.
    KILO_DISABLE_PROJECT_CONFIG=1 skips all project-level config files and directories, which also disables project commands.
  notes: |
    Config is loaded once at startup; changes require restarting Kilo. Project commands do not require a separate trust dialog because Kilo gates actions through its permission model.
    Commands can force a subagent invocation with subtask: true or by setting agent to a subagent name. Setting subtask: false prevents a subagent from being spawned for that command.
portability:
  portable: false
  non_portable_assets:
    - "$ARGUMENTS and $N positional placeholders (grammar varies by provider)"
    - "Frontmatter fields: agent, model, subtask"
    - "Inline !`command` shell injection and @path file references"
    - "Directory-to-slash name mapping for nested commands"
    - "JSON inline command definitions keyed under command"
    - "Kilo agent names, model IDs, and subtask behavior"
  rewrite_needed: true
  notes: |
    The Markdown prose body is mostly portable, but the frontmatter execution hints, placeholders, and file-reference syntax must be mapped to the target provider.
    JSON command objects are provider-specific and should be converted to the target's command file format rather than linked directly.
    Claude Code command files can often be dropped into `.kilo/commands/` with minimal changes because the $ARGUMENTS/$N placeholders and !`command` syntax overlap, but Claude-specific frontmatter (allowed-tools, effort, context, etc.) is ignored by Kilo and should be reviewed.
cli_params:
  - flag: --command <name>
    description: Run a named slash command in non-interactive mode; remaining positional arguments become the command arguments.
    example: kilo run --command submit-pr "--draft"
  - flag: --pure
    description: Run without external plugins. Does not disable commands defined in config or Markdown files.
    example: kilo --pure
  - flag: --model <provider/model>
    description: Override the model used when a command does not specify its own model.
    example: kilo run --command review --model anthropic/claude-sonnet-4-5
  - flag: --agent <name>
    description: Override the agent used when a command does not specify its own agent.
    example: kilo run --command plan --agent plan
  - flag: --variant <name>
    description: Override the model variant for this run.
    example: kilo run --command review --variant high
  - flag: --auto
    description: Run autonomously without prompts. Affects how permission approvals are handled, not command discovery.
    example: kilo run --command test --auto
env_vars:
  - name: KILO_CONFIG
    effect: Path to an additional config file loaded between global and project configs; can define commands under the "command" key.
  - name: KILO_CONFIG_DIR
    effect: Path to an additional config directory scanned like .kilo for config files and command definitions.
  - name: KILO_CONFIG_CONTENT
    effect: Inline JSON config content merged as a runtime override; can define or override commands.
  - name: KILO_DISABLE_PROJECT_CONFIG
    effect: Skip project kilo.json/kilo.jsonc files and .kilo/.kilocode/.opencode directories, which disables project-level commands.
  - name: KILO_PURE
    effect: Equivalent to --pure; skips external plugins, not config-defined commands.
  - name: KILO_DISABLE_DEFAULT_PLUGINS
    effect: Skip default plugins. Does not affect commands.
  - name: XDG_CONFIG_HOME
    effect: Overrides the default ~/.config base path used to resolve the global Kilo config and commands directory.
  - name: HOME
    effect: Used to expand ~ in config paths and command file locations.
changes: []
requires_claudine_update: false
reason: |
  This research document records Kilo Code's first-class workflow/slash-command support. It does not require Claudine code or schema changes; any linking logic should treat Kilo command files as rewrite-needed rather than portable.
---

# Kilo Code Workflows and Slash Commands

## Overview

Kilo Code calls user-defined reusable commands **workflows** and surfaces them as **slash commands** in the chat interface. A file at `.kilo/commands/submit-pr.md` is invoked by typing `/submit-pr`. This is distinct from Kilo's **Agent Skills**, which are loaded on-demand through the `skill` tool rather than invoked with `/`.

Support is **first class**: users can define commands at global, project, managed-system, and runtime-inline scopes; invoke them by name with `/`; pass arguments; override the model or agent; and force subagent execution. Built-in commands such as `/init`, `/review`, `/local-review`, and `/local-review-uncommitted` live in the same namespace and can be overridden by user-defined commands with the same name.

## Locations

Custom commands come from two shapes: Markdown files in `command/` or `commands/` directories, and the `"command"` object inside `kilo.json` / `kilo.jsonc` files. Kilo discovers config directories by walking from the current working directory up to the git worktree root, and also loads global user and managed-system directories. Three directory names are scanned: `.kilo` (modern), `.kilocode` (legacy), and `.opencode` (legacy).

| OS | Scope | Path | Notes |
| :- | :---- | :--- | :---- |
| macOS / Linux | User | `~/.config/kilo/commands/<name>.md` | Modern global Markdown commands. The singular `command/` form is also accepted. Legacy `~/.kilocode/commands/` and `~/.opencode/commands/` are scanned as well. |
| Windows | User | `%USERPROFILE%\.config\kilo\commands\<name>.md` | Same xdg-basedir resolution unless `XDG_CONFIG_HOME` is set. Legacy `.kilocode\commands\` and `.opencode\commands\` are also scanned. |
| macOS / Linux | User | `~/.config/kilo/kilo.json` | Modern global JSON config; commands live under `"command"`. `kilo.jsonc`, `opencode.json`, and `opencode.jsonc` are also accepted. |
| Windows | User | `%USERPROFILE%\.config\kilo\kilo.json` | Same xdg-basedir behavior. |
| macOS / Linux | Repo | `.kilo/commands/<name>.md` | Project Markdown commands. Kilo walks from cwd to the worktree root and loads commands from every `.kilo/`, `.kilocode/`, and `.opencode/` directory. |
| Windows | Repo | `.kilo\commands\<name>.md` | Same walk-up behavior; legacy directories are included. |
| macOS / Linux | Repo | `kilo.json` / `kilo.jsonc` | Project JSON config. Also reads `.kilo/kilo.json(c)`, `.kilocode/kilo.json(c)`, and legacy `opencode.json(c)` discovered along the path. |
| Windows | Repo | `kilo.json` / `kilo.jsonc` | Same behavior. |
| macOS | System / managed | `/Library/Application Support/kilo/kilo.json` | Managed/enterprise JSON config can define commands under `"command"`. |
| Linux | System / managed | `/etc/kilo/kilo.json` | Managed/enterprise JSON config. |
| Windows | System / managed | `%ProgramData%\kilo\kilo.json` | Managed/enterprise JSON config. |
| All | Env | `$KILO_CONFIG_DIR/commands/<name>.md` | Extra config directory appended to the search list when `KILO_CONFIG_DIR` is set. |

### Local observations

On the machine used for this research, `~/.kilo/` does not exist. `~/.config/kilo/` exists and contains only `kilo.jsonc` (with just a `$schema` line), `node_modules`, and lock files; there is no `commands/` or `command/` subdirectory. The repository `/Users/ken/.claudine/worktrees/rusty-biscuit/claudine` does not contain a `.kilo/`, `.kilocode/`, or `.opencode/` directory. No local Kilo slash-command files were observed.

## File Format

### Markdown commands

A command is a single Markdown file named after the command, inside a `command/` or `commands/` folder:

```text
.kilo/commands/
├── submit-pr.md
├── test.md
└── review/
    └── code.md
```

The file path under `command(s)/` becomes the slash name. The example above yields `submit-pr`, `test`, and `review/code`.

#### Frontmatter

YAML frontmatter is optional. Recognized fields:

| Field | Purpose | Example |
| :---- | :------ | :------ |
| `description` | Shown in the command picker and help. | `description: Submit a pull request with checks` |
| `agent` | Agent that executes the command. | `agent: code` |
| `model` | Model override for this command. | `model: anthropic/claude-sonnet-4-5` |
| `subtask` | Force (`true`) or prevent (`false`) subagent execution. | `subtask: true` |

The Markdown body below the frontmatter is the command **template**.

#### Argument substitution

| Token | Meaning |
| :---- | :------ |
| `$ARGUMENTS` | The entire raw argument string after the command name. |
| `$1`, `$2`, ... | Positional arguments. The highest `$N` used receives all remaining arguments joined by a space. |

Arguments are tokenized with shell-style quoting awareness: `/create-file "my file" src` produces `$1 = my file` and `$2 = src`.

#### Inline shell and file references

- `` !`command` `` runs the shell command and inserts its stdout into the template.
- `@path/to/file` resolves the file and attaches it to the prompt.

Example command file:

```markdown
---
description: Run tests and fix failures
agent: code
---

Run all tests in $1 and fix failures.
Use $ARGUMENTS for the full arg string.
```

### JSON commands

Commands can also be defined inline in any `kilo.json` / `kilo.jsonc` file:

```json
{
  "$schema": "https://app.kilo.ai/config.json",
  "command": {
    "submit-pr": {
      "template": "Submit a pull request with full checks...",
      "description": "Submit a pull request with full checks",
      "agent": "code",
      "model": "anthropic/claude-sonnet-4-5",
      "subtask": true
    }
  }
}
```

The `"template"` field is required; all other fields are optional and match the Markdown frontmatter fields.

## Invocation Model

### How commands are invoked

In the interactive TUI, type `/` followed by the command name at the start of a message:

- `/submit-pr`
- `/test --coverage`
- `/review/code src/lib.rs`

From the CLI, use the `--command` flag:

```bash
kilo run --command submit-pr "--draft"
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

If the template does not contain `$ARGUMENTS` or any `$N` placeholder, Kilo appends the raw argument string to the end of the rendered prompt.

### Output handling

The rendered Markdown template becomes the user prompt. During rendering:

1. `$ARGUMENTS` and `$N` placeholders are substituted.
2. `` !`command` `` markers are executed and replaced with their stdout.
3. `@path` references are resolved to file attachments.

The resulting prompt is sent to the conversation. If the command targets a subagent or has `subtask: true`, the prompt is sent as a subtask instead.

### Disable mechanisms

- Delete, rename, or move the Markdown file.
- Remove the key from the JSON `"command"` object.
- Override a built-in by defining a command with the same name.

There is no per-command `disable` frontmatter, no `--disable-commands` flag, and no safe mode that suppresses commands. Setting `KILO_DISABLE_PROJECT_CONFIG=1` disables all project-level config, including project commands.

### Trust and permissions

Project-level commands do not require a separate workspace-trust dialog. Kilo gates individual actions through its permission model (`permission` in `kilo.json`). A command file itself is treated as trusted config once it is discovered.

## Portability

Kilo Code slash commands are **not portable** to other agentic CLIs without rewriting.

What can be linked with rewrites:

- The Markdown prose body, after placeholder substitution is mapped.
- Simple command files that only use `$ARGUMENTS` and `$1`/`$2` can often be reused in Claude Code with minimal changes because the placeholder syntax overlaps.

What is provider-specific and must be rewritten or removed:

- `$ARGUMENTS` / `$N` placeholders (other providers may use `{{args}}`, `$ARGUMENTS[N]`, or named variables).
- `` !`command` `` shell injection and `@path` file references.
- Frontmatter fields `agent`, `model`, and `subtask`.
- Directory-to-slash name mapping for nested commands.
- JSON inline command definitions under `"command"`.
- Kilo-specific agent names, model IDs, and subtask behavior.

Because the command model, placeholder grammar, and permission system are Kilo-specific, Claudine should classify these assets as **rewrite needed** rather than linkable as-is.

## Claudine Linking Notes

- Classify Kilo Code as **first-class slash-command/workflow support** with **non-portable** command files.
- Do not symlink Kilo command files directly to another provider; extract the Markdown body and rewrite placeholders and frontmatter.
- For cross-provider sync, map `$ARGUMENTS` / `$N` to the target provider's argument grammar and strip or expand Kilo-specific shell/file-reference syntax.
- Map frontmatter fields individually; many (`agent`, `model`, `subtask`) have no universal equivalent.
- When linking Claude Code commands into Kilo, place them in `.kilo/commands/` or `~/.config/kilo/commands/`. Claude-specific frontmatter such as `allowed-tools` is ignored by Kilo, so review whether tool permissions need to be expressed through Kilo's `permission` config instead.
- Preserve the directory-to-slash naming rule if the target provider supports nested command names; otherwise flatten nested directories into prefixed names.

## Sources

- [Kilo homepage](https://kilo.ai/)
- [Kilo Documentation](https://kilo.ai/docs)
- [Kilo Workflows / slash commands documentation](https://kilo.ai/docs/customize/workflows)
- [Kilo Skills documentation](https://kilo.ai/docs/customize/skills)
- [Kilo CLI documentation](https://kilo.ai/docs/code-with-ai/platforms/cli)
- [Kilo GitHub repository](https://github.com/Kilo-Org/kilocode)
- [Kilo published config schema](https://app.kilo.ai/config.json)
- Kilo CLI built-in configuration reference extracted from `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@kilocode/cli/bin/.kilo`
- Local inspection of `~/.config/kilo/`, `~/.kilo/`, and `/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/.kilo/`
