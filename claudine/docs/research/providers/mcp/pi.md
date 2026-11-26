---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3
docs: https://github.com/earendil-works/pi/tree/main/packages/coding-agent#philosophy
support: none
protocol:
  versions: []
  transports: []
  lifecycle: |
    Pi core has no MCP lifecycle of its own. There is no MCP client embedded
    in the pi-coding-agent package, so there is no startup connect, no
    reconnect loop, no capability discovery, and no notifications path. The
    entire protocol stack is opt-in through a third-party extension that
    registers its own MCP client.
  notes: |
    Verified by direct source inspection of the installed package
    @mariozechner/pi-coding-agent@0.73.1 on this host: the only occurrences
    of "mcp" inside the bundled dist tree are inside
    `core/export-html/vendor/highlight.min.js` (C++ keywords like `std::cin`,
    `memcpy` that happen to contain the substring `mcp`) and the literal
    phrase `No MCP.` inside README.md / system-prompt docs. There is no
    MCP protocol code, no JSON-RPC client, and no MCP transport module.
    Pi's own README philosophy section names "No MCP" as a deliberate
    design choice and points to a community-built extension for users who
    want it: "Build CLI tools with READMEs (see Skills), or build an
    extension that adds MCP support."
config_files: []
cli_params: []
env_vars: []
server_schema:
  transports: []
  command_fields: []
  http_fields: []
  env_shape: |
    N/A — Pi core has no MCP server schema. Tool registration in Pi uses
    `pi.registerTool({ name, label, description, parameters, execute })`
    on the ExtensionAPI surface; that surface is Pi-native and not MCP.
  auth_shape: |
    N/A — Pi core has no MCP auth shape. LLM provider credentials use
    per-provider env vars (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, …) or
    `/login` OAuth for subscription providers; those credentials are
    unrelated to MCP server auth.
  notes: |
    Pi does not define an MCP server definition shape because it does not
    implement the protocol. The closest analog is a Pi package's `package.json`
    `pi` manifest, which declares resources by directory:
    `{ "pi": { "extensions": [...], "skills": [...], "prompts": [...], "themes": [...] } }`.
server_capabilities:
  tools: none
  resources: none
  prompts: none
  tool_list_changed: false
  resource_subscribe: false
  resource_list_changed: false
  prompt_list_changed: false
  notes: |
    Pi core exposes no MCP capabilities. It exposes built-in tools
    (`read`, `write`, `edit`, `bash`, `grep`, `find`, `ls`) and any
    extension-registered tools through `pi.registerTool()`. Resources are
    not an MCP concept in Pi; the closest analog is the `resources_discover`
    Pi-native extension event, which returns `skillPaths`, `promptPaths`,
    and `themePaths` for extension-supplied resource loading. Prompts are
    not an MCP concept in Pi either; Pi has its own Markdown prompt-template
    system (`/templatename`) and an Agent Skills standard
    (`/skill:name`). See `dynamic-resources` example extension for the
    Pi-native equivalent surface.
client_capabilities:
  roots: none
  sampling: none
  elicitation: none
  notes: |
    Pi core is not an MCP client and therefore does not expose
    `roots/list`, `sampling/createMessage`, or elicitation dialogs to
    MCP servers. Any client capability the user gets comes from the
    third-party extension they choose to install.
tool_surface:
  discovery: |
    N/A — Pi does not call `tools/list`. Built-in tools are hard-coded in
    the agent loop, and extension tools are registered at extension load
    time via `pi.registerTool()`.
  filtering: |
    `--tools`/`-t` (allowlist), `--exclude-tools`/`-xt`, `--no-builtin-tools`/`-nbt`,
    and `--no-tools`/`-nt` filter the visible tool set across built-in,
    extension, and custom tools. These flags are Pi-native, not MCP-specific.
    Resource-path settings in `settings.json` (`packages`, `extensions`,
    `skills`, `prompts`, `themes`) also gate which extensions load.
  approval: |
    Pi has no built-in permission popups. Tool calls fire unconditionally
    with the user's process privileges. The `permission-gate.ts` example
    extension demonstrates a confirmation flow on `tool_call` events for
    dangerous bash patterns (`rm -rf`, `sudo`); users build their own
    approval flow with extensions.
  result_handling: |
    Pi tool results are objects shaped `{ content: [{ type, text | ... }],
    details: { ... } }`. The `content` array is the LLM-facing payload and
    `details` is opaque extension state persisted into the session JSONL
    for proper forking support. Built-in tools truncate large output
    (50 KiB / 2000 lines for bash, similar guard for read).
  annotations_trusted: |
    N/A — Pi does not consume MCP tool annotations because it does not
    call MCP. Extension-defined tool schemas go through TypeBox and are
    passed to the provider API verbatim with whatever
    `_meta`/description the extension author chose.
  notes: |
    All MCP tool-surface concepts (discovery, filtering, approval, result
    handling, annotations) are inapplicable at the Pi-core layer. They
    become relevant only inside a third-party MCP adapter extension, where
    each extension defines its own tool-naming convention, output guard,
    and consent model.
resource_surface:
  supported: false
  uri_schemes: []
  templates: false
  subscriptions: false
  exposure_model: |
    N/A — MCP resources are not exposed by Pi core. The Pi-native analog
    is `@`-prefixed file references in the prompt (fuzzy-search project
    files) and skill/prompt/theme resources registered through Pi's
    `resources_discover` event or listed in package manifests.
  notes: |
    Pi does not implement MCP `resources/list`, `resources/read`,
    `resources/templates/list`, or `resources/subscribe`. There is no
    URI scheme abstraction at the Pi-core level for MCP resources.
prompt_surface:
  supported: false
  invocation: |
    N/A — MCP prompts are not exposed. Pi's own prompt-template feature
    is invoked with `/templatename` from the slash-command palette and
    expands a Markdown file from `~/.pi/agent/prompts/`, `.pi/prompts/`,
    or a package's `prompts/` directory.
  arguments: |
    Pi prompt templates use `{{var}}` interpolation against frontmatter
    variables (`{{focus}}` style), not MCP-style JSON-Schema arguments.
  exposure_model: |
    User-controlled via slash commands. Templates are not auto-invoked by
    the model; the user types `/name` to expand them into the editor.
  notes: |
    Pi's prompt-template system is unrelated to MCP prompts. Pi also
    supports Agent Skills (`/skill:name`) — a separate user-invoked
    on-demand capability system.
sync_behavior:
  import_supported: false
  export_supported: false
  apply_supported: false
  merge_strategy: none
  notes: |
    Claudine cannot import, export, or sync MCP servers with Pi because
    Pi core has no MCP config. A user-installed third-party adapter
    extension may have its own config file (for example
    `pi-mcp-adapter` reads `~/.config/mcp/mcp.json`, `.mcp.json`,
    `<pi agent dir>/mcp.json`, and `.pi/mcp.json`), but those configs
    belong to the extension, not to Pi. Claudine has no reliable signal
    that such an extension is installed without inspecting the user's
    `pi list` output, and the config schema varies by extension.
runtime_injection:
  supported: false
  mechanism: |
    N/A — there is no Pi-native one-run MCP injection flag. The closest
    Pi-native parallel is `--extension` (`-e`) which loads a single
    TypeScript extension file for the current run; that path could
    host a third-party MCP adapter extension but the extension's own
    config still controls which servers load.
  limitations: |
    Pi's `-e` flag is a generic extension loader, not an MCP injection
    surface. Even if Claudine shipped a self-contained MCP adapter
    extension and loaded it via `-e`, the servers it connects to would
    be controlled by that extension's config, not by Claudine. There is
    no `--mcp-config` equivalent on Pi, no shadow-config trick, and no
    wrapper-only path to authoritatively set the MCP server set for one
    run.
authorization:
  oauth: false
  credential_storage: |
    N/A for MCP. Pi stores LLM provider credentials in
    `~/.pi/agent/auth.json` (OAuth tokens and API keys for LLM providers).
    MCP auth is the third-party adapter extension's responsibility.
  token_scope: |
    N/A for MCP. OAuth for LLM providers uses each provider's flow
    surfaced through `/login` and `/logout` slash commands.
  stdio_secret_delivery: |
    N/A for MCP. If a user installs a third-party adapter, secrets are
    delivered by that extension. Common patterns seen in the third-party
    ecosystem: per-server `env` map, `${VAR}` and `$env:VAR`
    interpolation, and `bearerTokenEnv` to point at an env var holding
    a bearer token.
  notes: |
    Pi itself never speaks MCP. Authorization for MCP servers belongs to
    the installed adapter extension and varies by extension.
security:
  tool_filtering: |
    Pi's `--tools`/`--exclude-tools`/`--no-builtin-tools`/`--no-tools`
    flags filter the visible tool surface. There is no per-server or
    per-MCP-tool filter at the Pi-core level. A third-party MCP adapter
    extension typically adds its own filter (for example
    `pi-mcp-adapter`'s `excludeTools` per-server or global `disableProxyTool`).
  server_trust: |
    Pi has no MCP server trust model. Project trust (`/trust`,
    `defaultProjectTrust`, `--approve`/`--no-approve`) gates project-local
    `.pi/settings.json`, `.pi` resources, project skills, and project
    extensions, not MCP servers. Installed npm/git Pi packages run with
    full system access; the Pi README explicitly warns that
    "Pi packages run with full system access. Extensions execute
    arbitrary code, and skills can instruct the model to perform any
    action including running executables. Review source code before
    installing third-party packages."
  env_sanitization: |
    Pi has no MCP-specific env sanitization. Extensions run with the
    user's process environment. A per-extension process boundary is
    possible only via an OS-level sandbox such as Gondolin, plain
    Docker, or OpenShell (Pi documents three containerization patterns
    in `docs/containerization.md`).
  sandbox_interaction: |
    Pi does not sandbox core execution. Three documented patterns:
    Gondolin micro-VM extension that routes built-in tools and `!`
    commands into a local Linux micro-VM while keeping Pi on the host;
    plain Docker wrapping the whole `pi` process; and OpenShell
    policy-controlled sandbox. None of these are MCP-aware — they
    isolate the agent, not the MCP transport.
  response_filtering: |
    Pi has no MCP response filtering. The built-in `bash` tool truncates
    its own output at 50 KiB / 2000 lines; the `truncated-tool.ts`
    example extension wraps ripgrep with the same guard. A third-party
    MCP adapter extension implements its own output guard (for example
    `pi-mcp-adapter` defaults to a 50 KiB / 2000-line inline text cap
    with a temp-file spillover, and exposes `outputGuard` settings and
    an `MCP_OUTPUT_GUARD=0` env kill switch).
  notes: |
    Pi's overall security posture is "runs with the permissions of the
    launching user and process, opt into a container if you need
    isolation." Anything an MCP adapter extension does inherits that
    posture unless the extension itself adds its own sandboxing.
gaps:
  - |
    Pi core's MCP posture has been stable since launch: there is no
    first-party MCP implementation and no announced roadmap to add one.
    The README's "Philosophy" section calls "No MCP" out by name.
  - |
    The third-party MCP ecosystem for Pi is real and growing
    (`pi-mcp-adapter` 100k+ monthly npm downloads, 957 GitHub stars, 11
    dependents as of 2026-07-03) but has not converged on a single
    canonical config schema. Other competing extensions
    (`pi-mcp-extension`, `@spences10/pi-mcp`, `pi-mcp-connector`,
    `pi-mcp-audience`, `pi-tidy-mcp-adapter`, `@feniix/bridgekit`)
    define their own config shapes. Claudine should treat Pi as
    `support: none` for the provider itself; per-extension support
    would be a separate research scope.
  - |
    Sampling support varies by extension and is text-only where present.
    `pi-mcp-adapter` supports it via `sampling` and `samplingAutoApprove`
    settings and rejects non-text content (no tools, no audio, no image,
    no stop sequences, no context inclusion) with explicit errors. Pi
    core itself never samples.
  - |
    Elicitation support varies by extension. `pi-mcp-adapter` advertises
    form mode whenever Pi exposes dialog-capable UI and URL mode only in
    TUI mode; URL-required tool errors (`-32042`) are handled with a
    retry-after-browser-completion flow. Pi core has no elicitation.
  - |
    OAuth flows in third-party extensions include both dynamic client
    registration and pre-registered client ID/secret patterns. For
    remote/headless Pi sessions, the extension typically surfaces an
    `auth-start`/`auth-complete` proxy action that prints the
    authorization URL for the user to open in a local browser. There is
    no Claudine-managed apply path for any of these flows.
  - |
    Verified locally on this host: `pi --version` reports
    `0.73.1` (the installed package is `pi-coding-agent@0.73.1` under
    `@mariozechner/pi-coding-agent`, which is the npm scope during
    migration to `@earendil-works/pi-coding-agent`); `~/.pi/agent/`
    contains `settings.json` (with no MCP keys), `models.json`, and
    `auth.json`; `pi list` reports "No packages installed"; no MCP
    config files exist anywhere under `~/.config/mcp/`, `.mcp.json`,
    `.pi/mcp.json`, or `~/.pi/agent/mcp.json`. This host is a clean
    Pi installation with zero MCP involvement.
changes:
  - "Confirmed via direct source inspection on this host (pi-coding-agent@0.73.1): no MCP protocol code anywhere in the bundled dist tree outside README/CHANGELOG prose; the only literal occurrences of 'mcp' in dist files are C++ keywords in `core/export-html/vendor/highlight.min.js`."
  - "Confirmed via npm registry search (2026-07-03): the third-party MCP ecosystem for Pi is substantial and growing. `pi-mcp-adapter` v2.11.0 leads at ~100k monthly downloads, 957 GitHub stars, and 11 dependents. Competing extensions exist (`pi-mcp-extension`, `pi-mcp-connector`, `@spences10/pi-mcp`, `pi-mcp-audience`, `pi-tidy-mcp-adapter`, `@feniix/bridgekit`) with no convergence on a single canonical schema. Pi core's `support: none` classification is unchanged because none of these are first-party."
  - "Captured the dominant third-party config layout (`pi-mcp-adapter` v2.11.0) for downstream Claudine users who install an adapter: precedence is `~/.config/mcp/mcp.json` > `<pi agent dir>/mcp.json` > `.mcp.json` > `.pi/mcp.json`, with a `<pi agent dir>/mcp-cache.json` metadata cache and OAuth flows through `/mcp-auth` plus a proxy `auth-start`/`auth-complete` action for headless. The adapter exposes a single `mcp` proxy tool (~200 tokens) plus optional `directTools` registration of 5–20 specific tools per server."
  - "Added local verification of a clean Pi install: pi 0.73.1, no MCP config files, no MCP extensions, no MCP keys in `~/.pi/agent/settings.json`."
  - "Captured the package-scope migration note: this host has `@mariozechner/pi-coding-agent` (the legacy npm scope during migration to `@earendil-works/pi-coding-agent`). The 0.73.1 changelog documents `pi update --self` honoring the active package name returned by the Pi version check endpoint."
requires_claudine_update: false
reason: |
  Pi core's MCP posture is unchanged from the prior research and the
  provider is correctly classified as `support: none`. The only new
  finding is the size and fragmentation of the third-party MCP
  extension ecosystem, which is out of scope for a Pi-core research
  document and does not change the catalog's classification. No Claudine
  code or generated metadata changes are warranted.

---

# MCP Support in Pi

## Overview

[Pi](https://pi.dev/) is a minimal, extensible terminal coding agent harness built around TypeScript extensions, Agent Skills, prompt templates, and themes. Its core philosophy explicitly excludes the Model Context Protocol. The [homepage](https://pi.dev/) lists "No MCP" as a deliberate design choice under "What we didn't build," and the [coding-agent README's Philosophy section](https://github.com/earendil-works/pi/tree/main/packages/coding-agent#philosophy) repeats the same line: "**No MCP.** Build CLI tools with READMEs (see Skills), or build an extension that adds MCP support."

For Claudine, this means Pi is a `support: none` provider. Claudine should not attempt to import, export, sync, or runtime-inject MCP servers into Pi, and `claudine mcp` commands should not present Pi as a sync target. The provider's strongest MCP story is a user-installed third-party extension (see "Third-party MCP extensions" below), but that extension defines its own config schema and transport details — there is no first-party surface for Claudine to integrate with.

Surface inventory (one-line):

- **Tools** — not exposed: Pi does not call MCP `tools/list` or `tools/call`. Its tool surface is built-in (`read`, `write`, `edit`, `bash`, `grep`, `find`, `ls`) plus extension-registered tools.
- **Resources** — not exposed: MCP `resources/list`/`resources/read`/`resources/subscribe` are not implemented. The Pi-native analog is `@`-file references and the `resources_discover` extension event.
- **Prompts** — not exposed: MCP prompt templates are not implemented. Pi has its own Markdown prompt-template feature invoked with `/templatename`.
- **Roots** — not exposed: `roots/list` is not implemented by Pi core. Pi-built-in tools share the working directory the agent was launched from; nothing is exposed back to MCP servers.
- **Sampling** — not exposed: Pi core does not handle `sampling/createMessage`. Servers cannot ask Pi to run an LLM call.
- **Elicitation** — not exposed: Pi core has no MCP elicitation dialogs.

## Protocol and Transports

Pi does not implement the Model Context Protocol in any form. It does not spawn stdio MCP servers, connect to HTTP/SSE MCP endpoints, or speak Streamable HTTP. The only protocol stack Pi's TUI/SDK/RPC modes speak is the agent-loop RPC protocol (`pi --mode rpc`, documented at [`docs/rpc.md`](https://github.com/earendil-works/pi/tree/main/packages/coding-agent/docs/rpc.md)) and the LLM provider transports (SSE, WebSocket, `auto`) configured through the `transport` setting and the per-provider implementation in `@earendil-works/pi-ai`.

Direct evidence on this host:

- `pi --version` → `0.73.1`
- `pi --help` → no `mcp` subcommand, no `--mcp-config`, no `--mcp-servers` flag
- The installed `@mariozechner/pi-coding-agent@0.73.1` bundle was searched for `mcp`/`MCP`/`ModelContext`/`modelcontext` across `dist/**/*.js` and `dist/**/*.d.ts`. The only matches are:
  - C++ keywords in `core/export-html/vendor/highlight.min.js` (substring matches like `memcpy`, `std::cout`),
  - the literal "No MCP." string in the README and CHANGELOG,
  - the literal "MCP server integration" string inside the README's "Extensions" bullet list (a claim about what extensions *can* do, not a Pi-core feature).
- `pi-coding-agent`'s `system-prompt.ts` enumerates its own docs and examples but does not mention MCP.

Pi's philosophy list ([homepage](https://pi.dev/), [README](https://github.com/earendil-works/pi/tree/main/packages/coding-agent#philosophy)) groups "No MCP" alongside "No sub-agents," "No permission popups," "No plan mode," "No built-in to-dos," and "No background bash," all of which can be added with extensions.

## Configuration

Pi has no MCP-specific configuration files. Relevant persistent files include:

| File | Scope | Purpose |
| :--- | :---- | :------ |
| `~/.pi/agent/settings.json` | User | Global settings (model, theme, compaction, retry, resources, package list) |
| `.pi/settings.json` | Project | Project overrides; merges over global with nested-object merge |
| `~/.pi/agent/models.json` | User | JSONC-allowed custom provider and model definitions |
| `~/.pi/agent/auth.json` | User | Stored LLM-provider API keys and OAuth tokens (LLM providers, not MCP) |
| `~/.pi/agent/trust.json` | User | Saved project trust decisions (`/trust`) |
| `~/.pi/agent/extensions/` | User | User-installed extension files |
| `~/.pi/agent/skills/`, `~/.pi/agent/prompts/`, `~/.pi/agent/themes/` | User | User resources |
| `.pi/extensions/`, `.pi/skills/`, `.pi/prompts/`, `.pi/themes/` | Project | Project-local resources |
| `~/.pi/agent/git/`, `~/.pi/agent/npm/` | User | Installed package sources |
| `.pi/git/`, `.pi/npm/` | Project | Project-local installed package sources |
| `~/.pi/agent/sessions/` | User | Session JSONL files (tree-structured) |

None of these define MCP servers. The `settings.json` `resources` keys (`packages`, `extensions`, `skills`, `prompts`, `themes`) describe Pi-native resource directories, not MCP server definitions. A user who installs a third-party MCP adapter extension gets extension-specific config files; see "Third-party MCP extensions" below.

### CLI commands

The Pi CLI (`pi --help`) exposes package management, model selection, session management, resource loading, and prompt modes. The MCP-relevant subset is the generic resource loader:

| Flag | Effect |
| :--- | :------ |
| `-e`, `--extension <source>` | Load a single extension from path, npm, or git (repeatable) |
| `--no-extensions` | Disable extension discovery |
| `--skill <path>` | Load a single skill (repeatable) |
| `--no-skills` | Disable skill discovery |
| `--prompt-template <path>` | Load a single prompt template (repeatable) |
| `--no-prompt-templates` | Disable prompt-template discovery |
| `--theme <path>` | Load a single theme (repeatable) |
| `--no-themes` | Disable theme discovery |
| `-a`, `--approve` | Trust project-local files for this run (one-shot project-trust override) |
| `-na`, `--no-approve` | Ignore project-local files for this run |

There is no `pi mcp` subcommand, no `pi mcp add`/`list`/`get`/`remove`, and no `--mcp-config`. The `pi install`/`pi remove`/`pi uninstall`/`pi list`/`pi update`/`pi config` commands are general package-management commands (Pi packages bundle extensions/skills/prompts/themes), not MCP commands.

### Environment variables

Pi-level variables that affect agent behavior:

| Variable | Effect |
| :------- | :------ |
| `PI_CODING_AGENT_DIR` | Override the config directory (default `~/.pi/agent`) |
| `PI_CODING_AGENT_SESSION_DIR` | Override session storage directory (overridden by `--session-dir`) |
| `PI_PACKAGE_DIR` | Override package directory (useful for Nix/Guix where store paths tokenize poorly) |
| `PI_OFFLINE` / `--offline` | Disable startup network operations (update checks, package update checks, install/update telemetry) |
| `PI_SKIP_VERSION_CHECK` | Skip the Pi version update check at startup (no `pi.dev` request) |
| `PI_TELEMETRY` | Override install/update telemetry and provider attribution headers (`1`/`true`/`yes` to enable, `0`/`false`/`no` to disable). Does not disable update checks |
| `PI_CACHE_RETENTION` | `long` extends prompt cache (Anthropic 1h, OpenAI 24h) |
| `VISUAL` / `EDITOR` | External editor fallback for Ctrl+G (Notepad on Windows, nano elsewhere) |

None of these affect MCP behavior because Pi has no MCP layer.

## Server Definition Shape

Not applicable. Pi extensions register tools programmatically via `pi.registerTool({ name, label, description, parameters, execute })` on the `ExtensionAPI` surface. There is no MCP-equivalent `mcpServers` object, no `command`/`args`/`env` server map, and no `type: "stdio" | "http" | "sse"` discriminator. The closest analog is the `pi` key in a Pi package's `package.json`:

```json
{
  "name": "my-pi-package",
  "keywords": ["pi-package"],
  "pi": {
    "extensions": ["./extensions"],
    "skills": ["./skills"],
    "prompts": ["./prompts"],
    "themes": ["./themes"]
  }
}
```

Without a `pi` manifest, Pi auto-discovers from conventional directories (`extensions/`, `skills/`, `prompts/`, `themes/`).

## Tools, Resources, and Prompts

### Tools

Pi exposes built-in tools (`read`, `write`, `edit`, `bash`, `grep`, `find`, `ls`) and lets extensions register additional tools through `pi.registerTool()`. These tools are not MCP tools and do not go through `tools/list` or `tools/call`. The `parameters` field uses TypeBox schemas; tool results are objects shaped `{ content: [{ type, text | ... }], details: { ... } }`, persisted in the session JSONL `details` slot for proper forking support.

Built-in tools can be enabled or disabled through `--tools`/`--exclude-tools`/`--no-builtin-tools`/`--no-tools`. Extension tools are gated by extension discovery (`--no-extensions`, `--extension <source>`).

### Resources

MCP resources are not supported by Pi core. The Pi-native analog is two surfaces:

- **`@`-file references** — the user fuzzy-searches project files and attaches them to the conversation.
- **Extension-supplied resources** — an extension can register paths to skills, prompts, and themes through the `resources_discover` event (`skillPaths`, `promptPaths`, `themePaths`). The `dynamic-resources` example extension demonstrates this pattern.

Neither surface implements MCP `resources/list`, `resources/read`, `resources/templates/list`, or `resources/subscribe`.

### Prompts

MCP prompts are not supported by Pi core. Pi has its own prompt-template feature invoked with `/templatename` from the slash-command palette. Templates are Markdown files in `~/.pi/agent/prompts/`, `.pi/prompts/`, or a package's `prompts/` directory, with `{{var}}` interpolation against frontmatter variables. Pi also supports the Agent Skills standard, invoked with `/skill:name`.

## Roots, Sampling, and Elicitation

Pi core is not an MCP client and therefore does not expose:

- **`roots/list`** — Pi has no MCP root-boundary concept. Built-in and extension tools share the working directory the agent was launched from.
- **`sampling/createMessage`** — Pi does not handle server-requested LLM completions. Servers cannot ask Pi to invoke a model through the protocol.
- **`completion/complete`** — not handled.
- **Elicitation** — Pi has no MCP elicitation dialog. The `ctx.ui.select()`, `ctx.ui.input()`, and `ctx.ui.confirm()` extension APIs are Pi-native dialogs unrelated to MCP.

Any of these capabilities the user sees come from a third-party adapter extension they install.

## Third-party MCP extensions

A growing third-party ecosystem provides MCP support for Pi through extensions. The packages below are not maintained by Pi's authors; they are independent community projects that embed their own MCP client behind Pi's extension API. They are listed here as a survey for downstream Claudine users — Claudine should not attempt to integrate any of them at the provider level, because the config schemas and capabilities differ and there is no canonical winner.

| Package | npm | Notes |
| :------ | :-- | :---- |
| [`pi-mcp-adapter`](https://github.com/nicobailon/pi-mcp-adapter) | `pi-mcp-adapter` | Most-downloaded (~100k monthly, 957 stars, 11 dependents as of 2026-07-03). Reads shared `~/.config/mcp/mcp.json` and `.mcp.json`, plus Pi-owned overrides `<pi agent dir>/mcp.json` and `.pi/mcp.json`. Single proxy tool `mcp` (~200 tokens) plus optional `directTools` registration. Supports stdio and Streamable HTTP (with SSE fallback), bearer and OAuth (dynamic client registration or pre-registered), form-mode and URL-mode elicitation (URL mode only in TUI), text-only sampling with consent, output guard (50 KiB / 2000 lines), `/mcp` panel, `/mcp setup`, `/mcp reconnect`, `/mcp logout`, `/mcp-auth`, MCP-UI integration (Glimpse on macOS, browser fallback), and host-specific config imports for `cursor`, `claude-code`, `claude-desktop`, `vscode`, `windsurf`, `codex`. |
| `pi-mcp-extension` | `pi-mcp-extension` | Independent MCP client extension |
| `@spences10/pi-mcp` | `@spences10/pi-mcp` | MCP server integration package; exposes configured MCP tools safely and manages large responses |
| `pi-mcp-connector` | `pi-mcp-connector` | "MCP Gateway for Pi" — stdio/SSE/Streamable HTTP transports, session recovery, connection pooling, structured error handling |
| `pi-mcp-audience` | `pi-mcp-audience` | Audience-annotation filter; hides MCP content not intended for the user |
| `pi-tidy-mcp-adapter` | `pi-tidy-mcp-adapter` | Fork of `pi-mcp-adapter` |
| `@feniix/bridgekit` | `@feniix/bridgekit` | Bridges TypeBox-backed tools between Pi and MCP |

Even the most-downloaded extension does not share its config schema with the others, and Pi core provides no common substrate. Claudine should treat this ecosystem as out of scope for provider-level MCP integration; a Claudine user who installs one of these extensions is responsible for managing its config directly.

## Import, Export, and Sync

Claudine cannot import, export, or sync MCP servers with Pi because Pi core has no MCP config. Any third-party adapter extension defines its own server config and persistence. There is no `pi mcp add`/`list`/`remove` CLI surface, no Pi-owned MCP JSON file, and no schema published by Pi for MCP server definitions.

If a future user wants Claudine to sync MCP servers against a third-party Pi adapter extension, that adapter must be researched independently — its config schema, transport surface, and security model vary by package, and Claudine's catalog cannot assume any of them.

## Runtime Injection

There is no Pi-native runtime injection mechanism for MCP servers. Pi exposes `--extension` (`-e`) for loading a single TypeScript extension for the current run, but that flag is a generic extension loader, not an MCP injection surface. Even if Claudine shipped a self-contained MCP adapter extension and loaded it via `-e`, the servers it connects to would be controlled by that extension's config, not by Claudine. There is no `--mcp-config` equivalent on Pi, no shadow-config trick, and no wrapper-only path to authoritatively set the MCP server set for one run.

## Authorization and Credentials

Pi stores LLM-provider credentials in `~/.pi/agent/auth.json` (API keys and OAuth tokens for LLM providers like Anthropic, OpenAI, Google, Bedrock, etc.) and surfaces per-provider env-var paths (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GOOGLE_API_KEY`, etc.). Subscription OAuth for supported LLM providers runs through `/login` and `/logout` slash commands.

None of this is MCP-related. MCP server auth is the third-party adapter extension's responsibility; the dominant pattern in the third-party ecosystem is bearer tokens (`bearerToken` or `bearerTokenEnv`) or OAuth 2.0 (`oauth.grantType`, `oauth.clientId`, `oauth.clientSecret`, `oauth.scope`, `oauth.redirectUri`, `oauth.clientName`, `oauth.clientUri`), but the exact field names vary by extension.

## Security Model

Pi's security posture is intentionally minimal:

- **No built-in permission system.** Pi runs with the privileges of the launching user and process. There are no built-in permission popups; the `permission-gate.ts` example extension shows how to add a confirmation flow on `tool_call` events.
- **No MCP server trust gate.** Project trust (`/trust`, `defaultProjectTrust`, `--approve`/`--no-approve`) applies to project-local settings, resources, and project extensions, not MCP servers.
- **Extensions run arbitrary code.** Any Pi extension — including a third-party MCP adapter — executes TypeScript with the user's full process privileges and inherits the user's environment.
- **Optional sandboxing.** Three documented patterns in [`docs/containerization.md`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/containerization.md): Gondolin micro-VM extension, plain Docker, and OpenShell. None of these are MCP-aware — they isolate the agent process, not the MCP transport.
- **No response sanitization.** Pi core does not scan MCP tool results for prompt injection; the built-in `bash` tool truncates its own output at 50 KiB / 2000 lines, but that is a built-in tool behavior, not an MCP-specific guard. Third-party adapters implement their own output guard.

Pi's [README Security section](https://github.com/earendil-works/pi#permissions--containerization) puts it bluntly: "Pi does not include a built-in permission system for restricting filesystem, process, network, or credential access. By default, it runs with the permissions of the user and process that launched it."

## Mode-Specific Behavior

MCP is not available in any Pi mode because Pi core has no MCP layer:

- **Interactive mode** — full TUI; no `/mcp` command, no MCP panel. Slash commands include `/login`, `/logout`, `/model`, `/resume`, `/new`, `/tree`, `/trust`, `/fork`, `/clone`, `/compact`, `/copy`, `/export`, `/import`, `/share`, `/reload`, `/hotkeys`, `/changelog`, `/quit`. A user who installs a third-party MCP adapter typically gets `/mcp` and `/mcp setup` slash commands from that extension.
- **Print mode (`pi -p`)** — print response and exit. No MCP-specific flags.
- **JSON mode (`pi --mode json`)** — newline-delimited JSON event stream on stdout (see [`docs/json.md`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/json.md)). No MCP events.
- **RPC mode (`pi --mode rpc`)** — JSON protocol over stdin/stdout using strict LF-delimited JSONL framing (see [`docs/rpc.md`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/rpc.md)). No MCP JSON-RPC surface; this RPC protocol is Pi's own agent-loop protocol, not MCP.
- **SDK (`createAgentSession()` from `@earendil-works/pi-coding-agent`)** — embedding API. No MCP client APIs in the SDK; tools are registered with `pi.registerTool()`.

## Failure Modes

MCP is not applicable at the Pi-core layer. Pi's built-in tool failures and extension errors are surfaced in the TUI or JSONL event stream and handled by the agent loop's retry logic (`retry.enabled`, `retry.maxRetries`, `retry.baseDelayMs`, `retry.provider.timeoutMs`, `retry.provider.maxRetries`, `retry.provider.maxRetryDelayMs`). When a provider requests a retry delay longer than `retry.provider.maxRetryDelayMs`, the request fails immediately with an informative error instead of waiting silently.

For third-party adapter extensions, failure handling is whatever that extension implements.

## Claudine Integration Notes

- Classify Pi as `support: none` in the MCP catalog. Do not present Pi as a sync target for `claudine mcp export pi --apply` or similar commands.
- Do not inject MCP servers into Pi wrapper invocations. There is no `--mcp-config`-equivalent flag on Pi, and `--extension` is a generic loader, not an MCP surface.
- If a Claudine user explicitly asks for MCP support through Pi, the correct recommendation is to install a third-party adapter extension (currently `pi-mcp-adapter` is the most-downloaded) and manage its config outside Claudine. Do not attempt to wrap that extension's config in Claudine's catalog at the provider level.
- Re-check this classification only if Pi core publishes first-party MCP support — the existing `Philosophy` section explicitly opts out, so this would be a deliberate change.
- Pi's lack of an MCP layer is consistent across interactive, print, JSON, RPC, and SDK modes; the `claudine` Pi wrapper needs no MCP-aware mode handling.

## Sources

- [Pi homepage](https://pi.dev/)
- [Pi coding-agent README](https://github.com/earendil-works/pi/tree/main/packages/coding-agent)
- [Pi Philosophy section](https://github.com/earendil-works/pi/tree/main/packages/coding-agent#philosophy)
- [Pi settings documentation](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/settings.md)
- [Pi extensions documentation](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/extensions.md)
- [Pi containerization documentation](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/containerization.md)
- [Pi extensions README](https://github.com/earendil-works/pi/tree/main/packages/coding-agent/examples/extensions)
- [Pi CHANGELOG](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/CHANGELOG.md)
- ["What we didn't build" on pi.dev](https://pi.dev/) (lists "No MCP" alongside other intentional omissions)
- ["What if you don't need MCP at all?" — Mario Zechner](https://mariozechner.at/posts/2025-11-02-what-if-you-dont-need-mcp/)
- [npm registry search for "pi-mcp" packages](https://registry.npmjs.org/-/v1/search?text=pi-mcp) (community survey)
- [`pi-mcp-adapter` repository](https://github.com/nicobailon/pi-mcp-adapter) (dominant third-party MCP extension; surveyed for ecosystem context, not first-party)
- Local observation: `pi --version` → `0.73.1` (installed as `@mariozechner/pi-coding-agent`); `pi --help` shows no `mcp` subcommand; `pi list` reports "No packages installed"; `~/.pi/agent/settings.json` contains no MCP-related keys; no MCP config files exist anywhere under `~/.config/mcp/`, `.mcp.json`, `.pi/mcp.json`, or `~/.pi/agent/mcp.json`. The bundled `dist/` tree was searched for `mcp`/`MCP`/`ModelContext` — the only matches are C++ keywords in `core/export-html/vendor/highlight.min.js` and the literal "No MCP." string in the README.