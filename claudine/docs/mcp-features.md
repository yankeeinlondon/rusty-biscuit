# MCP Features

## Functional Areas

### 1. MCP Catalog Management & Initialization

- **Default Configurations**: Ability to set default MCP servers at both the User (`~/.claudine/user-mcp-defaults.json`) and Repo (`~/.claudine/repo-mcp-default.json`) scopes.
- **Initialization Trigger**: Trigger initialization automatically when a user runs `claudine mcp init` or passes the `--mcp` flag to a wrapped Agent.
- **Interactive Proactive Initialization**: When initialized via `claudine mcp init`, interactively ask the user to select User-scoped defaults via a multi-select widget (defaulting to none). If inside a git repo, prompt for Repo-scoped defaults.
- **Post-Initialization Help**: After interactive initialization, display the catalog entries and instructive bullet points on tag syntax, alias usage, etc.
- **Re-entry Initialization**: If `claudine mcp init` is run again:
    - If in an unconfigured git repo, remind about User defaults and prompt for Repo defaults.
    - Otherwise, display a "mini help" report detailing config file locations (with OSC8 links) and CLI management commands.

### 2. CLI Management Commands

- **`claudine mcp init`**: Command to initialize catalog and setup defaults interactively.
- **`claudine mcp add local`**: Interactive interview to add a Local server (Name, Command, Params, optional ENV).
- **`claudine mcp add remote`**: Interactive interview to add a Remote server (Name, URL, ENV).
- **`claudine alias <name> <alias>`**: Add an alias to a catalog entry, with interactive prompts for missing parameters.
- **`claudine mcp remove <name_or_alias>`**:
    - Remove a server by name, asking for confirmation, and cleaning up its aliases.
    - Cascade removal from user and repo defaults automatically.
    - Remove an alias without confirmation, reporting the owning server and remaining aliases.
    - Provide an interactive list of valid names/aliases if parameters are missing.
- **`claudine mcp list [filter]`**: Table display (via `TerminalTable`) of MCP servers, aliases, type, auth, and ENV. Support substring filtering and a `--alias` flag. Bold user/repo default entries. Maps to default `claudine mcp` call.
- **`claudine mcp config <name_or_alias>`**: Display the configuration for a specific server.
- **`claudine mcp check`**: Validate the current configuration.
- **`claudine mcp sync`**: Re-evaluate agent configurations to discover, import, and report any new MCP servers not in the catalog.

### 3. MCP Opt-In & Isolation (Agent Wrapping)

- **Opt-in Philosophy**: Require the `--mcp` flag to enable MCP servers in wrapped Agents to save context window.
- **Support for Interactive and Non-Interactive Modes**: Apply MCP opt-in logic to both modes seamlessly (e.g., `claudine codex --non-interactive "#tag"` and `claudine codex "#tag"`).
- **Inline Prompt Tag Syntax**: Detect tags in user prompts in the format `#<tag>`.
    - **Start Token**: Must begin with `#` at the start of the string or following a whitespace character, immediately followed by an alphabetical character (case-insensitive).
    - **End Token**: Must consist of valid characters (alphanumeric, `-`, `_`) and terminate at string end or a whitespace character.

### 4. Tag Matching & Resolution

- **Catalog Sourcing**: Source available MCP servers from User/Repo defaults and existing agent configurations.
- **Algorithmic Naming**: Auto-generate names for unnamed configurations using the executable name (stripping prefixes like `uv`, `npm`) or hashing the JSON definition with xxHash.
- **Exact Match Resolution**: Match the tag explicitly against the catalog names or aliases.
- **Caseless Match Resolution**: Match the tag case-insensitively against catalog names or aliases.
- **Substring Match Resolution**: Match the tag on any substring (starts with, ends with, contains).
    - Resolve uniquely if only one match is found.
    - If multiple matches: interactively ask the user to disambiguate, or error out immediately if `--strict` is used.
    - If no matches: remove the tag, warn via STDERR, and continue, or error out immediately if `--strict` is used.

## Source Code Review
*(Implementation status based on current repository state)*

### 1. MCP Catalog Management & Initialization

- **Default Configurations**: **Implemented**. Supported via `claudine::mcp::defaults` at both user and repo levels. Repo defaults **replace** user defaults (no merge).
- **Initialization Trigger**: **Implemented**. `mcp init` runs `McpImporter::import_all`, and `--mcp` on wrapped agents triggers reactive bootstrap.
- **Interactive Proactive Initialization**: **Implemented**. `mcp init` uses `MultiSelect` to choose user and repo defaults, with preselection of current user defaults during repo re-entry.
- **Post-Initialization Help**: **Implemented**. Displays catalog table, default summary, and instructive help on tags and aliases.
- **Re-entry Initialization**: **Implemented**. Shows current user defaults before repo prompt; displays file paths and management commands when fully initialized.

### 2. CLI Management Commands

- **`claudine mcp init`**: **Implemented**. Full interactive flow with re-entry awareness.
- **`claudine mcp add local`**: **Implemented**. Interactive interview for local stdio servers.
- **`claudine mcp add remote`**: **Implemented**. Interactive interview for remote HTTP servers.
- **`claudine alias <name> <alias>`**: **Implemented**. With interactive prompts for missing parameters.
- **`claudine mcp remove <name_or_alias>`**: **Implemented**. Removes servers (with confirmation) or aliases, cascades removal from user/repo defaults, and reports remaining aliases.
- **`claudine mcp list [filter]`**: **Implemented**. Table display with `--alias` filtering. Bold default entries.
- **`claudine mcp config <name_or_alias>`**: **Implemented**. Shows normalized definition plus provenance.
- **`claudine mcp check`**: **Implemented**. Validates transport, aliases, defaults, and provider-state.
- **`claudine mcp sync`**: **Implemented**. Catalog refresh (pull-only). Push-style export is a separate `export` command.

### 3. MCP Opt-In & Isolation (Agent Wrapping)

- **Opt-in Philosophy**: **Implemented**. The `--mcp` flag is required for MCP integration during agent wrapping.
- **Support for Interactive and Non-Interactive Modes**: **Implemented**.
- **Inline Prompt Tag Syntax**: **Implemented**. Extracts `#<tag>` tags from prompts via `lex_tags()`.

### 4. Tag Matching & Resolution

- **Catalog Sourcing**: **Implemented**. The catalog stores normalized MCP servers from providers.
- **Algorithmic Naming**: **Implemented**. Uses executable name heuristics and xxHash fallback.
- **Exact / Caseless / Substring Match Resolution**: **Implemented**. Four-tier resolution (exact ID, exact alias, caseless, substring) with interactive disambiguation and `--strict` mode.
- **Ambiguous Tag Behavior**: In interactive non-strict mode, cancelled disambiguation warns and drops the tag. In strict/non-interactive mode, ambiguity is a hard error.
