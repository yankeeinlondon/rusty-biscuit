//! Repository reads against real repositories: the committed diff (with the
//! Git CLI and `diff_scope.py` as parity oracles), HEAD identity, peeling,
//! ancestry, dirtiness, remote tips against a local bare remote, scope
//! notes, and GitHub remote recognition.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use gix::ObjectId;
use rstest::rstest;
use tempfile::TempDir;
use test_toolkit::{expect_level, Level};

use crate::ci_push::git::{
    attach_note, github_repo_of, utf8_path, ChangedPaths, CommandOutput, CommandRunner,
    DirectRunner, GitError, GithubRemote, RemoteTipError, Repository, TransportCommand,
};

/// A repository in a temporary directory, with the Git CLI isolated from
/// the host's configuration. The CLI is setup and oracle only.
pub(super) struct Fixture {
    dir: TempDir,
    pub root: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gitconfig"), "").unwrap();
        let root = dir.path().join("work");
        let fixture = Fixture { dir, root };
        fixture.git_at(fixture.dir.path(), &["init", "-q", "work"]);
        let mut config = std::fs::OpenOptions::new()
            .append(true)
            .open(fixture.root.join(".git/config"))
            .unwrap();
        config
            .write_all(b"[user]\n\tname = Fixture\n\temail = fixture@example.com\n")
            .unwrap();
        fixture
    }

    /// The temporary directory holding the repository.
    pub fn base(&self) -> &Path {
        self.dir.path()
    }

    /// A path beside the repository, for remotes and worktrees.
    pub fn scratch(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    pub fn command(&self, cwd: &Path) -> Command {
        let mut command = Command::new("git");
        command
            .current_dir(cwd)
            .stdin(Stdio::null())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.dir.path().join("gitconfig"))
            .env("HOME", self.dir.path())
            .args(["-c", "init.defaultBranch=main", "-c", "core.autocrlf=false"]);
        for name in TransportCommand::REMOVED_ENV {
            command.env_remove(name);
        }
        command
    }

    pub fn git_at(&self, cwd: &Path, args: &[&str]) -> Vec<u8> {
        let output = self.command(cwd).args(args).output().unwrap();
        assert_success(&output, args);
        output.stdout
    }

    pub fn git(&self, args: &[&str]) -> Vec<u8> {
        self.git_at(&self.root, args)
    }

    pub fn write(&self, path: &str, contents: &str) {
        let path = self.root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    /// Stages everything and commits it.
    pub fn commit(&self, message: &str) -> ObjectId {
        self.git(&["add", "-A"]);
        self.commit_index(message)
    }

    /// Commits the index as it stands.
    pub fn commit_index(&self, message: &str) -> ObjectId {
        self.git(&["commit", "-q", "-m", message]);
        self.rev_parse("HEAD")
    }

    pub fn rev_parse(&self, revision: &str) -> ObjectId {
        let hex = self.git(&["rev-parse", revision]);
        ObjectId::from_hex(String::from_utf8(hex).unwrap().trim().as_bytes()).unwrap()
    }

    pub fn repository(&self) -> Repository {
        Repository::discover(&self.root).unwrap()
    }
}

fn assert_success(output: &Output, args: &[&str]) {
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Ten distinct lines, so a one-line edit stays well above the 50% rename
/// threshold.
fn body(tag: &str) -> String {
    (0..10).map(|line| format!("{tag} line {line}\n")).collect()
}

#[cfg(unix)]
fn os(bytes: &[u8]) -> OsString {
    use std::os::unix::ffi::OsStrExt;
    std::ffi::OsStr::from_bytes(bytes).to_os_string()
}

#[cfg(not(unix))]
fn os(bytes: &[u8]) -> OsString {
    OsString::from(String::from_utf8(bytes.to_vec()).unwrap())
}

/// `diff_scope.py`'s `parse_name_status`, record for record.
fn parse_name_status(stream: &[u8]) -> ChangedPaths {
    let mut records: Vec<&[u8]> = stream.split(|byte| *byte == 0).collect();
    if records.last() == Some(&&b""[..]) {
        records.pop();
    }
    let (mut changed, mut deleted, mut renamed_from) = (Vec::new(), Vec::new(), Vec::new());
    let mut records = records.into_iter();
    while let Some(status) = records.next() {
        let mut path = records.next().expect("a status without a path");
        if matches!(status[0], b'R' | b'C') {
            if status[0] == b'R' {
                renamed_from.push(os(path));
            }
            path = records.next().expect("a rename without a destination");
        }
        changed.push(os(path));
        if status[0] == b'D' {
            deleted.push(os(path));
        }
    }
    ChangedPaths::new(changed, deleted, renamed_from)
}

/// Asserts the gix diff equals `git diff --name-status -z BASE HEAD --`.
fn assert_git_parity(fixture: &Fixture, base: ObjectId, head: ObjectId) -> ChangedPaths {
    let ours = fixture.repository().changed_paths(base, head).unwrap();
    let stream = fixture.git(&[
        "diff",
        "--name-status",
        "-z",
        &base.to_string(),
        &head.to_string(),
        "--",
    ]);
    assert_eq!(
        ours,
        parse_name_status(&stream),
        "git's stream: {:?}",
        String::from_utf8_lossy(&stream)
    );
    ours
}

fn strings(paths: &[OsString]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn renames_with_and_without_edits_match_git() {
    let fixture = Fixture::new();
    fixture.write("src/exact.rs", &body("exact"));
    fixture.write("src/edited.rs", &body("edited"));
    fixture.write("zz.txt", &body("zz"));
    let base = fixture.commit("base");
    std::fs::rename(
        fixture.root.join("src/exact.rs"),
        fixture.root.join("a-moved.rs"),
    )
    .unwrap();
    std::fs::remove_file(fixture.root.join("src/edited.rs")).unwrap();
    fixture.write(
        "src/renamed.rs",
        &body("edited").replace("line 3", "line three"),
    );
    fixture.write("zz.txt", "rewritten\n");
    let head = fixture.commit("head");

    let paths = assert_git_parity(&fixture, base, head);

    assert_eq!(
        strings(paths.changed()),
        ["a-moved.rs", "src/renamed.rs", "zz.txt"]
    );
    assert_eq!(
        strings(paths.renamed_from()),
        ["src/exact.rs", "src/edited.rs"]
    );
    assert!(paths.deleted().is_empty());
}

#[test]
fn a_copy_is_an_addition_of_its_destination() {
    let fixture = Fixture::new();
    fixture.write("original.txt", &body("copy"));
    let base = fixture.commit("base");
    fixture.write("copy.txt", &body("copy"));
    let head = fixture.commit("head");

    let paths = assert_git_parity(&fixture, base, head);

    assert_eq!(strings(paths.changed()), ["copy.txt"]);
    assert!(paths.renamed_from().is_empty() && paths.deleted().is_empty());
}

#[test]
fn a_deletion_is_changed_and_deleted() {
    let fixture = Fixture::new();
    fixture.write("gone.txt", &body("gone"));
    fixture.write("kept.txt", &body("kept"));
    let base = fixture.commit("base");
    std::fs::remove_file(fixture.root.join("gone.txt")).unwrap();
    let head = fixture.commit("head");

    let paths = assert_git_parity(&fixture, base, head);

    assert_eq!(strings(paths.changed()), ["gone.txt"]);
    assert_eq!(strings(paths.deleted()), ["gone.txt"]);
    assert!(paths.renamed_from().is_empty());
}

#[test]
fn an_empty_file_is_renamed_by_identity_as_git_renames_it() {
    let fixture = Fixture::new();
    fixture.write("pkg/__init__.py", "");
    let base = fixture.commit("base");
    std::fs::rename(fixture.root.join("pkg"), fixture.root.join("moved")).unwrap();
    let head = fixture.commit("head");

    let paths = assert_git_parity(&fixture, base, head);

    assert_eq!(strings(paths.renamed_from()), ["pkg/__init__.py"]);
    assert!(paths.deleted().is_empty());
}

#[test]
fn a_renamed_directory_reports_each_file_and_a_file_replaced_by_a_directory_is_deleted() {
    let fixture = Fixture::new();
    fixture.write("old/a.txt", &body("a"));
    fixture.write("old/b.txt", &body("b"));
    fixture.write("node", &body("node"));
    let base = fixture.commit("base");
    std::fs::rename(fixture.root.join("old"), fixture.root.join("new")).unwrap();
    std::fs::remove_file(fixture.root.join("node")).unwrap();
    fixture.write("node/inner.txt", "different\n");
    let head = fixture.commit("head");

    let paths = assert_git_parity(&fixture, base, head);

    assert_eq!(strings(paths.renamed_from()), ["old/a.txt", "old/b.txt"]);
    assert_eq!(strings(paths.deleted()), ["node"]);
}

#[cfg(unix)]
#[test]
fn a_type_change_and_a_mode_change_are_changes() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    fixture.write("target.txt", &body("target"));
    fixture.write("becomes-link", &body("link"));
    fixture.write("script.sh", "#!/bin/sh\n");
    let base = fixture.commit("base");
    std::fs::remove_file(fixture.root.join("becomes-link")).unwrap();
    std::os::unix::fs::symlink("target.txt", fixture.root.join("becomes-link")).unwrap();
    std::fs::set_permissions(
        fixture.root.join("script.sh"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let head = fixture.commit("head");

    let paths = assert_git_parity(&fixture, base, head);

    assert_eq!(strings(paths.changed()), ["becomes-link", "script.sh"]);
    assert!(paths.deleted().is_empty() && paths.renamed_from().is_empty());
}

#[test]
fn spaces_and_unicode_survive_verbatim() {
    let fixture = Fixture::new();
    fixture.write("docs/with space.md", &body("space"));
    fixture.write("docs/naïve café.md", &body("unicode"));
    let base = fixture.commit("base");
    std::fs::rename(
        fixture.root.join("docs/with space.md"),
        fixture.root.join("docs/still spaced.md"),
    )
    .unwrap();
    fixture.write("docs/naïve café.md", "changed\n");
    fixture.write("日本語/ファイル.txt", "new\n");
    let head = fixture.commit("head");

    let paths = assert_git_parity(&fixture, base, head);

    assert_eq!(
        strings(paths.changed()),
        [
            "docs/naïve café.md",
            "docs/still spaced.md",
            "日本語/ファイル.txt"
        ]
    );
}

#[cfg(unix)]
#[test]
fn newlines_and_non_utf8_bytes_in_names_survive_on_unix() {
    let fixture = Fixture::new();
    fixture.write("keep.txt", "keep\n");
    let base = fixture.commit("base");
    fixture.write("line\nbreak.txt", "newline\n");
    fixture.git(&["add", "line\nbreak.txt"]);
    // APFS refuses non-UTF-8 names, so the entry goes straight into the
    // index; Git stores and diffs the raw bytes either way.
    let blob = fixture.git(&["hash-object", "-w", "keep.txt"]);
    let blob = String::from_utf8(blob).unwrap();
    let mut cacheinfo = format!("100644,{},raw-", blob.trim()).into_bytes();
    cacheinfo.push(0xff);
    let output = fixture
        .command(&fixture.root)
        .args(["update-index", "--add", "--cacheinfo"])
        .arg(os(&cacheinfo))
        .output()
        .unwrap();
    assert_success(&output, &["update-index"]);
    let head = fixture.commit_index("head");

    let paths = assert_git_parity(&fixture, base, head);

    assert_eq!(paths.changed(), [os(b"line\nbreak.txt"), os(b"raw-\xff")]);
}

#[test]
fn a_path_windows_cannot_carry_is_a_located_error() {
    let error = utf8_path(b"bad-\xff.txt", "1111..2222").unwrap_err();

    assert_eq!(
        error,
        GitError::NonPortablePath {
            path: "bad-\u{FFFD}.txt".to_owned(),
            revision: "1111..2222".to_owned(),
        }
    );
    let message = error.to_string();
    assert!(
        message.contains("bad-\u{FFFD}.txt") && message.contains("1111..2222"),
        "{message}"
    );
    assert_eq!(
        utf8_path("ok/naïve.txt".as_bytes(), "r").unwrap(),
        "ok/naïve.txt"
    );
}

/// The arguments file is byte-identical to `diff_scope.py`'s for one diff
/// carrying every record kind.
#[test]
fn the_arguments_file_matches_diff_scope_py() {
    let python = ["python3", "python"].into_iter().find(|program| {
        Command::new(program)
            .arg("--version")
            .output()
            .is_ok_and(|out| out.status.success())
    });
    expect_level!(Level::L1, python.is_some(), "python3");
    let fixture = Fixture::new();
    fixture.write("moved.txt", &body("moved"));
    fixture.write("deleted.txt", &body("deleted"));
    fixture.write("edited.txt", "edited\n");
    let base = fixture.commit("base");
    fixture.write("dir with space/moved.txt", &body("moved"));
    std::fs::remove_file(fixture.root.join("moved.txt")).unwrap();
    std::fs::remove_file(fixture.root.join("deleted.txt")).unwrap();
    fixture.write("edited.txt", "edited again\n");
    fixture.write("ünïcode.txt", "added\n");
    let head = fixture.commit("head");
    let stream = fixture.git(&[
        "diff",
        "--name-status",
        "-z",
        &base.to_string(),
        &head.to_string(),
        "--",
    ]);
    let script = biscuit_test_harness::manifest_dir!().join("ci/diff_scope.py");
    let mut child = Command::new(python.unwrap())
        .arg(&script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&stream).unwrap();
    let expected = child.wait_with_output().unwrap();
    assert!(expected.status.success());

    let ours = fixture
        .repository()
        .changed_paths(base, head)
        .unwrap()
        .arguments_file();

    assert_eq!(
        String::from_utf8_lossy(&ours),
        String::from_utf8_lossy(&expected.stdout)
    );
    assert_eq!(ours, expected.stdout);
}

#[test]
fn an_empty_diff_still_ends_its_arguments() {
    assert_eq!(ChangedPaths::default().arguments_file(), b"--\0");
}

#[test]
fn head_identity_on_a_branch_detached_and_in_a_linked_worktree() {
    let fixture = Fixture::new();
    fixture.write("a", "a\n");
    let first = fixture.commit("first");
    fixture.write("a", "b\n");
    let second = fixture.commit("second");
    let linked = fixture.scratch("linked");
    fixture.git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "side",
        linked.to_str().unwrap(),
        &first.to_string(),
    ]);

    let on_branch = fixture.repository().head().unwrap();
    let in_linked = Repository::discover(&linked).unwrap().head().unwrap();
    fixture.git(&["checkout", "-q", "--detach"]);
    let detached = fixture.repository().head().unwrap();

    assert_eq!(
        (on_branch.commit, on_branch.branch.as_deref()),
        (second, Some("refs/heads/main"))
    );
    assert_eq!(
        (in_linked.commit, in_linked.branch.as_deref()),
        (first, Some("refs/heads/side"))
    );
    assert_eq!((detached.commit, detached.branch), (second, None));
}

#[test]
fn an_unborn_head_and_a_directory_outside_any_repository_are_errors() {
    let fixture = Fixture::new();
    let outside = tempfile::tempdir().unwrap();

    assert_eq!(
        fixture.repository().head().unwrap_err(),
        GitError::UnbornHead
    );
    assert!(matches!(
        Repository::discover(outside.path()).unwrap_err(),
        GitError::NotARepository { .. }
    ));
}

#[test]
fn peeling_reports_missing_objects_and_non_commits() {
    let fixture = Fixture::new();
    fixture.write("file", "content\n");
    let commit = fixture.commit("first");
    fixture.git(&["tag", "-a", "-m", "annotated", "v1"]);
    let tag = fixture.rev_parse("v1");
    let blob = fixture.rev_parse("HEAD:file");
    let missing = ObjectId::from_hex(b"0123456789abcdef0123456789abcdef01234567").unwrap();
    let repository = fixture.repository();

    assert_eq!(repository.peel_to_commit(tag).unwrap(), commit);
    assert_eq!(
        repository.peel_to_commit(missing).unwrap_err(),
        GitError::MissingObject { id: missing }
    );
    assert_eq!(
        repository.peel_to_commit(blob).unwrap_err(),
        GitError::NotACommit { id: blob }
    );
    assert!(repository.has_commit(commit).unwrap());
    assert!(!repository.has_commit(missing).unwrap());
    assert!(matches!(
        repository.changed_paths(missing, commit).unwrap_err(),
        GitError::MissingObject { .. }
    ));
}

#[test]
fn ancestry_follows_history_not_order() {
    let fixture = Fixture::new();
    fixture.write("f", "1\n");
    let root = fixture.commit("root");
    fixture.write("f", "2\n");
    let main = fixture.commit("main");
    fixture.git(&["checkout", "-q", "-b", "side", &root.to_string()]);
    fixture.write("g", "side\n");
    let side = fixture.commit("side");
    let repository = fixture.repository();

    assert!(repository.is_ancestor(root, main).unwrap());
    assert!(repository.is_ancestor(main, main).unwrap());
    assert!(!repository.is_ancestor(main, root).unwrap());
    assert!(!repository.is_ancestor(side, main).unwrap());
}

#[test]
fn dirtiness_counts_untracked_files_but_not_ignored_ones() {
    let fixture = Fixture::new();
    fixture.write(".gitignore", "ignored.log\n");
    fixture.commit("first");
    fixture.write("ignored.log", "noise\n");
    let ignored_only = fixture.repository().is_dirty().unwrap();
    fixture.write("untracked.txt", "new\n");

    assert!(!ignored_only);
    assert!(fixture.repository().is_dirty().unwrap());
}

/// Every transport command run, recorded before it is passed on.
struct Recording<R> {
    inner: R,
    commands: Vec<Vec<String>>,
}

impl<R> Recording<R> {
    fn new(inner: R) -> Self {
        Recording {
            inner,
            commands: Vec::new(),
        }
    }

    fn commands(&self) -> Vec<Vec<String>> {
        self.commands.clone()
    }
}

impl<R: CommandRunner> CommandRunner for Recording<R> {
    fn run(&mut self, command: &TransportCommand) -> std::io::Result<CommandOutput> {
        let mut argv = vec![command.program().to_owned()];
        argv.extend(
            command
                .args()
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned()),
        );
        self.commands.push(argv);
        self.inner.run(command)
    }
}

/// A bare remote holding `main` at the fixture's commit, and a second clone
/// that has pushed `next`, one commit the fixture has never seen.
fn remote_with_unseen_branch(fixture: &Fixture) -> (String, ObjectId, ObjectId) {
    fixture.write("f", "local\n");
    let local = fixture.commit("local");
    let remote = fixture.scratch("remote.git");
    let remote_path = remote.to_str().unwrap().to_owned();
    fixture.git_at(fixture.base(), &["init", "-q", "--bare", &remote_path]);
    fixture.git(&["push", "-q", &remote_path, "main"]);
    let other = fixture.scratch("other");
    fixture.git_at(
        fixture.base(),
        &["clone", "-q", &remote_path, other.to_str().unwrap()],
    );
    std::fs::write(other.join("f"), "unseen\n").unwrap();
    fixture.git_at(
        &other,
        &[
            "-c",
            "user.name=O",
            "-c",
            "user.email=o@example.com",
            "commit",
            "-q",
            "-am",
            "unseen",
        ],
    );
    fixture.git_at(&other, &["push", "-q", "origin", "HEAD:refs/heads/next"]);
    let unseen = ObjectId::from_hex(
        String::from_utf8(fixture.git_at(&other, &["rev-parse", "HEAD"]))
            .unwrap()
            .trim()
            .as_bytes(),
    )
    .unwrap();
    (remote_path, local, unseen)
}

#[test]
fn remote_tips_are_read_from_the_remote_fetched_once_and_cached() {
    let fixture = Fixture::new();
    let (remote, local, unseen) = remote_with_unseen_branch(&fixture);
    let repository = fixture.repository();
    let mut runner = Recording::new(DirectRunner);

    let (current, advertised, again, absent) = {
        let mut tips = crate::ci_push::git::Remote::new(
            &repository,
            &mut runner,
            remote.clone(),
            Some(&remote),
        );
        (
            tips.branch_tip("main"),
            tips.branch_tip("next"),
            tips.branch_tip("next"),
            tips.branch_tip("absent"),
        )
    };

    assert_eq!(current.unwrap(), local);
    assert_eq!(advertised.unwrap(), unseen);
    assert_eq!(again.unwrap(), unseen);
    assert!(repository.has_commit(unseen).unwrap());
    assert_eq!(
        absent.unwrap_err().to_string(),
        format!(
            "branch absent does not exist on {remote} ({remote}), or the remote could not be read"
        )
    );
    assert_eq!(
        runner.commands(),
        [
            vec![
                "git",
                "ls-remote",
                "--exit-code",
                &remote,
                "refs/heads/main"
            ],
            vec![
                "git",
                "ls-remote",
                "--exit-code",
                &remote,
                "refs/heads/next"
            ],
            vec!["git", "fetch", "--no-tags", &remote, "refs/heads/next"],
            vec![
                "git",
                "ls-remote",
                "--exit-code",
                &remote,
                "refs/heads/absent"
            ],
        ]
    );
}

/// A remote that advertises a commit no fetch delivers.
struct Advertising {
    tip: ObjectId,
}

impl CommandRunner for Advertising {
    fn run(&mut self, command: &TransportCommand) -> std::io::Result<CommandOutput> {
        let stdout = if command.args()[0] == "ls-remote" {
            format!("{}\trefs/heads/main\n", self.tip).into_bytes()
        } else {
            Vec::new()
        };
        Ok(CommandOutput {
            code: Some(0),
            stdout,
            stderr: Vec::new(),
        })
    }
}

#[test]
fn an_advertised_tip_still_missing_after_one_fetch_is_unfetchable() {
    let fixture = Fixture::new();
    fixture.write("f", "x\n");
    fixture.commit("first");
    let repository = fixture.repository();
    let tip = ObjectId::from_hex(b"0123456789abcdef0123456789abcdef01234567").unwrap();
    let mut runner = Recording::new(Advertising { tip });

    let (first, ensured) = {
        let mut remote = crate::ci_push::git::Remote::new(&repository, &mut runner, "origin", None);
        (remote.branch_tip("main"), remote.ensure_commit(tip, "main"))
    };

    assert_eq!(
        first.unwrap_err(),
        RemoteTipError::Unfetchable {
            branch: "main".to_owned(),
            remote: "origin".to_owned(),
            tip,
        }
    );
    assert!(!ensured.unwrap());
    assert_eq!(
        runner.commands(),
        [
            vec![
                "git",
                "ls-remote",
                "--exit-code",
                "origin",
                "refs/heads/main"
            ],
            vec!["git", "fetch", "--no-tags", "origin", "refs/heads/main"],
        ]
    );
    assert_eq!(TransportCommand::SET_ENV, [("GIT_TERMINAL_PROMPT", "0")]);
}

#[test]
fn a_scope_note_matches_git_notes_add_and_is_pushed() {
    let fixture = Fixture::new();
    fixture.write("f", "x\n");
    let head = fixture.commit("first");
    let remote = fixture.scratch("remote.git");
    let remote_path = remote.to_str().unwrap().to_owned();
    fixture.git_at(fixture.base(), &["init", "-q", "--bare", &remote_path]);
    let document = b"\n\n{\"kind\": \"scope\",   \n\n\n  \"base\": \"x\"}\t\n\n";
    let file = fixture.scratch("document.json");
    std::fs::write(&file, document).unwrap();
    fixture.git(&[
        "notes",
        "--ref",
        "refs/notes/oracle",
        "add",
        "-F",
        file.to_str().unwrap(),
        &head.to_string(),
    ]);
    let repository = fixture.repository();
    let mut runner = Recording::new(DirectRunner);

    let pushed = {
        let mut remote =
            crate::ci_push::git::Remote::new(&repository, &mut runner, remote_path.clone(), None);
        attach_note(
            &mut remote,
            "refs/notes/ci-local/scope",
            b"replaced\n",
            head,
        )
        .unwrap();
        attach_note(&mut remote, "refs/notes/ci-local/scope", document, head).unwrap();
        remote.push_ref("refs/notes/ci-local/scope")
    };

    let ours = fixture.git(&[
        "notes",
        "--ref",
        "refs/notes/ci-local/scope",
        "show",
        &head.to_string(),
    ]);
    let oracle = fixture.git(&[
        "notes",
        "--ref",
        "refs/notes/oracle",
        "show",
        &head.to_string(),
    ]);
    assert_eq!(
        String::from_utf8_lossy(&ours),
        String::from_utf8_lossy(&oracle)
    );
    assert_eq!(
        repository
            .read_note("refs/notes/ci-local/scope", head)
            .unwrap(),
        Some(oracle.clone())
    );
    assert_eq!(
        repository.read_note("refs/notes/absent", head).unwrap(),
        None
    );
    pushed.unwrap();
    let advertised = fixture.git_at(&remote, &["show-ref", "refs/notes/ci-local/scope"]);
    assert!(!advertised.is_empty());
    let commands = runner.commands();
    assert_eq!(
        commands[0],
        [
            "git",
            "fetch",
            "--no-tags",
            &remote_path,
            "+refs/notes/ci-local/scope:refs/notes/ci-local/scope"
        ]
    );
    assert_eq!(
        commands.last().unwrap(),
        &[
            "git",
            "push",
            "--no-verify",
            &remote_path,
            "refs/notes/ci-local/scope:refs/notes/ci-local/scope"
        ]
    );
}

#[rstest]
#[case::scp("git@github.com:owner/repo.git", GithubRemote::Repository("owner/repo".into()))]
#[case::ssh_user("ssh://git@github.com/owner/repo", GithubRemote::Repository("owner/repo".into()))]
#[case::ssh("ssh://github.com/owner/repo.git", GithubRemote::Repository("owner/repo".into()))]
#[case::https("https://github.com/owner/repo", GithubRemote::Repository("owner/repo".into()))]
#[case::https_credentials("https://user:token@github.com/owner/repo.git", GithubRemote::Repository("owner/repo".into()))]
#[case::http("http://github.com/owner/repo", GithubRemote::Repository("owner/repo".into()))]
#[case::trailing_slash("https://github.com/owner/repo/", GithubRemote::Repository("owner/repo".into()))]
#[case::git_then_slash("https://github.com/owner/repo.git/", GithubRemote::Repository("owner/repo".into()))]
#[case::too_many_segments("https://github.com/owner/repo/extra", GithubRemote::Unrecognized)]
#[case::owner_only("https://github.com/owner", GithubRemote::Unrecognized)]
#[case::empty_owner("https://github.com//repo", GithubRemote::Unrecognized)]
#[case::bare_host("https://github.com/", GithubRemote::Unrecognized)]
#[case::unread_spelling("git://github.com/owner/repo", GithubRemote::Unrecognized)]
#[case::www("https://www.github.com/owner/repo", GithubRemote::Unrecognized)]
#[case::other_host("git@gitlab.com:owner/repo.git", GithubRemote::NotGithub)]
#[case::filesystem("/srv/git/repo.git", GithubRemote::NotGithub)]
#[case::empty("", GithubRemote::NotGithub)]
fn github_remotes_are_recognized_exactly_as_the_shell_did(
    #[case] url: &str,
    #[case] expected: GithubRemote,
) {
    assert_eq!(github_repo_of(url), expected);
}
