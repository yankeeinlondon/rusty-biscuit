use std::sync::{Arc, Barrier, Mutex};

use super::*;

struct TestRoot(PathBuf);

impl TestRoot {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "playa-detached-{name}-{}-{}-{}",
            std::process::id(),
            unix_millis(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        ensure_spool_root(&path).expect("isolated spool should be created");
        Self(path)
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn ready_envelope(root: &Path, sequence: u64, name: &str) -> JobEnvelope {
    let source = root.join("files").join(format!("{name}.wav"));
    fs::write(&source, b"fake audio").expect("fixture audio should write");
    let executable = absolute_current_exe().expect("test executable should resolve");
    JobEnvelope {
        schema_version: SCHEMA_VERSION,
        job_id: JobId::allocated(sequence, biscuit_hash::xx_hash(name)),
        sequence,
        enqueued_at: chrono::Utc::now(),
        enqueuer_executable: OsValue::from(executable.clone().into_os_string()),
        enqueuer_fingerprint: Some(
            fingerprint_file(&executable).expect("test executable should fingerprint"),
        ),
        capability_version: CAPABILITY_VERSION,
        state: JobState::Ready {
            job: SpoolJob::PlayFile {
                path: OsValue::from(source.into_os_string()),
                playback: DetachedPlayback::default(),
                delete_after: false,
            },
        },
    }
}

fn publish(root: &Path, envelope: &JobEnvelope) -> PathBuf {
    let path = pending_path(root, envelope);
    atomic_write_json(&path, envelope).expect("fixture job should publish");
    path
}

#[test]
fn concurrent_sequence_allocation_is_unique_and_monotonic() {
    let root_guard = TestRoot::new("sequence");
    let root = Arc::new(root_guard.0.clone());
    let mut threads = Vec::new();
    for _ in 0..16 {
        let root = Arc::clone(&root);
        threads.push(std::thread::spawn(move || {
            let queue = open_lock(&root.join("queue.lock")).expect("queue lock should open");
            queue.lock_exclusive().expect("queue lock should acquire");
            let sequence = allocate_sequence(&root).expect("sequence should allocate");
            FileExtUnlock::unlock_file(&queue).expect("queue lock should release");
            sequence
        }));
    }
    let mut sequences = threads
        .into_iter()
        .map(|thread| thread.join().expect("allocator should not panic"))
        .collect::<Vec<_>>();
    sequences.sort_unstable();
    assert_eq!(sequences, (1..=16).collect::<Vec<_>>());
}

#[test]
fn persisted_job_survives_repeated_write_read_round_trip() {
    let root = TestRoot::new("repeated-round-trip");
    let original = ready_envelope(&root.0, 7, "repeated");
    let first_path = root.0.join("first.pending.json");
    atomic_write_json(&first_path, &original).expect("first generation should persist");
    let first: JobEnvelope = serde_json::from_slice(
        &fs::read(&first_path).expect("first generation should read"),
    )
    .expect("first generation should deserialize");

    let second_path = root.0.join("second.pending.json");
    atomic_write_json(&second_path, &first).expect("second generation should persist");
    let second: JobEnvelope = serde_json::from_slice(
        &fs::read(&second_path).expect("second generation should read"),
    )
    .expect("second generation should deserialize");

    assert_eq!(first, original);
    assert_eq!(second, original);
}

#[test]
fn default_root_hashes_the_stable_user_identity_and_versions_the_spool() {
    let root = default_spool_root(Path::new("/system-temp"), "uid-501");
    assert_eq!(
        root,
        Path::new("/system-temp")
            .join(format!("playa-spool-{:016x}", biscuit_hash::xx_hash("uid-501")))
            .join("v1")
    );
    assert!(!root.to_string_lossy().contains("uid-501"));
}

#[test]
fn scheduler_dispatches_lowest_sequence_and_never_overlaps() {
    let root = TestRoot::new("ordered");
    for sequence in [3, 1, 2] {
        publish(
            &root.0,
            &ready_envelope(&root.0, sequence, &format!("job-{sequence}")),
        );
    }

    let active = Arc::new(AtomicU64::new(0));
    let maximum = Arc::new(AtomicU64::new(0));
    let observed = Arc::new(Mutex::new(Vec::new()));
    let barrier = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let root_path = root.0.clone();
        let active = Arc::clone(&active);
        let maximum = Arc::clone(&maximum);
        let observed = Arc::clone(&observed);
        let barrier = Arc::clone(&barrier);
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            run_scheduler_with(&root_path, &mut |_, envelope, _| {
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                maximum.fetch_max(current, Ordering::SeqCst);
                observed
                    .lock()
                    .expect("observation lock should hold")
                    .push(envelope.sequence);
                std::thread::sleep(Duration::from_millis(5));
                active.fetch_sub(1, Ordering::SeqCst);
                Ok(JournalOutcome::CommandExit {
                    code: Some(0),
                    success: true,
                })
            })
            .expect("scheduler should drain cleanly");
        }));
    }
    barrier.wait();
    for worker in workers {
        worker.join().expect("worker should not panic");
    }

    assert_eq!(maximum.load(Ordering::SeqCst), 1);
    assert_eq!(
        *observed.lock().expect("observation lock should hold"),
        vec![1, 2, 3]
    );
    assert!(pending_paths(&root.0).expect("queue should scan").is_empty());
}

#[test]
fn empty_handoff_lock_order_makes_publication_owned() {
    let root = TestRoot::new("handoff");
    let worker = open_lock(&root.0.join("worker.lock")).expect("worker lock should open");
    worker.lock_exclusive().expect("worker should own playback");
    let queue = open_lock(&root.0.join("queue.lock")).expect("queue lock should open");
    queue.lock_exclusive().expect("publisher should own queue");

    let probe = open_lock(&root.0.join("worker.lock")).expect("probe should open");
    assert!(
        !probe.try_lock_exclusive().expect("probe should be supported"),
        "a publication under queue.lock belongs to the current worker"
    );

    FileExtUnlock::unlock_file(&queue).expect("queue should release");
    FileExtUnlock::unlock_file(&worker).expect("worker should release");
}

#[test]
fn spawn_failure_journals_and_removes_the_unowned_publication() {
    let root = TestRoot::new("spawn-failure");
    let state = ready_envelope(&root.0, 1, "spawn-failure").state;
    let error = enqueue_state_at(&root.0, state, |_| {
        Err(PlaybackError::DetachedWorkerSpawn {
            source: std::io::Error::other("fixture refused to spawn"),
        })
    })
    .expect_err("failed spawn must fail the enqueue");
    assert!(matches!(error, PlaybackError::DetachedWorkerSpawn { .. }));
    assert!(pending_paths(&root.0).unwrap().is_empty());
    let journal = fs::read_to_string(root.0.join("journal.jsonl")).unwrap();
    assert!(journal.contains("worker_spawn_failed"));
}

#[test]
fn held_worker_lock_owns_publication_without_spawning_a_successor() {
    let root = TestRoot::new("existing-worker");
    let worker = open_lock(&root.0.join("worker.lock")).unwrap();
    worker.lock_exclusive().unwrap();
    let state = ready_envelope(&root.0, 1, "owned").state;
    let job_id = enqueue_state_at(&root.0, state, |_| {
        panic!("an active worker must own the newly published job")
    })
    .expect("active worker should make enqueue successful");
    assert!(find_job_path(&root.0, &job_id, PENDING_SUFFIX)
        .unwrap()
        .is_some());
    FileExtUnlock::unlock_file(&worker).unwrap();
}

#[test]
fn stale_preparing_and_abandoned_in_flight_advance_without_playback() {
    let root = TestRoot::new("recovery");
    let mut stale = ready_envelope(&root.0, 1, "stale");
    stale.enqueued_at = chrono::Utc::now()
        - chrono::Duration::from_std(STALE_PENDING + Duration::from_millis(1)).unwrap();
    publish(&root.0, &stale);

    let mut preparing = ready_envelope(&root.0, 2, "preparing");
    preparing.state = JobState::Preparing {
        preparation: PreparingPayload {
            preparation_version: protocol::PREPARATION_VERSION,
            deadline: chrono::Utc::now() - chrono::Duration::milliseconds(1),
            private_data: serde_json::Map::from_iter([(
                "speech".to_string(),
                serde_json::json!("never journal this phrase"),
            )]),
        },
    };
    publish(&root.0, &preparing);

    let abandoned = ready_envelope(&root.0, 3, "abandoned");
    let abandoned_path = publish(&root.0, &abandoned);
    let in_flight = abandoned_path.with_file_name(
        abandoned_path
            .file_name()
            .expect("fixture has name")
            .to_string_lossy()
            .replace(PENDING_SUFFIX, IN_FLIGHT_SUFFIX),
    );
    fs::rename(abandoned_path, in_flight).expect("fixture should become in-flight");

    let calls = AtomicU64::new(0);
    run_scheduler_with(&root.0, &mut |_, _, _| {
        calls.fetch_add(1, Ordering::SeqCst);
        Err(protocol_error("must not run"))
    })
    .expect("recovery should finish");
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let journal = fs::read_to_string(root.0.join("journal.jsonl"))
        .expect("recovery should journal outcomes");
    assert!(journal.contains("stale_pending"));
    assert!(journal.contains("preparation_timed_out"));
    assert!(journal.contains("abandoned_in_flight"));
    assert!(!journal.contains("never journal this phrase"));
}

#[test]
fn preparing_head_holds_order_until_atomic_ready_publication() {
    let root = TestRoot::new("preparing-order");
    let mut first = ready_envelope(&root.0, 1, "first");
    first.state = JobState::Preparing {
        preparation: PreparingPayload::new(serde_json::json!({
            "speech": "Phase 1 of the plan in the claudine package area, was implemented successfully"
        })),
    };
    let first_path = publish(&root.0, &first);
    publish(&root.0, &ready_envelope(&root.0, 2, "second"));

    let replacement_root = root.0.clone();
    let replacement = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(40));
        let queue = open_lock(&replacement_root.join("queue.lock")).unwrap();
        queue.lock_exclusive().unwrap();
        let mut envelope: JobEnvelope = read_secure_json(&first_path).unwrap();
        let source = replacement_root.join("files/first.wav");
        envelope.state = JobState::Ready {
            job: SpoolJob::PlayFile {
                path: OsValue::from(source.into_os_string()),
                playback: DetachedPlayback::default(),
                delete_after: false,
            },
        };
        atomic_replace_json(&first_path, &envelope).unwrap();
        FileExtUnlock::unlock_file(&queue).unwrap();
    });

    let mut observed = Vec::new();
    run_scheduler_with(&root.0, &mut |_, envelope, _| {
        observed.push(envelope.sequence);
        Ok(JournalOutcome::CommandExit {
            code: Some(0),
            success: true,
        })
    })
    .expect("scheduler should wait and drain in order");
    replacement.join().expect("preparation helper should not panic");
    assert_eq!(observed, vec![1, 2]);
}

#[test]
fn unsupported_schema_is_quarantined_without_execution() {
    let root = TestRoot::new("version");
    let envelope = ready_envelope(&root.0, 1, "future");
    let path = pending_path(&root.0, &envelope);
    let mut value = serde_json::to_value(envelope).expect("fixture should serialize");
    value["schema_version"] = serde_json::json!(2);
    atomic_write_json(&path, &value).expect("future fixture should publish");

    let calls = AtomicU64::new(0);
    run_scheduler_with(&root.0, &mut |_, _, _| {
        calls.fetch_add(1, Ordering::SeqCst);
        Err(protocol_error("must not run"))
    })
    .expect("scheduler should quarantine and exit");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(fs::read_dir(&root.0)
        .expect("spool should list")
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().ends_with(QUARANTINE_SUFFIX)));
}

#[test]
fn journal_is_redacted_and_rotates_to_one_prior_file() {
    let root = TestRoot::new("journal");
    let envelope = ready_envelope(&root.0, 1, "secret-full-path");
    append_journal(&root.0, journal_failure(&envelope, "playback_failed"))
        .expect("journal should append");
    let first = fs::read_to_string(root.0.join("journal.jsonl")).expect("journal should read");
    assert!(!first.contains("secret-full-path.wav"));

    let large_reason = "x".repeat(8192);
    for sequence in 2..80 {
        let mut entry = journal_failure(&envelope, &large_reason);
        entry.sequence = sequence;
        append_journal(&root.0, entry).expect("journal should keep rotating");
    }
    assert!(root.0.join("journal.jsonl.1").exists());
    assert!(fs::metadata(root.0.join("journal.jsonl"))
        .expect("current journal metadata")
        .len()
        <= JOURNAL_MAX_BYTES);
    assert!(fs::metadata(root.0.join("journal.jsonl.1"))
        .expect("prior journal metadata")
        .len()
        <= JOURNAL_MAX_BYTES);
}

#[cfg(unix)]
#[test]
fn private_root_has_0700_permissions_and_links_are_rejected() {
    use std::os::unix::fs::{PermissionsExt as _, symlink};

    let root = TestRoot::new("permissions");
    assert_eq!(
        fs::metadata(&root.0)
            .expect("root metadata should read")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );

    let real = root.0.join("real.json");
    fs::write(&real, b"{}").expect("target should write");
    let linked = root.0.join("linked.pending.json");
    symlink(&real, &linked).expect("test symlink should create");
    assert!(read_secure_json::<serde_json::Value>(&linked).is_err());

    let parent = root.0.parent().expect("root has parent");
    let linked_root = parent.join(format!("{}-link", root.0.file_name().unwrap().to_string_lossy()));
    symlink(&root.0, &linked_root).expect("root symlink should create");
    assert!(validate_directory(&linked_root).is_err());
    fs::remove_file(linked_root).expect("root symlink should remove");
}

#[test]
fn replaced_delegate_executable_is_rejected_before_launch() {
    let root = TestRoot::new("delegate-fingerprint");
    let mut envelope = ready_envelope(&root.0, 1, "fingerprint");
    envelope.enqueuer_fingerprint = envelope.enqueuer_fingerprint.map(|value| value ^ 1);
    let job = match &envelope.state {
        JobState::Ready { job } => job,
        _ => unreachable!("fixture is ready"),
    };
    let error = delegate_job(&root.0, &envelope, job)
        .expect_err("replaced executable must not be launched");
    assert!(error.to_string().contains("replaced"));
}

#[test]
fn shipped_v1_protocol_corpus_deserializes_and_v2_remains_unsupported() {
    let fixture = |name: &str| {
        fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../claudine/fixes/2026-09-03-tts-not-finishing/fixtures")
                .join(name),
        )
        .expect("shipped protocol fixture should read")
    };

    let ready: JobEnvelope = serde_json::from_str(&fixture("v1-ready-job.json"))
        .expect("shipped ready job should match the v1 parser");
    assert!(matches!(ready.state, JobState::Ready { .. }));
    let preparing: JobEnvelope = serde_json::from_str(&fixture("v1-preparing-job.json"))
        .expect("shipped preparing job should match the v1 parser");
    assert!(matches!(preparing.state, JobState::Preparing { .. }));
    let report: DelegatedReport = serde_json::from_str(&fixture("v1-delegated-report.json"))
        .expect("shipped delegated report should match the v1 parser");
    assert_eq!(report.sequence, 42);

    let future: serde_json::Value = serde_json::from_str(&fixture("v2-unsupported-job.json"))
        .expect("future fixture should remain valid JSON");
    assert_ne!(
        future["schema_version"],
        serde_json::json!(SCHEMA_VERSION),
        "unsupported versions must be classified before typed execution"
    );
    let journal: JournalEntry = serde_json::from_str(&fixture("v1-journal-record.json"))
        .expect("shipped journal fixture should match the diagnostic reader");
    assert_eq!(journal.sequence, 42);
}

#[test]
fn shipped_ready_artifact_runs_through_normal_stale_policy() {
    let root = TestRoot::new("shipped-artifact");
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../claudine/fixes/2026-09-03-tts-not-finishing/fixtures/v1-ready-job.json"),
    )
    .expect("shipped ready fixture should read");
    let path = root
        .0
        .join("00000000000000000042-job-0000000000000042.pending.json");
    fs::write(&path, source).expect("shipped fixture should enter isolated spool");

    let calls = AtomicU64::new(0);
    run_scheduler_with(&root.0, &mut |_, _, _| {
        calls.fetch_add(1, Ordering::SeqCst);
        Err(protocol_error("historical artifact must not play"))
    })
    .expect("normal scheduler path should consume historical fixture");

    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let journal = fs::read_to_string(root.0.join("journal.jsonl"))
        .expect("historical fixture should journal its stale outcome");
    assert!(journal.contains("stale_pending"));
}

#[cfg(unix)]
#[test]
fn detached_process_uses_a_new_process_group() {
    let mut command = Command::new("sh");
    command
        .args(["-c", "printf '%s %s' \"$$\" \"$(ps -o pgid= -p $$ | tr -d ' ')\""])
        .stdout(Stdio::piped());
    configure_detached(&mut command);
    let output = command.output().expect("process-group fixture should run");
    assert!(output.status.success());
    let ids = String::from_utf8(output.stdout).expect("fixture output should be UTF-8");
    let mut ids = ids.split_whitespace();
    assert_eq!(ids.next(), ids.next(), "child PID should equal its new PGID");
}

#[cfg(windows)]
#[test]
fn detached_process_combines_all_required_windows_flags() {
    assert_eq!(detached_creation_flags(), 0x0800_0208);
}
