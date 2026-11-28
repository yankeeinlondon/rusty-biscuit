//! A real `compose`, `inline-compose`, or `sequence` runs the root's own
//! `initialize` before it reads any inclusion, prompt, sequence file,
//! schema, or data file, so a file that stack creates with a literal target
//! (`ensure_file`, `append_line`) is there when the run reads it. The dry run
//! runs no event: it leaves a read of such a file, and what depends on it,
//! `not checked` as known only at launch, instead of reporting it missing.
//!
//! Each row pairs both runs in independent fixtures: the initializer writes
//! the file the reader names (the dry run is not checked, the real run reads
//! it), or an unrelated file (both refuse the missing one). A file that
//! already exists is checked as it is. A prompt's own `initialize` and an
//! earlier task run after the sequence pre-flight reads every inclusion, so
//! a file they create is still refused by both runs, while a sibling read
//! of an initialized file stays unchecked.
//!
//! A destination spelled `{{ target }}` is resolved from the root's own
//! values, as the run resolves it; one the dry run cannot resolve claims no
//! file, yet leaves a read of any file it may create unchecked. An append to
//! an existing file (`append_line`, `append_jsonl`) changes what a reader
//! finds there, so the verdict waits for launch too, while the file's
//! existence stays proven; `ensure_file` leaves an existing file as it is.
//!
//! The real runs use the compiled `claudine-fake-goose`, which appends one
//! line per launch to `PROVIDER_MARKER`; no dry run launches it.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::common::{CliProcessFixture, strip_ansi};

/// What the dry run concludes about the reader.
enum Dry {
    /// Exit 0 and no error, with a `not checked` row for each check, as
    /// known only at launch.
    Unchecked(&'static [&'static str]),
    /// Exit 0, no error, and no `not checked` row.
    Proven,
    /// Exit 2 with an error of this code, and exactly these checks `not
    /// checked` as known only at launch.
    Refused(&'static str, &'static [&'static str]),
}

/// One side of a row: the dry run's conclusion and the real run's exit code
/// and provider launches.
struct Side {
    dry: Dry,
    real: i32,
    launched: usize,
}

const GENERATED: &str = "gen/notes.md";
const UNRELATED: &str = "gen/other.md";

/// A root `initialize` creating `target` with content.
fn ensure(target: &str) -> String {
    format!("initialize:\n  - ensure_file: [{target}, \"Some notes.\"]\n")
}

/// A root `initialize` appending each of `lines` to `target`.
fn append(target: &str, lines: &[&str]) -> String {
    let actions: String = lines.iter().map(|line| format!("  - append_line: [{target}, '{line}']\n")).collect();
    format!("initialize:\n{actions}")
}

/// A sequence whose root runs `init`, then one `child` task running
/// `child.md`.
fn sequence_with_child(init: &str, child: &str) -> Vec<(&'static str, String)> {
    vec![
        ("seq.md", format!("---\nagent: goose\n{init}sequence:\n  - name: child\n    prompt: child.md\n---\n")),
        ("child.md", child.to_string()),
    ]
}

const INCLUDES: &str = "Read:\n\n::file gen/notes.md\n";
const INLINE: &str = "prompt: \"Read:\\n\\n::file gen/notes.md\\n\"\n";

fn direct_compose(target: &str) -> Vec<(&'static str, String)> {
    vec![("doc.md", format!("---\nagent: goose\n{}---\n\n{INCLUDES}", ensure(target)))]
}

fn root_body(target: &str) -> Vec<(&'static str, String)> {
    vec![("seq.md", format!("---\nagent: goose\n{}sequence:\n  - name: body\n---\n\n{INCLUDES}", ensure(target)))]
}

fn inline_compose(target: &str) -> Vec<(&'static str, String)> {
    vec![("doc.md", format!("---\nagent: goose\n{}{INLINE}---\n", ensure(target)))]
}

fn sequence_inline_prompt(target: &str) -> Vec<(&'static str, String)> {
    sequence_with_child(&ensure(target), &format!("---\n{INLINE}---\n"))
}

fn referenced(child: &'static str) -> impl Fn(&str) -> Vec<(&'static str, String)> {
    move |target| sequence_with_child(&ensure(target), child)
}

/// A task of `field` naming `gen.yaml` (or `gen.md`), which the root
/// appends `lines` to; `p.md` is the prompt the generated file runs.
fn graph(field: &'static str, lines: &'static [&'static str]) -> impl Fn(&str) -> Vec<(&'static str, String)> {
    move |target| {
        let file = if field == "prompt" { "gen.md" } else { "gen.yaml" };
        let writes = if target == GENERATED { file } else { "other.yaml" };
        let list = match field {
            "root" => format!("sequence: {file}\n"),
            "catalog" => format!("sequence:\n  - name: t\n    group: g@{file}\n"),
            field => format!("sequence:\n  - name: t\n    {field}: {file}\n"),
        };
        vec![
            ("seq.md", format!("---\nagent: goose\n{}{list}---\n", append(writes, lines))),
            ("p.md", "Do it.\n".to_string()),
        ]
    }
}

const TASK_FILE: &[&str] = &["kind: task", "prompt: p.md"];
const GROUP_FILE: &[&str] = &["kind: group", "tasks:", "  - name: leg", "    prompt: p.md"];
const LIST_FILE: &[&str] = &["- name: inner", "  prompt: p.md"];
const CATALOG_FILE: &[&str] = &["kind: group-catalog", "groups:", "  - name: g", "    tasks:", "      - name: leg", "        prompt: p.md"];

/// `files` with the root initializer's destination spelled `{{ target }}`
/// and `target` set to the destination it wrote: the run resolves it to
/// the same file, so every verdict must stay the same.
fn templated(files: impl Fn(&str) -> Vec<(&'static str, String)>) -> impl Fn(&str) -> Vec<(&'static str, String)> {
    move |target| {
        let mut files = files(target);
        let root = &mut files[0].1;
        let destination = ["ensure_file: [", "append_line: [", "append_jsonl: ["]
            .iter()
            .find_map(|verb| {
                let start = root.find(verb)? + verb.len();
                let end = start + root[start..].find(',')?;
                Some(root[start..end].to_string())
            })
            .expect("the root initializes a file");
        *root = root
            .replace(&format!("[{destination},"), "[\"{{ target }}\",")
            .replacen("initialize:\n", &format!("target: {destination}\ninitialize:\n"), 1);
        files
    }
}

/// A child reading a file the root appends `lines` to (`target` is the
/// generated file's name, or the unrelated one).
fn reader(child: &'static str, file: &'static str, lines: &'static [&'static str]) -> impl Fn(&str) -> Vec<(&'static str, String)> {
    move |target| {
        let writes = if target == GENERATED { file.to_string() } else { format!("{file}.other") };
        sequence_with_child(&append(&writes, lines), child)
    }
}

const SCHEMA_CHILD: &str = "---\n$schema: gen/schema.yaml\ntopic: hi\n---\n\nAbout {{ topic }}.\n";
const SCHEMA_FILE: &[&str] = &["kind: schema", "$schema:", "    topic: string"];
const DATA_CHILD: &str = "About {{ load_data('gen/data.json').name }}.\n";
const DATA_FILE: &[&str] = &["{\"name\": \"x\"}"];
const EXISTS_CHILD: &str = "::block when=\"file_exists('gen/flag.md')\"\nRead.\n::end-block\n";
const FLAG_FILE: &[&str] = &["yes"];
const FACTS_FILE: &[&str] = &["---", "title: Facts", "---", "Facts."];
const TRY_FRONTMATTER_CHILD: &str = "::block when=\"try_frontmatter('gen/facts.md').ok\"\nRead.\n::end-block\n";
const FRONTMATTER_CHILD: &str = "::block when=\"frontmatter('gen/facts.md').title\"\nRead.\n::end-block\n";
const LAZY_TRY_CHILD: &str = "::block when=\"try(frontmatter('gen/facts.md'), {}).title\"\nRead.\n::end-block\n";
const TITLE_CHILD: &str = "::block when=\"markdown_title('gen/facts.md')\"\nRead.\n::end-block\n";
const BODY_CHILD: &str = "::block when=\"!markdown_body_empty('gen/facts.md')\"\nRead.\n::end-block\n";
const VALID_CHILD: &str = "::block when=\"validate_schema('gen/facts.md')\"\nRead.\n::end-block\n";
const FIND_CHILD: &str = "::block when=\"length(find_files('gen/*.md')) > 0\"\nRead.\n::end-block\n";
const ROWS_CHILD: &str = "::block when=\"length(load_data('gen/rows.jsonl')) > 0\"\nRead.\n::end-block\n";
const BLANK_INCLUDE_CHILD: &str = "::file gen/blank.md\n";
const BLANK_BODY_CHILD: &str = "::block when=\"!markdown_body_empty('gen/blank.md')\"\nRead.\n::end-block\n";
const PAGE_CHILD: &str = "::block when=\"validate_schema('page.md')\"\nRead.\n::end-block\n";
const PAGE: &str = "---\n$schema: gen/schema.yaml\ntopic: hi\n---\n";
const IMPORTING_SCHEMA: &str = "kind: schema\n$schema:\n    topic: Topic@types.yaml\n";
const TYPES_FILE: &[&str] = &["kind: schema", "types:", "    Topic: string"];

/// [`reader`] with `extra` files beside it.
fn reader_beside(
    child: &'static str,
    file: &'static str,
    lines: &'static [&'static str],
    extra: &'static [(&'static str, &'static str)],
) -> impl Fn(&str) -> Vec<(&'static str, String)> {
    move |target| {
        let mut files = reader(child, file, lines)(target);
        files.extend(extra.iter().map(|(path, text)| (*path, (*text).to_string())));
        files
    }
}

/// A child reading the existing blank `file`, which the root initializer
/// changes with `action` (`append_line`, `append_jsonl`, `ensure_file`),
/// or, for the unrelated side, leaves alone by writing another file.
fn existing_reader(child: &'static str, file: &'static str, action: &'static str) -> impl Fn(&str) -> Vec<(&'static str, String)> {
    move |target| {
        let writes = if target == GENERATED { file.to_string() } else { format!("{file}.other") };
        // A JSON Lines file gains a row; an object operand is passed
        // through a whole-value span.
        let (values, operand) = match (action, file.ends_with(".jsonl")) {
            ("append_jsonl", _) => ("record:\n  name: x\n", "\"{{ record }}\""),
            (_, true) => ("", "'{\"name\": \"x\"}'"),
            _ => ("", "'More text.'"),
        };
        let mut files = sequence_with_child(&format!("{values}initialize:\n  - {action}: [{writes}, {operand}]\n"), child);
        files.push((file, String::new()));
        files
    }
}

/// R4-F1.9: an existing one-task root list the initializer appends a task
/// naming a missing prompt to (or, unrelated, another file).
fn appended_root_list(target: &str) -> Vec<(&'static str, String)> {
    let writes = if target == GENERATED { "list.yaml" } else { "other.yaml" };
    vec![
        ("seq.md", format!("---\nagent: goose\n{}sequence: list.yaml\n---\n", append(writes, &["- name: two", "  prompt: missing.md"]))),
        ("list.yaml", "- name: one\n  prompt: p.md\n".to_string()),
        ("p.md", "Do it.\n".to_string()),
    ]
}

/// A direct compose whose own `initialize` appends the facts document (or
/// an unrelated file) and whose body is guarded by `try_frontmatter`.
fn direct_try_frontmatter(target: &str) -> Vec<(&'static str, String)> {
    let writes = if target == GENERATED { "gen/facts.md" } else { "gen/facts.md.other" };
    vec![("doc.md", format!("---\nagent: goose\n{}---\n\n{TRY_FRONTMATTER_CHILD}", append(writes, FACTS_FILE)))]
}

fn staged(name: &str, files: &[(&str, String)]) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fs::copy(
        biscuit_test_harness::bin_exe!("claudine-fake-goose"),
        fixture.bin_dir().join(format!("goose{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("the fake provider is built with `test-fixtures`");
    for (path, text) in files {
        let path = fixture.cwd().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
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

fn assert_dry(code: Option<i32>, envelope: &Value, expected: &Dry) {
    let rows: Vec<(String, String, String)> = envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let text = |key: &str| row[key].as_str().unwrap_or_default().to_string();
            (text("code"), text("severity"), text("message"))
        })
        .collect();
    let launch_time: Vec<&String> = rows
        .iter()
        .filter(|(code, _, message)| code == "dm.check.not_checked" && message.contains("known only at launch"))
        .map(|(_, _, message)| message)
        .collect();
    let errors: Vec<&str> = rows.iter().filter(|(_, severity, _)| severity == "error").map(|(code, _, _)| code.as_str()).collect();
    let unchecked = |checks: &[&str]| {
        assert_eq!(launch_time.len(), checks.len(), "{rows:#?}");
        for check in checks {
            assert!(
                launch_time.iter().any(|message| message.contains(&format!("not checked: {check}"))),
                "`{check}` must be unchecked as known only at launch: {rows:#?}"
            );
        }
    };
    match expected {
        Dry::Unchecked(checks) => {
            assert_eq!(code, Some(0), "{rows:#?}");
            assert!(errors.is_empty(), "{rows:#?}");
            unchecked(checks);
        }
        Dry::Proven => {
            assert_eq!(code, Some(0), "{rows:#?}");
            assert!(errors.is_empty() && !rows.iter().any(|(code, _, _)| code == "dm.check.not_checked"), "{rows:#?}");
        }
        Dry::Refused(expected, checks) => {
            assert_eq!(code, Some(2), "{rows:#?}");
            assert_eq!(errors, [*expected], "{rows:#?}");
            unchecked(checks);
        }
    }
}

fn run_side(name: &str, files: &[(&'static str, String)], command: &[&str], side: &Side) {
    let dry = staged(&format!("{name}-dry"), files);
    let (code, envelope) = dry_run(&dry, command);
    assert_dry(code, &envelope, &side.dry);
    let real = staged(&format!("{name}-real"), files);
    let output = real.command().env("PROVIDER_MARKER", marker(&real)).args(command).output().unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.status.code(), Some(side.real), "{stderr}");
    assert_eq!(launches(&real), side.launched, "{stderr}");
}

/// One module per row, with a `#[test]` per side: `files` builds the
/// fixture for the file the root initializer writes, the generated target
/// or an unrelated one.
macro_rules! initialized {
    ($($name:ident: $files:expr, $command:expr, generated: $generated:expr, unrelated: $unrelated:expr;)*) => {$(
        mod $name {
            use super::*;

            #[test]
            fn generated() {
                run_side(concat!("init-files-", stringify!($name)), &$files(GENERATED), &$command, &$generated);
            }

            #[test]
            fn unrelated() {
                run_side(concat!("init-files-unrelated-", stringify!($name)), &$files(UNRELATED), &$command, &$unrelated);
            }
        }
    )*};
}

const SEQUENCE: [&str; 2] = ["sequence", "seq.md"];
const COMPOSE: [&str; 2] = ["compose", "doc.md"];
const INLINE_COMPOSE: [&str; 2] = ["inline-compose", "doc.md"];
const MISSING_INCLUSION: &str = "dm.reference.missing_transclusion";
const MISSING_DOCUMENT: &str = "dm.reference.missing_document";

fn reads(dry: Dry) -> Side {
    Side { dry, real: 0, launched: 1 }
}

fn refuses(code: &'static str) -> Side {
    Side { dry: Dry::Refused(code, &[]), real: 1, launched: 0 }
}

initialized! {
    // R3-F3.1
    a_direct_compose_inclusion_of_an_initialized_file_is_not_checked: direct_compose, COMPOSE,
        generated: reads(Dry::Unchecked(&["transclusion"])), unrelated: refuses(MISSING_INCLUSION);
    a_root_body_inclusion_of_an_initialized_file_is_not_checked: root_body, SEQUENCE,
        generated: reads(Dry::Unchecked(&["transclusion"])), unrelated: refuses(MISSING_INCLUSION);
    // R3-F3.2: the real run reads the file, launches, and then stops at
    // the inline artifact's completion check, which the fake provider
    // does not satisfy; the unrelated miss stops it before launch.
    an_inline_compose_inclusion_of_an_initialized_file_is_not_checked: inline_compose, INLINE_COMPOSE,
        generated: Side { dry: Dry::Unchecked(&["transclusion"]), real: 1, launched: 1 }, unrelated: refuses(MISSING_INCLUSION);
    a_sequence_inline_prompt_inclusion_of_an_initialized_file_is_not_checked: sequence_inline_prompt, SEQUENCE,
        generated: Side { dry: Dry::Unchecked(&["transclusion"]), real: 1, launched: 1 }, unrelated: refuses(MISSING_INCLUSION);
    // R3-F3.3
    a_referenced_prompts_file_directive_on_an_initialized_file_is_not_checked: referenced(INCLUDES), SEQUENCE,
        generated: reads(Dry::Unchecked(&["transclusion"])), unrelated: refuses(MISSING_INCLUSION);
    a_referenced_prompts_code_directive_on_an_initialized_file_is_not_checked: referenced("Read:\n\n::code gen/notes.md\n"), SEQUENCE,
        generated: reads(Dry::Unchecked(&["transclusion"])), unrelated: refuses(MISSING_INCLUSION);
    a_referenced_prompts_prologue_on_an_initialized_file_is_not_checked: referenced("---\nprologue: gen/notes.md\n---\n\nRead.\n"), SEQUENCE,
        generated: reads(Dry::Unchecked(&["transclusion"])), unrelated: refuses(MISSING_INCLUSION);
    a_referenced_prompts_epilogue_on_an_initialized_file_is_not_checked: referenced("---\nepilogue: gen/notes.md\n---\n\nRead.\n"), SEQUENCE,
        generated: reads(Dry::Unchecked(&["transclusion"])), unrelated: refuses(MISSING_INCLUSION);
    // R3-F3.4: the task (or, for the root list, its tasks) is not read,
    // so nothing below it is checked.
    an_initialized_prompt_file_leaves_its_task_unchecked: graph("prompt", &["Generated prompt."]), SEQUENCE,
        generated: reads(Dry::Unchecked(&["task"])), unrelated: refuses(MISSING_DOCUMENT);
    an_initialized_task_file_leaves_its_task_unchecked: graph("task", TASK_FILE), SEQUENCE,
        generated: reads(Dry::Unchecked(&["task"])), unrelated: refuses(MISSING_DOCUMENT);
    an_initialized_group_file_leaves_its_task_unchecked: graph("group", GROUP_FILE), SEQUENCE,
        generated: reads(Dry::Unchecked(&["task"])), unrelated: refuses(MISSING_DOCUMENT);
    an_initialized_nested_sequence_file_leaves_its_task_unchecked: graph("sequence", LIST_FILE), SEQUENCE,
        generated: reads(Dry::Unchecked(&["task"])), unrelated: refuses(MISSING_DOCUMENT);
    an_initialized_root_sequence_file_leaves_the_tasks_unchecked: graph("root", LIST_FILE), SEQUENCE,
        generated: reads(Dry::Unchecked(&["tasks"])), unrelated: refuses(MISSING_DOCUMENT);
    // R3-F3.6: the schema is not prepared, so the composition it judges is
    // not checked either.
    an_initialized_schema_file_leaves_the_schema_and_preview_unchecked: reader(SCHEMA_CHILD, "gen/schema.yaml", SCHEMA_FILE), SEQUENCE,
        generated: reads(Dry::Unchecked(&["schema", "preview"])), unrelated: refuses("composition.schema_load");
    an_initialized_data_file_leaves_its_interpolation_unchecked: reader(DATA_CHILD, "gen/data.json", DATA_FILE), SEQUENCE,
        generated: reads(Dry::Unchecked(&["interpolation"])), unrelated: refuses("dm.expression.evaluation_failure");
    an_initialized_flag_file_leaves_the_block_condition_unchecked: reader(EXISTS_CHILD, "gen/flag.md", FLAG_FILE), SEQUENCE,
        generated: reads(Dry::Unchecked(&["`::block` condition"])), unrelated: refuses("dm.compose.empty_body");
    // R4-F3.2: `try_frontmatter` keeps the deferral instead of reading
    // `ok: false`; an ordinary miss is still `ok: false`, an empty body.
    an_initialized_document_leaves_a_try_frontmatter_condition_unchecked: reader(TRY_FRONTMATTER_CHILD, "gen/facts.md", FACTS_FILE), SEQUENCE,
        generated: reads(Dry::Unchecked(&["`::block` condition"])), unrelated: refuses("dm.compose.empty_body");
    a_direct_compose_initialized_document_leaves_a_try_frontmatter_condition_unchecked: direct_try_frontmatter, COMPOSE,
        generated: reads(Dry::Unchecked(&["`::block` condition"])), unrelated: refuses("dm.compose.empty_body");
    // R4-F3.3: the direct reader refuses an ordinary miss; the lazy `try`
    // recovers it into its fallback.
    an_initialized_document_leaves_a_frontmatter_condition_unchecked: reader(FRONTMATTER_CHILD, "gen/facts.md", FACTS_FILE), SEQUENCE,
        generated: reads(Dry::Unchecked(&["`::block` condition"])), unrelated: refuses("dm.expression.evaluation_failure");
    an_initialized_document_leaves_a_lazy_try_condition_unchecked: reader(LAZY_TRY_CHILD, "gen/facts.md", FACTS_FILE), SEQUENCE,
        generated: reads(Dry::Unchecked(&["`::block` condition"])), unrelated: refuses("dm.compose.empty_body");
}

const BLOCK: &[&str] = &["`::block` condition"];
const EVALUATION: &str = "dm.expression.evaluation_failure";
const EMPTY_BODY: &str = "dm.compose.empty_body";

// R4-F1.1–.7: a destination spelled `{{ target }}` resolves to the same file
// as the literal one, so each reader keeps the literal verdict, and an
// unrelated resolved destination keeps the real refusal.
initialized! {
    // R4-F1.1
    a_templated_direct_compose_inclusion_is_not_checked: templated(direct_compose), COMPOSE,
        generated: reads(Dry::Unchecked(&["transclusion"])), unrelated: refuses(MISSING_INCLUSION);
    a_templated_root_body_inclusion_is_not_checked: templated(root_body), SEQUENCE,
        generated: reads(Dry::Unchecked(&["transclusion"])), unrelated: refuses(MISSING_INCLUSION);
    a_templated_inline_compose_inclusion_is_not_checked: templated(inline_compose), INLINE_COMPOSE,
        generated: Side { dry: Dry::Unchecked(&["transclusion"]), real: 1, launched: 1 }, unrelated: refuses(MISSING_INCLUSION);
    a_templated_sequence_inline_prompt_inclusion_is_not_checked: templated(sequence_inline_prompt), SEQUENCE,
        generated: Side { dry: Dry::Unchecked(&["transclusion"]), real: 1, launched: 1 }, unrelated: refuses(MISSING_INCLUSION);
    a_templated_referenced_prompt_inclusion_is_not_checked: templated(referenced(INCLUDES)), SEQUENCE,
        generated: reads(Dry::Unchecked(&["transclusion"])), unrelated: refuses(MISSING_INCLUSION);
    // R4-F1.2
    a_templated_code_directive_is_not_checked: templated(referenced("Read:\n\n::code gen/notes.md\n")), SEQUENCE,
        generated: reads(Dry::Unchecked(&["transclusion"])), unrelated: refuses(MISSING_INCLUSION);
    a_templated_prologue_is_not_checked: templated(referenced("---\nprologue: gen/notes.md\n---\n\nRead.\n")), SEQUENCE,
        generated: reads(Dry::Unchecked(&["transclusion"])), unrelated: refuses(MISSING_INCLUSION);
    a_templated_epilogue_is_not_checked: templated(referenced("---\nepilogue: gen/notes.md\n---\n\nRead.\n")), SEQUENCE,
        generated: reads(Dry::Unchecked(&["transclusion"])), unrelated: refuses(MISSING_INCLUSION);
    // R4-F1.3
    a_templated_prompt_file_leaves_its_task_unchecked: templated(graph("prompt", &["Generated prompt."])), SEQUENCE,
        generated: reads(Dry::Unchecked(&["task"])), unrelated: refuses(MISSING_DOCUMENT);
    a_templated_task_file_leaves_its_task_unchecked: templated(graph("task", TASK_FILE)), SEQUENCE,
        generated: reads(Dry::Unchecked(&["task"])), unrelated: refuses(MISSING_DOCUMENT);
    a_templated_group_file_leaves_its_task_unchecked: templated(graph("group", GROUP_FILE)), SEQUENCE,
        generated: reads(Dry::Unchecked(&["task"])), unrelated: refuses(MISSING_DOCUMENT);
    an_initialized_catalog_file_leaves_its_task_unchecked: graph("catalog", CATALOG_FILE), SEQUENCE,
        generated: reads(Dry::Unchecked(&["task"])), unrelated: refuses(MISSING_DOCUMENT);
    a_templated_catalog_file_leaves_its_task_unchecked: templated(graph("catalog", CATALOG_FILE)), SEQUENCE,
        generated: reads(Dry::Unchecked(&["task"])), unrelated: refuses(MISSING_DOCUMENT);
    a_templated_nested_sequence_file_leaves_its_task_unchecked: templated(graph("sequence", LIST_FILE)), SEQUENCE,
        generated: reads(Dry::Unchecked(&["task"])), unrelated: refuses(MISSING_DOCUMENT);
    a_templated_root_sequence_file_leaves_the_tasks_unchecked: templated(graph("root", LIST_FILE)), SEQUENCE,
        generated: reads(Dry::Unchecked(&["tasks"])), unrelated: refuses(MISSING_DOCUMENT);
    // R4-F1.4
    a_templated_schema_file_leaves_the_schema_and_preview_unchecked: templated(reader(SCHEMA_CHILD, "gen/schema.yaml", SCHEMA_FILE)), SEQUENCE,
        generated: reads(Dry::Unchecked(&["schema", "preview"])), unrelated: refuses("composition.schema_load");
    // R4-F1.5
    a_templated_data_file_leaves_its_interpolation_unchecked: templated(reader(DATA_CHILD, "gen/data.json", DATA_FILE)), SEQUENCE,
        generated: reads(Dry::Unchecked(&["interpolation"])), unrelated: refuses(EVALUATION);
    a_templated_frontmatter_read_is_not_checked: templated(reader(FRONTMATTER_CHILD, "gen/facts.md", FACTS_FILE)), SEQUENCE,
        generated: reads(Dry::Unchecked(BLOCK)), unrelated: refuses(EVALUATION);
    an_initialized_title_read_is_not_checked: reader(TITLE_CHILD, "gen/facts.md", FACTS_FILE), SEQUENCE,
        generated: reads(Dry::Unchecked(BLOCK)), unrelated: refuses(EVALUATION);
    a_templated_title_read_is_not_checked: templated(reader(TITLE_CHILD, "gen/facts.md", FACTS_FILE)), SEQUENCE,
        generated: reads(Dry::Unchecked(BLOCK)), unrelated: refuses(EVALUATION);
    an_initialized_body_empty_read_is_not_checked: reader(BODY_CHILD, "gen/facts.md", FACTS_FILE), SEQUENCE,
        generated: reads(Dry::Unchecked(BLOCK)), unrelated: refuses(EVALUATION);
    a_templated_body_empty_read_is_not_checked: templated(reader(BODY_CHILD, "gen/facts.md", FACTS_FILE)), SEQUENCE,
        generated: reads(Dry::Unchecked(BLOCK)), unrelated: refuses(EVALUATION);
    an_initialized_validated_document_is_not_checked: reader(VALID_CHILD, "gen/facts.md", FACTS_FILE), SEQUENCE,
        generated: reads(Dry::Unchecked(BLOCK)), unrelated: refuses(EVALUATION);
    a_templated_validated_document_is_not_checked: templated(reader(VALID_CHILD, "gen/facts.md", FACTS_FILE)), SEQUENCE,
        generated: reads(Dry::Unchecked(BLOCK)), unrelated: refuses(EVALUATION);
    // R4-F1.6
    a_templated_flag_file_leaves_the_block_condition_unchecked: templated(reader(EXISTS_CHILD, "gen/flag.md", FLAG_FILE)), SEQUENCE,
        generated: reads(Dry::Unchecked(BLOCK)), unrelated: refuses(EMPTY_BODY);
    an_initialized_file_leaves_a_listing_unchecked: reader(FIND_CHILD, "gen/flag.md", FLAG_FILE), SEQUENCE,
        generated: reads(Dry::Unchecked(BLOCK)), unrelated: refuses(EMPTY_BODY);
    a_templated_file_leaves_a_listing_unchecked: templated(reader(FIND_CHILD, "gen/flag.md", FLAG_FILE)), SEQUENCE,
        generated: reads(Dry::Unchecked(BLOCK)), unrelated: refuses(EMPTY_BODY);
    // R4-F1.7: the page exists; its schema, or the schema's named-type
    // import, is the initialized file.
    an_initialized_schema_read_by_validate_schema_is_not_checked:
        reader_beside(PAGE_CHILD, "gen/schema.yaml", SCHEMA_FILE, &[("page.md", PAGE)]), SEQUENCE,
        generated: reads(Dry::Unchecked(BLOCK)), unrelated: refuses(EVALUATION);
    an_initialized_named_type_import_read_by_validate_schema_is_not_checked:
        reader_beside(PAGE_CHILD, "gen/types.yaml", TYPES_FILE, &[("page.md", PAGE), ("gen/schema.yaml", IMPORTING_SCHEMA)]), SEQUENCE,
        generated: reads(Dry::Unchecked(BLOCK)), unrelated: refuses(EVALUATION);
    // R4-F1.8: an append to an existing blank file changes what the reader
    // finds; an unrelated append leaves the empty-body verdict.
    an_appended_line_leaves_a_data_condition_unchecked: existing_reader(ROWS_CHILD, "gen/rows.jsonl", "append_line"), SEQUENCE,
        generated: reads(Dry::Unchecked(BLOCK)), unrelated: refuses(EMPTY_BODY);
    an_appended_record_leaves_a_data_condition_unchecked: existing_reader(ROWS_CHILD, "gen/rows.jsonl", "append_jsonl"), SEQUENCE,
        generated: reads(Dry::Unchecked(BLOCK)), unrelated: refuses(EMPTY_BODY);
    an_appended_include_is_not_checked: existing_reader(BLANK_INCLUDE_CHILD, "gen/blank.md", "append_line"), SEQUENCE,
        generated: reads(Dry::Unchecked(&["transclusion"])), unrelated: refuses(EMPTY_BODY);
    an_appended_document_leaves_a_body_empty_condition_unchecked: existing_reader(BLANK_BODY_CHILD, "gen/blank.md", "append_line"), SEQUENCE,
        generated: reads(Dry::Unchecked(BLOCK)), unrelated: refuses(EMPTY_BODY);
    // R4-F1.9
    an_appended_root_list_leaves_the_tasks_unchecked: appended_root_list, SEQUENCE,
        generated: Side { dry: Dry::Unchecked(&["tasks"]), real: 1, launched: 0 }, unrelated: reads(Dry::Proven);
}

/// R4-F1.8: `ensure_file` leaves an existing file as it is, so the reader's
/// verdict on its current bytes stands.
macro_rules! ensured_existing {
    ($($name:ident: $child:expr, $file:expr;)*) => {$(
        #[test]
        fn $name() {
            run_side(concat!("init-files-ensured-", stringify!($name)), &existing_reader($child, $file, "ensure_file")(GENERATED), &SEQUENCE, &refuses(EMPTY_BODY));
        }
    )*};
}

ensured_existing! {
    an_ensured_existing_data_file_keeps_the_empty_body_verdict: ROWS_CHILD, "gen/rows.jsonl";
    an_ensured_existing_include_keeps_the_empty_body_verdict: BLANK_INCLUDE_CHILD, "gen/blank.md";
    an_ensured_existing_document_keeps_the_empty_body_verdict: BLANK_BODY_CHILD, "gen/blank.md";
}

/// R3-F3.5: the prompt's own `initialize`, and an earlier task, run after
/// the sequence pre-flight has read every inclusion, so only the generated
/// side exists.
macro_rules! not_initialized {
    ($($name:ident: $files:expr, $command:expr, $side:expr;)*) => {$(
        #[test]
        fn $name() {
            run_side(concat!("init-files-", stringify!($name)), &$files(GENERATED), &$command, &$side);
        }
    )*};
}

not_initialized! {
    a_prompts_own_initializer_does_not_create_the_file_before_preflight: |target: &str| {
        sequence_with_child("", &format!("---\n{}---\n\n{INCLUDES}", ensure(target)))
    }, SEQUENCE,
        Side { dry: Dry::Refused(MISSING_INCLUSION, &[]), real: 1, launched: 0 };
    an_earlier_tasks_initializer_does_not_create_the_file_before_preflight: |target: &str| vec![
        ("seq.md", "---\nagent: goose\nsequence:\n  - name: writer\n    prompt: writer.md\n  - name: child\n    prompt: child.md\n---\n".to_string()),
        ("writer.md", format!("---\n{}---\n\nWrite.\n", ensure(target))),
        ("child.md", INCLUDES.to_string()),
    ], SEQUENCE,
        Side { dry: Dry::Refused(MISSING_INCLUSION, &[]), real: 1, launched: 0 };
    an_independent_sibling_inclusion_is_still_checked: referenced("Read:\n\n::file gen/notes.md\n\n::file missing.md\n"), SEQUENCE,
        Side { dry: Dry::Refused(MISSING_INCLUSION, &["transclusion"]), real: 1, launched: 0 };
    // R4-F1.10
    an_independent_sibling_beside_a_templated_file_is_still_checked:
        templated(referenced("Read:\n\n::file gen/notes.md\n\n::file missing.md\n")), SEQUENCE,
        Side { dry: Dry::Refused(MISSING_INCLUSION, &["transclusion"]), real: 1, launched: 0 };
    // A destination the dry run cannot resolve claims no file, but no file
    // it may create is reported missing: here any file in the folder.
    an_unresolvable_destination_leaves_a_read_it_may_create_unchecked: |_: &str| {
        sequence_with_child("target: Notes\ninitialize:\n  - ensure_file: [\"gen/{{ lower(target) }}.md\", \"Some notes.\"]\n", INCLUDES)
    }, SEQUENCE,
        reads(Dry::Unchecked(&["transclusion"]));
    // Its fixed text still bounds where it writes.
    an_unresolvable_destination_leaves_a_read_it_cannot_create_refused: |_: &str| {
        sequence_with_child(
            "target: Notes\ninitialize:\n  - ensure_file: [\"other/{{ lower(target) }}.md\", \"Some notes.\"]\n",
            INCLUDES,
        )
    }, SEQUENCE,
        refuses(MISSING_INCLUSION);
}

/// A file the initializer writes that already exists is read as it is.
macro_rules! existing {
    ($($name:ident: $files:expr, $existing:expr;)*) => {$(
        #[test]
        fn $name() {
            let mut files = $files(GENERATED);
            files.push($existing);
            run_side(concat!("init-files-existing-", stringify!($name)), &files, &SEQUENCE, &reads(Dry::Proven));
        }
    )*};
}

existing! {
    an_existing_initialized_inclusion_is_checked_as_it_is: referenced(INCLUDES), ("gen/notes.md", "Notes.\n".to_string());
    an_existing_initialized_prompt_file_is_checked_as_it_is: |_: &str| vec![
        ("seq.md", format!("---\nagent: goose\n{}sequence:\n  - name: t\n    prompt: gen.md\n---\n", ensure("gen.md"))),
    ], ("gen.md", "Generated prompt.\n".to_string());
    // R4-F1.6: an append changes what the file holds, never whether it
    // exists.
    an_existing_appended_flag_file_still_exists: reader(EXISTS_CHILD, "gen/flag.md", FLAG_FILE), ("gen/flag.md", "yes\n".to_string());
    an_existing_initialized_schema_is_checked_as_it_is: |target: &str| sequence_with_child(&ensure(target), SCHEMA_CHILD),
        ("gen/schema.yaml", "kind: schema\n$schema:\n    topic: string\n".to_string());
    an_existing_initialized_data_file_is_checked_as_it_is: |target: &str| sequence_with_child(&ensure(target), DATA_CHILD),
        ("gen/data.json", "{\"name\": \"x\"}\n".to_string());
}

/// R4-F1.8: an append to an existing prompt file changes the prompt the
/// task runs, so the task is not checked; the real run reads the appended
/// prompt.
#[test]
fn an_existing_appended_prompt_file_leaves_its_task_unchecked() {
    let mut files = graph("prompt", &["More."])(GENERATED);
    files.push(("gen.md", "Generated prompt.\n".to_string()));
    run_side("init-files-appended-prompt", &files, &SEQUENCE, &reads(Dry::Unchecked(&["task"])));
}

/// R4-F1.10: a templated destination outside the mutation root is refused
/// by the effect as the literal one is.
#[test]
fn a_templated_initializer_outside_the_mutation_root_creates_nothing() {
    let files = templated(|_: &str| sequence_with_child(&ensure(OUTSIDE), READS_OUTSIDE))(GENERATED);
    run_side("init-files-outside-templated", &files, &SEQUENCE, &refuses(MISSING_INCLUSION));
}

/// An initializer target outside the mutation root (the launch folder,
/// outside any repository) is one the effect refuses: the real
/// `initialize` fails before any read, so the file is never created, and
/// the dry run reads the missing file as it is instead of leaving it to
/// launch.
const OUTSIDE: &str = "../outside/notes.md";
const READS_OUTSIDE: &str = "Read:\n\n::file ../outside/notes.md\n";

macro_rules! outside_root {
    ($($name:ident: $files:expr, $command:expr;)*) => {$(
        #[test]
        fn $name() {
            run_side(concat!("init-files-outside-", stringify!($name)), &$files, &$command, &refuses(MISSING_INCLUSION));
        }
    )*};
}

outside_root! {
    a_direct_compose_initializer_outside_the_mutation_root_creates_nothing:
        [("doc.md", format!("---\nagent: goose\n{}---\n\n{READS_OUTSIDE}", ensure(OUTSIDE)))], COMPOSE;
    a_root_body_initializer_outside_the_mutation_root_creates_nothing:
        [("seq.md", format!("---\nagent: goose\n{}sequence:\n  - name: body\n---\n\n{READS_OUTSIDE}", ensure(OUTSIDE)))], SEQUENCE;
    a_sequence_initializer_outside_the_mutation_root_creates_nothing_for_a_prompt:
        sequence_with_child(&ensure(OUTSIDE), READS_OUTSIDE), SEQUENCE;
}
