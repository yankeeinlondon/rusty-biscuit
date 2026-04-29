use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn completions_bash_emits_non_empty_script() {
    cargo_bin_cmd!("question")
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("question"))
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn completions_zsh_emits_non_empty_script() {
    cargo_bin_cmd!("question")
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("question"))
        .stdout(predicate::str::contains("#compdef"));
}

#[test]
fn completions_fish_emits_non_empty_script() {
    cargo_bin_cmd!("question")
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("question"))
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn completions_script_mentions_choose_subcommands() {
    let shells = ["bash", "zsh", "fish"];
    for shell in shells {
        cargo_bin_cmd!("question")
            .args(["completions", shell])
            .assert()
            .success()
            .stdout(predicate::str::contains("choose-one"))
            .stdout(predicate::str::contains("choose-many"));
    }
}

#[test]
fn completions_script_mentions_sort_flags() {
    let shells = ["bash", "zsh", "fish"];
    for shell in shells {
        cargo_bin_cmd!("question")
            .args(["completions", shell])
            .assert()
            .success()
            .stdout(predicate::str::contains("sort"));
    }
}

#[test]
fn completions_script_mentions_source_flags() {
    let shells = ["bash", "zsh", "fish"];
    for shell in shells {
        cargo_bin_cmd!("question")
            .args(["completions", shell])
            .assert()
            .success()
            .stdout(predicate::str::contains("csv"))
            .stdout(predicate::str::contains("list"))
            .stdout(predicate::str::contains("rows"))
            .stdout(predicate::str::contains("file"))
            .stdout(predicate::str::contains("md"));
    }
}

#[test]
fn completions_script_mentions_padding_flags() {
    let shells = ["bash", "zsh", "fish"];
    for shell in shells {
        cargo_bin_cmd!("question")
            .args(["completions", shell])
            .assert()
            .success()
            .stdout(predicate::str::contains("padding"))
            .stdout(predicate::str::contains("pt"))
            .stdout(predicate::str::contains("pb"))
            .stdout(predicate::str::contains("pl"))
            .stdout(predicate::str::contains("pr"));
    }
}

#[test]
fn completions_script_mentions_convention_flags() {
    let shells = ["bash", "zsh", "fish"];
    for shell in shells {
        cargo_bin_cmd!("question")
            .args(["completions", shell])
            .assert()
            .success()
            .stdout(predicate::str::contains("label-convention"))
            .stdout(predicate::str::contains("value-convention"));
    }
}
