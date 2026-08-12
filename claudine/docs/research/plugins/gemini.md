---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7

homepage: https://geminicli.com/
docs: https://geminicli.com/docs/
plugin_docs: https://geminicli.com/docs/extensions/

support: first_class

locations:
  - os: macos
    scope: user
    path: ~/.gemini/extensions/<name>/
    notes: Installed extension root. Each extension is a copied directory with gemini-extension.json at the root.
  - os: linux
    scope: user
    path: ~/.gemini/extensions/<name>/
    notes: Same layout as macOS.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\extensions\\<name>\\"
    notes: Same layout as macOS.
  - os: macos
    scope: user
    path: ~/.gemini/extensions/<name>/.gemini-extension-install.json
    notes: Install metadata. Contains type field (github-release, git-clone, local) used by the update mechanism.
  - os: linux
    scope: user
    path: ~/.gemini/extensions/<name>/.gemini-extension-install.json
    notes: Install metadata.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\extensions\\<name>\\.gemini-extension-install.json"
    notes: Install metadata.
  - os: macos
    scope: user
    path: ~/.gemini/extensions/<name>/.env
    notes: Extension-specific environment file for declared settings. Loaded automatically.
  - os: linux
    scope: user
    path: ~/.gemini/extensions/<name>/.env
    notes: Extension-specific environment file.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\extensions\\<name>\\.env"
    notes: Extension-specific environment file.
  - os: macos
    scope: user
    path: ~/.gemini/skills/<name>/
    notes: User-scope Agent Skills directory. Also aliased as ~/.agents/skills/.
  - os: linux
    scope: user
    path: ~/.gemini/skills/<name>/
    notes: User-scope skills.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\skills\\<name>\\"
    notes: User-scope skills.
  - os: macos
    scope: repo
    path: .gemini/skills/<name>/
    notes: Workspace/project-scope Agent Skills. Aliased as .agents/skills/.
  - os: linux
    scope: repo
    path: .gemini/skills/<name>/
    notes: Workspace skills.
  - os: windows
    scope: repo
    path: ".gemini\\skills\\<name>\\"
    notes: Workspace skills.
  - os: macos
    scope: user
    path: ~/.gemini/commands/
    notes: User-scope custom slash commands (TOML files).
  - os: linux
    scope: user
    path: ~/.gemini/commands/
    notes: User commands.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\commands\\"
    notes: User commands.
  - os: macos
    scope: repo
    path: .gemini/commands/
    notes: Workspace custom slash commands.
  - os: linux
    scope: repo
    path: .gemini/commands/
    notes: Workspace commands.
  - os: windows
    scope: repo
    path: ".gemini\\commands\\"
    notes: Workspace commands.
  - os: macos
    scope: user
    path: ~/.gemini/agents/
    notes: User-scope subagent definition files (.md with YAML frontmatter).
  - os: linux
    scope: user
    path: ~/.gemini/agents/
    notes: User agents.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\agents\\"
    notes: User agents.
  - os: macos
    scope: repo
    path: .gemini/agents/
    notes: Workspace subagent definitions.
  - os: linux
    scope: repo
    path: .gemini/agents/
    notes: Workspace agents.
  - os: windows
    scope: repo
    path: ".gemini\\agents\\"
    notes: Workspace agents.
  - os: macos
    scope: repo
    path: .gemini/settings.json
    notes: Project settings can override extension MCP servers, hooks, and policies.
  - os: linux
    scope: repo
    path: .gemini/settings.json
    notes: Project settings override layer.
  - os: windows
    scope: repo
    path: ".gemini\\settings.json"
    notes: Project settings override layer.
  - os: macos
    scope: user
    path: ~/.gemini/settings.json
    notes: User settings layer.
  - os: linux
    scope: user
    path: ~/.gemini/settings.json
    notes: User settings layer.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\settings.json"
    notes: User settings layer.

manifest:
  file_names:
    - gemini-extension.json
  format: json
  required_fields:
    - name
    - version
  optional_fields:
    - description
    - mcpServers
    - contextFileName
    - excludeTools
    - migratedTo
    - plan
    - settings
    - themes
  package_layout: |
    Extension root contains gemini-extension.json plus optional component directories:
    - commands/ for custom slash-command TOML files
    - skills/<name>/SKILL.md for Agent Skills
    - agents/*.md for subagent definitions
    - hooks/hooks.json for lifecycle hooks
    - policies/*.toml for Policy Engine rules
    - GEMINI.md (or file named by contextFileName) for persistent context
    - package.json, src/, dist/ for Node.js MCP server code
    - .env for extension-specific settings injected at runtime
    The manifest name must match the extension directory name.
  notes: |
    The manifest is always required. contextFileName defaults to GEMINI.md when omitted but the file is present. mcpServers supports the same options as settings.json MCP config except trust. ${extensionPath}, ${workspacePath}, and ${/} are substituted in gemini-extension.json and hooks/hooks.json.

lifecycle:
  install: |
    gemini extensions install <source> [--ref <ref>] [--auto-update] [--pre-release] [--consent] [--skip-settings]. Source may be a GitHub repository URL or a local path. The CLI copies the extension into ~/.gemini/extensions/<name>/ and writes .gemini-extension-install.json. Installed extensions are enabled globally by default.
  update: |
    gemini extensions update <name> or gemini extensions update --all. Updates use GitHub API for GitHub releases, git ls-remote for git clones, and compare the source manifest version for local extensions. --auto-update on install enables automatic update checks.
  remove: |
    gemini extensions uninstall <name...> removes the extension directory under ~/.gemini/extensions/.
  enable_disable: |
    gemini extensions enable|disable <name> [--scope <scope>]. Scope is user or workspace. Extensions are enabled globally by default; disable prevents loading for that scope. All management operations require a CLI restart to take effect.
  trust: |
    Trust is established by explicit install/enable action with an interactive confirmation prompt. --consent skips the prompt. There is no code signing or sandbox boundary; the extension is trusted to run MCP servers, hooks, and scripts with user privileges.
  versioning: |
    Version is taken from the gemini-extension.json version field. For GitHub releases the CLI uses the latest release tag for update detection but displays the manifest version. For git clones it tracks HEAD commit. For local extensions it compares the source directory manifest version.
  notes: |
    gemini extensions link <path> creates a symbolic link from ~/.gemini/extensions/<name>/ to a local development directory so changes are reflected immediately after restart. gemini extensions validate <path> validates a local extension manifest.

packaged_resources:
  skills: full
  scripts: partial
  slash_commands: full
  subagents: full
  mcp_servers: full
  hooks: full
  prompts: none
  config: partial
  assets: full
  other:
    - themes
    - policies

discovery:
  mechanism: |
    Gemini CLI loads installed extensions from ~/.gemini/extensions/ at session startup and merges their configurations. Extension-provided skills are discovered alongside built-in, user, and workspace skills. Extension commands are merged with user and project commands. Extension hooks are merged from hooks/hooks.json. Extension MCP servers are merged with settings.json MCP servers.
  precedence: |
    Skills: built-in < extension < user (~/.gemini/skills/) < workspace (.gemini/skills/). Commands: extension commands have the lowest precedence; user (~/.gemini/commands/) and project (.gemini/commands/) commands take precedence. Hooks/settings: project settings > user settings > system settings > extensions. Workspace configuration takes precedence over extension configuration for MCP servers.
  namespacing: |
    Extension slash commands in subdirectories become namespaced with a colon (e.g., commands/gcs/sync.toml -> /gcs:sync). When an extension command conflicts with a user/project command, the extension version is prefixed with a dot using the extension name (e.g., /gcp.deploy). Skills bundled in extensions are exposed by their SKILL.md name and resolved by the standard discovery precedence.
  conflicts: |
    Exact skill name conflicts resolve in favor of the higher-precedence tier. Command conflicts are resolved by precedence and, for extension commands, by dot-prefix namespacing. MCP server name conflicts are resolved in favor of settings.json over the extension manifest.
  notes: |
    Extensions can also provide context via GEMINI.md (or contextFileName), which is loaded into the model's context for every session where the extension is active.

security:
  trust_model: |
    Extensions are highly trusted. Installation is the trust boundary; the user must confirm the security prompt (or pass --consent). There is no signature verification or sandbox isolation for extension code. Enterprise admins can disable extensions with admin.extensions.enabled: false or restrict installs with security.allowGitExtensionInstalls and security.allowedExtensions regex patterns.
  permissions: |
    Extension MCP servers, hook commands, and scripts run with user OS privileges. Extensions declare required environment variables in the settings array; only standard safe variables plus those declared variables are passed to extension processes. excludeTools can block specific tools (e.g., run_shell_command(rm -rf *)). Extension policies run in Policy Engine tier 2 and any allow decisions or yolo configurations are ignored.
  sandbox_interaction: |
    Extension-provided MCP servers, hooks, and scripts run outside the Gemini CLI sandbox (when sandboxing is enabled) and share the user's environment subject to the allowlist. The main CLI sandboxing applies to the agent's own tool execution, not to extension helper processes.
  credential_access: |
    Extensions cannot read arbitrary environment variables or host secrets. Sensitive settings marked sensitive: true are stored in the system keychain and injected as declared env vars. Non-sensitive settings are stored in the extension's .env file. Extension configs can reference $VAR_NAME, ${VAR_NAME}, or ${VAR_NAME:-DEFAULT_VALUE} syntax, which is resolved from the user's environment when settings are loaded.
  update_risk: |
    High for extensions installed with --auto-update or from git branches, because new commits are treated as updates. GitHub release installs pin to the Latest release unless --pre-release is used. Local linked extensions reflect source changes after restart. No rollback command is documented.
  notes: |
    Hooks execute arbitrary code and are fingerprinted at the project level; a changed command or name is treated as a new untrusted hook. Project-level hooks require workspace trust.

distribution:
  marketplace: true
  registry_url: https://geminicli.com/extensions/browse/
  source_types:
    - public GitHub repository
    - GitHub Releases archive
    - local folder or symlink
    - custom pre-built archives attached to GitHub Releases
  publishing: |
    Publish by hosting a public GitHub repository, adding the gemini-cli-extension topic, and ensuring gemini-extension.json is at the repository root. The gallery crawler indexes tagged repositories daily. No manual submission is required. GitHub Releases can distribute archives with platform-specific naming (darwin.arm64.my-tool.tar.gz, etc.).
  private_distribution: |
    Private Git repositories work via standard git credential helpers for manual install. There is no documented private registry beyond GitHub. Local folders and symlinks support internal/private extensions. Enterprise controls can whitelist or blacklist extension sources.
  notes: |
    The default extension registry URI is https://geminicli.com/extensions.json (experimental.extensionRegistryURI). Platform-specific archives use darwin/linux/win32 and x64/arm64; generic archives are used as fallback.

portability:
  link_plugin_as_unit: false
  extract_resources: true
  portable_resources:
    - skills
    - slash_commands
    - subagents
    - assets
  non_portable_assets:
    - gemini-extension.json manifest
    - MCP server configuration and credentials
    - settings array and .env values
    - hooks/hooks.json command definitions
    - theme JSON definitions
    - Policy Engine TOML rules
    - ${extensionPath} references
    - Node.js package.json and executable scripts
  rewrite_needed: true
  notes: |
    Extract Agent Skills (SKILL.md), custom commands (TOML), and subagents (.md frontmatter) for linking to other providers. Custom-command TOML syntax ({{args}}, !{...}, @{...}) is Gemini-specific and may need adaptation. Rewrite ${extensionPath} to the target provider's path variable or to relative paths. Do not link the manifest, MCP configs, hooks, themes, policies, or extension scripts intact because they depend on Gemini CLI runtime semantics and credentials.

cli_params:
  - flag: --extensions <extension_name ...> / -e <extension_name ...>
    description: Specifies a list of extensions to use for the session. Use gemini -e none to disable all extensions.
    example: gemini -e my-extension -e my-other-extension
  - flag: --list-extensions / -l
    description: Lists all available extensions and exits.
    example: gemini -l
  - flag: gemini extensions install <source> [--ref <ref>] [--auto-update] [--pre-release] [--consent] [--skip-settings]
    description: Install an extension from a GitHub URL or local path.
    example: gemini extensions install https://github.com/gemini-cli-extensions/workspace --ref stable
  - flag: gemini extensions uninstall <name...>
    description: Remove one or more installed extensions.
    example: gemini extensions uninstall my-extension
  - flag: gemini extensions list
    description: List installed extensions.
    example: gemini extensions list
  - flag: gemini extensions update [<name>] [--all]
    description: Update a named extension or all installed extensions.
    example: gemini extensions update --all
  - flag: gemini extensions enable <name> [--scope <scope>]
    description: Enable a disabled extension for user or workspace scope.
    example: gemini extensions enable my-extension --scope workspace
  - flag: gemini extensions disable <name> [--scope <scope>]
    description: Disable an extension for user or workspace scope.
    example: gemini extensions disable my-extension
  - flag: gemini extensions link <path>
    description: Symlink a local extension directory into ~/.gemini/extensions/ for development.
    example: gemini extensions link ./my-extension
  - flag: gemini extensions new <path> [template]
    description: Scaffold a new extension from a built-in template.
    example: gemini extensions new my-extension mcp-server
  - flag: gemini extensions validate <path>
    description: Validate an extension manifest locally.
    example: gemini extensions validate ./my-extension
  - flag: gemini extensions config [name] [setting] [--scope <scope>]
    description: Configure extension settings after installation.
    example: gemini extensions config my-extension API_KEY
  - flag: gemini extensions migrate
    description: Migrate hooks from Claude Code to Gemini CLI (only available under gemini hooks migrate).
    example: gemini hooks migrate

env_vars:
  - name: GEMINI_CLI_HOME
    effect: Root directory for Gemini CLI user configuration and storage; ~/.gemini is created inside it.
  - name: GEMINI_CLI_TRUST_WORKSPACE
    effect: If set to "true", trusts the current workspace for the session, bypassing the folder trust check.
  - name: GEMINI_CLI_TRUSTED_FOLDERS_PATH
    effect: Overrides the default location for trustedFolders.json.
  - name: GEMINI_API_KEY
    effect: API key for the Gemini API; one of several authentication methods.
  - name: GEMINI_MODEL
    effect: Specifies the default Gemini model to use.
  - name: GOOGLE_API_KEY
    effect: Google Cloud API key; required for Vertex AI express mode.
  - name: GOOGLE_CLOUD_PROJECT
    effect: Google Cloud project ID; required for Code Assist or Vertex AI.
  - name: GOOGLE_APPLICATION_CREDENTIALS
    effect: Path to Google Application Credentials JSON file.
  - name: GOOGLE_GEMINI_BASE_URL
    effect: Overrides the default base URL for Gemini API requests.
  - name: GOOGLE_VERTEX_BASE_URL
    effect: Overrides the default base URL for Vertex AI API requests.
  - name: GEMINI_SANDBOX
    effect: Alternative to the sandbox setting; accepts true, false, docker, podman, or a custom command string.
  - name: GEMINI_SYSTEM_MD
    effect: Replaces the built-in system prompt with content from a Markdown file.
  - name: GEMINI_WRITE_SYSTEM_MD
    effect: Writes the current built-in system prompt to a file for review.
  - name: SEATBELT_PROFILE
    effect: macOS-specific sandbox-exec profile selector (permissive-open, restrictive-open, strict-open, strict-proxied, or custom).
  - name: DEBUG / DEBUG_MODE
    effect: Enable verbose debug logging; excluded from project .env files by default.
  - name: NO_COLOR
    effect: Disables all color output in the CLI.
  - name: CLI_TITLE
    effect: Customizes the CLI window title.
  - name: GEMINI_CLI_SURFACE
    effect: Custom User-Agent label for API traffic reporting.
  - name: GEMINI_TELEMETRY_ENABLED
    effect: Overrides telemetry.enabled.
  - name: GEMINI_TELEMETRY_TRACES_ENABLED
    effect: Overrides telemetry.traces.
  - name: GEMINI_TELEMETRY_TARGET
    effect: Sets telemetry target (local or gcp).
  - name: GEMINI_TELEMETRY_OTLP_ENDPOINT
    effect: Sets the OTLP endpoint for telemetry.
  - name: GEMINI_TELEMETRY_OTLP_PROTOCOL
    effect: Sets the OTLP protocol (grpc or http).
  - name: GEMINI_TELEMETRY_LOG_PROMPTS
    effect: Enables/disables logging of user prompts.
  - name: GEMINI_TELEMETRY_OUTFILE
    effect: File path for local telemetry output.
  - name: GEMINI_TELEMETRY_USE_COLLECTOR
    effect: Enables/disables use of an external OTLP collector.
  - name: GOOGLE_CLOUD_LOCATION
    effect: Google Cloud project location; required for Vertex AI non-express mode.
  - name: GEMINI_CLI_SYSTEM_DEFAULTS_PATH
    effect: Overrides the system defaults file path.
  - name: GEMINI_CLI_SYSTEM_SETTINGS_PATH
    effect: Overrides the system settings file path.
  - name: CODE_ASSIST_ENDPOINT
    effect: Specifies the Code Assist server endpoint.

gaps:
  - No local extensions were installed on the inspected host; ~/.gemini/extensions/ did not exist. Layout is inferred from official docs and bundled examples.
  - Exact merge semantics for extension hooks with project/user hooks are not specified beyond the documented precedence order.
  - Subagents inside extensions are marked as a preview feature under active development.
  - No documented mechanism to pin an extension to a specific GitHub release tag after install except using --ref at install time.
  - No documented rollback or downgrade command for extension updates.

changes: []

requires_claudine_update: true
reason: |
  Claudine should model Gemini CLI extensions as a first-class plugin container. The linker should discover ~/.gemini/extensions/<name>/, read gemini-extension.json, and extract portable resources (skills, commands, subagents, static assets) while flagging Gemini-specific assets (manifest, MCP config, hooks, themes, policies, ${extensionPath} references, .env credentials, Node.js scripts) as non-portable. It should also account for extension-provided skills being discovered at lower precedence than user/workspace skills, and for slash-command namespacing using both colon (subdirectory) and dot (conflict) separators.
---

# Gemini CLI Extensions (Plugins)

## Overview

Gemini CLI does not use the word "plugin"; it calls the same packaging concept an **extension**. An extension is a directory with a `gemini-extension.json` manifest that bundles one or more agent resources—MCP servers, custom slash commands, Agent Skills, subagents, hooks, themes, policies, and a context file—into a single installable, versioned unit. Extensions are the closest equivalent to Claude Code's [plugins](https://code.claude.com/docs/en/plugins); the key difference is that Gemini's container is flatter (one manifest, one install directory) and is centered on GitHub-based distribution rather than a marketplace with installable marketplaces.

Extensions are loaded at session startup from `~/.gemini/extensions/<name>/`. Unlike Claude Code, which keeps a versioned cache and separate data directory, Gemini stores each installed extension as a single directory and writes install metadata to `.gemini-extension-install.json` inside that directory.

## Installation and Locations

Installed extensions live under the user home directory:

| Path | Purpose |
|------|---------|
| `~/.gemini/extensions/<name>/` | Copied or linked extension root. Must contain `gemini-extension.json`. |
| `~/.gemini/extensions/<name>/.gemini-extension-install.json` | Install metadata, including `type` (github-release, git-clone, local). |
| `~/.gemini/extensions/<name>/.env` | Extension-specific settings stored as environment variables. |

On this host, `~/.gemini/extensions/` did not exist because no extensions were installed; only `~/.gemini/skills/` (populated with symlinks to `~/.claude/skills/`) and other standard Gemini directories were present.

Extensions can also contribute resources that have their own standalone discovery paths. These paths are not inside the extension directory but are part of the same resource model:

| Resource | User scope | Workspace scope | Notes |
|----------|-----------|-----------------|-------|
| Agent Skills | `~/.gemini/skills/` (also `~/.agents/skills/`) | `.gemini/skills/` (also `.agents/skills/`) | Alias paths provide cross-tool interoperability. |
| Custom commands | `~/.gemini/commands/` | `.gemini/commands/` | TOML files; subdirectories become colon namespaces. |
| Subagents | `~/.gemini/agents/` | `.gemini/agents/` | Markdown files with YAML frontmatter. |
| Settings | `~/.gemini/settings.json` | `.gemini/settings.json` | Can override extension MCP servers and hooks. |

Installation scopes for enable/disable are `user` (global) and `workspace` (project). There is no `local` scope equivalent to Claude Code's `.claude/settings.local.json`.

## Manifest and Package Format

The manifest file is `gemini-extension.json` at the extension root.

Required fields:

- `name` — unique identifier and expected directory name; lowercase/numbers with dashes.
- `version` — semantic version string used for display and local-extension update comparison.

Optional fields:

- `description` — displayed in the gallery.
- `mcpServers` — map of MCP server configs. Supports all settings.json MCP options except `trust`.
- `contextFileName` — file to load as persistent context (defaults to `GEMINI.md` if present).
- `excludeTools` — array of tool names or command-specific restrictions to block.
- `migratedTo` — URL of a new repository; the CLI will migrate users on update.
- `plan` — planning configuration, e.g., `{ "directory": ".gemini/plans" }`.
- `settings` — array of `{ name, description, envVar, sensitive }` for user-provided values.
- `themes` — array of custom theme definitions.

A typical extension layout:

```text
my-extension/
├── gemini-extension.json
├── package.json
├── README.md
├── GEMINI.md
├── example.js
├── commands/
│   └── fs/
│       └── grep-code.toml
├── skills/
│   └── security-audit/
│       └── SKILL.md
├── agents/
│   └── reviewer.md
├── hooks/
│   └── hooks.json
├── policies/
│   └── policies.toml
└── .env
```

The `name` in `gemini-extension.json` must match the extension directory name. Path references inside the manifest should use `${extensionPath}` for portability.

## Packaged Resources

| Resource | Support | Location | Notes |
|----------|---------|----------|-------|
| Agent Skills | Full | `skills/<name>/SKILL.md` | Discovered with built-in/user/workspace skills; lower precedence than user/workspace. |
| Slash commands | Full | `commands/**/*.toml` | Subdirectories become colon namespaces; conflicts with user/project commands get dot-prefixed. |
| Subagents | Full | `agents/*.md` | Preview feature; Markdown with YAML frontmatter. |
| MCP servers | Full | `gemini-extension.json` `mcpServers` | Loaded on startup; `settings.json` MCP servers take precedence over extension MCP servers. |
| Hooks | Full | `hooks/hooks.json` | Merged with settings.json hooks; extension hooks have lowest precedence. |
| Themes | Full | `themes` array in manifest | Selectable via `/theme` or `ui.theme`. |
| Policies | Full | `policies/*.toml` | Run in Policy Engine tier 2; `allow` and yolo are ignored. |
| Context/prompts | Partial | `GEMINI.md` or `contextFileName` | Acts as persistent system context, not a standalone prompt container. |
| Scripts/executables | Partial | `package.json`, `src/`, `dist/`, scripts referenced by hooks/MCP | No automatic discovery as standalone commands; runs as Node.js MCP server or hook command. |
| Config | Partial | `settings` array, `plan`, `excludeTools` | Configuration surface is narrower than Claude Code's plugin manifest. |
| Assets | Full | Any files in the extension directory | Referenced by skills, commands, or MCP server code. |

## Lifecycle and Trust

Install:

```bash
gemini extensions install <source> [--ref <ref>] [--auto-update] [--pre-release] [--consent] [--skip-settings]
```

The CLI copies the source into `~/.gemini/extensions/<name>/`. For GitHub sources, `git` must be installed. GitHub Releases are preferred for speed and platform binaries.

Update:

```bash
gemini extensions update <name>
gemini extensions update --all
```

Update detection depends on the install type:

- GitHub releases: query GitHub API for the Latest tag.
- Git clone: compare remote HEAD with local HEAD via `git ls-remote`.
- Local extension: compare source directory manifest version with installed version.

Remove:

```bash
gemini extensions uninstall <name...>
```

Enable/disable:

```bash
gemini extensions enable|disable <name> [--scope <scope>]
```

Scope is `user` or `workspace`. Newly installed extensions are enabled globally by default. All management operations require a CLI restart to take effect.

Trust:

- No code signing, sandboxing, or verification beyond the install confirmation prompt.
- `--consent` skips the confirmation; otherwise the user must approve.
- Enterprise controls can disable extensions (`admin.extensions.enabled: false`), block Git installs (`security.allowGitExtensionInstalls: false`), or allow only matching names (`security.allowedExtensions` regex list).
- Hooks and MCP server code run with user privileges.

Versioning:

- The `version` field is required.
- GitHub release updates use tags, not the manifest version, but the manifest version is displayed.
- Git branch installs treat `HEAD` as latest.
- `migratedTo` allows seamless repository migration when an update is found in the old repo.

## Discovery and Precedence

Discovery order for skills (lowest to highest precedence):

1. Built-in skills
2. Extension skills
3. User skills (`~/.gemini/skills/` or `~/.agents/skills/`)
4. Workspace skills (`.gemini/skills/` or `.agents/skills/`)

Within the user or workspace tier, the `.agents/skills/` alias takes precedence over `.gemini/skills/`.

Command precedence:

1. Extension commands (lowest)
2. User commands (`~/.gemini/commands/`)
3. Project commands (`.gemini/commands/`) (highest)

When an extension command conflicts with a user/project command, the extension command is renamed to `/extensionName.command` using a dot separator. Extension subdirectory commands use a colon separator, e.g., `/gcs:sync`.

MCP servers and hooks merge with settings.json layers. Project settings > user settings > system settings > extensions.

Extension context (`GEMINI.md` or `contextFileName`) is loaded into the model context for every session where the extension is active, alongside hierarchical `GEMINI.md` files.

## Security and Runtime Behavior

Trust model:

- Installation is the trust boundary; no signature or sandbox verification.
- Users must trust the GitHub repo, local path, or release archive.
- Enterprise policy can restrict or disable extensions.

Permissions:

- Extension MCP servers, hooks, and scripts execute with the user's OS permissions.
- Environment variable sanitization: extensions and MCP servers receive only standard safe variables plus variables explicitly declared in the manifest `settings` array.
- Sensitive settings (`sensitive: true`) are stored in the system keychain and injected as declared env vars.
- `excludeTools` can restrict dangerous tools.
- Extension policies cannot approve tool calls or enable yolo mode.

Sandbox interaction:

- Extension helper processes run outside the Gemini CLI sandbox.
- The main agent's sandboxing applies to its own tool execution, not extension MCP servers or hook scripts.

Credential access:

- Extensions cannot read arbitrary host env vars or secrets.
- Declared settings are the only way to pass secrets into extension processes.
- Manifest strings can reference `$VAR_NAME`, `${VAR_NAME}`, or `${VAR_NAME:-DEFAULT_VALUE}` from the user's environment, but this is resolved at settings load time, not inside the extension sandbox.

Update risk:

- Auto-updates and branch-based installs can silently change behavior on restart.
- GitHub release installs are pinned to the Latest release unless `--pre-release` is used.
- No documented rollback command.

## Distribution

Gemini CLI extensions are distributed primarily through GitHub:

- Public gallery: [geminicli.com/extensions/browse/](https://geminicli.com/extensions/browse/)
- Discovery: add the `gemini-cli-extension` topic to a public GitHub repo; the crawler indexes tagged repos daily.
- Install sources:
  - GitHub repository URL (`gemini extensions install https://github.com/owner/repo`)
  - GitHub Releases archive (faster, supports platform-specific binaries)
  - Local folder or symlink (`gemini extensions link <path>`)
- Platform-specific archive naming: `{platform}.{arch}.{name}.{ext}` or `{platform}.{name}.{ext}`, where platform is `darwin`, `linux`, or `win32`; arch is `x64` or `arm64`; ext is `.tar.gz` or `.zip`.
- Private distribution: private Git repos via git credentials, local folders, or enterprise source control; no documented private marketplace.

Comparison with Claude Code: Claude Code supports installable marketplaces (official, community, third-party) via `claude plugin marketplace add`, version pinning, and managed enterprise settings. Gemini CLI has a public gallery but no equivalent installable marketplace concept; every install is either a GitHub URL, a GitHub Release, or a local path.

## Portability

Claudine should not link Gemini CLI extensions as intact units to other providers. Instead, extract the portable Markdown-based resources and flag Gemini-specific assets for rewrite or omission.

Portable resources:

- `skills/<name>/SKILL.md` — standard Agent Skills format.
- `commands/**/*.toml` — custom slash commands (syntax may need rewriting for other providers).
- `agents/*.md` — subagent definitions (YAML frontmatter may need field mapping).
- Static assets referenced by skills or commands.

Non-portable assets:

- `gemini-extension.json` manifest.
- MCP server configs and Node.js server code.
- `settings` array values and `.env` credentials.
- `hooks/hooks.json` hook command definitions.
- `themes` array definitions.
- `policies/*.toml` Policy Engine rules.
- `${extensionPath}`, `${workspacePath}`, and `${/}` references.
- `package.json` and build artifacts.

Rewrite is needed for slash-command TOML syntax (e.g., `!{...}` shell injection, `@{...}` file injection, `{{args}}`), for `GEMINI.md` context references, and for any path variables. Scripts should be treated as host-dependent and linked only with explicit OS gating or replacement.

## Claudine Linking Notes

- Treat `~/.gemini/extensions/` as the canonical extension state root; respect `GEMINI_CLI_HOME` if set.
- For each installed extension, read `gemini-extension.json` and `.gemini-extension-install.json`.
- Extract portable Markdown resources from `skills/`, `commands/`, and `agents/`.
- Rewrite namespaced slash commands into the target provider's format; preserve both colon-based subdirectory namespacing and dot-based conflict namespacing metadata.
- Do not extract or link non-portable assets (MCP configs, hooks, themes, policies, scripts, `.env`) without explicit user confirmation and host-aware rewriting.
- When linking Gemini-provided skills to another provider, prefer the standalone `~/.gemini/skills/` or `.gemini/skills/` paths because Gemini discovers them through the same standard as extensions but at higher precedence.
- Account for the extension skill precedence being lower than user/workspace skills; avoid creating name collisions unless the target provider also namespaces extension resources.
- Respect enterprise restrictions (`admin.extensions.enabled`, `security.allowGitExtensionInstalls`, `security.allowedExtensions`) and do not suggest linking extensions from blocked sources.

## Sources

- [Gemini CLI — Extensions overview](https://geminicli.com/docs/extensions/)
- [Gemini CLI — Extension reference](https://geminicli.com/docs/extensions/reference/)
- [Gemini CLI — Build extensions](https://geminicli.com/docs/extensions/writing-extensions/)
- [Gemini CLI — Release extensions](https://geminicli.com/docs/extensions/releasing/)
- [Gemini CLI — Agent Skills](https://geminicli.com/docs/cli/skills/)
- [Gemini CLI — Custom commands](https://geminicli.com/docs/cli/custom-commands/)
- [Gemini CLI — Subagents](https://geminicli.com/docs/core/subagents/)
- [Gemini CLI — Hooks](https://geminicli.com/docs/hooks/)
- [Gemini CLI — Configuration reference](https://geminicli.com/docs/reference/configuration/)
- [Gemini CLI — Command reference](https://geminicli.com/docs/reference/commands/)
- [Gemini CLI Extensions gallery](https://geminicli.com/extensions/)
- Local Gemini CLI installation docs at `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/docs/extensions/`
