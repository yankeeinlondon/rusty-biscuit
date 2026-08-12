---
$schema: ./_schema.yaml
created: 2026-07-08
last_updated: 2026-07-08
agent: codex
model: default

homepage: https://antigravity.google/product/antigravity-cli
docs: https://antigravity.google/docs/cli/overview
subagent_docs: https://antigravity.google/docs/cli/subagents

support: first_class

locations:
  - os: macos
    scope: user
    path: ~/.gemini/config/agents/
    notes: "Global user subagents are created from the /agents panel under the shared Antigravity configuration root. Local 1.1.0 inspection found ~/.gemini/config/ but no user agent definitions."
  - os: linux
    scope: user
    path: ~/.gemini/config/agents/
    notes: "Same shared configuration root as macOS. The installed CLI and changelog name ~/.gemini/config/ as the global location scanned at startup."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\config\\agents\\"
    notes: "Windows home-relative equivalent of the shared ~/.gemini/config/ root. The Windows install is first-class, but this path was not locally observed from macOS."
  - os: macos
    scope: repo
    path: .agents/agents/
    notes: "Project customization root discovered by walking from the current directory to the repository root. The aliases .agent/, _agents/, and _agent/ are also customization roots."
  - os: linux
    scope: repo
    path: .agents/agents/
    notes: "Same as macOS; .agent/, _agents/, and _agent/ are accepted customization root aliases."
  - os: windows
    scope: repo
    path: ".agents\\agents\\"
    notes: "Same semantics using Windows separators; .agent\\, _agents\\, and _agent\\ are accepted customization root aliases."
  - os: macos
    scope: extension
    path: ".agents/plugins/<plugin_name>/agents/"
    notes: "The 1.0.1 changelog says plugin discovery scans installed plugin directories for custom skills and specialized agents. Bundled plugin docs list skills/rules/hooks/MCP but lag the changelog for agents."
  - os: linux
    scope: extension
    path: ".agents/plugins/<plugin_name>/agents/"
    notes: "Same plugin containment pattern as macOS."
  - os: windows
    scope: extension
    path: ".agents\\plugins\\<plugin_name>\\agents\\"
    notes: "Same plugin containment pattern as macOS using Windows separators."

format:
  file_names:
    - "agent.md"
    - "agents/<agent_name>/agent.md"
  frontmatter: true
  required_fields:
    - "name (inferred from Markdown agent artifact and Antigravity customization conventions; full public schema not found)"
    - "description (routing text used by the parent agent/UI; inferred from skill/customization conventions)"
  optional_fields:
    - "prompt/instructions body (Markdown body becomes the specialist instructions)"
    - "model (not proven for agent.md; session model can be selected with --model)"
    - "tools or tool permissions (not proven for agent.md; permissions are governed by global/project settings and hooks)"
    - "color/display metadata (not documented)"
    - "max turns (not documented)"
  body_format: markdown
  notes: |
    Antigravity CLI 1.0.16 changed dynamically defined subagents from JSON to Markdown, and 1.1.0 fixed the /agents panel header from `agent.json` to `agent.md`. The installed binary contains `agent.md`, `agent.json`, and `ListAgents` strings, which is consistent with the changelog.

    No local user subagent files were present under ~/.gemini/config/ during this run, and ~/.antigravity did not exist. The public documentation URL exists, but the currently fetchable static pages do not expose a complete `agent.md` frontmatter schema. Treat the required/optional field list as the minimum safe linker model until a concrete `agent.md` file or schema is observed.

runtime:
  invocation: "Interactive Antigravity CLI exposes `/agents` for subagent management and a `/tasks` panel for background tasks. The product page describes subagents as concurrent agent sessions for background delegation. The local keybinding map includes `subagent.approve_fast` on `ctrl+k` and `subagent.jump_to_waiting` on `alt+j`. No non-interactive `agy --agent <name>` flag is documented."
  parent_child_context: "A subagent runs as a separate conversation context. Binary strings describe a subagent as inheriting the parent agent's full configuration, tools, system prompt, and model while running in a separate conversation. Completed subagents are reported back to the parent, and the parent can view the subagent conversation log and send follow-up messages by conversation ID."
  permissions_inheritance: "Subagents inherit the parent configuration and permission environment, with special subagent approval UI. Changelog 1.0.14 added an 'always proceeds' mode for subagents to auto-approve artifacts, and 1.0.15 fixed subagent approval keybinding rendering. CLI `--dangerously-skip-permissions` applies to the session and should be treated as inherited by spawned subagents unless a future source proves narrowing."
  model_inheritance: "The installed binary text states that subagents inherit the parent model. The top-level session model can be selected with `--model`; no per-agent `agent.md` model field is proven."
  tool_inheritance: "The installed binary text states that subagents inherit parent tools. MCP servers, hooks, rules, permissions, and plugins are loaded through the same global/project customization roots and therefore affect the parent session before delegation."
  max_turns: "No documented per-subagent turn limit. Changelog 1.0.2 fixed an unintended default 60-second interaction timeout specifically for subagents; print mode has `--print-timeout` but that is not a subagent turn limit."
  notes: "Multiple subagents and background tasks can run concurrently. `/agents` and status indicators expose active subagents; `/tasks` exposes background task logs. Non-interactive print mode does not document an API for selecting a specific subagent."

observability:
  stream_events:
    - "interactive status indicator for active subagents and background tasks"
    - "/agents panel"
    - "/tasks panel and task detail logs"
    - "conversation log for each subagent"
    - "Stop hook input field `fullyIdle` indicates whether all background tasks are done"
  hook_events:
    - "PreInvocation"
    - "PostInvocation"
    - "Stop"
  session_ids: true
  notes: "The installed binary says completed subagents have conversation IDs and can be messaged with `send_message` by `conversation_id`. Changelog 1.0.6 says subagent conversations are skipped from `/resume`, so wrappers should not assume child conversations are resumable through the normal resume picker."

portability:
  portable: false
  non_portable_assets:
    - "agent.md frontmatter schema is provider-specific and not fully public"
    - "Antigravity tool names and permission settings"
    - "Antigravity/Gemini customization roots such as .agents/ and ~/.gemini/config/"
    - "plugin packaging and enablement state"
    - "conversation IDs and task logs"
  rewrite_needed: true
  notes: "Prompt body text can be reused, but Claudine should not link Antigravity agent definitions into another provider as-is. Convert body instructions and map provider-specific metadata, tools, and permissions to the target provider's subagent schema."

cli_params:
  - flag: "--model"
    description: "Sets the model for the current CLI session; inherited by subagents according to installed runtime strings."
    example: "agy --model gemini-2.5"
  - flag: "--mode"
    description: "Sets the agent execution mode for this session; accepted values shown by help/changelog include accept-edits and plan, with request-review/default available through settings."
    example: "agy --mode plan"
  - flag: "--sandbox"
    description: "Runs the session with terminal restrictions enabled."
    example: "agy --sandbox"
  - flag: "--dangerously-skip-permissions"
    description: "Auto-approves all tool permission requests without prompting; affects the session permission posture."
    example: "agy --dangerously-skip-permissions"
  - flag: "--add-dir"
    description: "Adds directories to the workspace; affects roots available to the parent and inherited subagent context."
    example: "agy --add-dir ../other-repo"
  - flag: "--project"
    description: "Selects the project ID for the current session; affects project-scoped settings and permissions."
    example: "agy --project default-cli-project"
  - flag: "--new-project"
    description: "Creates a new project for the session; affects project-scoped settings."
    example: "agy --new-project"
  - flag: "--continue"
    description: "Continues the most recent conversation; subagent conversations are documented as skipped from /resume."
    example: "agy --continue"
  - flag: "--conversation"
    description: "Resumes a previous conversation by ID; no documented support for launching directly into a subagent."
    example: "agy --conversation <conversation-id>"
  - flag: "--print"
    description: "Runs a single prompt non-interactively and prints the response; no subagent selection flag is documented for print mode."
    example: "agy --print 'summarize this repo'"
  - flag: "--print-timeout"
    description: "Timeout for print mode wait; not a per-subagent turn cap."
    example: "agy --print --print-timeout 10m 'run a quick audit'"
  - flag: "--prompt-interactive"
    description: "Runs an initial prompt interactively and continues the session."
    example: "agy --prompt-interactive 'review the current change'"
  - flag: "--log-file"
    description: "Overrides CLI log file path; useful for wrapper observability."
    example: "agy --log-file /tmp/agy.log"
  - flag: "plugin install"
    description: "Installs a plugin, which can package/discover specialized agents according to the changelog."
    example: "agy plugin install <target>"
  - flag: "plugin enable"
    description: "Enables an installed plugin and its packaged resources."
    example: "agy plugin enable <name>"
  - flag: "plugin disable"
    description: "Disables an installed plugin and its packaged resources."
    example: "agy plugin disable <name>"
  - flag: "plugin validate"
    description: "Validates plugin structure."
    example: "agy plugin validate .agents/plugins/team"

env_vars:
  - name: "AGY_CLI_DISABLE_LATEX"
    effect: "Disables LaTeX rendering globally; rendering-only, no direct agent loading effect."
  - name: "AGY_CLI_HIDE_ACCOUNT_INFO"
    effect: "Hides account information in the header; rendering-only."
  - name: "AGY_CLI_CMD_OUTPUT_PERCENTAGE"
    effect: "Controls command-output height in the TUI; may affect observability but not definition discovery."
  - name: "HOME"
    effect: "Determines the home-relative ~/.gemini/config/ and ~/.gemini/antigravity-cli/ roots on macOS/Linux."
  - name: "USERPROFILE"
    effect: "Determines the home-relative .gemini\\config\\ and .gemini\\antigravity-cli\\ roots on Windows."

changes: []
requires_claudine_update: true
reason: "Claudine does not currently list Antigravity as a provider in the compiled roster, but Antigravity has first-class subagents with provider-specific Markdown agent artifacts and lifecycle/runtime observability that should be represented before linking or resume/proxy support targets it."
---

# Antigravity CLI Subagents

## Overview

Antigravity calls the feature **Subagents**. The closest durable definition artifact is a Markdown `agent.md` file managed by the interactive `/agents` panel and discovered from Antigravity customization roots. Runtime subagents are separate conversation contexts used for background or parallel work, while background tasks are surfaced in `/tasks`.

Support is first-class, but the static schema is only partially public. The official site has a subagents documentation route, the product page advertises subagents, the local CLI has `/agents` UI strings and subagent keybindings, and the changelog records the transition from `agent.json` to `agent.md`. Current public and bundled docs do not expose a complete `agent.md` frontmatter contract, so Claudine should link conservatively.

## Locations

| OS | Scope | Path | Status |
| --- | --- | --- | --- |
| macOS | User | `~/.gemini/config/agents/` | Documented/inferred from the `/agents` changelog and shared customization root. Not present locally. |
| Linux | User | `~/.gemini/config/agents/` | Same shared root as macOS. |
| Windows | User | `%USERPROFILE%\.gemini\config\agents\` | Home-relative Windows equivalent. |
| macOS/Linux | Repo | `.agents/agents/` | Inferred subdirectory below the documented `.agents/` customization root. |
| Windows | Repo | `.agents\agents\` | Windows path equivalent. |
| macOS/Linux | Extension | `.agents/plugins/<plugin_name>/agents/` | Inferred from plugin discovery for agents and the plugin directory pattern. |
| Windows | Extension | `.agents\plugins\<plugin_name>\agents\` | Windows path equivalent. |

The installed CLI was run with `HOME=/Users/ken/.claudine`. Local inspection found no `~/.antigravity` directory. It did find `~/.gemini/config/`, `~/.gemini/antigravity-cli/`, logs, keybindings, and bundled Antigravity guide/customization skills, but no local user subagent definitions.

The bundled customization guide documents `.agents/`, `.agent/`, `_agents/`, and `_agent/` as project customization roots and `~/.gemini/config/` as the global configuration root. It also says Antigravity walks from the current working directory to the repository root to discover project customizations.

## Definition Format

The current durable artifact is Markdown:

```markdown
---
name: reviewer
description: Review implementation changes for correctness, regressions, and missing tests.
---

Review code like an owner. Prioritize correctness, security, behavior regressions,
and missing test coverage. Return findings with file references and concrete
reproduction or verification notes where possible.
```

This example is a conservative Claudine-compatible shape, not a verified full Antigravity schema. Verified facts are narrower:

- Antigravity CLI 1.0.16 changed dynamically defined subagents from JSON to Markdown.
- Antigravity CLI 1.1.0 fixed the `/agents` header to display `agent.md` instead of `agent.json`.
- Antigravity's customization system uses Markdown files with YAML frontmatter for skills, and the binary includes `agent.md` plus `ListAgents` strings.

Until a real `agent.md` file or official schema is available, Claudine should treat `name`, `description`, and Markdown body instructions as the portable subset. Do not assume `model`, `tools`, `permissions`, `color`, or `max_turns` keys are accepted by Antigravity `agent.md`.

## Runtime Behavior

Subagents are invoked from an interactive session, primarily through the `/agents` panel, natural-language delegation, and background task flows. The CLI product page describes subagents as concurrent agent sessions for delegated background tasks. Local keybindings include `subagent.approve_fast` (`ctrl+k`) and `subagent.jump_to_waiting` (`alt+j`), confirming an interactive subagent approval/attention workflow.

The installed binary describes the default subagent as inheriting the parent agent's full configuration, including tools, system prompt, and model, while running in a separate conversation context. Completed subagents are reported back to the parent. The parent can view a subagent conversation log and send follow-up messages by `conversation_id`.

Multiple subagents/background tasks can exist at once. Changelog 1.0.15 added a status indicator for active subagents and background tasks, and 1.0.16 improved `/tasks` details as background logs stream. Changelog 1.0.6 says subagent conversations are skipped from `/resume`, so child sessions should not be treated as ordinary resumable conversations.

There is no documented non-interactive selector such as `agy --agent reviewer`. `agy --print` can run a one-shot session, and `--model`, `--mode`, `--sandbox`, `--dangerously-skip-permissions`, workspace/project flags, and plugin enablement affect the parent session that would spawn subagents.

## Observability

Antigravity exposes subagent lifecycle primarily through interactive surfaces and logs:

- `/agents` shows/manages subagents.
- `/tasks` shows background task status and details.
- The status indicator shows active subagents and background tasks.
- Keybindings can jump to waiting subagents and approve subagent work.
- CLI logs live under `~/.gemini/antigravity-cli/log/`, with `cli.log` symlinked to the active log.
- Conversation summaries are stored in `~/.gemini/antigravity-cli/conversation_summaries.db`.

The bundled hook docs do not define dedicated `SubagentStart` or `SubagentStop` hook events. They do define `PreInvocation`, `PostInvocation`, and `Stop`; the `Stop` payload includes `fullyIdle`, which is true only when all background tasks are done. That is the only documented hook field found in this run that exposes subagent/background-task lifecycle state.

## Portability

Antigravity subagent definitions are not portable as-is. The Markdown body can usually be reused, but the definition path, filename, frontmatter interpretation, runtime tool names, permission behavior, plugin containment, and conversation IDs are provider-specific.

For cross-provider linking, Claudine should:

- Preserve the instruction body.
- Preserve `name` and `description` when present.
- Rewrite or drop Antigravity-only metadata.
- Avoid mapping Antigravity permission behavior to another provider unless the target provider has an explicit equivalent.
- Treat plugin-packaged agents as extension-scoped resources, not standalone user files.

## Claudine Linking Notes

The agent linker should add Antigravity as a provider-specific agent target only after Claudine has an Antigravity provider record. Linking can discover likely definitions under:

- `~/.gemini/config/agents/`
- `.agents/agents/`
- `.agent/agents/`
- `_agents/agents/`
- `_agent/agents/`
- plugin-contained `agents/` folders

For lifecycle `proxy`, Claudine cannot rely on a documented CLI flag to target a named Antigravity subagent. Proxy behavior would need to inject an instruction into an ordinary `agy` session asking it to delegate, or drive the interactive `/agents` UI, which is less reliable for automation.

For lifecycle `resume`, Claudine should not assume subagents are resumable. Antigravity specifically skipped subagent conversations from `/resume`; the reliable handle is a runtime `conversation_id` while the parent session is alive, plus persisted logs/transcripts for observation.

Code or generated metadata changes are needed before first-class support: add Antigravity to provider metadata, define its agent definition locations as provider-specific Markdown resources, classify `agent.md` as non-portable without rewrite, and keep lifecycle support conservative until a stable stream or hook contract exposes start/stop IDs.

## Sources

- [Antigravity CLI product page](https://antigravity.google/product/antigravity-cli)
- [Antigravity CLI overview](https://antigravity.google/docs/cli/overview)
- [Antigravity CLI subagents docs route](https://antigravity.google/docs/cli/subagents)
- [google-antigravity/antigravity-cli README](https://github.com/google-antigravity/antigravity-cli)
- [Antigravity CLI changelog](https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md)
- Local `agy --help` and `agy plugin --help` output from `/Users/ken/.local/bin/agy` version installed on 2026-07-08.
- Local bundled guide: `/Users/ken/.claudine/.gemini/antigravity-cli/builtin/skills/agy-customizations/SKILL.md`
- Local bundled hooks guide: `/Users/ken/.claudine/.gemini/antigravity-cli/builtin/skills/agy-customizations/docs/hooks.md`
- Local bundled plugins guide: `/Users/ken/.claudine/.gemini/antigravity-cli/builtin/skills/agy-customizations/docs/plugins.md`
- Local bundled CLI guide: `/Users/ken/.claudine/.gemini/antigravity-cli/builtin/skills/antigravity_guide/references/cli.md`
