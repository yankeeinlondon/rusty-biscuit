# Phase 1 Baseline

Captured on 2026-07-10 before deleting the dead transform module.

## Worktree

Pre-existing changes preserved and left untouched:

- `D darkmatter/fixes/2026-07-10-error-not-available/spec.md`
- `?? .claude/settings.local.json`

The specification, render-tree parity tests, and schema resolver had no worktree changes.

## Test inventories

Inventories are the lexicographically sorted output of the listed command. The SHA-256 digest
allows later phases to compare every fully qualified test name without storing a large duplicate
listing in the repository.

| Inventory | Command | Count | SHA-256 |
| --- | --- | ---: | --- |
| Darkmatter | `cargo nextest list -p darkmatter --color never` | 5,368 | `f45a48ee0cfeab871f249f0fff4927bb6efff95a2d77247176df3949e413e9ed` |
| Level 2 render tree | `cargo nextest list -p darkmatter --test level2_render_tree_terminal --color never` | 20 | `e28f6b45cbdc5e3614639e22ec5bda7e208fd8e212fa255ef8bcbff597cac243` |

## Validation

- `cargo check -p darkmatter`: passed
- `just test`: started with no pre-existing failures observed; the complete post-change run is
  recorded below
- `just lint`: passed for `darkmatter`, `darkmatter-cli`, and `dmls`

## Retired API inventory

Before deletion, compiled-source matches for `markdown::transform`, `TransformReport`,
`TransformOptions`, `TransformContext`, `.transform()`, and `.transform_with()` existed only in
the uncompiled `darkmatter/lib/src/markdown/transform/` tree. Active documentation matches existed
in:

- `darkmatter/docs/topics/darkmatter-expressions.md`
- `darkmatter/docs/inline/text-replacement.md`

Matches in `darkmatter/features/_completed/` are historical records and are intentionally
unchanged.

## Post-change validation

- `cargo check -p darkmatter`: passed
- `just test`, run as four deterministic nextest count partitions to keep each non-interactive
  subprocess bounded: all Darkmatter, Darkmatter CLI, and DMLS Level-1 tests passed
- `just lint`: passed for `darkmatter`, `darkmatter-cli`, and `dmls`
- Post-change Darkmatter inventory: 5,368 tests with SHA-256
  `f45a48ee0cfeab871f249f0fff4927bb6efff95a2d77247176df3949e413e9ed`, exactly matching the
  baseline
