---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3

homepage: https://qwenlm.github.io/qwen-code-docs/en/users/overview/
docs: https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/
subagent_docs: https://qwenlm.github.io/qwen-code-docs/en/users/features/sub-agents/

support: first_class

locations:
  - os: macos
    scope: user
    path: "~/.qwen/agents/<name>.md"
    notes: "Personal subagents applied across all projects. Resolved via Storage.getGlobalQwenDir(); QWEN_HOME redirects this to $QWEN_HOME/agents/<name>.md. Each file is YAML-frontmatter Markdown; the basename (without .md) does NOT have to match the frontmatter `name` field, the loader scans every .md file in the directory. Observed on this host: three symlinks pointing to Claude-authored agents in ~/.claude/agents/."
  - os: linux
    scope: user
    path: "~/.qwen/agents/<name>.md"
    notes: "Same as macOS. Resolved under XDG-equivalent home via Storage.getGlobalQwenDir(); QWEN_HOME redirects."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\agents\\<name>.md"
    notes: "Same as macOS/Linux; backslash path form. QWEN_HOME redirects on Windows too."
  - os: macos
    scope: repo
    path: ".qwen/agents/<name>.md"
    notes: "Project-scope agents. The path is constructed from `<projectRoot>/.qwen/agents/`, where projectRoot comes from `config.getProjectRoot()` (the session's launch directory). Discovered at session start via SubagentManager.listSubagentsAtLevel('project'); in-session edits are not hot-reloaded, the cache refreshes via /agents UI CRUD or session start. Project root matching the home directory is treated as no-project."
  - os: linux
    scope: repo
    path: ".qwen/agents/<name>.md"
    notes: "Same as macOS. Resolved under the session's project root."
  - os: windows
    scope: repo
    path: ".qwen\\agents\\<name>.md"
    notes: "Same as macOS/Linux; backslash path form."
  - os: macos
    scope: extension
    path: "<extension-root>/agents/<name>.md"
    notes: "Extension-scoped agents. Source-confirmed via SubagentManager.listSubagentsAtLevel('extension'), which iterates `this.config.getActiveExtensions()` and returns each `extension.agents` array. The on-disk path is constructed by the extension's own loader; Qwen's CLI does not pin a directory layout here (unlike the `~/.qwen/extensions/<name>/skills/<skill>/SKILL.md` shape used for skills), the extension package's `agents` property carries an array of fully-formed `SubagentConfig` records."
  - os: linux
    scope: extension
    path: "<extension-root>/agents/<name>.md"
    notes: "Same as macOS; path is extension-defined."
  - os: windows
    scope: extension
    path: "<extension-root>\\agents\\<name>.md"
    notes: "Same as macOS/Linux."
  - os: macos
    scope: system
    path: "<qwen-code package>/bundled/<agent>.md (built-in)"
    notes: "Built-in subagents are NOT on disk in the user's environment. The three shipped built-ins (general-purpose, Explore, statusline-setup) are hardcoded in BuiltinAgentRegistry (packages/core/src/subagents/builtin-agents.ts) and emitted as SubagentConfig records with `level: 'builtin'` and `filePath: '<builtin:<name>>'` — not a real path. There is no filesystem-bundled agent directory."
  - os: linux
    scope: system
    path: "<qwen-code package>/bundled/<agent>.md (built-in)"
    notes: "Same as macOS; bundled agents are in-source constants, not files."
  - os: windows
    scope: system
    path: "<qwen-code package>\\bundled\\<agent>.md (built-in)"
    notes: "Same as macOS/Linux."
  - os: macos
    scope: other
    path: runtime session injection via SubagentManager.loadSessionSubagents()
    notes: "Session-level agents are NOT on disk; the SDK / programmatic API calls `SubagentManager.loadSessionSubagents(subagents)` to inject an in-memory list of SubagentConfig records. Cache holds them under `subagentsCache.get('session')` with `filePath: '<session:<name>>'`. Session-level agents are read-only and the highest-precedence source at runtime resolution."
  - os: linux
    scope: other
    path: runtime session injection via SubagentManager.loadSessionSubagents()
    notes: "Same as macOS."
  - os: windows
    scope: other
    path: runtime session injection via SubagentManager.loadSessionSubagents()
    notes: "Same as macOS/Linux."
  - os: macos
    scope: other
    path: 'fork (subagent_type: "fork")'
    notes: "A pseudo subagent type with no on-disk definition. When the Agent tool is called without an explicit subagent_type and the fork feature flag (`isForkSubagentEnabled(config)`) returns true, the parent model gets `subagent_type: 'fork'`. The fork is implemented in packages/core/src/tools/agent/fork-subagent.ts and shares the parent's prompt cache (FORKSUBAGENT_TYPE is the literal 'fork'). No agent definition is loaded for forks."
  - os: linux
    scope: other
    path: 'fork (subagent_type: "fork")'
    notes: "Same as macOS."
  - os: windows
    scope: other
    path: 'fork (subagent_type: "fork")'
    notes: "Same as macOS/Linux."

format:
  file_names:
    - "<name>.md (Markdown with YAML frontmatter; the basename does not need to match the frontmatter `name` field)"
    - "<extension-root>/agents/agents[] (extension-contributed SubagentConfig records; not file-based)"
    - "BuiltinAgentRegistry constants (built-in; not file-based)"
    - "SubagentManager.loadSessionSubagents(...) (session; not file-based)"
  frontmatter: true
  required_fields:
    - name (validated by SubagentValidator.validateName — must match /^[\\p{L}\\p{N}_-]+$/u, length 2..50, not starting/ending with - or _, not a reserved word like self/system/user/model/tool/config/default/main)
    - description (non-empty, ≤1000 chars triggers a soft warning)
  optional_fields:
    - "tools (allowlist of tool names; empty array means inherit-all; also accepts comma-separated string)"
    - "disallowedTools (denylist; supports `mcp__server` and `mcp__server__tool` MCP patterns)"
    - "model (selector: `inherit` | `fast` | `<model-id>` | `<authType>:<model-id>`)"
    - "approvalMode (`default` | `plan` | `auto-edit` | `yolo` | `bubble` — `bubble` is subagent-only)"
    - "runConfig (object with `max_time_minutes` and `max_turns`; legacy nested form)"
    - "maxTurns (top-level CC 2.1.168 bridge; positive integer, accepts numeric string; promoted from runConfig.max_turns)"
    - "permissionMode (CC bridge: `acceptEdits` | `auto` | `bypassPermissions` | `default` | `dontAsk` | `plan`; mapped to qwen approvalMode; approvalMode wins when both set)"
    - "mcpServers (CC bridge: record-of-records; per-agent MCP server overrides; keys shadow session-level servers on collision)"
    - "hooks (CC bridge: record-of-arrays keyed by HookEventName; per-agent hooks; v1 fires globally without per-agent scope filtering)"
    - "color (allowlist `red`/`blue`/`green`/`yellow`/`purple`/`orange`/`pink`/`cyan` plus legacy `auto`; values outside silently dropped with a warn)"
    - "background (boolean; when true OR'd with the tool's `run_in_background` parameter)"
  body_format: markdown
  notes: |
    The file format is the Claude Code 2.1.168 declarative-agent schema, ported per docs/declarative-agents-port.md. PR #4842 shipped the 16 fields end-to-end; PR #4870 replaced the YAML parser to support block scalars. The parser follows Claude's "DL7 lenient" posture: invalid optional fields are dropped to undefined with a warning rather than thrown (different from approvalMode, which predates the port and is still strict-throw).
    Frontmatter is parsed by `packages/core/src/utils/yaml-parser.ts` (hand-rolled splitter, no `gray-matter`/`js-yaml`/`front-matter` dep). The 16-field schema is mirrored verbatim: `name`, `description`, `model`, `tools`, `disallowedTools`, `effort` (deferred), `permissionMode`, `mcpServers`, `hooks`, `maxTurns`, `skills` (deferred), `initialPrompt` (deferred), `memory` (deferred), `background`, `isolation` (deferred), `color`. The five deferred fields are carried in the registry as no-op metadata in v1; runtime wiring lands in follow-up PRs when each prerequisite subsystem is ready.
    `parseSubagentContent` does case-insensitive name matching during lookup, so an Agent tool call with `subagent_type: "Test_Engineer"` resolves to a file whose `name: "test-engineer"`. The validator only soft-warns on case mixing and on mixing hyphens with underscores, so authors can ship non-canonical names as long as they pass the regex.
    Body text is the system prompt. There is no Markdown-only versus code-only separation; the entire trimmed body after the closing `---` becomes the subagent's `systemPrompt`. Embedded `${variable}` substitution is supported via `ContextState` at runtime; embedded `$ARGUMENTS`-style placeholders are NOT honored (Qwen's agent frontmatter is more limited than its skill frontmatter).

runtime:
  invocation: |
    Primary agents invoke subagents through the `agent` tool (`packages/core/src/tools/agent/agent.ts`, tool name `ToolNames.AGENT`, registered in the standard tool registry). The tool schema accepts:
    - `description` (required string, 3-5 word label)
    - `prompt` (required string, the delegated task)
    - `subagent_type` (optional string; defaults to `general-purpose` when omitted; `fork` is a deliberate pseudo-type)
    - `run_in_background` (boolean; top-level sessions only; from inside a sub-agent it runs in the foreground)
    - `isolation` (`worktree` only; spins up `<projectRoot>/.qwen/worktrees/agent-<7hex>`; auto-removed when no changes, preserved otherwise)
    - `name` and `plan_mode_required` (only when `experimental.agentTeam` is on, for spawning teammates via TeamManager)

    Three invocation paths reach a subagent:
    1. **Explicit type**: `agent(subagent_type="<name>", prompt="...")` — the parent model picks the type. `subagent_type` matches `availableSubagents` case-insensitively against the frontmatter `name`.
    2. **Implicit default**: `agent(prompt="...")` with no type — defaults to `general-purpose` (the canonical built-in name from `DEFAULT_BUILTIN_SUBAGENT_TYPE`).
    3. **Fork**: `agent(subagent_type="fork", prompt="...", name="...")` — when the `isForkSubagentEnabled(config)` flag is on (toggled via the experimental flag). Fork inherits the parent's full conversation history, system prompt, and tools; runs detached in the background; results are NOT returned to the parent (the parent gets a placeholder completion notification).
    4. **TUI/CLI direct**: there is currently no `--agent <name>` CLI flag in qwen-code (planned for the P3 phase of the declarative-agents port per docs/declarative-agents-port.md §D5 — once it lands, `qwen --agent code-reviewer` selects the agent as the main session agent, replacing the default system prompt unless the agent declares `appendSystemPrompt: true`).

    The Agent tool dynamically refreshes its schema enum via `subagentManager.addChangeListener` whenever the agent registry changes. Built-in types are advertised in the description; the enum lists every registered `name` (lowercase canonical). When the description is built, a team-coordination paragraph is appended only when `experimental.agentTeam` is enabled.
  parent_child_context: |
    Each subagent invocation runs in its own isolated session, built by `SubagentManager.createAgentHeadless()`. The child receives:
    - The agent's frontmatter body as the `systemPrompt` (no Qwen system-prompt preamble, no QWEN.md loading — the body is the full system prompt).
    - The delegated `prompt` as the task.
    - A fresh `Config` override created via `Object.create(runtimeContext)` (prototype delegation): distinct instance triggers the lazy `Config.getFileReadCache()` to give the subagent its own read cache, so prior-read enforcement on the parent's mutation paths is not silently weakened.
    - A fresh tool registry via `rebuildToolRegistryOnOverride()` so core tools (`EditTool`/`WriteFileTool`/`ReadFileTool`) resolve `this.config` to the subagent — without this rebuild, the parent's cached tool instances still reach the parent's FileReadCache.
    - When `mcpServers` is set, a merged MCP server set (`{...sessionServers, ...agentServers}`) anchored on the override; parallel discovery via `discoverToolsForServer()` for any per-agent servers.
    - The parent's exact generation config snapshot when the `model` selector resolves to another authType (`fast` selector → a dedicated ContentGenerator built by `buildRuntimeContentGeneratorView()`).
    - The per-agent `hooks` are registered against the session's `HookRegistry` under an `agent:<name>:<randomUUID>` scope at spawn, removed by the caller's `dispose` callback in a `finally` block. **v1 limitation**: hook entries fire globally for every matching event in the session, not only for that subagent's own tool calls — proper per-agent scope filtering is deferred.

    The child does NOT receive the parent's conversation history, prior skill invocations, or files already read by the parent. The exception is `fork` (`subagent_type: "fork"`), which uses `CacheSafeParams` to inherit the parent's exact API request prefix (system prompt, tools, conversation history). Forks share the parent's prompt cache prefix for cost-efficient parallel execution.

    The child's only return value to the parent is a final assistant message. The internal transcript (subagent tool calls, intermediate reasoning, the full conversation) stays inside the child session and is queryable through the persisted JSONL transcript at the path delivered to the `SubagentStop` hook as `agent_transcript_path`. Foreground subagents return their final text to the parent tool result; background subagents (`run_in_background: true`) keep running and the parent can send a follow-up through the `monitor` tool or through the Agent tool's external-input queue.
  permissions_inheritance: |
    Resolved by `agent.ts:resolveSubagentApprovalMode(parentApprovalMode, agentApprovalMode, isTrustedFolder)`:
    1. Permissive parent modes (`YOLO`, `AUTO_EDIT`, `AUTO`) always win — the child's `approvalMode` frontmatter is ignored.
    2. `bubble` (subagent-only) resolves to `Default` run behavior; the difference is only the background-launch path, which surfaces the confirmation to the parent session instead of auto-denying.
    3. Otherwise, the agent's `approvalMode` frontmatter applies. In untrusted folders, privileged modes (`yolo`, `auto-edit`, `auto`) are downgraded to the parent's mode to prevent a repo-defined subagent from opting itself into classifier-mediated automation.
    4. When the agent's `approvalMode` is omitted: `plan` parent → child stays in `plan`; default mode in a trusted folder → child gets `auto-edit` for autonomy; otherwise child inherits the parent's mode.

    The bridge from Claude Code's `permissionMode` to Qwen's `approvalMode` lives in `agent-frontmatter-schema.ts:claudePermissionModeToApprovalMode`:
    - `default` → `default`
    - `plan` → `plan`
    - `acceptEdits` → `auto-edit`
    - `auto` → `auto-edit`
    - `bypassPermissions` → `yolo`
    - `dontAsk` → `default` (preserves the restrictive intent)

    When both `permissionMode` and `approvalMode` are set in frontmatter, `approvalMode` wins (more specific to qwen-code) and `permissionMode` is dropped from the persisted file. Tool-call permission rules from the parent's `permissions` settings still apply — the subagent cannot escalate by sending messages.
  model_inheritance: |
    Resolved by `SubagentManager.resolveModelOverride(model, runtimeContext)`:
    - Omitted or `inherit` → the child uses the main conversation's model. Explicit `inherit` is optional because the absence of the field is equivalent.
    - `fast` → uses `runtimeContext.fastModel` (the session-level fast model setting). If `fastModel` resolves to a different authType, Qwen creates a dedicated ContentGenerator for the subagent via `buildRuntimeContentGeneratorView()` so the parent's authType is unaffected.
    - `<model-id>` (e.g. `qwen3-coder-plus`, `glm-5`) → uses the given model with the main conversation's authType.
    - `<authType>:<model-id>` (e.g. `openai:deepseek-v4-flash`) → uses the given authType and model id directly.
    - Unresolvable selectors (model not registered under any configured authType) → falls back to inherit.

    Per-agent `model` is a hard override on the parent session model. When a subagent uses a different authType, the subagent's tool registry and FileReadCache are still anchored on the per-agent Config override (separate isolation), but the API request goes through the dedicated ContentGenerator.
  tool_inheritance: |
    The base layer is the parent's available tool set. The subagent's `tools` is an allowlist; `disallowedTools` is a denylist applied after the allowlist. Both may contain MCP tools, and `disallowedTools` accepts MCP server-level patterns (`mcp__server` removes all tools from that server, `mcp__server__tool` removes a single tool). When `tools` is omitted, the child inherits all tools the parent has access to, then `disallowedTools` is removed from that set.

    For per-agent MCP server overrides: the frontmatter `mcpServers` record is shallow-merged with the session's MCP server set on key collision, agent's spec wins (matching CC's `scope: 'agent'` semantics). The merged set is anchored on the per-agent Config override, and the freshly rebuilt tool registry discovers each per-agent server in parallel via `Promise.allSettled` so a single bad server cannot block the others.

    The `EXCLUDED_TOOLS_FOR_SUBAGENTS` constant (defined in `packages/core/src/agents/runtime/agent-core.ts`) hardcodes a floor that is always removed from subagent tool pools regardless of what `tools` lists. The floor typically includes `SEND_MESSAGE` and `EXIT_PLAN_MODE`. Workflow PR #4732 plans to add `WORKFLOW` to that floor for recursive-fanout guards.
  max_turns: |
    Two layers. Top-level `maxTurns` (positive integer, accepts numeric string per DL7 `W46`) is the canonical form, bridged into `runConfig.max_turns` at parse time by `SubagentManager.convertToRuntimeConfig()` ("Top-level CC-style `maxTurns` wins over legacy nested `runConfig.max_turns`. Both are kept for backward compatibility, but when both are set, the top-level field is the authoritative source."). When the agent omits the field entirely, the child iterates until it returns, hits an error, or exceeds the parent's `general.chatRecording`-gated compaction threshold — no default cap. The forked subagent has its own default `FORK_DEFAULT_MAX_TURNS` (see `packages/core/src/tools/agent/fork-subagent.ts`).
  notes: |
    Concurrency: multiple subagents can run in parallel because the Agent tool can be called multiple times in one assistant turn; the Agent tool's `run_in_background: true` decouples each spawn from the parent's tool-loop step. The fork path is background-only by design.

    Nesting: subagents can spawn nested subagents via `agent()` (the parenthesized `Agent(worker, researcher)` allowlist from Claude Code does NOT have a Qwen equivalent — Qwen uses `permission.deny` and `permission.allow` rules on the parent's settings). The `EXCLUDED_TOOLS_FOR_SUBAGENTS` floor plus the `agent-team` recursion guard prevent fan-out runaway. The `model.maxSubagentDepth` setting caps nesting depth (1-100, normalized); the persisted `AgentPersistedCliFlags.maxSubagentDepth` survives resume via the `cliFlags` sidecar at `<projectDir>/agents/<agentId>/meta.json` (see `packages/core/src/agents/agent-transcript.ts`).

    Selection: automatic delegation is driven by the task prompt plus the subagent's `description` field. To make a subagent a strong candidate, the description should include phrases like "use proactively" or "must be used".

    Disabling: there is no per-agent disable flag in v1; removing the file or moving it out of the discovery scope is the only way to remove an agent. Built-in agents cannot be removed by the user. Safe mode (`--safe-mode` / `QWEN_CODE_SAFE_MODE`) overrides the listed levels to `['builtin']` only, dropping all user/project/extension agents; bare mode (`--bare`) loads the cache with no agents.

    Failure: a non-throwing agent termination mode (`ERROR`, `MAX_TURNS`, `TIMEOUT`) maps to a failed subagent span with `error`/`errorType` populated; throwing exceptions are caught and mapped to OTel exception attributes. The transcript records the `terminateMode` on `SubagentStop` and on the per-agent span end.

    Resume: a stopped subagent's persisted state lives at `<projectDir>/agents/<agentId>/meta.json` and `<projectDir>/agents/<agentId>/transcript.jsonl` (see `getAgentJsonlPath`, `getAgentMetaPath`, `patchAgentMeta`). Resumption is via the `monitor` tool's external-input queue (`enqueue({ kind: 'notification', text })` and `enqueue({ kind: 'message', text })` paths) or by re-invoking `agent(...)` with the same `subagent_type`. There is no `claude --resume --agent` equivalent in v1.

observability:
  stream_events:
    - "AgentToolInvocation eventEmitter fires AgentEventType.START (status: 'running')"
    - "AgentEventType.TOOL_CALL (per tool invocation: callId, name, status: 'executing', args, description)"
    - "AgentEventType.TOOL_RESULT (per tool return: callId, status: 'success'|'failed', error, responseParts)"
    - "AgentEventType.FINISH (terminateReason: 'GOAL' → 'completed', else 'failed')"
    - "AgentEventType.ERROR (terminateReason: <error message>)"
    - "AgentEventType.USAGE_METADATA (accumulated output tokens via candidatesTokenCount)"
    - "AgentEventType.TOOL_WAITING_APPROVAL (carries confirmationDetails bridged to UI inline prompt)"
    - "Subagent span emitted via startSubagentSpan / endSubagentSpan with metadata { status, terminateReason, error, errorType, resultSummaryPresent }"
  hook_events:
    - "SubagentStart (matcher targets agent_type: built-in name like 'general-purpose' or 'Explore', custom name, or fork)"
    - "SubagentStop (matcher targets agent_type the same way)"
    - "PreToolUse / PostToolUse / PostToolUseFailure / PermissionRequest (fire inside subagent tool calls)"
    - "Stop (subagent frontmatter `hooks.Stop`; v1 fires globally for the subagent's lifetime)"
  session_ids: true
  notes: |
    Subagent hooks carry a stable `agent_id` (random UUID assigned at spawn) and an `agent_type` (the agent's frontmatter `name`, or the literal `fork` for fork pseudo-types). The `SubagentStart` payload adds `permission_mode` (resolved approval mode string). The `SubagentStop` payload adds `agent_transcript_path` (JSONL transcript at `<projectDir>/agents/<agentId>/transcript.jsonl`), `last_assistant_message` (final assistant text), `stop_hook_active` (boolean), and `permission_mode` (same as start).

    The matcher semantics on `SubagentStart`/`SubagentStop` are agent-type-string matchers with regex (per `matchesAgentType`). `*` matches every agent type; a literal matches exactly; a regex pattern is evaluated via `new RegExp(matcher).test(agentType)`. Matchers are matched against the agent's `name` field for normal agents and the literal `fork` for fork pseudo-types.

    Per-agent frontmatter `hooks` (the CC bridge field) registers ephemeral entries against the session's `HookRegistry` under an `agent:<name>:<randomUUID>` scope. The v1 documented limitation is that hook entries fire globally for every matching event in the session, not only for that subagent's own tool calls — proper per-agent scope filtering at hook-firing time is deferred. The SubagentManager calls `hookRegistry.addAgentHooks(config.hooks, agentScope)` at spawn and returns an `unregisterAgentHooks` callback that runs from `dispose` in the caller's `finally` block (idempotent — second call is a no-op via the agentScope filter).

    The persisted subagent metadata lives at:
    - `<projectDir>/agents/<agentId>/meta.json` — patchable via `patchAgentMeta(metaPath, { status, lastUpdatedAt, lastError })` with persisted states `running` | `cancelled` | `completed` | `failed`.
    - `<projectDir>/agents/<agentId>/transcript.jsonl` — written via `attachJsonlTranscriptWriter()` for live-streamed events; read by resume / monitor code.
    - Sidecar: `<projectDir>/agents/<agentId>/cli-flags.json` — captures `AgentPersistedCliFlags` for resume (approval mode, bare, safeMode, sandbox, screenReader, model, maxSessionTurns, maxToolCalls, maxSubagentDepth).

portability:
  portable: false
  non_portable_assets:
    - "`approvalMode` values (`default`/`plan`/`auto-edit`/`yolo`/`bubble`) — Qwen-specific permission vocabulary"
    - "`bubble` subagent-only mode (surfaces confirmation to parent in background runs)"
    - "`permissionMode` mapping table (`acceptEdits`→`auto-edit`, `auto`→`auto-edit`, `bypassPermissions`→`yolo`, `dontAsk`→`default`) — CC bridge"
    - "`fast` model selector (resolves to session `fastModel` setting; supports cross-authType fastModel)"
    - "`<authType>:<model-id>` selector syntax (resolves through Qwen's `resolveModelId`)"
    - "`tools` allowlist with Qwen tool names (`read_file`, `glob`, `grep_search`, `edit`, `write_file`, `run_shell_command`, `ask_user_question`, `memory`, `skill`, etc.)"
    - "`disallowedTools` MCP patterns (`mcp__server`, `mcp__server__tool`)"
    - "`mcpServers` frontmatter (CC 2.1.168 schema: stdio/http/sse/ws; Qwen's MCP loader owns per-spec validation)"
    - "`hooks` frontmatter (CC `TKO` schema; Qwen's SessionHooksManager owns the discriminated union for command/http/function/prompt)"
    - "`color` allowlist `_Y` (CC-internal field; Qwen mirrors CC's enum plus legacy `auto` sentinel)"
    - "`runConfig.max_time_minutes` / `runConfig.max_turns` (legacy nested form)"
    - "Built-in agent types `general-purpose`, `Explore`, `statusline-setup` (no equivalents on other providers; Qwen's Explore is read-only by convention)"
    - "`subagent_type: 'fork'` pseudo-type (Qwen inherits the parent's prompt cache; CC has the same)"
    - "Body prompt text that references Qwen tool names, `$QWEN_*` env vars, `${QWEN_SESSION_ID}`-style substitutions, or `QWEN.md` memory files"
    - "`max_turns` validator floor at 100 with soft warning above; `EXCLUDED_TOOLS_FOR_SUBAGENTS` floor that always strips `SEND_MESSAGE`/`EXIT_PLAN_MODE` (planned `WORKFLOW`)"
    - "Extension-contributed SubagentConfig records (depend on Qwen's extension loader and `qwen-extension.json` manifest)"
    - "Built-in agents hardcoded in BuiltinAgentRegistry (no on-disk equivalent; cannot be exported)"
  rewrite_needed: true
  notes: |
    The Markdown body — the agent's purpose and instructions — is provider-neutral and can be lifted as-is. The required `name` and `description` are also portable across providers when the destination provider accepts the same identifier grammar (Qwen requires `/^[\p{L}\p{N}_-]+$/u`, 2-50 chars; CC requires lowercase letters and hyphens; OpenCode uses any filename stem).

    A safe cross-provider rewrite preserves `name` (subject to target's identifier grammar), `description`, the body Markdown, and the high-level intent of any `model` selector (mapped to the target's selector vocabulary). It must drop or remap:
    - `approvalMode` ↔ target's permission mode vocabulary
    - `permissionMode` (CC bridge; map to target's bridge)
    - `tools` / `disallowedTools` (Qwen tool names map to target's tool identifiers; MCP patterns may differ)
    - `mcpServers` (Qwen shape → target's MCP shape; per-spec union may differ)
    - `hooks` (CC `TKO` shape → target's hooks shape)
    - `color`, `background`, `runConfig.max_time_minutes` (Qwen-specific)
    - The `bubble` mode (subagent-only)
    - The `fast` and `<authType>:<model-id>` model selectors (target-specific)

    Plugin/extension-packaged agents are particularly hard to port because they depend on Qwen's extension loader, `qwen-extension.json` manifest, and the source-confirmed `extension.agents` array contract.

    The CC compatibility bridge (`permissionMode`, `mcpServers`, `hooks`, `maxTurns` top-level) means a `.claude/agents/<name>.md` file dropped into `.qwen/agents/` parses identically — this is the explicit design intent per docs/declarative-agents-port.md §D1 ("Reuse the existing yaml-parser for frontmatter") and §D7 ("permissionMode vs approvalMode — bridge, don't replace"). Reverse-direction linking requires the inverse bridge for Claude-style metadata that does not have a Qwen equivalent (e.g. CC's `isolation: 'worktree'` runtime, `skills` preload, `memory` scopes, `initialPrompt`, `effort`) — five fields are deferred in qwen-code and carried as no-op metadata today.

cli_params:
  - flag: --approval-mode <plan|default|auto-edit|auto|yolo>
    description: "Sets the session-wide approval mode. Permissive modes (`yolo`, `auto-edit`, `auto`) cascade into every spawned subagent — the child's `approvalMode` frontmatter is overridden. Restrictive modes (`plan`, `default`) honor the child's frontmatter unless the child is in an untrusted folder."
    example: "qwen --approval-mode auto-edit"
  - flag: --yolo
    description: "Alias for `--approval-mode=yolo`. Auto-approves all tool calls; cascades to all subagents."
    example: "qwen --yolo"
  - flag: --bare
    description: "Minimal mode. SubagentManager.refreshCache() loads no project, user, extension, or builtin agents — `listSubagents()` returns the cached empty set. Affects agents, skills, hooks, plugins, MCP, auto-memory, and CLAUDE.md-equivalent."
    example: "qwen --bare"
  - flag: --safe-mode / QWEN_CODE_SAFE_MODE
    description: "Restricts SubagentManager.refreshCache() to `['builtin']` only — project, user, and extension agents are not loaded even if their files exist."
    example: "qwen --safe-mode"
  - flag: --allowed-tools "<csv>"
    description: "Comma-separated tool names that bypass the confirmation dialog. For subagents this changes the parent's available tool surface; the child still narrows further via its own `tools` allowlist."
    example: "qwen --allowed-tools \"run_shell_command,edit,write_file\""
  - flag: --extensions <name>[,<name>...]
    description: "Restricts the session to named extensions. With `--extensions none`, all extensions are disabled, so extension-scope agents do not participate. With `--extensions <name>`, only that extension's `agents[]` array loads."
    example: "qwen --extensions my-extension"
  - flag: --list-extensions
    description: "Lists available extensions and exits; useful for enumerating extension sources that may contribute agents."
    example: "qwen --list-extensions"
  - flag: --include-directories <dir> / --add-dir <dir>
    description: "Adds workspace directories for context discovery. Subagent project-root discovery is rooted at the configured project root, not at added directories."
    example: "qwen --include-directories ../shared"
  - flag: --acp
    description: "Enables ACP mode (Agent Client Protocol). Stable; replaces deprecated `--experimental-acp`. Subagent tool calls are exposed through the ACP framework."
    example: "qwen --acp"
  - flag: --experimental-lsp
    description: "Enables the LSP tool. Subagents with `tools: [ToolNames.LSP]` get language-server features when enabled."
    example: "qwen --experimental-lsp"
  - flag: --telemetry / --telemetry-target / --telemetry-otlp-endpoint / --telemetry-otlp-protocol / --telemetry-log-prompts
    description: "Telemetry flags. Subagent spans are emitted via OpenTelemetry and include `startSubagentSpan`/`endSubagentSpan` calls. Per the subagent trace tree design, spans carry SubagentSpanMetadata with status, terminateReason, error, errorType, and resultSummaryPresent."
    example: "qwen --telemetry --telemetry-target otlp --telemetry-otlp-endpoint http://localhost:4317"
  - flag: qwen extensions install <source> [--scope user|project|workspace]
    description: "Installs an extension that may contribute agents via its `qwen-extension.json` manifest's `agents` field. `--scope user` → `~/.qwen/extensions/<name>/`; `--scope project` → `<projectRoot>/.qwen/extensions/<name>/`."
    example: "qwen extensions install @scope/my-extension --scope project"
  - flag: qwen extensions uninstall <name>
    description: "Removes an installed extension and therefore any agents it contributed."
    example: "qwen extensions uninstall my-extension"
  - flag: /agents create
    description: "Interactive slash command that creates a new subagent through a guided wizard. Source-confirmed via the SubAgents documentation page."
    example: "/agents create"
  - flag: /agents manage
    description: "Opens the agent management dialog for viewing and managing existing subagents (list, edit, delete, run). Triggers `SubagentManager.refreshCache()` for in-session updates."
    example: "/agents manage"
  - flag: /agents (slash)
    description: "Alias for `/agents manage`. Source-confirmed in docs/users/features/sub-agents.md."
    example: "/agents"

env_vars:
  - name: QWEN_HOME
    effect: "Overrides the global Qwen config directory. Resolves the user-scope subagent location from `~/.qwen/agents/` to `$QWEN_HOME/agents/`. Project and extension scopes are unaffected (extensions stay under `<projectRoot>/.qwen/extensions/`)."
  - name: QWEN_CODE_SAFE_MODE
    effect: "Truthy value enables safe mode. SubagentManager.refreshCache() loads only the `builtin` level; user, project, and extension agents are not registered. Equivalent to `--safe-mode`."
  - name: QWEN_CODE_ENABLE_AGENT_TEAM
    effect: "When `1`, enables the Agent Team feature (`team_create`, `task_create`, `send_message`, etc.) and the `name` / `plan_mode_required` Agent tool parameters. Requires restart."
  - name: experimental.agentTeam
    effect: "Settings.json key under `experimental.agentTeam`. Equivalent to QWEN_CODE_ENABLE_AGENT_TEAM but persistent; restart required."
  - name: QWEN_RUNTIME_DIR
    effect: "Overrides the runtime output directory (per-session chat logs, the `agents/<agentId>/meta.json` and `agents/<agentId>/transcript.jsonl` files, agent teams data). Does not relocate agent definitions, but affects where the per-agent persisted metadata is written."
  - name: QWEN_CODE_SYSTEM_SETTINGS_PATH
    effect: "Overrides the system settings file path. System settings can include `experimental.agentTeam`, which controls whether `name` and `plan_mode_required` are advertised to the Agent tool."
  - name: QWEN_CODE_SYSTEM_DEFAULTS_PATH
    effect: "Overrides the system defaults settings file path. Same effect on agent-team behavior."
  - name: NPM_TOKEN
    effect: "Used by `qwen extensions install` from npm. Affects whether private extension packages (and their bundled agents) can be fetched."
  - name: CLAUDINE_OPENCODE_STALL_TIMEOUT (not Qwen)
    effect: "Not applicable — this is a Claudine-side timeout for OpenCode. Listed here to clarify that Qwen Code does not currently publish its own agent-level timeout env var beyond `runConfig.max_time_minutes` (frontmatter-only)."

changes: []

requires_claudine_update: true
reason: |
  Claudine's `linking/agents.rs` module currently knows Claude Code subagents (`~/.claude/agents/`, `.claude/agents/`, plugin `agents/`, `--agents` inline) and OpenCode subagents (`~/.config/opencode/agents/`, `.opencode/agents/`, `opencode.json` `agent.<name>` block). It does NOT yet enumerate Qwen Code's surface: `~/.qwen/agents/<name>.md` (with QWEN_HOME redirect), `.qwen/agents/<name>.md` (under project root, no `.agents/agents/` alias confirmed), extension-contributed SubagentConfig records (NOT file-based — an `agents[]` array on the extension object), built-in agents (constants in BuiltinAgentRegistry, not on disk), and session-injected agents (via `SubagentManager.loadSessionSubagents()`, not on disk). A future Qwen agent-listing feature needs a sibling that walks these surfaces.

  Two important contrasts with the Claude Code and OpenCode linkers:
  1. Qwen's resolution order is `session > project > user > extension > builtin` (NOT CC's `policySettings > flagSettings > projectSettings > userSettings > plugin > built-in`). There is no `--agent <name>` CLI flag yet (planned P3 in docs/declarative-agents-port.md §D5), no `--agents <json>` flag (deferred P4), and no managed/system policy directory. The linker should not look for them.
  2. The 16-field schema is the Claude Code 2.1.168 schema ported verbatim (per `docs/declarative-agents-port.md` and `packages/core/src/subagents/agent-frontmatter-schema.ts:PERMISSION_MODE_VALUES`, `COLOR_VALUES`, `parseMaxTurns`, `parseAgentMcpServers`, `parseAgentHooks`). A linked `.claude/agents/*.md` file lands as a valid Qwen subagent today; a linked `.qwen/agents/*.md` file with `approvalMode` (Qwen-only) requires CC-compatibility-mode loss when sent back to CC. The bridge `permissionMode` → `approvalMode` is documented in the same file (`claudePermissionModeToApprovalMode`).

  For lifecycle `proxy`/`resume`: Qwen emits `SubagentStart` / `SubagentStop` hook events with a stable `agent_id` (UUID) and `agent_type` (the agent's `name` field, or the literal `fork` for fork pseudo-types). The `SubagentStop` payload also carries `agent_transcript_path` (JSONL at `<projectDir>/agents/<agentId>/transcript.jsonl`) and `last_assistant_message`. The persisted metadata sidecar at `<projectDir>/agents/<agentId>/meta.json` and the per-agent cli-flags sidecar are the resume surface. A wrapper that wants to address a specific subagent must capture the `agent_id` from `SubagentStart` and replay through the persisted metadata. Fork children cannot be resumed (results never come back to the parent); the parent's only awareness is the placeholder completion notification. Per the design doc, fork recursion is blocked at runtime (`isInForkChild()` scans for `<fork-boilerplate>` and rejects spawn attempts).
---

# Qwen Code Subagents

## Overview

Qwen Code treats user-defined **subagents** as a first-class feature: durable Markdown files with YAML frontmatter that change who does work inside a session. The provider calls them "SubAgents" (the docs headline) or "subagents" (the in-source module name `packages/core/src/subagents/`); the internal data model is `SubagentConfig`. The Agent tool (`packages/core/src/tools/agent/agent.ts`) is the delegation surface — it accepts a `subagent_type`, resolves it through `SubagentManager.loadSubagent()`, builds a fresh `AgentHeadless` via `SubagentManager.createAgentHeadless()`, and returns the final assistant text. A separate fork pseudo-type (`subagent_type: "fork"`) inherits the parent's full conversation context and shares prompt-cache prefix. Support is `first_class`: there are five scopes (session, project, user, extension, builtin), a documented 16-field frontmatter schema ported from Claude Code 2.1.168, runtime delegation semantics through `AgentTool`, isolated child sessions with their own FileReadCache and tool registry, and observable start/stop lifecycle via `SubagentStart` / `SubagentStop` hook events with stable `agent_id` per spawn.

This topic is the *definition* of subagents — where the files live, what frontmatter they accept, how the parent picks one, what context and permissions the child gets, and how a wrapper can observe starts/stops. The hooks topic owns lifecycle event semantics (this document records only which events expose agent lifecycle and what payload fields they carry). The plugins topic's `packaged_resources` records containment only — agent definitions packaged inside extensions still have their semantics documented here.

The Qwen Code subagent system is the Claude Code declarative-agents schema with adjustments for Qwen's permission vocabulary (`approvalMode` over `permissionMode`), tool identifiers (Qwen tool names like `read_file` / `run_shell_command` / `glob` / `grep_search`), model selection (`<authType>:<model-id>` selectors that resolve through `resolveModelId`), and storage layout (`~/.qwen/` rather than `~/.claude/`). The schema is intentionally byte-for-byte compatible with `.claude/agents/*.md` for the eight fields Qwen supports in v1; the bridge lives in `packages/core/src/subagents/agent-frontmatter-schema.ts`. Five CC fields are deliberately deferred (`effort`, `skills`, `initialPrompt`, `memory`, `isolation`) and carried as no-op metadata until the prerequisite subsystems land.

## Locations

Definitions are stored by scope. Qwen uses the in-source `SubagentLevel` enum (`packages/core/src/subagents/types.ts`) with five levels. The discovery order is `session > project > user > extension > builtin` (highest precedence first) — confirmed by `SubagentManager.loadSubagent()` and the listing precedence in `listSubagents()`:

| Scope | macOS | Linux | Windows | Notes |
|---|---|---|---|---|
| Session | runtime injection via `SubagentManager.loadSessionSubagents()` | same | same | SDK/programmatic API; not on disk. Cache holds them under `subagentsCache.get('session')` with `filePath: '<session:<name>>'`. Read-only. Highest precedence. |
| Project | `<projectRoot>/.qwen/agents/<name>.md` | same | `.qwen\agents\<name>.md` | Discovered at session start; not hot-reloaded. Project root matching the home directory is treated as no-project (returns empty list). |
| User | `~/.qwen/agents/<name>.md` | same | `%USERPROFILE%\.qwen\agents\<name>.md` | Resolved via `Storage.getGlobalQwenDir()`; `QWEN_HOME` redirects to `$QWEN_HOME/agents/<name>.md`. Symlinks are followed (observed on this host: three symlinks pointing to Claude-authored agents in `~/.claude/agents/`). |
| Extension | `<extension-root>/agents/<name>.md` (or extension-defined) | same | same | Extensions contribute agents through their `agents` property (an array of `SubagentConfig` records), not necessarily files on disk. The path is extension-defined; Qwen's CLI does not pin a directory layout. Active extensions only. |
| Built-in | hardcoded in `BuiltinAgentRegistry` (packages/core/src/subagents/builtin-agents.ts) | same | same | Three built-ins: `general-purpose` (default), `Explore`, `statusline-setup`. NOT on disk in the user's environment. `filePath` is the literal `<builtin:<name>>`. Cannot be modified or deleted. Lowest precedence. |
| Fork pseudo-type | n/a | n/a | n/a | `subagent_type: "fork"` is a built-in pseudo-type with no on-disk definition. Resolved via `fork-subagent.ts` against the parent's prompt cache. |

`Storage.getGlobalQwenDir()` (in `packages/core/src/config/storage.ts`) is the resolution root for user-level paths. `QWEN_HOME` overrides it; `QWEN_RUNTIME_DIR` does NOT relocate agent definitions (it only moves runtime output paths like `<runtimeDir>/projects/<project-hash>/chats/<sessionId>.runtime.json` and `<runtimeDir>/agents/<agentId>/meta.json`).

There is no on-disk analog of CC's managed-system policy directory (`/Library/Application Support/ClaudeCode/.claude/agents/`, `/etc/claude-code/.claude/agents/`, `C:\Program Files\ClaudeCode\.claude\agents\`). There is no `.agents/agents/` alias like the one Qwen uses for skills (where `SKILL_PROVIDER_CONFIG_DIRS = ['.qwen', '.agents']`); subagents are `.qwen/agents/` only.

On this host (macOS), `~/.qwen/agents/` contains three entries — `feature-tester-rust.md`, `feature-tester-typescript.md`, and `tester-agent.md` — all symlinks to `~/.claude/agents/*.md` files. The Qwen loader follows the symlinks and parses each file's frontmatter; the loader does not care that the files were authored for Claude. The `/agents create` and `/agents manage` slash commands list these via `SubagentManager.listSubagents()`.

## Definition Format

A subagent file is a single Markdown file with YAML frontmatter between `---` markers, where the body becomes the subagent's `systemPrompt`. Identity comes from the frontmatter `name` field, not the filename. Files outside the project/user/extension roots, or files whose required fields fail strict validation, are skipped with a warning (`warnInvalidSubagentFile` in `subagent-manager.ts`).

### Frontmatter schema

The 16-field schema mirrors Claude Code 2.1.168 verbatim, with `approvalMode` (qwen-specific) added as a sibling to `permissionMode` (CC bridge):

```yaml
---
name: code-reviewer
description: Reviews code for quality and best practices. Use proactively after non-trivial code changes.
tools:
  - read_file
  - glob
  - grep_search
  - run_shell_command
disallowedTools:
  - write_file
  - edit
model: fast
approvalMode: auto-edit
permissionMode: plan
runConfig:
  max_time_minutes: 10
  max_turns: 50
maxTurns: 50
color: cyan
background: false
mcpServers:
  filesystem:
    type: stdio
    command: node
    args: [/usr/local/lib/mcp-fs/server.js]
hooks:
  PreToolUse:
    - matcher: Bash
      hooks:
        - type: command
          command: echo "review-agent about to run a shell command"
---

You are a code reviewer focused on quality, security, and best practices.
For each issue, explain the problem, quote the offending code, and propose a
minimal patch. Prefer small, focused notes over broad rewrites.
```

The same shape also works inline via the Claude Code `--agents '<json>'` style — Qwen does not have a CLI equivalent yet (planned P4 in the declarative-agents port).

### Recognized fields

| Field | Required | Behavior |
|---|---|---|
| `name` | Yes | Validated by `SubagentValidator.validateName` — must match `/^[\p{L}\p{N}_-]+$/u`, length 2..50, not starting/ending with `-` or `_`, not a reserved word (`self`, `system`, `user`, `model`, `tool`, `config`, `default`, `main`). Soft warning on case mixing and on mixing hyphens with underscores. Case-insensitive lookup. |
| `description` | Yes | Non-empty string, ≤1000 chars (soft warning at the limit). Drives automatic delegation. |
| `tools` | No | Allowlist of tool names; empty array means inherit-all. Accepts comma-separated string. Resolved by `transformToToolNames` against the parent's tool registry (exact match first, then display name, then preserved as-is with a warn). |
| `disallowedTools` | No | Denylist of tool names; supports `mcp__server` and `mcp__server__tool` MCP patterns. Applied after the `tools` allowlist. |
| `model` | No | Selector: `inherit` (default), `fast`, `<model-id>`, or `<authType>:<model-id>`. Resolved by `resolveModelOverride` against the session's `modelProviders` registry; unresolvable values fall back to `inherit`. |
| `approvalMode` | No | One of `default`, `plan`, `auto-edit`, `yolo`, `bubble`. `bubble` is subagent-only. When set, wins over `permissionMode`. |
| `permissionMode` | No | CC bridge: `acceptEdits`, `auto`, `bypassPermissions`, `default`, `dontAsk`, `plan`. Mapped to `approvalMode` via `claudePermissionModeToApprovalMode`. Drops with warn on invalid values. |
| `runConfig.max_time_minutes` | No | Positive integer ≤60 (soft warning at the limit). |
| `runConfig.max_turns` | No | Positive integer ≤100 (soft warning at the limit). Legacy nested form; top-level `maxTurns` wins when both set. |
| `maxTurns` | No | Top-level promotion of `runConfig.max_turns`. Positive integer; accepts numeric string per CC `W46`. Invalid values dropped with warn. |
| `mcpServers` | No | Record of `{ name: { type, command, args, ... } }` MCP server specs. Shallow-merged with session-level servers; per-agent wins on key collision. Per-spec discriminated union validated by the runtime MCP loader. |
| `hooks` | No | Record of `{ HookEventName: HookMatcher[] }`. Per-agent hooks registered at spawn against the session's `HookRegistry` under `agent:<name>:<randomUUID>`. v1 fires globally for matching events (not per-agent scoped). |
| `color` | No | Allowlist: `red`, `blue`, `green`, `yellow`, `purple`, `orange`, `pink`, `cyan` (CC `_Y`); legacy `auto` sentinel preserved. Values outside silently dropped with a warn. |
| `background` | No | Boolean (also accepts `"true"` / `"false"` strings). OR'd with the Agent tool's `run_in_background` parameter; if either is truthy, the subagent runs in the background. |

The Markdown body becomes the `systemPrompt` field on `SubagentConfig` verbatim after the frontmatter is stripped. No Markdown-only versus code-only separation; the entire trimmed body is the prompt. Embedded `${variable}` substitution is supported via `ContextState` at runtime; `$ARGUMENTS`-style placeholders are NOT honored (Qwen's agent frontmatter is more limited than its skill frontmatter).

## Runtime Behavior

The Agent tool is the delegation surface. Its schema (set by `AgentTool` in `packages/core/src/tools/agent/agent.ts`) accepts:

- `description` (required string, 3-5 word label)
- `prompt` (required string, the delegated task)
- `subagent_type` (optional string; defaults to `general-purpose` when omitted; `fork` is a deliberate pseudo-type)
- `run_in_background` (boolean; top-level sessions only)
- `isolation` (`worktree` only; spins up `<projectRoot>/.qwen/worktrees/agent-<7hex>`; auto-removed when no changes, preserved otherwise)
- `name` and `plan_mode_required` (only when `experimental.agentTeam` is on, for spawning teammates via TeamManager)

Three invocation paths reach a subagent:

1. **Explicit type**: `agent(subagent_type="<name>", prompt="...")` — the parent model picks the type. `subagent_type` matches `availableSubagents` case-insensitively against the frontmatter `name`.
2. **Implicit default**: `agent(prompt="...")` with no type — defaults to `general-purpose` (the canonical built-in name from `DEFAULT_BUILTIN_SUBAGENT_TYPE`).
3. **Fork**: `agent(subagent_type="fork", prompt="...", name="...")` — when `isForkSubagentEnabled(config)` returns true. The fork inherits the parent's full conversation history, system prompt, and tools; runs detached in the background; results are NOT returned to the parent (the parent gets a placeholder completion notification).
4. **TUI/CLI direct**: there is no `--agent <name>` CLI flag in qwen-code today (planned P3 in docs/declarative-agents-port.md §D5). When it lands, `qwen --agent code-reviewer` will select the agent as the main session agent, replacing the default system prompt unless the agent declares `appendSystemPrompt: true`.

The Agent tool dynamically refreshes its schema enum via `subagentManager.addChangeListener` whenever the agent registry changes. Built-in types are always advertised; the enum lists every registered `name` (lowercase canonical). When the description is built, a team-coordination paragraph is appended only when `experimental.agentTeam` is enabled.

The child receives:

- Its own frontmatter body as the `systemPrompt` (no Qwen system-prompt preamble, no `QWEN.md` memory loading — the body is the full system prompt).
- The delegated `prompt` as the task.
- A fresh `Config` override created via `Object.create(runtimeContext)` (prototype delegation). The distinct instance triggers the lazy `Config.getFileReadCache()` to give the subagent its own read cache, so prior-read enforcement on the parent's mutation paths is not silently weakened.
- A fresh tool registry via `rebuildToolRegistryOnOverride()` so core tools (`EditTool` / `WriteFileTool` / `ReadFileTool`) resolve `this.config` to the subagent — without this rebuild, the parent's cached tool instances still reach the parent's FileReadCache.
- When `mcpServers` is set, a merged MCP server set (`{ ...sessionServers, ...agentServers }`) anchored on the override; parallel discovery via `discoverToolsForServer()` for any per-agent servers.
- The parent's exact generation config snapshot when the `model` selector resolves to another authType (`fast` selector → a dedicated ContentGenerator built by `buildRuntimeContentGeneratorView()` so the parent's authType is unaffected).

The child does NOT receive the parent's conversation history, prior skill invocations, or files already read by the parent — except via the `fork` pseudo-type, which uses `CacheSafeParams` to inherit the parent's exact API request prefix (system prompt, tools, conversation history). Forks share the parent's prompt cache prefix for cost-efficient parallel execution; a fork cannot spawn another fork (`isInForkChild()` scans for `<fork-boilerplate>` and rejects spawn attempts).

The child's returned state to the parent is:

- **Foreground subagent**: the final assistant message returned through the Agent tool result; the `SubagentStop` `last_assistant_message` field carries the same text.
- **Background subagent**: nothing immediate. The parent can send follow-up through the `monitor` tool's external-input queue (`enqueue({ kind: 'notification', text })` and `enqueue({ kind: 'message', text })` paths), which routes into the subagent's pending input queue.
- **Fork**: nothing. Results are reflected in the UI progress display but are not fed back into the main conversation. The parent AI sees a placeholder message and cannot act on the fork's output.

The per-agent `hooks` frontmatter registers ephemeral entries against the session's `HookRegistry` under `agent:<name>:<randomUUID>` at spawn; the caller's `dispose` callback in a `finally` block unregisters them. **v1 limitation** (documented in the docs page): hook entries fire globally for every matching event in the session, not only for that subagent's own tool calls — proper per-agent scope filtering is deferred.

Permission mode is resolved by `resolveSubagentApprovalMode(parentApprovalMode, agentApprovalMode, isTrustedFolder)`:

1. Permissive parent modes (`YOLO`, `AUTO_EDIT`, `AUTO`) always win — the child's `approvalMode` frontmatter is ignored.
2. `bubble` (subagent-only) resolves to `Default` run behavior; the difference is only the background-launch path, which surfaces the confirmation to the parent session instead of auto-denying.
3. Otherwise, the agent's `approvalMode` frontmatter applies. In untrusted folders, privileged modes (`yolo`, `auto-edit`, `auto`) are downgraded to the parent's mode to prevent a repo-defined subagent from opting itself into classifier-mediated automation.
4. When the agent's `approvalMode` is omitted: `plan` parent → child stays in `plan`; default mode in a trusted folder → child gets `auto-edit` for autonomy; otherwise child inherits the parent's mode.

The bridge from Claude Code's `permissionMode` to Qwen's `approvalMode` lives in `agent-frontmatter-schema.ts:claudePermissionModeToApprovalMode`. When both `permissionMode` and `approvalMode` are set in frontmatter, `approvalMode` wins (more specific to qwen-code) and `permissionMode` is dropped from the persisted file.

The Agent tool surface reflects the resolved permission mode through the `permission_mode` field on both `SubagentStart` and `SubagentStop` hook payloads, so a wrapper sees what was actually applied at runtime rather than what was requested.

## Observability

Subagent starts and stops are visible to wrappers through three coordinated surfaces:

1. **Hook events** — `SubagentStart` (when the subagent is spawned) and `SubagentStop` (when the subagent completes or terminates) fire as configurable lifecycle events. The matcher targets `agent_type`: built-in names (`general-purpose`, `Explore`, `statusline-setup`), custom `name` values, or the literal `fork` for fork pseudo-types. Matchers are matched as agent-type-string matchers with regex (`*` matches all, literal matches exactly, `new RegExp(matcher).test(agentType)` for patterns). The full JSON input schema adds to the shared base fields:
   - `agent_id` — stable unique identifier (random UUID) assigned at spawn.
   - `agent_type` — the agent's frontmatter `name`, or the literal `fork` for fork pseudo-types.
   - `permission_mode` — the resolved approval mode string (one of `default`, `plan`, `auto-edit`, `auto`, `yolo`, or the qwen-specific PermissionMode enum names).
   - For `SubagentStop`: also `agent_transcript_path` (JSONL at `<projectDir>/agents/<agentId>/transcript.jsonl`), `last_assistant_message` (final assistant text), `stop_hook_active` (boolean indicating whether a stop-hook is currently firing in the subagent), and `permission_mode` (same as start).

2. **Stream output (live progress)** — the Agent tool's `eventEmitter` fires `AgentEventType.START`, `TOOL_CALL`, `TOOL_RESULT`, `FINISH`, `ERROR`, `USAGE_METADATA`, and `TOOL_WAITING_APPROVAL` events on the parent's session. With `--output-format stream-json`, these stream out as the subagent makes progress. The Qwen code uses `agentTool.setCallId(callId)` so background agents carry the tool-use id through to completion notifications.

3. **Telemetry spans** — `startSubagentSpan` / `endSubagentSpan` emit OpenTelemetry spans with `SubagentSpanMetadata` (`status`, `terminateReason`, `error`, `errorType`, `resultSummaryPresent`). The mapping from `AgentTerminateMode` (`GOAL`, `CANCELLED`, `SHUTDOWN`, `ERROR`, `MAX_TURNS`, `TIMEOUT`) to span status is documented in `deriveSubagentOutcomeMetadata`. Exception paths populate `error` and `errorType` so dashboards filtering on `exception.message` / `error.type` see meaningful signals.

4. **Persisted metadata** — the per-agent sidecar at `<projectDir>/agents/<agentId>/meta.json` is patchable via `patchAgentMeta(metaPath, { status, lastUpdatedAt, lastError })` with states `running`, `cancelled`, `completed`, `failed`. The companion `<projectDir>/agents/<agentId>/cli-flags.json` carries `AgentPersistedCliFlags` (`approvalMode`, `bare`, `safeMode`, `sandbox`, `screenReader`, `model`, `maxSessionTurns`, `maxToolCalls`, `maxSubagentDepth`) for resume. `general.cleanupPeriodDays` (default 30) controls retention.

The `SubagentStart` / `SubagentStop` matcher semantics use regex match on the agent-type string. `*` matches every agent type; a literal matches exactly; a regex pattern is evaluated via `new RegExp(matcher).test(agentType)`. Built-in types (`general-purpose`, `Explore`, `statusline-setup`) and the fork literal (`fork`) are common matcher targets.

## Portability

Subagents are **not portable** across providers as-is. The Markdown body — the bulk of the agent's *purpose* — is provider-neutral and can be lifted verbatim. The `description` field carries the routing signal across implementations and is also portable when the destination provider accepts the same identifier grammar. The rest of the frontmatter is provider-specific.

| Field | Portable? | Rewrite target |
|---|---|---|
| `name` | depends | Must match the target provider's identifier grammar (CC: lowercase letters and hyphens; Qwen: `/^[\p{L}\p{N}_-]+$/u`; OpenCode: any filename stem). |
| `description` | yes | Carries the routing signal across providers. |
| `body` | partial | Verbatim is fine when the body uses only standard Markdown and references provider-neutral tools. References to Qwen tool names (`read_file`, `run_shell_command`, `glob`, `grep_search`) must be rewritten to the target's vocabulary. References to `${QWEN_*}` env vars or `QWEN.md` memory files must be rewritten. |
| `tools` / `disallowedTools` | no | Remap to the target provider's tool identifiers and MCP patterns. |
| `model` | no | Remap to the target's selector vocabulary (`fast`, `<authType>:<model-id>` are Qwen-specific). |
| `approvalMode` | no | Remap to the target's permission mode set (`default`/`plan`/`auto-edit`/`yolo`/`bubble`). The `bubble` mode is Qwen-only. |
| `permissionMode` | partial | CC bridge — already in CC's vocabulary; loses Qwen-specific resolution behavior. |
| `runConfig` / `maxTurns` | partial | Legacy nested form vs top-level form differs. |
| `mcpServers` | partial | The shape is shared across Qwen and CC (per-spec union: `stdio`/`http`/`sse`/`ws`); other providers may differ. |
| `hooks` | partial | CC `TKO` shape — same as CC's settings.json hooks. Other providers use different schemas. |
| `color` | no | Allowlist is shared with CC (`_Y`) but the Qwen legacy `auto` sentinel is provider-specific. |
| `background` | no | Qwen-specific runtime hint. |
| `isolation` | no | Qwen carries it as no-op metadata in v1 (deferred to follow-up PR). |
| `effort` / `skills` / `initialPrompt` / `memory` | no | Qwen carries these as no-op metadata in v1; CC owns the runtime. |
| Built-in types (`general-purpose`, `Explore`, `statusline-setup`) | no | No equivalents on other providers. The CC equivalents (`general-purpose`, `Explore`, `Plan`, `statusline-setup`, `claude-code-guide`) overlap partially but not exactly. |
| `subagent_type: 'fork'` pseudo-type | no | Implemented as a sibling pseudo-type in CC and Qwen; semantics are close (parent-cache inheritance, fire-and-forget background) but wiring differs. |

A safe cross-provider rewrite preserves `description`, the body Markdown, and the high-level intent of any `model` selector (mapped to the target's selector vocabulary). It must drop or remap `tools` / `disallowedTools`, the entire permission vocabulary, `mcpServers`, `hooks`, `color`, `background`, `runConfig`, `maxTurns`, and the `bubble` mode. Body text that references Qwen tool names must be rewritten to the target provider's vocabulary.

The CC compatibility bridge makes `.claude/agents/*.md` files land in Qwen as valid subagents (per the docs/declarative-agents-port.md design intent). The reverse — sending a `.qwen/agents/*.md` file back to CC — requires the inverse bridge for Qwen-only metadata (`approvalMode`, `bubble`, `<authType>:<model-id>` selectors, `fast`).

## Claudine Linking Notes

For Claudine's `linking/agents.rs` module and the planned lifecycle `proxy`/`resume` actions, what matters about Qwen Code subagents:

- **Discovery surfaces to enumerate** for a Qwen agent linker:
  1. `~/.qwen/agents/<name>.md` (user-scope, `QWEN_HOME` redirects).
  2. `<projectRoot>/.qwen/agents/<name>.md` (project-scope). Note: no `.agents/agents/` alias confirmed (only skills have that alias via `SKILL_PROVIDER_CONFIG_DIRS`).
  3. Active extensions' `agents` property — NOT file-based; carried on the `Extension` object as `agents: SubagentConfig[]`. The on-disk layout is extension-defined; Qwen's CLI does not pin it.
  4. Session-level injected agents — programmatic only; not on disk; reachable through `SubagentManager.listSubagents({ level: 'session' })` after `loadSessionSubagents(...)` has been called.
  5. Built-in agents — `BuiltinAgentRegistry.getBuiltinAgents()` returns the three hardcoded entries (`general-purpose`, `Explore`, `statusline-setup`) with `level: 'builtin'` and `filePath: '<builtin:<name>>'`.
  6. The `/agents manage` slash command UI; the agent count surfaced through `SubagentManager.listSubagents()`.
- **Listing command**: there is no standalone `qwen agents list` CLI. Use `/agents manage` interactive or `SubagentManager.listSubagents()` programmatically. Same-name collisions follow `session > project > user > extension > builtin` precedence; the project scope overrides user, which overrides extension, which overrides builtin.
- **Portability classification**: classify Qwen subagents as **non-portable**. The Markdown body carries most of the agent's *purpose* and may be lifted to another provider's body, but the frontmatter is provider-specific (16 fields ported from CC, five of them currently no-op metadata, plus Qwen-specific `approvalMode`, `bubble`, `fast`, `<authType>:<model-id>` selectors). Flag assets that depend on `permissionMode`, `mcpServers`, `hooks`, `color`, `background`, `runConfig`, `maxTurns`, `approvalMode`, `bubble`, or the Qwen-specific model selectors as needing rewrite, stripping, or host gating before they can land on another provider.
- **CC compatibility bridge**: a `.claude/agents/*.md` file dropped into `.qwen/agents/` parses identically for the eight CC fields Qwen supports in v1 (`name`, `description`, `model`, `tools`, `disallowedTools`, `permissionMode`, `mcpServers`, `hooks`, `maxTurns`, `color`, `background`). Five CC fields are carried as no-op metadata today (`effort`, `skills`, `initialPrompt`, `memory`, `isolation`). When linking from Qwen back to CC, the inverse bridge is needed for `approvalMode` and `bubble`.
- **Lifecycle `proxy`/`resume`**: the stable identity is the per-spawn `agent_id` (UUID) paired with `agent_type`. To address a specific subagent from a wrapper:
  - Capture the `agent_id` from the `SubagentStart` hook payload when the subagent spawns; capture the `agent_type` for cross-referencing with `/agents manage` listings.
  - The persisted metadata sidecar at `<projectDir>/agents/<agentId>/meta.json` is the source of truth for current status (`running` / `cancelled` / `completed` / `failed`). The companion `<projectDir>/agents/<agentId>/transcript.jsonl` is the conversation replay surface.
  - The `<projectDir>/agents/<agentId>/cli-flags.json` sidecar carries `AgentPersistedCliFlags` (`approvalMode`, `bare`, `safeMode`, `sandbox`, `screenReader`, `model`, `maxSessionTurns`, `maxToolCalls`, `maxSubagentDepth`) — required to restore the session's behavior on resume.
  - Resumption is via the `monitor` tool's external-input queue (`enqueue({ kind: 'message', text })`) or by re-invoking `agent(...)` with the same `subagent_type`. Fork children cannot be resumed (their results never return to the parent).
- **Permission policy**: when the parent uses a permissive mode (`YOLO`, `AUTO_EDIT`, `AUTO`), the child inherits it and the child's `approvalMode` frontmatter is ignored. Otherwise the child's `approvalMode` applies, with trusted-folder gating for privileged modes. A wrapper that pre-loads its own approval policy should treat the parent's effective permission mode as the ceiling and the child's `approvalMode` as the default-with-overrides.
- **When `claudine qwen` grows a wrapper agent-resolution path**: model resolution should follow Qwen's `resolveModelOverride` ladder — `(1) agent's model selector → (2) session's fastModel when selector=fast → (3) session's main model → (4) QWEN_CODE_SAFE_MODE / --safe-mode defaults to no override`. The per-spawn `permission_mode` field in `SubagentStart` / `SubagentStop` payloads is the authoritative resolved value, not the agent's `approvalMode` frontmatter.

## Changelog

- **2026-07-03** — Initial research. First-run coverage of Qwen Code's subagent system, ported from Claude Code 2.1.168 declarative-agents schema per `docs/declarative-agents-port.md`. Verified against `packages/core/src/subagents/{types.ts, subagent-manager.ts, validation.ts, builtin-agents.ts, agent-frontmatter-schema.ts}`, `packages/core/src/tools/agent/{agent.ts, fork-subagent.ts}`, `packages/core/src/agents/runtime/agent-headless.ts`, and the shipped `cli.js` bundle on this host (qwen-code 0.15.6). Documented the five scope levels (session, project, user, extension, builtin), the 16-field frontmatter schema (with five CC fields currently deferred), the bridge from CC `permissionMode` to Qwen `approvalMode`, the Agent tool invocation paths, the fork pseudo-type, the `SubagentStart`/`SubagentStop` hook lifecycle, and the persisted metadata sidecars used for resume.

## Sources

- [Qwen Code overview](https://qwenlm.github.io/qwen-code-docs/en/users/overview/)
- [Qwen Code SubAgents documentation](https://qwenlm.github.io/qwen-code-docs/en/users/features/sub-agents/)
- [Qwen Code Agent Tool (`agent`) developer docs](https://qwenlm.github.io/qwen-code-docs/en/developers/tools/task/)
- [Qwen Code configuration settings](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/)
- [Qwen Code approval mode documentation](https://qwenlm.github.io/qwen-code-docs/en/users/features/approval-mode/)
- [Qwen Code Fork Subagent Design](https://qwenlm.github.io/qwen-code-docs/en/design/fork-subagent/fork-subagent-design/)
- [Qwen Code Subagent Trace Tree Design](https://qwenlm.github.io/qwen-code-docs/en/design/telemetry-subagent-spans-design/)
- [Qwen Code Declarative Agent Definitions port doc](https://qwenlm.github.io/qwen-code-docs/en/declarative-agents-port/)
- [Qwen Code repository](https://github.com/QwenLM/qwen-code)
- [Qwen Code SubagentConfig types source](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/subagents/types.ts)
- [Qwen Code SubagentManager source](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/subagents/subagent-manager.ts)
- [Qwen Code SubagentValidator source](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/subagents/validation.ts)
- [Qwen Code BuiltinAgentRegistry source](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/subagents/builtin-agents.ts)
- [Qwen Code declarative-agent frontmatter schema source](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/subagents/agent-frontmatter-schema.ts)
- [Qwen Code AgentTool source](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/tools/agent/agent.ts)
- [Qwen Code ForkSubagent source](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/tools/agent/fork-subagent.ts)
- [Qwen Code Storage source](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/config/storage.ts)
- [Qwen Code agent transcript sidecar source](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/agents/agent-transcript.ts)