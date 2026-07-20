mod common;

use common::{md_cmd, md_file};
use biscuit_terminal::utils::UnicodeWidthStr;
use predicates::prelude::*;
use std::io::Write;

#[test]
fn test_clean_subcommand_stdin() {
    md_cmd()
        .args(["clean", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_clean_subcommand_file() {
    let tmp = md_file("# Hello \n\nWorld  \n");

    md_cmd()
        .arg("clean")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_clean_subcommand_indent() {
    md_cmd()
        .args(["clean", "-", "--indent", "4"])
        .write_stdin("- Parent\n  - Child\n    - Grandchild\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\n    - Child"))
        .stdout(predicate::str::contains("\n        - Grandchild"));
}

#[test]
fn test_clean_subcommand_strips_incidental_newlines_by_default() {
    md_cmd()
        .args(["clean", "-"])
        .write_stdin(
            "This paragraph was wrapped by an editor at a fixed column\n\
             even though Markdown treats it as one paragraph.\n",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "This paragraph was wrapped by an editor at a fixed column even though Markdown treats it as one paragraph.",
        ));
}

#[test]
fn test_clean_subcommand_fixed_width_reflows_to_target_width() {
    let assert = md_cmd()
        .args(["clean", "--fixed-width", "80", "-"])
        .write_stdin(
            "This paragraph starts with editor wrapping around a fixed column\n\
             and should be collapsed first before it is reflowed back to a\n\
             predictable target width for downstream documentation checks.\n",
        )
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let longest = stdout.lines().map(UnicodeWidthStr::width).max().unwrap_or(0);
    assert!(longest <= 80, "longest line was {longest}:\n{stdout}");
    assert!(stdout.lines().count() > 1, "expected wrapped output:\n{stdout}");
}

#[test]
fn test_clean_subcommand_ignore_incidental_newlines_preserves_source_wrapping() {
    let assert = md_cmd()
        .args(["clean", "--ignore-incidental-newlines", "-"])
        .write_stdin("Alpha wrapped\nbeta line\n")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Alpha wrapped\nbeta line"));
    assert!(!stdout.contains("Alpha wrapped beta line"));
}

#[test]
fn test_clean_subcommand_list_modes_match_library_contract() {
    let source = "- Alpha beta gamma delta\n    epsilon zeta eta theta.\n";

    let stripped = md_cmd()
        .args(["clean", "-"])
        .write_stdin(source)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(
        String::from_utf8(stripped).unwrap(),
        "- Alpha beta gamma delta epsilon zeta eta theta.\n"
    );

    let fixed = md_cmd()
        .args(["clean", "--fixed-width", "24", "-"])
        .write_stdin(source)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let fixed = String::from_utf8(fixed).unwrap();
    assert_eq!(
        fixed,
        "- Alpha beta gamma delta\n  epsilon zeta eta\n  theta.\n"
    );
    for line in fixed.lines() {
        assert!(UnicodeWidthStr::width(line) <= 24, "line exceeded width: {line:?}");
    }

    let preserved = md_cmd()
        .args(["clean", "--ignore-incidental-newlines", "-"])
        .write_stdin(source)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(String::from_utf8(preserved).unwrap(), source);
}

#[test]
fn test_clean_subcommand_fixed_width_treats_nine_digit_ordinal_as_an_ordered_marker() {
    let fixed = md_cmd()
        .args(["clean", "--fixed-width", "24", "-"])
        .write_stdin("123456789. Alpha beta gamma delta epsilon.\n")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert_eq!(
        String::from_utf8(fixed).unwrap(),
        concat!(
            "123456789. Alpha beta\n",
            "           gamma delta\n",
            "           epsilon.\n"
        )
    );
}

#[test]
fn test_clean_subcommand_fixed_width_treats_ten_digit_ordinal_as_prose() {
    let fixed = md_cmd()
        .args(["clean", "--fixed-width", "24", "-"])
        .write_stdin("1234567890. Alpha beta gamma delta epsilon zeta.\n")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // A ten-digit run is not a CommonMark marker, so no hanging indent may be synthesized.
    assert_eq!(
        String::from_utf8(fixed).unwrap(),
        concat!(
            "1234567890. Alpha beta\n",
            "gamma delta epsilon\n",
            "zeta.\n"
        )
    );
}

#[test]
fn test_clean_subcommand_fixed_width_keeps_reference_definitions_intact() {
    let source = concat!(
        "- Before [label][ref] alpha beta gamma delta.\n",
        "\n",
        "[ref]: https://example.com/a/very/long/path \"A descriptive title\"\n"
    );

    let fixed = md_cmd()
        .args(["clean", "--fixed-width", "24", "-"])
        .write_stdin(source)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert_eq!(
        String::from_utf8(fixed).unwrap(),
        concat!(
            "- Before [label][ref]\n",
            "  alpha beta gamma\n",
            "  delta.\n",
            "\n",
            "[ref]: https://example.com/a/very/long/path \"A descriptive title\"\n"
        )
    );
}

#[test]
fn test_clean_subcommand_rejects_fixed_width_with_ignore_incidental_newlines() {
    md_cmd()
        .args([
            "clean",
            "--fixed-width",
            "80",
            "--ignore-incidental-newlines",
            "-",
        ])
        .write_stdin("Alpha\nbeta\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_clean_subcommand_rejects_invalid_indent() {
    md_cmd()
        .args(["clean", "-", "--indent", "3"])
        .write_stdin("- Parent\n  - Child\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("indent must be one of: 2, 4, 8"));
}

#[test]
fn test_clean_subcommand_save_fixed_width_reports_delta() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(
        tmp,
        "# Title\n\n\
         This paragraph was wrapped by an editor at a fixed column\n\
         and should be collapsed first before being saved back out.\n"
    )
    .unwrap();

    md_cmd()
        .args(["clean", "--fixed-width", "40"])
        .arg(tmp.path())
        .arg("--save")
        .assert()
        .success()
        .stdout(predicate::str::contains("changed"));

    let updated = std::fs::read_to_string(tmp.path()).unwrap();
    assert_ne!(
        updated,
        "# Title\n\nThis paragraph was wrapped by an editor at a fixed column\nand should be collapsed first before being saved back out.\n"
    );
    let longest = updated.lines().map(UnicodeWidthStr::width).max().unwrap_or(0);
    assert!(longest <= 40, "longest line was {longest}:\n{updated}");
}

#[test]
fn test_clean_subcommand_save_reflows_list_and_is_stable_on_repeated_read() {
    let source = "# Title\n\n- Alpha beta gamma delta\n    epsilon zeta eta theta.\n";
    let expected = "# Title\n\n- Alpha beta gamma delta\n  epsilon zeta eta\n  theta.\n";
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "{source}").unwrap();
    tmp.flush().unwrap();

    md_cmd()
        .args(["clean", "--fixed-width", "24"])
        .arg(tmp.path())
        .arg("--save")
        .assert()
        .success()
        .stdout(predicate::str::contains("changed"));

    let first_read = std::fs::read_to_string(tmp.path()).unwrap();
    assert_eq!(first_read, expected);
    for line in first_read.lines() {
        assert!(UnicodeWidthStr::width(line) <= 24, "line exceeded width: {line:?}");
    }

    md_cmd()
        .args(["clean", "--fixed-width", "24"])
        .arg(tmp.path())
        .arg("--save")
        .assert()
        .success();
    let second_read = std::fs::read_to_string(tmp.path()).unwrap();
    assert_eq!(second_read, first_read);
}

#[test]
fn test_clean_subcommand_save_in_place_reports_delta() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "# Hello \n\nWorld  \n").unwrap();

    md_cmd()
        .arg("clean")
        .arg(tmp.path())
        .arg("--save")
        .assert()
        .success()
        .stdout(predicate::str::contains("Whitespace changes only"));

    let updated = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(updated.contains("# Hello"));
    assert!(updated.contains("World"));
    assert!(!updated.contains("# Hello "));
    assert!(!updated.contains("World  "));
    assert!(updated.ends_with('\n'));
    assert!(!updated.ends_with("\n\n"));
}

#[test]
fn test_clean_subcommand_save_verbose_reports_fixed_width_visual_delta() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(
        tmp,
        "# Title\n\n\
         This paragraph was wrapped by an editor at a fixed column\n\
         and should be collapsed first before the verbose delta is reported.\n"
    )
    .unwrap();

    md_cmd()
        .args(["clean", "--fixed-width", "40", "--save", "-v"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Modified (1):"))
        .stdout(predicate::str::contains("original"))
        .stdout(predicate::str::contains("updated"));
}

#[test]
fn test_clean_subcommand_save_verbose_after_subcommand_shows_visual_diff() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "# Hello \n\nWorld  \n").unwrap();

    md_cmd()
        .args(["clean", "--save", "-v"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Whitespace only"))
        .stdout(predicate::str::contains("original"))
        .stdout(predicate::str::contains("updated"))
        .stdout(predicate::str::contains("Content Visual Diff:").not());
}

#[test]
fn test_save_shorthand_cleans_in_place_and_reports_delta() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "# Hello \n\nWorld  \n").unwrap();

    md_cmd()
        .arg(tmp.path())
        .arg("--save")
        .assert()
        .success()
        .stdout(predicate::str::contains("Whitespace changes only"));

    let updated = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(updated.contains("# Hello"));
    assert!(updated.contains("World"));
    assert!(!updated.contains("# Hello "));
    assert!(!updated.contains("World  "));
}

#[test]
fn test_clean_save_rejects_stdin() {
    md_cmd()
        .args(["clean", "-", "--save"])
        .write_stdin("# Hello\n\nWorld\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--save requires an input file path (stdin is not supported)",
        ));
}
