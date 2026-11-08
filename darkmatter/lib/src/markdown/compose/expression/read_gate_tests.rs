//! The slot read gate through the dispatch entry: every input-reading
//! function asks it, a content read asks before reading or consulting the
//! shared memo, and a context without a gate is unchanged.

use std::path::Path;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use super::ExpressionError;
use super::ResolutionContext;
use super::functions::{ReadKind, call_function, input_reading_functions};
use super::read_gate::{GateRefusal, ReadGate, ReadLocation, ReadTarget};

/// A gate that records every target it is asked about and answers `allow`.
#[derive(Debug)]
struct RecordingGate {
    allow: bool,
    asked: Mutex<Vec<ReadTarget>>,
}

impl RecordingGate {
    fn new(allow: bool) -> Arc<Self> {
        Arc::new(Self { allow, asked: Mutex::new(Vec::new()) })
    }

    fn asked(&self) -> Vec<ReadTarget> {
        self.asked.lock().unwrap().clone()
    }
}

impl ReadGate for RecordingGate {
    fn check(&self, target: &ReadTarget) -> Result<(), GateRefusal> {
        self.asked.lock().unwrap().push(target.clone());
        if self.allow { Ok(()) } else { Err(GateRefusal::approval_required()) }
    }
}

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("os.yaml"), "- macOS\n- Linux\n- Windows\n").unwrap();
    std::fs::write(dir.path().join("page.md"), "---\ntitle: Page\n---\n# Page\n").unwrap();
    std::fs::write(dir.path().join("review-1.md"), "").unwrap();
    dir
}

fn context(dir: &Path) -> ResolutionContext {
    ResolutionContext::new(biscuit_file::FileResolutionContext::new(dir))
        .with_home_dir(dir.to_path_buf())
        .with_ctx_value("agent", json!("claude"))
}

fn gated(context: &ResolutionContext, gate: &Arc<RecordingGate>) -> ResolutionContext {
    let mut gated = context.clone();
    gated.slot_gate = Some(gate.clone() as Arc<dyn ReadGate>);
    gated
}

/// One call of each input-reading function, on an input that exists.
fn calls() -> Vec<(&'static str, Vec<Value>)> {
    let page = || json!("./page.md");
    vec![
        ("load_data", vec![json!("./os.yaml"), json!("string[]")]),
        ("file_exists", vec![page()]),
        ("frontmatter", vec![page()]),
        ("try_frontmatter", vec![page()]),
        ("markdown_title", vec![page()]),
        ("markdown_body_empty", vec![page()]),
        ("validate_schema", vec![page()]),
        ("find_files", vec![json!("./*.md")]),
        ("find_first_index", vec![json!("./review-1.md")]),
        ("find_last_index", vec![json!("./review-1.md")]),
        ("absolute", vec![page()]),
        ("relative", vec![page()]),
        ("basename", vec![page()]),
        ("basename_without_index", vec![json!("./review-1.md")]),
        ("dirname", vec![page()]),
        ("ext", vec![page()]),
        ("parent_dir", vec![page()]),
        ("file_trailing", vec![page()]),
        ("dir_leading", vec![page()]),
        ("join", vec![json!("."), json!("page.md")]),
        ("link", vec![page()]),
        ("code_link", vec![page()]),
        ("is_indexed_file", vec![json!("./review-1.md")]),
        ("file_index", vec![json!("./review-1.md")]),
        ("increment_file_index", vec![json!("./review-1.md")]),
        ("decrement_file_index", vec![json!("./review-1.md")]),
        ("has_skill", vec![json!("some-skill")]),
        ("has_local_skill", vec![json!("some-skill")]),
    ]
}

#[test]
fn the_call_table_covers_every_input_reading_function() {
    let mut covered: Vec<_> = calls().into_iter().map(|(name, _)| name).collect();
    let mut registered = input_reading_functions();
    covered.sort_unstable();
    registered.sort_unstable();
    assert_eq!(covered, registered);
}

#[test]
fn every_input_reading_function_asks_the_gate_and_a_refusal_is_typed() {
    let dir = fixture();
    let base = context(dir.path());
    for (name, args) in calls() {
        let gate = RecordingGate::new(false);
        let result = call_function(name, &args, Some(&gated(&base, &gate))).expect("registered");
        assert!(
            matches!(result, Err(ExpressionError::SlotReadRefused { refusal: GateRefusal::ApprovalRequired { .. }, .. })),
            "{name}: {result:?}"
        );
        assert!(gate.asked().iter().all(|target| target.function == name), "{name}: {:?}", gate.asked());
        // Without a gate the same call answers.
        assert!(call_function(name, &args, Some(&base)).expect("registered").is_ok(), "{name}");
    }
}

#[test]
fn a_content_read_asks_before_reading_and_a_refusal_reads_nothing() {
    let dir = fixture();
    let base = context(dir.path());
    let gate = RecordingGate::new(false);
    let refused = call_function("load_data", &[json!("./os.yaml"), json!("string[]")], Some(&gated(&base, &gate)));
    assert!(matches!(refused, Some(Err(ExpressionError::SlotReadRefused { .. }))), "{refused:?}");
    let os = biscuit_file::canonicalize_simplified(&dir.path().join("os.yaml")).unwrap();
    let first = &gate.asked()[0];
    assert_eq!((first.kind, &first.location), (ReadKind::Content, &ReadLocation::Local(os)));
    assert_eq!(base.data.stats().reads, 0, "nothing was read");
    assert!(base.data.read_set().is_empty(), "a refused read is no dependency");
}

#[test]
fn a_file_the_document_already_loaded_is_still_gated_for_the_slot() {
    let dir = fixture();
    let document = context(dir.path());
    let args = [json!("./os.yaml"), json!("string[]")];
    let loaded = call_function("load_data", &args, Some(&document)).unwrap().unwrap();
    assert_eq!(loaded, json!(["macOS", "Linux", "Windows"]));
    let hits = document.data.stats().source_hits;

    let gate = RecordingGate::new(false);
    let refused = call_function("load_data", &args, Some(&gated(&document, &gate))).unwrap();
    assert!(matches!(refused, Err(ExpressionError::SlotReadRefused { .. })), "{refused:?}");
    assert_eq!(document.data.stats().source_hits, hits, "the memo was never consulted for the slot");

    let allowed = RecordingGate::new(true);
    let value = call_function("load_data", &args, Some(&gated(&document, &allowed))).unwrap().unwrap();
    assert_eq!(value, loaded, "an allowed read answers from the shared memo");
    assert_eq!(allowed.asked()[0].kind, ReadKind::Content);
}

#[test]
fn a_refused_read_is_never_recovered_into_a_value() {
    let dir = fixture();
    let base = context(dir.path());
    let gate = RecordingGate::new(false);
    for raw in ["./page.md", "./absent.md"] {
        let result = call_function("try_frontmatter", &[json!(raw)], Some(&gated(&base, &gate))).unwrap();
        assert!(matches!(result, Err(ExpressionError::SlotReadRefused { .. })), "{raw}: {result:?}");
    }
}

#[test]
fn a_remote_read_asks_before_any_fetch() {
    let dir = fixture();
    let base = context(dir.path());
    let gate = RecordingGate::new(false);
    for (name, args) in [
        ("load_data", vec![json!("https://example.com/os.yaml"), json!("string[]")]),
        ("frontmatter", vec![json!("https://example.com/page.md")]),
        ("file_exists", vec![json!("https://example.com/page.md")]),
    ] {
        let result = call_function(name, &args, Some(&gated(&base, &gate))).unwrap();
        assert!(matches!(result, Err(ExpressionError::SlotReadRefused { .. })), "{name}: {result:?}");
    }
    assert!(gate.asked().iter().all(|target| matches!(target.location, ReadLocation::Remote(_))));
    assert!(base.data.read_set().is_empty(), "no URL was recorded or fetched");
}

#[test]
fn an_allowing_gate_changes_no_answer_and_pure_functions_never_ask() {
    let dir = fixture();
    let base = context(dir.path());
    let gate = RecordingGate::new(true);
    let gated = gated(&base, &gate);
    for (name, args) in calls() {
        let open = call_function(name, &args, Some(&base)).unwrap().unwrap();
        let through_gate = call_function(name, &args, Some(&gated)).unwrap().unwrap();
        assert_eq!(through_gate, open, "{name}");
    }
    let asked = gate.asked().len();
    for (name, args) in [("length", vec![json!([1, 2])]), ("has_command", vec![json!("sh")]), ("is_today", vec![json!("2024-01-01")])] {
        call_function(name, &args, Some(&gated)).unwrap().unwrap();
    }
    assert_eq!(gate.asked().len(), asked, "functions that read no input never ask");
}
