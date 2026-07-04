# Agent Skill Support Across Agentic CLI Providers

Skills matter because they give an agent durable, task-specific operating knowledge without forcing every prompt to restate context. A good skill packages routing metadata, instructions, reference material, scripts, templates, and sometimes provider-specific permissions or tools into a reusable unit. That makes agent behavior more predictable: the model can discover the right procedure, load only the detail it needs, and apply a workflow consistently across sessions and repositories.

Cross-provider skill portability is valuable for the same reason Claudine exists: teams do not want their operational knowledge locked inside one agentic CLI. If a repository has a mature release skill, review skill, migration skill, or domain-specific debugging skill, that knowledge should be linkable into every supported provider with a clear understanding of what survives unchanged and what becomes provider-specific. Portability is not binary. The same `SKILL.md` may be structurally portable while its activation rules, tool permissions, sidecars, scripts, or path assumptions are not.

The first three researched providers, Claude Code, Codex, and Gemini CLI, all now have first-class skill support and all align around the Agent Skills `SKILL.md` convention. That common core is meaningful: each provider understands a directory containing `SKILL.md`, YAML frontmatter, and Markdown instructions, with `name` and `description` as the main cross-tool routing fields. The differences begin immediately after that shared center.

## Provider Comparison

| Provider    | Support Level | Entry Point | Standard User Scope                                                                         | Standard Repo Scope                                                               | Activation Model                                                                                       |
|-------------|--------------:|-------------|---------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------|
| Claude Code | First class   | `SKILL.md`  | `~/.claude/skills/<skill-name>/SKILL.md`                                                    | `.claude/skills/<skill-name>/SKILL.md`                                            | Explicit `/skill-name`, automatic model invocation from metadata, and subagent preload unless disabled |
| Codex       | First class   | `SKILL.md`  | `~/.agents/skills/<skill-name>/SKILL.md` and legacy `~/.codex/skills/<skill-name>/SKILL.md` | `.agents/skills/<skill-name>/SKILL.md`                                            | Explicit `$skill-name` or `/skills`, plus implicit model selection unless blocked by sidecar policy    |
| Gemini CLI  | First class   | `SKILL.md`  | `~/.gemini/skills/<skill-name>/SKILL.md` and `~/.agents/skills/<skill-name>/SKILL.md`       | `.gemini/skills/<skill-name>/SKILL.md` and `.agents/skills/<skill-name>/SKILL.md` | Model proposes `activate_skill`; user consent injects the full skill body and resources                |

## Common Portable Core

The portable core across these providers is:

| Asset                                                   | Portability                                                                 |
|---------------------------------------------------------|-----------------------------------------------------------------------------|
| `SKILL.md` directory entry point                        | Portable across all three                                                   |
| YAML frontmatter                                        | Portable when limited to standard Agent Skills keys                         |
| `name`                                                  | Portable, but validation differs by provider                                |
| `description`                                           | Portable and important for automatic routing                                |
| Markdown body                                           | Mostly portable when it avoids provider-specific commands, tools, and paths |
| `license`, `compatibility`, `metadata`                  | Portable standard metadata                                                  |
| Supporting `references/`, `assets/`, `scripts/` folders | Structurally portable, but contents may be host- or provider-specific       |

The reusable artifact Claudine should treat as the highest-confidence portable unit is therefore the skill directory rooted at `SKILL.md`, with standard frontmatter and provider-neutral Markdown. Everything outside that center needs classification.

## Format And Metadata Differences

Claude Code implements the Agent Skills standard but relaxes it. `SKILL.md` is the required entry point, but Claude allows frontmatter fields to be omitted and can default `name` from the directory name. It recognizes many Claude-specific fields, including `when_to_use`, `argument-hint`, `arguments`, `disable-model-invocation`, `user-invocable`, `allowed-tools`, `disallowed-tools`, `model`, `effort`, `context`, `agent`, `hooks`, `paths`, and `shell`. This makes Claude skills expressive, but it also creates a wide non-portable edge.

Codex is stricter about the standard shape. It requires `name` and `description`, requires `name` to match the parent directory, and validates the name as lowercase letters, numbers, and hyphens with additional length and hyphen-placement rules. Codex also supports a provider-specific `agents/openai.yaml` sidecar for interface, implicit invocation policy, and MCP tool dependencies. That sidecar is useful inside Codex but should be treated as Codex-specific metadata during linking.

Gemini CLI also requires `name` and `description`; a missing field silently skips the skill. Its `name` comes from frontmatter rather than the directory name, and invalid filename characters are normalized. `SKILL.md` must appear either at the root of a skills directory or one directory deep, and the filename is case-sensitive on case-sensitive filesystems. Gemini intentionally ignores provider-specific frontmatter from other tools, which makes some linked skills appear to work while quietly losing behavior.

## Scope And Path Differences

Claude Code uses `.claude` as its provider namespace. It supports managed system skills, user skills under `~/.claude/skills/`, project skills under `.claude/skills/`, legacy `.claude/commands/*.md`, and plugin skills under `<plugin>/skills/`. It also supports nested project skills and qualified names for collisions in monorepos.

Codex uses the cross-tool `.agents` namespace as its preferred modern location, while still scanning legacy/default locations under `~/.codex/skills/`. User skills can live under `~/.agents/skills/` or `~/.codex/skills/`; repo skills live under `.agents/skills/` from the launch directory up to the Git repository root, plus a parent-directory shared scope. Codex also has bundled system skills under `$CODEX_HOME/skills/.system/`, Linux admin skills under `/etc/codex/skills/`, and plugin skills.

Gemini CLI supports both Gemini-specific and cross-tool locations. User skills can live under `~/.gemini/skills/` or `~/.agents/skills/`; workspace skills can live under `.gemini/skills/` or `.agents/skills/`. Within a tier, `.agents/skills/` takes precedence over `.gemini/skills/`. Gemini also has built-in npm-package skills and extension-bundled skills under `~/.gemini/extensions/<extension>/skills/`.

## Activation Differences

The biggest portability risk is activation semantics.

Claude Code makes skill metadata available up front, then progressively loads the full body when the model or user invokes the skill. A skill can be explicitly invoked as a slash command, auto-invoked from `description`, hidden from user invocation, blocked from model invocation, or routed into a subagent. Claude also has global and per-skill controls such as safe mode, bare mode, `skillOverrides`, and policy-managed skills.

Codex also uses progressive disclosure. It scans metadata at session start, then loads the body only when selected. Users can explicitly invoke a skill with `$skill-name`, and the model may implicitly select a skill from its `description`. Codex-specific `agents/openai.yaml` can disable implicit invocation while preserving explicit use. Individual skills can also be disabled through `[[skills.config]]` entries in `~/.codex/config.toml`.

Gemini CLI adds a consent gate. It scans enabled skills and injects `name` and `description` into the system prompt, but activation happens through the `activate_skill` tool. The model proposes a skill, the user sees a confirmation prompt, and only after approval is the body loaded and the skill directory added to allowed file paths. Built-in skills are pre-approved. Workspace skills are additionally gated by folder trust unless the workspace is trusted by settings, CLI flag, or environment variable.

## Portability Classification Implications

Claudine should classify skills as layered resources rather than single files with a yes/no portability bit.

The base layer is portable when a skill has a valid `SKILL.md`, standard Agent Skills frontmatter, and provider-neutral Markdown. Claude, Codex, and Gemini can all consume that center with path-specific linking.

The provider metadata layer is conditionally portable. Claude fields such as `context`, `agent`, `hooks`, `paths`, `shell`, `disable-model-invocation`, and `user-invocable` need Claude-specific handling. Codex `agents/openai.yaml`, MCP dependency declarations, and `policy.allow_implicit_invocation` need Codex-specific handling. Gemini activation consent, extension packaging, `.skill` archives, trust gates, and `.agents` precedence rules need Gemini-specific handling.

The executable/resource layer is host-sensitive. `scripts/`, inline shell commands, tool allowlists, MCP dependencies, and file references may depend on OS, shell, language runtime, repository layout, or provider tool names. These should be marked as non-portable or requiring rewrite unless Claudine can prove they are generic.

For linking strategy, the practical point of view is:

| Classification                 | Meaning                                                                                    | Linking Behavior                                                               |
|--------------------------------|--------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------|
| Portable                       | Standard `SKILL.md` content can be linked as-is                                            | Symlink or copy into provider skill root                                       |
| Portable With Provider Mapping | Core skill travels, but metadata needs translation or omission                             | Link core files and emit provider-specific sidecars/config where supported     |
| Linked But Degraded            | Provider will load the skill but ignore some semantics                                     | Link with warnings that activation, permissions, or tools changed              |
| Rewrite Required               | Skill depends on provider-only behavior or host-specific execution                         | Do not silently link as equivalent; require explicit rewrite or scoped warning |
| Non-Portable                   | Built-in, managed, plugin-only, or policy-controlled asset cannot be represented elsewhere | Inventory only; do not sync as a user/repo skill                               |

The initial provider set suggests Claudine should prefer a canonical Agent Skills directory as the source representation, then project it into provider-specific roots: `.claude/skills/` for Claude, `.agents/skills/` for Codex, and either `.agents/skills/` or `.gemini/skills/` for Gemini depending on desired precedence. The `.agents/skills/` path is especially important because both Codex and Gemini treat it as a first-class interoperability location.

The key design requirement is transparency. A linked skill should not imply identical behavior unless activation, metadata, permissions, resources, and host dependencies are equivalent. Claudine’s portability classification should explain what moved unchanged, what was translated, what was ignored, and what still requires provider-specific maintenance.
