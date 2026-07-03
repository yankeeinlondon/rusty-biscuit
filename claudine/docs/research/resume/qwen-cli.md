---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: opencode
model: kimi-for-coding/k2p7
docs: https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/
support: first_class
continuity_model: transcript_replay
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "CLI /resume or /continue slash command"
      - "interactive session picker"
    accepts_followup_prompt: false
    selection_methods:
      - latest
      - picker
      - id
    notes: "Interactive TUI resume only; no command-line follow-up prompt. The picker is scoped to the current project/worktree."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "qwen --continue -p \"<prompt>\""
      - "qwen --resume <session-id> -p \"<prompt>\""
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
    notes: "Continue-latest selects the most recent session for the current project directory. Explicit --resume requires the session ID."
  - mode: headless_server
    supported: true
    mechanisms:
      - "qwen serve POST /session/:id/load"
      - "qwen serve POST /session/:id/resume"
      - "qwen serve POST /session/:id/prompt"
    accepts_followup_prompt: true
    selection_methods:
      - id
    notes: "Load replays full ACP history over SSE; resume restores the session without replay. Follow-up prompts are sent via POST /session/:id/prompt."
  - mode: ide
    supported: true
    mechanisms:
      - "VS Code extension resume"
      - "Zed adapter"
      - "JetBrains adapter"
    accepts_followup_prompt: false
    selection_methods:
      - picker
      - latest
      - id
    notes: "IDE adapters use the daemon ACP load/resume paths; this research focuses on CLI behavior."
  - mode: api
    supported: true
    mechanisms:
      - "qwen serve HTTP API"
    accepts_followup_prompt: true
    selection_methods:
      - id
    notes: "Programmatic control uses the daemon HTTP API or the TypeScript/Python/Java SDKs."
session_id_capture:
  - surface: json_stream
    field: session_id
    format: uuid
    notes: "Present in --output-format stream-json system/session_start event and in every subsequent event."
  - surface: stdout
    field: session_id
    format: uuid
    notes: "Present in --output-format json result array under the system/session_start object."
  - surface: hook
    field: session_id
    format: uuid
    notes: "All hook inputs include session_id and transcript_path."
  - surface: session_file
    field: filename
    format: "<session-id>.jsonl"
    notes: "Transcript path is ~/.qwen/projects/<sanitized-cwd>/chats/<session-id>.jsonl."
  - surface: cli_command
    field: sessionId
    format: uuid
    notes: "qwen sessions list --json emits sessionId per line, plus filePath and cwd."
  - surface: interactive_ui
    field: sessionId
    format: uuid
    notes: "The /resume picker shows session metadata; IDs are exposed via qwen sessions list --json."
  - surface: log_file
    field: transcript_path
    format: string
    notes: "Hook input carries transcript_path; debug logs may also reference the session."
resume_invocations:
  - mode: interactive
    invocation: "qwen (no args), then /resume or /continue"
    accepts_prompt: false
    selection: picker
    notes: "Opens the interactive session picker for the current project/worktree."
  - mode: non_interactive
    invocation: "qwen --continue -p \"<follow-up prompt>\""
    accepts_prompt: true
    selection: latest
    notes: "Continues the most recent session for the current project directory."
  - mode: non_interactive
    invocation: "qwen --resume <session-id> -p \"<follow-up prompt>\""
    accepts_prompt: true
    selection: id
    notes: "Resumes a specific session by UUID and sends a new prompt."
  - mode: non_interactive
    invocation: "qwen --resume"
    accepts_prompt: false
    selection: picker
    notes: "Opens the interactive picker even from a non-interactive launch context; not scriptable."
  - mode: headless_server
    invocation: "POST /session/:id/load"
    accepts_prompt: false
    selection: id
    notes: "Replays full history over SSE; clients must subscribe immediately to capture replay."
  - mode: headless_server
    invocation: "POST /session/:id/resume"
    accepts_prompt: false
    selection: id
    notes: "Restores the session internally without SSE history replay."
  - mode: api
    invocation: "POST /session/:id/prompt"
    accepts_prompt: true
    selection: id
    notes: "Sends a follow-up prompt into an existing daemon session."
state_storage:
  - location: local
    os: all
    path: "~/.qwen/projects/<sanitized-cwd>/chats/<session-id>.jsonl"
    format: JSONL
    retention: unknown
    notes: "Project-scoped by sanitized absolute working directory. The JSONL format is internal; direct parsing is unsupported but possible for debugging. Sessions can be archived to chats/archive/."
  - location: local
    os: all
    path: "<chatsDir>/<session-id>.worktree.json"
    format: JSON
    retention: unknown
    notes: "Sidecar that persists the active worktree binding for a session. Deleted automatically if the worktree directory is gone on resume."
resume_scope:
  project_scoped: true
  cwd_scoped: true
  worktree_aware: true
  all_projects_supported: false
  branch_filtering: false
  notes: "Default lookup is scoped to the current project directory. Worktree sessions are stored under the worktree's own sanitized cwd and must be resumed with --worktree again. There is no documented all-projects picker or git-branch filter."
branching_checkpointing:
  branch_supported: true
  checkpoint_supported: true
  fork_invocation: "/branch"
  checkpoint_invocation: "/rewind"
  preserves_original: true
  notes: "/branch forks the current conversation into a new session, leaving the original intact. /rewind (or /rollback) rewinds conversation to a previous turn within the same session. /restore reverts project files to the checkpoint before a tool call ran. /fork spawns a background agent that inherits the full conversation."
restored_state:
  transcript: true
  tool_results: true
  approvals: session_only
  model: overridable
  cwd: restored
  env: current_process
  notes: "Transcript and tool outputs are replayed from local JSONL. The permission_mode active at session end is restored (source=resume in SessionStart hook). Model and system prompts can be overridden at resume time. Environment comes from the launching process. Worktree binding is restored from the .worktree.json sidecar when present."
hitl_resume:
  supported: true
  question_capture: "PreToolUse hook receives tool_name/tool_input/permission_mode; PermissionRequest hook fires when a permission dialog is shown. In daemon/ACP mode, permission_request SSE events carry requestId and options."
  answer_injection: "PreToolUse hook returns hookSpecificOutput.permissionDecision allow/deny/ask and updatedInput. Daemon: POST /permission/:requestId with outcome selected/cancelled votes on the pending request."
  limitations: "CLI hooks do not provide a defer-and-resume loop equivalent to Claude Code's tool_deferred; the hook must return a decision synchronously within its timeout. Daemon permission requests time out after 5 minutes by default."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: true
  pending_approval_resume: false
  limitations: "JSONL transcripts are written continuously, so crash, Ctrl+C, terminal close, and process kill preserve the session. Pending tool calls are part of the transcript and are re-evaluated on resume. Pending approvals/permission requests are resolved as cancelled when the session terminates, so they cannot be answered after the process is gone; use the daemon API for durable permission mediation."
observability:
  stream_events:
    - "system/session_start"
    - "assistant"
    - "result"
    - "worktree_restored"
  hook_events:
    - "SessionStart"
    - "SessionEnd"
    - "PreToolUse"
    - "PermissionRequest"
    - "Notification"
  failure_modes:
    - "session_closed"
    - "session_died"
    - "client_evicted"
    - "stream_error"
  notes: "stream-json and json output include session_id in every event. SessionStart hook includes source (startup/resume/clear/compact) and permission_mode. Daemon SSE exposes session_update, permission_request, permission_resolved, session_closed, session_died, and status endpoints."
quirks:
  - "Worktree sessions are stored under the worktree's own sanitized cwd; resuming them requires passing --worktree <slug> again."
  - "The --worktree flag cannot be combined with --acp or --experimental-acp."
  - "Mid-session enter_worktree does NOT switch process.cwd; only the startup --worktree flag changes the working directory."
  - "Archived sessions return 409 session_archived for load/resume until unarchived via POST /sessions/unarchive."
  - "Concurrent daemon load/resume calls coalesce for the same action or return 409 restore_in_progress for mixed load/resume races."
  - "The JSONL transcript format is internal and not versioned for external consumers; use /export for stable output."
  - "qwen serve non-blocking prompts return 202 immediately; the caller must correlate turn_complete/turn_error SSE events by promptId."
gaps:
  - "No documented retention or cleanup policy for the ~/.qwen/projects JSONL files."
  - "Whether MCP server connection state and OAuth tokens survive resume is not explicitly documented."
  - "Exact concurrency behavior for concurrent CLI --resume of the same session is not documented."
  - "Whether approval state beyond permission_mode (e.g. per-tool allowlists) survives resume is not documented."
changes: []
requires_claudine_update: true
reason: "This is the first schema-validated resume research document for Qwen CLI and should feed into Claudine's lifecycle resume action, session-id capture logic, and daemon/ACP integration paths."
---

## Overview

Qwen Code (binary `qwen`) provides first-class session resumption based on local JSONL transcripts. A "session" is a persisted conversation transcript plus optional worktree sidecars stored on disk; resume means loading that transcript and appending new turns, not reattaching to remote server state. The same transcript is used for interactive TUI sessions, headless `-p` runs, and `qwen serve` daemon sessions.

## Resume Semantics

Resume reconstructs model context from the local JSONL history file. The CLI replays the transcript before sending a new prompt, so tool results and conversation state survive across launches. The authoritative state is the JSONL file under `~/.qwen/projects/<sanitized-cwd>/chats/`; deleting it makes the session ID unresumable. `qwen serve` can alternatively restore without replaying history over SSE via the `/resume` endpoint.

## Supported Modes

| Mode | Entry point | Follow-up prompt | Selector |
|------|-------------|------------------|----------|
| Interactive CLI | `qwen`, then `/resume` or `/continue` | No | Picker / latest / ID |
| Non-interactive CLI | `qwen --continue -p "..."` | Yes | Latest in current project |
| Non-interactive CLI | `qwen --resume <id> -p "..."` | Yes | Exact session ID |
| Daemon / ACP | `POST /session/:id/load` | Via `POST /session/:id/prompt` | Exact session ID |
| Daemon / ACP | `POST /session/:id/resume` | Via `POST /session/:id/prompt` | Exact session ID |
| IDE | VS Code / Zed / JetBrains adapters | No | Picker / latest / ID |

Interactive resume is picker-based and does not accept a follow-up prompt on the command line. Headless `-p` and daemon prompt endpoints are the scriptable surfaces for sending new prompts into an existing session.

## Session ID Capture

Stable session identifiers are UUIDs. Capture surfaces:

- **`--output-format stream-json`**: `session_id` appears in the `system/session_start` event and in every subsequent line-delimited event.
- **`--output-format json`**: `session_id` appears in the `system/session_start` object of the final buffered array.
- **Hooks**: every hook input JSON includes `session_id` and `transcript_path`.
- **Session file**: `~/.qwen/projects/<sanitized-cwd>/chats/<session-id>.jsonl`.
- **CLI listing**: `qwen sessions list --json` emits one JSON object per line with `sessionId`, `filePath`, and `cwd`.
- **Interactive picker**: `/resume` shows session metadata; raw IDs are best obtained from `qwen sessions list --json`.

## Resume Invocation

Continue the latest session:

```bash
qwen --continue -p "Run the tests again and summarize failures"
```

Resume a specific session:

```bash
qwen --resume 123e4567-e89b-12d3-a456-426614174000 -p "Apply the follow-up refactor"
```

Interactive picker:

```text
/resume
/continue
```

Daemon load (replay history over SSE):

```bash
curl -X POST http://127.0.0.1:4170/session/$SID/load
```

Daemon resume (restore without replay):

```bash
curl -X POST http://127.0.0.1:4170/session/$SID/resume
```

Daemon follow-up prompt:

```bash
curl -X POST http://127.0.0.1:4170/session/$SID/prompt \
  -H "Content-Type: application/json" \
  -d '{"prompt":[{"type":"text","text":"next step"}]}'
```

## Session Lookup Scope

Sessions are stored per project directory. The sanitized absolute working directory determines the project hash and therefore the storage location. The default `--continue` lookup is scoped to the current directory. Worktree sessions are stored under the worktree's own sanitized cwd and must be resumed with `--worktree <slug>` again; `--resume` without the matching `--worktree` will not find them. There is no documented all-projects picker or git-branch filter in the CLI.

## State Storage

Resumable state is local, not server-side.

| OS | Path |
|----|------|
| macOS / Linux / Windows | `~/.qwen/projects/<sanitized-cwd>/chats/<session-id>.jsonl` |

The `<sanitized-cwd>` directory name is derived from the absolute working directory. Each line is an internal JSON message. The format is internal and may change between releases; direct parsing is unsupported. For stable access, use `/export` or the daemon API. Archived sessions live in `chats/archive/`. Active worktree bindings are persisted in `<chatsDir>/<session-id>.worktree.json`.

## Restored State

Resuming restores:

- The full conversation transcript and tool outputs from the JSONL file.
- The `permission_mode` active when the session ended (exposed as `source: resume` in the `SessionStart` hook).
- The working directory and, via the sidecar, any active `--worktree` binding.

Resuming does not restore the environment of the original session; it uses the environment of the process that runs the resume command. Model and system prompts can be overridden with `--model`, `--system-prompt`, or `--append-system-prompt` at resume time.

## Branching and Checkpoints

- `/branch` forks the current conversation into a new session, leaving the original intact.
- `/rewind` (or `/rollback`) rewinds the conversation to a previous turn within the same session.
- `/restore [<ID>]` reverts project files to the checkpoint before a tool call ran.
- `/fork <directive>` spawns a background agent that inherits the full conversation.

## Human-in-the-Loop Resume

Qwen Code supports programmatic permission decisions but not a native "defer and resume later" loop for headless CLI hooks:

- `PreToolUse` hooks receive `tool_name`, `tool_input`, and `permission_mode`, and must return `hookSpecificOutput.permissionDecision` (`allow`, `deny`, or `ask`) synchronously, optionally with `updatedInput`.
- `PermissionRequest` hooks fire when a permission dialog is shown and can return a structured decision.
- In `qwen serve` / ACP mode, permission requests are surfaced as SSE `permission_request` events with a `requestId`; clients vote via `POST /permission/:requestId`. Permission requests time out after 5 minutes by default.

For durable human-in-the-loop continuation, Claudine should target the daemon API rather than the synchronous CLI hook path.

## Interruption Recovery

Because transcripts are written continuously, sessions survive most interruptions:

- **Crash / terminal close / process kill**: the JSONL file remains and can be resumed.
- **Ctrl+C**: the session is preserved; resume with `--continue` or `--resume`.
- **Pending tool calls**: preserved in the transcript and re-evaluated on resume.
- **Pending approvals / permission requests**: resolved as `cancelled` when the session terminates, so they cannot be answered after the process is gone. Use the daemon API for durable permission mediation.
- **Network failure**: `QWEN_CODE_UNATTENDED_RETRY=1` retries transient 429/529 errors indefinitely.

## Observability

Events and surfaces that expose session identity or resumability:

- **`--output-format json` / `stream-json`**: `system/session_start`, `assistant`, `result`, and `worktree_restored` events carry `session_id`.
- **Hooks**: `SessionStart` (includes `source: resume` and `permission_mode`), `SessionEnd`, `PreToolUse`, `PermissionRequest`, `Notification`.
- **Daemon SSE**: `session_update`, `permission_request`, `permission_resolved`, `session_closed`, `session_died`, `client_evicted`, `stream_error`.
- **Status endpoints**: `GET /workspace/:id/sessions`, `GET /session/:id/status`, `GET /session/:id/tasks`.

## Quirks and Gaps

Quirks:

- Worktree sessions are stored under the worktree's own sanitized cwd; resuming them requires passing `--worktree <slug>` again.
- `--worktree` cannot be combined with `--acp` / `--experimental-acp`.
- Mid-session `enter_worktree` does not switch `process.cwd`; only the startup `--worktree` flag changes the working directory.
- Archived sessions return `409 session_archived` for `load`/`resume` until unarchived via `POST /sessions/unarchive`.
- Concurrent daemon `load`/`resume` calls coalesce for the same action or return `409 restore_in_progress` for mixed races.
- The JSONL transcript format is internal and not versioned for external consumers; use `/export` for stable output.
- `qwen serve` non-blocking prompts return HTTP 202 immediately; the caller must correlate `turn_complete` / `turn_error` SSE events by `promptId`.

Gaps:

- No documented retention or cleanup policy for `~/.qwen/projects` JSONL files.
- Whether MCP server connection state and OAuth tokens survive resume is not explicitly documented.
- Exact concurrency behavior for concurrent CLI `--resume` of the same session is not documented.
- Whether approval state beyond `permission_mode` (e.g. per-tool allowlists) survives resume is not documented.

## Claudine Integration Notes

For Claudine's lifecycle `resume` action and future HITL broker:

- Capture `session_id` from the `system/session_start` event of `qwen -p --output-format stream-json` or from a `SessionStart` hook.
- Use `qwen --resume <session-id> -p "<follow-up>"` for non-interactive continuation.
- For human-in-the-loop, prefer the `qwen serve` ACP path: subscribe to SSE, handle `permission_request` events, and vote via `POST /permission/:requestId`.
- Treat the JSONL transcript as read-only and unstable; do not build parsing logic against it.
- When resuming worktree sessions, pass the original `--worktree <slug>` alongside `--resume <session-id>`.
- Be aware that CLI `PreToolUse` hooks must decide synchronously; there is no equivalent to Claude Code's `tool_deferred` stop reason.

## Sources

- [Qwen Code Headless Mode](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/)
- [Qwen Code Commands](https://qwenlm.github.io/qwen-code-docs/en/users/features/commands/)
- [Qwen Code Hooks](https://qwenlm.github.io/qwen-code-docs/en/users/features/hooks/)
- [Qwen Code Worktrees](https://qwenlm.github.io/qwen-code-docs/en/users/features/worktree/)
- [Qwen Code Approval Mode](https://qwenlm.github.io/qwen-code-docs/en/users/features/approval-mode/)
- [Qwen Code Session Lifecycle & Identity](https://qwenlm.github.io/qwen-code-docs/en/developers/daemon/08-session-lifecycle/)
- [Qwen Code qwen serve HTTP protocol reference](https://qwenlm.github.io/qwen-code-docs/en/developers/qwen-serve-protocol/)
