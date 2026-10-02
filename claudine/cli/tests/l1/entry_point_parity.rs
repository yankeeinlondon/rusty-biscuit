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
    Consumer, DocumentCell, EntryPoint, Expected, Observed, Owner, ParityFixture, ParityReport, Row,
    ValueCell, rows_for,
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
            | EntryPoint::MdArgument
            | EntryPoint::DmlsDiagnostics
            | EntryPoint::DmlsDocumentLinks
            | EntryPoint::DmlsLinkGraph
            | EntryPoint::DmlsDefinition
            | EntryPoint::DmlsCodeActions => &self.plain,
        };
        (cli, fixture)
    }
}

fn run(fixtures: &Fixtures, row: &Row) -> (Expected, Observed) {
    let (cli, fixture) = fixtures.for_entry(row.entry());
    match row {
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
                | EntryPoint::MdArgument
                | EntryPoint::DmlsDiagnostics
                | EntryPoint::DmlsDocumentLinks
                | EntryPoint::DmlsLinkGraph
                | EntryPoint::DmlsDefinition
                | EntryPoint::DmlsCodeActions => unreachable!("{row:?} is not a claudine-cli document row"),
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
                | EntryPoint::MdArgument
                | EntryPoint::DmlsDiagnostics
                | EntryPoint::DmlsDocumentLinks
                | EntryPoint::DmlsLinkGraph
                | EntryPoint::DmlsDefinition
                | EntryPoint::DmlsCodeActions => unreachable!("{row:?} is not a claudine-cli value row"),
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
