# Rusty Biscuit Monorepo

## Workspace Gotchas

- 48 workspace members. Source of truth is `cargo metadata --no-deps --format-version 1` — not directory names.
- `schematic/schema` lives in the repo but is **excluded from the workspace**. Use `--manifest-path schematic/schema/Cargo.toml` to work on it.

## Package Area Conventions

- Most areas follow a `{area}/lib` + `{area}/cli` split. Notable exceptions:
    - `biscuit-visualized`, `tabby` — single crate
    - `homelab` — lib/cli/server plus per-device integration crates
    - `schematic` — `define` / `definitions` / `gen` / `oauth` / `schema`
    - `unchained-ai` — includes the `model_id` proc-macro crate
- `biscuit-speaks` CLI binary is named `so-you-say` (lives under `biscuit-speaks/cli`).
- `biscuit-tui` follows the lib/cli split; CLI binary is named `question` (lives under `biscuit-tui/cli`).

## Root `just` Coverage

Root `justfile` exposes `just test|lint|build|install|doctest`, iterating a **curated** area list — not every workspace member.

- shared recipes are all located in the `/just` folder
- each package area has it's own `justfile` but the shared recipes are leveraged as much as possible to keep as much uniformity as possible

## Rustdoc Convention

- No `# H1` inside `///` blocks — rustdoc already titles the item.
- `## H2` sections: `Examples`, `Returns`, `Errors`, `Panics`, `Safety`, `Notes`.
- Order: summary → `Examples` → `Returns` → `Errors` → `Panics` → `Safety` → `Notes`.

## Drift Maintenance

Update alongside code changes:

- READMEs when public behavior changes
- `docs/dependencies.md` (and per-area `docs/dependencies.md`) when crates are added/removed
- `.claude/skills/` when architecture or workflows change
- This file when workspace layout, commands, or repo-wide conventions change

## Authoritative Docs

- run `sniff repo` for the up to date list of package areas and packages
- Local skill catalog under `.claude/skills/` is the authoritative skill list.
