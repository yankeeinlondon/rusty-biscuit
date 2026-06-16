//! Raw runtime fact capture from chrono, std::env, and sniff.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{Map, Value};
use sniff::filesystem::blast_radius;
use sniff::filesystem::docs::{self, MarkdownMeta};
use sniff::filesystem::git::{FileStatus, GitRepo};
use sniff::filesystem::repo::{self, Package, RepoInfo};
use sniff::hardware::{self, HardwareInfo};
use sniff::os::{self, OsInfo, OsType};
use sniff::request::OsRequest;

use super::diagnostics::ContextMergeDiagnostic;
use super::format;

/// Result of a context capture pass: merged values, any diagnostics, and per-group timings.
type CaptureResult = (
    Map<String, Value>,
    Vec<ContextMergeDiagnostic>,
    Vec<(String, Duration)>,
);

// ── Context groups for demand-driven capture ─────────────────────

/// Groups of context variables that can be independently captured.
///
/// Used for demand-driven capture: only groups whose variables are
/// actually referenced in the document are captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ContextGroup {
    /// Date/time variables: now, today, year, month, dow, etc.
    DateTime,
    /// Git and repo structure: repo, repo_root, is_monorepo, packages, etc.
    Repo,
    /// File change tracking: dirty_files, staged_files, dirty_packages, etc.
    FileChanges,
    /// Programming language and package manager context.
    Languages,
    /// Document discovery: docs_readme, docs_blast_radius, docs_drift, docs_skill.
    Documents,
    /// OS detection: os, os_distro, os_version, os_package_manager.
    Os,
    /// Hardware summary: memory_total, memory_used, cpu_cores, cpu_arch.
    Hardware,
    /// GPU detection (separate — requires subprocess on macOS).
    Gpu,
}

impl ContextGroup {
    /// All available groups.
    pub(crate) fn all() -> [ContextGroup; 8] {
        [
            Self::DateTime,
            Self::Repo,
            Self::FileChanges,
            Self::Languages,
            Self::Documents,
            Self::Os,
            Self::Hardware,
            Self::Gpu,
        ]
    }

    /// Map a ctx variable name to its owning group.
    pub(crate) fn for_key(key: &str) -> Option<ContextGroup> {
        match key {
            // DateTime
            "now"
            | "now_utc"
            | "today"
            | "yesterday"
            | "tomorrow"
            | "today_utc"
            | "yesterday_utc"
            | "tomorrow_utc"
            | "day"
            | "day_abbr"
            | "day_utc"
            | "day_abbr_utc"
            | "year"
            | "year_utc"
            | "month"
            | "month_name"
            | "month_name_abbr"
            | "day_of_month"
            | "day_of_month_suffixed"
            | "time"
            | "time_military"
            | "time_utc"
            | "time_military_utc"
            | "timezone"
            | "timezone_offset"
            | "timezone_iana"
            | "start_of_week_sun"
            | "end_of_week_sun"
            | "start_of_week_mon"
            | "end_of_week_mon"
            | "start_of_week_sun_utc"
            | "end_of_week_sun_utc"
            | "start_of_week_mon_utc"
            | "end_of_week_mon_utc"
            | "season"
            | "timestamp"
            | "timestamp_ms" => Some(Self::DateTime),

            // Repo
            "repo"
            | "repo_root"
            | "is_monorepo"
            | "package_root"
            | "package_area_root"
            | "packages"
            | "packages_list"
            | "package_areas"
            | "package_areas_list"
            | "current_package"
            | "current_package_area"
            | "area"
            | "area_description"
            | "area_root"
            | "current_packages"
            | "depends_on"
            | "used_by" => Some(Self::Repo),

            // FileChanges
            "dirty_files"
            | "dirty_files_list"
            | "dirty_source_code_files"
            | "dirty_source_code_files_list"
            | "staged_files"
            | "staged_files_list"
            | "untracked_files"
            | "untracked_files_list"
            | "dirty_packages"
            | "dirty_packages_list"
            | "dirty_package_areas"
            | "dirty_package_areas_list"
            | "staged_packages"
            | "staged_packages_list"
            | "staged_package_areas"
            | "staged_package_areas_list"
            | "current_package_has_staged_files"
            | "current_package_area_has_staged_files"
            | "current_package_has_dirty_files"
            | "current_package_area_has_dirty_files" => Some(Self::FileChanges),

            // Languages
            "programming_languages_in_repo" | "programming_language" | "package_manager" => {
                Some(Self::Languages)
            }

            // Documents
            "docs_readme" | "docs_blast_radius" | "docs_drift" | "docs_skill" => {
                Some(Self::Documents)
            }

            // OS
            "os" | "os_distro" | "os_package_manager" | "os_version" => Some(Self::Os),

            // Hardware (CPU + memory — no subprocess)
            "memory_total" | "memory_used" | "memory_avail" | "cpu_cores" | "cpu_arch" => {
                Some(Self::Hardware)
            }

            // GPU (requires ioreg subprocess on macOS)
            "gpu" => Some(Self::Gpu),

            _ => None,
        }
    }
}

/// Scan document content for referenced `ctx.*` variables and return
/// the set of context groups needed.
///
/// Scans `{{ ctx.KEY }}` patterns in interpolation expressions and
/// `when="..."` condition strings. Does not parse the full AST — uses
/// simple substring scanning for speed.
pub(crate) fn scan_needed_groups(content: &str) -> HashSet<ContextGroup> {
    let mut groups = HashSet::new();

    // Fast path: if no "ctx." anywhere, nothing to capture
    if !content.contains("ctx.") {
        return groups;
    }

    // Scan for ctx.KEY patterns — covers both {{ ctx.KEY }} and when="ctx.KEY ..."
    let bytes = content.as_bytes();
    let prefix = b"ctx.";
    let mut pos = 0;
    while pos + prefix.len() < bytes.len() {
        if let Some(offset) = content[pos..].find("ctx.") {
            let start = pos + offset + prefix.len();
            // Extract the key name (alphanumeric + underscore)
            let key_end = content[start..]
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .map(|i| start + i)
                .unwrap_or(content.len());
            let key = &content[start..key_end];
            if !key.is_empty()
                && let Some(group) = ContextGroup::for_key(key)
            {
                groups.insert(group);
            }
            // Unknown keys are user-defined ctx (from frontmatter),
            // not runtime-captured — no additional groups needed.
            pos = key_end;
        } else {
            break;
        }
    }

    groups
}

/// Intermediate struct holding all raw sniff results.
///
/// Built once per compose run to avoid repeated sniff calls. All derived
/// context variables are computed from this single struct.
///
/// Uses `GitRepo` for atomic queries instead of the monolithic `detect_git`,
/// so only the fields actually needed are computed.
struct ContextCapture {
    /// Directory the capture was built for (the compose working directory).
    base_dir: PathBuf,
    /// Repository root path (cheap: from `GitRepo::discover`).
    repo_root: Option<PathBuf>,
    /// Repo name parsed from preferred remote URL (cheap: reads git config).
    repo_name: Option<String>,
    /// Whether a git repo was found at all.
    has_git: bool,
    repo_info: Option<RepoInfo>,
    docs: Option<Vec<MarkdownMeta>>,
    os_info: Option<OsInfo>,
    hardware_info: Option<HardwareInfo>,
    gpu_names: Option<String>,
    current_package: Option<Package>,
    current_package_area: Option<String>,
    dirty_paths: Vec<PathBuf>,
    staged_paths: Vec<PathBuf>,
    untracked_paths: Vec<PathBuf>,
    diagnostics: Vec<ContextMergeDiagnostic>,
    timings: Vec<(String, Duration)>,
}

impl ContextCapture {
    /// Build the capture from a base directory for the requested groups.
    ///
    /// Uses `GitRepo` for atomic queries. `GitRepo::discover` is the only
    /// sequential step (cheap — just finds `.git`). All other probes run
    /// in parallel via `std::thread::scope`.
    fn new(base_dir: &Path, groups: &[ContextGroup]) -> Self {
        let mut diagnostics = Vec::new();
        let mut timings = Vec::new();

        let need_git = groups.iter().any(|g| {
            matches!(
                g,
                ContextGroup::Repo
                    | ContextGroup::FileChanges
                    | ContextGroup::Languages
                    | ContextGroup::Documents
            )
        });
        let need_file_changes = groups.contains(&ContextGroup::FileChanges);
        let need_repo = groups.iter().any(|g| {
            matches!(
                g,
                ContextGroup::Repo
                    | ContextGroup::FileChanges
                    | ContextGroup::Languages
                    | ContextGroup::Documents
            )
        });
        let need_docs = groups.contains(&ContextGroup::Documents);
        let need_os = groups.contains(&ContextGroup::Os);
        let need_hw = groups.contains(&ContextGroup::Hardware);
        let need_gpu = groups.contains(&ContextGroup::Gpu);

        // ── Git discovery (near-instant — just finds .git) ───────────
        let t = Instant::now();
        let git_handle = if need_git {
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
        let repo_root = git_handle.as_ref().map(|h| h.repo_root().to_path_buf());
        let (_org, repo_name) = git_handle
            .as_ref()
            .map(|h| h.org_and_repo())
            .unwrap_or((None, None));
        if need_git {
            timings.push(("git".into(), t.elapsed()));
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
                            .and_then(|root| repo::detect_repo_structure(root).ok().flatten());
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
                        .and_then(|root| docs::detect_docs_with_packages(root, &package_list));
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

/// Capture all runtime context variables for the given base directory.
pub(crate) fn capture_runtime_context(base_dir: &Path) -> CaptureResult {
    capture_runtime_context_for_groups(base_dir, &ContextGroup::all())
}

/// Capture only the context groups needed for the given document content.
///
/// Scans `content` for `ctx.*` references and only captures the required
/// groups. If no `ctx.*` references are found, only populates datetime
/// (which is purely local computation with no I/O).
pub(crate) fn capture_runtime_context_for_content(base_dir: &Path, content: &str) -> CaptureResult {
    let mut groups = scan_needed_groups(content);
    // DateTime is always included (zero-cost local computation)
    groups.insert(ContextGroup::DateTime);
    let groups_vec: Vec<ContextGroup> = groups.into_iter().collect();
    capture_runtime_context_for_groups(base_dir, &groups_vec)
}

/// Capture runtime context for the specified groups only.
pub(crate) fn capture_runtime_context_for_groups(
    base_dir: &Path,
    groups: &[ContextGroup],
) -> CaptureResult {
    let cap = ContextCapture::new(base_dir, groups);
    let mut values = Map::new();

    if groups.contains(&ContextGroup::DateTime) {
        populate_datetime(&mut values);
    }

    if groups.contains(&ContextGroup::Repo) {
        populate_repo(&cap, &mut values);
    }

    if groups.contains(&ContextGroup::FileChanges) {
        populate_file_changes(&cap, &mut values);
        populate_package_changes(&cap, &mut values);
    }

    if groups.contains(&ContextGroup::Languages) {
        populate_languages(&cap, &mut values);
    }

    if groups.contains(&ContextGroup::Documents) {
        populate_docs(&cap, &mut values);
        populate_skills(&cap, &mut values);
    }

    if groups.contains(&ContextGroup::Os) {
        populate_os(&cap, &mut values);
    }

    if groups.contains(&ContextGroup::Hardware) {
        populate_hardware(&cap, &mut values);
    }

    (values, cap.diagnostics, cap.timings)
}

// ── Date/time helpers ─────────────────────────────────────────────

pub(crate) fn populate_datetime(values: &mut Map<String, Value>) {
    use chrono::{Datelike, Local, Utc};

    let now_local = Local::now();
    let now_utc = Utc::now();

    let today = now_local.date_naive();
    let yesterday = today - chrono::Duration::days(1);
    let tomorrow = today + chrono::Duration::days(1);

    let today_utc = now_utc.date_naive();
    let yesterday_utc = today_utc - chrono::Duration::days(1);
    let tomorrow_utc = today_utc + chrono::Duration::days(1);

    // Legacy fields
    let now_str = now_local.format("%Y-%m-%dT%H:%M:%S").to_string();
    let utc_str = now_utc.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let today_str = today.format("%Y-%m-%d").to_string();
    let yesterday_str = yesterday.format("%Y-%m-%d").to_string();
    let tomorrow_str = tomorrow.format("%Y-%m-%d").to_string();
    let year_str = now_local.format("%Y").to_string();
    let month_str = now_local.format("%m").to_string();
    let month_name_str = now_local.format("%B").to_string();
    let month_name_abbr_str = now_local.format("%b").to_string();

    // Core date/time
    values.insert("now".into(), Value::String(now_str));
    values.insert("now_utc".into(), Value::String(utc_str));
    values.insert("today".into(), Value::String(today_str));
    values.insert("yesterday".into(), Value::String(yesterday_str));
    values.insert("tomorrow".into(), Value::String(tomorrow_str));

    // UTC date variants
    values.insert(
        "today_utc".into(),
        Value::String(today_utc.format("%Y-%m-%d").to_string()),
    );
    values.insert(
        "yesterday_utc".into(),
        Value::String(yesterday_utc.format("%Y-%m-%d").to_string()),
    );
    values.insert(
        "tomorrow_utc".into(),
        Value::String(tomorrow_utc.format("%Y-%m-%d").to_string()),
    );

    // Day of week
    values.insert(
        "day".into(),
        Value::String(now_local.format("%A").to_string()),
    );
    values.insert(
        "day_abbr".into(),
        Value::String(now_local.format("%a").to_string()),
    );

    // UTC day of week
    let dow_utc_str = now_utc.format("%A").to_string();
    let dow_abbr_utc_str = now_utc.format("%a").to_string();
    values.insert("day_utc".into(), Value::String(dow_utc_str));
    values.insert("day_abbr_utc".into(), Value::String(dow_abbr_utc_str));

    // Year/month
    values.insert("year".into(), Value::String(year_str));
    values.insert(
        "year_utc".into(),
        Value::String(now_utc.format("%Y").to_string()),
    );
    values.insert("month".into(), Value::String(month_str));
    values.insert("month_name".into(), Value::String(month_name_str));
    values.insert("month_name_abbr".into(), Value::String(month_name_abbr_str));

    // Day of month
    let day_of_month = today.day();
    values.insert(
        "day_of_month".into(),
        Value::String(day_of_month.to_string()),
    );
    values.insert(
        "day_of_month_suffixed".into(),
        Value::String(format!(
            "{}{}",
            day_of_month,
            format::ordinal_suffix(day_of_month)
        )),
    );

    // Time fields
    values.insert(
        "time".into(),
        Value::String(now_local.format("%I:%M %p").to_string()),
    );
    values.insert(
        "time_military".into(),
        Value::String(now_local.format("%H:%M").to_string()),
    );
    values.insert(
        "time_utc".into(),
        Value::String(format!("{} (UTC)", now_utc.format("%I:%M %p"))),
    );
    values.insert(
        "time_military_utc".into(),
        Value::String(format!("{} (UTC)", now_utc.format("%H:%M"))),
    );

    // Timezone: sniff owns abbreviation derivation (handles chrono's
    // %Z offset fallback on macOS via IANA-to-abbreviation mapping).
    let tz_info = sniff::os::detect_timezone();
    values.insert(
        "timezone".into(),
        tz_info.timezone_abbr.map_or(Value::Null, Value::String),
    );
    values.insert(
        "timezone_offset".into(),
        Value::String(now_local.format("%z").to_string()),
    );
    values.insert(
        "timezone_iana".into(),
        tz_info.timezone.map_or(Value::Null, Value::String),
    );

    // Week boundaries (Sunday start)
    let weekday_num = today.weekday().num_days_from_sunday();
    let start_of_week_sun = today - chrono::Duration::days(weekday_num as i64);
    let end_of_week_sun = start_of_week_sun + chrono::Duration::days(6);
    values.insert(
        "start_of_week_sun".into(),
        Value::String(start_of_week_sun.format("%Y-%m-%d").to_string()),
    );
    values.insert(
        "end_of_week_sun".into(),
        Value::String(end_of_week_sun.format("%Y-%m-%d").to_string()),
    );

    // Week boundaries (Monday start)
    let weekday_mon = today.weekday().num_days_from_monday();
    let start_of_week_mon = today - chrono::Duration::days(weekday_mon as i64);
    let end_of_week_mon = start_of_week_mon + chrono::Duration::days(6);
    values.insert(
        "start_of_week_mon".into(),
        Value::String(start_of_week_mon.format("%Y-%m-%d").to_string()),
    );
    values.insert(
        "end_of_week_mon".into(),
        Value::String(end_of_week_mon.format("%Y-%m-%d").to_string()),
    );

    // UTC week boundaries
    let weekday_utc_sun = today_utc.weekday().num_days_from_sunday();
    let start_utc_sun = today_utc - chrono::Duration::days(weekday_utc_sun as i64);
    let end_utc_sun = start_utc_sun + chrono::Duration::days(6);
    values.insert(
        "start_of_week_sun_utc".into(),
        Value::String(start_utc_sun.format("%Y-%m-%d").to_string()),
    );
    values.insert(
        "end_of_week_sun_utc".into(),
        Value::String(end_utc_sun.format("%Y-%m-%d").to_string()),
    );

    let weekday_utc_mon = today_utc.weekday().num_days_from_monday();
    let start_utc_mon = today_utc - chrono::Duration::days(weekday_utc_mon as i64);
    let end_utc_mon = start_utc_mon + chrono::Duration::days(6);
    values.insert(
        "start_of_week_mon_utc".into(),
        Value::String(start_utc_mon.format("%Y-%m-%d").to_string()),
    );
    values.insert(
        "end_of_week_mon_utc".into(),
        Value::String(end_utc_mon.format("%Y-%m-%d").to_string()),
    );

    // Season
    values.insert(
        "season".into(),
        Value::String(format::determine_season(today.month(), today.day()).to_string()),
    );

    // Timestamps
    values.insert(
        "timestamp".into(),
        Value::Number(now_utc.timestamp().into()),
    );
    values.insert(
        "timestamp_ms".into(),
        Value::Number(now_utc.timestamp_millis().into()),
    );

    // Backward-compatible aliases (documented in `context-variables.md`).
    // Keep these in sync with the docs: any documented alias must resolve to
    // the same value as its canonical key so interpolation and the
    // `claudine context --values` report agree.
    populate_datetime_aliases(values);
}

/// Insert documented backward-compatible aliases for date/time keys.
///
/// Each alias mirrors the value of an existing canonical key. Aliases are
/// inserted only when the canonical key is present so a missing canonical
/// value cannot leak as a populated alias.
fn populate_datetime_aliases(values: &mut Map<String, Value>) {
    const ALIASES: &[(&str, &str)] =
        &[("utc", "now_utc"), ("dow", "day"), ("dow_abbr", "day_abbr")];
    for (alias, canonical) in ALIASES {
        if let Some(value) = values.get(*canonical).cloned() {
            values.insert((*alias).to_string(), value);
        }
    }
}

// ── Repo context ──────────────────────────────────────────────────

/// Removes a single trailing path separator so `repo_root` is join-safe and
/// matches `sniff repo root` (which also omits the trailing `/`).
fn strip_trailing_sep(s: &str) -> String {
    let trimmed = s.strip_suffix('/').or_else(|| s.strip_suffix('\\'));
    trimmed.unwrap_or(s).to_string()
}

fn populate_repo(cap: &ContextCapture, values: &mut Map<String, Value>) {
    let repo = cap.repo_info.as_ref();

    // repo and repo_root
    values.insert(
        "repo".into(),
        cap.repo_name
            .as_ref()
            .map_or(Value::Null, |r| Value::String(r.clone())),
    );
    values.insert(
        "repo_root".into(),
        cap.repo_root.as_ref().map_or(Value::Null, |r| {
            Value::String(strip_trailing_sep(&r.to_string_lossy()))
        }),
    );

    // is_monorepo
    values.insert(
        "is_monorepo".into(),
        Value::Bool(repo.is_some_and(|r| r.is_monorepo)),
    );

    // Package fields (null if not monorepo)
    let is_mono = repo.is_some_and(|r| r.is_monorepo);
    if is_mono {
        values.insert(
            "package_root".into(),
            cap.current_package.as_ref().map_or(Value::Null, |p| {
                Value::String(p.path.to_string_lossy().to_string())
            }),
        );

        values.insert(
            "package_area_root".into(),
            cap.current_package_area
                .as_ref()
                .map_or(Value::Null, |area| {
                    let root = repo.unwrap().root.join(area);
                    Value::String(root.to_string_lossy().to_string())
                }),
        );

        let packages: Vec<String> = repo
            .and_then(|r| r.packages.as_ref())
            .map(|pkgs| pkgs.iter().map(|p| p.name.clone()).collect())
            .unwrap_or_default();
        values.insert(
            "packages".into(),
            Value::String(format::format_csv(&packages)),
        );
        values.insert(
            "packages_list".into(),
            Value::String(format::format_md_list(&packages)),
        );

        let areas: Vec<String> = repo
            .and_then(|r| r.packages.as_ref())
            .map(|pkgs| {
                let mut areas: Vec<String> = pkgs.iter().map(|p| p.package_area.clone()).collect();
                areas.sort();
                areas.dedup();
                areas
            })
            .unwrap_or_default();
        values.insert(
            "package_areas".into(),
            Value::String(format::format_csv(&areas)),
        );
        values.insert(
            "package_areas_list".into(),
            Value::String(format::format_md_list(&areas)),
        );

        values.insert(
            "current_package".into(),
            cap.current_package
                .as_ref()
                .map_or(Value::Null, |p| Value::String(p.name.clone())),
        );
        values.insert(
            "current_package_area".into(),
            cap.current_package_area
                .as_ref()
                .map_or(Value::Null, |a| Value::String(a.clone())),
        );
    } else {
        values.insert("package_root".into(), Value::Null);
        values.insert("package_area_root".into(), Value::Null);
        values.insert("packages".into(), Value::Null);
        values.insert("packages_list".into(), Value::Null);
        values.insert("package_areas".into(), Value::Null);
        values.insert("package_areas_list".into(), Value::Null);
        values.insert("current_package".into(), Value::Null);
        values.insert("current_package_area".into(), Value::Null);
    }

    populate_monorepo_area(cap, values);
}

/// Populate the monorepo-scoped `area*`, `current_packages`, `depends_on`, and
/// `used_by` variables.
///
/// These are documented as monorepo-only. Outside a monorepo the `area*`
/// variables fall back per spec (empty strings; `area_root` is the repo root)
/// and the list-shaped variables render as empty strings.
fn populate_monorepo_area(cap: &ContextCapture, values: &mut Map<String, Value>) {
    let repo = cap.repo_info.as_ref();
    let is_monorepo = repo.is_some_and(|r| r.is_monorepo);

    let repo_root_str = || {
        cap.repo_root
            .as_ref()
            .map(|r| strip_trailing_sep(&r.to_string_lossy()))
            .unwrap_or_default()
    };

    let (area, area_description, area_root) = if !is_monorepo {
        (String::new(), String::new(), repo_root_str())
    } else if let Some(pkg) = cap.current_package.as_ref() {
        // Inside a package folder: the area is the package itself.
        (
            pkg.name.clone(),
            format!("{} package", pkg.name),
            strip_trailing_sep(&pkg.path.to_string_lossy()),
        )
    } else if let Some(area_name) = cap.current_package_area.as_deref().filter(|a| *a != "root") {
        // Inside a package area but not a package folder.
        let area_path = cap
            .repo_root
            .as_ref()
            .map(|r| r.join(area_name))
            .map(|p| strip_trailing_sep(&p.to_string_lossy()))
            .unwrap_or_default();
        (
            area_name.to_string(),
            format!("{area_name} package area"),
            area_path,
        )
    } else {
        // Monorepo root.
        (String::new(), String::new(), repo_root_str())
    };

    values.insert("area".into(), Value::String(area.clone()));
    values.insert("area_description".into(), Value::String(area_description));
    values.insert("area_root".into(), Value::String(area_root));

    // current_packages: Markdown bullet list of packages under the cwd.
    let current_packages = repo
        .and_then(|r| r.packages.as_ref())
        .map(|pkgs| {
            pkgs.iter()
                .filter(|p| p.path.starts_with(&cap.base_dir))
                .map(|p| format!("- {} ({})", p.name, p.relative))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    values.insert("current_packages".into(), Value::String(current_packages));

    // depends_on / used_by are scoped to the current `area`: the current
    // package when in one, all packages in the area when in an area, else all.
    let scope: Vec<&Package> = if let Some(pkg) = cap.current_package.as_ref() {
        vec![pkg]
    } else if !area.is_empty() {
        repo.and_then(|r| r.packages.as_ref())
            .map(|pkgs| pkgs.iter().filter(|p| p.package_area == area).collect())
            .unwrap_or_default()
    } else {
        repo.and_then(|r| r.packages.as_ref())
            .map(|pkgs| pkgs.iter().collect())
            .unwrap_or_default()
    };

    let depends_on = render_dependency_list(
        &scope,
        |p| p.depends_on.as_slice(),
        "depends on",
        |name| format!("- '{name}' has no dependencies on other packages in this monorepo"),
    );
    values.insert("depends_on".into(), Value::String(depends_on));

    let used_by = render_dependency_list(
        &scope,
        |p| p.used_by.as_slice(),
        "is used by",
        |name| format!("- '{name}' is not used by other packages in this monorepo"),
    );
    values.insert("used_by".into(), Value::String(used_by));
}

/// Render a nested Markdown dependency listing for the scoped packages.
///
/// `edges` yields the related package names; `verb` and `empty_line` supply the
/// wording (e.g. "depends on" / "has no dependencies …").
fn render_dependency_list(
    scope: &[&Package],
    edges: impl Fn(&Package) -> &[String],
    verb: &str,
    empty_line: impl Fn(&str) -> String,
) -> String {
    let mut out = Vec::new();
    for &pkg in scope {
        let deps = edges(pkg);
        if deps.is_empty() {
            out.push(empty_line(&pkg.name));
        } else {
            out.push(format!("- '{}' {verb}:", pkg.name));
            for d in deps {
                out.push(format!("    - {d}"));
            }
        }
    }
    out.join("\n")
}

// ── File changes context ──────────────────────────────────────────

fn populate_file_changes(cap: &ContextCapture, values: &mut Map<String, Value>) {
    // Dirty files
    let mut dirty: Vec<String> = cap
        .dirty_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    dirty.sort();
    values.insert(
        "dirty_files".into(),
        Value::String(format::format_csv(&dirty)),
    );
    values.insert(
        "dirty_files_list".into(),
        Value::String(format::format_md_list(&dirty)),
    );

    // Dirty source code files
    let dirty_source: Vec<String> = dirty
        .iter()
        .filter(|p| blast_radius::is_source_code_path(Path::new(p.as_str())))
        .cloned()
        .collect();
    values.insert(
        "dirty_source_code_files".into(),
        Value::String(format::format_csv(&dirty_source)),
    );
    values.insert(
        "dirty_source_code_files_list".into(),
        Value::String(format::format_md_list(&dirty_source)),
    );

    // Staged files
    let mut staged: Vec<String> = cap
        .staged_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    staged.sort();
    values.insert(
        "staged_files".into(),
        Value::String(format::format_csv(&staged)),
    );
    values.insert(
        "staged_files_list".into(),
        Value::String(format::format_md_list(&staged)),
    );

    // Untracked files
    let mut untracked: Vec<String> = cap
        .untracked_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    untracked.sort();
    values.insert(
        "untracked_files".into(),
        Value::String(format::format_csv(&untracked)),
    );
    values.insert(
        "untracked_files_list".into(),
        Value::String(format::format_md_list(&untracked)),
    );
}

// ── Package/area dirty/staged context ─────────────────────────────

fn populate_package_changes(cap: &ContextCapture, values: &mut Map<String, Value>) {
    let repo = cap.repo_info.as_ref();
    let packages = repo.and_then(|r| r.packages.as_ref());
    let is_mono = repo.is_some_and(|r| r.is_monorepo);

    if !is_mono || packages.is_none() {
        // Default boolean flags to false
        values.insert("dirty_packages".into(), Value::String(String::new()));
        values.insert("dirty_packages_list".into(), Value::String(String::new()));
        values.insert("dirty_package_areas".into(), Value::String(String::new()));
        values.insert(
            "dirty_package_areas_list".into(),
            Value::String(String::new()),
        );
        values.insert("staged_packages".into(), Value::String(String::new()));
        values.insert("staged_packages_list".into(), Value::String(String::new()));
        values.insert("staged_package_areas".into(), Value::String(String::new()));
        values.insert(
            "staged_package_areas_list".into(),
            Value::String(String::new()),
        );
        values.insert(
            "current_package_has_staged_files".into(),
            Value::Bool(false),
        );
        values.insert(
            "current_package_area_has_staged_files".into(),
            Value::Bool(false),
        );
        values.insert("current_package_has_dirty_files".into(), Value::Bool(false));
        values.insert(
            "current_package_area_has_dirty_files".into(),
            Value::Bool(false),
        );
        return;
    }

    let pkgs = packages.unwrap();

    // Helper: check if a repo-relative path belongs to a package
    let path_in_package = |path: &Path, pkg: &Package| -> bool {
        let pkg_rel = &pkg.relative;
        let path_str = path.to_string_lossy();
        path_str.starts_with(pkg_rel)
    };

    // Dirty packages
    let mut dirty_pkg_names: Vec<String> = pkgs
        .iter()
        .filter(|pkg| cap.dirty_paths.iter().any(|p| path_in_package(p, pkg)))
        .map(|p| p.name.clone())
        .collect();
    dirty_pkg_names.sort();
    dirty_pkg_names.dedup();

    let mut dirty_area_names: Vec<String> = pkgs
        .iter()
        .filter(|pkg| cap.dirty_paths.iter().any(|p| path_in_package(p, pkg)))
        .map(|p| p.package_area.clone())
        .collect();
    dirty_area_names.sort();
    dirty_area_names.dedup();

    values.insert(
        "dirty_packages".into(),
        Value::String(format::format_csv(&dirty_pkg_names)),
    );
    values.insert(
        "dirty_packages_list".into(),
        Value::String(format::format_md_list(&dirty_pkg_names)),
    );
    values.insert(
        "dirty_package_areas".into(),
        Value::String(format::format_csv(&dirty_area_names)),
    );
    values.insert(
        "dirty_package_areas_list".into(),
        Value::String(format::format_md_list(&dirty_area_names)),
    );

    // Staged packages
    let mut staged_pkg_names: Vec<String> = pkgs
        .iter()
        .filter(|pkg| cap.staged_paths.iter().any(|p| path_in_package(p, pkg)))
        .map(|p| p.name.clone())
        .collect();
    staged_pkg_names.sort();
    staged_pkg_names.dedup();

    let mut staged_area_names: Vec<String> = pkgs
        .iter()
        .filter(|pkg| cap.staged_paths.iter().any(|p| path_in_package(p, pkg)))
        .map(|p| p.package_area.clone())
        .collect();
    staged_area_names.sort();
    staged_area_names.dedup();

    values.insert(
        "staged_packages".into(),
        Value::String(format::format_csv(&staged_pkg_names)),
    );
    values.insert(
        "staged_packages_list".into(),
        Value::String(format::format_md_list(&staged_pkg_names)),
    );
    values.insert(
        "staged_package_areas".into(),
        Value::String(format::format_csv(&staged_area_names)),
    );
    values.insert(
        "staged_package_areas_list".into(),
        Value::String(format::format_md_list(&staged_area_names)),
    );

    // Current package/area boolean flags
    let cur_pkg_name = cap.current_package.as_ref().map(|p| &p.name);
    let cur_area = cap.current_package_area.as_deref();

    values.insert(
        "current_package_has_staged_files".into(),
        Value::Bool(cur_pkg_name.is_some_and(|name| staged_pkg_names.contains(name))),
    );
    values.insert(
        "current_package_area_has_staged_files".into(),
        Value::Bool(cur_area.is_some_and(|area| staged_area_names.iter().any(|a| a == area))),
    );
    values.insert(
        "current_package_has_dirty_files".into(),
        Value::Bool(cur_pkg_name.is_some_and(|name| dirty_pkg_names.contains(name))),
    );
    values.insert(
        "current_package_area_has_dirty_files".into(),
        Value::Bool(cur_area.is_some_and(|area| dirty_area_names.iter().any(|a| a == area))),
    );
}

// ── Programming language and package manager ──────────────────────

fn populate_languages(cap: &ContextCapture, values: &mut Map<String, Value>) {
    let repo = cap.repo_info.as_ref();

    if !cap.has_git {
        values.insert("programming_languages_in_repo".into(), Value::Null);
        values.insert("programming_language".into(), Value::Null);
        values.insert("package_manager".into(), Value::Null);
        return;
    }

    let packages = repo.and_then(|r| r.packages.as_ref());
    let is_mono = repo.is_some_and(|r| r.is_monorepo);

    // All languages across packages
    let all_langs: Vec<String> = packages
        .map(|pkgs| {
            let mut langs: Vec<String> = pkgs
                .iter()
                .filter_map(|p| p.primary_language.as_ref())
                .map(|l| format!("{l:?}"))
                .collect();
            langs.sort();
            langs.dedup();
            langs
        })
        .unwrap_or_default();

    values.insert(
        "programming_languages_in_repo".into(),
        if all_langs.is_empty() {
            Value::Null
        } else {
            Value::String(format::format_csv(&all_langs))
        },
    );

    // programming_language
    let programming_language = if is_mono {
        if let Some(ref pkg) = cap.current_package {
            // In a package: that package's primary language
            pkg.primary_language.as_ref().map(|l| format!("{l:?}"))
        } else if let Some(ref area) = cap.current_package_area {
            // In a package area: unique languages across packages in that area
            packages.map(|pkgs| {
                let mut area_langs: Vec<String> = pkgs
                    .iter()
                    .filter(|p| &p.package_area == area)
                    .filter_map(|p| p.primary_language.as_ref())
                    .map(|l| format!("{l:?}"))
                    .collect();
                area_langs.sort();
                area_langs.dedup();
                format::format_csv(&area_langs)
            })
        } else {
            // In monorepo root but not in a package/area
            if all_langs.len() == 1 {
                Some(all_langs[0].clone())
            } else {
                None
            }
        }
    } else {
        // Not monorepo: repo's primary language
        if all_langs.len() == 1 {
            Some(all_langs[0].clone())
        } else {
            all_langs.first().cloned()
        }
    };
    values.insert(
        "programming_language".into(),
        programming_language.map_or(Value::Null, Value::String),
    );

    // package_manager
    let package_manager = if is_mono {
        if let Some(ref pkg) = cap.current_package {
            pkg.package_managers.first().cloned()
        } else if let Some(ref area) = cap.current_package_area {
            let mut managers: Vec<String> = packages
                .map(|pkgs| {
                    pkgs.iter()
                        .filter(|p| &p.package_area == area)
                        .filter_map(|p| p.package_managers.first().cloned())
                        .collect()
                })
                .unwrap_or_default();
            managers.sort();
            managers.dedup();
            if managers.len() == 1 {
                Some(managers.remove(0))
            } else {
                None
            }
        } else {
            let mut managers: Vec<String> = packages
                .map(|pkgs| {
                    pkgs.iter()
                        .filter_map(|p| p.package_managers.first().cloned())
                        .collect()
                })
                .unwrap_or_default();
            managers.sort();
            managers.dedup();
            if managers.len() == 1 {
                Some(managers.remove(0))
            } else {
                None
            }
        }
    } else {
        packages.and_then(|pkgs| {
            pkgs.first()
                .and_then(|p| p.package_managers.first().cloned())
        })
    };
    values.insert(
        "package_manager".into(),
        package_manager.map_or(Value::Null, Value::String),
    );
}

// ── Document context ──────────────────────────────────────────────

fn populate_docs(cap: &ContextCapture, values: &mut Map<String, Value>) {
    let docs = cap.docs.as_ref();
    let is_mono = cap.repo_info.as_ref().is_some_and(|r| r.is_monorepo);

    // Scope filter: filter docs to the active scope
    let scope_filter = |doc: &MarkdownMeta| -> bool {
        if !is_mono {
            return true;
        }
        if let Some(ref pkg) = cap.current_package {
            doc.package.as_deref() == Some(&pkg.name)
        } else if let Some(ref area) = cap.current_package_area {
            // In an area: include docs from packages in this area
            cap.repo_info
                .as_ref()
                .and_then(|r| r.packages.as_ref())
                .map(|pkgs| {
                    pkgs.iter()
                        .filter(|p| &p.package_area == area)
                        .any(|p| doc.package.as_deref() == Some(&p.name))
                })
                .unwrap_or(false)
        } else {
            true // repo-wide
        }
    };

    // docs_readme
    let readmes: Vec<String> = docs
        .map(|all_docs| {
            all_docs
                .iter()
                .filter(|d| scope_filter(d))
                .filter(|d| {
                    d.filepath
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.eq_ignore_ascii_case("readme.md"))
                        .unwrap_or(false)
                })
                .map(|d| d.relative.clone())
                .collect()
        })
        .unwrap_or_default();
    values.insert(
        "docs_readme".into(),
        Value::String(format::format_csv(&readmes)),
    );

    // docs_blast_radius: docs with blast_radius frontmatter
    let blast_radius_docs: Vec<String> = docs
        .map(|all_docs| {
            all_docs
                .iter()
                .filter(|d| scope_filter(d))
                .filter(|d| d.blast_radius.is_some())
                .map(|d| d.relative.clone())
                .collect()
        })
        .unwrap_or_default();
    values.insert(
        "docs_blast_radius".into(),
        Value::String(format::format_csv(&blast_radius_docs)),
    );

    // docs_drift: docs whose blast_radius intersects dirty source set.
    // Uses normalized PathBuf comparison consistent with sniff's
    // find_blast_radius_documents() rather than substring matching.
    let dirty_source: std::collections::HashSet<PathBuf> = cap
        .dirty_paths
        .iter()
        .filter(|p| blast_radius::is_source_code_path(p))
        .cloned()
        .collect();

    let drift_docs: Vec<String> = docs
        .map(|all_docs| {
            all_docs
                .iter()
                .filter(|d| scope_filter(d))
                .filter(|d| {
                    if let Some(ref br) = d.blast_radius {
                        br.iter().any(|p| dirty_source.contains(p))
                    } else {
                        false
                    }
                })
                .map(|d| d.relative.clone())
                .collect()
        })
        .unwrap_or_default();
    values.insert(
        "docs_drift".into(),
        Value::String(format::format_csv(&drift_docs)),
    );
}

// ── Skill context ─────────────────────────────────────────────────

fn populate_skills(cap: &ContextCapture, values: &mut Map<String, Value>) {
    let repo_root = cap.repo_root.as_ref();

    let skill = repo_root.and_then(|root| {
        find_best_skill(
            root,
            cap.current_package.as_ref(),
            cap.current_package_area.as_deref(),
        )
    });

    values.insert(
        "docs_skill".into(),
        skill.map_or(Value::Null, Value::String),
    );
}

fn find_best_skill(
    repo_root: &Path,
    current_package: Option<&Package>,
    current_area: Option<&str>,
) -> Option<String> {
    let skill_dirs = [".claude/skills", ".agents/skills"];

    for base in &skill_dirs {
        let skills_dir = repo_root.join(base);
        if !skills_dir.is_dir() {
            continue;
        }

        let entries: Vec<_> = std::fs::read_dir(&skills_dir)
            .ok()?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();

        // Try matching by package name, then area name, then repo name
        let target_names: Vec<&str> = [
            current_package.map(|p| p.name.as_str()),
            current_area,
            repo_root.file_name().and_then(|n| n.to_str()),
        ]
        .into_iter()
        .flatten()
        .collect();

        for target in &target_names {
            for entry in &entries {
                let dir_name = entry.file_name();
                let dir_name_str = dir_name.to_string_lossy();
                if dir_name_str == *target {
                    let skill_file = entry.path().join("SKILL.md");
                    if skill_file.exists() {
                        let relative = skill_file
                            .strip_prefix(repo_root)
                            .ok()?
                            .to_string_lossy()
                            .to_string();
                        return Some(relative);
                    }
                }
            }
        }
    }

    None
}

// ── OS context ────────────────────────────────────────────────────

fn populate_os(cap: &ContextCapture, values: &mut Map<String, Value>) {
    let os_info = cap.os_info.as_ref();

    values.insert(
        "os".into(),
        os_info.map_or(Value::Null, |info| {
            Value::String(
                match info.os_type {
                    OsType::Windows => "Windows",
                    OsType::MacOS => "macOS",
                    OsType::Linux => "Linux",
                    _ => return Value::Null,
                }
                .to_string(),
            )
        }),
    );

    values.insert(
        "os_distro".into(),
        Value::String(
            os_info
                .and_then(|info| info.distribution.clone())
                .unwrap_or_default(),
        ),
    );

    values.insert(
        "os_package_manager".into(),
        os_info
            .and_then(|info| {
                info.system_package_managers
                    .as_ref()
                    .and_then(|spm| spm.primary.as_ref())
                    .map(|pm| Value::String(format!("{pm:?}")))
            })
            .unwrap_or(Value::Null),
    );

    values.insert(
        "os_version".into(),
        Value::String(os_info.map(|info| info.version.clone()).unwrap_or_default()),
    );
}

// ── Hardware context ──────────────────────────────────────────────

fn populate_hardware(cap: &ContextCapture, values: &mut Map<String, Value>) {
    let hw = cap.hardware_info.as_ref();

    values.insert(
        "memory_total".into(),
        hw.map_or(Value::Null, |h| {
            Value::String(format::format_bytes(h.memory.total_bytes))
        }),
    );

    values.insert(
        "memory_used".into(),
        hw.map_or(Value::Null, |h| {
            (h.memory.used_bytes * 100)
                .checked_div(h.memory.total_bytes)
                .map_or_else(
                    || Value::String("0%".to_string()),
                    |pct| Value::String(format!("{pct}%")),
                )
        }),
    );

    values.insert(
        "memory_avail".into(),
        hw.map_or(Value::Null, |h| {
            Value::String(format::format_bytes(h.memory.available_bytes))
        }),
    );

    values.insert(
        "cpu_cores".into(),
        hw.map_or(Value::Null, |h| Value::Number(h.cpu.logical_cores.into())),
    );

    values.insert(
        "cpu_arch".into(),
        hw.map_or(Value::Null, |h| Value::String(h.cpu.arch.clone())),
    );

    values.insert(
        "gpu".into(),
        cap.gpu_names
            .as_ref()
            .map_or(Value::Null, |n| Value::String(n.clone())),
    );
}

#[cfg(test)]
impl ContextCapture {
    /// Minimal capture with a fixed repo root and the supplied repo info.
    fn for_test_base(repo_root: PathBuf, repo_info: Option<RepoInfo>) -> Self {
        Self {
            base_dir: repo_root.clone(),
            repo_root: Some(repo_root),
            repo_name: None,
            has_git: true,
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

    fn for_test_non_monorepo() -> Self {
        let root = PathBuf::from("/tmp/not-a-monorepo");
        let ri = test_repo_info(root.clone(), false, None);
        Self::for_test_base(root, Some(ri))
    }

    fn for_test_monorepo_with_packages(pkgs: &[(&str, &str)]) -> Self {
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

    fn for_test_monorepo_in_package(name: &str, depends_on: &[&str], used_by: &[&str]) -> Self {
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
fn test_repo_info(root: PathBuf, is_monorepo: bool, packages: Option<Vec<Package>>) -> RepoInfo {
    RepoInfo {
        is_monorepo,
        monorepo_tool: None,
        workspace_tools: Vec::new(),
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
    fn populate_datetime_produces_all_expected_keys() {
        let mut values = Map::new();
        populate_datetime(&mut values);

        // Legacy fields
        assert!(values.contains_key("now"));
        assert!(values.contains_key("now_utc"));
        assert!(values.contains_key("today"));
        assert!(values.contains_key("yesterday"));
        assert!(values.contains_key("tomorrow"));
        assert!(values.contains_key("day"));
        assert!(values.contains_key("day_abbr"));
        assert!(values.contains_key("year"));
        assert!(values.contains_key("month"));
        assert!(values.contains_key("month_name"));
        assert!(values.contains_key("month_name_abbr"));

        // New date/time fields
        assert!(values.contains_key("today_utc"));
        assert!(values.contains_key("yesterday_utc"));
        assert!(values.contains_key("tomorrow_utc"));
        assert!(values.contains_key("day_utc"));
        assert!(values.contains_key("day_abbr_utc"));
        assert!(values.contains_key("year_utc"));
        assert!(values.contains_key("day_of_month"));
        assert!(values.contains_key("day_of_month_suffixed"));
        assert!(values.contains_key("time"));
        assert!(values.contains_key("time_military"));
        assert!(values.contains_key("timezone"));
        assert!(values.contains_key("timezone_offset"));
        assert!(values.contains_key("start_of_week_sun"));
        assert!(values.contains_key("end_of_week_sun"));
        assert!(values.contains_key("start_of_week_mon"));
        assert!(values.contains_key("end_of_week_mon"));
        assert!(values.contains_key("season"));
        assert!(values.contains_key("timestamp"));
        assert!(values.contains_key("timestamp_ms"));
    }

    /// Regression: documented backward-compatible aliases (`utc`, `dow`,
    /// `dow_abbr`) must be populated by `populate_datetime` so they
    /// resolve in interpolation and in `claudine context --values` reports
    /// instead of rendering as `null`.
    #[test]
    fn populate_datetime_populates_documented_aliases() {
        let mut values = Map::new();
        populate_datetime(&mut values);

        for (alias, canonical) in [("utc", "now_utc"), ("dow", "day"), ("dow_abbr", "day_abbr")] {
            let alias_value = values
                .get(alias)
                .unwrap_or_else(|| panic!("alias `{alias}` must be present"));
            let canonical_value = values
                .get(canonical)
                .unwrap_or_else(|| panic!("canonical `{canonical}` must be present"));
            assert!(!alias_value.is_null(), "alias `{alias}` must not be null",);
            assert_eq!(
                alias_value, canonical_value,
                "alias `{alias}` must mirror canonical `{canonical}`",
            );
        }
    }

    #[test]
    fn day_of_month_suffixed_formats_correctly() {
        use super::format::ordinal_suffix;
        assert_eq!(ordinal_suffix(1), "st");
        assert_eq!(ordinal_suffix(2), "nd");
        assert_eq!(ordinal_suffix(3), "rd");
        assert_eq!(ordinal_suffix(4), "th");
        assert_eq!(ordinal_suffix(11), "th");
        assert_eq!(ordinal_suffix(12), "th");
        assert_eq!(ordinal_suffix(13), "th");
        assert_eq!(ordinal_suffix(21), "st");
        assert_eq!(ordinal_suffix(22), "nd");
        assert_eq!(ordinal_suffix(23), "rd");
        assert_eq!(ordinal_suffix(31), "st");
    }

    #[test]
    fn strip_trailing_sep_removes_single_separator() {
        assert_eq!(strip_trailing_sep("/repo/"), "/repo");
        assert_eq!(strip_trailing_sep("/repo"), "/repo");
        assert_eq!(strip_trailing_sep("C:\\repo\\"), "C:\\repo");
        assert_eq!(strip_trailing_sep(""), "");
    }

    #[test]
    fn repo_root_has_no_trailing_slash() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let (values, _, _) = capture_runtime_context_for_groups(root, &[ContextGroup::Repo]);
        if let Some(Value::String(rr)) = values.get("repo_root") {
            assert!(
                !rr.ends_with('/'),
                "repo_root must not end with '/': got {rr:?}"
            );
            assert!(!rr.is_empty(), "repo_root should be non-empty in a repo");
        }
    }

    #[test]
    fn populate_datetime_includes_utc_time_variants() {
        let mut values = Map::new();
        populate_datetime(&mut values);
        let tu = values.get("time_utc").and_then(Value::as_str).unwrap_or("");
        let tmu = values
            .get("time_military_utc")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            tu.ends_with(" (UTC)"),
            "time_utc must end with ' (UTC)': {tu:?}"
        );
        assert!(
            tmu.ends_with(" (UTC)"),
            "time_military_utc must end with ' (UTC)': {tmu:?}"
        );
    }

    #[test]
    fn area_vars_empty_when_not_monorepo() {
        let cap = ContextCapture::for_test_non_monorepo();
        let mut values = Map::new();
        populate_repo(&cap, &mut values);
        assert_eq!(values.get("area"), Some(&Value::String(String::new())));
        assert_eq!(
            values.get("area_description"),
            Some(&Value::String(String::new()))
        );
        // area_root falls back to repo root (no trailing slash) when not a monorepo.
        if let Some(Value::String(ar)) = values.get("area_root") {
            assert!(
                !ar.ends_with('/'),
                "area_root must not end with '/': {ar:?}"
            );
        }
    }

    #[test]
    fn current_packages_lists_packages_under_cwd_as_markdown() {
        let cap = ContextCapture::for_test_monorepo_with_packages(&[
            ("alpha", "alpha/lib"),
            ("alpha-cli", "alpha/cli"),
        ]);
        let mut values = Map::new();
        populate_repo(&cap, &mut values);
        let listing = values
            .get("current_packages")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(listing.contains("- alpha (alpha/lib)"), "got: {listing}");
        assert!(
            listing.contains("- alpha-cli (alpha/cli)"),
            "got: {listing}"
        );
    }

    #[test]
    fn depends_on_renders_nested_list_scoped_to_area() {
        let cap = ContextCapture::for_test_monorepo_in_package("alpha", &["beta", "gamma"], &[]);
        let mut values = Map::new();
        populate_repo(&cap, &mut values);
        let s = values
            .get("depends_on")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(s.contains("'alpha' depends on:"), "got: {s}");
        assert!(s.contains("    - beta"), "got: {s}");
        assert!(s.contains("    - gamma"), "got: {s}");
    }

    #[test]
    fn used_by_renders_empty_message_when_no_dependents() {
        let cap = ContextCapture::for_test_monorepo_in_package("alpha", &[], &[]);
        let mut values = Map::new();
        populate_repo(&cap, &mut values);
        let s = values.get("used_by").and_then(Value::as_str).unwrap_or("");
        assert!(
            s.contains("'alpha' is not used by other packages in this monorepo"),
            "got: {s}"
        );
    }

    #[test]
    fn season_determination() {
        use super::format::determine_season;
        // Meteorological seasons
        assert_eq!(determine_season(1, 15), "Winter");
        assert_eq!(determine_season(3, 1), "Spring");
        assert_eq!(determine_season(6, 1), "Summer");
        assert_eq!(determine_season(9, 1), "Fall");
        assert_eq!(determine_season(12, 15), "Winter");
    }
}
