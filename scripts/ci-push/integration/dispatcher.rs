//! The dispatcher Git runs from the shared hooks directory (parity row D1):
//! each linked worktree's push runs that worktree's own checked-out hook,
//! with Git's arguments, stdin, and the hook's exit status intact.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use crate::launcher::{install_hook, real_git, windowless, HookRepository, HOOK};

/// A hook that records its label, argument count, arguments, and stdin
/// beside itself, then exits with a status of its own. A `sh` script is
/// what Git and the dispatcher run on every OS, so it needs no compiled
/// stand-in.
const RECORDER: &str = r#"#!/bin/sh
{ printf '%s\n' "__LABEL__" "$#" "$@"; cat; } >"$0.ran"
exit __STATUS__
"#;

fn recorder(label: &str, status: u8) -> String {
    RECORDER
        .replace("__LABEL__", label)
        .replace("__STATUS__", &status.to_string())
}

/// Runs the installed dispatcher the way Git runs a hook, from `worktree`.
fn dispatch(repository: &HookRepository, worktree: &Path, args: &[&str], stdin: &[u8]) -> Output {
    #[cfg(unix)]
    let mut command = Command::new(repository.dispatcher());
    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new(crate::launcher::git_sh());
        command.arg(repository.dispatcher());
        command
    };
    let path = repository.hermetic_path(&[]);
    repository.isolate(&mut command, &path);
    let mut child = windowless(command.args(args).current_dir(worktree))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| {
            panic!(
                "start the dispatcher with {}: {error}",
                real_git().display()
            )
        });
    child.stdin.take().unwrap().write_all(stdin).unwrap();
    child.wait_with_output().unwrap()
}

fn record(worktree: &Path) -> String {
    let file = worktree.join(format!("{HOOK}.ran"));
    std::fs::read_to_string(&file)
        .unwrap_or_else(|error| panic!("read {}: {error}", file.display()))
}

#[test]
fn the_dispatcher_runs_each_linked_worktrees_own_hook() {
    let repository =
        HookRepository::new(include_str!("../../../.githooks/pre-push-dispatcher"), &[]);
    let linked = repository.add_linked();
    install_hook(repository.main(), &recorder("main", 0));
    install_hook(&linked, &recorder("linked", 7));

    let main_run = dispatch(
        &repository,
        repository.main(),
        &["origin", "main-url"],
        b"main-stdin\n",
    );
    let linked_run = dispatch(
        &repository,
        &linked.join("sub"),
        &["upstream", "linked url"],
        b"linked-stdin\n",
    );

    assert_eq!(
        main_run.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&main_run.stderr)
    );
    assert_eq!(
        record(repository.main()),
        "main\n2\norigin\nmain-url\nmain-stdin\n"
    );
    assert_eq!(
        linked_run.status.code(),
        Some(7),
        "{}",
        String::from_utf8_lossy(&linked_run.stderr)
    );
    assert_eq!(
        record(&linked),
        "linked\n2\nupstream\nlinked url\nlinked-stdin\n"
    );
}
