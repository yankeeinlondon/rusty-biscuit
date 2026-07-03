---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: opencode
model: k2p7
docs: https://goose-docs.ai/docs/guides/goose-cli-commands/
system_prompt_docs: https://goose-docs.ai/docs/guides/context-engineering/prompt-templates/
append_support: native
replace_support: none
cli_params:
  - flag: --system
    mode: append
    value_shape: string
    description: Provide additional system instructions for a single non-interactive run.
    example: goose run -t "fix tests" --system "Always run just test before committing"
    notes: Only available on `goose run`, not on `goose session`. Content is appended to the built-in system prompt.
  - flag: --recipe
    mode: modify
    value_shape: string
    description: Load a YAML recipe that supplies its own instructions and prompt for the run.
    example: goose run --recipe security-auditor.yaml --params target=auth.ts
    notes: Recipes package reusable instructions, extensions, and parameters; they do not replace the core system prompt but override the run's task identity.
  - flag: --render-recipe
    mode: inspect
    value_shape: boolean
    description: Print the rendered recipe instead of running it.
    example: goose run --recipe security-auditor.yaml --render-recipe
    notes: Useful for verifying how recipe instructions and parameters will be combined.
  - flag: --explain
    mode: inspect
    value_shape: boolean
    description: Show a recipe's title, description, and parameters without running it.
    example: goose run --recipe security-auditor.yaml --explain
    notes: Does not affect the effective system prompt.
config_sources:
  - os: all
    scope: user
    path: ~/.config/goose/prompts/system.md
    mode: replace
    format: markdown
    notes: Overrides the built-in system prompt template. Changes take effect in new sessions. Deleting the file restores the default.
  - os: all
    scope: user
    path: ~/.config/goose/prompts/subagent_system.md
    mode: replace
    format: markdown
    notes: Overrides the built-in system prompt template used for subagents.
  - os: all
    scope: user
    path: ~/.config/goose/.goosehints
    mode: append
    format: markdown
    notes: Global project hints loaded at session start.
  - os: all
    scope: repo
    path: ./.goosehints
    mode: append
    format: markdown
    notes: Project-local hints. Nested files under the repo root are loaded as files in those directories are accessed, and remain active for the rest of the session.
  - os: all
    scope: user
    path: ~/.agents/AGENTS.md
    mode: append
    format: markdown
    notes: Global AGENTS.md context loaded at session start as of v1.41.0.
  - os: all
    scope: repo
    path: ./AGENTS.md
    mode: append
    format: markdown
    notes: Project-local AGENTS.md context loaded at session start and in nested directories.
  - os: all
    scope: user
    path: ~/.config/goose/config.yaml
    mode: modify
    format: yaml
    notes: Sets provider, model, extensions, tool filtering, and other runtime behavior that appears in prompt variable substitution.
  - os: all
    scope: repo
    path: .goose/recipes/*.yaml
    mode: modify
    format: yaml
    notes: Reusable run configurations with instructions, extensions, and parameters.
  - os: all
    scope: user
    path: ~/.config/goose/recipes/*.yaml
    mode: modify
    format: yaml
    notes: User-level reusable recipes.
env_vars:
  - name: CONTEXT_FILE_NAMES
    effect: Customizes the filenames Goose discovers as context/hint files (default ["AGENTS.md", ".goosehints"]).
    mode: modify
  - name: GOOSE_MOIM_MESSAGE_TEXT
    effect: Injects persistent text into Goose's working memory every turn.
    mode: append
  - name: GOOSE_MOIM_MESSAGE_FILE
    effect: Loads persistent instructions from a file into working memory every turn. Supports `~/` and is capped at 64 KB.
    mode: append
  - name: GOOSE_RECIPE_PATH
    effect: Adds directories to search for recipes.
    mode: modify
  - name: GOOSE_PATH_ROOT
    effect: Overrides the root directory for all Goose data, config, and state files; useful for isolated test runs.
    mode: modify
prompt_layers:
  - source: Built-in system.md template
    mode: replace
    scope:
      - session
    order_notes: Base layer; replaced entirely if a custom ~/.config/goose/prompts/system.md exists.
    notes: Defines Goose's role, extension handling, and response guidelines. Uses Jinja2 variable substitution.
  - source: moim_system_prompt_block
    mode: append
    scope:
      - session
    order_notes: Injected into the base system template if defined.
    notes: Part of the built-in template's `{% if moim_system_prompt_block is defined %}` block.
  - source: Extension instructions
    mode: append
    scope:
      - session
    order_notes: Rendered after the base template when extensions are active.
    notes: Each enabled extension contributes its instructions to the system prompt.
  - source: AGENTS.md / .goosehints hierarchy
    mode: append
    scope:
      - user
      - repo
    order_notes: Loaded from working directory up to repo root at session start; nested files load on demand.
    notes: Project context, not a hard system-prompt layer. Local files override global files.
  - source: --system flag
    mode: append
    scope:
      - run
    order_notes: Applied to a single `goose run` invocation.
    notes: Appends additional system instructions to the built-in prompt for that run only.
  - source: GOOSE_MOIM_MESSAGE_TEXT / GOOSE_MOIM_MESSAGE_FILE
    mode: append
    scope:
      - session
    order_notes: Re-injected into working memory every turn, after system prompt layers.
    notes: Intended for critical guardrails that must never be forgotten as context grows.
  - source: Recipe instructions
    mode: modify
    scope:
      - run
    order_notes: Applied when `goose run --recipe <file>` is used.
    notes: Supplies task-specific instructions and prompt templates but does not replace the core system prompt.
agent_prompting:
  supported: true
  definition_surface: Markdown files with YAML frontmatter in ~/.agents/agents/ or <project>/.agents/agents/ (also compatibility paths .goose/agents/, .claude/agents/, ~/.goose/agents/, ~/.claude/agents/)
  inheritance: Delegated custom agents run in separate sessions with the agent file body as their instructions. Subagents use the subagent_system.md template plus task-specific instructions and inherit extensions from the parent session unless restricted.
  isolation: Each delegated agent/subagent runs in its own session/context window; only the final summary returns to the parent.
  limitations: Subagents cannot spawn further subagents, enable/disable extensions, or manage scheduled tasks. Recipes with explicit `extensions` lists must include `summon` for delegation to work.
claudine_delivery:
  append_strategy: inline_flag
  replace_strategy: unsupported
  temp_file_required: false
  argv_limit: Subject to the platform argv size limit; no documented provider-side cap.
  notes: >-
    Goose only supports system-prompt append via `goose run --system <text>`. There is no `--system` flag on `goose session`,
    no `--system-prompt-file` equivalent, and no native full-replacement CLI flag. Claudine should pass composed append content
    directly to `--system` for non-interactive runs and warn/skip for interactive sessions and replace mode.
format_recommendations:
  append_format: markdown
  replace_format: xml_wrapped_markdown
  rationale: >-
    Goose's built-in system.md is Markdown with Jinja2 substitutions, so appended content blends best as plain Markdown with headers and lists.
    For full replacements via a custom system.md template or recipe, XML-wrapped Markdown helps the model distinguish sections such as
    `<instructions>`, `<constraints>`, and `<examples>` because the built-in structure is removed.
recent_changes:
  - date: "2026-07-03"
    version: "v1.41.0"
    change: Global hints loading from `~/.agents/AGENTS.md`.
    impact: User-level AGENTS.md context files are now discovered alongside `.goosehints`.
  - date: "2026-06-25"
    version: "v1.39.0"
    change: ACP session system prompt setter.
    impact: ACP clients can programmatically set the session system prompt; does not add a CLI flag.
  - date: "2026-05-13"
    version: "v1.34.0"
    change: Projects as backend sources with system prompt injection.
    impact: Project metadata can feed into the system prompt assembly.
  - date: "2026-02"
    version: "v1.27.0"
    change: Restored subagent system-prompt behavior.
    impact: Subagents again receive their dedicated system prompt template correctly.
quirks:
  - "`--system` is only available on `goose run`; `goose session` has no equivalent CLI flag."
  - "There is no native `--system-prompt-file` or full `--replace-system-prompt` CLI flag."
  - "Nested `.goosehints` / `AGENTS.md` files remain active for the rest of the session once loaded, even after leaving that directory."
  - "`GOOSE_MOIM_MESSAGE_TEXT` and `GOOSE_MOIM_MESSAGE_FILE` are re-injected every turn and can override softer system-prompt instructions."
  - "A recipe with an explicit `extensions` block must include `summon` (type: platform) or delegation/subagent tools will be unavailable."
  - "Custom `system.md` template changes only take effect in new sessions."
  - "Goose does not publish a CLI command to dump the fully rendered effective system prompt."
  - "Global hints now load from `~/.agents/AGENTS.md` as well as `~/.config/goose/.goosehints`."
gaps:
  - Exact ordering of `.goosehints`, `AGENTS.md`, `--system`, and recipe instructions is not fully documented.
  - No CLI command exports or inspects the effective built-in system prompt as plain text.
  - Whether `--system` content is appended before or after discovered context files is not explicitly documented.
  - Interactive session system-prompt append has no documented native mechanism.
changes:
  - "Corrected earlier research: Goose has no `--system-prompt` or `--system-prompt-file` flags; append is `goose run --system` only."
  - "Updated docs and repo URLs from `block.github.io/goose` and `block/goose` to `goose-docs.ai` and `aaif-goose/goose`."
  - "Added prompt templates, persistent instructions (MOIM), custom agents, and AGENTS.md context from current Goose docs."
  - "Set replace_support to `none` and claudine_delivery.replace_strategy to `unsupported`."
requires_claudine_update: true
reason: >-
  The current ProviderInfo declares interactive append via `SystemPromptDelivery::Custom(GooseRecipe)`, but the wrap layer treats `Custom` as a no-op.
  Goose only supports system-prompt append via `goose run --system <text>`; there is no interactive equivalent and no native replace mechanism.
  Claudine's `SystemPromptSpec` should be updated to reflect non-interactive-only inline-flag append and unsupported replace.
---

## Overview

Goose CLI builds the effective instructions for each session from several layers. The base is the built-in `system.md` Jinja2 template, which defines Goose's role, extension handling, and response guidelines. Users can override this template by placing a custom `system.md` under `~/.config/goose/prompts/`. On top of that base, Goose appends context from `AGENTS.md` and `.goosehints` files, extension instructions, a `--system` flag passed to `goose run`, and per-turn persistent instructions via `GOOSE_MOIM_MESSAGE_TEXT` / `GOOSE_MOIM_MESSAGE_FILE`. Full replacement of the system prompt from the CLI is not supported; the closest mechanisms are custom prompt templates, recipes, or the ACP session system-prompt setter.

## CLI Parameters

Only `goose run` exposes a flag that directly manipulates the system prompt for a single invocation.

| Flag | Mode | Effect |
| :--- | :--- | :--- |
| `--system "<text>"` | Append | Adds the supplied text as additional system instructions for the current `goose run`. |
| `--recipe <name_or_path>` | Modify | Loads a YAML recipe that supplies its own `instructions` and `prompt` for the run. |
| `--render-recipe` | Inspect | Prints the rendered recipe instead of executing it. |
| `--explain` | Inspect | Shows a recipe's title, description, and parameters. |

`goose session` has no `--system` flag. There are no `--system-prompt`, `--system-prompt-file`, or `--append-system-prompt` equivalents.

## Configuration and Discovery

### Prompt templates

Goose's built-in system prompt is a Jinja2 template stored in the application. Users can override it by creating files under:

- **macOS/Linux:** `~/.config/goose/prompts/`
- **Windows:** `%APPDATA%\Block\goose\config\prompts\`

Relevant templates:

| Template | Effect |
| :--- | :--- |
| `system.md` | Replaces the main session system prompt. |
| `subagent_system.md` | Replaces the system prompt used for subagents. |
| `plan.md`, `compaction.md`, `recipe.md`, etc. | Override other built-in behaviors. |

Changes take effect in new sessions. Deleting the custom file restores the default.

### Hints and context files

Goose discovers context files at session start and when accessing nested directories:

- `.goosehints` (global: `~/.config/goose/.goosehints`; project-local: `./.goosehints`)
- `AGENTS.md` (global: `~/.agents/AGENTS.md`; project-local: `./AGENTS.md`)

By default, Goose looks for `AGENTS.md` then `.goosehints`. The default can be changed with the `CONTEXT_FILE_NAMES` environment variable. Nested files are loaded as Goose reads files in those directories and remain active for the rest of the session.

### Recipes

Recipes are YAML files in `.goose/recipes/` or `~/.config/goose/recipes/` that package instructions, extensions, parameters, and settings. They are launched with `goose run --recipe <file>`. A recipe does not replace the core system prompt, but it defines the run's task identity and can supply its own `instructions` and `prompt`.

### Persistent instructions (MOIM)

The `GOOSE_MOIM_MESSAGE_TEXT` and `GOOSE_MOIM_MESSAGE_FILE` environment variables inject text into Goose's working memory every turn. Unlike `.goosehints`, which are loaded at session start, persistent instructions are re-read fresh each turn, making them suitable for critical guardrails.

## Prompt Layers and Precedence

```mermaid
graph TD
    A[Built-in system.md template] --> B{Custom system.md exists?}
    B -- yes --> C[Custom ~/.config/goose/prompts/system.md]
    B -- no --> D[Built-in system.md template]
    C --> E[moim_system_prompt_block if defined]
    D --> E
    E --> F[Extension instructions]
    F --> G[AGENTS.md / .goosehints hierarchy]
    G --> H[--system flag on goose run]
    H --> I[GOOSE_MOIM_MESSAGE_TEXT / FILE every turn]
    I --> J[Recipe instructions when --recipe is used]
    J --> K[User prompt]
```

Notes on precedence:

- A custom `system.md` replaces the built-in template entirely.
- `.goosehints` and `AGENTS.md` append project context, not a hard system-prompt layer.
- `--system` applies only to a single `goose run` invocation.
- MOIM persistent instructions are re-injected every turn and can override softer instructions.
- Recipe instructions define the run's task but do not replace the core system prompt.

## Agents and Subagents

Goose supports custom agents as Markdown files with YAML frontmatter. The file body becomes the agent's instructions.

| Surface | Location |
| :--- | :--- |
| Global agents | `~/.agents/agents/*.md` |
| Project agents | `<project>/.agents/agents/*.md` |
| Compatibility paths | `.goose/agents/`, `.claude/agents/`, `~/.goose/agents/`, `~/.claude/agents/` |

Frontmatter supports `name` (required), `description`, and `model`. Agents can be invoked by mention (`@code-reviewer`) or delegated to. Delegated agents run in isolated sessions; loading an agent adds its instructions to the current conversation without creating a separate session.

Subagents are spawned through the `summon` platform extension. They run in isolated context windows with their own `subagent_system.md` template plus task instructions. By default, subagents inherit extensions from the parent session, but access can be restricted. Subagents cannot spawn further subagents or manage extensions/schedules.

## Format Recommendations

| Goal | Recommended format | Rationale |
| :--- | :--- | :--- |
| Append (`--system`, `.goosehints`) | Pure Markdown | Headers, lists, and short paragraphs blend with Goose's Markdown-based system template. |
| Replace (custom `system.md` or recipe) | XML-wrapped Markdown | XML tags such as `<instructions>`, `<constraints>`, and `<examples>` help the model distinguish sections when the built-in structure is removed. |

For replacements via custom `system.md`, the file should reintroduce any extension handling, safety rules, and response guidelines the task still needs, because the built-in template is removed entirely.

## Recent Changes

- **v1.41.0 (2026-07-03)**: Global hints now load from `~/.agents/AGENTS.md` in addition to `~/.config/goose/.goosehints`.
- **v1.39.0 (2026-06-25)**: Added an ACP session system-prompt setter for programmatic clients.
- **v1.34.0–v1.38.0 (2026-05–2026-06)**: Projects can act as backend sources with system-prompt injection.
- **v1.27.0 (2026-02)**: Restored correct subagent system-prompt behavior.
- **v1.41.0 (2026-07-03)**: Added `--edit` session flag to edit conversation before forking.

## Quirks and Workarounds

- `--system` only works on `goose run`. For interactive sessions, use `.goosehints`, `AGENTS.md`, or load a custom agent instead.
- There is no CLI flag to replace the entire system prompt; use a custom `~/.config/goose/prompts/system.md` or a recipe.
- Nested `.goosehints` / `AGENTS.md` files are sticky: once loaded for a directory, they remain active for the session.
- MOIM persistent instructions are re-injected every turn and are stronger than one-shot system-prompt instructions.
- Recipes with an explicit `extensions` block must list `summon` (type: platform) or delegation tools will not be available.
- Custom prompt template changes only apply to new sessions; restart after editing `system.md`.
- Goose has no built-in command to export the fully rendered effective system prompt.

## Claudine Delivery Notes

- **Append**: For non-interactive `goose run`, pass the composed content to `--system <text>`. This is a per-invocation change that does not mutate user config files.
- **Replace**: Goose has no native CLI replace mechanism. Claudine should treat replace as unsupported for Goose.
- **Interactive**: `goose session` has no `--system` flag. Claudine should warn and skip system-prompt append in interactive mode.
- **No temp file**: Because Goose accepts `--system` as inline text, no temp file is required for append. Keep content within platform argv limits.

## Changelog

- Corrected earlier research: Goose append is `goose run --system <text>` only; there are no `--system-prompt` or `--system-prompt-file` flags.
- Updated documentation and repository URLs from `block.github.io/goose` and `block/goose` to `goose-docs.ai` and `aaif-goose/goose`.
- Added prompt templates, persistent instructions (MOIM), custom agents, AGENTS.md context, and recipe-based instructions from current Goose docs.
- Set `replace_support` to `none` and `claudine_delivery.replace_strategy` to `unsupported`.

## Sources

- [Goose CLI commands](https://goose-docs.ai/docs/guides/goose-cli-commands/)
- [Customizing prompt templates](https://goose-docs.ai/docs/guides/context-engineering/prompt-templates/)
- [Using goosehints](https://goose-docs.ai/docs/guides/context-engineering/using-goosehints/)
- [Persistent instructions](https://goose-docs.ai/docs/guides/context-engineering/using-persistent-instructions/)
- [Subagents](https://goose-docs.ai/docs/guides/context-engineering/subagents/)
- [Custom agents](https://goose-docs.ai/docs/guides/context-engineering/custom-agents/)
- [Recipe reference](https://goose-docs.ai/docs/guides/recipes/recipe-reference/)
- [Configuration files](https://goose-docs.ai/docs/guides/config-files/)
- [Environment variables](https://goose-docs.ai/docs/guides/environment-variables/)
- [Goose moves to AAIF](https://goose-docs.ai/blog/2026/04/07/goose-moves-to-aaif/)
- [Goose releases on GitHub](https://github.com/aaif-goose/goose/releases)
- [Built-in system.md template source](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/prompts/system.md)
