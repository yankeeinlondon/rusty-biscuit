# Agent Configuration Management

How Claudine registers itself with each agentic CLI's hook system and manages configuration across providers in a non-destructive manner.

---

## Table of Contents

1. [Problem Statement](#problem-statement)
2. [Provider Configuration Inventory](#provider-configuration-inventory)
3. [Non-Destructive Configuration Strategy](#non-destructive-configuration-strategy)
4. [Per-Provider Configuration Details](#per-provider-configuration-details)
5. [Shared Configuration Interface](#shared-configuration-interface)
6. [Wrapping an Agent (`claudine start`)](#wrapping-an-agent)
7. [Configuration Scope](#configuration-scope)
8. [Configuration Initialization (`init` Command)](#configuration-initialization)
9. [Configuration Maintenance Operations](#configuration-maintenance-operations)
10. [Error Handling and Edge Cases](#error-handling-and-edge-cases)

---

## Problem Statement

Each agentic CLI has its own configuration format, file location, and hook registration mechanism. Claudine must:

1. **Register** itself as a hook handler in each agent's config without clobbering existing user configuration
2. **Deregister** cleanly, restoring the config to its pre-Claudine state
3. **Detect** which agents are installed on the system
4. **Update** its registrations when the user changes their `~/.hooker` event bindings (e.g., adding or removing events)
5. **Handle** config format differences (JSON vs TOML, stdin vs argv, hooks vs plugins)

---

## Provider Configuration Inventory

### File Locations

| Provider | Format | User Config | Project Config | Config Home Env Var |
|---|---|---|---|---|
| Claude Code | JSON | `~/.claude/settings.json` | `.claude/settings.json` | — |
| Codex CLI | TOML | `~/.codex/config.toml` | `.codex/config.toml` | `CODEX_HOME` |
| Gemini CLI | JSON | `~/.gemini/settings.json` | `.gemini/settings.json` | — |
| OpenCode | JSON | `~/.config/opencode/opencode.json` | `opencode.json` | `OPENCODE_CONFIG` |
| Roo Code | JSON | `~/.config/roo/` (limited) | `.roo/` | — |

### Hook Registration Mechanism

| Provider | Mechanism | Event Delivery | Registration Target |
|---|---|---|---|
| Claude Code | `hooks` key in settings JSON | JSON on stdin | `hooks.*` entries pointing to `claudine handle <event>` |
| Codex CLI | `notify` key in config TOML | JSON as last argv argument | `notify` array pointing to `claudine handle` |
| Gemini CLI | `hooks` key in settings JSON | JSON on stdin | `hooks.*` entries pointing to `claudine handle <event>` |
| OpenCode | Plugin system + experimental hooks | Plugin API / subprocess | `.opencode/plugin/claudine-bridge.ts` bridge file |
| Roo Code | Stream consumption (no native hooks) | NDJSON on stdout | Wrapper script via `claudine start roo` only |

### Agent Detection

Claudine detects installed agents by checking for their binaries on `$PATH` and their configuration directories:

| Provider | Binary Name | Config Directory Exists |
|---|---|---|
| Claude Code | `claude` | `~/.claude/` |
| Codex CLI | `codex` | `~/.codex/` |
| Gemini CLI | `gemini` | `~/.gemini/` |
| OpenCode | `opencode` | `~/.config/opencode/` |
| Roo Code | `roo` | `~/.config/roo/` or `.roo/` in any recent project |

Detection uses `sniff_lib`'s program discovery for binary lookup and direct filesystem checks for config directories. An agent is considered "available" if either the binary exists on PATH or the config directory exists.

---

## Non-Destructive Configuration Strategy

The central constraint: Claudine must modify agent config files without losing or corrupting user settings that are unrelated to Claudine.

### Core Principles

1. **Read-Modify-Write with JSON/TOML preservation** — Parse the config, modify only the Claudine-owned keys, serialize back. Use `serde_json` with `Value` merging (not struct deserialization) to preserve unknown keys, comments (for TOML via `toml_edit`), and formatting.

2. **Claudine-owned markers** — All hook entries Claudine creates are tagged so they can be identified for updates and removal. For JSON configs, hooks include a `"__claudine": true` metadata marker in the command string or as a comment convention. For TOML, a `# managed by claudine` comment is placed above managed lines.

3. **Backup before write** — Before modifying any agent's config file, Claudine writes a backup to `~/.claudine/backups/<provider>/<timestamp>.bak`. The backup includes the full file contents and a SHA-256 hash for integrity verification.

4. **Atomic writes** — Write to a temp file in the same directory, then rename. This prevents partial writes from corrupting the config if Claudine crashes or is killed mid-write.

5. **Conflict detection** — Before writing, re-read the file and compare against our last-known state. If the file has been modified externally since our last write, warn the user and offer to merge or abort.

### Identification Convention

All Claudine-managed hook commands use the claudine binary with a consistent prefix pattern:

```
claudine handle <event_name>
```

This makes identification unambiguous. When scanning an agent's config, any hook command starting with `claudine handle` or `claudine ` is considered Claudine-managed. No separate metadata markers are needed because the command string itself is the identifier.

---

## Per-Provider Configuration Details

### Claude Code

**Config file:** `~/.claude/settings.json` (user) or `.claude/settings.json` (project)

**Native structure:**

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "claudine handle session_start",
            "timeout": 30
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "claudine handle before_tool",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
```

**Registration approach:**

1. Read `settings.json` as `serde_json::Value`
2. Navigate to `hooks` key (create if absent)
3. For each enabled `AgenticEvent` in the user's `~/.hooker`:
   - Map to the Claude-native event name (e.g., `BeforeTool` -> `"PreToolUse"`)
   - Check if a Claudine-managed matcher group already exists (command contains `claudine handle`)
   - If exists: update the command and timeout
   - If not: append a new matcher group to the event's array
4. For events that are NOT in `~/.hooker` but have Claudine entries in the config: remove them
5. Write back

**Event name mapping (Claudine -> Claude native):**

| AgenticEvent | Claude Hook Event |
|---|---|
| `session_start` | `SessionStart` |
| `session_end` | `SessionEnd` |
| `before_prompt` | `UserPromptSubmit` |
| `before_tool` | `PreToolUse` |
| `after_tool` | `PostToolUse` |
| `tool_error` | `PostToolUseFailure` |
| `permission_request` | `PermissionRequest` |
| `turn_complete` | `Stop` |
| `subagent_start` | `SubagentStart` |
| `subagent_stop` | `SubagentStop` |
| `before_compact` | `PreCompact` |
| `notification` | `Notification` |

**Non-destructive details:**

- Matcher groups belonging to other tools are left untouched
- If an event key (e.g., `"PreToolUse"`) already has entries from other tools, Claudine appends its matcher group to the array rather than replacing it
- The matcher is left as `""` (match all) because filtering is done inside Claudine based on the `~/.hooker` config

**Timeouts:** Default 30 seconds for observe-only actions (Speak, SoundEffect, Log). Since Claudine's actions are all non-blocking from the agent's perspective (TTS and sound effects run async), 30s is generous. If a user's action config includes `Log` with a remote server target, the timeout is raised to 60s.

---

### Codex CLI

**Config file:** `~/.codex/config.toml` (user) or `.codex/config.toml` (project)

**Native structure:**

```toml
# Existing user config
model = "gpt-5.2-codex"
approval_policy = "on-failure"

# Claudine-managed (added by claudine)
notify = ["claudine", "handle"]
```

**Registration approach:**

1. Read `config.toml` using `toml_edit` (preserves formatting and comments)
2. Check current `notify` value
3. If `notify` is absent or not pointing to Claudine: set it
4. If `notify` already points to a non-Claudine command: warn the user that Codex only supports a single `notify` target and offer to replace or skip

**Critical limitation:** Codex CLI only supports **one** `notify` target. If the user already has a custom notify script, Claudine cannot simply append — it must either:
- Replace the existing notify (with user consent and backup)
- Create a wrapper script at `~/.claudine/codex-notify-wrapper.sh` that calls both the original notify command and Claudine
- Skip Codex registration and inform the user

The wrapper approach is preferred:

```bash
#!/usr/bin/env bash
# Auto-generated by claudine. Original notify target preserved.
# Original: notify-send "Codex"
notify-send "Codex" "$1" &
claudine handle "$1" &
wait
```

**Event delivery difference:** Codex passes JSON as the **last CLI argument** (not stdin). The `claudine handle` subcommand detects this by checking if stdin is a TTY and falling back to reading `argv[last]` as JSON.

**Supported events:** Codex's external `notify` hook currently only fires for `agent-turn-complete`, mapping to `AgenticEvent::TurnComplete`. Other events are only available via the JSONL stream in non-interactive mode (covered in the [wrapper section](#wrapping-an-agent)).

---

### Gemini CLI

**Config file:** `~/.gemini/settings.json` (user) or `.gemini/settings.json` (project)

**Native structure:**

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup",
        "hooks": [
          {
            "name": "claudine-session-start",
            "type": "command",
            "command": "claudine handle session_start",
            "timeout": 30000,
            "description": "Claudine event handler"
          }
        ]
      }
    ],
    "BeforeTool": [
      {
        "matcher": "",
        "hooks": [
          {
            "name": "claudine-before-tool",
            "type": "command",
            "command": "claudine handle before_tool",
            "timeout": 30000,
            "description": "Claudine event handler"
          }
        ]
      }
    ]
  }
}
```

**Registration approach:** Nearly identical to Claude Code, with these differences:

- Gemini hooks have a `name` field — Claudine uses `"claudine-<event_name>"` as the name for identification
- Gemini timeouts are in **milliseconds** (Claude uses seconds)
- Gemini hooks have an optional `description` field — set to `"Claudine event handler"`
- Gemini supports runtime hook management via `/hooks disable <name>`, so our naming convention matters

**Event name mapping (Claudine -> Gemini native):**

| AgenticEvent | Gemini Hook Event |
|---|---|
| `session_start` | `SessionStart` |
| `session_end` | `SessionEnd` |
| `before_prompt` | `BeforeAgent` |
| `before_tool` | `BeforeTool` |
| `after_tool` | `AfterTool` |
| `turn_complete` | `AfterAgent` |
| `before_model` | `BeforeModel` |
| `after_model` | `AfterModel` |
| `before_compact` | `PreCompress` |
| `notification` | `Notification` |

**Non-destructive details:** Same array-append strategy as Claude. Gemini deduplicates hooks with identical `name` and `command` across config layers, so our consistent naming prevents duplicates when both user and project configs register the same event.

---

### OpenCode

**Config file:** `~/.config/opencode/opencode.json` (user) or `opencode.json` (project)

**Native structure:** OpenCode's primary hook mechanism is its **plugin system** (TypeScript/JavaScript modules), not JSON config hooks. The experimental `hook` config only supports `file_edited` and `session_completed`.

**Registration approach (dual strategy):**

#### Strategy A: Plugin Bridge (Primary)

Create a bridge plugin file that OpenCode loads and that invokes Claudine via subprocess:

**File:** `~/.config/opencode/plugin/claudine-bridge.ts`

```typescript
import type { Plugin } from "@opencode-ai/plugin"
import { execSync, execFile } from "child_process"

export default (async ({ client, project }) => {
  return {
    event: async ({ event }) => {
      const mapping: Record<string, string> = {
        "session.created": "session_start",
        "session.deleted": "session_end",
        "session.idle": "turn_complete",
        "session.error": "turn_error",
        "session.compacted": "before_compact",
        "permission.asked": "permission_request",
      }
      const claudineEvent = mapping[event.type]
      if (claudineEvent) {
        const payload = JSON.stringify({
          provider: "opencode",
          event_type: event.type,
          properties: event.properties,
          cwd: project.worktree,
        })
        execFile("claudine", ["handle", claudineEvent], {
          input: payload,
          timeout: 30000,
        })
      }
    },

    "tool.execute.before": async (input, output) => {
      const payload = JSON.stringify({
        provider: "opencode",
        event_type: "tool.execute.before",
        tool_name: input.tool,
        tool_input: output.args,
        session_id: input.sessionID,
      })
      execFile("claudine", ["handle", "before_tool"], {
        input: payload,
        timeout: 30000,
      })
    },

    "tool.execute.after": async (input, output) => {
      const payload = JSON.stringify({
        provider: "opencode",
        event_type: "tool.execute.after",
        tool_name: input.tool,
        tool_input: input.args,
        tool_response: output.output,
        session_id: input.sessionID,
      })
      execFile("claudine", ["handle", "after_tool"], {
        input: payload,
        timeout: 30000,
      })
    },

    "chat.message": async ({}, { message }) => {
      const payload = JSON.stringify({
        provider: "opencode",
        event_type: "chat.message",
        prompt: message,
      })
      execFile("claudine", ["handle", "before_prompt"], {
        input: payload,
        timeout: 30000,
      })
    },
  }
}) satisfies Plugin
```

#### Strategy B: Experimental Config Hooks (Supplementary)

For the limited events available via config:

```json
{
  "experimental": {
    "hook": {
      "file_edited": [
        {
          "command": ["claudine", "handle", "after_tool"],
          "environment": {}
        }
      ],
      "session_completed": [
        {
          "command": ["claudine", "handle", "turn_complete"],
          "environment": {}
        }
      ]
    }
  }
}
```

**Registration steps:**

1. Generate the bridge plugin file at the appropriate scope (user or project)
2. Register the plugin in `opencode.json` if not already listed:
   ```json
   { "plugin": ["claudine-bridge"] }
   ```
3. Optionally add experimental config hooks for `file_edited` and `session_completed`

**Non-destructive details:**
- The plugin file is standalone — it doesn't modify any existing plugins
- The `plugin` array in `opencode.json` is appended to, not replaced
- Removal: delete the bridge file and remove `"claudine-bridge"` from the `plugin` array
- The experimental hooks are additive (arrays can have multiple entries)

**Caveat:** OpenCode plugins run in-process as TypeScript. The bridge plugin spawns Claudine as a subprocess, introducing a small latency overhead. For observe-only actions (sound, speech, logging) this is acceptable.

---

### Roo Code

**Config directory:** `~/.config/roo/` (user) or `.roo/` (project)

**Native structure:** Roo Code CLI has **no hook system**. It provides a `--output-format stream-json` flag that emits NDJSON events to stdout, but there is no configuration key to register external programs as event handlers.

**Registration approach:** Roo Code cannot be configured for hooks via config files. The only option is the [wrapper approach](#wrapping-an-agent) described below.

For project-scoped integration, Claudine can register itself as an MCP server in `.roo/mcp.json`, but MCP tools are agent-callable functions, not lifecycle hooks — they serve a different purpose.

**What Claudine does for Roo Code:**

1. **No config file modification** — there is nothing to modify
2. **`claudine start roo` wrapper** — this is the only way to receive Roo Code events (see [wrapper section](#wrapping-an-agent))
3. **Future-proofing** — Roo Code has an open feature request for hooks (Discussion #6147). When/if shipped, Claudine adds a proper adapter.

---

## Shared Configuration Interface

All per-provider config manipulation is abstracted behind a single Rust trait:

```rust
use std::path::Path;

/// Manages hook registration in a specific agent's configuration files.
///
/// Each provider implements this trait to handle its native config format
/// and hook registration semantics. All implementations must follow the
/// non-destructive principles: read-modify-write, backup before write,
/// atomic file operations.
pub trait AgentConfigurator: Send + Sync {
    /// Which provider this configurator manages.
    fn provider(&self) -> Provider;

    /// Check if this agent is installed and available on the system.
    ///
    /// Returns `true` if either the agent binary is on PATH or
    /// the agent's config directory exists.
    fn is_available(&self) -> bool;

    /// Return the path to the user-scoped config file for this agent.
    fn user_config_path(&self) -> PathBuf;

    /// Return the path to the project-scoped config file for this agent,
    /// given the project root directory.
    fn project_config_path(&self, project_root: &Path) -> PathBuf;

    /// Read the current hook registrations from the config file.
    ///
    /// Returns a list of `AgenticEvent` values that currently have
    /// Claudine-managed hooks registered.
    fn read_registrations(&self, config_path: &Path)
        -> Result<Vec<AgenticEvent>>;

    /// Register Claudine hooks for the given events.
    ///
    /// Performs a non-destructive read-modify-write of the config file.
    /// Creates a backup before writing.
    ///
    /// ## Errors
    ///
    /// Returns an error if the config file cannot be read, parsed,
    /// or written, or if there is a conflict that requires user input.
    fn register(
        &self,
        config_path: &Path,
        events: &[AgenticEvent],
    ) -> Result<RegistrationResult>;

    /// Remove all Claudine-managed hooks from the config file.
    ///
    /// Restores the config to its pre-Claudine state.
    fn deregister(&self, config_path: &Path) -> Result<()>;

    /// Check if the config file has been externally modified since
    /// our last write.
    fn has_external_changes(&self, config_path: &Path) -> Result<bool>;
}

/// Outcome of a registration operation.
pub struct RegistrationResult {
    /// Events that were successfully registered.
    pub registered: Vec<AgenticEvent>,

    /// Events that were skipped with reasons.
    pub skipped: Vec<(AgenticEvent, SkipReason)>,

    /// Whether a backup was created.
    pub backup_path: Option<PathBuf>,

    /// Warnings for the user (e.g., "Codex only supports TurnComplete").
    pub warnings: Vec<String>,
}

/// Why an event was not registered for a specific provider.
pub enum SkipReason {
    /// The provider does not support this event natively.
    Unsupported,

    /// A conflicting non-Claudine hook exists and user chose not to replace.
    Conflict(String),

    /// The event was already registered and up-to-date.
    AlreadyRegistered,
}
```

### Implementation Plan

| Provider | Config Crate | Read/Write Strategy |
|---|---|---|
| Claude Code | `serde_json` | `Value`-level merge, preserve unknown keys |
| Codex CLI | `toml_edit` | Document-level edit, preserve formatting + comments |
| Gemini CLI | `serde_json` | `Value`-level merge (same approach as Claude) |
| OpenCode | `serde_json` + file write | JSON merge for config + write bridge plugin file |
| Roo Code | — | No config modification (wrapper-only) |

---

## Wrapping an Agent

### The Proposal

```sh
claudine start claude|codex|gemini|opencode|roo
```

This command would launch the agent as a child process, intercept its I/O streams, and fire Claudine events based on observed activity.

### Analysis

#### Benefits

1. **Roo Code support** — This is the **only** way to receive Roo Code events. The `roo --output-format stream-json` flag emits NDJSON that Claudine can parse in real-time. Without the wrapper, Roo Code is a dead end.

2. **Codex enrichment** — Codex's `notify` hook only fires `agent-turn-complete`. By wrapping `codex exec --json`, Claudine can observe the full JSONL event stream including `turn.started`, `item.completed` (tool executions), `turn.failed`, and `thread.started`. This unlocks events that are impossible via config alone.

3. **Session lifecycle tracking** — Wrapping gives Claudine precise knowledge of when an agent starts and stops, enabling reliable `SessionStart`/`SessionEnd` events even for providers that don't emit them natively.

4. **Log file access** — Some agents write session logs that are only accessible during the session:
   - Claude Code: `~/.claude/projects/<hash>/transcript.jsonl` (path available in hook payloads, but the wrapper knows the session directory at startup)
   - Codex CLI: `~/.codex/sessions/YYYY/MM/DD/*.jsonl`
   - Gemini CLI: `~/.gemini/tmp/<project_hash>/logs.json`
   - OpenCode: `~/.local/share/opencode/log/`

   A wrapper could tail these logs for richer event data, though this is fragile (log format is not a stable API).

5. **Pre-session environment snapshot** — The wrapper captures the `EnvironmentContext` (via `sniff_lib`) before the agent starts, guaranteeing the snapshot reflects the state the agent will operate in.

#### Costs

1. **TTY complications** — Agentic CLIs are interactive TUI applications. Wrapping them requires proper PTY forwarding to preserve colors, terminal size, cursor movement, and interactive prompts. This is non-trivial (requires `portable-pty` or similar) and is a significant source of bugs.

2. **Signal handling** — Ctrl+C, Ctrl+Z, SIGWINCH (terminal resize) must be forwarded correctly to the child process. Getting this wrong breaks the user experience.

3. **Duplicate event delivery** — If the user has both hook-based registration AND the wrapper running, events fire twice. Claudine must deduplicate.

4. **Adoption friction** — Requiring users to type `claudine start claude` instead of `claude` changes their muscle memory. Shell aliases help but add setup complexity.

5. **Exit code forwarding** — The wrapper must forward the child's exit code to the calling shell.

### Recommendation

**Implement the wrapper, but make it optional and secondary to hook-based registration.**

- For **Claude Code, Gemini CLI, OpenCode**: Hook-based registration is the primary mechanism. The wrapper adds no value that hooks don't already provide.
- For **Codex CLI**: The wrapper is valuable as a supplementary mechanism to unlock events beyond `agent-turn-complete`. Users who want full Codex event coverage use `claudine start codex`.
- For **Roo Code**: The wrapper is **required** — it's the only path to event access.

The wrapper implementation can be deferred to a later phase. The hook-based registration should be built first as it covers 3 of 5 providers completely and Codex partially.

### Wrapper Architecture

```
claudine start <agent>
        │
        ├── Detect agent binary path
        ├── Capture EnvironmentContext (sniff_lib)
        ├── Fire AgenticEvent::SessionStart
        │
        ├── For hook-based agents (claude, gemini, opencode):
        │   └── exec() the agent binary (replace process)
        │       Events handled via registered hooks (no wrapping needed)
        │
        └── For stream-based agents (codex, roo):
            ├── Spawn agent as child process
            ├── For codex: `codex exec --json` with JSONL parsing
            ├── For roo: `roo --output-format stream-json` with NDJSON parsing
            ├── Forward stdin/stdout/stderr with PTY
            ├── Parse event stream → AgenticEvent → dispatch actions
            └── On child exit:
                ├── Fire AgenticEvent::SessionEnd
                └── Forward exit code
```

For hook-based agents, `claudine start` can simply `exec()` the agent after firing `SessionStart` and capturing the environment. No PTY forwarding needed — the agent replaces the Claudine process entirely. The hooks handle events from there. This gives us the pre-session environment snapshot benefit without the PTY complexity.

---

## Configuration Scope

### User Scope (`~/.hooker`)

The primary configuration lives at `~/.hooker` (or `~/.hook-config` as an alternative name for the humor-averse). This file defines:

- Which events to listen for
- What actions to take for each event
- Global settings (TTS voice, default log target)
- Per-provider overrides

See `shared-event-model.md` for the full `HookerConfig` schema.

### Repo Scope (`<repo-root>/.hooker`)

A git repository can include a `.hooker` (or `.hook-config`) file at its root. This enables project-specific event handling.

**Merge behavior:** Repo-scoped config is merged on top of user-scoped config:

```
1. Load ~/.hooker as base config
2. If <cwd>/.hooker exists, load it
3. For each event in repo config:
   a. If event exists in user config: repo config REPLACES user config for that event
   b. If event is new: add it
4. Repo config's `settings` block merges field-by-field (repo wins on conflict)
```

The replacement-not-merge semantic matches the override pattern in `HookerConfig.EventBinding.overrides` — simpler to reason about than deep merging of action arrays.

**Detecting repo root:** Use `git rev-parse --show-toplevel` if in a git repo. If not in a git repo, check the cwd and walk up to the filesystem root looking for `.hooker` or `.hook-config`.

### File Discovery Order

```rust
/// Resolve the Claudine config file path, checking alternative names.
///
/// Checks `~/.hooker` first, then `~/.hook-config`. Returns the first
/// file that exists, or the default `~/.hooker` path if neither exists.
fn resolve_user_config() -> PathBuf {
    let home = dirs::home_dir().expect("HOME not set");
    let primary = home.join(".hooker");
    let alt = home.join(".hook-config");

    if primary.exists() {
        primary
    } else if alt.exists() {
        alt
    } else {
        primary // default creation path
    }
}

/// Resolve the repo-scoped config file, if any.
fn resolve_repo_config(project_root: &Path) -> Option<PathBuf> {
    let primary = project_root.join(".hooker");
    let alt = project_root.join(".hook-config");

    if primary.exists() {
        Some(primary)
    } else if alt.exists() {
        Some(alt)
    } else {
        None
    }
}
```

### Scope Interaction with Agent Config Registration

When Claudine registers hooks with an agent's config files, the **scope of the Claudine config** determines **where** the hooks are registered:

| Claudine Config Scope | Agent Config Scope |
|---|---|
| `~/.hooker` (user) | Agent's user config (e.g., `~/.claude/settings.json`) |
| `<repo>/.hooker` (repo) | Agent's project config (e.g., `.claude/settings.json`) |

This means:

- Running `claudine init` (no project context) writes to `~/.hooker` and registers hooks in agent user configs
- Running `claudine init --repo` (in a git repo) writes to `<repo>/.hooker` and registers hooks in agent project configs
- A user can have both scopes active simultaneously

---

## Configuration Initialization

The `init` command provides an interactive setup experience using the `inquire` crate.

### Flow

```
claudine init [--repo]
```

**Phase 1: Agent Discovery**

```
Scanning for installed agents...

  Found:
    [x] Claude Code    (~/.claude/ exists, claude on PATH)
    [x] Gemini CLI     (~/.gemini/ exists, gemini on PATH)
    [x] Codex CLI      (~/.codex/ exists, codex on PATH)
    [ ] OpenCode       (not found)
    [ ] Roo Code       (not found)

Which agents do you want to configure?
> [x] Claude Code
  [x] Gemini CLI
  [x] Codex CLI
```

Uses `inquire::MultiSelect`. Pre-selects all detected agents. Agents not found on the system are shown but unchecked with a note.

**Phase 2: Event Selection**

```
Which events do you want to hook into?
> [x] Session Start       (agent session begins)
  [x] Session End         (agent session ends)
  [x] Before Prompt       (user submits a prompt)
  [x] Before Tool         (before tool execution)
  [x] After Tool          (after tool completes)
  [x] Tool Error          (tool execution failed)
  [x] Permission Request  (agent needs permission)
  [x] Turn Complete       (agent finished responding)
  [x] Turn Error          (agent turn failed)
  [x] Subagent Start      (subagent spawned)
  [x] Subagent Stop       (subagent finished)
  [x] Before Model        (before LLM API call)
  [x] After Model         (after LLM response)
  [x] Before Compact      (context compaction)
  [x] Notification        (system notification)

(All selected by default. Use space to toggle, enter to confirm.)
```

Uses `inquire::MultiSelect` with all events pre-selected. A note shows which selected events are unsupported by which agents (e.g., "Note: Codex CLI only supports Turn Complete via hooks. Use `claudine start codex` for full coverage.").

**Phase 3: Per-Event Action Configuration**

For each selected event, ask how to handle it:

```
How should "Session Start" be handled?
> [x] Sound Effect
  [ ] Speak (TTS)
  [ ] Log to file
  [ ] Report to terminal

(Multiple selections allowed. Use space to toggle.)
```

If "Sound Effect" is selected:

```
Which sound for "Session Start"?
  power-up (Recommended)
  notification
  beep
  > power-up
```

Uses `inquire::Select` with a recommended default based on the suggested mappings in `shared-event-model.md`.

If "Speak (TTS)" is selected:

```
Message template for "Session Start":
  (Supports {placeholders}: {provider}, {env.branch}, {env.os}, etc.)
  > Session started on {env.branch}
```

Uses `inquire::Text` with a sensible default message.

If "Log to file" is selected:

```
Log file path:
  > ~/.claudine/events.jsonl
```

Uses `inquire::Text` with `~/.claudine/events.jsonl` as default.

**Phase 4: Global Settings**

```
TTS voice preference:
  System default (Recommended)
  Samantha (macOS)
  Custom
  > System default
```

**Phase 5: Write and Register**

```
Writing configuration to ~/.hooker ... done
Registering hooks with Claude Code ... done (12 events)
Registering hooks with Gemini CLI ... done (10 events)
Registering hooks with Codex CLI ... done (1 event via notify, 8 available via `claudine start codex`)

Configuration complete! Run `claudine about` for usage details.
```

### Quick Mode

For users who want sensible defaults without the interview:

```sh
claudine init --quick
```

This:
1. Auto-detects all installed agents
2. Enables all events
3. Sets `TurnComplete` -> SoundEffect (success)
4. Sets `ToolError` -> SoundEffect (error)
5. Sets `PermissionRequest` -> SoundEffect (notification)
6. Sets `SessionStart` -> SoundEffect (power-up)
7. Writes config and registers hooks

### Repo Mode

```sh
claudine init --repo
```

Same interview flow, but:
- Writes to `<repo-root>/.hooker` instead of `~/.hooker`
- Registers in agent project configs instead of user configs
- Requires user config to already exist (prompts to run `claudine init` first if not)
- Offers to add `.hooker` to `.gitignore` or commit it (for team-shared hooks)

---

## Configuration Maintenance Operations

Beyond `init`, Claudine provides commands for ongoing configuration management.

### `claudine sync`

Re-reads `~/.hooker` (and repo `.hooker` if present) and updates all agent config files to match. This is the primary way to apply changes after manually editing `~/.hooker`.

```sh
claudine sync [--dry-run] [--provider claude|codex|gemini|opencode|roo]
```

- `--dry-run`: Show what would change without writing
- `--provider`: Only sync a specific agent

### `claudine status`

Display the current state of all agent registrations:

```
Agent Registrations:

  Claude Code (~/.claude/settings.json)
    Registered events: session_start, before_tool, after_tool, turn_complete (4/15)
    Status: in sync

  Gemini CLI (~/.gemini/settings.json)
    Registered events: session_start, before_tool, after_tool, turn_complete (4/10)
    Status: in sync

  Codex CLI (~/.codex/config.toml)
    Registered events: turn_complete (1/1 available via hooks)
    Status: in sync
    Note: Use `claudine start codex` for full event coverage

  OpenCode (not installed)
  Roo Code (not installed)
```

### `claudine uninstall`

Remove all Claudine hooks from all agent configs and optionally remove `~/.hooker`:

```sh
claudine uninstall [--keep-config]
```

1. Deregister hooks from all agent configs
2. Remove OpenCode bridge plugin if it exists
3. Remove backup directory (`~/.claudine/backups/`)
4. Optionally remove `~/.hooker` (default: keep it)

---

## Error Handling and Edge Cases

### Config File Does Not Exist

If an agent's config file doesn't exist when Claudine tries to register:

- **Claude Code, Gemini CLI:** Create the file with just the `hooks` key. These agents handle missing config files gracefully.
- **Codex CLI:** Create `~/.codex/config.toml` with just the `notify` line.
- **OpenCode:** Create the plugin file and a minimal `opencode.json` if needed.

### Config File Has Syntax Errors

If parsing fails:

1. Log the parse error
2. Do NOT attempt to fix or rewrite the file
3. Report the error to the user: "Could not parse ~/.claude/settings.json: unexpected token at line 42. Please fix the syntax error and run `claudine sync` again."

### Agent Updates Change Config Format

Agent config formats can change between versions. Mitigation:

- Use `Value`-level manipulation (not struct deserialization) so unknown keys are preserved
- Version-check the agent binary when possible (`claude --version`, `gemini --version`)
- If the config format changes in a breaking way, Claudine's adapter for that agent fails gracefully with an actionable error message

### Concurrent Access

Multiple terminal sessions might run `claudine sync` simultaneously:

- Use file locking (`flock` on Linux/macOS, `LockFile` on Windows) when writing any config file
- Lock scope: the specific config file being written, not a global lock
- Lock timeout: 5 seconds, then fail with "Config file locked by another process"

### Claudine Binary Not on PATH

Agent hook commands reference `claudine` by name. If the binary is not on PATH:

- During `init`, check if `claudine` is on PATH
- If not, use the absolute path to the running binary (`std::env::current_exe()`)
- Warn the user: "claudine is not on your PATH. Hooks will use the absolute path /usr/local/bin/claudine. If you move the binary, run `claudine sync` to update."

### Codex Notify Conflict

When Codex already has a non-Claudine `notify` target:

```
Codex CLI already has a notify command configured:
  Current: ["notify-send", "Codex"]

How would you like to proceed?
  Create a wrapper that calls both (Recommended)
  Replace the existing notify target
  Skip Codex CLI
  > Create a wrapper that calls both
```

The wrapper is written to `~/.claudine/codex-notify-wrapper.sh`, made executable, and `config.toml` is updated to point to it. The original notify command is preserved in the wrapper script and in `~/.claudine/backups/codex/`.
