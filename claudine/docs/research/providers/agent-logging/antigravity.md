---
$schema: ./_schema.yaml
created: 2026-07-08
last_updated: 2026-07-08
agent: codex
model: default
has_official_schema: none

surfaces:
  - role: session_transcript
    path_macos: ~/.gemini/antigravity-cli/brain/{conversation_id}/.system_generated/logs/transcript.jsonl
    path_windows: "%USERPROFILE%\\.gemini\\antigravity-cli\\brain\\{conversation_id}\\.system_generated\\logs\\transcript.jsonl"
    path_linux: ~/.gemini/antigravity-cli/brain/{conversation_id}/.system_generated/logs/transcript.jsonl
    format: jsonl
    scope: per_session
    naming: transcript.jsonl under a conversation UUID brain directory
    rotation: session
    live_locked: false
    schema_versioning: none
    notes: "Observed compact per-session transcript. Each line is a step record with step_index, source, type, status, created_at, and optional content."
  - role: session_transcript
    path_macos: ~/.gemini/antigravity-cli/brain/{conversation_id}/.system_generated/logs/transcript_full.jsonl
    path_windows: "%USERPROFILE%\\.gemini\\antigravity-cli\\brain\\{conversation_id}\\.system_generated\\logs\\transcript_full.jsonl"
    path_linux: ~/.gemini/antigravity-cli/brain/{conversation_id}/.system_generated/logs/transcript_full.jsonl
    format: jsonl
    scope: per_session
    naming: transcript_full.jsonl under a conversation UUID brain directory
    rotation: session
    live_locked: false
    schema_versioning: none
    notes: "Observed full per-session transcript. On the host sample it matched transcript.jsonl, but the name implies a less-truncated variant."
  - role: state_db
    path_macos: ~/.gemini/antigravity-cli/conversations/{conversation_id}.db
    path_windows: "%USERPROFILE%\\.gemini\\antigravity-cli\\conversations\\{conversation_id}.db"
    path_linux: ~/.gemini/antigravity-cli/conversations/{conversation_id}.db
    format: sqlite
    scope: per_session
    naming: one SQLite database per conversation UUID
    rotation: session
    live_locked: true
    schema_versioning: none
    notes: "Observed WAL sidecars. Tables: steps, trajectory_meta, trajectory_metadata_blob, gen_metadata, executor_metadata, battle_mode_infos, parent_references. BLOB columns appear to hold protobuf payloads."
  - role: session_index
    path_macos: ~/.gemini/antigravity-cli/conversation_summaries.db
    path_windows: "%USERPROFILE%\\.gemini\\antigravity-cli\\conversation_summaries.db"
    path_linux: ~/.gemini/antigravity-cli/conversation_summaries.db
    format: sqlite
    scope: global
    naming: conversation_summaries.db
    rotation: none
    live_locked: true
    schema_versioning: none
    notes: "Observed WAL sidecars. Table conversation_summaries indexes conversation_id, title, preview, step_count, last_modified_time, workspace_uris, status, source, parent_conversation_id, last_user_input_time, and app_data_dir."
  - role: prompt_history
    path_macos: ~/.gemini/antigravity-cli/history.jsonl
    path_windows: "%USERPROFILE%\\.gemini\\antigravity-cli\\history.jsonl"
    path_linux: ~/.gemini/antigravity-cli/history.jsonl
    format: jsonl
    scope: global
    naming: history.jsonl
    rotation: none
    live_locked: false
    schema_versioning: none
    notes: "Observed prompt/slash-command history with display, timestamp, workspace, optional conversationId, and optional type."
  - role: app_log
    path_macos: ~/.gemini/antigravity-cli/log/cli-{YYYYMMDD}_{HHMMSS}.log
    path_windows: "%USERPROFILE%\\.gemini\\antigravity-cli\\log\\cli-{YYYYMMDD}_{HHMMSS}.log"
    path_linux: ~/.gemini/antigravity-cli/log/cli-{YYYYMMDD}_{HHMMSS}.log
    format: text
    scope: per_process
    naming: cli-{local-date}_{local-time}.log; ~/.gemini/antigravity-cli/cli.log symlinks to the latest file on macOS/Linux
    rotation: session
    live_locked: false
    schema_versioning: none
    notes: "Observed Go/glog-style language-server bootstrap and backend diagnostics. GitHub issues confirm this path is used for troubleshooting."
  - role: app_log
    path_macos: ~/.gemini/antigravity-cli/crashes/crash_{pid}_{uuid}.log
    path_windows: "%USERPROFILE%\\.gemini\\antigravity-cli\\crashes\\crash_{pid}_{uuid}.log"
    path_linux: ~/.gemini/antigravity-cli/crashes/crash_{pid}_{uuid}.log
    format: text
    scope: per_process
    naming: crash_{pid}_{uuid}.log
    rotation: session
    live_locked: false
    schema_versioning: none
    notes: "Observed crash file path; host sample was empty."
  - role: app_log
    path_macos: ~/Library/Application Support/Antigravity IDE/logs/{YYYYMMDD}T{HHMMSS}/{component}.log
    path_windows: "%APPDATA%\\Antigravity IDE\\logs\\{YYYYMMDD}T{HHMMSS}\\{component}.log"
    path_linux: ~/.config/Antigravity IDE/logs/{YYYYMMDD}T{HHMMSS}/{component}.log
    format: text
    scope: per_process
    naming: per-launch directory with main.log, sharedprocess.log, renderer logs, extension-host logs, telemetry.log, auth.log, terminal.log, and related VS Code-derived component logs
    rotation: session
    live_locked: false
    schema_versioning: none
    notes: "Observed desktop IDE component logs. These are application diagnostics, not agent transcripts."
  - role: app_log
    path_macos: ~/Library/Logs/Antigravity/{main,language_server}.log
    path_windows: "%APPDATA%\\Antigravity\\logs\\{main,language_server}.log"
    path_linux: ~/.config/Antigravity/logs/{main,language_server}.log
    format: text
    scope: global
    naming: main.log and language_server.log
    rotation: none
    live_locked: false
    schema_versioning: none
    notes: "Observed Antigravity hub/Electron and language-server logs. main.log explicitly points operators to language_server.log and Electron logs."
  - role: state_db
    path_macos: ~/Library/Application Support/Antigravity IDE/User/{globalStorage,workspaceStorage/{workspace_id}}/state.vscdb
    path_windows: "%APPDATA%\\Antigravity IDE\\User\\{globalStorage,workspaceStorage\\{workspace_id}}\\state.vscdb"
    path_linux: ~/.config/Antigravity IDE/User/{globalStorage,workspaceStorage/{workspace_id}}/state.vscdb
    format: sqlite
    scope: global
    naming: VS Code-compatible state.vscdb with ItemTable key/value BLOB rows
    rotation: none
    live_locked: false
    schema_versioning: none
    notes: "Observed desktop IDE state DBs. Useful for app state enrichment, but not an agent transcript schema."
  - role: session_transcript
    path_macos: ~/.gemini/antigravity-ide/conversations/{conversation_id}.pb
    path_windows: "%USERPROFILE%\\.gemini\\antigravity-ide\\conversations\\{conversation_id}.pb"
    path_linux: ~/.gemini/antigravity-ide/conversations/{conversation_id}.pb
    format: text
    scope: per_session
    naming: one binary protobuf conversation file per conversation UUID
    rotation: session
    live_locked: false
    schema_versioning: none
    notes: "Observed desktop/shared Antigravity session store. The sidecar schema lacks a protobuf format enum, so this binary protobuf surface is recorded as text with this caveat."

time_fields:
  - surface: session_transcript
    site: $.created_at
    unit: iso8601
    zone: utc
    confidence: observed
  - surface: prompt_history
    site: $.timestamp
    unit: unix_millis
    zone: utc
    confidence: observed
  - surface: app_log
    site: filename (cli-{YYYYMMDD}_{HHMMSS}.log)
    unit: iso8601
    zone: local
    confidence: observed
  - surface: app_log
    site: "CLI log line prefix (I/W/E MMDD HH:MM:SS.microseconds)"
    unit: iso8601
    zone: local
    confidence: inferred
  - surface: app_log
    site: "IDE logs launch directory ({YYYYMMDD}T{HHMMSS})"
    unit: iso8601
    zone: local
    confidence: observed
  - surface: app_log
    site: "IDE component log line prefix"
    unit: iso8601
    zone: local
    confidence: observed
  - surface: app_log
    site: "Hub Electron log line prefix"
    unit: iso8601
    zone: local
    confidence: observed
  - surface: app_log
    site: "Hub language_server glog line prefix (I/W/E MMDD HH:MM:SS.microseconds)"
    unit: iso8601
    zone: local
    confidence: inferred
  - surface: session_index
    site: conversation_summaries.last_modified_time
    unit: iso8601
    zone: unspecified
    confidence: inferred
  - surface: session_index
    site: conversation_summaries.last_user_input_time
    unit: iso8601
    zone: unspecified
    confidence: inferred
  - surface: state_db
    site: "filename ({conversation_id}.db)"
    unit: iso8601
    zone: unspecified
    confidence: inferred

record_types:
  - surface: session_transcript
    discriminator: $.type
    values: [CHECKPOINT, CONVERSATION_HISTORY, EPHEMERAL_MESSAGE, PLANNER_RESPONSE, USER_INPUT]
  - surface: session_transcript
    discriminator: $.source
    values: [MODEL, SYSTEM, USER_EXPLICIT]
  - surface: session_transcript
    discriminator: $.status
    values: [DONE]
  - surface: prompt_history
    discriminator: $.type
    values: [prompt, slash_command]
  - surface: state_db
    discriminator: conversations.steps.step_type
    values: ["14", "15", "23", "90", "98"]
  - surface: state_db
    discriminator: conversations.steps.status
    values: ["3"]
  - surface: app_log
    discriminator: "CLI glog severity prefix"
    values: [E, I, W]
  - surface: app_log
    discriminator: "IDE text log level"
    values: [error, info, warning]
  - surface: app_log
    discriminator: "Hub Electron text log level"
    values: [debug, error, info]

has_desktop_app: true
desktop_logs:
  same_log_format: false
  same_directory: false

changes: []
requires_claudine_update: true
reason: "Antigravity is not yet one of Claudine's compiled providers, and its useful ingestion surfaces are split across JSONL transcripts, per-conversation SQLite databases, prompt history, Go text logs, VS Code-derived desktop logs, and desktop protobuf stores."
---

# Antigravity Logging

## Introduction to Antigravity Logging

Antigravity splits logging and conversation state across three observed roots:

| Surface family | Observed macOS root | Purpose |
|---|---|---|
| CLI agent state | `~/.gemini/antigravity-cli/` | CLI settings, prompt history, per-conversation transcript JSONL, per-conversation SQLite databases, summaries, and CLI backend logs. |
| Desktop IDE diagnostics | `~/Library/Application Support/Antigravity IDE/logs/` | VS Code/Electron-style per-launch component logs for the desktop IDE. |
| Antigravity hub logs | `~/Library/Logs/Antigravity/` | Electron launcher and language-server logs for the Antigravity hub app. |

The user-visible `~/.antigravity` directory on this host mostly contained app install links, `argv.json`, and VS Code extension payloads. It did not contain the agent transcripts or the main runtime logs. The most useful ingestion surfaces were instead under `~/.gemini/antigravity-cli` and the macOS application log directories.

CLI transcripts are organized by conversation UUID:

```text
~/.gemini/antigravity-cli/
├── history.jsonl
├── conversation_summaries.db
├── conversations/{conversation_id}.db
├── conversations/{conversation_id}.db-{shm,wal}
├── brain/{conversation_id}/.system_generated/logs/transcript.jsonl
├── brain/{conversation_id}/.system_generated/logs/transcript_full.jsonl
├── log/cli-{YYYYMMDD}_{HHMMSS}.log
└── cli.log -> log/cli-{latest}.log
```

Observed CLI transcript records are JSONL step records. The per-conversation SQLite databases duplicate or enrich those steps in a more opaque store: the `steps` table has numeric `step_type`, numeric `status`, and protobuf-like BLOB columns for metadata, payloads, permissions, and render info. `conversation_summaries.db` is the global index. It uses SQLite and had live `-shm`/`-wal` sidecars on this host, so a collector must treat it as a live database.

Desktop IDE logs are organized by launch timestamp:

```text
~/Library/Application Support/Antigravity IDE/logs/{YYYYMMDD}T{HHMMSS}/
├── main.log
├── sharedprocess.log
├── auth.log
├── terminal.log
├── telemetry.log
├── window1/renderer.log
└── window1/exthost/exthost.log
```

Those logs are text diagnostics. They do not expose a complete agent transcript. Desktop conversation state was observed under `~/.gemini/antigravity-ide/conversations/{conversation_id}.pb`, which appears to be binary protobuf, plus desktop brain artifacts under `~/.gemini/antigravity-ide/brain/{conversation_id}/`.

The major observed log-message categories are:

| Surface | Record categories |
|---|---|
| `transcript*.jsonl` | `USER_INPUT`, `CONVERSATION_HISTORY`, `EPHEMERAL_MESSAGE`, `PLANNER_RESPONSE`, `CHECKPOINT` |
| `history.jsonl` | prompt entries and `slash_command` entries |
| per-conversation SQLite | numeric `step_type` values `14`, `15`, `23`, `90`, `98`; numeric `status` value `3` |
| CLI/backend text logs | Go/glog severities `I`, `W`, `E` |
| IDE component logs | bracketed text levels such as `info`, `warning`, `error` plus component/logger names |
| hub Electron logs | bracketed text levels `info`, `debug`, `error` |

The CLI settings file observed at `~/.gemini/antigravity-cli/settings.json` includes `enableTelemetry`, `model`, and `trustedWorkspaces`. The public settings docs document the same settings-file path. The local observed value had `"enableTelemetry": false`, which changes telemetry behavior but did not remove local logs. No documented or observed environment variable was found that relocates `~/.gemini/antigravity-cli`, changes transcript logging, or sets a general log level. Issue reports and local logs show project-local `.antigravitycli/mcp_config.json` discovery can be logged, but that is config discovery rather than a log-directory override.

## Logging Schema

No official Antigravity log schema was found in the public repository, public docs search results, or bundled local guide files. The GitHub repository is currently mostly README, examples, and changelog rather than source code containing schema definitions. The bundled local guides document `~/.gemini/antigravity-cli/settings.json`, but not transcript or SQLite schemas.

Community issue reports mention `~/.gemini/antigravity-cli/cli.log`, `~/.gemini/antigravity-cli/log/cli-*.log`, and `%AppData%/Antigravity/logs/` as troubleshooting surfaces, but they do not define a schema. Therefore the representative schema below is based on host-observed files.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntigravityTranscriptRecord {
    pub step_index: u64,
    pub source: AntigravityTranscriptSource,
    #[serde(rename = "type")]
    pub record_type: AntigravityTranscriptType,
    pub status: AntigravityTranscriptStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AntigravityTranscriptSource {
    Model,
    System,
    UserExplicit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AntigravityTranscriptType {
    Checkpoint,
    ConversationHistory,
    EphemeralMessage,
    PlannerResponse,
    UserInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AntigravityTranscriptStatus {
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntigravityHistoryRecord {
    pub display: String,
    pub timestamp: i64,
    pub workspace: String,
    #[serde(rename = "conversationId")]
    pub conversation_id: Option<String>,
    #[serde(rename = "type")]
    pub record_type: Option<AntigravityHistoryType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AntigravityHistoryType {
    SlashCommand,
}

#[derive(Debug, Clone)]
pub struct AntigravityConversationSummary {
    pub conversation_id: String,
    pub title: String,
    pub preview: String,
    pub step_count: i64,
    pub last_modified_time: String,
    pub workspace_uris: String,
    pub status: String,
    pub source: String,
    pub project_id: String,
    pub agent_name: String,
    pub parent_conversation_id: String,
    pub nesting_depth: i64,
    pub last_user_input_time: String,
    pub last_user_input_step_index: i64,
    pub app_data_dir: String,
}

#[derive(Debug, Clone)]
pub struct AntigravityConversationStepRow {
    pub idx: i64,
    pub step_type: i64,
    pub status: i64,
    pub has_subtrajectory: bool,
    pub step_format: i64,
    pub metadata: Option<Vec<u8>>,
    pub error_details: Option<Vec<u8>>,
    pub permissions: Option<Vec<u8>>,
    pub task_details: Option<Vec<u8>>,
    pub render_info: Option<Vec<u8>>,
    pub step_payload: Option<Vec<u8>>,
}
```

The safest ingestion order is:

1. Read `history.jsonl` for prompt-history and workspace context.
2. Read `brain/{conversation_id}/.system_generated/logs/transcript_full.jsonl` when present, falling back to `transcript.jsonl`.
3. Read `conversation_summaries.db` and `conversations/{conversation_id}.db` through SQLite APIs only when the databases are not actively being written, or through a SQLite backup API if live ingestion is necessary.
4. Use `log/cli-*.log`, desktop IDE logs, and hub logs for diagnostics and enrichment, not as primary transcript sources.

## Informational Content versus Hook Events

Filesystem logs are better when Claudine needs durable reconstruction: prompt history, previous assistant text, checkpoint records, session indexes, conversation IDs, per-session status, and backend diagnostics after the process has exited. Antigravity's JSONL transcripts and SQLite stores also survive runs where hooks were unavailable, disabled, or emitted only coarse lifecycle events.

Hook events are better for live lifecycle semantics: permission prompts, tool approvals, pre/post tool boundaries, stop events, and blocking decisions. The hooks topic should remain authoritative for those semantics. The log files can confirm that an event happened and provide surrounding context, but the observed transcript records are step snapshots rather than a complete hook-event stream.

Additional enrichment sources include:

| Source | Enrichment value |
|---|---|
| `~/.gemini/antigravity-cli/settings.json` | Model, telemetry preference, trusted workspaces. |
| `~/.gemini/antigravity-cli/cache/last_conversations.json` | Recent workspace ordering. |
| `~/.gemini/antigravity-cli/cache/projects.json` | Known project/workspace paths. |
| `~/.gemini/antigravity-cli/jetski_state.pbtxt` | Small text-protobuf state surface; observed but not decoded for this topic. |
| `~/.gemini/antigravity-cli/log/cli-*.log` | Auth, model-list, language-server, MCP discovery, and backend errors. |
| `~/Library/Logs/Antigravity/language_server.log` | Desktop language-server diagnostics, including references to missing or migrated trajectories. |
| `~/Library/Application Support/Antigravity IDE/User/.../state.vscdb` | Desktop app state and workspace state, useful only as optional enrichment. |

Claudine should treat Antigravity as requiring a provider-specific ingestion adapter rather than reusing a pure hook-log adapter. The transcript JSONL schema is straightforward, but the SQLite and protobuf stores need conservative handling because they may be live and because the protobuf payload schema is not public.

## Sources

- [Antigravity CLI repository](https://github.com/google-antigravity/antigravity-cli)
- [Antigravity CLI README](https://github.com/google-antigravity/antigravity-cli#readme)
- [Antigravity CLI changelog](https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md)
- [Using AGY CLI](https://antigravity.google/docs/cli-using)
- [Antigravity CLI settings](https://antigravity.google/docs/cli-settings)
- [Antigravity CLI product page](https://antigravity.google/product/antigravity-cli)
- [Google Developers Blog: Transitioning Gemini CLI to Antigravity CLI](https://developers.googleblog.com/an-important-update-transitioning-gemini-cli-to-antigravity-cli/)
- [GitHub issue #25: HTTP MCP servers with OAuth fail at runtime](https://github.com/google-antigravity/antigravity-cli/issues/25)
- [GitHub issue #60: project-local `.antigravitycli/mcp_config.json`](https://github.com/google-antigravity/antigravity-cli/issues/60)
- Local observed host files under `~/.antigravity`, `~/.gemini/antigravity-cli`, `~/.gemini/antigravity-ide`, `~/Library/Application Support/Antigravity IDE`, and `~/Library/Logs/Antigravity`.
