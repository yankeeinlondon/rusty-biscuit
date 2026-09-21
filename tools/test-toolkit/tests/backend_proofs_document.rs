//! `backend-proof verify`'s verdict document, the cross-language half of
//! `scripts/ci/completion.py --backend-proofs`.
//!
//! The Python side consumes `scripts/ci/fixtures/backend-proofs-tmux.json` in
//! `scripts/ci/test_completion.py`; this side proves the writer produces that
//! exact document, so the two ends of the wiring are pinned to one artifact.

use std::collections::BTreeSet;
use std::path::Path;

use test_toolkit::{
    BACKEND_EXECUTIONS_FILE, BACKEND_PROOFS_FILE, Backend, ExecutionDecision, ExecutionRecord,
    append_backend_execution, backend_proofs_json, clear_backend_evidence,
    read_backend_executions, workspace_root, write_backend_proofs,
};

fn record(backend: Backend, test: &str, decision: ExecutionDecision) -> ExecutionRecord {
    ExecutionRecord {
        backend: backend.as_str().to_owned(),
        test: test.to_owned(),
        decision: decision.as_str().to_owned(),
    }
}

fn required(backends: &[Backend]) -> BTreeSet<Backend> {
    backends.iter().copied().collect()
}

fn parsed(text: &str) -> serde_json::Value {
    serde_json::from_str(text).expect("the proof document is JSON")
}

#[test]
fn a_proven_backend_matches_the_shared_fixture_after_normalization() {
    let fixture = workspace_root().join("scripts/ci/fixtures/backend-proofs-tmux.json");
    let expected = std::fs::read_to_string(&fixture)
        .unwrap_or_else(|err| panic!("{}: {err}", fixture.display()));

    let records = [
        record(Backend::Tmux, "a::level2_one", ExecutionDecision::Run),
        record(Backend::Tmux, "a::level2_two", ExecutionDecision::Run),
        // Skips prove nothing and are not counted as executions.
        record(Backend::Tmux, "a::level2_three", ExecutionDecision::Skip),
        // Another backend's records never leak into a tmux-only requirement.
        record(Backend::Kitty, "a::level2_four", ExecutionDecision::Run),
    ];
    let document = backend_proofs_json(&required(&[Backend::Tmux]), &records);

    assert_eq!(parsed(&expected), parsed(&document));
    assert_eq!(expected, document, "the writer is deterministic byte for byte");
}

#[test]
fn an_unproven_backend_is_recorded_as_proven_false_not_omitted() {
    let records = [
        record(Backend::Tmux, "a::level2_one", ExecutionDecision::Run),
        record(Backend::Kitty, "a::level2_two", ExecutionDecision::Skip),
        record(Backend::Kitty, "a::level2_three", ExecutionDecision::Panic),
    ];
    let document = backend_proofs_json(&required(&[Backend::Kitty, Backend::Tmux]), &records);

    assert_eq!(
        parsed(r#"{"kitty":{"executed":0,"proven":false},"tmux":{"executed":1,"proven":true}}"#),
        parsed(&document)
    );
    assert!(
        document.find("\"kitty\"") < document.find("\"tmux\""),
        "keys are emitted in backend-name order: {document}"
    );
}

#[test]
fn verify_writes_the_document_and_reset_clears_both_files() {
    let temp = tempfile::tempdir().expect("temp dir");
    let stage: &Path = temp.path();
    let evidence = stage.join(BACKEND_EXECUTIONS_FILE);
    let proofs = stage.join(BACKEND_PROOFS_FILE);

    append_backend_execution(&evidence, Backend::Tmux, "a::level2_one", ExecutionDecision::Run)
        .expect("append");
    let records = read_backend_executions(&evidence).expect("read");
    write_backend_proofs(&proofs, &required(&[Backend::Tmux]), &records).expect("write");
    assert_eq!(
        parsed(r#"{"tmux":{"executed":1,"proven":true}}"#),
        parsed(&std::fs::read_to_string(&proofs).expect("proofs written"))
    );

    clear_backend_evidence(stage).expect("clear");
    assert!(!evidence.exists(), "reset removes the execution log");
    assert!(!proofs.exists(), "reset removes the previous verdict");
    clear_backend_evidence(stage).expect("clearing an already-clean stage is not an error");
}

#[test]
fn the_writer_creates_a_missing_stage_directory() {
    let temp = tempfile::tempdir().expect("temp dir");
    let proofs = temp.path().join("nested").join("stage").join(BACKEND_PROOFS_FILE);
    write_backend_proofs(&proofs, &required(&[Backend::Tmux]), &[]).expect("write");
    assert_eq!(
        parsed(r#"{"tmux":{"executed":0,"proven":false}}"#),
        parsed(&std::fs::read_to_string(&proofs).expect("proofs written"))
    );
}
