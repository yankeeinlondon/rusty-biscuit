//! Resume consent: a retry asks nothing more for the unstaged deletions that
//! its own operation's confirmed deletions explain, and everything else keeps
//! the ordinary consent, in the report and in the recheck under the lock.

use std::collections::HashSet;
use std::sync::atomic::AtomicBool;

use serde_json::Value;

use super::*;
use crate::remove::inventory::DirtyEntry;
use crate::remove::lifecycle::resume::resume_with;

const README: &str = "README.md";
const SRC_FILES: [&str; 2] = ["src/a.rs", "src/b.rs"];

/// [`fixture`] plus two clean tracked files a level below `README.md`, so
/// deletion reaches them first.
fn tracked_fixture() -> Fixture {
    let fixture = fixture();
    fs::create_dir(fixture.gone.join("src")).unwrap();
    for file in SRC_FILES {
        fixture.repo.commit_in(&fixture.gone, file);
    }
    fixture
}

/// Runs `attempt` (a first removal or a retry) with an interrupt raised at
/// the first deletion batch boundary where every path of `gone` is absent
/// from the quarantine, and returns the operation it left pending.
fn stop_when_gone(fixture: &Fixture, gone: &[&str], attempt: impl FnOnce(&Environment<'_>, &mut Hook<'_>) -> Result<Removal, Crashed>) -> OperationId {
    let stop = AtomicBool::new(false);
    let mut hook = |boundary: Boundary| {
        if let Boundary::DeletionBatch(_) = boundary {
            let quarantine = fixture.quarantine_of(&fixture.pending());
            if gone.iter().all(|path| fs::symlink_metadata(quarantine.join(path)).is_err()) {
                stop.store(true, Ordering::SeqCst);
            }
        }
        false
    };
    let interrupted = || stop.load(Ordering::SeqCst);
    let stopped = finished(with_env(&Usage::clear(), &Observers::default(), &interrupted, |env| attempt(env, &mut hook)).unwrap());
    assert_eq!(stopped.report.outcome, RemovalOutcome::CleanupFailed, "{:?}", stopped.report.errors);
    let id = fixture.pending();
    assert_eq!(inspect(&fixture.base(), &id).unwrap().resolution, Resolution::Quarantined, "something is left to delete");
    id
}

/// A first removal of `fixture`, stopped once `gone` is deleted.
fn first_stopped_when_gone(fixture: &Fixture, gone: &[&str]) -> OperationId {
    let (base, entry, approval, branches) = (fixture.base(), fixture.entry(), fixture.approval(), fixture.delete_branch());
    let request = RemovalRequest { base: &base, entry: &entry, requested_name: BRANCH, approval: &approval, branches: &branches };
    stop_when_gone(fixture, gone, |env, hook| run_with(&request, env, hook))
}

fn sorted(paths: &[&str]) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    paths.sort();
    paths
}

fn excluded(consent: &ResumeConsent) -> Vec<PathBuf> {
    let mut excluded = consent.excluded.clone();
    excluded.sort();
    excluded
}

fn status_of<'i>(inventory: &'i Inventory, path: &str) -> Option<&'i str> {
    inventory.dirty.iter().find(|entry| entry.path == Path::new(path)).map(|entry| entry.status.as_str())
}

fn without_consent(approval: Approval) -> Approval {
    Approval { discard_files: false, ..approval }
}

/// The spec's case: deletion stopped after removing tracked files, and the
/// retry needs no consent for the ` D` entries that left, through the
/// library's whole resume path with nothing approved for discarding.
#[test]
fn a_retry_asks_nothing_for_tracked_files_its_own_deletions_removed() {
    let fixture = tracked_fixture();
    let id = first_stopped_when_gone(&fixture, &[README, SRC_FILES[0], SRC_FILES[1]]);

    let consent = resume_consent(&fixture.base(), &id).unwrap();
    for path in [README, SRC_FILES[0], SRC_FILES[1]] {
        assert_eq!(status_of(&consent.raw, path), Some(" D"), "{path}: {:?}", consent.raw.dirty);
        assert_eq!(status_of(&consent.inventory, path), None, "{path} leaves consent");
    }
    assert_eq!(excluded(&consent), sorted(&[README, SRC_FILES[0], SRC_FILES[1]]));
    assert!(!consent.inventory.needs_consent(), "{:?}", consent.inventory.dirty);

    let resumed = finished_resume(&fixture, &id, Some(&without_consent(fixture.resume_approval(&id))), None, &fixture.delete_branch(), &Observers::default());
    assert_removed(&fixture, &resumed);
    assert!(!fixture.branch_exists());
    // Still counted where they were deleted: excluding them from consent
    // changes no journal or report.
    let readme = resumed.report.journal_entries.iter().find(|entry| entry.path.as_path() == Path::new(README)).expect("journaled");
    assert_eq!((readme.attempt, readme.state), (1, EntryState::Deleted));
}

/// Each attempt's confirmed deletions are read: the third attempt's consent
/// excludes what the first and the second deleted.
#[test]
fn confirmed_deletions_from_several_attempts_are_all_excluded() {
    let fixture = tracked_fixture();
    let id = first_stopped_when_gone(&fixture, &SRC_FILES);
    let quarantine = fixture.quarantine_of(&id);
    assert!(quarantine.join(README).is_file(), "the first attempt stopped before the top level");

    let second = resume_consent(&fixture.base(), &id).unwrap();
    assert_eq!(excluded(&second), sorted(&SRC_FILES));
    assert_eq!(status_of(&second.inventory, README), Some(" M"), "the edit still needs consent");
    assert!(second.inventory.needs_consent());
    let approval = fixture.resume_approval(&id);
    let branches = BranchActions::default();
    let base = fixture.base();
    let request = ResumeRequest { base: &base, id: &id, approval: Some(&approval), record: None, branches: &branches };
    assert_eq!(stop_when_gone(&fixture, &[README], |env, hook| resume_with(&request, env, hook)), id);

    let third = resume_consent(&fixture.base(), &id).unwrap();
    assert_eq!(excluded(&third), sorted(&[README, SRC_FILES[0], SRC_FILES[1]]));
    assert!(!third.inventory.needs_consent(), "{:?}", third.inventory.dirty);
    let resumed = finished_resume(&fixture, &id, Some(&without_consent(fixture.resume_approval(&id))), None, &branches, &Observers::default());
    assert_removed(&fixture, &resumed);
    let attempt_of = |path: &str| {
        resumed.report.journal_entries.iter().find(|entry| entry.path.as_path() == Path::new(path) && entry.state == EntryState::Deleted).map(|entry| entry.attempt)
    };
    assert_eq!((attempt_of(SRC_FILES[0]), attempt_of(README)), (Some(1), Some(2)));
}

/// Rewrites the result of `path` in the current journal: `None` cuts the
/// journal before its results line, as a crash before those results were
/// durable leaves it.
fn rewrite_result(journal: &Path, path: &str, state: Option<&str>) {
    let text = fs::read_to_string(journal).unwrap();
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let holds = |line: &Value| {
        line["results"]["entries"].as_array().is_some_and(|entries| {
            entries.iter().any(|entry| serde_json::from_value::<NativeString>(entry["path"].clone()).is_ok_and(|named| named.as_path() == Path::new(path)))
        })
    };
    let index = lines.iter().position(|line| holds(&serde_json::from_str(line).unwrap())).expect("a results line names the path");
    match state {
        None => lines.truncate(index),
        Some(state) => {
            let mut line: Value = serde_json::from_str(&lines[index]).unwrap();
            for entry in line["results"]["entries"].as_array_mut().unwrap() {
                if serde_json::from_value::<NativeString>(entry["path"].clone()).unwrap().as_path() == Path::new(path) {
                    entry["state"] = Value::from(state);
                }
            }
            lines[index] = line.to_string();
        }
    }
    fs::write(journal, format!("{}\n", lines.join("\n"))).unwrap();
}

/// Only a durable `Deleted` result explains a missing file: an externally
/// absent or uncertain entry keeps consent, and a journal that cannot be
/// read refuses (the recovery refusal) rather than enabling anything.
#[test]
fn only_a_confirmed_result_explains_a_missing_file_and_an_unreadable_journal_refuses() {
    let fixture = fixture();
    let id = first_stopped_when_gone(&fixture, &[README]);
    let journal = fixture.store().operation_dir(&id).join(JOURNAL_FILE);
    let original = fs::read(&journal).unwrap();
    let readme_excluded = || resume_consent(&fixture.base(), &id).unwrap().excluded.contains(&PathBuf::from(README));
    assert!(readme_excluded(), "control: the unedited journal explains it");

    rewrite_result(&journal, README, Some("externally_absent"));
    let consent = resume_consent(&fixture.base(), &id).unwrap();
    assert_eq!((consent.excluded.as_slice(), status_of(&consent.inventory, README)), (&[][..], Some(" D")));
    fs::write(&journal, &original).unwrap();

    rewrite_result(&journal, README, None);
    let summary = inspect(&fixture.base(), &id).unwrap().journal.unwrap();
    assert!(summary.uncertain.total() >= 1, "{summary:?}");
    let consent = resume_consent(&fixture.base(), &id).unwrap();
    assert_eq!((consent.excluded.as_slice(), status_of(&consent.inventory, README)), (&[][..], Some(" D")));
    fs::write(&journal, &original).unwrap();

    fs::write(&journal, b"{\"header\": not json\n").unwrap();
    assert!(matches!(resume_consent(&fixture.base(), &id), Err(Refusal::Store(StoreError::Journal { .. }))));
    let approval = Approval { inventory: Inventory::default(), fingerprint: String::new(), tip: fixture.repo.sha(BRANCH), discard_files: false };
    let base = fixture.base();
    let request = ResumeRequest { base: &base, id: &id, approval: Some(&approval), record: None, branches: &BranchActions::default() };
    let refusal = refused(with_env(&Usage::clear(), &Observers::default(), &|| false, |env| resume(&request, env)));
    assert!(matches!(&refusal, Refusal::Store(StoreError::Journal { .. })), "{refusal:?}");
    assert_eq!(refusal.exit_class(), ExitClass::Refused);
    fs::write(&journal, &original).unwrap();
    assert!(readme_excluded(), "restored");
}

/// A deleted path with any other shape now keeps consent: staged, recreated
/// as a file, a directory, or a link, or a staged addition whose working
/// file is gone.
#[test]
fn staged_and_recreated_paths_keep_consent() {
    let fixture = fixture();
    let id = first_stopped_when_gone(&fixture, &[README]);
    let quarantine = fixture.quarantine_of(&id);
    let consent = || resume_consent(&fixture.base(), &id).unwrap();
    let kept = |expected: &str| {
        let consent = consent();
        assert!(consent.excluded.is_empty(), "{expected}: {:?}", consent.excluded);
        assert_eq!(status_of(&consent.inventory, README), Some(expected), "{:?}", consent.inventory.dirty);
        assert!(consent.inventory.needs_consent());
    };
    assert_eq!(excluded(&consent()), sorted(&[README]), "control");

    fixture.repo.git_in(&quarantine, &["rm", "-q", "--cached", README]);
    kept("D ");
    fixture.repo.git_in(&quarantine, &["reset", "-q", "--", README]);

    fs::write(quarantine.join(README), "recreated\n").unwrap();
    kept(" M");
    fs::remove_file(quarantine.join(README)).unwrap();

    // Git still lists the file as an unstaged deletion: only inspecting the
    // path shows something is there.
    fs::create_dir(quarantine.join(README)).unwrap();
    kept(" D");
    fs::remove_dir(quarantine.join(README)).unwrap();

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("elsewhere", quarantine.join(README)).unwrap();
        kept(" T");
        fs::remove_file(quarantine.join(README)).unwrap();
    }

    fs::write(quarantine.join("added.txt"), "staged\n").unwrap();
    fixture.repo.git_in(&quarantine, &["add", "added.txt"]);
    fs::remove_file(quarantine.join("added.txt")).unwrap();
    assert_eq!(status_of(&consent().inventory, "added.txt"), Some("AD"));
    fixture.repo.git_in(&quarantine, &["rm", "-q", "--cached", "added.txt"]);

    assert_eq!(excluded(&consent()), sorted(&[README]), "restored");
}

/// A path recreated after the retry's report is deleted with the rest, and
/// the difference from what the user agreed to is reported.
#[test]
fn a_change_between_the_report_and_the_retry_is_reported() {
    let fixture = fixture();
    let id = first_stopped_when_gone(&fixture, &[README]);
    let quarantine = fixture.quarantine_of(&id);
    let approval = without_consent(fixture.resume_approval(&id));
    assert!(!approval.inventory.needs_consent(), "{:?}", approval.inventory.dirty);
    let base = fixture.base();
    let request = ResumeRequest { base: &base, id: &id, approval: Some(&approval), record: None, branches: &BranchActions::default() };

    fs::write(quarantine.join(README), "new work\n").unwrap();
    let resumed = finished(with_env(&Usage::clear(), &Observers::default(), &|| false, |env| resume(&request, env)));
    assert_removed(&fixture, &resumed);
    assert!(resumed.warnings.iter().any(|warning| warning.contains("files")), "{:?}", resumed.warnings);
}

fn dirty(status: &str, path: &str) -> DirtyEntry {
    DirtyEntry { status: status.into(), path: PathBuf::from(path), is_source: false }
}

fn raw(entries: &[(&str, &str)]) -> Inventory {
    Inventory { dirty: entries.iter().map(|(status, path)| dirty(status, path)).collect(), ..Inventory::default() }
}

/// Every porcelain shape but an exact ` D` keeps consent, even with a
/// confirmed result for its path and nothing there.
#[test]
fn only_an_unstaged_deletion_is_explained() {
    let deleted = HashSet::from([Path::new("a.txt")]);
    for status in [" D", "D ", "DD", "UD", "DU", "AD", "MD", "RD", " M", "M ", "A ", "??", " T"] {
        let (inventory, excluded) = explain_deletions(&raw(&[(status, "a.txt")]), &deleted, |_| true);
        let explained = status == " D";
        assert_eq!((excluded.len(), inventory.dirty.len()), (usize::from(explained), usize::from(!explained)), "{status:?}");
    }
}

/// Paths match exactly and per path: no result, a present path, a deleted
/// directory above, or another case keeps consent; a journal path built
/// with this platform's separator matches Git's `/`.
#[test]
fn a_path_is_explained_only_by_its_own_confirmed_result() {
    let entries = [(" D", "no-result.txt"), (" D", "present.txt"), (" D", "src/a.rs"), (" D", "readme.md"), (" D", "lib/b.rs")];
    let lib_b = Path::new("lib").join("b.rs");
    let deleted = HashSet::from([Path::new("present.txt"), Path::new("src"), Path::new("README.md"), lib_b.as_path()]);
    let asked = std::cell::RefCell::new(Vec::new());
    let (inventory, excluded) = explain_deletions(&raw(&entries), &deleted, |path| {
        asked.borrow_mut().push(path.to_path_buf());
        path != Path::new("present.txt")
    });
    assert_eq!(excluded, vec![PathBuf::from("lib/b.rs")]);
    let kept: Vec<&Path> = inventory.dirty.iter().map(|entry| entry.path.as_path()).collect();
    assert_eq!(kept, [Path::new("no-result.txt"), Path::new("present.txt"), Path::new("src/a.rs"), Path::new("readme.md")]);
    assert_eq!(asked.into_inner(), [PathBuf::from("present.txt"), lib_b], "only paths with a result are inspected");
}
