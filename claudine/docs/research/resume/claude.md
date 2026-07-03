---
$schema: ./_schema.yaml
created: 2026-04-02
last_updated: 2026-07-03
agent: codex
model: default
docs: https://code.claude.com/docs/en/sessions
support: first_class
continuity_model: mixed
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "claude --continue"
      - "claude --resume"
      - "claude --resume <session-id-or-name>"
      - "claude --from-pr <number>"
      - "/resume"
      - "interactive session picker"
    accepts_followup_prompt: false
    selection_methods:
      - latest
      - id
      - name
      - picker
      - all_projects
      - worktree
      - branch
      - pr
    notes: "Interactive resume reopens the conversation; the next prompt is typed after the session starts."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "claude -p --continue <prompt>"
      - "claude -p --resume <session-id> <prompt>"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
    notes: "This is the scriptable follow-up surface. Structured output is available with --output-format json or stream-json."
  - mode: headless_server
    supported: true
    mechanisms:
      - "claude remote-control"
      - "claude --remote-control"
      - "/remote-control"
    accepts_followup_prompt: true
    selection_methods:
      - name
      - picker
      - latest
    notes: "Remote Control keeps a local Claude Code process running and lets web/mobile clients steer it through claude.ai/code. This is live-process attach, not transcript replay."
  - mode: ide
    supported: true
    mechanisms:
      - "VS Code extension session history"
      - "JetBrains plugin session history"
    accepts_followup_prompt: false
    selection_methods:
      - picker
      - latest
      - unknown
    notes: "Official CLI session docs state IDEs keep their own session history. Detailed IDE storage and selectors were not locally verified."
  - mode: api
    supported: true
    mechanisms:
      - "Agent SDK query() continue"
      - "Agent SDK query() resume"
      - "Agent SDK query() fork_session"
      - "Agent SDK SessionStore"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
    notes: "The TypeScript and Python Agent SDKs expose continue/resume/fork options and result messages with session_id."
session_id_capture:
  - surface: json_stream
    field: session_id
    format: uuid
    notes: "Print-mode JSON and stream-json include session_id. SDK result messages also expose session_id; TypeScript exposes it earlier on the init SystemMessage."
  - surface: hook
    field: session_id
    format: uuid
    notes: "Hook and status-line JSON inputs include session_id and transcript_path for event-driven capture."
  - surface: session_file
    field: filename
    format: "<session-id>.jsonl"
    notes: "Observed local transcripts under ~/.claude/projects/<encoded-cwd>/<session-id>.jsonl. Each sampled line included sessionId when it belonged to a persisted session."
  - surface: session_file
    field: sessionId
    format: uuid
    notes: "Observed ~/.claude/sessions/<pid>.json live-session metadata with pid, status, kind, entrypoint, cwd, waitingFor, version, and sessionId."
  - surface: cli_command
    field: sessionId
    format: uuid
    notes: "claude agents --json is documented as a running/background-session listing surface; local metadata files corroborate the same sessionId shape."
  - surface: interactive_ui
    field: name
    format: string
    notes: "The picker displays set names, summaries, time, message count, git branch, and project path when widened. Default generated display names are not resume handles."
resume_invocations:
  - mode: interactive
    invocation: "claude --continue"
    accepts_prompt: false
    selection: latest
    notes: "Loads the most recent conversation in the current directory, including sessions that added this directory with /add-dir."
  - mode: interactive
    invocation: "claude --resume"
    accepts_prompt: false
    selection: picker
    notes: "Opens the human-oriented session picker. Keyboard controls widen to all worktrees or all projects and can filter by branch."
  - mode: interactive
    invocation: "claude --resume <session-id-or-name>"
    accepts_prompt: false
    selection: id
    notes: "A session ID search is scoped to the current project directory and its git worktrees. A set name resolves across the current repository and worktrees."
  - mode: interactive
    invocation: "claude --from-pr <number>"
    accepts_prompt: false
    selection: pr
    notes: "Resumes the session linked to the pull request."
  - mode: interactive
    invocation: "/resume [name-or-session-id]"
    accepts_prompt: false
    selection: picker
    notes: "Inside an active session, switches to another conversation. With no argument it opens the picker."
  - mode: non_interactive
    invocation: "claude -p --continue \"follow-up prompt\" --output-format json"
    accepts_prompt: true
    selection: latest
    notes: "Scriptable continue-latest. Use --bare for deterministic scripts when local customizations should be skipped."
  - mode: non_interactive
    invocation: "claude -p --resume <session-id> --output-format json \"follow-up prompt\""
    accepts_prompt: true
    selection: id
    notes: "Scriptable exact-session follow-up. The result can be captured from .result and .session_id."
  - mode: both
    invocation: "claude --continue --fork-session"
    accepts_prompt: false
    selection: latest
    notes: "Forks the latest session into a new session ID while preserving the original."
  - mode: both
    invocation: "claude --resume <session-id-or-name> --fork-session"
    accepts_prompt: false
    selection: id
    notes: "Forks a selected session into a new conversation history."
  - mode: headless_server
    invocation: "claude remote-control"
    accepts_prompt: true
    selection: picker
    notes: "Starts a local server-mode process and displays a session URL or QR code for claude.ai/code or mobile clients."
  - mode: headless_server
    invocation: "claude --remote-control [name]"
    accepts_prompt: true
    selection: name
    notes: "Starts an interactive local session with Remote Control enabled."
  - mode: api
    invocation: "query({ prompt, options: { continue: true } })"
    accepts_prompt: true
    selection: latest
    notes: "SDK continue-latest from the current directory."
  - mode: api
    invocation: "query({ prompt, options: { resume: session_id } })"
    accepts_prompt: true
    selection: id
    notes: "SDK exact-session resume by ID."
  - mode: api
    invocation: "query({ prompt, options: { resume: session_id, fork_session: true } })"
    accepts_prompt: true
    selection: id
    notes: "SDK fork from a prior session into a new session ID."
state_storage:
  - location: local
    os: macos
    path: "~/.claude/projects/<encoded-cwd>/<session-id>.jsonl"
    format: "Internal plaintext JSONL transcript; sidecar data under projects/<project>/<session>/, file-history/<session>/, session-env/, tasks/, shell-snapshots/, and debug/ as features require it."
    retention: "Deleted on startup after cleanupPeriodDays; default 30 days."
    notes: "On this macOS host, ~/.claude is a symlinked tree rooted at /Users/ken/.claudine/.claude. Observed transcripts contain mode, permission-mode, ai-title, last-prompt, assistant, user, system, attachment, queue-operation, file-history-snapshot, and toolUseResult entries."
  - location: local
    os: linux
    path: "~/.claude/projects/<encoded-cwd>/<session-id>.jsonl"
    format: "Internal plaintext JSONL transcript; same documented layout as macOS."
    retention: "Deleted on startup after cleanupPeriodDays; default 30 days."
    notes: "If CLAUDE_CONFIG_DIR is set, the projects directory moves under that directory."
  - location: local
    os: windows
    path: "%USERPROFILE%\\.claude\\projects\\<encoded-cwd>\\<session-id>.jsonl"
    format: "Internal plaintext JSONL transcript; same documented layout with Windows home-directory resolution."
    retention: "Deleted on startup after cleanupPeriodDays; default 30 days."
    notes: "Official docs say ~/.claude resolves to %USERPROFILE%\\.claude on Windows. Path separators differ, but the encoded project-directory rule is the same."
  - location: server
    path: "claude.ai/code"
    format: "Cloud session state for Claude Code on the web and routing state for Remote Control."
    retention: "Unknown."
    notes: "Web sessions run on Anthropic-managed infrastructure. Remote Control sessions run locally but are discoverable and steerable through claude.ai/code."
resume_scope:
  project_scoped: true
  cwd_scoped: true
  worktree_aware: true
  all_projects_supported: true
  branch_filtering: true
  notes: "Session ID lookup is scoped to the current project directory and its git worktrees. The picker starts at the current worktree, can widen to all worktrees with Ctrl+W, widen to all projects with Ctrl+A, and filter by branch with Ctrl+B. Set-name resolution spans the current repository and worktrees."
branching_checkpointing:
  branch_supported: true
  checkpoint_supported: true
  fork_invocation: "/branch [name]; claude --continue --fork-session; claude --resume <id-or-name> --fork-session; SDK resume + fork_session"
  checkpoint_invocation: "/rewind; Esc Esc with empty prompt; Agent SDK rewindFiles(userMessageId)"
  preserves_original: true
  notes: "Fork/branch creates a new session ID and leaves the original unchanged. Rewind operates within a session and can restore code, conversation, or both; checkpoints persist across resumed conversations but track Claude file-edit tools, not arbitrary Bash changes."
restored_state:
  transcript: true
  tool_results: true
  approvals: session_only
  model: overridable
  cwd: restored
  env: current_process
  notes: "Resume appends to the same session ID and transcript. Conversation, tool calls, and tool results are replayed from the transcript. Launch flags and settings can override model, permissions, and customizations. The current process supplies environment variables. Permissions approved for a session do not carry into a fork. The exact restoration of MCP connection state was not verified."
hitl_resume:
  supported: true
  question_capture: "In non-interactive mode, a PreToolUse hook can return permissionDecision defer. SDK/print results expose stop_reason tool_deferred and deferred_tool_use with the pending tool id, name, and input. Hooks and status-line inputs also provide session_id and transcript_path."
  answer_injection: "Resume the same session_id with claude -p --resume <session-id> or SDK query({ options: { resume } }) after the wrapper has collected input. For SDK live sessions, reinitialize() after a transport gap redispatches pending permission requests to canUseTool."
  limitations: "Defer is documented for -p / SDK workflows, not ordinary interactive terminal prompts. Prompt-tool approval cannot approve MCP tools marked as requiring user interaction as of v2.1.199. Wrapper callbacks must be idempotent because requests can be redispatched after reconnect."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: true
  pending_approval_resume: true
  limitations: "Transcripts are written continuously, so completed turns and recorded tool results survive terminal exit, Ctrl+C, crash, and process kill. Recovery of a tool call depends on it having been recorded or deferred before interruption. Simultaneously resuming the same session in two terminals is unsafe for automation because messages interleave into one transcript."
observability:
  stream_events:
    - "system/init"
    - "system/api_retry"
    - "result"
    - "stream-json events with session_id"
    - "SDK ResultMessage.session_id"
  hook_events:
    - "SessionStart"
    - "SessionEnd"
    - "PreToolUse"
    - "PostToolUse"
    - "Notification"
    - "StatusLine"
  failure_modes:
    - "No conversation found with session ID"
    - "tool_deferred"
    - "error_max_turns"
    - "error_max_budget_usd"
    - "api_retry"
  notes: "Observed ~/.claude/sessions/<pid>.json exposes live-session status values such as idle and waiting plus waitingFor text. Official script surfaces are JSON/stream-json, hooks/status-line transcript_path, Agent SDK messages, and claude agents --json for active/background sessions."
quirks:
  - "Sessions created with claude -p or the Agent SDK do not appear in the interactive picker, but remain resumable by session ID from the original project directory/worktree scope."
  - "The internal transcript format is explicitly unstable; local parsing is useful for validation but should not be Claudine's primary integration contract."
  - "A generated default session display name is not a resume handle. Only user-set names are accepted by claude --resume <name> and /resume <name>."
  - "The same session opened in two terminals without --fork-session appends both conversations to one transcript."
  - "--no-session-persistence and CLAUDE_CODE_SKIP_PROMPT_HISTORY disable resumability by suppressing transcript writes."
  - "Checkpoint rewind can restore Claude edit-tool changes, but not filesystem changes made by Bash commands or by unrelated concurrent sessions."
  - "Remote Control is live-process attach through claude.ai/code; it is not equivalent to non-interactive transcript replay and may require user authentication/trusted-device state."
gaps:
  - "No public stable schema exists for ~/.claude/projects transcript lines or ~/.claude/sessions live metadata files."
  - "Exact locking/concurrency behavior for simultaneous non-interactive resumes of one session is not documented beyond transcript interleaving warnings."
  - "IDE session storage, naming, and exact resume selectors were not locally verified."
  - "Remote Control retention, server-side identifiers, and scriptable APIs were not documented enough to treat as automation-safe."
  - "Whether MCP server connection state, OAuth refresh state, and enabled/disabled server toggles are restored on CLI resume was not verified."
  - "Web-session resume has server-side persistence, but the available local CLI handles for continuing an existing web session are limited to documented remote/teleport flows rather than a stable public resume API."
changes:
  - "2026-07-03: Reclassified continuity_model from transcript_replay to mixed because CLI/SDK use local transcript replay, Remote Control is live-process attach, and Claude Code on the web uses server-side cloud sessions."
  - "2026-07-03: Added Remote Control, claude --remote-control, claude remote-control, and /remote-control as resume-relevant live attach surfaces."
  - "2026-07-03: Added Agent SDK continue/resume/fork and SessionStore semantics as first-class API resume surfaces."
  - "2026-07-03: Updated session lookup behavior with current docs: ID lookup is scoped to current project directory and git worktrees; picker/name behavior differs."
  - "2026-07-03: Added observed ~/.claude/sessions/<pid>.json live-session metadata and observed transcript entry types from local ~/.claude inspection."
  - "2026-07-03: Added --no-session-persistence and CLAUDE_CODE_SKIP_PROMPT_HISTORY as resumability suppressors."
requires_claudine_update: true
reason: "Claudine should distinguish transcript replay from Remote Control live attach and SDK resume, capture session_id from JSON/SDK/hooks rather than parsing internal transcripts, and avoid treating picker-only or Remote Control surfaces as automation-safe lifecycle resume targets."
---

# Claude Code Session Resume

## Overview

Claude Code has first-class resume support, but it is no longer a single continuity model. The automation-safe CLI and Agent SDK path is local transcript replay: Claude Code continuously writes a plaintext JSONL transcript under `~/.claude/projects/<encoded-cwd>/`, then `--continue`, `--resume`, or SDK `continue`/`resume` reloads that history and appends to the same conversation. Forking and branching copy the prior transcript into a new session ID.

The main wrapper risk is conflating surfaces. `claude -p --resume <session-id>` is scriptable and can accept a follow-up prompt; `claude --resume` with no argument is a human picker; Remote Control is live-process attach through `claude.ai/code`; and Claude Code on the web is server-side cloud execution. Claudine should treat these as distinct resume modes with different safety and observability properties.

## Resume Semantics

In the CLI, a session is a saved conversation tied to a project directory. Official docs say sessions are saved continuously to local transcript files and that resuming with `claude --continue` or `claude --resume` reopens the same session ID and appends new messages to the existing conversation. Local inspection on this host, running Claude Code `2.1.200`, found transcripts under `~/.claude/projects/<encoded-cwd>/<session-id>.jsonl`. Sampled transcript lines contained fields such as `sessionId`, `uuid`, `parentUuid`, `cwd`, `gitBranch`, `timestamp`, `version`, `type`, `message`, `toolUseResult`, `permissionMode`, `mode`, `aiTitle`, and `lastPrompt`.

The resume patterns that apply are:

| Pattern | Supported | Continuity model |
|---|---:|---|
| Continue latest | Yes | Local transcript replay for CLI/SDK |
| Resume by handle | Yes | Session ID, set session name, PR handle, SDK session ID |
| Interactive picker | Yes | Human TUI over local session history |
| Non-interactive follow-up | Yes | `claude -p --resume <session-id> <prompt>` |
| Transcript replay | Yes | Primary CLI/SDK model |
| Server-side session | Yes | Claude Code on the web cloud sessions |
| Live-process attach | Yes | Remote Control |
| Branch/fork/rewind/checkpoint | Yes | Fork creates a new session ID; rewind changes same-session code/conversation state |
| Recovery resume | Partial | Continuous transcripts and deferred tools support recovery, but interrupted in-flight tool calls are only recoverable if recorded/deferred |
| Human-in-the-loop resume | Yes for `-p`/SDK defer | Hook/SDK capture plus resume by `session_id` |

A chat-history export is not resume by itself. `/export` creates a human-readable transcript; it does not provide a documented way to continue from the exported text. `CLAUDE.md`, auto memory, skills, rules, and settings are context sources that can affect a future run, but they are not prior-session continuation mechanisms.

## Supported Modes

| Mode | Surface | Selector | Follow-up prompt at invocation | Automation fit |
|---|---|---|---:|---|
| Interactive CLI | `claude --continue` | Latest in current directory | No | Low |
| Interactive CLI | `claude --resume` | Picker | No | Picker-only |
| Interactive CLI | `claude --resume <session-id-or-name>` | ID or set name | No | Moderate for ID/name, still interactive |
| Interactive CLI | `claude --from-pr <number>` | Pull request | No | Moderate, PR-specific |
| Interactive slash | `/resume [name-or-id]` | Picker, name, or ID | No | Human-oriented |
| Non-interactive CLI | `claude -p --continue "prompt"` | Latest in current directory | Yes | High |
| Non-interactive CLI | `claude -p --resume <session-id> "prompt"` | Exact ID | Yes | High |
| Agent SDK | `continue: true`, `resume: session_id` | Latest or exact ID | Yes | High |
| Agent SDK | `resume: session_id`, `fork_session: true` | Exact ID | Yes | High |
| Remote Control | `claude remote-control`, `claude --remote-control`, `/remote-control` | Session URL, QR code, web/mobile list | Yes from remote client | Human/live attach |
| IDE | VS Code and JetBrains histories | Picker/latest, details unknown | No verified CLI prompt injection | Human-oriented |
| Web | `claude --remote`, web UI, `/teleport` flows | Cloud/session UI | Yes from web UI | Server-side, not local transcript replay |

Official session docs state that `claude -p` and Agent SDK sessions do not appear in the interactive session picker, but can still be resumed by passing the session ID to `claude --resume <session-id>` from the directory where the session started. That means non-interactive sessions are resumable, but wrappers must capture the ID because the picker is not a reliable lookup surface for them.

## Session ID Capture

The stable handle is a UUID-shaped session ID. Scriptable capture surfaces are stronger than local file discovery:

- `claude -p --output-format json` returns structured JSON with a `session_id`.
- `claude -p --output-format stream-json --verbose` emits stream events that include the session ID.
- Agent SDK result messages expose `session_id` on success and error results. The TypeScript SDK also exposes the ID earlier on the init `SystemMessage`; Python nests it under `SystemMessage.data`.
- Hook and status-line inputs include `session_id` and `transcript_path`, which can be archived on `SessionEnd`.
- Local transcript filenames are `<session-id>.jsonl` under the encoded project directory.
- Local live-session metadata files were observed under `~/.claude/sessions/<pid>.json` with `sessionId`, `pid`, `status`, `kind`, `entrypoint`, `cwd`, `waitingFor`, `startedAt`, `updatedAt`, and `version`.
- `claude agents --json` is documented as exposing running/background-session identifiers.

The ID is available early enough for a wrapper in stream/SDK mode. For plain text `claude -p`, Claudine should prefer `--output-format json` or `stream-json`; otherwise it may need a hook or transcript-path side channel.

## Resume Invocation

Continue the latest local session:

```bash
claude --continue
claude -p --continue --output-format json "summarize the last turn"
```

Resume an exact local session:

```bash
claude --resume 266af3f6-5f01-4c59-9818-6b4df0462bf3
claude -p --resume 266af3f6-5f01-4c59-9818-6b4df0462bf3 --output-format json "continue with the next step"
```

Resume by set name or PR in interactive CLI:

```bash
claude -n auth-refactor
claude --resume auth-refactor
claude --from-pr 123
```

Branch or fork:

```bash
claude --continue --fork-session
claude --resume 266af3f6-5f01-4c59-9818-6b4df0462bf3 --fork-session
```

Inside an active session:

```text
/resume
/resume auth-refactor
/branch try-streaming-approach
/rewind
```

Agent SDK equivalents:

```ts
query({ prompt: "next step", options: { continue: true } });
query({ prompt: "next step", options: { resume: sessionId } });
query({ prompt: "alternate path", options: { resume: sessionId, forkSession: true } });
```

Remote Control attach:

```bash
claude remote-control
claude --remote-control "My Project"
```

For structured follow-up capture, the documented CLI form is:

```bash
claude -p --resume <session-id> --output-format json "summarize what we changed" | jq -r '.result'
```

## Session Lookup Scope

Session lookup is project-scoped and worktree-aware. The default picker shows interactive sessions from the current worktree plus sessions that added the current directory with `/add-dir`. `Ctrl+W` widens to all worktrees in the repository; `Ctrl+A` widens to all projects on the machine; `Ctrl+B` filters by the current git branch. Selecting a session from another worktree of the same repository resumes it in place. Selecting a session from an unrelated project copies a `cd` plus resume command to the clipboard rather than directly re-entering it.

Session ID lookup is stricter than picker browsing: official docs say a session created elsewhere reports `No conversation found with session ID: <session-id>` unless the resume is launched from the directory where the session started, or from the current project directory and its git worktrees. Resuming by set name resolves across the current repository and worktrees. If a session moves via `/cd`, recent Claude Code versions relocate it to the new directory's project storage.

## State Storage

By default, local CLI transcripts live at:

```text
macOS/Linux: ~/.claude/projects/<encoded-cwd>/<session-id>.jsonl
Windows:     %USERPROFILE%\.claude\projects\<encoded-cwd>\<session-id>.jsonl
```

`<encoded-cwd>` is the absolute working directory with non-alphanumeric characters replaced by `-`. Setting `CLAUDE_CONFIG_DIR` moves this storage under that directory. Official docs explicitly call the transcript entry format internal and unstable. Scripts should use JSON output, hooks, status-line input, SDK messages, or `/export` rather than relying on direct transcript parsing.

Local inspection on this macOS host found:

| Path | Observed purpose |
|---|---|
| `~/.claude/projects/<encoded-cwd>/<session-id>.jsonl` | Full transcript and metadata entries |
| `~/.claude/sessions/<pid>.json` | Live interactive session metadata, including `sessionId`, `status`, `cwd`, and `waitingFor` |
| `~/.claude/history.jsonl` | Prompt history with `display`, `pastedContents`, `project`, and `timestamp` |
| `~/.claude/file-history/<session>/` | Pre-edit file snapshots for checkpoint restore |
| `~/.claude/session-env/` | Per-session environment metadata |
| `~/.claude/shell-snapshots/` | Captured shell environments for Bash tool use |
| `~/.claude/tasks/` | Per-session task-list state |
| `~/.claude/debug/` | Per-session debug logs when debug mode is enabled |

The application data docs say transcripts, subagent transcripts, tool-result spill files, file history, plans, debug logs, paste/image caches, session-env, tasks, and shell snapshots are cleaned up on startup after `cleanupPeriodDays`; the default is 30 days. `history.jsonl` and `stats-cache.json` persist until deleted. Transcripts and prompt history are plaintext and not encrypted at rest.

For Claude Code on the web, session state lives in Anthropic-managed cloud infrastructure. Remote Control is different: the agent process and tools run on the user's machine, but web/mobile clients discover and steer the session through `claude.ai/code`.

## Restored State

Resume restores the conversation transcript, tool calls, and tool results recorded in the local JSONL history. Official SDK docs define a session as containing the prompt, every tool call, every tool result, and every response. Resume gives the agent the prior context: files it already read, analysis it already performed, and decisions it already made.

State that survives or is recalculated:

| State | Behavior |
|---|---|
| Conversation transcript | Restored and appended to the same session ID |
| Tool results | Restored when recorded in the transcript; large outputs may be sidecar files |
| Checkpoints | Persist across sessions and are cleaned up with the session |
| Model | Can be overridden at launch with `--model` or SDK options; settings precedence still applies |
| Permission mode | Can be overridden at launch; session-scoped approvals do not carry into forks |
| Working directory | Session lookup is cwd/project-scoped; picker/name rules may resume across worktrees |
| Environment variables | Current process environment and current settings are used |
| Hooks, skills, MCP, plugins, memory | Loaded from current settings unless skipped by flags such as `--bare` or `--safe-mode` |
| Attachments/pastes/images | Stored in transcript or cache paths when present; exact replay behavior was not fully verified |
| Pending tool calls | Recoverable when represented as deferred tool state or recorded transcript state |
| Pending approvals | Recoverable in SDK/`-p` defer workflows; ordinary terminal approval prompts are not a stable automation surface |

Resume keeps writing to the same transcript. Forking creates a new session ID and transcript branch. Rewind operates within a session and can restore code, conversation, or both from checkpoints.

## Branching and Checkpoints

Claude Code supports naming, renaming, listing, previewing, searching, branching, and checkpoint rewind. Names are set with `claude -n <name>` at startup, `/rename <name>` during a session, or `Ctrl+R` in the picker. Only set names are resume handles; generated default display names are not accepted by `claude --resume <name>`.

Branching creates a copy of the conversation so far and switches into the copy. `/branch [name]`, `claude --continue --fork-session`, `claude --resume <id-or-name> --fork-session`, and SDK `fork_session` preserve the original session and create a new session ID. The `/branch` confirmation prints the new and original IDs.

Checkpointing is same-session recovery, not a separate resume handle. `/rewind` or double `Esc` opens a menu to restore code and conversation, restore only conversation, restore only code, or summarize part of the conversation. Checkpoints persist across resumed conversations and are cleaned up with the session. They track file edits made through Claude's edit tools; Bash-made changes, external manual changes, and unrelated concurrent-session changes are not reliably reversible.

## Human-in-the-Loop Resume

Claude Code supports a usable human-in-the-loop resume loop for non-interactive and SDK callers. A `PreToolUse` hook can return `permissionDecision: "defer"` in `-p` mode. The result exposes `stop_reason: "tool_deferred"` and `deferred_tool_use` with the pending tool's `id`, `name`, and `input`. A wrapper can surface that request to a user elsewhere, then resume the same session ID after collecting the answer.

For SDK live clients, `canUseTool` and user-input callbacks handle approvals in-loop. The TypeScript SDK's `reinitialize()` is relevant after a transport gap: it re-sends initialize to a running CLI and redispatches pending permission requests to `canUseTool`; callbacks must be idempotent per request ID because a request whose response was lost can be delivered again.

Limitations:

- Defer is for `-p`/SDK-style workflows, not a general way to answer arbitrary interactive terminal prompts later.
- A prompt tool cannot approve an MCP tool marked as requiring user interaction as of Claude Code `2.1.199`; an allow result is converted to deny.
- Claudine should treat `deferred_tool_use` and hook payloads as the capture surface, not scrape terminal text.

## Interruption Recovery

Completed transcript state survives normal exits, `/clear`, Ctrl+C after a turn, terminal close, process kill after transcript writes, and crashes after transcript writes. On the next launch, `--continue`, `--resume`, or SDK resume can replay the saved history. Because transcript writes are continuous, the recovery boundary is the last persisted line, not a provider-side live conversation state.

Tool-call recovery depends on timing. Recorded tool results are replayable. Deferred tools are explicitly resumable. An in-flight Bash command or approval prompt interrupted before Claude Code writes a durable result may need to be retried or reconstructed by the model. `claude -p` also terminates background Bash tasks about five seconds after the final result, while background subagents/workflows are awaited with a documented ceiling.

Concurrent resume is unsafe for wrappers. Official docs warn that resuming the same session in two terminals without forking causes messages from both terminals to interleave into one transcript. Claudine should serialize lifecycle `resume` attempts for a session ID or fork before parallel continuation.

Remote Control handles a different failure class: if the laptop sleeps or the network drops, remote clients reconnect when the machine comes back online. That only applies while the local Remote Control process exists; it is not a substitute for `claude -p --resume` in automation.

## Observability

Resume-relevant observability surfaces:

| Surface | What it reveals |
|---|---|
| `claude -p --output-format json` | Final result, `session_id`, usage/cost metadata |
| `claude -p --output-format stream-json --verbose` | Event stream with session identity and retry events |
| Hooks | `session_id`, `transcript_path`, tool inputs, permission decisions, session lifecycle |
| Status line | JSON input with session and transcript context |
| Agent SDK | `SystemMessage`, `ResultMessage.session_id`, deferred tool state, streaming messages |
| `~/.claude/projects/.../*.jsonl` | Internal transcript and metadata, useful for validation only |
| `~/.claude/sessions/<pid>.json` | Local live-session status, observed values including `idle` and `waiting` |
| `claude agents --json` | Active/background session IDs and state |
| `--debug` / `~/.claude/debug/` | Per-session debug logs when enabled |

For Claudine, the preferred session-ID capture order is JSON/SDK stream, hook/status-line payload, explicit CLI result, then local file observation as a last-resort diagnostic.

## Quirks and Gaps

Quirks:

- `claude -p` and Agent SDK sessions are resumable by ID but excluded from the interactive picker.
- Session ID lookup and name lookup do not have identical scope rules.
- Picker selection is not an automation interface, especially after widening to unrelated projects where it may copy a command to the clipboard.
- `--no-session-persistence`, `persistSession: false` in TypeScript SDK, and `CLAUDE_CODE_SKIP_PROMPT_HISTORY` deliberately make sessions non-resumable.
- `--bare` is recommended for scripts and may become the default for `-p`; it skips auto-discovery of hooks, skills, plugins, MCP servers, auto memory, and `CLAUDE.md`.
- Direct transcript parsing can break on any Claude Code release.
- Remote Control requires Claude.ai authentication and may require organization-level enablement or trusted-device checks.

Gaps:

- The local JSONL transcript schema and `~/.claude/sessions/<pid>.json` schema are not documented as stable.
- IDE session resume behavior was not locally inspected.
- Remote Control does not expose a documented stable command/API for scriptable injection into an arbitrary existing remote-control session.
- Claude Code on the web has cloud sessions and teleport/remote workflows, but no documented local CLI equivalent of `claude -p --resume <cloud-session-id>`.
- Exact MCP connection/toggle/OAuth restoration after local transcript resume is unknown.
- Cleanup race behavior for an actively resumed session during a retention sweep is unknown.

## Claudine Integration Notes

For lifecycle `resume`, Claudine should target `claude -p --resume <session-id> --output-format json <prompt>` or the Agent SDK `resume` option when it needs a scriptable follow-up. It should capture `session_id` from JSON/stream-json/SDK/hook payloads at initial launch and persist that handle in Claudine's own run metadata. It should not rely on picker state, generated display names, or parsing internal transcripts as the primary integration contract.

For `retry`, if the original run produced no durable session ID or used `--no-session-persistence` / `CLAUDE_CODE_SKIP_PROMPT_HISTORY`, retry must start a fresh session or report that provider-level resume is unavailable. If a session ID exists, retry can either resume with a corrective prompt or fork first when preserving the failed transcript matters.

For `proxy`, Remote Control and Claude Code on the web are relevant human steering surfaces, but they are not equivalent to local non-interactive resume. A proxy action can direct a human to a Remote Control/web session, while automation should stay on `-p --resume` or SDK resume.

For future human-in-the-loop continuation, Claudine should build around `tool_deferred` / `deferred_tool_use` and SDK permission callbacks. Store the pending session ID, deferred tool ID, tool name, input, transcript path, and request ID if present. After the user answers out-of-band, resume the same session exactly once, with per-session locking to avoid transcript interleaving.

## Changelog

- 2026-07-03: Reclassified Claude Code from pure transcript replay to mixed continuity because Remote Control and web sessions add live-process/server-side continuation surfaces.
- 2026-07-03: Added Agent SDK `continue`, `resume`, `fork_session`, session ID capture, and cross-host storage notes.
- 2026-07-03: Added Remote Control as a live-process attach mode and separated it from automation-safe `claude -p --resume`.
- 2026-07-03: Updated lookup scope from current-directory shorthand to documented project/worktree/name/picker rules.
- 2026-07-03: Added local evidence from `~/.claude/projects`, `~/.claude/sessions`, `~/.claude/history.jsonl`, and Claude Code `2.1.200`.
- 2026-07-03: Added resumability suppressors: `--no-session-persistence`, `persistSession: false`, and `CLAUDE_CODE_SKIP_PROMPT_HISTORY`.

## Sources

- [Claude Code: Manage sessions](https://code.claude.com/docs/en/sessions)
- [Claude Code: How Claude Code works](https://code.claude.com/docs/en/how-claude-code-works)
- [Claude Code: Agent SDK work with sessions](https://code.claude.com/docs/en/agent-sdk/sessions)
- [Claude Code: Run Claude Code programmatically](https://code.claude.com/docs/en/headless)
- [Claude Code: CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Claude Code: Checkpointing](https://code.claude.com/docs/en/checkpointing)
- [Claude Code: Hooks reference](https://code.claude.com/docs/en/hooks)
- [Claude Code: Hooks guide](https://code.claude.com/docs/en/hooks-guide)
- [Claude Code: TypeScript SDK reference](https://code.claude.com/docs/en/agent-sdk/typescript)
- [Claude Code: Remote Control](https://code.claude.com/docs/en/remote-control)
- [Claude Code on the web](https://code.claude.com/docs/en/claude-code-on-the-web)
- [Claude Code: Explore the .claude directory](https://code.claude.com/docs/en/claude-directory)
- Local inspection of Claude Code `2.1.200` on macOS, including `~/.claude/projects`, `~/.claude/sessions`, `~/.claude/history.jsonl`, and `~/.claude/settings.json`.
