---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-01
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

env_vars: []

config_files:
  - os: all
    user: ~/.pi/agent/settings.json
    repo: .pi/settings.json

precedence:
  - source: CLI flags > environment variables > project settings > user/global settings
    scope: [permissions]
    merge_strategy: none
    notes: "Previous prose summary: CLI flags > environment variables (where they apply) > project settings (.pi/settings.json) > user/global settings (~/.pi/agent/settings.json)."

default_posture: "When nothing is configured, Pi runs with full access to the launching user's filesystem, shell, network, and credentials. All built-in tools are enabled and no interactive permission prompts or denials are issued."

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
    - "Tool gating is done through CLI allowlists and settings resource loading, not through permission rules."
    - "Project trust gates config loading but does not restrict tool execution."
    - "Extension-based permission gates are arbitrary TypeScript code and are not discoverable as static policy rules."

changes: []
requires_claudine_update: true
reason: "Claudine's PolicyEngine does not include a Pi backend; modeling Pi's all-permissive default, CLI tool allowlists, extension-based gates, and project-trust behavior requires a new backend and canonical rule mapping."

---

# Pi Permissions

## Introduction to Pi Permissions

Pi is a minimal, extensible coding agent harness. It deliberately does **not** ship with a built-in permission system for filesystem, process, network, or credential access. By default, Pi runs with the full permissions of the operating-system user and process that launched it. Any restriction must be imposed by the operating system, a container or sandbox, or by a user-provided extension.

Configuration files do not define execution permissions. `settings.json` controls general agent behavior, resource discovery, and project trust, but it has no `permissions` object and no allow/ask/deny rule grammar. The only native policy-like mechanisms are:

- **Project trust** — decides whether project-local `.pi` resources load.
- **Tool allowlists/denylists** — the `--tools`, `--exclude-tools`, `--no-builtin-tools`, and `--no-tools` CLI flags restrict which tools are exposed to the model.
- **Extensions** — TypeScript modules can subscribe to `tool_call` events and block individual calls.

There are no environment variables that define tool or filesystem permissions. Environment variables such as `PI_CODING_AGENT_DIR` change where configuration is loaded, and `PI_OFFLINE` disables startup network traffic, but neither grants nor denies execution rights.

### CLI parameters and precedence

The permission-adjacent CLI parameters are:

| Parameter | Effect |
| :----- | :----- |
| `--tools <list>` / `-t <list>` | Allowlist tool names for the session |
| `--exclude-tools <list>` / `-xt <list>` | Denylist tool names for the session |
| `--no-builtin-tools` / `-nbt` | Disable built-in tools, keep extension/custom tools |
| `--no-tools` / `-nt` | Disable all tools |
| `--approve` / `-a` | Trust project-local resources for this run |
| `--no-approve` / `-na` | Ignore project-local resources for this run |

Precedence for configuration that Pi does support is:

**CLI flags > environment variables (where they apply) > project settings (`.pi/settings.json`) > user/global settings (`~/.pi/agent/settings.json`).**

Because there is no permission-rule surface, there is no conflict resolution between allow and deny rules. `--tools` and `--exclude-tools` are applied as simple set operations at startup.

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
```

To add interactive confirmation, you must write an extension that listens to `tool_call` events. The [permission-gate.ts example](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/examples/extensions/permission-gate.ts) prompts before `rm -rf`, `sudo`, or `chmod/chown 777` and blocks in non-interactive mode.

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

## MCP and Permissions

Pi does not include built-in MCP support. The documentation explicitly says "No MCP" and recommends building CLI tools as skills or adding MCP through an extension. Because there is no native MCP stack, there are no native MCP permission rules either.

If an extension adds MCP servers, their safety depends entirely on:

- the sandbox or container in which Pi runs,
- any permission-gate extension that blocks risky tool calls,
- the host OS permissions available to the Pi process.

There is no `mcp__<server>__<tool>` allow/ask/deny syntax and no `PolicyEngine` mapping for MCP under Pi.
