use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::json;
use sniff::filesystem::query::{
    Access, Evidence, EvidenceKind, FileIdentity, MatchBasis, Mechanism, NativeString, PathUsageError, PathUsageReport,
    ProcessRecord,
};

use super::*;
use crate::remove::repair::{self, RepairAttempt};
use crate::remove::submodules::Submodule;
use crate::remove::test_support::{FixedMounts, TestRepo, link_dir, replace_with_link};
use crate::worktree::parse_worktree_list;

/// What a scripted query does: the report it returns, or its failure.
type Script = Box<dyn Fn(&Path) -> Result<PathUsageReport, PathUsageError> + Sync>;

/// A [`UsageSource`] answering from a script and counting its queries.
struct Usage {
    script: Script,
    queries: AtomicUsize,
}

impl Usage {
    fn answering(script: impl Fn(&Path) -> Result<PathUsageReport, PathUsageError> + Sync + 'static) -> Self {
        Self { script: Box::new(script), queries: AtomicUsize::new(0) }
    }

    /// Nobody uses the checkout, and every mechanism completed.
    fn clear() -> Self {
        Self::answering(|path| Ok(report(path)))
    }

    fn holding(processes: impl Fn(&Path) -> Vec<ProcessRecord> + Sync + 'static) -> Self {
        Self::answering(move |path| {
            let mut report = report(path);
            report.processes = processes(path);
            Ok(report)
        })
    }
}

impl UsageSource for Usage {
    fn query(&self, path: &Path, _index: Option<&std::sync::Arc<sniff::filesystem::query::TreeIndex>>) -> Result<PathUsageReport, PathUsageError> {
        self.queries.fetch_add(1, Ordering::SeqCst);
        (self.script)(path)
    }
}

/// A complete, usable report about `path` with no processes.
fn report(path: &Path) -> PathUsageReport {
    serde_json::from_value(json!({
        "schema_version": 1,
        "target": {
            "requested": {"display": path},
            "resolved": {"display": path},
            "kind": "directory",
            "identity": {"kind": "unix", "device": "7", "inode": "3"},
            "recursive": true
        },
        "outcome": "usable",
        "started_at": "2026-10-22T00:00:00Z",
        "finished_at": "2026-10-22T00:00:01Z",
        "budget_ms": 2000,
        "elapsed_us": 1000,
        "budget_exhausted": false,
        "processes": [],
        "coverage": [],
        "limitations": []
    }))
    .unwrap()
}

/// A process record with one piece of evidence on `path`. Built from JSON so
/// fields Sniff adds with defaults need no change here.
fn process(pid: u32, name: &str, kind: EvidenceKind, path: &Path, write: bool) -> ProcessRecord {
    let mut record: ProcessRecord = serde_json::from_value(json!({
        "pid": pid,
        "creation_token": "77",
        "start_time": null,
        "identity_uncertain": false,
        "name": {"display": name},
        "executable": {"display": format!("/usr/bin/{name}")},
        "user": "501",
        "evidence": []
    }))
    .unwrap();
    record.evidence.push(Evidence {
        kind,
        mechanism: match kind {
            EvidenceKind::WatchRegistration => Mechanism::Inotify,
            EvidenceKind::OpenHandle => Mechanism::OpenHandles,
            EvidenceKind::WorkingDirectory => Mechanism::WorkingDirectories,
            EvidenceKind::LoadedModule => Mechanism::LoadedModules,
        },
        matched_paths: vec![NativeString::from(path)],
        observed_path: Some(NativeString::from(path)),
        match_basis: MatchBasis::Identity,
        identity: Some(FileIdentity::Unix { device: 7, inode: 9 }),
        descriptor: Some(5),
        watch: None,
        access: (kind == EvidenceKind::OpenHandle).then_some(Access { read: true, write }),
        event_only: None,
    });
    record
}

struct FailingMounts;

impl MountInfo for FailingMounts {
    fn mounts(&self) -> io::Result<Vec<Mount>> {
        Err(io::Error::other("the mount table is unreadable"))
    }
}

/// Repairs nothing, so a missing `.git` stays unverifiable.
struct NoRepair;

impl RepairGit for NoRepair {
    fn repair(&self, _base: &Path, _target: &Path) -> RepairAttempt {
        RepairAttempt { exit_code: Some(0), ..RepairAttempt::default() }
    }

    fn rev_parse_path(&self, base: &Path, dir: &Path, flag: &str) -> Result<PathBuf, crate::error::WorktreeError> {
        repair::Git.rev_parse_path(base, dir, flag)
    }

    fn worktree_list(&self, base: &Path) -> Result<String, crate::error::WorktreeError> {
        repair::Git.worktree_list(base)
    }
}

fn mount(point: &Path, source: &str) -> Mount {
    Mount { mount_point: point.to_path_buf(), source: source.to_string(), fs_type: "ext4".into(), super_options: "rw".into() }
}

fn real(path: &Path) -> PathBuf {
    biscuit_file::canonicalize_simplified(path).unwrap()
}

/// Every injected input but the repository.
struct Inputs {
    usage: Usage,
    mounts: Box<dyn MountInfo>,
    git: Box<dyn RepairGit>,
    caller: Caller,
    host: Host,
}

impl Inputs {
    /// A clear usage report, an empty mount table, and a caller standing in
    /// the base checkout without the wrapper.
    fn new(repo: &TestRepo) -> Self {
        Self {
            usage: Usage::clear(),
            mounts: Box::new(FixedMounts::default()),
            git: Box::new(repair::Git),
            caller: Caller { cwd: repo.path(), shell_wrapper: false, shell: None, parent: None },
            host: Host::default(),
        }
    }

    fn gather(&self, repo: &TestRepo, entry: &WorktreeEntry) -> Facts {
        let base = repo.path();
        gather(&Sources {
            base: &base,
            entry,
            usage: &self.usage,
            mounts: self.mounts.as_ref(),
            git: self.git.as_ref(),
            caller: &self.caller,
            host: &self.host,
        })
    }
}

/// The listed entry recorded at `path` itself (not one a link leads to).
fn listed(repo: &TestRepo, path: &Path) -> WorktreeEntry {
    parse_worktree_list(&repo.git(&["worktree", "list", "--porcelain"]))
        .into_iter()
        .find(|entry| spelled(&entry.path) == spelled(path))
        .expect("listed")
}

fn fixture() -> (TestRepo, PathBuf) {
    let repo = TestRepo::new();
    let checkout = repo.add_worktree("feat/x", "x", "main");
    (repo, checkout)
}

fn rendered(facts: &Facts) -> Vec<String> {
    facts.warnings.iter().map(ToString::to_string).collect()
}

#[test]
fn a_clean_checkout_has_no_warnings() {
    let (repo, checkout) = fixture();
    let inputs = Inputs::new(&repo);

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    assert_eq!(facts.warnings, [], "{:?}", rendered(&facts));
    assert!(!facts.needs_force());
    assert_eq!(facts.blocker, None);
    assert!(matches!(&facts.target, Target::Checkout { path } if *path == listed(&repo, &checkout).path));
    assert_eq!(facts.prepared, Some(CheckoutState::Healthy));
    assert!(facts.admin.is_some());
    assert_eq!(facts.inventory.as_ref().map(|inventory| inventory.dirty.len()), Some(0));
    assert!(facts.usage.is_some());
    assert_eq!(inputs.usage.queries.load(Ordering::SeqCst), 1, "one usage query");
}

#[test]
fn a_lock_reason_is_shown_first() {
    let (repo, checkout) = fixture();
    repo.git(&["worktree", "lock", "--reason", "  on a USB drive \n", checkout.to_str().unwrap()]);
    fs::write(checkout.join("notes.md"), "draft").unwrap();

    let facts = Inputs::new(&repo).gather(&repo, &listed(&repo, &checkout));

    assert_eq!(facts.warnings[0], Warning::Locked { reason: Some("on a USB drive".into()) });
    assert!(facts.needs_force());
    let text = facts.warnings[0].to_string();
    assert!(text.contains("locked") && text.contains("on a USB drive") && text.contains("unlocks"), "{text}");
}

#[test]
fn a_lock_without_a_reason_says_none_was_given() {
    let (repo, checkout) = fixture();
    repo.git(&["worktree", "lock", checkout.to_str().unwrap()]);

    let facts = Inputs::new(&repo).gather(&repo, &listed(&repo, &checkout));

    assert_eq!(facts.warnings, [Warning::Locked { reason: None }]);
    assert!(facts.warnings[0].to_string().contains("no reason given"));
}

#[test]
fn a_lock_is_warned_about_when_the_directory_is_gone() {
    let (repo, checkout) = fixture();
    repo.git(&["worktree", "lock", "--reason", "kept", checkout.to_str().unwrap()]);
    fs::remove_dir_all(&checkout).unwrap();
    let inputs = Inputs::new(&repo);

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    assert_eq!(facts.target, Target::Missing);
    assert_eq!(facts.warnings, [Warning::Locked { reason: Some("kept".into()) }]);
    assert_eq!(inputs.usage.queries.load(Ordering::SeqCst), 0, "nothing to query");
}

#[test]
fn each_process_is_named_with_what_it_is_doing() {
    let (repo, checkout) = fixture();
    let mut inputs = Inputs::new(&repo);
    inputs.usage = Usage::holding(|root| {
        vec![
            process(4123, "cargo", EvidenceKind::OpenHandle, &root.join("target").join("out"), true),
            process(8124, "node", EvidenceKind::OpenHandle, &root.join("index.db"), false),
            process(3990, "rust-analyzer", EvidenceKind::WatchRegistration, &root.join("src"), false),
            process(5000, "zsh", EvidenceKind::WorkingDirectory, root, false),
            process(5001, "tool", EvidenceKind::LoadedModule, &root.join("tool"), false),
        ]
    });

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    let [Warning::Processes { programs, .. }] = facts.warnings.as_slice() else { panic!("{:?}", rendered(&facts)) };
    assert_eq!(programs.len(), 5);
    let text = facts.warnings[0].to_string();
    let out = Path::new("target").join("out");
    for expected in [
        "5 programs are using files in this worktree:".to_string(),
        format!("cargo (pid 4123) is writing {}", out.display()),
        "node (pid 8124) has index.db open".to_string(),
        "rust-analyzer (pid 3990) is watching src".to_string(),
        "zsh (pid 5000) is standing in the worktree".to_string(),
        "tool (pid 5001) is running from tool".to_string(),
        "Deleting now may make them fail.".to_string(),
    ] {
        assert!(text.contains(&expected), "{expected:?} in {text}");
    }
    assert!(facts.usage.is_some(), "the report is kept for enrichment");
}

/// The caller's own shell standing in the checkout is not "a program using
/// it": the shell-inside warning or the wrapper's move covers it. The same
/// parent holding a file open is listed.
#[test]
fn the_callers_own_shell_standing_in_the_checkout_is_not_listed() {
    let (repo, checkout) = fixture();
    let mut inputs = Inputs::new(&repo);
    inputs.caller.parent = Some(5000);
    inputs.caller.shell_wrapper = true;
    inputs.usage = Usage::holding(|root| {
        vec![
            process(5000, "zsh", EvidenceKind::WorkingDirectory, root, false),
            process(5001, "zsh", EvidenceKind::WorkingDirectory, root, false),
        ]
    });
    let facts = inputs.gather(&repo, &listed(&repo, &checkout));
    let [Warning::Processes { programs, .. }] = facts.warnings.as_slice() else { panic!("{:?}", rendered(&facts)) };
    assert_eq!(programs.iter().map(|holder| holder.pid).collect::<Vec<_>>(), [5001], "another shell is still listed");

    // Windows reports a shell's working directory as an open handle on it.
    inputs.usage = Usage::holding(|root| vec![process(5000, "powershell.exe", EvidenceKind::OpenHandle, root, false)]);
    let facts = inputs.gather(&repo, &listed(&repo, &checkout));
    assert!(facts.warnings.is_empty(), "{:?}", rendered(&facts));

    inputs.usage = Usage::holding(|root| vec![process(5000, "zsh", EvidenceKind::OpenHandle, &root.join("notes.md"), false)]);
    let facts = inputs.gather(&repo, &listed(&repo, &checkout));
    assert!(matches!(facts.warnings.as_slice(), [Warning::Processes { programs, .. }] if programs[0].pid == 5000), "{:?}", rendered(&facts));
}

#[test]
fn a_failed_usage_query_warns_that_programs_could_not_be_checked() {
    let (repo, checkout) = fixture();
    let mut inputs = Inputs::new(&repo);
    inputs.usage = Usage::answering(|path| {
        Err(PathUsageError::RootIdentity { path: path.to_path_buf(), source: io::Error::other("injected failure") })
    });

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    let [warning @ Warning::UsageIncomplete { .. }] = facts.warnings.as_slice() else { panic!("{:?}", rendered(&facts)) };
    let text = warning.to_string();
    assert!(text.contains("Some programs could not be checked") && text.contains("injected failure"), "{text}");
    assert!(facts.needs_force(), "N8");
    assert!(facts.usage.is_none());
}

#[test]
fn a_partial_usage_query_warns_with_what_was_missed() {
    let (repo, checkout) = fixture();
    let mut inputs = Inputs::new(&repo);
    inputs.usage = Usage::answering(|path| {
        let mut report = report(path);
        report.budget_exhausted = true;
        Ok(report)
    });

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    let [warning @ Warning::UsageIncomplete { .. }] = facts.warnings.as_slice() else { panic!("{:?}", rendered(&facts)) };
    assert!(warning.to_string().contains("the time allowed for the check ran out"), "{warning}");
    assert!(facts.usage.is_some());
}

/// The macOS query reports FSEvents watches as unsupported and open files
/// as partly checked on every run; neither is a miss, so a clean checkout
/// has no warning (and a non-interactive run needs no `--force`). A failed
/// mechanism is a miss.
#[test]
fn only_a_missed_usage_mechanism_warns() {
    fn with_coverage(path: &Path, coverage: serde_json::Value) -> PathUsageReport {
        let mut value = serde_json::to_value(report(path)).unwrap();
        value["coverage"] = coverage;
        serde_json::from_value(value).unwrap()
    }
    fn entry(mechanism: &str, status: &str) -> serde_json::Value {
        json!({
            "mechanism": mechanism, "scope": "target", "status": status, "attempted": 3, "succeeded": 2,
            "reason": "a reason", "truncation": null, "limitations": []
        })
    }
    let (repo, checkout) = fixture();
    let mut inputs = Inputs::new(&repo);
    inputs.usage = Usage::answering(|path| {
        Ok(with_coverage(path, json!([entry("fsevents", "unsupported"), entry("open_handles", "partial"), entry("working_directories", "complete")])))
    });
    let facts = inputs.gather(&repo, &listed(&repo, &checkout));
    assert!(facts.warnings.is_empty(), "{:?}", rendered(&facts));
    assert!(!facts.needs_force());

    let mut inputs = Inputs::new(&repo);
    inputs.usage = Usage::answering(|path| Ok(with_coverage(path, json!([entry("open_handles", "failed"), entry("fsevents", "unsupported")]))));
    let facts = inputs.gather(&repo, &listed(&repo, &checkout));
    let [warning @ Warning::UsageIncomplete { .. }] = facts.warnings.as_slice() else { panic!("{:?}", rendered(&facts)) };
    let text = warning.to_string();
    assert!(text.contains("open files failed (a reason)"), "{text}");
    assert!(!text.contains("FSEvents"), "an unsupported mechanism is not a miss: {text}");
}

/// R17: gone after the usage query is already removed, not a warning.
#[test]
fn a_checkout_that_vanishes_during_the_usage_query_is_already_removed() {
    let (repo, checkout) = fixture();
    let mut inputs = Inputs::new(&repo);
    inputs.usage = Usage::answering(|path| {
        fs::remove_dir_all(path).unwrap();
        Err(PathUsageError::MissingTarget { path: path.to_path_buf() })
    });

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    assert_eq!(facts.target, Target::Vanished);
    assert_eq!(facts.warnings, []);
    assert_eq!(facts.inventory, None);
}

#[test]
fn uncommitted_files_are_named_up_to_a_bound() {
    let (repo, checkout) = fixture();
    for name in ["a.txt", "b.txt", "c.txt", "d.txt", "e.txt", "f.txt", "g.txt"] {
        fs::write(checkout.join(name), name).unwrap();
    }

    let facts = Inputs::new(&repo).gather(&repo, &listed(&repo, &checkout));

    let [warning @ Warning::Uncommitted { files }] = facts.warnings.as_slice() else { panic!("{:?}", rendered(&facts)) };
    assert_eq!(files.len(), 7);
    assert_eq!(warning.to_string(), "7 uncommitted files will be lost: a.txt, b.txt, c.txt, d.txt, e.txt, and 2 more");
    assert_eq!(facts.inventory.unwrap().dirty.len(), 7, "kept for the late-change list");
}

/// R13: a failed `git status` gives its reason and no count.
#[test]
fn a_failed_status_is_unchecked_with_its_reason() {
    let (repo, checkout) = fixture();
    let admin = PathBuf::from(repo.git_in(&checkout, &["rev-parse", "--path-format=absolute", "--git-dir"]));
    // Longer than a checksum: a shorter file makes gix-index panic.
    fs::write(admin.join("index"), [b'x'; 64]).unwrap();

    let facts = Inputs::new(&repo).gather(&repo, &listed(&repo, &checkout));

    // The submodule inspection reads the same index and fails with it.
    let [warning @ Warning::Unchecked { .. }, Warning::SubmoduleUnchecked { path: None, .. }] = facts.warnings.as_slice() else {
        panic!("{:?}", rendered(&facts))
    };
    let text = warning.to_string();
    assert!(text.contains("Uncommitted files could not be checked") && text.contains("index file corrupt"), "{text}");
    assert_eq!(facts.inventory, None);
}

/// R10: a `.git` the repair could not verify.
#[test]
fn an_unverifiable_repair_warns_that_git_cannot_read_it() {
    let (repo, checkout) = fixture();
    fs::remove_file(checkout.join(".git")).unwrap();
    let mut inputs = Inputs::new(&repo);
    inputs.git = Box::new(NoRepair);

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    let [warning @ Warning::GitUnreadable { .. }] = facts.warnings.as_slice() else { panic!("{:?}", rendered(&facts)) };
    let text = warning.to_string();
    assert!(text.starts_with("Git cannot read this worktree: Git couldn't restore a verified link"), "{text}");
    assert!(text.contains("uncommitted files cannot be listed") && text.contains("everything in it will be deleted"), "{text}");
    assert!(matches!(facts.target, Target::Checkout { .. }));
    assert_eq!(facts.inventory, None);
}

/// The existing pre-consent repair of a missing `.git` is kept.
#[test]
fn a_missing_git_file_is_repaired_and_inventoried() {
    let (repo, checkout) = fixture();
    fs::remove_file(checkout.join(".git")).unwrap();
    fs::write(checkout.join("new.txt"), "x").unwrap();

    let facts = Inputs::new(&repo).gather(&repo, &listed(&repo, &checkout));

    assert!(matches!(facts.prepared, Some(CheckoutState::Repaired(_))));
    assert!(matches!(facts.warnings.as_slice(), [Warning::Uncommitted { files }] if files == &[PathBuf::from("new.txt")]));
}

#[test]
fn a_prunable_checkout_with_a_git_entry_gives_gits_reason() {
    let (repo, checkout) = fixture();
    let mut entry = listed(&repo, &checkout);
    entry.prunable = Some("gitdir file points to non-existent location".into());

    let facts = Inputs::new(&repo).gather(&repo, &entry);

    let [Warning::GitUnreadable { reason }] = facts.warnings.as_slice() else { panic!("{:?}", rendered(&facts)) };
    assert_eq!(reason, "gitdir file points to non-existent location");
}

/// R11: a file where the directory should be.
#[test]
fn a_file_at_the_path_says_what_is_there() {
    let (repo, checkout) = fixture();
    fs::remove_dir_all(&checkout).unwrap();
    fs::write(&checkout, "not a checkout").unwrap();
    let inputs = Inputs::new(&repo);

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    assert_eq!(facts.target, Target::NotADirectory);
    let [warning @ Warning::PathOccupied { .. }] = facts.warnings.as_slice() else { panic!("{:?}", rendered(&facts)) };
    assert!(warning.to_string().starts_with("A file is where the worktree's directory should be"), "{warning}");
    assert_eq!(inputs.usage.queries.load(Ordering::SeqCst), 0);
}

/// R11: a path whose name is too long to inspect.
#[cfg(unix)]
#[test]
fn an_uninspectable_path_gives_the_reason() {
    let (repo, checkout) = fixture();
    let mut entry = listed(&repo, &checkout);
    entry.path = checkout.with_file_name("n".repeat(300));
    entry.prunable = Some(String::new());

    let facts = Inputs::new(&repo).gather(&repo, &entry);

    assert_eq!(facts.target, Target::Uninspectable);
    let warning = facts.warnings.iter().find(|warning| matches!(warning, Warning::Uninspectable { .. })).expect("uninspectable");
    assert!(warning.to_string().contains("cannot be inspected"), "{warning}");
}

/// R12: the directory and its index are both gone.
#[test]
fn a_missing_directory_without_its_index_cannot_check_staged_changes() {
    let (repo, checkout) = fixture();
    let admin = PathBuf::from(repo.git_in(&checkout, &["rev-parse", "--path-format=absolute", "--git-dir"]));
    fs::remove_dir_all(&checkout).unwrap();
    fs::remove_file(admin.join("index")).unwrap();

    let facts = Inputs::new(&repo).gather(&repo, &listed(&repo, &checkout));

    assert_eq!(facts.target, Target::Missing);
    let [warning @ Warning::StagedUnknown { .. }] = facts.warnings.as_slice() else { panic!("{:?}", rendered(&facts)) };
    assert!(warning.to_string().contains("staged changes cannot be checked"), "{warning}");
}

#[test]
fn a_missing_directory_with_staged_changes_lists_them() {
    let (repo, checkout) = fixture();
    fs::write(checkout.join("staged.txt"), "x").unwrap();
    repo.git_in(&checkout, &["add", "staged.txt"]);
    fs::remove_dir_all(&checkout).unwrap();

    let facts = Inputs::new(&repo).gather(&repo, &listed(&repo, &checkout));

    let [warning @ Warning::StagedChanges { .. }] = facts.warnings.as_slice() else { panic!("{:?}", rendered(&facts)) };
    assert_eq!(warning.to_string(), "The worktree's directory is gone; 1 staged change in Git's record will be lost: staged.txt");
    assert!(matches!(facts.prepared, Some(CheckoutState::Missing(_))));
}

/// R9: a link to the worktree's own (moved) checkout covers both.
#[test]
fn a_link_to_the_worktrees_own_checkout_inspects_it() {
    let (repo, checkout) = fixture();
    let entry = listed(&repo, &checkout);
    let saved = replace_with_link(&checkout, "x-real");
    fs::write(saved.join("notes.md"), "draft").unwrap();

    let facts = Inputs::new(&repo).gather(&repo, &entry);

    let resolved = LinkTarget::Resolved(real(&saved));
    assert_eq!(facts.target, Target::Link { link: entry.path.clone(), resolved: resolved.clone(), own: true });
    assert_eq!(facts.warnings[0], Warning::Link { resolved, own: true });
    let text = facts.warnings[0].to_string();
    assert!(text.contains(&real(&saved).display().to_string()) && text.contains("both will be deleted"), "{text}");
    assert!(facts.warnings.contains(&Warning::Uncommitted { files: vec!["notes.md".into()] }), "{:?}", rendered(&facts));
}

#[test]
fn a_link_to_an_unrelated_directory_is_not_touched() {
    let (repo, checkout) = fixture();
    let entry = listed(&repo, &checkout);
    let elsewhere = tempfile::tempdir().unwrap();
    fs::remove_dir_all(&checkout).unwrap();
    link_dir(elsewhere.path(), &checkout);
    let inputs = Inputs::new(&repo);

    let facts = inputs.gather(&repo, &entry);

    let resolved = LinkTarget::Resolved(real(elsewhere.path()));
    assert_eq!(facts.warnings, [Warning::Link { resolved, own: false }]);
    assert!(facts.warnings[0].to_string().contains("which is not touched; only the link and Git's record are deleted"));
    assert_eq!(inputs.usage.queries.load(Ordering::SeqCst), 0, "the link's target is not inspected");
}

/// Another registered worktree's checkout is never this worktree's own,
/// though its `.git` is a valid worktree link.
#[test]
fn a_link_to_another_registered_worktree_is_never_own() {
    let (repo, checkout) = fixture();
    let other = repo.add_worktree("feat/y", "y", "main");
    let entry = listed(&repo, &checkout);
    fs::remove_dir_all(&checkout).unwrap();
    link_dir(&other, &checkout);

    let facts = Inputs::new(&repo).gather(&repo, &entry);

    assert_eq!(facts.warnings, [Warning::Link { resolved: LinkTarget::Resolved(real(&other)), own: false }], "{:?}", rendered(&facts));
    let admin = facts.admin.expect("the link's own record");
    assert_eq!(admin.dir.file_name(), Some(std::ffi::OsStr::new("x")));
}

#[test]
fn a_link_to_the_main_checkout_is_never_own() {
    let (repo, checkout) = fixture();
    let entry = listed(&repo, &checkout);
    fs::remove_dir_all(&checkout).unwrap();
    link_dir(&repo.path(), &checkout);

    let facts = Inputs::new(&repo).gather(&repo, &entry);

    assert_eq!(facts.warnings, [Warning::Link { resolved: LinkTarget::Resolved(real(&repo.path())), own: false }]);
}

#[cfg(unix)]
#[test]
fn a_shell_inside_without_the_wrapper_warns_on_unix() {
    let (repo, checkout) = fixture();
    let mut inputs = Inputs::new(&repo);
    inputs.caller.cwd = checkout.clone();

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    assert_eq!(facts.warnings, [Warning::ShellInside]);
    assert_eq!(facts.warnings[0].to_string(), "Your shell is inside this worktree; cd out of it afterward.");
    assert_eq!(facts.blocker, None);
}

#[cfg(windows)]
#[test]
fn a_shell_inside_without_the_wrapper_refuses_on_windows() {
    let (repo, checkout) = fixture();
    let mut inputs = Inputs::new(&repo);
    inputs.caller.cwd = checkout.clone();
    inputs.caller.shell = Some("pwsh.exe".into());

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    assert_eq!(facts.blocker, Some(Blocker::ShellInside { shell: Some("pwsh.exe".into()) }));
    assert_eq!(facts.warnings, []);
    let text = facts.blocker.unwrap().to_string();
    assert!(text.contains("pwsh.exe is standing in this worktree") && !text.contains('\n'), "{text}");
}

#[test]
fn a_shell_inside_with_the_wrapper_is_no_concern() {
    let (repo, checkout) = fixture();
    let mut inputs = Inputs::new(&repo);
    inputs.caller.cwd = checkout.clone();
    inputs.caller.shell_wrapper = true;

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    assert_eq!(facts.warnings, []);
    assert_eq!(facts.blocker, None);
}

#[test]
fn a_checkout_inside_a_synced_folder_warns_about_cloud_copies() {
    let (repo, checkout) = fixture();
    let mut inputs = Inputs::new(&repo);
    let root = checkout.parent().unwrap().to_path_buf();
    inputs.host = Host { synced_folders: vec![root.clone()] };

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    assert_eq!(facts.warnings, [Warning::SyncedFolder { root }]);
    assert!(facts.warnings[0].to_string().contains("removes the cloud copies too"));
}

/// R4, through an injected table carrying WSL2's `/mnt/c` options.
#[test]
fn a_checkout_on_drvfs_warns_and_suggests_windows() {
    let (repo, checkout) = fixture();
    let drive = real(checkout.parent().unwrap());
    let mut inputs = Inputs::new(&repo);
    inputs.mounts = Box::new(FixedMounts(vec![
        mount(drive.parent().unwrap(), "/dev/sda1"),
        Mount {
            mount_point: drive.clone(),
            source: "C:\\".into(),
            fs_type: "9p".into(),
            super_options: "rw,aname=drvfs;path=C:\\;uid=1000;gid=1000;symlinkroot=/mnt/".into(),
        },
    ]));

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    assert_eq!(facts.warnings, [Warning::Drvfs { mount_point: drive }]);
    let text = facts.warnings[0].to_string();
    assert!(text.contains("cannot be seen from WSL") && text.contains("slow") && text.contains("from Windows"), "{text}");
}

/// R7: a same-device bind mount is found by its mount point; the checkout's
/// own mount is not nested.
#[test]
fn a_nested_mount_names_its_source_and_stays() {
    let (repo, checkout) = fixture();
    let root = real(&checkout);
    let vendor = root.join("vendor");
    let mut inputs = Inputs::new(&repo);
    inputs.mounts = Box::new(FixedMounts(vec![mount(&root, "/dev/sda1"), mount(&vendor, "/dev/sda1")]));

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    assert_eq!(facts.warnings, [Warning::NestedMounts { mounts: vec![mount(&vendor, "/dev/sda1")] }]);
    let text = facts.warnings[0].to_string();
    assert!(text.starts_with("1 filesystem is mounted inside this worktree:"), "{text}");
    assert!(text.contains(&format!("{} (from /dev/sda1)", vendor.display())), "{text}");
    assert!(text.contains("Nothing on them is deleted") && text.contains("until they are unmounted"), "{text}");
}

#[test]
fn an_unreadable_mount_table_is_a_warning_not_silence() {
    let (repo, checkout) = fixture();
    let mut inputs = Inputs::new(&repo);
    inputs.mounts = Box::new(FailingMounts);

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    let [warning @ Warning::MountsUnchecked { .. }] = facts.warnings.as_slice() else { panic!("{:?}", rendered(&facts)) };
    assert!(warning.to_string().contains("the mount table is unreadable"), "{warning}");
}

/// R2's mapping, from hand-built reports.
#[test]
fn submodule_findings_map_to_warnings_and_notes() {
    let at_risk = Submodule {
        path: "libs/a".into(),
        git_dir: GitDirOwner::Owned("modules/a".into()),
        finding: Finding::AtRisk { dirty: vec!["x.rs".into()], unique_commits: 2, no_remote: false },
    };
    let remoteless = Submodule {
        path: "libs/b".into(),
        git_dir: GitDirOwner::Owned("modules/b".into()),
        finding: Finding::AtRisk { dirty: Vec::new(), unique_commits: 4, no_remote: true },
    };
    let unchecked = Submodule {
        path: "libs/c".into(),
        git_dir: GitDirOwner::Owned("modules/c".into()),
        finding: Finding::Unchecked("git status failed".into()),
    };
    let external = Submodule { path: "libs/d".into(), git_dir: GitDirOwner::External("/srv/d.git".into()), finding: Finding::Clean };
    let clean = Submodule { path: "libs/e".into(), git_dir: GitDirOwner::Owned("modules/e".into()), finding: Finding::Clean };
    let report = SubmoduleReport { submodules: vec![at_risk, remoteless, unchecked, external, clean], unlisted: Some("no .gitmodules".into()) };
    let (mut warnings, mut notes) = (Vec::new(), Vec::new());

    submodule_findings(report, &mut warnings, &mut notes);

    let text: Vec<String> = warnings.iter().map(ToString::to_string).collect();
    let a = Path::new("libs/a").join("x.rs");
    assert_eq!(
        text,
        [
            "This worktree's submodules could not be checked (no .gitmodules); their work will be deleted.".to_string(),
            format!("Submodule libs/a has work that exists only there and will be lost: 1 uncommitted file ({}) and 2 commits on no remote.", a.display()),
            "Submodule libs/b has work that exists only there and will be lost: 4 commits (no remote configured).".to_string(),
            "Submodule libs/c could not be checked (git status failed); its work will be deleted.".to_string(),
        ]
    );
    assert_eq!(notes, [Note::SubmoduleGitDataRetained { path: "libs/d".into(), git_dir: "/srv/d.git".into() }]);
    assert!(notes[0].to_string().contains("that data is not deleted"));
}

/// Checkpoint 4: several conditions at once render every item, lock first.
#[test]
fn a_combined_fixture_renders_every_warning_lock_first() {
    let (repo, checkout) = fixture();
    repo.git(&["worktree", "lock", "--reason", "pinned", checkout.to_str().unwrap()]);
    fs::write(checkout.join("notes.md"), "draft").unwrap();
    let root = real(&checkout);
    let mut inputs = Inputs::new(&repo);
    inputs.mounts = Box::new(FixedMounts(vec![mount(&root.join("cache"), "tmpfs")]));
    inputs.usage = Usage::answering(|path| {
        let mut report = report(path);
        report.budget_exhausted = true;
        report.processes = vec![process(4123, "cargo", EvidenceKind::OpenHandle, &path.join("notes.md"), false)];
        Ok(report)
    });

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    let text = rendered(&facts);
    assert_eq!(text.len(), 5, "{text:#?}");
    assert!(text[0].contains("locked") && text[0].contains("pinned"), "{text:#?}");
    for expected in ["cargo (pid 4123) has notes.md open", "Some programs could not be checked", "1 uncommitted file will be lost: notes.md", "mounted inside this worktree"] {
        assert!(text.iter().any(|item| item.contains(expected)), "{expected:?} in {text:#?}");
    }
    assert!(facts.needs_force());
}

// ---- Process context (R27) --------------------------------------------------

fn ancestor(pid: u32, name: &str, app_name: Option<&str>) -> sniff::filesystem::query::Ancestor {
    sniff::filesystem::query::Ancestor { pid, name: Some(NativeString::from(name)), app_name: app_name.map(NativeString::from) }
}

/// A holder with one open handle at `root/<file>`.
fn holder(name: Option<&str>, file: &str, writing: bool, root: &Path) -> Holder {
    Holder {
        pid: 4123,
        creation_token: Some(77),
        identity_uncertain: false,
        name: name.map(NativeString::from),
        executable: None,
        start_time: None,
        app_name: None,
        ancestors: Vec::new(),
        evidence: vec![crate::remove::usage::HolderEvidence {
            kind: EvidenceKind::OpenHandle,
            location: EvidenceLocation::Path(NativeString::from(root.join(file))),
            writing,
        }],
    }
}

fn at(text: &str) -> chrono::DateTime<chrono::FixedOffset> {
    chrono::DateTime::parse_from_rfc3339(text).unwrap()
}

/// The spec's example: name, PID, start time, and the terminal it was run
/// from, then what it is doing.
#[test]
fn a_process_is_named_with_its_start_and_the_known_program_it_was_run_from() {
    let root = Path::new("/w");
    let mut cargo = holder(Some("cargo"), "out", true, root);
    cargo.start_time = Some(at("2026-10-22T14:02:30+01:00").to_utc());
    cargo.ancestors = vec![ancestor(900, "zsh", None), ancestor(800, "login", None), ancestor(700, "wezterm-gui", Some("WezTerm"))];

    assert_eq!(identify(&cargo, at("2026-10-22T18:00:00+01:00")), "cargo (pid 4123, started 14:02, run from WezTerm)");
    assert_eq!(identify(&cargo, at("2026-10-23T09:00:00+01:00")), "cargo (pid 4123, started Oct 22 14:02, run from WezTerm)");
    assert!(describe_process(&cargo, root).ends_with(") is writing out"), "{}", describe_process(&cargo, root));
}

/// Known names are matched without case or `.exe`; without one, the nearest
/// ancestor with an app name is used; with neither, nothing is claimed.
#[test]
fn the_origin_is_a_known_program_else_the_nearest_app_else_nothing() {
    let root = Path::new("/w");
    let mut node = holder(Some("node.exe"), "index.db", false, root);
    node.ancestors = vec![ancestor(10, "pwsh.exe", Some("PowerShell 7")), ancestor(9, "Code.exe", Some("Visual Studio Code"))];
    assert!(identify(&node, at("2026-10-22T00:00:00Z")).ends_with("run from VS Code)"));

    node.ancestors = vec![ancestor(10, "zsh", None), ancestor(9, "Some Helper", Some("Hammerspoon"))];
    assert!(identify(&node, at("2026-10-22T00:00:00Z")).ends_with("run from Hammerspoon)"));

    node.ancestors = vec![ancestor(10, "bash", None)];
    assert_eq!(identify(&node, at("2026-10-22T00:00:00Z")), "node.exe (pid 4123)");
}

/// Sniff's app name leads when it says more than the process name, and an
/// open handle is "open" unless writing was observed on that handle itself.
#[test]
fn an_app_name_leads_and_only_an_observed_write_reads_as_writing() {
    let root = Path::new("/w");
    let mut node = holder(Some("node.exe"), "index.db", false, root);
    node.app_name = Some(NativeString::from("Node.js JavaScript Runtime"));
    node.evidence.push(crate::remove::usage::HolderEvidence {
        kind: EvidenceKind::OpenHandle,
        location: EvidenceLocation::Path(NativeString::from(root.join("log"))),
        writing: true,
    });

    assert_eq!(
        describe_process(&node, root),
        "Node.js JavaScript Runtime (node.exe, pid 4123) is writing log, has index.db open"
    );

    let mut same = holder(Some("WezTerm"), "a", false, root);
    same.app_name = Some(NativeString::from("wezterm"));
    assert!(describe_process(&same, root).starts_with("WezTerm (pid 4123)"), "an app name equal to the name is not repeated");
    assert!(describe_process(&holder(None, "a", false, root), root).starts_with("A program (pid 4123) has a open"));
}

/// Through the real usage policy: Sniff's context on the record reaches the
/// rendered warning.
#[test]
fn sniffs_process_context_reaches_the_warning() {
    let (repo, checkout) = fixture();
    let mut inputs = Inputs::new(&repo);
    inputs.usage = Usage::holding(|root| {
        let mut record = process(4123, "cargo", EvidenceKind::OpenHandle, &root.join("out"), true);
        record.ancestors = vec![ancestor(700, "claude", None)];
        vec![record]
    });

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    let text = facts.warnings[0].to_string();
    assert!(text.contains("cargo (pid 4123, run from Claude Code) is writing out"), "{text}");
}

// ---- Submodules through the real inspector (R2) ------------------------------

/// A worktree with the submodules `names` of a one-commit source repository,
/// committed so the worktree itself is clean. Returns each submodule's
/// checkout, configured to commit.
fn with_submodules(repo: &TestRepo, source: &TestRepo, names: &[&str]) -> (PathBuf, Vec<PathBuf>) {
    let checkout = repo.add_worktree("feat/subs", "subs", "main");
    let admin = admin_entry_for(&repo.path(), &checkout).unwrap();
    let mut subs = Vec::new();
    for name in names {
        repo.git_in(&checkout, &["-c", "protocol.file.allow=always", "submodule", "add", "-q", source.path().to_str().unwrap(), name]);
        crate::test_support::append_config(
            &admin.dir.join("modules").join(name).join("config"),
            &[("user.email", "test@example.com"), ("user.name", "Test User"), ("commit.gpgsign", "false")],
        );
        subs.push(checkout.join(name));
    }
    repo.git_in(&checkout, &["commit", "-q", "-m", "submodules"]);
    (checkout, subs)
}

/// The spec's L1 case: a submodule holding an uncommitted file and an
/// unpushed commit warns about both; a clean one beside it adds nothing.
#[test]
fn a_submodule_with_work_only_there_warns_and_a_clean_one_does_not() {
    let repo = TestRepo::new();
    let source = TestRepo::new();
    let (checkout, subs) = with_submodules(&repo, &source, &["clean", "risky"]);
    repo.commit_in(&subs[1], "unpushed.txt");
    fs::write(subs[1].join("draft.txt"), "draft").unwrap();
    let inputs = Inputs::new(&repo);

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    let text = rendered(&facts);
    let submodule_warnings: Vec<&String> = text.iter().filter(|line| line.starts_with("Submodule")).collect();
    assert_eq!(submodule_warnings.len(), 1, "{text:?}");
    let warning = submodule_warnings[0];
    assert!(warning.starts_with("Submodule risky has work that exists only there"), "{warning}");
    assert!(warning.contains("draft.txt") && warning.contains("1 commit on no remote"), "{warning}");
    assert!(facts.needs_force());
    assert!(facts.notes.is_empty(), "both submodules keep their Git data in this worktree's record");
}

#[test]
fn a_worktree_whose_submodules_are_clean_has_no_warnings() {
    let repo = TestRepo::new();
    let source = TestRepo::new();
    let (checkout, _) = with_submodules(&repo, &source, &["clean"]);
    let inputs = Inputs::new(&repo);

    let facts = inputs.gather(&repo, &listed(&repo, &checkout));

    assert_eq!(rendered(&facts), Vec::<String>::new());
    assert!(!facts.needs_force());
}
