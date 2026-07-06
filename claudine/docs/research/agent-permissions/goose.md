---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-03
agent: codex
model: default

cli_params:
  - param: --with-builtin
    style: switch
    description: "Loads one or more named built-in extensions for this session, changing the visible tool surface without changing the approval mode. The value is a comma-delimited list of built-in extension names."
    example: "GOOSE_MODE=approve goose run -t \"review this code\" --no-profile --with-builtin developer"
    example_description: "Runs with default profile extensions skipped and only the developer built-in extension added, while approval behavior comes from GOOSE_MODE."
  - param: --with-extension
    style: switch
    description: "Adds a stdio extension command for this session. This can expose MCP-style tools; those tools are then governed by Goose mode and permission.yaml rules."
    example: "GOOSE_MODE=approve goose session --no-profile --with-extension \"uvx mcp-server-git\""
    example_description: "Starts an interactive session with user/profile extensions skipped and one explicit stdio extension."
  - param: --with-streamable-http-extension
    style: switch
    description: "Adds a streamable HTTP extension for this session. The value is a URL, optionally followed by key/value options such as timeout=100."
    example: "GOOSE_MODE=approve goose session --no-profile --with-streamable-http-extension \"http://127.0.0.1:3000/mcp timeout=100\""
    example_description: "Adds one HTTP MCP-style extension to an approve-mode session."
  - param: --no-profile
    style: switch
    description: "Skips the user's default/profile extensions, limiting the visible tools to CLI-specified extensions and built-ins."
    example: "goose session --no-profile --with-builtin developer"
    example_description: "Starts an interactive session without profile extensions and adds only the developer built-in extension."
  - param: --container
    style: switch
    description: "Runs stdio and built-in extension processes inside the specified Docker container. This is extension process isolation, not a Goose approval-mode flag."
    example: "goose session --container goose-sandbox"
    example_description: "Starts a session whose extension processes run in the named Docker container when the required extension binaries exist there."
  - param: --max-tool-repetitions
    style: switch
    description: "Sets the maximum number of consecutive identical tool calls with identical parameters. It is a loop-control guard adjacent to permissions, not an allow/deny rule."
    example: "goose run -t \"fix tests\" --max-tool-repetitions 3"
    example_description: "Caps repeated identical tool use during a non-interactive run."
  - param: --max-turns
    style: switch
    description: "Sets the maximum number of turns allowed without user input. It limits autonomous progress but does not change which tools are approved."
    example: "goose run -t \"implement the task\" --max-turns 20"
    example_description: "Runs with a smaller autonomous turn budget."
  - param: --interactive
    style: switch
    description: "For goose run, continues into an interactive session after processing initial input. This matters because approval prompts can only be answered interactively."
    example: "GOOSE_MODE=approve goose run -t \"inspect this repo\" --interactive"
    example_description: "Lets the run continue in an interactive session where approval prompts can be answered."
  - param: --no-session
    style: switch
    description: "For goose run, executes without creating or using a saved session file. It does not isolate permissions or disable tool approvals."
    example: "GOOSE_MODE=auto goose run -t \"summarize this repo\" --no-session"
    example_description: "Runs without storing a session while retaining the effective GOOSE_MODE."
  - param: --debug
    style: switch
    description: "Shows complete tool parameters and responses without truncation. This can expose sensitive tool data in output but does not approve or deny tools."
    example: "goose session --debug"
    example_description: "Starts an interactive session with full debug visibility."

env_vars:
  - name: GOOSE_MODE
    effect: "Controls the session mode: auto, approve, smart_approve, or chat. It overrides config.yaml for the effective mode."
    effect_category: approval_mode
  - name: GOOSE_ALLOWLIST
    effect: "URL to a YAML allowlist of extension installation commands. When set, Goose allows only extension install commands that exactly match allowlist entries."
    effect_category: policy_overlay
  - name: GOOSE_SANDBOX
    effect: "Enables the optional Goose Desktop macOS sandbox when set to true or 1. This is documented for Desktop, not ordinary CLI launch."
    effect_category: sandbox_control
  - name: GOOSE_PATH_ROOT
    effect: "Overrides the base directory for Goose config, data, state, plugin, and agent paths. Useful for isolated tests or wrappers."
    effect_category: state_home_relocation
  - name: GOOSE_ADDITIONAL_CONFIG_FILES
    effect: "Colon-separated additional YAML config files loaded after system config and before user config; can inject organization or wrapper defaults."
    effect_category: config_injection
  - name: GOOSE_DISABLE_KEYRING
    effect: "Disables native keyring secret storage; secrets fall back to plaintext secrets.yaml, changing the security posture of extension credentials."
    effect_category: security_hardening
  - name: GOOSE_DEBUG
    effect: "Enables debug output with full tool parameters and responses; similar to the --debug CLI switch."
    effect_category: none
  - name: SECURITY_PROMPT_ENABLED
    effect: "Enables prompt-injection detection that can identify harmful tool requests."
    effect_category: threat_detection
  - name: SECURITY_PROMPT_THRESHOLD
    effect: "Sets the prompt-injection detection threshold from 0.01 to 1.0."
    effect_category: threat_detection
  - name: SECURITY_PROMPT_CLASSIFIER_ENABLED
    effect: "Enables an external classifier for prompt-injection detection."
    effect_category: threat_detection
  - name: SECURITY_PROMPT_CLASSIFIER_ENDPOINT
    effect: "Sets the endpoint URL for the prompt-injection classifier."
    effect_category: threat_detection
  - name: SECURITY_PROMPT_CLASSIFIER_TOKEN
    effect: "Sets the authentication token for the prompt-injection classifier endpoint."
    effect_category: threat_detection
  - name: GOOSE_SANDBOX_PROTECT_FILES
    effect: "Controls the protected-file list used by the optional Desktop macOS sandbox."
    effect_category: sandbox_control
  - name: GOOSE_SANDBOX_ALLOW_IP
    effect: "Adds IP exceptions to the optional Desktop macOS sandbox network controls."
    effect_category: sandbox_control
  - name: GOOSE_SANDBOX_ALLOW_SSH
    effect: "Allows SSH exceptions in the optional Desktop macOS sandbox network controls."
    effect_category: sandbox_control
  - name: GOOSE_SANDBOX_GIT_HOSTS
    effect: "Configures git host SSH exceptions for the optional Desktop macOS sandbox."
    effect_category: sandbox_control

config_files:
  - os: macos
    user: "Library/Application Support/Block/goose/config.yaml"
    repo: ""
    notes: "Source resolves the config directory through etcetera with Block/goose app metadata; legacy/docs examples often show ~/.config/goose. permission.yaml lives next to config.yaml. No local Goose config existed on this host to inspect."
  - os: linux
    user: ".config/goose/config.yaml"
    repo: ""
    notes: "permission.yaml lives at ~/.config/goose/permission.yaml. A system config may exist at /etc/goose/config.yaml. No built-in repo-scoped permission file is documented."
  - os: windows
    user: "AppData\\Roaming\\Block\\goose\\config\\config.yaml"
    repo: ""
    notes: "permission.yaml lives next to config.yaml under the Goose config directory. System config is documented under ProgramData. No built-in repo-scoped permission file is documented."

precedence:
  - source: environment variables
    scope: [approval_mode, sandbox, mcp, tool_visibility, security_controls]
    merge_strategy: none
    notes: "GOOSE_MODE overrides config mode. GOOSE_PATH_ROOT can move all config/state paths for a run. GOOSE_ALLOWLIST and sandbox/security env vars are runtime controls."
  - source: cli
    scope: [tool_visibility, sandbox, other]
    merge_strategy: none
    notes: "Goose has no CLI approval-mode flag. CLI switches can skip profile extensions, add explicit extensions, run extensions in Docker, control headless/session behavior, and cap repetition/turns."
  - source: user_config
    scope: [approval_mode, mcp, tool_visibility, security_controls]
    merge_strategy: none
    notes: "User config values are loaded after system and additional config files and replace earlier values by key."
  - source: additional_config_files
    scope: [approval_mode, mcp, tool_visibility, security_controls]
    merge_strategy: none
    notes: "GOOSE_ADDITIONAL_CONFIG_FILES injects extra YAML files after system config and before user config; user config can still override those keys."
  - source: system_config
    scope: [approval_mode, mcp, tool_visibility, security_controls]
    merge_strategy: none
    notes: "System config provides a low-precedence baseline, not a managed lock."
  - source: permission.yaml
    scope: [rules, approval_mode, mcp]
    merge_strategy: none
    notes: "Per-tool levels are read from one user-scoped permission.yaml and are consulted only in approve and smart_approve modes. SmartApprove also mutates this file for cached classifications."
  - source: built-in defaults
    scope: [approval_mode, tool_visibility]
    merge_strategy: none
    notes: "Source declares GooseMode::Auto as the default and enables several platform extensions by default."

default_posture: "With no config, env, or CLI guidance, Goose defaults to auto mode, so visible tools are auto-approved. The default visible tool surface includes default-enabled built-in extensions such as developer, analyze, todo, apps, extensionmanager, summon, top-of-mind, and skills."

cli_zero_permissions:
  supported: true
  invocation: "GOOSE_MODE=chat goose session --no-profile"
  mechanism: "Uses the session-scoped GOOSE_MODE=chat environment override to disable tool calls and --no-profile to skip profile extensions."
  limitations: "This is not CLI-only in the strict flag-only sense because Goose has no --mode, --tools, --allowed-tools, or --disallowed-tools flag. It can start one session with no tool execution, but Goose cannot add back selected permissions by CLI in the same run except by exposing selected extensions; switching to approve/auto requires GOOSE_MODE or /mode."

agent_permissions:
  allowed: false
  fm_properties: []

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "auto mode via GOOSE_MODE=auto, GOOSE_MODE: auto in config.yaml, or the interactive /mode auto command."

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - "Goose has no CLI approval-mode flag, so one-shot PolicyEngine mutations must emit environment variables rather than argv."
    - "Goose permission rules are exact tool-name lists, not path, command-pattern, domain, or structured MCP matchers."
    - "Auto mode ignores permission.yaml tool levels, which means static allow/ask/deny rules do not always describe effective behavior."
    - "SmartApprove uses tool annotations plus LLM read-only classification and writes cached decisions to permission.yaml."
    - "Tool visibility is extension/profile based and separate from approval policy."
    - "There is no built-in repo-scoped permission file."
    - "Optional Desktop sandbox and CLI --container isolation are separate from approval mode and are not currently modeled by the Goose backend."
    - "Claudine's current Goose PolicyEngine backend only discovers config paths and marks all query and mutation capabilities unsupported."

permission_entities:
  - entity: tool
    native_names: ["permission.yaml user.always_allow", "permission.yaml user.ask_before", "permission.yaml user.never_allow", "permission.yaml smart_approve.*"]
    notes: "Fine-grained decisions target exact exposed tool names."
  - entity: tool_group
    native_names: ["extensions.<name>.available_tools", "--with-builtin", "--with-extension", "--with-streamable-http-extension", "--no-profile"]
    notes: "Goose changes tool visibility by loading, skipping, or filtering extensions."
  - entity: command
    native_names: ["developer shell tool", "command-level permissions requested but not implemented"]
    notes: "Shell commands are exposed as tool calls; no current native command pattern grammar was found. A current GitHub issue requests command-level permissions."
  - entity: path
    native_names: ["Desktop sandbox protected files", "developer text/file tools"]
    notes: "Core approval rules are not path scoped. Protected paths exist only in the optional Desktop sandbox."
  - entity: workspace
    native_names: ["working directory", ".goosehints", ".goose/recipes", ".agents/agents", ".claude/agents"]
    notes: "Goose auto-discovers project context and agents; there is no folder trust prompt or workspace permission rule."
  - entity: mcp_server
    native_names: ["extensions.<name>", "GOOSE_ALLOWLIST"]
    notes: "MCP servers are modeled as extensions; install commands can be constrained by an exact-match allowlist."
  - entity: mcp_tool
    native_names: ["extension-prefixed tool names", "available_tools", "unprefixed_tools"]
    notes: "MCP tools are governed as ordinary exposed tools by name."
  - entity: subagent
    native_names: ["summon load", "summon delegate", "recipes", ".agents/agents", ".claude/agents"]
    notes: "Subagents/delegation inherit the parent session mode and visible tools; no separate subagent permission policy was found."
  - entity: mode
    native_names: ["auto", "approve", "smart_approve", "chat", "GOOSE_MODE", "/mode"]
    notes: "The coarse session mode is the primary approval-control entity."
  - entity: approval_category
    native_names: ["always_allow", "ask_before", "never_allow"]
    notes: "The three per-tool permission levels in permission.yaml."
  - entity: sandbox
    native_names: ["GOOSE_SANDBOX", "--container"]
    notes: "GOOSE_SANDBOX is Desktop macOS sandboxing; --container isolates extension processes in Docker."
  - entity: extension
    native_names: ["extensions.<name>", "--with-builtin", "--with-extension", "--with-streamable-http-extension", "--no-profile"]
    notes: "Extensions define the visible tool set and can add MCP servers."
  - entity: slash_command
    native_names: ["/mode", "/permissions", "/debug"]
    notes: "Interactive commands can switch modes, inspect/update permissions, and toggle debug output."

approval_modes:
  - name: auto
    effect: "Auto-approves all visible tool calls. permission.yaml user rules are not consulted."
    interactive: true
    non_interactive: true
    aliases: ["GOOSE_MODE=auto", "GOOSE_MODE: auto", "/mode auto", "Automatically approve tool calls"]
  - name: approve
    effect: "Checks user-defined permission.yaml levels first; unknown or ask_before tools require approval, never_allow denies, and always_allow runs."
    interactive: true
    non_interactive: true
    aliases: ["GOOSE_MODE=approve", "GOOSE_MODE: approve", "/mode approve", "Ask before every tool call"]
  - name: smart_approve
    effect: "Allows read-only annotated tools and cached read-only classifications; otherwise prompts or uses LLM read-only detection and caches the result."
    interactive: true
    non_interactive: true
    aliases: ["GOOSE_MODE=smart_approve", "GOOSE_MODE: smart_approve", "/mode smart_approve", "Ask only for sensitive tool calls"]
  - name: chat
    effect: "No tool calls are executed."
    interactive: true
    non_interactive: true
    aliases: ["GOOSE_MODE=chat", "GOOSE_MODE: chat", "/mode chat", "Chat only, no tool calls"]

rule_model:
  decisions: ["always_allow", "ask_before", "never_allow"]
  syntax: "permission.yaml is a YAML map with user and smart_approve categories; each category may contain always_allow, ask_before, and never_allow arrays of exact exposed tool names."
  precedence: "In approve and smart_approve, user-defined permission.yaml levels are checked first. If no user level exists, read-only annotations can allow; smart_approve cached always_allow can allow; extension management always asks; smart_approve unknowns may be classified by an LLM; otherwise unknown tools ask. In auto, rules are ignored. In chat, tools are skipped."
  merge_semantics: "One user-scoped permission.yaml is used. Updating a tool removes it from the other decision lists in the same category, so each tool has one level per category."
  matcher_semantics: "Exact string match against the exposed tool name. Extension tools are typically server__tool unless configured as unprefixed. No wildcard, glob, regex, path, domain, or command-argument matcher was found."
  default_decision: "auto allows; chat skips/denies tool use; approve asks for unknown tools; smart_approve asks or classifies unknown tools after read-only annotation/cache checks."

tool_visibility:
  supported: true
  mechanisms:
    - "--no-profile skips profile/default extension loading."
    - "--with-builtin adds selected built-in extensions."
    - "--with-extension adds stdio extension commands."
    - "--with-streamable-http-extension adds HTTP extensions."
    - "extensions.<name>.available_tools filters exposed tools from a configured extension."
    - "GOOSE_MODE=chat leaves tools non-executable even if visible."
  notes: "Tool visibility and approval policy are distinct. Goose does not expose a generic --tools, --allowedTools, or --disallowedTools flag."

sandbox:
  supported: true
  modes: ["desktop-mac-sandbox", "extension-container"]
  backends: ["macOS sandbox-exec/Seatbelt for Goose Desktop", "Docker container for CLI extension processes"]
  filesystem_control: "The Desktop sandbox can protect files such as ~/.ssh, shell startup files, Goose config, and sandbox config. CLI --container only moves extension processes into a Docker container; it is not a path rule for the main Goose CLI process."
  network_control: "The Desktop sandbox denies direct network access and routes traffic through an egress proxy with blocklist and exception variables. No ordinary CLI network sandbox flag was found."
  notes: "Sandboxing is separate from approval mode. GOOSE_SANDBOX is documented for Desktop launch; --container is available on session/run and requires Docker plus matching extension binaries in the container."

trust_and_admin:
  folder_trust: "No folder/project trust prompt was found. Goose discovers local .goosehints, .goose/recipes, .agents/agents, and .claude/agents from the working tree/ancestor context."
  managed_policy: "System config and GOOSE_ADDITIONAL_CONFIG_FILES can provide defaults, and GOOSE_ALLOWLIST can constrain extension installation commands. These are not documented as immutable managed policy layers; user config can override ordinary config keys."
  safe_mode: "No dedicated safe-mode flag was found. Closest session-scoped controls are GOOSE_MODE=chat, --no-profile, and --container."
  notes: "Because there is no trust gate, wrapper launches that want isolation should use GOOSE_PATH_ROOT for a temporary config root plus GOOSE_MODE and extension flags."

mcp_permissions:
  supported: true
  server_filters:
    - "GOOSE_ALLOWLIST restricts new extension installation commands by exact command match."
    - "extensions.<name>.enabled can disable configured extensions."
    - "--no-profile prevents profile-configured extensions from loading."
    - "--with-extension and --with-streamable-http-extension explicitly add servers for one launch."
  tool_filters:
    - "permission.yaml can allow, ask, or deny exposed MCP tool names."
    - "extensions.<name>.available_tools can expose only selected tools from an extension."
  trust_model: "MCP servers are extensions. Installation is constrained by allowlist or interactive approval, but there is no project-scoped MCP trust gate like a repo .mcp.json approval."
  notes: "MCP tools execute as extension code or remote tools, not as an OS-enforced per-tool sandbox. CLI --container can isolate stdio/built-in extension processes when configured."

headless_behavior: "In goose run/headless mode, auto mode runs tools without prompting. approve and smart_approve can run pre-allowed or read-only-classified tools, but any action requiring approval has no documented programmatic approval channel; use --interactive to continue into a prompt-capable session or preconfigure permission.yaml."

approval_persistence: "Explicit tool permission changes persist in permission.yaml under the user category. SmartApprove caches read-only classification results in permission.yaml under smart_approve, so approval-relevant classifications can outlive a session."

protected_paths:
  - "Goose config directory (especially config.yaml and permission.yaml) is treated by Claudine as provider config and is protected in its partial backend."
  - "~/.ssh/ under the optional Desktop macOS sandbox"
  - "~/.bashrc, ~/.zshrc, ~/.bash_profile, ~/.zprofile under the optional Desktop macOS sandbox"
  - "Goose sandbox config under the optional Desktop macOS sandbox"

security_posture: "Goose CLI permissions are primarily a client-side approval and static tool-name policy system, with SmartApprove adding dynamic LLM classification. OS-level enforcement is only present in the optional Desktop macOS sandbox and Docker extension-container mode; ordinary CLI approval is not an OS sandbox."

changes:
  - "Refreshed the research on 2026-07-03 against the current Goose docs redirect, AAIF/block Goose source, open issue tracker, and local host state."
  - "Confirmed the current source still defaults GooseMode to auto, despite a documentation inconsistency that lists smart_approve as an environment-variable default."
  - "Confirmed that no Goose binary or local Goose config directory existed on this macOS host, so CLI help and config shapes were verified from source and docs rather than local files."
  - "Corrected user config path notes to reflect the current source path resolver using Block/goose app metadata while noting legacy/docs ~/.config examples."
  - "Added --with-streamable-http-extension, --max-tool-repetitions, --max-turns, --interactive, --no-session, and --debug to the permission-adjacent CLI metadata."
  - "Updated the zero-permission assessment: a session-scoped no-tool posture is possible with GOOSE_MODE=chat plus --no-profile, but it is not flag-only and selected permissions cannot be added back by CLI approval rules."
  - "Documented current command-level rule limitations and the feature request for finer command-level permissions."
  - "Kept the conclusion that Claudine requires PolicyEngine updates because the current Goose backend remains partial and mutation/query support is unsupported."

requires_claudine_update: true
reason: "Claudine's Goose PolicyEngine backend currently only discovers Goose config files and reports them as protected paths. Accurate provider metadata and policy mutation require parsing GOOSE_MODE, permission.yaml user and smart_approve levels, extension-based tool visibility, MCP extension filters, session-scoped env overrides, and sandbox/container posture."
---

# Goose CLI Permissions and Security Controls

## Introduction to Goose CLI Permissions

Goose CLI defines permissions with a coarse session mode plus optional exact-name tool rules. The coarse modes are `auto`, `approve`, `smart_approve`, and `chat`. Current source marks `auto` as the default. The docs are inconsistent here: the configuration-file docs and `GooseMode` source indicate `auto`, while the environment-variable docs still describe `GOOSE_MODE` as defaulting to `smart_approve`.

Tool-level permissions live in `permission.yaml` and use three lists: `always_allow`, `ask_before`, and `never_allow`. These rules target the exposed tool name, such as `shell`, `text_editor`, or an extension-prefixed MCP tool name. They do not target shell command strings, filesystem paths, domains, or command arguments.

Configuration can affect permissions through:

- The user config file, which can set `GOOSE_MODE` and extension configuration.
- `permission.yaml`, which stores explicit user tool levels and SmartApprove cached classifications.
- Environment variables such as `GOOSE_MODE`, `GOOSE_ALLOWLIST`, `GOOSE_PATH_ROOT`, `GOOSE_ADDITIONAL_CONFIG_FILES`, `GOOSE_SANDBOX`, and prompt-injection detection variables.
- Interactive slash commands such as `/mode` and `/permissions`.

Goose CLI does not currently expose a `--mode`, `--yolo`, `--approval-mode`, `--tools`, `--allowed-tools`, or `--disallowed-tools` flag. Source inspection of `crates/goose-cli/src/cli.rs` found permission-adjacent flags that control tool visibility or isolation: `--with-builtin`, `--with-extension`, `--with-streamable-http-extension`, `--no-profile`, `--container`, `--interactive`, `--no-session`, `--max-tool-repetitions`, `--max-turns`, and `--debug`.

Precedence is split by surface. `GOOSE_MODE` overrides config for the approval mode. CLI extension flags affect session-local tool visibility. User config is loaded after system and additional config files, so ordinary user keys replace lower-precedence defaults. `permission.yaml` is a separate user-scoped permission store consulted only in `approve` and `smart_approve` modes.

Permission/approval policy is separate from tool visibility. Extension loading decides which tools the model can see. Mode and `permission.yaml` decide whether a visible tool call is approved, denied, or needs approval.

No local Goose config existed on this macOS host under the standard paths checked, and the `goose` binary was not installed. Observed config shapes in this update therefore come from current source and documentation, not local files.

## Permissions Use Cases

### Default

With no environment variables, config files, or CLI switches, Goose uses `auto` mode. In this mode all visible tools are auto-approved. Default-enabled platform extensions in current source include `developer`, `analyze`, `todo`, `apps`, `extensionmanager`, `summon`, `tom`, and `skills`; non-default extensions such as memory-like or computer-control features must be explicitly enabled.

A Claudine `PolicyEngine` description would set the effective approval mode to auto and treat visible tools as allowed. That broad default is conceptually expressible, but it is not currently covered by the Goose backend. The backend does not parse `GOOSE_MODE`, extension visibility, MCP tools, or `permission.yaml`.

The current `PolicyEngine` is not ergonomic for Goose because Goose rules are exact tool-name lists, while the canonical engine exposes path, command, network, MCP, agent, and runtime axes. Without changes, PolicyEngine cannot safely define the true default posture because it cannot know the loaded tool surface or the active mode.

### Whitelisting

For interactive whitelisting, use `approve` mode and grant only the tools you want in `permission.yaml`:

```bash
GOOSE_MODE=approve goose session --no-profile --with-builtin developer
```

Example `permission.yaml`:

```yaml
user:
  always_allow:
    - list_files
    - read_file
  ask_before:
    - text_editor
    - shell
  never_allow:
    - apps__delete_app
smart_approve:
  always_allow: []
  ask_before: []
  never_allow: []
```

The best session-scoped locked-down launch for a future Claudine wrapper is:

```bash
GOOSE_MODE=chat goose session --no-profile
```

That starts with no tool execution and skips profile extensions without mutating user config. The limitation is important: it is not pure CLI flags because mode control is env/config/slash-command based, and Goose cannot add back selected per-tool approvals with CLI flags in the same run. A wrapper that needs selected permissions should use a temporary `GOOSE_PATH_ROOT` with generated config/permission files, or launch `GOOSE_MODE=approve` with a generated `permission.yaml` in the temporary config root.

Additional session-scoped examples:

```bash
# Ask for all unknown tool use in an interactive session.
GOOSE_MODE=approve goose session --no-profile --with-builtin developer

# Non-interactive run with only one built-in extension visible.
GOOSE_MODE=approve goose run -t "inspect the project" --no-profile --with-builtin developer

# Add one stdio MCP server and require approval for unknown tools.
GOOSE_MODE=approve goose session --no-profile --with-extension "uvx mcp-server-git"

# Keep profile tools hidden and isolate extension processes in Docker.
GOOSE_MODE=approve goose run -t "review changes" --no-profile --with-builtin developer --container goose-sandbox
```

PolicyEngine can describe the intent as deny-by-default plus explicit allows/asks, but Goose has no native deny-all wildcard or CLI rule grammar. A complete Claudine implementation would need to write a temporary `permission.yaml`, set `GOOSE_MODE`, and control extension visibility.

### YOLO

Goose's YOLO equivalent is `auto` mode:

- `GOOSE_MODE=auto goose session`
- `GOOSE_MODE=auto goose run -t "do the task"`
- `GOOSE_MODE: auto` in config
- `/mode auto` interactively

YOLO is available in both interactive and non-interactive sessions. In `auto`, all visible tool calls are approved. `permission.yaml` user rules are ignored. OS permissions, extension behavior, prompt-injection/security inspectors, Docker extension isolation, and any Desktop sandbox restrictions can still block work.

### Root User

No current docs or inspected source path showed a root-user special case for Goose permissions. Running as root does not appear to disable `auto` mode or add approval prompts. The resulting risk is the normal OS risk: tools run with the privileges of the Goose process.

### Configuring the Default

User-scope configuration uses the Goose config directory and `permission.yaml`. Current source resolves the config directory with `etcetera` using Block/goose app metadata and supports `GOOSE_PATH_ROOT` for an alternate root. On Linux this is documented as `~/.config/goose`; on Windows it is under `%APPDATA%\Block\goose\config`; on macOS the source comments preserve `Block/goose` application-support compatibility, while docs and legacy examples may show `~/.config/goose`.

There is no built-in repo-scoped permission file. Goose does read project context such as `.goosehints`, recipes, and agent definitions, but those are not the permission-rule store.

Permission grammar:

```yaml
user:
  always_allow:
    - read_file
    - list_files
  ask_before:
    - shell
    - text_editor
  never_allow:
    - apps__delete_app
smart_approve:
  always_allow:
    - analyze
  ask_before:
    - unknown_write_tool
  never_allow: []
```

The matcher is exact exposed tool name. Updating a tool removes it from the other lists in the same category.

### Extending the Base

Because Goose lacks repo-scoped permission files, narrower overrides are usually environment or launch-surface overrides:

```yaml
# user config
GOOSE_MODE: auto
```

```bash
# one cautious session
GOOSE_MODE=approve goose session --no-profile --with-builtin developer
```

```bash
# wrapper-isolated config root
GOOSE_PATH_ROOT="$(mktemp -d)" GOOSE_MODE=chat goose run -t "summarize"
```

```bash
# system/additional config baseline, then user config can still override ordinary keys
GOOSE_ADDITIONAL_CONFIG_FILES=/opt/company/goose-baseline.yaml goose session
```

For a Claudine wrapper, the strategic pattern is a temporary `GOOSE_PATH_ROOT` plus explicit `GOOSE_MODE` and extension flags. That avoids mutating the user's Goose config and keeps Claudine's provider run isolated from ordinary Goose sessions.

## Tools and Permissions

Current source defines default-enabled platform extensions including:

| Extension | Default | Prefixing | Permission relevance |
| --- | --- | --- | --- |
| `developer` | enabled | unprefixed | File, shell, and project tools such as editing, reading, listing, and shell execution. |
| `analyze` | enabled | unprefixed | Code-structure analysis tools. |
| `todo` | enabled | prefixed | Todo-list tools. |
| `apps` | enabled | prefixed | Create/manage Goose apps; includes destructive app operations. |
| `extensionmanager` | enabled | prefixed | Extension discovery and management; current permission inspector always requires approval for the manage-extensions tool outside auto. |
| `summon` | enabled | unprefixed | Load context and delegate to subagents. |
| `tom` | enabled | prefixed/no ordinary tools | Injects top-of-mind context. |
| `skills` | enabled | unprefixed | Discovers and provides skill instructions. |

Other bundled extensions are available but not default-enabled. They can be added with `--with-builtin` or configuration.

Permissions map to tool calls in this order:

1. `chat` skips tool calls.
2. `auto` allows tool calls without consulting `permission.yaml`.
3. `approve` and `smart_approve` first check `permission.yaml` user levels.
4. If there is no user level, read-only tool annotations can allow a tool.
5. In `smart_approve`, cached read-only decisions can allow; unknown tools can be sent to LLM read-only detection and cached as `always_allow` or `ask_before`.
6. Extension-management tools are forced to require approval outside auto.
7. Unknown tools require approval.
8. Non-permission inspectors can further modify the decision.

Native permission entities include tools, tool groups/extensions, MCP servers as extensions, MCP tools as exposed tool names, approval modes, approval categories, the optional Desktop sandbox, Docker extension containers, and subagent delegation tools. Command strings and filesystem paths are not first-class permission matchers today. A GitHub feature request for finer permissions granularity asks for per-command authorization for the shell developer tool, which confirms this is not yet a native rule grammar.

Approvals persist in `permission.yaml`. SmartApprove also persists cached read-only classifications there. No separate project-level approval persistence was found.

## Sandboxing, Trust, and Administrative Controls

Goose approval modes are not an OS sandbox. Ordinary Goose CLI tools run with the permissions of the Goose process.

There are two adjacent isolation controls:

- Goose Desktop's optional macOS sandbox, enabled with `GOOSE_SANDBOX`, uses macOS `sandbox-exec`/Seatbelt plus a local egress proxy. It can block writes to sensitive paths and restrict network egress.
- Goose CLI's `--container` runs stdio and built-in extension processes inside a Docker container. It requires the extension to exist inside the container and, for built-ins, Goose to be installed there.

No folder/project trust dialog was found. Goose can auto-discover `.goosehints`, `.goose/recipes`, `.agents/agents`, and `.claude/agents` from project context. That makes wrapper-side isolation important when running in untrusted repos.

Administrative controls are baseline controls rather than immutable managed policy:

- System config can provide low-precedence defaults.
- `GOOSE_ADDITIONAL_CONFIG_FILES` can inject additional config before user config.
- `GOOSE_ALLOWLIST` can restrict extension installation commands by exact match.

Protected paths are mainly sandbox-specific. Claudine also treats Goose config paths as provider-reserved because mutating `config.yaml` or `permission.yaml` changes provider security posture.

The honest security posture is a combination: static client-side policy, advisory approval prompts, dynamic SmartApprove classification, optional Desktop OS sandboxing, and optional Docker isolation for extension processes.

## MCP and Permissions

Goose treats MCP servers as extensions. A configured or CLI-added extension exposes tools to the model. Those tools are then evaluated by the same mode and `permission.yaml` mechanism as built-in tools.

To make MCP safer:

- Use `GOOSE_MODE=approve` or `GOOSE_MODE=smart_approve` rather than `auto`.
- Use `--no-profile` and add only the MCP extensions needed for the session.
- Use `GOOSE_ALLOWLIST` to constrain extension installation commands.
- Use `extensions.<name>.available_tools` to expose only selected server tools.
- Add `never_allow` entries for high-risk exposed tool names.
- Use `--container` when stdio/built-in extension process isolation is needed and practical.

Server-level filters include `GOOSE_ALLOWLIST`, extension enablement, `--no-profile`, and explicit session extension flags. Tool-level filters include `available_tools` and `permission.yaml` exact-name decisions. No MCP resource-specific approval grammar, response-interception permission rule, or project MCP trust file was found.

MCP/extension tools do not automatically run inside a provider sandbox in ordinary CLI use. They run as extension code or remote HTTP tools unless `--container` is used for applicable extension processes.

## Non-Interactive Behavior

`goose run` uses headless execution unless `--interactive` is supplied. In headless mode, `auto` approves visible tool calls. `approve` and `smart_approve` can proceed only for tools that are pre-allowed or classified read-only; if a tool requires user approval, there is no documented programmatic approval channel comparable to a permission-prompt callback. For prompt-capable approval, use `--interactive` or preconfigure an isolated `permission.yaml`.

## Sources

- [Goose docs root requested by task](https://block.github.io/goose/)
- [Goose Permission Modes](https://goose-docs.ai/docs/guides/managing-tools/goose-permissions/)
- [Managing Tool Permissions](https://goose-docs.ai/docs/guides/managing-tools/tool-permissions/)
- [Configuration Files](https://goose-docs.ai/docs/guides/config-files/)
- [Environment Variables](https://goose-docs.ai/docs/guides/environment-variables/)
- [Extension Allowlist](https://goose-docs.ai/docs/guides/allowlist/)
- [Goose CLI Commands](https://goose-docs.ai/docs/guides/goose-cli-commands/)
- [Running Tasks](https://goose-docs.ai/docs/guides/running-tasks/)
- [Subagents](https://goose-docs.ai/docs/guides/context-engineering/subagents/)
- [macOS Sandbox for Goose Desktop](https://goose-docs.ai/docs/guides/sandbox/)
- [Goose source repository requested by task](https://github.com/block/goose)
- [Current Goose source mirror: `GooseMode`](https://github.com/aaif-goose/goose/blob/main/crates/goose-provider-types/src/goose_mode.rs)
- [Current Goose source: `permission.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/config/permission.rs)
- [Current Goose source: `permission_inspector.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/permission/permission_inspector.rs)
- [Current Goose source: `cli.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/cli.rs)
- [Current Goose source: `paths.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/config/paths.rs)
- [Current Goose source: platform extensions](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/agents/platform_extensions/mod.rs)
- [GitHub issue: finer command-level permission granularity](https://github.com/block/goose/issues/2659)
- [Claudine Goose PolicyEngine backend](../../../../lib/src/permissions/providers/goose.rs)

## Changelog

- 2026-07-03: Refreshed the merged permissions/security-control research against current Goose docs, current Goose source, local host state, and the issue tracker. Updated frontmatter to schema contract values for this run, added source-derived CLI security-adjacent switches, clarified that no local Goose config or binary was available to inspect, documented command-level rule absence, corrected config path caveats, and retained the conclusion that Claudine needs Goose PolicyEngine backend updates.
- 2026-07-02: Refreshed research against current Goose documentation and source code. Corrected default mode to auto (source is authoritative over docs inconsistency). Documented absence of CLI permission flags. Added full schema-required frontmatter fields. Expanded coverage of sandboxing, MCP permissions, subagent behavior, non-interactive behavior, protected paths, and administrative controls. Updated sources to current goose-docs.ai and aaif-goose/goose locations.
