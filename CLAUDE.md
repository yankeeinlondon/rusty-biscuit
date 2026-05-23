# Rusty Biscuit Monorepo

## Workspace Gotchas

- 48 workspace members. Source of truth is `cargo metadata --no-deps --format-version 1` — not directory names.
- `schematic/schema` lives in the repo but is **excluded from the workspace**. Use `--manifest-path schematic/schema/Cargo.toml` to work on it.

## Language 

- always prefer **US English (en-US)** over other English variants such as UK English (en-GB) when creating symbol names or writing documentation

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

## Rules

- **Rule 1** — Think Before Coding.
    No silent assumptions. State what you're assuming. Surface trade-offs. Ask before guessing. Push back when a simpler approach exists.
- **Rule 2** — Simplicity First.
    Minimum code that solves the problem. No speculative features. No abstractions for single-use code. If a senior engineer would call it overcomplicated — simplify.
- **Rule 3** — Surgical Changes.
    Touch only what you must. Don't "improve" adjacent code, comments, or formatting. Don't refactor what isn't broken. Match existing style.
- **Rule 4** — Goal-Driven Execution.
    Define success criteria. Loop until verified. Don't tell Claude what steps to follow, tell it what success looks like and let it iterate.

## Hashing

- any hashing requirements should prefer using the crypto, non-crypto, and password hashing that **biscuit-hash** provides
- in the case of hashing Markdown documents, the **Darkmatter** hasher should be used (as it uses a Markdown aware approach)
