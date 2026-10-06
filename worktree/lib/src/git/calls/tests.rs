use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::*;
use crate::git::{
    git_command, git_command_allow_no_match, git_command_in, git_from, git_from_bytes, git_from_bytes_allow_no_match,
    git_from_bytes_with_env, git_from_output, git_from_raw, git_rev_parse, recorder,
};
use crate::remove::test_support::TestRepo;
use crate::worktree::{gather_dirtiness, parse_worktree_list};

/// One Git process that starts and succeeds in `dir`.
fn one_call(dir: &Path) {
    git_from(dir, dir, &["rev-parse", "--git-dir"]).expect("rev-parse");
}

fn missing_dir(repo: &TestRepo) -> PathBuf {
    repo.path().join("no-such-dir")
}

#[test]
fn nested_scopes_each_include_their_descendants() {
    let repo = TestRepo::new();
    let dir = repo.path();

    let outer = CallScope::enter();
    one_call(&dir);
    let inner = CallScope::enter();
    one_call(&dir);
    one_call(&dir);
    assert_eq!(outer.calls(), 1, "an open inner scope has not added to its parent yet");
    assert_eq!(inner.finish(), 2);
    assert_eq!(outer.calls(), 3, "the parent counts each child call once");
    one_call(&dir);
    assert!(is_counting());
    assert_eq!(outer.finish(), 4);
    assert!(!is_counting(), "finishing the last scope leaves none open");
}

#[test]
fn a_dropped_scope_adds_its_count_to_the_enclosing_one() {
    let repo = TestRepo::new();
    let outer = CallScope::enter();
    {
        let _inner = CallScope::enter();
        one_call(&repo.path());
    }
    let leaked = CallScope::enter();
    one_call(&repo.path());
    std::mem::forget(leaked);
    assert_eq!(outer.finish(), 2, "a leaked inner scope is folded in, not lost");
    assert!(!is_counting());
}

#[test]
fn a_call_on_an_unrelated_thread_is_not_counted() {
    let repo = TestRepo::new();
    let dir = repo.path();

    let scope = CallScope::enter();
    one_call(&dir);
    let other_listing = std::thread::scope(|threads| {
        let unrelated = threads.spawn(|| {
            assert!(!is_counting(), "a new thread starts with no scope");
            one_call(&dir);
            one_call(&dir);
            one_call(&dir);
        });
        let own_scope = threads.spawn(|| {
            let scope = CallScope::enter();
            one_call(&dir);
            one_call(&dir);
            scope.finish()
        });
        unrelated.join().unwrap();
        own_scope.join().unwrap()
    });
    assert_eq!(other_listing, 2, "a concurrent scope on another thread counts only its own calls");
    assert_eq!(scope.finish(), 1);
}

#[test]
fn spawned_tasks_return_counts_that_add_to_the_spawner_after_joining() {
    let repo = TestRepo::new();
    let dir = repo.path();

    let scope = CallScope::enter();
    let handle = TaskHandle::current();
    let values: Vec<usize> = std::thread::scope(|threads| {
        let tasks: Vec<_> = (1..=3)
            .map(|n| {
                let dir = &dir;
                threads.spawn(move || {
                    handle.run(|| {
                        (0..n).for_each(|_| one_call(dir));
                        n
                    })
                })
            })
            .collect();
        tasks.into_iter().map(|task| joined(task.join().unwrap())).collect()
    });
    assert_eq!(values, [1, 2, 3]);
    assert_eq!(scope.finish(), 6);
}

#[test]
fn a_task_handle_run_without_spawning_counts_once() {
    let repo = TestRepo::new();
    let scope = CallScope::enter();
    let handle = TaskHandle::current();
    let ((), calls) = handle.run(|| one_call(&repo.path()));
    assert_eq!(calls, 0, "the open scope already counted the call");
    joined(((), calls));
    assert_eq!(scope.finish(), 1);
}

#[test]
fn the_library_spawn_sites_return_their_tasks_counts() {
    let repo = TestRepo::new();
    repo.add_linked_worktree("feature-a");
    repo.add_linked_worktree("feature-b");
    let entries = parse_worktree_list(&repo.git(&["worktree", "list", "--porcelain"]));
    assert_eq!(entries.len(), 3);

    let scope = CallScope::enter();
    let dirty = gather_dirtiness(&entries);
    assert_eq!(dirty.len(), 3);
    assert_eq!(scope.finish(), 3, "one `git status` per entry, each on its own thread");
}

#[test]
#[serial_test::serial]
fn a_failed_spawn_or_an_injected_failure_counts_nothing() {
    let repo = TestRepo::new();
    let missing = missing_dir(&repo);

    let scope = CallScope::enter();
    assert!(git_command_in(&missing, &["status"]).is_err());
    assert!(git_from(&missing, &repo.path(), &["status"]).is_err());
    assert!(git_from_bytes(&missing, &repo.path(), &["status"], Some(b"input")).is_err());
    assert!(git_from_output(&missing, &repo.path(), &[OsStr::new("status")]).is_err());
    assert!(crate::live_remote::run_noninteractive(&missing, &["status"], Duration::from_secs(5)).is_err());
    {
        let _failure = recorder::fail_matching(|args| args == ["--version"]);
        let error = git_command(&["--version"]).unwrap_err();
        assert!(error.to_string().contains("injected failure"), "{error}");
    }
    assert_eq!(scope.finish(), 0);
}

#[test]
fn a_process_that_exits_nonzero_is_counted() {
    let repo = TestRepo::new();
    let dir = repo.path();
    let missing_ref = ["rev-parse", "--verify", "-q", "refs/heads/no-such-branch"];

    let scope = CallScope::enter();
    assert!(git_command_in(&dir, &missing_ref).is_err());
    assert!(git_from_bytes(&dir, &dir, &missing_ref, None).is_err());
    let output = git_from_output(&dir, &dir, &missing_ref.map(OsStr::new)).unwrap();
    assert!(!output.status.success());
    assert_eq!(scope.finish(), 3);
}

#[test]
#[serial_test::serial]
fn every_helper_counts_one_start_however_it_is_wrapped() {
    let repo = TestRepo::new();
    let dir = repo.path();
    let dir = dir.as_path();
    let count = |call: &dyn Fn()| {
        let scope = CallScope::enter();
        call();
        scope.finish()
    };
    let no_match = ["merge-base", "--is-ancestor", "HEAD", "HEAD"];

    let helpers: [(&str, &dyn Fn()); 10] = [
        ("git_command", &|| drop(git_command(&["--version"]))),
        ("git_command_allow_no_match", &|| drop(git_command_allow_no_match(&["--version"]))),
        ("git_rev_parse", &|| drop(git_rev_parse("--git-dir"))),
        ("git_command_in", &|| drop(git_command_in(dir, &["rev-parse", "HEAD"]))),
        ("git_from", &|| drop(git_from(dir, dir, &["rev-parse", "HEAD"]))),
        ("git_from_raw", &|| drop(git_from_raw(dir, dir, &["ls-files", "-z"]))),
        ("git_from_bytes", &|| drop(git_from_bytes(dir, dir, &["hash-object", "--stdin"], Some(b"x")))),
        ("git_from_bytes_with_env", &|| {
            drop(git_from_bytes_with_env(dir, dir, &["ls-files"], &[("GIT_TERMINAL_PROMPT", OsStr::new("0"))]))
        }),
        ("git_from_bytes_allow_no_match", &|| drop(git_from_bytes_allow_no_match(dir, dir, &no_match, None))),
        ("git_from_output", &|| drop(git_from_output(dir, dir, &[OsStr::new("status")]))),
    ];
    for (name, helper) in helpers {
        assert_eq!(count(helper), 1, "{name}");
    }
}

#[test]
#[serial_test::serial]
fn a_fast_forward_counts_every_git_process_it_starts() {
    let repo = TestRepo::with_origin();
    repo.push_commit_to_origin("main", "remote.txt");
    repo.git(&["fetch", "-q", "origin"]);

    recorder::start_recording();
    let scope = CallScope::enter();
    let result = crate::fast_forward::fast_forward_default(&repo.path(), "main");
    let counted = scope.finish();
    let recorded = recorder::finish_recording();

    assert!(matches!(result, crate::fast_forward::FfResult::Moved { .. }), "{result:?}");
    assert!(recorder::has_any_matching(&recorded, |args| args.first().map(String::as_str) == Some("merge")));
    assert_eq!(counted, recorded.len() as u64, "every recorded call started: {recorded:?}");
}

/// The transport is spawned outside the recorded helpers, after one recorded
/// `config` read for the SSH command, so it counts one more than the recorder.
#[test]
#[serial_test::serial]
fn the_transport_counts_its_own_process_and_its_config_read() {
    let repo = TestRepo::new();
    let dir = repo.path();

    for (args, succeeds) in [(["rev-parse", "HEAD"].as_slice(), true), (&["rev-parse", "--verify", "-q", "refs/heads/nope"], false)] {
        recorder::start_recording();
        let scope = CallScope::enter();
        let result = crate::live_remote::run_noninteractive(&dir, args, Duration::from_secs(5));
        let counted = scope.finish();
        let recorded = recorder::finish_recording();

        assert_eq!(result.is_ok(), succeeds, "{result:?}");
        assert!(!recorded.iter().any(|call| call.first().map(String::as_str) == Some("rev-parse")));
        assert_eq!(counted, recorded.len() as u64 + 1, "{args:?}: {recorded:?}");
    }
}

#[test]
fn with_no_scope_open_nothing_is_counted() {
    let repo = TestRepo::new();
    assert!(!is_counting());
    let handle = TaskHandle::current();
    one_call(&repo.path());
    add_joined(5);
    let ((), task_calls) = std::thread::scope(|threads| {
        threads.spawn(|| handle.run(|| one_call(&repo.path()))).join().unwrap()
    });
    assert_eq!(task_calls, 0, "a task spawned while not counting counts nothing");

    let scope = CallScope::enter();
    assert_eq!(scope.finish(), 0, "nothing from before the scope leaks into it");
}

#[test]
#[serial_test::serial]
fn the_recorder_still_logs_every_requested_call_while_counting() {
    let repo = TestRepo::new();
    let missing = missing_dir(&repo);

    recorder::start_recording();
    let scope = CallScope::enter();
    git_command(&["--version"]).unwrap();
    {
        let _failure = recorder::fail_matching(|args| args == ["--version"]);
        assert!(git_command(&["--version"]).is_err());
    }
    assert!(git_command_in(&missing, &["status"]).is_err());
    let counted = scope.finish();
    let recorded = recorder::finish_recording();

    assert_eq!(counted, 1, "only the call that started Git counts");
    assert_eq!(
        recorded,
        [vec!["--version"], vec!["--version"], vec!["status"]]
            .map(|call| call.into_iter().map(String::from).collect::<Vec<_>>()),
        "the recorder logs injected failures and failed spawns as before"
    );
}
