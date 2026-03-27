---
prompt: |-
    Your task is to do online research and detail out the way in which MCP servers are configured when working with OpenCode CLI.

    - which files are involved configuring MCP servers
        - is it just one file for User scope and another for Repo scope?
        - what about MCP's which were brought in via a plugin?
    - what command line CLI switches are provided that modify configuration?
    - What is the URL for documentation on MCP support for OpenCode CLI?

    Your final deliverable is a Markdown document (written to the body of this page) which well formed and idiomatic Markdown. Tables are Markdown tables. Links are Markdown links.  

    If you want visualize an idea then using a Mermaid block is the best way to do that.
---

# MCP Servers Configuration in OpenCode CLI

OpenCode supports MCP (Model Context Protocol) servers for extending tool capabilities. This document covers configuration files, CLI commands, and documentation references.

## Configuration Files

OpenCode uses a **merged configuration** approach rather than strict user/repo separation. Multiple config files are combined, with later sources overriding earlier ones for conflicting keys.

### Config File Locations

| Scope | Location | Description |
|-------|----------|-------------|
| **Remote** | `.well-known/opencode` | Organizational defaults (fetched from your provider) |
| **Global (User)** | `~/.config/opencode/opencode.json` | User-wide preferences |
| **Project** | `opencode.json` in project root | Project-specific settings |
| **Inline** | `OPENCODE_CONFIG_CONTENT` env var | Runtime overrides |

### Config Precedence Order

Config sources are loaded in this order (later sources override earlier ones):

1. Remote config (from `.well-known/opencode`) - organizational defaults
2. Global config (`~/.config/opencode/opencode.json`) - user preferences
3. Custom config (`OPENCODE_CONFIG` env var) - custom overrides
4. Project config (`opencode.json` in project) - project-specific settings
5. `.opencode` directories - agents, commands, plugins
6. Inline config (`OPENCODE_CONFIG_CONTENT` env var) - runtime overrides

### MCP Configuration Structure

MCP servers are configured under the `mcp` key in your config file:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "server-name": {
      "type": "local" | "remote",
      "command": ["npx", "-y", "my-mcp-command"],  // local only
      "url": "https://my-mcp-server.com",             // remote only
      "enabled": true,
      "environment": {},                              // local only
      "headers": {},                                  // remote only
      "oauth": {} | false,
      "timeout": 5000
    }
  }
}
```

### Plugins and MCP

**MCPs are not brought in via plugins.** MCP servers are configured directly in the OpenCode config file under the `mcp` section. Plugins serve a different purpose—they extend OpenCode through:

- Custom hooks (e.g., `tool.execute.before`, `session.created`)
- Custom tools
- Event handling

However, plugins can programmatically add MCP servers through the `mcp` configuration if needed, but this is not the typical pattern. The standard approach is to configure MCPs directly in the config file.

## CLI Commands for MCP Management

OpenCode provides several `opencode mcp` subcommands:

| Command | Description |
|---------|-------------|
| `opencode mcp add` | Interactive wizard to add a local or remote MCP server |
| `opencode mcp list` | List all configured MCP servers and their connection status |
| `opencode mcp ls` | Short alias for `list` |
| `opencode mcp auth [name]` | Authenticate with an OAuth-enabled MCP server |
| `opencode mcp auth list` | List OAuth-capable servers and their auth status |
| `opencode mcp auth ls` | Short alias for `auth list` |
| `opencode mcp logout [name]` | Remove OAuth credentials for an MCP server |
| `opencode mcp debug <name>` | Debug OAuth connection issues for a specific server |

### Example Usage

```bash
# Add a new MCP server interactively
opencode mcp add

# List all configured MCP servers
opencode mcp list

# Authenticate with a specific MCP server
opencode mcp auth sentry

# Debug OAuth issues
opencode mcp debug my-oauth-server
```

## Documentation URL

The official documentation for MCP servers in OpenCode CLI is available at:

**[https://opencode.ai/docs/mcp-servers/](https://opencode.ai/docs/mcp-servers/)**

Additional related documentation:
- [Config Overview](https://opencode.ai/docs/config/) - General configuration guidance
- [CLI Reference](https://opencode.ai/docs/cli/) - Full CLI command reference
- [Plugins](https://opencode.ai/docs/plugins/) - For extending OpenCode beyond MCP

## Local vs Remote MCP Servers

### Local MCP Servers

Run locally via command execution:

```json
{
  "mcp": {
    "my-local-mcp": {
      "type": "local",
      "command": ["npx", "-y", "@modelcontextprotocol/server-everything"],
      "environment": {
        "MY_ENV_VAR": "value"
      },
      "enabled": true
    }
  }
}
```

### Remote MCP Servers

Connect to remote HTTP-based MCP servers:

```json
{
  "mcp": {
    "sentry": {
      "type": "remote",
      "url": "https://mcp.sentry.dev/mcp",
      "oauth": {},
      "enabled": true
    }
  }
}
```

## OAuth Support

OpenCode automatically handles OAuth authentication for remote MCP servers:

- **Automatic**: Most OAuth servers work without special config
- **Pre-registered**: Provide `clientId` and `clientSecret` in config
- **Manual trigger**: Use `opencode mcp auth <server-name>`

Credentials are stored securely in `~/.local/share/opencode/mcp-auth.json`.

## Managing MCP Tools

MCP tools can be globally enabled/disabled via the `tools` config key:

```json
{
  "tools": {
    "my-mcp-server": false  // Disable all tools from this MCP
  }
}
```

Or use glob patterns:

```json
{
  "tools": {
    "my-mcp*": false  // Disable all MCPs matching pattern
  }
}
```

For per-agent MCP management, disable globally then enable in agent config:

```json
{
  "tools": {
    "my-mcp*": false
  },
  "agent": {
    "my-agent": {
      "tools": {
        "my-mcp*": true
      }
    }
  }
}
```
