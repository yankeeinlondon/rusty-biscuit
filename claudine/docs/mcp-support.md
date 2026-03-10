# MCP Support

Claudine ships a normalized MCP catalog with three distinct workflows:

- import native provider configs into `~/.claudine/mcp/`
- choose effective defaults at user or repo scope
- either export those defaults back to native configs or inject them at wrapper runtime when the provider supports it

## Command Family

| Command | Behavior |
|---------|----------|
| `claudine mcp` | List catalog entries, defaults, and provider presence |
| `claudine mcp init` | Import the catalog, create defaults files, and guide first-time setup |
| `claudine mcp list [filter]` | List catalog entries and optionally filter by ID substring |
| `claudine mcp list --alias <filter>` | Filter the catalog by alias substring |
| `claudine mcp add local` | Add a local stdio MCP server interactively |
| `claudine mcp add remote` | Add a remote HTTP MCP server interactively |
| `claudine mcp config <id-or-alias>` | Show a normalized server definition plus provenance |
| `claudine mcp default [ids...]` | Replace user-scope default server IDs |
| `claudine mcp default --repo [ids...]` | Replace repo-scope default server IDs |
| `claudine mcp alias <id-or-alias> <alias>` | Add a catalog alias |
| `claudine mcp remove <id-or-alias>` | Remove a catalog entry or just an alias |
| `claudine mcp check` | Validate transport combinations, aliases, defaults, and provider-state |
| `claudine mcp sync` | Refresh the catalog from provider configs (pull-only; use `export` to push) |
| `claudine mcp export <provider> [--scope user\|repo] [--apply]` | Dry-run or apply export of effective defaults to a native provider config |

`--json` is available across the command family. Text `config` output redacts env/header values by key name; `claudine mcp config --json` returns the stored definition, including env/header values.

## Claudine Storage

| Path | Purpose |
|------|---------|
| `~/.claudine/mcp/catalog.json` | Normalized server catalog keyed by stable ID |
| `~/.claudine/mcp/defaults.json` | User-scope ordered default server IDs |
| `~/.claudine/mcp/provider-state.json` | Imported/managed provenance and native-name tracking |
| `<repo>/.claudine/mcp.json` | Repo-scope ordered default server IDs |

Repo defaults replace user defaults; they do not merge.

## Provider Coverage

| Provider | Import | Sync | Runtime `--mcp` | Notes |
|----------|--------|------|-----------------|-------|
| Claude | `~/.claude.json`, repo `.mcp.json`, `.claude/settings.local.json`, and plugin configs | user `~/.claude.json`, repo `.mcp.json` | No | Import/sync only in v1 |
| Codex | user/repo `.codex/config.toml` | user/repo `.codex/config.toml` | Yes | Shadow-home TOML injection |
| Gemini | user/repo `.gemini/settings.json` | user/repo `.gemini/settings.json` | Yes | Shadow-home JSON injection plus `--allowed-mcp-server-names` |
| OpenCode | user `~/.config/opencode/opencode.json`, repo `opencode.json` | same | Yes | Runtime injection uses `OPENCODE_CONFIG_CONTENT` |
| Roo | repo `.roo/mcp.json`; macOS user import from VS Code global storage | repo `.roo/mcp.json` | No | Import/sync only; no wrapper command |
| Goose, Kimi, Qwen | No | No | No | Not modeled in the MCP module yet |

## Import And Refresh Workflow

`claudine mcp init` never edits provider configs. It scans the supported native files, normalizes each server, fingerprints the provider-agnostic definition, and then:

- imports a new catalog entry when the fingerprint is new
- merges into an existing entry when the fingerprint already exists
- adds an alias when the same server arrived under a different native name
- records import provenance in `provider-state.json`

`claudine mcp sync` refreshes the catalog from supported provider configs and creates explicit defaults files when they do not exist yet.

`claudine mcp export <provider>` works from the effective default set for the selected scope. It is a dry run unless `--apply` is passed. On apply it:

- creates a backup before writing an existing native config
- preserves non-MCP config in the provider file
- removes previously managed MCP entries that are no longer desired
- keeps foreign/native entries that Claudine does not manage
- records managed ownership and native names in `provider-state.json`

## Wrapper Runtime Behavior

`claudine <provider> --mcp` composes a session from:

1. repo defaults if `<repo>/.claudine/mcp.json` exists, otherwise user defaults
2. any explicit `--use id-or-alias[,id-or-alias...]`
3. resolved `#tags` stripped from the prompt in initial Codex, Gemini, and OpenCode prompts

`--use` also enables MCP composition by itself; it does not replace defaults.

Runtime injection is provider-specific:

- OpenCode sets `OPENCODE_CONFIG_CONTENT` and does not need a shadow home.
- Codex writes `mcp_servers` into the shadow `~/.claudine/.codex/config.toml`.
- Gemini writes `mcpServers` into the shadow `~/.claudine/.gemini/settings.json`, carries the mirrored sidecar files when present, and appends `--allowed-mcp-server-names`.

Wrapper `--dry-run` includes the resolved MCP server set, prompt tags, cleaned prompt, injected env vars, extra args, and written shadow-home files.

## Current Limits

- Runtime MCP injection is not available for Claude, Goose, Kimi, Qwen, or Roo. Use `claudine mcp export <provider> --apply` when that provider has native config support.
- Prompt-tag activation runs on the initial prompt only. Tags are stripped before forwarding; unmatched tags warn by default and fail with `--strict`.
- Defaults are stored verbatim. Missing IDs surface later as warnings during wrapper launch or `claudine mcp check`.
- `claudine mcp remove` deletes the catalog entry only. It does not clean defaults or native provider configs for you.
- Shadow-home runtime injection for Codex and Gemini writes under `~/.claudine` and currently leaves those shadow config files in place after the wrapped process exits.
