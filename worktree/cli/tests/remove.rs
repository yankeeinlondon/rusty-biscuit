//! `wt remove` through the real binary: the flag surface, the spec's Examples
//! table, exit codes, and the move-first handoff.
//!
//! Every run has stdin redirected and `CI`/`WT_SHELL_WRAPPER` removed, so it
//! is non-interactive and wrapper-less unless a test says otherwise, and sets
//! `NO_COLOR`. Remotes
//! are local bare repositories: the PR lookup answers "unsupported host" at
//! once and the live checks run `git ls-remote` against the bare repository.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use predicates::prelude::*;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = worktree::git::command()
        .current_dir(dir)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("git should be installed");
    assert!(
        out.status.success(),
        "git {args:?} in {dir:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Appends `text` to the repository's own config file: one write, where a
/// `git config` per key costs a process each (20–50 ms on Windows).
fn append_config(dir: &Path, text: &str) {
    append_to_config_file(&dir.join(".git").join("config"), text);
}

/// [`append_config`] for a config file by path, such as a bare
/// repository's `config`.
fn append_to_config_file(config: &Path, text: &str) {
    use std::io::Write as _;
    let mut file = fs::OpenOptions::new().append(true).open(config).unwrap_or_else(|e| panic!("{config:?}: {e}"));
    file.write_all(text.as_bytes()).unwrap();
}

/// `maintenance.auto = false`: every commit and push otherwise starts a
/// detached `git maintenance run --auto`, one more process per step.
fn configure(dir: &Path) {
    append_config(
        dir,
        "[user]\n\temail = test@example.com\n\tname = Test User\n[commit]\n\tgpgsign = false\n[gc]\n\tauto = 0\n[maintenance]\n\tauto = false\n",
    );
}

/// `git remote add origin <url>`, written to the config file.
fn add_origin(dir: &Path, url: &str) {
    // A Windows path's backslashes are escapes in a config file.
    let url = url.replace('\\', "\\\\").replace('"', "\\\"");
    append_config(dir, &format!("[remote \"origin\"]\n\turl = \"{url}\"\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n"));
}

/// `git rev-parse HEAD` in `dir` right after a commit there, read from the
/// loose ref the commit wrote rather than by another process. Anything else
/// (a subdirectory, a packed ref, a detached HEAD) asks Git.
fn head_after_commit(dir: &Path) -> String {
    let read = || -> Option<String> {
        let dot_git = dir.join(".git");
        let git_dir = match fs::read_to_string(&dot_git) {
            Ok(link) => dir.join(link.trim().strip_prefix("gitdir: ")?),
            Err(_) if dot_git.is_dir() => dot_git,
            Err(_) => return None,
        };
        let common = match fs::read_to_string(git_dir.join("commondir")) {
            Ok(relative) => git_dir.join(relative.trim()),
            Err(_) => git_dir.clone(),
        };
        let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
        let sha = fs::read_to_string(common.join(head.trim().strip_prefix("ref: ")?)).ok()?;
        Some(sha.trim().to_string()).filter(|sha| sha.len() >= 40)
    };
    read().unwrap_or_else(|| git(dir, &["rev-parse", "HEAD"]))
}

fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Copies `from` into `to`, leaving out `git init`'s `*.sample` hooks.
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let (source, name) = (entry.path(), entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&source, &to.join(&name));
        } else if !name.to_string_lossy().ends_with(".sample") {
            fs::copy(&source, to.join(&name)).unwrap();
        }
    }
}

/// The start every [`Fixture`] copies: `repo` on `main` with one commit
/// and a bare `origin.git` cloned from it, built once per build of this test
/// binary and shared by its test processes (nextest runs each test in a
/// process of its own). It holds no absolute path: the clone's
/// `remote.origin.url` is cut, and a fixture names its own `origin`.
///
/// It lives beside the binary, so `cargo clean` removes it, and is keyed by
/// the binary's path, size, and modification time. A process that finds none
/// builds one, renames it into place (a racing process's copy is discarded),
/// and removes earlier builds' templates; a finished template is never
/// written again.
fn template() -> PathBuf {
    use std::hash::{Hash, Hasher};
    let exe = std::env::current_exe().unwrap();
    let metadata = fs::metadata(&exe).unwrap();
    let mut key = std::collections::hash_map::DefaultHasher::new();
    (&exe, metadata.len(), metadata.modified().unwrap()).hash(&mut key);
    let parent = exe.parent().unwrap().join("wt-remove-fixture-templates");
    let template = parent.join(format!("{:016x}", key.finish()));
    if template.is_dir() {
        return template;
    }
    fs::create_dir_all(&parent).unwrap();
    let building = tempfile::tempdir_in(&parent).unwrap();
    let repo = building.path().join("repo");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    configure(&repo);
    fs::write(repo.join("README.md"), "README.md\n").unwrap();
    git(&repo, &["add", "README.md"]);
    git(&repo, &["commit", "-q", "-m", "add README.md"]);
    // A local bare clone hardlinks the objects in one process; `init
    // --bare` and a push start several (the push's receive-pack and
    // pack-objects among them), 20–50 ms each on Windows.
    git(building.path(), &["clone", "-q", "--bare", "repo", "origin.git"]);
    let config = building.path().join("origin.git").join("config");
    let text = fs::read_to_string(&config).unwrap();
    let kept = text.split_inclusive('\n').scan(true, |keep, line| {
        if line.starts_with('[') {
            *keep = !line.starts_with("[remote ");
        }
        Some((*keep).then_some(line))
    });
    fs::write(&config, kept.flatten().collect::<String>()).unwrap();
    append_to_config_file(&config, "[maintenance]\n\tauto = false\n");
    let built = building.keep();
    if fs::rename(&built, &template).is_err() {
        assert!(template.is_dir(), "the template could not be put in place at {template:?}");
        let _ = fs::remove_dir_all(&built);
    }
    // Another key belongs to an earlier build of this binary, which no
    // longer exists at this path; an unfinished build is a racing process's.
    for entry in fs::read_dir(&parent).unwrap().flatten() {
        let name = entry.file_name();
        if name.len() == 16 && name != template.file_name().unwrap() {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
    template
}

/// What `git update-ref <name> <commit>` writes in the repository at
/// `git_dir` when no reflog is kept: one loose ref, which overrides a packed
/// one.
fn write_ref(git_dir: &Path, name: &str, commit: &str) {
    let reference = git_dir.join(name);
    fs::create_dir_all(reference.parent().unwrap()).unwrap();
    fs::write(reference, format!("{commit}\n")).unwrap();
}

struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    /// `root/repo` on `main` with one commit; worktrees go in `root/wts`.
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        copy_tree(&template().join("repo"), &root.path().join("repo"));
        Self { root }
    }

    /// [`Fixture::new`] plus a bare `origin` holding `main`, which `main`
    /// tracks, as `git push -u origin main` leaves them.
    fn with_origin() -> Self {
        let this = Self::new();
        let (repo, origin) = (this.repo(), this.origin());
        copy_tree(&template().join("origin.git"), &origin);
        add_origin(&repo, origin.to_str().unwrap());
        this.track("main", "main");
        this.set_tracking_ref("main", &this.tip("main"));
        this
    }

    fn repo(&self) -> PathBuf {
        self.root.path().join("repo")
    }

    fn origin(&self) -> PathBuf {
        self.root.path().join("origin.git")
    }

    fn commit(&self, dir: &Path, file: &str) -> String {
        fs::write(dir.join(file), format!("{file}\n")).unwrap();
        git(dir, &["add", file]);
        git(dir, &["commit", "-q", "-m", &format!("add {file}")]);
        head_after_commit(dir)
    }

    /// A new branch `branch` at `main`, checked out in `root/wts/<dir_name>`.
    /// The branch is written first, which spares `worktree add -b` its
    /// `git branch` child.
    fn add_worktree(&self, branch: &str, dir_name: &str) -> PathBuf {
        let path = self.root.path().join("wts").join(dir_name);
        write_ref(&self.repo().join(".git"), &format!("refs/heads/{branch}"), &self.tip("main"));
        git(&self.repo(), &["worktree", "add", "-q", path.to_str().unwrap(), branch]);
        path
    }

    /// The commit `refs/heads/<branch>` names, read from its loose ref when
    /// there is one.
    fn tip(&self, branch: &str) -> String {
        let loose = self.repo().join(".git").join("refs").join("heads").join(branch);
        match fs::read_to_string(loose) {
            Ok(sha) => sha.trim().to_string(),
            Err(_) => git(&self.repo(), &["rev-parse", &format!("refs/heads/{branch}")]),
        }
    }

    fn set_tracking_ref(&self, branch: &str, commit: &str) {
        write_ref(&self.repo().join(".git"), &format!("refs/remotes/origin/{branch}"), commit);
    }

    /// What `git branch --set-upstream-to=origin/<upstream> <branch>` writes.
    fn track(&self, branch: &str, upstream: &str) {
        append_config(&self.repo(), &format!("[branch \"{branch}\"]\n\tremote = origin\n\tmerge = refs/heads/{upstream}\n"));
    }

    /// What `git push <bare> <commit>:refs/heads/<branch>` leaves in the
    /// bare repository `bare`: the branch at `commit`, with the objects it
    /// needs. It starts no process, where a push starts seven (receive-pack,
    /// pack-objects, unpack-objects, rev-list, and for-each-ref among them).
    /// `bare` receives every loose object it lacks, a superset of the
    /// commit's; `wt` reads only a remote's refs. A repository with a pack
    /// falls back to `git push`.
    fn publish(&self, bare: &Path, commit: &str, branch: &str) {
        let objects = self.repo().join(".git").join("objects");
        let packed = fs::read_dir(objects.join("pack")).is_ok_and(|mut entries| entries.next().is_some());
        if packed {
            git(&self.repo(), &["push", "-q", "--force", bare.to_str().unwrap(), &format!("{commit}:refs/heads/{branch}")]);
            return;
        }
        for directory in fs::read_dir(&objects).unwrap().map(|entry| entry.unwrap()) {
            let name = directory.file_name();
            if name.len() != 2 || !directory.file_type().unwrap().is_dir() {
                continue;
            }
            for object in fs::read_dir(directory.path()).unwrap().map(|entry| entry.unwrap()) {
                let target = bare.join("objects").join(&name).join(object.file_name());
                if !target.exists() {
                    fs::create_dir_all(target.parent().unwrap()).unwrap();
                    fs::copy(object.path(), target).unwrap();
                }
            }
        }
        write_ref(bare, &format!("refs/heads/{branch}"), commit);
    }

    /// What `git push origin <commit>:refs/heads/<branch>` leaves behind:
    /// [`Fixture::publish`] to origin, and the tracking ref `origin/<branch>`
    /// moved, as origin's fetch refspec makes a push do.
    fn push(&self, commit: &str, branch: &str) {
        self.publish(&self.origin(), commit, branch);
        self.set_tracking_ref(branch, commit);
    }

    /// `git push -u origin <branch>`: [`Fixture::push`] of the branch's tip,
    /// tracked.
    fn push_upstream(&self, branch: &str) {
        self.push(&self.tip(branch), branch);
        self.track(branch, branch);
    }

    /// Someone else's push to origin: origin's `refs/heads/<branch>` moved
    /// to `commit` (an object origin has), or deleted, while this
    /// repository's tracking ref stays where it was.
    fn others_push(&self, branch: &str, commit: Option<&str>) {
        let reference = format!("refs/heads/{branch}");
        match commit {
            Some(commit) => git(&self.origin(), &["update-ref", &reference, commit]),
            None => git(&self.origin(), &["update-ref", "-d", &reference]),
        };
    }

    fn wt(&self, cwd: &Path) -> assert_cmd::Command {
        let mut cmd = assert_cmd::Command::cargo_bin("wt").unwrap();
        cmd.current_dir(cwd)
            .env_remove("WT_SHELL_WRAPPER")
            .env_remove("CI")
            .env_remove("COMPLETE")
            .env("GIT_TERMINAL_PROMPT", "0")
            // Plain text, so needles never straddle an SGR sequence.
            .env("NO_COLOR", "1")
            .write_stdin("");
        cmd
    }

    fn branch_exists(&self, branch: &str) -> bool {
        worktree::git::command()
            .current_dir(self.repo())
            .args(["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")])
            .status()
            .unwrap()
            .success()
    }

    fn origin_has(&self, branch: &str) -> bool {
        !git(&self.repo(), &["ls-remote", "origin", &format!("refs/heads/{branch}")]).is_empty()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Ok(path) = worktree::fork_origin::fork_origin_path(&self.repo()) {
            let _ = fs::remove_file(path);
        }
    }
}

#[test]
fn unchanged_included_copy_and_other_ignored_files_remove_without_force() {
    let fixture = Fixture::new();
    fs::write(fixture.repo().join(".gitignore"), b".env\ntarget/\n").unwrap();
    fs::write(fixture.repo().join(".worktreeinclude"), b".env\n").unwrap();
    git(&fixture.repo(), &["add", ".gitignore", ".worktreeinclude"]);
    git(&fixture.repo(), &["commit", "-q", "-m", "include rules"]);
    fs::write(fixture.repo().join(".env"), b"SECRET=old\n").unwrap();
    fs::create_dir_all(fixture.root.path().join("wts")).unwrap();
    let created = fixture.wt(&fixture.repo()).args(["create", "included"])
        .env("WT", fixture.root.path().join("wts")).output().unwrap();
    assert!(created.status.success(), "{}", String::from_utf8_lossy(&created.stderr));
    let target = fixture.root.path().join("wts/repo/included");
    let record = worktree::copy_record::record_path(&fixture.repo(), &target).unwrap();
    assert!(record.exists());
    fs::create_dir_all(target.join("target")).unwrap();
    fs::write(target.join("target/build"), b"generated").unwrap();
    fs::write(fixture.repo().join(".env"), b"SECRET=new\n").unwrap();
    let removed = fixture.wt(&fixture.repo()).args(["remove", "included"])
        .output().unwrap();
    assert!(removed.status.success(), "{}", String::from_utf8_lossy(&removed.stderr));
    assert!(!target.exists());
    assert!(!record.exists());
}

fn setup_included(fixture: &Fixture) -> PathBuf {
    fs::write(fixture.repo().join(".gitignore"), b".env\n").unwrap();
    fs::write(fixture.repo().join(".worktreeinclude"), b".env\n").unwrap();
    git(&fixture.repo(), &["add", ".gitignore", ".worktreeinclude"]);
    git(&fixture.repo(), &["commit", "-q", "-m", "include rules"]);
    fs::write(fixture.repo().join(".env"), b"SECRET=old\n").unwrap();
    fs::create_dir_all(fixture.root.path().join("wts")).unwrap();
    let output = fixture.wt(&fixture.repo()).args(["create", "included"])
        .env("WT", fixture.root.path().join("wts")).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    fixture.root.path().join("wts/repo/included")
}

#[test]
fn changed_included_copy_requires_force_and_preserves_state_on_refusal() {
    let fixture = Fixture::new();
    let target = setup_included(&fixture);
    fs::write(target.join(".env"), b"SECRET=new\n").unwrap();
    let output = fixture.wt(&fixture.repo()).args(["remove", "included"]).output().unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report = flowed(&stderr_of(&output));
    assert!(
        report.contains("1 included file copied in by .worktreeinclude will be lost: .env (changed since it was copied)"),
        "{report}"
    );
    assert!(report.contains("Add --force to delete it anyway."), "{report}");
    assert!(target.join(".env").exists());
    assert!(fixture.branch_exists("included"));
    fixture.wt(&fixture.repo()).args(["remove", "included", "--force"])
        .assert().code(0);
    assert!(!target.exists());
}

mod same_size_included_edit_with_restored_mtime_requires_consent_at_both_sizes {
    use super::*;

    const ROWS: [usize; 2] = [11, 2_000_000];

    fn check(size: usize) {
        let fixture = Fixture::new();
        fs::write(fixture.repo().join(".gitignore"), b".env\n").unwrap();
        fs::write(fixture.repo().join(".worktreeinclude"), b".env\n").unwrap();
        git(&fixture.repo(), &["add", ".gitignore", ".worktreeinclude"]);
        git(&fixture.repo(), &["commit", "-q", "-m", "include rules"]);
        fs::write(fixture.repo().join(".env"), vec![b'a'; size]).unwrap();
        fs::create_dir_all(fixture.root.path().join("wts")).unwrap();
        let created = fixture.wt(&fixture.repo()).args(["create", "included"])
            .env("WT", fixture.root.path().join("wts")).output().unwrap();
        assert!(created.status.success(), "{}", String::from_utf8_lossy(&created.stderr));
        let target = fixture.root.path().join("wts/repo/included");
        let path = target.join(".env");
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        fs::write(&path, vec![b'b'; size]).unwrap();
        fs::File::options().write(true).open(&path).unwrap()
            .set_times(fs::FileTimes::new().set_modified(modified)).unwrap();
        let output = fixture.wt(&fixture.repo()).args(["remove", "included"]).output().unwrap();
        assert_eq!(output.status.code(), Some(3), "size={size}: {}", String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8_lossy(&output.stderr).contains("changed"));
        assert_eq!(fs::read(path).unwrap(), vec![b'b'; size]);
    }

    #[test]
    fn small() {
        check(ROWS[0]);
    }

    #[test]
    fn large() {
        check(ROWS[1]);
    }

    /// One test per row of `ROWS` (test-speed.md #2), so a row added
    /// there fails here until it has its own test above.
    #[test]
    fn every_row_has_a_test() {
        assert_eq!(ROWS.len(), 2);
    }
}

#[test]
fn no_record_uses_distinct_source_and_new_file_needs_consent() {
    let fixture = Fixture::new();
    fs::write(fixture.repo().join(".gitignore"), b".env\n").unwrap();
    fs::write(fixture.repo().join(".worktreeinclude"), b".env\n").unwrap();
    git(&fixture.repo(), &["add", ".gitignore", ".worktreeinclude"]);
    git(&fixture.repo(), &["commit", "-q", "-m", "include rules"]);
    fs::write(fixture.repo().join(".env"), b"SECRET=old\n").unwrap();
    let target = fixture.add_worktree("feat/x", "feat-x");
    fs::write(target.join(".env"), b"SECRET=old\n").unwrap();
    fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).assert().code(0);
    assert!(!target.exists());

    let target = fixture.add_worktree("feat/y", "feat-y");
    fs::write(target.join(".env"), b"SECRET=new\n").unwrap();
    fs::remove_file(fixture.repo().join(".env")).unwrap();
    fixture.wt(&fixture.repo()).args(["remove", "feat-y"]).assert().code(3)
        .stderr(predicate::str::contains(".env"))
        .stderr(predicate::str::contains("new"));
    assert!(target.join(".env").exists());
}

/// The handoff's second run compares nothing about the checkout's content
/// (R18): a source that changed between the runs no longer matters.
#[test]
fn no_record_source_change_between_the_runs_is_deleted_by_the_second_run() {
    let fixture = Fixture::new();
    fs::write(fixture.repo().join(".gitignore"), b".env\n").unwrap();
    fs::write(fixture.repo().join(".worktreeinclude"), b".env\n").unwrap();
    git(&fixture.repo(), &["add", ".gitignore", ".worktreeinclude"]);
    git(&fixture.repo(), &["commit", "-q", "-m", "include rules"]);
    fs::write(fixture.repo().join(".env"), b"SECRET=old\n").unwrap();
    let target = fixture.add_worktree("feat/x", "feat-x");
    fs::write(target.join(".env"), b"SECRET=old\n").unwrap();
    let (landing, token) = first_run(&fixture, &target, &[]);
    fs::write(fixture.repo().join(".env"), b"SECRET=new\n").unwrap();
    fixture.wt(&landing).args(["remove", "--handoff", &token]).assert().code(0);
    assert_deleted(&fixture, &target, "feat/x");
    assert_eq!(fs::read(fixture.repo().join(".env")).unwrap(), b"SECRET=new\n", "the source is untouched");
}

#[test]
fn no_record_uses_the_checked_out_fork_parent_as_source() {
    let fixture = Fixture::new();
    fs::write(fixture.repo().join(".gitignore"), b".env\n").unwrap();
    fs::write(fixture.repo().join(".worktreeinclude"), b".env\n").unwrap();
    git(&fixture.repo(), &["add", ".gitignore", ".worktreeinclude"]);
    git(&fixture.repo(), &["commit", "-q", "-m", "include rules"]);
    fs::write(fixture.repo().join(".env"), b"BASE=1\n").unwrap();
    let parent = fixture.add_worktree("feat/parent", "parent");
    fs::write(parent.join(".env"), b"PARENT=1\n").unwrap();
    let target = fixture.add_worktree("feat/child", "child");
    fs::write(target.join(".env"), b"PARENT=1\n").unwrap();
    let path = worktree::fork_origin::fork_origin_path(&fixture.repo()).unwrap();
    worktree::fork_origin::record(&path, "feat/child", worktree::fork_origin::ForkOrigin {
        base_branch: "feat/parent".into(), base_sha: git(&fixture.repo(), &["rev-parse", "HEAD"]), created_at: 0,
    }).unwrap();
    fixture.wt(&fixture.repo()).args(["remove", "child"]).assert().code(0);
    assert!(!target.exists());
    assert!(parent.join(".env").exists());
}

#[test]
fn changed_include_rules_between_the_runs_are_deleted_by_the_second_run() {
    let fixture = Fixture::new();
    let target = setup_included(&fixture);
    let first = fixture.wt(&target).env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "included"]).assert().code(0).get_output().stdout.clone();
    let (landing, token) = protocol(&first);
    fs::write(target.join(".worktreeinclude"), b"other.env\n").unwrap();
    fixture.wt(&landing).args(["remove", "--handoff", &token]).assert().code(0);
    assert_deleted(&fixture, &target, "included");
}

#[test]
fn a_changed_copy_record_between_the_runs_is_deleted_with_the_checkout() {
    let fixture = Fixture::new();
    let target = setup_included(&fixture);
    let first = fixture.wt(&target).env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "included"]).assert().code(0).get_output().stdout.clone();
    let (landing, token) = protocol(&first);
    let path = worktree::copy_record::record_path(&fixture.repo(), &target).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["source_label"] = serde_json::Value::String("changed source label".into());
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    fixture.wt(&landing).args(["remove", "--handoff", &token]).assert().code(0);
    assert_deleted(&fixture, &target, "included");
    assert!(!path.exists(), "the copy record goes with the checkout");
}

#[test]
fn changed_included_contents_between_the_runs_are_deleted_by_the_second_run() {
    let fixture = Fixture::new();
    let target = setup_included(&fixture);
    fs::write(target.join(".env"), b"SECRET=one\n").unwrap();
    let first = fixture.wt(&target).env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "included", "--force"])
        .assert().code(0).get_output().stdout.clone();
    let (landing, token) = protocol(&first);
    fs::write(target.join(".env"), b"SECRET=two\n").unwrap();
    fixture.wt(&landing).args(["remove", "--handoff", &token]).assert().code(0);
    assert_deleted(&fixture, &target, "included");
}

#[test]
fn corrupt_copy_record_falls_back_to_source_without_consent() {
    let fixture = Fixture::new();
    let target = setup_included(&fixture);
    let path = worktree::copy_record::record_path(&fixture.repo(), &target).unwrap();
    fs::write(&path, b"{not json").unwrap();
    let output = fixture.wt(&fixture.repo()).args(["remove", "included"]).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stderr).contains("copy record is untrusted"));
    assert!(!target.exists());
    assert!(!path.exists());
}

#[test]
fn unavailable_distinct_source_marks_included_file_unknown() {
    let fixture = Fixture::new();
    fs::write(fixture.repo().join(".gitignore"), b".env\n").unwrap();
    fs::write(fixture.repo().join(".worktreeinclude"), b".env\n").unwrap();
    git(&fixture.repo(), &["add", ".gitignore", ".worktreeinclude"]);
    git(&fixture.repo(), &["commit", "-q", "-m", "include rules"]);
    let target = fixture.add_worktree("feat/x", "feat-x");
    fs::write(target.join(".env"), b"SECRET=old\n").unwrap();
    let path = worktree::fork_origin::fork_origin_path(&fixture.repo()).unwrap();
    worktree::fork_origin::record(&path, "feat/x", worktree::fork_origin::ForkOrigin {
        base_branch: "feat/x".into(), base_sha: git(&fixture.repo(), &["rev-parse", "HEAD"]), created_at: 0,
    }).unwrap();
    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("will be lost: .env (could not be compared: "), "{stderr}");
    assert!(target.join(".env").exists());
    fixture.wt(&fixture.repo()).args(["remove", "feat-x", "--force"])
        .assert().code(0);
    assert!(!target.exists());
}

/// Include rules that cannot be read are a warning, not a failure before
/// removal: the included files could not be checked, and `--force` deletes
/// them with the checkout.
#[test]
fn invalid_include_rules_are_a_warning_and_force_deletes_the_checkout() {
    let fixture = Fixture::new();
    let target = fixture.add_worktree("feat/x", "feat-x");
    fs::create_dir(target.join(".worktreeinclude")).unwrap();
    fs::write(target.join(".worktreeinclude/entry"), b"secret").unwrap();

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("Files copied in by .worktreeinclude could not be checked ("), "{stderr}");
    assert!(stderr.contains("they will be deleted."), "{stderr}");
    assert!(stderr.contains("Nothing was removed."), "{stderr}");
    assert!(target.join(".worktreeinclude/entry").exists());
    assert_nothing_removed(&fixture, &target);

    fixture.wt(&fixture.repo()).args(["remove", "feat-x", "--force"]).assert().code(0);
    assert_deleted(&fixture, &target, "feat/x");
}

// --- Flag surface -----------------------------------------------------------

#[test]
fn retired_flags_are_clap_errors() {
    let fixture = Fixture::new();
    fixture.add_worktree("feat/x", "feat-x");
    for args in [
        &["remove", "feat/x", "-b"][..],
        &["remove", "feat/x", "--branch"],
        &["remove", "feat/x", "-f"],
        &["remove", "feat/x", "-ff"],
        // R28: `--force` replaced it, with no alias.
        &["remove", "feat/x", "--force-worktree"],
        &["remove", "feat/x", "--remove-branch"],
        &["remove", "feat/x", "--remove-remote"],
    ] {
        fixture
            .wt(&fixture.repo())
            .args(args)
            .assert()
            .code(2)
            .stderr(predicate::str::contains("unexpected argument"));
    }
    assert!(fixture.root.path().join("wts/feat-x").exists());
}

#[test]
fn help_lists_the_three_force_flags_and_hides_handoff() {
    assert_cmd::Command::cargo_bin("wt")
        .unwrap()
        .args(["remove", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--force "))
        .stdout(predicate::str::contains("--force-worktree").not())
        .stdout(predicate::str::contains("--force-branch"))
        .stdout(predicate::str::contains("--force-remote"))
        .stdout(predicate::str::contains("--handoff").not())
        .stdout(predicate::str::contains("-f,").not());
}

#[test]
fn handoff_conflicts_with_a_name_and_every_force_flag() {
    let token = "a".repeat(32);
    for extra in [
        &["feat/x"][..],
        &["--force"],
        &["--force-branch"],
        &["--force-remote"],
    ] {
        assert_cmd::Command::cargo_bin("wt")
            .unwrap()
            .args(["remove", "--handoff", &token])
            .args(extra)
            .assert()
            .code(2);
    }
    assert_cmd::Command::cargo_bin("wt")
        .unwrap()
        .args(["remove"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("required"));
}

#[test]
fn an_unknown_name_fails_and_the_base_checkout_is_refused() {
    let fixture = Fixture::new();
    fixture
        .wt(&fixture.repo())
        .args(["remove", "no-such-worktree"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("not found"));
    fixture
        .wt(&fixture.repo())
        .args(["remove", "base", "--force", "--force-branch"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("main checkout"));
}

/// `sleep 60` started in `dir`, killed and reaped when dropped, so a failed
/// assertion never leaks it.
#[cfg(unix)]
struct Holder(std::process::Child);

#[cfg(unix)]
impl Holder {
    fn standing_in(dir: &Path) -> Self {
        Self(Command::new("sleep").arg("60").current_dir(dir).spawn().unwrap())
    }

    fn pid(&self) -> u32 {
        self.0.id()
    }
}

#[cfg(unix)]
impl Drop for Holder {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// R1: a program using the worktree is a warning naming it and its pid, not
/// a stop. Nobody can be asked here, so without `--force` nothing is
/// removed (exit 3); `--force` deletes the worktree with the program still
/// standing in it.
#[cfg(unix)]
#[test]
fn a_process_standing_in_the_worktree_is_a_warning_and_force_deletes_it() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/busy", "busy");
    let holder = Holder::standing_in(&wt);

    let output = fixture.wt(&fixture.repo()).args(["remove", "busy"]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("⚠ 1 program is using files in this worktree:"), "{stderr}");
    assert!(stderr.contains(&format!("sleep (pid {}", holder.pid())), "{stderr}");
    assert!(stderr.contains("Deleting now may make them fail."), "{stderr}");
    assert!(stderr.contains("There is no terminal to ask whether to delete worktree busy"), "{stderr}");
    assert!(stderr.contains("Add --force to delete it anyway."), "{stderr}");
    assert!(output.stdout.is_empty(), "no protocol lines");
    assert!(wt.join("README.md").exists());
    assert!(fixture.branch_exists("feat/busy"));
    assert!(listed_block(&fixture, &wt).is_some(), "still registered");
    assert_eq!(pending_removals(&fixture), 0, "a refusal saves no approval");

    let output = fixture.wt(&fixture.repo()).args(["remove", "busy", "--force"]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Removed busy ("), "{stderr}");
    drop(holder);
    assert_deleted(&fixture, &wt, "feat/busy");
}

/// R3: a lock is the first warning, with its reason. Without `--force`
/// nothing is removed and the lock stays; `--force` unlocks the worktree
/// and deletes it like any other.
#[test]
fn a_locked_worktree_shows_its_reason_and_force_unlocks_and_deletes_it() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fs::write(wt.join("notes.txt"), "draft\n").unwrap();
    git(&fixture.repo(), &["worktree", "lock", "--reason", "on a USB disk", wt.to_str().unwrap()]);

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    let lock = stderr
        .find("⚠ This worktree is locked (git worktree lock), with the reason: on a USB disk. Deleting it unlocks it.")
        .unwrap_or_else(|| panic!("the lock and its reason are shown: {stderr}"));
    let files = stderr.find("⚠ 1 uncommitted file will be lost: notes.txt").expect("the file is shown");
    assert!(lock < files, "the lock comes first: {stderr}");
    assert!(stderr.contains("Add --force to delete it anyway."), "{stderr}");
    assert!(listed_block(&fixture, &wt).unwrap().contains("locked"), "the lock stays");
    assert_nothing_removed(&fixture, &wt);

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x", "--force"]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Removed feat-x (") && stderr.contains("and deleted branch feat/x."), "{stderr}");
    assert_deleted(&fixture, &wt, "feat/x");
}

/// A lock reason comes from whoever locked the worktree: a terminal control
/// sequence and Prose markup in it are shown as literal text, and no escape
/// byte from it reaches the terminal (`NO_COLOR` keeps `wt`'s own styling
/// out of stderr too).
#[cfg(unix)]
#[test]
fn a_lock_reason_with_terminal_controls_and_markup_is_shown_literally() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    git(&fixture.repo(), &["worktree", "lock", "--reason", "\u{1b}[31m<b>x</b>", wt.to_str().unwrap()]);

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(!output.stderr.contains(&0x1b), "no escape byte is written: {stderr:?}");
    assert!(flowed(&stderr).contains(r"with the reason: \u{1b}[31m<b>x</b>. Deleting it unlocks it."), "{stderr}");
    assert_nothing_removed(&fixture, &wt);
}

/// The operation directories under the repository's removal store that
/// still hold a record (or a legacy journal): unfinished removals.
fn pending_removals(fixture: &Fixture) -> usize {
    let store = fixture.repo().join(".git").join("wt-removal");
    let Ok(entries) = fs::read_dir(store) else { return 0 };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join("record.json").exists() || entry.path().join("journal.jsonl").exists())
        .count()
}

// --- The spec's Examples table ----------------------------------------------

#[test]
fn example_clean_and_merged_removes_worktree_and_branch() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("chore/old-cleanup", "old-cleanup");
    fixture.commit(&wt, "cleanup.txt");
    git(&fixture.repo(), &["merge", "-q", "--no-ff", "-m", "merge", "chore/old-cleanup"]);

    fixture
        .wt(&fixture.repo())
        .args(["remove", "old-cleanup"])
        .assert()
        .code(0)
        .stdout("")
        .stderr(predicate::str::contains("Safe"))
        .stderr(predicate::str::contains("its commits are on main"))
        // Nothing to warn about: no question, and no note about late changes.
        .stderr(predicate::str::contains("⚠").not())
        .stderr(predicate::str::contains("Files created or changed before deletion finishes").not())
        .stderr(predicate::str::is_match(r"Removed old-cleanup \(\d+ entries\) and deleted branch chore/old-cleanup\.\n").unwrap())
        // The usage check and the move add nothing to a successful removal.
        .stderr(predicate::str::contains("Discovery").not())
        .stderr(predicate::str::contains("wt-remove").not())
        .stderr(predicate::str::contains("Retry:").not())
        .stderr(predicate::str::contains("report.json").not());
    assert_deleted(&fixture, &wt, "chore/old-cleanup");
}

/// A clean, merged removal starts this many Git processes, each 20–50 ms on
/// Windows. `GIT_TRACE` logs one `built-in` line per process:
///
/// - the report: `git status` for the file inventory, then the branch's
///   tier reads (`rev-list --left-right --count`, `log` of the commits only
///   it holds, `for-each-ref --contains`);
/// - the lifecycle: `git status` once more just before deletion, to name
///   files that changed after the question (R14), `git worktree remove` of
///   the moved-aside registration, and `git branch -D`.
///
/// Metadata (origin, refs, the common directory), every worktree listing,
/// and the plain index reads are read in-process, so no `git worktree list`
/// is started.
#[test]
fn a_clean_removal_starts_7_git_processes() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("chore/old-cleanup", "old-cleanup");
    let trace = fixture.root.path().join("git-trace.log");

    fixture.wt(&fixture.repo()).env("GIT_TRACE", &trace).args(["remove", "old-cleanup"]).assert().code(0);

    assert!(!wt.exists());
    let log = fs::read_to_string(&trace).unwrap();
    let started: Vec<&str> = log.lines().filter(|line| line.contains("trace: built-in: git ")).collect();
    assert_eq!(started.len(), 7, "{started:#?}");
    let listings = started.iter().filter(|line| line.contains("built-in: git worktree list")).count();
    assert_eq!(listings, 0, "{started:#?}");
    let statuses = started.iter().filter(|line| line.contains("built-in: git status")).count();
    assert_eq!(statuses, 2, "the report's and the late-change check's: {started:#?}");
}

#[test]
fn example_pushed_without_a_pr_is_pretty_safe_and_names_the_origin_copy() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("feat/dark-fixes", "feat-dark-fixes");
    fixture.commit(&wt, "dark.txt");
    fixture.push_upstream("feat/dark-fixes");

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-dark-fixes"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Pretty safe"))
        .stderr(predicate::str::contains("origin/feat/dark-fixes"))
        .stderr(predicate::str::contains("PR status unavailable"))
        .stderr(predicate::str::contains("and deleted branch feat/dark-fixes."));
    assert!(!wt.exists());
    assert!(!fixture.branch_exists("feat/dark-fixes"));
    assert!(fixture.origin_has("feat/dark-fixes"), "origin is never touched without --force-remote");
}

#[test]
fn example_unique_commits_without_a_terminal_keep_the_branch_with_a_warning() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("spike/parser", "spike-parser");
    for file in ["a.txt", "b.txt", "c.txt", "d.txt"] {
        fixture.commit(&wt, file);
    }

    fixture
        .wt(&fixture.repo())
        .args(["remove", "spike-parser"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Not safe"))
        .stderr(predicate::str::contains("4 commits exist nowhere else"))
        .stderr(predicate::str::contains("Warning:"))
        .stderr(predicate::str::contains("kept branch spike/parser"));
    assert!(!wt.exists(), "the worktree is still removed");
    assert!(fixture.branch_exists("spike/parser"));
}

#[test]
fn example_force_branch_deletes_unique_commits() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("spike/parser", "spike-parser");
    for file in ["a.txt", "b.txt", "c.txt", "d.txt"] {
        fixture.commit(&wt, file);
    }

    fixture
        .wt(&fixture.repo())
        .args(["remove", "spike-parser", "--force-branch"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("and deleted branch spike/parser (4 commits lost)."));
    assert!(!wt.exists());
    assert!(!fixture.branch_exists("spike/parser"));
}

#[test]
fn example_dirty_files_without_a_terminal_refuse_with_nothing_removed() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("fix/wt-ux", "fix-wt-ux");
    fs::write(wt.join("README.md"), "changed\n").unwrap();
    fs::write(wt.join("lib.rs"), "fn a() {}\n").unwrap();
    fs::write(wt.join("notes.txt"), "keep\n").unwrap();

    for ci in [None, Some("true")] {
        let mut cmd = fixture.wt(&fixture.repo());
        if let Some(ci) = ci {
            cmd.env("CI", ci);
        }
        cmd.args(["remove", "fix-wt-ux"])
            .assert()
            .code(3)
            .stdout("")
            .stderr(predicate::str::contains("⚠ 3 uncommitted files will be lost: README.md, lib.rs, notes.txt"))
            .stderr(predicate::str::contains("Files created or changed before deletion finishes are deleted too."))
            .stderr(predicate::str::contains("Nothing was removed. There is no terminal to ask"))
            .stderr(predicate::str::contains("Add --force to delete it anyway."));
    }
    assert!(wt.join("notes.txt").exists());
    assert!(fixture.branch_exists("fix/wt-ux"));
}

#[test]
fn example_all_three_flags_remove_worktree_branch_and_origin_branch() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("fix/wt-ux", "fix-wt-ux");
    fixture.commit(&wt, "fix.txt");
    fixture.push_upstream("fix/wt-ux");
    fs::write(wt.join("scratch.txt"), "x\n").unwrap();

    fixture
        .wt(&fixture.repo())
        .args([
            "remove",
            "fix-wt-ux",
            "--force",
            "--force-branch",
            "--force-remote",
        ])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("origin/fix/wt-ux will be deleted"))
        .stderr(predicate::str::contains("Deleted origin/fix/wt-ux"));
    assert!(!wt.exists());
    assert!(!fixture.branch_exists("fix/wt-ux"));
    assert!(!fixture.origin_has("fix/wt-ux"));
}

// --- Ignored entries, conflicts, and remote failures -------------------------

mod ordinary_ignored_entries_are_disposable {
    use super::*;

    const ROWS: [(&str, &str); 2] = [(".env", ".env"), ("target/", "target/debug/app")];

    fn check((ignore, make): (&str, &str)) {
        let fixture = Fixture::new();
        fs::write(fixture.repo().join(".gitignore"), format!("{ignore}\n")).unwrap();
        git(&fixture.repo(), &["add", ".gitignore"]);
        git(&fixture.repo(), &["commit", "-q", "-m", "ignore"]);
        let wt = fixture.add_worktree("feat/x", "feat-x");
        let file = wt.join(make);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "local only\n").unwrap();

        fixture
            .wt(&fixture.repo())
            .args(["remove", "feat-x"])
            .assert()
            .code(0)
            .stderr(predicate::str::contains("Also deletes ignored files").not());
        assert!(!wt.exists(), "{ignore}");
    }

    #[test]
    fn a_dotenv_file() {
        check(ROWS[0]);
    }

    #[test]
    fn a_target_directory() {
        check(ROWS[1]);
    }

    /// One test per row of `ROWS` (test-speed.md #2), so a row added
    /// there fails here until it has its own test above.
    #[test]
    fn every_row_has_a_test() {
        assert_eq!(ROWS.len(), 2);
    }
}

#[test]
fn ignored_children_in_tracked_directories_are_not_reported() {
    let fixture = Fixture::new();
    fs::write(fixture.repo().join(".gitignore"), b"*.log\ntarget/\n").unwrap();
    fs::create_dir_all(fixture.repo().join("darkmatter/src")).unwrap();
    fixture.commit(&fixture.repo(), "darkmatter/src/lib.rs");
    git(&fixture.repo(), &["add", ".gitignore"]);
    git(&fixture.repo(), &["commit", "-q", "-m", "ignore"]);
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fs::write(wt.join("darkmatter/build.log"), b"cache").unwrap();
    fs::create_dir_all(wt.join("target")).unwrap();
    fs::write(wt.join("target/app"), b"generated").unwrap();

    fixture.wt(&fixture.repo()).args(["remove", "feat-x"])
        .assert().code(0)
        .stderr(predicate::str::contains("Also deletes ignored files").not());
    assert!(!wt.exists());
}

#[test]
fn mixed_ignored_directory_reports_only_protected_files() {
    let fixture = Fixture::new();
    fs::write(fixture.repo().join(".gitignore"), b"config/\n").unwrap();
    fs::write(fixture.repo().join(".worktreeinclude"), b"config/.env\n").unwrap();
    git(&fixture.repo(), &["add", ".gitignore", ".worktreeinclude"]);
    git(&fixture.repo(), &["commit", "-q", "-m", "include rules"]);
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fs::create_dir_all(wt.join("config")).unwrap();
    fs::write(wt.join("config/.env"), b"secret").unwrap();
    fs::write(wt.join("config/cache.bin"), b"cache").unwrap();

    fixture.wt(&fixture.repo()).args(["remove", "feat-x"])
        .assert().code(3)
        .stderr(predicate::str::contains("config/.env (new)"))
        .stderr(predicate::str::contains("config/cache.bin").not());
    assert!(wt.join("config/.env").exists());
    assert!(wt.join("config/cache.bin").exists());
}

/// R28: `--force-branch` says nothing about the checkout, so a dirty
/// worktree still needs `--force` when nobody can be asked; together they
/// delete both.
#[test]
fn force_branch_on_a_dirty_worktree_without_force_refuses_with_nothing_removed() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fixture.commit(&wt, "unique.txt");
    fs::write(wt.join("scratch.txt"), "x\n").unwrap();

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-x", "--force-branch"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("⚠ 1 uncommitted file will be lost: scratch.txt"))
        .stderr(predicate::str::contains("Add --force to delete it anyway."));
    assert!(wt.join("scratch.txt").exists());
    assert_nothing_removed(&fixture, &wt);

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-x", "--force", "--force-branch"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("and deleted branch feat/x (1 commit lost)."));
    assert_deleted(&fixture, &wt, "feat/x");
}

/// R28: `--force` answers the checkout question only; a branch whose commits
/// exist nowhere else is still kept without `--force-branch`.
#[test]
fn force_alone_keeps_a_branch_whose_commits_exist_nowhere_else() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let tip = fixture.commit(&wt, "unique.txt");
    fs::write(wt.join("scratch.txt"), "x\n").unwrap();

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x", "--force"]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Not safe: 1 commit exists nowhere else"), "{stderr}");
    assert!(stderr.contains("Warning: kept branch feat/x: its commits exist nowhere else."), "{stderr}");
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert!(!stderr.contains("deleted branch"), "{stderr}");
    assert!(fs::symlink_metadata(&wt).is_err());
    assert!(listed_block(&fixture, &wt).is_none());
    assert_eq!(git(&fixture.repo(), &["rev-parse", "refs/heads/feat/x"]), tip);
}

#[test]
fn force_remote_with_an_unreachable_origin_removes_locally_then_exits_1() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fixture.push_upstream("feat/x");
    git(&fixture.repo(), &["remote", "set-url", "origin", "/nonexistent/origin.git"]);

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("could not be reached"))
        .stderr(predicate::str::contains("Removed feat-x ("))
        .stderr(predicate::str::contains("origin/feat/x was not deleted: "))
        .stderr(predicate::str::contains("Finish with git push origin --delete feat/x."));
    assert!(!wt.exists(), "the local removal has already happened");
}

#[test]
fn force_remote_without_a_remote_branch_says_so() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("feat/x", "feat-x");

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("no matching remote branch"))
        .stderr(predicate::str::contains("Nothing to delete on origin."));
    assert!(!wt.exists());
    assert!(!fixture.branch_exists("feat/x"), "on main, so Safe");
}

/// A branch that was Pretty safe only because it was pushed becomes Not safe
/// under `--force-remote`, so the local copy survives.
#[test]
fn force_remote_keeps_a_branch_whose_only_other_copy_it_deletes() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fixture.commit(&wt, "only-here.txt");
    fixture.push_upstream("feat/x");

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Not safe"))
        .stderr(predicate::str::contains("kept branch feat/x"))
        .stderr(predicate::str::contains("Deleted origin/feat/x"));
    assert!(fixture.branch_exists("feat/x"));
    assert!(!fixture.origin_has("feat/x"));
}

/// `feat/x` has one unique commit, pushed only as origin's `main`, and
/// tracks `origin/main`, so `--force-remote` deletes `main` on origin. Local
/// `main` is behind.
fn feature_tracking_origin_main(fixture: &Fixture) -> (PathBuf, String) {
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let tip = fixture.commit(&wt, "only-here.txt");
    fixture.push(&tip, "main");
    // What `git branch --set-upstream-to=origin/main feat/x` and `git remote
    // set-head origin main` write, without a process each.
    append_config(&fixture.repo(), "[branch \"feat/x\"]\n\tremote = origin\n\tmerge = refs/heads/main\n");
    fs::write(fixture.repo().join(".git/refs/remotes/origin/HEAD"), "ref: refs/remotes/origin/main\n").unwrap();
    // A bare repository refuses to delete its HEAD branch by default.
    append_to_config_file(&fixture.origin().join("config"), "[receive]\n\tdenyDeleteCurrent = ignore\n");
    (wt, tip)
}

fn assert_kept_after_deleting_origin_main(fixture: &Fixture, wt: &Path, tip: &str) {
    assert!(!wt.exists());
    assert!(!fixture.origin_has("main"));
    assert!(fixture.branch_exists("feat/x"), "the only remaining copy");
    assert_eq!(git(&fixture.repo(), &["rev-parse", "feat/x"]), tip);
}

/// The destination about to be deleted is `origin/main`, the default
/// branch's copy; it must not make the branch Safe.
#[test]
fn force_remote_of_the_upstream_default_branch_keeps_the_local_branch() {
    let fixture = Fixture::with_origin();
    let (wt, tip) = feature_tracking_origin_main(&fixture);

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Not safe"))
        .stderr(predicate::str::contains("kept branch feat/x"))
        .stderr(predicate::str::contains("Deleted origin/main"));
    assert_kept_after_deleting_origin_main(&fixture, &wt, &tip);
}

#[test]
fn force_remote_of_the_upstream_default_branch_keeps_the_local_branch_after_a_handoff() {
    let fixture = Fixture::with_origin();
    let (wt, tip) = feature_tracking_origin_main(&fixture);
    // The first run decides the branch is kept and says so; the second run
    // only carries out the remote step.
    let first = fixture
        .wt(&wt)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Not safe"))
        .stderr(predicate::str::contains("kept branch feat/x"))
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&first);

    fixture
        .wt(&landing)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Removed feat-x ("))
        .stderr(predicate::str::contains("deleted branch").not())
        .stderr(predicate::str::contains("Deleted origin/main"));
    assert_kept_after_deleting_origin_main(&fixture, &wt, &tip);
}

#[test]
fn force_remote_of_the_upstream_default_branch_accepts_an_independent_tag() {
    let fixture = Fixture::with_origin();
    let (wt, tip) = feature_tracking_origin_main(&fixture);
    // Lightweight, whatever the host's `tag.gpgSign`.
    git(&fixture.repo(), &["update-ref", "refs/tags/v1", &tip]);

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Pretty safe"))
        .stderr(predicate::str::contains("Deleted origin/main"));
    assert!(!wt.exists());
    assert!(!fixture.origin_has("main"));
    assert!(!fixture.branch_exists("feat/x"));
}

#[test]
fn a_detached_worktree_depends_only_on_its_files() {
    let fixture = Fixture::new();
    let path = fixture.root.path().join("wts").join("bisect");
    git(&fixture.repo(), &["worktree", "add", "-q", "--detach", path.to_str().unwrap(), "main"]);

    fixture
        .wt(&fixture.repo())
        .args(["remove", "bisect"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("no branch to delete"));
    assert!(!path.exists());
}

#[test]
fn the_report_names_ahead_behind_against_the_selected_target() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fixture.commit(&wt, "one.txt");
    fixture.commit(&wt, "two.txt");
    // origin/main moves ahead of local main (someone else's commit): it
    // becomes the target. One push writes both refs on origin and, through
    // origin's fetch refspec, both tracking refs, as a fetch would.
    let upstream = git(&fixture.repo(), &["commit-tree", "-p", "main", "-m", "upstream", "main^{tree}"]);
    fixture.push(&fixture.tip("feat/x"), "elsewhere");
    fixture.push(&upstream, "main");
    assert_eq!(git(&fixture.repo(), &["rev-parse", "origin/main"]), upstream);

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-x"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("2 ahead, 1 behind origin/main"))
        .stderr(predicate::str::contains("(as of your last fetch)"))
        .stderr(predicate::str::contains("origin/elsewhere"));
}

/// Item 4: merged into HEAD but not into its upstream. `git branch -d`
/// refused this with "not fully merged" and the old `-b` reported it as
/// preserved; the tiers say Safe, so it is deleted.
#[test]
fn a_branch_merged_into_head_but_not_its_upstream_is_deleted() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("fix/ci-build", "ci-build");
    fixture.commit(&wt, "one.txt");
    fixture.push_upstream("fix/ci-build");
    fixture.commit(&wt, "two.txt");
    git(&fixture.repo(), &["merge", "-q", "--no-ff", "-m", "merge", "fix/ci-build"]);

    fixture
        .wt(&fixture.repo())
        .args(["remove", "ci-build"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("and deleted branch fix/ci-build."))
        .stderr(predicate::str::contains("preserved").not());
    assert!(!fixture.branch_exists("fix/ci-build"));
}

// --- Move-first removal ------------------------------------------------------

fn protocol(stdout: &[u8]) -> (PathBuf, String) {
    let text = String::from_utf8_lossy(stdout);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "expected cd: then remove-handoff: in {text:?}");
    let cd = lines[0].strip_prefix("cd:").expect("cd: first");
    let token = lines[1].strip_prefix("remove-handoff:").expect("handoff second");
    (PathBuf::from(cd), token.to_string())
}

/// R15 on Unix: a shell standing in the worktree without the wrapper is a
/// warning (it must `cd` out afterward), so nobody can be asked here and
/// nothing is removed without `--force`; `--force` deletes it.
#[cfg(unix)]
#[test]
fn standing_inside_without_the_wrapper_is_a_warning_and_force_deletes() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");

    fixture
        .wt(&wt)
        .args(["remove", "feat-x"])
        .assert()
        .code(3)
        .stdout("")
        .stderr(predicate::str::contains("⚠ Your shell is inside this worktree; cd out of it afterward."))
        .stderr(predicate::str::contains("Nothing was removed."))
        .stderr(predicate::str::contains("Add --force to delete it anyway."));
    assert_nothing_removed(&fixture, &wt);

    fixture
        .wt(&wt)
        .args(["remove", "feat-x", "--force"])
        .assert()
        .code(0)
        .stdout("")
        .stderr(predicate::str::contains("Removed feat-x ("));
    assert_deleted(&fixture, &wt, "feat/x");
}

/// R15 on Windows: the shell's working directory blocks deleting the
/// folder, so the run refuses before any question (exit 4), even with
/// `--force`.
#[cfg(windows)]
#[test]
fn standing_inside_without_the_wrapper_on_windows_refuses_with_exit_4() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");

    for flags in [&[][..], &["--force"]] {
        fixture
            .wt(&wt)
            .args(["remove", "feat-x"])
            .args(flags)
            .assert()
            .code(4)
            .stdout("")
            .stderr(predicate::str::contains("Nothing was removed"))
            .stderr(predicate::str::contains("Windows cannot delete a folder a program is using"));
        assert_nothing_removed(&fixture, &wt);
    }
}

#[test]
fn standing_inside_with_the_wrapper_hands_off_then_finishes_from_the_landing() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.repo().join("docs")).unwrap();
    fixture.commit(&fixture.repo().join("docs"), ".keep");
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let inside = wt.join("docs");

    let first = fixture
        .wt(&inside)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Moving you to the base repo"))
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&first);
    // The subdirectory is kept because it exists in the base repo too.
    assert_eq!(canonical(&landing), canonical(&fixture.repo().join("docs")));
    assert!(wt.exists(), "the first run removes nothing");
    assert_eq!(pending_removals(&fixture), 1, "the first run saved its approval");

    fixture
        .wt(&landing)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stdout("")
        .stderr(predicate::str::contains("Removed feat-x ("))
        .stderr(predicate::str::contains("and deleted branch feat/x."));
    assert_deleted(&fixture, &wt, "feat/x");

    // The token was consumed.
    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("There is no pending removal for this handoff token."));
}

#[test]
fn the_handoff_lands_in_the_fork_parent_worktree() {
    let fixture = Fixture::new();
    let parent = fixture.add_worktree("feat/theme", "feat-theme");
    let child = fixture.add_worktree("feat/dark-fixes", "feat-dark-fixes");
    let store = worktree::fork_origin::fork_origin_path(&fixture.repo()).unwrap();
    worktree::fork_origin::record(
        &store,
        "feat/dark-fixes",
        worktree::fork_origin::ForkOrigin {
            base_branch: "feat/theme".into(),
            base_sha: git(&fixture.repo(), &["rev-parse", "feat/theme"]),
            created_at: 0,
        },
    )
    .unwrap();

    let first = fixture
        .wt(&child)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-dark-fixes"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("feat/theme worktree"))
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&first);
    assert_eq!(canonical(&landing), canonical(&parent));

    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0);
    assert!(!child.exists());
    assert!(parent.exists());
}

fn first_run(fixture: &Fixture, wt: &Path, extra: &[&str]) -> (PathBuf, String) {
    let out = fixture
        .wt(wt)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x"])
        .args(extra)
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    protocol(&out)
}

/// The branch was approved at the tip the first run showed; a commit made
/// between the runs was never assessed. The checkout is still deleted, the
/// branch is kept (exit 1).
#[test]
fn a_changed_branch_tip_between_the_runs_deletes_the_checkout_and_keeps_the_branch() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let (landing, token) = first_run(&fixture, &wt, &[]);

    let moved = fixture.commit(&wt, "late.txt");
    let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert!(stderr.contains(&format!("Branch feat/x was kept: it moved to {moved} after it was checked.")), "{stderr}");
    assert!(stderr.contains("Delete it with git branch -D feat/x if you no longer need it."), "{stderr}");
    assert_checkout_gone(&fixture, &wt);
    assert_eq!(git(&fixture.repo(), &["rev-parse", "refs/heads/feat/x"]), moved);
}

#[test]
fn a_new_disposable_ignored_entry_between_the_runs_does_not_refuse() {
    let fixture = Fixture::new();
    fs::write(fixture.repo().join(".gitignore"), ".env\n").unwrap();
    git(&fixture.repo(), &["add", ".gitignore"]);
    git(&fixture.repo(), &["commit", "-q", "-m", "ignore"]);
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let (landing, token) = first_run(&fixture, &wt, &[]);

    fs::write(wt.join(".env"), "SECRET=1\n").unwrap();
    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0);
    assert!(!wt.exists());
}

/// Restaging keeps the status (`MM`) and the working bytes. The second run
/// compares nothing about the content (R18), so the new staged version is
/// deleted with the checkout.
#[test]
fn a_restaged_version_between_the_runs_is_deleted_by_the_second_run() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let stage_then_edit = |staged: &str| {
        fs::write(wt.join("README.md"), staged).unwrap();
        git(&wt, &["add", "README.md"]);
        fs::write(wt.join("README.md"), "working copy\n").unwrap();
        assert_eq!(git(&wt, &["status", "--porcelain=v1"]), "MM README.md");
    };
    stage_then_edit("staged before\n");
    let (landing, token) = first_run(&fixture, &wt, &["--force"]);

    stage_then_edit("new staged work\n");
    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Removed feat-x ("));
    assert_deleted(&fixture, &wt, "feat/x");
}

/// Git lists an untracked nested repository as one `?? nested/` entry, so
/// `git status` never sees a change to its contents.
fn worktree_with_nested_repo(fixture: &Fixture) -> PathBuf {
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let nested = wt.join("nested");
    fs::create_dir(&nested).unwrap();
    git(&nested, &["init", "-q", "-b", "main"]);
    fs::write(nested.join("notes"), "approved content\n").unwrap();
    assert_eq!(git(&wt, &["status", "--porcelain=v1", "-uall"]), "?? nested/");
    wt
}

#[test]
fn changes_inside_an_untracked_nested_repo_between_the_runs_are_deleted_by_the_second_run() {
    let fixture = Fixture::new();
    let wt = worktree_with_nested_repo(&fixture);
    let (landing, token) = first_run(&fixture, &wt, &["--force"]);

    fs::write(wt.join("nested/notes"), "new work after approval\n").unwrap();
    fs::write(wt.join("nested/new-file"), "new work\n").unwrap();
    assert_eq!(git(&wt, &["status", "--porcelain=v1", "-uall"]), "?? nested/");
    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Removed feat-x ("));
    assert_deleted(&fixture, &wt, "feat/x");
}

#[test]
fn an_unchanged_untracked_nested_repo_is_removed_by_the_handoff() {
    let fixture = Fixture::new();
    let wt = worktree_with_nested_repo(&fixture);
    let (landing, token) = first_run(&fixture, &wt, &["--force"]);

    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Removed feat-x ("));
    assert_deleted(&fixture, &wt, "feat/x");
}

/// A file the second run cannot read is deleted with the rest (R13, R18):
/// nothing reads it any more.
#[cfg(unix)]
#[test]
fn an_unreadable_file_inside_a_nested_repo_is_deleted_by_the_handoff() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new();
    let wt = worktree_with_nested_repo(&fixture);
    let (landing, token) = first_run(&fixture, &wt, &["--force"]);

    let notes = wt.join("nested/notes");
    fs::set_permissions(&notes, fs::Permissions::from_mode(0o000)).unwrap();
    let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
    let _ = fs::set_permissions(&notes, fs::Permissions::from_mode(0o644));

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert_deleted(&fixture, &wt, "feat/x");
}

/// 1.5 MiB of varied bytes: past the 1 MiB size a metadata shortcut was once
/// proposed for.
fn large_contents() -> Vec<u8> {
    (0..1_572_864_u32).map(|index| (index % 251) as u8).collect()
}

/// Flips bytes at the start, middle, and end of `path` and then restores
/// its modification time, so neither its size nor its time shows the edit.
fn edit_keeping_size_and_mtime(path: &Path) {
    let before = fs::metadata(path).unwrap();
    let modified = before.modified().unwrap();
    let mut bytes = fs::read(path).unwrap();
    let last = bytes.len() - 1;
    for index in [0, bytes.len() / 2, last] {
        bytes[index] ^= 0x01;
    }
    fs::write(path, &bytes).unwrap();
    // A separate handle, so no later write through it can move the time.
    fs::OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    let after = fs::metadata(path).unwrap();
    assert_eq!(after.len(), before.len(), "{path:?} kept its size");
    assert_eq!(after.modified().unwrap(), modified, "{path:?} kept its modification time");
}

/// The spec's old fingerprint scenario: an edit that keeps both size and
/// modification time, to a large dirty file and to files inside an untracked
/// nested repository. The second run compares no content (R18), so the edit
/// is deleted with the checkout, and the consumed token cannot be replayed.
mod a_same_size_same_mtime_edit_between_the_runs_is_deleted_by_the_second_run {
    use super::*;

    const ROWS: [&str; 4] = ["a large untracked file", "a large modified tracked file", "a file inside an untracked nested repo", "a large file inside an untracked nested repo"];

    fn check(case: &str) {
        let fixture = Fixture::new();
        let (wt, edited) = match case {
            "a large untracked file" => {
                let wt = fixture.add_worktree("feat/x", "feat-x");
                fs::write(wt.join("big.bin"), large_contents()).unwrap();
                (wt.clone(), wt.join("big.bin"))
            }
            "a large modified tracked file" => {
                fs::write(fixture.repo().join("big.bin"), large_contents()).unwrap();
                git(&fixture.repo(), &["add", "big.bin"]);
                git(&fixture.repo(), &["commit", "-q", "-m", "big"]);
                let wt = fixture.add_worktree("feat/x", "feat-x");
                let mut dirty = large_contents();
                dirty[10] ^= 0x02;
                fs::write(wt.join("big.bin"), dirty).unwrap();
                assert_eq!(git(&wt, &["status", "--porcelain=v1"]), "M big.bin");
                (wt.clone(), wt.join("big.bin"))
            }
            "a file inside an untracked nested repo" => {
                let wt = worktree_with_nested_repo(&fixture);
                (wt.clone(), wt.join("nested/notes"))
            }
            _ => {
                let wt = worktree_with_nested_repo(&fixture);
                fs::write(wt.join("nested/big.bin"), large_contents()).unwrap();
                (wt.clone(), wt.join("nested/big.bin"))
            }
        };
        let status = git(&wt, &["status", "--porcelain=v1", "-uall"]);
        let (landing, token) = first_run(&fixture, &wt, &["--force"]);

        edit_keeping_size_and_mtime(&edited);
        assert_eq!(git(&wt, &["status", "--porcelain=v1", "-uall"]), status, "{case}: git sees no change");
        fixture
            .wt(&landing)
            .args(["remove", "--handoff", &token])
            .assert()
            .code(0)
            .stderr(predicate::str::contains("Removed feat-x ("));
        assert_deleted(&fixture, &wt, "feat/x");

        fixture
            .wt(&landing)
            .args(["remove", "--handoff", &token])
            .assert()
            .code(3)
            .stderr(predicate::str::contains("There is no pending removal for this handoff token."));
    }

    #[test]
    fn a_large_untracked_file() {
        check(ROWS[0]);
    }

    #[test]
    fn a_large_modified_tracked_file() {
        check(ROWS[1]);
    }

    #[test]
    fn a_file_inside_an_untracked_nested_repo() {
        check(ROWS[2]);
    }

    #[test]
    fn a_large_file_inside_an_untracked_nested_repo() {
        check(ROWS[3]);
    }

    /// One test per row of `ROWS` (test-speed.md #2), so a row added
    /// there fails here until it has its own test above.
    #[test]
    fn every_row_has_a_test() {
        assert_eq!(ROWS.len(), 4);
    }
}

/// Keeps `path` unreadable (mode 000) while alive. Unix only: on Windows a
/// file is made unreadable by holding it open without sharing, which the
/// removal waits on until it is closed (R22), a different scenario.
#[cfg(unix)]
struct Unreadable {
    path: PathBuf,
}

#[cfg(unix)]
impl Unreadable {
    /// `None` when the file stays readable (root ignores mode bits).
    fn new(path: &Path) -> Option<Self> {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o000)).unwrap();
        let this = Self { path: path.to_path_buf() };
        fs::File::open(path).is_err().then_some(this)
    }
}

#[cfg(unix)]
impl Drop for Unreadable {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(0o644));
    }
}

/// A dirty file the second run cannot read, at the top level or inside an
/// untracked nested repository, is deleted with the checkout: the second
/// run reads no content (R13, R18).
#[cfg(unix)]
mod an_unreadable_dirty_file_between_the_runs_is_deleted_by_the_second_run {
    use super::*;

    const ROWS: [&str; 2] = ["an untracked file", "a file inside an untracked nested repo"];

    fn check(case: &str) {
        let fixture = Fixture::new();
        let (wt, dirty) = if case == "an untracked file" {
            let wt = fixture.add_worktree("feat/x", "feat-x");
            fs::write(wt.join("scratch.txt"), "approved scratch\n").unwrap();
            (wt.clone(), wt.join("scratch.txt"))
        } else {
            let wt = worktree_with_nested_repo(&fixture);
            (wt.clone(), wt.join("nested/notes"))
        };
        let (landing, token) = first_run(&fixture, &wt, &["--force"]);

        let Some(unreadable) = Unreadable::new(&dirty) else {
            eprintln!("skipping {case}: the file stays readable to this user");
            return;
        };
        let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
        drop(unreadable);

        let stderr = stderr_of(&output);
        assert_eq!(output.status.code(), Some(0), "{case}: {stderr}");
        assert!(stderr.contains("Removed feat-x ("), "{case}: {stderr}");
        assert_deleted(&fixture, &wt, "feat/x");
    }

    #[test]
    fn an_untracked_file() {
        check(ROWS[0]);
    }

    #[test]
    fn a_file_inside_an_untracked_nested_repo() {
        check(ROWS[1]);
    }

    /// One test per row of `ROWS` (test-speed.md #2), so a row added
    /// there fails here until it has its own test above.
    #[test]
    fn every_row_has_a_test() {
        assert_eq!(ROWS.len(), 2);
    }
}

/// Paths that are not UTF-8. `\xff` and `\xfe` both decode lossily to
/// U+FFFD; the second run compares no content (R18), so a change between
/// the runs is deleted with the checkout whatever bytes name it.
#[cfg(unix)]
mod non_utf8_paths {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::symlink;

    use super::*;

    const APPROVED: &[u8] = b"target-\xff";
    const CHANGED: &[u8] = b"target-\xfe";

    fn relink(link: &Path, target: &[u8]) {
        let _ = fs::remove_file(link);
        symlink(OsStr::from_bytes(target), link).unwrap();
    }

    /// A file named by `name`, or `None` (with a note) on a filesystem that
    /// refuses names that are not UTF-8, as macOS APFS does.
    fn create(dir: &Path, name: &[u8], content: &str) -> Option<PathBuf> {
        let path = dir.join(OsStr::from_bytes(name));
        match fs::write(&path, content) {
            Ok(()) => Some(path),
            Err(error) => {
                eprintln!("skipped: this filesystem refuses a non-UTF-8 file name ({error})");
                None
            }
        }
    }

    fn assert_deleted_by_the_second_run(fixture: &Fixture, wt: &Path, landing: &Path, token: &str) {
        fixture
            .wt(landing)
            .args(["remove", "--handoff", token])
            .assert()
            .code(0)
            .stderr(predicate::str::contains("Removed feat-x ("));
        assert_deleted(fixture, wt, "feat/x");
    }

    #[test]
    fn a_changed_symlink_target_between_the_runs_is_deleted_by_the_second_run() {
        let fixture = Fixture::new();
        let wt = fixture.add_worktree("feat/x", "feat-x");
        let link = wt.join("link");
        relink(&link, APPROVED);
        let (landing, token) = first_run(&fixture, &wt, &["--force"]);

        relink(&link, CHANGED);
        assert_deleted_by_the_second_run(&fixture, &wt, &landing, &token);
    }

    #[test]
    fn a_changed_symlink_target_inside_an_untracked_nested_repo_is_deleted_by_the_second_run() {
        let fixture = Fixture::new();
        let wt = worktree_with_nested_repo(&fixture);
        let link = wt.join("nested/link");
        relink(&link, APPROVED);
        let (landing, token) = first_run(&fixture, &wt, &["--force"]);

        relink(&link, CHANGED);
        assert_eq!(git(&wt, &["status", "--porcelain=v1", "-uall"]), "?? nested/");
        assert_deleted_by_the_second_run(&fixture, &wt, &landing, &token);
    }

    #[test]
    fn an_unchanged_non_utf8_symlink_target_is_removed_by_the_handoff() {
        let fixture = Fixture::new();
        let wt = fixture.add_worktree("feat/x", "feat-x");
        relink(&wt.join("link"), APPROVED);
        let (landing, token) = first_run(&fixture, &wt, &["--force"]);

        assert_deleted_by_the_second_run(&fixture, &wt, &landing, &token);
    }

    #[test]
    fn an_edited_non_utf8_file_name_between_the_runs_is_deleted_by_the_second_run() {
        let fixture = Fixture::new();
        let wt = fixture.add_worktree("feat/x", "feat-x");
        let Some(file) = create(&wt, b"notes-\xff", "approved content\n") else {
            return;
        };
        let (landing, token) = first_run(&fixture, &wt, &["--force"]);

        fs::write(&file, "new work after approval\n").unwrap();
        assert_deleted_by_the_second_run(&fixture, &wt, &landing, &token);
    }

    #[test]
    fn a_nested_child_renamed_to_a_name_with_the_same_lossy_text_is_deleted_by_the_second_run() {
        let fixture = Fixture::new();
        let wt = worktree_with_nested_repo(&fixture);
        let nested = wt.join("nested");
        let Some(approved) = create(&nested, b"child-\xff", "same content\n") else {
            return;
        };
        let (landing, token) = first_run(&fixture, &wt, &["--force"]);

        fs::rename(&approved, nested.join(OsStr::from_bytes(b"child-\xfe"))).unwrap();
        assert_deleted_by_the_second_run(&fixture, &wt, &landing, &token);
    }
}

/// A chmod keeps a modified tracked file's status (` M`), its bytes, and its
/// index entry, so only the working file's mode tells the states apart. The
/// second run compares no content (R18), so neither mode stops it.
#[cfg(unix)]
mod executable_bit {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn set_mode(path: &Path, mode: u32) {
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }

    /// `run.sh` committed at `100644` with `core.filemode=true`, then edited
    /// without staging in `feat-x`.
    fn worktree_with_modified_script(fixture: &Fixture) -> (PathBuf, PathBuf) {
        let repo = fixture.repo();
        git(&repo, &["config", "core.filemode", "true"]);
        fs::write(repo.join("run.sh"), "echo one\n").unwrap();
        set_mode(&repo.join("run.sh"), 0o644);
        git(&repo, &["add", "run.sh"]);
        git(&repo, &["commit", "-q", "-m", "script"]);
        let wt = fixture.add_worktree("feat/x", "feat-x");
        let script = wt.join("run.sh");
        fs::write(&script, "echo edited\n").unwrap();
        set_mode(&script, 0o644);
        assert_eq!(git(&wt, &["status", "--porcelain=v1"]), "M run.sh");
        (wt, script)
    }

    #[test]
    fn a_changed_executable_bit_between_the_runs_is_deleted_by_the_second_run() {
        let fixture = Fixture::new();
        let (wt, script) = worktree_with_modified_script(&fixture);
        let (landing, token) = first_run(&fixture, &wt, &["--force"]);

        set_mode(&script, 0o755);
        assert_eq!(git(&wt, &["status", "--porcelain=v1"]), "M run.sh");
        assert!(git(&wt, &["diff", "--summary"]).contains("mode change 100644 => 100755"));
        fixture
            .wt(&landing)
            .args(["remove", "--handoff", &token])
            .assert()
            .code(0)
            .stderr(predicate::str::contains("Removed feat-x ("));
        assert_deleted(&fixture, &wt, "feat/x");
    }

    #[test]
    fn an_unchanged_executable_bit_is_removed_by_the_handoff() {
        let fixture = Fixture::new();
        let (wt, _script) = worktree_with_modified_script(&fixture);
        let (landing, token) = first_run(&fixture, &wt, &["--force"]);

        fixture
            .wt(&landing)
            .args(["remove", "--handoff", &token])
            .assert()
            .code(0)
            .stderr(predicate::str::contains("Removed feat-x ("));
        assert_deleted(&fixture, &wt, "feat/x");
    }
}

/// An expired token changes nothing (exit 3), and the approval it carried
/// is still saved: the next `wt remove` of the name finishes it without
/// asking.
#[test]
fn an_expired_token_refuses_with_exit_3_and_a_later_remove_finishes_it() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let (landing, token) = first_run(&fixture, &wt, &[]);

    let record = worktree::remove::handoff::handoff_path(&fixture.repo(), &token).unwrap();
    let text = fs::read_to_string(&record).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&text).unwrap();
    json["created_at"] = serde_json::json!(1);
    fs::write(&record, serde_json::to_vec(&json).unwrap()).unwrap();

    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("The removal handoff expired before the shell moved."))
        .stderr(predicate::str::contains("Run wt remove with the worktree's name to finish it."));
    assert_nothing_removed(&fixture, &wt);
    assert!(!record.exists(), "an expired record is still consumed");
    assert_eq!(pending_removals(&fixture), 1, "the approval stays");

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert!(!stderr.contains("Removing worktree"), "finished from its saved approval, with no new report: {stderr}");
    assert_deleted(&fixture, &wt, "feat/x");
}

/// Another program stands in the worktree (a `sleep` started there). The
/// first wrapper run lists it as a program using the worktree (R1), so
/// without a terminal it needs `--force`; with it, the run hands off, and
/// the second run, after the program exited, removes.
#[cfg(unix)]
#[test]
fn another_program_inside_is_listed_and_force_hands_off_to_a_second_run_that_removes() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let shell = Holder::standing_in(&wt);

    let output = fixture.wt(&wt).env("WT_SHELL_WRAPPER", "1").args(["remove", "feat-x"]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains(&format!("sleep (pid {}", shell.pid())), "{stderr}");
    assert!(output.stdout.is_empty(), "no protocol lines after a refusal");

    let (landing, token) = first_run(&fixture, &wt, &["--force"]);
    drop(shell);
    let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert!(output.stdout.is_empty(), "the second run prints no protocol lines");
    assert_deleted(&fixture, &wt, "feat/x");
}

/// The caller's own shell standing in the worktree is not "a program using
/// it": the wrapper moves it out. `wt` runs as the child of an `sh` whose
/// working directory is the worktree; the clean worktree hands off without
/// `--force` and without listing the shell.
#[cfg(unix)]
#[test]
fn the_callers_own_shell_inside_is_not_listed_and_the_wrapper_run_hands_off() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    // A second command keeps `sh` from replacing itself with `wt`, so `sh`
    // stays `wt`'s parent.
    let output = Command::new("sh")
        .arg("-c")
        .arg("\"$0\" remove feat-x; status=$?; exit $status")
        .arg(env!("CARGO_BIN_EXE_wt"))
        .current_dir(&wt)
        .env("WT_SHELL_WRAPPER", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("NO_COLOR", "1")
        .env_remove("CI")
        .env_remove("COMPLETE")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(!stderr.contains("using files in this worktree"), "{stderr}");
    let (_, token) = protocol(&output.stdout);
    assert!(!token.is_empty());
    assert!(wt.exists(), "the first run deletes nothing");
}

/// A different process still using the worktree when the second run starts
/// does not stop it (R18): the first run's approval covers the checkout,
/// and the second run prints no protocol line.
#[cfg(unix)]
#[test]
fn a_process_still_inside_after_the_handoff_does_not_stop_the_second_run() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let (landing, token) = first_run(&fixture, &wt, &[]);
    let holder = Holder::standing_in(&wt);

    let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();

    drop(holder);
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert!(output.stdout.is_empty(), "no cd: or remove-handoff: from the second run");
    assert_deleted(&fixture, &wt, "feat/x");
}

#[test]
fn a_caller_still_inside_the_target_is_refused_with_exit_3() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let (_landing, token) = first_run(&fixture, &wt, &[]);

    fixture
        .wt(&wt)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "Nothing was removed: the shell is still inside the worktree, so it was not removed.",
        ));
    assert_nothing_removed(&fixture, &wt);
}

#[test]
fn a_missing_or_malformed_token_refuses_with_exit_3() {
    let fixture = Fixture::new();
    for token in ["0".repeat(32), "../../escape".to_string()] {
        fixture
            .wt(&fixture.repo())
            .args(["remove", "--handoff", &token])
            .assert()
            .code(3)
            .stderr(predicate::str::contains("Nothing was removed. There is no pending removal for this handoff token."));
    }
}

#[test]
fn the_handoff_carries_force_flags_to_the_second_run() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fixture.commit(&wt, "unique.txt");
    fs::write(wt.join("scratch.txt"), "x\n").unwrap();
    let first = fixture
        .wt(&wt)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x", "--force", "--force-branch"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("1 commit exists nowhere else"))
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&first);
    assert!(wt.join("scratch.txt").exists());

    // The first run named the lost commit and the caller approved it; the
    // second run does not reassess safety, so it claims no count.
    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("and deleted branch feat/x."))
        .stderr(predicate::str::contains("lost").not());
    assert_deleted(&fixture, &wt, "feat/x");
}

/// A branch approved because it was safe is checked again by the second
/// run; once its only other copy is gone, the checkout is still deleted and
/// the branch kept (exit 1).
#[test]
fn a_branch_that_stops_being_safe_between_the_runs_is_kept_after_the_checkout_is_deleted() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let tip = fixture.commit(&wt, "unique.txt");
    git(&fixture.repo(), &["branch", "backup", &tip]);
    let (landing, token) = first_run(&fixture, &wt, &[]);

    // The only other copy disappears.
    git(&fixture.repo(), &["branch", "-D", "backup"]);
    let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert!(stderr.contains("Branch feat/x was kept: it is no longer safe to delete"), "{stderr}");
    assert!(stderr.contains("Delete it with git branch -D feat/x if you no longer need it."), "{stderr}");
    assert_checkout_gone(&fixture, &wt);
    assert_eq!(git(&fixture.repo(), &["rev-parse", "refs/heads/feat/x"]), tip);
}

/// Two origin branches at one commit. The approved remote step is bound to
/// the destination the first run reported, so a new upstream between the
/// runs changes nothing about which branch on origin is deleted.
#[test]
fn a_changed_upstream_between_the_runs_deletes_only_the_approved_origin_branch() {
    let fixture = Fixture::with_origin();
    fixture.push(&fixture.tip("main"), "approved");
    fixture.push(&fixture.tip("main"), "unapproved");
    let wt = fixture.add_worktree("feat/x", "feat-x");
    git(&fixture.repo(), &["branch", "--set-upstream-to=origin/approved", "feat/x"]);
    let (landing, token) = first_run(&fixture, &wt, &["--force-remote"]);

    git(&fixture.repo(), &["branch", "--set-upstream-to=origin/unapproved", "feat/x"]);
    let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Deleted origin/approved"), "{stderr}");
    assert!(!fixture.origin_has("approved"));
    assert!(fixture.origin_has("unapproved"), "never approved, never deleted");
    assert_deleted(&fixture, &wt, "feat/x");
}

// --- What the second run proves again -----------------------------------------

/// `text` with every soft-wrapped line joined to the one before it, so a
/// needle never straddles the report's wrap at the terminal width.
fn flowed(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if !out.is_empty() && !out.ends_with('\n') && line.len() > trimmed.len() && !trimmed.is_empty() {
            out.push(' ');
            out.push_str(trimmed);
        } else {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(line);
        }
    }
    out
}

/// The checkout is gone with its registration, no unfinished removal is
/// left, and no hidden deletion directory remains beside it.
fn assert_checkout_gone(fixture: &Fixture, wt: &Path) {
    assert!(fs::symlink_metadata(wt).is_err(), "the worktree path is gone: {wt:?}");
    assert!(listed_block(fixture, wt).is_none(), "unregistered:\n{}", porcelain(fixture));
    assert_eq!(pending_removals(fixture), 0, "no unfinished removal is left");
    let leftovers: Vec<_> = fs::read_dir(wt.parent().unwrap())
        .map(|entries| entries.flatten().map(|entry| entry.file_name()).collect())
        .unwrap_or_default();
    assert!(
        !leftovers.iter().any(|name| name.to_string_lossy().starts_with(".wt-remove-")),
        "no hidden deletion directory is left: {leftovers:?}"
    );
}

/// [`assert_checkout_gone`], and `branch` deleted too.
fn assert_deleted(fixture: &Fixture, wt: &Path, branch: &str) {
    assert_checkout_gone(fixture, wt);
    assert!(!fixture.branch_exists(branch), "branch {branch} is deleted");
}

/// The worktree directory, its registration, and the branch are all intact.
fn assert_nothing_removed(fixture: &Fixture, wt: &Path) {
    assert!(wt.exists(), "the worktree directory stays");
    assert!(
        git(&fixture.repo(), &["worktree", "list", "--porcelain"]).contains("branch refs/heads/feat/x"),
        "the worktree stays registered"
    );
    assert!(fixture.branch_exists("feat/x"), "the branch stays");
}

/// A clone of `origin` standing in for someone else's pushes.
fn pusher(fixture: &Fixture) -> PathBuf {
    let pusher = fixture.root.path().join("pusher");
    git(fixture.root.path(), &["clone", "-q", fixture.origin().to_str().unwrap(), "pusher"]);
    configure(&pusher);
    pusher
}

/// The first run trusts `origin/main` as of the last fetch; the second run
/// must ask origin, because the tracking ref cannot see a force-push made
/// after the approval. The checkout is deleted either way; the branch is
/// kept (exit 1).
#[test]
fn a_branch_proved_only_by_origin_default_is_kept_once_origin_drops_the_tip() {
    let fixture = Fixture::with_origin();
    let old_main = git(&fixture.repo(), &["rev-parse", "main"]);
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let tip = fixture.commit(&wt, "only-here.txt");
    fixture.push(&tip, "main");
    assert_eq!(git(&fixture.repo(), &["rev-parse", "origin/main"]), tip);
    assert_eq!(git(&fixture.repo(), &["rev-parse", "main"]), old_main, "local main is older");

    let first = fixture
        .wt(&wt)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("its commits are on origin/main"))
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&first);

    fixture.others_push("main", Some(&old_main));
    assert_eq!(git(&fixture.repo(), &["rev-parse", "origin/main"]), tip, "the tracking ref is unchanged");

    let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert!(stderr.contains("Branch feat/x was kept: it is no longer safe to delete"), "{stderr}");
    assert!(stderr.contains("origin/main has moved"), "{stderr}");
    assert!(!stderr.contains("Detached HEAD"), "{stderr}");
    assert_checkout_gone(&fixture, &wt);
    assert_eq!(git(&fixture.repo(), &["rev-parse", "feat/x"]), tip);
}

#[test]
fn a_branch_proved_only_by_origin_default_is_deleted_while_origin_still_has_it() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let tip = fixture.commit(&wt, "only-here.txt");
    fixture.push(&tip, "main");
    let (landing, token) = first_run(&fixture, &wt, &[]);

    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("and deleted branch feat/x."));
    assert_deleted(&fixture, &wt, "feat/x");
}

/// `feat/x` pushed to origin at the worktree's tip (main's commit, so the
/// local branch is Safe on its own), and the handoff approved with
/// `--force-remote` while origin had it.
fn approved_with_origin_branch_present(fixture: &Fixture) -> (PathBuf, PathBuf, String) {
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fixture.push_upstream("feat/x");
    let (landing, token) = first_run(fixture, &wt, &["--force-remote"]);
    (wt, landing, token)
}

/// Someone else deleted the approved branch on origin between the runs.
#[test]
fn an_origin_branch_deleted_between_the_runs_does_not_stop_the_removal() {
    let fixture = Fixture::with_origin();
    let (wt, landing, token) = approved_with_origin_branch_present(&fixture);

    fixture.others_push("feat/x", None);
    let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert!(!stderr.contains("Detached HEAD"), "{stderr}");
    assert_eq!(output.status.code(), Some(0), "an absent branch counts as done: {stderr}");
    assert_deleted(&fixture, &wt, "feat/x");
    assert!(!fixture.origin_has("feat/x"));
}

/// The push URL still reads the same, but nothing answers there: the
/// checkout and the local branch go, and the branch on origin is kept
/// (exit 1) with how to finish.
#[test]
fn an_unreachable_origin_in_the_second_run_keeps_the_origin_branch_after_the_checkout_is_deleted() {
    let fixture = Fixture::with_origin();
    let (wt, landing, token) = approved_with_origin_branch_present(&fixture);

    let moved = fixture.root.path().join("moved.git");
    fs::rename(fixture.origin(), &moved).unwrap();
    let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
    fs::rename(&moved, fixture.origin()).unwrap();

    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert!(stderr.contains("origin/feat/x was kept: "), "{stderr}");
    assert!(stderr.contains("Delete it with git push origin --delete feat/x if you no longer need it."), "{stderr}");
    assert_deleted(&fixture, &wt, "feat/x");
    assert!(fixture.origin_has("feat/x"));
}

/// Someone else pushed to the approved branch between the runs: the
/// deletion's lease fails, so their commit survives on origin, and the
/// checkout is deleted anyway (exit 1).
#[test]
fn an_origin_branch_moved_between_the_runs_is_kept_by_the_lease() {
    let fixture = Fixture::with_origin();
    let (wt, landing, token) = approved_with_origin_branch_present(&fixture);

    let pusher = pusher(&fixture);
    git(&pusher, &["checkout", "-q", "-b", "feat/x", "origin/feat/x"]);
    let pushed = fixture.commit(&pusher, "their-work.txt");
    git(&pusher, &["push", "-q", "origin", "feat/x"]);
    let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert!(stderr.contains("origin/feat/x was kept: "), "{stderr}");
    assert!(!stderr.contains("Deleted origin/feat/x"), "{stderr}");
    assert_deleted(&fixture, &wt, "feat/x");
    assert_eq!(TwoRemotes::head_in(&fixture.origin(), "feat/x"), Some(pushed));
}

#[test]
fn an_origin_branch_still_absent_in_the_second_run_leaves_nothing_to_delete() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let first = fixture
        .wt(&wt)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Nothing to delete on origin."))
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&first);

    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Removed feat-x ("))
        .stderr(predicate::str::contains("origin/").not());
    assert_deleted(&fixture, &wt, "feat/x");
    assert!(!fixture.origin_has("feat/x"));
}

/// Origin had no branch to delete when the first run asked, so no remote
/// step was approved; one created between the runs is never deleted.
#[test]
fn an_origin_branch_created_between_the_runs_is_not_deleted() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let (landing, token) = first_run(&fixture, &wt, &["--force-remote"]);

    fixture.others_push("feat/x", Some(&fixture.tip("main")));
    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Removed feat-x ("))
        .stderr(predicate::str::contains("Deleted origin/").not());
    assert_deleted(&fixture, &wt, "feat/x");
    assert!(fixture.origin_has("feat/x"));
}

/// A push that lands while the second run deletes the approved branch on
/// origin (here from a `pre-push` hook, which runs inside the deletion's
/// own push) fails its lease. The checkout is gone by then; the branch on
/// origin is kept with Git's reason (exit 1), and nothing is left to retry.
#[test]
fn a_push_during_the_second_runs_deletion_fails_the_lease_and_keeps_the_origin_branch() {
    let fixture = Fixture::with_origin();
    // The base checkout is on `main`.
    let main = head_after_commit(&fixture.repo());
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fixture.commit(&wt, "only-here.txt");
    fixture.push_upstream("feat/x");
    let (landing, token) = first_run(&fixture, &wt, &["--force-remote"]);

    let hooks = fixture.root.path().join("hooks");
    fs::create_dir(&hooks).unwrap();
    let origin = fixture.origin().to_str().unwrap().replace('\\', "/");
    let hook = hooks.join("pre-push");
    fs::write(
        &hook,
        format!("#!/bin/sh\ngit --git-dir=\"{origin}\" update-ref refs/heads/feat/x {main}\n"),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    }
    // A Windows path's backslashes are escapes in a config file.
    let hooks_path = hooks.to_str().unwrap().replace('\\', "\\\\");
    append_config(&fixture.repo(), &format!("[core]\n\thooksPath = \"{hooks_path}\"\n"));

    let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert!(stderr.contains("origin/feat/x was kept: "), "{stderr}");
    assert!(stderr.contains("Delete it with git push origin --delete feat/x if you no longer need it."), "{stderr}");
    assert!(!stderr.contains("Retry:") && !stderr.contains("report.json"), "{stderr}");
    assert!(output.stdout.is_empty(), "no protocol lines from the second run");
    assert_eq!(pending_removals(&fixture), 0, "nothing is left to retry");
    assert_checkout_gone(&fixture, &wt);
    assert!(fixture.branch_exists("feat/x"), "not safe without origin's copy, so kept");
    assert_eq!(TwoRemotes::head_in(&fixture.origin(), "feat/x"), Some(main));
}

// --- Second runs that need no network -------------------------------------------

const PROXIED_ORIGIN: &str = "http://gitea.test/o/r.git";

/// Stands in for every HTTP host: origin is `http://gitea.test/o/r.git` and
/// the proxy variables point at a local port, so each request the PR lookup
/// or a git transport makes arrives here as one connection. It is closed
/// unanswered (origin is unreachable), except that a PR list request gets
/// [`CountingProxy::answer_pulls`]'s body when one is set.
struct CountingProxy {
    address: std::net::SocketAddr,
    requests: std::sync::mpsc::Receiver<(std::net::SocketAddr, String)>,
    pulls: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl CountingProxy {
    fn start() -> Self {
        use std::io::{Read, Write};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, requests) = std::sync::mpsc::channel();
        let pulls = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
        let answer = std::sync::Arc::clone(&pulls);
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let Ok(peer) = stream.peer_addr() else { continue };
                // Every client here (reqwest, git's curl, the sentinel) sends
                // its request at once; the timeout only bounds a silent one.
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
                let mut head = Vec::new();
                let mut chunk = [0_u8; 4096];
                while !head.windows(4).any(|window| window == b"\r\n\r\n") && head.len() < 65_536 {
                    match stream.read(&mut chunk) {
                        Ok(0) | Err(_) => break,
                        Ok(read) => head.extend_from_slice(&chunk[..read]),
                    }
                }
                let line = String::from_utf8_lossy(&head).lines().next().unwrap_or_default().to_string();
                if is_pr_lookup(&line)
                    && let Some(body) = answer.lock().unwrap().clone()
                {
                    let _ = write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                }
                if sender.send((peer, line)).is_err() {
                    return;
                }
            }
        });
        Self {
            address,
            requests,
            pulls,
        }
    }

    fn apply(&self, cmd: &mut assert_cmd::Command) {
        let url = format!("http://{}", self.address);
        for name in ["HTTP_PROXY", "http_proxy", "HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"] {
            cmd.env(name, &url);
        }
        cmd.env_remove("NO_PROXY").env_remove("no_proxy");
        // A developer's real provider token never reaches the stand-in.
        for name in [
            "GITEA_TOKEN",
            "FORGEJO_TOKEN",
            "CODEBERG_TOKEN",
            "GH_TOKEN",
            "GITHUB_TOKEN",
            "GITLAB_TOKEN",
            "GITLAB_PRIVATE_TOKEN",
            "BITBUCKET_TOKEN",
        ] {
            cmd.env_remove(name);
        }
    }

    /// Serves `body` to every later PR list request; `None` leaves them
    /// unanswered, like every other request.
    fn answer_pulls(&self, body: Option<String>) {
        *self.pulls.lock().unwrap() = body;
    }

    /// The request line of each connection since the previous call. A
    /// sentinel connection is accepted after every earlier one, so none
    /// still in the backlog is missed.
    fn requests(&self) -> Vec<String> {
        use std::io::Write;

        let mut sentinel = std::net::TcpStream::connect(self.address).unwrap();
        sentinel.write_all(b"SENTINEL / HTTP/1.1\r\n\r\n").unwrap();
        let sentinel_address = sentinel.local_addr().unwrap();
        let mut lines = Vec::new();
        loop {
            let (peer, line) = self
                .requests
                .recv_timeout(std::time::Duration::from_secs(30))
                .expect("the proxy accepts the sentinel");
            if peer == sentinel_address {
                return lines;
            }
            lines.push(line);
        }
    }

    /// Connections since the previous call (see [`CountingProxy::requests`]).
    fn connections(&self) -> usize {
        self.requests().len()
    }
}

/// sniff's Gitea PR list (`GET http://gitea.test/api/v1/repos/o/r/pulls?…`);
/// every other request through the proxy is a git transport's.
fn is_pr_lookup(request_line: &str) -> bool {
    request_line.contains("/api/v1/")
}

/// Runs the first run (which asks origin about PRs) and the second run with
/// every HTTP request counted, and returns the second run's stderr after
/// asserting it made none.
fn second_run_without_network(fixture: &Fixture, wt: &Path, extra: &[&str]) -> String {
    add_origin(&fixture.repo(), PROXIED_ORIGIN);
    let proxy = CountingProxy::start();
    let mut first = fixture.wt(wt);
    proxy.apply(&mut first);
    let out = first
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x"])
        .args(extra)
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&out);
    assert!(proxy.connections() > 0, "the first run's PR lookup reaches the proxy");

    let mut second = fixture.wt(&landing);
    proxy.apply(&mut second);
    let output = second.args(["remove", "--handoff", &token]).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert_eq!(proxy.connections(), 0, "the second run made a network request: {stderr}");
    assert!(!wt.exists(), "{stderr}");
    stderr
}

#[test]
fn keeping_the_branch_makes_no_network_request_in_the_second_run() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fixture.commit(&wt, "only-here.txt");

    let stderr = second_run_without_network(&fixture, &wt, &[]);
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert!(!stderr.contains("deleted branch"), "{stderr}");
    assert!(fixture.branch_exists("feat/x"));
}

#[test]
fn an_explicitly_deleted_branch_makes_no_network_request_in_the_second_run() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fixture.commit(&wt, "only-here.txt");

    let stderr = second_run_without_network(&fixture, &wt, &["--force-branch"]);
    assert!(stderr.contains("and deleted branch feat/x."), "{stderr}");
    assert!(!stderr.contains("lost"), "safety was not reassessed: {stderr}");
    assert!(!fixture.branch_exists("feat/x"));
}

mod automatic_deletion_with_local_proof_makes_no_network_request_in_the_second_run {
    use super::*;

    const ROWS: [&str; 3] = ["the local default branch", "another local branch", "a tag"];

    fn check(proof: &str) {
        let fixture = Fixture::new();
        let wt = fixture.add_worktree("feat/x", "feat-x");
        // The worktree starts at main's tip; the other proofs hold a commit
        // of its own.
        if proof != "the local default branch" {
            let tip = fixture.commit(&wt, "only-here.txt");
            let proving_ref = if proof == "a tag" { "refs/tags/v1" } else { "refs/heads/backup" };
            git(&fixture.repo(), &["update-ref", proving_ref, &tip]);
        }

        let stderr = second_run_without_network(&fixture, &wt, &[]);
        assert!(stderr.contains("and deleted branch feat/x."), "{proof}: {stderr}");
        assert!(!fixture.branch_exists("feat/x"), "{proof}");
    }

    #[test]
    fn the_local_default_branch() {
        check(ROWS[0]);
    }

    #[test]
    fn another_local_branch() {
        check(ROWS[1]);
    }

    #[test]
    fn a_tag() {
        check(ROWS[2]);
    }

    /// One test per row of `ROWS` (test-speed.md #2), so a row added
    /// there fails here until it has its own test above.
    #[test]
    fn every_row_has_a_test() {
        assert_eq!(ROWS.len(), 3);
    }
}

/// A merged Gitea PR from `o/r`'s own `feat/x` whose head is `head`, as
/// sniff's PR list reads it.
fn merged_pr_list(head: &str) -> String {
    serde_json::json!([{
        "number": 7,
        "title": "feat/x",
        "state": "closed",
        "user": {"login": "dev"},
        "head": {"ref": "feat/x", "label": "feat/x", "sha": head, "repo": {"full_name": "o/r"}},
        "base": {"ref": "main", "repo": {"full_name": "o/r"}},
        "created_at": "2026-01-01T00:00:00Z",
        "merged": true,
        "merged_at": "2026-01-02T00:00:00Z",
        "html_url": "http://gitea.test/o/r/pulls/7",
    }])
    .to_string()
}

/// Only a merged PR holds the tip, so the second run must ask the provider
/// again: it deletes the branch while the PR still answers, and keeps it
/// (exit 1) once the PR is gone or the provider does not answer. The
/// checkout is deleted either way.
mod automatic_deletion_proved_only_by_a_pr_asks_the_provider_again_in_the_second_run {
    use super::*;

    const ROWS: [(&str, i32); 3] = [("the same PR", 0), ("no PR", 1), ("no answer", 1)];

    fn check((second_answer, expected_code): (&str, i32)) {
        let fixture = Fixture::new();
        add_origin(&fixture.repo(), PROXIED_ORIGIN);
        let wt = fixture.add_worktree("feat/x", "feat-x");
        let tip = fixture.commit(&wt, "only-here.txt");
        let proxy = CountingProxy::start();
        proxy.answer_pulls(Some(merged_pr_list(&tip)));

        let mut first = fixture.wt(&wt);
        proxy.apply(&mut first);
        let out = first
            .env("WT_SHELL_WRAPPER", "1")
            .args(["remove", "feat-x"])
            .assert()
            .code(0)
            .stderr(predicate::str::contains("PR #7"))
            .get_output()
            .stdout
            .clone();
        let (landing, token) = protocol(&out);
        assert!(proxy.requests().iter().any(|line| is_pr_lookup(line)), "{second_answer}");

        proxy.answer_pulls(match second_answer {
            "the same PR" => Some(merged_pr_list(&tip)),
            "no PR" => Some("[]".to_string()),
            _ => None,
        });
        let mut second = fixture.wt(&landing);
        proxy.apply(&mut second);
        let output = second.args(["remove", "--handoff", &token]).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        let requests = proxy.requests();
        assert!(
            requests.iter().any(|line| is_pr_lookup(line)),
            "{second_answer}: the second run asked the provider: {requests:?}"
        );
        assert_eq!(output.status.code(), Some(expected_code), "{second_answer}: {stderr}");
        assert!(stderr.contains("Removed feat-x ("), "{second_answer}: {stderr}");
        if expected_code == 0 {
            assert!(stderr.contains("and deleted branch feat/x."), "{second_answer}: {stderr}");
            assert_deleted(&fixture, &wt, "feat/x");
        } else {
            assert!(stderr.contains("Branch feat/x was kept: it is no longer safe to delete"), "{second_answer}: {stderr}");
            assert_checkout_gone(&fixture, &wt);
            assert_eq!(git(&fixture.repo(), &["rev-parse", "feat/x"]), tip);
        }
    }

    #[test]
    fn the_same_pr() {
        check(ROWS[0]);
    }

    #[test]
    fn no_pr() {
        check(ROWS[1]);
    }

    #[test]
    fn no_answer() {
        check(ROWS[2]);
    }

    /// One test per row of `ROWS` (test-speed.md #2), so a row added
    /// there fails here until it has its own test above.
    #[test]
    fn every_row_has_a_test() {
        assert_eq!(ROWS.len(), 3);
    }
}

/// `feat/x` has a commit of its own that only `origin/backup` holds (not the
/// default branch), and the first run approved automatic deletion on that
/// proof after checking origin live.
fn approved_on_another_origin_branch(fixture: &Fixture) -> (PathBuf, PathBuf, String, String) {
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let tip = fixture.commit(&wt, "only-here.txt");
    fixture.push(&tip, "backup");
    assert_eq!(git(&fixture.repo(), &["rev-parse", "origin/backup"]), tip);
    let out = fixture
        .wt(&wt)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("origin/backup"))
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&out);
    (wt, landing, token, tip)
}

/// The last remote protection is taken away on origin itself; the tracking
/// ref still holds the tip, so only a live check can notice. The checkout
/// is deleted either way; the branch is kept (exit 1).
mod a_branch_proved_only_by_another_origin_branch_is_kept_once_origin_drops_it {
    use super::*;

    const ROWS: [&str; 3] = ["unchanged", "deleted on origin", "moved on origin"];

    fn check(change: &str) {
        let fixture = Fixture::with_origin();
        let old_main = git(&fixture.repo(), &["rev-parse", "main"]);
        let (wt, landing, token, tip) = approved_on_another_origin_branch(&fixture);

        if change != "unchanged" {
            match change {
                "deleted on origin" => fixture.others_push("backup", None),
                _ => fixture.others_push("backup", Some(&old_main)),
            }
            assert_eq!(git(&fixture.repo(), &["rev-parse", "origin/backup"]), tip, "{change}");
        }

        let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Removed feat-x ("), "{change}: {stderr}");
        if change == "unchanged" {
            assert_eq!(output.status.code(), Some(0), "{change}: {stderr}");
            assert!(stderr.contains("and deleted branch feat/x."), "{change}: {stderr}");
            assert_deleted(&fixture, &wt, "feat/x");
        } else {
            assert_eq!(output.status.code(), Some(1), "{change}: {stderr}");
            assert!(stderr.contains("Branch feat/x was kept: it is no longer safe to delete"), "{change}: {stderr}");
            assert_checkout_gone(&fixture, &wt);
            assert_eq!(git(&fixture.repo(), &["rev-parse", "feat/x"]), tip, "{change}");
        }
    }

    #[test]
    fn unchanged() {
        check(ROWS[0]);
    }

    #[test]
    fn deleted_on_origin() {
        check(ROWS[1]);
    }

    #[test]
    fn moved_on_origin() {
        check(ROWS[2]);
    }

    /// One test per row of `ROWS` (test-speed.md #2), so a row added
    /// there fails here until it has its own test above.
    #[test]
    fn every_row_has_a_test() {
        assert_eq!(ROWS.len(), 3);
    }
}

/// The same approval, but origin now answers only through the network,
/// where nothing answers: the second run asks rather than trusting the
/// first run's live check, and an unanswered question keeps the branch
/// after the checkout is deleted (exit 1).
#[test]
fn automatic_deletion_proved_only_by_another_origin_branch_asks_origin_again_in_the_second_run() {
    let fixture = Fixture::with_origin();
    let (wt, landing, token, tip) = approved_on_another_origin_branch(&fixture);
    git(&fixture.repo(), &["remote", "set-url", "origin", PROXIED_ORIGIN]);

    let proxy = CountingProxy::start();
    let mut second = fixture.wt(&landing);
    proxy.apply(&mut second);
    let output = second.args(["remove", "--handoff", &token]).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let requests = proxy.requests();
    assert!(
        requests.iter().any(|line| !is_pr_lookup(line)),
        "the second run checked origin's head live: {requests:?}"
    );
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert!(stderr.contains("Branch feat/x was kept: it is no longer safe to delete"), "{stderr}");
    assert_checkout_gone(&fixture, &wt);
    assert_eq!(git(&fixture.repo(), &["rev-parse", "feat/x"]), tip);
}

/// With `--force-remote` and an origin that does not answer, the first run's
/// preflight binds no branch on origin, so nothing on origin is approved:
/// the second run makes no request at all (no PR lookup, no preflight),
/// whether the branch is kept, explicitly deleted, or proved by a tag, and
/// removes the checkout. The first run already said origin was not deleted.
mod a_force_remote_second_run_after_an_unanswered_preflight_makes_no_request {
    use super::*;

    const ROWS: [(&str, &[&str]); 3] = [("keep", &["--force-remote"]), ("explicit delete", &["--force-remote", "--force-branch"]), ("tag proof", &["--force-remote"])];

    fn check((approval, extra): (&str, &[&str])) {
        let fixture = Fixture::new();
        add_origin(&fixture.repo(), PROXIED_ORIGIN);
        let wt = fixture.add_worktree("feat/x", "feat-x");
        let tip = fixture.commit(&wt, "only-here.txt");
        if approval == "tag proof" {
            git(&fixture.repo(), &["update-ref", "refs/tags/v1", &tip]);
        }
        let proxy = CountingProxy::start();

        let mut first = fixture.wt(&wt);
        proxy.apply(&mut first);
        let output = first.env("WT_SHELL_WRAPPER", "1").args(["remove", "feat-x"]).args(extra).output().unwrap();
        let stderr = flowed(&stderr_of(&output));
        assert_eq!(output.status.code(), Some(0), "{approval}: {stderr}");
        assert!(stderr.contains("could not be reached"), "{approval}: {stderr}");
        assert!(stderr.contains("origin/feat/x was not deleted: "), "{approval}: {stderr}");
        let (landing, token) = protocol(&output.stdout);
        assert!(proxy.connections() > 0, "{approval}: the first run reaches the proxy");

        let mut second = fixture.wt(&landing);
        proxy.apply(&mut second);
        let output = second.args(["remove", "--handoff", &token]).output().unwrap();
        let stderr = stderr_of(&output);
        let requests = proxy.requests();
        assert!(requests.is_empty(), "{approval}: the second run made a request: {requests:?}");
        assert_eq!(output.status.code(), Some(0), "{approval}: {stderr}");
        assert!(stderr.contains("Removed feat-x ("), "{approval}: {stderr}");
        assert_checkout_gone(&fixture, &wt);
        assert_eq!(fixture.branch_exists("feat/x"), approval == "keep", "{approval}: {stderr}");
    }

    #[test]
    fn keep() {
        check(ROWS[0]);
    }

    #[test]
    fn explicit_delete() {
        check(ROWS[1]);
    }

    #[test]
    fn tag_proof() {
        check(ROWS[2]);
    }

    /// One test per row of `ROWS` (test-speed.md #2), so a row added
    /// there fails here until it has its own test above.
    #[test]
    fn every_row_has_a_test() {
        assert_eq!(ROWS.len(), 3);
    }
}

/// With a branch on origin bound by the first run, the second run rechecks
/// the push endpoint from the configuration and deletes under its lease:
/// that push is its only remote command (no preflight `ls-remote`), and a
/// branch proved by a local tag is reconfirmed locally.
#[test]
fn a_force_remote_second_run_runs_no_ls_remote_only_the_deletion_push() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let tip = fixture.commit(&wt, "only-here.txt");
    fixture.push_upstream("feat/x");
    git(&fixture.repo(), &["update-ref", "refs/tags/v1", &tip]);
    let (landing, token) = first_run(&fixture, &wt, &["--force-remote"]);

    let trace = fixture.root.path().join("second-run-trace.log");
    let output = fixture.wt(&landing).env("GIT_TRACE", &trace).args(["remove", "--handoff", &token]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("and deleted branch feat/x."), "{stderr}");
    assert!(stderr.contains("Deleted origin/feat/x"), "{stderr}");
    let log = fs::read_to_string(&trace).unwrap();
    let started: Vec<&str> = log.lines().filter(|line| line.contains("trace: built-in: git ")).collect();
    assert!(!started.iter().any(|line| line.contains("git ls-remote")), "{started:#?}");
    assert_eq!(started.iter().filter(|line| line.contains("git push")).count(), 1, "{started:#?}");
    assert_deleted(&fixture, &wt, "feat/x");
    assert!(!fixture.origin_has("feat/x"));
}

// --- The repository `--force-remote` deletes from -----------------------------

/// `approved.git` and `other.git`, each holding `main` and `feat/x` at the same
/// commit, with `origin` pointing at `approved.git`; `feat/x` is also a
/// worktree.
struct TwoRemotes {
    fixture: Fixture,
    wt: PathBuf,
    approved: PathBuf,
    other: PathBuf,
}

impl TwoRemotes {
    fn new() -> Self {
        let fixture = Fixture::new();
        let approved = fixture.root.path().join("approved.git");
        let other = fixture.root.path().join("other.git");
        let origin = Self::url(&approved).to_string();
        Self::build(fixture, approved, other, &origin)
    }

    /// The bare repositories are `approved` and `other` inside the base
    /// checkout, and origin's URL is the relative path `approved`.
    fn relative() -> Self {
        let fixture = Fixture::new();
        let approved = fixture.repo().join("approved");
        let other = fixture.repo().join("other");
        let this = Self::build(fixture, approved, other, "approved");
        assert_eq!(git(&this.fixture.repo(), &["remote", "get-url", "--push", "origin"]), "approved");
        this
    }

    fn build(fixture: Fixture, approved: PathBuf, other: PathBuf, origin: &str) -> Self {
        let wt = fixture.add_worktree("feat/x", "feat-x");
        // Each holds `main` and `feat/x`, as a bare clone of the repository
        // would: the template's clone holds `main`, and `feat/x` is `main`.
        for bare in [&approved, &other] {
            copy_tree(&template().join("origin.git"), bare);
            write_ref(bare, "refs/heads/feat/x", &fixture.tip("main"));
        }
        add_origin(&fixture.repo(), origin);
        Self {
            fixture,
            wt,
            approved,
            other,
        }
    }

    fn url(path: &Path) -> &str {
        path.to_str().unwrap()
    }

    fn head_in(bare: &Path, branch: &str) -> Option<String> {
        let out = git(bare, &["for-each-ref", "--format=%(objectname)", &format!("refs/heads/{branch}")]);
        (!out.is_empty()).then_some(out)
    }

    /// Neither repository lost its `feat/x`.
    fn assert_remotes_untouched(&self) {
        assert!(Self::head_in(&self.approved, "feat/x").is_some(), "approved.git keeps feat/x");
        assert!(Self::head_in(&self.other, "feat/x").is_some(), "other.git keeps feat/x");
    }

    /// Nothing was removed anywhere.
    fn assert_untouched(&self) {
        self.assert_remotes_untouched();
        assert!(self.wt.exists(), "the worktree stays");
        assert!(self.fixture.branch_exists("feat/x"), "the local branch stays");
    }

    /// The first run approves deleting `origin/feat/x`, `change` runs, and
    /// the second run finds that origin no longer pushes where it was
    /// approved: the checkout and the (Safe) local branch go, the branch on
    /// origin is kept in both repositories (exit 1). Returns the second
    /// run's stderr, flowed.
    fn handoff_keeps_origin_after(&self, change: impl FnOnce(&Self)) -> String {
        let (landing, token) = first_run(&self.fixture, &self.wt, &["--force-remote"]);
        change(self);
        let output = self.fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
        let stderr = flowed(&stderr_of(&output));
        assert_eq!(output.status.code(), Some(1), "{stderr}");
        assert!(stderr.contains("Removed feat-x ("), "{stderr}");
        assert!(stderr.contains("origin/feat/x was kept: "), "{stderr}");
        assert!(stderr.contains("Delete it with git push origin --delete feat/x"), "{stderr}");
        assert!(!stderr.contains("Deleted origin/"), "{stderr}");
        assert_deleted(&self.fixture, &self.wt, "feat/x");
        self.assert_remotes_untouched();
        stderr
    }
}

#[test]
fn a_changed_origin_url_between_the_runs_keeps_the_origin_branch_after_the_checkout_is_deleted() {
    let remotes = TwoRemotes::new();
    let stderr = remotes.handoff_keeps_origin_after(|remotes| {
        git(&remotes.fixture.repo(), &["remote", "set-url", "origin", TwoRemotes::url(&remotes.other)]);
    });
    assert!(stderr.contains("origin no longer pushes to "), "{stderr}");
}

#[test]
fn a_changed_origin_push_url_between_the_runs_keeps_the_origin_branch_after_the_checkout_is_deleted() {
    let remotes = TwoRemotes::new();
    let stderr = remotes.handoff_keeps_origin_after(|remotes| {
        git(&remotes.fixture.repo(), &["remote", "set-url", "--push", "origin", TwoRemotes::url(&remotes.other)]);
    });
    assert!(stderr.contains("origin no longer pushes to "), "{stderr}");
}

/// Fetch and push URLs differ, and `approved.git`'s `feat/x` sits at another
/// commit: observing the fetch URL would take the lease against the wrong
/// head (or miss the branch), so a deletion that succeeds proves the report,
/// the lease, and the push all used the push URL.
#[test]
fn a_separate_push_url_is_both_observed_and_deleted_from() {
    let remotes = TwoRemotes::new();
    let repo = remotes.fixture.repo();
    // A commit on a local branch `side` only, made without a checkout.
    let side = git(&repo, &["commit-tree", "-p", "main", "-m", "side", "main^{tree}"]);
    write_ref(&repo.join(".git"), "refs/heads/side", &side);
    remotes.fixture.publish(&remotes.approved, &side, "feat/x");
    // What `git remote set-url --push origin <other>` writes.
    let other = TwoRemotes::url(&remotes.other).replace('\\', "\\\\");
    append_config(&repo, &format!("[remote \"origin\"]\n\tpushurl = \"{other}\"\n"));

    let out = remotes
        .fixture
        .wt(&remotes.wt)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("other.git"))
        .stderr(predicate::str::contains("origin/feat/x will be deleted; it has no commits"))
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&out);

    remotes
        .fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Deleted origin/feat/x"));
    assert_eq!(TwoRemotes::head_in(&remotes.other, "feat/x"), None);
    assert_eq!(TwoRemotes::head_in(&remotes.approved, "feat/x"), Some(side));
    assert!(!remotes.wt.exists());
}

/// Deleting from several repositories would act on copies the report never
/// showed, so the run refuses before removing anything (exit 3, since
/// leaving out `--force-remote` is the remedy).
#[test]
fn several_push_urls_refuse_force_remote_with_nothing_removed() {
    let remotes = TwoRemotes::new();
    let repo = remotes.fixture.repo();
    for bare in [&remotes.approved, &remotes.other] {
        git(&repo, &["remote", "set-url", "--add", "--push", "origin", TwoRemotes::url(bare)]);
    }

    remotes
        .fixture
        .wt(&repo)
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(3)
        .stdout("")
        .stderr(predicate::str::contains("pushes to 2 repositories"))
        .stderr(predicate::str::contains("Nothing was removed"));
    remotes.assert_untouched();
}

/// [`TwoRemotes`] plus `fetch.git` (same branches) as origin's only URL, and
/// `url.<approved>.pushInsteadOf=<fetch>`, so origin's resolved push URL is
/// `approved.git`.
fn rewritten_to_approved() -> (TwoRemotes, String) {
    let remotes = TwoRemotes::new();
    let repo = remotes.fixture.repo();
    let fetch = remotes.fixture.root.path().join("fetch.git");
    git(remotes.fixture.root.path(), &["init", "-q", "--bare", "-b", "main", TwoRemotes::url(&fetch)]);
    git(&repo, &["push", "-q", TwoRemotes::url(&fetch), "main", "feat/x"]);
    git(&repo, &["remote", "set-url", "origin", TwoRemotes::url(&fetch)]);
    let approved_rule = format!("url.{}.pushInsteadOf", TwoRemotes::url(&remotes.approved));
    git(&repo, &["config", &approved_rule, TwoRemotes::url(&fetch)]);
    assert_eq!(
        git(&repo, &["remote", "get-url", "--push", "origin"]),
        TwoRemotes::url(&remotes.approved)
    );
    let redirect = format!("url.{}.pushInsteadOf", TwoRemotes::url(&remotes.other));
    (remotes, redirect)
}

fn refs_of(bare: &Path) -> String {
    git(bare, &["for-each-ref", "--format=%(objectname) %(refname)"])
}

/// A second rule maps the resolved push URL (`approved.git`) on to
/// `other.git`; `git push <approved>` would delete from `other.git`.
#[test]
fn a_rewrite_rule_matching_the_push_url_refuses_force_remote_with_nothing_removed() {
    let (remotes, redirect) = rewritten_to_approved();
    let repo = remotes.fixture.repo();
    git(&repo, &["config", &redirect, TwoRemotes::url(&remotes.approved)]);
    let other_before = refs_of(&remotes.other);

    remotes
        .fixture
        .wt(&repo)
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(3)
        .stdout("")
        .stderr(predicate::str::contains("Nothing was removed"))
        .stderr(predicate::str::contains("would rewrite origin's push URL"))
        .stderr(predicate::str::contains("pushinsteadof="))
        .stderr(predicate::str::contains("Deleted").not());
    remotes.assert_untouched();
    assert_eq!(refs_of(&remotes.other), other_before, "other.git is never modified");
}

/// The rule appears between the runs while `git remote get-url --push`
/// still prints the approved URL, so only the rewrite check can see it.
#[test]
fn a_rewrite_rule_added_between_the_runs_keeps_the_origin_branch_after_the_checkout_is_deleted() {
    let (remotes, redirect) = rewritten_to_approved();
    let other_before = refs_of(&remotes.other);
    remotes.handoff_keeps_origin_after(|remotes| {
        let repo = remotes.fixture.repo();
        git(&repo, &["config", &redirect, TwoRemotes::url(&remotes.approved)]);
        assert_eq!(git(&repo, &["remote", "get-url", "--push", "origin"]), TwoRemotes::url(&remotes.approved));
    });
    assert_eq!(refs_of(&remotes.other), other_before, "other.git is never modified");
}

/// Remote `approved` fetches from `approved` but pushes to `other`, so
/// `ls-remote approved` and `push approved` would each read it as that remote
/// rather than as origin's relative path.
fn name_the_endpoint_as_a_remote(remotes: &TwoRemotes) {
    let repo = remotes.fixture.repo();
    git(&repo, &["remote", "add", "approved", TwoRemotes::url(&remotes.approved)]);
    git(&repo, &["remote", "set-url", "--push", "approved", TwoRemotes::url(&remotes.other)]);
    assert_eq!(git(&repo, &["remote", "get-url", "--push", "origin"]), "approved");
}

#[test]
fn an_endpoint_that_names_another_remote_refuses_force_remote_with_nothing_removed() {
    let remotes = TwoRemotes::relative();
    name_the_endpoint_as_a_remote(&remotes);
    let other_before = refs_of(&remotes.other);

    remotes
        .fixture
        .wt(&remotes.fixture.repo())
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(3)
        .stdout("")
        .stderr(predicate::str::contains("Nothing was removed"))
        .stderr(predicate::str::contains("as the name of the remote defined by remote.approved."))
        .stderr(predicate::str::contains("Deleted").not());
    remotes.assert_untouched();
    assert_eq!(refs_of(&remotes.other), other_before, "other is never modified");
}

/// `git remote get-url approved` reports no such remote for a remote given
/// only by `-c`, yet `git push approved` uses it.
#[test]
fn an_endpoint_naming_a_remote_from_command_line_config_refuses_with_nothing_removed() {
    let remotes = TwoRemotes::relative();
    let other_before = refs_of(&remotes.other);

    remotes
        .fixture
        .wt(&remotes.fixture.repo())
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "remote.approved.pushurl")
        .env("GIT_CONFIG_VALUE_0", TwoRemotes::url(&remotes.other))
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(3)
        .stdout("")
        .stderr(predicate::str::contains("remote defined by remote.approved.pushurl"))
        .stderr(predicate::str::contains("Deleted").not());
    remotes.assert_untouched();
    assert_eq!(refs_of(&remotes.other), other_before, "other is never modified");
}

/// The remote appears between the runs while origin's push URL still prints
/// `approved`, so only the endpoint recheck can see it.
#[test]
fn a_remote_named_like_the_endpoint_added_between_the_runs_keeps_the_origin_branch_after_the_checkout_is_deleted() {
    let remotes = TwoRemotes::relative();
    let other_before = refs_of(&remotes.other);
    remotes.handoff_keeps_origin_after(name_the_endpoint_as_a_remote);
    assert_eq!(refs_of(&remotes.other), other_before, "other is never modified");
}

#[test]
fn an_unambiguous_relative_endpoint_is_observed_and_deleted_from() {
    let remotes = TwoRemotes::relative();
    let other_before = refs_of(&remotes.other);

    remotes
        .fixture
        .wt(&remotes.fixture.repo())
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Origin (approved)"))
        .stderr(predicate::str::contains("Deleted origin/feat/x"));
    assert_eq!(TwoRemotes::head_in(&remotes.approved, "feat/x"), None);
    assert_eq!(refs_of(&remotes.other), other_before, "other is never modified");
    assert!(!remotes.wt.exists());
}

// --- Worktrees Git can no longer read -----------------------------------------

fn porcelain(fixture: &Fixture) -> String {
    git(&fixture.repo(), &["worktree", "list", "--porcelain"])
}

/// The porcelain block for the worktree at `path`, if Git still lists it.
fn listed_block(fixture: &Fixture, path: &Path) -> Option<String> {
    let wanted = canonical(path.parent().unwrap()).join(path.file_name().unwrap());
    porcelain(fixture).split("\n\n").find(|block| {
        block.lines().next().and_then(|line| line.strip_prefix("worktree ")).is_some_and(|listed| {
            let listed = Path::new(listed);
            canonical(listed.parent().unwrap()).join(listed.file_name().unwrap()) == wanted
        })
    }).map(str::to_string)
}

fn stderr_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A worktree inside the base checkout. Only there can a caller standing in
/// it with its `.git` gone still reach the repository: Git discovers the
/// enclosing base checkout.
fn add_nested_worktree(fixture: &Fixture, branch: &str, dir_name: &str) -> PathBuf {
    let path = fixture.repo().join(".nested").join(dir_name);
    git(&fixture.repo(), &["worktree", "add", "-q", "-b", branch, path.to_str().unwrap(), "main"]);
    path
}

#[test]
fn a_missing_directory_has_only_its_record_removed_and_its_safe_branch_deleted() {
    let fixture = Fixture::new();
    let gone = fixture.add_worktree("feat/gone", "feat-gone");
    let other = fixture.add_worktree("feat/other", "feat-other");
    fs::remove_dir_all(&gone).unwrap();
    assert!(listed_block(&fixture, &gone).unwrap().contains("prunable"));

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-gone"]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(
        stderr.contains("Removed the record of worktree feat-gone (its directory was already gone) and deleted branch feat/gone."),
        "{stderr}"
    );
    // Nothing to warn about: there were no files to check.
    assert!(!stderr.contains('⚠'), "{stderr}");
    assert!(listed_block(&fixture, &gone).is_none(), "{}", porcelain(&fixture));
    assert!(!fixture.branch_exists("feat/gone"));
    assert!(!gone.exists(), "nothing is recreated");
    assert!(listed_block(&fixture, &other).is_some() && other.join("README.md").exists());
    assert!(fixture.branch_exists("feat/other"));
}

#[test]
fn staged_work_left_in_a_missing_directory_record_needs_consent() {
    let fixture = Fixture::new();
    let gone = fixture.add_worktree("feat/gone", "feat-gone");
    fs::write(gone.join("staged.txt"), "staged\n").unwrap();
    git(&gone, &["add", "staged.txt"]);
    fs::remove_dir_all(&gone).unwrap();

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-gone"]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(
        stderr.contains("The worktree's directory is gone; 1 staged change in Git's record will be lost: staged.txt"),
        "{stderr}"
    );
    assert!(stderr.contains("Add --force to delete it anyway."), "{stderr}");
    assert!(listed_block(&fixture, &gone).is_some(), "the record stays");
    assert!(fixture.branch_exists("feat/gone"));

    fixture.wt(&fixture.repo()).args(["remove", "feat-gone", "--force"]).assert().code(0);
    assert!(listed_block(&fixture, &gone).is_none());
}

#[test]
fn a_missing_detached_directory_has_its_record_removed() {
    let fixture = Fixture::new();
    let gone = fixture.root.path().join("wts").join("detached-gone");
    git(&fixture.repo(), &["worktree", "add", "-q", "--detach", gone.to_str().unwrap(), "main"]);
    fs::remove_dir_all(&gone).unwrap();

    fixture
        .wt(&fixture.repo())
        .args(["remove", "detached-gone"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Detached HEAD"))
        .stderr(predicate::str::contains("Removed the record of worktree detached-gone"));
    assert!(listed_block(&fixture, &gone).is_none());
}

#[test]
fn a_missing_directory_keeps_an_unsafe_branch_without_a_terminal() {
    let fixture = Fixture::new();
    let gone = fixture.add_worktree("feat/gone", "feat-gone");
    fixture.commit(&gone, "unique.txt");
    fs::remove_dir_all(&gone).unwrap();

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-gone"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Removed the record"))
        .stderr(predicate::str::contains("kept branch"));
    assert!(listed_block(&fixture, &gone).is_none());
    assert!(fixture.branch_exists("feat/gone"), "its commits exist nowhere else");
}

/// R12: with the directory gone and its index missing, staged work cannot
/// be ruled out. That is a warning; `--force` removes the record.
#[test]
fn a_missing_directory_whose_index_is_gone_is_a_warning_and_force_removes_the_record() {
    let fixture = Fixture::new();
    let gone = fixture.add_worktree("feat/gone", "feat-gone");
    let admin = PathBuf::from(git(&gone, &["rev-parse", "--path-format=absolute", "--git-dir"]));
    fs::remove_dir_all(&gone).unwrap();
    fs::remove_file(admin.join("index")).unwrap();

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-gone"]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("⚠ The worktree's directory is gone, and its staged changes cannot be checked ("), "{stderr}");
    assert!(stderr.contains("Git's record will be removed."), "{stderr}");
    assert!(stderr.contains("Add --force to delete it anyway."), "{stderr}");
    assert!(!stderr.contains("prune"), "{stderr}");
    assert!(listed_block(&fixture, &gone).is_some());
    assert!(fixture.branch_exists("feat/gone"));

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-gone", "--force"]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Removed the record of worktree feat-gone (its directory was already gone)"), "{stderr}");
    assert!(listed_block(&fixture, &gone).is_none());
    assert!(!fixture.branch_exists("feat/gone"), "Safe on main");
}

/// The defect's original case, shaped like the observed one: detached, its
/// `.git` file gone, a working file kept.
#[test]
fn an_unlinked_worktree_is_repaired_then_its_files_are_protected() {
    let fixture = Fixture::new();
    let wt = fixture.root.path().join("wts").join("lhg-before");
    git(&fixture.repo(), &["worktree", "add", "-q", "--detach", wt.to_str().unwrap(), "main"]);
    fs::write(wt.join("README.md"), "edited, never committed\n").unwrap();
    fs::remove_file(wt.join(".git")).unwrap();

    let output = fixture.wt(&fixture.repo()).args(["remove", "lhg-before"]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("Its .git file was missing; wt restored it to check the files"), "{stderr}");
    assert!(stderr.contains("⚠ 1 uncommitted file will be lost: README.md"), "{stderr}");
    assert!(stderr.contains("Nothing was removed"), "{stderr}");
    assert!(!stderr.contains("not a git repository"), "{stderr}");
    assert_eq!(fs::read_to_string(wt.join("README.md")).unwrap(), "edited, never committed\n");
    assert!(wt.join(".git").is_file(), "the repaired link is not rolled back");
    let block = listed_block(&fixture, &wt).expect("still registered");
    assert!(!block.contains("prunable"), "{block}");

    fixture.wt(&fixture.repo()).args(["remove", "lhg-before", "--force"]).assert().code(0);
    assert!(!wt.exists());
    assert!(listed_block(&fixture, &wt).is_none());
}

#[test]
fn a_clean_unlinked_worktree_is_repaired_and_removed_with_its_safe_branch() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fs::remove_file(wt.join(".git")).unwrap();

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Its .git file was missing"), "{stderr}");
    assert!(!stderr.contains('⚠'), "a repaired clean checkout warns about nothing: {stderr}");
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert!(stderr.contains("and deleted branch feat/x."), "{stderr}");
    assert_deleted(&fixture, &wt, "feat/x");
}

/// A directory Git can't write into: `git worktree repair` cannot recreate
/// `.git`, so Git cannot read the worktree (R10). That is a warning with
/// Git's reason; `--force` deletes the checkout and Git's record.
#[cfg(unix)]
#[test]
fn a_repair_that_is_not_verified_is_a_warning_and_force_deletes_it() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fs::write(wt.join("notes.txt"), "keep me\n").unwrap();
    fs::remove_file(wt.join(".git")).unwrap();
    fs::set_permissions(&wt, fs::Permissions::from_mode(0o555)).unwrap();
    struct Writable<'a>(&'a Path);
    impl Drop for Writable<'_> {
        fn drop(&mut self) {
            let _ = fs::set_permissions(self.0, fs::Permissions::from_mode(0o755));
        }
    }
    let _restore = Writable(&wt);
    if fs::write(wt.join("probe"), "").is_ok() {
        eprintln!("skipped: this user can write into a read-only directory");
        return;
    }

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("⚠ Git cannot read this worktree: Git couldn't restore a verified link"), "{stderr}");
    assert!(stderr.contains("Its uncommitted files cannot be listed, and everything in it will be deleted."), "{stderr}");
    assert!(!stderr.contains("wt restored it to check"), "never claimed: {stderr}");
    assert!(!stderr.contains("prune"), "{stderr}");
    assert_eq!(fs::read_to_string(wt.join("notes.txt")).unwrap(), "keep me\n");
    assert!(listed_block(&fixture, &wt).is_some(), "the record stays");
    assert!(fixture.branch_exists("feat/x"));

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x", "--force"]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert_deleted(&fixture, &wt, "feat/x");
}

/// R11: a file where the worktree's directory should be is a warning saying
/// what is there; `--force` deletes it and Git's record.
#[test]
fn a_worktree_replaced_by_a_file_is_a_warning_and_force_deletes_it_and_its_record() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fs::remove_dir_all(&wt).unwrap();
    fs::write(&wt, "not a checkout\n").unwrap();

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("⚠ A file is where the worktree's directory should be ("), "{stderr}");
    assert!(stderr.contains("It will be deleted, with Git's record."), "{stderr}");
    assert_eq!(fs::read_to_string(&wt).unwrap(), "not a checkout\n");
    assert!(listed_block(&fixture, &wt).is_some());
    assert!(fixture.branch_exists("feat/x"));

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x", "--force"]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert_deleted(&fixture, &wt, "feat/x");
}

/// R9: a link to somewhere that is not this worktree's checkout is never
/// followed. The warning says its target is not touched; `--force` deletes
/// only the link and Git's record.
#[cfg(unix)]
#[test]
fn a_worktree_replaced_by_a_foreign_link_deletes_only_the_link_and_its_record() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let elsewhere = fixture.root.path().join("elsewhere");
    fs::create_dir(&elsewhere).unwrap();
    fs::write(elsewhere.join("precious.txt"), "precious\n").unwrap();
    fs::remove_dir_all(&wt).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &wt).unwrap();

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("⚠ The path is a link to "), "{stderr}");
    assert!(stderr.contains("which is not touched; only the link and Git's record are deleted."), "{stderr}");
    assert!(fs::symlink_metadata(&wt).unwrap().file_type().is_symlink());
    assert!(listed_block(&fixture, &wt).is_some());

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x", "--force"]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert_deleted(&fixture, &wt, "feat/x");
    assert_eq!(fs::read_to_string(elsewhere.join("precious.txt")).unwrap(), "precious\n");
    assert!(!elsewhere.join(".git").exists(), "nothing was repaired through the link");
}

/// Not prunable, so nothing is prepared, but `git status` fails (R13): the
/// warning gives Git's reason, nothing is removed without `--force`, and
/// `--force` deletes the checkout.
#[test]
fn a_status_failure_is_a_warning_with_gits_reason_and_force_deletes_it() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fs::write(wt.join(".git"), "garbage\n").unwrap();
    assert!(!listed_block(&fixture, &wt).unwrap().contains("prunable"));

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("⚠ Uncommitted files could not be checked ("), "{stderr}");
    assert!(stderr.contains("fatal:"), "Git's own reason is kept: {stderr}");
    assert!(stderr.contains("whatever is there will be deleted."), "{stderr}");
    assert!(!stderr.contains("Removed"), "{stderr}");
    assert_eq!(fs::read_to_string(wt.join(".git")).unwrap(), "garbage\n", "nothing was repaired");
    assert!(wt.join("README.md").exists());
    assert!(fixture.branch_exists("feat/x"));

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x", "--force"]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert_deleted(&fixture, &wt, "feat/x");
}

/// R3 for a worktree whose directory is gone: Git never marks a locked
/// record `prunable`, and the lock is the warning. `--force` unlocks it and
/// removes the record.
#[test]
fn a_locked_worktree_whose_directory_is_gone_is_unlocked_and_its_record_removed_with_force() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    git(&fixture.repo(), &["worktree", "lock", wt.to_str().unwrap()]);
    fs::remove_dir_all(&wt).unwrap();

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(
        stderr.contains("⚠ This worktree is locked (git worktree lock), with no reason given. Deleting it unlocks it."),
        "{stderr}"
    );
    assert!(listed_block(&fixture, &wt).unwrap().contains("locked"));
    assert!(fixture.branch_exists("feat/x"));

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x", "--force"]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Removed the record of worktree feat-x (its directory was already gone)"), "{stderr}");
    assert!(listed_block(&fixture, &wt).is_none(), "{}", porcelain(&fixture));
    assert!(!fixture.branch_exists("feat/x"), "Safe on main");
}

#[test]
fn a_repaired_worktree_hands_off_and_finishes_with_the_ordinary_checks() {
    let fixture = Fixture::new();
    let wt = add_nested_worktree(&fixture, "feat/x", "feat-x");
    fs::remove_file(wt.join(".git")).unwrap();

    let first = fixture
        .wt(&wt)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Its .git file was missing"))
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&first);
    assert!(wt.join(".git").is_file());

    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Removed feat-x ("));
    assert_deleted(&fixture, &wt, "feat/x");
}

#[test]
fn a_link_broken_between_the_runs_refuses_without_another_repair() {
    let fixture = Fixture::new();
    let wt = add_nested_worktree(&fixture, "feat/x", "feat-x");
    fs::remove_file(wt.join(".git")).unwrap();
    let (landing, token) = first_run(&fixture, &wt, &[]);

    fs::remove_file(wt.join(".git")).unwrap();
    let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("Nothing was removed: the worktree can't be inspected: "), "{stderr}");
    assert!(!wt.join(".git").exists(), "the second run never repairs");
    assert!(wt.join("README.md").exists());
    assert!(fixture.branch_exists("feat/x"));
}

mod a_link_redirected_between_the_runs_refuses_with_nothing_removed {
    use super::*;

    const ROWS: [bool; 2] = [true, false];

    fn check(repaired_first: bool) {
        let fixture = Fixture::new();
        let wt = add_nested_worktree(&fixture, "feat/x", "feat-x");
        let other = fixture.add_worktree("feat/y", "feat-y");
        if repaired_first {
            fs::remove_file(wt.join(".git")).unwrap();
        }
        let (landing, token) = first_run(&fixture, &wt, &[]);

        let other_record = git(&other, &["rev-parse", "--path-format=absolute", "--git-dir"]);
        fs::write(wt.join(".git"), format!("gitdir: {other_record}\n")).unwrap();
        let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
        let stderr = stderr_of(&output);
        assert_eq!(output.status.code(), Some(3), "repaired first: {repaired_first}: {stderr}");
        assert!(stderr.contains("Nothing was removed: the worktree changed after it was checked: its .git file names "), "{stderr}");
        assert!(wt.join("README.md").exists());
        assert!(other.join("README.md").exists());
        assert!(fixture.branch_exists("feat/x") && fixture.branch_exists("feat/y"));
    }

    #[test]
    fn repaired_first() {
        check(ROWS[0]);
    }

    #[test]
    fn healthy_first() {
        check(ROWS[1]);
    }

    /// One test per row of `ROWS` (test-speed.md #2), so a row added
    /// there fails here until it has its own test above.
    #[test]
    fn every_row_has_a_test() {
        assert_eq!(ROWS.len(), 2);
    }
}

// --- A checkout replaced by a readable link ----------------------------------
//
// Git reads the checkout through such a link and does not mark it `prunable`,
// yet `git worktree remove` would delete the files it leads to and then fail
// on the link.

/// Moves the directory at `path` to its sibling `saved` and puts a directory
/// link to it at `path`: a symbolic link, or on Windows a junction, which
/// needs no privilege. Returns the moved directory.
fn replace_with_link(path: &Path) -> PathBuf {
    let saved = path.with_file_name("saved");
    fs::rename(path, &saved).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&saved, path).unwrap();
    #[cfg(windows)]
    {
        let status = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(path)
            .arg(&saved)
            .stdout(std::process::Stdio::null())
            .status()
            .expect("cmd runs");
        assert!(status.success(), "mklink /J needs no privilege");
    }
    saved
}

fn is_link(path: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}

/// The files, the link, the record, and the branch all survive.
fn assert_link_refusal_kept_everything(fixture: &Fixture, wt: &Path, saved: &Path, output: &std::process::Output) {
    let stderr = stderr_of(output);
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(!stderr.contains("Removed"), "{stderr}");
    assert_eq!(fs::read_to_string(saved.join("a")).unwrap(), "a\n", "committed file kept: {stderr}");
    assert!(is_link(wt), "the link is left alone");
    assert!(listed_block(fixture, wt).is_some(), "the record stays:\n{}", porcelain(fixture));
    assert!(fixture.branch_exists("feat/x"));
}

/// `feat-x` on `feat/x` with a committed `a`, then moved to `saved` and
/// replaced by a link to it. Returns the worktree path and `saved`.
fn replaced_by_a_readable_link(fixture: &Fixture, worktree: impl Fn(&Fixture) -> PathBuf) -> (PathBuf, PathBuf) {
    let wt = worktree(fixture);
    fixture.commit(&wt, "a");
    let saved = replace_with_link(&wt);
    let block = listed_block(fixture, &wt).expect("still registered");
    assert!(!block.contains("prunable"), "Git reads through the link:\n{block}");
    (wt, saved)
}

/// R9: the path is a link to this worktree's own checkout (its `.git` leads
/// back to this worktree's record). The warning says so, and `--force`
/// deletes both the link and the checkout it leads to.
#[test]
fn a_checkout_replaced_by_a_link_to_itself_is_a_warning_and_force_deletes_both() {
    let fixture = Fixture::new();
    let (wt, saved) = replaced_by_a_readable_link(&fixture, |fixture| fixture.add_worktree("feat/x", "feat-x"));

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert!(stderr.contains("⚠ The path is a link to the worktree's real location at "), "{stderr}");
    assert!(stderr.contains("; both will be deleted."), "{stderr}");
    assert!(!stderr.contains("Git cannot read"), "Git can read it; never claimed: {stderr}");
    assert_link_refusal_kept_everything(&fixture, &wt, &saved, &output);

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x", "--force", "--force-branch"]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Removed feat-x (") && stderr.contains("and deleted branch feat/x (1 commit lost)."), "{stderr}");
    assert_deleted(&fixture, &wt, "feat/x");
    assert!(fs::symlink_metadata(&saved).is_err(), "the checkout the link led to is deleted too");
}

/// A wrapper caller standing in the checkout through a link to it (R9's own
/// link) is inside the worktree. A link saves no record to hand off, so the
/// run deletes both, then prints `cd:` for the wrapper to move the shell.
#[test]
fn a_wrapper_caller_inside_a_checkout_reached_through_its_link_is_moved_out_after_the_removal() {
    let fixture = Fixture::new();
    let (wt, saved) = replaced_by_a_readable_link(&fixture, |fixture| add_nested_worktree(fixture, "feat/x", "feat-x"));

    let output = fixture.wt(&wt).env("WT_SHELL_WRAPPER", "1").args(["remove", "feat-x", "--force"]).output().unwrap();

    // A link has no record to hand off: both are deleted (R9), then the
    // wrapper is told where to move the shell.
    let stderr = stderr_of(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stdout.starts_with("cd:") && !stdout.contains("remove-handoff:"), "stdout {stdout:?}, stderr {stderr}");
    assert!(!saved.exists() && !is_link(&wt), "both the link and its checkout are deleted: {stderr}");
    assert!(listed_block(&fixture, &wt).is_none());
}

#[test]
fn a_checkout_replaced_by_a_link_between_the_runs_refuses_the_approval() {
    let fixture = Fixture::new();
    let wt = add_nested_worktree(&fixture, "feat/x", "feat-x");
    fixture.commit(&wt, "a");
    let (landing, token) = first_run(&fixture, &wt, &["--force", "--force-branch"]);

    let saved = replace_with_link(&wt);
    let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();

    let stderr = flowed(&stderr_of(&output));
    assert!(stderr.contains("Nothing was removed: "), "{stderr}");
    assert!(stderr.contains("something other than the worktree is at"), "{stderr}");
    assert!(stderr.contains("which no step of its removal left, so everything was kept."), "{stderr}");
    assert_link_refusal_kept_everything(&fixture, &wt, &saved, &output);
}

/// A `git` on `PATH` that delegates every call, then, after a successful-or-
/// not `worktree repair`, moves the repaired checkout to `saved` and puts a
/// link to it in its place: every Git-side check reads through the link.
#[cfg(unix)]
#[test]
fn a_checkout_replaced_by_a_link_after_repair_refuses() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fixture.commit(&wt, "a");
    fs::remove_file(wt.join(".git")).unwrap();
    let saved = wt.with_file_name("saved");
    let real_git = String::from_utf8(Command::new("sh").args(["-c", "command -v git"]).output().unwrap().stdout).unwrap();
    let bin = fixture.root.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let shim = bin.join("git");
    fs::write(
        &shim,
        format!(
            "#!/bin/sh\n'{}' \"$@\"\nstatus=$?\ncase \"$*\" in *'worktree repair'*) mv '{wt}' '{saved}' && ln -s '{saved}' '{wt}' ;; esac\nexit $status\n",
            real_git.trim(),
            wt = wt.display(),
            saved = saved.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default());

    let output = fixture
        .wt(&fixture.repo())
        .env("PATH", path)
        .args(["remove", "feat-x", "--force", "--force-branch"])
        .output()
        .unwrap();

    let stderr = stderr_of(&output);
    assert!(stderr.contains("Git couldn't restore a verified link"), "{stderr}");
    assert!(stderr.contains("is now a link"), "{stderr}");
    assert!(!stderr.contains("wt restored it to check"), "never claimed: {stderr}");
    assert_link_refusal_kept_everything(&fixture, &wt, &saved, &output);
}

/// The guard inspects only the target itself: a worktree reached through a
/// linked ancestor directory is removed as usual.
#[cfg(unix)]
#[test]
fn a_worktree_under_a_linked_ancestor_still_removes() {
    let fixture = Fixture::new();
    let real_parent = fixture.root.path().join("real-wts");
    fs::create_dir(&real_parent).unwrap();
    let alias = fixture.root.path().join("alias-wts");
    std::os::unix::fs::symlink(&real_parent, &alias).unwrap();
    let wt = alias.join("feat-x");
    git(&fixture.repo(), &["worktree", "add", "-q", "-b", "feat/x", wt.to_str().unwrap(), "main"]);

    fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).assert().code(0).stderr(predicate::str::contains("Removed feat-x ("));
    assert!(!real_parent.join("feat-x").exists());
    assert!(!fixture.branch_exists("feat/x"));
}

/// A `git` first on `PATH` that delegates every call to the real Git, then
/// runs `after` (a shell snippet, which can call the real Git as `"$real"`)
/// once, after the first call whose arguments contain `trigger`. Delegated
/// output (buffered in a file, so `-z` NULs survive) and exit status pass
/// through.
#[cfg(unix)]
fn one_shot_git_shim(fixture: &Fixture, trigger: &str, after: &str) -> String {
    use std::os::unix::fs::PermissionsExt;

    let real_git = String::from_utf8(Command::new("sh").args(["-c", "command -v git"]).output().unwrap().stdout).unwrap();
    let bin = fixture.root.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let fired = bin.join("fired");
    let shim = bin.join("git");
    fs::write(
        &shim,
        format!(
            "#!/bin/sh\nreal='{git}'\nout=$(mktemp)\n\"$real\" \"$@\" > \"$out\"\nstatus=$?\n\
            case \"$*\" in *'{trigger}'*) if [ ! -e '{fired}' ]; then : > '{fired}'; {after}; fi ;; esac\n\
            cat \"$out\"\nrm -f \"$out\"\nexit $status\n",
            git = real_git.trim(),
            fired = fired.display(),
        ),
    )
    .unwrap();
    fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).unwrap();
    format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default())
}

/// The review's reproduction: the surviving index is inspected clean, then
/// gains a staged version before removal. Removal approves the record, not
/// a snapshot of it (R14), so the record goes with the late staged version.
#[cfg(unix)]
#[test]
fn staged_work_written_to_a_missing_directory_record_after_its_check_is_removed_with_the_record() {
    let fixture = Fixture::new();
    let gone = fixture.add_worktree("feat/gone", "feat-gone");
    let admin = PathBuf::from(git(&gone, &["rev-parse", "--path-format=absolute", "--git-dir"]));
    let index = admin.join("index");
    let clean = fs::read(&index).unwrap();
    fs::write(gone.join("a"), "staged later\n").unwrap();
    git(&gone, &["add", "a"]);
    let staged = fixture.root.path().join("staged-index");
    fs::copy(&index, &staged).unwrap();
    fs::write(&index, &clean).unwrap();
    fs::remove_dir_all(&gone).unwrap();
    let path = one_shot_git_shim(&fixture, "diff-index", &format!("cp '{}' '{}'", staged.display(), index.display()));

    let output = fixture.wt(&fixture.repo()).env("PATH", path).args(["remove", "feat-gone"]).output().unwrap();

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Removed the record of worktree feat-gone (its directory was already gone)"), "{stderr}");
    assert!(listed_block(&fixture, &gone).is_none(), "the record is removed");
    assert!(!admin.exists(), "with its index");
    assert!(!fixture.branch_exists("feat/gone"), "Safe on main");
}

/// A worktree on `branch` whose index Git keeps split, optionally with
/// `staged.txt` staged first, with its directory deleted; returns its
/// administrative `index` and the `sharedindex.*` file that index references.
fn missing_with_a_split_index(fixture: &Fixture, branch: &str, dir_name: &str, stage: bool) -> (PathBuf, PathBuf, PathBuf) {
    let gone = fixture.add_worktree(branch, dir_name);
    if stage {
        fs::write(gone.join("staged.txt"), "staged\n").unwrap();
        git(&gone, &["add", "staged.txt"]);
    }
    git(&gone, &["update-index", "--split-index"]);
    let admin = PathBuf::from(git(&gone, &["rev-parse", "--path-format=absolute", "--git-dir"]));
    let shared: Vec<PathBuf> = fs::read_dir(&admin)
        .unwrap()
        .map(|dirent| dirent.unwrap().path())
        .filter(|path| path.file_name().unwrap().to_string_lossy().starts_with("sharedindex."))
        .collect();
    let [shared] = shared.as_slice() else { panic!("one shared index: {shared:?}") };
    fs::remove_dir_all(&gone).unwrap();
    (gone, admin.join("index"), shared.clone())
}

/// The controls for the split-index case below: an unchanged split index
/// is removed, and its reported staged work is discarded with `--force`.
#[test]
fn a_missing_directory_with_an_unchanged_split_index_is_removed() {
    let fixture = Fixture::new();
    let (gone, _, _) = missing_with_a_split_index(&fixture, "feat/clean", "clean", false);
    let output = fixture.wt(&fixture.repo()).args(["remove", "clean"]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(listed_block(&fixture, &gone).is_none());
    assert!(!fixture.branch_exists("feat/clean"));

    let (gone, _, _) = missing_with_a_split_index(&fixture, "feat/staged", "staged", true);
    let output = fixture.wt(&fixture.repo()).args(["remove", "staged"]).output().unwrap();
    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("1 staged change in Git's record will be lost: staged.txt"), "{stderr}");
    let output = fixture.wt(&fixture.repo()).args(["remove", "staged", "--force"]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(listed_block(&fixture, &gone).is_none());
}

/// The review's split-index reproduction: the shared file a split index's
/// entries live in changes after `diff-index` checked it, leaving `index`
/// itself byte-identical. Removal approves the record, not a snapshot of it
/// (R14), so the record is removed with both index files: unforced over a
/// clean index, and with `--force` over reported staged work.
#[cfg(unix)]
#[test]
fn a_split_index_whose_shared_file_changes_after_its_check_is_removed_with_the_record() {
    let cases = [
        (false, &[][..], "corrupted", "printf corrupt > \"$shared\""),
        (true, &["--force"][..], "removed", "rm \"$shared\""),
    ];
    for (stage, flags, shape, edit) in cases {
        let fixture = Fixture::new();
        let (gone, index, shared) = missing_with_a_split_index(&fixture, "feat/split", "split", stage);
        let path = one_shot_git_shim(&fixture, "diff-index", &format!("shared='{}'; {edit}", shared.display()));

        let output = fixture.wt(&fixture.repo()).env("PATH", path).args(["remove", "split"]).args(flags).output().unwrap();

        let stderr = stderr_of(&output);
        let case = format!("{shape}, staged: {stage}, flags: {flags:?}");
        assert_eq!(output.status.code(), Some(0), "{case}: {stderr}");
        assert!(stderr.contains("Removed the record of worktree split (its directory was already gone)"), "{case}: {stderr}");
        assert!(listed_block(&fixture, &gone).is_none(), "{case}: the record is removed");
        assert!(!index.exists() && fs::symlink_metadata(&shared).is_err(), "{case}: with both index files");
        assert!(!fixture.branch_exists("feat/split"), "{case}: Safe on main");
    }
}

/// The branch was judged safe at the tip the report showed; a commit that
/// lands on it while the worktree is removed was never assessed, so the
/// branch is kept after the checkout is deleted (exit 1).
#[cfg(unix)]
#[test]
fn a_branch_that_moves_after_its_worktree_is_removed_is_kept() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let other = fixture.add_worktree("feat/other", "feat-other");
    let moved = fixture.commit(&other, "unassessed.txt");
    let path = one_shot_git_shim(
        &fixture,
        "worktree remove",
        &format!("\"$real\" -C '{}' update-ref refs/heads/feat/x {moved}", fixture.repo().display()),
    );

    let output = fixture.wt(&fixture.repo()).env("PATH", path).args(["remove", "feat-x"]).output().unwrap();

    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert!(stderr.contains(&format!("Branch feat/x was kept: it moved to {moved} after it was checked.")), "{stderr}");
    assert!(stderr.contains("Delete it with git branch -D feat/x if you no longer need it."), "{stderr}");
    assert!(!stderr.contains("deleted branch"), "{stderr}");
    assert_checkout_gone(&fixture, &wt);
    assert_eq!(git(&fixture.repo(), &["rev-parse", "refs/heads/feat/x"]), moved);
}

/// R14: `--force` approves deleting the checkout, not the snapshot the
/// report listed. A version staged after the report (here, as soon as the
/// report read the index) is deleted with the rest.
#[cfg(unix)]
#[test]
fn a_change_staged_after_the_report_is_deleted_with_force() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fs::write(wt.join("notes.txt"), "reported\n").unwrap();
    fs::write(wt.join("README.md"), "staged after the report\n").unwrap();
    let path = one_shot_git_shim(&fixture, "ls-files --stage", &format!("\"$real\" -C '{}' add README.md", wt.display()));

    // The report's index read is a Git call again, so the shim fires right
    // after it rather than at the lifecycle's binding.
    let output = fixture
        .wt(&fixture.repo())
        .env("PATH", path)
        .env(worktree::git::TEST_LIST_THROUGH_GIT_VAR, "1")
        .args(["remove", "feat-x", "--force"])
        .output()
        .unwrap();

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert_deleted(&fixture, &wt, "feat/x");
}

/// R14: a file created after the question's `git status` is deleted too,
/// and the final line names it, since the question did not.
#[cfg(unix)]
#[test]
fn a_file_created_after_the_report_is_deleted_and_named_in_the_final_line() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fs::write(wt.join("notes.txt"), "reported\n").unwrap();
    let path = one_shot_git_shim(&fixture, "status", &format!("echo late > '{}'", wt.join("late.rs").display()));

    let output = fixture.wt(&fixture.repo()).env("PATH", path).args(["remove", "feat-x", "--force"]).output().unwrap();

    let stderr = flowed(&stderr_of(&output));
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("⚠ 1 uncommitted file will be lost: notes.txt"), "{stderr}");
    assert!(stderr.contains("; also deleted 1 file changed after you answered: late.rs."), "{stderr}");
    assert_deleted(&fixture, &wt, "feat/x");
}

