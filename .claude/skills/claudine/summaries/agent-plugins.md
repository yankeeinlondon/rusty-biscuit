# Agentic CLI Plugin and Extension Ecosystems

Plugin ecosystems matter because agentic CLIs are becoming runtime platforms, not just prompt runners. A plugin can install reusable instructions, slash commands, subagents, MCP servers, hooks, scripts, UI assets, auth connectors, provider settings, and policy. That means a plugin can change what an agent knows, which tools it can call, what events fire around a session, what credentials are reachable, and which commands users can invoke.

That overlaps directly with Claudine's cross-provider resource-linking model. Claudine already treats skills, commands, agents, scripts, hooks, and MCP servers as provider-shaped resources that can be discovered, classified, linked, or rewritten across CLIs. Plugins complicate that model because they bundle portable resources together with provider-specific executable surfaces. A `SKILL.md` inside a plugin may be broadly portable; the plugin manifest, hook payload schema, MCP process environment, namespace syntax, install metadata, and update behavior usually are not.

The useful distinction is therefore not simply "does this provider have plugins?" It is:

- Does the provider have an installable unit?
- Is that unit declarative, executable, or mixed?
- Which contained resources can Claudine safely extract?
- Which resources must stay provider-local?
- Does the provider namespace plugin resources in a way Claudine must preserve or rewrite?
- Can the plugin system serve as a distribution channel for Claudine-managed resources?

## Provider Patterns

Claude Code, Codex, Gemini CLI, Pi, and Qwen Code have first-class plugin, extension, or package containers. They use manifests or package metadata, install flows, managed install state, and explicit startup or reload discovery. These are the strongest candidates for Claudine-aware import and export, but they are not interchangeable formats.

Kimi has a documented plugin system, but it is narrower. Plugins are user-scoped, managed through the `/plugins` TUI rather than a shell `kimi plugin` subcommand, and can package skills, slash commands, MCP servers, hooks, session-start skill references, skill instructions, interface metadata, and static assets. They do not support plugin-defined subagents.

Goose has a real but narrow plugin model. Plugins are directories discovered from `~/.agents/plugins/<name>/` and `.agents/plugins/<name>/`; they may contain a manifest plus `skills/` and `hooks/`. Goose follows the Open Plugins shape and accepts Open Plugins and Gemini-style manifest names, but Goose only documents support for skills and hooks. Slash commands, subagents, MCP servers, recipes, and extensions are configured outside plugins.

OpenCode is different. Its plugins are executable TypeScript/JavaScript modules loaded at startup from the `plugin` array in `opencode.json`, from npm packages installed by Bun, or from local plugin directories. There is no dedicated plugin manifest or declarative container format. Plugins can register hooks, custom tools, auth providers, provider/model hooks, config mutation hooks, shell environment hooks, and TUI extensions, but they do not package skills, slash commands, subagents, MCP servers, prompts, or assets as extractable plugin resources. Claudine should treat OpenCode plugins as provider-local runtime code, not as cross-provider resource bundles.

Pi packages are source-based npm/git/local package roots, not a marketplace-catalog plugin format. A package can expose TypeScript extensions, Agent Skills, prompt templates, themes, and arbitrary supporting assets through `package.json` `pi` fields or conventional directories. Pi core does not provide native MCP or subagents; those surfaces can exist only through executable extensions, so they should be treated as provider-local code rather than portable manifest resources.

Kilo has two separate extension surfaces. Runtime plugins are TypeScript/JavaScript modules loaded from config, npm packages, or scanned `plugin/` / `plugins/` directories; they can register hooks, tools, auth/model providers, TUI behavior, shell environment hooks, and request mutations. Kilo Marketplace is different: it is a catalog for skills, agents, and MCP servers, and installation extracts those resources into normal Kilo locations rather than preserving a plugin container. Runtime plugins are non-portable executable code; Marketplace resources are potential extraction candidates.

## Comparison

| Provider    | Ecosystem                                         | Manifest / format                                                                                                                                                                         | Main install model                                                                                                                                                | Portable resource potential                                                                                                                             |
|-------------|---------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------|
| Claude Code | First-class plugins                               | `.claude-plugin/plugin.json`                                                                                                                                                              | Marketplace, Git, local directory, session plugin                                                                                                                 | High: skills, commands, agents, assets                                                                                                                  |
| Codex       | First-class plugins                               | `.codex-plugin/plugin.json` plus marketplace JSON                                                                                                                                         | Plugin marketplace, repo/personal marketplaces, local directory                                                                                                   | High: skills, commands, agents, assets                                                                                                                  |
| Gemini CLI  | First-class extensions                            | `gemini-extension.json`                                                                                                                                                                   | GitHub repo/release, local directory/link                                                                                                                         | High: skills, commands, agents, assets                                                                                                                  |
| Goose       | Partial plugins                                   | `plugin.json`, `.plugin/plugin.json`, `.goose-plugin/plugin.json`, or `gemini-extension.json`; manifest optional, `name` only required when present                                       | `goose plugin install <git-url>`, `goose plugin update <name>`, or manual copy into `.agents/plugins/`                                                            | Medium: extract Agent Skills only; hooks/scripts/manifests stay Goose-local                                                                             |
| Kimi Code   | Documented user-scoped plugin system              | `kimi.plugin.json` or `.kimi-plugin/plugin.json`; required `name` only                                                                                                                    | `/plugins` TUI from marketplace, GitHub URL, zip URL, local directory, or custom marketplace JSON                                                                 | Medium-high: skills, commands, static assets; no subagents                                                                                              |
| OpenCode    | Runtime plugins                                   | No dedicated manifest; local `.ts`/`.js` module or npm package referenced from `opencode.json` `plugin` array                                                                             | `opencode plugin <module>` / `opencode plug <module>`, config `plugin` array, npm via Bun, or local files in plugin directories                                   | Low: do not extract plugin code; use separate `.opencode/skills`, `.opencode/commands`, `.opencode/agents`, and `opencode.json` `mcp` resources instead |
| Qwen Code   | First-class extensions                            | `qwen-extension.json`; can convert `gemini-extension.json` and Claude `.claude-plugin/plugin.json` / `marketplace.json`                                                                   | `qwen extensions install` from Git, GitHub shorthand, local path/archive, archive URL, scoped npm package, Claude marketplace source, or Gemini extension Git URL | High: skills, commands, agents, `QWEN.md` context                                                                                                       |
| Pi          | First-class package ecosystem                     | `package.json` with `pi.extensions`, `pi.skills`, `pi.prompts`, `pi.themes`, or conventional `extensions/`, `skills/`, `prompts/`, `themes/` directories                                  | `pi install <source>` from npm, git/URL, or local path; `-l/--local` for project scope; package gallery lists installable source strings                          | Medium-high: skills, prompt templates, themes, passive assets; TypeScript extensions are Pi-only                                                        |
| Kilo        | Runtime plugins plus Marketplace resource catalog | Runtime plugins use npm `package.json` exports or local `.ts`/`.js` default exports; Marketplace uses `SKILL.md`, `AGENT_DEFINITION.md`, `MCP.yaml`, generated `marketplace.yaml` indexes | `kilo plugin <module>`, config `plugin` array, scanned plugin directories, Marketplace UI                                                                         | Medium: Marketplace skills/agents/MCP candidates after extraction; runtime plugins are non-portable                                                     |

## Asset Types

The most portable assets are Markdown-like, declarative, and instruction-oriented.

| Asset type                                                | Portability                                                           |
|-----------------------------------------------------------|-----------------------------------------------------------------------|
| Agent Skills                                              | Usually portable, with namespace and path rewrites                    |
| Slash commands / prompt templates                         | Usually portable, but command syntax and namespacing differ           |
| Subagents                                                 | Portable only between providers with compatible frontmatter/contracts |
| Context files such as `GEMINI.md` / `QWEN.md`             | Partially portable as provider-specific persistent context            |
| Static assets / README / examples                         | Portable as support files                                             |
| MCP server declarations                                   | Not safely portable without rewrite and credential review             |
| Hooks                                                     | Provider-specific; event schemas and execution environments differ    |
| Scripts and `bin/` executables                            | Non-portable unless explicitly reviewed                               |
| Runtime plugin code                                       | Provider-local                                                        |
| Auth connectors, app connectors, channels, TUI extensions | Provider-local                                                        |
| Themes/output styles                                      | Usually provider-local                                                |

Claude, Codex, Gemini, and Qwen package the key resource trio Claudine already understands: skills, slash commands, and agents/subagents. Kimi packages skills and slash commands, plus MCP servers, hooks, session-start skill references, skill instructions, interface metadata, and assets, but not subagents. Goose plugin assets are primarily `skills/<skill-name>/SKILL.md`; Goose hooks, scripts, manifests, `${PLUGIN_ROOT}` references, auto-update metadata, and `gemini-extension.json` are provider-local. Qwen extensions can also declare `lspServers`, `channels`, `settings`, `excludeTools`, and a `contextFileName` that defaults to `QWEN.md` when present.

## Installation and Discovery

Claude and Codex use marketplace-style plugin installation into versioned cache directories. Enablement is stored in provider settings, and plugin resources are loaded at startup or reload. Both namespace plugin resources strongly enough that Claudine must translate names when exporting them.

Gemini installs extensions into `~/.gemini/extensions/<name>/` and requires `gemini-extension.json`. It merges extension-provided skills, commands, agents, hooks, MCP servers, policies, and context into the session. User and project resources generally outrank extension resources.

Goose scans `~/.agents/plugins/<name>/` and project-local `.agents/plugins/<name>/` at session startup. Git-backed plugins install with `goose plugin install <git-url>` and update with `goose plugin update <plugin-name>`; local plugins are copied into the plugin directory. Disablement is config-driven via `disabledPlugins` in Goose settings files, including user settings and project `.config/goose/settings.json` / `.config/goose/settings.local.json`.

Kimi stores plugin state under `$KIMI_CODE_HOME/plugins/`, defaulting to `~/.kimi-code/plugins/`. `installed.json` records installed plugins, enablement state, and plugin MCP server toggles; enabled plugin sources are copied to `plugins/managed/<id>/`. Management is TUI slash-command driven (`/plugins install`, `/plugins enable|disable`, `/plugins remove`, `/plugins reload`, `/plugins mcp enable|disable`). Installs are user-scoped and apply across projects; project-scoped plugin installation is not documented.

OpenCode discovers plugins from npm package names in `opencode.json` `plugin`, local `.ts`/`.js` files in `~/.config/opencode/plugins/`, and local `.ts`/`.js` files in `.opencode/plugins/`. npm plugins are installed implicitly with Bun at startup; local plugins are enabled by file presence. The documented CLI install command is `opencode plugin <module>` with alias `opencode plug <module>`. There is no documented plugin update, uninstall, list, or per-plugin enable/disable command; `--pure` disables external plugins for one invocation.

Qwen Code installs extensions into `~/.qwen/extensions/<name>/` on macOS/Linux and `%USERPROFILE%\.qwen\extensions\<name>\` on Windows. The root contains `qwen-extension.json`, `.qwen-extension-install.json`, optional `.env`, and resource directories such as `commands/`, `skills/`, `agents/`, and `hooks/`. Enablement is tracked in `~/.qwen/extensions/extension-enablement.json`; project/workspace scope applies scope-specific enablement and settings rather than creating a separate project extension copy. Startup discovery scans the extensions directory unless Qwen is launched with `--bare` / `QWEN_CODE_SIMPLE`; `--extensions` / `-e` restricts loading to named extensions. Qwen can ingest Claude Code and Gemini plugin formats and rewrite them into Qwen extensions.

Pi records package sources in `~/.pi/agent/settings.json` or project `.pi/settings.json`. `pi install <source>` installs globally by default; `pi install <source> -l/--local` records a project package that loads only after project trust. Sources include npm specs, git URLs/shorthands, HTTPS/SSH git sources, and local files or directories. Pi also auto-discovers direct resource folders such as `~/.pi/agent/extensions`, `~/.pi/agent/skills`, `~/.agents/skills`, `~/.pi/agent/prompts`, and `~/.pi/agent/themes`, plus project `.pi/*` and `.agents/skills` equivalents. The official package gallery is an index of installable packages, not a catalog users add before installing.

Kilo runtime plugins are discovered from the merged config `plugin` array and from `{plugin,plugins}/*.{ts,js}` under scanned config directories. Global runtime plugins live under `~/.config/kilo/plugin/`; project plugins under `.kilo/plugin/`, with legacy `.kilocode/plugin/` also scanned. `kilo plugin <module>` installs npm runtime plugins and patches local or global config, but it is not a Marketplace install command. Marketplace installation is UI-driven and writes extracted skills, agents, or MCP config into normal project/global resource locations.

## Scoping and Namespacing

Scoping varies enough that Claudine should model it explicitly rather than infer it from file paths.

Claude and Codex support user/project/local or marketplace-backed enablement and use plugin-qualified names such as `plugin-name:resource-name`. This is friendly to conflict avoidance but requires rewrite when linking to providers that do not use plugin namespaces.

Gemini and Qwen prefer normal resource names with precedence and conflict handling. Qwen extension slash commands keep their natural names unless they conflict with user or project commands, in which case they are renamed to `extensionName.commandName`. User/project agents shadow extension agents; user MCP and LSP config overrides extension entries by name. Skills are tagged by source but are not prefixed by default.

Goose supports user-scope and project-scope plugins. Open Plugins skills are exposed as `plugin-name:skill-name`, while Gemini-style plugin skills may keep their original skill names. Conflict and precedence behavior is not documented for user-vs-project collisions, plugin-vs-plugin collisions, or plugin skills vs standalone skills, so Claudine should record provenance and avoid assuming deterministic override rules.

Kimi uses a unique plugin id from the manifest `name` field. Slash commands are namespaced as `<plugin>:<command>` and invoked as `/<plugin>:<command>`, preventing command collisions. Skills are exposed by their declared skill `name` and invoked with `/skill:<name>`; the docs do not describe automatic plugin-prefixing or collision behavior for skills.

OpenCode and Kilo runtime plugins do not expose plugin-qualified resource names. Hooks, custom tools, TUI extensions, and related registrations enter shared runtime registries. Custom tools can override built-in tool names. Kilo Marketplace-installed resources use their normal Kilo names after extraction. Pi package resources are also not automatically namespaced by package name; extension commands, prompt templates, and skills register under their own names, with collisions handled by precedence, warnings, and diagnostics rather than by a stable plugin namespace.

## Security and Update Risk

Every provider's plugin system is high trust. None of the researched ecosystems should be treated as a sandbox boundary or signed package format.

Common risks:

- Hooks and scripts run with user OS privileges.
- MCP servers may spawn processes or reach credentials through environment variables.
- Runtime plugin code can access the filesystem, network, SDK clients, auth flows, shell helpers, or provider credentials visible to the process.
- Auto-update or floating npm/git sources can silently change behavior.
- Project-scoped plugins may require workspace trust, but trust does not make the payload portable.

Claude, Codex, Gemini, Kimi, and Qwen generally keep MCP approvals separate from plugin installation, which is good. But the plugin still controls the suggested MCP configuration and process command. Claudine should continue treating MCP catalog entries as normalized records requiring explicit import/sync behavior, not as opaque plugin contents to copy wholesale.

Goose plugin hooks execute local shell commands through `sh -c`, receive event payloads on stdin, inherit the user's environment, and run with normal OS privileges. `goose plugin install --auto-update <git-url>` can update a git-backed plugin before skills load; the optional `version` field is informational and does not pin behavior.

Kimi adds some concrete guardrails but remains high trust. Third-party installs require confirmation and show a source tier badge. Manifest component paths must start with `./` and remain inside the plugin root after symlink resolution. Hooks and MCP servers still run with user OS privileges, hooks receive `KIMI_CODE_HOME` and `KIMI_PLUGIN_ROOT`, and MCP configs can read environment variables via `bearerTokenEnvVar`.

Qwen requires `--consent` or interactive confirmation, and local-path installs require workspace trust. Extension MCP server `trust` fields are stripped, so MCP servers still require normal per-server approval. Sensitive extension settings are stored in the OS keychain; non-sensitive values go to `.env`. Qwen sandboxing can constrain shell commands and spawned MCP/LSP/hook processes when enabled, but it is off by default.

OpenCode, Pi, and Kilo runtime plugins are the highest-risk category because they are executable extension code. OpenCode plugins receive runtime capabilities such as the OpenCode SDK client, Bun shell helper, directory/worktree context, and hooks for shell environment, auth, custom tools, provider behavior, and TUI behavior. Pi TypeScript extensions execute in the Pi process with user permissions and can access the filesystem, environment, package dependencies, network, and Pi runtime APIs. Kilo disables npm lifecycle scripts when installing runtime npm plugins, which reduces install-time package risk, but loaded plugin code still runs with user privileges.

## Implications for Claudine

Claudine should treat plugins as discovery containers and provider-native export targets, not as universal linking units.

For cross-provider linking, the right default is extraction:

- Extract `SKILL.md` resources from plugin skill directories.
- Extract Markdown slash commands where the target provider supports commands.
- Extract agent/subagent definitions only when the source and target contracts are compatible or a known rewrite exists.
- Preserve static assets referenced by extracted Markdown when paths can be rewritten.
- Record provenance: source provider, plugin name, plugin version/source, original namespace, and extracted path.

The right default for high-risk surfaces is refusal or explicit rewrite:

- Do not blindly link plugin manifests.
- Do not copy hooks across providers without event-schema rewrite.
- Do not copy MCP server configs into the MCP catalog without normal MCP validation, credential handling, and provider-specific sync rules.
- Do not link runtime plugin code across providers.
- Do not preserve provider path variables such as `${extensionPath}`, `${CLAUDE_PLUGIN_ROOT}`, `${PLUGIN_ROOT}`, `${PLUGIN_PATH}`, `KIMI_PLUGIN_ROOT`, or `KIMI_CODE_HOME` without rewriting.

The MCP catalog should remain Claudine's source of truth for normalized MCP state. Plugin-declared MCP servers can be import candidates, but not authoritative catalog entries. A plugin may be a convenient place to discover an MCP server; Claudine's catalog is where that server becomes provider-agnostic.

Provider-specific import posture should be:

- Claude, Codex, Gemini, and Qwen: import declarative skills, commands, compatible agents, context files, and passive assets; treat hooks, MCP, LSP, channels, app connectors, settings, and executable paths as explicit rewrite/import candidates.
- Goose: discover enabled plugins from `~/.agents/plugins/<name>/` and `.agents/plugins/<name>/`, honor `disabledPlugins`, and extract only enabled plugin skills. Keep hooks, scripts, manifests, `${PLUGIN_ROOT}` references, Gemini extension metadata, and auto-update source data non-portable.
- Kimi: use `$KIMI_CODE_HOME/plugins/installed.json` as the registry, inspect enabled managed copies under `plugins/managed/<id>/`, and parse `kimi.plugin.json` before `.kimi-plugin/plugin.json`. Extract Markdown skills, command Markdown, README/LICENSE files, and referenced static assets; omit or rewrite `mcpServers`, hooks, `sessionStart.skill`, `skillInstructions`, interface metadata, `KIMI_PLUGIN_ROOT` references, and executable paths.
- OpenCode: ignore `~/.config/opencode/plugins/`, `.opencode/plugins/`, and npm packages listed in `opencode.json` `plugin` during cross-provider linking. Portable OpenCode resources should be handled through separate OpenCode conventions such as `.opencode/skills/`, `.opencode/commands/`, `.opencode/agents/`, `.opencode/tools/`, and `opencode.json` `mcp`.
- Pi: model package source string, scope, filters, install root, and pinned/unpinned update posture separately from extracted resources. Extract `SKILL.md` skills, prompt-template Markdown, compatible theme JSON, and passive assets; classify TypeScript extensions, extension dependencies, registered tools/commands/flags, and extension-mediated MCP or subagent behavior as non-portable executable assets.
- Kilo: model runtime plugins, Marketplace resource catalog entries, and already-extracted normal resources separately. Runtime plugin config entries, npm package references, and `.ts` / `.js` plugin files are non-portable. Marketplace `SKILL.md` directories can be linked as skills if bundled `references/`, `assets/`, and `scripts/` are preserved with executable-risk metadata. `AGENT_DEFINITION.md` and command Markdown need provider-specific frontmatter review. `MCP.yaml` entries should become MCP catalog import candidates only after credential stripping and normal MCP validation.

## Should Claudine Exploit Plugins as a Distribution Channel?

Yes, but selectively.

Claudine should exploit plugin ecosystems for provider-native distribution when the target provider has a stable declarative plugin format. Claude Code, Codex, Gemini, Kimi, Pi, and Qwen are candidates for generated provider-native bundles, but the output must match each provider's actual container semantics. Kimi output should be limited to the resources its format supports: skills, commands, assets, and carefully reviewed Kimi-local MCP/hook declarations. It should not be treated as an agent/subagent distribution target. Pi output should be a Pi package only when Claudine intentionally wants to distribute Pi-local extensions, prompt templates, skills, or themes; executable extensions should not be generated as a side effect of ordinary linking.

Qwen is especially interesting as an import target because it can convert Claude Code and Gemini extension formats, but Claudine should still generate native `qwen-extension.json` bundles when targeting Qwen directly; the converted source may already carry another provider's assumptions and namespace rules.

Claudine should not try to define one plugin package that all providers consume unchanged. The overlap is real, but the runtime semantics are too different. Namespacing, trust, update behavior, hook schemas, MCP config, credential storage, and command formats all diverge.

The strategic shape should be:

1. Claudine maintains provider-agnostic resource models for skills, commands, agents, and MCP servers.
2. Provider plugin systems become export targets and import sources.
3. Plugin manifests are generated per provider, not shared.
4. Executable surfaces are kept out of automatic cross-provider linking unless Claudine has a deliberate adapter.
5. The MCP catalog remains separate from plugin packaging, with plugin MCP declarations treated as discoverable candidates.

In other words: plugins are valuable distribution channels, not Claudine's abstraction boundary. Claudine's boundary should stay at normalized resources and catalog entries. Plugins should be one way to package those resources for a specific provider, and one way to discover provider-local resources that Claudine may extract, classify, and link.
