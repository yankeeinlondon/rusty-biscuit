---
agent: open_code/zai-coding-plan/glm-5.2
phases: 8
created: 2026-06-15
start_phase: 1
yolo: "true"
source_files_during_phase_1:
  - sniff/lib/src/filesystem/repo/standard.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/src/filesystem/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/lib/Cargo.toml
  - sniff/lib/src/filesystem/repo/glob.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/src/filesystem/repo/standard.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/filesystem/repo/cargo.rs
  - sniff/lib/src/filesystem/repo/npm.rs
  - sniff/lib/src/filesystem/repo/nx_turbo.rs
  - sniff/lib/tests/fixtures.rs
  - sniff/lib/tests/integration.rs
  - sniff/cli/tests/snapshots/snapshots__help_output.snap
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - sniff/lib/src/filesystem/repo/standard.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/filesystem/repo/topology.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/src/filesystem/repo/cargo.rs
  - sniff/lib/src/filesystem/repo/npm.rs
  - sniff/lib/src/filesystem/repo/nx_turbo.rs
  - sniff/lib/tests/fixtures.rs
  - sniff/lib/tests/integration.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - darkmatter/lib/src/markdown/compose/context/capture.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
  - sniff
  - sniff-cli
  - darkmatter
---

# Improved Monorepo Capture — Execution Plan

## Goal

Land a new `MonorepoStandard` enum (and supporting const-descriptor / topology
types) alongside the existing `MonorepoTool` enum so sniff can honestly answer
"is this a monorepo?", distinguish the packages a standard defines, and expose
rich metadata about how the monorepo is organized. The legacy `MonorepoTool`
JSON contract is preserved during migration and only formally deprecated once
every repo command and JSON snapshot has parity coverage for the new fields.

The library owns all detection and business logic; the CLI only formats
library-provided facts. Package detection never executes a subprocess.

- Spec: [`spec.md`](./spec.md)
- Background research: [`../../docs/research/monorepo.md`](../../docs/research/monorepo.md)
- Use the `sniff`, `cli`, `rust`, `rust-devops`, and `monorepos` agent skills.

## Fixed Decisions (from spec)

- `MonorepoStandard` is a single flat `#[non_exhaustive]` enum; richness lives
  in a const descriptor each variant returns via `fn spec(self)`.
- Roles are a property (`Role`), **not** a sibling enum. A strict
  Standard-vs-Orchestrator split cannot express "both".
- Serialization uses `#[serde(rename_all = "kebab-case")]`; `spec().id` matches
  the wire value. This is an intentional contract change for the new type.
- JS variants stay per-binary (`NpmWorkspaces` / `YarnWorkspaces` /
  `BunWorkspaces`) even though they share the `package.json#workspaces`
  membership field; lockfile precedence disambiguates.
- Detection yields a **forest** (`DetectedStandard` + `MonorepoLayer`), not a
  flat list. Even single-root repos model the layer from day one.
- `MembershipModel` distinguishes `RootGlobs { dialect }` / `RootExplicit` /
  `LeafMarkers { file }` / `InlineTargets` / `LocalPathDependencies`.
- The new glob expander must be cross-platform: workspace files use `/`
  separators even on Windows.
- sniff **never executes** a monorepo binary to detect packages. The
  `enumerate_packages` / `run_in_package` / `run_across_all` descriptors are
  advisory `InvocationTemplate`s for consumers, not commands sniff runs.
- `PackageProvenance` is first-class and filesystem-only (`Globbed`,
  `Explicit`, `LeafMarkers`, `LocalPathDependencies`, `Lockfile`). There is
  deliberately **no `Tool` variant**.
- Topology open question → **Option 1**: expose `monorepo_layers` only inside
  rich repo scopes for now. Do not add focused leaves (`repo standards`,
  `repo layers`) in this feature.
- SwiftPM open question → **Option 3 for the first pass**: defer `SwiftPackage`
  from the descriptor table entirely (leave a TODO). Do not produce false
  positives from multi-target `Package.swift`.
- `RepoInfo` grows **additive** fields; `monorepo_tool` and `workspace_tools`
  remain unchanged until the migration is complete.

## Key Files

| Area | File | Role |
|------|------|------|
| Types | `sniff/lib/src/filesystem/repo/types.rs` | `MonorepoTool`, `RepoInfo`, `Package` (legacy) |
| New types | `sniff/lib/src/filesystem/repo/standard.rs` (new) | `MonorepoStandard`, `MonorepoStandardSpec`, supporting types |
| Detection | `sniff/lib/src/filesystem/repo/detection.rs` | `detect_repo_inner_with_shared`, `expand_glob_patterns_with_deps` |
| Detectors | `sniff/lib/src/filesystem/repo/{cargo,npm,nx_turbo,python,go}.rs` | Per-standard detectors |
| Manifest index | `sniff/lib/src/filesystem/repo/manifest_index.rs` | `ManifestIndex`, `CargoLockVersions` |
| Registry | `sniff/lib/src/package/registry.rs` | `LanguagePackageManager` registry (single source of truth for binary availability) |
| Public re-exports | `sniff/lib/src/filesystem/repo/mod.rs`, `sniff/lib/src/filesystem/mod.rs` | Re-exports |
| CLI text output | `sniff/cli/src/output/filesystem/repo.rs` | `format_monorepo_tool`, `render_repo_section` |
| CLI JSON output | `sniff/cli/src/output/repo_json.rs` | `structure_value`, `build_aggregate_value` |
| Integration fixtures | `sniff/lib/tests/fixtures.rs` | `create_cargo_workspace`, `create_pnpm_workspace`, ... |

## Phase 1: Type Foundations and Const Descriptor Table

**Depends on:** None.
**Parallelizable with:** Nothing (every later phase consumes these types).

Land the full data model as a pure, zero-runtime-cost library addition. No
detection wiring, no CLI changes — just types, `spec()` accessors, and unit
tests over the descriptor table. This phase is the contract every later phase
is verified against.

- [x] Create `sniff/lib/src/filesystem/repo/standard.rs` and register it in
  `sniff/lib/src/filesystem/repo/mod.rs`.
- [x] Add `MonorepoStandard` enum with the variant set from the spec
  (`CargoWorkspace`, `NpmWorkspaces`, `PnpmWorkspaces`, `YarnWorkspaces`,
  `BunWorkspaces`, `UvWorkspace`, `GoWorkspace`, `GradleMultiProject`,
  `MavenMultiModule`, `DotNetSolution`, `Bazel`, `Pants`, `Buck2`,
  `RushStack`, `Nx`, `Turborepo`, `Lerna`, `Unknown`).
- [x] Leave `SwiftPackage` out of the descriptor table with a `// TODO(swift):
  see spec § "How should SwiftPM be represented?"` marker so the variant is
  not introduced until option 2 (`.package(path:)`) lands.
- [x] Derive `Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash`
  with `#[serde(rename_all = "kebab-case")]`.
- [x] Add the supporting types from the spec exactly as specified:
  `MonorepoStandardSpec`, `Role`, `Marker`, `MarkerContent`,
  `MarkerConfidence`, `MembershipModel`, `GlobDialect`, `RootMembership`,
  `WorkspaceMultiplicity`, `BinarySpec`, `WrapperScript`, `InvocationTemplate`,
  `Token`, `NestingPolicy`, `ResolvedBinary`, `BinarySource`,
  `PackageProvenance`, `DetectionConfidence`, `DetectedStandard`,
  `MonorepoLayer`, `LayerPackage`.
- [x] Implement `MonorepoStandard::spec(self) -> &'static MonorepoStandardSpec`
  returning a `const` table for every variant. Populate descriptors only for
  the variants sniff already detects today (`CargoWorkspace`,
  `NpmWorkspaces`, `PnpmWorkspaces`, `YarnWorkspaces`, `Nx`, `Turborepo`,
  `Lerna`, `Unknown`); other variants return a stub with `markers: &[]` and a
  `// TODO(standard): populate in phase {n}` comment so they cannot accidentally
  match during detection.
- [x] For every implemented descriptor, assert in a unit test that
  `spec().id == <kebab-case of the variant>` and that the `id` matches the
  serde wire value (round-trip `serde_json::to_string` +
  `serde_json::from_str`).
- [x] Add a unit test per implemented variant asserting `roles`, `markers`,
  `root_membership`, `nesting_policy`, and `binary` metadata match the spec's
  decision log (e.g. `CargoWorkspace` →
  `RootMembership::WhenManifestDeclaresPackage`,
  `NestingPolicy::ForbidsNested`, `MembershipModel::RootGlobs { dialect:
  Cargo, include: "workspace.members", exclude: Some("workspace.exclude") }`).
- [x] Add a unit test asserting `Nx`, `Turborepo`, and `Lerna` carry
  `Role::OrchestratesTasks` but **not** `Role::DefinesMembership` (this is the
  authority-vs-orchestrator invariant later phases rely on).
- [x] Add `MonorepoStandardSpec` unit tests asserting `enumerate_packages`,
  `run_in_package`, and `run_across_all` are `Option<InvocationTemplate>` and
  that no implemented variant yet populates them (they land in Phase 7).
- [x] Re-export the new types from `sniff/lib/src/filesystem/repo/mod.rs` and
  `sniff/lib/src/filesystem/mod.rs` so they are reachable as
  `sniff::filesystem::repo::MonorepoStandard`.

**Validation checkpoint:**

- `cargo check -p sniff` passes.
- `cargo nextest run -p sniff --lib standard::` passes (the new unit tests).
- `git grep MonorepoStandard sniff/cli` returns no hits (CLI is untouched).
- The legacy `MonorepoTool` enum and all existing tests are byte-for-byte
  unchanged: `cargo nextest run -p sniff` is green.

## Phase 2: Cross-Platform, Dialect-Aware Glob Expander

**Depends on:** Phase 1 (`GlobDialect` exists).
**Parallelizable with:** Phase 3 after the signature below is stable, but the
recommended ordering is Phase 2 first because Phase 3's `MonorepoLayer` builder
calls the new expander and `PackageProvenance::Globbed` must be trustworthy
before any layer is reported.

Replace the `prefix*`-only `expand_glob_patterns_with_deps` helper with a
dialect-aware, cross-platform expander. This is a prerequisite the spec calls
out explicitly under "Sequencing": "Land `PackageProvenance` + a **correct**
glob expander ... This alone makes `Globbed` trustworthy."

- [x] Add `globset` to `sniff/lib/Cargo.toml` (already have `ignore` +
  `walkdir`; `globset` is the linebender sibling that implements minimatch
  `**`, `{a,b}`, `!negation`). Keep it outside the workspace Cargo.lock-free
  path by gating behind no feature flag (it is always needed for membership
  expansion).
- [x] Introduce `expand_membership_globs(root, patterns, dialect,
  lock_versions, standard) -> Vec<Package>` in
  `sniff/lib/src/filesystem/repo/detection.rs` (or a new
  `sniff/lib/src/filesystem/repo/glob.rs`), replacing the body of
  `expand_glob_patterns_with_deps` with a delegation shim so existing callers
  compile unchanged.
  *(Implemented in new `glob.rs`. The old `expand_glob_patterns_with_deps` was
  fully removed and all callers updated directly — `expand_membership_globs` is
  the replacement; `tool: MonorepoTool` is retained for `create_package` and the
  discovery source is derived from it, so `MonorepoStandard` is not threaded into
  `create_package` yet.)*
- [x] Normalize **patterns** and **candidate relative paths** to
  slash-separated logical paths before matching (workspace files use `/`
  separators even on Windows). Convert accepted members back to `PathBuf`
  using `PathBuf::from(s.replace('/', std::path::MAIN_SEPARATOR_STR))` only at
  the final `Package` construction.
- [x] For `GlobDialect::Cargo`, restrict the matcher to Cargo's documented
  subset: a leading path component may end with `*` (prefix match); `**` is
  accepted only as a full component (Cargo 1.74+). Reject brace expansion and
  negation with a `debug!` log + an empty match (do not panic).
- [x] For `GlobDialect::Minimatch`, use `globset::GlobSetBuilder` with
  `globset::Glob::compile_matcher`. Support negation by partitioning patterns
  into `include` and `exclude` sets (a leading `!` excludes). Support brace
  expansion via `globset::Glob::new(...)` which handles `{a,b}` natively.
- [x] Walk candidate directories once with the existing `ignore::WalkBuilder`
  (already used by `ManifestIndex::build`) so a multi-pattern workspace does
  not re-walk the tree per pattern. Honor `.gitignore` and skip
  `node_modules` / `target` / `dist` / `build` exactly as
  `discover_packages_from_manifests_in_tree` does today.
- [x] Keep the `lock_versions: &Option<CargoLockVersions>` and
  `discovery_source: PackageDiscoverySource` parameters so the helper can still
  construct full `Package` values via `create_package`. Do **not** thread
  `MonorepoStandard` into `create_package` yet — that is Phase 3.
- [x] Update every existing detector (`cargo.rs`, `npm.rs`, `nx_turbo.rs`) to
  call `expand_membership_globs` with the dialect from the corresponding
  `MonorepoStandard::spec().membership` (`RootGlobs { dialect, .. }`). For
  Cargo → `GlobDialect::Cargo`; for npm/pnpm/yarn/nx/turbo/lerna →
  `GlobDialect::Minimatch`.
  *(Added `MonorepoStandard::glob_dialect()` so detectors read the dialect from
  the descriptor table rather than hard-coding it.)*
- [x] Add unit tests for the new expander covering:
  - Cargo prefix glob (`members/*` matches `members/a`, not `members/a/b`).
  - Cargo `**` component glob (accepted as full component, rejected mid-path).
  - Minimatch `**` (matches arbitrarily deep).
  - Minimatch brace expansion (`packages/{app,lib}/*`).
  - Minimatch negation (`packages/*` + `!packages/excluded`).
  - Windows-style candidate paths (`packages\app`) with POSIX patterns
    (`packages/*`) — must match after slash normalization.
- [x] Add an L1 fixture test mirroring `create_pnpm_workspace` that uses
  `members/**` against a nested layout and asserts the nested packages are
  discovered (today the prefix-only expander misses them).

**Validation checkpoint:**

- `cargo nextest run -p sniff --lib` is green (existing detectors behave
  identically; new glob tests pass).
- A manual fixture run on the rusty-biscuit monorepo (which has both
  `members/*` and nested manifests) shows the same package set before and
  after the swap: `cargo run -p sniff-cli -- repo package-count` is unchanged.
- `git grep expand_glob_patterns_with_deps sniff/lib` shows only the delegation
  shim (or its replacement) — no other callers reference the old signature.

## Phase 3: Detection Wiring — MonorepoLayer Builder and Honest `is_monorepo`

**Depends on:** Phase 1 (types), Phase 2 (correct glob expander).
**Parallelizable with:** Nothing — this phase changes `RepoInfo` and every
detector.

Grow `RepoInfo` additively with `monorepo_layers` and `monorepo_standards`,
wire every existing detector to populate them, derive the
authority-vs-orchestrator relationship from `Role`, and replace the
"workspace_tools non-empty ⇒ is_monorepo" heuristic with the per-standard
non-degenerate membership predicate. The legacy `monorepo_tool` /
`workspace_tools` fields stay populated with their current values so existing
JSON consumers see no change.

- [x] Extend `RepoInfo` in `types.rs` with two additive, serde-skipped-when-empty
  fields:
  ```rust
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub monorepo_standards: Vec<DetectedStandard>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub monorepo_layers: Vec<MonorepoLayer>,
  ```
  Update every `RepoInfo { ... }` literal in `detection.rs`, `cargo.rs`,
  `npm.rs`, `nx_turbo.rs`, the CLI fixture helpers, and the repo-JSON tests so
  the struct continues to construct. Use `..Default::default()` where the
  helper exists; otherwise explicit empty `Vec`s.
- [x] Add a `MonorepoLayerBuilder` in
  `sniff/lib/src/filesystem/repo/detection.rs` (or a new `topology.rs`) that:
  1. Takes a root path and the list of `(MonorepoStandard, Vec<Package>,
     Option<ResolvedBinary>)` tuples produced by detectors.
  2. Groups by root.
  3. Splits each root's standards into `authority` (the variant whose
     `spec().roles` includes `Role::DefinesMembership`) and `orchestrators`
     (those whose roles include `Role::OrchestratesTasks` only).
  4. Returns `Vec<MonorepoLayer>` with `provenance` populated from the
     detector's membership source (`Globbed` for Cargo/npm/pnpm/yarn;
     `Explicit` for Maven/Gradle once Phase 5 lands; `LeafMarkers` for
     Bazel/Pants/Buck2 once Phase 6 lands).
- [x] Refactor `collect_repo_info` so each detector's result contributes both
  to the legacy `workspace_tools`/`packages` (unchanged behavior) **and** to
  the new `monorepo_standards` / `monorepo_layers` collections.
- [x] Implement `MonorepoStandard::membership_resolves_non_degenerately(layer:
  &MonorepoLayer) -> bool` using `spec().multiplicity` +
  `spec().root_membership`:
  - For `WorkspaceMultiplicity::MemberCount`: ≥2 resolved packages, **or** 1
    non-root member plus a root that `RootMembership` permits to count.
  - For `WorkspaceMultiplicity::PackageBoundaryOnly`: never true on its own
    (targets/products are not packages) — reserved for SwiftPM.
- [x] Replace the `if workspace_tools.is_empty() { return Ok((None, None)); }`
  gate with a new rule: **at least one layer whose membership resolves
  non-degenerately ⇒ `is_monorepo`**, with `DetectionConfidence` recording
  whether the answer was `MarkerConfirmed` or `Inferred`. Keep the legacy
  `monorepo_tool` / `workspace_tools` populated exactly as today so JSON
  parity is preserved.
- [x] For repos that previously reported `is_monorepo = true` purely because
  an `nx.json` existed (no membership authority), downgrade `is_monorepo` to
  `false` and record a `DetectionConfidence::Inferred` standard with
  `MonorepoStandard::Unknown` so the regression is observable in JSON.
- [x] Add an integration test in `sniff/lib/tests/integration.rs` that
  constructs the existing `create_cargo_workspace`,
  `create_pnpm_workspace`, and `create_mixed_nested_workspace` fixtures and
  asserts:
  - `repo.monorepo_layers` has exactly one entry per root.
  - The `authority` matches the expected `MonorepoStandard`.
  - `repo.monorepo_tool` and `repo.workspace_tools` are unchanged from the
    pre-feature behavior (snapshot both fields).
- [x] Add a JSON snapshot test that runs `detect_repo_structure` on each
  fixture, serializes `RepoInfo` with `serde_json::to_value`, and asserts:
  - `monorepo_tool` / `workspace_tools` keys still exist (legacy contract).
  - `monorepo_standards` / `monorepo_layers` keys exist when non-empty.
  - Standard ids inside the new keys are kebab-case
    (`cargo-workspace`, `pnpm-workspaces`, ...).
- [x] Add a regression test for the degenerate cases the spec calls out:
  - `Cargo.toml` with `[workspace]` but empty `members`.
  - `package.json` with `workspaces: []`.
  - `pnpm-workspace.yaml` with `packages: []`.
  Each must report `is_monorepo == false` and an empty `monorepo_layers`.
- [x] Add an `Nx + pnpm` fixture (both `nx.json` and `pnpm-workspace.yaml` at
  the same root) and assert the resulting `MonorepoLayer` has
  `authority = PnpmWorkspaces` and `orchestrators = [Nx]`.

**Validation checkpoint:**

- `cargo nextest run -p sniff` green.
- `cargo nextest run -p sniff-cli` green — no CLI changes yet, but every CLI
  test that reads `RepoInfo` must still pass.
- The integration test suite's `test_detect_cargo_workspace` and
  `test_detect_pnpm_workspace` assertions are unchanged.
- `sniff repo --json | jq '.repo.monorepo_tool, .repo.workspace_tools,
  .repo.monorepo_standards[0].standard, .repo.monorepo_layers[0].authority'`
  on the rusty-biscuit repo returns sensible values for all four keys.

## Phase 4: New Membership Authorities — Bun, uv, Go Workspace

**Depends on:** Phase 3 (detection framework).
**Parallelizable with:** Phase 5 and Phase 6 **after** the descriptor-table
stubs from Phase 1 are replaced for each variant. Each standard is independent
and can be implemented on a separate branch, merged in any order.

These three are the highest-value additions because they reuse manifests sniff
already parses (`package.json`, `pyproject.toml`, `go.mod`).

### 4A — Bun Workspaces

- [x] Populate `MonorepoStandard::BunWorkspaces.spec()` with the same markers
  as `NpmWorkspaces` (file `package.json`, `Field("workspaces")`) plus a
  `BinarySpec { name: "bun", min_version: None, wrapper: None }`. Roles:
  `[DefinesMembership, OrchestratesTasks, ManagesDependencies]`.
  `MembershipModel::RootGlobs { dialect: Minimatch, include: "workspaces",
  exclude: None }`. `RootMembership::Never`. `NestingPolicy::AllowsNested`.
- [x] Add `detect_bun_workspace(root) -> Result<Option<RepoInfo>>` in
  `sniff/lib/src/filesystem/repo/npm.rs` gated on
  `root.join("bun.lock").exists() || root.join("bun.lockb").exists()` **and**
  a non-empty `workspaces` field in `package.json`. Reuse
  `expand_membership_globs` with `GlobDialect::Minimatch`.
- [x] Disambiguate against npm/pnpm/yarn via the existing
  `resolve_js_package_manager` precedence (Bun wins when `bun.lock` /
  `bun.lockb` is present).
  *(Implemented as a `has_bun_lockfile` guard in `npm.rs`: `detect_npm_workspace`
  returns `None` when a Bun lockfile is present so Bun owns the layer.)*
- [x] Add a fixture `create_bun_workspace` in `fixtures.rs` (mirror
  `create_pnpm_workspace` with a `bun.lock` sentinel) and an integration test
  asserting `monorepo_layers[0].authority == BunWorkspaces`.
- [x] Add a degenerate fixture (`bun.lock` present, `workspaces: []`) and
  assert `is_monorepo == false`.

### 4B — uv Workspace

- [x] Populate `MonorepoStandard::UvWorkspace.spec()`: marker file
  `pyproject.toml`, `Field("tool.uv.workspace.members")`. Roles:
  `[DefinesMembership, OrchestratesTasks, ManagesDependencies]`.
  `MembershipModel::RootGlobs { dialect: Minimatch, include:
  "tool.uv.workspace.members", exclude: None }`. `RootMembership::Always` (the
  root `[project]` is itself a workspace member).
  `NestingPolicy::ForbidsNested`. `BinarySpec { name: "uv", min_version:
  Some("0.4"), wrapper: None }`.
- [x] Add `detect_uv_workspace(root)` in a new
  `sniff/lib/src/filesystem/repo/uv.rs` (or extend `python.rs`). Parse the
  `tool.uv.workspace.members` array via the existing
  `biscuit_file::toml_crate` helper. Reuse `expand_membership_globs` with
  `GlobDialect::Minimatch`.
- [x] Add `uv.rs` to `sniff/lib/src/filesystem/repo/mod.rs`.
- [x] Add a fixture `create_uv_workspace` (root `pyproject.toml` with
  `[tool.uv.workspace] members = ["packages/*"]`, two child `pyproject.toml`
  files) and a degenerate variant.
- [x] Add an integration test asserting
  `monorepo_layers[0].authority == UvWorkspace` and that the root is counted
  as a member (`packages.len() == 3`: root + 2 children) because
  `RootMembership::Always`.

### 4C — Go Workspace (`go.work`)

- [x] Populate `MonorepoStandard::GoWorkspace.spec()`: marker file `go.work`,
  `MarkerContent::Existence`. Roles:
  `[DefinesMembership, OrchestratesTasks, ManagesDependencies]`.
  `MembershipModel::RootExplicit` (the `use` directives list literal module
  paths, not globs). `RootMembership::Never`. `NestingPolicy::AllowsNested`.
  `BinarySpec { name: "go", min_version: Some("1.20"), wrapper: None }`.
- [x] Add `detect_go_workspace(root)` in `sniff/lib/src/filesystem/repo/go.rs`.
  Parse the `use` directives (lines of the form `use ./path` or a parenthesized
  `use ( ... )` block). For each resolved path, call `create_package` with the
  standard. `PackageProvenance::Explicit`.
  *(`PackageProvenance::Explicit` is derived from the `RootExplicit` membership
  model by the topology builder, so the detector calls `create_package` with the
  legacy `MonorepoTool::Unknown` and the provenance is assigned by the layer.)*
- [x] Add a fixture `create_go_workspace` (root `go.work` with
  `use (\n\t./svc-a\n\t./svc-b\n)`, two child modules with `go.mod`) and a
  degenerate variant (`go.work` with an empty `use` block).
- [x] Add an integration test asserting
  `monorepo_layers[0].authority == GoWorkspace` and that the resolved packages
  match the explicit paths.

**Validation checkpoint (Phase 4):**

- `cargo nextest run -p sniff` green, including the three new fixture suites.
- Each new standard has both a positive and a degenerate test.
- The descriptor-table unit tests from Phase 1 now cover the new variants
  (Phase 1 left them as stubs; Phase 4 replaces the stubs with real data and
  the tests must be updated to assert the real values).

## Phase 5: New Membership Authorities — Gradle, Maven, .NET Solution

**Depends on:** Phase 3. **Parallelizable with:** Phase 4 and Phase 6.

The JVM + .NET families use `RootExplicit` membership (literal paths, not
globs) and exercise the `wrapper` field of `BinarySpec` for the first time.

### 5A — Gradle Multi-Project Build

- [ ] Populate `MonorepoStandard::GradleMultiProject.spec()`: marker file
  `settings.gradle` (or `settings.gradle.kts`),
  `Field("include")`. Roles: `[DefinesMembership, OrchestratesTasks,
  ManagesDependencies]`. `MembershipModel::RootExplicit`.
  `RootMembership::Never`. `NestingPolicy::AllowsNested`. `BinarySpec { name:
  "gradle", min_version: None, wrapper: Some(WrapperScript { posix: "gradlew",
  windows: "gradlew.bat" }) }`.
- [ ] Add `detect_gradle_workspace(root)` in a new
  `sniff/lib/src/filesystem/repo/gradle.rs`. Parse `include 'a:b:c'` directives
  and translate the Gradle path (`:`-separated) to a relative directory
  (`a/b/c`). Prefer the wrapper script (`gradlew` / `gradlew.bat`) over the
  system `gradle` when resolving the acting binary in Phase 7.
- [ ] Register `gradle.rs` in `mod.rs`.
- [ ] Add a fixture `create_gradle_workspace` and a degenerate variant.
- [ ] Add an integration test asserting
  `monorepo_layers[0].authority == GradleMultiProject` and that the wrapper
  script presence is recorded (Phase 7 wires the actual resolution).

### 5B — Maven Multi-Module

- [ ] Populate `MonorepoStandard::MavenMultiModule.spec()`: marker file
  `pom.xml`, `Field("modules")`. Roles: `[DefinesMembership,
  OrchestratesTasks, ManagesDependencies]`. `MembershipModel::RootExplicit`.
  `RootMembership::Never` (parent POM has `packaging=pom`).
  `NestingPolicy::AllowsNested`. `BinarySpec { name: "mvn", min_version: None,
  wrapper: Some(WrapperScript { posix: "mvnw", windows: "mvnw.cmd" }) }`.
- [ ] Add `detect_maven_workspace(root)` in a new
  `sniff/lib/src/filesystem/repo/maven.rs`. Parse `<modules><module>...</module>
  </modules>` via a minimal XML reader (`quick-xml` is already a dependency
  candidate — verify in `sniff/lib/Cargo.toml`; otherwise use a focused regex
  since the Maven schema is strict).
- [ ] Register `maven.rs` in `mod.rs`.
- [ ] Add a fixture `create_maven_workspace` and a degenerate variant.
- [ ] Add an integration test asserting
  `monorepo_layers[0].authority == MavenMultiModule`.

### 5C — .NET Solution

- [ ] Populate `MonorepoStandard::DotNetSolution.spec()`: marker file
  `*.sln` (or `*.slnx`), `MarkerContent::Existence`. Roles:
  `[DefinesMembership, OrchestratesTasks]`. `MembershipModel::RootExplicit`
  (the solution's `Project(...)` entries list literal `.csproj`/`.fsproj`
  paths). `RootMembership::Never`. `NestingPolicy::AllowsNested`.
  `BinarySpec { name: "dotnet", min_version: None, wrapper: None }`.
- [ ] Add `detect_dotnet_solution(root)` in a new
  `sniff/lib/src/filesystem/repo/dotnet.rs`. Walk the root for `*.sln` /
  `*.slnx` files; for each, parse the `Project("...") = "Name", "path.proj",
  ...` lines and resolve each project's directory.
- [ ] Register `dotnet.rs` in `mod.rs`.
- [ ] Add a fixture `create_dotnet_solution` and a degenerate variant.
- [ ] Add an integration test asserting
  `monorepo_layers[0].authority == DotNetSolution`.

**Validation checkpoint (Phase 5):**

- `cargo nextest run -p sniff` green.
- Each new variant has positive + degenerate coverage.
- Descriptor-table unit tests for the new variants pass.

## Phase 6: Polyglot Build Systems — Bazel, Pants, Buck2, RushStack

**Depends on:** Phase 3. **Parallelizable with:** Phase 4 and Phase 5.

These four introduce the `LeafMarkers { file }` membership model: packages are
discovered by walking for per-directory build files, not by parsing a root
manifest. The walk **is** the membership model.

### 6A — Bazel

- [ ] Populate `MonorepoStandard::Bazel.spec()`: marker files `WORKSPACE` /
  `WORKSPACE.bazel` (root) and `BUILD` / `BUILD.bazel` (leaf). Roles:
  `[DefinesMembership, OrchestratesTasks, ManagesDependencies]`.
  `MembershipModel::LeafMarkers { file: "BUILD" }` (detector also accepts
  `BUILD.bazel`). `RootMembership::Never`. `NestingPolicy::IgnoresNested` (a
  nested `WORKSPACE` starts a separate workspace; its subtree is ignored by
  the parent). `BinarySpec { name: "bazel", min_version: None, wrapper:
  Some(WrapperScript { posix: "bazelw", windows: "bazelw.bat" }) }` (the
  wrapper is `bazelisk`-style; record the conventional names).
- [ ] Add `detect_bazel_workspace(root)` in a new
  `sniff/lib/src/filesystem/repo/bazel.rs`. Walk the tree with
  `ignore::WalkBuilder`; for each directory containing `BUILD` or
  `BUILD.bazel`, record a package rooted there with
  `PackageProvenance::LeafMarkers`. Stop descending into any subtree that
  contains a nested `WORKSPACE` / `WORKSPACE.bazel` (record the nested root as
  a separate `DetectedStandard` at `DetectionConfidence::MarkerConfirmed`).
- [ ] Register `bazel.rs` in `mod.rs`.
- [ ] Add a fixture `create_bazel_workspace` (root `WORKSPACE`, nested
  `BUILD` files in `a/` and `b/`, plus a nested `WORKSPACE` in `nested/` with
  its own `BUILD`) and assert both layers are detected and the nested
  subtree is excluded from the parent's package list.
- [ ] Add a degenerate variant (`WORKSPACE` present, zero `BUILD` files) and
  assert `is_monorepo == false`.

### 6B — Pants

- [ ] Populate `MonorepoStandard::Pants.spec()`: marker files `pants.toml`
  (root) and `BUILD.pants` (leaf). Roles: `[DefinesMembership,
  OrchestratesTasks, ManagesDependencies]`. `MembershipModel::LeafMarkers {
  file: "BUILD.pants" }`. `RootMembership::Never`.
  `NestingPolicy::AllowsNested`. `BinarySpec { name: "pants", min_version:
  None, wrapper: None }`.
- [ ] Add `detect_pants_workspace(root)` (a focused `pants.rs` or share
  `polyglot.rs`). Same leaf-walk strategy as Bazel with the Pants-specific
  marker files.
- [ ] Add a fixture and a degenerate variant.
- [ ] Add an integration test asserting
  `monorepo_layers[0].authority == Pants`.

### 6C — Buck2

- [ ] Populate `MonorepoStandard::Buck2.spec()`: marker files `BUCK` (root
  optional) and `BUCK` / `TARGETS` (leaf). Roles: `[DefinesMembership,
  OrchestratesTasks, ManagesDependencies]`. `MembershipModel::LeafMarkers {
  file: "BUCK" }` (detector also accepts `TARGETS`). `RootMembership::Never`.
  `NestingPolicy::AllowsNested`. `BinarySpec { name: "buck2", min_version:
  None, wrapper: None }`.
- [ ] Add `detect_buck2_workspace(root)` with the same leaf-walk strategy.
- [ ] Add a fixture and a degenerate variant.
- [ ] Add an integration test asserting
  `monorepo_layers[0].authority == Buck2`.

### 6D — Rush Stack

- [ ] Populate `MonorepoStandard::RushStack.spec()`: marker file
  `rush.json`, `Field("projects")`. Roles: `[DefinesMembership,
  OrchestratesTasks]`. `MembershipModel::RootExplicit` (the `projects` array
  lists `{ projectFolder, packageName }` objects). `RootMembership::Never`.
  `NestingPolicy::AllowsNested`. `BinarySpec { name: "rush", min_version:
  None, wrapper: None }`.
- [ ] Add `detect_rush_workspace(root)` in `npm.rs` (Rush is a JS-family
  orchestrator). Parse `rush.json` via `serde_json`; for each entry in
  `projects`, resolve `projectFolder` and call `create_package`.
- [ ] Add a fixture `create_rush_workspace` and a degenerate variant.
- [ ] Add an integration test asserting
  `monorepo_layers[0].authority == RushStack`.

**Validation checkpoint (Phase 6):**

- `cargo nextest run -p sniff` green.
- Bazel's nested-`WORKSPACE` behavior is covered by a dedicated test.
- Each leaf-marker variant has a test that proves the leaf walk discovers
  packages in subdirectories without a root manifest list.

## Phase 7: Binary Resolution, Advisory Templates, and Lockfile Provenance

**Depends on:** Phase 4–6 (so every `BinarySpec` is populated).
**Parallelizable with:** Phase 8 (CLI output) **partially** — the CLI
rendering can proceed in parallel for the topology fields, but the
`ResolvedBinary` rendering in CLI depends on this phase.

Wire the static `BinarySpec` metadata to the existing
`LanguagePackageManager` registry and `ExecutableIndex` so each
`DetectedStandard` carries a `ResolvedBinary` (or `None`). Populate the
advisory `InvocationTemplate`s. Add the `Lockfile` provenance tier for the
ecosystems where it gives the biggest fidelity-per-effort win (pnpm, uv,
Cargo). No subprocess is ever executed.

- [ ] For every variant with a populated `BinarySpec`, populate the
  `enumerate_packages`, `run_in_package`, and `run_across_all`
  `InvocationTemplate`s on the spec using the `Token` enum
  (`Lit`, `Package`, `Task`, `AllPackages`). Examples:
  - Cargo: `cargo metadata --no-deps --format-version 1`,
    `cargo build -p {Package}`, `cargo build --workspace`.
  - pnpm: `pnpm --filter {Package} {Task}`, `pnpm -r {Task}`.
  - Go: `go work edit -json`, `go test ./{Package}/...`, `go test ./...`.
- [ ] Add `resolve_acting_binary(standard, root, executable_index) ->
  Option<ResolvedBinary>` in `sniff/lib/src/filesystem/repo/standard.rs`.
  Strategy:
  1. If `spec().binaries[0].wrapper` is `Some`, look for the wrapper script
     at `root.join(wrapper.posix)` (POSIX) or `root.join(wrapper.windows)`
     (Windows). If present → `BinarySource::Wrapper`.
  2. Otherwise query `ExecutableIndex::find_with_source(name)` (already
     exercised by `test_executable_index_parity_with_which_for_common_programs`).
     If found → `BinarySource::Path`.
  3. If neither → `BinarySource::Missing` (return `None`; do **not** error).
- [ ] When `BinarySpec.min_version` is `Some`, compare against the resolved
  version string (best-effort semver parse; fall back to a prefix match on
  the major). Populate `satisfies_min_version: Option<bool>`. A missing or
  unparseable version yields `None`, never `Some(false)` unless the parse
  succeeded and the comparison was conclusive.
- [ ] Thread `resolve_acting_binary` into `detect_repo_inner_with_shared` so
  every `DetectedStandard.binary` is populated (or `None`) before the
  `RepoInfo` is returned.
- [ ] **Lockfile provenance tier** (fast-follow, prioritized pnpm → uv →
  Cargo). For each:
  - **pnpm** (`pnpm-lock.yaml`): parse `importers:` keys; they are the
    resolved member paths. Compare against the manifest-derived package set.
    When they agree, upgrade `provenance` to `Lockfile`. When they disagree,
    keep the manifest as authority (`Globbed`) and attach a
    `lockfile_match: Option<bool>` detail to the `MonorepoLayer` (additive
    field; serde-skipped when `None`).
  - **uv** (`uv.lock`, TOML): parse the `workspace.members` entries.
  - **Cargo** (`Cargo.lock`): corroborate the globbed member dirs by checking
    each has a `Cargo.toml` whose `[package].name` appears in the lockfile's
    `[[package]]` table.
- [ ] Add the additive `lockfile_match: Option<bool>` field to
  `MonorepoLayer` (serde-skipped when `None`) per the spec's "stale lockfile
  must not delete packages" rule.
- [ ] Add unit tests for `resolve_acting_binary` using the existing
  `ExecutableIndex` abstraction with **synthetic** entries (per spec: "No
  package detection test may require `cargo`, `pnpm`, `go`, `gradle`, or any
  other external monorepo binary to be installed").
- [ ] Add a lockfile-parity test for pnpm: construct a fixture whose
  `pnpm-lock.yaml` `importers:` agree with the manifest globs and assert
  `monorepo_layers[0].provenance == Lockfile`.
- [ ] Add a lockfile-drift test for pnpm: construct a fixture whose
  `pnpm-lock.yaml` is missing a member present in the manifest and assert
  `provenance == Globbed` (manifest remains authority) **and**
  `lockfile_match == Some(false)`.

**Validation checkpoint (Phase 7):**

- `cargo nextest run -p sniff` green; no test requires an external monorepo
  binary (all binary-resolution tests use synthetic `ExecutableIndex`
  entries).
- `sniff repo --json` on the rusty-biscuit repo now includes a
  `monorepo_standards[].binary` object whose `source` is `Path` or `Wrapper`
  when the tool is installed.
- The advisory templates are present in the JSON (`enumerate_packages`,
  `run_in_package`, `run_across_all`) but sniff itself never executes them.

## Phase 8: CLI Output, JSON Surfaces, and Deprecation

**Depends on:** Phase 7.
**Parallelizable with:** Nothing — this is the closing phase.

Expose the new metadata through the CLI's existing `Renderable` components
and JSON builders. Mark `MonorepoTool` as `#[deprecated]` **only after** every
repo command and JSON snapshot exercises the new fields. Update docs and the
`sniff` skill.

- [ ] Extend `sniff/cli/src/output/filesystem/repo.rs::format_monorepo_tool`
  to delegate to `format_monorepo_standard` when the new fields are populated.
  Render the primary authority's `display_name` and, when there are
  orchestrators, append `<dim> + Nx</dim>` etc.
- [ ] Add a `format_monorepo_layer` helper that renders the
  authority-vs-orchestrator relationship and the package count per layer, using
  `Prose` + `UnorderedList` from `biscuit-terminal` (per the context's
  rendering rules). Reserve `stdout` for main content; `stderr` only for
  diagnostics.
- [ ] When `repo.monorepo_layers.len() > 1`, render each layer as its own
  section under the existing `Repository` heading. When exactly one layer,
  fold the new metadata into the existing one-liner so the common case is not
  noisier.
- [ ] Extend `sniff/cli/src/output/repo_json.rs::structure_value` so the
  `structure` JSON scope includes `monorepo_layers` and `monorepo_standards`
  when non-empty. Use the `#[serde(default, skip_serializing_if =
  "Vec::is_empty")]` attributes so the keys are absent (not `null`) on
  non-monorepo repos.
- [ ] Extend `build_aggregate_value` so the `structure` child inside the bare
  `sniff repo --json` aggregate automatically carries the new keys (it already
  delegates to `structure_value`, so this should be a no-op once
  `structure_value` is updated — verify with a test).
- [ ] **Do not** add focused leaves (`sniff repo standards`, `sniff repo
  layers`) in this feature (spec open-question option 1: defer).
- [ ] **Do not** change `sniff repo is-monorepo` or `sniff repo package-count`
  contracts. They already read `repo.is_monorepo` / `repo.packages.len()`,
  which are now populated by the honest predicate from Phase 3.
- [ ] Add CLI snapshot tests (using `insta` with `NO_COLOR=1`) for:
  - A single-authority Cargo monorepo (text + JSON).
  - A multi-layer repo (Cargo + pnpm side by side at the root).
  - An authority + orchestrator repo (pnpm + Nx).
  - A degenerate repo (`[workspace]` with no members) → text says "Single
    package", JSON has no `monorepo_*` keys.
- [ ] After every repo CLI snapshot is green and every JSON consumer path
  exercises the new fields, add `#[deprecated(note = "Use MonorepoStandard
  via RepoInfo::monorepo_layers instead")]` to `MonorepoTool` in `types.rs`.
  Update `clippy::deprecated` allowlists so the library compiles cleanly.
- [ ] Audit every remaining `MonorepoTool::` reference in `sniff/lib` and
  `sniff/cli` (the `grep` earlier in this plan found them). Keep the legacy
  field populated until a follow-up spec removes it; this phase only marks it
  deprecated.
- [ ] Update the `sniff` skill (`.opencode/skill/sniff/SKILL.md`) with a
  section on the new topology types and the authority-vs-orchestrator model.
- [ ] Update `sniff/lib/README.md` and `sniff/cli/README.md` with the new JSON
  keys and the deprecation note for `monorepo_tool` / `workspace_tools`.

**Validation checkpoint (Phase 8):**

- `cargo nextest run -p sniff --lib --bins` green.
- `cargo nextest run -p sniff-cli` green.
- `just lint sniff sniff-cli` clean (clippy + fmt).
- `just doctest sniff sniff-cli` green.
- `sniff repo --json | jq 'keys'` on the rusty-biscuit repo includes
  `structure`, and `structure.monorepo_layers` is a non-empty array whose
  first entry has `authority == "cargo-workspace"`.
- `sniff repo --json | jq '.structure.monorepo_tool'` still returns
  `"cargo_workspace"` (legacy contract preserved).
- The `MonorepoTool` enum carries a `#[deprecated]` attribute and the build
  emits no new warnings inside `sniff/lib` or `sniff/cli` beyond the
  intentional deprecation site.

## Cross-Cutting Constraints (apply to every phase)

- **No subprocess for package detection.** Every test must be runnable on a
  host without `cargo`, `pnpm`, `go`, `gradle`, etc. installed. Binary
  availability tests use synthetic `ExecutableIndex` entries.
- **stdout vs stderr.** When the CLI renders new topology output, `stdout`
  carries the main content (including `--json` payloads); `stderr` carries
  only diagnostics.
- **Additive JSON.** The legacy `monorepo_tool` and `workspace_tools` keys
  remain until a separate cleanup spec removes them. New keys use
  `#[serde(default, skip_serializing_if = ...)]` so old consumers see no
  diff on repos that do not exercise the new model.
- **Kebab-case ids.** Every `MonorepoStandard` variant serializes as
  kebab-case; `spec().id` matches the serde wire value.
- **Rustdoc conventions.** New `///` docs follow the monorepo's H2 section
  order (`## Examples`, `## Returns`, `## Errors`, `## Panics`, `## Safety`,
  `## Notes`). No `# H1` inside `///` blocks.
- **US English** for all symbol names and documentation.

## Out of Scope (deferred by the spec)

- Removing `MonorepoTool` entirely (needs a separate cleanup spec).
- SwiftPM (`SwiftPackage` variant) — deferred per open-question option 3.
- Focused CLI leaves (`sniff repo standards`, `sniff repo layers`).
- Versioning / release tooling (Changesets, Rush change files, `lerna
  publish`).
- Executing monorepo binaries for enumeration — `InvocationTemplate`s
  describe the commands for consumers, sniff itself never runs them.
