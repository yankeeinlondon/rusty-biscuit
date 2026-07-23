//! Shared aggregation helper for collapsing per-package values into a
//! scope-bounded distinct set.
//!
//! Used by `sniff repo test-runner`, `sniff repo version`, and (in a later
//! phase) `sniff repo package-manager`. The collapse rule from
//! `2026-06-14-more-repo/spec.md`:
//!
//! ```text
//! package        -> singular value for that package
//! package-area   -> union across contained packages; uniform -> singular,
//!                    else unique list
//! repo root      -> union across all packages; uniform -> singular, else list
//! ```

use std::collections::HashSet;
use std::path::Path;

use crate::Result;
use crate::filesystem::repo::cargo::cargo_package_version_with_source;
use crate::filesystem::repo::detection::probe_exists;
use crate::filesystem::repo::identity::{read_json, read_toml};
use crate::filesystem::repo::npm::npm_package_version;
use crate::filesystem::repo::python::pyproject_package_version;
use crate::filesystem::repo::test_runner_usage::TestRunnerUsage;
use crate::filesystem::repo::types::{
    ExternalDependency, ExternalDependencyFamily, ExternalDependencyFilter, Package, RepoInfo,
};

/// Which set of packages a collapse operates over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateScope {
    /// A single named package — collapse is trivially that package's values.
    Package(String),
    /// A package-area — collapse across every package whose `package_area`
    /// equals the given name.
    PackageArea(String),
    /// The whole repository — collapse across every package.
    Repo,
}

/// Result of collapsing values across a scope.
#[derive(Clone, PartialEq, Eq)]
pub enum AggregateResult<T> {
    /// Exactly one distinct value was found.
    Singular(T),
    /// Two or more distinct values were found.
    Multiple(Vec<T>),
    /// No values were found.
    Empty,
}

impl<T: std::fmt::Debug> std::fmt::Debug for AggregateResult<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Singular(v) => f.debug_tuple("Singular").field(v).finish(),
            Self::Multiple(v) => f.debug_tuple("Multiple").field(v).finish(),
            Self::Empty => write!(f, "Empty"),
        }
    }
}

impl<T> AggregateResult<T> {
    /// Returns the singular value, panicking when the result is not singular.
    #[track_caller]
    pub fn expect_singular(self) -> T {
        match self {
            Self::Singular(v) => v,
            _ => panic!("expected AggregateResult::Singular"),
        }
    }

    /// Returns `true` when the result is singular.
    #[must_use]
    pub fn is_singular(&self) -> bool {
        matches!(self, Self::Singular(_))
    }

    /// Returns the values as a slice (`Singular` yields a one-element slice
    /// via conversion; callers wanting owned values should use
    /// [`into_values`](Self::into_values)).
    #[must_use]
    pub fn as_values(&self) -> Vec<&T>
    where
        T: Clone,
    {
        match self {
            Self::Singular(v) => vec![v],
            Self::Multiple(v) => v.iter().collect(),
            Self::Empty => Vec::new(),
        }
    }

    /// Convert into an owned `Vec<T>`.
    #[must_use]
    pub fn into_values(self) -> Vec<T> {
        match self {
            Self::Singular(v) => vec![v],
            Self::Multiple(v) => v,
            Self::Empty => Vec::new(),
        }
    }
}

/// Resolve the aggregation scope for `dir` within `info`.
///
/// Selection rule (mirrors the `package-manager` / `test-runner` spec):
///
/// - Non-monorepo with a single discovered package → that package.
/// - Non-monorepo with no discovered packages → [`AggregateScope::Repo`] so
///   the caller can detect at the repo root directly.
/// - Inside a specific package → that package.
/// - Otherwise inside a package-area → that area.
/// - Otherwise (repo root) → [`AggregateScope::Repo`].
#[must_use]
pub fn resolve_scope(info: &RepoInfo, dir: &Path) -> AggregateScope {
    if !info.is_monorepo {
        if let Some(packages) = &info.packages
            && packages.len() == 1
        {
            return AggregateScope::Package(packages[0].name.clone());
        }
        return AggregateScope::Repo;
    }
    if let Some(pkg) = info.package_for_dir(dir) {
        return AggregateScope::Package(pkg.name.clone());
    }
    if let Some(area) = info.package_area_for_dir(dir) {
        return AggregateScope::PackageArea(area.to_string());
    }
    AggregateScope::Repo
}

/// Collect the distinct values across all packages in `scope`, deduped by the
/// key returned from `key_fn`, preserving first-seen order.
///
/// `extract` returns the full set of values a single package contributes
/// (e.g. `pkg.test_runners.clone()` or `pkg.package_managers.clone()`).
#[must_use]
pub fn collapse_package_values<T, K, F, KF>(
    packages: &[Package],
    scope: &AggregateScope,
    extract: F,
    key_fn: KF,
) -> Vec<T>
where
    T: Clone,
    K: Eq + std::hash::Hash,
    F: Fn(&Package) -> Vec<T>,
    KF: Fn(&T) -> K,
{
    let mut out: Vec<T> = Vec::new();
    let mut seen: HashSet<K> = HashSet::new();
    for pkg in packages.iter().filter(|p| in_scope(p, scope)) {
        for value in extract(pkg) {
            let key = key_fn(&value);
            if seen.insert(key) {
                out.push(value);
            }
        }
    }
    out
}

/// Collapse into an [`AggregateResult`]: zero values → `Empty`, one →
/// `Singular`, more than one → `Multiple`.
#[must_use]
pub fn aggregate_package_values<T, K, F, KF>(
    packages: &[Package],
    scope: &AggregateScope,
    extract: F,
    key_fn: KF,
) -> AggregateResult<T>
where
    T: Clone,
    K: Eq + std::hash::Hash,
    F: Fn(&Package) -> Vec<T>,
    KF: Fn(&T) -> K,
{
    let mut values = collapse_package_values(packages, scope, extract, key_fn);
    match values.len() {
        0 => AggregateResult::Empty,
        1 => AggregateResult::Singular(values.pop().expect("exactly one")),
        _ => AggregateResult::Multiple(values),
    }
}

/// One distinct test-runner usage in a scope, with the packages that attribute
/// it.
///
/// Entries are keyed by the full [`TestRunnerUsage`] (runner **and** evidence
/// source), so a workspace-root config shared by many crates collapses to a
/// single entry naming all of them, while per-package configs of the same
/// runner remain distinct entries. `packages` preserves first-seen order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestRunnerAttribution {
    /// The detected runner and the evidence that attributed it.
    pub usage: TestRunnerUsage,
    /// Names of the in-scope packages contributing this exact usage.
    pub packages: Vec<String>,
}

/// Collapse the per-package test runners across `scope` into distinct usages,
/// each carrying the packages that attribute it.
///
/// Two packages that resolve to the same `(runner, source)` collapse to one
/// entry (e.g. every crate governed by a single workspace-root `nextest.toml`);
/// differing usages stay separate so a caller can list which package uses what.
/// Per-package prioritization has already happened in `detect_test_runners`, so
/// `Package::test_runners` holds each package's effective runner(s).
#[must_use]
pub fn aggregate_test_runners(
    packages: &[Package],
    scope: &AggregateScope,
) -> Vec<TestRunnerAttribution> {
    let mut out: Vec<TestRunnerAttribution> = Vec::new();
    for pkg in packages.iter().filter(|p| in_scope(p, scope)) {
        for usage in &pkg.test_runners {
            if let Some(entry) = out.iter_mut().find(|e| &e.usage == usage) {
                if !entry.packages.contains(&pkg.name) {
                    entry.packages.push(pkg.name.clone());
                }
            } else {
                out.push(TestRunnerAttribution {
                    usage: usage.clone(),
                    packages: vec![pkg.name.clone()],
                });
            }
        }
    }
    out
}

/// Returns `true` when `pkg` belongs to `scope`.
fn in_scope(pkg: &Package, scope: &AggregateScope) -> bool {
    match scope {
        AggregateScope::Repo => true,
        AggregateScope::Package(name) => &pkg.name == name,
        AggregateScope::PackageArea(area) => &pkg.package_area == area,
    }
}

/// Where a package's declared version was read from.
///
/// `path` is always repo-root-relative so the JSON and verbose text can
/// hyperlink it directly. `inherited` is only `true` for Cargo packages
/// whose `[package].version = { workspace = true }` resolved to the root
/// `[workspace.package].version`; all other sources (literal Cargo versions,
/// npm, pyproject) are `inherited: false`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionSource {
    /// Manifest filename the version was read from
    /// (`Cargo.toml`, `package.json`, `pyproject.toml`).
    pub manifest: String,
    /// Repo-relative path to that manifest, for hyperlinks.
    pub path: String,
    /// True when the value was inherited from the workspace root
    /// (Cargo `version.workspace = true` → root `[workspace.package].version`).
    pub inherited: bool,
}

/// One physical version source, grouped with the packages that read it.
///
/// Used to report distinct manifest sources that contribute the same version
/// string. A workspace where every crate inherits one `[workspace.package]`
/// version collapses to one entry; a workspace where some packages inherit
/// and others declare a literal version exposes two source groups for the
/// same version string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionSourceAttribution {
    /// The manifest source for this group of packages.
    pub source: VersionSource,
    /// In-scope package names that read their version from this source.
    pub packages: Vec<String>,
}

/// A distinct version value across a scope, with the packages that carry it
/// and the manifest sources that produced that value.
///
/// Uniform repos collapse to one entry whose `sources` lists every manifest
/// that contributed; differing versions stay as separate entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionAttribution {
    /// The version string, exactly as declared by the manifest.
    pub version: String,
    /// In-scope packages contributing this exact version (first-seen order).
    pub packages: Vec<String>,
    /// Distinct manifest sources contributing this exact version (first-seen
    /// order). Multiple sources imply the same version was read from several
    /// manifests (e.g. a workspace-root Cargo value plus a literal override
    /// in one member crate).
    pub sources: Vec<VersionSourceAttribution>,
}

/// Collapse per-package versions across `scope` into distinct entries.
///
/// Packages sharing the same version string collapse into one entry that
/// names all of them; differing versions stay separate. Packages with no
/// resolvable version contribute nothing. Source evidence is grouped by
/// `(manifest, path, inherited)` so JSON never implies a version came from
/// one manifest when several packages read the same value from different
/// ones.
///
/// The manifest source is resolved at aggregation time by re-reading the
/// package's manifests in the same priority order as the detection-time
/// `resolve_package_version` (Cargo with workspace inheritance → npm with
/// root-fallback → pyproject). Re-parsing is the documented trade-off
/// (approach A) that keeps the per-package `Package` schema unchanged.
#[must_use]
pub fn aggregate_versions(
    packages: &[Package],
    scope: &AggregateScope,
    root: &Path,
) -> Vec<VersionAttribution> {
    // First pass: for each in-scope package with a resolvable version, capture
    // the (version, source) pair. Source resolution re-reads the manifest
    // stack so the per-package `Package` shape does not need to carry it.
    struct Resolved {
        package: String,
        version: String,
        source: VersionSource,
    }

    let mut resolved: Vec<Resolved> = Vec::new();
    for pkg in packages.iter().filter(|p| in_scope(p, scope)) {
        if let Some((version, source)) = resolve_version_source(pkg, root) {
            resolved.push(Resolved {
                package: pkg.name.clone(),
                version,
                source,
            });
        }
    }

    // Second pass: collapse by version, then group by source inside each
    // version bucket. First-seen order is preserved on both axes.
    let mut entries: Vec<VersionAttribution> = Vec::new();
    for entry in resolved {
        let bucket_idx = match entries.iter().position(|e| e.version == entry.version) {
            Some(idx) => idx,
            None => {
                let idx = entries.len();
                entries.push(VersionAttribution {
                    version: entry.version.clone(),
                    packages: Vec::new(),
                    sources: Vec::new(),
                });
                idx
            }
        };
        let bucket = &mut entries[bucket_idx];
        if !bucket.packages.contains(&entry.package) {
            bucket.packages.push(entry.package.clone());
        }
        let source_idx = match bucket.sources.iter().position(|s| s.source == entry.source) {
            Some(idx) => idx,
            None => {
                let idx = bucket.sources.len();
                bucket.sources.push(VersionSourceAttribution {
                    source: entry.source.clone(),
                    packages: Vec::new(),
                });
                idx
            }
        };
        let source_bucket = &mut bucket.sources[source_idx];
        if !source_bucket.packages.contains(&entry.package) {
            source_bucket.packages.push(entry.package);
        }
    }
    entries
}

/// Resolve a single package's version + manifest source by re-reading the
/// manifest stack in `resolve_package_version`'s priority order.
///
/// Returns `None` when no manifest in the package's directory (or the
/// repository root for npm) carries a version string.
fn resolve_version_source(pkg: &Package, root: &Path) -> Option<(String, VersionSource)> {
    let cargo_toml = pkg.path.join("Cargo.toml");
    if probe_exists(&cargo_toml)
        && let Some(parsed) = read_toml(&cargo_toml)
        && let Some((version, rel_path, inherited)) =
            cargo_package_version_with_source(&parsed, &cargo_toml, root)
    {
        return Some((
            version,
            VersionSource {
                manifest: "Cargo.toml".to_string(),
                path: rel_path,
                inherited,
            },
        ));
    }

    let package_json = pkg.path.join("package.json");
    if probe_exists(&package_json) {
        if let Some(parsed) = read_json(&package_json)
            && let Some(version) = npm_package_version(&parsed)
        {
            return Some((
                version,
                VersionSource {
                    manifest: "package.json".to_string(),
                    path: repo_relative(&package_json, root),
                    inherited: false,
                },
            ));
        }
        if pkg.path != root {
            let root_package_json = root.join("package.json");
            if probe_exists(&root_package_json)
                && let Some(parsed) = read_json(&root_package_json)
                && let Some(version) = npm_package_version(&parsed)
            {
                return Some((
                    version,
                    VersionSource {
                        manifest: "package.json".to_string(),
                        path: repo_relative(&root_package_json, root),
                        inherited: false,
                    },
                ));
            }
        }
    }

    let pyproject_toml = pkg.path.join("pyproject.toml");
    if probe_exists(&pyproject_toml)
        && let Some(parsed) = read_toml(&pyproject_toml)
        && let Some(version) = pyproject_package_version(&parsed)
    {
        return Some((
            version,
            VersionSource {
                manifest: "pyproject.toml".to_string(),
                path: repo_relative(&pyproject_toml, root),
                inherited: false,
            },
        ));
    }

    None
}

/// Resolve the version declared in a single directory's manifests, used as the
/// non-monorepo / no-catalog fallback for `sniff repo version`.
///
/// Unlike [`aggregate_versions`], which keys by package name, this returns
/// exactly one [`VersionAttribution`] with empty `packages` and a single
/// source — the directory's own manifest, read in the same priority order
/// as `resolve_package_version` (Cargo with workspace inheritance →
/// workspace-shared `version` → npm with root-fallback → pyproject).
/// Returns `None` when no manifest in the directory carries a version
/// string.
#[must_use]
pub fn resolve_directory_version(root: &Path) -> Option<VersionAttribution> {
    let cargo_toml = root.join("Cargo.toml");
    if probe_exists(&cargo_toml)
        && let Some(parsed) = read_toml(&cargo_toml)
        && let Some((version, rel_path, inherited)) =
            cargo_package_version_with_source(&parsed, &cargo_toml, root)
    {
        return Some(VersionAttribution {
            version,
            packages: Vec::new(),
            sources: vec![VersionSourceAttribution {
                source: VersionSource {
                    manifest: "Cargo.toml".to_string(),
                    path: rel_path,
                    inherited,
                },
                packages: Vec::new(),
            }],
        });
    }

    // Workspace-only root (`[workspace]` with `[workspace.package].version` but
    // no `[package]`) is a common shape — pin the directory to the shared
    // version so a single-member workspace reports its own declared version.
    if probe_exists(&cargo_toml)
        && let Some(parsed) = read_toml(&cargo_toml)
        && let Some(version) = parsed
            .get("workspace")
            .and_then(|w| w.get("package"))
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
    {
        return Some(VersionAttribution {
            version: version.to_string(),
            packages: Vec::new(),
            sources: vec![VersionSourceAttribution {
                source: VersionSource {
                    manifest: "Cargo.toml".to_string(),
                    path: "Cargo.toml".to_string(),
                    inherited: false,
                },
                packages: Vec::new(),
            }],
        });
    }

    let package_json = root.join("package.json");
    if probe_exists(&package_json)
        && let Some(parsed) = read_json(&package_json)
        && let Some(version) = npm_package_version(&parsed)
    {
        return Some(VersionAttribution {
            version,
            packages: Vec::new(),
            sources: vec![VersionSourceAttribution {
                source: VersionSource {
                    manifest: "package.json".to_string(),
                    path: repo_relative(&package_json, root),
                    inherited: false,
                },
                packages: Vec::new(),
            }],
        });
    }

    let pyproject_toml = root.join("pyproject.toml");
    if probe_exists(&pyproject_toml)
        && let Some(parsed) = read_toml(&pyproject_toml)
        && let Some(version) = pyproject_package_version(&parsed)
    {
        return Some(VersionAttribution {
            version,
            packages: Vec::new(),
            sources: vec![VersionSourceAttribution {
                source: VersionSource {
                    manifest: "pyproject.toml".to_string(),
                    path: repo_relative(&pyproject_toml, root),
                    inherited: false,
                },
                packages: Vec::new(),
            }],
        });
    }

    None
}

/// Collapse the bare-aggregate top-level `version` for a `SniffRepo` JSON.
///
/// Returns `Some(version)` when `aggregate_versions` at
/// `AggregateScope::Repo` finds **exactly one** distinct version across all
/// packages; returns `None` when the collapse finds zero or more than one
/// distinct version. The caller (the bare `sniff repo --json` aggregate
/// builder) treats `None` as `null`.
///
/// Centralizing the rule in the library keeps the CLI JSON builder free of
/// manifest knowledge — it just calls this helper and serializes the result
/// into the existing `Option<String>` slot. The serialized type does not
/// change, so consumers of `repo --json` see the same `string | null`
/// contract; only the value improves (e.g. a pure-virtual Cargo workspace
/// with uniform member versions now reports the version string instead of
/// `null`).
#[must_use]
pub fn bare_aggregate_version(packages: &[Package], root: &Path) -> Option<String> {
    let entries = aggregate_versions(packages, &AggregateScope::Repo, root);
    match entries.as_slice() {
        [single] => Some(single.version.clone()),
        _ => None,
    }
}

/// Compute a repo-relative path for a manifest file, with forward-slash
/// separators. Returns the original (non-relative) path as a fallback so
/// callers always get a non-empty value to display.
fn repo_relative(absolute: &Path, root: &Path) -> String {
    absolute
        .strip_prefix(root)
        .ok()
        .and_then(|p| p.to_str())
        .map(|s| s.replace('\\', "/"))
        .unwrap_or_else(|| {
            absolute
                .to_str()
                .map(|s| s.to_string())
                .unwrap_or_default()
        })
}

/// Resolve an explicit scope override, falling back to the CWD-derived scope
/// when no override is set.
///
/// Priority order: `--all` → `--package` → `--package-area` → CWD. The
/// three overrides are mutually exclusive at the clap layer (Phase 2), but
/// this function checks them defensively so a misuse does not panic.
///
/// `--package` errors when the named target does not match any catalog
/// package or matches more than one (ambiguous). `--package-area` errors
/// when the named area is not in the catalog; areas are unique by name, so
/// ambiguity is not a concern for them.
pub fn resolve_scope_with_overrides(
    info: &RepoInfo,
    cwd: &Path,
    all: bool,
    package: Option<&str>,
    package_area: Option<&str>,
) -> Result<AggregateScope> {
    if all {
        return Ok(AggregateScope::Repo);
    }
    if let Some(name) = package {
        return resolve_named_package(info, name);
    }
    if let Some(area) = package_area {
        return resolve_named_package_area(info, area);
    }
    Ok(resolve_scope(info, cwd))
}

fn resolve_named_package(info: &RepoInfo, name: &str) -> Result<AggregateScope> {
    let packages = info.packages.as_deref().unwrap_or(&[]);
    let mut matches: Vec<&str> = packages
        .iter()
        .filter(|p| p.name == name)
        .map(|p| p.name.as_str())
        .collect();
    match matches.len() {
        0 => {
            let mut valid: Vec<&str> =
                packages.iter().map(|p| p.name.as_str()).collect();
            valid.sort();
            valid.dedup();
            Err(crate::SniffError::UnknownPackage {
                name: name.to_string(),
                valid: valid.join(", "),
            })
        }
        1 => Ok(AggregateScope::Package(matches.pop().expect("len == 1").to_string())),
        _ => {
            matches.sort();
            Err(crate::SniffError::AmbiguousPackage {
                name: name.to_string(),
                count: matches.len(),
                matches: matches.join(", "),
            })
        }
    }
}

fn resolve_named_package_area(info: &RepoInfo, area: &str) -> Result<AggregateScope> {
    let packages = info.packages.as_deref().unwrap_or(&[]);
    let mut valid: Vec<&str> = packages.iter().map(|p| p.package_area.as_str()).collect();
    valid.sort();
    valid.dedup();
    if !valid.contains(&area) {
        return Err(crate::SniffError::UnknownPackageArea {
            area: area.to_string(),
            valid: valid.join(", "),
        });
    }
    Ok(AggregateScope::PackageArea(area.to_string()))
}

/// Collect external dependencies declared by packages in `scope`.
#[must_use]
pub fn collect_external_dependencies(
    repo: &RepoInfo,
    scope: &AggregateScope,
    filter: ExternalDependencyFilter,
) -> Vec<ExternalDependency> {
    let filter = filter.normalize();
    let mut out = Vec::new();
    let Some(packages) = repo.packages.as_deref() else {
        collect_repo_root_dependency_family(
            &mut out,
            "root",
            ExternalDependencyFamily::Dependencies,
            repo.dependencies.as_deref(),
            filter,
        );
        collect_repo_root_dependency_family(
            &mut out,
            "root",
            ExternalDependencyFamily::DevDependencies,
            repo.dev_dependencies.as_deref(),
            filter,
        );
        collect_repo_root_dependency_family(
            &mut out,
            "root",
            ExternalDependencyFamily::PeerDependencies,
            repo.peer_dependencies.as_deref(),
            filter,
        );
        collect_repo_root_dependency_family(
            &mut out,
            "root",
            ExternalDependencyFamily::OptionalDependencies,
            repo.optional_dependencies.as_deref(),
            filter,
        );
        return out;
    };

    for pkg in packages.iter().filter(|pkg| in_scope(pkg, scope)) {
        collect_dependency_family(
            &mut out,
            pkg,
            ExternalDependencyFamily::Dependencies,
            pkg.dependencies.as_deref(),
            filter,
        );
        collect_dependency_family(
            &mut out,
            pkg,
            ExternalDependencyFamily::DevDependencies,
            pkg.dev_dependencies.as_deref(),
            filter,
        );
        collect_dependency_family(
            &mut out,
            pkg,
            ExternalDependencyFamily::PeerDependencies,
            pkg.peer_dependencies.as_deref(),
            filter,
        );
        collect_dependency_family(
            &mut out,
            pkg,
            ExternalDependencyFamily::OptionalDependencies,
            pkg.optional_dependencies.as_deref(),
            filter,
        );
    }
    out
}

fn collect_dependency_family(
    out: &mut Vec<ExternalDependency>,
    pkg: &Package,
    family: ExternalDependencyFamily,
    deps: Option<&[crate::package::DependencyEntry]>,
    filter: ExternalDependencyFilter,
) {
    collect_repo_root_dependency_family(out, &pkg.name, family, deps, filter);
}

fn collect_repo_root_dependency_family(
    out: &mut Vec<ExternalDependency>,
    package: &str,
    family: ExternalDependencyFamily,
    deps: Option<&[crate::package::DependencyEntry]>,
    filter: ExternalDependencyFilter,
) {
    if !filter.includes(family) {
        return;
    }
    for dependency in deps.unwrap_or(&[]) {
        out.push(ExternalDependency {
            package: package.to_string(),
            family,
            dependency: dependency.clone(),
        });
    }
}

/// Packages and package areas that own a set of changed paths.
///
/// Both lists are `BTreeSet`-sorted and deduplicated.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PathAttribution {
    pub packages: Vec<String>,
    pub package_areas: Vec<String>,
}

/// Attribute repository-relative `paths` to the packages that contain them.
///
/// A path is attributed to *every* package whose `relative` prefix contains it,
/// so a path under a nested package is reported against both the nested package
/// and any ancestor package that also matches.
///
/// ## Notes
///
/// The `O(paths × packages)` lossy-string scan and the shallowest-match
/// semantics are the pre-existing CLI behavior, moved here verbatim so that
/// **Phase 4** has a single site at which to swap in the shared deepest-prefix
/// index (umbrella spec R6.4/R6.5). Do not tune the internals here: the
/// deepest-prefix change alters attribution results for nested packages and
/// must land as its own reviewed change.
pub fn attribute_paths(packages: &[Package], paths: &[std::path::PathBuf]) -> PathAttribution {
    let mut package_names = std::collections::BTreeSet::new();
    let mut package_areas = std::collections::BTreeSet::new();

    for path in paths {
        // Preserved lossiness: Phase 4 replaces this with a component-aware
        // `Path` comparison per umbrella spec R6.6.
        let path = path.to_string_lossy();
        for pkg in packages {
            if package_contains_path(pkg, &path, packages) {
                package_names.insert(pkg.name.clone());
                package_areas.insert(pkg.package_area.clone());
            }
        }
    }

    PathAttribution {
        packages: package_names.into_iter().collect(),
        package_areas: package_areas.into_iter().collect(),
    }
}

/// Whether `pkg` contains the repository-relative `path`.
///
/// A root package (`relative == ""`) has no prefix to match on, so it claims
/// only the paths that no *other*, non-root package claims — otherwise it would
/// claim the entire repository.
fn package_contains_path(pkg: &Package, path: &str, packages: &[Package]) -> bool {
    let prefix = pkg.relative.trim_start_matches("./");
    if prefix.is_empty() {
        return !packages.iter().any(|other| {
            let other_prefix = other.relative.trim_start_matches("./");
            !other_prefix.is_empty() && path.starts_with(other_prefix)
        });
    }
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::repo::PackageProvenance;
    use crate::filesystem::repo::standard::MonorepoStandard;
    use crate::filesystem::repo::test_runner_usage::TestRunnerSource;
    use crate::filesystem::repo::types::PackageEcosystem;
    use std::path::PathBuf;

    fn pkg(name: &str, area: &str, pms: &[&str]) -> Package {
        Package {
            name: name.to_string(),
            path: PathBuf::from(name),
            relative: name.to_string(),
            package_area: area.to_string(),
            ecosystem: PackageEcosystem::Unknown,
            standard: MonorepoStandard::Unknown,
            provenance: PackageProvenance::ManifestScan,
            package_managers: pms.iter().map(|s| (*s).to_string()).collect(),
            ..Package::default()
        }
    }

    fn pms_of(result: &AggregateResult<String>) -> Vec<String> {
        result.clone().into_values()
    }

    fn runner_pkg(name: &str, area: &str, usages: Vec<TestRunnerUsage>) -> Package {
        Package {
            test_runners: usages,
            ..pkg(name, area, &[])
        }
    }

    fn config(runner: crate::programs::enums::TestRunner, path: &str) -> TestRunnerUsage {
        TestRunnerUsage {
            runner,
            source: TestRunnerSource::Config {
                filename: path.to_string(),
                path: path.to_string(),
            },
        }
    }

    #[test]
    fn test_runner_shared_root_config_collapses_to_one_entry_naming_all_packages() {
        use crate::programs::enums::TestRunner;
        // Two crates governed by the same workspace-root nextest config: one
        // entry, both packages attributed.
        let packages = vec![
            runner_pkg("a", "x", vec![config(TestRunner::Nextest, ".config/nextest.toml")]),
            runner_pkg("b", "x", vec![config(TestRunner::Nextest, ".config/nextest.toml")]),
        ];
        let entries = aggregate_test_runners(&packages, &AggregateScope::Repo);
        assert_eq!(entries.len(), 1, "shared config should collapse, got {entries:?}");
        assert_eq!(entries[0].usage.runner, TestRunner::Nextest);
        assert_eq!(entries[0].packages, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_runner_distinct_usages_stay_separate_entries() {
        use crate::programs::enums::TestRunner;
        let packages = vec![
            runner_pkg("a", "x", vec![config(TestRunner::Nextest, ".config/nextest.toml")]),
            runner_pkg("b", "x", vec![config(TestRunner::Vitest, "b/vitest.config.ts")]),
        ];
        let entries = aggregate_test_runners(&packages, &AggregateScope::Repo);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].usage.runner, TestRunner::Vitest);
        assert_eq!(entries[1].packages, vec!["b".to_string()]);
    }

    #[test]
    fn singular_scope_returns_one_value() {
        let packages = vec![pkg("a", "root", &["cargo"])];
        let scope = AggregateScope::Package("a".to_string());
        let result = aggregate_package_values(
            &packages,
            &scope,
            |p| p.package_managers.clone(),
            |s| s.clone(),
        );
        assert_eq!(pms_of(&result), vec!["cargo".to_string()]);
        assert!(result.is_singular());
    }

    #[test]
    fn package_area_uniform_collapses_to_singular() {
        let packages = vec![pkg("a", "sniff", &["cargo"]), pkg("b", "sniff", &["cargo"])];
        let scope = AggregateScope::PackageArea("sniff".to_string());
        let result = aggregate_package_values(
            &packages,
            &scope,
            |p| p.package_managers.clone(),
            |s| s.clone(),
        );
        assert!(result.is_singular());
        assert_eq!(pms_of(&result), vec!["cargo".to_string()]);
    }

    #[test]
    fn package_area_variant_collapses_to_unique_list() {
        let packages = vec![
            pkg("a", "sniff", &["cargo"]),
            pkg("b", "sniff", &["npm"]),
            pkg("c", "sniff", &["cargo"]),
        ];
        let scope = AggregateScope::PackageArea("sniff".to_string());
        let result = aggregate_package_values(
            &packages,
            &scope,
            |p| p.package_managers.clone(),
            |s| s.clone(),
        );
        assert_eq!(pms_of(&result), vec!["cargo", "npm"]);
        assert!(!result.is_singular());
    }

    #[test]
    fn repo_scope_unions_all_packages() {
        let packages = vec![pkg("a", "sniff", &["cargo"]), pkg("b", "tools", &["npm"])];
        let scope = AggregateScope::Repo;
        let result = aggregate_package_values(
            &packages,
            &scope,
            |p| p.package_managers.clone(),
            |s| s.clone(),
        );
        assert_eq!(pms_of(&result), vec!["cargo", "npm"]);
    }

    #[test]
    fn collapse_dedupes_within_a_package() {
        let packages = vec![pkg("a", "root", &["cargo", "cargo"])];
        let scope = AggregateScope::Repo;
        let result = aggregate_package_values(
            &packages,
            &scope,
            |p| p.package_managers.clone(),
            |s| s.clone(),
        );
        assert_eq!(pms_of(&result), vec!["cargo".to_string()]);
    }

    #[test]
    fn empty_scope_yields_empty_result() {
        let packages: Vec<Package> = vec![];
        let scope = AggregateScope::Repo;
        let result = aggregate_package_values(
            &packages,
            &scope,
            |p| p.package_managers.clone(),
            |s| s.clone(),
        );
        assert_eq!(pms_of(&result), Vec::<String>::new());
        assert!(matches!(result, AggregateResult::Empty));
    }

    #[test]
    fn resolve_scope_non_monorepo_single_package() {
        let info = RepoInfo {
            is_monorepo: false,
            root: PathBuf::from("/repo"),
            packages: Some(vec![pkg("only", "root", &["cargo"])]),
            ..RepoInfo::default()
        };
        assert_eq!(
            resolve_scope(&info, Path::new("/repo")),
            AggregateScope::Package("only".to_string())
        );
    }

    #[test]
    fn resolve_scope_monorepo_inside_package() {
        let mut info = RepoInfo {
            is_monorepo: true,
            root: PathBuf::from("/repo"),
            packages: Some(vec![pkg("sniff-lib", "sniff", &["cargo"])]),
            ..RepoInfo::default()
        };
        // Point the package path at the temp-ish location used by `pkg`.
        info.packages.as_mut().unwrap()[0].path = PathBuf::from("/repo/sniff/lib");
        assert_eq!(
            resolve_scope(&info, Path::new("/repo/sniff/lib/src")),
            AggregateScope::Package("sniff-lib".to_string())
        );
    }

    #[test]
    fn resolve_scope_monorepo_at_root() {
        let info = RepoInfo {
            is_monorepo: true,
            root: PathBuf::from("/repo"),
            packages: Some(vec![pkg("a", "sniff", &["cargo"])]),
            ..RepoInfo::default()
        };
        assert_eq!(
            resolve_scope(&info, Path::new("/repo")),
            AggregateScope::Repo
        );
    }

    // --- aggregate_versions ---------------------------------------------------

    /// Create a synthetic `Package` with a populated `path` so the source
    /// resolver can read manifests off disk. `version` is intentionally left
    /// `None` on the package — `aggregate_versions` re-resolves source
    /// independently and never trusts the cached `Package.version`.
    fn pkg_with_path(name: &str, area: &str, path: PathBuf) -> Package {
        Package {
            path,
            relative: name.to_string(),
            package_area: area.to_string(),
            ..pkg(name, area, &[])
        }
    }

    fn write_cargo_toml(dir: &Path, body: &str) {
        std::fs::write(dir.join("Cargo.toml"), body).unwrap();
    }

    fn write_package_json(dir: &Path, body: &str) {
        std::fs::write(dir.join("package.json"), body).unwrap();
    }

    #[test]
    fn aggregate_versions_uniform_collapses_to_one_entry_naming_all_packages() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let a = root.join("a");
        let b = root.join("b");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        write_cargo_toml(&a, "[package]\nname = \"a\"\nversion = \"0.1.0\"\n");
        write_cargo_toml(&b, "[package]\nname = \"b\"\nversion = \"0.1.0\"\n");

        let packages = vec![
            pkg_with_path("a", "root", a.clone()),
            pkg_with_path("b", "root", b.clone()),
        ];
        let entries = aggregate_versions(&packages, &AggregateScope::Repo, root);
        assert_eq!(entries.len(), 1, "uniform should collapse, got {entries:?}");
        assert_eq!(entries[0].version, "0.1.0");
        assert_eq!(
            entries[0].packages,
            vec!["a".to_string(), "b".to_string()],
            "packages should preserve first-seen order"
        );
        // Two distinct per-package manifests produce two distinct source
        // groups, but both contribute the same version.
        assert_eq!(entries[0].sources.len(), 2);
        for source in &entries[0].sources {
            assert_eq!(source.source.manifest, "Cargo.toml");
            assert!(!source.source.inherited);
        }
        let source_paths: std::collections::HashSet<&str> = entries[0]
            .sources
            .iter()
            .map(|s| s.source.path.as_str())
            .collect();
        assert!(source_paths.contains("a/Cargo.toml"));
        assert!(source_paths.contains("b/Cargo.toml"));
    }

    #[test]
    fn aggregate_versions_variant_keeps_each_version_separate() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let a = root.join("a");
        let b = root.join("b");
        let c = root.join("c");
        for dir in [&a, &b, &c] {
            std::fs::create_dir_all(dir).unwrap();
        }
        write_cargo_toml(&a, "[package]\nname = \"a\"\nversion = \"1.0.0\"\n");
        write_cargo_toml(&b, "[package]\nname = \"b\"\nversion = \"2.0.0\"\n");
        write_cargo_toml(&c, "[package]\nname = \"c\"\nversion = \"1.0.0\"\n");

        let packages = vec![
            pkg_with_path("a", "root", a),
            pkg_with_path("b", "root", b),
            pkg_with_path("c", "root", c),
        ];
        let entries = aggregate_versions(&packages, &AggregateScope::Repo, root);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].version, "1.0.0");
        assert_eq!(entries[0].packages, vec!["a".to_string(), "c".to_string()]);
        assert_eq!(entries[1].version, "2.0.0");
        assert_eq!(entries[1].packages, vec!["b".to_string()]);
    }

    #[test]
    fn aggregate_versions_single_package_returns_one_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let a = root.join("a");
        std::fs::create_dir_all(&a).unwrap();
        write_cargo_toml(&a, "[package]\nname = \"a\"\nversion = \"0.3.1\"\n");

        let packages = vec![pkg_with_path("a", "root", a)];
        let entries = aggregate_versions(&packages, &AggregateScope::Repo, root);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].version, "0.3.1");
        assert_eq!(entries[0].packages, vec!["a".to_string()]);
    }

    #[test]
    fn aggregate_versions_empty_scope_yields_empty_result() {
        let entries = aggregate_versions(&[], &AggregateScope::Repo, Path::new("/repo"));
        assert!(entries.is_empty(), "empty input should produce no entries");
    }

    #[test]
    fn aggregate_versions_package_with_no_manifest_contributes_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let a = root.join("a");
        std::fs::create_dir_all(&a).unwrap();
        // No manifest at all under `a`.

        let packages = vec![pkg_with_path("a", "root", a)];
        let entries = aggregate_versions(&packages, &AggregateScope::Repo, root);
        assert!(entries.is_empty(), "no manifest → no entry, got {entries:?}");
    }

    #[test]
    fn aggregate_versions_multiple_manifest_sources_for_same_version() {
        // One package reads its version from a per-package Cargo.toml; another
        // reads the same version from the root `package.json` (npm root
        // fallback). The two sources must be reported as distinct
        // `VersionSourceAttribution` entries under the same version — never
        // collapsed into a misleading single source.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let a = root.join("a");
        let b = root.join("b");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        write_cargo_toml(
            &a,
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n",
        );
        // b has a package.json with no version field; resolution falls
        // through to the root package.json (the npm root-fallback).
        write_package_json(&b, r#"{"name":"b"}"#);
        write_package_json(root, r#"{"name":"root","version":"0.1.0"}"#);

        let packages = vec![pkg_with_path("a", "root", a), pkg_with_path("b", "root", b)];
        let entries = aggregate_versions(&packages, &AggregateScope::Repo, root);
        assert_eq!(entries.len(), 1, "same version collapses to one entry");
        assert_eq!(entries[0].version, "0.1.0");
        assert_eq!(
            entries[0].packages,
            vec!["a".to_string(), "b".to_string()]
        );
        assert!(
            entries[0].sources.len() > 1,
            "expected multiple sources for the same version, got {entries:?}"
        );
        let manifests: Vec<&str> = entries[0]
            .sources
            .iter()
            .map(|s| s.source.manifest.as_str())
            .collect();
        assert!(manifests.contains(&"Cargo.toml"));
        assert!(manifests.contains(&"package.json"));
        // The `a` package attributes to the Cargo source; the `b` package
        // attributes to the package.json source — never the other way around.
        let cargo_src = entries[0]
            .sources
            .iter()
            .find(|s| s.source.manifest == "Cargo.toml")
            .unwrap();
        assert_eq!(cargo_src.packages, vec!["a".to_string()]);
        let npm_src = entries[0]
            .sources
            .iter()
            .find(|s| s.source.manifest == "package.json")
            .unwrap();
        assert_eq!(npm_src.packages, vec!["b".to_string()]);
        assert_eq!(npm_src.source.path, "package.json");
    }

    #[test]
    fn aggregate_versions_workspace_inheritance_resolves_to_root_with_inherited_flag() {
        // Two member crates both use `version.workspace = true`; the value
        // comes from the root `[workspace.package].version` and must be
        // reported with `inherited: true` and the root manifest as the
        // source.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let a = root.join("a");
        let b = root.join("b");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        write_cargo_toml(
            root,
            "[workspace]\nmembers = [\"a\", \"b\"]\n\n[workspace.package]\nversion = \"2.0.0\"\n",
        );
        write_cargo_toml(
            &a,
            "[package]\nname = \"a\"\nversion.workspace = true\n",
        );
        write_cargo_toml(
            &b,
            "[package]\nname = \"b\"\nversion.workspace = true\n",
        );

        let packages = vec![pkg_with_path("a", "root", a), pkg_with_path("b", "root", b)];
        let entries = aggregate_versions(&packages, &AggregateScope::Repo, root);
        assert_eq!(entries.len(), 1, "shared workspace version collapses, got {entries:?}");
        assert_eq!(entries[0].version, "2.0.0");
        assert_eq!(
            entries[0].packages,
            vec!["a".to_string(), "b".to_string()]
        );
        assert_eq!(entries[0].sources.len(), 1);
        assert_eq!(entries[0].sources[0].source.manifest, "Cargo.toml");
        assert!(
            entries[0].sources[0].source.inherited,
            "inherited flag must be set for workspace-inherited versions"
        );
        assert_eq!(
            entries[0].sources[0].source.path, "Cargo.toml",
            "inherited source points at the root manifest"
        );
    }

    #[test]
    fn aggregate_versions_workspace_inheritance_missing_root_version_returns_none() {
        // `version.workspace = true` with no `[workspace.package].version` at
        // the root must contribute nothing — never panic, never fall back to
        // a literal.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let a = root.join("a");
        std::fs::create_dir_all(&a).unwrap();
        write_cargo_toml(root, "[workspace]\nmembers = [\"a\"]\n");
        write_cargo_toml(
            &a,
            "[package]\nname = \"a\"\nversion.workspace = true\n",
        );

        let packages = vec![pkg_with_path("a", "root", a)];
        let entries = aggregate_versions(&packages, &AggregateScope::Repo, root);
        assert!(
            entries.is_empty(),
            "missing root workspace version should contribute nothing, got {entries:?}"
        );
    }

    #[test]
    fn aggregate_versions_npm_root_fallback_for_package() {
        // A package with a `package.json` that has no `version` field should
        // fall back to the root `package.json` per the
        // `resolve_package_version` priority order.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let pkg_dir = root.join("web");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        write_package_json(&pkg_dir, r#"{"name":"web"}"#);
        write_package_json(root, r#"{"name":"app","version":"0.9.3"}"#);

        let packages = vec![pkg_with_path("web", "root", pkg_dir)];
        let entries = aggregate_versions(&packages, &AggregateScope::Repo, root);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].version, "0.9.3");
        assert_eq!(entries[0].sources.len(), 1);
        assert_eq!(entries[0].sources[0].source.manifest, "package.json");
        assert_eq!(entries[0].sources[0].source.path, "package.json");
        assert!(!entries[0].sources[0].source.inherited);
    }

    #[test]
    fn aggregate_versions_respects_package_area_scope() {
        // `AggregateScope::PackageArea("sniff")` should select only the
        // packages in that area, leaving others out.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let sniff_pkg = root.join("sniff").join("lib");
        let other_pkg = root.join("tools").join("cli");
        std::fs::create_dir_all(&sniff_pkg).unwrap();
        std::fs::create_dir_all(&other_pkg).unwrap();
        write_cargo_toml(
            &sniff_pkg,
            "[package]\nname = \"sniff-lib\"\nversion = \"1.0.0\"\n",
        );
        write_cargo_toml(
            &other_pkg,
            "[package]\nname = \"tools-cli\"\nversion = \"2.0.0\"\n",
        );

        let packages = vec![
            pkg_with_path("sniff-lib", "sniff", sniff_pkg),
            pkg_with_path("tools-cli", "tools", other_pkg),
        ];
        let entries = aggregate_versions(
            &packages,
            &AggregateScope::PackageArea("sniff".to_string()),
            root,
        );
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].version, "1.0.0");
        assert_eq!(entries[0].packages, vec!["sniff-lib".to_string()]);
    }

    // --- resolve_scope_with_overrides ----------------------------------------

    fn info_with_areas(area_names: &[&str]) -> RepoInfo {
        let packages: Vec<Package> = area_names
            .iter()
            .map(|name| pkg(name, "root", &["cargo"]))
            .collect();
        RepoInfo {
            is_monorepo: true,
            root: PathBuf::from("/repo"),
            packages: Some(packages),
            ..RepoInfo::default()
        }
    }

    #[test]
    fn resolve_scope_overrides_all_returns_repo_scope() {
        let info = info_with_areas(&["a", "b"]);
        let scope = resolve_scope_with_overrides(&info, Path::new("/repo"), true, None, None)
            .expect("`--all` always resolves to Repo");
        assert_eq!(scope, AggregateScope::Repo);
    }

    #[test]
    fn resolve_scope_overrides_valid_package() {
        let info = info_with_areas(&["alpha", "beta"]);
        let scope = resolve_scope_with_overrides(
            &info,
            Path::new("/repo"),
            false,
            Some("alpha"),
            None,
        )
        .expect("known package should resolve");
        assert_eq!(scope, AggregateScope::Package("alpha".to_string()));
    }

    #[test]
    fn resolve_scope_overrides_unknown_package_errors() {
        let info = info_with_areas(&["alpha", "beta"]);
        let err = resolve_scope_with_overrides(
            &info,
            Path::new("/repo"),
            false,
            Some("ghost"),
            None,
        )
        .expect_err("unknown package should error");
        match err {
            crate::SniffError::UnknownPackage { name, valid } => {
                assert_eq!(name, "ghost");
                assert!(valid.contains("alpha"));
                assert!(valid.contains("beta"));
            }
            other => panic!("expected UnknownPackage, got {other:?}"),
        }
    }

    #[test]
    fn resolve_scope_overrides_ambiguous_package_errors() {
        // Two catalog entries with the same name (e.g. workspace with two
        // members both named `core`). The function must surface this as
        // ambiguous rather than silently picking one.
        let mut info = info_with_areas(&["core", "other"]);
        // Inject a second "core" entry to simulate the duplicate.
        info.packages
            .as_mut()
            .unwrap()
            .push(pkg("core", "tools", &["cargo"]));
        let err = resolve_scope_with_overrides(
            &info,
            Path::new("/repo"),
            false,
            Some("core"),
            None,
        )
        .expect_err("ambiguous package should error");
        match err {
            crate::SniffError::AmbiguousPackage { name, count, .. } => {
                assert_eq!(name, "core");
                assert_eq!(count, 2);
            }
            other => panic!("expected AmbiguousPackage, got {other:?}"),
        }
    }

    #[test]
    fn resolve_scope_overrides_valid_package_area() {
        let mut info = info_with_areas(&["a"]);
        info.packages.as_mut().unwrap()[0].package_area = "sniff".to_string();
        let scope = resolve_scope_with_overrides(
            &info,
            Path::new("/repo"),
            false,
            None,
            Some("sniff"),
        )
        .expect("known area should resolve");
        assert_eq!(scope, AggregateScope::PackageArea("sniff".to_string()));
    }

    #[test]
    fn resolve_scope_overrides_unknown_package_area_errors() {
        let info = info_with_areas(&["a"]);
        let err = resolve_scope_with_overrides(
            &info,
            Path::new("/repo"),
            false,
            None,
            Some("ghost"),
        )
        .expect_err("unknown area should error");
        match err {
            crate::SniffError::UnknownPackageArea { area, valid } => {
                assert_eq!(area, "ghost");
                assert!(valid.contains("root"));
            }
            other => panic!("expected UnknownPackageArea, got {other:?}"),
        }
    }

    #[test]
    fn resolve_scope_overrides_cwd_default_when_no_flags() {
        // No override → falls through to the existing `resolve_scope`. Inside
        // a package directory, that returns the package scope.
        let mut info = info_with_areas(&["sniff-lib"]);
        info.packages.as_mut().unwrap()[0].path = PathBuf::from("/repo/sniff/lib");
        let scope = resolve_scope_with_overrides(
            &info,
            Path::new("/repo/sniff/lib/src"),
            false,
            None,
            None,
        )
        .expect("no overrides should never error");
        assert_eq!(scope, AggregateScope::Package("sniff-lib".to_string()));
    }

    // --- bare_aggregate_version ---------------------------------------------

    #[test]
    fn bare_aggregate_version_uniform_returns_the_shared_string() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let a = root.join("a");
        let b = root.join("b");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        write_cargo_toml(&a, "[package]\nname = \"a\"\nversion = \"0.1.0\"\n");
        write_cargo_toml(&b, "[package]\nname = \"b\"\nversion = \"0.1.0\"\n");

        let packages = vec![
            pkg_with_path("a", "root", a),
            pkg_with_path("b", "root", b),
        ];
        assert_eq!(
            bare_aggregate_version(&packages, root),
            Some("0.1.0".to_string())
        );
    }

    #[test]
    fn bare_aggregate_version_zero_packages_returns_none() {
        // Empty catalog: no resolvable version anywhere, so the collapse
        // reports `null` (matching the legacy "missing" semantic).
        assert!(bare_aggregate_version(&[], Path::new("/repo")).is_none());
    }

    #[test]
    fn bare_aggregate_version_two_distinct_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let a = root.join("a");
        let b = root.join("b");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        write_cargo_toml(&a, "[package]\nname = \"a\"\nversion = \"1.0.0\"\n");
        write_cargo_toml(&b, "[package]\nname = \"b\"\nversion = \"2.0.0\"\n");

        let packages = vec![
            pkg_with_path("a", "root", a),
            pkg_with_path("b", "root", b),
        ];
        assert!(
            bare_aggregate_version(&packages, root).is_none(),
            "variance must collapse to `None`"
        );
    }

    #[test]
    fn bare_aggregate_version_workspace_inheritance_collapses_to_root_version() {
        // A pure-virtual root with `[workspace.package].version` and one
        // inheriting member: aggregate finds one distinct version
        // (`"0.1.0"`) and reports it.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write_cargo_toml(
            root,
            "[workspace]\nmembers = [\"a\"]\n[workspace.package]\nversion = \"0.1.0\"\n",
        );
        let a = root.join("a");
        std::fs::create_dir_all(&a).unwrap();
        write_cargo_toml(&a, "[package]\nname = \"a\"\nversion.workspace = true\n");

        let packages = vec![pkg_with_path("a", "root", a)];
        assert_eq!(
            bare_aggregate_version(&packages, root),
            Some("0.1.0".to_string())
        );
    }
}
