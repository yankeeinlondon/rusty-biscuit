---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7

cli_params:
  - param: tools
    style: switch
    description: Allowlist specific built-in, extension, and custom tools for the session. The model can only use the named tools.
    example: pi --tools read,grep,find,ls -p "Review the code"
    example_description: Limits the session to read-only built-in tools.
  - param: exclude-tools
    style: switch
    description: Disable specific tool names across built-in, extension, and custom tools.
    example: pi --exclude-tools bash -p "Answer without running commands"
    example_description: Removes the bash tool from the session tool set.
  - param: no-builtin-tools
    style: switch
    description: Disable the built-in tools by default while keeping extension and custom tools enabled.
    example: pi --no-builtin-tools -e ./my-tools.ts
    example_description: Starts Pi with only extension-provided tools.
  - param: no-tools
    style: switch
    description: Disable all tools by default.
    example: pi --no-tools -p "Answer from context only"
    example_description: Runs a text-only session with no tool calling.
  - param: no-extensions
    style: switch
    description: Disable auto-discovered extensions. Explicit --extension paths still load.
    example: pi --no-extensions -e ./permission-gate.ts
    example_description: Starts Pi without auto-discovered extensions, loading only the explicit permission gate.
  - param: no-skills
    style: switch
    description: Disable skill discovery and loading.
    example: pi --no-skills -p "Answer without skills"
    example_description: Runs without loading any skills.
  - param: no-prompt-templates
    style: switch
    description: Disable prompt template discovery and loading.
    example: pi --no-prompt-templates -p "Answer without templates"
    example_description: Runs without loading any prompt templates.
  - param: no-themes
    style: switch
    description: Disable theme discovery and loading.
    example: pi --no-themes
    example_description: Runs with the default theme only.
  - param: no-context-files
    style: switch
    description: Disable AGENTS.md and CLAUDE.md context file discovery and loading.
    example: pi --no-context-files -p "Answer generically"
    example_description: Ignores project instruction files for the session.
  - param: approve
    style: switch
    description: Trust project-local files, settings, extensions, and packages for this run.
    example: pi -a -p "Run CI task"
    example_description: Auto-approves project trust for a non-interactive run.
  - param: no-approve
    style: switch
    description: Ignore project-local files, settings, extensions, and packages for this run.
    example: pi -na -p "Answer generically"
    example_description: Declines project trust for the session.

env_vars:
  - name: PI_CODING_AGENT_DIR
    effect: Overrides the configuration directory (default ~/.pi/agent). Changing this affects where settings.json, trust.json, extensions, skills, and sessions are loaded from.
  - name: PI_CODING_AGENT_SESSION_DIR
    effect: Overrides the session storage directory. Has lower precedence than --session-dir.
  - name: PI_OFFLINE
    effect: Disables startup network operations (update checks, package update checks, install telemetry). Security-adjacent because it prevents external network calls during startup.
  - name: PI_TELEMETRY
    effect: Overrides install/update telemetry and provider attribution headers. Does not grant or deny tool permissions.
  - name: PI_PACKAGE_DIR
    effect: Overrides the package installation directory. Affects where installed Pi packages (which may contain extensions) are loaded from.

config_files:
  - os: all
    user: ~/.pi/agent/settings.json
    repo: .pi/settings.json
    notes: Global settings are overridden by project settings. Nested objects merge; scalars replace. There is no permissions key in either file.

precedence:
  - source: cli
    scope: [tools, extensions, skills, prompts, themes, context_files, project_trust]
    merge_strategy: none
    notes: CLI flags are temporary session overrides. --tools/--exclude-tools/--no-tools adjust the active tool set; --approve/--no-approve override project trust for one run.
  - source: environment variables
    scope: [config_paths, startup_network]
    merge_strategy: none
    notes: PI_CODING_AGENT_DIR and PI_CODING_AGENT_SESSION_DIR override settings-derived paths. PI_OFFLINE disables startup network operations.
  - source: project settings
    scope: [settings]
    merge_strategy: shallow
    notes: .pi/settings.json overrides global settings. Nested objects merge; scalars replace.
  - source: user settings
    scope: [settings]
    merge_strategy: shallow
    notes: ~/.pi/agent/settings.json provides baseline values when no project override exists.

default_posture: "When nothing is configured, Pi runs with full access to the launching user's filesystem, shell, network, and credentials. All built-in tools are enabled and no interactive permission prompts or denials are issued."

cli_zero_permissions:
  supported: true
  invocation: "pi --no-tools"
  mechanism: "The --no-tools flag disables all built-in and extension tools for the session, leaving the model with no callable tools."
  limitations: "Extensions loaded via --extension can still register custom tools. To fully lock down tool calling, combine --no-tools with --no-extensions and load only explicitly vetted extensions. Filesystem, network, and process access are still bounded only by the OS user permissions."

agent_permissions:
  allowed: false

yolo:
  has_interactive_yolo: false
  has_non_interactive_yolo: false
  mechanism: "none"

policy_engine:
  ergonomic: false
  provides_coverage: false
  gaps:
    - "Pi has no native permission model, so there is nothing for PolicyEngine to map to its allow/ask/deny axes."
    - "Pi is not one of PolicyEngine's supported backends, so queries would return Unknown."
    - "Tool gating is done through CLI allowlists and extension code, not through static permission rules."
    - "Project trust gates resource loading but does not restrict tool execution."
    - "Extension-based permission gates are arbitrary TypeScript code and are not discoverable as static policy rules."

permission_entities:
  - entity: tool
    native_names: ["--tools", "--exclude-tools", "--no-builtin-tools", "--no-tools"]
    notes: "Tool visibility is controlled through CLI flags that include or exclude tool names from the active set. There are no per-tool allow/ask/deny rules."
  - entity: extension
    native_names: ["--extension", "--no-extensions", "settings.json extensions/packages"]
    notes: "Extensions can register tools and subscribe to tool_call events to block or allow individual calls. Extensions run with full system permissions."
  - entity: workspace
    native_names: ["project trust", "--approve", "--no-approve", "defaultProjectTrust"]
    notes: "Project trust gates whether .pi/settings.json, project extensions, and project packages load. It does not restrict what trusted tools can do."
  - entity: sandbox
    native_names: ["containerization", "Gondolin", "Docker", "OpenShell"]
    notes: "Pi has no built-in sandbox; isolation is achieved by running Pi or its tool execution in external containers or micro-VMs."

approval_modes: []

rule_model:
  decisions: []
  syntax: "none"
  precedence: "none"
  merge_semantics: "none"
  matcher_semantics: "none"
  default_decision: "allow"

tool_visibility:
  supported: true
  mechanisms:
    - "--tools <list> allowlists specific tool names across built-in, extension, and custom tools."
    - "--exclude-tools <list> denies specific tool names across built-in, extension, and custom tools."
    - "--no-builtin-tools disables built-in tools while keeping extension/custom tools."
    - "--no-tools disables all tools by default."
    - "Extensions can register or unregister tools at runtime via pi.registerTool() and pi.setActiveTools()."
  notes: "Tool visibility controls which tools appear in the model's context. It is independent of any approval flow; Pi has no built-in approval prompts."

sandbox:
  supported: false
  modes: []
  backends: []
  filesystem_control: "none"
  network_control: "none"
  notes: "Pi has no native OS-enforced sandbox. The documentation describes three containerization patterns: Gondolin (micro-VM tool routing), plain Docker (whole process isolation), and OpenShell (policy-controlled sandbox). These are external to Pi and must be configured by the user."

trust_and_admin:
  folder_trust: "On interactive startup, Pi asks before trusting a project folder that contains project-local settings, resources, or .agents/skills. Trusting allows .pi/settings.json, .pi resources, and project extensions to load. Decisions are saved in ~/.pi/agent/trust.json. Non-interactive modes use defaultProjectTrust (ask/always/never) unless --approve or --no-approve is passed."
  managed_policy: "none"
  safe_mode: "none"
  notes: "There is no managed/admin policy layer. Project trust only gates resource loading, not tool capabilities."

mcp_permissions:
  supported: false
  server_filters: []
  tool_filters: []
  trust_model: "none"
  notes: "Pi does not include built-in MCP support. MCP can be added only through a custom extension, at which point its security depends entirely on the extension and the host/container in which Pi runs."

headless_behavior: "In non-interactive -p mode and JSON/RPC modes, Pi does not show interactive permission prompts. Project trust dialogs are skipped; defaultProjectTrust or --approve/--no-approve controls whether project resources load. Without an extension that implements confirmation, all available tools execute with the OS user's permissions."

approval_persistence: "Pi has no built-in approval persistence for individual tool calls. Project trust decisions persist in ~/.pi/agent/trust.json until the user changes them."

protected_paths: []

security_posture: "Pi's default security posture is all-permissive: it runs with the launching user's OS permissions and provides no built-in permission prompts, sandbox, or static policy engine. Restrictions must be imposed externally via OS permissions, containers, or user-provided extensions that subscribe to tool_call events."

changes:
  - "Verified current Pi behavior against v0.73.1 local install and pi.dev / GitHub documentation as of 2026-07-02."
  - "Added missing CLI flags observed in --help: --no-extensions/-ne, --no-skills/-ns, --no-prompt-templates/-np, --no-themes, --no-context-files/-nc."
  - "Expanded environment variables list with PI_CODING_AGENT_DIR, PI_CODING_AGENT_SESSION_DIR, PI_OFFLINE, PI_TELEMETRY, and PI_PACKAGE_DIR."
  - "Added detailed sandboxing/containerization coverage (Gondolin, Docker, OpenShell) from current containerization.md."
  - "Documented project trust behavior, trust.json, defaultProjectTrust, and --approve/--no-approve precedence."
  - "Filled all schema-required frontmatter fields introduced by the merged permissions topic."
  - "Confirmed Pi still has no built-in MCP support, no approval modes, no YOLO mode, and no static permission rule grammar."

requires_claudine_update: true
reason: "Claudine's PolicyEngine does not include a Pi backend. Modeling Pi's all-permissive default, CLI tool allowlists/denylists, project-trust behavior, and extension-based permission gates requires a new backend and canonical rule mapping."
---

# Pi Permissions

## Introduction to Pi Permissions

Pi is a minimal, extensible coding agent harness. It deliberately does not ship with a built-in permission system for filesystem, process, network, or credential access. By default, Pi runs with the full permissions of the operating-system user and process that launched it. Any restriction must be imposed by the operating system, a container or sandbox, or by a user-provided extension.

Configuration files do not define execution permissions. `settings.json` controls general agent behavior, resource discovery, and project trust, but it has no `permissions` object and no allow/ask/deny rule grammar. The only native policy-like mechanisms are:

- **Project trust** — decides whether project-local `.pi` resources load.
- **Tool visibility** — the `--tools`, `--exclude-tools`, `--no-builtin-tools`, and `--no-tools` CLI flags restrict which tools are exposed to the model.
- **Extension gates** — TypeScript modules can subscribe to `tool_call` events and block individual calls.

There are no environment variables that define tool or filesystem permissions. Environment variables such as `PI_CODING_AGENT_DIR` change where configuration is loaded, `PI_OFFLINE` disables startup network traffic, and `PI_TELEMETRY` controls telemetry, but none grants or denies execution rights.

### CLI parameters and precedence

The permission-adjacent CLI parameters are:

| Parameter | Effect |
| :----- | :----- |
| `--tools <list>` / `-t <list>` | Allowlist tool names for the session |
| `--exclude-tools <list>` / `-xt <list>` | Denylist tool names for the session |
| `--no-builtin-tools` / `-nbt` | Disable built-in tools, keep extension/custom tools |
| `--no-tools` / `-nt` | Disable all tools |
| `--no-extensions` / `-ne` | Disable auto-discovered extensions |
| `--no-skills` / `-ns` | Disable skill discovery |
| `--no-prompt-templates` / `-np` | Disable prompt template discovery |
| `--no-themes` | Disable theme discovery |
| `--no-context-files` / `-nc` | Disable AGENTS.md/CLAUDE.md discovery |
| `--approve` / `-a` | Trust project-local resources for this run |
| `--no-approve` / `-na` | Ignore project-local resources for this run |

Precedence for configuration that Pi does support is:

**CLI flags > environment variables (where they apply) > project settings (`.pi/settings.json`) > user/global settings (`~/.pi/agent/settings.json`).**

Because there is no permission-rule surface, there is no conflict resolution between allow and deny rules. `--tools` and `--exclude-tools` are applied as simple set operations at startup.

### Permission policy vs tool visibility

Pi does not separate approval policy from tool visibility because it has no approval policy. The CLI flags only control which tools are visible and callable. Once a tool is visible, it runs with the OS user's permissions without further prompting.

## Permissions Use Cases

### Default

If no environment variable, config file, or CLI switch is provided, Pi's effective permissions are the same as the launching user. The model can read any file the user can read, write any file the user can write, run any shell command the user can run, and make outbound network requests if the host allows them. All built-in tools are enabled and no approval prompts are shown.

A Claudine `PolicyEngine` description of this posture is not possible today. `PolicyEngine` models providers with explicit permission axes (allow/ask/deny for read, write, execute, network, MCP, agents), and Pi is not one of its supported backends. Without a Pi backend, `PolicyEngine` would return `Unknown` for every query. Adding Pi support would require treating Pi as a special all-permissive provider and mapping its CLI tool flags into canonical rules.

### Whitelisting

Pi has no built-in whitelisting mode, but you can approximate one with the CLI tool flags:

```bash
# Start in a read-only mode: only read/search/ls tools are available
pi --tools read,grep,find,ls -p "Review the code"

# Disable bash for a session while keeping everything else
pi --exclude-tools bash -p "Explain this file"

# Disable all built-in tools and load only a custom, audited tool set
pi --no-builtin-tools -e ./restricted-tools.ts

# Start with no tools at all, then add back only what is needed
pi --no-tools --tools read -p "Summarize this file"
```

To add interactive confirmation, you must write an extension that listens to `tool_call` events. The [permission-gate.ts example](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/examples/extensions/permission-gate.ts) prompts before `rm -rf`, `sudo`, or `chmod/chown 777` and blocks in non-interactive mode.

The best **CLI-only, session-scoped** way to start with no permissions or no tools is:

```bash
pi --no-tools
```

This disables all built-in and extension tools for the session. You can add back specific built-in tools with `--tools <list>` or load a vetted extension with `--extension <path>`. For a fully locked-down posture that also blocks auto-discovered extensions, combine with `--no-extensions`:

```bash
pi --no-tools --no-extensions -e ./permission-gate.ts -p "Review with gate"
```

`PolicyEngine` cannot describe this use case for Pi without a new backend. The engine has no way to know that `--tools read,grep,find,ls` maps to `can_read(path) -> Allow` and `can_execute(command) -> Deny`, because Pi does not persist that policy in a config file or expose it through a provider-native model.

### YOLO

Pi has no YOLO or bypass-permissions mode. The default behavior is already fully permissive: all tools run without prompting and the agent acts with the user's OS permissions. There is no flag, environment variable, or setting that switches into or out of a "run everything" mode because that is the baseline.

### Root User

Pi does not change its behavior when started as `root` or under `sudo`. It continues to run with the permissions of the launching process, which in this case are elevated. There is no special root block or bypass-permissions refusal. Because there is no YOLO mode, the question of whether it is allowed as root does not apply.

### Configuring the Default

There are no files that configure default execution permissions for Pi. The files that come closest are general settings files:

| Scope | Path | What it controls |
| :----- | :----- | :----- |
| User | `~/.pi/agent/settings.json` | Global settings, resource paths, packages, model defaults |
| Repo | `.pi/settings.json` | Project settings that override global settings |

Neither file has a `permissions` key or a grammar for allow/ask/deny rules. The only trust-related file is `~/.pi/agent/trust.json`, which stores saved project-trust decisions, not tool permissions.

You can influence the tool surface by adding extension paths to `settings.json` under the `extensions` or `packages` keys, but those extensions themselves define the policy in code, not in a declarative permission grammar.

### Extending the Base

Because Pi does not have layered permission policies, you cannot override a user-scope permission rule with a repo-scope rule. The closest analogue is overriding which resources and tools load:

```bash
# User settings might load a global permission-gate extension,
# but the repo can still request a different extension via CLI.
pi --no-extensions -e ./repo/strict-gate.ts -p "run tests"
```

In this example the CLI explicitly disables all auto-discovered extensions and loads only `strict-gate.ts`. That is a resource-loading override, not a permission-rule override.

## Tools and Permissions

Pi's built-in tools are:

| Tool | Gated by permissions? | Notes |
| :----- | :----- | :----- |
| `read` | No | Reads files with the user's OS permissions |
| `write` | No | Creates/overwrites files with the user's OS permissions |
| `edit` | No | Edits files with the user's OS permissions |
| `bash` | No | Runs shell commands with the user's OS permissions |
| `grep` | No | Searches file contents |
| `find` | No | Finds files by name or pattern |
| `ls` | No | Lists directory contents |

Permissions map to tool calls only through:

1. **Tool availability** — `--tools`, `--exclude-tools`, `--no-builtin-tools`, and `--no-tools` include or exclude tools from the active set.
2. **Extension gates** — a `tool_call` handler can block or allow individual calls based on arbitrary logic.
3. **Host permissions** — the underlying OS user determines what files and commands are actually accessible.

There is no native permission-rule grammar. Pi does not support decisions such as `allow`, `ask`, `deny`, `prompt`, or `forbidden` as static configuration. Approval modes, partial modes, and command-pattern matching must be implemented in extension code.

## Sandboxing, Trust, and Administrative Controls

### Sandboxing

Pi has no built-in sandbox. The documentation describes three external containerization patterns:

| Pattern | What is isolated | Best for |
| :----- | :----- | :----- |
| Gondolin extension | Built-in tools and `!` commands | Local micro-VM isolation while keeping auth on host |
| Plain Docker | Whole `pi` process | Simple local isolation |
| OpenShell | Whole `pi` process | Policy-controlled local or remote managed sandbox |

These are not Pi-native controls. They rely on the user configuring a separate isolation layer.

### Trust and administrative controls

**Folder/project trust**: On interactive startup, Pi asks before trusting a project folder that contains project-local settings, resources, or `.agents/skills`. Trusting a project allows `.pi/settings.json`, `.pi` resources, missing package installs, and project extensions to load. Untrusted folders ignore those project-local surfaces. Trust decisions are saved in `~/.pi/agent/trust.json`.

In non-interactive modes (`-p`, `--mode json`, `--mode rpc`), the trust prompt is skipped and `defaultProjectTrust` (`ask`, `always`, `never`) controls fallback behavior. `--approve`/`--no-approve` override this for one run.

There are no managed or admin policy layers.

### Protected paths

Pi does not maintain a provider-reserved protected-path list. The `protected-paths.ts` example extension blocks writes to `.env`, `.git/`, and `node_modules/`, but that is sample code, not a Pi guarantee.

## MCP and Permissions

Pi does not include built-in MCP support. The documentation explicitly says "No MCP" and recommends building CLI tools as skills or adding MCP through an extension. Because there is no native MCP stack, there are no native MCP permission rules either.

If an extension adds MCP servers, their safety depends entirely on:

- the sandbox or container in which Pi runs,
- any permission-gate extension that blocks risky tool calls,
- the host OS permissions available to the Pi process.

There is no `mcp__<server>__<tool>` allow/ask/deny syntax and no `PolicyEngine` mapping for MCP under Pi.

## Non-Interactive Behavior

In non-interactive `-p` mode, JSON mode, and RPC mode, Pi cannot show interactive permission prompts. Project trust dialogs are skipped; `defaultProjectTrust` or `--approve`/`--no-approve` controls whether project resources load. Without an extension that implements confirmation, every visible tool executes with the OS user's permissions. There is no programmatic approval channel.

## Sources

- [Pi website](https://pi.dev/)
- [Pi GitHub repository](https://github.com/earendil-works/pi)
- [Pi coding agent README](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/README.md)
- [Containerization patterns](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/containerization.md)
- [Settings documentation](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/settings.md)
- [Extensions documentation](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/extensions.md)
- [Permission gate example](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/examples/extensions/permission-gate.ts)
- [Protected paths example](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/examples/extensions/protected-paths.ts)

## Changelog

- 2026-07-02: Refreshed research against Pi v0.73.1, pi.dev, and current GitHub documentation. Added missing CLI flags, environment variables, containerization patterns, project trust details, and all schema-required frontmatter fields. Confirmed Pi still has no native permission system, MCP support, or approval modes.
