# PolicyEngine

`PolicyEngine` is Claudine's provider-agnostic permissions subsystem in the `services` module. It normalizes provider-native permission models into a single query and mutation surface.

## Purpose

PolicyEngine answers questions like: can this provider read/write a path, will a command run automatically or be denied, is a domain allowed, can an MCP server/tool be used, can a subagent be spawned, what config change would grant or deny a permission.

It is intentionally separate from `ProtectService`. After the 2026-04-06 Protect refactor, `ProtectService` is a standalone regex-backed deny catalog that blocks dangerous bash commands, sensitive write paths, and MCP prompt-injection payloads with a binary `Allow` or `Block` result. `PolicyEngine` remains the source of truth for what the provider is configured to allow, ask, deny, or leave ambiguous.

## Built-in Backends

7 providers: Claude, Codex, Gemini, OpenCode, Goose, Kimi, Qwen. Each backend has capability metadata (`engine.capabilities(provider)`) covering fidelity, query types, and mutation support.

## Core Concepts

### Configured vs Effective Policy

- `ConfiguredPolicySnapshot` -- on-disk policy only
- `EffectivePolicySnapshot` -- on-disk policy plus CLI/runtime overrides (e.g. `--permission-mode auto`, `--full-auto`)

### Native vs Canonical

Backends keep native representations internally for mutation planning. The engine also produces a canonical model with six axes: filesystem, commands, network, MCP, agents, runtime.

### Explainability

Queries return `QueryResult` (not a bare boolean):

- `effect: Option<PolicyEffect>` -- Allow, Ask, Deny, or None (unknown)
- `certainty` -- Exact, BestEffort, Unknown
- `stability` -- Stable, MayChangeWithCli, MayChangeAtRuntime, Unknown
- `matched_rules` -- canonical rules with provenance back to native source
- `explanation` -- summary + structured reasons with source id, native reference, message, fidelity
- `warnings` -- trust ambiguity, approximation notes, unsupported mutations

## PolicyContext

Every query starts with `PolicyContext`: cwd (path normalization), repo_root (config discovery), home_dir (user config), system_root (tests), env (provider discovery), trust (trust-gated providers).

Path queries are normalized relative to cwd and classified as workspace, provider-config, home, temp, system, or external.

## Query API

Both snapshot types expose convenience methods: `can_read`, `can_write`, `can_traverse`, `can_execute`, `can_access_domain`, `can_use_mcp_server`, `can_use_mcp_tool`, `can_spawn_subagent`, `can_switch_mode`, `can_modify_own_config`. Generic entrypoint: `snapshot.query(&PolicyQuery::...)`.

MCP tool queries inherit server-level policy -- if a server is denied, its tools are denied too.

`can_modify_own_config()` checks whether canonical policy protects the provider's own config paths, not filesystem writability.

## Mutation Planning

PolicyEngine supports mutation planning via `PolicyChange` with two persistence modes:

- `Persistent` -- plan config-file changes (targets: Auto, UserConfig, RepoConfig, LocalOverride)
- `OneShot` -- plan CLI/env overrides only

Operations include GrantRead/Write, DenyRead/Write, AllowCommand, DenyCommand, AllowDomain, AllowMcpServer/Tool, SetApprovalMode, SetSandboxMode, and more.

Plans are inspectable before application. Persistent plans contain file edit previews (path, description, after_preview). One-shot plans contain argv/env for the caller. `plan.apply()` only executes persistent edits.

LocalOverride is provider-specific -- Claude supports it, providers without a local override concept reject it.

## Ambiguity Cases

- **Trust-gated repo policy**: unknown trust produces unknown query answers with warnings (Codex, Gemini)
- **CLI-sensitive configured results**: configured snapshots return `MayChangeWithCli` where launch flags could change behavior
- **Provider-specific limitations**: certainty degrades to BestEffort, provenance fidelity may be Approximate, warnings explain limitations
