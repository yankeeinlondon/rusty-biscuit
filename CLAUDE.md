# Rusty Biscuit Monorepo

## Workspace Scope

- The Cargo workspace currently has 48 members.
- The source of truth for workspace membership is the root `Cargo.toml` plus `cargo metadata --no-deps --format-version 1`.
- Not every top-level directory is a workspace member. Trust Cargo metadata over directory names.
- `schematic/schema` exists in the repo but is currently excluded from the workspace.

## Monorepo Structure

Current workspace members, grouped by package area:

```txt
• agent-sandbox
    • agent-sandbox-cli v0.1.0 (agent-sandbox/cli) [Rust]
• biscuit-file
    • biscuit-file-cli v0.1.0 (biscuit-file/cli) [Rust]
    • biscuit-file v0.1.0 (biscuit-file/lib) [Rust]
• biscuit-hash
    • biscuit-hash-cli v0.1.0 (biscuit-hash/cli) [Rust]
    • biscuit-hash v0.1.0 (biscuit-hash/lib) [Rust]
• biscuit-speaks
    • biscuit-speaks-cli v0.1.0 (biscuit-speaks/cli) [Rust]
    • biscuit-speaks v0.1.0 (biscuit-speaks/lib) [Rust]
• biscuit-terminal
    • biscuit-terminal-cli v0.1.0 (biscuit-terminal/cli) [Rust]
    • biscuit-terminal v0.1.0 (biscuit-terminal/lib) [Rust]
• biscuit-visualized
    • biscuit-visualized v0.1.0 (biscuit-visualized/src) [Rust]
• claudine
    • claudine-cli v0.1.0 (claudine/cli) [Rust]
    • claudine v0.1.0 (claudine/lib) [Rust]
• darkmatter
    • darkmatter-cli v0.1.0 (darkmatter/cli) [Rust]
    • darkmatter v0.1.0 (darkmatter/lib) [Rust]
• homelab
    • arcam-amp-integration v0.1.0 (homelab/arcam-amp-integration) [Rust]
    • eversolo-integration v0.1.0 (homelab/eversolo-integration) [Rust]
    • homelab-cli v0.1.0 (homelab/cli) [Rust]
    • homelab v0.1.0 (homelab/lib) [Rust]
    • homelab-server v0.1.0 (homelab/server) [Rust]
    • sony-receiver-integration v0.1.0 (homelab/sony-receiver-integration) [Rust]
    • unfolded-integration-helper v0.1.0 (homelab/unfolded-integration-helper) [Rust]
• messenger
    • messenger-cli v0.1.0 (messenger/cli) [Rust]
    • messenger v0.1.0 (messenger/lib) [Rust]
• model-citizen
    • model-citizen-cli v0.1.0 (model-citizen/cli) [Rust]
    • model-citizen v0.1.0 (model-citizen/lib) [Rust]
• playa
    • playa-cli v0.1.0 (playa/cli) [Rust]
    • playa v0.1.0 (playa/lib) [Rust]
• queue
    • queue-cli v0.1.0 (queue/cli) [Rust]
    • queue v0.1.0 (queue/lib) [Rust]
• research
    • research-cli v0.1.0 (research/cli) [Rust]
    • research v0.1.0 (research/lib) [Rust]
• schematic
    • schematic-define v0.1.0 (schematic/define) [Rust]
    • schematic-definitions v0.1.0 (schematic/definitions) [Rust]
    • schematic-gen v0.1.0 (schematic/gen) [Rust]
    • schematic-oauth v0.1.0 (schematic/oauth) [Rust]
    • schematic-schema (schematic/schema) [excluded from workspace]
• sniff
    • sniff-cli v0.1.0 (sniff/cli) [Rust]
    • sniff v0.1.0 (sniff/lib) [Rust]
• tabby
    • tabby v0.1.0 (tabby) [Rust]
    • ui v0.1.0 (tabby/ui) [Rust]
• tree-hugger
    • tree-hugger-cli v0.1.0 (tree-hugger/cli) [Rust]
    • tree-hugger v0.1.0 (tree-hugger/lib) [Rust]
• tui
    • tui v0.1.0 (tui) [Rust]
• unchained-ai
    • model_id v0.1.0 (unchained-ai/model_id) [Rust]
    • unchained-ai-cli v0.1.0 (unchained-ai/cli) [Rust]
    • unchained-ai-gen v0.1.0 (unchained-ai/gen) [Rust]
    • unchained-ai v0.1.0 (unchained-ai/lib) [Rust]
• worktree
    • worktree-cli v0.1.0 (worktree/cli) [Rust]
    • worktree v0.1.0 (worktree/lib) [Rust]
```

## Common Commands

Prefer `just` when the relevant recipe exists, but do not assume the root `justfile` covers the entire workspace.

### Root `just`

The root `justfile` exposes the main orchestration commands:

- `just test`
- `just lint`
- `just build`
- `just install`
- `just doctest`

These recipes iterate a curated list of package areas from the root `justfile`. They do **not** currently cover every workspace member.

Entries currently listed in the root `areas := ...` variable:

- `biscuit-file`
- `biscuit-hash`
- `biscuit-speaks`
- `biscuit-terminal`
- `claudine`
- `darkmatter`
- `homelab`
- `model-citizen`
- `playa`
- `queue`
- `research`
- `schematic`
- `sniff`
- `tree-hugger`
- `unchained-ai`
- `so-you-say`

Operational note:

- `so-you-say` appears in the root `areas := ...` list, but there is no top-level `so-you-say/justfile`; that CLI lives under `biscuit-speaks/cli`.

Workspace members not covered by the root `areas := ...` list include:

- `agent-sandbox`
- `biscuit-visualized`
- `messenger`
- `tabby`
- `tui`
- `worktree`

For those, use an area `justfile` when present or fall back to direct `cargo` commands.

### Area `justfile`s

Package areas with their own `justfile`:

- `biscuit-file`
- `biscuit-hash`
- `biscuit-speaks`
- `biscuit-terminal`
- `biscuit-visualized`
- `claudine`
- `darkmatter`
- `homelab`
- `messenger`
- `model-citizen`
- `playa`
- `queue`
- `research`
- `schematic`
- `sniff`
- `tree-hugger`
- `unchained-ai`
- `worktree`

Top-level workspace areas without an area `justfile` today:

- `agent-sandbox`
- `tabby`
- `tui`

### Focused Cargo Commands

For targeted work, use direct Cargo commands as needed:

- `cargo test -p <crate>`
- `cargo test -p <crate> <filter> -- --nocapture`
- `cargo build -p <crate>`
- `cargo fmt --package <crate>`
- `cargo metadata --no-deps --format-version 1`

If you need to work on `schematic/schema`, use `--manifest-path schematic/schema/Cargo.toml` because it is excluded from the main workspace.

## Local Skills

This repository has a large local skill catalog under `.claude/skills/`. The most relevant repo-specific skills are:

- `biscuit-hash`
- `biscuit-speaks`
- `biscuit-terminal`
- `biscuit-visualized`
- `claudine`
- `darkmatter`
- `homelab`
- `messenger`
- `model-citizen`
- `playa`
- `queue`
- `research`
- `schematic`
- `schematic-define`
- `sniff`
- `so-you-say`
- `tree-hugger`
- `unchained-ai`
- `unfolded-circle`

There are also general-purpose Rust and tooling skills available locally, including:

- `acp`
- `agent-observability`
- `async-trait`
- `audio-programming`
- `clap`
- `cli`
- `color-eyre`
- `crossterm`
- `dirs`
- `hugging-face-api`
- `indicatif`
- `monorepos`
- `nextest`
- `ollama`
- `prettyplease`
- `resvg`
- `rig`
- `rust`
- `rust-testing`
- `rust-tray-icon`
- `serial_test`
- `syntect`
- `terminal`
- `thiserror`
- `toml`
- `ts_rs`
- `tts`
- `tui`
- `two-face`
- `viuer`

The source of truth is the directory listing under `.claude/skills/`.

## Rust Documentation Best Practices

- Avoid explicit `# Heading` (H1) inside a `///` docblock unless intentionally titling the item.
- Rustdoc already supplies the item name as a top-level title.
- Adding an H1 usually duplicates visual hierarchy.
- Use `## Heading` (H2) for primary sections.
- Common sections:
    - `## Examples`
    - `## Returns`
    - `## Errors`
    - `## Panics`
    - `## Safety`
    - `## Notes`
- Use `### Heading` (H3) only for subsections.

Recommended section order:

1. Brief summary paragraph
2. `## Examples`
3. `## Returns`
4. `## Errors`
5. `## Panics`
6. `## Safety`
7. `## Notes` or `## Implementation Notes`

## Testing

Common testing crates and patterns used across the repo:

- `wiremock` for HTTP mocking in provider/API tests
- `tempfile` for temporary workspace and output fixtures
- `serial_test` for env-var or global-state isolation

Use the repo `just` recipes when they apply, and use focused `cargo test -p ...` runs when the root orchestration would be too broad or would skip the package you are changing.

### Error Handling

- Library code generally uses `thiserror` for domain error types.
- Avoid `unwrap()` and `expect()` in production code paths.
- Public APIs should return `Result` where failure is possible.

## Documentation Conventions

Package documentation follows a layered structure:

- Base README at the package-area root for goals and module links
- Submodule READMEs for implementation details
- `docs/` folders for package-specific research, design notes, and dependency details

Avoiding drift:

- Update READMEs when public behavior changes
- Update `docs/dependencies.md` when crates are added or removed
- Update skill files under `.claude/skills/` when architecture or workflows change
- Update this `CLAUDE.md` when workspace layout, commands, or repo-wide conventions change

## Additional Documentation

Useful current docs in this worktree:

- `docs/package-structure.md`
- `docs/dependencies.md`
- `docs/ai-tooling.md`
- `docs/publishing-to-crates.md`
- `docs/shell-completions.md`
- `docs/unfolded-circle-integrations.md`
- `research/docs/architecture.md`

Package-specific docs worth checking when relevant:

- `claudine/docs/`
- `darkmatter/docs/`
- `homelab/docs/`
- `messenger/docs/`
- `model-citizen/docs/`
- `schematic/docs/`
- `sniff/docs/`
- `unchained-ai/docs/`
- `worktree/docs/`
