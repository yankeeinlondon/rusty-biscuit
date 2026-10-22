//! Darkmatter's spellings of the events Claudine renamed (`composed`, `failed`)
//! are refused when Claudine parses a document's lifecycle, naming Claudine's
//! spelling, before any lifecycle effect or provider launch.

use crate::common;

use common::{CliProcessFixture, strip_ansi, write};

fn stderr(output: &std::process::Output) -> String {
    strip_ansi(&String::from_utf8_lossy(&output.stderr))
}

#[test]
fn compose_dry_run_refuses_a_composed_key_naming_start() {
    let fixture = CliProcessFixture::named("renamed-event-dry-run");
    write(&fixture.cwd().join("doc.md"), "---\ncomposed:\n    - info: \"ready\"\n---\nBody.\n");

    let output = fixture
        .command()
        .args(["compose", "doc.md", "--dry-run", "--claude"])
        .output()
        .unwrap();

    let stderr = stderr(&output);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("renamed lifecycle event"), "{stderr}");
    assert!(stderr.contains("`start:`"), "{stderr}");
}

#[cfg(unix)]
#[test]
fn compose_refuses_a_failed_key_before_initialize_runs_or_a_provider_launches() {
    let fixture = CliProcessFixture::named("renamed-event-launch");
    // `append_line` resolves against the repository root, so the marker would
    // land in `cwd` if `initialize` ran.
    fixture.initialize_repository();
    let records = common::launch_recorder::install(&fixture, "claude", "exit 0");
    write(
        &fixture.cwd().join("doc.md"),
        "---\ninitialize:\n    - {append_line: ['events.log', 'initialize']}\n\
         failed:\n    - warn: \"no\"\n---\nBody.\n",
    );

    let output = fixture.command().args(["compose", "doc.md", "--claude"]).output().unwrap();

    let stderr = stderr(&output);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("`blocked:`"), "{stderr}");
    assert!(common::launch_recorder::launches(&records).is_empty(), "{stderr}");
    assert!(!fixture.cwd().join("events.log").exists(), "initialize ran:\n{stderr}");
}
