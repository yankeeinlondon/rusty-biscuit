//! Lifecycle text keeps Darkmatter's escape semantics through the real
//! compose path: `{{{ x }}}`, `\{{ x }}`, and `\{\{ x }}` each print the
//! literal `{{ x }}`, whether the escape sits in mixed text, is the whole
//! value, is stored by `set:` and read back, or is a top-level event field.
//! (Interpolation keeps a backslash escape as authored, as Darkmatter does; the
//! `stderr` channel's inline Markdown rendering is what drops the backslash.)
//!
//! The lifecycle keys are excluded from compose's first frontmatter pass so
//! Claudine can evaluate them when the event fires; a test that hands the
//! executor JSON directly skips that pass, which is where an escape was once
//! lost.

use crate::common;
use common::{CliProcessFixture, strip_ansi, write, write_dry_run_provider_stub};

const DOCUMENT: &str = r#"---
title: probe
initialize:
  stack:
    - action: { stderr: "MIXED-1=[{{{ title }}}]" }
    - action: { stderr: "MIXED-2=[\\{{ title }}]" }
    - action: { stderr: "MIXED-3=[\\{\\{ title }}]" }
    - action: { stderr: "{{{ title }}}" }
    - action: { stderr: "\\{{ title }}" }
    - action: { stderr: "\\{\\{ title }}" }
    - action:
        set:
          stored_1: "{{{ title }}}"
          stored_2: "\\{{ title }}"
          stored_3: "\\{\\{ title }}"
    - action: { stderr: "SET-1=[{{ stored_1 }}] SET-2=[{{ stored_2 }}] SET-3=[{{ stored_3 }}]" }
start:
  stderr: "START-1=[{{{ title }}}] START-2=[\\{{ title }}] START-3=[\\{\\{ title }}]"
  stack:
    - action: { error: "halt before the provider" }
---
body
"#;

#[test]
fn every_escape_form_prints_literal_text_in_lifecycle_values() {
    let fixture = CliProcessFixture::named("lifecycle-literal-escapes");
    write_dry_run_provider_stub(fixture.bin_dir(), "claude");
    let document = fixture.cwd().join("probe.md");
    write(&document, DOCUMENT);

    let output = fixture
        .command()
        .args(["compose", "-y", "--claude"])
        .arg(&document)
        .output()
        .expect("claudine must spawn");

    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let lines: Vec<&str> = stderr.lines().map(str::trim).collect();
    let expected = [
        "MIXED-1=[{{ title }}]",
        "MIXED-2=[{{ title }}]",
        "MIXED-3=[{{ title }}]",
        "SET-1=[{{ title }}] SET-2=[{{ title }}] SET-3=[{{ title }}]",
        "START-1=[{{ title }}] START-2=[{{ title }}] START-3=[{{ title }}]",
    ];
    for line in expected {
        assert!(lines.contains(&line), "missing `{line}` in stderr:\n{stderr}");
    }
    let whole_values = lines.iter().filter(|line| **line == "{{ title }}").count();
    assert_eq!(whole_values, 3, "each whole-value escape prints literal text:\n{stderr}");
    assert!(!stderr.contains("probe]"), "an escape was evaluated:\n{stderr}");

    assert!(stderr.contains("halt before the provider"), "the run stops on the authored error:\n{stderr}");
    assert!(!output.status.success(), "an authored error fails the run");
    assert!(
        !stdout.contains("SHOULD NOT RUN") && !stderr.contains("SHOULD NOT RUN"),
        "the provider must not start"
    );
    assert!(!fixture.audio_spool().exists(), "no lifecycle audio is published");
}

/// A `proxy` `with:` value is evaluated in the source, inside a lifecycle key,
/// so its escape must reach the target as literal data.
#[test]
fn proxy_with_values_deliver_a_triple_brace_escape_as_literal_text() {
    let fixture = CliProcessFixture::named("lifecycle-literal-escapes-proxy");
    write_dry_run_provider_stub(fixture.bin_dir(), "claude");
    let source = fixture.cwd().join("source.md");
    write(
        &source,
        r#"---
title: probe
initialize:
  stack:
    - action:
        action: proxy
        target: "./target.md"
        with:
          whole: "{{{ title }}}"
          mixed: "MIXED=[{{{ title }}}]"
---
source body
"#,
    );
    write(
        &fixture.cwd().join("target.md"),
        r#"---
title: target
initialize:
  stack:
    - action: { stderr: "WHOLE=[{{ whole }}] {{ mixed }}" }
    - action: { error: "halt in the target" }
---
target body
"#,
    );

    let output = fixture
        .command()
        .args(["compose", "-y", "--claude"])
        .arg(&source)
        .output()
        .expect("claudine must spawn");

    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(
        stderr.lines().any(|line| line.trim() == "WHOLE=[{{ title }}] MIXED=[{{ title }}]"),
        "the overlay carries literal text:\n{stderr}"
    );
    assert!(stderr.contains("halt in the target"), "the run stops in the target:\n{stderr}");
    assert!(!output.status.success(), "an authored error fails the run");
    assert!(
        !stdout.contains("SHOULD NOT RUN") && !stderr.contains("SHOULD NOT RUN"),
        "the provider must not start"
    );
    assert!(!fixture.audio_spool().exists(), "no lifecycle audio is published");
}

/// A lifecycle `shell` command is resolved at pre-flight and runs its approved
/// bytes, so its escape must survive into the command as literal text.
#[cfg(unix)]
#[test]
fn lifecycle_shell_commands_run_a_triple_brace_escape_as_literal_text() {
    let fixture = CliProcessFixture::named("lifecycle-literal-escapes-shell");
    write_dry_run_provider_stub(fixture.bin_dir(), "claude");
    let document = fixture.cwd().join("probe.md");
    write(
        &document,
        r#"---
title: probe
start:
  stack:
    - action: { shell: "printf '%s' 'SHELL=[{{{ title }}}]' > shell-out.txt" }
    - action: { error: "halt before the provider" }
---
body
"#,
    );

    let output = fixture
        .command()
        .args(["compose", "-y", "--claude"])
        .arg(&document)
        .output()
        .expect("claudine must spawn");

    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let written = std::fs::read_to_string(fixture.cwd().join("shell-out.txt"))
        .unwrap_or_else(|error| panic!("the shell action must run ({error}):\n{stderr}"));
    assert_eq!(written, "SHELL=[{{ title }}]");
    assert!(!output.status.success(), "an authored error fails the run");
    assert!(
        !stdout.contains("SHOULD NOT RUN") && !stderr.contains("SHOULD NOT RUN"),
        "the provider must not start"
    );
}
