---
$schema: ./_schema.yaml
created: 2026-07-08
last_updated: 2026-07-08
agent: codex
model: default
docs: https://antigravity.google/docs/cli/reference
system_prompt_docs: https://antigravity.google/docs/cli/best-practices
append_support: file
replace_support: agent_spec
cli_params:
  - flag: "--add-dir <DIR>"
    mode: modify
    value_shape: "repeatable directory path"
    description: "Adds a directory to the workspace for the current session. Additional workspace roots can cause more directory rules and customizations to be discovered."
    example: "agy --add-dir ../shared -p \"Review this workspace\""
    notes: "Prompt-adjacent only. The installed `agy 1.1.0 --help` lists the flag, but does not document rule-discovery details. Use only when the wrapper intentionally wants Antigravity to see another tree."
  - flag: "--mode <accept-edits|plan>"
    mode: modify
    value_shape: "execution mode string"
    description: "Sets the agent execution mode for the current session."
    example: "agy --mode plan -p \"Design the migration\""
    notes: "Affects behavioral mode and likely prompt preamble/tool policy, but it is not a user prompt injection or replacement mechanism. Changelog 1.1.0 says mode cycling is default -> accept-edits -> plan and adds request-review as the default behavior."
  - flag: "--sandbox"
    mode: modify
    value_shape: "boolean switch"
    description: "Runs with terminal restrictions enabled."
    example: "agy --sandbox -p \"Inspect this repository\""
    notes: "Prompt-adjacent only; sandbox/tool policy can alter tool instructions. No evidence that it appends or replaces natural-language system prompt text."
  - flag: "--dangerously-skip-permissions"
    mode: modify
    value_shape: "boolean switch"
    description: "Auto-approves tool permission requests without prompting."
    example: "agy --dangerously-skip-permissions -p \"Run the local checks\""
    notes: "Prompt-adjacent only. It changes permission behavior and may affect tool-use instructions, but is not a system-prompt delivery surface."
  - flag: "--model <MODEL>"
    mode: other
    value_shape: "model id or alias"
    description: "Selects the model for the current CLI session."
    example: "agy --model gemini-3-pro -p \"Summarize the diff\""
    notes: "Not a prompt-control switch. Model choice can affect backend prompt templates or available capabilities, but no prompt text semantics are documented."
  - flag: "--project <PROJECT_ID>"
    mode: other
    value_shape: "project id"
    description: "Sets the Antigravity project for the current CLI session."
    example: "agy --project default-cli-project -p \"List project assumptions\""
    notes: "Can affect workspace/project config selection. It does not directly append or replace system prompt text."
  - flag: "--new-project"
    mode: other
    value_shape: "boolean switch"
    description: "Creates a new project for the current CLI session."
    example: "agy --new-project -p \"Initialize project context\""
    notes: "Can change project-scoped configuration, but is not a prompt override."
  - flag: "--print / --prompt / -p <TEXT>"
    mode: other
    value_shape: "inline user prompt"
    description: "Runs a single prompt non-interactively and prints the response."
    example: "agy -p \"Summarize README.md\""
    notes: "This is the likely wrapper execution mode, but installed `agy 1.1.0` exposes no accompanying system-prompt append, replace, inspect, or export flag."
  - flag: "--prompt-interactive / -i <TEXT>"
    mode: other
    value_shape: "inline initial user prompt"
    description: "Starts an interactive session with an initial prompt."
    example: "agy -i \"Explain the architecture\""
    notes: "Initial user prompt only. No system-prompt semantics."
  - flag: "--continue / -c"
    mode: other
    value_shape: "boolean switch"
    description: "Continues the most recent conversation."
    example: "agy -c -p \"Continue with the next file\""
    notes: "Resume behavior can reuse conversation state. Re-supply any wrapper-created temporary rule files in the launched workspace if deterministic append behavior is required."
  - flag: "--conversation <ID>"
    mode: other
    value_shape: "conversation id"
    description: "Resumes a previous conversation by id."
    example: "agy --conversation 754ece5a-b744-473e-8c81-0c0e220fd55a -p \"Continue\""
    notes: "Prompt-control state on resumed conversations is not documented; do not assume a previous temporary append remains effective."
  - flag: "--log-file <PATH>"
    mode: inspect
    value_shape: "file path"
    description: "Overrides the CLI log file path."
    example: "agy --log-file /tmp/agy.log -p \"noop\""
    notes: "Inspection-adjacent only. Logs are useful for debugging startup/config discovery, but no supported effective-prompt export was found."
  - flag: "agy changelog"
    mode: inspect
    value_shape: "subcommand"
    description: "Shows local changelog and release notes."
    example: "agy changelog"
    notes: "Useful for recent prompt-adjacent changes. Version 1.0.16 changed dynamically defined subagents from JSON to Markdown; 1.1.0 fixed `/agents` to display the active global subagent directory."
  - flag: "agy plugin <install|import|enable|disable|list|validate>"
    mode: modify
    value_shape: "subcommand and plugin name/path"
    description: "Manages plugins, which may bundle rules, skills, hooks, and MCP server configuration."
    example: "agy plugin enable team-developer-kit"
    notes: "Plugins can indirectly append rules into the effective prompt. Use of plugin commands mutates user config and is not suitable for transient wrapper prompt delivery."
  - flag: "/agents"
    mode: modify
    value_shape: "interactive slash command"
    description: "Opens the subagent panel for creating, viewing, and running subagents."
    example: "agy # then type /agents"
    notes: "Interactive only. Changelog 1.1.0 says the panel now displays global subagents in `~/.gemini/config/` and uses `agent.md`, not `agent.json`, for creation. Not usable as a non-interactive wrapper delivery surface."
  - flag: "none: --append-system-prompt / --replace-system-prompt"
    mode: unknown
    value_shape: "not present"
    description: "Installed `agy 1.1.0 --help` exposes no native append, replace, inspect, or export system-prompt flags."
    example: "agy --help"
    notes: "Do not carry over Gemini CLI's `GEMINI_SYSTEM_MD` behavior to Antigravity without new evidence."
config_sources:
  - os: macos
    scope: user
    path: "~/.gemini/GEMINI.md"
    mode: append
    format: markdown
    notes: "Global home-level rule file observed locally under `/Users/ken/.gemini/GEMINI.md`. Antigravity customization docs say directory-based `GEMINI.md` and `AGENTS.md` files are Markdown rules loaded hierarchically."
  - os: linux
    scope: user
    path: "~/.gemini/GEMINI.md"
    mode: append
    format: markdown
    notes: "Same home-relative path shape as macOS. The installed docs use `~/.gemini/config/` for global configuration and `GEMINI.md` / `AGENTS.md` for directory rules."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\GEMINI.md"
    mode: append
    format: markdown
    notes: "Windows spelling inferred from Antigravity's home-relative `.gemini` convention; not locally tested on Windows."
  - os: macos
    scope: repo
    path: "GEMINI.md"
    mode: append
    format: markdown
    notes: "Directory-based rule file. Antigravity walks from the current working directory or relevant file directory up to the repository root and loads matching rules."
  - os: linux
    scope: repo
    path: "GEMINI.md"
    mode: append
    format: markdown
    notes: "Same repository-relative filename as macOS."
  - os: windows
    scope: repo
    path: "GEMINI.md"
    mode: append
    format: markdown
    notes: "Same repository-relative filename as macOS/Linux."
  - os: macos
    scope: repo
    path: "AGENTS.md"
    mode: append
    format: markdown
    notes: "Directory-based rule file. Antigravity docs and changelog say `AGENTS.md` is supported in addition to `GEMINI.md`."
  - os: linux
    scope: repo
    path: "AGENTS.md"
    mode: append
    format: markdown
    notes: "Same repository-relative filename as macOS."
  - os: windows
    scope: repo
    path: "AGENTS.md"
    mode: append
    format: markdown
    notes: "Same repository-relative filename as macOS/Linux."
  - os: macos
    scope: repo
    path: ".agents/rules/*.md"
    mode: append
    format: markdown
    notes: "Project customization rule files. The built-in customization guide lists `.agents/`, `.agent/`, `_agents/`, and `_agent/` as workspace customization roots; `rules/*.md` rules may be always-on or model-decision triggered."
  - os: linux
    scope: repo
    path: ".agents/rules/*.md"
    mode: append
    format: markdown
    notes: "Same repository-relative path shape as macOS."
  - os: windows
    scope: repo
    path: ".agents\\rules\\*.md"
    mode: append
    format: markdown
    notes: "Windows path spelling of the repository customization rules directory."
  - os: macos
    scope: user
    path: "~/.gemini/config/"
    mode: modify
    format: other
    notes: "Global customization root for skills, plugins, hooks, MCP, and subagents. Local active HOME also had `/Users/ken/.claudine/.gemini/config/`; real home had `/Users/ken/.gemini/config/`."
  - os: linux
    scope: user
    path: "~/.gemini/config/"
    mode: modify
    format: other
    notes: "Same home-relative global customization root as macOS."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\config\\"
    mode: modify
    format: other
    notes: "Windows spelling inferred from the home-relative `.gemini` convention and prior Antigravity research."
  - os: macos
    scope: agent
    path: "~/.gemini/config/agents/*.md"
    mode: replace
    format: markdown
    notes: "Global subagent definitions. Local examples also existed in legacy `/Users/ken/.gemini/agents/*.md`; changelog 1.1.0 says `/agents` now points users at `~/.gemini/config/` and `agent.md`."
  - os: linux
    scope: agent
    path: "~/.gemini/config/agents/*.md"
    mode: replace
    format: markdown
    notes: "Same home-relative agent definition path shape as macOS; not locally tested on Linux."
  - os: windows
    scope: agent
    path: "%USERPROFILE%\\.gemini\\config\\agents\\*.md"
    mode: replace
    format: markdown
    notes: "Windows spelling inferred from the global config root; not locally tested."
  - os: macos
    scope: extension
    path: ".agents/plugins/<name>/rules/*.md"
    mode: append
    format: markdown
    notes: "Plugin rules are loaded when the plugin is enabled. This is a persistent customization surface, not a transient wrapper surface."
  - os: linux
    scope: extension
    path: ".agents/plugins/<name>/rules/*.md"
    mode: append
    format: markdown
    notes: "Same repository-relative plugin rule path shape as macOS."
  - os: windows
    scope: extension
    path: ".agents\\plugins\\<name>\\rules\\*.md"
    mode: append
    format: markdown
    notes: "Windows path spelling of plugin rules."
env_vars:
  - name: "AGY_CLI_CMD_OUTPUT_PERCENTAGE"
    effect: "Controls maximum command-output height in the TUI; prompt-adjacent display setting only."
    mode: other
  - name: "AGY_CLI_DISABLE_LATEX"
    effect: "Disables LaTeX rendering globally; no system-prompt effect."
    mode: disable
  - name: "AGY_CLI_HIDE_ACCOUNT_INFO"
    effect: "Hides email and plan tier from the header; no system-prompt effect."
    mode: other
  - name: "GEMINI_SYSTEM_MD"
    effect: "No verified Antigravity CLI effect. This is a Gemini CLI mechanism and must not be assumed for `agy`."
    mode: unknown
prompt_layers:
  - source: "built-in Antigravity base prompt"
    mode: unknown
    scope: ["builtin"]
    order_notes: "Lowest provider-owned layer, but exact ordering is not documented."
    notes: "The binary contains prompt-template strings and confidentiality text, but no supported export or override path was found."
  - source: "execution mode and tool policy"
    mode: modify
    scope: ["builtin", "session"]
    order_notes: "Applied as part of session construction before or alongside tools."
    notes: "`--mode`, `--sandbox`, and permission settings change behavior and likely prompt/tool instructions, but not user-authored system prompt text."
  - source: "global rules"
    mode: append
    scope: ["user"]
    order_notes: "Loaded through customization discovery. Relative order against repo rules is not fully documented."
    notes: "`~/.gemini/GEMINI.md` was present locally; global configuration root is `~/.gemini/config/`."
  - source: "workspace and directory rules"
    mode: append
    scope: ["repo"]
    order_notes: "Hierarchical discovery walks from the current working directory or relevant file directory up to the repository root."
    notes: "`GEMINI.md`, `AGENTS.md`, and `.agents/rules/*.md` are Markdown rule sources. Standalone `GEMINI.md` and `AGENTS.md` do not support frontmatter."
  - source: "plugins"
    mode: modify
    scope: ["extension"]
    order_notes: "Plugin rules merge into the active rule set when the plugin is enabled."
    notes: "Plugins can package rules, skills, hooks, and MCP configs. They are persistent and should not be used for transient wrapper prompt delivery unless isolated under a shadow HOME."
  - source: "skills"
    mode: modify
    scope: ["extension", "user", "repo"]
    order_notes: "Names and descriptions are injected first; full skill content is loaded only after activation."
    notes: "Progressive disclosure limits prompt load. Skills are not the right surface for guaranteed wrapper system-prompt append."
  - source: "subagent definition"
    mode: replace
    scope: ["agent", "subagent"]
    order_notes: "A subagent has its own Markdown instruction body distinct from the parent session."
    notes: "Local examples use Markdown frontmatter (`name`, `description`, optional `model`) plus body as the role prompt. The binary says a subagent can be defined with name, description, system prompt, and tool groups."
agent_prompting:
  supported: true
  definition_surface: "Markdown agent files discovered from Antigravity agent configuration roots, currently `~/.gemini/config/agents/*.md` for global agents; legacy local examples also existed in `~/.gemini/agents/*.md`."
  inheritance: "Undocumented for user-authored agent files. Binary strings mention an inherited subagent that keeps the parent agent's full configuration including tools, system prompt, and model, but the public file-level inheritance contract was not found."
  isolation: "Subagents run as separate conversations/background tasks and return results to the parent. Changelog entries describe active subagents, background tasks, and skipped subagent conversations in `/resume`."
  limitations: "No non-interactive CLI flag was found to launch the top-level session as an arbitrary agent file. `/agents` is interactive. Using agent files for wrapper replacement would require persistent config or a shadow HOME."
claudine_delivery:
  append_strategy: "file_flag"
  replace_strategy: "unsupported"
  temp_file_required: true
  argv_limit: "Avoid inline prompt text; `agy` has no native inline system-prompt flag, and rule files avoid shell/argv limits."
  notes: "For append, use a temporary workspace rule file such as `<shadow-workspace>/AGENTS.md` or `.agents/rules/claudine.md` only in a wrapper-controlled working tree. `--add-dir` can include a temporary directory, but rule discovery from added dirs is not fully proven, so the safer path is launching from a shadow workspace that contains the prompt file and points at the real repo only when acceptable. For replace, no supported non-mutating top-level base-prompt replacement was found; do not mutate `~/.gemini/config/` or rely on Gemini CLI's `GEMINI_SYSTEM_MD`."
format_recommendations:
  append_format: markdown
  replace_format: markdown
  rationale: "Antigravity's documented prompt-adjacent customization surfaces are Markdown-centric: `GEMINI.md`, `AGENTS.md`, `.agents/rules/*.md`, skills, and agent `.md` files. XML-wrapped Markdown may be useful for wrapper delimiters inside a rule body, but it is not provider-documented. YAML/JSON are for config manifests, not natural-language prompt bodies."
recent_changes:
  - date: "2026-05-19"
    version: "1.0.0"
    change: "Initial Antigravity CLI release and transition announcement from Gemini CLI."
    impact: "Antigravity CLI retained Agent Skills, Hooks, Subagents, and Extensions as plugins, but did not ship Gemini CLI's documented system prompt override/export behavior."
  - date: "2026-05-20"
    version: "unknown"
    change: "GitHub issue #50 requested a way to export and modify the system prompt."
    impact: "As of 2026-07-08 the issue remained open in the public tracker, supporting the conclusion that export/modify is not yet a documented feature."
  - date: "2026-06-09"
    version: "1.0.16"
    change: "Dynamically defined subagents transitioned from JSON to Markdown."
    impact: "Subagent prompt replacement should target Markdown agent files, not `agent.json`."
  - date: "2026-07-08"
    version: "1.1.0"
    change: "The `/agents` panel was fixed to display `agent.md` and the active global configuration directory `~/.gemini/config/`."
    impact: "Global subagent definitions should be created under the shared config root, but this still mutates user config unless Claudine uses a shadow HOME."
quirks:
  - "The public docs site is a JavaScript app; direct page fetches return the SPA shell, so CLI help, changelog, embedded built-in docs, and local files are more reliable for exact behavior."
  - "The installed binary contains `SystemPrompt` and prompt-template strings, but no supported system-prompt export command was found."
  - "Local shell HOME was `/Users/ken/.claudine`, so `~/.gemini` in command output referred to `/Users/ken/.claudine/.gemini`; real user config also existed under `/Users/ken/.gemini`."
  - "`/Users/ken/.antigravity` existed, but prompt/config examples relevant to the CLI were under `.gemini`, not `.antigravity`."
  - "Antigravity uses both historical and current locations: local legacy subagent examples existed in `/Users/ken/.gemini/agents/*.md`, while changelog 1.1.0 points `/agents` at `~/.gemini/config/`."
  - "Standalone `GEMINI.md` and `AGENTS.md` rule files are Markdown without frontmatter; rule files under customization roots may be more structured, but the installed `rules.md` excerpt only fully documents standalone files."
  - "Community system-prompt dumps exist, but Antigravity's own prompt declares confidentiality, so they should not be used as implementation ground truth."
gaps:
  - "No official Antigravity page specifically documents top-level system prompt append, replacement, inspection, or export."
  - "Could not verify a native equivalent of Gemini CLI's `GEMINI_SYSTEM_MD` in Antigravity CLI."
  - "Could not verify whether `--add-dir` reliably triggers rule discovery for a prompt-only temporary directory in `--print` mode."
  - "Could not verify Windows and Linux path behavior locally; OS-specific paths are inferred from documented home-relative conventions and prior CLI research."
  - "Could not verify exact effective prompt ordering among built-in prompt, global rules, repo rules, plugin rules, skills, and subagent prompts."
  - "Could not inspect the full effective built-in prompt through a supported provider command."
changes: []
requires_claudine_update: true
reason: "Antigravity should be represented as append-only through temporary Markdown rule files for now, with top-level replace marked unsupported despite subagent-specific Markdown prompts."
---

# Antigravity System Prompt Research

## Overview

Antigravity CLI (`agy`) does not currently expose a verified top-level `--append-system-prompt`, `--replace-system-prompt`, or prompt export flag. Installed `agy 1.1.0 --help` lists session, model, project, prompt, permission, sandbox, logging, update, model, and plugin controls, but no direct system prompt controls.

The supported prompt-adjacent customization model is file-based. Antigravity discovers Markdown rules from `GEMINI.md`, `AGENTS.md`, and `.agents/rules/*.md`; discovers skills, plugins, hooks, and MCP configuration from customization roots; and supports Markdown subagent definitions whose body acts as the subagent's distinct instruction prompt.

For Claudine, this means append can be approximated through a temporary Markdown rule file in a wrapper-controlled workspace. Replacement of the top-level built-in prompt should be treated as unsupported until Antigravity provides a native flag, config key, environment variable, or documented agent-file launch mode. Subagent prompt replacement is real, but it does not replace the orchestrator's built-in prompt.

## CLI Parameters

| Parameter | Mode | Prompt effect | Wrapper notes |
| --- | --- | --- | --- |
| `--add-dir <DIR>` | modify | Adds a workspace directory, which may expand discovered rules/customizations. | Potential append helper, but not proven enough to rely on for prompt-only temp dirs. |
| `--mode <accept-edits\|plan>` | modify | Changes execution mode and likely prompt/tool policy. | Not a user instruction append/replace mechanism. |
| `--sandbox` | modify | Enables terminal restrictions and likely sandbox-aware tool instructions. | Prompt-adjacent only. |
| `--dangerously-skip-permissions` | modify | Changes approval behavior. | Not prompt delivery. |
| `--model <MODEL>` | other | Selects the model. | May change backend template selection, but no documented prompt text semantics. |
| `--project <PROJECT_ID>` / `--new-project` | other | Selects or creates project context. | Can affect project-scoped config discovery. |
| `--print`, `--prompt`, `-p <TEXT>` | other | Sends the user prompt non-interactively. | Best execution mode for wrappers, but has no system-prompt companion flag. |
| `--prompt-interactive`, `-i <TEXT>` | other | Starts interactive mode with an initial user prompt. | User prompt only. |
| `--continue`, `-c`, `--conversation <ID>` | other | Resumes previous session state. | Re-apply any wrapper-managed temporary rules on resumed runs. |
| `--log-file <PATH>` | inspect | Writes logs to a known path. | Useful for debugging config discovery; not prompt export. |
| `agy changelog` | inspect | Shows release notes. | Confirms Markdown subagent changes in `1.0.16` and `agent.md` path fixes in `1.1.0`. |
| `agy plugin ...` | modify | Plugins may contribute rules. | Persistent user config mutation; avoid for transient wrapper prompts. |
| `/agents` | modify | Interactive subagent creation/management. | Not a non-interactive wrapper surface. |

No installed help output or official source found a native `--append-system-prompt`, `--replace-system-prompt`, `--system-prompt`, `--system-prompt-file`, or export flag for Antigravity CLI.

## Configuration and Discovery

Antigravity's customization system is the main way users affect the effective instruction stack.

| Source | Scope | Format | Behavior |
| --- | --- | --- | --- |
| `GEMINI.md` | user/repo/directory | Markdown | Directory rule loaded from the file's directory up to the repository root. |
| `AGENTS.md` | repo/directory | Markdown | Same role as `GEMINI.md`; added in Antigravity before the CLI transition and retained by `agy`. |
| `.agents/rules/*.md` | repo | Markdown | Project customization rules. |
| `.agents/skills/<name>/SKILL.md` | repo | Markdown with frontmatter | Skill names/descriptions are injected; full skill content is loaded on demand. |
| `.agents/plugins/<name>/plugin.json` plus `rules/` | repo/extension | JSON plus Markdown | Plugin rules merge into the active rule set when enabled. |
| `~/.gemini/config/` | user | mixed | Global customization root for shared config, including current global subagent location. |
| `~/.gemini/config/agents/*.md` | agent/subagent | Markdown with frontmatter | Distinct subagent prompt definitions. |
| `~/.gemini/antigravity-cli/settings.json` | user | JSON | CLI-specific settings; no prompt override key was verified. |

Local inspection found `/Users/ken/.antigravity`, but the CLI-relevant local state and customizations were under `/Users/ken/.gemini` and the session HOME shadow `/Users/ken/.claudine/.gemini`. The real home had `GEMINI.md`, `agents/`, `skills/`, `config/`, `antigravity/`, `antigravity-ide/`, and `antigravity-cli/`. The active session HOME had `~/.gemini/config/config.json`, project JSON files, and `~/.gemini/antigravity-cli` logs/cache/state.

## Prompt Layers and Precedence

Antigravity documents customization priority, but not exact final system prompt concatenation order. The best supported model is:

```mermaid
flowchart TD
    A[Built-in Antigravity base prompt] --> B[Execution mode, sandbox, tool policy]
    B --> C[Global rules and customization config]
    C --> D[Workspace and directory rules]
    D --> E[Plugin-provided rules]
    E --> F[Skill names and descriptions]
    F --> G[Activated skill bodies]
    G --> H[User prompt]
    D --> I[Subagent definition]
    I --> J[Separate subagent conversation]
```

The built-in customization guide gives priority from highest to lowest as workspace project, declared configurations, global discovery, built-in customizations, and global declared configurations. It also says rule files are deduplicated by resolved path and injected at most once per conversation turn.

For wrappers, the practical issue is not only order but mutability. Rules and subagents are discoverable files. Writing them into the real repository or real user config permanently mutates user state. A wrapper must use a temporary workspace, shadow HOME, or another reversible overlay if it wants append behavior.

## Agents and Subagents

Antigravity supports subagents with distinct prompts. Local Markdown examples in `/Users/ken/.gemini/agents/*.md` used frontmatter such as `name`, `description`, and optional `model`, followed by a Markdown body containing the subagent instructions. Installed binary strings also describe creating a subagent with a name, description, system prompt, and tool groups.

Recent changelog entries matter:

| Version | Change | Prompt impact |
| --- | --- | --- |
| `1.0.16` | Dynamically defined subagents moved from JSON to Markdown. | Do not target `agent.json` for new subagent prompt definitions. |
| `1.1.0` | `/agents` now displays `agent.md` and the active global config directory `~/.gemini/config/`. | Global subagents should be considered config-root Markdown assets. |

Subagents are not a clean top-level replacement strategy for Claudine today. No non-interactive `agy --agent-file` or equivalent launch flag was found. Using a subagent definition would require writing persistent config or running an interactive flow unless Claudine can isolate Antigravity under a shadow HOME and automate a supported launch path.

## Format Recommendations

Use Markdown for both append and any future replacement candidate.

| Mode | Recommended format | Rationale |
| --- | --- | --- |
| Append | Markdown | Native rule surfaces are Markdown: `GEMINI.md`, `AGENTS.md`, and rules files. |
| Replace | Markdown | Subagent prompt definitions are Markdown; any future prompt file support is likely to follow the same convention. |

XML-wrapped Markdown is not provider-documented. It can still be useful inside Claudine-authored rule bodies to delimit wrapper instructions, but it adds tokens and should not be required. YAML and JSON are for manifests/config, not prompt prose.

## Recent Changes

| Date | Version | Change | Impact |
| --- | --- | --- | --- |
| 2026-05-19 | `1.0.0` | Antigravity CLI launched during the Gemini CLI transition. | Critical Gemini CLI features such as skills, hooks, subagents, and extensions/plugins were retained, but direct system prompt override/export parity was not documented. |
| 2026-05-20 | unknown | Public issue #50 requested system prompt export and modification. | Supports the conclusion that this is not a documented feature today. |
| 2026-06-09 | `1.0.16` | Dynamic subagents changed from JSON to Markdown. | Agent prompt research should target Markdown definitions. |
| 2026-07-08 | `1.1.0` | `/agents` path display fixed to `~/.gemini/config/` and `agent.md`. | Confirms the current global agent definition surface. |

## Quirks and Workarounds

Antigravity's public docs site is a JavaScript application. Direct `curl` of docs pages returns the SPA shell, so local `agy --help`, `agy changelog`, installed built-in skills, and the public GitHub issue tracker were more useful for exact behavior than rendered docs fetches.

The installed binary includes system prompt template strings and confidentiality text, but this is not a supported export interface. Do not build Claudine behavior around reverse-engineered prompt dumps.

`~/.antigravity` existed locally, but the CLI used `.gemini` state/config roots. The provider roster currently mentioning `~/.antigravity` is not enough for prompt delivery decisions.

There is historical path churn. Local legacy agents existed in `/Users/ken/.gemini/agents/*.md`, while the current changelog points creation to `~/.gemini/config/`. Claudine should avoid writing to either real path for transient delivery.

## Claudine Delivery Notes

Recommended append strategy: write a temporary Markdown rule file in a wrapper-controlled workspace and launch `agy` from that workspace. If the real repository must also be visible, test `--add-dir <real-repo>` carefully; the reverse approach, launching from the real repo and adding a prompt-only temp dir, is not yet proven to trigger rule discovery.

Recommended replace strategy: unsupported for the top-level orchestrator. Subagent Markdown files can define distinct prompts, but no supported non-interactive entry point was found to run an arbitrary subagent as the main session.

Avoid mutating:

- `/Users/ken/.gemini/config/`
- `/Users/ken/.gemini/agents/`
- `/Users/ken/.gemini/antigravity-cli/settings.json`
- repository `GEMINI.md` or `AGENTS.md`

If Antigravity later supports a configurable HOME or config root, Claudine can use a shadow HOME with `~/.gemini/config/agents/` and temporary rule files. Until then, wrapper-level replacement should be reported as unsupported rather than approximated with persistent user config.

## Sources

- [Antigravity CLI reference docs](https://antigravity.google/docs/cli/reference)
- [Antigravity CLI best practices docs](https://antigravity.google/docs/cli/best-practices)
- [Antigravity CLI product page](https://antigravity.google/product/antigravity-cli)
- [Antigravity CLI README](https://raw.githubusercontent.com/google-antigravity/antigravity-cli/main/README.md)
- [Google Developers Blog: Transitioning Gemini CLI to Antigravity CLI](https://developers.googleblog.com/an-important-update-transitioning-gemini-cli-to-antigravity-cli/)
- [GitHub issue #50: Ability to export and modify system prompt](https://github.com/google-antigravity/antigravity-cli/issues/50)
- Local command output: `agy --help`, `agy --version`, `agy changelog`, and `agy help plugin` from installed `agy 1.1.0`
- Local file inspected: `/Users/ken/.claudine/.gemini/antigravity-cli/builtin/skills/agy-customizations/SKILL.md`
- Local file inspected: `/Users/ken/.claudine/.gemini/antigravity-cli/builtin/skills/agy-customizations/docs/rules.md`
- Local file inspected: `/Users/ken/.claudine/.gemini/antigravity-cli/builtin/skills/agy-customizations/docs/json_configs.md`
- Local file inspected: `/Users/ken/.claudine/.gemini/antigravity-cli/builtin/skills/agy-customizations/docs/plugins.md`
- Local file inspected: `/Users/ken/.claudine/.gemini/antigravity-cli/builtin/skills/antigravity_guide/references/cli.md`
