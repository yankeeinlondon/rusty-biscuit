# Phase 1 Parity Audit — Monorepo Type Unification

This document records the complete parity audit produced during Phase 1 of the
monorepo type unification feature. It is the reference checklist for Phases 2–8.

## Preconditions (all MET)

| Condition | Status | Evidence |
|-----------|--------|----------|
| Every `sniff repo` subcommand / JSON snapshot exercises `monorepo_layers` / `monorepo_standards` | **MET** | `format_monorepo_layer` exists at `sniff/cli/src/output/filesystem/repo.rs:186`; `render_repo_section` uses it for multi-layer repos (`repo.rs:559-564`, `repo.rs:965-971`); `repo_json.rs` tests assert `monorepo_standards` and `monorepo_layers` are present (`repo_json.rs:2489-2494`, `repo_json.rs:2561-2569`). |
| `MonorepoTool` carries `#[deprecated(note = "Use MonorepoStandard via RepoInfo::monorepo_layers instead")]` | **MET** | `sniff/lib/src/filesystem/repo/types.rs:18` has the required `#[deprecated]` attribute. |
| `Lockfile` / `Globbed` / `Explicit` / `LeafMarkers` provenance tiers populated on real layers | **MET** | `Globbed` and `Lockfile` are asserted on real Cargo/pnpm/uv layers in `sniff/lib/tests/integration.rs:664-746`; `Explicit` is produced by Go/Maven/Gradle/DotNet `RootExplicit` membership models and exercised in `tests/integration.rs:980`; `LeafMarkers` is produced by the Bazel/Pants/Buck2 detectors and exercised in `tests/integration.rs:1127-1248`. |

## Design decisions locked

1. **Manifest-scan provenance** — Add `PackageProvenance::ManifestScan` to
   `sniff/lib/src/filesystem/repo/standard.rs`, serializing as `"manifest-scan"`.
   It is the canonical provenance for packages discovered by
   `discover_packages_from_index` that are not declared by any membership
   authority. It is intentionally distinct from `LeafMarkers`
   (Bazel/Pants/Buck2 build files) and from authority-derived `Globbed` /
   `Explicit`.

2. **D2 information-loss confirmation** — Over the rusty-biscuit repo, every
   multi-element `discovery_sources` array is `[authority, manifest_scan]`. No
   genuine multi-authority packages were found. Replacing the `Vec` with a
   single `standard` + `provenance` does not flatten any real multi-ownership
   case.

## Parity hit summary

| Search | Hits | Scope |
|---|---|---|
| `MonorepoTool` | 123 | `sniff/lib` + `sniff/cli` |
| `PackageDiscoverySource` / `discovery_sources` / `discovery_source_for_tool` | 61 | `sniff/lib` + `sniff/cli` |
| `monorepo_tool` / `workspace_tools` | 234 | `sniff` + `claudine` |
| `LayerPackage` | 35 | `sniff` |
| `Package { ... discovery_sources: ... }` literals | 18 | repo-wide |

## `Package { ... discovery_sources: ... }` literal sites

These are the mechanical update sites when `discovery_sources` is removed in
Phase 5.

- `sniff/lib/src/filesystem/git/recent_commits.rs:1414,1444,1474,1504`
- `sniff/lib/src/filesystem/repo/detection.rs:1462,1500`
- `sniff/lib/src/filesystem/repo/types.rs:433,463`
- `sniff/cli/src/output/filesystem/mod.rs:1360`
- `sniff/cli/src/output/repo_json.rs:1082,1314,1637,1952`
- `claudine/cli/src/commands/wrap/env.rs:793,823,865,895`

`darkmatter/lib/src/markdown/compose/context/capture.rs` constructs `Package`
literals using `..Default::default()`, so it will continue to compile once the
field is removed.

## Full `MonorepoTool` hits

Command: `git grep -n "MonorepoTool" -- sniff/lib sniff/cli`

```
sniff/cli/src/lib.rs:8:// `MonorepoTool` is deprecated in favor of `MonorepoStandard` but the CLI
sniff/cli/src/output/filesystem/repo.rs:147:/// Format the legacy `MonorepoTool` enum for display.
sniff/cli/src/output/filesystem/repo.rs:149:fn format_legacy_monorepo_tool(tool: &sniff::filesystem::repo::MonorepoTool) -> &'static str {
sniff/cli/src/output/filesystem/repo.rs:150:    use sniff::filesystem::repo::MonorepoTool;
sniff/cli/src/output/filesystem/repo.rs:152:        MonorepoTool::CargoWorkspace => "Cargo Workspace",
sniff/cli/src/output/filesystem/repo.rs:153:        MonorepoTool::NpmWorkspaces => "npm Workspaces",
sniff/cli/src/output/filesystem/repo.rs:154:        MonorepoTool::PnpmWorkspaces => "pnpm Workspaces",
sniff/cli/src/output/filesystem/repo.rs:155:        MonorepoTool::YarnWorkspaces => "Yarn Workspaces",
sniff/cli/src/output/filesystem/repo.rs:156:        MonorepoTool::Nx => "Nx",
sniff/cli/src/output/filesystem/repo.rs:157:        MonorepoTool::Turborepo => "Turborepo",
sniff/cli/src/output/filesystem/repo.rs:158:        MonorepoTool::Lerna => "Lerna",
sniff/cli/src/output/filesystem/repo.rs:159:        MonorepoTool::Unknown => "Unknown",
sniff/cli/src/output/filesystem/repo.rs:167:/// populated, preserving the legacy `MonorepoTool` text only as a fallback.
sniff/lib/README.md:468:- `MonorepoTool` - **Deprecated**; use `MonorepoStandard` via `RepoInfo::monorepo_layers`
sniff/lib/src/filesystem/mod.rs:45:    MonorepoStandard, MonorepoStandardSpec, MonorepoTool, Package, PackageDiscoverySource,
sniff/lib/src/filesystem/repo/cargo.rs:13:use super::types::{MonorepoTool, RepoInfo};
sniff/lib/src/filesystem/repo/cargo.rs:71:        MonorepoTool::CargoWorkspace,
sniff/lib/src/filesystem/repo/cargo.rs:80:        MonorepoTool::CargoWorkspace,
sniff/lib/src/filesystem/repo/cargo.rs:93:        monorepo_tool: Some(MonorepoTool::CargoWorkspace),
sniff/lib/src/filesystem/repo/cargo.rs:94:        workspace_tools: vec![MonorepoTool::CargoWorkspace],
sniff/lib/src/filesystem/repo/detection.rs:46:    MonorepoTool, Package, PackageDiscoverySource, PackageEcosystem, PackageFiles,
sniff/lib/src/filesystem/repo/detection.rs:238:    // New membership authorities (Bun, uv, Go) have no legacy `MonorepoTool`
sniff/lib/src/filesystem/repo/detection.rs:368:                MonorepoTool::Unknown,
sniff/lib/src/filesystem/repo/detection.rs:409:    workspace_tools: &mut Vec<MonorepoTool>,
sniff/lib/src/filesystem/repo/detection.rs:448:/// `workspace_tools` list: Bun, uv, and Go have no [`MonorepoTool`] counterpart,
sniff/lib/src/filesystem/repo/detection.rs:1009:pub(crate) fn discovery_source_for_tool(tool: MonorepoTool) -> PackageDiscoverySource {
sniff/lib/src/filesystem/repo/detection.rs:1011:        MonorepoTool::CargoWorkspace => PackageDiscoverySource::CargoWorkspace,
sniff/lib/src/filesystem/repo/detection.rs:1012:        MonorepoTool::PnpmWorkspaces => PackageDiscoverySource::PnpmWorkspace,
sniff/lib/src/filesystem/repo/detection.rs:1013:        MonorepoTool::NpmWorkspaces => PackageDiscoverySource::NpmWorkspace,
sniff/lib/src/filesystem/repo/detection.rs:1014:        MonorepoTool::YarnWorkspaces => PackageDiscoverySource::YarnWorkspace,
sniff/lib/src/filesystem/repo/detection.rs:1015:        MonorepoTool::Nx => PackageDiscoverySource::Nx,
sniff/lib/src/filesystem/repo/detection.rs:1016:        MonorepoTool::Turborepo => PackageDiscoverySource::Turborepo,
sniff/lib/src/filesystem/repo/detection.rs:1017:        MonorepoTool::Lerna => PackageDiscoverySource::Lerna,
sniff/lib/src/filesystem/repo/detection.rs:1018:        MonorepoTool::Unknown => PackageDiscoverySource::ManifestScan,
sniff/lib/src/filesystem/repo/detection.rs:1339:    tool: MonorepoTool,
sniff/lib/src/filesystem/repo/detection.rs:1351:    tool: MonorepoTool,
sniff/lib/src/filesystem/repo/dotnet.rs:11:use super::types::{MonorepoTool, PackageDiscoverySource, RepoInfo};
sniff/lib/src/filesystem/repo/dotnet.rs:47:                MonorepoTool::Unknown,
sniff/lib/src/filesystem/repo/glob.rs:28:use super::types::{MonorepoTool, Package};
sniff/lib/src/filesystem/repo/glob.rs:52:    tool: MonorepoTool,
sniff/lib/src/filesystem/repo/go.rs:9:use super::types::{MonorepoTool, PackageDiscoverySource, RepoInfo};
sniff/lib/src/filesystem/repo/go.rs:33:            MonorepoTool::Unknown,
sniff/lib/src/filesystem/repo/gradle.rs:8:use super::types::{MonorepoTool, PackageDiscoverySource, RepoInfo};
sniff/lib/src/filesystem/repo/gradle.rs:37:            MonorepoTool::Unknown,
sniff/lib/src/filesystem/repo/manifest_index.rs:15:use super::types::{MonorepoTool, Package, PackageDiscoverySource};
sniff/lib/src/filesystem/repo/manifest_index.rs:297:    tool: MonorepoTool,
sniff/lib/src/filesystem/repo/manifest_index.rs:307:    tool: MonorepoTool,
sniff/lib/src/filesystem/repo/manifest_index.rs:327:    tool: MonorepoTool,
sniff/lib/src/filesystem/repo/manifest_index.rs:393:    tool: MonorepoTool,
sniff/lib/src/filesystem/repo/maven.rs:11:use super::types::{MonorepoTool, PackageDiscoverySource, RepoInfo};
sniff/lib/src/filesystem/repo/maven.rs:35:            MonorepoTool::Unknown,
sniff/lib/src/filesystem/repo/mod.rs:30:    MonorepoTool, Package, PackageDiscoverySource, PackageEcosystem, RepoInfo, detect_repo,
sniff/lib/src/filesystem/repo/nested.rs:47:use super::types::{MonorepoTool, Package, RepoInfo};
sniff/lib/src/filesystem/repo/nested.rs:142:    workspace_tools: &mut Vec<MonorepoTool>,
sniff/lib/src/filesystem/repo/nested.rs:303:    workspace_tools: &mut Vec<MonorepoTool>,
sniff/lib/src/filesystem/repo/nested.rs:451:/// outcome to `standard` regardless of the legacy [`MonorepoTool`] the
sniff/lib/src/filesystem/repo/nested.rs:461:    workspace_tools: &mut Vec<MonorepoTool>,
sniff/lib/src/filesystem/repo/npm.rs:13:use super::types::{MonorepoTool, PackageDiscoverySource, RepoInfo};
sniff/lib/src/filesystem/repo/npm.rs:136:        MonorepoTool::PnpmWorkspaces,
sniff/lib/src/filesystem/repo/npm.rs:144:        monorepo_tool: Some(MonorepoTool::PnpmWorkspaces),
sniff/lib/src/filesystem/repo/npm.rs:145:        workspace_tools: vec![MonorepoTool::PnpmWorkspaces],
sniff/lib/src/filesystem/repo/npm.rs:189:        MonorepoTool::Unknown,
sniff/lib/src/filesystem/repo/npm.rs:236:        MonorepoTool::NpmWorkspaces,
sniff/lib/src/filesystem/repo/npm.rs:244:        monorepo_tool: Some(MonorepoTool::NpmWorkspaces),
sniff/lib/src/filesystem/repo/npm.rs:245:        workspace_tools: vec![MonorepoTool::NpmWorkspaces],
sniff/lib/src/filesystem/repo/npm.rs:281:        MonorepoTool::YarnWorkspaces,
sniff/lib/src/filesystem/repo/npm.rs:289:        monorepo_tool: Some(MonorepoTool::YarnWorkspaces),
sniff/lib/src/filesystem/repo/npm.rs:290:        workspace_tools: vec![MonorepoTool::YarnWorkspaces],
sniff/lib/src/filesystem/repo/npm.rs:324:            MonorepoTool::Unknown,
sniff/lib/src/filesystem/repo/npm.rs:435:    tool: MonorepoTool,
sniff/lib/src/filesystem/repo/npm.rs:440:        MonorepoTool::PnpmWorkspaces => return "pnpm",
sniff/lib/src/filesystem/repo/npm.rs:441:        MonorepoTool::YarnWorkspaces => return "yarn",
sniff/lib/src/filesystem/repo/nx_turbo.rs:16:use super::types::{MonorepoTool, PackageDiscoverySource, RepoInfo};
sniff/lib/src/filesystem/repo/nx_turbo.rs:32:            MonorepoTool::Nx,
sniff/lib/src/filesystem/repo/nx_turbo.rs:41:        expand_membership_globs(root, &patterns, dialect, MonorepoTool::Nx, &lock_versions)
sniff/lib/src/filesystem/repo/nx_turbo.rs:46:            MonorepoTool::Nx,
sniff/lib/src/filesystem/repo/nx_turbo.rs:57:        monorepo_tool: Some(MonorepoTool::Nx),
sniff/lib/src/filesystem/repo/nx_turbo.rs:58:        workspace_tools: vec![MonorepoTool::Nx],
sniff/lib/src/filesystem/repo/nx_turbo.rs:84:            MonorepoTool::Turborepo,
sniff/lib/src/filesystem/repo/nx_turbo.rs:97:            MonorepoTool::Turborepo,
sniff/lib/src/filesystem/repo/nx_turbo.rs:104:            MonorepoTool::Turborepo,
sniff/lib/src/filesystem/repo/nx_turbo.rs:115:        monorepo_tool: Some(MonorepoTool::Turborepo),
sniff/lib/src/filesystem/repo/nx_turbo.rs:116:        workspace_tools: vec![MonorepoTool::Turborepo],
sniff/lib/src/filesystem/repo/nx_turbo.rs:142:            MonorepoTool::Lerna,
sniff/lib/src/filesystem/repo/nx_turbo.rs:155:            MonorepoTool::Lerna,
sniff/lib/src/filesystem/repo/nx_turbo.rs:162:            MonorepoTool::Lerna,
sniff/lib/src/filesystem/repo/nx_turbo.rs:173:        monorepo_tool: Some(MonorepoTool::Lerna),
sniff/lib/src/filesystem/repo/nx_turbo.rs:174:        workspace_tools: vec![MonorepoTool::Lerna],
sniff/lib/src/filesystem/repo/polyglot.rs:26:use super::types::{MonorepoTool, Package, PackageDiscoverySource};
sniff/lib/src/filesystem/repo/polyglot.rs:149:                        MonorepoTool::Unknown,
sniff/lib/src/filesystem/repo/standard.rs:2://! organized than the legacy [`MonorepoTool`] enum can express.
sniff/lib/src/filesystem/repo/standard.rs:4://! [`MonorepoStandard`] separates the three axes `MonorepoTool` conflates:
sniff/lib/src/filesystem/repo/standard.rs:15://! [`MonorepoTool`]: super::types::MonorepoTool
sniff/lib/src/filesystem/repo/topology.rs:21:use super::types::{MonorepoTool, Package};
sniff/lib/src/filesystem/repo/topology.rs:31:/// Map a legacy [`MonorepoTool`] to its [`MonorepoStandard`] counterpart.
sniff/lib/src/filesystem/repo/topology.rs:32:pub(crate) fn standard_for_tool(tool: MonorepoTool) -> MonorepoStandard {
sniff/lib/src/filesystem/repo/topology.rs:34:        MonorepoTool::CargoWorkspace => MonorepoStandard::CargoWorkspace,
sniff/lib/src/filesystem/repo/topology.rs:35:        MonorepoTool::NpmWorkspaces => MonorepoStandard::NpmWorkspaces,
sniff/lib/src/filesystem/repo/topology.rs:36:        MonorepoTool::PnpmWorkspaces => MonorepoStandard::PnpmWorkspaces,
sniff/lib/src/filesystem/repo/topology.rs:37:        MonorepoTool::YarnWorkspaces => MonorepoStandard::YarnWorkspaces,
sniff/lib/src/filesystem/repo/topology.rs:38:        MonorepoTool::Nx => MonorepoStandard::Nx,
sniff/lib/src/filesystem/repo/topology.rs:39:        MonorepoTool::Turborepo => MonorepoStandard::Turborepo,
sniff/lib/src/filesystem/repo/topology.rs:40:        MonorepoTool::Lerna => MonorepoStandard::Lerna,
sniff/lib/src/filesystem/repo/topology.rs:41:        MonorepoTool::Unknown => MonorepoStandard::Unknown,
sniff/lib/src/filesystem/repo/topology.rs:248:            standard_for_tool(MonorepoTool::CargoWorkspace),
sniff/lib/src/filesystem/repo/topology.rs:251:        assert_eq!(standard_for_tool(MonorepoTool::Nx), MonorepoStandard::Nx);
sniff/lib/src/filesystem/repo/topology.rs:253:            standard_for_tool(MonorepoTool::Unknown),
sniff/lib/src/filesystem/repo/types.rs:20:pub enum MonorepoTool {
sniff/lib/src/filesystem/repo/types.rs:87:    pub monorepo_tool: Option<MonorepoTool>,
sniff/lib/src/filesystem/repo/types.rs:90:    pub workspace_tools: Vec<MonorepoTool>,
sniff/lib/src/filesystem/repo/uv.rs:12:use super::types::{MonorepoTool, PackageDiscoverySource, RepoInfo};
sniff/lib/src/filesystem/repo/uv.rs:33:        MonorepoTool::Unknown,
sniff/lib/src/filesystem/repo/uv.rs:42:        MonorepoTool::Unknown,
sniff/lib/src/lib.rs:1:// `MonorepoTool` is deprecated in favor of `MonorepoStandard` but remains in
sniff/lib/tests/integration.rs:1:// `MonorepoTool` is deprecated but these integration tests still assert the
sniff/lib/tests/integration.rs:190:        Some(sniff::filesystem::MonorepoTool::PnpmWorkspaces)
sniff/lib/tests/integration.rs:278:        Some(sniff::filesystem::MonorepoTool::CargoWorkspace)
sniff/lib/tests/integration.rs:282:        vec![sniff::filesystem::MonorepoTool::CargoWorkspace]
sniff/lib/tests/integration.rs:648:        Some(sniff::filesystem::MonorepoTool::PnpmWorkspaces)
sniff/lib/tests/integration.rs:769:        Some(sniff::filesystem::MonorepoTool::CargoWorkspace)
sniff/lib/tests/integration.rs:773:            .contains(&sniff::filesystem::MonorepoTool::PnpmWorkspaces)
sniff/lib/tests/integration.rs:918:    // Bun has no legacy `MonorepoTool`, so the legacy field stays empty.
sniff/lib/tests/integration.rs:1031:    // Gradle has no legacy `MonorepoTool`, so the legacy field stays empty.
```

## Full `PackageDiscoverySource` / `discovery_sources` / `discovery_source_for_tool` hits

Command: `git grep -n "PackageDiscoverySource\|discovery_sources\|discovery_source_for_tool" -- sniff/lib sniff/cli`

```
sniff/cli/src/output/filesystem/mod.rs:1360:            discovery_sources: vec![],
sniff/cli/src/output/repo_json.rs:1082:                discovery_sources: vec![],
sniff/cli/src/output/repo_json.rs:1314:                discovery_sources: vec![],
sniff/cli/src/output/repo_json.rs:1637:                discovery_sources: vec![],
sniff/cli/src/output/repo_json.rs:1952:                discovery_sources: vec![],
sniff/lib/src/filesystem/git/recent_commits.rs:1414:                    discovery_sources: vec![],
sniff/lib/src/filesystem/git/recent_commits.rs:1444:                    discovery_sources: vec![],
sniff/lib/src/filesystem/git/recent_commits.rs:1474:                    discovery_sources: vec![],
sniff/lib/src/filesystem/git/recent_commits.rs:1504:                    discovery_sources: vec![],
sniff/lib/src/filesystem/mod.rs:45:    MonorepoStandard, MonorepoStandardSpec, MonorepoTool, Package, PackageDiscoverySource,
sniff/lib/src/filesystem/repo/detection.rs:46:    MonorepoTool, Package, PackageDiscoverySource, PackageEcosystem, PackageFiles,
sniff/lib/src/filesystem/repo/detection.rs:370:                PackageDiscoverySource::ManifestScan,
sniff/lib/src/filesystem/repo/detection.rs:1009:pub(crate) fn discovery_source_for_tool(tool: MonorepoTool) -> PackageDiscoverySource {
sniff/lib/src/filesystem/repo/detection.rs:1011:        MonorepoTool::CargoWorkspace => PackageDiscoverySource::CargoWorkspace,
sniff/lib/src/filesystem/repo/detection.rs:1012:        MonorepoTool::PnpmWorkspaces => PackageDiscoverySource::PnpmWorkspace,
sniff/lib/src/filesystem/repo/detection.rs:1013:        MonorepoTool::NpmWorkspaces => PackageDiscoverySource::NpmWorkspace,
sniff/lib/src/filesystem/repo/detection.rs:1014:        MonorepoTool::YarnWorkspaces => PackageDiscoverySource::YarnWorkspace,
sniff/lib/src/filesystem/repo/detection.rs:1015:        MonorepoTool::Nx => PackageDiscoverySource::Nx,
sniff/lib/src/filesystem/repo/detection.rs:1016:        MonorepoTool::Turborepo => PackageDiscoverySource::Turborepo,
sniff/lib/src/filesystem/repo/detection.rs:1017:        MonorepoTool::Lerna => PackageDiscoverySource::Lerna,
sniff/lib/src/filesystem/repo/detection.rs:1018:        MonorepoTool::Unknown => PackageDiscoverySource::ManifestScan,
sniff/lib/src/filesystem/repo/detection.rs:1096:    for source in incoming.discovery_sources {
sniff/lib/src/filesystem/repo/detection.rs:1097:        if !existing.discovery_sources.contains(&source) {
sniff/lib/src/filesystem/repo/detection.rs:1098:            existing.discovery_sources.push(source);
sniff/lib/src/filesystem/repo/detection.rs:1341:    discovery_source: PackageDiscoverySource,
sniff/lib/src/filesystem/repo/detection.rs:1353:    discovery_source: PackageDiscoverySource,
sniff/lib/src/filesystem/repo/detection.rs:1462:        discovery_sources: vec![discovery_source],
sniff/lib/src/filesystem/repo/detection.rs:1500:            discovery_sources: Vec::new(),
sniff/lib/src/filesystem/repo/dotnet.rs:11:use super::types::{MonorepoTool, PackageDiscoverySource, RepoInfo};
sniff/lib/src/filesystem/repo/dotnet.rs:49:                PackageDiscoverySource::ManifestScan,
sniff/lib/src/filesystem/repo/glob.rs:25:use super::detection::{create_package, discovery_source_for_tool};
sniff/lib/src/filesystem/repo/glob.rs:55:    let discovery_source = discovery_source_for_tool(tool);
sniff/lib/src/filesystem/repo/go.rs:9:use super::types::{MonorepoTool, PackageDiscoverySource, RepoInfo};
sniff/lib/src/filesystem/repo/go.rs:35:            PackageDiscoverySource::ManifestScan,
sniff/lib/src/filesystem/repo/gradle.rs:8:use super::types::{MonorepoTool, PackageDiscoverySource, RepoInfo};
sniff/lib/src/filesystem/repo/gradle.rs:39:            PackageDiscoverySource::ManifestScan,
sniff/lib/src/filesystem/repo/manifest_index.rs:15:use super::types::{MonorepoTool, Package, PackageDiscoverySource};
sniff/lib/src/filesystem/repo/manifest_index.rs:299:    discovery_source: PackageDiscoverySource,
sniff/lib/src/filesystem/repo/manifest_index.rs:309:    discovery_source: PackageDiscoverySource,
sniff/lib/src/filesystem/repo/manifest_index.rs:329:    discovery_source: PackageDiscoverySource,
sniff/lib/src/filesystem/repo/manifest_index.rs:395:    discovery_source: PackageDiscoverySource,
sniff/lib/src/filesystem/repo/maven.rs:11:use super::types::{MonorepoTool, PackageDiscoverySource, RepoInfo};
sniff/lib/src/filesystem/repo/maven.rs:37:            PackageDiscoverySource::ManifestScan,
sniff/lib/src/filesystem/repo/mod.rs:30:    MonorepoTool, Package, PackageDiscoverySource, PackageEcosystem, RepoInfo, detect_repo,
sniff/lib/src/filesystem/repo/npm.rs:13:use super::types::{MonorepoTool, PackageDiscoverySource, RepoInfo};
sniff/lib/src/filesystem/repo/npm.rs:326:            PackageDiscoverySource::ManifestScan,
sniff/lib/src/filesystem/repo/nx_turbo.rs:16:use super::types::{MonorepoTool, PackageDiscoverySource, RepoInfo};
sniff/lib/src/filesystem/repo/nx_turbo.rs:34:            PackageDiscoverySource::Nx,
sniff/lib/src/filesystem/repo/nx_turbo.rs:48:            PackageDiscoverySource::Nx,
sniff/lib/src/filesystem/repo/nx_turbo.rs:86:            PackageDiscoverySource::Turborepo,
sniff/lib/src/filesystem/repo/nx_turbo.rs:106:            PackageDiscoverySource::Turborepo,
sniff/lib/src/filesystem/repo/nx_turbo.rs:144:            PackageDiscoverySource::Lerna,
sniff/lib/src/filesystem/repo/nx_turbo.rs:164:            PackageDiscoverySource::Lerna,
sniff/lib/src/filesystem/repo/polyglot.rs:26:use super::types::{MonorepoTool, Package, PackageDiscoverySource};
sniff/lib/src/filesystem/repo/polyglot.rs:151:                        PackageDiscoverySource::ManifestScan,
sniff/lib/src/filesystem/repo/types.rs:61:pub enum PackageDiscoverySource {
sniff/lib/src/filesystem/repo/types.rs:136:    pub discovery_sources: Vec<PackageDiscoverySource>,
sniff/lib/src/filesystem/repo/types.rs:433:                    discovery_sources: Vec::new(),
sniff/lib/src/filesystem/repo/types.rs:463:                    discovery_sources: Vec::new(),
sniff/lib/src/filesystem/repo/uv.rs:12:use super::types::{MonorepoTool, PackageDiscoverySource, RepoInfo};
sniff/lib/src/filesystem/repo/uv.rs:44:        PackageDiscoverySource::ManifestScan,
```

## Full `monorepo_tool` / `workspace_tools` hits (representative)

Command: `git grep -n "monorepo_tool\|workspace_tools" -- sniff claudine`

Key consumers by category:

- **In-process field reads**
  - `claudine/lib/src/events/environment.rs:181,261-262,274`
  - `claudine/lib/src/dispatch/expression.rs:286,290`
  - `claudine/lib/src/dispatch/template.rs:133,372,514`
  - `sniff/cli/src/output/filesystem/repo.rs:168,178-179,536,634,946`
  - `sniff/lib/src/filesystem/repo/detection.rs:392-393`
  - `sniff/lib/src/filesystem/repo/nested.rs:470-477`
- **JSON/template docs**
  - `sniff/cli/README.md:459`
  - `sniff/docs/cli/repo_structure.md:152-153`
  - `sniff/lib/README.md:489,517`
  - `claudine/lib/README.md:139`
  - `claudine/docs/topics/configuring-actions.md:251,317`
  - `claudine/docs/topics/log-reporting.md:52`
  - `claudine/docs/topics/unified-events.md:951`
- **Tests / snapshots**
  - `sniff/cli/tests/snapshots.rs:415-416,498-499`
  - `sniff/cli/tests/snapshots/*.snap` (legacy keys still present)
  - `sniff/lib/tests/integration.rs:189,277-281,647,768-772,868-873,919,...`
  - `claudine/cli/src/commands/wrap/env.rs:933-934`
  - `claudine/lib/src/dispatch/expression.rs:385,425,645`
  - `claudine/lib/src/dispatch/runner/mod.rs:542`
  - `claudine/lib/src/dispatch/template.rs:659`
  - `claudine/lib/src/events/environment.rs:458`
- **Detectors**
  - `sniff/lib/src/filesystem/repo/{cargo,npm,nx_turbo,go,uv,gradle,maven,dotnet}.rs`

Full list: 234 hits; see the generated audit file `parity_legacy_fields.txt` if needed.

## Full `LayerPackage` hits

Command: `git grep -n "LayerPackage" -- sniff`

```
sniff/cli/src/output/repo_json.rs:2408:            BinarySource, DetectedStandard, DetectionConfidence, LayerPackage, MonorepoLayer,
sniff/cli/src/output/repo_json.rs:2445:                        LayerPackage {
sniff/cli/src/output/repo_json.rs:2451:                        LayerPackage {
sniff/lib/src/filesystem/repo/mod.rs:25:    InvocationTemplate, LayerPackage, Marker, MarkerConfidence, MarkerContent, MembershipModel,
sniff/lib/src/filesystem/repo/standard.rs:388:    pub packages: Vec<LayerPackage>,
sniff/lib/src/filesystem/repo/standard.rs:393:pub struct LayerPackage {
sniff/lib/src/filesystem/repo/standard.rs:2003:            .map(|i| LayerPackage {
sniff/lib/src/filesystem/repo/topology.rs:18:    DetectedStandard, DetectionConfidence, LayerPackage, MonorepoLayer, MonorepoStandard,
sniff/lib/src/filesystem/repo/topology.rs:75:                .map(|pkg| LayerPackage {
```

## D2 confirmation evidence

Command:

```bash
cargo run -p sniff-cli --bin sniff -- repo structure --json > /tmp/sniff_repo_structure.json
jq '[.packages[] | select(.discovery_sources and (.discovery_sources | length) > 1) | .discovery_sources | sort] | unique' /tmp/sniff_repo_structure.json
```

Result:

```json
[
  ["cargo_workspace", "manifest_scan"],
  ["manifest_scan", "pnpm_workspace"]
]
```

Every multi-element `discovery_sources` array is the `[authority, manifest_scan]`
pattern. No genuine multi-authority case was found.

## Validation run

- `just test` in `sniff/` — 690 tests passed, 2 skipped.
- `just lint` in `sniff/` — clean (no warnings/errors).
