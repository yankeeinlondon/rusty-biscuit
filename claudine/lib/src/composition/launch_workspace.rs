use std::path::PathBuf;

/// Workspace context for the directory where a provider child process
/// will be launched.
///
/// Carries the launch CWD, detected repo root, child process working
/// directory, and optional monorepo package metadata. Built once from
/// a shared `sniff::detect_with_plan` result and threaded through the
/// composition pipeline to avoid redundant filesystem walks.
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

/// Monorepo package metadata for a launch workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageContext {
    pub package_area: String,
    pub package: Option<String>,
    pub candidates: Vec<String>,
}
