//! A `key=value` setter written after a provider switch, through the compiled
//! binary: it is applied to the composition and never forwarded, on every
//! command (`compose`, `inline-compose`, `sequence`) and every relaunch
//! (retry, resume, proxy target), while the switch and its own value still
//! reach the provider token for token.
//!
//! The dry-run reproduction and the ordering errors need no provider and run
//! on every OS. The launch tests record argv with shell-stub providers, so
//! they are Unix-only; the behavior under test is platform-neutral argv
//! handling.

use std::fs;

use crate::common;
use common::{CliProcessFixture, strip_ansi};

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

/// The spec's shell-free reproduction document.
const REPRODUCTION: &str = "---\nphase: 1\n---\nPhase {{ phase }}\n";

/// The Codex config value from the reproduction; its quotes are part of the
/// value and must reach the provider unchanged.
const EFFORT: &str = r#"model_reasoning_effort="medium""#;

/// The tokens of the dry run's "Provider args" row, read from the cell
/// rather than the table's spacing.
fn provider_args_row(stderr: &str) -> Vec<String> {
    let line = stderr
        .lines()
        .find(|line| line.contains("Provider args"))
        .unwrap_or_else(|| panic!("no Provider args row:\n{stderr}"));
    let cell = line
        .split(['│', '|'])
        .map(str::trim)
        .rfind(|cell| !cell.is_empty())
        .unwrap();
    cell.split_whitespace().map(str::to_owned).collect()
}

// ── The reproduction (every OS, no provider installed) ──

#[test]
fn the_reproduction_applies_a_setter_after_the_switch_in_a_dry_run() {
    let fixture = CliProcessFixture::named("setter-after-switch-repro");
    let file = doc(&fixture, "plan.md", REPRODUCTION);

    for (label, args) in [
        ("control", ["compose", &file, "phase=2", "--codex", "-c", EFFORT, "--dry-run"]),
        ("defect", ["compose", &file, "--codex", "-c", EFFORT, "phase=2", "--dry-run"]),
    ] {
        let (code, stdout, stderr) = run(&fixture, &args);

        assert_eq!(code, 0, "{label}: {stderr}");
        assert!(stdout.contains("Phase 2") && !stdout.contains("Phase 1"), "{label}: {stdout}");
        assert_eq!(provider_args_row(&stderr), ["-c", EFFORT], "{label}: {stderr}");
    }
    assert_eq!(fs::read_to_string(&file).unwrap(), REPRODUCTION, "a dry run never writes the file");
}

// ── Ordering errors are unchanged (every OS) ──

#[test]
fn a_switch_or_separator_before_the_file_keeps_its_ordering_guidance() {
    let fixture = CliProcessFixture::named("setter-after-switch-ordering");
    let file = doc(&fixture, "plan.md", REPRODUCTION);

    let (code, _, stderr) = run(&fixture, &["compose", "-c", "x=y", &file, "phase=2"]);
    assert_ne!(code, 0, "{stderr}");
    assert!(stderr.contains("provider switch '-c' appears before the composition file"), "{stderr}");
    assert!(stderr.contains("Supported order: claudine compose <file>"), "{stderr}");

    let (code, _, stderr) = run(&fixture, &["compose", "--", &file, "phase=2"]);
    assert_ne!(code, 0, "{stderr}");
    assert!(stderr.contains("'--' appears before the composition file"), "{stderr}");
    assert!(stderr.contains("Supported order: claudine compose <file>"), "{stderr}");
}

#[cfg(unix)]
mod launched {
    use super::*;
    use common::launch_recorder::{install, launches, occurrences, prompts};

    fn contains(args: &[String], token: &str) -> bool {
        args.iter().any(|arg| arg == token)
    }

    fn probe(fixture: &CliProcessFixture) -> String {
        fs::read_to_string(fixture.cwd().join("probe.log")).unwrap_or_default()
    }

    /// Records the composed `phase` once the run succeeds.
    const PROBE: &str = "success:\n  stack:\n    - action: {append_line: ['probe.log', 'phase={{ phase }}']}\n";

    /// A Codex stub whose structured stream names thread `thread-7`, so a
    /// failed launch can be resumed; launch 0 fails, later launches succeed.
    const RESUMABLE_CODEX: &str = "printf '%s\\n' '{\"type\":\"thread.started\",\"thread_id\":\"thread-7\"}'\n\
         printf '%s\\n' '{\"type\":\"turn.started\"}'\n\
         printf '%s\\n' '{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}'\n\
         [ \"$n\" = 0 ] && exit 1\nexit 0";

    /// The provider got `-c x=y` exactly once and no setter.
    fn assert_only_the_switch_was_forwarded(launch: &[String]) {
        assert_eq!(occurrences(launch, &["-c", "x=y"]), 1, "{launch:?}");
        assert!(
            !launch.iter().any(|arg| arg.starts_with("phase=") || arg.contains("Phase ")),
            "a setter or the prompt reached the argv: {launch:?}"
        );
    }

    // ── Each command (compose, inline-compose, sequence) ──

    #[test]
    fn compose_renders_a_post_switch_setter_and_forwards_only_the_switch() {
        let fixture = CliProcessFixture::named("setter-after-switch-compose");
        let log = install(&fixture, "codex", "exit 0");
        let file = doc(&fixture, "plan.md", REPRODUCTION);

        let (code, _, stderr) = run(&fixture, &["compose", &file, "--codex", "-c", "x=y", "phase=2"]);

        assert_eq!(code, 0, "{stderr}");
        let launches = launches(&log);
        assert_eq!(launches.len(), 1, "{launches:?}");
        assert_eq!(launches[0][..3], ["codex", "exec", "-c"], "{launches:?}");
        assert_eq!(launches[0][3], "x=y", "{launches:?}");
        assert_only_the_switch_was_forwarded(&launches[0]);
        assert_eq!(prompts(&log)[0].trim(), "Phase 2");
    }

    #[test]
    fn inline_compose_launches_with_the_setter_and_never_persists_it() {
        let fixture = CliProcessFixture::named("setter-after-switch-inline");
        let path = fixture.cwd().join("plan.md");
        let log = install(
            &fixture,
            "codex",
            &format!("printf '\\nAgent wrote this.\\n' >> '{}'\nexit 0", path.display()),
        );
        let file = doc(&fixture, "plan.md", "---\nphase: 1\nprompt: 'Write phase {{ phase }}.'\n---\nOld body\n");

        let (code, _, stderr) = run(&fixture, &["inline-compose", &file, "--codex", "-c", "x=y", "phase=2"]);

        assert_eq!(code, 0, "{stderr}");
        let launches = launches(&log);
        assert_eq!(launches.len(), 1, "{launches:?}");
        assert_only_the_switch_was_forwarded(&launches[0]);
        let prompt = &prompts(&log)[0];
        assert!(prompt.contains("Write phase 2."), "the launch input lacks the setter:\n{prompt}");
        let written = fs::read_to_string(&path).unwrap();
        assert!(written.contains("Agent wrote this."), "{written}");
        assert!(written.contains("phase: 1\n"), "the authored value must stay:\n{written}");
        assert!(!written.contains("phase: 2") && !written.contains("x=y"), "the overlay was persisted:\n{written}");
    }

    #[test]
    fn every_sequence_step_sees_the_setter_and_receives_only_the_switch() {
        let fixture = CliProcessFixture::named("setter-after-switch-sequence");
        let log = install(&fixture, "codex", "exit 0");
        doc(&fixture, "a.md", "---\nphase: 1\n---\nStep A phase {{ phase }}\n");
        doc(&fixture, "b.md", "---\nphase: 1\n---\nStep B phase {{ phase }}\n");
        let seq = doc(
            &fixture,
            "seq.md",
            "---\nsequence:\n  - name: a\n    prompt: a.md\n  - name: b\n    prompt: b.md\n---\nBody\n",
        );

        let (code, _, stderr) = run(&fixture, &["sequence", &seq, "--codex", "-c", "x=y", "phase=2"]);

        assert_eq!(code, 0, "{stderr}");
        let launches = launches(&log);
        assert_eq!(launches.len(), 2, "{launches:?}");
        for launch in &launches {
            assert_only_the_switch_was_forwarded(launch);
        }
        let prompts: Vec<String> = prompts(&log).iter().map(|prompt| prompt.trim().to_owned()).collect();
        assert_eq!(prompts, ["Step A phase 2", "Step B phase 2"]);
    }

    // ── Relaunches keep the original ownership ──

    #[test]
    fn a_retry_relaunches_with_the_setter_applied_and_the_switch_once() {
        let fixture = CliProcessFixture::named("setter-after-switch-retry");
        let log = install(&fixture, "codex", "[ \"$n\" = 0 ] && exit 1\nexit 0");
        let file = doc(
            &fixture,
            "plan.md",
            &format!("---\nphase: 1\nfailure:\n  stack:\n    - action: {{retry: 1}}\n{PROBE}---\nPhase {{{{ phase }}}}\n"),
        );

        let (code, _, stderr) = run(&fixture, &["compose", &file, "--codex", "-c", "x=y", "phase=2"]);

        assert_eq!(code, 0, "{stderr}");
        let launches = launches(&log);
        assert_eq!(launches.len(), 2, "{launches:?}");
        for launch in &launches {
            assert_only_the_switch_was_forwarded(launch);
        }
        assert_eq!(prompts(&log).iter().map(|p| p.trim()).collect::<Vec<_>>(), ["Phase 2", "Phase 2"]);
        assert_eq!(probe(&fixture).trim(), "phase=2");
    }

    #[test]
    fn a_proxy_target_composes_with_the_setter_and_gets_the_switch_once() {
        let fixture = CliProcessFixture::named("setter-after-switch-proxy");
        let log = install(&fixture, "codex", "[ \"$n\" = 0 ] && exit 1\nexit 0");
        doc(&fixture, "target.md", &format!("---\nphase: 1\n{PROBE}---\nTarget phase {{{{ phase }}}}\n"));
        let file = doc(
            &fixture,
            "plan.md",
            "---\nphase: 1\nfailure:\n  stack:\n    - action: {proxy: './target.md'}\n---\nPhase {{ phase }}\n",
        );

        let (code, _, stderr) = run(&fixture, &["compose", &file, "--codex", "-c", "x=y", "phase=2"]);

        assert_eq!(code, 0, "{stderr}");
        let launches = launches(&log);
        assert_eq!(launches.len(), 2, "{launches:?}");
        for launch in &launches {
            assert_only_the_switch_was_forwarded(launch);
        }
        assert_eq!(
            prompts(&log).iter().map(|p| p.trim()).collect::<Vec<_>>(),
            ["Phase 2", "Target phase 2"]
        );
        assert_eq!(probe(&fixture).trim(), "phase=2");
    }

    #[test]
    fn a_resume_keeps_the_setter_and_resends_the_switch_once() {
        let fixture = CliProcessFixture::named("setter-after-switch-resume");
        let log = install(&fixture, "codex", RESUMABLE_CODEX);
        let file = doc(
            &fixture,
            "plan.md",
            &format!(
                "---\nphase: 1\nfailure:\n  stack:\n    - action: {{resume: 'continue'}}\n{PROBE}---\nPhase {{{{ phase }}}}\n"
            ),
        );

        let (code, _, stderr) = run(&fixture, &["compose", &file, "--codex", "-c", "x=y", "phase=2"]);

        assert_eq!(code, 0, "{stderr}");
        let launches = launches(&log);
        assert_eq!(launches.len(), 2, "{launches:?}\n{stderr}");
        assert_eq!(occurrences(&launches[1], &["resume", "thread-7"]), 1, "{launches:?}");
        for launch in &launches {
            assert_only_the_switch_was_forwarded(launch);
        }
        assert_eq!(probe(&fixture).trim(), "phase=2", "the resumed run lost the setter");
    }

    // ── Failures launch nothing ──

    /// Runs each refused command against `fixture`, whose only provider was
    /// installed with its record directory at `log`, and asserts it exits
    /// non-zero before any spawn and any lifecycle, with every `fields`
    /// substring on stderr, no setter value echoed, and nothing on stdout.
    fn assert_refused(fixture: &CliProcessFixture, log: &std::path::Path, cases: &[(&str, Vec<&str>, &[&str])]) {
        for (label, args, fields) in cases {
            let (code, stdout, stderr) = run(fixture, args);
            let flat = stderr.replace('┃', " ").split_whitespace().collect::<Vec<_>>().join(" ");

            assert_ne!(code, 0, "{label}: {flat}");
            for field in *fields {
                assert!(flat.contains(field), "{label}: missing {field:?} in {flat}");
            }
            assert!(!flat.contains("phase=2"), "{label}: the setter value was echoed: {flat}");
            assert!(stdout.trim().is_empty(), "{label}: stdout: {stdout}");
        }
        assert!(launches(log).is_empty(), "nothing may be spawned: {:?}", launches(log));
        assert_eq!(probe(fixture), "", "no lifecycle may run");
    }

    #[test]
    fn refused_codex_ownership_launches_nothing() {
        let fixture = CliProcessFixture::named("setter-after-switch-refused-codex");
        let log = install(&fixture, "codex", "exit 0");
        let declared = doc(&fixture, "declared.md", &format!("---\n$schema:\n  phase: string\n{PROBE}---\nBody\n"));
        let templated = doc(&fixture, "templated.md", &format!("---\n$schema: \"{{{{ env.OWN_SCHEMA }}}}\"\n{PROBE}---\nBody\n"));
        let missing_value: &[&str] = &["`-c` takes a value for Codex", "the `phase` setter after it", "separate provider value", "`--`"];

        assert_refused(
            &fixture,
            &log,
            &[
                ("declared setter leaves -c empty", vec!["compose", &declared, "--codex", "-c", "phase=2"], missing_value),
                ("a later word never reattaches", vec!["compose", &declared, "--codex", "-c", "phase=2", "x=y"], missing_value),
                (
                    "unestablished schema",
                    vec!["compose", &templated, "--codex", "-c", "phase=2"],
                    &["cannot tell whether `phase=…` after `-c`", "--set", "`--`"],
                ),
            ],
        );
    }

    /// No schema claims `x`, so the `[claude, codex]` union forwards `x=y`
    /// with `-c` (Codex takes a string) and `phase=2` is a setter; the run
    /// resolves to Claude, the only installed candidate, whose `-c` takes no
    /// value, so it fails before the spawn rather than rerouting `x=y`.
    #[test]
    fn refused_claude_resolution_launches_nothing() {
        let fixture = CliProcessFixture::named("setter-after-switch-refused-claude");
        let log = install(&fixture, "claude", "exit 0");
        let union = doc(
            &fixture,
            "union.md",
            &format!("---\nagent: [claude, codex]\n$schema:\n  phase: string\n{PROBE}---\nBody\n"),
        );

        assert_refused(
            &fixture,
            &log,
            &[(
                "resolved to Claude",
                vec!["compose", &union, "-c", "x=y", "phase=2"],
                &["`-c` takes no further value for Claude", "`x=y`", "`--`"],
            )],
        );
    }

    // ── The `--` escape hatch and setter typing ──

    #[test]
    fn after_the_separator_setter_shaped_words_are_provider_data() {
        let fixture = CliProcessFixture::named("setter-after-switch-separator");
        let log = install(&fixture, "codex", "exit 0");
        let file = doc(
            &fixture,
            "plan.md",
            &format!("---\nphase: 1\n$schema:\n  phase: number\n{PROBE}---\nPhase {{{{ phase }}}}\n"),
        );

        let (code, _, stderr) = run(&fixture, &["compose", &file, "--codex", "--", "-c", "x=y", "phase=2"]);

        assert_eq!(code, 0, "{stderr}");
        let launch = &launches(&log)[0];
        assert_eq!(occurrences(launch, &["-c", "x=y", "phase=2"]), 1, "{launch:?}");
        assert!(!contains(launch, "--"), "the separator is consumed: {launch:?}");
        assert_eq!(prompts(&log)[0].trim(), "Phase 1", "a word after `--` is never a setter");
        assert_eq!(probe(&fixture).trim(), "phase=1");
    }

    /// Reclaimed setters keep their JSON5 types, the last occurrence wins
    /// across the switch, and shorthand beats `--set` wherever `--set` sits
    /// (`--set` is accepted once, so each placement is its own run).
    #[test]
    fn reclaimed_setters_keep_types_and_precedence_end_to_end() {
        let fixture = CliProcessFixture::named("setter-after-switch-typing");
        let log = install(&fixture, "codex", "exit 0");
        let file = doc(
            &fixture,
            "plan.md",
            "---\nphase: 1\n---\nnext={{ count + 1 }} enabled={{ enabled == true }} empty=[{{ empty }}] label={{ label }} phase={{ phase }}\n",
        );
        let set = r#"{"phase": 9}"#;
        let tail = ["--codex", "-c", "count=3", "count=3", "enabled=true", "empty=", "label=a=b"];

        let mut set_before = vec!["compose", file.as_str(), "phase=7", "--set", set];
        set_before.extend(tail);
        set_before.push("phase=2");
        let mut set_after = vec!["compose", file.as_str(), "phase=7"];
        set_after.extend(tail);
        set_after.extend(["--set", set, "phase=2"]);

        for (run_index, args) in [set_before, set_after].iter().enumerate() {
            let (code, _, stderr) = run(&fixture, args);

            assert_eq!(code, 0, "{args:?}: {stderr}");
            let launch = &launches(&log)[run_index];
            assert_eq!(occurrences(launch, &["-c", "count=3"]), 1, "the switch value stays a string: {launch:?}");
            assert_eq!(launch.iter().filter(|arg| *arg == "count=3").count(), 1, "{launch:?}");
            for setter in ["enabled=true", "empty=", "label=a=b", "phase=2", "phase=7", set] {
                assert!(!contains(launch, setter), "{setter} was forwarded: {launch:?}");
            }
            assert_eq!(
                prompts(&log)[run_index].trim(),
                "next=4 enabled=true empty=[] label=a=b phase=2",
                "{args:?}"
            );
        }
    }
}
