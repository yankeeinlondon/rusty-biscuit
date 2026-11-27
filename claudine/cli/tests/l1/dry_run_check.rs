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
mod compose_checks_the_provider_and_its_model_requirement {
    use super::*;

    fn fixture(name: &str) -> CliProcessFixture {
        let fixture = CliProcessFixture::named(name);
        fs::write(fixture.cwd().join("doc.md"), "---\nagent: claude\n---\nHello {{ who }}\n").unwrap();
        fixture
    }

    #[test]
    fn a_provider_not_on_path_is_an_error() {
        let fixture = fixture("dry-run-compose-missing");
        let missing = run(&fixture, &["compose", "doc.md", "who=world", "--dry-run"]);
        assert_eq!(missing.status.code(), Some(2), "stderr:\n{}", missing.stderr);
        assert!(prose(&missing.stderr).contains("is not installed: `claude` is not on PATH"), "stderr:\n{}", missing.stderr);
    }

    #[test]
    fn an_installed_provider_previews_the_body() {
        let fixture = fixture("dry-run-compose-clean");
        install_provider_binary(&fixture, "claude");
        let clean = run(&fixture, &["compose", "doc.md", "who=world", "--dry-run"]);
        assert_eq!(clean.status.code(), Some(0), "stderr:\n{}", clean.stderr);
        assert_eq!(clean.stdout.trim(), "Hello world");
        assert!(clean.stderr.contains("Dry run found 0 errors and 0 warnings; 0 checks not run. Previewed 1 of 1 task."));
    }

    #[test]
    fn opencode_without_a_model_is_an_error() {
        let fixture = fixture("dry-run-compose-model");
        install_provider_binary(&fixture, "claude");
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
}

/// A transcluded file the real run would replace with a notice and warn
/// about (it does not parse) is a warning row of the dry run in the
/// including document, not a clean report; the same document with a valid
/// part is the control ([`a_valid_transcluded_part_is_a_clean_report`]).
#[test]
fn a_tolerated_transclusion_failure_is_a_warning_row() {
    let fixture = transclusion_fixture("dry-run-tolerated-transclusion");
    fs::write(fixture.cwd().join("part.md"), "---\ntitle: [unclosed\n---\npart text\n").unwrap();
    let human = run(&fixture, &["compose", "doc.md", "--dry-run"]);
    assert_eq!(human.status.code(), Some(2), "stderr:\n{}", human.stderr);
    assert!(prose(&human.stderr).contains("Failed to parse frontmatter in"), "stderr:\n{}", human.stderr);
    assert!(human.stderr.contains("Dry run found 0 errors and 1 warning"), "stderr:\n{}", human.stderr);
    assert!(human.stdout.contains("_Could not transclude `part.md`_"), "stdout:\n{}", human.stdout);

    let json = run(&fixture, &["compose", "doc.md", "--dry-run", "--json"]);
    assert_eq!(json.status.code(), Some(2), "stdout:\n{}", json.stdout);
    let envelope: Value = serde_json::from_str(&json.stdout).unwrap();
    let diagnostics = envelope["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 1, "{}", json.stdout);
    assert_eq!(diagnostics[0]["severity"], "warning", "{}", json.stdout);
    // R1-F10.7: the tolerated child failure keeps its class, at the
    // inclusion that was left out.
    assert_eq!(diagnostics[0]["code"], "dm.transclusion.failure", "{}", json.stdout);
    assert!(diagnostics[0]["source"]["file"].as_str().unwrap().ends_with("doc.md"), "{}", json.stdout);
    assert_eq!((&diagnostics[0]["source"]["line"], &diagnostics[0]["source"]["column"]), (&Value::from(6), &Value::from(1)), "{}", json.stdout);
    assert_eq!(envelope["summary"]["warnings"], 1, "{}", json.stdout);
}

/// The control of [`a_tolerated_transclusion_failure_is_a_warning_row`]: a
/// part that parses is transcluded and the report is clean.
#[test]
fn a_valid_transcluded_part_is_a_clean_report() {
    let fixture = transclusion_fixture("dry-run-tolerated-transclusion-control");
    fs::write(fixture.cwd().join("part.md"), "part text\n").unwrap();
    let control = run(&fixture, &["compose", "doc.md", "--dry-run"]);
    assert_eq!(control.status.code(), Some(0), "stderr:\n{}", control.stderr);
    assert!(control.stdout.contains("part text"), "stdout:\n{}", control.stdout);
    assert!(control.stderr.contains("Dry run found 0 errors and 0 warnings"), "stderr:\n{}", control.stderr);
}

/// A fixture whose `doc.md` transcludes `part.md`, which the caller writes.
fn transclusion_fixture(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    install_provider_binary(&fixture, "claude");
    fs::write(fixture.cwd().join("doc.md"), "---\nagent: claude\n---\n# Doc\n\n::file part.md\n\nbody\n").unwrap();
    fixture
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
    let human = run(&fixture, &["sequence", "seq.md", "--dry-run"]);
    let stderr = prose(&human.stderr);
    assert_eq!(human.status.code(), Some(2), "stderr:\n{}", human.stderr);
    // Each parser's coordinate is the row's location, in the file's frame
    // (`child.md`'s frontmatter starts on line 2).
    for expected in [
        "is not valid YAML: YAML parse error: sequence[0]: duplicate entry with key \"name\" at line 3 column 5 at child.yaml:3:5",
        "is not valid JSON: JSON5 parse error",
        "line 1 column 15 (char 14) at rows.json5:1:15",
        "repeats a key: line 3: duplicate key `name`",
        "at rows.jsonl:3:23",
        "child.md: Failed to parse frontmatter",
        "at child.md:4:5",
        "Previewed 1 of 2 tasks.",
    ] {
        assert!(stderr.contains(expected), "{expected:?} in stderr:\n{}", human.stderr);
    }
    assert!(human.stdout.contains("the control prompt"), "stdout:\n{}", human.stdout);

    let json = run(&fixture, &["sequence", "seq.md", "--dry-run", "--json"]);
    let envelope: Value = serde_json::from_str(&json.stdout).unwrap();
    let located: Vec<(String, Value, Value)> = envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|diagnostic| diagnostic["severity"] == "error")
        .map(|diagnostic| {
            let source = &diagnostic["source"];
            let file = source["file"].as_str().unwrap().rsplit(['/', '\\']).next().unwrap().to_string();
            (file, source["line"].clone(), source["column"].clone())
        })
        .collect();
    let expected: Vec<(String, Value, Value)> =
        [("child.yaml", 3, 5), ("rows.json5", 1, 15), ("rows.jsonl", 3, 23), ("child.md", 4, 5)]
            .into_iter()
            .map(|(file, line, column)| (file.to_string(), Value::from(line), Value::from(column)))
            .collect();
    assert_eq!(located, expected, "{}", json.stdout);
    assert!(!fixture.cwd().join("marker").exists(), "a dry run runs no command");
}

/// R1-F10.3: an executable reference that is not a non-empty file reference
/// (a number, an empty string) is located at the value the task wrote, for
/// each executable; the valid control is still previewed.
#[test]
fn a_malformed_executable_reference_is_located_at_its_value() {
    let fixture = CliProcessFixture::named("dry-run-reference-values");
    install_provider_binary(&fixture, "claude");
    fs::write(fixture.cwd().join("ok.md"), "the control prompt\n").unwrap();
    fs::write(
        fixture.cwd().join("seq.md"),
        "---\nagent: claude\nsequence:\n  - name: number\n    prompt: 42\n  - name: empty\n    prompt: \"\"\n  - name: task\n    task: \"\"\n  - name: group\n    group: 3\n  - name: sequence\n    sequence: \"\"\n  - name: tail\n    prompt: ./ok.md\n---\n",
    )
    .unwrap();
    let json = run(&fixture, &["sequence", "seq.md", "--dry-run", "--json"]);
    assert_eq!(json.status.code(), Some(2), "{}", json.stdout);
    let envelope: Value = serde_json::from_str(&json.stdout).unwrap();
    let located: Vec<(Value, Value)> = envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|diagnostic| diagnostic["severity"] == "error")
        .map(|diagnostic| (diagnostic["source"]["line"].clone(), diagnostic["source"]["column"].clone()))
        .collect();
    let expected: Vec<(Value, Value)> =
        [(5, 13), (7, 13), (9, 11), (11, 12), (13, 15)].into_iter().map(|(line, column)| (line.into(), column.into())).collect();
    assert_eq!(located, expected, "{}", json.stdout);
}

/// A root list read from a referenced formal document, a referenced data
/// file, or a `{{ … }}` value: its middle row declares two executables and
/// is reported at its own place, and both siblings are still previewed.
mod a_bad_row_in_a_root_source_leaves_its_siblings_previewed {
    use super::*;

    const ROWS: &str = "- name: one\n  prompt: ./one.md\n- name: two\n  shell: touch marker\n  prompt: ./one.md\n- name: three\n  prompt: ./three.md\n";

    /// `ROWS` as the list under `key`.
    fn under(key: &str) -> String {
        format!("{key}:\n{}", ROWS.lines().map(|line| format!("  {line}\n")).collect::<String>())
    }

    #[test]
    fn formal() {
        assert_route("formal", "sequence: ./child.yaml\n", Some("at child.yaml:5:5"));
    }

    // A selected data row resolves from the root, and is located in the
    // data file that wrote it.
    #[test]
    fn data() {
        assert_route("data", "sequence: ./rows.yaml\n", Some("at rows.yaml:3:3"));
    }

    // Rows an expression computed have no proven place of their own.
    #[test]
    fn expression() {
        assert_route("expression", &format!("sequence: \"{{{{ rows }}}}\"\n{}", under("rows")), Some("in seq.md"));
    }

    fn assert_route(route: &str, root: &str, at: Option<&str>) {
        let fixture = CliProcessFixture::named(&format!("dry-run-root-sources-{route}"));
        install_provider_binary(&fixture, "claude");
        fs::write(fixture.cwd().join("one.md"), "first prompt\n").unwrap();
        fs::write(fixture.cwd().join("three.md"), "third prompt\n").unwrap();
        fs::write(fixture.cwd().join("child.yaml"), format!("kind: sequence\n{}", under("sequence"))).unwrap();
        fs::write(fixture.cwd().join("rows.yaml"), ROWS).unwrap();
        fs::write(fixture.cwd().join("seq.md"), format!("---\nagent: claude\n{root}---\n")).unwrap();
        let run = run(&fixture, &["sequence", "seq.md", "--dry-run"]);
        let stderr = prose(&run.stderr);
        assert_eq!(run.status.code(), Some(2), "{route} stderr:\n{}", run.stderr);
        for expected in [
            "task two (sequence[1])".to_string(),
            "this task names more than one executable: `shell`, `prompt`".to_string(),
            "Previewed 2 of 3 tasks.".to_string(),
        ].into_iter().chain(at.map(str::to_string)) {
            assert!(stderr.contains(&expected), "{route}: {expected:?} in stderr:\n{}", run.stderr);
        }
        assert!(run.stdout.contains("first prompt") && run.stdout.contains("third prompt"), "{route} stdout:\n{}", run.stdout);
        assert!(!fixture.cwd().join("marker").exists(), "a dry run runs no command");
    }
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
/// task's turn would pass it. With the sequence's input missing, the input
/// is reported once, by its owner, and the task is still previewed: what
/// reads the input, or a `with:` value derived from it, stays as written and
/// marked, and the rest composes.
#[test]
fn a_nested_task_previews_with_values_known_at_launch_and_marks_a_missing_input() {
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
    assert!(report.contains("not checked: input `plan` (a `with:` value is known only when the task runs)"), "{report}");
    assert_eq!(
        missing.stdout.trim(),
        "Plan [{{ plan }} [not evaluated in a dry run]] from [{{ spec }} [missing input]]",
        "stderr:\n{}",
        missing.stderr
    );
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
    let mut command = fixture.command();
    command.env("NO_COLOR", "1").env_remove("OPENCODE_MODEL").env_remove("MODEL");
    // A fixture that is graded on a terminal names the streams the check
    // treats as terminals; no terminal is opened.
    if let Ok(streams) = fs::read_to_string(fixture.cwd().join("terminal.txt")) {
        command.env("CLAUDINE_TEST_DRY_RUN_TERMINAL", streams.trim());
    }
    let output = command.args(args).output().unwrap();
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
/// everything. Each directory's `args.txt` holds the command line, and an
/// optional `terminal.txt` the streams the check grades as terminals. The
/// finding's report is pinned in both forms, plain text and JSON, as
/// `code_<code>_human` and `code_<code>_json`.
/// The `kind` (`finding` or `control`) fixture of `code`: the finding's
/// report carries the code and matches its JSON and human snapshots; the
/// control's carries no such row and exits 0.
fn assert_code_fixture(code: &str, kind: &str) {
    let root = biscuit_test_harness::manifest_dir!().join("tests/fixtures/check/codes").join(code);
    let full = format!("composition.{code}");
    let expected = kind == "finding";
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
        return;
    }
    let mut envelope = envelope;
    portable(&mut envelope, &launch_spellings(&fixture));
    insta::assert_json_snapshot!(format!("code_{code}_json"), envelope);
    let human = run_without_model_env(&fixture, &line.split_whitespace().chain(["--dry-run"]).collect::<Vec<_>>());
    insta::assert_snapshot!(format!("code_{code}_human"), human.stderr);
}

/// Two tests per code the registry marks `dry_run_report`, its finding and
/// its control, plus guards that the list equals the registry's scope and
/// that every fixture directory and snapshot has its code.
mod codes {
    macro_rules! code_fixtures {
        ($($code:ident),* $(,)?) => {
            $(#[test] fn $code() { super::assert_code_fixture(stringify!($code), "finding"); })*
            mod control {
                $(#[test] fn $code() { super::super::assert_code_fixture(stringify!($code), "control"); })*
            }
            const CODES: &[&str] = &[$(stringify!($code)),*];
        };
    }
    code_fixtures!(
        selection_key_as_data,
        no_executable,
        multiple_executables,
        sequence_shape,
        sequence_undefined,
        provider_unresolved,
        provider_unknown,
        provider_not_installed,
        model_required,
        invalid_arguments,
        missing_properties,
        file_choice_pending,
    );

    /// The scope comes from the registry: every code it marks
    /// `dry_run_report` has its two tests here, so a new scoped declaration
    /// without its fixture fails.
    #[test]
    fn every_scoped_registry_code_has_its_tests() {
        let mut scoped: Vec<String> = claudine::diagnostics::CODES
            .iter()
            .filter(|spec| spec.dry_run_report)
            .map(|spec| spec.code.strip_prefix("composition.").unwrap_or(spec.code).to_string())
            .collect();
        scoped.sort();
        let mut declared: Vec<String> = CODES.iter().map(|code| code.to_string()).collect();
        declared.sort();
        assert_eq!(declared, scoped);
    }

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

    /// The catalog snapshot guard: each code has its plain and JSON report
    /// snapshots, and every code snapshot belongs to a code with a fixture.
    #[test]
    fn every_code_has_its_snapshots_and_every_snapshot_its_code() {
        let dir = biscuit_test_harness::manifest_dir!().join("tests/l1/snapshots");
        let prefix = "l1__dry_run_check__code_";
        let mut snapshots: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|entry| {
                let name = entry.unwrap().file_name().to_string_lossy().into_owned();
                name.strip_prefix(prefix)?.strip_suffix(".snap").map(str::to_string)
            })
            .collect();
        snapshots.sort();
        let mut expected: Vec<String> =
            CODES.iter().flat_map(|code| [format!("{code}_human"), format!("{code}_json")]).collect();
        expected.sort();
        assert_eq!(snapshots, expected);
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
/// directory are byte-for-byte the same afterwards. One test per command
/// route, each on its own fixture.
mod a_dry_run_writes_nothing_anywhere {
    use super::*;

    macro_rules! no_write_routes {
        ($($name:ident => [$($arg:literal),+ $(,)?]),+ $(,)?) => {
            $(#[test]
            fn $name() {
                assert_writes_nothing(stringify!($name), &[$($arg),+]);
            })+
        };
    }

    no_write_routes! {
        sequence => ["sequence", "seq.md", "--dry-run"],
        sequence_json_yolo => ["sequence", "seq.md", "--dry-run", "--json", "--yolo"],
        compose_yolo => ["compose", "prompt.md", "--dry-run", "--yolo"],
        inline_compose => ["inline-compose", "prompt.md", "--dry-run"],
    }

    fn assert_writes_nothing(route: &str, args: &[&str]) {
        let fixture = CliProcessFixture::named(&format!("dry-run-no-write-{route}")).without_darkmatter_config();
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
        let run = run(&fixture, args);
        assert!(matches!(run.status.code(), Some(0 | 2)), "{args:?}: {}", run.stderr);
        assert_eq!(tree(&workspace), before, "{args:?} wrote to the fixture");
    }
}

// A step's preview reads `seq.*` as the real run binds it at the step's turn.
#[test]
fn a_step_preview_reads_its_seq_binding() {
    let fixture = CliProcessFixture::named("dry-run-seq-binding");
    install_provider_binary(&fixture, "goose");
    fs::write(fixture.cwd().join("s.md"), "---\nsequence:\n  - name: alpha\n  - name: beta\n---\nDo {{ seq.name }} at {{ seq.index }}\n").unwrap();
    let run = run(&fixture, &["sequence", "s.md", "--dry-run", "--goose"]);
    assert_eq!(run.status.code(), Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("Do alpha at 1") && run.stdout.contains("Do beta at 2"), "{}", run.stdout);
}

// A YAML sequence with a document-level `prompt:` composes that prompt
// inline at each step, as the real run does; it is not bodyless.
#[test]
fn a_yaml_sequence_with_a_prompt_composes_it_inline() {
    let fixture = CliProcessFixture::named("dry-run-yaml-prompt");
    install_provider_binary(&fixture, "goose");
    fs::write(fixture.cwd().join("y.yaml"), "kind: sequence\ntopic: docs\nprompt: \"Step {{ seq.name }}/{{ topic }}.\"\nsequence:\n  - alpha\n").unwrap();
    let run = run(&fixture, &["sequence", "y.yaml", "--dry-run", "--goose"]);
    assert_eq!(run.status.code(), Some(0), "{}", run.stderr);
    assert!(prose(&run.stderr).contains("composes the `prompt` of y.yaml"), "{}", run.stderr);
    assert!(run.stdout.contains("Step alpha/docs."), "{}", run.stdout);
}

// An inline container's `$schema` judges values at its turn: not checked.
#[test]
fn an_inline_containers_schema_is_not_checked() {
    let fixture = CliProcessFixture::named("dry-run-container-schema");
    fs::write(
        fixture.cwd().join("c.yaml"),
        "kind: sequence\nsequence:\n  - name: review\n    $schema: { reviewer: string(required) }\n    sequence: [{ name: sign, side_effect: { set: { done: true } } }]\n",
    ).unwrap();
    let run = run(&fixture, &["sequence", "c.yaml", "--dry-run"]);
    assert_eq!(run.status.code(), Some(0), "{}", run.stderr);
    assert!(prose(&run.stderr).contains("not checked: `$schema` (an inline container's schema judges the values at its turn)"), "{}", run.stderr);
}

/// A fixture named `name` with a `goose` provider on `PATH` and each
/// `(path, text)` file written into its launch directory.
fn sequence_fixture(name: &str, files: &[(&str, &str)]) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    install_provider_binary(&fixture, "goose");
    for (path, text) in files {
        fs::write(fixture.cwd().join(path), text).unwrap();
    }
    fixture
}

/// The passive check of `seq.md`: the human report, and the JSON envelope
/// with the launch directory made portable.
fn sequence_reports(fixture: &CliProcessFixture) -> (Run, Value) {
    let human = run(fixture, &["sequence", "seq.md", "--dry-run"]);
    let json = run(fixture, &["sequence", "seq.md", "--dry-run", "--json"]);
    let mut envelope: Value = serde_json::from_str(&json.stdout).unwrap_or_else(|error| panic!("{error}: {}", json.stdout));
    portable(&mut envelope, &launch_spellings(fixture));
    (human, envelope)
}

/// The JSON envelope of the passive check of `seq.md`, for a control that
/// needs only its rows.
fn sequence_json(fixture: &CliProcessFixture) -> (Run, Value) {
    let json = run(fixture, &["sequence", "seq.md", "--dry-run", "--json"]);
    let envelope: Value = serde_json::from_str(&json.stdout).unwrap_or_else(|error| panic!("{error}: {}", json.stdout));
    (json, envelope)
}

/// `text` with the launch directory spelled `/work`, so a plain snapshot is
/// the same on every host.
fn portable_text(fixture: &CliProcessFixture, text: &str) -> String {
    let mut value = Value::String(text.to_string());
    portable(&mut value, &launch_spellings(fixture));
    value.as_str().unwrap().to_string()
}

/// The error rows of an envelope, as `(code, message, hint)`.
fn error_rows(envelope: &Value) -> Vec<(String, String, Option<String>)> {
    envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["severity"] == "error")
        .map(|row| {
            (
                row["code"].as_str().unwrap().to_string(),
                row["message"].as_str().unwrap().to_string(),
                row["hint"].as_str().map(ToString::to_string),
            )
        })
        .collect()
}

/// A missing prompt file: the passive check names it, and the real run's
/// stop line names the prompt file and the proven cause. Control: the file
/// present, nothing is reported.
#[test]
fn a_missing_prompt_file_is_named_by_its_noun_and_cause() {
    let seq = "---\nagent: goose\nsequence:\n  - name: find\n    prompt: ./missing.md\n---\n";
    let fixture = sequence_fixture("seq-site-missing-prompt", &[("seq.md", seq)]);
    let (human, envelope) = sequence_reports(&fixture);
    assert_eq!(human.status.code(), Some(2), "{}", human.stderr);
    insta::assert_snapshot!("seq_site_missing_prompt_human", portable_text(&fixture, &human.stderr));
    insta::assert_json_snapshot!("seq_site_missing_prompt_json", envelope);
}

/// The real run of [`a_missing_prompt_file_is_named_by_its_noun_and_cause`]:
/// its stop line names the prompt file and the proven cause.
#[test]
fn a_real_run_stops_at_the_missing_prompt_file() {
    let seq = "---\nagent: goose\nsequence:\n  - name: find\n    prompt: ./missing.md\n---\n";
    let fixture = sequence_fixture("seq-site-missing-prompt-real", &[("seq.md", seq)]);
    let real = run(&fixture, &["sequence", "seq.md"]);
    assert!(
        prose(&real.stderr).contains("The run stopped at: prompt file `./missing.md` could not be resolved: file not found"),
        "{}",
        real.stderr
    );
}

/// The control of [`a_missing_prompt_file_is_named_by_its_noun_and_cause`]:
/// the file present, nothing is reported.
#[test]
fn a_present_prompt_file_is_not_reported() {
    let seq = "---\nagent: goose\nsequence:\n  - name: find\n    prompt: ./missing.md\n---\n";
    let control = sequence_fixture("seq-site-missing-prompt-control", &[("seq.md", seq), ("missing.md", "find it\n")]);
    let (json, envelope) = sequence_json(&control);
    assert_eq!(json.status.code(), Some(0), "{}", json.stdout);
    assert!(error_rows(&envelope).is_empty(), "{envelope:#}");
}

/// A task file that is not valid YAML: one row names the task file and the
/// parse failure, located in that file, with the hint for the cause.
/// Control: valid YAML reports nothing.
#[test]
fn a_task_file_yaml_error_names_the_file_and_the_parse_stage() {
    let seq = "---\nagent: goose\nsequence:\n  - name: t\n    task: ./bad.yaml\n---\n";
    let fixture = sequence_fixture("seq-site-yaml", &[("seq.md", seq), ("bad.yaml", "kind: task\nprompt: [unclosed\n")]);
    let (human, envelope) = sequence_reports(&fixture);
    assert_eq!(human.status.code(), Some(2), "{}", human.stderr);
    // The row holds the absolute path, so its wrapping varies by host: the
    // JSON snapshot pins the text, and the human report its shape.
    let stderr = prose(&human.stderr);
    assert!(stderr.contains("task t (sequence[0]) ✗ task file `"), "{}", human.stderr);
    assert!(stderr.contains("at bad.yaml:3:1 hint: Fix the YAML syntax of `"), "{}", human.stderr);
    insta::assert_json_snapshot!("seq_site_yaml_json", envelope);
    let rows = error_rows(&envelope);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert!(rows[0].1.starts_with("task file `/work/bad.yaml` is not valid YAML:"), "{rows:?}");
    assert_eq!(rows[0].2.as_deref(), Some("Fix the YAML syntax of `/work/bad.yaml`."));
}

/// The real run of [`a_task_file_yaml_error_names_the_file_and_the_parse_stage`]
/// stops on the same parse failure.
#[test]
fn a_real_run_stops_at_the_task_file_yaml_error() {
    let seq = "---\nagent: goose\nsequence:\n  - name: t\n    task: ./bad.yaml\n---\n";
    let fixture = sequence_fixture("seq-site-yaml-real", &[("seq.md", seq), ("bad.yaml", "kind: task\nprompt: [unclosed\n")]);
    let real = run(&fixture, &["sequence", "seq.md"]);
    assert!(prose(&real.stderr).contains("is not valid YAML"), "{}", real.stderr);
}

/// The control of [`a_task_file_yaml_error_names_the_file_and_the_parse_stage`]:
/// valid YAML reports nothing.
#[test]
fn a_valid_task_file_is_not_reported() {
    let seq = "---\nagent: goose\nsequence:\n  - name: t\n    task: ./bad.yaml\n---\n";
    let control = sequence_fixture(
        "seq-site-yaml-control",
        &[("seq.md", seq), ("bad.yaml", "kind: task\nprompt: ./p.md\n"), ("p.md", "do it\n")],
    );
    let (json, envelope) = sequence_json(&control);
    assert_eq!(json.status.code(), Some(0), "{}", json.stdout);
    assert!(error_rows(&envelope).is_empty(), "{envelope:#}");
}

/// A step naming two executables: the real run stops at a one-based step
/// that lists every executable a step may declare. Control: one executable.
#[test]
fn an_exclusive_executable_names_the_one_based_step_and_the_choices() {
    let seq = "---\nagent: goose\nsequence:\n  - name: both\n    prompt: ./x.md\n    sequence: ./y.md\n---\n";
    let fixture = sequence_fixture("seq-site-exclusive", &[("seq.md", seq), ("x.md", "x\n")]);
    let (human, envelope) = sequence_reports(&fixture);
    assert_eq!(human.status.code(), Some(2), "{}", human.stderr);
    insta::assert_snapshot!("seq_site_exclusive_human", portable_text(&fixture, &human.stderr));
    insta::assert_json_snapshot!("seq_site_exclusive_json", envelope);
}

/// The real run of [`an_exclusive_executable_names_the_one_based_step_and_the_choices`]:
/// its stop line names the one-based step and the choices.
#[test]
fn a_real_run_stops_at_the_exclusive_executable_step() {
    let seq = "---\nagent: goose\nsequence:\n  - name: both\n    prompt: ./x.md\n    sequence: ./y.md\n---\n";
    let fixture = sequence_fixture("seq-site-exclusive-real", &[("seq.md", seq), ("x.md", "x\n")]);
    let real = run(&fixture, &["sequence", "seq.md"]);
    assert!(
        prose(&real.stderr).contains(
            "The run stopped at: step 1 declares more than one executable field (prompt, sequence); \
             a step declares at most one of prompt, shell, side_effect, group, task, sequence"
        ),
        "{}",
        real.stderr
    );
}

/// The control of [`an_exclusive_executable_names_the_one_based_step_and_the_choices`]:
/// one executable reports nothing.
#[test]
fn a_single_executable_is_not_reported() {
    let control = sequence_fixture(
        "seq-site-exclusive-control",
        &[("seq.md", "---\nagent: goose\nsequence:\n  - name: both\n    prompt: ./x.md\n---\n"), ("x.md", "x\n")],
    );
    let (json, envelope) = sequence_json(&control);
    assert_eq!(json.status.code(), Some(0), "{}", json.stdout);
    assert!(error_rows(&envelope).is_empty(), "{envelope:#}");
}

/// An unknown field on a task file: one row, the shipped schema's, located
/// at the key and attached to the task occurrence, with the near miss as its
/// hint; the parser's refusal of the same key is not a second row. Control:
/// a key near nothing gets the schema's row with no did-you-mean.
#[test]
fn an_unknown_task_field_offers_its_near_miss() {
    let seq = "---\nagent: goose\nsequence:\n  - name: t\n    task: ./t.yaml\n---\n";
    let fixture = sequence_fixture(
        "seq-site-unknown-field",
        &[("seq.md", seq), ("t.yaml", "kind: task\nprompt: ./x.md\nwhem: \"true\"\n"), ("x.md", "x\n")],
    );
    let (human, envelope) = sequence_reports(&fixture);
    assert_eq!(human.status.code(), Some(2), "{}", human.stderr);
    insta::assert_snapshot!("seq_site_unknown_field_human", portable_text(&fixture, &human.stderr));
    insta::assert_json_snapshot!("seq_site_unknown_field_json", envelope);
    let rows = error_rows(&envelope);
    assert_eq!(rows.len(), 1, "{envelope:#}");
    assert_eq!(rows[0].0, "dm.schema.unknown_key");
    assert_eq!(rows[0].2.as_deref(), Some("did you mean `when`?"));
}

/// The control of [`an_unknown_task_field_offers_its_near_miss`]: a key near
/// nothing is still one row, whose hint is to remove the key or use one of
/// the task's declared keys.
#[test]
fn an_unknown_task_field_near_nothing_is_one_row() {
    let seq = "---\nagent: goose\nsequence:\n  - name: t\n    task: ./t.yaml\n---\n";
    let control = sequence_fixture(
        "seq-site-unknown-field-control",
        &[("seq.md", seq), ("t.yaml", "kind: task\nprompt: ./x.md\nzzzzzz: \"true\"\n"), ("x.md", "x\n")],
    );
    let (_, envelope) = sequence_json(&control);
    let rows = error_rows(&envelope);
    assert_eq!(rows.len(), 1, "{envelope:#}");
    assert_eq!(rows[0].0, "dm.schema.unknown_key", "{envelope:#}");
    assert!(rows[0].1.contains("'zzzzzz' was unexpected"), "{envelope:#}");
    assert_eq!(
        rows[0].2.as_deref(),
        Some("remove `zzzzzz`, or use one of `name`, `when`, `with`, `prompt`, `shell`, `side_effect`, `task`, `sequence`, …"),
        "{envelope:#}"
    );
}

/// `agent: []` in a prompt document: one row, the provider selection's.
const EMPTY_AGENT_SEQ: &str = "---\nagent: goose\nsequence:\n  - name: one\n    prompt: ./task.md\n---\n";

/// `args` over a document whose `agent: []` is reported once, by the provider
/// selection row, and not again as a schema row.
fn assert_empty_agent_reported_once(name: &str, args: &[&str]) {
    let fixture = sequence_fixture(name, &[("seq.md", EMPTY_AGENT_SEQ), ("task.md", "---\nagent: []\n---\nTask body\n")]);
    let run = run(&fixture, args);
    let stderr = prose(&run.stderr);
    assert!(stderr.contains("an `agent` list needs at least one entry"), "{args:?}: {}", run.stderr);
    assert!(!stderr.contains("schema validation failed"), "{args:?}: one row per problem:\n{}", run.stderr);
    assert!(stderr.contains("found 1 error"), "{args:?}: {}", run.stderr);
}

#[test]
fn an_empty_agent_list_is_reported_once_in_a_dry_run() {
    assert_empty_agent_reported_once("seq-agent-empty-dry", &["sequence", "seq.md", "--dry-run"]);
}

#[test]
fn an_empty_agent_list_is_reported_once_when_a_real_run_stops() {
    assert_empty_agent_reported_once("seq-agent-empty-real", &["sequence", "seq.md"]);
}

/// The same value supplied by a task's `with:` is one row too.
#[test]
fn an_empty_agent_list_from_with_is_reported_once() {
    let inline = sequence_fixture(
        "seq-agent-empty-with",
        &[("seq.md", "---\nagent: goose\nsequence:\n  - name: one\n    prompt: ./p.md\n    with: { agent: [] }\n---\n"), ("p.md", "hi\n")],
    );
    let inline_run = run(&inline, &["sequence", "seq.md", "--dry-run"]);
    assert!(prose(&inline_run.stderr).contains("found 1 error"), "{}", inline_run.stderr);
}

/// Control: a second schema problem in the same file keeps its schema row.
#[test]
fn another_schema_problem_beside_an_empty_agent_list_keeps_its_row() {
    let control = sequence_fixture(
        "seq-agent-empty-control",
        &[("seq.md", EMPTY_AGENT_SEQ), ("task.md", "---\nagent: []\nyolo: maybe\n---\nTask body\n")],
    );
    let control_run = run(&control, &["sequence", "seq.md", "--dry-run"]);
    let stderr = prose(&control_run.stderr);
    assert!(stderr.contains("an `agent` list needs at least one entry"), "{}", control_run.stderr);
    assert!(stderr.contains("`yolo` must be a boolean"), "{}", control_run.stderr);
    assert!(stderr.contains("found 2 errors"), "{}", control_run.stderr);
}
