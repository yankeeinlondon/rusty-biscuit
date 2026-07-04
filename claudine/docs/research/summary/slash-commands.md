---
sequence:
- name: draft
- name: iterate
- name: finalize
prompt: |-
  Slash commands — user-authored reusable prompts invoked by name — exist in most agentic CLIs, but each provider chooses its own on-disk format, metadata keys, storage locations and scopes, argument handling, and invocation grammar. Claudine links commands across providers, so this variance is what its portability classification must capture.

  ## Task

  Your task is to report on slash command / reusable prompt support across the Agentic CLI providers Claudine supports.

  - your report should start by outlining why reusable prompt commands matter to agentic workflows and why cross-provider sharing is valuable
  - and then shift its focus to how providers differ: on-disk format, recognized metadata, user/repo scopes and paths, argument and interpolation support, and invocation grammar
  - close with a point of view on the implications for Claudine's linking strategy and portability classification

  As background material we have slash-commands research documents for each provider that Claudine supports. They can be found at `@claudine/docs/research/slash-commands/*.md`.

  Important: your final response is saved verbatim as the body of this summary document, so it must be the complete document text and nothing else — no preamble, no commentary. Never write to this document yourself.

  ::block when="state.name == 'draft'"
  - Iterate over the first three research documents to develop a point of view on how to write this document and then produce an initial draft of the document
  ::end-block
  ::block when="state.name == 'iterate'"

  - Note: the initial draft has already been created — it is the body of `@claudine/docs/research/summary/slash-commands.md` (everything below the frontmatter); read it from there
  - Act as an orchestrator and iterate over each remaining provider's research document:
      - provide the subagent the current draft and ask them to return an improved draft based on the research document they've been assigned
  - Once every remaining provider has been incorporated, your final response is the fully updated draft
  ::end-block

  ::block when="state.name == 'finalize'"

  The document has now gone through several rounds of improvement and your task is just to make sure the document is consistent in tone and detail and that nothing looks incorrect or incomplete. The current draft is the body of `@claudine/docs/research/summary/slash-commands.md` (everything below the frontmatter); read it from there, make any adjustments, and your final response will be considered the finalized summary document.
  ::end-block
hash: a9df60bf31aaf652-b3ec7098629ba055
last_updated: 2026-07-03
---
## Why Reusable Prompt Commands Matter

Reusable prompt commands are one of the main ways agentic CLI users turn repeated workflow knowledge into a durable interface. A good command captures more than a prompt snippet: it can encode task framing, expected inputs, tool assumptions, file context, model or agent preferences, and sometimes approval or execution behavior. Teams use these commands for repeatable reviews, release chores, debugging playbooks, documentation updates, migrations, research passes, and repo-specific operating procedures.

That matters because agentic workflows are stateful and collaborative. A command gives the user a stable handle such as `/review`, `/commit`, `/run-tests`, `$skill-creator`, `/skill:deploy`, or `/component Button` instead of asking every person to remember the same long instruction block. Repo-scoped commands let project maintainers ship workflow knowledge with the codebase. User-scoped commands let individuals carry personal working patterns across projects.

Cross-provider sharing is valuable for the same reason Claudine exists: users do not live in one provider forever. They switch between Claude Code, Codex, Gemini, Goose, Kimi, OpenCode, Qwen, Roo, and other CLIs based on model quality, permissions, UI, cost, local policy, or task shape. If a useful reusable prompt can move with them, the command becomes a durable asset rather than provider lock-in. But sharing is only safe when Claudine understands which parts are portable prose and which parts are provider-specific execution semantics.

This summary focuses on Claudine's compiled provider set: Claude Code, Codex CLI, Gemini CLI, Goose, Kimi Code, OpenCode, Qwen Code, and Roo Code. The current slash-command research folder also includes forward-looking research for Pi and Kilo Code. Those are useful for future provider metadata work, but they should not be silently treated as currently compiled Claudine providers.

## The Core Problem

All providers expose something command-like, but they disagree on almost every important detail.

| Provider    | Primary reusable command surface      | Native invocation                                      | Main artifact                                 |
|:------------|:--------------------------------------|:-------------------------------------------------------|:----------------------------------------------|
| Claude Code | Skills and legacy custom commands     | `/name`                                                | `SKILL.md` directory or `.md` command file    |
| Codex CLI   | Agent Skills                          | `$SkillName` or `/skills` picker                       | `SKILL.md` directory                          |
| Gemini CLI  | Custom commands plus Agent Skills     | `/name` for commands; model/tool activation for skills | TOML command file or `SKILL.md` directory     |
| Goose       | Custom slash command mapped to recipe | `/name`                                                | `config.yaml` entry plus YAML/JSON recipe     |
| Kimi Code   | Agent Skills and flow skills          | `/skill:<name>` or `/flow:<name>`                      | `SKILL.md` directory or flat Markdown skill   |
| OpenCode    | Custom commands                       | `/name` or `opencode run --command`                    | Markdown command file or JSON config entry    |
| Qwen Code   | Custom commands plus Agent Skills     | `/name`, `/<skill-name>`, or `/skills <name>`          | Markdown command file or `SKILL.md` directory |
| Roo Code    | Custom commands and skills            | `/name`                                                | Markdown command file or skill directory      |

The shared concept is simple: a durable named prompt resource. The implementation surface is not simple. Claudine cannot classify portability by file extension alone, and it cannot assume that a visible slash command in one CLI maps to a visible slash command in another.

## On-Disk Formats

The first portability split is artifact shape.

Claude Code and OpenCode are the closest group among current Claudine providers. They accept Markdown command bodies, YAML frontmatter, `$ARGUMENTS`, positional `$N` placeholders, and inline shell injection with bang-backtick command syntax. Even there, the formats are not identical. Claude has skills, legacy `.claude/commands`, plugin namespaces, model/effort/context fields, tool permission fields, and workspace trust behavior. OpenCode adds JSON command definitions under a `command` object, `agent`/`model`/`variant`/`subtask` fields, file references, managed or runtime config layers, and slash-separated nested command names such as `/review/code`.

Codex uses the open Agent Skills shape: a directory containing `SKILL.md` with required `name` and `description` frontmatter. It is not a user-defined `/name` slash-command system. Codex skills are invoked by `$SkillName`, selected through `/skills`, or activated implicitly. This makes Codex skill files structurally more portable to other Agent Skills implementations, but less directly equivalent to classic slash commands.

Gemini has two surfaces. Custom slash commands are TOML files with a required `prompt` string and optional `description`. Agent Skills are separate `SKILL.md` directories. A Gemini TOML command cannot be linked into Markdown-command providers as-is; it must be converted into the target provider's command file format.

Goose is more indirect. A Goose slash command is not a command file. It is an entry in user `config.yaml` under `slash_commands:` that maps a flat command name to a recipe file path. The recipe is YAML or JSON and may include `instructions`, `prompt`, `parameters`, extensions, settings, retry behavior, sub-recipes, and response schemas. Claudine should treat this as a two-artifact command resource: config entry plus referenced recipe.

Kimi's user-defined command surface is Agent Skills, not a separate custom slash-command file format. A standard skill is a directory with a `SKILL.md` entry point, and Kimi also accepts a flat `<name>.md` file placed directly in a skills directory. Standard skills are invoked with `/skill:<name>`. Flow skills set `type: flow` in frontmatter, embed a Mermaid or D2 diagram, and run through the flow engine with `/flow:<name>`.

Qwen has Markdown custom commands in `.qwen/commands` and skills in `.qwen/skills`; deprecated TOML command files are still parsed, but Markdown is the current command format. Qwen also supports extension-provided skills declared by enabled extensions.

Roo Code is modeled in Claudine's current provider catalog as Markdown commands under `.roo/commands` and skills under `.roo/skills`. Command frontmatter includes fields such as `description`, `argument-hint`, and `mode`; project commands override user commands. The current slash-command research folder does not include a dedicated Roo topic document, so Roo details should remain flagged as catalog-derived until refreshed through the same research workflow as the other providers.

## Metadata Differences

Metadata is one of the least portable parts of these systems.

Claude Code recognizes fields such as `name`, `description`, `when_to_use`, `argument-hint`, `arguments`, `disable-model-invocation`, `user-invocable`, `allowed-tools`, `disallowed-tools`, `model`, `effort`, `context`, `agent`, `hooks`, `paths`, and `shell`.

Codex's portable Agent Skills core is narrower: `name` and `description` are required, with optional fields such as `license`, `compatibility`, `metadata`, and experimental `allowed-tools`. Codex-specific `agents/openai.yaml` is not part of the portable core.

Gemini TOML commands recognize `prompt` and `description`; Gemini skills use `name` and `description`. OpenCode command metadata includes `description`, `agent`, `model`, `variant`, and `subtask`. Qwen custom commands use Markdown with optional `description`; Qwen skills use required `name` and `description`, plus optional `priority`, `paths`, `user-invocable`, and `disable-model-invocation`. Kimi follows Agent Skills metadata for standard skills and adds `type: flow` for flow skills. Goose recipe metadata is its own schema rather than Markdown frontmatter. Roo command metadata is Markdown frontmatter, but its mode-aware fields are Roo-specific.

The practical rule is that descriptive metadata can often be mapped, but execution metadata usually cannot. A `description` can survive most conversions. A Claude `allowed-tools`, OpenCode `subtask`, Kimi `type: flow`, Goose `extensions`, Qwen `paths`, or Roo `mode` field carries provider-specific behavior and needs either a targeted rewrite or an explicit portability warning.

## Scope and Storage Differences

Providers also disagree on where commands live.

| Provider    | User command or skill location examples                                                                |
|:------------|:-------------------------------------------------------------------------------------------------------|
| Claude Code | `~/.claude/skills/<name>/SKILL.md`, `~/.claude/commands/<name>.md`                                     |
| Codex CLI   | `~/.agents/skills/<name>/SKILL.md`, deprecated `~/.codex/skills/<name>/SKILL.md`                       |
| Gemini CLI  | `~/.gemini/commands/**/*.toml`, `~/.gemini/skills/`, `~/.agents/skills/`                               |
| Goose       | `~/.config/goose/config.yaml`, `~/.config/goose/recipes/<name>.yaml`                                   |
| Kimi Code   | `~/.kimi/skills`, `~/.claude/skills`, `~/.codex/skills`, `~/.config/agents/skills`, `~/.agents/skills` |
| OpenCode    | `~/.config/opencode/commands/<name>.md`, `~/.config/opencode/opencode.json`                            |
| Qwen Code   | `~/.qwen/commands/<name>.md`, `~/.qwen/skills/<name>/SKILL.md`                                         |
| Roo Code    | `~/.roo/commands`, `~/.roo/skills`                                                                     |

Repo scope is also inconsistent. Claude uses `.claude/skills` and `.claude/commands`. Codex prefers `.agents/skills` and also scans `.codex/skills` under trust rules. Gemini uses `.gemini/commands`, `.gemini/skills`, and `.agents/skills`. Goose supports repo-local recipe files such as `.goose/recipes/<name>.yaml`, but the slash-command mapping itself is documented as user config. OpenCode walks `.opencode/commands` and config files up to the worktree root. Qwen uses `.qwen/commands` and `.qwen/skills`. Kimi discovers project skills from brand and generic skill directories near the nearest git ancestor. Roo uses `.roo/commands` and `.roo/skills`, with mode-specific skill directories also represented in Claudine's catalog.

This means Claudine's linker needs provider-specific placement rules. There is no universal "commands directory" and no universal meaning for user, repo, extension, package, managed, runtime, or system scope.

## Argument and Interpolation Differences

Argument handling is a major portability boundary.

Claude Code supports a rich placeholder grammar: `$ARGUMENTS` for the raw argument string, `$ARGUMENTS[N]`, `$N`, named arguments declared in frontmatter, and quoting rules for multi-word values. If `$ARGUMENTS` is absent, Claude appends the input as an `ARGUMENTS:` block.

OpenCode overlaps with Claude but is not identical. It supports `$ARGUMENTS` and positional `$1`, `$2`, etc.; the highest-numbered positional placeholder receives remaining arguments joined by spaces. Arguments are tokenized with shell-style quote awareness. If a template contains no `$ARGUMENTS` or `$N` placeholders, the raw argument string is appended to the rendered prompt.

Gemini and Qwen use `{{args}}` for the raw argument string. Qwen appends arguments to the rendered prompt, separated by two line breaks, when `{{args}}` is absent. Qwen also shell-escapes `{{args}}` automatically when it appears inside `!{...}`. Gemini custom commands have the same broad raw-argument shape but live in TOML rather than Markdown.

Goose accepts exactly one raw parameter after the slash command name and maps it into a declared recipe parameter, substituted with Jinja-style `{{ parameter_name }}` placeholders in recipe fields such as `instructions`, `prompt`, and activities. Any other recipe parameters must be optional and provide defaults.

Codex Agent Skills have no placeholder grammar; any arguments are ordinary prompt text after `$SkillName` or after selecting a skill. Kimi similarly appends additional text after `/skill:<name>` or `/flow:<name>` without documented positional or named placeholder substitution. Roo's provider catalog marks prompt arguments as positional-only, but the dedicated slash-command research refresh is still missing and should verify exact substitution syntax before Claudine rewrites Roo arguments automatically.

Shell and file interpolation are even less portable. Claude and OpenCode support inline shell output with similar bang-backtick command syntax. OpenCode supports `@path` file references. Gemini and Qwen use `!{...}` shell injection and `@{...}` file injection; Qwen expands file references first, shell commands second, and `{{args}}` last. Goose recipes can include extensions, settings, sub-recipes, retry checks, and response schemas that reach beyond prompt interpolation.

Claudine should treat shell injection, file injection, dynamic context expansion, recipe extensions, subtask routing, flow execution, mode selection, and tool permissions as non-portable by default.

## Invocation Grammar

The visible command grammar differs enough that "slash command" is an overloaded term.

| Provider    | Invocation examples                        | Namespace behavior                                                                                   |
|:------------|:-------------------------------------------|:-----------------------------------------------------------------------------------------------------|
| Claude Code | `/commit`, `/research:publish`             | Skills, legacy commands, built-ins, nested skills, and plugins share or qualify names                |
| Codex CLI   | `$skill-creator`, `/skills`                | User-defined skills are not `/name` commands                                                         |
| Gemini CLI  | `/commit`, `/git:commit`                   | TOML command paths become colon namespaces                                                           |
| Goose       | `/run-tests`                               | Flat custom command names in `config.yaml`; built-ins win                                            |
| Kimi Code   | `/skill:code-style`, `/flow:release`       | Skill and flow prefixes are part of invocation grammar                                               |
| OpenCode    | `/test`, `/review/code`                    | Nested directories map to slash-style command names                                                  |
| Qwen Code   | `/commit`, `/git:commit`, `/skills review` | Nested command directories become colon namespaces; skills are also user-invokable unless disabled   |
| Roo Code    | `/name`                                    | Current catalog models Markdown commands in `.roo/commands`; project commands override user commands |

These differences affect discoverability and linking. A Claude nested command may look like `/research:publish`; an OpenCode nested command may look like `/research/publish`; a Qwen nested command uses colon names such as `/research:publish`; a Kimi skill requires `/skill:research-publish`; a Codex skill is invoked as `$research-publish` or selected through `/skills`.

A linked file that preserves the prompt body but changes invocation grammar is useful, but it is not "portable" in the strict sense. The user-facing command name has changed.

## Trust, Precedence, and Disable Behavior

Trust and precedence are not uniform.

Claude project skills and commands require workspace trust before project-level permissions take effect. Codex project skills require project trust. Gemini workspace commands and skills are gated by folder trust. Qwen project resources are gated by Trusted Folders when enabled. OpenCode does not use the same separate trust dialog for command files; it gates actions through its permission model. Kimi does not document an explicit workspace-trust dialog for repo skills. Goose slash-command definitions live in user config, while the referenced recipe may come from several locations and still obey Goose's normal tool approval mode. Roo's trust and reload semantics should be verified in a dedicated Roo slash-command research pass before Claudine relies on them for automated classification.

Conflict handling also differs. Claude has precedence among managed, personal, project, plugin, skill, and legacy command resources. Codex does not merge same-name skills in the same slash namespace because user skills are not `/name` commands. Gemini project commands override user commands, but built-ins win on collision. Goose built-ins win and custom names are flat. OpenCode lets user-defined commands override built-ins and merges JSON and Markdown definitions. Qwen project commands override user commands and project skills override personal skills, but exact command-versus-skill collision precedence is not documented. Kimi groups discovered skills under Project, User, Extra, and Built-in scopes, with Project > User > Extra > Built-in precedence and brand-directory precedence of Kimi > Claude > Codex within a scope.

Disable mechanisms vary from deleting files to config flags, CLI flags, environment variables, in-session commands, and frontmatter fields. Qwen supports `user-invocable: false`, `disable-model-invocation: true`, `--disabled-slash-commands`, `slashCommands.disabled`, and `QWEN_DISABLED_SLASH_COMMANDS`. Gemini supports `/commands reload`, `/skills disable`, `/skills enable`, settings-based skill disablement, and extension disablement. OpenCode mostly requires removing or renaming files or JSON entries, though project config can be skipped. Goose slash commands are disabled by removing the `slash_commands:` entry and restarting, or by removing the referenced recipe. Claudine's model should record disable support as provider-specific metadata, not as a common capability.

## Portability View

A useful portability classification should separate four cases.

**Portable Agent Skills core:** A standard `SKILL.md` directory with only common Agent Skills metadata and Markdown body can often be linked or copied among Agent Skills-compatible providers, especially Codex and providers that search `.agents/skills`. Kimi, Gemini, Qwen, and Codex all participate in this ecosystem to some degree. Even then, invocation semantics differ, so "portable" should mean the artifact is structurally portable, not that the user invokes it the same way everywhere.

**Prompt-body portable with rewrite:** Most Markdown command bodies are reusable after transforming metadata and argument placeholders. Claude, OpenCode, Qwen, Roo, and standard Kimi skill bodies often fall into this category. The prose can survive; `$ARGUMENTS`, `{{args}}`, `$1`, `/skill:` prefixes, model fields, agent fields, mode fields, subtask fields, tool fields, file references, and shell blocks need mapping or removal.

**Format-convertible but non-portable:** Gemini TOML commands, deprecated Qwen TOML commands, Goose recipes, and OpenCode JSON command objects can be converted, but the source artifact should not be linked directly into another provider. Claudine needs to parse and emit a target-native resource. Goose is especially important because the command is split between a config entry and a recipe file.

**Provider-specific execution assets:** Shell injection, file injection, extension hooks, recipe extensions, response schemas, retry checks, sub-recipes, flow diagrams, subagent routing, model overrides, permission fields, path-gated activation, trust-gated project loading, package-managed resources, runtime config overlays, and managed policy behavior are not portable by default. Claudine should preserve them only when the target provider has an explicit equivalent.

## Implications for Claudine

Claudine's linking strategy should avoid a single "slash command" abstraction that assumes `/name`, Markdown, and `$ARGUMENTS`. The better model is a provider-normalized reusable prompt resource with explicit fields for artifact kind, storage scope, invocation grammar, argument grammar, interpolation features, metadata fields, trust requirements, precedence, disable mechanisms, and portability class.

The command linker should be conservative.

| Source feature                | Claudine strategy                                                                                 |
|:------------------------------|:--------------------------------------------------------------------------------------------------|
| Plain Markdown prose          | Link or copy when the target artifact can hold Markdown                                           |
| Standard Agent Skills core    | Treat as the most portable shared format                                                          |
| Provider-specific frontmatter | Map known descriptive fields; strip or warn on execution fields                                   |
| Argument placeholders         | Rewrite only with an explicit source-to-target grammar mapping                                    |
| Raw appended arguments        | Preserve only when the target has equivalent append semantics                                     |
| Shell and file injection      | Mark non-portable unless the target has a verified equivalent                                     |
| Goose recipes                 | Treat as config-plus-recipe resources, not standalone command files                               |
| Gemini TOML commands          | Convert to target-native command format rather than linking                                       |
| OpenCode JSON commands        | Convert the `command` entry to the target's native command format                                 |
| Qwen Markdown commands        | Rewrite `{{args}}`, `!{...}`, `@{...}`, and colon namespaces as needed                            |
| Qwen skills                   | Map the Agent Skills core, but preserve or warn on Qwen-only fields                               |
| Kimi standard skills          | Reuse the body, but rewrite `/skill:<name>` invocation semantics                                  |
| Kimi flow skills              | Classify separately from ordinary prompt commands                                                 |
| Codex skills                  | Do not present as native `/name` slash commands                                                   |
| Roo commands                  | Keep supported but mark detailed portability as research-pending until the Roo topic is refreshed |

The portability label should communicate whether Claudine can link the artifact as-is, link it with path placement only, transform it predictably, or require human review. A command whose body is reusable but whose runtime behavior changes should not be labeled fully portable.

The point of view for Claudine is therefore: make sharing easy, but make semantics explicit. Reusable prompt commands are valuable because they preserve workflow knowledge across tools and projects. The same portability system must protect users from false equivalence. A linked `/deploy` that silently drops shell expansion, ignores tool restrictions, changes argument parsing, skips a Goose recipe extension, bypasses trust expectations, or invokes a different agent is worse than no link at all. Claudine should favor transparent classification, targeted rewrites, and visible warnings over broad automatic linking.
