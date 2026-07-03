---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-01
agent: open_code
model: kimi-for-coding/k2p7

cli_params:
  - param: sandbox
    style: switch
    description: Select the sandbox policy for model-generated shell commands. Values are read-only, workspace-write, and danger-full-access.
    example: codex --sandbox workspace-write
    example_description: Allows Codex to read and write files inside the workspace and prompts for actions outside that scope.
  - param: ask-for-approval
    style: switch
    description: Control when Codex pauses for human approval before running a command. Values are untrusted, on-request, and never. Deprecated on-failure is also accepted.
    example: codex --ask-for-approval on-request
    example_description: Codex asks before executing actions that leave the sandbox or use the network, but runs workspace-safe commands automatically.
  - param: dangerously-bypass-approvals-and-sandbox
    style: switch
    description: Run every command without approvals or sandboxing. Alias --yolo. Only use inside an externally hardened environment.
    example: codex --yolo "make broad changes across the filesystem"
    example_description: Disables both the sandbox and approval prompts for the session.
  - param: add-dir
    style: switch
    description: Grant additional directories write access alongside the main workspace. Repeatable for multiple paths.
    example: codex --add-dir ../shared --add-dir ../docs
    example_description: Expands the writable workspace to include sibling directories for this session.
  - param: config
    style: switch
    description: Override configuration values at runtime. Values parse as TOML if possible; otherwise the literal string is used. Can override permission-related keys.
    example: codex -c 'default_permissions = ":read-only"'
    example_description: Sets the active permission profile for the session without editing config files.
  - param: profile
    style: switch
    description: Layer a profile config file from $CODEX_HOME on top of the base user config. Useful for saved permission presets.
    example: codex --profile readonly-quiet
    example_description: Loads ~/.codex/readonly-quiet.config.toml, which might set sandbox read-only and approval never.
  - param: ignore-user-config
    style: switch
    description: Skip loading the user's ~/.codex/config.toml for this run, useful for controlled automation environments.
    example: codex exec --ignore-user-config --sandbox read-only "summarize"
    example_description: Runs a non-interactive task without any user-level permission defaults.
  - param: ignore-rules
    style: switch
    description: Skip user and project execpolicy .rules files for this run.
    example: codex exec --ignore-rules --sandbox workspace-write "run tests"
    example_description: Runs tests without applying any command prefix rules from ~/.codex/rules/ or .codex/rules/.

env_vars:
  - name: CODEX_HOME
    effect: Sets the root directory for Codex state, including config.toml, auth, logs, sessions, skills, and standalone package metadata. Defaults to ~/.codex.
  - name: CODEX_SQLITE_HOME
    effect: Sets where SQLite-backed state is stored. The sqlite_home config option takes precedence, and relative paths resolve from the current working directory.

config_files:
  - os: all
    user: ~/.codex/config.toml
    repo: .codex/config.toml

precedence:
  - source: managed requirements > CLI flags and config overrides > project config > profile files > user config > system config > built-in defaults
    scope: [permissions]
    merge_strategy: nearest
    notes: "Previous prose summary: managed requirements > CLI flags and --config overrides > project .codex/config.toml (root to current working directory, closest wins, trusted projects only) > profile files > user ~/.codex/config.toml > system /etc/codex/config.toml > built-in defaults."

default_posture: "When nothing is configured, Codex adapts to the working directory. Trusted version-controlled folders start in an Auto posture (workspace-write sandbox with on-request approvals). Untrusted or non-version-controlled folders start in read-only sandbox with on-request approvals. Network access is off by default."

agent_permissions:
  allowed: true
  fm_properties:
    - sandbox_mode
    - default_permissions
    - approval_policy
    - mcp_servers
    - features.network_proxy

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "--dangerously-bypass-approvals-and-sandbox or --yolo CLI flag; equivalent to sandbox_mode = danger-full-access with approval_policy = never"

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - Sandbox mode (read-only/workspace-write/danger-full-access) and approval policy are orthogonal axes, not a unified allow/ask/deny rule model.
    - Permission profiles use TOML filesystem/network rules with special tokens such as :minimal, :workspace_roots, :root, :tmpdir, and :slash_tmp that do not map directly to PolicyEngine path queries.
    - The network_proxy feature and sandbox_workspace_write.network_access interplay are not represented in PolicyEngine's command/network axes.
    - execpolicy .rules files use Starlark prefix_rule definitions with prompt/allow/forbidden decisions outside PolicyEngine's static rule model.
    - Managed requirements (requirements.toml) can enforce constraints and defaults from cloud, MDM, and system sources with higher precedence than user configuration.
    - Granular approval_policy exposes per-category toggles (sandbox_approval, rules, mcp_elicitations, request_permissions, skill_approval).
    - Subagent custom agent TOML files can override sandbox_mode, default_permissions, approval_policy, and mcp_servers independently of the parent session.

changes: []

requires_claudine_update: true
reason: "Codex CLI uses a dual-layer permission model (sandbox mode + approval policy), beta TOML permission profiles with filesystem/network tokens, execpolicy Starlark rules, and managed requirements. Claudine's PolicyEngine currently models a single canonical allow/ask/deny surface. Supporting Codex accurately requires extending PolicyEngine with Codex-specific backends, precedence layers, and mutation planning for sandbox_mode, approval_policy, default_permissions, and permission profile tables."
---

# Codex CLI Permissions

## Introduction to Codex CLI Permissions

Codex CLI uses a two-layer permission model. The **sandbox mode** defines what model-generated commands can technically access: which files they can read or write, and whether they can reach the network. The **approval policy** defines when Codex must stop and ask before executing an action, such as leaving the sandbox, using the network, or running a command outside a trusted set. These layers are independent but work together.

Permissions can be defined through:

1. **Configuration files** in TOML, primarily `~/.codex/config.toml` for user defaults and `.codex/config.toml` for project-scoped overrides. Project layers load only for trusted projects.
2. **CLI flags** such as `--sandbox`, `--ask-for-approval`, and `--yolo`.
3. **In-session controls** such as `/permissions` in the interactive TUI.

Codex also supports **permission profiles** (beta), which are named policies combining filesystem rules and network rules in a single `[permissions.<name>]` table. Built-in profiles are `:read-only`, `:workspace`, and `:danger-full-access`. Permission profiles do not compose with the older `sandbox_mode` / `sandbox_workspace_write` settings; configure one system or the other.

### Sandbox modes

| Mode | Filesystem | Network | Best for |
| :--- | :--- | :--- | :--- |
| `read-only` | Read workspace and temp directories only | Off | Exploration, safe browsing, CI read tasks |
| `workspace-write` | Read and write workspace and temp directories; protected paths like `.git`, `.codex`, and `.agents` stay read-only | Off by default; enable with `sandbox_workspace_write.network_access` | Everyday coding in a trusted repo |
| `danger-full-access` | No sandbox restrictions | Unrestricted | Isolated containers, CI runners where the outer environment is the security boundary |

### Approval policies

| Policy | Behavior |
| :--- | :--- |
| `untrusted` | Only known-safe read operations run automatically; mutating or external commands require approval |
| `on-request` | Workspace-safe actions run automatically; sandbox escalations, network use, and external edits prompt |
| `never` | No approval prompts; actions that cannot proceed without approval fail or are denied |
| `granular` | Per-category toggles for `sandbox_approval`, `rules`, `mcp_elicitations`, `request_permissions`, and `skill_approval` |

Deprecated `on-failure` is still accepted but maps to `on-request` behavior.

### CLI parameters and precedence

The permission-related CLI parameters are listed in the frontmatter. In summary:

- `--sandbox <mode>` sets the session sandbox policy.
- `--ask-for-approval <policy>` sets when Codex must pause for approval.
- `--dangerously-bypass-approvals-and-sandbox` (alias `--yolo`) disables both the sandbox and approvals.
- `--add-dir <path>` expands the writable workspace.
- `-c key=value` or `--config key=value` overrides any config value, including permission profiles and feature flags.
- `--profile <name>` layers a saved preset from `~/.codex/profile-name.config.toml`.
- `--ignore-user-config` and `--ignore-rules` strip user-level config and execpolicy rules for controlled runs.

Precedence is documented in the frontmatter. The key points are:

- Managed requirements from cloud, MDM, or `/etc/codex/requirements.toml` constrain values and cannot be overridden.
- CLI flags and `--config` overrides beat all file-based config.
- Project `.codex/config.toml` overrides user config for trusted projects; the closest file to the current working directory wins when multiple project layers exist.
- Profile files layer between project config and user config.
- System `/etc/codex/config.toml` provides defaults below user config.

## Permissions Use Cases

### Default

If no environment variable, config file, or CLI switch configures permissions, Codex inspects the working directory. For trusted version-controlled folders, the effective default is `sandbox_mode = "workspace-write"` with `approval_policy = "on-request"`: Codex can read, edit, and run commands inside the workspace, but must ask before editing outside the workspace or using the network. For untrusted or non-version-controlled folders, Codex starts in `read-only` with `on-request` approvals.

A PolicyEngine description of the default posture would need to represent:

- `can_read(path)` → Allow for workspace and temp paths; Ask for paths outside the workspace.
- `can_write(path)` → Allow for workspace paths (except protected subpaths); Ask or Deny for paths outside the workspace.
- `can_execute(command)` → Allow for workspace-safe commands; Ask for commands that leave the workspace or use the network.
- `can_access_domain(domain)` → Deny by default; Ask if network access is enabled.
- `can_spawn_subagent(agent)` → Allow, but the subagent inherits the same sandbox and approval policy.

This use case is only partially ergonomic in PolicyEngine. The engine can model read/write/execute/network/agent axes, but Codex's default posture depends on VCS status and trust, and the Ask/Allow split is driven by sandbox mode rather than explicit rules. Without changes, PolicyEngine cannot express the dynamic trust-gated default or the protected-path carveouts that Codex applies inside `workspace-write`.

### Whitelisting

To start with no permissions and require every needed permission to be asked for or explicitly declared, use `read-only` sandbox with `on-request` or `untrusted` approval policy, and avoid enabling network access. In a config file:

```toml
# ~/.codex/config.toml
sandbox_mode = "read-only"
approval_policy = "untrusted"
```

With this configuration, Codex can only read files in the workspace and temp directories. Any command execution, file edit, or network request requires explicit approval in an interactive session.

CLI examples:

```bash
# Non-interactive read-only CI task; never prompts
codex exec --sandbox read-only --ask-for-approval never "summarize the repo"

# Interactive exploration that asks before any action outside the sandbox
codex --sandbox read-only --ask-for-approval untrusted "explain the auth module"

# Workspace-write for edits, but still ask before network or external commands
codex --sandbox workspace-write --ask-for-approval on-request "refactor the parser"
```

To grant additional permissions for one session, use `--add-dir`, `-c features.network_proxy.enabled=true`, or `-c sandbox_workspace_write.network_access=true`.

PolicyEngine can describe the whitelisted posture by setting `SetApprovalMode` to a restrictive equivalent and adding allow rules for the approved read surface. However, Codex does not have a single "deny all by default" mode that maps cleanly to PolicyEngine's `Deny` default. The closest equivalent is `read-only` + `untrusted`, which still allows reads without per-file prompts. PolicyEngine cannot force Codex to prompt for every read; the approval prompt behavior is a runtime UI concern tied to sandbox mode and approval policy, not a static rule.

### YOLO

In Codex CLI, YOLO mode is activated by `--dangerously-bypass-approvals-and-sandbox` or its alias `--yolo`. It is also equivalent to setting `sandbox_mode = "danger-full-access"` together with `approval_policy = "never"`.

Ways to enter YOLO mode:

- Start with `--yolo` or `--dangerously-bypass-approvals-and-sandbox`.
- Start with `--sandbox danger-full-access --ask-for-approval never`.
- Use `-c sandbox_mode=danger-full-access -c approval_policy=never`.
- Switch to full access interactively via `/permissions` if the current config allows it.

Availability:

- **Interactive sessions**: yes, when started with one of the enabling flags or when allowed by config and selected via `/permissions`.
- **Non-interactive sessions**: yes, `codex exec --yolo` works, and is the intended way to run inside an externally hardened CI container.

When in YOLO mode:

- **Allowed**: almost all tool calls execute without prompting, including file edits anywhere on the filesystem, shell commands, network requests, MCP tool calls, subagent spawns, and web search.
- **Still constrained**: managed requirements can still disallow `danger-full-access` or `approval_policy = "never"`; if so, Codex falls back to a compliant value. Destructive app or MCP tool hints may still surface approval prompts when the tool advertises destructive side effects, depending on the effective approval policy.
- **Not allowed**: it cannot override managed requirements that block full access.

### Root User

When Codex CLI is started as root or under `sudo` on macOS or Linux, the behavior depends on the sandbox mode and the platform:

- `read-only` and `workspace-write` modes work normally for root, subject to OS-level sandbox support.
- `danger-full-access` and `--yolo` are available to root; Codex does not refuse them based on uid alone. The documentation recommends running `--yolo` only inside externally hardened environments such as containers, not on a normal root shell.
- If the platform sandbox cannot be enforced (for example, inside a container that lacks `bwrap`/`seccomp` capabilities), Codex may refuse to run restricted modes and require `--sandbox danger-full-access` or `--yolo` so the outer container provides the isolation.

On native Windows, the `[windows].sandbox` setting (`elevated` or `unelevated`) affects sandbox strength, but root-like behavior is not a separate concept.

### Configuring the Default

Default permissions are configured through TOML files at several scopes:

- **User scope**: `~/.codex/config.toml` applies across all projects.
- **Repo/project scope**: `.codex/config.toml` applies to everyone working in the repository and can be checked into version control. Only loads for trusted projects.
- **Profile scope**: `~/.codex/profile-name.config.toml` applies when selected with `--profile profile-name`.
- **System scope**: `/etc/codex/config.toml` on Unix applies as a machine-wide default.
- **Managed scope**: cloud-managed or MDM-pushed `requirements.toml` and `/etc/codex/managed_config.toml` enforce defaults and constraints.

For the schema's `config_files` field, user scope is `~/.codex/config.toml` and repo scope is `.codex/config.toml`.

Examples that illustrate the grammar:

```toml
# ~/.codex/config.toml — user-wide defaults
sandbox_mode = "workspace-write"
approval_policy = "on-request"

[sandbox_workspace_write]
network_access = false
```

```toml
# .codex/config.toml — repo-shared defaults
default_permissions = "project-edit"

[permissions.project-edit]
extends = ":workspace"

[permissions.project-edit.filesystem.":workspace_roots"]
"**/*.env" = "deny"

[permissions.project-edit.network]
enabled = true

[permissions.project-edit.network.domains]
"api.openai.com" = "allow"
"*.github.com" = "allow"
```

```toml
# ~/.codex/readonly-quiet.config.toml — profile for CI
sandbox_mode = "read-only"
approval_policy = "never"
```

### Extending the Base

Default permissions can be set at user scope and then narrowed or extended by narrower scopes.

**Example 1: user allows network, repo denies it.**

User `~/.codex/config.toml`:

```toml
[sandbox_workspace_write]
network_access = true
```

Repo `.codex/config.toml`:

```toml
[sandbox_workspace_write]
network_access = false
```

Result: network access is disabled in the repository because the project config overrides the user config.

**Example 2: user default mode, CLI override.**

User `~/.codex/config.toml`:

```toml
sandbox_mode = "workspace-write"
approval_policy = "on-request"
```

CLI:

```bash
codex --sandbox read-only --ask-for-approval untrusted
```

Result: the session starts in read-only sandbox with untrusted approval policy. CLI flags override settings.

**Example 3: repo permission profile, user profile layers on top.**

Repo `.codex/config.toml`:

```toml
default_permissions = "team-default"

[permissions.team-default]
extends = ":workspace"
```

CLI:

```bash
codex --profile stricter-network -c 'default_permissions = "stricter"'
```

Result: the CLI selects the `stricter` profile (defined in `~/.codex/stricter.config.toml`) and the `-c` override sets it as the active permission profile, taking precedence over the repo default.

## Tools and Permissions

Codex CLI provides a mix of built-in tools, app/connector tools, MCP tools, and subagent tools. The following table lists the common built-in tools and how permissions apply.

| Tool | Permission boundary |
| :--- | :--- |
| `shell` / `bash` | Governed by sandbox mode and approval policy. Read-only sandbox blocks writes; workspace-write allows edits inside the workspace. Network access requires `sandbox_workspace_write.network_access` or YOLO mode. |
| File read / `read_file` | Allowed inside the sandbox read surface. Outside the sandbox, workspace-write mode asks for approval. |
| File write / `apply_patch` | Allowed inside the sandbox write surface. Read-only mode blocks writes; workspace-write protects `.git`, `.codex`, and `.agents`. |
| `web_search` | Independent of command sandbox network. Defaults to cached mode; live mode requires `--search` or `web_search = "live"`. Disabled with `web_search = "disabled"`. |
| `mcp__<server>__<tool>` | Governed by MCP server config (`enabled_tools`, `disabled_tools`, `default_tools_approval_mode`, per-tool `approval_mode`) and the active approval policy. |
| `spawn_agent` / subagent tools | Subagents inherit the parent sandbox and approval policy at spawn time, including any live runtime overrides such as `/permissions` changes or `--yolo`. Custom agent files can override `sandbox_mode`, `default_permissions`, `approval_policy`, and `mcp_servers`. |
| `request_permissions` | Surface controlled by `approval_policy.granular.request_permissions`; can ask the user to escalate sandbox or approval mode. |
| `image_generation` | Uses `gpt-image-2` and is gated by product availability and usage limits, not by sandbox mode. |

Permissions map to tool calls through the sandbox layer first, then the approval policy layer. A tool call is allowed only if the sandbox permits the underlying filesystem or network access, and then only if the approval policy does not require a prompt. `execpolicy` `.rules` files add a third layer for command prefix decisions (`allow`, `prompt`, `forbidden`) on top of the sandbox.

## MCP and Permissions

MCP servers extend Codex with external tools. Their configuration lives in the same `config.toml` layers as other settings: user `~/.codex/config.toml`, project `.codex/config.toml`, and custom agent files under `.codex/agents/`.

Permission controls for MCP:

- **Server enable/disable**: `mcp_servers.<id>.enabled = false` disables a server without removing its config.
- **Tool allowlist/denylist**: `enabled_tools` and `disabled_tools` restrict which tools from a server are exposed. `disabled_tools` applies after `enabled_tools`.
- **Default approval mode**: `mcp_servers.<id>.default_tools_approval_mode` sets `auto`, `prompt`, or `approve` for all tools on that server unless overridden.
- **Per-tool approval mode**: `mcp_servers.<id>.tools.<tool>.approval_mode` overrides the default for a single tool.
- **Destructive hints**: Tools that advertise `destructive_hint = true` always require approval when the active policy would otherwise auto-approve.
- **Managed requirements**: Admins can restrict which MCP servers users may enable by defining approved identities in `requirements.toml` based on `command` (for stdio) or `url` (for HTTP). An empty `mcp_servers` table disables all MCP servers.

To make MCP safer:

- Use project-scoped `.codex/config.toml` only in trusted projects; untrusted project layers are skipped entirely.
- Set `default_tools_approval_mode = "prompt"` or `"approve"` only for servers you trust, and leave unknown servers on `auto` or `prompt`.
- Use `enabled_tools` to expose only the tools a workflow needs; deny high-risk tools such as write or delete operations by default.
- Enable `features.network_proxy` with an explicit domain allowlist so MCP servers cannot reach arbitrary hosts even when command network access is on.
- In CI, use `codex exec --ignore-user-config --sandbox read-only --ask-for-approval never` or restrict MCP through managed requirements.
- Review plugin-bundled MCP servers under `plugins.<plugin>.mcp_servers.<server>` with the same `enabled_tools`, `disabled_tools`, and `approval_mode` controls.
