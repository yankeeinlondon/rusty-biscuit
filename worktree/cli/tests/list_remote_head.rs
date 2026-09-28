//! `wt list`'s update flow against real git (spec acceptance 4): a local bare
//! `origin`, a `pusher` clone standing in for everyone else, and the listed
//! clone with one linked worktree.
//!
//! Every listing launches the worker and waits up to 3 s for its attempt, so
//! a variance is fetched before the listing gathers its refs. Some tests run
//! `wt internal-refresh <main>` as a direct child first, to know the store
//! before a listing reads it. An [`UploadPackGate`] holds one `upload-pack`
//! (the check's `ls-remote`, or the fetch) to prove the rows rendered while
//! the worker is still working.

mod perf_support;
mod remote_fixture;

use std::fs;
use std::time::{Duration, Instant};

use perf_support::{refresh_workers, wait_for_refresh_workers};
use remote_fixture::{Fixture, UploadPackGate, WORKER_WAIT};
use serial_test::serial;
use worktree::remote_head::{refresh_receipt_path, remote_head_lock_path};

#[test]
#[serial]
fn a_push_elsewhere_is_fetched_by_the_worker_and_then_reads_as_behind_and_checked() {
    let fixture = Fixture::new();
    let local = fixture.git(&fixture.main, &["rev-parse", "main"]);
    let pushed = fixture.commit_and_push("second");

    fixture.refresh();

    // The check found the variance and fetched exactly the tracking ref.
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "origin/main"]), pushed);
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "main"]), local, "the local branch did not move");
    assert!(!fixture.main.join(".git").join("FETCH_HEAD").exists(), "no FETCH_HEAD");
    assert_eq!(fixture.stored_head()["sha"], pushed, "the fetched tip is the answer");
    assert_eq!(fixture.stored_head()["source"], "fetch");
    assert_eq!(fixture.stored_document()["attempt"]["outcome"]["kind"], "fetched");

    // The listing checks again and finds no variance.
    let caption = fixture.list();
    assert!(caption.contains("main is 1 commit behind origin/main (checked just now)"), "{caption}");
    assert!(!caption.contains("tracking ref"), "{caption}");
    assert_eq!(fixture.stored_document()["attempt"]["outcome"]["kind"], "in-sync");
    assert!(wait_for_refresh_workers(&fixture.main, 0, WORKER_WAIT).is_empty(), "its worker finished");
}

#[test]
#[serial]
fn a_listing_fetches_a_variance_and_counts_from_the_fetched_tip() {
    let fixture = Fixture::new();
    let local = fixture.git(&fixture.main, &["rev-parse", "main"]);
    let feature_ref = fixture.git(&fixture.main, &["rev-parse", "feature"]);
    let pushed = fixture.commit_and_push("second");

    let caption = fixture.list();

    // Refs, counts, and caption all describe the state after the fetch.
    assert!(caption.contains("main is 1 commit behind origin/main (updated from origin just now)"), "{caption}");
    assert!(caption.contains("main is 1 commit behind origin/main; run wt --ff to fast-forward it."), "{caption}");
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "origin/main"]), pushed);
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "main"]), local, "the local branch did not move");
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "feature"]), feature_ref, "no other ref moved");
    assert!(!fixture.main.join(".git").join("FETCH_HEAD").exists(), "no FETCH_HEAD");
    assert!(!caption.contains("running this command again"), "nothing unfinished: {caption}");

    // No variance now: checked, and nothing is fetched.
    let refs = fixture.git(&fixture.main, &["for-each-ref"]);
    let caption = fixture.list();
    assert!(caption.contains("main is 1 commit behind origin/main (checked just now)"), "{caption}");
    assert_eq!(fixture.git(&fixture.main, &["for-each-ref"]), refs, "no fetch without a variance");
}

#[test]
#[serial]
fn an_in_sync_check_fetches_nothing() {
    let fixture = Fixture::new();
    let refs = fixture.git(&fixture.main, &["for-each-ref"]);

    fixture.refresh();

    assert_eq!(fixture.git(&fixture.main, &["for-each-ref"]), refs, "no ref moved");
    assert_eq!(fixture.stored_head()["source"], "git", "a local origin is checked by ls-remote");
    let attempt = &fixture.stored_document()["attempt"];
    assert_eq!(attempt["outcome"]["kind"], "in-sync");
    assert_eq!(attempt["phase"]["kind"], "checking", "a local origin is no fallback");
}

#[test]
#[serial]
fn a_forced_worker_records_the_given_attempt_and_a_receipt_for_both_halves() {
    const ID: &str = "00112233445566778899aabbccddeeff";
    let fixture = Fixture::new();
    let pushed = fixture.commit_and_push("second");
    let receipt_path = fixture.cache_file(refresh_receipt_path(&fixture.main, ID).expect("receipt path"));

    // Unforced: the attempt runs under the given id, and no receipt is written.
    fixture.run_worker(&["--attempt", ID]);
    assert_eq!(fixture.stored_document()["attempt"]["id"], ID);
    assert!(!receipt_path.exists(), "only a forced run writes a receipt");

    fixture.run_worker(&["--attempt", ID, "--force"]);
    let receipt: serde_json::Value =
        serde_json::from_slice(&fs::read(&receipt_path).expect("a receipt")).expect("json");
    assert_eq!(receipt["attempt_id"], ID);
    assert_eq!(receipt["branch"], "main");
    assert_eq!(receipt["head"], "ok", "{receipt}");
    // A local origin is no provider, so the PR half fails as `other`.
    assert_eq!(receipt["prs"], serde_json::json!({ "kind": "failed", "failure": { "kind": "other" } }), "{receipt}");
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "origin/main"]), pushed);
    let _ = fs::remove_file(receipt_path);
}

#[test]
#[serial]
fn a_manual_fetch_before_the_listing_is_checked_and_never_reported_as_a_move() {
    let fixture = Fixture::new();
    fixture.refresh();

    fixture.commit_and_push("second");
    fixture.git(&fixture.main, &["fetch", "origin"]);
    let pushed = fixture.git(&fixture.main, &["rev-parse", "origin/main"]);
    let caption = fixture.list();

    assert!(caption.contains("main is 1 commit behind origin/main (checked just now)"), "{caption}");
    for claim in ["moved", "advanced", "differs"] {
        assert!(!caption.contains(claim), "nothing {claim}: {caption}");
    }
    assert_eq!(fixture.stored_head()["sha"], pushed.as_str(), "this run's check replaced the answer");
}

#[test]
#[serial]
fn a_deleted_then_recreated_remote_branch_is_reported_absent_then_fetched_back() {
    let fixture = Fixture::new();
    let tip = fixture.git(&fixture.bare, &["rev-parse", "main"]);

    fixture.git(&fixture.bare, &["update-ref", "-d", "refs/heads/main"]);
    let caption = fixture.list();
    assert_eq!(fixture.stored_head()["sha"], serde_json::Value::Null, "a verified absence");
    assert!(
        caption.contains(
            "main is in sync with origin/main (main was absent on origin when checked just now; origin/main is a local tracking ref)"
        ),
        "{caption}"
    );
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "origin/main"]), tip, "absence deletes no ref");

    fixture.git(&fixture.main, &["fetch", "--prune", "origin"]);
    let caption = fixture.list();
    assert!(caption.contains("main was absent on origin when checked just now"), "{caption}");
    assert!(!caption.contains("in sync with"), "no tracking ref, no comparison: {caption}");

    // Recreated on origin: the listing sees it and fetches the tracking ref
    // back.
    fixture.git(&fixture.bare, &["update-ref", "refs/heads/main", &tip]);
    let caption = fixture.list();
    assert_eq!(fixture.stored_head()["sha"], tip.as_str(), "present again");
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "origin/main"]), tip, "fetched back");
    assert!(caption.contains("main is in sync with origin/main (updated from origin just now)"), "{caption}");
}

#[test]
#[serial]
fn the_main_checkout_and_a_linked_worktree_share_one_live_head_store() {
    let fixture = Fixture::new();

    // From the linked worktree, `wt list` launches the worker for the main
    // checkout, which records into the main checkout's store.
    let caption = fixture.list_from(&fixture.linked);
    assert!(caption.contains("main is in sync with origin/main (checked just now)"), "{caption}");
    let tip = fixture.git(&fixture.main, &["rev-parse", "origin/main"]);
    assert_eq!(fixture.stored_head()["sha"], tip.as_str(), "stored at the main checkout's path");

    for dir in [&fixture.main, &fixture.linked] {
        let caption = fixture.list_from(dir);
        assert!(caption.contains("(checked just now)"), "{dir:?}: {caption}");
    }
    assert!(wait_for_refresh_workers(&fixture.main, 0, WORKER_WAIT).is_empty(), "every worker finished");
}

#[test]
#[serial]
fn a_check_still_running_at_the_deadline_is_still_checking_and_the_next_run_shows_it() {
    let fixture = Fixture::new();
    fixture.refresh();
    let gate = UploadPackGate::install(&fixture, 0);

    let started = Instant::now();
    let caption = fixture.list();
    let elapsed = started.elapsed();

    assert!(
        caption.contains(
            "main is in sync with origin/main (origin hasn't answered yet; still checking in the background; last checked with origin less than 1 min ago)"
        ),
        "the previous answer stays usable and dated: {caption}"
    );
    assert!(caption.contains("running this command again will provide updated metrics"), "the hint: {caption}");
    assert!(elapsed < Duration::from_secs(5), "bounded by the 3 s wait: {elapsed:?}");
    assert_eq!(fixture.stored_document()["attempt"]["phase"]["kind"], "checking");
    assert!(fixture.stored_document()["attempt"]["outcome"].is_null(), "still running");

    gate.release();
    assert!(wait_for_refresh_workers(&fixture.main, 0, WORKER_WAIT).is_empty(), "the worker finished");
    assert_eq!(fixture.stored_document()["attempt"]["outcome"]["kind"], "in-sync", "and published");
    let caption = fixture.list();
    assert!(caption.contains("main is in sync with origin/main (checked just now)"), "{caption}");
    assert!(!caption.contains("running this command again"), "{caption}");
}

#[test]
#[serial]
fn a_fetch_still_running_at_the_deadline_is_still_pulling_and_publishes_after_the_listing() {
    let fixture = Fixture::new();
    let before = fixture.git(&fixture.main, &["rev-parse", "origin/main"]);
    let pushed = fixture.commit_and_push("second");
    let gate = UploadPackGate::install(&fixture, 1);

    let caption = fixture.list();

    // One coherent local snapshot: the tracking ref as it was, compared as
    // such.
    assert!(
        caption.contains(
            "main is in sync with local origin/main (origin differed when checked just now; pulling remote updates in the background)"
        ),
        "{caption}"
    );
    assert!(caption.contains("running this command again"), "{caption}");
    assert!(!caption.contains("run wt --ff"), "no suggestion before the fetch finished: {caption}");
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "origin/main"]), before);
    assert_eq!(fixture.stored_head()["sha"], pushed.as_str(), "the check was published before the fetch");

    gate.release();
    assert!(wait_for_refresh_workers(&fixture.main, 0, WORKER_WAIT).is_empty(), "the worker finished");
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "origin/main"]), pushed, "fetched after wt list exited");
    assert_eq!(fixture.stored_document()["attempt"]["outcome"]["kind"], "fetched");
}

#[test]
#[serial]
fn a_failed_check_still_suggests_fast_forwarding_to_the_local_tracking_ref() {
    let fixture = Fixture::new();
    fixture.commit_and_push("second");
    fixture.git(&fixture.main, &["fetch", "origin"]);
    let gate = UploadPackGate::failing(&fixture, 0);

    let caption = fixture.list();

    assert!(caption.contains("main is 1 commit behind origin/main (couldn't check origin;"), "the reason stays: {caption}");
    assert!(caption.contains("main is 1 commit behind origin/main; run wt --ff to fast-forward it."), "{caption}");
    assert_eq!(fixture.stored_document()["attempt"]["outcome"]["kind"], "check-failed");
    assert_eq!(gate.runs(), 1, "only the check ran");
}

#[test]
#[serial]
fn a_failed_fetch_still_suggests_fast_forwarding_to_the_local_tracking_ref() {
    let fixture = Fixture::new();
    fixture.commit_and_push("second");
    fixture.git(&fixture.main, &["fetch", "origin"]);
    let fetched = fixture.git(&fixture.main, &["rev-parse", "origin/main"]);
    fixture.commit_and_push("third");
    let gate = UploadPackGate::failing(&fixture, 1);

    let caption = fixture.list();

    assert!(
        caption.contains("main is 1 commit behind local origin/main (origin differed when checked just now; fetch failed"),
        "the reason stays: {caption}"
    );
    assert!(caption.contains("main is 1 commit behind origin/main; run wt --ff to fast-forward it."), "{caption}");
    assert_eq!(fixture.stored_document()["attempt"]["outcome"]["kind"], "fetch-failed");
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "origin/main"]), fetched, "the tracking ref did not move");
    assert_eq!(gate.runs(), 2, "one check and one failed fetch");
}

#[test]
#[serial]
fn a_second_listing_adopts_the_running_attempt_and_asks_origin_nothing() {
    let fixture = Fixture::new();
    let gate = UploadPackGate::install(&fixture, 0);

    let first = fixture.wt(&fixture.main).arg("list").spawn().expect("first wt list");
    gate.wait_for_runs(1);
    let caption = fixture.list();

    assert!(caption.contains("still checking in the background"), "adopted, not failed: {caption}");
    assert!(!caption.contains("couldn't check origin"), "the contender is no finished check: {caption}");
    gate.release();
    let first = first.wait_with_output().expect("first wt list finishes");
    assert!(first.status.success());
    assert!(wait_for_refresh_workers(&fixture.main, 0, WORKER_WAIT).is_empty(), "every worker finished");
    assert_eq!(gate.runs(), 1, "one check between the two listings");
}

#[test]
#[serial]
fn an_origin_that_cannot_be_read_keeps_the_previous_answer_dated() {
    let fixture = Fixture::new();
    fixture.refresh();
    let answer = fixture.stored_head();
    let moved = fixture.root.path().join("moved.git");
    fs::rename(&fixture.bare, &moved).expect("move origin away");

    let caption = fixture.list();

    assert!(
        caption.contains("main is in sync with origin/main (couldn't check origin; last checked with origin less than 1 min ago)"),
        "{caption}"
    );
    assert_eq!(fixture.stored_head(), answer, "a failed check never replaces the answer");
    assert_eq!(fixture.stored_document()["attempt"]["outcome"]["kind"], "check-failed");
    fs::rename(&moved, &fixture.bare).expect("move origin back");
}

#[test]
#[serial]
fn without_an_origin_leftover_tracking_refs_show_no_caption_and_start_no_worker() {
    let fixture = Fixture::new();
    let tip = fixture.git(&fixture.main, &["rev-parse", "origin/main"]);
    fixture.git(&fixture.main, &["remote", "remove", "origin"]);
    fixture.git(&fixture.main, &["update-ref", "refs/remotes/origin/main", &tip]);

    let caption = fixture.list();

    for text in ["tracking ref", "checked", "origin/main", "in sync"] {
        assert!(!caption.contains(text), "{text:?} shown without an origin: {caption}");
    }
    assert!(refresh_workers(&fixture.main).is_empty(), "no worker");
    assert!(!remote_head_lock_path(&fixture.head_store()).exists(), "no worker ever took the live-head lock");
    assert!(!fixture.head_store().exists());
}
