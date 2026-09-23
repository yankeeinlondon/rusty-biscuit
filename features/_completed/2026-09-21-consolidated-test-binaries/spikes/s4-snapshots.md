---
kind: spike-record
feature: 2026-09-21-consolidated-test-binaries
created: 2026-09-21
plan_phase: 1
status: complete
insta_resolved: insta 1.47.2 (workspace Cargo.lock)
---

# S4 — Snapshot mapping probe

## Question

When a test file becomes a module of a consolidated binary, which Insta
snapshots move, what is the mechanical old→new rule, and does a
byte-preserving move pass under `INSTA_UPDATE=no`?

## Method

This spike used a scratch crate (`s4-probe`, system temp directory, `insta
= "=1.47.2"`, which is the workspace's locked version). It mirrors claudine's
`wrap_basics.rs` shape: one top-level `assert_snapshot!` plus one inside an
inline `mod nested`.

1. The old layout was `tests/wrap_basics.rs` as its own target.
   `INSTA_UPDATE=always` wrote
   `tests/snapshots/wrap_basics__help_lists_wrapper_subcommands.snap` and
   `tests/snapshots/wrap_basics__nested__inner_case.snap`.
2. The file moved to `tests/l1/wrap_basics.rs` under
   `tests/l1/main.rs` (`mod wrap_basics;`), with `autotests = false` and
   `[[test]] name = "l1"`. Without mapping the snapshots, `INSTA_UPDATE=no`
   **fails** both tests. Insta names the file it expected:
   `tests/l1/snapshots/l1__wrap_basics__help_lists_wrapper_subcommands.snap`
   and `tests/l1/snapshots/l1__wrap_basics__nested__inner_case.snap`. No
   `.snap.new` was written. A missed mapping is therefore a hard failure, as
   spec §5 requires.
3. The snapshots were copied byte-for-byte to the new names (identical SHA-1
   per pair) and the old directory was removed. `INSTA_UPDATE=no` then passes
   both tests, and so does `CI=true` (Insta's CI default). `.snap.new` count: 0.
4. With `INSTA_REQUIRE_FULL_MATCH=1`, the moved snapshots **fail**, because
   the header's `source: tests/wrap_basics.rs` metadata no longer matches
   the new source path. No recipe, workflow, or `insta.yaml` in the
   repository sets `INSTA_REQUIRE_FULL_MATCH`. Only `INSTA_WORKSPACE_ROOT` is
   set, by the archive/WSL legs in `_package-ci.yml` and `_wsl-ci.yml`. So
   byte-identical moves pass every canonical path. The stale `source:` header
   is metadata only and stays unchanged. Rewriting it would change
   snapshot bytes, which acceptance 6 forbids.

## The mapping rule

Insta 1.47 derives both parts of the path from the assertion's call site:

- **directory** = `<dir of the assertion's source file>/snapshots/`
- **file name** = `module_path!()` with `::` → `__`, including the crate
  (test-target) name, then `__<snapshot name>.snap`

So for an assertion whose source file moves from `tests/<f>.rs` to
`tests/<target>/<module>.rs`:

```text
old: tests/snapshots/<f>__<inner path>__<name>.snap
new: tests/<target>/snapshots/<target>__<module>__<inner path>__<name>.snap
```

`<module>` is the module name the migration manifest assigns (the former
target name, or its recorded neutral alias). A nested crate root that
becomes a directory module follows the same rule. Darkmatter's
`tests/error_snapshots/main.rs` + `condition.rs` becomes
`tests/<target>/error_snapshots/mod.rs` + `condition.rs`, so

```text
old: tests/error_snapshots/snapshots/error_snapshots__condition__eval.snap
new: tests/<target>/error_snapshots/snapshots/<target>__error_snapshots__condition__eval.snap
```

The repository sets no `Settings` override (`set_snapshot_path`,
`prepend_module_to_snapshot`, `snapshot_suffix`, `with_settings!`) in any of
the four test trees. That was checked with `git grep` at this revision, so the
default rule is the only rule. `check-snapshots` must re-verify that
absence at migration time, because one override would invalidate the rule for
its module.

## Per-package mapping-table shape

| Package | Tracked `.snap` files under `tests/` | Snapshot directories | Asserting source files (S2 scan) | Rows in the mapping table |
|---|---:|---|---:|---:|
| `claudine-cli` (pilot) | 3 | `tests/snapshots/` | 1 (`wrap_basics.rs`) | 3 |
| `darkmatter` | 200 | `tests/snapshots/` (129), `tests/error_snapshots/snapshots/` (71) | 19 | 200 |
| `darkmatter-cli` | 0 | — | 0 | 0 (proof of no snapshots) |
| `biscuit-terminal` | 1,086 | `tests/snapshots/` | 2 (`inline_content_matrix.rs`, `layout_matrix.rs`) | 1,086 |

Before any migration, no `.snap.new` file exists in any of the four trees.

The pilot's projected table follows. `wrap_basics` carries no tier marker,
so it keeps its name as a module (R2) inside `l1`:

| Old | New |
|---|---|
| `claudine/cli/tests/snapshots/wrap_basics__help_lists_wrapper_subcommands.snap` | `claudine/cli/tests/l1/snapshots/l1__wrap_basics__help_lists_wrapper_subcommands.snap` |
| `claudine/cli/tests/snapshots/wrap_basics__wrapper_help_includes_expected_flags.snap` | `claudine/cli/tests/l1/snapshots/l1__wrap_basics__wrapper_help_includes_expected_flags.snap` |
| `claudine/cli/tests/snapshots/wrap_basics__wrapper_reports_removed_sensitive_env_names.snap` | `claudine/cli/tests/l1/snapshots/l1__wrap_basics__wrapper_reports_removed_sensitive_env_names.snap` |

## Consequences for `check-snapshots` (Phase 2)

A table row is `{old path, new path, sha256}`. The checker must:

1. Derive every **expected** new path from the manifest (source move + module
   name) using the rule above. A hand-written row that disagrees with the
   derived path fails.
2. Require every tracked `.snap` under a migrated `tests/` tree to be either
   mapped or proven unaffected. Unaffected means its asserting source did
   not move, so its path is unchanged.
3. Compare byte hashes old→new, and fail on any `.snap.new` file or on
   `INSTA_REQUIRE_FULL_MATCH` appearing in a recipe.
4. Map by **module path**, not by file stem. Biscuit-terminal's 1,086
   `layout_matrix__*` snapshots come from one loop assertion in
   `layout_matrix.rs` (`layout_matrix_snapshots`, which is `#[ignore]`d today:
   FIXME table-width) that uses runtime-built snapshot names. They cannot be
   derived from the list of test functions. The Phase 6 table must be
   generated by applying the rule to the tracked files, never enumerated by
   hand. Because the test is ignored, `INSTA_UPDATE=no` runs do **not** read
   these files. Their mapping is proven by the rule plus byte hashes, not by a
   passing run. Phase 6 must say so rather than claim run coverage.

The spike did not verify the pair inside the real `claudine-cli` package,
because that would be a production-tree change in a phase that permits none.
The scratch crate reproduces the exact mechanism with the locked Insta
version. The real pair is verified by the Phase 3 "Snapshot moves" task
under `INSTA_UPDATE=no`.
