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
    - Remove an alias without confirmation, reporting the server's remaining aliases.
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

- **Default Configurations**: **Implemented**. Default configurations are supported via `claudine::mcp::defaults` at both user and repo levels.
- **Initialization Trigger**: **Partially Implemented**. The CLI has the `mcp init` command which runs `McpImporter::import_all`. It does not appear to trigger automatically from the `--mcp` flag on wrapped agents.
- **Interactive Proactive Initialization**: **Not Implemented**. The `mcp init` command simply scans and imports native provider configs without the interactive multi-select widgets specified.
- **Post-Initialization Help**: **Not Implemented**. The output of `mcp init` is a static summary of imported/merged servers, not the tutorial instructions described.
- **Re-entry Initialization**: **Not Implemented**. Running `init` multiple times just re-runs the importer without the context-aware "mini help" or git-repo prompts.

### 2. CLI Management Commands

- **`claudine mcp init`**: **Partially Implemented**. Exists but lacks the new interactive flow.
- **`claudine mcp add local`**: **Not Implemented**.
- **`claudine mcp add remote`**: **Not Implemented**.
- **`claudine alias <name> <alias>`**: **Implemented** (via `mcp alias add|remove`). However, interactive prompting for missing parameters is absent.
- **`claudine mcp remove <name_or_alias>`**: **Partially Implemented**. Exists as `remove <id>` with a confirmation step, but doesn't handle aliases transparently in a single command (`alias remove` is separate). Interactive lists for missing arguments are absent.
- **`claudine mcp list [filter]`**: **Partially Implemented**. Exists as the default `mcp` command. Uses a basic text table instead of `biscuit-terminal`'s `TerminalTable`. Substring filtering and `--alias` flags are not supported.
- **`claudine mcp config <name_or_alias>`**: **Implemented**. Exists as `claudine mcp show <id>`.
- **`claudine mcp check`**: **Not Implemented**.
- **`claudine mcp sync`**: **Partially Implemented**. The current `sync` command (`SyncExportArgs`) pushes Claudine's catalog to a provider, rather than pulling/discovering from agents into the catalog as described in the new specification.

### 3. MCP Opt-In & Isolation (Agent Wrapping)

- **Opt-in Philosophy**: **Implemented**. The `--mcp` flag exists and is required for MCP integration during agent wrapping.
- **Support for Interactive and Non-Interactive Modes**: **Implemented**.
- **Inline Prompt Tag Syntax**: **Not Implemented**. The current implementation expects servers to be specified via explicit `--use` arguments rather than extracting `#<tag>` tags directly from the prompt text.

### 4. Tag Matching & Resolution

- **Catalog Sourcing**: **Implemented**. The catalog stores normalized MCP servers from providers.
- **Algorithmic Naming**: **Partially Implemented**. The importer assigns names based on the native provider configurations, but advanced heuristic extraction (stripping `uv`/`npm`) or the `xxHash` fallback is missing.
- **Exact / Caseless / Substring Match Resolution**: **Not Implemented**. Since the inline tag syntax `#<tag>` and its runtime resolution logic are not yet implemented, the associated fuzzy matching, interactive disambiguation, and `--strict` behaviors are also missing. Catalog resolution currently uses strict exact matching on IDs via `catalog.resolve(id)`.
