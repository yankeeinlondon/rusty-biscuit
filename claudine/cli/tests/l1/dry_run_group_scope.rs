//! A prompt inside a group reads its group's `group` values, which the run
//! binds only when the group starts (from `variables` its events may
//! change), so the dry run reads `group` as known only at launch there: a
//! check that reads it is `not checked`, never the `claudine.outside-group`
//! scope error of a standalone composition. This holds wherever the group
//! is declared (inline, a `kind: group` file, a catalog entry) and below
//! it (a nested sequence in a leg). Outside every group, `group` is still
//! the scope error both runs refuse. Passes a real run makes before any
//! group starts (the condition-blind reference walk, approval discovery)
//! bind no group, so what they refuse is still reported, at that boundary.
//!
//! Each row pairs the dry run with the real non-interactive run of the
//! same files, which uses the compiled `claudine-fake-goose`; it appends
//! one line per launch to `PROVIDER_MARKER`.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::common::{CliProcessFixture, strip_ansi};

const BLOCK: &str = "`::block` condition";
const COLLECTION: &str = "`::map`/`::filter` collection";
const OUTSIDE_GROUP: &str = "outside-group";

/// What the dry run concludes.
enum Dry {
    /// Exit 0, no error, and a `not checked` row naming each check.
    Unchecked(&'static [&'static str]),
    /// Exit 0 and no `not checked` row.
    Proven,
    /// Exit 2 with exactly these error codes, in any order.
    Refused(&'static [&'static str]),
}

/// Where the reader's task sits.
#[derive(Clone, Copy)]
enum Placement {
    /// A leg of an inline group.
    Inline,
    /// A leg of a `kind: group` file.
    GroupFile,
    /// A leg of a `kind: group-catalog` entry.
    Catalog,
    /// A task of a nested sequence inside an inline group's leg.
    Nested,
    /// A plain sequence step, in no group.
    Outside,
}

const VARIABLES: &str = "{ topic: research, items: [1, 2] }";

/// The sequence placing a task running `reader.md` (with `with`, if any, as
/// its `with:`) at `placement`, under a group whose `topic` is `topic`, and
/// `reader.md` itself.
fn placed_with(placement: Placement, reader: &str, with: &str, topic: &str) -> Vec<(&'static str, String)> {
    let variables = VARIABLES.replace("research", topic);
    let leg = |indent: &str| {
        let with = if with.is_empty() { String::new() } else { format!("{indent}  with: {with}\n") };
        format!("{indent}- name: leg\n{indent}  prompt: reader.md\n{with}")
    };
    let mut files = match placement {
        Placement::Inline => vec![(
            "seq.md",
            format!("---\nagent: goose\nsequence:\n  - name: g\n    group:\n      name: g\n      variables: {variables}\n      tasks:\n{}---\n", leg("        ")),
        )],
        Placement::GroupFile => vec![
            ("seq.md", "---\nagent: goose\nsequence:\n  - name: g\n    group: g.yaml\n---\n".to_string()),
            ("g.yaml", format!("kind: group\nname: g\nvariables: {variables}\ntasks:\n{}", leg("  "))),
        ],
        Placement::Catalog => vec![
            ("seq.md", "---\nagent: goose\nsequence:\n  - name: g\n    group: g@catalog.yaml\n---\n".to_string()),
            ("catalog.yaml", format!("kind: group-catalog\ngroups:\n  - name: g\n    variables: {variables}\n    tasks:\n{}", leg("      "))),
        ],
        Placement::Nested => vec![(
            "seq.md",
            format!(
                "---\nagent: goose\nsequence:\n  - name: g\n    group:\n      name: g\n      variables: {variables}\n      tasks:\n        - name: inner\n          sequence:\n{}---\n",
                leg("            ")
            ),
        )],
        Placement::Outside => vec![("seq.md", format!("---\nagent: goose\nsequence:\n{}---\n", leg("  ")))],
    };
    files.push(("reader.md", reader.to_string()));
    files
}

fn placed(placement: Placement, reader: &str) -> Vec<(&'static str, String)> {
    placed_with(placement, reader, "", "research")
}

/// A reader whose whole body is guarded by `condition`.
fn guarded(condition: &str) -> String {
    format!("---\n---\n\n::block when=\"{condition}\"\nRead.\n::end-block\n")
}

/// A reader holding `region` between composed text, over a declared
/// `items` list.
fn collection(region: &str) -> String {
    format!("---\nitems: [1, 2]\n$schema:\n  items: \"number[]\"\n---\n\nIntro.\n\n{region}\nOutro.\n")
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

const COMMAND: [&str; 2] = ["sequence", "seq.md"];

fn dry_run(fixture: &CliProcessFixture) -> (Option<i32>, Value) {
    let output = fixture
        .command()
        .env("PROVIDER_MARKER", marker(fixture))
        .args(COMMAND)
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

fn assert_dry(code: Option<i32>, envelope: &Value, expected: &Dry, outside: bool) {
    let rows = rows(envelope);
    let unchecked: Vec<&String> =
        rows.iter().filter(|(code, _, _)| code == "dm.check.not_checked").map(|(_, _, message)| message).collect();
    let mut errors: Vec<&str> = rows.iter().filter(|(_, severity, _)| severity == "error").map(|(code, _, _)| code.as_str()).collect();
    // Inside a group, no check reports the group as out of scope.
    if !outside {
        assert!(rows.iter().all(|(_, _, message)| !message.contains(OUTSIDE_GROUP)), "{rows:#?}");
    }
    match expected {
        Dry::Unchecked(checks) => {
            assert_eq!(code, Some(0), "{rows:#?}");
            assert!(errors.is_empty(), "{rows:#?}");
            for check in *checks {
                assert!(
                    unchecked.iter().any(|message| message.contains(&format!("not checked: {check}"))
                        && (message.contains("known only at launch") || message.contains("known only when the task runs"))),
                    "`{check}` must be unchecked as known only at launch: {rows:#?}"
                );
            }
        }
        Dry::Proven => {
            assert_eq!(code, Some(0), "{rows:#?}");
            assert!(errors.is_empty() && unchecked.is_empty(), "{rows:#?}");
        }
        Dry::Refused(expected) => {
            assert_eq!(code, Some(2), "{rows:#?}");
            errors.sort_unstable();
            let mut expected = expected.to_vec();
            expected.sort_unstable();
            assert_eq!(errors, expected, "{rows:#?}");
        }
    }
}

/// One `#[test]` per row: the dry run's conclusion, then the real run's
/// exit code and launch count for the same files. A real run that stops
/// before any task prints the passive check's report, which must not call
/// an in-scope group out of scope either.
macro_rules! group_case {
    ($($name:ident: $files:expr, $dry:expr, real: $real:expr, launched: $launched:expr $(, $outside:ident)?;)*) => {$(
        #[test]
        fn $name() {
            let fixture = staged(concat!("group-", stringify!($name)), &$files);
            let outside = false $(|| stringify!($outside) == "outside")?;
            let (code, envelope) = dry_run(&fixture);
            assert_dry(code, &envelope, &$dry, outside);
            let output = fixture.command().env("PROVIDER_MARKER", marker(&fixture)).args(COMMAND).output().unwrap();
            let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
            assert_eq!(output.status.code(), Some($real), "{stderr}");
            assert_eq!(launches(&fixture), $launched, "{stderr}");
            if !outside {
                assert!(!stderr.contains(OUTSIDE_GROUP), "{stderr}");
            }
        }
    )*};
}

const DOTTED: &str = "group.topic == 'research'";
const BRACKET: &str = "group['topic'] == 'research'";
const PARENTHESIZED: &str = "(group).topic == 'research'";
const LITERAL: &str = "'research' == 'research'";

group_case! {
    // R4-F2.1: every spelling, every placement.
    an_inline_group_leg_dotted_read_is_conditional: placed(Placement::Inline, &guarded(DOTTED)),
        Dry::Unchecked(&[BLOCK]), real: 0, launched: 1;
    an_inline_group_leg_bracket_read_is_conditional: placed(Placement::Inline, &guarded(BRACKET)),
        Dry::Unchecked(&[BLOCK]), real: 0, launched: 1;
    an_inline_group_leg_parenthesized_read_is_conditional: placed(Placement::Inline, &guarded(PARENTHESIZED)),
        Dry::Unchecked(&[BLOCK]), real: 0, launched: 1;
    a_group_file_leg_dotted_read_is_conditional: placed(Placement::GroupFile, &guarded(DOTTED)),
        Dry::Unchecked(&[BLOCK]), real: 0, launched: 1;
    a_group_file_leg_bracket_read_is_conditional: placed(Placement::GroupFile, &guarded(BRACKET)),
        Dry::Unchecked(&[BLOCK]), real: 0, launched: 1;
    a_group_file_leg_parenthesized_read_is_conditional: placed(Placement::GroupFile, &guarded(PARENTHESIZED)),
        Dry::Unchecked(&[BLOCK]), real: 0, launched: 1;
    a_catalog_entry_leg_dotted_read_is_conditional: placed(Placement::Catalog, &guarded(DOTTED)),
        Dry::Unchecked(&[BLOCK]), real: 0, launched: 1;
    a_catalog_entry_leg_bracket_read_is_conditional: placed(Placement::Catalog, &guarded(BRACKET)),
        Dry::Unchecked(&[BLOCK]), real: 0, launched: 1;
    a_catalog_entry_leg_parenthesized_read_is_conditional: placed(Placement::Catalog, &guarded(PARENTHESIZED)),
        Dry::Unchecked(&[BLOCK]), real: 0, launched: 1;
    a_nested_sequence_in_a_leg_dotted_read_is_conditional: placed(Placement::Nested, &guarded(DOTTED)),
        Dry::Unchecked(&[BLOCK]), real: 0, launched: 1;
    a_nested_sequence_in_a_leg_bracket_read_is_conditional: placed(Placement::Nested, &guarded(BRACKET)),
        Dry::Unchecked(&[BLOCK]), real: 0, launched: 1;
    a_nested_sequence_in_a_leg_parenthesized_read_is_conditional: placed(Placement::Nested, &guarded(PARENTHESIZED)),
        Dry::Unchecked(&[BLOCK]), real: 0, launched: 1;
    // The value is not predicted: another authored topic leaves the dry run
    // conditional, while the live prompt is empty.
    another_topic_is_not_predicted: placed_with(Placement::Inline, &guarded(DOTTED), "", "other"),
        Dry::Unchecked(&[BLOCK]), real: 1, launched: 0;
    a_literal_true_condition_in_a_group_composes: placed(Placement::Inline, &guarded(LITERAL)),
        Dry::Proven, real: 0, launched: 1;
    a_dotted_read_outside_any_group_is_refused_by_both: placed(Placement::Outside, &guarded(DOTTED)),
        Dry::Refused(&["dm.expression.evaluation_failure"]), real: 1, launched: 0, outside;
    a_bracket_read_outside_any_group_is_refused_by_both: placed(Placement::Outside, &guarded(BRACKET)),
        Dry::Refused(&["dm.expression.evaluation_failure"]), real: 1, launched: 0, outside;
    a_parenthesized_read_outside_any_group_is_refused_by_both: placed(Placement::Outside, &guarded(PARENTHESIZED)),
        Dry::Refused(&["dm.expression.evaluation_failure"]), real: 1, launched: 0, outside;

    // R4-F2.2
    a_body_interpolation_of_group_is_unchecked: placed(Placement::Inline, "---\n---\n\nRead {{ group.topic }}.\n"),
        Dry::Unchecked(&["interpolation"]), real: 0, launched: 1;
    a_frontmatter_value_of_group_is_unchecked: placed(Placement::Inline, "---\nsubject: \"{{ group.topic }}\"\n---\n\nRead {{ subject }}.\n"),
        Dry::Unchecked(&["interpolation", "frontmatter value"]), real: 0, launched: 1;
    an_enum_on_a_group_value_draws_no_verdict: placed(
        Placement::Inline,
        "---\nsubject: \"{{ group.topic }}\"\n$schema:\n  subject: enum(research)\n---\n\nRead {{ subject }}.\n",
    ), Dry::Unchecked(&["interpolation", "frontmatter value"]), real: 0, launched: 1;
    an_enum_on_a_static_value_in_a_group_is_still_judged: placed(
        Placement::Inline,
        "---\nsubject: other\n$schema:\n  subject: enum(research)\n---\n\nRead {{ subject }}.\n",
    ), Dry::Refused(&["dm.schema.constraint"]), real: 1, launched: 0;
    a_body_interpolation_of_group_outside_any_group_is_refused: placed(Placement::Outside, "---\n---\n\nRead {{ group.topic }}.\n"),
        Dry::Refused(&["dm.expression.evaluation_failure"]), real: 1, launched: 0, outside;

    // R4-F2.3: the stub writes no artifact, so the inline run fails after
    // its launch.
    an_inline_prompt_on_a_leg_is_conditional: placed(
        Placement::Inline,
        "---\nprompt: |\n  ::block when=\"group.topic == 'research'\"\n  Read.\n  ::end-block\n---\n",
    ), Dry::Unchecked(&[BLOCK]), real: 1, launched: 1;
    // Outside a group the inline prompt meets the scope error, which this
    // route reports as its composition failure.
    an_inline_prompt_outside_any_group_is_refused: placed(
        Placement::Outside,
        "---\nprompt: |\n  ::block when=\"group.topic == 'research'\"\n  Read.\n  ::end-block\n---\n",
    ), Dry::Refused(&["composition.failed"]), real: 1, launched: 0, outside;

    // R4-F2.5
    a_static_inclusion_guarded_by_group_is_conditional: {
        let mut files = placed(Placement::Inline, "---\n---\n\nIntro.\n\n::file ./static.md when=\"group.topic == 'research'\"\n");
        files.push(("static.md", "Included.\n".to_string()));
        files
    }, Dry::Unchecked(&["transclusion"]), real: 0, launched: 1;
    a_missing_static_inclusion_guarded_by_group_is_still_reported: placed(
        Placement::Inline,
        "---\n---\n\nIntro.\n\n::file ./static.md when=\"group.topic == 'research'\"\n",
    ), Dry::Refused(&["dm.reference.missing_transclusion"]), real: 1, launched: 0;

    // R4-F2.7: a `with:` value reading `group` is known at the task's turn.
    a_with_input_reading_group_is_known_at_the_tasks_turn: placed_with(
        Placement::Inline,
        "---\n$schema:\n  topic: string(required)\n---\n\nRead {{ topic }}.\n",
        "{ topic: \"{{ group.topic }}\" }",
        "research",
    ), Dry::Unchecked(&["input `topic`"]), real: 0, launched: 1;
}

/// R4-F2.6: an inclusion target named by a group value. The real run's
/// pre-flight resolves it before any group starts, as `./.md`, and stops
/// there; the dry run reports that refusal once, and leaves the preview's
/// group-dependent target unchecked rather than out of scope.
mod named_by_group {
    use super::*;

    fn files(reader: &str) -> Vec<(&'static str, String)> {
        let mut files = placed(Placement::Inline, reader);
        files.push(("research.md", "Included.\n".to_string()));
        files
    }

    group_case! {
        a_file_target: files("---\n---\n\n::file ./{{ group.topic }}.md\n"),
            Dry::Refused(&["dm.reference.missing_transclusion"]), real: 1, launched: 0;
        a_code_target: files("---\n---\n\n::code ./{{ group.topic }}.md\n"),
            Dry::Refused(&["dm.reference.missing_transclusion"]), real: 1, launched: 0;
        a_prologue_target: files("---\nprologue: ./{{ group.topic }}.md\n---\n\nBody.\n"),
            Dry::Refused(&["dm.reference.missing_transclusion"]), real: 1, launched: 0;
        an_epilogue_target: files("---\nepilogue: ./{{ group.topic }}.md\n---\n\nBody.\n"),
            Dry::Refused(&["dm.reference.missing_transclusion"]), real: 1, launched: 0;
    }

    /// The refused target's own preview is `not checked`, not a scope error.
    #[test]
    fn the_group_dependent_preview_is_unchecked() {
        let fixture = staged("group-named-preview", &files("---\n---\n\n::file ./{{ group.topic }}.md\n"));
        let (_, envelope) = dry_run(&fixture);
        let rows = rows(&envelope);
        assert!(
            rows.iter().any(|(code, _, message)| code == "dm.check.not_checked"
                && message.contains("transclusion")
                && message.contains("known only at launch")),
            "{rows:#?}"
        );
    }
}

/// R4-F2.4: a group read in each collection site of both directives is one
/// collection deferral (a page block inside one is its condition's), the
/// literal rewrite composes, and an untyped `::map group.items` is still the
/// declaration error both runs refuse before reading any value.
mod collections {
    use super::*;

    fn positional(directive: &str, read: &str) -> String {
        let callback = if directive == "map" { format!("i => {read} ? i : 0") } else { format!("i => {read}") };
        format!("::{directive} items \"{callback}\"\n- {{{{ i }}}}\n::end-{directive}\n")
    }

    fn with_option(directive: &str, option: &str) -> String {
        let callback = if directive == "map" { "i => i" } else { "i => true" };
        format!("::{directive} items \"{callback}\" {option}\n- {{{{ i }}}}\n::end-{directive}\n")
    }

    fn option(directive: &str, name: &str, read: &str) -> String {
        match name {
            "sort" => with_option(directive, &format!("sort=\"i => length({}) + i\"", read.split(" ==").next().unwrap())),
            "until" => with_option(directive, &format!("until=\"i => !({read})\"")),
            "when" => with_option(directive, &format!("when=\"{read}\"")),
            name => with_option(directive, &format!("{name}=\"i => {read}\"")),
        }
    }

    fn nested_block(directive: &str, read: &str) -> String {
        let callback = if directive == "map" { "i => i" } else { "i => true" };
        format!("::{directive} items \"{callback}\"\n- item {{{{ i }}}}\n::block when=\"{read}\"\nguarded {{{{ i }}}}\n::end-block\n::end-{directive}\n")
    }

    /// One `#[test]` per directive and site: the group read is deferred, and
    /// the literal rewrite of the same region composes.
    macro_rules! site {
        ($($name:ident: $region:expr, $check:expr;)*) => {$(
            #[test]
            fn $name() {
                let region: fn(&str) -> String = $region;
                let fixture = staged(concat!("group-collection-", stringify!($name)), &placed(Placement::Inline, &collection(&region(DOTTED))));
                let (code, envelope) = dry_run(&fixture);
                assert_dry(code, &envelope, &Dry::Unchecked(&[$check]), false);
                let fixture = staged(concat!("group-collection-literal-", stringify!($name)), &placed(Placement::Inline, &collection(&region(LITERAL))));
                let (code, envelope) = dry_run(&fixture);
                assert_dry(code, &envelope, &Dry::Proven, false);
            }
        )*};
    }

    site! {
        map_positional: |read| positional("map", read), COLLECTION;
        filter_positional: |read| positional("filter", read), COLLECTION;
        map_when: |read| option("map", "when", read), COLLECTION;
        filter_when: |read| option("filter", "when", read), COLLECTION;
        map_filter: |read| option("map", "filter", read), COLLECTION;
        filter_filter: |read| option("filter", "filter", read), COLLECTION;
        map_sort: |read| option("map", "sort", read), COLLECTION;
        filter_sort: |read| option("filter", "sort", read), COLLECTION;
        map_while: |read| option("map", "while", read), COLLECTION;
        filter_while: |read| option("filter", "while", read), COLLECTION;
        map_until: |read| option("map", "until", read), COLLECTION;
        filter_until: |read| option("filter", "until", read), COLLECTION;
        map_page_block: |read| nested_block("map", read), BLOCK;
        filter_page_block: |read| nested_block("filter", read), BLOCK;
    }

    group_case! {
        a_collection_reading_group_launches: placed(Placement::Inline, &collection(&positional("map", DOTTED))),
            Dry::Unchecked(&[COLLECTION]), real: 0, launched: 1;
        an_untyped_group_source_is_still_a_declaration_error: placed(
            Placement::Inline,
            "---\n---\n\n::map group.items \"i => i\"\n- {{ i }}\n::end-map\n",
        ), Dry::Refused(&["dm.collection.untyped_map_source"]), real: 1, launched: 0;
    }
}

/// R4-F2.7: the task's `agent` and `model` read through `with:` from
/// `group` are decided at launch, and a templated shell leg is a command
/// the dry run does not run, whose approval the real run asks for.
mod task_consumers {
    use super::*;

    #[test]
    fn a_with_agent_and_model_reading_group_are_decided_at_launch() {
        let mut files = placed_with(
            Placement::Inline,
            "---\n---\n\nRead.\n",
            "{ agent: \"{{ group.topic }}\", model: \"{{ group.topic }}\" }",
            "goose",
        );
        files[0].1 = files[0].1.replace("agent: goose\n", "");
        let fixture = staged("group-with-agent", &files);
        let (code, envelope) = dry_run(&fixture);
        assert_dry(code, &envelope, &Dry::Unchecked(&["provider", "model requirement", "input `agent`, `model`"]), false);
        let output = fixture.command().env("PROVIDER_MARKER", marker(&fixture)).args(COMMAND).output().unwrap();
        let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
        assert_eq!(output.status.code(), Some(0), "{stderr}");
        assert_eq!(launches(&fixture), 1, "{stderr}");
    }

    #[test]
    fn a_templated_shell_leg_is_not_run() {
        let mut files = placed(Placement::Inline, "---\n---\n\nRead.\n");
        files[0].1 = files[0].1.replace("prompt: reader.md", "shell: echo {{ group.topic }}");
        let fixture = staged("group-shell-leg", &files);
        let (code, envelope) = dry_run(&fixture);
        let rows = rows(&envelope);
        assert_eq!(code, Some(0), "{rows:#?}");
        assert!(rows.iter().any(|(_, _, message)| message.contains("not checked: command")), "{rows:#?}");
        assert!(rows.iter().all(|(_, _, message)| !message.contains(OUTSIDE_GROUP)), "{rows:#?}");
        // Non-interactively the real run cannot ask for the command's approval.
        let output = fixture.command().env("PROVIDER_MARKER", marker(&fixture)).args(COMMAND).output().unwrap();
        let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
        assert_eq!(output.status.code(), Some(1), "{stderr}");
        assert!(!stderr.contains(OUTSIDE_GROUP), "{stderr}");
    }
}
