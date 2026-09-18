//! Serializable recent-commit payload.
//!
//! The JSON shape is the contract described in
//! `sniff/docs/topics/repo/recent-commits-schema.md`, as amended by
//! `sniff/features/2026-09-15-recent-commits/spec.md` (bare array, author,
//! `moved` file kind, three-state `remote`, monorepo-only attribution).

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::filesystem::git::DeltaKind;
use crate::filesystem::git::discovery::CommittedFileChange;
use crate::filesystem::path_kind::{ChangeCategory, classify_path};

/// A collection of recent commits, newest first.
///
/// Serializes as a bare JSON array of [`RecentCommit`] objects.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecentCommits {
    commits: Vec<RecentCommit>,
    /// Working-tree root the commits were collected from, kept out of JSON for
    /// text reports that link changed files. `None` after deserialization.
    #[serde(skip)]
    repo_root: Option<PathBuf>,
}

impl RecentCommits {
    pub(crate) fn from_commits(commits: Vec<RecentCommit>) -> Self {
        Self {
            commits,
            repo_root: None,
        }
    }

    pub(crate) fn with_repo_root(mut self, repo_root: PathBuf) -> Self {
        self.repo_root = Some(repo_root);
        self
    }

    pub(crate) fn repo_root(&self) -> Option<&Path> {
        self.repo_root.as_deref()
    }

    /// The commits, newest first.
    pub fn commits(&self) -> &[RecentCommit] {
        &self.commits
    }

    pub fn len(&self) -> usize {
        self.commits.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commits.is_empty()
    }

    /// The payload as a bare JSON array. Verbosity and author display options
    /// never affect it.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.commits)
            .expect("recent-commit payload has only string map keys and cannot fail to serialize")
    }
}

/// One commit in a [`RecentCommits`] collection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecentCommit {
    /// Full hexadecimal commit hash.
    pub hash: String,
    /// Commit time, serialized as RFC 3339 in UTC (`+00:00`) regardless of the
    /// options' display timezone.
    #[serde(with = "rfc3339_utc")]
    pub datetime: DateTime<Utc>,
    pub author: RecentCommitAuthor,
    /// Conventional-commit operation, or `null` for a non-conventional subject.
    pub operation: Option<String>,
    /// Conventional-commit scope, or `null` when absent.
    pub scope: Option<String>,
    /// The message up to the first `.` or newline.
    pub heading: String,
    /// Prose after the heading and before the bullet points; possibly empty.
    pub description: String,
    pub bullet_points: Vec<String>,
    /// Files changed relative to the first parent. Empty for a no-change merge.
    pub files: Vec<RecentCommitFile>,
    pub file_types: RecentCommitFileTypes,
    /// Present only in a monorepo, where both arrays are always emitted (and
    /// may be empty). Outside a monorepo neither key is serialized.
    #[serde(flatten)]
    pub attribution: Option<RecentCommitPackages>,
    /// `true` when a local remote-tracking ref contains the commit, `false`
    /// when every remote-tracking tip was walked without finding it, and `null`
    /// when containment could not be determined.
    pub remote: Option<bool>,
    /// Browser URL for the commit on the containing remote; omitted when no
    /// containing remote yields one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentCommitAuthor {
    pub name: String,
    pub email: String,
}

/// Monorepo package attribution for one commit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentCommitPackages {
    /// Names of packages owning at least one changed file, sorted.
    pub packages: Vec<String>,
    /// Package areas of those packages, sorted.
    pub package_areas: Vec<String>,
}

/// How a file changed. Renames and copies are both `moved`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecentCommitFileKind {
    Modified,
    Added,
    Deleted,
    Moved,
}

impl From<DeltaKind> for RecentCommitFileKind {
    fn from(kind: DeltaKind) -> Self {
        match kind {
            DeltaKind::Added => Self::Added,
            DeltaKind::Modified => Self::Modified,
            DeltaKind::Deleted => Self::Deleted,
            DeltaKind::Renamed | DeltaKind::Copied => Self::Moved,
        }
    }
}

/// One file changed by a commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentCommitFile {
    pub kind: RecentCommitFileKind,
    /// Repository-relative path with `/` separators.
    pub path: String,
    /// Source path of a `moved` file; absent for every other kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_path: Option<String>,
    /// Lines added; absent when the change is binary or line statistics are
    /// unavailable (never reported as `0` in that case).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub added: Option<u64>,
    /// Lines removed; absent under the same conditions as `added`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub removed: Option<u64>,
}

impl From<CommittedFileChange> for RecentCommitFile {
    fn from(change: CommittedFileChange) -> Self {
        Self {
            kind: change.kind.into(),
            path: change.path.to_string_lossy().into_owned(),
            original_path: change
                .original_path
                .map(|path| path.to_string_lossy().into_owned()),
            added: change.line_counts.map(|counts| counts.added),
            removed: change.line_counts.map(|counts| counts.removed),
        }
    }
}

/// Which change categories a commit touched.
///
/// Each flag is set when at least one changed file — or the source path of a
/// `moved` file — classifies into that [`ChangeCategory`]. Files classified as
/// [`ChangeCategory::Other`] set no flag.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentCommitFileTypes {
    pub source_code: bool,
    pub web_assets: bool,
    pub images: bool,
    pub documentation: bool,
    pub configuration: bool,
    pub cicd: bool,
}

impl RecentCommitFileTypes {
    pub fn from_files(files: &[RecentCommitFile]) -> Self {
        let mut types = Self::default();
        let paths = files.iter().flat_map(|file| {
            std::iter::once(file.path.as_str()).chain(file.original_path.as_deref())
        });
        for path in paths {
            match classify_path(Path::new(path)) {
                ChangeCategory::SourceCode => types.source_code = true,
                ChangeCategory::WebAssets => types.web_assets = true,
                ChangeCategory::Images => types.images = true,
                ChangeCategory::Documentation => types.documentation = true,
                ChangeCategory::Configuration => types.configuration = true,
                ChangeCategory::Cicd => types.cicd = true,
                ChangeCategory::Other => {}
            }
        }
        types
    }
}

/// `DateTime<Utc>` as `YYYY-MM-DDTHH:MM:SS+00:00`. chrono's default serde
/// form uses `Z`, which the payload contract does not.
mod rfc3339_utc {
    use chrono::{DateTime, SecondsFormat, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(
        value: &DateTime<Utc>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&value.to_rfc3339_opts(SecondsFormat::AutoSi, false))
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<DateTime<Utc>, D::Error> {
        let raw = String::deserialize(deserializer)?;
        DateTime::parse_from_rfc3339(&raw)
            .map(|parsed| parsed.with_timezone(&Utc))
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::git::discovery::LineCounts;
    use chrono::TimeZone;
    use serde_json::json;
    use std::path::PathBuf;

    fn file(kind: RecentCommitFileKind, path: &str) -> RecentCommitFile {
        RecentCommitFile {
            kind,
            path: path.to_string(),
            original_path: None,
            added: None,
            removed: None,
        }
    }

    fn monorepo_commit() -> RecentCommit {
        let files = vec![
            RecentCommitFile {
                added: Some(3),
                removed: Some(0),
                ..file(RecentCommitFileKind::Modified, "sniff/lib/src/lib.rs")
            },
            RecentCommitFile {
                original_path: Some("docs/old.md".to_string()),
                added: Some(0),
                removed: Some(0),
                ..file(RecentCommitFileKind::Moved, "docs/new.md")
            },
            file(RecentCommitFileKind::Added, "assets/logo.png"),
        ];
        RecentCommit {
            hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
            datetime: Utc.with_ymd_and_hms(2026, 9, 17, 8, 30, 0).unwrap(),
            author: RecentCommitAuthor {
                name: "Ada Lovelace".to_string(),
                email: "ada@example.com".to_string(),
            },
            operation: Some("feat".to_string()),
            scope: Some("sniff".to_string()),
            heading: "add recent commits".to_string(),
            description: "Longer prose.".to_string(),
            bullet_points: vec!["first".to_string(), "second".to_string()],
            file_types: RecentCommitFileTypes::from_files(&files),
            files,
            attribution: Some(RecentCommitPackages {
                packages: vec!["sniff".to_string()],
                package_areas: vec!["sniff".to_string()],
            }),
            remote: Some(true),
            commit_url: Some(
                "https://github.com/o/r/commit/0123456789abcdef0123456789abcdef01234567"
                    .to_string(),
            ),
        }
    }

    fn single_package_commit() -> RecentCommit {
        RecentCommit {
            operation: None,
            scope: None,
            description: String::new(),
            bullet_points: Vec::new(),
            files: Vec::new(),
            file_types: RecentCommitFileTypes::default(),
            attribution: None,
            remote: None,
            commit_url: None,
            ..monorepo_commit()
        }
    }

    #[test]
    fn serializes_as_a_bare_array_with_the_exact_keys_order_and_values() {
        let commits = RecentCommits::from_commits(vec![monorepo_commit()]);

        let expected = concat!(
            r#"[{"hash":"0123456789abcdef0123456789abcdef01234567","#,
            r#""datetime":"2026-09-17T08:30:00+00:00","#,
            r#""author":{"name":"Ada Lovelace","email":"ada@example.com"},"#,
            r#""operation":"feat","scope":"sniff","heading":"add recent commits","#,
            r#""description":"Longer prose.","bullet_points":["first","second"],"#,
            r#""files":[{"kind":"modified","path":"sniff/lib/src/lib.rs","added":3,"removed":0},"#,
            r#"{"kind":"moved","path":"docs/new.md","original_path":"docs/old.md","added":0,"removed":0},"#,
            r#"{"kind":"added","path":"assets/logo.png"}],"#,
            r#""file_types":{"source_code":true,"web_assets":false,"images":true,"#,
            r#""documentation":true,"configuration":false,"cicd":false},"#,
            r#""packages":["sniff"],"package_areas":["sniff"],"#,
            r#""remote":true,"#,
            r#""commit_url":"https://github.com/o/r/commit/0123456789abcdef0123456789abcdef01234567"}]"#,
        );
        // Direct serialization keeps declaration order; `to_json()` is the
        // same document as a `Value`, whose map orders keys itself.
        assert_eq!(serde_json::to_string(&commits).unwrap(), expected);
        assert_eq!(
            commits.to_json(),
            serde_json::from_str::<serde_json::Value>(expected).unwrap()
        );
        assert!(commits.to_json().is_array());
        assert!(commits.to_json()[0].get("period_label").is_none());
        assert!(commits.to_json()[0].get("repo_root").is_none());
    }

    #[test]
    fn non_monorepo_commit_omits_attribution_and_url_but_keeps_nullable_fields() {
        let json = RecentCommits::from_commits(vec![single_package_commit()]).to_json();
        let object = json[0].as_object().unwrap();

        assert!(!object.contains_key("packages"));
        assert!(!object.contains_key("package_areas"));
        assert!(!object.contains_key("commit_url"));
        assert_eq!(object["operation"], json!(null));
        assert_eq!(object["scope"], json!(null));
        assert_eq!(object["remote"], json!(null));
        assert_eq!(object["description"], json!(""));
        assert_eq!(object["bullet_points"], json!([]));
        assert_eq!(object["files"], json!([]));
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "author",
                "bullet_points",
                "datetime",
                "description",
                "file_types",
                "files",
                "hash",
                "heading",
                "operation",
                "remote",
                "scope",
            ]
        );
    }

    #[test]
    fn monorepo_commit_without_packaged_files_serializes_empty_arrays() {
        let commit = RecentCommit {
            attribution: Some(RecentCommitPackages::default()),
            ..single_package_commit()
        };
        let json = RecentCommits::from_commits(vec![commit]).to_json();

        assert_eq!(json[0]["packages"], json!([]));
        assert_eq!(json[0]["package_areas"], json!([]));
    }

    #[test]
    fn remote_false_is_serialized_distinctly_from_null() {
        let commit = RecentCommit {
            remote: Some(false),
            ..single_package_commit()
        };
        assert_eq!(
            RecentCommits::from_commits(vec![commit]).to_json()[0]["remote"],
            json!(false)
        );
    }

    #[test]
    fn empty_collection_is_an_empty_array() {
        let commits = RecentCommits::default();
        assert!(commits.is_empty());
        assert_eq!(commits.len(), 0);
        assert_eq!(commits.to_json(), json!([]));
    }

    #[test]
    fn json_round_trips_through_repeated_read_write_read() {
        let original =
            RecentCommits::from_commits(vec![monorepo_commit(), single_package_commit()]);

        let first_text = serde_json::to_string(&original).unwrap();
        let first_read: RecentCommits = serde_json::from_str(&first_text).unwrap();
        let second_text = serde_json::to_string(&first_read).unwrap();
        let second_read: RecentCommits = serde_json::from_str(&second_text).unwrap();

        assert_eq!(first_read, original);
        assert_eq!(second_read, original);
        assert_eq!(second_text, first_text);
        assert_eq!(second_read.commits()[1].attribution, None);
    }

    #[test]
    fn datetime_parses_non_utc_offsets_into_utc() {
        let text = serde_json::to_string(&single_package_commit())
            .unwrap()
            .replace("2026-09-17T08:30:00+00:00", "2026-09-17T14:00:00+05:30");
        let commit: RecentCommit = serde_json::from_str(&text).unwrap();

        assert_eq!(
            commit.datetime,
            Utc.with_ymd_and_hms(2026, 9, 17, 8, 30, 0).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&commit).unwrap()["datetime"],
            json!("2026-09-17T08:30:00+00:00")
        );
    }

    #[test]
    fn malformed_datetime_is_a_deserialization_error() {
        let text = serde_json::to_string(&single_package_commit())
            .unwrap()
            .replace("2026-09-17T08:30:00+00:00", "yesterday");
        assert!(serde_json::from_str::<RecentCommit>(&text).is_err());
    }

    #[test]
    fn unknown_file_kind_is_a_deserialization_error() {
        for kind in ["renamed", "copied", "Moved"] {
            let text = format!(r#"{{"kind":"{kind}","path":"a.rs"}}"#);
            assert!(
                serde_json::from_str::<RecentCommitFile>(&text).is_err(),
                "{kind}"
            );
        }
    }

    #[test]
    fn delta_kinds_normalize_rewrites_to_moved() {
        let serialized: Vec<serde_json::Value> = [
            DeltaKind::Added,
            DeltaKind::Modified,
            DeltaKind::Deleted,
            DeltaKind::Renamed,
            DeltaKind::Copied,
        ]
        .into_iter()
        .map(|kind| serde_json::to_value(RecentCommitFileKind::from(kind)).unwrap())
        .collect();

        assert_eq!(
            serialized,
            [
                json!("added"),
                json!("modified"),
                json!("deleted"),
                json!("moved"),
                json!("moved")
            ]
        );
    }

    #[test]
    fn committed_changes_project_to_public_file_records() {
        let renamed = CommittedFileChange {
            path: PathBuf::from("new/name.rs"),
            kind: DeltaKind::Renamed,
            original_path: Some(PathBuf::from("old/name.rs")),
            line_counts: Some(LineCounts {
                added: 0,
                removed: 0,
            }),
        };
        let copied_binary = CommittedFileChange {
            path: PathBuf::from("assets/copy.png"),
            kind: DeltaKind::Copied,
            original_path: Some(PathBuf::from("assets/logo.png")),
            line_counts: None,
        };

        assert_eq!(
            serde_json::to_value(RecentCommitFile::from(renamed)).unwrap(),
            json!({"kind": "moved", "path": "new/name.rs", "original_path": "old/name.rs", "added": 0, "removed": 0})
        );
        assert_eq!(
            serde_json::to_value(RecentCommitFile::from(copied_binary)).unwrap(),
            json!({"kind": "moved", "path": "assets/copy.png", "original_path": "assets/logo.png"})
        );
    }

    #[test]
    fn file_types_reflect_every_category_touched_including_move_sources() {
        let files = vec![
            file(RecentCommitFileKind::Modified, ".github/workflows/ci.yml"),
            file(RecentCommitFileKind::Added, "site/index.html"),
            RecentCommitFile {
                original_path: Some("config/app.toml".to_string()),
                ..file(RecentCommitFileKind::Moved, "data/app.csv")
            },
        ];

        assert_eq!(
            RecentCommitFileTypes::from_files(&files),
            RecentCommitFileTypes {
                source_code: false,
                web_assets: true,
                images: false,
                documentation: false,
                configuration: true,
                cicd: true,
            }
        );
        assert_eq!(
            RecentCommitFileTypes::from_files(&[file(RecentCommitFileKind::Added, "LICENSE")]),
            RecentCommitFileTypes::default()
        );
    }
}
