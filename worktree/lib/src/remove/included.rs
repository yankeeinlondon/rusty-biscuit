//! Consent assessment for ignored files selected by `.worktreeinclude`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::compare::{self, Comparison, FilesystemReader, Policy};
use crate::copy_record::{self, LoadedRecord};
use crate::error::WorktreeError;
use crate::fork_origin::{ForkOriginStore, fork_origin_path};
use crate::git::git_from;
use crate::include::{EntryKind, IncludeRules, guarded_kind, relative_path, resolve_include_set};
use crate::worktree::{CopySource, copy_source, parse_worktree_list};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulesBinding {
    pub location: Option<PathBuf>,
    pub presence: String,
    pub content_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BaselineBinding {
    Record { identity: String, content_digest: String },
    NoRecord { source: PathBuf, per_file_results: Vec<(String, String)>, untrusted_record_digest: Option<String> },
    Unavailable { untrusted_record_digest: Option<String> },
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mark { New, Changed, Unknown(String), Unchanged, Missing }

impl Mark {
    pub fn needs_consent(&self) -> bool {
        matches!(self, Self::New | Self::Changed | Self::Unknown(_))
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::New => "new", Self::Changed => "changed", Self::Unknown(_) => "unknown",
            Self::Unchanged => "unchanged", Self::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludedAssessment {
    pub needs_consent: Vec<(PathBuf, Mark)>,
    pub rules: RulesBinding,
    pub baseline: BaselineBinding,
    pub selected: Vec<(PathBuf, Mark)>,
    pub warnings: Vec<String>,
}

impl Default for IncludedAssessment {
    fn default() -> Self {
        Self { needs_consent: Vec::new(), rules: RulesBinding {
            location: None, presence: "missing".into(), content_digest: None,
        }, baseline: BaselineBinding::None, selected: Vec::new(), warnings: Vec::new() }
    }
}

fn digest(bytes: &[u8]) -> String {
    copy_record::path_hex(&biscuit_hash::blake3_hash_bytes(bytes))
}

fn source_for(base: &Path, worktree: &Path, branch: Option<&str>) -> Option<PathBuf> {
    let entries = parse_worktree_list(&git_from(base, base, &["worktree", "list", "--porcelain"]).ok()?);
    let store = ForkOriginStore::load_from(&fork_origin_path(base).ok()?);
    let fork = branch.and_then(|name| store.get(name)).map(|record| record.base_branch.as_str());
    let source = match copy_source(&entries, fork) {
        CopySource::Worktree(entry) | CopySource::Base(entry) => entry.path,
        CopySource::Ambiguous(_) => return None,
    };
    if fs::canonicalize(&source).ok()? == fs::canonicalize(worktree).ok()? { return None; }
    Some(source)
}

/// Resolves the effective rules and compares selected ignored files against
/// the trusted copy record, or a distinct source checkout when none exists.
pub fn classify_included(base: &Path, worktree: &Path, branch: Option<&str>)
    -> Result<IncludedAssessment, WorktreeError> {
    let (location, rules) = match IncludeRules::locate(worktree) {
        IncludeRules::Missing => (base.join(".worktreeinclude"), IncludeRules::locate(base)),
        other => (worktree.join(".worktreeinclude"), other),
    };
    let mut assessment = IncludedAssessment::default();
    match rules {
        IncludeRules::Missing => return Ok(assessment),
        IncludeRules::Indeterminate(reason) => return Err(WorktreeError::IncludeSetDiscovery(reason)),
        IncludeRules::Empty | IncludeRules::Present(_) => {
            let bytes = fs::read(&location).map_err(|e| WorktreeError::IncludeSetDiscovery(e.to_string()))?;
            assessment.rules = RulesBinding { location: Some(location.clone()),
                presence: if bytes.is_empty() { "empty" } else { "present" }.into(),
                content_digest: Some(digest(&bytes)) };
        }
    }
    let set = resolve_include_set(base, worktree, &location)?;
    let admin = git_from(base, worktree, &["rev-parse", "--path-format=absolute", "--git-dir"])?;
    let registration = copy_record::registration(Path::new(admin.trim()))?;
    let loaded = copy_record::load(base, worktree, &registration);
    if let LoadedRecord::Untrusted(reason) = &loaded {
        assessment.warnings.push(format!("copy record is untrusted: {reason}"));
    }
    let source = if matches!(loaded, LoadedRecord::Trusted(_)) { None } else { source_for(base, worktree, branch) };
    let record_digest = copy_record::record_path(base, worktree).ok()
        .and_then(|path| fs::read(path).ok()).map(|bytes| digest(&bytes));
    assessment.baseline = match &loaded {
        LoadedRecord::Trusted(record) => {
            BaselineBinding::Record { identity: record.registration.clone(),
                content_digest: record_digest.clone().unwrap_or_default() }
        }
        LoadedRecord::Absent | LoadedRecord::Untrusted(_) => match &source {
            Some(path) => BaselineBinding::NoRecord { source: path.clone(),
                per_file_results: Vec::new(), untrusted_record_digest: record_digest.clone() },
            None => BaselineBinding::Unavailable { untrusted_record_digest: record_digest.clone() },
        },
    };
    for entry in set.entries {
        let path = relative_path(&entry.path).map_err(|e| WorktreeError::IncludeSetDiscovery(e.to_string()))?;
        let mut source_observation = String::new();
        let mark = match &loaded {
            LoadedRecord::Trusted(record) => {
                let key = copy_record::path_hex(&entry.path);
                match record.files.iter().find(|file| file.path_hex == key) {
                    Some(file) => from_comparison(compare::compare(&file.observation,
                        &worktree.join(&path), Policy::Included, &FilesystemReader)),
                    None => Mark::New,
                }
            }
            LoadedRecord::Absent | LoadedRecord::Untrusted(_) => match &source {
                Some(source) => match guarded_kind(source, &path) {
                    Ok(Some(EntryKind::File | EntryKind::Symlink)) => match compare::observe(&source.join(&path), &FilesystemReader) {
                    Ok(baseline) => {
                        source_observation = digest(&serde_json::to_vec(&baseline)?);
                        from_comparison(compare::compare(&baseline,
                            &worktree.join(&path), Policy::Included, &FilesystemReader))
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        source_observation = "missing".into(); Mark::New
                    }
                    Err(error) => {
                        source_observation = error.to_string(); Mark::Unknown(error.to_string())
                    }
                    },
                    Ok(None) => match fs::symlink_metadata(source.join(&path)) {
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                            source_observation = "missing".into(); Mark::New
                        }
                        _ => Mark::Unknown("source path cannot be safely inspected".into()),
                    },
                    Ok(Some(EntryKind::Other)) => Mark::Unknown("source has unsupported file kind".into()),
                    Err(error) => Mark::Unknown(error.to_string()),
                },
                None => Mark::Unknown("no distinct copy source".into()),
            },
        };
        if let BaselineBinding::NoRecord { per_file_results, .. } = &mut assessment.baseline {
            per_file_results.push((copy_record::path_hex(&entry.path), source_observation));
        }
        if mark.needs_consent() { assessment.needs_consent.push((path.clone(), mark.clone())); }
        assessment.selected.push((path, mark));
    }
    Ok(assessment)
}

fn from_comparison(result: Comparison) -> Mark {
    match result {
        Comparison::Unchanged => Mark::Unchanged, Comparison::Changed => Mark::Changed,
        Comparison::Missing => Mark::Missing, Comparison::Unknown(reason) => Mark::Unknown(reason),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remove::test_support::TestRepo;

    #[test]
    fn own_empty_rules_do_not_fall_back_to_base_and_bad_rules_refuse() {
        let repo = TestRepo::new();
        fs::write(repo.path().join(".gitignore"), b".env\n").unwrap();
        repo.git(&["add", ".gitignore"]);
        repo.git(&["commit", "-q", "-m", "ignore"]);
        repo.write_include_rules(b".env\n");
        let target = repo.add_linked_worktree("included");
        fs::write(target.join(".env"), b"secret").unwrap();
        let fallback = classify_included(&repo.path(), &target, Some("included")).unwrap();
        assert_eq!(fallback.rules.location, Some(repo.path().join(".worktreeinclude")));
        assert_eq!(fallback.needs_consent[0].1, Mark::New);

        fs::write(target.join(".worktreeinclude"), b"").unwrap();
        let empty = classify_included(&repo.path(), &target, Some("included")).unwrap();
        assert_eq!(empty.rules.presence, "empty");
        assert_eq!(empty.rules.location, Some(target.join(".worktreeinclude")));
        assert!(empty.needs_consent.is_empty());

        fs::remove_file(target.join(".worktreeinclude")).unwrap();
        fs::create_dir(target.join(".worktreeinclude")).unwrap();
        assert!(matches!(classify_included(&repo.path(), &target, Some("included")),
            Err(WorktreeError::IncludeSetDiscovery(_))));
    }
}
