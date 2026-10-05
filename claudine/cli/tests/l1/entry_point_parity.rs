//! The entry-point parity matrix's claudine-cli runner. Table 1's documents
//! run through Claudine composition (`claudine compose --dry-run`), launched
//! from the fixture repository root. Table 2's values, from the repository
//! root and from the package, run through three entry points:
//!
//! - completion (`claudine __complete … compose <value> resolved=`): the value
//!   is the committed prompt, and the `resolved` suggestions name the target
//!   it resolved to;
//! - the prompt argument composed (`claudine compose --dry-run <value>`), the
//!   execution of the same value, which also compares a failure's class;
//! - a supplied schema `file` value (`claudine compose --dry-run <document>
//!   target=<value>`).
//!
//! Completion and the prompt argument run in their own fixture, whose every
//! target declares `$schema: { resolved: enum(t<N>) }` (`N` its index in
//! [`ParityFixture::targets`]), so completion names the file it resolved.
//! Composition and supplied values run in a fixture with plain targets.
//!
//! Glob rows: composition lists `::file-links` and `find_files()` and
//! validates `match()` for a frontmatter value (Table 1); completion offers a
//! `file(match(...))` property's candidates (`claudine __complete … compose
//! <prompt> spec=`) and a supplied `spec=<value>` is validated, from each
//! launch directory (Table 2). The chooser's walk is the binary's unit test
//! (`completion/schema_completion/parity_tests.rs`).
//!
//! Every spawn goes through `CliProcessFixture`, whose `home/` is the child's
//! `HOME` (`USERPROFILE` on Windows) and the fixture `HOME` of the matrix; the
//! binary captures that process as its request snapshot on every OS. The
//! matrix's configured extra `@` root is Claudine's own user prompt root
//! (`~/.claudine/prompts`), which the binary registers from that home. A
//! failure's class is read only from the
//! stable `failure: <name>` row of the rendered file-reference error, never
//! from message text.

#[path = "../../../../darkmatter/lib/tests/common/entry_point_parity/mod.rs"]
mod matrix;

use std::path::{Path, PathBuf};
use std::process::Output;

use biscuit_file::{ResolutionFailure, to_portable_string};
use matrix::{
    Consumer, CrossRepositoryFixture, DocumentCell, EntryPoint, Expected, GlobConsumer, GlobDocumentCell, GlobValueCell,
    LAUNCH_MAGIC, Observed, Owner, ParityFixture, ParityReport, Row, SOURCE_MAGIC, ValueCell, rows_for,
};

use crate::common::{CliProcessFixture, strip_ansi};

/// The class named by the first `failure: <name>` row in `text`.
fn failure_row(text: &str) -> Option<ResolutionFailure> {
    let (_, rest) = text.split_once("failure:")?;
    let name: String = rest
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_lowercase() || *c == '-')
        .collect();
    match name.as_str() {
        "invalid-reference" => Some(ResolutionFailure::InvalidReference),
        "missing-context" => Some(ResolutionFailure::MissingContext),
        "no-match" => Some(ResolutionFailure::NoMatch),
        "io" => Some(ResolutionFailure::Io),
        "unsupported-remote" => Some(ResolutionFailure::UnsupportedRemote),
        _ => None,
    }
}

/// The composed body of a dry run: stdout up to the separator that opens the
/// resolved-frontmatter report, which names the document but no target.
fn composed_body(stdout: &str) -> &str {
    stdout.split('\u{254c}').next().unwrap_or(stdout)
}

/// The result of one dry run. A run whose output names a failure reports its
/// class, whatever the exit status.
fn observe(fixture: &ParityFixture, consumer: Consumer, tree_root: &Path, output: &Output) -> Observed {
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    if let Some(failure) = failure_row(&stderr).or_else(|| failure_row(&stdout)) {
        return Observed::Failure(failure);
    }
    if !output.status.success() {
        return Observed::Unexpected(format!("{} without a failure row: {stderr}", output.status));
    }
    let body = composed_body(&stdout);
    let targets = match consumer {
        Consumer::SchemaFile => fixture.stored_value_targets(body, tree_root),
        Consumer::File | Consumer::Code | Consumer::TocLinking | Consumer::MarkdownLink => {
            fixture.marked_targets(body)
        }
    };
    match targets.as_slice() {
        [path] => Observed::File(path.clone()),
        _ => Observed::Unexpected(format!("{} targets in stdout {stdout:?}", targets.len())),
    }
}

/// The argument naming `document` from the repository root: relative inside
/// the repository, absolute outside it.
fn document_argument(fixture: &ParityFixture, document: &Path) -> String {
    match document.strip_prefix(fixture.repo()) {
        Ok(relative) => to_portable_string(relative),
        Err(_) => document.to_string_lossy().into_owned(),
    }
}

fn compose(cli: &CliProcessFixture, fixture: &ParityFixture, cell: &DocumentCell) -> Observed {
    let document = fixture.write_document(cell);
    let output = cli
        .command_builder()
        .ambient_context(&fixture.repo())
        .build()
        .args(["compose", "--dry-run"])
        .arg(document_argument(fixture, &document))
        .output()
        .expect("run claudine compose --dry-run");
    observe(fixture, cell.consumer, &fixture.tree_root(cell), &output)
}

/// The document every Table 2 value is supplied to: one eager `file`
/// property, rendered as a `stored-value=` line.
fn value_document(fixture: &ParityFixture) -> PathBuf {
    fixture.repo().join("value-cell.md")
}

/// The schema property every completion-fixture target declares.
const RESOLVED_PROPERTY: &str = "resolved";

/// Gives every target of the completion fixture a `$schema` whose one enum
/// member names the target, keeping its `## TARGET <id>` body.
fn declare_target_schemas(fixture: &ParityFixture) {
    for (index, target) in fixture.targets().iter().enumerate() {
        let body = std::fs::read_to_string(target).expect("read a target");
        let declared = format!("---\n$schema:\n  {RESOLVED_PROPERTY}: enum(t{index})\n---\n\n{body}");
        std::fs::write(target, declared).expect("declare a target's schema");
    }
}

/// `claudine __complete` for `claudine compose <value> resolved=` from the
/// cell's launch directory. Completion names no failure class: no
/// suggestions is [`Observed::Unresolved`].
fn completion(cli: &CliProcessFixture, fixture: &ParityFixture, cell: &ValueCell) -> Observed {
    let value = fixture.value(cell);
    let setter = format!("{RESOLVED_PROPERTY}=");
    let output = cli
        .command_builder()
        .ambient_context(&fixture.launch_dir(cell.launch))
        .build()
        .args(["__complete", "--current", "3", "--", "claudine", "compose"])
        .arg(&value)
        .arg(&setter)
        .output()
        .expect("run claudine __complete");
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        return Observed::Unexpected(format!(
            "{}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let targets = fixture.targets();
    let mut named = Vec::new();
    for line in stdout.lines().filter(|line| !line.is_empty()) {
        let member = line
            .strip_prefix(setter.as_str())
            .map(|rest| rest.trim_matches('\''))
            .and_then(|member| member.strip_prefix('t'))
            .and_then(|index| index.parse::<usize>().ok())
            .and_then(|index| targets.get(index));
        match member {
            Some(target) => named.push(target.clone()),
            None => return Observed::Unexpected(format!("suggestion {line:?} names no target")),
        }
    }
    match named.as_slice() {
        [] => Observed::Unresolved,
        [path] => Observed::File(path.clone()),
        _ => Observed::Unexpected(format!("{} targets suggested: {stdout:?}", named.len())),
    }
}

/// `claudine compose --dry-run <value>` from the cell's launch directory: the
/// prompt argument completion resolves, composed.
fn prompt_argument(cli: &CliProcessFixture, fixture: &ParityFixture, cell: &ValueCell) -> Observed {
    let value = fixture.value(cell);
    let output = cli
        .command_builder()
        .ambient_context(&fixture.launch_dir(cell.launch))
        .build()
        .args(["compose", "--dry-run"])
        .arg(&value)
        .output()
        .expect("run claudine compose --dry-run with a prompt argument");
    observe(fixture, Consumer::File, &fixture.repo(), &output)
}

/// `claudine compose --dry-run <document> target=<value>` from the cell's
/// launch directory.
fn supplied_value(cli: &CliProcessFixture, fixture: &ParityFixture, cell: &ValueCell) -> Observed {
    let value = fixture.value(cell);
    let output = cli
        .command_builder()
        .ambient_context(&fixture.launch_dir(cell.launch))
        .build()
        .args(["compose", "--dry-run"])
        .arg(value_document(fixture))
        .arg(format!("target={value}"))
        .output()
        .expect("run claudine compose --dry-run with a supplied value");
    observe(fixture, Consumer::SchemaFile, &fixture.repo(), &output)
}

/// A failed run's output, for [`matrix::validation_verdicts`].
fn outcome(output: &Output) -> Result<(), String> {
    if output.status.success() {
        Ok(())
    } else {
        Err(strip_ansi(&format!(
            "{}{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        )))
    }
}

/// `claudine compose --dry-run <document>` from the repository root.
fn dry_run(cli: &CliProcessFixture, fixture: &ParityFixture, document: &Path) -> Output {
    cli.command_builder()
        .ambient_context(&fixture.repo())
        .build()
        .args(["compose", "--dry-run"])
        .arg(document_argument(fixture, document))
        .output()
        .expect("run claudine compose --dry-run")
}

/// Composition of a Table 1 glob cell: the files `::file-links` or
/// `find_files()` lists, or each `match()` candidate's verdict.
fn compose_glob(cli: &CliProcessFixture, fixture: &ParityFixture, cell: &GlobDocumentCell) -> Observed {
    if cell.consumer == GlobConsumer::MatchValidation {
        let outcomes = fixture
            .glob_match_documents(cell)
            .into_iter()
            .map(|(candidate, document)| (candidate, outcome(&dry_run(cli, fixture, &document))))
            .collect();
        return matrix::validation_verdicts(outcomes);
    }
    let output = dry_run(cli, fixture, &fixture.glob_document(cell));
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    if let Some(failure) = failure_row(&stderr).or_else(|| failure_row(&stdout)) {
        return Observed::Failure(failure);
    }
    if !output.status.success() {
        return Observed::Unexpected(format!("{} without a failure row: {stderr}", output.status));
    }
    let body = composed_body(&stdout);
    match cell.consumer {
        GlobConsumer::FileLinks => match fixture.file_links_listed(body) {
            files if files.is_empty() => Observed::Unexpected(format!("no `::file-links` tree in {body:?}")),
            files => Observed::FileSet(files),
        },
        GlobConsumer::FindFiles => fixture
            .find_files_listed(body)
            .map_or_else(|| Observed::Unexpected(format!("no `find_files()` line in {body:?}")), Observed::Files),
        GlobConsumer::MatchValidation | GlobConsumer::MatchCompletion => unreachable!("{cell:?} lists no files"),
    }
}

/// `claudine __complete … compose <prompt> spec=` from the cell's launch
/// directory: the files the candidates name, in order.
fn complete_glob(cli: &CliProcessFixture, fixture: &ParityFixture, cell: &GlobValueCell) -> Observed {
    let output = cli
        .command_builder()
        .ambient_context(&fixture.launch_dir(cell.launch))
        .build()
        .args(["__complete", "--current", "3", "--", "claudine", "compose"])
        .arg(fixture.glob_value_document(cell))
        .arg("spec=")
        .output()
        .expect("run claudine __complete");
    if !output.status.success() {
        return Observed::Unexpected(format!("{}: {}", output.status, String::from_utf8_lossy(&output.stderr)));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();
    for line in stdout.lines().filter(|line| !line.is_empty()) {
        match line.strip_prefix("spec='").and_then(|rest| rest.strip_suffix('\'')) {
            Some(value) => files.push(fixture.completion_path(value, cell.launch)),
            None => return Observed::Unexpected(format!("suggestion {line:?} is not a `spec` value")),
        }
    }
    if files.is_empty() { Observed::Unresolved } else { Observed::Files(files) }
}

/// `claudine compose --dry-run <prompt> spec=<candidate>` from the cell's
/// launch directory, for each candidate.
fn supplied_glob_value(cli: &CliProcessFixture, fixture: &ParityFixture, cell: &GlobValueCell) -> Observed {
    let prompt = fixture.glob_value_document(cell);
    let outcomes = fixture
        .glob_candidates()
        .into_iter()
        .map(|candidate| {
            let output = cli
                .command_builder()
                .ambient_context(&fixture.launch_dir(cell.launch))
                .build()
                .args(["compose", "--dry-run"])
                .arg(&prompt)
                .arg(format!("spec={}", to_portable_string(&candidate)))
                .output()
                .expect("run claudine compose --dry-run with a supplied value");
            (candidate, outcome(&output))
        })
        .collect();
    matrix::validation_verdicts(outcomes)
}

/// The two fixtures the claudine-cli rows run in.
struct Fixtures {
    /// Plain targets: composition and supplied values.
    plain: (CliProcessFixture, ParityFixture),
    /// Targets that declare a `resolved` schema: completion and the prompt
    /// argument it round-trips through.
    declared: (CliProcessFixture, ParityFixture),
}

impl Fixtures {
    fn for_entry(&self, entry: EntryPoint) -> (&CliProcessFixture, &ParityFixture) {
        let (cli, fixture) = match entry {
            EntryPoint::ClaudineCompletion | EntryPoint::ClaudinePromptArgument => &self.declared,
            EntryPoint::ClaudineComposition
            | EntryPoint::ClaudineSuppliedValue
            | EntryPoint::ComposePipeline
            | EntryPoint::Preflight
            | EntryPoint::SchemaValidation
            | EntryPoint::MdCompose
            | EntryPoint::MdSchemaValidate
            | EntryPoint::MdArgument(_)
            | EntryPoint::DmlsDiagnostics
            | EntryPoint::DmlsDocumentLinks
            | EntryPoint::DmlsLinkGraph
            | EntryPoint::DmlsDefinition
            | EntryPoint::DmlsCodeActions
            | EntryPoint::ClaudineChooser => &self.plain,
        };
        (cli, fixture)
    }
}

fn run(fixtures: &Fixtures, row: &Row) -> (Expected, Observed) {
    let (cli, fixture) = fixtures.for_entry(row.entry());
    match row {
        Row::GlobDocument(cell) => {
            assert_eq!(cell.entry, EntryPoint::ClaudineComposition, "{row:?}");
            (fixture.expected_glob_document(cell), compose_glob(cli, fixture, cell))
        }
        Row::GlobValue(cell) => {
            let observed = match cell.entry {
                EntryPoint::ClaudineCompletion => complete_glob(cli, fixture, cell),
                EntryPoint::ClaudineSuppliedValue => supplied_glob_value(cli, fixture, cell),
                other => unreachable!("{other:?} runs no claudine-cli glob value row"),
            };
            (fixture.expected_glob_value(cell), observed)
        }
        Row::Document(cell) => {
            let observed = match cell.entry {
                EntryPoint::ClaudineComposition => compose(cli, fixture, cell),
                EntryPoint::ClaudineCompletion
                | EntryPoint::ClaudinePromptArgument
                | EntryPoint::ClaudineSuppliedValue
                | EntryPoint::ComposePipeline
                | EntryPoint::Preflight
                | EntryPoint::SchemaValidation
                | EntryPoint::MdCompose
                | EntryPoint::MdSchemaValidate
                | EntryPoint::MdArgument(_)
                | EntryPoint::DmlsDiagnostics
                | EntryPoint::DmlsDocumentLinks
                | EntryPoint::DmlsLinkGraph
                | EntryPoint::DmlsDefinition
                | EntryPoint::DmlsCodeActions
                | EntryPoint::ClaudineChooser => unreachable!("{row:?} is not a claudine-cli document row"),
            };
            (fixture.expected_document(cell), observed)
        }
        Row::Value(cell) => {
            let observed = match cell.entry {
                EntryPoint::ClaudineCompletion => completion(cli, fixture, cell),
                EntryPoint::ClaudinePromptArgument => prompt_argument(cli, fixture, cell),
                EntryPoint::ClaudineSuppliedValue => supplied_value(cli, fixture, cell),
                EntryPoint::ClaudineComposition
                | EntryPoint::ComposePipeline
                | EntryPoint::Preflight
                | EntryPoint::SchemaValidation
                | EntryPoint::MdCompose
                | EntryPoint::MdSchemaValidate
                | EntryPoint::MdArgument(_)
                | EntryPoint::DmlsDiagnostics
                | EntryPoint::DmlsDocumentLinks
                | EntryPoint::DmlsLinkGraph
                | EntryPoint::DmlsDefinition
                | EntryPoint::DmlsCodeActions
                | EntryPoint::ClaudineChooser => unreachable!("{row:?} is not a claudine-cli value row"),
            };
            (fixture.expected_value(cell), observed)
        }
    }
}

fn fixtures() -> Fixtures {
    let plain_cli = CliProcessFixture::named("entry_point_parity");
    let plain = ParityFixture::create(plain_cli.workspace_path());
    std::fs::write(value_document(&plain), Consumer::SchemaFile.document("placeholder"))
        .expect("write the value document");
    let declared_cli = CliProcessFixture::named("entry_point_parity_completion");
    let declared = ParityFixture::create(declared_cli.workspace_path());
    declare_target_schemas(&declared);
    Fixtures { plain: (plain_cli, plain), declared: (declared_cli, declared) }
}

#[test]
fn claudine_entry_points_agree_on_every_reference() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../../darkmatter/lib/tests/common/entry_point_parity/mod.rs");

    let fixtures = fixtures();
    let rows = rows_for(Owner::ClaudineCli);

    // Documents are written before the spawns run concurrently, so two
    // threads never write one file.
    for row in &rows {
        let (_, fixture) = fixtures.for_entry(row.entry());
        match row {
            Row::Document(cell) => {
                fixture.write_document(cell);
            }
            Row::Value(cell) => {
                fixture.value(cell);
            }
            Row::GlobDocument(cell) => match cell.consumer {
                GlobConsumer::MatchValidation => {
                    fixture.write_glob_match_documents(cell);
                }
                _ => {
                    fixture.write_glob_document(cell);
                }
            },
            Row::GlobValue(cell) => {
                fixture.write_glob_value_document(cell);
            }
        }
    }
    const THREADS: usize = 8;
    let chunk = rows.len().div_ceil(THREADS);
    let results: Vec<(Row, Expected, Observed)> = std::thread::scope(|scope| {
        let handles: Vec<_> = rows
            .chunks(chunk)
            .map(|rows| {
                let fixtures = &fixtures;
                scope.spawn(move || {
                    rows.iter()
                        .map(|row| {
                            let (expected, observed) = run(fixtures, row);
                            (*row, expected, observed)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().flat_map(|handle| handle.join().unwrap()).collect()
    });

    let mut report = ParityReport::new(Owner::ClaudineCli);
    for (row, expected, observed) in &results {
        let (_, fixture) = fixtures.for_entry(row.entry());
        report.record(fixture, row, expected, observed);
    }
    report.assert_parity();
}

/// `claudine <args>` launched from the cross-repository fixture's launch
/// repository.
fn claudine_from_launch(cli: &CliProcessFixture, fixture: &CrossRepositoryFixture, args: &[&std::ffi::OsStr]) -> Output {
    cli.command_builder()
        // The launch repository decides the `@` scope, which is the subject.
        .ambient_context(&fixture.launch())
        .build()
        .args(args)
        .output()
        .expect("run claudine")
}

/// Completion lines for `claudine __complete … <command> <prompt> <partial>`.
fn complete_from_launch(
    cli: &CliProcessFixture,
    fixture: &CrossRepositoryFixture,
    command: &str,
    prompt: &Path,
    partial: &str,
) -> Vec<String> {
    let args: Vec<&std::ffi::OsStr> = vec![
        "__complete".as_ref(),
        "--current".as_ref(),
        "3".as_ref(),
        "--".as_ref(),
        "claudine".as_ref(),
        command.as_ref(),
        prompt.as_os_str(),
        partial.as_ref(),
    ];
    let output = claudine_from_launch(cli, fixture, &args);
    assert!(output.status.success(), "__complete failed: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// Setter completion for a prompt in another repository reads its `$schema`
/// the way composition does: `&`, `^`, and bare root lookups (and the `./`
/// and absolute controls) find the source repository's schema, `@` the
/// launch repository's, in every composition command, for values and for
/// names in authored order.
#[test]
fn external_prompt_schema_completion_keeps_both_anchors() {
    let _ = include_str!("../../../../darkmatter/lib/tests/common/entry_point_parity/mod.rs");
    let cli = CliProcessFixture::named("cross_repository_completion");
    let fixture = CrossRepositoryFixture::create(cli.workspace_path());
    let prompts: Vec<_> = fixture
        .schema_references()
        .into_iter()
        .enumerate()
        .map(|(index, (label, reference, owner))| {
            (label, fixture.write_schema_document(&format!("prompt-{index}"), &reference, ""), owner)
        })
        .collect();

    let mut failures = Vec::new();
    for command in ["compose", "inline-compose", "sequence"] {
        for (label, prompt, owner) in &prompts {
            let mut values = complete_from_launch(&cli, &fixture, command, prompt, "zebra=");
            values.sort();
            let mut expected: Vec<String> =
                owner.zebra_values().iter().map(|value| format!("zebra='{value}'")).collect();
            expected.sort();
            if values != expected {
                failures.push(format!("{command} {label}: values {values:?}, expected {expected:?}"));
            }
            let names = complete_from_launch(&cli, &fixture, command, prompt, "a");
            if names != ["zebra=", "apple="] {
                failures.push(format!("{command} {label}: names {names:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "external prompt completion:\n{}", failures.join("\n"));
}

/// Claudine composition of a prompt in another repository, the behavior
/// completion must match: `@magic.md` reads the launch repository's file, and
/// every `$schema` spelling accepts a value only its expected owner declares.
#[test]
fn external_prompt_composition_keeps_both_anchors() {
    let _ = include_str!("../../../../darkmatter/lib/tests/common/entry_point_parity/mod.rs");
    let cli = CliProcessFixture::named("cross_repository_composition");
    let fixture = CrossRepositoryFixture::create(cli.workspace_path());

    let document = fixture.write_magic_document();
    let output = claudine_from_launch(&cli, &fixture, &["compose".as_ref(), "--dry-run".as_ref(), document.as_os_str()]);
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    assert!(
        output.status.success() && stdout.contains(LAUNCH_MAGIC) && !stdout.contains(SOURCE_MAGIC),
        "claudine compose must read the launch `@magic.md`; status {}, stdout:\n{stdout}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );

    let mut failures = Vec::new();
    for (index, (label, reference, owner)) in fixture.schema_references().into_iter().enumerate() {
        let prompt = fixture.write_schema_document(
            &format!("composed-{index}"),
            &reference,
            &format!("zebra: {}\n", owner.zebra_values()[0]),
        );
        let output = claudine_from_launch(&cli, &fixture, &["compose".as_ref(), "--dry-run".as_ref(), prompt.as_os_str()]);
        if !output.status.success() {
            failures.push(format!(
                "{label} `{reference}` ({owner:?}): {}",
                strip_ansi(&String::from_utf8_lossy(&output.stderr))
            ));
        }
    }
    assert!(failures.is_empty(), "external prompt composition:\n{}", failures.join("\n"));
}
