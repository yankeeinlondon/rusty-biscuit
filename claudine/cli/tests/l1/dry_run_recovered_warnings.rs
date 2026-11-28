//! A preview recovered past a missing inclusion keeps the warnings its
//! preparation reports.
//!
//! The prompt includes `./first.md`, then `./child.md`; the child tolerates
//! its own missing `./other.md` (`ignore_invalid: true`), which a real run
//! prints as a warning. With `first.md` missing, the first preparation fails
//! and the preview is retried past that inclusion: the retry's warning is
//! still a row, beside the one refusal of `first.md`. Created, `first.md`
//! leaves the warning alone; with `other.md` created too, the check is clean.
//! Each route (direct compose, inline compose, a sequence task naming either
//! document, and a sequence body step) runs every state as its own test.

use std::fs;

use serde_json::Value;

use crate::common::CliProcessFixture;

const REFERENCE: &str = "dm.reference.missing_transclusion";
const NO_EXECUTABLE: &str = "composition.no_executable";

/// The prompt body: two inclusions, the first independent of the child's.
const BODY: &str = "Intro.\n\n::file ./first.md\n\n::file ./child.md\n";
/// The tolerant child, whose own inclusion is missing until created.
const CHILD: &str = "---\ntitle: c\nignore_invalid: true\n---\nChild\n\n::file ./other.md\n";

/// Which inclusions exist.
#[derive(Clone, Copy)]
enum State {
    /// Neither `first.md` nor `other.md`.
    BothMissing,
    /// `first.md` only.
    FirstCreated,
    /// Both.
    AllCreated,
}

/// The documents a route writes and the command it checks.
#[derive(Clone, Copy)]
enum Route {
    Compose,
    InlineCompose,
    TaskPrompt,
    TaskInlinePrompt,
    BodyStep,
}

fn direct(body: &str) -> String {
    format!("---\ntitle: x\nagent: claude\n---\n{body}")
}

fn inline(body: &str) -> String {
    format!("---\ntitle: x\nagent: claude\nprompt: {}\n---\n", serde_json::to_string(body).unwrap())
}

/// A fixture with a `claude` on its `PATH` that the check only looks up.
fn fixture(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    let path = fixture.bin_dir().join(format!("claude{}", std::env::consts::EXE_SUFFIX));
    fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fixture
}

/// Writes `route`'s documents in `state` and returns the checked command.
fn arrange(fixture: &CliProcessFixture, route: Route, state: State) -> Vec<&'static str> {
    let cwd = fixture.cwd();
    fs::write(cwd.join("child.md"), CHILD).unwrap();
    if matches!(state, State::FirstCreated | State::AllCreated) {
        fs::write(cwd.join("first.md"), "First.\n").unwrap();
    }
    if matches!(state, State::AllCreated) {
        fs::write(cwd.join("other.md"), "Other.\n").unwrap();
    }
    let task = |prompt: &str| format!("---\nagent: claude\nsequence:\n  - name: write\n    prompt: {prompt}\n---\n");
    match route {
        Route::Compose => {
            fs::write(cwd.join("doc.md"), direct(BODY)).unwrap();
            vec!["compose", "doc.md"]
        }
        Route::InlineCompose => {
            fs::write(cwd.join("notes.md"), inline(BODY)).unwrap();
            vec!["inline-compose", "notes.md"]
        }
        Route::TaskPrompt => {
            fs::write(cwd.join("doc.md"), direct(BODY)).unwrap();
            fs::write(cwd.join("seq.md"), task("./doc.md")).unwrap();
            vec!["sequence", "seq.md"]
        }
        Route::TaskInlinePrompt => {
            fs::write(cwd.join("notes.md"), inline(BODY)).unwrap();
            fs::write(cwd.join("seq.md"), task("./notes.md")).unwrap();
            vec!["sequence", "seq.md"]
        }
        Route::BodyStep => {
            fs::write(cwd.join("seq.md"), format!("---\nagent: claude\nsequence:\n  - name: write\n---\n{BODY}")).unwrap();
            vec!["sequence", "seq.md"]
        }
    }
}

/// The exit status and the `(code, severity)` rows of a `--json` dry run.
fn check(fixture: &CliProcessFixture, args: &[&str]) -> (Option<i32>, Vec<(String, String)>) {
    let output = fixture.command().args(args).args(["--dry-run", "--json"]).output().unwrap();
    assert!(!fixture.audio_spool().exists());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| panic!("not JSON:\n{stdout}"));
    let rows = report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (row["code"].as_str().unwrap().to_string(), row["severity"].as_str().unwrap().to_string()))
        .collect();
    (output.status.code(), rows)
}

/// The rows `route` reports in `state`, in report order.
fn expected(route: Route, state: State) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    if matches!(route, Route::BodyStep) {
        rows.push((NO_EXECUTABLE, "info"));
    }
    match state {
        State::BothMissing => rows.extend([(REFERENCE, "error"), (REFERENCE, "warning")]),
        State::FirstCreated => rows.push((REFERENCE, "warning")),
        State::AllCreated => {}
    }
    rows.into_iter().map(|(code, severity)| (code.to_string(), severity.to_string())).collect()
}

macro_rules! cases {
    ($($name:ident: $route:ident, $state:ident;)*) => {$(
        #[test]
        fn $name() {
            let fixture = fixture(concat!("dry-run-recovered-", stringify!($name)));
            let args = arrange(&fixture, Route::$route, State::$state);

            let (code, rows) = check(&fixture, &args);

            let expected = expected(Route::$route, State::$state);
            let problems = expected.iter().any(|(_, severity)| severity != "info");
            assert_eq!(code, Some(if problems { 2 } else { 0 }), "{rows:?}");
            assert_eq!(rows, expected);
        }
    )*};
}

cases! {
    compose_both_missing: Compose, BothMissing;
    compose_first_created: Compose, FirstCreated;
    compose_all_created: Compose, AllCreated;
    inline_compose_both_missing: InlineCompose, BothMissing;
    inline_compose_first_created: InlineCompose, FirstCreated;
    inline_compose_all_created: InlineCompose, AllCreated;
    task_prompt_both_missing: TaskPrompt, BothMissing;
    task_prompt_first_created: TaskPrompt, FirstCreated;
    task_prompt_all_created: TaskPrompt, AllCreated;
    task_inline_prompt_both_missing: TaskInlinePrompt, BothMissing;
    task_inline_prompt_first_created: TaskInlinePrompt, FirstCreated;
    task_inline_prompt_all_created: TaskInlinePrompt, AllCreated;
    body_step_both_missing: BodyStep, BothMissing;
    body_step_first_created: BodyStep, FirstCreated;
    body_step_all_created: BodyStep, AllCreated;
}
