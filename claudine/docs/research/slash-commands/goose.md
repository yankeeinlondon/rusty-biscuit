---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7

homepage: https://block.github.io/goose/
docs: https://goose-docs.ai/docs/
slash_docs: https://goose-docs.ai/docs/guides/context-engineering/slash-commands/

support: first_class

locations:
  - os: macos
    scope: user
    path: ~/.config/goose/config.yaml
    notes: |
      Primary user configuration file. The `slash_commands:` key maps command names to recipe file paths. Discovered at session start. On macOS `~/.config/goose` resolves to `$HOME/.config/goose`.
  - os: linux
    scope: user
    path: ~/.config/goose/config.yaml
    notes: Same behavior as macOS; ~/.config/goose resolves to `$HOME/.config/goose`.
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\config\\config.yaml"
    notes: Windows user configuration file. `slash_commands:` lives under this file.
  - os: macos
    scope: user
    path: ~/.config/goose/recipes/<name>.yaml
    notes: Default global recipe storage. Recipes referenced by `slash_commands[*].recipe_path` may live here.
  - os: linux
    scope: user
    path: ~/.config/goose/recipes/<name>.yaml
    notes: Same as macOS.
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\config\\recipes\\<name>.yaml"
    notes: Same as macOS.
  - os: macos
    scope: repo
    path: .goose/recipes/<name>.yaml
    notes: Project-level recipe storage. Recipes may be referenced by user-scoped slash commands; there is no documented project-scoped `config.yaml` for slash commands.
  - os: linux
    scope: repo
    path: .goose/recipes/<name>.yaml
    notes: Same as macOS.
  - os: windows
    scope: repo
    path: ".goose\\recipes\\<name>.yaml"
    notes: Same as macOS.
  - os: macos
    scope: other
    path: "$PWD/*.yaml or paths in GOOSE_RECIPE_PATH"
    notes: Recipes can also reside in the current working directory or in colon-separated directories listed in the GOOSE_RECIPE_PATH environment variable.
  - os: linux
    scope: other
    path: "$PWD/*.yaml or paths in GOOSE_RECIPE_PATH"
    notes: Same as macOS.
  - os: windows
    scope: other
    path: "$PWD\\*.yaml or paths in GOOSE_RECIPE_PATH"
    notes: Same as macOS; GOOSE_RECIPE_PATH uses semicolon separators on Windows.

format:
  file_names:
    - config.yaml
    - "*.yaml"
    - "*.json"
  frontmatter: false
  required_fields:
    - slash_commands.command
    - slash_commands.recipe_path
  optional_fields: []
  argument_syntax: |
    Slash commands pass exactly one raw parameter from the message text to the recipe. Inside the recipe file the parameter is declared in the `parameters:` list with `key`, `input_type`, `requirement`, and `description`, and substituted via Jinja-style `{{ parameter_name }}` placeholders in `instructions`, `prompt`, and `activities`. Any additional recipe parameters must declare `default` values.
  body_format: yaml
  notes: |
    A slash command is not a standalone file; it is a two-item entry in the `slash_commands:` list of `config.yaml`. The `command` string (without a leading `/`) names the slash command, and `recipe_path` points to a YAML or JSON recipe file. Recipe files follow the Goose recipe schema: `title`, `description`, and at least one of `instructions` or `prompt` are required, with optional `parameters`, `extensions`, `settings`, `sub_recipes`, `retry`, and `response`. The `recipe_dir` built-in parameter resolves to the directory containing the recipe file.

command_model:
  invocation: |
    In an interactive session, type `/` followed by the command name at the start of a message, e.g. `/run-tests` or `/translator where is the library`. Built-in slash commands (`/help`, `/recipe`, `/compact`, `/skills`, etc.) share the same `/` namespace and take precedence over custom commands.
  namespacing: |
    Custom slash commands share the `/` namespace with built-in session commands. Command names are case-insensitive and must be unique with no spaces. If a custom command name conflicts with a built-in command, the built-in command wins. There is no documented directory-based or plugin namespacing for slash commands; each command is a flat name in `config.yaml`.
  arguments: |
    Exactly one parameter may be passed after the command name. Quotation marks are optional for multi-word values. The provider sends the raw text after the command name to the recipe as the value of the single declared parameter. All other recipe parameters must be optional and provide `default` values. If the recipe file is missing or invalid, the message is treated as ordinary user text.
  output_handling: |
    When a slash command runs, Goose loads the referenced recipe, substitutes the provided parameter into `{{ parameter_name }}` placeholders, and sends the recipe's `instructions` and `prompt` fields to the model as context. The content is loaded into the conversation but not displayed in the chat transcript. The model then responds using the recipe's context and instructions.
  disabled_mechanism: |
    Remove the entry from `slash_commands:` in `config.yaml` and restart the session. There is no per-command disable flag or in-session reload command documented for slash commands. Recipes themselves can be removed or renamed to invalidate the `recipe_path`.
  notes: |
    Custom slash commands require a valid recipe file at the configured `recipe_path`. Built-in commands cannot be overridden. The provider does not perform workspace trust gating specifically for slash commands, but recipe execution obeys the normal Goose tool-approval model (GOOSE_MODE) and any extension allowlist.

portability:
  portable: false
  non_portable_assets:
    - "Goose recipe YAML/JSON schema (title, description, instructions, prompt, parameters, extensions, settings, sub_recipes, retry, response)"
    - "Jinja-style {{ parameter_name }} placeholders"
    - "Recipe extension definitions (MCP servers, inline_python, etc.)"
    - "Goose-specific settings (goose_provider, goose_model, temperature, max_turns)"
    - "Goose retry and response.json_schema blocks"
    - "Indirection through config.yaml slash_commands list"
  rewrite_needed: true
  notes: |
    A Goose slash command is a config entry plus a recipe file, so it cannot be linked as-is to another provider. Claudine would need to extract the recipe's `instructions`/`prompt` prose, map the single `{{ parameter_name }}` placeholder to the target provider's argument grammar, and convert the recipe file to the target's command format (often Markdown with YAML frontmatter). Recipe-specific mechanics such as extensions, sub_recipes, retry checks, and response schemas are Goose-specific and must be stripped or rewritten.

cli_params:
  - flag: goose run --recipe <file> [--params KEY=VALUE ...]
    description: Load and run a recipe directly. This is the non-interactive equivalent of a slash command.
    example: goose run --recipe deploy.yaml --params env=production
  - flag: goose run --interactive
    description: Keep the session open after running initial input so slash commands can be typed.
    example: goose run --recipe daily.yaml --interactive
  - flag: goose recipe list [--format json] [--verbose]
    description: List available recipes from local directories and configured GitHub repositories.
    example: goose recipe list --verbose
  - flag: goose recipe validate <file>
    description: Validate a recipe file against the Goose recipe schema.
    example: goose recipe validate run-tests.yaml
  - flag: /skills [name ...]
    description: In-session command to list or load Agent Skills. Not a slash-command management command, but the closest in-session skill surface.
    example: /skills code-review

env_vars:
  - name: GOOSE_RECIPE_PATH
    effect: |
      Colon-separated list (semicolon on Windows) of additional directories to search for recipes. Affects which recipe files can be referenced by slash_commands.recipe_path.
  - name: GOOSE_RECIPE_GITHUB_REPO
    effect: |
      Configures a GitHub repository (format "owner/repo") to search for recipes. Requires the `gh` CLI to be installed and authenticated.
  - name: GOOSE_PATH_ROOT
    effect: |
      Overrides the root directory for all Goose data, config, and state files, which relocates `config/` and therefore the `config.yaml` containing `slash_commands:`.
  - name: GOOSE_MODE
    effect: |
      Tool execution mode (auto, approve, chat, smart_approve). Affects whether a recipe's tool-using steps run without user approval.
  - name: GOOSE_SHELL
    effect: |
      Overrides the shell used for Developer extension commands and recipe retry/on_failure shell checks.

changes: []

requires_claudine_update: true
reason: |
  Claudine's command linker should model Goose CLI slash commands as config-level indirection: a `slash_commands:` list in `~/.config/goose/config.yaml` (and the Windows equivalent) where each entry maps a flat command name to a recipe file path. Linking must also account for recipe files stored in `~/.config/goose/recipes/`, `.goose/recipes/`, the working directory, and `GOOSE_RECIPE_PATH`. Because the command artifact is a YAML recipe with Jinja parameters and Goose-specific extensions/settings, these assets should be classified as non-portable and requiring rewrite.
---

# Goose CLI Slash Commands

## Overview

Goose CLI calls its user-defined reusable commands **custom slash commands**. A custom slash command is a named shortcut configured in `config.yaml` that points to a **recipe** file. Recipes are Goose's reusable workflow format — YAML or JSON files that package instructions, prompts, parameters, extensions, and settings.

Support is **first class**: users can define arbitrary command names in their config, invoke them with `/` in an interactive session, and pass one parameter. Built-in session commands such as `/help`, `/recipe`, `/compact`, and `/skills` share the same `/` namespace.

This topic covers the slash-command surface. The related **Agent Skills** system (directory-based `SKILL.md` resources) and **goosehints** / `AGENTS.md` persistent instruction files are separate reuse mechanisms; see the Goose skills research doc for those.

## Locations

### Command definitions

Custom slash commands are defined in the user configuration file, not as standalone files. There is no documented project-level `config.yaml` for slash commands, and there is no extension/plugin command directory for them.

| OS | Scope | Path | Notes |
| :- | :---- | :--- | :---- |
| macOS / Linux | User | `~/.config/goose/config.yaml` | The `slash_commands:` list lives here. |
| Windows | User | `%APPDATA%\Block\goose\config\config.yaml` | Same behavior as macOS / Linux. |

### Recipe files referenced by commands

The `recipe_path` in each slash command entry points to a recipe file. Recipes are discovered from several locations:

| OS | Scope | Path | Notes |
| :- | :---- | :--- | :---- |
| macOS / Linux | User | `~/.config/goose/recipes/<name>.yaml` | Default global recipe directory. |
| Windows | User | `%APPDATA%\Block\goose\config\recipes\<name>.yaml` | Windows equivalent. |
| macOS / Linux | Repo | `.goose/recipes/<name>.yaml` | Project-specific recipes. |
| Windows | Repo | `.goose\recipes\<name>.yaml` | Windows equivalent. |
| All | Other | Current directory or `GOOSE_RECIPE_PATH` | Recipes can also be loaded from `$PWD` or directories listed in the `GOOSE_RECIPE_PATH` environment variable. |

### Local observations

On this machine, `~/.config/goose/` does not exist, so there are no local slash commands or recipes to inspect. `~/.agents/skills/` exists with a single skill directory (`find-skills`), but that belongs to the Agent Skills system, not slash commands.

## File Format

### Config entry

A slash command is a two-field entry under the `slash_commands:` key in `config.yaml`:

```yaml
slash_commands:
  - command: "run-tests"
    recipe_path: "/path/to/recipe.yaml"
  - command: "daily-report"
    recipe_path: "/Users/me/.config/goose/recipes/report.yaml"
```

| Field | Required | Description |
| :---- | :------- | :---------- |
| `command` | Yes | The command name without the leading `/`. Must be unique and contain no spaces. |
| `recipe_path` | Yes | Absolute or relative path to a YAML or JSON recipe file. |

### Recipe file

Recipes follow the Goose recipe schema. They are usually YAML. Required fields are `title`, `description`, and at least one of `instructions` or `prompt`.

Example recipe that works with a slash command:

```yaml
version: "1.0.0"
title: "Run Tests"
description: "Run the project test suite with an optional filter."
parameters:
  - key: filter
    input_type: string
    requirement: optional
    default: ""
    description: "Optional test filter string."
instructions: "Run the tests for this project."
prompt: "Please run the test suite{{ ' matching: ' + filter if filter else '' }}. Report the results."
```

### Parameter substitution

Recipe fields support Jinja-style templates:

| Syntax | Meaning |
| :----- | :------ |
| `{{ parameter_name }}` | Substitutes the value of a declared parameter. |
| `{{ recipe_dir }}` | Built-in parameter resolving to the recipe file's directory. |

Slash commands themselves have no placeholder grammar; they simply pass the raw text after the command name into the recipe as the value of the parameter declared in `parameters`.

## Invocation Model

### How commands are invoked

In an interactive Goose session, type `/` followed by the command name at the start of a message. Examples:

```text
/run-tests
/translator where is the library
/daily-report --since yesterday
```

### Namespacing and conflicts

Custom slash commands share the `/` namespace with built-in session commands. The rules are:

- Command names are case-insensitive (`/Bug` and `/bug` are the same command).
- Command names must be unique and contain no spaces.
- If a custom command name conflicts with a built-in command, the built-in command wins.
- There is no directory-based or plugin-based namespace for slash commands; each command is a flat name in `config.yaml`.

### Arguments

Slash commands accept **only one parameter**. Any additional values in the recipe must be modeled as `parameters` with `default` values. The raw text after the command name is passed to the recipe as the value of the declared parameter. Quotation marks are optional for multi-word values.

For example, with this config and recipe:

```yaml
slash_commands:
  - command: translator
    recipe_path: ~/.config/goose/recipes/translator.yaml
```

```yaml
title: "Translator"
description: "Translate text into English."
parameters:
  - key: text
    input_type: string
    requirement: required
    description: "Text to translate."
instructions: "Translate the provided text into English."
prompt: "Translate this into English: {{ text }}"
```

Typing `/translator where is the library` substitutes `where is the library` into `{{ text }}`.

### Output handling

When a slash command runs, Goose loads the referenced recipe, performs parameter substitution, and sends the recipe's `instructions` and `prompt` to the model as context. The recipe content is loaded into the active conversation but is not displayed in the chat transcript. The model responds using the recipe's instructions and any extensions or settings it declares.

### Disable mechanisms

- Remove the entry from `slash_commands:` in `config.yaml`.
- Rename or remove the recipe file referenced by `recipe_path`.
- There is no per-command disable flag or in-session reload command documented for slash commands.

## Portability

Goose custom slash commands are **not portable** to other agentic CLIs without rewriting.

What can be reused with transformation:

- The prose instructions and prompt inside the recipe file.

What is provider-specific and must be rewritten or removed:

- The `slash_commands:` indirection in `config.yaml`.
- The Goose recipe schema (YAML/JSON with `title`, `description`, `instructions`, `prompt`, `parameters`, `extensions`, `settings`, `sub_recipes`, `retry`, `response`).
- Jinja-style `{{ parameter_name }}` placeholders.
- Recipe `extensions` blocks, including MCP servers and `inline_python`.
- Recipe `settings` such as `goose_provider`, `goose_model`, `temperature`, and `max_turns`.
- Recipe `retry` checks and `response.json_schema` structured output.
- The single-parameter limitation and case-insensitive command names.

Because a slash command is a config entry pointing to a recipe, Claudine cannot symlink it directly to another provider. The recipe body must be extracted and converted to the target provider's command format, and the parameter placeholder must be mapped to the target's argument grammar.

## Claudine Linking Notes

- Classify Goose CLI as **first-class slash-command support** with **non-portable** command assets.
- Do not symlink `config.yaml` or recipe files directly to another provider. Instead, parse the `slash_commands:` list and the referenced recipe files.
- For each command, extract the recipe's `instructions` and `prompt`, map the single `{{ parameter_name }}` placeholder to the target provider's argument token, and generate a target-native command file.
- Strip Goose-specific recipe fields (`extensions`, `settings`, `sub_recipes`, `retry`, `response`) unless the target provider has direct equivalents.
- Recognize recipe storage locations: `~/.config/goose/recipes/`, `.goose/recipes/`, working directory, and paths from `GOOSE_RECIPE_PATH`.
- Note the single-parameter limitation when translating commands that expect multiple arguments.

## Sources

- [Goose homepage](https://block.github.io/goose/)
- [Goose documentation](https://goose-docs.ai/docs/)
- [Custom slash commands](https://goose-docs.ai/docs/guides/context-engineering/slash-commands/)
- [Recipes overview](https://goose-docs.ai/docs/guides/recipes/)
- [Recipe reference](https://goose-docs.ai/docs/guides/recipes/recipe-reference)
- [Saving recipes](https://goose-docs.ai/docs/guides/recipes/storing-recipes)
- [Configuration files](https://goose-docs.ai/docs/guides/config-files/)
- [Environment variables](https://goose-docs.ai/docs/guides/environment-variables/)
- [CLI commands](https://goose-docs.ai/docs/guides/goose-cli-commands/)
- [Goose GitHub repository](https://github.com/aaif-goose/goose)
