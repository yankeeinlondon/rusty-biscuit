//! Monorepo standard model: a higher-fidelity description of how a monorepo is
//! organized than the legacy flat-tool enum could express.
//!
//! [`MonorepoStandard`] separates three axes the legacy model conflated:
//! membership format (what file declares which packages belong), role (what the
//! tool does — defines membership, orchestrates tasks, manages dependencies),
//! and acting binary (what you actually run). Richness lives in a const
//! descriptor each variant returns via [`MonorepoStandard::spec`]; roles are a
//! property, not a sibling enum.
//!
//! This module is pure data and accessors: detection wiring, the glob expander,
//! and CLI output land in later phases. See the feature spec at
//! `sniff/features/2026-06-15-improved-monorepo-capture/spec.md`.

use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::executable_index::ExecutableIndex;
use crate::filesystem::file_types::ProgrammingLanguage;

// TODO(swift): see spec § "How should SwiftPM be represented?". The
// `SwiftPackage` variant is intentionally absent until option 2
// (`.package(path:)` local-package detection) lands; a multi-target
// `Package.swift` must not be reported as a monorepo.

/// A standard for organizing a monorepo.
///
/// A detection result is a set of these (see [`DetectedStandard`] and
/// [`MonorepoLayer`]). Each variant exposes a static [`MonorepoStandardSpec`]
/// via [`spec`](Self::spec). Variants serialize as kebab-case, and each
/// variant's `spec().id` matches that wire value.
#[non_exhaustive]
#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(rename_all = "kebab-case")]
pub enum MonorepoStandard {
    // Membership authorities (workspace standards)
    /// Rust Cargo workspace (`[workspace].members`).
    CargoWorkspace,
    /// npm workspaces (`package.json#workspaces`).
    NpmWorkspaces,
    /// pnpm workspaces (`pnpm-workspace.yaml`).
    PnpmWorkspaces,
    /// Yarn workspaces (`package.json#workspaces`).
    YarnWorkspaces,
    /// Bun workspaces (`package.json#workspaces` + `bun.lock`).
    BunWorkspaces,
    /// uv workspace (`pyproject.toml#tool.uv.workspace.members`).
    UvWorkspace,
    /// Go workspace (`go.work`).
    GoWorkspace,
    /// Gradle multi-project build (`settings.gradle` `include`).
    GradleMultiProject,
    /// Maven multi-module build (`pom.xml` `<modules>`).
    MavenMultiModule,
    /// .NET solution (`*.sln` / `*.slnx`).
    DotNetSolution,
    // Polyglot build systems — BOTH define membership AND orchestrate
    /// Bazel (`WORKSPACE` + leaf `BUILD` files).
    Bazel,
    /// Pants (`pants.toml` + leaf `BUILD.pants` files).
    Pants,
    /// Buck2 (leaf `BUCK` / `TARGETS` files).
    Buck2,
    /// Rush Stack (`rush.json` `projects`).
    RushStack,
    // Pure orchestrators (layered on a membership authority)
    /// Nx orchestrator (`nx.json`).
    Nx,
    /// Turborepo orchestrator (`turbo.json`).
    Turborepo,
    /// Lerna orchestrator (`lerna.json`).
    Lerna,
    /// Fallback when a monorepo is inferred but no standard is confirmed.
    #[default]
    Unknown,
}

/// What a standard does. A standard can hold several roles; the
/// authority-vs-orchestrator relationship is derived from this set rather than
/// stored as a flat list.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    /// Declares which packages belong to the monorepo.
    DefinesMembership,
    /// Runs tasks across packages.
    OrchestratesTasks,
    /// Resolves and installs dependencies.
    ManagesDependencies,
}

/// A detection proof: a file plus a content predicate and a confidence weight.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct Marker {
    /// Marker file name (e.g. `"Cargo.toml"`).
    pub file: &'static str,
    /// What the file must contain to count.
    pub requires: MarkerContent,
    /// How strongly this marker proves the standard.
    pub confidence: MarkerConfidence,
}

/// The content predicate a [`Marker`] file must satisfy.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MarkerContent {
    /// File presence alone is sufficient (`pnpm-workspace.yaml`, `go.work`).
    Existence,
    /// A keyed field must be present and non-empty (e.g. `Cargo.toml` →
    /// `"workspace.members"`, `package.json` → `"workspaces"`).
    Field(&'static str),
}

/// How strongly a [`Marker`] proves a standard is in use.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum MarkerConfidence {
    /// Proves the standard on its own.
    Strong,
    /// Corroborates but does not prove on its own.
    Secondary,
}

/// How a standard declares which packages belong.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MembershipModel {
    /// Root manifest lists member globs (Cargo, npm, pnpm, uv).
    RootGlobs {
        /// Glob dialect the expander must interpret.
        dialect: GlobDialect,
        /// The field holding include patterns (e.g. `"workspace.members"`).
        include: &'static str,
        /// The field holding exclude patterns, if any.
        exclude: Option<&'static str>,
    },
    /// Root manifest lists explicit member paths, not globs (`go.work` `use`,
    /// Maven `<modules>`, Gradle `include`, Rush `projects`).
    RootExplicit,
    /// Packages are any directory containing a build file (Bazel/Pants/Buck
    /// `BUILD` files). Leaf-ward, not root-ward.
    LeafMarkers {
        /// The per-directory build file name (e.g. `"BUILD"`).
        file: &'static str,
    },
    /// Targets declared inline in a single manifest (SwiftPM `Package.swift`).
    InlineTargets,
    /// A manifest lists local path dependencies pointing at other packages
    /// (SwiftPM `.package(path:)`, ad hoc Rust path-dependency setups).
    LocalPathDependencies,
}

/// Glob dialect, so the expander interprets member patterns correctly.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum GlobDialect {
    /// Cargo's documented subset (prefix `*`, limited `**`).
    Cargo,
    /// minimatch-style: `**`, `{a,b}`, `!negation` (npm/pnpm/yarn/bun, uv).
    Minimatch,
}

/// Whether the workspace root is itself a package.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum RootMembership {
    /// Root is never a member (pnpm, npm, yarn, Maven parent `packaging=pom`).
    Never,
    /// Root is always a member (uv: the root `[project]` is a member).
    Always,
    /// Root is a member only when its manifest also declares a package
    /// (Cargo: `[workspace]` + `[package]` in the same `Cargo.toml`).
    WhenManifestDeclaresPackage,
}

/// What counts toward "is this multi-package?".
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceMultiplicity {
    /// The standard declares package members directly.
    MemberCount,
    /// The standard has nested target/product concepts; targets alone do not
    /// make a monorepo (SwiftPM).
    PackageBoundaryOnly,
}

/// A binary that operates a standard, plus its wrapper and version policy.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct BinarySpec {
    /// Executable name (e.g. `"cargo"`, `"pnpm"`, `"gradle"`).
    pub name: &'static str,
    /// Argument that prints the version (e.g. `"--version"`).
    pub version_arg: &'static str,
    /// Minimum version that supports the standard, if gated (e.g. `"7"` for
    /// npm workspaces, `"1.20"` for `go.work`).
    pub min_version: Option<&'static str>,
    /// Repo-local wrapper script that should be preferred over the system
    /// binary, if any (e.g. `gradlew`).
    pub wrapper: Option<WrapperScript>,
}

/// A repo-local wrapper script that supersedes the system binary.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct WrapperScript {
    /// POSIX wrapper name (e.g. `"gradlew"`, `"mvnw"`).
    pub posix: &'static str,
    /// Windows wrapper name (e.g. `"gradlew.bat"`, `"mvnw.cmd"`).
    pub windows: &'static str,
}

/// An advisory command template a consumer can render. **sniff never executes
/// these** — they are reference metadata, like `markers` or `primary_language`.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct InvocationTemplate {
    /// The program to run (e.g. `"cargo"`).
    pub program: &'static str,
    /// The argument template, with substitution tokens.
    pub args: &'static [Token],
}

/// A token in an [`InvocationTemplate`]'s argument list.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Token {
    /// A literal argument.
    Lit(&'static str),
    /// Substituted with the target package name.
    Package,
    /// Substituted with the user task (build/test/...).
    Task,
    /// Expands to the "run everywhere" flag(s).
    AllPackages,
}

/// Whether a nested marker starts a new workspace.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum NestingPolicy {
    /// Overlapping/nested workspaces are forbidden (Cargo). Stop walking.
    ForbidsNested,
    /// A nested marker starts a separate workspace; ignore its subtree from the
    /// parent's perspective (Bazel nested `WORKSPACE`).
    IgnoresNested,
    /// Nested workspaces are allowed and discovered as their own roots.
    AllowsNested,
}

/// Compile-time metadata describing how a [`MonorepoStandard`] works.
///
/// Returned by reference from [`MonorepoStandard::spec`]; all data is `'static`
/// with zero runtime cost.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct MonorepoStandardSpec {
    /// Stable serde id, matching the variant's kebab-case wire value
    /// (e.g. `"cargo-workspace"`).
    pub id: &'static str,
    /// Human-facing name (e.g. `"Cargo Workspace"`).
    pub display_name: &'static str,
    /// Natural-language label for CLI rendering (e.g. `"cargo"`,
    /// `"pnpm workspaces"`, `"Nx"`).
    pub label: &'static str,
    /// Roles this standard holds.
    pub roles: &'static [Role],
    /// Primary language, or `None` for polyglot standards.
    pub primary_language: Option<ProgrammingLanguage>,
    /// Detection proofs: file + content predicate + confidence.
    pub markers: &'static [Marker],
    /// How packages are declared.
    pub membership: MembershipModel,
    /// Whether the workspace root is itself a package.
    pub root_membership: RootMembership,
    /// What counts toward "multi-package?".
    pub multiplicity: WorkspaceMultiplicity,
    /// Binaries that operate this standard.
    pub binaries: &'static [BinarySpec],
    /// Advisory: how to enumerate packages. **Never executed by sniff.**
    pub enumerate_packages: Option<InvocationTemplate>,
    /// Advisory: how to run a task in one package. **Never executed by sniff.**
    pub run_in_package: Option<InvocationTemplate>,
    /// Advisory: how to run a task across all packages. **Never executed.**
    pub run_across_all: Option<InvocationTemplate>,
    /// Whether a nested marker starts a new workspace.
    pub nesting_policy: NestingPolicy,
}

/// A binary resolved for a detected standard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedBinary {
    /// Executable name.
    pub name: String,
    /// Absolute path, if resolved.
    pub path: Option<PathBuf>,
    /// Version string, if probed.
    pub version: Option<String>,
    /// Whether the resolved version satisfies the spec's `min_version`.
    /// `None` when no minimum is declared or the version is unknown/unparseable.
    pub satisfies_min_version: Option<bool>,
    /// How the binary was resolved.
    pub source: BinarySource,
}

/// How a [`ResolvedBinary`] was located.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum BinarySource {
    /// A repo-local wrapper script.
    Wrapper,
    /// Found on `PATH`.
    Path,
    /// Not found.
    Missing,
}

/// How a layer's package list was derived. sniff only ever reports
/// filesystem-derived provenance — there is deliberately no `Tool` variant.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum PackageProvenance {
    /// Expanded membership globs. Best-effort; bounded by the glob expander.
    Globbed,
    /// Parsed a root manifest's explicit member path list.
    Explicit,
    /// Walked leaf-side marker files such as `BUILD` / `BUCK` / `project.json`.
    LeafMarkers,
    /// Parsed local path dependencies as package links.
    LocalPathDependencies,
    /// Parsed the committed lockfile's resolved member set. High fidelity,
    /// still filesystem-only.
    Lockfile,
    /// Discovered by walking for per-directory manifests without a confirming
    /// membership authority. This is the fallback provenance for packages found
    /// by manifest index scans.
    #[default]
    ManifestScan,
}

/// Whether a detection was marker-confirmed or merely inferred.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum DetectionConfidence {
    /// A Strong marker matched and membership resolved non-degenerately.
    MarkerConfirmed,
    /// Inferred from weaker signals; standard reported as
    /// [`MonorepoStandard::Unknown`].
    Inferred,
}

/// A raw detection: which standard matched, where, and with what binary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetectedStandard {
    /// The detected standard.
    pub standard: MonorepoStandard,
    /// Where this standard's root marker lives.
    pub root: PathBuf,
    /// Which marker files actually matched.
    pub matched_markers: Vec<PathBuf>,
    /// Acting binary resolved from lockfile/wrapper, if any.
    pub binary: Option<ResolvedBinary>,
    /// Detection confidence.
    pub confidence: DetectionConfidence,
}

/// A membership layer: the authority that declares packages plus any
/// orchestrators riding on top, with the packages it resolved.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MonorepoLayer {
    /// Where this layer's membership root lives.
    pub root: PathBuf,
    /// The standard that actually declares the packages (role
    /// [`Role::DefinesMembership]).
    pub authority: MonorepoStandard,
    /// Orchestrators riding on top (role [`Role::OrchestratesTasks`] only).
    pub orchestrators: Vec<MonorepoStandard>,
    /// How this layer's package list was derived. Packages inherit it.
    pub provenance: PackageProvenance,
    /// Whether the committed lockfile agrees with the manifest-derived package
    /// set, if a lockfile was consulted. `None` when no lockfile was parsed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lockfile_match: Option<bool>,
    /// Whether the layer's root manifest also declares a package, when the
    /// standard's [`RootMembership`] is [`RootMembership::WhenManifestDeclaresPackage`].
    ///
    /// For [`RootMembership::Always`] this is always `true`; for
    /// [`RootMembership::Never`] it is always `false`. It is consulted by
    /// [`MonorepoStandard::membership_resolves_non_degenerately`] so a single
    /// member plus a non-package root (e.g. a virtual Cargo workspace with one
    /// member) is correctly treated as degenerate.
    #[serde(default)]
    pub root_is_package: bool,
    /// Packages resolved for this layer, with repo-relative paths matching
    /// [`Package::relative`]. Each entry resolves to exactly one package in the
    /// canonical `RepoInfo.packages` catalog.
    pub packages: Vec<String>,
}

impl MonorepoStandard {
    /// The glob dialect this standard's membership patterns use, when its
    /// membership is glob-based.
    ///
    /// Returns `None` for standards whose membership is not declared with globs
    /// (explicit paths, leaf markers, inline targets). Detectors read the
    /// dialect from here so the descriptor table stays the single source of
    /// truth.
    pub(crate) fn glob_dialect(self) -> Option<GlobDialect> {
        match self.spec().membership {
            MembershipModel::RootGlobs { dialect, .. } => Some(dialect),
            _ => None,
        }
    }

    /// Whether this standard declares which packages belong to the monorepo
    /// (role [`Role::DefinesMembership`]).
    pub(crate) fn defines_membership(self) -> bool {
        self.spec().roles.contains(&Role::DefinesMembership)
    }

    /// Whether this standard only orchestrates tasks and does **not** declare
    /// membership — the orchestrator half of the authority-vs-orchestrator
    /// split (`Nx`, `Turborepo`, `Lerna`).
    pub(crate) fn orchestrates_tasks_only(self) -> bool {
        let roles = self.spec().roles;
        roles.contains(&Role::OrchestratesTasks) && !roles.contains(&Role::DefinesMembership)
    }

    /// The [`PackageProvenance`] a layer rooted on this standard's membership
    /// model carries. Packages in the layer inherit it.
    pub(crate) fn membership_provenance(self) -> PackageProvenance {
        // Unknown is the fallback when no membership authority is confirmed; the
        // packages were discovered by manifest index scan, not by an explicit
        // member list.
        if self == MonorepoStandard::Unknown {
            return PackageProvenance::ManifestScan;
        }
        match self.spec().membership {
            MembershipModel::RootGlobs { .. } => PackageProvenance::Globbed,
            MembershipModel::RootExplicit => PackageProvenance::Explicit,
            MembershipModel::LeafMarkers { .. } => PackageProvenance::LeafMarkers,
            MembershipModel::LocalPathDependencies => PackageProvenance::LocalPathDependencies,
            // No standard with `InlineTargets` membership is detected yet
            // (SwiftPM is deferred); treat it as explicitly listed.
            MembershipModel::InlineTargets => PackageProvenance::Explicit,
        }
    }

    /// Whether `layer`'s resolved membership is rich enough to call the repo a
    /// monorepo, per this standard's [`WorkspaceMultiplicity`] and
    /// [`RootMembership`].
    ///
    /// A `MemberCount` standard needs at least two resolved packages, or a
    /// single non-root member when its [`RootMembership`] also lets the root
    /// count. `WhenManifestDeclaresPackage` consults
    /// [`MonorepoLayer::root_is_package`] so a virtual Cargo workspace with a
    /// single member (no `[package]` at the root) is honestly degenerate. A
    /// `PackageBoundaryOnly` standard never resolves non-degenerately on its
    /// own — targets and products are not packages.
    pub fn membership_resolves_non_degenerately(self, layer: &MonorepoLayer) -> bool {
        match self.spec().multiplicity {
            WorkspaceMultiplicity::MemberCount => match layer.packages.len() {
                0 => false,
                1 => match self.spec().root_membership {
                    RootMembership::Always => true,
                    RootMembership::WhenManifestDeclaresPackage => layer.root_is_package,
                    RootMembership::Never => false,
                },
                _ => true,
            },
            WorkspaceMultiplicity::PackageBoundaryOnly => false,
        }
    }

    /// The compile-time descriptor for this standard.
    pub const fn spec(self) -> &'static MonorepoStandardSpec {
        match self {
            Self::CargoWorkspace => &CARGO_WORKSPACE_SPEC,
            Self::NpmWorkspaces => &NPM_WORKSPACES_SPEC,
            Self::PnpmWorkspaces => &PNPM_WORKSPACES_SPEC,
            Self::YarnWorkspaces => &YARN_WORKSPACES_SPEC,
            Self::Nx => &NX_SPEC,
            Self::Turborepo => &TURBOREPO_SPEC,
            Self::Lerna => &LERNA_SPEC,
            Self::Unknown => &UNKNOWN_SPEC,
            Self::BunWorkspaces => &BUN_WORKSPACES_SPEC,
            Self::UvWorkspace => &UV_WORKSPACE_SPEC,
            Self::GoWorkspace => &GO_WORKSPACE_SPEC,
            Self::GradleMultiProject => &GRADLE_MULTI_PROJECT_SPEC,
            Self::MavenMultiModule => &MAVEN_MULTI_MODULE_SPEC,
            Self::DotNetSolution => &DOTNET_SOLUTION_SPEC,
            Self::Bazel => &BAZEL_SPEC,
            Self::Pants => &PANTS_SPEC,
            Self::Buck2 => &BUCK2_SPEC,
            Self::RushStack => &RUSH_STACK_SPEC,
        }
    }
}

/// Resolve the acting binary for `standard` at `root` using the repo-local
/// wrapper script (when declared) or the shared `ExecutableIndex`.
///
/// This function never executes a binary: it only checks the wrapper script's
/// existence on disk and asks the index whether the PATH binary is present.
/// `version` and `satisfies_min_version` are left as `None` because populating
/// them would require spawning `--version`, which violates sniff's
/// filesystem-only detection boundary (spec §8). A consumer that wants the
/// version is expected to run the advisory `version_arg` itself.
///
/// ## Returns
///
/// - `Some(ResolvedBinary)` — when a wrapper script or PATH binary is found.
/// - `None` — when neither is present. The repo is still recognized as using
///   the standard; only its acting binary is unavailable on this host.
pub fn resolve_acting_binary(
    standard: MonorepoStandard,
    root: &Path,
    index: &ExecutableIndex,
) -> Option<ResolvedBinary> {
    let spec = standard.spec();
    let binary = spec.binaries.first()?;

    // Wrapper scripts take precedence over system binaries. Only the file's
    // existence is checked — never executed.
    if let Some(wrapper) = binary.wrapper {
        let wrapper_path = if cfg!(windows) {
            root.join(wrapper.windows)
        } else {
            root.join(wrapper.posix)
        };
        if wrapper_path.exists() {
            return Some(ResolvedBinary {
                name: binary.name.to_string(),
                path: Some(wrapper_path),
                version: None,
                satisfies_min_version: None,
                source: BinarySource::Wrapper,
            });
        }
    }

    // Fall back to PATH / bundle / Windows fallback indexes. The index's
    // `find_with_source` answers from cache or a `which` lookup; it does not
    // spawn the binary.
    let (path, _source) = index.find_with_source(binary.name)?;
    Some(ResolvedBinary {
        name: binary.name.to_string(),
        path: Some(path),
        version: None,
        satisfies_min_version: None,
        source: BinarySource::Path,
    })
}

/// Test-only variant of [`resolve_acting_binary`] that injects a raw version
/// output string instead of probing the filesystem, so unit tests stay
/// hermetic and do not require real monorepo binaries.
#[cfg(test)]
pub fn resolve_acting_binary_with_version(
    standard: MonorepoStandard,
    root: &Path,
    index: &ExecutableIndex,
    raw_version: Option<&str>,
) -> Option<ResolvedBinary> {
    let spec = standard.spec();
    let binary = spec.binaries.first()?;
    let version = raw_version.and_then(extract_version);

    if let Some(wrapper) = binary.wrapper {
        let wrapper_path = if cfg!(windows) {
            root.join(wrapper.windows)
        } else {
            root.join(wrapper.posix)
        };
        if wrapper_path.exists() {
            let satisfies = min_version_satisfies(&version, binary.min_version);
            return Some(ResolvedBinary {
                name: binary.name.to_string(),
                path: Some(wrapper_path),
                version,
                satisfies_min_version: satisfies,
                source: BinarySource::Wrapper,
            });
        }
    }

    let (path, _source) = index.find_with_source(binary.name)?;
    let satisfies = min_version_satisfies(&version, binary.min_version);
    Some(ResolvedBinary {
        name: binary.name.to_string(),
        path: Some(path),
        version,
        satisfies_min_version: satisfies,
        source: BinarySource::Path,
    })
}

/// Extract the first version-like token from a `--version` output block.
///
/// Used by the hermetic test helper [`resolve_acting_binary_with_version`]
/// which never spawns a real binary; the production path leaves `version` as
/// `None` to honor the no-subprocess detection boundary.
#[cfg(test)]
fn extract_version(text: &str) -> Option<String> {
    let first_line = text.lines().next().unwrap_or(text);
    let re = regex::Regex::new(r"v?(\d+(?:\.\d+)*)").expect("static version regex is valid");
    re.captures(first_line)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

/// Parse a dotted version string into a `(major, minor, patch)` tuple.
///
/// Missing components are padded with zeros; trailing components beyond patch
/// are ignored. Returns `None` for empty or non-numeric input.
#[cfg(test)]
fn parse_version_tuple(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let patch = parts.next().unwrap_or("0").parse().unwrap_or(0);
    Some((major, minor, patch))
}

/// Whether the resolved `version` satisfies the `min_version` requirement.
///
/// Returns `None` when either side is missing or unparseable, never
/// `Some(false)` unless both sides parsed conclusively.
#[cfg(test)]
fn min_version_satisfies(version: &Option<String>, min_version: Option<&str>) -> Option<bool> {
    let version = version.as_ref()?;
    let min = min_version?;
    let resolved = parse_version_tuple(version)?;
    let required = parse_version_tuple(min)?;
    Some(resolved >= required)
}

const CARGO_WORKSPACE_SPEC: MonorepoStandardSpec = MonorepoStandardSpec {
    id: "cargo-workspace",
    display_name: "Cargo Workspace",
    label: "cargo",
    roles: &[
        Role::DefinesMembership,
        Role::OrchestratesTasks,
        Role::ManagesDependencies,
    ],
    primary_language: Some(ProgrammingLanguage::Rust),
    markers: &[Marker {
        file: "Cargo.toml",
        requires: MarkerContent::Field("workspace.members"),
        confidence: MarkerConfidence::Strong,
    }],
    membership: MembershipModel::RootGlobs {
        dialect: GlobDialect::Cargo,
        include: "workspace.members",
        exclude: Some("workspace.exclude"),
    },
    root_membership: RootMembership::WhenManifestDeclaresPackage,
    multiplicity: WorkspaceMultiplicity::MemberCount,
    binaries: &[BinarySpec {
        name: "cargo",
        version_arg: "--version",
        min_version: None,
        wrapper: None,
    }],
    enumerate_packages: Some(InvocationTemplate {
        program: "cargo",
        args: &[
            Token::Lit("metadata"),
            Token::Lit("--no-deps"),
            Token::Lit("--format-version"),
            Token::Lit("1"),
        ],
    }),
    run_in_package: Some(InvocationTemplate {
        program: "cargo",
        args: &[Token::Task, Token::Lit("-p"), Token::Package],
    }),
    run_across_all: Some(InvocationTemplate {
        program: "cargo",
        args: &[Token::Task, Token::AllPackages],
    }),
    nesting_policy: NestingPolicy::ForbidsNested,
};

const NPM_WORKSPACES_SPEC: MonorepoStandardSpec = MonorepoStandardSpec {
    id: "npm-workspaces",
    display_name: "npm Workspaces",
    label: "npm workspaces",
    roles: &[
        Role::DefinesMembership,
        Role::OrchestratesTasks,
        Role::ManagesDependencies,
    ],
    primary_language: Some(ProgrammingLanguage::JavaScript),
    markers: &[Marker {
        file: "package.json",
        requires: MarkerContent::Field("workspaces"),
        confidence: MarkerConfidence::Strong,
    }],
    membership: MembershipModel::RootGlobs {
        dialect: GlobDialect::Minimatch,
        include: "workspaces",
        exclude: None,
    },
    root_membership: RootMembership::Never,
    multiplicity: WorkspaceMultiplicity::MemberCount,
    binaries: &[BinarySpec {
        name: "npm",
        version_arg: "--version",
        min_version: Some("7"),
        wrapper: None,
    }],
    enumerate_packages: Some(InvocationTemplate {
        program: "npm",
        args: &[
            Token::Lit("ls"),
            Token::Lit("--workspaces"),
            Token::Lit("--json"),
        ],
    }),
    run_in_package: Some(InvocationTemplate {
        program: "npm",
        args: &[
            Token::Lit("run"),
            Token::Task,
            Token::Lit("--workspace"),
            Token::Package,
        ],
    }),
    run_across_all: Some(InvocationTemplate {
        program: "npm",
        args: &[Token::Lit("run"), Token::Task, Token::AllPackages],
    }),
    nesting_policy: NestingPolicy::AllowsNested,
};

const PNPM_WORKSPACES_SPEC: MonorepoStandardSpec = MonorepoStandardSpec {
    id: "pnpm-workspaces",
    display_name: "pnpm Workspaces",
    label: "pnpm workspaces",
    roles: &[
        Role::DefinesMembership,
        Role::OrchestratesTasks,
        Role::ManagesDependencies,
    ],
    primary_language: Some(ProgrammingLanguage::JavaScript),
    markers: &[Marker {
        file: "pnpm-workspace.yaml",
        requires: MarkerContent::Existence,
        confidence: MarkerConfidence::Strong,
    }],
    membership: MembershipModel::RootGlobs {
        dialect: GlobDialect::Minimatch,
        include: "packages",
        exclude: None,
    },
    root_membership: RootMembership::Never,
    multiplicity: WorkspaceMultiplicity::MemberCount,
    binaries: &[BinarySpec {
        name: "pnpm",
        version_arg: "--version",
        min_version: None,
        wrapper: None,
    }],
    enumerate_packages: Some(InvocationTemplate {
        program: "pnpm",
        args: &[
            Token::Lit("ls"),
            Token::Lit("-r"),
            Token::Lit("--depth"),
            Token::Lit("-1"),
            Token::Lit("--json"),
        ],
    }),
    run_in_package: Some(InvocationTemplate {
        program: "pnpm",
        args: &[Token::Lit("--filter"), Token::Package, Token::Task],
    }),
    run_across_all: Some(InvocationTemplate {
        program: "pnpm",
        args: &[Token::AllPackages, Token::Task],
    }),
    nesting_policy: NestingPolicy::AllowsNested,
};

const YARN_WORKSPACES_SPEC: MonorepoStandardSpec = MonorepoStandardSpec {
    id: "yarn-workspaces",
    display_name: "Yarn Workspaces",
    label: "yarn workspaces",
    roles: &[
        Role::DefinesMembership,
        Role::OrchestratesTasks,
        Role::ManagesDependencies,
    ],
    primary_language: Some(ProgrammingLanguage::JavaScript),
    markers: &[Marker {
        file: "package.json",
        requires: MarkerContent::Field("workspaces"),
        confidence: MarkerConfidence::Strong,
    }],
    membership: MembershipModel::RootGlobs {
        dialect: GlobDialect::Minimatch,
        include: "workspaces",
        exclude: None,
    },
    root_membership: RootMembership::Never,
    multiplicity: WorkspaceMultiplicity::MemberCount,
    binaries: &[BinarySpec {
        name: "yarn",
        version_arg: "--version",
        min_version: None,
        wrapper: None,
    }],
    enumerate_packages: Some(InvocationTemplate {
        program: "yarn",
        args: &[
            Token::Lit("workspaces"),
            Token::Lit("list"),
            Token::Lit("--json"),
        ],
    }),
    run_in_package: Some(InvocationTemplate {
        program: "yarn",
        args: &[Token::Lit("workspace"), Token::Package, Token::Task],
    }),
    run_across_all: Some(InvocationTemplate {
        program: "yarn",
        args: &[Token::Lit("workspaces"), Token::Lit("foreach"), Token::Task],
    }),
    nesting_policy: NestingPolicy::AllowsNested,
};

// Nx orchestrates tasks; it never declares membership (it delegates to the
// underlying package-manager workspace). The `LeafMarkers` value reflects how
// Nx discovers projects for orchestration, not a membership authority.
const NX_SPEC: MonorepoStandardSpec = MonorepoStandardSpec {
    id: "nx",
    display_name: "Nx",
    label: "Nx",
    roles: &[Role::OrchestratesTasks],
    primary_language: None,
    markers: &[Marker {
        file: "nx.json",
        requires: MarkerContent::Existence,
        confidence: MarkerConfidence::Strong,
    }],
    membership: MembershipModel::LeafMarkers {
        file: "project.json",
    },
    root_membership: RootMembership::Never,
    multiplicity: WorkspaceMultiplicity::MemberCount,
    binaries: &[BinarySpec {
        name: "nx",
        version_arg: "--version",
        min_version: None,
        wrapper: None,
    }],
    enumerate_packages: Some(InvocationTemplate {
        program: "nx",
        args: &[Token::Lit("show"), Token::Lit("projects")],
    }),
    run_in_package: Some(InvocationTemplate {
        program: "nx",
        args: &[Token::Task, Token::Package],
    }),
    run_across_all: Some(InvocationTemplate {
        program: "nx",
        args: &[Token::Lit("run-many"), Token::Lit("-t"), Token::Task],
    }),
    nesting_policy: NestingPolicy::AllowsNested,
};

// Turborepo rides on the package manager's workspace globs; it orchestrates but
// does not own membership.
const TURBOREPO_SPEC: MonorepoStandardSpec = MonorepoStandardSpec {
    id: "turborepo",
    display_name: "Turborepo",
    label: "Turborepo",
    roles: &[Role::OrchestratesTasks],
    primary_language: None,
    markers: &[Marker {
        file: "turbo.json",
        requires: MarkerContent::Existence,
        confidence: MarkerConfidence::Strong,
    }],
    membership: MembershipModel::RootGlobs {
        dialect: GlobDialect::Minimatch,
        include: "workspaces",
        exclude: None,
    },
    root_membership: RootMembership::Never,
    multiplicity: WorkspaceMultiplicity::MemberCount,
    binaries: &[BinarySpec {
        name: "turbo",
        version_arg: "--version",
        min_version: None,
        wrapper: None,
    }],
    enumerate_packages: Some(InvocationTemplate {
        program: "turbo",
        args: &[Token::Lit("ls")],
    }),
    run_in_package: Some(InvocationTemplate {
        program: "turbo",
        args: &[
            Token::Lit("run"),
            Token::Task,
            Token::Lit("--filter"),
            Token::Package,
        ],
    }),
    run_across_all: Some(InvocationTemplate {
        program: "turbo",
        args: &[Token::Lit("run"), Token::Task],
    }),
    nesting_policy: NestingPolicy::AllowsNested,
};

// Lerna reads its own `packages` field but is, in modern usage, a task
// orchestrator layered on a package-manager workspace.
const LERNA_SPEC: MonorepoStandardSpec = MonorepoStandardSpec {
    id: "lerna",
    display_name: "Lerna",
    label: "Lerna",
    roles: &[Role::OrchestratesTasks],
    primary_language: None,
    markers: &[Marker {
        file: "lerna.json",
        requires: MarkerContent::Existence,
        confidence: MarkerConfidence::Strong,
    }],
    membership: MembershipModel::RootGlobs {
        dialect: GlobDialect::Minimatch,
        include: "packages",
        exclude: None,
    },
    root_membership: RootMembership::Never,
    multiplicity: WorkspaceMultiplicity::MemberCount,
    binaries: &[BinarySpec {
        name: "lerna",
        version_arg: "--version",
        min_version: None,
        wrapper: None,
    }],
    enumerate_packages: Some(InvocationTemplate {
        program: "lerna",
        args: &[Token::Lit("list")],
    }),
    run_in_package: Some(InvocationTemplate {
        program: "lerna",
        args: &[
            Token::Lit("run"),
            Token::Task,
            Token::Lit("--scope"),
            Token::Package,
        ],
    }),
    run_across_all: Some(InvocationTemplate {
        program: "lerna",
        args: &[Token::Lit("run"), Token::Task],
    }),
    nesting_policy: NestingPolicy::AllowsNested,
};

const UNKNOWN_SPEC: MonorepoStandardSpec = MonorepoStandardSpec {
    id: "unknown",
    display_name: "Unknown",
    label: "unknown",
    roles: &[],
    primary_language: None,
    markers: &[],
    membership: MembershipModel::RootExplicit,
    root_membership: RootMembership::Never,
    multiplicity: WorkspaceMultiplicity::MemberCount,
    binaries: &[],
    enumerate_packages: None,
    run_in_package: None,
    run_across_all: None,
    nesting_policy: NestingPolicy::AllowsNested,
};

// Bun shares npm's `package.json#workspaces` membership field; the `bun.lock` /
// `bun.lockb` lockfile is what disambiguates Bun from npm/yarn at detection.
const BUN_WORKSPACES_SPEC: MonorepoStandardSpec = MonorepoStandardSpec {
    id: "bun-workspaces",
    display_name: "Bun Workspaces",
    label: "bun workspaces",
    roles: &[
        Role::DefinesMembership,
        Role::OrchestratesTasks,
        Role::ManagesDependencies,
    ],
    primary_language: Some(ProgrammingLanguage::JavaScript),
    markers: &[Marker {
        file: "package.json",
        requires: MarkerContent::Field("workspaces"),
        confidence: MarkerConfidence::Strong,
    }],
    membership: MembershipModel::RootGlobs {
        dialect: GlobDialect::Minimatch,
        include: "workspaces",
        exclude: None,
    },
    root_membership: RootMembership::Never,
    multiplicity: WorkspaceMultiplicity::MemberCount,
    binaries: &[BinarySpec {
        name: "bun",
        version_arg: "--version",
        min_version: None,
        wrapper: None,
    }],
    enumerate_packages: None,
    run_in_package: Some(InvocationTemplate {
        program: "bun",
        args: &[
            Token::Lit("run"),
            Token::Lit("--filter"),
            Token::Package,
            Token::Task,
        ],
    }),
    run_across_all: Some(InvocationTemplate {
        program: "bun",
        args: &[Token::Lit("run"), Token::Task],
    }),
    nesting_policy: NestingPolicy::AllowsNested,
};

// uv counts its own root `[project]` as a workspace member, so `RootMembership`
// is `Always` — unlike the JS authorities, which never count the root.
const UV_WORKSPACE_SPEC: MonorepoStandardSpec = MonorepoStandardSpec {
    id: "uv-workspace",
    display_name: "uv Workspace",
    label: "uv workspace",
    roles: &[
        Role::DefinesMembership,
        Role::OrchestratesTasks,
        Role::ManagesDependencies,
    ],
    primary_language: Some(ProgrammingLanguage::Python),
    markers: &[Marker {
        file: "pyproject.toml",
        requires: MarkerContent::Field("tool.uv.workspace.members"),
        confidence: MarkerConfidence::Strong,
    }],
    membership: MembershipModel::RootGlobs {
        dialect: GlobDialect::Minimatch,
        include: "tool.uv.workspace.members",
        exclude: None,
    },
    root_membership: RootMembership::Always,
    multiplicity: WorkspaceMultiplicity::MemberCount,
    binaries: &[BinarySpec {
        name: "uv",
        version_arg: "--version",
        min_version: Some("0.4"),
        wrapper: None,
    }],
    enumerate_packages: Some(InvocationTemplate {
        program: "uv",
        args: &[Token::Lit("tree")],
    }),
    run_in_package: Some(InvocationTemplate {
        program: "uv",
        args: &[
            Token::Lit("run"),
            Token::Lit("--package"),
            Token::Package,
            Token::Task,
        ],
    }),
    run_across_all: Some(InvocationTemplate {
        program: "uv",
        args: &[Token::Lit("run"), Token::Task],
    }),
    nesting_policy: NestingPolicy::ForbidsNested,
};

// `go.work` lists literal module paths via `use` directives, not globs, so its
// membership model is `RootExplicit`.
const GO_WORKSPACE_SPEC: MonorepoStandardSpec = MonorepoStandardSpec {
    id: "go-workspace",
    display_name: "Go Workspace",
    label: "go workspace",
    roles: &[
        Role::DefinesMembership,
        Role::OrchestratesTasks,
        Role::ManagesDependencies,
    ],
    primary_language: Some(ProgrammingLanguage::Go),
    markers: &[Marker {
        file: "go.work",
        requires: MarkerContent::Existence,
        confidence: MarkerConfidence::Strong,
    }],
    membership: MembershipModel::RootExplicit,
    root_membership: RootMembership::Never,
    multiplicity: WorkspaceMultiplicity::MemberCount,
    binaries: &[BinarySpec {
        name: "go",
        version_arg: "version",
        min_version: Some("1.20"),
        wrapper: None,
    }],
    enumerate_packages: Some(InvocationTemplate {
        program: "go",
        args: &[Token::Lit("work"), Token::Lit("edit"), Token::Lit("-json")],
    }),
    run_in_package: Some(InvocationTemplate {
        program: "go",
        args: &[Token::Task, Token::Lit("./...")],
    }),
    run_across_all: Some(InvocationTemplate {
        program: "go",
        args: &[Token::Task, Token::Lit("./...")],
    }),
    nesting_policy: NestingPolicy::AllowsNested,
};

// Gradle lists subprojects via `include` directives in `settings.gradle[.kts]`,
// which are explicit Gradle paths (`:a:b`), not globs. The repo-local `gradlew`
// wrapper is preferred over the system `gradle` when resolving the acting binary.
const GRADLE_MULTI_PROJECT_SPEC: MonorepoStandardSpec = MonorepoStandardSpec {
    id: "gradle-multi-project",
    display_name: "Gradle Multi-Project Build",
    label: "Gradle",
    roles: &[
        Role::DefinesMembership,
        Role::OrchestratesTasks,
        Role::ManagesDependencies,
    ],
    primary_language: Some(ProgrammingLanguage::Java),
    markers: &[
        Marker {
            file: "settings.gradle",
            requires: MarkerContent::Field("include"),
            confidence: MarkerConfidence::Strong,
        },
        Marker {
            file: "settings.gradle.kts",
            requires: MarkerContent::Field("include"),
            confidence: MarkerConfidence::Strong,
        },
    ],
    membership: MembershipModel::RootExplicit,
    root_membership: RootMembership::Never,
    multiplicity: WorkspaceMultiplicity::MemberCount,
    binaries: &[BinarySpec {
        name: "gradle",
        version_arg: "--version",
        min_version: None,
        wrapper: Some(WrapperScript {
            posix: "gradlew",
            windows: "gradlew.bat",
        }),
    }],
    enumerate_packages: Some(InvocationTemplate {
        program: "gradle",
        args: &[Token::Lit("projects")],
    }),
    run_in_package: Some(InvocationTemplate {
        program: "gradle",
        args: &[Token::Task],
    }),
    run_across_all: Some(InvocationTemplate {
        program: "gradle",
        args: &[Token::Task],
    }),
    nesting_policy: NestingPolicy::AllowsNested,
};

// Maven lists submodules via `<modules><module>...</module></modules>` in the
// parent `pom.xml`; those are explicit relative directory paths. The parent POM
// has `packaging=pom` and is never itself a built module.
const MAVEN_MULTI_MODULE_SPEC: MonorepoStandardSpec = MonorepoStandardSpec {
    id: "maven-multi-module",
    display_name: "Maven Multi-Module Build",
    label: "Maven",
    roles: &[
        Role::DefinesMembership,
        Role::OrchestratesTasks,
        Role::ManagesDependencies,
    ],
    primary_language: Some(ProgrammingLanguage::Java),
    markers: &[Marker {
        file: "pom.xml",
        requires: MarkerContent::Field("modules"),
        confidence: MarkerConfidence::Strong,
    }],
    membership: MembershipModel::RootExplicit,
    root_membership: RootMembership::Never,
    multiplicity: WorkspaceMultiplicity::MemberCount,
    binaries: &[BinarySpec {
        name: "mvn",
        version_arg: "--version",
        min_version: None,
        wrapper: Some(WrapperScript {
            posix: "mvnw",
            windows: "mvnw.cmd",
        }),
    }],
    enumerate_packages: None,
    run_in_package: Some(InvocationTemplate {
        program: "mvn",
        args: &[Token::Lit("-pl"), Token::Package, Token::Task],
    }),
    run_across_all: Some(InvocationTemplate {
        program: "mvn",
        args: &[Token::Task],
    }),
    nesting_policy: NestingPolicy::AllowsNested,
};

// A `.sln` / `.slnx` solution lists its `Project(...)` entries as explicit
// `.csproj` / `.fsproj` paths. The solution file name is arbitrary, so its
// marker uses a glob; detection scans the root for any solution file.
const DOTNET_SOLUTION_SPEC: MonorepoStandardSpec = MonorepoStandardSpec {
    id: "dot-net-solution",
    display_name: ".NET Solution",
    label: ".NET solution",
    roles: &[Role::DefinesMembership, Role::OrchestratesTasks],
    primary_language: Some(ProgrammingLanguage::CSharp),
    markers: &[Marker {
        file: "*.sln",
        requires: MarkerContent::Existence,
        confidence: MarkerConfidence::Strong,
    }],
    membership: MembershipModel::RootExplicit,
    root_membership: RootMembership::Never,
    multiplicity: WorkspaceMultiplicity::MemberCount,
    binaries: &[BinarySpec {
        name: "dotnet",
        version_arg: "--version",
        min_version: None,
        wrapper: None,
    }],
    enumerate_packages: Some(InvocationTemplate {
        program: "dotnet",
        args: &[Token::Lit("sln"), Token::Lit("list")],
    }),
    run_in_package: Some(InvocationTemplate {
        program: "dotnet",
        args: &[Token::Task, Token::Package],
    }),
    run_across_all: Some(InvocationTemplate {
        program: "dotnet",
        args: &[Token::Task],
    }),
    nesting_policy: NestingPolicy::AllowsNested,
};

// Bazel discovers packages by walking for per-directory `BUILD` / `BUILD.bazel`
// files (the membership model *is* the walk), so it is polyglot with no single
// primary language. A nested `WORKSPACE` / `MODULE.bazel` starts a separate
// workspace whose subtree the parent ignores (`IgnoresNested`). The wrapper is
// the conventional `bazelisk`-style `bazelw`.
const BAZEL_SPEC: MonorepoStandardSpec = MonorepoStandardSpec {
    id: "bazel",
    display_name: "Bazel",
    label: "Bazel",
    roles: &[
        Role::DefinesMembership,
        Role::OrchestratesTasks,
        Role::ManagesDependencies,
    ],
    primary_language: None,
    markers: &[
        Marker {
            file: "WORKSPACE",
            requires: MarkerContent::Existence,
            confidence: MarkerConfidence::Strong,
        },
        Marker {
            file: "WORKSPACE.bazel",
            requires: MarkerContent::Existence,
            confidence: MarkerConfidence::Strong,
        },
        Marker {
            file: "MODULE.bazel",
            requires: MarkerContent::Existence,
            confidence: MarkerConfidence::Strong,
        },
    ],
    membership: MembershipModel::LeafMarkers { file: "BUILD" },
    root_membership: RootMembership::Never,
    multiplicity: WorkspaceMultiplicity::MemberCount,
    binaries: &[BinarySpec {
        name: "bazel",
        version_arg: "--version",
        min_version: None,
        wrapper: Some(WrapperScript {
            posix: "bazelw",
            windows: "bazelw.bat",
        }),
    }],
    enumerate_packages: Some(InvocationTemplate {
        program: "bazel",
        args: &[Token::Lit("query"), Token::Lit("//...")],
    }),
    run_in_package: Some(InvocationTemplate {
        program: "bazel",
        args: &[Token::Task, Token::Lit("//...")],
    }),
    run_across_all: Some(InvocationTemplate {
        program: "bazel",
        args: &[Token::Task, Token::Lit("//...")],
    }),
    nesting_policy: NestingPolicy::IgnoresNested,
};

// Pants is rooted at `pants.toml` and discovers packages by walking for leaf
// `BUILD.pants` files (the detector also accepts plain `BUILD`).
const PANTS_SPEC: MonorepoStandardSpec = MonorepoStandardSpec {
    id: "pants",
    display_name: "Pants",
    label: "Pants",
    roles: &[
        Role::DefinesMembership,
        Role::OrchestratesTasks,
        Role::ManagesDependencies,
    ],
    primary_language: None,
    markers: &[Marker {
        file: "pants.toml",
        requires: MarkerContent::Existence,
        confidence: MarkerConfidence::Strong,
    }],
    membership: MembershipModel::LeafMarkers {
        file: "BUILD.pants",
    },
    root_membership: RootMembership::Never,
    multiplicity: WorkspaceMultiplicity::MemberCount,
    binaries: &[BinarySpec {
        name: "pants",
        version_arg: "--version",
        min_version: None,
        wrapper: None,
    }],
    enumerate_packages: Some(InvocationTemplate {
        program: "pants",
        args: &[Token::Lit("list"), Token::Lit("::")],
    }),
    run_in_package: Some(InvocationTemplate {
        program: "pants",
        args: &[Token::Task, Token::Package, Token::Lit("::")],
    }),
    run_across_all: Some(InvocationTemplate {
        program: "pants",
        args: &[Token::Task, Token::Lit("::")],
    }),
    nesting_policy: NestingPolicy::AllowsNested,
};

// Buck2 identifies the project root by `.buckconfig` and discovers packages by
// walking for leaf `BUCK` files (the detector also accepts `TARGETS`).
const BUCK2_SPEC: MonorepoStandardSpec = MonorepoStandardSpec {
    id: "buck2",
    display_name: "Buck2",
    label: "Buck2",
    roles: &[
        Role::DefinesMembership,
        Role::OrchestratesTasks,
        Role::ManagesDependencies,
    ],
    primary_language: None,
    markers: &[Marker {
        file: ".buckconfig",
        requires: MarkerContent::Existence,
        confidence: MarkerConfidence::Strong,
    }],
    membership: MembershipModel::LeafMarkers { file: "BUCK" },
    root_membership: RootMembership::Never,
    multiplicity: WorkspaceMultiplicity::MemberCount,
    binaries: &[BinarySpec {
        name: "buck2",
        version_arg: "--version",
        min_version: None,
        wrapper: None,
    }],
    enumerate_packages: Some(InvocationTemplate {
        program: "buck2",
        args: &[Token::Lit("targets"), Token::Lit("//...")],
    }),
    run_in_package: Some(InvocationTemplate {
        program: "buck2",
        args: &[Token::Task, Token::Lit("//...")],
    }),
    run_across_all: Some(InvocationTemplate {
        program: "buck2",
        args: &[Token::Task, Token::Lit("//...")],
    }),
    nesting_policy: NestingPolicy::AllowsNested,
};

// Rush lists `{ projectFolder, packageName }` entries in `rush.json#projects`;
// those are explicit paths, not globs. Rush orchestrates JS-family packages and
// declares their membership, but dependency installation is delegated to the
// underlying package manager (`ManagesDependencies` is intentionally absent).
const RUSH_STACK_SPEC: MonorepoStandardSpec = MonorepoStandardSpec {
    id: "rush-stack",
    display_name: "Rush Stack",
    label: "Rush",
    roles: &[Role::DefinesMembership, Role::OrchestratesTasks],
    primary_language: Some(ProgrammingLanguage::JavaScript),
    markers: &[Marker {
        file: "rush.json",
        requires: MarkerContent::Field("projects"),
        confidence: MarkerConfidence::Strong,
    }],
    membership: MembershipModel::RootExplicit,
    root_membership: RootMembership::Never,
    multiplicity: WorkspaceMultiplicity::MemberCount,
    binaries: &[BinarySpec {
        name: "rush",
        version_arg: "--version",
        min_version: None,
        wrapper: None,
    }],
    enumerate_packages: Some(InvocationTemplate {
        program: "rush",
        args: &[Token::Lit("list")],
    }),
    run_in_package: Some(InvocationTemplate {
        program: "rush",
        args: &[Token::Lit("build"), Token::Lit("--to"), Token::Package],
    }),
    run_across_all: Some(InvocationTemplate {
        program: "rush",
        args: &[Token::Lit("build")],
    }),
    nesting_policy: NestingPolicy::AllowsNested,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Variants whose descriptors are fully populated in this phase.
    const IMPLEMENTED: &[MonorepoStandard] = &[
        MonorepoStandard::CargoWorkspace,
        MonorepoStandard::NpmWorkspaces,
        MonorepoStandard::PnpmWorkspaces,
        MonorepoStandard::YarnWorkspaces,
        MonorepoStandard::BunWorkspaces,
        MonorepoStandard::UvWorkspace,
        MonorepoStandard::GoWorkspace,
        MonorepoStandard::GradleMultiProject,
        MonorepoStandard::MavenMultiModule,
        MonorepoStandard::DotNetSolution,
        MonorepoStandard::Bazel,
        MonorepoStandard::Pants,
        MonorepoStandard::Buck2,
        MonorepoStandard::RushStack,
        MonorepoStandard::Nx,
        MonorepoStandard::Turborepo,
        MonorepoStandard::Lerna,
        MonorepoStandard::Unknown,
    ];

    fn wire_id(standard: MonorepoStandard) -> String {
        serde_json::to_string(&standard)
            .unwrap()
            .trim_matches('"')
            .to_string()
    }

    #[test]
    fn implemented_spec_id_matches_kebab_wire_value() {
        for &standard in IMPLEMENTED {
            let wire = wire_id(standard);
            assert_eq!(
                standard.spec().id,
                wire,
                "spec().id must match the serde wire value for {standard:?}"
            );
            // Round-trip through the wire form to prove the id is the contract.
            let json = format!("\"{wire}\"");
            let back: MonorepoStandard = serde_json::from_str(&json).unwrap();
            assert_eq!(back, standard);
        }
    }

    #[test]
    fn every_variant_id_is_kebab_case() {
        // Stubs carry a correct id too, so detection-time lookups by id are
        // never surprised once a stub is upgraded.
        let all = [
            MonorepoStandard::CargoWorkspace,
            MonorepoStandard::NpmWorkspaces,
            MonorepoStandard::PnpmWorkspaces,
            MonorepoStandard::YarnWorkspaces,
            MonorepoStandard::BunWorkspaces,
            MonorepoStandard::UvWorkspace,
            MonorepoStandard::GoWorkspace,
            MonorepoStandard::GradleMultiProject,
            MonorepoStandard::MavenMultiModule,
            MonorepoStandard::DotNetSolution,
            MonorepoStandard::Bazel,
            MonorepoStandard::Pants,
            MonorepoStandard::Buck2,
            MonorepoStandard::RushStack,
            MonorepoStandard::Nx,
            MonorepoStandard::Turborepo,
            MonorepoStandard::Lerna,
            MonorepoStandard::Unknown,
        ];
        for standard in all {
            assert_eq!(standard.spec().id, wire_id(standard), "{standard:?}");
        }
    }

    #[test]
    fn polyglot_leaf_marker_descriptors_match_decision_log() {
        // Bazel/Pants/Buck2 declare membership by walking for per-directory
        // build files, so their membership model is `LeafMarkers` and they carry
        // no single primary language.
        let bazel = MonorepoStandard::Bazel.spec();
        assert_eq!(bazel.primary_language, None);
        assert_eq!(
            bazel.membership,
            MembershipModel::LeafMarkers { file: "BUILD" }
        );
        assert_eq!(bazel.nesting_policy, NestingPolicy::IgnoresNested);
        assert_eq!(
            bazel.binaries[0].wrapper,
            Some(WrapperScript {
                posix: "bazelw",
                windows: "bazelw.bat",
            })
        );
        let bazel_markers: Vec<&str> = bazel.markers.iter().map(|m| m.file).collect();
        assert_eq!(
            bazel_markers,
            vec!["WORKSPACE", "WORKSPACE.bazel", "MODULE.bazel"]
        );

        let pants = MonorepoStandard::Pants.spec();
        assert_eq!(
            pants.membership,
            MembershipModel::LeafMarkers {
                file: "BUILD.pants"
            }
        );
        assert_eq!(pants.markers[0].file, "pants.toml");
        assert_eq!(pants.nesting_policy, NestingPolicy::AllowsNested);

        let buck2 = MonorepoStandard::Buck2.spec();
        assert_eq!(
            buck2.membership,
            MembershipModel::LeafMarkers { file: "BUCK" }
        );
        assert_eq!(buck2.markers[0].file, ".buckconfig");
        assert_eq!(buck2.binaries[0].name, "buck2");

        for standard in [
            MonorepoStandard::Bazel,
            MonorepoStandard::Pants,
            MonorepoStandard::Buck2,
        ] {
            assert_eq!(
                standard.membership_provenance(),
                PackageProvenance::LeafMarkers,
                "{standard:?}"
            );
        }
    }

    #[test]
    fn rush_descriptor_lists_explicit_projects() {
        let spec = MonorepoStandard::RushStack.spec();
        assert_eq!(spec.id, "rush-stack");
        assert_eq!(spec.primary_language, Some(ProgrammingLanguage::JavaScript));
        // Rush declares membership and orchestrates, but defers dependency
        // installation to the underlying package manager.
        assert_eq!(
            spec.roles,
            &[Role::DefinesMembership, Role::OrchestratesTasks]
        );
        assert_eq!(
            spec.markers,
            &[Marker {
                file: "rush.json",
                requires: MarkerContent::Field("projects"),
                confidence: MarkerConfidence::Strong,
            }]
        );
        assert_eq!(spec.membership, MembershipModel::RootExplicit);
        assert_eq!(
            MonorepoStandard::RushStack.membership_provenance(),
            PackageProvenance::Explicit
        );
        assert_eq!(spec.binaries[0].name, "rush");
        assert_eq!(spec.binaries[0].wrapper, None);
    }

    #[test]
    fn cargo_workspace_descriptor_matches_decision_log() {
        let spec = MonorepoStandard::CargoWorkspace.spec();
        assert_eq!(spec.display_name, "Cargo Workspace");
        assert_eq!(spec.primary_language, Some(ProgrammingLanguage::Rust));
        assert_eq!(
            spec.roles,
            &[
                Role::DefinesMembership,
                Role::OrchestratesTasks,
                Role::ManagesDependencies,
            ]
        );
        assert_eq!(
            spec.markers,
            &[Marker {
                file: "Cargo.toml",
                requires: MarkerContent::Field("workspace.members"),
                confidence: MarkerConfidence::Strong,
            }]
        );
        assert_eq!(
            spec.membership,
            MembershipModel::RootGlobs {
                dialect: GlobDialect::Cargo,
                include: "workspace.members",
                exclude: Some("workspace.exclude"),
            }
        );
        assert_eq!(
            spec.root_membership,
            RootMembership::WhenManifestDeclaresPackage
        );
        assert_eq!(spec.nesting_policy, NestingPolicy::ForbidsNested);
        assert_eq!(spec.binaries.len(), 1);
        assert_eq!(spec.binaries[0].name, "cargo");
        assert_eq!(spec.binaries[0].wrapper, None);
    }

    #[test]
    fn js_authorities_share_workspaces_field_but_differ_by_binary() {
        // npm/yarn point at the same membership field; only the binary differs.
        let npm = MonorepoStandard::NpmWorkspaces.spec();
        let yarn = MonorepoStandard::YarnWorkspaces.spec();
        assert_eq!(npm.markers, yarn.markers);
        assert_eq!(npm.membership, yarn.membership);
        assert_eq!(npm.binaries[0].name, "npm");
        assert_eq!(yarn.binaries[0].name, "yarn");
        assert_eq!(npm.binaries[0].min_version, Some("7"));
    }

    #[test]
    fn orchestrators_orchestrate_but_do_not_define_membership() {
        // The authority-vs-orchestrator invariant later phases rely on.
        for standard in [
            MonorepoStandard::Nx,
            MonorepoStandard::Turborepo,
            MonorepoStandard::Lerna,
        ] {
            let roles = standard.spec().roles;
            assert!(
                roles.contains(&Role::OrchestratesTasks),
                "{standard:?} must orchestrate tasks"
            );
            assert!(
                !roles.contains(&Role::DefinesMembership),
                "{standard:?} must not define membership"
            );
        }
    }

    #[test]
    fn authorities_define_membership() {
        for standard in [
            MonorepoStandard::CargoWorkspace,
            MonorepoStandard::NpmWorkspaces,
            MonorepoStandard::PnpmWorkspaces,
            MonorepoStandard::YarnWorkspaces,
        ] {
            assert!(
                standard.spec().roles.contains(&Role::DefinesMembership),
                "{standard:?} must define membership"
            );
        }
    }

    #[test]
    fn bun_workspaces_descriptor_matches_decision_log() {
        let spec = MonorepoStandard::BunWorkspaces.spec();
        assert_eq!(spec.primary_language, Some(ProgrammingLanguage::JavaScript));
        // Bun and npm share the `package.json#workspaces` membership field; only
        // the acting binary differs.
        let npm = MonorepoStandard::NpmWorkspaces.spec();
        assert_eq!(spec.markers, npm.markers);
        assert_eq!(spec.membership, npm.membership);
        assert_eq!(spec.root_membership, RootMembership::Never);
        assert_eq!(spec.nesting_policy, NestingPolicy::AllowsNested);
        assert_eq!(spec.binaries[0].name, "bun");
        assert_eq!(spec.binaries[0].wrapper, None);
        assert_eq!(
            spec.roles,
            &[
                Role::DefinesMembership,
                Role::OrchestratesTasks,
                Role::ManagesDependencies,
            ]
        );
    }

    #[test]
    fn uv_workspace_descriptor_counts_the_root_as_a_member() {
        let spec = MonorepoStandard::UvWorkspace.spec();
        assert_eq!(spec.primary_language, Some(ProgrammingLanguage::Python));
        assert_eq!(
            spec.markers,
            &[Marker {
                file: "pyproject.toml",
                requires: MarkerContent::Field("tool.uv.workspace.members"),
                confidence: MarkerConfidence::Strong,
            }]
        );
        assert_eq!(
            spec.membership,
            MembershipModel::RootGlobs {
                dialect: GlobDialect::Minimatch,
                include: "tool.uv.workspace.members",
                exclude: None,
            }
        );
        // The root `[project]` is itself a workspace member.
        assert_eq!(spec.root_membership, RootMembership::Always);
        assert_eq!(spec.nesting_policy, NestingPolicy::ForbidsNested);
        assert_eq!(spec.binaries[0].name, "uv");
        assert_eq!(spec.binaries[0].min_version, Some("0.4"));
    }

    #[test]
    fn go_workspace_descriptor_lists_explicit_paths() {
        let spec = MonorepoStandard::GoWorkspace.spec();
        assert_eq!(spec.primary_language, Some(ProgrammingLanguage::Go));
        assert_eq!(
            spec.markers,
            &[Marker {
                file: "go.work",
                requires: MarkerContent::Existence,
                confidence: MarkerConfidence::Strong,
            }]
        );
        // `use` directives list literal module paths, not globs.
        assert_eq!(spec.membership, MembershipModel::RootExplicit);
        assert_eq!(MonorepoStandard::GoWorkspace.glob_dialect(), None);
        assert_eq!(
            MonorepoStandard::GoWorkspace.membership_provenance(),
            PackageProvenance::Explicit
        );
        assert_eq!(spec.root_membership, RootMembership::Never);
        assert_eq!(spec.binaries[0].name, "go");
        assert_eq!(spec.binaries[0].min_version, Some("1.20"));
    }

    #[test]
    fn gradle_descriptor_uses_explicit_paths_and_a_wrapper() {
        let spec = MonorepoStandard::GradleMultiProject.spec();
        assert_eq!(spec.primary_language, Some(ProgrammingLanguage::Java));
        assert_eq!(
            spec.roles,
            &[
                Role::DefinesMembership,
                Role::OrchestratesTasks,
                Role::ManagesDependencies,
            ]
        );
        // Both Groovy and Kotlin settings DSLs are accepted markers.
        let marker_files: Vec<&str> = spec.markers.iter().map(|m| m.file).collect();
        assert_eq!(marker_files, vec!["settings.gradle", "settings.gradle.kts"]);
        assert_eq!(spec.membership, MembershipModel::RootExplicit);
        assert_eq!(spec.root_membership, RootMembership::Never);
        assert_eq!(spec.nesting_policy, NestingPolicy::AllowsNested);
        assert_eq!(spec.binaries[0].name, "gradle");
        assert_eq!(
            spec.binaries[0].wrapper,
            Some(WrapperScript {
                posix: "gradlew",
                windows: "gradlew.bat",
            })
        );
    }

    #[test]
    fn maven_descriptor_uses_explicit_modules_and_a_wrapper() {
        let spec = MonorepoStandard::MavenMultiModule.spec();
        assert_eq!(spec.primary_language, Some(ProgrammingLanguage::Java));
        assert_eq!(
            spec.markers,
            &[Marker {
                file: "pom.xml",
                requires: MarkerContent::Field("modules"),
                confidence: MarkerConfidence::Strong,
            }]
        );
        assert_eq!(spec.membership, MembershipModel::RootExplicit);
        // The parent POM (`packaging=pom`) is never itself a module.
        assert_eq!(spec.root_membership, RootMembership::Never);
        assert_eq!(spec.binaries[0].name, "mvn");
        assert_eq!(
            spec.binaries[0].wrapper,
            Some(WrapperScript {
                posix: "mvnw",
                windows: "mvnw.cmd",
            })
        );
    }

    #[test]
    fn dotnet_descriptor_lists_projects_without_a_wrapper() {
        let spec = MonorepoStandard::DotNetSolution.spec();
        assert_eq!(spec.id, "dot-net-solution");
        assert_eq!(spec.primary_language, Some(ProgrammingLanguage::CSharp));
        // .NET solutions define membership and orchestrate, but do not manage
        // dependencies (NuGet does, per project).
        assert_eq!(
            spec.roles,
            &[Role::DefinesMembership, Role::OrchestratesTasks]
        );
        assert_eq!(spec.membership, MembershipModel::RootExplicit);
        assert_eq!(spec.root_membership, RootMembership::Never);
        assert_eq!(spec.binaries[0].name, "dotnet");
        assert_eq!(spec.binaries[0].wrapper, None);
        assert_eq!(
            MonorepoStandard::DotNetSolution.membership_provenance(),
            PackageProvenance::Explicit
        );
    }

    #[test]
    fn invocation_templates_are_populated_for_phase_7() {
        // Every variant with an acting binary must describe how to run a task
        // in one package and across all packages. Enumeration may be absent for
        // standards that do not expose a stable list command.
        for &standard in IMPLEMENTED {
            let spec = standard.spec();
            if spec.binaries.is_empty() {
                assert!(
                    spec.enumerate_packages.is_none()
                        && spec.run_in_package.is_none()
                        && spec.run_across_all.is_none(),
                    "{standard:?} has no binary so it must have no templates"
                );
                continue;
            }
            assert!(
                spec.run_in_package.is_some(),
                "{standard:?} must declare run_in_package"
            );
            assert!(
                spec.run_across_all.is_some(),
                "{standard:?} must declare run_across_all"
            );
        }
    }

    fn test_index_with(names: &[&str]) -> ExecutableIndex {
        let mut entries = HashMap::new();
        for name in names {
            entries.insert(OsString::from(*name), PathBuf::from("/usr/bin").join(name));
        }
        ExecutableIndex::for_test(entries)
    }

    #[test]
    fn resolve_acting_binary_prefers_wrapper_over_path() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::write(root.join("gradlew"), "#!/bin/sh\necho 8.0\n").unwrap();
            fs::set_permissions(root.join("gradlew"), fs::Permissions::from_mode(0o755)).unwrap();
        }
        #[cfg(windows)]
        fs::write(root.join("gradlew.bat"), "@echo off\r\necho 8.0\r\n").unwrap();

        let index = test_index_with(&["gradle"]);
        let resolved = resolve_acting_binary_with_version(
            MonorepoStandard::GradleMultiProject,
            root,
            &index,
            Some("8.0"),
        );
        assert!(resolved.is_some());
        let binary = resolved.unwrap();
        assert_eq!(binary.name, "gradle");
        assert_eq!(binary.source, BinarySource::Wrapper);
        let wrapper_name = if cfg!(windows) { "gradlew.bat" } else { "gradlew" };
        assert!(binary.path.as_ref().unwrap().ends_with(wrapper_name));
    }

    #[test]
    fn resolve_acting_binary_falls_back_to_path_when_wrapper_missing() {
        let dir = tempfile::tempdir().unwrap();
        let index = test_index_with(&["gradle"]);
        let resolved = resolve_acting_binary_with_version(
            MonorepoStandard::GradleMultiProject,
            dir.path(),
            &index,
            Some("8.0"),
        );
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().source, BinarySource::Path);
    }

    #[test]
    fn resolve_acting_binary_returns_none_when_binary_missing() {
        let dir = tempfile::tempdir().unwrap();
        let index = ExecutableIndex::for_test(HashMap::new());
        let resolved = resolve_acting_binary_with_version(
            MonorepoStandard::GradleMultiProject,
            dir.path(),
            &index,
            None,
        );
        assert!(resolved.is_none());
    }

    #[test]
    fn resolve_acting_binary_checks_min_version() {
        let dir = tempfile::tempdir().unwrap();
        let index = test_index_with(&["go"]);

        // go.work requires Go >= 1.20.
        let satisfied = resolve_acting_binary_with_version(
            MonorepoStandard::GoWorkspace,
            dir.path(),
            &index,
            Some("go version go1.21.0 darwin/arm64"),
        );
        assert_eq!(satisfied.unwrap().satisfies_min_version, Some(true));

        let unsatisfied = resolve_acting_binary_with_version(
            MonorepoStandard::GoWorkspace,
            dir.path(),
            &index,
            Some("go version go1.19 darwin/arm64"),
        );
        assert_eq!(unsatisfied.unwrap().satisfies_min_version, Some(false));

        let unparseable = resolve_acting_binary_with_version(
            MonorepoStandard::GoWorkspace,
            dir.path(),
            &index,
            Some("dev"),
        );
        assert_eq!(unparseable.unwrap().satisfies_min_version, None);
    }

    /// Regression test for the no-subprocess detection boundary (spec §8).
    ///
    /// Both a wrapper script and a PATH binary are rigged to write a marker
    /// file if ever executed. `resolve_acting_binary` must report them without
    /// spawning either, so neither marker may appear on disk afterwards, and
    /// `version` / `satisfies_min_version` must be `None`.
    #[cfg(unix)]
    #[test]
    fn resolve_acting_binary_never_spawns_wrapper_or_path_binary() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Sentinel wrapper that touches a marker file if executed. The Gradle
        // descriptor declares `gradlew` as its wrapper name.
        let wrapper = root.join("gradlew");
        let wrapper_marker = root.join("WRAPPER_RAN");
        fs::write(
            &wrapper,
            format!("#!/bin/sh\ntouch {}\necho 8.0\n", wrapper_marker.display()),
        )
        .unwrap();
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();

        // Sentinel PATH binary that touches a different marker if executed.
        // We point the synthetic index at a fake `gradle` in a temp bin dir.
        let bin_dir = dir.path().join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        let path_binary = bin_dir.join("gradle");
        let path_marker = root.join("PATH_RAN");
        fs::write(
            &path_binary,
            format!("#!/bin/sh\ntouch {}\necho 8.0\n", path_marker.display()),
        )
        .unwrap();
        fs::set_permissions(&path_binary, fs::Permissions::from_mode(0o755)).unwrap();

        let mut entries = HashMap::new();
        entries.insert(OsString::from("gradle"), path_binary.clone());
        let index = ExecutableIndex::for_test(entries);

        // Wrapper script is present, so it wins over the PATH binary.
        let resolved = resolve_acting_binary(MonorepoStandard::GradleMultiProject, root, &index)
            .expect("wrapper script exists; resolve_acting_binary must report it without spawning");
        assert_eq!(resolved.source, BinarySource::Wrapper);
        assert_eq!(resolved.version, None);
        assert_eq!(resolved.satisfies_min_version, None);

        // The PATH binary must also be reachable without spawning when no
        // wrapper is present.
        fs::remove_file(&wrapper).unwrap();
        let resolved_path =
            resolve_acting_binary(MonorepoStandard::GradleMultiProject, root, &index)
                .expect("PATH binary is in the index; resolve_acting_binary must report it");
        assert_eq!(resolved_path.source, BinarySource::Path);
        assert_eq!(resolved_path.version, None);
        assert_eq!(resolved_path.satisfies_min_version, None);

        // The defining assertion: neither sentinel was touched.
        assert!(
            !wrapper_marker.exists(),
            "resolve_acting_binary must not execute the wrapper script"
        );
        assert!(
            !path_marker.exists(),
            "resolve_acting_binary must not execute the PATH binary"
        );
    }

    fn layer_with(authority: MonorepoStandard, package_count: usize) -> MonorepoLayer {
        let packages = (0..package_count)
            .map(|i| format!("packages/pkg-{i}"))
            .collect();
        // The helper mirrors the typical "single member plus non-package root"
        // Cargo virtual layout only when explicitly asked. By default the test
        // helper reports a non-package root, which exercises the new
        // `root_is_package: false` degenerate path.
        MonorepoLayer {
            root: PathBuf::from("/repo"),
            authority,
            orchestrators: Vec::new(),
            provenance: PackageProvenance::Globbed,
            lockfile_match: None,
            root_is_package: false,
            packages,
        }
    }

    #[test]
    fn membership_role_helpers_split_authorities_from_orchestrators() {
        assert!(MonorepoStandard::CargoWorkspace.defines_membership());
        assert!(!MonorepoStandard::CargoWorkspace.orchestrates_tasks_only());
        assert!(MonorepoStandard::Nx.orchestrates_tasks_only());
        assert!(!MonorepoStandard::Nx.defines_membership());
    }

    #[test]
    fn membership_provenance_maps_from_model() {
        assert_eq!(
            MonorepoStandard::CargoWorkspace.membership_provenance(),
            PackageProvenance::Globbed
        );
        assert_eq!(
            MonorepoStandard::PnpmWorkspaces.membership_provenance(),
            PackageProvenance::Globbed
        );
    }

    #[test]
    fn manifest_scan_provenance_wire_value_and_unknown_consistency() {
        // Manifest-scan is the provenance for packages discovered without a
        // confirming membership authority, and it serializes as kebab-case.
        let wire = serde_json::to_string(&PackageProvenance::ManifestScan)
            .unwrap()
            .trim_matches('"')
            .to_string();
        assert_eq!(wire, "manifest-scan");

        // The fallback Unknown standard has no membership authority, so its
        // derived provenance is manifest-scan rather than Explicit.
        assert_eq!(
            MonorepoStandard::Unknown.membership_provenance(),
            PackageProvenance::ManifestScan
        );
    }

    #[test]
    fn two_members_resolve_non_degenerately() {
        let layer = layer_with(MonorepoStandard::PnpmWorkspaces, 2);
        assert!(MonorepoStandard::PnpmWorkspaces.membership_resolves_non_degenerately(&layer));
    }

    #[test]
    fn single_member_resolves_only_when_root_counts() {
        // pnpm never counts the root, so a lone member is degenerate.
        let pnpm = layer_with(MonorepoStandard::PnpmWorkspaces, 1);
        assert!(!MonorepoStandard::PnpmWorkspaces.membership_resolves_non_degenerately(&pnpm));
        // A virtual Cargo workspace (root_is_package = false) with one member
        // is degenerate — the previous predicate treated it as a monorepo,
        // contrary to the spec's "honest" rule.
        let cargo_virtual = layer_with(MonorepoStandard::CargoWorkspace, 1);
        assert!(
            !MonorepoStandard::CargoWorkspace.membership_resolves_non_degenerately(&cargo_virtual),
            "virtual Cargo workspace with one member must be degenerate"
        );
        // A Cargo workspace whose root also declares a [package] is a real
        // monorepo with one member plus the root.
        let mut cargo_root_pkg = layer_with(MonorepoStandard::CargoWorkspace, 1);
        cargo_root_pkg.root_is_package = true;
        assert!(
            MonorepoStandard::CargoWorkspace.membership_resolves_non_degenerately(&cargo_root_pkg)
        );
        // uv counts the root unconditionally.
        let uv = layer_with(MonorepoStandard::UvWorkspace, 1);
        assert!(MonorepoStandard::UvWorkspace.membership_resolves_non_degenerately(&uv));
    }

    #[test]
    fn zero_members_never_resolve() {
        let layer = layer_with(MonorepoStandard::CargoWorkspace, 0);
        assert!(!MonorepoStandard::CargoWorkspace.membership_resolves_non_degenerately(&layer));
    }
}
