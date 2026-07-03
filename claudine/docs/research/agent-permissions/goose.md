---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-01
agent: open_code
model: kimi-for-coding/k2p7

cli_params: []

env_vars:
  - name: GOOSE_MODE
    effect: Controls the session tool-execution mode (auto, approve, smart_approve, chat). Overrides the GOOSE_MODE value in config.yaml.
  - name: GOOSE_ALLOWLIST
    effect: URL to a YAML allowlist of MCP server installation commands. When set, Goose only installs extensions whose command exactly matches an entry in the list.

config_files:
  - os: all
    user: ~/.config/goose/permission.yaml
    repo: ""

precedence:
  - source: environment variables > config.yaml GOOSE_MODE > built-in default
    scope: [permissions]
    merge_strategy: none
    notes: "Previous prose summary: environment variables > config.yaml GOOSE_MODE > built-in default (auto). Goose CLI has no permission-mode CLI flags; extension flags such as --no-profile add or remove tools for the session but do not override mode or permission.yaml rules."

default_posture: "When nothing is configured, Goose CLI starts in auto mode and auto-approves all tool calls (safety inspectors may still block). No per-tool allow/ask/deny rules are configured."

agent_permissions:
  allowed: false

yolo:
  has_interactive_yolo: true
  has_non_interactive_yolo: true
  mechanism: "GOOSE_MODE=auto environment variable, GOOSE_MODE: auto in config.yaml, or the interactive /mode auto slash command"

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

changes: []

requires_claudine_update: true
reason: "Claudine's Goose PolicyEngine backend currently marks all capabilities false and only reports protected config paths. To support Goose permissions properly, the backend needs to parse GOOSE_MODE, model permission.yaml tool levels, handle the SmartApprove LLM cache, and plan mutations."
---

# Goose CLI Permissions

## Introduction to Goose CLI Permissions

Goose CLI controls what an agent can do through two mechanisms:

1. A session-wide **permission mode** (`auto`, `approve`, `smart_approve`, or `chat`).
2. Optional per-tool **permission levels** (`always_allow`, `ask_before`, `never_allow`) stored in `permission.yaml`.

Permissions can be defined through:

- **Configuration files**: `~/.config/goose/config.yaml` holds `GOOSE_MODE`; `~/.config/goose/permission.yaml` holds per-tool levels; `~/.config/goose/permissions/tool_permissions.json` is an auto-managed runtime cache of SmartApprove/approval decisions.
- **Environment variables**: `GOOSE_MODE` and `GOOSE_ALLOWLIST`.
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
       - tree
       - read_image
       - analyze
       - load_skill
     ask_before:
       - shell
       - write
       - edit
       - delegate
       - load
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

Important caveat: non-interactive `approve` or `smart_approve` sessions cannot receive user approval, so they will fail or hang when an unapproved tool is needed. True unattended whitelisting is not supported; for automation you must use `auto` mode, which then ignores tool-level rules.

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

Goose CLI does **not** provide a repo-scoped permission file. The only way to vary behavior per repository is to set `GOOSE_MODE` per shell session or use a recipe/local config workflow.

`permission.yaml` grammar:

```yaml
user:
  always_allow:
    - tree
    - read_image
    - analyze
  ask_before:
    - shell
    - write
    - edit
  never_allow:
    - apps__delete_app
smart_approve:
  always_allow:
    - tree
  ask_before: []
  never_allow: []
```

- Top-level keys are permission categories. `user` stores explicit choices; `smart_approve` stores cached LLM classifications.
- Each category contains three lists of tool names: `always_allow`, `ask_before`, `never_allow`.
- A tool can appear in only one list per category; the last write wins.

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
| `developer` | unprefixed | `write`, `edit`, `shell`, `tree`, `read_image` |
| `analyze` | unprefixed | `analyze` |
| `summon` | unprefixed | `load`, `delegate` |
| `skills` | unprefixed | `load_skill` |
| `todo` | prefixed | `todo__todo_write` |
| `apps` | prefixed | `apps__list_apps`, `apps__create_app`, `apps__iterate_app`, `apps__delete_app` |
| `extensionmanager` | prefixed | `extensionmanager__search_available_extensions`, `extensionmanager__manage_extensions`, `extensionmanager__list_resources`, `extensionmanager__read_resource` |
| `tom` | n/a | (no tools; injects context) |

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

## Sources

- [Goose Permission Modes](https://goose-docs.ai/docs/guides/managing-tools/goose-permissions)
- [Managing Tool Permissions](https://goose-docs.ai/docs/guides/managing-tools/tool-permissions)
- [Configuration Files](https://goose-docs.ai/docs/guides/config-files)
- [Environment Variables](https://goose-docs.ai/docs/guides/environment-variables)
- [Extension Allowlist](https://goose-docs.ai/docs/guides/allowlist)
- [Goose CLI Commands](https://goose-docs.ai/docs/guides/goose-cli-commands)
- [Goose source: `GooseMode`](https://github.com/aaif-goose/goose/blob/main/crates/goose-providers/src/goose_mode.rs)
- [Goose source: `permission.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/config/permission.rs)
- [Goose source: `permission_inspector.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/permission/permission_inspector.rs)
- [Goose source: `platform_extensions/mod.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/agents/platform_extensions/mod.rs)
- [Goose source: `cli.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/cli.rs)
- [Claudine Goose PolicyEngine backend](../../../../lib/src/permissions/providers/goose.rs)
