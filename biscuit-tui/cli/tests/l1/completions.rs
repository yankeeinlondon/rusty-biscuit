use predicates::prelude::*;

#[test]
fn completions_bash_emits_non_empty_script() {
    assert_cmd::Command::cargo_bin("question")
        .unwrap()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("question"))
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn completions_zsh_emits_non_empty_script() {
    assert_cmd::Command::cargo_bin("question")
        .unwrap()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("question"))
        .stdout(predicate::str::contains("#compdef"));
}

#[test]
fn completions_fish_emits_non_empty_script() {
    assert_cmd::Command::cargo_bin("question")
        .unwrap()
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
        assert_cmd::Command::cargo_bin("question")
            .unwrap()
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
        assert_cmd::Command::cargo_bin("question")
            .unwrap()
            .args(["completions", shell])
            .assert()
            .success()
            .stdout(predicate::str::contains("sort"));
    }
}

// Phase 4: completions must list `inverse` as the canonical `--sort`
// value across every supported shell. `reverse` is a hidden alias
// (clap's `#[clap(alias = "reverse")]` with the variant renamed to
// `Inverse`) so it must NOT appear as a presented choice.

#[test]
fn completions_zsh_contains_inverse_for_sort() {
    assert_cmd::Command::cargo_bin("question")
        .unwrap()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("inverse"));
}

#[test]
fn completions_bash_contains_inverse_for_sort() {
    assert_cmd::Command::cargo_bin("question")
        .unwrap()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("inverse"));
}

#[test]
fn completions_fish_contains_inverse_for_sort() {
    assert_cmd::Command::cargo_bin("question")
        .unwrap()
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("inverse"));
}

#[test]
fn completions_zsh_does_not_present_reverse_as_canonical() {
    // `reverse` is a hidden alias and must not appear in the
    // generated zsh completion script. clap omits aliases marked as
    // hidden via `#[clap(alias = "...")]`.
    assert_cmd::Command::cargo_bin("question")
        .unwrap()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("reverse").not());
}

#[test]
fn completions_bash_does_not_present_reverse_as_canonical() {
    assert_cmd::Command::cargo_bin("question")
        .unwrap()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("reverse").not());
}

#[test]
fn completions_fish_does_not_present_reverse_as_canonical() {
    assert_cmd::Command::cargo_bin("question")
        .unwrap()
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("reverse").not());
}

#[test]
fn completions_script_mentions_source_flags() {
    let shells = ["bash", "zsh", "fish"];
    for shell in shells {
        assert_cmd::Command::cargo_bin("question")
            .unwrap()
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
        assert_cmd::Command::cargo_bin("question")
            .unwrap()
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
        assert_cmd::Command::cargo_bin("question")
            .unwrap()
            .args(["completions", shell])
            .assert()
            .success()
            .stdout(predicate::str::contains("label-convention"))
            .stdout(predicate::str::contains("value-convention"));
    }
}

#[test]
fn completions_do_not_present_hidden_legacy_flags() {
    let shells = ["bash", "zsh", "fish", "powershell", "elvish"];
    for shell in shells {
        assert_cmd::Command::cargo_bin("question")
            .unwrap()
            .args(["completions", shell])
            .assert()
            .success()
            .stdout(predicate::str::contains("options-from-file").not())
            .stdout(predicate::str::contains("options-from-dictionary").not());
    }
}

#[test]
fn completions_only_offer_hotkey_prefixes_after_bracket_prefix() {
    assert_cmd::Command::cargo_bin("question")
        .unwrap()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("|| -z \"$PREFIX\"").not())
        .stdout(predicate::str::contains("[CTRL+"))
        .stdout(predicate::str::contains("[ALT+"))
        .stdout(predicate::str::contains("[OPT+"));

    assert_cmd::Command::cargo_bin("question")
        .unwrap()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("COMP_CWORD}\" -eq 2 && -z \"$cur\"").not())
        .stdout(predicate::str::contains("[CTRL+"))
        .stdout(predicate::str::contains("[ALT+"))
        .stdout(predicate::str::contains("[OPT+"));
}

// Phase 5: completions must include the `--active-color` flag and its
// four palette values across all supported shells.

#[test]
fn completions_include_active_color_flag() {
    let shells = ["bash", "zsh", "fish"];
    for shell in shells {
        assert_cmd::Command::cargo_bin("question")
            .unwrap()
            .args(["completions", shell])
            .assert()
            .success()
            .stdout(predicate::str::contains("active-color"));
    }
}

#[test]
fn completions_include_active_color_values() {
    let shells = ["bash", "zsh", "fish"];
    for shell in shells {
        assert_cmd::Command::cargo_bin("question")
            .unwrap()
            .args(["completions", shell])
            .assert()
            .success()
            .stdout(predicate::str::contains("grey"))
            .stdout(predicate::str::contains("green"))
            .stdout(predicate::str::contains("yellow"))
            .stdout(predicate::str::contains("red"));
    }
}
