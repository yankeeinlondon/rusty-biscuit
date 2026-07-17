use chrono::{DateTime, Duration, Local, NaiveDate, Utc};
use gix::bstr::ByteSlice;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tracing::debug;

use crate::filesystem::git::ConventionalCommit;
use crate::filesystem::git::discovery::DeltaKind;
use crate::filesystem::path_kind::{is_documentation_path, is_source_code_path};
use crate::filesystem::repo::{Package, RepoInfo, detect_repo};
use crate::performance::{self, counters};
use crate::{Result, SniffError};

// ---------------------------------------------------------------------------
// Period parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum PeriodSpecifier {
    Duration(Duration),
    Date(NaiveDate),
    Hash(String),
    Count(usize),
    Today,
    Yesterday,
}

pub fn parse_period(input: &str) -> Result<PeriodSpecifier> {
    let trimmed = input.trim();
    let lower = trimmed.to_ascii_lowercase();

    if lower == "today" {
        return Ok(PeriodSpecifier::Today);
    }
    if lower == "yesterday" {
        return Ok(PeriodSpecifier::Yesterday);
    }

    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return Ok(PeriodSpecifier::Date(date));
    }

    // A bare positive decimal integer means "the last N commits".
    // This check runs before hash detection so that all-digit inputs
    // are always interpreted as a count, not a (rare) all-decimal SHA.
    if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(count) = trimmed.parse::<usize>()
            && count > 0
        {
            return Ok(PeriodSpecifier::Count(count));
        }
        return Err(SniffError::InvalidPeriod(trimmed.to_string()));
    }

    if let Some(spec) = parse_duration(&lower) {
        return Ok(PeriodSpecifier::Duration(spec));
    }

    if trimmed.len() >= 7 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(PeriodSpecifier::Hash(trimmed.to_string()));
    }

    Err(SniffError::InvalidPeriod(trimmed.to_string()))
}

fn parse_duration(input: &str) -> Option<Duration> {
    let input = input.trim();

    let (num_str, unit): (&str, &str) = if let Some((n, u)) = split_number_unit(input) {
        (n, u)
    } else if let Some(pos) = input.find(char::is_alphabetic) {
        let (n, u) = input.split_at(pos);
        (n, u)
    } else {
        return None;
    };

    let count: i64 = num_str.trim().parse().ok()?;
    if count <= 0 {
        return None;
    }

    let unit = unit.trim();

    match unit {
        "h" | "hour" | "hours" => Some(Duration::hours(count)),
        "d" | "day" | "days" => Some(Duration::days(count)),
        "w" | "wk" | "week" | "weeks" => Some(Duration::weeks(count)),
        "mo" | "m" | "month" | "months" => Some(Duration::days(count * 30)),
        "y" | "yr" | "year" | "years" => Some(Duration::days(count * 365)),
        _ => None,
    }
}

fn split_number_unit(input: &str) -> Option<(&str, &str)> {
    let input = input.trim();
    let pos = input
        .find(char::is_whitespace)
        .or_else(|| input.find(char::is_alphabetic))?;
    let num_part = &input[..pos];
    let unit_part = input[pos..].trim_start();
    if unit_part.is_empty() || num_part.is_empty() {
        return None;
    }
    Some((num_part, unit_part))
}

// ---------------------------------------------------------------------------
// Commit message parsing
// ---------------------------------------------------------------------------

pub fn parse_commit_message(message: &str) -> (String, Vec<String>) {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return (String::new(), Vec::new());
    }

    let paragraphs: Vec<&str> = trimmed.split("\n\n").collect();
    let mut description = String::new();
    let mut bullet_points = Vec::new();

    let mut first_para_consumed = false;
    for para in &paragraphs {
        let lines: Vec<&str> = para.lines().collect();
        let mut para_description_parts = Vec::new();

        for line in &lines {
            let stripped = line.trim();
            if let Some(bullet) = stripped
                .strip_prefix("- ")
                .or_else(|| stripped.strip_prefix("* "))
            {
                bullet_points.push(bullet.to_string());
            } else if !stripped.is_empty() {
                para_description_parts.push(stripped.to_string());
            }
        }

        if !para_description_parts.is_empty() {
            if !first_para_consumed || description.is_empty() {
                description = para_description_parts.join(" ");
                first_para_consumed = true;
            } else {
                description.push(' ');
                description.push_str(&para_description_parts.join(" "));
            }
        } else if !bullet_points.is_empty() && !first_para_consumed {
            first_para_consumed = true;
        } else if bullet_points.is_empty()
            && para_description_parts.is_empty()
            && lines.iter().all(|l| l.trim().is_empty())
        {
            // blank paragraph separator — just continue
        }
    }

    (description, bullet_points)
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A file touched by a commit together with the kind of change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitFileChange {
    /// Path relative to the repository root.
    pub path: String,
    /// How the file was changed (added, modified, deleted, renamed, copied).
    pub kind: DeltaKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDesc {
    pub hash: String,
    pub datetime: String,
    pub packages: Option<Vec<String>>,
    pub package_areas: Option<Vec<String>>,
    pub files: Vec<CommitFileChange>,
    pub description: String,
    pub bullet_points: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDescSet {
    pub commits: Vec<CommitDesc>,
    pub period_label: String,
    pub repo_root: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packages: Option<Vec<Package>>,
}

// ---------------------------------------------------------------------------
// Query functions
// ---------------------------------------------------------------------------

/// [`get_recent_commits_by_duration`] against an already-observed repository.
///
/// Takes both the discovered handle and the `RepoInfo` the caller has already
/// detected, so neither the repository discovery nor the [`detect_repo`] pass
/// that `get_recent_commits_by_duration` performs internally is repeated. Pass
/// `repo_info: None` only when the caller genuinely has no package catalog —
/// commits then carry no package attribution, exactly as for a non-monorepo.
pub fn get_recent_commits_by_duration_with_repo(
    repo: &super::GitRepo,
    duration: Duration,
    period_label: &str,
    repo_info: Option<&RepoInfo>,
) -> Result<CommitDescSet> {
    let until = Utc::now();
    let since = until - duration;
    let commits =
        repo.with_cached_gix(|gix| collect_commits_in_range(gix, since, until, repo_info))?;

    Ok(CommitDescSet {
        commits,
        period_label: period_label.to_string(),
        repo_root: repo.repo_root().to_path_buf(),
        packages: repo_info.and_then(|ri| ri.packages.clone()),
    })
}

pub fn get_recent_commits_by_duration(
    base_dir: &Path,
    duration: Duration,
    period_label: &str,
) -> Result<CommitDescSet> {
    let mut repo = super::open::trusted_discover(base_dir)?
        .ok_or_else(|| SniffError::NotARepository(base_dir.to_path_buf()))?;
    super::open::configure_cache(&mut repo);
    let repo_root = repo
        .workdir()
        .map(Path::to_path_buf)
        .ok_or_else(|| SniffError::NotARepository(base_dir.to_path_buf()))?;

    let until = Utc::now();
    let since = until - duration;
    let repo_info = detect_repo(&repo_root)?;

    let commits = collect_commits_in_range(&repo, since, until, repo_info.as_ref())?;
    let packages = repo_info.as_ref().and_then(|ri| ri.packages.clone());

    Ok(CommitDescSet {
        commits,
        period_label: period_label.to_string(),
        repo_root,
        packages,
    })
}

pub fn get_recent_commits_in_range(
    base_dir: &Path,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
    period_label: &str,
) -> Result<CommitDescSet> {
    let mut repo = super::open::trusted_discover(base_dir)?
        .ok_or_else(|| SniffError::NotARepository(base_dir.to_path_buf()))?;
    super::open::configure_cache(&mut repo);
    let repo_root = repo
        .workdir()
        .map(Path::to_path_buf)
        .ok_or_else(|| SniffError::NotARepository(base_dir.to_path_buf()))?;

    let repo_info = detect_repo(&repo_root)?;

    let commits = collect_commits_in_range(&repo, since, until, repo_info.as_ref())?;
    let packages = repo_info.as_ref().and_then(|ri| ri.packages.clone());

    Ok(CommitDescSet {
        commits,
        period_label: period_label.to_string(),
        repo_root,
        packages,
    })
}

pub fn get_recent_commits_by_date(base_dir: &Path, date: NaiveDate) -> Result<CommitDescSet> {
    let mut repo = super::open::trusted_discover(base_dir)?
        .ok_or_else(|| SniffError::NotARepository(base_dir.to_path_buf()))?;
    super::open::configure_cache(&mut repo);
    let repo_root = repo
        .workdir()
        .map(Path::to_path_buf)
        .ok_or_else(|| SniffError::NotARepository(base_dir.to_path_buf()))?;

    let since = date.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc();
    let until = Utc::now();
    let repo_info = detect_repo(&repo_root)?;

    let commits = collect_commits_in_range(&repo, since, until, repo_info.as_ref())?;
    let packages = repo_info.as_ref().and_then(|ri| ri.packages.clone());
    let period_label = format!("since {}", date);

    Ok(CommitDescSet {
        commits,
        period_label,
        repo_root,
        packages,
    })
}

pub fn get_recent_commits_by_hash(base_dir: &Path, hash: &str) -> Result<CommitDescSet> {
    let mut repo = super::open::trusted_discover(base_dir)?
        .ok_or_else(|| SniffError::NotARepository(base_dir.to_path_buf()))?;
    super::open::configure_cache(&mut repo);
    let repo_root = repo
        .workdir()
        .map(Path::to_path_buf)
        .ok_or_else(|| SniffError::NotARepository(base_dir.to_path_buf()))?;

    let target = repo.rev_parse_single(hash).map_err(|e| {
        debug!(hash = hash, error = %e, "could not resolve hash");
        SniffError::git("revparse", e)
    })?;
    let target_oid = target.detach();

    let head = repo.head_id().map_err(|e| SniffError::git("head", e))?;
    let head_oid = head.detach();

    if target_oid != head_oid {
        let reachable = repo
            .merge_base(head_oid, target_oid)
            .ok()
            .map(|base| base.detach() == target_oid)
            .unwrap_or(false);
        if !reachable {
            return Err(SniffError::HashNotReachable {
                hash: hash.to_string(),
            });
        }
    }

    let repo_info = detect_repo(&repo_root)?;

    let commits = collect_commits_from_hash_to_head(&repo, target_oid, repo_info.as_ref())?;
    let packages = repo_info.as_ref().and_then(|ri| ri.packages.clone());
    let short_hash = &hash[..hash.len().min(8)];
    let period_label = format!("since commit {}", short_hash);

    Ok(CommitDescSet {
        commits,
        period_label,
        repo_root,
        packages,
    })
}

/// Return the most recent `count` commits reachable from `HEAD`, newest-first.
///
/// ## Errors
///
/// Returns `SniffError::NotARepository` if `base_dir` is not inside a git
/// repository, or `SniffError::InvalidPeriod` if `count` is zero.
pub fn get_recent_commits_by_count(base_dir: &Path, count: usize) -> Result<CommitDescSet> {
    if count == 0 {
        return Err(SniffError::InvalidPeriod("0".to_string()));
    }

    let mut repo = super::open::trusted_discover(base_dir)?
        .ok_or_else(|| SniffError::NotARepository(base_dir.to_path_buf()))?;
    super::open::configure_cache(&mut repo);
    let repo_root = repo
        .workdir()
        .map(Path::to_path_buf)
        .ok_or_else(|| SniffError::NotARepository(base_dir.to_path_buf()))?;

    let repo_info = detect_repo(&repo_root)?;

    let commits = collect_commits_by_count(&repo, count, repo_info.as_ref())?;
    let packages = repo_info.as_ref().and_then(|ri| ri.packages.clone());
    let period_label = format!("last {} commits", count);

    Ok(CommitDescSet {
        commits,
        period_label,
        repo_root,
        packages,
    })
}

// ---------------------------------------------------------------------------
// Internal commit walker
// ---------------------------------------------------------------------------

fn collect_commits_in_range(
    repo: &gix::Repository,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
    repo_info_opt: Option<&crate::filesystem::repo::RepoInfo>,
) -> Result<Vec<CommitDesc>> {
    let mut commits = Vec::new();

    let Some(head) = super::discovery::head_id_opt(repo)? else {
        return Ok(commits);
    };

    let walk = repo
        .rev_walk(Some(head))
        .sorting(gix::revision::walk::Sorting::ByCommitTime(
            gix::traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .use_commit_graph(Some(true))
        .all()
        .map_err(|e| SniffError::git("revwalk", e))?;

    let mut diff_cache = repo
        .diff_resource_cache_for_tree_diff()
        .map_err(|e| SniffError::git("diff", e))?;

    let packages = repo_info_opt.and_then(|ri| ri.packages.as_ref());
    let is_monorepo = repo_info_opt.is_some_and(|ri| ri.is_monorepo);

    for info_result in walk {
        let info = info_result.map_err(|e| SniffError::git("revwalk", e))?;
        performance::increment_counter(counters::GIT_COMMIT_VISITS, 1);

        // Git commit times are whole seconds; treat the commit as occurring at
        // the start of its second and compare against the full-precision
        // `since`/`until` instants. Truncating the bounds to seconds instead
        // would make a 0-width window (`Duration::days(0)`, since == until ==
        // now) wrongly include a commit from the current second.
        let commit_time = DateTime::from_timestamp(info.commit_time(), 0).unwrap_or_default();
        // `ByCommitTime` is a lazy frontier walk: a tip is always yielded
        // before its parents, so an old HEAD can precede a newer ancestor.
        // Filter out-of-range commits with `continue` rather than `break` —
        // breaking on the first old commit would hide newer parents behind a
        // skewed HEAD (matching the git2 baseline's full-ancestry scan).
        if commit_time < since || commit_time > until {
            continue;
        }

        let sha = info.id.to_string();
        let files_raw =
            super::discovery::get_commit_files_with_cache_fallible(repo, info.id, &mut diff_cache)?;
        let files: Vec<CommitFileChange> = files_raw
            .iter()
            .map(|(p, k)| CommitFileChange {
                path: p.to_string_lossy().to_string(),
                kind: *k,
            })
            .collect();

        let commit = info.object().map_err(|e| SniffError::git("object", e))?;
        let message_raw = commit
            .message_raw()
            .map_err(|e| SniffError::git("message", e))?;
        let message = String::from_utf8_lossy(message_raw.trim());
        let (description, bullet_points) = parse_commit_message(&message);

        let (commit_packages, commit_package_areas) = if is_monorepo {
            if let Some(pkgs) = packages {
                let mut pkg_set: BTreeSet<String> = BTreeSet::new();
                let mut area_set: BTreeSet<String> = BTreeSet::new();

                for file_path in &files_raw {
                    let file_path_buf = &file_path.0;
                    for pkg in pkgs.iter() {
                        if file_path_buf.starts_with(&pkg.relative) {
                            pkg_set.insert(pkg.name.clone());
                            area_set.insert(pkg.package_area.clone());
                        }
                    }
                }

                (
                    Some(pkg_set.into_iter().collect()),
                    Some(area_set.into_iter().collect()),
                )
            } else {
                (Some(vec![]), Some(vec![]))
            }
        } else {
            (None, None)
        };

        commits.push(CommitDesc {
            hash: sha,
            datetime: commit_time.to_rfc3339(),
            packages: commit_packages,
            package_areas: commit_package_areas,
            files,
            description,
            bullet_points,
        });
    }

    Ok(commits)
}

/// Walk from HEAD down to (and including) `target_oid`, returning commits in
/// newest-first order. The caller must have already validated that `target_oid`
/// is an ancestor of HEAD.
fn collect_commits_from_hash_to_head(
    repo: &gix::Repository,
    target_oid: gix::ObjectId,
    repo_info_opt: Option<&crate::filesystem::repo::RepoInfo>,
) -> Result<Vec<CommitDesc>> {
    let mut commits = Vec::new();

    let Some(head) = super::discovery::head_id_opt(repo)? else {
        return Ok(commits);
    };

    let walk = repo
        .rev_walk(Some(head))
        .sorting(gix::revision::walk::Sorting::ByCommitTime(
            gix::traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .use_commit_graph(Some(true))
        .all()
        .map_err(|e| SniffError::git("revwalk", e))?;

    let mut diff_cache = repo
        .diff_resource_cache_for_tree_diff()
        .map_err(|e| SniffError::git("diff", e))?;

    let packages = repo_info_opt.and_then(|ri| ri.packages.as_ref());
    let is_monorepo = repo_info_opt.is_some_and(|ri| ri.is_monorepo);

    for info_result in walk {
        let info = info_result.map_err(|e| SniffError::git("revwalk", e))?;
        performance::increment_counter(counters::GIT_COMMIT_VISITS, 1);

        let commit_time = DateTime::from_timestamp(info.commit_time(), 0).unwrap_or_default();

        let sha = info.id.to_string();
        let files_raw =
            super::discovery::get_commit_files_with_cache_fallible(repo, info.id, &mut diff_cache)?;
        let files: Vec<CommitFileChange> = files_raw
            .iter()
            .map(|(p, k)| CommitFileChange {
                path: p.to_string_lossy().to_string(),
                kind: *k,
            })
            .collect();

        let commit = info.object().map_err(|e| SniffError::git("object", e))?;
        let message_raw = commit
            .message_raw()
            .map_err(|e| SniffError::git("message", e))?;
        let message = String::from_utf8_lossy(message_raw.trim());
        let (description, bullet_points) = parse_commit_message(&message);

        let (commit_packages, commit_package_areas) = if is_monorepo {
            if let Some(pkgs) = packages {
                let mut pkg_set: BTreeSet<String> = BTreeSet::new();
                let mut area_set: BTreeSet<String> = BTreeSet::new();

                for file_path in &files_raw {
                    let file_path_buf = &file_path.0;
                    for pkg in pkgs.iter() {
                        if file_path_buf.starts_with(&pkg.relative) {
                            pkg_set.insert(pkg.name.clone());
                            area_set.insert(pkg.package_area.clone());
                        }
                    }
                }

                (
                    Some(pkg_set.into_iter().collect()),
                    Some(area_set.into_iter().collect()),
                )
            } else {
                (Some(vec![]), Some(vec![]))
            }
        } else {
            (None, None)
        };

        commits.push(CommitDesc {
            hash: sha,
            datetime: commit_time.to_rfc3339(),
            packages: commit_packages,
            package_areas: commit_package_areas,
            files,
            description,
            bullet_points,
        });

        // Stop after including the target commit
        if info.id == target_oid {
            break;
        }
    }

    Ok(commits)
}

/// Walk HEAD newest-first and emit at most `count` commits.
fn collect_commits_by_count(
    repo: &gix::Repository,
    count: usize,
    repo_info_opt: Option<&crate::filesystem::repo::RepoInfo>,
) -> Result<Vec<CommitDesc>> {
    let mut commits = Vec::new();

    let Some(head) = super::discovery::head_id_opt(repo)? else {
        return Ok(commits);
    };

    let walk = repo
        .rev_walk(Some(head))
        .sorting(gix::revision::walk::Sorting::ByCommitTime(
            gix::traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .use_commit_graph(Some(true))
        .all()
        .map_err(|e| SniffError::git("revwalk", e))?;

    let mut diff_cache = repo
        .diff_resource_cache_for_tree_diff()
        .map_err(|e| SniffError::git("diff", e))?;

    let packages = repo_info_opt.and_then(|ri| ri.packages.as_ref());
    let is_monorepo = repo_info_opt.is_some_and(|ri| ri.is_monorepo);

    for info_result in walk {
        if commits.len() >= count {
            break;
        }
        let info = info_result.map_err(|e| SniffError::git("revwalk", e))?;
        performance::increment_counter(counters::GIT_COMMIT_VISITS, 1);

        let commit_time = DateTime::from_timestamp(info.commit_time(), 0).unwrap_or_default();

        let sha = info.id.to_string();
        let files_raw =
            super::discovery::get_commit_files_with_cache_fallible(repo, info.id, &mut diff_cache)?;
        let files: Vec<CommitFileChange> = files_raw
            .iter()
            .map(|(p, k)| CommitFileChange {
                path: p.to_string_lossy().to_string(),
                kind: *k,
            })
            .collect();

        let commit = info.object().map_err(|e| SniffError::git("object", e))?;
        let message_raw = commit
            .message_raw()
            .map_err(|e| SniffError::git("message", e))?;
        let message = String::from_utf8_lossy(message_raw.trim());
        let (description, bullet_points) = parse_commit_message(&message);

        let (commit_packages, commit_package_areas) = if is_monorepo {
            if let Some(pkgs) = packages {
                let mut pkg_set: BTreeSet<String> = BTreeSet::new();
                let mut area_set: BTreeSet<String> = BTreeSet::new();

                for file_path in &files_raw {
                    let file_path_buf = &file_path.0;
                    for pkg in pkgs.iter() {
                        if file_path_buf.starts_with(&pkg.relative) {
                            pkg_set.insert(pkg.name.clone());
                            area_set.insert(pkg.package_area.clone());
                        }
                    }
                }

                (
                    Some(pkg_set.into_iter().collect()),
                    Some(area_set.into_iter().collect()),
                )
            } else {
                (Some(vec![]), Some(vec![]))
            }
        } else {
            (None, None)
        };

        commits.push(CommitDesc {
            hash: sha,
            datetime: commit_time.to_rfc3339(),
            packages: commit_packages,
            package_areas: commit_package_areas,
            files,
            description,
            bullet_points,
        });
    }

    Ok(commits)
}

// ---------------------------------------------------------------------------
// Rendering — Markdown methods
// ---------------------------------------------------------------------------

impl CommitDescSet {
    /// Render every commit in the set using the commit-centric layout.
    pub fn describe(&self, plain: bool) -> String {
        self.render_commit_centric(None, plain)
    }

    /// Render only commits that touched at least one source-code file,
    /// and list only the matching files under "Files Impacted".
    pub fn source_code_changes(&self, plain: bool) -> String {
        self.render_commit_centric(Some(ChangeKind::SourceCode), plain)
    }

    /// Render only commits that touched at least one documentation file,
    /// and list only the matching files under "Files Impacted".
    pub fn documentation_changes(&self, plain: bool) -> String {
        self.render_commit_centric(Some(ChangeKind::Documentation), plain)
    }

    fn render_commit_centric(&self, filter: Option<ChangeKind>, plain: bool) -> String {
        let mut out = String::new();

        if let Some(kind) = filter {
            let title = match kind {
                ChangeKind::SourceCode => "Source Code Changes",
                ChangeKind::Documentation => "Documentation Changes",
            };
            out.push_str(&format!("### {} (_{}_)\n\n", title, self.period_label));
        }

        let today = Local::now().date_naive();
        let mut emitted = 0usize;

        for commit in &self.commits {
            let files: Vec<&CommitFileChange> = match filter {
                None => commit.files.iter().collect(),
                Some(kind) => commit
                    .files
                    .iter()
                    .filter(|f| file_matches_kind(&f.path, kind))
                    .collect(),
            };

            if files.is_empty() {
                continue;
            }

            render_commit_block(&mut out, commit, &files, &self.repo_root, today, plain);
            emitted += 1;
        }

        if filter.is_some() && emitted == 0 {
            return String::new();
        }

        out
    }
}

#[derive(Debug, Clone, Copy)]
enum ChangeKind {
    SourceCode,
    Documentation,
}

fn file_matches_kind(path: &str, kind: ChangeKind) -> bool {
    let p = PathBuf::from(path);
    match kind {
        ChangeKind::SourceCode => is_source_code_path(&p),
        ChangeKind::Documentation => is_documentation_path(&p),
    }
}

/// Append one commit's block to `out` in the commit-centric layout:
///
/// ```text
/// - [shorthash] type(scope) at 1:01pm Today: description
///
///     **Description:**
///
///     - bullet 1
///
///     **Files Impacted:**
///
///     - modified: [path](file://...)
/// ```
fn render_commit_block(
    out: &mut String,
    commit: &CommitDesc,
    files: &[&CommitFileChange],
    repo_root: &Path,
    today: NaiveDate,
    plain: bool,
) {
    let short_hash = &commit.hash[..commit.hash.len().min(7)];
    let time_label = commit_time_label(&commit.datetime, today);
    let (prefix, message) = split_conventional(&commit.description);

    let header = match (prefix, message) {
        (Some(pfx), msg) => format!(
            "- [{hash}] {prefix} at {time}: {msg}\n",
            hash = short_hash,
            prefix = pfx,
            time = time_label,
            msg = msg,
        ),
        (None, msg) => format!(
            "- [{hash}] at {time}: {msg}\n",
            hash = short_hash,
            time = time_label,
            msg = msg,
        ),
    };
    out.push_str(&header);
    out.push('\n');

    if !commit.bullet_points.is_empty() {
        out.push_str("    **Description:**\n\n");
        for bp in &commit.bullet_points {
            out.push_str(&format!("    - {}\n", bp));
        }
        out.push('\n');
    }

    out.push_str("    **Files Impacted:**\n\n");
    for file in files {
        let label = file.kind.to_string();
        if plain {
            out.push_str(&format!("    - {}: {}\n", label, file.path));
        } else {
            let url = file_url(&repo_root.join(&file.path));
            out.push_str(&format!("    - {}: [{}]({})\n", label, file.path, url));
        }
    }
    out.push('\n');
}

/// Split a commit description like `refactor(sniff): improve X` into
/// `(Some("refactor(sniff)"), "improve X")`. If the description is not in
/// conventional-commit form, returns `(None, <description>)`.
fn split_conventional(description: &str) -> (Option<String>, String) {
    let parsed = ConventionalCommit::parse(description);
    match parsed.operation {
        Some(op) => {
            let prefix = match parsed.scope {
                Some(scope) => format!("{}({})", op, scope),
                None => op,
            };
            (Some(prefix), parsed.description)
        }
        None => (None, description.to_string()),
    }
}

/// Format a commit timestamp like `1:01pm Today` / `12:32pm Yesterday` /
/// `2026-04-01 at 9:30am`, converted to the viewer's local timezone so that
/// "Today"/"Yesterday" labels line up with what the reader expects.
fn commit_time_label(datetime_rfc3339: &str, today_local: NaiveDate) -> String {
    let Ok(dt) = DateTime::parse_from_rfc3339(datetime_rfc3339) else {
        return datetime_rfc3339.to_string();
    };

    let local = dt.with_timezone(&Local);
    let time_str = local.format("%-I:%M%P").to_string();
    let commit_date = local.date_naive();
    let yesterday = today_local - Duration::days(1);

    if commit_date == today_local {
        format!("{} Today", time_str)
    } else if commit_date == yesterday {
        format!("{} Yesterday", time_str)
    } else {
        format!("{} at {}", commit_date.format("%Y-%m-%d"), time_str)
    }
}

/// Convert an absolute file path to a proper `file://` URI.
///
/// Uses `url::Url::from_file_path` for correct cross-platform encoding
/// (Windows drive letters, spaces, reserved characters).
fn file_url(path: &Path) -> String {
    url::Url::from_file_path(path)
        .map(|u| u.to_string())
        .unwrap_or_else(|()| format!("file://{}", path.to_string_lossy()))
}

// ---------------------------------------------------------------------------
// Filtering
// ---------------------------------------------------------------------------

impl CommitDescSet {
    pub fn filter_by_actions<I, S>(&mut self, actions: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let allowed: BTreeSet<String> = actions
            .into_iter()
            .map(|action| action.as_ref().to_ascii_lowercase())
            .collect();

        if allowed.is_empty() {
            return;
        }

        self.commits.retain(|commit| {
            ConventionalCommit::parse(&commit.description)
                .operation
                .is_some_and(|operation| allowed.contains(&operation.to_ascii_lowercase()))
        });
    }

    pub fn filter_by_package(&mut self, package_name: &str) -> Result<()> {
        let Some(ref packages) = self.packages else {
            return Err(SniffError::NotAMonorepo(self.repo_root.clone()));
        };

        let package_lower = package_name.to_ascii_lowercase();
        let pkg = packages
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(&package_lower));
        let Some(pkg) = pkg else {
            let valid: Vec<String> = packages.iter().map(|p| p.name.clone()).collect();
            return Err(SniffError::UnknownPackage {
                name: package_name.to_string(),
                valid: valid.join(", "),
            });
        };

        let pkg_relative = PathBuf::from(&pkg.relative);

        for commit in &mut self.commits {
            commit
                .files
                .retain(|f| PathBuf::from(&f.path).starts_with(&pkg_relative));

            if commit.files.is_empty() {
                continue;
            }

            let mut new_pkgs = BTreeSet::new();
            let mut new_areas = BTreeSet::new();
            for file in &commit.files {
                let path = PathBuf::from(&file.path);
                for p in packages.iter() {
                    if path.starts_with(PathBuf::from(&p.relative)) {
                        new_pkgs.insert(p.name.clone());
                        new_areas.insert(p.package_area.clone());
                    }
                }
            }
            commit.packages = Some(new_pkgs.into_iter().collect());
            commit.package_areas = Some(new_areas.into_iter().collect());
        }

        self.commits.retain(|c| !c.files.is_empty());

        // Rewrite top-level packages to only include the matched package
        if let Some(ref mut pkgs) = self.packages {
            pkgs.retain(|p| p.name.eq_ignore_ascii_case(&package_lower));
        }

        Ok(())
    }

    pub fn filter_by_package_area(&mut self, area_name: &str) -> Result<()> {
        let Some(ref packages) = self.packages else {
            return Err(SniffError::NotAMonorepo(self.repo_root.clone()));
        };

        let area_lower = area_name.to_ascii_lowercase();
        let matching_packages: Vec<&Package> = packages
            .iter()
            .filter(|p| {
                let pkg_area = p.package_area.to_ascii_lowercase();
                pkg_area == area_lower || pkg_area.starts_with(&format!("{area_lower}/"))
            })
            .collect();

        if matching_packages.is_empty() {
            let valid: Vec<String> = packages
                .iter()
                .map(|p| p.package_area.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            return Err(SniffError::UnknownPackageArea {
                area: area_name.to_string(),
                valid: valid.join(", "),
            });
        }

        let package_roots: Vec<PathBuf> = matching_packages
            .iter()
            .map(|p| PathBuf::from(&p.relative))
            .collect();

        for commit in &mut self.commits {
            commit.files.retain(|f| {
                let path = PathBuf::from(&f.path);
                package_roots.iter().any(|root| path.starts_with(root))
            });

            if commit.files.is_empty() {
                continue;
            }

            let mut new_pkgs = BTreeSet::new();
            let mut new_areas = BTreeSet::new();
            for file in &commit.files {
                let path = PathBuf::from(&file.path);
                for p in packages.iter() {
                    if path.starts_with(PathBuf::from(&p.relative)) {
                        new_pkgs.insert(p.name.clone());
                        new_areas.insert(p.package_area.clone());
                    }
                }
            }
            commit.packages = Some(new_pkgs.into_iter().collect());
            commit.package_areas = Some(new_areas.into_iter().collect());
        }

        self.commits.retain(|c| !c.files.is_empty());

        // Rewrite top-level packages to only include those in the matched area
        let matching_names: BTreeSet<String> =
            matching_packages.iter().map(|p| p.name.clone()).collect();
        if let Some(ref mut pkgs) = self.packages {
            pkgs.retain(|p| matching_names.contains(&p.name));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_period_tests {
        use super::*;

        #[test]
        fn today_keyword() {
            assert_eq!(parse_period("today").unwrap(), PeriodSpecifier::Today);
        }

        #[test]
        fn yesterday_keyword() {
            assert_eq!(
                parse_period("yesterday").unwrap(),
                PeriodSpecifier::Yesterday
            );
        }

        #[test]
        fn today_case_insensitive() {
            assert_eq!(parse_period("Today").unwrap(), PeriodSpecifier::Today);
            assert_eq!(parse_period("TODAY").unwrap(), PeriodSpecifier::Today);
        }

        #[test]
        fn iso_date() {
            let result = parse_period("2025-12-04").unwrap();
            assert_eq!(
                result,
                PeriodSpecifier::Date(NaiveDate::from_ymd_opt(2025, 12, 4).unwrap())
            );
        }

        #[test]
        fn duration_days() {
            let result = parse_period("3d").unwrap();
            assert_eq!(result, PeriodSpecifier::Duration(Duration::days(3)));
        }

        #[test]
        fn duration_days_long() {
            let result = parse_period("3 days").unwrap();
            assert_eq!(result, PeriodSpecifier::Duration(Duration::days(3)));
        }

        #[test]
        fn duration_hours() {
            let result = parse_period("6h").unwrap();
            assert_eq!(result, PeriodSpecifier::Duration(Duration::hours(6)));
        }

        #[test]
        fn duration_weeks() {
            let result = parse_period("1w").unwrap();
            assert_eq!(result, PeriodSpecifier::Duration(Duration::weeks(1)));
        }

        #[test]
        fn duration_weeks_long() {
            let result = parse_period("1 week").unwrap();
            assert_eq!(result, PeriodSpecifier::Duration(Duration::weeks(1)));
        }

        #[test]
        fn duration_months_short() {
            let result = parse_period("3m").unwrap();
            assert_eq!(result, PeriodSpecifier::Duration(Duration::days(90)));
        }

        #[test]
        fn duration_months_long() {
            let result = parse_period("3 months").unwrap();
            assert_eq!(result, PeriodSpecifier::Duration(Duration::days(90)));
        }

        #[test]
        fn duration_months_mo() {
            let result = parse_period("3mo").unwrap();
            assert_eq!(result, PeriodSpecifier::Duration(Duration::days(90)));
        }

        #[test]
        fn duration_years() {
            let result = parse_period("1y").unwrap();
            assert_eq!(result, PeriodSpecifier::Duration(Duration::days(365)));
        }

        #[test]
        fn hash_short() {
            let result = parse_period("a1b2c3d").unwrap();
            assert_eq!(result, PeriodSpecifier::Hash("a1b2c3d".to_string()));
        }

        #[test]
        fn hash_long() {
            let result = parse_period("a1b2c3d4e5f6").unwrap();
            assert_eq!(result, PeriodSpecifier::Hash("a1b2c3d4e5f6".to_string()));
        }

        #[test]
        fn invalid_input() {
            assert!(parse_period("foo").is_err());
        }

        #[test]
        fn invalid_unit() {
            assert!(parse_period("3x").is_err());
        }

        #[test]
        fn bare_number_is_count() {
            assert_eq!(parse_period("10").unwrap(), PeriodSpecifier::Count(10));
            assert_eq!(parse_period("123").unwrap(), PeriodSpecifier::Count(123));
            assert_eq!(parse_period("1").unwrap(), PeriodSpecifier::Count(1));
        }

        #[test]
        fn bare_zero_invalid() {
            assert!(parse_period("0").is_err());
        }

        #[test]
        fn bare_number_beats_hash_lookalike() {
            // A 7+ digit numeric string should be parsed as a count, not a hash.
            assert_eq!(
                parse_period("1234567").unwrap(),
                PeriodSpecifier::Count(1234567)
            );
        }

        #[test]
        fn hash_too_short() {
            assert!(parse_period("a1b2c3").is_err());
        }
    }

    mod parse_commit_message_tests {
        use super::*;

        #[test]
        fn simple_message() {
            let (desc, bullets) = parse_commit_message("feat(cli): add new flag");
            assert_eq!(desc, "feat(cli): add new flag");
            assert!(bullets.is_empty());
        }

        #[test]
        fn message_with_bullets() {
            let msg = "feat(sniff): add recent commits\n\n- added period parsing\n- added CommitDesc struct";
            let (desc, bullets) = parse_commit_message(msg);
            assert_eq!(desc, "feat(sniff): add recent commits");
            assert_eq!(bullets.len(), 2);
            assert_eq!(bullets[0], "added period parsing");
            assert_eq!(bullets[1], "added CommitDesc struct");
        }

        #[test]
        fn message_with_asterisk_bullets() {
            let msg = "fix: resolve issue\n\n* fixed bug A\n* fixed bug B";
            let (desc, bullets) = parse_commit_message(msg);
            assert_eq!(desc, "fix: resolve issue");
            assert_eq!(bullets.len(), 2);
            assert_eq!(bullets[0], "fixed bug A");
        }

        #[test]
        fn empty_message() {
            let (desc, bullets) = parse_commit_message("");
            assert!(desc.is_empty());
            assert!(bullets.is_empty());
        }

        #[test]
        fn multi_paragraph_with_mixed_content() {
            let msg = "feat: big feature\n\nFirst paragraph.\n\n- bullet one\n- bullet two";
            let (desc, bullets) = parse_commit_message(msg);
            assert!(desc.contains("feat: big feature"));
            assert!(desc.contains("First paragraph."));
            assert_eq!(bullets.len(), 2);
        }
    }

    mod rendering_tests {
        use super::*;

        fn fc(path: &str, kind: DeltaKind) -> CommitFileChange {
            CommitFileChange {
                path: path.to_string(),
                kind,
            }
        }

        fn sample_set() -> CommitDescSet {
            CommitDescSet {
                commits: vec![CommitDesc {
                    hash: "a1b2c3d4e5f6a7b8c9d0".to_string(),
                    datetime: "2026-04-09T14:30:00+00:00".to_string(),
                    packages: Some(vec!["sniff".to_string()]),
                    package_areas: Some(vec!["sniff".to_string()]),
                    files: vec![
                        fc("sniff/lib/src/lib.rs", DeltaKind::Modified),
                        fc("sniff/lib/README.md", DeltaKind::Added),
                        fc("sniff/lib/Cargo.toml", DeltaKind::Modified),
                    ],
                    description: "feat(sniff): add recent commits".to_string(),
                    bullet_points: vec![
                        "added period parsing".to_string(),
                        "added CommitDesc struct".to_string(),
                    ],
                }],
                period_label: "last 3 days".to_string(),
                repo_root: PathBuf::from("/repo"),
                packages: None,
            }
        }

        #[test]
        fn describe_renders_commit_centric_header() {
            let set = sample_set();
            let md = set.describe(true);
            // Header: - [shorthash] <conventional prefix> at <time>: <message>
            assert!(
                md.contains("- [a1b2c3d] feat(sniff) at "),
                "expected commit-centric header with short hash + conventional prefix, got:\n{}",
                md
            );
            assert!(
                md.contains(": add recent commits"),
                "expected conventional-commit message suffix, got:\n{}",
                md
            );
        }

        #[test]
        fn describe_renders_description_and_files_impacted_blocks() {
            let set = sample_set();
            let md = set.describe(true);
            assert!(md.contains("    **Description:**\n"));
            assert!(md.contains("    - added period parsing"));
            assert!(md.contains("    **Files Impacted:**\n"));
            assert!(md.contains("    - modified: sniff/lib/src/lib.rs"));
            assert!(md.contains("    - added: sniff/lib/README.md"));
        }

        #[test]
        fn describe_plain_no_hyperlinks() {
            let set = sample_set();
            let md = set.describe(true);
            assert!(!md.contains("](file://"));
            assert!(md.contains("sniff/lib/src/lib.rs"));
        }

        #[test]
        fn describe_with_hyperlinks() {
            let set = sample_set();
            let md = set.describe(false);
            // url::Url::from_file_path produces file:///repo/... on Unix
            assert!(
                md.contains("](file:///repo/sniff/lib/src/lib.rs)"),
                "Expected file URI in markdown link, got: {}",
                md
            );
        }

        #[test]
        fn source_code_changes_filters_files_within_commits() {
            let set = sample_set();
            let md = set.source_code_changes(true);
            assert!(md.contains("### Source Code Changes"));
            // The commit header is still there
            assert!(md.contains("- [a1b2c3d] feat(sniff) at "));
            // Source file shows up in Files Impacted
            assert!(md.contains("- modified: sniff/lib/src/lib.rs"));
            // Non-source files are dropped even though commit touched them
            assert!(!md.contains("README.md"));
            assert!(!md.contains("Cargo.toml"));
        }

        #[test]
        fn documentation_changes_filters_files_within_commits() {
            let set = sample_set();
            let md = set.documentation_changes(true);
            assert!(md.contains("### Documentation Changes"));
            assert!(md.contains("- [a1b2c3d] feat(sniff) at "));
            assert!(md.contains("- added: sniff/lib/README.md"));
            assert!(!md.contains("sniff/lib/src/lib.rs"));
            assert!(!md.contains("Cargo.toml"));
        }

        #[test]
        fn source_code_changes_empty_when_none() {
            let set = CommitDescSet {
                commits: vec![CommitDesc {
                    hash: "abc1234".to_string(),
                    datetime: "2026-04-09T14:30:00+00:00".to_string(),
                    packages: None,
                    package_areas: None,
                    files: vec![fc("README.md", DeltaKind::Modified)],
                    description: "docs only".to_string(),
                    bullet_points: vec![],
                }],
                period_label: "last 3 days".to_string(),
                repo_root: PathBuf::from("/repo"),
                packages: None,
            };
            let md = set.source_code_changes(true);
            assert!(md.is_empty());
        }

        #[test]
        fn describe_header_without_conventional_commit() {
            let set = CommitDescSet {
                commits: vec![CommitDesc {
                    hash: "deadbeef".to_string(),
                    datetime: "2026-04-09T14:30:00+00:00".to_string(),
                    packages: None,
                    package_areas: None,
                    files: vec![fc("src/lib.rs", DeltaKind::Modified)],
                    description: "ship it".to_string(),
                    bullet_points: vec![],
                }],
                period_label: "last 3 days".to_string(),
                repo_root: PathBuf::from("/repo"),
                packages: None,
            };
            let md = set.describe(true);
            assert!(md.contains("- [deadbee] at "));
            assert!(md.contains(": ship it"));
        }

        #[test]
        fn commit_time_label_absolute_for_old_commit() {
            // Date well in the past to avoid "Today"/"Yesterday" flakiness.
            let today = NaiveDate::from_ymd_opt(2026, 4, 9).unwrap();
            let label = commit_time_label("2024-01-15T09:05:00+00:00", today);
            assert!(
                label.starts_with("2024-01-15 at "),
                "expected absolute-date label, got: {}",
                label
            );
        }
    }

    mod filtering_tests {
        use super::*;
        use crate::filesystem::repo::Package;

        #[test]
        fn filter_by_actions_keeps_matching_conventional_commits() {
            let mut set = CommitDescSet {
                commits: vec![
                    CommitDesc {
                        hash: "feat1234".to_string(),
                        datetime: "2026-04-09T14:30:00+00:00".to_string(),
                        packages: None,
                        package_areas: None,
                        files: vec![CommitFileChange {
                            path: "src/lib.rs".to_string(),
                            kind: DeltaKind::Modified,
                        }],
                        description: "feat(sniff): add action filtering".to_string(),
                        bullet_points: vec![],
                    },
                    CommitDesc {
                        hash: "fix12345".to_string(),
                        datetime: "2026-04-09T13:30:00+00:00".to_string(),
                        packages: None,
                        package_areas: None,
                        files: vec![CommitFileChange {
                            path: "src/lib.rs".to_string(),
                            kind: DeltaKind::Modified,
                        }],
                        description: "fix(sniff): preserve filtered output".to_string(),
                        bullet_points: vec![],
                    },
                    CommitDesc {
                        hash: "plain123".to_string(),
                        datetime: "2026-04-09T12:30:00+00:00".to_string(),
                        packages: None,
                        package_areas: None,
                        files: vec![CommitFileChange {
                            path: "src/lib.rs".to_string(),
                            kind: DeltaKind::Modified,
                        }],
                        description: "ship it".to_string(),
                        bullet_points: vec![],
                    },
                ],
                period_label: "last 3 days".to_string(),
                repo_root: PathBuf::from("/repo"),
                packages: None,
            };

            set.filter_by_actions(["feat", "fix"]);

            assert_eq!(set.commits.len(), 2);
            assert_eq!(
                set.commits[0].description,
                "feat(sniff): add action filtering"
            );
            assert_eq!(
                set.commits[1].description,
                "fix(sniff): preserve filtered output"
            );
        }

        fn make_packages() -> Vec<Package> {
            vec![
                Package {
                    path: PathBuf::from("/repo/pkg-a"),
                    relative: String::from("pkg-a"),
                    package_area: String::from("pkg"),
                    name: String::from("pkg-a"),
                    ecosystem: crate::filesystem::repo::PackageEcosystem::Cargo,
                    standard: crate::filesystem::repo::MonorepoStandard::Unknown,
                    provenance: crate::filesystem::repo::PackageProvenance::ManifestScan,
                    nested_packages: vec![],
                    primary_language: None,
                    secondary_languages: vec![],
                    languages: vec![],
                    frameworks: vec![],
                    file_associations: vec![],
                    configuration: vec![],
                    documentation: vec![],
                    editor_config: None,
                    command_runner: vec![],
                    package_managers: vec![],
                    test_runners: vec![],
                    version: None,
                    features: vec![],
                    depends_on: vec![],
                    used_by: vec![],
                    dependencies: None,
                    dev_dependencies: None,
                    peer_dependencies: None,
                    optional_dependencies: None,
                    is_updatable: None,
                    has_major_update: None,
                    is_excluded: false,
                },
                Package {
                    path: PathBuf::from("/repo/pkg-b"),
                    relative: String::from("pkg-b"),
                    package_area: String::from("pkg"),
                    name: String::from("pkg-b"),
                    ecosystem: crate::filesystem::repo::PackageEcosystem::Cargo,
                    standard: crate::filesystem::repo::MonorepoStandard::Unknown,
                    provenance: crate::filesystem::repo::PackageProvenance::ManifestScan,
                    nested_packages: vec![],
                    primary_language: None,
                    secondary_languages: vec![],
                    languages: vec![],
                    frameworks: vec![],
                    file_associations: vec![],
                    configuration: vec![],
                    documentation: vec![],
                    editor_config: None,
                    command_runner: vec![],
                    package_managers: vec![],
                    test_runners: vec![],
                    version: None,
                    features: vec![],
                    depends_on: vec![],
                    used_by: vec![],
                    dependencies: None,
                    dev_dependencies: None,
                    peer_dependencies: None,
                    optional_dependencies: None,
                    is_updatable: None,
                    has_major_update: None,
                    is_excluded: false,
                },
                Package {
                    path: PathBuf::from("/repo/apps/web"),
                    relative: String::from("apps/web"),
                    package_area: String::from("apps"),
                    name: String::from("apps-web"),
                    ecosystem: crate::filesystem::repo::PackageEcosystem::Node,
                    standard: crate::filesystem::repo::MonorepoStandard::Unknown,
                    provenance: crate::filesystem::repo::PackageProvenance::ManifestScan,
                    nested_packages: vec![],
                    primary_language: None,
                    secondary_languages: vec![],
                    languages: vec![],
                    frameworks: vec![],
                    file_associations: vec![],
                    configuration: vec![],
                    documentation: vec![],
                    editor_config: None,
                    command_runner: vec![],
                    package_managers: vec![],
                    test_runners: vec![],
                    version: None,
                    features: vec![],
                    depends_on: vec![],
                    used_by: vec![],
                    dependencies: None,
                    dev_dependencies: None,
                    peer_dependencies: None,
                    optional_dependencies: None,
                    is_updatable: None,
                    has_major_update: None,
                    is_excluded: false,
                },
                Package {
                    path: PathBuf::from("/repo/apps/browser"),
                    relative: String::from("apps/browser"),
                    package_area: String::from("apps"),
                    name: String::from("apps-browser"),
                    ecosystem: crate::filesystem::repo::PackageEcosystem::Node,
                    standard: crate::filesystem::repo::MonorepoStandard::Unknown,
                    provenance: crate::filesystem::repo::PackageProvenance::ManifestScan,
                    nested_packages: vec![],
                    primary_language: None,
                    secondary_languages: vec![],
                    languages: vec![],
                    frameworks: vec![],
                    file_associations: vec![],
                    configuration: vec![],
                    documentation: vec![],
                    editor_config: None,
                    command_runner: vec![],
                    package_managers: vec![],
                    test_runners: vec![],
                    version: None,
                    features: vec![],
                    depends_on: vec![],
                    used_by: vec![],
                    dependencies: None,
                    dev_dependencies: None,
                    peer_dependencies: None,
                    optional_dependencies: None,
                    is_updatable: None,
                    has_major_update: None,
                    is_excluded: false,
                },
            ]
        }

        fn fc(path: &str) -> CommitFileChange {
            CommitFileChange {
                path: path.to_string(),
                kind: DeltaKind::Modified,
            }
        }

        fn cross_package_set() -> CommitDescSet {
            CommitDescSet {
                commits: vec![
                    CommitDesc {
                        hash: "abc1234".to_string(),
                        datetime: "2026-04-09T14:30:00+00:00".to_string(),
                        packages: Some(vec!["pkg-a".to_string(), "pkg-b".to_string()]),
                        package_areas: Some(vec!["pkg".to_string()]),
                        files: vec![fc("pkg-a/src/lib.rs"), fc("pkg-b/src/main.rs")],
                        description: "cross-package commit".to_string(),
                        bullet_points: vec![],
                    },
                    CommitDesc {
                        hash: "def4567".to_string(),
                        datetime: "2026-04-09T13:00:00+00:00".to_string(),
                        packages: Some(vec!["pkg-a".to_string()]),
                        package_areas: Some(vec!["pkg".to_string()]),
                        files: vec![fc("pkg-a/src/lib.rs")],
                        description: "pkg-a only".to_string(),
                        bullet_points: vec![],
                    },
                    CommitDesc {
                        hash: "ghi7890".to_string(),
                        datetime: "2026-04-09T12:00:00+00:00".to_string(),
                        packages: Some(vec!["apps-web".to_string(), "apps-browser".to_string()]),
                        package_areas: Some(vec!["apps".to_string()]),
                        files: vec![fc("apps/web/src/index.ts"), fc("apps/browser/src/main.ts")],
                        description: "apps commit".to_string(),
                        bullet_points: vec![],
                    },
                ],
                period_label: "last 3 days".to_string(),
                repo_root: PathBuf::from("/repo"),
                packages: Some(make_packages()),
            }
        }

        #[test]
        fn filter_by_package_narrows_files_within_commits() {
            let mut set = cross_package_set();
            let _ = set.filter_by_package("pkg-a");

            assert_eq!(set.commits.len(), 2);

            for commit in &set.commits {
                for file in &commit.files {
                    assert!(
                        file.path.starts_with("pkg-a/"),
                        "File {} should be under pkg-a/",
                        file.path
                    );
                }
            }
        }

        #[test]
        fn filter_by_package_unknown_returns_error() {
            let mut set = cross_package_set();
            let result = set.filter_by_package("nonexistent");

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, crate::SniffError::UnknownPackage { .. }));
        }

        #[test]
        fn filter_by_package_area_prefix_matching() {
            let mut set = cross_package_set();
            let _ = set.filter_by_package_area("apps");

            for commit in &set.commits {
                for file in &commit.files {
                    assert!(
                        file.path.starts_with("apps/"),
                        "File {} should be under apps/",
                        file.path
                    );
                }
            }
        }

        #[test]
        fn filter_by_package_area_includes_nested_areas() {
            let mut set = cross_package_set();
            let _ = set.filter_by_package_area("apps");

            assert!(!set.commits.is_empty());
            for commit in &set.commits {
                assert!(
                    commit.files.iter().all(|f| f.path.starts_with("apps/")),
                    "All files should be under apps/ area"
                );
            }
        }

        #[test]
        fn filter_by_package_area_unknown_returns_error() {
            let mut set = cross_package_set();
            let result = set.filter_by_package_area("nonexistent");

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, crate::SniffError::UnknownPackageArea { .. }));
        }

        #[test]
        fn filter_preserves_packages_after_file_filtering() {
            let mut set = cross_package_set();
            let _ = set.filter_by_package("pkg-b");

            assert_eq!(set.commits.len(), 1);
            let commit = &set.commits[0];
            assert!(
                commit
                    .packages
                    .as_ref()
                    .unwrap()
                    .contains(&"pkg-b".to_string())
            );
        }
    }
}
