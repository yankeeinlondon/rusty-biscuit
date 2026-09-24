---
kind: spike
feature: 2026-09-22-consolidated-test-binaries-wave-2
spike: S4
created: 2026-09-23
rev: 208051f75
---

# S4 — Dependency census for the `test-toolkit` layout gate

## No cycle

The `test-toolkit` graph (`cargo tree -p test-toolkit -e normal,build`, 43
crates) contains **none** of the ten packages. Its only workspace dependency is
`biscuit-test-harness`. Its dev-dependencies are `serial_test` and `tempfile`.
Adding `test-toolkit` as a dev-dependency of any of the ten therefore creates
no cycle.

## Who already has it unconditionally

Every `l1` binary hosts the `test_layout` guard, so each package needs
`test-toolkit` in its **feature-less** dev graph.

| Package | Current declaration | Needed change |
|---|---|---|
| `claudine` | `[dev-dependencies]`, unconditional | none |
| `sniff` | `[dev-dependencies]`, unconditional | none |
| `sniff-cli` | `[dev-dependencies]`, unconditional | none |
| `dmls` | `[dev-dependencies]`, unconditional | none |
| `biscuit-tui-cli` | `[dev-dependencies]`, unconditional | none |
| `biscuit-file` | absent | add unconditional dev-dependency |
| `tree-hugger` | absent | add unconditional dev-dependency |
| `schematic-gen` | `[dependencies]`, `optional = true`, enabled by `terminal-tests` | **also** add unconditional dev-dependency |
| `biscuit-terminal-cli` | `[dependencies]`, `optional = true`, enabled by `terminal-tests` | **also** add unconditional dev-dependency |
| `claudine-gen` | `[dependencies]`, `optional = true`, enabled by `terminal-tests` | **also** add unconditional dev-dependency |

The last three rows are a finding the plan did not make. R7 named only
`biscuit-file` and `tree-hugger`. Under the feature-less `()` set,
`use test_toolkit::…` in their `l1` root would not resolve. The precedent is
`biscuit-terminal/lib/Cargo.toml` (wave 1): it keeps the optional regular
dependency for `terminal-tests` (`:74`) and adds an unconditional
dev-dependency (`:98`). Cargo allows one crate in both tables, and
`dep:test-toolkit` keeps working. R7 is amended accordingly.

## Crates added to each package's dev graph

The method was `cargo tree -p <pkg> -e normal,build,dev --prefix none`, with
no features, compared against the `test-toolkit` graph (`comm -23`).

| Package | Crates added (besides `test-toolkit` itself) |
|---|---|
| `biscuit-file` | `fs4`, `matchers`, `nu-ansi-term`, `sharded-slab`, `thread_local`, `tracing-log`, `tracing-subscriber` |
| `tree-hugger` | the `biscuit-file` list plus `lazy_static` |
| `schematic-gen` | the `biscuit-file` list plus `lazy_static` |
| `biscuit-terminal-cli` | `fs4` |
| `claudine-gen` | `fs4` |

Each of these crates is already in `Cargo.lock` once, at the version other
workspace packages build, so the change adds no new crate or version to the
workspace. The additions are dev-only, so no library or binary graph changes.

## Documentation obligations (R7)

- `docs/dependencies.md` lists `test-toolkit` (`:536`) and explains other
  dev-only uses (`:116`). Each package commit that adds the dev-dependency
  adds a line in that style, stating that it adds no crate to the workspace.
- Per-area `docs/dependencies.md`:
  - `schematic/docs/dependencies.md` already lists `test-toolkit` (`:123`).
    Note the new unconditional dev use.
  - `biscuit-file`, `biscuit-terminal`, and `claudine` each get a line in
    their area file if one exists at the time.
  - `tree-hugger` has no `docs/dependencies.md`. Record that in the package
    commit rather than creating the file for one line.
