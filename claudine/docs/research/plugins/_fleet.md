---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/plugins/{{state.file}}"
# NOTE: `grant:` is not implemented yet. Run with `--yolo` so the provider can
# inspect local plugin folders under {{state.user_dir}} when they exist.
grant:
    read:
        - "{{state.user_dir}}"
agent: opencode
model: kimi-for-coding/k2p7
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
initialize:
    stack:
        - when: "!file_exists(file) || !frontmatter(file, 'last_updated') || date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - message: "The provider **{{state.name}}** needs to update its research on **plugins**"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - stderr: "The provider **{{state.name}}** has research for **plugins** that is current; skipping updates"
              - skip
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The **Plugins** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **Plugins** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the Plugins research on **{{state.name}}** failed to complete!"
    warn: "The Plugins research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---
# Plugin Research on {{state.name}}

## Skills

Use the 'claudine' skill.

## Context

Prior-generation research in `../cross-referencing/` is a validation asset for humans
— do not open, paraphrase, or cite it; your research must be independent.

A **plugin** is a packaging unit that bundles multiple extension assets into one installable, shareable unit. Rather than scattering custom instructions, tool configs, and hook scripts across a project, plugins wrap them together so the agent can discover and use them as a coherent capability.

Plugins may include:

- Agent Skills
- scripts
- slash commands / prompts
- agents/subagents definitions
- MCP servers 
- LSP Server integrations
- Formatting rules or visual or linguistic themes/styles

How plugins are installed and configured varies by tool. There is no standard that governs this even though some of the assets which are contained by plugins are more standards based (e.g., Agent Skills and MCP, etc.)

This topic owns the plugin **container**: containment, extraction, and the
install/discovery/trust machinery around it. The semantics of each contained resource
belong to its sibling topic (skills, slash-commands, subagents, hooks, MCP).

Your primary goal is to research how **{{state.name}}** works with "plugins".

::block when="state.name != 'Claude Code'"

As is common in Agentic CLI platforms today, Claude Code is often seen as the leader and trend setter in this space. You should make clear comparisons between how {{state.name}} differs from Claude Code as this will often serve as a good reference point for readers. Cite the doc/URL for every comparison claim; do not rely on memory.

::end-block

Write the research prose to `{{file}}` before adding the frontmatter structured data described below.

## Research Deliverables

Write prose specific enough that Claudine can implement plugin discovery and linking
without guessing. Prefer exact paths, manifest fields, package layouts, commands,
precedence rules, trust behavior, and verified limitations over general descriptions.

In the body, cover:

- The provider's plugin system: what a plugin is, what it can contain, and when it is
  loaded.
- Installation locations and scopes: user, repo/project, workspace, system, marketplace,
  extension, or other.
- Manifest and package format: file names, required fields, optional fields, directory
  layout, archive/repo format, scripts, assets, and examples.
- Lifecycle behavior: install, update, remove, enable, disable, pin version, trust,
  audit, and safe-mode behavior.
- Packaged resource coverage: Agent Skills, scripts, slash commands, subagents, MCP,
  hooks, prompts, config, assets, and any other provider-specific resource.
- Discovery and precedence: load order, namespacing, conflicts, shadowing, plugin
  resource visibility, and interaction with user/repo resources outside plugins.
- Runtime and security behavior: permissions, sandboxing, credentials, executable code,
  update risk, and whether plugin-provided resources are trusted differently.
- Distribution: marketplace, registry, Git repo, local folder, archive, private
  distribution, and publishing rules.
- Portability: whether Claudine should link the whole plugin, extract contained
  resources, rewrite provider-specific metadata, or mark the plugin non-portable.
- Claudine integration notes: what the linker should do, what it should avoid, and
  whether the research implies code or generated-metadata changes.

## Frontmatter Contract

Read `./_schema.yaml` before writing. It is the machine-validated contract. Populate
frontmatter as follows:

- `$schema` - set to the string `./_schema.yaml`.
- `created` - first-run date, `{{ctx.today}}`. Preserve the existing value on update.
- `last_updated` - set to `{{ctx.today}}`.
- `agent` - set to `{{env.AGENT}}`.
- `model` - set to `{{env.MODEL || 'default'}}`.
- `homepage` - provider homepage URL, when useful for identification.
- `docs` - best general official documentation URL for this provider's CLI/config.
- `plugin_docs` - best official URL specifically covering plugins, extensions,
  packages, marketplaces, or add-ons. Omit only when no such page exists and explain the
  documentation gap in the body.
- `support` - classify the provider's plugin/container implementation:
  - `first_class`: documented plugins with clear install, manifest/package, lifecycle,
    discovery, and trust behavior.
  - `partial`: plugin behavior exists, but important implementation details are limited,
    unstable, undocumented, UI-only, marketplace-only, or unavailable in CLI mode.
  - `convention_only`: bundles are implemented through documented folders, repos,
    manifests, or conventions rather than a dedicated plugin system.
  - `none`: clear evidence the provider has no plugin/container mechanism.
  - `unknown`: current sources do not prove the answer after serious research.
- `locations` - one record per plugin storage or installation location: `os`, `scope`,
  `path`, and optional `notes`. Use template paths like `~/.provider/plugins`,
  `.provider/plugins`, or registry names rather than host-specific absolute paths.
- `manifest` - plugin manifest/package shape:
  - `file_names`: accepted manifest names or package entry points, such as
    `plugin.json`, `extension.yaml`, `package.json`, or `README.md`.
  - `format`: manifest/package format: `markdown`, `yaml`, `json`, `jsonc`, `toml`,
    `text`, `archive`, `repo`, `other`, or `unknown`.
  - `required_fields`: required manifest fields.
  - `optional_fields`: recognized optional fields.
  - `package_layout`: directory layout and where contained resources live.
  - `notes`: examples, schema links, package roots, generated files, or undocumented
    constraints.
- `lifecycle` - install/update/remove/enable/disable/trust/version behavior. Include
  whether lifecycle actions are CLI-driven, UI-driven, config-driven, marketplace-driven,
  or manual filesystem operations.
- `packaged_resources` - classify what plugins can contain:
  - `skills`, `scripts`, `slash_commands`, `subagents`, `mcp_servers`, `hooks`,
    `prompts`, `config`, and `assets` are each `full`, `partial`, `none`, or `unknown`.
  - `other` lists additional provider-specific resource types.
  - this records **containment/extraction only** — the semantics of each contained
    resource type are owned by its sibling topic.
- `discovery` - explain mechanism, precedence, namespacing, conflicts, and notes.
  Include whether plugin resources shadow user/repo resources, whether user/repo
  resources shadow plugin resources, and whether plugin resources appear with prefixes.
- `security` - trust and runtime risk:
  - `trust_model`: how plugins are trusted, approved, signed, sandboxed, or blocked.
  - `permissions`: how plugin-provided tools/scripts/MCP/resources request permissions.
  - `sandbox_interaction`: whether plugin code/resources run inside provider sandboxing.
  - `credential_access`: whether plugins can read env vars, config secrets, tokens, or
    credential stores.
  - `update_risk`: whether updates can change behavior silently.
  - `notes`: any additional caveats.
- `distribution` - marketplace/registry/source behavior:
  - `marketplace`: true if an official marketplace or registry exists.
  - `registry_url`: marketplace/registry URL when known.
  - `source_types`: supported sources such as local folder, Git repo, archive, npm,
    marketplace, URL, or private registry.
  - `publishing`: how plugins are published.
  - `private_distribution`: enterprise/private sharing behavior.
  - `notes`: install provenance, signing, ownership, or moderation.
- `portability` - Claudine's linking classification:
  - `link_plugin_as_unit`: true when the plugin container can be shared intact.
  - `extract_resources`: true when Claudine should link contained skills/scripts/
    commands/subagents/etc. rather than the whole plugin.
  - `portable_resources`: contained resource types that are portable.
  - `non_portable_assets`: binaries, scripts, provider metadata, marketplace IDs,
    credentials, hooks, MCP config, or other assets that cannot be shared directly.
  - `rewrite_needed`: true when metadata/content must be transformed.
  - `notes`: describe the exact rewrite or why no safe rewrite exists.
- `cli_params` - every CLI flag/subcommand that installs, lists, updates, removes,
  enables, disables, trusts, audits, loads, or points to plugins. Use `[]` only after
  checking docs and `--help`, and state the absence in the body.
- `env_vars` - environment variables that influence plugin paths, config roots,
  marketplace behavior, trust, safe mode, loading, or disabling. Use `[]` only when
  verified absent.
- `gaps` - missing docs, unsupported inspection, contradictory claims, untested behavior,
  or local config unavailable.
- `changes` - on first run, `[]`; on update, concise strings describing changes since
  the previous research. Do not use old research as proof for current facts.
- `requires_claudine_update` - `true` only when Claudine code, schemas, generated
  metadata, or linking rules should change because of the research.
- `reason` - required when `requires_claudine_update` is true; otherwise a short
  explanation is still useful.

## Useful Examples

These examples show the expected specificity. Do not copy them unless verified for
{{state.name}}.

```yaml
support: first_class
locations:
  - os: macos
    scope: user
    path: "~/.provider/plugins"
    notes: "Installed plugins are expanded into one directory per plugin on macOS."
  - os: linux
    scope: user
    path: "~/.config/provider/plugins"
    notes: "Example Linux/XDG location; verify exact provider behavior."
  - os: windows
    scope: user
    path: "%APPDATA%\\Provider\\plugins"
    notes: "Example Windows location; verify exact provider behavior."
manifest:
  file_names: ["plugin.json"]
  format: json
  required_fields: ["name", "version"]
  optional_fields: ["skills", "commands", "agents", "mcpServers"]
  package_layout: "skills/ contains Agent Skills; commands/ contains slash commands; agents/ contains subagents."
  notes: "Manifest paths are relative to the plugin root."
```

```yaml
packaged_resources:
  skills: full
  scripts: partial
  slash_commands: full
  subagents: full
  mcp_servers: partial
  hooks: none
  prompts: full
  config: partial
  assets: full
  other: ["themes"]
discovery:
  mechanism: "Provider scans enabled plugin directories at startup."
  precedence: "Repo resources shadow plugin resources; plugin resources shadow built-ins."
  namespacing: "Plugin commands are exposed as /plugin-name:command."
  conflicts: "Exact name conflicts are rejected at startup."
  notes: "Disabled plugins are ignored without deleting files."
```

```yaml
portability:
  link_plugin_as_unit: false
  extract_resources: true
  portable_resources: ["skills", "slash_commands", "subagents"]
  non_portable_assets: ["plugin manifest", "MCP server credentials", "postinstall script"]
  rewrite_needed: true
  notes: "Contained Markdown resources are portable after metadata rewrite; executable scripts are provider-specific."
```

## Research Questions

- How does the provider implement plugins as a container for agent resources?
- What can a plugin contain: Agent Skills, scripts, slash commands, subagents, MCP,
  hooks, prompts, config, assets, or other resources?
- Where are plugins installed by OS and scope?
- What manifest/package format is recognized?
- How are plugins installed, updated, removed, enabled, disabled, trusted, and versioned?
- How do CLI switches interact with the plugin functionality?
- Are there environment variables which impact how plugins operate?
- How are plugin-provided resources discovered, namespaced, ordered, and deconflicted?
- How do plugin resources interact with user/repo resources outside the plugin?
- Can plugins run executable code or scripts? If so, when and with what permissions?
- Can plugins access credentials, environment variables, filesystem paths, MCP servers,
  or provider config?
- Is there a marketplace, registry, Git/local install path, archive format, or private
  distribution mechanism?
- Should Claudine link the plugin as a unit, extract contained resources, rewrite
  metadata, or mark it non-portable?

## Body Structure

- `## Overview`
- `## Installation and Locations`
- `## Manifest and Package Format`
- `## Packaged Resources`
- `## Lifecycle and Trust`
- `## Discovery and Precedence`
- `## Security and Runtime Behavior`
- `## Distribution`
- `## Portability`
- `## Claudine Linking Notes`
- `## Changelog` when `update` is true
- `## Sources`

## Task

Follow these steps exactly:

::block when="update"
- Read existing research in `{{file}}`.

    > Prior research may be stale. Use it to preserve useful topics and write the
    > changelog, not as proof of current behavior.

::end-block
- Research the current behavior using official documentation first, then source code,
  release notes, `--help`, and local inspection where useful.
- Inspect `{{state.user_dir}}` when it exists and the provider stores plugins there.
  State what you observed, including when no local plugin/config resources exist.
::block when="update"
- Update `{{file}}` with current research and add a `## Changelog` entry.
::end-block
::block when="!update"
- Write and save the new research document to `{{file}}`.
::end-block
- Set all frontmatter required by `./_schema.yaml`.
- Cite sources as Markdown links in `## Sources`.

## Output

::file @prompts/make-it-markdown.md

## Exit Criteria

You are done when `{{file}}` has been saved with complete prose research, all
frontmatter fields populated appropriately, `$schema: ./_schema.yaml`, and
`md schema validate '{{file}}'` returns `true`.

- You do not need to run tests or lints.
- This task has no code modifications.
