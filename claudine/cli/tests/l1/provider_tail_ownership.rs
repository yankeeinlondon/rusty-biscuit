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

use crate::common;
use common::launch_recorder::{install as stub, launches, occurrences};
use common::{CliProcessFixture, strip_ansi};

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

/// The setter-table row whose value only one candidate takes, followed by a
/// setter: `x=y` is forwarded with `-c` because Codex takes a string, and
/// `phase=2` is a setter. The run resolves to Claude, whose `-c` takes no
/// value, so it fails before the spawn instead of rerouting `x=y` into
/// frontmatter. (A document with no `agent` at all cannot resolve a provider
/// without a terminal, so the union here comes from the `agent` list; the
/// every-provider union is covered by the ownership unit tests.)
#[test]
fn a_union_setter_row_resolved_to_claude_fails_before_its_spawn() {
    let fixture = CliProcessFixture::named("own-setter-union");
    let log = stub(&fixture, "claude", "exit 0");
    // Only `phase` is a parameter: no schema claims the provider value key `x`.
    let file = doc(
        &fixture,
        "plan.md",
        &format!("---\nagent: [claude, codex]\n$schema:\n  phase: string\n{PROBE}---\nBody\n"),
    );

    let (code, stderr) = run(&fixture, &["compose", &file, "-c", "x=y", "phase=2"]);

    assert_ne!(code, 0, "{stderr}");
    assert!(launches(&log).is_empty(), "Claude must never be spawned");
    assert!(stderr.contains("`-c` takes no further value for Claude"), "{stderr}");
    assert!(stderr.contains("`x=y`"), "names the forwarded token: {stderr}");
    assert!(!stderr.contains("phase=2"), "the setter is not provider data: {stderr}");
    assert_eq!(probe(&fixture), "", "no lifecycle ran");
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
    assert_names_the_declared_setter(&stderr);
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
    assert_names_the_declared_setter(&stderr);
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
    assert_names_the_declared_setter(&stderr);

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

/// The grouped root help screen's usage line.
const ROOT_HELP: &str = "Usage: claudine <command> [options]";

/// Exit code, stdout, and stderr (both flattened) of `claudine args`.
fn run_for_help(fixture: &CliProcessFixture, args: &[&str]) -> (i32, String, String) {
    let output = fixture.command().args(args).output().unwrap();
    (
        output.status.code().unwrap_or(-1),
        flat(&strip_ansi(&String::from_utf8_lossy(&output.stdout))),
        flat(&strip_ansi(&String::from_utf8_lossy(&output.stderr))),
    )
}

/// Every Claudine help form on a composition command shows help and exits 0
/// whether the file is omitted, absent, or present but unreadable — a help
/// request is answered before the file requirement and never opens the file.
/// A `--help` after the authored `--` belongs to the provider, so that run is
/// an ordinary missing-document failure.
#[test]
fn help_opens_no_composition_file() {
    let fixture = CliProcessFixture::named("own-help");
    let launch_dir = stub(&fixture, "codex", "exit 0");
    let unreadable = doc(&fixture, "unreadable.md", "---\nagent: codex\n---\nBody\n");
    fs::set_permissions(&unreadable, std::os::unix::fs::PermissionsExt::from_mode(0o000)).unwrap();

    for subcommand in ["compose", "inline-compose", "sequence"] {
        let help_forms: [Vec<&str>; 11] = [
            vec![subcommand, "--help"],
            vec![subcommand, "-h"],
            vec!["--help", subcommand],
            vec!["-h", subcommand],
            vec!["--plain", "--debug", "info", "--help", subcommand],
            vec![subcommand, "missing.md", "--help"],
            vec![subcommand, "--help", "missing.md"],
            vec![subcommand, "missing.md", "-c", "x", "--help"],
            vec![subcommand, &unreadable, "--help"],
            vec![subcommand, &unreadable, "--codex", "-h"],
            vec!["--help", subcommand, &unreadable],
        ];
        for args in help_forms {
            let (code, stdout, stderr) = run_for_help(&fixture, &args);
            assert_eq!(code, 0, "{args:?}: {stderr}");
            assert!(stdout.contains(ROOT_HELP), "{args:?} showed no help: {stdout}");
        }

        let (code, stdout, stderr) = run_for_help(&fixture, &[subcommand, "missing.md", "--", "--help"]);
        assert_eq!(code, 1, "{subcommand} missing.md -- --help: {stderr}");
        assert!(!stdout.contains(ROOT_HELP), "help after `--` is the provider's: {stdout}");
        assert!(stderr.contains("CompositionError"), "{stderr}");
    }
    assert!(launches(&launch_dir).is_empty(), "a help request launched a provider");
}

/// The sibling help surfaces: every direct wrapper keeps its own help, a root
/// help request outranks any subcommand's required arguments, and nested
/// administrative subcommands accept `--help`/`-h` without their required
/// arguments.
#[test]
fn help_needs_no_required_argument_on_any_command() {
    let fixture = CliProcessFixture::named("own-help-siblings");

    for wrapper in [
        "claude", "codex", "gemini", "goose", "kimi", "opencode", "qwen", "kilo", "pi", "antigravity",
    ] {
        let (code, stdout, stderr) = run_for_help(&fixture, &[wrapper, "--help"]);
        assert_eq!(code, 0, "{wrapper} --help: {stderr}");
        assert!(stdout.contains(&format!("Usage: claudine {wrapper} ")), "{wrapper}: {stdout}");
    }

    for root_form in [
        &["--help", "completions"][..],
        &["--help", "budget"],
        &["--help", "budget", "grant"],
        &["-h", "mcp", "alias"],
        &["--help", "handle"],
        &["--help", "hooks"],
        &["--help", "claude"],
    ] {
        let (code, stdout, stderr) = run_for_help(&fixture, root_form);
        assert_eq!(code, 0, "{root_form:?}: {stderr}");
        assert!(stdout.contains(ROOT_HELP), "{root_form:?}: {stdout}");
    }

    for (args, usage) in [
        (&["hooks", "--help"][..], "Usage: claudine hooks"),
        (&["providers", "--help"], "Usage: claudine providers"),
        (&["actions", "--help"], "Usage: claudine actions"),
        (&["completions", "--help"], "Usage: claudine completions"),
        (&["budget", "--help"], "Usage: claudine budget"),
        (&["budget", "grant", "--help"], "Usage: claudine budget grant"),
        (&["budget", "init", "-h"], "Usage: claudine budget init"),
        (&["mcp", "alias", "--help"], "Usage: claudine mcp alias"),
        (&["logs", "sessions", "-h"], "Usage: claudine logs sessions"),
    ] {
        let (code, stdout, stderr) = run_for_help(&fixture, args);
        assert_eq!(code, 0, "{args:?}: {stderr}");
        assert!(stdout.contains(usage), "{args:?}: {stdout}");
    }
}

/// The missing-value error names the declared setter that stayed Claudine's
/// and the separate-value or `--` remedy, without echoing any setter value.
fn assert_names_the_declared_setter(stderr: &str) {
    assert!(stderr.contains("the `phase` setter after it"), "{stderr}");
    assert!(stderr.contains("separate provider value") && stderr.contains("`--`"), "{stderr}");
    assert!(!stderr.contains("phase=2") && !stderr.contains("x=y"), "{stderr}");
}
