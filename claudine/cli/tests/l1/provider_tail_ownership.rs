//! Type-aware ownership through the compiled binary: which arguments after
//! the composition file are Claudine's setters, `argv` positionals, or the
//! provider tail, decided from the file as authored and the compiled switch
//! catalog, and the resolved-provider check that stops a launch before its
//! spawn.
//!
//! Every provider here is a shell stub, so the file is Unix-only; the
//! behavior under test is platform-neutral argv handling.
#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};

use crate::common;
use common::{CliProcessFixture, strip_ansi, write_executable};

/// Install a stub `binary` that records each launch (binary name, then its
/// arguments, `\x1f` separated, in `launches/launch-NNN`), then runs
/// `behavior`, which can read the 0-based launch number `$n`.
fn stub(fixture: &CliProcessFixture, binary: &str, behavior: &str) -> PathBuf {
    let dir = fixture.home().join("launches");
    fs::create_dir_all(&dir).unwrap();
    write_executable(
        &fixture.bin_dir().join(binary),
        &format!(
            "#!/bin/sh\n\
             dir='{dir}'\n\
             n=$(cat \"$dir/count\" 2>/dev/null || echo 0)\n\
             echo $((n + 1)) > \"$dir/count\"\n\
             file=$(printf 'launch-%03d' \"$n\")\n\
             {{ printf '%s\\037' '{binary}'; for arg in \"$@\"; do printf '%s\\037' \"$arg\"; done; }} > \"$dir/$file\"\n\
             {behavior}\n",
            dir = dir.display()
        ),
    );
    dir
}

fn launches(dir: &Path) -> Vec<Vec<String>> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .filter(|name| name.starts_with("launch-"))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
        .iter()
        .map(|name| {
            fs::read_to_string(dir.join(name))
                .unwrap()
                .split('\u{1f}')
                .filter(|arg| !arg.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .collect()
}

fn occurrences(args: &[String], run: &[&str]) -> usize {
    args.windows(run.len())
        .filter(|window| window.iter().zip(run).all(|(arg, want)| arg == want))
        .count()
}

fn contains(args: &[String], token: &str) -> bool {
    args.iter().any(|arg| arg == token)
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> (i32, String) {
    let output = fixture.command().args(args).output().unwrap();
    (
        output.status.code().unwrap_or(-1),
        flat(&strip_ansi(&String::from_utf8_lossy(&output.stderr))),
    )
}

/// `text` with status-block gutters removed and whitespace collapsed.
fn flat(text: &str) -> String {
    text.replace('┃', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn doc(fixture: &CliProcessFixture, name: &str, content: &str) -> String {
    let path = fixture.cwd().join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, content).unwrap();
    path.to_str().unwrap().to_owned()
}

fn probe(fixture: &CliProcessFixture) -> String {
    fs::read_to_string(fixture.cwd().join("probe.log")).unwrap_or_default()
}

/// A `success` stack that records `phase`, `x`, and `argv` as composed.
const PROBE: &str = "success:\n  stack:\n    - action: {append_line: ['probe.log', 'phase=[{{ phase }}] x=[{{ x }}] argv={{ argv }}']}\n";

/// A document with `extra` frontmatter, a `phase` schema parameter, and the
/// probe.
fn probed(extra: &str) -> String {
    format!("---\n{extra}$schema:\n  phase: string\n  x: string\n  argv: string[]\n{PROBE}---\nBody\n")
}

// ── The spec's three example commands (criteria 16, 17, 18) ──

#[test]
fn a_string_switch_takes_one_setter_and_a_schema_parameter_stays_claudines() {
    let fixture = CliProcessFixture::named("own-headline");
    let log = stub(&fixture, "codex", "exit 0");
    let file = doc(&fixture, "plan.md", &probed(""));

    let (code, stderr) = run(&fixture, &["compose", &file, "--codex", "-c", "model_reasoning_effort=low", "phase=2"]);

    assert_eq!(code, 0, "{stderr}");
    let launch = &launches(&log)[0];
    assert_eq!(occurrences(launch, &["-c", "model_reasoning_effort=low"]), 1, "{launch:?}");
    assert!(!contains(launch, "phase=2"), "{launch:?}");
    assert_eq!(probe(&fixture).trim(), "phase=[2] x=[] argv=");
}

/// With no provider named the candidates are every provider the frontmatter
/// lists; Codex reads `-c` as a string, so the setter is still forwarded.
#[test]
fn with_no_provider_named_a_setter_one_candidate_takes_is_forwarded() {
    let fixture = CliProcessFixture::named("own-headline-union");
    let log = stub(&fixture, "codex", "exit 0");
    let file = doc(&fixture, "plan.md", &probed("agent: [claude, codex]\n"));

    let (code, stderr) = run(&fixture, &["compose", &file, "-c", "model_reasoning_effort=low", "phase=2"]);

    assert_eq!(code, 0, "{stderr}");
    let launch = &launches(&log)[0];
    assert_eq!(launch[0], "codex", "only Codex is installed: {launch:?}");
    assert_eq!(occurrences(launch, &["-c", "model_reasoning_effort=low"]), 1, "{launch:?}");
    assert_eq!(probe(&fixture).trim(), "phase=[2] x=[] argv=");
}

#[test]
fn a_variadic_switch_takes_a_run_and_a_later_setter_is_claudines() {
    let fixture = CliProcessFixture::named("own-variadic");
    let log = stub(&fixture, "claude", "exit 0");
    let file = doc(&fixture, "plan.md", &probed(""));

    let (code, stderr) = run(&fixture, &["compose", &file, "--claude", "--add-dir", "a", "b", "x=y"]);

    assert_eq!(code, 0, "{stderr}");
    let launch = &launches(&log)[0];
    assert_eq!(occurrences(launch, &["--add-dir", "a", "b"]), 1, "{launch:?}");
    assert!(!contains(launch, "x=y"), "{launch:?}");
    assert_eq!(probe(&fixture).trim(), "phase=[] x=[y] argv=");
}

// ── The resolved-provider check (criterion 21) ──

/// The union forwarded the value because Codex takes one; the run resolved
/// to Claude, whose `-c` takes none, so it fails before Claude is spawned.
#[test]
fn a_resolved_provider_that_takes_no_value_fails_before_its_spawn() {
    let fixture = CliProcessFixture::named("own-extra-value");
    let log = stub(&fixture, "claude", "exit 0");
    let file = doc(&fixture, "plan.md", &probed("agent: [claude, codex]\n"));

    let (code, stderr) = run(&fixture, &["compose", &file, "-c", "model_reasoning_effort=low"]);

    assert_ne!(code, 0, "{stderr}");
    assert!(launches(&log).is_empty(), "Claude must never be spawned");
    assert!(stderr.contains("`-c` takes no further value for Claude"), "{stderr}");
    assert!(stderr.contains("`model_reasoning_effort=low`"), "names the token: {stderr}");
    assert!(stderr.contains("`--`"), "{stderr}");
    assert!(!stderr.contains("Forwarding provider arguments"), "no notice for a refused launch: {stderr}");
}

/// A sequence whose step provider is known statically fails before step 1.
#[test]
fn a_sequence_fails_before_step_one_when_a_known_step_provider_disagrees() {
    let fixture = CliProcessFixture::named("own-sequence-preflight");
    let log = stub(&fixture, "claude", "exit 0");
    doc(&fixture, "a.md", "---\ntitle: a\n---\nTask A\n");
    let seq = doc(
        &fixture,
        "seq.md",
        "---\nagent: [claude, codex]\nsequence:\n  - name: a\n    prompt: a.md\n  - name: b\n    prompt: a.md\n---\nBody\n",
    );

    let (code, stderr) = run(&fixture, &["sequence", &seq, "-c", "x=y"]);

    assert_ne!(code, 0, "{stderr}");
    assert!(launches(&log).is_empty(), "no step may start");
    assert!(stderr.contains("`-c` takes no further value for Claude"), "{stderr}");
}

/// Codex `--image` is a list at `exec` and one value at `exec resume`: the
/// fresh launch runs, the resume fails before its spawn.
#[test]
fn a_resume_is_checked_at_the_resume_entrypoint() {
    let fixture = CliProcessFixture::named("own-resume");
    let log = stub(
        &fixture,
        "codex",
        "printf '%s\\n' '{\"type\":\"thread.started\",\"thread_id\":\"thread-7\"}'\n\
         printf '%s\\n' '{\"type\":\"turn.started\"}'\n\
         printf '%s\\n' '{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}'\n\
         exit 1",
    );
    let file = doc(&fixture, "plan.md", "---\nfailure:\n  stack:\n    - action: {resume: 'continue'}\n---\nBody\n");

    let (code, stderr) = run(&fixture, &["compose", &file, "--codex", "--image", "a.png", "b.png"]);

    assert_ne!(code, 0, "{stderr}");
    let launches = launches(&log);
    assert_eq!(launches.len(), 1, "the resume must not spawn: {launches:?}");
    assert_eq!(occurrences(&launches[0], &["--image", "a.png", "b.png"]), 1, "{launches:?}");
    assert!(stderr.contains("`--image` takes no further value for Codex"), "{stderr}");
    assert!(stderr.contains("`exec resume` command"), "{stderr}");
}

/// A Claudine option between a switch and a word ends the value run.
#[test]
fn a_claudine_option_interrupting_a_value_run_fails_rather_than_reattaching() {
    let fixture = CliProcessFixture::named("own-interrupt");
    let log = stub(&fixture, "codex", "exit 0");
    let file = doc(&fixture, "plan.md", &probed(""));

    let (code, stderr) = run(&fixture, &["compose", &file, "--codex", "-c", "phase=2", "x=y"]);

    assert_ne!(code, 0, "{stderr}");
    assert!(launches(&log).is_empty());
    assert!(stderr.contains("`-c` takes a value for Codex"), "{stderr}");
}

// ── Ambiguity and the candidate set (criteria 19, 20, 29) ──

#[test]
fn a_disagreement_without_a_terminal_fails_with_guidance() {
    let fixture = CliProcessFixture::named("own-ambiguous");
    let log = stub(&fixture, "codex", "exit 0");
    let file = doc(&fixture, "plan.md", "---\nagent: [claude, codex]\n---\nBody\n");

    let (code, stderr) = run(&fixture, &["compose", &file, "-c", "foo"]);

    assert_ne!(code, 0, "{stderr}");
    assert!(launches(&log).is_empty());
    assert!(stderr.contains("ambiguous provider argument"), "{stderr}");
    assert!(stderr.contains("Claude: takes no value") && stderr.contains("Codex: takes one value"), "{stderr}");
    assert!(stderr.contains("`--codex`") && stderr.contains("`--`"), "{stderr}");

    // Naming the provider settles it.
    let (code, stderr) = run(&fixture, &["compose", &file, "--codex", "-c", "foo"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(occurrences(&launches(&log)[0], &["-c", "foo"]), 1);
}

/// Ownership reads the file as authored: a caller `agent=codex` changes the
/// run's provider selection, not who owns `foo`.
#[test]
fn a_caller_agent_setter_does_not_narrow_the_candidates() {
    let fixture = CliProcessFixture::named("own-snapshot");
    stub(&fixture, "codex", "exit 0");
    let file = doc(&fixture, "plan.md", "---\nagent: [claude, codex]\n---\nBody\n");

    let (code, stderr) = run(&fixture, &["compose", &file, "agent=codex", "-c", "foo"]);

    assert_ne!(code, 0, "{stderr}");
    assert!(stderr.contains("ambiguous provider argument"), "{stderr}");
}

/// A templated `agent` is not evaluated: every provider stays a candidate.
#[test]
fn a_templated_agent_keeps_every_provider_a_candidate() {
    let fixture = CliProcessFixture::named("own-templated-agent");
    stub(&fixture, "codex", "exit 0");
    let file = doc(&fixture, "plan.md", "---\nagent: \"{{ env.OWN_AGENT }}\"\n---\nBody\n");

    let output = fixture.command().env("OWN_AGENT", "codex").args(["compose", &file, "-c", "foo"]).output().unwrap();
    let stderr = flat(&strip_ansi(&String::from_utf8_lossy(&output.stderr)));

    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("ambiguous provider argument"), "{stderr}");
    assert!(stderr.contains("Gemini"), "every provider is a candidate: {stderr}");
}

// ── The authored `$schema` (criterion 29) ──

/// The schema file resolves next to the document, not the launch directory,
/// and a name declared in only one union arm still counts.
#[test]
fn schema_names_come_from_a_source_relative_union() {
    let fixture = CliProcessFixture::named("own-schema-union");
    let log = stub(&fixture, "codex", "exit 0");
    doc(&fixture, "docs/a.schema.yaml", "$schema:\n  title: string\n");
    doc(&fixture, "docs/b.schema.yaml", "$schema:\n  phase: string\n");
    let file = doc(
        &fixture,
        "docs/plan.md",
        "---\n$schema: [./a.schema.yaml, ./b.schema.yaml]\ntitle: t\n---\nBody\n",
    );

    let (code, stderr) = run(&fixture, &["compose", &file, "--codex", "-c", "phase=2"]);

    assert_ne!(code, 0, "`phase` is a parameter, so `-c` is left empty: {stderr}");
    assert!(launches(&log).is_empty());
    assert!(stderr.contains("`-c` takes a value for Codex"), "{stderr}");
}

/// A raw JSON Schema contributes its declared top-level property names.
#[test]
fn a_raw_json_schema_contributes_its_top_level_names() {
    let fixture = CliProcessFixture::named("own-schema-raw");
    let log = stub(&fixture, "codex", "exit 0");
    doc(
        &fixture,
        "plan.schema.json",
        r#"{"type":"object","properties":{"phase":{"type":"string","properties":{"nested":{}}}}}"#,
    );
    let file = doc(&fixture, "plan.md", "---\n$schema: ./plan.schema.json\n---\nBody\n");

    let (code, stderr) = run(&fixture, &["compose", &file, "--codex", "-c", "phase=2"]);
    assert_ne!(code, 0, "`phase` is declared, so `-c` is left empty: {stderr}");
    assert!(stderr.contains("`-c` takes a value for Codex"), "{stderr}");

    // A nested property is not a top-level name: the setter is the switch's.
    let (code, stderr) = run(&fixture, &["compose", &file, "--codex", "-c", "nested=1"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(occurrences(&launches(&log)[0], &["-c", "nested=1"]), 1);
}

/// A `--set` that adds a `$schema` changes validation, not ownership: the
/// authored document declares nothing, so `phase=2` is `-c`'s value.
#[test]
fn a_caller_schema_override_does_not_change_ownership() {
    let fixture = CliProcessFixture::named("own-schema-override");
    let log = stub(&fixture, "codex", "exit 0");
    let file = doc(&fixture, "plan.md", "---\ntitle: t\n---\nBody\n");

    let (code, stderr) = run(
        &fixture,
        &["compose", &file, "--codex", "--set", r#"{"$schema":{"phase":"string"}}"#, "-c", "phase=2"],
    );

    assert_eq!(code, 0, "{stderr}");
    assert_eq!(occurrences(&launches(&log)[0], &["-c", "phase=2"]), 1);
}

#[test]
fn an_unreadable_schema_is_an_error_not_a_guess() {
    let fixture = CliProcessFixture::named("own-schema-missing");
    let log = stub(&fixture, "codex", "exit 0");
    let file = doc(&fixture, "plan.md", "---\n$schema: ./missing.schema.yaml\n---\nBody\n");

    let (code, stderr) = run(&fixture, &["compose", &file, "--codex", "-c", "k=v"]);

    assert_ne!(code, 0, "{stderr}");
    assert!(launches(&log).is_empty());
    assert!(stderr.contains("missing.schema.yaml"), "{stderr}");
}

#[test]
fn a_templated_schema_makes_a_contested_setter_an_error() {
    let fixture = CliProcessFixture::named("own-schema-templated");
    let log = stub(&fixture, "codex", "exit 0");
    let file = doc(&fixture, "plan.md", "---\n$schema: \"{{ env.OWN_SCHEMA }}\"\n---\nBody\n");

    let (code, stderr) = run(&fixture, &["compose", &file, "--codex", "-c", "k=v"]);

    assert_ne!(code, 0, "{stderr}");
    assert!(launches(&log).is_empty());
    assert!(stderr.contains("cannot tell whether `k=…` after `-c`"), "{stderr}");
    assert!(stderr.contains("--set"), "{stderr}");
}

// ── Positionals: `argv` (criterion 23) ──

#[test]
fn bare_words_become_argv_in_order_and_override_an_authored_argv() {
    let fixture = CliProcessFixture::named("own-argv");
    let log = stub(&fixture, "codex", "exit 0");
    let file = doc(&fixture, "plan.md", &probed("argv: [authored]\n"));

    let (code, stderr) = run(
        &fixture,
        &["compose", &file, "alpha", "--codex", "-c", "model=y", "beta", "phase=2", "--", "gamma"],
    );

    assert_eq!(code, 0, "{stderr}");
    assert_eq!(probe(&fixture).trim(), r#"phase=[2] x=[] argv=["alpha","beta"]"#);
    let launch = &launches(&log)[0];
    assert_eq!(occurrences(launch, &["-c", "model=y", "gamma"]), 1, "{launch:?}");
    assert!(!contains(launch, "alpha") && !contains(launch, "beta"), "{launch:?}");

    // With no positionals the authored value is left alone.
    fs::remove_file(fixture.cwd().join("probe.log")).unwrap();
    let (code, stderr) = run(&fixture, &["compose", &file, "--codex"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(probe(&fixture).trim(), r#"phase=[] x=[] argv=["authored"]"#);
}

#[test]
fn argv_cannot_be_set_by_name() {
    let fixture = CliProcessFixture::named("own-argv-reserved");
    let log = stub(&fixture, "codex", "exit 0");
    let file = doc(&fixture, "plan.md", "---\ntitle: t\n---\nBody\n");

    for args in [
        vec!["compose", file.as_str(), "--codex", "argv=a"],
        vec!["compose", file.as_str(), "--codex", "-c", "argv=a"],
        vec!["compose", "argv=a", file.as_str(), "--codex"],
        vec!["compose", file.as_str(), "--codex", "--set", r#"{"argv":["a"]}"#],
    ] {
        let (code, stderr) = run(&fixture, &args);
        assert_ne!(code, 0, "{args:?}: {stderr}");
        assert!(stderr.contains("reserved for positional arguments"), "{args:?}: {stderr}");
    }
    assert!(launches(&log).is_empty());

    // After `--` it is provider data.
    let (code, stderr) = run(&fixture, &["compose", &file, "--codex", "--", "argv=a"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(occurrences(&launches(&log)[0], &["argv=a"]), 1);
}

/// `argv` is caller input: every sequence step and a proxy target see it.
#[test]
fn argv_reaches_every_sequence_step_and_a_proxy_target() {
    let fixture = CliProcessFixture::named("own-argv-sequence");
    stub(&fixture, "codex", "exit 0");
    let record = "success:\n  stack:\n    - action: {append_line: ['probe.log', '{{ title }} argv={{ argv }}']}\n";
    doc(&fixture, "a.md", &format!("---\ntitle: a\n{record}---\nTask A\n"));
    doc(
        &fixture,
        "b.md",
        "---\ntitle: b\ninitialize:\n  stack:\n    - action: {proxy: './target.md'}\n---\nTask B\n",
    );
    doc(&fixture, "target.md", &format!("---\ntitle: target\nagent: codex\n{record}---\nTarget\n"));
    let seq = doc(
        &fixture,
        "seq.md",
        "---\nagent: codex\nsequence:\n  - name: a\n    prompt: a.md\n  - name: b\n    prompt: b.md\n---\nBody\n",
    );

    let (code, stderr) = run(&fixture, &["sequence", &seq, "one", "two"]);

    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        probe(&fixture).lines().collect::<Vec<_>>(),
        [r#"a argv=["one","two"]"#, r#"target argv=["one","two"]"#]
    );
}

#[test]
fn inline_compose_never_persists_argv() {
    let fixture = CliProcessFixture::named("own-argv-inline");
    let path = fixture.cwd().join("plan.md");
    stub(
        &fixture,
        "codex",
        &format!("printf '\\nAgent wrote this.\\n' >> '{}'\nexit 0", path.display()),
    );
    let file = doc(&fixture, "plan.md", "---\ntitle: plan\nprompt: 'Write about {{ argv }}.'\n---\nOld body\n");

    let (code, stderr) = run(&fixture, &["inline-compose", &file, "--codex", "alpha", "beta"]);

    assert_eq!(code, 0, "{stderr}");
    let written = fs::read_to_string(&path).unwrap();
    assert!(written.contains("Agent wrote this."), "{written}");
    assert!(!written.contains("argv:") && !written.contains("alpha"), "argv was persisted:\n{written}");
}

// ── Help needs no file (criterion 27) ──

#[test]
fn help_opens_no_composition_file() {
    let fixture = CliProcessFixture::named("own-help");
    for subcommand in ["compose", "inline-compose", "sequence"] {
        let (code, stderr) = run(&fixture, &[subcommand, "missing.md", "-c", "x", "--help"]);
        assert_eq!(code, 0, "{subcommand}: {stderr}");
    }
}
