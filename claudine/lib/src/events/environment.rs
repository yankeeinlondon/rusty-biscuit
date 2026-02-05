use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Host and repository environment snapshot.
///
/// Detected once at session start via `sniff::detect_with_config`
/// and cached for the session lifetime. Attached to every `EventMeta`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvironmentContext {
    /// Operating system information.
    #[serde(default)]
    pub os: OsContext,

    /// Hardware summary.
    #[serde(default)]
    pub hardware: HardwareContext,

    /// Git repository state (if cwd is inside a repo).
    #[serde(default)]
    pub git: Option<GitContext>,

    /// Project/repository structure.
    #[serde(default)]
    pub repo: Option<RepoContext>,

    /// Primary programming language detected in the project.
    #[serde(default)]
    pub primary_language: Option<String>,
}

/// Operating system identification.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OsContext {
    /// OS family: "macos", "linux", "windows", etc.
    #[serde(default)]
    pub os_type: String,

    /// Display name (e.g., "macOS", "Ubuntu", "Windows 11").
    #[serde(default)]
    pub name: String,

    /// Short version string (e.g., "15.3", "24.04").
    #[serde(default)]
    pub version: String,

    /// Kernel version string.
    #[serde(default)]
    pub kernel: String,

    /// Machine hostname.
    #[serde(default)]
    pub hostname: String,

    /// Linux distribution family, if applicable.
    /// Values: "debian", "redhat", "arch", "alpine", "nixos", "gentoo", or `None`.
    #[serde(default)]
    pub linux_family: Option<String>,

    /// Detected system package managers (e.g., \["brew", "port"\] on macOS).
    #[serde(default)]
    pub package_managers: Vec<String>,
}

/// Hardware summary relevant to event handling.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HardwareContext {
    /// CPU architecture: "x86_64", "aarch64", etc.
    #[serde(default)]
    pub arch: String,

    /// CPU brand string (e.g., "Apple M4 Max").
    #[serde(default)]
    pub cpu: String,

    /// Logical core count.
    #[serde(default)]
    pub cores: usize,

    /// Total system memory in bytes.
    #[serde(default)]
    pub memory_bytes: u64,

    /// Available system memory in bytes at detection time.
    #[serde(default)]
    pub memory_available_bytes: u64,
}

/// Git repository state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitContext {
    /// Absolute path to the repository root.
    pub repo_root: PathBuf,

    /// Current branch name, or `None` for detached HEAD.
    #[serde(default)]
    pub branch: Option<String>,

    /// Whether the working tree has uncommitted changes.
    #[serde(default)]
    pub is_dirty: bool,

    /// Count of staged files.
    #[serde(default)]
    pub staged_count: usize,

    /// Count of modified but unstaged files.
    #[serde(default)]
    pub unstaged_count: usize,

    /// Count of untracked files.
    #[serde(default)]
    pub untracked_count: usize,

    /// SHA of the most recent commit.
    #[serde(default)]
    pub head_sha: Option<String>,

    /// Message of the most recent commit.
    #[serde(default)]
    pub head_message: Option<String>,

    /// Git user.name from config.
    #[serde(default)]
    pub user_name: Option<String>,

    /// Git user.email from config.
    #[serde(default)]
    pub user_email: Option<String>,

    /// Primary remote name (usually "origin").
    #[serde(default)]
    pub remote_name: Option<String>,

    /// Primary remote URL.
    #[serde(default)]
    pub remote_url: Option<String>,

    /// Hosting provider for the primary remote.
    /// Values: "github", "gitlab", "bitbucket", "azure_devops", etc.
    #[serde(default)]
    pub hosting_provider: Option<String>,
}

/// Project and monorepo structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoContext {
    /// Whether this project is a monorepo.
    #[serde(default)]
    pub is_monorepo: bool,

    /// Monorepo tool if detected.
    /// Values: "cargo_workspace", "npm_workspaces", "pnpm_workspaces",
    /// "yarn_workspaces", "nx", "turborepo", "lerna".
    #[serde(default)]
    pub monorepo_tool: Option<String>,

    /// Absolute path to the project root.
    pub root: PathBuf,

    /// Package names within the monorepo (empty for single-package repos).
    #[serde(default)]
    pub packages: Vec<String>,
}

impl From<sniff::SniffResult> for EnvironmentContext {
    fn from(result: sniff::SniffResult) -> Self {
        let os = if let Some(os_info) = result.os {
            let os_type = format!("{:?}", os_info.os_type).to_lowercase();

            let linux_family = os_info
                .linux_distro
                .as_ref()
                .map(|d| format!("{:?}", d.family).to_lowercase());

            let package_managers = os_info
                .system_package_managers
                .map(|spm| spm.managers.iter().map(|m| m.manager.to_string()).collect())
                .unwrap_or_default();

            OsContext {
                os_type,
                name: os_info.name,
                version: os_info.version,
                kernel: os_info.kernel,
                hostname: os_info.hostname,
                linux_family,
                package_managers,
            }
        } else {
            OsContext::default()
        };

        let hardware = if let Some(hw) = result.hardware {
            HardwareContext {
                arch: hw.cpu.arch.clone(),
                cpu: hw.cpu.brand.clone(),
                cores: hw.cpu.logical_cores,
                memory_bytes: hw.memory.total_bytes,
                memory_available_bytes: hw.memory.available_bytes,
            }
        } else {
            HardwareContext::default()
        };

        let fs = result.filesystem;

        let git = fs.as_ref().and_then(|f| {
            f.git.as_ref().map(|g| {
                let head_commit = g.recent.first();
                let primary_remote = g.remotes.first();

                GitContext {
                    repo_root: g.repo_root.clone(),
                    branch: g.current_branch.clone(),
                    is_dirty: g.status.is_dirty,
                    staged_count: g.status.staged_count,
                    unstaged_count: g.status.unstaged_count,
                    untracked_count: g.status.untracked_count,
                    head_sha: head_commit.map(|c| c.sha.clone()),
                    head_message: head_commit.map(|c| c.message.clone()),
                    user_name: g.config.user_name.clone(),
                    user_email: g.config.user_email.clone(),
                    remote_name: primary_remote.map(|r| r.name.clone()),
                    remote_url: primary_remote.and_then(|r| r.url.clone()),
                    hosting_provider: primary_remote
                        .map(|r| format!("{:?}", r.provider).to_lowercase()),
                }
            })
        });

        let repo = fs.as_ref().and_then(|f| {
            f.repo.as_ref().map(|r| {
                let monorepo_tool = r
                    .monorepo_tool
                    .as_ref()
                    .map(|t| format!("{:?}", t).to_lowercase());

                let packages = r
                    .packages
                    .as_ref()
                    .map(|pkgs| pkgs.iter().map(|p| p.name.clone()).collect())
                    .unwrap_or_default();

                RepoContext {
                    is_monorepo: r.is_monorepo,
                    monorepo_tool,
                    root: r.root.clone(),
                    packages,
                }
            })
        });

        let primary_language = fs
            .as_ref()
            .and_then(|f| f.languages.as_ref())
            .and_then(|l| l.primary.clone());

        EnvironmentContext {
            os,
            hardware,
            git,
            repo,
            primary_language,
        }
    }
}

/// Detect the environment context for the given working directory.
///
/// Uses `sniff` with a fast configuration (no network calls,
/// single commit, no deep inspection) to gather OS, hardware,
/// git, and repository information.
pub fn detect_environment(cwd: &Path) -> EnvironmentContext {
    let config = sniff::SniffConfig::new()
        .base_dir(cwd.to_path_buf())
        .deep(false)
        .commit_count(1)
        .skip_network();

    let result = sniff::detect_with_config(config).unwrap_or(sniff::SniffResult {
        os: None,
        hardware: None,
        network: None,
        filesystem: None,
    });

    EnvironmentContext::from(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_produces_valid_empty_state() {
        let ctx = EnvironmentContext::default();
        assert!(ctx.os.os_type.is_empty());
        assert!(ctx.os.name.is_empty());
        assert!(ctx.os.package_managers.is_empty());
        assert!(ctx.hardware.arch.is_empty());
        assert_eq!(ctx.hardware.cores, 0);
        assert_eq!(ctx.hardware.memory_bytes, 0);
        assert!(ctx.git.is_none());
        assert!(ctx.repo.is_none());
        assert!(ctx.primary_language.is_none());
    }

    #[test]
    fn default_round_trip_json() {
        let ctx = EnvironmentContext::default();
        let json = serde_json::to_string(&ctx).unwrap();
        let back: EnvironmentContext = serde_json::from_str(&json).unwrap();
        assert!(back.os.os_type.is_empty());
        assert!(back.git.is_none());
        assert!(back.repo.is_none());
    }

    #[test]
    fn deserialize_with_git_context() {
        let json = serde_json::json!({
            "os": {
                "os_type": "macos",
                "name": "macOS",
                "version": "15.3",
                "kernel": "Darwin 25.3.0",
                "hostname": "test-host",
                "package_managers": ["brew"]
            },
            "hardware": {
                "arch": "aarch64",
                "cpu": "Apple M4 Max",
                "cores": 16,
                "memory_bytes": 68719476736_u64,
                "memory_available_bytes": 34359738368_u64
            },
            "git": {
                "repo_root": "/tmp/repo",
                "branch": "main",
                "is_dirty": true,
                "staged_count": 2,
                "unstaged_count": 1,
                "untracked_count": 0,
                "head_sha": "abc123",
                "user_name": "Test User"
            },
            "repo": {
                "is_monorepo": true,
                "monorepo_tool": "cargoworkspace",
                "root": "/tmp/repo",
                "packages": ["lib", "cli"]
            },
            "primary_language": "Rust"
        });
        let ctx: EnvironmentContext = serde_json::from_value(json).unwrap();
        assert_eq!(ctx.os.os_type, "macos");
        assert_eq!(ctx.hardware.cores, 16);
        assert_eq!(ctx.git.as_ref().unwrap().branch.as_deref(), Some("main"));
        assert!(ctx.git.as_ref().unwrap().is_dirty);
        assert_eq!(ctx.git.as_ref().unwrap().staged_count, 2);
        assert!(ctx.repo.as_ref().unwrap().is_monorepo);
        assert_eq!(ctx.repo.as_ref().unwrap().packages.len(), 2);
        assert_eq!(ctx.primary_language.as_deref(), Some("Rust"));
    }

    #[test]
    fn from_empty_sniff_result() {
        let result = sniff::SniffResult {
            os: None,
            hardware: None,
            network: None,
            filesystem: None,
        };
        let ctx = EnvironmentContext::from(result);
        assert!(ctx.os.os_type.is_empty());
        assert!(ctx.hardware.arch.is_empty());
        assert!(ctx.git.is_none());
        assert!(ctx.repo.is_none());
    }
}
