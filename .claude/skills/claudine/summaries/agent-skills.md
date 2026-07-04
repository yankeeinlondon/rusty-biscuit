# Agent Skill Support Across Agentic CLI Providers

Agent skills matter because they turn task knowledge into durable, reusable operating procedure. A good skill packages routing metadata, instructions, reference material, scripts, templates, and sometimes provider-specific permissions or tools into a unit the agent can discover and load only when relevant. That makes agent behavior more predictable across sessions: the model does not need every prompt to restate the release process, review rubric, migration checklist, or domain-specific debugging playbook.

Cross-provider portability is valuable for the same reason Claudine exists. Teams should not have to rewrite their operational knowledge every time they switch between Claude Code, Codex, Gemini CLI, Goose, Kimi, OpenCode, Qwen, Pi, or Kilo. But portability is not binary. The current provider set broadly converges on `SKILL.md` as the common artifact, yet differs in metadata validation, scope paths, precedence, trust gates, activation tools, permission semantics, sidecars, package systems, and extension formats. Claudine's portability classification has to capture those layers rather than treating a linked skill as automatically equivalent everywhere.

## High-Level Finding

The current skills research shows first-class Agent Skills support across the researched provider roster relevant to Claudine's linking work. The common center is a directory containing `SKILL.md` with YAML frontmatter and Markdown instructions. Most providers also preserve adjacent `scripts/`, `references/`, `assets/`, examples, or templates as part of the skill package.

The divergence is in the edges:

| Provider    | Support     | Primary Entry Point                 | Main User Scope                                                                                  | Main Repo Scope                                                                            | Activation Shape                                                                    |
|-------------|-------------|-------------------------------------|--------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------|
| Claude Code | First class | `SKILL.md`                          | `~/.claude/skills/<name>/SKILL.md`                                                               | `.claude/skills/<name>/SKILL.md`                                                           | Explicit `/name`, model invocation, subagent preload, settings overrides            |
| Codex       | First class | `SKILL.md`                          | `~/.agents/skills/` and `~/.codex/skills/`                                                       | `.agents/skills/`                                                                          | Explicit `$name` or `/skills`, implicit model selection, sidecar policy             |
| Gemini CLI  | First class | `SKILL.md`                          | `~/.gemini/skills/` and `~/.agents/skills/`                                                      | `.gemini/skills/` and `.agents/skills/`                                                    | Model proposes `activate_skill`; user consent loads body/resources                  |
| Goose       | First class | `SKILL.md`                          | `~/.agents/skills/`, provider config-dir skills, `~/.claude/skills/`, `~/.config/agents/skills/` | `.agents/skills/`, `.goose/skills/`, `.claude/skills/`                                     | Summon extension lists and loads skills; `/skills` can load explicitly              |
| Kimi Code   | First class | `SKILL.md` or flat `<name>.md`      | `~/.kimi/skills/`, `~/.claude/skills/`, `~/.codex/skills/`, generic agent dirs                   | `.kimi/skills/`, `.claude/skills/`, `.codex/skills/`, `.agents/skills/`                    | Prompt-visible metadata plus explicit `/skill:<name>` and `/flow:<name>`            |
| OpenCode    | First class | `SKILL.md`                          | `~/.config/opencode/{skill,skills}/`, `~/.claude/skills/`, `~/.agents/skills/`                   | `.opencode/{skill,skills}/`, `.claude/skills/`, `.agents/skills/`                          | Model sees `<available_skills>` and calls `skill` tool with permission check        |
| Qwen Code   | First class | `SKILL.md`                          | `~/.qwen/skills/` and `~/.agents/skills/`                                                        | `.qwen/skills/` and `.agents/skills/`                                                      | Model invocation through Skill tool, direct slash invocation, path-gated activation |
| Pi          | First class | `SKILL.md` or Pi-native flat `*.md` | `~/.pi/agent/skills/` and `~/.agents/skills/`                                                    | `.pi/skills/` and `.agents/skills/`                                                        | Model-visible metadata when `read` is available; explicit `/skill:<name>`           |
| Kilo Code   | First class | `SKILL.md`                          | `~/.kilo/skills/`, `~/.kilocode/skills/`, `~/.agents/skills/`, `~/.claude/skills/`               | `.kilo/{skill,skills}/`, `.kilocode/{skill,skills}/`, `.agents/skills/`, `.claude/skills/` | Model sees skill list and calls `skill` tool, filtered by permission policy         |

## Portable Core

The most portable unit is the skill directory rooted at `SKILL.md`.

| Asset                                  | Portability                                                                                             |
|----------------------------------------|---------------------------------------------------------------------------------------------------------|
| `SKILL.md` entry point                 | Portable across the researched providers, except Pi and Kimi also accept flat `.md` forms in some roots |
| YAML frontmatter                       | Portable when limited to standard Agent Skills keys                                                     |
| `name`                                 | Portable in concept, but validation, defaults, and directory-name matching differ                       |
| `description`                          | The most important routing field; missing descriptions often hide or reject a skill                     |
| Markdown body                          | Mostly portable when it avoids provider-specific tool names, commands, paths, and activation language   |
| `license`, `compatibility`, `metadata` | Generally portable standard metadata                                                                    |
| Adjacent files                         | Structurally portable, but host/runtime/provider assumptions require review                             |

The safest canonical representation for Claudine is therefore: a directory named for the skill, containing `SKILL.md`, standard `name` and `description` frontmatter, provider-neutral Markdown, and relative sibling assets.

## Format And Metadata Differences

The providers agree on the broad shape but not on strictness.

Claude Code implements the Agent Skills standard but is permissive. `SKILL.md` is the entry point, but fields can be omitted and `name` can default from the directory. Claude recognizes many provider-specific keys: `when_to_use`, `argument-hint`, `arguments`, `disable-model-invocation`, `user-invocable`, `allowed-tools`, `disallowed-tools`, `model`, `effort`, `context`, `agent`, `hooks`, `paths`, and `shell`.

Codex is stricter. It requires `name` and `description`; `name` must match the parent directory and follow lowercase alphanumeric hyphen rules. Codex also supports a provider-specific `agents/openai.yaml` sidecar for interface, implicit invocation policy, and MCP tool dependencies.

Gemini CLI requires `name` and `description`; missing fields silently skip the skill. It takes `name` from frontmatter, not the directory, normalizes invalid filename characters, and ignores provider-specific frontmatter from other tools.

Goose requires `name` and `description` and reads `metadata` as the agentskills.io free-form bag. Goose-specific `argument-hint` and `arguments` survive as untyped properties. It records non-`SKILL.md` files as supporting files that can be loaded by the skill runtime.

Kimi is notably lenient. `name` and `description` are optional at parse time: `name` defaults from the directory or flat filename, and `description` falls back to the first non-empty body line or `"No description provided."`. Kimi also supports `type: flow` for Mermaid/D2 flow skills, which is provider-specific.

OpenCode's docs require `name` and `description`, strict name grammar, and directory-name matching, but current source is more permissive: it accepts any string `name`, treats `description` as optional, and does not enforce directory parity. In practice, skills without descriptions are hidden from the model-facing skill list.

Qwen requires frontmatter bounded by `---`, and requires `name` and `description`. It recognizes Qwen-specific keys including camelCase `allowedTools`, `hooks`, `model`, `paths`, `priority`, `argument-hint`, `when_to_use`, `disable-model-invocation`, and `user-invocable`.

Pi requires a non-blank `description`; `name` can default from the parent directory, and Pi intentionally does not enforce `name == directory_name`. It accepts direct root `*.md` skills in Pi-native roots but not in `.agents/skills` roots.

Kilo's docs require `name` and `description`, but current source enforces only `name`; description-less skills load into the registry but are filtered out of the prompt formatter. Kilo also does not currently enforce the documented directory-name match.

## Scope And Path Differences

The major interoperability path is `.agents/skills/`. Codex, Gemini, Goose, Kimi, OpenCode, Qwen, Pi, and Kilo all use or scan `.agents/skills/` in at least one scope. Claude Code does not treat `.agents/skills/` as its native path; its canonical namespace is `.claude/skills/`.

Provider-branded paths remain important:

| Provider    | Branded User Paths                                    | Branded Repo Paths                                   |
|-------------|-------------------------------------------------------|------------------------------------------------------|
| Claude Code | `~/.claude/skills/`                                   | `.claude/skills/`                                    |
| Codex       | `~/.codex/skills/` plus `$CODEX_HOME/skills/.system/` | none branded; repo uses `.agents/skills/`            |
| Gemini CLI  | `~/.gemini/skills/`                                   | `.gemini/skills/`                                    |
| Goose       | provider config-dir `skills/` path                    | `.goose/skills/`                                     |
| Kimi Code   | `~/.kimi/skills/`, plus Claude/Codex brand fallbacks  | `.kimi/skills/`, `.claude/skills/`, `.codex/skills/` |
| OpenCode    | `~/.config/opencode/{skill,skills}/`                  | `.opencode/{skill,skills}/`                          |
| Qwen Code   | `~/.qwen/skills/`                                     | `.qwen/skills/`                                      |
| Pi          | `~/.pi/agent/skills/`                                 | `.pi/skills/`                                        |
| Kilo Code   | `~/.kilo/skills/`, `~/.kilocode/skills/`              | `.kilo/{skill,skills}/`, `.kilocode/{skill,skills}/` |

There are also provider-managed and non-user scopes: Claude managed skills, Codex bundled/admin/plugin skills, Gemini built-ins/extensions, Goose plugins/built-ins, Kimi built-ins/plugins, OpenCode built-ins/configured URLs, Qwen bundled/extension skills, Pi package and extension-discovered resources, and Kilo built-ins/URL caches/marketplace-installed skills. Claudine should inventory these, but not silently sync them as ordinary user-authored skills.

## Discovery And Precedence

Precedence is not portable.

Claude uses managed > personal > project > plugin, with nested project disambiguation and `skillOverrides` able to hide or demote skills.

Codex orders from lower to higher priority as system/bundled, admin, user, repo, plugin. Same-name conflicts are not merged; both can remain available in the selector.

Gemini orders built-in \< extension \< user \< workspace. Within user and workspace tiers, `.agents/skills/` beats `.gemini/skills/`.

Goose scans project first, then global, then plugin, then built-in, and first encounter wins. Its project order is `.agents/skills`, `.goose/skills`, `.claude/skills`; global order is `~/.agents/skills`, Goose config-dir skills, `~/.claude/skills`, and `~/.config/agents/skills`.

Kimi uses first match wins across a prioritized root list. Project brand directories outrank project generic directories, which outrank user brand and user generic directories, then extra paths, plugin roots, and built-ins. Brand priority is `kimi > claude > codex`.

OpenCode uses last-writer-wins. It registers built-ins first, then external global and project paths, native config directories, configured `skills.paths[]`, and URL-backed skills. Later same-name entries replace earlier ones.

Qwen uses project > user > extension > bundled. Duplicate names are shadowed by the first higher-precedence tier, and model-facing lists are alphabetically sorted; `priority` only affects `/skills` display.

Pi uses first-writer-wins with collision diagnostics. Trusted project `.pi/skills` and project `.agents/skills` are added before user native and user `.agents` paths. CLI and extension paths are merged into the effective list with canonical path de-duplication.

Kilo uses last-one-wins by frontmatter `name`. Built-ins are seeded first; compatibility directories, Kilo config directories, configured paths, and URL-backed sources can override earlier entries.

## Activation Mechanics

Activation is where a linked skill can most easily change behavior.

Claude Code progressively loads skills. Metadata is available up front; the full body loads through explicit slash invocation, model invocation from `description`, or subagent preload. Frontmatter and settings can disable model invocation, hide user invocation, alter tool permissions, or route behavior through agents/hooks.

Codex also uses progressive disclosure. Users can invoke `$skill-name` or `/skills`; the model may select a skill from its description. `agents/openai.yaml` can disable implicit invocation while preserving explicit use.

Gemini adds a consent gate. The model proposes a skill through `activate_skill`; the user confirms; only then are the body and resources injected and the skill directory added to allowed paths. Built-ins are pre-approved.

Goose relies on the Summon built-in extension. Skills are exposed through the extension and `/skills`; disabling Summon disables all skill discovery and loading.

Kimi injects skill name/path/description grouped by scope, then lets the model read the skill. Users can explicitly invoke `/skill:<name>`. Flow skills add `/flow:<name>` and diagram-driven execution semantics.

OpenCode shows `<available_skills>` entries and requires the model to call the `skill` tool. Loading runs through permission checks. Agent-level `tools.skill: false` or `permission.skill` rules can hide or reject skills.

Qwen supports both model invocation and direct user invocation. `disable-model-invocation`, `user-invocable`, `skills.disabled`, safe mode, bare mode, extension enablement, and path-gated activation all affect whether a skill appears or can be used.

Pi appends `<available_skills>` only when the `read` tool is available. `disable-model-invocation` hides the skill from model discovery but not explicit `/skill:<name>`. Project resources are trust-gated.

Kilo formats an available-skill list and loads bodies through a `skill` tool filtered by permission policy. It can hide skills by denying the `skill` permission globally or per name.

## Portability Classification

Claudine should classify skills as layered resources.

| Layer                                 | Portable?              | Notes                                                                                                                                             |
|---------------------------------------|------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------|
| Standard `SKILL.md` directory         | Usually portable       | Best canonical artifact when it has `name`, `description`, provider-neutral Markdown, and relative assets                                         |
| Standard metadata                     | Mostly portable        | `name`, `description`, `license`, `compatibility`, `metadata` are the safest keys                                                                 |
| Provider metadata                     | Provider-specific      | Claude hooks/agents/context, Codex sidecars, Qwen hooks/paths, Pi trust/package wiring, OpenCode/Kilo URL sources, etc. need mapping or filtering |
| Activation controls                   | Conditional            | `disable-model-invocation`, `user-invocable`, implicit policy, consent gates, and path activation differ by provider                              |
| Permissions/tool allowlists           | Usually rewrite-needed | Tool names and permission grammars are provider-specific                                                                                          |
| Scripts/assets                        | Host-sensitive         | Preserve structurally, but flag OS, shell, runtime, binary, and repo-layout assumptions                                                           |
| Built-ins/extensions/plugins/packages | Usually non-portable   | Inventory as provider-managed unless materialized as a normal user/repo skill directory                                                           |

A practical Claudine classification should look like this:

| Classification                 | Meaning                                                                                                              | Linking Behavior                                                             |
|--------------------------------|----------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------|
| Portable                       | Standard `SKILL.md` content can be linked with no semantic rewrite beyond path placement                             | Symlink or copy into provider skill root                                     |
| Portable With Provider Mapping | Core skill travels, but metadata or activation controls need translation or omission                                 | Link core files and emit provider-specific sidecars/config where supported   |
| Linked But Degraded            | Target provider loads the skill but ignores some semantics                                                           | Link with warnings about ignored activation, permissions, tools, or metadata |
| Rewrite Required               | Skill depends on provider-only behavior, flat-file shape, sidecar config, package wiring, or host-specific execution | Do not present as equivalent; require explicit rewrite or scoped warning     |
| Non-Portable                   | Built-in, managed, bundled, plugin-only, extension-only, trust-state, cache-state, or policy-controlled asset        | Inventory only; do not sync as a user/repo skill                             |

## Implications For Claudine

Claudine should use the Agent Skills directory as the canonical source representation where possible, then project it into provider-specific roots. `.agents/skills/` is the most important cross-provider path because it is first-class or scanned by Codex, Gemini, Goose, Kimi, OpenCode, Qwen, Pi, and Kilo. `.claude/skills/` remains essential because Claude Code is the origin of the convention and several providers scan it as a compatibility path.

The linker should never imply identical behavior just because two providers can parse `SKILL.md`. It should report four facts for every linked skill:

1. What moved unchanged: `SKILL.md`, standard frontmatter, Markdown body, sibling assets.
2. What was mapped: path placement, frontmatter keys, invocation flags, sidecars, or provider config.
3. What was ignored or degraded: unsupported metadata, activation controls, permissions, hooks, model selectors, path gates, package manifests.
4. What remains host-specific: scripts, binaries, OS paths, shell assumptions, MCP dependencies, and repository layout assumptions.

The strategic point of view is that Claudine should optimize for transparent interoperability, not false equivalence. A linked skill is useful when the agent can discover and apply the same operational knowledge. It is only portable when the target provider preserves the same routing, activation, permission, resource, and execution semantics. Where it cannot, Claudine's portability classification should make the loss explicit and keep provider-owned assets out of the shared skill pool.
