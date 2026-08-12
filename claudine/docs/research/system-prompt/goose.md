---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3
docs: https://goose-docs.ai/docs/guides/goose-cli-commands/
system_prompt_docs: https://goose-docs.ai/docs/guides/context-engineering/prompt-templates/
append_support: native
replace_support: indirect
cli_params:
  - flag: --system <TEXT>
    mode: append
    value_shape: inline text
    description: Adds additional system instructions for a single `goose run` invocation. The text is added as a `system_prompt_extra` under the key `additional` and is appended to the rendered system prompt under the heading `# Additional Instructions:`.
    example: goose run --system "Always run just test before committing" -t "fix tests"
    notes: 'Only available on `goose run`, not on `goose session`. In clap, `conflicts_with` is set to `"recipe"`, so `--system` and `--recipe` are mutually exclusive. Internally wired through `additional_system_prompt` on `SessionBuilderConfig` and `agent.extend_system_prompt("additional", text)`. No `--system-file` equivalent exists.'
  - flag: --recipe <RECIPE_FILE>
    mode: modify
    value_shape: YAML or JSON file path
    description: Loads a recipe whose `instructions` and `prompt` fields supply the task identity and task instructions. Recipes do not replace the core system prompt; they layer instructions on top.
    example: 'goose run --recipe security-auditor.yaml --params target=auth.ts'
    notes: 'Recipe `instructions` are added via `recipe::apply_recipe_components`. Recipes that define an explicit `extensions` list must include `summon` (type: platform) for subagent delegation to work, because that explicit list replaces the default platform extensions.'
  - flag: --sub-recipe <RECIPE>
    mode: modify
    value_shape: YAML or JSON file path
    description: Includes a subrecipe alongside the main recipe. Subrecipes auto-inject the `summon` platform extension.
    example: goose run --recipe main.yaml --sub-recipe helper.yaml
    notes: Subrecipes execute the `summon` platform extension implicitly so delegation works without listing it manually.
  - flag: --with-extension <COMMAND>
    mode: other
    value_shape: stdio command
    description: Adds a stdio MCP extension. Each extension contributes a section to the rendered system prompt under `# Extensions` (see `crates/goose/src/prompts/system.md`).
    example: goose run --with-extension "npx -y @modelcontextprotocol/server-memory" -t "list memories"
    notes: Affects the system prompt only because extension `instructions` are rendered into the base template.
  - flag: --with-streamable-http-extension <URL>
    mode: other
    value_shape: URL
    description: Adds a remote MCP extension over Streamable HTTP. Same prompt impact as stdio extensions.
    example: goose run --with-streamable-http-extension "http://localhost:8080/mcp" -t "list files"
    notes: Same prompt rendering behavior as stdio extensions.
  - flag: --with-builtin <name>
    mode: other
    value_shape: builtin name (comma-separated)
    description: Enables one or more builtin extensions. Their instructions are rendered into the system prompt.
    example: goose run --with-builtin developer,github -t "commit these changes"
    notes: Same prompt rendering behavior as stdio extensions.
  - flag: --render-recipe
    mode: inspect
    value_shape: boolean
    description: Prints the rendered recipe instead of running it. Useful for verifying how recipe `instructions` and parameters will be combined.
    example: goose run --recipe security-auditor.yaml --render-recipe
    notes: Does not affect the effective system prompt; the rendered output is recipe instructions plus the parameter-substituted prompt.
  - flag: --explain
    mode: inspect
    value_shape: boolean
    description: Shows a recipe's title, description, and parameters without running it.
    example: goose run --recipe security-auditor.yaml --explain
    notes: Does not affect the effective system prompt.
  - flag: --no-session
    mode: other
    value_shape: boolean
    description: Runs `goose run` without creating or storing a session file. Does not change the system prompt; useful for fully ephemeral CI runs.
    example: goose run --no-session -i instructions.txt
    notes: Same prompt surface as a regular session.
  - flag: --interactive / -s
    mode: other
    value_shape: boolean
    description: Continue in interactive mode after processing initial input. Does not change the system prompt; subsequent turns still re-inject MOIM persistent instructions every turn.
    example: goose run --recipe task.yaml --interactive
    notes: Same prompt surface; MOIM re-injection still applies.
config_sources:
  - os: macos
    scope: user
    path: ~/.config/goose/prompts/system.md
    mode: replace
    format: markdown
    notes: Overrides the built-in `system.md` Jinja2 template via `prompt_template::render_template("system.md", ...)`. Changes only take effect in new sessions. Delete the file to restore the default.
  - os: linux
    scope: user
    path: ~/.config/goose/prompts/system.md
    mode: replace
    format: markdown
    notes: Linux equivalent of the macOS path; same behavior.
  - os: windows
    scope: user
    path: '%APPDATA%\Block\goose\config\prompts\system.md'
    mode: replace
    format: markdown
    notes: Windows equivalent. The Block namespace (not goose) is intentional on Windows.
  - os: macos
    scope: user
    path: ~/.config/goose/prompts/subagent_system.md
    mode: replace
    format: markdown
    notes: Overrides the built-in `subagent_system.md` template used for subagents. Same per-session reload semantics as `system.md`.
  - os: linux
    scope: user
    path: ~/.config/goose/prompts/subagent_system.md
    mode: replace
    format: markdown
    notes: Linux equivalent.
  - os: windows
    scope: user
    path: '%APPDATA%\Block\goose\config\prompts\subagent_system.md'
    mode: replace
    format: markdown
    notes: Windows equivalent.
  - os: macos
    scope: user
    path: ~/.config/goose/.goosehints
    mode: append
    format: markdown
    notes: Global hints loaded at session start; appended as `hints` extra under `# Additional Instructions:`.
  - os: linux
    scope: user
    path: ~/.config/goose/.goosehints
    mode: append
    format: markdown
    notes: Linux equivalent.
  - os: windows
    scope: user
    path: '%APPDATA%\Block\goose\config\.goosehints'
    mode: append
    format: markdown
    notes: Windows equivalent; follows the Block namespace.
  - os: macos
    scope: user
    path: ~/.agents/AGENTS.md
    mode: append
    format: markdown
    notes: Global AGENTS.md context loaded at session start as of v1.41.0 (2026-07-03). Same `hints` extra as `.goosehints`.
  - os: linux
    scope: user
    path: ~/.agents/AGENTS.md
    mode: append
    format: markdown
    notes: Linux equivalent.
  - os: windows
    scope: user
    path: '%USERPROFILE%\.agents\AGENTS.md'
    mode: append
    format: markdown
    notes: Windows equivalent; uses `~/.agents` (not the Block namespace).
  - os: macos
    scope: repo
    path: ./AGENTS.md
    mode: append
    format: markdown
    notes: Project-level AGENTS.md. Loaded from the working directory up to the repo root at session start; nested files load on demand as goose reads files in those directories. Local files override global files.
  - os: linux
    scope: repo
    path: ./AGENTS.md
    mode: append
    format: markdown
    notes: Linux equivalent.
  - os: windows
    scope: repo
    path: '.\AGENTS.md'
    mode: append
    format: markdown
    notes: Windows equivalent.
  - os: macos
    scope: repo
    path: ./.goosehints
    mode: append
    format: markdown
    notes: Project-level hints file. Same nested-load semantics as project AGENTS.md.
  - os: linux
    scope: repo
    path: ./.goosehints
    mode: append
    format: markdown
    notes: Linux equivalent.
  - os: windows
    scope: repo
    path: '.\.goosehints'
    mode: append
    format: markdown
    notes: Windows equivalent.
  - os: macos
    scope: user
    path: ~/.config/goose/config.yaml
    mode: modify
    format: yaml
    notes: Provider, model, extensions, tool filtering, goose_mode, theme, and `GOOSE_SYSTEM_PROMPT_FILE_PATH` (undocumented) all flow through `Config::global()`. Settings that change behavior feed into prompt variables and the `extensions` section of the rendered prompt.
  - os: linux
    scope: user
    path: ~/.config/goose/config.yaml
    mode: modify
    format: yaml
    notes: Linux equivalent.
  - os: windows
    scope: user
    path: '%APPDATA%\Block\goose\config\config.yaml'
    mode: modify
    format: yaml
    notes: Windows equivalent.
  - os: macos
    scope: user
    path: ~/.config/goose/recipes/*.yaml
    mode: modify
    format: yaml
    notes: User-level recipes. Each recipe supplies `instructions` and `prompt` that become `system_prompt_extras` plus the user-turn task.
  - os: linux
    scope: user
    path: ~/.config/goose/recipes/*.yaml
    mode: modify
    format: yaml
    notes: Linux equivalent.
  - os: windows
    scope: user
    path: '%APPDATA%\Block\goose\config\recipes\*.yaml'
    mode: modify
    format: yaml
    notes: Windows equivalent.
  - os: macos
    scope: repo
    path: .goose/recipes/*.yaml
    mode: modify
    format: yaml
    notes: Project-level recipes.
  - os: linux
    scope: repo
    path: .goose/recipes/*.yaml
    mode: modify
    format: yaml
    notes: Linux equivalent.
  - os: windows
    scope: repo
    path: '.goose\recipes\*.yaml'
    mode: modify
    format: yaml
    notes: Windows equivalent.
  - os: macos
    scope: user
    path: ~/.agents/agents/*.md
    mode: replace
    format: markdown
    notes: 'Global custom agents. The Markdown body becomes the agent''s instructions. Loaded via `summon` delegation or `@mention` in chat. Compatibility paths: `.goose/agents/`, `.claude/agents/`, `~/.goose/agents/`, `~/.claude/agents/`.'
  - os: linux
    scope: user
    path: ~/.agents/agents/*.md
    mode: replace
    format: markdown
    notes: Linux equivalent.
  - os: windows
    scope: user
    path: '%USERPROFILE%\.agents\agents\*.md'
    mode: replace
    format: markdown
    notes: Windows equivalent.
  - os: macos
    scope: repo
    path: .agents/agents/*.md
    mode: replace
    format: markdown
    notes: Project custom agents. Project agents take precedence on name collisions.
  - os: linux
    scope: repo
    path: .agents/agents/*.md
    mode: replace
    format: markdown
    notes: Linux equivalent.
  - os: windows
    scope: repo
    path: '.agents\agents\*.md'
    mode: replace
    format: markdown
    notes: Windows equivalent.
env_vars:
  - name: CONTEXT_FILE_NAMES
    effect: Customizes the filenames Goose discovers as context/hint files. Accepts a JSON array of strings (default `["AGENTS.md", ".goosehints"]` per the hints guide).
    mode: modify
  - name: GOOSE_MOIM_MESSAGE_TEXT
    effect: Injects persistent text into Goose's MOIM working memory every turn. Concatenated with `GOOSE_MOIM_MESSAGE_FILE` content when both are set.
    mode: append
  - name: GOOSE_MOIM_MESSAGE_FILE
    effect: Reads file contents and injects them into MOIM working memory every turn. Supports `~/` and is capped at 64 KB (UTF-8 safe truncation).
    mode: append
  - name: GOOSE_RECIPE_PATH
    effect: Adds colon-separated (Unix) or semicolon-separated (Windows) directories to recipe search path.
    mode: modify
  - name: GOOSE_PATH_ROOT
    effect: Overrides the root directory for all Goose data, config, and state files (default macOS `~/Library/Application Support/Block/goose/`, Linux `~/.local/share/goose/`, Windows `%APPDATA%\Block\goose\`).
    mode: modify
  - name: GOOSE_SYSTEM_PROMPT_FILE_PATH
    effect: Undocumented in user docs but wired in `crates/goose-cli/src/session/builder.rs` via `config.get_param("GOOSE_SYSTEM_PROMPT_FILE_PATH")`. Reads the file contents and calls `agent.override_system_prompt`, replacing the `system.md` template for the session. Exits with a render error if the file cannot be read.
    mode: replace
  - name: GOOSE_SUBAGENT_MAX_TURNS
    effect: Sets the default maximum turn count for subagents before timeout (default 25).
    mode: modify
prompt_layers:
  - source: Custom ~/.config/goose/prompts/system.md (or ACP `Set` mode)
    mode: replace
    scope: ["session", "subagent"]
    order_notes: Highest-precedence template; replaces the built-in `system.md` entirely. Activated by writing the file or by `agent.override_system_prompt(...)` from ACP.
    notes: Jinja2-rendered with the same context as the built-in template (`extensions`, `current_date_time`, `extension_tool_limits`, `goose_mode`, `is_autonomous`, `enable_subagents`, `max_extensions`, `max_tools`, `code_execution_mode`, `moim_system_prompt_block`).
  - source: Built-in system.md template (crates/goose/src/prompts/system.md)
    mode: replace
    scope: ["builtin", "session"]
    order_notes: Base template when no custom override exists. Rendered via `prompt_template::render_template("system.md", ...)`.
    notes: Declares Goose's role, includes `moim_system_prompt_block`, extension section, tool-limit suggestion, and a response-guidelines section.
  - source: Built-in subagent_system.md template (crates/goose/src/prompts/subagent_system.md)
    mode: replace
    scope: ["subagent"]
    order_notes: Used by subagents spawned through `summon` (recipe delegation or ACP `delegate` tool). Overrides the main `system.md` for subagent sessions.
    notes: Independent of the parent session's effective prompt. Has its own `system_prompt_override` and `system_prompt_extras` map.
  - source: Extension instructions
    mode: append
    scope: ["session", "subagent"]
    order_notes: Rendered into the `# Extensions` section of the base template via the `extensions` Jinja2 variable.
    notes: Each enabled extension contributes `name`, optional resources note, and its `instructions` text. Disabled extensions are filtered out.
  - source: system_prompt_extras (final_output_tool, additional, chat_mode, hints, project_instructions, others)
    mode: append
    scope: ["session", "subagent"]
    order_notes: Joined with `\n\n` separators and appended to the rendered template under the heading `# Additional Instructions:`. Order is the order keys were inserted into the `IndexMap`.
    notes: The `additional` key comes from `--system`; the `hints` key comes from `with_hints(...)`; the `chat_mode` key is added in chat-only mode; the `final_output` key is added when the final_output tool is in use; the `project_instructions` key is added in `agent.reply()` via `load_project_instructions`.
  - source: AGENTS.md / .goosehints hierarchy
    mode: append
    scope: ["user", "repo"]
    order_notes: Files discovered from the working directory up to the repo root at session start. Nested files load on demand as Goose reads files in those directories, then remain active for the rest of the session.
    notes: Local files override global files on name collisions. Filenames are configurable via `CONTEXT_FILE_NAMES`. `@path` references embed file contents immediately; plain references are advisory.
  - source: --system flag (additional_system_prompt)
    mode: append
    scope: ["run"]
    order_notes: Added to `system_prompt_extras` under key `additional` via `agent.extend_system_prompt("additional", text)`. Conflicts with `--recipe` in clap.
    notes: Per-invocation; not persisted. No `--system-file` equivalent.
  - source: GOOSE_MOIM_MESSAGE_TEXT / GOOSE_MOIM_MESSAGE_FILE (MOIM)
    mode: append
    scope: ["session", "subagent"]
    order_notes: Injected into the base template's `moim_system_prompt_block` Jinja2 variable AND re-read fresh every turn by the MOIM component.
    notes: Stronger than one-shot system-prompt additions because the text cannot be forgotten as context grows.
  - source: Recipe instructions (instructions, prompt)
    mode: modify
    scope: ["run"]
    order_notes: Applied through `recipe::apply_recipe_components` when `goose run --recipe <file>` is used. At least one of `instructions` or `prompt` must be present (validated by `validate_prompt_or_instructions`).
    notes: Recipes do not replace the core system prompt; they supply task-specific instructions and a Jinja2-substituted prompt.
  - source: ACP set_session_system_prompt (Set / Append)
    mode: replace
    scope: ["session"]
    order_notes: Replaces the template entirely (Set) or appends an extra under a caller-supplied key (Append).
    notes: Exposed only through the ACP server (`goose acp`), not via CLI. Empty `text` with mode Set calls `clear_system_prompt_override`; with mode Append calls `remove_system_prompt_extra(key)`.
agent_prompting:
  supported: true
  definition_surface: 'Markdown files with YAML frontmatter (`name` required; `description` and `model` optional) under global `~/.agents/agents/*.md` or project `<project>/.agents/agents/*.md`. Compatibility discovery paths: `.goose/agents/`, `.claude/agents/`, `~/.goose/agents/`, `~/.claude/agents/`.'
  inheritance: Delegated agents run in separate sessions with the agent file body as their instructions. Loading an agent in the current session adds its instructions to context without creating a separate session. Agents can be invoked by mention (`@code-reviewer`) or delegated to (subagent spawn). When invoked via a recipe, the recipe's `extensions` list must include `summon` (or `sub_recipes` must be defined, in which case `summon` is auto-injected).
  isolation: Each delegated agent/subagent runs in its own session/context window; only the final summary returns to the parent. Subagents use the `subagent_system.md` template plus their task instructions plus any inherited extensions.
  limitations: Subagents cannot spawn further subagents, enable/disable extensions, or manage scheduled tasks (per the platform extension security constraints in `summon`). External subagents run as MCP servers and inherit the parent's environment variables (`env_keys`). Default subagent turn limit is 25; default timeout is 5 minutes.
claudine_delivery:
  append_strategy: inline_flag
  replace_strategy: unsupported
  temp_file_required: false
  argv_limit: Subject to the platform argv size limit; the `--system <TEXT>` value is a single inline string with no documented provider-side cap. Long content is better passed via `GOOSE_SYSTEM_PROMPT_FILE_PATH` (replace) or via `GOOSE_MOIM_MESSAGE_FILE` (append, every turn) than through argv.
  notes: "Goose only exposes a native append flag (`goose run --system <TEXT>`); there is no `--system-file` and no full CLI replace flag. Claudine should pass composed append content directly to `--system` for non-interactive runs. For interactive `goose session` there is no native append flag — interactive append can only be achieved via a shadow `~/.config/goose/.goosehints` or `~/.agents/AGENTS.md`, or by injecting `GOOSE_MOIM_MESSAGE_FILE` content. The undocumented `GOOSE_SYSTEM_PROMPT_FILE_PATH` config param is a possible replace path, but using it would persist config for the duration of the process and is not part of the public API surface."
format_recommendations:
  append_format: markdown
  replace_format: markdown
  rationale: "Goose's built-in `system.md` is Markdown with Jinja2 substitutions, and the `# Additional Instructions:` heading that wraps the appended extras is plain Markdown. Pure Markdown blends naturally with both the base template and the appended block. XML wrapping is not used anywhere in the prompt templates, recipes, or AGENTS.md examples; tags like `<instructions>` would add tokens without a documented benefit. For a custom `system.md` that fully replaces the template, the same Markdown format applies because Jinja2 expects raw Markdown."
recent_changes:
  - date: "2026-07-03"
    version: "v1.41.0"
    change: "Global hints now load from `~/.agents/AGENTS.md` (in addition to `~/.config/goose/.goosehints`). Discovered via PR #9736."
    impact: "User-level AGENTS.md context files are now discovered alongside `.goosehints`. Discovery behavior is controlled by `CONTEXT_FILE_NAMES`; the default documented in the hints guide is `[\"AGENTS.md\", \".goosehints\"]`."
  - date: "2026-06-25"
    version: "v1.39.0"
    change: "Added ACP `set_session_system_prompt` method with `Set` and `Append` modes (PR #9478). Set mode calls `agent.override_system_prompt(text)`; Append mode calls `agent.extend_system_prompt(key, text)` with a required caller-supplied key."
    impact: "ACP clients can now programmatically replace or append to the active session's system prompt without restarting. Not exposed via the CLI; visible only in the ACP server source (`crates/goose/src/acp/server/manage_sessions.rs`)."
  - date: "2026-05–2026-06"
    version: "v1.34.0–v1.38.0"
    change: "Projects can act as backend sources with system-prompt injection (PR #8739). Project metadata feeds into `load_project_instructions` and is appended to the rendered system prompt in `agent.reply()`."
    impact: "Each project can carry its own `instructions` field that is appended to the system prompt when the project is active."
  - date: "2026-02"
    version: "v1.27.0"
    change: "Restored correct subagent system-prompt behavior. Subagents again receive their dedicated `subagent_system.md` template plus task instructions instead of inheriting the parent's full prompt."
    impact: "Subagents have an isolated prompt surface; custom `subagent_system.md` files are honored again."
quirks:
  - "`--system` is only available on `goose run`. `goose session` has no equivalent CLI flag; interactive append has no first-class CLI mechanism."
  - "There is no native `--system-prompt-file` or full `--replace-system-prompt` CLI flag. The only replace path is a custom `~/.config/goose/prompts/system.md` (persistent) or the undocumented `GOOSE_SYSTEM_PROMPT_FILE_PATH` config param (per-session)."
  - "`--system` and `--recipe` are mutually exclusive in clap (`conflicts_with = \"recipe\"`). To combine them, use a recipe's `extensions` block plus the recipe's own `instructions` field, or pass instructions through `.goosehints` / AGENTS.md."
  - "The `additional` extra key under `# Additional Instructions:` is rendered as plain text; no Jinja2 substitution is applied to `--system` content. The base template, however, still goes through `prompt_template::render_template`."
  - "MOIM persistent instructions (`GOOSE_MOIM_MESSAGE_TEXT`, `GOOSE_MOIM_MESSAGE_FILE`) are re-read fresh every turn and can override softer system-prompt instructions because they cannot be forgotten as context grows."
  - "Nested `.goosehints` / `AGENTS.md` files remain active for the rest of the session once loaded, even after leaving that directory. Update reliably only by restarting the session."
  - "Recipes with an explicit `extensions` block must include `summon` (type: platform) or delegation/subagent tools will be unavailable. Recipes that define `sub_recipes` auto-inject `summon` and do not need to list it explicitly."
  - "Custom `system.md` (and `subagent_system.md`) changes only take effect in new sessions. Restart after editing."
  - "Goose does not publish a CLI command to dump the fully rendered effective system prompt as plain text. The closest is `--render-recipe` for recipe-only inspection."
  - "`CONTEXT_FILE_NAMES` default is documented inconsistently: the [environment variables guide](https://goose-docs.ai/docs/guides/environment-variables) lists `[\".goosehints\"]`; the [hints guide](https://goose-docs.ai/docs/guides/context-engineering/using-goosehints) lists `[\"AGENTS.md\", \".goosehints\"]`. The hints guide reflects the v1.41.0+ behavior; the env vars page appears stale."
  - "`GOOSE_SYSTEM_PROMPT_FILE_PATH` is referenced in code (`config.get_param` in `builder.rs`) but is absent from the public environment variables guide and from `goose --help`. It is an undocumented internal mechanism, not a supported public API."
  - "Subagents cannot spawn further subagents, enable/disable extensions, or manage scheduled tasks (per the `summon` platform extension's documented security constraints)."
  - "Default subagent timeout is 5 minutes; default max turns is 25 (`GOOSE_SUBAGENT_MAX_TURNS`). Override per-call via the `summon` `delegate` tool's `max_turns` parameter or per-recipe via `settings.max_turns`."
  - "Custom agents can be loaded into the current session (adds instructions to context) or delegated to (separate session, returns summary). The two behaviors are distinct and invoked through different tools/keywords."
gaps:
  - "The exact ordering of `.goosehints`, `AGENTS.md`, `--system`, and recipe `instructions` in the effective prompt is partially inferred from the `system_prompt_extras` IndexMap insertion order in `prompt_manager.rs`; an end-to-end dump is not publicly exposed for verification."
  - "Goose has no CLI to export or inspect the fully rendered effective built-in system prompt as plain text; `--render-recipe` only covers the recipe layer."
  - "Whether `--system` content is visible to subagents spawned in the same session is undocumented. Code suggests each subagent session has its own `prompt_manager`, so the parent's `--system` text is not automatically inherited."
  - "The `CONTEXT_FILE_NAMES` default inconsistency between the environment-variables guide (`[\".goosehints\"]`) and the hints guide (`[\"AGENTS.md\", \".goosehints\"]`) needs code-level resolution; the local macOS install is not available on this host, so the actual default could not be probed."
  - "`GOOSE_SYSTEM_PROMPT_FILE_PATH` is undocumented. Its behavior is inferred from `crates/goose-cli/src/session/builder.rs`. It may be removed or renamed in a future release; treat as fragile."
  - "Whether the `additional` system-prompt extra (from `--system`) participates in MOIM re-injection is unclear; the documented MOIM re-injection path appears to only re-read `GOOSE_MOIM_MESSAGE_*` env vars."
  - "The full list of overridable prompt templates (`template_registry`) lives in `crates/goose/src/prompt_template.rs`. Only `system.md`, `subagent_system.md`, `plan.md`, `compaction.md`, `permission_judge.md`, `recipe.md`, `apps_create.md`, and `apps_iterate.md` are documented in the prompt templates guide; any newer additions need a code-side audit."
changes:
  - "2026-07-03 refresh: split `os: all` config_sources records into separate macOS/Linux/Windows records to satisfy the `_schema.yaml` `os` enum (a formatting fix, not a behavior change). Added the undocumented `GOOSE_SYSTEM_PROMPT_FILE_PATH` config param discovered in `crates/goose-cli/src/session/builder.rs`. Documented the ACP `set_session_system_prompt` method with Set/Append modes from `crates/goose/src/acp/server/manage_sessions.rs`. Promoted `replace_support` from `none` to `indirect` because the ACP method and the `system_prompt_override` runtime hook together provide a programmatic replace path; the CLI still has no native replace flag. Promoted `append_support` rationale from `native` to a clear `native` (the `--system` flag maps to `extend_system_prompt(\"additional\", text)` and is documented end-to-end in code). Confirmed v1.41.0 release notes for AGENTS.md global hint loading and `CONTEXT_FILE_NAMES` default. Cross-referenced the recipe `summon` requirement and subagent security constraints. Verified that `--system` and `--recipe` are mutually exclusive in clap via `conflicts_with`."
requires_claudine_update: true
reason: "The current ProviderInfo for Goose declares `SystemPromptDelivery::Custom(GooseRecipe)` for interactive append, but the wrap layer treats `Custom` as a no-op. Goose's actual model is: append via `goose run --system <TEXT>` (non-interactive only, native inline flag); replace only indirectly via a custom `~/.config/goose/prompts/system.md` (persistent) or the undocumented `GOOSE_SYSTEM_PROMPT_FILE_PATH` config param. Claudine's `SystemPromptSpec` should reflect non-interactive-only inline-flag append and indirect replace (with a temp-file + GOOSE_SYSTEM_PROMPT_FILE_PATH approach OR via a shadowed prompts directory); interactive append has no first-class CLI mechanism and should fall back to `GOOSE_MOIM_MESSAGE_FILE` or a shadow `.goosehints`."
---

# Goose CLI System Prompt Research

## Overview

Goose CLI builds the effective prompt for each session from several layers. The base is a Jinja2 template stored as `crates/goose/src/prompts/system.md` (or `subagent_system.md` for subagents). Users can override the base template by writing a custom `system.md` under `~/.config/goose/prompts/` (macOS/Linux) or `%APPDATA%\Block\goose\config\prompts\` (Windows). On top of the base, the prompt builder renders extension instructions, then appends an ordered set of `system_prompt_extras` under the heading `# Additional Instructions:`. Extras come from four sources: a `hints` key (the AGENTS.md / .goosehints hierarchy), an `additional` key (`goose run --system <TEXT>`), a `chat_mode` key when chat-only mode is active, and a `final_output` key when the final-output tool is engaged. MOIM persistent instructions (`GOOSE_MOIM_MESSAGE_TEXT` / `GOOSE_MOIM_MESSAGE_FILE`) are injected into the base template's `moim_system_prompt_block` and re-read fresh every turn, making them stronger than one-shot system-prompt additions. Recipe `instructions` do not replace the core system prompt; they layer task-specific instructions on top via `recipe::apply_recipe_components`. There is no `--system-prompt` or `--system-prompt-file` CLI flag; the only programmatic replace paths are a custom `system.md` file, the undocumented `GOOSE_SYSTEM_PROMPT_FILE_PATH` config param, or the ACP `set_session_system_prompt` method.

## CLI Parameters

The `goose run` subcommand exposes the only native system-prompt flag. There is no equivalent on `goose session`.

| Flag | Mode | Effect |
| :--- | :--- | :--- |
| `--system <TEXT>` | Append | Adds the supplied text as an extra under the key `additional`; appended to the rendered prompt under `# Additional Instructions:`. |
| `--recipe <FILE>` | Modify | Loads a recipe whose `instructions` and `prompt` fields become task instructions. Mutually exclusive with `--system`. |
| `--sub-recipe <FILE>` | Modify | Adds a subrecipe alongside the main recipe; auto-injects the `summon` platform extension. |
| `--with-extension <COMMAND>` | Other | Adds a stdio extension whose `instructions` are rendered into the `# Extensions` section of the base template. |
| `--with-streamable-http-extension <URL>` | Other | Adds a remote extension; same prompt impact as stdio. |
| `--with-builtin <name>` | Other | Enables a builtin extension; same prompt impact. |
| `--render-recipe` | Inspect | Prints the rendered recipe instead of running it. |
| `--explain` | Inspect | Shows the recipe's title, description, and parameters. |
| `--no-session` | Other | Disables session persistence; no prompt-surface change. |
| `--interactive / -s` | Other | Continues in interactive mode after initial input; no prompt-surface change. |

The `--system` flag is wired in `crates/goose-cli/src/cli.rs` (`input.system -> input_config.additional_system_prompt -> SessionBuilderConfig.additional_system_prompt -> agent.extend_system_prompt("additional", text)`). It conflicts with `--recipe` in clap, so the two cannot be combined on a single `goose run` invocation.

## Configuration and Discovery

### Prompt templates

Goose's built-in prompts live under `crates/goose/src/prompts/` and are registered in `crates/goose/src/prompt_template.rs`. The user can override any of them by writing the same filename under `~/.config/goose/prompts/` (macOS/Linux) or `%APPDATA%\Block\goose\config\prompts\` (Windows).

| Template | Effect |
| :--- | :--- |
| `system.md` | Replaces the main session system prompt. |
| `subagent_system.md` | Replaces the system prompt used for subagents. |
| `plan.md` | Overrides the planning-mode prompt. |
| `compaction.md` | Overrides the conversation-compaction prompt. |
| `permission_judge.md` | Overrides the read-only permission classifier. |
| `recipe.md` | Overrides the recipe-generation prompt. |
| `apps_create.md`, `apps_iterate.md` | Override the standalone-apps prompts (Desktop only). |

Overriding `system.md` or `subagent_system.md` is the only fully supported persistent replace path. Changes take effect only in new sessions.

### Hints and context files

Goose discovers two filenames by default and merges them into the `hints` system-prompt extra at session start:

- **Global**: `~/.config/goose/.goosehints` (macOS/Linux), `%APPDATA%\Block\goose\config\.goosehints` (Windows)
- **Global**: `~/.agents/AGENTS.md` (all OSes, as of v1.41.0)
- **Project**: `./.goosehints` and `./AGENTS.md` from the working directory up to the repo root

Filenames can be customized with `CONTEXT_FILE_NAMES` (JSON array). Nested files load on demand as Goose reads files in those directories, then remain active for the rest of the session. Local files override global files on name collisions.

### Recipes

Recipes are YAML or JSON files that package instructions, extensions, parameters, retry logic, and a structured `response.json_schema`. At least one of `instructions` or `prompt` must be present. Recipes are launched via `goose run --recipe <file>` and supply task-specific guidance on top of the core system prompt. Recipes with an explicit `extensions` block must include `summon` (type: platform) or the `delegate` tool will be unavailable; recipes that define `sub_recipes` auto-inject `summon`.

### MOIM persistent instructions

`GOOSE_MOIM_MESSAGE_TEXT` and `GOOSE_MOIM_MESSAGE_FILE` inject text into Goose's MOIM working memory every turn. Both are concatenated when both are set. Content is capped at 64 KB with UTF-8 safe truncation. Unlike `.goosehints`, MOIM instructions cannot be forgotten as context grows, making them the strongest documented lever for "must never be ignored" guardrails.

### ACP runtime setters (undocumented in user docs; in code)

The `goose acp` server exposes a `set_session_system_prompt` JSON-RPC method:

- `mode = "set"`: Replaces the prompt template for the active session via `agent.override_system_prompt(text)`. Empty text clears the override.
- `mode = "append"`: Adds an extra under a caller-supplied `key` via `agent.extend_system_prompt(key, text)`. Empty text removes the extra under that key.

These setters are exposed only via ACP, not via the CLI.

## Prompt Layers and Precedence

The effective system prompt for a session is assembled in this order:

```mermaid
graph TD
    A[Custom system.md override OR ACP Set] --> B[Built-in system.md template]
    B --> C[moim_system_prompt_block if defined]
    C --> D[Extension instructions rendered under # Extensions]
    D --> E[system_prompt_extras joined under # Additional Instructions:]
    E --> E1[hints key -- AGENTS.md + .goosehints]
    E --> E2[additional key -- --system flag]
    E --> E3[chat_mode key -- chat-only mode]
    E --> E4[final_output key -- final_output_tool]
    E --> E5[other keys via extend_system_prompt]
    E --> F[project_instructions appended in agent.reply]
    F --> G[User prompt]
    G --> H[Per-turn MOIM re-injection from GOOSE_MOIM_MESSAGE_*]
```

Notes on precedence:

- A custom `system.md` (or ACP `Set` mode) replaces the built-in template entirely; the rest of the layers still attach.
- `.goosehints` and `AGENTS.md` are appended as the `hints` extra inside `# Additional Instructions:`.
- `--system` is appended under the `additional` key; it does not replace anything.
- `GOOSE_MOIM_MESSAGE_*` instructions appear in the base template's `moim_system_prompt_block` AND are re-read fresh every turn via the MOIM component.
- Recipe `instructions` and `prompt` are layered on top by `recipe::apply_recipe_components` and never replace the core system prompt.

## Agents and Subagents

Goose supports two distinct agent surfaces: custom agents (loaded into the current session or delegated to) and subagents (spawned through the `summon` platform extension).

### Custom agents

Custom agents live as Markdown files with YAML frontmatter (`name` required; `description` and `model` optional) under:

- Global: `~/.agents/agents/*.md` (macOS/Linux/Windows)
- Project: `<project>/.agents/agents/*.md`

Compatibility paths also discovered: `.goose/agents/`, `.claude/agents/`, `~/.goose/agents/`, `~/.claude/agents/`. The Markdown body becomes the agent's instructions.

| Use | Mechanism |
| :--- | :--- |
| Load into current session | `@code-reviewer` mention, or "Load the code-reviewer agent" |
| Delegate to a new session | "Delegate to code-reviewer: ..." (via `summon` `delegate` tool) |

A delegated agent runs in a separate session with its own `prompt_manager` and returns a summary to the parent.

### Subagents

Subagents are spawned via the `summon` platform extension's `delegate` and `load` tools. They run in isolated sessions with the `subagent_system.md` template (or a custom override) plus the task instructions.

| Setting | Default | Override |
| :--- | :--- | :--- |
| Max turns | 25 | `GOOSE_SUBAGENT_MAX_TURNS`, `recipe.settings.max_turns`, or per-call `max_turns` parameter |
| Timeout | 5 minutes | Per-call prompt override |
| Extensions | Inherited from parent | Restrict via per-call prompt |
| Return mode | Full details | Per-call "summary only" prompt |

Documented security constraints: subagents cannot spawn further subagents, enable/disable extensions, or manage scheduled tasks. External subagents are MCP servers (e.g., Codex running as `mcp-server`) that inherit the parent's `env_keys`.

## Format Recommendations

| Goal | Recommended format | Rationale |
| :--- | :--- | :--- |
| Append (`--system`, `.goosehints`, AGENTS.md) | Pure Markdown | Headers, bullet lists, and short paragraphs blend cleanly with Goose's Markdown system prompt. XML tags add tokens without a documented benefit. |
| Replace (custom `system.md`, recipe `instructions`) | Pure Markdown | Goose's templates use Markdown with Jinja2 substitutions; XML wrapping would break Jinja2-aware processing and is not used in any documented example. |

The base template uses Jinja2 substitutions, so variables like `{{ extensions }}`, `{{ hints }}`, and `{{ moim_system_prompt_block }}` are reserved inside the template. Avoid writing `{{ ... }}` in appended content unless you intend literal Jinja2 substitution.

## Recent Changes

- **v1.41.0 (2026-07-03)**: Global hints now load from `~/.agents/AGENTS.md` in addition to `~/.config/goose/.goosehints`. Discovery is configurable via `CONTEXT_FILE_NAMES`.
- **v1.39.0 (2026-06-25)**: Added the ACP `set_session_system_prompt` method with `Set` and `Append` modes. Replaces or appends to the active session's system prompt programmatically.
- **v1.34.0–v1.38.0 (2026-05–2026-06)**: Projects can act as backend sources with system-prompt injection via `load_project_instructions`. Project metadata feeds into the rendered prompt in `agent.reply()`.
- **v1.27.0 (2026-02)**: Restored correct subagent system-prompt behavior; custom `subagent_system.md` overrides are honored again.
- **v1.41.0 (2026-07-03)**: Added the `--edit` session flag for editing a session's conversation before forking (`goose session --resume --edit --fork`). Unrelated to system prompts; listed here for completeness because it appears in the same release.

## Quirks and Workarounds

- `--system` only works on `goose run`. For interactive sessions, use `.goosehints`, AGENTS.md, `GOOSE_MOIM_MESSAGE_TEXT` / `GOOSE_MOIM_MESSAGE_FILE`, or load a custom agent.
- There is no CLI flag to replace the entire system prompt. The supported persistent replace path is a custom `~/.config/goose/prompts/system.md`; the undocumented per-session replace path is `GOOSE_SYSTEM_PROMPT_FILE_PATH`.
- `--system` and `--recipe` are mutually exclusive in clap. Combine them by writing the instructions into a recipe's `instructions` field, or by using `.goosehints` / AGENTS.md instead of `--system`.
- MOIM persistent instructions are re-read fresh every turn and are stronger than one-shot `--system` text because they cannot be forgotten as context grows.
- Nested `.goosehints` / `AGENTS.md` files are sticky: once loaded for a directory, they remain active for the rest of the session.
- Recipes with an explicit `extensions` block must list `summon` (type: platform) or delegation/subagent tools will not be available. Recipes with `sub_recipes` auto-inject `summon`.
- Custom `system.md` / `subagent_system.md` overrides only apply to new sessions; restart after editing.
- Goose has no CLI to export the fully rendered effective system prompt as plain text. The closest is `--render-recipe`, which only covers the recipe layer.
- `CONTEXT_FILE_NAMES` default is documented inconsistently between the environment variables guide (`[".goosehints"]`) and the hints guide (`["AGENTS.md", ".goosehints"]`); the hints guide reflects v1.41.0+ behavior.
- `GOOSE_SYSTEM_PROMPT_FILE_PATH` is wired in `crates/goose-cli/src/session/builder.rs` but is absent from the public environment variables guide. Treat as an undocumented internal mechanism.
- Subagents cannot spawn further subagents, enable/disable extensions, or manage scheduled tasks. Default subagent turn limit is 25; default timeout is 5 minutes.
- A delegated custom agent runs in a separate session with the agent file body as its instructions. Loading an agent (via `@mention` or "Load the X agent") adds its instructions to the current conversation context without creating a separate session.

## Claudine Delivery Notes

- **Append (non-interactive)**: Use `goose run --system <TEXT>` to append composed content for a single run. This is per-invocation and does not mutate user config.
- **Append (interactive)**: There is no native CLI flag. The wrapper can fall back to writing a shadow `.goosehints` under a temporary working directory (or to `GOOSE_MOIM_MESSAGE_FILE`) to inject instructions that survive across turns. Neither path mutates the user's actual config files.
- **Replace (CLI)**: Not supported natively. The supported persistent replace path is a custom `system.md`, which would persist beyond the wrapper run — not desirable for ephemeral wrapping. The undocumented `GOOSE_SYSTEM_PROMPT_FILE_PATH` config param is a per-session replace that the wrapper can set via `Config::global().set_param(...)`, but it is not part of the public API and should be treated as fragile. A safer alternative for non-interactive runs is to write a shadow `system.md` under a shadowed config root via `GOOSE_PATH_ROOT`.
- **Replace (ACP)**: For ACP clients, use the `set_session_system_prompt` JSON-RPC method. Not applicable to non-ACP wrappers.
- **No temp file needed for append**: `--system <TEXT>` accepts inline text, so a temp file is unnecessary for append. Keep content within platform argv limits.
- **Temp file for replace (if used)**: If using a shadowed `system.md` or `GOOSE_SYSTEM_PROMPT_FILE_PATH`, the wrapper must write a temp file containing the composed prompt and reference it from the config layer.

## Changelog

- 2026-07-03 — refresh: split `os: all` config_sources records into separate macOS/Linux/Windows records to satisfy the `_schema.yaml` `os` enum (a formatting fix, not a behavior change). Documented the previously undocumented `GOOSE_SYSTEM_PROMPT_FILE_PATH` config param discovered in `crates/goose-cli/src/session/builder.rs`. Documented the ACP `set_session_system_prompt` method (Set / Append modes) from `crates/goose/src/acp/server/manage_sessions.rs`. Promoted `replace_support` from `none` to `indirect` to reflect the runtime replace paths (`system_prompt_override` via custom `system.md`, `GOOSE_SYSTEM_PROMPT_FILE_PATH`, and ACP `Set`). Added the prompt-layer rendering order inferred from `crates/goose/src/agents/prompt_manager.rs` (`system_prompt_extras` IndexMap → `# Additional Instructions:` join). Added the Mermaid diagram of the prompt layer order. Added a note on the `CONTEXT_FILE_NAMES` default inconsistency between the two relevant docs. Verified v1.41.0 release notes for `~/.agents/AGENTS.md` global hints (PR #9736). Cross-referenced the recipe `summon` requirement, subagent security constraints, and `conflicts_with = "recipe"` clap constraint.

## Sources

- [Goose CLI commands](https://goose-docs.ai/docs/guides/goose-cli-commands/)
- [Customizing prompt templates](https://goose-docs.ai/docs/guides/context-engineering/prompt-templates/)
- [Using goosehints](https://goose-docs.ai/docs/guides/context-engineering/using-goosehints)
- [Persistent instructions](https://goose-docs.ai/docs/guides/context-engineering/using-persistent-instructions)
- [Subagents](https://goose-docs.ai/docs/guides/context-engineering/subagents)
- [Custom agents](https://goose-docs.ai/docs/guides/context-engineering/custom-agents)
- [Configuration files](https://goose-docs.ai/docs/guides/config-files)
- [Environment variables](https://goose-docs.ai/docs/guides/environment-variables)
- [Recipe reference](https://goose-docs.ai/docs/guides/recipes/recipe-reference)
- [Goose moves to AAIF](https://goose-docs.ai/blog/2026/04/07/goose-moves-to-aaif/)
- [Goose releases on GitHub](https://github.com/aaif-goose/goose/releases)
- [Built-in `system.md` template source](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/prompts/system.md)
- [Built-in `subagent_system.md` template source](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/prompts/subagent_system.md)
- [Prompt manager (`system_prompt_extras` + builder)](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/agents/prompt_manager.rs)
- [CLI `--system` wiring (`InputConfig.additional_system_prompt`)](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/cli.rs)
- [Session builder (extend_system_prompt + GOOSE_SYSTEM_PROMPT_FILE_PATH)](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/session/builder.rs)
- [ACP set_session_system_prompt handler](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/acp/server/manage_sessions.rs)
- [ACP server new_session / fork_session entry points](https://github.com/aaif-goose/goose/tree/main/crates/goose/src/acp/server)