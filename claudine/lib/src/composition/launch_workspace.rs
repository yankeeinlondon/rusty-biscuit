use std::path::{Path, PathBuf};

use sniff::filesystem::repo::{Package, RepoInfo};

/// Workspace context for the directory where a provider child process
/// will be launched.
///
/// Carries the launch CWD, detected repo root, child process working
/// directory, and optional monorepo package metadata. Canonical command paths
/// project it from request-owned repository evidence without another
/// filesystem walk.
#[derive(Debug, Clone)]
pub struct LaunchWorkspaceContext {
    pub launch_cwd: PathBuf,
    pub repo_root: Option<PathBuf>,
    pub child_cwd: PathBuf,
    pub package_context: Option<PackageContext>,
    pub warnings: Vec<String>,
}

impl Default for LaunchWorkspaceContext {
    fn default() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            launch_cwd: cwd.clone(),
            repo_root: None,
            child_cwd: cwd,
            package_context: None,
            warnings: Vec::new(),
        }
    }
}

impl LaunchWorkspaceContext {
    /// Project a launch workspace from an already observed repository.
    ///
    /// `source_repo_root` affects repository-scoped metadata only. The child
    /// working directory remains anchored to the repository containing the
    /// launch directory, preserving the existing cross-repository composition
    /// contract without another filesystem discovery pass.
    pub fn from_repo_info(
        launch_cwd: &Path,
        launch_repo_root: Option<&Path>,
        repo: Option<&RepoInfo>,
        source_repo_root: Option<&Path>,
    ) -> Self {
        let launch_repo_root = launch_repo_root
            .map(Path::to_path_buf)
            .or_else(|| repo.map(|repo| repo.root.clone()));
        let repo_root = source_repo_root
            .map(Path::to_path_buf)
            .or_else(|| launch_repo_root.clone());
        let child_cwd = launch_repo_root
            .clone()
            .unwrap_or_else(|| launch_cwd.to_path_buf());

        let (package_context, warnings) = match repo {
            Some(repo) if repo.is_monorepo => match repo.packages.as_deref() {
                Some(packages) => package_context_for_dir(launch_cwd, repo, packages),
                None => (
                    None,
                    vec![format!(
                        "monorepo detected at '{}' but no packages were reported",
                        biscuit_file::to_portable_string(&repo.root)
                    )],
                ),
            },
            _ => (None, Vec::new()),
        };

        Self {
            launch_cwd: launch_cwd.to_path_buf(),
            repo_root,
            child_cwd,
            package_context,
            warnings,
        }
    }
}

fn package_context_for_dir(
    cwd: &Path,
    repo: &RepoInfo,
    packages: &[Package],
) -> (Option<PackageContext>, Vec<String>) {
    if let Some(package) = deepest_package(cwd, packages) {
        return (
            Some(PackageContext {
                package_area: package.package_area.clone(),
                package: Some(package.name.clone()),
                candidates: vec![package.name.clone()],
            }),
            Vec::new(),
        );
    }

    if let Some(area) = deepest_package_area(cwd, repo, packages) {
        let mut candidates = packages
            .iter()
            .filter(|package| package.package_area == area)
            .map(|package| package.name.clone())
            .collect::<Vec<_>>();
        candidates.sort();
        return (
            Some(PackageContext {
                package_area: area,
                package: None,
                candidates,
            }),
            Vec::new(),
        );
    }

    (
        None,
        vec![format!(
            "monorepo detected at '{}' but no package area matched cwd '{}'",
            biscuit_file::to_portable_string(&repo.root),
            biscuit_file::to_portable_string(cwd)
        )],
    )
}

fn deepest_package<'a>(cwd: &Path, packages: &'a [Package]) -> Option<&'a Package> {
    packages
        .iter()
        .filter(|package| cwd.starts_with(&package.path))
        .max_by_key(|package| package.path.components().count())
}

fn deepest_package_area(cwd: &Path, repo: &RepoInfo, packages: &[Package]) -> Option<String> {
    packages
        .iter()
        .map(|package| {
            let path = if package.package_area == "root" {
                repo.root.clone()
            } else {
                repo.root.join(&package.package_area)
            };
            (path, package.package_area.clone())
        })
        .filter(|(path, _)| cwd.starts_with(path))
        .max_by_key(|(path, _)| path.components().count())
        .map(|(_, area)| area)
}

/// Monorepo package metadata for a launch workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageContext {
    pub package_area: String,
    pub package: Option<String>,
    pub candidates: Vec<String>,
}
