use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Host and repository environment snapshot.
///
/// Canonical wrapper and composition paths project this from invocation-owned
/// observations; compatibility and hook paths may still detect it directly.
/// Attached to every `EventMeta`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
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

    /// Monorepo package area inferred by the wrapper, if available.
    #[serde(default)]
    pub package_area: Option<String>,

    /// Monorepo package inferred by the wrapper, if available.
    #[serde(default)]
    pub package: Option<String>,

    /// Claudine's own process ID, captured once at wrapper startup.
    ///
    /// Populated for every wrapper-emitted record by reading `CLAUDINE_PID`
    /// from the environment (set in the child env by the wrapper, or set in
    /// the wrapper's own env at startup). Older JSONL lines that pre-date
    /// this field deserialize as `None` via `#[serde(default)]`.
    #[serde(default)]
    pub claudine_pid: Option<u32>,
}

/// Operating system identification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    /// Repository name from the primary remote (e.g., "cargo").
    #[serde(default)]
    pub repo_name: Option<String>,

    /// Organization or owner name from the primary remote (e.g., "rust-lang").
    #[serde(default)]
    pub repo_org: Option<String>,
}

/// Project and monorepo structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepoContext {
    /// Whether this project is a monorepo.
    #[serde(default)]
    pub is_monorepo: bool,

    /// Monorepo authority standard if detected.
    /// Values: "cargo-workspace", "npm-workspaces", "pnpm-workspaces",
    /// "yarn-workspaces", "nx", "turborepo", "lerna", etc.
    #[serde(default)]
    pub monorepo_standard: Option<String>,

    /// Orchestrators riding on the primary monorepo layer.
    /// Values: "nx", "turborepo", "lerna".
    #[serde(default)]
    pub monorepo_orchestrators: Vec<String>,

    /// Deprecated alias for `monorepo_standard`.
    ///
    /// Kept for one release so existing templates using
    /// `{{project.monorepo_tool}}` continue to resolve. New templates should
    /// use `{{project.monorepo_standard}}`.
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
                let status = g.status.as_ref();

                GitContext {
                    repo_root: g.repo_root.clone(),
                    branch: g.current_branch.clone(),
                    is_dirty: status.map(|s| s.is_dirty).unwrap_or(false),
                    staged_count: status.map(|s| s.staged_count).unwrap_or(0),
                    unstaged_count: status.map(|s| s.unstaged_count).unwrap_or(0),
                    untracked_count: status.map(|s| s.untracked_count).unwrap_or(0),
                    head_sha: head_commit.map(|c| c.sha.clone()),
                    head_message: head_commit.map(|c| c.message.clone()),
                    user_name: g.config.user_name.clone(),
                    user_email: g.config.user_email.clone(),
                    remote_name: primary_remote.map(|r| r.name.clone()),
                    remote_url: primary_remote.and_then(|r| r.url.clone()),
                    hosting_provider: primary_remote
                        .map(|r| format!("{:?}", r.provider).to_lowercase()),
                    repo_name: g.repo.clone(),
                    repo_org: g.org.clone(),
                }
            })
        });

        let repo = fs.as_ref().and_then(|f| {
            f.repo.as_ref().map(|r| {
                let (monorepo_standard, monorepo_orchestrators) = r
                    .primary_layer()
                    .map(|layer| {
                        let standard = layer.authority.spec().id.to_string();
                        let orchestrators = layer
                            .orchestrators
                            .iter()
                            .map(|o| o.spec().id.to_string())
                            .collect();
                        (Some(standard), orchestrators)
                    })
                    .unwrap_or_default();

                let packages = r
                    .packages
                    .as_ref()
                    .map(|pkgs| pkgs.iter().map(|p| p.name.clone()).collect())
                    .unwrap_or_default();

                RepoContext {
                    is_monorepo: r.is_monorepo,
                    monorepo_standard: monorepo_standard.clone(),
                    monorepo_orchestrators,
                    // `monorepo_tool` is a deprecated alias kept for one
                    // release; derive it from the new topology field.
                    monorepo_tool: monorepo_standard,
                    root: r.root.clone(),
                    packages,
                }
            })
        });

        let primary_language = fs
            .as_ref()
            .and_then(|f| f.languages.as_ref())
            .and_then(|l| l.primary.as_ref())
            .map(|l| l.to_string());

        EnvironmentContext {
            os,
            hardware,
            git,
            repo,
            primary_language,
            package_area: None,
            package: None,
            claudine_pid: None,
        }
    }
}

/// Build an `EnvironmentContext` from a pre-computed `SniffResult`.
///
/// Use this when a caller has already invoked `sniff::detect_with_plan`
/// (e.g. to build a `LaunchContext` in the same startup pass) and wants
/// to avoid triggering a second full filesystem walk.
///
/// The returned context still has `PACKAGE_AREA` / `PACKAGE` populated
/// from the current process environment, matching `detect_environment_fast`.
pub fn environment_context_from_sniff_result(result: sniff::SniffResult) -> EnvironmentContext {
    let mut context = EnvironmentContext::from(result);
    apply_wrapper_package_context(&mut context, &lookup_env_var);
    context
}

/// Build an environment context from observed host facts and a captured
/// process-environment snapshot.
///
/// Unlike [`environment_context_from_sniff_result`], this projection performs
/// no late environment reads. Invocation-scoped callers use it after child CWD
/// or HOME changes so event metadata remains tied to launch inputs.
pub fn environment_context_from_sniff_result_and_env(
    result: sniff::SniffResult,
    environment: &HashMap<String, String>,
) -> EnvironmentContext {
    let mut context = EnvironmentContext::from(result);
    apply_wrapper_package_context(&mut context, &|name| environment.get(name).cloned());
    context
}

/// Detect the environment context for the given working directory.
///
/// Uses `sniff` with a plan-based configuration: no network calls,
/// single commit, no deep inspection, hardware summary only (CPU +
/// memory — skips storage, GPU, and audio enumeration).
pub fn detect_environment(cwd: &Path) -> EnvironmentContext {
    use sniff::request::*;

    let plan = DetectionPlan::new()
        .base_dir(cwd.to_path_buf())
        .os(OsRequest::full())
        .hardware(HardwareRequest::summary())
        .without_network()
        .filesystem(FilesystemRequest::new().git(GitRequest::full().commit_count(1)));

    let result = sniff::detect_with_plan(plan).unwrap_or_default();

    let mut context = EnvironmentContext::from(result);
    apply_wrapper_package_context(&mut context, &lookup_env_var);
    context
}

/// Lightweight environment detection for hook handlers.
///
/// Only detects git summary and repo structure (needed for log path
/// resolution and event metadata). Skips OS, hardware, network, file
/// inventory, docs, and formatting to minimize latency — critical for
/// hooks where the provider may cancel slow processes during shutdown.
pub fn detect_environment_fast(cwd: &Path) -> EnvironmentContext {
    use sniff::request::*;

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

    let result = sniff::detect_with_plan(plan).unwrap_or_default();

    let mut context = EnvironmentContext::from(result);
    apply_wrapper_package_context(&mut context, &lookup_env_var);
    context
}

fn apply_wrapper_package_context(
    context: &mut EnvironmentContext,
    lookup: &dyn Fn(&str) -> Option<String>,
) {
    context.package_area = lookup("PACKAGE_AREA")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    context.package = lookup("PACKAGE")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    // `CLAUDINE_PID` is stamped onto the child environment by
    // `wrap::env::build_child_env_with_launch` before the provider is
    // spawned. When this code runs inside the wrapper process itself,
    // the env var has not yet been set in our own process env — fall
    // back to `std::process::id()` so wrapper-side detection produces
    // the same value the child will see. Inside a hook handler (a
    // descendant of the spawned provider) the env var is already
    // present and we use it directly.
    context.claudine_pid = lookup("CLAUDINE_PID")
        .and_then(|value| value.trim().parse::<u32>().ok())
        .or_else(|| Some(std::process::id()));
}

fn lookup_env_var(name: &str) -> Option<String> {
    std::env::var(name).ok()
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
        assert!(ctx.package_area.is_none());
        assert!(ctx.package.is_none());
    }

    #[test]
    fn default_round_trip_json() {
        let ctx = EnvironmentContext::default();
        let json = serde_json::to_string(&ctx).unwrap();
        let back: EnvironmentContext = serde_json::from_str(&json).unwrap();
        assert!(back.os.os_type.is_empty());
        assert!(back.git.is_none());
        assert!(back.repo.is_none());
        assert!(back.package_area.is_none());
        assert!(back.package.is_none());
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
                "user_name": "Test User",
                "repo_name": "my-project",
                "repo_org": "my-org"
            },
            "repo": {
                "is_monorepo": true,
                "monorepo_standard": "cargo-workspace",
                "monorepo_orchestrators": ["nx"],
                "monorepo_tool": "cargoworkspace",
                "root": "/tmp/repo",
                "packages": ["lib", "cli"]
            },
            "primary_language": "Rust",
            "package_area": "claudine",
            "package": "claudine-cli"
        });
        let ctx: EnvironmentContext = serde_json::from_value(json).unwrap();
        assert_eq!(ctx.os.os_type, "macos");
        assert_eq!(ctx.hardware.cores, 16);
        assert_eq!(ctx.git.as_ref().unwrap().branch.as_deref(), Some("main"));
        assert!(ctx.git.as_ref().unwrap().is_dirty);
        assert_eq!(ctx.git.as_ref().unwrap().staged_count, 2);
        assert_eq!(
            ctx.git.as_ref().unwrap().repo_name.as_deref(),
            Some("my-project")
        );
        assert_eq!(
            ctx.git.as_ref().unwrap().repo_org.as_deref(),
            Some("my-org")
        );
        assert!(ctx.repo.as_ref().unwrap().is_monorepo);
        assert_eq!(
            ctx.repo.as_ref().unwrap().monorepo_standard.as_deref(),
            Some("cargo-workspace")
        );
        assert_eq!(
            ctx.repo.as_ref().unwrap().monorepo_orchestrators,
            vec!["nx"]
        );
        assert_eq!(
            ctx.repo.as_ref().unwrap().monorepo_tool.as_deref(),
            Some("cargoworkspace")
        );
        assert_eq!(ctx.repo.as_ref().unwrap().packages.len(), 2);
        assert_eq!(ctx.primary_language.as_deref(), Some("Rust"));
        assert_eq!(ctx.package_area.as_deref(), Some("claudine"));
        assert_eq!(ctx.package.as_deref(), Some("claudine-cli"));
    }

    #[test]
    fn from_sniff_result_populates_monorepo_topology() {
        use sniff::filesystem::FilesystemInfo;
        use sniff::filesystem::repo::Package;
        use sniff::filesystem::repo::standard::{
            MonorepoLayer, MonorepoStandard, PackageProvenance,
        };

        let repo = sniff::filesystem::repo::RepoInfo {
            is_monorepo: true,
            root: PathBuf::from("/repo"),
            packages: Some(vec![Package {
                path: PathBuf::from("/repo/lib"),
                relative: "lib".to_string(),
                package_area: "root".to_string(),
                name: "lib".to_string(),
                ..Package::default()
            }]),
            monorepo_standards: vec![],
            monorepo_layers: vec![MonorepoLayer {
                root: PathBuf::from("/repo"),
                authority: MonorepoStandard::CargoWorkspace,
                orchestrators: vec![MonorepoStandard::Nx],
                provenance: PackageProvenance::Globbed,
                lockfile_match: None,
                root_is_package: true,
                packages: vec![],
            }],
            ..sniff::filesystem::repo::RepoInfo::default()
        };

        let result = sniff::SniffResult {
            filesystem: Some(FilesystemInfo {
                repo: Some(repo),
                ..FilesystemInfo::default()
            }),
            ..sniff::SniffResult::default()
        };

        let ctx = EnvironmentContext::from(result);
        let repo_ctx = ctx.repo.unwrap();
        assert_eq!(
            repo_ctx.monorepo_standard.as_deref(),
            Some("cargo-workspace")
        );
        assert_eq!(repo_ctx.monorepo_orchestrators, vec!["nx"]);
        assert_eq!(repo_ctx.monorepo_tool.as_deref(), Some("cargo-workspace"));
    }

    #[test]
    fn detect_environment_fast_on_rusty_biscuit_preserves_template_values() {
        // Regression: template consumers rely on these values staying stable
        // on the rusty-biscuit repository after topology changes.
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir.parent().unwrap().parent().unwrap();
        let ctx = detect_environment_fast(repo_root);

        let repo_ctx = ctx.repo.expect("rusty-biscuit should produce repo context");
        assert!(repo_ctx.is_monorepo);
        assert_eq!(
            repo_ctx.monorepo_standard.as_deref(),
            Some("cargo-workspace"),
            "monorepo_standard must be cargo-workspace on rusty-biscuit"
        );
        assert_eq!(
            repo_ctx.monorepo_orchestrators,
            Vec::<String>::new(),
            "rusty-biscuit has no orchestrators"
        );
        assert_eq!(
            repo_ctx.monorepo_tool.as_deref(),
            Some("cargo-workspace"),
            "deprecated monorepo_tool alias must equal monorepo_standard"
        );
        assert!(!repo_ctx.packages.is_empty(), "rusty-biscuit has packages");
    }

    #[test]
    fn from_empty_sniff_result() {
        let result = sniff::SniffResult::default();
        let ctx = EnvironmentContext::from(result);
        assert!(ctx.os.os_type.is_empty());
        assert!(ctx.hardware.arch.is_empty());
        assert!(ctx.git.is_none());
        assert!(ctx.repo.is_none());
        assert!(ctx.package_area.is_none());
        assert!(ctx.package.is_none());
    }

    #[test]
    fn wrapper_package_context_uses_non_empty_env_values() {
        let mut ctx = EnvironmentContext::default();
        apply_wrapper_package_context(&mut ctx, &|name| match name {
            "PACKAGE_AREA" => Some(" claudine ".to_string()),
            "PACKAGE" => Some("cl-cli".to_string()),
            _ => None,
        });

        assert_eq!(ctx.package_area.as_deref(), Some("claudine"));
        assert_eq!(ctx.package.as_deref(), Some("cl-cli"));
    }

    #[test]
    fn wrapper_package_context_ignores_empty_values() {
        let mut ctx = EnvironmentContext::default();
        apply_wrapper_package_context(&mut ctx, &|name| match name {
            "PACKAGE_AREA" => Some("   ".to_string()),
            "PACKAGE" => Some(String::new()),
            _ => None,
        });

        assert!(ctx.package_area.is_none());
        assert!(ctx.package.is_none());
    }

    /// Phase 3 — `CLAUDINE_PID` env var is honored when present.
    #[test]
    fn wrapper_package_context_reads_claudine_pid_from_env() {
        let mut ctx = EnvironmentContext::default();
        apply_wrapper_package_context(&mut ctx, &|name| match name {
            "CLAUDINE_PID" => Some("12345".to_string()),
            _ => None,
        });

        assert_eq!(ctx.claudine_pid, Some(12345));
    }

    /// Phase 3 — when `CLAUDINE_PID` is unset, the helper falls back to
    /// `std::process::id()` so wrapper-side detection still produces a PID.
    #[test]
    fn wrapper_package_context_falls_back_to_process_id() {
        let mut ctx = EnvironmentContext::default();
        apply_wrapper_package_context(&mut ctx, &|_| None);

        assert_eq!(ctx.claudine_pid, Some(std::process::id()));
    }

    /// Phase 3 — `claudine_pid` MUST round-trip through serde.
    #[test]
    fn claudine_pid_round_trips_json() {
        let ctx = EnvironmentContext {
            claudine_pid: Some(99_876),
            ..EnvironmentContext::default()
        };
        let json = serde_json::to_value(&ctx).unwrap();
        assert_eq!(json["claudine_pid"], 99_876);

        let back: EnvironmentContext = serde_json::from_value(json).unwrap();
        assert_eq!(back.claudine_pid, Some(99_876));
    }

    /// Phase 3 — legacy JSONL without `claudine_pid` MUST deserialize as `None`.
    #[test]
    fn claudine_pid_defaults_to_none_on_deserialize() {
        let json = serde_json::json!({
            "os": { "os_type": "macos" }
        });
        let ctx: EnvironmentContext = serde_json::from_value(json).unwrap();
        assert!(ctx.claudine_pid.is_none());
    }
}
