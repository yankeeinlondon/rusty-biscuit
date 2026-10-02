//! The entry-point parity matrix's darkmatter runner: the compose pipeline,
//! pre-flight, and schema validation, each through its public API with a
//! request snapshot that carries the fixture `HOME`, the configured extra `@`
//! root, and no process state.

#[path = "../common/entry_point_parity/mod.rs"]
mod matrix;

use std::collections::HashSet;
use std::path::Path;

use biscuit_file::PathPosition;
use darkmatter::markdown::compose::preflight::{PreflightGraphNode, PreflightResolvedTarget};
use darkmatter::markdown::compose::{
    ComposeOptions, ComposeReport, ComposeRequest, RequestSnapshot, build_resolution_context,
};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::{Markdown, MarkdownError};
use matrix::{
    ChainCase, ChainOutcome, Consumer, Depth, HASH_CONSUMERS, DocumentCell, EntryPoint, Observed, Owner, ParityFixture,
    ParityReport, Row, rows_for,
};

/// Table 1's launch directory is the repository root; the snapshot holds the
/// fixture `HOME`, the configured extra `@` root, and an empty environment.
fn snapshot(fixture: &ParityFixture) -> RequestSnapshot {
    RequestSnapshot::new(fixture.repo())
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
            | EntryPoint::ClaudineCompletion
            | EntryPoint::ClaudinePromptArgument
            | EntryPoint::ClaudineSuppliedValue => unreachable!("{:?} is not darkmatter's", cell.entry),
        };
        report.record(&fixture, &row, &fixture.expected_document(&cell), &observed);
    }
    report.assert_parity();
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
