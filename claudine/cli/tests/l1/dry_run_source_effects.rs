//! A sequence dry run never calls an effectful expression function, wherever
//! the expression sits: a root `{{ … }}` source, a `::template(…)` operator,
//! a root or nested formal `template:`, or a prompt body. The effectful call
//! is `has_alias('ll')`, which starts the login shell named by `$SHELL`; the
//! test points `$SHELL` at a fake `bash` that appends a line to a marker file
//! each time it starts. The pure control `file_exists('./ask.md')` still
//! evaluates, so its source resolves and its task is previewed.
//!
//! The probe reads `$SHELL` and a script with a shebang, both Unix-only; on
//! Windows `has_alias` asks PowerShell through `ComSpec`, which this fake
//! cannot stand in for.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use crate::common::{CliProcessFixture, strip_ansi};

const EFFECT: &str = "has_alias('ll')";
const PURE: &str = "file_exists('./ask.md')";
const SHELL_ROW: &str = "`has_alias()` starts the login shell, which a dry run does not do";

struct Case {
    fixture: CliProcessFixture,
    marker: PathBuf,
}

/// A fixture with a provider stub, `ask.md`, and a fake `bash` (outside the
/// launch directory) that records each start in the returned marker path.
fn case(name: &str) -> Case {
    let fixture = CliProcessFixture::named(name);
    let claude = fixture.bin_dir().join("claude");
    fs::write(&claude, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).unwrap();
    let shell_dir = fixture.workspace_path().join("shell");
    fs::create_dir_all(&shell_dir).unwrap();
    let marker = shell_dir.join("launches");
    let bash = shell_dir.join("bash");
    fs::write(
        &bash,
        format!("#!/bin/sh\necho launched >> '{}'\nprintf 'darkmatter-shell-probe:alias\\n'\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&bash, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(fixture.cwd().join("ask.md"), "---\nagent: claude\n---\nAsk.\n").unwrap();
    Case { fixture, marker }
}

/// The dry run of `seq.md` (written as `frontmatter` plus `body`): its exit
/// code and its report as one line of prose.
fn dry_run(case: &Case, frontmatter: &str) -> (Option<i32>, String, String) {
    fs::write(case.fixture.cwd().join("seq.md"), format!("---\n{frontmatter}---\nBody.\n")).unwrap();
    let bash = case.marker.with_file_name("bash");
    let output = case
        .fixture
        .command()
        .env("SHELL", &bash)
        .args(["sequence", "seq.md", "--dry-run"])
        .output()
        .unwrap();
    assert!(!case.fixture.audio_spool().exists());
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let report = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    (output.status.code(), report, strip_ansi(&String::from_utf8_lossy(&output.stdout)))
}

fn assert_no_launch(case: &Case, report: &str) {
    assert!(!case.marker.exists(), "the login shell started during a dry run; report:\n{report}");
}

/// Each site with the effectful call and with the pure control: the
/// effectful one starts no shell and reports the call as not checked; the
/// pure one starts no shell, reports nothing about it, and previews `ask`.
fn assert_site(name: &str, frontmatter: impl Fn(&str) -> String, extra: &[(&str, &str)]) {
    let case = case(name);
    for (file, text) in extra {
        fs::write(case.fixture.cwd().join(file), text).unwrap();
    }

    let (code, report, _) = dry_run(&case, &frontmatter(EFFECT));
    assert_no_launch(&case, &report);
    assert_eq!(code, Some(0), "an unchecked value is not a defect; report:\n{report}");
    assert!(report.contains(SHELL_ROW), "missing the deferral row in:\n{report}");

    let (code, report, stdout) = dry_run(&case, &frontmatter(PURE));
    assert_no_launch(&case, &report);
    assert_eq!(code, Some(0), "report:\n{report}");
    assert!(!report.contains("has_alias"), "report:\n{report}");
    assert!(report.contains("Previewed 1 of 1 task."), "the pure source did not resolve; report:\n{report}");
    assert!(stdout.contains("Ask."), "stdout:\n{stdout}");
}

/// R1-F1.1: a root `{{ … }}` source whose ternary reads the call.
#[test]
fn a_root_source_expression_calls_no_effectful_function() {
    assert_site(
        "dry-run-effect-root",
        |call| {
            format!(
                "agent: claude\nsequence: \"{{{{ {call} ? [{{name: 'ask', prompt: './ask.md'}}] : [{{name: 'ask', prompt: './ask.md'}}] }}}}\"\n"
            )
        },
        &[],
    );
}

/// R1-F1.2: a `::template(…)` operator over a referenced file's row.
#[test]
fn a_template_operator_calls_no_effectful_function() {
    assert_site(
        "dry-run-effect-operator",
        |call| format!("agent: claude\nsequence: \"./rows.yaml::template({call} ? 'ask' : 'ask')\"\n"),
        &[("rows.yaml", "- prompt: ./ask.md\n")],
    );
}

/// R1-F1.3: a root formal `template:` field that supplies the task's prompt.
#[test]
fn a_root_formal_template_calls_no_effectful_function() {
    assert_site(
        "dry-run-effect-template",
        |call| {
            format!(
                "agent: claude\ntemplate:\n  prompt: \"{{{{ {call} ? './ask.md' : './ask.md' }}}}\"\nsequence:\n  - name: ask\n"
            )
        },
        &[],
    );
}

/// R1-F1.4: the same field on a nested inline sequence's `template:`.
#[test]
fn a_nested_formal_template_calls_no_effectful_function() {
    assert_site(
        "dry-run-effect-nested",
        |call| {
            format!(
                "agent: claude\nsequence:\n  - name: outer\n    template:\n      prompt: \"{{{{ {call} ? './ask.md' : './ask.md' }}}}\"\n    sequence:\n      - name: ask\n"
            )
        },
        &[],
    );
}

/// R1-F1.5: the call in a prompt body stays an interpolation deferral.
#[test]
fn a_prompt_body_calls_no_effectful_function() {
    let case = case("dry-run-effect-body");
    fs::write(case.fixture.cwd().join("ask.md"), format!("---\nagent: claude\n---\nAsk {{{{ {EFFECT} }}}}.\n")).unwrap();
    let (code, report, _) = dry_run(&case, "agent: claude\nsequence:\n  - name: ask\n    prompt: ./ask.md\n");
    assert_no_launch(&case, &report);
    assert_eq!(code, Some(0), "report:\n{report}");
    assert!(report.contains(&format!("not checked: interpolation ({SHELL_ROW})")), "report:\n{report}");
}

/// The other evaluators a check reaches: a root frontmatter value its owner
/// preparation evaluates, a `with:` value, and a prompt's frontmatter value.
/// None of them starts the shell, and the task is still previewed.
#[test]
fn owner_values_with_values_and_prompt_frontmatter_call_no_effectful_function() {
    let case = case("dry-run-effect-siblings");
    fs::write(
        case.fixture.cwd().join("ask.md"),
        format!("---\nagent: claude\nprobe: \"{{{{ {EFFECT} }}}}\"\n---\nAsk.\n"),
    )
    .unwrap();
    let frontmatter = format!(
        "agent: claude\nflag: \"{{{{ {EFFECT} }}}}\"\nsequence:\n  - name: ask\n    prompt: ./ask.md\n    with:\n      mood: \"{{{{ {EFFECT} }}}}\"\n"
    );
    let (code, report, stdout) = dry_run(&case, &frontmatter);
    assert_no_launch(&case, &report);
    assert_eq!(code, Some(0), "report:\n{report}");
    assert!(report.contains("Previewed 1 of 1 task."), "report:\n{report}");
    assert!(stdout.contains("Ask."), "stdout:\n{stdout}");
}
