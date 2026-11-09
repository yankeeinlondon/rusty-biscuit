//! `load_data` options inside schema slots: observations re-recorded at every
//! memo layer, the slot gate before any warmed layer, one generation for all
//! layers, and a three-argument call's target collected passively.
//!
//! Each case works in its own temp folder with no repository and an empty
//! temp home (so no persisted grant applies), with every request anchored
//! there, and shares one explicit data runtime across the compositions it
//! runs so the memo stays warm between them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::data::{Observation, ObservationKind};
use darkmatter::markdown::compose::expression::read_gate::ReadLocation;
use darkmatter::markdown::compose::remote::discover_remote_urls_from_expressions;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionOptions,
};
use darkmatter::markdown::compose::{
    ComposeFailure, ComposeOptions, ComposeRequest, ComposeSource, DataRuntime, DataRuntimeStats, RequestSnapshot,
    ShellExpansionError,
};
use darkmatter::markdown::schemas::slots::grants::{SlotGrantDecision, SlotGrantRequest};

const APPROVAL_REQUIRED: &str = "dm.schema.slot_approval_required";
const ROWS: &str = "rows.csv";
const ROWS_TEXT: &str = "a\n1\n";

/// A headerless read of `rows.csv`; the same options across cases so a slot
/// call can meet the layers a document call warmed.
const HEADERLESS: &str = "load_data('./rows.csv', 'tuple([string])[]', {header: false})";

/// A temp folder holding `rows.csv`, a temp home, and the shared runtime.
struct Case {
    _temp: tempfile::TempDir,
    root: PathBuf,
    home: tempfile::TempDir,
    runtime: DataRuntime,
}

impl Case {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(temp.path()).expect("canonical temp dir");
        let case = Self { _temp: temp, root, home: tempfile::tempdir().expect("temp home"), runtime: DataRuntime::new() };
        case.write(ROWS, ROWS_TEXT);
        case
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.path(name);
        std::fs::write(&path, text).expect("write");
        path
    }

    fn request(&self, document: &Path, options: ComposeOptions) -> ComposeRequest {
        let snapshot = RequestSnapshot::new(&self.root).with_home(Some(self.home.path().to_path_buf())).with_env(HashMap::new());
        let options = options.with_source_file(document).with_data_runtime(self.runtime.clone());
        ComposeRequest::prepare(options, &snapshot).expect("request")
    }

    fn compose_with(&self, document: &Path, options: ComposeOptions) -> Result<String, Box<ComposeFailure>> {
        let request = self.request(document, options);
        let markdown = Markdown::try_from(document).expect("document");
        markdown.compose_with_warnings(&request).map(|(composed, _)| composed.content().trim().to_string())
    }

    /// Composes `body` as `name` and returns the composed text.
    fn compose_body(&self, name: &str, body: &str) -> String {
        let document = self.write(name, body);
        self.compose_with(&document, ComposeOptions::new())
            .unwrap_or_else(|failure| panic!("{body:?} composes: {}", failure.error))
    }

    /// The observation one read of `rows.csv` adds.
    fn rows_read(&self) -> Observation {
        Observation {
            entry: biscuit_file::to_portable_string(&self.path(ROWS)),
            kind: ObservationKind::File,
            hash: Some(biscuit_hash::blake3_hash_bytes(ROWS_TEXT.as_bytes())),
        }
    }

    /// The observations of `rows.csv` recorded while `run` ran.
    fn rows_observed_during(&self, run: impl FnOnce()) -> Vec<Observation> {
        let entry = biscuit_file::to_portable_string(&self.path(ROWS));
        let mark = self.runtime.read_mark();
        run();
        self.runtime.observations_since(mark).into_iter().filter(|observation| observation.entry == entry).collect()
    }
}

/// The counters that show which memo layer answered a call.
fn layers(stats: DataRuntimeStats) -> (usize, usize, usize, usize, usize) {
    (stats.reads, stats.parses, stats.binds, stats.source_hits, stats.bind_hits)
}

/// An approval handler that leaves every slot read unanswered and records
/// what it was asked; it denies every shell command.
#[derive(Default)]
struct Recording {
    asked: Mutex<Vec<SlotGrantRequest>>,
}

impl Recording {
    fn asked(&self) -> Vec<SlotGrantRequest> {
        self.asked.lock().expect("asked").clone()
    }
}

impl ShellApprovalHandler for Recording {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        Ok(ShellApprovalDecision::Deny)
    }

    fn approve_slot_reads(&self, requests: &[SlotGrantRequest]) -> Result<Vec<SlotGrantDecision>, ShellExpansionError> {
        self.asked.lock().expect("asked").extend(requests.iter().cloned());
        Ok(requests.iter().map(|_| SlotGrantDecision::Unanswered).collect())
    }
}

struct AllowShell;

impl ShellApprovalHandler for AllowShell {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

/// `doc.md` under an inline `$schema` whose `count` slot calls `HEADERLESS`.
fn slot_document(case: &Case) -> PathBuf {
    case.write(
        "slot.md",
        &format!("---\n$schema:\n    count: \"string(min({{{{ length({HEADERLESS}) }}}}))\"\ncount: abc\n---\n\nBody.\n"),
    )
}

/// A cold call, then calls answered by the raw layer (a new delimiter), the
/// parse layer (only `header` differs, which binding applies), and the bind
/// layer (a repeat), each in its own composition over one runtime.
#[test]
fn memo_hits_at_every_layer_rerecord_the_observation() {
    let case = Case::new();
    let calls = [
        ("cold", "{}", (1, 1, 1, 0, 0)),
        ("raw", "{delimiter: ';'}", (1, 2, 2, 0, 0)),
        ("parse", "{header: false}", (1, 2, 3, 1, 0)),
        ("bind", "{header: false}", (1, 2, 3, 2, 1)),
    ];
    for (layer, options, expected) in calls {
        let body = format!("{{{{ length(load_data('./rows.csv', 'tuple([string])[]', {options})) }}}}\n");
        let observed = case.rows_observed_during(|| {
            case.compose_body(&format!("{layer}.md"), &body);
        });
        assert_eq!(observed, vec![case.rows_read()], "the {layer} call observes the read once");
        assert_eq!(layers(case.runtime.stats()), expected, "the {layer} layer answered: {:?}", case.runtime.stats());
    }
}

/// A document call warms all three layers; a slot making the same call with
/// an unanswering handler is refused before any of them is consulted.
#[test]
fn slot_gate_refuses_before_any_warmed_layer() {
    let case = Case::new();
    assert_eq!(case.compose_body("warm.md", &format!("{{{{ length({HEADERLESS}) }}}}\n")), "2");
    let warmed = case.runtime.stats();
    assert_eq!((warmed.reads, warmed.parses, warmed.binds), (1, 1, 1), "{warmed:?}");

    let document = slot_document(&case);
    let handler = Arc::new(Recording::default());
    let mut failure = None;
    let observed = case.rows_observed_during(|| {
        failure = case.compose_with(&document, ComposeOptions::new().with_shell_approval_handler(handler.clone())).err();
    });
    let failure = failure.expect("an unanswered slot read fails the composition");
    let codes: Vec<String> = error_diagnostics(&failure.error).into_iter().map(|record| record.code).collect();
    assert_eq!(codes, vec![APPROVAL_REQUIRED.to_string()], "{}", failure.error);

    let locations: Vec<ReadLocation> = handler.asked().into_iter().map(|request| request.location).collect();
    assert!(locations.contains(&ReadLocation::Local(case.path(ROWS))), "the gate saw the target: {locations:?}");
    // The contract (a separate memo, kept apart per slot session) is prepared
    // before the gate; only the three data-source layers must stay untouched.
    assert_eq!(layers(case.runtime.stats()), layers(warmed), "no layer answered the refused call: {:?}", case.runtime.stats());
    assert_eq!(observed, Vec::<Observation>::new(), "the refused call observed nothing");
}

/// One executed command ends reuse at every layer at once: the generation
/// grows by one, and the same three calls after it redo the raw, parse, and
/// bind work they shared before it.
#[test]
fn generation_advances_once_for_all_layers() {
    let case = Case::new();
    let call = |options: &str| format!("{{{{ length(load_data('./rows.csv', 'tuple([string])[]', {options})) }}}}");
    let warm = [call("{}"), call("{delimiter: ';'}"), call("{header: false}")].join("\n\n");
    let document = case.write("doc.md", &format!("{warm}\n\n::shell git --version\n\n{warm}\n"));
    let options = ComposeOptions::new()
        .with_shell(ShellExpansionOptions { policy_root: Some(case.root.clone()), ..Default::default() })
        .with_shell_working_directory(&case.root)
        .with_shell_approval_handler(Arc::new(AllowShell));
    assert_eq!(case.runtime.memo_generation(), 0);
    case.compose_with(&document, options).unwrap_or_else(|failure| panic!("composes: {}", failure.error));

    assert_eq!(case.runtime.memo_generation(), 1, "one command, one generation");
    let stats = case.runtime.stats();
    assert_eq!(stats.invalidations, 1, "{stats:?}");
    assert_eq!(layers(stats), (2, 4, 6, 2, 0), "each layer redid its work after the command: {stats:?}");
}

/// The passive pre-flight collects a three-argument call's literal target
/// (N29) without reading it.
#[test]
fn three_argument_target_is_collected_passively() {
    let case = Case::new();
    let document = slot_document(&case);
    let request = case.request(&document, ComposeOptions::new());
    let handler = Arc::new(Recording::default());
    let observed = case.rows_observed_during(|| {
        let grants = Markdown::try_from(document.as_path())
            .expect("document")
            .compose_slot_approvals(&request, Some(handler.clone()))
            .expect("approvals");
        assert!(grants.is_empty(), "{grants:?}");
    });

    let asked = handler.asked();
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert_eq!(asked[0].function, "load_data");
    assert_eq!(asked[0].location, ReadLocation::Local(case.path(ROWS)));
    assert_eq!(layers(case.runtime.stats()), (0, 0, 0, 0, 0), "{:?}", case.runtime.stats());
    assert_eq!(observed, Vec::<Observation>::new(), "collecting read nothing");
}

/// Pre-fetch discovery finds a three-argument call's URL (N20), whether the
/// function is spelled canonically or in a folded spelling.
#[test]
fn three_argument_url_is_discovered_before_fetch() {
    let case = Case::new();
    let content = [
        "{{ load_data('https://example.com/export?id=7', '{name: string}', {format: 'json'}) }}",
        "{{ loadData('https://example.org/rows', 'tuple([string])[]', {format: 'csv', header: false}) }}",
        "{{ LOAD_DATA('https://example.net/rows.tsv', 'tuple([string])[]', {header: false}) }}",
    ]
    .join("\n\n");
    let source = ComposeSource::File(case.path("doc.md"));
    let found: Vec<String> =
        discover_remote_urls_from_expressions(&content, &source).into_iter().map(|url| url.normalized_key).collect();
    assert_eq!(
        found,
        vec![
            "https://example.com/export?id=7".to_string(),
            "https://example.org/rows".to_string(),
            "https://example.net/rows.tsv".to_string(),
        ]
    );
}
