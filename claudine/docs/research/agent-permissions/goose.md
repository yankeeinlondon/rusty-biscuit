---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7

cli_params:
  - param: --with-builtin
    style: switch
    description: "Loads only the named built-in extensions for the session, changing which built-in tools are visible to the model. This narrows the tool surface but does not change the approval mode."
    example: "GOOSE_MODE=approve goose run -t \"review this code\" --no-profile --with-builtin developer"
    example_description: "Runs with user/profile extensions disabled and only the developer built-in extension visible, while GOOSE_MODE=approve controls approval behavior."
  - param: --with-extension
    style: switch
    description: "Loads an explicitly specified stdio extension for the session. This can add MCP-style tools to the visible tool surface; those tools are then governed by Goose's mode and permission.yaml rules."
    example: "goose session --no-profile --with-extension \"uvx mcp-server-git\""
    example_description: "Starts a session without profile extensions and adds one explicit extension command."
  - param: --no-profile
    style: switch
    description: "Skips the user's default/profile extensions, preventing those extension tools from being exposed to the model for the session."
    example: "goose session --no-profile --with-builtin developer"
    example_description: "Starts an interactive session with the default profile skipped and only the developer built-in extension loaded."
  - param: --container
    style: switch
    description: "Runs stdio and built-in extensions inside a specified Docker container. This is isolation for extension processes, not a Goose approval-mode flag."
    example: "goose session --container goose-sandbox"
    example_description: "Starts a session whose extension processes run inside the named Docker container."

env_vars:
  - name: GOOSE_MODE
    effect: Controls the session tool-execution mode (auto, approve, smart_approve, chat). Highest precedence for the mode; overrides config.yaml.
  - name: GOOSE_ALLOWLIST
    effect: URL to a YAML allowlist of MCP server installation commands. When set, Goose only installs extensions whose command exactly matches an entry in the list.
  - name: GOOSE_SANDBOX
    effect: Enables the optional macOS Desktop sandbox (true or 1). Restricts file, network, and process access via sandbox-exec and a local egress proxy.
  - name: GOOSE_PATH_ROOT
    effect: Overrides the root directory for all Goose config, data, and state files. Useful for isolated test/CI environments.
  - name: GOOSE_ADDITIONAL_CONFIG_FILES
    effect: Colon-separated list of additional config YAML files to load after system config and before user config. Can inject managed or repo-scoped policy.
  - name: SECURITY_PROMPT_ENABLED
    effect: Enables prompt-injection detection to identify potentially harmful commands.
  - name: SECURITY_PROMPT_THRESHOLD
    effect: Sensitivity threshold for prompt-injection detection (0.01 to 1.0); higher is stricter.
  - name: SECURITY_PROMPT_CLASSIFIER_ENABLED
    effect: Enables ML-based prompt injection detection with an external endpoint.
  - name: SECURITY_PROMPT_CLASSIFIER_ENDPOINT
    effect: URL for the ML-based prompt-injection classifier.
  - name: SECURITY_PROMPT_CLASSIFIER_TOKEN
    effect: Authentication token for the ML-based prompt-injection classifier endpoint.
  - name: GOOSE_DISABLE_KEYRING
    effect: Disables system keyring for secret storage; secrets fall back to plaintext secrets.yaml.
  - name: GOOSE_DEBUG
    effect: Enables debug mode showing full tool parameters and responses; also toggled with /r in-session.

config_files:
  - os: macos
    user: ~/.config/goose/config.yaml
    repo: ""
    notes: No built-in repo-scoped config file exists for Goose. permission.yaml lives at ~/.config/goose/permission.yaml and tool_permissions.json at ~/.config/goose/permissions/tool_permissions.json.
  - os: linux
    user: ~/.config/goose/config.yaml
    repo: ""
    notes: Same path layout as macOS. System config can also exist at /etc/goose/config.yaml.
  - os: windows
    user: "%APPDATA%\\Block\\goose\\config\\config.yaml"
    repo: ""
    notes: permission.yaml lives at %APPDATA%\Block\goose\config\permission.yaml. No built-in repo-scoped config file.

precedence:
  - source: environment variables
    scope: [approval_mode, extensions, security, sandbox]
    merge_strategy: none
    notes: Env vars take precedence over config files for keys like GOOSE_MODE, GOOSE_ALLOWLIST, GOOSE_SANDBOX, and SECURITY_PROMPT_ENABLED.
  - source: user_config
    scope: [approval_mode, extensions, security, sandbox]
    merge_strategy: none
    notes: ~/.config/goose/config.yaml (or Windows equivalent). Per-key replacement of system defaults.
  - source: system_config
    scope: [approval_mode, extensions, security]
    merge_strategy: none
    notes: /etc/goose/config.yaml on Unix, %PROGRAMDATA%\goose\config.yaml on Windows. Lowest precedence before built-in defaults.
  - source: built-in default
    scope: [approval_mode, extensions]
    merge_strategy: none
    notes: GOOSE_MODE defaults to auto; default platform extensions (developer, analyze, summon, etc.) are enabled.

default_posture: "When nothing is configured, Goose CLI starts in auto mode and auto-approves all enabled tool calls, including file writes, shell commands, and MCP tool calls. Safety inspectors (prompt-injection detection, adversary mode) may still block specific calls, but the default posture is permissive."

cli_zero_permissions:
  supported: false
  invocation: ""
  mechanism: "Goose has no dedicated CLI flag to set the permission mode or to start with no tools. The closest session-scoped lockdown is GOOSE_MODE=chat goose session, which uses an environment variable (not a CLI flag) to disable all tool use."
  limitations: "There is no --mode, --yolo, --tools, --allowedTools, or --disallowedTools CLI flag. Starting in approve mode still requires pre-allowed tools or an interactive terminal, and permissions cannot be added back via CLI flags in the same run."

agent_permissions:
  allowed: false

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "GOOSE_MODE=auto environment variable, GOOSE_MODE: auto in config.yaml, or the interactive /mode auto slash command."

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - "Goose has no CLI permission-mode flag, so PolicyEngine cannot model or emit a CLI override for the mode."
    - "SmartApprove delegates read-only classification to an LLM and caches the result in permission.yaml; PolicyEngine cannot predict this dynamic classification."
    - "Auto mode ignores user tool permission levels, which conflicts with PolicyEngine's assumption that tool-level allow/ask/deny rules are always respected."
    - "Tool permission rules are per tool name (with optional extension prefix), not filesystem path, command pattern, or domain, so PolicyEngine's filesystem/command/network axes do not map directly."
    - "Goose has no repo-scoped permission config file; PolicyEngine cannot target RepoConfig for tool permissions."
    - "The runtime permission cache (tool_permissions.json) is auto-managed and not expressible as static policy."
    - "Claudine's current Goose PolicyEngine backend is partial and declares no query or mutation capabilities."

permission_entities:
  - entity: tool
    native_names: ["permission.yaml user.always_allow", "permission.yaml user.ask_before", "permission.yaml user.never_allow", "permission.yaml smart_approve.*"]
    notes: "Fine-grained control is by exact tool name (or extension-prefixed name). Three decision levels: always_allow, ask_before, never_allow."
  - entity: tool_group
    native_names: ["extensions.<name>.available_tools"]
    notes: "Available tools can be limited per extension, which reduces the tool surface but is not an approval rule."
  - entity: mcp_server
    native_names: ["GOOSE_ALLOWLIST", "extensions.<name>.enabled", "extensions.<name>.available_tools"]
    notes: "Extension installation can be restricted by allowlist; individual extensions can be enabled/disabled."
  - entity: mcp_tool
    native_names: ["permission.yaml user.*", "permission.yaml smart_approve.*"]
    notes: "MCP tools are exposed with an extension prefix by default (or unprefixed if configured) and are governed like built-in tools."
  - entity: mode
    native_names: ["GOOSE_MODE", "/mode auto", "/mode approve", "/mode smart_approve", "/mode chat"]
    notes: "The session-wide mode is the primary coarse control; tool-level rules are only consulted in approve and smart_approve modes."
  - entity: approval_category
    native_names: ["always_allow", "ask_before", "never_allow"]
    notes: "The three decision values in permission.yaml. Within a category, a tool can appear in only one list; last write wins."
  - entity: sandbox
    native_names: ["GOOSE_SANDBOX"]
    notes: "Optional macOS Desktop sandbox via sandbox-exec; CLI has no sandbox flag except --container for Docker isolation of extensions."
  - entity: subagent
    native_names: ["delegate", "load", "recipes"]
    notes: "Subagents inherit extensions and mode from the parent; no separate subagent permission mode or scoped rules."
  - entity: extension
    native_names: ["extensions.<name>", "--with-builtin", "--with-extension", "--no-profile"]
    notes: "Extensions add tools; visibility is controlled by which extensions are enabled, not by approval policy."

approval_modes:
  - name: auto
    effect: "All tool calls are auto-approved. permission.yaml rules are ignored."
    interactive: true
    non_interactive: true
    aliases: ["auto", "Completely Autonomous", "/mode auto"]
  - name: approve
    effect: "Every tool call prompts for approval unless the tool is marked always_allow in permission.yaml."
    interactive: true
    non_interactive: true
    aliases: ["approve", "Manual Approval", "/mode approve"]
  - name: smart_approve
    effect: "Read-only or previously cached read-only tools run without approval; other tools prompt. Uses LLM classification and caches results."
    interactive: true
    non_interactive: true
    aliases: ["smart_approve", "Smart Approval", "/mode smart_approve"]
  - name: chat
    effect: "No tools are used; the session is chat-only."
    interactive: true
    non_interactive: true
    aliases: ["chat", "Chat Only", "/mode chat"]

rule_model:
  decisions: ["always_allow", "ask_before", "never_allow"]
  syntax: "permission.yaml groups tool names under user: and smart_approve:, each with always_allow, ask_before, and never_allow string arrays. Tool names are exact (e.g. shell, developer__text_editor, github__create_issue); no parameter, path, or glob scoping."
  precedence: "In approve/smart_approve, user-defined level wins if present. Otherwise, read-only tool annotations or cached smart_approve allow can allow. The extension management tool always requires approval. Unknown tools ask."
  merge_semantics: "Only one user-scoped permission.yaml exists. Within a category, a tool can be in only one list; updating a level removes it from the other two lists. There is no repo/user/local merge."
  matcher_semantics: "Exact string match against the tool name exposed to the model. Extension-prefixed names use the configured prefix (e.g. extension__tool) or are unprefixed when available_tools/unprefixed_tools is set. No wildcards, globs, or regex."
  default_decision: "auto allows all; approve and smart_approve ask for unknown tools; chat denies/skips all tools."

tool_visibility:
  supported: true
  mechanisms:
    - "--with-builtin <name> loads only the named built-in extensions."
    - "--with-extension <command> loads only the explicitly specified stdio extensions."
    - "--no-profile skips the user's default/profile extensions, leaving only CLI-specified extensions."
    - "extensions.<name>.available_tools in config.yaml limits which tools from an extension are exposed."
    - "GOOSE_MODE=chat disables all tools for the session."
  notes: "There is no flag to hide individual built-in tools. Tool visibility is controlled by which extensions are loaded and available_tools filters, separate from the approval mode."

sandbox:
  supported: true
  modes: ["desktop-mac-sandbox"]
  backends: ["macOS sandbox-exec (Seatbelt)"]
  filesystem_control: "Optional seatbelt profile blocks writes to ~/.ssh/, shell config files, and ~/.config/goose/ (including config.yaml and sandbox config). GOOSE_SANDBOX_PROTECT_FILES controls the default protected-file list."
  network_control: "All direct network access is denied; traffic is forced through a local egress proxy. Domain blocklist in ~/.config/goose/sandbox/blocked.txt; GOOSE_SANDBOX_ALLOW_IP, GOOSE_SANDBOX_ALLOW_SSH, and GOOSE_SANDBOX_GIT_HOSTS configure exceptions."
  notes: "Sandbox is Desktop-only and enabled via GOOSE_SANDBOX=true open -a Goose. CLI has no built-in sandbox; --container runs stdio and built-in extensions inside a Docker container instead."

trust_and_admin:
  folder_trust: "Goose does not gate project folders with a trust dialog. It auto-discovers .goosehints, .goose/recipes, .agents/agents, and .claude/agents in the working directory and ancestor paths."
  managed_policy: "Administrative baseline policy can be delivered via /etc/goose/config.yaml (Unix) or %PROGRAMDATA%\\goose\\config.yaml (Windows) and via GOOSE_ADDITIONAL_CONFIG_FILES. User config replaces values by key; there is no managed-only lock."
  safe_mode: "No safe-mode flag exists. The closest equivalents are --no-profile (skip default extensions) and GOOSE_MODE=chat (disable all tools)."
  notes: "GOOSE_ALLOWLIST is an env-only administrative control that restricts which extension installation commands are permitted. It does not restrict already-installed extensions."

mcp_permissions:
  supported: true
  server_filters:
    - "GOOSE_ALLOWLIST URL enforces exact command-match for new extension installation."
    - "extensions.<name>.enabled in config.yaml controls whether a configured extension loads."
    - "--no-profile prevents auto-loading user/profile extensions."
  tool_filters:
    - "permission.yaml tool-level always_allow/ask_before/never_allow rules apply to MCP tools by their exposed name."
    - "extensions.<name>.available_tools can expose only a subset of an MCP server's tools."
  trust_model: "Extension installation is approved interactively or constrained by GOOSE_ALLOWLIST. OAuth tokens are stored per user. There is no project-scoped .mcp.json trust gate."
  notes: "MCP tools run in-process or as stdio subprocesses; they are not isolated by the macOS Desktop sandbox. --container can place stdio and built-in extension processes in Docker."

headless_behavior: "In non-interactive goose run, auto mode auto-approves every tool call. approve and smart_approve modes fail immediately when a tool that is not pre-allowed or read-only annotated is encountered; there is no programmatic approval channel. Pre-approve needed tools in permission.yaml or use auto mode for unattended execution."

approval_persistence: "User tool permission levels persist in ~/.config/goose/permission.yaml. SmartApprove caches LLM read-only classifications under the smart_approve category of the same file. Runtime approval decisions may also be cached in ~/.config/goose/permissions/tool_permissions.json."

protected_paths:
  - "~/.ssh/ (blocked from write in macOS Desktop sandbox)"
  - "~/.bashrc, ~/.zshrc, ~/.bash_profile, ~/.zprofile (blocked from write in macOS Desktop sandbox)"
  - "~/.config/goose/config.yaml (blocked from write in macOS Desktop sandbox)"
  - "~/.config/goose/sandbox/ (blocked from write in macOS Desktop sandbox)"

security_posture: "Goose's permission system is primarily a client-side static policy engine with advisory approval prompts. The optional macOS Desktop sandbox adds OS-level filesystem and network isolation via sandbox-exec. Default CLI execution relies on OS permissions and user approval, not an enforced sandbox."

changes:
  - "Corrected the default GOOSE_MODE: source code confirms auto is the default, while the published environment-variables documentation incorrectly lists smart_approve as the default."
  - "Documented that Goose has no CLI permission-mode flag and no CLI allow/deny tool flags; all permission-mode control is via GOOSE_MODE env/config."
  - "Added full schema-required fields: cli_zero_permissions, permission_entities, approval_modes, rule_model, tool_visibility, sandbox, trust_and_admin, mcp_permissions, headless_behavior, approval_persistence, protected_paths, security_posture."
  - "Expanded sandbox coverage: macOS Desktop-only sandbox-exec sandbox, egress proxy, file/network controls, and Docker --container for CLI extensions."
  - "Updated MCP and extension permission coverage: GOOSE_ALLOWLIST exact-match semantics, available_tools filtering, extension enablement, and sandbox bypass notes."
  - "Added subagent permission behavior: inherits parent mode/extensions, no separate scoped rules, and autonomous subagents are disabled in approve/smart_approve/chat modes."
  - "Clarified config file paths and the absence of a built-in repo-scoped config file."
  - "Updated sources to current goose-docs.ai URLs and current aaif-goose/goose source paths."
  - "Surgically corrected cli_params after fleet completion: Goose has no CLI approval-mode flag, but it does expose tool-surface and extension-isolation flags that affect permissions-adjacent behavior."

requires_claudine_update: true
reason: "Claudine's Goose PolicyEngine backend currently marks all capabilities false and only reports protected config paths. To support Goose permissions properly, the backend needs to parse GOOSE_MODE, model permission.yaml tool levels, handle the SmartApprove LLM cache, understand extension-based tool visibility, and plan mutations."
---

# Goose CLI Permissions

## Introduction to Goose CLI Permissions

Goose CLI controls what an agent can do through two mechanisms:

1. A session-wide **permission mode** (`auto`, `approve`, `smart_approve`, or `chat`).
2. Optional per-tool **permission levels** (`always_allow`, `ask_before`, `never_allow`) stored in `permission.yaml`.

Permissions can be defined through:

- **Configuration files**: `~/.config/goose/config.yaml` holds `GOOSE_MODE`; `~/.config/goose/permission.yaml` holds per-tool levels; `~/.config/goose/permissions/tool_permissions.json` is an auto-managed runtime cache of SmartApprove/approval decisions.
- **Environment variables**: `GOOSE_MODE`, `GOOSE_ALLOWLIST`, `GOOSE_SANDBOX`, and security-related variables.
- **Interactive controls**: the `/mode` slash command inside a session.

Goose CLI does **not** expose a launch flag such as `--mode` or `--yolo` for permission modes. The only CLI levers that affect the permission surface are extension flags (`--with-builtin`, `--with-extension`, `--no-profile`, etc.), which change which tools are available for policy evaluation.

### Permission modes

| Mode | Behavior |
| :--- | :--- |
| `auto` | All tool calls are auto-approved. This is the default. |
| `approve` | Every tool call prompts for approval unless the tool is marked `always_allow` in `permission.yaml`. |
| `smart_approve` | Read-only or previously cached read-only tools run without approval; other tools prompt. |
| `chat` | No tools are used; the session is chat-only. |

### Configuration precedence

| Source | Effect |
| :--- | :--- |
| `GOOSE_MODE` environment variable | Highest precedence for the mode. |
| `GOOSE_MODE` in `~/.config/goose/config.yaml` | Overrides the built-in default. |
| Built-in default (`auto`) | Lowest precedence. |

Per-tool levels in `permission.yaml` are only consulted in `approve` and `smart_approve` modes. In `auto` mode they are ignored.

### Permission policy vs tool visibility

Goose separates **which tools are visible to the model** from **which visible tools are pre-approved**:

- **Approval policy** (`GOOSE_MODE`, `permission.yaml` levels) decides whether a tool call runs and whether it prompts.
- **Tool visibility** (loaded extensions, `extensions.<name>.available_tools`, `--with-builtin`, `--with-extension`, `--no-profile`) decides which tools appear in the model's context. A tool from an unloaded extension is not visible, but visibility does not imply approval.

## Permissions Use Cases

### Default

If no environment variable, config file, or CLI switch configures permissions, Goose CLI starts in `auto` mode. All enabled tools run without prompting, including file writes, shell commands, and MCP tool calls. Safety inspectors (prompt-injection detection, adversary mode, egress filtering) may still block specific calls, but the default posture is permissive.

A PolicyEngine description of the default would be:

- `SetApprovalMode(Auto)`.
- All `can_use_tool`, `can_execute`, and `can_write` queries return `Allow`.
- No static tool-level rules are configured.

This is expressible in PolicyEngine, but the description is incomplete because it cannot capture the dynamic SmartApprove cache or the safety-inspector overrides that may still block calls.

### Whitelisting

Goose does not have a single "deny everything except allowlist" mode, but you can approximate whitelisting interactively by setting the mode to `approve` and only granting `always_allow` to the tools you need.

To start with no permissions and require every needed permission to be asked for or explicitly declared:

1. Set the session mode to `approve`:

   ```bash
   GOOSE_MODE=approve goose session
   ```

   In this mode every tool call prompts until it is added to `always_allow`.

2. Or pre-declare allowed tools in `~/.config/goose/permission.yaml`:

   ```yaml
   user:
     always_allow:
       - text_editor
       - list_files
       - read_file
     ask_before:
       - shell
       - write_file
     never_allow:
       - apps__delete_app
   ```

CLI examples that narrow the tool surface:

```bash
# Start an interactive session where every tool must be approved
goose session
# then inside the session: /mode approve

# Run non-interactively with only the developer extension loaded
# (still auto mode; combine with GOOSE_MODE=approve for approval mode)
GOOSE_MODE=approve goose run -t "review this code" --no-profile --with-builtin developer

# Start a chat-only session
GOOSE_MODE=chat goose session
```

Important caveat: non-interactive `approve` or `smart_approve` sessions cannot receive user approval, so they will fail when an unapproved tool is needed. True unattended whitelisting is not supported; for automation you must use `auto` mode, which then ignores tool-level rules.

PolicyEngine can describe the intent (`SetApprovalMode(Approve)` plus `Allow`/`Ask`/`Deny` rules for tool names), but it is not ergonomic because Goose rules are tool-name oriented, not path/command oriented, and the effective behavior depends on whether the session is interactive.

### YOLO

Goose's equivalent of YOLO mode is `auto` mode. A session can be put into it by:

- `GOOSE_MODE=auto` before launch.
- `GOOSE_MODE: auto` in `~/.config/goose/config.yaml`.
- The interactive slash command `/mode auto`.

Availability:

- **Interactive sessions**: yes, via `/mode auto` or pre-configured mode.
- **Non-interactive sessions**: yes, via `GOOSE_MODE=auto` or config.
- **Root/sudo**: Goose does not detect or block `auto` mode when running as root.

In `auto` mode:

- **Allowed**: all tool calls execute without user approval, including file edits, shell commands, MCP tool calls, and subagent delegation.
- **Still enforced**: safety inspectors may deny specific calls; OS-level permissions still apply.
- **Ignored**: user tool permission levels in `permission.yaml` are not consulted.

### Root User

Goose CLI does not appear to change its permission behavior when running as root or under `sudo`. There is no root-block for `auto` mode in the source or documentation. The usual filesystem and process privileges of the root user apply, but Goose itself does not add extra gates.

### Configuring the Default

Default permissions are configured at **user scope** only:

- `~/.config/goose/config.yaml` for `GOOSE_MODE`.
- `~/.config/goose/permission.yaml` for tool-level permissions.

Goose CLI does **not** provide a repo-scoped permission file. The only way to vary behavior per repository is to set `GOOSE_MODE` per shell session, use `GOOSE_ADDITIONAL_CONFIG_FILES`, or use a recipe/local config workflow.

`permission.yaml` grammar:

```yaml
user:
  always_allow:
    - text_editor
    - list_files
    - read_file
  ask_before:
    - shell
    - write_file
  never_allow:
    - apps__delete_app
smart_approve:
  always_allow:
    - text_editor
  ask_before: []
  never_allow: []
```

- Top-level keys are permission categories. `user` stores explicit choices; `smart_approve` stores cached LLM classifications.
- Each category contains three lists of tool names: `always_allow`, `ask_before`, `never_allow`.
- A tool can appear in only one list per category; updating a level removes it from the other two.

### Extending the Base

Because Goose has no repo-scoped permission file, the main ways to override defaults are environment variables and session-level extension flags.

**Example 1: user config auto, but one session in approve mode**

`~/.config/goose/config.yaml`:

```yaml
GOOSE_MODE: auto
```

CLI override:

```bash
GOOSE_MODE=approve goose session
```

Result: the session runs in `approve` mode despite the user default.

**Example 2: disable default extensions and load only a specific set**

```bash
goose session --no-profile --with-builtin developer,memory
```

Result: only the `developer` and `memory` extensions are loaded, so the agent has access to only their tools.

**Example 3: config says smart_approve, but a non-interactive run needs auto**

```bash
GOOSE_MODE=auto goose run -t "run the build script" --no-session
```

Result: `auto` mode overrides the configured `smart_approve` for that run.

## Tools and Permissions

The default Goose CLI session loads the following platform extensions (all `default_enabled: true`):

| Extension | Prefixing | Default tools |
| :--- | :--- | :--- |
| `developer` | unprefixed | `text_editor`, `shell`, `list_files`, `read_file`, etc. |
| `analyze` | unprefixed | `analyze` |
| `summon` | unprefixed | `load`, `delegate` |
| `skills` | unprefixed | skill-loading tools |
| `todo` | prefixed | `todo__todo_write` |
| `apps` | prefixed | `apps__list_apps`, `apps__create_app`, `apps__iterate_app`, `apps__delete_app` |
| `extensionmanager` | prefixed | extension management tools |
| `tom` | n/a | context injection, no tools |

Built-in MCP extensions (`memory`, `computercontroller`, `autovisualiser`, `tutorial`) are not loaded by default; they can be added with `--with-builtin <name>`.

### How permissions map to tool calls

1. The session mode is checked first.
   - `chat`: every tool is skipped.
   - `auto`: every tool is allowed; `permission.yaml` is ignored.
   - `approve` / `smart_approve`: per-tool rules are evaluated.

2. In `approve` / `smart_approve`:
   - If the tool has a user-level `always_allow` entry, it runs.
   - If it has a `never_allow` entry, it is denied.
   - If it has an `ask_before` entry, or no entry, it prompts for approval.
   - In `smart_approve`, read-only tool annotations and cached LLM classifications can promote a tool to `always_allow`.
   - `extensionmanager__manage_extensions` always requires approval.

3. Security inspectors (prompt injection, adversary mode, egress) can override the permission result and deny a call.

4. Approved or auto-approved calls may still be blocked by the OS, the sandbox, or MCP server-level restrictions.

## Sandboxing, Trust, and Administrative Controls

### Sandboxing

Goose Desktop supports an optional macOS sandbox that is separate from the permission-mode system. It provides OS-level filesystem and network isolation via `sandbox-exec` and a local egress proxy.

- **Backend**: macOS `sandbox-exec` (Seatbelt) plus a local HTTP CONNECT egress proxy.
- **Filesystem**: by default, writes are blocked to `~/.ssh/`, shell config files, and `~/.config/goose/` paths.
- **Network**: direct network access is denied; all traffic is routed through the egress proxy, which honors a domain blocklist in `~/.config/goose/sandbox/blocked.txt` and git-host SSH allowlists.
- **Process restrictions**: tunneling tools (`nc`, `netcat`, `socat`, `telnet`) and raw sockets are blocked when enabled.
- **Scope**: sandboxing is Desktop-only. The Goose CLI does not provide a sandbox flag; `--container` runs stdio and built-in extensions inside a specified Docker container instead.

### Trust and administrative controls

Goose does not implement a project-folder trust dialog. It auto-discovers context files (`.goosehints`, `.goose/recipes`, agent definitions) in the working directory and ancestor paths.

Administrative policy can be delivered via:

- System config (`/etc/goose/config.yaml` on Unix, `%PROGRAMDATA%\goose\config.yaml` on Windows).
- `GOOSE_ADDITIONAL_CONFIG_FILES` to inject extra YAML config files.
- `GOOSE_ALLOWLIST` to restrict which extension installation commands are permitted.

These sources are loaded before the user config, and user config values replace them by key. There is no managed-only lock concept.

### Protected paths

When the macOS Desktop sandbox is enabled, writes are blocked to:

- `~/.ssh/`
- `~/.bashrc`, `~/.zshrc`, `~/.bash_profile`, `~/.zprofile`
- `~/.config/goose/config.yaml`
- `~/.config/goose/sandbox/`

Outside the sandbox, the Developer extension shell tool runs with the user's normal OS privileges.

## MCP and Permissions

MCP servers are loaded into Goose as **extensions**. Once loaded, their tools are governed by the same permission system as built-in tools:

- MCP tools are exposed with an extension prefix by default (e.g., `github__create_issue`).
- If the extension is configured with `unprefixed_tools: true`, the tools keep their native names.
- The session mode determines whether those tools auto-run or require approval.
- Per-tool rules in `permission.yaml` can allow, ask, or deny individual MCP tools by their exposed name.

Making MCP safer:

- Use `GOOSE_ALLOWLIST` to restrict which MCP servers can be installed.
- Load only the extensions you need; avoid broad `--with-extension` additions.
- Use `approve` or `smart_approve` mode instead of `auto`.
- Add `never_allow` rules for high-risk MCP tools in `permission.yaml`.
- Use the `available_tools` field in extension config to limit which tools from an MCP server are exposed.
- Enable prompt-injection detection with `SECURITY_PROMPT_ENABLED=true`.

MCP tools run in-process or as stdio subprocesses; they are not isolated by the Desktop macOS sandbox. Use `--container` to run extension processes inside Docker.

## Non-Interactive Behavior

In non-interactive `goose run`:

- `auto` mode auto-approves every tool call.
- `approve` and `smart_approve` modes fail immediately when a tool that is not pre-allowed or read-only annotated is encountered.
- There is no programmatic approval channel (no equivalent to Claude Code's `--permission-prompt-tool`).
- Pre-approve needed tools in `permission.yaml` or use `auto` mode for unattended execution.

## Sources

- [Goose Permission Modes](https://goose-docs.ai/docs/guides/managing-tools/goose-permissions)
- [Managing Tool Permissions](https://goose-docs.ai/docs/guides/managing-tools/tool-permissions)
- [Configuration Files](https://goose-docs.ai/docs/guides/config-files)
- [Environment Variables](https://goose-docs.ai/docs/guides/environment-variables)
- [Extension Allowlist](https://goose-docs.ai/docs/guides/allowlist)
- [Goose CLI Commands](https://goose-docs.ai/docs/guides/goose-cli-commands)
- [Running Tasks](https://goose-docs.ai/docs/guides/running-tasks)
- [Subagents](https://goose-docs.ai/docs/guides/context-engineering/subagents)
- [macOS Sandbox for Goose Desktop](https://goose-docs.ai/docs/guides/sandbox)
- [CLI Providers](https://goose-docs.ai/docs/guides/cli-providers)
- [Goose source: `GooseMode`](https://github.com/aaif-goose/goose/blob/main/crates/goose-provider-types/src/goose_mode.rs)
- [Goose source: `permission.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/config/permission.rs)
- [Goose source: `permission_inspector.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/permission/permission_inspector.rs)
- [Goose source: `platform_extensions/mod.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/agents/platform_extensions/mod.rs)
- [Goose source: `cli.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/cli.rs)
- [Claudine Goose PolicyEngine backend](../../../../lib/src/permissions/providers/goose.rs)

## Changelog

- 2026-07-02: Refreshed research against current Goose documentation and source code. Corrected default mode to auto (source is authoritative over docs inconsistency). Documented absence of CLI permission flags. Added full schema-required frontmatter fields. Expanded coverage of sandboxing, MCP permissions, subagent behavior, non-interactive behavior, protected paths, and administrative controls. Updated sources to current goose-docs.ai and aaif-goose/goose locations.
