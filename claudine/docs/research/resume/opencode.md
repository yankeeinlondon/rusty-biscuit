---
$schema: ./_schema.yaml
created: 2026-04-02
last_updated: 2026-07-02
agent: opencode
model: kimi-for-coding/k2p7
docs: https://opencode.ai/docs/cli
support: first_class
continuity_model: server_session
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "opencode --continue"
      - "opencode --session <id>"
      - "opencode --fork"
      - "TUI slash commands /sessions /resume /continue"
      - "TUI session picker (Ctrl+X L)"
    accepts_followup_prompt: false
    selection_methods:
      - latest
      - id
      - picker
      - all_projects
    notes: "Follow-up prompts are typed inside the resumed TUI session. The picker shows titles, not raw IDs."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "opencode run --continue <message>"
      - "opencode run --session <id> <message>"
      - "opencode run --session <id> --fork <message>"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
    notes: "The only scriptable CLI surface that sends a new prompt into an existing session. --continue picks the most recent root session from the current project."
  - mode: headless_server
    supported: true
    mechanisms:
      - "opencode serve"
      - "opencode web"
      - "opencode attach <url> --session <id>"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
      - picker
    notes: "The local headless server owns the authoritative session store. Clients attach via HTTP/SDK and can send follow-up prompts through the message API."
  - mode: ide
    supported: true
    mechanisms:
      - "OpenCode IDE extensions"
      - "ACP-compatible editors (Zed, JetBrains, Neovim) via opencode acp"
    accepts_followup_prompt: false
    selection_methods:
      - picker
      - latest
    notes: "IDE/ACP surfaces maintain their own view of session history; this research focuses on CLI and server behavior."
  - mode: api
    supported: true
    mechanisms:
      - "HTTP OpenAPI server at http://hostname:port"
      - "@opencode-ai/sdk client.session.prompt()"
      - "@opencode-ai/sdk client.event.subscribe()"
    accepts_followup_prompt: true
    selection_methods:
      - id
    notes: "Programmatic resume and follow-up are first-class via the server API and SDK."
session_id_capture:
  - surface: json_stream
    field: sessionID
    format: "ses_<base62>"
    notes: "Present on every JSON event emitted by opencode run --format json."
  - surface: cli_command
    field: id
    format: "ses_<base62>"
    notes: "opencode session list --format json returns objects with id, title, updated, created, projectId, and directory."
  - surface: session_file
    field: id
    format: "SQLite row in opencode.db"
    notes: "The session database lives in the OpenCode data directory. Session IDs are stable primary keys in the local database, not derived from filenames."
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
    invocation: "opencode --continue"
    accepts_prompt: false
    selection: latest
    notes: "Resumes the most recent root session for the current project/directory in the TUI."
  - mode: interactive
    invocation: "opencode --session <session-id>"
    accepts_prompt: false
    selection: id
    notes: "Opens the TUI on the specified session."
  - mode: interactive
    invocation: "opencode --session <session-id> --fork"
    accepts_prompt: false
    selection: id
    notes: "Forks the specified session into a new session, then opens it in the TUI."
  - mode: interactive
    invocation: "/sessions (alias /resume, /continue)"
    accepts_prompt: false
    selection: picker
    notes: "Inside the TUI, opens the session selector."
  - mode: non_interactive
    invocation: "opencode run --continue <message>"
    accepts_prompt: true
    selection: latest
    notes: "Sends a follow-up message into the most recent root session and returns the response."
  - mode: non_interactive
    invocation: "opencode run --session <session-id> <message>"
    accepts_prompt: true
    selection: id
    notes: "Sends a follow-up message into the specified session and returns the response."
  - mode: non_interactive
    invocation: "opencode run --session <session-id> --fork <message>"
    accepts_prompt: true
    selection: id
    notes: "Forks the session, sends the message into the new session, and returns the response."
  - mode: headless_server
    invocation: "POST /session/:id/message { parts: [...] }"
    accepts_prompt: true
    selection: id
    notes: "Send a message to an existing session through the headless server and wait for the assistant response."
  - mode: headless_server
    invocation: "POST /session/:id/fork { messageID? }"
    accepts_prompt: false
    selection: id
    notes: "Fork an existing session at an optional message ID through the server API."
  - mode: api
    invocation: "client.session.prompt({ path: { id }, body: { parts: [...] } })"
    accepts_prompt: true
    selection: id
    notes: "SDK method to send a follow-up prompt into an existing session."
state_storage:
  - location: local
    os: macos
    path: "~/.local/share/opencode/opencode.db"
    format: SQLite
    retention: "Controlled by OPENCODE_DISABLE_PRUNE and internal pruning; shared conversations persist until unshared."
    notes: "Observed on macOS. The database is the authoritative session store for the local server."
  - location: local
    os: linux
    path: "~/.local/share/opencode/opencode.db"
    format: SQLite
    retention: "Controlled by OPENCODE_DISABLE_PRUNE and internal pruning; shared conversations persist until unshared."
    notes: "Inferred from XDG-style paths observed on macOS and standard Node.js app conventions."
  - location: local
    os: windows
    path: "%LOCALAPPDATA%\\opencode\\opencode.db"
    format: SQLite
    retention: "Controlled by OPENCODE_DISABLE_PRUNE and internal pruning; shared conversations persist until unshared."
    notes: "Inferred from standard Node.js app conventions; not verified by local inspection."
  - location: local
    os: all
    path: "<data-dir>/snapshot/ and <data-dir>/storage/"
    format: "proprietary (snapshots and auxiliary storage)"
    retention: "Same as database"
    notes: "Additional session artifacts (file snapshots, tool output, exported data) live alongside the database. Direct parsing is unsupported."
resume_scope:
  project_scoped: true
  cwd_scoped: true
  worktree_aware: false
  all_projects_supported: true
  branch_filtering: false
  notes: "Sessions have a projectId and directory. --continue lists sessions and picks the first root session, which can surprise users when unrelated directories share a project record. The API and picker can access sessions across all projects."
branching_checkpointing:
  branch_supported: true
  checkpoint_supported: true
  fork_invocation: "opencode --session <id> --fork or POST /session/:id/fork"
  checkpoint_invocation: "POST /session/:id/revert { messageID, partID? } or /undo /redo in TUI"
  preserves_original: true
  notes: "Forking creates a new session record and copies message history, leaving the original intact. Revert rewinds conversation state within the same session; undo/redo operate on file edits."
restored_state:
  transcript: true
  tool_results: true
  approvals: unknown
  model: overridable
  cwd: configurable
  env: current_process
  notes: "Transcript and tool results are loaded from the local session database. --model and --dir can override the original choices at resume time. Environment comes from the launching process, not the session. Approval restore behavior is not explicitly documented."
hitl_resume:
  supported: true
  question_capture: "Subscribe to server SSE events via client.event.subscribe() or poll GET /session/status; question and permission requests block the session until answered."
  answer_injection: "POST /session/:id/permissions/:permissionID { response, remember? } answers a permission request; POST /session/:id/message sends a user reply into a session waiting on a question tool."
  limitations: "Stock opencode run disables the question tool and auto-rejects runtime permission prompts, so it cannot broker HITL. Use the server/SDK API for human-in-the-loop workflows."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: true
  pending_approval_resume: true
  limitations: "Sessions are persisted continuously to the local SQLite database, so crash, Ctrl+C, terminal close, and process kill leave the session resumable. Pending questions/permissions recover if the server process is still running; if the server is killed, resuming from the DB restores the conversation but may reset in-flight approval/question state."
observability:
  stream_events:
    - "sessionID in every opencode run --format json event"
    - "server SSE event stream (client.event.subscribe)"
    - "GET /session/status returns per-session status"
  hook_events:
    - "plugin event hook receives session lifecycle events"
  failure_modes:
    - "auto-rejected permission in opencode run"
    - "session abort via POST /session/:id/abort"
  notes: "The JSON stream is the most reliable CLI capture surface. The server exposes health, session list, session status, and event endpoints for programmatic monitoring."
quirks:
  - "opencode run in default mode does not print a dedicated session-id banner; scripts must use --format json or session list."
  - "opencode run explicitly disables the question tool and auto-rejects permission prompts, making it unsuitable for human-in-the-loop resumption."
  - "--continue resolves by listing sessions and picking the first root session, which can select the wrong session when multiple directories map to the same projectId."
  - "Forking copies message history into a new session; long sessions can make fork slow."
  - "Shared sessions sync conversation history to OpenCode servers; disabling sharing via config or OPENCODE_AUTO_SHARE keeps data local."
  - "The SQLite database and auxiliary storage formats are internal and not stable for direct parsing; use export/import or the API for structured access."
gaps:
  - "Exact concurrency semantics when two clients resume/send messages to the same session simultaneously are not documented."
  - "Whether approval state (always-allow settings, per-session permissions) is fully restored on resume is not explicitly documented."
  - "Windows session storage paths are inferred, not verified by local inspection."
  - "Retention sweep timing and behavior for active sessions are not documented."
changes:
  - "2026-07-02: Replaced free-form frontmatter with schema-validated fields."
  - "2026-07-02: Updated against OpenCode CLI 1.17.13 and current server/API docs."
  - "2026-07-02: Classified continuity model as server_session (local SQLite-backed server) rather than transcript replay."
  - "2026-07-02: Added headless_server and api resume modes with exact HTTP/SDK invocations."
  - "2026-07-02: Documented that opencode run disables question tool and auto-rejects permissions, so HITL requires server/SDK."
requires_claudine_update: true
reason: "The refreshed schema, server_session continuity model, and verified HITL path via server/SDK (rather than opencode run) should feed into Claudine's lifecycle resume action, session-id capture logic, and future human-in-the-loop broker implementation for OpenCode."
---

## Overview

OpenCode's session resume is a first-class, local-server-backed feature. Every session is stored in a local SQLite database (`opencode.db`) that is owned by an OpenCode server process; the TUI, `opencode run`, the SDK, and the HTTP API are all clients of that server. Resume means reattaching to the same session record and appending new turns, not replaying a standalone transcript file. The CLI supports interactive resumption (`--continue`, `--session`, `/sessions`), scriptable non-interactive follow-up (`opencode run --session <id> <message>`), and full programmatic control through the headless server and SDK.

## Resume Semantics

An OpenCode "session" is a persisted conversation record in the local SQLite database, identified by a stable `ses_...` ID. The authoritative state lives in the database and the currently running server process. When you resume, the client asks the server for the session, loads its message history, and continues from the last turn. Because the TUI itself is a client to the server, resuming the same session from multiple clients is possible and will share the same underlying record.

## Supported Modes

| Mode | Entry point | Follow-up prompt | Selector |
|------|-------------|------------------|----------|
| Interactive CLI | `opencode --continue` | No | Latest root session |
| Interactive CLI | `opencode --session <id>` | No | Exact session ID |
| Interactive CLI | `/sessions` (alias `/resume`, `/continue`) | No | Picker |
| Interactive CLI | `Ctrl+X L` | No | Picker |
| Non-interactive CLI | `opencode run --continue <message>` | Yes | Latest root session |
| Non-interactive CLI | `opencode run --session <id> <message>` | Yes | Exact session ID |
| Headless server | `opencode serve` / `opencode web` | Yes | ID or picker |
| HTTP/API | `POST /session/:id/message` | Yes | Exact session ID |
| SDK | `client.session.prompt({ path: { id }, body: { parts } })` | Yes | Exact session ID |
| IDE/ACP | `opencode acp` | No | Picker |

## Session ID Capture

Stable session identifiers use the `ses_<base62>` format. Capture surfaces:

- **`opencode run --format json`**: every emitted JSON event includes `sessionID`.
- **`opencode session list --format json`**: returns an array of `{ id, title, updated, created, projectId, directory }`.
- **`opencode export [sessionID]`**: exports the full session payload as JSON; without an ID it prompts for selection.
- **Local database**: `opencode.db` in the OpenCode data directory holds session records.
- **TUI picker**: shows titles and timestamps, not raw IDs.

## Resume Invocation

Continue the latest session interactively:

```bash
opencode --continue
```

Resume a specific session interactively:

```bash
opencode --session ses_0d98dfb69ffeScBxUnWWLfhCAl
```

Branch while resuming:

```bash
opencode --session ses_0d98dfb69ffeScBxUnWWLfhCAl --fork
```

Send a follow-up non-interactively:

```bash
opencode run --session ses_0d98dfb69ffeScBxUnWWLfhCAl "what is the next step?"
```

Continue the latest non-interactive session:

```bash
opencode run --continue "finish the task"
```

Resume via the headless server and SDK:

```bash
opencode serve --port 4096
```

```typescript
import { createOpencodeClient } from "@opencode-ai/sdk"
const client = createOpencodeClient({ baseUrl: "http://localhost:4096" })
await client.session.prompt({
  path: { id: "ses_0d98dfb69ffeScBxUnWWLfhCAl" },
  body: { parts: [{ type: "text", text: "what is the next step?" }] },
})
```

## Session Lookup Scope

Sessions are associated with a `projectId` and a `directory`. The default `--continue` lookup lists sessions and picks the first root session, which can be surprising when multiple working directories share a project record. The API and TUI picker can widen to all projects. There is no documented git-branch or worktree filtering for resume.

## State Storage

Resumable state is local and server-side in the same process.

| OS | Path |
|----|------|
| macOS / Linux | `~/.local/share/opencode/opencode.db` |
| Windows (inferred) | `%LOCALAPPDATA%\opencode\opencode.db` |

Additional artifacts (snapshots, tool output, exported JSON) live under `<data-dir>/snapshot/`, `<data-dir>/storage/`, and `<data-dir>/tool-output/`. The SQLite schema and auxiliary formats are internal and may change between releases; scripts should use `opencode export`, `opencode import`, or the HTTP/SDK API for stable access.

## Restored State

Resuming restores:

- The full conversation transcript and tool results from the database.
- The working directory from the session record, overridable with `--dir`.
- The model from the session record, overridable with `--model` or the API `model` field.

Resuming does not restore the environment of the original session; it uses the environment of the launching process. Approval-state restore behavior is not explicitly documented.

## Branching and Checkpoints

Branching creates a copy of the conversation and leaves the original intact:

- CLI: `opencode --session <id> --fork` or `opencode run --session <id> --fork <message>`.
- API: `POST /session/:id/fork { messageID? }`.
- SDK: `client.session.fork({ path: { id }, body: { messageID? } })`.

Checkpointing within a session is available via:

- TUI: `/undo` and `/redo` for file edits.
- API: `POST /session/:id/revert { messageID, partID? }` and `POST /session/:id/unrevert`.

## Human-in-the-Loop Resume

OpenCode supports human-in-the-loop resumption, but **not** through stock `opencode run`. That command disables the `question` tool and auto-rejects runtime permission prompts. For Claudine-style brokering:

1. Start or attach to an OpenCode server (`opencode serve` or SDK `createOpencode()`).
2. Create or identify a session via `client.session.create()` or `client.session.list()`.
3. Subscribe to events via `client.event.subscribe()`.
4. When a `permission.asked` or `question.asked` event arrives, capture it.
5. Answer through `POST /session/:id/permissions/:permissionID` or by sending a user message into the session.
6. The session continues from the same `sessionID`.

## Interruption Recovery

Because session state is persisted continuously to the local SQLite database, sessions survive most interruptions:

- **Crash / terminal close / process kill**: the database remains and the session can be resumed.
- **Ctrl+C**: the session is preserved; resume with `--continue` or `--session`.
- **Pending tool call / approval / question**: if the server process is still running, the pending request remains blocked until answered. If the server is killed, resuming from the DB restores the conversation but may reset in-flight interactive state.

## Observability

Surfaces that expose session identity or resumability:

- **`opencode run --format json`**: every event carries `sessionID`.
- **`opencode session list --format json`**: lists all sessions with IDs, titles, and directories.
- **`opencode export [sessionID]`**: dumps session data as JSON.
- **Server SSE stream**: `GET /event` and SDK `client.event.subscribe()` stream lifecycle and request events.
- **Server session status**: `GET /session/status` returns `{ [sessionID]: SessionStatus }`.
- **Server health**: `GET /global/health` returns version and liveness.

## Quirks and Gaps

Quirks:

- `opencode run` does not print a dedicated session ID in default mode; use `--format json` or `session list`.
- `opencode run` is intentionally non-interactive and will not broker questions or permissions.
- `--continue` can select an unexpected session when multiple directories share a project record.
- Forking copies full message history; very long sessions may be slow to fork.
- Shared sessions upload conversation history to OpenCode servers; keep `share: disabled` or `OPENCODE_AUTO_SHARE=false` for local-only data.
- Direct parsing of `opencode.db` or auxiliary storage files is unsupported and unstable.

Gaps:

- Exact concurrency guarantees when two clients write to the same session are not documented.
- Whether approval state is fully restored on resume is not explicitly documented.
- Windows session storage paths are inferred from Node.js conventions, not locally verified.
- Retention sweep timing and behavior for active sessions are not documented.

## Claudine Integration Notes

For Claudine's lifecycle `resume` action and future HITL broker:

- Capture `sessionID` from `opencode run --format json` or from `opencode session list --format json`.
- Use `opencode run --session <id> "<follow-up>"` for simple non-interactive continuation.
- Do **not** rely on `opencode run` for human-in-the-loop; instead run or attach to an OpenCode server and use the SDK/HTTP API.
- Subscribe to server events to detect `permission.asked` and `question.asked`, and answer through the appropriate API endpoints.
- Treat the SQLite database and snapshot directories as internal storage; use export/import or the API for structured access.
- When resuming after failure, be aware that model and directory can be overridden but approval-state restore is undocumented.

## Changelog

- 2026-07-02: Converted to schema-validated frontmatter.
- 2026-07-02: Updated against OpenCode CLI 1.17.13 and current server/API documentation.
- 2026-07-02: Reclassified continuity model as `server_session` (local SQLite-backed server) rather than transcript replay.
- 2026-07-02: Added headless server, HTTP API, and SDK resume modes with exact invocations.
- 2026-07-02: Documented that `opencode run` disables the question tool and auto-rejects permissions, so HITL workflows require the server/SDK surface.
- 2026-07-02: Added local storage paths, observability surfaces, and gaps around concurrency and approval-state restore.

## Sources

- [OpenCode CLI docs](https://opencode.ai/docs/cli)
- [OpenCode Server docs](https://opencode.ai/docs/server)
- [OpenCode SDK docs](https://opencode.ai/docs/sdk)
- [OpenCode Share docs](https://opencode.ai/docs/share)
- [OpenCode ACP docs](https://opencode.ai/docs/acp)
- [OpenCode GitHub repository](https://github.com/anomalyco/opencode)
