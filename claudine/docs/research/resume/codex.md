---
$schema: ./_schema.yaml
created: 2026-04-02
last_updated: 2026-07-03
agent: codex
model: default
docs: https://developers.openai.com/codex/cli/features
support: first_class
continuity_model: mixed
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "codex resume [SESSION_ID|SESSION_NAME] [PROMPT]"
      - "codex resume --last [PROMPT]"
      - "codex resume --all --last [PROMPT]"
      - "codex resume --include-non-interactive"
      - "/resume slash command"
      - "interactive saved-session picker"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
      - name
      - picker
      - all_projects
    notes: "Interactive resume loads a saved local thread into the TUI. --last is cwd-scoped unless --all is passed. --include-non-interactive makes exec sessions eligible for the picker and --last selection."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "codex exec resume [SESSION_ID|THREAD_NAME] [PROMPT]"
      - "codex exec resume --last [PROMPT]"
      - "codex exec resume --all --last [PROMPT]"
      - "codex exec resume --json"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
      - name
      - all_projects
    notes: "Scriptable follow-up uses codex exec resume. The prompt can be a positional string or '-' to read stdin. Sessions created with --ephemeral do not persist rollout files and therefore are not resumable later."
  - mode: headless_server
    supported: true
    mechanisms:
      - "codex app-server --listen stdio://"
      - "codex app-server --listen ws://IP:PORT"
      - "codex resume --remote <ADDR> [SESSION_ID]"
      - "thread/resume JSON-RPC"
    accepts_followup_prompt: true
    selection_methods:
      - id
      - picker
      - latest
    notes: "The app-server is a local/headless JSON-RPC server. thread/resume loads a thread, and turn/start or turn/steer supplies follow-up input. The CLI remote TUI path is interactive; programmatic use should speak app-server JSON-RPC directly."
  - mode: ide
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "The app-server powers rich clients such as the VS Code extension, but IDE-specific resume behavior is outside this CLI-focused research."
  - mode: api
    supported: true
    mechanisms:
      - "app-server thread/list"
      - "app-server thread/read"
      - "app-server thread/resume"
      - "app-server thread/fork"
      - "app-server thread/rollback"
      - "app-server turn/start"
      - "app-server turn/steer"
    accepts_followup_prompt: true
    selection_methods:
      - id
      - all_projects
    notes: "There is no public hosted HTTP resume API for Codex CLI sessions. The local app-server JSON-RPC API is documented and can list, read, resume, fork, archive, delete, and roll back local threads."
session_id_capture:
  - surface: json_stream
    field: thread_id
    format: uuid
    notes: "codex exec --json emits thread.started with thread_id. The non-interactive documentation shows this as the session/thread identifier."
  - surface: session_file
    field: payload.session_id
    format: uuid
    notes: "Local rollout JSONL starts with a session_meta record whose payload contains session_id and id. Local inspection of 0.142.5 showed session_id nested under payload, not top-level."
  - surface: session_file
    field: filename
    format: "rollout-<timestamp>-<uuid>.jsonl"
    notes: "The rollout filename embeds the same UUID as the thread id."
  - surface: log_file
    field: session_id
    format: uuid
    notes: "~/.codex/history.jsonl records one row per saved prompt with session_id and ts."
  - surface: log_file
    field: id
    format: uuid
    notes: "~/.codex/session_index.jsonl stores id, thread_name, and updated_at for indexed sessions."
  - surface: log_file
    field: threads.id
    format: uuid
    notes: "~/.codex/state_5.sqlite stores thread metadata in the threads table, including id, rollout_path, cwd, source, archived, git_branch, model, sandbox_policy, and approval_mode."
  - surface: hook
    field: session_id
    format: uuid
    notes: "Codex hooks receive session_id, transcript_path, cwd, hook_event_name, and model on stdin."
  - surface: interactive_ui
    field: "picker / /status"
    format: uuid
    notes: "Official features docs say the session ID can be copied from the picker, /status, or files under ~/.codex/sessions."
  - surface: other
    field: "thread.id / thread.sessionId"
    format: uuid
    notes: "app-server thread/start, thread/resume, thread/read, and thread/list expose thread ids in JSON-RPC results."
resume_invocations:
  - mode: interactive
    invocation: "codex resume"
    accepts_prompt: false
    selection: picker
    notes: "Opens the interactive saved-session picker for recent interactive sessions."
  - mode: interactive
    invocation: "codex resume --all"
    accepts_prompt: false
    selection: picker
    notes: "Opens the picker across directories by disabling cwd filtering."
  - mode: interactive
    invocation: "codex resume --last [PROMPT]"
    accepts_prompt: true
    selection: latest
    notes: "Resumes the most recent recorded session for the current working directory."
  - mode: interactive
    invocation: "codex resume --all --last [PROMPT]"
    accepts_prompt: true
    selection: all_projects
    notes: "Resumes the most recent recorded session across directories."
  - mode: interactive
    invocation: "codex resume --include-non-interactive --last [PROMPT]"
    accepts_prompt: true
    selection: latest
    notes: "Allows the interactive resume path to select a saved exec session."
  - mode: interactive
    invocation: "codex resume <SESSION_ID|SESSION_NAME> [PROMPT]"
    accepts_prompt: true
    selection: id
    notes: "Current CLI help accepts a UUID or session name; UUIDs take precedence if the argument parses as a UUID."
  - mode: interactive
    invocation: "/resume"
    accepts_prompt: false
    selection: picker
    notes: "Inside the TUI, opens the saved-session picker and reloads the selected transcript."
  - mode: non_interactive
    invocation: "codex exec resume --last [PROMPT]"
    accepts_prompt: true
    selection: latest
    notes: "Scriptable follow-up into the latest saved exec session for the current working directory."
  - mode: non_interactive
    invocation: "codex exec resume --all --last [PROMPT]"
    accepts_prompt: true
    selection: all_projects
    notes: "Scriptable follow-up into the latest saved exec session across directories."
  - mode: non_interactive
    invocation: "codex exec resume <SESSION_ID|THREAD_NAME> [PROMPT]"
    accepts_prompt: true
    selection: id
    notes: "Scriptable follow-up into an exact session. Use --json for JSONL events and - to read the follow-up prompt from stdin."
  - mode: headless_server
    invocation: "codex resume --remote <ws://host:port|wss://host:port|unix://PATH> [SESSION_ID]"
    accepts_prompt: false
    selection: id
    notes: "Connects the TUI to an app-server endpoint and resumes through the remote TUI."
  - mode: api
    invocation: "{\"method\":\"thread/resume\",\"id\":11,\"params\":{\"threadId\":\"<SESSION_ID>\"}}"
    accepts_prompt: false
    selection: id
    notes: "Loads an existing thread in app-server; follow with turn/start to append a new prompt."
  - mode: api
    invocation: "{\"method\":\"turn/start\",\"id\":12,\"params\":{\"threadId\":\"<SESSION_ID>\",\"input\":[{\"type\":\"text\",\"text\":\"next step\"}]}}"
    accepts_prompt: true
    selection: id
    notes: "Starts the follow-up turn after app-server thread/resume."
  - mode: api
    invocation: "{\"method\":\"turn/steer\",\"id\":13,\"params\":{\"threadId\":\"<SESSION_ID>\",\"expectedTurnId\":\"<TURN_ID>\",\"input\":[{\"type\":\"text\",\"text\":\"correction\"}]}}"
    accepts_prompt: true
    selection: id
    notes: "Adds user input to an active in-flight app-server turn rather than creating a new turn."
state_storage:
  - location: local
    os: macos
    path: "~/.codex/sessions/<YYYY>/<MM>/<DD>/rollout-<timestamp>-<session-id>.jsonl"
    format: JSONL
    retention: "Saved unless history.persistence is none or an exec run uses --ephemeral; no documented rollout cleanup sweep."
    notes: "Observed on this macOS host. The path may be redirected if CODEX_HOME is changed. This host also had ~/.claudine/.codex symlinks to ~/.codex."
  - location: local
    os: linux
    path: "~/.codex/sessions/<YYYY>/<MM>/<DD>/rollout-<timestamp>-<session-id>.jsonl"
    format: JSONL
    retention: "Saved unless history.persistence is none or an exec run uses --ephemeral; no documented rollout cleanup sweep."
    notes: "Same default Unix-style CODEX_HOME layout as macOS."
  - location: local
    os: windows
    path: "%USERPROFILE%\\.codex\\sessions\\<YYYY>\\<MM>\\<DD>\\rollout-<timestamp>-<session-id>.jsonl"
    format: JSONL
    retention: "Saved unless history.persistence is none or an exec run uses --ephemeral; no documented rollout cleanup sweep."
    notes: "Windows uses the user profile .codex directory and backslash paths."
  - location: local
    os: macos
    path: "~/.codex/state_5.sqlite"
    format: SQLite
    retention: "Not documented."
    notes: "Observed table threads with id, rollout_path, cwd, source, model_provider, title, sandbox_policy, approval_mode, archived, git_sha, git_branch, model, reasoning_effort, preview, and recency fields."
  - location: local
    os: linux
    path: "~/.codex/state_5.sqlite"
    format: SQLite
    retention: "Not documented."
    notes: "Expected same CODEX_HOME-relative state DB as macOS."
  - location: local
    os: windows
    path: "%USERPROFILE%\\.codex\\state_5.sqlite"
    format: SQLite
    retention: "Not documented."
    notes: "Expected same CODEX_HOME-relative state DB using Windows home path."
  - location: local
    os: macos
    path: "~/.codex/history.jsonl"
    format: JSONL
    retention: "history.max_bytes caps this file by dropping oldest entries when set; history.persistence controls saving to history.jsonl."
    notes: "Observed rows contain session_id, ts, and text."
  - location: local
    os: linux
    path: "~/.codex/history.jsonl"
    format: JSONL
    retention: "history.max_bytes caps this file by dropping oldest entries when set; history.persistence controls saving to history.jsonl."
    notes: "Same CODEX_HOME-relative layout as macOS."
  - location: local
    os: windows
    path: "%USERPROFILE%\\.codex\\history.jsonl"
    format: JSONL
    retention: "history.max_bytes caps this file by dropping oldest entries when set; history.persistence controls saving to history.jsonl."
    notes: "Windows path differs even though the file format is the same."
  - location: local
    os: macos
    path: "~/.codex/session_index.jsonl"
    format: JSONL
    retention: "Not documented."
    notes: "Observed rows contain id, thread_name, and updated_at. It may be stale relative to state_5.sqlite and rollout files."
  - location: local
    os: linux
    path: "~/.codex/session_index.jsonl"
    format: JSONL
    retention: "Not documented."
    notes: "Same CODEX_HOME-relative layout as macOS."
  - location: local
    os: windows
    path: "%USERPROFILE%\\.codex\\session_index.jsonl"
    format: JSONL
    retention: "Not documented."
    notes: "Windows path differs even though the file format is the same."
  - location: local
    os: macos
    path: "~/.codex/archived_sessions/"
    format: JSONL
    retention: "Until unarchived or deleted."
    notes: "Observed archived rollout file on this host. app-server docs say archiving moves persisted thread logs here."
  - location: local
    os: linux
    path: "~/.codex/archived_sessions/"
    format: JSONL
    retention: "Until unarchived or deleted."
    notes: "Same CODEX_HOME-relative layout as macOS."
  - location: local
    os: windows
    path: "%USERPROFILE%\\.codex\\archived_sessions\\"
    format: JSONL
    retention: "Until unarchived or deleted."
    notes: "Windows path differs even though the file format is the same."
resume_scope:
  project_scoped: true
  cwd_scoped: true
  worktree_aware: true
  all_projects_supported: true
  branch_filtering: false
  notes: "CLI --last and picker lookup are cwd-scoped by default; --all disables cwd filtering. state_5.sqlite stores git_branch and git_sha, and separate worktree paths naturally form separate cwd scopes, but the CLI does not expose a branch filter. app-server thread/list can filter by exact cwd and search term."
branching_checkpointing:
  branch_supported: true
  checkpoint_supported: true
  fork_invocation: "codex fork [SESSION_ID] [PROMPT]; /fork; app-server thread/fork"
  checkpoint_invocation: "app-server thread/rollback; /compact; app-server thread/compact/start"
  preserves_original: true
  notes: "Fork creates a fresh thread id from stored history and preserves the original. app-server thread/rollback removes the last N turns from in-memory context and records a rollback marker in the persisted log; it is rollback, not a named checkpoint. /compact and thread/compact/start summarize history to reduce context and preserve continuity, but they are not reversible checkpoints."
restored_state:
  transcript: true
  tool_results: true
  approvals: preserved
  model: overridable
  cwd: configurable
  env: current_process
  notes: "Official docs say resumed runs keep transcript, plan history, and approvals. Local turn_context records cwd, workspace_roots, approval_policy, sandbox_policy, permission_profile, model, effort, current_date, timezone, personality, collaboration_mode, and summary. Resume-time flags or app-server turn/start fields can override model, cwd, sandbox, approval policy, and extra roots. The launching process supplies the environment."
hitl_resume:
  supported: true
  question_capture: "For codex exec, capture assistant questions from JSONL item.completed agent_message events or from the final message. Hooks expose UserPromptSubmit, PermissionRequest, PostToolUse, SubagentStop, and Stop inputs. app-server can surface server-initiated approval requests and item/tool/requestUserInput requests."
  answer_injection: "For CLI automation, run codex exec resume <session_id> '<user answer>' so the answer becomes the next user turn. For app-server, call thread/resume if needed and then turn/start, or use turn/steer for an active turn. Stop hooks can continue a completed turn by returning decision: block with a reason that becomes a continuation prompt."
  limitations: "CLI exec does not provide a native defer token or structured answer-injection API. PermissionRequest can allow or deny approvals, and reserved interrupt fields fail closed today. Robust HITL in the middle of an active turn is better matched to app-server than to codex exec."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: true
  pending_approval_resume: true
  limitations: "Transcript records are written continuously, and Ctrl+C, terminal close, crash, or process kill leave saved rollout state when persistence is enabled. A pending tool or approval is represented in transcript/app-server state, but CLI docs do not guarantee exact mid-tool replay semantics. app-server has turn/interrupt for in-flight turns and runtime thread status, but concurrent resume/write safety for the same session is not documented."
observability:
  stream_events:
    - "thread.started"
    - "turn.started"
    - "turn.completed"
    - "turn.failed"
    - "item.started"
    - "item.completed"
    - "error"
    - "thread/started"
    - "thread/status/changed"
    - "thread/archived"
    - "thread/unarchived"
    - "thread/deleted"
    - "item/started"
    - "item/completed"
    - "item/agentMessage/delta"
    - "serverRequest/resolved"
  hook_events:
    - "SessionStart"
    - "PreToolUse"
    - "PermissionRequest"
    - "PostToolUse"
    - "PreCompact"
    - "PostCompact"
    - "UserPromptSubmit"
    - "SubagentStart"
    - "SubagentStop"
    - "Stop"
  failure_modes:
    - "turn.failed"
    - "error"
    - "thread status systemError"
    - "app-server JSON-RPC error"
  notes: "codex exec --json is the stable stream surface for non-interactive wrappers. Hooks expose session_id and transcript_path but the transcript path is explicitly not a stable hook interface. app-server exposes richer thread status, list/read/resume APIs, turn streams, and server-initiated approval/user-input requests."
quirks:
  - "codex-cli 0.142.5 help shows codex resume [SESSION_ID] [PROMPT] and codex exec resume [SESSION_ID] [PROMPT]; both accept UUIDs or names/thread names, with UUIDs taking precedence."
  - "codex exec and codex exec resume now expose --ephemeral. Ephemeral exec runs intentionally avoid persisted rollout files and are not future-resumable."
  - "The official reference page documents codex resume SESSION_ID as uuid, while local help also accepts session names. Wrappers should treat names as convenience selectors and prefer UUID capture."
  - "Local rollout JSONL stores session_id under the session_meta payload. Older research claimed a top-level session_id field; that is not what local 0.142.5 files show."
  - "history.persistence controls saving to history.jsonl according to docs, but actual resumability depends on rollout files and state/index metadata too."
  - "session_index.jsonl can lag local rollout files and state_5.sqlite; the app-server and recent bug reports indicate index/state/rollout drift is a real failure mode."
  - "Direct rollout parsing is useful for recovery audits but unsupported as a stable integration surface."
  - "The WebSocket app-server transport is documented as experimental and unsupported; non-loopback listeners require explicit auth hardening."
  - "app-server thread/resume does not update thread.updatedAt or rollout mtime until a turn starts."
  - "app-server thread/list defaults sourceKinds to interactive sources cli and vscode; exec sessions require explicit sourceKinds if using the API for discovery."
gaps:
  - "No documented CLI command emits a machine-readable list of sessions; app-server thread/list is the documented programmatic path."
  - "No documented automatic retention policy for rollout JSONL files, state_5.sqlite threads, or archived_sessions beyond history.max_bytes for history.jsonl and explicit archive/delete."
  - "Exact behavior when two processes resume and append to the same rollout concurrently is undocumented."
  - "Exact replay behavior for a crash during a running shell command or unsatisfied approval is not guaranteed by docs."
  - "IDE extension resume behavior was not independently researched; only its app-server foundation is documented here."
  - "The local npm package ships a native binary, so source-level verification was done through official docs, CLI help, generated local artifacts, and SQLite inspection rather than readable installed source."
changes:
  - "2026-07-03: Corrected producer metadata to agent codex, model default, and refreshed against codex-cli 0.142.5."
  - "2026-07-03: Added --ephemeral as a non-interactive persistence/resume boundary."
  - "2026-07-03: Corrected rollout JSONL field location: session_id is in session_meta.payload on observed local files."
  - "2026-07-03: Added state_5.sqlite threads as a local session lookup and observability source."
  - "2026-07-03: Added app-server JSON-RPC resume, list, read, fork, rollback, turn/start, and turn/steer behavior."
  - "2026-07-03: Split state storage paths into separate macOS, Linux, and Windows records."
  - "2026-07-03: Updated HITL assessment to distinguish exec follow-up prompting from app-server active-turn steering and server-initiated requests."
requires_claudine_update: true
reason: "Claudine's Codex resume metadata should account for --ephemeral non-resumable exec runs, session_id capture from session_meta.payload and state_5.sqlite, app-server thread/resume and turn/steer support, and the lack of a stable CLI session-list command."
---

# Codex CLI Resume Semantics

## Overview

Codex CLI has first-class resume support for local terminal sessions. The practical model is local transcript replay: Codex stores session rollout logs under `~/.codex/sessions/`, indexes threads in local state, and reloads a saved thread when the user runs `codex resume`, `/resume`, or `codex exec resume`. For automation, the most important path is `codex exec resume <session-id> "<follow-up>"`, optionally with `--json` to capture machine-readable stream events.

The current CLI also exposes a local app-server JSON-RPC API with `thread/list`, `thread/read`, `thread/resume`, `thread/fork`, `thread/rollback`, `turn/start`, and `turn/steer`. That makes Codex's continuity model mixed: the regular CLI resumes from persisted local transcript state, while app-server can keep loaded threads in a local server process and accept follow-up or steering input over JSON-RPC. The main wrapper risks are selecting the wrong cwd-scoped session, assuming ephemeral exec runs are resumable, treating internal rollout JSONL as stable, and trying to automate an interactive picker rather than capturing a UUID up front.

## Resume Semantics

A Codex CLI session is a local thread with persisted transcript and metadata. Local inspection of `codex-cli 0.142.5` showed rollout JSONL files at `~/.codex/sessions/<YYYY>/<MM>/<DD>/rollout-<timestamp>-<uuid>.jsonl`. The first record is `session_meta`; its `payload.session_id` and `payload.id` match the UUID in the filename. Subsequent records include `event_msg`, `response_item`, and `turn_context`. `turn_context` records operational state such as `cwd`, `workspace_roots`, `approval_policy`, `sandbox_policy`, `permission_profile`, `model`, `effort`, `current_date`, `timezone`, `personality`, `collaboration_mode`, and `summary`.

The applicable resume patterns are continue-latest, resume by UUID or name, interactive picker, non-interactive follow-up, local transcript replay, local server/API resume, fork, rollback, compaction, and interruption recovery. A chat-history export is not resume unless Codex can load it as a thread. Memory files, rules, plugins, MCP config, and project instructions are context sources for a new or resumed turn, not prior-session continuation mechanisms by themselves.

## Supported Modes

| Mode | Entry point | Follow-up prompt | Selector | Notes |
|------|-------------|------------------|----------|-------|
| Interactive CLI | `codex resume` | No | Picker | Human picker for saved interactive sessions. |
| Interactive CLI | `codex resume --last [PROMPT]` | Yes | Latest in current cwd | Add `--all` to ignore cwd filtering. |
| Interactive CLI | `codex resume --include-non-interactive --last [PROMPT]` | Yes | Latest including exec | Useful when an exec transcript should be reopened in the TUI. |
| Interactive CLI | `codex resume <SESSION_ID\|SESSION_NAME> [PROMPT]` | Yes | UUID or name | Local help says UUIDs take precedence. |
| TUI slash command | `/resume` | No | Picker | Reloads a selected saved transcript. |
| Non-interactive CLI | `codex exec resume --last [PROMPT]` | Yes | Latest in current cwd | Scriptable continuation for automation. |
| Non-interactive CLI | `codex exec resume --all --last [PROMPT]` | Yes | Latest across directories | Disables cwd filtering for latest selection. |
| Non-interactive CLI | `codex exec resume <SESSION_ID\|THREAD_NAME> [PROMPT]` | Yes | UUID or name | Use `--json` for JSONL stream events and `-` to read the prompt from stdin. |
| Remote TUI | `codex resume --remote <ADDR> [SESSION_ID]` | No | UUID or picker | Interactive TUI over a local app-server endpoint. |
| app-server API | `thread/resume` then `turn/start` | Yes | Thread id | Programmatic local JSON-RPC path. |
| app-server API | `turn/steer` | Yes | Active thread and turn id | Adds input to an in-flight turn. |

Sessions created by normal `codex exec` are persisted and resumable. Sessions created with `codex exec --ephemeral` or `codex exec resume --ephemeral` intentionally avoid persisted session files and should be treated as not resumable after the process exits.

## Session ID Capture

Stable session identifiers are UUID-like thread IDs. Good capture surfaces are:

- `codex exec --json`: `thread.started` emits `thread_id`.
- Rollout filename: `rollout-<timestamp>-<uuid>.jsonl`.
- Rollout JSONL: `session_meta.payload.session_id` and `session_meta.payload.id`.
- Hooks: every command hook receives `session_id` and `transcript_path` on stdin.
- `~/.codex/history.jsonl`: rows contain `session_id`, `ts`, and prompt text.
- `~/.codex/session_index.jsonl`: rows contain `id`, `thread_name`, and `updated_at`.
- `~/.codex/state_5.sqlite`: table `threads` contains `id`, `rollout_path`, `cwd`, `source`, `archived`, `git_sha`, `git_branch`, `model`, `sandbox_policy`, and `approval_mode`.
- app-server: thread responses include `thread.id` and commonly `thread.sessionId`.
- Interactive UI: the official features page says the ID can be copied from the picker, `/status`, or files under `~/.codex/sessions/`.

For Claudine, the earliest scriptable handle is `thread_id` from `codex exec --json` `thread.started`. For non-JSON exec runs, the stable fallback is local state inspection, but that should be treated as recovery logic rather than the primary path.

## Resume Invocation

Continue the latest interactive session for the current cwd:

```bash
codex resume --last
codex resume --last "continue the migration"
```

Continue the latest interactive session across directories:

```bash
codex resume --all --last
codex resume --all --last "continue the migration"
```

Resume a specific session:

```bash
codex resume 019f2903-a44a-78f0-ae5a-faa950d10438
codex resume 019f2903-a44a-78f0-ae5a-faa950d10438 "implement the next step"
```

Resume a saved exec session non-interactively:

```bash
codex exec resume --last "fix the race condition"
codex exec resume --all --last "summarize what changed"
codex exec resume 019f2903-a44a-78f0-ae5a-faa950d10438 "continue"
printf '%s\n' "continue" | codex exec resume 019f2903-a44a-78f0-ae5a-faa950d10438 -
codex exec resume --json 019f2903-a44a-78f0-ae5a-faa950d10438 "continue"
```

Resume from inside the TUI:

```text
/resume
```

Resume through app-server JSON-RPC:

```json
{"method":"thread/resume","id":11,"params":{"threadId":"019f2903-a44a-78f0-ae5a-faa950d10438"}}
{"method":"turn/start","id":12,"params":{"threadId":"019f2903-a44a-78f0-ae5a-faa950d10438","input":[{"type":"text","text":"continue"}]}}
```

Steer an already active app-server turn:

```json
{"method":"turn/steer","id":13,"params":{"threadId":"019f2903-a44a-78f0-ae5a-faa950d10438","expectedTurnId":"turn_456","input":[{"type":"text","text":"focus on failing tests first"}]}}
```

## Session Lookup Scope

`codex resume --last` and `codex exec resume --last` are scoped to the current working directory by default. `--all` disables cwd filtering. The interactive picker also has `--all`, and `--include-non-interactive` includes exec sessions in the picker and `--last` selection.

Codex stores cwd in rollout metadata and in `state_5.sqlite`. The `threads` table observed locally also stores `git_branch`, `git_sha`, and `git_origin_url`, so worktrees are distinguishable by cwd and metadata. There is no documented CLI branch filter. app-server `thread/list` supports filters including `cwd`, `searchTerm`, `archived`, `modelProviders`, and `sourceKinds`; by default `sourceKinds` lists interactive `cli` and `vscode` sources, so API callers must include `exec` when they want non-interactive sessions.

## State Storage

Resumable state is local by default.

| OS | Default transcript path |
|----|-------------------------|
| macOS | `~/.codex/sessions/<YYYY>/<MM>/<DD>/rollout-<timestamp>-<uuid>.jsonl` |
| Linux | `~/.codex/sessions/<YYYY>/<MM>/<DD>/rollout-<timestamp>-<uuid>.jsonl` |
| Windows | `%USERPROFILE%\.codex\sessions\<YYYY>\<MM>\<DD>\rollout-<timestamp>-<uuid>.jsonl` |

Additional local state:

| File | Purpose |
|------|---------|
| `~/.codex/state_5.sqlite` | Thread inventory and metadata. Local schema includes the `threads` table with cwd, source, title, archive state, model, sandbox, approval, rollout path, and git metadata. |
| `~/.codex/history.jsonl` | Prompt history rows with `session_id`, `ts`, and text. |
| `~/.codex/session_index.jsonl` | Lightweight index rows with `id`, `thread_name`, and `updated_at`. |
| `~/.codex/archived_sessions/` | Rollout files moved by archive operations. |
| `~/.codex/sqlite/codex-dev.db` | Older/auxiliary SQLite state observed on this host; current thread inventory was in `state_5.sqlite`. |

The transcript format is explicitly not a stable interface for hooks, and direct parsing should be considered unsupported. Use `codex exec --json`, hooks, or app-server APIs when possible. Local inspection remains valuable for recovery and validation because actual files reveal whether a session exists, whether it is archived, and whether indexes are stale.

`history.max_bytes` caps `history.jsonl` by dropping oldest entries when set. `history.persistence` controls whether Codex saves session transcripts to `history.jsonl`. The docs do not describe an automatic cleanup policy for rollout files, archived rollout files, or `state_5.sqlite`.

## Restored State

The official features page says resumed runs keep the original transcript, plan history, and approvals. Local `turn_context` confirms that Codex records `cwd`, `workspace_roots`, `approval_policy`, `sandbox_policy`, `permission_profile`, `model`, `effort`, `personality`, `collaboration_mode`, `current_date`, `timezone`, and summary mode.

Resume-time options can override important state. CLI resume accepts global flags such as `--model`, `--cd`, `--add-dir`, `--sandbox`, and `--ask-for-approval`. app-server `turn/start` can also override model, effort, personality, cwd, sandbox policy, summary, and output schema; docs say those settings become defaults for later turns on the same thread. Environment variables are from the current launching process, not restored from the original process.

Resume appends new turns to the same thread. Forking creates a separate thread. app-server `thread/resume` alone does not update `thread.updatedAt` or the rollout file mtime; starting a turn does.

## Branching and Checkpoints

Codex supports forking:

- `/fork` forks the current TUI conversation into a fresh thread id.
- `codex fork [SESSION_ID] [PROMPT]` forks a saved session from the terminal.
- `codex fork --last` picks the latest session without opening the picker.
- app-server `thread/fork` creates a new thread id and reports `forkedFromId` when available.

Codex also supports archive, unarchive, delete, compaction, and app-server rollback:

- `/archive`, `codex archive <SESSION>`, and app-server `thread/archive` move persisted rollout logs to `archived_sessions/`.
- `/delete`, `codex delete <SESSION>`, and app-server `thread/delete` permanently remove active or archived thread logs and spawned descendants.
- `codex unarchive <SESSION>` and app-server `thread/unarchive` restore archived rollout logs.
- `/compact` and app-server `thread/compact/start` summarize history to reduce context.
- app-server `thread/rollback` drops the last N turns from in-memory context and records a rollback marker in the persisted rollout log.

Fork preserves the original session. Rollback mutates the target thread's loaded context and records the mutation; it is not a named checkpoint system. No documented CLI command creates named checkpoints or rewinds to arbitrary earlier points.

## Human-in-the-Loop Resume

Codex CLI does not provide a single native defer token that lets a wrapper pause a non-interactive turn, ask a human elsewhere, and inject the answer into the same suspended turn. The reliable CLI-level pattern is follow-up prompting:

1. Run `codex exec --json "task"`.
2. Capture `thread_id` from `thread.started`.
3. Detect a question from `item.completed` agent messages or from the final assistant output.
4. Ask the user through Claudine's channel.
5. Run `codex exec resume <thread_id> "<answer>"`.

Hooks provide useful interception points but not a generic defer/resume token. `PermissionRequest` can allow or deny approvals before Codex asks the user; docs say reserved fields such as `interrupt` fail closed today. `Stop` and `SubagentStop` can continue after a turn by returning `decision: "block"` with a reason that becomes continuation input.

For true active-turn human-in-the-loop workflows, app-server is the stronger substrate. It has server-initiated approval requests, `item/tool/requestUserInput`, `turn/steer` for adding input to an active turn, and `turn/interrupt` for cancellation. Claudine should treat that as a different integration mode from `codex exec`.

## Interruption Recovery

Because rollout files are written as the session runs, saved sessions survive ordinary terminal exit, Ctrl+C, crashes, and process kills when persistence is enabled. The next resume reloads the last recorded transcript state. Network/API failures in non-interactive mode surface as `turn.failed` or `error` events in `codex exec --json`; the next resume continues from the recorded history rather than reattaching to remote model state.

Pending tools and approvals are more nuanced. The transcript and app-server state can show that a tool or approval was in progress, and official docs say approvals are kept on resume, but the exact replay behavior after a crash during a shell command or pending approval is not specified. app-server exposes `turn/interrupt`, runtime thread status, and loaded-thread APIs, which are better for active recovery than the plain CLI. Concurrent resumes of the same session are not documented as safe; wrappers should serialize appends per session id.

## Observability

Useful observability surfaces:

- `codex exec --json`: `thread.started`, `turn.started`, `turn.completed`, `turn.failed`, `item.started`, `item.completed`, and `error`.
- Hooks: `SessionStart`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `PreCompact`, `PostCompact`, `UserPromptSubmit`, `SubagentStart`, `SubagentStop`, and `Stop`. Common input includes `session_id`, `transcript_path`, `cwd`, `hook_event_name`, and `model`.
- Rollout JSONL: internal transcript records including `session_meta`, `turn_context`, tool calls, tool results, and messages.
- `state_5.sqlite`: local thread inventory with session metadata and rollout paths.
- app-server JSON-RPC: `thread/list`, `thread/read`, `thread/resume`, `thread/loaded/list`, `thread/turns/list`, `thread/status/changed`, `turn/start`, `turn/steer`, `turn/interrupt`, `item/started`, `item/completed`, and approval/request notifications.
- `codex doctor --json`: local diagnostics may help detect state inventory issues, but it is not a session resume API.

## Quirks and Gaps

Quirks:

- Local help for `codex resume` and `codex exec resume` accepts UUIDs or names/thread names, while the public command reference emphasizes UUIDs. Prefer UUIDs for automation.
- `codex exec --ephemeral` and `codex exec resume --ephemeral` deliberately avoid persistent rollout files.
- The observed rollout format stores `session_id` in `session_meta.payload`, not as a top-level JSONL field.
- `session_index.jsonl`, `state_5.sqlite`, and rollout files can drift. Direct `codex resume <id>` may still work when picker/index behavior is stale.
- `thread/list` defaults to interactive source kinds; include `exec` when looking for non-interactive runs through app-server.
- WebSocket app-server transport is experimental and unsupported; non-loopback listeners need explicit auth configuration.
- `thread/resume` does not update recency timestamps until a new turn starts.

Gaps:

- No documented machine-readable CLI session-list command.
- No documented automatic retention policy for rollout files, archived sessions, or `state_5.sqlite`.
- No documented concurrency semantics for multiple appenders resuming the same thread.
- No precise guarantee for replaying a crash that happens mid-tool or mid-approval.
- IDE extension resume behavior was not independently verified.

## Claudine Integration Notes

For Claudine lifecycle `resume`, capture the Codex session id as early as possible from `codex exec --json` `thread.started.thread_id` or from hooks. Persist that id in Claudine's run record. Do not depend on the interactive picker.

For non-interactive continuation, use:

```bash
codex exec resume <session-id> "<follow-up>"
```

Add `--json` when Claudine needs structured continuation events. Do not mark runs launched with `--ephemeral` as resumable.

For `retry`, decide whether the failed turn should be retried as a new follow-up prompt in the same transcript or as a fresh run. If retrying in the same transcript, use `codex exec resume <session-id>` and explicitly override model, cwd, sandbox, or approval settings when Claudine's policy requires them.

For `proxy` or future human-in-the-loop recovery, `codex exec resume` can inject an answer as the next turn after a completed question. It cannot answer an arbitrary suspended approval prompt with a structured token. If Claudine needs active-turn steering, pending user-input handling, or approval request resolution, target app-server JSON-RPC rather than the plain exec CLI.

For session lookup, use captured IDs first. If recovery requires discovery, prefer app-server `thread/list` or `state_5.sqlite` over `session_index.jsonl`, and treat direct file parsing as an unsupported recovery fallback.

## Changelog

- 2026-07-03: Refreshed against `codex-cli 0.142.5`, official docs, local CLI help, local rollout files, and local SQLite state.
- 2026-07-03: Corrected frontmatter producer metadata from prior `opencode`/third-party model values to `agent: codex` and `model: default`.
- 2026-07-03: Added `--ephemeral` as a persistence boundary for non-interactive sessions.
- 2026-07-03: Corrected local rollout JSONL field claims: observed session IDs are nested in `session_meta.payload`.
- 2026-07-03: Added `state_5.sqlite` `threads` as a current local lookup and recovery source.
- 2026-07-03: Added app-server JSON-RPC resume, list, read, fork, rollback, turn/start, and turn/steer behavior.
- 2026-07-03: Split state storage paths into separate macOS, Linux, and Windows records.

## Sources

- [Codex CLI features - Resuming conversations](https://developers.openai.com/codex/cli/features)
- [Codex CLI command reference](https://developers.openai.com/codex/cli/reference)
- [Codex non-interactive mode](https://developers.openai.com/codex/noninteractive)
- [Codex CLI slash commands](https://developers.openai.com/codex/cli/slash-commands)
- [Codex hooks](https://developers.openai.com/codex/hooks)
- [Codex configuration reference](https://developers.openai.com/codex/config-reference)
- [Codex app-server](https://developers.openai.com/codex/app-server)
- [openai/codex repository](https://github.com/openai/codex)
- Local CLI inspection: `codex-cli 0.142.5` help for `codex`, `codex resume`, `codex exec`, `codex exec resume`, `codex fork`, `codex archive`, `codex delete`, and `codex app-server`.
- Local state inspection: `~/.codex/sessions/**/*.jsonl`, `~/.codex/history.jsonl`, `~/.codex/session_index.jsonl`, `~/.codex/state_5.sqlite`, and `~/.codex/archived_sessions/` on this macOS host.
