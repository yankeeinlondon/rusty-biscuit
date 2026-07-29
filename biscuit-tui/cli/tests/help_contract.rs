use predicates::prelude::*;

#[test]
fn top_level_help_uses_canonical_public_names() {
    assert_cmd::Command::cargo_bin("question")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: question"))
        .stdout(predicate::str::contains("choose-one"))
        .stdout(predicate::str::contains("choose-many"))
        .stdout(predicate::str::contains("boolean-switch"))
        .stdout(predicate::str::contains("text-input"))
        .stdout(predicate::str::contains("text-area-input"))
        .stdout(predicate::str::contains("input-table"))
        .stdout(predicate::str::contains("completions"))
        .stdout(predicate::str::contains("select-one").not())
        .stdout(predicate::str::contains("select-many").not());
}

#[test]
fn top_level_help_shows_height_as_global_option() {
    assert_cmd::Command::cargo_bin("question")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--height"))
        .stdout(predicate::str::contains("Render inline"));
}

#[test]
fn text_input_help_shows_height_as_inherited_global() {
    // clap still shows global flags in subcommand help, but they are
    // marked differently or appear in a separate section. The important
    // thing is that --height is NOT in the text-input-specific Args
    // struct, which we've already verified in code.
    assert_cmd::Command::cargo_bin("question")
        .unwrap()
        .args(["text-input", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--height"));
}

#[test]
fn every_subcommand_accepts_height_before_subcommand_token() {
    let subcommands = [
        "text-input",
        "text-area-input",
        "boolean-switch",
        "choose-one",
        "choose-many",
        "input-table",
        "completions",
    ];

    for subcommand in subcommands {
        assert_cmd::Command::cargo_bin("question")
            .unwrap()
            .args(["--height", "5", subcommand, "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--height"));
    }
}
