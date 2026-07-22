use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use git2::{
    build::CheckoutBuilder, IndexAddOption, Oid, Repository, RepositoryInitOptions, ResetType,
};
use sniff::{
    SniffError, filesystem::git::merge_conflicts_with_branch_at,
    programs::find_program_with_source,
};
use tempfile::TempDir;

#[derive(Clone, Copy)]
enum Change<'a> {
    Write(&'a str, &'a str),
    Remove(&'a str),
    Rename(&'a str, &'a str),
}

struct Fixture {
    _dir: TempDir,
    repo: Repository,
}

impl Fixture {
    fn new(files: &[(&str, &str)]) -> Self {
        let dir = TempDir::new().expect("temp repository");
        let mut options = RepositoryInitOptions::new();
        options.initial_head("main");
        let repo = Repository::init_opts(dir.path(), &options).expect("initialize repository");
        let fixture = Self { _dir: dir, repo };
        let changes = files
            .iter()
            .map(|(path, content)| Change::Write(path, content))
            .collect::<Vec<_>>();
        fixture.commit("base", &changes, &[]);
        fixture
    }

    fn path(&self) -> &Path {
        self.repo.workdir().expect("non-bare fixture")
    }

    fn head(&self) -> Oid {
        self.repo
            .head()
            .expect("HEAD")
            .peel_to_commit()
            .expect("HEAD commit")
            .id()
    }

    fn create_branch(&self, name: &str, target: Oid) {
        let commit = self.repo.find_commit(target).expect("branch target");
        self.repo.branch(name, &commit, false).expect("create branch");
    }

    fn checkout(&self, name: &str) {
        self.repo
            .set_head(&format!("refs/heads/{name}"))
            .expect("attach HEAD");
        self.repo
            .checkout_head(Some(CheckoutBuilder::new().force()))
            .expect("checkout branch");
    }

    fn commit(&self, message: &str, changes: &[Change<'_>], extra_parents: &[Oid]) -> Oid {
        let mut index = self.repo.index().expect("repository index");
        for change in changes {
            match *change {
                Change::Write(path, content) => {
                    let absolute = self.path().join(path);
                    if let Some(parent) = absolute.parent() {
                        fs::create_dir_all(parent).expect("create parent directory");
                    }
                    fs::write(&absolute, content).expect("write fixture file");
                    index.add_path(Path::new(path)).expect("stage fixture file");
                }
                Change::Remove(path) => {
                    let absolute = self.path().join(path);
                    if absolute.is_dir() {
                        fs::remove_dir_all(&absolute).expect("remove fixture directory");
                    } else if absolute.exists() {
                        fs::remove_file(&absolute).expect("remove fixture file");
                    }
                    index.remove_path(Path::new(path)).expect("stage removal");
                }
                Change::Rename(from, to) => {
                    let source = self.path().join(from);
                    let destination = self.path().join(to);
                    if let Some(parent) = destination.parent() {
                        fs::create_dir_all(parent).expect("create rename parent");
                    }
                    fs::rename(&source, &destination).expect("rename fixture file");
                    index.remove_path(Path::new(from)).expect("stage rename source");
                    index.add_path(Path::new(to)).expect("stage rename destination");
                }
            }
        }
        index
            .add_all(["*"], IndexAddOption::DEFAULT, None)
            .expect("stage fixture tree");
        index.write().expect("write fixture index");
        let tree_id = index.write_tree().expect("write fixture tree");
        let tree = self.repo.find_tree(tree_id).expect("fixture tree");
        let signature = git2::Signature::now("Test", "test@example.com").expect("signature");
        let head_parent = self.repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let mut parents = Vec::new();
        if let Some(parent) = head_parent.as_ref() {
            parents.push(parent);
        }
        let extra = extra_parents
            .iter()
            .map(|id| self.repo.find_commit(*id).expect("extra parent"))
            .collect::<Vec<_>>();
        parents.extend(extra.iter());
        self.repo
            .commit(Some("HEAD"), &signature, &signature, message, &tree, &parents)
            .expect("commit fixture")
    }

    fn diverged(
        files: &[(&str, &str)],
        ours: &[Change<'_>],
        theirs: &[Change<'_>],
    ) -> Self {
        let fixture = Self::new(files);
        let base = fixture.head();
        fixture.create_branch("incoming", base);
        fixture.commit("ours", ours, &[]);
        fixture.checkout("incoming");
        fixture.commit("theirs", theirs, &[]);
        fixture.checkout("main");
        fixture
    }
}

fn git2_conflict_paths(repo: &Repository, ours: Oid, theirs: Oid) -> Vec<PathBuf> {
    let ours = repo.find_commit(ours).expect("ours commit");
    let theirs = repo.find_commit(theirs).expect("theirs commit");
    let index = repo
        .merge_commits(&ours, &theirs, None)
        .expect("git2 merge oracle");
    let mut paths = BTreeSet::new();
    for conflict in index.conflicts().expect("git2 conflict iterator") {
        let conflict = conflict.expect("git2 conflict");
        for entry in [conflict.ancestor, conflict.our, conflict.their]
            .into_iter()
            .flatten()
        {
            paths.insert(PathBuf::from(String::from_utf8_lossy(&entry.path).as_ref()));
        }
    }
    paths.into_iter().collect()
}

fn branch_tip(repo: &Repository, branch: &str) -> Oid {
    repo.find_reference(&format!("refs/heads/{branch}"))
        .expect("branch reference")
        .peel_to_commit()
        .expect("branch commit")
        .id()
}

fn canonical_git_conflict_paths(fixture: &Fixture) -> Option<Vec<PathBuf>> {
    let (git, _) = find_program_with_source("git")?;
    let merge = Command::new(&git)
        .args(["-C"])
        .arg(fixture.path())
        .args(["merge", "--no-commit", "--no-ff", "--no-verify", "incoming"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("run canonical git merge");
    assert!(
        merge.status.success() || merge.status.code() == Some(1),
        "canonical git merge failed unexpectedly: {}",
        String::from_utf8_lossy(&merge.stderr)
    );
    let output = Command::new(git)
        .args(["-C"])
        .arg(fixture.path())
        .args(["ls-files", "--unmerged", "-z"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("read canonical git conflict index");
    assert!(output.status.success(), "canonical git ls-files failed");
    let mut paths = output
        .stdout
        .split(|byte| *byte == 0)
        .filter_map(|entry| {
            entry
                .iter()
                .position(|byte| *byte == b'\t')
                .map(|tab| &entry[tab + 1..])
        })
        .map(|path| {
            let path = String::from_utf8_lossy(path).replace("~incoming", "~MERGE_HEAD");
            PathBuf::from(path)
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    Some(paths)
}

fn criss_cross_fixture() -> Fixture {
    let fixture = Fixture::new(&[("a.txt", "base\n"), ("b.txt", "base\n")]);
    let base = fixture.head();
    fixture.create_branch("incoming", base);

    let a1 = fixture.commit("a1", &[Change::Write("a.txt", "a1\n")], &[]);
    fixture.checkout("incoming");
    let b1 = fixture.commit("b1", &[Change::Write("b.txt", "b1\n")], &[]);

    fixture.checkout("main");
    fixture.commit("a2", &[Change::Write("b.txt", "b1\n")], &[b1]);
    fixture.checkout("incoming");
    fixture.commit("b2", &[Change::Write("a.txt", "a1\n")], &[a1]);
    fixture.checkout("main");
    fixture
}

#[test]
fn public_api_matches_git2_index_oracle_for_core_conflicts() {
    let cases = [
        (
            "clean",
            vec![("ours.txt", "base\n"), ("theirs.txt", "base\n")],
            vec![Change::Write("ours.txt", "ours\n")],
            vec![Change::Write("theirs.txt", "theirs\n")],
        ),
        (
            "content",
            vec![("shared.txt", "base\n")],
            vec![Change::Write("shared.txt", "ours\n")],
            vec![Change::Write("shared.txt", "theirs\n")],
        ),
        (
            "add/add",
            vec![],
            vec![Change::Write("added.txt", "ours\n")],
            vec![Change::Write("added.txt", "theirs\n")],
        ),
        (
            "modify/delete",
            vec![("removed.txt", "base\n")],
            vec![Change::Write("removed.txt", "ours\n")],
            vec![Change::Remove("removed.txt")],
        ),
    ];

    for (name, files, ours, theirs) in cases {
        let fixture = Fixture::diverged(&files, &ours, &theirs);
        let expected = git2_conflict_paths(
            &fixture.repo,
            branch_tip(&fixture.repo, "main"),
            branch_tip(&fixture.repo, "incoming"),
        );
        let actual = merge_conflicts_with_branch_at(fixture.path(), "incoming")
            .unwrap_or_else(|error| panic!("{name}: prediction failed: {error}"));
        assert_eq!(actual, expected, "{name}");
    }
}

#[test]
fn structural_conflicts_preserve_every_temporary_index_path() {
    let fixture = Fixture::diverged(
        &[("old.txt", "base\n")],
        &[Change::Rename("old.txt", "ours.txt")],
        &[Change::Rename("old.txt", "theirs.txt")],
    );
    let actual = merge_conflicts_with_branch_at(fixture.path(), "incoming").expect("prediction");
    assert_eq!(
        actual,
        vec![
            PathBuf::from("old.txt"),
            PathBuf::from("ours.txt"),
            PathBuf::from("theirs.txt"),
        ]
    );

    let ours_change_locations = BTreeSet::from([
        PathBuf::from("old.txt"),
        PathBuf::from("ours.txt"),
    ]);
    assert_ne!(
        actual.into_iter().collect::<BTreeSet<_>>(),
        ours_change_locations,
        "one side's changed paths are not a valid conflict-path authority"
    );
}

#[test]
fn conflict_paths_are_direction_sensitive() {
    let fixture = Fixture::diverged(
        &[],
        &[Change::Write("node/child.txt", "ours child\n")],
        &[Change::Write("node", "theirs file\n")],
    );
    let theirs_into_ours =
        merge_conflicts_with_branch_at(fixture.path(), "incoming").expect("theirs into ours");
    fixture.checkout("incoming");
    let ours_into_theirs =
        merge_conflicts_with_branch_at(fixture.path(), "main").expect("ours into theirs");
    assert_eq!(theirs_into_ours, vec![PathBuf::from("node~MERGE_HEAD")]);
    assert_eq!(ours_into_theirs, vec![PathBuf::from("node")]);
    assert_ne!(theirs_into_ours, ours_into_theirs);
}

#[test]
fn multi_path_conflicts_are_sorted_deduplicated_and_portable() {
    let fixture = Fixture::diverged(
        &[("nested/z.txt", "base\n"), ("a.txt", "base\n")],
        &[
            Change::Write("nested/z.txt", "ours z\n"),
            Change::Write("a.txt", "ours a\n"),
        ],
        &[
            Change::Write("nested/z.txt", "theirs z\n"),
            Change::Write("a.txt", "theirs a\n"),
        ],
    );
    assert_eq!(
        merge_conflicts_with_branch_at(fixture.path(), "incoming").expect("multi-path prediction"),
        vec![PathBuf::from("a.txt"), PathBuf::from("nested/z.txt")]
    );
}

#[test]
fn structural_fixtures_match_canonical_git_when_available() {
    let fixtures = [
        Fixture::diverged(
            &[("old.txt", "base\n")],
            &[Change::Rename("old.txt", "ours.txt")],
            &[Change::Rename("old.txt", "theirs.txt")],
        ),
        Fixture::diverged(
            &[],
            &[Change::Write("node/child.txt", "ours child\n")],
            &[Change::Write("node", "theirs file\n")],
        ),
        Fixture::diverged(
            &[("nested/z.txt", "base\n"), ("a.txt", "base\n")],
            &[
                Change::Write("nested/z.txt", "ours z\n"),
                Change::Write("a.txt", "ours a\n"),
            ],
            &[
                Change::Write("nested/z.txt", "theirs z\n"),
                Change::Write("a.txt", "theirs a\n"),
            ],
        ),
        criss_cross_fixture(),
    ];

    for fixture in fixtures {
        let actual = merge_conflicts_with_branch_at(fixture.path(), "incoming")
            .expect("structural prediction");
        let Some(expected) = canonical_git_conflict_paths(&fixture) else {
            return;
        };
        assert_eq!(actual, expected);
    }
}

#[test]
fn clean_and_no_op_merges_return_empty_and_accept_exact_branch_spellings() {
    let fixture = Fixture::diverged(
        &[("ours.txt", "base\n"), ("theirs.txt", "base\n")],
        &[Change::Write("ours.txt", "ours\n")],
        &[Change::Write("theirs.txt", "theirs\n")],
    );
    assert!(
        merge_conflicts_with_branch_at(fixture.path(), "incoming")
            .expect("clean divergent merge")
            .is_empty()
    );
    assert_eq!(
        merge_conflicts_with_branch_at(fixture.path(), "incoming").expect("short branch"),
        merge_conflicts_with_branch_at(fixture.path(), "refs/heads/incoming")
            .expect("fully qualified branch")
    );
    assert!(
        merge_conflicts_with_branch_at(fixture.path(), "main")
            .expect("same branch")
            .is_empty()
    );

    let ancestor = Fixture::new(&[("file.txt", "base\n")]);
    let base = ancestor.head();
    ancestor.create_branch("ancestor", base);
    ancestor.commit("main ahead", &[Change::Write("file.txt", "ahead\n")], &[]);
    assert!(
        merge_conflicts_with_branch_at(ancestor.path(), "ancestor")
            .expect("already-contained branch")
            .is_empty()
    );
    ancestor.checkout("ancestor");
    assert!(
        merge_conflicts_with_branch_at(ancestor.path(), "main")
            .expect("fast-forward branch")
            .is_empty()
    );
}

#[test]
fn prediction_preconditions_are_errors() {
    let outside = TempDir::new().expect("non-repository directory");
    assert!(merge_conflicts_with_branch_at(outside.path(), "main").is_err());

    let unborn_dir = TempDir::new().expect("unborn repository directory");
    Repository::init(unborn_dir.path()).expect("unborn repository");
    assert!(merge_conflicts_with_branch_at(unborn_dir.path(), "main").is_err());

    let detached = Fixture::new(&[("file.txt", "base\n")]);
    detached
        .repo
        .set_head_detached(detached.head())
        .expect("detach HEAD");
    assert!(merge_conflicts_with_branch_at(detached.path(), "main").is_err());

    let fixture = Fixture::new(&[("file.txt", "base\n")]);
    for invalid in [
        "missing",
        "refs/heads/../main",
        " main",
        "HEAD",
        "refs/tags/main",
        &fixture.head().to_string(),
    ] {
        assert!(
            merge_conflicts_with_branch_at(fixture.path(), invalid).is_err(),
            "{invalid:?} must not resolve through DWIM"
        );
    }

    let unrelated = Fixture::new(&[("main.txt", "main\n")]);
    let signature = git2::Signature::now("Test", "test@example.com").expect("signature");
    let blob = unrelated.repo.blob(b"orphan\n").expect("orphan blob");
    let mut builder = unrelated.repo.treebuilder(None).expect("orphan tree builder");
    builder
        .insert("orphan.txt", blob, 0o100644)
        .expect("orphan tree entry");
    let tree_id = builder.write().expect("orphan tree");
    let tree = unrelated.repo.find_tree(tree_id).expect("orphan tree object");
    let orphan = unrelated
        .repo
        .commit(None, &signature, &signature, "orphan", &tree, &[])
        .expect("orphan commit");
    unrelated.create_branch("unrelated", orphan);
    assert!(merge_conflicts_with_branch_at(unrelated.path(), "unrelated").is_err());

    let corrupt = Fixture::diverged(
        &[("file.txt", "base\n")],
        &[Change::Write("file.txt", "ours\n")],
        &[Change::Write("file.txt", "theirs\n")],
    );
    let incoming = branch_tip(&corrupt.repo, "incoming").to_string();
    let object_path = corrupt
        .repo
        .path()
        .join("objects")
        .join(&incoming[..2])
        .join(&incoming[2..]);
    let mut permissions = fs::metadata(&object_path)
        .expect("incoming object metadata")
        .permissions();
    permissions.set_readonly(false);
    fs::set_permissions(&object_path, permissions).expect("make incoming object writable");
    fs::write(object_path, b"corrupt").expect("corrupt incoming commit");
    assert!(merge_conflicts_with_branch_at(corrupt.path(), "incoming").is_err());
}

#[test]
fn unsafe_merge_configuration_is_rejected_without_launching_commands() {
    for (attribute, setting) in [
        ("merge=unsafe", "merge.unsafe.driver"),
        ("filter=unsafe", "filter.unsafe.process"),
    ] {
        let attributes = format!("shared.txt {attribute}\n");
        let fixture = Fixture::diverged(
            &[
                (".gitattributes", attributes.as_str()),
                ("shared.txt", "base\n"),
            ],
            &[Change::Write("shared.txt", "ours\n")],
            &[Change::Write("shared.txt", "theirs\n")],
        );
        fixture
            .repo
            .config()
            .expect("repository config")
            .set_str(setting, "definitely-not-an-executable %O %A %B")
            .expect("unsafe merge setting");
        let error = merge_conflicts_with_branch_at(fixture.path(), "incoming")
            .expect_err("external command configuration must be rejected");
        assert!(matches!(
            error,
            SniffError::UnsupportedMergeConfiguration {
                setting: actual,
                path,
            } if actual == setting && path == Path::new("shared.txt")
        ));
    }

    let renormalize = Fixture::diverged(
        &[("shared.txt", "base\n")],
        &[Change::Write("shared.txt", "ours\n")],
        &[Change::Write("shared.txt", "theirs\n")],
    );
    renormalize
        .repo
        .config()
        .expect("repository config")
        .set_bool("merge.renormalize", true)
        .expect("renormalize setting");
    assert!(matches!(
        merge_conflicts_with_branch_at(renormalize.path(), "incoming"),
        Err(SniffError::UnsupportedMergeConfiguration { setting, path })
            if setting == "merge.renormalize" && path == Path::new("shared.txt")
    ));

    let builtin = Fixture::diverged(
        &[
            (".gitattributes", "shared.txt merge=text\n"),
            ("shared.txt", "base\n"),
        ],
        &[Change::Write("shared.txt", "ours\n")],
        &[Change::Write("shared.txt", "theirs\n")],
    );
    assert_eq!(
        merge_conflicts_with_branch_at(builtin.path(), "incoming")
            .expect("built-in text merge remains supported"),
        vec![PathBuf::from("shared.txt")]
    );
}

/// Rejecting unsupported settings must not swallow the merges that cannot
/// reach a content merge at all: git takes one side wholesale, so no external
/// behavior is in play no matter what the repository configures.
#[test]
fn trivial_merges_return_empty_despite_unsupported_configuration() {
    let enable_renormalize = |fixture: &Fixture| {
        fixture
            .repo
            .config()
            .expect("repository config")
            .set_bool("merge.renormalize", true)
            .expect("renormalize setting");
    };

    let same_branch = Fixture::new(&[("file.txt", "base\n")]);
    enable_renormalize(&same_branch);
    assert_eq!(
        merge_conflicts_with_branch_at(same_branch.path(), "main").expect("same-branch merge"),
        Vec::<PathBuf>::new()
    );

    // `main` is ahead of `ancestor`, so the two directions cover the
    // already-contained and fast-forward shapes. Both diverge in content from
    // the merge base, so a participating-path count alone would not exempt them.
    let ancestor = Fixture::new(&[("file.txt", "base\n")]);
    let base = ancestor.head();
    ancestor.create_branch("ancestor", base);
    ancestor.commit("main ahead", &[Change::Write("file.txt", "ahead\n")], &[]);
    enable_renormalize(&ancestor);
    assert_eq!(
        merge_conflicts_with_branch_at(ancestor.path(), "ancestor")
            .expect("already-contained merge"),
        Vec::<PathBuf>::new()
    );
    ancestor.checkout("ancestor");
    assert_eq!(
        merge_conflicts_with_branch_at(ancestor.path(), "main").expect("fast-forward merge"),
        Vec::<PathBuf>::new()
    );
}

/// A clean built-in merge is not evidence that the real merge is clean: the
/// approximation ignores exactly the external behavior that would decide it.
#[test]
fn unsafe_merge_configuration_is_rejected_when_the_builtin_merge_is_clean() {
    // Each side touches a different file, so the built-in merge never conflicts.
    // `.gitattributes` is committed in the base and unchanged on both sides,
    // leaving `ours.txt` and `theirs.txt` as the participating paths.
    let clean_fixture = |attributes: &str| {
        Fixture::diverged(
            &[
                (".gitattributes", attributes),
                ("ours.txt", "base\n"),
                ("theirs.txt", "base\n"),
            ],
            &[Change::Write("ours.txt", "ours\n")],
            &[Change::Write("theirs.txt", "theirs\n")],
        )
    };

    let mut cases = vec![(
        "ours.txt merge=unsafe\n".to_string(),
        "merge.unsafe.driver".to_string(),
    )];
    cases.extend(["process", "clean", "smudge"].map(|suffix| {
        (
            "ours.txt filter=unsafe\n".to_string(),
            format!("filter.unsafe.{suffix}"),
        )
    }));

    for (attributes, setting) in cases {
        let fixture = clean_fixture(&attributes);
        assert_eq!(
            merge_conflicts_with_branch_at(fixture.path(), "incoming")
                .expect("baseline prediction"),
            Vec::<PathBuf>::new(),
            "{setting}: fixture must be clean under the built-in driver"
        );

        fixture
            .repo
            .config()
            .expect("repository config")
            .set_str(&setting, "definitely-not-an-executable %O %A %B")
            .expect("unsafe merge setting");
        let before = snapshot(&fixture);
        let error = merge_conflicts_with_branch_at(fixture.path(), "incoming")
            .expect_err("a clean built-in merge must not mask external command configuration");
        assert!(
            matches!(
                &error,
                SniffError::UnsupportedMergeConfiguration {
                    setting: actual,
                    path,
                } if *actual == setting && path == Path::new("ours.txt")
            ),
            "{setting}: unexpected error {error}"
        );
        assert_eq!(
            snapshot(&fixture),
            before,
            "{setting}: rejection must not mutate the repository"
        );
    }

    let renormalize = clean_fixture("");
    assert_eq!(
        merge_conflicts_with_branch_at(renormalize.path(), "incoming")
            .expect("baseline prediction"),
        Vec::<PathBuf>::new()
    );
    renormalize
        .repo
        .config()
        .expect("repository config")
        .set_bool("merge.renormalize", true)
        .expect("renormalize setting");
    let before = snapshot(&renormalize);
    assert!(matches!(
        merge_conflicts_with_branch_at(renormalize.path(), "incoming"),
        Err(SniffError::UnsupportedMergeConfiguration { setting, path })
            if setting == "merge.renormalize" && path == Path::new("ours.txt")
    ));
    assert_eq!(snapshot(&renormalize), before);

    let builtin = clean_fixture("ours.txt merge=text\n");
    assert_eq!(
        merge_conflicts_with_branch_at(builtin.path(), "incoming")
            .expect("built-in driver without renormalize remains supported"),
        Vec::<PathBuf>::new()
    );
}

#[test]
fn live_index_and_worktree_state_do_not_affect_prediction() {
    let fixture = Fixture::diverged(
        &[
            (".gitattributes", "shared.txt merge=text\n"),
            ("shared.txt", "base\n"),
        ],
        &[Change::Write("shared.txt", "ours\n")],
        &[Change::Write("shared.txt", "theirs\n")],
    );
    let expected = merge_conflicts_with_branch_at(fixture.path(), "incoming")
        .expect("baseline prediction");

    let incoming = fixture
        .repo
        .find_annotated_commit(branch_tip(&fixture.repo, "incoming"))
        .expect("annotated incoming commit");
    fixture
        .repo
        .merge(&[&incoming], None, None)
        .expect("create conflicted live index");
    assert!(
        fixture.repo.index().expect("conflicted index").has_conflicts(),
        "fixture must contain unresolved live-index stages"
    );
    assert_eq!(
        merge_conflicts_with_branch_at(fixture.path(), "incoming")
            .expect("prediction with conflicted live index"),
        expected
    );
    let ours = fixture
        .repo
        .find_object(branch_tip(&fixture.repo, "main"), None)
        .expect("ours object");
    fixture
        .repo
        .reset(&ours, ResetType::Hard, None)
        .expect("restore live index and worktree");
    fixture.repo.cleanup_state().expect("clear merge state");

    fs::write(
        fixture.path().join(".gitattributes"),
        "shared.txt merge=unsafe filter=unsafe\n",
    )
    .expect("modify attributes in worktree");
    fs::write(fixture.path().join("shared.txt"), "unstaged content\n")
        .expect("modify tracked file");
    fs::write(fixture.path().join("untracked.txt"), "untracked\n")
        .expect("create untracked file");
    let mut index = fixture.repo.index().expect("live index");
    index
        .add_path(Path::new(".gitattributes"))
        .expect("stage attributes");
    index.write().expect("write staged attributes");
    assert_eq!(
        merge_conflicts_with_branch_at(fixture.path(), "incoming")
            .expect("prediction with staged and dirty state"),
        expected
    );

    fs::write(fixture.repo.path().join("index"), b"corrupt live index")
        .expect("corrupt live index");
    assert_eq!(
        merge_conflicts_with_branch_at(fixture.path(), "incoming")
            .expect("prediction with corrupt live index"),
        expected
    );
    fs::remove_file(fixture.repo.path().join("index")).expect("remove live index");
    assert_eq!(
        merge_conflicts_with_branch_at(fixture.path(), "incoming")
            .expect("prediction with missing live index"),
        expected
    );
}

#[derive(Debug, PartialEq, Eq)]
struct RepositorySnapshot {
    head: Vec<u8>,
    refs: BTreeMap<String, String>,
    index: Vec<u8>,
    files: BTreeMap<String, Vec<u8>>,
    objects: BTreeSet<String>,
}

fn snapshot(fixture: &Fixture) -> RepositorySnapshot {
    let git_dir = fixture.repo.path();
    let head = fs::read(git_dir.join("HEAD")).expect("HEAD bytes");
    let refs = fixture
        .repo
        .references()
        .expect("references")
        .map(|reference| {
            let reference = reference.expect("reference");
            let name = reference.name().expect("UTF-8 reference").to_string();
            let target = reference
                .target()
                .map(|id| id.to_string())
                .or_else(|| reference.symbolic_target().map(str::to_string))
                .expect("reference target");
            (name, target)
        })
        .collect();
    let index = fs::read(git_dir.join("index")).expect("index bytes");
    let mut files = BTreeMap::new();
    collect_files(fixture.path(), fixture.path(), &mut files);
    let mut objects = BTreeSet::new();
    collect_object_ids(&git_dir.join("objects"), &git_dir.join("objects"), &mut objects);
    RepositorySnapshot {
        head,
        refs,
        index,
        files,
        objects,
    }
}

fn collect_files(root: &Path, directory: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
    for entry in fs::read_dir(directory).expect("read fixture directory") {
        let entry = entry.expect("fixture entry");
        let path = entry.path();
        if path.file_name().is_some_and(|name| name == ".git") {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, out);
        } else {
            let relative = path
                .strip_prefix(root)
                .expect("relative fixture path")
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/");
            out.insert(relative, fs::read(path).expect("fixture file bytes"));
        }
    }
}

fn collect_object_ids(root: &Path, directory: &Path, out: &mut BTreeSet<String>) {
    for entry in fs::read_dir(directory).expect("read object directory") {
        let entry = entry.expect("object entry");
        let path = entry.path();
        if path.is_dir() {
            collect_object_ids(root, &path, out);
        } else {
            out.insert(
                path.strip_prefix(root)
                    .expect("relative object path")
                    .to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "/"),
            );
        }
    }
}

#[test]
fn prediction_is_read_only_for_a_clean_auto_merge() {
    let fixture = Fixture::diverged(
        &[("ours.txt", "base\n"), ("theirs.txt", "base\n")],
        &[Change::Write("ours.txt", "ours\n")],
        &[Change::Write("theirs.txt", "theirs\n")],
    );
    let before = snapshot(&fixture);
    assert!(
        merge_conflicts_with_branch_at(fixture.path(), "incoming")
            .expect("clean prediction")
            .is_empty()
    );
    assert_eq!(snapshot(&fixture), before);
}

#[test]
fn prediction_is_read_only_for_criss_cross_virtual_merge_base() {
    let fixture = criss_cross_fixture();
    let a2 = branch_tip(&fixture.repo, "main");
    let b2 = branch_tip(&fixture.repo, "incoming");

    let gix_repo = gix::open(fixture.path()).expect("gix fixture");
    assert_eq!(
        gix_repo
            .merge_bases_many(
                gix::ObjectId::from_hex(a2.to_string().as_bytes()).expect("a2 id"),
                &[gix::ObjectId::from_hex(b2.to_string().as_bytes()).expect("b2 id")],
            )
            .expect("multiple merge bases")
            .len(),
        2,
        "fixture must force virtual merge-base synthesis"
    );

    let before = snapshot(&fixture);
    assert!(
        merge_conflicts_with_branch_at(fixture.path(), "incoming")
            .expect("criss-cross prediction")
            .is_empty()
    );
    assert_eq!(snapshot(&fixture), before);
}

