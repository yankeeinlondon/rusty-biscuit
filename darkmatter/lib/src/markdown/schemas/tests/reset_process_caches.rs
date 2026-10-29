//! [`reset_process_caches_for_bench`] makes the next validation cold without
//! changing its verdict.

use std::path::Path;

use super::super::triggers::PayloadMemoTally;
use super::super::{DarkmatterSchemas, SHARED_VALIDATOR_CACHE, reset_process_caches_for_bench};
use crate::markdown::Markdown;

const TRIGGER: &str = "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema: payload.yaml\n";
const PAYLOAD: &str = "$schema:\n  model: string(required)\n";

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, contents).expect("write fixture");
}

/// `(valid, problem messages, shared validators cached, payload memo counts)`
/// for one validation through a fresh wrapper with trigger discovery.
fn validate(root: &Path, document: &Markdown) -> (bool, Vec<String>, usize, (u64, u64)) {
    let context = biscuit_file::FileResolutionContext::from_snapshot(root, None, std::collections::HashMap::new())
        .with_repository_root(root);
    let tally = PayloadMemoTally::start();
    let report = DarkmatterSchemas::new(context)
        .with_trigger_discovery()
        .expect("trigger discovery")
        .validate(document)
        .expect("validation runs");
    let counts = tally.counts();
    let messages = report.problems.iter().map(|problem| problem.message.clone()).collect();
    (report.valid, messages, SHARED_VALIDATOR_CACHE.len(), (counts.hits, counts.misses))
}

#[test]
fn reset_process_caches_makes_the_next_validation_cold_with_the_same_verdict() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    std::fs::create_dir_all(root.join(".git")).expect("repository marker");
    write(&root.join("schemas/trigger.yaml"), TRIGGER);
    write(&root.join("schemas/payload.yaml"), PAYLOAD);
    let path = root.join("doc.md");
    write(&path, "---\nprompt: hello\nmodel:\n  - 1\n---\nbody\n");
    let document = Markdown::try_from(path.as_path()).expect("document");

    reset_process_caches_for_bench();
    assert!(SHARED_VALIDATOR_CACHE.is_empty(), "a reset empties the shared validator cache");

    let (valid, problems, compiled, (hits, misses)) = validate(&root, &document);
    assert!(!valid, "a list `model` violates the trigger payload");
    assert!(compiled > 0, "a cold validation compiles a validator");
    assert_eq!((hits, misses), (0, 1), "a cold validation resolves the payload");

    let warm = validate(&root, &document);
    assert_eq!(warm, (valid, problems.clone(), compiled, (1, 0)), "a warm validation reuses both");

    reset_process_caches_for_bench();
    assert!(SHARED_VALIDATOR_CACHE.is_empty(), "a second reset empties the cache again");
    let cold = validate(&root, &document);
    assert_eq!(cold, (valid, problems, compiled, (0, 1)), "after a reset: same verdict, compiled again");
}
