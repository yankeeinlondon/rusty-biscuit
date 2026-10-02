//! The entry-point parity matrix's darkmatter-cli runner: `md compose` and
//! `md schema validate` over Table 1's documents, launched from the fixture
//! repository root, and `md compose <value>` over Table 2's caller-supplied
//! values from the repository root and from the package.
//!
//! Every spawn goes through `CliProcessFixture`, whose `home/` is the
//! child's `HOME` (`USERPROFILE` on Windows) and the fixture `HOME` of the
//! matrix; `md`'s process snapshot reads its home from that environment on
//! every OS. `md` configures no extra `@` root, so the matrix gives it no
//! configured-root cell. A failure's class is
//! read only from the stable `failure: <name>` row `md` renders for a failed
//! file reference, never from message text.

#[path = "../../../lib/tests/common/entry_point_parity/mod.rs"]
mod matrix;

use std::path::Path;
use std::process::Output;

use biscuit_file::{ResolutionFailure, to_portable_string};
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use matrix::{
    Consumer, DocumentCell, EntryPoint, Expected, Observed, Owner, ParityFixture, ParityReport, Row,
    ValueCell, rows_for,
};

use crate::common::CliProcessFixture;

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

/// The result of one `md` run. A run whose output names a failure reports
/// its class, whatever the exit status: a tolerated failure renders a notice
/// in the document and its warning on stderr.
fn observe(fixture: &ParityFixture, consumer: Consumer, tree_root: &Path, output: &Output) -> Observed {
    let stdout = strip_escape_codes(String::from_utf8_lossy(&output.stdout).as_ref());
    let stderr = strip_escape_codes(String::from_utf8_lossy(&output.stderr).as_ref());
    if let Some(failure) = failure_row(&stderr).or_else(|| failure_row(&stdout)) {
        return Observed::Failure(failure);
    }
    if !output.status.success() {
        return Observed::Unexpected(format!("{} without a failure row: {stderr}", output.status));
    }
    let targets = match consumer {
        Consumer::SchemaFile => stdout
            .lines()
            .filter_map(|line| line.trim().strip_prefix(matrix::STORED_VALUE_PREFIX))
            .map(|value| fixture.stored_value_path(value.trim(), tree_root))
            .collect(),
        _ => fixture.marked_targets(&stdout),
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

fn md_compose(cli: &CliProcessFixture, fixture: &ParityFixture, cell: &DocumentCell) -> Observed {
    let document = fixture.write_document(cell);
    let output = cli
        .command_builder()
        .ambient_context(&fixture.repo())
        .build()
        .arg("compose")
        .arg(document_argument(fixture, &document))
        .output()
        .expect("run md compose");
    observe(fixture, cell.consumer, &fixture.tree_root(cell), &output)
}

fn md_schema_validate(cli: &CliProcessFixture, fixture: &ParityFixture, cell: &DocumentCell) -> Observed {
    let document = fixture.write_document(cell);
    let output = cli
        .command_builder()
        .ambient_context(&fixture.repo())
        .build()
        .args(["schema", "validate"])
        .arg(document_argument(fixture, &document))
        .output()
        .expect("run md schema validate");
    match observe(fixture, cell.consumer, &fixture.tree_root(cell), &output) {
        // Validation is read-only: it accepts a value without naming its file.
        Observed::Unexpected(_) if output.status.success() => Observed::Accepted,
        observed => observed,
    }
}

/// `md compose <value>`, the value passed as one argument (no shell), so a
/// `~/…` value reaches `md` as a quoted `'~/…'` would.
fn md_argument(cli: &CliProcessFixture, fixture: &ParityFixture, cell: &ValueCell) -> Observed {
    let value = fixture.value(cell);
    let output = cli
        .command_builder()
        .ambient_context(&fixture.launch_dir(cell.launch))
        .build()
        .arg("compose")
        .arg(&value)
        .output()
        .expect("run md compose");
    observe(fixture, Consumer::File, &fixture.repo(), &output)
}

fn run(cli: &CliProcessFixture, fixture: &ParityFixture, row: &Row) -> (Expected, Observed) {
    match row {
        Row::Document(cell) => {
            let observed = match cell.entry {
                EntryPoint::MdCompose => md_compose(cli, fixture, cell),
                EntryPoint::MdSchemaValidate => md_schema_validate(cli, fixture, cell),
                EntryPoint::MdArgument
                | EntryPoint::ComposePipeline
                | EntryPoint::Preflight
                | EntryPoint::SchemaValidation
                | EntryPoint::DmlsDiagnostics
                | EntryPoint::DmlsDocumentLinks
                | EntryPoint::DmlsLinkGraph
                | EntryPoint::DmlsDefinition
                | EntryPoint::DmlsCodeActions
                | EntryPoint::ClaudineComposition
                | EntryPoint::ClaudineCompletion => unreachable!("{row:?} is not a darkmatter-cli document row"),
            };
            (fixture.expected_document(cell), observed)
        }
        Row::Value(cell) => {
            let observed = match cell.entry {
                EntryPoint::MdArgument => md_argument(cli, fixture, cell),
                EntryPoint::MdCompose
                | EntryPoint::MdSchemaValidate
                | EntryPoint::ComposePipeline
                | EntryPoint::Preflight
                | EntryPoint::SchemaValidation
                | EntryPoint::DmlsDiagnostics
                | EntryPoint::DmlsDocumentLinks
                | EntryPoint::DmlsLinkGraph
                | EntryPoint::DmlsDefinition
                | EntryPoint::DmlsCodeActions
                | EntryPoint::ClaudineComposition
                | EntryPoint::ClaudineCompletion => unreachable!("{row:?} is not a darkmatter-cli value row"),
            };
            (fixture.expected_value(cell), observed)
        }
    }
}

#[test]
fn md_entry_points_agree_on_every_reference() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");

    let cli = CliProcessFixture::named("entry_point_parity");
    let fixture = ParityFixture::create(cli.workspace_path());
    let rows = rows_for(Owner::DarkmatterCli);

    // Documents are written before the spawns run concurrently, so two
    // threads never write one file.
    for row in &rows {
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
                let (cli, fixture) = (&cli, &fixture);
                scope.spawn(move || {
                    rows.iter()
                        .map(|row| {
                            let (expected, observed) = run(cli, fixture, row);
                            (*row, expected, observed)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().flat_map(|handle| handle.join().unwrap()).collect()
    });

    let mut report = ParityReport::new(Owner::DarkmatterCli);
    for (row, expected, observed) in &results {
        report.record(&fixture, row, expected, observed);
    }
    report.assert_parity();
}
