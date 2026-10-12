//! The entry-point parity matrix's darkmatter runner: the compose pipeline,
//! pre-flight, and schema validation, each through its public API with a
//! request snapshot that carries the fixture `HOME`, the configured extra `@`
//! root, and no process state.
//!
//! Glob rows: the compose pipeline lists `::file-links` and `find_files()`
//! and validates `match()` for a frontmatter value (Table 1) and for a
//! caller-supplied `--set` value from each launch directory (Table 2);
//! pre-flight and schema validation validate the frontmatter value.

#[path = "../common/entry_point_parity/mod.rs"]
mod matrix;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use biscuit_file::PathPosition;
use darkmatter::markdown::compose::preflight::{PreflightGraphNode, PreflightResolvedTarget};
use darkmatter::markdown::compose::{
    ComposeOptions, ComposeReport, ComposeRequest, RequestSnapshot, build_resolution_context,
};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::{Markdown, MarkdownError};
use matrix::{
    ChainCase, ChainOutcome, Consumer, Depth, HASH_CONSUMERS, DocumentCell, EntryPoint, GlobConsumer, GlobDocumentCell,
    GlobValueCell, Launch, Observed, Owner, ParityFixture, ParityReport, Row, rows_for,
};

/// Table 1's launch directory is the repository root; the snapshot holds the
/// fixture `HOME`, the configured extra `@` root, and an empty environment.
fn snapshot(fixture: &ParityFixture) -> RequestSnapshot {
    RequestSnapshot::new(fixture.repo())
        .with_home(Some(fixture.home()))
        .with_magic_root(fixture.configured_magic_root(), PathPosition::Start)
}

/// A Table 2 snapshot: the same `HOME` and extra `@` root, launched from
/// `launch`.
fn launch_snapshot(fixture: &ParityFixture, launch: Launch) -> RequestSnapshot {
    RequestSnapshot::new(fixture.launch_dir(launch))
        .with_home(Some(fixture.home()))
        .with_magic_root(fixture.configured_magic_root(), PathPosition::Start)
}

/// The one target `targets` names, or what went wrong.
fn one_file(targets: Vec<std::path::PathBuf>, output: &str) -> Observed {
    match targets.as_slice() {
        [path] => Observed::File(path.clone()),
        _ => Observed::Unexpected(format!("{} targets in output {output:?}", targets.len())),
    }
}

/// A composed cell's result: the file its output names, or the class of
/// the failure composition tolerated (a notice in place of the content).
fn composed_target(fixture: &ParityFixture, cell: &DocumentCell, output: &str, report: &ComposeReport) -> Observed {
    if let Some(failure) = report.warnings.iter().find_map(|warning| warning.resolution_failure) {
        return Observed::Failure(failure);
    }
    match cell.consumer {
        Consumer::SchemaFile => one_file(fixture.stored_value_targets(output, &fixture.tree_root(cell)), output),
        _ => one_file(fixture.marked_targets(output), output),
    }
}

fn error_class(error: &MarkdownError) -> Observed {
    error
        .resolution_failure()
        .map_or_else(|| Observed::Unexpected(format!("unclassified error: {error}")), Observed::Failure)
}

fn request_for(fixture: &ParityFixture, document: &Path) -> ComposeRequest {
    ComposeRequest::prepare(ComposeOptions::new().with_source_file(document), &snapshot(fixture))
        .expect("prepare the request")
}

fn compose_pipeline(fixture: &ParityFixture, cell: &DocumentCell) -> Observed {
    let document = fixture.write_document(cell);
    let md = Markdown::try_from(document.as_path()).expect("read cell document");
    match md.compose_with(&request_for(fixture, &document)) {
        Ok((composed, report)) => composed_target(fixture, cell, composed.content(), &report),
        Err(error) => error_class(&error),
    }
}

/// The cell's own reference, below row (a)'s `~` hop: the deepest
/// document's first `::code`/`::toc-linking` target, else its first edge.
fn leaf_target(node: &PreflightGraphNode) -> Option<&PreflightResolvedTarget> {
    if let Some(target) = node.targets.first() {
        return Some(&target.resolved_target);
    }
    let edge = node.edges.first()?;
    leaf_target(&edge.child).or(Some(&edge.resolved_target))
}

fn preflight(fixture: &ParityFixture, cell: &DocumentCell) -> Observed {
    let document = fixture.write_document(cell);
    let md = Markdown::try_from(document.as_path()).expect("read cell document");
    let report = match md.compose_preflight(&request_for(fixture, &document)) {
        Ok(report) => report,
        Err(error) => return error_class(&error),
    };
    match cell.consumer {
        // Pre-flight validates frontmatter but reports no value.
        Consumer::SchemaFile => Observed::Accepted,
        _ => match leaf_target(&report.preflight_graph) {
            Some(PreflightResolvedTarget::File { path, .. }) => Observed::File(path.clone()),
            other => Observed::Unexpected(format!("pre-flight target {other:?}")),
        },
    }
}

fn schema_validation(fixture: &ParityFixture, cell: &DocumentCell) -> Observed {
    let document = fixture.write_document(cell);
    let md = Markdown::try_from(document.as_path()).expect("read cell document");
    let context = build_resolution_context(&snapshot(fixture)).expect("build the context");
    let schemas = DarkmatterSchemas::new(context);
    let report = schemas.validate(&md).expect("prepare the schema");
    if let Some(reference) = report.problems.iter().find_map(|problem| problem.file_reference.as_ref()) {
        return Observed::Failure(reference.resolution_failure());
    }
    if !report.problems.is_empty() {
        return Observed::Unexpected(format!("problems without a file reference: {:?}", report.problems));
    }
    let effective = schemas.effective_for(&md).expect("schema").expect("a declared schema");
    let frontmatter = serde_json::to_value(md.frontmatter().as_map()).expect("frontmatter as JSON");
    let normalized = effective.normalize_frontmatter(&frontmatter, &HashSet::new()).value;
    match normalized["target"].as_str() {
        Some(value) => Observed::File(fixture.stored_value_path(value, &fixture.tree_root(cell))),
        None => Observed::Unexpected(format!("normalized frontmatter {normalized}")),
    }
}

/// A `::file-links` or `find_files()` cell composed: the files it lists, or
/// the class of the failure composition raised or tolerated.
fn compose_glob_listing(fixture: &ParityFixture, cell: &GlobDocumentCell) -> Observed {
    let document = fixture.write_glob_document(cell);
    let md = Markdown::try_from(document.as_path()).expect("read cell document");
    let (composed, report) = match md.compose_with(&request_for(fixture, &document)) {
        Ok(result) => result,
        Err(error) => return error_class(&error),
    };
    if let Some(failure) = report.warnings.iter().find_map(|warning| warning.resolution_failure) {
        return Observed::Failure(failure);
    }
    let output = composed.content();
    match cell.consumer {
        GlobConsumer::FileLinks => match fixture.file_links_listed(output) {
            files if files.is_empty() => Observed::Unexpected(format!("no `::file-links` tree in {output:?}")),
            files => Observed::FileSet(files),
        },
        GlobConsumer::FindFiles => fixture
            .find_files_listed(output)
            .map_or_else(|| Observed::Unexpected(format!("no `find_files()` line in {output:?}")), Observed::Files),
        GlobConsumer::MatchValidation | GlobConsumer::MatchCompletion => unreachable!("{cell:?} lists no files"),
    }
}

/// A Table 1 `match()` validation cell: each candidate's document through
/// `entry`.
fn glob_match_document(fixture: &ParityFixture, cell: &GlobDocumentCell) -> Observed {
    let outcomes = fixture
        .write_glob_match_documents(cell)
        .into_iter()
        .map(|(candidate, document)| {
            let md = Markdown::try_from(document.as_path()).expect("read cell document");
            let outcome = match cell.entry {
                EntryPoint::ComposePipeline => md
                    .compose_with(&request_for(fixture, &document))
                    .map(|_| ())
                    .map_err(|error| format!("{error:?}")),
                EntryPoint::Preflight => md
                    .compose_preflight(&request_for(fixture, &document))
                    .map(|_| ())
                    .map_err(|error| format!("{error:?}")),
                EntryPoint::SchemaValidation => {
                    let schemas = DarkmatterSchemas::new(build_resolution_context(&snapshot(fixture)).expect("context"));
                    let report = schemas.validate(&md).expect("prepare the schema");
                    if report.problems.is_empty() { Ok(()) } else { Err(format!("{:?}", report.problems)) }
                }
                other => unreachable!("{other:?} validates no darkmatter glob document"),
            };
            (candidate, outcome)
        })
        .collect();
    matrix::validation_verdicts(outcomes)
}

/// A Table 2 `match()` validation cell: each candidate supplied as `spec`
/// (`--set`) from the cell's launch directory.
fn glob_match_value(fixture: &ParityFixture, cell: &GlobValueCell) -> Observed {
    let document = fixture.write_glob_value_document(cell);
    let md = Markdown::try_from(document.as_path()).expect("read cell document");
    let outcomes = fixture
        .glob_candidates()
        .into_iter()
        .map(|candidate| {
            let value = biscuit_file::to_portable_string(&candidate);
            let options = ComposeOptions::new()
                .with_source_file(&document)
                .with_set_overrides(serde_json::json!({ "spec": value }));
            let request =
                ComposeRequest::prepare(options, &launch_snapshot(fixture, cell.launch)).expect("prepare the request");
            let outcome = md.compose_with(&request).map(|_| ()).map_err(|error| format!("{error:?}"));
            (candidate, outcome)
        })
        .collect::<Vec<(PathBuf, Result<(), String>)>>();
    matrix::validation_verdicts(outcomes)
}

#[test]
fn every_entry_point_has_a_row() {
    for entry in EntryPoint::ALL {
        assert!(!entry.rows().is_empty(), "{entry:?} has no row in Table 1 or Table 2");
    }
}

/// Every cell costs a repository discovery in `ComposeRequest::prepare`, so one
/// test over all cells is far past the L1 budget; the matrix is split across
/// tests nextest runs in parallel. A `match()` validation row prepares a request
/// per candidate (over a second each), so those rows are spread over their own
/// parts; the light rows share the rest.
const HEAVY_PARTS: usize = 12;
const LIGHT_PARTS: usize = 14;

fn is_heavy(row: &Row) -> bool {
    matches!(row, Row::GlobValue(_) | Row::GlobDocument(GlobDocumentCell { consumer: GlobConsumer::MatchValidation, .. }))
}

/// Every owned entry point has a row, so the parts together run each one.
#[test]
fn every_darkmatter_entry_point_runs_a_matrix_row() {
    let rows = rows_for(Owner::Darkmatter);
    for entry in EntryPoint::ALL.into_iter().filter(|entry| entry.owner() == Owner::Darkmatter) {
        assert!(rows.iter().any(|row| row.entry() == entry), "{entry:?} has no darkmatter matrix row");
    }
}

/// Runs one part of the matrix; the union of all parts is every row.
fn run_matrix_part(part: usize) {
    let root = tempfile::TempDir::new().expect("fixture root");
    let fixture = ParityFixture::create(root.path());
    let mut report = ParityReport::new(Owner::Darkmatter);
    let (mut heavy, mut light) = (0, 0);
    let rows = rows_for(Owner::Darkmatter).into_iter().filter(|row| {
        let slot = if is_heavy(row) {
            heavy += 1;
            (heavy - 1) % HEAVY_PARTS
        } else {
            light += 1;
            HEAVY_PARTS + (light - 1) % LIGHT_PARTS
        };
        slot == part
    });
    for row in rows {
        let cell = match row {
            Row::Document(cell) => cell,
            Row::GlobDocument(cell) => {
                let observed = match cell.consumer {
                    GlobConsumer::FileLinks | GlobConsumer::FindFiles => compose_glob_listing(&fixture, &cell),
                    GlobConsumer::MatchValidation => glob_match_document(&fixture, &cell),
                    GlobConsumer::MatchCompletion => unreachable!("darkmatter completes nothing: {row:?}"),
                };
                report.record(&fixture, &row, &fixture.expected_glob_document(&cell), &observed);
                continue;
            }
            Row::GlobValue(cell) => {
                let observed = glob_match_value(&fixture, &cell);
                report.record(&fixture, &row, &fixture.expected_glob_value(&cell), &observed);
                continue;
            }
            Row::Value(_) => panic!("darkmatter runs no Table 2 reference value: {row:?}"),
        };
        let observed = match cell.entry {
            EntryPoint::ComposePipeline => compose_pipeline(&fixture, &cell),
            EntryPoint::Preflight => preflight(&fixture, &cell),
            EntryPoint::SchemaValidation => schema_validation(&fixture, &cell),
            EntryPoint::MdCompose
            | EntryPoint::MdSchemaValidate
            | EntryPoint::MdArgument(_)
            | EntryPoint::DmlsDiagnostics
            | EntryPoint::DmlsDocumentLinks
            | EntryPoint::DmlsLinkGraph
            | EntryPoint::DmlsDefinition
            | EntryPoint::DmlsCodeActions
            | EntryPoint::ClaudineComposition
            | EntryPoint::ClaudineCompletion
            | EntryPoint::ClaudinePromptArgument
            | EntryPoint::ClaudineSuppliedValue
            | EntryPoint::ClaudineChooser => unreachable!("{:?} is not darkmatter's", cell.entry),
        };
        report.record(&fixture, &row, &fixture.expected_document(&cell), &observed);
    }
    report.assert_cells_agree();
}

macro_rules! matrix_parts {
    ($($name:ident => $part:expr),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                run_matrix_part($part);
            }
        )*
    };
}

matrix_parts! {
    darkmatter_entry_points_agree_on_every_reference_part_00 => 0,
    darkmatter_entry_points_agree_on_every_reference_part_01 => 1,
    darkmatter_entry_points_agree_on_every_reference_part_02 => 2,
    darkmatter_entry_points_agree_on_every_reference_part_03 => 3,
    darkmatter_entry_points_agree_on_every_reference_part_04 => 4,
    darkmatter_entry_points_agree_on_every_reference_part_05 => 5,
    darkmatter_entry_points_agree_on_every_reference_part_06 => 6,
    darkmatter_entry_points_agree_on_every_reference_part_07 => 7,
    darkmatter_entry_points_agree_on_every_reference_part_08 => 8,
    darkmatter_entry_points_agree_on_every_reference_part_09 => 9,
    darkmatter_entry_points_agree_on_every_reference_part_10 => 10,
    darkmatter_entry_points_agree_on_every_reference_part_11 => 11,
    darkmatter_entry_points_agree_on_every_reference_part_12 => 12,
    darkmatter_entry_points_agree_on_every_reference_part_13 => 13,
    darkmatter_entry_points_agree_on_every_reference_part_14 => 14,
    darkmatter_entry_points_agree_on_every_reference_part_15 => 15,
    darkmatter_entry_points_agree_on_every_reference_part_16 => 16,
    darkmatter_entry_points_agree_on_every_reference_part_17 => 17,
    darkmatter_entry_points_agree_on_every_reference_part_18 => 18,
    darkmatter_entry_points_agree_on_every_reference_part_19 => 19,
    darkmatter_entry_points_agree_on_every_reference_part_20 => 20,
    darkmatter_entry_points_agree_on_every_reference_part_21 => 21,
    darkmatter_entry_points_agree_on_every_reference_part_22 => 22,
    darkmatter_entry_points_agree_on_every_reference_part_23 => 23,
    darkmatter_entry_points_agree_on_every_reference_part_24 => 24,
    darkmatter_entry_points_agree_on_every_reference_part_25 => 25,
}

/// The compose pipeline over each `::toc-linking` fallback chain: the
/// selected file's headings, nothing (and no warning) for a suppressed chain,
/// or the first alternative's class.
#[test]
fn compose_pipeline_selects_toc_linking_fallback_alternatives() {
    let root = tempfile::TempDir::new().expect("fixture root");
    let fixture = ParityFixture::create(root.path());
    let mut mismatches = Vec::new();
    for case in ChainCase::ALL {
        let document = fixture.write_chain_document(case);
        if let Err(mismatch) = fixture.compare_chain(case, &compose_outcome(&fixture, &document)) {
            mismatches.push(mismatch);
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// A `#` in a `::file`, `::code`, or `::toc-linking` target is part of the
/// filename, never an anchor.
#[test]
fn compose_pipeline_reads_a_hash_in_a_directive_target_as_part_of_the_filename() {
    let root = tempfile::TempDir::new().expect("fixture root");
    let fixture = ParityFixture::create(root.path());
    let mismatches: Vec<String> = HASH_CONSUMERS
        .into_iter()
        .filter_map(|consumer| {
            let document = fixture.write_hash_document(consumer);
            fixture.compare_hash(consumer, &compose_outcome(&fixture, &document)).err()
        })
        .collect();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// The one target a composed document's output names, nothing (and no
/// warning), or the failure class composition raised or tolerated.
fn compose_outcome(fixture: &ParityFixture, document: &Path) -> ChainOutcome {
    let md = Markdown::try_from(document).expect("read the document");
    match md.compose_with(&request_for(fixture, document)) {
        Err(error) => match error.resolution_failure() {
            Some(failure) => ChainOutcome::Failure(failure),
            None => ChainOutcome::Unexpected(format!("unclassified error: {error}")),
        },
        Ok((_, report)) if let Some(failure) =
            report.warnings.iter().find_map(|warning| warning.resolution_failure) =>
        {
            ChainOutcome::Failure(failure)
        }
        Ok((composed, report)) => {
            let targets = fixture.marked_targets(composed.content());
            match (targets.as_slice(), report.warnings.as_slice()) {
                ([file], []) => ChainOutcome::File(file.clone()),
                ([], []) => ChainOutcome::Suppressed,
                _ => ChainOutcome::Unexpected(format!(
                    "targets {targets:?} and warnings {:?}",
                    report.warnings
                )),
            }
        }
    }
}

/// Pre-flight's answer for a document: the one target it resolved, nothing
/// (a suppressed chain), or the failure class it raised.
fn preflight_outcome(fixture: &ParityFixture, document: &Path) -> ChainOutcome {
    let md = Markdown::try_from(document).expect("read the document");
    match md.compose_preflight(&request_for(fixture, document)) {
        Err(error) => match error.resolution_failure() {
            Some(failure) => ChainOutcome::Failure(failure),
            None => ChainOutcome::Unexpected(format!("unclassified error: {error}")),
        },
        Ok(report) => {
            let graph = &report.preflight_graph;
            match (graph.targets.len() + graph.edges.len(), leaf_target(graph)) {
                (0, _) => ChainOutcome::Suppressed,
                (1, Some(PreflightResolvedTarget::File { path, .. })) => ChainOutcome::File(path.clone()),
                _ => ChainOutcome::Unexpected(format!(
                    "targets {:?} and edges {:?}",
                    graph.targets, graph.edges
                )),
            }
        }
    }
}

/// Pre-flight applies the `::toc-linking` chain rule exactly as composition
/// does.
#[test]
fn preflight_selects_toc_linking_fallback_alternatives() {
    let root = tempfile::TempDir::new().expect("fixture root");
    let fixture = ParityFixture::create(root.path());
    let mismatches: Vec<String> = ChainCase::ALL
        .into_iter()
        .filter_map(|case| {
            let document = fixture.write_chain_document(case);
            fixture.compare_chain(case, &preflight_outcome(&fixture, &document)).err()
        })
        .collect();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// Pre-flight reads a `#` in a directive target as part of the filename.
#[test]
fn preflight_reads_a_hash_in_a_directive_target_as_part_of_the_filename() {
    let root = tempfile::TempDir::new().expect("fixture root");
    let fixture = ParityFixture::create(root.path());
    let mismatches: Vec<String> = HASH_CONSUMERS
        .into_iter()
        .filter_map(|consumer| {
            let document = fixture.write_hash_document(consumer);
            fixture.compare_hash(consumer, &preflight_outcome(&fixture, &document)).err()
        })
        .collect();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// Each directive consumer's existing target resolves in pre-flight and its
/// missing target fails there with `NoMatch`, as in the compose pipeline;
/// none succeeds without checking its target.
#[test]
fn preflight_resolves_existing_and_rejects_missing_directive_targets() {
    let root = tempfile::TempDir::new().expect("fixture root");
    let fixture = ParityFixture::create(root.path());
    let existing = ChainOutcome::File(fixture.repo().join("root-only.md"));
    let missing = ChainOutcome::Failure(biscuit_file::ResolutionFailure::NoMatch);
    let mut mismatches = Vec::new();
    for consumer in HASH_CONSUMERS {
        for (slug, reference, expected) in [("existing", "&root-only.md", &existing), ("missing", "&missing.md", &missing)] {
            let document = fixture.depth_dir(Depth::One).join(format!("target-{consumer:?}-{slug}.md"));
            std::fs::write(&document, consumer.document(reference)).expect("write the document");
            for (entry, observed) in [
                ("pre-flight", preflight_outcome(&fixture, &document)),
                ("compose", compose_outcome(&fixture, &document)),
            ] {
                if !outcome_matches(expected, &observed) {
                    mismatches.push(format!("{entry} {consumer:?} `{reference}`: expected {expected:?}, got {observed:?}"));
                }
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

fn outcome_matches(expected: &ChainOutcome, observed: &ChainOutcome) -> bool {
    match (expected, observed) {
        (ChainOutcome::File(want), ChainOutcome::File(got)) => matrix::identity(want) == matrix::identity(got),
        (want, got) => want == got,
    }
}

/// A request prepared in one repository composes and pre-flights a document
/// from another with the launch `@` scope: `::file @magic.md` reads the
/// launch repository's file. The library derives the source from the
/// request's context, so the scope cannot be lost here; the CLI and Claudine
/// runners hold the rebuilds that could lose it to this expectation.
#[test]
fn external_document_magic_reference_keeps_the_launch_scope() {
    let root = tempfile::TempDir::new().expect("fixture root");
    let fixture = matrix::CrossRepositoryFixture::create(root.path());
    let document = fixture.write_magic_document();
    let request = ComposeRequest::prepare(
        ComposeOptions::new().with_source_file(&document),
        &RequestSnapshot::new(fixture.launch()),
    )
    .expect("prepare the request");
    let md = Markdown::try_from(document.as_path()).expect("read the document");

    let (composed, _) = md.compose_with(&request).expect("compose");
    assert!(
        composed.content().contains(matrix::LAUNCH_MAGIC) && !composed.content().contains(matrix::SOURCE_MAGIC),
        "{}",
        composed.content(),
    );
    let report = md.compose_preflight(&request).expect("pre-flight");
    match leaf_target(&report.preflight_graph) {
        Some(PreflightResolvedTarget::File { path, .. }) => {
            assert_eq!(matrix::identity(path), matrix::identity(&fixture.launch().join("magic.md")));
        }
        other => panic!("pre-flight target {other:?}"),
    }
}
