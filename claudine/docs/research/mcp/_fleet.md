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
        - when: "!file_exists(file) || !frontmatter(file, 'last_updated') || date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - message: "The provider **{{state.name}}** needs to update its research on **MCP**"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - stderr: "The provider **{{state.name}}** has research for **MCP** that is current; skipping updates"
              - skip
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - warn: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The **MCP** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **MCP** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the MCP research on **{{state.name}}** failed to complete!"
    warn: "The MCP research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---
# MCP Research on {{state.name}}

## Skills

Use the 'claudine' skill.

## Scope

Research Model Context Protocol (MCP) support for **{{state.desc}}**. This topic feeds
Claudine's MCP catalog, import/export/sync behavior, runtime injection, and provider
security posture, so the goal is to understand how this provider configures, exposes,
constrains, and secures MCP servers.

Boundary against the `hooks` topic: `hooks` owns lifecycle-event semantics (native event
inventories, payloads, return contracts). This topic owns the MCP surfaces — server
configuration, protocol/transport coverage, tool/resource/prompt exposure,
authorization, security, and MCP notifications. If MCP activity fires a provider
lifecycle event, record the MCP side here and leave the event semantics to `hooks`.

**Never cite Claudine's own documentation as evidence.** Claudine's `mcp` module docs
describe *Claudine's* catalog and injection behavior, NOT provider behavior. Verify
every claim against the provider's primary sources — official docs, source code,
release notes, `--help`, and local inspection.

Write the result to `{{file}}`. Include `$schema: ./_schema.yaml` in frontmatter so the
document can be validated, but treat the instructions below as the source of what
high-quality research must contain.

## Document Structure

Do not reduce MCP to "tool server config". The protocol has distinct surfaces — tools,
resources, prompts, roots, sampling, elicitation — and providers expose each
differently. The body must say explicitly which surfaces the provider supports, hides,
forwards, filters, or ignores. Ask "which mechanisms exist and how do they work",
never "does {{state.name}} support X". Write the body with these H2 sections:

- `## Overview`
    - Summarize provider MCP support and the strongest integration path available to
      Claudine: import/export/sync against persistent config, one-run runtime
      injection, manual-only config, partial support, or none. When several paths are
      true, lead with the strongest one (a provider with both persistent sync and
      one-run injection is a runtime-injection provider whose sync story belongs in
      Import, Export, and Sync)
    - Give a one-line inventory of which MCP surfaces are exposed versus ignored; the
      later sections expand each
- `## Protocol and Transports`
    - Protocol generation: cite explicit protocol version dates when docs expose them;
      otherwise describe the observed feature generation
    - Transports: stdio is local subprocess JSON-RPC; Streamable HTTP is the modern
      remote transport; HTTP+SSE/SSE is legacy/compatibility; custom transports may
      exist. Which does the provider accept, and is legacy SSE still tolerated?
    - Session lifecycle: when servers connect, reconnect behavior, and how
      `list_changed` notifications are handled
- `## Configuration`
    - Every persistent file that defines MCP servers or MCP policy, by OS and scope.
      State macOS, Linux, and Windows paths separately — never claim one path covers
      all OSes; Windows path syntax alone makes that ambiguous. Use separate entries
      for user, repo, managed/system, and plugin scopes
    - CLI commands and switches that add/list/remove/import/export servers, enable or
      trust them, or point at alternate config files
    - Environment variables that change MCP config, injection, auth, or server
      visibility — not provider-wide variables unless they change MCP behavior
- `## Server Definition Shape`
    - The accepted shape of ONE server definition (not the whole config file), using
      provider-native keys: command/stdio fields, remote HTTP fields, how per-server
      env vars are represented, how auth headers/tokens/OAuth references are
      represented, required fields, and quirks
- `## Tools, Resources, and Prompts`
    - Tools are model-controlled (`tools/list`, `tools/call`): document discovery,
      per-tool filtering separate from approval policy, approval flow, timeouts,
      result handling before output reaches the model, and whether tool annotations
      are treated as trusted
    - Resources are application-controlled context identified by URI: listing/reading,
      templates, subscriptions, URI schemes, and who selects them — user, model, or
      both. Distinguish real resources from tools that merely return resource links
    - Prompts are user-controlled templates: are they exposed as slash commands,
      palette actions, or not at all? Do not describe them as automatic model tools
      unless the provider actually does that
- `## Roots, Sampling, and Elicitation`
    - These are the client capabilities the provider offers BACK to servers — the side
      provider docs usually omit, but it matters for security and interoperability
    - Roots are client-provided filesystem/workspace boundaries: what does the
      provider answer to `roots/list`, and where does that boundary come from?
    - Sampling lets servers request LLM calls through the client — powerful; does the
      provider support it, and is explicit user approval required?
    - Elicitation lets servers collect structured user input — it must not carry
      sensitive information; is it supported and gated?
- `## Import, Export, and Sync`
    - Be precise about direction: can Claudine read provider config and normalize it
      (import), write provider-shaped config (export), or apply changes through the
      provider CLI/API instead of editing files directly?
    - How the provider combines user/repo/managed/plugin config sources (merge,
      shadow, replace, nearest-wins)
- `## Runtime Injection`
    - Can MCP servers be injected for a single run without permanently mutating user
      or repo config? Name the exact mechanism (flag, env var, inline config) and its
      limitations — this matters most for Claudine wrappers
    - If unsupported, state the closest alternative and why it is not safe for
      one-run use
- `## Authorization and Credentials`
    - Authorization differs by transport: HTTP/Streamable HTTP may use OAuth-style
      flows (token storage, token audience/resource binding, static headers); stdio
      servers should receive secrets via env/config/credential stores, not the HTTP
      auth flow
    - Where credentials are stored, per-user versus per-project, and whether Claudine
      can avoid reading or writing secrets
- `## Security Model`
    - Server allowlists/denylists, and per-tool include/exclude filters separate from
      approval policy
    - Repo trust gates, safe mode, and admin/managed policy restrictions
    - Whether MCP subprocesses inherit the user environment, and where secrets end up
    - Sandbox/container boundaries around MCP servers, and whether tool results are
      scanned or filtered for prompt injection
    - Whether roots constrain filesystem-like servers, and whether sampling and
      elicitation require explicit user consent
- `## Mode-Specific Behavior`
    - Differences between interactive, non-interactive/headless, ACP, IDE, and server
      modes — especially whether MCP approvals ride the same permission model as
      native tools, and which modes drop MCP entirely
- `## Failure Modes`
    - What happens when a configured server fails to start, emits stderr, hangs, or
      returns oversized output; retry/reconnect and timeout behavior
- `## Gaps`
    - Claims that could not be verified and protocol surfaces the provider docs do
      not describe clearly enough
- `## Claudine Integration Notes`
    - What Claudine's catalog, sync, and injection layers should do — and avoid — for
      this provider
- `## Changelog`
    - only when `update` is true
- `## Sources`
    - all useful resources you used in your research, as Markdown links; every
      mechanism claim needs a URL or an observed-on-host reference

## Task

Follow these steps exactly:

::block when="update"
- Read existing research in `{{file}}`

    > **Note:** agentic CLIs change rapidly, so assume the prior research is out of
    > date. You are reading it primarily to report changes into the `## Changelog`
    > section. Never substitute information from the old research for doing your own
    > up-to-date research.

::end-block
- Perform research on the topic using official documentation first, then source code,
  release notes, `--help`, and local inspection

    > **Evidence requirement:** you have read access to `{{state.user_dir}}` on this
    > host. Inspect the *actual* MCP config files there and prefer what you observe
    > over what documentation claims. Negative probes are evidence too ("no MCP key
    > exists in the real config" is a finding). Unanswered is not the same as omitted
    > — record `unknown` with a note rather than dropping a field, and never invent
    > behavior to complete the exercise.

::block when="update"
- Update the document with your research
- Add an entry to the `## Changelog` section
::end-block
::block when="!update"
- Write and save research to `{{file}}`, with the body organized per the Document
  Structure above
::end-block
- Set the `$schema` property of `{{file}}` to the string `./_schema.yaml`

    > This is a file reference to this topic's schema sidecar. Read `_schema.yaml`
    > (it sits next to this sequence file) before filling frontmatter — it is the
    > authoritative field contract, expressed as a `SimpleSchema`, and
    > `md schema validate` will enforce it against everything you write.

- Now capture the facts you documented above into the document's frontmatter:
    ::block when="!update"
    - `created` - set to "{{ctx.today}}"
    ::end-block
    - `last_updated` - set to "{{ctx.today}}"
    - `agent` - set to "{{env.AGENT}}"
    - `model` - set to "{{env.MODEL || 'default' }}"
    - `docs` - the best official MCP docs URL; when no MCP-specific page exists, use
      the best official config/integration docs and explain the gap in `gaps` or the
      body
    - `support` - the integration-path classification you justified in `## Overview`.
      Choose the strongest true value for Claudine: `import_sync` >
      `runtime_injection` > `manual_config` > `partial`. Use `none` only with clear
      evidence MCP is unsupported, and `unknown` only when current evidence is
      insufficient
    - `protocol` - the versions, transports, and lifecycle facts from
      `## Protocol and Transports`
    - `config_files` - one record per OS + scope + path from `## Configuration`; use
      template paths (e.g. `~/.provider/mcp.json`), and never a single record
      claiming to cover all OSes
    - `cli_params` - the MCP-specific commands/switches from `## Configuration` only —
      no unrelated general flags
    - `env_vars` - the MCP-affecting variables from `## Configuration` only
    - `server_schema` - the single-server shape from `## Server Definition Shape`,
      using provider-native key names
    - `server_capabilities` - the tools/resources/prompts coverage and the
      `list_changed`/subscribe flags from `## Tools, Resources, and Prompts`
    - `client_capabilities` - the roots/sampling/elicitation coverage from
      `## Roots, Sampling, and Elicitation`
    - `tool_surface` - discovery, filtering, approval, result handling, and
      annotation-trust facts from `## Tools, Resources, and Prompts`
    - `resource_surface` / `prompt_surface` - the exposure models from
      `## Tools, Resources, and Prompts`
    - `sync_behavior` - the import/export/apply directions and merge strategy from
      `## Import, Export, and Sync`
    - `runtime_injection` - the mechanism and limitations from `## Runtime Injection`
    - `authorization` - the credential-handling facts from
      `## Authorization and Credentials`
    - `security` - the trust/filtering/sandboxing answers from `## Security Model`
    - `gaps` - the unverified claims listed in `## Gaps`
    ::block when="update"
    - `changes` - a list of string descriptions summarizing the changes discovered
      since the last research was done
    ::end-block
    ::block when="!update"
    - `changes` - set to `[]`
    ::end-block
    - `requires_claudine_update` - set to true/false based on whether you believe
      Claudine code or generated metadata must change because of your research —
      not merely because documentation changed
        - if `true`, you must also set the `reason` frontmatter property to describe
          why

## Output

::file @prompts/make-it-markdown.md

## Exit Criteria

You are done with this task when the Markdown "{{file}}" has been saved with:

1. all research in the body of the document, organized per the Document Structure
2. all Frontmatter properties set
3. running `md schema validate '{{file}}'` returns `true` (indicating that all
   Frontmatter was set correctly)

- you do not need to run any tests or lints
- this task had no code modifications in it
