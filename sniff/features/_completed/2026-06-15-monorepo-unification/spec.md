---
status: ready for planning and implementation
reviewed: true
depends-on: 2026-06-15-improved-monorepo-capture
---

# Monorepo Type Unification

Consolidate the legacy package/monorepo types into the `MonorepoStandard`
topology model introduced by [improved-monorepo-capture](../2026-06-15-improved-monorepo-capture/spec.md).
That feature deliberately landed the new model **additively** and left three
legacy types coexisting with their successors. This spec is the deferred
cleanup it pointed at:

> **Removing `MonorepoTool`** — this spec starts the additive migration.
> Removal needs a separate compatibility cleanup once new fields are exercised
> by the CLI and tests.

and the three items under its `## Existing-type impact (for later planning)`
section:

1. `PackageDiscoverySource` is nearly "which `MonorepoStandard` found this
   package" — candidate for unification once `MonorepoStandard` lands.
2. `PackageEcosystem` overlaps `spec().primary_language` but stays for now.
3. `RepoInfo.monorepo_tool` / `workspace_tools` are superseded by the topology
   types; both enums coexist during migration.

This is a **breaking JSON contract change**. The additive-migration constraint
that governed improved-monorepo-capture is intentionally retired here: the
purpose of this feature is to delete the duplicated surface, not preserve it.

- Background research: [`sniff/docs/research/monorepo.md`](../../docs/research/monorepo.md)
- Use the `sniff`, `cli`, `rust`, `rust-devops`, and `monorepos` agent skills.
- The CLI owns reporting; the library owns all detection/business logic.

## Precondition (hard gate)

This feature **must not start** until improved-monorepo-capture is fully
landed and verified, specifically:

- Every `sniff repo` subcommand and JSON snapshot exercises
  `monorepo_layers` / `monorepo_standards` (the parity coverage that Phase 8
  of that plan requires before `#[deprecated]` is applied).
- `MonorepoTool` already carries `#[deprecated(note = "Use MonorepoStandard
  via RepoInfo::monorepo_layers instead")]`.
- The `Lockfile` / `Globbed` / `Explicit` / `LeafMarkers` provenance tiers are
  populated on real layers, so `PackageProvenance` is a trustworthy
  replacement for `PackageDiscoverySource`.

If any of these is not true, this spec is blocked.

## Goals

1. **Remove `MonorepoTool` entirely** — delete the enum, the
   `RepoInfo.monorepo_tool` / `workspace_tools` fields, the text formatter
   (`format_monorepo_tool`), and every JSON surface that emits its PascalCase
   wire values.
2. **Remove `PackageDiscoverySource`**, replacing it with `MonorepoStandard` +
   `PackageProvenance` on `Package` — a package already knows which standard
   owns it (`LayerPackage.standard`) and how its boundary was derived
   (`LayerPackage.provenance`). The enum and the `discovery_sources: Vec<…>`
   field are both deleted once that data is canonical.
3. **Reconcile `Package` and `LayerPackage`** — make the rich `Package`
   catalog canonical, move `standard` + `provenance` onto `Package`, and make
   `MonorepoLayer.packages` a thin reference list. Today there are two package
   representations describing overlapping facts; this feature removes that
   duplication rather than expanding it.
4. **Resolve the `PackageEcosystem` vs `ProgrammingLanguage` overlap** — either
   keep `PackageEcosystem` with a documented, narrow reason, or fold it into
   per-package language inference. The improved-monorepo-capture note leans
   "keep"; this spec must make that call explicit rather than implicit.

## Current state (what we are unifying)

### `MonorepoTool` (legacy, deprecated)

- Definition: `sniff/lib/src/filesystem/repo/types.rs:15`.
- 8 variants: `CargoWorkspace`, `NpmWorkspaces`, `PnpmWorkspaces`,
  `YarnWorkspaces`, `Nx`, `Turborepo`, `Lerna`, `Unknown`.
- **No `rename_all`** — serializes to PascalCase (`"CargoWorkspace"`).
- Fields on `RepoInfo` (`types.rs:78`):
  `monorepo_tool: Option<MonorepoTool>`, `workspace_tools: Vec<MonorepoTool>`.
- Constructed in `cargo.rs`, `npm.rs`, `nx_turbo.rs`, and the
  `Unknown` fallback in `detection.rs`.
- CLI: `format_monorepo_tool` at
  `sniff/cli/src/output/filesystem/repo.rs:143`; JSON via `structure_value` /
  `build_aggregate_value` in `sniff/cli/src/output/repo_json.rs`.

### `PackageDiscoverySource`

- Definition: `sniff/lib/src/filesystem/repo/types.rs:59`.
- 8 variants: `CargoWorkspace`, `PnpmWorkspace`, `NpmWorkspace`,
  `YarnWorkspace`, `Nx`, `Turborepo`, `Lerna`, `ManifestScan`.
- `#[serde(rename_all = "snake_case")]` → `"cargo_workspace"`,
  `"manifest_scan"`, etc.
- Stored as `Package.discovery_sources: Vec<PackageDiscoverySource>`
  (`types.rs:125`), deduplicated on merge (`detection.rs:719`).
- Produced via `discovery_source_for_tool()` (`detection.rs:632`) — a 1:1 map
  off `MonorepoTool`.

### `PackageEcosystem`

- Definition: `sniff/lib/src/filesystem/repo/types.rs:37`.
- 5 variants: `Cargo`, `Node`, `Python`, `Go`, `Unknown` (`Unknown` is
  `#[default]`). `#[serde(rename_all = "snake_case")]`.
- Inferred by manifest-file presence in `detect_package_ecosystem()`
  (`detection.rs:645`), first-match-wins, no content inspection.
- Stored as `Package.ecosystem` and serialized unconditionally.

### `ProgrammingLanguage`

- Definition: `sniff/lib/src/filesystem/file_types/model.rs:84` (~28 variants).
- Stored as `Package.primary_language: Option<ProgrammingLanguage>` and
  `Package.secondary_languages: Vec<ProgrammingLanguage>`, derived from file
  scanning (rich `full()` repo mode only).

### `LayerPackage` (new, from improved-monorepo-capture)

- Carries `name`, `relative`, `standard: MonorepoStandard`,
  `provenance: PackageProvenance`. This is the data `PackageDiscoverySource`
  was a lossy stand-in for.

## The core insight: the legacy types are lossy projections

`PackageDiscoverySource` conflates two axes that `MonorepoStandard` separated:

- **Membership authority** (`CargoWorkspace`, `PnpmWorkspace`, …) — *what
  declares the package*.
- **Orchestrator** (`Nx`, `Turborepo`, `Lerna`) — *what runs tasks over it*.

In the legacy model a package "discovered by Nx" records
`PackageDiscoverySource::Nx`, even though Nx never declares membership — it
delegates to an underlying authority (pnpm, npm) or, standalone, to project
files. The new model already captures this correctly:
`LayerPackage.standard` is the **authority**, and orchestrators live on
`MonorepoLayer.orchestrators`. So the unification is **not** a mechanical
rename — it is the removal of a type whose variants encode a category error the
new model fixes.

`ManifestScan` similarly maps to "no authority" — i.e.
`MonorepoStandard::Unknown` + `PackageProvenance` (most naturally `LeafMarkers`
or a new manifest-scan provenance), never to a real standard.

## Reviewed decisions

These decisions are part of the reviewed spec. Planning should produce concrete
tasks for them rather than reopening the design unless implementation uncovers
new evidence.

### D1 — Delete `MonorepoTool`; no replacement field on `RepoInfo`

`monorepo_tool` and `workspace_tools` are removed from `RepoInfo`. Consumers
read `monorepo_layers` (authority + orchestrators per root) and
`monorepo_standards` (raw detections) instead. The repo-level "primary tool"
one-liner the CLI renders is derived from `monorepo_layers[0].authority`.

- **Breaking JSON:** the `monorepo_tool` and `workspace_tools` keys disappear.
- The PascalCase wire values (`"CargoWorkspace"`) are gone; the kebab-case
  `monorepo_layers[].authority` (`"cargo-workspace"`) is the only surface.

### D2 — Replace `Package.discovery_sources` with `standard` + `provenance`

Drop `PackageDiscoverySource` entirely. `Package` gains:

```rust
pub standard: MonorepoStandard,        // the membership authority that owns it
pub provenance: PackageProvenance,     // how its boundary was derived
```

Both already exist on `LayerPackage`; this lifts them onto the canonical
`Package`. `discovery_source_for_tool()` and the `discovery_sources` merge
logic are deleted.

- **Open sub-question:** `discovery_sources` was a `Vec` because one package
  could be found by multiple mechanisms. With a single authority + provenance,
  do we lose information? Recommendation: **no** — multiple "sources" in the
  legacy model were almost always `[authority, manifest_scan]` duplicates that
  the new layer model represents as one authority with a provenance tier.
  Confirm with a parity audit over the rusty-biscuit repo.

### D3 — `Package` is canonical; layers carry relative package references

`MonorepoLayer.packages` must not duplicate the rich `Package` data. The
selected design is:

```rust
pub struct MonorepoLayer {
    pub root: PathBuf,
    pub authority: MonorepoStandard,
    pub orchestrators: Vec<MonorepoStandard>,
    pub provenance: PackageProvenance,
    pub packages: Vec<PathBuf>, // paths relative to the repository root
}
```

`RepoInfo.packages` is the canonical package catalog. Each `Package` owns the
package's `standard` and `provenance`, plus the existing language, dependency,
documentation, package-area, and change-scope fields. `MonorepoLayer.packages`
is a topology index into that catalog using repo-relative paths, not layer-root
relative paths. Repo-relative paths are already stable in `Package.relative`,
avoid ambiguity when a repo has multiple layers with the same package basename,
and let JSON consumers join layers to package details without normalizing
different path bases.

Reader note: the parent spec landed `LayerPackage { name, relative, standard,
provenance }` additively to keep parity visible. That shape was useful during
the migration, but keeping it after `Package.standard` / `Package.provenance`
would create two mutable sources of truth. This spec intentionally collapses
the duplicated data into `Package`.

Rejected alternatives:

| Alternative | Pros | Cons |
|-------------|------|------|
| Keep `LayerPackage` as-is | Minimal code churn from the additive model; layer JSON is self-contained. | Duplicates `standard`, `provenance`, name, and path; risks drift between `RepoInfo.packages` and `monorepo_layers[].packages`; larger JSON for bare `sniff repo --json`. |
| Promote `LayerPackage` to canonical and have `RepoInfo.packages` hold it | One package representation in topology-heavy code. | Loses or awkwardly grafts the rich `Package` fields used by package-area, dependency, docs, language, and change-scope commands. |
| Use numeric package ids in `MonorepoLayer.packages` | Compact and unambiguous. | Adds a new identity system to a path-oriented API and makes JSON less ergonomic for scripts. |

### D4 — Keep `PackageEcosystem`, document the reason

`PackageEcosystem` is **not** redundant with `spec().primary_language`:

- `spec().primary_language` is a property of the *standard* (`Some(Rust)` for
  Cargo, `None` for polyglot Bazel/Nx/Turbo).
- `Package.ecosystem` is a property of the *individual package*, inferred from
  its own manifest — which is exactly what a polyglot standard needs
  per-package.
- `Package.primary_language` (from file scanning) is only available in rich
  `full()` mode; `ecosystem` is available in cheap `structure()` mode.

Recommendation: **keep `PackageEcosystem`**, add a doc comment stating why it
is distinct from both `ProgrammingLanguage` and `spec().primary_language`, and
close the question. Do not fold it.

- **Alternative to weigh:** derive `Package.ecosystem` from the owning
  standard's `spec().primary_language` when present, falling back to
  manifest inference only for polyglot standards. Reduces one inference path
  but couples package ecosystem to standard detection.

## JSON / CLI contract impact (breaking)

| Surface | Today | After unification |
|---------|-------|-------------------|
| `structure.monorepo_tool` | `"CargoWorkspace"` \| absent | **removed** |
| `structure.workspace_tools` | `["CargoWorkspace"]` | **removed** |
| `structure.monorepo_layers[].authority` | `"cargo-workspace"` | unchanged (now sole source) |
| `structure.monorepo_layers[].packages[]` | `{ name, relative, standard, provenance }` | repo-relative path string |
| `package.discovery_sources` | `["cargo_workspace", …]` | **removed** |
| `package.standard` | — | `"cargo-workspace"` (new) |
| `package.provenance` | — | `"globbed"` \| `"lockfile"` \| … (new) |
| `package.ecosystem` | `"cargo"` | unchanged (D4) |

Single-value leaves keep their contracts: `sniff repo is-monorepo`,
`package-count`, `version`, `name` read `is_monorepo` / `packages.len()` /
manifest values, none of which change shape. The affected subcommands are the
multi-field ones — `structure`, `packages`, `package-areas`, `deps`, the
package-change families, and bare `sniff repo --json` — all of which embed the
full `RepoInfo` / `Package` shape.

Because this is the deliberate breaking step, the README JSON docs
(`sniff/lib/README.md`, `sniff/cli/README.md`), focused CLI docs under
`sniff/docs/`, and the `sniff` skill must be updated in the same change.
Downstream claudine code currently reads `meta.env.repo.monorepo_tool` and
documents `{{project.monorepo_tool}}`; the implementation must migrate that
surface before deleting the legacy field from `sniff`. The compatible
replacement is `project.monorepo_standard`, sourced from the first
membership-authority layer when present, with `project.monorepo_orchestrators`
available for Nx/Turbo/Lerna-style task runners. If claudine intentionally
keeps the old template variable for one release, it must be derived from the
new fields and documented as deprecated rather than parsed from removed JSON.

## Implementation scope (provisional)

1. Audit every read of `MonorepoTool`, `discovery_sources`, and the removed
   JSON keys across `sniff/lib`, `sniff/cli`, `sniff/docs`, and known
   downstream consumers (claudine). Produce the parity list before deleting
   anything.
2. Add `standard` + `provenance` to `Package`; populate from the layer builder.
   Remove `discovery_sources`, `PackageDiscoverySource`, and
   `discovery_source_for_tool()`.
3. Replace `LayerPackage` with repo-relative package path references on
   `MonorepoLayer.packages`; ensure every reference resolves to exactly one
   `RepoInfo.packages[].relative` entry.
4. Remove `MonorepoTool`, the `RepoInfo` fields, `format_monorepo_tool`, and
   the JSON keys. Re-derive the CLI one-liner from `monorepo_layers`.
5. Apply the D4 decision by documenting `PackageEcosystem` as a cheap
   package-manifest classification distinct from `ProgrammingLanguage` and
   `MonorepoStandard::spec().primary_language`.
6. Migrate claudine's repo environment/template surface from `monorepo_tool` to
   the new standard/orchestrator fields, including docs and tests.
7. Update CLI text/JSON output, READMEs, focused docs, and the `sniff` skill.
8. Update all fixtures and snapshot tests to the new shapes.

## Testing and acceptance criteria

- No reference to `MonorepoTool` or `PackageDiscoverySource` remains in
  `sniff/lib` or `sniff/cli` (`git grep` clean, except possibly a CHANGELOG).
- Every `sniff repo` JSON snapshot is updated and green; the removed keys are
  absent (not `null`).
- A package's owning standard and provenance are assertable directly off
  `RepoInfo.packages` without consulting `monorepo_layers`.
- Every `monorepo_layers[].packages[]` entry is a repo-relative path that
  resolves to exactly one `RepoInfo.packages[].relative` entry; no package name
  joins are required.
- Parity test: the package set and per-package authority on the rusty-biscuit
  repo match the pre-unification `discovery_sources`-derived view (no package
  silently changes owner).
- The "Nx delegates to pnpm" case asserts `package.standard ==
  pnpm-workspaces` (authority), **not** `nx` — proving the category error is
  fixed, not preserved.
- No package detection test requires an external monorepo binary (inherited
  constraint).
- Terminal output uses `biscuit-terminal` renderables; `stdout` for main
  content, `stderr` for diagnostics only.
- claudine no longer parses `monorepo_tool` from `sniff` JSON. Any retained
  `{{project.monorepo_tool}}` compatibility alias is derived from
  `monorepo_layers`, marked deprecated in claudine docs, and covered by tests.

## Open questions

### Should the removed JSON keys get a deprecation window, or hard-break?

improved-monorepo-capture already deprecates `MonorepoTool` at the Rust level.
Options:

1. **Hard-break** at this feature: remove the keys outright. Simplest;
   relies on the prior deprecation having given consumers notice.
2. **One more release** emitting both old and new keys, with the old keys
   behind a `#[deprecated]`-style doc note, removed in a follow-up.

Recommendation: hard-break (option 1), since the additive coexistence period
*was* improved-monorepo-capture. Confirm no external consumer still reads the
PascalCase values.

### Is there a downstream consumer of `discovery_sources` we are missing?

`PackageDiscoverySource` is `#[non_exhaustive]` and serialized. Before deleting
it, confirm no claudine / commit-prompt path parses `discovery_sources` from
`sniff` JSON.

## Out of scope

- Adding new `MonorepoStandard` variants or detectors — that is
  improved-monorepo-capture's remit.
- SwiftPM (`SwiftPackage`) — still deferred per the parent spec.
- Focused CLI leaves (`sniff repo standards`, `sniff repo layers`) — still
  deferred (parent spec open-question option 1).
- Executing monorepo binaries — sniff package detection remains parse-only.
