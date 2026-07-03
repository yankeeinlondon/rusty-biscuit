---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: opencode
model: kimi-for-coding/k2p7
docs: https://pi.dev/docs/latest/sessions
support: first_class
continuity_model: transcript_replay
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "pi -c / --continue"
      - "pi -r / --resume"
      - "pi --session <path|id>"
      - "/resume slash command"
      - "/tree in-place tree navigation"
      - "/import <file>"
    accepts_followup_prompt: false
    selection_methods:
      - latest
      - id
      - picker
      - other
    notes: "Follow-up prompts are typed inside the resumed TUI session. `other` covers selection by file path or partial session UUID."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "pi -p --continue [message]"
      - "pi -p --session <path|id> [message]"
      - "pi --mode json --session <path|id> [message]"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
      - other
    notes: "Print and JSON modes append the supplied prompt to the selected JSONL session and return the response. Resuming a session whose cwd differs from the current directory can prompt to fork interactively."
  - mode: headless_server
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "No standalone headless server. Programmatic control uses an in-process SDK or a spawned RPC subprocess."
  - mode: ide
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "No documented IDE extension resume surface."
  - mode: api
    supported: true
    mechanisms:
      - "pi --mode rpc --session <path|id>"
      - "RPC switch_session command"
      - "SDK SessionManager.open(path) / SessionManager.continueRecent(cwd)"
      - "SDK AgentSessionRuntime.switchSession() / fork()"
    accepts_followup_prompt: true
    selection_methods:
      - id
      - other
    notes: "`other` covers selection by absolute file path. The RPC process loads the transcript and appends to it while alive; it is not a remote server."
session_id_capture:
  - surface: json_stream
    field: id
    format: uuid
    notes: "First line of `pi --mode json` output is the session header. Subsequent events do not repeat the session ID."
  - surface: session_file
    field: filename
    format: "<timestamp>_<uuid>.jsonl"
    notes: "Session UUID also appears as the `id` field in the JSONL header."
  - surface: interactive_ui
    field: sessionId
    format: uuid
    notes: "/session shows the session ID and file path. The /resume picker shows display names/timestamps, not raw IDs."
  - surface: other
    field: sessionId
    format: uuid
    notes: "SDK `session.sessionId`, RPC `get_state` response, and `get_session_stats` response expose the UUID."
resume_invocations:
  - mode: interactive
    invocation: "pi -c"
    accepts_prompt: false
    selection: latest
    notes: "Continue the most recent session for the current working directory."
  - mode: interactive
    invocation: "pi -r"
    accepts_prompt: false
    selection: picker
    notes: "Open the interactive session picker for the current project/directory."
  - mode: interactive
    invocation: "pi --session <path|id>"
    accepts_prompt: false
    selection: id
    notes: "Open the TUI on the specified session file or partial UUID."
  - mode: interactive
    invocation: "/resume"
    accepts_prompt: false
    selection: picker
    notes: "Inside the TUI, browse and select previous sessions for the current directory."
  - mode: interactive
    invocation: "/tree"
    accepts_prompt: false
    selection: picker
    notes: "Navigate the in-session tree and continue from any previous entry without creating a new file."
  - mode: interactive
    invocation: "/import <file>"
    accepts_prompt: false
    selection: other
    notes: "Load a JSONL session file as a new session. `other` represents file-path selection."
  - mode: non_interactive
    invocation: "pi -p --continue [message]"
    accepts_prompt: true
    selection: latest
    notes: "Send a follow-up prompt into the most recent session for the current directory and print the response."
  - mode: non_interactive
    invocation: "pi -p --session <path|id> [message]"
    accepts_prompt: true
    selection: id
    notes: "Send a follow-up prompt into a specific session and print the response."
  - mode: non_interactive
    invocation: "pi --mode json --session <path|id> [message]"
    accepts_prompt: true
    selection: id
    notes: "Same as print mode but emits structured JSONL events to stdout."
  - mode: api
    invocation: "pi --mode rpc --session <path|id>"
    accepts_prompt: true
    selection: id
    notes: "Launch an RPC subprocess already loaded into the specified session, then send `prompt` commands."
  - mode: api
    invocation: '{"type":"switch_session","sessionPath":"/path/to/session.jsonl"}'
    accepts_prompt: false
    selection: other
    notes: "Change the active session in a running RPC process."
  - mode: api
    invocation: "SessionManager.open('/path/to/session.jsonl')"
    accepts_prompt: false
    selection: other
    notes: "Open an existing session file via the SDK for in-process use."
state_storage:
  - location: local
    os: all
    path: "~/.pi/agent/sessions/--<cwd>--/<timestamp>_<uuid>.jsonl"
    format: JSONL
    retention: "Manual/until deleted. No documented automatic retention or cleanup policy."
    notes: "`<cwd>` is the absolute working directory with `/` replaced by `-`. The format is versioned (v3) and documented. Overridden by `--session-dir`, `PI_CODING_AGENT_SESSION_DIR`, or `settings.json sessionDir`."
resume_scope:
  project_scoped: true
  cwd_scoped: true
  worktree_aware: false
  all_projects_supported: false
  branch_filtering: false
  notes: "Sessions are stored per working directory. The interactive picker and `SessionManager.list(cwd)` scope to the current directory. SDK `SessionManager.listAll()` can enumerate all sessions but is not a resume UI."
branching_checkpointing:
  branch_supported: true
  checkpoint_supported: false
  fork_invocation: "CLI: pi --fork <path|id>; TUI: /fork; SDK: SessionManager.forkFrom() / createBranchedSession(); RPC: fork"
  checkpoint_invocation: ""
  preserves_original: true
  notes: "/tree navigates the existing tree in place and leaves all branches in the same file. /fork and /clone create new session files, leaving the source intact. There is no rewind/checkpoint command."
restored_state:
  transcript: true
  tool_results: true
  approvals: unknown
  model: overridable
  cwd: restored
  env: current_process
  notes: "Transcript and tool results are replayed from the JSONL file. The original cwd is stored in the session header; resuming from a different cwd can prompt to fork. Model and thinking level can be overridden at resume time via CLI flags or commands. Pi has no built-in permission/approval system, so there is no approval state to restore; extensions may add their own."
hitl_resume:
  supported: false
  question_capture: ""
  answer_injection: ""
  limitations: "Pi deliberately omits built-in permission popups and question tools. Extensions can request user input via the RPC `extension_ui_request` sub-protocol, but pending requests are owned by the live process and are not persisted in the session file for later answer injection."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: true
  pending_approval_resume: false
  limitations: "Sessions are append-only JSONL files, so crash, Ctrl+C, terminal close, and process kill generally leave the session resumable. A kill mid-turn may leave a partial assistant message as the current leaf. There is no built-in approval state to recover."
observability:
  stream_events:
    - "session header (id, cwd, version) in --mode json"
    - "agent_start / agent_end"
    - "turn_start / turn_end"
    - "message_start / message_update / message_end"
    - "tool_execution_start / tool_execution_update / tool_execution_end"
  hook_events: []
  failure_modes:
    - "auto_retry_start / auto_retry_end on transient provider errors"
  notes: "The JSONL session file itself is the authoritative resume state. SDK/RPC state queries expose sessionId, sessionFile, and message counts."
quirks:
  - "Resuming a session from a different working directory in print/JSON mode can block on an interactive 'Fork this session into current directory?' prompt."
  - "`--session` and `--fork` accept either a full file path or a partial session UUID; partial UUIDs may collide."
  - "`/tree` edits the active leaf in the same file, so branching does not create a new session unless you explicitly /fork or /clone."
  - "`/import` loads a JSONL file as a new session; it does not overwrite the source file."
  - "`--no-session` runs ephemerally and produces no resumable file."
  - "The session directory name encodes the absolute path with `-` separators, which can make cross-machine paths non-portable."
gaps:
  - "Whether print/JSON mode `--session` accepts a follow-up prompt when the cwd matches is inferred from CLI behavior, not explicitly documented."
  - "Exact concurrency semantics when two processes open the same `.jsonl` file simultaneously are not documented."
  - "Whether CLI resume flags (`--model`, `--provider`, `--thinking`) fully override session-restored model state is not explicitly documented."
  - "Windows session storage paths are inferred from documentation; not locally verified."
  - "Whether any extension-driven permission/question state is persisted across process restarts is not documented."
changes: []
requires_claudine_update: true
reason: "Pi is on the Claudine research roster but is not yet a code-supported provider. The verified resume semantics, session-id surfaces, and cross-cwd fork prompt should feed provider metadata and wrapper design."
---

## Overview

Pi's session resume is a first-class, local-transcript-replay system. Every session is a versioned JSONL file stored under `~/.pi/agent/sessions/`, organized by working directory. Resume means loading that file, rebuilding the conversation tree, and appending new turns. Pi supports interactive resumption (`-c`, `-r`, `/resume`, `/tree`), scriptable non-interactive follow-up (`pi -p --session <file|uuid> <prompt>`), and programmatic control through the SDK or the RPC subprocess protocol.

## Resume Semantics

A Pi "session" is a persisted JSONL transcript on disk. The authoritative state is the file itself; there is no separate server process or remote session store. The file uses a tree structure (`id`/`parentId`) so `/tree` can branch in place without creating new files. Resuming a session loads the file, restores the active leaf, and continues appending. Forking (`/fork`, `--fork`, `/clone`) copies history into a new file while leaving the original intact.

## Supported Modes

| Mode | Entry point | Follow-up prompt | Selector |
|------|-------------|------------------|----------|
| Interactive CLI | `pi -c` | No | Latest in current directory |
| Interactive CLI | `pi -r` | No | Picker |
| Interactive CLI | `pi --session <path\|id>` | No | File path or partial UUID |
| Interactive CLI | `/resume` | No | Picker |
| Interactive CLI | `/tree` | No | Tree picker |
| Interactive CLI | `/import <file>` | No | File path |
| Non-interactive CLI | `pi -p --continue [message]` | Yes | Latest in current directory |
| Non-interactive CLI | `pi -p --session <path\|id> [message]` | Yes | File path or partial UUID |
| Non-interactive CLI | `pi --mode json --session <path\|id> [message]` | Yes | File path or partial UUID |
| RPC / SDK | `pi --mode rpc --session <path\|id>` then `prompt` | Yes | File path or UUID |
| RPC / SDK | `switch_session`, `SessionManager.open()` | No | File path |

## Session ID Capture

Stable session identifiers are UUIDs. They can be captured from:

- **`pi --mode json`**: the first line is a `session` header containing `id`.
- **The session file path**: `~/.pi/agent/sessions/--<cwd>--/<timestamp>_<uuid>.jsonl`.
- **The session file header**: line 1 has `"type":"session","id":"<uuid>"`.
- **`/session`**: shows the session ID and file path in the TUI.
- **SDK/RPC**: `session.sessionId`, `get_state.sessionId`, and `get_session_stats.sessionId`.

The interactive `/resume` picker shows display names and timestamps, not raw IDs.

## Resume Invocation

Continue the latest session interactively:

```bash
pi -c
```

Browse and resume a session:

```bash
pi -r
```

Resume a specific session in the TUI:

```bash
pi --session 019f2352-39b4-73fa-9065-78dfc099ac1e
```

Send a follow-up non-interactively:

```bash
pi -p --session ~/.pi/agent/sessions/--Users-ken-Projects-foo--/2026-07-02T14-53-39-124Z_019f2352-39b4-73fa-9065-78dfc099ac1e.jsonl "next step"
```

Continue the latest session with a new prompt:

```bash
pi -p --continue "finish the task"
```

Resume via RPC:

```bash
pi --mode rpc --session /path/to/session.jsonl
```

Then send a prompt over stdin:

```json
{"type":"prompt","message":"what is the next step?"}
```

## Session Lookup Scope

Sessions are scoped by the working directory used when they were created. The storage directory encodes the absolute path with `/` replaced by `-`. The interactive picker and `SessionManager.list(cwd)` default to the current directory. There is no documented git-branch, worktree, or cross-project filtering in the CLI picker, although the SDK exposes `SessionManager.listAll()`.

## State Storage

Resumable state is local and file-based.

| OS | Path |
|----|------|
| macOS / Linux | `~/.pi/agent/sessions/--<cwd>--/<timestamp>_<uuid>.jsonl` |
| Windows (inferred) | `%USERPROFILE%\.pi\agent\sessions\--<cwd>--\<timestamp>_<uuid>.jsonl` |

The JSONL format is versioned and documented in [Session File Format](https://pi.dev/docs/latest/session-format). Direct parsing is supported, but the SDK's `SessionManager` is the most stable interface. Storage location can be overridden with `--session-dir`, `PI_CODING_AGENT_SESSION_DIR`, or the `sessionDir` setting. There is no documented automatic retention; sessions persist until manually deleted.

## Restored State

Resuming restores:

- The full conversation transcript and tool results from the JSONL file.
- The original working directory from the session header. Resuming from a different cwd can trigger a fork prompt.
- The current model and thinking level recorded in the file, overridable at resume time.

Resuming does not restore the environment of the original session; it uses the launching process environment. Pi has no built-in permission/approval system, so there is no approval state to restore.

## Branching and Checkpoints

Branching is a core feature:

- `/tree` navigates the session tree in place. All branches remain in the same file.
- `/fork` creates a new session file from a previous user message.
- `/clone` duplicates the current active branch into a new session file.
- `--fork <path|id>` forks from the CLI.
- SDK: `SessionManager.forkFrom()`, `createBranchedSession()`, `runtime.fork()`.
- RPC: `fork`, `clone`, `get_fork_messages`.

There is no `/rewind` or checkpoint command. Because `/tree` keeps history in one file, the original branch is not destroyed, but the active leaf moves.

## Human-in-the-Loop Resume

Pi does not support native human-in-the-loop resume. It deliberately omits built-in permission popups and question tools. Extensions can build confirmation flows or use the RPC `extension_ui_request` / `extension_ui_response` sub-protocol, but any pending request is tied to the live process and is not persisted in the session file for later injection.

## Interruption Recovery

Because sessions are append-only JSONL files, most interruptions leave the session resumable:

- **Crash / terminal close / process kill**: the file remains and can be resumed.
- **Ctrl+C**: the session is preserved; resume with `-c`, `-r`, or `--session`.
- **Pending tool call**: tool results are written to the file; a kill mid-turn may leave a partial assistant message as the leaf.
- **Pending approval / question**: not applicable; no built-in approval state is persisted.

## Observability

Surfaces that expose session identity or resumability:

- **`--mode json`**: first event is the `session` header with `id` and `cwd`.
- **Session file path/header**: stable UUID and encoded cwd.
- **RPC `get_state` / `get_session_stats`**: return `sessionId`, `sessionFile`, `messageCount`, etc.
- **SDK `session.sessionId` and `session.sessionFile`**.
- **Lifecycle events**: `agent_start`, `turn_start`, `message_start`, `tool_execution_*`, etc., stream progress but do not repeat the session ID.

## Quirks and Gaps

Quirks:

- Resuming a session from a different working directory in print/JSON mode can block on an interactive fork prompt.
- `--session` and `--fork` accept a partial UUID, which can collide.
- `/tree` changes the active leaf in the same file; it does not create a checkpoint.
- The storage directory name encodes the absolute path, so paths are not portable across machines.

Gaps:

- Whether print/JSON `--session` accepts a positional follow-up prompt when cwd matches is inferred from observed behavior, not an explicit docs statement.
- Concurrency semantics when two processes open the same `.jsonl` file are not documented.
- Whether resume-time CLI flags fully override session-restored model state is not explicitly documented.
- Windows storage paths are inferred, not locally verified.

## Claudine Integration Notes

For Claudine's lifecycle `resume` action and future recovery:

- Capture the session UUID from the first line of `pi --mode json` output or from the session filename.
- Use `pi -p --session <file|uuid> "<follow-up>"` for scriptable continuation from a matching cwd.
- Use `pi -p --continue "<follow-up>"` to target the latest session in the current directory.
- Handle the cross-cwd fork prompt when resuming `--session` from a different directory; prefer launching from the session's original cwd or passing the absolute file path and confirming behavior.
- For richer programmatic control, spawn `pi --mode rpc --session <file>` and drive it via JSONL commands.
- Treat the JSONL file as the source of truth; direct parsing is documented, but prefer SDK `SessionManager` when in-process.
- Do not rely on native HITL resume; build or require an extension if user questions must be brokered.

## Sources

- [Pi Sessions documentation](https://pi.dev/docs/latest/sessions)
- [Pi Session File Format](https://pi.dev/docs/latest/session-format)
- [Pi JSON Event Stream Mode](https://pi.dev/docs/latest/json)
- [Pi RPC Mode](https://pi.dev/docs/latest/rpc)
- [Pi SDK documentation](https://pi.dev/docs/latest/sdk)
- [Pi Settings documentation](https://pi.dev/docs/latest/settings)
- [Pi GitHub README / CLI reference](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/README.md)
- [Pi project website](https://pi.dev)
