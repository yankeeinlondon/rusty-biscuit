---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/acp/{{state.file}}"
# NOTE: `grant:` is not implemented yet. Run with `--yolo` so the provider can
# inspect local ACP/auth/config examples under {{state.user_dir}} when they exist.
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
              - stderr: "Research for <b>{{state.name}}</b> ACP is already up to date ({{ctx.today}}) — skipping."
              - skip
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The **ACP** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **ACP** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the ACP research on **{{state.name}}** failed to complete!"
    warn: "The ACP research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---

## Skills

Use the 'claudine' and 'acp' skills.

## Scope

Do a deep dive on the Agent Client Protocol implementation or adapter available for
**{{state.desc}}**. This topic feeds future Claudine ACP client/adapter work: launching
providers over ACP, handling reverse requests, routing streaming updates to UI layers,
and enforcing host-side filesystem, terminal, and permission policy.

Write the result to `{{file}}` and include `$schema: ./_schema.yaml` in frontmatter.

## Required Frontmatter

Populate every applicable field from `./_schema.yaml`:

- `created`, `last_updated`, `agent`, `model`
- `docs`, `acp_docs`, `repo`
- `support`
- `launch_modes`
- `protocol_versions`
- `capabilities`
- `reverse_requests`
- `permission_model`
- `filesystem_model`
- `terminal_model`
- `streaming_model`
- `auth_setup`
- `env_vars`
- `rust_client`
- `compatibility`
- `recent_changes`
- `quirks`
- `gaps`
- `changes`
- `requires_claudine_update`
- `reason`

Use `support: none` only when there is clear evidence that the provider has no ACP
mode and no maintained adapter. Use `adapter` when ACP support exists through a bridge
process rather than the provider's primary CLI binary.

## Research Questions

- Does the provider expose native ACP mode, an adapter package, partial support, or no support?
- What exact command launches the ACP agent, and what transport/framing does it use?
- Which ACP protocol version or SDK/schema release is supported?
- Which session, streaming, permission, filesystem, terminal, MCP, auth, media, plan, and extension capabilities are supported?
- Which reverse requests can the agent send to the client, and which ones must a Claudine client implement?
- How should a client respond to permission requests, file reads/writes, and host command execution requests?
- What authentication or setup must happen before the ACP agent can run headlessly?
- What quirks, gotchas, client incompatibilities, version mismatches, or adapter issues do developers report, and what workarounds exist?
- What recent changes affect ACP support?

## Rust Sections Required In Body

After the provider deep dive, add Rust-oriented sections that cover:

1. How a Rust client can interact programmatically with the agent using ACP.
2. How to handle reverse requests where the agent asks the client to fulfill a tool request, file read/write, permission prompt, or similar operation.
3. How a Rust client can respond to requests to execute commands on the host system.
4. How a Rust client can use `mpsc` channels to send streaming text and events to a desktop app framework such as Tauri or iced.

Prefer the official `agent-client-protocol` Rust crate when it fits. If provider support
requires an adapter or a lower-level JSON-RPC implementation, explain why.

## Body Structure

- `## Overview`
- `## Launching ACP`
- `## Protocol and Capabilities`
- `## Reverse Requests`
- `## Permissions, Filesystem, and Terminal`
- `## Streaming and UI Integration`
- `## Authentication and Setup`
- `## Compatibility, Quirks, and Workarounds`
- `## Recent Changes`
- `## Rust Client Example`
- `## Rust Reverse Request Handling`
- `## Rust Host Command Handling`
- `## Rust Desktop Streaming Bridge`
- `## Claudine Integration Notes`
- `## Changelog` when `update` is true
- `## Sources`

Use current official documentation, adapter documentation, issue trackers, and local
inspection where available. Cite sources as Markdown links.

Do not add thinking or preparatory statements to the document body. Those can go to
stdout during the run, but the saved Markdown body must contain only the research.
