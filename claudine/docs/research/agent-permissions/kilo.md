---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-01
agent: open_code
model: kimi-for-coding/k2p7

cli_params:
  - param: auto
    style: switch
    description: Start a non-interactive `kilo run` session in auto-approve mode. Permission requests for the main session and tracked Task child sessions are approved automatically unless explicitly denied.
    example: kilo run --auto "refactor the auth module"
    example_description: Runs a headless task where all non-denied permission requests are approved, including permissions requested by spawned subagent tasks.
  - param: dangerously-skip-permissions
    style: switch
    description: Start a non-interactive `kilo run` session that auto-approves any permission request that is not explicitly denied. Unlike --auto, this flag does not track or auto-approve Task child-session permissions.
    example: kilo run --dangerously-skip-permissions "deploy to staging"
    example_description: Runs a headless deployment prompt with all non-denied permission requests approved for the main session only.
  - param: agent
    style: switch
    description: Select the active agent for the session. Each agent can define its own permission profile, so this flag determines which permission set is evaluated for tool calls.
    example: kilo --agent plan
    example_description: Starts an interactive session with the plan agent, which defaults write and bash permissions to ask.
  - param: permissions
    style: switch
    description: For `kilo agent create` only. Comma-separated list of permissions to allow when scaffolding a new agent. Any permission not listed is denied in the generated agent.
    example: kilo agent create --permissions read,grep --mode subagent
    example_description: Creates a new read-only subagent that is allowed only read and grep.

env_vars:
  - name: KILO_PERMISSION
    effect: Provides an inline JSON permissions configuration that is merged into the effective config for the session, overriding config-file permissions.
  - name: KILO_CONFIG_CONTENT
    effect: Provides inline JSON config content that can include a permission object and overrides most config file values.
  - name: KILO_CONFIG
    effect: Points to a custom config file path; that file may define permissions and is loaded between global and project config.
  - name: KILO_DISABLE_PROJECT_CONFIG
    effect: When set, skips loading project-level config files (kilo.json, opencode.json, .kilo/, .kilocode/), so only global, env, and CLI sources shape permissions.

config_files:
  user: ~/.config/kilo/kilo.json
  repo: kilo.json

precedence: "CLI flags (e.g. --auto, --dangerously-skip-permissions) > KILO_PERMISSION > macOS managed preferences / MDM > managed config directory > active organization config (Kilo Gateway) > KILO_CONFIG_CONTENT > project config (kilo.json and .kilo/) > KILO_CONFIG custom path > global user config (~/.config/kilo/kilo.json) > remote .well-known/opencode. Within a config object, rules are evaluated in order and the last matching rule wins."

default_posture: "With no configuration, Kilo Code uses permissive defaults: most built-in tools are allowed automatically, while doom_loop and external_directory ask for approval. The read tool is allowed by default, but .env files are denied."

agent_permissions:
  allowed: true
  fm_properties:
    - permission

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "--auto or --dangerously-skip-permissions on `kilo run`; interactive TUI sessions can also toggle auto-approve permissions from the command palette"

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - Kilo Code permissions are tool-centric (read, edit, bash, webfetch, skill, question, doom_loop, external_directory, etc.) and support wildcard/last-match-wins patterns, while PolicyEngine's canonical model is organized around filesystem, command, network, MCP, agent, and runtime axes.
    - The default permissive posture (most tools allow by default) is the inverse of PolicyEngine's typical ask/deny defaults, requiring explicit modeling.
    - external_directory is a path-scoped permission key rather than a true tool, and PolicyEngine would need to represent it as a workspace/external path rule with matching semantics.
    - doom_loop is a runtime recovery guard, not a standard tool or resource permission.
    - Agent-specific permissions and task subagent permissions are supported by Kilo but require PolicyEngine to scope rules by agent name.
    - MCP tools are addressed by server-prefixed wildcard names (e.g., mymcp_*); PolicyEngine's MCP axis may not support arbitrary tool-name wildcard rules.
    - Kilo-specific permission keys (agent_manager, notebook_read, notebook_edit, notebook_execute, repo_clone, repo_overview) extend the OpenCode grammar and have no direct PolicyEngine mapping.

changes: []

requires_claudine_update: true
reason: "Kilo Code's permission grammar is inherited from OpenCode: tool-centric wildcard patterns, last-rule-wins evaluation, external_directory, doom_loop, agent/task permissions, permissive defaults, and Kilo-specific keys do not map cleanly to PolicyEngine's canonical axes. Supporting Kilo permissions accurately will require backend work in the PolicyEngine OpenCode/Kilo backend and mutation planning for kilo.json permission objects."
---

# Kilo Code Permissions

## Introduction to Kilo Code Permissions

Kilo Code controls tool access with a single `permission` configuration object. Each permission key maps to one or more tools and resolves to one of three actions:

- `"allow"` — run without approval
- `"ask"` — prompt the user for approval
- `"deny"` — block the action

Kilo CLI is a fork of [OpenCode](https://opencode.ai), and the permission system is largely the same. Permissions can be configured through JSON/JSONC config files, inline environment variables, Markdown agent frontmatter, and a small set of CLI flags. Unlike some other agents, Kilo defaults to a permissive posture: most tools are allowed unless a rule says otherwise.

### Configuration files

The `permission` key lives in `kilo.json` or `kilo.jsonc` (legacy `opencode.json` / `opencode.jsonc` are still read). It can be a single action string that applies to all tools, or an object that maps tool names to action strings or granular pattern objects. See [Configuring the Default](#configuring-the-default) for file locations and examples.

### Environment variables

The main environment variables that influence permissions are:

| Variable | Effect |
| :----- | :----- |
| `KILO_PERMISSION` | Inline JSON permissions config merged into the effective config. |
| `KILO_CONFIG_CONTENT` | Inline JSON config content; can include a full `permission` object. |
| `KILO_CONFIG` | Path to a custom config file that may contain a `permission` object. |
| `KILO_DISABLE_PROJECT_CONFIG` | Skip loading project-level config files, so only global/env/CLI sources apply. |

### CLI parameters

Only a few CLI switches directly affect permissions:

| Flag | What it does |
| :----- | :----- |
| `--auto` | On `kilo run`, enable auto-approve mode for the main session and tracked Task child sessions. |
| `--dangerously-skip-permissions` | On `kilo run`, auto-approve non-denied permission requests for the main session only. |
| `--agent <name>` | Use the named agent, whose `permission` profile (if any) is applied. |
| `--permissions <list>` | Only for `kilo agent create`. Lists permissions to allow in the generated agent. |

### Precedence

Effective permissions are built from multiple layers. Highest-wins ordering is:

1. CLI flags such as `--auto` and `--dangerously-skip-permissions`
2. `KILO_PERMISSION`
3. macOS managed preferences / MDM
4. Managed config directory
5. Active organization config from Kilo Gateway
6. `KILO_CONFIG_CONTENT`
7. Project `kilo.json` / `kilo.jsonc` and `.kilo/` / `.kilocode/` config directories
8. Custom config path from `KILO_CONFIG`
9. Global `~/.config/kilo/kilo.json` / `kilo.jsonc`
10. Remote `.well-known/opencode` organizational defaults

Within any config object, rules are evaluated in order and the **last matching rule wins**.

## Permissions Use Cases

### Default

If no environment variable, config file, or CLI switch configures permissions, Kilo Code starts from permissive defaults:

- Most tools are `"allow"`.
- `doom_loop` and `external_directory` are `"ask"`.
- `read` is `"allow"`, but `.env` files are denied by default.

A PolicyEngine description of the default posture would be:

- `can_read(path)` → Allow for workspace paths; Deny for `.env` files.
- `can_write(path)` → Allow for workspace paths.
- `can_execute(command)` → Allow for bash commands.
- `can_access_domain(domain)` → Allow for webfetch/websearch.
- `can_use_mcp_server(server)` / `can_use_mcp_tool(server, tool)` → Allow.
- `can_spawn_subagent(agent)` → Allow.
- `can_loop_recovery()` → Ask (doom_loop).
- `can_access_external_directory(path)` → Ask.

This use case is not ergonomic in PolicyEngine without adjustments. PolicyEngine's canonical axes (filesystem, command, network, MCP, agent, runtime) do not line up one-to-one with Kilo's tool keys, and the permissive default is the opposite of PolicyEngine's usual ask/deny baseline. No changes are required to describe the broad idea, but full coverage of the default posture would need new mappings for `doom_loop`, `external_directory`, and the `.env` deny rule.

### Whitelisting

To start with no permissions and require every needed permission to be asked for or explicitly declared, set a global deny rule and then add specific allow or ask rules.

```json
{
  "$schema": "https://app.kilo.ai/config.json",
  "permission": {
    "*": "deny",
    "read": "allow",
    "grep": "allow",
    "glob": "allow",
    "bash": {
      "*": "ask",
      "git status *": "allow",
      "git log *": "allow"
    },
    "edit": "ask"
  }
}
```

In an interactive session, `ask` causes Kilo to prompt. In a non-interactive `kilo run`, `ask` is effectively deny because there is no user to approve, so you should pre-declare `allow` rules for any tool the headless session needs.

Because Kilo does not have a dedicated `--permission` runtime flag, you usually whitelist through config or environment:

```bash
# Headless run with a locked-down allowlist via env
KILO_PERMISSION='{"*":"deny","read":"allow","grep":"allow","bash":{"git status *":"allow"}}' \
  kilo run "summarize the auth module"

# Use the built-in plan agent to default bash/edit to ask
kilo --agent plan

# Create and use a read-only subagent
kilo agent create --permissions read,grep --mode subagent --description "read-only explorer"
kilo --agent read-only-explorer
```

PolicyEngine can express this use case as `SetApprovalMode` to a deny-by-default posture plus explicit `GrantRead`, `AllowCommand`, and similar rules. It is not fully ergonomic because Kilo's tool-key wildcard patterns and last-match-wins ordering do not map directly to PolicyEngine's rule model. Without changes, PolicyEngine could describe the intent but not the exact pattern-matching behavior or agent-scoped deny defaults.

### YOLO

Kilo Code's YOLO mode is called **auto-approve**. A session can be put into this mode by:

- Starting `kilo run` with `--auto`, for example `kilo run --auto "..."`.
- Starting `kilo run` with `--dangerously-skip-permissions`.
- Setting `permission: "allow"` or `permission: { "*": "allow" }` in config.
- Using an agent whose permissions are all `allow`.
- Toggling **Enable auto-approve permissions** from the interactive TUI command palette (inherited from OpenCode).

Availability:

- **Interactive sessions**: yes, via the TUI command palette auto-approve toggle or by using an agent configured with all-allow permissions.
- **Non-interactive sessions**: yes, via `kilo run --auto` or `kilo run --dangerously-skip-permissions`.

When in auto-approve mode:

- **Allowed**: any tool call that is not explicitly denied is approved automatically, including bash, edit/write, webfetch, websearch, MCP tools, and subagent spawns.
- **Still enforced**: explicit `"deny"` rules in config are still enforced; if a tool is denied it will not run.
- **Not allowed**: auto-approve cannot override managed/MDM config that denies an action.

### Root User

The public Kilo Code documentation does not describe any special permission behavior when the CLI is started as root or under `sudo`. The source code does not contain a root/sudo gate for auto-approve mode. Unlike Claude Code, there is no documented restriction that disables YOLO mode for root sessions. Therefore, YOLO mode remains available to root sessions unless an administrator blocks it through managed config.

### Configuring the Default

Default permissions are configured through JSON/JSONC config files at two main scopes:

- **User scope**: `~/.config/kilo/kilo.json` or `kilo.jsonc` (legacy `opencode.json` / `opencode.jsonc` are also read). Applies across all projects.
- **Repo scope**: `kilo.json` or `kilo.jsonc` in the project root, or inside `.kilo/` / `.kilocode/` (legacy `.opencode/` is also read). Applies to everyone working in the repository and can be checked into version control.

Agent-specific defaults can also be defined in Markdown files under `~/.config/kilo/agents/` or `.kilo/agents/`.

Examples that illustrate the grammar:

```json
// ~/.config/kilo/kilo.json — user-wide defaults
{
  "$schema": "https://app.kilo.ai/config.json",
  "permission": {
    "*": "ask",
    "bash": {
      "*": "ask",
      "git *": "allow",
      "npm *": "allow"
    },
    "read": "allow"
  }
}
```

```json
// kilo.json — repo-shared defaults
{
  "$schema": "https://app.kilo.ai/config.json",
  "permission": {
    "edit": "ask",
    "bash": {
      "*": "ask",
      "npm test": "allow"
    },
    "external_directory": {
      "~/shared/**": "allow"
    }
  }
}
```

```json
// Agent config in kilo.json
{
  "$schema": "https://app.kilo.ai/config.json",
  "agent": {
    "review": {
      "mode": "subagent",
      "description": "Read-only code reviewer",
      "permission": {
        "edit": "deny",
        "write": "deny",
        "bash": "deny"
      }
    }
  }
}
```

### Extending the Base

Default permissions can be set at user scope and then narrowed or extended by narrower scopes or CLI flags.

**Example 1: user allows, repo denies.**

User `~/.config/kilo/kilo.json`:

```json
{
  "permission": {
    "bash": {
      "rm *": "allow"
    }
  }
}
```

Repo `kilo.json`:

```json
{
  "permission": {
    "bash": {
      "rm *": "deny"
    }
  }
}
```

Result: `rm` is blocked in the repository because the later project config overrides the earlier global config.

**Example 2: repo default ask, CLI auto-approve override.**

Repo `kilo.json`:

```json
{
  "permission": {
    "edit": "ask",
    "bash": "ask"
  }
}
```

CLI:

```bash
kilo run --auto "apply the suggested refactor"
```

Result: the non-interactive run auto-approves non-denied edit and bash requests for this session.

**Example 3: global whitelist plus project additions.**

Global config:

```json
{
  "permission": {
    "*": "deny",
    "read": "allow",
    "grep": "allow"
  }
}
```

Repo `kilo.json`:

```json
{
  "permission": {
    "bash": {
      "npm test": "allow"
    }
  }
}
```

Result: in this repo, read, grep, and `npm test` are allowed; everything else is denied.

## Tools and Permissions

Kilo Code provides the following built-in tools. Each tool is gated by a permission key. Some keys cover multiple tools.

| Tool | Permission key | Permission required by default |
| :----- | :----- | :----- |
| `bash` | `bash` | Allow |
| `edit` | `edit` | Allow |
| `write` | `edit` (covers all file modifications) | Allow |
| `apply_patch` | `edit` (covers all file modifications) | Allow |
| `read` | `read` | Allow, except `.env` files are denied |
| `grep` | `grep` | Allow |
| `glob` | `glob` | Allow |
| `list` | `list` | Allow |
| `lsp` | `lsp` | Allow (requires experimental flag) |
| `skill` | `skill` | Allow |
| `todowrite` / `todoread` | `todowrite` | Allow |
| `webfetch` | `webfetch` | Allow |
| `websearch` | `websearch` | Allow |
| `question` | `question` | Allow |
| `task` (subagent spawn) | `task` | Allow |

Additional Kilo-specific permission keys defined in the config schema include `agent_manager`, `repo_clone`, `repo_overview`, and notebook tools (`notebook_read`, `notebook_edit`, `notebook_execute`). Their availability depends on feature flags and client surface.

Permission rules match the tool input. For example, `bash` rules match parsed command strings, `read`/`edit` rules match file paths, and `webfetch` rules match URLs. Wildcards follow simple glob semantics: `*` matches zero or more characters, `?` matches exactly one character, and all other characters match literally. Rules are evaluated in order and the **last matching rule wins**, so a common pattern is to place `"*": "ask"` first and more specific allow/deny rules after it.

## MCP and Permissions

MCP servers add external tools that appear alongside built-in tools. Once a server is configured under the `mcp` object, its tools are registered with the server name as a prefix (for example, a server named `mymcp` exposes tools like `mymcp_search`).

Permissions interact with MCP tools in two ways:

1. **Server enablement**: A server can be enabled or disabled with `enabled: true`/`false`. A disabled server is not available.
2. **Tool-level rules**: The global `permission` object can target MCP tools by name or wildcard. For example:

```json
{
  "$schema": "https://app.kilo.ai/config.json",
  "permission": {
    "mymcp_*": "ask",
    "mymcp_write_file": "deny",
    "mymcp_read_file": "allow"
  }
}
```

The legacy `tools` object can also disable an entire MCP server or pattern:

```json
{
  "tools": {
    "mymcp_*": false
  }
}
```

To make MCP safer:

- Deny all MCP tools by default and allow only specific servers or operations.
- Disable high-risk servers globally and enable them only for specific agents.
- Use `ask` for write/delete operations while keeping read operations allowed.
- Keep the MCP server list short to reduce context size and attack surface.
- Use the experimental `policies` feature to deny untrusted LLM providers, since MCP servers may forward requests through configured providers.
