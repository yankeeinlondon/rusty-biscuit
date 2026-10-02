//! The entry-point parity matrix's darkmatter runner: the compose pipeline,
//! pre-flight, and schema validation, each through its public API with a
//! request snapshot that carries the fixture `HOME` and no process state.

#[path = "../common/entry_point_parity/mod.rs"]
mod matrix;

use std::collections::HashSet;
use std::path::Path;

use darkmatter::markdown::compose::preflight::{PreflightGraphNode, PreflightResolvedTarget};
use darkmatter::markdown::compose::{
    ComposeOptions, ComposeReport, ComposeRequest, RequestSnapshot, build_resolution_context,
};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::{Markdown, MarkdownError};
use matrix::{
    Consumer, DocumentCell, EntryPoint, Observed, Owner, ParityFixture, ParityReport, Row,
    rows_for,
};

/// Table 1's launch directory is the repository root; the snapshot holds the
/// fixture `HOME` and an empty environment.
fn snapshot(fixture: &ParityFixture) -> RequestSnapshot {
    RequestSnapshot::new(fixture.repo()).with_home(Some(fixture.home()))
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

/// The deepest first edge: the cell's own reference, below row (a)'s `~`
/// hop.
fn leaf_target(node: &PreflightGraphNode) -> Option<&PreflightResolvedTarget> {
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

#[test]
fn every_entry_point_has_a_row() {
    for entry in EntryPoint::ALL {
        assert!(!entry.rows().is_empty(), "{entry:?} has no row in Table 1 or Table 2");
    }
}

#[test]
fn darkmatter_entry_points_agree_on_every_reference() {
    let root = tempfile::TempDir::new().expect("fixture root");
    let fixture = ParityFixture::create(root.path());
    let mut report = ParityReport::new(Owner::Darkmatter);
    for row in rows_for(Owner::Darkmatter) {
        let Row::Document(cell) = row else {
            panic!("darkmatter runs Table 1 only: {row:?}");
        };
        let observed = match cell.entry {
            EntryPoint::ComposePipeline => compose_pipeline(&fixture, &cell),
            EntryPoint::Preflight => preflight(&fixture, &cell),
            EntryPoint::SchemaValidation => schema_validation(&fixture, &cell),
            EntryPoint::MdCompose
            | EntryPoint::MdSchemaValidate
            | EntryPoint::MdArgument
            | EntryPoint::DmlsDiagnostics
            | EntryPoint::DmlsDocumentLinks
            | EntryPoint::DmlsLinkGraph
            | EntryPoint::DmlsDefinition
            | EntryPoint::DmlsCodeActions
            | EntryPoint::ClaudineComposition
            | EntryPoint::ClaudineCompletion => unreachable!("{:?} is not darkmatter's", cell.entry),
        };
        report.record(&fixture, &row, &fixture.expected_document(&cell), &observed);
    }
    report.assert_parity();
}
