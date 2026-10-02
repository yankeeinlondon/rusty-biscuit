//! The forwarded provider tail through the compiled binary: the exact child
//! argv on every launch a composition makes (fresh, retry, proxy target,
//! resume, each sequence step), secrets kept off every display surface, and
//! the one correlated report when a provider rejects the tail.
//!
//! Every provider here is a shell stub, so the file is Unix-only; the
//! behavior under test is platform-neutral argv and report handling.
#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};

use crate::common;
use common::{CliProcessFixture, strip_ansi, write_executable};

const API_KEY: &str = "sk-proj-launchsecret0123456789";
const TOKEN: &str = "sk-ant-tokensecret987654321";
const ATTACHED: &str = "sk-proj-attachedsecret5555";

/// Install a stub `binary` that records each launch, then runs `behavior`.
///
/// Launch `n` (0-based, in order, shared by every stub in the fixture) writes
/// the binary name and then its arguments to `launches/launch-NNN` (`\x1f`
/// separated), and its `AGENT_PARAMS` to `launches/params-N`. `behavior` is
/// shell code that can read `$n`.
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
             printf '%s' \"$AGENT_PARAMS\" > \"$dir/params-$n\"\n\
             {behavior}\n",
            dir = dir.display()
        ),
    );
    dir
}

/// Every recorded launch, in launch order.
fn launches(dir: &Path) -> Vec<Vec<String>> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("launch-"))
        .collect();
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

fn run(fixture: &CliProcessFixture, args: &[&str]) -> (i32, String, String) {
    let output = fixture.command().args(args).output().unwrap();
    (
        output.status.code().unwrap_or(-1),
        strip_ansi(&String::from_utf8_lossy(&output.stdout)),
        strip_ansi(&String::from_utf8_lossy(&output.stderr)),
    )
}

fn doc(fixture: &CliProcessFixture, name: &str, content: &str) -> String {
    let path = fixture.cwd().join(name);
    fs::write(&path, content).unwrap();
    path.to_str().unwrap().to_owned()
}

/// A document whose `success` records what `model_reasoning_effort` resolved
/// to; it stays empty unless a setter applied it.
const SETTER_PROBE: &str = "---\ntitle: plan\nsuccess:\n  stack:\n    - action: {append_line: ['probe.log', 'effort=[{{ model_reasoning_effort }}]']}\n---\nPlan body\n";

fn probe(fixture: &CliProcessFixture) -> String {
    fs::read_to_string(fixture.cwd().join("probe.log")).unwrap_or_default()
}

const HEADLINE: [&str; 2] = ["-c", "model_reasoning_effort=low"];

// ── Exact child argv (criteria 1, 2) ──

#[test]
fn compose_forwards_the_headline_tail_exactly_and_applies_no_setter() {
    let fixture = CliProcessFixture::named("tail-launch-compose");
    let log = stub(&fixture, "codex", "exit 0");
    let file = doc(&fixture, "plan.md", SETTER_PROBE);

    let (code, _, stderr) = run(&fixture, &["compose", &file, "--codex", HEADLINE[0], HEADLINE[1]]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let launches = launches(&log);
    assert_eq!(launches.len(), 1, "{launches:?}");
    assert_eq!(occurrences(&launches[0], &HEADLINE), 1, "{launches:?}");
    assert_eq!(probe(&fixture).trim(), "effort=[]", "the setter-shaped value was applied");
}

#[test]
fn compose_consumes_the_separator_and_forwards_the_explicit_tail() {
    let fixture = CliProcessFixture::named("tail-launch-explicit");
    let log = stub(&fixture, "codex", "exit 0");
    let file = doc(&fixture, "plan.md", SETTER_PROBE);

    let (code, _, stderr) = run(&fixture, &["compose", &file, "--codex", "--", "-c", "value"]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let launch = &launches(&log)[0];
    assert_eq!(occurrences(launch, &["-c", "value"]), 1, "{launch:?}");
    assert!(!launch.iter().any(|arg| arg == "--"), "the separator is consumed: {launch:?}");
}

#[test]
fn inline_compose_forwards_the_headline_tail_and_keeps_it_out_of_the_document() {
    let fixture = CliProcessFixture::named("tail-launch-inline");
    let path = fixture.cwd().join("plan.md");
    let log = stub(
        &fixture,
        "codex",
        &format!("printf '\\nAgent wrote this.\\n' >> '{}'\nexit 0", path.display()),
    );
    let file = doc(&fixture, "plan.md", "---\ntitle: plan\nprompt: Write the plan.\n---\nOld body\n");

    let (code, _, stderr) =
        run(&fixture, &["inline-compose", &file, "--codex", HEADLINE[0], HEADLINE[1]]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let launches = launches(&log);
    assert_eq!(launches.len(), 1, "{launches:?}");
    assert_eq!(occurrences(&launches[0], &HEADLINE), 1, "{launches:?}");
    let written = fs::read_to_string(&path).unwrap();
    assert!(written.contains("Agent wrote this."), "{written}");
    assert!(!written.contains("model_reasoning_effort"), "a tail value reached the file:\n{written}");
}

#[test]
fn sequence_forwards_the_headline_tail_to_each_step_and_applies_no_setter() {
    let fixture = CliProcessFixture::named("tail-launch-sequence");
    let log = stub(&fixture, "codex", "exit 0");
    doc(&fixture, "a.md", SETTER_PROBE);
    doc(&fixture, "b.md", SETTER_PROBE);
    let seq = doc(
        &fixture,
        "seq.md",
        "---\nsequence:\n  - name: a\n    prompt: a.md\n  - name: b\n    prompt: b.md\n---\nBody\n",
    );

    let (code, _, stderr) = run(&fixture, &["sequence", &seq, "--codex", HEADLINE[0], HEADLINE[1]]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let launches = launches(&log);
    assert_eq!(launches.len(), 2, "{launches:?}");
    for launch in &launches {
        assert_eq!(occurrences(launch, &HEADLINE), 1, "{launches:?}");
    }
    assert_eq!(probe(&fixture).lines().collect::<Vec<_>>(), ["effort=[]", "effort=[]"]);
}

// ── Exactly once across re-launches (criteria 7, 25) ──

#[test]
fn a_retry_relaunches_with_the_tail_exactly_once() {
    let fixture = CliProcessFixture::named("tail-launch-retry");
    let log = stub(&fixture, "codex", "[ \"$n\" = 0 ] && exit 1\nexit 0");
    let file = doc(&fixture, "plan.md", "---\nfailure:\n  stack:\n    - action: {retry: 1}\n---\nBody\n");

    let (code, _, stderr) = run(&fixture, &["compose", &file, "--codex", "-c", "x=y", "-c", "x=y"]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let launches = launches(&log);
    assert_eq!(launches.len(), 2, "{launches:?}");
    for launch in &launches {
        assert_eq!(occurrences(launch, &["-c", "x=y", "-c", "x=y"]), 1, "{launches:?}");
        assert_eq!(occurrences(launch, &["-c", "x=y"]), 2, "{launches:?}");
    }
}

#[test]
fn a_proxy_target_launches_with_the_tail_exactly_once() {
    let fixture = CliProcessFixture::named("tail-launch-proxy");
    let log = stub(&fixture, "codex", "[ \"$n\" = 0 ] && exit 1\nexit 0");
    doc(&fixture, "target.md", "---\ntitle: target\n---\nTarget body\n");
    let file = doc(
        &fixture,
        "plan.md",
        "---\nfailure:\n  stack:\n    - action: {proxy: './target.md'}\n---\nBody\n",
    );

    let (code, _, stderr) = run(&fixture, &["compose", &file, "--codex", "-c", "x=y"]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let launches = launches(&log);
    assert_eq!(launches.len(), 2, "{launches:?}\nstderr:\n{stderr}");
    for launch in &launches {
        assert_eq!(occurrences(launch, &["-c", "x=y"]), 1, "{launches:?}");
    }
}

/// The resumed attempt targets Codex's resume entrypoint and still carries the
/// authored tail once: repeated switches kept in order, and a user `--json`
/// neither dropped nor doubled by the transport allowlist.
#[test]
fn a_resume_carries_the_tail_exactly_once() {
    let fixture = CliProcessFixture::named("tail-launch-resume");
    let log = stub(
        &fixture,
        "codex",
        "printf '%s\\n' '{\"type\":\"thread.started\",\"thread_id\":\"thread-7\"}'\n\
         printf '%s\\n' '{\"type\":\"turn.started\"}'\n\
         printf '%s\\n' '{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}'\n\
         [ \"$n\" = 0 ] && exit 1\nexit 0",
    );
    let file = doc(
        &fixture,
        "plan.md",
        "---\nfailure:\n  stack:\n    - action: {resume: 'continue'}\n---\nBody\n",
    );
    let tail = ["--add-dir", "a", "--add-dir", "a", "--json"];

    let mut args = vec!["compose", file.as_str(), "--codex"];
    args.extend(tail);
    let (code, _, stderr) = run(&fixture, &args);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let launches = launches(&log);
    assert_eq!(launches.len(), 2, "{launches:?}\nstderr:\n{stderr}");
    let (fresh, resumed) = (&launches[0], &launches[1]);
    assert_eq!(occurrences(resumed, &["resume", "thread-7"]), 1, "{resumed:?}");
    for launch in [fresh, resumed] {
        assert_eq!(occurrences(launch, &tail), 1, "{launch:?}");
        assert_eq!(launch.iter().filter(|arg| *arg == "--json").count(), 1, "{launch:?}");
        assert_eq!(launch.iter().filter(|arg| *arg == "--add-dir").count(), 2, "{launch:?}");
    }
}

/// Each step of a sequence whose steps launch different providers gets the
/// same tail, and each provider gets its own notice. Sequence steps share the
/// sequence's `agent`, so step `b` reaches Claude by handing off to a document
/// that names it.
#[test]
fn each_step_of_a_multi_provider_sequence_carries_the_tail_once() {
    let fixture = CliProcessFixture::named("tail-launch-multi");
    let log = stub(&fixture, "codex", "exit 0");
    stub(&fixture, "claude", "exit 0");
    doc(&fixture, "a.md", "---\ntitle: a\n---\nTask A\n");
    doc(
        &fixture,
        "b.md",
        "---\ninitialize:\n  stack:\n    - action: {proxy: './b-claude.md'}\n---\nTask B\n",
    );
    doc(&fixture, "b-claude.md", "---\nagent: claude\n---\nTask B on Claude\n");
    let seq = doc(
        &fixture,
        "seq.md",
        "---\nagent: codex\nsequence:\n  - name: a\n    prompt: a.md\n  - name: b\n    prompt: b.md\n---\nBody\n",
    );

    let (code, _, stderr) = run(&fixture, &["sequence", &seq, "-c", "x=y"]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let launches = launches(&log);
    assert_eq!(launches.len(), 2, "{launches:?}");
    assert_eq!((launches[0][0].as_str(), launches[1][0].as_str()), ("codex", "claude"));
    for launch in &launches {
        assert_eq!(occurrences(launch, &["-c", "x=y"]), 1, "{launches:?}");
    }
    assert_eq!(count(&stderr, "Forwarding provider arguments to Codex: -c"), 1, "{stderr}");
    assert_eq!(count(&stderr, "Forwarding provider arguments to Claude: -c"), 1, "{stderr}");
}

// ── Secrets (criteria 9, 28) ──

/// A secret-shaped tail reaches the provider unchanged and appears on no
/// display surface: the notice, dry run, debug traces, `AGENT_PARAMS`, or the
/// correlated report (whose provider diagnostic echoes it).
#[test]
fn secrets_reach_the_child_and_no_display_surface() {
    let fixture = CliProcessFixture::named("tail-launch-secrets");
    let log = stub(
        &fixture,
        "goose",
        &format!("[ \"$n\" = 0 ] && exit 0\necho \"error: unexpected argument '{API_KEY}' found\" >&2\nexit 2"),
    );
    let file = doc(&fixture, "plan.md", "---\ntitle: plan\n---\nPlan body\n");
    let token = format!("--token={TOKEN}");
    let attached = format!("-c{ATTACHED}");
    let tail = ["--api-key", API_KEY, token.as_str(), attached.as_str()];
    let secrets = [API_KEY, TOKEN, ATTACHED];

    let with = |extra: &[&str]| {
        let mut args = vec!["compose", "--goose"];
        args.extend(extra);
        args.push(file.as_str());
        args.extend(tail);
        args.into_iter().map(str::to_owned).collect::<Vec<_>>()
    };

    // A launch that succeeds: the notice, plus debug tracing.
    let output = fixture
        .command()
        .env("RUST_LOG", "claudine=debug")
        .args(with(&[]))
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "stderr:\n{stderr}");
    assert_eq!(count(&stderr, "Forwarding provider arguments to Goose"), 1, "{stderr}");
    // A launch the provider rejects: the correlated report.
    let output = fixture.command().args(with(&[])).output().unwrap();
    let rejected = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.status.code(), Some(2), "stderr:\n{rejected}");
    assert_eq!(count(&rejected, "likely caused by the forwarded arguments"), 1, "{rejected}");
    assert!(rejected.contains("unexpected argument '****' found"), "masked, not dropped: {rejected}");
    // The dry run's "Provider args" row.
    let output = fixture.command().args(with(&["--dry-run"])).output().unwrap();
    let dry_run = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(dry_run.contains("Provider args"), "{dry_run}");

    for (surface, text) in [("notice/debug", &stderr), ("correlated", &rejected), ("dry run", &dry_run)] {
        for secret in secrets {
            assert!(!text.contains(secret), "{secret} leaked on {surface}:\n{text}");
        }
    }
    for n in 0..2 {
        let params = fs::read_to_string(log.join(format!("params-{n}"))).unwrap();
        assert!(!params.is_empty(), "AGENT_PARAMS is set");
        for secret in secrets {
            assert!(!params.contains(secret), "{secret} leaked into AGENT_PARAMS: {params}");
        }
    }
    for launch in launches(&log) {
        assert_eq!(occurrences(&launch, &tail), 1, "the child gets the tail unchanged: {launch:?}");
    }
}

// ── Correlated reports (criteria 10, 28) ──

/// `text` with status-block gutters removed and whitespace collapsed, so a
/// phrase the renderer wrapped across lines reads as one.
fn flat(text: &str) -> String {
    text.replace('┃', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn count(haystack: &str, needle: &str) -> usize {
    flat(haystack).matches(needle).count()
}

const CORRELATED: &str = "likely caused by the forwarded arguments";

#[test]
fn a_captured_stderr_rejection_is_reported_once_with_its_excerpt() {
    let fixture = CliProcessFixture::named("tail-correlate-stderr");
    stub(&fixture, "goose", "echo \"error: unexpected argument '--bogus' found\" >&2\nexit 2");
    let file = doc(&fixture, "plan.md", "---\ntitle: plan\n---\nBody\n");

    let (code, stdout, stderr) = run(&fixture, &["compose", "--goose", &file, "--bogus"]);

    assert_eq!(code, 2, "the provider's exit code is preserved:\n{stderr}");
    assert_eq!(count(&stderr, CORRELATED), 1, "{stderr}");
    assert_eq!(count(&stderr, "Goose rejected its arguments"), 1, "{stderr}");
    assert_eq!(count(&stderr, "forwarded arguments: --bogus."), 1, "{stderr}");
    assert!(!stderr.contains("recogni"), "{stderr}");
    assert_eq!(
        count(&format!("{stdout}\n{stderr}"), "error: unexpected argument '--bogus' found"),
        1,
        "the diagnostic is shown once, not echoed raw and again in the report:\n{stderr}"
    );

    // Quieted, the failure headline is gone, so the report carries the excerpt.
    let (code, _, quiet) = run(&fixture, &["compose", "--goose", "--quiet", &file, "--bogus"]);
    assert_eq!(code, 2, "{quiet}");
    assert_eq!(count(&quiet, CORRELATED), 1, "never suppressed by --quiet:\n{quiet}");
    assert_eq!(count(&quiet, "error: unexpected argument '--bogus' found"), 1, "{quiet}");
}

#[test]
fn a_structured_stdout_rejection_is_correlated() {
    let fixture = CliProcessFixture::named("tail-correlate-stdout");
    stub(&fixture, "codex", "echo \"error: unknown option '--bogus'\"\nexit 2");
    let file = doc(&fixture, "plan.md", "---\ntitle: plan\n---\nBody\n");

    let (code, stdout, stderr) = run(&fixture, &["compose", "--codex", &file, "--bogus"]);

    assert_eq!(code, 2, "stderr:\n{stderr}");
    assert_eq!(count(&stderr, CORRELATED), 1, "{stderr}");
    assert_eq!(
        count(&format!("{stdout}\n{stderr}"), "error: unknown option '--bogus'"),
        1,
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

/// A structured run streams the provider's stderr live, so the report names
/// the switch and points at it instead of repeating it.
#[test]
fn a_streamed_stderr_rejection_is_correlated_without_repeating_it() {
    let fixture = CliProcessFixture::named("tail-correlate-streamed");
    stub(&fixture, "codex", "echo \"error: unexpected argument '--bogus' found\" >&2\nexit 2");
    let file = doc(&fixture, "plan.md", "---\ntitle: plan\n---\nBody\n");

    let (code, _, stderr) = run(&fixture, &["compose", "--codex", &file, "--bogus"]);

    assert_eq!(code, 2, "stderr:\n{stderr}");
    assert_eq!(count(&stderr, CORRELATED), 1, "{stderr}");
    assert_eq!(count(&stderr, "Its own message is shown above."), 1, "{stderr}");
    let report = &stderr[stderr.find("Agent Error").expect("a report")..];
    assert!(!report.contains("unexpected argument"), "the report repeats the line:\n{report}");
}

#[test]
fn an_operand_only_explicit_tail_rejection_is_reported_once() {
    let fixture = CliProcessFixture::named("tail-correlate-explicit");
    stub(&fixture, "goose", "echo \"error: unexpected argument found\" >&2\nexit 2");
    let file = doc(&fixture, "plan.md", "---\ntitle: plan\n---\nBody\n");

    let (code, _, stderr) = run(&fixture, &["compose", "--goose", &file, "--", "stray"]);

    assert_eq!(code, 2, "stderr:\n{stderr}");
    assert_eq!(count(&stderr, CORRELATED), 1, "{stderr}");
    assert_eq!(count(&stderr, "an opaque argument tail (passed after --)."), 1, "{stderr}");
    assert!(!stderr.contains("stray"), "{stderr}");
}

/// Failures that are not a rejection of the forwarded tail are never
/// attributed to it, and the exit code is preserved.
#[test]
fn other_failures_are_not_attributed_to_the_tail() {
    let cases: &[(&str, &str, &[&str])] = &[
        ("injected", "echo \"error: unexpected argument '--output-last-message' found\" >&2\nexit 2", &[]),
        ("auth", "echo 'Error: authentication failed' >&2\nexit 2", &[]),
        ("api", "echo 'API Error: 500 internal' >&2\nexit 2", &[]),
        ("ambiguous", "echo 'something odd happened' >&2\nexit 2", &[]),
        ("interrupted", "echo \"error: unexpected argument '-c' found\" >&2\nexit 130", &[]),
        ("timeout", "sleep 5\nexit 0", &["--timeout", "1s"]),
    ];
    for (name, behavior, extra) in cases {
        let fixture = CliProcessFixture::named(&format!("tail-correlate-{name}"));
        stub(&fixture, "goose", behavior);
        let file = doc(&fixture, "plan.md", "---\ntitle: plan\n---\nBody\n");
        let mut args = vec!["compose", "--goose"];
        args.extend(*extra);
        args.extend([file.as_str(), "-c", "x=y"]);

        let (code, _, stderr) = run(&fixture, &args);

        assert_ne!(code, 0, "{name}: stderr:\n{stderr}");
        if !matches!(*name, "timeout") {
            let expected = if *name == "interrupted" { 130 } else { 2 };
            assert_eq!(code, expected, "{name}: stderr:\n{stderr}");
        }
        assert_eq!(count(&stderr, CORRELATED), 0, "{name} was misattributed:\n{stderr}");
    }
}
