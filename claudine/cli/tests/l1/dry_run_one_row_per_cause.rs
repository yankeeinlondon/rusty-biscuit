//! A dry run reports one row for one authored cause, however many of its
//! passes meet it.
//!
//! The check reaches a prompt through several passes (the condition-blind
//! reference walk, the input check, the preview and its retry, a sequence
//! document's own preparation), and each can meet the same typed cause. Each
//! case below names the cause, the route that reaches it, and the exact rows
//! the dry run prints: code, the noun its message names, and the authored
//! position. Its fixed control (the file created, the slot legal) checks
//! clean. Distinct authored sections, and repeated task occurrences of one
//! file, stay distinct rows.
//!
//! - an inline prompt's `prologue:`/`epilogue:` is the section's, at the
//!   frontmatter value, though its generated body is composed apart from the
//!   file;
//! - a `$schema` slot that reads a bare document key is the slot's own typed
//!   row, at the slot, and never also the launch check's
//!   `composition.schema_slot_failed`;
//! - a `$schema` that cannot be loaded or parsed is the launch check's class,
//!   at the `$schema` value, and never also Darkmatter's `dm.schema.prepare`;
//! - a sequence's own failed verdict is not met again by a body step that
//!   composes that document.

use std::fs;

use serde_json::Value;

use crate::common::CliProcessFixture;

const REFERENCE: &str = "dm.reference.missing_transclusion";
const FORBIDDEN_BINDING: &str = "dm.schema.slot_forbidden_binding";
const SCHEMA_LOAD: &str = "composition.schema_load";
const SCHEMA_PARSE: &str = "composition.schema_parse";
const SCHEMA_VALIDATION: &str = "composition.schema_validation";
const NO_EXECUTABLE: &str = "composition.no_executable";
const NOT_CHECKED: &str = "dm.check.not_checked";

/// One expected row: its code, a phrase its message names, where it is
/// (`file`, line, column), and the property of the task it is under.
struct Row {
    code: &'static str,
    noun: &'static str,
    at: Option<(&'static str, u64, u64)>,
    task: Option<&'static str>,
}

const fn row(code: &'static str, noun: &'static str, at: Option<(&'static str, u64, u64)>, task: Option<&'static str>) -> Row {
    Row { code, noun, at, task }
}

/// An inline prompt with a missing `prologue:` (line 4) beside `prompt:`.
const INLINE_PROLOGUE: &str = "---\ntitle: x\nagent: claude\nprologue: ./pro.md\nprompt: \"Hello.\"\n---\n";
/// An inline prompt whose second `epilogue:` item (line 6) is missing.
const INLINE_EPILOGUE: &str = "---\ntitle: x\nagent: claude\nepilogue:\n  - ./ok.md\n  - ./epi.md\nprompt: \"Hello.\"\n---\n";
/// Both sections missing: two authored causes.
const INLINE_BOTH: &str =
    "---\ntitle: x\nagent: claude\nprologue: ./pro.md\nepilogue:\n  - ./ok.md\n  - ./epi.md\nprompt: \"Hello.\"\n---\n";
/// A chained prompt with a missing `prologue:` (line 4).
const DIRECT_PROLOGUE: &str = "---\ntitle: x\nagent: claude\nprologue: ./pro.md\n---\nBody.\n";
/// A sequence whose body step composes the root, with its missing
/// `prologue:` (line 3).
const BODY_PROLOGUE: &str = "---\nagent: claude\nprologue: ./pro.md\nsequence:\n  - name: body\n---\nBody.\n";

/// A schema whose slot reads a bare document key (line 3, column 20).
const BAD_SLOT: &str = "kind: schema\n$schema:\n  choice: \"enum({{ choices }})\"\n";
/// The same slot reading a data file, which a dry run may read.
const LEGAL_SLOT: &str = "kind: schema\n$schema:\n  choice: \"enum({{ load_data(\\\"./choices.yaml\\\", \\\"string[]\\\") }})\"\n";
const SLOT_DIRECT: &str = "---\n$schema: ./s.yaml\nagent: claude\nchoice: a\n---\nBody.\n";
const SLOT_INLINE: &str = "---\n$schema: ./s.yaml\nagent: claude\nchoice: a\nprompt: \"Hello.\"\n---\n";
const SLOT_ROOT: &str = "---\n$schema: ./s.yaml\nagent: claude\nchoice: a\nsequence:\n  - name: one\n    prompt: ./ok.md\n---\n";
const SLOT_BODY: &str = "---\n$schema: ./s.yaml\nagent: claude\nchoice: a\nsequence:\n  - name: body\n---\nBody.\n";

/// A `$schema` (line 2, column 10) naming no file.
const MISSING_SCHEMA: &str = "---\n$schema: ./nope.yaml\nagent: claude\n---\nBody.\n";
const MISSING_SCHEMA_INLINE: &str = "---\n$schema: ./nope.yaml\nagent: claude\nprompt: \"Hello.\"\n---\n";
/// An inline `$schema` whose `count` (line 3, column 10) does not parse.
const UNPARSED_SCHEMA: &str = "---\n$schema:\n  count: \"nmber(min(0)\"\nagent: claude\ncount: 1\n---\nBody.\n";
/// A sequence whose own required value (line 3, column 8) fails its schema,
/// and whose body step composes it.
const ROOT_INVALID_BODY: &str =
    "---\nagent: claude\ntopic: missing\n$schema:\n  topic: enum(a; required)\nsequence:\n  - name: body\n---\nBody {{ topic }}.\n";
/// A sequence whose own `$schema` (line 3, column 10) names no file, and
/// whose body step composes it.
const ROOT_MISSING_SCHEMA_BODY: &str = "---\nagent: claude\n$schema: ./nope.yaml\nsequence:\n  - name: body\n---\nBody.\n";

const OK: &str = "---\ntitle: ok\n---\nOK.\n";

fn single(name: &'static str, text: &str) -> Vec<(&'static str, String)> {
    vec![(name, text.to_string())]
}

/// `files`, plus `ok.md`.
fn with_ok(mut files: Vec<(&'static str, String)>) -> Vec<(&'static str, String)> {
    files.push(("ok.md", OK.to_string()));
    files
}

/// `files`, plus the created targets `pro.md` and `epi.md`.
fn created(files: Vec<(&'static str, String)>) -> Vec<(&'static str, String)> {
    let mut files = with_ok(files);
    files.push(("pro.md", "Prologue.\n".to_string()));
    files.push(("epi.md", "Epilogue.\n".to_string()));
    files
}

/// A sequence of tasks, each running `prompt: <file>`.
fn tasks(prompts: &[&str], doc: &'static str, text: &str) -> Vec<(&'static str, String)> {
    let mut seq = "---\nagent: claude\nsequence:\n".to_string();
    for (index, prompt) in prompts.iter().enumerate() {
        seq.push_str(&format!("  - name: t{index}\n    prompt: ./{prompt}\n"));
    }
    seq.push_str("---\n");
    vec![("seq.md", seq), (doc, text.to_string())]
}

/// `files` beside the bad slot's schema.
fn bad_slot(mut files: Vec<(&'static str, String)>) -> Vec<(&'static str, String)> {
    files.push(("s.yaml", BAD_SLOT.to_string()));
    with_ok(files)
}

/// `files` beside the legal slot's schema and its data.
fn legal_slot(mut files: Vec<(&'static str, String)>) -> Vec<(&'static str, String)> {
    files.push(("s.yaml", LEGAL_SLOT.to_string()));
    files.push(("choices.yaml", "- a\n- b\n".to_string()));
    with_ok(files)
}

/// A fixture holding `files`, with a `claude` on its `PATH` that the check
/// only looks up.
fn staged(name: &str, files: &[(&str, String)]) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    let path = fixture.bin_dir().join(format!("claude{}", std::env::consts::EXE_SUFFIX));
    fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    for (file, text) in files {
        fs::write(fixture.cwd().join(file), text).unwrap();
    }
    fixture
}

/// Asserts that `actual` is exactly `expected`, row for row.
fn assert_rows(actual: &[Value], expected: &[Row]) {
    assert_eq!(actual.len(), expected.len(), "{actual:#?}");
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(actual["code"], expected.code, "{actual:#?}");
        let message = actual["message"].as_str().unwrap();
        assert!(message.contains(expected.noun), "`{}` names `{}`: {actual:#?}", expected.code, expected.noun);
        match expected.at {
            Some((file, line, column)) => {
                let source = &actual["source"];
                let named = source["file"].as_str().unwrap().replace('\\', "/");
                assert!(named.ends_with(&format!("/{file}")), "at {file}: {actual:#?}");
                assert_eq!((source["line"].as_u64(), source["column"].as_u64()), (Some(line), Some(column)), "{actual:#?}");
            }
            None => assert!(actual["source"]["line"].is_null(), "{actual:#?}"),
        }
        let property = actual["trail"].as_array().and_then(|frames| frames.last()).map(|frame| frame["property"].clone());
        assert_eq!(property, expected.task.map(Value::from), "{actual:#?}");
    }
}

/// One `#[test]` per case: the dry run's exit status and its exact rows.
macro_rules! one_row_per_cause {
    ($($name:ident: $files:expr, [$($arg:expr),+] => $exit:expr, [$($row:expr),* $(,)?];)*) => {$(
        #[test]
        fn $name() {
            let fixture = staged(concat!("one-row-", stringify!($name)), &$files);
            let output = fixture.command().args([$($arg),+]).args(["--dry-run", "--json"]).output().unwrap();
            assert!(!fixture.audio_spool().exists());
            let stdout = String::from_utf8_lossy(&output.stdout);
            let report: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| panic!("not JSON:\n{stdout}"));
            let rows = report["diagnostics"].as_array().unwrap();
            assert_eq!(output.status.code(), Some($exit), "{rows:#?}");
            assert_rows(rows, &[$($row),*]);
        }
    )*};
}

one_row_per_cause! {
    // R2-F3.1: standalone inline compose, each section alone, then both.
    inline_prologue_is_one_row_at_its_value: single("notes.md", INLINE_PROLOGUE), ["inline-compose", "notes.md"] => 2,
        [row(REFERENCE, "in prologue:", Some(("notes.md", 4, 11)), None)];
    inline_epilogue_item_is_one_row_at_its_value: with_ok(single("notes.md", INLINE_EPILOGUE)), ["inline-compose", "notes.md"] => 2,
        [row(REFERENCE, "in epilogue:", Some(("notes.md", 6, 5)), None)];
    inline_sections_are_one_row_each: with_ok(single("notes.md", INLINE_BOTH)), ["inline-compose", "notes.md"] => 2, [
        row(REFERENCE, "in prologue:", Some(("notes.md", 4, 11)), None),
        row(REFERENCE, "in epilogue:", Some(("notes.md", 7, 5)), None),
    ];
    inline_sections_created_check_clean: created(single("notes.md", INLINE_BOTH)), ["inline-compose", "notes.md"] => 0, [];

    // R2-F3.2: a sequence's inline prompt file, once per task occurrence.
    inline_prompt_task_sections_are_one_row_per_occurrence: with_ok(tasks(&["notes.md", "notes.md"], "notes.md", INLINE_BOTH)),
        ["sequence", "seq.md"] => 2, [
        row(REFERENCE, "in prologue:", Some(("notes.md", 4, 11)), Some("sequence[0]")),
        row(REFERENCE, "in epilogue:", Some(("notes.md", 7, 5)), Some("sequence[0]")),
        row(REFERENCE, "in prologue:", Some(("notes.md", 4, 11)), Some("sequence[1]")),
        row(REFERENCE, "in epilogue:", Some(("notes.md", 7, 5)), Some("sequence[1]")),
    ];
    inline_prompt_task_sections_created_check_clean: created(tasks(&["notes.md", "notes.md"], "notes.md", INLINE_BOTH)),
        ["sequence", "seq.md"] => 0, [];

    // R2-F3.3: the chained controls keep their one located row.
    direct_prologue_is_one_row_at_its_value: single("doc.md", DIRECT_PROLOGUE), ["compose", "doc.md"] => 2,
        [row(REFERENCE, "in prologue:", Some(("doc.md", 4, 11)), None)];
    direct_prologue_created_checks_clean: created(single("doc.md", DIRECT_PROLOGUE)), ["compose", "doc.md"] => 0, [];
    chained_prompt_task_prologue_is_one_row: tasks(&["doc.md"], "doc.md", DIRECT_PROLOGUE), ["sequence", "seq.md"] => 2,
        [row(REFERENCE, "in prologue:", Some(("doc.md", 4, 11)), Some("sequence[0]"))];
    chained_prompt_task_prologue_created_checks_clean: created(tasks(&["doc.md"], "doc.md", DIRECT_PROLOGUE)),
        ["sequence", "seq.md"] => 0, [];
    body_step_prologue_is_one_row: single("seq.md", BODY_PROLOGUE), ["sequence", "seq.md"] => 2, [
        row(NO_EXECUTABLE, "composes the body", None, Some("sequence[0]")),
        row(REFERENCE, "in prologue:", Some(("seq.md", 3, 11)), Some("sequence[0]")),
    ];
    body_step_prologue_created_checks_clean: created(single("seq.md", BODY_PROLOGUE)), ["sequence", "seq.md"] => 0,
        [row(NO_EXECUTABLE, "composes the body", None, Some("sequence[0]"))];

    // R2-F3.5: an illegal slot is the slot's own row, once per occurrence.
    direct_slot_is_one_typed_row: bad_slot(single("doc.md", SLOT_DIRECT)), ["compose", "doc.md"] => 2,
        [row(FORBIDDEN_BINDING, "cannot read `choices`", Some(("s.yaml", 3, 20)), None)];
    inline_slot_is_one_typed_row: bad_slot(single("notes.md", SLOT_INLINE)), ["inline-compose", "notes.md"] => 2,
        [row(FORBIDDEN_BINDING, "cannot read `choices`", Some(("s.yaml", 3, 20)), None)];
    prompt_task_slot_is_one_typed_row_per_occurrence: bad_slot(tasks(&["doc.md", "doc.md"], "doc.md", SLOT_DIRECT)),
        ["sequence", "seq.md"] => 2, [
        row(FORBIDDEN_BINDING, "cannot read `choices`", Some(("s.yaml", 3, 20)), Some("sequence[0]")),
        row(FORBIDDEN_BINDING, "cannot read `choices`", Some(("s.yaml", 3, 20)), Some("sequence[1]")),
    ];
    sequence_root_slot_is_one_typed_row: bad_slot(single("seq.md", SLOT_ROOT)), ["sequence", "seq.md"] => 2, [
        row(FORBIDDEN_BINDING, "cannot read `choices`", Some(("s.yaml", 3, 20)), None),
        row(NOT_CHECKED, "inputs and preview", None, Some("sequence[0]")),
    ];
    direct_legal_slot_checks_clean: legal_slot(single("doc.md", SLOT_DIRECT)), ["compose", "doc.md"] => 0, [];
    inline_legal_slot_checks_clean: legal_slot(single("notes.md", SLOT_INLINE)), ["inline-compose", "notes.md"] => 0, [];
    prompt_task_legal_slot_checks_clean: legal_slot(tasks(&["doc.md", "doc.md"], "doc.md", SLOT_DIRECT)),
        ["sequence", "seq.md"] => 0, [];

    // R2-F3.6: a `$schema` that cannot be prepared is the launch check's
    // class, at the `$schema` value.
    direct_missing_schema_is_one_row: single("doc.md", MISSING_SCHEMA), ["compose", "doc.md"] => 2,
        [row(SCHEMA_LOAD, "could not resolve $schema reference", Some(("doc.md", 2, 10)), None)];
    inline_missing_schema_is_one_row: single("notes.md", MISSING_SCHEMA_INLINE), ["inline-compose", "notes.md"] => 2,
        [row(SCHEMA_LOAD, "could not resolve $schema reference", Some(("notes.md", 2, 10)), None)];
    prompt_task_missing_schema_is_one_row: tasks(&["doc.md"], "doc.md", MISSING_SCHEMA), ["sequence", "seq.md"] => 2,
        [row(SCHEMA_LOAD, "could not resolve $schema reference", Some(("doc.md", 2, 10)), Some("sequence[0]"))];
    direct_unparsed_schema_is_one_row: single("doc.md", UNPARSED_SCHEMA), ["compose", "doc.md"] => 2,
        [row(SCHEMA_PARSE, "unterminated constraint list", Some(("doc.md", 3, 10)), None)];

    // R2-F3.7: a body step does not meet its own document's failed verdict
    // again.
    body_step_of_a_root_with_a_bad_slot_is_not_previewed: bad_slot(single("seq.md", SLOT_BODY)), ["sequence", "seq.md"] => 2, [
        row(FORBIDDEN_BINDING, "cannot read `choices`", Some(("s.yaml", 3, 20)), None),
        row(NO_EXECUTABLE, "composes the body", None, Some("sequence[0]")),
        row(NOT_CHECKED, "preview (seq.md failed its own checks)", None, Some("sequence[0]")),
    ];
    body_step_of_a_root_with_an_invalid_value_is_not_previewed: single("seq.md", ROOT_INVALID_BODY), ["sequence", "seq.md"] => 2, [
        row(SCHEMA_VALIDATION, "is not one of", Some(("seq.md", 3, 8)), None),
        row(NO_EXECUTABLE, "composes the body", None, Some("sequence[0]")),
        row(NOT_CHECKED, "preview (seq.md failed its own checks)", None, Some("sequence[0]")),
    ];
    body_step_of_a_root_with_a_missing_schema_is_not_previewed: single("seq.md", ROOT_MISSING_SCHEMA_BODY), ["sequence", "seq.md"] => 2, [
        row(SCHEMA_LOAD, "could not resolve $schema reference", Some(("seq.md", 3, 10)), None),
        row(NO_EXECUTABLE, "composes the body", None, Some("sequence[0]")),
        row(NOT_CHECKED, "preview (seq.md failed its own checks)", None, Some("sequence[0]")),
    ];
}
