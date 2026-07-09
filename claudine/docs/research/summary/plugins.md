---
$schema:
    file: file(required;eager)
file: docs/research/summary/plugins.md
sequence:
    - name: draft
    - name: iterate
    - name: finalize
prompt: |-
    Subagent definitions — named, specialized agents that a session can delegate work to, often with their own prompt, model, and tool restrictions — vary widely across agentic CLIs, from first-class definition files to nothing at all. Claudine links agent definitions across providers and also has to reason about subagent observability during wrapped runs.

    ## Task

    Your task is to report on \"plugin\" support across the Agentic CLI providers. Plugins -- unfortunately -- are even less of a \"standard\" than agent definitions and slash commands.\n\n- your report should start by outlining why plugins matter to agentic processes\n- and then shift its focus to how providers differ: \n    - definition format and metadata, \n    - user/repo scopes, model and tool restriction support, \n    - configuration files and schemas\n    - invocation mechanics (CLI influences, ENV influences, config influences)\n    - asset types that can be included in a plugin\n- in the \"plugin\" space Claude Code is seen as the leader and \"standard bearer\", so:\n    - when looking at Claude Code competitors always take the time to describe how it varies from Claude Code\n\nAs background material we have plugin research documents for each provider that Claudine supports. They can be found at `@claudine/docs/research/plugins/*.md`.\n\nImportant: your final response is saved verbatim as the body of this summary document, so it must be the complete document text and nothing else — no preamble, no commentary. Never write to this document yourself.\n\n::block when=\"state.name == 'draft'\"\n- Iterate over the first three research documents to develop a point of view on how to write this document and then produce an initial draft of the document\n::end-block\n::block when=\"state.name == 'iterate'\"\n\n- Note: the initial draft has already been created — it is the body of `@claudine/docs/research/summary/subagents.md` (everything below the frontmatter); read it from there\n- Act as an orchestrator and iterate over each remaining provider's research document:\n    - provide the subagent the current draft and ask them to return an improved draft based on the research document they've been assigned\n- Once every remaining provider has been incorporated, your final response is the fully updated draft\n::end-block\n\n::block when=\"state.name == 'finalize'\"\n\nThe document has now gone through several rounds of improvement and your task is just to make sure the document is consistent in tone and detail and that nothing looks incorrect or incomplete. The current draft is the body of `@claudine/docs/research/summary/plugins.md` (everything below the frontmatter); read it from there, make any adjustments, and your final response will be considered the finalized summary document.\n::end-block"
success:
    stack:
        - when: "state.name == 'finalize'"
          action:
              - message: "🚀  summary research for **plugins** completed!"
              - success: 'Summary research for **plugins** is now complete: `./docs/research/summary/plugins.md`'
        - when: "state.name != 'finalize'"
          action:
              - message: "👓  Summary research for **plugins** completed the _{{state.name}}_ stage"
failure:
    message: 'The summary research for **plugins** failed to complete: {{ err.message }}'
last_updated: 2026-07-09
hash: 61b61bb5ca4dfa9f-c25a7f1f0ebc0f2e
---
# Plugin Support Across Agentic CLIs

## Why plugins matter

Plugins turn an agentic CLI from a configurable assistant into an extensible execution platform. They package related capabilities—such as instructions, skills, commands, subagents, hooks, MCP servers, and executable helpers—so they can be installed, versioned, shared, enabled, and removed as a coherent unit.

That packaging matters to agentic processes because plugins can affect every layer of a session:

- **Reasoning context:** skills, commands, context files, and output styles change how the model approaches work.
- **Delegation:** packaged subagents add specialized roles with their own prompts and potentially their own model or tool policies.
- **Tool access:** MCP servers, connectors, policies, and executable helpers expand or restrict what the agent can do.
- **Lifecycle behavior:** hooks and monitors can run before, during, or after tool calls and sessions.
- **Reproducibility:** manifests and versioned distributions make a working agent setup easier to share across users and repositories.
- **Security:** installing a plugin may authorize executable code, credentials, network services, and automatic updates.
- **Observability:** plugins can introduce subagents, hooks, and external tools whose activity may not be represented consistently in a provider’s normal event stream.

For Claudine, a plugin therefore cannot be treated as merely another configuration directory. Claudine needs to identify which plugin version and resources were active during a wrapped run, retain their provenance, and distinguish observable provider activity from plugin-owned processes that execute outside the provider’s sandbox or event protocol.

This initial comparison covers the first three providers in Claudine’s research roster: Claude Code, Codex CLI, and Gemini CLI.

## Comparative Overview

Claude Code is the current standard bearer because it has the most complete plugin lifecycle and the broadest package vocabulary. Codex follows a recognizably Claude-like model but remains younger and more marketplace-dependent. Gemini calls its containers “extensions” and uses a flatter, GitHub-centered system with unusually explicit tool restrictions.

| Capability                     | Claude Code                                                      | Codex CLI                                   | Gemini CLI                                                      |
|--------------------------------|------------------------------------------------------------------|---------------------------------------------|-----------------------------------------------------------------|
| Provider term                  | Plugin                                                           | Plugin                                      | Extension                                                       |
| Support level                  | First-class                                                      | First-class, experimental                   | First-class                                                     |
| Manifest                       | `.claude-plugin/plugin.json`                                     | `.codex-plugin/plugin.json`                 | `gemini-extension.json`                                         |
| Required metadata              | Manifest may be omitted; `name` is required when present         | `name`                                      | `name`, `version`                                               |
| Installed layout               | Versioned user cache plus persistent data                        | Versioned user cache                        | One installed directory per extension                           |
| User scope                     | Yes                                                              | Yes                                         | Yes                                                             |
| Repository influence           | Project/local settings and project skill plugins                 | Repository or personal marketplace catalogs | Workspace enablement and higher-precedence standalone resources |
| Session-only loading           | `--plugin-dir`, `--plugin-url`                                   | No equivalent documented                    | Local linking, but restart-based                                |
| Package-wide model restriction | None documented                                                  | None documented                             | None documented                                                 |
| Package-wide tool restriction  | No general manifest allow/deny list                              | No general manifest allow/deny list         | `excludeTools` plus packaged policies                           |
| Marketplace                    | Installable official, community, and private marketplaces        | Curated and private marketplace catalogs    | Public gallery, but no installable marketplace layer            |
| Safe-mode influence            | `--safe-mode`, `--bare`, and corresponding environment variables | No plugin-specific safe mode documented     | Enterprise configuration can disable or restrict extensions     |
| Reload behavior                | `/reload-plugins` or restart                                     | Restart                                     | Restart                                                         |

## Claude Code: The Baseline

Claude Code defines a plugin as a namespaced, installable package whose resources are discovered from conventional directories and optional manifest paths. Its package can be distributed through a marketplace, Git repository, npm package, local directory, URL, or archive. See [Create plugins](https://code.claude.com/docs/en/plugins) and the [plugin reference](https://code.claude.com/docs/en/plugins-reference).

### Definition and metadata

A conventional package contains:

```text
plugin-root/
├── .claude-plugin/
│   └── plugin.json
├── skills/
├── commands/
├── agents/
├── hooks/
├── .mcp.json
├── .lsp.json
├── output-styles/
├── themes/
├── monitors/
├── bin/
├── settings.json
└── README.md
```

The manifest supports descriptive metadata such as `name`, `displayName`, `version`, `description`, author and repository information, licensing, and keywords. It can also declare or redirect component paths for skills, commands, agents, hooks, MCP servers, LSP servers, output styles, themes, monitors, channels, dependencies, and user configuration.

The manifest itself is optional. Without it, Claude Code can derive the package name from its directory and discover components in their conventional locations. This makes simple plugins easy to author, while the full manifest supports richer distribution and lifecycle behavior.

### Scope, model, and tool restrictions

Claude Code has the richest scope model of the three providers:

- User settings in `~/.claude/settings.json`
- Project settings in `.claude/settings.json`
- Gitignored local settings in `.claude/settings.local.json`
- Read-only managed settings
- Session-only plugins loaded through CLI flags
- User or project `@skills-dir` plugins discovered through skill directories

The plugin manifest does not define a package-wide model allowlist or model pin. Model and tool choices may exist inside contained resource definitions, especially subagents, but those belong to the contained resource schema rather than the plugin container.

Plugin subagents have an important security restriction: they cannot declare `hooks`, `mcpServers`, or `permissionMode`. Plugin-provided MCP servers still use normal MCP approval, while executable plugin components run with the user’s OS privileges.

### Configuration and invocation

Installed plugins are enabled through `enabledPlugins` in the applicable settings layer. Claude also maintains installation and marketplace registries under `~/.claude/plugins/`, including a versioned cache, persistent plugin data, known marketplaces, installed-plugin records, and a local blocklist.

Plugins can be managed through both slash commands and the CLI:

```text
/plugin install
/plugin update
/plugin enable
/plugin disable
/plugin uninstall
/reload-plugins

claude plugin install
claude plugin update
claude plugin enable
claude plugin disable
claude plugin uninstall
claude plugin marketplace add
```

`--plugin-dir` and `--plugin-url` add session-only plugins and override an installed plugin with the same name for that session. Plugin skills, commands, and agents are namespaced using forms such as `plugin-name:resource-name`.

Environment variables can relocate the plugin cache, seed container images, tune Git operations and installation synchronization, control updates, or disable plugins. Of particular importance to wrapped execution:

- `CLAUDE_CODE_PLUGIN_CACHE_DIR` changes the plugin state root.
- `CLAUDE_CODE_PLUGIN_SEED_DIR` supplies preinstalled plugin content.
- `CLAUDE_CODE_SAFE_MODE` disables plugins with most customizations.
- `CLAUDE_CODE_SIMPLE` disables plugin and resource discovery.
- `DISABLE_AUTOUPDATER` and `FORCE_AUTOUPDATE_PLUGINS` influence update behavior.

### Assets

Claude Code has the broadest plugin package:

- Agent Skills
- Slash commands
- Subagents
- MCP and LSP servers
- Hooks and background monitors
- Scripts and executables
- Themes and output styles
- Channels
- Limited plugin settings and user configuration
- Static documentation and assets

It does not define a separate generic “prompt” resource; skills and commands fill that role.

## Codex CLI: Claude-Like, but Marketplace-Centered

Codex has adopted many of Claude Code’s plugin concepts: a hidden manifest directory, namespaced resources, a versioned cache, marketplace catalogs, and packages containing skills, commands, subagents, hooks, and MCP servers. Its implementation is nevertheless distinct and currently less mature. See the [Codex plugins overview](https://developers.openai.com/codex/plugins) and [Build plugins](https://developers.openai.com/codex/plugins/build).

### Definition and metadata

A Codex package uses `.codex-plugin/plugin.json`. The only required field is `name`; optional fields include version, description, author, repository and license information, keywords, component paths, and extensive install-surface metadata under `interface`.

```text
plugin-root/
├── .codex-plugin/
│   └── plugin.json
├── skills/
├── agents/
├── commands/
├── hooks/
├── .app.json
├── .mcp.json
├── assets/
├── scripts/
├── bin/
└── README.md
```

Compared with Claude Code, Codex’s manifest is less focused on runtime variety and more heavily invested in marketplace presentation and ChatGPT integration. The `interface` object can define display text, category, capabilities, branding, screenshots, privacy links, and suggested prompts. `.app.json` maps plugin-local app names to ChatGPT connector identifiers.

Unlike Claude’s optional manifest, Codex requires `.codex-plugin/plugin.json`. No official JSON Schema URL is documented for either the plugin manifest or marketplace catalog.

### Scope, model, and tool restrictions

Installed packages live in a user-level cache under `~/.codex/plugins/cache/`. Repository and user scopes primarily affect marketplace discovery through `.agents/plugins/marketplace.json` and `~/.agents/plugins/marketplace.json`; they do not provide Claude’s equivalent combination of user, project, local, managed, and session-only plugin instances.

Codex does not document package-wide model or tool restrictions in `plugin.json`. Contained skills and agents remain subject to Codex’s normal approval policy, while individual plugin MCP servers can be constrained through plugin-scoped configuration and identity allowlists.

Codex adds a trust distinction absent from Claude’s baseline: plugin hooks are skipped until the user explicitly trusts the current hook definition. This is narrower than sandboxing—the hook still runs with user privileges once trusted—but it creates a separate review boundary for executable lifecycle behavior.

### Configuration and invocation

Enablement is stored in TOML rather than Claude’s JSON settings:

```toml
[plugins."github@openai-curated"]
enabled = true
```

Installation and marketplace management are CLI-driven:

```text
codex plugin add
codex plugin list
codex plugin remove
codex plugin marketplace add
codex plugin marketplace list
codex plugin marketplace upgrade
codex plugin marketplace remove
```

The TUI’s `/plugins` browser can install plugins and toggle their enabled state. There are no dedicated `plugin enable` or `plugin disable` subcommands; enablement is changed in `config.toml` or through the TUI, followed by a restart.

Compared with Claude Code, Codex has no documented session-only `--plugin-dir` or `--plugin-url` mechanism, no per-plugin update command, and no plugin-specific safe mode. Updates refresh marketplace snapshots rather than directly updating a named package.

`CODEX_HOME` is the principal environment influence because it relocates Codex state, including configuration and the plugin cache. Other documented Codex environment variables affect authentication, diagnostics, or general runtime behavior rather than plugin discovery specifically.

### Assets

Codex plugins can contain:

- Agent Skills
- Slash commands
- Subagents
- Hooks
- MCP servers
- ChatGPT apps and connectors
- Scripts and executables
- Static assets and marketplace presentation media
- Limited plugin and MCP policy configuration

Compared with Claude Code, Codex lacks documented LSP servers, themes, output styles, monitors, channels, dependency metadata, and rich plugin user configuration. Its distinctive asset is the ChatGPT connector mapping in `.app.json`.

## Gemini CLI: Extensions Rather Than Plugins

Gemini’s equivalent container is called an extension. It packages much of the same agent functionality but uses a flatter layout and a GitHub-centered distribution model. See the [extensions overview](https://geminicli.com/docs/extensions/) and [extension reference](https://geminicli.com/docs/extensions/reference/).

### Definition and metadata

Every extension has a required root-level `gemini-extension.json` containing at least `name` and `version`.

```text
extension-root/
├── gemini-extension.json
├── GEMINI.md
├── commands/
├── skills/
├── agents/
├── hooks/
├── policies/
├── package.json
├── .env
└── README.md
```

Optional manifest fields include MCP servers, a persistent context filename, excluded tools, repository migration information, planning configuration, user settings, and themes. `${extensionPath}` is the portable reference for files inside the installed extension.

This differs materially from Claude Code. Gemini has no hidden manifest subdirectory, requires an explicit semantic version, and places tool exclusions and themes directly in the extension manifest. It has no Claude-style marketplace manifest or dependency system.

### Scope, model, and tool restrictions

Extensions are installed beneath `~/.gemini/extensions/<name>/`. Enablement can be scoped to the user or workspace, but the installed package itself remains user-level. Gemini has no equivalent to Claude’s local settings scope or session-only plugin flags.

No package-wide model pin or model allowlist is documented. `GEMINI_MODEL` can select the CLI’s default model, but it is a process-level setting rather than extension metadata.

Gemini is the only one of these three providers with an explicit extension-wide tool restriction: `excludeTools` can block named tools or particular commands. Extensions may also contain `policies/*.toml`; extension policies cannot approve tool calls or activate yolo mode, which prevents a plugin from using policy files to grant itself broader authority.

This is a significant departure from Claude Code. Claude’s plugin container primarily packages capabilities and relies on normal permission checks, while Gemini’s manifest can directly subtract tools from the active environment.

### Configuration and invocation

Gemini stores each installed extension as one directory rather than using Claude’s or Codex’s versioned cache. `.gemini-extension-install.json` records how the extension was installed, while `.env` stores extension-specific settings as environment variables.

The lifecycle is managed through:

```text
gemini extensions install
gemini extensions link
gemini extensions update
gemini extensions uninstall
gemini extensions enable
gemini extensions disable
gemini extensions list
gemini extensions config
gemini extensions validate
```

All management changes require a restart. GitHub Releases, Git clones, local directories, and symlinks are supported. `--ref`, `--auto-update`, `--pre-release`, `--consent`, and `--skip-settings` influence installation.

`GEMINI_CLI_HOME` relocates Gemini’s user configuration and extension storage. Enterprise JSON settings can disable extensions, prohibit Git-based installation, or restrict extensions by name. Declared sensitive settings are stored in the system keychain and injected only through manifest-declared environment variables.

Compared with Claude Code, Gemini has no installable marketplace hierarchy. Its public gallery discovers GitHub repositories tagged `gemini-cli-extension`, but installation still resolves to a repository, release, or local path.

### Assets

Gemini extensions can contain:

- Agent Skills
- TOML slash commands
- Preview subagents
- MCP servers
- Hooks
- Themes
- Policy Engine rules
- Persistent `GEMINI.md` context
- Planning configuration
- Scripts, Node.js packages, and platform binaries
- Static assets
- Declared user settings and secrets

Gemini’s distinctive assets are policies, persistent context, and manifest-level tool exclusions. Its custom commands are TOML rather than Claude’s and Codex’s Markdown command files, increasing the amount of transformation required for cross-provider linking.

## Configuration and Schema Differences

There is no shared plugin schema across these providers:

| Concern                  | Claude Code                            | Codex CLI                          | Gemini CLI                                 |
|--------------------------|----------------------------------------|------------------------------------|--------------------------------------------|
| Package manifest         | JSON, optionally absent                | Required JSON                      | Required JSON                              |
| Runtime enablement       | Layered `settings.json`                | User `config.toml`                 | User/workspace `settings.json`             |
| Marketplace catalog      | `.claude-plugin/marketplace.json`      | `.agents/plugins/marketplace.json` | None                                       |
| Installed-state metadata | Central JSON registries                | Cache plus TOML enablement         | Per-extension installation JSON            |
| Extension settings       | `userConfig` and plugin settings       | Limited plugin/MCP tables          | Manifest declarations plus `.env`          |
| Validation               | Manifest discovery and plugin commands | No published schema documented     | `gemini extensions validate`               |
| Safe-mode configuration  | CLI and environment                    | None documented                    | Enterprise settings can disable extensions |

The similarity of the manifests should not obscure their incompatibility. Field names, default paths, merge behavior, package layout, namespacing, and lifecycle state all differ.

## Portability and Claudine Implications

Claudine should not link any of these containers intact across providers. A whole-package symlink would preserve the wrong manifest, paths, trust assumptions, invocation syntax, and executable behavior.

A safer strategy is resource extraction:

- Treat Agent Skills as the strongest portability candidate.
- Transform slash commands between Markdown and Gemini TOML formats.
- Map subagent frontmatter explicitly, including model and tool fields.
- Preserve plugin identity and namespace as provenance even when the target provider uses a different invocation syntax.
- Copy static assets only when referenced by a portable resource.
- Treat hooks, MCP servers, policies, credentials, scripts, executables, and connector mappings as provider-specific.
- Require host-aware review before transferring executable or credential-bearing resources.
- Record the active plugin version, scope, and source in wrapped-run metadata.

Plugin observability also needs to be resource-aware. A wrapper should distinguish:

1. Provider-native subagent or tool events emitted through the normal stream.
2. Plugin hooks and helper processes that may run outside that stream.
3. MCP or connector activity whose visibility depends on provider protocol support.
4. Resources installed but disabled, shadowed, untrusted, or suppressed by safe-mode settings.

Claude Code provides the most complete reference model for discovery, lifecycle, namespaces, and distribution. Codex demonstrates that a competitor can closely resemble Claude while changing configuration, trust, update, and connector semantics. Gemini demonstrates a more independent design in which the package is flatter, distribution is GitHub-oriented, and tool restrictions and policies are first-class extension concerns.

The emerging common denominator is therefore not a universal plugin format. It is a package containing discoverable agent resources plus provider-specific lifecycle and trust metadata. Claudine should normalize the inventory and provenance of those resources while preserving—and never silently flattening—the provider-specific security and execution semantics.
