//! The passive document check (`commands/document_check.rs`) on `sequence`,
//! `compose`, and `inline-compose`: it reports every problem it finds on
//! stderr, previews what composed on stdout, runs no command, side effect, or
//! provider, writes nothing, and exits 2 when any finding is an error or a
//! warning. The JSON report replaces both outputs with one envelope on stdout.

use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::common::{CliProcessFixture, strip_ansi};

struct Run {
    status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> Run {
    let output = fixture.command().env("NO_COLOR", "1").args(args).output().unwrap();
    assert!(!fixture.audio_spool().exists());
    Run {
        status: output.status,
        stdout: strip_ansi(&String::from_utf8_lossy(&output.stdout)),
        stderr: strip_ansi(&String::from_utf8_lossy(&output.stderr)),
    }
}

/// `text` with each run of whitespace as one space, so a wrapped row reads as
/// one line.
fn prose(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A fixture holding `tests/fixtures/check/<name>/` in its launch directory.
fn check_fixture(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named("dry-run-check");
    copy_tree(&biscuit_test_harness::manifest_dir!().join("tests/fixtures/check").join(name), fixture.cwd());
    fixture
}

/// An executable named `binary` on the fixture's `PATH`, which the check
/// only looks up and never runs.
fn install_provider_binary(fixture: &CliProcessFixture, binary: &str) {
    let path = fixture.bin_dir().join(format!("{binary}{}", std::env::consts::EXE_SUFFIX));
    fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// `value` with the launch directory spelled `/work` and `/` separators, so a
/// JSON snapshot is the same on every host.
fn portable(value: &mut Value, launch: &[String]) {
    match value {
        Value::String(text) => {
            let mut normalized = text.replace('\\', "/");
            for prefix in launch {
                normalized = normalized.replace(prefix.as_str(), "/work");
            }
            *text = normalized;
        }
        Value::Array(items) => items.iter_mut().for_each(|item| portable(item, launch)),
        Value::Object(map) => map.values_mut().for_each(|item| portable(item, launch)),
        _ => {}
    }
}

fn launch_spellings(fixture: &CliProcessFixture) -> Vec<String> {
    let mut spellings = vec![fixture.cwd().to_string_lossy().replace('\\', "/")];
    if let Ok(canonical) = fixture.cwd().canonicalize() {
        let canonical = canonical.to_string_lossy().replace('\\', "/");
        spellings.push(canonical.trim_start_matches("//?/").to_string());
        spellings.push(canonical);
    }
    spellings.sort_by_key(|spelling| std::cmp::Reverse(spelling.len()));
    spellings
}

/// Run 1 of the motivating session: two misplaced selection keys and a
/// missing provider on `find`, and a task with no executable over a YAML
/// file. Every problem is reported, `find` is still previewed, and the run
/// exits 2.
#[test]
fn fleet_run1_reports_every_problem_and_previews_the_composable_task() {
    let fixture = check_fixture("fleet-run1");
    let run = run(&fixture, &["sequence", "renderers/_fleet.yaml", "--dry-run"]);
    assert_eq!(run.status.code(), Some(2), "stderr:\n{}", run.stderr);
    insta::assert_snapshot!(run.stderr);
    assert!(run.stdout.contains("write a short overview to ./overview.md"), "stdout:\n{}", run.stdout);
    assert!(!run.stderr.contains('┃'), "a dry run never prints the standard error report:\n{}", run.stderr);
}

/// Run 2: the missing prompt file no longer hides `find`'s findings, and the
/// JSON envelope carries the same findings as data.
#[test]
fn fleet_run2_names_the_missing_prompt_file_and_keeps_checking() {
    let fixture = check_fixture("fleet-run2");
    let human = run(&fixture, &["sequence", "renderers/_fleet.yaml", "--dry-run"]);
    assert_eq!(human.status.code(), Some(2), "stderr:\n{}", human.stderr);
    insta::assert_snapshot!("fleet_run2_human", human.stderr);

    let json = run(&fixture, &["sequence", "renderers/_fleet.yaml", "--dry-run", "--json"]);
    assert_eq!(json.status.code(), Some(2), "stderr:\n{}", json.stderr);
    assert!(json.stderr.is_empty(), "JSON mode prints nothing on stderr:\n{}", json.stderr);
    let mut envelope: Value = serde_json::from_str(&json.stdout).unwrap();
    assert_eq!(envelope["summary"], serde_json::json!({ "errors": 2, "warnings": 2, "info": 3 }));
    portable(&mut envelope, &launch_spellings(&fixture));
    insta::assert_json_snapshot!("fleet_run2_json", envelope);
}

/// The check continues past a task that fails: the third task is previewed
/// after the second names a missing file, and an unknown provider is an
/// error distinct from no provider at all.
#[test]
fn a_failed_task_does_not_stop_the_tasks_after_it() {
    let fixture = CliProcessFixture::named("dry-run-continue");
    install_provider_binary(&fixture, "claude");
    fs::write(fixture.cwd().join("one.md"), "first prompt\n").unwrap();
    fs::write(fixture.cwd().join("three.md"), "third prompt\n").unwrap();
    fs::write(
        fixture.cwd().join("seq.yaml"),
        "sequence:\n  - name: one\n    prompt: ./one.md\n    with: { agent: nosuch }\n  - name: two\n    prompt: ./two.md\n  - name: three\n    prompt: ./three.md\n    with: { agent: claude }\n",
    )
    .unwrap();
    let run = run(&fixture, &["sequence", "seq.yaml", "--dry-run"]);
    assert_eq!(run.status.code(), Some(2), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("`nosuch` (with.agent) is not a known provider"), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("the prompt file does not exist"), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("Previewed 2 of 3 tasks."), "stderr:\n{}", run.stderr);
    assert!(run.stdout.contains("first prompt") && run.stdout.contains("third prompt"), "stdout:\n{}", run.stdout);
}

/// A `shell:` task, a `side_effect:` task, and a prompt's frontmatter
/// command are shown as not checked and never run; with nothing else to
/// report the dry run exits 0.
#[test]
fn commands_and_side_effects_never_run() {
    let fixture = CliProcessFixture::named("dry-run-effects");
    install_provider_binary(&fixture, "claude");
    fs::write(fixture.cwd().join("prompt.md"), "---\nagent: claude\nstamp: $(touch shell-marker)\n---\nstamp {{ stamp }}\n")
        .unwrap();
    fs::write(
        fixture.cwd().join("seq.md"),
        "---\nsequence:\n  - name: run\n    shell: \"touch task-marker\"\n  - name: write\n    side_effect: { append_line: [effect-marker, a] }\n  - name: ask\n    prompt: ./prompt.md\n---\n",
    )
    .unwrap();
    let run = run(&fixture, &["sequence", "seq.md", "--dry-run"]);
    assert_eq!(run.status.code(), Some(0), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("not checked: command (a dry run runs no commands)"), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("not checked: side effect (a dry run dispatches no side effects)"), "stderr:\n{}", run.stderr);
    let report = prose(&run.stderr);
    assert!(
        report.contains("not checked: frontmatter value (needs the output of `touch shell-marker`, which a dry run does not run)"),
        "stderr:\n{}",
        run.stderr
    );
    assert!(report.contains("not checked: interpolation (reads `stamp`, which is not known until the document runs)"), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("Previewed 1 of 3 tasks."), "stderr:\n{}", run.stderr);
    assert_eq!(run.stdout.trim(), "stamp {{ stamp }} [not evaluated in a dry run]");
    for marker in ["shell-marker", "task-marker", "effect-marker"] {
        assert!(!fixture.cwd().join(marker).exists(), "{marker} was created");
    }
}

/// A dry run with no configuration creates none: no `~/.darkmatter.yaml`,
/// no `~/.claudine/`, and nothing in the launch directory.
#[test]
fn a_dry_run_creates_no_configuration() {
    let fixture = check_fixture("fleet-run1").without_darkmatter_config();
    let before: Vec<_> = fs::read_dir(fixture.cwd()).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    let run = run(&fixture, &["sequence", "renderers/_fleet.yaml", "--dry-run"]);
    assert_eq!(run.status.code(), Some(2), "stderr:\n{}", run.stderr);
    assert_eq!(fs::read_dir(fixture.home()).unwrap().count(), 0, "the home directory is untouched");
    let after: Vec<_> = fs::read_dir(fixture.cwd()).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    assert_eq!(before, after);
}

/// `compose` checks one document: an installed provider and a composable
/// body exit 0 with the preview on stdout; a provider not on `PATH` and
/// OpenCode without a model are errors.
#[test]
fn compose_checks_the_provider_and_its_model_requirement() {
    let fixture = CliProcessFixture::named("dry-run-compose");
    fs::write(fixture.cwd().join("doc.md"), "---\nagent: claude\n---\nHello {{ who }}\n").unwrap();
    let missing = run(&fixture, &["compose", "doc.md", "who=world", "--dry-run"]);
    assert_eq!(missing.status.code(), Some(2), "stderr:\n{}", missing.stderr);
    assert!(prose(&missing.stderr).contains("is not installed: `claude` is not on PATH"), "stderr:\n{}", missing.stderr);

    install_provider_binary(&fixture, "claude");
    let clean = run(&fixture, &["compose", "doc.md", "who=world", "--dry-run"]);
    assert_eq!(clean.status.code(), Some(0), "stderr:\n{}", clean.stderr);
    assert_eq!(clean.stdout.trim(), "Hello world");
    assert!(clean.stderr.contains("Dry run found 0 errors and 0 warnings; 0 checks not run. Previewed 1 of 1 task."));

    install_provider_binary(&fixture, "opencode");
    let model = fixture
        .command()
        .env("NO_COLOR", "1")
        .env_remove("OPENCODE_MODEL")
        .env_remove("MODEL")
        .args(["compose", "doc.md", "who=world", "--dry-run", "--provider", "opencode"])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&model.stderr));
    assert_eq!(model.status.code(), Some(2), "stderr:\n{stderr}");
    assert!(stderr.contains("OpenCode needs a model in a non-interactive run"), "stderr:\n{stderr}");
}

/// `inline-compose --dry-run` leaves the input file byte-for-byte intact.
#[test]
fn inline_compose_dry_run_leaves_the_file_intact() {
    let fixture = CliProcessFixture::named("dry-run-inline");
    install_provider_binary(&fixture, "claude");
    let text = "---\nagent: claude\nprompt: Summarize this\n---\nBody\n";
    fs::write(fixture.cwd().join("doc.md"), text).unwrap();
    let run = run(&fixture, &["inline-compose", "doc.md", "--dry-run"]);
    assert_eq!(run.status.code(), Some(0), "stderr:\n{}", run.stderr);
    assert_eq!(fs::read_to_string(fixture.cwd().join("doc.md")).unwrap(), text);
}

/// A root document that does not exist is a finding in the report, not the
/// standard error report.
#[test]
fn a_missing_root_document_is_a_finding() {
    let fixture = CliProcessFixture::named("dry-run-missing-root");
    let run = run(&fixture, &["sequence", "nope.md", "--dry-run", "--json"]);
    assert_eq!(run.status.code(), Some(2), "stderr:\n{}", run.stderr);
    let envelope: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(envelope["diagnostics"][0]["code"], "dm.reference.missing_document");
    assert_eq!(envelope["outcome"], "failure");
}

/// `--dry-run-shell` is refused by the passive check in every position on
/// every document command, with no report and nothing run.
#[test]
fn dry_run_shell_is_refused_by_the_passive_check() {
    let fixture = CliProcessFixture::named("dry-run-shell-removed");
    fs::write(fixture.cwd().join("seq.md"), "---\nsequence:\n  - name: a\n    shell: \"touch marker\"\n---\n").unwrap();
    for command in ["sequence", "compose", "inline-compose"] {
        for args in [
            vec![command, "--dry-run-shell", "--dry-run", "seq.md"],
            vec![command, "seq.md", "--dry-run", "--dry-run-shell"],
        ] {
            let run = run(&fixture, &args);
            assert_eq!(run.status.code(), Some(2), "{args:?}: stderr:\n{}", run.stderr);
            assert!(run.stderr.contains("`--dry-run-shell` was removed"), "{args:?}: stderr:\n{}", run.stderr);
            assert!(run.stdout.is_empty(), "{args:?}: stdout:\n{}", run.stdout);
            assert!(!run.stderr.contains("Dry run of"), "{args:?}");
        }
    }
    assert!(!fixture.cwd().join("marker").exists());
}

/// The shipped schemas' key findings on the root as authored: a removed key
/// on the root, a group, or a leg is an error with its replacement, once per
/// key; a misspelled group key gets its did-you-mean; a near-miss step key
/// is a warning; `agent` (reported on its own) and `group.fail_fast` (legal)
/// get no schema finding.
#[test]
fn removed_unknown_and_near_miss_keys_are_findings_with_their_hints() {
    let fixture = CliProcessFixture::named("dry-run-schema-keys");
    install_provider_binary(&fixture, "claude");
    fs::write(fixture.cwd().join("ask.md"), "ask\n").unwrap();
    fs::write(
        fixture.cwd().join("seq.md"),
        "---\nfail_fast: null\nsequence:\n  - name: ask\n    promt: ./other.md\n    prompt: ./ask.md\n    agent: claude\n  - name: checks\n    group:\n      fail_fast: false\n      execution: parallel\n      max_paralel: 2\n      tasks:\n        - name: lint\n          shell: just lint\n          teardown: ./clean.md\n---\n",
    )
    .unwrap();
    let run = run(&fixture, &["sequence", "seq.md", "--dry-run"]);
    let stderr = prose(&run.stderr);
    assert_eq!(run.status.code(), Some(2), "stderr:\n{}", run.stderr);
    for expected in [
        "key `fail_fast` has been removed",
        "set fail_fast on a group object",
        "omit execution; use a nested sequence for ordered work",
        "use a group's `finalize:`; wrap a single task in a group of one",
        "did you mean `max_parallel`?",
        "key `promt` is not a declared field, so it is kept as data",
        "did you mean `prompt`?",
    ] {
        assert!(stderr.contains(expected), "{expected:?} in stderr:\n{}", run.stderr);
    }
    assert_eq!(stderr.matches("key `fail_fast` has been removed").count(), 1, "stderr:\n{}", run.stderr);
    assert!(!stderr.contains("key `agent`"), "`agent` has its own finding: {}", run.stderr);
}

/// A formal YAML sequence file is judged by its kind's definition.
#[test]
fn a_formal_sequence_files_removed_root_key_is_a_finding() {
    let fixture = CliProcessFixture::named("dry-run-schema-keys-yaml");
    fs::write(fixture.cwd().join("seq.yaml"), "kind: sequence\nfail_fast: true\nsequence:\n  - name: run\n    shell: just test\n")
        .unwrap();
    let run = run(&fixture, &["sequence", "seq.yaml", "--dry-run"]);
    let stderr = prose(&run.stderr);
    assert_eq!(run.status.code(), Some(2), "stderr:\n{}", run.stderr);
    assert!(stderr.contains("key `fail_fast` has been removed"), "stderr:\n{}", run.stderr);
    assert!(stderr.contains("set fail_fast on a group object"), "stderr:\n{}", run.stderr);
}

/// A referenced document that was read and failed to parse fails only the
/// task referencing it, naming the resolved file and the loader's line and
/// column, once per loader family; the control after it is still previewed
/// and nothing runs.
#[test]
fn a_document_that_fails_to_parse_fails_only_its_task() {
    let fixture = CliProcessFixture::named("dry-run-loaders");
    install_provider_binary(&fixture, "claude");
    let files = [
        ("child.yaml", "kind: sequence\nsequence:\n  - name: a\n    name: b\n    prompt: ./ok.md\n"),
        ("rows.json5", "[{name: 'a'}] xx\n"),
        ("rows.jsonl", "{\"name\": \"a\"}\n\n{\"name\": \"b\", \"name\": \"c\"}\n"),
        ("child.md", "---\nkind: sequence\nsequence:\n  - name: a\n    name: b\n---\n"),
        ("ok.md", "the control prompt\n"),
    ];
    for (name, text) in files {
        fs::write(fixture.cwd().join(name), text).unwrap();
    }
    fs::write(
        fixture.cwd().join("seq.md"),
        "---\nagent: claude\nsequence:\n  - name: yaml\n    sequence: ./child.yaml\n  - name: json5\n    sequence: ./rows.json5\n  - name: jsonl\n    sequence: ./rows.jsonl\n  - name: markdown\n    sequence: ./child.md\n  - name: mark\n    shell: touch marker\n  - name: tail\n    prompt: ./ok.md\n---\n",
    )
    .unwrap();
    let run = run(&fixture, &["sequence", "seq.md", "--dry-run"]);
    let stderr = prose(&run.stderr);
    assert_eq!(run.status.code(), Some(2), "stderr:\n{}", run.stderr);
    for expected in [
        "child.yaml: YAML parse error: sequence[0]: duplicate entry with key \"name\" at line 3 column 5",
        "rows.json5: JSON5 parse error",
        "line 1 column 15",
        "rows.jsonl: line 3: duplicate key `name`",
        "child.md: Failed to parse frontmatter",
        "Previewed 1 of 2 tasks.",
    ] {
        assert!(stderr.contains(expected), "{expected:?} in stderr:\n{}", run.stderr);
    }
    assert!(run.stdout.contains("the control prompt"), "stdout:\n{}", run.stdout);
    assert!(!fixture.cwd().join("marker").exists(), "a dry run runs no command");
}

/// Each occurrence of a referenced sequence is its own subject, labeled by
/// the task that references it, and a missing required input is one
/// `composition.missing_properties` row under its task.
#[test]
fn nested_occurrences_and_missing_inputs_are_reported_under_their_task() {
    let fixture = CliProcessFixture::named("dry-run-occurrences");
    install_provider_binary(&fixture, "claude");
    fs::write(fixture.cwd().join("needs.md"), "---\n$schema:\n  topic: string(required)\n---\nAbout {{ topic }}\n").unwrap();
    fs::write(fixture.cwd().join("child.yaml"), "kind: sequence\nsequence:\n  - name: inner\n    prompt: ./gone.md\n").unwrap();
    fs::write(
        fixture.cwd().join("seq.md"),
        "---\nagent: claude\nsequence:\n  - name: first\n    sequence: ./child.yaml\n  - name: again\n    sequence: ./child.yaml\n  - name: need\n    prompt: ./needs.md\n---\n",
    )
    .unwrap();
    let run = run(&fixture, &["sequence", "seq.md", "--dry-run", "--json"]);
    assert_eq!(run.status.code(), Some(2), "stdout:\n{}", run.stdout);
    let report: Value = serde_json::from_str(&run.stdout).unwrap();
    let diagnostics = report["diagnostics"].as_array().unwrap();
    let trail = |diagnostic: &Value| -> Vec<String> {
        diagnostic["trail"].as_array().unwrap().iter().map(|frame| format!("{} {}", frame["host"].as_str().unwrap(), frame["label"].as_str().unwrap())).collect()
    };
    let missing: Vec<_> = diagnostics.iter().filter(|diagnostic| diagnostic["code"] == "dm.reference.missing_document").map(trail).collect();
    assert_eq!(missing, [["sequence first", "task inner"], ["sequence again", "task inner"]], "one finding per occurrence:\n{}", run.stdout);
    let inputs: Vec<_> = diagnostics.iter().filter(|diagnostic| diagnostic["code"] == "composition.missing_properties").map(trail).collect();
    assert_eq!(inputs, [["task need"]], "{}", run.stdout);
}

/// A single-pass lifecycle literal that nests a span is refused before any
/// provider starts: for a sequence's referenced prompt at its task, and for
/// the document `compose` runs.
#[test]
fn a_nested_lifecycle_literal_is_a_finding() {
    let fixture = CliProcessFixture::named("dry-run-nested-literal");
    install_provider_binary(&fixture, "claude");
    fs::write(fixture.cwd().join("ok.md"), "---\nagent: claude\n---\nfine\n").unwrap();
    fs::write(fixture.cwd().join("bad.md"), "---\nagent: claude\nsuccess:\n    say: \"{{ ok ? 'done in {{area}}' : 'failed' }}\"\n---\nBody\n").unwrap();
    fs::write(fixture.cwd().join("seq.md"), "---\nsequence:\n  - name: fine\n    prompt: ./ok.md\n  - name: nested\n    prompt: ./bad.md\n---\n").unwrap();
    for args in [&["sequence", "seq.md", "--dry-run"][..], &["compose", "bad.md", "--dry-run"][..]] {
        let run = run(&fixture, args);
        let stderr = prose(&run.stderr);
        assert_eq!(run.status.code(), Some(2), "{args:?}; stderr:\n{}", run.stderr);
        assert!(stderr.contains("nests `{{area}}` inside the string literal"), "{args:?}; stderr:\n{}", run.stderr);
    }
}

/// A renamed lifecycle event in the document `compose` runs is refused, as
/// its preparation refuses it.
#[test]
fn compose_reports_a_renamed_lifecycle_event() {
    let fixture = CliProcessFixture::named("dry-run-renamed-event");
    install_provider_binary(&fixture, "claude");
    fs::write(fixture.cwd().join("doc.md"), "---\nagent: claude\ncomposed:\n    - info: \"ready\"\n---\nBody.\n").unwrap();
    let run = run(&fixture, &["compose", "doc.md", "--dry-run"]);
    let stderr = prose(&run.stderr);
    assert_eq!(run.status.code(), Some(2), "stderr:\n{}", run.stderr);
    assert!(stderr.contains("lifecycle event `composed` is named `start`"), "stderr:\n{}", run.stderr);
}

/// A setter naming a sequence declaration is refused before the list is
/// read, as a real run refuses it.
#[test]
fn a_setter_naming_a_declaration_is_a_finding() {
    let fixture = CliProcessFixture::named("dry-run-declaration-setter");
    fs::write(fixture.cwd().join("seq.yaml"), "sequence:\n  - name: run\n    shell: just test\n").unwrap();
    let run = run(&fixture, &["sequence", "seq.yaml", "--dry-run", "template={}"]);
    assert_eq!(run.status.code(), Some(2), "stderr:\n{}", run.stderr);
    assert!(prose(&run.stderr).contains("template"), "stderr:\n{}", run.stderr);
}

/// Every file under `dir`, relative to it, sorted.
fn files_under(dir: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.push(path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// The preview composes what it can and leaves what needs an effect as
/// written and marked: a command value and the spans that read it, a body
/// `::shell`, a `::block` whose condition reads it, and a function that
/// starts the login shell. Each is its own `not checked` row; independent
/// parts compose; nothing runs and nothing is written.
#[test]
fn a_preview_leaves_what_needs_an_effect_as_written_and_marks_it() {
    let fixture = CliProcessFixture::named("dry-run-passive-preview");
    install_provider_binary(&fixture, "claude");
    fs::write(
        fixture.cwd().join("doc.md"),
        "---\nagent: claude\nbranch: $(touch fm-marker && echo main)\ntitle: Release\n---\n# {{ title }}\nOn {{ branch }}.\n::shell touch body-marker\n::block when=\"branch == 'main'\"\nmain only\n::end-block\nAlias {{ has_alias(\"ll\") }}.\n",
    )
    .unwrap();
    let home_before = files_under(fixture.home());

    let run = run(&fixture, &["compose", "doc.md", "--dry-run"]);

    assert_eq!(run.status.code(), Some(0), "stderr:\n{}", run.stderr);
    let mark = "[not evaluated in a dry run]";
    assert!(run.stdout.contains("# Release\n"), "stdout:\n{}", run.stdout);
    assert!(run.stdout.contains(&format!("On {{{{ branch }}}} {mark}.")), "stdout:\n{}", run.stdout);
    assert!(run.stdout.contains(&format!("::shell touch body-marker {mark}")), "stdout:\n{}", run.stdout);
    assert!(run.stdout.contains(&format!("::block when=\"branch == 'main'\" {mark}\nmain only\n::end-block")), "stdout:\n{}", run.stdout);
    assert!(run.stdout.contains(&format!("Alias {{{{ has_alias(\"ll\") }}}} {mark}.")), "stdout:\n{}", run.stdout);
    let report = prose(&run.stderr);
    for row in [
        "not checked: frontmatter value (needs the output of `touch fm-marker && echo main`, which a dry run does not run)",
        // Rows that share a reason are listed together.
        "not checked: `::block` condition, interpolation (reads `branch`, which is not known until the document runs)",
        "not checked: `::shell` directive (needs the output of `touch body-marker`, which a dry run does not run)",
        "not checked: interpolation (`has_alias()` starts the login shell, which a dry run does not do)",
    ] {
        assert!(report.contains(row), "missing `{row}` in:\n{}", run.stderr);
    }
    assert!(!report.contains("Release"), "an independent value is not a row:\n{}", run.stderr);
    for marker in ["fm-marker", "body-marker"] {
        assert!(!fixture.cwd().join(marker).exists(), "{marker} was created");
    }
    assert_eq!(files_under(fixture.home()), home_before, "the dry run wrote under the home directory");
}

/// A nested task's `with:` value known at launch reaches its preview as the
/// task's turn would pass it; with the sequence's input missing, the task
/// waits for it instead of reporting the same input again.
#[test]
fn a_nested_task_previews_with_values_known_at_launch_and_waits_for_a_missing_input() {
    let fixture = CliProcessFixture::named("dry-run-passive-with");
    install_provider_binary(&fixture, "claude");
    fs::write(
        fixture.cwd().join("seq.md"),
        "---\nagent: claude\n$schema:\n    spec: string(required)\nplan: \"{{ spec + '-plan' }}\"\nsequence:\n    - name: outer\n      sequence:\n          - name: plan\n            prompt: ./plan.md\n            with: { plan: \"{{ plan }}\" }\n---\n",
    )
    .unwrap();
    fs::write(
        fixture.cwd().join("plan.md"),
        "---\n$schema:\n    spec: string(required)\n    plan: string(required)\n---\nPlan [{{ plan }}] from [{{ spec }}]\n",
    )
    .unwrap();

    let given = run(&fixture, &["sequence", "seq.md", "spec=work", "--dry-run"]);
    assert_eq!(given.status.code(), Some(0), "stderr:\n{}", given.stderr);
    assert_eq!(given.stdout.trim(), "Plan [work-plan] from [work]", "stderr:\n{}", given.stderr);

    let missing = run(&fixture, &["sequence", "seq.md", "--dry-run"]);
    assert_eq!(missing.status.code(), Some(2), "stdout:\n{}", missing.stdout);
    let report = prose(&missing.stderr);
    assert_eq!(report.matches("`spec` (string) is required and has no value").count(), 1, "{report}");
    assert!(report.contains("not checked: inputs, preview (it waits for an input its sequence has no value for)"), "{report}");
    assert!(missing.stdout.trim().is_empty(), "stdout:\n{}", missing.stdout);
}

/// A body whose every block was left out is an empty-prompt error that names
/// each block's condition and line, and claims nothing it did not observe.
#[test]
fn an_empty_body_names_the_blocks_that_were_left_out() {
    let fixture = CliProcessFixture::named("dry-run-passive-empty");
    install_provider_binary(&fixture, "claude");
    fs::write(fixture.cwd().join("doc.md"), "---\nagent: claude\nreview: false\n---\n::block when=\"review\"\nReview it.\n::end-block\n")
        .unwrap();

    let run = run(&fixture, &["compose", "doc.md", "--dry-run"]);

    assert_eq!(run.status.code(), Some(2), "stderr:\n{}", run.stderr);
    let report = prose(&run.stderr);
    assert!(
        report.contains("the prompt for this task is empty: rendered to nothing: every block was left out because its condition was false: `review` (line 5)"),
        "{report}"
    );
    assert!(report.contains("pass the values those conditions read"), "{report}");
    assert!(run.stdout.trim().is_empty(), "stdout:\n{}", run.stdout);
}

/// The inline preview runs the real command's baseline rehearsal: an
/// indented frontmatter root, which the run could not edit safely, is
/// refused, and the file is untouched.
#[test]
fn an_inline_preview_refuses_a_baseline_the_run_could_not_edit() {
    let fixture = CliProcessFixture::named("dry-run-passive-inline-baseline");
    install_provider_binary(&fixture, "claude");
    let text = "---\n  agent: claude\n  prompt: Summarize.\n---\nNotes.\n";
    fs::write(fixture.cwd().join("notes.md"), text).unwrap();

    let run = run(&fixture, &["inline-compose", "notes.md", "--dry-run"]);

    assert_eq!(run.status.code(), Some(2), "stderr:\n{}", run.stderr);
    assert!(prose(&run.stderr).contains("Cannot edit frontmatter text safely"), "stderr:\n{}", run.stderr);
    assert_eq!(fs::read_to_string(fixture.cwd().join("notes.md")).unwrap(), text);
}

/// `args` run with the model variables a host might export removed, so a
/// provider's model requirement reads only what the fixture wrote.
fn run_without_model_env(fixture: &CliProcessFixture, args: &[&str]) -> Run {
    let output = fixture
        .command()
        .env("NO_COLOR", "1")
        .env_remove("OPENCODE_MODEL")
        .env_remove("MODEL")
        .args(args)
        .output()
        .unwrap();
    Run {
        status: output.status,
        stdout: strip_ansi(&String::from_utf8_lossy(&output.stdout)),
        stderr: strip_ansi(&String::from_utf8_lossy(&output.stderr)),
    }
}

/// The standard error report's gutter: a dry run never prints it.
const STATUS_BLOCK_GUTTER: char = '┃';

/// The check's own code `composition.<code>` is in the report of
/// `tests/fixtures/check/codes/<code>/finding/` and absent from the clean
/// report of the `control/` beside it, so a check cannot pass by firing on
/// everything. Each directory's `args.txt` holds the command line.
fn assert_code_fixture(code: &str) {
    let root = biscuit_test_harness::manifest_dir!().join("tests/fixtures/check/codes").join(code);
    let full = format!("composition.{code}");
    for (kind, expected) in [("finding", true), ("control", false)] {
        let fixture = CliProcessFixture::named("dry-run-code");
        copy_tree(&root.join(kind), fixture.cwd());
        install_provider_binary(&fixture, "claude");
        install_provider_binary(&fixture, "opencode");
        let line = fs::read_to_string(fixture.cwd().join("args.txt")).unwrap();
        let mut args: Vec<&str> = line.split_whitespace().collect();
        args.extend(["--dry-run", "--json"]);
        let run = run_without_model_env(&fixture, &args);
        let envelope: Value = serde_json::from_str(&run.stdout).unwrap_or_else(|_| panic!("{full} {kind}: {}", run.stdout));
        let found = envelope["diagnostics"].as_array().unwrap().iter().any(|diagnostic| diagnostic["code"] == full.as_str());
        assert_eq!(found, expected, "{full} {kind}:\n{}", run.stdout);
        if !expected {
            assert_eq!(run.status.code(), Some(0), "{full} control is clean:\n{}", run.stdout);
        }
    }
}

/// One test per code of the check's own (`internal.bug` has no fixture: no
/// input reaches it), plus a guard that every fixture directory has its test.
mod codes {
    macro_rules! code_fixtures {
        ($($code:ident),* $(,)?) => {
            $(#[test] fn $code() { super::assert_code_fixture(stringify!($code)); })*
            const CODES: &[&str] = &[$(stringify!($code)),*];
        };
    }
    code_fixtures!(
        selection_key_as_data,
        no_executable,
        multiple_executables,
        sequence_shape,
        provider_unresolved,
        provider_unknown,
        provider_not_installed,
        model_required,
        invalid_arguments,
    );

    #[test]
    fn every_fixture_directory_has_its_test() {
        let root = biscuit_test_harness::manifest_dir!().join("tests/fixtures/check/codes");
        let mut found: Vec<String> = std::fs::read_dir(root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        found.sort();
        let mut declared: Vec<String> = CODES.iter().map(|code| code.to_string()).collect();
        declared.sort();
        assert_eq!(found, declared);
    }
}

/// A usage error is clap's, before any dry run starts: `--json` without
/// `--dry-run` prints the usage message and no report. Help still wins over
/// every other argument, the removed `--dry-run-shell` included.
#[test]
fn a_usage_error_prints_no_report_and_help_wins() {
    let fixture = CliProcessFixture::named("dry-run-usage");
    fs::write(fixture.cwd().join("doc.md"), "---\nagent: claude\n---\nHello\n").unwrap();
    for command in ["sequence", "compose", "inline-compose"] {
        let refused = run(&fixture, &[command, "doc.md", "--json"]);
        assert_eq!(refused.status.code(), Some(2), "{command}: {}", refused.stderr);
        assert!(refused.stderr.contains("--dry-run"), "{command}: {}", refused.stderr);
        assert!(!refused.stderr.contains("Dry run"), "{command}: {}", refused.stderr);
        assert!(refused.stdout.is_empty(), "{command}: {}", refused.stdout);

        let help = run(&fixture, &[command, "doc.md", "--dry-run", "--dry-run-shell", "--help"]);
        assert_eq!(help.status.code(), Some(0), "{command}: {}", help.stderr);
        assert!(help.stdout.contains("Usage"), "{command}: {}", help.stdout);
        assert!(!help.stderr.contains("Dry run"), "{command}: {}", help.stderr);
    }
}

/// Claudine's options are its own wherever they sit before the first `--`:
/// `--json` after the file selects the JSON report. After `--` the same
/// token is the provider's, and the human report runs.
#[test]
fn json_after_the_file_is_claudines_and_after_double_dash_is_the_providers() {
    let fixture = CliProcessFixture::named("dry-run-boundary");
    install_provider_binary(&fixture, "claude");
    fs::write(fixture.cwd().join("doc.md"), "---\nagent: claude\n---\nHello\n").unwrap();

    let json = run(&fixture, &["compose", "doc.md", "--dry-run", "--json"]);
    assert_eq!(json.status.code(), Some(0), "{}", json.stdout);
    assert!(json.stderr.is_empty(), "{}", json.stderr);
    let envelope: Value = serde_json::from_str(&json.stdout).unwrap();
    assert_eq!(envelope["outcome"], "success");

    let forwarded = run(&fixture, &["compose", "doc.md", "--dry-run", "--", "--json"]);
    assert_eq!(forwarded.status.code(), Some(0), "{}", forwarded.stderr);
    assert_eq!(forwarded.stdout.trim(), "Hello");
    assert!(forwarded.stderr.starts_with("Dry run of doc.md"), "{}", forwarded.stderr);
}

/// A warning alone exits 2, as an error does; only `info` findings (or none)
/// exit 0.
#[test]
fn a_warning_alone_exits_two() {
    let fixture = check_fixture("codes/selection_key_as_data/finding");
    install_provider_binary(&fixture, "claude");
    let run = run(&fixture, &["sequence", "seq.yaml", "--dry-run", "--json"]);
    assert_eq!(run.status.code(), Some(2), "{}", run.stdout);
    let envelope: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(envelope["summary"]["errors"], 0, "{}", run.stdout);
    assert_eq!(envelope["summary"]["warnings"], 1, "{}", run.stdout);
}

/// A fixture for the outcome `name`, the command that checks it, and its
/// exit status.
fn outcome_fixture(name: &str) -> (CliProcessFixture, Vec<&'static str>, i32) {
    let doc = "---\nagent: claude\n---\nHello\n";
    let (fixture, args, code) = match name {
        "clean" => {
            let fixture = CliProcessFixture::named("dry-run-streams-clean");
            fs::write(fixture.cwd().join("doc.md"), doc).unwrap();
            (fixture, vec!["compose", "doc.md"], 0)
        }
        "problems" => (check_fixture("fleet-run1"), vec!["sequence", "renderers/_fleet.yaml"], 2),
        "missing_root" => (CliProcessFixture::named("dry-run-streams-missing"), vec!["sequence", "nope.md"], 2),
        "unreadable_root" => {
            let fixture = CliProcessFixture::named("dry-run-streams-unreadable");
            fs::create_dir_all(fixture.cwd().join("dir.md")).unwrap();
            (fixture, vec!["compose", "dir.md"], 2)
        }
        "invalid_configuration" => {
            let fixture = CliProcessFixture::named("dry-run-streams-config");
            fs::write(fixture.cwd().join("doc.md"), doc).unwrap();
            fs::create_dir_all(fixture.home().join(".claudine")).unwrap();
            fs::write(fixture.home().join(".claudine/config.yaml"), "not_a_setting: 1\n").unwrap();
            (fixture, vec!["compose", "doc.md"], 2)
        }
        other => panic!("no outcome fixture `{other}`"),
    };
    install_provider_binary(&fixture, "claude");
    (fixture, args, code)
}

/// The outcome `name` keeps its streams apart: the human report is all of
/// stderr and never the standard error report; stdout holds only previews;
/// with `--json`, stdout is one JSON value, stderr is empty, and the exit
/// status is the same.
fn assert_streams_apart(name: &str) {
    let (fixture, args, code) = outcome_fixture(name);
    let mut human_args = args.clone();
    human_args.push("--dry-run");
    let human = run(&fixture, &human_args);
    assert_eq!(human.status.code(), Some(code), "{name}: {}", human.stderr);
    assert!(human.stderr.starts_with("Dry run"), "{name}: {}", human.stderr);
    assert!(human.stderr.trim_end().lines().last().unwrap().starts_with("Dry run found"), "{name}: {}", human.stderr);
    assert!(!human.stderr.contains(STATUS_BLOCK_GUTTER), "{name} printed the standard error report: {}", human.stderr);
    assert!(!human.stdout.contains("Dry run"), "{name}: {}", human.stdout);

    let mut json_args = human_args.clone();
    json_args.push("--json");
    let json = run(&fixture, &json_args);
    assert_eq!(json.status.code(), Some(code), "{name}: {}", json.stdout);
    assert!(json.stderr.is_empty(), "{name}: {}", json.stderr);
    let envelope: Value = serde_json::from_str(&json.stdout).unwrap_or_else(|error| panic!("{name}: {error}: {}", json.stdout));
    assert_eq!(envelope["outcome"], if code == 0 { "success" } else { "failure" }, "{name}");
}

mod streams {
    #[test]
    fn clean() {
        super::assert_streams_apart("clean");
    }
    #[test]
    fn problems() {
        super::assert_streams_apart("problems");
    }
    #[test]
    fn missing_root() {
        super::assert_streams_apart("missing_root");
    }
    #[test]
    fn unreadable_root() {
        super::assert_streams_apart("unreadable_root");
    }
    #[test]
    fn invalid_configuration() {
        super::assert_streams_apart("invalid_configuration");
    }
}

/// An invalid configuration file is a finding, located in the file; the
/// check goes on with the defaults, so the document is still previewed, and
/// the file is left as it was.
#[test]
fn an_invalid_configuration_is_reported_and_the_check_goes_on() {
    let fixture = CliProcessFixture::named("dry-run-invalid-config");
    install_provider_binary(&fixture, "claude");
    fs::write(fixture.cwd().join("doc.md"), "---\nagent: claude\n---\nHello\n").unwrap();
    let config = fixture.home().join(".claudine/config.yaml");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::write(&config, "not_a_setting: 1\n").unwrap();

    let human = run(&fixture, &["compose", "doc.md", "--dry-run"]);
    assert_eq!(human.status.code(), Some(2), "{}", human.stderr);
    assert_eq!(human.stdout.trim(), "Hello", "the document is still previewed");

    let json = run(&fixture, &["compose", "doc.md", "--dry-run", "--json"]);
    let envelope: Value = serde_json::from_str(&json.stdout).unwrap();
    let finding = envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"].as_str().is_some_and(|code| code.starts_with("config.")))
        .unwrap_or_else(|| panic!("no configuration finding: {}", json.stdout));
    assert_eq!(finding["severity"], "error");
    assert!(finding["source"]["file"].as_str().unwrap().ends_with("config.yaml"), "{finding}");
    assert_eq!(fs::read_to_string(&config).unwrap(), "not_a_setting: 1\n");
}

/// A task that fails to compose is a finding for that task; the report and
/// previews of the tasks after it are kept.
#[test]
fn a_task_that_fails_to_compose_keeps_the_output_after_it() {
    let fixture = CliProcessFixture::named("dry-run-compose-failure");
    install_provider_binary(&fixture, "claude");
    fs::write(fixture.cwd().join("one.md"), "first prompt\n").unwrap();
    fs::write(fixture.cwd().join("two.md"), "::file ./gone.md\n").unwrap();
    fs::write(fixture.cwd().join("three.md"), "third prompt\n").unwrap();
    fs::write(
        fixture.cwd().join("seq.yaml"),
        "agent: claude\nsequence:\n  - name: one\n    prompt: ./one.md\n  - name: two\n    prompt: ./two.md\n  - name: three\n    prompt: ./three.md\n",
    )
    .unwrap();
    let run = run(&fixture, &["sequence", "seq.yaml", "--dry-run"]);
    assert_eq!(run.status.code(), Some(2), "{}", run.stderr);
    assert!(run.stderr.contains("gone.md"), "{}", run.stderr);
    assert!(run.stderr.contains("Previewed 2 of 3 tasks."), "{}", run.stderr);
    assert!(run.stdout.contains("first prompt") && run.stdout.contains("third prompt"), "{}", run.stdout);
}

/// Every file under `dir` with its bytes, and every directory, keyed by its
/// path relative to `dir`.
fn tree(dir: &Path) -> std::collections::BTreeMap<String, Option<Vec<u8>>> {
    fn walk(root: &Path, dir: &Path, out: &mut std::collections::BTreeMap<String, Option<Vec<u8>>>) {
        for entry in fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            let key = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            if path.is_dir() {
                out.insert(key, None);
                walk(root, &path, out);
            } else {
                out.insert(key, Some(fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// A dry run of a document that would migrate a legacy configuration, set up
/// Darkmatter, publish the host registration, run commands, lifecycle
/// events, and side effects, and fill caches writes nothing: the fixture's
/// home (configuration, registration, caches), launch directory, and `PATH`
/// directory are byte-for-byte the same afterwards.
#[test]
fn a_dry_run_writes_nothing_anywhere() {
    let fixture = CliProcessFixture::named("dry-run-no-write").without_darkmatter_config();
    install_provider_binary(&fixture, "claude");
    fs::create_dir_all(fixture.home().join(".claudine")).unwrap();
    fs::write(fixture.home().join(".claudine/config.json"), "{ \"preferred_agent\": \"claude\" }").unwrap();
    fs::write(
        fixture.cwd().join("prompt.md"),
        "---\nagent: claude\nstamp: $(touch frontmatter-marker)\ninitialize:\n  - append_line: [initialize-marker, a]\n  - say: hello\n---\nstamp {{ stamp }}\n::shell touch body-marker\n",
    )
    .unwrap();
    fs::write(
        fixture.cwd().join("seq.md"),
        "---\nagent: claude\ninitialize:\n  - append_line: [sequence-marker, a]\nsequence:\n  - name: run\n    shell: \"touch task-marker\"\n  - name: write\n    side_effect: { append_line: [effect-marker, a] }\n  - name: ask\n    prompt: ./prompt.md\n---\n",
    )
    .unwrap();
    let workspace = fixture.cwd().parent().unwrap().to_path_buf();
    let before = tree(&workspace);
    for args in [
        vec!["sequence", "seq.md", "--dry-run"],
        vec!["sequence", "seq.md", "--dry-run", "--json", "--yolo"],
        vec!["compose", "prompt.md", "--dry-run", "--yolo"],
        vec!["inline-compose", "prompt.md", "--dry-run"],
    ] {
        let run = run(&fixture, &args);
        assert!(matches!(run.status.code(), Some(0 | 2)), "{args:?}: {}", run.stderr);
        assert_eq!(tree(&workspace), before, "{args:?} wrote to the fixture");
    }
}
