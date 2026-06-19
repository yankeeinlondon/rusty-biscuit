# Spec: Consistent `--package` and `--package-area` Flags Across `sniff repo`

## Problem

Several `sniff repo` subcommands use a single `--package` flag that ambiguously matches **either** a package name **or** a package area. This is confusing and inconsistent with commands that already offer separate `--package` and `--package-area` flags.

Additionally, many commands rely solely on positional `filter` args (which support `@area` syntax and negation) and lack the explicit named flags entirely, making discovery harder for users.

## Goal

Every `sniff repo` subcommand that scopes or filters by package or package area must offer **both** `--package <PKG>` and `--package-area <AREA>` as explicit named flags. The two flags must be orthogonal: `--package` resolves to a single crate/package name, and `--package-area` resolves to a single package area (directory). When both are provided, they intersect (AND logic).

Commands where package scoping is semantically meaningless (e.g., `repo hash`, `repo remote`, `repo root`) are excluded.

## Affected Commands

### Tier 1: Combined `--package` behavior (primary targets)

These commands have a single `--package` flag that falls back from package name to package area. This fallback behavior will be **removed**.

| Command | Current Flag | Needs Added |
|---------|--------------|-------------|
| `repo git-status` | `--package` (combined) | `--package-area` |
| `repo unstaged-files` | `--package` (combined) | `--package-area` |
| `repo untracked-files` | `--package` (combined) | `--package-area` |

### Tier 2: Positional `filter` only

These commands have no named `--package` or `--package-area` flags. The positional `filter` arg supports substring matching, `@area` syntax, and `!` negation. The positional filter **must be retained** for backward compatibility and advanced use cases. The new named flags are added as additional, exact-match constraints.

| Command | Current Filter | Needs Added |
|---------|----------------|-------------|
| `repo structure` | positional `filter` | `--package`, `--package-area` |
| `repo package-dependencies` | positional `filter` | `--package`, `--package-area` |
| `repo dirty-packages` | positional `filter` | `--package`, `--package-area` |
| `repo staged-packages` | positional `filter` | `--package`, `--package-area` |
| `repo unstaged-packages` | positional `filter` | `--package`, `--package-area` |
| `repo dirty-package-areas` | positional `filter` | `--package`, `--package-area` |
| `repo staged-package-areas` | positional `filter` | `--package`, `--package-area` |
| `repo unstaged-package-areas` | positional `filter` | `--package`, `--package-area` |

### Tier 3: Missing `--package`

These commands already have `--package-area` and positional `filter`, but lack `--package`.

| Command | Current Flags | Needs Added |
|---------|---------------|-------------|
| `repo packages` | `--package-area`, positional `filter` | `--package` |
| `repo package-areas` | `--package-area`, positional `filter` | `--package` |

### Already consistent (minor update for `-p`)

| Command | Flags |
|---------|-------|
| `repo staged-files` | `--package`, `--package-area` (via `FileListArgs`) |
| `repo dirty-files` | `--package`, `--package-area` (via `FileListArgs`) |
| `repo dirty-source-code` | `--package`, `--package-area` (via `FileListArgs`) |
| `repo staged-source-code` | `--package`, `--package-area` (via `FileListArgs`) |
| `repo unstaged-source-code` | `--package`, `--package-area` (via `FileListArgs`) |
| `repo recent-commits` | `--package`, `--package-area` |
| `repo source-code-changes` | `--package`, `--package-area` |
| `repo documentation-changes` | `--package`, `--package-area` |
| `repo blast-radius` | `--package`, `--package-area` |

> **Note:** The five `FileListArgs`-based commands (`staged-files`, `dirty-files`, `dirty-source-code`, `staged-source-code`, `unstaged-source-code`) already expose both `--package` and `--package-area`, but their `--package` field lacks the `short` attribute. Adding `short` to the `package` field in `FileListArgs` will enable the `-p` short flag for these commands. The remaining four commands (`recent-commits`, `source-code-changes`, `documentation-changes`, `blast-radius`) truly need no changes.

### Excluded (package scoping is meaningless)

`repo hash`, `repo remote`, `repo package`, `repo package-area`, `repo package-root`, `repo package-area-root`, `repo root`, `repo is-current-package-area-dirty`, `repo package-area-has-source-code-changes`, `repo has-merge-conflict`, `repo pr`, `repo language`, `repo worktree`.

## Semantics

### `--package <PKG>`

- **Exact case-insensitive match** on `Package.name`. No substring, no prefix — the provided value must equal a known package name.
- Tab-completion powered by `repo_package_candidates()`.
- Unknown name → hard error listing valid package names.

### `--package-area <AREA>`

- **Case-insensitive prefix match** on `Package.package_area`. This means `--package-area homelab` matches packages in `homelab` **and** in `homelab/server`, `homelab/integrations`, etc. This preserves the hierarchical directory-tree semantics of package areas.
- Tab-completion powered by `repo_package_area_candidates()`.
- Unknown prefix → hard error listing valid package areas.

> **Note on the asymmetry:** `--package` is an exact match because package names are unique identifiers. `--package-area` is a prefix match because areas are hierarchical directory paths — users expect `homelab` to also cover `homelab/server`. This mirrors the existing behavior in `blast_radius.rs` and `collect_changed_paths`, which already use `starts_with` for area-based filtering.

### Interaction between `--package` and `--package-area`

When **both** are provided, the result is the **intersection** (AND logic) of the two scopes:

- For file-path commands (`git-status`, `*files`): the path must be under **both** the package directory and the package area directory. In practice, if `--package` is a child of `--package-area`, the package path wins (it is the narrower prefix). The implementation should compute both path prefixes and use the more restrictive one, or simply apply both `starts_with` checks.
- For commit commands (`recent-commits` etc.): already applies both `filter_by_package` and `filter_by_package_area` sequentially — keep this pattern.
- For package-list commands (`dirty-packages`, `packages`, etc.): the package must match `--package` AND belong to `--package-area`.

**Non-overlapping scopes produce a hard error.** If both flags are provided but the resolved package does **not** live under the specified package area, the CLI must exit with a hard error rather than silently returning empty results. Example:

```
error: Package 'sniff-lib' is in area 'sniff', not 'homelab'
```

This is consistent with the hard error strategy for unknown names and prevents user confusion from a typo that would otherwise produce a silent empty result.

### Interaction with positional `filter`

Positional `filter` args are **retained** and act as an additional constraint. The final result is the intersection of:
1. Positional `filter` (if any)
2. `--package` (if any)
3. `--package-area` (if any)

## Implementation Plan

### 1. Refactor `resolve_package_path`

The current `resolve_package_path` in `sniff/cli/src/commands/mod.rs` tries name first, then falls back to area. This fallback must be eliminated.

Introduce two explicit helpers:

```rust
/// Resolve a package name to its relative directory path.
fn resolve_package_path(result: &SniffResult, name: &str) -> Result<String, Box<dyn std::error::Error>>;

/// Resolve a package area name to its relative directory path.
fn resolve_package_area_path(result: &SniffResult, area: &str) -> Result<String, Box<dyn std::error::Error>>;
```

Both should return the path prefix with a trailing slash (e.g., `sniff/cli/` or `sniff/`).

### 2. Update CLI argument definitions

In `sniff/cli/src/args/repo.rs`, update the `RepoSubcommand` enum:

- **Tier 1**: Replace the combined `package: Option<String>` with both `package: Option<String>` and `package_area: Option<String>`. Add `short` to the `package` field to preserve the `-p` short flag.
- **Tier 2 & 3**: Add both fields to the respective variant structs. Add `short` to the `package` field to enable the `-p` short flag.

Use the existing completion candidate functions:

```rust
#[arg(short, long, value_name = "PKG", add = clap_complete::engine::ArgValueCandidates::new(repo_package_candidates))]
package: Option<String>,

#[arg(long, value_name = "AREA", add = clap_complete::engine::ArgValueCandidates::new(repo_package_area_candidates))]
package_area: Option<String>,
```

Update `RepoAction` variants to carry both fields where applicable.

### 3. Update command dispatch

In `sniff/cli/src/commands/mod.rs`:

- **Tier 1 (`git-status`)**: Replace the single `package_for_git` resolution with separate handling for `package` and `package_area`. Compute both path prefixes. Apply the narrower prefix (or both checks) to:
  - `get_commits_for_path`
  - `file_changes` retain
  - `status.dirty` retain
  - `status.untracked` retain
  - Recompute counts.

- **Tier 1 (`unstaged-files`, `untracked-files`)**: These delegate to `handle_file_list_command`. The `FileListArgs` struct already has both fields, but the CLI variants currently construct `FileListArgs` with `package_area: None`. Update the conversion to pass through both values.

- **Tier 2 & 3**: Update the respective handler functions in `sniff/cli/src/commands/repo.rs` and output modules to accept and apply the new scoping fields.

### 4. Update output/filter modules

- `sniff/cli/src/output/filesystem/packages.rs` — add package/area scoping to `DirtyPackages`, `StagedPackages`, `UnstagedPackages`.
- `sniff/cli/src/output/filesystem/package_areas.rs` — add package/area scoping to `DirtyPackageAreas`, `StagedPackageAreas`, `UnstagedPackageAreas`.
- `sniff/cli/src/output/filesystem/repo.rs` — add package/area scoping to `Structure`.
- `sniff/cli/src/output/filesystem/deps.rs` — add package/area scoping to `Deps`.
- `sniff/cli/src/output/repo_json.rs` — ensure JSON output respects the new flags.

### 5. Update documentation

Update the following docs under `sniff/docs/cli/`:

- `repo_git-status.md`
- `repo_unstaged-files.md` (if it exists; create if not)
- `repo_untracked-files.md` (if it exists; create if not)
- Any other affected command docs to document the new flags.

### 6. Add/update tests

Add CLI integration tests in `sniff/cli/tests/cli.rs` covering:

- `--package` filters to exactly one package
- `--package-area` filters by prefix (e.g., `--package-area homelab` matches `homelab`, `homelab/server`, `homelab/integrations`)
- Both flags together intersect correctly
- Both flags together with non-overlapping scopes produces a hard error (e.g., `--package sniff-lib --package-area homelab`)
- Unknown `--package` or `--package-area` produces a hard error with valid options listed
- Positional `filter` still works and intersects with named flags

## Performance Requirements

1. **No additional git history walks**. Commands that already walk commits (`git-status`) must not walk more than they do today. Path-based filtering happens **after** (or during) the existing walk, not in a second pass.

2. **No duplicate diff generation**. `git-status` currently calls `get_commits_for_path` once. With both `--package` and `--package-area`, derive the single most restrictive path prefix and call `get_commits_for_path` once. Do not call it twice.

3. **O(1) per-file filtering**. All file-path retains (`file_changes`, `dirty`, `untracked`) use `starts_with` on existing collections. This must remain O(n) where n = number of files already collected.

4. **Lazy resolution**. Package/area resolution (the linear scan over the package list) must happen **after** detection results are available and only when the flags are actually provided. The resolution cost is negligible (~48 workspace members).

5. **No regression for unscoped usage**. When neither flag is provided, the code path must be identical to today — no extra allocations, no extra filtering.

## Backward Compatibility

- **Positional `filter` args are preserved** on all Tier 2 & 3 commands. Existing scripts using `sniff repo dirty-packages sniff` continue to work.
- **The combined fallback behavior is intentionally removed** from Tier 1 commands. This is a breaking change for users who passed a package area name to `--package` on `git-status`, `unstaged-files`, or `untracked-files`. Those users must switch to `--package-area`. This breakage is acceptable because the current behavior is ambiguous and documented as such.
- **The short flag `-p` is being added to `--package` on all commands.** This is a non-breaking addition. Tier 1 commands (`git-status`) already had `-p` on their `--package` flag — no change. FileListArgs-based commands (`staged-files`, `dirty-files`, etc.) gain `-p` by adding `short` to the existing `--package` field. Tier 2 & 3 commands get `-p` on their new `--package` fields. `--package-area` does **not** receive a short flag.

## Files to Modify

| File | Change |
|------|--------|
| `sniff/cli/src/args/repo.rs` | Add `--package` / `--package-area` to affected `RepoSubcommand` variants; update `RepoAction` |
| `sniff/cli/src/args/mod.rs` | Add `short` to `--package` field in `FileListArgs` to enable `-p` short flag |
| `sniff/cli/src/commands/mod.rs` | Refactor `resolve_package_path`; update `git-status` dispatch; update `UnstagedFiles`/`UntrackedFiles` dispatch |
| `sniff/cli/src/commands/repo.rs` | Update `handle_file_list_command` and package/area handlers |
| `sniff/cli/src/output/filesystem/packages.rs` | Apply package/area scoping |
| `sniff/cli/src/output/filesystem/package_areas.rs` | Apply package/area scoping |
| `sniff/cli/src/output/filesystem/repo.rs` | Apply package/area scoping to `Structure` |
| `sniff/cli/src/output/filesystem/deps.rs` | Apply package/area scoping to `Deps` |
| `sniff/cli/src/output/repo_json.rs` | Respect new flags in JSON output |
| `sniff/docs/cli/repo_git-status.md` | Document separate flags |
| `sniff/cli/tests/cli.rs` | Add integration tests |
