//! A `::map`/`::filter` that meets a value a dry run cannot have (a source
//! an earlier task writes, a later step's `seq.outputs`, an effectful
//! expression function) is left whole and `not checked`, while the same
//! collection over static values composes and a genuine error on known data
//! still refuses. Each case is one dry run of a sequence (or a direct or
//! inline composition) whose reader holds the collection; the rows the
//! check prints are the evidence. Where a real run is part of the row, it
//! uses the compiled `claudine-fake-goose`, which appends one line per
//! launch to `PROVIDER_MARKER`.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::common::{CliProcessFixture, strip_ansi};

const COLLECTION: &str = "`::map`/`::filter` collection";
const BLOCK: &str = "`::block` condition";
const AT_LAUNCH: &str = "known only at launch";
const UNDECLARED: &str = "dm.expression.undeclared_property";

/// What the dry run concludes.
enum Dry {
    /// Exit 0, no error, and a `not checked` info row naming each check for
    /// a reason holding `why`.
    Unchecked(&'static [&'static str], &'static str),
    /// Exit 0 and no `not checked` row.
    Proven,
    /// Exit 2 with an error row of this code.
    Refused(&'static str),
}

/// A sequence whose `first` task runs `writer.md`, which writes `key` on
/// success, then `reader`, whose prompt file is `reader`.
fn two_tasks(key: &str, reader: &str) -> Vec<(&'static str, String)> {
    vec![
        (
            "seq.md",
            "---\nagent: goose\nsequence:\n  - name: first\n    prompt: writer.md\n  - name: reader\n    prompt: reader.md\n---\n"
                .to_string(),
        ),
        ("writer.md", format!("---\nsuccess:\n  - set:\n      {key}: [1, 2]\n---\n\nWrite.\n")),
        ("reader.md", reader.to_string()),
    ]
}

/// A reader holding `region` between composed text, over a declared
/// `items` list.
fn reader(region: &str) -> String {
    format!("---\nitems: [1, 2]\n$schema:\n  items: \"number[]\"\n---\n\nIntro.\n\n{region}\nOutro.\n")
}

/// R3-F2.1: the collection reads `items`, which the first task writes when
/// `key` is `items`.
fn source(key: &str, region: &str) -> Vec<(&'static str, String)> {
    two_tasks(key, &reader(region))
}

/// R3-F2.2 to R3-F2.4: the reader holds `region`; the first task writes an
/// unrelated key, so only the sequence's own values can be unavailable.
fn step(region: &str) -> Vec<(&'static str, String)> {
    two_tasks("other", &reader(region))
}

fn staged(name: &str, files: &[(&str, String)]) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fs::copy(
        biscuit_test_harness::bin_exe!("claudine-fake-goose"),
        fixture.bin_dir().join(format!("goose{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("the fake provider is built with `test-fixtures`");
    for (path, text) in files {
        fs::write(fixture.cwd().join(path), text).unwrap();
    }
    fixture
}

fn marker(fixture: &CliProcessFixture) -> PathBuf {
    fixture.cwd().join("provider-launched.txt")
}

fn launches(fixture: &CliProcessFixture) -> usize {
    fs::read_to_string(marker(fixture)).map_or(0, |log| log.lines().count())
}

fn dry_run(fixture: &CliProcessFixture, command: &[&str]) -> (Option<i32>, Value) {
    let output = fixture
        .command()
        .env("PROVIDER_MARKER", marker(fixture))
        .args(command)
        .args(["--dry-run", "--json"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope = serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
    assert_eq!(launches(fixture), 0, "a dry run launched a provider");
    (output.status.code(), envelope)
}

/// Every row as `(code, severity, message)`.
fn rows(envelope: &Value) -> Vec<(String, String, String)> {
    envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let text = |key: &str| row[key].as_str().unwrap_or_default().to_string();
            (text("code"), text("severity"), text("message"))
        })
        .collect()
}

fn assert_dry(code: Option<i32>, envelope: &Value, expected: &Dry) {
    let rows = rows(envelope);
    let unchecked: Vec<&(String, String, String)> =
        rows.iter().filter(|(code, _, _)| code == "dm.check.not_checked").collect();
    let errors: Vec<&String> = rows.iter().filter(|(_, severity, _)| severity == "error").map(|(code, _, _)| code).collect();
    // A collection's item binding is the collection's, never an undeclared
    // document property, in a body and an inline `prompt:` alike.
    assert!(rows.iter().all(|(code, _, _)| code != UNDECLARED), "{rows:#?}");
    for refused in ["dm.collection.callback_failed", "composition.failed", "dm.expression.evaluation_failure"] {
        if !matches!(expected, Dry::Refused(code) if *code == refused) {
            assert!(rows.iter().all(|(code, _, _)| code != refused), "no `{refused}` row: {rows:#?}");
        }
    }
    match expected {
        Dry::Unchecked(checks, why) => {
            assert_eq!(code, Some(0), "{rows:#?}");
            assert!(errors.is_empty(), "{rows:#?}");
            for check in *checks {
                assert!(
                    unchecked.iter().any(|(_, severity, message)| severity == "info"
                        && message.contains(&format!("not checked: {check}"))
                        && message.contains(why)),
                    "`{check}` must be an info `not checked` row for `{why}`: {rows:#?}"
                );
            }
        }
        Dry::Proven => {
            assert_eq!(code, Some(0), "{rows:#?}");
            assert!(errors.is_empty() && unchecked.is_empty(), "{rows:#?}");
        }
        Dry::Refused(expected) => {
            assert_eq!(code, Some(2), "{rows:#?}");
            assert!(errors.iter().any(|code| code == expected), "expected `{expected}`: {rows:#?}");
        }
    }
}

/// One `#[test]` per case: the dry run's conclusion, then, when given, the
/// real run's exit code and launch count for the same files.
macro_rules! collection_case {
    ($($name:ident: $files:expr, $dry:expr $(, real: $real:expr, launched: $launched:expr)?;)*) => {$(
        #[test]
        fn $name() {
            let fixture = staged(concat!("collection-", stringify!($name)), &$files);
            let command: &[&str] = &["sequence", "seq.md"];
            let (code, envelope) = dry_run(&fixture, command);
            assert_dry(code, &envelope, &$dry);
            $(
                let output = fixture.command().env("PROVIDER_MARKER", marker(&fixture)).args(command).output().unwrap();
                let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
                assert_eq!(output.status.code(), Some($real), "{stderr}");
                assert_eq!(launches(&fixture), $launched, "{stderr}");
            )?
        }
    )*};
}

const MAP_SOURCE: &str = "::map items \"i => i * 10\"\n- {{ i }}\n::end-map\n";
const FILTER_SOURCE: &str = "::filter items \"i => i > 1\"\n- {{ i }}\n::end-filter\n";
const OUTPUTS: &str = "length(seq.outputs) > 0";
const POSITION: &str = "seq.index == 2";

fn positional_map(read: &str) -> String {
    format!("::map items \"i => {read} ? i : 0\"\n- {{{{ i }}}}\n::end-map\n")
}

fn positional_filter(read: &str) -> String {
    format!("::filter items \"i => {read}\"\n- {{{{ i }}}}\n::end-filter\n")
}

/// A `directive` with an always-true positional callback and `option`.
fn with_option(directive: &str, option: &str) -> String {
    let callback = if directive == "map" { "i => i" } else { "i => true" };
    format!("::{directive} items \"{callback}\" {option}\n- {{{{ i }}}}\n::end-{directive}\n")
}

fn nested_block(read: &str) -> String {
    format!("::map items \"i => i\"\n- item {{{{ i }}}}\n::block when=\"{read}\"\nguarded {{{{ i }}}}\n::end-block\n::end-map\n")
}

collection_case! {
    // R3-F2.1
    a_map_over_a_source_an_earlier_task_writes_is_not_checked: source("items", MAP_SOURCE),
        Dry::Unchecked(&[COLLECTION], AT_LAUNCH), real: 0, launched: 2;
    a_filter_over_a_source_an_earlier_task_writes_is_not_checked: source("items", FILTER_SOURCE),
        Dry::Unchecked(&[COLLECTION], AT_LAUNCH), real: 0, launched: 2;
    a_map_beside_an_unrelated_write_composes: source("other", MAP_SOURCE), Dry::Proven, real: 0, launched: 2;
    a_filter_beside_an_unrelated_write_composes: source("other", FILTER_SOURCE), Dry::Proven, real: 0, launched: 2;
    // R3-F2.2
    a_map_transform_reading_outputs_is_not_checked: step(&positional_map(OUTPUTS)),
        Dry::Unchecked(&[COLLECTION], AT_LAUNCH), real: 0, launched: 2;
    a_filter_predicate_reading_outputs_is_not_checked: step(&positional_filter(OUTPUTS)),
        Dry::Unchecked(&[COLLECTION], AT_LAUNCH), real: 0, launched: 2;
    a_map_transform_reading_the_position_composes: step(&positional_map(POSITION)), Dry::Proven, real: 0, launched: 2;
    a_filter_predicate_reading_the_position_composes: step(&positional_filter(POSITION)), Dry::Proven, real: 0, launched: 2;
    // R3-F2.3
    a_map_filter_option_reading_outputs_is_not_checked: step(&with_option("map", &format!("filter=\"i => {OUTPUTS}\""))),
        Dry::Unchecked(&[COLLECTION], AT_LAUNCH);
    a_map_while_option_reading_outputs_is_not_checked: step(&with_option("map", &format!("while=\"i => {OUTPUTS}\""))),
        Dry::Unchecked(&[COLLECTION], AT_LAUNCH);
    a_map_until_option_reading_outputs_is_not_checked: step(&with_option("map", &format!("until=\"i => !({OUTPUTS})\""))),
        Dry::Unchecked(&[COLLECTION], AT_LAUNCH);
    a_map_sort_option_reading_outputs_is_not_checked: step(&with_option("map", "sort=\"i => length(seq.outputs) + i\"")),
        Dry::Unchecked(&[COLLECTION], AT_LAUNCH);
    a_filter_filter_option_reading_outputs_is_not_checked: step(&with_option("filter", &format!("filter=\"i => {OUTPUTS}\""))),
        Dry::Unchecked(&[COLLECTION], AT_LAUNCH);
    a_filter_while_option_reading_outputs_is_not_checked: step(&with_option("filter", &format!("while=\"i => {OUTPUTS}\""))),
        Dry::Unchecked(&[COLLECTION], AT_LAUNCH);
    a_filter_until_option_reading_outputs_is_not_checked: step(&with_option("filter", &format!("until=\"i => !({OUTPUTS})\""))),
        Dry::Unchecked(&[COLLECTION], AT_LAUNCH);
    a_filter_sort_option_reading_outputs_is_not_checked: step(&with_option("filter", "sort=\"i => length(seq.outputs) + i\"")),
        Dry::Unchecked(&[COLLECTION], AT_LAUNCH);
    static_map_options_compose: step(&with_option(
        "map",
        &format!("filter=\"i => {POSITION}\" while=\"i => {POSITION}\" sort=\"i => seq.index + i\""),
    )), Dry::Proven;
    static_filter_options_compose: step(&with_option(
        "filter",
        &format!("filter=\"i => {POSITION}\" until=\"i => !({POSITION})\" sort=\"i => seq.index - i\""),
    )), Dry::Proven;
    a_static_boolean_sort_key_is_still_refused: step(&with_option("map", &format!("sort=\"i => {POSITION}\""))),
        Dry::Refused("dm.collection.invalid_sort_key");
    // R3-F2.4
    a_map_gate_reading_outputs_is_not_checked: step(&with_option("map", &format!("when=\"{OUTPUTS}\""))),
        Dry::Unchecked(&[COLLECTION], AT_LAUNCH), real: 0, launched: 2;
    a_filter_gate_reading_outputs_is_not_checked: step(&with_option("filter", &format!("when=\"{OUTPUTS}\""))),
        Dry::Unchecked(&[COLLECTION], AT_LAUNCH), real: 0, launched: 2;
    a_page_block_inside_a_map_reading_outputs_is_not_checked: step(&nested_block(OUTPUTS)),
        Dry::Unchecked(&[BLOCK], AT_LAUNCH), real: 0, launched: 2;
    static_collection_gates_compose: step(&format!(
        "{}{}{}",
        with_option("map", &format!("when=\"{POSITION}\"")),
        with_option("filter", &format!("when=\"{POSITION}\"")),
        nested_block(POSITION)
    )), Dry::Proven;
}

/// R3-F2.5: `has_alias('ll')` in a callback starts the login shell named by
/// `$SHELL`, here a fake `bash` that records each start. The dry run never
/// starts it, in a direct composition, an inline composition, or a
/// sequence's inline prompt, and reports the collection as one info
/// `not checked` row; a pure callback composes. The probe reads `$SHELL`
/// and a script with a shebang, so these run on Unix only.
#[cfg(unix)]
mod effectful {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    const SHELL_ROW: &str = "`has_alias()` starts the login shell, which a dry run does not do";
    const EFFECT: &str = "::filter items \"i => has_alias('ll')\"\n- {{ i }}\n::end-filter\n";
    const PURE: &str = "::filter items \"i => upper('ll') == 'LL'\"\n- {{ i }}\n::end-filter\n";
    const OUTPUTS_FILTER: &str = "::filter items \"i => length(seq.outputs) > 0\"\n- {{ i }}\n::end-filter\n";

    /// The files of `mode` holding `region`, the command, and the prompt
    /// file the region sits in.
    fn files(mode: &str, region: &str) -> (Vec<(&'static str, String)>, &'static [&'static str]) {
        let inline = format!(
            "---\nagent: goose\nitems: [1, 2]\n$schema:\n  items: \"number[]\"\nprompt: |\n  Intro.\n{}\n---\n",
            region.lines().map(|line| format!("  {line}")).collect::<Vec<_>>().join("\n")
        );
        match mode {
            "compose" => (
                vec![("doc.md", format!("---\nagent: goose\n{}", &reader(region)["---\n".len()..]))],
                &["compose", "doc.md"],
            ),
            "inline" => (vec![("doc.md", inline)], &["inline-compose", "doc.md"]),
            _ => (two_tasks("other", &inline.replace("agent: goose\n", "")), &["sequence", "seq.md"]),
        }
    }

    fn run(name: &str, mode: &str, region: &str) -> (Option<i32>, Value, bool) {
        let (files, command) = files(mode, region);
        let fixture = staged(name, &files);
        let shell_dir = fixture.workspace_path().join("shell");
        fs::create_dir_all(&shell_dir).unwrap();
        let shell_marker = shell_dir.join("launches");
        let bash = shell_dir.join("bash");
        fs::write(
            &bash,
            format!("#!/bin/sh\necho launched >> '{}'\nprintf 'darkmatter-shell-probe:alias\\n'\n", shell_marker.display()),
        )
        .unwrap();
        fs::set_permissions(&bash, fs::Permissions::from_mode(0o755)).unwrap();
        let output = fixture
            .command()
            .env("PROVIDER_MARKER", marker(&fixture))
            .env("SHELL", &bash)
            .args(command)
            .args(["--dry-run", "--json"])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let envelope = serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
        assert_eq!(launches(&fixture), 0, "a dry run launched a provider");
        (output.status.code(), envelope, shell_marker.exists())
    }

    macro_rules! effectful_case {
        ($($name:ident: $mode:literal;)*) => {$(
            #[test]
            fn $name() {
                let (code, envelope, shell_started) = run(concat!("collection-effect-", stringify!($name)), $mode, EFFECT);
                assert!(!shell_started, "the login shell started during a dry run: {envelope:#}");
                assert_dry(code, &envelope, &Dry::Unchecked(&[COLLECTION], SHELL_ROW));

                let (code, envelope, shell_started) = run(concat!("collection-pure-", stringify!($name)), $mode, PURE);
                assert!(!shell_started, "{envelope:#}");
                assert_dry(code, &envelope, &Dry::Proven);
            }
        )*};
    }

    effectful_case! {
        an_effectful_callback_in_a_direct_composition_is_not_checked: "compose";
        an_effectful_callback_in_an_inline_composition_is_not_checked: "inline";
        an_effectful_callback_in_a_sequence_inline_prompt_is_not_checked: "sequence";
    }

    /// The sequence's inline prompt reading a later step's outputs in its
    /// callback is the same deferral, for the launch-time value.
    #[test]
    fn a_sequence_inline_prompt_callback_reading_outputs_is_not_checked() {
        let (code, envelope, _) = run("collection-inline-outputs", "sequence", OUTPUTS_FILTER);
        assert_dry(code, &envelope, &Dry::Unchecked(&[COLLECTION], AT_LAUNCH));
    }
}

/// R3-F2.6: an inline `prompt:` holding a `::map` whose body reads its item
/// is composed as a body, where the collection binds the item, so neither
/// run reports the item as an undeclared document property, in
/// `inline-compose` or a sequence's inline prompt. A name nothing declares,
/// outside the collection, still warns: in both runs of `inline-compose`,
/// and in the sequence's dry run (a real sequence step prints no
/// undeclared-property warning at all). The real runs launch the stub,
/// which writes no artifact, so only the launch and the warning are theirs
/// to prove here.
mod inline_item_binding {
    use super::*;

    fn inline(region: &str) -> String {
        format!(
            "---\nagent: goose\nitems: [1, 2]\n$schema:\n  items: \"number[]\"\nprompt: |\n  Intro.\n{}\n---\n",
            region.lines().map(|line| format!("  {line}")).collect::<Vec<_>>().join("\n")
        )
    }

    const ITEM: &str = "::map items \"i => i * 10\"\n- {{ i }}\n::end-map\n";
    const UNDECLARED_OUTSIDE: &str = "::map items \"i => i * 10\"\n- {{ i }}\n::end-map\nAbout {{ nope }}.\n";

    /// The dry run's undeclared-property messages, and, when `real`, the
    /// real run's stderr.
    fn runs(name: &str, mode: &str, region: &str, real: bool) -> (Vec<String>, String) {
        let (files, command): (Vec<(&str, String)>, &[&str]) = match mode {
            "inline" => (vec![("doc.md", inline(region))], &["inline-compose", "doc.md"]),
            _ => (
                vec![
                    ("seq.md", "---\nagent: goose\nsequence:\n  - name: reader\n    prompt: reader.md\n---\n".to_string()),
                    ("reader.md", inline(region).replace("agent: goose\n", "")),
                ],
                &["sequence", "seq.md"],
            ),
        };
        let fixture = staged(name, &files);
        let (code, envelope) = dry_run(&fixture, command);
        let undeclared: Vec<String> = rows(&envelope)
            .into_iter()
            .filter(|(code, _, _)| code == UNDECLARED)
            .map(|(_, _, message)| message)
            .collect();
        // A warning is a problem for the dry run's exit status.
        assert_eq!(code, Some(if undeclared.is_empty() { 0 } else { 2 }), "{envelope:#}");
        if !real {
            return (undeclared, String::new());
        }
        let output = fixture.command().env("PROVIDER_MARKER", marker(&fixture)).args(command).output().unwrap();
        let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
        assert_eq!(launches(&fixture), 1, "{stderr}");
        (undeclared, stderr)
    }

    #[test]
    fn an_inline_compose_map_reading_its_item_is_not_an_undeclared_read() {
        let (undeclared, stderr) = runs("inline-item-compose", "inline", ITEM, true);
        assert!(undeclared.is_empty(), "{undeclared:#?}");
        assert!(!stderr.contains("undeclared document property"), "{stderr}");
    }

    #[test]
    fn an_inline_compose_name_outside_the_map_still_warns() {
        let (undeclared, stderr) = runs("inline-nope-compose", "inline", UNDECLARED_OUTSIDE, true);
        assert_eq!(undeclared.len(), 1, "{undeclared:#?}");
        assert!(undeclared[0].starts_with("`nope`"), "{undeclared:#?}");
        assert!(stderr.contains("`nope`") && !stderr.contains("`i` "), "{stderr}");
    }

    #[test]
    fn a_sequence_inline_prompt_map_reading_its_item_is_not_an_undeclared_read() {
        let (undeclared, stderr) = runs("inline-item-sequence", "sequence", ITEM, true);
        assert!(undeclared.is_empty(), "{undeclared:#?}");
        assert!(!stderr.contains("undeclared document property"), "{stderr}");
    }

    #[test]
    fn a_sequence_inline_prompt_name_outside_the_map_still_warns() {
        let (undeclared, _) = runs("inline-nope-sequence", "sequence", UNDECLARED_OUTSIDE, false);
        assert_eq!(undeclared.len(), 1, "{undeclared:#?}");
        assert!(undeclared[0].starts_with("`nope`"), "{undeclared:#?}");
    }
}
