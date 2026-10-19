//! A forbidden action in the `cancelled` stack of a container nested three
//! levels deep is a parse error raised before anything runs: not the root
//! `initialize` or `blocked` stacks, and not the first task.

use std::fs;

use crate::common::{CliProcessFixture, strip_ansi};

fn assert_rejected_before_anything_runs(nested: &str) {
    let fixture = CliProcessFixture::named("sequence-deep-cancelled");
    let root = format!(concat!(
        "---\ninitialize: [{{ stdout: INITIALIZE-RAN }}]\nblocked: [{{ stdout: BLOCKED-RAN }}]\nsequence:\n",
        "  - {{ name: first, side_effect: {{ append_line: [marker.txt, ran] }} }}\n",
        "  - {}\n---\n",
    ), nested);
    fs::write(fixture.cwd().join("root.md"), root).unwrap();
    let output = fixture.command().args(["sequence", "--goose", "--yolo", "--silent"])
        .arg(fixture.cwd().join("root.md")).assert().failure().get_output().clone();
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(stderr.contains("`retry`") && stderr.contains("`cancelled`"), "{stderr}");
    assert!(!stdout.contains("INITIALIZE-RAN") && !stdout.contains("BLOCKED-RAN"), "{stdout}");
    assert!(!fixture.cwd().join("marker.txt").exists(), "the first task ran");
    assert!(!fixture.audio_spool().exists());
}

#[test]
fn innermost_group_cancelled_is_rejected_before_anything_runs() {
    assert_rejected_before_anything_runs(
        "{ name: a, sequence: [{ name: b, sequence: [{ name: c, group: { name: g, tasks: [{ name: leg, sequence: [] }], cancelled: [{ retry: 2 }] } }] }] }",
    );
}

#[test]
fn sequence_under_groups_cancelled_is_rejected_before_anything_runs() {
    assert_rejected_before_anything_runs(
        "{ name: a, group: { name: g1, tasks: [{ name: b, group: { name: g2, tasks: [{ name: c, sequence: [], cancelled: [{ retry: 2 }] }] } }] } }",
    );
}
