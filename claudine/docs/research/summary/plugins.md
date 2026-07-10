---
sequence:
- name: draft
- name: iterate
- name: finalize
prompt: "Subagent definitions — named, specialized agents that a session can delegate work to, often with their own prompt, model, and tool restrictions — vary widely across agentic CLIs, from first-class definition files to nothing at all. Claudine links agent definitions across providers and also has to reason about subagent observability during wrapped runs.\n\n## Task\n\nYour task is to report on \"plugin\" support across the Agentic CLI providers. Plugins -- unfortunately -- are even less of a \"standard\" than agent definitions and slash commands.\n\n- your report should start by outlining why plugins matter to agentic processes\n- and then shift its focus to how providers differ: \n    - definition format and metadata, \n    - user/repo scopes, model and tool restriction support, \n    - configuration files and schemas\n    - invocation mechanics (CLI influences, ENV influences, config influences)\n    - asset types that can be included in a plugin\n- in the \"plugin\" space Claude Code is seen as the leader and \"standard bearer\", so:\n    - when looking at Claude Code competitors always take the time to describe how it varies from Claude Code\n\nAs background material we have plugin research documents for each provider that Claudine supports. They can be found at `@claudine/docs/research/plugins/*.md`.\n\nImportant: your final response is saved verbatim as the body of this summary document, so it must be the complete document text and nothing else — no preamble, no commentary. Never write to this document yourself.\n\n::block when=\"state.name == 'draft'\"\n- Iterate over the first three research documents to develop a point of view on how to write this document and then produce an initial draft of the document\n::end-block\n::block when=\"state.name == 'iterate'\"\n\n- Note: the initial draft has already been created — it is the body of `@claudine/docs/research/summary/subagents.md` (everything below the frontmatter); read it from there\n- Act as an orchestrator and iterate over each remaining provider's research document:\n    - provide the subagent the current draft and ask them to return an improved draft based on the research document they've been assigned\n- Once every remaining provider has been incorporated, your final response is the fully updated draft\n::end-block\n\n::block when=\"state.name == 'finalize'\"\n\nThe document has now gone through several rounds of improvement and your task is just to make sure the document is consistent in tone and detail and that nothing looks incorrect or incomplete. The current draft is the body of `@claudine/docs/research/summary/plugins.md` (everything below the frontmatter); read it from there, make any adjustments, and your final response will be considered the finalized summary document.\n::end-block"
success:
  message: Summary research for **plugins** is now complete!
  success: 'Summary research for **plugins** is now complete: `./docs/research/summary/plugins.md`'
failure:
  message: 'The summary research for **plugins** failed to complete: {{ err.message }}'
$schema:
  file: file(required;eager)
file: docs/research/summary/plugins.md
last_updated: 2026-07-09
hash: 6f9d868be700fa5c-dd47e8289f030cef
---
# Plugin Support Across Agentic CLI Providers

Plugins matter because agentic workflows rarely consist of one prompt or tool. A reusable workflow may need instructions, commands, specialist agents, MCP servers, hooks, policy rules, helper executables, credentials, and reference assets to operate together. Packaging those pieces makes a workflow easier to discover, install, version, enable, share, update, and remove coherently.

Plugins are also among the most security-sensitive extension surfaces in an agentic CLI. A plugin may alter model context, register or replace tools, start local processes, execute hooks, receive credentials, or update independently of the CLI. Mature support therefore needs more than an extension directory:

- a package and metadata contract,
- user, repo, managed, and session scopes,
- lifecycle and version management,
- invocation and activation rules,
- asset discovery and namespacing,
- model and tool restrictions,
- configuration and credential handling,
- distribution,
- trust and execution boundaries.

Claude Code is the standard bearer. It offers the broadest declarative container and the most complete combination of marketplaces, versioned caching, persistent data, project recommendations, local and managed scopes, session loading, dependencies, namespacing, validation, and lifecycle commands.

Competitors use “plugin” for substantially different abstractions:

- Codex closely follows Claude Code's declarative package and marketplace model.
- Gemini CLI and Qwen Code use flat declarative extension containers.
- Pi uses npm, Git, URL, or local packages that combine passive resources with executable TypeScript extensions.
- Antigravity has an agent-plugin directory format, but its IDE, 2.0, and CLI surfaces are only partially aligned.
- Goose confirms only skills and hooks despite accepting broader manifest vocabulary.
- Kimi Code manages user-scoped runtime and tool bundles.
- OpenCode plugins are executable JavaScript or TypeScript modules.
- Kilo combines runtime modules with a separate Marketplace that extracts individual resources.

## Provider Shape

| Provider    | Support                      | Definition                                                        | Storage and scopes                                                                   | Main contents                                                                                                  | Difference from Claude Code                                                                                     |
|-------------|------------------------------|-------------------------------------------------------------------|--------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------|
| Claude Code | First-class; reference point | Optional `.claude-plugin/plugin.json`                             | Versioned cache; user, project, local, managed, skills-directory, and session scopes | Skills, commands, agents, hooks, MCP, LSP, monitors, themes, styles, channels, config, executables, and assets | Baseline                                                                                                        |
| Codex       | First-class, experimental    | `.codex-plugin/plugin.json`                                       | Versioned user cache; user enablement; repo and user marketplaces                    | Skills, commands, agents, hooks, MCP, ChatGPT apps, executables, and assets                                    | Similar architecture, but fewer scopes and a smaller manifest, validation, and lifecycle surface                |
| Gemini CLI  | First-class extension        | Required `gemini-extension.json`                                  | One user installation directory; user/workspace enablement; session selection        | Skills, TOML commands, agents, hooks, MCP, themes, policies, context, settings, scripts, and assets            | Flat GitHub-oriented extension rather than a versioned marketplace package; stronger environment filtering      |
| Qwen Code   | First-class extension        | Native `qwen-extension.json`; converts Gemini and Claude packages | One user installation directory; user/workspace enablement; session selection        | Skills, commands, agents, hooks, MCP, LSP, channels, context, settings, and executables                        | Broad assets and compatibility conversion, but no native persistent marketplace registry and weaker namespacing |
| Pi          | First-class package          | `package.json` with optional `pi` block or conventions            | User/project settings; npm, Git, and local stores; temporary session packages        | TypeScript extensions, skills, prompts, themes, dependencies, and passive assets                               | Source-oriented executable package without Claude's automatic package namespace                                 |
| Antigravity | Partial but structured       | `plugin.json` plus conventional files                             | Global, workspace, and CLI staging directories; bundled IDE customizations           | Skills, rules, MCP, hooks, and partial agents, commands, and assets                                            | Genuine container, but lifecycle, precedence, versioning, publishing, and agent schema remain incomplete        |
| Goose       | Partial                      | Optional Open Plugins, Goose, or Gemini manifest                  | User/repo directories; settings deny list                                            | Skills, hooks, scripts, and supporting assets                                                                  | Broad accepted vocabulary, but only skills and hooks are confirmed at runtime                                   |
| Kimi Code   | Partial but managed          | `kimi.plugin.json` or `.kimi-plugin/plugin.json`                  | User registry and managed copies; official/custom marketplace                        | Skills, commands, MCP, hooks, session-start skill, instructions, and assets                                    | User-scoped runtime/tool bundle with no project plugin or packaged subagent support                             |
| OpenCode    | Partial; code-first          | `.ts`/`.js` module or npm package; no plugin manifest             | User/repo files and layered config                                                   | Hooks, tools, auth, provider/model hooks, config mutation, and TUI extensions                                  | Executable runtime API rather than a declarative content container                                              |
| Kilo Code   | Partial; dual mechanism      | Runtime module/npm package plus Marketplace resource manifests    | User/repo runtime config; Marketplace extraction into normal resource locations      | Runtime hooks, tools, auth, providers, and TUI; Marketplace skills, agents, and MCP                            | Runtime modules plus a resource catalog, not one retained package                                               |

“Plugin” is therefore not a stable capability label. It may mean a governed content package, a source package with executable modules, a compatibility conversion target, a small skills/hooks bundle, an IDE-managed customization, or arbitrary runtime code.

## Definition Format and Metadata

### Claude Code: the benchmark

Claude Code conventionally uses `.claude-plugin/plugin.json`, although the manifest is optional. When present, only `name` is required.

Its metadata can include:

- `$schema`,
- `name`, `displayName`, and `version`,
- description, author, homepage, repository, license, and keywords,
- `defaultEnabled`,
- skills, commands, and agents,
- hooks, MCP servers, and LSP servers,
- output styles, themes, monitors, and channels,
- `userConfig`,
- dependencies.

Paths are relative to the plugin root and begin with `./`. Merge rules differ by asset family. Claude Code publishes a SchemaStore schema and provides strict native validation.

This combination—broad metadata, machine-readable schema, native validation, component inventory, and package lifecycle—is the standard competitors most often match only in part.

### Codex

Codex uses `.codex-plugin/plugin.json` with required `name`. It supports common package metadata plus skills, apps, MCP servers, and hooks; commands and agents also follow recognized package conventions.

Its `interface` object adds installation-surface descriptions, category, capabilities, privacy and terms URLs, default prompts, branding, and screenshots. A separate `.app.json` maps plugin-local app names to ChatGPT connectors.

Codex is structurally closest to Claude Code. Its main differences are a narrower manifest, user-oriented enablement, restart-based activation, app/connector integration, and no documented public plugin schema or strict validator equivalent.

### Gemini CLI

Gemini requires `gemini-extension.json`; `name` and `version` are required, and the name must match the directory.

Optional fields include:

- `mcpServers`,
- `contextFileName`,
- `excludeTools`,
- `migratedTo`,
- `plan`,
- `settings`,
- `themes`.

Gemini provides native validation and substitutions such as `${extensionPath}` and `${workspacePath}`. Compared with Claude Code, it has a smaller manifest and less package-level metadata, but stronger first-class settings, environment filtering, and tool-exclusion fields.

### Qwen Code

Qwen requires `qwen-extension.json` and supports commands, skills, agents, MCP/LSP servers, hooks, channels, context, settings, and `excludeTools`.

It can convert Gemini and Claude packages during installation. Conversion copies or rewrites recognized resources and emits a Qwen manifest.

That bridge is useful, but it is not semantic equivalence. Namespaces, precedence, hook payloads, trust, tool names, scripts, and unsupported fields may change. Claudine should preserve the original provider and conversion provenance.

### Pi

Pi uses the standard npm `package.json`; no Pi-specific field is required. A package may declare:

- `pi.extensions`,
- `pi.skills`,
- `pi.prompts`,
- `pi.themes`,
- `pi.image`,
- `pi.video`.

Paths support globs and exclusions. Without a `pi` block, Pi discovers conventional `extensions/`, `skills/`, `prompts/`, and `themes/` directories.

Unlike Claude Code, Pi uses a general-purpose package manifest instead of a dedicated agentic schema. Several capabilities exist only through executable extension modules rather than declarative fields, and Pi does not publish a complete schema for the `pi` object.

### Antigravity

Antigravity uses `plugin.json` as the root marker, but its documentation is inconsistent:

- Antigravity 2.0 and IDE documentation say `name` may be omitted and default to the directory name.
- CLI documentation and the published schema require `name`, allow optional `$schema` and `description`, and reject additional properties.

The CLI schema has no documented fields for version, author, license, dependencies, engines, permissions, activation events, or component paths. Components are instead discovered by convention:

- `mcp_config.json`,
- `hooks.json`,
- `skills/<name>/SKILL.md`,
- `rules/<name>.md`,
- an `agents/` directory mentioned only by CLI documentation.

Compared with Claude Code, Antigravity's manifest is primarily an identity marker. Its capabilities come from fixed filenames and directories rather than a component inventory. The `agents/` schema is undocumented, so subagent portability cannot yet be assumed.

### Goose

Goose accepts optional Open Plugins, Goose, or Gemini manifests. Those manifests can contain vocabulary for commands, agents, skills, rules, hooks, MCP, LSP, and output styles, but Goose confirms only skills and hooks as plugin-provided runtime resources.

Compared with Claude Code, manifest parsing is broader than the implemented container. Claudine must not infer support merely because Goose accepts a field.

### Kimi Code

Kimi accepts `kimi.plugin.json` or `.kimi-plugin/plugin.json`. Its manifest can describe interface metadata, skills, commands, MCP servers, hooks, a session-start skill, and skill instructions.

Unsupported fields such as `tools`, `apps`, `inject`, and `configFile` are diagnosed and ignored. Compared with Claude Code, Kimi's container is smaller, user-scoped, and centered on session initialization and tool bundles rather than a general package ecosystem.

### OpenCode and Kilo

OpenCode has no plugin manifest. A plugin function and the `@opencode-ai/plugin` types form its runtime contract.

Kilo runtime plugins similarly use JavaScript or TypeScript module exports and npm metadata. Its Marketplace uses separate manifests for skills, agents, and MCP servers. Installing a Marketplace item extracts it into the corresponding normal resource location; it does not preserve a unified plugin container.

Both differ fundamentally from Claude Code's declarative, namespaced package. They expose executable extension APIs whose behavior cannot be understood from passive metadata alone.

## Scopes, Storage, Discovery, and Precedence

### Claude Code

Claude Code supports:

- user scope,
- shared project scope,
- gitignored local project scope,
- managed/system scope,
- session-only loading,
- user and repo skills-directory packages.

Installed versions live under a marketplace/plugin/version cache. Persistent plugin data is stored separately. Session packages can override installed packages, and managed settings govern lower scopes.

Plugin skills, commands, and agents are namespaced with the plugin identity. This substantially reduces collisions and gives Claudine a stable package-to-resource relationship.

### Codex

Codex uses a versioned user cache and user `config.toml` enablement. Repo and user catalogs advertise packages, but a repo catalog does not establish repo-scoped activation.

Compared with Claude Code, Codex lacks equivalent local-project, managed, skills-directory, and arbitrary session package scopes. Plugin changes are also more restart-oriented.

### Gemini CLI and Qwen Code

Gemini and Qwen store installed packages in user extension directories while supporting user/workspace enablement and session selection. The extension itself remains user-installed; workspace state controls whether it is active.

Both generally give user and project resources precedence over extension resources. Unlike Claude Code's consistently qualified plugin resources, extension commands may receive a qualifier only when a conflict occurs, while skills or agents may be shadowed.

Qwen's conversion support does not preserve Claude's namespace contract. A converted package may therefore collide with resources that were distinct under Claude Code.

### Pi

Pi has genuine user and project package scopes:

- user packages in `~/.pi/agent/settings.json`,
- project packages in `.pi/settings.json`,
- session extensions supplied with `-e` or `--extension`.

npm, Git, and local packages use different stores. Project content is gated by project trust.

Project and user resources take precedence over package resources. Package resources are not automatically namespaced:

- extension commands use their registered names,
- skills use `/skill:name`,
- prompt templates use `/filename`.

This is a major difference from Claude Code's package namespace.

### Antigravity

Antigravity documents several agent-plugin roots:

- global: `~/.gemini/config/plugins/<name>/`,
- CLI staging: `~/.gemini/antigravity-cli/plugins/<name>/`,
- workspace: `.agents/plugins/<name>/`,
- alternate workspace: `_agents/plugins/<name>/`.

Antigravity 2.0 and the IDE scan global and workspace folders. CLI-installed or imported plugins are staged in the CLI root. Google-built bundled plugins can also be enabled through the IDE Customizations UI, although their storage and update behavior are undocumented.

Precedence remains unspecified:

- workspace versus global,
- plugin versus standalone skills or rules,
- duplicate plugin names,
- contained component collisions,
- built-in versus plugin ordering.

Claude Code is substantially more deterministic. Claudine should record Antigravity scope and origin without inventing shadowing rules.

The observed `~/.antigravity/extensions/` directory is separate. It contains VS Code-style IDE extensions, not Antigravity agent plugins, and must not be included in agent-plugin discovery.

### Goose, Kimi Code, OpenCode, and Kilo Code

Goose discovers plugins from user and repo directories and supports a settings deny list. Open Plugin skills receive a plugin namespace, while converted Gemini skills retain their original names. User/repo collision behavior is not fully documented.

Kimi installs plugins only into a user registry and managed-copy tree. Commands are namespaced as `<plugin>:<command>`, but skills use their declared names.

OpenCode loads global and project runtime modules through layered configuration. It provides no package resource namespace, and custom tools can override built-ins by name.

Kilo loads built-ins, global config plugins, global plugin files, project config plugins, and project plugin files in a defined order. Marketplace resources follow normal resource precedence after extraction rather than retaining a package namespace.

## Models, Tools, MCP, and Restrictions

Plugin-wide model selection is uncommon. Packaging an agent can preserve whatever model and tool controls that provider's agent-definition format supports, but those are component-level controls, not a general plugin policy.

| Control                    | Claude           | Codex            | Gemini                  | Qwen                                   | Pi                           | Antigravity                        | Goose          | Kimi                        | OpenCode                       | Kilo                                 |
|----------------------------|------------------|------------------|-------------------------|----------------------------------------|------------------------------|------------------------------------|----------------|-----------------------------|--------------------------------|--------------------------------------|
| Package-wide model choice  | No               | No               | No                      | No                                     | Extension-mediated           | No                                 | No             | No                          | Runtime-mediated               | Runtime-mediated                     |
| Package-wide tool denial   | No general field | No general field | `excludeTools`          | `excludeTools`                         | CLI/extension-mediated       | No manifest field                  | None confirmed | Unsupported `tools` ignored | Runtime code can replace tools | Runtime code can replace tools       |
| Packaged agents            | Yes              | Yes              | Preview                 | Yes                                    | No native asset              | Partial; schema unknown            | No             | No                          | Separate resource              | Marketplace resource                 |
| Packaged MCP               | Yes              | Yes              | Yes                     | Yes                                    | Extension-mediated           | Yes                                | Not confirmed  | Yes                         | Separate config                | Marketplace resource                 |
| Provider permission system | Yes              | Yes              | Yes, including policies | Yes                                    | No built-in permission popup | Allow/ask/deny plus hook decisions | Limited        | MCP approval                | Underlying tool policy         | Normal Kilo policy plus runtime code |
| Plugin-specific sandbox    | No               | No               | No                      | No; global sandbox can include helpers | No                           | No; global terminal sandbox        | No             | No                          | No                             | No                                   |

Gemini and Qwen stand out for `excludeTools`, which can disable tools or command patterns at the extension/session level. This is more direct than Claude Code's plugin manifest, although it is still not a plugin sandbox.

Antigravity's manifest cannot select models or declare tool allowlists. MCP servers add tools, while hooks can participate in permission decisions. Its CLI permission engine uses Deny > Ask > Allow for resources such as commands, unsandboxed execution, MCP tools, files, and URLs. A `PreToolUse` hook can return `allow`, `deny`, `ask`, or `force_ask`.

That gives Antigravity hooks more operational authority than a passive Claude skill, but it is provider-specific hook behavior rather than a portable package policy. Its terminal sandbox is global or session-level, not plugin-specific.

OpenCode and Kilo runtime modules have the broadest programming authority. They can register or replace tools and influence provider/model behavior directly, but do so through trusted executable code rather than declarative restrictions.

## Configuration Files and Schemas

| Provider    | Package/runtime definition                          | Enablement and state                                    | Main configuration surfaces                                               | Schema posture                                                    |
|-------------|-----------------------------------------------------|---------------------------------------------------------|---------------------------------------------------------------------------|-------------------------------------------------------------------|
| Claude      | Optional plugin manifest                            | Scoped settings, registries, cache, and persistent data | MCP, LSP, hooks, monitors, options, and dependencies                      | Public SchemaStore schema plus strict native validation           |
| Codex       | Plugin manifest                                     | User `config.toml`, cache, and marketplace snapshots    | Apps, MCP policy, hooks, and ChatGPT state                                | No documented public plugin schema or Claude-equivalent validator |
| Gemini      | Required extension manifest                         | Install metadata and user/workspace enablement          | Hooks, policies, commands, context, settings, and `.env`                  | Native validation; dedicated manifest contract                    |
| Qwen        | Native or converted manifest                        | Install metadata and enablement registry                | Commands, agents, MCP/LSP, hooks, channels, context, settings, and `.env` | Native format plus conversion logic                               |
| Pi          | `package.json` plus optional `pi` object            | User/project package entries and resource filters       | Extensions, skills, prompts, themes, and dependencies                     | npm schema covers the package; no complete published `pi` schema  |
| Antigravity | Minimal `plugin.json`                               | CLI registry/staging or scanned directory presence      | `mcp_config.json`, `hooks.json`, skills, rules, and undocumented agents   | Published manifest schema is strict but extremely small           |
| Goose       | Optional multi-dialect manifest                     | Directory presence and deny list                        | Skills, hooks, and scripts                                                | Accepted schema vocabulary exceeds confirmed runtime behavior     |
| Kimi        | Two manifest locations                              | `installed.json` and managed copies                     | Inline MCP/hooks and session-start behavior                               | Provider-specific manifest with diagnostics for ignored fields    |
| OpenCode    | No manifest                                         | File presence or config array                           | Executable hooks and layered config                                       | TypeScript runtime types are the effective contract               |
| Kilo        | Module/npm identity; Marketplace resource manifests | Runtime presence/config and extracted resources         | Kilo config, skills, agents, commands, and MCP                            | Separate runtime and resource schemas                             |

Antigravity's `mcp_config.json` and `hooks.json` carry most executable behavior. MCP definitions may include local commands, arguments, environment variables, working directories, remote URLs, headers, authentication types, OAuth credentials, and Google ADC configuration. Hook definitions include event handlers, commands, and timeouts.

These files are configuration, but they are also executable and credential-bearing security surfaces. They should never be treated as passive metadata.

## Invocation Mechanics

Plugin behavior is influenced by three distinct channels:

1. CLI operations that install, select, suppress, or reload plugins.
2. Environment variables that relocate state, inject credentials, alter discovery, or disable extensions.
3. Persistent configuration that controls enablement, precedence, permissions, and resource filtering.

| Provider    | Important CLI influence                                                                                                                                      | Important environment influence                                                                                                       | Important config influence                                                                    |
|-------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------|
| Claude      | Full install/update/remove/enable/disable/list/details/validate/prune/tag/marketplace lifecycle; `--plugin-dir`, `--plugin-url`, `--bare`, and `--safe-mode` | Cache/seed paths, synchronous installation, Git timeouts, marketplace refresh, auto-update, safe/bare mode, and private-source tokens | Scoped `enabledPlugins`, plugin config, known marketplaces, and managed restrictions          |
| Codex       | `codex plugin add/remove/list`, marketplace management, and TUI browsing                                                                                     | Credential and connector environment inherited by plugin resources; no comparably broad plugin-specific env surface documented        | User `config.toml` controls enablement and plugin-scoped MCP policy                           |
| Gemini      | Install/update/uninstall/list/enable/disable/link/new/validate/config/migrate; `--extensions` limits session selection                                       | `GEMINI_CLI_HOME`, sandbox and workspace-trust variables, declared extension settings, model/API variables, and telemetry controls    | User/workspace enablement, settings, policies, MCP, hooks, and filtered environment injection |
| Qwen        | Install/update/uninstall/list/enable/disable/link/new/settings; `--extensions`, `--bare`, and disabled-command controls                                      | `QWEN_CODE_SIMPLE`, sandbox variables, disabled commands, Git/npm credentials, and runtime directory                                  | Enablement registry, workspace settings, keychain-backed secrets, MCP/LSP, hooks, and context |
| Pi          | Install/remove/list/update/config; user/project selection; `-e`; per-resource disable flags                                                                  | `PI_CODING_AGENT_DIR`, package/session directories, offline mode, telemetry, and Git behavior                                         | User/project `packages` entries and filters for extensions, skills, prompts, and themes       |
| Antigravity | List/install/uninstall/enable/disable plus locally observed import/validate/link commands                                                                    | No plugin-specific variables documented                                                                                               | Scanned paths, CLI enablement state, permission rules, and global/session sandbox settings    |
| Goose       | Install and update; skills invoked through `/skills`                                                                                                         | No plugin-specific variables documented                                                                                               | User/repo presence and plugin deny list                                                       |
| Kimi        | In-session `/plugins` management, reload, marketplace, and per-plugin MCP toggles                                                                            | `KIMI_CODE_HOME` and `KIMI_CODE_PLUGIN_MARKETPLACE_URL`                                                                               | User registry, managed copies, plugin enablement, and MCP state                               |
| OpenCode    | Module install command where available; `--pure`; debug-config inspection                                                                                    | Config path/content overrides, default-plugin suppression, auto-update controls, and Claude-compatibility suppression                 | Layered config arrays and global/project plugin directories                                   |
| Kilo        | `kilo plugin`, global/force options, `--pure`, plus separate Marketplace UI operations                                                                       | Config path/content, pure mode, project/default-plugin suppression, permission overlay, and XDG roots                                 | Global/project plugin arrays, local module directories, and extracted Marketplace resources   |

Claude Code provides the clearest separation between package lifecycle, session loading, persistent scoped enablement, and managed policy. Other providers often combine these concerns or expose only part of the lifecycle.

Antigravity exposes numerous lifecycle verbs, but it has no documented plugin update command, version pin, lockfile, safe mode, audit, or provenance command. Its general `agy update` updates the CLI rather than individual plugins.

Kimi's lifecycle is TUI-oriented. OpenCode and Kilo are primarily presence/config-driven: executable modules load because their files or package entries are present, while `--pure` suppresses external runtime plugins.

## Asset Types

| Asset                  | Claude                     | Codex                 | Gemini                | Qwen                    | Pi                              | Antigravity                                                         | Goose               | Kimi                       | OpenCode plugin      | Kilo runtime/Marketplace          |
|------------------------|----------------------------|-----------------------|-----------------------|-------------------------|---------------------------------|---------------------------------------------------------------------|---------------------|----------------------------|----------------------|-----------------------------------|
| Skills                 | Packaged                   | Packaged              | Packaged              | Packaged                | Packaged                        | Packaged                                                            | Packaged            | Packaged                   | Separate             | Marketplace skill                 |
| Commands               | Markdown                   | Markdown              | TOML                  | Markdown or legacy TOML | Prompts plus extension commands | Skills become commands; workflows are outside the documented layout | Separate recipes    | Markdown                   | Separate             | Normal workflows/commands         |
| Subagents              | Packaged                   | Packaged              | Preview               | Packaged                | Extension-mediated only         | Partial; undocumented `agents/` schema                              | No                  | No                         | Separate             | Marketplace/custom agents         |
| MCP                    | Packaged                   | Packaged              | Packaged              | Packaged                | Extension-mediated              | Packaged `mcp_config.json`                                          | Unconfirmed         | Packaged                   | Separate config      | Marketplace MCP                   |
| LSP                    | Packaged                   | No researched support | No researched support | Packaged                | Extension-mediated              | No documented plugin LSP                                            | Unconfirmed         | No                         | Not a package asset  | No Marketplace type               |
| Hooks                  | Packaged config            | Packaged config       | Packaged config       | Packaged config         | Extension events                | Packaged `hooks.json`                                               | Packaged config     | Packaged config            | Core runtime API     | Core runtime API                  |
| Rules/policies         | No dedicated rule asset    | MCP policy            | Policy TOML           | `excludeTools`          | Extension-mediated              | Packaged Markdown rules                                             | Unconfirmed         | No                         | Runtime code         | Runtime code                      |
| Custom tools/providers | Servers and assets         | Apps and MCP          | MCP                   | MCP and channels        | Direct extension API            | MCP plus hook permission decisions                                  | Separate extensions | MCP                        | Direct API           | Direct API                        |
| Themes/TUI             | Themes and styles          | Interface metadata    | Themes                | Limited                 | Themes and extension UI         | Separate IDE extensions, not agent-plugin assets                    | No                  | No                         | TUI hooks            | TUI modules and themes            |
| Persistent context     | No dedicated generic asset | No                    | `GEMINI.md`           | `QWEN.md`               | Prompt templates                | Skills and rules                                                    | No                  | Session-start instructions | Separate             | Agents, commands, and skills      |
| Executables            | Scripts and `bin/`         | Scripts and `bin/`    | Hook/MCP code         | MCP/LSP/channel code    | TypeScript extensions           | Hook commands and MCP processes                                     | Hook scripts        | MCP/hook commands          | Module and Bun shell | Runtime modules and skill scripts |
| Static assets          | Full                       | Full                  | Full                  | Partial                 | Full                            | Partial through contained resources                                 | Partial             | Full                       | No container         | Resource/package-specific         |

Claude Code's advantage is not that every individual asset type is unique. Its advantage is that these assets participate in one documented, namespaced, versioned, inspectable package.

Pi, OpenCode, and Kilo can be more programmable than Claude plugins because executable modules can register arbitrary behavior. That programmability comes with weaker portability and a larger trust surface.

Antigravity's strongest portable asset is the Agent Skill. Rules are prompt-like but provider-specific. MCP and hooks are structurally discoverable but executable and credential-bearing. Its undocumented agent format should be treated as unknown rather than optimistically portable.

## Distribution, Versioning, and Lifecycle

### Claude Code

Claude Code supports official, community, and private marketplaces; multiple source types; versioned caches; per-plugin lifecycle; persistent data; dependencies; pruning; and reload.

It is the clearest example of a plugin ecosystem rather than merely an extension loader.

### Codex

Codex supports curated, repo, and personal marketplaces with versioned cached installations. Compared with Claude Code, its lifecycle is smaller: enablement is config/TUI-driven, updates occur primarily through marketplace refresh, and restart is generally required.

### Gemini CLI

Gemini is GitHub-centered and supports install metadata, explicit updates, optional auto-update, linking, validation, migration, and user/workspace enablement. It lacks Claude's marketplace registry and versioned cache model.

### Qwen Code

Qwen installs from Git, npm, archives, local paths, and Claude/Gemini sources. It has no equivalent native persistent marketplace registry. Its breadth comes from source support and conversion rather than Claude-style marketplace governance.

### Pi

Pi is source-centric through npm, Git, local paths, and a gallery that indexes packages rather than acting as an installable catalog. npm and Git provide version identity, while local and session sources remain more fluid.

### Antigravity

Antigravity exposes Google-built bundled plugins, manual global/workspace folders, local or remote CLI installation, `plugin@marketplace` syntax, import from Gemini or Claude, and marketplace link generation.

It does not document a complete public publishing workflow, package ownership model, version field, update channel, pinning, rollback, signature, or provenance system. It therefore has distribution entry points without Claude Code's fully specified marketplace governance.

### Goose, Kimi Code, OpenCode, and Kilo Code

Goose distributes through Git or manual copy and documents install/update more clearly than removal or version governance.

Kimi has official and custom marketplaces plus GitHub, archive, and local-path installation.

OpenCode uses npm or local executable code and community lists rather than a declarative marketplace package.

Kilo uses npm or local runtime modules plus a curated resource Marketplace. Marketplace resources lose their original container identity after extraction unless provenance is separately retained.

## Security and Trust

All providers treat extensions as highly trusted, but their controls differ.

Claude Code provides managed source restrictions, workspace trust, blocklists, MCP approval, secure option storage, and version-aware package state.

Codex adds hook review, plugin-scoped MCP identity restrictions, and ChatGPT connector OAuth. Plugin-owned hooks, scripts, MCP servers, and executables are not protected by the model's shell sandbox.

Gemini adds allowlists, hook fingerprinting, environment filtering, keychain settings, `excludeTools`, and restrictive policies.

Qwen adds installation consent, workspace trust, stripped MCP trust, keychain settings, tool exclusions, and optional sandboxing that can include spawned helper processes.

Pi relies on project trust and source/package review. Extensions run with process privileges and have no built-in plugin sandbox or permission-popup system.

Antigravity has no documented plugin-specific signature, trust review, audit, or sandbox. Trust depends on the resource:

- skills and rules influence instructions,
- hooks execute commands and can affect permission decisions,
- MCP servers launch processes or contact remote services,
- hook inputs expose conversation, workspace, transcript, and artifact paths,
- MCP configuration may contain credentials and headers.

Its global permission engine and terminal sandbox provide operational controls, but they do not authenticate the package or establish provenance.

Goose relies mainly on source trust. Kimi adds source tiers, confirmation, path confinement, MCP toggles, and diagnostics. OpenCode and Kilo runtime modules have broad direct code authority and should be reviewed like application code, not passive configuration.

## Portability and Claudine Linking

Claudine should distinguish at least:

- `declarative_container`,
- `limited_container`,
- `converted_container`,
- `runtime_module`,
- `resource_marketplace_entry`,
- `extracted_resource`,
- `ide_extension`.

The generally portable core is:

- Agent Skills,
- command or prompt intent,
- documented subagent prompts,
- passive context or rule text,
- documentation and static assets.

Executable and configuration surfaces require provider-specific review:

- hooks,
- MCP and LSP servers,
- authentication and credentials,
- scripts and binaries,
- runtime modules,
- environment substitutions,
- permission policies,
- marketplace and installation state.

Provider-specific implications include:

- Claude Code and Codex containers can usually be retained as native package units while supported declarative resources are linked individually.
- Gemini and Qwen extensions require precedence, settings, environment, and namespace rewrites.
- Qwen and Antigravity compatibility imports must preserve source-provider and transformation provenance and report dropped or rewritten semantics.
- Pi packages should remain Pi-native source units; passive skills, prompts, themes, and assets may be extracted separately.
- Goose manifests must be reduced to runtime-confirmed skills and hooks.
- Kimi packages remain user-scoped native bundles; their commands and skills follow different naming rules.
- OpenCode and Kilo runtime modules should remain provider-native executable integrations.
- Kilo Marketplace items must be modeled separately from the normal resources they install.
- Antigravity `skills/*/SKILL.md` is portable after metadata validation; `rules/*.md` requires semantic mapping; `mcp_config.json` requires secret stripping and transport review; `hooks.json` is provider-specific; `agents/` remains unknown.
- `~/.antigravity/extensions/` must not be scanned as an agent-plugin directory.

## Point of View

Claude Code currently expresses the broadest declarative meaning of plugin: a namespaced, marketplace-distributed, versioned package that can affect nearly every layer of an agentic session.

Codex meaningfully matches that architecture but is more app-centric, user-scoped, config-driven, and restart-oriented. Gemini favors a required flat extension, explicit precedence, persistent context, restrictive policy, and controlled settings injection. Qwen adds broad assets and conversion from Claude and Gemini sources, but that compatibility is translational rather than semantic.

Pi is a first-class package ecosystem with a different center of gravity: npm and Git sources, project trust, resource filters, prompt templates, themes, and executable TypeScript extensions. It is more programmable than Claude Code's declarative container but less standardized around agents, MCP, permissions, and namespaces.

Antigravity has a genuine agent-plugin directory with skills, rules, MCP, hooks, and partial subagent support. It also has CLI import/install operations and IDE-managed bundled customizations. Its manifest is minimal, however; its IDE and CLI documentation diverge, precedence and runtime namespacing are unspecified, and no complete versioning or publishing model is documented.

Goose's confirmed runtime is principally skills and hooks. Kimi is a managed user-scoped runtime/tool bundle. OpenCode and Kilo represent the code-first branch, with Kilo additionally extracting Marketplace resources into normal locations.

Claudine should therefore link supported declarative resources, not plugin labels. It should preserve native package and conversion provenance, distinguish IDE extensions and runtime modules from portable containers, and never treat compatibility import or accepted manifest vocabulary as proof that plugin semantics are interchangeable.
