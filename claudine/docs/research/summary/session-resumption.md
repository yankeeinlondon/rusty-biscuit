---
sequence:
- name: draft
- name: iterate
- name: finalize
prompt: |-
  Resuming a prior session with its context intact is what makes recovery and iterative follow-up workflows possible — it is the mechanism behind Claudine's lifecycle `resume` recovery action. Providers differ in whether resumption exists at all, how sessions are identified and discovered, and what continuation semantics apply.

  ## Task

  Your task is to report on session resumption support across the Agentic CLI providers Claudine supports.

  - your report should start by outlining why session resumption matters to agentic processes (error recovery, follow-up prompts, long-running work)
  - and then shift its focus to how providers differ: resume flags and commands, session-ID discovery and injection, continuation semantics, and limitations (non-interactive resume, cross-machine, expiry)
  - close with a point of view on how each provider's mechanism fits Claudine's resume recovery action and planned ResumeSpec metadata

  As background material we have resume research documents for each provider that Claudine supports. They can be found at `@claudine/docs/research/resume/*.md`.

  Important: your final response is saved verbatim as the body of this summary document, so it must be the complete document text and nothing else — no preamble, no commentary. Never write to this document yourself.

  ::block when="state.name == 'draft'"
  - Iterate over the first three research documents to develop a point of view on how to write this document and then produce an initial draft of the document
  ::end-block
  ::block when="state.name == 'iterate'"

  - Note: the initial draft has already been created — it is the body of `@claudine/docs/research/summary/session-resumption.md` (everything below the frontmatter); read it from there
  - Act as an orchestrator and iterate over each remaining provider's research document:
      - provide the subagent the current draft and ask them to return an improved draft based on the research document they've been assigned
  - Once every remaining provider has been incorporated, your final response is the fully updated draft
  ::end-block

  ::block when="state.name == 'finalize'"

  The document has now gone through several rounds of improvement and your task is just to make sure the document is consistent in tone and detail and that nothing looks incorrect or incomplete. The current draft is the body of `@claudine/docs/research/summary/session-resumption.md` (everything below the frontmatter); read it from there, make any adjustments, and your final response will be considered the finalized summary document.
  ::end-block
hash: bb06014c9d46530e-c3f55d3155020440
last_updated: 2026-07-03
---
# Session Resumption Across Agentic CLI Providers

Session resumption is one of the core capabilities that turns an agentic CLI from a one-shot command runner into a recoverable process. Agentic work is long-running, stateful, and failure-prone: the model reads files, forms plans, applies edits, receives tool results, asks for permission, hits rate limits, and may run long enough that terminals close, networks fail, shells die, or a wrapper times out. If the provider can resume the same session, Claudine can recover from those interruptions without throwing away context or asking the model to rediscover the work from scratch.

Resume also matters for iterative follow-up workflows. A lifecycle action may need to run the provider again after a successful or failed pass: fix a missing artifact, answer a model question collected out-of-band, continue after a policy decision, summarize the finished work, or ask for a narrower correction. Starting fresh is weaker than resuming because the provider may have transcript context, tool results, plans, permissions, checkpoints, or project metadata that are not fully reconstructable from the filesystem alone.

The hard part is that providers do not mean the same thing by "resume." Some replay a local transcript into a new process. Some load a local SQLite-backed server session. Some expose both CLI replay and a live daemon API. Some have a human picker but no automation-safe selector. Some preserve approvals; others recalculate them. Some can resume non-interactive sessions; others hide those sessions from interactive pickers unless explicitly requested. Claudine's lifecycle `resume` recovery action therefore cannot be a single provider-neutral command string. It needs provider metadata that says how to capture the handle, how to re-enter the session, what state is actually restored, and which limitations make resume unsafe.

## Provider Landscape

| Provider    | Practical support           | Continuity model                                                                     | Best Claudine primitive                                                                                                                                                                                        |
|-------------|-----------------------------|--------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Claude Code | First-class                 | Mixed: local transcript replay, SDK resume, Remote Control/live attach, web sessions | Capture `session_id` from JSON, stream-json, SDK, hooks, or status-line input; use `claude -p --resume <id> --output-format json "<prompt>"` or SDK resume                                                     |
| Codex CLI   | First-class                 | Mixed: local rollout replay plus app-server JSON-RPC                                 | Capture `thread_id` from `codex exec --json` `thread.started` or hooks; use `codex exec resume --json <id> "<prompt>"` or app-server `thread/resume` plus `turn/start`                                         |
| Gemini CLI  | First-class                 | Local transcript replay                                                              | Capture `session_id`; use `gemini -p "<prompt>" --resume <id> --output-format stream-json`                                                                                                                     |
| Goose       | First-class, with caveats   | Local SQLite transcript replay                                                       | Capture explicit `YYYYMMDD_N` session ID; use `goose run --resume --session-id <id> -t "<prompt>" --output-format stream-json`                                                                                 |
| Kimi Code   | First-class, with caveats   | Local transcript replay; ACP/server surfaces load local state                        | Capture `session_<uuid>` from hooks or `$KIMI_CODE_HOME/session_index.jsonl`; use `kimi -p "<prompt>" --session <id> --output-format stream-json` or ACP `session/load`/`session/resume` then `session/prompt` |
| OpenCode    | First-class                 | Local persisted server session                                                       | Capture `ses_...` from `opencode run --format json` or `opencode session list --format json`; use `opencode run --session <id> "<prompt>"` or server/SDK message APIs                                          |
| Qwen Code   | First-class                 | Mixed: local transcript replay plus daemon session restore                           | Capture UUID from structured output, hooks, or `qwen sessions list --json`; use `qwen --resume <id> -p "<prompt>"` or daemon load/resume plus prompt                                                           |
| Roo Code    | Promising but least-current | Local task history plus shadow-Git checkpoints                                       | Capture `taskId`; `roo resume <task-id>` is reported, but wrapper-grade non-interactive prompt injection needs refreshed validation                                                                            |

The strongest automation pattern is consistent across most providers: capture a stable session handle as early as possible, persist it in Claudine run metadata, and resume by explicit handle with a follow-up prompt. Claudine should treat "continue latest" and picker-based resume as human conveniences, not as reliable recovery primitives.

## Resume Flags and Commands

Claude, Codex, Gemini, Qwen, Kimi, Goose, and OpenCode all expose direct resume commands. The names differ:

- Claude: `--continue`, `--resume <session-id>`, `claude -p --continue --output-format json "<prompt>"`, `claude -p --resume <session-id> --output-format json "<prompt>"`, plus SDK `continue`, `resume`, and `fork_session`.
- Codex: `codex resume ...` for TUI resume, `codex exec resume --json <SESSION_ID|THREAD_NAME> "<prompt>"` for headless follow-up, `codex exec resume <id> -` for stdin, and app-server `thread/resume` plus `turn/start`.
- Gemini: `--resume`, `-r`, `--resume latest`, `--resume <index>`, `--resume <session_id>`, and `gemini -p "<prompt>" --resume <session_id> --output-format stream-json`.
- Goose: `goose session --resume ...` interactively and `goose run --resume --session-id <id> -t "<prompt>"` headlessly. Follow-up text can come from `--text`/`-t`, `--instructions`, stdin, or a recipe.
- Kimi: `--continue`, `--session`, `--session <id>`, hidden/alias-style `--resume`/`-r`, `kimi -p "<prompt>" --session <id>`, and ACP `session/load`, `session/resume`, and `session/prompt`.
- OpenCode: `--continue`, `--session <id>`, `opencode run --continue "<message>"`, `opencode run --session <id> "<message>"`, `opencode run --session <id> --fork "<message>"`, and server/SDK message APIs.
- Qwen: `--continue`, `--resume <session-id>`, `--fork-session`, and daemon routes such as `POST /session/:id/load`, `POST /session/:id/resume`, and `POST /session/:id/prompt`.
- Roo: older research reports `roo resume <task-id>`, with task history and programmatic extension APIs, but not enough current evidence for a first-class Claudine wrapper contract.

The most important split is interactive versus non-interactive. Interactive resume usually opens a TUI or reconnects a client to a saved session; some providers accept an initial prompt on the interactive resume command, but the process still belongs to a human-oriented surface. That is useful for `proxy`, but weak for lifecycle `resume`. Claudine needs a non-interactive form that accepts a follow-up prompt and returns structured output. Claude, Codex, Gemini, Goose, Kimi, OpenCode, and Qwen have such a form. Roo should be treated as needing refreshed validation before being used as an automated resume target.

## Session-ID Discovery and Injection

Resume requires a handle that can be rediscovered or captured. Providers expose that handle through different surfaces:

- Claude emits `session_id` in JSON/stream-json print mode, SDK messages, hooks, status-line inputs, transcript filenames, and `claude agents --json` for running/background sessions.
- Codex emits `thread_id` from `codex exec --json` `thread.started`, stores IDs in rollout filenames, `history.jsonl`, `session_index.jsonl`, and `state_5.sqlite`, and exposes thread IDs through app-server APIs. Rollout files observed locally store the ID at `session_meta.payload.session_id`.
- Gemini emits `session_id` in stream-json init events, hooks, `gemini --list-sessions`, transcript metadata, and TUI footer commands.
- Goose can list IDs through `goose session list --format json`, and may print an ID in the session header unless quiet output suppresses it. `stream-json` is useful for progress but not a stable ID capture surface.
- Kimi indexes sessions in `$KIMI_CODE_HOME/session_index.jsonl`, exposes hook `session_id`, and uses ACP session identifiers. Current Kimi Code defaults to `~/.kimi-code`, not legacy `~/.kimi`.
- OpenCode emits `sessionID` in `opencode run --format json`, returns IDs from `opencode session list --format json`, and exposes IDs through server/SDK APIs. `opencode export <sessionID>` and `opencode db path` are useful diagnostics, not primary integration contracts.
- Qwen exposes UUIDs through structured output, hooks, `qwen sessions list --json` on current versions, transcript filenames, and daemon events. Older installed versions may lack the `sessions` subcommand.
- Roo uses a `taskId` surfaced through `taskCreated`, `roo history`, or JSON output according to the older research.

Claudine should prefer structured provider output, hooks, documented list/status APIs, or server APIs over direct parsing of internal transcript files. Local transcript parsing is valuable for recovery diagnostics, but most providers treat those formats as internal and unstable.

Injection also differs. Some providers take the follow-up prompt as a positional CLI argument. Some can read it from stdin. Some require a two-step API sequence: load or resume the session, then submit a message or turn. Planned `ResumeSpec` metadata should separate `handle_capture` from `followup_injection`, rather than assuming a resume command both selects and prompts in one step.

## Continuation Semantics

The main continuity models are:

- **Transcript replay:** Claude CLI, Gemini, Goose, Kimi, and Qwen standalone CLI reconstruct context from persisted transcripts. Codex also uses rollout replay for normal CLI resume.
- **Local server session:** OpenCode treats the local server/session store as the normal model. Codex app-server, Qwen daemon, and Kimi ACP/server surfaces add similar local API layers over persisted session state.
- **Live-process attach:** Claude Remote Control, OpenCode attach, and Qwen daemon clients can attach to a running process or server.
- **Server-side/cloud session:** Claude Code on the web and Kilo Cloud Agent-style sessions are server-side or cloud-mediated; they should not be treated as equivalent to local CLI transcript replay.

Transcript replay is good for crash recovery and follow-up prompts, but it is not the same as resuming an in-flight tool call. A tool result must have been written to durable state to be replayable. A permission request must have a provider-supported pending-request API to be answerable later. Otherwise, the resumed provider can continue the conversation, but not necessarily the exact suspended operation.

Resume also does not guarantee full environment restoration. Most providers use the current process environment at resume time. Model, sandbox, approval mode, working directory, MCP config, extra roots, and plugin state may be restored, recalculated, or overrideable depending on provider and flags. Claudine should record original launch metadata and reapply what matters instead of assuming the provider does.

## Scope and Lookup

Session lookup scope is one of the easiest ways to resume the wrong work.

Claude is project/worktree-aware; session ID lookup is stricter than picker browsing, and print/SDK sessions may be resumable by ID without appearing in the interactive picker. Codex is cwd-scoped by default but has `--all`; app-server listing can filter by cwd and source kind, and API callers must include `exec` source kinds when looking for non-interactive sessions. Gemini is project/current-directory scoped; worktree resume requires launching from the intended worktree. Qwen is project/current-cwd scoped and branch-aware in picker-style workflows; a UUID alone may not be sufficient from the wrong project/worktree scope. OpenCode records project/worktree and directory, and `--continue` is implicit enough that Claudine should prefer explicit IDs. Goose is unusual: plain resume is global over visible sessions, not repository-scoped. Kimi groups sessions by working-directory key and has all-directory picker behavior in some surfaces, but Claudine should preserve the original cwd.

`ResumeSpec` should capture scope explicitly: cwd-scoped, project-scoped, worktree-aware, all-projects capable, branch-filter capable, and whether an explicit ID is sufficient without launching from the original directory. For providers where launch cwd matters, Claudine should store and reuse the original cwd.

## Limitations

Non-interactive resume is not always equivalent to interactive resume. Claude and Codex have strong headless continuation surfaces. Gemini, OpenCode, Qwen, Goose, and Kimi also support scriptable follow-up, but their guarantees differ. Goose cannot durably pause for approval and answer later. Kimi non-interactive mode uses auto permission behavior and has no documented persisted pending-question injection. OpenCode and Qwen have richer server/daemon paths for live permission mediation, but pending approvals are still live state and may be cancelled when the session/server dies.

Cross-machine resume is generally weak unless the provider has server-side state or the full local session store is copied. Local transcript replay providers depend on host-local files: `~/.claude`, `~/.codex`, `~/.gemini`, `~/.local/share/goose`, `~/.kimi-code`, `~/.local/share/opencode`, `~/.qwen`, or VS Code/Roo storage. A session ID alone is not portable if the backing transcript, database, checkpoint store, or server state is absent.

Expiry and cleanup also vary. Claude and Gemini document default cleanup around 30 days for many session artifacts. Codex rollout cleanup is not clearly documented; `--ephemeral` intentionally prevents future resume. Goose sessions appear indefinite until removed, while logs have separate cleanup. Kimi retention is not clearly documented. OpenCode session database retention is not clearly documented, though logs keep the most recent files. Qwen retention is unknown, and archived Qwen sessions must be unarchived before load/resume. `ResumeSpec` should include `retention_known`, `retention_policy`, and `non_resumable_conditions` such as `--ephemeral`, `--no-session`, disabled persistence, archived sessions, missing local storage, disabled chat recording, or provider-specific no-persistence flags.

Concurrency is broadly unsafe or undocumented. Claude explicitly warns about interleaved transcript writes when the same session is resumed twice. Codex, Gemini, Goose, Kimi, OpenCode, and Qwen do not provide enough high-level guarantees to let Claudine concurrently append to the same session. Qwen daemon restore has explicit in-progress protection, but standalone JSONL appends remain undocumented. Claudine should serialize resume attempts per provider session ID unless a provider API explicitly guarantees safe queuing.

## Fit for Claudine Resume

Claudine's lifecycle `resume` action should be metadata-driven, not hard-coded around command names. The planned `ResumeSpec` should describe at least:

- Support level: none, interactive-only, non-interactive, API/server, or mixed.
- Continuity model: transcript replay, local server session, live attach, cloud session, or mixed.
- Stable handle fields and capture surfaces.
- Resume invocation templates for latest, explicit ID, fork, and API/server modes.
- Whether a follow-up prompt can be injected at resume time.
- Whether structured output is available after resume.
- Lookup scope and required launch cwd.
- Storage location and retention boundaries.
- Restored state: transcript, tool results, approvals, model, cwd, env, sandbox, MCP, attachments.
- HITL capability: no, follow-up-only, synchronous live API, durable deferred request, or mixed.
- Interruption semantics: crash, Ctrl+C, pending tool, pending approval, concurrent resume.
- Non-resumable cases and provider-specific traps.
- Persistence boundaries such as Claude `--no-session-persistence`, SDK `persistSession: false`, `CLAUDE_CODE_SKIP_PROMPT_HISTORY`, Codex `--ephemeral`, Goose `--no-session`, or provider-specific disabled chat recording.
- Picker behavior, including whether non-interactive sessions appear in human pickers.
- API resume sequences, such as Codex `thread/resume` then `turn/start`, or Qwen `load`/`resume` then `prompt`.

Provider fit:

Claude is an excellent fit for lifecycle `resume` when Claudine captures `session_id` from JSON, stream-json, SDK, hooks, or status-line input and resumes with `claude -p --resume <session-id> --output-format json "<prompt>"` or SDK `resume`. Its strongest durable HITL path is `PreToolUse` defer in print/SDK workflows: capture `tool_deferred`, `deferred_tool_use`, the session ID, and the tool ID/name/input, then resume the same session after out-of-band input. Remote Control and Claude Code on the web are better modeled as `proxy` or live attach, not as automation-safe lifecycle resume.

Codex is also an excellent fit, but Claudine should treat `codex exec resume` and app-server as distinct modes. The simple wrapper path is `codex exec resume --json <thread-id> "<prompt>"`, with the ID captured from `codex exec --json` `thread.started.thread_id` or hooks. The richer path is app-server `thread/resume` followed by `turn/start`, or `turn/steer` for an active turn. Claudine must record and honor `--ephemeral` because those runs are intentionally not future-resumable, and it should prefer UUIDs over thread names for automation.

Gemini is a good fit for transcript-level recovery and follow-up prompts. Claudine should capture `session_id` from stream-json init output or hook input and store the original project/worktree cwd. The preferred non-interactive path is `gemini -p "<follow-up>" --resume <session_id> --output-format stream-json`. Full session IDs are first-class selectors; `latest` and numeric indexes are convenience selectors and should not be used for automated recovery unless Claudine has just listed sessions in the same directory. HITL should be modeled as a new follow-up turn after blocking or capturing `ask_user`, not as durable pending-tool resume.

Goose fits Claudine lifecycle `resume` for normal follow-up and crash recovery, but not durable HITL. Claudine should capture an explicit session ID such as `YYYYMMDD_N` and resume with `goose run --resume --session-id <id> -t "<follow-up>" --output-format stream-json`. Avoid global `--resume` and name-based selection for automation; plain resume is not repository-scoped, and explicit `--session-id` is the safest path for preserving saved provider/model metadata. `stream-json` is useful for progress, not handle capture. Headless resume stays in the current launch directory, so Claudine should restore cwd itself. Approval and MCP elicitation prompts are live-process-only and should fail or proxy rather than be modeled as resumable pending questions.

Kimi fits prompt-time continuation and ACP-driven clients, but pending approvals/questions are not durable recovery points. Claudine should prefer current Kimi Code behavior: `KIMI_CODE_HOME` defaulting to `~/.kimi-code`, `session_index.jsonl`, `sessions/<workDirKey>/<sessionId>/state.json`, and `agents/*/wire.jsonl`. Use `-p/--prompt`, not legacy `--print`, for non-interactive follow-up. The preferred path is `kimi -p "<prompt>" --session <session_<uuid>> --output-format stream-json`; ACP clients can `session/load` or `session/resume`, then `session/prompt`. Claudine should preserve the original cwd and serialize per session ID.

OpenCode is a strong fit for Claudine lifecycle `resume` when Claudine captures the `ses_...` handle from `opencode run --format json` or `opencode session list --format json` and resumes explicitly with `opencode run --session <id> "<follow-up>"`. For durable HITL, Claudine should prefer the server/SDK path, using status/events plus permission and message endpoints such as `POST /session/:id/message`, `POST /session/:id/prompt_async`, `POST /session/:id/permissions/:permissionID`, and `GET /session/status`. `--continue` is acceptable only as a human convenience or last-resort latest selector. Pending permissions, questions, and tool calls are live server state; after server death, Claudine can resume the transcript but should not assume the exact blocked operation is still answerable.

Qwen fits strongly for both CLI resume and daemon-backed workflows. Claudine should distinguish standalone transcript replay from daemon load/resume/prompt. The CLI path is `qwen --resume <id> -p "<prompt>"` or `qwen --continue -p "<prompt>"`; `--fork-session` is valid only with `--continue` or `--resume`. The daemon path is `POST /session/:id/load` to restore and replay ACP history, or `POST /session/:id/resume` to restore without replay, followed by `POST /session/:id/prompt` for the follow-up. `qwen/control/session/continue` is a distinct path for continuing an interrupted last turn without adding a synthetic user prompt. Claudine should not assume pending daemon permissions survive session close; they are live and answerable only while the daemon session is alive.

Roo should remain promising but unverified for Claudine lifecycle `resume`. The available research describes a local `taskId`, task history, shadow-Git checkpoints, `roo resume <task-id>`, and programmatic `waitingForInput`/`respond` style HITL handling. That maps well to a future `ResumeSpec` with local handle capture and API-mediated human input, but the current document does not establish a wrapper-grade non-interactive resume command that both selects a task and injects a follow-up prompt. Claudine should require refreshed validation before treating Roo as a first-class lifecycle resume target.

## Research-Only Providers

Pi and Kilo are researched future providers, not part of Claudine's compiled support roster in this summary. Their mechanisms are mature enough to shape `ResumeSpec`, but implementation should wait for provider metadata and wrapper support.

Pi has first-class provider-side resume through local JSONL transcript replay. It supports `pi -c`, `pi -r`, `pi --session <path|id>`, `pi -p --continue "<prompt>"`, `pi -p --session <path|id> "<prompt>"`, `pi --mode json --session <path|id> "<prompt>"`, and RPC `pi --mode rpc --session <path|id>` followed by `prompt`, `steer`, or `follow_up`. It captures UUIDs from JSON mode, JSONL headers/filenames, `/session`, and RPC/SDK state. The main traps for a future Claudine integration are cross-project UUID-prefix matches that can prompt to fork, `$HOME` overlay confusion, non-resumable `--no-session` or in-memory sessions, no durable HITL after process exit, and undocumented concurrent writes.

Kilo has first-class provider-side resume through a local/server session model backed by SQLite, plus cloud import paths. It supports `kilo --continue`, `kilo --session <id>`, `kilo --session <id> --fork`, `kilo --session <id> --cloud-fork`, `kilo run --continue <message>`, `kilo run --session <id> <message>`, and `kilo run --attach <url> --session <id> <message>`. Server and SDK paths include `POST /session/{sessionID}/message`, `POST /session/{sessionID}/fork`, `POST /session/{sessionID}/revert`, and `createKiloClient().session.prompt()`. It is an excellent future `ResumeSpec` influence because it has structured `sessionID` output, worktree-aware scope, fork/revert support, HITL endpoints, and busy-session behavior. The main caveat is that plain `kilo run` is poor for HITL because it denies questions and auto-rejects permission prompts unless configured otherwise; pending questions and permissions survive only while the owning server/runtime is alive.

## Point of View

Claudine should make resume opportunistic but precise. If a provider gives us a stable handle, a scriptable prompt-injection path, and enough structured observability, `resume` should be the default recovery action for follow-up and post-failure continuation. If any of those pieces are missing, Claudine should degrade explicitly: retry fresh, proxy to a human surface, fork before continuing, or report that provider-level resume is unavailable for that run.

The design pressure on `ResumeSpec` is to avoid collapsing different provider semantics into one boolean. Resume support is not just "has a resume flag." Claudine needs to know whether the handle is durable, whether the session is local or remote, whether lookup is cwd-scoped, whether non-interactive sessions appear in discovery, whether the follow-up can be injected in the same command, whether HITL is durable or live-only, and which launch settings must be reapplied. That metadata is what lets Claudine's lifecycle `resume` action be safe recovery rather than a best-effort command replay.
