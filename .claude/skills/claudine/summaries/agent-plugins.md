# Agentic CLI Plugin and Extension Ecosystems

Plugin ecosystems matter because agentic CLIs are no longer just prompt runners. They are becoming runtime platforms: they load reusable instructions, slash commands, subagents, MCP servers, hooks, scripts, UI assets, auth connectors, and provider-specific policy/configuration. A plugin can change what an agent knows, what tools it can call, what events fire around a session, what credentials are reachable, and what commands users can invoke.

That overlaps directly with Claudine's resource-linking model. Claudine already treats skills, commands, agents, scripts, hooks, and MCP servers as provider-shaped resources that can be discovered, classified, linked, or rewritten across CLIs. Plugins complicate that model because they bundle portable resources together with provider-specific executable surfaces. A `SKILL.md` inside a plugin may be broadly portable; the plugin manifest, hook payload schema, MCP process environment, namespace syntax, install metadata, and auto-update behavior usually are not.

The useful distinction is therefore not "does this provider have plugins?" It is:

- Does the provider have an installable unit?
- Is that unit declarative, executable, or mixed?
- Which contained resources can Claudine safely extract?
- Which resources must stay provider-local?
- Does the provider namespace plugin resources in a way Claudine must preserve or rewrite?
- Can the plugin system serve as a distribution channel for Claudine-managed resources?

## Provider Patterns

Claude Code, Codex, Gemini, Kimi, and Qwen have first-class plugin or extension containers. They use JSON manifests, install commands or TUI management flows, versioned or managed install directories, and explicit discovery at startup or reload. These are the strongest candidates for Claudine-aware resource extraction.

Goose has a narrower plugin model. It follows the Open Plugins shape, but Goose only clearly documents skills and hooks. It is useful for skill distribution but should not be treated as a full cross-provider bundle format.

OpenCode is different. Its plugins are TypeScript/JavaScript runtime modules loaded from config, npm, or plugin directories. They can register hooks, tools, auth providers, model hooks, and TUI behavior, but they are executable OpenCode code, not portable resource containers. Claudine should not try to extract cross-provider resources from OpenCode plugins.

Pi and Kilo broaden the research picture. Pi packages are npm/git/local packages that can carry extensions, skills, prompt templates, themes, and assets. Kilo has runtime plugins plus a Marketplace that extracts skills, agents, and MCP entries into normal resource locations. Both are important for future provider support, but they reinforce the same rule: package-as-unit is provider-local; contained Markdown resources may be portable.

## Comparison

| Provider    | Ecosystem                                  | Manifest / format                                                                             | Main install model                                        | Portable resource potential                  |
|-------------|--------------------------------------------|-----------------------------------------------------------------------------------------------|-----------------------------------------------------------|----------------------------------------------|
| Claude Code | First-class plugins                        | `.claude-plugin/plugin.json`                                                                  | Marketplace, Git, local dir, session plugin               | High: skills, commands, agents, assets       |
| Codex       | First-class plugins                        | `.codex-plugin/plugin.json` plus marketplace JSON                                             | Plugin marketplace, repo/personal marketplaces, local dir | High: skills, commands, agents, assets       |
| Gemini CLI  | First-class extensions                     | `gemini-extension.json`                                                                       | GitHub repo/release, local dir/link                       | High: skills, commands, agents, assets       |
| Goose       | Partial plugins                            | `plugin.json`, `.plugin/plugin.json`, `.goose-plugin/plugin.json`, or Gemini extension format | Git clone or copied directory                             | Medium: primarily skills                     |
| Kimi Code   | First-class but TUI-managed plugins        | `kimi.plugin.json` or `.kimi-plugin/plugin.json`                                              | `/plugins` TUI from marketplace, GitHub, zip, local dir   | Medium-high: skills, commands, assets        |
| OpenCode    | Runtime plugins                            | TS/JS module or npm package                                                                   | `opencode.json` plugin array, npm, local files            | Low: do not extract                          |
| Qwen Code   | First-class extensions                     | `qwen-extension.json`; can convert Claude/Gemini formats                                      | Git, npm, archives, local paths, Claude/Gemini sources    | High: skills, commands, agents, context      |
| Pi          | Package ecosystem                          | `package.json` with `pi` fields or conventional dirs                                          | npm, git, local path                                      | Medium-high: skills, prompts, themes, assets |
| Kilo        | Runtime plugins plus Marketplace resources | npm/local plugin modules; Marketplace `SKILL.md`, `AGENT_DEFINITION.md`, `MCP.yaml`           | npm/local runtime plugins; Marketplace UI                 | Medium: extracted Marketplace resources      |

## Asset Types

The most portable assets are Markdown-like, declarative, and instruction-oriented:

| Asset type                                                | Portability                                                              |
|-----------------------------------------------------------|--------------------------------------------------------------------------|
| Agent Skills                                              | Usually portable, with namespace and path rewrites                       |
| Slash commands / prompt templates                         | Usually portable, but command syntax and namespacing differ              |
| Subagents                                                 | Portable only between providers with similar agent frontmatter/contracts |
| Context files such as `GEMINI.md` / `QWEN.md`             | Partially portable as provider-specific persistent context               |
| Static assets / README / examples                         | Portable as support files                                                |
| MCP server declarations                                   | Not safely portable without rewrite and credential review                |
| Hooks                                                     | Provider-specific; event schemas and execution environments differ       |
| Scripts and `bin/` executables                            | Non-portable unless explicitly reviewed                                  |
| Runtime plugin code                                       | Provider-local                                                           |
| Auth connectors, app connectors, channels, TUI extensions | Provider-local                                                           |
| Themes/output styles                                      | Usually provider-local                                                   |

Claude, Codex, Gemini, Kimi, and Qwen all package the key resource trio Claudine already understands: skills, slash commands, and agents/subagents. They also package higher-risk integration surfaces such as hooks and MCP servers. That mixed payload is the central design issue.

## Installation and Discovery

Claude and Codex use marketplace-style plugin installation into versioned cache directories. Enablement is stored in provider settings, and plugin resources are loaded at startup or reload. Both namespace plugin resources strongly enough that Claudine must translate names when exporting them.

Gemini installs extensions into `~/.gemini/extensions/<name>/` and requires `gemini-extension.json`. It merges extension-provided skills, commands, agents, hooks, MCP servers, policies, and context into the session. User and project resources generally outrank extension resources.

Kimi stores managed plugin copies under `$KIMI_CODE_HOME/plugins/managed/<id>/` and records state in `installed.json`. Management is TUI slash-command driven rather than shell-CLI driven. Plugins are user-scoped, with `/reload` or a new session needed for changes.

Qwen stores extensions under `~/.qwen/extensions/<name>/` and uses `extension-enablement.json` for scope-specific enablement. A major distinction is conversion: Qwen can ingest Claude Code and Gemini plugin formats and rewrite them into Qwen extensions. That makes Qwen a consumer of other ecosystems, not just a peer format.

Goose scans `~/.agents/plugins/` and `.agents/plugins/`, with disabling controlled by `disabledPlugins` in Goose settings. There is no marketplace and no fully documented conflict model.

OpenCode loads plugins from `opencode.json`, global/project plugin directories, and npm packages. There is no declarative plugin manifest and no resource namespace. Plugin code joins global runtime registries.

Pi resolves package sources from settings and can install from npm, git, or local paths. Package resources are not automatically namespaced. Kilo similarly separates runtime plugin modules from Marketplace resources that are installed into normal resource locations.

## Scoping and Namespacing

Scoping varies enough that Claudine should model it explicitly rather than infer it from file paths.

Claude and Codex support user/project/local or marketplace-backed enablement and use plugin-qualified names such as `plugin-name:resource-name`. This is friendly to conflict avoidance but requires rewrite when linking to providers that do not use plugin namespaces.

Gemini and Qwen prefer normal resource names with precedence and conflict handling. Qwen prefixes extension commands only when a conflict exists. Gemini may dot-prefix extension commands on conflict. Skills are generally resolved through discovery precedence rather than strict plugin namespaces.

Kimi namespaces slash commands as `<plugin>:<command>` but does not clearly document automatic skill-name prefixing.

Goose uses `plugin-name:skill-name` for Open Plugins skills, but Gemini-style plugin skills may keep their original names.

OpenCode, Pi, and Kilo runtime plugins mostly register into shared namespaces. That makes them powerful but weaker as portable distribution units.

## Security and Update Risk

Every provider's plugin system is high trust. None of the researched ecosystems should be treated as a sandbox boundary or signed package format.

Common risks:

- Hooks and scripts run with user OS privileges.
- MCP servers may spawn processes or reach credentials through environment variables.
- Runtime plugin code can access the filesystem, network, SDK clients, auth flows, or shell helpers.
- Auto-update or floating npm/git sources can silently change behavior.
- Project-scoped plugins may require workspace trust, but trust does not make the payload portable.

Claude, Codex, Gemini, Kimi, and Qwen generally keep MCP approvals separate from plugin installation, which is good. But the plugin still controls the suggested MCP configuration and process command. Claudine should continue treating MCP catalog entries as normalized records requiring explicit import/sync behavior, not as opaque plugin contents to copy wholesale.

## Implications for Claudine

Claudine should treat plugins as discovery containers, not as universal linking units.

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
- Do not preserve provider path variables such as `${extensionPath}`, `${CLAUDE_PLUGIN_ROOT}`, `${PLUGIN_ROOT}`, or `KIMI_PLUGIN_ROOT` without rewriting.

The MCP catalog should remain Claudine's source of truth for normalized MCP state. Plugin-declared MCP servers can be import candidates, but not authoritative catalog entries. A plugin may be a convenient place to discover an MCP server; Claudine's catalog is where that server becomes provider-agnostic.

## Should Claudine Exploit Plugins as a Distribution Channel?

Yes, but selectively.

Claudine should exploit plugin ecosystems for provider-native distribution when the target provider has a stable declarative plugin format. Claude Code, Codex, Gemini, Kimi, and Qwen are good candidates for generated provider-native bundles containing Claudine-linked skills, commands, agents, and supporting assets. Qwen is especially interesting because it can consume Claude and Gemini formats, but Claudine should still generate native Qwen output when targeting Qwen directly.

Claudine should not try to define one plugin package that all providers consume unchanged. The overlap is real, but the runtime semantics are too different. Namespacing, trust, update behavior, hook schemas, MCP config, credential storage, and command formats all diverge.

The strategic shape should be:

1. Claudine maintains provider-agnostic resource models for skills, commands, agents, and MCP servers.
2. Provider plugin systems become export targets and import sources.
3. Plugin manifests are generated per provider, not shared.
4. Executable surfaces are kept out of automatic cross-provider linking unless Claudine has a deliberate adapter.
5. The MCP catalog remains separate from plugin packaging, with plugin MCP declarations treated as discoverable candidates.

In other words: plugins are valuable distribution channels, not Claudine's abstraction boundary. Claudine's boundary should stay at normalized resources and catalog entries. Plugins should be one way to package those resources for a specific provider, and one way to discover provider-local resources that Claudine may extract, classify, and link.
