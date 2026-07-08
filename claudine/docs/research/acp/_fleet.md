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
        - when: "!file_exists(file) || !frontmatter(file, 'last_updated') || date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - message: "The provider **{{state.name}}** needs to update its research on **ACP**"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - stderr: "The provider **{{state.name}}** has research for **ACP** that is current; skipping updates"
              - skip
success:
    stack:
        - when: "!file_exists(file) || frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The **ACP** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **ACP** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the ACP research on **{{state.name}}** failed to complete!"
    warn: "The ACP research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---
# ACP Research on {{state.name}}

## Skills

Use the 'claudine' and 'acp' skills.

## Scope

Do a deep dive on the Agent Client Protocol implementation or adapter available for
**{{state.desc}}**. This topic feeds future Claudine ACP client/adapter work: launching
providers over ACP, handling reverse requests, routing streaming updates to UI layers,
and enforcing host-side filesystem, terminal, and permission policy.

**Boundary:** the `non-interactive-sessions` and `streaming` topics own each provider's
proprietary stream/output protocols; this topic covers only the Agent Client Protocol
(JSON-RPC) surface and its adapters — mention a proprietary protocol only where an
adapter translates it to ACP.

Prior-generation research files in this directory (`gemini-cli.md`, `kimi-code-cli.md`)
are validation assets for humans — do NOT open, paraphrase, or cite them. The standalone
`json-rpc.md` background document is likewise not provider evidence for this run; your
research must be independent.

## Document Structure

The research deliverable is a prose document a maintainer can learn the provider's ACP
story from. Write the body of `{{file}}` using these sections; frontmatter is distilled
from this body afterward, never invented separately.

- `## Overview` Section
    - Classify the provider's ACP support: a native ACP mode in the primary CLI
      binary, an adapter/bridge package, partial support, or none. The
      adapter-vs-native distinction is the load-bearing fact of this topic — claim
      `none` only when there is clear evidence the provider has no ACP mode **and**
      no maintained adapter; classify as adapter when ACP support exists through a
      bridge process rather than the provider's primary CLI binary
    - When support is adapter-based, name the adapter package(s) and explain what the
      bridge translates between
- `## Launching ACP` Section
    - The exact command that launches the ACP agent, its arguments, and the
      transport/framing it uses (stdio JSON-RPC, HTTP, WebSocket, …)
    - Distinguish clearly between launching the provider's own binary and launching
      an adapter that spawns it
- `## Protocol and Capabilities` Section
    - Which ACP protocol version or SDK/schema release is supported, and how you
      verified it (initialize handshake, adapter changelog, source code)
    - Which session, streaming, permission, filesystem, terminal, MCP, auth, media,
      plan, and extension capabilities are supported, partial, or unsupported
- `## Reverse Requests` Section
    - Which reverse requests the agent can send to the client (permission prompts,
      file reads/writes, terminal lifecycle, …), with example payloads where
      documented
    - Which of these a Claudine client must implement before the agent is usable at
      all, versus which are capability-gated
- `## Permissions, Filesystem, and Terminal` Section
    - How a client should respond to permission requests, file read/write requests,
      and host command execution requests
    - Path conventions (absolute vs relative, line-number base), sandboxing
      expectations, and the process-lifecycle responsibilities the client carries
- `## Streaming and UI Integration` Section
    - Which streaming update notification types the provider or adapter emits — text,
      thought, tool, plan, mode — and how a client should route them into a UI event
      loop
- `## Authentication and Setup` Section
    - What authentication or setup must happen before the ACP agent can run
      headlessly, and which environment variables affect launch, auth, or transport
- `## Compatibility, Quirks, and Workarounds` Section
    - Quirks, gotchas, client incompatibilities, version mismatches, or adapter
      issues developers report, with workarounds — name the affected client and cite
      the issue where possible
- `## Recent Changes` Section
    - Recent provider, adapter, or SDK changes that affect ACP support, with dates or
      versions
- Rust sections — after the provider deep dive, add four Rust-oriented sections.
  Prefer the official `agent-client-protocol` Rust crate when it fits; if provider
  support requires an adapter or a lower-level JSON-RPC implementation, explain why:
    - `## Rust Client Example` Section — how a Rust client can interact
      programmatically with the agent using ACP
    - `## Rust Reverse Request Handling` Section — how to handle reverse requests
      where the agent asks the client to fulfill a tool request, file read/write,
      permission prompt, or similar operation
    - `## Rust Host Command Handling` Section — how a Rust client can respond to
      requests to execute commands on the host system
    - `## Rust Desktop Streaming Bridge` Section — how a Rust client can use `mpsc`
      channels to send streaming text and events to a desktop app framework such as
      Tauri or iced
- `## Claudine Integration Notes` Section
    - A practical synthesis: what adding ACP support for this provider would require
      from Claudine — launch detection, capability negotiation, reverse-request
      routing, streaming bridge, and auth preconditions
- `## Changelog` Section (update runs only)
    - Summarize what changed since the prior research
- `## Sources`
    - add all useful resources you used as Markdown links — official docs, adapter
      docs, issue trackers, and local inspection

Do not add thinking or preparatory statements to the document body. Those can go to
stdout during the run, but the saved Markdown body must contain only the research.

## Task

Follow these steps exactly:

::block when="update"
- Read existing research in `{{file}}`

    > **Note:** the speed at which Agentic CLIs change is rapid and therefore you
    > should assume that the prior research is out of date. You are reading this
    > primarily to be able to effectively report the changes into the `## Changelog`
    > section of the document. Critically, you should never substitute information in
    > the old research for doing your own (up-to-date) research.

::end-block
- Perform research on the topic

    > **Evidence requirement:** you have read access to `{{state.user_dir}}` on this
    > host. Inspect any actual ACP, auth, or config artifacts there when they exist,
    > and prefer what you observe over what documentation claims. Negative probes are
    > evidence too — "the installed CLI rejects an ACP flag" is a finding. Unanswered
    > is not the same as omitted: record `unknown` with a note rather than dropping a
    > field.

::block when="update"
- Update the document with your research
- Add an entry to the `## Changelog` section
::end-block
::block when="!update"
- Write and save the research to `{{file}}`, following the Document Structure above
::end-block
- Set the `$schema` property of `{{file}}` to the string `./_schema.yaml`

    > This is a file reference to this topic's schema sidecar. Read `_schema.yaml`
    > (it sits next to this sequence file) before filling frontmatter — it is the
    > authoritative field contract, and `md schema validate` will enforce it against
    > everything you write.

- Now capture the facts you documented above into the document's frontmatter:
    ::block when="!update"
    - `created` - set to "{{ctx.today}}"
    ::end-block
    - `last_updated` - set to "{{ctx.today}}"
    - `agent` - set to "{{env.AGENT}}"
    - `model` - set to "{{env.MODEL || 'default' }}"
    - `docs`, `acp_docs`, `repo` - the URLs cited in `## Overview` and `## Sources`
    - `support` - your `## Overview` classification. Use `none` only with clear
      evidence of no ACP mode and no maintained adapter; use `adapter` when support
      rides on a bridge process
    - `launch_modes` - from `## Launching ACP`
    - `protocol_versions` and `capabilities` - from `## Protocol and Capabilities`
    - `reverse_requests` - from `## Reverse Requests`
    - `permission_model`, `filesystem_model`, `terminal_model` - from
      `## Permissions, Filesystem, and Terminal`
    - `streaming_model` - from `## Streaming and UI Integration`
    - `auth_setup` and `env_vars` - from `## Authentication and Setup`
    - `rust_client` - from the Rust sections
    - `compatibility` and `quirks` - from `## Compatibility, Quirks, and Workarounds`
    - `recent_changes` - from `## Recent Changes`
    - `gaps` - claims you could not verify from docs or local inspection
    ::block when="update"
    - `changes` - add a list of string descriptions which summarize the changes discovered since the last research was done
    ::end-block
    ::block when="!update"
    - `changes` - set to `[]`
    ::end-block
    - `requires_claudine_update` - set to true/false based on whether you believe there will be required code changes to **Claudine** based on the changes discovered in your research.
        - If you respond with `true` then you must also set the `reason` frontmatter property to describe why you think that

## Output

::file @prompts/make-it-markdown.md

## Exit Criteria

You are done with this task when the Markdown "{{file}}" has been saved with:

1. all research in the body of the document, following the Document Structure
2. and all Frontmatter properties have been set
3. running `md schema validate '{{file}}'` returns `true` (indicating that all Frontmatter was set correctly)

- you do not need to run any tests or lints
- this task had no code modifications in it
