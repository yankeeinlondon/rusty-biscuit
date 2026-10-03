//! A sequence step launches its planned model when the document it runs names
//! none.
//!
//! The sequence plans one target per step before any step runs: from the
//! review screen, or, as here, from the sequence document's own `agent` and
//! `model`. Each launch then rebuilds its identity from the document it runs,
//! with `--model` first, the launched document's own `model:` next, and the
//! planned model as the fallback for a document that names none. These tests
//! run the shipped binary against a recording `claude` stub and read the
//! model each step's launch received.
//!
//! The review screen's own choices reach the same fallback; they are asserted
//! in a real terminal by `level2_sequence_review_screen`.

use std::fs;

use crate::common::{CliProcessFixture, write, write_executable};

/// Records each launch's `--model` argument, `MODEL`, and stdin, one
/// directory per launch.
fn install_recording_claude(fixture: &CliProcessFixture) {
    let record = fixture.workspace_path().join("record");
    fs::create_dir_all(&record).unwrap();
    write_executable(
        &fixture.bin_dir().join("claude"),
        &format!(
            "#!/bin/sh\n\
             d=$(/usr/bin/mktemp -d '{record}/launch.XXXXXX')\n\
             if [ -n \"${{MODEL+x}}\" ]; then printf '%s' \"$MODEL\" > \"$d/model_env\"; fi\n\
             prev=''\n\
             for arg in \"$@\"; do\n\
             \x20 if [ \"$prev\" = '--model' ]; then printf '%s' \"$arg\" > \"$d/model_arg\"; fi\n\
             \x20 prev=\"$arg\"\n\
             done\n\
             /bin/cat > \"$d/stdin\"\n\
             printf '%s\\n' '{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"done\",\"session_id\":\"s\"}}'\n\
             exit 0\n",
            record = record.display(),
        ),
    );
}

/// `(step token, --model, MODEL)` for every launch, sorted by token.
fn launches(fixture: &CliProcessFixture) -> Vec<(String, Option<String>, Option<String>)> {
    let mut launches: Vec<_> = fs::read_dir(fixture.workspace_path().join("record"))
        .expect("record dir")
        .map(|entry| {
            let dir = entry.expect("record entry").path();
            let read = |name: &str| fs::read_to_string(dir.join(name)).ok();
            let stdin = read("stdin").unwrap_or_default();
            let token = ["BODY-STEP", "OWN-MODEL-STEP", "NO-MODEL-STEP"]
                .into_iter()
                .find(|token| stdin.contains(token))
                .unwrap_or_else(|| panic!("a launch carried no step token: {stdin}"))
                .to_string();
            (token, read("model_arg"), read("model_env"))
        })
        .collect();
    launches.sort();
    launches
}

fn stage(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fixture.seed_user_config();
    install_recording_claude(&fixture);
    let cwd = fixture.cwd();
    write(&cwd.join("no-model.md"), "Token NO-MODEL-STEP.\n");
    write(
        &cwd.join("own-model.md"),
        "---\nmodel: own-model\n---\nToken OWN-MODEL-STEP.\n",
    );
    write(
        &cwd.join("seq.md"),
        "---\n\
         agent: claude\n\
         model: planned-model\n\
         sequence:\n\
         \x20   - name: no-model\n\
         \x20     prompt: \"./no-model.md\"\n\
         \x20   - name: own-model\n\
         \x20     prompt: \"./own-model.md\"\n\
         \x20   - name: body\n\
         ---\n\
         Token BODY-STEP.\n",
    );
    fixture
}

fn some(value: &str) -> Option<String> {
    Some(value.to_string())
}

#[test]
fn a_prompt_step_without_a_model_launches_the_planned_model() {
    let fixture = stage("sequence-planned-model");
    let output = fixture.command().args(["sequence", "seq.md"]).output().unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(0), "{text}");
    assert_eq!(
        launches(&fixture),
        vec![
            ("BODY-STEP".into(), some("planned-model"), some("planned-model")),
            ("NO-MODEL-STEP".into(), some("planned-model"), some("planned-model")),
            ("OWN-MODEL-STEP".into(), some("own-model"), some("own-model")),
        ],
        "{text}"
    );
    assert!(!fixture.audio_spool().exists(), "lifecycle audio was published");
}

#[test]
fn an_explicit_model_outranks_every_document_and_the_plan() {
    let fixture = stage("sequence-planned-model-cli");
    let output = fixture
        .command()
        .args(["sequence", "--claude", "--model", "cli-model", "seq.md"])
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.status.code(), Some(0), "{text}");
    assert_eq!(
        launches(&fixture),
        vec![
            ("BODY-STEP".into(), some("cli-model"), some("cli-model")),
            ("NO-MODEL-STEP".into(), some("cli-model"), some("cli-model")),
            ("OWN-MODEL-STEP".into(), some("cli-model"), some("cli-model")),
        ],
        "{text}"
    );
}
