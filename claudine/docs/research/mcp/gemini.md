---
prompt: |-
    Your task is to do online research and detail out the way in which MCP servers are configured when working with Gemini CLI.

    - which files are involved configuring MCP servers
        - is it just one file for User scope and another for Repo scope?
        - what about MCP's which were brought in via a plugin?
    - what command line CLI switches are provided that modify configuration?
    - What is the URL for documentation on MCP support for Gemini CLI?

    Your final deliverable is a Markdown document (written to the body of this page) which well formed and idiomatic Markdown. Tables are Markdown tables. Links are Markdown links.  

    If you want visualize an idea then using a Mermaid block is the best way to do that.
---

# Gemini CLI - Model Context Protocol (MCP) Configuration

This document outlines how Model Context Protocol (MCP) servers are configured when working with the Gemini CLI.

## Configuration Files and Scopes

MCP servers are configured via a `settings.json` file. The servers are defined within a top-level `mcpServers` object, which specifies connection details (such as the command, URL, or HTTP URL) and optional settings including arguments, environment variables, and tool filters.

The configuration files are separated by scope:

| Scope | File Path | Description |
| :--- | :--- | :--- |
| **User Scope** | `~/.gemini/settings.json` | Global configuration applied to all projects for the current user. |
| **Repo (Project) Scope** | `.gemini/settings.json` | Project-specific configuration located at the root of the repository. |
| **Enablement State** | `~/.gemini/mcp-server-enablement.json` | Tracks the enabled or disabled state of individual servers. |
| **OAuth Tokens** | `~/.gemini/mcp-oauth-tokens.json` | Securely stores OAuth tokens for remote MCP servers. |

### Configuration Precedence

When multiple configuration scopes are involved, the settings are merged. If there are naming conflicts, higher-level settings override lower-level ones:

```mermaid
graph TD
    A[Plugin/Extension MCP Servers] --> B[User settings.json]
    B --> C[Repo/Project settings.json]
    C --> D((Final MCP Server List))
    
    style A fill:#f9f,stroke:#333,stroke-width:2px
    style B fill:#bbf,stroke:#333,stroke-width:2px
    style C fill:#dfd,stroke:#333,stroke-width:2px
    style D fill:#fdd,stroke:#333,stroke-width:4px
```

## Plugins and Extensions

Plugins and extensions can bundle their own MCP servers. 

- **Definition:** These are defined in the `mcpServers` section of their respective `gemini-extension.json` manifest files.
- **Startup:** Bundled servers are loaded automatically when the CLI starts.
- **Precedence:** If a server name defined in an extension conflicts with one defined in a User or Repo `settings.json` file, the configuration in `settings.json` takes precedence.
- **Portability:** Extensions use the `${extensionPath}` variable to portably refer to internal files related to their bundled servers.

## Command Line Interface (CLI) Switches

The Gemini CLI provides the `gemini mcp` command group to manage MCP server configurations directly without manually editing the JSON files.

| Command | Description | Options / Switches |
| :--- | :--- | :--- |
| `gemini mcp add` | Adds a new MCP server configuration. | `-s, --scope` (user/project)<br>`-t, --transport` (stdio/sse/http)<br>`-e, --env`<br>`-H, --header`<br>`--timeout`<br>`--trust`<br>`--description`<br>`--include-tools`<br>`--exclude-tools` |
| `gemini mcp remove <name>` | Removes a server configuration by name. | `-s, --scope` |
| `gemini mcp list` | Displays all configured servers and their connection status. | |
| `gemini mcp enable <name>` | Enables a server. | `--session` (temporary change for the current session only) |
| `gemini mcp disable <name>` | Disables a server. | `--session` (temporary change for the current session only) |

### Session Restrictions

You can restrict which servers are allowed to connect during a single execution session using the main command switch:
`gemini --allowed-mcp-server-names <server_name>`

## Documentation URLs

For more details on MCP support in the Gemini CLI and the Model Context Protocol itself, refer to the following resources:

- **Model Context Protocol (Official Standard):** [https://modelcontextprotocol.io](https://modelcontextprotocol.io)
- **Official MCP Servers List:** [https://github.com/modelcontextprotocol/servers](https://github.com/modelcontextprotocol/servers)
