---
$schema: ./_schema.yaml
created: 2026-07-08
last_updated: 2026-07-08
agent: codex
model: default
docs: https://antigravity.google/docs/cli/conversations
support: partial
continuity_model: mixed
resume_modes:
  - mode: interactive
    supported: true
    mechanisms:
      - "/resume conversation picker"
      - "/switch alias"
      - "/conversation alias"
      - "agy --continue"
      - "agy --conversation <ID>"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
      - picker
    notes: "The TUI has a human-oriented /resume picker. CLI launch flags can continue the latest conversation or resume a specific conversation ID; --prompt-interactive can provide an initial prompt and then keep the TUI open."
  - mode: non_interactive
    supported: true
    mechanisms:
      - "agy --continue --print <PROMPT>"
      - "agy --conversation <ID> --print <PROMPT>"
      - "agy --conversation <ID> --prompt <PROMPT>"
    accepts_followup_prompt: true
    selection_methods:
      - latest
      - id
    notes: "Print mode can send a follow-up prompt into latest or explicit-ID conversations. Local authentication failure prevented proving a successful resumed turn in this run, but installed help, prior print-mode logs, and changelog entries all expose the surface."
  - mode: headless_server
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "No public local server API for resuming CLI conversations was found."
  - mode: ide
    supported: true
    mechanisms:
      - "CLI session export to Antigravity 2.0 GUI"
      - "/resume picker CLI and IDE tabs"
    accepts_followup_prompt: false
    selection_methods:
      - picker
      - id
    notes: "The README says terminal sessions can be exported to Antigravity 2.0 GUI. Official snippets describe /resume tabs for CLI local conversations and IDE conversations, but the exact export/resume invocation was not verified."
  - mode: api
    supported: false
    mechanisms: []
    accepts_followup_prompt: false
    selection_methods: []
    notes: "No documented SDK or HTTP resume API for CLI sessions was found."
session_id_capture:
  - surface: stdout
    field: conversation_id
    format: uuid string inside --output-format json response
    notes: "The hidden --output-format json shape includes conversation_id, status, response, error, duration_seconds, num_turns, and usage. Local auth-failure probes returned conversation_id as an empty string, so successful ID emission was not re-verified in this run."
  - surface: log_file
    field: conversationID / conversation
    format: uuid
    notes: "Observed in ~/.gemini/antigravity-cli/log/cli-20260708_114540.log: printmode started with conversationID=\"\", created conversation 754ece5a-b744-473e-8c81-0c0e220fd55a, then sent the message to that conversation."
  - surface: session_file
    field: filename
    format: "<conversation-id>.db"
    notes: "Observed SQLite conversation files under ~/.gemini/antigravity-cli/conversations/ with UUID filenames."
  - surface: session_file
    field: conversation_summaries.conversation_id
    format: uuid
    notes: "Observed schema in ~/.gemini/antigravity-cli/conversation_summaries.db. The table was empty on this host, so it is a cache/index surface rather than guaranteed authoritative storage."
  - surface: session_file
    field: cache.last_conversations
    format: "workspace path -> conversation UUID"
    notes: "Observed ~/.gemini/antigravity-cli/cache/last_conversations.json mapping workspace directories to latest conversation IDs."
  - surface: session_file
    field: history.jsonl.conversationId
    format: uuid
    notes: "Observed history rows for slash commands with conversationId and workspace."
  - surface: hook
    field: conversationId + transcriptPath
    format: uuid + path
    notes: "Installed agy-customizations hook docs define common hook fields including conversationId, workspacePaths, transcriptPath, artifactDirectoryPath, and modelName."
  - surface: interactive_ui
    field: "Resume: agy --conversation=<id>"
    format: uuid
    notes: "A public tutorial reports the TUI exit footer prints a resume command with the conversation ID. Local changelog 1.0.13 says a redundant same-project resume hint was removed, leaving the standard resume command."
resume_invocations:
  - mode: interactive
    invocation: "/resume"
    accepts_prompt: false
    selection: picker
    notes: "Opens the conversation picker overlay in the TUI."
  - mode: interactive
    invocation: "/switch"
    accepts_prompt: false
    selection: picker
    notes: "Official CLI reference snippets list /switch as an alias for /resume."
  - mode: interactive
    invocation: "/conversation"
    accepts_prompt: false
    selection: picker
    notes: "Official CLI reference snippets list /conversation as an alias for /resume."
  - mode: interactive
    invocation: "agy --continue"
    accepts_prompt: false
    selection: latest
    notes: "Launches the TUI continuing the most recent conversation."
  - mode: interactive
    invocation: "agy --conversation <CONVERSATION_ID>"
    accepts_prompt: false
    selection: id
    notes: "Launches the TUI against a specific previous conversation."
  - mode: interactive
    invocation: "agy --conversation <CONVERSATION_ID> --prompt-interactive \"continue\""
    accepts_prompt: true
    selection: id
    notes: "Supplies an initial prompt and then keeps the session interactive."
  - mode: non_interactive
    invocation: "agy --continue --print \"continue\""
    accepts_prompt: true
    selection: latest
    notes: "Scriptable follow-up into the most recent conversation."
  - mode: non_interactive
    invocation: "agy --conversation <CONVERSATION_ID> --print \"continue\""
    accepts_prompt: true
    selection: id
    notes: "Scriptable follow-up into a specific conversation. A 1.0.9 changelog entry fixed this path so print mode only emits the newly generated response rather than dumping the whole history."
  - mode: non_interactive
    invocation: "agy --conversation <CONVERSATION_ID> --print \"continue\" --output-format json"
    accepts_prompt: true
    selection: id
    notes: "Structured output is accepted by the installed 1.1.0 binary, but hidden from agy --help. Auth-failure probes returned a JSON error object with empty conversation_id."
state_storage:
  - location: local
    os: macos
    path: "/Users/<user>/.gemini/antigravity-cli/conversations/<conversation-id>.db"
    format: SQLite
    retention: "Not documented."
    notes: "Observed on this macOS host. Tables include trajectory_meta, steps, gen_metadata, executor_metadata, parent_references, trajectory_metadata_blob, and battle_mode_infos."
  - location: local
    os: linux
    path: "/home/<user>/.gemini/antigravity-cli/conversations/<conversation-id>.db"
    format: SQLite
    retention: "Not documented."
    notes: "Expected same home-relative Antigravity CLI data directory as macOS. The public changelog says SQLite is the CLI conversation format."
  - location: local
    os: windows
    path: "%USERPROFILE%\\.gemini\\antigravity-cli\\conversations\\<conversation-id>.db"
    format: SQLite
    retention: "Not documented."
    notes: "Expected Windows user-profile equivalent. Changelog entries discuss Windows print-mode output and PowerShell defaults, but did not document a different conversation directory."
  - location: local
    os: macos
    path: "/Users/<user>/.gemini/antigravity-cli/brain/<conversation-id>/.system_generated/logs/transcript.jsonl"
    format: JSONL transcript mirror
    retention: "Not documented."
    notes: "Observed transcript.jsonl and transcript_full.jsonl for two CLI conversations. These are useful diagnostics but the resumable database appears to be the SQLite conversation file."
  - location: local
    os: linux
    path: "/home/<user>/.gemini/antigravity-cli/brain/<conversation-id>/.system_generated/logs/transcript.jsonl"
    format: JSONL transcript mirror
    retention: "Not documented."
    notes: "Expected same home-relative layout as macOS."
  - location: local
    os: windows
    path: "%USERPROFILE%\\.gemini\\antigravity-cli\\brain\\<conversation-id>\\.system_generated\\logs\\transcript.jsonl"
    format: JSONL transcript mirror
    retention: "Not documented."
    notes: "Expected Windows user-profile equivalent."
resume_scope:
  project_scoped: true
  cwd_scoped: true
  worktree_aware: true
  all_projects_supported: true
  branch_filtering: false
  notes: "Observed cache/last_conversations.json maps absolute workspace paths to latest conversation IDs. cache/projects.json maps workspace paths to project IDs. Official snippets describe /resume tabs for CLI local and IDE conversations, and --continue is described as most recent conversation, not branch-scoped."
branching_checkpointing:
  branch_supported: false
  checkpoint_supported: true
  fork_invocation: ""
  checkpoint_invocation: "automatic CHECKPOINT transcript steps; /rewind string exists in installed binary and changelog"
  preserves_original: false
  notes: "No fork/branch command was verified. Local transcripts include SYSTEM CHECKPOINT records summarizing truncated context, and changelog entries mention /rewind behavior. Rewind appears to alter the current conversation rather than preserve an original branch."
restored_state:
  transcript: true
  tool_results: true
  approvals: unknown
  model: overridable
  cwd: current_launch_dir
  env: current_process
  notes: "Resume is backed by local SQLite trajectory state plus JSONL transcript mirrors and the shared Antigravity backend. Conversation DB steps store step_payload, metadata, permissions, render_info, task_details, and error_details blobs. Launch flags/settings still choose model, mode, sandbox, permissions, workspace dirs, and project selection unless the provider restores more internally; that was not proven."
hitl_resume:
  supported: false
  question_capture: "Hooks receive conversationId and transcriptPath, and PreToolUse hooks can return ask or force_ask decisions."
  answer_injection: "No deferred-answer API was found. A wrapper can only resume the conversation later with agy --conversation <ID> --print <ANSWER> or the TUI equivalent."
  limitations: "This creates a new user turn; it does not answer a still-pending approval prompt or suspended tool call. Pending permission prompts are not known to be durable."
interruption_recovery:
  crash_resume: true
  ctrl_c_resume: true
  pending_tool_resume: false
  pending_approval_resume: false
  limitations: "Local SQLite and transcript files are written during runs, and the TUI supports --continue/--conversation after exit. Changelog 1.0.11 says Ctrl+C interrupts active operations and double-press exits. Pending approvals and in-flight tool calls were not verified as resumable; concurrent resume behavior is undocumented."
observability:
  stream_events: []
  hook_events:
    - PreToolUse
    - PostToolUse
    - UserPromptSubmit
    - Stop
    - SessionStart
  failure_modes:
    - authentication failed or timed out
    - missing or invalid conversation ID
    - SQLite cache/index empty
    - provider network failure
    - pending permission timeout
  notes: "Useful surfaces are --output-format json in print mode, CLI logs under ~/.gemini/antigravity-cli/log, hook payloads, cache/last_conversations.json, history.jsonl, conversation SQLite DBs, and brain transcript JSONL files."
quirks:
  - "The required ~/.antigravity inspection found installed binaries and extensions, but no conversation/session files. CLI conversation state lives under ~/.gemini/antigravity-cli on this host."
  - "The app and CLI share a backend family but use separate app data dirs: ~/.gemini/antigravity for Antigravity 2.0 and ~/.gemini/antigravity-cli for the CLI."
  - "Official docs are an Angular app; direct curl of docs pages mostly returns the shell, so installed help, local built-in guide files, README, changelog, and local storage inspection were more useful."
  - "The public README says terminal sessions can be exported to the Antigravity 2.0 GUI, but the exact CLI invocation was not found."
  - "The installed 1.1.0 help lists --conversation and --continue but does not show value metavariables; --conversation is value-taking despite that help formatting."
  - "The hidden --output-format json flag is accepted but omitted from agy --help."
  - "conversation_summaries.db existed with the expected schema but zero rows on this host; do not rely on it as the only lookup source."
  - "Direct parsing of SQLite blobs and transcript JSONL is unsupported and should be diagnostic only."
gaps:
  - "Whether successful --output-format json always emits a non-empty conversation_id before or after a resumed print-mode turn."
  - "Exact behavior for invalid but well-formed conversation IDs could not be isolated because auth failed before resume validation."
  - "Whether sessions created by non-interactive print mode are always selectable in /resume across projects."
  - "Whether approval decisions, pending approval prompts, pending tool calls, and background tasks survive resume."
  - "Whether concurrent agy --conversation <ID> resumes are locked, rejected, or can interleave writes."
  - "Exact export path from CLI conversations to Antigravity 2.0 GUI."
  - "Whether retention or cleanup policies remove old conversation DBs."
changes: []
requires_claudine_update: true
reason: "Claudine will need an Antigravity provider profile that treats resume as partial: use --conversation for explicit handles, --continue only for human latest-session flows, inspect ~/.gemini/antigravity-cli rather than ~/.antigravity for local state, and avoid relying on undocumented SQLite/blob parsing for automation."
---

# Antigravity CLI Resume Research

## Overview

Antigravity CLI has partial resume support. The user-facing surfaces are real: the TUI has a `/resume` conversation picker, `agy --continue` continues the most recent conversation, and `agy --conversation <ID>` targets a specific prior conversation. Print mode can combine those selectors with `--print` or `--prompt`, so the intended automation shape is `agy --conversation <ID> --print "follow-up"`.

The practical wrapper risk is not the lack of a resume command; it is identity capture and storage stability. On this host the installed 1.1.0 CLI stores conversations under `~/.gemini/antigravity-cli`, not under `~/.antigravity`, and the durable conversation body is SQLite plus protobuf-like blobs with JSONL transcript mirrors. The supported integration point should be the CLI flag surface, not direct database parsing. Claudine should treat `--continue` as unsafe for parallel automation because it selects the latest conversation, while `--conversation <ID>` is the only explicit-handle path.

## Resume Semantics

An Antigravity CLI session is a conversation identified by a UUID. Local inspection found one SQLite database per conversation at `~/.gemini/antigravity-cli/conversations/<conversation-id>.db`, plus per-conversation brain directories at `~/.gemini/antigravity-cli/brain/<conversation-id>/`. The SQLite schema includes `trajectory_meta`, `steps`, `gen_metadata`, `executor_metadata`, `parent_references`, `trajectory_metadata_blob`, and `battle_mode_infos`. The `steps` table stores indexed records with step type, status, metadata, permissions, task details, render info, step payload, and error detail blobs.

Resume appears to be mixed local transcript/state replay plus the shared Antigravity backend. It is not live-process attach: a later `agy` process uses stored conversation identity and local state to reopen or continue a conversation. It is also not a public hosted session API: no documented HTTP or SDK resume endpoint for CLI conversations was found. The applicable patterns are continue latest, resume by conversation ID, interactive picker, non-interactive follow-up, transcript replay/state reload, checkpoint-style compaction, IDE handoff/export, and interruption recovery.

Chat-history exports are not resume unless Antigravity can continue from them. Memory, settings, skills, project rules, and MCP configuration are context sources, not prior-session continuation mechanisms. The local `transcript.jsonl` and `transcript_full.jsonl` files are evidence of what the conversation contained, but direct transcript parsing should not be treated as the supported resume API.

## Supported Modes

| Mode | Surface | Selector | Follow-up prompt | Automation fit |
| --- | --- | --- | --- | --- |
| Interactive TUI | `/resume`, `/switch`, `/conversation` | Picker | No prompt at selection time | Human-oriented |
| Interactive launch | `agy --continue` | Latest | Optional with `--prompt-interactive` | Weak, latest can collide |
| Interactive launch | `agy --conversation <ID>` | Conversation ID | Optional with `--prompt-interactive` | Good for exact re-entry |
| Non-interactive print | `agy --continue --print <PROMPT>` | Latest | Yes | Unsafe for parallel wrappers |
| Non-interactive print | `agy --conversation <ID> --print <PROMPT>` | Conversation ID | Yes | Best CLI automation surface |
| IDE | CLI export / picker tabs | Picker or ID | Unknown | Human-oriented; incomplete evidence |
| API/headless server | None found | None | No | Unsupported |

The installed `agy --help` shows `--continue`, `-c`, `--conversation`, `--prompt-interactive`, `--print`, `--prompt`, and `--print-timeout`. The help formatting does not show a metavariable for `--conversation`, but the description says it resumes a previous conversation by ID. The top-level help also omits the accepted `--output-format json` flag.

The public changelog matters for non-interactive resume. Version 1.0.9 fixed print-mode resumption with `--conversation`/`-c -p` so the CLI prints only the newly generated response instead of the entire historical conversation. That indicates non-interactive follow-up into an existing conversation is a supported behavior, not just an accidental flag combination.

Sessions created in print mode are persisted locally. A local print-mode log from `2026-07-08 11:45:41` shows print mode starting with an empty `conversationID`, silently authenticating, creating conversation `754ece5a-b744-473e-8c81-0c0e220fd55a`, and sending the user message to that conversation. The matching SQLite conversation file and brain transcript directory exist on disk.

## Session ID Capture

The best explicit handle is the conversation UUID. Local evidence found the ID in several places:

| Surface | Field | Evidence |
| --- | --- | --- |
| CLI logs | `conversationID` / `conversation` | `~/.gemini/antigravity-cli/log/cli-20260708_114540.log` records creation and message sending for `754ece5a-b744-473e-8c81-0c0e220fd55a`. |
| Conversation file | Filename | `~/.gemini/antigravity-cli/conversations/<uuid>.db`. |
| Latest cache | JSON value | `cache/last_conversations.json` maps workspace paths to latest conversation IDs. |
| Prompt history | `conversationId` | `history.jsonl` records slash command history rows with workspace and conversation ID. |
| Hook payloads | `conversationId` | Built-in customization docs define common hook payload fields. |
| JSON output | `conversation_id` | `--output-format json` returns a JSON object containing this field; local auth-failure probes returned it empty. |

The ID is stable enough for later `agy --conversation <ID>` invocation once captured. The weak point is early capture in automation. If `--output-format json` succeeds, `conversation_id` is the right stdout field. If stdout is text-only or a run fails before a conversation is created, a wrapper would otherwise have to mine logs or local cache files, which is race-prone and unsupported.

## Resume Invocation

Continue latest:

```bash
agy --continue
agy --continue --print "continue from the previous step"
agy -c -p "continue from the previous step"
```

Resume exact conversation:

```bash
agy --conversation 754ece5a-b744-473e-8c81-0c0e220fd55a
agy --conversation 754ece5a-b744-473e-8c81-0c0e220fd55a --print "continue from the previous step"
agy --conversation 754ece5a-b744-473e-8c81-0c0e220fd55a --prompt "continue from the previous step" --output-format json
agy --conversation 754ece5a-b744-473e-8c81-0c0e220fd55a --prompt-interactive "continue from the previous step"
```

Interactive picker:

```text
/resume
/switch
/conversation
```

The local explicit-resume probe was:

```bash
agy --conversation 754ece5a-b744-473e-8c81-0c0e220fd55a --print "Reply with exactly RESUME_OK." --output-format json --print-timeout 45s
```

It returned:

```json
{"conversation_id":"","status":"ERROR","response":"","error":"authentication failed or timed out","duration_seconds":0,"num_turns":0,"usage":{"input_tokens":0,"output_tokens":0,"thinking_tokens":0,"total_tokens":0}}
```

That proves the structured output shape and failure mode, but not a successful resumed turn in this run. Earlier local logs from the same host prove successful print-mode conversation creation and durable local state.

## Session Lookup Scope

Session lookup is workspace and project aware. Local `cache/last_conversations.json` maps absolute workspace paths to latest conversation IDs:

```json
{
  "/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine": "754ece5a-b744-473e-8c81-0c0e220fd55a",
  "/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff": "11b7b749-a8d5-4622-98b7-3b038e6174ca"
}
```

Local `cache/projects.json` maps workspace paths to project IDs, and logs show the backend synchronizing the active project from the conversation switch. This makes `--continue` sensitive to workspace/project history and unsuitable for concurrent wrappers unless Claudine deliberately wants "latest for this human context" semantics.

The `/resume` picker appears broader than a single current working directory. Official snippets describe tabs for CLI local TUI conversations and IDE conversations, and the changelog mentions picker work such as lazy loading, search/filtering behavior, SQLite database scanning, and skipping subagent conversations. No branch-specific filter was found.

## State Storage

The required inspection of `/Users/ken/.antigravity` found installed binaries and extensions only:

```text
/Users/ken/.antigravity/argv.json
/Users/ken/.antigravity/antigravity/bin/agy
/Users/ken/.antigravity/antigravity/bin/antigravity
/Users/ken/.antigravity/extensions/...
```

No session or conversation files were present under `/Users/ken/.antigravity`.

Actual CLI conversation state on this macOS host lives under `/Users/ken/.gemini/antigravity-cli`:

| Path | Format | Role |
| --- | --- | --- |
| `conversations/<conversation-id>.db` | SQLite | Durable per-conversation trajectory store. |
| `conversation_summaries.db` | SQLite | Summary/index cache; empty on this host during inspection. |
| `brain/<conversation-id>/.system_generated/logs/transcript.jsonl` | JSONL | Compact transcript mirror. |
| `brain/<conversation-id>/.system_generated/logs/transcript_full.jsonl` | JSONL | Full transcript mirror. |
| `cache/last_conversations.json` | JSON | Workspace to latest conversation mapping. |
| `cache/projects.json` | JSON | Workspace to project ID mapping. |
| `history.jsonl` | JSONL | Prompt/slash-command history with workspace and optional conversation ID. |
| `log/cli-*.log` | Text log | Lifecycle, auth, project, print-mode, and conversation events. |

The Antigravity 2.0 app uses a separate sibling data directory, `/Users/ken/.gemini/antigravity`, with protobuf conversation files and its own brain/artifact state. The hook docs call out product-specific transcript and artifact paths: CLI uses `antigravity-cli/`, Antigravity 2.0 uses `antigravity/`, and IDE uses `antigravity-ide/`.

The storage format is not documented as stable. The public changelog says SQLite conversation support was added in 1.0.4 and "will be CLI's conversation format"; later changelog entries mention `/resume` scanning SQLite `.db` and `.db-wal` files. That is enough to recognize the format, but not enough to make direct SQLite parsing a supported integration path.

## Restored State

Resume restores the conversation transcript and stored trajectory steps. Local JSONL transcript records include user input, conversation history, ephemeral system messages, model planner responses, model thinking, and system checkpoint summaries. SQLite step rows preserve payload and metadata blobs, including fields for permissions, task details, render info, and errors.

Tool results appear to survive as part of the trajectory/transcript. Local sample conversations were simple greeting sessions, so they did not include command output, file edits, or permission approval rows. The schema has columns capable of storing those records, and the installed binary includes transcript guidance stating each line represents a user or model action and that `transcript_full.jsonl` contains untruncated content.

Do not assume launch-time state is fully restored. Model, mode, sandbox, `--add-dir`, project selection, permissions, MCP servers, and environment variables are controlled by current settings and launch flags unless Antigravity internally overrides them from the conversation. Local logs show settings changes and project synchronization during conversation switch, but they do not prove that environment variables, sandbox mode, approval state, or extra roots survive unchanged.

The resumed session appears to keep the same conversation ID. Whether it writes to the same SQLite database, creates a new trajectory branch inside the database, or appends with additional metadata depends on internal storage behavior that was not safely verified beyond normal append-style logs.

## Branching and Checkpoints

No first-class branch or fork command was verified. The installed binary and changelog mention `/rewind`, and local transcripts include `SYSTEM` `CHECKPOINT` records that summarize earlier context. Those checkpoints are context compaction records inside the conversation, not a user-addressable checkpoint API.

The `/resume` picker supports conversation management behaviors beyond simple loading. Official snippets and changelog entries mention browsing previous conversations, search/filtering, rename input improvements, delete-related UI behavior, skipping subagent conversations, and support for scanning SQLite `.db` and `.db-wal` files. None of those prove a safe automation API for branch, fork, or rewind.

The README says "Session Export" lets terminal sessions continue in the Antigravity 2.0 GUI. The exact command or UI path was not found in installed help or local docs.

## Human-in-the-Loop Resume

Antigravity hooks can observe useful identity. The installed customization docs define common hook fields `conversationId`, `workspacePaths`, `transcriptPath`, `artifactDirectoryPath`, and `modelName`. `PreToolUse` hooks receive the tool call and step index, and can return decisions such as `allow`, `deny`, `ask`, or `force_ask`.

That is not a deferred human-in-the-loop resume API. No command or API was found to capture a pending approval prompt, ask the user elsewhere, and submit an answer back into the same suspended tool call. A wrapper can deny or let a prompt time out, ask the human through Claudine, and later run `agy --conversation <ID> --print "<answer>"`, but that is a new user turn rather than a continuation of the pending permission request.

## Interruption Recovery

Antigravity can resume after normal exit and likely after process interruption as long as the conversation state was written. The TUI has `--continue` and `--conversation`, local conversation databases are written during runs, and transcript mirrors are present. The 1.0.11 changelog says Ctrl+C first cancels active operations and double-press triggers exit, which supports a recovery model where the user can reopen the conversation afterward.

The behavior for pending tool calls and pending approvals is not established. The changelog includes fixes for prompt/permission races and pending states, but no durable pending-approval resume contract was found. If a process is killed while a tool call is in flight, Claudine should assume the provider may leave a partial step and should resume by adding a new user turn that asks the agent to inspect current state.

Concurrent resumes of the same conversation are undocumented. Because local storage uses SQLite plus WAL files, there may be database-level serialization, but semantic safety is unknown. Claudine should avoid running multiple `agy --conversation <ID>` processes concurrently against the same conversation.

## Observability

Useful resume observability surfaces are:

| Surface | Resume value |
| --- | --- |
| `--output-format json` | Structured print-mode result with `conversation_id`, `status`, `response`, `error`, timing, turn count, and usage. |
| `~/.gemini/antigravity-cli/log/cli-*.log` | Conversation creation, active conversation, project synchronization, print-mode state, auth failures, and stream completion. |
| `cache/last_conversations.json` | Latest conversation by workspace path. |
| `history.jsonl` | User prompt and slash-command history with workspace and optional conversation ID. |
| `conversations/<id>.db` | Durable SQLite conversation store. |
| `brain/<id>/.system_generated/logs/transcript*.jsonl` | Human-readable transcript mirrors and checkpoint summaries. |
| Hooks | `conversationId` and `transcriptPath` in hook payloads. |

Observed failure modes include `authentication failed or timed out`, unauthenticated model/config polling, empty summary cache, and the risk that an invalid conversation ID cannot be diagnosed until after auth/setup.

## Quirks and Gaps

Quirks:

- `/Users/ken/.antigravity` is not where CLI conversations live on this host.
- The installed CLI data path is under `.gemini`, reflecting Antigravity's shared backend lineage.
- The public docs site is a client-rendered Angular app, so direct text extraction is incomplete.
- `agy --help` omits `--output-format json` even though the installed binary accepts it.
- `agy --help` formats `--conversation` as if it had no value, but its description and behavior are value-taking.
- `conversation_summaries.db` can exist with zero rows.
- SQLite and transcript parsing are useful evidence but unsupported as an integration contract.

Gaps:

- Successful structured `conversation_id` capture from a resumed print-mode turn was not re-verified because local auth failed.
- Invalid conversation ID behavior was not isolated because auth failed first.
- Pending approval, pending tool call, and background task recovery remain unknown.
- Concurrent resume behavior is unknown.
- Retention and cleanup policy for conversation DBs is unknown.
- CLI-to-GUI export invocation is unknown.

## Claudine Integration Notes

Claudine should model Antigravity resume as partial but useful. The only automation-safe selector is an explicit conversation ID with `agy --conversation <ID> --print <PROMPT>`. `agy --continue` should be exposed only as a latest-session human convenience because it can attach to the wrong conversation when several Claudine runs share a host.

For lifecycle `resume`, Claudine should capture `conversation_id` from structured print output when present, and can use hook `conversationId` or logs as diagnostic fallback. It should not parse SQLite blobs for normal operation. If a run fails before a conversation ID is emitted, retry/resume should degrade to a fresh run or require a caller-supplied conversation ID.

For `retry`, Claudine should prefer a new print-mode turn in the same conversation when an explicit ID exists. For `proxy`, Antigravity's current surfaces do not provide a suspended-turn API; Claudine can only ask another route and inject the result as a new user turn. For future human-in-the-loop recovery, a pending permission request should be treated as non-resumable unless Antigravity later documents an answer-injection API.

## Changelog

Initial research file created on 2026-07-08.

## Sources

- [Managing Conversations - Google Antigravity Documentation](https://antigravity.google/docs/cli/conversations)
- [CLI Reference - Google Antigravity Documentation](https://antigravity.google/docs/cli-reference)
- [Using AGY CLI - Google Antigravity Documentation](https://antigravity.google/docs/cli/using)
- [google-antigravity/antigravity-cli README](https://github.com/google-antigravity/antigravity-cli)
- [google-antigravity/antigravity-cli CHANGELOG](https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md)
- [GitHub issue #7: emit per-conversation ID for print mode](https://github.com/google-antigravity/antigravity-cli/issues/7)
- Local installed `agy` 1.1.0 help and probes on this macOS host.
- Local storage inspection under `/Users/ken/.antigravity`, `/Users/ken/.gemini/antigravity-cli`, and `/Users/ken/.gemini/antigravity`.
