---
$schema: ./_schema.yaml
created: 2026-07-08
last_updated: 2026-07-08
agent: codex
model: default

cli_params:
  - param: sandbox
    style: equals
    description: "Session-scoped sandbox override. The installed 1.1.0 help lists --sandbox as enabling terminal restrictions, while current docs show boolean forms such as --sandbox=false; observed help accepts --sandbox=true and --sandbox=false."
    example: "agy --sandbox=true"
    example_description: "Starts the CLI with terminal sandboxing enabled for this session."
  - param: dangerously-skip-permissions
    style: switch
    description: "Auto-approves all tool permission requests without prompting. It is the Antigravity CLI YOLO equivalent and should be treated as bypassing the permission prompt layer."
    example: "agy --dangerously-skip-permissions"
    example_description: "Starts an interactive session that does not stop for tool permission confirmations."
  - param: mode
    style: switch
    description: "Sets the agent execution mode for this session. Current help documents accept-edits and plan; the changelog says 1.1.0 cycles default -> accept-edits -> plan and adds request-review as default write-review behavior."
    example: "agy --mode plan"
    example_description: "Starts the session in plan mode, reducing mutation behavior by requiring planning rather than direct edits."
  - param: add-dir
    style: switch
    description: "Adds a directory to the active workspace. Repeatable; affects the workspace boundary whose files are auto-allowed by default."
    example: "agy --add-dir ../shared --add-dir ../docs"
    example_description: "Adds sibling directories to the workspace for this session."
  - param: print
    style: switch
    description: "Runs a single prompt non-interactively and prints the response. Approval prompts cannot be answered in the normal TUI flow in this mode."
    example: "agy --print \"summarize this repository\""
    example_description: "Runs a headless one-shot prompt."
  - param: prompt
    style: switch
    description: "Alias for --print."
    example: "agy --prompt \"list the risky permission rules\""
    example_description: "Runs a headless one-shot prompt using the alias."
  - param: prompt-interactive
    style: switch
    description: "Runs an initial prompt interactively and then continues the session, so normal permission prompts can still be handled."
    example: "agy --prompt-interactive \"inspect the permission settings\""
    example_description: "Starts interactively after submitting an initial prompt."
  - param: print-timeout
    style: switch
    description: "Timeout for print mode wait; default shown by installed 1.1.0 help is 5m0s."
    example: "agy --print --print-timeout 10m \"run a read-only audit\""
    example_description: "Gives a non-interactive run a longer wait budget."
  - param: project
    style: switch
    description: "Selects the project ID for the current CLI session. Project selection affects project-scoped folders, permissions, and MCP/customization settings."
    example: "agy --project default-cli-project"
    example_description: "Runs against the named project configuration."
  - param: new-project
    style: switch
    description: "Creates a new project for the session. This can create a fresh project-scoped settings/permission context."
    example: "agy --new-project"
    example_description: "Starts in a new project context rather than reusing an existing one."
  - param: conversation
    style: switch
    description: "Resumes a previous conversation by ID. Resumed sessions may carry conversation state and project association."
    example: "agy --conversation 00000000-0000-0000-0000-000000000000"
    example_description: "Resumes a specific conversation."
  - param: continue
    style: switch
    description: "Continues the most recent conversation; alias -c. This can reuse the prior conversation and project context."
    example: "agy --continue"
    example_description: "Continues the latest conversation."
  - param: model
    style: switch
    description: "Selects the model for the current session. This is not a permission switch but affects provider behavior and should be cataloged adjacent to wrapper security posture."
    example: "agy --model \"Gemini 3.1 Pro (High)\""
    example_description: "Starts with a specific model."
  - param: log-file
    style: switch
    description: "Overrides the CLI log file path. Not a permission grant, but it changes where security-relevant logs are written."
    example: "agy --log-file /tmp/agy.log"
    example_description: "Writes CLI logs to a chosen file."

env_vars:
  - name: HOME
    effect: "Antigravity CLI stores user state under $HOME/.gemini. In this session HOME was /Users/ken/.claudine, so CLI config and logs were observed under /Users/ken/.claudine/.gemini as well as the human home /Users/ken/.gemini."
    effect_category: state_home_relocation
  - name: EDITOR
    effect: "Used when settings editor value is auto or when the external prompt editor is opened. It does not directly grant agent tool permissions."
    effect_category: none
  - name: AGY_CLI_CMD_OUTPUT_PERCENTAGE
    effect: "Current changelog documents this as a TUI command-output height customization variable. It does not change permission decisions."
    effect_category: none
  - name: AGY_CLI_HIDE_ACCOUNT_INFO
    effect: "Current changelog documents this as a TUI privacy variable to hide email and plan tier in the header. It does not change permission decisions."
    effect_category: none
  - name: AGY_CLI_DISABLE_LATEX
    effect: "Current changelog documents this as disabling LaTeX rendering globally. It does not change permission decisions."
    effect_category: none

config_files:
  - os: macos
    user: ".gemini/antigravity-cli/settings.json"
    repo: ".agents/mcp_config.json; no repo-local settings.json permission file documented"
    notes: "Global/shared settings also appear in .gemini/config/config.json and project records in .gemini/config/projects/*.json. The observed /Users/ken/.antigravity directory contained IDE/VS Code extension state, not CLI permissions. Observed CLI settings under /Users/ken/.gemini/antigravity-cli/settings.json contained telemetry, model, and trustedWorkspaces but no permissions."
  - os: linux
    user: ".gemini/antigravity-cli/settings.json"
    repo: ".agents/mcp_config.json; no repo-local settings.json permission file documented"
    notes: "Paths are relative to the user's home directory. Docs state global MCP is .gemini/config/mcp_config.json and workspace MCP is .agents/mcp_config.json; project permission storage is described as project-level settings, not as a checked-in repo permission file."
  - os: windows
    user: ".gemini\\antigravity-cli\\settings.json"
    repo: ".agents\\mcp_config.json; no repo-local settings.json permission file documented"
    notes: "Docs say Windows permission paths are normalized by stripping drive letters and converting backslashes to forward slashes before rule evaluation. Windows sandbox docs conflict: CLI sandbox page lists AppContainer, while Antigravity 2.0 permissions page says terminal sandboxing is preview on macOS/Linux and coming soon to Windows."

precedence:
  - source: cli
    scope: ["approval_mode", "sandbox", "workspace", "provider_model", "config_loading", "security_controls"]
    merge_strategy: none
    notes: "Docs state launch flags temporarily override persistent preferences for a session, and the settings UI marks active command-flag overrides. --dangerously-skip-permissions bypasses prompts for the session."
  - source: project_settings
    scope: ["rules", "sandbox", "approval_mode", "workspace", "trust", "mcp", "customization_resources", "security_controls"]
    merge_strategy: nearest
    notes: "Antigravity 2.0 docs say project settings apply within a specific project and project-level permissions take priority over global settings. Observed CLI logs apply project permission grants after CLI settings initialization."
  - source: user_config
    scope: ["rules", "approval_mode", "sandbox", "mcp", "tool_visibility", "extensions", "hooks", "skills", "slash_commands", "customization_resources", "general_config"]
    merge_strategy: shallow
    notes: "CLI settings live in ~/.gemini/antigravity-cli/settings.json; global MCP lives in ~/.gemini/config/mcp_config.json. Sparse persistence writes only non-default values."
  - source: plugin_config
    scope: ["mcp", "extensions", "hooks", "skills", "slash_commands", "customization_resources", "rules", "tool_visibility"]
    merge_strategy: shallow
    notes: "Plugins may contain skills, agents, rules, MCP servers, and hooks. JSON customization configs support include_only/exclude regex filters and inheritance."
  - source: built_in_defaults
    scope: ["rules", "approval_mode", "sandbox", "tool_visibility", "mcp", "security_controls"]
    merge_strategy: none
    notes: "Defaults include toolPermission=request-review, artifactReviewPolicy=asks-for-review, allowNonWorkspaceAccess=false, enableTerminalSandbox=false, workspace file access auto-allowed, and unconfigured command/MCP/web/non-workspace actions Ask."

default_posture: "With no explicit permission rules, Antigravity CLI defaults to toolPermission=request-review, enableTerminalSandbox=false, allowNonWorkspaceAccess=false, and no permissions list. Files inside the active workspace are auto-allowed; web actions, commands, MCP tools, and non-workspace files default to Ask."

cli_zero_permissions:
  supported: false
  invocation: "No documented CLI-only no-tools or deny-all invocation. The closest prompt-minimizing posture is agy --mode plan --sandbox=true, but it still exposes read/workspace tools and relies on default Ask for commands, MCP, web, and non-workspace files."
  mechanism: "No provider-native no-tools flag, empty tool allowlist, or inline deny-all rule flag was found in --help, subcommand help, current docs, or observed config."
  limitations: "Fine-grained deny-all requires writing settings.json permissions such as deny rules for command(*), mcp(*), read_url(*), execute_url(*), and selected file paths. There is no CLI surface to add back individual allow rules in the same run without mutating config."

agent_permissions:
  allowed: true
  fm_properties:
    - "subagent tool/permission delegation is decided by the main agent"
    - "custom subagents can be packaged under plugin agents/"

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "--dangerously-skip-permissions auto-approves all tool permission requests; settings toolPermission=always-proceed is the persistent autonomy preset. Non-interactive --print accepts --dangerously-skip-permissions according to installed help, but current issue discussions warn about sandbox bypass interactions."

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - "Antigravity has an action(target) rule grammar with Deny > Ask > Allow conflict precedence, command token regex matching, domain matching, and wildcard namespaces; PolicyEngine can approximate but not fully mutate this native syntax today."
    - "Workspace file access is auto-allowed by default and project-scoped settings are stored in provider user state rather than a repo-local permission file."
    - "Sandbox grants are derived from permissions: read_file/write_file/read_url populate filesystem and network sandbox allowlists, while unsandboxed grants bypass containment for matched commands."
    - "Tool visibility is separate for MCP disabledTools and plugin/customization loading, but built-in tools do not have a documented CLI allowlist."
    - "Subagent permissions are delegated by the main agent and surfaced through fast-path approvals rather than static per-subagent config."
    - "Current docs do not document managed/admin policy layers or trust-gating semantics sufficiently for exact effective-policy modeling."

permission_entities:
  - entity: tool
    native_names: ["read_file", "write_file", "read_url", "execute_url", "command", "unsandboxed", "mcp"]
    notes: "Permission resources use action(target). Built-in sensitive surfaces include file access, browser/web access, terminal commands, sandbox bypass, and MCP tool calls."
  - entity: command
    native_names: ["command(prefix)", "command(regex)", "command(*)"]
    notes: "Commands match by exact word/token prefix; each whitespace-separated token is evaluated as an anchored regex."
  - entity: path
    native_names: ["read_file(path)", "write_file(path)", "allowNonWorkspaceAccess", "projectResources.resources.folderUri"]
    notes: "Workspace paths are auto-allowed by default. write_file implies read_file; deny read implies deny write. Windows paths normalize before matching."
  - entity: workspace
    native_names: ["workspace roots", "projectResources", "--add-dir", "trustedWorkspaces"]
    notes: "Workspace roots define the default auto-allowed file boundary. Observed settings included trustedWorkspaces, but current docs do not fully describe trust semantics for CLI permission loading."
  - entity: mcp_server
    native_names: ["mcpServers.<server>", "disabled"]
    notes: "Global servers live in ~/.gemini/config/mcp_config.json; workspace servers live in .agents/mcp_config.json; plugin servers live in plugin mcp_config.json. disabled removes a server without deleting config."
  - entity: mcp_tool
    native_names: ["mcp(server/tool)", "mcp(server/*)", "mcp(*)", "disabledTools"]
    notes: "MCP permission rules approve or ask for tool execution; disabledTools withholds tools from the model surface."
  - entity: mcp_resource
    native_names: ["MCP resources"]
    notes: "Docs say MCP can expose resources and prompts, but the documented permission grammar targets MCP tools via mcp(server/tool); separate resource-specific permission syntax was not found."
  - entity: subagent
    native_names: ["/agents", "Subagent Detail View", "subagent.approve_fast", "subagent.jump_to_waiting"]
    notes: "Subagents can request approvals; users can approve from detail view or fast-path status alert."
  - entity: agent
    native_names: ["agents/", "main agent delegation"]
    notes: "Plugins can package agents. Docs state the main agent decides subagent tools and permissions, including MCP use and write access."
  - entity: mode
    native_names: ["default", "accept-edits", "plan", "request-review"]
    notes: "1.1.0 changelog says mode cycling is default -> accept-edits -> plan and request-review is default write-review behavior. Installed help accepts --mode accept-edits or plan."
  - entity: approval_category
    native_names: ["toolPermission", "artifactReviewPolicy", "request-review", "proceed-in-sandbox", "always-proceed", "strict"]
    notes: "toolPermission controls tool prompts; artifactReviewPolicy controls artifact/code review prompts separately."
  - entity: sandbox
    native_names: ["enableTerminalSandbox", "--sandbox", "unsandboxed(...)"]
    notes: "Sandbox mode is separate from approval mode; unsandboxed permission targets commands that bypass containment."
  - entity: hook
    native_names: ["hooks.json", "PreToolUse", "PostToolUse", "PreInvocation", "PostInvocation", "Stop"]
    notes: "Hooks can return allow, deny, ask, force_ask for PreToolUse and permissionOverrides. They are configured by customization roots/plugins."
  - entity: extension
    native_names: ["plugins", "plugin.json", "plugins.json", "include_only", "exclude"]
    notes: "Plugins bundle MCP, hooks, skills, agents, and rules. Discovery can be filtered by regex include_only/exclude."
  - entity: slash_command
    native_names: ["/permissions", "/config", "/settings", "/mcp", "/agents", "/skills", "/hooks", "/plan", "/planning"]
    notes: "Slash commands expose interactive management and mode switching surfaces; docs and changelog disagree on legacy /planning versus /plan naming."

approval_modes:
  - name: "request-review"
    effect: "Default toolPermission. Prompts for write/bash/web tools and, in 1.1.0, pauses before file write operations for line-level diff review."
    interactive: true
    non_interactive: false
    aliases: ["toolPermission=request-review", "/permissions", "request-review default behavior"]
  - name: "proceed-in-sandbox"
    effect: "Auto-approves terminal commands that run inside the secure sandbox and requests manual approval when a command attempts to bypass the sandbox."
    interactive: true
    non_interactive: true
    aliases: ["toolPermission=proceed-in-sandbox"]
  - name: "always-proceed"
    effect: "Never prompts for tool permissions except explicit deny rules; persistent autonomy preset."
    interactive: true
    non_interactive: true
    aliases: ["toolPermission=always-proceed", "/permissions always-proceed"]
  - name: "strict"
    effect: "Prompts for all non-read tools."
    interactive: true
    non_interactive: false
    aliases: ["toolPermission=strict", "/permissions strict"]
  - name: "plan"
    effect: "Session execution mode for planning. Current docs/changelog describe it as plan generation mode; installed help accepts --mode plan."
    interactive: true
    non_interactive: true
    aliases: ["--mode plan", "/plan", "/planning"]
  - name: "accept-edits"
    effect: "Session execution mode available in 1.1.0 mode cycling and --mode help; exact approval semantics beyond accepting edits were not documented in the fetched docs."
    interactive: true
    non_interactive: true
    aliases: ["--mode accept-edits"]
  - name: "dangerously-skip-permissions"
    effect: "Auto-approves all tool permission requests without prompting."
    interactive: true
    non_interactive: true
    aliases: ["--dangerously-skip-permissions", "YOLO"]

rule_model:
  decisions: ["allow", "ask", "deny", "force_ask"]
  syntax: "Persistent permission rules are strings in settings JSON under permissions.allow, permissions.ask, and permissions.deny using action(target). Supported actions are read_file, write_file, read_url, execute_url, command, unsandboxed, and mcp. PreToolUse hooks can also emit allow, deny, ask, or force_ask plus permissionOverrides."
  precedence: "Deny > Ask > Allow for conflicting persistent rules. force_ask from hooks ignores cached permissions for the current tool event."
  merge_semantics: "Docs describe global, project, workspace, and plugin/customization sources but do not fully specify structural merge for permission arrays. Observed logs apply shared user permissions first and project permission grants afterward; project settings take priority over global settings."
  matcher_semantics: "Paths are absolute or workspace-relative and recursive for directories. Windows strips drive letters and converts backslashes to forward slashes. read_url matches hostnames and subdomains and ignores path segments. command and unsandboxed split on whitespace and evaluate each token as an anchored regex. mcp matches server/tool, server/*, or *."
  default_decision: "Workspace file reads/writes are auto-allowed; read_url and execute_url default to Ask; command, mcp, execute_url, and non-workspace files default to Ask."

tool_visibility:
  supported: true
  mechanisms:
    - "MCP disabledTools hides specific tools from a server."
    - "MCP server disabled hides an entire configured server."
    - "Plugin enable/disable and plugins.json include_only/exclude filters control plugin-provided skills, agents, hooks, rules, and MCP servers."
    - "No documented CLI-only built-in --no-tools or --allowed-tools surface was found."
  notes: "Permission approval and tool visibility are distinct. permissions.allow/ask/deny determine what happens when a visible sensitive action is requested; disabledTools and disabled servers prevent MCP tools from being exposed."

sandbox:
  supported: true
  modes: ["disabled", "enabled", "single-command run in sandbox", "single-command run without sandbox", "proceed-in-sandbox", "unsandboxed grants"]
  backends: ["macOS sandbox-exec", "Linux nsjail", "Windows AppContainer documented by CLI sandbox page but contradicted by Antigravity 2.0 permissions page"]
  filesystem_control: "read_file grants populate read-only sandbox allowlists; write_file grants populate read-write allowlists; workspace files are auto-allowed by default. Docs and changelog identify .git and system paths as protected or dangerous."
  network_control: "read_url domain grants are compiled into the sandbox outbound network allowlist; docs say sandbox restricts unauthorized remote network calls and raw TCP queries."
  notes: "Sandboxing is OS-enforced for terminal commands, separate from approval prompts. A current GitHub issue reports that --dangerously-skip-permissions can make --sandbox ineffective, so wrappers should not combine YOLO with sandbox and assume containment."

trust_and_admin:
  folder_trust: "Observed settings include trustedWorkspaces and first-run codelab prompts ask whether the user trusts a folder before allowing read, edit, and execute access. Current fetched docs did not fully specify trust gates or trust file formats."
  managed_policy: "No managed/admin policy layer for Antigravity CLI permissions was found in current CLI docs, --help, repository README/changelog, or observed config files."
  safe_mode: "No provider-native safe mode flag was found. Antigravity desktop changelog history mentions older secure mode, but current fetched CLI docs do not expose a secure-mode CLI control."
  notes: "Project settings and project permission grants override or narrow global settings. Plugins/customizations can add hooks/MCP/rules and should be considered security-relevant project or global configuration."

mcp_permissions:
  supported: true
  server_filters: ["mcpServers.<name>.disabled", "plugin enable/disable", "global ~/.gemini/config/mcp_config.json", "workspace .agents/mcp_config.json"]
  tool_filters: ["disabledTools", "mcp(server/tool)", "mcp(server/*)", "mcp(*)"]
  trust_model: "MCP tools default to Ask unless allowed by policy. Remote MCP can use headers, Google ADC, or OAuth; tokens are stored under ~/.gemini/antigravity/mcp_oauth_tokens.json according to docs."
  notes: "Docs do not state that MCP servers run inside the terminal sandbox. Stdio MCP servers are separate local processes launched from MCP configuration, so wrappers should treat MCP as potentially outside command sandbox unless verified. No response interception/sanitization grammar was documented."

headless_behavior: "In --print/--prompt mode the TUI approval card is unavailable. The docs do not document a programmatic approval channel; use --dangerously-skip-permissions only for deliberate non-interactive YOLO, otherwise approval-required actions should be expected to fail, stall, or require a mode-specific denial path."
approval_persistence: "Persistent approvals live in settings/project permission lists. Interactive file, URL, and MCP prompt edits apply the expanded grant for the remainder of the turn; terminal command scope editing is not supported. Project-level permissions can accumulate as the user interacts with an agent."
protected_paths:
  - ".git/"
  - "/home/user/.ssh"
  - "system paths"
  - "provider app data directories under ~/.gemini/antigravity*, which are app data but not documented as universally protected from writes"
security_posture: "Antigravity combines a static/advisory permission engine, interactive approval UX, and OS-enforced terminal sandboxing. File/MCP/web approval controls are not themselves an OS sandbox; terminal sandboxing is the OS-enforced boundary, and MCP process containment is not documented."
changes: []
requires_claudine_update: true
reason: "Antigravity is a new provider for the catalog and its permission model has native action(target) rules, Deny > Ask > Allow precedence, sandbox-derived allowlists, MCP disabledTools visibility, project-scoped user-state permissions, and no CLI-only zero-permissions baseline; Claudine will need provider metadata and likely PolicyEngine extensions for accurate modeling."
---

# Antigravity Permissions and Security Controls

## Introduction to Antigravity Permissions

Antigravity uses a unified fine-grained permission engine. Every sensitive operation is modeled as a permission resource string in the form `action(target)`, and the same action grammar is documented for Antigravity 2.0 and Antigravity CLI. The core access lists are `deny`, `ask`, and `allow`: deny blocks immediately, ask prompts the user, and allow auto-approves.

For Antigravity CLI, persistent permissions are configured in JSON under `~/.gemini/antigravity-cli/settings.json`:

```json
{
  "permissions": {
    "allow": ["command(git)", "write_file(src/)", "mcp(linter/*)"],
    "deny": ["command(rm -rf)", "write_file(.git/)"],
    "ask": ["command(*)", "execute_url(aws.amazon.com)"]
  }
}
```

Current Antigravity 2.0 docs describe global settings, project settings, standalone conversation settings, and project-level permissions. CLI docs describe global CLI settings plus project permission grants in provider user state. Observed logs on this machine show the CLI initializing `permissions=<nil>, toolPermission=request-review`, loading no shared permissions from `~/.gemini/config/config.json`, and then applying or clearing project permission grants.

No environment variable was found that directly changes Antigravity permission decisions. `HOME` matters because Antigravity CLI stores config under `$HOME/.gemini`. In this session, `HOME=/Users/ken/.claudine`, so a second observed CLI state tree existed under `/Users/ken/.claudine/.gemini`. The human home `/Users/ken/.gemini` also contained Antigravity CLI settings. `/Users/ken/.antigravity` existed but contained IDE/VS Code extension state and `argv.json`, not the CLI permission catalog.

The installed `agy` 1.1.0 help lists the relevant flags: `--sandbox`, `--dangerously-skip-permissions`, `--mode`, `--add-dir`, `--project`, `--new-project`, `--print`, `--prompt`, `--prompt-interactive`, and `--print-timeout`. The docs state command-line overrides temporarily override persistent settings for a single session and are shown in `/config` as command-flag overrides. CLI flags outrank user/project defaults for their active session, but the current docs do not document enterprise managed-policy layers.

Permission and approval policy must be distinguished from tool visibility. Permission rules decide what happens when a visible tool/action is requested. Tool visibility is controlled separately for MCP by `disabled` servers and `disabledTools`, and for plugins/customizations by plugin enablement plus include/exclude filters. No CLI-only `--no-tools`, built-in tool allowlist, or built-in tool denylist flag was found.

## Permissions Use Cases

### Default

With no explicit permission rules, Antigravity CLI defaults to `toolPermission: "request-review"`, `enableTerminalSandbox: false`, `allowNonWorkspaceAccess: false`, and no `permissions` arrays. Files inside the active workspace are auto-allowed. Web browsing (`read_url`, `execute_url`), terminal commands, MCP tools, and non-workspace files default to Ask.

The `PolicyEngine` can partially describe this as a workspace allow policy plus Ask for command, network, MCP, and external file access. It is not ergonomic because Antigravity's default is not a flat rule list: workspace file access is implicit, approval mode is a separate preset, sandboxing is separate and off by default, and project settings live in provider user state. Without changes, `PolicyEngine` can answer many default queries best-effort, but it cannot exactly represent Antigravity's action grammar, command token-regex semantics, turn-scoped prompt grants, or sandbox-derived network/filesystem allowlists.

### Whitelisting

Antigravity can be made whitelist-like persistently by setting broad deny or ask rules and then adding narrower allow rules:

```json
{
  "toolPermission": "strict",
  "allowNonWorkspaceAccess": false,
  "permissions": {
    "deny": ["write_file(.git/)", "command(sudo)", "mcp(sql/execute_mutation)"],
    "ask": ["command(*)", "read_url(*)", "execute_url(*)", "mcp(*)"],
    "allow": ["command(git status)", "command(npm run test)", "read_url(docs.rs)"]
  }
}
```

The best CLI-only, session-scoped locked-down invocation is not truly no-permissions:

```bash
agy --mode plan --sandbox=true
```

This starts with planning and terminal sandboxing, but it does not hide all tools and does not set a deny-all rule. Additional permissions cannot be added back from the CLI as inline permission rules. They must be persisted in settings/project permissions or granted interactively when Ask prompts appear. Examples of additional persistent grants are `command(npm run (build|lint|test))`, `read_file(/var/log/app)`, `write_file(src/)`, `read_url(google.com)`, and `mcp(linter/*)`.

For Claudine, this is the largest wrapper gap. A future wrapper cannot launch Antigravity from a guaranteed no-tools/no-permissions baseline using only CLI flags. `PolicyEngine` can plan persistent edits to `settings.json`, but that mutates user state and does not satisfy session-scoped wrapper isolation.

### YOLO

Antigravity YOLO surfaces are:

- `agy --dangerously-skip-permissions`: auto-approves all tool permission requests without prompting.
- `toolPermission: "always-proceed"`: persistent preset that never prompts except for explicit deny rules.
- Interactive `/permissions` can select `always-proceed`.

The installed help exposes `--dangerously-skip-permissions` for both interactive and `--print` invocations. In YOLO, approval prompts are bypassed, but explicit deny rules should still matter for `always-proceed`. A current GitHub issue reports surprising behavior where combining `--dangerously-skip-permissions` with `--sandbox` appears to make sandboxing ineffective; Claudine should not treat YOLO-plus-sandbox as a safe autonomous sandbox posture without fresh verification.

### Root User

No current Antigravity CLI documentation or source-facing help was found that changes permission behavior when the CLI is started as root. No root-specific prohibition on YOLO was found. Running as root would make the OS account more powerful, so the advisory permission engine and any sandbox escape or unsandboxed command are higher impact.

### Configuring the Default

User-scoped CLI settings live in:

```text
~/.gemini/antigravity-cli/settings.json
```

Shared/global MCP lives in:

```text
~/.gemini/config/mcp_config.json
```

Project records were observed under:

```text
~/.gemini/config/projects/*.json
```

Workspace MCP lives in:

```text
.agents/mcp_config.json
```

The current docs did not document a checked-in repo-local `settings.json` for permission rules. Project-level permissions are described as Antigravity project settings and observed as provider user state, not as a repository file.

### Extending the Base

Practical layering examples:

- A user sets `toolPermission: "request-review"` globally, then a project sets stricter project permissions such as `ask: ["command(*)"]` and `deny: ["write_file(.git/)"]`.
- A user enables `enableTerminalSandbox: true`, then a specific prompt approval runs one command without sandbox restrictions for a single execution.
- A user allows `mcp(linter/*)` globally, while an MCP server config uses `disabledTools` to hide a destructive linter tool from the model entirely.
- A wrapper starts `agy --sandbox=true --mode plan`, overriding the persisted sandbox and mode for that session without editing settings.

## Tools and Permissions

The documented sensitive action namespaces are:

| Action | What It Covers | Default |
| --- | --- | --- |
| `read_file` | File/folder reads | Ask, except workspace auto-allowed |
| `write_file` | File/folder writes; implies matching read | Ask, except workspace auto-allowed |
| `read_url` | Fetching web content and browser loading; also sandbox network allowlist | Ask |
| `execute_url` | Browser actuation such as clicking or typing | Ask |
| `command` | Terminal commands | Ask |
| `unsandboxed` | Commands allowed to bypass terminal sandbox | Ask |
| `mcp` | MCP tools by server/tool, server wildcard, or global wildcard | Ask |

Antigravity also provides adjacent tools and surfaces by default: file tools, terminal command execution, browser/web tools, MCP tools from configured servers, subagents/background tasks, slash commands, skills, hooks, and plugins.

Rule decisions are `allow`, `ask`, and `deny`, with hook-level `force_ask`. Persistent conflict precedence is explicitly Deny > Ask > Allow. `*` is a namespace wildcard. Command targets are whitespace-tokenized and each token is evaluated as an anchored regex. `read_url` matches hostnames/subdomains and ignores path segments. File paths are recursive for directories; Windows normalizes paths before matching.

Approvals can persist as settings/project rules, and prompt-card target edits for file, URL, and MCP grants apply for the remainder of the turn. Scope editing is not supported for terminal commands.

## Sandboxing, Trust, and Administrative Controls

Antigravity's terminal sandbox is separate from approval mode. CLI docs describe native OS containment:

| OS | Backend |
| --- | --- |
| macOS | `sandbox-exec` |
| Linux | `nsjail` |
| Windows | `AppContainer` |

The Antigravity 2.0 permissions page says terminal sandboxing is preview on macOS/Linux and coming soon to Windows, so Windows support should be treated as uncertain until observed on Windows.

When sandboxing is enabled, `read_file` grants become read-only filesystem allowlists, `write_file` grants become read-write filesystem allowlists, and `read_url` domains become outbound network allowlists. The command prompt can also run a single command without sandbox restrictions when sandboxing is enabled, or run a single command in sandbox when sandboxing is disabled.

Folder/project trust exists in practice: codelab onboarding asks whether the user trusts a folder before allowing read, edit, and execute access, and observed settings included `trustedWorkspaces`. The fetched docs do not fully specify what trust gates for CLI config, MCP, hooks, plugins, or auto-approval.

No managed/admin policy layer was found in current CLI docs or observed config. Protected path behavior is partly rule-based and partly sandbox hardening: docs use `.git/` and `.ssh` examples, and the changelog says `.git` was added to the core dangerous paths list and system path writes are blocked.

The honest posture is a combination: static/advisory permission policy plus interactive UX prompts plus OS-enforced sandbox for terminal commands. File/MCP/web approval controls are not themselves OS sandboxing.

## MCP and Permissions

MCP is first-class and uses the same permission grammar:

```text
mcp(server/tool)
mcp(server/*)
mcp(*)
```

Unconfigured MCP tools default to Ask. MCP can be made safer by disabling unused servers, using `disabledTools` to hide destructive tools from the model, and adding explicit Ask or Deny rules such as `mcp(sql/execute_mutation)` or `mcp(*)`.

Global MCP config lives at `~/.gemini/config/mcp_config.json`; workspace MCP config lives at `.agents/mcp_config.json`; plugin MCP config lives under plugin directories. Server entries support `command`, `args`, `env`, `cwd`, `serverUrl`, `headers`, `authProviderType`, `oauth`, `disabled`, and `disabledTools`.

The docs do not state that MCP stdio processes run inside the terminal sandbox. Until verified, Claudine should model MCP execution as a separate local/remote tool surface that can bypass terminal sandbox assumptions.

## Non-Interactive Behavior

`agy --print` and `agy --prompt` run a single prompt non-interactively. The current docs do not document an in-band programmatic approval channel for Ask prompts. In headless runs, use `--dangerously-skip-permissions` only when YOLO is intended; otherwise approval-required tools should be expected to fail, stall, or be denied by provider behavior that is not yet documented.

`--prompt-interactive` submits an initial prompt and then continues in the TUI, so normal approval cards can still be handled.

## Sources

- [Antigravity CLI product page](https://antigravity.google/product/antigravity-cli)
- [Antigravity CLI GitHub repository](https://github.com/google-antigravity/antigravity-cli)
- [Antigravity CLI changelog](https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md)
- [Antigravity CLI permissions docs](https://antigravity.google/docs/cli/permissions)
- [Antigravity CLI sandbox docs](https://antigravity.google/docs/cli/sandbox)
- [Antigravity CLI settings docs](https://antigravity.google/docs/cli/settings)
- [Antigravity CLI reference docs](https://antigravity.google/docs/cli/reference)
- [Antigravity MCP docs](https://antigravity.google/docs/mcp)
- [Antigravity 2.0 permissions docs](https://antigravity.google/docs/permissions)
- [Antigravity 2.0 settings docs](https://antigravity.google/docs/settings)
- Local installed CLI: `agy --version` returned `1.1.0`; `agy --help` listed the CLI flags captured in frontmatter.
- Local observed config: `/Users/ken/.gemini/antigravity-cli/settings.json`, `/Users/ken/.gemini/config/config.json`, `/Users/ken/.gemini/config/projects/*.json`, and corresponding `$HOME=/Users/ken/.claudine` paths under `/Users/ken/.claudine/.gemini`.
