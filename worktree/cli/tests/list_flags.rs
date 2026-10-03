//! The listing-only flags through the real binary (spec §7–§9): `-r` /
//! `--refresh`, `--ignore-api`, and `--ff` / `--fast-forward` parse on `wt`
//! and `wt list`, are refused with every other command, show in help and
//! completion, and do what they say against real git and local stand-ins.
//!
//! `--ignore-api` writes `~/.wt.json` under `HOME`, which native Windows does
//! not consult for the home directory, so its binary tests are Unix-only; the
//! library's `api_preference` tests cover `%USERPROFILE%`.

mod perf_support;
mod remote_fixture;

use std::fs;
use std::process::Stdio;
use std::time::{Duration, Instant};

use assert_cmd::cargo::cargo_bin;
use clap::Parser as _;
use perf_support::{FakeGitea, GiteaReply, KillOnDrop, MixedFixture, WorkerReaper, wait_for_refresh_workers};
use remote_fixture::{Fixture, UploadPackGate, WORKER_WAIT, assert_no_spinner};
use serial_test::serial;
use worktree_cli::{Cli, Commands};

/// The completion receipts (`<repo hash>.refresh-receipt.<attempt id>.json`)
/// beside `store`, whose name ends in `suffix`, with their contents.
fn receipts_beside(store: &std::path::Path, suffix: &str) -> Vec<(std::path::PathBuf, serde_json::Value)> {
    let name = store.file_name().expect("file name").to_string_lossy().into_owned();
    let prefix = format!("{}refresh-receipt.", name.strip_suffix(suffix).expect("store name"));
    let Ok(entries) = fs::read_dir(store.parent().expect("store dir")) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
        .map(|entry| {
            let document = fs::read(entry.path())
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                .unwrap_or(serde_json::Value::Null);
            (entry.path(), document)
        })
        .collect()
}

/// What a successful listing left beside the PR store once its worker
/// exited, swept afterwards as the next worker would sweep it.
///
/// The wait deletes each receipt it read, but it can end on a new PR
/// publication and a finished head before its worker has written the
/// receipt, and that receipt then survives the wait. So the only survivor
/// allowed is a complete receipt of this listing's own attempt (the one the
/// remote-head store records) whose PR half published; a worker's sweep
/// keeps it while younger than `ATTEMPT_MAX_AGE` and removes it after.
fn assert_at_most_a_late_success_receipt(fixture: &MixedFixture, context: &str) {
    use worktree::remote_head::{ATTEMPT_MAX_AGE, Attempt, load_receipt, remove_stale_receipts};

    let mut left = receipts_beside(&fixture.pr_store(), "prs.json");
    assert!(left.len() <= 1, "{context}: one launch, so at most one late receipt: {left:?}");
    let Some((path, receipt)) = left.pop() else {
        return;
    };
    let store: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.remote_head_store()).expect("remote-head store")).expect("json");
    let ours = &store["attempt"];
    assert_eq!(receipt["attempt_id"], ours["id"], "{context}: only this listing's own attempt");
    assert_eq!(receipt["prs"], serde_json::json!({ "kind": "ok" }), "{context}: a receipt the wait needed is deleted");
    let text = |value: &serde_json::Value| value.as_str().expect("a string").to_string();
    let attempt = Attempt::begin(text(&ours["id"]), text(&ours["origin_digest"]), text(&ours["branch"]), 0);
    assert!(load_receipt(&path, &attempt).is_some(), "{context}: a complete receipt: {receipt}");

    let now = std::time::SystemTime::now();
    remove_stale_receipts(&path, now);
    assert!(path.exists(), "{context}: a young receipt is kept by the sweep");
    remove_stale_receipts(&path, now + ATTEMPT_MAX_AGE + Duration::from_secs(1));
    assert!(!path.exists(), "{context}: and removed once older than ATTEMPT_MAX_AGE");
}

/// Whitespace collapsed, so wrapped lines read as one.
fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `wt <args>` in the fixture's main checkout; its collapsed stderr.
fn run(fixture: &Fixture, args: &[&str]) -> String {
    let output = fixture.wt(&fixture.main).args(args).output().expect("wt runs");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "wt {args:?} failed:\n{stderr}");
    assert_no_spinner(&stderr);
    collapsed(&stderr)
}

// Parsing, refusal, help, and completion.

#[test]
fn every_listing_flag_parses_on_wt_and_on_wt_list() {
    let parse = |args: &[&str]| Cli::try_parse_from(args).unwrap_or_else(|e| panic!("{args:?}: {e}"));
    for prefix in [&["wt"][..], &["wt", "list"][..]] {
        let with = |flags: &[&str]| parse(&[prefix, flags].concat());
        assert!(with(&["-r"]).refresh);
        assert!(with(&["--refresh"]).refresh);
        assert!(with(&["--ignore-api"]).ignore_api);
        assert!(with(&["--ff"]).fast_forward);
        assert!(with(&["--fast-forward"]).fast_forward);
        let all = with(&["--ff", "-r", "--ignore-api"]);
        assert!(all.fast_forward && all.refresh && all.ignore_api, "every combination is accepted");
        assert!(matches!(all.command, None | Some(Commands::List)));
    }
}

#[test]
fn every_listing_flag_is_refused_with_other_commands() {
    let repo = tempfile::tempdir().expect("temp dir");
    for flag in ["-r", "--refresh", "--ignore-api", "--ff", "--fast-forward"] {
        for command in [&["create", "x"][..], &["go", "base"][..], &["remove", "x"][..]] {
            for args in [[command, &[flag]].concat(), [&[flag][..], command].concat()] {
                let output = std::process::Command::new(cargo_bin("wt"))
                    .current_dir(repo.path())
                    .args(&args)
                    .output()
                    .expect("wt runs");
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert_eq!(output.status.code(), Some(2), "{args:?}: {stderr}");
                assert!(stderr.contains("applies only to listing"), "{args:?}: {stderr}");
                assert!(stderr.contains(&format!("`wt {}`", command[0])), "{args:?}: {stderr}");
                assert!(!repo.path().join("x").exists());
            }
        }
    }
}

#[test]
fn help_lists_every_listing_flag() {
    for args in [&["--help"][..], &["list", "--help"][..]] {
        let output = std::process::Command::new(cargo_bin("wt")).args(args).output().expect("help");
        assert!(output.status.success());
        let help = String::from_utf8_lossy(&output.stdout);
        for flag in ["-r, --refresh", "--ignore-api", "--fast-forward", "--ff"] {
            assert!(help.contains(flag), "{args:?} lacks {flag}:\n{help}");
        }
    }
}

#[test]
fn completion_offers_every_listing_flag() {
    let output = std::process::Command::new(cargo_bin("wt"))
        .env("COMPLETE", "fish")
        .args(["--", "wt", "--"])
        .output()
        .expect("completion");
    assert!(output.status.success());
    let mut offered: Vec<String> = String::from_utf8_lossy(&output.stdout).lines().map(str::to_string).collect();
    offered.sort();
    insta::assert_snapshot!("global_flag_completions", offered.join("\n"));
}

// `-r` / `--refresh`.

#[test]
#[serial]
fn refresh_waits_for_both_halves_and_asks_again_like_every_listing() {
    let fixture = MixedFixture::new().with_gitea_origin();
    fixture.seed_pr_store(Duration::from_secs(10), 99, "divergent-0");
    fixture.seed_remote_head_store(Duration::ZERO, Some("0123456789abcdef0123456789abcdef01234567"));
    let gitea = FakeGitea::new(GiteaReply::Open(vec![(7, "divergent-1")]));
    let _reaper = WorkerReaper::new(&fixture, &gitea);

    // An ordinary listing asks despite the young answer, too.
    let output = fixture.wt_command_via_gitea(&gitea).arg("list").env("NO_COLOR", "1").output().expect("wt list");
    assert!(output.status.success());
    assert!(wait_for_refresh_workers(fixture.main(), 0, WORKER_WAIT).is_empty());
    assert_eq!(gitea.requests(), 1, "a young answer is requested again");
    assert!(String::from_utf8_lossy(&output.stderr).contains("PR #7"), "and shown by the listing that waited for it");
    assert_at_most_a_late_success_receipt(&fixture, "wt list");
    let checks = gitea.branch_requests();

    let output = fixture.wt_command_via_gitea(&gitea).args(["-r"]).env("NO_COLOR", "1").output().expect("wt -r");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");

    assert_eq!(gitea.requests(), 2, "the forced PR half asked despite the young answer");
    assert_eq!(gitea.branch_requests(), checks + 1, "and the live-head half checked again");
    assert!(stderr.contains("PR #7"), "this run shows the answer it waited for:\n{stderr}");
    assert!(!stderr.contains("PR #99"), "{stderr}");
    assert!(!collapsed(&stderr).contains("running this command again"), "nothing was left running: {stderr}");
    assert!(wait_for_refresh_workers(fixture.main(), 0, Duration::from_secs(2)).is_empty(), "both halves ended");
    assert_at_most_a_late_success_receipt(&fixture, "wt -r");
}

/// How old the PR answer stored before `wt -r` launches is.
#[derive(Clone, Copy)]
enum Seeded {
    /// Two hours old, seeded before the holder starts.
    Old,
    /// Stamped now, seeded once the holder's request is held: its
    /// `fetched_at` is at or after the holder's, so only the publication id
    /// can show the holder's write.
    Young,
}

/// `wt -r` whose PR half finds another worker's request holding the PR lock;
/// that request is answered `holder_reply` once `wt -r`'s worker has recorded
/// the contention. Returns `wt -r`'s collapsed stderr and the PR requests made.
fn refresh_against_a_holder(holder_reply: GiteaReply, seeded: Seeded) -> (String, usize) {
    let fixture = MixedFixture::new().with_gitea_origin();
    for (stale, _) in receipts_beside(&fixture.pr_store(), "prs.json") {
        let _ = fs::remove_file(stale);
    }
    if let Seeded::Old = seeded {
        fixture.seed_pr_store(Duration::from_secs(2 * 3_600), 99, "divergent-0");
    }
    fixture.seed_remote_head_store(Duration::ZERO, Some("0123456789abcdef0123456789abcdef01234567"));
    let gitea = FakeGitea::new(GiteaReply::Status(503));
    gitea.hold();
    let _reaper = WorkerReaper::new(&fixture, &gitea);
    // Both children are killed and reaped if a setup assertion fails before
    // their waits; they drop before the reaper.
    let mut holder = KillOnDrop::spawn(fixture.refresh_worker_via_gitea(&gitea));
    assert!(gitea.wait_for_waiting(1, WORKER_WAIT), "the holder's PR request is held with its lock");
    if let Seeded::Young = seeded {
        fixture.seed_pr_store(Duration::ZERO, 99, "divergent-0");
    }

    let refresh = KillOnDrop::spawn_with(
        fixture
            .wt_command_via_gitea(&gitea)
            .arg("-r")
            .env("NO_COLOR", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
    );
    let deadline = Instant::now() + WORKER_WAIT;
    // wt -r keeps its receipt until its wait ends, which is after the
    // holder's lock opens.
    let contended = loop {
        let recorded = receipts_beside(&fixture.pr_store(), "prs.json")
            .iter()
            .any(|(_, receipt)| receipt["prs"]["kind"] == "contended");
        if recorded || Instant::now() >= deadline {
            break recorded;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    gitea.release(holder_reply);
    holder.wait();
    let output = refresh.wait_with_output();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(contended, "wt -r's worker found the PR lock held:\n{stderr}");
    assert!(output.status.success(), "{stderr}");
    assert!(wait_for_refresh_workers(fixture.main(), 0, WORKER_WAIT).is_empty(), "every worker ended");
    (collapsed(&stderr), gitea.requests())
}

#[test]
#[serial]
fn refresh_shows_the_answer_a_contending_holder_published() {
    let (stderr, requests) = refresh_against_a_holder(GiteaReply::Open(vec![(7, "divergent-1")]), Seeded::Old);

    assert!(stderr.contains("PR #7"), "{stderr}");
    assert!(!stderr.contains("PR #99") && !stderr.contains("PRs as of"), "{stderr}");
    assert_eq!(requests, 1, "the holder's answer needed no second request");
}

/// The holder replaces a young answer without moving `fetched_at` forward
/// (whole seconds, stamped at its request's start).
#[test]
#[serial]
fn refresh_shows_a_holders_answer_that_replaced_a_young_one_within_its_second() {
    let (stderr, requests) = refresh_against_a_holder(GiteaReply::Open(vec![(7, "divergent-1")]), Seeded::Young);

    assert!(stderr.contains("PR #7"), "{stderr}");
    assert!(!stderr.contains("PR #99") && !stderr.contains("PRs as of"), "{stderr}");
    assert_eq!(requests, 1, "the holder's answer needed no second request");
}

/// The holder's request fails and stores nothing, so `wt -r` asks again
/// itself; that fails too, and the old answer is shown with its age.
#[test]
#[serial]
fn refresh_asks_again_when_a_contending_holder_failed() {
    let (stderr, requests) = refresh_against_a_holder(GiteaReply::Status(500), Seeded::Old);

    assert_eq!(requests, 2, "wt -r made its own request once the holder's failed");
    assert!(stderr.contains("PR #99") && stderr.contains("PRs as of 2 h ago"), "{stderr}");
}

#[test]
#[serial]
fn refresh_on_a_local_origin_fetches_and_reports_like_a_listing() {
    let fixture = Fixture::new();
    let pushed = fixture.commit_and_push("second");

    let caption = run(&fixture, &["-r"]);

    assert!(caption.contains("main is 1 commit behind origin/main (updated from origin just now)"), "{caption}");
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "origin/main"]), pushed);
    assert_eq!(receipts_beside(&fixture.head_store(), "remote-head.json"), [], "wt -r deleted the receipt it read");
}

/// The worker can write nothing, so it records no attempt and no receipt:
/// `-r` must still end at once, not at its 75 s bound.
#[cfg(unix)]
#[test]
#[serial]
fn refresh_is_bounded_when_the_worker_can_publish_nothing() {
    use std::os::unix::fs::PermissionsExt as _;

    let fixture = Fixture::new();
    fixture.refresh();
    let stores = fixture.head_store().parent().expect("store directory").to_path_buf();
    let original = fs::metadata(&stores).expect("stores").permissions();
    fs::set_permissions(&stores, fs::Permissions::from_mode(0o555)).expect("read-only stores");

    let started = Instant::now();
    let output = fixture.wt(&fixture.main).arg("-r").output().expect("wt -r");
    let elapsed = started.elapsed();
    fs::set_permissions(&stores, original).expect("restore the stores");

    let stderr = collapsed(&String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "{stderr}");
    assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");
    assert!(stderr.contains("couldn't check origin; last checked with origin"), "{stderr}");
}

// `--ignore-api`.

#[cfg(unix)]
mod ignore_api {
    use super::*;
    use perf_support::ProxyStub;

    fn preference(fixture: &MixedFixture) -> std::path::PathBuf {
        fixture.home().join(".wt.json")
    }

    #[test]
    #[serial]
    fn the_repository_is_recorded_before_the_run_and_no_provider_is_asked() {
        let fixture = MixedFixture::new().with_github_origin();
        // A young stored answer, from before the repository was ignored.
        fixture.seed_pr_store(Duration::from_secs(10), 99, "divergent-0");
        let proxy = ProxyStub::closing_after(Duration::ZERO);

        let output =
            fixture.wt_command_via(&proxy).args(["--ignore-api"]).env("NO_COLOR", "1").output().expect("wt");
        let stderr = collapsed(&String::from_utf8_lossy(&output.stderr));
        assert!(output.status.success(), "{stderr}");
        assert!(wait_for_refresh_workers(fixture.main(), 0, WORKER_WAIT).is_empty());

        let stored: serde_json::Value =
            serde_json::from_slice(&fs::read(preference(&fixture)).expect("~/.wt.json")).expect("json");
        assert_eq!(
            stored,
            serde_json::json!({
                "format_version": 1,
                "ignore_api": [{ "host": "github.com", "port": 443, "path": "owner/repo" }],
            })
        );
        assert_eq!(proxy.connections(), 0, "no PR request and no branch-head request");
        assert!(!stderr.contains("ls-remote"), "no fallback notice: {stderr}");
        assert!(!stderr.contains("GITHUB_TOKEN"), "no credentials line: {stderr}");
        assert!(!stderr.contains("PR #"), "no badges without the API, not even stored ones: {stderr}");
        assert!(!stderr.contains("PRs as of") && !stderr.contains("couldn't get open PRs"), "no PR item: {stderr}");
        assert!(!stderr.contains("running this command again"), "{stderr}");

        // The choice persists without the flag.
        let output = fixture.wt_command_via(&proxy).arg("list").env("NO_COLOR", "1").output().expect("wt list");
        let stderr = collapsed(&String::from_utf8_lossy(&output.stderr));
        assert!(output.status.success(), "{stderr}");
        assert!(wait_for_refresh_workers(fixture.main(), 0, WORKER_WAIT).is_empty());
        assert_eq!(proxy.connections(), 0);
        assert!(!stderr.contains("PR #") && !stderr.contains("PRs as of"), "{stderr}");
    }

    #[test]
    #[serial]
    fn a_corrupt_file_ignores_nothing_and_is_never_overwritten() {
        let fixture = MixedFixture::new().with_github_origin();
        fs::write(preference(&fixture), b"{not json").expect("corrupt file");
        let proxy = ProxyStub::closing_after(Duration::ZERO);

        let output = fixture.wt_command_via(&proxy).arg("list").output().expect("wt list");
        assert!(output.status.success());
        assert!(wait_for_refresh_workers(fixture.main(), 0, WORKER_WAIT).is_empty());
        assert!(proxy.connections() > 0, "read as empty: the provider is asked");

        let output = fixture.wt_command_via(&proxy).arg("--ignore-api").output().expect("wt --ignore-api");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{stderr}");
        assert!(stderr.contains("cannot record the preference"), "{stderr}");
        assert!(!stderr.contains('┌'), "nothing is rendered: {stderr}");
        assert_eq!(fs::read(preference(&fixture)).expect("file"), b"{not json", "left as it was");
    }

    #[test]
    #[serial]
    fn without_an_identifiable_origin_the_flag_fails_and_records_nothing() {
        let no_origin = MixedFixture::new();
        let local = MixedFixture::new().with_origin("/srv/git/repo.git");
        for (fixture, reason) in [(&no_origin, "no origin"), (&local, "a local path")] {
            let output = fixture.wt_command().arg("--ignore-api").output().expect("wt --ignore-api");
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert_eq!(output.status.code(), Some(1), "{stderr}");
            assert!(stderr.contains("--ignore-api needs an origin"), "{stderr}");
            assert!(stderr.contains(reason), "{stderr}");
            assert!(!stderr.contains('┌'), "nothing is rendered: {stderr}");
            assert!(!preference(fixture).exists(), "no meaningless entry");
        }
    }
}

// `--ff` / `--fast-forward`.

#[test]
#[serial]
fn fast_forward_moves_a_checked_out_default_branch_after_the_update() {
    let fixture = Fixture::new();
    let pushed = fixture.commit_and_push("second");

    let caption = run(&fixture, &["--ff"]);

    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "main"]), pushed);
    assert_eq!(fs::read_to_string(fixture.main.join("file.txt")).expect("file"), "second\n", "the tree moved too");
    assert!(caption.contains("main is in sync with origin/main (updated from origin just now)"), "{caption}");
    assert!(!caption.contains("to fast-forward it"), "{caption}");
    assert!(!caption.contains("fast-forwarded"), "{caption}");
}

#[test]
#[serial]
fn fast_forward_moves_a_default_branch_no_checkout_holds() {
    let fixture = Fixture::new();
    fixture.git(&fixture.main, &["switch", "-c", "side"]);
    let pushed = fixture.commit_and_push("second");

    let caption = run(&fixture, &["--fast-forward"]);

    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "main"]), pushed);
    assert_eq!(fixture.git(&fixture.main, &["branch", "--show-current"]), "side", "no checkout changed");
    assert!(caption.contains("main is in sync with origin/main"), "{caption}");
}

#[test]
#[serial]
fn fast_forward_refuses_to_overwrite_uncommitted_changes() {
    let fixture = Fixture::new();
    let local = fixture.git(&fixture.main, &["rev-parse", "main"]);
    fixture.commit_and_push("second");
    fs::write(fixture.main.join("file.txt"), "mine\n").expect("local change");

    let caption = run(&fixture, &["--ff"]);

    assert!(
        caption.contains("main wasn't fast-forwarded: the checkout has uncommitted changes to files the update touches."),
        "{caption}"
    );
    assert_eq!(fs::read_to_string(fixture.main.join("file.txt")).expect("file"), "mine\n");
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "main"]), local);
    assert!(caption.contains("main is 1 commit behind origin/main"), "{caption}");
}

#[test]
#[serial]
fn fast_forward_refuses_a_diverged_branch_and_suggests_nothing() {
    let fixture = Fixture::new();
    fs::write(fixture.main.join("local.txt"), "local\n").expect("write");
    fixture.git(&fixture.main, &["add", "."]);
    fixture.git(&fixture.main, &["commit", "-m", "local"]);
    let local = fixture.git(&fixture.main, &["rev-parse", "main"]);
    fixture.commit_and_push("second");

    let caption = run(&fixture, &["--ff"]);
    assert!(caption.contains("main has diverged from origin/main, so it can't be fast-forwarded."), "{caption}");
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "main"]), local);

    let caption = run(&fixture, &["list"]);
    assert!(caption.contains("main has diverged from origin/main (1 commit ahead, 1 commit behind)"), "{caption}");
    assert!(!caption.contains("to fast-forward it"), "no suggestion when diverged: {caption}");
}

#[test]
#[serial]
fn fast_forward_in_sync_or_ahead_changes_nothing_and_says_nothing() {
    let fixture = Fixture::new();
    let refs = fixture.git(&fixture.main, &["for-each-ref"]);
    let caption = run(&fixture, &["--ff"]);
    assert_eq!(fixture.git(&fixture.main, &["for-each-ref"]), refs);
    assert!(!caption.contains("fast-forward"), "{caption}");

    fs::write(fixture.main.join("local.txt"), "local\n").expect("write");
    fixture.git(&fixture.main, &["add", "."]);
    fixture.git(&fixture.main, &["commit", "-m", "local"]);
    let refs = fixture.git(&fixture.main, &["for-each-ref"]);
    let caption = run(&fixture, &["--ff"]);
    assert_eq!(fixture.git(&fixture.main, &["for-each-ref"]), refs);
    assert!(caption.contains("main is 1 commit ahead of origin/main"), "{caption}");
    assert!(!caption.contains("fast-forward"), "{caption}");
}

#[test]
#[serial]
fn a_failed_check_fast_forwards_to_the_local_tracking_ref_and_keeps_its_reason() {
    let fixture = Fixture::new();
    let pushed = fixture.commit_and_push("second");
    fixture.git(&fixture.main, &["fetch", "origin"]);
    let moved = fixture.root.path().join("moved.git");
    fs::rename(&fixture.bare, &moved).expect("move origin away");

    let caption = run(&fixture, &["--ff"]);
    fs::rename(&moved, &fixture.bare).expect("move origin back");

    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "main"]), pushed, "moved to the local origin/main");
    assert!(caption.contains("main is in sync with origin/main (couldn't check origin;"), "{caption}");
}

#[test]
#[serial]
fn fast_forward_with_refresh_updates_once_and_moves_once() {
    let fixture = Fixture::new();
    let pushed = fixture.commit_and_push("second");
    let gate = UploadPackGate::install(&fixture, 99);

    let caption = run(&fixture, &["--ff", "-r"]);

    assert_eq!(gate.runs(), 2, "one check and one fetch");
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "main"]), pushed);
    assert!(fixture.git(&fixture.main, &["reflog", "-1", "--format=%gs", "main"]).contains("merge"), "one move");
    assert!(caption.contains("main is in sync with origin/main (updated from origin just now)"), "{caption}");
}
