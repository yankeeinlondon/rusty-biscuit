---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default
docs: https://qwenlm.github.io/qwen-code-docs/en/users/common-workflow/
support: first_class
continuity_model: mixed
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "qwen --continue"
      - "qwen --resume"
      - "/resume or /continue slash command"
      - "interactive session picker"
    accepts_followup_prompt: false
    selection_methods:
      - latest
      - id
      - picker
    notes: "Interactive continue-latest and picker resume are documented. The picker is human-oriented; current docs describe selection by list metadata, not a stable machine API."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "qwen --continue -p \"<prompt>\""
      - "qwen --resume <session-id> -p \"<prompt>\""
      - "qwen --continue --fork-session -p \"<prompt>\""
      - "qwen --resume <session-id> --fork-session -p \"<prompt>\""
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
    notes: "Headless resume is scriptable and project-scoped. Current 0.19.6 code rejects --continue with --resume, rejects --session-id with resume flags, and permits --fork-session only with --continue or --resume."
  - mode: headless_server
    supported: true
    mechanisms:
      - "qwen serve POST /session/:id/load"
      - "qwen serve POST /session/:id/resume"
      - "qwen serve POST /session/:id/prompt"
      - "qwen serve qwen/control/session/continue"
    accepts_followup_prompt: true
    selection_methods:
      - id
    notes: "Daemon load replays ACP history; resume restores without replay. Prompt submission is separate and FIFO-queued per session."
  - mode: api
    supported: true
    mechanisms:
      - "qwen serve HTTP API"
      - "ACP bridge and SDK daemon clients"
    accepts_followup_prompt: true
    selection_methods:
      - id
    notes: "The HTTP API exposes session load/resume, prompt, cancel, metadata, archive, unarchive, and delete routes. The bridge still uses ACP unstable_resumeSession internally while advertising the stable daemon session_resume capability."
  - mode: ide
    supported: true
    mechanisms:
      - "VS Code/Zed/JetBrains daemon adapters"
    accepts_followup_prompt: true
    selection_methods:
      - id
      - picker
    notes: "IDE adapters are daemon clients; this document treats the daemon API as the integration surface and does not verify each IDE UI."
session_id_capture:
  - surface: stdout
    field: session_id/sessionId
    format: uuid
    notes: "Headless --output-format json emits structured output; current docs say it contains the full conversation log. Stream details belong to the non-interactive-sessions topic."
  - surface: json_stream
    field: session_id/sessionId
    format: uuid
    notes: "Stream-json output is documented for real-time message objects. Current source has explicit stream-json mode and session-start event handling, but stream event field depth is outside this topic."
  - surface: session_file
    field: sessionId
    format: uuid
    notes: "Observed local transcripts use JSONL records with sessionId and filenames <session-id>.jsonl under ~/.qwen/projects/<sanitized-cwd>/chats."
  - surface: cli_command
    field: sessionId
    format: JSON Lines
    notes: "Current docs specify qwen sessions list --json emits sessionId, startTime, mtime, prompt, gitBranch, customTitle, titleSource, filePath, and cwd. Installed 0.15.6 on this host does not yet support this subcommand."
  - surface: hook
    field: session_id/transcript_path
    format: uuid/path
    notes: "Hook payloads include session identity for lifecycle events; this was verified from current source/local hook schemas rather than a dedicated resume doc."
  - surface: interactive_ui
    field: session picker row
    format: human-readable metadata
    notes: "The picker shows summary/initial prompt, elapsed time, message count, and git branch; use the CLI list or transcript filename for scriptable IDs."
resume_invocations:
  - mode: interactive
    invocation: "qwen --continue"
    accepts_prompt: false
    selection: latest
    notes: "Resumes the most recent conversation for the current project and opens the interactive UI."
  - mode: interactive
    invocation: "qwen --resume"
    accepts_prompt: false
    selection: picker
    notes: "Opens the conversation picker. Not suitable for automation."
  - mode: interactive
    invocation: "/resume or /continue"
    accepts_prompt: false
    selection: picker
    notes: "Slash command resumes a previous session from inside the TUI."
  - mode: non_interactive
    invocation: "qwen --continue -p \"<follow-up prompt>\""
    accepts_prompt: true
    selection: latest
    notes: "Headless continue-latest for the current project."
  - mode: non_interactive
    invocation: "qwen --resume <session-id> -p \"<follow-up prompt>\""
    accepts_prompt: true
    selection: id
    notes: "Headless exact-session resume by UUID."
  - mode: non_interactive
    invocation: "qwen --resume <session-id> --fork-session -p \"<follow-up prompt>\""
    accepts_prompt: true
    selection: id
    notes: "Creates a new forked session from the resumed transcript, then sends the prompt into the fork."
  - mode: headless_server
    invocation: "POST /session/:id/load"
    accepts_prompt: false
    selection: id
    notes: "Restores a persisted session and replays full ACP history to clients."
  - mode: headless_server
    invocation: "POST /session/:id/resume"
    accepts_prompt: false
    selection: id
    notes: "Restores a persisted session without replaying history over the stream."
  - mode: headless_server
    invocation: "POST /session/:id/prompt"
    accepts_prompt: true
    selection: id
    notes: "Submits a follow-up prompt to a loaded/resumed daemon session."
  - mode: headless_server
    invocation: "qwen/control/session/continue"
    accepts_prompt: false
    selection: id
    notes: "Daemon control path attempts to continue an interrupted last turn without adding a synthetic user prompt."
state_storage:
  - location: local
    os: macos
    path: "~/.qwen/projects/<sanitized-absolute-cwd>/chats/<session-id>.jsonl"
    format: "JSONL internal transcript records"
    retention: unknown
    notes: "Observed on this macOS host under /Users/ken/.qwen/projects/.../chats. The process obeys HOME/runtimeOutputDir; this session's sandbox HOME also produced /Users/ken/.claudine/.qwen debug files."
  - location: local
    os: linux
    path: "~/.qwen/projects/<sanitized-absolute-cwd>/chats/<session-id>.jsonl"
    format: "JSONL internal transcript records"
    retention: unknown
    notes: "Docs use the POSIX ~ path for Linux/macOS. Direct parsing is possible but unsupported; prefer qwen sessions list or daemon routes where available."
  - location: local
    os: windows
    path: "%USERPROFILE%\\.qwen\\projects\\<sanitized-absolute-cwd>\\chats\\<session-id>.jsonl"
    format: "JSONL internal transcript records"
    retention: unknown
    notes: "Windows path is inferred from Qwen's use of the user's home directory and Node path joining. The sanitized project id differs because Windows absolute paths differ."
  - location: local
    os: macos
    path: "~/.qwen/projects/<sanitized-absolute-cwd>/chats/archive/<session-id>.jsonl"
    format: "JSONL internal transcript records"
    retention: unknown
    notes: "Daemon archive moves active chats into chats/archive; archived sessions must be unarchived before load/resume."
  - location: local
    os: linux
    path: "~/.qwen/projects/<sanitized-absolute-cwd>/chats/archive/<session-id>.jsonl"
    format: "JSONL internal transcript records"
    retention: unknown
    notes: "Same archive semantics as macOS."
  - location: local
    os: windows
    path: "%USERPROFILE%\\.qwen\\projects\\<sanitized-absolute-cwd>\\chats\\archive\\<session-id>.jsonl"
    format: "JSONL internal transcript records"
    retention: unknown
    notes: "Same archive semantics as POSIX paths, with Windows separators/home."
resume_scope:
  project_scoped: true
  cwd_scoped: true
  worktree_aware: true
  all_projects_supported: false
  branch_filtering: true
  notes: "Session service constructs storage from the launch cwd and filters records by project hash derived from the recorded cwd. The picker shows git branch and current docs/blogs describe search plus branch filtering in /resume. There is no documented all-projects resume selector."
branching_checkpointing:
  branch_supported: true
  checkpoint_supported: true
  fork_invocation: "/branch or qwen --resume <id> --fork-session"
  checkpoint_invocation: "/rewind or /rollback; /restore <ID> for file checkpoint restore"
  preserves_original: true
  notes: "/branch forks the current conversation into a new session. Current CLI also has --fork-session with --continue/--resume. /rewind rewrites the active conversation path by parentUuid chain; /restore reverts files to a tool-call checkpoint."
restored_state:
  transcript: true
  tool_results: true
  approvals: unknown
  model: overridable
  cwd: current_launch_dir
  env: current_process
  notes: "Transcript records and tool outputs are replayed from JSONL. Docs claim the resumed conversation starts with the same model/configuration, but current CLI still resolves launch-time argv/settings, permits --model/system-prompt overrides, and constructs SessionService from current cwd. Approval mode is resolved from current argv/settings; only daemon live entries retain pending permission state while alive."
hitl_resume:
  supported: true
  question_capture: "CLI hooks can observe permission-related events synchronously. Daemon/ACP sessions expose permission requests with requestId, options, sessionId, and originator client identity on the event stream."
  answer_injection: "CLI hooks must answer synchronously. Daemon clients submit permission votes through POST /permission/:requestId or ACP session/permission with requestId/sessionId and selected outcome."
  limitations: "This is live-process/daemon mediation, not durable offline HITL. On session termination, pending daemon permissions are cancelled with reason session_closed, and CLI hook prompts cannot be deferred across process exit."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: true
  pending_approval_resume: false
  limitations: "Completed records are appended continuously, so crash/kill/Ctrl+C preserve prior transcript state. The daemon has an explicit continue-last-turn control path for interrupted prompts or dangling tool calls. Pending approvals are cancelled when the daemon session terminates; concurrent daemon load/resume calls coalesce or return RestoreInProgressError. Concurrent standalone CLI resumes of the same JSONL are not documented as safe."
observability:
  stream_events:
    - "session_start"
    - "session_update"
    - "permission_request"
    - "permission_resolved"
    - "session_metadata_updated"
    - "session_closed"
    - "session_died"
    - "client_evicted"
    - "stream_error"
  hook_events:
    - "SessionStart"
    - "SessionEnd"
    - "PreToolUse"
    - "PermissionRequest"
    - "UserPromptSubmit"
  failure_modes:
    - "SessionNotFoundError"
    - "WorkspaceMismatchError"
    - "RestoreInProgressError"
    - "session_closed"
    - "session_died"
    - "client_evicted"
    - "stream_error"
  notes: "qwen sessions list --json is the cleanest scriptable persisted-session lookup in current docs. Daemon SSE and status routes expose live lifecycle state; transcript JSONL and debug logs expose local forensic details but are internal formats."
quirks:
  - "Installed qwen on this host is 0.15.6 while npm latest is 0.19.6; 0.15.6 lacks the documented qwen sessions subcommand."
  - "Session lookup is based on the current launch cwd/project hash, so a known UUID is not enough if launched from the wrong project/worktree scope."
  - "The JSONL transcript format is internal and parentUuid-based; direct mutation or parsing as an integration contract is risky."
  - "qwen --session-id cannot be combined with --continue or --resume; it is only for starting a new session with a specific UUID."
  - "--fork-session is valid only with --continue or --resume and creates a new UUID before continuing."
  - "qwen serve POST /session/:id/load and /resume have different stream behavior: load replays history; resume restores without replay."
  - "Archived sessions are not loadable/resumable until unarchived."
  - "Daemon prompt calls have no server-side timeout in the documented Stage 1 protocol; callers must set their own timeout or cancel."
  - "Pending permission requests are live daemon state and are cancelled on session close/die; they are not recoverable from JSONL alone."
gaps:
  - "Exact standalone CLI behavior for simultaneous qwen --resume <same-id> processes is not documented."
  - "The full set of launch-time configuration that is restored versus recalculated is not documented; current source suggests argv/settings are recalculated while transcript is restored."
  - "Whether MCP connection/OAuth runtime state survives CLI transcript resume was not verified."
  - "Windows storage was inferred from Node home/path behavior and docs; no Windows host probe was available."
  - "Retention/cleanup policy for active JSONL transcripts is undocumented."
changes:
  - "Updated from stale 2026-07-02 research to current Qwen Code 0.19.6 docs/source while preserving local 0.15.6 transcript evidence."
  - "Changed continuity_model from transcript_replay to mixed because daemon resume combines transcript restoration with live ACP child/session state."
  - "Added qwen sessions list --json as the documented scriptable session lookup surface."
  - "Added CLI --fork-session and daemon archive/delete/unarchive metadata behavior."
  - "Corrected state_storage to one record per OS and separated active versus archived paths."
  - "Recorded observed local transcript fields and noted installed-version gaps."
requires_claudine_update: true
reason: "Claudine should account for Qwen's current first-class resume surfaces: qwen sessions list --json for ID lookup, headless --continue/--resume with follow-up prompt, --fork-session, and daemon load/resume/prompt semantics. Existing provider metadata based only on older CLI behavior would miss scriptable lookup and fork/archive states."
---

# Qwen CLI Resume Behavior

## Overview

Qwen Code has first-class resume support for both human and automated workflows. The CLI persists conversation transcripts as project-scoped JSONL files and reconstructs model context from those files when using `--continue`, `--resume <session-id>`, the `/resume` slash command, or the session picker. Headless mode can resume and immediately send a follow-up prompt, which makes it usable for Claudine's lifecycle `resume` action.

The continuity model is mixed. Standalone CLI resume is local transcript replay: the transcript file is authoritative, and Qwen rebuilds history from persisted records. `qwen serve` adds a live daemon model: HTTP clients can create, attach to, load, resume, prompt, cancel, and close ACP-backed sessions, with live event streams and permission mediation. The main wrapper risks are scope and state assumptions: session lookup is current-project/current-cwd scoped, JSONL is an internal format, and launch-time config, approval policy, environment, and working directory should not be treated as fully restored unless the wrapper supplied them.

## Resume Semantics

For the standalone CLI, a session is a persisted local transcript under the Qwen runtime directory. On this macOS host, real transcripts were observed under `/Users/ken/.qwen/projects/<sanitized-cwd>/chats/<session-id>.jsonl`. Each line is a JSON object with fields such as `uuid`, `parentUuid`, `sessionId`, `timestamp`, `type`, `cwd`, `version`, `gitBranch`, and type-specific payloads. Observed record types include `user`, `assistant`, `tool_result`, and `system`. Assistant records include `model`, `message.parts`, `usageMetadata`, and `contextWindowSize`; tool result records preserve the function response; system records preserve telemetry and slash-command events.

Current source reconstructs a linear conversation by walking the `parentUuid` chain from the last record, aggregating records with the same UUID. This means Qwen can preserve dead branches inside one JSONL while resume follows the active leaf. The `/rewind` behavior relies on this tree shape: older records remain on disk, but the active parent chain changes so resume ignores abandoned descendants.

The following resume patterns apply:

| Pattern | Supported | Continuity model | Notes |
|---|---:|---|---|
| Continue latest | Yes | Local transcript replay | `qwen --continue`, optionally with `-p` in headless mode. |
| Resume by handle | Yes | Local transcript replay | `qwen --resume <session-id>`, optionally with `-p`. |
| Interactive picker | Yes | Local transcript replay | `qwen --resume` and `/resume` show a human picker. |
| Non-interactive follow-up | Yes | Local transcript replay | `qwen --continue -p ...` and `qwen --resume <id> -p ...`. |
| Transcript replay | Yes | Local JSONL | The transcript is authoritative for standalone CLI resume. |
| Server-side session | Yes | Daemon ACP session plus transcript restoration | `qwen serve` exposes load/resume/prompt routes. |
| Live-process attach | Yes | Daemon session attach | Daemon clients attach to live sessions by session ID/client ID. |
| Branch/fork/rewind/checkpoint | Yes | Parent-linked transcript plus file checkpoints | `/branch`, `--fork-session`, `/rewind`, `/restore`. |
| Recovery resume | Partial | Transcript replay plus daemon continue control | Completed records survive; live pending approvals do not survive process death. |
| Human-in-the-loop resume | Partial | Live daemon mediation | Durable offline permission resume is not supported. |

Exports, memory files, and project instructions are not resume mechanisms by themselves. `/export` can serialize session history, and memory/context files can influence future sessions, but a wrapper should treat only provider-supported transcript/session loading as resume.

## Supported Modes

| Mode | Surface | Follow-up prompt | Selector | Automation fit |
|---|---|---:|---|---|
| Interactive CLI | `qwen --continue` | No | Latest | Human-oriented. |
| Interactive CLI | `qwen --resume` | No | Picker | Not scriptable. |
| Interactive CLI | `/resume` or `/continue` | No | Picker | Not scriptable. |
| Non-interactive CLI | `qwen --continue -p "..."` | Yes | Latest | Scriptable. |
| Non-interactive CLI | `qwen --resume <id> -p "..."` | Yes | Session ID | Scriptable. |
| Non-interactive CLI | `qwen --resume <id> --fork-session -p "..."` | Yes | Session ID | Scriptable branch/fork. |
| CLI lookup | `qwen sessions list --json` | N/A | Current project | Scriptable session discovery. |
| Daemon/API | `POST /session/:id/load` | Via prompt route | Session ID | Scriptable live/session restore. |
| Daemon/API | `POST /session/:id/resume` | Via prompt route | Session ID | Scriptable restore without replay. |
| Daemon/API | `POST /session/:id/prompt` | Yes | Session ID | Scriptable follow-up. |

Current docs say `--continue` resumes the most recent conversation and `--resume` displays a picker. Headless docs add `qwen --continue -p "<prompt>"` and `qwen --resume <session-id> -p "<prompt>"` for scripts. Current 0.19.6 source validates that `--continue` and `--resume` are mutually exclusive, that `--session-id` cannot be combined with either resume flag, and that `--fork-session` is only valid with `--continue` or `--resume`.

Sessions created by non-interactive runs are persisted to the same project-scoped JSONL storage when chat recording is enabled, so they are resumable by the same mechanisms. The installed 0.15.6 binary on this host did create transcripts for short headless test runs. The current `qwen sessions list` subcommand is documented in 0.19.6 docs but was a negative probe on this host's older 0.15.6 binary.

## Session ID Capture

The stable handle is the session UUID. Capture surfaces are:

- Local transcript path: `<session-id>.jsonl` under `~/.qwen/projects/<sanitized-cwd>/chats/`.
- Transcript records: every observed JSONL line contains `sessionId`.
- `qwen sessions list --json`: documented JSON Lines output with `sessionId`, `startTime`, `mtime`, `prompt`, `gitBranch`, `customTitle`, `titleSource`, `filePath`, and `cwd`.
- Headless JSON/stream-json output: current CLI supports `--output-format json` and `--output-format stream-json`; stream detail belongs to the non-interactive-sessions topic, but session identity is part of the structured event surface.
- Hooks: Qwen hook schemas include session identity and transcript path for lifecycle/tool events.
- Daemon responses/events: `POST /session`, load/resume responses, session events, permission events, metadata updates, close/die frames, and heartbeat responses all carry session identity.

The handle is available as soon as the session starts. For wrapper logic, the safest path is to capture the session ID from structured output when launching headless, or query `qwen sessions list --json` for current-project sessions after the run. Directly scanning JSONL files works for recovery/forensics, but the format is internal.

## Resume Invocation

Continue the latest session interactively:

```bash
qwen --continue
```

Continue the latest session headlessly:

```bash
qwen --continue -p "Run the tests again and summarize failures"
```

Open the picker:

```bash
qwen --resume
```

Resume a specific session headlessly:

```bash
qwen --resume 123e4567-e89b-12d3-a456-426614174000 -p "Apply the follow-up refactor"
```

Fork a prior session and send the follow-up into the fork:

```bash
qwen --resume 123e4567-e89b-12d3-a456-426614174000 --fork-session -p "Try the alternate implementation"
```

List sessions for the current project:

```bash
qwen sessions list --json --limit 50
```

Daemon load/resume and prompt:

```bash
curl -X POST http://127.0.0.1:4170/session/$SID/load
curl -X POST http://127.0.0.1:4170/session/$SID/resume
curl -X POST http://127.0.0.1:4170/session/$SID/prompt \
  -H 'Content-Type: application/json' \
  -d '{"prompt":[{"type":"text","text":"Continue the investigation"}]}'
```

`POST /session/:id/load` and `POST /session/:id/resume` do not themselves supply a user prompt. The prompt is sent through `POST /session/:id/prompt`. The protocol docs distinguish these restore paths: load replays full ACP history, while resume restores without replay.

## Session Lookup Scope

Standalone session lookup is project/current-directory scoped. Current source constructs `SessionService(cwd)`, derives a project directory from that cwd, lists JSONL files from that project's `chats/` directory, and filters candidates by a project hash derived from the first record's `cwd`. A known UUID launched from the wrong project scope can fail lookup.

Worktrees are scope-relevant. The worktree docs state that resuming a session created inside a named worktree requires the same worktree context, for example `qwen --resume <id> --worktree foo`. Current docs and blog posts describe branch metadata and branch filtering in `/resume`; the picker/list surface includes `gitBranch`.

There is no documented all-projects resume selector. `qwen sessions list --json` is documented as recent session listing for the current project, not a global search across every `~/.qwen/projects/*` directory.

## State Storage

The core storage is local JSONL:

| OS | Active path | Format | Notes |
|---|---|---|---|
| macOS | `~/.qwen/projects/<sanitized-absolute-cwd>/chats/<session-id>.jsonl` | JSONL | Observed on this host under `/Users/ken/.qwen/...`. |
| Linux | `~/.qwen/projects/<sanitized-absolute-cwd>/chats/<session-id>.jsonl` | JSONL | Same documented POSIX path shape. |
| Windows | `%USERPROFILE%\.qwen\projects\<sanitized-absolute-cwd>\chats\<session-id>.jsonl` | JSONL | Inferred from home-directory/path behavior; not probed on Windows. |

Archive storage:

| OS | Archived path | Format | Notes |
|---|---|---|---|
| macOS | `~/.qwen/projects/<sanitized-absolute-cwd>/chats/archive/<session-id>.jsonl` | JSONL | Daemon archive moves the active JSONL here. |
| Linux | `~/.qwen/projects/<sanitized-absolute-cwd>/chats/archive/<session-id>.jsonl` | JSONL | Same as macOS. |
| Windows | `%USERPROFILE%\.qwen\projects\<sanitized-absolute-cwd>\chats\archive\<session-id>.jsonl` | JSONL | Same semantics with Windows separators. |

The current protocol also mentions preserved file history, subagent transcripts, and runtime sidecars during archive/delete operations. Those are related state, but the transcript JSONL is the resumable conversation record. Retention is undocumented. `general.chatRecording` controls whether chat history is saved; disabling it prevents `--continue` and `--resume` from working.

The JSONL format is not documented as a stable external API. Direct parsing is useful for recovery diagnostics and local validation, but Claudine should prefer CLI/daemon surfaces when available.

## Restored State

Restored:

- Conversation transcript and parent-linked active branch.
- Tool outputs/results recorded in the transcript.
- System records such as slash-command and telemetry records where present.
- Chat-compression checkpoints, per headless docs.
- Custom titles/tags, represented by `custom_title` system records.

Recalculated or launch-dependent:

- Working directory/project scope comes from the launch cwd and any `--worktree` handling.
- Environment variables come from the current process.
- Approval mode is resolved from current argv/settings in source; pending approvals are live daemon state, not transcript state.
- Model and system prompt are at least partly overridable at resume time; docs say the resumed conversation starts with the same model/configuration, but current source still resolves launch-time settings and supports model/system-prompt flags.
- MCP runtime connection state was not verified as restored.

Resume appends to the same session file for normal resume/continue. Forking creates a new session ID and copies/re-roots from the source before continuing, preserving the original.

## Branching and Checkpoints

Qwen supports several session-history operations:

- `/rename` or `/tag` renames/tags the current session by appending a `custom_title` system record.
- `/delete` and `qwen serve` `POST /sessions/delete` remove persisted session JSONL files.
- `/branch` forks the current conversation into a new session.
- `--fork-session` forks from `--continue` or `--resume`.
- `/rewind` or `/rollback` rewinds conversation to a prior turn.
- `/restore <ID>` reverts project files to a checkpoint before a tool call.
- `/export html|md|json|jsonl` exports session history, but export is not itself a resumable state file.
- `POST /sessions/archive` moves active JSONL into `chats/archive/`; `POST /sessions/unarchive` moves it back so clients can load/resume again.

The original session is preserved by branching/forking. Rewind changes the active path within the current session, while abandoned descendants remain in the JSONL as non-active branches.

## Human-in-the-Loop Resume

For the standalone CLI, human-in-the-loop decisions through hooks are synchronous. A hook can receive permission/tool context and return a decision, but there is no durable "ask the user elsewhere, terminate, then resume with the answer" loop comparable to a provider-level deferred tool call. If the CLI exits, the pending prompt/approval is gone.

For daemon sessions, human-in-the-loop mediation is stronger while the process is alive. The daemon emits permission requests with request IDs and session identity, and clients can answer through `POST /permission/:requestId` or the ACP session permission method. Client IDs allow permission policy attribution. This is suitable for Claudine proxying a live question to a user and injecting the answer, provided Claudine keeps the daemon session alive.

The limitation is durability: the daemon lifecycle docs state that pending permissions are resolved as cancelled on termination. JSONL replay cannot resurrect an unanswered permission request after a crash or explicit close.

## Interruption Recovery

Completed transcript records survive terminal close, Ctrl+C, crash, process kill, provider errors after a record flush, and network loss to the extent they were already written. The observed transcript files are appended continuously during the run, not written only at final exit.

Daemon recovery has a separate live path. `qwen/control/session/continue` classifies interrupted prompts or dangling tool calls and can continue the last unfinished turn without adding a synthetic user prompt. `POST /session/:id/cancel` cancels the active prompt; deleting a live session cancels active prompts and pending permissions, emits `session_closed`, and leaves on-disk JSONL available for later load.

Concurrent daemon load/resume of the same session is guarded by `pendingRestoreIds`; calls coalesce or return `RestoreInProgressError` with HTTP 409 and retry guidance. Concurrent standalone CLI resume of the same JSONL is not documented as safe; Claudine should avoid it or serialize by session ID.

## Observability

Useful observability surfaces:

- `qwen sessions list --json` for current-project persisted sessions.
- JSONL transcripts for forensic inspection: record type, cwd, branch, model, tool result, timestamps, usage metadata.
- Debug logs under `~/.qwen/debug/<session-id>.txt` when debug mode is enabled.
- `qwen serve` lifecycle routes: `/daemon/status`, `/capabilities`, `/workspace/:id/sessions`, `/session/:id/context`, `/session/:id/tasks`, `/session/:id/lsp`, `/session/:id/events`.
- Daemon SSE events: `session_update`, `permission_request`, `permission_resolved`, `session_metadata_updated`, `session_closed`, `session_died`, `client_evicted`, and `stream_error`.
- Hook events: `SessionStart`, `SessionEnd`, `PreToolUse`, `PermissionRequest`, and `UserPromptSubmit`.

The daemon protocol also exposes error shapes useful for wrapper logic: unknown session returns `SessionNotFoundError`, workspace mismatch returns `WorkspaceMismatchError`, concurrent restore can return `RestoreInProgressError`, and prompt calls can return `cancelled`, `error`, `max_tokens`, or `length` stop reasons.

## Quirks and Gaps

Quirks:

- This host's installed Homebrew `qwen-code` is `0.15.6`, while npm `latest` is `0.19.6`. The installed binary does not support `qwen sessions list --json`, although current docs and source do.
- The sandboxed session `HOME` was `/Users/ken/.claudine`, so `~/.qwen` during tool calls pointed at `/Users/ken/.claudine/.qwen`; the real user's observed transcripts are under `/Users/ken/.qwen`.
- A session UUID is not globally enough; Qwen filters by current project/cwd hash.
- The transcript file is parent-linked; a naive "last N lines" replay may include dead branches or metadata records that Qwen itself would not treat as active history.
- Archived sessions are storage-only until unarchived.
- Daemon `load` and `resume` are not equivalent: `load` replays history to clients; `resume` restores without replay.
- Daemon prompt routes have no server-side timeout in Stage 1; wrappers need client-side timeouts.

Gaps:

- Exact behavior for two standalone CLI processes concurrently resuming and appending to the same JSONL is undocumented.
- The exact boundary between restored and recalculated model/configuration is underdocumented and partly source-inferred.
- MCP runtime connection and OAuth state restoration across CLI resume was not verified.
- Windows storage paths were not probed on a Windows host.
- Retention and cleanup policy for active transcripts is unknown.

## Claudine Integration Notes

Claudine can treat Qwen as a first-class resume provider for standalone headless sessions:

- Capture session IDs from structured output where possible.
- Fall back to `qwen sessions list --json` scoped to the launch cwd for session lookup on current Qwen versions.
- Use `qwen --continue -p "<prompt>"` for continue-latest and `qwen --resume <id> -p "<prompt>"` for explicit-handle resume.
- Use `--fork-session` when lifecycle policy wants a branch that preserves the original session.
- Serialize resume attempts by session ID to avoid undocumented concurrent JSONL appends.
- Preserve the original launch cwd/worktree when resuming, because lookup is project scoped.

For `retry`, Claudine should prefer a new prompt into the same transcript only when provider semantics match the lifecycle intent. For `proxy` and future human-in-the-loop recovery, `qwen serve` is the stronger substrate because permission questions and answers are live API events. That support is not durable across daemon death, so Claudine must keep the daemon/session alive while routing the user question.

## Changelog

- 2026-07-03: Refreshed against current official docs, npm `@qwen-code/qwen-code` `0.19.6`, installed Homebrew `qwen-code` `0.15.6`, and observed local transcripts under `/Users/ken/.qwen`.
- 2026-07-03: Changed the continuity model to mixed to account for both standalone transcript replay and daemon live ACP session restoration.
- 2026-07-03: Added documented `qwen sessions list --json`, `--fork-session`, archive/unarchive/delete, daemon load/resume/prompt, and daemon permission mediation.
- 2026-07-03: Corrected state storage into separate macOS, Linux, and Windows records and noted archived transcript paths.
- 2026-07-03: Recorded installed-version negative probes: `qwen sessions` is unavailable in local 0.15.6 even though current docs describe it.

## Sources

- [Qwen Code common workflows: resume previous conversations](https://qwenlm.github.io/qwen-code-docs/en/users/common-workflow/)
- [Qwen Code headless mode: resume previous sessions](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/)
- [Qwen Code commands: session management and slash commands](https://qwenlm.github.io/qwen-code-docs/en/users/features/commands/)
- [Qwen Code settings: `general.chatRecording` and resume-related UI settings](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/)
- [Qwen Code qwen serve HTTP protocol reference](https://qwenlm.github.io/qwen-code-docs/en/developers/qwen-serve-protocol/)
- [Qwen Code daemon session lifecycle and identity](https://qwenlm.github.io/qwen-code-docs/en/developers/daemon/08-session-lifecycle/)
- [Qwen Code worktrees documentation](https://qwenlm.github.io/qwen-code-docs/en/users/features/worktree/)
- Local inspection: installed `/opt/homebrew/bin/qwen` version `0.15.6`, npm package `@qwen-code/qwen-code@0.19.6`, source bundle under `/tmp/qwen-code-research/package`, and real local transcripts under `/Users/ken/.qwen/projects/*/chats/*.jsonl`.
