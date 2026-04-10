use chrono::{DateTime, Duration, NaiveDate, Utc};
use git2::Repository;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use tracing::debug;

use crate::filesystem::blast_radius::{is_documentation_path, is_source_code_path};
use crate::filesystem::git::detection::get_commit_files;
use crate::filesystem::repo::{detect_repo, Package};
use crate::{Result, SniffError};

// ---------------------------------------------------------------------------
// Period parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum PeriodSpecifier {
    Duration(Duration),
    Date(NaiveDate),
    Hash(String),
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

fn parse_commit_message(message: &str) -> (String, Vec<String>) {
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
            if !first_para_consumed {
                description = para_description_parts.join(" ");
                first_para_consumed = true;
            } else if description.is_empty() {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDesc {
    pub hash: String,
    pub datetime: String,
    pub packages: Option<Vec<String>>,
    pub package_areas: Option<Vec<String>>,
    pub files: Vec<String>,
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

pub fn get_recent_commits_by_duration(
    base_dir: &Path,
    duration: Duration,
    period_label: &str,
) -> Result<CommitDescSet> {
    let repo = Repository::discover(base_dir)
        .map_err(|_| SniffError::NotARepository(base_dir.to_path_buf()))?;
    let repo_root = repo
        .workdir()
        .ok_or_else(|| SniffError::NotARepository(base_dir.to_path_buf()))?
        .to_path_buf();

    let until = Utc::now();
    let since = until - duration;
    let repo_info = detect_repo(&repo_root)?;

    let commits = collect_commits_in_range(&repo, since, until, repo_info.as_ref());
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
    let repo = Repository::discover(base_dir)
        .map_err(|_| SniffError::NotARepository(base_dir.to_path_buf()))?;
    let repo_root = repo
        .workdir()
        .ok_or_else(|| SniffError::NotARepository(base_dir.to_path_buf()))?
        .to_path_buf();

    let repo_info = detect_repo(&repo_root)?;

    let commits = collect_commits_in_range(&repo, since, until, repo_info.as_ref());
    let packages = repo_info.as_ref().and_then(|ri| ri.packages.clone());

    Ok(CommitDescSet {
        commits,
        period_label: period_label.to_string(),
        repo_root,
        packages,
    })
}

pub fn get_recent_commits_by_date(base_dir: &Path, date: NaiveDate) -> Result<CommitDescSet> {
    let repo = Repository::discover(base_dir)
        .map_err(|_| SniffError::NotARepository(base_dir.to_path_buf()))?;
    let repo_root = repo
        .workdir()
        .ok_or_else(|| SniffError::NotARepository(base_dir.to_path_buf()))?
        .to_path_buf();

    let since = date.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc();
    let until = Utc::now();
    let repo_info = detect_repo(&repo_root)?;

    let commits = collect_commits_in_range(&repo, since, until, repo_info.as_ref());
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
    let repo = Repository::discover(base_dir)
        .map_err(|_| SniffError::NotARepository(base_dir.to_path_buf()))?;
    let repo_root = repo
        .workdir()
        .ok_or_else(|| SniffError::NotARepository(base_dir.to_path_buf()))?
        .to_path_buf();

    let obj = repo.revparse_single(hash).map_err(|e| {
        debug!(hash = hash, error = %e, "could not resolve hash");
        SniffError::Git(e)
    })?;
    let target_commit = obj.peel_to_commit().map_err(|e| {
        debug!(hash = hash, error = %e, "could not peel to commit");
        SniffError::Git(e)
    })?;
    let target_oid = target_commit.id();

    let repo_info = detect_repo(&repo_root)?;

    let commits = collect_commits_from_hash_to_head(&repo, target_oid, repo_info.as_ref());
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

// ---------------------------------------------------------------------------
// Internal commit walker
// ---------------------------------------------------------------------------

fn collect_commits_in_range(
    repo: &Repository,
    since: DateTime<Utc>,
    until: DateTime<Utc>,
    repo_info_opt: Option<&crate::filesystem::repo::RepoInfo>,
) -> Vec<CommitDesc> {
    let mut commits = Vec::new();

    let Ok(mut revwalk) = repo.revwalk() else {
        return commits;
    };
    revwalk
        .set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)
        .ok();
    if revwalk.push_head().is_err() {
        return commits;
    }

    let packages = repo_info_opt.and_then(|ri| ri.packages.as_ref());
    let is_monorepo = repo_info_opt.is_some_and(|ri| ri.is_monorepo);

    for oid_result in revwalk {
        let Ok(oid) = oid_result else {
            continue;
        };
        let Ok(commit) = repo.find_commit(oid) else {
            continue;
        };

        let commit_time = DateTime::from_timestamp(commit.time().seconds(), 0).unwrap_or_default();
        if commit_time < since {
            break;
        }
        if commit_time >= until {
            continue;
        }

        let sha = oid.to_string();
        let files_raw = get_commit_files(repo, &sha);
        let files: Vec<String> = files_raw
            .iter()
            .map(|(p, _)| p.to_string_lossy().to_string())
            .collect();

        if files.is_empty() {
            continue;
        }

        let message = commit.message().unwrap_or("").trim();
        let (description, bullet_points) = parse_commit_message(message);

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

    commits
}

fn collect_commits_from_hash_to_head(
    repo: &Repository,
    target_oid: git2::Oid,
    repo_info_opt: Option<&crate::filesystem::repo::RepoInfo>,
) -> Vec<CommitDesc> {
    let mut commits = Vec::new();

    let packages = repo_info_opt.and_then(|ri| ri.packages.as_ref());
    let is_monorepo = repo_info_opt.is_some_and(|ri| ri.is_monorepo);

    if let Ok(target_commit) = repo.find_commit(target_oid) {
        let commit_time =
            DateTime::from_timestamp(target_commit.time().seconds(), 0).unwrap_or_default();
        let sha = target_oid.to_string();
        let files_raw = get_commit_files(repo, &sha);
        let files: Vec<String> = files_raw
            .iter()
            .map(|(p, _)| p.to_string_lossy().to_string())
            .collect();

        if !files.is_empty() {
            let message = target_commit.message().unwrap_or("").trim();
            let (description, bullet_points) = parse_commit_message(message);

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
    }

    let Ok(mut revwalk) = repo.revwalk() else {
        return commits;
    };
    revwalk
        .set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)
        .ok();

    if revwalk.push_head().is_err() {
        return commits;
    }

    if revwalk.hide(target_oid).is_err() {
        return commits;
    }

    for oid_result in revwalk {
        let Ok(oid) = oid_result else {
            continue;
        };
        let Ok(commit) = repo.find_commit(oid) else {
            continue;
        };

        let commit_time = DateTime::from_timestamp(commit.time().seconds(), 0).unwrap_or_default();

        let sha = oid.to_string();
        let files_raw = get_commit_files(repo, &sha);
        let files: Vec<String> = files_raw
            .iter()
            .map(|(p, _)| p.to_string_lossy().to_string())
            .collect();

        if files.is_empty() {
            continue;
        }

        let message = commit.message().unwrap_or("").trim();
        let (description, bullet_points) = parse_commit_message(message);

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

    commits
}

// ---------------------------------------------------------------------------
// Rendering — Markdown methods
// ---------------------------------------------------------------------------

impl CommitDescSet {
    pub fn describe(&self, plain: bool) -> String {
        let mut out = String::new();

        for commit in &self.commits {
            let date_str = extract_display_date(&commit.datetime);

            out.push_str(&format!("## {}\n\n", date_str));

            let short_hash = &commit.hash[..commit.hash.len().min(8)];
            out.push_str(&format!("- **Commit:** {}\n", short_hash));

            out.push_str("- **Files:**\n");
            for file in &commit.files {
                if plain {
                    out.push_str(&format!("    - {}\n", file));
                } else {
                    let abs = self.repo_root.join(file);
                    let abs_str = abs.to_string_lossy();
                    let file_url = if abs_str.starts_with('/') {
                        format!("file://{}", abs_str)
                    } else {
                        format!("file://{}", abs_str)
                    };
                    out.push_str(&format!("    - [{}]({})\n", file, file_url));
                }
            }

            out.push_str(&format!("- **Description:** {}\n", commit.description));
            for bp in &commit.bullet_points {
                out.push_str(&format!("    - {}\n", bp));
            }

            out.push('\n');
        }

        out
    }

    pub fn source_code_changes(&self, plain: bool) -> String {
        self.file_grouped_changes(ChangeKind::SourceCode, plain)
    }

    pub fn documentation_changes(&self, plain: bool) -> String {
        self.file_grouped_changes(ChangeKind::Documentation, plain)
    }

    fn file_grouped_changes(&self, kind: ChangeKind, plain: bool) -> String {
        let title = match kind {
            ChangeKind::SourceCode => "Source Code Changes",
            ChangeKind::Documentation => "Documentation Changes",
        };

        let mut file_commits: BTreeMap<String, Vec<&CommitDesc>> = BTreeMap::new();

        for commit in &self.commits {
            for file in &commit.files {
                let path = PathBuf::from(file);
                let matches = match kind {
                    ChangeKind::SourceCode => is_source_code_path(&path),
                    ChangeKind::Documentation => is_documentation_path(&path),
                };
                if matches {
                    file_commits.entry(file.clone()).or_default().push(commit);
                }
            }
        }

        if file_commits.is_empty() {
            return String::new();
        }

        let mut out = format!("### {} (_{}_)\n\n", title, self.period_label);

        for (file, commits) in &file_commits {
            if plain {
                out.push_str(&format!("- {}\n", file));
            } else {
                let abs = self.repo_root.join(file);
                let abs_str = abs.to_string_lossy();
                let file_url = if abs_str.starts_with('/') {
                    format!("file://{}", abs_str)
                } else {
                    format!("file://{}", abs_str)
                };
                out.push_str(&format!("- [{}]({})\n", file, file_url));
            }

            for commit in commits {
                let date_str = extract_display_date(&commit.datetime);
                let short_hash = &commit.hash[..commit.hash.len().min(8)];
                out.push_str(&format!(
                    "    - {} - _{}_ as part of commit **{}**\n",
                    date_str, commit.description, short_hash
                ));
                for bp in &commit.bullet_points {
                    out.push_str(&format!("        - {}\n", bp));
                }
            }
        }

        out
    }
}

#[derive(Debug, Clone, Copy)]
enum ChangeKind {
    SourceCode,
    Documentation,
}

fn extract_display_date(datetime: &str) -> String {
    if let Ok(dt) = DateTime::parse_from_rfc3339(datetime) {
        dt.format("%Y-%m-%d at %H:%M").to_string()
    } else {
        datetime.to_string()
    }
}

// ---------------------------------------------------------------------------
// Filtering
// ---------------------------------------------------------------------------

impl CommitDescSet {
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
            let filtered_files: Vec<String> = commit
                .files
                .iter()
                .filter(|f| PathBuf::from(f).starts_with(&pkg_relative))
                .cloned()
                .collect();

            commit.files = filtered_files;

            if commit.files.is_empty() {
                continue;
            }

            let mut new_pkgs = BTreeSet::new();
            let mut new_areas = BTreeSet::new();
            for file_path in &commit.files {
                let path = PathBuf::from(file_path);
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
            let filtered_files: Vec<String> = commit
                .files
                .iter()
                .filter(|f| {
                    let path = PathBuf::from(f);
                    package_roots.iter().any(|root| path.starts_with(root))
                })
                .cloned()
                .collect();

            commit.files = filtered_files;

            if commit.files.is_empty() {
                continue;
            }

            let mut new_pkgs = BTreeSet::new();
            let mut new_areas = BTreeSet::new();
            for file_path in &commit.files {
                let path = PathBuf::from(file_path);
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
        fn bare_number_invalid() {
            assert!(parse_period("123").is_err());
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

        fn sample_set() -> CommitDescSet {
            CommitDescSet {
                commits: vec![CommitDesc {
                    hash: "a1b2c3d4e5f6a7b8c9d0".to_string(),
                    datetime: "2026-04-09T14:30:00+00:00".to_string(),
                    packages: Some(vec!["sniff".to_string()]),
                    package_areas: Some(vec!["sniff".to_string()]),
                    files: vec![
                        "sniff/lib/src/lib.rs".to_string(),
                        "sniff/lib/README.md".to_string(),
                        "sniff/lib/Cargo.toml".to_string(),
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
        fn describe_produces_markdown() {
            let set = sample_set();
            let md = set.describe(true);
            assert!(md.contains("## 2026-04-09 at 14:30"));
            assert!(md.contains("**Commit:** a1b2c3d4"));
            assert!(md.contains("sniff/lib/src/lib.rs"));
            assert!(md.contains("**Description:**"));
            assert!(md.contains("added period parsing"));
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
            assert!(md.contains("](file:///repo/sniff/lib/src/lib.rs)"));
        }

        #[test]
        fn source_code_changes_filters_correctly() {
            let set = sample_set();
            let md = set.source_code_changes(true);
            assert!(md.contains("Source Code Changes"));
            assert!(md.contains("sniff/lib/src/lib.rs"));
            assert!(!md.contains("README.md"));
            assert!(!md.contains("Cargo.toml"));
        }

        #[test]
        fn documentation_changes_filters_correctly() {
            let set = sample_set();
            let md = set.documentation_changes(true);
            assert!(md.contains("Documentation Changes"));
            assert!(md.contains("sniff/lib/README.md"));
            assert!(!md.contains("sniff/lib/src/lib.rs"));
            assert!(!md.contains("Cargo.toml"));
        }

        #[test]
        fn source_code_changes_empty_when_none() {
            let set = CommitDescSet {
                commits: vec![CommitDesc {
                    hash: "abc123".to_string(),
                    datetime: "2026-04-09T14:30:00+00:00".to_string(),
                    packages: None,
                    package_areas: None,
                    files: vec!["README.md".to_string()],
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
    }

    mod filtering_tests {
        use super::*;
        use crate::filesystem::repo::Package;

        fn make_packages() -> Vec<Package> {
            vec![
                Package {
                    path: PathBuf::from("/repo/pkg-a"),
                    relative: String::from("pkg-a"),
                    package_area: String::from("pkg"),
                    name: String::from("pkg-a"),
                    ecosystem: crate::filesystem::repo::PackageEcosystem::Cargo,
                    discovery_sources: vec![],
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
                    discovery_sources: vec![],
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
                    discovery_sources: vec![],
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
                    discovery_sources: vec![],
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

        fn cross_package_set() -> CommitDescSet {
            CommitDescSet {
                commits: vec![
                    CommitDesc {
                        hash: "abc123".to_string(),
                        datetime: "2026-04-09T14:30:00+00:00".to_string(),
                        packages: Some(vec!["pkg-a".to_string(), "pkg-b".to_string()]),
                        package_areas: Some(vec!["pkg".to_string()]),
                        files: vec![
                            "pkg-a/src/lib.rs".to_string(),
                            "pkg-b/src/main.rs".to_string(),
                        ],
                        description: "cross-package commit".to_string(),
                        bullet_points: vec![],
                    },
                    CommitDesc {
                        hash: "def456".to_string(),
                        datetime: "2026-04-09T13:00:00+00:00".to_string(),
                        packages: Some(vec!["pkg-a".to_string()]),
                        package_areas: Some(vec!["pkg".to_string()]),
                        files: vec!["pkg-a/src/lib.rs".to_string()],
                        description: "pkg-a only".to_string(),
                        bullet_points: vec![],
                    },
                    CommitDesc {
                        hash: "ghi789".to_string(),
                        datetime: "2026-04-09T12:00:00+00:00".to_string(),
                        packages: Some(vec!["apps-web".to_string(), "apps-browser".to_string()]),
                        package_areas: Some(vec!["apps".to_string()]),
                        files: vec![
                            "apps/web/src/index.ts".to_string(),
                            "apps/browser/src/main.ts".to_string(),
                        ],
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
                        file.starts_with("pkg-a/"),
                        "File {} should be under pkg-a/",
                        file
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
                        file.starts_with("apps/"),
                        "File {} should be under apps/",
                        file
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
                    commit.files.iter().all(|f| f.starts_with("apps/")),
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
            assert!(commit
                .packages
                .as_ref()
                .unwrap()
                .contains(&"pkg-b".to_string()));
        }
    }
}
