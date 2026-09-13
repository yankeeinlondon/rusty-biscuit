mod common;

use common::CliProcessFixture;
use predicates::prelude::*;

#[test]
fn test_compose_scalar_ctx_without_allow_override_fails() {
    let fixture = CliProcessFixture::named("test_compose_scalar_ctx_without_allow_override_fails");
    fixture
        .command()
        .args(["compose", "-", "--no-baseline-schema"])
        .write_stdin("---\nctx: hello\n---\n# Test {{ ctx.today }}")
        .assert()
        .failure()
        .stderr(predicate::str::contains("CtxMergeError"))
        .stderr(predicate::str::contains("JSON object"));
}

#[test]
fn test_compose_scalar_ctx_with_allow_override_succeeds() {
    let fixture = CliProcessFixture::named("test_compose_scalar_ctx_with_allow_override_succeeds");
    // --allow-ctx-override downgrades the error to a warning
    fixture
        .command()
        .args([
            "compose",
            "-",
            "--allow-ctx-override",
            "--no-baseline-schema",
        ])
        .write_stdin("---\nctx: hello\n---\n# Test")
        .assert()
        .success()
        .stderr(predicate::str::contains("Document ctx was not an object"));
}

#[test]
fn test_compose_object_ctx_collision_emits_warning() {
    let fixture = CliProcessFixture::named("test_compose_object_ctx_collision_emits_warning");
    // A document with an object ctx that collides with runtime keys should
    // succeed but emit a collision warning on stderr.
    fixture
        .command()
        .args(["compose", "-", "--no-baseline-schema"])
        .write_stdin("---\nctx:\n  today: custom-value\n---\n# Test")
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "conflict with those provided by Darkmatter",
        ));
}

/// - emits the error type name (`TransclusionError`) on stderr,
/// - emits a human-readable summary (`cycle detected`),
/// - emits a hint-tagged token from the rendered block.
#[test]
fn test_block_rendering_transclusion_cycle_tty() {
    let fixture = CliProcessFixture::named("test_block_rendering_transclusion_cycle_tty");
    assert!(fixture.initialize_repository());
    let a = fixture.write_file("cwd/a.md", "# A\n\n::file ./b.md\n");
    fixture.write_file("cwd/b.md", "# B\n\n::file ./a.md\n");

    fixture
        .command()
        .arg("compose")
        .arg(&a)
        .assert()
        .failure()
        .stderr(predicate::str::contains("TransclusionError"))
        .stderr(predicate::str::contains("cycle detected"))
        .stderr(predicate::str::contains("Break the cycle"));
}

/// Non-TTY block rendering: the same cycle error must still produce
/// readable plain text (optimistic 80-column render) when stderr is
/// piped. `assert_cmd` runs commands with piped stdio by default, so
/// this test naturally exercises the non-TTY branch in `main.rs`.
#[test]
fn test_block_rendering_transclusion_cycle_non_tty() {
    let fixture = CliProcessFixture::named("test_block_rendering_transclusion_cycle_non_tty");
    assert!(fixture.initialize_repository());
    let a = fixture.write_file("cwd/a.md", "# A\n\n::file ./b.md\n");
    fixture.write_file("cwd/b.md", "# B\n\n::file ./a.md\n");

    let output = fixture.command().arg("compose").arg(&a).output().unwrap();

    assert!(!output.status.success(), "expected non-zero exit code");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cycle detected"),
        "stderr should contain human-readable summary in non-TTY mode\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Break the cycle"),
        "stderr should contain hint from rendered block in non-TTY mode\nstderr:\n{stderr}"
    );
}
