---
blast_radius:
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/output/filesystem/repo.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/filesystem/repo/standard.rs
  - sniff/lib/src/filesystem/repo/topology.rs
---

# The `sniff repo structure` Subcommand

Detects and displays the structure of a repository, whether single-package or monorepo. Catalogs all packages, their languages, and interdependencies. Run it explicitly with `sniff repo structure`; the bare `sniff repo` command dispatches to [`sniff repo name`](./repo_name.md).

## Scope Resolution

- If the current directory is inside a **git repository**, the repo root is discovered automatically
- If **not** in a git repo, the current directory (or `--base`) is treated as the repo root
- All workspace tools and package managers are detected at the repo root
- Hidden directories (`.git`, `.cache`, etc.), `node_modules`, `target`, and `vendor` are skipped

## Default Behavior (Non-verbose)

### Single-Package Repository

For non-monorepos, a summary is shown:

```
Repository

  Type: Single-package
  Root: /absolute/path/to/repo
```

### Monorepo

For monorepos, the heading shows the unified per-standard label and package
count:

- `Prose::new(`<b><u>Repository</u></b> <dim>({authority_label} / {total_count} packages)</dim>`)`
- `Prose::new(`<b><u>Repository</u></b> <dim>({orchestrator_label} (using {authority_label}) / {total_count} packages)</dim>`)`

Both `{authority_label}` and `{orchestrator_label}` are drawn from each
standard's `spec().label` (for example, `cargo`, `pnpm workspaces`, `Nx`).

When filtering reduces the set:

- `Prose::new(`<b><u>Repository</u></b> <dim>({authority_label} / showing {shown} of {total} packages)</dim>`)`

### Package Listing

Packages are grouped by **package area** (top-level directory), displayed in hierarchical order:

```
Repository (cargo / 35 packages)

  sniff
    sniff-cli v0.1.0 (sniff/cli) [Rust]
    sniff v0.1.0 (sniff/lib) [Rust]
  biscuit-hash
    biscuit-hash-cli v0.1.0 (biscuit-hash/cli) [Rust]
    biscuit-hash v0.1.0 (biscuit-hash/lib) [Rust]
```

Each package line renders as:

- Prose::new(`<b>{name}</b> <dim>v{version}</dim> <dim>({relative_path})</dim> <dim>[{primary_language}]</dim>`)

Packages excluded from the workspace are shown in orange: `<orange>{name}</orange>`

When `--latest-versions` is active, packages with updatable dependencies append a `*` indicator:
- `<yellow>*</yellow>` — updates available
- `<red>*</red>` — major version update available

## Verbose Mode (`-v`)

Adding `-v` expands each package to show additional details:

```
sniff-cli v0.1.0 (sniff/cli) [Rust]
  depends on: sniff
  updates: 3 updates, 1 major
```

### Double-verbose (`-vv`)

At verbosity level 2+:

```
sniff-cli v0.1.0 (sniff/cli) [Rust]
  depends on: sniff
  used by: homelab-cli
  langs: Rust, TOML
  frameworks: clap, tokio
  updates: 3 updates, 1 major
```

## Package Filtering

A positional `filter` argument narrows the output:

```
sniff repo [filter...]
```

| Pattern | Behavior |
|---------|----------|
| `name` | Includes packages whose name contains the substring |
| `@area` | Includes all packages in the named area |
| `!name` | Excludes packages matching the substring |

- **OR logic**: multiple filters include a package if any match
- Matching is case-insensitive
- Example: `sniff repo @sniff biscuit` shows packages in the sniff area OR whose name contains "biscuit"

## Scoping to a Package or Package Area

Two flags layer on top of the positional filter:

| Flag | Match Semantics |
|------|-----------------|
| `-p/--package <PKG>` | Exact (case-insensitive) match on `Package.name` |
| `--package-area <AREA>` | Case-insensitive prefix match on `Package.package_area` (so `--package-area homelab` matches `homelab` and `homelab/server`) |

```bash
sniff repo structure -p sniff-cli                      # Only the sniff-cli package
sniff repo structure --package-area homelab            # All homelab/* packages
sniff repo structure -p sniff-cli --package-area sniff # Intersection
```

Passing both flags requires the resolved package to live inside the resolved area; otherwise the command fails with an error like `error: Package 'sniff-lib' is in area 'sniff', not 'homelab'`. Unknown values for either flag fail with an error listing the valid names.

## The `--latest-versions` Flag

Queries package registries (crates.io, npm, PyPI, etc.) for each dependency to check for updates. Version checks run in parallel with bounded concurrency.

When enabled:
- `is_updatable` and `has_major_update` are populated for each dependency
- In verbose mode, per-package update counts are shown
- At `-vv`, individual package transitions are listed: `some-dep (1.0 → 2.0)`

A legend is shown when update indicators are present:
```
* dependency updates available  * major version update  packages in orange are excluded
```

## JSON Output (`--json`)

```
sniff --json repo [filter...]
```

Returns a `RepoInfo` object:

```json
{
  "is_monorepo": true,
  "root": "/absolute/path/to/repo",
  "monorepo_standards": [
    {
      "standard": "cargo-workspace",
      "root": "/absolute/path/to/repo",
      "matched_markers": ["Cargo.toml"],
      "binary": {
        "name": "cargo",
        "path": "/usr/bin/cargo",
        "version": "1.80.0",
        "source": "path"
      },
      "confidence": "marker-confirmed"
    }
  ],
  "monorepo_layers": [
    {
      "root": "/absolute/path/to/repo",
      "authority": "cargo-workspace",
      "orchestrators": [],
      "provenance": "globbed",
      "packages": ["sniff/lib", "sniff/cli"]
    }
  ],
  "packages": [
    {
      "path": "/absolute/path/to/sniff/cli",
      "relative": "sniff/cli",
      "package_area": "sniff",
      "name": "sniff-cli",
      "ecosystem": "cargo",
      "standard": "cargo-workspace",
      "provenance": "globbed",
      "primary_language": "Rust",
      "version": "0.1.0",
      "depends_on": ["sniff"],
      "used_by": [],
      "is_updatable": false,
      "has_major_update": false,
      "is_excluded": false,
      "dependencies": [
        {
          "name": "clap",
          "actual_version": "4.4.0",
          "latest_version": null,
          "is_updatable": false,
          "has_major_update": false
        }
      ]
    }
  ]
}
```

The legacy keys `monorepo_tool`, `workspace_tools`, and `discovery_sources` are no longer emitted.

## Plain Output (`--plain`)

Adding `--plain` strips all ANSI escape codes from text output.
