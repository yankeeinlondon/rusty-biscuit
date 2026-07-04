---
sequence:
- name: draft
- name: iterate
- name: finalize
prompt: "The MCP protocol is an important standard that all Agentic CLI's support to one degree or another and because it is a real standard the MCP servers themselves can provide their services in a largely Agent neutral manner. However \"the last mile\" problem always provides small variance in how MCP is configured, packaged, or enabled in each agent.\n\n## Task\n\nYour task is to report on the support for MCP in Claudine, focusing on the variants imposed by the Agentic CLI providers Claudine supports.\n\n- your report should start by outlining the key benefits that MCP provides to agentic processes\n- and then shift it's focus to how Claudine's supported providers support (or don't support) various aspects of MCP. \n\nAs background material we have MCP research documents for each provider that Claudine supports. They can be found at `@claudine/docs/research/mcp/*.md`. \n\nImportant: your final response is saved verbatim as the body of this summary document, so it must be the complete document text and nothing else — no preamble, no commentary. Never write to this document yourself.\n\n::block when=\"state.name == 'draft'\"\n- Iterate over the first three reasearch documents to develop a point of view on how to write this document and then produce an initial draft of the document\n::end-block\n::block when=\"state.name == 'iterate'\"\n\n- Note: the initial draft has already been created — it is the body of `@claudine/docs/research/summary/mcp.md` (everything below the frontmatter); read it from there\n- Act as an orchestrator and iterate over each remaining provider's research document:\n    - provide the subagent the current draft and ask them to return an improved draft based on the research document they've been assigned\n- Once every remaining provider has been incorporated, your final response is the fully updated draft\n::end-block\n\n::block when=\"state.name == 'finalize'\"\n\nThe document has now gone through several rounds of improvement and your task is just to make sure the document is consistent in tone and detail and that nothing looks incorrect or incomplete. The current draft is the body of `@claudine/docs/research/summary/mcp.md` (everything below the frontmatter); read it from there, make any adjustments, and your final response will be considered the finalized summary document.\n::end-block"
hash: a28a35a4aea35c5e-b00a255b69beb457
last_updated: 2026-07-03
---
# MCP Support in Claudine

MCP gives agentic processes a shared extension protocol instead of a new integration layer for every agent. A server can expose tools, resources, prompts, and selected client capabilities once, then multiple agentic CLIs can consume that service with mostly the same semantics.

The practical benefits are:

- **Agent-neutral services.** Teams can package filesystem, database, browser, ticketing, design-system, or internal-docs integrations once and reuse them across agents.
- **Lower switching cost.** MCP moves integrations out of provider-specific plugin APIs and into a protocol that can survive a change in agent runtime.
- **Composable context.** Tools, resources, and prompts let an agent discover capabilities, read external context, and invoke reusable workflows without baking every integration into the model wrapper.
- **Cleaner privilege boundaries.** MCP servers can be enabled per task, per repo, or per user instead of being globally available to every session.
- **Operational consistency.** Stdio, Streamable HTTP, SSE, OAuth, bearer-token, and env-var patterns recur across providers, which gives Claudine a common catalog shape even when each provider stores and activates MCP differently.
- **Better automation posture.** A wrapper can choose an effective MCP set for a single run instead of relying on whatever the user happened to leave enabled in persistent provider config.

The standard does not remove the “last mile” problem. Each provider chooses its own config file, merge rules, trust gates, auth storage, tool naming, runtime injection mechanism, and subset of MCP features. Claudine’s job is not to pretend every provider is identical. It is to normalize the catalog, preserve provider-specific fields where needed, and make activation behavior explicit.

## Claudine’s MCP Model

Claudine treats MCP as an opt-in runtime surface. The catalog records what MCP servers are available; wrapper execution decides what is enabled for a particular run.

The main Claudine concepts are:

- **Catalog:** normalized MCP server inventory in `~/.claudine/mcp/catalog.json`.
- **Defaults:** user defaults in `~/.claudine/mcp/defaults.json` and optional repo defaults in `<repo>/.claudine/mcp.json`.
- **Provider state:** sync/import/export state in `~/.claudine/mcp/provider-state.json`.
- **Wrapper activation:** `--mcp` enables effective defaults; `--use id-or-alias[,...]` adds explicit servers and enables MCP mode.
- **Provider export:** `claudine mcp export <provider> --apply` writes the effective Claudine set into provider-native config when runtime injection is not available or not implemented.

The important distinction is between provider-native support and Claudine-mediated support. Several providers have meaningful MCP implementations that Claudine does not yet import, export, sync, or inject.

## Provider Matrix

| Provider    | Provider-native MCP posture                                                                                                        | Claudine support today                     | Claudine runtime injection today | Main last-mile variance                                                                      |
|-------------|------------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------|----------------------------------|----------------------------------------------------------------------------------------------|
| Claude Code | Rich MCP client: tools, prompts, partial resources, partial roots, elicitation, OAuth, managed policy, multiple transports         | Import/export/sync                         | No                               | Native `--mcp-config` exists, but Claudine currently treats Claude as persistent-config only |
| Codex       | Strong MCP client: stdio + Streamable HTTP, tools, partial resources, elicitation, OAuth, managed requirements                     | Import/export/sync                         | Yes                              | Shadow `CODEX_HOME` with generated TOML                                                      |
| Gemini CLI  | Strong MCP client: stdio/SSE/HTTP, tools, prompts, partial resources, roots, OAuth, folder trust                                   | Import/export/sync                         | Yes                              | Shadow `GEMINI_CLI_HOME` with generated `.gemini/settings.json` and copied sidecars          |
| Goose       | Broad provider-native extension model: tools, prompts, roots, sampling, elicitation, partial resources                             | Not wired in current Claudine MCP behavior | No                               | One-run activation is argv synthesis, not config injection                                   |
| Kimi Code   | Provider-native MCP tools through user/project `mcp.json`, plugin `mcpServers`, HTTP/SSE OAuth, filters, approval rules            | Not wired in current Claudine MCP behavior | No                               | Persistent JSON only; no documented one-run injection flag                                   |
| OpenCode    | Strong MCP client: local/remote servers, tools, prompts, partial resources, roots, OAuth                                           | Import/export/sync                         | Yes                              | `OPENCODE_CONFIG_CONTENT` overlay, not strict isolation                                      |
| Qwen Code   | Provider-native MCP with tools, prompts, partial resources, OAuth, folder trust, settings hierarchy, daemon-mode session injection | Not wired in current Claudine MCP behavior | No                               | Standalone CLI lacks one-run injection; daemon SDK differs from CLI                          |
| Roo Code    | Claudine has repo MCP import/export support for `.roo/mcp.json` and known VS Code extension config surfaces                        | Import/export/sync                         | No                               | VS Code extension configuration rather than a standalone wrapper runtime                     |

Pi and Kilo Code appear in the MCP research roster but are not current compiled Claudine providers. Pi has no provider-native MCP support; third-party Pi MCP adapter extensions are extension-specific and should not be modeled as Pi-native support. Kilo Code has substantial OpenCode-derived MCP support and a plausible `KILO_CONFIG_CONTENT` runtime path, but Claudine does not yet expose Kilo as a provider.

## Key Variants Claudine Must Normalize

### Configuration Shape

The same logical MCP server may be represented as JSON, JSONC, TOML, YAML, or a plugin/extension manifest.

Claude, Gemini, Kimi, Qwen, and Roo are JSON-family providers, but their keys and locations differ. Claude uses `mcpServers` in `~/.claude.json` and `.mcp.json`. Gemini and Qwen embed `mcpServers` in general `settings.json` files. Kimi uses `$KIMI_CODE_HOME/mcp.json` and `.kimi-code/mcp.json`. Roo uses `.roo/mcp.json` plus VS Code extension state.

Codex uses TOML under `[mcp_servers.<id>]`, with managed requirements and plugin policy also expressed in TOML layers.

OpenCode uses JSON/JSONC under a top-level `mcp` object, with config layered from user, project, environment, remote, and managed sources.

Goose uses YAML under `extensions:` and has related permission, secret, allowlist, and adversary files.

This means Claudine’s catalog cannot be a lowest-common-denominator JSON blob. It needs normalized fields for common behavior, plus provider overrides for auth, filtering, timeouts, trust, and other native fields that must survive import/export.

### Runtime Injection

Runtime injection is the largest practical divider.

Codex, Gemini, and OpenCode are Claudine’s current runtime-injection targets:

- **Codex:** Claudine creates a shadow `CODEX_HOME` containing generated `config.toml`.
- **Gemini:** Claudine creates a shadow `GEMINI_CLI_HOME` containing generated `.gemini/settings.json`, with relevant sidecars copied where appropriate.
- **OpenCode:** Claudine injects an `OPENCODE_CONFIG_CONTENT` JSON overlay.

Claude Code has a strong native one-run mechanism: `--mcp-config <file-or-json>`, with `--strict-mcp-config` when the injected set should be exclusive. Claudine does not currently implement a Claude runtime injector, so Claude remains export/apply from Claudine’s perspective. Managed policy such as `disableSideloadFlags` can also block `--mcp-config`.

Goose has viable one-run activation, but not through a config blob. Claudine would need to synthesize repeated `--with-extension`, `--with-streamable-http-extension`, and `--with-builtin` flags for `goose run` or `goose session`.

Kimi and standalone Qwen should be treated as persistent-config providers for wrapper purposes. Kimi has no documented `--mcp-config` or inline config mechanism. Qwen standalone can be managed through settings and `qwen mcp` commands, while daemon mode supports per-session `newSession({mcpServers})`; that should be reserved for a future daemon adapter.

### Feature Surface

“MCP support” is not a single yes/no capability. Tool support does not imply resources, prompts, roots, sampling, or elicitation.

| Feature     | Claude  | Codex   | Gemini  | Goose   | Kimi                 | OpenCode | Qwen                               |
|-------------|---------|---------|---------|---------|----------------------|----------|------------------------------------|
| Tools       | Full    | Full    | Full    | Full    | Full                 | Full     | Full                               |
| Resources   | Partial | Partial | Partial | Partial | Unknown/undocumented | Partial  | Partial                            |
| Prompts     | Full    | None    | Full    | Partial | Unknown/undocumented | Full     | Full                               |
| Roots       | Partial | None    | Full    | Full    | Unknown/undocumented | Full     | Full in daemon, unclear standalone |
| Sampling    | Unknown | None    | None    | Full    | Unknown/undocumented | None     | Unknown                            |
| Elicitation | Full    | Full    | None    | Full    | Unknown/undocumented | None     | Unknown                            |

Codex is the clearest example of why this precision matters: it has a strong MCP client for tools and partial resources, but no surfaced MCP prompt support. Goose is the opposite extreme: it exposes one of the broadest client capability sets, including sampling and elicitation, but Claudine has not wired it into MCP management yet.

### Trust, Approval, and Security

Every provider treats MCP tools as powerful. None should be treated as inherently safe because the server speaks MCP.

Common patterns:

- Project-scoped config can execute local stdio commands.
- OAuth generally cannot bootstrap in non-interactive wrapper runs.
- Stdio servers often inherit process environment plus explicit env entries.
- Provider shell sandboxes do not necessarily contain MCP subprocesses.
- Tool annotations are hints unless a provider explicitly elevates them into policy.
- MCP tool results should be treated as untrusted model input.

Claude, Gemini, Qwen, and Codex have project or folder trust gates for project config. Kimi does not document an equivalent trust gate for project `.kimi-code/mcp.json`. OpenCode does not document a project-trust dialog for project MCP config. Goose has no repo-level MCP config file, but does have extension allowlists, per-tool permission files, and optional adversary review.

Claudine’s `protect` layer should continue to scan MCP responses defensively. Native MCP response sanitization is absent, partial, or undocumented across the provider set.

## Current Point of View

MCP support in Claudine should be understood as two layers:

1. **Provider-native support:** what the underlying agent CLI can do.
2. **Claudine-mediated support:** what Claudine can safely import, export, sync, and inject today.

Those are not always the same. Goose, Kimi, and Qwen have meaningful provider-native MCP support, but current Claudine MCP behavior does not implement their import/export/runtime paths. Claude has a capable native runtime flag, but Claudine runtime injection is currently implemented only for Codex, Gemini, and OpenCode. Roo has implementation-level import/export support in Claudine, but lacks the same current MCP research depth as the other providers.

The strategic direction should remain:

- Keep Claudine’s catalog provider-agnostic, but preserve provider-specific fields.
- Prefer runtime injection for wrappers when the provider offers a clean, reversible mechanism.
- Use export/apply for providers whose only safe path is persistent config.
- Treat OAuth tokens and secrets as opaque provider state, not catalog data to copy around.
- Keep MCP opt-in by default so sessions do not accidentally inherit expensive or risky tool surfaces.
- Model provider feature support precisely: tools are not the same as resources, prompts, roots, sampling, or elicitation.
- Surface drift explicitly when provider-native capability exists but Claudine has not implemented the last mile yet.

The core standard is shared, but the operational contract is provider-specific. Claudine’s value is making that variance visible and controlled instead of forcing users to remember every provider’s MCP dialect.
