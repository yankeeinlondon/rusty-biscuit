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
    - `claudine` — `lib` / `cli` plus `contract` (biscuit-contract adapter), `catalog-types` (leaf: shared provider vocab enums), and `gen` (the `claudine-gen` codegen binary that generates each `provider/<slug>/data.rs`)
- `biscuit-speaks` CLI binary is named `so-you-say` (lives under `biscuit-speaks/cli`).
- `biscuit-tui` follows the lib/cli split; CLI binary is named `question` (lives under `biscuit-tui/cli`).

## Root `just` Coverage

Root `justfile` exposes `just test|lint|build|install|doctest`, iterating a **curated** area list — not every workspace member.

- shared recipes are all located in the `/just` folder
- each package area has it's own `justfile` but the shared recipes are leveraged as much as possible to keep as much uniformity as possible

## Formatting

`main` branch is the formatting authority.

- Never run `cargo fmt` / `rustfmt` write-mode unless explicitly asked. Match surrounding style by hand when editing.
- Reason: `rust-toolchain.toml` pins `channel = "stable"`, not a specific rustfmt version. Ad-hoc fmt reformats to whatever rustfmt floats in locally, which drifts from `main` and poisons branch↔`main` merges — a repo-wide reformat touches nearly every line, so git silently mis-merges reformatted-but-old code with `main`'s real changes.
- `cargo fmt --check` (read-only) is fine for diagnosis.
- To resolve a merge poisoned by a stray reformat: reset every file whose only branch-side change was a `style`/reformat commit back to `main` (`git checkout MERGE_HEAD -- <file>`), keeping only genuine semantic work.

## Rustdoc Convention

- No `# H1` inside `///` blocks — rustdoc already titles the item.
- `## H2` sections: `Examples`, `Returns`, `Errors`, `Panics`, `Safety`, `Notes`.
- Order: summary → `Examples` → `Returns` → `Errors` → `Panics` → `Safety` → `Notes`.

## Comment Quality

Structural rules above are silent on *content*. Prefer comments that carry information the code does not. See [`docs/comment-quality.md`](docs/comment-quality.md) for worked before/after examples.

Anti-patterns — remove on sight:

1. **HOW-narration** — prose that restates the implementation step-by-step.
2. **Tautological examples** — assertions guaranteed by the function signature.
3. **`## Arguments` / `## Returns` blocks that duplicate field docs** — use only when adding a constraint not expressed by the type.
4. **Format-string, color, or glyph narration** — quoting literal `format!` strings, ANSI colors, or emoji codepoints in prose. They drift.
5. **Redundant accessor docs** — one-line `///` on `fn foo() -> bool { self.foo }`. Prefer `#[allow(missing_docs)]` at impl level.
6. **Section-marker `//` comments** — `// Protocol` immediately above `extra.insert("protocol", …)`.
7. **Heavy-setup doc examples** — >20-line fixtures for one assertion. Link to a real test or omit.
8. **Stale comments past their code** — comments that no longer match what the code does. Worse than no comment.

Positive criteria — comments worth their length:

- **A. Contract or invariant** not derivable from types (e.g. `atomic_write`'s `last-rename-wins` semantics).
- **B. WHY a counter-intuitive choice was made** — at the surprising line, not in the surrounding docblock.
- **C. Semantics of complex return shapes** (e.g. `extract_frontmatter_text`'s `base_line`).
- **D. Hidden coupling or external surprise** (e.g. "serialize-compatible with X", "persisted to disk").
- **E. Link to authoritative design** at module (`//!`) / module-defining-type (`///`) level only. Per-function linking is a treadmill.

**Authoring discipline.** Any edit that changes a symbol's behavior must include a pass over its `///`/`//!` docs and inline `//` comments. Fix or delete drifted ones in the same change. Reviewers should flag behavior-changing PRs that do not touch the relevant comments.

**Scope discipline.** Comment-only cleanup commits must contain no behavior changes. If `git diff` of the commit shows non-comment line changes (rendering, format strings, constants, glyphs), split the behavior change into a separate commit before requesting review.

- When in doubt, ask: *would deleting this comment lose information a future reader needs?* If no, delete.
- when drift between comments and code is detected, always assume the code is correct and the comment is wrong (unless instructed otherwise); take appropriate actions and communicate that this drift was detected and how it was resolved

## Drift Maintenance

Update alongside code changes:

- READMEs when public behavior changes
- `docs/dependencies.md` (and per-area `docs/dependencies.md`) when crates are added/removed
- `.claude/skills/` when architecture or workflows change
- This file when workspace layout, commands, or repo-wide conventions change

## Authoritative Docs

- run `sniff repo packages` for the up to date list of package areas and packages
- Local skill catalog under `.claude/skills/` is the authoritative skill list.
- `.claude/skills/rust-testing/SKILL.md` — testing tier taxonomy, canonical `just` recipes, and `require_level!` usage.

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

## Features and Fixes

- each package area will have a `features` and `fixes` directory which contains specs
    - **features** tend to be larger in scope and can introduce new features
    - **fixes** are primarily focused on fixing existing functionality
- we have two lifecycle directories `_unscheduled` and `_completed` which can be found as subdirectories of features/fixes
    - features/fixes which have been identified but not scheduled (aka, lower urgency) will be found in `_unscheduled` with non-dated filename
    - features/fixes as direct subdirectories are "active" features/fixes and should always follow the format `YYYY-MM-DD-{name}`
        - the files in a feature/fix can vary but almost always will be the `spec.md` file
    - when a feature/fix is completed it is moved to `_completed`
