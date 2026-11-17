//! A wrapper's options mean the same wherever they are written before the
//! first `--`, and nothing after that `--` is Claudine's. Each provider is a
//! recording shell stub, so the file is Unix-only; the behavior under test is
//! platform-neutral argv handling.
#![cfg(unix)]

use crate::common;
use common::launch_recorder::{install as stub, launches, occurrences};
use common::{CliProcessFixture, strip_ansi};

fn fixture(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fixture.seed_user_config();
    fixture
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> (i32, String) {
    let output = fixture.command().args(args).output().unwrap();
    (output.status.code().unwrap_or(-1), strip_ansi(&String::from_utf8_lossy(&output.stderr)))
}

/// `-m` after the prompt is Claudine's model option in every spelling,
/// rendered in the provider's own spelling, never forwarded as written.
#[test]
fn a_value_option_after_the_prompt_is_applied_by_claudine() {
    for (index, spelling) in [&["-m", "opus"][..], &["-mopus"], &["--model=opus"], &["-ymopus"]].into_iter().enumerate() {
        let fixture = fixture(&format!("wrapper-option-after-prompt-{index}"));
        let log = stub(&fixture, "claude", "exit 0");

        let (_, stderr) = run(&fixture, &[&["claude", "hi"][..], spelling].concat());

        let launch = launches(&log);
        assert_eq!(launch.len(), 1, "{spelling:?}: {stderr}");
        assert_eq!(occurrences(&launch[0], &["--model", "opus"]), 1, "{spelling:?}: {:?}", launch[0]);
        assert!(!launch[0].iter().any(|arg| arg.starts_with("-m") || arg.starts_with("-y")), "{spelling:?}: {:?}", launch[0]);
    }
}

/// `--dry-run` after the prompt is Claudine's: nothing launches.
#[test]
fn dry_run_after_the_prompt_launches_nothing() {
    let fixture = fixture("wrapper-dry-run-after-prompt");
    let log = stub(&fixture, "claude", "exit 0");

    let (code, stderr) = run(&fixture, &["claude", "hi", "--dry-run"]);

    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.contains("[DRY RUN]"), "{stderr}");
    assert!(launches(&log).is_empty());
}

/// After the first `--`, a Claudine-looking option is the provider's and
/// reaches it verbatim, in order.
#[test]
fn options_after_the_separator_reach_the_provider_verbatim() {
    let fixture = fixture("wrapper-options-after-separator");
    let log = stub(&fixture, "claude", "exit 0");

    let (code, stderr) = run(&fixture, &["claude", "hi", "--", "--output", "json", "--help", "-m", "x"]);

    assert_ne!(code, 2, "nothing after `--` is refused: {stderr}");
    let launch = launches(&log);
    assert_eq!(launch.len(), 1, "{stderr}");
    assert_eq!(occurrences(&launch[0], &["--output", "json", "--help", "-m", "x"]), 1, "{:?}", launch[0]);
}
