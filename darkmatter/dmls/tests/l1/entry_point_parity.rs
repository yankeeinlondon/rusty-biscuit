//! The entry-point parity matrix's DMLS runner: published diagnostics,
//! document links, the workspace link graph, go-to-definition, and code
//! actions, each through one in-memory LSP session whose request snapshot
//! carries the fixture `HOME`.
//!
//! DMLS receives documents only as file URIs, so every cell is Table 1 row
//! (b): a document opened by its absolute path. The workspace folder is the
//! fixture root, so the graph indexes every target (including `HOME` and
//! `outside.md`); each document's request directory is still its repository
//! root. A failure is read from the `resolution_failure` class on the
//! reference's diagnostic (`data: {"resolution_failure": "<Class>"}`), never
//! from a message.

#[path = "../../../lib/tests/common/entry_point_parity/mod.rs"]
mod matrix;

use crate::common;

use std::path::{Path, PathBuf};

use biscuit_file::ResolutionFailure;
use common::{LspFixture, LspWorkspace};
use matrix::{Consumer, DocumentCell, EntryPoint, Observed, Owner, ParityFixture, ParityReport, Row, rows_for};
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

/// The workspace link graph: a Markdown link's document link is the graph's
/// `references` edge (`resolved_document_path`), and a missing edge is the
/// graph's `dm.links.broken_path` diagnostic (`diagnose_unresolved`). Both
/// read the indexed workspace, not the disk.
fn link_graph(state: &DocumentState) -> Observed {
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
fn definition(session: &mut LspFixture<'_>, cell: &DocumentCell, state: &DocumentState) -> Observed {
    let (line, character) = reference_position(cell.consumer, &state.text);
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

/// Code actions over the document's diagnostics: a resolved reference is
/// offered no create-file fix (and is observed through its link), and a
/// tree escape must never be offered a file outside the tree.
fn code_actions(session: &mut LspFixture<'_>, state: &DocumentState) -> Observed {
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
    match (&observed, created.as_slice()) {
        (_, []) => observed,
        (Observed::Failure(ResolutionFailure::NoMatch), _) => observed,
        _ => Observed::Unexpected(format!("{observed:?} offered create-file {created:?}")),
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

    let snapshot = LspFixture::fixture_snapshot(&workspace).with_home(Some(fixture.home()));
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
                EntryPoint::DmlsLinkGraph => link_graph(&state),
                EntryPoint::DmlsDefinition => definition(&mut session, cell, &state),
                EntryPoint::DmlsCodeActions => code_actions(&mut session, &state),
                EntryPoint::ComposePipeline
                | EntryPoint::Preflight
                | EntryPoint::SchemaValidation
                | EntryPoint::MdCompose
                | EntryPoint::MdSchemaValidate
                | EntryPoint::MdArgument
                | EntryPoint::ClaudineComposition
                | EntryPoint::ClaudineCompletion => unreachable!("{:?} is not DMLS's", cell.entry),
            };
            report.record(&fixture, row, &fixture.expected_document(cell), &observed);
        }
        session.notify("textDocument/didClose", json!({ "textDocument": { "uri": state.uri } }));
    }
    session.shutdown();
    report.assert_parity();
}
