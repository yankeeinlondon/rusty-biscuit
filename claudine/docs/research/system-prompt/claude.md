---
$schema: ./_schema.yaml
created: '2026-03-30'
last_updated: '2026-07-03'
agent: open_code
model: minimax/MiniMax-M3
docs: https://code.claude.com/docs/en/cli-reference
system_prompt_docs: https://code.claude.com/docs/en/agent-sdk/modifying-system-prompts
append_support: native
replace_support: native
cli_params:
- flag: --append-system-prompt
  mode: append
  value_shape: string
  description: Append custom text to the end of the default system prompt.
  example: claude --append-system-prompt "Always use TypeScript"
  notes: Temporary per-invocation addition. Works in interactive and non-interactive modes. Can be combined with --system-prompt
    or --system-prompt-file. Appends after the default engineering instructions or after the active output style.
- flag: --append-system-prompt-file
  mode: append
  value_shape: path
  description: Load additional system prompt text from a file and append it to the default prompt.
  example: claude --append-system-prompt-file ./extra-rules.txt
  notes: 'Absent from `claude --help` output (the CLI does not list every flag) but accepted by the binary; tested locally
    on v2.1.200 with a non-existent path returning "Append system prompt file not found: <path>". Can be combined with replacement
    flags.'
- flag: --system-prompt
  mode: replace
  value_shape: string
  description: Replace the entire default system prompt with custom text.
  example: claude --system-prompt "You are a Python expert"
  notes: Mutually exclusive with --system-prompt-file. Drops built-in tool guidance, safety instructions, and coding conventions;
    the caller takes responsibility for anything the task still needs. CLAUDE.md, memory, and skills still load as project
    context.
- flag: --system-prompt-file
  mode: replace
  value_shape: path
  description: Load a system prompt from a file, replacing the default prompt.
  example: claude --system-prompt-file ./custom-prompt.txt
  notes: Absent from `claude --help` output but accepted by the binary; tested locally on v2.1.200. Mutually exclusive with
    --system-prompt.
- flag: --exclude-dynamic-system-prompt-sections
  mode: modify
  value_shape: boolean
  description: Move per-machine sections (working directory, environment info, memory paths, git-repo flag) from the system
    prompt into the first user message to improve cross-machine prompt-cache reuse.
  example: claude -p --exclude-dynamic-system-prompt-sections "query"
  notes: 'Only applies with the default system prompt; ignored when --system-prompt or --system-prompt-file is set. SDK equivalent
    is `excludeDynamicSections: true` on the preset object.'
- flag: --bare
  mode: disable
  value_shape: boolean
  description: Minimal mode. Skip hooks, LSP, plugin sync, attribution, auto-memory, background prefetches, keychain reads,
    and CLAUDE.md auto-discovery. Sets CLAUDE_CODE_SIMPLE=1. Anthropic auth becomes strictly ANTHROPIC_API_KEY or apiKeyHelper
    via --settings (OAuth and keychain are never read); 3P providers (Bedrock/Vertex/Foundry) use their own credentials. Skills
    still resolve via /skill-name.
  example: claude --bare -p "query"
  notes: Equivalent to CLAUDE_CODE_SIMPLE=1. As of v2.1.200 the doc text emphasizes providing context explicitly via --system-prompt[-file],
    --append-system-prompt[-file], --add-dir (CLAUDE.md dirs), --mcp-config, --settings, --agents, --plugin-dir.
- flag: --safe-mode
  mode: disable
  value_shape: boolean
  description: Start with CLAUDE.md, skills, plugins, hooks, MCP servers, custom commands and agents, output styles, workflows,
    custom themes, keybindings, status line, file-suggestion commands, LSP servers, and auto-memory disabled for troubleshooting.
    Authentication, model selection, built-in tools, and permissions still work normally. Managed policy still applies, including
    policy-configured hooks, status line, and file-suggestion commands; managed plugins, managed skills, managed CLAUDE.md,
    and policy-configured MCP servers do load.
  example: claude --safe-mode
  notes: Equivalent to CLAUDE_CODE_SAFE_MODE=1. Added in v2.1.169. Differs from --bare, which uses a minimal system prompt
    and reduced tool set.
- flag: --agent
  mode: replace
  value_shape: agent name
  description: Specify an agent for the current session. The main thread takes on that subagent's system prompt, tool restrictions,
    permission mode, MCP servers, hooks, skills, and model.
  example: claude --agent my-custom-agent
  notes: Replaces the default Claude Code system prompt entirely (same effect as --system-prompt). CLAUDE.md and project memory
    still load as context. Overrides the `agent` setting.
- flag: --agents
  mode: replace
  value_shape: JSON
  description: Define custom subagents dynamically via JSON. Uses the same field names as subagent frontmatter (description,
    prompt, tools, disallowedTools, model, permissionMode, mcpServers, hooks, maxTurns, skills, initialPrompt, memory, effort,
    background, isolation, color). The `prompt` field is the system prompt.
  example: claude --agents '{"reviewer":{"description":"Reviews code","prompt":"You are a code reviewer"}}'
  notes: Session-only; not persisted to disk. Multiple subagents can be defined in one call. Managed settings > --agents >
    project .claude/agents/ > user ~/.claude/agents/ > plugin agents/ on name collisions.
- flag: --setting-sources
  mode: modify
  value_shape: comma-separated list
  description: 'Restrict which settings scopes load: `user`, `project`, `local`. Empty list disables all non-managed scopes.'
  example: claude --setting-sources ""
  notes: Affects CLAUDE.md, agent, output style, and MCP discovery; `--setting-sources ""` together with explicit flags is
    the SDK pattern used by the `claude_code` preset.
- flag: --settings
  mode: modify
  value_shape: file or JSON string
  description: Path to a settings JSON file or a JSON string to load additional settings from.
  example: claude --settings ./override.json
  notes: Merges with user/project/local/managed settings.
- flag: --add-dir
  mode: other
  value_shape: path list
  description: Grant file access to additional working directories.
  example: claude --add-dir ../apps ../lib
  notes: By default, .claude/ configuration is not discovered from these directories; set CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD=1
    to also load CLAUDE.md, .claude/CLAUDE.md, .claude/rules/*.md, and CLAUDE.local.md from them.
- flag: --print / -p
  mode: other
  value_shape: boolean
  description: Non-interactive mode. Print response and exit.
  example: claude -p "explain this function"
  notes: Works with --append-system-prompt, --system-prompt, and --system-prompt-file. CLAUDE.md still loads. Workspace trust
    dialog is skipped in this mode.
config_sources:
- os: macos
  scope: system
  path: /Library/Application Support/ClaudeCode/CLAUDE.md
  mode: append
  format: markdown
  notes: Organization-wide managed CLAUDE.md; loaded before user and project CLAUDE.md and cannot be excluded by user or project
    settings (claudeMdExcludes does not apply).
- os: linux
  scope: system
  path: /etc/claude-code/CLAUDE.md
  mode: append
  format: markdown
  notes: Managed CLAUDE.md on Linux and WSL; cannot be excluded.
- os: windows
  scope: system
  path: C:\Program Files\ClaudeCode\CLAUDE.md
  mode: append
  format: markdown
  notes: Managed CLAUDE.md on Windows; cannot be excluded.
- os: macos
  scope: system
  path: /Library/Application Support/ClaudeCode/managed-settings.json
  mode: modify
  format: json
  notes: Managed settings file; can include `claudeMd` content and enforce policy; cannot be overridden by user/project settings.
    A drop-in directory `managed-settings.d/` alongside it is merged alphabetically with numeric prefix ordering.
- os: linux
  scope: system
  path: /etc/claude-code/managed-settings.json
  mode: modify
  format: json
  notes: Managed settings file; same content model as macOS path.
- os: windows
  scope: system
  path: C:\Program Files\ClaudeCode\managed-settings.json
  mode: modify
  format: json
  notes: Managed settings file. Legacy C:\ProgramData\ClaudeCode\managed-settings.json path is no longer supported as of v2.1.75.
- os: macos
  scope: system
  path: com.anthropic.claudecode (managed preferences domain / MDM plist)
  mode: modify
  format: json
  notes: MDM-deployed managed settings on macOS; plist top-level keys mirror managed-settings.json. Deploy via Jamf, Iru/Kandji,
    or similar MDM tools.
- os: windows
  scope: system
  path: HKLM\SOFTWARE\Policies\ClaudeCode (Settings REG_SZ or REG_EXPAND_SZ value)
  mode: modify
  format: json
  notes: Registry-deployed managed settings via Group Policy or Intune. HKCU\SOFTWARE\Policies\ClaudeCode is the user-level
    fallback.
- os: macos
  scope: user
  path: ~/.claude/CLAUDE.md
  mode: append
  format: markdown
  notes: Personal preferences loaded at the start of every session after managed CLAUDE.md and before project CLAUDE.md.
- os: linux
  scope: user
  path: ~/.claude/CLAUDE.md
  mode: append
  format: markdown
  notes: Linux path equivalent; same load order as macOS.
- os: windows
  scope: user
  path: '%USERPROFILE%\.claude\CLAUDE.md'
  mode: append
  format: markdown
  notes: Windows path equivalent; same load order as macOS/Linux.
- os: macos
  scope: repo
  path: ./CLAUDE.md or ./.claude/CLAUDE.md
  mode: append
  format: markdown
  notes: Project-level instructions; loaded after user CLAUDE.md. Shared via source control.
- os: linux
  scope: repo
  path: ./CLAUDE.md or ./.claude/CLAUDE.md
  mode: append
  format: markdown
  notes: Linux path equivalent.
- os: windows
  scope: repo
  path: .\CLAUDE.md or .\.claude\CLAUDE.md
  mode: append
  format: markdown
  notes: Windows path equivalent.
- os: macos
  scope: repo
  path: ./CLAUDE.local.md
  mode: append
  format: markdown
  notes: Personal project-specific preferences; should be added to .gitignore. Appended after CLAUDE.md within each directory.
- os: linux
  scope: repo
  path: ./CLAUDE.local.md
  mode: append
  format: markdown
  notes: Linux path equivalent.
- os: windows
  scope: repo
  path: .\CLAUDE.local.md
  mode: append
  format: markdown
  notes: Windows path equivalent.
- os: macos
  scope: repo
  path: ./.claude/rules/*.md
  mode: append
  format: markdown
  notes: Path-scoped rules loaded when Claude reads files matching their `paths:` frontmatter patterns. Rules without `paths:`
    are loaded unconditionally at launch.
- os: linux
  scope: repo
  path: ./.claude/rules/*.md
  mode: append
  format: markdown
  notes: Linux path equivalent.
- os: windows
  scope: repo
  path: .\.claude\rules\*.md
  mode: append
  format: markdown
  notes: Windows path equivalent.
- os: macos
  scope: user
  path: ~/.claude/output-styles/*.md
  mode: replace
  format: markdown
  notes: 'User-level output styles; selected via /config or `outputStyle` setting. Replaces default engineering instructions
    unless `keep-coding-instructions: true` is set in frontmatter.'
- os: linux
  scope: user
  path: ~/.claude/output-styles/*.md
  mode: replace
  format: markdown
  notes: Linux path equivalent.
- os: windows
  scope: user
  path: '%USERPROFILE%\.claude\output-styles\*.md'
  mode: replace
  format: markdown
  notes: Windows path equivalent.
- os: macos
  scope: repo
  path: ./.claude/output-styles/*.md
  mode: replace
  format: markdown
  notes: Project-level output styles; closest nested directory to the working directory wins on name collisions (v2.1.178+).
- os: linux
  scope: repo
  path: ./.claude/output-styles/*.md
  mode: replace
  format: markdown
  notes: Linux path equivalent.
- os: windows
  scope: repo
  path: .\.claude\output-styles\*.md
  mode: replace
  format: markdown
  notes: Windows path equivalent.
- os: macos
  scope: user
  path: ~/.claude/agents/*.md
  mode: replace
  format: markdown
  notes: User-level subagent definitions. The Markdown body is the subagent's system prompt; frontmatter sets tools, model,
    permissionMode, hooks, mcpServers, skills, memory, background, effort, isolation, color, and initialPrompt.
- os: linux
  scope: user
  path: ~/.claude/agents/*.md
  mode: replace
  format: markdown
  notes: Linux path equivalent.
- os: windows
  scope: user
  path: '%USERPROFILE%\.claude\agents\*.md'
  mode: replace
  format: markdown
  notes: Windows path equivalent.
- os: macos
  scope: repo
  path: ./.claude/agents/*.md
  mode: replace
  format: markdown
  notes: Project-level subagent definitions; closest nested directory to the working directory wins on name collisions (v2.1.178+).
- os: linux
  scope: repo
  path: ./.claude/agents/*.md
  mode: replace
  format: markdown
  notes: Linux path equivalent.
- os: windows
  scope: repo
  path: .\.claude\agents\*.md
  mode: replace
  format: markdown
  notes: Windows path equivalent.
- os: macos
  scope: user
  path: ~/.claude/settings.json
  mode: modify
  format: json
  notes: 'User-level settings. Prompt-affecting keys: `outputStyle`, `agent`, `includeGitInstructions`, `claudeMdExcludes`,
    `autoMemoryEnabled`, `autoMemoryDirectory`, `autoCompactEnabled`, `permissionMode`.'
- os: linux
  scope: user
  path: ~/.claude/settings.json
  mode: modify
  format: json
  notes: Linux path equivalent.
- os: windows
  scope: user
  path: '%USERPROFILE%\.claude\settings.json'
  mode: modify
  format: json
  notes: Windows path equivalent.
- os: macos
  scope: repo
  path: ./.claude/settings.json
  mode: modify
  format: json
  notes: Project settings; checked into source control.
- os: linux
  scope: repo
  path: ./.claude/settings.json
  mode: modify
  format: json
  notes: Linux path equivalent.
- os: windows
  scope: repo
  path: .\.claude\settings.json
  mode: modify
  format: json
  notes: Windows path equivalent.
- os: macos
  scope: repo
  path: ./.claude/settings.local.json
  mode: modify
  format: json
  notes: Local-only settings overrides; should be in .gitignore.
- os: linux
  scope: repo
  path: ./.claude/settings.local.json
  mode: modify
  format: json
  notes: Linux path equivalent.
- os: windows
  scope: repo
  path: .\.claude\settings.local.json
  mode: modify
  format: json
  notes: Windows path equivalent.
env_vars:
- name: CLAUDE_CODE_DISABLE_CLAUDE_MDS
  effect: Set to 1 to prevent loading any CLAUDE.md memory files into context, including user, project, and auto-memory files.
  mode: modify
- name: CLAUDE_CODE_DISABLE_AUTO_MEMORY
  effect: 'Set to 1 to disable auto memory (load and write). Set to 0 to force auto memory on even when --bare or autoMemoryEnabled:
    false would otherwise disable it.'
  mode: modify
- name: CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD
  effect: Set to 1 to load CLAUDE.md, .claude/CLAUDE.md, .claude/rules/*.md, and CLAUDE.local.md from directories passed via
    --add-dir. By default, --add-dir grants file access but does not load memory files.
  mode: modify
- name: CLAUDE_CODE_DISABLE_GIT_INSTRUCTIONS
  effect: Set to 1 to remove built-in commit and PR workflow instructions and the git status snapshot from the system prompt.
    Takes precedence over the includeGitInstructions setting.
  mode: modify
- name: CLAUDE_CODE_ATTRIBUTION_HEADER
  effect: Set to 0 to omit the attribution block (client version and prompt fingerprint) from the start of the system prompt.
    Improves prompt-cache hit rates when routing through an LLM gateway.
  mode: modify
- name: CLAUDE_CODE_SIMPLE
  effect: Set to 1 to run with a minimal system prompt and only the Bash, file read, and file edit tools. Disables auto-discovery
    of hooks, skills, plugins, MCP servers, auto memory, and CLAUDE.md. Equivalent to --bare.
  mode: disable
- name: CLAUDE_CODE_SIMPLE_SYSTEM_PROMPT
  effect: Set to 1 to use a shorter system prompt and abbreviated tool descriptions on any model. Set to 0/false/no/off to
    opt out. Full tool set, hooks, MCP servers, and CLAUDE.md discovery remain enabled.
  mode: modify
- name: CLAUDE_CODE_SAFE_MODE
  effect: Set to 1 to start with CLAUDE.md, skills, plugins, hooks, MCP servers, custom commands and agents, output styles,
    workflows, themes, keybindings, status line, file-suggestion commands, LSP servers, and auto-memory disabled. Equivalent
    to --safe-mode. Managed policy still applies. Inherited by directly spawned child processes.
  mode: disable
- name: CLAUDE_CODE_DISABLE_EXPLORE_PLAN_AGENTS
  effect: Set to 1 to disable only the built-in Explore and Plan subagents (Claude explores with its search tools or the general-purpose
    subagent instead). Custom subagents named Explore or Plan are unaffected. Requires v2.1.198 or later.
  mode: modify
- name: CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS
  effect: Set to 1 to disable every built-in subagent type (Explore, Plan, general-purpose). Only applies in non-interactive
    mode (-p) and the Agent SDK. Useful for SDK users who want a blank slate.
  mode: modify
- name: CLAUDE_CODE_FORK_SUBAGENT
  effect: Set to 1 to enable forked subagents that inherit the full conversation context. Overrides any server-side rollout.
  mode: modify
- name: CLAUDE_CODE_SUBAGENT_MODEL
  effect: Override the subagent model resolution. As of v2.1.196, setting it to inherit is the same as leaving it unset. Requires
    v2.1.196+ for the inherit behavior change.
  mode: modify
- name: CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS
  effect: Set to 1 to enable agent teams (experimental, disabled by default). As of v2.1.178 every session has one implicit
    team when enabled; TeamCreate/TeamDelete were removed.
  mode: modify
- name: CLAUDE_CODE_DISABLE_BACKGROUND_TASKS
  effect: Set to 1 to disable all background task functionality, including run_in_background on Bash and subagent tools, auto-backgrounding,
    and the Ctrl+B shortcut.
  mode: modify
- name: CLAUDE_CODE_DISABLE_AGENT_VIEW
  effect: Set to 1 to turn off background agents and agent view (claude agents, --bg, /background, on-demand supervisor).
    Equivalent to the disableAgentView setting.
  mode: modify
- name: CLAUDE_DISABLE_ADOPT
  effect: Set to 1 to stop in-flight background work instead of carrying it over when backgrounding a session with left-arrow
    or /background. Requires v2.1.195+.
  mode: modify
- name: CLAUDE_AUTO_BACKGROUND_TASKS
  effect: Set to 1 to force automatic backgrounding of long-running agent tasks. Subagents are moved to the background after
    approximately two minutes.
  mode: modify
- name: CLAUDE_ASYNC_AGENT_STALL_TIMEOUT_MS
  effect: Stall timeout in milliseconds for background subagents (default 600000 / 10 min). Resets on each streaming progress
    event.
  mode: other
- name: TASK_MAX_OUTPUT_LENGTH
  effect: Maximum number of characters in subagent output before truncation (default 32000, max 160000).
  mode: other
- name: DISABLE_PROMPT_CACHING
  effect: Set to 1 to disable prompt caching for all models (takes precedence over per-model settings).
  mode: modify
- name: CLAUDE_AX_SCREEN_READER
  effect: 'Set to 1 to render screen-reader friendly output (flat text, no decorative borders or animations). Precedence:
    --ax-screen-reader flag > this env var > axScreenReader setting.'
  mode: other
prompt_layers:
- source: Default system prompt ("claude_code" preset)
  mode: replace
  scope:
  - builtin
  order_notes: Base layer; replaced entirely by --system-prompt or --system-prompt-file.
  notes: Includes tool guidance, safety instructions, coding conventions, and dynamic environment context. Contains per-machine
    sections (cwd, env info, memory paths, git-repo flag) that break prompt-cache reuse across machines unless --exclude-dynamic-system-prompt-sections
    is set.
- source: Output style
  mode: replace
  scope:
  - session
  order_notes: Applied after the default engineering instructions and before --append-system-prompt.
  notes: 'Selected via outputStyle setting, /config, or plugin force-for-plugin. Frontmatter `keep-coding-instructions: true`
    decides whether the style replaces or layers on the default. Changes only take effect after /clear or restart.'
- source: --append-system-prompt / --append-system-prompt-file
  mode: append
  scope:
  - session
  order_notes: Appended after the default system prompt (after any output style).
  notes: Temporary per-invocation additions. Combine freely with --system-prompt or --system-prompt-file.
- source: Managed CLAUDE.md / claudeMd setting
  mode: append
  scope:
  - organization
  order_notes: Loaded before user and project CLAUDE.md.
  notes: Cannot be excluded by claudeMdExcludes. Injected as a project-context message, not into the system prompt itself.
- source: User CLAUDE.md (~/.claude/CLAUDE.md)
  mode: append
  scope:
  - user
  order_notes: Loaded after managed and before project CLAUDE.md.
  notes: Injected as project context, not into the system prompt itself.
- source: Project CLAUDE.md (./CLAUDE.md or ./.claude/CLAUDE.md)
  mode: append
  scope:
  - repo
  order_notes: Loaded after user CLAUDE.md.
  notes: Injected as project context, not into the system prompt itself. Imports via @path expand at launch.
- source: CLAUDE.local.md
  mode: append
  scope:
  - repo
  order_notes: Appended after CLAUDE.md within each directory.
  notes: Injected as project context.
- source: Auto memory MEMORY.md
  mode: append
  scope:
  - repo
  order_notes: First 200 lines or 25 KB of MEMORY.md loaded at startup.
  notes: Injected as project context, not into the system prompt itself. Topic files load on demand via Read.
- source: Path-scoped rules (.claude/rules/*.md with `paths:` frontmatter)
  mode: append
  scope:
  - repo
  order_notes: Loaded when Claude reads files matching the paths glob.
  notes: Injected as project context. v2.1.198+ matches symlinked paths to the project directory.
- source: Skills (via Skill tool or `skills:` preload)
  mode: append
  scope:
  - session
  - subagent
  order_notes: Load on demand via Skill tool, or preloaded into subagent context via frontmatter.
  notes: Loaded into the conversation as context, not into the system prompt itself.
- source: Subagent system prompt
  mode: replace
  scope:
  - subagent
  order_notes: Custom system prompt for the subagent; runs in an isolated context window.
  notes: Replaces the default Claude Code system prompt for that subagent (same as --system-prompt). CLAUDE.md and auto memory
    still load unless the subagent is the built-in Explore or Plan, which skip CLAUDE.md and the parent git status.
agent_prompting:
  supported: true
  definition_surface: 'Markdown files with YAML frontmatter in ~/.claude/agents/, ./.claude/agents/, managed .claude/agents/,
    or plugin agents/. Inline via --agents JSON (uses frontmatter field names plus prompt). /agents wizard was removed in
    v2.1.198: ask Claude to write the file or edit directly.'
  inheritance: Subagents receive their own system prompt (the file body) plus basic environment details (working directory);
    they do NOT receive the full Claude Code system prompt. CLAUDE.md and auto memory load for every built-in and custom subagent
    except Explore and Plan, which skip CLAUDE.md and the parent git status. Skills can be preloaded via the `skills:` frontmatter
    field. As of v2.1.198 subagents also inherit the main session's extended thinking configuration.
  isolation: 'Each subagent runs in its own context window; only the final summary returns to the parent. Subagents can also
    use `isolation: worktree` to run in a temporary git worktree branched from the default branch (not the parent''s HEAD).'
  limitations: Cannot spawn AskUserQuestion, EnterPlanMode, ExitPlanMode (unless permissionMode is plan), ScheduleWakeup,
    or WaitForMcpServers. Subagents are capped at 5 levels of nested depth (foreground and background, since v2.1.181/2.1.172).
    Background subagents surface permission prompts in the main session (v2.1.186+). Built-in Explore and Plan skip CLAUDE.md
    and parent git status. Plugin subagents cannot use the hooks, mcpServers, or permissionMode frontmatter fields. Toolset
    restrictions listed in `tools`/`disallowedTools` cannot re-add tools removed by managed policy.
claudine_delivery:
  append_strategy: file_flag
  replace_strategy: file_flag
  temp_file_required: true
  argv_limit: No published argv limit for --append-system-prompt or --system-prompt; use the file variants for large prompts
    to avoid ARG_MAX pressure.
  notes: Claudine's wrapper accepts --append-system-prompt/--asp and --replace-system-prompt/--rsp as file paths. It discovers
    system-prompt.md from the launch-CWD hierarchy and passes the resolved content to Claude Code's native --append-system-prompt-file
    or --system-prompt-file flags. This avoids persistent mutation of user settings.json, CLAUDE.md, or output styles. Both
    file flags work in v2.1.200 (verified locally); --append-system-prompt-file and --system-prompt-file are absent from `claude
    --help` but accepted by the binary, so callers should not rely on --help to detect them.
format_recommendations:
  append_format: markdown
  replace_format: xml_wrapped_markdown
  rationale: 'Appending layers onto the already-structured claude_code preset works best with plain Markdown headers and lists.
    Replacing the entire prompt removes the default structure, so XML tags (e.g. <rules>, <constraints>, <context>, <examples>)
    help the model distinguish instruction categories. For Claude Code the Agent SDK documentation recommends a custom string
    or `{ type: preset, preset: claude_code, append: ... }` shape, so plain Markdown and Markdown-with-XML-tags are both idiomatic.'
recent_changes:
- date: '2026-07-02'
  version: 2.1.199
  change: Fixed subagents cut off by a rate limit or server error silently failing instead of returning partial work to the
    parent; subagents reporting API errors (e.g. usage limit reached) are now reported as errors instead of success. CLAUDE_CODE_MAX_RETRIES
    cap raised to 15; CLAUDE_CODE_RETRY_WATCHDOG raises the default retry count for transient errors to 300 and lifts the
    cap.
  impact: Affects how subagent failures surface to the parent; transient-error retry policy is now dramatically more permissive
    under CLAUDE_CODE_RETRY_WATCHDOG=1.
- date: '2026-07-01'
  version: 2.1.198
  change: Subagents now run in the background by default; built-in Explore inherits the main session's model (capped at Opus);
    subagents and context compaction inherit the session's extended thinking configuration; the /agents wizard was removed
    (ask Claude or edit .claude/agents/ directly); path-scoped rules now match when the target file is reached via a symlinked
    path.
  impact: Changes default lifecycle (background) and cost profile of subagents; the Explore agent's model follows the main
    session; agent prompts remain isolated.
- date: '2026-06-29'
  version: 2.1.196
  change: CLAUDE_CODE_SUBAGENT_MODEL=inherit now behaves like unset (resolution continues with the per-invocation model parameter,
    then the frontmatter); v2.1.186 prior behavior forced subagents onto the main conversation's model.
  impact: Subagent model resolution is now more predictable; older docs/scripts that assumed inherit forced the main model
    should be updated.
- date: '2026-06-24'
  version: 2.1.191
  change: Foreground subagents now respect the same 5-level depth limit as background subagents.
  impact: Prevents unbounded nested agent recursion.
- date: '2026-06-23'
  version: 2.1.187
  change: org-configured model restrictions now apply to /model, --model, ANTHROPIC_MODEL, and the /advisor picker with a
    'restricted by your organization's settings' message when a restricted model is selected.
  impact: 'Indirect prompt-surface change: switching models can fail closed on managed deployments.'
- date: '2026-06-22'
  version: 2.1.186
  change: Background subagents surface permission prompts in the main session with the agent name; Esc denies just that tool;
    sandbox credentials setting added; Agent(type) deny rules and Agent(x,y) allowlist restrictions now enforced for named
    subagent spawns; skill frontmatter accepts kebab-case/snake_case/camelCase for display-name, default-enabled, fallback,
    and metadata.* keys.
  impact: Background subagents no longer auto-deny; permission policy gains Agent(agent_type) syntax; skill metadata becomes
    more forgiving.
- date: '2026-06-19'
  version: 2.1.183
  change: Added /config key=value syntax to set any setting from the prompt (e.g. /config thinking=false), works in interactive,
    -p, and Remote Control. Added auto-mode safety to block destructive git commands and terraform/pulumi/cdk destroy unless
    explicitly asked. Deprecated/auto-updated model warnings now also cover agent frontmatter.
  impact: Output style (a system prompt layer) can be changed mid-session via /config but only takes effect after /clear or
    restart. The agent frontmatter model warning closes a gap where a stale frontmatter would silently use a deprecated model.
- date: '2026-06-17'
  version: 2.1.181
  change: Added /config key=value syntax for any setting, including outputStyle, from the prompt. Added CLAUDE_CLIENT_PRESENCE_FILE
    env var. Foreground subagents now respect the 5-level depth limit. Improved the subagent panel (idle auto-hide, 5-row
    cap, scroll hints).
  impact: Output style (a system prompt layer) can be changed mid-session via /config but only takes effect after /clear or
    restart.
- date: '2026-06-15'
  version: 2.1.178
  change: Nested .claude/ directories closest to the working directory now win when output style, agent, workflow, or skill
    names collide. Skills in nested .claude/skills appear as <dir>:<name>. TeamCreate/TeamDelete tools removed; CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1
    enables one implicit team per session.
  impact: Allows monorepo-style prompt layering at the subdirectory level; standardizes naming and team model.
- date: '2026-06-08'
  version: 2.1.169
  change: Added --safe-mode flag and CLAUDE_CODE_SAFE_MODE environment variable to disable CLAUDE.md, output styles, skills,
    plugins, hooks, MCP, and auto-memory for troubleshooting. Added /cd command that preserves prompt cache. Managed settings
    with invalid entries apply their remaining valid policies and surface the validation error instead of silently dropping
    the whole payload. The 'CLAUDE.md is too long' warning threshold now scales with the model's context window.
  impact: Provides a clean way to verify whether the effective prompt is being affected by local customizations; managed-settings
    robustness improved.
- date: '2026-06-22'
  version: 2.1.186
  change: CLAUDE_CODE_MAX_RETRIES capped at 15; CLAUDE_CODE_RETRY_WATCHDOG introduced for unattended sessions.
  impact: Affects retry behavior but not the system prompt surface directly.
quirks:
- CLAUDE.md content is injected as a project-context message after the system prompt, not into the system prompt itself; it
  is softer than --append-system-prompt.
- --append-system-prompt text is appended after the active output style, so a configured output style with conflicting rules
  can shadow appended rules. When --system-prompt is used, output style and append flags can still be combined but only the
  append still applies to the replaced prompt.
- The default system prompt embeds per-machine sections (cwd, platform, shell, OS version, memory paths, git-repo flag), which
  invalidates prompt cache across different machines unless --exclude-dynamic-system-prompt-sections (or excludeDynamicSections
  in the SDK preset object) is used.
- Replacing the system prompt drops all built-in tool guidance and safety instructions; the caller must re-implement anything
  the task still needs. CLAUDE.md and memory still load as context.
- --system-prompt and --system-prompt-file are mutually exclusive; append flags can be combined with either replacement flag.
  --append-system-prompt and --append-system-prompt-file can also be combined with each other.
- --append-system-prompt-file and --system-prompt-file are not listed in `claude --help` output but the binary accepts them;
  do not rely on --help to detect them in scripts (verified locally on v2.1.200).
- Project-root CLAUDE.md and CLAUDE.local.md survive /compact and are re-injected; nested subdirectory CLAUDE.md files do
  not reload automatically after compaction, only when Claude next reads a file in that subdirectory.
- Managed policy CLAUDE.md cannot be excluded by claudeMdExcludes. claudeMdExcludes accepts glob patterns or absolute paths;
  only user, project, and local memory files are excludable.
- claudeMd in settings.json is honored only in managed or policy settings and ignored in user, project, and local settings.
- Output style changes take effect after /clear or restart because output style is part of the system prompt, which is read
  once at session start.
- Skills are loaded into the conversation, not into the system prompt; they are softer than --append-system-prompt but load
  automatically when Claude determines they are relevant (model-invocation) or only on explicit Skill calls.
- --safe-mode disables auto-discovery but does not change the system prompt surface; --bare replaces the system prompt with
  a minimal one and only allows Bash, Read, Edit.
- 'Managed settings parse tolerantly since v2.1.169: invalid entries are stripped with a warning and the valid subset is enforced.
  Per-field strict-mode overrides apply for security-enforcement fields (allowedMcpServers, allowManagedMcpServersOnly, availableModels,
  enforceAvailableModels, forceLoginOrgUUID, deniedMcpServers, sandbox.credentials).'
- The /output-style command was deprecated in v2.1.73 and removed in v2.1.91; use /config or edit the outputStyle setting
  directly.
- Subagents run in the background by default since v2.1.198; before that, Claude chose foreground vs. background based on
  the task. Background subagents surface every permission prompt in the main session (v2.1.186+) with the agent name shown.
- The /agents wizard was removed in v2.1.198; create agents by asking Claude to write them or by editing .claude/agents/ directly.
- CLAUDE_CODE_DISABLE_EXPLORE_PLAN_AGENTS=1 (v2.1.198+) removes only the Explore and Plan built-in subagents; CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS=1
  removes every built-in type but only in -p and the Agent SDK.
- CLAUDE_CODE_ATTRIBUTION_HEADER=0 is the recommended setting when routing through an LLM gateway because the per-client attribution
  block otherwise breaks prompt-cache hit rates across users.
- Block-level HTML comments in CLAUDE.md files are stripped before injection, so they can be used for human maintainer notes
  without spending context tokens; comments inside code blocks are preserved.
gaps:
- Anthropic does not publish the full default system prompt, so exact token counts and section ordering can only be inferred
  from documentation, /context output, and observed behavior.
- No documented API exports or inspects the effective built-in system prompt as plain text; /context shows only high-level
  sections and token counts.
- The behavior of multiple --append-system-prompt / --append-system-prompt-file flags in one command is undocumented (whether
  they concatenate, conflict, or take last-wins).
- The exact merge behavior when --setting-sources is given an empty string is undocumented beyond 'disable all non-managed
  scopes'; some sources (like auto-memory storage) may still load.
- The CLAUDE.md source-order relative to CLAUDE.md imports (@path) for memory files in nested subdirectories is not fully
  documented.
- Whether --append-system-prompt text is visible to subagents spawned in the same session is undocumented; the docs describe
  subagents as receiving only their own system prompt plus basic environment details.
- The exact precedence of claudeMdExcludes patterns vs. deep nested CLAUDE.md discovery in monorepos is not fully documented;
  /doctor reports loaded files but does not explain why an excluded file was excluded.
changes:
- '2026-07-03 refresh: split os: all config_sources into per-OS records (macos/linux/windows) to satisfy the schema; added
  2.1.199 (2026-07-02) to recent_changes; added newly documented env vars (CLAUDE_CODE_DISABLE_CLAUDE_MDS, CLAUDE_CODE_DISABLE_EXPLORE_PLAN_AGENTS,
  CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS, CLAUDE_CODE_FORK_SUBAGENT, CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS, CLAUDE_CODE_DISABLE_BACKGROUND_TASKS,
  CLAUDE_CODE_DISABLE_AGENT_VIEW, CLAUDE_DISABLE_ADOPT, CLAUDE_AUTO_BACKGROUND_TASKS, CLAUDE_ASYNC_AGENT_STALL_TIMEOUT_MS,
  TASK_MAX_OUTPUT_LENGTH, CLAUDE_CODE_SIMPLE_SYSTEM_PROMPT, DISABLE_PROMPT_CACHING); added --agents, --agent, --setting-sources,
  --settings, --add-dir, --print flags; recorded managed-settings.json, MDM plist, and registry paths per OS; updated agent_prompting
  inheritance to reflect the new docs (CLAUDE.md loads for subagents except Explore/Plan; subagents no longer receive the
  full Claude Code system prompt; /agents wizard removed in v2.1.198); added quirks for managed-settings tolerant parsing
  and /output-style removal; verified locally on v2.1.200 that --append-system-prompt-file and --system-prompt-file are accepted
  by the binary even though absent from `claude --help`.'
requires_claudine_update: false
reason: 'Claude Code''s native CLI already provides direct --append-system-prompt-file and --system-prompt-file flags, which
  align with Claudine''s file-based delivery model. No new wrapper mechanism is required. The schema-driven split of os: all
  into per-OS records is a formatting fix to satisfy _schema.yaml, not a behavior change.'
---


## Overview

Claude Code builds the effective prompt for every session from several ordered layers. The base is the unpublished default system prompt (the `claude_code` preset), which contains tool-use guidance, safety instructions, coding conventions, and dynamic environment context. On top of that, optional output styles, per-invocation append/replace flags, and project-context files such as `CLAUDE.md` and auto memory shape what Claude knows and how it behaves. Subagents run with their own isolated system prompts and return only a summary to the parent session.

Claudine’s wrapper strategy mirrors Claude Code’s first-class delivery: write the resolved prompt to a temporary file and pass `--append-system-prompt-file` or `--system-prompt-file`. Both file flags are accepted by the binary in v2.1.200 even though `claude --help` does not list them, so the wrapper does not need to fall back to inline text for large prompts.

## CLI Parameters

Claude Code exposes two pairs of flags that directly manipulate the system prompt for a single invocation. They work in both interactive and non-interactive (`-p`) modes.

| Flag | Mode | Effect |
| :--- | :--- | :--- |
| `--append-system-prompt "<text>"` | Append | Adds text to the end of the default system prompt. |
| `--append-system-prompt-file <path>` | Append | Adds the contents of a file to the end of the default system prompt. |
| `--system-prompt "<text>"` | Replace | Replaces the entire default system prompt with the supplied text. |
| `--system-prompt-file <path>` | Replace | Replaces the entire default system prompt with the contents of a file. |

`--system-prompt` and `--system-prompt-file` are mutually exclusive. The append flags can be combined with either replacement flag (or with each other). Replacement drops the built-in tool guidance and safety instructions, so it is only appropriate when the new prompt supplies everything the task needs. Append is the safer default because it preserves the `claude_code` preset.

Adjacent flags that affect the system prompt or its surrounding surface:

| Flag | Effect |
| :--- | :--- |
| `--exclude-dynamic-system-prompt-sections` | Moves per-machine sections (working directory, environment info, memory paths, git-repo flag) out of the system prompt and into the first user message to improve cross-machine prompt-cache reuse. Ignored when replacing the system prompt. |
| `--bare` | Uses a minimal system prompt with only Bash, file read, and file edit tools; skips hooks, LSP, plugin sync, attribution, auto-memory, background prefetches, keychain reads, and CLAUDE.md auto-discovery. Sets `CLAUDE_CODE_SIMPLE=1`. |
| `--safe-mode` | Disables CLAUDE.md, output styles, skills, plugins, hooks, MCP servers, custom commands and agents, workflows, themes, keybindings, status line, file-suggestion commands, LSP servers, and auto-memory. Managed policy still applies. Sets `CLAUDE_CODE_SAFE_MODE=1`. |
| `--agent <name>` | Runs the main thread as a named subagent, applying that agent's system prompt, tool restrictions, permission mode, MCP servers, hooks, skills, and model. Replaces the default system prompt the same way `--system-prompt` does. |
| `--agents <json>` | Defines custom subagents inline via JSON for the current session (frontmatter field names plus `prompt`). Multiple subagents per call. |
| `--setting-sources <list>` | Restricts which settings scopes load (`user`, `project`, `local`). Empty list disables all non-managed scopes. |
| `--settings <file-or-json>` | Loads additional settings from a file or JSON string. |
| `--add-dir <path…>` | Grants file access to additional working directories. Use with `CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD=1` to also load memory files from them. |
| `--print / -p` | Non-interactive mode; works with every flag in this table. |

## Configuration and Discovery

Beyond CLI flags, Claude Code discovers persistent instruction sources automatically.

### CLAUDE.md hierarchy

`CLAUDE.md` files are plain Markdown files that load at session start. They are injected into the conversation as project context rather than into the system prompt itself. Discovery walks up the directory tree from the working directory, loading files in this order:

1. Managed policy `CLAUDE.md` (macOS `/Library/Application Support/ClaudeCode/CLAUDE.md`, Linux/WSL `/etc/claude-code/CLAUDE.md`, Windows `C:\Program Files\ClaudeCode\CLAUDE.md`).
2. User `~/.claude/CLAUDE.md` (or `%USERPROFILE%\.claude\CLAUDE.md` on Windows).
3. Project `./CLAUDE.md` or `./.claude/CLAUDE.md`.
4. Project-local `./CLAUDE.local.md` alongside each loaded `CLAUDE.md`.

Within each directory, `CLAUDE.local.md` is appended after `CLAUDE.md`. Files closer to the working directory are loaded later, so more specific instructions take precedence. Subdirectory `CLAUDE.md` files under the working directory load on demand when Claude reads files in those subdirectories. As of v2.1.198 path-scoped rules under `.claude/rules/*.md` also match when the target file is reached via a symlinked path to the project directory.

CLAUDE.md files support `@path` imports (relative or absolute, recursive up to four hops, code-fence-aware parsing). Imported files load at launch and contribute to context size. Block-level HTML comments are stripped before injection; comments inside code blocks are preserved. The `--init` flow can generate a starter `CLAUDE.md`; `CLAUDE_CODE_NEW_INIT=1` enables an interactive multi-phase flow.

### Output styles

Output styles are Markdown files stored in `~/.claude/output-styles/`, `./.claude/output-styles/`, or the managed `.claude/output-styles/` directory. They directly modify the system prompt and can replace the default engineering instructions unless their frontmatter sets `keep-coding-instructions: true`. They are activated via the `outputStyle` setting in `settings.json` or through `/config`. Because output styles are part of the system prompt, changes only take effect after `/clear` or a new session. Plugin `output-styles/` directories are scanned as well, and a plugin can set `force-for-plugin: true` to apply its style automatically. As of v2.1.178, when nested `.claude/output-styles/` directories define styles with the same name, the one closest to the working directory wins.

### Settings files

`settings.json` at user, project, or local scope can set:

- "`outputStyle`: selects an output style."
- "`agent`: runs the main thread as a named subagent, applying that agent's system prompt and restrictions."
- "`includeGitInstructions`: includes or excludes built-in commit/PR workflow instructions and the git status snapshot."
- "`claudeMdExcludes`: glob patterns that skip specific `CLAUDE.md` files (managed policy files cannot be excluded)."
- "`claudeMd`: managed-only inline CLAUDE.md-style instructions (honored only in managed/policy settings)."
- `autoMemoryEnabled`, `autoMemoryDirectory`, `autoCompactEnabled`, `permissionMode`, and other runtime keys described in the settings reference.

`managed-settings.json`, MDM plist (`com.anthropic.claudecode`), and the Windows registry key `HKLM\SOFTWARE\Policies\ClaudeCode` (or `HKCU\…` for user-level) deliver organization-wide managed settings. Managed settings parse tolerantly since v2.1.169: invalid entries are stripped with a warning and the valid subset is enforced, with stricter per-field overrides for security-enforcement keys (`allowedMcpServers`, `availableModels`, etc.).

### Subagent definitions

Custom subagents are Markdown files with YAML frontmatter in `~/.claude/agents/`, `./.claude/agents/`, managed `.claude/agents/`, or plugin `agents/` directories. The file body becomes the subagent's system prompt. Definitions can also be passed inline for a single session via the `--agents` CLI flag or configured through managed settings. Subagent definitions are also available to agent teams when spawning teammates: the teammate uses the definition's `tools` and `model`, with the definition's body appended to the teammate's system prompt as additional instructions. Plugin subagents cannot use `hooks`, `mcpServers`, or `permissionMode` frontmatter fields.

## Prompt Layers and Precedence

The final context for a session is assembled from the following layers, from most foundational to most specific.

```mermaid
graph TD
    A[Default claude_code system prompt] --> B{Output style configured?}
    B -- yes --> C[Output style instructions]
    B -- no --> D[Default engineering instructions]
    C --> E[--append-system-prompt / --append-system-prompt-file]
    D --> E
    E --> F[Managed CLAUDE.md / claudeMd]
    F --> G[User CLAUDE.md]
    G --> H[Project CLAUDE.md]
    H --> I[CLAUDE.local.md]
    I --> J[Auto memory MEMORY.md]
    J --> K[Path-scoped rules loaded on demand]
    K --> L[Skills loaded on demand or via preload]
    L --> M[User prompt]
```

Notes on precedence:

- `--system-prompt` or `--system-prompt-file` replaces layers A, B, C, D, and E entirely; `CLAUDE.md` and memory still load as project context.
- `--append-system-prompt` and `--append-system-prompt-file` add to layer E after any output style.
- `CLAUDE.md`, auto memory, and skills are user/project-context messages, not part of the system prompt, so they are softer than `--append-system-prompt`.
- Managed policy `CLAUDE.md` cannot be skipped with `claudeMdExcludes`.

## Agents and Subagents

Claude Code supports custom agents defined as Markdown files with YAML frontmatter in `~/.claude/agents/`, `./.claude/agents/`, the managed `.claude/agents/`, or plugin `agents/` directories. Each subagent has its own system prompt (the file body), its own tool allowlist or denylist, an optional model, permission mode, MCP servers, hooks, skills, memory scope, effort level, background flag, isolation mode, and display color. Definitions can also be passed inline via the `--agents` JSON flag.

Key behaviors:

- Subagents run in isolated context windows. Only the final summary returns to the parent. Subagents do not inherit the full Claude Code system prompt — they receive their own system prompt plus basic environment details (working directory). CLAUDE.md and auto memory load for every built-in and custom subagent except Explore and Plan, which skip CLAUDE.md and the parent git status.
- The main session can run as a subagent via `claude --agent <name>` or the `agent` setting; this replaces the default system prompt with the agent's system prompt, the same way `--system-prompt` does. CLAUDE.md and project memory still load.
- Built-in subagents include Explore, Plan, and general-purpose. As of v2.1.198 Explore inherits the main session's model (capped at Opus) and subagents run in the background by default; both foreground and background subagents respect a five-level depth limit (since v2.1.181/2.1.172).
- The `Agent` tool replaced the older `Task` tool in v2.1.63; `Task(...)` references still work as aliases. Agent teams (`CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`) remove `TeamCreate`/`TeamDelete` and give every session an implicit team; teammates can be spawned directly via the Agent tool's `name` parameter.
- Background subagents surface every permission prompt in the main session with the agent name (since v2.1.186); Esc denies just that tool. To disable built-in Explore and Plan only, set `CLAUDE_CODE_DISABLE_EXPLORE_PLAN_AGENTS=1` (v2.1.198+). To disable every built-in subagent type in `-p` or the Agent SDK, set `CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS=1`.

## Format Recommendations

| Goal | Recommended format | Rationale |
| :--- | :--- | :--- |
| Append | Pure Markdown | Headers, bullet lists, and short paragraphs blend cleanly with the existing structured system prompt. |
| Replace | XML-wrapped Markdown | When the built-in structure is removed, XML tags such as `<rules>`, `<constraints>`, `<context>`, and `<examples>` help the model distinguish instruction categories. |

For replacements, the prompt must explicitly supply any tool-calling guidance, safety instructions, and environment context the task requires because the default `claude_code` preset is removed entirely. The Agent SDK's documented preset form (`{ type: "preset", preset: "claude_code", append: "..." }`) accepts the same Markdown payload.

## Recent Changes

- "**v2.1.200 (2026-07-03)**: Local binary in use; current docs still label this `2.1.199`. The `--bare` doc text expanded to enumerate the surfaces skipped and to recommend explicit context sources for bare scripts."
- "**v2.1.199 (2026-07-02)**: Subagent transient-error reporting improved; `CLAUDE_CODE_MAX_RETRIES` cap raised to 15 and `CLAUDE_CODE_RETRY_WATCHDOG` raises the default retry count for non-capacity transient errors to 300."
- "**v2.1.198 (2026-07-01)**: Subagents now run in the background by default; built-in Explore inherits the main session's model (capped at Opus); subagents and context compaction inherit the session's extended thinking configuration; `/agents` wizard removed; path-scoped rules now match symlinked target paths."
- **v2.1.196 (2026-06-29)**: `CLAUDE_CODE_SUBAGENT_MODEL=inherit` now behaves like unset; the `claude agents` side panel was overhauled.
- "**v2.1.191 (2026-06-24)**: Foreground subagents now respect the same five-level depth limit as background subagents."
- "**v2.1.187 (2026-06-23)**: Org-configured model restrictions now apply to `/model`, `--model`, `ANTHROPIC_MODEL`, and `/advisor` with a \"restricted by your organization's settings\" message."
- "**v2.1.186 (2026-06-22)**: Background subagents surface permission prompts in the main session with the agent name; `Agent(agent_type)` deny/allowlist syntax enforced for named subagent spawns; sandbox credentials setting added; `CLAUDE_CODE_MAX_RETRIES` capped at 15 and `CLAUDE_CODE_RETRY_WATCHDOG` introduced."
- **v2.1.183 (2026-06-19)**: `/config key=value` syntax supports any setting; auto mode blocks destructive git and Terraform/Pulumi/CDK `destroy` commands unless explicitly asked; agent frontmatter model warnings now mirror the deprecated-model warning surface.
- **v2.1.181 (2026-06-17)**: `/config key=value` syntax added for any setting, including `outputStyle`, from the prompt. Foreground subagents now respect the 5-level depth limit.
- "**v2.1.178 (2026-06-15)**: Nested `.claude/` directories closest to the working directory now win on output-style, agent, workflow, and skill name collisions; `TeamCreate`/`TeamDelete` removed; `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1` gives every session an implicit team."
- "**v2.1.169 (2026-06-08)**: Added `--safe-mode` flag and `CLAUDE_CODE_SAFE_MODE` to disable CLAUDE.md, output styles, skills, plugins, hooks, MCP, and auto-memory for troubleshooting. `/cd` preserves prompt cache. Managed settings with invalid entries apply their remaining valid policies and surface the validation error. CLAUDE.md \"too long\" warning threshold now scales with model context."

## Quirks and Workarounds

- `CLAUDE.md` is context, not enforcement. For behavior that must run at a specific lifecycle point, use a hook such as `PreToolUse` or `PostToolUse` rather than relying on `CLAUDE.md` alone.
- If an output style is configured, its instructions are placed before `--append-system-prompt` content, so style rules can shadow appended rules. When `--system-prompt` is used, the append flags still work but apply to the replaced prompt.
- "The default prompt includes dynamic per-machine sections that break prompt-cache reuse across machines. Use `--exclude-dynamic-system-prompt-sections` (CLI) or `excludeDynamicSections: true` (SDK) for scripted, multi-user workloads."
- Project-root `CLAUDE.md` survives `/compact`, but nested subdirectory `CLAUDE.md` files do not reload automatically; they come back when Claude next reads a file in that subdirectory.
- `--safe-mode` is the cleanest way to verify whether a misbehavior is caused by local customizations; `--bare` swaps in a minimal system prompt and tool set, which is useful for scripted runs.
- Replacing the system prompt is powerful but removes the built-in tool and safety guidance; most use cases are better served by `--append-system-prompt` or an output style.
- `--append-system-prompt-file` and `--system-prompt-file` are absent from `claude --help` but accepted by the binary in v2.1.200; do not rely on `--help` to detect them in scripts (verified locally).
- Subagents run in the background by default since v2.1.198; background subagents surface every permission prompt in the main session. Use `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS=1` to disable backgrounding entirely.
- "Managed settings parse tolerantly: invalid entries are stripped with a warning and the valid subset is enforced (per-field strict overrides exist for security-enforcement keys). `claudeMd` is honored only in managed or policy settings."
- The `/output-style` command was removed in v2.1.91; use `/config` or edit the `outputStyle` setting directly.
- Block-level HTML comments in CLAUDE.md files are stripped before injection, so they are useful for maintainer notes without spending context tokens; comments inside code blocks are preserved.

## Claudine Delivery Notes

Claudine should continue using its file-based delivery path:

- Discover a `system-prompt.md` file from the launch working-directory hierarchy.
- For append mode, write the resolved content to a temporary file and invoke Claude Code with `--append-system-prompt-file <tmp>`.
- For replace mode, write the resolved content to a temporary file and invoke Claude Code with `--system-prompt-file <tmp>`.
- Both modes are temporary per-invocation changes, so no user `settings.json`, `CLAUDE.md`, or output style is permanently mutated.
- Because Claude Code natively supports both inline and file flags, the file-backed approach avoids argv-length limits and keeps the wrapper simple. The local binary (v2.1.200) accepts both `--append-system-prompt-file` and `--system-prompt-file` even though they are absent from `claude --help`.

## Changelog

- "**2026-07-03 — refresh**: split `os: all` config_sources records into separate macos/linux/windows records to satisfy the `_schema.yaml` `os` enum (a formatting fix, not a behavior change). Added v2.1.199 to `recent_changes`. Added newly documented environment variables (`CLAUDE_CODE_DISABLE_CLAUDE_MDS`, `CLAUDE_CODE_DISABLE_EXPLORE_PLAN_AGENTS`, `CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS`, `CLAUDE_CODE_FORK_SUBAGENT`, `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS`, `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS`, `CLAUDE_CODE_DISABLE_AGENT_VIEW`, `CLAUDE_DISABLE_ADOPT`, `CLAUDE_AUTO_BACKGROUND_TASKS`, `CLAUDE_ASYNC_AGENT_STALL_TIMEOUT_MS`, `TASK_MAX_OUTPUT_LENGTH`, `CLAUDE_CODE_SIMPLE_SYSTEM_PROMPT`, `DISABLE_PROMPT_CACHING`). Added `--agents`, `--agent`, `--setting-sources`, `--settings`, `--add-dir`, `--print` flags. Recorded `managed-settings.json`, MDM plist, and Windows registry paths per OS. Updated `agent_prompting` to reflect the new docs (subagents no longer receive the full Claude Code system prompt; CLAUDE.md loads for every subagent except Explore/Plan; `/agents` wizard removed in v2.1.198). Added quirks for managed-settings tolerant parsing, the `/output-style` removal, and HTML comment stripping in CLAUDE.md. Verified locally on v2.1.200 that `--append-system-prompt-file` and `--system-prompt-file` are accepted by the binary even though absent from `claude --help`."
- "**2026-07-02 — prior refresh** (carried in): introduced `--append-system-prompt`, `--append-system-prompt-file`, `--system-prompt`, `--system-prompt-file`, `--exclude-dynamic-system-prompt-sections`, `--bare`, `--safe-mode`, `claudeMd` managed setting, output styles, subagents, CLAUDE.md hierarchy, env vars `CLAUDE_CODE_DISABLE_AUTO_MEMORY`/`CLAUDE_CODE_DISABLE_GIT_INSTRUCTIONS`/`CLAUDE_CODE_ATTRIBUTION_HEADER`/`CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD`/`CLAUDE_CODE_SIMPLE`/`CLAUDE_CODE_SAFE_MODE`/`CLAUDE_AX_SCREEN_READER`."

## Sources

- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Modifying system prompts (Agent SDK)](https://code.claude.com/docs/en/agent-sdk/modifying-system-prompts)
- [How Claude remembers your project](https://code.claude.com/docs/en/memory)
- [Claude Code settings](https://code.claude.com/docs/en/settings)
- [Create custom subagents](https://code.claude.com/docs/en/sub-agents)
- [Output styles](https://code.claude.com/docs/en/output-styles)
- [Explore the context window](https://code.claude.com/docs/en/context-window)
- [Environment variables](https://code.claude.com/docs/en/env-vars)
- [Claude Code changelog](https://code.claude.com/docs/en/changelog)
- [Headless mode (bare mode)](https://code.claude.com/docs/en/headless)
- [Agent SDK overview](https://code.claude.com/docs/en/agent-sdk/overview)