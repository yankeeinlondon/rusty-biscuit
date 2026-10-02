//! The forwarding notice, tail redaction, and non-UTF-8 refusal, observed
//! through the compiled binary for composition and the direct wrappers.

use std::ffi::OsString;
use std::fs;

use crate::common;
use common::{CliProcessFixture, strip_ansi};
#[cfg(unix)]
use common::write_executable;

#[cfg(unix)]
const SECRET: &str = "sk-proj-tailsecret0123456789";

/// Install a fake `codex` that records each launch's arguments in its own
/// file under `launches/` in the fixture home (parallel tasks must not
/// interleave), `\x1f` separated because an argument may span lines.
#[cfg(unix)]
fn recording_codex(fixture: &CliProcessFixture) -> std::path::PathBuf {
    let dir = fixture.home().join("launches");
    fs::create_dir_all(&dir).unwrap();
    write_executable(
        &fixture.bin_dir().join("codex"),
        &format!(
            "#!/bin/sh\n\
             for arg in \"$@\"; do printf '%s\\037' \"$arg\"; done > '{dir}/launch-'$$\n\
             exit 0\n",
            dir = dir.display()
        ),
    );
    dir
}

/// Every launch recorded since the last call, each as its argument vector.
///
/// Records are removed as they are read, so a test that runs the binary
/// more than once reads each run's launches separately.
#[cfg(unix)]
fn launches(dir: &std::path::Path) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let record = fs::read_to_string(&path).unwrap();
        fs::remove_file(&path).unwrap();
        records.push(
            record
                .split('\u{1f}')
                .filter(|arg| !arg.is_empty())
                .map(str::to_owned)
                .collect(),
        );
    }
    records
}

/// How many times `run` appears contiguously in `args`.
#[cfg(unix)]
fn occurrences(args: &[String], run: &[&str]) -> usize {
    args.windows(run.len())
        .filter(|window| window.iter().zip(run).all(|(arg, want)| arg == want))
        .count()
}

#[cfg(unix)]
fn prompt_file(fixture: &CliProcessFixture) -> String {
    let md = fixture.cwd().join("plan.md");
    fs::write(&md, "---\ntitle: plan\n---\nPlan body\n").unwrap();
    md.to_str().unwrap().to_owned()
}

#[cfg(unix)]
fn run(fixture: &CliProcessFixture, args: &[&str]) -> (i32, String) {
    let output = fixture.command().args(args).output().unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    (output.status.code().unwrap_or(-1), stderr)
}

/// The forwarding notices in `stderr`, each unwrapped onto one line.
///
/// The status component word-wraps to the terminal width, so a notice is
/// read from its line up to the next blank line or status glyph.
#[cfg(unix)]
fn notice_lines(stderr: &str) -> Vec<String> {
    let flat = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    flat.split("Forwarding ")
        .skip(1)
        .map(|rest| {
            let end = rest.find(['ℹ', '✓', '■', '⚠']).unwrap_or(rest.len());
            format!("Forwarding {}", rest[..end].trim_end())
        })
        .collect()
}

#[cfg(unix)]
#[test]
fn compose_implicit_tail_notice_names_switches_and_forwards_tokens_unchanged() {
    let fixture = CliProcessFixture::named("tail-notice-implicit");
    let log = recording_codex(&fixture);
    let file = prompt_file(&fixture);

    let (code, stderr) = run(
        &fixture,
        &[
            "compose",
            "--codex",
            &file,
            "-c",
            "model_reasoning_effort=low",
            "--api-key",
            SECRET,
            "-csecretvalue",
        ],
    );

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let notices = notice_lines(&stderr);
    assert_eq!(notices.len(), 1, "stderr:\n{stderr}");
    assert!(
        notices[0].contains(
            "Forwarding provider arguments to Codex: -c, --api-key, a short switch with \
             attached text (not shown)"
        ),
        "{notices:?}"
    );
    for hidden in ["model_reasoning_effort", SECRET, "secretvalue", "recogni"] {
        assert!(!stderr.contains(hidden), "{hidden} leaked:\n{stderr}");
    }
    let launches = launches(&log);
    assert_eq!(launches.len(), 1, "{launches:?}");
    assert_eq!(
        occurrences(
            &launches[0],
            &["-c", "model_reasoning_effort=low", "--api-key", SECRET, "-csecretvalue"],
        ),
        1,
        "the child must receive the tail unchanged: {launches:?}"
    );
}

#[cfg(unix)]
#[test]
fn compose_explicit_and_mixed_tails_report_the_opaque_suffix() {
    let fixture = CliProcessFixture::named("tail-notice-explicit");
    let log = recording_codex(&fixture);
    let file = prompt_file(&fixture);

    let (code, stderr) = run(&fixture, &["compose", "--codex", &file, "--", "-c", "opaque-value"]);
    assert_eq!(code, 0, "stderr:\n{stderr}");
    let explicit_launches = launches(&log);
    assert_eq!(occurrences(&explicit_launches[0], &["-c", "opaque-value"]), 1);
    let notices = notice_lines(&stderr);
    assert_eq!(notices.len(), 1, "stderr:\n{stderr}");
    assert!(
        notices[0].contains("Forwarding an opaque argument tail to Codex (passed after --)."),
        "{notices:?}"
    );
    assert!(!notices[0].contains("-c"), "{notices:?}");
    assert!(!stderr.contains("opaque-value"), "stderr:\n{stderr}");

    let (code, stderr) = run(
        &fixture,
        &["compose", "--codex", &file, "-c", "x=y", "--", "--native", "z"],
    );
    assert_eq!(code, 0, "stderr:\n{stderr}");
    let notices = notice_lines(&stderr);
    assert_eq!(notices.len(), 1, "stderr:\n{stderr}");
    assert!(
        notices[0].contains(
            "Forwarding provider arguments to Codex: -c, followed by an opaque argument tail \
             (passed after --)."
        ),
        "{notices:?}"
    );
    assert!(!notices[0].contains("--native"), "{notices:?}");

    let mixed_launches = launches(&log);
    assert_eq!(
        occurrences(&mixed_launches[0], &["-c", "x=y", "--native", "z"]),
        1,
        "{mixed_launches:?}"
    );
}

#[cfg(unix)]
#[test]
fn quiet_and_silent_suppress_the_notice_but_not_forwarding() {
    for switch in ["--quiet", "--silent"] {
        let fixture = CliProcessFixture::named("tail-notice-quiet");
        let log = recording_codex(&fixture);
        let file = prompt_file(&fixture);

        let (code, stderr) = run(&fixture, &["compose", "--codex", switch, &file, "-c", "x=y"]);

        assert_eq!(code, 0, "{switch} stderr:\n{stderr}");
        assert!(notice_lines(&stderr).is_empty(), "{switch} stderr:\n{stderr}");
        assert_eq!(occurrences(&launches(&log)[0], &["-c", "x=y"]), 1, "{switch}");
    }
}

#[cfg(unix)]
#[test]
fn sequence_steps_and_parallel_tasks_share_one_notice_per_pair() {
    let fixture = CliProcessFixture::named("tail-notice-sequence");
    let log = recording_codex(&fixture);
    fs::write(fixture.cwd().join("a.md"), "---\ntitle: a\n---\nTask A\n").unwrap();
    fs::write(fixture.cwd().join("b.md"), "---\ntitle: b\n---\nTask B\n").unwrap();
    let md = fixture.cwd().join("seq.md");
    fs::write(
        &md,
        r#"---
sequence:
  - step_one
  - step_two
  - name: fan
    group:
      name: fan
      execution: parallel
      tasks:
        - name: a
          prompt: a.md
        - name: b
          prompt: b.md
---
Step body
"#,
    )
    .unwrap();

    let (code, stderr) = run(
        &fixture,
        &["sequence", "--codex", md.to_str().unwrap(), "-c", "x=y"],
    );

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let launches = launches(&log);
    assert_eq!(launches.len(), 4, "{launches:?}\nstderr:\n{stderr}");
    for launch in &launches {
        assert_eq!(occurrences(launch, &["-c", "x=y"]), 1, "{launches:?}");
    }
    let notices = notice_lines(&stderr);
    assert_eq!(notices.len(), 1, "stderr:\n{stderr}");
    assert!(
        notices[0].contains("Forwarding provider arguments to Codex: -c"),
        "{notices:?}"
    );
}

/// `stderr` with ANSI removed and every run of whitespace collapsed, so a
/// word-wrapped sentence reads as one line.
#[cfg(unix)]
fn flattened(stderr: &str) -> String {
    stderr.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Each implicit switch is explained from the compiled Codex catalog at the
/// `exec` command the launch uses: a researched one by what it is, an
/// unresearched one as forwarded anyway. Both launch paths read the same
/// catalog, and the child still receives every token unchanged.
#[cfg(unix)]
#[test]
fn each_forwarded_switch_is_explained_from_the_compiled_catalog() {
    let fixture = CliProcessFixture::named("tail-notice-explained");
    let log = recording_codex(&fixture);
    let file = prompt_file(&fixture);
    let unrecognized = "--frobnicate: Claudine's compiled Codex switch catalog has no established \
                        type for it at its `exec` command; Claudine forwards it anyway.";

    let (code, stderr) = run(
        &fixture,
        &["compose", "--codex", &file, "-c", "model_reasoning_effort=low", "--frobnicate"],
    );
    assert_eq!(code, 0, "stderr:\n{stderr}");
    let flat = flattened(&stderr);
    assert!(flat.contains("-c is Codex's --config switch ("), "{flat}");
    assert!(flat.contains("); forwarding to Codex."), "{flat}");
    assert!(flat.contains(unrecognized), "{flat}");
    for hidden in ["model_reasoning_effort", "reject", "recogni"] {
        assert!(!flat.contains(hidden), "{hidden}: {flat}");
    }
    assert_eq!(
        occurrences(&launches(&log)[0], &["-c", "model_reasoning_effort=low", "--frobnicate"]),
        1
    );

    let (code, stderr) = run(&fixture, &["codex", "-c", "x=y", "--frobnicate", "do the thing"]);
    assert_eq!(code, 0, "stderr:\n{stderr}");
    let flat = flattened(&stderr);
    assert!(flat.contains("-c is Codex's --config switch ("), "{flat}");
    assert!(flat.contains(unrecognized), "{flat}");
    assert!(!flat.contains("x=y"), "{flat}");
    assert_eq!(occurrences(&launches(&log)[0], &["-c", "x=y", "--frobnicate"]), 1);

    // Quiet output drops the explanations with the notice.
    let (code, stderr) = run(&fixture, &["codex", "--quiet", "-c", "x=y", "do the thing"]);
    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert!(!flattened(&stderr).contains("--config switch"), "{stderr}");
}

#[cfg(unix)]
#[test]
fn direct_wrapper_announces_the_shared_notice() {
    let fixture = CliProcessFixture::named("tail-notice-direct");
    let log = recording_codex(&fixture);

    let (code, stderr) = run(&fixture, &["codex", "-c", "x=y", "do the thing"]);

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let notices = notice_lines(&stderr);
    assert_eq!(notices.len(), 1, "stderr:\n{stderr}");
    assert!(
        notices[0].contains("Forwarding provider arguments to Codex: -c"),
        "{notices:?}"
    );
    assert!(!notices[0].contains("x=y"), "{notices:?}");
    assert_eq!(occurrences(&launches(&log)[0], &["-c", "x=y"]), 1);

    let (code, stderr) = run(&fixture, &["codex", "--quiet", "-c", "x=y", "do the thing"]);
    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert!(notice_lines(&stderr).is_empty(), "stderr:\n{stderr}");

    // A prompt alone forwards nothing, so there is nothing to announce.
    let (code, stderr) = run(&fixture, &["codex", "do the thing"]);
    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert!(notice_lines(&stderr).is_empty(), "stderr:\n{stderr}");
}

#[cfg(unix)]
#[test]
fn direct_wrapper_reports_the_suffix_after_its_consumed_separator_as_opaque() {
    let fixture = CliProcessFixture::named("tail-notice-direct-explicit");
    let log = recording_codex(&fixture);

    let (code, stderr) = run(
        &fixture,
        &["codex", "-c", "x=y", "do the thing", "--", "--native"],
    );

    assert_eq!(code, 0, "stderr:\n{stderr}");
    let notices = notice_lines(&stderr);
    assert_eq!(notices.len(), 1, "stderr:\n{stderr}");
    assert!(
        notices[0].contains(
            "Forwarding provider arguments to Codex: -c, followed by an opaque argument tail"
        ),
        "{notices:?}"
    );
    assert_eq!(occurrences(&launches(&log)[0], &["--native"]), 1);
}

#[cfg(unix)]
#[test]
fn dry_run_and_debug_traces_redact_tail_secrets() {
    let fixture = CliProcessFixture::named("tail-redaction");
    let log = recording_codex(&fixture);
    let file = prompt_file(&fixture);

    // Composition dry run: the "Provider args" row is redacted.
    let (code, stderr) = run(
        &fixture,
        &["compose", "--codex", "--dry-run", &file, "--api-key", SECRET, "--token=sk-ant-other99"],
    );
    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert!(stderr.contains("Provider args"), "stderr:\n{stderr}");
    assert!(stderr.contains("--api-key **** --token=****"), "stderr:\n{stderr}");
    assert!(!stderr.contains(SECRET) && !stderr.contains("sk-ant-other99"), "{stderr}");

    // Direct-wrapper dry run: the full command line is redacted. The
    // attached form keeps the secret from being read as the prompt.
    let attached = format!("--api-key={SECRET}");
    let (code, stderr) = run(&fixture, &["codex", "--dry-run", &attached, "prompt"]);
    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert!(stderr.contains("Command:"), "stderr:\n{stderr}");
    assert!(!stderr.contains(SECRET), "stderr:\n{stderr}");

    // Debug traces of the provider argv, on both launch paths. The stderr
    // formatter prints only selected fields today, so this guards the
    // stderr surface; the argv field itself is redacted at the call site
    // for any subscriber that does print it.
    for args in [
        vec!["compose", "--codex", "--yolo", file.as_str(), "--api-key", SECRET],
        vec!["codex", "--yolo", attached.as_str(), "prompt"],
    ] {
        let output = fixture
            .command()
            .env("RUST_LOG", "claudine=debug")
            .args(&args)
            .output()
            .unwrap();
        let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
        assert!(output.status.success(), "{args:?} stderr:\n{stderr}");
        assert!(stderr.contains("yolo applied to provider argv"), "{args:?}:\n{stderr}");
        assert!(!stderr.contains(SECRET), "{args:?} leaked into a trace:\n{stderr}");
    }
    assert!(
        launches(&log)
            .iter()
            .all(|launch| launch.iter().any(|arg| arg.contains(SECRET))),
        "the child still receives the secret"
    );
}

// ── Non-UTF-8 refusal: composition and direct wrappers agree ──

#[cfg(unix)]
fn invalid_token() -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(vec![b'v', 0xFF, b'x'])
}

#[cfg(windows)]
fn invalid_token() -> OsString {
    use std::os::windows::ffi::OsStringExt;
    // An unpaired surrogate is a valid Windows argument and invalid UTF-8.
    OsString::from_wide(&[0x0076, 0xD800, 0x0078])
}

#[test]
fn composition_refuses_a_non_utf8_tail_token_without_launching() {
    let fixture = CliProcessFixture::named("tail-non-utf8");
    let md = fixture.cwd().join("plan.md");
    fs::write(&md, "---\ntitle: plan\n---\nPlan body\n").unwrap();

    for explicit in [false, true] {
        let mut command = fixture.command();
        command.args(["compose", "--codex", md.to_str().unwrap(), "-c"]);
        if explicit {
            command.arg("--");
        }
        let output = command.arg(invalid_token()).output().unwrap();
        let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

        assert!(!output.status.success(), "stderr:\n{stderr}");
        // `-c` is forwarded argument 1 either way; the bad token is 2.
        assert!(
            stderr.contains("provider argument 2 "),
            "explicit={explicit} stderr:\n{stderr}"
        );
        assert!(stderr.contains("not valid UTF-8"), "stderr:\n{stderr}");
        assert!(!stderr.contains('\u{FFFD}'), "the bytes must not be echoed:\n{stderr}");
    }
}

#[test]
fn direct_wrapper_also_refuses_non_utf8_passthrough() {
    let fixture = CliProcessFixture::named("tail-non-utf8-direct");

    let output = fixture
        .command()
        .args(["codex", "-c"])
        .arg(invalid_token())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(stderr.to_lowercase().contains("utf-8"), "stderr:\n{stderr}");
}
