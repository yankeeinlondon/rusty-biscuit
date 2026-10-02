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
/// that names it. `--add-dir` takes one value for Codex and a list for Claude,
/// so the one value it was given is right for both.
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

    let (code, _, stderr) = run(&fixture, &["sequence", &seq, "--add-dir", "x"]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let launches = launches(&log);
    assert_eq!(launches.len(), 2, "{launches:?}");
    assert_eq!((launches[0][0].as_str(), launches[1][0].as_str()), ("codex", "claude"));
    for launch in &launches {
        assert_eq!(occurrences(launch, &["--add-dir", "x"]), 1, "{launches:?}");
    }
    assert_eq!(count(&stderr, "Forwarding provider arguments to Codex: --add-dir"), 1, "{stderr}");
    assert_eq!(count(&stderr, "Forwarding provider arguments to Claude: --add-dir"), 1, "{stderr}");
}

/// A step that hands off to a provider whose researched types reject the
/// tail fails before that provider is spawned: Codex takes `-c x=y`, Claude's
/// `-c` takes no value.
#[test]
fn a_proxy_to_a_provider_that_types_the_tail_differently_fails_before_its_spawn() {
    let fixture = CliProcessFixture::named("tail-launch-multi-mismatch");
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

    assert_ne!(code, 0, "stderr:\n{stderr}");
    let launches = launches(&log);
    assert_eq!(launches.len(), 1, "Claude must never be spawned: {launches:?}");
    assert_eq!(launches[0][0], "codex");
    let flat = flat(&stderr);
    assert!(flat.contains("`-c` takes no further value for Claude"), "{stderr}");
    assert!(!flat.contains("likely caused by the forwarded arguments"), "not a native exit: {stderr}");
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

const EMBEDDED: &str = "sk-proj-embeddedsecret0123456789";
const LONG_ATTACHED: &str = "sk-proj-longattached0123456789";
const SHORT_ATTACHED: &str = "sk-proj-shortattached012345678";
const FLAG_ATTACHED: &str = "sk-proj-flagattached0123456789";
const FLAG_SEPARATE: &str = "sk-proj-flagseparate0123456789";

/// A credential inside an otherwise ordinary token — a configuration
/// assignment value or a long attached value — is masked like the shapes the
/// sensitive-flag and short-attachment rules cover, on every surface of all
/// three composition commands and the direct wrapper, while the child gets
/// the original tokens.
#[test]
fn embedded_credentials_reach_the_child_and_no_display_surface() {
    let assignment = format!("api_key={EMBEDDED}");
    let long = format!("--config={LONG_ATTACHED}");
    let short = format!("-c{SHORT_ATTACHED}");
    let token = format!("--token={FLAG_ATTACHED}");
    let composition_tail = [
        "-c",
        assignment.as_str(),
        long.as_str(),
        short.as_str(),
        token.as_str(),
        "--api-key",
        FLAG_SEPARATE,
    ];
    // The direct wrapper reads a separate word as its prompt, so its
    // sensitive flag carries the value attached.
    let api_key = format!("--api-key={FLAG_SEPARATE}");
    let direct_tail = [
        "-c",
        assignment.as_str(),
        long.as_str(),
        short.as_str(),
        token.as_str(),
        api_key.as_str(),
    ];
    let secrets = [EMBEDDED, LONG_ATTACHED, SHORT_ATTACHED, FLAG_ATTACHED, FLAG_SEPARATE];

    for command in ["compose", "inline-compose", "sequence", "codex"] {
        let fixture = CliProcessFixture::named(&format!("tail-embedded-{command}"));
        let body = fixture.cwd().join("inline.md");
        let log = stub(
            &fixture,
            "codex",
            &format!("printf '\\nAgent wrote this.\\n' >> '{}'\nexit 0", body.display()),
        );
        let target = match command {
            "compose" => doc(&fixture, "plan.md", "---\ntitle: plan\n---\nPlan body\n"),
            "inline-compose" => doc(&fixture, "inline.md", "---\ntitle: plan\nprompt: Write the plan.\n---\nOld body\n"),
            "sequence" => {
                doc(&fixture, "plan.md", "---\ntitle: plan\n---\nPlan body\n");
                doc(&fixture, "seq.md", "---\nsequence:\n  - name: plan\n    prompt: plan.md\n---\nBody\n")
            }
            _ => "do the thing".to_string(),
        };
        let tail: &[&str] = if command == "codex" { &direct_tail } else { &composition_tail };
        let with = |extra: &[&str]| {
            let mut args = vec![command];
            args.extend(extra);
            if command == "codex" {
                args.extend(tail);
                args.push(target.as_str());
            } else {
                args.extend([target.as_str(), "--codex"]);
                args.extend(tail);
            }
            args.into_iter().map(str::to_owned).collect::<Vec<_>>()
        };

        let output = fixture.command().args(with(&["--dry-run"])).output().unwrap();
        let dry_run = strip_ansi(&String::from_utf8_lossy(&output.stderr));
        assert!(output.status.success(), "{command} dry run:\n{dry_run}");
        // The dry-run table wraps cells anywhere and the direct command line
        // is shell-quoted, so compare without whitespace, borders, or quotes.
        let compact: String = dry_run
            .chars()
            .filter(|ch| !ch.is_whitespace() && !matches!(ch, '│' | '\''))
            .collect();
        let shown_masked = if command == "codex" { "--api-key=****" } else { "--api-key****" };
        for masked in ["-capi_key=****", "--config=****", "-c****", "--token=****", shown_masked] {
            assert!(compact.contains(masked), "{command} dry run lacks {masked}:\n{dry_run}");
        }

        let output = fixture
            .command()
            .env("RUST_LOG", "claudine=debug")
            .args(with(&[]))
            .output()
            .unwrap();
        let launched = strip_ansi(&String::from_utf8_lossy(&output.stderr));
        assert!(output.status.success(), "{command} launch:\n{launched}");

        for (surface, text) in [("dry run", &dry_run), ("notice/debug", &launched)] {
            for secret in secrets {
                assert!(!text.contains(secret), "{command}: {secret} leaked on {surface}:\n{text}");
            }
        }
        let params = fs::read_to_string(log.join("params-0")).unwrap();
        assert!(params.contains("api_key=****"), "{command} AGENT_PARAMS: {params}");
        for secret in secrets {
            assert!(!params.contains(secret), "{command}: {secret} leaked into AGENT_PARAMS: {params}");
        }
        let launches = launches(&log);
        assert_eq!(launches.len(), 1, "{command}: {launches:?}");
        assert_eq!(occurrences(&launches[0], tail), 1, "{command}: the child gets the tail unchanged: {launches:?}");
    }
}

/// A provider diagnostic that echoes an embedded or attached credential,
/// with or without its flag, is masked in the correlated report.
#[test]
fn a_correlated_report_masks_echoed_embedded_credentials() {
    let fixture = CliProcessFixture::named("tail-embedded-report");
    stub(
        &fixture,
        "goose",
        &format!(
            "echo \"error: unexpected argument '--config' found; value {LONG_ATTACHED}; key hunter2hunter2\" >&2\nexit 2"
        ),
    );
    let file = doc(&fixture, "plan.md", "---\ntitle: plan\n---\nPlan body\n");
    let long = format!("--config={LONG_ATTACHED}");

    let (code, _, stderr) = run(
        &fixture,
        &["compose", "--goose", "--quiet", &file, &long, "--profile=api_key=hunter2hunter2"],
    );

    assert_eq!(code, 2, "stderr:\n{stderr}");
    assert_eq!(count(&stderr, CORRELATED), 1, "{stderr}");
    assert!(count(&stderr, "value ****; key ****") >= 1, "masked, not dropped:\n{stderr}");
    assert!(!stderr.contains(LONG_ATTACHED) && !stderr.contains("hunter2hunter2"), "{stderr}");
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
