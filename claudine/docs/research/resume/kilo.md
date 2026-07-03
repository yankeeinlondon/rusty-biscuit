---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: opencode
model: kimi-for-coding/k2p7
docs: https://kilo.ai/docs/code-with-ai/platforms/cli
support: first_class
continuity_model: server_session
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "kilo --continue / -c"
      - "kilo --session <id>"
      - "kilo --session <id> --fork"
      - "kilo --session <id> --cloud-fork"
      - "TUI slash commands /sessions /resume /continue"
    accepts_followup_prompt: false
    selection_methods:
      - latest
      - id
      - picker
      - all_projects
    notes: "Follow-up prompts are typed inside the resumed TUI session. The picker shows titles, not raw IDs. --cloud-fork fetches a cloud session and continues it locally."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "kilo run --continue <message>"
      - "kilo run --session <id> <message>"
      - "kilo run --session <id> --fork <message>"
      - "kilo run --session <id> --cloud-fork <message>"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
    notes: "The scriptable CLI surface that sends a new prompt into an existing session. --continue picks the most recent root session from the current project. --replay can replay visible history on interactive resume."
  - mode: headless_server
    supported: true
    mechanisms:
      - "kilo serve"
      - "kilo web"
      - "kilo daemon"
      - "kilo attach <url> --session <id>"
      - "kilo run --attach <url> --session <id>"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
      - picker
    notes: "Local headless server owns the authoritative session store. Clients attach via HTTP or the running server and can send follow-up prompts. The daemon and console provide persistent server management."
  - mode: ide
    supported: true
    mechanisms:
      - "VS Code extension resume past conversations"
      - "JetBrains plugin resume"
    accepts_followup_prompt: false
    selection_methods:
      - picker
      - latest
    notes: "IDE extensions maintain their own session history; this research focuses on CLI behavior."
  - mode: api
    supported: true
    mechanisms:
      - "HTTP OpenAPI server exposed by kilo serve / kilo daemon"
      - "@opencode-ai/sdk client.session.prompt() (Kilo CLI is an OpenCode fork)"
      - "@opencode-ai/sdk client.event.subscribe()"
    accepts_followup_prompt: true
    selection_methods:
      - id
    notes: "Programmatic resume and follow-up are first-class via the local server API and SDK. Exact endpoint paths are inherited from the OpenCode upstream; Kilo docs do not republish them."
session_id_capture:
  - surface: json_stream
    field: sessionID
    format: "ses_<base62>"
    notes: "Present on every JSON event emitted by kilo run --format json. Inferred from OpenCode heritage; locally verified for session list IDs."
  - surface: cli_command
    field: id
    format: "ses_<base62>"
    notes: "kilo session list --format json returns objects with id, title, updated, created, projectId, directory, and project."
  - surface: session_file
    field: id
    format: "SQLite row in kilo.db"
    notes: "The session database lives in the Kilo data directory. Session IDs are stable primary keys in the local database, not derived from filenames."
  - surface: interactive_ui
    field: title
    format: string
    notes: "The TUI session picker shows session titles and timestamps, not raw IDs. Raw IDs are exposed through the external session list or export commands."
  - surface: log_file
    field: run
    format: hex
    notes: "stderr logs identify the run instance, not the session. Session ID must be captured from the JSON stream or session list."
resume_invocations:
  - mode: interactive
    invocation: "kilo --continue"
    accepts_prompt: false
    selection: latest
    notes: "Resumes the most recent root session for the current project/directory in the TUI."
  - mode: interactive
    invocation: "kilo --session <session-id>"
    accepts_prompt: false
    selection: id
    notes: "Opens the TUI on the specified session."
  - mode: interactive
    invocation: "kilo --session <session-id> --fork"
    accepts_prompt: false
    selection: id
    notes: "Forks the specified session into a new session, then opens it in the TUI."
  - mode: interactive
    invocation: "kilo --session <session-id> --cloud-fork"
    accepts_prompt: false
    selection: id
    notes: "Fetches a cloud session and continues it locally in the TUI."
  - mode: interactive
    invocation: "/sessions (alias /resume, /continue)"
    accepts_prompt: false
    selection: picker
    notes: "Inside the TUI, opens the session selector."
  - mode: non_interactive
    invocation: "kilo run --continue <message>"
    accepts_prompt: true
    selection: latest
    notes: "Sends a follow-up message into the most recent root session and returns the response."
  - mode: non_interactive
    invocation: "kilo run --session <session-id> <message>"
    accepts_prompt: true
    selection: id
    notes: "Sends a follow-up message into the specified session and returns the response."
  - mode: non_interactive
    invocation: "kilo run --session <session-id> --fork <message>"
    accepts_prompt: true
    selection: id
    notes: "Forks the session, sends the message into the new session, and returns the response."
  - mode: non_interactive
    invocation: "kilo run --session <session-id> --cloud-fork <message>"
    accepts_prompt: true
    selection: id
    notes: "Fetches a cloud session, forks it locally, sends the message, and returns the response."
  - mode: headless_server
    invocation: "POST /session/:id/message { parts: [...] }"
    accepts_prompt: true
    selection: id
    notes: "Send a message to an existing session through the headless server and wait for the assistant response. Endpoint inherited from OpenCode upstream."
  - mode: headless_server
    invocation: "POST /session/:id/fork { messageID? }"
    accepts_prompt: false
    selection: id
    notes: "Fork an existing session at an optional message ID through the server API. Endpoint inherited from OpenCode upstream."
  - mode: api
    invocation: "client.session.prompt({ path: { id }, body: { parts: [...] } })"
    accepts_prompt: true
    selection: id
    notes: "SDK method to send a follow-up prompt into an existing session. SDK is inherited from OpenCode upstream."
state_storage:
  - location: local
    os: macos
    path: "~/.local/share/kilo/kilo.db"
    format: SQLite
    retention: "Controlled by internal pruning; shared conversations persist until unshared."
    notes: "Observed on macOS. The database is the authoritative session store for the local server."
  - location: local
    os: linux
    path: "~/.local/share/kilo/kilo.db"
    format: SQLite
    retention: "Controlled by internal pruning; shared conversations persist until unshared."
    notes: "Inferred from XDG-style paths observed on macOS and standard Node.js app conventions."
  - location: local
    os: windows
    path: "%LOCALAPPDATA%\\kilo\\kilo.db"
    format: SQLite
    retention: "Controlled by internal pruning; shared conversations persist until unshared."
    notes: "Inferred from standard Node.js app conventions; not verified by local inspection."
  - location: local
    os: all
    path: "<data-dir>/snapshot/ and <data-dir>/storage/"
    format: "proprietary (snapshots and auxiliary storage)"
    retention: "Same as database"
    notes: "Additional session artifacts (file snapshots, tool output, exported data) live alongside the database. Direct parsing is unsupported."
  - location: server
    os: all
    path: "Kilo Cloud Agent / Kilo Gateway"
    format: "cloud session record"
    retention: "Inactive cloud agent sessions are deleted after 7 days during beta; expired sessions remain accessible via the CLI."
    notes: "Cloud sessions can be fetched and resumed locally with --cloud-fork. Remote mode syncs local sessions to the cloud dashboard."
resume_scope:
  project_scoped: true
  cwd_scoped: true
  worktree_aware: false
  all_projects_supported: true
  branch_filtering: false
  notes: "Sessions have a projectId and directory. --continue lists sessions and picks the first root session, which can surprise users when unrelated directories share a project record. The API and picker can access sessions across all projects. Cloud-fork extends scope to cloud-hosted sessions."
branching_checkpointing:
  branch_supported: true
  checkpoint_supported: true
  fork_invocation: "kilo --session <id> --fork or kilo run --session <id> --fork; POST /session/:id/fork"
  checkpoint_invocation: "POST /session/:id/revert { messageID, partID? } or /undo /redo in TUI"
  preserves_original: true
  notes: "Forking creates a new session record and copies message history, leaving the original intact. Revert rewinds conversation state within the same session; undo/redo operate on file edits. The session table schema includes parent_id for forks and revert for checkpoint state."
restored_state:
  transcript: true
  tool_results: true
  approvals: unknown
  model: overridable
  cwd: configurable
  env: current_process
  notes: "Transcript and tool results are loaded from the local session database. --model and --dir can override the original choices at resume time. Environment comes from the launching process, not the session. The session table has a permission column, but whether it is fully restored on resume is not explicitly documented."
hitl_resume:
  supported: true
  question_capture: "Subscribe to server SSE events via client.event.subscribe() or poll GET /session/status; question and permission requests block the session until answered."
  answer_injection: "POST /session/:id/permissions/:permissionID { response, remember? } answers a permission request; POST /session/:id/message sends a user reply into a session waiting on a question tool."
  limitations: "Stock kilo run disables the question tool and auto-rejects runtime permission prompts, so it cannot broker HITL. Use the server/SDK API, remote mode, or Cloud Agent for human-in-the-loop workflows."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: true
  pending_approval_resume: true
  limitations: "Sessions are persisted continuously to the local SQLite database, so crash, Ctrl+C, terminal close, and process kill leave the session resumable. Pending questions/permissions recover if the server process is still running; if the server is killed, resuming from the DB restores the conversation but may reset in-flight approval/question state."
observability:
  stream_events:
    - "sessionID in every kilo run --format json event"
    - "server SSE event stream (client.event.subscribe)"
    - "GET /session/status returns per-session status"
  hook_events:
    - "plugin event hook receives session lifecycle events"
  failure_modes:
    - "auto-rejected permission in kilo run"
    - "session abort via POST /session/:id/abort"
  notes: "The JSON stream is the most reliable CLI capture surface. The server exposes health, session list, session status, and event endpoints for programmatic monitoring. kilo session list --format json is the stablest non-stream source of IDs."
quirks:
  - "kilo run in default mode does not print a dedicated session-id banner; scripts must use --format json or session list."
  - "kilo run explicitly disables the question tool and auto-rejects permission prompts, making it unsuitable for human-in-the-loop resumption."
  - "--continue resolves by listing sessions and picking the first root session, which can select the wrong session when multiple directories map to the same projectId."
  - "Forking copies message history into a new session; long sessions can make fork slow."
  - "Shared sessions sync conversation history to Kilo servers; disabling sharing via config keeps data local."
  - "The SQLite database and auxiliary storage formats are internal and not stable for direct parsing; use export/import or the API for structured access."
  - "macOS stores data under ~/.local/share/kilo rather than ~/Library, following XDG-style paths."
  - "--cloud-fork requires the session to be available from Kilo Cloud / Gateway and is only usable with --session."
  - "kilo run --replay and --replay-limit control whether resumed interactive history is replayed to the model; defaults to false."
gaps:
  - "Exact concurrency semantics when two clients resume/send messages to the same session simultaneously are not documented."
  - "Whether approval state (always-allow settings, per-session permissions) is fully restored on resume is not explicitly documented."
  - "Windows session storage paths are inferred, not verified by local inspection."
  - "Retention sweep timing and behavior for active sessions are not documented."
  - "The exact OpenAPI/SSE endpoint paths and SDK methods are inherited from OpenCode upstream and not republished in Kilo docs."
  - "Whether kilo run --format json emits sessionID on every event was not locally verified with a live run."
changes: []
requires_claudine_update: true
reason: "Kilo Code is on the Claudine research roster but is not yet a code-supported provider. The verified server_session continuity model, Kilo-specific cloud-fork and remote-mode resume paths, and confirmed local storage paths should feed provider metadata and wrapper design."
---

## Overview

Kilo Code's session resume is a first-class, local-server-backed feature inherited from its OpenCode upstream, with Kilo-specific extensions for cloud sessions and remote connections. Every session is stored in a local SQLite database (`kilo.db`) that is owned by a Kilo server process; the TUI, `kilo run`, the SDK, and the HTTP API are all clients of that server. Resume means reattaching to the same session record and appending new turns, not replaying a standalone transcript file. The CLI supports interactive resumption (`--continue`, `--session`, `/sessions`), scriptable non-interactive follow-up (`kilo run --session <id> <message>`), full programmatic control through the headless server and SDK, and Kilo-specific cloud session resumption via `--cloud-fork`.

## Resume Semantics

A Kilo "session" is a persisted conversation record in the local SQLite database, identified by a stable `ses_...` ID. The authoritative state lives in the database and the currently running server process. When you resume, the client asks the server for the session, loads its message history, and continues from the last turn. Because the TUI itself is a client to the server, resuming the same session from multiple clients is possible and will share the same underlying record. Cloud sessions can be fetched and resumed locally, and remote mode can sync local sessions to the Kilo Cloud Agent dashboard.

## Supported Modes

| Mode | Entry point | Follow-up prompt | Selector |
|------|-------------|------------------|----------|
| Interactive CLI | `kilo --continue` | No | Latest root session |
| Interactive CLI | `kilo --session <id>` | No | Exact session ID |
| Interactive CLI | `kilo --session <id> --fork` | No | Exact session ID, then fork |
| Interactive CLI | `kilo --session <id> --cloud-fork` | No | Cloud session ID |
| Interactive CLI | `/sessions` (alias `/resume`, `/continue`) | No | Picker |
| Non-interactive CLI | `kilo run --continue <message>` | Yes | Latest root session |
| Non-interactive CLI | `kilo run --session <id> <message>` | Yes | Exact session ID |
| Non-interactive CLI | `kilo run --session <id> --fork <message>` | Yes | Exact session ID, fork first |
| Non-interactive CLI | `kilo run --session <id> --cloud-fork <message>` | Yes | Cloud session ID |
| Headless server | `kilo serve` / `kilo web` / `kilo daemon` | Yes | ID or picker |
| HTTP/API | `POST /session/:id/message` | Yes | Exact session ID |
| SDK | `client.session.prompt({ path: { id }, body: { parts } })` | Yes | Exact session ID |
| IDE | VS Code / JetBrains extension | No | Picker |

`kilo run` also supports `--attach <url>` to send a prompt through a running server rather than starting a new local process.

## Session ID Capture

Stable session identifiers use the `ses_<base62>` format. Capture surfaces:

- **`kilo run --format json`**: every emitted JSON event includes `sessionID` (inherited from OpenCode; not locally verified with a live run).
- **`kilo session list --format json`**: returns an array of `{ id, title, updated, created, projectId, directory, project }`.
- **`kilo export [sessionID]`**: exports the full session payload as JSON; without an ID it prompts for selection.
- **Local database**: `kilo.db` in the Kilo data directory holds session records.
- **TUI picker**: shows titles and timestamps, not raw IDs.

## Resume Invocation

Continue the latest session interactively:

```bash
kilo --continue
```

Resume a specific session interactively:

```bash
kilo --session ses_273de4d0cffewNgA2v0GJo0w5o
```

Branch while resuming:

```bash
kilo --session ses_273de4d0cffewNgA2v0GJo0w5o --fork
```

Resume a cloud session locally:

```bash
kilo --session ses_<cloud-id> --cloud-fork
```

Send a follow-up non-interactively:

```bash
kilo run --session ses_273de4d0cffewNgA2v0GJo0w5o "what is the next step?"
```

Continue the latest non-interactive session:

```bash
kilo run --continue "finish the task"
```

Attach to a running server and resume:

```bash
kilo run --attach http://localhost:4096 --session ses_273de4d0cffewNgA2v0GJo0w5o "next step"
```

Resume via the headless server and SDK:

```bash
kilo serve --port 4096
```

```typescript
import { createOpencodeClient } from "@opencode-ai/sdk"
const client = createOpencodeClient({ baseUrl: "http://localhost:4096" })
await client.session.prompt({
  path: { id: "ses_273de4d0cffewNgA2v0GJo0w5o" },
  body: { parts: [{ type: "text", text: "what is the next step?" }] },
})
```

## Session Lookup Scope

Sessions are associated with a `projectId` and a `directory`. The default `--continue` lookup lists sessions and picks the first root session, which can be surprising when multiple working directories share a project record. The API and TUI picker can widen to all projects. There is no documented git-branch or worktree filtering for resume. `--cloud-fork` extends scope to sessions stored in Kilo Cloud / Gateway.

## State Storage

Resumable state is local and server-side in the same process, with optional cloud persistence.

| OS | Path |
|----|------|
| macOS | `~/.local/share/kilo/kilo.db` |
| Linux (inferred) | `~/.local/share/kilo/kilo.db` |
| Windows (inferred) | `%LOCALAPPDATA%\kilo\kilo.db` |

Global paths observed on macOS:

```
data   ~/.local/share/kilo
state  ~/.local/state/kilo
config ~/.config/kilo
cache  ~/.cache/kilo
log    ~/.local/share/kilo/log
tmp    /var/folders/.../kilo
```

Additional artifacts (snapshots, tool output, exported JSON) live under `<data-dir>/snapshot/`, `<data-dir>/storage/`, `<data-dir>/tool-output/`, and a separate `session-export.db`. The SQLite schema and auxiliary formats are internal and may change between releases; scripts should use `kilo export`, `kilo import`, or the HTTP/SDK API for stable access.

The `session` table schema includes `project_id`, `parent_id` (for forks), `directory`, `revert` (for checkpoint state), `permission`, `agent`, and `model` columns.

Cloud Agent sessions are stored in Kilo Cloud; inactive sessions are deleted after 7 days during beta, but expired sessions remain accessible via the CLI.

## Restored State

Resuming restores:

- The full conversation transcript and tool results from the database.
- The working directory from the session record, overridable with `--dir`.
- The model from the session record, overridable with `--model` or the API `model` field.

Resuming does not restore the environment of the original session; it uses the environment of the process that runs the resume command. Approval-state restore behavior is not explicitly documented, although a `permission` column exists in the session table.

## Branching and Checkpoints

Branching creates a copy of the conversation and leaves the original intact:

- CLI: `kilo --session <id> --fork` or `kilo run --session <id> --fork <message>`.
- API: `POST /session/:id/fork { messageID? }`.
- SDK: `client.session.fork({ path: { id }, body: { messageID? } })`.

Checkpointing within a session is available via:

- TUI: `/undo` and `/redo` for file edits.
- API: `POST /session/:id/revert { messageID, partID? }` and `POST /session/:id/unrevert`.

## Human-in-the-Loop Resume

Kilo supports human-in-the-loop resumption, but **not** through stock `kilo run`. That command disables the `question` tool and auto-rejects runtime permission prompts. For Claudine-style brokering:

1. Start or attach to a Kilo server (`kilo serve`, `kilo daemon`, or SDK `createOpencode()`).
2. Create or identify a session via `client.session.create()` or `client.session.list()`.
3. Subscribe to events via `client.event.subscribe()`.
4. When a `permission.asked` or `question.asked` event arrives, capture it.
5. Answer through `POST /session/:id/permissions/:permissionID` or by sending a user message into the session.
6. The session continues from the same `sessionID`.

Remote mode and Cloud Agent can also surface questions/permissions for answering elsewhere.

## Interruption Recovery

Because session state is persisted continuously to the local SQLite database, sessions survive most interruptions:

- **Crash / terminal close / process kill**: the database remains and the session can be resumed.
- **Ctrl+C**: the session is preserved; resume with `--continue` or `--session`.
- **Pending tool call / approval / question**: if the server process is still running, the pending request remains blocked until answered. If the server is killed, resuming from the DB restores the conversation but may reset in-flight interactive state.

## Observability

Surfaces that expose session identity or resumability:

- **`kilo run --format json`**: every event carries `sessionID` (inherited from OpenCode).
- **`kilo session list --format json`**: lists all sessions with IDs, titles, and directories.
- **`kilo export [sessionID]`**: dumps session data as JSON.
- **Server SSE stream**: `GET /event` and SDK `client.event.subscribe()` stream lifecycle and request events.
- **Server session status**: `GET /session/status` returns `{ [sessionID]: SessionStatus }`.
- **Server health**: `GET /global/health` returns version and liveness.
- **`kilo debug paths`**: shows global data, config, cache, state, and log paths.

## Quirks and Gaps

Quirks:

- `kilo run` does not print a dedicated session ID in default mode; use `--format json` or `session list`.
- `kilo run` is intentionally non-interactive and will not broker questions or permissions.
- `--continue` can select an unexpected session when multiple directories share a project record.
- Forking copies full message history; very long sessions may be slow to fork.
- Shared sessions upload conversation history to Kilo servers; keep sharing disabled for local-only data.
- Direct parsing of `kilo.db` or auxiliary storage files is unsupported and unstable.
- macOS stores data under `~/.local/share/kilo` rather than `~/Library`.
- `--cloud-fork` is only usable with `--session` and requires the cloud session to be accessible.
- `kilo run --replay` and `--replay-limit` control whether resumed interactive history is replayed to the model.

Gaps:

- Exact concurrency guarantees when two clients write to the same session are not documented.
- Whether approval state is fully restored on resume is not explicitly documented.
- Windows session storage paths are inferred from Node.js conventions, not locally verified.
- Retention sweep timing and behavior for active sessions are not documented.
- Exact OpenAPI/SSE endpoint paths and SDK methods are inherited from OpenCode and not republished in Kilo docs.
- Whether `kilo run --format json` emits `sessionID` on every event was not locally verified with a live run.

## Claudine Integration Notes

For Claudine's lifecycle `resume` action and future HITL broker:

- Capture `sessionID` from `kilo run --format json` or from `kilo session list --format json`.
- Use `kilo run --session <id> "<follow-up>"` for simple non-interactive continuation.
- Do **not** rely on `kilo run` for human-in-the-loop; instead run or attach to a Kilo server and use the SDK/HTTP API.
- Subscribe to server events to detect `permission.asked` and `question.asked`, and answer through the appropriate API endpoints.
- Treat the SQLite database and snapshot directories as internal storage; use export/import or the API for structured access.
- Be aware of Kilo-specific extensions: `--cloud-fork` for cloud sessions, `kilo daemon`/`kilo console` for persistent servers, and `kilo run --attach` for reusing an existing server.
- When resuming after failure, be aware that model and directory can be overridden but approval-state restore is undocumented.

## Sources

- [Kilo Code CLI docs](https://kilo.ai/docs/code-with-ai/platforms/cli)
- [Kilo Code CLI command reference](https://kilo.ai/docs/code-with-ai/platforms/cli-reference)
- [Kilo Code Cloud Agent docs](https://kilo.ai/docs/code-with-ai/platforms/cloud-agent)
- [Kilo Code GitHub repository](https://github.com/Kilo-Org/kilocode)
- [OpenCode CLI docs](https://opencode.ai/docs/cli) (Kilo CLI is an OpenCode fork)
- [OpenCode Server docs](https://opencode.ai/docs/server)
- [OpenCode SDK docs](https://opencode.ai/docs/sdk)
