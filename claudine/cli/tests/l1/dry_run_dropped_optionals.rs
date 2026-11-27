//! A dry run grades a schema problem as the real route does. `compose`,
//! `inline-compose`, and a root step composing the root drop an optional
//! property whose value fails its schema, with a warning, and run without it;
//! a sequence's prompt task (and a nested body step) is composed by the
//! task's pre-launch barrier, which refuses it. So the dry run's row is a
//! `composition.optional_dropped` warning (exit 2, as for every warning) on
//! the first routes and the schema's own error on the others, and a required
//! property's invalid or missing value is an error everywhere.
//!
//! Each row pairs the dry run with the real non-interactive run of the same
//! files, through the compiled `claudine-fake-goose`, which appends one line
//! per launch to `PROVIDER_MARKER` (and, for `inline.md`, writes its body).

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::common::{CliProcessFixture, strip_ansi};

const DROPPED: &str = "composition.optional_dropped";

/// What the dry run concludes.
enum Dry {
    /// Exit 2 with no error and a `composition.optional_dropped` warning
    /// naming each property.
    Dropped(&'static [&'static str]),
    /// Exit 2 with an error row of this code, and a
    /// `composition.optional_dropped` warning for exactly these properties.
    Refused(&'static str, &'static [&'static str]),
    /// Exit 2 with exactly one error, of this code, for the invalid required
    /// value, and a `composition.optional_dropped` warning for each invalid
    /// optional beside it, which the real run drops once that value is
    /// fixed. The real run's error lists the required value alone.
    RefusedBeside(&'static str, &'static [&'static str]),
}

const OPTIONAL: &str = "---\nagent: goose\ntopic: missing\n$schema:\n  topic: enum(a)\n---\n\nRead {{ topic }}.\n";
const OPTIONAL_TEMPLATE: &str =
    "---\nagent: goose\ntopic: \"{{ 'missing' }}\"\n$schema:\n  topic: enum(a)\n---\n\nRead {{ topic }}.\n";
const OPTIONAL_NUMBER: &str = "---\nagent: goose\ncount: many\n$schema:\n  count: number\n---\n\nRead {{ count }}.\n";
const REQUIRED: &str = "---\nagent: goose\ntopic: missing\n$schema:\n  topic: enum(a; required)\n---\n\nRead {{ topic }}.\n";
const REQUIRED_ABSENT: &str = "---\nagent: goose\n$schema:\n  topic: string(required)\n---\n\nRead {{ topic }}.\n";
const INLINE: &str = "---\nagent: goose\ntopic: missing\n$schema:\n  topic: enum(a)\nprompt: \"Write about {{ topic }}.\"\n---\n";
/// An invalid required `count` beside an invalid optional `topic`, each
/// literal or a template the composition renders.
const BESIDE_LITERALS: &str =
    "---\nagent: goose\ntopic: missing\ncount: many\n$schema:\n  topic: enum(a)\n  count: number(required)\n---\n\nRead {{ topic }}.\n";
const BESIDE_TEMPLATES: &str = "---\nagent: goose\ntopic: \"{{ 'missing' }}\"\ncount: \"{{ 'many' }}\"\n$schema:\n  topic: enum(a)\n  count: number(required)\n---\n\nRead {{ topic }}.\n";
const BESIDE_INLINE: &str = "---\nagent: goose\ntopic: \"{{ 'missing' }}\"\ncount: many\n$schema:\n  topic: enum(a)\n  count: number(required)\nprompt: \"Write about {{ topic }}.\"\n---\n";
/// A sequence root's literal optional is dropped by the root's own
/// pre-validation, which warns before the root's verdict refuses `count`.
const BESIDE_SEQUENCE_BODY: &str = "---\nagent: goose\ntopic: missing\ncount: \"{{ 'many' }}\"\n$schema:\n  topic: enum(a)\n  count: number(required)\nsequence:\n  - name: body\n---\n\nRead {{ topic }}.\n";
const SEQUENCE_BODY: &str =
    "---\nagent: goose\ntopic: missing\n$schema:\n  topic: enum(a)\nsequence:\n  - name: body\n---\n\nRead {{ topic }}.\n";

fn single(name: &'static str, text: &str) -> Vec<(&'static str, String)> {
    vec![(name, text.to_string())]
}

/// A sequence whose one task runs `doc.md` as `task` (its executable line).
fn prompt_task(task: &str, doc: &'static str, text: &str) -> Vec<(&'static str, String)> {
    vec![
        ("seq.md", format!("---\nagent: goose\nsequence:\n  - name: reader\n    {task}\n---\n")),
        (doc, text.to_string()),
    ]
}

/// A root whose own optional is invalid, beside a valid prompt task.
fn root_optional() -> Vec<(&'static str, String)> {
    vec![
        (
            "seq.md",
            "---\nagent: goose\ntopic: missing\n$schema:\n  topic: enum(a)\nsequence:\n  - name: reader\n    prompt: ok.md\n---\n"
                .to_string(),
        ),
        ("ok.md", "---\nagent: goose\n---\n\nRead.\n".to_string()),
    ]
}

/// The root's body composed by a step of an inline nested sequence.
fn nested_body() -> Vec<(&'static str, String)> {
    single(
        "seq.md",
        "---\nagent: goose\ntopic: missing\n$schema:\n  topic: enum(a)\nsequence:\n  - name: inner\n    sequence:\n      - name: body\n---\n\nRead {{ topic }}.\n",
    )
}

/// A referenced sequence document's own body step.
fn referenced_body() -> Vec<(&'static str, String)> {
    vec![
        ("seq.md", "---\nagent: goose\nsequence:\n  - name: outer\n    sequence: child.md\n---\n".to_string()),
        ("child.md", SEQUENCE_BODY.to_string()),
    ]
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

fn run(fixture: &CliProcessFixture, command: &[&str], extra: &[&str]) -> std::process::Output {
    let mut cmd = fixture.command();
    cmd.env("PROVIDER_MARKER", marker(fixture)).args(command).args(extra);
    let inline = fixture.cwd().join("inline.md");
    if inline.exists() {
        cmd.env("FAKE_INLINE_DOCUMENT", inline);
    }
    cmd.output().unwrap()
}

/// Every row as `(code, severity, message)`.
fn rows(envelope: &Value) -> Vec<(String, String, String)> {
    envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let text = |value: &Value| value.as_str().unwrap_or_default().to_string();
            (text(&row["code"]), text(&row["severity"]), text(&row["message"]))
        })
        .collect()
}

fn assert_dry(code: Option<i32>, envelope: &Value, expected: &Dry) {
    let rows = rows(envelope);
    let errors: Vec<&str> = rows.iter().filter(|row| row.1 == "error").map(|row| row.0.as_str()).collect();
    let dropped: Vec<&(String, String, String)> = rows.iter().filter(|row| row.0 == DROPPED).collect();
    assert_eq!(code, Some(2), "{rows:#?}");
    assert!(dropped.iter().all(|row| row.1 == "warning"), "{rows:#?}");
    let (expected_dropped, refused) = match expected {
        Dry::Dropped(properties) => (*properties, None),
        Dry::Refused(code, properties) => (*properties, Some(*code)),
        Dry::RefusedBeside(code, properties) => {
            assert_eq!(errors, [*code], "the required value's error alone: {rows:#?}");
            (*properties, Some(*code))
        }
    };
    match refused {
        None => assert!(errors.is_empty(), "{rows:#?}"),
        Some(code) => assert!(errors.contains(&code), "expected `{code}`: {rows:#?}"),
    }
    assert_eq!(dropped.len(), expected_dropped.len(), "{rows:#?}");
    for property in expected_dropped {
        let needle = format!("dropped optional schema property `{property}`");
        assert!(dropped.iter().any(|row| row.2.contains(&needle)), "one `{DROPPED}` row for `{property}`: {rows:#?}");
    }
}

/// One `#[test]` per row: the dry run's conclusion, then the real run's exit
/// code and launch count for the same files.
macro_rules! dropped_optionals {
    ($($name:ident: $files:expr, $command:expr, $dry:expr, real: $real:expr, launched: $launched:expr;)*) => {$(
        #[test]
        fn $name() {
            let fixture = staged(concat!("dropped-optional-", stringify!($name)), &$files);
            let command: &[&str] = &$command;
            let output = run(&fixture, command, &["--dry-run", "--json"]);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let envelope: Value = serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
            assert_eq!(launches(&fixture), 0, "a dry run launched a provider");
            assert_dry(output.status.code(), &envelope, &$dry);
            let real = run(&fixture, command, &[]);
            let stderr = strip_ansi(&String::from_utf8_lossy(&real.stderr));
            assert_eq!(real.status.code(), Some($real), "{stderr}");
            assert_eq!(launches(&fixture), $launched, "{stderr}");
            if matches!($dry, Dry::Dropped(_) | Dry::Refused(_, [_, ..])) {
                assert!(stderr.contains("warning: dropped optional schema property"), "{stderr}");
            }
        }
    )*};
}

const SEQUENCE: [&str; 2] = ["sequence", "seq.md"];

dropped_optionals! {
    compose_drops_an_invalid_optional_literal: single("doc.md", OPTIONAL), ["compose", "doc.md"],
        Dry::Dropped(&["topic"]), real: 0, launched: 1;
    compose_drops_an_invalid_optional_template_at_composition: single("doc.md", OPTIONAL_TEMPLATE), ["compose", "doc.md"],
        Dry::Dropped(&["topic"]), real: 0, launched: 1;
    compose_drops_an_optional_of_the_wrong_type: single("doc.md", OPTIONAL_NUMBER), ["compose", "doc.md"],
        Dry::Dropped(&["count"]), real: 0, launched: 1;
    compose_drops_an_invalid_optional_setter: single("doc.md", REQUIRED_ABSENT.replace("string(required)", "enum(a)").as_str()),
        ["compose", "doc.md", "topic=missing"],
        Dry::Dropped(&["topic"]), real: 0, launched: 1;
    compose_refuses_an_invalid_required_value: single("doc.md", REQUIRED), ["compose", "doc.md"],
        Dry::Refused("dm.schema.constraint", &[]), real: 1, launched: 0;
    compose_refuses_a_missing_required_value: single("doc.md", REQUIRED_ABSENT), ["compose", "doc.md"],
        Dry::Refused("composition.missing_properties", &[]), real: 1, launched: 0;
    inline_compose_drops_an_invalid_optional: single("inline.md", INLINE), ["inline-compose", "inline.md"],
        Dry::Dropped(&["topic"]), real: 0, launched: 1;
    compose_refuses_the_required_value_beside_a_literal_optional: single("doc.md", BESIDE_LITERALS), ["compose", "doc.md"],
        Dry::RefusedBeside("dm.schema.type_mismatch", &["topic"]), real: 1, launched: 0;
    compose_refuses_the_required_value_beside_a_template_optional: single("doc.md", BESIDE_TEMPLATES), ["compose", "doc.md"],
        Dry::RefusedBeside("dm.schema.type_mismatch", &["topic"]), real: 1, launched: 0;
    inline_compose_refuses_the_required_value_beside_an_optional: single("inline.md", BESIDE_INLINE), ["inline-compose", "inline.md"],
        Dry::RefusedBeside("dm.schema.type_mismatch", &["topic"]), real: 1, launched: 0;
    // The root's own verdict refuses before its body step composes.
    a_root_body_step_refuses_the_required_value_beside_an_optional: single("seq.md", BESIDE_SEQUENCE_BODY), SEQUENCE,
        Dry::RefusedBeside("composition.schema_validation", &["topic"]), real: 1, launched: 0;
    a_root_body_step_drops_the_roots_invalid_optional: single("seq.md", SEQUENCE_BODY), SEQUENCE,
        Dry::Dropped(&["topic"]), real: 0, launched: 1;
    a_sequence_root_drops_its_own_invalid_optional: root_optional(), SEQUENCE,
        Dry::Dropped(&["topic"]), real: 0, launched: 1;
    a_prompt_task_refuses_its_documents_invalid_optional: prompt_task("prompt: doc.md", "doc.md", OPTIONAL), SEQUENCE,
        Dry::Refused("dm.schema.constraint", &[]), real: 1, launched: 0;
    an_inline_prompt_task_refuses_its_documents_invalid_optional: prompt_task("prompt: inline.md\n    inline-compose: true", "inline.md", INLINE), SEQUENCE,
        Dry::Refused("dm.schema.constraint", &[]), real: 1, launched: 0;
    // The root drops it at the start, as the real run warns; the nested
    // step's barrier then rereads the file and refuses it.
    a_nested_body_step_refuses_the_roots_invalid_optional: nested_body(), SEQUENCE,
        Dry::Refused("dm.schema.constraint", &["topic"]), real: 1, launched: 0;
    a_referenced_documents_body_step_refuses_its_invalid_optional: referenced_body(), SEQUENCE,
        Dry::Refused("dm.schema.constraint", &[]), real: 1, launched: 0;
}
