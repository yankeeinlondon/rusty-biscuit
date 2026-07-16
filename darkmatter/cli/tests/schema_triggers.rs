//! CLI coverage for repository-scoped trigger schemas.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn md_cmd() -> assert_cmd::Command {
    cargo_bin_cmd!("md")
}

fn write(root: &Path, relative: &str, content: &str) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, content).unwrap();
    path
}

fn fixture(document_frontmatter: &str) -> (TempDir, PathBuf) {
    let temp = TempDir::new().unwrap();
    std::fs::create_dir(temp.path().join(".git")).unwrap();
    write(
        temp.path(),
        "schemas/prompt.trigger.yaml",
        "kind: trigger-schema\nmatch:\n  kind: enum(prompt; required)\n$schema: prompt.yaml\n",
    );
    write(
        temp.path(),
        "schemas/prompt.yaml",
        "$schema:\n  owner: string(required)\n",
    );
    let document = write(
        temp.path(),
        "docs/prompt.md",
        &format!("---\n{document_frontmatter}\n---\nBody\n"),
    );
    (temp, document)
}

fn dialect_fixture() -> TempDir {
    let temp = TempDir::new().unwrap();
    std::fs::create_dir(temp.path().join(".git")).unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/schema-triggers");
    for directory in ["schemas", "docs"] {
        for entry in std::fs::read_dir(source.join(directory)).unwrap() {
            let entry = entry.unwrap();
            write(
                temp.path(),
                &format!("{directory}/{}", entry.file_name().to_string_lossy()),
                &std::fs::read_to_string(entry.path()).unwrap(),
            );
        }
    }
    temp
}

#[test]
fn schema_validate_honors_triggers_and_raw_mode() {
    let (_temp, document) = fixture("kind: prompt");
    md_cmd()
        .args(["schema", "validate"])
        .arg(&document)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("owner"));
    md_cmd()
        .args(["schema", "validate", "--no-trigger-schemas"])
        .arg(&document)
        .assert()
        .success();
}

#[test]
fn schema_validate_re_resolves_after_assignments() {
    let (_temp, document) = fixture("kind: note");
    md_cmd()
        .args(["schema", "validate"])
        .arg(&document)
        .arg("kind=prompt")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("owner"));
}

#[test]
fn compose_honors_triggers_and_raw_mode() {
    let (_temp, document) = fixture("kind: prompt");
    md_cmd()
        .arg("compose")
        .arg(&document)
        .assert()
        .failure()
        .stderr(predicate::str::contains("owner"));
    md_cmd()
        .args(["compose", "--no-trigger-schemas"])
        .arg(&document)
        .assert()
        .success();
}

#[test]
fn compose_re_resolves_trigger_after_shell_value_becomes_concrete() {
    let (temp, document) = fixture("kind: $(echo prompt)");
    write(
        temp.path(),
        "docs/.darkmatter-shell-whitelist",
        "exact echo prompt\n",
    );
    md_cmd()
        .arg("compose")
        .arg(&document)
        .assert()
        .failure()
        .stderr(predicate::str::contains("owner"));
}

#[test]
fn triggers_command_prints_shared_trace() {
    let (_temp, document) = fixture("kind: note");
    md_cmd()
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
    let temp = TempDir::new().unwrap();
    std::fs::create_dir(temp.path().join(".git")).unwrap();
    write(temp.path(), "schemas/placeholder.yaml", "$schema:\n  title: string\n");
    write(temp.path(), "docs/local.yaml", "$schema:\n  title: string(required)\n");
    let document = write(
        temp.path(),
        "docs/doc.md",
        "---\n$schema: local.yaml\ntitle: Test\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(document)
        .assert()
        .code(2)
        .stdout(predicate::str::contains("./local.yaml"));
}

#[test]
fn dialect_family_has_identical_compose_validate_and_trace_activation() {
    let temp = dialect_fixture();
    for (document, trigger, required) in [
        ("claudine.md", "claudine.trigger.yaml", "provider"),
        ("inline-compose.md", "inline-compose.trigger.yaml", "output"),
        ("sequence.md", "sequence.trigger.yaml", "sequence_name"),
    ] {
        let path = temp.path().join("docs").join(document);
        md_cmd()
            .args(["schema", "validate"])
            .arg(&path)
            .assert()
            .code(1)
            .stdout(predicate::str::contains(required));
        md_cmd()
            .arg("compose")
            .arg(&path)
            .assert()
            .failure()
            .stderr(predicate::str::contains(required));
        md_cmd()
            .args(["schema", "triggers"])
            .arg(&path)
            .assert()
            .success()
            .stdout(predicate::str::contains(trigger))
            .stdout(predicate::str::contains("matched"));
    }

    let plain = temp.path().join("docs/plain.md");
    md_cmd().args(["schema", "validate"]).arg(&plain).assert().success();
    md_cmd().arg("compose").arg(&plain).assert().success();
}
