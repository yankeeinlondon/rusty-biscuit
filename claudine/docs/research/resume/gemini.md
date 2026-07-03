---
$schema: ./_schema.yaml
created: 2026-04-02
last_updated: 2026-07-02
agent: opencode
model: kimi-for-coding/k2p7
docs: https://geminicli.com/docs/cli/tutorials/session-management/
support: first_class
continuity_model: transcript_replay
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "gemini -r latest"
      - "gemini -r <index>"
      - "/resume slash command"
      - "interactive session picker TUI"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - index
      - picker
    notes: "Follow-up prompt can be supplied as an additional positional argument (gemini -r latest 'next step'). --resume accepts 'latest' or a numeric index from --list-sessions."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "gemini -p '<prompt>' --resume latest"
      - "gemini -p '<prompt>' --resume <index>"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - index
    notes: "Non-interactive resume uses the -p/--prompt flag. The resume selector is still 'latest' or an index."
  - mode: headless_server
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "No standalone headless server CLI."
  - mode: ide
    supported: true
    mechanisms:
      - "VS Code companion extension resume"
    accepts_followup_prompt: false
    selection_methods:
      - picker
      - latest
    notes: "IDE surfaces keep their own session history; this research focuses on CLI behavior."
  - mode: api
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "No direct public HTTP API for session resume. Programmatic control uses gemini -p/--prompt with --output-format json/stream-json."
session_id_capture:
  - surface: json_stream
    field: sessionId
    format: uuid
    notes: "Headless --output-format stream-json/json emits an init event with session metadata. Field name observed as camelCase in session files; stream init event is documented to include session ID and model."
  - surface: hook
    field: session_id
    format: uuid
    notes: "All hook stdin JSON payloads include session_id and transcript_path."
  - surface: session_file
    field: sessionId
    format: uuid
    notes: "First line of the session JSONL contains sessionId, projectHash, startTime, lastUpdated, kind. Filename is session-<timestamp>-<sessionId>.jsonl."
  - surface: cli_command
    field: index + sessionId
    format: "<index>. <snippet> (<relative time>) [<uuid>]"
    notes: "gemini --list-sessions emits an ordered list with index, preview, age, and UUID."
  - surface: other
    field: GEMINI_SESSION_ID
    format: uuid
    notes: "Environment variable set in hook/command subprocesses."
resume_invocations:
  - mode: interactive
    invocation: "gemini -r latest"
    accepts_prompt: true
    selection: latest
    notes: "Resumes the most recent session for the current project. An optional positional prompt can follow."
  - mode: interactive
    invocation: "gemini -r <index>"
    accepts_prompt: true
    selection: index
    notes: "Resumes the session at the given index from gemini --list-sessions."
  - mode: interactive
    invocation: "gemini -r latest 'next step'"
    accepts_prompt: true
    selection: latest
    notes: "Continue latest session and send the prompt as the next user turn."
  - mode: interactive
    invocation: "/resume"
    accepts_prompt: false
    selection: picker
    notes: "Inside an active session, opens the interactive session browser."
  - mode: non_interactive
    invocation: "gemini -p 'next step' --resume latest"
    accepts_prompt: true
    selection: latest
    notes: "Scriptable follow-up into the most recent session for the current project."
  - mode: non_interactive
    invocation: "gemini -p 'next step' --resume 5"
    accepts_prompt: true
    selection: index
    notes: "Scriptable follow-up into the session at index 5."
  - mode: interactive
    invocation: "/resume save <tag>"
    accepts_prompt: false
    selection: name
    notes: "Saves the current conversation as a named checkpoint."
  - mode: interactive
    invocation: "/resume resume <tag>"
    accepts_prompt: false
    selection: name
    notes: "Loads a previously saved named checkpoint, forking the conversation."
state_storage:
  - location: local
    os: all
    path: "~/.gemini/tmp/<project_hash>/chats/session-<timestamp>-<session_id>.jsonl"
    format: JSONL
    retention: "general.sessionRetention.maxAge default '30d', minRetention '1d'; maxCount optional."
    notes: "Session transcript and metadata are stored locally per project. Format is internal; direct parsing is unsupported."
  - location: local
    os: all
    path: "~/.gemini/tmp/<project_hash>/"
    format: directory
    retention: "Same session retention sweep."
    notes: "Manual chat checkpoints from /resume save live here. Windows path: C:\\Users\\<user>\\.gemini\\tmp\\<project_hash>\\."
  - location: local
    os: all
    path: "~/.gemini/history/<project_hash>"
    format: shadow Git repository
    retention: "Same session retention sweep."
    notes: "Automatic file-system checkpoints created before mutating tools when general.checkpointing.enabled is true."
resume_scope:
  project_scoped: true
  cwd_scoped: true
  worktree_aware: false
  all_projects_supported: false
  branch_filtering: false
  notes: "Session lookup is scoped to the current project directory. --list-sessions, /resume, and saved checkpoints only show sessions/checkpoints for the current project."
branching_checkpointing:
  branch_supported: true
  checkpoint_supported: true
  fork_invocation: '"/resume save <tag>" then "/resume resume <tag>"'
  checkpoint_invocation: "/restore [tool_call_id]"
  preserves_original: true
  notes: "Manual /resume checkpoints create a new branch of history. Automatic checkpointing (disabled by default) snapshots files in a shadow Git repo and can be restored with /restore."
restored_state:
  transcript: true
  tool_results: true
  approvals: cleared
  model: overridable
  cwd: restored
  env: current_process
  notes: "Conversation transcript and tool results are replayed from the local JSONL. Approval mode and sandbox settings come from launch flags/settings, not the session. Model can be overridden with --model."
hitl_resume:
  supported: true
  question_capture: "BeforeTool hook matching ask_user receives tool_input.questions in stdin JSON. Notification hook receives ToolPermission alerts but cannot act on them."
  answer_injection: "No native defer/answer-injection API. A wrapper can deny or block the ask_user tool via BeforeTool, ask the user elsewhere, then resume the session with the answer as a new prompt using gemini -r <id> -p '<answer>'."
  limitations: "BeforeTool can allow, deny/block, or rewrite tool_input; it cannot pause mid-turn and resume later. ask_user is inherently interactive and may fail in non-TTY headless mode. Notification hook is observability-only."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: true
  pending_approval_resume: true
  limitations: "Sessions are written continuously, so crash, Ctrl+C, terminal close, and process kill preserve the transcript. No documented mid-turn auto-resume after network failure; ask_user and interactive approval prompts may block or error in non-interactive mode."
observability:
  stream_events:
    - "init"
    - "message"
    - "tool_use"
    - "tool_result"
    - "error"
    - "result"
  hook_events:
    - "SessionStart"
    - "SessionEnd"
    - "BeforeAgent"
    - "AfterAgent"
    - "BeforeModel"
    - "AfterModel"
    - "BeforeToolSelection"
    - "BeforeTool"
    - "AfterTool"
    - "Notification"
    - "PreCompress"
  failure_modes:
    - "error"
    - "tool error / denied tool"
  notes: "All hooks receive session_id and transcript_path. The init stream event carries session metadata. Stop reasons are not documented as a first-class resume signal."
quirks:
  - "--resume accepts 'latest' or an index per gemini --help. The CLI cheatsheet also shows UUID examples, which creates ambiguity about whether raw session IDs are accepted."
  - "Sessions are strictly project-scoped; --list-sessions and /resume only show sessions for the current project directory."
  - "Manual conversation checkpoints (/resume save/resume) are separate from automatic file checkpoints (/restore)."
  - "Automatic checkpointing is disabled by default and must be enabled in settings.json (general.checkpointing.enabled)."
  - "--session-file loads a session from an arbitrary JSON file; --session-id manually sets the UUID for a new session."
  - "ask_user is an interactive tool; non-interactive mode (-p) may not be able to render it unless a hook intercepts it."
  - "The session JSONL format is internal and not documented as stable for external parsers."
gaps:
  - "Whether --resume accepts a raw session UUID in addition to an index (CLI help vs cheatsheet conflict)."
  - "Exact schema and field names of the stream-json init event."
  - "Whether approval state (e.g., 'always allow' choices) is persisted inside the session file."
  - "Exact concurrency semantics when the same session is resumed from multiple processes."
  - "Whether MCP server connection state and OAuth tokens survive resume."
  - "How project_hash is computed from the project directory."
changes:
  - "2026-07-02: Replaced free-form prompt frontmatter with schema-validated fields."
  - "2026-07-02: Added exact CLI invocations, session-id capture surfaces, local storage paths, and retention settings from official docs and local inspection."
  - "2026-07-02: Clarified HITL model: BeforeTool intercepts ask_user, but answer injection must be emulated via follow-up resume prompt."
  - "2026-07-02: Distinguished manual /resume checkpoints from automatic /restore checkpointing."
requires_claudine_update: true
reason: "The schema-validated facts, exact resume invocations, session-id capture surfaces, and the corrected HITL model for Gemini CLI should feed into Claudine's lifecycle resume action, session-id capture logic, and provider metadata."
---

## Overview

Gemini CLI's session resume is a first-class, local-transcript-replay system. Every session writes continuously to a JSONL file under `~/.gemini/tmp/<project_hash>/chats/`, and the same file is replayed when the session is continued with `--resume` or `/resume`. Sessions are scoped to the project directory, so resuming from a different directory requires running Gemini CLI from the original project.

## Resume Semantics

A Gemini CLI "session" is a persisted conversation transcript stored locally on disk, plus optional automatic file checkpoints in a shadow Git repository. Resume means loading that transcript and appending new turns to it, not reattaching to a remote server or live process. The authoritative state is the local JSONL transcript.

## Supported Modes

| Mode | Entry point | Follow-up prompt | Selector |
|------|-------------|------------------|----------|
| Interactive CLI | `gemini -r latest` | Optional positional | Latest session in current project |
| Interactive CLI | `gemini -r <index>` | Optional positional | Session by `--list-sessions` index |
| Interactive CLI | `/resume` | No | Picker |
| Interactive CLI | `/resume save <tag>` / `/resume resume <tag>` | No | Named checkpoint |
| Non-interactive CLI | `gemini -p "<prompt>" --resume latest` | Yes (`-p`) | Latest session in current project |
| Non-interactive CLI | `gemini -p "<prompt>" --resume <index>` | Yes (`-p`) | Session by index |

There is no standalone headless server or public HTTP API for session resume.

## Session ID Capture

Stable session identifiers are UUIDs. Capture surfaces:

- **`--output-format stream-json` / `json`**: the `init` event includes the session ID.
- **Hooks**: every hook stdin JSON includes `session_id` and `transcript_path`.
- **Session file**: `~/.gemini/tmp/<project_hash>/chats/session-<timestamp>-<session_id>.jsonl`; the first line contains `sessionId`.
- **`--list-sessions`**: prints an indexed list with the UUID in brackets.
- **Environment**: `GEMINI_SESSION_ID` is set in hook/command subprocesses.

## Resume Invocation

Continue the latest session:

```bash
gemini -r latest
gemini -r latest "continue with the next step"
```

Resume by index:

```bash
gemini --list-sessions
gemini -r 1
gemini -r 1 "finish this task"
```

Resume in non-interactive mode:

```bash
gemini -p "finish this task" --resume latest
gemini -p "finish this task" --resume 1
```

Resume from inside the TUI:

```text
/resume
```

Save and resume a named checkpoint (fork):

```text
/resume save decision-point
...
/resume resume decision-point
```

## Session Lookup Scope

Sessions are stored per project. The default lookup is scoped to the current project directory:

- `gemini --list-sessions` shows only sessions for the current project.
- `/resume` only lists sessions and checkpoints saved within the current project.
- To resume a session from a different project, run Gemini CLI from that project's directory.

There is no documented git branch, worktree, or all-projects filter for session resume.

## State Storage

Resumable state is local, not server-side.

| OS | Session transcript path |
|----|-------------------------|
| macOS / Linux | `~/.gemini/tmp/<project_hash>/chats/session-<timestamp>-<session_id>.jsonl` |
| Windows | `C:\Users\<user>\.gemini\tmp\<project_hash>\chats\session-<timestamp>-<session_id>.jsonl` |

Additional local state:

| Path | Purpose |
|------|---------|
| `~/.gemini/tmp/<project_hash>/` | Manual chat checkpoints from `/resume save` |
| `~/.gemini/history/<project_hash>` | Shadow Git repository for automatic file checkpoints |

Retention is controlled by `general.sessionRetention` in `settings.json` (default `maxAge: "30d"`, minimum `"1d"`, optional `maxCount`).

## Restored State

Resuming restores:

- The full conversation transcript and tool results from the JSONL file.
- The working directory/project context.

Resuming does **not** restore:

- The approval mode or sandbox policy; these come from launch flags/settings (`--approval-mode`, `--sandbox`, `general.defaultApprovalMode`, etc.).
- The environment of the original session; it uses the environment of the process that runs the resume command.
- The model selection can be overridden at resume time with `--model` / `-m`.

## Branching and Checkpoints

Gemini CLI supports two related but distinct checkpoint mechanisms:

- **Manual conversation checkpoints**: `/resume save <tag>` saves the current conversation history. `/resume resume <tag>` loads it, creating a new branch of history without losing the original.
- **Automatic file checkpoints**: enabled with `general.checkpointing.enabled` in `settings.json`. Before any mutating tool runs, Gemini CLI creates a shadow Git commit under `~/.gemini/history/<project_hash>`. Use `/restore [tool_call_id]` to revert files and conversation to that point.

## Human-in-the-Loop Resume

Gemini CLI does **not** provide a native defer-and-resume loop like Claude Code's `permissionDecision: "defer"`. A wrapper can emulate HITL as follows:

1. Register a `BeforeTool` hook matching the `ask_user` tool.
2. When the hook fires, capture `tool_input.questions`.
3. Return `decision: "deny"` (or exit code 2) so the tool does not execute.
4. The wrapper presents the question to the user and collects an answer.
5. Resume the session with the answer as a follow-up prompt: `gemini -r <session-id> -p "<user_answer>"`.

Caveats:

- `BeforeTool` can allow, deny/block, or rewrite `tool_input`; it cannot pause mid-turn and resume later.
- The `Notification` hook can observe `ToolPermission` alerts but cannot grant permissions.
- `ask_user` is inherently interactive and may fail in non-TTY headless mode.

## Interruption Recovery

Because transcripts are written continuously, sessions survive most interruptions:

- **Crash / terminal close / process kill**: the JSONL file remains and can be resumed.
- **Ctrl+C**: the session is preserved; resume with `gemini -r latest`.
- **Pending tool calls**: the transcript records the request; resuming replays context and the model can continue.
- **Pending approvals**: the approval mode is re-evaluated from launch settings on resume.

There is no documented mid-turn auto-resume after a network failure.

## Observability

Events and surfaces that expose session identity or resumability:

- **`--output-format stream-json`**: `init`, `message`, `tool_use`, `tool_result`, `error`, and `result` events.
- **Hooks**: `SessionStart`, `SessionEnd`, `BeforeAgent`, `AfterAgent`, `BeforeModel`, `AfterModel`, `BeforeToolSelection`, `BeforeTool`, `AfterTool`, `Notification`, and `PreCompress` all include `session_id` and `transcript_path`.
- **Local files**: the session JSONL filename and first-line metadata.
- **`--list-sessions`**: emits the index and UUID for each project session.

## Quirks and Gaps

Quirks:

- `--help` says `--resume` accepts `"latest"` or an index, while the CLI cheatsheet also shows UUID examples. Treat index/`latest` as the documented stable selector.
- Sessions are strictly project-scoped; there is no built-in cross-project resume.
- Manual `/resume` checkpoints and automatic `/restore` checkpoints are separate systems.
- Automatic checkpointing is off by default.
- The session JSONL format is internal and not versioned for external consumers.

Gaps:

- Whether `--resume` accepts a raw session UUID in addition to an index.
- Exact field names of the stream-json `init` event.
- Whether approval state ("always allow" choices) is persisted in the session file.
- Concurrency guarantees when the same session is resumed from multiple processes.
- Whether MCP server connection state and OAuth tokens survive resume.
- How `project_hash` is derived from the project directory.

## Claudine Integration Notes

For Claudine's lifecycle `resume` action and future HITL broker:

- Capture the session ID from the `init` event of `gemini -p ... --output-format stream-json` or from a `SessionStart`/`BeforeTool` hook.
- Use `gemini -p "<follow-up>" --resume latest` or `--resume <index>` for non-interactive continuation. Keep a mapping from Claudine's run to the numeric index or UUID returned by `--list-sessions`.
- For human-in-the-loop, install a `BeforeTool` hook matching `ask_user`, deny the tool, extract `tool_input.questions`, ask the user, then resume with the answer as a follow-up prompt.
- Treat the JSONL transcript as read-only and unstable; do not build parsing logic against it.
- Be aware that approval mode and sandbox settings are not restored from the session; pass `--approval-mode`/`--sandbox` explicitly if the launch environment requires them.
- Consider the project-scoped lookup: resume must be run from the same project directory as the original session.

## Changelog

- 2026-07-02: Converted to schema-validated frontmatter.
- 2026-07-02: Added exact resume invocations, `--list-sessions` output format, and local storage paths from docs and local inspection.
- 2026-07-02: Documented the HITL model as hook intercept + follow-up resume prompt rather than native defer.
- 2026-07-02: Distinguished manual `/resume` checkpoints from automatic `/restore` checkpointing.

## Sources

- [Gemini CLI — Session management tutorial](https://geminicli.com/docs/cli/tutorials/session-management/)
- [Gemini CLI — Checkpointing](https://geminicli.com/docs/cli/checkpointing/)
- [Gemini CLI — Rewind](https://geminicli.com/docs/cli/rewind/)
- [Gemini CLI — Headless mode](https://geminicli.com/docs/cli/headless/)
- [Gemini CLI — Hooks reference](https://geminicli.com/docs/hooks/reference/)
- [Gemini CLI — Hooks overview](https://geminicli.com/docs/hooks/)
- [Gemini CLI — Command reference](https://geminicli.com/docs/reference/commands/)
- [Gemini CLI — CLI cheatsheet](https://geminicli.com/docs/cli/cli-reference/)
- [Gemini CLI — Configuration reference](https://geminicli.com/docs/reference/configuration/)
- [Gemini CLI — Tools reference](https://geminicli.com/docs/reference/tools/)
- [Gemini CLI — Ask User tool](https://geminicli.com/docs/tools/ask-user/)
- [Gemini CLI repository](https://github.com/google-gemini/gemini-cli)
