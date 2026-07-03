---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/mcp/{{state.file}}"
# NOTE: `grant:` is not implemented yet. Run with `--yolo` so the provider can
# inspect local MCP config examples under {{state.user_dir}} when they exist.
grant:
    read:
        - "{{state.user_dir}}"
agent: opencode
model: kimi-for-coding/k2p7
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
initialize:
    stack:
        - when: "file_exists(file) && frontmatter(file, 'last_updated') == ctx.today"
          action:
              - stderr: "Research for <b>{{state.name}}</b> MCP is already up to date ({{ctx.today}}) — skipping."
              - skip
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
---

## Skills

Use the 'claudine' skill.

## Scope

Research Model Context Protocol support for **{{state.desc}}**. This topic feeds
Claudine's MCP catalog, import/export/sync behavior, runtime injection, and provider
security posture. Write the result to `{{file}}` and include `$schema: ./_schema.yaml`
in frontmatter.

## Required Frontmatter

Populate every applicable field from `./_schema.yaml`:

- `created`, `last_updated`, `agent`, `model`
- `docs`
- `support`
- `config_files`
- `cli_params`
- `env_vars`
- `server_schema`
- `sync_behavior`
- `runtime_injection`
- `security`
- `changes`
- `requires_claudine_update`
- `reason`

Use `support: none` only when MCP support is clearly absent. Use `unknown` where the
current documentation does not prove the answer.

## Research Questions

- Does the provider support MCP by import/sync, runtime injection, manual config, or not at all?
- Where are MCP server definitions stored by OS and scope?
- What server definition shape is accepted for command, HTTP/SSE, stdio, auth, and env?
- Are there CLI switches or commands for listing, importing, exporting, applying, or syncing servers?
- Can Claudine inject MCP servers for one run without mutating user config?
- How are server trust, tool filtering, environment sanitization, sandboxing, and response filtering handled?
- Which environment variables affect MCP behavior?

## Body Structure

- `## Overview`
- `## Configuration`
- `## Server Definition Shape`
- `## Import, Export, and Sync`
- `## Runtime Injection`
- `## Security Model`
- `## Claudine Integration Notes`
- `## Changelog` when `update` is true
- `## Sources`

Use current official documentation and local inspection where available. Cite sources as
Markdown links.
