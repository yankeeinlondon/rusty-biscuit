# Better Package Detection

## Overview

`sniff` currently detects monorepos by picking the first matching workspace tool and returning immediately.

That works for single-ecosystem repositories, but it breaks down in mixed monorepos that contain nested packages from different ecosystems. The `homelab/server` case is a concrete example:

- [`homelab/server`](/Volumes/coding/personal/rusty-biscuit/homelab/server) is a Rust package defined by [`homelab/server/Cargo.toml`](/Volumes/coding/personal/rusty-biscuit/homelab/server/Cargo.toml)
- [`homelab/server/frontend`](/Volumes/coding/personal/rusty-biscuit/homelab/server/frontend) is a separate frontend package defined by [`homelab/server/frontend/package.json`](/Volumes/coding/personal/rusty-biscuit/homelab/server/frontend/package.json)
- the frontend package is already declared in the repository pnpm workspace via [`pnpm-workspace.yaml`](/Volumes/coding/personal/rusty-biscuit/pnpm-workspace.yaml)

Today, `sniff language` run from `homelab/server` counts the nested frontend files as if they belonged to the Rust package, causing TypeScript/Vue to dominate the language report.

This document proposes a package-detection model that works correctly for mixed-language, mixed-workspace repositories.

## Problem Statement

The current logic in [`sniff/lib/src/filesystem/repo.rs`](/Volumes/coding/personal/rusty-biscuit/sniff/lib/src/filesystem/repo.rs) has two core limitations:

1. `detect_repo()` returns the first detected monorepo tool instead of combining package evidence across tools.
2. Package language detection scans the package directory recursively without excluding nested package roots.

That creates two classes of bad results:

- mixed-workspace repos under-report packages
- parent package language scans absorb child package languages

In `homelab/server`, the second problem is the visible symptom, but the first problem is the reason `frontend` was not available as a separate package boundary during analysis.

## Goals

- Detect packages across multiple workspace ecosystems in the same repository.
- Treat nested package roots as package boundaries.
- Exclude nested packages from a parent package’s language scan by default.
- Preserve support for current Cargo, npm, pnpm, Yarn, Nx, Turborepo, and Lerna detection.
- Make primary-language detection for a package reflect that package’s own code, not code from embedded subpackages.
- Keep the model explainable and deterministic.

## Non-Goals

- This document does not redesign the file-type taxonomy from the separate file-association design.
- This document does not require Cargo workspaces to enumerate non-Cargo packages.
- This document does not require cross-tool dependency graph resolution between Rust and JS packages in the first phase.

## Design Principles

### Package Boundaries Are Stronger Than Directory Ownership

If a directory contains its own package manifest, it should normally be treated as a distinct package boundary even if it is physically nested under another package directory.

### Workspace Discovery Should Be Additive

Different workspace tools should contribute package roots to a shared package graph rather than competing in a winner-takes-all order.

### Source-Root Hints Are Secondary

Manifest-declared source roots such as Cargo `lib.path` and `bin.path` are useful hints for weighting and explainability, but they should not be the primary fix for nested package contamination.

## Current Example

### Repository Facts

- [`homelab/server/Cargo.toml`](/Volumes/coding/personal/rusty-biscuit/homelab/server/Cargo.toml) defines package `homelab-server`
- its Rust entrypoints are:
  - [`homelab/server/src/lib.rs`](/Volumes/coding/personal/rusty-biscuit/homelab/server/src/lib.rs)
  - [`homelab/server/src/main.rs`](/Volumes/coding/personal/rusty-biscuit/homelab/server/src/main.rs)
- [`homelab/server/frontend/package.json`](/Volumes/coding/personal/rusty-biscuit/homelab/server/frontend/package.json) defines package `homelab-frontend`
- [`pnpm-workspace.yaml`](/Volumes/coding/personal/rusty-biscuit/pnpm-workspace.yaml) explicitly includes `homelab/server/frontend`

### What Should Happen

`sniff` should detect two distinct packages:

- `homelab-server`
- `homelab-frontend`

When analyzing `homelab-server`, the language scan should exclude the nested `frontend` package root. That alone should make Rust the dominant language for the server package.

## Proposed Model

### Package Discovery Becomes Multi-Source

Instead of returning the first `RepoInfo`, the detection pipeline should collect package candidates from every relevant source:

- Cargo workspace members
- pnpm workspace packages
- npm workspace packages
- Yarn workspace packages
- Nx/Turborepo/Lerna discovered packages
- standalone manifest discovery for known manifest types

The result should be a unified package set with per-package metadata describing how the package was discovered.

### New Concepts

Suggested internal model:

```rust
pub enum PackageEcosystem {
    Cargo,
    Node,
    Python,
    Go,
    Unknown,
}

pub enum PackageDiscoverySource {
    CargoWorkspace,
    PnpmWorkspace,
    NpmWorkspace,
    YarnWorkspace,
    Nx,
    Turborepo,
    Lerna,
    ManifestScan,
}

pub struct PackageBoundary {
    pub root: PathBuf,
    pub ecosystem: PackageEcosystem,
    pub discovery_sources: Vec<PackageDiscoverySource>,
}
```

The current `Package` struct can remain the main serialized type, but internally it should be built from a richer set of boundary metadata.

## Detection Strategy

### Phase 1: Collect Workspace-Declared Package Roots

Every workspace detector should contribute package roots rather than returning a full and final `RepoInfo`.

Suggested refactor:

- `detect_cargo_workspace()` becomes a collector of Cargo package boundaries
- `detect_pnpm_workspace()` becomes a collector of Node package boundaries
- same for npm, Yarn, Nx, Turborepo, and Lerna

Then a top-level coordinator merges all candidates.

### Phase 2: Add Manifest-Based Nested Package Discovery

Even if a package was not declared by a workspace tool, nested manifests should still be visible.

Relevant manifests:

- `Cargo.toml`
- `package.json`
- `pyproject.toml`
- `go.mod`

This is especially important for:

- embedded frontends
- integration helpers
- local tools
- examples and demos that are packaged independently

Manifest discovery should be constrained by ignore rules and by a list of excluded directories like `node_modules`, `target`, `dist`, and `build`.

### Phase 3: Merge and Deduplicate

Package candidates should be merged by canonical root path.

If the same package root is discovered by multiple systems:

- keep one package
- retain all discovery sources
- infer the ecosystem from the manifest types present

Example:

- `homelab/server/frontend` might be discovered by `PnpmWorkspace` and by `ManifestScan`
- that should still produce one package

## Nested Package Boundary Rules

### Default Rule

If package `B` is nested inside package `A`, then `B` is excluded from `A`’s recursive language and file scans by default.

This is the key fix for the `homelab/server` case.

### Rationale

Nested packages are almost always independently buildable/deployable units. Treating their files as part of the parent package creates misleading package metrics.

### Boundary Computation

For each package:

1. identify all other package roots that are descendants of this package root
2. sort nested roots by path length
3. exclude those subtrees from recursive scans

Suggested helper:

```rust
pub struct ScanBoundary {
    pub include_root: PathBuf,
    pub exclude_roots: Vec<PathBuf>,
}
```

The recursive scanner should accept `exclude_roots` and skip traversal into those directories.

## Should Cargo Entrypoints Limit the Scan?

### Recommended Answer

No, not as the primary rule.

Cargo `lib.path` and `bin.path` are useful metadata, but they are too narrow to define package ownership on their own.

If `sniff` only scanned Cargo entrypoint directories such as `src/`, it would miss valid package-owned content like:

- `tests/`
- `examples/`
- `benches/`
- `build.rs`
- schema files
- migrations
- fixture data
- docs bundled with the package

### Better Use of Cargo Source Metadata

Cargo source paths should be used for:

- explainability in output
- weighting for primary-language determination
- detecting source roots for focused summaries

But package boundaries should still be determined by package manifests, not just by source directories.

## Language Detection Rules

### Package-Level Language Scan

When `sniff language` is run in a package directory:

1. determine the containing package boundary
2. determine nested package roots inside that package
3. scan recursively within the package root
4. skip nested package roots
5. aggregate language stats from only the remaining files

For `homelab/server`, that means:

- include `src/`, `tests/`, `README.md`, and package-owned files
- exclude `frontend/`

### Optional Source-Root Weighting

Once nested package exclusion is implemented, source-root hints can further improve results.

Examples:

- Rust files under `src/`, `tests/`, `benches/`, and `examples/` can receive slightly higher weighting than unrelated top-level files
- Cargo `lib.path` and `bin.path` can be included in an explanation section

This should be a refinement, not the fix that package detection depends on.

## CLI Behavior

### `sniff language`

For package-local invocation, the command should analyze the current package boundary, not just the raw current working directory tree.

If the current directory is inside a known package:

- use the package root as the scan root
- exclude nested package roots by default

If the current directory is not inside a package:

- fall back to the current directory scan behavior

### `sniff repo`

`sniff repo` should list both parent and nested packages when they are valid package roots.

For the current example, the repo/package listing should include:

- [`homelab/server`](/Volumes/coding/personal/rusty-biscuit/homelab/server)
- [`homelab/server/frontend`](/Volumes/coding/personal/rusty-biscuit/homelab/server/frontend)

### Possible Future Flags

The default should be boundary-aware exclusion. If needed later, a flag can expose raw recursive behavior:

```txt
sniff language --include-nested-packages
```

That should be opt-in, not the default.

## RepoInfo and Package Model Changes

### Keep Monorepo Tool, Add Mixed Discovery Metadata

The current `RepoInfo.monorepo_tool` is too singular for mixed ecosystems. The root repo may still have a dominant tool, but package discovery should carry more detail.

Suggested additions:

```rust
pub struct Package {
    pub path: PathBuf,
    pub name: String,
    pub ecosystem: PackageEcosystem,
    pub discovery_sources: Vec<PackageDiscoverySource>,
    pub nested_packages: Vec<String>,
    // existing fields...
}
```

Potential `RepoInfo` addition:

```rust
pub struct RepoInfo {
    pub workspace_tools: Vec<MonorepoTool>,
    // existing fields...
}
```

The old singular `monorepo_tool` can be kept temporarily for compatibility, but the internal model should stop assuming only one tool was relevant.

## Implementation Plan

### Phase 1: Refactor Detection Into Collectors

- change workspace detectors to return package boundary candidates
- add a coordinator that merges package roots from all detectors
- preserve existing output shape initially

### Phase 2: Add Nested Boundary Exclusion to Recursive Scans

- update language scanning to accept excluded subtrees
- exclude nested package roots from parent package scans
- update package-level creation paths to use boundary-aware scans

### Phase 3: Improve Package Attribution in CLI

- show ecosystem and detection source in verbose package output
- ensure `sniff repo` surfaces nested packages clearly
- ensure `sniff language` from inside a package uses the package boundary

### Phase 4: Add Source-Root Hints

- parse Cargo `lib.path` and `bin.path`
- optionally parse other ecosystem source-root hints later
- use these hints for weighting and explanation, not hard package boundaries

## Testing Strategy

### Mixed Workspace Tests

Add tests for a repository that contains:

- a Cargo workspace package
- a nested pnpm package inside one Cargo package

Expected result:

- both packages are discovered
- the nested package is not counted in the parent package’s language breakdown

### Parent/Child Boundary Tests

Create a fixture:

- parent Rust package
- child frontend package under `frontend/`

Assertions:

- parent primary language is Rust
- child primary language is TypeScript or JavaScript depending on fixture contents
- parent package scan excludes child files

### Workspace Merge Tests

Add cases where the same package root is found by:

- workspace declaration
- manifest scan

Expected result:

- single package entry
- multiple discovery sources retained

### Fallback Tests

If run in an arbitrary directory that is not a detected package:

- `sniff language` should still analyze the current directory
- no package boundary assumptions should be required

## Recommended Decision

For the `homelab/server` example, the correct fix is:

1. detect `frontend` as a distinct package boundary
2. exclude nested package roots from parent package language scans
3. optionally use Cargo source-root metadata as a secondary weighting hint

The important point is that Cargo source metadata alone is not enough. The package model needs to understand that `frontend` is its own package. Once that boundary exists, the language report for `homelab-server` becomes accurate for the right reason.

## Summary

The package-detection bug is fundamentally a boundary problem, not just a language-scoring problem.

`sniff` should stop assuming a directory tree belongs to a single package just because it starts at one package root. In mixed monorepos, nested manifests define real package boundaries and must be respected.

If `sniff`:

- merges package discovery across workspace tools
- recognizes nested package manifests
- excludes nested package roots from parent scans

then the `homelab/server` case is fixed cleanly and the same approach will generalize to other embedded frontend, demo, tool, and polyglot package layouts.
