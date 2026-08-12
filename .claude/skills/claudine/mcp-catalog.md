# MCP Catalog Management

The MCP catalog is saved in `~/.claudine/mcp/catalog.json` and represents all **possible** configurations that a user (and their repos) have used before.

It's important to recognize that the **catalog**:

- is that is **possible**, not
- what is **available** when you wrap an Agent execution

## Sensible Defaults

It is recommended that by default, no MCP servers are configured. This, however, may be an unpopular approach and if you want to specify -- at the _user_ or _repo_ level -- a set of MCP server's that you always want to be used you can do that too.

- this mapping of default MCP servers is done in two files:

    - `~/.claudine/mcp/defaults.json`
    - `<repo>/.claudine/mcp.json`

- the _user_ scoped MCP servers are a singular list,
- the _repo_ scoped MCP servers live in the repo that owns them
- the two configuration files will always exist (at least once MCP mode initialization has taken place). Again, by default we'll have **no** MCP servers as a default.

## Initializing MCP mode

The _initialization_ of the MCP mode of Claudine will happen under any of the following events:

- user runs `claudine mcp init`
- user starts a wrapped Agent with the `--mcp` flag (aka, reactively requesting use of MCP mode prior to initialization)

While both ways of initializing the catalog and default settings are completely fine, the `claudine mcp init` is often preferable because:

- this process of initialization is considered _proactive_ and will therefore be _interactive_
- we will initialize the catalog without interaction but in this use case we will then ask the user which of the configurations they want to ALWAYS have turned on at the User scoped level; this will be a multi-select widget from the `inquire` crate and the default will be to have no MCP servers selected.
- if the user has run this command inside a **git** repo then we will follow-up asking the user if they want any MCP servers to always be enabled for the repo.
- The two mapping files are then set and the configuration is complete:
    - We should list the user the catalog's entries so they are aware of what's possible
    - We can can then add a few instructive bullet points on how to use the catalog:
        - the tag syntax
        - the CLI command to create an _alias_ and how that convenience can make life easier for callers
        - etc.

### Re-entry to Initialization

If a user runs `claudine mcp init` _after_ the catalog has been setup already:

- if the user is in a **git** repo and that git repo has not been configured in `<repo>/.claudine/mcp.json` then we will:
    - remind them what MCP servers are _always_ included at the user level
    - ask them to choose any MCP servers they _always_ want to configure at the repo level
        - the options are a multi-select box
        - any options chosen at User scope
- in all other cases we should provide a "mini help" report:
    - tell them the User (or User/Repo) have already been initialized
    - let them know where the two configuration files are (make this an OSC8 link so they can easily bring up in an editor if they wish)
    - list out the CLI commands which are intended for MCP mode management


## CLI Management Commands

The MCP mode is activated by using the `--mcp` flag but there are several management functions associated with the MCP mode that are provided as well. A user can always choose to configure their MCP configuration by directly editing the config files but the CLI commands help ensure that valid configuration is always provided.

- `claudine mcp init` - already discussed in prior section, used to initialize the MCP catalog and setup defaults
- `claudine mcp add local` - adds a Local server
    - this process will be run through an interactive interview with the user
    - Name, Command and Params, optional ENV
- `claudine mcp add remote` - adds a remote MCP server
    - this process will be run through an interactive interview with the user
    - Name, URL, ENV, other?
- `claudine mcp alias <name> <alias>`
    - adds a new _alias_ to an existing MCP server in the catalog
    - both `name` and `alias` are optional parameters
        - those parameters which are not provided will be asked for in an interactive prompt
- `claudine mcp remove <name_or_alias>`
    - if we can match on a **name** in the catalog then we'll confirm with the user that the MCP server should really be removed and mention any aliases which point to this server (they will be removed as well)
    - if we don't match but do match on **alias** in the catalog then:
        - we will remove the alias without confirmation
        - we will report that the referenced MCP server _still_ exists and any other aliases it may still have
    - if no name or alias is provided then:
        - we will ask them to pass in a valid name or alias
        - we will provide a list of valid names (with an aliases they have in parenthesis)
- `claudine mcp list <filter>`
    - lists all the MCP server's in the catalog, filtered by the optional filter parameter passed in
    - The MCP report will be a table (using biscuit-terminal's `TerminalTable` struct). Columns include:
        - MCP server
        - Aliases
        - Type (local, SSE, JSON-RPC)
        - Auth (any auth requirements)
        - ENV
    - in addition to the normal "filter" parameter a user can use the `--alias <filter>` CLI switch to filter using aliases instead of names
    - any MCP servers listed in the tables will be bold-faced if it is a User or Repo default
- `claudine mcp config <name_or_alias>`
    - Returns the configuration for a particular MCP server in the catalog
- `claudine mcp check`
    - Checks that the current configuration is valid
- `claudine mcp sync`
    - This will re-evaluate all the Agent configurations for MCP and if it finds any new server's which are not currently in these configurations it will add it to the catalog and report the new server's availability
- `claudine mcp export <provider> [--scope user|repo] [--apply]`
    - This writes the effective Claudine defaults back into a provider's native MCP config

> **Note:** if the `claudine mcp` subcommand is provided without any additional specificity then we will run `claudine mcp list`.
