# MCP Isolation

Sometimes it feels good to add an MCP server that we deem as being useful for some of the work we're doing. The problem is that MCP servers can eat into the available context window and it's far too easy to accidentally leave MCP servers in sessions which will definitely NOT benefit from them.

For this reason, rather than an opt-out philosophy, Claudine espouses an "opt-in" approach to MCP services. To use this philosophy when using Claudine's _wrapping_ functionality we add the `--mcp` flag to the call.

When the wrapped provider cannot accept runtime MCP injection, Claudine stops and directs the user to `claudine mcp export <provider> --apply` instead of silently proceeding.

## Opting In

When a user has passed in a non-interactive prompt to an Agent for processing or to kick off a new interactive session, the process is the same:

```sh
# run an non-interactive session and add the `brave-search` MCP server
claudine codex --non-interactive "#brave-search Who is donald duck?"
# start an interactive session and opt into the `brave-search` MCP server
claudine codex "#brave-search"
```

> These two examples both showing using the `codex` Agent in interactive or non-interactive modes using the `#{tag}` syntax which indicates MCP opt in

### Opt In Syntax

The prompt provided to a wrapped agent can include one or more "tags" which take the form of: `#${tag}` string where:

- Start Token:

    - the `#` symbol is either at the start of the prompt string or has a whitespace character before it
    - the character immediately following the `#` character is an alphabetical character

    > Note: alphabetic character casing is ignored when considering matching

- End Token:

    A tag is terminated successfully when:

    - all characters since the start token are "valid characters"; valid characters are:
        - alphanumeric and `-`, `_` characters
    - a "terminal condition" is met in the prompt; terminal conditions are:
        - the end of the line is reached
        - a whitespace character is found

### Tag Matching

The goal is to _find a match_ between the **tag** the user provided and an MCP server in Claudine's configuration catalog.

#### The Catalog

- The **catalog** of MCP servers _available_ are initially sourced from EVERY MCP configuration from both the User and Repo scope is added to Claudine catalog
- The user from that point on can _manage_ their Claudine configuration independently from Agent specific MCP configuration
    - use `claudine mcp add <config>` to add a configuration
    - use `claudine mcp remove <name>` to remove a configuration
    - use `claudine mcp alias <name> <alias>` create an "alias" for a MCP server
    - use `claudine mcp list <filter>` to list the catalog and optional filter on a substring
    - for more details on managing the catalog refer to: [MCP Catalog Management](./mcp-catalog.md)
- Every item in the catalog will have a **name** which we are trying to match against
    - if the underlying configuration didn't include a "name" property in their definition we will algorithmically create one:
        - the name assigned when no name is expressed and the agent is executed locally, is the "executable" name
            - we will, however, be smart about known prefixes like `uv`, `npm`, etc. which point to the server which is being specified
            - example: `uv some-server` as the server executable would create a name of `some-server` not `uv`
        - if no known means is available to provide a "name" meaningfully (e.g., with semantic understanding) we will instead opt to hash the entire configuration JSON with xxHash (biscuit-hash library) and give it a hash name
            - the user can later rename it or create an alias to it to make usage more convenient

#### Matching Process

- **Exact** First
    - We _will_ use fuzzy matching to aid in matching but to start we will try to match explicitly
        - if there is a direct name match between the **tag** and a **name** in the catalog then we resolve to this
        - if there is a direct name match between the **tag** and an **alias** in the catalog then we resolve to this
- **Caseless** Matching
    - Next we will use the same "exact match" approach but lowercase all names and tags in the comparison to make casing irrelevant to matching
- **Substring** Matching
    - This matcher will first create a _list_ of matches rather than a singular match
    - This matcher is always done in a caseless manner and will match on any substring (e.g., "starting with", "ending with", "has in it's interior")
        - if a singular item matches the result then this is used
        - if MORE than one item matches:
            - by default, the user is interactively requested to specify which tag (or tags) were intended
            - in non-interactive runs, ambiguity is treated as a hard error because there is no safe prompt path
            - if the CLI command contains the `--strict` flag then instead of asking it will simply exit in an error. The error will express what the ambiguous tag was and what the possible matches were
        - if NO matches were found:
            - by default, we remove the tag reference from the prompt, report a warning to STDERR, but execute the Agent
            - if the `--strict` flag is used, however, we will immediately stop with an error describing what has happened
