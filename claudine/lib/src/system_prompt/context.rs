use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use sniff::filesystem::repo::Package;
use sniff::request::*;

/// Filesystem roots resolved from the directory the user launched Claudine in.
///
/// Existing paths are canonicalized and projected without Windows verbatim
/// prefixes. Fields are `None` when the corresponding scope is not detected
/// (e.g. not inside a git repo or monorepo package).
#[derive(Debug, Clone)]
pub struct LaunchContext {
    /// The working directory Claudine was launched from.
    pub cwd: PathBuf,
    /// Git repository root, if detected.
    pub repo_root: Option<PathBuf>,
    /// Package-area root inside a monorepo.
    ///
    /// For the "root" package area this equals `repo_root`.
    pub package_area_root: Option<PathBuf>,
    /// Deepest matching workspace package root.
    pub package_root: Option<PathBuf>,
    /// The provider slug (e.g. "claude", "codex") to inject into
    /// `{{env.AGENT}}` template references during system-prompt composition.
    /// When `None`, the wrapper falls back to the process environment.
    pub agent: Option<String>,
}

impl LaunchContext {
    /// Build a launch context from the given working directory.
    ///
    /// Uses a single `sniff::detect_with_plan` call with git summary
    /// and repo structure detection.
    pub fn from_cwd(cwd: &Path) -> Result<Self, crate::error::ClaudineError> {
        let plan = DetectionPlan::new()
            .base_dir(cwd.to_path_buf())
            .without_os()
            .without_hardware()
            .without_network()
            .filesystem(
                FilesystemRequest::new()
                    .git(GitRequest::summary())
                    .repo(RepoRequest::structure())
                    .without_file_inventory()
                    .without_docs()
                    .without_formatting(),
            );

        let result = sniff::detect_with_plan(plan)
            .map_err(|e| crate::error::ClaudineError::LaunchContextDetection(Arc::new(e)))?;

        Ok(Self::from_sniff_result(&result, cwd))
    }

    /// Build a launch context from a pre-computed `SniffResult`.
    ///
    /// Use this when the caller already has the output of
    /// `sniff::detect_with_plan` (for example because another component
    /// in the same startup path needs an `EnvironmentContext` from the
    /// same plan) and wants to avoid triggering a second filesystem walk.
    ///
    /// The caller is responsible for supplying a plan that includes at
    /// least git summary + repo structure, otherwise the resulting
    /// context will have no `repo_root` / package scopes.
    pub fn from_sniff_result(result: &sniff::SniffResult, cwd: &Path) -> Self {
        let fs = result.filesystem.as_ref();
        let git_root = fs.and_then(|f| f.git.as_ref()).map(|g| g.repo_root.clone());
        let repo = fs.and_then(|f| f.repo.as_ref());

        let (package_root, package_area_root) = match repo {
            Some(repo) if repo.is_monorepo => {
                if let Some(packages) = repo.packages.as_deref() {
                    let pkg_root = select_package_root(cwd, packages);
                    let area_root = select_package_area_root(cwd, &repo.root, packages);
                    (pkg_root, area_root)
                } else {
                    (None, None)
                }
            }
            _ => (None, None),
        };

        LaunchContext {
            cwd: projected_path(cwd),
            repo_root: git_root.map(|p| projected_path(&p)),
            package_area_root: package_area_root.map(|p| projected_path(&p)),
            package_root: package_root.map(|p| projected_path(&p)),
            agent: None,
        }
    }

    /// Deduplicated search directories in precedence order.
    ///
    /// Returns unique paths for: package root, package-area root, repo root.
    /// The user-home scope (`~/.claudine/`) is NOT included here — the
    /// caller adds it as a final fallback.
    pub fn search_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::with_capacity(3);
        let mut seen = HashSet::new();
        for p in [&self.package_root, &self.package_area_root, &self.repo_root]
            .into_iter()
            .flatten()
        {
            if seen.insert(p.clone()) {
                dirs.push(p.clone());
            }
        }
        dirs
    }
}

/// Select the deepest matching package root for the given cwd.
fn select_package_root(cwd: &Path, packages: &[Package]) -> Option<PathBuf> {
    let cwd_normalized = projected_path(cwd);

    packages
        .iter()
        .filter_map(|package| {
            let package_path = projected_path(&package.path);
            if cwd_normalized.starts_with(&package_path) {
                Some((package_path.components().count(), package_path))
            } else {
                None
            }
        })
        .max_by_key(|(depth, _)| *depth)
        .map(|(_, path)| path)
}

/// Select the deepest matching package-area root for the given cwd.
fn select_package_area_root(cwd: &Path, repo_root: &Path, packages: &[Package]) -> Option<PathBuf> {
    let cwd_normalized = projected_path(cwd);
    let repo_root_normalized = projected_path(repo_root);

    packages
        .iter()
        .map(|package| {
            if package.package_area == "root" {
                repo_root_normalized.clone()
            } else {
                repo_root_normalized.join(&package.package_area)
            }
        })
        .filter(|area_root| cwd_normalized.starts_with(area_root))
        .max_by_key(|area_root| area_root.components().count())
}

/// Path form safe for authored comparisons and public projections.
pub(crate) fn projected_path(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| dunce::simplified(path).to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    #[cfg(windows)]
    fn assert_not_verbatim(path: &Path) {
        assert!(
            !path.to_string_lossy().starts_with(r"\\?\"),
            "projected path retained a Windows verbatim prefix: {}",
            path.display()
        );
    }

    #[cfg(windows)]
    #[test]
    fn projected_paths_simplify_existing_drive_and_authored_unc_forms() {
        let existing = TempDir::new().unwrap();
        let drive_projection = projected_path(existing.path());
        assert_not_verbatim(&drive_projection);
        assert_eq!(drive_projection, dunce::canonicalize(existing.path()).unwrap());

        let unc = Path::new(r"\\server\share\.hidden\missing");
        let unc_projection = projected_path(unc);
        assert_not_verbatim(&unc_projection);
        assert_eq!(unc_projection, dunce::simplified(unc));
    }

    /// Initialize a real git repository in the given directory.
    fn init_git_repo(dir: &Path) {
        Command::new("git")
            .args(["init", "--initial-branch=main"])
            .current_dir(dir)
            .output()
            .expect("git init failed");
        // Create an initial commit so HEAD is valid
        Command::new("git")
            .args(["commit", "--allow-empty", "-m", "init"])
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .output()
            .expect("git commit failed");
    }

    /// Create a Cargo workspace toml that declares packages.
    fn write_cargo_workspace(repo_root: &Path, members: &[&str]) {
        let members_str: Vec<String> = members.iter().map(|m| format!("    \"{m}\"")).collect();
        let content = format!("[workspace]\nmembers = [\n{}\n]\n", members_str.join(",\n"));
        fs::write(repo_root.join("Cargo.toml"), content).unwrap();
    }

    /// Create a package Cargo.toml with a given name.
    fn write_package_toml(package_dir: &Path, name: &str) {
        fs::create_dir_all(package_dir).unwrap();
        let content =
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n");
        fs::write(package_dir.join("Cargo.toml"), content).unwrap();
    }

    #[test]
    fn from_cwd_in_package() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        init_git_repo(root);
        write_cargo_workspace(root, &["claudine/lib", "claudine/cli"]);
        write_package_toml(&root.join("claudine/lib"), "claudine");
        write_package_toml(&root.join("claudine/cli"), "claudine-cli");

        let cwd = root.join("claudine/cli");
        let ctx = LaunchContext::from_cwd(&cwd).unwrap();

        assert!(ctx.package_root.is_some(), "package_root should be set");
        let pkg_root = ctx.package_root.unwrap();
        assert!(
            pkg_root.ends_with("claudine/cli"),
            "package_root should end with claudine/cli, got: {}",
            pkg_root.display()
        );

        assert!(
            ctx.package_area_root.is_some(),
            "package_area_root should be set"
        );
        let area_root = ctx.package_area_root.unwrap();
        assert!(
            area_root.ends_with("claudine"),
            "package_area_root should end with claudine, got: {}",
            area_root.display()
        );

        assert!(ctx.repo_root.is_some(), "repo_root should be set");
    }

    #[test]
    fn from_cwd_in_package_area() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        init_git_repo(root);
        write_cargo_workspace(root, &["claudine/lib", "claudine/cli"]);
        write_package_toml(&root.join("claudine/lib"), "claudine");
        write_package_toml(&root.join("claudine/cli"), "claudine-cli");

        // CWD is in the package area but not inside a specific package
        let cwd = root.join("claudine");
        let ctx = LaunchContext::from_cwd(&cwd).unwrap();

        assert!(
            ctx.package_area_root.is_some(),
            "package_area_root should be set"
        );
        let area_root = ctx.package_area_root.unwrap();
        assert!(
            area_root.ends_with("claudine"),
            "package_area_root should end with claudine, got: {}",
            area_root.display()
        );

        assert!(ctx.repo_root.is_some(), "repo_root should be set");
    }

    #[test]
    fn from_cwd_at_repo_root() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        init_git_repo(root);
        // Single-package repo (not a monorepo)
        let content = "[package]\nname = \"my-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";
        fs::write(root.join("Cargo.toml"), content).unwrap();

        let ctx = LaunchContext::from_cwd(root).unwrap();

        assert!(ctx.repo_root.is_some(), "repo_root should be set");
        assert!(
            ctx.package_root.is_none(),
            "package_root should be None for single-package repo"
        );
        assert!(
            ctx.package_area_root.is_none(),
            "package_area_root should be None for single-package repo"
        );
    }

    #[test]
    fn from_cwd_outside_repo() {
        let tmp = TempDir::new().unwrap();
        // No .git, no Cargo.toml
        let ctx = LaunchContext::from_cwd(tmp.path()).unwrap();

        assert!(ctx.repo_root.is_none(), "repo_root should be None");
        assert!(ctx.package_root.is_none(), "package_root should be None");
        assert!(
            ctx.package_area_root.is_none(),
            "package_area_root should be None"
        );
    }

    #[test]
    fn search_dirs_dedupes() {
        let path = PathBuf::from("/repo");
        let ctx = LaunchContext {
            agent: None,
            cwd: path.clone(),
            repo_root: Some(path.clone()),
            package_area_root: Some(path.clone()),
            package_root: Some(path.clone()),
        };

        let dirs = ctx.search_dirs();
        assert_eq!(dirs.len(), 1, "all same path should dedupe to 1");
        assert_eq!(dirs[0], path);
    }

    #[test]
    fn search_dirs_ordering() {
        let pkg = PathBuf::from("/repo/claudine/cli");
        let area = PathBuf::from("/repo/claudine");
        let repo = PathBuf::from("/repo");

        let ctx = LaunchContext {
            agent: None,
            cwd: pkg.clone(),
            repo_root: Some(repo.clone()),
            package_area_root: Some(area.clone()),
            package_root: Some(pkg.clone()),
        };

        let dirs = ctx.search_dirs();
        assert_eq!(dirs.len(), 3);
        assert_eq!(dirs[0], pkg, "package root should be first");
        assert_eq!(dirs[1], area, "package area root should be second");
        assert_eq!(dirs[2], repo, "repo root should be third");
    }

    #[test]
    fn search_dirs_partial_context() {
        let repo = PathBuf::from("/repo");
        let ctx = LaunchContext {
            agent: None,
            cwd: repo.clone(),
            repo_root: Some(repo.clone()),
            package_area_root: None,
            package_root: None,
        };

        let dirs = ctx.search_dirs();
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], repo);
    }

    #[test]
    fn search_dirs_empty_context() {
        let ctx = LaunchContext {
            agent: None,
            cwd: PathBuf::from("/tmp"),
            repo_root: None,
            package_area_root: None,
            package_root: None,
        };

        let dirs = ctx.search_dirs();
        assert!(dirs.is_empty());
    }
}
