//! What a built checkout needs from the lifecycle: the usage check gets the
//! attempt's own inventory as its tree index (diagnostics after a stop walk
//! afresh), each attempt refuses before writing when its journal cannot fit,
//! and pending removals saved by earlier report versions still resume.

use std::sync::atomic::AtomicBool;

use serde_json::Value;

use super::*;

/// A first removal interrupted once two batches are deleted; returns its
/// report and the operation it left pending.
fn interrupted_first_attempt(fixture: &Fixture, usage: &Usage) -> (OperationReport, OperationId) {
    let entry = fixture.entry();
    let (approval, branches) = (fixture.approval(), fixture.delete_branch());
    let request = RemovalRequest { base: &fixture.base(), entry: &entry, requested_name: BRANCH, approval: &approval, branches: &branches };
    let interrupted = AtomicBool::new(false);
    let mut hook = |boundary: Boundary| {
        if boundary == Boundary::DeletionBatch(3) {
            interrupted.store(true, Ordering::SeqCst);
        }
        false
    };
    let stop = || interrupted.load(Ordering::SeqCst);
    let stopped = finished(with_env(usage, &Observers::default(), &stop, |env| run_with(&request, env, &mut hook)).unwrap());
    assert_eq!(stopped.report.outcome, RemovalOutcome::CleanupFailed, "{:?}", stopped.report.errors);
    (stopped.report, fixture.pending())
}

fn resume_in(fixture: &Fixture, id: &OperationId, usage: &Usage, space: &Space) -> Removal {
    let fresh = fixture.resume_approval(id);
    let (base, branches) = (fixture.base(), fixture.delete_branch());
    let request = ResumeRequest { base: &base, id, approval: Some(&fresh), record: None, branches: &branches };
    with_space(usage, &Observers::default(), &|| false, space, |env| resume(&request, env))
}

/// The first attempt's preflight gets an index of exactly its inventory,
/// rooted at the checkout; the diagnostic query after its stop gets none and
/// walks what is left. The retry's preflight gets an index of the fresh
/// inventory of survivors, which it then deletes.
#[test]
fn preflight_queries_get_their_attempts_index_and_diagnostics_get_none() {
    let fixture = fixture();
    let usage = Usage::clear();
    let (_, id) = interrupted_first_attempt(&fixture, &usage);

    let indexes = usage.indexes();
    let Some((Some(preflight), diagnostics)) = indexes.split_first() else { panic!("{indexes:?}") };
    assert_eq!(preflight.len() as u64, FIXTURE_ENTRIES.total());
    assert_eq!(preflight.root(), fixture.store().read_record(&id).unwrap().checkout_identity);
    assert!(!diagnostics.is_empty(), "the stop ran a diagnostic query");
    assert!(diagnostics.iter().all(Option::is_none), "a diagnostic walks the leftovers: {diagnostics:?}");

    let retry = Usage::clear();
    let resumed = finished(resume_in(&fixture, &id, &retry, &Space::plenty()));
    assert_removed(&fixture, &resumed);
    let retried = retry.indexes();
    let [Some(survivors)] = retried.as_slice() else { panic!("{retried:?}") };
    let deleted_by_retry = resumed.report.journal_entries.iter().filter(|entry| entry.attempt == 2).count();
    assert_eq!(survivors.len(), deleted_by_retry, "the retry's index is its own inventory");
    assert!(survivors.len() < preflight.len());
}

/// The Windows Search recheck queries with the very index the first query
/// had; only the root is verified again.
#[test]
fn the_windows_search_recheck_reuses_the_attempts_index() {
    let fixture = fixture();
    let usage = Usage::search_first(vec![search_holder(&fixture.gone)], Vec::new());
    let finished = finished(run(&fixture, &fixture.approval(), &fixture.delete_branch(), &usage, &Observers::default(), &mut |_| false).unwrap());
    assert_removed(&fixture, &finished);
    let indexes = usage.indexes();
    let [Some(first), Some(recheck)] = indexes.as_slice() else { panic!("{indexes:?}") };
    assert!(Arc::ptr_eq(first, recheck), "one index per attempt");
}

/// Too little space, or no reading, on the recovery store's filesystem
/// refuses the first attempt before anything is published: an environment
/// block (exit 4) naming the store, the bytes needed and available, and the
/// checkout, registration, and branch are as they were.
#[test]
fn a_first_attempt_whose_journal_cannot_fit_refuses_before_publishing() {
    let fixture = fixture();
    let store_root = fixture.store().root().to_path_buf();
    for space in [Space::reading(Ok(0)), Space::reading(Err("the volume is offline"))] {
        let entry = fixture.entry();
        let (approval, branches) = (fixture.approval(), fixture.delete_branch());
        let request = RemovalRequest { base: &fixture.base(), entry: &entry, requested_name: BRANCH, approval: &approval, branches: &branches };
        let refusal = refused(with_space(&Usage::clear(), &Observers::default(), &|| false, &space, |env| remove_checkout(&request, env)));

        assert_eq!(refusal.exit_class(), ExitClass::Blocked, "{refusal}");
        let message = refusal.to_string();
        assert!(message.contains(&store_root.display().to_string()), "{message}");
        match &refusal {
            Refusal::Store(StoreError::InsufficientSpace { path, needed, available }) => {
                assert_eq!((path, *available), (&store_root, 0));
                assert!(*needed > 0 && message.contains(&needed.to_string()) && message.contains(" 0 bytes available"), "{message}");
            }
            Refusal::Store(StoreError::SpaceUnknown { path, reason, .. }) => {
                assert_eq!((path, reason.as_str()), (&store_root, "the volume is offline"));
                assert!(message.contains("the volume is offline"), "{message}");
            }
            other => panic!("expected a space refusal, got {other:?}"),
        }
        assert_eq!(space.asked(), std::slice::from_ref(&store_root));
        assert!(fixture.untouched());
        assert!(fixture.no_pending(), "no operation was published");
    }
    // The control: the same removal with room runs.
    assert_removed(&fixture, &finished(run_plain(&fixture)));
}

/// A retry that cannot fit its new journal (plus the previous journal's
/// archive copy) refuses before any further deletion, archive, or
/// replacement: the moved checkout, the journal, and the earlier report are
/// as the stop left them, and a retry with room then finishes, accounting
/// for every entry once.
#[test]
fn a_retry_whose_journal_cannot_fit_refuses_before_archiving_or_deleting() {
    let fixture = fixture();
    let (first, id) = interrupted_first_attempt(&fixture, &Usage::clear());
    let quarantine = fixture.quarantine_of(&id);
    let dir = fixture.store().operation_dir(&id);
    let listing = |path: &Path| crate::remove::lifecycle::retained_entries(path, &first.quarantine.as_ref().unwrap().identity).unwrap();
    let left = listing(&quarantine);
    let journal = fs::read(dir.join(JOURNAL_FILE)).unwrap();
    let report = fs::read(dir.join(REPORT_FILE)).unwrap();

    let space = Space::reading(Ok(1));
    let refusal = match resume_in(&fixture, &id, &Usage::clear(), &space) {
        Removal::Refused(refusal) => *refusal,
        Removal::Finished(finished) => panic!("expected a refusal, got {:?}", finished.report),
    };
    assert!(matches!(&refusal, Refusal::Store(StoreError::InsufficientSpace { path, available: 1, .. }) if *path == dir), "{refusal:?}");
    assert_eq!(refusal.exit_class(), ExitClass::Blocked);
    assert_eq!(space.asked(), std::slice::from_ref(&dir));
    assert_eq!(listing(&quarantine), left, "nothing more was deleted");
    assert_eq!(fs::read(dir.join(JOURNAL_FILE)).unwrap(), journal);
    assert!(!dir.join("journal-1.jsonl").exists(), "nothing was archived");
    assert_eq!(fs::read(dir.join(REPORT_FILE)).unwrap(), report);
    assert_eq!(fixture.pending(), id);
    assert!(fixture.branch_exists());

    let resumed = finished(resume_in(&fixture, &id, &Usage::clear(), &Space::plenty()));
    assert_removed(&fixture, &resumed);
    assert_accounted_once(&resumed.report, FIXTURE_ENTRIES);
}

/// Removes every `key` from `value`, at any depth.
fn strip(value: &mut Value, key: &str) {
    match value {
        Value::Object(map) => {
            map.remove(key);
            map.values_mut().for_each(|value| strip(value, key));
        }
        Value::Array(items) => items.iter_mut().for_each(|value| strip(value, key)),
        _ => {}
    }
}

/// A pending removal whose saved report a version 4 or 5 `wt` wrote (Sniff
/// reports without index provenance; version 4 also without observation
/// diagnostics) resumes under a newly sized journal: its discovery reads as
/// `unknown` provenance, its attempt and confirmed deletions are kept, and
/// the saved report is version 6 and reads back.
#[test]
fn a_pending_removal_from_report_versions_4_and_5_resumes_without_losing_its_evidence() {
    for version in [4u64, 5] {
        let fixture = fixture();
        let (first, id) = interrupted_first_attempt(&fixture, &Usage::clear());
        let path = fixture.store().operation_dir(&id).join(REPORT_FILE);
        let mut saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        saved["report_version"] = version.into();
        strip(&mut saved, "index_provenance");
        if version == 4 {
            strip(&mut saved, "observation");
        }
        fs::write(&path, serde_json::to_vec_pretty(&saved).unwrap()).unwrap();
        let old = fixture.store().read_report(&id).unwrap().expect("an old report reads");
        assert_eq!(old.report_version as u64, version);
        let DiscoveryResult::Report(preflight) = &old.discovery[0].result else { panic!("{:?}", old.discovery) };
        assert_eq!(preflight.index_provenance, sniff::filesystem::query::IndexProvenance::Unknown, "version {version}");

        let resumed = finished(resume_in(&fixture, &id, &Usage::clear(), &Space::plenty()));
        assert_removed(&fixture, &resumed);
        assert_accounted_once(&resumed.report, FIXTURE_ENTRIES);
        let deleted_first: Vec<_> = first.journal_entries.iter().filter(|entry| entry.state == EntryState::Deleted).collect();
        let kept_first: Vec<_> = resumed.report.journal_entries.iter().filter(|entry| entry.attempt == 1).collect();
        assert_eq!(kept_first, deleted_first, "version {version}: the earlier confirmed deletions are kept");
        let [earlier] = resumed.report.earlier_attempts.as_slice() else { panic!("{:?}", resumed.report.earlier_attempts) };
        assert_eq!(earlier.discovery, old.discovery, "version {version}");
        assert_eq!(earlier.observation.is_none(), version == 4);
        let reread = saved_report(&resumed);
        assert_eq!(reread.report_version, crate::remove::outcome::REPORT_VERSION);
        assert_eq!(reread.report_version, 6);
    }
}
