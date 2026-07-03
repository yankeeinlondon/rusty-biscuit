---
$schema: ./_schema.yaml
created: 2026-04-02
last_updated: 2026-07-02
agent: opencode
model: kimi-for-coding/k2p7
docs: https://code.claude.com/docs/en/sessions
support: first_class
continuity_model: transcript_replay
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "CLI --continue"
      - "CLI --resume (session picker)"
      - "CLI --resume <name>"
      - "/resume slash command"
      - "interactive session picker TUI"
    accepts_followup_prompt: false
    selection_methods:
      - latest
      - id
      - name
      - picker
      - worktree
      - all_projects
    notes: "Follow-up prompts are typed inside the resumed interactive session, not supplied on the resume command line."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "claude -p --continue"
      - "claude -p --resume <session-id> [prompt]"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
    notes: "The only scriptable surface that sends a new prompt into an existing session."
  - mode: headless_server
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "No standalone headless server CLI. Background sessions are supervised local processes, not a remote API."
  - mode: ide
    supported: true
    mechanisms:
      - "VS Code extension resume past conversations"
      - "JetBrains plugin resume"
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
    notes: "No direct public HTTP API for session resume. Programmatic control uses the Agent SDK or claude -p."
session_id_capture:
  - surface: json_stream
    field: session_id
    format: uuid
    notes: "Present in every --output-format json / stream-json event and in the final result object."
  - surface: hook
    field: session_id
    format: uuid
    notes: "Present in all hook JSON inputs, including SessionStart, PreToolUse, and PermissionRequest."
  - surface: log_file
    field: CLAUDE_CODE_SESSION_ID
    format: uuid
    notes: "Set in the environment of Bash, PowerShell, hook, and MCP stdio subprocesses. Debug logs are ~/.claude/debug/<session-id>.txt."
  - surface: session_file
    field: filename
    format: "<session-id>.jsonl"
    notes: "Transcript path is ~/.claude/projects/<encoded-cwd>/<session-id>.jsonl."
  - surface: cli_command
    field: sessionId
    format: uuid
    notes: "claude agents --json returns sessionId for active and recently-completed background sessions."
  - surface: interactive_ui
    field: name
    format: string
    notes: "The picker shows names, summaries, and branches, not raw IDs. IDs are exposed via /rename or external listing."
resume_invocations:
  - mode: interactive
    invocation: "claude --continue"
    accepts_prompt: false
    selection: latest
    notes: "Resumes the most recent session for the current directory / worktree."
  - mode: interactive
    invocation: "claude --resume"
    accepts_prompt: false
    selection: picker
    notes: "Opens the interactive session picker."
  - mode: interactive
    invocation: "claude --resume <name>"
    accepts_prompt: false
    selection: name
    notes: "Exact name match across the repository and its worktrees."
  - mode: interactive
    invocation: "/resume"
    accepts_prompt: false
    selection: picker
    notes: "Inside an active session, opens the session picker."
  - mode: interactive
    invocation: "/resume <name>"
    accepts_prompt: false
    selection: name
    notes: "Exact name match; ambiguous names error inside the TUI."
  - mode: non_interactive
    invocation: "claude -p --continue"
    accepts_prompt: true
    selection: latest
    notes: "Continues the most recent non-interactive session in the current directory."
  - mode: non_interactive
    invocation: "claude -p --resume <session-id> [follow-up prompt]"
    accepts_prompt: true
    selection: id
    notes: "Scriptable follow-up into an exact session."
state_storage:
  - location: local
    os: all
    path: "~/.claude/projects/<encoded-cwd>/<session-id>.jsonl"
    format: JSONL
    retention: "30 days by default (settings.cleanupPeriodDays); minimum 1 day"
    notes: "The encoded directory name replaces non-alphanumeric characters with '-'. Format is internal and changes between releases; direct parsing is unsupported. Use --output-format json or /export for stable interfaces."
  - location: local
    os: all
    path: "~/.claude/jobs/<short-id>/"
    format: "supervisor background-session files"
    retention: "Same cleanupPeriodDays sweep"
    notes: "Background sessions from agent view / claude --bg. Transcripts still live under ~/.claude/projects/."
resume_scope:
  project_scoped: true
  cwd_scoped: true
  worktree_aware: true
  all_projects_supported: true
  branch_filtering: true
  notes: "Default lookup is current directory / worktree. Ctrl+W widens to all worktrees, Ctrl+A to all projects, Ctrl+B filters by git branch. Resuming by ID from another worktree of the same repo resumes in place; unrelated projects copy a cd + resume command to clipboard."
branching_checkpointing:
  branch_supported: true
  checkpoint_supported: true
  fork_invocation: "/branch [name]"
  checkpoint_invocation: "/rewind"
  preserves_original: true
  notes: "Branching leaves the original session intact and creates a copy. CLI equivalent is --continue --fork-session or --resume <id> --fork-session. Checkpointing (/rewind) reverts code and/or conversation within the same session; it does not create a separate session."
restored_state:
  transcript: true
  tool_results: true
  approvals: session_only
  model: overridable
  cwd: restored
  env: current_process
  notes: "Transcript and tool results are replayed from local JSONL. The permission mode active when a tool was deferred is restored on --resume, except plan and bypassPermissions are never carried over. Model and cwd can be overridden at resume time; environment comes from the launching process, not the session."
hitl_resume:
  supported: true
  question_capture: "PreToolUse hook on AskUserQuestion receives tool_input.questions in stdin JSON. Notification hook with matcher permission_prompt or elicitation_dialog can also surface prompts."
  answer_injection: "On claude -p --resume <session-id>, the same PreToolUse hook fires again and returns permissionDecision: allow with updatedInput.answers mapping question text to answer label."
  limitations: "Only works in non-interactive -p mode. Only valid when Claude makes a single tool call in the turn; batches ignore defer. Interactive sessions log a warning and ignore defer. MCP tools marked _meta[anthropic/requiresUserInteraction] cannot be auto-approved by hook."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: true
  pending_approval_resume: true
  limitations: "Transcripts are written continuously, so sessions survive crash, Ctrl+C, terminal close, and process kill. Deferred tools are preserved in the transcript and re-run on --resume. CLAUDE_CODE_RESUME_INTERRUPTED_TURN=1 auto-resumes mid-turn in SDK mode. Pending approvals are recovered because the permission mode is restored, but plan and bypassPermissions modes are not restored."
observability:
  stream_events:
    - "system/init"
    - "system/api_retry"
    - "result"
  hook_events:
    - "SessionStart"
    - "SessionEnd"
    - "PreToolUse"
    - "PermissionRequest"
    - "Notification"
  failure_modes:
    - "tool_deferred"
    - "tool_deferred_unavailable"
  notes: "All stream events include session_id. Hooks receive session_id and transcript_path. Agent view / claude agents --json exposes id, sessionId, state, and waitingFor for background sessions."
quirks:
  - "--resume restores the permission mode that was active when the tool was deferred, except plan and bypassPermissions are never carried over."
  - "Resuming the same session in two terminals without forking causes messages from both to interleave into one transcript."
  - "Sessions created with claude -p do not appear in the interactive session picker, but are still resumable by ID."
  - "The JSONL transcript format is internal and changes between releases; scripts should use --output-format json or /export."
  - "Nested interactive sessions started from inside Claude's Bash tool are excluded from --resume / --continue unless CLAUDE_CODE_FORCE_SESSION_PERSISTENCE=1 is set."
  - "Background sessions isolate file edits in .claude/worktrees/; deleting the session removes the worktree and uncommitted changes."
gaps:
  - "Exact concurrency control for simultaneous resumes of the same session is not documented beyond 'messages interleave'."
  - "Retention sweep timing and behavior when a session is actively resumed during cleanup are not documented."
  - "Whether MCP server connection state and OAuth tokens are restored on resume is not explicitly documented."
changes:
  - "2026-07-02: Replaced free-form frontmatter with schema-validated fields."
  - "2026-07-02: Added explicit coverage of --fork-session, --from-pr, --no-session-persistence, and agent-view background sessions."
  - "2026-07-02: Documented defer single-tool-call limitation and permission-mode restore exceptions."
  - "2026-07-02: Added local storage paths and retention policy from settings docs."
requires_claudine_update: true
reason: "The new schema format and the verified defer/permission-mode/background-session semantics should feed into Claudine's lifecycle resume action, session-id capture logic, and HITL broker implementation."
---

## Overview

Claude Code's session resume is a first-class, local-transcript-replay system. Every session writes continuously to a JSONL file under `~/.claude/projects/<encoded-cwd>/`, and the same file is replayed when the session is continued, resumed by ID, or resumed by name. The CLI supports both interactive resumption (`--continue`, `--resume`, `/resume`) and scriptable non-interactive follow-up (`claude -p --resume <session-id>`). For human-in-the-loop automation, `PreToolUse` hooks can defer an `AskUserQuestion` tool call in `-p` mode, exit with `stop_reason: tool_deferred`, and later inject an answer on `--resume`.

## Resume Semantics

A Claude Code "session" is a persisted conversation transcript stored locally on disk. Resume means loading that transcript and appending new turns to it, not reattaching to a remote server or live process. The authoritative state is the JSONL transcript plus the permission mode recorded in it. Sessions can be resumed interactively, non-interactively, from the VS Code extension, or from the JetBrains plugin. The desktop app and Claude Code on the web maintain their own session history and are out of scope for the CLI behavior described here.

## Supported Modes

| Mode | Entry point | Follow-up prompt | Selector |
|------|-------------|------------------|----------|
| Interactive CLI | `claude --continue` | No | Latest in current directory |
| Interactive CLI | `claude --resume` | No | Picker |
| Interactive CLI | `claude --resume <name>` | No | Exact name |
| Interactive CLI | `/resume` | No | Picker |
| Interactive CLI | `/resume <name>` | No | Exact name |
| Non-interactive CLI | `claude -p --continue` | Yes | Latest in current directory |
| Non-interactive CLI | `claude -p --resume <id> [prompt]` | Yes | Exact session ID |

`claude -p` sessions do not appear in the interactive session picker, but they are fully resumable by ID and by `--continue` from the same directory.

## Session ID Capture

Stable session identifiers are UUIDs. Capture surfaces:

- **`--output-format json` / `stream-json`**: `session_id` appears in the `result` object and in every event.
- **Hooks**: every hook input JSON includes `session_id` and `transcript_path`.
- **Environment**: `CLAUDE_CODE_SESSION_ID` is set in Bash, PowerShell, hook, and MCP stdio subprocesses.
- **Transcript file**: `~/.claude/projects/<encoded-cwd>/<session-id>.jsonl`.
- **Agent view**: `claude agents --json` returns `sessionId` for active and recently completed background sessions.
- **Debug logs**: `~/.claude/debug/<session-id>.txt` when debug mode is enabled.

The interactive picker intentionally shows session names and summaries rather than raw IDs.

## Resume Invocation

Continue the latest session:

```bash
claude --continue
claude -p --continue "next step"
```

Resume a specific session by ID:

```bash
claude -p --resume 266af3f6-5f01-4c59-9818-6b4df0462bf3 "summarize what we changed"
```

Resume by name in an interactive session:

```text
/resume auth-refactor
```

Branch while resuming:

```bash
claude --continue --fork-session
claude --resume auth-refactor --fork-session
```

Resume a session linked to a pull request:

```bash
claude --from-pr 123
claude --from-pr https://github.com/org/repo/pull/123
```

## Session Lookup Scope

Sessions are stored per project directory. The default lookup is scoped to the current directory and its git worktrees. The interactive picker can widen with keyboard shortcuts:

- `Ctrl+W`: all worktrees of the current repository.
- `Ctrl+A`: all projects on the machine.
- `Ctrl+B`: filter to the current git branch.

Resuming by name resolves across the current repository and its worktrees. Resuming by ID from another worktree of the same repository resumes the session in place; selecting a session from an unrelated project copies a `cd` + `resume` command to the clipboard.

## State Storage

Resumable state is local, not server-side.

| OS | Path |
|----|------|
| macOS / Linux / WSL | `~/.claude/projects/<encoded-cwd>/<session-id>.jsonl` |
| Windows | `%USERPROFILE%\.claude\projects\<encoded-cwd>\<session-id>.jsonl` |

The `<encoded-cwd>` directory name is the absolute working directory with non-alphanumeric characters replaced by `-`. Each line of the JSONL file is an internal message, tool call, or metadata entry. The format is explicitly internal and may change between releases; direct parsing is unsupported. For stable access, use `--output-format json`, `/export`, or the Agent SDK.

Retention is controlled by `cleanupPeriodDays` in settings (default 30 days, minimum 1). To run without persistence, use `claude -p --no-session-persistence` or set `CLAUDE_CODE_SKIP_PROMPT_HISTORY=1`.

Background sessions from `claude --bg` or agent view also write transcripts to the same path and store supervisor state under `~/.claude/jobs/<short-id>/`.

## Restored State

Resuming restores:

- The full conversation transcript and tool results recorded in the JSONL file.
- The permission mode active when the session ended, except `plan` and `bypassPermissions` are never carried over.
- The working directory and any `/add-dir` directories from the original launch.
- MCP servers, settings, plugins, and fallback model from the original launch.

Resuming does not restore the environment of the original session; it uses the environment of the process that runs the resume command. Model and effort can be overridden with `--model` and `--effort` at resume time.

## Branching and Checkpoints

Branching creates a copy of the conversation and leaves the original intact.

- `/branch [name]` from inside a session.
- `claude --continue --fork-session` or `claude --resume <id> --fork-session` from the CLI.

Permissions approved with "allow for this session" do not carry over to the new branch.

Checkpointing (`/rewind`) reverts file edits and/or conversation to an earlier point within the same session. It is not a separate session. As of v2.1.191, `/rewind` can also resume the conversation that was active before `/clear`.

## Human-in-the-Loop Resume

Claude Code supports a native defer-and-resume loop for non-interactive sessions:

1. Run `claude -p "task that may ask a question"`.
2. When Claude calls `AskUserQuestion`, a `PreToolUse` hook fires with `tool_input.questions`.
3. The hook returns `permissionDecision: "defer"`.
4. The process exits with `stop_reason: "tool_deferred"` and a `deferred_tool_use` payload containing the tool `id`, `name`, and `input`.
5. The wrapper presents the question to the user.
6. The wrapper runs `claude -p --resume <session-id>`.
7. The same `PreToolUse` hook fires again and returns `permissionDecision: "allow"` with `updatedInput.answers` mapping question text to answer label.

Important constraints:

- `defer` is honored only in `claude -p` mode. Interactive sessions log a warning and ignore it.
- It works only when Claude makes a single tool call in the turn. If multiple tools are called, `defer` is ignored with a warning.
- MCP tools marked `_meta["anthropic/requiresUserInteraction"]` cannot be auto-approved by a hook, even with `updatedInput`.

`PermissionRequest` hooks can also allow or deny permission requests programmatically, but `PermissionRequest` does not fire in non-interactive mode; use `PreToolUse` for scripted control.

## Interruption Recovery

Because transcripts are written continuously, sessions survive most interruptions:

- **Crash / terminal close / process kill**: the JSONL file remains and can be resumed.
- **Ctrl+C**: the session is preserved; resume with `--continue` or `--resume`.
- **Pending deferred tool**: preserved in the transcript and re-run on `--resume`.
- **Pending approval / question**: the permission mode is restored on resume, so the same approval state applies.
- **Network failure / API error**: `CLAUDE_CODE_RESUME_INTERRUPTED_TURN=1` auto-resumes mid-turn in SDK mode; otherwise the turn ends and can be continued.

`plan` and `bypassPermissions` modes are never restored on resume.

## Observability

Events and surfaces that expose session identity or resumability:

- **`--output-format json` / `stream-json`**: `system/init`, `system/api_retry`, and `result` events all carry `session_id`.
- **Hooks**: `SessionStart`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `SessionEnd`, and others include `session_id` and `transcript_path`.
- **Agent view**: `claude agents --json` exposes `id`, `sessionId`, `name`, `state`, `waitingFor`, and `cwd`.
- **Notification hook**: `permission_prompt`, `elicitation_dialog`, `agent_needs_input`, and `agent_completed` types.
- **Stop reasons**: `tool_deferred` and `tool_deferred_unavailable` signal deferral state.

## Quirks and Gaps

Quirks:

- Concurrent resumes of the same session without forking cause messages from both terminals to interleave into one transcript.
- `--resume` restores the saved permission mode, but `plan` and `bypassPermissions` are always reset.
- Nested interactive `claude` sessions started from inside Claude's Bash tool are excluded from `--resume` and `--continue` unless `CLAUDE_CODE_FORCE_SESSION_PERSISTENCE=1` is set.
- Background sessions isolate file edits in `.claude/worktrees/`; deleting the session removes that worktree and any uncommitted changes.
- The default auto-generated name (`<dir>-<suffix>`) is not a resume handle; only explicitly set names work with `--resume <name>`.

Gaps:

- Anthropic does not document the exact concurrency guarantees when the same session is resumed from multiple processes.
- Whether MCP OAuth state and connection state survive resume is not explicitly documented.
- The JSONL schema is internal and not versioned for external consumers.

## Claudine Integration Notes

For Claudine's lifecycle `resume` action and future HITL broker:

- Capture `session_id` from the `result` object of `claude -p --output-format json` or from a `SessionStart`/`PreToolUse` hook.
- Use `claude -p --resume <session-id> "<follow-up>"` for non-interactive continuation.
- For human-in-the-loop, install a `PreToolUse` hook that matches `AskUserQuestion` and returns `permissionDecision: "defer"`. On `tool_deferred`, extract `deferred_tool_use.input.questions`, ask the user, then resume with `permissionDecision: "allow"` and `updatedInput.answers`.
- Treat the JSONL transcript as read-only and unstable; do not build parsing logic against it.
- When resuming after a failure, be aware that `plan` and `bypassPermissions` modes are not restored, so re-pass `--permission-mode` if required.
- Background sessions from `claude --bg` can be monitored via `claude agents --json` and attached with `claude attach <id>`, but they are not a substitute for a headless API.

## Changelog

- 2026-07-02: Converted to schema-validated frontmatter.
- 2026-07-02: Added `--fork-session`, `--from-pr`, `--no-session-persistence`, agent-view background sessions, and the `defer` single-tool limitation.
- 2026-07-02: Documented permission-mode restore exceptions and exact local storage paths.

## Sources

- [Claude Code sessions documentation](https://code.claude.com/docs/en/sessions)
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Claude Code hooks reference](https://code.claude.com/docs/en/hooks)
- [Claude Code headless / non-interactive mode](https://code.claude.com/docs/en/headless)
- [Claude Code checkpointing](https://code.claude.com/docs/en/checkpointing)
- [Claude Code agent view](https://code.claude.com/docs/en/agent-view)
- [Claude Code environment variables](https://code.claude.com/docs/en/env-vars)
- [Claude Code settings](https://code.claude.com/docs/en/settings)
