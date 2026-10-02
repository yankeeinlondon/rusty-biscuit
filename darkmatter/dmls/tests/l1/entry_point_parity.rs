//! The entry-point parity matrix's DMLS runner: published diagnostics,
//! document links, go-to-definition, and code actions, each through one
//! in-memory LSP session whose request snapshot carries the fixture `HOME`
//! and the configured extra `@` root, and the workspace link graph, indexed
//! through the public `index_workspace` pipeline with the same snapshot.
//!
//! Every editor surface runs every consumer: `::file`, `::code`,
//! `::toc-linking`, a `file(eager)` value, and a Markdown link. Code actions
//! offer a create-file fix only for a broken Markdown link, so for every
//! other consumer the cell asserts that none is offered.
//!
//! DMLS receives documents only as file URIs, so every cell is Table 1 row
//! (b): a document opened by its absolute path. The workspace folder is the
//! fixture root, so the graph indexes every target (including `HOME` and
//! `outside.md`); each document's request directory is still its repository
//! root. A failure is read from the `resolution_failure` class on the
//! reference's diagnostic (`data: {"resolution_failure": "<Class>"}`), never
//! from a message.
//!
//! Because that folder indexes every target, a second test opens only a
//! `docs/` folder below the repository root, so each target is an existing
//! file the editor never indexed.

#[path = "../../../lib/tests/common/entry_point_parity/mod.rs"]
mod matrix;

use crate::common;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{PathPosition, ResolutionFailure};
use darkmatter::markdown::compose::RequestSnapshot;
use dmls::config::WorkspaceConfig;
use dmls::context::RepositoryContexts;
use dmls::graph::{EdgeKind, EdgeTarget, NodeId, NodeKind, WorkspaceGraph};
use dmls::workspace::{SilentProgress, index_workspace};
use common::{LspFixture, LspWorkspace};
use matrix::{ChainCase, ChainOutcome, Consumer, DocumentCell, HASH_CONSUMERS, EntryPoint, Observed, Owner, ParityFixture, ParityReport, Row, rows_for};
use serde_json::{Value, json};

const CLASSES: [ResolutionFailure; 5] = [
    ResolutionFailure::InvalidReference,
    ResolutionFailure::MissingContext,
    ResolutionFailure::NoMatch,
    ResolutionFailure::Io,
    ResolutionFailure::UnsupportedRemote,
];

/// A client that watches files, with work-done progress so the test can wait
/// for the startup index, and resource operations so create-file actions are
/// offered.
fn client_params(folder: &Path) -> Value {
    json!({
        "processId": null,
        "clientInfo": { "name": "Parity Client", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "window": { "workDoneProgress": true },
            "workspace": {
                "didChangeWatchedFiles": { "dynamicRegistration": true },
                "workspaceEdit": { "documentChanges": true, "resourceOperations": ["create"] }
            },
            "textDocument": { "codeAction": {} }
        },
        "workspaceFolders": [
            { "uri": url::Url::from_directory_path(folder).unwrap().as_str(), "name": "parity" }
        ]
    })
}

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn uri_path(target: &str) -> PathBuf {
    url::Url::parse(target).unwrap().to_file_path().unwrap()
}

/// What the session published and linked for one open cell document.
struct DocumentState {
    uri: String,
    text: String,
    diagnostics: Vec<Value>,
    /// Every document-link target, in response order.
    links: Vec<PathBuf>,
}

impl DocumentState {
    /// The classes on this document's file-reference diagnostics.
    fn classes(&self) -> Vec<ResolutionFailure> {
        self.classified().map(|(_, class)| class).collect()
    }

    fn classified(&self) -> impl Iterator<Item = (&Value, ResolutionFailure)> {
        self.diagnostics.iter().filter_map(|diagnostic| {
            let name = diagnostic["data"]["resolution_failure"].as_str()?;
            let class = CLASSES.into_iter().find(|class| format!("{class:?}") == name);
            Some((diagnostic, class.unwrap_or_else(|| panic!("unknown class {name}"))))
        })
    }
}

/// The reference's position in a cell document: one column into the
/// reference text, which every consumer's span covers.
fn reference_position(consumer: Consumer, text: &str) -> (u32, u32) {
    let (line_index, line) = text
        .lines()
        .enumerate()
        .find(|(_, line)| match consumer {
            Consumer::File | Consumer::Code | Consumer::TocLinking => line.starts_with("::"),
            Consumer::SchemaFile => line.starts_with("target: "),
            Consumer::MarkdownLink => line.starts_with("[target]("),
        })
        .unwrap_or_else(|| panic!("no {consumer:?} reference in {text:?}"));
    let column = match consumer {
        Consumer::File | Consumer::Code | Consumer::TocLinking => line.find(' ').unwrap() + 1,
        Consumer::SchemaFile => line.find('"').unwrap() + 1,
        Consumer::MarkdownLink => line.find("](").unwrap() + 2,
    };
    (line_index as u32, column as u32 + 1)
}

/// One linked file, else the class on the reference's diagnostic. A link to
/// a file that does not exist (DMLS navigates a missing frontmatter value to
/// where it would be created) is no resolution.
fn linked_or_class(state: &DocumentState) -> Observed {
    let classes = state.classes();
    let files: Vec<&PathBuf> = state.links.iter().filter(|path| path.is_file()).collect();
    match (files.as_slice(), classes.as_slice()) {
        ([file], []) => Observed::File((*file).clone()),
        ([], [class]) => Observed::Failure(*class),
        _ => Observed::Unexpected(format!(
            "links {:?} and classes {classes:?} in {:#?}",
            state.links, state.diagnostics
        )),
    }
}

fn document_links(state: &DocumentState) -> Observed {
    linked_or_class(state)
}

/// Schema validation of a `file(eager)` value: no classified diagnostic
/// means the value resolved, and its frontmatter document link names the
/// file; otherwise the diagnostic's class.
fn diagnostics(state: &DocumentState) -> Observed {
    linked_or_class(state)
}

/// The graph node kind and edge kind that carry `consumer`'s reference.
fn graph_edge(consumer: Consumer) -> (NodeKind, EdgeKind) {
    match consumer {
        Consumer::MarkdownLink => (NodeKind::Link, EdgeKind::References),
        Consumer::File | Consumer::Code => (NodeKind::TransclusionTarget, EdgeKind::Transcludes),
        Consumer::TocLinking | Consumer::SchemaFile => (NodeKind::FileReference, EdgeKind::UsesFile),
    }
}

/// The workspace link graph's answer for the cell document's one reference:
/// the indexed document or unindexed file its edge targets. With no resolved
/// edge (a broken transclusion has none; a broken link or file use is
/// `Unresolved`), the class is the one on the reference's published
/// diagnostic, which the graph's `diagnose_unresolved` and the directive
/// diagnostics compute.
fn link_graph(graph: &WorkspaceGraph, document: &Path, consumer: Consumer, state: &DocumentState) -> Observed {
    let Some(doc_id) = graph
        .documents()
        .iter()
        .position(|record| matrix::identity(&record.path) == matrix::identity(document))
    else {
        return Observed::Unexpected(format!("{} is not indexed", document.display()));
    };
    let (node_kind, edge_kind) = graph_edge(consumer);
    let mut files = Vec::new();
    for id in 0..graph.node_count() {
        let node_id = NodeId(id as u32);
        let Some(node) = graph.node(node_id) else { continue };
        if node.document.0 as usize != doc_id || node.kind != node_kind {
            continue;
        }
        for edge in graph.outgoing(node_id, edge_kind) {
            match &edge.target {
                EdgeTarget::Node(target) => {
                    let record = graph.node(*target).and_then(|target| graph.document(target.document));
                    files.extend(record.map(|record| record.path.clone()));
                }
                EdgeTarget::File(path) => files.push(path.clone()),
                EdgeTarget::Unresolved(_) => {}
            }
        }
    }
    let classes = state.classes();
    match (files.as_slice(), classes.as_slice()) {
        ([file], []) => Observed::File(file.clone()),
        ([], [class]) => Observed::Failure(*class),
        _ => Observed::Unexpected(format!(
            "graph {edge_kind:?} targets {files:?} and classes {classes:?} in {:#?}",
            state.diagnostics
        )),
    }
}

/// A Markdown link's graph projection in the session: its document link is
/// the graph's `references` edge (`resolved_document_path`), and a missing
/// edge is the graph's `dm.links.broken_path` diagnostic
/// (`diagnose_unresolved`).
fn markdown_link_projection(state: &DocumentState) -> Observed {
    let broken: Vec<ResolutionFailure> = state
        .classified()
        .filter(|(diagnostic, _)| diagnostic["code"] == json!("dm.links.broken_path"))
        .map(|(_, class)| class)
        .collect();
    match (state.links.as_slice(), broken.as_slice()) {
        ([file], []) => Observed::File(file.clone()),
        ([], [class]) => Observed::Failure(*class),
        _ => Observed::Unexpected(format!(
            "graph links {:?} and broken classes {broken:?} in {:#?}",
            state.links, state.diagnostics
        )),
    }
}

/// Go-to-definition on the reference: one location, else no location and
/// the class on the reference's diagnostic (R5).
fn definition(session: &mut LspFixture<'_>, consumer: Consumer, state: &DocumentState) -> Observed {
    let (line, character) = reference_position(consumer, &state.text);
    let response = session.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": state.uri },
            "position": { "line": line, "character": character }
        }),
    );
    let targets: Vec<PathBuf> = response
        .result
        .unwrap_or(Value::Null)
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|location| location["uri"].as_str().map(uri_path))
        .collect();
    let classes = state.classes();
    match (targets.as_slice(), classes.as_slice()) {
        ([file], []) => Observed::File(file.clone()),
        ([], [class]) => Observed::Failure(*class),
        _ => Observed::Unexpected(format!("definitions {targets:?} and classes {classes:?}")),
    }
}

/// Code actions over the document's diagnostics, observed through the
/// document's link or class. Only a broken Markdown link (`NoMatch`) may be
/// offered a create-file fix: a resolved reference, a tree escape, and every
/// directive or schema value must be offered none.
fn code_actions(session: &mut LspFixture<'_>, consumer: Consumer, state: &DocumentState) -> Observed {
    let start = json!({ "line": 0, "character": 0 });
    let range = state
        .classified()
        .next()
        .map_or_else(|| json!({ "start": start, "end": start }), |(diagnostic, _)| diagnostic["range"].clone());
    let response = session.request(
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": state.uri },
            "range": range,
            "context": { "diagnostics": state.diagnostics }
        }),
    );
    let created: Vec<String> = response
        .result
        .unwrap_or(Value::Null)
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .flat_map(|action| action["edit"]["documentChanges"].as_array().cloned().unwrap_or_default())
        .filter(|change| change["kind"] == json!("create"))
        .filter_map(|change| change["uri"].as_str().map(str::to_string))
        .collect();
    let observed = linked_or_class(state);
    match (consumer, &observed, created.as_slice()) {
        (_, _, []) => observed,
        (Consumer::MarkdownLink, Observed::Failure(ResolutionFailure::NoMatch), _) => observed,
        _ => Observed::Unexpected(format!("{consumer:?} {observed:?} offered create-file {created:?}")),
    }
}

/// Opens `document`, then records what DMLS published and linked for it.
fn open_document(session: &mut LspFixture<'_>, document: &Path) -> DocumentState {
    let document_uri = uri(document);
    let text = std::fs::read_to_string(document).unwrap();
    session.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": { "uri": document_uri, "languageId": "markdown", "version": 1, "text": text }
        }),
    );
    session.flush_server();
    let latest = session
        .take_buffered_diagnostics(&document_uri)
        .unwrap_or_else(|| panic!("no diagnostics published for {document_uri}"));
    while session.take_buffered_diagnostics(&document_uri).is_some() {}
    let response =
        session.request("textDocument/documentLink", json!({ "textDocument": { "uri": document_uri } }));
    let links = response
        .result
        .unwrap_or(Value::Null)
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|link| link["target"].as_str().map(uri_path))
        .collect();
    DocumentState {
        uri: document_uri,
        text,
        diagnostics: latest["diagnostics"].as_array().cloned().unwrap_or_default(),
        links,
    }
}

#[test]
fn dmls_entry_points_agree_on_every_reference() {
    // The shared matrix is a test input of this test (CI's test-input index).
    let _ = include_str!("../../../lib/tests/common/entry_point_parity/mod.rs");
    let workspace = LspWorkspace::new();
    let fixture = ParityFixture::create(workspace.path());

    // Entry points sharing a consumer share one document. Every document is
    // written before startup, so the startup index holds every target and
    // every cell document.
    let mut documents: Vec<(PathBuf, Vec<(Row, DocumentCell)>)> = Vec::new();
    for row in rows_for(Owner::Dmls) {
        let Row::Document(cell) = row else {
            panic!("DMLS runs Table 1 only: {row:?}");
        };
        let path = fixture.write_document(&cell);
        match documents.iter_mut().find(|(document, _)| *document == path) {
            Some((_, cells)) => cells.push((row, cell)),
            None => documents.push((path, vec![(row, cell)])),
        }
    }

    let snapshot = fixture_snapshot(&workspace, &fixture);
    let graph = index_workspace(
        &[fixture.root().to_path_buf()],
        &WorkspaceConfig::default(),
        &SilentProgress,
        Arc::new(RepositoryContexts::new(snapshot.clone())),
    )
    .snapshot();
    let mut session = LspFixture::start_with_snapshot(&workspace, snapshot);
    session.initialize(client_params(fixture.root()));
    session.wait_for_startup_complete();

    // One document open at a time: DMLS republishes every open document's
    // diagnostics on each open, so opening all of them is quadratic.
    let mut report = ParityReport::new(Owner::Dmls);
    for (document, cells) in &documents {
        let state = open_document(&mut session, document);
        for (row, cell) in cells {
            let observed = match cell.entry {
                EntryPoint::DmlsDiagnostics => diagnostics(&state),
                EntryPoint::DmlsDocumentLinks => document_links(&state),
                EntryPoint::DmlsLinkGraph => link_graph(&graph, document, cell.consumer, &state),
                EntryPoint::DmlsDefinition => definition(&mut session, cell.consumer, &state),
                EntryPoint::DmlsCodeActions => code_actions(&mut session, cell.consumer, &state),
                EntryPoint::ComposePipeline
                | EntryPoint::Preflight
                | EntryPoint::SchemaValidation
                | EntryPoint::MdCompose
                | EntryPoint::MdSchemaValidate
                | EntryPoint::MdArgument
                | EntryPoint::ClaudineComposition
                | EntryPoint::ClaudineCompletion
                | EntryPoint::ClaudinePromptArgument
                | EntryPoint::ClaudineSuppliedValue => unreachable!("{:?} is not DMLS's", cell.entry),
            };
            report.record(&fixture, row, &fixture.expected_document(cell), &observed);
        }
        session.notify("textDocument/didClose", json!({ "textDocument": { "uri": state.uri } }));
    }
    session.shutdown();
    report.assert_parity();
}

/// Every authored form that reaches `repo/target.md` (or `HOME/target.md`)
/// from `repo/docs/`, which lies below it.
const ABOVE_THE_FOLDER: [&str; 6] = ["&target.md", "^target.md", "@target.md", "~/target.md", "target.md", "../target.md"];

/// Hover text over the reference.
fn hover(session: &mut LspFixture<'_>, state: &DocumentState) -> String {
    hover_on(session, Consumer::MarkdownLink, state)
}

fn hover_on(session: &mut LspFixture<'_>, consumer: Consumer, state: &DocumentState) -> String {
    let (line, character) = reference_position(consumer, &state.text);
    let response = session.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": state.uri },
            "position": { "line": line, "character": character }
        }),
    );
    response.result.unwrap_or(Value::Null)["contents"]["value"].as_str().unwrap_or_default().to_string()
}

/// Whether the reference's diagnostics offer a create-file action.
fn offers_create_file(session: &mut LspFixture<'_>, state: &DocumentState) -> bool {
    let start = json!({ "line": 0, "character": 0 });
    let end = json!({ "line": state.text.lines().count(), "character": 0 });
    let response = session.request(
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": state.uri },
            "range": { "start": start, "end": end },
            "context": { "diagnostics": state.diagnostics }
        }),
    );
    response
        .result
        .unwrap_or(Value::Null)
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .flat_map(|action| action["edit"]["documentChanges"].as_array().cloned().unwrap_or_default())
        .any(|change| change["kind"] == json!("create"))
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// Opens every form's document under `folder`, bare and with `fragment`,
/// returning one line per disagreement with the expected target.
fn above_the_folder_mismatches(
    folder: &Path,
    fragment: &str,
    workspace: &LspWorkspace,
    repo: &Path,
    home: &Path,
    magic: &Path,
) -> Vec<String> {
    let snapshot = RequestSnapshot::new(workspace.path())
        .with_home(Some(home.to_path_buf()))
        .with_magic_root(magic, PathPosition::End);
    let mut session = LspFixture::start_with_snapshot(workspace, snapshot);
    session.initialize(client_params(folder));
    session.wait_for_startup_complete();

    let mut mismatches = Vec::new();
    for (index, form) in ABOVE_THE_FOLDER.iter().enumerate() {
        let expected = canonical(&if form.starts_with('~') { home.join("target.md") } else { repo.join("target.md") });
        for reference in [form.to_string(), format!("{form}#{fragment}")] {
            let document = repo.join(format!("docs/form-{index}.md"));
            std::fs::write(&document, format!("# Form\n\n[target]({reference})\n")).unwrap();
            let state = open_document(&mut session, &document);
            let mut check = |surface: &str, observed: Observed| match observed {
                Observed::File(path) if canonical(&path) == expected => {}
                other => mismatches.push(format!("{} `{reference}` {surface}: {other:?}", folder.display())),
            };
            check("link graph and broken-link diagnostic", markdown_link_projection(&state));
            check("document link", document_links(&state));
            check("definition", definition(&mut session, Consumer::MarkdownLink, &state));
            let hover = hover(&mut session, &state);
            if hover.contains("Unresolved") || !hover.contains("target.md") {
                mismatches.push(format!("{} `{reference}` hover: {hover:?}", folder.display()));
            }
            if offers_create_file(&mut session, &state) {
                mismatches.push(format!("{} `{reference}` code actions offer to create the file", folder.display()));
            }
            session.notify("textDocument/didClose", json!({ "textDocument": { "uri": state.uri } }));
        }
    }
    session.shutdown();
    mismatches
}

#[test]
fn markdown_links_resolve_existing_targets_above_a_narrower_workspace_folder() {
    let workspace = LspWorkspace::new();
    let repo = workspace.path().join("repo");
    let home = workspace.path().join("home");
    let magic = workspace.path().join("magic");
    std::fs::create_dir_all(repo.join(".git/objects")).unwrap();
    std::fs::create_dir_all(repo.join(".git/refs/heads")).unwrap();
    std::fs::write(repo.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(repo.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n").unwrap();
    std::fs::create_dir_all(repo.join("docs")).unwrap();
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&magic).unwrap();
    std::fs::write(repo.join("target.md"), "# Target\n").unwrap();
    std::fs::write(home.join("target.md"), "# Target\n").unwrap();

    // The narrower folder indexes only `docs/`, so a fragment on a target is
    // unknown, never missing. The whole repository is the positive control:
    // its targets are indexed and their anchors checked.
    let mut mismatches =
        above_the_folder_mismatches(&repo.join("docs"), "anywhere", &workspace, &repo, &home, &magic);
    mismatches.extend(above_the_folder_mismatches(&repo, "target", &workspace, &repo, &home, &magic));
    assert!(mismatches.is_empty(), "{} mismatches:\n{}", mismatches.len(), mismatches.join("\n"));
}

/// A chain's links (or definitions) and classes: one file, no file and no
/// class (suppressed), or one class.
fn chain_outcome(files: &[PathBuf], classes: &[ResolutionFailure]) -> ChainOutcome {
    match (files, classes) {
        ([file], []) => ChainOutcome::File(file.clone()),
        ([], []) => ChainOutcome::Suppressed,
        ([], [class]) => ChainOutcome::Failure(*class),
        _ => ChainOutcome::Unexpected(format!("files {files:?} and classes {classes:?}")),
    }
}

/// The hover's verdict: the chosen file, intentional suppression, or a
/// missing target (whose class is the diagnostic's).
fn chain_hover(hover: &str, state: &DocumentState, fixture: &ParityFixture) -> ChainOutcome {
    if hover.contains("⚠️") {
        return match state.classes().as_slice() {
            [class] => ChainOutcome::Failure(*class),
            other => ChainOutcome::Unexpected(format!("missing-target hover with classes {other:?}: {hover}")),
        };
    }
    if hover.contains("renders nothing") {
        return ChainOutcome::Suppressed;
    }
    let root_only = canonical(&fixture.repo().join("root-only.md"));
    match hover.split('`').find(|segment| Path::new(segment).is_file()) {
        Some(path) => ChainOutcome::File(PathBuf::from(path)),
        None if hover.contains(&*root_only.to_string_lossy()) => ChainOutcome::File(root_only),
        None => ChainOutcome::Unexpected(format!("hover {hover:?}")),
    }
}

/// What each DMLS projection of `state`'s one directive target observed:
/// diagnostics with document links, definition, and hover; plus a
/// `code actions` mismatch when a create-file action is offered.
fn directive_outcomes(
    session: &mut LspFixture<'_>,
    consumer: Consumer,
    state: &DocumentState,
    fixture: &ParityFixture,
) -> Vec<(&'static str, ChainOutcome)> {
    let classes = state.classes();
    let linked: Vec<PathBuf> = state.links.iter().filter(|path| path.is_file()).cloned().collect();
    let (line, character) = reference_position(consumer, &state.text);
    let definitions: Vec<PathBuf> = session
        .request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": state.uri },
                "position": { "line": line, "character": character }
            }),
        )
        .result
        .unwrap_or(Value::Null)
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|location| location["uri"].as_str().map(uri_path))
        .collect();
    let hover = hover_on(session, consumer, state);
    let mut outcomes = vec![
        ("diagnostics and document links", chain_outcome(&linked, &classes)),
        ("definition", chain_outcome(&definitions, &classes)),
        ("hover", chain_hover(&hover, state, fixture)),
    ];
    if offers_create_file(session, state) {
        outcomes.push(("code actions", ChainOutcome::Unexpected("offered a create-file action".into())));
    }
    outcomes
}

/// The editor's startup snapshot: the fixture `HOME` and the configured
/// extra `@` root.
fn fixture_snapshot(workspace: &LspWorkspace, fixture: &ParityFixture) -> RequestSnapshot {
    LspFixture::fixture_snapshot(workspace)
        .with_home(Some(fixture.home()))
        .with_magic_root(fixture.configured_magic_root(), PathPosition::Start)
}

/// Starts a session over `fixture` with its `HOME` and configured `@` root.
fn start_session<'w>(workspace: &'w LspWorkspace, fixture: &ParityFixture) -> LspFixture<'w> {
    let snapshot = fixture_snapshot(workspace, fixture);
    let mut session = LspFixture::start_with_snapshot(workspace, snapshot);
    session.initialize(client_params(fixture.root()));
    session.wait_for_startup_complete();
    session
}

/// Every DMLS projection of a `::toc-linking` fallback chain agrees with
/// composition (`lib/tests/l1/entry_point_parity.rs`): diagnostics, document
/// links, definition, and hover select the first existing alternative, a
/// chain ending in `false` is never a broken target, and no chain is offered
/// a create-file action.
#[test]
fn directive_projections_select_toc_linking_fallback_alternatives() {
    let workspace = LspWorkspace::new();
    let fixture = ParityFixture::create(workspace.path());
    let documents: Vec<(ChainCase, PathBuf)> =
        ChainCase::ALL.into_iter().map(|case| (case, fixture.write_chain_document(case))).collect();
    let mut session = start_session(&workspace, &fixture);

    let mut mismatches = Vec::new();
    for (case, document) in &documents {
        let state = open_document(&mut session, document);
        for (surface, observed) in directive_outcomes(&mut session, Consumer::TocLinking, &state, &fixture) {
            if let Err(mismatch) = fixture.compare_chain(*case, &observed) {
                mismatches.push(format!("{surface}: {mismatch}"));
            }
        }
        session.notify("textDocument/didClose", json!({ "textDocument": { "uri": state.uri } }));
    }
    session.shutdown();
    assert!(mismatches.is_empty(), "{} mismatches:\n{}", mismatches.len(), mismatches.join("\n"));
}

/// A `#` in a `::file`, `::code`, or `::toc-linking` target is part of the
/// filename in every DMLS projection, as in composition: `&root-only.md#x`
/// is a broken target even though `root-only.md` exists.
#[test]
fn directive_projections_read_a_hash_in_a_target_as_part_of_the_filename() {
    let workspace = LspWorkspace::new();
    let fixture = ParityFixture::create(workspace.path());
    let documents: Vec<(Consumer, PathBuf)> =
        HASH_CONSUMERS.into_iter().map(|consumer| (consumer, fixture.write_hash_document(consumer))).collect();
    let mut session = start_session(&workspace, &fixture);

    let mut mismatches = Vec::new();
    for (consumer, document) in &documents {
        let state = open_document(&mut session, document);
        for (surface, observed) in directive_outcomes(&mut session, *consumer, &state, &fixture) {
            if let Err(mismatch) = fixture.compare_hash(*consumer, &observed) {
                mismatches.push(format!("{surface}: {mismatch}"));
            }
        }
        session.notify("textDocument/didClose", json!({ "textDocument": { "uri": state.uri } }));
    }
    session.shutdown();
    assert!(mismatches.is_empty(), "{} mismatches:\n{}", mismatches.len(), mismatches.join("\n"));
}
