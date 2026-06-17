//! Shared aggregation helper for collapsing per-package values into a
//! scope-bounded distinct set.
//!
//! Used by `sniff repo test-runner` and (in a later phase) `sniff repo
//! package-manager`. The collapse rule from `2026-06-14-more-repo/spec.md`:
//!
//! ```text
//! package        -> singular value for that package
//! package-area   -> union across contained packages; uniform -> singular,
//!                    else unique list
//! repo root      -> union across all packages; uniform -> singular, else list
//! ```

use std::collections::HashSet;
use std::path::Path;

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
}
