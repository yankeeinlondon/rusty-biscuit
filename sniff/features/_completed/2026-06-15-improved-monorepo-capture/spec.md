---
status: ready for planning and implementation
reviewed: true
---

# Improved Monorepo Capture

Introduce a new `MonorepoStandard` enum (and supporting types) that replaces the
existing `MonorepoTool` enum with a higher-fidelity model of how a monorepo is
organized. `MonorepoStandard` lands **alongside** `MonorepoTool` initially; the
long-term aim is to eliminate `MonorepoTool` after all library and CLI JSON
surfaces have migrated.

- Background research: [`sniff/docs/research/monorepo.md`](../../docs/research/monorepo.md)
- Use the `sniff`, `cli`, `rust`, `rust-devops`, and `monorepos` agent skills.
- The CLI owns reporting; the library owns all detection/business logic.

## Goals

1. **Detect** when a monorepo standard is in use (and answer the binary
   "is this a monorepo?" question honestly).
2. **Distinguish** the packages a standard defines.
3. **Provide rich, high-quality metadata** about how the monorepo is set up.

## Core principle: separate the axes `MonorepoTool` conflates

`MonorepoTool` collapses three orthogonal concerns into one flat list. The new
model keeps them apart:

1. **Membership format** — *what file/field declares which packages belong*
   (`Cargo.toml [workspace].members`, `pnpm-workspace.yaml packages:`,
   `go.work use`, Bazel `BUILD` files). This is what answers "distinguish the
   packages."
2. **Role** — *what the tool does*: defines membership, orchestrates tasks,
   manages dependencies. Nx/Turbo/Lerna only orchestrate; Cargo/uv/Go only
   define + build; Bazel/Pants/Buck2 do **both**. A single-axis enum cannot
   express "both," which is why a strict two-enum (Standard vs Orchestrator)
   split also fails — roles are modeled as a *property*, not a separate enum.
3. **Acting binary** — *what you actually run*. npm, yarn, and bun all consume
   the same `package.json#workspaces` field, but `npm run build -w X` ≠
   `yarn workspace X build` ≠ `bun run --filter X build`. The binary is resolved
   from the lockfile, not the membership file.

## The enum

`MonorepoStandard` is a single flat `#[non_exhaustive]` enum. A detection result
is a set of these (see Topology). Richness lives in a const descriptor each
variant returns; roles are a property rather than a sibling enum.

The enum must derive `Serialize` / `Deserialize` with a stable
`#[serde(rename_all = "kebab-case")]` representation, and each variant's
`spec().id` must match that wire value. This intentionally differs from the
current Rust-internal `MonorepoTool` serialization. Because this is a new type,
the kebab-case ids are an intended contract change, not an accidental break.

```rust
#[non_exhaustive]
pub enum MonorepoStandard {
    // Membership authorities (workspace standards)
    CargoWorkspace, NpmWorkspaces, PnpmWorkspaces, YarnWorkspaces,
    BunWorkspaces, UvWorkspace, GoWorkspace,
    GradleMultiProject, MavenMultiModule, DotNetSolution, SwiftPackage,
    // Polyglot build systems — BOTH define membership AND orchestrate
    Bazel, Pants, Buck2, RushStack,
    // Pure orchestrators (layered on a membership authority)
    Nx, Turborepo, Lerna,
    // Fallback
    Unknown,
}
```

Reader note: `SwiftPackage` is provisional. SwiftPM's `Package.swift` primarily
declares targets and products inside one package, so it must not be counted as a
multi-package monorepo merely because it has multiple targets. See the SwiftPM
open question below before implementing that variant.

### JS variants: per-binary, not per-format (decision)

npm/yarn/bun share one membership format (`package.json#workspaces`); only pnpm
has its own file. We keep `NpmWorkspaces` / `YarnWorkspaces` / `BunWorkspaces` as
distinct variants — their `markers` point at the *same* field — because the
invocation templates differ per binary. Detection disambiguates by lockfile
precedence (the existing `resolve_js_package_manager` logic).

## The heart: a const metadata descriptor

Every variant exposes `fn spec(self) -> &'static MonorepoStandardSpec`. All
compile-time data, zero runtime cost.

```rust
pub struct MonorepoStandardSpec {
    pub id: &'static str,             // "cargo-workspace" (stable serde id)
    pub display_name: &'static str,   // "Cargo Workspace"
    pub roles: &'static [Role],       // [DefinesMembership, OrchestratesTasks, ...]
    pub primary_language: Option<ProgrammingLanguage>, // Some(Rust); None = polyglot
    pub markers: &'static [Marker],   // detection proof: file + content predicate + confidence
    pub membership: MembershipModel,  // how packages are declared
    pub root_membership: RootMembership, // is the workspace root itself a package?
    pub multiplicity: WorkspaceMultiplicity, // what counts toward "multi-package?"
    pub binaries: &'static [BinarySpec], // name + version arg + wrapper policy + min_version
    // Advisory operation descriptors — sniff NEVER executes these (see §8).
    pub enumerate_packages: Option<InvocationTemplate>, // e.g. `cargo metadata --no-deps --format-version 1`
    pub run_in_package: Option<InvocationTemplate>,
    pub run_across_all: Option<InvocationTemplate>,
    pub nesting_policy: NestingPolicy, // does a nested marker start a new workspace?
}
```

### Supporting types

```rust
pub enum Role { DefinesMembership, OrchestratesTasks, ManagesDependencies }

pub struct Marker {
    pub file: &'static str,            // "Cargo.toml"
    pub requires: MarkerContent,       // Existence | Field("workspace.members")
    pub confidence: MarkerConfidence,  // Strong (proves it) | Secondary (corroborates)
}

pub enum MarkerContent {
    /// File presence alone is sufficient (pnpm-workspace.yaml, go.work).
    Existence,
    /// A keyed field must be present & non-empty
    /// (Cargo.toml -> "workspace.members", package.json -> "workspaces",
    ///  pom.xml -> "modules").
    Field(&'static str),
}

/// Captures the deep structural split between root-ward and leaf-ward
/// membership, and (for the root-ward cases) globs vs explicit paths.
pub enum MembershipModel {
    /// Root manifest lists member globs (Cargo, npm, pnpm, uv).
    RootGlobs { dialect: GlobDialect, include: &'static str, exclude: Option<&'static str> },
    /// Root manifest lists explicit member paths — NOT globs
    /// (go.work `use`, Maven <modules>, Gradle `include`, Rush `projects`).
    RootExplicit,
    /// Packages are any directory containing a build file
    /// (Bazel/Pants/Buck `BUILD` files). Leaf-ward, not root-ward.
    LeafMarkers { file: &'static str },
    /// Targets declared inline in a single manifest (SwiftPM Package.swift).
    InlineTargets,
    /// A manifest lists local path dependencies that point at other packages
    /// (SwiftPM `.package(path:)`, older ad hoc Rust path dependency setups).
    LocalPathDependencies,
}

/// Glob dialect, so the expander interprets member patterns correctly.
pub enum GlobDialect {
    /// Cargo's documented subset (prefix `*`, limited `**`).
    Cargo,
    /// minimatch-style: `**`, `{a,b}`, `!negation` (npm/pnpm/yarn/bun, uv).
    Minimatch,
}

/// Whether the workspace root is itself a package.
pub enum RootMembership {
    /// Root is never a member (pnpm, npm, yarn, Maven parent `packaging=pom`).
    Never,
    /// Root is always a member (uv: the root `[project]` is a workspace member).
    Always,
    /// Root is a member only when its manifest also declares a package
    /// (Cargo: `[workspace]` + `[package]` in the same Cargo.toml).
    WhenManifestDeclaresPackage,
}

pub enum WorkspaceMultiplicity {
    /// The standard declares package members directly.
    MemberCount,
    /// The standard has nested target/product concepts; targets alone do not
    /// make a monorepo.
    PackageBoundaryOnly,
}

pub struct BinarySpec {
    pub name: &'static str,                 // "cargo", "pnpm", "gradle"
    pub version_arg: &'static str,          // "--version"
    pub min_version: Option<&'static str>,  // "7" for npm workspaces, "1.20" for go.work
    pub wrapper: Option<WrapperScript>,     // gradlew / gradlew.bat
}

pub struct WrapperScript {
    pub posix: &'static str,    // "gradlew", "mvnw"
    pub windows: &'static str,  // "gradlew.bat", "mvnw.cmd"
}

pub struct InvocationTemplate {
    pub program: &'static str,        // "cargo"
    pub args: &'static [Token],       // [Lit("build"), Lit("-p"), Package]
}

pub struct ResolvedBinary {
    pub name: String,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    pub satisfies_min_version: Option<bool>,
    pub source: BinarySource,
}

pub enum BinarySource {
    Wrapper,
    Path,
    Missing,
}

pub struct LayerPackage {
    pub name: String,
    pub relative: PathBuf,
    pub standard: MonorepoStandard,
    pub provenance: PackageProvenance,
}

pub enum Token {
    Lit(&'static str),  // literal arg
    Package,            // substituted with the target package name
    Task,               // substituted with the user task (build/test/...)
    AllPackages,        // expands to the "run everywhere" flag(s)
}

pub enum NestingPolicy {
    /// Overlapping/nested workspaces are forbidden (Cargo). Stop walking.
    ForbidsNested,
    /// A nested marker starts a SEPARATE workspace; ignore its subtree
    /// from the parent's perspective (Bazel nested WORKSPACE).
    IgnoresNested,
    /// Nested workspaces are allowed and discovered as their own roots.
    AllowsNested,
}
```

## Decision log

### 1. Topology — detection is a forest, not a flat list

Detection is **not** "one set of standards at the repo root." A Cargo workspace
can contain a pnpm workspace several directories down; a repo can host a Cargo
workspace and a uv workspace side by side at the root. Detection yields
standards-with-roots:

```rust
pub struct DetectedStandard {
    pub standard: MonorepoStandard,
    pub root: PathBuf,                  // where this standard's root marker lives
    pub matched_markers: Vec<PathBuf>,  // which marker files actually matched
    pub binary: Option<ResolvedBinary>, // acting binary resolved from lockfile/wrapper
    pub confidence: DetectionConfidence,
}
```

The walker consults `spec().nesting_policy` to decide whether to recurse past a
found marker. Even though ~95% of repos have exactly one standard at the top,
modeling the forest from the start avoids a lossy flattening that is expensive
to undo later.

### 2. Authority vs orchestrators — derive the relationship, don't flatten

A flat `[PnpmWorkspaces, Nx]` loses the salient fact: **Nx delegates membership
to pnpm**. The detection result for a given root distinguishes:

```rust
pub struct MonorepoLayer {
    pub root: PathBuf,
    /// The standard that actually declares the packages (role DefinesMembership).
    pub authority: MonorepoStandard,
    /// Orchestrators riding on top (role OrchestratesTasks, not the authority).
    pub orchestrators: Vec<MonorepoStandard>,
    /// How this layer's package list was derived (see §8). Packages inherit it.
    pub provenance: PackageProvenance,
    /// Packages resolved for this membership layer, using paths relative to
    /// this layer root.
    pub packages: Vec<LayerPackage>,
}
```

This is derived from the `roles` data, not stored as an undifferentiated list.
Consumers get the answer they actually want: "packages come from pnpm; tasks run
through Nx."

If a root contains multiple membership authorities side by side (for example a
Cargo workspace and a uv workspace both rooted at the repository root), detection
produces multiple `MonorepoLayer`s with the same root. They are not merged into a
single fake authority. Repo-level package lists can still present a flattened
view, but the layer relationship remains available for consumers that need to
know which standard owns which package.

### 3. A real "is this multi-package?" predicate per standard

"Marker present" ≠ "is a monorepo." Each standard knows its own degenerate case:
`[workspace]` with no members, `workspaces: []`, a `Package.swift` with a single
target, a lone `pom.xml` with no `<modules>`. The `MembershipModel` carries
enough to answer "did this resolve non-degenerately?"

The default rule is:

- at least two resolved package boundaries; or
- one non-root member plus a root manifest that is also a package, when the
  standard's `RootMembership` says the root can count.

Targets, products, tasks, projects, and CI jobs are not packages unless the
standard defines them as package boundaries. This prevents SwiftPM targets, Nx
project metadata, or Bazel targets from inflating `is_monorepo`.

The repo-level decision becomes: **any standard whose membership resolves
non-degenerately → monorepo**, with `DetectionConfidence` recording whether the
answer was marker-confirmed or inferred.

```rust
pub enum DetectionConfidence {
    /// A Strong marker matched and membership resolved non-degenerately.
    MarkerConfirmed,
    /// Inferred from weaker signals (e.g. ≥2 independent package manifests in
    /// different trees) — standard reported as Unknown.
    Inferred,
}
```

### 4. `MembershipModel`: globs vs explicit, with glob dialects

Maven `<modules>` and Gradle `include 'a:b:c'` are **literal paths, not globs** —
they are `RootExplicit`. Among real glob users, dialects differ (Cargo's `*`-
prefix subset vs minimatch `**`/`{a,b}`/`!neg`); `RootGlobs` carries a
`GlobDialect` so the expander is correct. (See research §4.4 / §5.6 — the current
hand-rolled `expand_glob_patterns_with_deps` only understands `prefix*`.)

The new expander must be cross-platform: workspace files use `/` separators even
on Windows, while the implementation compares against `Path` values. Normalize
patterns and candidate relative paths to slash-separated logical paths before
matching, then convert accepted members back to `PathBuf`.

### 5. Root-as-package is per-standard static knowledge

Modeled as `RootMembership` (`Never | Always | WhenManifestDeclaresPackage`)
rather than the current inconsistent per-tool behavior (research §4.5).

### 6. Version-gated capability

`BinarySpec.min_version` covers detection gates (npm ≥7 for workspaces, Go ≥1.20
for `go.work`). This gate affects "can the host operate this standard with the
resolved binary?", not whether the committed files prove the repo uses the
standard. For example, a repo with `go.work` is still a Go workspace on a host
without `go` installed; the `ResolvedBinary` is absent or version-incompatible.

The few standards with version-branched invocation — really just Yarn Classic
(`yarn workspaces run`) vs Berry (`yarn workspaces foreach`) — carry variant
templates. We do **not** over-generalize this for the 90% that do not need it.

### 7. Single source of truth for binary availability

`BinarySpec` declares the binary name, wrapper policy, and `min_version`. It does
**not** re-implement `which cargo`. The existing `LanguagePackageManager`
registry (`registry.rs`) and `ExecutableIndex` answer "present? which version?".
Repo detection wires the two together (closing research §5.3), instead of
creating a second source of truth.

### 8. Static-knowledge vs requires-execution boundary

**sniff never executes a binary to detect packages.** This is a hard constraint
inherited from sniff's cost model: detection is filesystem-only (read + parse),
which is why even git was migrated to pure-Rust `gix`. The cost ladder is
`structure()` (cheap parse) → `full()` (heavy parse); there is no "run a
subprocess" rung, and we are not adding one.

**Packages are derived from configuration files**, leaning on our model of each
standard. The key realization is that "authoritative package list" almost never
requires running the tool — membership is filesystem-derivable for every
standard we cover:

- pnpm `pnpm-lock.yaml` `importers:` *is* the resolved member list, keyed by path
- npm `package-lock.json` v2+ `packages` carries workspace entries
- uv `uv.lock` (TOML) lists members; Cargo members are dirs with `Cargo.toml`
  under the globs, corroborated by `Cargo.lock`
- Go / Gradle / Maven declare explicit member paths (`use`, `include`, `<modules>`)
- Bazel / Pants / Buck2 are discovered by walking for their leaf markers
  (`BUILD`, `BUILD.bazel`, `BUCK`, `BUILD.pants`) — that walk *is* the membership
  model
- Nx project files (`project.json`, `workspace.json`, `angular.json`) describe
  projects for orchestration; they are package boundaries only when no membership
  authority is present and the project root has a package manifest or a language
  manifest

The committed **lockfile** is the high-fidelity, filesystem-resident source:
far more reliable than re-expanding globs by hand, still a pure parse.

#### Provenance is a first-class, honest field

Each layer records how its package list was derived. sniff only ever reports
filesystem-derived provenance:

```rust
pub enum PackageProvenance {
    /// Expanded membership globs.
    /// Best-effort; bounded by our glob expander.
    Globbed,
    /// Parsed a root manifest's explicit member path list.
    Explicit,
    /// Walked leaf-side marker files such as BUILD / BUCK / project.json.
    LeafMarkers,
    /// Parsed local path dependencies as package links.
    LocalPathDependencies,
    /// Parsed the committed lockfile's resolved member set.
    /// High fidelity, still filesystem-only.
    Lockfile,
}
```

There is deliberately **no `Tool` variant** — sniff never executes, so it never
emits tool-derived provenance.

When lockfile and manifest disagree, the manifest remains the membership
authority and the lockfile becomes corroborating evidence. The package should
record both facts: provenance `Globbed` or `Explicit` for the authority, plus a
lockfile match/mismatch detail. A stale lockfile must not delete packages from
the reported monorepo.

#### Operation descriptors are advisory metadata, never executed

`enumerate_packages`, `run_in_package`, and `run_across_all` on the spec describe
the standard's CLI surface — the appropriate command and params for those
operations — as `InvocationTemplate`s. They exist so a **consumer** (claudine,
the commit prompt) can construct `cargo build -p X` / `pnpm --filter X build` /
`cargo metadata …` correctly. **sniff itself never runs them.** They are
reference metadata, on the same footing as `primary_language` or `markers`.

#### Sequencing

1. Land the `PackageProvenance` field + a **correct glob expander** (replacing the
   `prefix*`-only `expand_glob_patterns_with_deps`, research §4.4 / §5.6). This
   alone makes `Globbed` trustworthy.
2. Add the `Lockfile` tier per ecosystem as a fast-follow, prioritizing
   pnpm / uv / Cargo (biggest fidelity-per-effort).

Provenance attaches to `MonorepoLayer` (membership resolution is a layer-level
act); `Package` inherits it.

## Public API and JSON impact

This feature is a library-first change. Add the new types to `sniff-lib` and make
`detect_repo_structure` / `detect_repo` populate them before changing CLI
reporting.

`RepoInfo` should grow additive fields during migration:

```rust
pub struct RepoInfo {
    pub is_monorepo: bool,
    pub monorepo_tool: Option<MonorepoTool>,       // legacy, deprecated later
    pub workspace_tools: Vec<MonorepoTool>,        // legacy, deprecated later
    pub monorepo_layers: Vec<MonorepoLayer>,       // new
    pub monorepo_standards: Vec<DetectedStandard>, // new raw detections
    // existing fields...
}
```

Serialization rules:

- new enum ids use kebab-case (`cargo-workspace`, `pnpm-workspaces`);
- existing `MonorepoTool` JSON remains unchanged until the migration removes it;
- `sniff repo --json` may add `monorepo_layers` / `monorepo_standards` to rich
  scopes such as `structure`, but should not remove `monorepo_tool` or
  `workspace_tools` in the same implementation step;
- single-value leaves (`sniff repo is-monorepo`, `package-count`, etc.) keep
  their current contracts unless a separate spec intentionally changes them.

Reader note — why not replace immediately: `RepoInfo` is serialized by multiple
CLI paths and used by recent-commit/package-area logic. An additive migration
lets implementation verify parity and update downstream commands without
breaking existing JSON consumers in the same change.

## Implementation scope

Implement in this order:

1. Add `MonorepoStandard`, descriptors, roles, markers, membership models,
   binary metadata, and serde ids in the library.
2. Add a `MonorepoLayer` builder that maps current Cargo/npm/pnpm/Yarn/Nx/Turbo/
   Lerna detection into the new model without changing existing behavior.
3. Replace the glob expansion helper with dialect-aware matching and update the
   existing detectors to use it.
4. Add new standards incrementally: Bun, uv, Go, Gradle, Maven, .NET solution,
   Bazel/Pants/Buck2, Rush. Each standard must include tests for marker
   detection, degenerate membership, root-membership behavior, and package
   enumeration.
5. Wire binary resolution through `LanguagePackageManager` / `ExecutableIndex`
   and wrappers, preserving the "no subprocess for package detection" rule.
6. Update CLI output to report the new metadata through existing `Renderable`
   components and JSON builders. The CLI must only format library-provided
   facts.
7. Deprecate `MonorepoTool` only after all repo commands and JSON snapshots have
   parity coverage for the new fields.

## Testing and acceptance criteria

- Unit tests cover every `MonorepoStandardSpec` descriptor: stable id,
  display name, roles, marker predicates, root-membership policy, and binary
  metadata.
- Fixture tests cover at least one positive and one degenerate repo for each
  implemented standard.
- Cross-platform tests assert slash-normalized glob matching with Windows-style
  candidate paths and POSIX-style workspace patterns.
- Existing `sniff repo packages`, `package-areas`, `is-monorepo`, `structure`,
  and bare `sniff repo --json` tests continue to pass.
- JSON snapshot tests prove `monorepo_tool` / `workspace_tools` remain present
  during migration and new fields serialize with kebab-case standard ids.
- No package detection test may require `cargo`, `pnpm`, `go`, `gradle`, or any
  other external monorepo binary to be installed. Binary availability tests use
  the existing executable-index abstraction with synthetic entries.
- Terminal output introduced by this work uses `biscuit-terminal` renderables
  (`Prose` or existing repo components), with `stdout` reserved for main command
  content and `stderr` only for diagnostics.

## Open questions

### How should multi-root topology appear in public JSON?

The library should model a forest now, but exposing every root immediately in
top-level CLI JSON is a public contract decision. Three viable options:

1. **Expose only `monorepo_layers` inside rich repo scopes for now.**
   Pros: additive, low risk, enough for consumers that already parse `structure`.
   Cons: users cannot query a single standard/root directly yet.
2. **Add new focused leaves such as `sniff repo standards` and
   `sniff repo layers`.**
   Pros: clear drill-down commands and scope-complete JSON.
   Cons: expands CLI surface and requires output design beyond this library
   migration.
3. **Replace existing `is-monorepo` text output with the primary standard.**
   Pros: compact and script-friendly when there is one root.
   Cons: lossy for side-by-side roots and conflicts with the current leaf
   contract.

Recommendation: choose option 1 for this implementation. It preserves existing
CLI contracts while making the richer model available to JSON consumers. Add
focused leaves in a follow-up once real output examples show what users need.

### How should SwiftPM be represented?

SwiftPM does not have a direct analogue to Cargo workspace members. A single
`Package.swift` can declare many targets/products, but those are not packages in
Sniff's current package-area sense.

1. **Treat multi-target `Package.swift` as `SwiftPackage`.**
   Pros: easy to detect.
   Cons: incorrectly reports ordinary Swift packages as monorepos.
2. **Detect only multiple local Swift packages connected by `.package(path:)`.**
   Pros: preserves the package-boundary definition.
   Cons: requires a Swift manifest parser or conservative text extraction.
3. **Defer SwiftPM and report it only through general manifest scanning.**
   Pros: avoids a false-positive monorepo standard.
   Cons: leaves Swift monorepos less rich in the first pass.

Recommendation: choose option 2 when SwiftPM is implemented; until then leave
`SwiftPackage` behind a TODO or omit it from the implemented descriptor table.
This keeps `is_monorepo` honest.

## Out of scope (consciously deferred)

- **Versioning / release tooling** (Changesets, Rush change files, `lerna
  publish`) — a plausible fourth role, but orthogonal to "distinguish packages +
  metadata."
- **Removing `MonorepoTool`** — this spec starts the additive migration. Removal
  needs a separate compatibility cleanup once new fields are exercised by the
  CLI and tests.
- **Executing monorepo binaries for enumeration** (`cargo metadata`, `pnpm list`,
  `go work edit -json`, etc.) — invocation templates can describe those commands
  for consumers, but Sniff package detection remains parse-only.

## Existing-type impact (for later planning)

- `PackageDiscoverySource` is nearly "which `MonorepoStandard` found this
  package" — candidate for unification once `MonorepoStandard` lands.
- `PackageEcosystem` overlaps `spec().primary_language` but stays for now:
  polyglot standards (Bazel/Nx/Turbo) still need per-package language inference.
- `RepoInfo.monorepo_tool` / `workspace_tools` (`Option<MonorepoTool>` /
  `Vec<MonorepoTool>`) are superseded by the topology types above; both enums
  coexist during migration.
