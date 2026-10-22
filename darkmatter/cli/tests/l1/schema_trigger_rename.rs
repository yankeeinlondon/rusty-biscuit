//! The schema-trigger renames on the `md` surface
//! (`2026-10-27-darkmatter-lifecycle-ownership`, criteria 16 and 17):
//! `--no-schema-triggers` works on `md compose`, `md schema validate`, and
//! `md clean`; `--no-trigger-schemas` is a usage error naming its
//! replacement; completions offer only the new flag; and the trigger input
//! matrix reaches `md compose` (the full matrix runs in-process in the
//! library's `trigger_input_matrix`).

use crate::common;

use common::CliProcessFixture;
use predicates::prelude::*;
use std::path::{Path, PathBuf};

fn write(root: &Path, relative: &str, content: &str) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, content).unwrap();
    path
}

/// A repository with `trigger` as `schemas/prompt.trigger.yaml`, whose
/// control payload requires `owner`, and a `kind: prompt` document.
fn repository(process: &CliProcessFixture, trigger: &str) -> PathBuf {
    let root = process.cwd();
    let git = root.join(".git");
    std::fs::create_dir_all(git.join("objects")).unwrap();
    std::fs::create_dir_all(git.join("refs/heads")).unwrap();
    std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(git.join("config"), "[core]\nrepositoryformatversion = 0\nbare = false\n").unwrap();
    write(root, "schemas/prompt.trigger.yaml", trigger);
    write(root, "schemas/prompt.yaml", "$schema:\n  owner: string(required)\n");
    write(root, "docs/prompt.md", "---\nkind: prompt\nmode: m\n---\nComposed body.\n")
}

const CONTROL: &str = "kind: schema-trigger\nmatch:\n  - kind: enum(prompt; required)\n  - group:\n      - mode: string(required)\n      - level: number(required)\n$schema: prompt.yaml\n";

#[test]
fn the_new_flag_disables_triggers_on_compose_validate_and_clean() {
    let process = CliProcessFixture::new();
    let document = repository(&process, CONTROL);
    for subcommand in [&["compose"][..], &["schema", "validate"], &["clean"]] {
        process
            .command()
            .args(subcommand)
            .arg("--no-schema-triggers")
            .arg(&document)
            .assert()
            .success();
    }
    // Without the flag the trigger's payload applies.
    process.command().args(["schema", "validate"]).arg(&document).assert().code(1).stdout(predicate::str::contains("owner"));
}

#[test]
fn the_old_flag_is_a_usage_error_naming_the_new_one() {
    let process = CliProcessFixture::new();
    let document = repository(&process, CONTROL);
    for subcommand in [&["compose"][..], &["schema", "validate"], &["clean"]] {
        process
            .command()
            .args(subcommand)
            .arg("--no-trigger-schemas")
            .arg(&document)
            .assert()
            .code(2)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains("`--no-trigger-schemas` was renamed"))
            .stderr(predicate::str::contains("use `--no-schema-triggers`"));
    }
}

#[test]
fn completions_offer_only_the_new_flag() {
    let process = CliProcessFixture::new();
    for words in [&["md", "compose"][..], &["md", "schema", "validate"], &["md", "clean"]] {
        for prefix in ["--no-", "--no-t", "--no-s"] {
            let output = process
                .command()
                .env("COMPLETE", "fish")
                .arg("--")
                .args(words)
                .arg(prefix)
                .output()
                .expect("md runs");
            let candidates = String::from_utf8_lossy(&output.stdout);
            assert!(!candidates.contains("--no-trigger-schemas"), "{words:?} {prefix}: {candidates}");
            if prefix != "--no-t" {
                assert!(candidates.contains("--no-schema-triggers"), "{words:?} {prefix}: {candidates}");
            }
        }
    }
}

#[test]
fn compose_applies_the_control_trigger() {
    let process = CliProcessFixture::new();
    let document = repository(&process, CONTROL);
    process
        .command()
        .arg("compose")
        .arg(&document)
        .assert()
        .failure()
        .stdout(predicate::str::contains("Composed body").not())
        .stderr(predicate::str::contains("owner"));
}

#[test]
fn compose_reports_an_invalid_trigger_field_by_location_and_composes_nothing() {
    for (edit, fragments) in [
        (CONTROL.replace("  - group:\n      - mode: string(required)\n      - level: number(required)\n", "  - group: null\n"), &["match[1].group", "null"][..]),
        (CONTROL.replace("  - group:\n", "  - any:\n"), &["match[1].any", "renamed", "group:"]),
        (CONTROL.replace("kind: schema-trigger\n", "kind: trigger-schema\n"), &["trigger-schema", "renamed", "schema-trigger"]),
    ] {
        let process = CliProcessFixture::new();
        let document = repository(&process, &edit);
        let output = process.command().arg("compose").arg(&document).output().expect("md runs");
        assert!(!output.status.success());
        assert!(output.stdout.is_empty(), "nothing is composed");
        let stderr = String::from_utf8_lossy(&output.stderr);
        // The error box wraps at the terminal width, and a long temp path (as
        // on Windows) can split a fragment across rows; match the rows joined,
        // without the styling that starts each row.
        let unwrapped: String = without_escapes(&stderr)
            .lines()
            .map(|row| row.trim_start_matches(|c: char| c.is_whitespace() || c == '┃').trim_end())
            .collect();
        for fragment in ["prompt.trigger.yaml"].iter().chain(fragments) {
            assert!(unwrapped.contains(fragment), "`{fragment}` not in {stderr}");
        }
    }
}

/// `text` without CSI (`ESC [ … final`) and OSC (`ESC ] … ST`) sequences.
fn without_escapes(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            plain.push(c);
            continue;
        }
        match chars.next() {
            Some('[') => {
                chars.by_ref().find(|c| ('@'..='~').contains(c));
            }
            Some(']') => {
                while let Some(c) = chars.next() {
                    if c == '\u{7}' || (c == '\u{1b}' && chars.next_if_eq(&'\\').is_some()) {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    plain
}
