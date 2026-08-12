use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

use sniff::filesystem::docs::{self as sniff_docs, MarkdownMeta};
use sniff::filesystem::git::{FileChange, FileStatus, GitInfo, GitRepo};
use sniff::filesystem::LanguageBreakdown;
use sniff::filesystem::repo::{self as sniff_repo, Package, RepoInfo};
use sniff::hardware::{self, GpuInfo, HardwareInfo};
use sniff::os::{self, OsInfo};
use sniff::request::OsRequest;

use super::{ContextGroup, ContextMergeDiagnostic};

#[cfg(test)]
static GIT_DISCOVERY_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Default)]
enum EvidenceSlot<T> {
    #[default]
    Missing,
    Supplied(T),
}

impl<T> EvidenceSlot<T> {
    fn as_ref(&self) -> Option<&T> {
        match self {
            Self::Missing => None,
            Self::Supplied(value) => Some(value),
        }
    }

    fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }
}

#[derive(Debug, Clone)]
struct RepositoryEvidence {
    root: Option<PathBuf>,
    info: Option<RepoInfo>,
}

/// Request-owned facts used by fail-closed runtime-context capture.
///
/// Every builder marks its corresponding group as supplied. Passing `None`
/// records an observed absence; omitting the builder leaves the evidence
/// missing and produces a partial-capture diagnostic if that group is needed.
#[derive(Debug, Clone)]
pub struct ContextCaptureEvidence {
    environment: HashMap<String, String>,
    git: EvidenceSlot<Option<GitInfo>>,
    repository: EvidenceSlot<RepositoryEvidence>,
    file_changes: EvidenceSlot<Vec<FileChange>>,
    languages: EvidenceSlot<Option<LanguageBreakdown>>,
    documents: EvidenceSlot<Option<Vec<MarkdownMeta>>>,
    skill: EvidenceSlot<Option<String>>,
    os: EvidenceSlot<Option<OsInfo>>,
    hardware: EvidenceSlot<Option<HardwareInfo>>,
    gpus: EvidenceSlot<Vec<GpuInfo>>,
}

impl ContextCaptureEvidence {
    /// Starts an evidence bundle with the invocation's immutable environment.
    pub fn new(environment: HashMap<String, String>) -> Self {
        Self {
            environment,
            git: EvidenceSlot::Missing,
            repository: EvidenceSlot::Missing,
            file_changes: EvidenceSlot::Missing,
            languages: EvidenceSlot::Missing,
            documents: EvidenceSlot::Missing,
            skill: EvidenceSlot::Missing,
            os: EvidenceSlot::Missing,
            hardware: EvidenceSlot::Missing,
            gpus: EvidenceSlot::Missing,
        }
    }

    /// Supplies Git identity/status evidence or an observed non-repository.
    pub fn with_git(mut self, git: Option<GitInfo>) -> Self {
        self.git = EvidenceSlot::Supplied(git);
        self
    }

    /// Supplies topology evidence, deriving its root from `RepoInfo` when present.
    pub fn with_repo(mut self, repo: Option<RepoInfo>) -> Self {
        let root = repo.as_ref().map(|repo| repo.root.clone());
        self.repository = EvidenceSlot::Supplied(RepositoryEvidence { root, info: repo });
        self
    }

    /// Supplies repository presence separately from optional topology.
    ///
    /// This preserves a plain Git checkout whose topology detector returned
    /// `None`, and represents a bare/non-repository source with `root: None`.
    pub fn with_repository(
        mut self,
        root: Option<PathBuf>,
        repo: Option<RepoInfo>,
    ) -> Self {
        self.repository = EvidenceSlot::Supplied(RepositoryEvidence { root, info: repo });
        self
    }

    /// Supplies the file-change facts requested from Sniff.
    pub fn with_file_changes(mut self, changes: Vec<FileChange>) -> Self {
        self.file_changes = EvidenceSlot::Supplied(changes);
        self
    }

    /// Supplies repository language evidence or an observed absence.
    pub fn with_languages(mut self, languages: Option<LanguageBreakdown>) -> Self {
        self.languages = EvidenceSlot::Supplied(languages);
        self
    }

    /// Supplies detected Markdown metadata or an observed absence.
    pub fn with_documents(mut self, documents: Option<Vec<MarkdownMeta>>) -> Self {
        self.documents = EvidenceSlot::Supplied(documents);
        self
    }

    /// Supplies document metadata and derives the matching skill from explicit
    /// repository evidence.
    ///
    /// This helper reads only the supplied repository's skill directories. It
    /// performs no Git discovery or topology detection.
    pub fn with_documents_for_source(
        mut self,
        documents: Option<Vec<MarkdownMeta>>,
        base_dir: &Path,
        repository_root: Option<&Path>,
        repo: Option<&RepoInfo>,
    ) -> Self {
        let (current_package, current_area) = current_package_context(base_dir, repo);
        let skill = repository_root.and_then(|root| {
            super::docs::find_best_skill(
                root,
                current_package.as_ref(),
                current_area.as_deref(),
            )
        });
        self.documents = EvidenceSlot::Supplied(documents);
        self.skill = EvidenceSlot::Supplied(skill);
        self
    }

    /// Supplies the best matching repository-relative skill path, if any.
    pub fn with_skill(mut self, skill: Option<String>) -> Self {
        self.skill = EvidenceSlot::Supplied(skill);
        self
    }

    /// Supplies OS evidence or a retained detection failure/absence.
    pub fn with_os(mut self, os: Option<OsInfo>) -> Self {
        self.os = EvidenceSlot::Supplied(os);
        self
    }

    /// Supplies CPU/memory evidence or a retained detection failure/absence.
    pub fn with_hardware(mut self, hardware: Option<HardwareInfo>) -> Self {
        self.hardware = EvidenceSlot::Supplied(hardware);
        self
    }

    /// Supplies GPU observations; an empty vector is an observed absence.
    pub fn with_gpus(mut self, gpus: Vec<GpuInfo>) -> Self {
        self.gpus = EvidenceSlot::Supplied(gpus);
        self
    }

    pub(super) fn environment(&self) -> &HashMap<String, String> {
        &self.environment
    }
}

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
    pub(super) languages: Option<LanguageBreakdown>,
    pub(super) docs: Option<Vec<MarkdownMeta>>,
    pub(super) best_skill: Option<String>,
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
    /// Uses `GitRepo` for atomic queries. Git identity and file changes share
    /// the one non-Sync handle; independent topology and host probes run in
    /// parallel via `std::thread::scope`.
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

        // `GitRepo` deliberately owns a non-Sync gix handle. Query file changes
        // from that same handle before spawning the independent probes so the
        // file-change group never rediscovers the repository.
        let (file_changes, file_changes_elapsed) = if need_file_changes && has_git {
            let t = Instant::now();
            let changes = git_handle
                .as_ref()
                .and_then(|repo| repo.file_changes().ok())
                .unwrap_or_default();
            (changes, t.elapsed())
        } else {
            (Vec::new(), Duration::ZERO)
        };
        if need_file_changes {
            timings.push(("file_changes".into(), file_changes_elapsed));
        }

        // ── All remaining probes run in parallel ─────────────────────
        let (repo_info, docs, os_info, hardware_info, gpu_names) = std::thread::scope(|s| {
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

                // Docs runs after repo because it needs the package list. OS
                // and hardware probes continue independently while it runs.
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

                (repo_info, docs, os_info, hardware_info, gpu_names)
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
        let (current_package, current_package_area) =
            current_package_context(base_dir, repo_info.as_ref());
        let (dirty_paths, staged_paths, untracked_paths) = changed_paths(&file_changes);
        let best_skill = need_docs.then(|| {
            super::docs::find_best_skill(
                repo_root.as_deref()?,
                current_package.as_ref(),
                current_package_area.as_deref(),
            )
        }).flatten();

        Self {
            base_dir: base_dir.to_path_buf(),
            repo_root,
            repo_name,
            has_git,
            git_branch,
            git_worktree,
            merge_conflicts,
            repo_info,
            languages: None,
            docs,
            best_skill,
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

    /// Builds a capture exclusively from request-owned evidence.
    pub(super) fn from_evidence(
        base_dir: &Path,
        groups: &[ContextGroup],
        evidence: &ContextCaptureEvidence,
    ) -> Self {
        let mut diagnostics = Vec::new();
        let mut missing_areas = std::collections::HashSet::new();
        let mut require = |missing: bool, area: &'static str| {
            if missing && missing_areas.insert(area) {
                diagnostics.push(ContextMergeDiagnostic::PartialRuntimeCapture {
                    area,
                    detail: "required capture evidence was not supplied".to_string(),
                });
            }
        };

        let needs_repository = groups.iter().any(|group| {
            matches!(
                group,
                ContextGroup::Repo
                    | ContextGroup::FileChanges
                    | ContextGroup::Languages
                    | ContextGroup::Documents
            )
        });
        require(
            groups.contains(&ContextGroup::Git) && evidence.git.is_missing(),
            "git",
        );
        require(needs_repository && evidence.git.is_missing(), "git");
        require(
            needs_repository && evidence.repository.is_missing(),
            "repo",
        );
        require(
            groups.contains(&ContextGroup::FileChanges) && evidence.file_changes.is_missing(),
            "file_changes",
        );
        require(
            groups.contains(&ContextGroup::Languages) && evidence.languages.is_missing(),
            "languages",
        );
        require(
            groups.contains(&ContextGroup::Languages)
                && evidence
                    .git
                    .as_ref()
                    .is_some_and(Option::is_some)
                && evidence
                    .languages
                    .as_ref()
                    .is_some_and(Option::is_none),
            "languages",
        );
        require(
            groups.contains(&ContextGroup::Documents) && evidence.documents.is_missing(),
            "documents",
        );
        require(
            groups.contains(&ContextGroup::Documents) && evidence.skill.is_missing(),
            "documents.skill",
        );
        require(
            groups.contains(&ContextGroup::Os) && evidence.os.is_missing(),
            "os",
        );
        require(
            groups.contains(&ContextGroup::Os)
                && evidence.os.as_ref().is_some_and(Option::is_none),
            "os",
        );
        require(
            groups.contains(&ContextGroup::Hardware) && evidence.hardware.is_missing(),
            "hardware",
        );
        require(
            groups.contains(&ContextGroup::Hardware)
                && evidence.hardware.as_ref().is_some_and(Option::is_none),
            "hardware",
        );
        require(
            groups.contains(&ContextGroup::Gpu) && evidence.gpus.is_missing(),
            "gpu",
        );

        let git_info = evidence.git.as_ref().and_then(Option::as_ref);
        let repository = evidence.repository.as_ref();
        let repo_info = repository.and_then(|repository| repository.info.clone());
        let repo_root = repository.and_then(|repository| repository.root.clone());
        let file_changes = evidence
            .file_changes
            .as_ref()
            .cloned()
            .unwrap_or_default();
        let merge_conflicts = evidence
            .file_changes
            .as_ref()
            .map(Vec::as_slice)
            .or_else(|| git_info.map(|info| info.file_changes.as_slice()))
            .unwrap_or_default()
            .iter()
            .filter(|change| change.status == FileStatus::Conflicted)
            .map(|change| change.path.clone())
            .collect();
        let git_worktree = git_info.and_then(|info| {
            info.worktrees
                .values()
                .find(|worktree| worktree.is_current)
                .and_then(|worktree| worktree.filepath.file_name())
                .map(|name| name.to_string_lossy().into_owned())
        });
        let (current_package, current_package_area) =
            current_package_context(base_dir, repo_info.as_ref());
        let (dirty_paths, staged_paths, untracked_paths) = changed_paths(&file_changes);
        let gpu_names = evidence.gpus.as_ref().and_then(|gpus| {
            (!gpus.is_empty()).then(|| {
                gpus.iter()
                    .map(|gpu| gpu.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
        });

        Self {
            base_dir: base_dir.to_path_buf(),
            repo_root,
            repo_name: git_info.and_then(|info| info.repo.clone()),
            has_git: git_info.is_some(),
            git_branch: git_info.and_then(|info| info.current_branch.clone()),
            git_worktree,
            merge_conflicts,
            repo_info,
            languages: evidence.languages.as_ref().cloned().flatten(),
            docs: evidence.documents.as_ref().cloned().flatten(),
            best_skill: evidence.skill.as_ref().cloned().flatten(),
            os_info: evidence.os.as_ref().cloned().flatten(),
            hardware_info: evidence.hardware.as_ref().cloned().flatten(),
            gpu_names,
            current_package,
            current_package_area,
            dirty_paths,
            staged_paths,
            untracked_paths,
            diagnostics,
            timings: Vec::new(),
        }
    }
}

fn current_package_context(
    base_dir: &Path,
    repo_info: Option<&RepoInfo>,
) -> (Option<Package>, Option<String>) {
    let Some(repo) = repo_info.filter(|repo| repo.is_monorepo) else {
        return (None, None);
    };
    let package = repo.packages.as_ref().and_then(|packages| {
        packages
            .iter()
            .find(|package| base_dir.starts_with(&package.path))
            .cloned()
    });
    let area = package
        .as_ref()
        .map(|package| package.package_area.clone())
        .or_else(|| {
            repo.packages.as_ref().and_then(|packages| {
                packages
                    .iter()
                    .find(|package| base_dir.starts_with(repo.root.join(&package.package_area)))
                    .map(|package| package.package_area.clone())
            })
        });
    (package, area)
}

fn changed_paths(file_changes: &[FileChange]) -> (Vec<PathBuf>, Vec<PathBuf>, Vec<PathBuf>) {
    let dirty = file_changes
        .iter()
        .filter(|change| {
            matches!(
                change.status,
                FileStatus::Modified
                    | FileStatus::Both
                    | FileStatus::Staged
                    | FileStatus::Untracked
            )
        })
        .map(|change| change.path.clone())
        .collect();
    let staged = file_changes
        .iter()
        .filter(|change| matches!(change.status, FileStatus::Staged | FileStatus::Both))
        .map(|change| change.path.clone())
        .collect();
    let untracked = file_changes
        .iter()
        .filter(|change| change.status == FileStatus::Untracked)
        .map(|change| change.path.clone())
        .collect();
    (dirty, staged, untracked)
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
            languages: None,
            docs: None,
            best_skill: None,
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

    #[test]
    fn file_changes_reuse_the_original_git_discovery() {
        let outside = tempfile::tempdir().expect("temporary non-repository directory");
        GIT_DISCOVERY_COUNT.store(0, Ordering::Relaxed);

        let _capture = ContextCapture::new(outside.path(), &[ContextGroup::FileChanges]);

        assert_eq!(GIT_DISCOVERY_COUNT.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn supplied_repository_and_file_changes_never_discover_git() {
        let outside = tempfile::tempdir().expect("temporary non-repository directory");
        let evidence = ContextCaptureEvidence::new(HashMap::new())
            .with_git(None)
            .with_repository(None, None)
            .with_file_changes(Vec::new());
        GIT_DISCOVERY_COUNT.store(0, Ordering::Relaxed);

        let capture = ContextCapture::from_evidence(
            outside.path(),
            &[ContextGroup::Repo, ContextGroup::FileChanges],
            &evidence,
        );

        assert_eq!(GIT_DISCOVERY_COUNT.load(Ordering::Relaxed), 0);
        assert!(capture.diagnostics.is_empty());
        assert!(capture.timings.is_empty());
    }

    #[test]
    fn supplied_documents_derive_the_canonical_package_skill() {
        let directory = tempfile::tempdir().expect("temporary repository directory");
        let root = directory.path();
        let package_root = root.join("widgets").join("lib");
        let skill_file = root
            .join(".claude")
            .join("skills")
            .join("widgets")
            .join("SKILL.md");
        std::fs::create_dir_all(skill_file.parent().expect("skill parent"))
            .expect("create skill directory");
        std::fs::create_dir_all(&package_root).expect("create package directory");
        std::fs::write(&skill_file, "# Widgets\n").expect("write skill file");

        let package = Package {
            name: "widgets".to_string(),
            relative: "widgets/lib".to_string(),
            package_area: "widgets".to_string(),
            path: package_root.clone(),
            ..Default::default()
        };
        let repo = test_repo_info(root.to_path_buf(), true, Some(vec![package]));
        let evidence = ContextCaptureEvidence::new(HashMap::new())
            .with_git(None)
            .with_repository(Some(root.to_path_buf()), Some(repo.clone()))
            .with_documents_for_source(
                None,
                &package_root,
                Some(root),
                Some(&repo),
            );

        let capture =
            ContextCapture::from_evidence(&package_root, &[ContextGroup::Documents], &evidence);

        assert_eq!(
            capture.best_skill.as_deref().map(Path::new),
            skill_file.strip_prefix(root).ok(),
        );
        assert!(capture.diagnostics.is_empty());
    }
}
