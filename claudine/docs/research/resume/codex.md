---
$schema: ./_schema.yaml
created: 2026-04-02
last_updated: 2026-07-02
agent: opencode
model: kimi-for-coding/k2p7
docs: https://developers.openai.com/codex/cli/features
support: first_class
continuity_model: transcript_replay
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "codex resume [SESSION_ID]"
      - "codex resume --last"
      - "codex resume --all"
      - "/resume slash command"
      - "interactive session picker TUI"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
      - name
      - picker
      - all_projects
    notes: "Follow-up prompt can be supplied on the command line as the second positional argument. --last scopes to current directory unless --all is passed. --include-non-interactive includes exec sessions in picker/--last."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "codex exec resume [SESSION_ID] [PROMPT]"
      - "codex exec resume --last [PROMPT]"
      - "codex exec resume --all --last [PROMPT]"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
      - name
      - all_projects
    notes: "The only scriptable surface that sends a new prompt into an existing session. Prompt can be literal text or '-' to read from stdin."
  - mode: headless_server
    supported: true
    mechanisms:
      - "codex app-server --listen ws://..."
      - "codex resume --remote ws://..."
      - "codex exec resume --remote ws://... is rejected (remote only supports codex, resume, fork, archive, delete, unarchive)"
    accepts_followup_prompt: false
    selection_methods:
      - id
      - picker
      - latest
    notes: "Remote TUI connects to an app-server. Session state is still local to the app-server host."
  - mode: ide
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "Codex IDE extensions exist but their session resume behavior is out of scope for this CLI-focused research."
  - mode: api
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "No direct public HTTP API for session resume. Programmatic control uses codex exec --json or the app-server/WebSocket layer."
session_id_capture:
  - surface: json_stream
    field: thread_id
    format: uuid
    notes: "codex exec --json emits thread.started with thread_id, which is the session UUID for exec runs."
  - surface: session_file
    field: filename
    format: "rollout-<timestamp>-<uuid>.jsonl"
    notes: "Interactive session transcripts are stored under ~/.codex/sessions/YYYY/MM/DD/rollout-<timestamp>-<uuid>.jsonl. The UUID is the session_id."
  - surface: session_file
    field: session_id
    format: uuid
    notes: "session_id appears as a top-level field inside rollout JSONL events."
  - surface: log_file
    field: session_id
    format: uuid
    notes: "~/.codex/history.jsonl records session_id per user prompt; ~/.codex/session_index.jsonl records id and thread_name."
  - surface: hook
    field: session_id
    format: uuid
    notes: "All lifecycle hooks receive session_id and transcript_path in their stdin JSON payload."
  - surface: interactive_ui
    field: thread_name / session name
    format: string
    notes: "The TUI picker shows names and summaries; raw IDs are visible in rollout filenames or via hooks."
resume_invocations:
  - mode: interactive
    invocation: "codex resume --last [PROMPT]"
    accepts_prompt: true
    selection: latest
    notes: "Resumes the most recent interactive session for the current working directory."
  - mode: interactive
    invocation: "codex resume --all --last [PROMPT]"
    accepts_prompt: true
    selection: latest
    notes: "Resumes the most recent session across all directories."
  - mode: interactive
    invocation: "codex resume <SESSION_ID> [PROMPT]"
    accepts_prompt: true
    selection: id
    notes: "Resumes a specific session by UUID."
  - mode: interactive
    invocation: "codex resume <SESSION_NAME> [PROMPT]"
    accepts_prompt: true
    selection: name
    notes: "Resumes a specific session by name. UUIDs take precedence if the argument parses as a UUID."
  - mode: interactive
    invocation: "/resume"
    accepts_prompt: false
    selection: picker
    notes: "Inside the TUI, opens the saved-session picker."
  - mode: non_interactive
    invocation: "codex exec resume --last [PROMPT]"
    accepts_prompt: true
    selection: latest
    notes: "Resumes the most recent exec session for the current working directory."
  - mode: non_interactive
    invocation: "codex exec resume --all --last [PROMPT]"
    accepts_prompt: true
    selection: all_projects
    notes: "Resumes the most recent exec session across all directories."
  - mode: non_interactive
    invocation: "codex exec resume <SESSION_ID> [PROMPT]"
    accepts_prompt: true
    selection: id
    notes: "Scriptable follow-up into an exact exec session."
  - mode: headless_server
    invocation: "codex resume --remote ws://host:port <SESSION_ID>"
    accepts_prompt: false
    selection: id
    notes: "Connects the TUI to a remote app-server and resumes the session hosted there."
state_storage:
  - location: local
    os: all
    path: "~/.codex/sessions/<YYYY>/<MM>/<DD>/rollout-<timestamp>-<session-id>.jsonl"
    format: JSONL
    retention: "Controlled by history.persistence = save-all | none in ~/.codex/config.toml; no documented automatic cleanup sweep beyond history.max_bytes capping history.jsonl."
    notes: "Interactive transcripts. Format is internal and may change; direct parsing is unsupported. Stable access for scripts should use codex exec --json or hooks."
  - location: local
    os: all
    path: "~/.codex/history.jsonl"
    format: JSONL
    retention: "history.max_bytes caps size by dropping oldest entries when set."
    notes: "One record per user prompt with session_id and ts."
  - location: local
    os: all
    path: "~/.codex/session_index.jsonl"
    format: JSONL
    retention: "Same as rollout files."
    notes: "Index records with id, thread_name, updated_at."
  - location: local
    os: all
    path: "~/.codex/sqlite/"
    format: SQLite
    retention: "Not documented."
    notes: "codex-dev.db and related SQLite files hold goals, logs, memories, and state. Session state is primarily in JSONL transcripts, not the SQLite DB."
  - location: local
    os: all
    path: "~/.codex/archived_sessions/"
    format: JSONL
    retention: "Not documented."
    notes: "Sessions archived with codex archive move here; restored with codex unarchive."
resume_scope:
  project_scoped: true
  cwd_scoped: true
  worktree_aware: false
  all_projects_supported: true
  branch_filtering: false
  notes: "Default lookup is current working directory. --all disables cwd filtering and shows sessions from any directory. No documented git branch or worktree filtering for session resume."
branching_checkpointing:
  branch_supported: true
  checkpoint_supported: false
  fork_invocation: "codex fork [SESSION_ID] [PROMPT]"
  checkpoint_invocation: ""
  preserves_original: true
  notes: "codex fork and the /fork slash command create a new thread with a fresh ID, copying the original transcript. The original remains untouched. codex archive/unarchive manage visibility in the picker but are not checkpoints."
restored_state:
  transcript: true
  tool_results: true
  approvals: preserved
  model: overridable
  cwd: configurable
  env: current_process
  notes: "Transcript and tool results are replayed from the rollout JSONL. turn_context records approval_policy, sandbox_policy, model, effort, etc., and these are restored. Model, cwd, and sandbox can be overridden at resume time with --model, --cd, --sandbox, --ask-for-approval. Environment comes from the launching process, not the session."
hitl_resume:
  supported: false
  question_capture: "No native question-deferral hook. The model can ask a question in an agent_message during codex exec --json; the wrapper must detect it from the stream."
  answer_injection: "Wrapper runs codex exec resume <session_id> '<user_answer>' so the answer appears as the next user turn in the same transcript."
  limitations: "This is follow-up prompting, not a structured answer-injection API. PermissionRequest hooks can allow/deny supported tools programmatically, but exec mode defaults approval to never and PermissionRequest primarily fires for interactive approvals. There is no equivalent to Claude Code's PreToolUse permissionDecision: defer."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: true
  pending_approval_resume: true
  limitations: "Transcripts are written continuously, so crash, Ctrl+C, terminal close, and process kill preserve the session. Pending tool calls and approvals are recovered via transcript replay because approval_policy is stored in turn_context. There is no documented mid-turn auto-resume after network loss; the turn ends and the next resume continues from the last recorded event."
observability:
  stream_events:
    - "thread.started"
    - "turn.started"
    - "turn.completed"
    - "turn.failed"
    - "item.started"
    - "item.completed"
    - "error"
  hook_events:
    - "SessionStart"
    - "PreToolUse"
    - "PermissionRequest"
    - "PostToolUse"
    - "UserPromptSubmit"
    - "Stop"
  failure_modes:
    - "turn.failed"
    - "error"
  notes: "codex exec --json is the stable machine-readable surface. Hooks receive session_id and transcript_path. Stop hooks can continue a turn by returning decision: block with a reason that becomes a new user prompt."
quirks:
  - "codex resume --help accepts an optional [PROMPT] positional argument, but the TUI resumes interactively; the prompt is sent as the first user message after loading the transcript."
  - "codex exec resume --last scopes to the current directory unless --all is passed. There is also --include-non-interactive for the interactive resume picker."
  - "Session IDs appear in rollout filenames and inside rollout JSONL as session_id. Exec --json uses thread_id for the same concept."
  - "The rollout JSONL format is internal and may change between releases; scripts should prefer codex exec --json or hooks."
  - "codex exec runs non-interactively and defaults approval to never; do not rely on interactive approval prompts in exec mode."
  - "codex archive/unarchive and codex delete accept session names or UUIDs; UUIDs take precedence. --force delete requires a UUID."
  - "Remote resume requires an app-server started with codex app-server --listen ws://... and, for non-local connections, WebSocket auth."
gaps:
  - "No documented automatic cleanup/retention sweep for session rollout files; only history.max_bytes and history.persistence are exposed."
  - "Exact concurrency semantics when the same session is resumed from multiple terminals or processes are not documented."
  - "Whether MCP server connection state and OAuth tokens survive resume is not explicitly documented."
  - "No public API to list sessions programmatically; only the interactive picker and the local JSONL index files."
changes:
  - "2026-07-02: Converted from free-form prompt frontmatter to schema-validated fields."
  - "2026-07-02: Updated resume invocation list to include codex resume [PROMPT], --include-non-interactive, and codex exec resume --all --last."
  - "2026-07-02: Documented local storage paths (rollout JSONL, history.jsonl, session_index.jsonl, archived_sessions, sqlite) from local inspection."
  - "2026-07-02: Clarified that Codex has no native question-deferral mechanism; HITL must use follow-up prompting via codex exec resume."
requires_claudine_update: true
reason: "The schema-validated facts, exact resume invocations, session-id capture surfaces, and the corrected HITL model (follow-up prompting rather than native defer) should feed into Claudine's lifecycle resume action, session-id capture logic, and provider metadata for Codex."
---

## Overview

Codex CLI's session resume is a first-class, local-transcript-replay system. Every session writes continuously to a JSONL rollout file under `~/.codex/sessions/<YYYY>/<MM>/<DD>/`, and the same file is replayed when the session is continued, resumed by ID, or resumed by name. The CLI supports both interactive resumption (`codex resume`, `/resume`) and scriptable non-interactive follow-up (`codex exec resume <session-id> "<prompt>"`). There is no native question-deferral mechanism like Claude Code's `permissionDecision: defer`; a wrapper that wants to broker human answers must detect the question from the `codex exec --json` stream and resume with the answer as a follow-up prompt.

## Resume Semantics

A Codex "session" is a persisted conversation transcript stored locally on disk. Resume means loading that transcript and appending new turns to it, not reattaching to a remote server or live process. The authoritative state is the rollout JSONL transcript plus the `turn_context` recorded in it (approval policy, sandbox policy, model, effort, etc.). Sessions can be resumed interactively, non-interactively, or through a TUI connected to a remote app-server. The Codex desktop app, IDE extensions, and Codex Cloud tasks maintain their own session history and are out of scope for the CLI behavior described here.

## Supported Modes

| Mode | Entry point | Follow-up prompt | Selector |
|------|-------------|------------------|----------|
| Interactive CLI | `codex resume --last [PROMPT]` | Yes | Latest in current directory |
| Interactive CLI | `codex resume --all --last [PROMPT]` | Yes | Latest across all directories |
| Interactive CLI | `codex resume <ID> [PROMPT]` | Yes | Exact session ID |
| Interactive CLI | `codex resume <NAME> [PROMPT]` | Yes | Exact session name |
| Interactive CLI | `/resume` | No | Picker |
| Non-interactive CLI | `codex exec resume --last [PROMPT]` | Yes | Latest in current directory |
| Non-interactive CLI | `codex exec resume --all --last [PROMPT]` | Yes | Latest across all directories |
| Non-interactive CLI | `codex exec resume <ID> [PROMPT]` | Yes | Exact session ID |
| Remote TUI | `codex resume --remote ws://... <ID>` | No | ID/picker via app-server |

`codex exec` sessions do not appear in the interactive resume picker unless `--include-non-interactive` is passed, but they are fully resumable by ID and by `--last`.

## Session ID Capture

Stable session identifiers are UUIDs. Capture surfaces:

- **`codex exec --json`**: `thread.started` emits `thread_id`, which is the session UUID for exec runs.
- **Rollout filename**: `~/.codex/sessions/<YYYY>/<MM>/<DD>/rollout-<timestamp>-<uuid>.jsonl`.
- **Rollout JSONL**: `session_id` appears as a top-level field in events.
- **Hooks**: every hook input JSON includes `session_id` and `transcript_path`.
- **`~/.codex/history.jsonl`**: records `session_id` per user prompt with timestamp.
- **`~/.codex/session_index.jsonl`**: records `id`, `thread_name`, and `updated_at`.

The interactive picker intentionally shows session names and summaries rather than raw IDs.

## Resume Invocation

Continue the latest interactive session:

```bash
codex resume --last
codex resume --last "next step"
```

Resume the latest session across all directories:

```bash
codex resume --all --last
codex resume --all --last "next step"
```

Resume a specific session by ID:

```bash
codex resume 019f2318-c526-7ad1-be39-431a13d3a30b
codex resume 019f2318-c526-7ad1-be39-431a13d3a30b "implement the plan"
```

Resume in non-interactive mode:

```bash
codex exec resume --last "fix the race conditions you found"
codex exec resume 019f2318-c526-7ad1-be39-431a13d3a30b "implement the plan"
```

Resume from inside the TUI:

```text
/resume
```

## Session Lookup Scope

Sessions are stored per working directory. The default lookup is scoped to the current working directory. The interactive picker can widen with `--all`, and `codex resume --all --last` selects the most recent session across all directories. There is no documented git branch or worktree filtering for session resume.

## State Storage

Resumable state is local, not server-side.

| OS | Interactive transcript path |
|----|-----------------------------|
| macOS / Linux / WSL | `~/.codex/sessions/<YYYY>/<MM>/<DD>/rollout-<timestamp>-<uuid>.jsonl` |
| Windows | `%USERPROFILE%\.codex\sessions\<YYYY>\<MM>\<DD>\rollout-<timestamp>-<uuid>.jsonl` |

Additional local state files:

| File | Purpose |
|------|---------|
| `~/.codex/history.jsonl` | One record per user prompt with `session_id` and `ts`. |
| `~/.codex/session_index.jsonl` | Index with `id`, `thread_name`, `updated_at`. |
| `~/.codex/sqlite/codex-dev.db` | SQLite state for goals, logs, memories, etc. |
| `~/.codex/archived_sessions/` | Transcripts moved here by `codex archive`. |

Each line of the rollout JSONL file is an internal event, message, tool call, or metadata entry. The format is explicitly internal and may change between releases; direct parsing is unsupported. For stable access, use `codex exec --json` or hooks.

Persistence is controlled by `history.persistence = "save-all" | "none"` in `~/.codex/config.toml`. `history.max_bytes` caps `history.jsonl` by dropping oldest entries when set. There is no documented automatic cleanup sweep for rollout files.

## Restored State

Resuming restores:

- The full conversation transcript and tool results recorded in the rollout JSONL.
- The `turn_context` recorded at the start of the turn, including `approval_policy`, `sandbox_policy`, `model`, `effort`, `personality`, and `cwd`.
- Any `--add-dir` directories from the original launch.
- MCP servers, skills, plugins, and rules from the active config layers.

Resuming does not restore the environment of the original session; it uses the environment of the process that runs the resume command. Model, cwd, sandbox, and approval policy can be overridden at resume time with `--model`, `--cd`, `--sandbox`, and `--ask-for-approval`.

## Branching and Checkpoints

Branching creates a copy of the conversation and leaves the original intact.

- `/fork` from inside a session.
- `codex fork [SESSION_ID] [PROMPT]` from the terminal.
- `codex fork --last` skips the picker.

Codex does not have a checkpoint/rewind feature. `codex archive` and `codex unarchive` manage visibility in the session picker but do not revert conversation state.

## Human-in-the-Loop Resume

Codex does **not** provide a native defer-and-resume loop for non-interactive sessions. There is no equivalent to Claude Code's `PreToolUse` `permissionDecision: "defer"`. A wrapper can emulate HITL as follows:

1. Run `codex exec --json "task that may ask a question"`.
2. Capture `thread_id` from the `thread.started` event.
3. Parse the JSONL stream. If the model asks a question in an `agent_message` or a tool fails requiring clarification, the turn completes.
4. Present the question to the user.
5. Run `codex exec resume <thread_id> "<user_answer>"` so the answer becomes the next user turn in the same transcript.

Caveats:

- This is follow-up prompting, not structured answer injection.
- `PermissionRequest` hooks can allow or deny supported tool approvals programmatically, but `codex exec` defaults to `approval_policy = never`, so interactive approval prompts are not surfaced in exec mode.
- The `Stop` hook can continue a completed turn by returning `decision: "block"` with a `reason` that becomes a new user prompt, but this is a continuation hook, not a mid-turn pause.

## Interruption Recovery

Because transcripts are written continuously, sessions survive most interruptions:

- **Crash / terminal close / process kill**: the rollout JSONL remains and can be resumed.
- **Ctrl+C**: the session is preserved; resume with `codex resume --last` or `codex exec resume --last`.
- **Pending tool calls**: preserved in the transcript and replayed on resume.
- **Pending approvals**: the `approval_policy` stored in `turn_context` is restored on resume, so the same approval state applies.
- **Network failure / API error**: the current turn ends with a `turn.failed` or `error` event in `codex exec --json`; resume continues from the last recorded event. There is no documented mid-turn auto-resume.

## Observability

Events and surfaces that expose session identity or resumability:

- **`codex exec --json`**: `thread.started`, `turn.started`, `turn.completed`, `turn.failed`, `item.started`, `item.completed`, and `error` events.
- **Hooks**: `SessionStart`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `UserPromptSubmit`, and `Stop` all include `session_id` and `transcript_path`.
- **Rollout files**: `~/.codex/sessions/<YYYY>/<MM>/<DD>/rollout-<timestamp>-<uuid>.jsonl` and the embedded `session_id` field.
- **Index files**: `~/.codex/history.jsonl` and `~/.codex/session_index.jsonl`.

## Quirks and Gaps

Quirks:

- `codex resume --help` shows an optional `[PROMPT]` argument, but the resumed session runs in the TUI; the prompt is sent as the first user message after loading the transcript.
- `codex exec resume --last` scopes to the current directory unless `--all` is passed.
- Interactive resume can include non-interactive sessions in the picker with `--include-non-interactive`.
- Session IDs appear in rollout filenames and as `session_id` inside rollout JSONL, while `codex exec --json` uses `thread_id` for the same UUID.
- The rollout JSONL format is internal and may change between releases; scripts should use `codex exec --json` or hooks.
- `codex archive`/`unarchive` and `codex delete` accept session names or UUIDs; UUIDs take precedence. `codex delete --force` requires a UUID.
- Remote resume requires an app-server started with `codex app-server --listen ws://...`; the TUI connects with `codex resume --remote ws://...`.

Gaps:

- No documented automatic cleanup/retention sweep for rollout files beyond `history.max_bytes`.
- Exact concurrency guarantees when the same session is resumed from multiple processes are not documented.
- Whether MCP server connection state and OAuth tokens survive resume is not explicitly documented.
- No public API or CLI command to list sessions programmatically; wrappers must read the local index files or use hooks.

## Claudine Integration Notes

For Claudine's lifecycle `resume` action and future HITL broker:

- Capture the session ID from `codex exec --json` (`thread_id` in `thread.started`) or from a `SessionStart` hook.
- Use `codex exec resume <session-id> "<follow-up>"` for non-interactive continuation.
- For human-in-the-loop, detect questions from `agent_message` events in the `codex exec --json` stream, ask the user, then resume with the answer as a follow-up prompt. Do not expect a native defer/answer-injection API.
- Treat the rollout JSONL transcript as read-only and unstable; do not build parsing logic against it.
- When resuming after a failure, be aware that `approval_policy` and `sandbox_policy` are restored from `turn_context`; override them explicitly if the launch environment requires different permissions.
- Consider reading `~/.codex/session_index.jsonl` or `~/.codex/history.jsonl` only as a fallback for session discovery; prefer capturing IDs from the stream or hooks.

## Changelog

- 2026-07-02: Converted to schema-validated frontmatter.
- 2026-07-02: Added exact resume invocations, `--include-non-interactive`, and `--all` scoping.
- 2026-07-02: Documented local storage paths from local inspection.
- 2026-07-02: Clarified that Codex has no native question-deferral mechanism; HITL uses follow-up prompting.

## Sources

- [Codex CLI features — Resuming conversations](https://developers.openai.com/codex/cli/features)
- [Codex CLI command reference](https://developers.openai.com/codex/cli/reference)
- [Codex CLI slash commands](https://developers.openai.com/codex/cli/slash-commands)
- [Codex non-interactive mode](https://developers.openai.com/codex/noninteractive)
- [Codex hooks](https://developers.openai.com/codex/hooks)
- [Codex environment variables](https://developers.openai.com/codex/environment-variables)
- [Codex configuration reference](https://developers.openai.com/codex/config-reference)
- [openai/codex repository](https://github.com/openai/codex)
