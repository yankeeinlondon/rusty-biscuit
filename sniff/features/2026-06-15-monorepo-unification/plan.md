---
agent: open_code/zai-coding-plan/glm-5.2
phases: 8
created: 2026-06-15
start_phase: 1
yolo: "true"
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - sniff/features/2026-06-15-monorepo-unification/plan.md
docs_created_during_phase_1:
  - sniff/features/2026-06-15-monorepo-unification/phase-1-audit.md
skills_files_updated_during_phase_1: []
packages: []
---

# Monorepo Type Unification — Execution Plan

## Goal

Delete the legacy `MonorepoTool` and `PackageDiscoverySource` surfaces,
collapsing the duplicated package representation (`LayerPackage` ↔ `Package`)
into a single canonical `Package` catalog that carries its own `standard` and
`provenance`. This is the **deliberate breaking step** the additive
[improved-monorepo-capture](../2026-06-15-improved-monorepo-capture/plan.md)
feature pointed at: the PascalCase `monorepo_tool` / `workspace_tools` JSON keys
and the `discovery_sources` array disappear, replaced by the kebab-case
topology model (`monorepo_layers[].authority`) and per-package
`standard` / `provenance`.

The library owns all detection and business logic; the CLI only reports
library-provided facts. Package detection never executes a subprocess.

- Spec: [`spec.md`](./spec.md)
- Background research: [`../../docs/research/monorepo.md`](../../docs/research/monorepo.md)
- Use the `sniff`, `cli`, `rust`, `rust-devops`, and `monorepos` agent skills.

## Fixed Decisions (from the reviewed spec)

- **D1** — Delete `MonorepoTool` entirely. `RepoInfo.monorepo_tool` and
  `RepoInfo.workspace_tools` are removed with **no replacement field**. The
  repo-level "primary tool" one-liner the CLI renders is re-derived from
  `monorepo_layers[0].authority.spec().display_name`. The PascalCase wire values
  (`"CargoWorkspace"`) are gone; the kebab-case `monorepo_layers[].authority`
  (`"cargo-workspace"`) is the sole surface.
- **D2** — Replace `Package.discovery_sources: Vec<PackageDiscoverySource>` with
  two scalar fields: `standard: MonorepoStandard` (the membership authority) and
  `provenance: PackageProvenance` (how the boundary was derived). Both are lifted
  from `LayerPackage` onto the canonical `Package`. A single authority +
  provenance does not lose information: the legacy `Vec` was almost always
  `[authority, manifest_scan]` duplicates the layer model already represents as
  one authority with a provenance tier. A parity audit (Phase 1) confirms this.
- **D3** — `RepoInfo.packages` is the canonical package catalog.
  `MonorepoLayer.packages` becomes `Vec<PathBuf>` holding **repo-relative**
  package paths (not layer-root-relative) that each resolve to exactly one
  `RepoInfo.packages[].relative` entry. The `LayerPackage` struct is deleted.
- **D4** — Keep `PackageEcosystem`. It is a property of the *individual package*
  (inferred from its own manifest, available in cheap `structure()` mode),
  distinct from both `spec().primary_language` (a property of the *standard*) and
  `Package.primary_language` (only available in rich `full()` mode). A doc
  comment records this distinction.
- **Hard-break (open question resolution)** — Remove the legacy JSON keys
  outright. The additive coexistence period *was* improved-monorepo-capture;
  there is no second deprecation window.
- **claudine compatibility** — Claudine consumes the sniff **library**
  in-process (`sniff::detect_with_plan` → `RepoInfo` field access), not the CLI
  JSON stream. Its `RepoContext.monorepo_tool` is therefore a **compile-time**
  dependency that must migrate to `monorepo_layers` *before* the field is deleted
  from `RepoInfo` (Phase 3 precedes Phase 6). A retained
  `{{project.monorepo_tool}}` template alias, if kept for one release, must be
  derived from `monorepo_layers` and documented as deprecated.

## Precondition (hard gate — read before starting)

This feature **may proceed to implementation phases (2+)** because the three
blocking conditions below have been verified as MET. The parent
`improved-monorepo-capture` feature has landed (its artifacts are now in
`sniff/features/_completed/2026-06-15-improved-monorepo-capture/`).

| Condition | Status | Evidence |
|-----------|--------|----------|
| Every `sniff repo` subcommand / JSON snapshot exercises `monorepo_layers` / `monorepo_standards` | **MET** | `format_monorepo_layer` exists at `sniff/cli/src/output/filesystem/repo.rs:186`; `render_repo_section` uses it for multi-layer repos (`repo.rs:559-564`, `repo.rs:965-971`); `repo_json.rs` tests assert `monorepo_standards` and `monorepo_layers` are present (`repo_json.rs:2489-2494`, `repo_json.rs:2561-2569`). |
| `MonorepoTool` carries `#[deprecated(note = "Use MonorepoStandard via RepoInfo::monorepo_layers instead")]` | **MET** | `sniff/lib/src/filesystem/repo/types.rs:18` has the required `#[deprecated]` attribute. |
| `Lockfile` / `Globbed` / `Explicit` / `LeafMarkers` provenance tiers populated on real layers | **MET** | `Globbed` and `Lockfile` are asserted on real Cargo/pnpm/uv layers in `sniff/lib/tests/integration.rs:664-746`; `Explicit` is produced by Go/Maven/Gradle/DotNet `RootExplicit` membership models and exercised in `tests/integration.rs:980`; `LeafMarkers` is produced by the Bazel/Pants/Buck2 detectors and exercised in `tests/integration.rs:1127-1248`. |

**Consequence:** execution of Phases 2–8 is unblocked. The parity audit and
decision tasks in Phase 1 have been completed and are recorded below.

## Key Files

| Area | File | Role |
|------|------|------|
| Types (legacy) | `sniff/lib/src/filesystem/repo/types.rs` | `MonorepoTool`, `PackageDiscoverySource`, `RepoInfo.monorepo_tool`/`workspace_tools`, `Package.discovery_sources`, `PackageEcosystem` |
| Types (new) | `sniff/lib/src/filesystem/repo/standard.rs` | `MonorepoLayer`, `LayerPackage` (to delete), `PackageProvenance`, `MonorepoStandard` |
| Topology | `sniff/lib/src/filesystem/repo/topology.rs` | `standard_for_tool`, `build_monorepo_layers`, `DetectorOutcome` |
| Detection | `sniff/lib/src/filesystem/repo/detection.rs` | `create_package`, `discovery_source_for_tool`, `collect_repo_info`, `merge_package_into`, `detect_repo_inner_with_shared` |
| Glob expander | `sniff/lib/src/filesystem/repo/glob.rs` | `expand_membership_globs` (takes `tool: MonorepoTool`) |
| Manifest index | `sniff/lib/src/filesystem/repo/manifest_index.rs` | `discover_packages_*` (take `tool` + `discovery_source`) |
| Detectors | `sniff/lib/src/filesystem/repo/{cargo,npm,nx_turbo,go,uv,gradle,maven,dotnet}.rs` | Each returns `RepoInfo` with `monorepo_tool`/`workspace_tools` |
| Re-exports | `sniff/lib/src/filesystem/repo/mod.rs`, `sniff/lib/src/filesystem/mod.rs` | Public surface (`MonorepoTool`, `LayerPackage`, `PackageDiscoverySource`) |
| CLI text | `sniff/cli/src/output/filesystem/repo.rs` | `format_monorepo_tool`, `render_repo_section` (lines 469, 871) |
| CLI JSON | `sniff/cli/src/output/repo_json.rs` | `structure_value`, `build_aggregate_value` |
| Claudine env | `claudine/lib/src/events/environment.rs` | `RepoContext.monorepo_tool`, `From<SniffResult>` |
| Claudine templates | `claudine/lib/src/dispatch/template.rs` | `EnvMonorepoTool`, `project.monorepo_tool` |
| Claudine expressions | `claudine/lib/src/dispatch/expression.rs` | `project.monorepo_tool` lookup |
| Docs | `sniff/lib/README.md`, `sniff/cli/README.md`, `sniff/docs/cli/repo_structure.md`, `claudine/lib/README.md`, `claudine/docs/topics/*` | JSON key docs, template var docs |
| Skill | `.opencode/skill/sniff/SKILL.md` | Topology model documentation |

## Phase 1: Precondition Gate and Parity Audit

**Depends on:** Nothing.
**Parallelizable with:** Nothing (every later phase consumes the audit + decisions).

Verify the hard-gate precondition, produce the complete parity list of every
legacy-type read, and lock the two open design questions the spec leaves to
implementation. The audit deliverable is the reference every later phase checks
against; the decisions are the contract the deletions honor.

### 1A — Precondition verification (hard gate)

- [x] Confirm `MonorepoTool` in `sniff/lib/src/filesystem/repo/types.rs` carries
  `#[deprecated(note = "Use MonorepoStandard via RepoInfo::monorepo_layers instead")]`.
  If absent, parent Phase 8 has not landed — **stop and coordinate**.
- [x] Confirm the CLI renders `monorepo_layers` / `monorepo_standards` in every
  `sniff repo` subcommand and JSON snapshot (a `format_monorepo_layer` helper or
  equivalent exists). If absent, parent Phase 8 has not landed — **stop and
  coordinate**.
- [x] Confirm `PackageProvenance::{Globbed, Explicit, LeafMarkers, Lockfile}` are
  each populated on at least one real layer across the test fixtures. If
  `LeafMarkers` or `Lockfile` are unpopulated, record which are missing;
  coordinate with the parent feature. Note: no current `PackageDiscoverySource`
  variant maps to `LeafMarkers` or `Lockfile`, so their absence does not impair
  *replacement fidelity* — but the spec treats it as blocking, so surface the
  gap explicitly before proceeding.

### 1B — Parity audit (the reference list)

- [x] Run `git grep -n "MonorepoTool" -- sniff/lib sniff/cli` and record every
  hit (definitions, constructors, match arms, doc examples, test literals).
  This is the deletion checklist for Phases 5–6.
- [x] Run `git grep -n "PackageDiscoverySource\|discovery_sources\|discovery_source_for_tool" -- sniff/lib sniff/cli`
  and record every hit. This is the deletion checklist for Phase 5.
- [x] Run `git grep -n "monorepo_tool\|workspace_tools" -- sniff claudine` across
  `.rs` and `.md` files. Record each consumer (in-process field reads vs.
  documented JSON keys vs. template variables).
- [x] Run `git grep -n "LayerPackage" -- sniff` and record every construction
  site (topology builder, test helpers, re-exports). This is the checklist for
  Phase 4.
- [x] Identify every `Package { ... }` struct literal that sets
  `discovery_sources` (found in: `detection.rs` tests, `types.rs` tests,
  `recent_commits.rs`, `darkmatter/.../capture.rs`, `sniff/cli` test modules,
  `claudine/cli/.../env.rs`). These are the mechanical update sites when the
  field changes shape.

### 1C — Lock open design questions

- [x] **Manifest-scan provenance.** Packages discovered by
  `discover_packages_from_index` (nested packages not declared in any workspace
  member list) currently carry `PackageDiscoverySource::ManifestScan`. Decide
  their `PackageProvenance`: **recommendation — add a new
  `PackageProvenance::ManifestScan` variant** to `standard.rs`, semantically
  distinct from `LeafMarkers` (which the parent reserves for Bazel/Pants/Buck2
  `BUILD` files). `ManifestScan` signals "no membership authority; found by
  walking for per-directory manifests." Record the decision; Phase 2 implements
  it. If the audit shows `LeafMarkers` is unambiguously equivalent in this repo,
  reuse it instead — but only if a doc comment can keep the two call sites
  distinguishable.
- [x] **D2 information-loss confirmation.** Audit the rusty-biscuit repo's
  pre-unification `discovery_sources` values. Confirm that every multi-element
  `Vec` is the `[authority, ManifestScan]` duplicate pattern the spec predicts,
  not a genuine multi-authority case the single-scalar model would flatten. If a
  genuine multi-authority case is found, escalate before Phase 5.

**Validation checkpoint:**

- The precondition table is filled with pass/fail per condition, with file:line
  evidence.
- The parity audit list is committed alongside this plan (or in a scratch doc the
  implementation team references).
- Both open questions have a recorded decision with rationale.

## Phase 1 Audit Results

All three hard-gate preconditions are **MET**. The complete parity audit and the
locked design decisions are recorded in
[`phase-1-audit.md`](./phase-1-audit.md). Summary:

- **Manifest-scan provenance decision:** Add `PackageProvenance::ManifestScan`
  (serde `"manifest-scan"`) to `sniff/lib/src/filesystem/repo/standard.rs` for
  packages discovered by `discover_packages_from_index` without a membership
  authority. It is semantically distinct from `LeafMarkers`.
- **D2 confirmation:** Every multi-element `discovery_sources` array in the
  rusty-biscuit repo is `[authority, manifest_scan]`; no genuine multi-authority
  case exists.
- **Parity hit counts:** `MonorepoTool` 123 hits; `PackageDiscoverySource` /
  `discovery_sources` / `discovery_source_for_tool` 61 hits;
  `monorepo_tool` / `workspace_tools` 234 hits; `LayerPackage` 35 hits;
  `Package { ... discovery_sources: ... }` literals 18 hits.
- **Validation:** `just test` in `sniff/` passed (690 passed, 2 skipped);
  `just lint` in `sniff/` was clean.

## Phase 2: Promote `standard` + `provenance` onto `Package` (additive)

**Depends on:** Phase 1 (provenance decision locked).
**Parallelizable with:** Phase 3 (claudine migration) — the two touch disjoint
files and both compile against the existing `RepoInfo`.

Make `Package` the canonical carrier of `standard` and `provenance` by adding
the two fields and threading them through every package construction path. This
phase is **purely additive**: the legacy `discovery_sources` field stays, the
legacy `MonorepoTool` stays, and both old and new representations coexist so
compilation is green at every step. No JSON contract changes yet.

- [ ] If Phase 1C chose a new variant, add `PackageProvenance::ManifestScan` to
  `sniff/lib/src/filesystem/repo/standard.rs` (kebab-case serde:
  `"manifest-scan"`). Add a unit test asserting its wire value and that
  `MonorepoStandard::Unknown.spec()` membership is consistent with it.
- [ ] Add two fields to `Package` in `types.rs`:
  ```rust
  pub standard: MonorepoStandard,
  pub provenance: PackageProvenance,
  ```
  Derive `Default` via `MonorepoStandard::Unknown` and
  `PackageProvenance::Explicit` (or the manifest-scan variant if added) so
  existing `Package::default()` and `..Package::default()` literals compile
  unchanged. Import `MonorepoStandard` / `PackageProvenance` into `types.rs`.
- [ ] Change `create_package` in `detection.rs` to accept
  `standard: MonorepoStandard` and `provenance: PackageProvenance` (replacing or
  supplementing the current `tool` + `discovery_source` params). Populate the two
  new `Package` fields from them. **Keep the legacy params for now** so callers
  compile; the full signature cleanup lands in Phases 5–6.
- [ ] Change `expand_membership_globs` in `glob.rs` to accept `standard` +
  `provenance` alongside the existing `tool` param, forwarding them to
  `create_package`. Derive `provenance` from the standard's
  `membership_provenance()` inside the expander when the caller does not override
  it.
- [ ] Change the `discover_packages_*` functions in `manifest_index.rs` to accept
  `standard` + `provenance`, forwarding to `create_package`. The nested-package
  discovery call in `detect_repo_inner_with_shared` (line ~281) passes
  `MonorepoStandard::Unknown` + the manifest-scan provenance from Phase 1C.
- [ ] Update `merge_package_into` in `detection.rs` so that when two package
  records merge, a non-`Unknown` incoming `standard` upgrades an `Unknown`
  existing one (the authority wins over the manifest-scan default). Provenance
  follows the surviving authority. This mirrors the current
  `discovery_sources` dedup semantics.
- [ ] Apply **D4**: add a rustdoc `## Notes` block to `PackageEcosystem` in
  `types.rs` stating why it is distinct from both `MonorepoStandard::spec().primary_language`
  (a property of the standard) and `Package.primary_language` (rich-mode-only file
  scan). Follow the repo's H2 section order; no `# H1`.
- [ ] Update the `detect_repo` doc example in `types.rs` (currently references
  `info.monorepo_tool` at line ~355) so it no longer demonstrates the soon-to-be
  deleted field — read `info.monorepo_layers` instead, or simplify to
  `info.is_monorepo` / `info.packages`.

**Validation checkpoint:**

- `cargo nextest run -p sniff` green — every existing test passes unchanged
  (the new fields default and the old fields remain).
- `cargo run -p sniff-cli -- repo packages --json | jq '.packages[0] | keys'`
  on the rusty-biscuit repo includes `standard` and `provenance` alongside the
  still-present `discovery_sources`.
- `git grep -n "standard:" sniff/lib/src/filesystem/repo/types.rs` shows the new
  `Package` field.

## Phase 3: Migrate claudine to the topology model

**Depends on:** Phase 1 (audit confirms the consumer surface).
**Parallelizable with:** Phase 2 (disjoint files; both compile against current
`RepoInfo`).
**Must complete before:** Phase 6 (which deletes `RepoInfo.monorepo_tool`).

Claudine reads `RepoInfo.monorepo_tool` in-process via `From<SniffResult>`.
Before that field can be deleted, claudine must source its repo classification
from `monorepo_layers` instead. This phase adds the new fields, repoints the
population logic, and updates the template/expression surface — all while the
legacy field still exists, so claudine compiles throughout.

- [ ] Extend `RepoContext` in `claudine/lib/src/events/environment.rs` with:
  ```rust
  pub monorepo_standard: Option<String>,
  pub monorepo_orchestrators: Vec<String>,
  ```
  Keep `monorepo_tool: Option<String>` for now (removed in the cleanup task
  below or retained as a deprecated alias — see decision below).
- [ ] Update `From<SniffResult> for EnvironmentContext` so `monorepo_standard`
  is populated from `r.monorepo_layers.first().authority` (serialized to its
  kebab-case wire value via `serde_json` or `spec().id`) and
  `monorepo_orchestrators` from the first layer's `orchestrators` Vec. Gate on
  `!r.monorepo_layers.is_empty()`.
- [ ] Add template variables `EnvMonorepoStandard` (`project.monorepo_standard`)
  and `EnvMonorepoOrchestrators` (`project.monorepo_orchestrators`) to
  `claudine/lib/src/dispatch/template.rs`, including `key()`, `description()`,
  `category()`, `all()`, and `resolve()` arms.
- [ ] Add the corresponding expression lookups for `project.monorepo_standard`
  and `project.monorepo_orchestrators` in `claudine/lib/src/dispatch/expression.rs`.
- [ ] **`{{project.monorepo_tool}}` disposition.** Either (a) derive it from
  `monorepo_standard` and mark it deprecated in docs + code
  (`#[deprecated]`-style doc note), keeping it for one release; or (b) remove it
  outright. Recommendation: **(a)** for one release, since template variables are
  user-facing strings — but the derived value must come from
  `monorepo_standard`, never from a deleted sniff field. If kept, update
  `resolve()` so `EnvMonorepoTool` reads `repo.monorepo_standard` (the new
  claudine field), not `repo.monorepo_tool`.
- [ ] Update `claudine/lib/src/events/environment.rs` tests: the
  `deserialize_with_git_context` test (line ~458) uses `"monorepo_tool":
  "cargoworkspace"` — add `monorepo_standard` / `monorepo_orchestrators` to the
  fixture and assert the new fields round-trip.
- [ ] Update `claudine/cli/src/commands/wrap/env.rs` test literals that
  construct `Package { discovery_sources: vec![], ... }` — these compile
  against the sniff `Package` struct and will break when Phase 5 removes the
  field. Defer the literal cleanup to Phase 5/8; here, only ensure the new
  claudine fields are tested.

**Validation checkpoint:**

- `cargo nextest run -p claudine` green.
- `cargo build -p claudine` succeeds with **zero** reads of
  `RepoInfo.monorepo_tool` or `RepoInfo.workspace_tools` in non-test code
  (`git grep -n "\.monorepo_tool\|\.workspace_tools" claudine/lib/src` returns no
  production hits; only deprecated-alias derivation if option (a) was chosen).
- A unit test asserts `monorepo_standard == Some("cargo-workspace")` when given a
  `RepoInfo` fixture with a Cargo authority layer.

## Phase 4: Collapse `LayerPackage` into repo-relative path references

**Depends on:** Phase 2 (`Package.standard` / `Package.provenance` exist and are
populated, so `LayerPackage`'s data is redundant).
**Parallelizable with:** Nothing in this phase changes the topology contract that
Phase 5–6 deletions rely on.

`MonorepoLayer.packages` currently holds `Vec<LayerPackage>`, duplicating the
`standard`, `provenance`, name, and path data already on `Package` (after Phase
2). Per **D3**, collapse it to `Vec<PathBuf>` (repo-relative paths) and delete
the `LayerPackage` struct. Every layer package path must resolve to exactly one
`RepoInfo.packages[].relative` entry.

- [ ] Change `MonorepoLayer.packages` in `standard.rs` from `Vec<LayerPackage>`
  to `Vec<PathBuf>`. Update the struct doc comment to state the paths are
  repo-relative (matching `Package.relative`) so JSON consumers can join layers
  to package details without normalizing path bases.
- [ ] Delete the `LayerPackage` struct from `standard.rs`.
- [ ] Update `build_monorepo_layers` in `topology.rs` so each layer's `packages`
  Vec is built from `outcome.packages[].relative` (repo-relative strings
  converted to `PathBuf`), not from `LayerPackage { .. }` literals. The
  `standard` and `provenance` are already on each `Package`; the layer no longer
  re-records them per package.
- [ ] Remove the `LayerPackage` re-export from
  `sniff/lib/src/filesystem/repo/mod.rs` and `sniff/lib/src/filesystem/mod.rs`.
- [ ] Add an invariant test in `topology.rs`: after `build_monorepo_layers`,
  every `layer.packages[].relative` resolves to exactly one entry in a synthetic
  `RepoInfo.packages` catalog — no dangling references, no package name joins
  required.
- [ ] Update the `layer_with` test helper and all topology unit tests to the new
  `Vec<PathBuf>` shape.

**Validation checkpoint:**

- `cargo nextest run -p sniff` green.
- `git grep -n "LayerPackage" sniff` returns zero hits (struct, re-exports, and
  test helpers all removed).
- `cargo run -p sniff-cli -- repo --json | jq '.repo.monorepo_layers[0].packages'`
  on the rusty-biscuit repo returns an array of path strings (e.g.
  `["sniff/lib", "sniff/cli", ...]`), not objects.

## Phase 5: Delete the `PackageDiscoverySource` surface

**Depends on:** Phase 2 (`standard` + `provenance` are the canonical
replacement) and Phase 4 (topology no longer depends on `LayerPackage`).
**Parallelizable with:** Nothing — this removes a public type.

Remove the `PackageDiscoverySource` enum, the `discovery_sources` field, and the
`discovery_source_for_tool` mapper. After this phase, `Package.standard` +
`Package.provenance` are the sole record of how a package was discovered. The
`MonorepoTool` enum intentionally survives this phase because `discovery_source_for_tool`
and several detectors still reference it — it is removed in Phase 6.

- [ ] Remove `discovery_sources: Vec<PackageDiscoverySource>` from `Package` in
  `types.rs`. Remove the field from every `Package { ... }` literal identified in
  the Phase 1B audit (`detection.rs` tests, `types.rs` tests,
  `recent_commits.rs`, `darkmatter/.../capture.rs`, `sniff/cli` test modules,
  `claudine/cli/.../env.rs`).
- [ ] Delete the `PackageDiscoverySource` enum from `types.rs` and its re-export
  from `sniff/lib/src/filesystem/repo/mod.rs`.
- [ ] Delete `discovery_source_for_tool` from `detection.rs`.
- [ ] Remove the `discovery_source` parameter from `create_package`,
  `expand_membership_globs`, and the `discover_packages_*` family in
  `manifest_index.rs`. The `provenance` parameter (added in Phase 2) is the
  replacement.
- [ ] Remove the `discovery_sources` merge loop from `merge_package_into` in
  `detection.rs` (lines ~835–839). The `standard`/`provenance` merge logic from
  Phase 2 already covers the authority-wins semantics.
- [ ] Remove the `PackageDiscoverySource::ManifestScan` argument from the nested
  `discover_packages_from_index` call in `detect_repo_inner_with_shared`
  (line ~286); the manifest-scan `PackageProvenance` variant from Phase 2
  replaces it.

**Validation checkpoint:**

- `cargo nextest run -p sniff --lib --bins` green.
- `cargo nextest run -p sniff-cli` green.
- `git grep -n "PackageDiscoverySource\|discovery_sources" sniff/lib sniff/cli`
  returns zero hits (a CHANGELOG entry, if any, is the only allowed exception).
- `cargo run -p sniff-cli -- repo packages --json | jq '.packages[0] | has("discovery_sources")'`
  returns `false`.

## Phase 6: Delete the `MonorepoTool` surface (the breaking cut)

**Depends on:** Phase 3 (claudine no longer reads `RepoInfo.monorepo_tool`),
Phase 5 (`discovery_source_for_tool` — the last function mapping `MonorepoTool`
to the deleted enum — is gone).
**Parallelizable with:** Nothing — this is the breaking JSON change.

Delete the `MonorepoTool` enum, the `RepoInfo.monorepo_tool` /
`workspace_tools` fields, the `standard_for_tool` bridge, and the
`format_monorepo_tool` CLI helper. Detectors report their `MonorepoStandard`
directly to the topology builder instead of routing through the legacy enum.
The PascalCase JSON keys (`monorepo_tool`, `workspace_tools`) disappear.

- [ ] Refactor every detector (`cargo.rs`, `npm.rs`, `nx_turbo.rs`, `gradle.rs`,
  `maven.rs`, `dotnet.rs`, `go.rs`, `uv.rs`) so it no longer constructs a
  `RepoInfo` with `monorepo_tool` / `workspace_tools`. Each detector returns its
  packages (tagged with `standard` + `provenance` via Phase 2) and its root; the
  outcome collector builds the `DetectorOutcome` directly. If keeping the
  `RepoInfo` return shape temporarily, set the legacy fields to `None` / empty —
  but the cleaner path is to return `(MonorepoStandard, PathBuf, Vec<Package>)`
  or reuse `DetectorOutcome` directly.
- [ ] Merge `collect_repo_info` and `collect_standard_outcome` in `detection.rs`
  into a single collector that takes `(Option<DetectorOutcome>)` and folds it
  into `packages` + `outcomes`. There is no longer a split between "legacy tool
  detectors" and "new standard detectors" — every detector is a standard
  detector.
- [ ] Delete `standard_for_tool` from `topology.rs` and its unit test
  (`standard_for_tool_round_trips_every_variant`).
- [ ] Remove `monorepo_tool` and `workspace_tools` from `RepoInfo` in
  `types.rs`. Remove the `let mut workspace_tools = Vec::new();` accumulation in
  `detect_repo_inner_with_shared` and the `monorepo_tool: workspace_tools.first().copied()`
  / `workspace_tools` fields from the final `RepoInfo` literal.
- [ ] Delete the `MonorepoTool` enum from `types.rs` and its re-export from
  `sniff/lib/src/filesystem/repo/mod.rs`.
- [ ] Remove the now-dead `tool: MonorepoTool` parameter from `create_package`,
  `expand_membership_globs`, and the `discover_packages_*` family (it survived
  Phase 5 because `discovery_source_for_tool` consumed it; that function is now
  gone).
- [ ] Delete `format_monorepo_tool` from `sniff/cli/src/output/filesystem/repo.rs`
  (lines 142–155).
- [ ] Re-derive the CLI "primary tool" one-liner in `render_repo_section`
  (lines 469–473 and 871–875) from `repo.monorepo_layers.first()`:
  `authority.spec().display_name`, appending
  `<dim> + {orchestrator.display_name}</dim>` for each orchestrator on the first
  layer. When no layer exists, fall back to `"Unknown"`.

**Validation checkpoint:**

- `cargo build` for the whole workspace succeeds (sniff + sniff-cli + claudine +
  darkmatter all compile).
- `cargo nextest run -p sniff --lib --bins` green.
- `git grep -n "MonorepoTool\|monorepo_tool\|workspace_tools\|format_monorepo_tool\|standard_for_tool" sniff/lib sniff/cli`
  returns zero hits (CHANGELOG excepted).
- `cargo run -p sniff-cli -- repo --json | jq '.repo | has("monorepo_tool"), has("workspace_tools")'`
  returns `false` for both.
- `cargo run -p sniff-cli -- repo` text output shows the authority display name
  (e.g. "Cargo Workspace") in the one-liner, sourced from `monorepo_layers`.

## Phase 7: JSON contract, CLI text rendering, docs, and skill

**Depends on:** Phase 6 (the legacy fields are gone; the new shapes are stable).
**Parallelizable with:** Phase 8 test-fixture work can begin in parallel once the
JSON shape is frozen, but snapshot regeneration belongs in Phase 8.

Align every external surface with the post-unification contract: JSON keys
match the spec's impact table, CLI text renders authority + orchestrators via
`biscuit-terminal` renderables, and all documentation reflects the breaking
change.

- [ ] Verify `structure_value` / `build_aggregate_value` in `repo_json.rs`
  serialize the updated `RepoInfo` / `Package` shapes correctly. The removed keys
  (`monorepo_tool`, `workspace_tools`, `discovery_sources`) must be **absent**
  (not `null`) — confirm the fields are gone from the structs (Phase 6) so serde
  cannot emit them. The new keys (`package.standard`, `package.provenance`) are
  always present; `monorepo_layers[].packages[]` are path strings.
- [ ] Add a CLI snapshot test (using `insta` with `NO_COLOR=1`) covering the
  post-unification text output for a Cargo monorepo: the one-liner reads
  "Cargo Workspace" from the authority layer, and the package list renders
  unchanged.
- [ ] Add a CLI snapshot for an authority + orchestrator repo (pnpm + Nx):
  the one-liner appends ` + Nx`.
- [ ] Update `sniff/lib/README.md`: remove the `monorepo_tool` / `workspace_tools`
  / `discovery_sources` documentation; document `package.standard`,
  `package.provenance`, and the now-canonical `monorepo_layers[].packages` path
  strings. Fix the doc example that prints `info.monorepo_tool`.
- [ ] Update `sniff/cli/README.md` and `sniff/docs/cli/repo_structure.md`: the
  JSON example (currently `"monorepo_tool": "Cargo"`, `"workspace_tools":
  ["Cargo"]` at lines 152–153) is replaced with the `monorepo_layers` shape and
  the new per-package `standard` / `provenance` keys.
- [ ] Update `claudine/lib/README.md`: document `{{project.monorepo_standard}}`
  and `{{project.monorepo_orchestrators}}`; mark `{{project.monorepo_tool}}`
  deprecated if the Phase 3 alias was retained, or remove it.
- [ ] Update `claudine/docs/topics/unified-events.md`,
  `configuring-actions.md`, and `log-reporting.md` wherever
  `monorepo_tool` appears in examples or tables.
- [ ] Update `.opencode/skill/sniff/SKILL.md` with a section on the unified
  topology model: `Package` is canonical, layers carry repo-relative path
  references, `MonorepoTool` / `PackageDiscoverySource` are gone.

**Validation checkpoint:**

- `just lint sniff sniff-cli claudine` clean (clippy + fmt).
- `just doctest sniff sniff-cli claudine` green.
- The spec's JSON / CLI contract impact table matches actual output:
  `sniff repo --json` has no `monorepo_tool`, `workspace_tools`, or
  `discovery_sources` keys; `package.standard` and `package.provenance` are
  present; `monorepo_layers[].packages[]` are path strings.

## Phase 8: Fixtures, snapshots, parity tests, and acceptance

**Depends on:** Phase 7 (JSON shape frozen).
**Parallelizable with:** Nothing — this is the closing validation gate.

Regenerate every snapshot, update every fixture, and add the parity + acceptance
tests that prove the unification did not silently change a package's owning
authority. This is the phase where the spec's "Testing and acceptance criteria"
section is satisfied in full.

- [ ] Update every `Package { ... }` and `RepoInfo { ... }` literal in
  `sniff/lib/tests/integration.rs`, `sniff/lib/tests/fixtures.rs`, and the CLI
  test modules so they compile against the final shapes (no `discovery_sources`,
  no `monorepo_tool` / `workspace_tools`; `standard` + `provenance` populated
  where the test asserts them).
- [ ] Regenerate all `insta` snapshots under `sniff/cli/tests/snapshots/` with
  `cargo nextest run -p sniff-cli -- --review` (or `INSTA_UPDATE=always`). Confirm
  the removed keys are absent and the new keys are present in every snapshot.
- [ ] Add a **parity test** over the rusty-biscuit repo: the package set and
  each package's owning authority (derived from `Package.standard`) match the
  pre-unification `discovery_sources`-derived view. No package silently changes
  owner. (Use the Phase 1B audit as the baseline.)
- [ ] Add the **"Nx delegates to pnpm" regression test**: a fixture with both
  `nx.json` and `pnpm-workspace.yaml` at the same root asserts that a package's
  `standard == MonorepoStandard::PnpmWorkspaces` (the authority), **not** `Nx` —
  proving the legacy category error is fixed, not preserved. The orchestrator
  `Nx` appears only on `monorepo_layers[0].orchestrators`.
- [ ] Add an **assertable-from-catalog test**: a package's owning standard and
  provenance are readable directly off `RepoInfo.packages` without consulting
  `monorepo_layers`. Conversely, every `monorepo_layers[].packages[]` path
  resolves to exactly one `RepoInfo.packages[].relative` entry.
- [ ] Confirm **no external monorepo binary** is required by any test (inherited
  constraint): `cargo nextest run -p sniff` passes on a host without `cargo`,
  `pnpm`, `go`, etc. on PATH (the existing synthetic-`ExecutableIndex` pattern).
- [ ] Confirm **stdout / stderr discipline**: `--json` output is valid JSON on
  stdout with nothing else; diagnostics (if any) go to stderr.
- [ ] Run the full PR gate: `just lint && just doctest && just test && just test-l2`
  from the `sniff/` directory (and the equivalent for claudine).

**Validation checkpoint (acceptance gate):**

- `git grep -n "MonorepoTool\|PackageDiscoverySource" sniff/lib sniff/cli`
  returns zero hits (CHANGELOG excepted).
- Every `sniff repo` JSON snapshot is green; the removed keys are absent (not
  `null`).
- A package's `standard` + `provenance` are assertable directly off
  `RepoInfo.packages` without consulting `monorepo_layers`.
- Every `monorepo_layers[].packages[]` entry resolves to exactly one
  `RepoInfo.packages[].relative` entry.
- The parity test passes: no package silently changed owner.
- The "Nx delegates to pnpm" test passes: `package.standard == PnpmWorkspaces`.
- claudine no longer parses `monorepo_tool` from sniff; any retained
  `{{project.monorepo_tool}}` alias is derived from `monorepo_layers`, documented
  as deprecated, and covered by a test.
- Terminal output uses `biscuit-terminal` renderables; stdout carries main
  content, stderr carries diagnostics only.

## Cross-Cutting Constraints (apply to every phase)

- **Breaking JSON, no deprecation window.** The additive coexistence period was
  improved-monorepo-capture. This feature removes the duplicated surface in one
  cut; there is no second bridge layer.
- **Compile-time claudine dependency.** Claudine consumes the sniff library
  in-process. Phase 3 (claudine migration) must land before Phase 6 (field
  deletion) so the workspace compiles at every commit.
- **No subprocess for package detection.** Every test must run on a host without
  `cargo`, `pnpm`, `go`, `gradle`, etc. Binary-availability tests use synthetic
  `ExecutableIndex` entries.
- **stdout vs stderr.** stdout carries the main content (including `--json`
  payloads, which must be valid JSON alone on stdout); stderr carries only
  diagnostics.
- **Kebab-case ids.** Every `MonorepoStandard` variant serializes as kebab-case;
  `spec().id` matches the serde wire value. `PackageProvenance` likewise.
- **Repo-relative paths.** `MonorepoLayer.packages` holds repo-relative paths
  (matching `Package.relative`), never layer-root-relative paths, so JSON
  consumers join layers to packages without path-base normalization.
- **Rustdoc conventions.** New/edited `///` docs follow the monorepo's H2 section
  order (`## Examples`, `## Returns`, `## Errors`, `## Panics`, `## Safety`,
  `## Notes`). No `# H1` inside `///` blocks. US English for all symbol names and
  documentation.
- **Comment discipline.** When a behavior-changing edit touches a symbol, pass
  over its `///` / `//` comments and fix or delete drifted ones in the same
  change. Comments that merely restate the implementation step-by-step are
  removed on sight.

## Out of Scope (deferred by the spec)

- Adding new `MonorepoStandard` variants or detectors — that is
  improved-monorepo-capture's remit (Phases 6–7 of the parent: Bazel, Pants,
  Buck2, RushStack, binary resolution, lockfile provenance).
- SwiftPM (`SwiftPackage`) — still deferred per the parent spec.
- Focused CLI leaves (`sniff repo standards`, `sniff repo layers`) — still
  deferred (parent spec open-question option 1).
- Executing monorepo binaries for enumeration — `InvocationTemplate`s describe
  commands for consumers; sniff itself never runs them.
