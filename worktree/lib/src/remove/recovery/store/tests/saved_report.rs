//! Saving a report with its entries file, completing, and reading saved
//! reports of every version back through [`RecoveryStore::read_report`].

use super::*;
use crate::remove::outcome::REPORT_VERSION;

/// Every recovered state at once: `a` deleted, `e/x` already absent, the
/// link `l` kept, and an open batch whose `d/a` is still present (planned)
/// and whose `d/b` is gone (uncertain).
fn every_state(f: &Fixture) -> (RecoveryRecord, Operation, Vec<JournalEntry>) {
    let saved = record(f, "wt1");
    let mut op = begin(f, saved.clone());
    op.journal().begin_batch(vec![file("a")]).unwrap();
    op.journal().finish_batch(&[ResultState::Deleted]).unwrap();
    op.journal().begin_batch(vec![file("e/x")]).unwrap();
    op.journal().finish_batch(&[ResultState::ExternallyAbsent]).unwrap();
    op.journal().begin_batch(vec![PlannedEntry { path: NativeString::from("l"), kind: EntryKind::Link }]).unwrap();
    op.journal().finish_batch(&[ResultState::Kept]).unwrap();
    op.journal().begin_batch(vec![file("d/a"), file("d/b")]).unwrap();
    let entries = op.journal_entries(|path| Ok(path == Path::new("d/a"))).unwrap();
    let states: Vec<_> = entries.iter().map(|entry| entry.state).collect();
    assert_eq!(
        states,
        [EntryState::Deleted, EntryState::ExternallyAbsent, EntryState::Kept, EntryState::Planned, EntryState::Uncertain]
    );
    (saved, op, entries)
}

fn report_for(saved: &RecoveryRecord, entries: Vec<JournalEntry>) -> OperationReport {
    let mut report = OperationReport::new(Some(saved.operation_id.clone()), RemovalOutcome::CleanupFailed, saved.original.clone());
    report.errors.push(format!("{} entries", entries.len()));
    report.set_entries(entries);
    report
}

/// The saved `report.json` as plain JSON.
fn raw_report(dir: &Path) -> Value {
    serde_json::from_slice(&fs::read(dir.join(REPORT_FILE)).unwrap()).unwrap()
}

/// `report` as [`RecoveryStore::read_report`] returns a version 6 report:
/// without its entries.
fn without_entries(report: &OperationReport) -> OperationReport {
    OperationReport { journal_entries: Vec::new(), ..report.clone() }
}

/// Hidden temporaries left in `dir`.
fn temporaries(dir: &Path) -> Vec<String> {
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".tmp"))
        .collect()
}

/// A stop's report names `entries.jsonl.gz` and lists nothing inline; the
/// file holds every entry in every state, as one terminated JSON line each,
/// with a gzip header that names no file and no time. Reading the report
/// back gives everything else unchanged.
#[test]
fn a_saved_report_keeps_its_entries_in_the_entries_file() {
    let f = fixture();
    let (saved, op, entries) = every_state(&f);
    let mut report = report_for(&saved, entries.clone());
    report.set_deleted(vec![NativeString::from("/q/a")]);
    report.timeline.push(crate::remove::outcome::TimelineEvent {
        at_ms: 7,
        step: crate::remove::outcome::Step::DeletionBatch,
        result: crate::remove::outcome::StepResult::Succeeded,
        detail: None,
    });
    op.save_report(&report).unwrap();
    let dir = op.dir().to_path_buf();

    let raw = raw_report(&dir);
    assert!(raw.get("journal_entries").is_none(), "{raw}");
    assert_eq!(raw["entries_file"], json!(ENTRIES_FILE));
    assert_eq!(raw["report_version"], json!(REPORT_VERSION));
    assert_eq!(read_entries(&dir.join(ENTRIES_FILE)), entries);
    let gzip = fs::read(dir.join(ENTRIES_FILE)).unwrap();
    assert_eq!(&gzip[..3], [0x1f, 0x8b, 8], "a deflate gzip member");
    assert_eq!(gzip[3] & 0x08, 0, "no file name in the header");
    assert_eq!(&gzip[4..8], [0, 0, 0, 0], "no modification time in the header");
    #[cfg(unix)]
    assert_eq!(mode(&dir.join(ENTRIES_FILE)), 0o600);

    let read = f.store.read_report(&saved.operation_id).unwrap().expect("saved");
    assert_eq!(read, without_entries(&report));
    assert_eq!(read.entries_file.as_deref(), Some(ENTRIES_FILE));
    assert_eq!(temporaries(&dir), Vec::<String>::new());

    // Equal entries, equal bytes.
    op.save_report(&report).unwrap();
    assert_eq!(fs::read(dir.join(ENTRIES_FILE)).unwrap(), gzip);
}

/// Each stop replaces both files; a report with no entries names none
/// (`null`) and removes the earlier entries file, so it never names a list
/// older than itself.
#[test]
fn every_save_replaces_the_entries_and_a_report_without_entries_names_none() {
    let f = fixture();
    let (saved, op, entries) = every_state(&f);
    let dir = op.dir().to_path_buf();
    op.save_report(&report_for(&saved, entries[..1].to_vec())).unwrap();
    assert_eq!(read_entries(&dir.join(ENTRIES_FILE)), entries[..1]);
    op.save_report(&report_for(&saved, entries.clone())).unwrap();
    assert_eq!(read_entries(&dir.join(ENTRIES_FILE)), entries);

    let none = report_for(&saved, Vec::new());
    assert_eq!(none.entries_file, None);
    op.save_report(&none).unwrap();
    let raw = raw_report(&dir);
    assert_eq!(raw["entries_file"], Value::Null, "explicitly null, not absent");
    assert!(raw.get("journal_entries").is_none(), "{raw}");
    assert!(!dir.join(ENTRIES_FILE).exists());
    assert_eq!(f.store.read_report(&saved.operation_id).unwrap(), Some(none));
    assert_eq!(temporaries(&dir), Vec::<String>::new());

    // Never a path taken from the report itself.
    let mut elsewhere = report_for(&saved, entries);
    elsewhere.entries_file = Some("../entries.jsonl.gz".into());
    assert!(matches!(op.save_report(&elsewhere), Err(StoreError::Io(_))));
    assert!(!dir.join(ENTRIES_FILE).exists() && !f.store.root().join(ENTRIES_FILE).exists());
}

/// Completion of a removal that deleted nothing: no entries file, and the
/// (empty) journal still goes.
#[test]
fn completion_without_entries_writes_no_entries_file() {
    let f = fixture();
    let saved = record(&f, "wt1");
    let op = begin(&f, saved.clone());
    let dir = op.dir().to_path_buf();
    let report = report_for(&saved, op.journal_entries(everything_present).unwrap());
    assert_eq!(op.complete(&report, true).unwrap(), Vec::new());
    assert_eq!(raw_report(&dir)["entries_file"], Value::Null);
    assert!(!dir.join(ENTRIES_FILE).exists() && !dir.join(JOURNAL_FILE).exists());
}

/// Completion whose journal could not be listed (the lifecycle then saves
/// no entries and says so): the journals are kept, and an entries file an
/// earlier stop saved is removed rather than left looking complete.
#[test]
fn completion_that_could_not_list_entries_keeps_the_journals_and_no_stale_entries() {
    let f = fixture();
    let (saved, op, entries) = every_state(&f);
    let dir = op.dir().to_path_buf();
    op.save_report(&report_for(&saved, entries)).unwrap();
    assert!(dir.join(ENTRIES_FILE).exists());

    assert_eq!(op.complete(&report_for(&saved, Vec::new()), false).unwrap(), Vec::new());
    assert_eq!(raw_report(&dir)["entries_file"], Value::Null);
    assert!(!dir.join(ENTRIES_FILE).exists());
    assert!(dir.join(JOURNAL_FILE).exists());
    assert_eq!(f.store.read_record(&saved.operation_id).unwrap().completed_phase, Phase::Completed);
}

/// Completion stopped at each step of publishing, by a crash (nothing more
/// written or cleaned up) or a failed write: each file is complete, old or
/// new; the journals stay until completion is durable; and completion is
/// recorded only after both files are published.
#[test]
fn completion_stopped_at_each_step_leaves_complete_files_and_the_journals() {
    let cases = [
        (SaveStep::EntriesWritten, Fault::Crash),
        (SaveStep::EntriesWritten, Fault::Fail),
        (SaveStep::EntriesFlushed, Fault::Crash),
        (SaveStep::EntriesFlushed, Fault::Fail),
        (SaveStep::EntriesPublished, Fault::Fail),
        (SaveStep::ReportPublished, Fault::Fail),
        (SaveStep::Completed, Fault::Crash),
    ];
    for (at, fault) in cases {
        let name = format!("{at:?} {fault:?}");
        let f = fixture();
        let (saved, op, entries) = every_state(&f);
        let dir = op.dir().to_path_buf();
        let old = report_for(&saved, entries[..1].to_vec());
        op.save_report(&old).unwrap();
        let new = report_for(&saved, entries.clone());

        let injected = fault::inject_save(move |step, _| (step == at).then_some(fault));
        let outcome = op.complete(&new, true);
        drop(injected);
        assert!(matches!(outcome, Err(StoreError::Io(_))), "{name}: {outcome:?}");

        let entries_published = at >= SaveStep::EntriesPublished;
        let report_published = at >= SaveStep::ReportPublished;
        let expected_entries = if entries_published { &entries[..] } else { &entries[..1] };
        assert_eq!(read_entries(&dir.join(ENTRIES_FILE)), expected_entries, "{name}");
        let expected_report = if report_published { &new } else { &old };
        assert_eq!(f.store.read_report(&saved.operation_id).unwrap(), Some(without_entries(expected_report)), "{name}");
        assert!(dir.join(JOURNAL_FILE).exists(), "{name}: the journal outlives every stop");
        let phase = f.store.read_record(&saved.operation_id).unwrap().completed_phase;
        assert_eq!(phase == Phase::Completed, at == SaveStep::Completed, "{name}: {phase:?}");
        if fault == Fault::Fail {
            assert_eq!(temporaries(&dir), Vec::<String>::new(), "{name}: a failed write removes its temporary");
        } else if at < SaveStep::EntriesPublished {
            // A crashed write leaves its hidden temporary, which no reader
            // takes for the entries file.
            assert_eq!(temporaries(&dir).len(), 1, "{name}");
        }
        if phase != Phase::Completed {
            // Still pending: the journals, not the entries file, are what
            // recovery reads.
            assert_eq!(f.store.recover(&saved.operation_id, &saved.repository, |path| Ok(path == Path::new("d/a"))).unwrap().entries, entries);
        }
    }
}

// ---- Reading saved reports of every version --------------------------------

/// A report a version 5 `wt` saved, frozen: entries inline, `observation`
/// explicitly `null`, no `entries_file`.
fn version_5_report(saved: &RecoveryRecord) -> Value {
    let entry = |batch: u64, path: &str, state: &str| json!({"attempt": 1, "batch": batch, "path": {"display": path}, "kind": "file", "state": state});
    json!({
        "report_version": 5,
        "operation_id": saved.operation_id.as_str(),
        "outcome": "activity_detected",
        "original": serde_json::to_value(&saved.original).unwrap(),
        "original_state": "moved",
        "quarantine": null,
        "registration": "at_quarantine",
        "timeline": [{"at_ms": 1, "step": "moved", "result": "succeeded", "detail": null}],
        "discovery": [],
        "activity": [],
        "journal": {
            "deleted": {"files": 1, "directories": 0, "links": 0},
            "externally_absent": {"files": 0, "directories": 0, "links": 0},
            "kept": {"files": 0, "directories": 0, "links": 0},
            "planned": {"files": 0, "directories": 0, "links": 0},
            "uncertain": {"files": 1, "directories": 0, "links": 0}
        },
        "journal_entries": [entry(1, "a", "deleted"), entry(2, "d/a", "uncertain")],
        "deleted": [{"display": "/q/a"}],
        "notes": ["concurrent_write_limit"],
        "branches": [],
        "errors": ["the removal was interrupted"],
        "truncation": {"activity_omitted": 0, "retained_omitted": 0, "deleted_omitted": 0},
        "earlier_attempts": [],
        "observation": null
    })
}

/// The same report as a version 4 `wt` saved it: no `observation`.
fn version_4_report(saved: &RecoveryRecord) -> Value {
    let mut report = version_5_report(saved);
    report["report_version"] = json!(4);
    report.as_object_mut().unwrap().remove("observation");
    report
}

/// A report this `wt` saves: `entries_file` named, nothing inline.
fn version_6_report(saved: &RecoveryRecord) -> Value {
    let mut report = version_5_report(saved);
    report["report_version"] = json!(6);
    let members = report.as_object_mut().unwrap();
    members.remove("journal_entries");
    members.insert("entries_file".into(), json!(ENTRIES_FILE));
    report
}

/// Writes `bytes` as operation `saved`'s `report.json` and reads it back.
fn read_saved(f: &Fixture, saved: &RecoveryRecord, bytes: &[u8]) -> Result<Option<OperationReport>, StoreError> {
    storage::write_durable(&f.store.operation_dir(&saved.operation_id).join(REPORT_FILE), bytes).unwrap();
    f.store.read_report(&saved.operation_id)
}

fn read_value(f: &Fixture, saved: &RecoveryRecord, value: &Value) -> Result<Option<OperationReport>, StoreError> {
    read_saved(f, saved, &serde_json::to_vec(value).unwrap())
}

/// Asserts that `result` is a located report error, naming `report.json`.
#[track_caller]
fn assert_refused(f: &Fixture, saved: &RecoveryRecord, result: Result<Option<OperationReport>, StoreError>, case: &str) {
    let path = f.store.operation_dir(&saved.operation_id).join(REPORT_FILE);
    match result {
        Err(error @ StoreError::Report { .. }) => {
            assert!(matches!(&error, StoreError::Report { path: named, .. } if *named == path), "{case}: {error:?}");
            assert!(error.to_string().contains(&path.display().to_string()), "{case}: {error}");
        }
        other => panic!("{case}: expected a report error, got {other:?}"),
    }
}

/// The controls: a version 4 report reads with unknown observation, a
/// version 5 one with its inline entries, a version 6 one naming its entries
/// file with none listed, and `null` naming none; a missing report is `None`.
#[test]
fn saved_reports_of_versions_4_5_and_6_read_back() {
    let f = fixture();
    let saved = record(&f, "wt1");
    drop(begin(&f, saved.clone()));
    assert!(f.store.read_report(&saved.operation_id).unwrap().is_none(), "never saved");

    let v4 = read_value(&f, &saved, &version_4_report(&saved)).unwrap().unwrap();
    let v5 = read_value(&f, &saved, &version_5_report(&saved)).unwrap().unwrap();
    let v6 = read_value(&f, &saved, &version_6_report(&saved)).unwrap().unwrap();
    assert_eq!((v4.report_version, v5.report_version, v6.report_version), (4, 5, 6));
    let paths = |report: &OperationReport| report.journal_entries.iter().map(|entry| entry.path.clone()).collect::<Vec<_>>();
    assert_eq!(paths(&v4), [NativeString::from("a"), NativeString::from("d/a")]);
    assert_eq!(v5.journal_entries, v4.journal_entries);
    assert_eq!(v5.journal_entries[1].state, EntryState::Uncertain);
    assert!(v6.journal_entries.is_empty());
    assert_eq!((v4.entries_file.as_deref(), v6.entries_file.as_deref()), (None, Some(ENTRIES_FILE)));
    // Everything but the entry members reads alike in every version.
    let shared = |report: OperationReport| OperationReport { report_version: 0, journal_entries: Vec::new(), entries_file: None, ..report };
    assert_eq!(shared(v4.clone()), shared(v5.clone()));
    assert_eq!(shared(v5), shared(v6.clone()));
    assert_eq!((v4.observation, v4.errors), (None, vec!["the removal was interrupted".to_string()]));

    let mut null = version_6_report(&saved);
    null["entries_file"] = Value::Null;
    assert_eq!(read_value(&f, &saved, &null).unwrap().unwrap().entries_file, None);

    // `operation_id` and `quarantine` may be `null` but not absent.
    for member in ["operation_id", "quarantine"] {
        let mut null = version_6_report(&saved);
        null[member] = Value::Null;
        let read = read_value(&f, &saved, &null).unwrap().unwrap();
        assert_eq!((read.operation_id.is_none(), read.quarantine.is_none()), (member == "operation_id", true), "{member}");
        null.as_object_mut().unwrap().remove(member);
        assert_refused(&f, &saved, read_value(&f, &saved, &null), &format!("{member} absent"));
    }

    // What this `wt` writes reads back as written, twice over.
    let mut report = v6;
    report.set_entries(Vec::new());
    for _ in 0..2 {
        let bytes = serde_json::to_vec_pretty(&report).unwrap();
        assert_eq!(read_saved(&f, &saved, &bytes).unwrap(), Some(report.clone()));
    }
}

/// `report_version`, one validator for every version: only a whole number
/// this `wt` reads.
#[test]
fn a_report_version_that_is_missing_wrong_or_unknown_is_refused() {
    let f = fixture();
    let saved = record(&f, "wt1");
    drop(begin(&f, saved.clone()));
    for (case, version) in [
        ("null", Some(Value::Null)),
        ("string", Some(json!("6"))),
        ("empty string", Some(json!(""))),
        ("empty object", Some(json!({}))),
        ("negative", Some(json!(-6))),
        ("fractional", Some(json!(5.5))),
        ("older", Some(json!(3))),
        ("newer", Some(json!(7))),
        ("absent", None),
    ] {
        let mut report = version_6_report(&saved);
        match version {
            Some(version) => report["report_version"] = version,
            None => drop(report.as_object_mut().unwrap().remove("report_version")),
        }
        assert_refused(&f, &saved, read_value(&f, &saved, &report), case);
    }
}

/// `entries_file`: required in version 6, where only `null` or the one name
/// is accepted, and unknown to versions 4 and 5. Version 6 lists no entries
/// inline, even an empty list.
#[test]
fn entries_file_must_be_null_or_its_one_name_and_only_in_version_6() {
    let f = fixture();
    let saved = record(&f, "wt1");
    drop(begin(&f, saved.clone()));
    for (case, named) in [
        ("absent", None),
        ("wrong type", Some(json!(123))),
        ("array", Some(json!([ENTRIES_FILE]))),
        ("empty", Some(json!(""))),
        ("another file", Some(json!("entries.jsonl"))),
        ("absolute", Some(json!("/tmp/entries.jsonl.gz"))),
        ("traversal", Some(json!("../entries.jsonl.gz"))),
    ] {
        let mut report = version_6_report(&saved);
        match named {
            Some(named) => report["entries_file"] = named,
            None => drop(report.as_object_mut().unwrap().remove("entries_file")),
        }
        assert_refused(&f, &saved, read_value(&f, &saved, &report), case);
    }
    for inline in [json!([]), version_5_report(&saved)["journal_entries"].clone()] {
        let mut report = version_6_report(&saved);
        report["journal_entries"] = inline;
        assert_refused(&f, &saved, read_value(&f, &saved, &report), "version 6 with inline entries");
    }
    for legacy in [version_4_report(&saved), version_5_report(&saved)] {
        for named in [Value::Null, json!(ENTRIES_FILE)] {
            let mut report = legacy.clone();
            report["entries_file"] = named.clone();
            assert_refused(&f, &saved, read_value(&f, &saved, &report), &format!("version {} with {named}", legacy["report_version"]));
        }
    }
}

/// The inline list of versions 4 and 5: required, and every element a
/// whole entry; an empty list is no entries.
#[test]
fn a_legacy_inline_entry_list_must_be_present_and_whole() {
    let f = fixture();
    let saved = record(&f, "wt1");
    drop(begin(&f, saved.clone()));
    let valid = version_5_report(&saved)["journal_entries"][0].clone();
    for (case, list) in [
        ("absent", None),
        ("null", Some(Value::Null)),
        ("wrong type", Some(json!({"entries": []}))),
        ("one element wrong", Some(json!([valid.clone(), 123]))),
        ("every element wrong", Some(json!([123]))),
    ] {
        for legacy in [version_4_report(&saved), version_5_report(&saved)] {
            let mut report = legacy.clone();
            match &list {
                Some(list) => report["journal_entries"] = list.clone(),
                None => drop(report.as_object_mut().unwrap().remove("journal_entries")),
            }
            assert_refused(&f, &saved, read_value(&f, &saved, &report), &format!("{case} in version {}", legacy["report_version"]));
        }
    }
    let mut empty = version_5_report(&saved);
    empty["journal_entries"] = json!([]);
    assert_eq!(read_value(&f, &saved, &empty).unwrap().unwrap().journal_entries, Vec::new());
}

/// Each member of an inline entry is required and typed; the path and enum
/// validators are the journal's, so one valid and one invalid case each
/// prove the wiring.
#[test]
fn every_member_of_a_legacy_inline_entry_is_required_and_typed() {
    let f = fixture();
    let saved = record(&f, "wt1");
    drop(begin(&f, saved.clone()));
    let with_entry = |edit: &dyn Fn(&mut serde_json::Map<String, Value>)| {
        let mut report = version_5_report(&saved);
        edit(report["journal_entries"][0].as_object_mut().unwrap());
        read_value(&f, &saved, &report)
    };
    for member in ["attempt", "batch", "path", "kind", "state"] {
        assert_refused(&f, &saved, with_entry(&|entry| drop(entry.remove(member))), &format!("{member} absent"));
        assert_refused(&f, &saved, with_entry(&|entry| drop(entry.insert(member.into(), Value::Null))), &format!("{member} null"));
        assert_refused(&f, &saved, with_entry(&|entry| drop(entry.insert(member.into(), json!([])))), &format!("{member} wrong type"));
        assert_refused(&f, &saved, with_entry(&|entry| drop(entry.insert(member.into(), json!("")))), &format!("{member} empty"));
    }
    for (member, invalid) in [
        ("attempt", json!(-1)),
        ("batch", json!(1.5)),
        ("path", json!({"display": "a", "surprise": 1})),
        ("kind", json!("socket")),
        ("state", json!("gone")),
    ] {
        assert_refused(&f, &saved, with_entry(&|entry| drop(entry.insert(member.into(), invalid.clone()))), &format!("{member} {invalid}"));
    }
    assert_refused(&f, &saved, with_entry(&|entry| drop(entry.insert("surprise".into(), json!(1)))), "an unknown entry member");
    let read = with_entry(&|entry| {
        entry.insert("kind".into(), json!("link"));
        entry.insert("state".into(), json!("externally_absent"));
    })
    .unwrap()
    .unwrap();
    assert_eq!((read.journal_entries[0].kind, read.journal_entries[0].state), (EntryKind::Link, EntryState::ExternallyAbsent));
}

/// The document itself: one JSON object, no key twice at any depth, no
/// unknown member, nothing after it. An empty file is unreadable, not
/// missing.
#[test]
fn a_report_document_that_is_not_one_strict_object_is_refused() {
    let f = fixture();
    let saved = record(&f, "wt1");
    drop(begin(&f, saved.clone()));
    let valid = serde_json::to_string(&version_6_report(&saved)).unwrap();
    let body = valid.strip_prefix('{').unwrap();
    for (case, text) in [
        ("empty file", String::new()),
        ("malformed", valid[..valid.len() / 2].to_string()),
        ("trailing data", format!("{valid} trailing")),
        ("a second document", format!("{valid}\n{valid}")),
        ("not an object", "[]".to_string()),
        ("a repeated top-level key", format!(r#"{{"outcome":"removed",{body}"#)),
        ("a repeated nested key", valid.replacen(r#""step":"moved""#, r#""step":"moved","step":"moved""#, 1)),
        ("an unknown member", format!(r#"{{"surprise":true,{body}"#)),
    ] {
        assert_ne!(text, valid, "{case}");
        assert_refused(&f, &saved, read_saved(&f, &saved, text.as_bytes()), case);
    }
    let StoreError::Report { reason, .. } = read_saved(&f, &saved, &valid.as_bytes()[..valid.len() / 2]).unwrap_err() else { unreachable!() };
    assert!(reason.contains("line 1 column"), "a parse error keeps its location: {reason}");
}

/// Retention dates a finished removal by its report's last timeline event
/// whatever the report version, and never opens the entries file: a
/// version 6 report beside an entries file that is not even gzip, and a
/// version 5 report listing its entries inline, both expire after 15 days
/// and are kept at 13.
#[test]
fn retention_dates_reports_of_either_shape_without_reading_the_entries() {
    let f = fixture();
    let mut cases = Vec::new();
    for (age_days, kept) in [(15, false), (13, true)] {
        let finished = NOW_MS - age_days * DAY_MS;
        // Version 6, saved through completion with its entries.
        let (saved, op, entries) = every_state(&f);
        let dir = op.dir().to_path_buf();
        let mut report = report_for(&saved, entries);
        report.timeline.push(crate::remove::outcome::TimelineEvent {
            at_ms: finished,
            step: crate::remove::outcome::Step::Completed,
            result: crate::remove::outcome::StepResult::Succeeded,
            detail: None,
        });
        assert_eq!(op.complete(&report, true).unwrap(), Vec::new());
        storage::write_durable(&dir.join(ENTRIES_FILE), b"not gzip").unwrap();
        backdate(&dir, 0);
        cases.push((format!("version 6, {age_days} days"), dir, kept));

        // Version 5, its report replaced by the frozen bytes.
        let saved = record(&f, &format!("legacy-{age_days}"));
        let op = begin(&f, saved.clone());
        let dir = op.dir().to_path_buf();
        assert_eq!(op.complete(&report_for(&saved, Vec::new()), true).unwrap(), Vec::new());
        let mut legacy = version_5_report(&saved);
        legacy["timeline"][0]["at_ms"] = json!(finished);
        storage::write_durable(&dir.join(REPORT_FILE), &serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();
        backdate(&dir, 0);
        cases.push((format!("version 5, {age_days} days"), dir, kept));
    }

    f.store.prune_completed(now());

    for (case, dir, kept) in cases {
        assert_eq!(dir.exists(), kept, "{case}");
    }
}
