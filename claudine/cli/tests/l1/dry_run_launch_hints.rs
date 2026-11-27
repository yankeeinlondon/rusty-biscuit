//! A sequence task's provider or model hint that reads a value an earlier
//! task's lifecycle `set` writes is known only at launch. The dry run leaves
//! the provider and the model requirement unchecked for that reason, as the
//! real run's graph decides them at launch, rather than judging the value the
//! hint holds now. A writer that sets an unrelated key leaves the hint
//! static, and both the dry run and the real non-interactive run refuse it.
//!
//! Providers are local stubs: the dry run only looks them up on `PATH`, and
//! the real-run cases use the compiled `claudine-fake-goose`, which appends
//! one line per launch to `PROVIDER_MARKER`.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::common::{CliProcessFixture, strip_ansi};

/// The first task's prompt: it runs on Goose and, on success, writes `key`.
fn writer(key: &str) -> String {
    format!("---\nagent: goose\nsuccess:\n  - set:\n      {key}: goose\n---\n\nWrite.\n")
}

/// A fixture holding `files` whose `PATH` has the fake Goose and, for each
/// of `stubs`, an executable the dry run only looks up.
fn staged(name: &str, files: &[(&str, String)], stubs: &[&str]) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fs::copy(
        biscuit_test_harness::bin_exe!("claudine-fake-goose"),
        fixture.bin_dir().join(format!("goose{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("the fake provider is built with `test-fixtures`");
    for stub in stubs {
        let path = fixture.bin_dir().join(format!("{stub}{}", std::env::consts::EXE_SUFFIX));
        fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    for (path, text) in files {
        fs::write(fixture.cwd().join(path), text).unwrap();
    }
    fixture
}

fn marker(fixture: &CliProcessFixture) -> PathBuf {
    fixture.cwd().join("provider-launched.txt")
}

fn launches(fixture: &CliProcessFixture) -> usize {
    fs::read_to_string(marker(fixture)).map_or(0, |log| log.lines().count())
}

/// The dry run's JSON envelope and exit code, with the model variables a
/// host might export removed.
fn dry_run(fixture: &CliProcessFixture) -> (Option<i32>, Value) {
    let output = fixture
        .command()
        .env("PROVIDER_MARKER", marker(fixture))
        .env_remove("OPENCODE_MODEL")
        .env_remove("MODEL")
        .args(["sequence", "seq.md", "--dry-run", "--json"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope = serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
    assert_eq!(launches(fixture), 0, "a dry run launched a provider");
    (output.status.code(), envelope)
}

/// The real non-interactive run: its exit code and plain stderr.
fn real_run(fixture: &CliProcessFixture) -> (Option<i32>, String) {
    let output = fixture
        .command()
        .env("PROVIDER_MARKER", marker(fixture))
        .args(["sequence", "seq.md"])
        .output()
        .unwrap();
    (output.status.code(), strip_ansi(&String::from_utf8_lossy(&output.stderr)))
}

/// Every row as `(code, message)`.
fn rows(envelope: &Value) -> Vec<(String, String)> {
    envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (row["code"].as_str().unwrap().to_string(), row["message"].as_str().unwrap().to_string()))
        .collect()
}

/// The provider and model rows of the task `reader`: no provider or model
/// finding, and both checks unchecked for the launch-time reason `key` reads.
fn assert_known_only_at_launch(envelope: &Value, key: &str) {
    let rows = rows(envelope);
    for (code, message) in &rows {
        assert!(
            !code.starts_with("composition.provider_") && code != "composition.model_required",
            "no provider or model verdict before launch: {code} {message}\n{rows:#?}"
        );
    }
    let reason = format!("known only at launch: the task's `{key}` reads a value a lifecycle `set` may write before it runs");
    for check in ["provider", "model requirement"] {
        assert!(
            rows.iter().any(|(code, message)| code == "dm.check.not_checked"
                && message.contains(check)
                && message.contains(&reason)),
            "`{check}` must be unchecked for the launch-time reason: {rows:#?}"
        );
    }
}

fn codes(envelope: &Value) -> Vec<String> {
    rows(envelope).into_iter().map(|(code, _)| code).collect()
}

/// R1-F4.1: the prompt's own `pick` is `nosuch`, but the earlier `success`
/// writes `pick: goose` first.
fn frontmatter_agent_files(written: &str) -> Vec<(&'static str, String)> {
    vec![
        ("seq.md", "---\nsequence:\n  - name: first\n    prompt: writer.md\n  - name: reader\n    prompt: reader.md\n---\n".into()),
        ("writer.md", writer(written)),
        ("reader.md", "---\npick: nosuch\nagent: \"{{ pick }}\"\n---\n\nRead.\n".into()),
    ]
}

#[test]
fn a_frontmatter_agent_reading_a_written_key_is_unchecked() {
    let fixture = staged("launch-hint-frontmatter", &frontmatter_agent_files("pick"), &[]);
    let (code, envelope) = dry_run(&fixture);
    assert_known_only_at_launch(&envelope, "agent");
    assert_eq!(code, Some(0), "{envelope:#}");
}

/// R1-F4.4: an unrelated write leaves `nosuch` static; the dry run and the
/// real non-interactive run both refuse it before any task starts.
#[test]
fn a_frontmatter_agent_beside_an_unrelated_write_is_refused_in_both_runs() {
    let fixture = staged("launch-hint-frontmatter-control", &frontmatter_agent_files("other"), &[]);
    let (code, envelope) = dry_run(&fixture);
    assert!(codes(&envelope).contains(&"composition.provider_unknown".to_string()), "{envelope:#}");
    assert_eq!(code, Some(2));
    let (code, stderr) = real_run(&fixture);
    assert_ne!(code, Some(0), "{stderr}");
    assert!(stderr.contains("nosuch"), "{stderr}");
    assert_eq!(launches(&fixture), 0, "the real run refuses before task one: {stderr}");
}

/// R1-F4.2: the reader's `with.agent` reaches `pick` through `choice` →
/// `mid`; only Goose is installed, which the written `pick` names.
fn chained_with_files(written: &str) -> Vec<(&'static str, String)> {
    vec![
        (
            "seq.md",
            "---\npick: claude\nmid: \"{{ pick }}\"\nchoice: \"{{ mid }}\"\nsequence:\n  - name: first\n    prompt: writer.md\n  - name: reader\n    prompt: plain.md\n    with: { agent: \"{{ choice }}\" }\n---\n".into(),
        ),
        ("writer.md", writer(written)),
        ("plain.md", "Plain.\n".into()),
    ]
}

#[test]
fn a_with_agent_through_a_chain_to_a_written_key_is_unchecked() {
    let fixture = staged("launch-hint-chain", &chained_with_files("pick"), &[]);
    let (code, envelope) = dry_run(&fixture);
    assert_known_only_at_launch(&envelope, "agent");
    assert_eq!(code, Some(0), "{envelope:#}");
}

/// The real run of the same tree launches the written Goose for both tasks.
#[test]
fn the_real_run_of_the_chain_launches_the_written_provider() {
    let fixture = staged("launch-hint-chain-real", &chained_with_files("pick"), &[]);
    let (code, stderr) = real_run(&fixture);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(launches(&fixture), 2, "{stderr}");
}

/// R1-F4.4 for the chain: an unrelated write leaves Claude static, which is
/// not installed; both runs refuse it before any task starts.
#[test]
fn a_with_agent_chain_beside_an_unrelated_write_is_refused_in_both_runs() {
    let fixture = staged("launch-hint-chain-control", &chained_with_files("other"), &[]);
    let (code, envelope) = dry_run(&fixture);
    assert!(codes(&envelope).contains(&"composition.provider_not_installed".to_string()), "{envelope:#}");
    assert_eq!(code, Some(2));
    let (code, stderr) = real_run(&fixture);
    assert_ne!(code, Some(0), "{stderr}");
    assert_eq!(launches(&fixture), 0, "the real run refuses before task one: {stderr}");
}

/// A late `model` under a settled provider: the reader has no `agent`, so
/// the sequence's OpenCode (which needs a model) is chosen, and its `model`
/// reads the written `pick`. `model` absent is the control: the requirement
/// is then a proven error.
fn late_model_files(model: &str) -> Vec<(&'static str, String)> {
    vec![
        (
            "seq.md",
            "---\nagent: opencode\nsequence:\n  - name: first\n    prompt: writer.md\n  - name: reader\n    prompt: reader.md\n---\n".into(),
        ),
        ("writer.md", writer("pick")),
        ("reader.md", format!("---\n{model}---\n\nRead.\n")),
    ]
}

#[test]
fn a_late_model_under_a_settled_provider_leaves_only_the_model_unchecked() {
    let fixture = staged("launch-hint-late-model", &late_model_files("pick: null\nmodel: \"{{ pick }}\"\n"), &["opencode"]);
    let (code, envelope) = dry_run(&fixture);
    let rows = rows(&envelope);
    assert!(!codes(&envelope).iter().any(|code| code.starts_with("composition.")), "{rows:#?}");
    let reason = "known only at launch: the task's `model` reads a value a lifecycle `set` may write before it runs";
    assert!(
        rows.iter().any(|(code, message)| code == "dm.check.not_checked"
            && message.contains("model requirement")
            && message.contains(reason)
            && !message.contains("provider")),
        "{rows:#?}"
    );
    assert_eq!(code, Some(0), "{envelope:#}");

    let control = staged("launch-hint-late-model-control", &late_model_files(""), &["opencode"]);
    let (code, envelope) = dry_run(&control);
    assert!(codes(&envelope).contains(&"composition.model_required".to_string()), "{envelope:#}");
    assert_eq!(code, Some(2));
}

/// A reader inside a referenced sequence document that names its own
/// `agent`: the reader's launch-time `agent` keeps the decision at launch,
/// with the document's `agent` its fallback.
#[test]
fn a_nested_reader_under_an_enclosing_agent_is_unchecked() {
    let files = vec![
        ("seq.md", "---\nsequence:\n  - name: first\n    prompt: writer.md\n  - name: outer\n    sequence: ./child.md\n---\n".to_string()),
        ("child.md", "---\nagent: goose\nsequence:\n  - name: reader\n    prompt: reader.md\n---\n".to_string()),
        ("writer.md", writer("pick")),
        ("reader.md", "---\npick: nosuch\nagent: \"{{ pick }}\"\n---\n\nRead.\n".to_string()),
    ];
    let fixture = staged("launch-hint-nested", &files, &[]);
    let (code, envelope) = dry_run(&fixture);
    assert_known_only_at_launch(&envelope, "agent");
    assert_eq!(code, Some(0), "{envelope:#}");
}
