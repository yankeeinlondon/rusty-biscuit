---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3

homepage: https://www.kimi.com/code/
docs: https://moonshotai.github.io/kimi-cli/en/
subagent_docs: https://moonshotai.github.io/kimi-cli/en/customization/agents.html

support: first_class

locations:
  - os: macos
    scope: system
    path: "<site-packages>/kimi_cli/agents/{default,okabe}/agent.yaml"
    notes: "Built-in agents ship inside the installed `kimi-cli` Python wheel. Resolved by `kimi_cli.agentspec.get_agents_dir()` to the package directory; `default/agent.yaml` and `okabe/agent.yaml` are the two shipped root agents. Each is a directory containing `agent.yaml`, `system.md`, and the per-subagent YAMLs (`coder.yaml`, `explore.yaml`, `plan.yaml`). The site-packages location follows the active Python interpreter; the only fixed path is `kimi_cli/agents/`."
  - os: linux
    scope: system
    path: "<site-packages>/kimi_cli/agents/{default,okabe}/agent.yaml"
    notes: "Same as macOS. The Linux wheel installs the same `kimi_cli/agents/` tree; `kimi --agent default` and `kimi --agent okabe` resolve here."
  - os: windows
    scope: system
    path: "<site-packages>\\kimi_cli\\agents\\{default,okabe}\\agent.yaml"
    notes: "Same Python-wheel layout on Windows. Path uses backslashes on native Windows; Kimi CLI's Windows runtime additionally requires Git Bash for the `Shell` tool (`KIMI_CLI_GIT_BASH_PATH` overrides discovery)."
  - os: macos
    scope: other
    path: "--agent-file <path/to/agent.yaml> (CLI flag, per-session only)"
    notes: "User-defined custom agents are NOT auto-discovered. They are passed via the `--agent-file` flag per invocation, so they exist only for the lifetime of the session. `kimi --agent-file /path/to/my-agent.yaml` is the documented entry point. The file format is identical to the built-in `agent.yaml` (YAML, `version: 1`, `agent.name`, `agent.system_prompt_path`, `agent.tools`, `agent.extend`, `agent.subagents` block). Any path readable by the user is accepted; no per-user `~/.kimi/agents/` or per-repo `.kimi/agents/` directory is scanned today."
  - os: linux
    scope: other
    path: "--agent-file <path/to/agent.yaml> (CLI flag, per-session only)"
    notes: "Same as macOS. There is no `~/.config/kimi/agents/` auto-discovery either; user agents are flag-loaded only."
  - os: windows
    scope: other
    path: "--agent-file <path\\to\\agent.yaml> (CLI flag, per-session only)"
    notes: "Same as macOS / Linux. Quote PowerShell-safe; the file format is identical to the built-in `agent.yaml`."
  - os: macos
    scope: system
    path: "<session-dir>/subagents/<agent_id>/{context.jsonl,wire.jsonl,meta.json,prompt.txt,output}"
    notes: "Per-session runtime storage for subagent instances. `<session-dir>` is `~/.kimi/sessions/<md5(work_dir)>/<session_id>/` (or `$KIMI_SHARE_DIR/sessions/...` when the env var is set). `<agent_id>` is `a<8-hex>` (e.g. `a1b2c3d4`) and the instance directory is created by `kimi_cli.subagents.store.SubagentStore.create_instance` and held under the parent session's `subagents/` subdirectory. The legacy README describes this as a wire-record plus context-plus-output set; it is regenerated on resume and reloaded across multiple subagent invocations."
  - os: linux
    scope: system
    path: "<session-dir>/subagents/<agent_id>/{context.jsonl,wire.jsonl,meta.json,prompt.txt,output}"
    notes: "Same as macOS. Session directory root is `~/.kimi/sessions/<md5(work_dir)>/<session_id>/` (or `$KIMI_SHARE_DIR/sessions/...`)."
  - os: windows
    scope: system
    path: "<session-dir>\\subagents\\<agent_id>\\{context.jsonl,wire.jsonl,meta.json,prompt.txt,output}"
    notes: "Same as macOS / Linux, using backslashes on native Windows. Path layout is the same Python file API."
  - os: macos
    scope: extension
    path: "plugin <plugin_dir>/{agents/agent.yaml, agents/<name>/agent.yaml} (hypothetical — not yet observed)"
    notes: "The legacy `kimi-cli` documentation does not describe plugin-packaged agent definitions today. The plugin loader (`kimi_cli.plugin.tool.load_plugin_tools`) loads plugin-supplied **tools** rather than agent files; the agent file path is always package- or `--agent-file`-sourced. This is a candidate extension surface for future Claudine cross-checking; treat as a planned-but-undefined shape and verify per release before linking."
  - os: linux
    scope: extension
    path: "plugin <plugin_dir>/agents/<name>.yaml (hypothetical — not yet observed)"
    notes: "Same as macOS — speculative. No documented plugin-agent surface in `kimi-cli` main as of 1.48.0."
  - os: windows
    scope: extension
    path: "plugin <plugin_dir>\\agents\\<name>.yaml (hypothetical — not yet observed)"
    notes: "Same as macOS / Linux — speculative."

format:
  file_names:
    - "agent.yaml"
  frontmatter: false
  required_fields:
    - "version (the only required top-level scalar; `1` is the only supported value via `DEFAULT_AGENT_SPEC_VERSION` and `SUPPORTED_AGENT_SPEC_VERSIONS = (\"1\",)`)"
    - "agent (YAML object — at minimum, `name`, `system_prompt_path`, and `tools` after inheritance resolution; or `extend: default` plus any subset of overrides)"
  optional_fields:
    - "extend (string — `default` to inherit the built-in default agent, or a relative path to another `agent.yaml`; defaults to none / no inheritance)"
    - "name (string — agent name; required when not inheriting; inherited from the parent if absent)"
    - "system_prompt_path (string — path to the Markdown system prompt file, relative to the agent YAML's directory; required when not inheriting)"
    - "system_prompt_args (map[string,string] — extra Jinja2 substitutions exposed as `${KEY}` in the system prompt; merged on `extend`, key-by-key, so the child can add or override variables; `ROLE_ADDITIONAL` is the conventional variable used to inject subagent-specific guidance)"
    - "model (string — default model alias for this agent; propagated to `AgentTypeDefinition.default_model` so the subagent can be launched without a per-invocation `model` parameter)"
    - "when_to_use (string — natural-language usage guidance surfaced in the `Agent` tool's per-type description and used by the parent's tool selection)"
    - "tools (array[string] — `module:ClassName` allowlist when `allowed_tools` is null; the canonical set of tools exposed by the agent)"
    - "allowed_tools (array[string] | null — explicit allowlist that takes precedence over `tools`; when non-null, the `subagents[]` entry's `ToolPolicy` is set to `allowlist` and the subagent only sees these tools)"
    - "exclude_tools (array[string] — list of `module:ClassName` entries to strip from `tools`; the standard way to remove inherited tools without rewriting the whole allowlist)"
    - "subagents (map[string, {path, description}] — built-in subagent types this agent can launch. Each value is a `SubagentSpec`; `path` is a path (relative to this agent YAML's directory) to the subagent's own `agent.yaml`, and `description` is the natural-language description surfaced via the `Agent` tool's `${BUILTIN_AGENT_TYPES_MD}` rendering)"
  body_format: yaml
  notes: |
    The agent file is a **two-key** YAML envelope: a `version: 1` scalar and an `agent:` mapping. There is no Markdown body — the system prompt lives in a separate Markdown file referenced by `system_prompt_path`. The system prompt supports Jinja2 templating with `${VAR}` syntax (the agent loader uses a custom Jinja2 environment with `variable_start_string = \"${\"` and `variable_end_string = \"}\"`), and accepts `{% include %}` directives for splicing additional files. The built-in system prompt inherits `${ROLE_ADDITIONAL}` from the subagent YAMLs, which is how a subagent gets its role-specific instructions.

    Inheritance (`extend: default` or `extend: <relative-path>`) is a **shallow, field-by-field** merge: scalars like `name`, `system_prompt_path`, `model`, `when_to_use`, `exclude_tools`, `subagents` are replaced wholesale, while `system_prompt_args` is **merged** (the child can add or override individual keys). `tools` and `allowed_tools` are replaced (not merged); `exclude_tools` is replaced. A child that wants to *add* a tool to an inherited allowlist must restate the whole list. Subagent paths in the `subagents:` block are resolved **relative to the parent file's directory**, not to the working directory or to `~/.kimi/`, so a user-defined subagent must sit beside the parent agent YAML (or be reached by an explicit relative path).

    The built-in subagent types in the default agent (`coder`, `explore`, `plan`) follow the same format. `coder` and `explore` extend the default agent's `tools` via `allowed_tools:`, while `plan` further narrows with `exclude_tools` (no `Shell`, no write tools). `okabe` extends the default agent and adds `SendDMail` (a checkpoint-rollback D-Mail tool).

    The new `kimi-code` (TypeScript) implementation at `MoonshotAI/kimi-code` is the migration target and does **not** expose a user-defined agent file format. The new runtime keeps only the three built-in sub-agent types (`coder`, `explore`, `plan`) and surfaces them through the `Agent` tool. Users who want to change behavior use `$KIMI_CODE_HOME/AGENTS.md`, `~/.agents/AGENTS.md`, or `.kimi-code/AGENTS.md` instruction files instead of a YAML agent spec. Document the new `kimi-code` shape as `none` for `file_names` — there are no user-editable definition files, only an instruction file that goes through a different topic (instruction / AGENTS.md), not the agent-definition topic.

runtime:
  invocation: |
    Two surfaces invoke a subagent:

    (1) **Automatic / parent-driven** — the main agent (the root agent loaded from the parent `agent.yaml`) calls the `Agent` tool (`kimi_cli.tools.agent:Agent`) when it judges a sub-task to be more efficient in an isolated context. The `Agent` tool's description (`kimi_cli/tools/agent/description.md`) says: "If the `Agent` tool is available, you can use it to delegate a focused subtask to a subagent instance. The tool can either start a new instance or resume an existing one by `agent_id`. Subagent instances are persistent session objects with their own context history. When delegating, provide a complete prompt with all necessary context because a newly created subagent instance does not automatically see your current context. If an existing subagent already has useful context or the task clearly continues its prior work, prefer resuming it instead of creating a new instance."

    (2) **Tool call parameters** — `Agent(description: str, prompt: str, subagent_type: str = "coder", model: str | None = None, resume: str | None = None, run_in_background: bool = False, timeout: int | None = None)`. `description` is a 3–5 word label, `prompt` is the full task text (subagents do **not** see the parent's conversation), `subagent_type` selects one of the entries registered from the parent agent's `subagents:` block, `model` overrides the subagent's default model (validated against `config.models` aliases), `resume` re-enters a previously-created subagent by `agent_id` (returning a fresh, empty `description`/`prompt` and a guarded check that the instance is not still running), `run_in_background` switches from foreground (`ForegroundSubagentRunner`) to background (`BackgroundAgentRunner`) execution, and `timeout` bounds the run (30–3600s; foreground default is no timeout, background default is `BackgroundConfig.agent_task_timeout_s = 900`).

    Foreground is the default. The subagent's response (its final assistant message) is the only thing returned to the parent. If the summary is shorter than 200 chars the runner calls the soul a second time with a `SUMMARY_CONTINUATION_PROMPT` ("Your previous response was too brief. Please provide a more comprehensive summary …") and the extended summary replaces the short one in the parent's view.
  parent_child_context: |
    Subagents run in their **own** KimiSoul, with an independent `Context`, an independent `KimiToolset` (loaded from the subagent's own `tools` / `allowed_tools` / `exclude_tools` resolution), an independent `SubagentBuilder`-cloned LLM (`clone_llm_with_model_alias` — same provider, possibly different `model` alias), and an independent `DenwaRenji` instance. They do **not** receive the parent's conversation history, the parent's AGENTS.md / system prompt, the parent's wire log, or any prior tool calls. The system prompt is built from the subagent's own `system.yaml` with `${ROLE_ADDITIONAL}` and `${KIMI_SKILLS}` / `${KIMI_WORK_DIR}` / `${KIMI_NOW}` / `${KIMI_ADDITIONAL_DIRS_INFO}` substituted in.

    The runtime's `copy_for_subagent(agent_id, subagent_type, llm_override)` clones the parent's `Runtime` but sets `role = "subagent"`, attaches the subagent id and type, and **shares** (not copies) `subagent_store`, `approval_runtime`, `root_wire_hub`, `additional_dirs`, `skills_dirs`, and the parent `Session`. The `Approval` is shared through `Approval.share()` so the parent's "always allow" rules propagate. The subagent's `Context` is restored from `SubagentStore.context_path(agent_id)` so a resumed subagent keeps its prior turns; on the first run the system prompt is also written to `context.jsonl` so subsequent resumes see the same prompt the child ran with.

    The `explore` subagent is special-cased in `kimi_cli.subagents.core.prepare_soul`: for non-resumed runs, the subagent's prompt is **prefixed** with a `<git-context>` block (`collect_git_context(KIMI_WORK_DIR)`), giving the explore agent immediate context about the repository's git state before it begins its search.

    Returned state: foreground — the final assistant text (`return_last_only = true` is enforced by the runner). Background — a structured `ToolReturnValue` with the lines `task_id`, `kind`, `status`, `description`, `agent_id`, `actual_subagent_type`, `automatic_notification: true`, and a `resume_hint: Use Agent(resume="<agent_id>", prompt="...") to continue this instance later.`
  permissions_inheritance: |
    The shared `Approval` runtime gives subagents the parent's allow/deny rules and the parent's "always approve" history — a subagent does not re-prompt for a tool that the parent already approved. The `Agent` tool itself is always allowed (it is a built-in for the root agent and is excluded for subagents by every shipped subagent spec via `exclude_tools: ["kimi_cli.tools.agent:Agent"]`), so the parent can chain multiple subagent invocations without an extra approval.

    Subagents cannot re-delegate: the `Agent` tool is removed from every built-in subagent's `tools` / `allowed_tools` (and `AgentTool.__call__` early-returns with `Subagents cannot launch other subagents` when `runtime.role != "root"`). Background subagents cannot stop or modify each other either — `TaskStop` is in the parent's `tools` but the subagent's `KimiToolset` is built from the subagent's own `tools` / `allowed_tools` / `exclude_tools`, so a background subagent does not see the `TaskStop` tool by default unless its spec lists it.

    YOLO mode (`kimi --yolo` / `--auto-approve`) and `--afk` both flow through `Runtime.create`'s `effective_yolo` / `afk` propagation and into the `ApprovalState`; the subagent's `Approval.share()` shares that state, so YOLO/AFK inheritance is implicit. Plan mode is set on the parent only and is not propagated; subagents always start in `plan_mode = false`.
  model_inheritance: |
    Resolution order, top wins (per `kimi_cli.subagents.builder.SubagentBuilder.resolve_effective_model`):

    1. `Agent(model="<alias>")` per-invocation override from the parent's tool call (`params.model`).
    2. `AgentLaunchSpec.effective_model` (the model captured at instance creation; for non-overridden calls this is `subagent_type_def.default_model`).
    3. The subagent's `default_model` from the `AgentTypeDefinition` (i.e. the `model` field on the subagent's `agent.yaml`).
    4. The parent session's current model.

    Step 1 is validated against `runtime.config.models` aliases in `AgentTool.__call__` (returns `Unknown model alias: <alias>` if missing). Step 2 is re-validated on resume because the configured aliases may have changed since the instance was created. The `clone_llm_with_model_alias` helper produces a fresh `LLM` for the subagent; the parent and subagent share the provider, OAuth manager, and session id but the LLM object itself is a clone.
  tool_inheritance: |
    The parent's tool set is **not** inherited as a bulk. Each subagent declares its own tool surface via `allowed_tools` (allowlist), `tools` (allowlist), or `exclude_tools` (denylist relative to the inherited default). The default agent's `tools` list is the canonical super-set (`Agent`, `AskUserQuestion`, `SetTodoList`, `Shell`, `TaskList`, `TaskOutput`, `TaskStop`, `ReadFile`, `ReadMediaFile`, `Glob`, `Grep`, `WriteFile`, `StrReplaceFile`, `SearchWeb`, `FetchURL`, `EnterPlanMode`, `ExitPlanMode`). Subagents narrow it:
    - `coder` allows: `Shell`, `ReadFile`, `ReadMediaFile`, `Glob`, `Grep`, `WriteFile`, `StrReplaceFile`, `SearchWeb`, `FetchURL`; excludes: `Agent`, `AskUserQuestion`, `SetTodoList`, `ExitPlanMode`, `EnterPlanMode`.
    - `explore` allows the coder set minus `WriteFile` and `StrReplaceFile`; excludes the same set as `coder`.
    - `plan` allows only `ReadFile`, `ReadMediaFile`, `Glob`, `Grep`, `SearchWeb`, `FetchURL`; excludes `Agent`, `AskUserQuestion`, `SetTodoList`, `ExitPlanMode`, `EnterPlanMode`, `Shell`, `WriteFile`, `StrReplaceFile` — i.e. **no shell, no writes**.

    Plugin tools loaded by `kimi_cli.plugin.tool.load_plugin_tools` are added to the parent's toolset. Subagents do **not** automatically inherit plugin tools — the plugin's `tools/` files are loaded by the parent and only seen by subagents whose `tools` / `allowed_tools` explicitly reference them. MCP tools configured at the parent level are also passed through per-invocation: `load_agent(agent_file, runtime, mcp_configs=[...])` is called by `KimiCLI.create` with the parent's MCP configs, and subagents receive a **fresh** `load_agent` call from `SubagentBuilder.build_builtin_instance` with `mcp_configs=[]` — so MCP tools are **not** inherited by default. To get MCP tools into a subagent, the subagent's own `agent.yaml` must reference the same MCP server, or the subagent's `tools` list must include the MCP-derived `module:ClassName`.

    Subagents cannot re-delegate (the `Agent` tool is excluded by every shipped subagent spec and the runtime blocks it for `role != "root"`).
  max_turns: |
    Not a turn count, but a step count: `LoopControl.max_steps_per_turn` (config default 1000) and `LoopControl.max_retries_per_step` (config default 3) bound the subagent's main loop. `max_ralph_iterations` (config default 0) and `agent_task_timeout_s` (config default 900s for background) are inherited from the parent config unless overridden by the subagent's own setup. Per-invocation `Agent(timeout=…)` (30–3600s) overrides the background timeout. `max_steps_per_turn` defaults to the **config** value, which is the same for the parent and the subagent, so a subagent cannot be "larger" than its parent by default.
  notes: |
    Concurrency: multiple subagents can run in parallel because the parent's `Agent` tool calls are not serialized. Background subagent capacity is bounded by `BackgroundConfig.max_running_tasks` (default 4); foreground subagents run in the same asyncio task as the parent tool call (one foreground at a time per parent turn).

    Nesting: nesting is **not** allowed. `Agent` is excluded from every built-in subagent's `tools` / `allowed_tools`, and `AgentTool.__call__` returns `Subagents cannot launch other subagents` when the runtime role is not `"root"`. There is no documentation for user-defined subagents that opt in to nesting.

    Selection: the parent model picks a `subagent_type` from the `BUILTIN_AGENT_TYPES_MD` lines that `AgentTool.__init__` injects into the tool description. The list is built from `runtime.labor_market.builtin_types` (the `LaborMarket` registered by the parent's `load_agent`), so it follows the parent agent file: switching to `okabe` does **not** change the subagent set (okabe's only difference is the added `SendDMail` tool on the root agent), and a `--agent-file` whose `subagents:` block lists `["coder", "reviewer", "auditor"]` will surface those three types in the parent's `Agent` tool description.

    Failure: a subagent that hits `max_steps_per_turn`, an LLM API error, or an unexpected exception is reported back to the parent as a `SoulRunFailure` with a `brief` of `Max steps reached` / `API error (<status>)` / `LLM provider error` / `Agent run error`. A timed-out subagent returns a `ToolError(message="Agent timed out after {t}s.", brief=f"Agent timed out ({t}s)")`. The parent's own session is not marked failed; the parent's `kimi` process continues normally.

    Resume: `SubagentStore.create_instance` writes `meta.json` immediately, then `update_instance` flips the status to `running_foreground` / `running_background` / `completed` / `failed` / `killed` as the run progresses. A second `Agent(resume="<agent_id>", …)` call re-enters the same instance by reading its `meta.json` and restoring the `Context` from `context.jsonl`. The same `agent_id` is reused for the lifetime of the parent session; a stale foreground record is **cleaned up on next startup** by `KimiCLI._cleanup_stale_foreground_subagents`.

    The new `kimi-code` (TypeScript) keeps the same three built-in subagent types and the same `Agent`-tool semantics but, as of 0.14.0, drops the YAML `--agent-file` entry point — there is no `--agent` / `--agent-file` flag in the new `kimi-code` `kimi --help`, only the built-in main agent with auto-scheduled subagents. The `kimi migrate` subcommand is the supported upgrade path for users who authored custom YAML agents.

observability:
  stream_events:
    - "SubagentEvent (wire envelope wrapping any `Event` from a subagent; fields: `parent_tool_call_id`, `agent_id`, `subagent_type`, `event`. Documented in `kimi_cli.wire.types.SubagentEvent`.)"
    - "StepBegin (emitted on every subagent step; `n: int` is the step number; same as the parent main loop.)"
    - "StepInterrupted (emitted when a subagent step is interrupted by user action or error.)"
    - "StepRetry (emitted on each retry of a step; `n`, `next_attempt`, `max_attempts`, `wait_s`, `error_type`, `status_code`.)"
    - "StatusUpdate (emitted on the subagent; carries `context_usage`, `context_tokens`, `max_context_tokens`, `token_usage`, `message_id`, `plan_mode`, `mcp_status`.)"
    - "Notification (subagent-sourced notifications bubble to the parent via the shared `NotificationManager`.)"
    - "CompactionBegin / CompactionEnd (auto-compaction boundaries inside a subagent's context, emitted on the subagent's wire.)"
    - "HookTriggered / HookResolved (server-side and wire-side hook lifecycle; the engine wraps both surfaces; aggregate `action: \"allow\" | \"block\"` and per-hook `duration_ms`.)"
    - "agent_id (`a<8-hex>`; generated by `uuid.uuid4().hex[:8]` in the foreground path and by the background path; visible in the `Agent` tool's `ToolReturnValue`, in the parent wire log, and in `<session-dir>/subagents/<agent_id>/meta.json`.)"
    - "actual_subagent_type (the subagent type actually used after resume, in case the parent's stored launch spec differs; surfaced in the background `ToolReturnValue`.)"
    - "task_id (background only; surfaced as `task_id: <id>` in the `ToolReturnValue`; same id used by `TaskList` / `TaskOutput` / `TaskStop`.)"
  hook_events:
    - "SubagentStart (payload: `agent_name`, `prompt`; matcher targets the subagent name built into the description. Documented in `kimi_cli.hooks.config.HookEventType` and `kimi_cli.hooks.events.subagent_start`.)"
    - "SubagentStop (payload: `agent_name`, `response: str = \"\"`; documented in `kimi_cli.hooks.events.subagent_stop`.)"
    - "PreToolUse / PostToolUse / PostToolUseFailure (subagent-scope tool hooks — the parent's `kimi_cli.hooks.engine.HookEngine` matcher can be the `Agent` tool, so hooks see every subagent tool call through the parent's tool-call surface.)"
    - "UserPromptSubmit / SessionStart / SessionEnd / PreCompact / PostCompact / Stop / StopFailure / Notification (full Kimi-CLI hook event set; documented in `kimi_cli.hooks.config.HookEventType`.)"
  session_ids: true
  notes: |
    Each subagent has a **stable `agent_id`** (`a<8-hex>`) and a **stable wire log** at `<session-dir>/subagents/<agent_id>/wire.jsonl`. The parent's session id is the same as the subagent's session id (the subagent shares the parent's `Session`); differentiation is by the `agent_id` key, not by a separate session id.

    `SubagentStore` is a small on-disk JSON + JSONL store. Per-instance files:
    - `meta.json` — atomic-write of `AgentInstanceRecord` (pydantic-validated against `_AgentInstanceRecordPayload`); updated by `update_instance` on every status change.
    - `context.jsonl` — the subagent's full `Context` (system prompt + message history); restored on resume.
    - `wire.jsonl` — the subagent's wire record (tool calls, status updates, subagent events, hook triggers).
    - `prompt.txt` — a snapshot of the prompt the subagent was launched with (debug aid; written by `prepare_soul`).
    - `output` — a tail file the subagent can stream text into (the `SubagentOutputWriter` writes here).

    The parent's wire log also embeds `SubagentEvent` envelopes with `agent_id` and `subagent_type`, so a stream consumer reading the parent's wire can reconstruct the entire subagent timeline without re-reading the subagent's wire. A consumer that wants the raw subagent wire can correlate via `agent_id` and read `<session-dir>/subagents/<agent_id>/wire.jsonl` directly.

    The `cleanupPeriodDays`-style retention policy is not documented; deletion is per-session (`Session.delete()`) which removes the entire `subagents/` subtree.

    Hooks are matched on `agent_name`, which `AgentTool._builtin_type_lines` builds from the runtime `LaborMarket`'s registered types (i.e. the parent agent's `subagents:` block). A custom subagent added to a `--agent-file` parent (e.g. `reviewer`) is auto-discoverable by a `SubagentStart` / `SubagentStop` hook with `matcher = "reviewer"`.

    The new `kimi-code` (TypeScript) describes the same lifecycle in its docs (`Sub-agent runtime state is persisted to the agents/ subdirectory of the current session directory … wire.jsonl`), but the TypeScript implementation does not yet publish the same `SubagentEvent` wire envelope in the docs reviewed (2026-07-02). Treat the new TS stream-event surface as `partial` until the matching `kimi-code` SDK source is reviewed; the hook events (`SubagentStart` / `SubagentStop`) are documented in the new docs at `https://moonshotai.github.io/kimi-code/en/customization/hooks.html` (Beta).

portability:
  portable: false
  non_portable_assets:
    - "`version: 1` envelope and the `agent:` key shape (Kimi CLI's AgentSpec pydantic model; other providers use TOML/Markdown/YAML with different envelopes)"
    - "`kimi_cli.tools.*:<ClassName>` tool identifiers (Kimi-specific module paths)"
    - "`extend: default` magic value (resolves to `kimi_cli.agents.default.agent.yaml`; other providers do not have a `default` inheritance root)"
    - "`subagents:` block with `path` + `description` (Kimi-specific composition shape; Claude Code uses frontmatter `subagents:` is not a thing, Goose uses `sub_recipes:` / `properties`, Codex uses TOML `agents/`)"
    - "`system_prompt_path` Markdown + `${VAR}` / `{% include %}` Jinja templating (custom Jinja environment with `variable_start_string = \"${\"`; this is Kimi-specific and not portable to providers that do not have Jinja in their system prompt pipeline)"
    - "`system_prompt_args` (the `${ROLE_ADDITIONAL}` pattern is Kimi-specific — other providers do not have a way to splice role-specific guidance into an inherited system prompt)"
    - "`allowed_tools` / `exclude_tools` allow-and-deny tool lists with `module:ClassName` paths (Claude Code uses `tools: [\"Read\", \"Bash\"]` short names; Goose uses extension names; Codex uses TOML tool entries)"
    - "`model: <alias>` default (Kimi model alias scheme; not portable to Claude's `sonnet`/`opus`/`haiku` family or to Codex's `gpt-5-codex` family)"
    - "`when_to_use` field (Kimi uses it in the `Agent` tool's per-type description; other providers have `description` instead)"
    - "`Agent` tool's `subagent_type` / `resume` / `run_in_background` / `timeout` parameters (Claude Code's `Agent` tool has `subagent_type`, `description`, `prompt`, `model`, `run_in_background`, and `name`; the parameter shapes are NOT 1:1)"
    - "Built-in subagent type names `coder`, `explore`, `plan` (Claude Code ships `Explore`, `Plan`, `general-purpose`, `statusline-setup`, `claude-code-guide`; Goose ships `developer`, `computercontroller`, etc.)"
    - "`KIMI_*` env vars (`KIMI_SHARE_DIR`, `KIMI_CLI_GIT_BASH_PATH`, `KIMI_BUILD_SHA`, `KIMI_CODE_OAUTH_KEY`) — not portable"
    - "`~/.kimi/` share directory (Kimi-specific; Claude Code uses `~/.claude/`, Goose uses `~/.config/goose/`)"
    - "Subagent storage layout `<session-dir>/subagents/<agent_id>/` with `context.jsonl` + `wire.jsonl` + `meta.json` + `prompt.txt` (Kimi-specific; not portable to Claude Code's `~/.claude/projects/{project}/{sessionId}/subagents/agent-{agentId}.jsonl` shape)"
    - "The new `kimi-code` (TypeScript) drops `--agent-file` entirely — there is no portable target to link to for users who upgrade to `kimi-code`; the only knob left is the AGENTS.md instruction file (covered by the instruction-files topic, not the agent topic)"
  rewrite_needed: true
  notes: |
    The body system prompt (the Markdown at `system_prompt_path`) is the most portable bit, but it is **not** what `claudine agents` should target — Kimi agent files are not body-only Markdown; the `agent:` mapping is the definition. A cross-provider rewrite therefore needs a translation pass:

    | Kimi CLI field | Claude Code equivalent | Goose equivalent | Codex equivalent |
    |---|---|---|---|
    | `version: 1` + `agent:` envelope | n/a (Claude Code uses a Markdown file with YAML frontmatter) | n/a (Goose uses `*.md` frontmatter) | n/a (Codex uses TOML `agents/<name>.toml`) |
    | `agent.name` | frontmatter `name` | frontmatter `name` (Goose is more permissive; slugify on cross-link) | TOML `name` |
    | `agent.system_prompt_path` (Markdown file) | frontmatter-less body, since Claude Code's subagent body IS the system prompt | body of the `.md` agent | `developer_instructions` |
    | `agent.system_prompt_args.ROLE_ADDITIONAL` | inline into body (no separate arg map in Claude Code) | inline into body (Goose's `properties` bag is the closest, but is provider-internal) | inline into `developer_instructions` |
    | `agent.model` (alias) | frontmatter `model` (`sonnet` / `opus` / `haiku` / `fable` / full ID / `inherit`) | frontmatter `model` (Goose does not currently propagate it to runtime) | TOML `model` |
    | `agent.tools` / `agent.allowed_tools` / `agent.exclude_tools` | frontmatter `tools` allowlist (`Agent(…)` allowlist is Claude-specific) | per-invocation `delegate.extensions: [...]` (no per-definition allowlist) | TOML `tools` |
    | `agent.subagents.{name}.{path, description}` | n/a (Claude Code has no per-agent subagent registry; subagent types are global `Agent` tool entries) | n/a (Goose's subagents are global `SourceType::Agent` entries, not nested) | n/a (Codex has no nested-subagent block) |
    | `agent.when_to_use` | frontmatter `description` (Claude uses description as the routing signal) | frontmatter `description` (Goose surfaces it through `summon`) | TOML `description` |

    Subagent storage layouts are not interchangeable: Kimi's `<session-dir>/subagents/<agent_id>/{context.jsonl, wire.jsonl, meta.json, prompt.txt, output}` is a per-subagent directory of multiple files; Claude Code writes a single `agent-{agentId}.jsonl` per subagent. A linker that wants to surface Kimi subagent state to a Claude-Code consumer must rebuild the JSONL shape from the Kimi files.

    The new `kimi-code` does not have a `--agent-file` flag, so the cross-link from `claudine agents` would have to (a) keep the link as a Kimi-CLI-only path and warn that it does not apply to `kimi-code`, or (b) translate the YAML into a `KIMI_CODE_HOME/AGENTS.md` instruction block (which is a different topic and loses the per-tool allowlist, the per-subagent composition, and the inheritance model).

cli_params:
  - flag: --agent <default|okabe>
    description: "Selects a built-in root agent spec; resolves to the package's `kimi_cli/agents/{default,okabe}/agent.yaml`. Mutually exclusive with `--agent-file`. Default: `default` (when neither flag is given)."
    example: "kimi --agent okabe"
  - flag: --agent-file <path>
    description: "Loads a custom root agent spec from a YAML file. The file must use the `version: 1` + `agent:` shape; it is loaded by `kimi_cli.agentspec.load_agent_spec` with the same validation, inheritance, and tool-resolution path as the built-in agents. Mutually exclusive with `--agent`. Per-session only — not persisted."
    example: "kimi --agent-file /path/to/my-agent.yaml"
  - flag: --work-dir <path> (alias -w)
    description: "Sets the working directory for the session. The `KIMI_WORK_DIR` and `KIMI_WORK_DIR_LS` system-prompt args are derived from this path; the project root is detected by walking up to the nearest `.git`."
    example: "kimi --work-dir /path/to/project"
  - flag: --add-dir <path> [...]
    description: "Adds an additional directory to the workspace scope; `KIMI_ADDITIONAL_DIRS_INFO` is computed from the union. Subagents share `additional_dirs` (the runtime copies the list reference), so an `--add-dir` added at the parent is visible to the subagent's `KIMI_WORK_DIR` and `KIMI_ADDITIONAL_DIRS_INFO`."
    example: "kimi --add-dir ../shared --add-dir ~/notes"
  - flag: --session <id> (alias -S, -r, --resume)
    description: "Resumes a session. With an id: resume that session. Without an id: interactively pick from `Session.list(work_dir)`. The session's persisted subagent instances are restored alongside the parent context, so a resumed session's `subagents/<agent_id>/` directories survive and `Agent(resume=…)` keeps working."
    example: "kimi -S 5a6c8a1f-9b1d-4d6b-9a4e-1b7c7f8c2c7b"
  - flag: --continue (alias -C)
    description: "Resumes the previous session for the working directory. Same subagent-restoration semantics as `--session`."
    example: "kimi -C"
  - flag: --yolo (aliases --yes, -y, --auto-approve)
    description: "Auto-approves every approval request for this session. Propagated to subagents through the shared `Approval` runtime, so the subagent does not re-prompt for tools the parent would have auto-approved."
    example: "kimi --yolo"
  - flag: --afk
    description: "Run in away-from-keyboard mode: no user is present, `AskUserQuestion` is auto-dismissed, and tool calls are auto-approved. Persists in `SessionState.approval.afk`. Propagated to subagents."
    example: "kimi --afk"
  - flag: --plan
    description: "Start the session in plan mode (read-only exploration + plan submission). Plan mode is parent-scoped — subagents do not inherit it."
    example: "kimi --plan"
  - flag: --prompt <text> (aliases -p, --command, -c)
    description: "Run one prompt non-interactively and print the response. In `kimi-cli` this routes through `run_print`; subagents can still be invoked by the parent, but the wrapper exits after the parent turn completes."
    example: "kimi --prompt \"Summarize the diff\""
  - flag: --input-format <text|stream-json>
    description: "Input format for `--prompt` mode. Pairs with `--output-format`."
    example: "kimi --prompt \"…\" --input-format stream-json --output-format stream-json"
  - flag: --output-format <text|stream-json>
    description: "Output format for `--prompt` mode. The `stream-json` output is the wire-protocol stream (the same envelope a wrapper would parse), including subagent events."
    example: "kimi --prompt \"…\" --output-format stream-json"
  - flag: --final-message-only
    description: "Only print the final assistant message in `--print` mode. Suppresses intermediate wire events (still emitted, but the wrapper filters)."
    example: "kimi --print --final-message-only"
  - flag: --quiet
    description: "Alias for `--print --output-format text --final-message-only`. Conflicts with `--acp` / `--wire`."
    example: "kimi --quiet --prompt \"…\""
  - flag: --acp (deprecated in legacy kimi-cli; replaced by `kimi acp` subcommand)
    description: "Run as Agent Client Protocol server. The `kimi acp` subcommand is the supported entry point in the legacy kimi-cli; the new `kimi-code` exposes `kimi acp` directly."
    example: "kimi acp"
  - flag: --wire
    description: "Run as Wire server (experimental). Wire is Kimi CLI's lower-level stream-json RPC surface used by web/visualizer UIs."
    example: "kimi --wire"
  - flag: --skills-dir <path> [...]
    description: "Override the default skill discovery with explicit directories. Subagents see the same `skills_dirs` (the runtime copies the list reference), so a subagent's `${KIMI_SKILLS}` reflects the parent's override."
    example: "kimi --skills-dir ~/team-skills"
  - flag: --mcp-config-file <path> [...]
    description: "Add an MCP config file (JSON) to the session. MCP tools are loaded into the parent's toolset; subagents do **not** inherit MCP tools unless their own `agent.yaml` references the same MCP server."
    example: "kimi --mcp-config-file ./mcp.json"
  - flag: --mcp-config <json> [...]
    description: "Inline MCP config JSON; same semantics as `--mcp-config-file`."
    example: "kimi --mcp-config '{\"mcpServers\": {\"context7\": {\"url\": \"https://mcp.context7.com/mcp\"}}}'"
  - flag: --config <toml-or-json>
    description: "Inline config override; merged with the on-disk `~/.kimi/config.toml`."
    example: "kimi --config '[providers.kimi] type = \"kimi\"'"
  - flag: --config-file <path>
    description: "Use a specific config file instead of `~/.kimi/config.toml`."
    example: "kimi --config-file ./team-config.toml"
  - flag: --model <alias> (alias -m)
    description: "Override the LLM model alias for this session. `AgentTool.__call__` validates against `config.models` and propagates to the subagent via `Agent(model=…)` or through the parent's `LLM`."
    example: "kimi --model kimi-for-coding"
  - flag: --thinking / --no-thinking
    description: "Toggle the session's thinking mode. Inherited by subagents through the shared `LLM` (or the `LLM` clone when `clone_llm_with_model_alias` is used)."
    example: "kimi --no-thinking"
  - flag: --max-steps-per-turn <n>
    description: "Override `LoopControl.max_steps_per_turn` for the session. Inherited by subagents because the subagent uses the same `Config` object (the runtime's `config` field is shared, not copied)."
    example: "kimi --max-steps-per-turn 50"
  - flag: --max-retries-per-step <n>
    description: "Override `LoopControl.max_retries_per_step` for the session."
    example: "kimi --max-retries-per-step 5"
  - flag: --max-ralph-iterations <n>
    description: "Override `LoopControl.max_ralph_iterations` (-1 for unlimited). Subagents do not currently use the Ralph loop (only the root agent does), so this is a parent-only setting that does not propagate to the subagent's loop."
    example: "kimi --max-ralph-iterations 3"
  - flag: --verbose
    description: "Print verbose information to stderr (separate from `--debug`'s log-file TRACE level)."
    example: "kimi --verbose"
  - flag: --debug
    description: "Log TRACE-level information to `~/.kimi/logs/kimi.log`. Subagent runs are also captured because the logger is process-global."
    example: "kimi --debug"
  - flag: --version (alias -V)
    description: "Print the kimi-cli version and exit."
    example: "kimi --version"
  - flag: --help (alias -h)
    description: "Show help and exit. (No auto-discovered agent file is mentioned in the help output — `--agent-file` is the only user-agent entry point.)"
    example: "kimi --help"

env_vars:
  - name: KIMI_SHARE_DIR
    effect: "Override the share / data root. Default: `~/.kimi/`. When set, the share directory is `${KIMI_SHARE_DIR}/` and `<session-dir>` becomes `${KIMI_SHARE_DIR}/sessions/<md5(work_dir)>/<session_id>/`. Subagent instances are stored under this root."
  - name: KIMI_CLI_GIT_BASH_PATH
    effect: "Override the path to `bash.exe` on Windows. Kimi CLI's Windows runtime requires Git for Windows for its `Shell` tool. Has no effect on macOS / Linux. The default discovery order is `KIMI_CLI_GIT_BASH_PATH` → `where.exe git` → `git --exec-path` → `C:\\Program Files\\Git\\bin\\bash.exe`."
  - name: KIMI_BUILD_SHA
    effect: "Override the build identifier (`remote@sha`) shown by `kimi --version`. Read by `kimi_cli.constant.get_build_sha` (priority 1)."
  - name: KIMI_CODE_OAUTH_KEY
    effect: "Override the OAuth keyring key for Moonshot OAuth credentials. Read by `kimi_cli.auth.oauth`."
  - name: KIMI_GIT_BASH_INSTALL_HINT (read-only constant, not a flag)
    effect: "Not an env var; the `Environment._GIT_BASH_INSTALL_HINT` constant surfaces a message when bash is missing on Windows."
  - name: KIMI_PROXY_*
    effect: "Generic HTTP/HTTPS proxy environment variables. Routed by `kimi_cli.utils.proxy.normalize_proxy_env` at startup; affects both parent and subagent outbound HTTP (MCP servers, OAuth calls, web tools)."
  - name: KIMI_SHELL_PATH (new kimi-code only)
    effect: "Override the absolute path of `bash.exe` on Windows for the new `kimi-code` (TypeScript) install. Documented in the `kimi-code` README. Not recognized by legacy `kimi-cli` (which uses `KIMI_CLI_GIT_BASH_PATH` instead)."
  - name: KIMI_CODE_HOME (new kimi-code only)
    effect: "Override the `kimi-code` data root. Default: `~/.kimi-code/`. Moves the entire `config.toml` / `tui.toml` / `sessions/` / `agents/` / `skills/` / `logs/` tree. Documented in `https://moonshotai.github.io/kimi-code/en/configuration/config-files`."
  - name: KIMI_API_KEY (new kimi-code only)
    effect: "Per the new `kimi-code` config docs, the CLI does NOT auto-fall-back to `KIMI_API_KEY` from the shell; the API key must be written into `config.toml`. The `KIMI_MODEL_API_KEY` env var is honored when `KIMI_MODEL_NAME` is set."
  - name: KIMI_MODEL_NAME / KIMI_MODEL_* (new kimi-code only)
    effect: "If `KIMI_MODEL_NAME` is set, the new `kimi-code` synthesizes a provider + model from `KIMI_MODEL_PROVIDER_TYPE`, `KIMI_MODEL_API_KEY`, `KIMI_MODEL_BASE_URL`, `KIMI_MODEL_MAX_CONTEXT_SIZE`, `KIMI_MODEL_CAPABILITIES` and makes it the default. Applied by `applyEnvModelConfig` in `packages/agent-core/src/config/env-model.ts`."

changes: []

requires_claudine_update: true
reason: |
  Claudine's `linking` module currently recognizes Claude Code, Codex, Goose, OpenCode, and Qwen (the 8-provider roster) but has no Kimi CLI row. The Kimi subagent story is the most unusual of the eight because it is split across two implementations:

  1. **Legacy `kimi-cli` (Python, `MoonshotAI/kimi-cli`)** — `first_class` support: a real `AgentSpec` schema (`src/kimi_cli/agentspec.py`), a `LaborMarket` registry (`src/kimi_cli/subagents/registry.py`), and per-instance `SubagentStore` (`src/kimi_cli/subagents/store.py`). The `--agent <default|okabe>` and `--agent-file <path>` flags are the entry points; there is no auto-discovered user/repo agent directory. The build-time `system_prompt_path` is a sibling Markdown file; the `subagents:` block is the per-agent subagent registry.

  2. **New `kimi-code` (TypeScript, `MoonshotAI/kimi-code`)** — `none` for user-defined agent files. The new `kimi-code` has no `--agent` or `--agent-file` flag (verified against `kimi --help` on the locally installed `0.14.0`). The runtime keeps the three built-in subagent types (`coder`, `explore`, `plan`) but no longer exposes a user-defined agent YAML. Users who want to customize behavior write `$KIMI_CODE_HOME/AGENTS.md`, `~/.agents/AGENTS.md`, `.kimi-code/AGENTS.md`, or `.kimi/AGENTS.md` (instruction files — a different topic).

  Claudine's agent linker must therefore:

  - Walk the **built-in agent directories** at `kimi_cli/agents/{default,okabe}/agent.yaml` to seed the `claudine agents` listing (a `pip show kimi-cli | grep Location` lookup, or the same Python-relative resolution `get_agents_dir()` uses). Surface the two root agents with their per-subagent subdirectories (`coder.yaml`, `explore.yaml`, `plan.yaml`) as the catalog.
  - Recognize `--agent-file` as the per-session user-agent entry point. There is **no** user-global or repo-scanned agent directory to walk; user agents are flag-loaded only. The linker should not search `~/.kimi/agents/` or `.kimi/agents/`.
  - Cross-walk the `agent:` mapping into the linker-row schema. Required fields: `name`, `system_prompt_path`, `tools`. Optional: `extend`, `system_prompt_args`, `model`, `when_to_use`, `allowed_tools`, `exclude_tools`, `subagents`. Each `subagents:<name>` entry is a separate linker row whose `path` is the `path:` value resolved against the parent file's directory.
  - Recognize the two subagent storage layouts and expose them through `claudine logs` / a future `--subagent` flag. For the legacy kimi-cli, subagent state lives in `<session-dir>/subagents/<agent_id>/{meta.json, context.jsonl, wire.jsonl, prompt.txt, output}`. For the new kimi-code, subagent state lives in `<session-dir>/agents/<agent_id>/wire.jsonl` (per the new docs; the same `agent_id` shape — `a<8-hex>` — is preserved).
  - Honor the four-tier `AgentTool` model-resolution ladder when sizing per-invocation overrides: `Agent(model)` → `AgentLaunchSpec.effective_model` → `subagent_type.default_model` → parent model. A wrapper that wants to reproduce a subagent run must capture all four.
  - Recognize the **`Agent` tool's `resume` parameter** as the canonical subagent-resume path. `SubagentStore.require_instance(agent_id)` raises `FileNotFoundError` if the instance is missing; `update_instance` flips the status atomically. The hooks topic (separately owned) records the `SubagentStart` / `SubagentStop` payload shape and the matcher convention; this topic owns only the agent-definition half.
  - Treat the new `kimi-code` `--agent` / `--agent-file` removal as a **break** for cross-linkers. The docs explicitly say "Installing Kimi Code CLI automatically migrates your configuration and sessions" and the `kimi migrate` subcommand handles the conversion. A user-defined YAML from `kimi-cli` does **not** flow forward; the only way to express the same customization in `kimi-code` is through `AGENTS.md` (a different topic) or through the new `plugins/` system (also a different topic — see the plugins research).
---

# Kimi Code CLI Subagents

## Overview

Kimi Code CLI is published under two related but distinct implementations: the legacy Python `kimi-cli` (`MoonshotAI/kimi-cli`, last release 1.48.0, 2026-06-22) and the new TypeScript `kimi-code` (`MoonshotAI/kimi-code`, last release 0.22.1, 2026-07-02). The legacy README states directly: **"Kimi CLI is evolving into Kimi Code CLI — the next-generation terminal AI agent from the same team. Installing Kimi Code CLI automatically migrates your configuration and sessions. This project will be gradually wound down; the docs and existing installations remain available."** The two share the same agent / subagent vocabulary — `coder`, `explore`, `plan` — and the same `Agent` tool shape, but they expose **different** customization surfaces for the definition side of the feature:

- The legacy `kimi-cli` is `first_class` for user-defined agents and subagents. A user authors a `version: 1` YAML file with an `agent:` mapping, registers built-in subagent types in the `subagents:` block, and points `kimi` at the file with `kimi --agent-file /path/to/agent.yaml`. The two shipped root agents (`default` and `okabe`) ship as `kimi_cli/agents/{default,okabe}/agent.yaml` inside the wheel and are selected with `kimi --agent default` or `kimi --agent okabe`.
- The new `kimi-code` has **dropped** `--agent` and `--agent-file` (verified against the locally installed 0.14.0 `kimi --help` and the published `apps/kimi-code/src/cli/options.ts`). It keeps the three built-in subagent types and lets the main agent schedule them automatically; users who want to alter behavior write `$KIMI_CODE_HOME/AGENTS.md`, `~/.agents/AGENTS.md`, `.kimi-code/AGENTS.md`, or `.kimi/AGENTS.md` instead. The agent-definition topic for the new runtime is `none` (the surface is owned by the instruction-files topic, not this one).

This document treats the legacy `kimi-cli` as the canonical source of truth for **user-defined** agents because that is the surface that exists today. The new `kimi-code` is documented where its subagent lifecycle diverges from the legacy one (storage layout, hook events, env vars) so Claudine can handle both binaries without confusing them.

The provider calls the feature "Agents and Subagents" (`docs/en/customization/agents.html`). The runtime vocabulary is "agent" (the root main agent) and "subagent" (any child launched by the `Agent` tool); user-defined subagent types are registered as entries in the parent agent's `subagents:` block and exposed as `subagent_type` enum values to the parent model. There is no "mode", "persona", or "worker" terminology — Kimi CLI uses a single word, **subagent**, for every variant of the feature.

## Locations

Kimi CLI's agent locations split cleanly into three layers: built-in package data, per-session runtime state, and the per-invocation `--agent-file` override. The user / repo / extension layers that exist in Claude Code and Goose do **not** exist for Kimi CLI's user-defined agent files — there is no auto-discovered `~/.kimi/agents/` directory, and the CLI rejects `--agent-file` paths outside the file system. The new `kimi-code` removes the per-invocation override entirely (no `--agent` / `--agent-file` flag in the new CLI surface), so the user-defined-agent location record collapses to the built-in `kimi_cli/agents/{default,okabe}/agent.yaml` paths.

| Scope | macOS | Linux | Windows | Notes |
|---|---|---|---|---|
| Built-in package | `<site-packages>/kimi_cli/agents/{default,okabe}/agent.yaml` | same | `<site-packages>\kimi_cli\agents\{default,okabe}\agent.yaml` | Resolved by `kimi_cli.agentspec.get_agents_dir()` to the package's own `agents/` subdirectory. Each agent is a directory containing `agent.yaml`, `system.md`, and the per-subagent YAMLs (`coder.yaml`, `explore.yaml`, `plan.yaml`). Selected by `kimi --agent default` or `kimi --agent okabe`. |
| Per-invocation user agent | `--agent-file <path>` (per-session only) | same | `--agent-file <path>` (per-session only) | No on-disk persistence; the file lives wherever the user keeps it. Any user-readable path is accepted. The file format is identical to the built-in `agent.yaml`. |
| Subagent runtime state (per session) | `<session-dir>/subagents/<agent_id>/{meta.json, context.jsonl, wire.jsonl, prompt.txt, output}` | same | `<session-dir>\subagents\<agent_id>\...` | `<session-dir>` is `~/.kimi/sessions/<md5(work_dir)>/<session_id>/` (or `$KIMI_SHARE_DIR/sessions/...`). `<agent_id>` is `a<8-hex>`. The new `kimi-code` (TypeScript) writes the same subagent state to `<session-dir>/agents/<agent_id>/wire.jsonl` per its docs. |
| Plugin-provided agent (hypothetical) | n/a | n/a | n/a | The legacy `kimi-cli` plugin loader (`kimi_cli.plugin.tool.load_plugin_tools`) loads plugin **tools**, not agent files. There is no documented plugin-agent surface in `kimi-cli` main as of 1.48.0. Treat as a planned-but-undefined shape. |

`SubagentStore.root` is `<session>/subagents/`, computed by `Session.subagents_dir`. The `get_share_dir()` helper resolves to `${KIMI_SHARE_DIR:-$HOME/.kimi}/`, then `sessions/` is appended. The MD5 in the path is `hashlib.md5(work_dir.encode()).hexdigest()`, so two sessions on the same work directory share the parent session directory but have separate session subdirectories.

On this host (macOS) we have the new `kimi-code` installed at `/Users/ken/.kimi-code/` and a leftover `~/.kimi/` from the legacy install. The new install is at version 0.14.0, the legacy share dir is mostly empty (the `kimi migrate` subcommand has already been run — see `~/.kimi/.migrated-to-kimi-code`).

## Definition Format

A Kimi CLI agent file is a two-key YAML envelope: a `version: 1` scalar and an `agent:` mapping. There is no Markdown body — the system prompt lives in a separate Markdown file referenced by `system_prompt_path`, and subagent definitions live in additional YAML files referenced by the `subagents:` block.

```yaml
version: 1
agent:
  name: code-reviewer
  extend: default
  system_prompt_path: ./system.md
  system_prompt_args:
    ROLE_ADDITIONAL: |
      You are a focused code-review subagent. Be terse; flag every blocking issue
      with a file/line and a one-line fix.
  model: kimi-for-coding
  when_to_use: |
    Use this agent for code review of a diff before merging.
  allowed_tools:
    - "kimi_cli.tools.shell:Shell"
    - "kimi_cli.tools.file:ReadFile"
    - "kimi_cli.tools.file:ReadMediaFile"
    - "kimi_cli.tools.file:Glob"
    - "kimi_cli.tools.file:Grep"
    - "kimi_cli.tools.web:SearchWeb"
    - "kimi_cli.tools.web:FetchURL"
  exclude_tools:
    - "kimi_cli.tools.agent:Agent"
    - "kimi_cli.tools.ask_user:AskUserQuestion"
    - "kimi_cli.tools.todo:SetTodoList"
    - "kimi_cli.tools.plan:ExitPlanMode"
    - "kimi_cli.tools.plan.enter:EnterPlanMode"
    - "kimi_cli.tools.file:WriteFile"
    - "kimi_cli.tools.file:StrReplaceFile"
  subagents:
    reviewer:
      path: ./reviewer-sub.yaml
      description: "Strict code reviewer; flags correctness issues and security smells."
    auditor:
      path: ./auditor-sub.yaml
      description: "Read-only dependency and license auditor."
```

The same `agent:` mapping is the input to `kimi_cli.agentspec.load_agent_spec`. Fields are validated by the pydantic `AgentSpec` model (`src/kimi_cli/agentspec.py`):

| Field | Type | Required after `extend` resolution | Notes |
|---|---|---|---|
| `version` | string `1` | yes (top-level) | The only supported value. |
| `agent` | mapping | yes | Contains every other field. |
| `agent.extend` | string | no | `default` to inherit the built-in `default/agent.yaml`, or a relative path to another `agent.yaml`. Recursively resolved. |
| `agent.name` | string | required when not inheriting | The agent's name. Inherited when omitted. |
| `agent.system_prompt_path` | path | required when not inheriting | Relative to the agent YAML's directory. Resolved to an absolute path by `load_agent_spec`. |
| `agent.system_prompt_args` | map[string,string] | no | Merged (not replaced) on `extend`. Exposed as `${KEY}` in the system prompt. |
| `agent.model` | string (model alias) | no | Becomes the subagent's `default_model` when this agent is registered as a subagent. |
| `agent.when_to_use` | string | no | Surfaced in the `Agent` tool's `${BUILTIN_AGENT_TYPES_MD}` description. |
| `agent.tools` | array[string] | required when not inheriting | `module:ClassName` allowlist used when `allowed_tools` is null. |
| `agent.allowed_tools` | array[string] \| null | no | Explicit allowlist; when non-null, the subagent's `ToolPolicy.mode` is `allowlist` and the subagent only sees these tools. |
| `agent.exclude_tools` | array[string] | no | Denylist relative to the inherited `tools`. |
| `agent.subagents` | map[string, SubagentSpec] | no | Each value is `{path: <relative path>, description: <string>}`. `path` is resolved relative to the parent file's directory. |

The system prompt Markdown supports Jinja2 templating with the `Agent`'s own custom environment — `variable_start_string = "${"`, `variable_end_string = "}"`, `lstrip_blocks = True`, `trim_blocks = True`, `undefined = StrictUndefined`. Built-in variables (`KIMI_NOW`, `KIMI_WORK_DIR`, `KIMI_WORK_DIR_LS`, `KIMI_AGENTS_MD`, `KIMI_SKILLS`, `KIMI_ADDITIONAL_DIRS_INFO`, `KIMI_OS`, `KIMI_SHELL`) are passed as keyword arguments, and `system_prompt_args` are passed as `**args`. `{% include %}` directives pull in additional files (the loader uses `FileSystemLoader(path.parent)`).

The subagent file format is the same `version: 1` + `agent:` envelope, but typically the only meaningful keys are `extend: <parent-path>`, `system_prompt_args.ROLE_ADDITIONAL`, `when_to_use`, `allowed_tools`, and `exclude_tools`. The default `coder`, `explore`, and `plan` shipped with `kimi-cli` 1.48.0 are worked examples.

## Runtime Behavior

The `Agent` tool (`kimi_cli.tools.agent:Agent`, registered with the toolset at `kimi_cli.tools.agent.AgentTool`) is the only entry point for subagent invocation. The parent main agent loads the `Agent` tool from its `tools` list (or `allowed_tools` if the parent overrides the toolset) and the `AgentTool.__init__` injects the runtime's `LaborMarket`'s registered subagent types into the tool description as `${BUILTIN_AGENT_TYPES_MD}`. The parent's prompt therefore sees a per-type summary like ``- `coder`: General software engineering (Tools: Shell, ReadFile, ..., Model: kimi-for-coding, Background: yes). When to use: ...``.

The `Agent` tool's parameter shape is:

| Parameter | Type | Default | Notes |
|---|---|---|---|
| `description` | string (3–5 words) | required | Short label; persisted to the subagent instance's `meta.json`. |
| `prompt` | string | required | Full task text; the subagent does **not** see the parent's conversation. |
| `subagent_type` | string | `"coder"` | One of the parent's `subagents:` block keys. |
| `model` | string \| null | null | Per-invocation model alias override; validated against `runtime.config.models`. |
| `resume` | string \| null | null | Re-enter a previously-created subagent by `agent_id`. |
| `run_in_background` | bool | `false` | Switch from foreground to background runner. |
| `timeout` | int \| null | null | Wall-clock timeout, 30–3600 s. Foreground: defaults to no timeout. Background: defaults to `BackgroundConfig.agent_task_timeout_s` (900 s). |

Foreground execution is the default. `ForegroundSubagentRunner` builds the subagent with `SubagentBuilder.build_builtin_instance(agent_id, type_def, launch_spec)`, restores the subagent's `Context` from `SubagentStore.context_path(agent_id)`, runs `KimiSoul` against the prompt, then runs the optional `SUMMARY_CONTINUATION_PROMPT` if the final assistant text is shorter than 200 characters. The runner returns a single `ToolReturnValue` (the `ToolOk` case) with the final assistant text, or a `ToolError` with a `brief` of `Max steps reached` / `API error (<status>)` / `LLM provider error` / `Agent run error` / `Agent timed out (<t>s)` / `Agent not found` / `Invalid subagent type` / `Background start failed` / `Agent unavailable` / `Agent already running`.

Background execution routes through `BackgroundTaskManager.create_agent_task`, which records the `task_id`, `kind`, `status`, `description`, `agent_id`, `actual_subagent_type`, and the `resume_hint` line. The task id and the agent id are both persisted to `SubagentStore.update_instance` so the parent can re-enter with `Agent(resume="<agent_id>", …)` or read the result with `TaskOutput(task_id="…", block=true)`. Stale foreground records (`status == "running_foreground"` at the start of a new session) are reaped by `KimiCLI._cleanup_stale_foreground_subagents` so a previous parent's death does not leave a stuck instance.

## Observability

Subagent lifecycle is visible to wrappers through three coordinated surfaces:

1. **Wire-protocol events** — the parent's wire log contains a `SubagentEvent` envelope (`kimi_cli.wire.types.SubagentEvent`) that wraps any of the regular `Event` types emitted by the subagent. The envelope's `parent_tool_call_id` lets a consumer correlate the subagent's tool calls with the `Agent` tool call in the parent; `agent_id` and `subagent_type` identify the instance. The wrapped `Event` is one of: `StepBegin` (every step), `StepInterrupted` (user / error interruption), `StepRetry` (retry with `n`, `next_attempt`, `max_attempts`, `wait_s`, `error_type`, `status_code`), `CompactionBegin` / `CompactionEnd` (auto-compaction), `StatusUpdate` (`context_usage`, `context_tokens`, `max_context_tokens`, `token_usage`, `message_id`, `plan_mode`, `mcp_status`), `Notification` (bubbled from the subagent's `NotificationManager`), `HookTriggered` / `HookResolved` (hook lifecycle). In `kimi --prompt --output-format stream-json`, these stream to stdout in NDJSON.
2. **Subagent wire log** — `<session-dir>/subagents/<agent_id>/wire.jsonl` is a per-instance NDJSON of every event the subagent itself emitted. A consumer that wants the raw subagent timeline (without the parent's interleaving) reads this file. The parent's `SubagentEvent` envelopes and the subagent's own wire share the same schema, so the two can be merged by `agent_id`.
3. **Hook events** — `kimi_cli.hooks.engine.HookEngine` fires the documented Kimi-CLI hook event set, which includes `SubagentStart` (payload: `agent_name`, `prompt`) and `SubagentStop` (payload: `agent_name`, `response`). The matcher targets the subagent type name (e.g. `matcher = "coder"`). Both payloads are also available to the `Agent` tool's parent via the `SubagentEvent` envelope. Other lifecycle events (`PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `UserPromptSubmit`, `SessionStart`, `SessionEnd`, `PreCompact`, `PostCompact`, `Stop`, `StopFailure`, `Notification`) are defined in `kimi_cli.hooks.config.HookEventType` and built by `kimi_cli.hooks.events.<event_name>`.

The new `kimi-code` (TypeScript) documents the same `SubagentStart` / `SubagentStop` hook events at `https://moonshotai.github.io/kimi-code/en/customization/hooks.html` and stores subagent state under `<session-dir>/agents/<agent_id>/wire.jsonl`. The TypeScript runtime does not yet publish the `SubagentEvent` envelope on the new docs; treat the new TS stream-event surface as `partial` until the SDK source confirms it.

## Portability

Kimi CLI's agent files are **not portable** across providers as-is. The `agent:` mapping and the `version: 1` envelope are Kimi-CLI-specific; the body system prompt (a sibling Markdown file) is the most portable bit when it does not reference Kimi-specific tools or env vars, but Kimi's definition is in the YAML mapping, not the body. A cross-provider rewrite requires a translation pass.

| Kimi CLI field | Claude Code equivalent | Goose equivalent | Codex equivalent |
|---|---|---|---|
| `version: 1` + `agent:` envelope | n/a (Claude Code uses a Markdown file with YAML frontmatter) | n/a (Goose uses `*.md` frontmatter) | n/a (Codex uses TOML `agents/<name>.toml`) |
| `agent.name` | frontmatter `name` | frontmatter `name` (Goose is more permissive; slugify on cross-link) | TOML `name` |
| `agent.system_prompt_path` (Markdown file) | body of the `.md` file (Claude Code's subagent body IS the system prompt) | body of the `.md` agent | `developer_instructions` |
| `agent.system_prompt_args.ROLE_ADDITIONAL` | inline into body (no separate arg map in Claude Code) | inline into body (Goose's `properties` bag is the closest, but is provider-internal) | inline into `developer_instructions` |
| `agent.model` (alias) | frontmatter `model` (`sonnet` / `opus` / `haiku` / `fable` / full ID / `inherit`) | frontmatter `model` (Goose does not currently propagate it to runtime) | TOML `model` |
| `agent.tools` / `agent.allowed_tools` / `agent.exclude_tools` | frontmatter `tools` allowlist (`Agent(…)` allowlist is Claude-specific) | per-invocation `delegate.extensions: [...]` (no per-definition allowlist) | TOML `tools` |
| `agent.subagents.{name}.{path, description}` | n/a (Claude Code has no per-agent subagent registry; subagent types are global `Agent` tool entries) | n/a (Goose's subagents are global `SourceType::Agent` entries, not nested) | n/a (Codex has no nested-subagent block) |
| `agent.when_to_use` | frontmatter `description` (Claude uses description as the routing signal) | frontmatter `description` (Goose surfaces it through `summon`) | TOML `description` |

Subagent storage layouts are not interchangeable: Kimi CLI's `<session-dir>/subagents/<agent_id>/{context.jsonl, wire.jsonl, meta.json, prompt.txt, output}` is a per-subagent directory of multiple files; Claude Code writes a single `agent-{agentId}.jsonl` per subagent under `~/.claude/projects/{project}/{sessionId}/subagents/`. A linker that wants to surface Kimi subagent state to a Claude-Code consumer must rebuild the JSONL shape from the Kimi files.

The new `kimi-code` does not have a `--agent-file` flag, so the cross-link from `claudine agents` would have to (a) keep the link as a Kimi-CLI-only path and warn that it does not apply to `kimi-code`, or (b) translate the YAML into a `KIMI_CODE_HOME/AGENTS.md` instruction block (a different topic) — losing the per-tool allowlist, the per-subagent composition, and the inheritance model.

## Claudine Linking Notes

For Claudine's `linking` module and the planned lifecycle `proxy` / `resume` actions, what matters about Kimi CLI subagents:

- Treat `kimi_cli/agents/{default,okabe}/agent.yaml` as the canonical built-in agent location, discoverable via `kimi_cli.agentspec.get_agents_dir()` from the same `pip show kimi-cli | grep Location` lookup Claudine already uses for other Python-installed tools. The per-subagent YAMLs (`coder.yaml`, `explore.yaml`, `plan.yaml`) are separate linker rows whose `path` field is resolved relative to the parent agent file's directory.
- The `--agent-file` flag is the only user-defined-agent entry point. There is **no** `~/.kimi/agents/` or `.kimi/agents/` directory to walk; the user-agent surface is flag-loaded only. A `claudine agents` row that claims to find a Kimi user agent at a fixed path is wrong.
- Subagent storage has two shapes. Legacy `kimi-cli` writes `<session-dir>/subagents/<agent_id>/{meta.json, context.jsonl, wire.jsonl, prompt.txt, output}`. New `kimi-code` (TypeScript) writes `<session-dir>/agents/<agent_id>/wire.jsonl`. A wrapper that wants to correlate subagent runs across both binaries must check which `~/.kimi` vs `~/.kimi-code` data root is in use (`KIMI_SHARE_DIR` for legacy, `KIMI_CODE_HOME` for new) and read the matching layout.
- A linked Kimi agent is portable when its `system_prompt_args.ROLE_ADDITIONAL` and `system_prompt_path` Markdown body carry only natural-language guidance. Flag assets that depend on `extend: default` (which resolves to the legacy wheel's built-in path), `kimi_cli.tools.*:<ClassName>` tool references, the `subagents:` block, the `model:` alias, or the `when_to_use` field — they need rewriting, stripping, or host gating before they can land elsewhere.
- For lifecycle `proxy` / `resume`: the wrapper must capture and replay the `agent_id` (`a<8-hex>`) for the subagent it wants to address. The subagent's stable storage at `<session-dir>/subagents/<agent_id>/` is the source of truth for resume; the parent's wire log's `SubagentEvent` envelopes carry the same `agent_id` and `subagent_type` so a wrapper can correlate without re-reading the subagent files. The `Agent(resume="<agent_id>", …)` shape is the canonical resume path; a `task_id` (background only) is the secondary handle.
- Model resolution for any per-invocation override must follow the four-tier ladder: `Agent(model)` → `AgentLaunchSpec.effective_model` → `subagent_type.default_model` → parent model. A wrapper that pre-loads a model override must use `runtime.config.models` validation, not free-form strings, because the `Agent` tool rejects unknown aliases.
- Permission policy: the parent's YOLO/AFK modes and approval allow/deny rules propagate through the shared `Approval` runtime, so the subagent's effective policy is identical to the parent's. Plan mode is parent-scoped only and does not propagate.
- Whenever Claudine's wrapper code grows a `kimi agents` command or a `--agent-file` resolution path, it should resolve the build-time `agents/` directory with the same Python-relative lookup the legacy kimi-cli uses (`Path(__file__).parent / "agents"` from `kimi_cli.agentspec`), and it should treat the new `kimi-code` install as a `none` for user-defined-agent discovery (use the new AGENTS.md instruction-files topic instead). The `kimi migrate` subcommand is the supported upgrade path for users who authored custom YAML agents in the legacy kimi-cli.

## Sources

- [Kimi Code CLI — Agents and Subagents (legacy kimi-cli docs)](https://moonshotai.github.io/kimi-cli/en/customization/agents.html)
- [Kimi Code CLI — Agents and Sub-Agents (new kimi-code docs)](https://moonshotai.github.io/kimi-code/en/customization/agents.html)
- [Kimi Code CLI — Configuration files (new kimi-code docs)](https://moonshotai.github.io/kimi-code/en/configuration/config-files)
- [Kimi Code CLI — Agent Skills (new kimi-code docs)](https://moonshotai.github.io/kimi-code/en/customization/skills.html)
- [Kimi Code CLI — Hooks (new kimi-code docs)](https://moonshotai.github.io/kimi-code/en/customization/hooks.html)
- [Kimi Code CLI — Hooks (legacy kimi-cli docs)](https://moonshotai.github.io/kimi-cli/en/customization/hooks.html)
- [Kimi Code CLI — Command reference (legacy kimi-cli docs)](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html)
- [MoonshotAI/kimi-cli repository (legacy Python)](https://github.com/MoonshotAI/kimi-cli)
- [MoonshotAI/kimi-code repository (new TypeScript)](https://github.com/MoonshotAI/kimi-code)
- [kimi-cli source — `src/kimi_cli/agentspec.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/agentspec.py)
- [kimi-cli source — `src/kimi_cli/agents/default/agent.yaml`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/agents/default/agent.yaml)
- [kimi-cli source — `src/kimi_cli/agents/default/coder.yaml`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/agents/default/coder.yaml)
- [kimi-cli source — `src/kimi_cli/agents/default/explore.yaml`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/agents/default/explore.yaml)
- [kimi-cli source — `src/kimi_cli/agents/default/plan.yaml`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/agents/default/plan.yaml)
- [kimi-cli source — `src/kimi_cli/agents/okabe/agent.yaml`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/agents/okabe/agent.yaml)
- [kimi-cli source — `src/kimi_cli/tools/agent/__init__.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/tools/agent/__init__.py)
- [kimi-cli source — `src/kimi_cli/tools/agent/description.md`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/tools/agent/description.md)
- [kimi-cli source — `src/kimi_cli/subagents/builder.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/subagents/builder.py)
- [kimi-cli source — `src/kimi_cli/subagents/core.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/subagents/core.py)
- [kimi-cli source — `src/kimi_cli/subagents/models.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/subagents/models.py)
- [kimi-cli source — `src/kimi_cli/subagents/registry.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/subagents/registry.py)
- [kimi-cli source — `src/kimi_cli/subagents/store.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/subagents/store.py)
- [kimi-cli source — `src/kimi_cli/subagents/runner.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/subagents/runner.py)
- [kimi-cli source — `src/kimi_cli/soul/agent.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/soul/agent.py)
- [kimi-cli source — `src/kimi_cli/session.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/session.py)
- [kimi-cli source — `src/kimi_cli/hooks/config.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/hooks/config.py)
- [kimi-cli source — `src/kimi_cli/hooks/engine.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/hooks/engine.py)
- [kimi-cli source — `src/kimi_cli/hooks/events.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/hooks/events.py)
- [kimi-cli source — `src/kimi_cli/wire/types.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/types.py)
- [kimi-cli source — `src/kimi_cli/cli/__init__.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/cli/__init__.py)
- [kimi-cli source — `src/kimi_cli/share.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/share.py)
- [kimi-code source — `apps/kimi-code/src/cli/options.ts`](https://github.com/MoonshotAI/kimi-code/blob/main/apps/kimi-code/src/cli/options.ts)
- [kimi-code source — `packages/agent-core/src/config/env-model.ts`](https://github.com/MoonshotAI/kimi-code/blob/main/packages/agent-core/src/config/env-model.ts)
- [kimi-code source — `packages/agent-core/src/config/kimi-env-params.ts`](https://github.com/MoonshotAI/kimi-code/blob/main/packages/agent-core/src/config/kimi-env-params.ts)
- [Kimi Code (product homepage)](https://www.kimi.com/code/)
