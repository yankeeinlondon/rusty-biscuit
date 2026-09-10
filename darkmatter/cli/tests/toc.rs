mod common;

use common::CliProcessFixture;
use predicates::prelude::*;

#[test]
fn test_toc_subcommand_output() {
    let fixture = CliProcessFixture::named("toc-test-toc-subcommand-output");
    fixture
        .command()
        .args(["toc", "-"])
        .write_stdin("# Top\n\n## Section A\n\n## Section B\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Top"))
        .stdout(predicate::str::contains("Section A"));
}

#[test]
fn test_toc_subcommand_json_output() {
    let fixture = CliProcessFixture::named("toc-test-toc-subcommand-json-output");
    fixture
        .command()
        .args(["toc", "--json", "-"])
        .write_stdin("# Top\n\n## Section A\n\n## Section B\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"structure\""));
}

#[test]
fn test_toc_subcommand_ignores_tab_indented_frontmatter() {
    let fixture =
        CliProcessFixture::named("toc-test-toc-subcommand-ignores-tab-indented-frontmatter");
    let input = "---\nprompt: |-\n\tLine one\n\tLine two\nlast_updated: 2026-02-27\n---\n# macOS Audio\n\n## Details\n";

    fixture
        .command()
        .args(["toc", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("macOS Audio"))
        .stdout(predicate::str::contains("Details"))
        .stdout(predicate::str::contains("last_updated").not());
}
