//! `RecentCommits::collect` through the public API against real temporary
//! repositories.
//!
//! Fixtures build history in-process with `git2` and explicit signatures, so
//! host Git configuration, hooks, credentials, and the network are never
//! consulted.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone, Utc};
use git2::{ObjectType, Oid, Repository, Signature, Time};
use serde_json::json;
use sniff::SniffError;
use sniff::filesystem::git::{
    GitRepo, NamedDate, RecentCommitFileKind, RecentCommits, RecentCommitsOptions,
};
use sniff::filesystem::path_kind::ChangeCategory;
use sniff::filesystem::repo::detect_repo;
use sniff::performance::{PerformanceCollector, counters, with_current_collector};
use tempfile::TempDir;

const ADA: (&str, &str) = ("Ada Lovelace", "ada@example.com");
const GRACE: (&str, &str) = ("Grace Hopper", "grace@navy.mil");

/// 2025-11-14T22:13:20Z; every default commit is one minute later than the last.
const BASE_SECONDS: i64 = 1_763_158_400;

struct Fixture {
    dir: TempDir,
    repo: Repository,
    next_seconds: Cell<i64>,
}

impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", ADA.0).unwrap();
            config.set_str("user.email", ADA.1).unwrap();
        }
        Self {
            dir,
            repo,
            next_seconds: Cell::new(BASE_SECONDS),
        }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn write(&self, relative: &str, contents: impl AsRef<[u8]>) -> &Self {
        let path = self.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
        self
    }

    fn remove(&self, relative: &str) -> &Self {
        fs::remove_file(self.path().join(relative)).unwrap();
        self
    }

    fn head(&self) -> Option<Oid> {
        self.repo.head().ok().and_then(|head| head.target())
    }

    /// Stage the whole worktree and commit it on `HEAD` as Ada at the next
    /// default timestamp.
    fn commit(&self, message: &str) -> Oid {
        let seconds = self.next_seconds.get();
        self.next_seconds.set(seconds + 60);
        self.commit_as(message, ADA, seconds, 0)
    }

    fn commit_as(&self, message: &str, author: (&str, &str), seconds: i64, offset_minutes: i32) -> Oid {
        let parents: Vec<Oid> = self.head().into_iter().collect();
        self.commit_on("HEAD", message, author, seconds, offset_minutes, &parents)
    }

    fn commit_on(
        &self,
        reference: &str,
        message: &str,
        author: (&str, &str),
        seconds: i64,
        offset_minutes: i32,
        parents: &[Oid],
    ) -> Oid {
        let mut index = self.repo.index().unwrap();
        index
            .add_all(["*"].iter(), git2::IndexAddOption::FORCE, None)
            .unwrap();
        index.update_all(["*"].iter(), None).unwrap();
        index.write().unwrap();
        let tree = self.repo.find_tree(index.write_tree().unwrap()).unwrap();
        let signature =
            Signature::new(author.0, author.1, &Time::new(seconds, offset_minutes)).unwrap();
        let parents: Vec<git2::Commit<'_>> = parents
            .iter()
            .map(|id| self.repo.find_commit(*id).unwrap())
            .collect();
        let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();
        self.repo
            .commit(
                Some(reference),
                &signature,
                &signature,
                message,
                &tree,
                &parent_refs,
            )
            .unwrap()
    }

    fn set_ref(&self, name: &str, target: Oid) {
        self.repo.reference(name, target, true, "fixture").unwrap();
    }

    fn collect(&self, options: &RecentCommitsOptions) -> sniff::Result<RecentCommits> {
        let repo = GitRepo::discover(self.path()).unwrap().unwrap();
        RecentCommits::collect(&repo, options)
    }

    /// Collect under a private collector and return the work counters too.
    fn collect_counted(
        &self,
        options: &RecentCommitsOptions,
    ) -> (sniff::Result<RecentCommits>, BTreeMap<String, u64>) {
        let collector = PerformanceCollector::new_shared();
        let result = with_current_collector(Some(Arc::clone(&collector)), || self.collect(options));
        (result, collector.snapshot(StdDuration::ZERO).counters)
    }
}

fn counter(counts: &BTreeMap<String, u64>, name: &str) -> u64 {
    counts.get(name).copied().unwrap_or(0)
}

fn headings(commits: &RecentCommits) -> Vec<&str> {
    commits.commits().iter().map(|commit| commit.heading.as_str()).collect()
}

fn utc_offset() -> FixedOffset {
    FixedOffset::east_opt(0).unwrap()
}

mod selection {
    use super::*;

    #[test]
    fn default_is_the_last_ten_commits_as_the_exact_bare_array_schema() {
        let fixture = Fixture::new();
        for index in 0..12 {
            fixture.write(&format!("src/file{index}.rs"), format!("fn f{index}() {{}}\n"));
            fixture.commit(&format!("feat(core): add file {index}\n\nBody prose.\n\n- bullet {index}"));
        }

        let commits = fixture.collect(&RecentCommitsOptions::new()).unwrap();

        assert_eq!(commits.len(), 10);
        assert_eq!(headings(&commits)[0], "add file 11");
        assert_eq!(headings(&commits)[9], "add file 2");

        let json = commits.to_json();
        let newest = &json.as_array().unwrap()[0];
        assert_eq!(
            newest,
            &json!({
                "hash": commits.commits()[0].hash,
                "datetime": "2025-11-14T22:24:20+00:00",
                "author": {"name": "Ada Lovelace", "email": "ada@example.com"},
                "operation": "feat",
                "scope": "core",
                "heading": "add file 11",
                "description": "Body prose.",
                "bullet_points": ["bullet 11"],
                "files": [{"kind": "added", "path": "src/file11.rs", "added": 1, "removed": 0}],
                "file_types": {
                    "source_code": true, "web_assets": false, "images": false,
                    "documentation": false, "configuration": false, "cicd": false
                },
                "remote": false
            })
        );
    }

    #[test]
    fn count_zero_is_rejected_before_any_history_is_read() {
        let fixture = Fixture::new();
        fixture.write("a.txt", "a").commit("first");

        let (result, counts) = fixture.collect_counted(&RecentCommitsOptions::new().count(0));

        assert!(matches!(result, Err(SniffError::InvalidPeriod(value)) if value == "0"));
        assert_eq!(counter(&counts, counters::GIT_COMMIT_VISITS), 0);
    }

    #[test]
    fn count_larger_than_history_returns_every_commit() {
        let fixture = Fixture::new();
        fixture.write("a.txt", "a").commit("first");
        fixture.write("b.txt", "b").commit("second");

        let commits = fixture.collect(&RecentCommitsOptions::new().count(50)).unwrap();

        assert_eq!(headings(&commits), ["second", "first"]);
    }

    #[test]
    fn empty_matches_and_unborn_head_are_successful_empty_collections() {
        let unborn = Fixture::new();
        let commits = unborn.collect(&RecentCommitsOptions::new()).unwrap();
        assert!(commits.is_empty());
        assert_eq!(commits.to_json(), json!([]));

        let fixture = Fixture::new();
        fixture.write("a.txt", "a").commit("chore: only commit");
        for options in [
            RecentCommitsOptions::new().operation("fix"),
            RecentCommitsOptions::new().date(NaiveDate::from_ymd_opt(2001, 1, 1).unwrap()),
            RecentCommitsOptions::new().author("nobody"),
        ] {
            let commits = fixture.collect(&options).unwrap();
            assert!(commits.is_empty(), "{options:?}");
            assert_eq!(commits.to_json(), json!([]));
        }
    }

    #[test]
    fn duration_keeps_only_commits_inside_the_window() {
        let fixture = Fixture::new();
        let now = Utc::now().timestamp();
        fixture.write("old.txt", "old").commit_as("five days ago", ADA, now - 5 * 86_400, 0);
        fixture.write("new.txt", "new").commit_as("an hour ago", ADA, now - 3_600, 0);

        let commits = fixture
            .collect(&RecentCommitsOptions::new().duration(chrono::Duration::days(2)))
            .unwrap();

        assert_eq!(headings(&commits), ["an hour ago"]);
    }

    #[test]
    fn specific_date_is_one_local_day_in_the_options_offset() {
        let fixture = Fixture::new();
        // 2026-03-31T18:45Z == 2026-04-01T00:15 at +05:30.
        let early = Utc.with_ymd_and_hms(2026, 3, 31, 18, 45, 0).unwrap();
        // 2026-04-02T07:30Z == 2026-04-01T23:30 at -08:00.
        let late_evening = Utc.with_ymd_and_hms(2026, 4, 2, 7, 30, 0).unwrap();
        // 2026-04-02T08:30Z == 2026-04-02T00:30 at -08:00.
        let after_midnight = Utc.with_ymd_and_hms(2026, 4, 2, 8, 30, 0).unwrap();
        fixture.write("a.txt", "a").commit_as("early", ADA, early.timestamp(), 330);
        fixture.write("b.txt", "b").commit_as("late evening", ADA, late_evening.timestamp(), -480);
        fixture.write("c.txt", "c").commit_as("after midnight", ADA, after_midnight.timestamp(), -480);

        let april_first = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let select = |offset: FixedOffset| {
            fixture
                .collect(&RecentCommitsOptions::new().date(april_first).timezone(offset))
                .unwrap()
        };

        let pacific = select(FixedOffset::west_opt(8 * 3600).unwrap());
        assert_eq!(headings(&pacific), ["late evening"]);
        // JSON stays UTC even though the commit and the selection use -08:00.
        assert_eq!(
            pacific.to_json()[0]["datetime"],
            json!("2026-04-02T07:30:00+00:00")
        );

        let india = select(FixedOffset::east_opt(5 * 3600 + 1800).unwrap());
        assert_eq!(headings(&india), ["early"]);

        assert!(select(utc_offset()).is_empty());
    }

    #[test]
    fn today_and_yesterday_are_local_calendar_days_in_the_options_offset() {
        let fixture = Fixture::new();
        let now = Utc::now();
        // Pick the offset that puts "now" at local noon, so one minute ago is
        // today and thirteen hours ago is yesterday regardless of when this runs.
        let seconds_of_day = now.timestamp().rem_euclid(86_400) as i32;
        let noon = FixedOffset::east_opt((43_200 - seconds_of_day) % 86_400).unwrap();
        fixture
            .write("a.txt", "a")
            .commit_as("two days ago", ADA, now.timestamp() - 37 * 3_600, 0);
        fixture
            .write("b.txt", "b")
            .commit_as("yesterday evening", ADA, now.timestamp() - 13 * 3_600, 0);
        fixture
            .write("c.txt", "c")
            .commit_as("a minute ago", ADA, now.timestamp() - 60, 0);

        let on = |day: NamedDate| {
            let commits = fixture
                .collect(&RecentCommitsOptions::new().named_date(day).timezone(noon))
                .unwrap();
            headings(&commits).iter().map(|h| h.to_string()).collect::<Vec<_>>()
        };

        assert_eq!(on(NamedDate::Today), ["a minute ago"]);
        assert_eq!(on(NamedDate::Yesterday), ["yesterday evening"]);
    }

    #[test]
    fn hash_selects_from_the_tip_through_the_hash_inclusive() {
        let fixture = Fixture::new();
        fixture.write("a.txt", "a").commit("first");
        let second = fixture.write("b.txt", "b").commit("second");
        fixture.write("c.txt", "c").commit("third");
        fixture.write("d.txt", "d").commit("fourth");

        let abbreviated = &second.to_string()[..10];
        let commits = fixture
            .collect(&RecentCommitsOptions::new().hash(abbreviated))
            .unwrap();

        assert_eq!(headings(&commits), ["fourth", "third", "second"]);
    }

    #[test]
    fn a_skewed_old_head_does_not_hide_in_window_ancestors() {
        let fixture = Fixture::new();
        let now = Utc::now().timestamp();
        fixture.write("a.txt", "a").commit_as("recent parent", ADA, now - 60, 0);
        // 2000-01-01T00:00:00Z: far outside the window, but newest in topology.
        fixture.write("b.txt", "b").commit_as("old head", ADA, 946_684_800, 0);

        let commits = fixture
            .collect(&RecentCommitsOptions::new().duration(chrono::Duration::days(7)))
            .unwrap();

        assert_eq!(headings(&commits), ["recent parent"]);
    }

    #[test]
    fn an_empty_commit_is_a_hash_boundary_with_no_files() {
        let fixture = Fixture::new();
        fixture.write("a.txt", "a").commit("first");
        let empty = fixture.commit("empty boundary");
        fixture.write("b.txt", "b").commit("after");

        let commits = fixture
            .collect(&RecentCommitsOptions::new().hash(empty.to_string()))
            .unwrap();

        assert_eq!(headings(&commits), ["after", "empty boundary"]);
        assert!(commits.commits()[1].files.is_empty());
        assert_eq!(commits.to_json()[1]["files"], json!([]));
    }

    #[test]
    fn unknown_or_unreachable_hashes_are_typed_errors() {
        let fixture = Fixture::new();
        let base = fixture.write("a.txt", "a").commit("base");
        fixture.write("b.txt", "b").commit("main");
        let side = fixture.commit_on("refs/heads/side", "side", ADA, BASE_SECONDS + 3_600, 0, &[base]);

        for hash in [side.to_string(), "deadbeefdeadbeef".to_string()] {
            let result = fixture.collect(&RecentCommitsOptions::new().hash(hash.clone()));
            assert!(
                matches!(&result, Err(SniffError::HashNotReachable { hash: reported }) if *reported == hash),
                "{hash}: {result:?}"
            );
        }
    }

    #[test]
    fn branch_is_a_history_base_with_local_first_then_remote_tracking_fallback() {
        let fixture = Fixture::new();
        // The bare-name fallback searches configured remotes.
        fixture.repo.remote("origin", "https://example.com/o/r.git").unwrap();
        let first = fixture.write("a.txt", "a").commit("first");
        let second = fixture.write("b.txt", "b").commit("second");
        fixture.set_ref("refs/heads/feature", first);
        fixture.set_ref("refs/remotes/origin/feature", second);
        let remote_only =
            fixture.commit_on("refs/remotes/origin/remote-only", "remote only", ADA, BASE_SECONDS + 3_600, 0, &[first]);
        fixture.write("c.txt", "c").commit("third on head");

        let from = |branch: &str| {
            fixture
                .collect(&RecentCommitsOptions::new().branch(branch))
                .map(|commits| headings(&commits).iter().map(|h| h.to_string()).collect::<Vec<_>>())
        };

        assert_eq!(from("feature").unwrap(), ["first"], "local branch wins");
        assert_eq!(from("origin/feature").unwrap(), ["second", "first"]);
        assert_eq!(from("remote-only").unwrap(), ["remote only", "first"]);
        assert_eq!(from("origin/remote-only").unwrap(), ["remote only", "first"]);
        assert_eq!(
            fixture.collect(&RecentCommitsOptions::new()).unwrap().commits()[0].heading,
            "third on head"
        );
        assert_ne!(remote_only, second);

        for missing in ["missing", "not a valid..ref"] {
            assert!(
                matches!(from(missing), Err(SniffError::UnknownBranch { name }) if name == missing),
                "{missing}"
            );
        }
    }
}

mod filters {
    use super::*;

    #[test]
    fn count_is_satisfied_by_matching_commits_found_during_the_walk() {
        let fixture = Fixture::new();
        for (index, message) in ["fix: one", "feat: two", "feat: three", "fix: four", "feat: five"]
            .iter()
            .enumerate()
        {
            fixture.write(&format!("f{index}.txt"), "line\n");
            fixture.commit(message);
        }

        let (result, counts) =
            fixture.collect_counted(&RecentCommitsOptions::new().count(2).operation("fix"));
        let commits = result.unwrap();

        assert_eq!(headings(&commits), ["four", "one"]);
        // Only the two survivors are diffed; filtered commits cost no content diff.
        assert_eq!(counter(&counts, counters::GIT_FILE_DIFFS), 2);
    }

    #[test]
    fn repeated_operations_match_any_case_insensitively_and_exclude_non_conventional() {
        let fixture = Fixture::new();
        for message in ["Planning: roadmap", "FIX(cli): typo", "plain message", "feat: add", "docs: readme"] {
            fixture.commit(message);
        }

        let commits = fixture
            .collect(
                &RecentCommitsOptions::new()
                    .count(50)
                    .operation("planning")
                    .operation("fix"),
            )
            .unwrap();

        assert_eq!(headings(&commits), ["typo", "roadmap"]);
        assert_eq!(commits.commits()[0].operation.as_deref(), Some("FIX"));
    }

    #[test]
    fn scope_and_operation_filters_combine_with_and() {
        let fixture = Fixture::new();
        for message in ["fix(cli): a", "fix(lib): b", "feat(cli): c", "fix: d"] {
            fixture.commit(message);
        }

        let commits = fixture
            .collect(&RecentCommitsOptions::new().operation("fix").scope("CLI"))
            .unwrap();

        assert_eq!(headings(&commits), ["a"]);
    }

    #[test]
    fn author_is_a_case_insensitive_substring_of_name_or_email() {
        let fixture = Fixture::new();
        fixture.commit_as("by ada", ADA, BASE_SECONDS, 0);
        fixture.commit_as("by grace", GRACE, BASE_SECONDS + 60, 0);

        let by = |needle: &str| {
            let commits = fixture
                .collect(&RecentCommitsOptions::new().author(needle))
                .unwrap();
            headings(&commits).iter().map(|h| h.to_string()).collect::<Vec<_>>()
        };

        assert_eq!(by("EXAMPLE.com"), ["by ada"], "email only");
        assert_eq!(by("ada"), ["by ada"], "name only");
        assert_eq!(by("hopper"), ["by grace"]);
        assert_eq!(by("a"), ["by grace", "by ada"]);

        let all = fixture.collect(&RecentCommitsOptions::new()).unwrap();
        let commit = &all.commits()[1];
        assert_eq!(commit.author.name, "Ada Lovelace");
        assert_eq!(commit.author.email, "ada@example.com");
    }

    #[test]
    fn file_type_filters_require_every_listed_category() {
        let fixture = Fixture::new();
        fixture.write("README.md", "# docs\n").commit("docs only");
        fixture.write("src/lib.rs", "pub fn a() {}\n").commit("source only");
        fixture
            .write("docs/guide.md", "guide\n")
            .write("src/main.rs", "fn main() {}\n")
            .commit("docs and source");
        fixture.write(".github/workflows/ci.yml", "on: push\n").commit("ci");

        let with = |categories: &[ChangeCategory]| {
            let options = categories
                .iter()
                .fold(RecentCommitsOptions::new(), |options, category| {
                    options.has_file_type(*category)
                });
            let commits = fixture.collect(&options).unwrap();
            headings(&commits).iter().map(|h| h.to_string()).collect::<Vec<_>>()
        };

        assert_eq!(with(&[ChangeCategory::Documentation]), ["docs and source", "docs only"]);
        assert_eq!(
            with(&[ChangeCategory::Documentation, ChangeCategory::SourceCode]),
            ["docs and source"]
        );
        assert_eq!(with(&[ChangeCategory::Cicd]), ["ci"]);
    }
}

mod files {
    use super::*;

    #[test]
    fn rename_and_binary_changes_surface_through_collection() {
        let fixture = Fixture::new();
        let prose = "line one\nline two\nline three\nline four\n";
        fixture
            .write("docs/old.md", prose)
            .write("assets/logo.png", [0x89, b'P', b'N', b'G', 0, 1, 2, 3])
            .commit("add");
        fixture
            .remove("docs/old.md")
            .write("docs/new.md", prose)
            .write("assets/logo.png", [0x89, b'P', b'N', b'G', 0, 9, 9, 9])
            .commit("move and edit");

        let commits = fixture.collect(&RecentCommitsOptions::new().count(1)).unwrap();

        assert_eq!(
            commits.to_json()[0]["files"],
            json!([
                {"kind": "modified", "path": "assets/logo.png"},
                {"kind": "moved", "path": "docs/new.md", "original_path": "docs/old.md", "added": 0, "removed": 0}
            ])
        );
        assert!(commits.commits()[0].file_types.images);
        assert!(commits.commits()[0].file_types.documentation);

        // A copy of a file modified in the same commit is `moved` from it, and
        // the edited source keeps its own `modified` record.
        fixture
            .write("docs/new.md", format!("{prose}line five\n"))
            .write("docs/copy.md", prose)
            .commit("edit and copy");
        let commits = fixture.collect(&RecentCommitsOptions::new().count(1)).unwrap();
        let files: Vec<(RecentCommitFileKind, &str, Option<&str>, bool)> = commits.commits()[0]
            .files
            .iter()
            .map(|file| {
                (
                    file.kind,
                    file.path.as_str(),
                    file.original_path.as_deref(),
                    file.added.is_some() && file.removed.is_some(),
                )
            })
            .collect();
        assert_eq!(
            files,
            [
                (RecentCommitFileKind::Moved, "docs/copy.md", Some("docs/new.md"), true),
                (RecentCommitFileKind::Modified, "docs/new.md", None, true),
            ]
        );
    }

    #[test]
    fn merges_are_included_with_first_parent_files_and_no_change_merges_have_none() {
        let fixture = Fixture::new();
        let base = fixture.write("a.txt", "a\n").commit("base");
        fixture.write("side.txt", "side\n");
        let side = fixture.commit_on("refs/heads/side", "side", ADA, BASE_SECONDS + 600, 0, &[base]);
        fixture.remove("side.txt");
        let main = fixture.write("main.txt", "main\n").commit_as("main", ADA, BASE_SECONDS + 1_200, 0);
        fixture.write("side.txt", "side\n");
        let merge = fixture.commit_on("HEAD", "Merge branch 'side'", ADA, BASE_SECONDS + 1_800, 0, &[main, side]);
        // Same tree as its first parent: merging an already-merged branch.
        fixture.commit_on("HEAD", "Merge branch 'side' again", ADA, BASE_SECONDS + 2_400, 0, &[merge, side]);

        let commits = fixture.collect(&RecentCommitsOptions::new().count(3)).unwrap();
        let json = commits.to_json();

        assert_eq!(headings(&commits), ["Merge branch 'side' again", "Merge branch 'side'", "main"]);
        assert_eq!(json[0]["files"], json!([]));
        assert_eq!(json[1]["files"], json!([{"kind": "added", "path": "side.txt", "added": 1, "removed": 0}]));
    }

    #[test]
    fn non_utf8_author_and_message_bytes_are_replaced_not_rejected() {
        let fixture = Fixture::new();
        let tree = fixture.repo.treebuilder(None).unwrap().write().unwrap();
        let mut raw = format!("tree {tree}\n").into_bytes();
        raw.extend_from_slice(b"author J\xf6rg <j\xf6rg@example.com> 1763158400 +0000\n");
        raw.extend_from_slice(b"committer J\xf6rg <j\xf6rg@example.com> 1763158400 +0000\n");
        raw.extend_from_slice(b"encoding ISO-8859-1\n\nfix: caf\xe9 au lait\n");
        let id = fixture.repo.odb().unwrap().write(ObjectType::Commit, &raw).unwrap();
        fixture.set_ref("refs/heads/main", id);
        fixture.repo.set_head("refs/heads/main").unwrap();

        let commits = fixture.collect(&RecentCommitsOptions::new()).unwrap();
        let commit = &commits.commits()[0];

        assert_eq!(commit.author.name, "J\u{FFFD}rg");
        assert_eq!(commit.author.email, "j\u{FFFD}rg@example.com");
        assert_eq!(commit.operation.as_deref(), Some("fix"));
        assert_eq!(commit.heading, "caf\u{FFFD} au lait");
        assert_eq!(commit.hash, id.to_string());
    }
}

mod attribution {
    use super::*;

    /// A Cargo workspace with a package nested inside another, a second
    /// package, and a file directly under the `crates` area.
    fn monorepo() -> Fixture {
        let fixture = Fixture::new();
        fixture
            .write(
                "Cargo.toml",
                "[workspace]\nmembers = [\"crates/alpha\", \"crates/alpha/nested\", \"crates/beta\"]\n",
            )
            .write("crates/alpha/Cargo.toml", "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2024\"\n")
            .write("crates/alpha/src/lib.rs", "pub fn alpha() {}\n")
            .write("crates/alpha/nested/Cargo.toml", "[package]\nname = \"nested\"\nversion = \"0.1.0\"\nedition = \"2024\"\n")
            .write("crates/alpha/nested/src/lib.rs", "pub fn nested() {}\n")
            .write("crates/beta/Cargo.toml", "[package]\nname = \"beta\"\nversion = \"0.1.0\"\nedition = \"2024\"\n")
            .write("crates/beta/src/lib.rs", "pub fn beta() {}\n")
            .commit("initial workspace");
        fixture
            .write("crates/alpha/nested/src/lib.rs", "pub fn nested() { /* v2 */ }\n")
            .commit("nested only");
        fixture.write("crates/README.md", "# crates\n").commit("area root file");
        fixture
            .write("crates/beta/src/lib.rs", "pub fn beta() { /* v2 */ }\n")
            .write("README.md", "# root\n")
            .commit("beta and root readme");
        fixture
    }

    fn packages_of(commits: &RecentCommits) -> Vec<(String, serde_json::Value, serde_json::Value)> {
        let json = commits.to_json();
        commits
            .commits()
            .iter()
            .zip(json.as_array().unwrap())
            .map(|(commit, value)| {
                (
                    commit.heading.clone(),
                    value["packages"].clone(),
                    value["package_areas"].clone(),
                )
            })
            .collect()
    }

    #[test]
    fn monorepo_commits_carry_deepest_package_arrays_and_area_files_stay_unattributed() {
        let fixture = monorepo();

        let commits = fixture.collect(&RecentCommitsOptions::new()).unwrap();

        assert_eq!(
            packages_of(&commits),
            [
                ("beta and root readme".into(), json!(["beta"]), json!(["crates"])),
                ("area root file".into(), json!([]), json!([])),
                ("nested only".into(), json!(["nested"]), json!(["crates/alpha"])),
                (
                    "initial workspace".into(),
                    json!(["alpha", "beta", "nested"]),
                    json!(["crates", "crates/alpha"])
                ),
            ]
        );
    }

    #[test]
    fn non_monorepo_commits_omit_both_attribution_keys() {
        let fixture = Fixture::new();
        fixture
            .write("Cargo.toml", "[package]\nname = \"solo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n")
            .write("src/lib.rs", "pub fn solo() {}\n")
            .commit("solo");

        let json = fixture.collect(&RecentCommitsOptions::new()).unwrap().to_json();

        let object = json[0].as_object().unwrap();
        assert!(!object.contains_key("packages"));
        assert!(!object.contains_key("package_areas"));
    }

    #[test]
    fn package_and_area_filters_use_the_same_deepest_owner_as_attribution() {
        let fixture = monorepo();
        let select = |options: RecentCommitsOptions| {
            let commits = fixture.collect(&options).unwrap();
            headings(&commits).iter().map(|h| h.to_string()).collect::<Vec<_>>()
        };

        assert_eq!(select(RecentCommitsOptions::new().package("ALPHA")), ["initial workspace"]);
        assert_eq!(
            select(RecentCommitsOptions::new().package("nested")),
            ["nested only", "initial workspace"]
        );
        assert_eq!(
            select(RecentCommitsOptions::new().package_area("crates")),
            ["beta and root readme", "nested only", "initial workspace"]
        );
        assert_eq!(
            select(RecentCommitsOptions::new().package_area("crates/alpha")),
            ["nested only", "initial workspace"]
        );
        assert_eq!(
            select(RecentCommitsOptions::new().package("beta").package_area("crates/alpha")),
            ["initial workspace"],
            "package and area filters AND together"
        );
    }

    #[test]
    fn unknown_package_or_area_is_a_typed_error_listing_valid_names() {
        let fixture = monorepo();

        let error = fixture
            .collect(&RecentCommitsOptions::new().package("gamma"))
            .unwrap_err();
        assert!(
            matches!(&error, SniffError::UnknownPackage { name, valid } if name == "gamma" && valid == "alpha, beta, nested"),
            "{error:?}"
        );

        let error = fixture
            .collect(&RecentCommitsOptions::new().package_area("apps"))
            .unwrap_err();
        assert!(
            matches!(&error, SniffError::UnknownPackageArea { area, valid } if area == "apps" && valid == "crates, crates/alpha"),
            "{error:?}"
        );
    }

    #[test]
    fn package_filter_outside_a_monorepo_is_not_a_monorepo_error() {
        let fixture = Fixture::new();
        fixture.write("a.txt", "a").commit("first");

        let error = fixture
            .collect(&RecentCommitsOptions::new().package("anything"))
            .unwrap_err();

        assert!(matches!(error, SniffError::NotAMonorepo(_)), "{error:?}");
    }

    #[test]
    fn attribution_uses_the_structure_tier_without_inventory_language_or_doc_walks() {
        let fixture = monorepo();

        let (result, counts) = fixture.collect_counted(&RecentCommitsOptions::new().package_area("crates"));
        result.unwrap();

        for name in [
            counters::FS_INVENTORY_ACCEPTED,
            counters::FS_DOCS_PARSED,
            counters::REPO_PACKAGE_ENRICHMENTS,
            counters::REPO_LOCKFILE_PARSES,
            counters::PROC_SPAWNS,
            counters::REMOTE_REQUESTS,
        ] {
            assert_eq!(counter(&counts, name), 0, "{name}: {counts:?}");
        }
        assert!(counter(&counts, counters::REPO_MANIFEST_PARSES) > 0, "{counts:?}");

        // Control: full-tier detection of the same tree does perform the
        // enrichment work the assertions above rule out.
        let collector = PerformanceCollector::new_shared();
        with_current_collector(Some(Arc::clone(&collector)), || detect_repo(fixture.path()).unwrap());
        let full = collector.snapshot(StdDuration::ZERO).counters;
        assert!(counter(&full, counters::REPO_PACKAGE_ENRICHMENTS) > 0, "{full:?}");
    }
}

mod linking {
    use super::*;

    #[test]
    fn pushed_commits_link_to_the_containing_remote_without_network_work() {
        let fixture = Fixture::new();
        fixture.repo.remote("origin", "git@github.com:octo/repo.git").unwrap();
        let pushed = fixture.write("a.txt", "a").commit("pushed");
        fixture.set_ref("refs/remotes/origin/main", pushed);
        let unpushed = fixture.write("b.txt", "b").commit("unpushed");

        let (result, counts) = fixture.collect_counted(&RecentCommitsOptions::new());
        let json = result.unwrap().to_json();

        assert_eq!(json[0]["hash"], json!(unpushed.to_string()));
        assert_eq!(json[0]["remote"], json!(false));
        assert!(json[0].get("commit_url").is_none());
        assert_eq!(json[1]["remote"], json!(true));
        assert_eq!(
            json[1]["commit_url"],
            json!(format!("https://github.com/octo/repo/commit/{pushed}"))
        );

        // One ref observation for the whole collection, and no fetch, child
        // process, or provider request.
        assert_eq!(counter(&counts, counters::GIT_REF_WALKS), 1, "{counts:?}");
        assert_eq!(counter(&counts, counters::PROC_SPAWNS), 0);
        assert_eq!(counter(&counts, counters::REMOTE_REQUESTS), 0);
    }

    #[test]
    fn empty_collection_performs_no_linking() {
        let fixture = Fixture::new();
        fixture.repo.remote("origin", "git@github.com:octo/repo.git").unwrap();
        let only = fixture.write("a.txt", "a").commit("chore: only");
        fixture.set_ref("refs/remotes/origin/main", only);

        let (result, counts) = fixture.collect_counted(&RecentCommitsOptions::new().operation("fix"));

        assert!(result.unwrap().is_empty());
        assert_eq!(counter(&counts, counters::GIT_REF_WALKS), 0, "{counts:?}");
    }

    #[test]
    fn self_hosted_remote_is_contained_without_a_commit_url() {
        let fixture = Fixture::new();
        fixture.repo.remote("origin", "https://git.example.com/team/app.git").unwrap();
        let pushed = fixture.write("a.txt", "a").commit("pushed");
        fixture.set_ref("refs/remotes/origin/main", pushed);

        let json = fixture.collect(&RecentCommitsOptions::new()).unwrap().to_json();

        assert_eq!(json[0]["remote"], json!(true));
        assert!(json[0].get("commit_url").is_none());
    }
}

/// Serialization of a collected result survives a read/write/read round trip
/// unchanged, including `null` remote and omitted optional keys.
#[test]
fn collected_json_round_trips() {
    let fixture = Fixture::new();
    fixture.write("a.md", "a\n").commit("docs: first. More.\n\n- one");
    let collected = fixture.collect(&RecentCommitsOptions::new()).unwrap();

    let first = serde_json::to_string(&collected).unwrap();
    let reread: RecentCommits = serde_json::from_str(&first).unwrap();
    let second = serde_json::to_string(&reread).unwrap();

    // The collection root is report context, not payload, so compare commits.
    assert_eq!(reread.commits(), collected.commits());
    assert_eq!(second, first);
    assert_eq!(
        DateTime::parse_from_rfc3339(reread.to_json()[0]["datetime"].as_str().unwrap())
            .unwrap()
            .offset()
            .local_minus_utc(),
        0
    );
    assert_eq!(reread.commits()[0].files[0].kind, RecentCommitFileKind::Added);
}
