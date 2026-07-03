---
$schema: ./_schema.yaml
created: 2026-04-02
last_updated: 2026-07-03
agent: codex
model: default
docs: https://opencode.ai/docs/cli/
support: first_class
continuity_model: server_session
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "opencode --continue"
      - "opencode --session <id>"
      - "opencode --fork"
      - "TUI /sessions, /resume, and /continue"
    accepts_followup_prompt: false
    selection_methods:
      - latest
      - id
      - picker
    notes: "The resumed TUI accepts the next prompt interactively. The picker is human-oriented and should not be treated as scriptable."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "opencode run --continue <message>"
      - "opencode run --session <id> <message>"
      - "opencode run --session <id> --fork <message>"
      - "opencode run --attach <url> --session <id> <message>"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
    notes: "Scriptable follow-up is first-class. Use --format json when Claudine needs raw event capture; stream parsing depth belongs to the non-interactive-sessions topic."
  - mode: headless_server
    supported: true
    mechanisms:
      - "opencode serve"
      - "opencode web"
      - "opencode attach <url>"
      - "HTTP /session and /session/:id/message endpoints"
    accepts_followup_prompt: true
    selection_methods:
      - id
      - picker
      - all_projects
    notes: "The server exposes session list, message, fork, abort, status, permission response, and event endpoints."
  - mode: api
    supported: true
    mechanisms:
      - "@opencode-ai/sdk createOpencode()"
      - "@opencode-ai/sdk session and event clients"
    accepts_followup_prompt: true
    selection_methods:
      - id
      - all_projects
    notes: "The SDK is a typed client over the OpenCode server; it can start a local server or connect to one."
  - mode: ide
    supported: true
    mechanisms:
      - "OpenCode IDE extension terminal launch/focus"
      - "opencode acp"
    accepts_followup_prompt: false
    selection_methods:
      - latest
      - picker
    notes: "IDE and ACP surfaces can reopen or focus OpenCode sessions, but this research did not verify an IDE-side scriptable follow-up contract."
session_id_capture:
  - surface: json_stream
    field: sessionID
    format: "ses_<base62-like-id>"
    notes: "Verified locally with opencode run --format json --model invalid-provider/invalid-model; even the early error event included sessionID. Stream-event depth belongs to non-interactive-sessions."
  - surface: cli_command
    field: id
    format: "JSON objects from opencode session list --format json"
    notes: "Verified locally with opencode 1.17.13. Objects include id, title, updated, created, projectId, and directory."
  - surface: cli_command
    field: info.id
    format: "JSON from opencode export <sessionID>"
    notes: "Verified locally. Export also includes projectID, directory, title, agent, model, version, permission, timestamps, messages, and parts."
  - surface: cli_command
    field: path
    format: "opencode db path"
    notes: "Verified locally; prints the SQLite database path. Useful for diagnostics, not a stable integration API."
  - surface: session_file
    field: id
    format: "SQLite session.id"
    notes: "Verified in local opencode.db. Direct DB parsing is internal and should be treated as evidence only."
  - surface: log_file
    field: run
    format: "short hex run id"
    notes: "Verified with --print-logs. Logs expose process run IDs and config load paths; they are not stable session handles."
resume_invocations:
  - mode: interactive
    invocation: "opencode --continue"
    accepts_prompt: false
    selection: latest
    notes: "Starts the TUI on the last session."
  - mode: interactive
    invocation: "opencode --session <session-id>"
    accepts_prompt: false
    selection: id
    notes: "Starts the TUI on a specific session ID."
  - mode: interactive
    invocation: "opencode --continue --fork or opencode --session <session-id> --fork"
    accepts_prompt: false
    selection: id
    notes: "Forks while continuing; the next prompt is typed in the TUI."
  - mode: interactive
    invocation: "/sessions"
    accepts_prompt: false
    selection: picker
    notes: "TUI command to list and switch sessions; aliases are /resume and /continue."
  - mode: non_interactive
    invocation: "opencode run --continue \"<message>\""
    accepts_prompt: true
    selection: latest
    notes: "Sends a follow-up prompt to the last session and prints the answer."
  - mode: non_interactive
    invocation: "opencode run --session <session-id> \"<message>\""
    accepts_prompt: true
    selection: id
    notes: "Sends a follow-up prompt to a specific session."
  - mode: non_interactive
    invocation: "opencode run --session <session-id> --fork \"<message>\""
    accepts_prompt: true
    selection: id
    notes: "Forks the specified session and sends the prompt to the new session."
  - mode: non_interactive
    invocation: "opencode run --attach http://localhost:4096 --session <session-id> \"<message>\""
    accepts_prompt: true
    selection: id
    notes: "Uses an existing server instead of starting a fresh local server."
  - mode: headless_server
    invocation: "POST /session/:id/message"
    accepts_prompt: true
    selection: id
    notes: "Body accepts messageID?, model?, agent?, noReply?, system?, tools?, and parts; response includes message info and parts."
  - mode: headless_server
    invocation: "POST /session/:id/prompt_async"
    accepts_prompt: true
    selection: id
    notes: "Same body as /session/:id/message, but returns immediately with 204 No Content."
  - mode: headless_server
    invocation: "POST /session/:id/fork"
    accepts_prompt: false
    selection: id
    notes: "Forks at an optional messageID and returns a new session."
  - mode: api
    invocation: "client.session.prompt({ path: { id }, body: { parts: [...] } })"
    accepts_prompt: true
    selection: id
    notes: "SDK wrapper around the server message endpoint."
state_storage:
  - location: local
    os: macos
    path: "~/.local/share/opencode/opencode.db"
    format: "SQLite plus WAL/SHM, snapshot git repositories, storage/session_diff JSON, and logs"
    retention: "OpenCode troubleshooting docs say the most recent 10 log files are kept; session database retention is not documented. Shared conversations persist until unshared."
    notes: "Verified on this macOS host with HOME=/Users/ken/.claudine. opencode db path printed /Users/ken/.claudine/.local/share/opencode/opencode.db."
  - location: local
    os: linux
    path: "~/.local/share/opencode/opencode.db"
    format: "SQLite plus WAL/SHM and adjacent data directories"
    retention: "Session database retention is not documented. Logs retain the most recent 10 files."
    notes: "Official troubleshooting docs use the same data directory for macOS and Linux; not locally verified on Linux."
  - location: local
    os: windows
    path: "%USERPROFILE%\\.local\\share\\opencode\\opencode.db"
    format: "SQLite plus adjacent data directories"
    retention: "Session database retention is not documented. Logs retain the most recent 10 files."
    notes: "Official troubleshooting docs direct Windows users to %USERPROFILE%\\.local\\share\\opencode. Not locally verified on Windows."
resume_scope:
  project_scoped: true
  cwd_scoped: true
  worktree_aware: true
  all_projects_supported: true
  branch_filtering: false
  notes: "Local tables store project.worktree and session.directory. opencode session list --format json returns projectId and directory. There is no documented branch selector or branch filter."
branching_checkpointing:
  branch_supported: true
  checkpoint_supported: true
  fork_invocation: "opencode --session <id> --fork, opencode run --session <id> --fork <message>, or POST /session/:id/fork"
  checkpoint_invocation: "TUI /undo and /redo for messages/file edits; POST /session/:id/revert and POST /session/:id/unrevert"
  preserves_original: true
  notes: "Fork returns a separate session. Revert/unrevert mutate visibility/state inside the same session rather than creating a new branch."
restored_state:
  transcript: true
  tool_results: true
  approvals: unknown
  model: overridable
  cwd: configurable
  env: current_process
  notes: "The database and export contain transcript messages, parts, session directory, agent, model, and session permission rules. CLI --model, --agent, --dir, --variant, and API message fields can override launch-time choices. Environment is the current process, with plugin hooks able to inject shell env."
hitl_resume:
  supported: true
  question_capture: "Use server events via SDK event.subscribe() or the HTTP event stream; session status is available from GET /session/status. Local non-interactive run sessions also show question: deny in session permission JSON."
  answer_injection: "POST /session/:id/permissions/:permissionID responds to permission requests. A user answer to an agent question is sent as a new message with POST /session/:id/message or SDK session.prompt."
  limitations: "Stock opencode run is designed for non-interactive automation and local evidence shows it creates sessions with question, plan_enter, and plan_exit denied. Claudine should use the server/API path for durable HITL brokering."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: true
  pending_approval_resume: false
  limitations: "Persisted sessions survive terminal close, Ctrl+C, and process death as conversation records. In-flight tool execution, permission prompts, and question waits are live server state; if the server dies, resume can continue the transcript but should not assume the exact pending call is still blocked and answerable. Concurrent writes to the same session are undocumented."
observability:
  stream_events:
    - "opencode run --format json raw events"
    - "GET /event server-sent event stream"
    - "SDK client.event.subscribe()"
    - "GET /session/status"
  hook_events:
    - "Plugin event hooks such as tool.execute.before"
    - "Plugin shell.env hook for execution environment injection"
  failure_modes:
    - "permission denied or auto-approved according to permission rules and --auto"
    - "session abort through POST /session/:id/abort"
    - "question/plan tools denied in non-interactive run sessions"
  notes: "For wrapper-grade capture, prefer JSON stream, server events, and session status over log scraping. Logs are useful diagnostics and include run id and config paths, but not a stable resume handle."
quirks:
  - "opencode run default output is formatted text; use --format json for raw events."
  - "opencode session list defaults to a table; use --format json for stable session ID capture."
  - "opencode export without a session ID is interactive; pass the ID in automation."
  - "The local SQLite schema is observable but internal; use CLI export/import, session list JSON, or the HTTP/SDK API for integrations."
  - "Non-interactive run sessions observed locally carry session permission rules denying question, plan_enter, and plan_exit."
  - "The official troubleshooting storage description mentions project/global storage, while current local 1.17.13 uses opencode.db as the session database plus adjacent storage directories."
  - "The Windows data path in current official docs is %USERPROFILE%\\.local\\share\\opencode, not %LOCALAPPDATA%\\opencode."
gaps:
  - "Exact ordering and locking semantics for two clients sending prompts to the same session at the same time are not documented."
  - "Whether remembered permission decisions are restored across every resume path is not documented."
  - "Linux and Windows storage layouts were not locally inspected in this run."
  - "The precise lifecycle of a pending permission/question after server crash is not documented."
  - "No official resume-specific page exists; the relevant facts are spread across CLI, TUI, server, SDK, troubleshooting, share, permissions, and plugin docs."
changes:
  - "2026-07-03: Reverified OpenCode 1.17.13 locally with opencode db path, session list --format json, opencode export, and SQLite schema inspection."
  - "2026-07-03: Corrected Windows storage path to the current official %USERPROFILE%\\.local\\share\\opencode location."
  - "2026-07-03: Added local evidence that non-interactive run sessions persist question, plan_enter, and plan_exit as denied session permissions."
  - "2026-07-03: Added opencode db path and opencode export as explicit session ID and diagnostic capture surfaces."
  - "2026-07-03: Verified that opencode run --format json emits sessionID on an early invalid-model error event."
  - "2026-07-03: Clarified that pending approvals/questions are not safely recoverable after server death even though the transcript remains resumable."
requires_claudine_update: true
reason: "Claudine's OpenCode resume/HITL implementation should capture session IDs from --format json or session list --format json, use server/API paths for durable human-in-the-loop continuation, and avoid assuming pending approvals survive server death."
---

# OpenCode CLI Resume Support

## Overview

OpenCode has first-class resume support. A session is a persisted local record with a stable `ses_...` ID, a project/worktree association, a session directory, transcript messages, message parts, model/agent metadata, and session permission state. The CLI can continue the latest session, resume an explicit session ID, fork a session, send a non-interactive follow-up prompt, or delegate to a running headless server. The server and SDK expose the same session model through list, message, fork, revert, abort, status, event, and permission-response APIs.

For Claudine, the main risk is not whether resume exists; it does. The risks are target selection and live-state assumptions. `--continue` is convenient but implicit, while `--session <id>` is the safer automation selector. The durable transcript survives process death, but a pending permission, pending question, or running tool is live server state and should not be assumed to survive a crashed or killed server.

## Resume Semantics

OpenCode's practical continuity model is `server_session`: the session is stored locally and served through an OpenCode backend. On this host, `opencode db path` returned:

```text
/Users/ken/.claudine/.local/share/opencode/opencode.db
```

The local SQLite database contains tables including `session`, `message`, `part`, `project`, `permission`, `session_context_epoch`, `session_input`, `session_message`, `session_share`, and `todo`. The observed `session` table stores `id`, `project_id`, `parent_id`, `slug`, `directory`, `title`, `version`, `revert`, `permission`, `agent`, `model`, cost/token fields, and timestamps. The observed `project` table stores `worktree` and VCS metadata. The observed `message` and `part` rows store JSON payloads for user prompts and message parts.

The applicable resume patterns are:

| Pattern | Supported | Continuity model |
|---------|-----------|------------------|
| Continue latest | Yes | Local server session selected by OpenCode |
| Resume by handle | Yes | Stable `ses_...` session ID |
| Interactive picker | Yes | TUI session selector |
| Non-interactive follow-up | Yes | `opencode run` sends another prompt into a session |
| Transcript replay | No as the primary model | Export/import exists, but ordinary resume uses the session store |
| Server-side session | Yes | Local headless server and SDK over local persistent state |
| Live-process attach | Yes | `opencode attach` and `opencode run --attach` connect to an existing server |
| Branch/fork/rewind/checkpoint | Yes | Fork creates a new session; undo/redo and revert mutate session state |
| Recovery resume | Partial | Transcript survives; exact in-flight tool/approval state is not guaranteed after server death |
| Human-in-the-loop resume | Yes through server/API | Not safe through stock `opencode run` alone |

A chat-history export is useful for backup, import, and inspection, but it is not the normal resume mechanism unless it is imported back into OpenCode. Memory files, `AGENTS.md`, config, plugins, and project instructions are context sources, not prior-session continuation by themselves.

## Supported Modes

| Mode | Entry point | Follow-up prompt at invocation | Selector | Automation fit |
|------|-------------|-------------------------------|----------|----------------|
| Interactive CLI | `opencode --continue` | No | Latest | Human-oriented; implicit target |
| Interactive CLI | `opencode --session <id>` | No | Session ID | Scriptable launch, interactive continuation |
| Interactive CLI | `opencode --session <id> --fork` | No | Session ID | Scriptable launch, interactive continuation |
| TUI | `/sessions`, `/resume`, `/continue` | No | Picker | Picker-only; do not automate |
| Non-interactive CLI | `opencode run --continue "<message>"` | Yes | Latest | Scriptable but implicit target |
| Non-interactive CLI | `opencode run --session <id> "<message>"` | Yes | Session ID | Best simple resume command for Claudine |
| Non-interactive CLI | `opencode run --attach <url> --session <id> "<message>"` | Yes | Session ID | Useful when Claudine owns a long-lived server |
| Headless server | `POST /session/:id/message` | Yes | Session ID | Best for lifecycle/HITL control |
| SDK | `client.session.prompt(...)` | Yes | Session ID | Typed wrapper over the server |
| IDE/ACP | IDE command or `opencode acp` | Not verified | Latest/picker | Human/editor surface |

Sessions created in non-interactive mode are persisted and resumable. Local session rows created by `opencode run` were present in `opencode.db`, appeared in `opencode session list --format json`, and exported successfully with `opencode export <sessionID>`.

Session names/titles are not equivalent to session IDs. `opencode session list --format json` returns titles and IDs, but resume commands take the ID. The interactive picker displays human session information and should not be treated as an automation selector.

## Session ID Capture

Reliable capture surfaces:

| Surface | Command or path | Fields observed or documented | Notes |
|---------|-----------------|-------------------------------|-------|
| JSON event stream | `opencode run --format json ...` | `sessionID` | Verified locally, including on an early invalid-model error event. Use this for live non-interactive wrapper capture. |
| Session list JSON | `opencode session list --format json --max-count N` | `id`, `title`, `updated`, `created`, `projectId`, `directory` | Verified locally. |
| Export JSON | `opencode export <sessionID>` | `info.id`, `info.projectID`, `info.directory`, `info.agent`, `info.model`, `info.permission`, `messages`, `parts` | Verified locally. |
| Database path | `opencode db path` | path to `opencode.db` | Diagnostic only; schema is internal. |
| Local DB | `session.id` | `ses_...` | Evidence source, not a stable API contract. |
| Logs | `--print-logs`, files under `log/` | `run=<hex>` | Run ID is not the resume session ID. |

The handle becomes available as soon as a session is created. For automation, capture it from `opencode run --format json` or from `opencode session list --format json`; do not scrape the table output.

## Resume Invocation

Continue the latest session interactively:

```bash
opencode --continue
```

Resume a specific session interactively:

```bash
opencode --session ses_0daa3c5a3ffefUerdMcdwktJMT
```

Fork while resuming:

```bash
opencode --session ses_0daa3c5a3ffefUerdMcdwktJMT --fork
```

Send a non-interactive follow-up to the latest session:

```bash
opencode run --continue "finish the task"
```

Send a non-interactive follow-up to a known session:

```bash
opencode run --session ses_0daa3c5a3ffefUerdMcdwktJMT "what is the next step?"
```

Attach to an existing server and send a prompt:

```bash
opencode serve --port 4096
opencode run --attach http://localhost:4096 --session ses_0daa3c5a3ffefUerdMcdwktJMT "continue"
```

Use the HTTP API:

```http
POST /session/ses_0daa3c5a3ffefUerdMcdwktJMT/message
Content-Type: application/json

{
  "parts": [
    { "type": "text", "text": "what is the next step?" }
  ]
}
```

Use the SDK:

```typescript
import { createOpencode } from "@opencode-ai/sdk"

const { client } = await createOpencode({ port: 4096 })
await client.session.prompt({
  path: { id: "ses_0daa3c5a3ffefUerdMcdwktJMT" },
  body: {
    parts: [{ type: "text", text: "what is the next step?" }],
  },
})
```

Structured answer capture is available through `opencode run --format json` and through the server/SDK response objects. The exact stream-event taxonomy belongs to the sibling `non-interactive-sessions` topic.

## Session Lookup Scope

OpenCode sessions are project- and directory-aware. Local inspection showed `project.worktree` for the rusty-biscuit worktree and `session.directory` for individual runs. `opencode session list --format json` returned `projectId` and `directory`, which is enough for Claudine to disambiguate sessions by repository/worktree path before choosing a handle.

`--continue` is an implicit latest-session selector. Claudine should avoid it when it has a captured session ID because it can target the wrong run after multiple sessions in the same project. There is no documented git branch filter, PR filter, or worktree branch selector. The server `GET /session` endpoint lists sessions, and the TUI picker can browse/switch sessions, but picker selection is not a scriptable API.

## State Storage

OpenCode stores application data on disk. Current troubleshooting docs state:

| OS | Official data directory |
|----|-------------------------|
| macOS | `~/.local/share/opencode/` |
| Linux | `~/.local/share/opencode/` |
| Windows | `%USERPROFILE%\.local\share\opencode` |

On this macOS host, with `HOME=/Users/ken/.claudine`, observed files included:

```text
~/.local/share/opencode/opencode.db
~/.local/share/opencode/opencode.db-wal
~/.local/share/opencode/opencode.db-shm
~/.local/share/opencode/storage/session_diff/*.json
~/.local/share/opencode/snapshot/<project-id>/
~/.local/share/opencode/log/*.log
```

The SQLite schema is not documented as stable. It is useful evidence for research and debugging, but Claudine should integrate through the CLI JSON surfaces or the HTTP/SDK API. Official troubleshooting docs say the most recent 10 log files are retained. Session retention/pruning behavior for unshared active sessions is not documented. Shared conversations are synced to OpenCode's servers and remain accessible until unshared.

## Restored State

Verified or documented as restored:

- Conversation transcript: stored as `message` and `part` rows and exported as JSON.
- Tool results and file diffs: exported/session API surfaces include message parts and diff endpoints.
- Session ID, title, project ID, directory, agent, model, version, token/cost summary, and timestamps.
- Session permission rules: local `opencode run` sessions contained a `permission` JSON field denying `question`, `plan_enter`, and `plan_exit`.
- Working directory: stored as `session.directory`; launch can override with `--dir`.
- Model/agent: stored in session metadata; launch/API can override with `--model`, `--agent`, `--variant`, or API message fields.

Not safely restored or not verified:

- Environment variables: use the current process environment; plugins can inject shell environment through `shell.env`.
- Approval prompts: remembered permission behavior across every resume path is undocumented.
- Pending tool calls and approvals: live server state may survive while the server process remains alive, but should not be assumed after server death.
- MCP server process state: config is reloaded for the current launch/server; warm server attachment can avoid cold starts, but a new process is not the old process.
- Attachments: `opencode run --file` adds files to a message; whether every attachment form is replayed identically on resume was not independently verified.

Resume appends to the existing session unless `--fork` or the fork API is used. Fork creates a new session and preserves the original.

## Branching and Checkpoints

OpenCode supports session branching and in-session rewind behavior:

| Feature | Surface | Preserves original |
|---------|---------|--------------------|
| Fork session | `opencode --session <id> --fork` | Yes |
| Fork with non-interactive prompt | `opencode run --session <id> --fork "<message>"` | Yes |
| Fork via API | `POST /session/:id/fork` with optional `messageID` | Yes |
| Rename/update title | `PATCH /session/:id` with `title` | Same session |
| Delete | `opencode session delete <sessionID>` or `DELETE /session/:id` | No |
| Undo/redo | TUI `/undo`, `/redo` | Same session |
| Revert/unrevert | `POST /session/:id/revert`, `POST /session/:id/unrevert` | Same session |
| Export/import | `opencode export`, `opencode import` | Import creates/restores from serialized data |
| Share/unshare | `/share`, `/unshare`, server share endpoints | Same session plus remote shared copy |

The TUI docs describe `/undo` as removing the most recent user message, subsequent responses, and file changes; `/redo` restores previously undone message/file changes. The server docs expose revert/unrevert at message/part granularity.

## Human-in-the-Loop Resume

OpenCode can support Claudine-style human-in-the-loop continuation through the server/API path:

1. Start or attach to a server with `opencode serve`, `opencode web`, SDK `createOpencode()`, or `opencode run --attach`.
2. Capture the session ID from run JSON, session list JSON, export JSON, or API creation/listing.
3. Subscribe to events with SDK `client.event.subscribe()` or the server event stream.
4. Use `GET /session/status` to observe blocked/running state.
5. Respond to permission requests with `POST /session/:id/permissions/:permissionID`.
6. Send a user answer or continuation with `POST /session/:id/message` or SDK `session.prompt`.

Stock `opencode run` is not enough for durable HITL brokering. Local session rows created by non-interactive runs contained:

```json
[
  { "permission": "question", "pattern": "*", "action": "deny" },
  { "permission": "plan_enter", "pattern": "*", "action": "deny" },
  { "permission": "plan_exit", "pattern": "*", "action": "deny" }
]
```

That is appropriate for non-interactive automation, but it means Claudine should not expect `opencode run` to pause for a human question in the same way the TUI/server can.

## Interruption Recovery

Durable recovery is good for completed transcript state:

- Terminal close, Ctrl+C, and process kill leave a session record that can be resumed by ID.
- A crashed client can be replaced by `opencode --session <id>`, `opencode run --session <id> "<message>"`, or server/API calls.
- A killed server can be restarted and pointed at the same local data directory.

Live recovery is weaker:

- A pending permission can be answered through `POST /session/:id/permissions/:permissionID` only while the server still knows that pending permission ID.
- A pending tool call may not be restartable as the same in-flight call after process death.
- A pending agent question should be modeled as a new user message after resume unless the server exposes a still-pending question event.
- Concurrent resumes/sends to the same session are not documented as safe, rejected, or serialized. Claudine should serialize its own writes per OpenCode session ID.

## Observability

Useful resume observability surfaces:

| Surface | What it reveals |
|---------|-----------------|
| `opencode run --format json` | Raw run events; session ID capture belongs here for non-interactive wrappers. |
| `opencode session list --format json` | Stable session IDs, titles, timestamps, project IDs, directories. |
| `opencode export <sessionID>` | Full exported session JSON for diagnostics and backup. |
| `opencode db path` | Local database path for diagnostics. |
| `GET /session/status` | Per-session status map. |
| `GET /session/:id` and `GET /session/:id/message` | Session metadata and message details. |
| `GET /event` / SDK `event.subscribe()` | Live server event stream. |
| Plugin hooks | Event and tool hooks; `shell.env` can reveal or modify execution environment. |
| Logs | Config load paths, run IDs, service messages, failures. |

Logs are diagnostic, not the primary resume API. The local `--print-logs` output exposed config load paths under `~/.config/opencode` and the current repo `.opencode`, plus a short `run` ID; it did not provide a stable resume handle.

## Quirks and Gaps

Quirks:

- `opencode session list` defaults to table output; use `--format json`.
- `opencode export` without a session ID prompts for selection; pass the ID in automation.
- `opencode run --continue` is scriptable but implicit; prefer `--session <id>`.
- Non-interactive run sessions deny `question`, `plan_enter`, and `plan_exit` at the session permission layer.
- The local database is easy to inspect but unsupported as a contract.
- Official docs do not have a single resume page; the behavior is distributed across CLI, TUI, server, SDK, troubleshooting, permissions, share, and plugins docs.
- Windows storage is under `%USERPROFILE%\.local\share\opencode` according to current official docs, which differs from many Node desktop conventions.

Gaps:

- Concurrent write semantics for one session are unknown.
- Remembered approval/permission restoration across all resume paths is unknown.
- Pending approval/question durability across server restart is unknown.
- Linux and Windows storage were not inspected locally.
- Exact retention/pruning for active unshared sessions is unknown.
- IDE and ACP resume semantics were not deeply verified beyond documented launch/focus and ACP availability.

## Claudine Integration Notes

For lifecycle `resume`, Claudine should store the OpenCode `ses_...` ID as soon as it sees it. The safest non-interactive resume command is:

```bash
opencode run --session <session-id> "<follow-up>"
```

When Claudine controls a longer recovery or HITL workflow, prefer a server-backed path:

```bash
opencode serve --port 4096
opencode run --attach http://localhost:4096 --session <session-id> "<follow-up>"
```

or use the SDK/HTTP API directly. Claudine should serialize writes per session ID, use `GET /session/status` and event subscription for lifecycle state, answer permission prompts through the permission endpoint, and send human answers as normal user messages when no still-pending question API state is available.

For `retry`, create a new session unless the user's intent is explicitly to continue the old transcript. For `proxy`, preserve the session ID and include the session export/list metadata in any handoff. For future HITL, do not depend on `opencode run` to pause for questions; use the server event stream and message/permission endpoints.

## Changelog

- 2026-07-03: Reverified OpenCode 1.17.13 locally with `opencode db path`, `opencode session list --format json`, `opencode export`, and SQLite schema inspection.
- 2026-07-03: Corrected Windows storage path to the current official `%USERPROFILE%\.local\share\opencode` location.
- 2026-07-03: Added local evidence that non-interactive run sessions persist `question`, `plan_enter`, and `plan_exit` as denied session permissions.
- 2026-07-03: Added `opencode db path` and `opencode export` as explicit session ID and diagnostic capture surfaces.
- 2026-07-03: Verified that `opencode run --format json` emits `sessionID` on an early invalid-model error event.
- 2026-07-03: Clarified that pending approvals/questions are not safely recoverable after server death even though the transcript remains resumable.
- 2026-07-02: Converted to schema-validated frontmatter.
- 2026-07-02: Updated against OpenCode CLI 1.17.13 and current server/API documentation.
- 2026-07-02: Reclassified continuity model as `server_session` rather than transcript replay.
- 2026-07-02: Added headless server, HTTP API, and SDK resume modes with exact invocations.
- 2026-07-02: Documented that `opencode run` disables question-style HITL workflows, so durable HITL requires the server/SDK surface.

## Sources

- [OpenCode CLI docs](https://opencode.ai/docs/cli/)
- [OpenCode TUI docs](https://opencode.ai/docs/tui/)
- [OpenCode Server docs](https://opencode.ai/docs/server/)
- [OpenCode SDK docs](https://opencode.ai/docs/sdk/)
- [OpenCode Troubleshooting docs](https://opencode.ai/docs/troubleshooting/)
- [OpenCode Permissions docs](https://opencode.ai/docs/permissions/)
- [OpenCode Share docs](https://opencode.ai/docs/share/)
- [OpenCode Plugins docs](https://opencode.ai/docs/plugins/)
- Local inspection on 2026-07-03: `opencode 1.17.13`, `opencode db path`, `opencode session list --format json`, `opencode export <sessionID>`, `~/.local/share/opencode/opencode.db`, and `~/.config/opencode/opencode.jsonc`.
