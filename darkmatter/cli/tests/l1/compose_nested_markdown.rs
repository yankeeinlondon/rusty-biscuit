//! `as_markdown(content)` through the normal `md compose` path: reference
//! validation, shell-command preflight approval, and the terminal compose.

use crate::common;

use common::CliProcessFixture;
use predicates::prelude::*;

/// AC15: nested content reads the root's `ctx` and transcludes relative to the
/// root document.
#[test]
fn compose_nested_markdown_shares_context_and_root_relative_transclusion() {
    let fixture = CliProcessFixture::named("compose_nested_markdown_shares_context");
    fixture.write_file("cwd/docs/part.md", "NESTED PART CONTENT\n");
    let document = fixture.write_file(
        "cwd/docs/root.md",
        "root-id={{ ctx.id }}\n\n{{ as_markdown(\"nested-id={{ ctx.id }}\\n\\n::file ./part.md\") }}\n",
    );

    let output = fixture.command().arg("compose").arg(&document).output().unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value = |name: &str| {
        stdout
            .lines()
            .find_map(|line| line.trim().strip_prefix(name))
            .unwrap_or_else(|| panic!("missing {name}: {stdout}"))
            .to_string()
    };
    assert!(!value("root-id=").is_empty(), "{stdout}");
    assert_eq!(value("nested-id="), value("root-id="));
    assert!(stdout.contains("NESTED PART CONTENT"), "{stdout}");
}

/// AC32: preflight approval discovers a nested command without composing it,
/// and the command runs as often as the same command written in the body.
#[cfg(unix)]
#[test]
fn compose_approves_and_runs_a_nested_shell_command() {
    let fixture = CliProcessFixture::named("compose_approves_and_runs_a_nested_shell_command");
    let log = fixture.workspace_path().join("runs.log");
    common::write_executable(
        &fixture.bin_dir().join("record-run"),
        &format!("#!/bin/sh\necho \"$1\" >> '{}'\necho recorded-$1\n", log.display()),
    );
    let document = fixture.write_file(
        "cwd/root.md",
        "::shell record-run body\n\n{{ as_markdown(\"::shell record-run nested\") }}\n",
    );
    fixture.write_file("cwd/.darkmatter-shell-whitelist", "prefix record-run\n");

    fixture
        .command()
        .arg("compose")
        .arg(&document)
        .assert()
        .success()
        .stdout(predicate::str::contains("recorded-body").and(predicate::str::contains("recorded-nested")));

    let runs = std::fs::read_to_string(&log).unwrap();
    let count = |name: &str| runs.lines().filter(|line| *line == name).count();
    assert!(count("nested") >= 1, "{runs}");
    assert_eq!(count("nested"), count("body"), "{runs}");
}

/// AC32: nested content whose shape waits on frontmatter shell expansion is
/// rejected before any command runs.
#[cfg(unix)]
#[test]
fn compose_rejects_dynamic_nested_content_before_execution() {
    let fixture = CliProcessFixture::named("compose_rejects_dynamic_nested_content_before_execution");
    let sentinel = fixture.workspace_path().join("nested-ran");
    let document = fixture.write_file(
        "cwd/root.md",
        &format!(
            "---\npart: \"$(printf '::shell touch {}')\"\n---\n{{{{ as_markdown(part) }}}}\n",
            sentinel.display()
        ),
    );
    fixture.write_file("cwd/.darkmatter-shell-whitelist", "prefix printf\nprefix touch\n");

    fixture
        .command_builder()
        // `printf` and `touch` are the approved commands under test.
        .host_path()
        .build()
        .arg("compose")
        .arg(&document)
        .assert()
        .failure()
        .stderr(predicate::str::contains("dynamic command shape").and(predicate::str::contains("frontmatter.part")));

    assert!(!sentinel.exists(), "no command may run before the rejection");
}

/// AC16 through the CLI: mutual `as_markdown`/`::file` recursion ends in an
/// error, not a hang.
#[test]
fn compose_reports_mixed_recursion() {
    let fixture = CliProcessFixture::named("compose_reports_mixed_recursion");
    fixture.write_file("cwd/b.md", "{{ as_markdown(\"::file ./a.md\") }}\n");
    let document = fixture.write_file("cwd/a.md", "{{ as_markdown(\"::file ./b.md\") }}\n");

    fixture
        .command()
        .arg("compose")
        .arg(&document)
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .failure()
        .stderr(predicate::str::contains("ycle"));
}
