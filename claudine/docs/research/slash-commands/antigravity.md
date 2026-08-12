---
$schema: ./_schema.yaml
created: 2026-07-08
last_updated: 2026-07-08
agent: codex
model: default
homepage: https://antigravity.google/product/antigravity-cli
docs: https://antigravity.google/docs/cli/overview
slash_docs: https://antigravity.google/docs/cli/plugins
support: first_class
locations:
  - os: macos
    scope: user
    path: "~/.gemini/config/skills/<skill-folder>/SKILL.md"
    notes: "Documented global scope shared by Antigravity products; no user-created skills were present in the local macOS inspection."
  - os: linux
    scope: user
    path: "~/.gemini/config/skills/<skill-folder>/SKILL.md"
    notes: "Documented Unix-style global scope shared by Antigravity products."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\config\\skills\\<skill-folder>\\SKILL.md"
    notes: "Windows template for the documented home-relative global scope; official docs use Unix-style examples."
  - os: macos
    scope: user
    path: "~/.gemini/antigravity-cli/skills/<skill-folder>/SKILL.md"
    notes: "Documented Antigravity CLI-specific global scope in codelab guidance; local install had no custom files here."
  - os: linux
    scope: user
    path: "~/.gemini/antigravity-cli/skills/<skill-folder>/SKILL.md"
    notes: "CLI-specific global scope for Unix-like hosts."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\antigravity-cli\\skills\\<skill-folder>\\SKILL.md"
    notes: "Windows template for the CLI-specific global scope."
  - os: macos
    scope: repo
    path: "<project-root>/.agents/skills/<skill-folder>/SKILL.md"
    notes: "Documented project/workspace scope; local workspace did not contain .agents or .agent skills."
  - os: linux
    scope: repo
    path: "<project-root>/.agents/skills/<skill-folder>/SKILL.md"
    notes: "Documented project/workspace scope."
  - os: windows
    scope: repo
    path: "<project-root>\\.agents\\skills\\<skill-folder>\\SKILL.md"
    notes: "Windows template for the documented project/workspace scope."
  - os: macos
    scope: repo
    path: "<project-root>/.agent/skills/<skill-folder>/SKILL.md"
    notes: "Also documented by codelab and local built-in guide as an accepted project customization root."
  - os: linux
    scope: repo
    path: "<project-root>/.agent/skills/<skill-folder>/SKILL.md"
    notes: "Alternative project customization root."
  - os: windows
    scope: repo
    path: "<project-root>\\.agent\\skills\\<skill-folder>\\SKILL.md"
    notes: "Windows template for the alternative project customization root."
  - os: macos
    scope: system
    path: "~/.gemini/antigravity-cli/builtin/skills/<skill-folder>/SKILL.md"
    notes: "Observed locally for built-in skill-derived commands, including antigravity-guide and agy-customizations."
  - os: linux
    scope: system
    path: "~/.gemini/antigravity-cli/builtin/skills/<skill-folder>/SKILL.md"
    notes: "Built-in CLI skill location inferred from local layout and changelog references."
  - os: windows
    scope: system
    path: "%USERPROFILE%\\.gemini\\antigravity-cli\\builtin\\skills\\<skill-folder>\\SKILL.md"
    notes: "Windows template for the built-in CLI skill location."
  - os: macos
    scope: extension
    path: "~/.gemini/config/plugins/<plugin-name>/skills/<skill-folder>/SKILL.md"
    notes: "Plugins can package skills; installed plugins are imported to the shared configuration directory."
  - os: linux
    scope: extension
    path: "~/.gemini/config/plugins/<plugin-name>/skills/<skill-folder>/SKILL.md"
    notes: "Plugin-packaged skill location."
  - os: windows
    scope: extension
    path: "%USERPROFILE%\\.gemini\\config\\plugins\\<plugin-name>\\skills\\<skill-folder>\\SKILL.md"
    notes: "Windows template for plugin-packaged skills."
format:
  file_names:
    - "skills/<skill-folder>/SKILL.md"
    - "plugins/<plugin-name>/skills/<skill-folder>/SKILL.md"
    - "skills.json"
    - "plugins.json"
  frontmatter: true
  required_fields:
    - name
    - description
  optional_fields: []
  argument_syntax: "No documented placeholder syntax for direct slash-command arguments. The visible command is generated from the skill name; user text after invocation remains part of the user prompt and is not documented as substituted into SKILL.md."
  body_format: markdown
  notes: "A command-shaped artifact is a skill directory, not a standalone command file. SKILL.md uses YAML frontmatter followed by Markdown instructions. Optional scripts/, resources/, references/, and examples/ directories can be referenced by relative path. skills.json and plugins.json can register additional scan roots with entries, inherits, include_only, and exclude filters."
command_model:
  invocation: "Open an interactive agy TUI session and type /<skill-name>, for example /refactor-ui. Type /skills to inspect loaded skills. Built-ins such as /help, /settings, /permissions, /mcp, /agents, /hooks, /plan, and /diff use the same leading slash surface."
  namespacing: "Skill-derived commands share the slash-command surface with built-ins. The skill command name is derived from the SKILL.md name field. Local built-in guidance says higher-priority customizations override lower-priority customizations on naming conflicts; it does not document a user-skill override for built-in slash commands. Plugin skills may be namespaced when necessary to prevent collisions."
  arguments: "No formal command argument grammar is documented for skill-derived slash commands. Autocomplete supports fuzzy and partial substring matching. Changelog evidence shows slash-command history, alias completion for built-ins, and correct submission of autocompleted skill commands, but no quoting or positional placeholder behavior for skills."
  output_handling: "Invoking a skill-derived command activates the skill. The model initially sees skill names and descriptions, then loads the full SKILL.md and linked resources only when activated. Markdown instructions enter the agent context; scripts are not executed by the slash command itself, but the agent may run referenced helper scripts later through normal tool execution and permission/sandbox policy."
  disabled_mechanism: "Remove or rename the skill directory, filter it through skills.json include_only/exclude, disable the containing plugin with agy plugin disable <name>, or remove the plugin. No per-SKILL.md disabled field is documented."
  notes: "Workspace customization discovery walks from the current directory toward the repository root and recognizes .agents, .agent, _agents, and _agent roots in local built-in guidance. Local install used HOME=/Users/ken/.claudine and contained built-in skills under ~/.gemini/antigravity-cli/builtin/skills but no ~/.antigravity directory and no custom skill command files."
portability:
  portable: false
  non_portable_assets:
    - "SKILL.md skill package layout"
    - "YAML frontmatter name and description metadata"
    - "Skill-derived slash-command invocation"
    - "Progressive disclosure semantics"
    - "skills.json and plugins.json discovery filters"
    - "Plugin packaging and optional namespacing"
    - "Relative scripts/resources/references assets"
  rewrite_needed: true
  notes: "The Markdown instruction body can often be transformed into another provider's command prompt, but Claudine must synthesize or preserve a command name from the skill name, map description metadata to the target provider's command metadata, decide how to handle lack of argument placeholders, and either copy or reject referenced scripts/resources. Standalone Claude/Gemini-style command Markdown files should not be linked into Antigravity as-is; they need conversion to skills/<name>/SKILL.md."
cli_params:
  - flag: "agy plugin install <target>"
    description: "Installs a plugin that can expose skills, which become skill-derived slash commands."
    example: "agy plugin install owner/repo/path"
  - flag: "agy plugin import [source]"
    description: "Imports plugins from supported sources such as gemini or claude into the shared configuration area."
    example: "agy plugin import gemini"
  - flag: "agy plugin list"
    description: "Lists imported plugins; local inspection returned no imported plugins."
    example: "agy plugin list"
  - flag: "agy plugin enable <name>"
    description: "Enables an installed plugin and its packaged skills."
    example: "agy plugin enable team-developer-kit"
  - flag: "agy plugin disable <name>"
    description: "Disables an installed plugin, hiding packaged skills from runtime discovery."
    example: "agy plugin disable team-developer-kit"
  - flag: "agy plugin uninstall <name>"
    description: "Removes an installed plugin and its packaged skills."
    example: "agy plugin uninstall team-developer-kit"
  - flag: "agy plugin validate [path]"
    description: "Validates a plugin manifest and package layout before installation or sharing."
    example: "agy plugin validate .agents/plugins/team-developer-kit"
  - flag: "--add-dir"
    description: "Adds a directory to the active workspace; changelog notes custom skills and system slash commands reload on conversation switch or /add-dir."
    example: "agy --add-dir ../shared-tools"
  - flag: "--project"
    description: "Selects a project for the session; project-specific configuration has precedence over global settings."
    example: "agy --project default-cli-project"
  - flag: "--new-project"
    description: "Creates a new project for the session, affecting project-scoped configuration."
    example: "agy --new-project"
  - flag: "--sandbox"
    description: "Runs with terminal restrictions enabled; referenced scripts still go through normal tool execution and sandbox policy."
    example: "agy --sandbox"
  - flag: "--dangerously-skip-permissions"
    description: "Auto-approves tool permission requests. This affects scripts or tools a skill instructs the agent to run, not skill discovery itself."
    example: "agy --dangerously-skip-permissions"
env_vars: []
changes: []
requires_claudine_update: true
reason: "Antigravity should be modeled as skill-derived slash-command support, not direct Markdown command-file support. Claudine command linking needs a provider-specific rewrite path from command files to skills/<name>/SKILL.md and should avoid simple symlinks into a commands directory."
---

# Antigravity Skill-Derived Slash Commands

## Overview

Antigravity CLI does support user-defined, command-shaped reusable entries, but the documented surface is **Agent Skills**, not a separate custom slash-command directory. Official Antigravity CLI docs describe plugins and skills as the extensibility model, and the codelab states that registered skills automatically become slash commands inside the TUI, for example `/refactor-ui`.

The practical command artifact is therefore a `SKILL.md` file inside a skill directory. The slash command is generated from the skill metadata, and the command activation loads the skill instructions into the active agent context. This is first-class support for user-defined commands, but not portable as a direct command-file link.

Built-in slash commands and skill-derived slash commands share the leading `/` interaction surface. Built-ins such as `/help`, `/skills`, `/settings`, `/permissions`, `/mcp`, `/agents`, `/hooks`, `/plan`, and `/diff` are not user-defined artifacts. User commands are the skills exposed through the same surface.

Local inspection on 2026-07-08 found no `~/.antigravity` directory. The installed `agy` binary was present at `/Users/ken/.local/bin/agy`; with `HOME=/Users/ken/.claudine`, Antigravity data existed under `/Users/ken/.claudine/.gemini/antigravity-cli` and `/Users/ken/.claudine/.gemini/config`. Built-in skills existed under `~/.gemini/antigravity-cli/builtin/skills`, including `antigravity_guide` and `agy-customizations`. No user-created custom skills were present under `~/.gemini/config/skills`, `~/.gemini/antigravity-cli/skills`, `.agents/skills`, or `.agent/skills`.

## Locations

| Scope | macOS/Linux Template | Windows Template | Notes |
| --- | --- | --- | --- |
| Shared user skills | `~/.gemini/config/skills/<skill-folder>/SKILL.md` | `%USERPROFILE%\.gemini\config\skills\<skill-folder>\SKILL.md` | Documented as a global scope shared across Antigravity products. |
| CLI user skills | `~/.gemini/antigravity-cli/skills/<skill-folder>/SKILL.md` | `%USERPROFILE%\.gemini\antigravity-cli\skills\<skill-folder>\SKILL.md` | Codelab guidance calls this the Antigravity CLI global scope when `~/.agents/skills` output is not visible to the CLI. |
| Project skills | `<project-root>/.agents/skills/<skill-folder>/SKILL.md` | `<project-root>\.agents\skills\<skill-folder>\SKILL.md` | Documented project/workspace scope. Local built-in guidance also recognizes `.agent`, `_agents`, and `_agent` customization roots. |
| Alternative project skills | `<project-root>/.agent/skills/<skill-folder>/SKILL.md` | `<project-root>\.agent\skills\<skill-folder>\SKILL.md` | Mentioned in codelab guidance and local built-in customization documentation. |
| Built-in CLI skills | `~/.gemini/antigravity-cli/builtin/skills/<skill-folder>/SKILL.md` | `%USERPROFILE%\.gemini\antigravity-cli\builtin\skills\<skill-folder>\SKILL.md` | Observed locally. These are system-provided commands, not user-defined support by themselves. |
| Plugin skills | `~/.gemini/config/plugins/<plugin-name>/skills/<skill-folder>/SKILL.md` | `%USERPROFILE%\.gemini\config\plugins\<plugin-name>\skills\<skill-folder>\SKILL.md` | Plugins can package skills, rules, hooks, and MCP configuration. |

`skills.json` and `plugins.json` can explicitly register non-standard locations. In local built-in guidance, each file supports `entries` and `inherits`; each entry has a required `path` plus optional `include_only` and `exclude` regex filters. Paths may be absolute, `~/` home-relative, or workspace-relative.

## File Format

A skill is a directory under a `skills/` folder. The required file is `SKILL.md`.

```text
skills/<skill-name>/
├── SKILL.md
├── scripts/
├── examples/
├── resources/
└── references/
```

`SKILL.md` starts with YAML frontmatter. `name` and `description` are required. The body is Markdown instructions. Optional sibling directories are assets the agent can read or run later; a slash-command invocation does not directly execute them.

```markdown
---
name: git-commit-formatter
description: Formats git commit messages according to Conventional Commits specification. Use this when the user asks to commit changes or write a commit message.
---

# Git Commit Formatter Skill

When writing a git commit message, follow the Conventional Commits specification.

## Instructions

1. Inspect the staged changes.
2. Choose an allowed type.
3. Write an imperative, concise subject.
```

There is no documented Antigravity-specific placeholder such as `$ARGUMENTS`, `$1`, or `{{args}}` for skill-derived commands. The user can type natural language after the slash command, but current sources do not define parsing, quoting, default values, validation, or substitution into `SKILL.md`.

The local built-in `agy-customizations` skill describes progressive disclosure: only names and descriptions are injected by default, and the full skill content is loaded when the model or user activates it. Its `SKILL.md` also shows that skill names are not necessarily identical to folder names: the folder `antigravity_guide` contained `name: antigravity-guide`.

## Invocation Model

In an interactive Antigravity CLI session, the user invokes a skill-derived command by typing `/` plus the skill name, such as:

```text
/refactor-ui
```

The `/skills` command lists loaded skills. The TUI also autocompletes slash commands. Changelog entries document fuzzy and partial substring matching, slash-command history, alias completion for built-ins, and fixes for autocompleted skill commands so they submit correctly.

When invoked, a skill command activates the associated skill. The skill's Markdown body and linked resources become model context through progressive disclosure. The command body is not pasted as a visible user message in the transcript. Scripts referenced from the skill are execution instructions for the agent; they run only if the agent later calls terminal tools, subject to normal permissions, sandboxing, and approval policy.

Precedence is partly documented by local built-in guidance: workspace project customizations override declared workspace configuration, which overrides global discovery, which overrides built-in customizations, with global declared configurations listed last. The same guidance says higher-priority customizations override lower-priority customizations on skill-name conflicts. It does not prove that a custom skill can override a built-in TUI slash command such as `/help`; Claudine should assume built-ins are reserved unless Antigravity documents otherwise.

Repo trust is not documented as a separate prompt for skill-derived slash commands in the sources checked. Project skills are loaded from version-controlled customization roots, and any script execution they request still goes through Antigravity's normal tool permission and sandbox model.

## Portability

Antigravity skill-derived commands are not directly portable to providers that use standalone Markdown command files. A portable conversion can usually preserve the instruction body, but it must rewrite metadata and invocation behavior.

Claudine should treat Antigravity command import as:

1. Read `skills/<name>/SKILL.md`.
2. Use frontmatter `name` as the visible slash-command name.
3. Use `description` as command discovery metadata where the target provider supports it.
4. Convert the Markdown body into the target provider's prompt body.
5. Flag or copy referenced `scripts/`, `resources/`, `references/`, and `examples/` assets according to the target provider's asset support.
6. Do not invent an argument placeholder. If the target provider requires one, generate a conservative command that treats trailing user text as ordinary prompt text or require manual review.

For export into Antigravity, Claudine should synthesize:

```text
skills/<command-name>/SKILL.md
```

with `name`, `description`, and a Markdown body. Claude Code or Gemini-style command files with `$ARGUMENTS`, `{{args}}`, shell interpolation, or provider-specific frontmatter need a rewrite and should not be symlinked into Antigravity.

## Claudine Linking Notes

The command linker should classify Antigravity as **rewrite-required skill-derived slash commands**.

Recommended behavior:

- Detect `SKILL.md` under supported skill roots as Antigravity command resources.
- Use the `name` field, not only the folder name, for visible command identity.
- Surface `description` as the discoverability text.
- Mark referenced sibling asset directories as non-portable unless the target provider has an equivalent asset model.
- Support `skills.json` include/exclude filters when determining whether a skill is active.
- Treat plugin-packaged skills as extension-scoped commands and preserve plugin boundaries when reporting provenance.
- Avoid direct links to `.commands`, `commands/`, or standalone `*.md` command roots; Antigravity does not document those as the current CLI command format.

`requires_claudine_update` is `true` because the existing command-linking model needs an Antigravity adapter that converts command files to skill packages and prevents false-positive direct Markdown command linking.

## Sources

- [Antigravity CLI product page](https://antigravity.google/product/antigravity-cli)
- [Antigravity CLI overview](https://antigravity.google/docs/cli/overview)
- [Plugins & Skills documentation](https://antigravity.google/docs/cli/plugins)
- [CLI reference documentation](https://antigravity.google/docs/cli/reference)
- [Authoring Google Antigravity Skills codelab](https://codelabs.developers.google.com/getting-started-with-antigravity-skills)
- [Accelerating Development with Antigravity CLI codelab](https://codelabs.developers.google.com/genai-for-dev-antigravity-cli)
- [google-antigravity/antigravity-cli repository](https://github.com/google-antigravity/antigravity-cli)
- [Antigravity CLI changelog](https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md)
