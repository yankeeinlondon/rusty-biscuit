//! A broken inclusion under `--dry-run` is reported whether or not the
//! prompt's preview composes.
//!
//! The check walks each prompt's inclusions condition-blind, as a real run's
//! pre-flight does, apart from the preview, so a preview that fails first
//! (`{{ 3 / 0 }}` fails body interpolation before transclusion runs) still
//! leaves the missing `::file` reported, and the preview's failure stays its
//! own finding. One test per route: direct compose, inline compose, a
//! sequence prompt task, plus the excluded-block rule. Each also runs a
//! control that repairs one problem and keeps only the other's finding, the
//! reference's held once.
//!
//! An inclusion whose target reads a value the dry run has not got (a
//! missing input, a `$( … )` value, an effectful function's result) names no
//! path: it is `not checked`, never a parse failure or a missing file. And
//! the walk follows composition's rule for tolerating an invalid inclusion:
//! the document that writes it decides, a transcluded child included.

use std::fs;

use serde_json::Value;

use crate::common::CliProcessFixture;

const REFERENCE: &str = "dm.reference.missing_transclusion";
const EVALUATION: &str = "dm.expression.evaluation_failure";
const MISSING_INPUT: &str = "composition.missing_properties";
const NOT_CHECKED: &str = "dm.check.not_checked";

/// The reproduction body: an intro, a broken `::file`, and a failing
/// expression after it.
const BODY: &str = "Intro.\n\n::file ./missing.md\n\nValue {{ 3 / 0 }}\n";

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

/// The exit status and the finding codes of a `--json` dry run.
fn check(fixture: &CliProcessFixture, args: &[&str]) -> (Option<i32>, Vec<String>) {
    let (code, rows) = check_rows(fixture, args);
    (code, rows.iter().map(|row| row["code"].as_str().unwrap().to_string()).collect())
}

/// The exit status and the finding rows of a `--json` dry run.
fn check_rows(fixture: &CliProcessFixture, args: &[&str]) -> (Option<i32>, Vec<Value>) {
    let output = fixture.command().args(args).args(["--dry-run", "--json"]).output().unwrap();
    assert!(!fixture.audio_spool().exists());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| panic!("not JSON:\n{stdout}"));
    (output.status.code(), report["diagnostics"].as_array().unwrap().clone())
}

/// `rows` as `(code, severity)` pairs, with a `not checked` row's message.
fn summary(rows: &[Value]) -> Vec<(String, String)> {
    rows.iter()
        .map(|row| {
            let code = row["code"].as_str().unwrap().to_string();
            let detail = if code == NOT_CHECKED { row["message"].as_str() } else { row["severity"].as_str() };
            (code, detail.unwrap().to_string())
        })
        .collect()
}

fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected.iter().map(|(code, detail)| ((*code).to_string(), (*detail).to_string())).collect()
}

/// Direct compose: both findings; with the expression repaired, the
/// reference alone, once.
#[test]
fn compose_reports_the_reference_beside_a_failed_preview() {
    let fixture = fixture("dry-run-refs-compose");
    let doc = |body: &str| format!("---\ntitle: x\nagent: claude\n---\n{body}");
    fs::write(fixture.cwd().join("doc.md"), doc(BODY)).unwrap();

    let (code, codes) = check(&fixture, &["compose", "doc.md"]);
    assert_eq!(code, Some(2), "{codes:?}");
    assert_eq!(codes, [REFERENCE, EVALUATION]);

    fs::write(fixture.cwd().join("doc.md"), doc(&BODY.replace("{{ 3 / 0 }}", "{{ 3 / 1 }}"))).unwrap();
    let (code, codes) = check(&fixture, &["compose", "doc.md"]);
    assert_eq!(code, Some(2), "{codes:?}");
    assert_eq!(codes, [REFERENCE]);
}

/// Inline compose: the delivered `prompt:` is walked; with the included
/// file created, only the evaluation failure remains.
#[test]
fn inline_compose_reports_the_reference_beside_a_failed_preview() {
    let fixture = fixture("dry-run-refs-inline");
    let prompt = serde_json::to_string(BODY).unwrap();
    fs::write(fixture.cwd().join("notes.md"), format!("---\ntitle: x\nagent: claude\nprompt: {prompt}\n---\n")).unwrap();

    let (code, rows) = check_rows(&fixture, &["inline-compose", "notes.md"]);
    let codes: Vec<&str> = rows.iter().map(|row| row["code"].as_str().unwrap()).collect();
    assert_eq!(code, Some(2), "{codes:?}");
    // The input check reads `prompt:` as a frontmatter value and the preview
    // composes it as the body: one failure, kept where it is written.
    assert_eq!(codes, [EVALUATION, REFERENCE]);
    assert_eq!(rows[0]["source"]["line"], 4, "{rows:?}");

    fs::write(fixture.cwd().join("missing.md"), "Included.\n").unwrap();
    let (code, codes) = check(&fixture, &["inline-compose", "notes.md"]);
    assert_eq!(code, Some(2), "{codes:?}");
    assert_eq!(codes, [EVALUATION]);
}

/// A sequence prompt task: both findings under the task; with the
/// expression repaired, the reference alone, once.
#[test]
fn a_prompt_task_reports_the_reference_beside_a_failed_preview() {
    let fixture = fixture("dry-run-refs-prompt-task");
    fs::write(fixture.cwd().join("seq.md"), "---\nagent: claude\nsequence:\n  - name: write\n    prompt: ./task.md\n---\n")
        .unwrap();
    fs::write(fixture.cwd().join("task.md"), format!("---\ntitle: x\n---\n{BODY}")).unwrap();

    let (code, codes) = check(&fixture, &["sequence", "seq.md"]);
    assert_eq!(code, Some(2), "{codes:?}");
    assert_eq!(codes, [REFERENCE, EVALUATION]);

    fs::write(fixture.cwd().join("task.md"), format!("---\ntitle: x\n---\n{}", BODY.replace("{{ 3 / 0 }}", "3"))).unwrap();
    let (code, codes) = check(&fixture, &["sequence", "seq.md"]);
    assert_eq!(code, Some(2), "{codes:?}");
    assert_eq!(codes, [REFERENCE]);
}

/// A broken inclusion inside a `::block` a false condition leaves out is
/// still an error, as the real run's condition-blind pre-flight refuses it;
/// created, the document checks clean.
#[test]
fn an_inclusion_in_an_excluded_block_keeps_the_real_runs_rule() {
    let fixture = fixture("dry-run-refs-excluded-block");
    let doc = "---\ntitle: x\nagent: claude\n---\nIntro.\n\n::block when=\"false\"\n::file ./missing.md\n::end-block\n\nok\n";
    fs::write(fixture.cwd().join("doc.md"), doc).unwrap();

    let (code, codes) = check(&fixture, &["compose", "doc.md"]);
    assert_eq!(code, Some(2), "{codes:?}");
    assert_eq!(codes, [REFERENCE]);

    fs::write(fixture.cwd().join("missing.md"), "Included.\n").unwrap();
    let (code, codes) = check(&fixture, &["compose", "doc.md"]);
    assert_eq!(code, Some(0), "{codes:?}");
    assert!(codes.is_empty(), "{codes:?}");
}

/// The `not checked` row of an inclusion whose target reads `topic`.
const TARGET_READS_TOPIC: &str = "not checked: transclusion (reads `topic`, which has no value)";

/// A body `::file`/`::code` whose target reads a missing input: the input is
/// the one finding and the inclusions are not checked, never a parse failure
/// or a missing file. Supplied, the target resolves and the walk checks it.
#[test]
fn a_directive_target_reading_a_missing_input_is_not_checked() {
    let fixture = fixture("dry-run-refs-missing-target");
    let doc = "---\ntitle: x\nagent: claude\n$schema:\n  topic: string(required)\n---\nIntro.\n\n::file ./{{ topic }}.md\n\n::code ./{{topic}}.rs\n";
    fs::write(fixture.cwd().join("doc.md"), doc).unwrap();

    let (code, rows) = check_rows(&fixture, &["compose", "doc.md"]);
    assert_eq!(code, Some(2), "{rows:?}");
    assert_eq!(summary(&rows), pairs(&[(NOT_CHECKED, TARGET_READS_TOPIC), (MISSING_INPUT, "error")]));

    let (code, codes) = check(&fixture, &["compose", "doc.md", "topic=zz"]);
    assert_eq!(code, Some(2), "{codes:?}");
    assert_eq!(codes, [REFERENCE, REFERENCE]);
}

/// The same for `prologue:` and `epilogue:` values; supplied and present,
/// the document checks clean.
#[test]
fn a_prologue_target_reading_a_missing_input_is_not_checked() {
    let fixture = fixture("dry-run-refs-missing-prologue");
    let doc = "---\ntitle: x\nagent: claude\nprologue: ./{{ topic }}.md\nepilogue: ./{{topic}}.md\n$schema:\n  topic: string(required)\n---\nIntro.\n";
    fs::write(fixture.cwd().join("doc.md"), doc).unwrap();
    fs::write(fixture.cwd().join("a.md"), "Included.\n").unwrap();

    let (code, rows) = check_rows(&fixture, &["compose", "doc.md"]);
    assert_eq!(code, Some(2), "{rows:?}");
    assert_eq!(summary(&rows), pairs(&[(NOT_CHECKED, TARGET_READS_TOPIC), (MISSING_INPUT, "error")]));

    let (code, codes) = check(&fixture, &["compose", "doc.md", "topic=a"]);
    assert_eq!((code, codes), (Some(0), Vec::<String>::new()));
}

/// A directive target that reads a `$( … )` value names a command whose
/// shape the approval discovery cannot know, so a real run refuses it
/// before anything runs, whatever the command prints, and the dry run
/// reports that refusal; a literal value is checked, its missing file
/// reported.
#[test]
fn a_directive_target_reading_a_command_value_is_the_discovery_refusal() {
    let fixture = fixture("dry-run-refs-command-target");
    let doc = |topic: &str| format!("---\ntitle: x\nagent: claude\ntopic: {topic}\n---\nIntro.\n\n::file ./{{{{ topic }}}}.md\n");
    fs::write(fixture.cwd().join("doc.md"), doc("$(echo a)")).unwrap();
    fs::write(fixture.cwd().join("a.md"), "Included.\n").unwrap();

    let (code, rows) = check_rows(&fixture, &["compose", "doc.md"]);
    assert_eq!(code, Some(2), "{rows:?}");
    let errors: Vec<&Value> = rows.iter().filter(|row| row["severity"] == "error").collect();
    assert_eq!(errors.len(), 1, "{rows:?}");
    assert_eq!(errors[0]["code"], "dm.shell.failure", "{rows:?}");
    assert!(errors[0]["message"].as_str().unwrap().starts_with("dynamic command shape"), "{rows:?}");
    let real = fixture.command().args(["compose", "doc.md"]).output().unwrap();
    assert_eq!(real.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&real.stderr).contains("dynamic command shape"));

    fs::remove_file(fixture.cwd().join("a.md")).unwrap();
    fs::write(fixture.cwd().join("doc.md"), doc("a")).unwrap();
    let (code, codes) = check(&fixture, &["compose", "doc.md"]);
    assert_eq!((code, codes), (Some(2), vec![REFERENCE.to_string()]));
}

/// A sequence task's `with:` value that calls an effectful function is not
/// known in a dry run, so its prompt's inclusion that reads it is not
/// checked; a literal value is checked.
#[test]
fn a_directive_target_reading_an_effectful_with_value_is_not_checked() {
    let fixture = fixture("dry-run-refs-effectful-target");
    let seq = |topic: &str| format!("---\nagent: claude\nsequence:\n  - name: write\n    prompt: ./task.md\n    with:\n      topic: \"{topic}\"\n---\n");
    fs::write(fixture.cwd().join("seq.md"), seq("{{ has_alias('zz') }}")).unwrap();
    fs::write(fixture.cwd().join("task.md"), "---\ntitle: t\n$schema:\n  topic: string(required)\n---\nT\n\n::file ./{{ topic }}.md\n").unwrap();

    let (code, rows) = check_rows(&fixture, &["sequence", "seq.md"]);
    assert_eq!(code, Some(0), "{rows:?}");
    let codes: Vec<&str> = rows.iter().map(|row| row["code"].as_str().unwrap()).collect();
    assert!(codes.iter().all(|held| *held == NOT_CHECKED), "{rows:?}");
    assert!(rows.iter().any(|row| row["message"].as_str().unwrap().starts_with("not checked: transclusion (reads `topic`")), "{rows:?}");

    fs::write(fixture.cwd().join("seq.md"), seq("zz")).unwrap();
    let (code, codes) = check(&fixture, &["sequence", "seq.md"]);
    assert_eq!((code, codes), (Some(2), vec![REFERENCE.to_string()]));
}

/// A sequence whose one task runs `./task.md` with `topic` as its `with:`.
fn effectful_task(topic: &str) -> String {
    format!("---\nagent: claude\nsequence:\n  - name: write\n    prompt: ./task.md\n    with:\n      topic: \"{topic}\"\n---\n")
}

const EFFECTFUL: &str = "{{ has_alias('zz') }}";

/// The approval discovery a real run makes of `task.md` before any task
/// reads the `with:` value as it is then; the dry run has no value for an
/// effectful one, so a `prologue:` naming a file by it names no file there
/// either. A literal value names a file both runs find missing.
#[test]
fn a_prologue_target_reading_an_effectful_with_value_is_not_checked() {
    let fixture = fixture("dry-run-refs-effectful-prologue");
    fs::write(fixture.cwd().join("seq.md"), effectful_task(EFFECTFUL)).unwrap();
    fs::write(fixture.cwd().join("task.md"), "---\n$schema:\n  topic: string(required)\nprologue: \"./{{ topic }}.md\"\n---\nT\n").unwrap();

    let (code, rows) = check_rows(&fixture, &["sequence", "seq.md"]);
    assert_eq!(code, Some(0), "{rows:?}");
    assert!(rows.iter().all(|row| row["code"] == NOT_CHECKED), "{rows:?}");
    assert!(rows.iter().any(|row| row["message"].as_str().unwrap().starts_with("not checked: transclusion (reads `topic`")), "{rows:?}");

    fs::write(fixture.cwd().join("seq.md"), effectful_task("zz")).unwrap();
    let (code, codes) = check(&fixture, &["sequence", "seq.md"]);
    assert_eq!((code, codes), (Some(2), vec![REFERENCE.to_string()]));
}

/// `::code` shares the discovery walk's directive reading with `::file`.
#[test]
fn a_code_target_reading_an_effectful_with_value_is_not_checked() {
    let fixture = fixture("dry-run-refs-effectful-code");
    fs::write(fixture.cwd().join("seq.md"), effectful_task(EFFECTFUL)).unwrap();
    fs::write(fixture.cwd().join("task.md"), "---\n$schema:\n  topic: string(required)\n---\nT\n\n::code ./{{ topic }}.md\n").unwrap();

    let (code, rows) = check_rows(&fixture, &["sequence", "seq.md"]);
    assert_eq!(code, Some(0), "{rows:?}");
    assert!(rows.iter().all(|row| row["code"] == NOT_CHECKED), "{rows:?}");

    fs::write(fixture.cwd().join("seq.md"), effectful_task("zz")).unwrap();
    let (code, codes) = check(&fixture, &["sequence", "seq.md"]);
    assert_eq!((code, codes), (Some(2), vec![REFERENCE.to_string()]));
}

/// A prompt whose only content is a `prologue:` the dry run left unread is
/// not proven empty: the real run includes whatever the value names. With
/// no `prologue:` it is.
#[test]
fn an_empty_body_beside_an_unread_prologue_is_not_proven_empty() {
    let fixture = fixture("dry-run-refs-unread-prologue-body");
    fs::write(fixture.cwd().join("seq.md"), effectful_task(EFFECTFUL)).unwrap();
    let task = |front: &str| format!("---\n$schema:\n  topic: string(required)\n{front}---\n");
    fs::write(fixture.cwd().join("task.md"), task("prologue: \"./{{ topic }}.md\"\n")).unwrap();

    let (code, rows) = check_rows(&fixture, &["sequence", "seq.md"]);
    assert_eq!(code, Some(0), "{rows:?}");
    assert!(rows.iter().all(|row| row["code"] == NOT_CHECKED), "{rows:?}");

    fs::write(fixture.cwd().join("task.md"), task("")).unwrap();
    let (code, codes) = check(&fixture, &["sequence", "seq.md"]);
    assert_eq!(code, Some(2), "{codes:?}");
    assert!(codes.iter().any(|held| held == "dm.compose.empty_body"), "{codes:?}");
}

/// Phase 1c's probe of a root body step composes it before any task, with
/// no outputs yet, so its block is left out there; a root `prologue:` the
/// dry run cannot read keeps that composition from being proven empty. With
/// no `prologue:` the probe's empty body is the refusal.
#[test]
fn a_root_body_probe_beside_an_unread_prologue_is_not_proven_empty() {
    let fixture = fixture("dry-run-refs-probe-unread-prologue");
    let seq = |front: &str| {
        format!(
            "---\nagent: claude\ntopic: \"{EFFECTFUL}\"\n{front}sequence:\n  - name: first\n    prompt: writer.md\n  - name: body\n---\n\n::block when=\"length(seq.outputs) > 0\"\nRead.\n::end-block\n"
        )
    };
    fs::write(fixture.cwd().join("writer.md"), "Write.\n").unwrap();
    fs::write(fixture.cwd().join("seq.md"), seq("prologue: \"./{{ topic }}.md\"\n")).unwrap();

    let (code, rows) = check_rows(&fixture, &["sequence", "seq.md"]);
    assert_eq!(code, Some(0), "{rows:?}");
    assert!(rows.iter().all(|row| row["severity"] != "error"), "{rows:?}");

    fs::write(fixture.cwd().join("seq.md"), seq("")).unwrap();
    let (code, codes) = check(&fixture, &["sequence", "seq.md"]);
    assert_eq!(code, Some(2), "{codes:?}");
    assert!(codes.iter().any(|held| held == "dm.compose.empty_body"), "{codes:?}");
}

/// A child's own `ignore_invalid: true` tolerates its broken inclusion as
/// composition does: a warning (still a problem for the exit status), never
/// a refusal. Its control, the child
/// without it, is refused.
#[test]
fn a_childs_own_ignore_invalid_tolerates_its_inclusion() {
    let fixture = fixture("dry-run-refs-child-tolerates");
    fs::write(fixture.cwd().join("doc.md"), "---\ntitle: p\nagent: claude\n---\nParent\n\n::file ./child.md\n").unwrap();
    let child = |front: &str| format!("---\ntitle: c\n{front}---\nChild\n\n::file ./missing.md\n");
    fs::write(fixture.cwd().join("child.md"), child("ignore_invalid: true\n")).unwrap();

    let (code, rows) = check_rows(&fixture, &["compose", "doc.md"]);
    assert_eq!(code, Some(2), "{rows:?}");
    assert_eq!(summary(&rows), pairs(&[(REFERENCE, "warning")]));

    fs::write(fixture.cwd().join("child.md"), child("")).unwrap();
    let (code, rows) = check_rows(&fixture, &["compose", "doc.md"]);
    assert_eq!(code, Some(2), "{rows:?}");
    assert_eq!(summary(&rows), pairs(&[(REFERENCE, "error")]));
}

/// A child that writes `ignore_invalid: false` under a tolerant parent is
/// refused, as composition refuses it; without its own value it takes the
/// parent's and is tolerated.
#[test]
fn a_childs_own_ignore_invalid_false_refuses_under_a_tolerant_parent() {
    let fixture = fixture("dry-run-refs-child-refuses");
    fs::write(
        fixture.cwd().join("doc.md"),
        "---\ntitle: p\nagent: claude\nignore_invalid: true\n---\nParent\n\n::file ./child.md\n",
    )
    .unwrap();
    let child = |front: &str| format!("---\ntitle: c\n{front}---\nChild\n\n::file ./missing.md\n");
    fs::write(fixture.cwd().join("child.md"), child("ignore_invalid: false\n")).unwrap();

    let (code, rows) = check_rows(&fixture, &["compose", "doc.md"]);
    assert_eq!(code, Some(2), "{rows:?}");
    assert_eq!(summary(&rows), pairs(&[(REFERENCE, "error")]));

    fs::write(fixture.cwd().join("child.md"), child("")).unwrap();
    let (code, rows) = check_rows(&fixture, &["compose", "doc.md"]);
    assert_eq!(code, Some(2), "{rows:?}");
    assert_eq!(summary(&rows), pairs(&[(REFERENCE, "warning")]));
}
