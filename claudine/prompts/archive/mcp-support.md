# MCP Support

## Summary

Claudine should treat MCP the same way it treats hooks, skills, and commands: as a cross-provider capability with a Claudine-owned abstraction layer, not as eight unrelated vendor configurations.

The current draft has the right product instinct, but it assumes more runtime control than the providers actually expose. The design needs three explicit constraints:

1. Claudine can own the **catalog** of known MCP servers.
2. Claudine can own **session composition** when the provider exposes a safe runtime injection path.
3. Claudine must fall back to **sync/export** when a provider cannot be cleanly wrapped at runtime.

That gives us a practical design that works now, instead of a universal `--mcp` promise that only some providers can honor.

## Goals

- Maintain one normalized catalog of MCP servers across providers.
- Default to zero active servers so context remains intentional.
- Import native provider configs without destructive edits.
- Allow repo-specific defaults that can replace user defaults for a given repo.
- Support ephemeral runtime MCP activation for providers that can be wrapped safely.
- Preserve provider-native behavior when Claudine MCP mode is not in use.

## Non-Goals

- Dynamically changing the active MCP set in the middle of an already-running interactive session in v1.
- Shipping runtime MCP support for all 8 providers before the research is complete.
- Solving secret storage beyond what current providers already do in v1.

## Rollout

The research in `claudine/docs/mcp` is not complete enough to justify an all-providers day-one implementation. The runtime feature should ship in phases.

| Provider | Import Into Catalog | `--mcp` Runtime Mode | Initial Strategy |
| --- | --- | --- | --- |
| OpenCode | Yes | Yes | Inject generated config through `OPENCODE_CONFIG_CONTENT` |
| Codex | Yes | Yes | Generate temporary `config.toml` in a shadow `HOME` |
| Gemini | Yes | Yes | Generate temporary `settings.json` in a shadow `HOME`, optionally gate with `--allowed-mcp-server-names` |
| Claude | Yes | Not in phase 1 | Import/sync only until local/project/plugin precedence is modeled cleanly |
| Roo Code | Yes | Not applicable | Sync/export only; Roo is not a standalone CLI wrapper target |
| Qwen Code | Later | Later | Blocked on incomplete MCP research |
| Goose | Later | Later | Blocked on missing MCP research doc |
| Kimi Code | Later | Later | Blocked on missing MCP research doc |

The important decision is that **provider parity is gated by verified design**, not by the fact that Claudine already knows about the provider elsewhere.

## Command Surface

The current feature set is missing one critical command: a way to materialize Claudine-managed MCP config back into native provider config for providers that cannot be wrapped. The command surface should be:

- `claudine mcp init`
    - Scan native provider MCP configs and import them into the Claudine catalog.
- `claudine mcp`
    - List catalog entries, aliases, defaults, and provider presence.
- `claudine mcp show <id>`
    - Show the normalized server definition and import provenance.
- `claudine mcp default [ids...]`
    - Set user-scope defaults.
- `claudine mcp default --repo [ids...]`
    - Set repo-scope defaults for the current repo.
- `claudine mcp alias add <id> <alias>`
    - Add an alias for an existing catalog entry.
- `claudine mcp alias remove <alias>`
    - Remove an alias.
- `claudine mcp remove <id>`
    - Remove a catalog server after confirmation.
- `claudine mcp sync <provider> [--scope user|repo] [--apply]`
    - Write the effective Claudine-managed MCP set back into the provider's native config.
- `claudine <agent> --mcp [--use <id-or-alias>[,<id-or-alias>...]]`
    - Launch a wrapped agent using Claudine-owned MCP session composition.

`claudine mcp sync` is necessary because Roo Code cannot be wrapped and Claude may not be safe to wrap in phase 1.

## Storage Model

The current `user-mapping.json` / `project-mapping.json` idea is too thin. Arrays of names are not enough to support dedupe, rename tracking, provenance, or sync ownership. Claudine should separate four concerns:

### 1. Global Catalog

Path:

- `~/.claudine/mcp/catalog.json`

This is the source of truth for normalized MCP definitions.

Example shape:

```json
{
  "version": 1,
  "servers": {
    "google-calendar": {
      "id": "google-calendar",
      "aliases": ["calendar", "gcal"],
      "transport": "stdio",
      "command": "/opt/homebrew/bin/uvx",
      "args": ["mcp-server-google-calendar"],
      "cwd": null,
      "env": {
        "GOOGLE_APPLICATION_CREDENTIALS": "/Users/ken/.config/gcal.json"
      },
      "url": null,
      "headers": {},
      "enabled_tools": [],
      "disabled_tools": [],
      "required": false,
      "metadata": {
        "description": "Google Calendar MCP server",
        "created_from": "codex:user",
        "fingerprint": "sha256:..."
      },
      "provider_overrides": {}
    }
  }
}
```

### 2. User Defaults

Path:

- `~/.claudine/mcp/defaults.json`

This file contains the user's default active set.

```json
{
  "version": 1,
  "defaults": ["sequential-thinking", "slack"]
}
```

### 3. Repo Defaults

Path:

- `<repo>/.claudine/mcp.json`

This file contains the repo's desired defaults and intentionally stores only catalog IDs, not full server definitions or secrets.

```json
{
  "version": 1,
  "defaults": ["github", "linear"]
}
```

If a repo references a catalog ID the current user does not have, Claudine should warn and continue.

### 4. Provider State

Path:

- `~/.claudine/mcp/provider-state.json`

This is local machine state, not a user-facing config file. It tracks what Claudine has seen in native provider configs and what Claudine itself has written back.

Example shape:

```json
{
  "version": 1,
  "providers": {
    "codex": {
      "user": [
        {
          "catalog_id": "google-calendar",
          "native_name": "calendar",
          "source": "~/.codex/config.toml",
          "origin": "imported",
          "last_seen": "2026-03-09T02:14:00Z"
        }
      ]
    }
  },
  "repos": {
    "/Volumes/coding/personal/rusty-biscuit": {
      "providers": {
        "gemini": {
          "repo": [
            {
              "catalog_id": "linear",
              "native_name": "linear",
              "source": ".gemini/settings.json",
              "origin": "managed",
              "last_seen": "2026-03-09T02:15:00Z"
            }
          ]
        }
      }
    }
  }
}
```

This is the key design change: the mapping/state file must be rich enough to answer:

- Which native server became which catalog entry?
- Was it imported from the provider, or written there by Claudine?
- Which repo did a repo-scoped import belong to?
- Which native name should be updated or removed during sync?

## Normalized Server Schema

The catalog cannot only mirror Claude Code's MCP schema. Codex and OpenCode already require additional fields. The normalized schema should include:

- `transport`: `stdio | http | sse`
- `command`: executable for `stdio`
- `args`: command arguments for `stdio`
- `cwd`: optional working directory
- `env`: environment variables for `stdio`
- `url`: endpoint for `http` or `sse`
- `headers`: static HTTP headers
- `enabled_tools`: allow-list of tool names
- `disabled_tools`: deny-list of tool names
- `required`: whether session startup should fail if the server is unavailable
- `metadata`: description, provenance, import timestamps, fingerprint
- `provider_overrides`: escape hatch for provider-specific fields we cannot normalize yet

Design rule: prefer a normalized superset first, and only use `provider_overrides` for fields that do not generalize cleanly.

## Import Design

`claudine mcp init` should be idempotent. Its job is to inventory native provider MCP servers and merge them into the Claudine catalog, not to mutate provider config.

The import algorithm should be:

1. Discover supported provider config files for the current machine and current repo.
2. Parse each provider's MCP definitions into the normalized schema.
3. Compute a stable fingerprint from the normalized server definition.
4. If the fingerprint already exists in the catalog:
   - Reuse the existing catalog entry.
   - Add the provider's native name as an alias if useful.
5. If the fingerprint is new:
   - Create a new catalog entry with a stable slug ID.
6. Record provenance in `provider-state.json`.
7. Print a report showing imported, merged, conflicted, and skipped entries.

### Conflict Rules

- Same fingerprint, different names: merge into one catalog entry and preserve native names as aliases.
- Same name, different fingerprint: create a new catalog ID and flag the ambiguity to the user.
- Alias collisions with existing IDs: reject the alias and require explicit rename.

## Defaults and Effective Session Set

The current draft is directionally right: defaults should start empty.

The effective MCP set for a session should be computed as:

1. Start with user defaults.
2. If repo defaults exist for the current repo, replace the user defaults with repo defaults.
3. Add any explicit `--use` values from the command line.
4. Add any `#tag` values extracted from the prompt, if the prompt is passing through Claudine.
5. De-duplicate the final set.

Repo defaults should replace user defaults rather than merge with them. That keeps repo behavior intentional and mirrors Claudine's broader repo-isolation posture.

## `#tag` Activation

The `#tag` concept is good, but the current draft overstates where it can work.

### What Works

`#tag` activation works when Claudine sees the prompt text **before** the provider processes it:

- non-interactive wrapped execution
- prompt-bearing CLI invocations
- any future Claudine command that accepts a prompt directly

When a `#tag` is detected:

- resolve it to a catalog ID or alias
- add that server to the session set
- remove the tag token from the outgoing prompt

### What Does Not Work in v1

For a long-running interactive wrapped session like:

```sh
claudine codex --mcp
```

Claudine does **not** see later prompts typed inside Codex, Gemini, or OpenCode. That means it cannot reliably hot-add MCP servers in the middle of the session unless the provider supports dynamic reloading and Claudine has a provider-specific control path.

So the v1 rule should be:

- `#tag` is a launch-time composition feature, not a mid-session mutation feature.
- fully interactive sessions without an initial prompt should use `--use`

Example:

```sh
claudine codex --mcp --use google-calendar,slack
```

This is a necessary constraint, not a nice-to-have clarification.

## Tag Matching

Resolution order should be deterministic:

1. exact catalog ID
2. exact alias
3. normalized exact match (case-insensitive, `-` and `_` treated as equivalent)
4. prefix match
5. substring match

If more than one result exists at the same rank:

- in an interactive TTY: prompt the user to choose
- in non-interactive mode: fail with a concise ambiguity error and show the candidates

Never silently choose among multiple partial matches.

## Runtime Injection Strategy

There is no single cross-provider injection mechanism. Claudine should use a strategy table per provider.

### OpenCode

- Preferred path: inject generated config through `OPENCODE_CONFIG_CONTENT`
- Benefit: no file edits, no shadow-home complexity, truly session-scoped
- Phase: 1

### Codex

- Preferred path: create a temporary `~/.codex/config.toml` inside a shadow `HOME`
- Reason: Codex already fits Claudine's existing shadow-home model
- Caveat: project config layering and trusted-project behavior must be tested carefully
- Phase: 1

### Gemini

- Preferred path: create a temporary `~/.gemini/settings.json` inside a shadow `HOME`
- Optional reinforcement: use `--allowed-mcp-server-names` to gate the launched session
- Caveat: sidecar enablement and OAuth files may need mirroring into the shadow home
- Phase: 1

### Claude

- Preferred eventual path: temporary user/local overlay in a shadow `HOME`
- Caveat: user, project, local, plugin, and managed scopes all exist, which makes true isolation more subtle
- Phase: import/sync first, runtime later

### Roo Code

- No wrapper runtime mode
- Support only import and sync

### Qwen, Goose, Kimi

- No runtime design should be committed until `claudine/docs/mcp` contains completed research docs for them

## Sync / Export Design

`claudine mcp sync` should materialize the effective Claudine-managed set into a provider's native config. This is how we support non-wrapper providers and team workflows that still want native provider config populated.

Rules:

- Sync only touches the provider's MCP section, never unrelated config.
- Sync only removes entries previously marked as `origin = "managed"` in `provider-state.json`.
- Imported foreign/native entries that Claudine did not create are preserved.
- Sync always makes a backup before writing.

This gives Claudine a safe coexistence story instead of an all-or-nothing takeover.

## Native Config Coexistence

Without `--mcp`:

- the provider behaves exactly as it does today
- Claudine does not mask or rewrite MCP configuration

With `--mcp`:

- the active session is composed from Claudine defaults plus explicit session additions
- provider-native MCP config is not treated as the session source of truth
- provider-specific runtime strategy decides whether that composition is injected ephemerally or must be materialized via sync first

That distinction should stay explicit throughout the implementation.

## Secrets and Output Safety

v1 should not introduce a new secret management abstraction. Claudine should:

- preserve imported env/header values as-is in the catalog
- never duplicate secrets into defaults or provider-state files
- redact sensitive values in `claudine mcp` and `claudine mcp show` output by default

If secret-manager integration is added later, it should layer onto the catalog rather than block v1.

## Design Summary

The refined design is:

- one global Claudine catalog
- explicit user and repo default sets
- structured provider-state instead of thin provider-to-name mappings
- launch-time `#tag` activation only
- runtime `--mcp` only where the provider has a credible session injection strategy
- sync/export for the providers that do not

That keeps the feature cohesive while staying honest about what the underlying platforms can actually support.
