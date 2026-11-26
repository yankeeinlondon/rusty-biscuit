---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-03
agent: codex
model: default

cli_params:
  - param: tools
    style: switch
    description: "Comma-separated allowlist of tool names. Only listed built-in, extension, and SDK/custom tools are exposed for the session."
    example: 'pi --tools read,grep,find,ls -p "Review the code"'
    example_description: "Starts a non-interactive read/search-only run by exposing only read, grep, find, and ls."
  - param: exclude-tools
    style: switch
    description: "Comma-separated denylist of tool names. The listed built-in, extension, and SDK/custom tools are removed from the active set after any allowlist is applied."
    example: 'pi --exclude-tools bash -p "Answer without running commands"'
    example_description: "Keeps default tools except the shell tool."
  - param: no-tools
    style: switch
    description: "Disables all tools by default by mapping the session to an empty tool allowlist."
    example: 'pi --no-tools -p "Answer from the prompt only"'
    example_description: "Runs a tool-free session."
  - param: no-builtin-tools
    style: switch
    description: "Disables the default built-in tools while leaving extension and custom tools available unless separately filtered."
    example: "pi --no-builtin-tools -e ./my-tools.ts"
    example_description: "Starts without Pi's built-in read, bash, edit, and write tools, using tools supplied by an explicit extension."
  - param: extension
    style: switch
    description: "Loads an extension file or directory; can be repeated. Extensions can register tools, commands, flags, hooks, providers, and policy-like gates."
    example: "pi --extension ./permission-gate.ts"
    example_description: "Loads an explicit extension for this run."
  - param: no-extensions
    style: switch
    description: "Disables extension auto-discovery. Explicit --extension/-e paths still load."
    example: 'pi --no-extensions -p "Use only built-in behavior"'
    example_description: "Prevents user and project extension discovery for the session."
  - param: skill
    style: switch
    description: "Loads a skill file or directory; can be repeated. Skills are prompt resources, not execution permissions, but they can influence tool use."
    example: 'pi --skill ./SKILL.md -p "Use this procedure"'
    example_description: "Adds one explicit skill resource."
  - param: no-skills
    style: switch
    description: "Disables skill discovery and loading."
    example: 'pi --no-skills -p "Answer without skill instructions"'
    example_description: "Runs without user or project skills."
  - param: prompt-template
    style: switch
    description: "Loads a prompt template file or directory; can be repeated. Prompt templates can add slash-command-like workflows."
    example: 'pi --prompt-template ./prompts'
    example_description: "Adds explicit prompt templates."
  - param: no-prompt-templates
    style: switch
    description: "Disables prompt-template discovery and loading."
    example: 'pi --no-prompt-templates -p "Answer without templates"'
    example_description: "Runs without discovered prompt templates."
  - param: theme
    style: switch
    description: "Loads a theme file or directory; can be repeated. This is UI-adjacent and not an execution permission."
    example: "pi --theme ./theme.json"
    example_description: "Loads an explicit theme."
  - param: no-themes
    style: switch
    description: "Disables theme discovery and loading."
    example: "pi --no-themes"
    example_description: "Runs with built-in/default theme behavior."
  - param: no-context-files
    style: switch
    description: "Disables AGENTS.md and CLAUDE.md context-file discovery."
    example: 'pi --no-context-files -p "Ignore repository instructions"'
    example_description: "Prevents project and global context files from influencing the run."
  - param: approve
    style: switch
    description: "Trusts project-local Pi settings, resources, packages, and extensions for this run."
    example: 'pi --approve -p "Run the project task"'
    example_description: "Session-scoped project trust approval."
  - param: no-approve
    style: switch
    description: "Declines project-local Pi settings, resources, packages, and extensions for this run."
    example: 'pi --no-approve -p "Summarize from prompt context only"'
    example_description: "Session-scoped project trust denial."
  - param: offline
    style: switch
    description: "Disables startup network operations such as update checks, package update checks, and install/update telemetry."
    example: 'pi --offline -p "Work without startup network checks"'
    example_description: "Prevents Pi-managed startup network calls for one run."
  - param: mode
    style: switch
    description: "Selects output mode: text, json, or rpc. This affects whether extension UI/approval prompts can be shown programmatically."
    example: 'pi --mode json "Inspect this"'
    example_description: "Runs in JSON event-stream mode."
  - param: print
    style: switch
    description: "Runs a non-interactive prompt and exits. Extension UI helpers are unavailable in print mode."
    example: 'pi --print "List files"'
    example_description: "Headless single-prompt execution."
  - param: no-session
    style: switch
    description: "Uses an in-memory session and avoids saving transcript state."
    example: 'pi --no-session -p "One-shot private prompt"'
    example_description: "Avoids persisting the session JSONL."
  - param: session-dir
    style: switch
    description: "Overrides the directory used for session storage and lookup; takes precedence over PI_CODING_AGENT_SESSION_DIR and settings.json sessionDir."
    example: 'pi --session-dir /tmp/pi-sessions -p "Use temporary session storage"'
    example_description: "Moves session persistence to a temporary directory for this run."

env_vars:
  - name: PI_CODING_AGENT_DIR
    effect: "Overrides the Pi agent configuration directory, which changes where settings.json, trust.json, models.json, auth.json, extensions, skills, prompts, themes, and sessions are read from."
    effect_category: state_home_relocation
  - name: PI_CODING_AGENT_SESSION_DIR
    effect: "Overrides session storage unless --session-dir is provided."
    effect_category: state_home_relocation
  - name: PI_PACKAGE_DIR
    effect: "Overrides the package asset directory; this affects built-in assets and package/resource lookup, including extension package contents."
    effect_category: state_home_relocation
  - name: PI_OFFLINE
    effect: "Disables startup network operations when set to 1/true/yes, including update checks, package update checks, and install/update telemetry."
    effect_category: network_control
  - name: PI_SKIP_VERSION_CHECK
    effect: "Skips the startup latest-version request to pi.dev without disabling other network operations."
    effect_category: network_control
  - name: PI_TELEMETRY
    effect: "Overrides install/update telemetry and provider attribution headers when set to 1/true/yes or 0/false/no. It does not grant or deny tool permissions."
    effect_category: none

config_files:
  - os: macos
    user: ".pi/agent/settings.json"
    repo: ".pi/settings.json"
    notes: "Relative to the user's home directory and repo root. Project settings load only after project trust is granted. Observed local /Users/ken/.pi/agent/settings.json contains model defaults only; no permission settings were present."
  - os: linux
    user: ".pi/agent/settings.json"
    repo: ".pi/settings.json"
    notes: "Same home-relative and repo-relative paths as macOS. Project settings load only after project trust is granted."
  - os: windows
    user: ".pi/agent/settings.json"
    repo: ".pi/settings.json"
    notes: "Same home-relative and repo-relative paths using the Windows home directory. Project settings load only after project trust is granted."
  - os: macos
    user: ".pi/agent/trust.json"
    repo: "none"
    notes: "Stores persisted project trust decisions by canonical directory. No local trust.json was present in /Users/ken/.pi/agent during this run."
  - os: linux
    user: ".pi/agent/trust.json"
    repo: "none"
    notes: "Stores persisted project trust decisions by canonical directory."
  - os: windows
    user: ".pi/agent/trust.json"
    repo: "none"
    notes: "Stores persisted project trust decisions by canonical directory."

precedence:
  - source: cli
    scope: [tool_visibility, trust, config_loading, other]
    merge_strategy: none
    notes: "CLI flags are session-scoped. --tools creates an allowlist, --exclude-tools removes names, --no-tools creates an empty allowlist, --approve/--no-approve override project trust for the run, and --offline overrides startup network behavior."
  - source: extension_hooks
    scope: [other, trust, tool_visibility, slash_commands]
    merge_strategy: shallow
    notes: "Loaded extensions run in load order. tool_call handlers can mutate input; the first { block: true } result blocks the call. Extension code is trusted code running with the Pi process permissions."
  - source: env
    scope: [config_loading, other]
    merge_strategy: none
    notes: "Environment variables mostly move config/storage locations or disable startup network/telemetry surfaces. They do not define allow/ask/deny policy."
  - source: repo_config
    scope: [general_config, customization_resources, extensions, skills]
    merge_strategy: shallow
    notes: "Project .pi/settings.json overlays global settings after project trust. Nested setting objects are merged at the top nested-object level; arrays and scalars replace. Project settings do not contain a native permissions key."
  - source: trust_store
    scope: [trust]
    merge_strategy: nearest
    notes: "Saved trust decisions in trust.json apply by closest current or parent directory before defaultProjectTrust."
  - source: user_config
    scope: [general_config, customization_resources, extensions, skills]
    merge_strategy: shallow
    notes: "User/global settings provide the baseline. defaultProjectTrust is documented as a global-only setting."

default_posture: "Pi starts with read, bash, edit, and write built-in tools enabled, no native approval prompts, and the filesystem/process/network permissions of the launching OS user. grep, find, and ls are built-in read-only tools but are normally enabled only when explicitly selected or activated by an extension."

cli_zero_permissions:
  supported: true
  invocation: "pi --no-tools --no-extensions --no-skills --no-prompt-templates --no-context-files --no-themes --no-approve --offline --no-session"
  mechanism: "Empty tool allowlist plus disabled resource discovery, declined project trust, disabled startup network operations, and in-memory session state."
  limitations: "This removes model-callable tools and most local resource influence for one run, but it is not an OS sandbox. Pi can still call the selected model provider, built-in startup/runtime code runs as the OS user, explicitly supplied extensions would execute if added, and there is no CLI mechanism to add permissions back later except by choosing a different allowlist at launch."

agent_permissions:
  allowed: false
  fm_properties: []

yolo:
  has_interactive_yolo: false
  has_non_interactive_yolo: false
  mechanism: "No explicit YOLO flag, mode, env var, or config key. Pi's default is already permissive, but it is not exposed as a named YOLO mode."

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - "PolicyEngine has no Pi backend today."
    - "Pi core has no native static allow/ask/deny rule grammar for tool calls, paths, commands, network, or MCP."
    - "Pi's strongest native control is tool visibility, not approval policy."
    - "Project trust gates project-local resource loading but does not constrain trusted tools after startup."
    - "Extension-based permission gates are arbitrary TypeScript code and may be dynamic, stateful, or undiscoverable from static config."
    - "Optional packages such as pi-permission-system define their own non-core policy grammar that would need separate provider-extension modeling."

permission_entities:
  - entity: tool
    native_names: ["read", "bash", "edit", "write", "grep", "find", "ls", "--tools", "--exclude-tools", "--no-tools", "--no-builtin-tools", "pi.getActiveTools", "pi.setActiveTools", "pi.registerTool"]
    notes: "Tool visibility is native. Per-call allow/deny is implemented only through extension tool_call handlers."
  - entity: command
    native_names: ["bash", "user_bash", "! command", "!! command"]
    notes: "The model can invoke the bash tool when active. Interactive user shell shortcuts can be intercepted by extensions through user_bash."
  - entity: path
    native_names: ["tool input path fields", "extension tool_call input", "protected-paths example"]
    notes: "Pi core has no path rule grammar. Extensions can inspect path-bearing tool inputs and block or mutate them."
  - entity: workspace
    native_names: ["project trust", "defaultProjectTrust", "trust.json", "--approve", "--no-approve", "/trust"]
    notes: "Trust gates project-local settings/resources/packages/extensions, not runtime filesystem access."
  - entity: extension
    native_names: ["extensions", "packages", "--extension", "--no-extensions", "pi.registerFlag", "pi.on"]
    notes: "Extensions are trusted code with full process permissions. They can add tools, commands, hooks, flags, providers, and policy-like behavior."
  - entity: slash_command
    native_names: ["pi.registerCommand", "prompt templates", "skill commands", "/trust", "/settings", "/reload"]
    notes: "Slash commands can be built-in, extension-provided, prompt-template-provided, or skill-derived. They are not permission rules but can mutate session behavior."
  - entity: hook
    native_names: ["project_trust", "before_provider_request", "after_provider_response", "tool_call", "tool_result", "user_bash", "input", "context", "before_agent_start"]
    notes: "Hooks are extension APIs. tool_call can block; tool_result and provider/context hooks can rewrite data."
  - entity: mcp_server
    native_names: []
    notes: "Pi core does not include built-in MCP support. MCP may be implemented by extensions or packages."
  - entity: mcp_tool
    native_names: []
    notes: "No native MCP tool filtering exists in Pi core."
  - entity: agent
    native_names: ["subagent example extension", "agentScope", "tools frontmatter"]
    notes: "Subagents are not a native Pi core feature. The example subagent extension supports user/project agent files with tools/model frontmatter."
  - entity: mode
    native_names: ["--mode text", "--mode json", "--mode rpc", "--print"]
    notes: "Mode affects UI availability and output protocol, not native approval policy."
  - entity: sandbox
    native_names: ["Gondolin extension", "Docker", "OpenShell", "sandbox example extension"]
    notes: "Sandboxing is external or extension-provided, not built into Pi core."

approval_modes: []

rule_model:
  decisions: ["allow by omission", "block via extension { block: true, reason?: string }"]
  syntax: "Pi core has no static rule file or allow/ask/deny grammar. Extension TypeScript can inspect and mutate tool_call input and return { block: true, reason?: string }; project_trust extensions return { trusted: \"yes\" | \"no\" | \"undecided\", remember?: boolean }."
  precedence: "For tool_call handlers, extensions run in load order and the first blocking result stops the tool. If no handler blocks, the call proceeds. CLI tool visibility removes unavailable tools before the model can call them."
  merge_semantics: "No native rule merge. Settings merge globally then project after trust; extension behavior composes procedurally in load order."
  matcher_semantics: "No native matcher semantics. Matching is arbitrary extension code; examples use regular expressions for bash commands and substring checks for paths."
  default_decision: "allow for any visible tool call"

tool_visibility:
  supported: true
  mechanisms:
    - "--tools <comma-list> allowlists active tool names."
    - "--exclude-tools <comma-list> removes active tool names."
    - "--no-tools starts with an empty active-tool allowlist."
    - "--no-builtin-tools disables the default built-in tool set while preserving extension/custom tools."
    - "Extensions can call pi.registerTool(), pi.getActiveTools(), and pi.setActiveTools()."
  notes: "Tool visibility is separate from approval policy. Pi can hide tools from the model, but visible tools are not natively pre-approved versus approval-required; they simply execute unless an extension blocks them."

sandbox:
  supported: false
  modes: []
  backends: []
  filesystem_control: "none in Pi core"
  network_control: "none in Pi core; --offline only disables Pi-managed startup network operations, not model/provider calls or tool network access"
  notes: "Pi documents external containment patterns: whole-process Docker, whole-process OpenShell, host Pi with Gondolin micro-VM routing for built-in tools and ! commands, and an example sandbox extension for bash."

trust_and_admin:
  folder_trust: "Project trust gates loading of .pi/settings.json, .pi resources, project packages, project extensions, project system prompt files, and project .agents/skills. Trust can be saved through /trust in trust.json; --approve and --no-approve override for one run."
  managed_policy: "No native managed/admin policy layer was found."
  safe_mode: "No native safe mode was found. A locked-down run must be assembled from CLI flags such as --no-tools, --no-extensions, --no-context-files, --no-approve, --offline, and --no-session."
  notes: "AGENTS.md and CLAUDE.md context files load regardless of project trust unless --no-context-files is used. Project trust is an input-loading guard, not a sandbox."

mcp_permissions:
  supported: false
  server_filters: []
  tool_filters: []
  trust_model: "No native MCP trust model in Pi core."
  notes: "Pi documentation states it intentionally does not include built-in MCP. MCP can be added by extensions/packages; the Pi package catalog lists pi-permission-system as an extension with MCP-related gates, but that is optional third-party extension behavior, not a Pi core permission surface."

headless_behavior: "In print mode and JSON mode, extension UI helpers cannot prompt; a permission-gate extension must block, allow, or use non-UI logic. RPC mode exposes extension dialogs through the JSON protocol, but native project-trust prompts are not shown in non-interactive modes and fall back to saved trust/defaultProjectTrust/--approve/--no-approve."

approval_persistence: "Pi core has no per-tool approval persistence. Project trust decisions persist in .pi/agent/trust.json by canonical directory; extension-provided gates may persist their own decisions if they implement that behavior."

protected_paths: []

security_posture: "Pi core is a permissive local agent harness with tool visibility filters and a project-resource trust gate. It is not an OS-enforced sandbox or native static policy engine; stronger guarantees require external containment or trusted extension/package code."

changes:
  - "Refreshed against earendil-works/pi main at commit 23d1462611ab74b4874c35e701a43d7caa5e3de3 and package version 0.80.3 on 2026-07-03."
  - "Corrected config_files to use separate macOS, Linux, and Windows records instead of the legacy unsupported os: all value."
  - "Verified that current Pi core still has no native permission prompts, no explicit YOLO flag, no managed/admin policy, no built-in MCP, and no native sandbox."
  - "Updated the default posture to distinguish the default active tools read/bash/edit/write from additional built-in read-only tools grep/find/ls that are normally activated explicitly."
  - "Documented current CLI-only locked-down launch using --no-tools plus resource/trust/session/startup-network suppression flags."
  - "Added extension hook behavior as the lower-level policy mechanism: tool_call can mutate input or block; tool_result and provider hooks can rewrite data; extension errors in tool_call fail closed by blocking execution."
  - "Added the newly observed optional pi-permission-system package as non-core context without treating its grammar as Pi native policy."
  - "Recorded local config inspection: /Users/ken/.pi/agent/settings.json existed with model defaults only, no trust.json existed, and no repo .pi/settings.json existed in this workspace."

requires_claudine_update: true
reason: "Claudine's PolicyEngine still lacks a Pi backend and cannot currently model Pi's core distinction between tool visibility, project trust, and arbitrary extension-mediated blocking."
---

# Pi Permissions and Security Controls

## Introduction to Pi Permissions

Pi core does not define a native permission policy for filesystem, process, network, credential, or MCP access. Its own security documentation describes Pi as a local coding agent that runs with the permissions of the user account that starts it, and the repository README states that Pi does not include a built-in permission system for restricting filesystem, process, network, or credential access.

The controls Pi does provide are narrower:

- Tool visibility controls which tools are exposed to the model.
- Project trust controls whether project-local Pi resources are loaded.
- Extensions can implement custom gates by intercepting events such as `tool_call`, `user_bash`, `project_trust`, and provider payload hooks.
- External isolation can be supplied by Docker, OpenShell, Gondolin, or other OS/container mechanisms.

Configuration files can define project trust defaults, package/resource loading, and extension loading. They do not define native allow/ask/deny permissions. The relevant files are `~/.pi/agent/settings.json`, `<repo>/.pi/settings.json`, and `~/.pi/agent/trust.json`. Project settings are loaded only after project trust is granted.

Observed local configuration on this host:

- `/Users/ken/.pi/agent/settings.json` exists and contains only `lastChangelogVersion`, `defaultProvider`, and `defaultModel`.
- `/Users/ken/.pi/agent/trust.json` does not exist.
- This repository has no `.pi/settings.json`.
- The session `$HOME` also had `/Users/ken/.claudine/.pi/agent/auth.json`, but no settings or trust file.

Environment variables that influence security-adjacent behavior are mostly path and startup-network switches. `PI_CODING_AGENT_DIR` moves the whole agent config directory, `PI_CODING_AGENT_SESSION_DIR` moves session storage, `PI_PACKAGE_DIR` changes package asset lookup, `PI_OFFLINE` disables startup network operations, `PI_SKIP_VERSION_CHECK` disables the version check, and `PI_TELEMETRY` affects telemetry/provider-attribution headers. None of these variables grants or denies tool execution.

Permission-adjacent CLI switches have higher precedence than config and env where they apply:

| CLI switch | Effect |
| --- | --- |
| `--tools <list>` / `-t <list>` | Exposes only the comma-separated tool names. |
| `--exclude-tools <list>` / `-xt <list>` | Removes the comma-separated tool names. |
| `--no-tools` / `-nt` | Starts with no active tools by using an empty allowlist. |
| `--no-builtin-tools` / `-nbt` | Disables default built-in tools but keeps extension/custom tools enabled. |
| `--extension <path>` / `-e <path>` | Loads an explicit extension for the run. |
| `--no-extensions` / `-ne` | Disables auto-discovered extensions; explicit `-e` still loads. |
| `--skill <path>` | Loads an explicit skill. |
| `--no-skills` / `-ns` | Disables skill discovery and loading. |
| `--prompt-template <path>` | Loads explicit prompt templates. |
| `--no-prompt-templates` / `-np` | Disables prompt-template discovery and loading. |
| `--theme <path>` | Loads an explicit theme. |
| `--no-themes` | Disables theme discovery and loading. |
| `--no-context-files` / `-nc` | Disables `AGENTS.md` and `CLAUDE.md` context loading. |
| `--approve` / `-a` | Trusts project-local Pi resources for this run. |
| `--no-approve` / `-na` | Ignores project-local Pi resources for this run. |
| `--offline` | Disables startup network operations for this run. |
| `--mode text|json|rpc` | Selects output/UI protocol. |
| `--print` / `-p` | Runs one non-interactive prompt and exits. |
| `--no-session` | Avoids saving a persistent session transcript. |
| `--session-dir <dir>` | Overrides session storage location for the run. |

Precedence is best summarized as:

```text
CLI flags > extension hook decisions at runtime > environment path/network switches > trusted project settings > trust.json saved decisions/defaultProjectTrust > user settings
```

This is not a single unified policy stack. CLI tool switches affect tool visibility before the model sees tools. Extension hooks act later, around runtime events. Project trust gates loading of project-local configuration and code.

Permission/approval policy is distinct from tool visibility. Pi core supports tool visibility but not native approval policy. If `bash` is visible, Pi core does not ask before running shell commands. If `write` is visible, Pi core does not ask before writing. Approval prompts require extension code or external wrapping.

## Permissions Use Cases

### Default

With no relevant CLI flags, env vars, config, or saved trust decision, Pi starts with the default active built-in tools: `read`, `bash`, `edit`, and `write`. These tools run with the Pi process permissions. Pi also ships `grep`, `find`, and `ls` as built-in read-only tools, but they are normally activated explicitly, such as with `--tools read,grep,find,ls`, or by extension/runtime tool changes.

Project trust defaults to `ask` for interactive sessions when trust-requiring project resources exist. In non-interactive modes, `ask` behaves as not trusted unless a saved trust decision or `--approve` says otherwise. `AGENTS.md` and `CLAUDE.md` context files still load unless `--no-context-files` is used.

PolicyEngine is not ergonomic for this default because Pi core does not expose a native policy grammar. A future Pi backend could represent the default as:

- tool visibility: `read`, `bash`, `edit`, `write`
- approval mode: no native prompts
- filesystem/process/network: OS-user permissions
- project config: gated by project trust

Without code changes, PolicyEngine cannot define this use case as Pi-native policy because there is no Pi provider backend and no Pi rule syntax to emit.

### Whitelisting

Pi can start from fewer visible tools, but it does not have a native "ask for needed permissions" mode. The closest native pattern is CLI allowlisting:

```bash
pi --tools read,grep,find,ls -p "Review the code without changing it"
pi --tools read -p "Read only the files I mention"
pi --exclude-tools bash -p "Use file tools but do not run commands"
```

To start with no model-callable tools for one Claudine-controlled session, the best CLI-only invocation is:

```bash
pi --no-tools --no-extensions --no-skills --no-prompt-templates --no-context-files --no-themes --no-approve --offline --no-session
```

This is session-scoped and does not mutate the user's Pi config. It disables all tools, disables automatic extension/resource/context loading, declines project-local Pi resources, disables startup network operations, and avoids persistent session state.

Limitations:

- It is not an OS sandbox.
- Pi can still contact the selected model provider unless the model/provider configuration itself prevents that.
- It does not provide an interactive escalation channel to add a tool mid-run from the CLI.
- Explicit `--extension` paths would still run if Claudine supplied them.
- Runtime code still runs with the launching user's permissions.

Additional permissions can be granted by choosing a more permissive allowlist at launch:

```bash
pi --tools read,grep,find,ls --no-extensions --no-context-files -p "Analyze only"
pi --tools read,bash --exclude-tools write,edit -p "Run read-only diagnostics"
pi --tools read,edit,write --exclude-tools bash -p "Patch files without shell commands"
```

PolicyEngine could model these as tool-visibility rules, but not as approval rules. A useful PolicyEngine improvement would be a separate `ToolVisibility` surface plus a provider capability flag that says "visible means executable without native approval."

### YOLO

Pi has no explicit YOLO mode. There is no `--yolo`, approval-mode flag, env var, or settings key for bypassing prompts because Pi core has no native permission prompts to bypass.

In practice, Pi's default is permissive: active tools execute as the OS user. That default is available in both interactive and non-interactive sessions. What is allowed is whatever the active tools, extensions, OS account, shell, filesystem permissions, network policy, and provider credentials allow. What is not allowed is anything outside those OS/runtime boundaries, or any tool call blocked by an extension.

### Root User

I found no current source branch that changes Pi's permission behavior when the process runs as root. Pi relies on the OS account boundary, so running as root expands the effective filesystem/process permissions available to tools and extensions. There is still no special YOLO switch; the same permissive default applies with root's larger OS privileges.

### Configuring the Default

Pi settings use JSON:

```json
{
  "defaultProjectTrust": "ask",
  "extensions": ["/path/to/extension.ts"],
  "packages": ["npm:some-pi-package@1.0.0"],
  "skills": ["/path/to/skills"],
  "prompts": ["/path/to/prompts"],
  "themes": ["/path/to/theme.json"],
  "sessionDir": "/tmp/pi-sessions",
  "images": {
    "blockImages": true
  },
  "httpProxy": "http://127.0.0.1:7890"
}
```

User scope:

- macOS/Linux/Windows home-relative: `.pi/agent/settings.json`
- saved trust decisions: `.pi/agent/trust.json`

Repo scope:

- `.pi/settings.json`
- `.pi/extensions`, `.pi/skills`, `.pi/prompts`, `.pi/themes`
- `.pi/SYSTEM.md`, `.pi/APPEND_SYSTEM.md`

There is no native `permissions` key. Extension packages may define their own config grammar, but that grammar belongs to the extension, not Pi core.

### Extending the Base

Examples:

- A user config can set `"defaultProjectTrust": "never"` to avoid loading project-local Pi resources by default; `pi --approve` overrides that for one run.
- A user config can load global extensions, while `pi --no-extensions` disables auto-discovery for one session.
- A trusted repo can add `.pi/settings.json` with project packages/extensions; `pi --no-approve` ignores those resources for one session.
- A default session may expose `read,bash,edit,write`; `pi --tools read,grep,find,ls` replaces that with a read/search allowlist.
- A global extension can implement a policy gate; `pi --no-extensions` removes that auto-discovered gate unless the gate is supplied explicitly with `-e`.

## Tools and Permissions

Pi core ships these built-in tools:

| Tool | Default active? | Notes |
| --- | --- | --- |
| `read` | Yes | Reads files, including supported images. |
| `bash` | Yes | Executes shell commands. |
| `edit` | Yes | Edits files with find/replace style operations. |
| `write` | Yes | Creates or overwrites files. |
| `grep` | No | Read-only search tool, commonly enabled in read-only allowlists. |
| `find` | No | Read-only file glob/search tool, commonly enabled in read-only allowlists. |
| `ls` | No | Read-only directory listing tool, commonly enabled in read-only allowlists. |

Permissions map to tool calls only indirectly:

- `--tools` and `--exclude-tools` change whether a tool can be called.
- Extension `tool_call` handlers can inspect the tool name and input, mutate input, or return `{ block: true, reason }`.
- Extension `tool_result` handlers can rewrite returned content.
- Extension `before_provider_request` handlers can inspect or replace provider payloads.
- Extension `user_bash` handlers can intercept user `!`/`!!` shell commands.

Native permission entities and adjacent controls are:

| Entity | Native names | Native rule support |
| --- | --- | --- |
| Tool | `read`, `bash`, `edit`, `write`, `grep`, `find`, `ls`, extension tools | Visibility only; block via extension hook. |
| Command | `bash`, `user_bash`, slash commands | No static command policy; extension hooks can intercept. |
| Path | Tool input paths | No static path policy; extension code can inspect. |
| Workspace | project trust, `trust.json`, `defaultProjectTrust` | Gates project-local resource loading. |
| Extension | `--extension`, settings `extensions`, packages | Trusted code with full process permissions. |
| Hook | `tool_call`, `tool_result`, `project_trust`, provider hooks | Procedural extension control. |
| Slash command | built-in, extension, prompt-template, skill commands | Workflow surface, not native permission policy. |
| MCP | none in core | Extension/package only. |
| Agent/subagent | none in core | Example extension only. |
| Sandbox | none in core | External/container/extension only. |

Pi's native rule grammar is minimal because there is no static policy language. The only native decision-like hook for tool calls is extension-provided blocking:

```typescript
pi.on("tool_call", async (event, ctx) => {
  if (event.toolName === "bash" && String(event.input.command).includes("rm -rf")) {
    return { block: true, reason: "Dangerous command" };
  }
});
```

Matcher semantics are therefore whatever the extension implements: regular expressions, globs, prefixes, path normalization, model classification, UI prompts, or external policy calls. Conflict precedence is procedural: handlers run in extension load order, later handlers see earlier mutations, and the first blocking result blocks the tool. If no hook blocks, a visible tool executes.

Pi has no native approval modes or aliases such as plan, auto-edit, accept-edits, classifier mode, or bypass mode. The example `plan-mode` extension implements a read-only exploration mode with `/plan`, `--plan`, and `Ctrl+Alt+P`, but that is extension behavior. The same is true for third-party permission packages.

Pi core approvals do not persist because Pi core has no per-tool approvals. Project trust persists in `trust.json`. Extension-provided approvals may persist only if the extension implements persistence.

## Sandboxing, Trust, and Administrative Controls

Pi has no built-in sandbox. Built-in tools, extension tools, package installs, shell commands, language servers, and test commands run as ordinary local processes with the Pi process permissions.

External isolation patterns documented by Pi:

| Pattern | Isolated surface | Notes |
| --- | --- | --- |
| Gondolin extension | Built-in tools and `!` commands | Host Pi keeps auth; tool execution is routed into a local Linux micro-VM. Other custom extension tools still run on the host unless they delegate too. |
| Plain Docker | Whole Pi process | Simple local isolation; API keys and mounted files enter the container. |
| OpenShell | Whole Pi process | Policy-controlled sandbox with filesystem, process, network, credential, and inference controls when configured through an OpenShell gateway. |
| Example sandbox extension | Bash tool | Example extension using `@anthropic-ai/sandbox-runtime`; not core behavior. |

Project trust is a resource-loading gate. It applies when Pi finds trust-requiring resources such as `.pi/settings.json`, `.pi/extensions`, `.pi/skills`, `.pi/prompts`, `.pi/themes`, `.pi/SYSTEM.md`, `.pi/APPEND_SYSTEM.md`, or project `.agents/skills`. Trusting allows those resources to load. Declining skips them.

Trust does not gate:

- ordinary filesystem access by active tools
- `AGENTS.md` and `CLAUDE.md` context files, unless `--no-context-files` is used
- global/user extensions
- explicit CLI `-e` extensions
- model output or prompt injection from loaded context

No managed/admin policy layer was found. No native safe mode was found. No provider-reserved protected paths were found in Pi core; examples such as blocking `.env`, `.git/`, or `node_modules/` are extension code.

The honest security posture is a combination of advisory/runtime controls:

- Tool visibility: native, static per run.
- Project trust: native input-loading guard.
- Extension gates: procedural, trusted code, best-effort unless the extension itself uses OS isolation.
- Sandbox: not native; external or extension-provided.

## MCP and Permissions

Pi core does not include built-in MCP. Current Pi usage documentation explicitly says Pi intentionally does not include built-in MCP, subagents, permission popups, plan mode, to-dos, or background bash; those workflows are expected to be built or installed as extensions/packages.

Because MCP is extension/package-defined, there are no native Pi server filters, tool filters, resource filters, trust flags, response interception rules, or sandbox routing rules for MCP. An MCP extension would run with the same process permissions as Pi unless it delegates work to a container, VM, remote gateway, or OS sandbox.

Permissions can make MCP safer only through non-core mechanisms:

- Run Pi in Docker/OpenShell or another OS sandbox before enabling MCP.
- Use `--no-extensions` by default and load only a vetted MCP extension with `-e`.
- Use `--tools`/`--exclude-tools` if the MCP extension registers named tools that Pi can filter.
- Add an extension `tool_call` gate to filter MCP-like tools by server/tool/resource name.
- Add `tool_result` or provider hooks to redact responses if the extension exposes those data paths.

The Pi package catalog currently lists `pi-permission-system`, a third-party extension that advertises centralized permission gates for tool, bash, MCP, skill, and special operations. That is useful current ecosystem context, but it is not Pi core behavior and should not be treated as a native provider permission grammar.

## Non-Interactive Behavior

Print mode (`-p`) and JSON mode do not provide extension UI prompts. Extension gates that need confirmation must choose a default behavior; Pi's own `permission-gate.ts` example blocks dangerous commands when no UI is available. RPC mode has an extension UI channel, so extension prompts can be handled programmatically, but native project trust still does not prompt in non-interactive modes.

For project trust, non-interactive modes use saved trust decisions, `defaultProjectTrust`, and `--approve`/`--no-approve`. With the default `defaultProjectTrust: "ask"` and no saved decision, trust-requiring project resources are ignored.

Pi core has no programmatic approval channel for native tool calls because there is no native approval system. Extension-provided approval channels are extension-specific.

## Changelog

- 2026-07-03: Refreshed against current Pi source and docs. Replaced legacy `os: all` config metadata with schema-valid per-OS records. Confirmed Pi core remains permissive by default with no native permission prompts, no explicit YOLO switch, no built-in MCP, no native sandbox, and no managed policy layer.
- 2026-07-03: Corrected default tool description: Pi defaults to `read`, `bash`, `edit`, and `write`; `grep`, `find`, and `ls` are built-in read-only tools but are not in the default active set.
- 2026-07-03: Added extension hook semantics, headless behavior, and the current optional `pi-permission-system` package as non-core context.
- 2026-07-02: Prior research reported Pi v0.73.1 behavior, documented tool visibility switches, project trust, and lack of native permissions.

## Sources

- [Pi README, Permissions & Containerization](https://github.com/earendil-works/pi#permissions--containerization)
- [Pi Security docs](https://pi.dev/docs/latest/security)
- [Pi Containerization docs](https://pi.dev/docs/latest/containerization)
- [Pi Extensions docs](https://pi.dev/docs/latest/extensions)
- [Pi Usage docs](https://pi.dev/docs/latest/usage)
- [Pi Settings docs](https://pi.dev/docs/latest/settings)
- [Pi package catalog: pi-permission-system](https://pi.dev/packages/pi-permission-system)
- [Pi source: CLI args](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/cli/args.ts)
- [Pi source: settings manager](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/settings-manager.ts)
- [Pi source: project trust](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/project-trust.ts)
- [Pi source: SDK tool options](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/sdk.ts)
- [Pi source: AgentSession tool registry](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/agent-session.ts)
- [Pi source: extension event types](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/extensions/types.ts)
- [Pi source: permission-gate example](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/examples/extensions/permission-gate.ts)
- [Pi source: plan-mode example](https://github.com/earendil-works/pi/tree/main/packages/coding-agent/examples/extensions/plan-mode)
- [Pi source: subagent example](https://github.com/earendil-works/pi/tree/main/packages/coding-agent/examples/extensions/subagent)
