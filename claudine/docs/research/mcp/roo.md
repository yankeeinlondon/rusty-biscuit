---
prompt: |-
    Your task is to do online research and detail out the way in which MCP servers are configured when working with Roo Code.

    - which files are involved configuring MCP servers
        - is it just one file for User scope and another for Repo scope?
        - what about MCP's which were brought in via a plugin?
    - what command line CLI switches are provided that modify configuration?
    - What is the URL for documentation on MCP support for Roo Code?

    Your final deliverable is a Markdown document (written to the body of this page) which well formed and idiomatic Markdown. Tables are Markdown tables. Links are Markdown links.  

    If you want visualize an idea then using a Mermaid block is the best way to do that.
---

## Key Findings

### Configuration Files

Roo Code uses **two main configuration files** for MCP servers:

| Scope | File | Purpose |
|-------|------|---------|
| **Global (User)** | `mcp_settings.json` | Applies across all workspaces |
| **Project (Repo)** | `.roo/mcp.json` | Project-specific, can be version-controlled |

**Platform-specific paths for the global file:**

- **macOS:** `~/Library/Application Support/Code/User/globalStorage/rooveterinaryinc.roo-cline/settings/mcp_settings.json`
- **Windows:** `%APPDATA%\Code\User\globalStorage\rooveterinaryinc.roo-cline\settings\mcp_settings.json`
- **Linux:** `~/.config/Code/User/globalStorage/rooveterinaryinc.roo-cline/settings/mcp_settings.json`

### MCPs Installed via Marketplace/Plugins

MCPs installed through the Roo Code MCP Marketplace are stored in the **same configuration files** based on your chosen installation scope:

- **Project scope** → `.roo/mcp.json`
- **Global scope** → `mcp_settings.json`

### CLI Switches

Roo Code is a VS Code extension without native MCP CLI switches, but options include:

- VS Code CLI: `code --install-extension`
- Third-party `roo.py` installer with options like `--scope`, `--skip-env`, `--debug`
- VS Code Command Palette commands like `roo-cline.setCustomStoragePath`

### Documentation URL

The official MCP documentation for Roo Code is at: **https://docs.roocode.com/features/mcp/overview**

---

The full document includes detailed configuration examples, Mermaid diagrams showing the configuration architecture, platform-specific examples, and a comprehensive quick reference table.

