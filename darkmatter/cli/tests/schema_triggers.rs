//! CLI coverage for repository-scoped trigger schemas.

mod common;

use common::CliProcessFixture;
use predicates::prelude::*;
use std::path::{Path, PathBuf};

fn write(root: &Path, relative: &str, content: &str) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, content).unwrap();
    path
}

fn initialize_repository(root: &Path) {
    let git_dir = root.join(".git");
    std::fs::create_dir_all(git_dir.join("objects")).unwrap();
    std::fs::create_dir_all(git_dir.join("refs/heads")).unwrap();
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(
        git_dir.join("config"),
        "[core]\nrepositoryformatversion = 0\nbare = false\n",
    )
    .unwrap();
}

fn fixture(process: &CliProcessFixture, document_frontmatter: &str) -> PathBuf {
    let root = process.cwd();
    initialize_repository(root);
    write(
        root,
        "schemas/prompt.trigger.yaml",
        "kind: trigger-schema\nmatch:\n  kind: enum(prompt; required)\n$schema: prompt.yaml\n",
    );
    write(
        root,
        "schemas/prompt.yaml",
        "$schema:\n  owner: string(required)\n",
    );
    write(
        root,
        "docs/prompt.md",
        &format!("---\n{document_frontmatter}\n---\nBody\n"),
    )
}

fn dialect_fixture(process: &CliProcessFixture) {
    let root = process.cwd();
    initialize_repository(root);
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/schema-triggers");
    for directory in ["schemas", "docs"] {
        for entry in std::fs::read_dir(source.join(directory)).unwrap() {
            let entry = entry.unwrap();
            write(
                root,
                &format!("{directory}/{}", entry.file_name().to_string_lossy()),
                &std::fs::read_to_string(entry.path()).unwrap(),
            );
        }
    }
}

#[test]
fn schema_validate_honors_triggers_and_raw_mode() {
    let process = CliProcessFixture::new();
    let document = fixture(&process, "kind: prompt");
    process
        .command()
        .args(["schema", "validate"])
        .arg(&document)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("owner"));
    process
        .command()
        .args(["schema", "validate", "--no-trigger-schemas"])
        .arg(&document)
        .assert()
        .success();
}

#[test]
fn schema_validate_re_resolves_after_assignments() {
    let process = CliProcessFixture::new();
    let document = fixture(&process, "kind: note");
    process
        .command()
        .args(["schema", "validate"])
        .arg(&document)
        .arg("kind=prompt")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("owner"));
}

#[test]
fn compose_honors_triggers_and_raw_mode() {
    let process = CliProcessFixture::new();
    let document = fixture(&process, "kind: prompt");
    process
        .command()
        .arg("compose")
        .arg(&document)
        .assert()
        .failure()
        .stderr(predicate::str::contains("owner"));
    process
        .command()
        .args(["compose", "--no-trigger-schemas"])
        .arg(&document)
        .assert()
        .success();
}

#[test]
fn compose_re_resolves_trigger_after_shell_value_becomes_concrete() {
    let process = CliProcessFixture::new();
    let document = fixture(&process, "kind: $(echo prompt)");
    write(
        process.cwd(),
        "docs/.darkmatter-shell-whitelist",
        "exact echo prompt\n",
    );
    // `$(echo prompt)` is executed as a program, and on Windows `echo` exists
    // only as Git's echo.exe outside System32, so the host PATH is declared.
    process
        .command_builder()
        .host_path()
        .build()
        .arg("compose")
        .arg(&document)
        .assert()
        .failure()
        .stderr(predicate::str::contains("owner"));
}

#[test]
fn triggers_command_prints_shared_trace() {
    let process = CliProcessFixture::new();
    let document = fixture(&process, "kind: note");
    process
        .command()
        .args(["schema", "triggers"])
        .arg(&document)
        .assert()
        .success()
        .stdout(predicate::str::contains("Schema roots"))
        .stdout(predicate::str::contains("prompt.trigger.yaml"))
        .stdout(predicate::str::contains("arm 1"))
        .stdout(predicate::str::contains("defeated"));
}

#[test]
fn sibling_only_bare_reference_suggests_explicit_relative_path() {
    let process = CliProcessFixture::new();
    initialize_repository(process.cwd());
    write(
        process.cwd(),
        "schemas/placeholder.yaml",
        "$schema:\n  title: string\n",
    );
    write(
        process.cwd(),
        "docs/local.yaml",
        "$schema:\n  title: string(required)\n",
    );
    let document = write(
        process.cwd(),
        "docs/doc.md",
        "---\n$schema: local.yaml\ntitle: Test\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(document)
        .assert()
        .code(2)
        .stdout(predicate::str::contains("./local.yaml"));
}

#[test]
fn dialect_family_has_identical_compose_validate_and_trace_activation() {
    let process = CliProcessFixture::new();
    dialect_fixture(&process);
    for (document, trigger, required) in [
        ("claudine.md", "claudine.trigger.yaml", "provider"),
        ("inline-compose.md", "inline-compose.trigger.yaml", "output"),
        ("sequence.md", "sequence.trigger.yaml", "sequence_name"),
    ] {
        let path = process.cwd().join("docs").join(document);
        process
            .command()
            .args(["schema", "validate"])
            .arg(&path)
            .assert()
            .code(1)
            .stdout(predicate::str::contains(required));
        process
            .command()
            .arg("compose")
            .arg(&path)
            .assert()
            .failure()
            .stderr(predicate::str::contains(required));
        process
            .command()
            .args(["schema", "triggers"])
            .arg(&path)
            .assert()
            .success()
            .stdout(predicate::str::contains(trigger))
            .stdout(predicate::str::contains("matched"));
    }

    let plain = process.cwd().join("docs/plain.md");
    process
        .command()
        .args(["schema", "validate"])
        .arg(&plain)
        .assert()
        .success();
    process
        .command()
        .arg("compose")
        .arg(&plain)
        .assert()
        .success();
}
