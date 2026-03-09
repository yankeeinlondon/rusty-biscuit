# MCP Support

Currently Claudine provides no MCP support but in this specification we will start adding it.

## Features

- `claudine mcp init` - cross platform configuration detection and ingestion
    - this will allow us to have a single file which represents a catalog of _all_ MCP servers the user has ever used:
        - `~/.claudine/mcp.json` 
    - we will have a _mapping_ file for each scope (user, repo) to map the default MCP configuration for each environment
        - `~/.claudine/user-mapping.json`
        - `~/.claudine/project-mapping.json`

    - this command can be run more than once but it's objectives are always the same ... to chronicle the full inventory of MCP servers and to map this catalog to various Agentic CLI's to indicate the platform's configuration

- `claudine mcp default`
    - Allows a user to specify which MCP servers should -- _by default_ -- be active for both:
        - User scope
        - Repo scope
    - Repo scope -- _when defined_ -- fully replaces User scope while in the directory tree of that 

- `claudine <agent> --mcp`
    - We can already wrap calls to agentic CLI's with commands like:
        - `claudine codex`, 
        - `claudine opencode`, etc. 
    - but when we add the `--mcp` flag we move away from each vendor's and repo's separate configurations and into the **claudine** based MCP configuration world.

## Claudine MCP Configuration

When we are in claudine's MCP mode by using the `--mcp` flag -- both when running non-interactive prompts or wrapping an Agent interactively -- the configuration for MCP becomes identical immediately because we are no longer using the provider's MCP configuration but Claudine's instead.

By default both the **user** and **repo** scoped MCP servers are empty in Claudine. This allows for the maximum context window to be given to the current problem you're trying to solve.

### Intentionality

- too often we setup MCP servers with the intention of using them only to find that 90% of the time we're just reducing the context window and the tool is not going to be used on the particular problem we have
- by defaulting to _nothing_ we can then allow a shorthand way of adding in an MCP server anytime we need it
- adding in an MCP server when we need it is done with an inline `#` and then the name of the MCP server you want to use
    - if I wrote the following prompt:

        "What is my first meeting today #google-calendar"

    - This would activate the Google Calendar MCP server (assuming you've configured it before)
    - Where the `#google-calendar` shows up in the prompt doesn't matter because when we detect the tag syntax we will use it to match to an MCP server in the catalog and then
        - add configuration for the matched MCP server to make it active in the current session
        - the `#tag` will be stripped out of the prompt as it's utility has been realized by activating the MCP server and is not an actual part of the prompt you want the Agent to see

### Tag Matching

Every MCP server has a name assigned to it and when a user adds a `#tag` syntax to their preliminary prompt then we will try for an exact match but we then immediately follow with a fuzzy finder that ignores case, allows `-` instead of `_` (and visa-versa) etc. 

- If all we get is partial (aka, substring) matches, and there is more than one, then we interactively ask the user to clarify which one they intended.
- If there is a single partial match we'll use that

Finally, for users who are using zsh completions we will also be able to autocomplete valid `#tag` values.

### Managing Tags

1. A user can always get a **list** of the available MCP tags available to them by running `claudine mcp`.

2. If there is a particular tag which a user uses a lot or where the MCP server's name is annoyingly long, the user can specify an **alias** with:

    ```sh
    claudine mcp alias [tag] [alias]
    ```

    If we ran `claudine mcp foobar foo` then we'd have an alias `#foo` which could be used synonymously to `#foobar`.

    - aliases assigned will be shown in a highlighted color when `claudine mcp` is run.

3. If an alias is no longer wanted then it can be easily removed with:

    ```sh
    claudine mcp remove alias [alias]
    ```

4. In similar fashion if I want to remove an entire catalog MCP server, you can do that too with a similar syntax:

    ```sh
    claudine mcp remove server [tag]
    ```

    Because this operation has greater impact then just removing an alias we will always present the user with a confirmation challenge before actually removing the MCP server.

> **Note:**
>
> - An **alias** may not take a name that is in direct conflict with a MCP server's name
> - Once an **alias** has been defined, new MCP server's added must avoid name conflicts with other MCP servers as well as aliased names.


## MCP Catalog

### Schema for Catalog

We need to have a `struct` which acts as the schema for our MCP Server. It should include:

- `type` - stdio | http | sse
- `command` - the executable command to run the server
- `args` - command line arguments for the server
- `env` - ENV variables
- `cwd` = the working directory for the server process

The above properties are nothing more than what Claude Code defines today. If we need to add more during the design or implementation we are at the liberty to do so.

**Note:** 

- the **command** is typically better save as a fully qualified filepath; some clients tend to fail when we assume the executable path will be available inside the Agentic CLI's environment

### Mapping Schemas

Currently the plan is to create user and repo-based mappings between the MCP servers and the provider's configuration. If we can do these lookups quickly then we might not need the mapping files.

- nothing is needed to have the provider's MCP configs behave normally because we're not removing or modifying their native configs
- instead, when we are using the MCP mode with the `--mcp` switch then we use must modify the environment in such a way that no MCP configuration is available as a baseline
    - we will probably use a technique similar to what we use for `--repo` flag to mask out all skills, agents, and commands from the user scope while maintaining the repo's assets 
