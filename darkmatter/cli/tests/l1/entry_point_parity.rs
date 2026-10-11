//! The entry-point parity matrix's darkmatter-cli runner: `md compose` and
//! `md schema validate` over Table 1's documents, launched from the fixture
//! repository root, and `md compose <value>` over Table 2's caller-supplied
//! values from the repository root and from the package.
//!
//! Every spawn goes through `CliProcessFixture`, whose `home/` is the
//! child's `HOME` (`USERPROFILE` on Windows) and the fixture `HOME` of the
//! matrix; `md`'s process snapshot reads its home from that environment on
//! every OS. Every run passes the matrix's configured extra `@` root as
//! `--magic-root`, so `@configured-doc.md` resolves through the flag. A
//! failure's class is read only from the stable `failure: <name>` row `md`
//! renders for a failed file reference, never from message text.
//!
//! Glob rows: `md compose` lists `::file-links` and `find_files()` and, with
//! `md schema validate`, validates `match()` for a frontmatter value; the
//! document's `--set` value is the library runner's Table 2 row.

#[path = "../../../lib/tests/common/entry_point_parity/mod.rs"]
mod matrix;

use std::path::{Path, PathBuf};
use std::process::Output;

use biscuit_file::{ResolutionFailure, to_portable_string};
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use matrix::{
    Consumer, CrossRepositoryFixture, DocumentCell, EntryPoint, Expected, GlobConsumer, GlobDocumentCell, LAUNCH_MAGIC,
    MdRoute, Observed, Owner, ParityFixture, ParityReport, Row, SOURCE_MAGIC, ValueCell, rows_for,
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
        .arg("--magic-root")
        .arg(fixture.configured_magic_root())
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
        .arg("--magic-root")
        .arg(fixture.configured_magic_root())
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

/// `md --magic-root <root> <args…> <document>` from the repository root.
fn md_on(cli: &CliProcessFixture, fixture: &ParityFixture, args: &[&str], document: &Path) -> Output {
    cli.command_builder()
        .ambient_context(&fixture.repo())
        .build()
        .arg("--magic-root")
        .arg(fixture.configured_magic_root())
        .args(args)
        .arg(document_argument(fixture, document))
        .output()
        .expect("run md")
}

/// `md compose` of a `::file-links` or `find_files()` cell: the files it
/// lists, or the class on its failure row.
fn md_glob_listing(cli: &CliProcessFixture, fixture: &ParityFixture, cell: &GlobDocumentCell) -> Observed {
    let output = md_on(cli, fixture, &["compose"], &fixture.glob_document(cell));
    let stdout = strip_escape_codes(String::from_utf8_lossy(&output.stdout).as_ref());
    let stderr = strip_escape_codes(String::from_utf8_lossy(&output.stderr).as_ref());
    if let Some(failure) = failure_row(&stderr).or_else(|| failure_row(&stdout)) {
        return Observed::Failure(failure);
    }
    if !output.status.success() {
        return Observed::Unexpected(format!("{} without a failure row: {stderr}", output.status));
    }
    match cell.consumer {
        GlobConsumer::FileLinks => match fixture.file_links_listed(&stdout) {
            files if files.is_empty() => Observed::Unexpected(format!("no `::file-links` tree in {stdout:?}")),
            files => Observed::FileSet(files),
        },
        GlobConsumer::FindFiles => fixture
            .find_files_listed(&stdout)
            .map_or_else(|| Observed::Unexpected(format!("no `find_files()` line in {stdout:?}")), Observed::Files),
        GlobConsumer::MatchValidation | GlobConsumer::MatchCompletion => unreachable!("{cell:?} lists no files"),
    }
}

/// `md compose` or `md schema validate` of each candidate's `match()`
/// document.
fn md_glob_match(cli: &CliProcessFixture, fixture: &ParityFixture, cell: &GlobDocumentCell) -> Observed {
    let args: &[&str] = match cell.entry {
        EntryPoint::MdCompose => &["compose"],
        EntryPoint::MdSchemaValidate => &["schema", "validate"],
        other => unreachable!("{other:?} validates no md glob document"),
    };
    let outcomes: Vec<(PathBuf, Result<(), String>)> = fixture
        .glob_match_documents(cell)
        .into_iter()
        .map(|(candidate, document)| {
            let output = md_on(cli, fixture, args, &document);
            let outcome = if output.status.success() {
                Ok(())
            } else {
                Err(strip_escape_codes(
                    format!("{}{}", String::from_utf8_lossy(&output.stderr), String::from_utf8_lossy(&output.stdout))
                        .as_str(),
                ))
            };
            (candidate, outcome)
        })
        .collect();
    matrix::validation_verdicts(outcomes)
}

/// The document a delta route compares the value against: no target
/// heading, so only the value's heading names a target.
fn delta_control(fixture: &ParityFixture) -> std::path::PathBuf {
    fixture.root().join("delta-control.md")
}

/// `md <route> <value>`, the value passed as one argument (no shell), so a
/// `~/…` value reaches `md` as a quoted `'~/…'` would. Each route is observed
/// through its own result: the heading it renders, the title it reads, the
/// path it reports, the hash it prints.
fn md_argument(cli: &CliProcessFixture, fixture: &ParityFixture, route: MdRoute, cell: &ValueCell) -> Observed {
    let value = fixture.value(cell);
    let control = delta_control(fixture);
    let control = control.to_string_lossy().into_owned();
    let args: Vec<&str> = match route {
        MdRoute::Render => vec!["render", &value],
        MdRoute::Compose => vec!["compose", &value],
        MdRoute::Clean => vec!["clean", &value],
        MdRoute::Toc => vec!["toc", "--json", &value],
        MdRoute::FrontmatterGet => vec!["get", &value, "title"],
        MdRoute::FrontmatterSet => vec!["set", &value, "routed", "yes"],
        MdRoute::FrontmatterRm => vec!["rm", &value, "title", "--json"],
        MdRoute::Hash => vec!["hash", &value],
        MdRoute::DeltaBase => vec!["delta", &value, &control, "--json"],
        MdRoute::DeltaUpdated => vec!["delta", &control, &value, "--json"],
        MdRoute::Graph => vec!["graph", "--json", &value],
        MdRoute::Edit => vec!["edit", &value],
        MdRoute::ValidateRefs => vec!["validate", "refs", "--graph", "mermaid", &value],
        MdRoute::SchemaValidate => vec!["schema", "validate", &value],
        MdRoute::SchemaDetect => vec!["schema", "detect", &value],
        MdRoute::SchemaTriggers => vec!["schema", "triggers", &value],
        MdRoute::CodeBlockFile => vec!["code-block", "--file", &value],
        MdRoute::CodeBlockDefault => vec!["code-block", &value],
    };
    let mut command = cli.command_builder().ambient_context(&fixture.launch_dir(cell.launch)).build();
    command.arg("--magic-root").arg(fixture.configured_magic_root()).args(&args);
    if route == MdRoute::Edit {
        command.env("EDITOR", NO_OP_EDITOR).env("VISUAL", NO_OP_EDITOR);
    }
    let output = command.output().expect("run md");
    let stdout = strip_escape_codes(String::from_utf8_lossy(&output.stdout).as_ref());
    let stderr = strip_escape_codes(String::from_utf8_lossy(&output.stderr).as_ref());
    if let Some(failure) = failure_row(&stderr).or_else(|| failure_row(&stdout)) {
        return Observed::Failure(failure);
    }
    if !output.status.success() && route != MdRoute::Edit {
        return Observed::Unexpected(format!("{} without a failure row: {stderr}", output.status));
    }
    let one = |files: Vec<std::path::PathBuf>| match files.as_slice() {
        [path] => Observed::File(path.clone()),
        _ => Observed::Unexpected(format!("{} files named in stdout {stdout:?}", files.len())),
    };
    let json = || serde_json::from_str::<serde_json::Value>(&stdout).unwrap_or_default();
    match route {
        MdRoute::Render | MdRoute::Compose | MdRoute::Clean | MdRoute::CodeBlockFile => {
            one(fixture.marked_targets(&stdout))
        }
        MdRoute::DeltaBase | MdRoute::DeltaUpdated => one(fixture.marked_targets(&stdout)),
        MdRoute::CodeBlockDefault => match fixture.marked_targets(&stdout).as_slice() {
            [] if stdout.contains(value.as_str()) => Observed::Literal,
            _ => one(fixture.marked_targets(&stdout)),
        },
        MdRoute::Toc => match json()["title"].as_str() {
            Some(id) => Observed::File(fixture.root().join(id)),
            None => Observed::Unexpected(format!("no title in {stdout:?}")),
        },
        MdRoute::FrontmatterGet => Observed::File(fixture.root().join(stdout.trim().trim_matches('"'))),
        MdRoute::FrontmatterSet if !stdout.contains("routed: yes") => {
            Observed::Unexpected(format!("the set property is missing from {stdout:?}"))
        }
        MdRoute::FrontmatterSet => one(fixture.marked_targets(&stdout)),
        MdRoute::FrontmatterRm => match (json()["filename"].as_str(), json()["removed"].clone()) {
            (Some(path), removed) if removed == serde_json::json!(["title"]) => {
                Observed::File(std::path::PathBuf::from(path))
            }
            _ => Observed::Unexpected(format!("rm did not remove `title`: {stdout:?}")),
        },
        MdRoute::Hash => one(
            fixture
                .route_files()
                .into_iter()
                .filter(|path| simple_hash(path).as_deref() == Some(stdout.trim()))
                .collect(),
        ),
        MdRoute::Graph => match json()["source"].as_str() {
            Some(path) => Observed::File(std::path::PathBuf::from(path)),
            None => Observed::Unexpected(format!("no source in {stdout:?}")),
        },
        MdRoute::ValidateRefs => one(mermaid_nodes(fixture, &stdout)),
        MdRoute::SchemaValidate => match stdout.split_once("(file://") {
            Some((_, rest)) => Observed::File(std::path::PathBuf::from(rest.split(')').next().unwrap_or_default())),
            None => Observed::Unexpected(format!("no document link in {stdout:?}")),
        },
        MdRoute::SchemaDetect => one(fixture.route_file_by_slug(&stdout)),
        MdRoute::Edit => edit_result(&stdout, &stderr),
        MdRoute::SchemaTriggers => match stdout.lines().find_map(|line| line.trim().strip_prefix("Document:")) {
            Some(path) => Observed::File(std::path::PathBuf::from(path.trim())),
            None => Observed::Unexpected(format!("no document line in {stdout:?}")),
        },
    }
}

/// An editor that exits at once without writing.
#[cfg(not(windows))]
const NO_OP_EDITOR: &str = "/usr/bin/true";
#[cfg(windows)]
const NO_OP_EDITOR: &str = "cmd /c rem";

/// `md edit` with [`NO_OP_EDITOR`] prints the file it opened; a file it had
/// to create is left empty, so `md` reports it as empty after editing and the
/// runner removes it again.
fn edit_result(stdout: &str, stderr: &str) -> Observed {
    if let Some((_, rest)) = stderr.split_once("File is empty after editing:") {
        let created = std::path::PathBuf::from(rest.trim());
        let _ = std::fs::remove_file(&created);
        return Observed::File(created);
    }
    match stdout.lines().map(str::trim).filter(|line| !line.is_empty()).collect::<Vec<_>>().as_slice() {
        [path] => Observed::File(std::path::PathBuf::from(path)),
        _ => Observed::Unexpected(format!("md edit: stdout {stdout:?}, stderr {stderr:?}")),
    }
}

/// The hash `md hash <file>` prints for `path` in a scrubbed environment.
fn simple_hash(path: &Path) -> Option<String> {
    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::hash::{MdHashKind, MdHashOptions};
    let markdown = Markdown::try_from(path).ok()?;
    markdown.compute_hash(MdHashKind::Simple, &MdHashOptions::default()).flat_string()
}

/// The route files a Mermaid reference graph's nodes name: a node's id is
/// its path with every other character than ASCII letters and digits as
/// `_`, and its label starts with the file name.
fn mermaid_nodes(fixture: &ParityFixture, stdout: &str) -> Vec<std::path::PathBuf> {
    let mangle = |path: &Path| -> String {
        path.to_string_lossy().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
    };
    let nodes: Vec<(&str, &str)> = stdout
        .lines()
        .filter_map(|line| line.trim().split_once("[\""))
        .collect();
    fixture
        .route_files()
        .into_iter()
        .filter(|path| {
            let canonical = biscuit_file::canonicalize_simplified(path).unwrap_or_else(|_| path.clone());
            let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            nodes.iter().any(|(id, label)| {
                (*id == mangle(path) || *id == mangle(&canonical)) && label.starts_with(&format!("{name}<br/>"))
            })
        })
        .collect()
}

fn run(cli: &CliProcessFixture, fixture: &ParityFixture, row: &Row) -> (Expected, Observed) {
    match row {
        Row::GlobDocument(cell) => {
            let observed = match cell.consumer {
                GlobConsumer::FileLinks | GlobConsumer::FindFiles => md_glob_listing(cli, fixture, cell),
                GlobConsumer::MatchValidation => md_glob_match(cli, fixture, cell),
                GlobConsumer::MatchCompletion => unreachable!("md completes nothing: {row:?}"),
            };
            (fixture.expected_glob_document(cell), observed)
        }
        Row::GlobValue(_) => unreachable!("md runs no Table 2 glob row: {row:?}"),
        Row::Document(cell) => {
            let observed = match cell.entry {
                EntryPoint::MdCompose => md_compose(cli, fixture, cell),
                EntryPoint::MdSchemaValidate => md_schema_validate(cli, fixture, cell),
                EntryPoint::MdArgument(_)
                | EntryPoint::ComposePipeline
                | EntryPoint::Preflight
                | EntryPoint::SchemaValidation
                | EntryPoint::DmlsDiagnostics
                | EntryPoint::DmlsDocumentLinks
                | EntryPoint::DmlsLinkGraph
                | EntryPoint::DmlsDefinition
                | EntryPoint::DmlsCodeActions
                | EntryPoint::ClaudineComposition
                | EntryPoint::ClaudineCompletion
                | EntryPoint::ClaudinePromptArgument
                | EntryPoint::ClaudineSuppliedValue
                | EntryPoint::ClaudineChooser => unreachable!("{row:?} is not a darkmatter-cli document row"),
            };
            (fixture.expected_document(cell), observed)
        }
        Row::Value(cell) => {
            let observed = match cell.entry {
                EntryPoint::MdArgument(route) => md_argument(cli, fixture, route, cell),
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
                | EntryPoint::ClaudineCompletion
                | EntryPoint::ClaudinePromptArgument
                | EntryPoint::ClaudineSuppliedValue
                | EntryPoint::ClaudineChooser => unreachable!("{row:?} is not a darkmatter-cli value row"),
            };
            (fixture.expected_value(cell), observed)
        }
    }
}

/// One `md` spawn costs far more than a library call, so the matrix is split
/// across tests nextest runs in parallel. A `match()` validation row spawns
/// `md` once per candidate, so those rows get a part of their own; the light
/// rows share the rest.
const HEAVY_PART: usize = 0;
const LIGHT_PARTS: usize = 21;

fn is_heavy(row: &Row) -> bool {
    matches!(row, Row::GlobDocument(GlobDocumentCell { consumer: GlobConsumer::MatchValidation, .. }))
}

/// Every owned entry point has a row, so the parts together run each one.
#[test]
fn every_md_entry_point_runs_a_matrix_row() {
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    let rows = rows_for(Owner::DarkmatterCli);
    for entry in EntryPoint::ALL.into_iter().filter(|entry| entry.owner() == Owner::DarkmatterCli) {
        assert!(rows.iter().any(|row| row.entry() == entry), "{entry:?} has no darkmatter-cli matrix row");
    }
}

/// Runs one part of the matrix; the union of all parts is every row. Each
/// part has its own fixture, so a mutating route cannot disturb another
/// part's reads.
fn run_matrix_part(part: usize) {
    let cli = CliProcessFixture::named(&format!("entry_point_parity_{part:02}"));
    let fixture = ParityFixture::create(cli.workspace_path());
    fixture.write_route_files();
    std::fs::write(delta_control(&fixture), "# CONTROL\n").unwrap();
    let mut light = 0;
    let mine: Vec<Row> = rows_for(Owner::DarkmatterCli)
        .into_iter()
        .filter(|row| {
            let slot = if is_heavy(row) {
                HEAVY_PART
            } else {
                light += 1;
                HEAVY_PART + 1 + (light - 1) % LIGHT_PARTS
            };
            slot == part
        })
        .collect();
    // A mutating route's cells run after the others, each on a restored
    // fixture, so no cell reads a file another is changing.
    let mutates = |row: &Row| matches!(row.entry(), EntryPoint::MdArgument(route) if route.mutates());
    let (serial, rows): (Vec<Row>, Vec<Row>) = mine.into_iter().partition(mutates);

    // Every document is written before the spawns, as a cell's glob listing
    // sees the whole directory.
    for row in &rows {
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
            Row::GlobValue(_) => {}
        }
    }
    let mut results: Vec<(Row, Expected, Observed)> = rows
        .iter()
        .map(|row| {
            let (expected, observed) = run(&cli, &fixture, row);
            (*row, expected, observed)
        })
        .collect();
    for row in &serial {
        fixture.write_route_files();
        let (expected, observed) = run(&cli, &fixture, row);
        results.push((*row, expected, observed));
    }

    let mut report = ParityReport::new(Owner::DarkmatterCli);
    for (row, expected, observed) in &results {
        report.record(&fixture, row, expected, observed);
    }
    report.assert_cells_agree();
}

#[test]
fn md_entry_points_agree_on_every_reference_part_00() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(0);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_01() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(1);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_02() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(2);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_03() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(3);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_04() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(4);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_05() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(5);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_06() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(6);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_07() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(7);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_08() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(8);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_09() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(9);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_10() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(10);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_11() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(11);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_12() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(12);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_13() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(13);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_14() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(14);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_15() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(15);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_16() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(16);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_17() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(17);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_18() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(18);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_19() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(19);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_20() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(20);
}

#[test]
fn md_entry_points_agree_on_every_reference_part_21() {
    // The coupling CI's test-input index reads (see the module docs of the
    // shared file); the `#[path]` include alone is not counted.
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    run_matrix_part(21);
}


/// `md <args>` launched from the cross-repository fixture's launch
/// repository.
fn md_from_launch(cli: &CliProcessFixture, fixture: &CrossRepositoryFixture, args: &[&std::ffi::OsStr]) -> Output {
    cli.command_builder()
        // The launch repository decides the `@` scope, which is the subject.
        .ambient_context(&fixture.launch())
        .build()
        .args(args)
        .output()
        .expect("run md")
}

fn stdout_of(output: &Output) -> String {
    strip_escape_codes(String::from_utf8_lossy(&output.stdout).as_ref())
}

/// A document in another repository keeps the launch `@` scope: its
/// `::file @magic.md` composes and graphs the launch repository's file, not
/// the one at its own repository root.
#[test]
fn external_document_magic_reference_keeps_the_launch_scope() {
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    let cli = CliProcessFixture::named("cross_repository_magic");
    let fixture = CrossRepositoryFixture::create(cli.workspace_path());
    let document = fixture.write_magic_document();

    let composed = md_from_launch(&cli, &fixture, &["compose".as_ref(), document.as_os_str()]);
    let stdout = stdout_of(&composed);
    assert!(
        composed.status.success() && stdout.contains(LAUNCH_MAGIC) && !stdout.contains(SOURCE_MAGIC),
        "md compose must read the launch `@magic.md`; status {}, stdout:\n{stdout}\nstderr:\n{}",
        composed.status,
        String::from_utf8_lossy(&composed.stderr),
    );

    let graphed = md_from_launch(&cli, &fixture, &["graph".as_ref(), "--json".as_ref(), document.as_os_str()]);
    assert!(graphed.status.success(), "md graph: {}", String::from_utf8_lossy(&graphed.stderr));
    let graph: serde_json::Value = serde_json::from_slice(&graphed.stdout).expect("md graph --json is JSON");
    let targets: Vec<_> = graph["transclusions"]
        .as_array()
        .expect("transclusions array")
        .iter()
        .map(|edge| matrix::identity(Path::new(edge["target"].as_str().expect("target path"))))
        .collect();
    assert_eq!(targets, [matrix::identity(&fixture.launch().join("magic.md"))], "{graph:#}");
}

/// The schema readers of `md` find `$schema: '@order.yaml'` in the launch
/// repository for a document in another repository, so `zebra: launch`
/// (valid only against the launch schema) is accepted by validation, by
/// compose's schema stage, and by `clean`'s schema reader.
#[test]
fn external_document_magic_schema_keeps_the_launch_scope() {
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    let cli = CliProcessFixture::named("cross_repository_magic_schema");
    let fixture = CrossRepositoryFixture::create(cli.workspace_path());
    let document = fixture.write_launch_enum_document();

    for args in [&["schema", "validate"][..], &["compose"][..]] {
        let mut argv: Vec<&std::ffi::OsStr> = args.iter().map(|arg| arg.as_ref()).collect();
        argv.push(document.as_os_str());
        let output = md_from_launch(&cli, &fixture, &argv);
        assert!(
            output.status.success(),
            "md {args:?} must accept `zebra: launch`; status {}, stdout:\n{}\nstderr:\n{}",
            output.status,
            stdout_of(&output),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    let cleaned = md_from_launch(&cli, &fixture, &["clean".as_ref(), "--json".as_ref(), document.as_os_str()]);
    assert!(cleaned.status.success(), "md clean: {}", String::from_utf8_lossy(&cleaned.stderr));
    let report: serde_json::Value = serde_json::from_slice(&cleaned.stdout).expect("md clean --json is JSON");
    assert_eq!(report["diagnostics"], serde_json::json!([]), "{report:#}");
}

/// The other half: the same document's `&`, `^`, bare, `./`, and absolute
/// `$schema` references find its own repository's schema, while `@` finds the
/// launch repository's.
#[test]
fn external_document_schema_references_use_their_own_anchors() {
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    let cli = CliProcessFixture::named("cross_repository_schema_anchors");
    let fixture = CrossRepositoryFixture::create(cli.workspace_path());
    let mut failures = Vec::new();
    for (index, (label, reference, owner)) in fixture.schema_references().into_iter().enumerate() {
        let document = fixture.write_schema_document(
            &format!("anchor-{index}"),
            &reference,
            &format!("zebra: {}\n", owner.zebra_values()[0]),
        );
        let output = md_from_launch(&cli, &fixture, &["schema".as_ref(), "validate".as_ref(), document.as_os_str()]);
        if !output.status.success() {
            failures.push(format!("{label} `{reference}` ({owner:?}): {}", stdout_of(&output)));
        }
    }
    assert!(failures.is_empty(), "schema anchors:\n{}", failures.join("\n"));
}

/// `MdRequest::document_context`, the context every `md` route opens a
/// document with, keeps both anchors for a document in another repository:
/// its repository is the source's, its `@` scope the launch's.
#[test]
fn document_context_keeps_the_source_repository_and_the_launch_scope() {
    use biscuit_file::FileReference;
    use darkmatter::markdown::compose::RequestSnapshot;
    use darkmatter_cli::request::MdRequest;

    let cli = CliProcessFixture::named("cross_repository_document_context");
    let fixture = CrossRepositoryFixture::create(cli.workspace_path());
    let document = fixture.write_magic_document();
    let request = MdRequest::new(RequestSnapshot::new(fixture.launch()).with_home(Some(cli.home().to_path_buf())));
    let opening = FileReference::new(&to_portable_string(&document)).unwrap();

    for opening in [None, Some(&opening)] {
        let context = request.document_context(opening, &document).expect("document context");
        assert_eq!(
            context.repository_root().map(matrix::identity),
            Some(matrix::identity(&fixture.source())),
            "repository (opening {opening:?})",
        );
        assert_eq!(
            context.launch_magic_scope().repository_root().map(matrix::identity),
            Some(matrix::identity(&fixture.launch())),
            "launch `@` scope (opening {opening:?})",
        );
    }
}

/// `md code-block --content` renders every value as typed, reference or not,
/// and reads no file; without a flag, multi-line input is code even when its
/// first line names a file.
#[test]
fn code_block_content_keeps_every_value_literal() {
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    let cli = CliProcessFixture::named("code_block_content");
    let fixture = ParityFixture::create(cli.workspace_path());
    fixture.write_route_files();
    let values = ["./sibling.md", "root-only.md", "&root-only.md", "^area-doc.md", "@magic-doc.md", "~/notes/beside.md", "./@", "@", "&", "^", "!legacy.md", "../../../outside.md"];
    let mut failures = Vec::new();
    let run = |args: &[&str]| {
        let output = cli
            .command_builder()
            .ambient_context(&fixture.package())
            .build()
            .args(args)
            .output()
            .expect("run md code-block");
        (output.status.success(), strip_escape_codes(String::from_utf8_lossy(&output.stdout).as_ref()))
    };
    for value in values {
        let (success, stdout) = run(&["code-block", "--content", value]);
        if !success || !stdout.contains(value) || !fixture.marked_targets(&stdout).is_empty() {
            failures.push(format!("--content {value:?}: {stdout:?}"));
        }
    }
    let multi_line = "&root-only.md\nsecond line";
    let (success, stdout) = run(&["code-block", multi_line]);
    if !success || !stdout.contains("second line") || !fixture.marked_targets(&stdout).is_empty() {
        failures.push(format!("multi-line default input: {stdout:?}"));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// `md hash <directory>` names its directory with the file-argument grammar:
/// `&area` is the repository's `area/`, a relative directory may not leave
/// the repository, and a malformed introducer is refused even when a
/// directory of that literal name exists.
#[test]
fn hash_directory_arguments_use_the_reference_grammar() {
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    let cli = CliProcessFixture::named("hash_directory_arguments");
    let fixture = ParityFixture::create(cli.workspace_path());
    std::fs::create_dir_all(fixture.package().join("@")).unwrap();
    std::fs::write(fixture.package().join("@/literal.md"), "# LITERAL\n").unwrap();
    std::fs::create_dir_all(fixture.root().join("beyond")).unwrap();
    std::fs::write(fixture.root().join("beyond/outside.md"), "# OUTSIDE\n").unwrap();
    let hash = |argument: &str| {
        cli.command_builder()
            .ambient_context(&fixture.package())
            .build()
            .args(["hash", argument])
            .output()
            .expect("run md hash")
    };

    let by_path = hash(&fixture.repo().join("area").to_string_lossy());
    let by_reference = hash("&area");
    assert!(by_path.status.success(), "{by_path:?}");
    assert_eq!(by_reference.stdout, by_path.stdout, "`&area` hashes the repository's area/: {by_reference:?}");

    for argument in ["@", "../../../beyond"] {
        let output = hash(argument);
        let stderr = strip_escape_codes(String::from_utf8_lossy(&output.stderr).as_ref());
        assert_eq!(
            failure_row(&stderr),
            Some(ResolutionFailure::InvalidReference),
            "md hash {argument:?}: {} stdout {:?} stderr {stderr}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
        );
    }
}

/// `md render` resolves page-relative assets beside the file its argument
/// opened: a `style.page.stylesheet` of `./route.css` in a document opened as
/// `&styled/doc.md` is the stylesheet beside that document.
#[test]
fn render_reads_page_assets_beside_the_opened_document() {
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    let cli = CliProcessFixture::named("render_page_assets");
    let fixture = ParityFixture::create(cli.workspace_path());
    let styled = fixture.repo().join("styled");
    std::fs::create_dir_all(&styled).unwrap();
    std::fs::write(styled.join("route.css"), ".route-marker { color: red; }\n").unwrap();
    std::fs::write(
        styled.join("doc.md"),
        "---\nstyle:\n  page:\n    stylesheet: ./route.css\n---\n\n# Styled\n",
    )
    .unwrap();
    let output = cli
        .command_builder()
        .ambient_context(&fixture.package())
        .build()
        .args(["render", "--output", "html", "&styled/doc.md"])
        .output()
        .expect("run md render");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains(".route-marker"),
        "{} stdout {stdout:?} stderr {:?}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );
}

/// The seven reference forms a detected document carries, by property name.
const DETECTION_FORMS: [&str; 7] = ["relative", "bare", "repo", "scoped", "magic", "home", "absolute"];

/// `source/docs/doc.md` whose frontmatter holds every [`DETECTION_FORMS`]
/// value, each naming an existing file: `./beside.md` beside it, `target.md`
/// at the source root (bare, `&`, `^`), `@magic.md` (both repositories have
/// one), `~/home.md` in the fixture home, and the target's absolute path.
fn write_detection_document(cli: &CliProcessFixture, fixture: &CrossRepositoryFixture) -> std::path::PathBuf {
    let source = fixture.source();
    let docs = source.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join("beside.md"), "# Beside\n").unwrap();
    std::fs::write(source.join("target.md"), "# Target\n").unwrap();
    std::fs::write(cli.home().join("home.md"), "# Home\n").unwrap();
    let absolute = to_portable_string(&source.join("target.md"));
    let document = docs.join("doc.md");
    std::fs::write(
        &document,
        format!(
            "---\nrelative: './beside.md'\nbare: 'target.md'\nrepo: '&target.md'\nscoped: '^target.md'\n\
             magic: '@magic.md'\nhome: '~/home.md'\nabsolute: '{absolute}'\n---\n\n# Detect\n"
        ),
    )
    .unwrap();
    document
}

/// The `name: type` lines of every schema `md schema detect` printed, one map
/// per emitted schema (one per document without `--merge`).
fn detected_types(stdout: &str) -> Vec<std::collections::BTreeMap<String, String>> {
    let mut schemas = Vec::new();
    for line in stdout.lines() {
        if line.starts_with("$schema:") {
            schemas.push(std::collections::BTreeMap::new());
        } else if let (Some(schema), Some((name, ty))) = (schemas.last_mut(), line.strip_prefix("  ").and_then(|l| l.split_once(": "))) {
            schema.insert(name.trim().to_string(), ty.trim().to_string());
        }
    }
    schemas
}

/// Runs `md schema detect` from `launch` over `documents` in every mode
/// (the first alone, all without `--merge`, all with `--merge`) and
/// returns one line per property of `forms` whose inferred type is not
/// `file` (`file(required)` under `--merge`).
fn detection_misses(
    cli: &CliProcessFixture,
    launch: &Path,
    label: &str,
    documents: &[&std::ffi::OsStr],
    forms: &[&str],
) -> Vec<String> {
    let mut misses = Vec::new();
    let mut runs: Vec<(&str, Vec<&std::ffi::OsStr>, usize, &str)> = Vec::new();
    runs.push(("single", vec![documents[0]], 1, "file"));
    runs.push(("multiple", documents.to_vec(), documents.len(), "file"));
    let mut merged: Vec<&std::ffi::OsStr> = vec!["--merge".as_ref()];
    merged.extend_from_slice(documents);
    runs.push(("--merge", merged, 1, "file(required)"));
    for (mode, arguments, schemas_expected, expected) in runs {
        let output = cli
            .command_builder()
            .ambient_context(launch)
            .build()
            .args(["schema", "detect"])
            .args(&arguments)
            .output()
            .expect("run md schema detect");
        let stdout = stdout_of(&output);
        let schemas = detected_types(&stdout);
        if !output.status.success() || schemas.len() != schemas_expected {
            misses.push(format!(
                "{label} {mode}: status {}, {} schema(s), stdout:\n{stdout}\nstderr:\n{}",
                output.status,
                schemas.len(),
                String::from_utf8_lossy(&output.stderr)
            ));
            continue;
        }
        for (index, schema) in schemas.iter().enumerate() {
            for form in forms {
                let observed = schema.get(*form).map(String::as_str);
                if observed != Some(expected) {
                    misses.push(format!("{label} {mode} #{index}: {form} is {observed:?}, expected {expected}"));
                }
            }
        }
    }
    misses
}

/// `md schema detect` infers a document's references in the document's own
/// context: launched from another repository, every existing reference form
/// is `file` in each detection mode, exactly as it is when launched from the
/// document's repository (the positive control), whichever spelling of the
/// document's path names it.
#[test]
fn schema_detect_infers_references_in_the_document_context() {
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    let cli = CliProcessFixture::named("cross_repository_detect");
    let fixture = CrossRepositoryFixture::create(cli.workspace_path());
    let document = write_detection_document(&cli, &fixture);
    let canonical = biscuit_file::canonicalize_simplified(&document).expect("canonical document");
    // The workspace spelling and the canonical one differ where the temp
    // directory is a symlink (macOS `/var` for `/private/var`); a link to the
    // source repository gives every Unix host a second spelling.
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut spellings = vec![("workspace spelling", document.clone()), ("canonical spelling", canonical)];
    #[cfg(unix)]
    {
        let alias = cli.workspace_path().join("source-alias");
        std::os::unix::fs::symlink(fixture.source(), &alias).unwrap();
        spellings.push(("symlink alias", alias.join("docs/doc.md")));
    }

    let mut misses = Vec::new();
    for (launch_label, launch) in [("source launch", fixture.source()), ("foreign launch", fixture.launch())] {
        for (spelling, path) in &spellings {
            let label = format!("{launch_label}, {spelling}");
            misses.extend(detection_misses(&cli, &launch, &label, &[path.as_os_str(), path.as_os_str()], &DETECTION_FORMS));
        }
    }
    assert!(misses.is_empty(), "detected types:\n{}", misses.join("\n"));
}

/// A document outside every repository, opened from a launch repository by
/// absolute path or by a quoted `~/notes/doc.md`, infers its existing
/// relative, home, `@` (the launch scope), and absolute references as `file`;
/// opened through `~`, its home tree also admits `../parent.md`.
#[test]
fn schema_detect_keeps_a_repository_free_document_context() {
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    let cli = CliProcessFixture::named("repository_free_detect");
    let fixture = CrossRepositoryFixture::create(cli.workspace_path());
    let home = cli.home();
    let notes = home.join("notes");
    std::fs::create_dir_all(&notes).unwrap();
    std::fs::write(notes.join("beside.md"), "# Beside\n").unwrap();
    std::fs::write(home.join("parent.md"), "# Parent\n").unwrap();
    std::fs::write(home.join("home.md"), "# Home\n").unwrap();
    std::fs::write(home.join("absolute.md"), "# Absolute\n").unwrap();
    let absolute = to_portable_string(&home.join("absolute.md"));
    let document = notes.join("doc.md");
    std::fs::write(
        &document,
        format!(
            "---\nrelative: './beside.md'\nparent: '../parent.md'\nmagic: '@magic.md'\nhome: '~/home.md'\n\
             absolute: '{absolute}'\n---\n\n# Free\n"
        ),
    )
    .unwrap();

    let forms = ["relative", "magic", "home", "absolute"];
    let mut misses =
        detection_misses(&cli, &fixture.launch(), "absolute opening", &[document.as_os_str(); 2], &forms);
    let home_opening: &std::ffi::OsStr = "~/notes/doc.md".as_ref();
    misses.extend(detection_misses(
        &cli,
        &fixture.launch(),
        "home opening",
        &[home_opening; 2],
        &["relative", "parent", "magic", "home", "absolute"],
    ));
    assert!(misses.is_empty(), "detected types:\n{}", misses.join("\n"));
}

/// The library's detection surfaces are passive: given the context
/// `MdRequest::document_context` prepares for an external document, each
/// infers every reference form as `file`; given the launch context, whose
/// ordinary derivation cannot admit the document, each infers `string`
/// (the incompatibility `md schema detect` must not pass on).
#[test]
fn detection_surfaces_infer_through_the_context_they_are_given() {
    use biscuit_file::FileReference;
    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::compose::RequestSnapshot;
    use darkmatter::markdown::schemas::{
        DarkmatterSchemas, DetectOptions, SimplifiedSchema, detect_from_document, detect_schema,
        detect_schema_with_contexts, schema_to_yaml,
    };
    use darkmatter_cli::request::MdRequest;

    let cli = CliProcessFixture::named("detection_surfaces");
    let fixture = CrossRepositoryFixture::create(cli.workspace_path());
    let document = write_detection_document(&cli, &fixture);
    let request = MdRequest::new(RequestSnapshot::new(fixture.launch()).with_home(Some(cli.home().to_path_buf())));
    let opening = FileReference::new(&to_portable_string(&document)).unwrap();
    let document_context = request.document_context(Some(&opening), &document).expect("document context");
    let launch_context = request.launch_context().expect("launch context").clone();
    let markdown = Markdown::try_from(document.as_path()).expect("document parses");

    // `--merge` options, so every form must read `file(required)` or
    // `string(required)`, through the printed form `md` emits.
    let types = |schema: SimplifiedSchema| -> Vec<String> {
        let printed = detected_types(&schema_to_yaml(&schema));
        assert_eq!(printed.len(), 1, "one schema");
        DETECTION_FORMS.iter().map(|form| format!("{form}: {:?}", printed[0].get(*form))).collect()
    };
    let surfaces = |context: &biscuit_file::FileResolutionContext| -> Vec<(&'static str, Vec<String>)> {
        let options = DetectOptions { merge: true };
        let pair = [context.clone(), context.clone()];
        vec![
            ("detect_from_document", types(SimplifiedSchema::Single(detect_from_document(&markdown, context)))),
            ("detect_schema", types(detect_schema(&[&markdown, &markdown], options, context))),
            ("detect_schema_with_contexts", types(detect_schema_with_contexts(&[&markdown, &markdown], options, &pair))),
            ("DarkmatterSchemas::detect", types(DarkmatterSchemas::new(context.clone()).detect(&[&markdown, &markdown], options))),
        ]
    };

    for (surface, observed) in surfaces(&document_context) {
        for line in observed {
            assert!(line.ends_with("Some(\"file\")") || line.ends_with("Some(\"file(required)\")"), "{surface} with the document context: {line}");
        }
    }
    for (surface, observed) in surfaces(&launch_context) {
        for line in observed {
            assert!(line.ends_with("Some(\"string\")") || line.ends_with("Some(\"string(required)\")"), "{surface} with the launch context: {line}");
        }
    }
}
