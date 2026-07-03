---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: opencode
model: k2p7
docs: https://developers.openai.com/codex/cli
system_prompt_docs: https://developers.openai.com/codex/guides/agents-md
append_support: config
replace_support: config
cli_params:
  - flag: --config / -c
    mode: append
    value_shape: key=value
    description: Override any config.toml value. Pass developer_instructions="..." to append inline instructions to the built-in system prompt.
    example: codex exec "Summarize instructions" -c developer_instructions="Always use TypeScript"
    notes: Applies only to the current invocation. Values are parsed as TOML when possible; multi-line strings need TOML quoting/escaping.
  - flag: --config / -c
    mode: replace
    value_shape: key=value
    description: Override any config.toml value. Pass model_instructions_file=path to replace the built-in instructions with the contents of a Markdown file.
    example: codex exec "Refactor auth" -c model_instructions_file=./custom-prompt.md
    notes: Applies only to the current invocation. The file becomes the instruction chain in place of the default AGENTS.md-based built-in instructions.
  - flag: --enable
    mode: modify
    value_shape: feature
    description: Force-enable a feature flag (translates to -c features.<name>=true). Useful for goals, multi_agent, memories, hooks, etc.
    example: codex --enable goals
    notes: Indirectly shapes the prompt by enabling features such as goals or multi-agent collaboration.
  - flag: --disable
    mode: modify
    value_shape: feature
    description: Force-disable a feature flag (translates to -c features.<name>=false).
    example: codex --disable memories
    notes: Indirectly shapes the prompt by disabling optional layers such as memories or hooks.
config_sources:
  - os: all
    scope: user
    path: ~/.codex/config.toml
    mode: modify
    format: toml
    notes: User-level durable config. Can set developer_instructions, model_instructions_file, model_reasoning_effort, personality, memories, etc.
  - os: all
    scope: repo
    path: .codex/config.toml
    mode: modify
    format: toml
    notes: Project-scoped config loaded only when the project is trusted. Cannot override provider, auth, profile, notify, or telemetry keys.
  - os: all
    scope: user
    path: ~/.codex/AGENTS.md
    mode: append
    format: markdown
    notes: Global project-instructions file. Loaded if AGENTS.override.md does not exist in the same directory.
  - os: all
    scope: user
    path: ~/.codex/AGENTS.override.md
    mode: append
    format: markdown
    notes: Temporary global override. Takes precedence over AGENTS.md in the same directory.
  - os: all
    scope: repo
    path: AGENTS.md
    mode: append
    format: markdown
    notes: Project-level instructions. Codex walks from the project root down to the current working directory and concatenates files.
  - os: all
    scope: repo
    path: AGENTS.override.md
    mode: append
    format: markdown
    notes: Per-directory override. In each directory, AGENTS.override.md wins over AGENTS.md.
  - os: all
    scope: repo
    path: project_doc_fallback_filenames
    mode: append
    format: markdown
    notes: User-configurable fallback filenames (e.g. TEAM_GUIDE.md, .agents.md) treated as instruction files during directory walk.
  - os: all
    scope: agent
    path: ~/.codex/agents/*.toml
    mode: replace
    format: toml
    notes: Personal custom agent definitions. Each file must define name, description, and developer_instructions.
  - os: all
    scope: agent
    path: .codex/agents/*.toml
    mode: replace
    format: toml
    notes: Project-scoped custom agent definitions. Loaded only when the project is trusted.
  - os: all
    scope: user
    path: ~/.codex/prompts/*.md
    mode: append
    format: markdown
    notes: Deprecated custom prompt files (user scope only). Skills are the recommended replacement.
  - os: all
    scope: user
    path: ~/.codex/skills/*/
    mode: append
    format: markdown
    notes: Skill metadata and SKILL.md instructions can be injected into the prompt when the agent selects the skill.
  - os: all
    scope: repo
    path: .codex/skills/*/
    mode: append
    format: markdown
    notes: Project-level skills; closest to the working directory wins on name collisions.
env_vars:
  - name: CODEX_HOME
    effect: Changes the root directory for config, auth, logs, sessions, skills, and AGENTS.md discovery.
    mode: modify
  - name: CODEX_MODEL
    effect: Overrides the model selection; equivalent to setting model in config.toml.
    mode: other
  - name: OPENAI_MODEL
    effect: Alternative environment variable for model selection.
    mode: other
  - name: CODEX_API_KEY
    effect: Provides an API key for a single non-interactive codex exec run. Does not affect prompt content.
    mode: other
  - name: CODEX_ACCESS_TOKEN
    effect: Provides a ChatGPT or Codex access token for trusted automation.
    mode: other
  - name: RUST_LOG
    effect: Controls diagnostic log verbosity; useful for verifying which instruction files loaded.
    mode: inspect
prompt_layers:
  - source: Built-in Codex system prompt
    mode: replace
    scope:
      - session
    order_notes: Base layer; replaced entirely by model_instructions_file.
    notes: Contains tool-use guidance, safety instructions, and coding conventions. OpenAI does not publish the full text.
  - source: model_instructions_file
    mode: replace
    scope:
      - session
    order_notes: Replaces the built-in instruction chain when configured.
    notes: Configurable via -c model_instructions_file=path or config.toml. The caller becomes responsible for any tool guidance the task still needs.
  - source: AGENTS.md hierarchy
    mode: append
    scope:
      - user
      - repo
    order_notes: Global AGENTS.md/AGENTS.override.md first, then project root down to current working directory.
    notes: Concatenated with blank lines; stops at project_doc_max_bytes (default 32 KiB). Files closer to CWD appear later and override earlier guidance.
  - source: developer_instructions
    mode: append
    scope:
      - session
    order_notes: Appended to the instruction chain for the current invocation.
    notes: Delivered via -c developer_instructions="..." or config.toml. Temporary when passed on the CLI.
  - source: Personality
    mode: modify
    scope:
      - session
    order_notes: Applied via the personality setting.
    notes: Requires features.personality = true. Shapes communication style (e.g. pragmatic, friendly, none).
  - source: Model reasoning effort
    mode: modify
    scope:
      - session
    order_notes: Applied via model_reasoning_effort.
    notes: minimal/low/medium/high/xhigh. Affects the reasoning depth portion of the prompt.
  - source: Memories
    mode: append
    scope:
      - session
    order_notes: Injected when memories feature is enabled.
    notes: Controlled by features.memories and memories.use_memories.
  - source: Skills metadata
    mode: append
    scope:
      - session
    order_notes: Skill name/description injected at startup; full SKILL.md loaded when the skill is selected.
    notes: Discovery paths include ~/.codex/skills, .codex/skills, and .agents/skills.
  - source: Custom agent developer_instructions
    mode: replace
    scope:
      - subagent
    order_notes: Replaces the parent session's built-in instructions for the spawned subagent.
    notes: Each custom agent TOML file defines its own developer_instructions. Parent settings are inherited when omitted.
agent_prompting:
  supported: true
  definition_surface: TOML files in ~/.codex/agents/ or .codex/agents/
  inheritance: Subagents inherit parent session model, reasoning effort, sandbox mode, MCP servers, and skills.config when those keys are omitted in the agent file.
  isolation: Each subagent runs in its own thread; only results/summaries return to the parent. Sandbox and approval controls are inherited but can be overridden per agent.
  limitations: Nesting depth is capped by agents.max_depth (default 1). Concurrent threads capped by agents.max_threads (default 6). Built-in agents default, worker, and explorer exist; custom agents with matching names take precedence.
claudine_delivery:
  append_strategy: config_key_inline
  replace_strategy: config_key_file
  temp_file_required: true
  argv_limit: No published argv limit; prefer writing large prompts to a file and passing the path via -c model_instructions_file for replacements.
  notes: Claudine already implements Codex delivery via -c developer_instructions for append and -c model_instructions_file for replace. The wrapper discovers system-prompt.md from the launch-CWD hierarchy, prepares it with Darkmatter composition, and passes the resolved content to Codex's universal config-override flag. This avoids mutating user config.toml or AGENTS.md files.
format_recommendations:
  append_format: markdown
  replace_format: xml_wrapped_markdown
  rationale: Appended instructions are concatenated with AGENTS.md files and the built-in prompt, so plain Markdown blends cleanly. Replacing the entire instruction chain removes Codex's built-in structure; XML tags help the model distinguish rules, constraints, context, and examples.
recent_changes:
  - date: "2026-06"
    version: unknown
    change: Custom agents moved to standalone TOML files under ~/.codex/agents/ and .codex/agents/, each requiring name, description, and developer_instructions.
    impact: Subagents can now carry distinct system prompts with their own models, reasoning, and sandbox settings.
  - date: "2026-05"
    version: unknown
    change: Multi-agent collaboration tools (spawn_agent, send_input, resume_agent, wait_agent, close_agent) enabled by default.
    impact: Subagents now run in parallel by default; max depth and thread limits are configurable.
  - date: "2026-04"
    version: unknown
    change: Personality selection controls stabilized and enabled by default.
    impact: Adds a persona layer to the system prompt without replacing tool guidance.
  - date: "2026-03"
    version: unknown
    change: model_reasoning_effort controls stabilized with levels minimal/low/medium/high/xhigh.
    impact: Adds a reasoning-depth layer to the system prompt.
quirks:
  - Codex has no dedicated --system-prompt or --append-system-prompt flag; prompt overrides are delivered through the universal -c config-override mechanism.
  - -c values are parsed as TOML when possible, so multi-line developer_instructions need careful quoting/escaping on the command line.
  - model_instructions_file replaces the built-in instruction chain, but it is unclear whether AGENTS.md files are still appended afterward.
  - The AGENTS.md instruction chain is capped at project_doc_max_bytes (default 32 KiB); large projects must split guidance across nested directories or raise the limit.
  - AGENTS.override.md in the same directory shadows AGENTS.md, which can surprise users who expect the base file to remain active.
  - Project-scoped .codex/config.toml is ignored for provider, auth, notify, profile, and telemetry keys.
  - Custom prompts under ~/.codex/prompts/*.md are deprecated in favor of skills and AGENTS.md.
  - Subagent nesting defaults to depth 1, so recursive delegation requires explicitly raising agents.max_depth.
  - There is no documented CLI command or environment variable that exports the full effective built-in system prompt as plain text.
gaps:
  - OpenAI does not publish the full default Codex system prompt, so exact section ordering and token counts cannot be verified.
  - No documented provider mechanism exports or inspects the effective built-in prompt; only indirect verification via logs or asking Codex is available.
  - It is unclear whether developer_instructions and model_instructions_file can be combined in one invocation.
  - It is unclear whether developer_instructions delivered via -c supports multi-line TOML string literals cleanly across all shells.
  - The exact precedence between model_instructions_file and AGENTS.md/developer_instructions is not fully specified in the public docs.
changes: []
requires_claudine_update: false
reason: Claudine's provider metadata already encodes Codex system-prompt delivery as ConfigKeyInline (-c developer_instructions) for append and ConfigKeyFile (-c model_instructions_file) for replace. No new wrapper mechanism is required.
---

## Overview

Codex CLI builds the effective prompt for every session from a layered instruction chain. Unlike Claude Code, Codex does not expose dedicated `--system-prompt` flags. Instead, it uses the universal `-c` config-override flag to inject `developer_instructions` (append) or point to a `model_instructions_file` (replace). Persistent project instructions are discovered automatically through an `AGENTS.md` hierarchy, while custom subagents carry their own `developer_instructions` in standalone TOML files.

## CLI Parameters

Codex exposes one general-purpose flag that covers prompt overrides, plus feature toggles that indirectly shape the prompt.

| Flag | Mode | Effect |
| :--- | :--- | :--- |
| `-c developer_instructions="..."` | Append | Injects inline instructions on top of the built-in prompt and AGENTS.md chain. |
| `-c model_instructions_file=path` | Replace | Uses the contents of a Markdown file as the instruction chain, replacing the built-in instructions. |
| `--enable <feature>` | Modify | Force-enables a feature flag such as `goals`, `multi_agent`, or `memories`. |
| `--disable <feature>` | Modify | Force-disables a feature flag such as `memories` or `hooks`. |

`-c` values are parsed as TOML when possible; otherwise the literal string is used. This means multi-line `developer_instructions` values must be properly quoted/escaped. Both append and replace are temporary per-invocation changes when passed on the CLI.

## Configuration and Discovery

Beyond CLI overrides, Codex discovers persistent instruction sources automatically.

### config.toml layers

User-level settings live in `~/.codex/config.toml`. Project-scoped overrides live in `.codex/config.toml`, but Codex only loads the project layer when the project is trusted, and it ignores provider, auth, notify, profile, and telemetry keys from that layer.

Relevant config keys:

| Key | Effect |
| :--- | :--- |
| `developer_instructions` | Inline additional instructions appended to the session. |
| `model_instructions_file` | Path to a Markdown file that replaces the built-in instructions. |
| `model_reasoning_effort` | Adjusts reasoning depth (`minimal` to `xhigh`). |
| `personality` | Sets a communication style/persona layer. |
| `features.memories` | Enables memory injection into the prompt. |
| `features.multi_agent` | Enables subagent tools. |
| `project_doc_max_bytes` | Caps the combined AGENTS.md chain (default 32 KiB). |
| `project_doc_fallback_filenames` | Additional filenames treated as instruction files. |

### AGENTS.md hierarchy

Codex reads `AGENTS.md` files before doing any work. Discovery follows this order:

1. Global: `~/.codex/AGENTS.override.md` if it exists; otherwise `~/.codex/AGENTS.md`.
2. Project scope: starting at the project root (usually the Git root), Codex walks down to the current working directory. In each directory it checks `AGENTS.override.md`, then `AGENTS.md`, then any fallback filenames.
3. Merge order: files are concatenated from root to current directory with blank lines. Files closer to the working directory appear later and therefore override earlier guidance.

Codex stops adding files once the combined size reaches `project_doc_max_bytes`.

### Custom agents

Custom agents are standalone TOML files under `~/.codex/agents/` or `.codex/agents/`. Each file must define:

- `name`
- `description`
- `developer_instructions`

Optional fields such as `model`, `model_reasoning_effort`, `sandbox_mode`, `mcp_servers`, and `skills.config` inherit from the parent session when omitted. If a custom agent name matches a built-in agent (`default`, `worker`, `explorer`), the custom definition takes precedence.

### Skills and deprecated custom prompts

Skills are discovered under `~/.codex/skills/`, `.codex/skills/`, and `.agents/skills/`. Skill `name` and `description` are injected at startup; the full `SKILL.md` is loaded when the agent selects the skill. The legacy `~/.codex/prompts/*.md` custom prompt files are deprecated.

## Prompt Layers and Precedence

The final instruction chain is assembled from the following layers, from most foundational to most specific.

```mermaid
graph TD
    A[Built-in Codex system prompt] --> B{model_instructions_file configured?}
    B -- yes --> C[Contents of model_instructions_file]
    B -- no --> D[AGENTS.md hierarchy]
    C --> E[developer_instructions]
    D --> E
    E --> F[Personality layer]
    F --> G[Reasoning effort layer]
    G --> H[Memories]
    H --> I[Skill metadata]
    I --> J[User prompt]
```

Notes on precedence:

- `model_instructions_file` replaces the built-in prompt and the AGENTS.md-based chain.
- `developer_instructions` appends to whatever instruction chain is active.
- `AGENTS.md` files are concatenated and capped at `project_doc_max_bytes`.
- Personality, reasoning effort, memories, and skills modify or append additional structure on top of the base chain.

## Agents and Subagents

Codex supports multi-agent workflows through built-in agents (`default`, `worker`, `explorer`) and user-defined custom agents. Subagents run in isolated threads and only their results return to the parent.

Key behaviors:

- Custom agents are defined as TOML files with their own `developer_instructions`.
- Subagents inherit the parent session's model, reasoning effort, sandbox mode, MCP servers, and skills when those keys are omitted.
- Sandbox and approval controls can be overridden per agent.
- `agents.max_depth` defaults to 1, limiting recursive delegation unless raised.
- `agents.max_threads` defaults to 6, capping concurrent subagent threads.

## Format Recommendations

| Goal | Recommended format | Rationale |
| :--- | :--- | :--- |
| Append | Pure Markdown | Headers, bullet lists, and short paragraphs blend cleanly with the concatenated AGENTS.md chain and built-in instructions. |
| Replace | XML-wrapped Markdown | When the built-in structure is removed, XML tags such as `<rules>`, `<constraints>`, `<context>`, and `<examples>` help the model distinguish instruction categories. |

For replacements, the prompt should explicitly supply any tool-calling guidance, safety instructions, and environment context the task requires because the default built-in instructions are removed.

## Recent Changes

- **Custom agent TOML files (2026-06)**: Subagents can now be defined as standalone TOML files under `~/.codex/agents/` and `.codex/agents/`, each carrying its own `developer_instructions` and optional model/sandbox/MCP overrides.
- **Multi-agent collaboration enabled by default (2026-05)**: Tools such as `spawn_agent`, `send_input`, `resume_agent`, `wait_agent`, and `close_agent` are now on by default.
- **Personality controls stabilized (2026-04)**: The `personality` setting became a stable, on-by-default prompt layer.
- **Reasoning effort controls stabilized (2026-03)**: `model_reasoning_effort` with levels `minimal`/`low`/`medium`/`high`/`xhigh` became the standard way to adjust reasoning depth.
- **AGENTS.md and skills model (late 2025-2026)**: Codex moved from monolithic custom prompt files toward the `AGENTS.md` hierarchy and modular skills.

## Quirks and Workarounds

- Codex has no dedicated system-prompt flag; use the universal `-c` config override.
- Because `-c` values parse as TOML, multi-line `developer_instructions` strings need shell-escaped TOML quoting.
- The AGENTS.md chain is capped at 32 KiB by default; split large guidance across nested directories or raise `project_doc_max_bytes`.
- An `AGENTS.override.md` in the same directory silently shadows `AGENTS.md`, which can confuse users who expect both to load.
- Project-level `.codex/config.toml` cannot override provider, auth, profile, notify, or telemetry keys.
- Replacing instructions via `model_instructions_file` removes Codex's built-in guidance, so the replacement should include any tool/safety rules the task still needs.
- There is no built-in command to dump the effective system prompt; indirect verification requires enabling `log_dir` or asking Codex to summarize loaded instructions.

## Claudine Delivery Notes

Claudine should continue using its existing config-override delivery path:

- Discover a `system-prompt.md` file from the launch working-directory hierarchy.
- For append mode, prepare the content and pass it inline via `-c developer_instructions="..."`.
- For replace mode, write the resolved content to a temporary file and pass the path via `-c model_instructions_file=<tmp>`.
- Both modes are temporary per-invocation changes, so no user `config.toml`, `AGENTS.md`, or skill is permanently mutated.
- Because Codex parses `-c` values as TOML, Claudine must quote/escape multi-line content appropriately; using the file-backed replacement path avoids shell-escaping complexity for large prompts.

## Sources

- [Codex CLI overview](https://developers.openai.com/codex/cli)
- [Command line options](https://developers.openai.com/codex/cli/reference)
- [Prompting Codex](https://developers.openai.com/codex/prompting)
- [Custom instructions with AGENTS.md](https://developers.openai.com/codex/guides/agents-md)
- [Subagents](https://developers.openai.com/codex/subagents)
- [Configuration reference](https://developers.openai.com/codex/config-reference)
- [Environment variables](https://developers.openai.com/codex/environment-variables)
- [Codex GitHub repository](https://github.com/openai/codex)
