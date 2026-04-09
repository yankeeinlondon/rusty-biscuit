use chrono::{DateTime, Duration, NaiveDate, Utc};
use git2::Repository;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use tracing::debug;

use crate::filesystem::blast_radius::{is_documentation_path, is_source_code_path};
use crate::filesystem::git::detection::get_commit_files;
use crate::filesystem::repo::detect_repo;
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

    let since = Utc::now() - duration;
    let repo_info = detect_repo(&repo_root)?;

    let commits = collect_commits_since(&repo, since, repo_info.as_ref());

    Ok(CommitDescSet {
        commits,
        period_label: period_label.to_string(),
        repo_root,
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
    let repo_info = detect_repo(&repo_root)?;

    let commits = collect_commits_since(&repo, since, repo_info.as_ref());
    let period_label = format!("since {}", date);

    Ok(CommitDescSet {
        commits,
        period_label,
        repo_root,
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
    let commit = obj.peel_to_commit().map_err(|e| {
        debug!(hash = hash, error = %e, "could not peel to commit");
        SniffError::Git(e)
    })?;

    let since = DateTime::from_timestamp(commit.time().seconds(), 0).unwrap_or_default();
    let repo_info = detect_repo(&repo_root)?;

    let commits = collect_commits_since(&repo, since, repo_info.as_ref());
    let short_hash = &hash[..hash.len().min(8)];
    let period_label = format!("since commit {}", short_hash);

    Ok(CommitDescSet {
        commits,
        period_label,
        repo_root,
    })
}

// ---------------------------------------------------------------------------
// Internal commit walker
// ---------------------------------------------------------------------------

fn collect_commits_since(
    repo: &Repository,
    since: DateTime<Utc>,
    repo_info_opt: Option<&crate::filesystem::repo::RepoInfo>,
) -> Vec<CommitDesc> {
    let mut commits = Vec::new();

    let Ok(mut revwalk) = repo.revwalk() else {
        return commits;
    };
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
                    let path_str = file_path.0.to_string_lossy();
                    for pkg in pkgs.iter() {
                        if path_str.starts_with(pkg.relative.as_str()) {
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
    pub fn filter_by_package(&mut self, package_name: &str) {
        let package_lower = package_name.to_ascii_lowercase();
        self.commits.retain(|commit| {
            commit
                .packages
                .as_ref()
                .is_some_and(|pkgs| pkgs.iter().any(|p| p.eq_ignore_ascii_case(&package_lower)))
        });

        for commit in &mut self.commits {
            if let Some(ref mut pkgs) = commit.packages {
                pkgs.retain(|p| p.eq_ignore_ascii_case(&package_lower));
            }
        }
    }

    pub fn filter_by_package_area(&mut self, area_name: &str) {
        let area_lower = area_name.to_ascii_lowercase();
        self.commits.retain(|commit| {
            commit
                .package_areas
                .as_ref()
                .is_some_and(|areas| areas.iter().any(|a| a.eq_ignore_ascii_case(&area_lower)))
        });

        for commit in &mut self.commits {
            if let Some(ref mut areas) = commit.package_areas {
                areas.retain(|a| a.eq_ignore_ascii_case(&area_lower));
            }
        }
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
            };
            let md = set.source_code_changes(true);
            assert!(md.is_empty());
        }
    }
}
