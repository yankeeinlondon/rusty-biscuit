---
last_updated: 2026-04-29
has_official_schema: false
---

# OpenCode CLI Logging Research

## Introduction to OpenCode CLI Logging

OpenCode (by Anomaly, [https://opencode.ai](https://opencode.ai)) is an open-source AI coding agent built in TypeScript with a client/server architecture. Its logging and data persistence strategy is **dual-layered**: lightweight plain-text diagnostic log files alongside a SQLite database that stores the structured session, message, and event data.

### Log Locations

OpenCode stores all persistent data under a platform-specific XDG-compatible data directory:

| Platform | Path |
|----------|------|
| macOS | `~/Library/Application Support/opencode/` |
| Linux | `~/.local/share/opencode/` |
| Windows | `%APPDATA%/opencode/` |

The data directory contains the following relevant files and directories:

| Path | Description |
|------|-------------|
| `opencode.db` | SQLite (WAL mode) database holding sessions, messages, parts, events, projects, workspaces, todos, and permissions |
| `opencode.db-wal` / `opencode.db-shm` | SQLite WAL journal files |
| `log/*.log` | Per-invocation plain-text diagnostic logs |
| `storage/` | Legacy JSON file storage (pre-migration; used for session diffs and snapshots) |
| `snapshot/` | Git tree snapshots keyed by SHA |
| `project/` | Legacy project metadata (pre-migration) |
| `tool-output/` | Cached tool execution output |
| `auth.json` | Provider authentication tokens |

### Log File Organization

Diagnostic log files are **per-invocation**. Each time OpenCode starts, it creates a new log file named with an ISO 8601 timestamp:

```
log/2026-04-30T060028.log
log/2026-04-30T055902.log
log/2025-07-02T23:29:03.log
```

There is **no automatic log rotation, archiving, or compaction** of log files. Old log files accumulate indefinitely. The log files are append-only for the lifetime of a single process invocation.

### Log File Format

The log files use a **structured plain-text format** (not JSON, not JSONL). Each line follows this pattern:

```
<LEVEL>  <timestamp> +<elapsed_ms> service=<service_name> <key>=<value> [<key>=<value> ...] <message>
```

Example lines:

```
INFO  2026-04-30T06:00:28 +139ms service=default version=1.14.29 args=["models"] process_role=main run_id=7d50ee35-5e12-4a9d-8fc9-8e58d901c3d5 opencode
INFO  2026-04-30T06:00:28 +1ms service=default directory=/Users/ken/project creating instance
INFO  2026-04-30T06:00:28 +56ms service=db path=/Users/ken/.local/share/opencode/opencode.db opening database
WARN  2026-04-30T06:00:28 +2ms service=config path=/Users/ken/.config/opencode/config.json tui keys in opencode config are deprecated
INFO  2026-04-30T06:00:28 +3ms service=provider providerID=deepseek found
INFO  2026-04-30T06:00:28 +7ms service=bus type=session.updated publishing
```

The log level can be one of: `INFO`, `WARN`, `ERROR`, `DEBUG`.

Key fields observed:

| Field | Description |
|-------|-------------|
| `service` | The subsystem emitting the log (e.g., `db`, `session`, `bus`, `config`, `provider`, `plugin`, `server`, `storage`) |
| `directory` | Working directory context (when applicable) |
| `path` | File path being operated on |
| `type` | Event type for bus messages |
| `providerID` | LLM provider identifier |
| `modelID` | Model identifier |
| `version` | OpenCode version |
| `args` | CLI arguments |
| `process_role` | `main` or `server` |
| `run_id` | UUID for the process run |
| `sessionID` / `session` | Session identifier |

### SQLite Database

OpenCode uses **SQLite with WAL mode** as its primary structured data store. The database is managed through [Drizzle ORM](https://orm.drizzle.team/) with migrations. Key configuration:

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;
PRAGMA cache_size = -64000;
PRAGMA foreign_keys = ON;
```

The database file path defaults to `$XDG_DATA_HOME/opencode/opencode.db` (or a channel-specific variant like `opencode-beta.db` for non-stable installation channels). It can be overridden with the `OPENCODE_DB` environment variable.

### Major Log/Data Types

OpenCode distinguishes the following categories of structured data in its SQLite database:

| Category | Tables | Description |
|----------|--------|-------------|
| **Sessions** | `session` | Conversation sessions with metadata (title, project, timestamps, permissions, diff summaries) |
| **Messages** | `message` | User and assistant messages, keyed by session. The `data` JSON column stores role, model, cost, tokens, etc. |
| **Parts** | `part` | Granular message parts: `text`, `tool`, `reasoning`, `step-start`, `step-finish`, `file`, `snapshot`, `patch`, `compaction`, `subtask`, `agent`, `retry` |
| **Events** | `event`, `event_sequence` | Event-sourced sync events (used when workspaces feature flag is enabled) |
| **Projects** | `project` | Git-based project metadata (worktree, VCS type, name) |
| **Workspaces** | `workspace` | Workspace entries within a project |
| **Todos** | `todo` | Per-session todo items |
| **Permissions** | `permission` | Per-project permission rulesets |
| **Session Entries** | `session_entry` | Alternative normalized event log (v2 schema): user prompts, assistant steps, tool calls, compactions |

Observed part type distribution from a production database:

| Part Type | Count | Description |
|-----------|-------|-------------|
| `tool` | 58,515 | Tool invocations (bash, read, edit, grep, glob, task, etc.) |
| `step-start` | 49,128 | LLM inference step begins |
| `step-finish` | 48,541 | LLM inference step completes (with cost/token accounting) |
| `reasoning` | 41,552 | Chain-of-thought reasoning text |
| `text` | 25,023 | Assistant text output |
| `patch` | 7,268 | File patches applied |
| `file` | 422 | File attachments |
| `snapshot` | 127 | Git tree snapshots |
| `compaction` | 35 | Context compaction events |
| `subtask` | 2 | Sub-agent task spawning |

Tool call distribution:

| Tool | Count |
|------|-------|
| `bash` | 28,894 |
| `read` | 13,933 |
| `edit` | 5,048 |
| `task` | 2,803 |
| `grep` | 2,701 |
| `todowrite` | 1,465 |
| `glob` | 1,346 |
| `apply_patch` | 629 |
| `webfetch` | 513 |

## Logging Schema

### Official Schema Status

OpenCode **does not publish an official standalone schema document** for its log output or database contents. The schema is defined implicitly through:

1. **Drizzle ORM table definitions** in the source code at [`packages/opencode/src/session/session.sql.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/session.sql.ts)
2. **Effect Schema structs** in [`packages/opencode/src/session/session.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/session.ts) and [`packages/opencode/src/session/message-v2.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/message-v2.ts)
3. **Bus event definitions** in [`packages/opencode/src/bus/bus-event.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/bus/bus-event.ts)
4. **Sync event definitions** in [`packages/opencode/src/sync/index.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/sync/index.ts)

The database tables themselves serve as the closest thing to an official schema. They are created and evolved through Drizzle migrations in `packages/opencode/migration/`.

### Representative Rust Schema

Based on analysis of the source code and production database data, the following Rust types model OpenCode's structured logging data:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    #[serde(rename = "user")]
    User(UserMessage),
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub id: String,
    pub session_id: String,
    pub time: MessageTime,
    #[serde(default)]
    pub agent: String,
    #[serde(default)]
    pub model: Option<ModelRef>,
    #[serde(default)]
    pub format: Option<OutputFormat>,
    #[serde(default)]
    pub system: Option<String>,
    #[serde(default)]
    pub tools: Option<std::collections::HashMap<String, bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub id: String,
    pub session_id: String,
    pub time: AssistantTime,
    pub parent_id: String,
    pub model_id: String,
    pub provider_id: String,
    pub agent: String,
    #[serde(default)]
    pub mode: String,
    pub cost: f64,
    pub tokens: TokenUsage,
    #[serde(default)]
    pub error: Option<AssistantError>,
    #[serde(default)]
    pub path: Option<WorkPath>,
    #[serde(default)]
    pub finish: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    #[serde(default)]
    pub total: Option<u64>,
    pub input: u64,
    pub output: u64,
    pub reasoning: u64,
    pub cache: CacheUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheUsage {
    pub read: u64,
    pub write: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRef {
    pub provider_id: String,
    pub model_id: String,
    #[serde(default)]
    pub variant: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkPath {
    pub cwd: String,
    pub root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTime {
    pub created: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantTime {
    pub created: u64,
    #[serde(default)]
    pub completed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Part {
    #[serde(rename = "text")]
    Text(TextPart),
    #[serde(rename = "tool")]
    Tool(ToolPart),
    #[serde(rename = "reasoning")]
    Reasoning(ReasoningPart),
    #[serde(rename = "step-start")]
    StepStart(StepStartPart),
    #[serde(rename = "step-finish")]
    StepFinish(StepFinishPart),
    #[serde(rename = "file")]
    File(FilePart),
    #[serde(rename = "snapshot")]
    Snapshot(SnapshotPart),
    #[serde(rename = "patch")]
    Patch(PatchPart),
    #[serde(rename = "compaction")]
    Compaction(CompactionPart),
    #[serde(rename = "subtask")]
    Subtask(SubtaskPart),
    #[serde(rename = "agent")]
    Agent(AgentPart),
    #[serde(rename = "retry")]
    Retry(RetryPart),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPart {
    pub id: String,
    pub session_id: String,
    pub message_id: String,
    pub text: String,
    #[serde(default)]
    pub synthetic: Option<bool>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPart {
    pub id: String,
    pub session_id: String,
    pub message_id: String,
    pub call_id: String,
    pub tool: String,
    pub state: ToolState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum ToolState {
    #[serde(rename = "pending")]
    Pending { input: serde_json::Value, raw: String },
    #[serde(rename = "running")]
    Running {
        input: serde_json::Value,
        #[serde(default)]
        title: Option<String>,
    },
    #[serde(rename = "completed")]
    Completed {
        input: serde_json::Value,
        output: String,
        title: String,
        metadata: serde_json::Value,
        time: ToolTime,
    },
    #[serde(rename = "error")]
    Error {
        input: serde_json::Value,
        error: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolTime {
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningPart {
    pub id: String,
    pub session_id: String,
    pub message_id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepStartPart {
    pub id: String,
    pub session_id: String,
    pub message_id: String,
    #[serde(default)]
    pub snapshot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepFinishPart {
    pub id: String,
    pub session_id: String,
    pub message_id: String,
    pub reason: String,
    pub cost: f64,
    pub tokens: TokenUsage,
    #[serde(default)]
    pub snapshot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project_id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub slug: String,
    pub directory: String,
    pub title: String,
    pub version: String,
    pub time: SessionTime,
    #[serde(default)]
    pub summary: Option<SessionSummary>,
    #[serde(default)]
    pub share: Option<SessionShare>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTime {
    pub created: u64,
    pub updated: u64,
    #[serde(default)]
    pub compacting: Option<u64>,
    #[serde(default)]
    pub archived: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub additions: u64,
    pub deletions: u64,
    pub files: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionShare {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AssistantError {
    Auth { name: String, provider_id: String, message: String },
    Api { name: String, message: String, status_code: Option<u64>, is_retryable: bool },
    Unknown { name: String, message: String },
    Aborted { name: String, message: String },
    ContextOverflow { name: String, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "json_schema")]
    JsonSchema { schema: serde_json::Value },
}
```

### Database Table Schema (SQL)

The actual SQLite DDL for the core tables:

```sql
CREATE TABLE session (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    workspace_id TEXT,
    parent_id TEXT,
    slug TEXT NOT NULL,
    directory TEXT NOT NULL,
    path TEXT,
    title TEXT NOT NULL,
    version TEXT NOT NULL,
    share_url TEXT,
    summary_additions INTEGER,
    summary_deletions INTEGER,
    summary_files INTEGER,
    summary_diffs TEXT,
    revert TEXT,
    permission TEXT,
    time_created INTEGER NOT NULL,
    time_updated INTEGER NOT NULL,
    time_compacting INTEGER,
    time_archived INTEGER
);

CREATE TABLE message (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES session(id) ON DELETE CASCADE,
    time_created INTEGER NOT NULL,
    time_updated INTEGER NOT NULL,
    data TEXT NOT NULL
);

CREATE TABLE part (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL REFERENCES message(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL,
    time_created INTEGER NOT NULL,
    time_updated INTEGER NOT NULL,
    data TEXT NOT NULL
);

CREATE TABLE event (
    id TEXT PRIMARY KEY,
    aggregate_id TEXT NOT NULL REFERENCES event_sequence(aggregate_id) ON DELETE CASCADE,
    seq INTEGER NOT NULL,
    type TEXT NOT NULL,
    data TEXT NOT NULL
);
```

The `message.data` and `part.data` columns store JSON blobs with the shape described in the Rust schema above.

## Informational Content versus Hook Events

### When File-System Logs Are a Better Source

**Diagnostic log files** (`log/*.log`) are useful for:

- **Debugging startup failures** — provider discovery, config loading, plugin initialization, migration errors
- **Performance analysis** — the `+NNms` elapsed time field tracks time between log lines within a single invocation
- **Server lifecycle events** — HTTP request/response timing, SSE connections, PTY creation
- **Crash forensics** — the last log file before a crash contains the final state
- **Process-level context** — `run_id`, `process_role`, `version`, and CLI `args` are only in log files

**SQLite database** (`opencode.db`) is the authoritative source for:

- **Complete conversation history** — every user prompt and assistant response with full fidelity
- **Token and cost accounting** — per-step cost/token breakdowns in `step-finish` parts
- **Tool call audit trails** — tool names, inputs, outputs, timing, and status (pending/running/completed/error)
- **Session lifecycle** — creation, updates, archival, compaction, forking, deletion
- **Cross-session analytics** — aggregate metrics across all sessions, projects, and time ranges
- **Code change tracking** — snapshot and patch parts record file diffs applied during sessions

### When Hook/Plugin Events Are a Better Source

OpenCode's **plugin event system** (exposed through `@opencode-ai/plugin`) is better for:

- **Real-time notification** — `session.idle`, `session.error`, `tui.toast.show` fire immediately when something happens
- **Permission enforcement** — `permission.ask` hook allows programmatic allow/deny decisions
- **Tool call interception** — `tool.execute.before` can modify or block tool calls before execution
- **Live message observation** — `message.updated`, `message.part.delta` provide streaming text deltas
- **System prompt modification** — `experimental.chat.system.transform` can inject custom instructions
- **User interaction tracking** — `tui.command.execute`, `tui.session.select` capture TUI interactions

Hook events are **transient** — they are only available during the session lifetime and are not persisted (except indirectly through the sync event system that writes them to SQLite).

### Additional Enrichment Sources

1. **OpenCode HTTP API** — OpenCode runs a local HTTP server (visible in log files as `method=GET/POST path=/...`). The API exposes endpoints like `/session_create`, `/session_messages`, `/provider_list`, `/event` (SSE stream), and `/file_search`. Querying this API during a live session provides real-time state that is richer than file logs alone.

2. **`OPENCODE_CONFIG_CONTENT` environment variable** — The inline config override can inject provider configuration and permissions, which affect logging behavior.

3. **Session share URLs** — Sessions can be shared via `session.share`, generating a URL that contains a serialized view of the session. The `share_url` is stored in the `session` table.

4. **Tool output cache** — The `tool-output/` directory contains cached tool execution results, providing the full output for large tool calls that may be truncated in the database after compaction.

5. **Git snapshot data** — The `snapshot/` directory stores Git tree objects referenced by `snapshot` and `step-start` parts, enabling reconstruction of the file system state at any point in a session.

6. **GlobalBus** — The `GlobalBus` EventEmitter in the server process bridges project-scoped bus events to a global scope, emitting events with `{directory, project, workspace, payload}`. This is the mechanism by which the TUI and desktop app receive cross-project notifications.

## Sources

- [OpenCode GitHub Repository](https://github.com/anomalyco/opencode)
- [OpenCode Documentation](https://opencode.ai/docs)
- [OpenCode Plugin Documentation](https://opencode.ai/docs/plugins)
- [OpenCode Configuration Documentation](https://opencode.ai/docs/config)
- [Session SQL Schema (source)](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/session.sql.ts)
- [Session Module (source)](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/session.ts)
- [Message V2 Module (source)](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/message-v2.ts)
- [Database Module (source)](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/storage/db.ts)
- [Storage Module (source)](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/storage/storage.ts)
- [Bus Module (source)](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/bus/index.ts)
- [Bus Event Definitions (source)](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/bus/bus-event.ts)
- [Sync Event System (source)](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/sync/index.ts)
- [Global Bus (source)](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/bus/global.ts)
- [Session Entry V2 (source)](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/v2/session-entry.ts)
- [OpenCode npm Package](https://www.npmjs.com/package/opencode-ai)
- [OpenCode Plugin Package](https://www.npmjs.com/package/@opencode-ai/plugin)
- [Drizzle ORM](https://orm.drizzle.team/)
