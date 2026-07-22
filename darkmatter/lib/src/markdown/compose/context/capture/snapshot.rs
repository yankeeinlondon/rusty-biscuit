use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

use sniff::filesystem::docs::{self as sniff_docs, MarkdownMeta};
use sniff::filesystem::git::{FileStatus, GitRepo};
use sniff::filesystem::repo::{self as sniff_repo, Package, RepoInfo};
use sniff::hardware::{self, HardwareInfo};
use sniff::os::{self, OsInfo};
use sniff::request::OsRequest;

use super::{ContextGroup, ContextMergeDiagnostic};

#[cfg(test)]
static GIT_DISCOVERY_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Intermediate struct holding all raw sniff results.
///
/// Built once per compose run to avoid repeated sniff calls. All derived
/// context variables are computed from this single struct.
///
/// Uses `GitRepo` for atomic queries instead of the monolithic `detect_git`,
/// so only the fields actually needed are computed.
pub(super) struct ContextCapture {
    /// Directory the capture was built for (the compose working directory).
    pub(super) base_dir: PathBuf,
    /// Repository root path (cheap: from `GitRepo::discover`).
    pub(super) repo_root: Option<PathBuf>,
    /// Repo name parsed from preferred remote URL (cheap: reads git config).
    pub(super) repo_name: Option<String>,
    /// Whether a git repo was found at all.
    pub(super) has_git: bool,
    pub(super) git_branch: Option<String>,
    pub(super) git_worktree: Option<String>,
    pub(super) merge_conflicts: Vec<PathBuf>,
    pub(super) repo_info: Option<RepoInfo>,
    pub(super) docs: Option<Vec<MarkdownMeta>>,
    pub(super) os_info: Option<OsInfo>,
    pub(super) hardware_info: Option<HardwareInfo>,
    pub(super) gpu_names: Option<String>,
    pub(super) current_package: Option<Package>,
    pub(super) current_package_area: Option<String>,
    pub(super) dirty_paths: Vec<PathBuf>,
    pub(super) staged_paths: Vec<PathBuf>,
    pub(super) untracked_paths: Vec<PathBuf>,
    pub(super) diagnostics: Vec<ContextMergeDiagnostic>,
    pub(super) timings: Vec<(String, Duration)>,
}

impl ContextCapture {
    /// Build the capture from a base directory for the requested groups.
    ///
    /// Uses `GitRepo` for atomic queries. `GitRepo::discover` is the only
    /// sequential step (cheap — just finds `.git`). All other probes run
    /// in parallel via `std::thread::scope`.
    pub(super) fn new(base_dir: &Path, groups: &[ContextGroup]) -> Self {
        let mut diagnostics = Vec::new();
        let mut timings = Vec::new();

        let need_git_group = groups.contains(&ContextGroup::Git);
        let need_repo = groups.iter().any(|g| {
            matches!(
                g,
                ContextGroup::Repo
                    | ContextGroup::FileChanges
                    | ContextGroup::Languages
                    | ContextGroup::Documents
            )
        });
        let need_git = need_git_group || need_repo;
        let need_file_changes = groups.contains(&ContextGroup::FileChanges);
        let need_docs = groups.contains(&ContextGroup::Documents);
        let need_os = groups.contains(&ContextGroup::Os);
        let need_hw = groups.contains(&ContextGroup::Hardware);
        let need_gpu = groups.contains(&ContextGroup::Gpu);

        // ── Git discovery (near-instant — just finds .git) ───────────
        let t = Instant::now();
        let git_handle = if need_git {
            #[cfg(test)]
            GIT_DISCOVERY_COUNT.fetch_add(1, Ordering::Relaxed);
            match GitRepo::discover(base_dir) {
                Ok(handle) => handle,
                Err(e) => {
                    diagnostics.push(ContextMergeDiagnostic::PartialRuntimeCapture {
                        area: "git",
                        detail: e.to_string(),
                    });
                    None
                }
            }
        } else {
            None
        };
        let has_git = git_handle.is_some();
        // A bare repository's `repo_root` is its git directory, not a checkout.
        // Repo-structure and document scans must not walk it, so they degrade to
        // no root while the Git group's HEAD-derived fields stay valid.
        let repo_root = git_handle
            .as_ref()
            .filter(|h| !h.is_bare())
            .map(|h| h.repo_root().to_path_buf());
        let (_org, repo_name) = if need_repo {
            git_handle
                .as_ref()
                .map(|h| h.org_and_repo())
                .unwrap_or((None, None))
        } else {
            (None, None)
        };
        if need_git {
            timings.push(("git".into(), t.elapsed()));
        }

        let (mut git_branch, mut git_worktree, mut merge_conflicts) =
            (None, None, Vec::new());
        if need_git_group
            && let Some(repo) = git_handle.as_ref()
        {
            match repo.try_current_branch() {
                Ok(value) => git_branch = value,
                Err(error) => diagnostics.push(ContextMergeDiagnostic::PartialRuntimeCapture {
                    area: "git",
                    detail: format!("branch: {error}"),
                }),
            }
            match repo.try_current_worktree_name() {
                Ok(value) => git_worktree = value,
                Err(error) => diagnostics.push(ContextMergeDiagnostic::PartialRuntimeCapture {
                    area: "git",
                    detail: format!("worktree: {error}"),
                }),
            }
            match repo.merge_conflicts() {
                Ok(value) => merge_conflicts = value,
                Err(error) => diagnostics.push(ContextMergeDiagnostic::PartialRuntimeCapture {
                    area: "git",
                    detail: format!("merge_conflicts: {error}"),
                }),
            }
        }

        // ── All remaining probes run in parallel ─────────────────────
        let (file_changes, repo_info, docs, os_info, hardware_info, gpu_names) =
            std::thread::scope(|s| {
                let fc_handle = if need_file_changes && has_git {
                    let bd = base_dir.to_path_buf();
                    Some(s.spawn(move || {
                        let t = Instant::now();
                        let result = GitRepo::discover(&bd)
                            .ok()
                            .flatten()
                            .and_then(|h| h.file_changes().ok());
                        (result.unwrap_or_default(), t.elapsed())
                    }))
                } else {
                    None
                };

                let repo_handle = if need_repo {
                    let rr = &repo_root;
                    Some(s.spawn(move || {
                        let t = Instant::now();
                        let result = rr
                            .as_ref()
                            .and_then(|root| sniff_repo::detect_repo_structure(root).ok().flatten());
                        (result, t.elapsed())
                    }))
                } else {
                    None
                };

                let os_handle = if need_os {
                    Some(s.spawn(|| {
                        let t = Instant::now();
                        let request = OsRequest::full()
                            .include_locale(false)
                            .include_timezone(false)
                            .include_ntp_status(false);
                        let result = os::detect_os_with_request(&request);
                        (result, t.elapsed())
                    }))
                } else {
                    None
                };

                let hw_handle = if need_hw {
                    Some(s.spawn(|| {
                        let t = Instant::now();
                        let result = hardware::detect_hardware_summary();
                        (result, t.elapsed())
                    }))
                } else {
                    None
                };

                let gpu_handle = if need_gpu {
                    Some(s.spawn(|| {
                        let t = Instant::now();
                        let gpus = hardware::detect_gpus();
                        let names = if gpus.is_empty() {
                            None
                        } else {
                            Some(
                                gpus.iter()
                                    .map(|g| g.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                            )
                        };
                        (names, t.elapsed())
                    }))
                } else {
                    None
                };

                // Collect repo first — docs depends on it
                let (repo_info, repo_elapsed) = repo_handle
                    .map(|h| h.join().unwrap_or((None, Duration::ZERO)))
                    .unwrap_or((None, Duration::ZERO));

                // Docs runs after repo because it needs the package list.
                // It still runs concurrently with file_changes/os/hw which
                // haven't been joined yet.
                let docs = if need_docs {
                    let t = Instant::now();
                    let package_list: Vec<(String, PathBuf)> = repo_info
                        .as_ref()
                        .and_then(|ri| ri.packages.as_ref())
                        .map(|pkgs| {
                            pkgs.iter()
                                .map(|p| {
                                    let rel_path = p
                                        .path
                                        .strip_prefix(&repo_info.as_ref().unwrap().root)
                                        .unwrap_or(&p.path)
                                        .to_path_buf();
                                    (p.name.clone(), rel_path)
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    let result = repo_root
                        .as_ref()
                        .and_then(|root| sniff_docs::detect_docs_with_packages(root, &package_list));
                    timings.push(("docs".into(), t.elapsed()));
                    result
                } else {
                    None
                };

                if need_repo {
                    timings.push(("repo".into(), repo_elapsed));
                }

                let (file_changes, fc_elapsed) = fc_handle
                    .map(|h| h.join().unwrap_or((Vec::new(), Duration::ZERO)))
                    .unwrap_or((Vec::new(), Duration::ZERO));
                if need_file_changes {
                    timings.push(("file_changes".into(), fc_elapsed));
                }

                let os_info = os_handle.map(|h| match h.join() {
                    Ok((result, elapsed)) => {
                        timings.push(("os".into(), elapsed));
                        result.map_err(|e| e.to_string())
                    }
                    Err(_) => Err("OS detection panicked".to_string()),
                });

                let hardware_info = hw_handle.map(|h| match h.join() {
                    Ok((result, elapsed)) => {
                        timings.push(("hardware".into(), elapsed));
                        result.map_err(|e| e.to_string())
                    }
                    Err(_) => Err("hardware detection panicked".to_string()),
                });

                let gpu_names = gpu_handle.and_then(|h| {
                    let (names, elapsed) = h.join().unwrap_or((None, Duration::ZERO));
                    timings.push(("gpu".into(), elapsed));
                    names
                });

                (
                    file_changes,
                    repo_info,
                    docs,
                    os_info,
                    hardware_info,
                    gpu_names,
                )
            });

        let os_info = match os_info {
            Some(Ok(info)) => Some(info),
            Some(Err(detail)) => {
                diagnostics
                    .push(ContextMergeDiagnostic::PartialRuntimeCapture { area: "os", detail });
                None
            }
            None => None,
        };

        let hardware_info = match hardware_info {
            Some(Ok(info)) => Some(info),
            Some(Err(detail)) => {
                diagnostics.push(ContextMergeDiagnostic::PartialRuntimeCapture {
                    area: "hardware",
                    detail,
                });
                None
            }
            None => None,
        };

        // ── Derived fields from git/repo ──────────────────────────────
        let (current_package, current_package_area) = if let Some(ref ri) = repo_info {
            if ri.is_monorepo {
                let pkg = ri.packages.as_ref().and_then(|packages| {
                    packages
                        .iter()
                        .find(|p| base_dir.starts_with(&p.path))
                        .cloned()
                });
                let area = pkg.as_ref().map(|p| p.package_area.clone()).or_else(|| {
                    ri.packages.as_ref().and_then(|packages| {
                        let areas: Vec<_> = packages
                            .iter()
                            .filter(|p| {
                                let area_path = ri.root.join(&p.package_area);
                                base_dir.starts_with(&area_path)
                            })
                            .collect();
                        areas.first().map(|p| p.package_area.clone())
                    })
                });
                (pkg, area)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        let dirty_paths = file_changes
            .iter()
            .filter(|fc| {
                matches!(
                    fc.status,
                    FileStatus::Modified
                        | FileStatus::Both
                        | FileStatus::Staged
                        | FileStatus::Untracked
                )
            })
            .map(|fc| fc.path.clone())
            .collect();

        let staged_paths = file_changes
            .iter()
            .filter(|fc| matches!(fc.status, FileStatus::Staged | FileStatus::Both))
            .map(|fc| fc.path.clone())
            .collect();

        let untracked_paths = file_changes
            .iter()
            .filter(|fc| matches!(fc.status, FileStatus::Untracked))
            .map(|fc| fc.path.clone())
            .collect();

        Self {
            base_dir: base_dir.to_path_buf(),
            repo_root,
            repo_name,
            has_git,
            git_branch,
            git_worktree,
            merge_conflicts,
            repo_info,
            docs,
            os_info,
            hardware_info,
            gpu_names,
            current_package,
            current_package_area,
            dirty_paths,
            staged_paths,
            untracked_paths,
            diagnostics,
            timings,
        }
    }
}

#[cfg(test)]
impl ContextCapture {
    /// Minimal capture with a fixed repo root and the supplied repo info.
    pub(super) fn for_test_base(repo_root: PathBuf, repo_info: Option<RepoInfo>) -> Self {
        Self {
            base_dir: repo_root.clone(),
            repo_root: Some(repo_root),
            repo_name: None,
            has_git: true,
            git_branch: None,
            git_worktree: None,
            merge_conflicts: Vec::new(),
            repo_info,
            docs: None,
            os_info: None,
            hardware_info: None,
            gpu_names: None,
            current_package: None,
            current_package_area: None,
            dirty_paths: Vec::new(),
            staged_paths: Vec::new(),
            untracked_paths: Vec::new(),
            diagnostics: Vec::new(),
            timings: Vec::new(),
        }
    }

    pub(super) fn for_test_non_monorepo() -> Self {
        let root = PathBuf::from("/tmp/not-a-monorepo");
        let ri = test_repo_info(root.clone(), false, None);
        Self::for_test_base(root, Some(ri))
    }

    pub(super) fn for_test_monorepo_with_packages(pkgs: &[(&str, &str)]) -> Self {
        let root = PathBuf::from("/tmp/mono");
        let packages = pkgs
            .iter()
            .map(|(name, relative)| Package {
                name: (*name).to_string(),
                relative: (*relative).to_string(),
                path: root.join(relative),
                ..Default::default()
            })
            .collect();
        let ri = test_repo_info(root.clone(), true, Some(packages));
        Self::for_test_base(root, Some(ri))
    }

    pub(super) fn for_test_monorepo_in_package(name: &str, depends_on: &[&str], used_by: &[&str]) -> Self {
        let root = PathBuf::from("/tmp/mono");
        let pkg = Package {
            name: name.to_string(),
            relative: format!("{name}/lib"),
            package_area: name.to_string(),
            path: root.join(name).join("lib"),
            depends_on: depends_on.iter().map(|s| (*s).to_string()).collect(),
            used_by: used_by.iter().map(|s| (*s).to_string()).collect(),
            ..Default::default()
        };
        let ri = test_repo_info(root.clone(), true, Some(vec![pkg.clone()]));
        let mut cap = Self::for_test_base(root, Some(ri));
        cap.current_package = Some(pkg);
        cap
    }
}

#[cfg(test)]
pub(super) fn test_repo_info(root: PathBuf, is_monorepo: bool, packages: Option<Vec<Package>>) -> RepoInfo {
    RepoInfo {
        is_monorepo,
        root,
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        packages,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_group_performs_one_trusted_discovery_for_all_fields() {
        let outside = tempfile::tempdir().expect("temporary non-repository directory");
        GIT_DISCOVERY_COUNT.store(0, Ordering::Relaxed);

        let capture = ContextCapture::new(outside.path(), &[ContextGroup::Git]);

        assert_eq!(GIT_DISCOVERY_COUNT.load(Ordering::Relaxed), 1);
        assert_eq!(capture.git_branch, None);
        assert_eq!(capture.git_worktree, None);
        assert!(capture.merge_conflicts.is_empty());
        assert!(capture.diagnostics.is_empty());
    }
}
